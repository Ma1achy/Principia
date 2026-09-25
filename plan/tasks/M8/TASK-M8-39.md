# TASK-M8-39 — The fixed scripted suite and the calibration campaign

- **Milestone:** M8
- **Closes:** REQ-TOOL-087, REQ-VAL-101, REQ-PERF-070, REQ-PERF-073
- **Depends on:** TASK-M8-26, TASK-M8-38
- **Needs (earlier milestones):** REQ-TOOL-006, REQ-PERF-028, REQ-PERF-038, REQ-PERF-041, REQ-RENDER-036, REQ-PERF-026
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
A fixed scripted suite (fixed slices, zoom ladder, pan path, playhead march, no user input) replays identically on every device, sweeps several quality settings, and stays runnable on first launch after calibration. The calibration campaign runs each device across the whole ladder, including settings it will fail, long enough to see thermal behaviour, and inverts the records into tier boundaries (the highest eps and budget that held 16.7 ms, and where it fell back toward 41.7 ms). The at-rest catch-up of checkerboard's stale half is measured on the weakest target device, and Potato is run on a phone.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "1.1 The fixed suite — comparability across devices"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Then invert"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Push every device past where it is comfortable"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Thermal is the honest budget"
- `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"

## Deliverables
- `crates/engine/src/suite/fixed.rs` + `prin suite --sweep …` (in `crates/prin`).
- `xtask/src/campaign.rs` — `cargo xtask bench campaign` runs the ladder and writes the inversion report.
- Recorded results (PR description and `fixtures/bench/`): per-device campaign records and derived boundaries; the fill-in frame time on the weakest device; Potato on a phone.

## Acceptance tests
- `cargo xtask bench fixed-suite` — the suite replays deterministically and reports per-setting results across the sweep (REQ-TOOL-087).
- `cargo xtask bench campaign` — campaign records per device across all settings; derived boundaries recorded (REQ-VAL-101).
- `cargo xtask bench checkerboard-fill-weak-device` — frame time of the fill-in frame on the weakest target device, recorded (REQ-PERF-070).
- `cargo xtask bench potato-phone` — run Potato on a phone already owned; record frame rate and responsiveness (REQ-PERF-073).

## Notes
- REQ-PERF-070 and REQ-PERF-073 need real devices; the recorded runs are attached to the PR. The campaign and the phone run use the browser build (TASK-M8-37, TASK-M8-38).
