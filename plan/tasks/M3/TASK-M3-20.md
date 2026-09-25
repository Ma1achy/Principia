# TASK-M3-20 — The CPU worker pool and the hover-trajectory benchmark

- **Milestone:** M3
- **Closes:** REQ-PERF-003, REQ-PERF-004, REQ-PERF-083, REQ-PERF-085
- **Depends on:** TASK-M3-19, TASK-M0-19
- **Needs (earlier milestones):** REQ-TOOL-001
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
The engine's rayon pool leaves at least one core free by default (more on small-core devices, per the calibrated reserve) and is configurable; one hover trajectory (`computeIC` at t = 50 on one core) fits inside a 60 fps frame.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "CPU"
- `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-144 — pointer_channels §3 is normative too *(closes RQ-74, amends R-109)*"

## Deliverables
- `crates/engine/src/pool.rs` — pool construction from available cores minus the reserve; override in config.
- `xtask` bench `hover-computeic` — computeIC for a typical IC at t = 50 on one core, reporting against 16.7 ms.
- Calibration proposal for the core reserve with UI/OS responsiveness measured under a full pool on the smallest-core target devices.

## Acceptance tests
- `cargo test -p engine pool_reserve` — pool size ≤ available cores − 1 by default; config overrides it (REQ-PERF-003).
- `cargo xtask bench hover-computeic` — computeIC for a typical IC at t = 50 on one core < 16.7 ms (measured 6.1 ms; 1.6 ms at t = 13) (REQ-PERF-004).
- decisions.md records the reserve rule with its evidence (UI and OS responsiveness on the smallest-core target devices under a full pool); the human confirms it at the M3 gate (REQ-PERF-083).
- Proposal: the benchmark ICs (typical and tail) with their total_substeps percentiles; the human confirms them at the M3 gate (REQ-PERF-085).

## Notes
- RQ-74 ruled: R-144 — pointer_channels §3 (responsiveness) is normative beside §4.
- 'Typical IC' is not defined; the bench states the IC it uses.
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-PERF-083.
- Closes, for gaps the corpus leaves open: REQ-PERF-085 (R-71 calibration) (classification accepted by R-132).
