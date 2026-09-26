# TASK-M0-19 — The benchmark runner and the session header

- **Milestone:** M0
- **Closes:** REQ-SYS-067, REQ-TOOL-001, REQ-TOOL-121
- **Depends on:** TASK-M0-04, TASK-M0-14, TASK-M0-18, TASK-M0-05, TASK-M0-06, TASK-M0-20
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
`cargo xtask bench <bench>` runs a fixed benchmark (telemetry §1.1: comparability across devices) headless and writes the same v1 file `prin profile` writes, with its session header filled once from the live system: device (GPU and CPU model, core counts, and VRAM or unified memory size — unified memory in its own field), backend and driver version, precision support and the reported f64 rate, build (commit hash, release profile, feature flags) and display (resolution, refresh, DPI scale). A result is compared with its baseline through `prin profile diff`. The first bench times the trivial kernel of TASK-M0-14.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "1.1 The fixed suite — comparability across devices"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"

## Deliverables
- `xtask/src/bench.rs` — `cargo xtask bench <bench>` and `cargo xtask bench --all` (not in any hosted workflow: run on the human's own Mac via `prin profile`, at milestone gates and on demand, R-186), baselines in `fixtures/bench/<bench>/baseline.json`, compared with `prin profile diff`.
- The session-header probe (wgpu adapter info from the harness, CPU model and cores, memory, build metadata from `git` and cargo) — in `crates/engine/src/telemetry/session.rs`, used by both `prin profile` and the bench runner.
- Bench `trivial-kernel`.
- `.github/workflows/nightly.yml` — `schedule` (daily) and `workflow_dispatch`: the CPU suites (`cargo xtask ci`) and the lavapipe GPU suites; the aggregate survey joins it at M8 (R-134, R-177). No benchmarks (R-186).
- `.github/workflows/gate.yml` — `workflow_dispatch` with input `milestone`: runs `cargo xtask ci`, the GPU suites on `macos-15` and lavapipe, and `cargo xtask screenshot --all`, then `cargo xtask gate-report --milestone <Mn>`, which lists every requirement in that milestone's and every earlier milestone's gate block (`plan/MILESTONES.md`) with the result of its acceptance run, and uploads the report as an artifact (R-177). The benchmark and performance-gate results come from the human's Mac: `gate-report --bench-results <dir>` reads their `prin profile` files, and marks each such requirement "awaiting the human's run" until they are supplied (R-186).

## Acceptance tests
- `cargo test -p engine session_header` — the session header contains every telemetry §2 session field; on a unified-memory adapter fixture unified memory is recorded in its own field and not as VRAM; on a discrete fixture VRAM is recorded and unified memory is absent (REQ-TOOL-001).
- `cargo xtask bench trivial-kernel` — writes a v1 file whose session header is complete, and `prin profile diff` against the checked-in baseline runs (REQ-TOOL-001).
- `cargo test -p xtask gate_report` — on a fixture milestone block, the report names every requirement id with a pass/fail from the fixture results and fails when one is missing (REQ-SYS-067).
- Review checklist (code): no workflow runs bench; `nightly.yml` is scheduled and runs the CPU and lavapipe suites; `gate.yml` takes a `milestone` input, runs every hosted suite, and reports benchmark requirements as awaiting the human's Mac run until their files are supplied (REQ-SYS-067).
- Definition: the f64-rate source and the headless display fields written into dd_telemetry_and_tiers §2 and approved by the physics reviewer (REQ-TOOL-121).

## Notes
- The first benchmark requirement is M3 (REQ-PERF-004); the runner exists before it (MILESTONES M0).
- Where the probe lives is a layout choice (engine telemetry, beside the future frame loop); the firewall requirement REQ-SYS-052 (M8) governs what the engine exposes `pub`.
- See Gaps: the f64 rate, and display fields in a headless run.
- Closes, for gaps the corpus leaves open: REQ-TOOL-121 (R-72 definition) (classification accepted by R-132).
