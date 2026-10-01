# TASK-M0-19 — The benchmark runner and the session header

- **Milestone:** M0
- **Closes:** REQ-SYS-067, REQ-TOOL-001, REQ-TOOL-121, REQ-VAL-150
- **Depends on:** TASK-M0-04, TASK-M0-14, TASK-M0-18, TASK-M0-05, TASK-M0-06, TASK-M0-20, TASK-M0-23, TASK-M0-49
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~500 lines

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
- `decisions.md` § "R-196 — Mutation testing joins the QA gate"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-308 — A session that opens no GPU writes `api: "none"`, its GPU fields null *(closes RQ-180)*"
- `decisions.md` § "R-348 — Mutants runs get a per-mutant timeout and a per-process memory cap on test processes; both values are calibrated"

## Deliverables
- `xtask/src/bench.rs` — `cargo xtask bench <bench>` and `cargo xtask bench --all` (not in any hosted workflow: run on the human's own Mac via `prin profile`, at milestone gates and on demand, R-186), baselines in `fixtures/bench/<bench>/baseline.json`, compared with `prin profile diff`.
- The session-header probe (wgpu adapter info from the harness, CPU model and cores, memory, build metadata from `git` and cargo) — in `crates/engine/src/telemetry/session.rs`, used by both `prin profile` and the bench runner. It fills the GPU fields only from an adapter the run has already opened, and never opens one to fill the header; a run that opens no GPU gets R-308's form (`backend.api` "none", `backend.driver`, `device.gpu`, `device.gpu_cores`, `device.memory` and `precision` null; telemetry §5) (applied per R-308).
- Bench `trivial-kernel`.
- `.github/workflows/nightly.yml` — `schedule` (daily) and `workflow_dispatch`: the CPU suites (`cargo xtask ci`) and the lavapipe GPU suites; the aggregate survey joins it at M8 (R-134, R-177). No benchmarks (R-186).
- `nightly.yml` also runs the full `cargo mutants` over the workspace, with the exclusion list of TASK-M0-23's `.cargo/mutants.toml`, and uploads the mutants report as an artifact (R-196, R-198). It runs under R-348's two caps, the per-mutant timeout and the per-process memory cap on test processes, at the values TASK-M0-49 keeps in one place (REQ-VAL-180, REQ-VAL-181).
- `.github/workflows/gate.yml` — `workflow_dispatch` with input `milestone`: runs `cargo xtask ci`, the GPU suites on `macos-15` and lavapipe, and `cargo xtask screenshot --all`, then `cargo xtask gate-report --milestone <Mn>`, which lists every requirement in that milestone's and every earlier milestone's gate block (`plan/MILESTONES.md`) with the result of its acceptance run, and uploads the report as an artifact (R-177). The benchmark and performance-gate results come from the human's Mac: `gate-report --bench-results <dir>` reads their `prin profile` files, and marks each such requirement "awaiting the human's run" until they are supplied (R-186).

## Acceptance tests
- `cargo test -p engine session_header` — the session header contains every telemetry §2 session field; on a unified-memory adapter fixture unified memory is recorded in its own field and not as VRAM; on a discrete fixture VRAM is recorded and unified memory is absent (REQ-TOOL-001).
- `cargo xtask bench trivial-kernel` — writes a v1 file whose session header is complete, and `prin profile diff` against the checked-in baseline runs (REQ-TOOL-001).
- `cargo test -p xtask gate_report` — on a fixture milestone block, the report names every requirement id with a pass/fail from the fixture results and fails when one is missing (REQ-SYS-067).
- Review checklist (code): no workflow runs bench; `nightly.yml` is scheduled and runs the CPU and lavapipe suites; `gate.yml` takes a `milestone` input, runs every hosted suite, and reports benchmark requirements as awaiting the human's Mac run until their files are supplied (REQ-SYS-067).
- Review checklist (code): `nightly.yml` runs the full `cargo mutants` over the workspace with the per-PR job's exclusion list, and uploads its report, listing caught, missed, unviable and timed-out mutants (REQ-VAL-150).
- Review checklist (code): `nightly.yml`'s full `cargo mutants` run passes the per-mutant timeout and runs its test
  processes under the memory cap, read from the place TASK-M0-49 keeps them, as `mutants.yml`'s shards do
  (REQ-VAL-179, R-348).
- Definition: the f64-rate source and the headless display fields written into dd_telemetry_and_tiers §2 and approved by the physics reviewer (REQ-TOOL-121).

## Notes
- The first benchmark requirement is M3 (REQ-PERF-004); the runner exists before it (MILESTONES M0).
- Where the probe lives is a layout choice (engine telemetry, beside the future frame loop); the firewall requirement REQ-SYS-052 (M8) governs what the engine exposes `pub`.
- See Gaps: the f64 rate, and display fields in a headless run.
- Closes, for gaps the corpus leaves open: REQ-TOOL-121 (R-72 definition) (classification accepted by R-132).
- R-348 (1 Oct): every `cargo mutants` run gets a per-mutant timeout and a per-process memory cap on test
  processes; TASK-M0-49 sets both for the per-PR shards, and this task's nightly run reads the same values. Applied per
  R-204, accepted by R-352: this task depends on TASK-M0-49.
