# TASK-M8-34 — Measure: boundary dimension on a dragged region (10_measure.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-121, REQ-VAL-110, REQ-VAL-111, REQ-GUI-147, REQ-GUI-148, REQ-GUI-149, REQ-VAL-143
- **Depends on:** TASK-M8-05
- **Needs (earlier milestones):** REQ-SCHED-031, REQ-PAY-064, REQ-SCHED-048, REQ-COL-002
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~500 lines

## Goal
The Measure tool lets the user drag a square region on the plot, draws the sampled pairs with disagreeing pairs marked, leaves on Esc, and offers Export CSV and Keep region. It estimates by uncertainty exponent or box count with stated samples (pairs per ε), ε range and pair classifier, and reports α ± error and D = 2 − α for the slice, on a uniform grid with FULL_RETENTION (R-39). The resolution hazard is two actions: a threshold sweep and a matched N / 2N pair. The validation fixture of known dimension and the default classifier are defined in §G10; the ε range, samples per ε and the fit tolerance on the fixture are calibrated.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "10 Measure — a tool on the figure"
- `docs/gui/principia_render_gui_spec.md` § "G10. Measure — a tool on the figure (`10_measure.png`)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/read_first/principia_00_philosophy.md` § "4.5a A quantity that does not converge under refinement is measuring the sampling"

## Deliverables
- `crates/engine/src/measure/{dimension,sweep}.rs` — uncertainty-exponent and box-count estimators, the log-log fit, N / 2N pairing.
- `crates/gui/src/tools/measure.rs` — the figure tool and side panel, CSV export.
- Doc change: `docs/gui/principia_render_gui_spec.md` §G10 (fixture, its known dimension, default classifier).
- Calibration proposals: ε range and samples per ε (fit stability on the fixture); the α tolerance with its margin.

## Acceptance tests
- `cargo xtask screenshot 10_measure` and `cargo test -p gui measure_csv_rows` — screenshot against 10_measure.png; the CSV has one row per sampled pair (REQ-GUI-121).
- `cargo xtask gate measure-dimension` (tolerance: REQ-GUI-149, calibrated) — on a fixture region of known boundary dimension, D = 2 − α is reported with its fit error; the method, samples, ε range and classifier are shown (REQ-VAL-110).
- `cargo test -p engine measure_hazard_actions` — each button produces its paired result; the N / 2N pair differs only in sample count (REQ-VAL-111).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "G10. Measure — a tool on the figure (`10_measure.png`)" — §G10 names the fixture, its known dimension and the default classifier (e.g. outcome class); the physics reviewer approves the doc change before merge (REQ-GUI-147).
- Review checklist (physics reviewer) of the ε range and sample proposal — the proposal shows the log-log fit's stability over the chosen ε range and sample count on the known-dimension fixture; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-148).
- `cargo xtask gate measure-dimension` — the proposal shows the fitted α on the fixture at the default settings and the tolerance with its margin; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-149).
- Definition: the Measure threshold sweep written into render_gui_spec §G10 and approved by the physics reviewer (REQ-VAL-143).

## Notes
- What the "threshold sweep" sweeps (which threshold) is not stated in §G10.
- Calibrations (R-71) proposed here: REQ-GUI-148, REQ-GUI-149. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- Definitions (R-72) written here: REQ-GUI-147. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
- Closes, for gaps the corpus leaves open: REQ-VAL-143 (R-72 definition) (classification accepted by R-132).
