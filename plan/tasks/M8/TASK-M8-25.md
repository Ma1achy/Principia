# TASK-M8-25 — Quality warnings (Green / Yellow / Red), per-choice suppression, and the target-utilisation ceiling

- **Milestone:** M8
- **Closes:** REQ-GUI-045, REQ-GUI-046, REQ-PERF-074, REQ-GUI-044, REQ-PERF-092
- **Depends on:** TASK-M8-24
- **Needs (earlier milestones):** REQ-PERF-025, REQ-PERF-031, REQ-PERF-028, REQ-PERF-040, REQ-PERF-053, REQ-TOOL-001
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
An attempted quality setting change is checked and answered with one of three non-blocking severities: Green applies silently; Yellow shows an fps estimate and "use recommended instead"; Red shows needed vs estimated GB and still lets the user proceed. Nothing warns on the current state. Each dialog offers "don't warn me again for this specific choice", which silences only that choice. The yellow / red fps thresholds and the memory-fit margin are set by measurement at build time. A user-visible target utilisation ("use up to N cores", "cap at 30 fps") defaults to leaving headroom, full utilisation opt-in, and telemetry records which was in force.

## References
- `docs/design/principia_memory_tiers.md` § "7. The "are you sure?" safety system (three severities, none blocking)"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "A deliberate ceiling, user-visible"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/quality/precheck.rs` — severity from the memory estimate (REQ-PERF-025) and the fps model.
- `crates/gui/src/dialogs/quality_warning.rs` — the dialogs and per-choice suppression (ViewUI-side store).
- `crates/gui/src/windows/utilisation.rs` + SimConfig / session field for the ceiling; the telemetry session block records it.
- `cargo xtask bench quality-warn-thresholds` — the measurement that sets the thresholds and margin.
- Tests: `warning_suppression`; screenshot cases `04_windows/warn_yellow`, `04_windows/warn_red`, `04_windows/utilisation`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (Yellow and Red dialogs) — attempt a Yellow and a Red change; dialogs show the stated content and allow proceeding; no warning appears on current state (REQ-GUI-045).
- `cargo test -p gui warning_suppression` — suppress for E = 15; a later render_scale 2.0 change still warns (REQ-GUI-046).
- `cargo xtask bench quality-warn-thresholds` — record the measured thresholds and margin with their method (REQ-PERF-074).
- `cargo xtask screenshot 04_windows` (utilisation control) and `cargo test -p engine utilisation_recorded` — the utilisation control is visible; default leaves headroom; the record names the ceiling (REQ-GUI-044).
- Proposal: the default target-utilisation ceiling, with measured cost against full utilisation; the human confirms it at the M8 gate (REQ-PERF-092).

## Notes
- Neither the GUI spec nor the artboards place the target-utilisation control, and the default headroom is not given.
- Waits on RQ-100 (`REVIEW_QUEUE.md`): Existing requirements closed after the task that needs them.
- Waits on RQ-105 (`REVIEW_QUEUE.md`): GUI surfaces with no artboard.
- Closes, for gaps the corpus leaves open: REQ-PERF-092 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
