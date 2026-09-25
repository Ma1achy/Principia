# TASK-M5-24 — Loop hygiene: never wait, per-frame scheduler budget, background priority, backgrounding, queue depth

- **Milestone:** M5
- **Closes:** REQ-SYS-031, REQ-SYS-034, REQ-SCHED-023, REQ-SCHED-076, REQ-SCHED-043, REQ-SCHED-044, REQ-PERF-024, REQ-PERF-082
- **Depends on:** TASK-M5-14, TASK-M5-21, TASK-M5-04
- **Needs (earlier milestones):** REQ-PERF-003, REQ-PERF-010, REQ-TOOL-001, REQ-TOOL-006
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~420 lines

## Goal
The loop never waits: it runs on a dedicated render thread, off the input/GUI thread (R-113), never awaits GPU work, and the input → view-state →
uniform write → draw path is the only synchronous one. Scheduler work per frame is time-budgeted (N quads, remainder
deferred) with the budget proposed as a calibration; background work (prebake, export job, catch-up march) runs at lower
priority, pre-emptible and yielding; focus loss or a hidden window stops GPU work and the telemetry records it; the GPU
in-flight queue depth is kept shallow, its value proposed as a calibration.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- `docs/contracts/principia_caching_contract.md` § "Part 8 — The composed guarantee"
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "CPU"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "GPU"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Collect everything relevant, in one file"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/engine/src/frame/{budget.rs, background.rs, focus.rs}`; queue-depth control in the dispatch submitter.
- Benchmarks: `cargo xtask bench main-thread-latency`, `cargo xtask bench deep-zoom-landing`,
  `cargo xtask bench input-to-photon`.
- Tests `crates/engine/tests/loop_hygiene.rs`.
- Calibration proposals for the per-frame budget and the queue depth, with their benchmark evidence.

## Acceptance tests
- `cargo xtask bench main-thread-latency` — under heavy scheduler load, main-thread frame/input latency shows no hitch (record max frame time) (REQ-SYS-031).
- Review checklist (code) — the loop has no blocking await on GPU completion; it is hosted on a dedicated render thread, not the input/GUI thread (REQ-SYS-034).
- `cargo xtask bench deep-zoom-landing` — deep-zoom gesture landing with dozens of Jacobian quads: frame time stays within budget (REQ-SCHED-023).
- Review checklist (perf) — decisions.md records the budget with its evidence: frame-time percentiles on the deep-zoom landing benchmark with dozens of Jacobian quads; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-SCHED-076).
- `cargo xtask bench deep-zoom-landing` — produces the evidence for the proposal: decisions.md records the budget with its evidence: frame-time percentiles on the deep-zoom landing benchmark with dozens of Jacobian quads (REQ-SCHED-076).
- `cargo test -p engine background_priority` — under a running export the frame loop keeps its budget (REQ-SCHED-043).
- `cargo test -p engine backgrounding_stops_gpu` — hiding the tab halts dispatches; the record logs it (REQ-SCHED-044).
- `cargo xtask bench input-to-photon` — measured input-to-photon latency with in-flight depth recorded in telemetry (REQ-PERF-024).
- Review checklist (perf) — decisions.md records the depth with its evidence: measured input-to-photon latency and throughput at each candidate depth; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-PERF-082).
- `cargo xtask bench input-to-photon` — produces the evidence for the proposal: decisions.md records the depth with its evidence: measured input-to-photon latency and throughput at each candidate depth (REQ-PERF-082).

## Notes
- Calibrations (R-71) this task proposes: REQ-SCHED-076, REQ-PERF-082.
- REQ-SCHED-076 and REQ-PERF-082 are R-71 calibrations: proposed values with evidence in the PR; the human
  confirms them at the M5 gate.
- RQ-99 ruled: R-113 — the native frame loop runs on a dedicated render thread (REQ-SYS-034's M5 half); the wasm-engine worker is REQ-SYS-039/049 (M8, TASK-M8-37). REQ-DEC-036 moved to M5 (TASK-M5-04), so the deep-zoom-landing benchmark's Jacobian quads carry real x₀/J_D; the switchover (REQ-DEC-033/037) is still M6's.
