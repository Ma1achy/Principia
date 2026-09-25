# TASK-M6-18 — The arbiter's timescales: motion offset, change under cover, hysteresis, sensors and persistence

- **Milestone:** M6
- **Closes:** REQ-PERF-059, REQ-PERF-060, REQ-PERF-061, REQ-PERF-062, REQ-PERF-063, REQ-PERF-064, REQ-PERF-066, REQ-PERF-089
- **Depends on:** TASK-M6-17, TASK-M5-22, TASK-M5-23, TASK-M5-28
- **Needs (earlier milestones):** REQ-SCHED-034, REQ-SCHED-036, REQ-GUI-009, REQ-TOOL-051, REQ-SCHED-046
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-3
- **Size:** ~450 lines

## Goal
The controller's cascade: `motion_rung = resting_rung − offset` with the offset learned from the transition sensor as a monotone function of playhead depth; per-frame transients change nothing; resting-rung changes only in sustained quiescence; render-key knobs move under motion/blur cover and sim-key knobs only at natural invalidation moments; checkerboard is never toggled, only costed. It drops fast, rises slow, holds a dead zone and remembers failed rungs with decaying caution. Sensors report sim-time debt (p95 frame time secondary), time-to-resync and fallback-visible duration. The model persists under a provenance signature, and the user's preset intent is restored verbatim. The tunables are set by recorded measurement.

## References
- `docs/design/principia_quality_device_note.md` § "4. Resting rung + motion offset (one ladder, two setpoints)"
- `docs/design/principia_quality_device_note.md` § "Open sub-questions (settle at implementation)"
- `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the one above)"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `docs/design/principia_quality_device_note.md` § "6. Change under cover (the perceptual magic)"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_quality_device_note.md` § "7. Hysteresis + failed-rung memory"
- `docs/design/principia_quality_device_note.md` § "8. Sense what the user feels, not what the GPU reports"
- `docs/design/principia_quality_device_note.md` § "9. Persist the model, not just the rung"
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/quality/cascade.rs`, `hysteresis.rs`, `sensors.rs`, `persist.rs` (serde record with provenance: adapter info + key limits + probe/kernel version).
- Simulated load traces in `fixtures/bench/controller_traces/`; `cargo xtask bench controller-tunables` recording each tunable with the measurement that set it.

## Acceptance tests
- `cargo test -p engine motion_offset_monotone` — motion rung ≤ resting rung always; offset non-decreasing in t (REQ-PERF-059).
- `cargo test -p engine transients_change_nothing` — a single slow frame changes nothing; a rung change is never issued during navigation or catch-up (REQ-PERF-060).
- `cargo test -p engine change_under_cover` — a sim-key rung change waits for the next invalidation event; checkerboard_mode is never written by the arbiter (REQ-PERF-061).
- `cargo test -p engine hysteresis_failed_rung` — simulated load traces show no oscillation into a remembered failed rung before the decay (REQ-PERF-062).
- `cargo test -p engine sensors_felt_quantities` — sensor outputs are these quantities; injected sim-time debt drops the rung (REQ-PERF-063).
- `cargo test -p engine persist_provenance` — reload with matching provenance: no probe, same rung; changed kernel version: re-probe; stored Custom values restored unchanged (REQ-PERF-064).
- `cargo xtask bench controller-tunables` — record each tunable's value and the measurement that set it (REQ-PERF-066).
- Proposal: the arbiter's tunables with their recorded measurements; the human confirms them at the M6 gate (REQ-PERF-089).

## Notes
- REQ-PERF-066 is "set by recorded measurement" but not `kind: calibration`; the recorded values still go to the human at the M6 gate.
- Closes, for gaps the corpus leaves open: REQ-PERF-089 (R-71 calibration) (classification accepted by R-132).
