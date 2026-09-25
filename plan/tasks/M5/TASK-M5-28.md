# TASK-M5-28 — The frame record, percentiles and the bounded telemetry file

- **Milestone:** M5
- **Closes:** REQ-TOOL-050, REQ-TOOL-051, REQ-TOOL-052, REQ-TOOL-053, REQ-TOOL-054, REQ-TOOL-117
- **Depends on:** TASK-M5-21
- **Needs (earlier milestones):** REQ-TOOL-001, REQ-TOOL-002, REQ-TOOL-005, REQ-TOOL-006, REQ-TOOL-008
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Every frame the loop emits a record with frame_ms, quads_computed, quads_reused, samples, substeps_total,
playhead_dt, camera_delta, tree_depth_max (and leaf count) and stage_ms over integrate / reduce / colour / upload / present,
measured in every build and never compiled out. Reports give p50 / p95 / p99 / max, split moving from still frames by
camera_delta > 0 and headline frac_frames_over_41.7ms during motion. The file size is bounded by downsampling or idle
roll-up, the scheme and its N proposed as a calibration. `prin` batch renders emit the same record (camera_delta = 0, no
present stage) and one parser reads both.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/telemetry/{record.rs, writer.rs}`; `crates/prin/src/profile/report.rs` (percentiles).
- Tests `crates/engine/tests/telemetry.rs`, `crates/prin/tests/report.rs`.
- `cargo xtask bench telemetry-overhead` (release build).
- The size-bound calibration proposal: file sizes of a long 60 fps session with idle stretches.

## Acceptance tests
- `cargo test -p engine frame_record_fields` — a recorded frame deserialises with every field present (REQ-TOOL-050).
- `cargo test -p prin report_percentiles` — synthetic frame log: percentiles and the motion-only headline match hand-computed values (REQ-TOOL-051).
- `cargo test -p engine telemetry_bounded` — a simulated long 60 fps session with idle stretches produces a file below a bound and keeps every motion frame (REQ-TOOL-052).
- `cargo xtask bench telemetry-overhead` — release build emits stage timings; measured instrumentation overhead is negligible against 16.7 ms (REQ-TOOL-053).
- `cargo test -p prin batch_parses_interactive` — a prin batch output parses with the interactive parser (REQ-TOOL-054).
- Review checklist (perf) — file sizes of a long 60 fps session with idle stretches under the chosen scheme and N, with every motion frame kept; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-TOOL-117).

## Notes
- Calibrations (R-71) this task proposes: REQ-TOOL-117.
- REQ-TOOL-117 is an R-71 calibration; the human confirms it at the M5 gate.
