# TASK-M0-19 — The benchmark runner and the session header

- **Milestone:** M0
- **Closes:** REQ-TOOL-001
- **Depends on:** TASK-M0-04, TASK-M0-14, TASK-M0-18
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
`cargo xtask bench <bench>` runs a fixed benchmark (telemetry §1.1: comparability across devices) headless and writes the same v1 file `prin profile` writes, with its session header filled once from the live system: device (GPU and CPU model, core counts, and VRAM or unified memory size — unified memory in its own field), backend and driver version, precision support and the reported f64 rate, build (commit hash, release profile, feature flags) and display (resolution, refresh, DPI scale). A result is compared with its baseline through `prin profile diff`. The first bench times the trivial kernel of TASK-M0-14.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "1.1 The fixed suite — comparability across devices"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"

## Deliverables
- `xtask/src/bench.rs` — `cargo xtask bench <bench>` and `cargo xtask bench --all` (registered in `cargo xtask ci`), baselines in `fixtures/bench/<bench>/baseline.json`, compared with `prin profile diff`.
- The session-header probe (wgpu adapter info from the harness, CPU model and cores, memory, build metadata from `git` and cargo) — in `crates/engine/src/telemetry/session.rs`, used by both `prin profile` and the bench runner.
- Bench `trivial-kernel`.

## Acceptance tests
- `cargo test -p engine session_header` — the session header contains every telemetry §2 session field; on a unified-memory adapter fixture unified memory is recorded in its own field and not as VRAM; on a discrete fixture VRAM is recorded and unified memory is absent (REQ-TOOL-001).
- `cargo xtask bench trivial-kernel` — writes a v1 file whose session header is complete, and `prin profile diff` against the checked-in baseline runs (REQ-TOOL-001).

## Notes
- The first benchmark requirement is M3 (REQ-PERF-004); the runner exists before it (MILESTONES M0).
- Where the probe lives is a layout choice (engine telemetry, beside the future frame loop); the firewall requirement REQ-SYS-052 (M8) governs what the engine exposes `pub`.
- See Gaps: the f64 rate, and display fields in a headless run.
