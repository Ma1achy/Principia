# TASK-M8-35 — Research (v2): periodic-orbit seeding, Newton refinement, catalogue comparison (11_research.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-122, REQ-GUI-150
- **Depends on:** TASK-M8-11, TASK-M8-05, TASK-M3-30, TASK-M4-15
- **Needs (earlier milestones):** REQ-VAL-043, REQ-VAL-041, REQ-TOOL-044, REQ-PAY-043, REQ-SYS-017, REQ-VAL-126, REQ-INT-001
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~450 lines

## Goal
The Research window's first tool seeds periodic orbits from spiral cores where the winding number diverges, Newton-refines each seed in f64, reports residual and period per seed, and compares against the Šuvakov–Dmitrašinović catalogue. The Newton convergence residual and the catalogue-match tolerance are calibrated on seeds that do and do not refine to catalogue orbits.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "11 Research — first pass (v2)"
- `docs/gui/principia_render_gui_spec.md` § "G11. Research — first pass (v2) (`11_research.png`)"
- `docs/design/principia_dd_validation_orbits.md` § "1.4 Šuvakov–Dmitrašinović (13 families, 2013) and Broucke–Hénon–Hadjidemetriou"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"

## Deliverables
- `crates/validation/src/research/{seed,newton,catalogue}.rs` — the catalogue table from dd_validation_orbits §1.4.
- `crates/gui/src/windows/research/seeding.rs`.
- Calibration proposal: residuals and period errors for refining and non-refining seeds.

## Acceptance tests
- `cargo xtask gate research-seed-catalogue` (residual and tolerance: REQ-GUI-150, calibrated) — on a region containing a catalogue orbit, a seed refines to it and matches its catalogue period (REQ-GUI-122).
- `cargo xtask gate research-seed-catalogue` — the proposal shows residuals and period errors for seeds that refine to catalogue orbits and for seeds that do not; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-150).

## Notes
- R-133: the "winding number" the seeds come from is the payload's winding — the unwrapped phase θ̃ and orbit_count = ⌊|θ̃|/2π⌋ (REQ-INT-001, REQ-PAY-043); spiral cores are where it diverges.
- Calibrations (R-71) proposed here: REQ-GUI-150. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
