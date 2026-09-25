# TASK-M7-20 — Colour-vision simulation: real Viénot and Brettel

- **Milestone:** M7
- **Closes:** REQ-COL-042, REQ-COL-045
- **Depends on:** TASK-M7-02
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
Colour-vision simulation as a display-stage function on linear sRGB: real Viénot simulation for protan and deutan and real Brettel simulation for tritan, through LMS space, with matrices and golden values from a published reference implementation named with its version in dd_colouring §3.8 (R-78); achromatopsia multiplies the linear triplet by §3.8's M_achrom. The modes offered are off, deuteranopia, protanopia and tritanopia.

## References
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-78 — Real Viénot and Brettel colour-vision simulation *(closes RQ-29)*"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"

## Deliverables
- `crates/render/shaders/wgsl/compositor/cvd.wgsl` (a fixed display-stage pass, not scanned) and the Rust mirror `crates/render/src/display/cvd.rs`.
- `fixtures/cvd/` — the reference implementation's golden values.
- `docs/design/principia_dd_colouring.md` §3.8 — the reference implementation named with its version (R-78 says it is named when this task lands), with the "Removed lines" note.

## Acceptance tests
- `cargo test -p render cvd_simulation` — dd_colouring unit test 10: protan, deutan and tritan match the named reference implementation's golden values; achrom yields R = G = B exactly; applying any simulation pre-linearisation produces a detectable difference (REQ-COL-042).
- `cargo test -p render cvd_modes` — each of off / deuteranopia / protanopia / tritanopia transforms a test colour set by its Viénot or Brettel simulation, matching the reference golden values; off is identity (REQ-COL-045).

## Notes
- Gap: R-78 leaves the reference implementation (and version) to be named when the task lands; no requirement carries that choice to the human, so it is flagged in the milestone report.
- Gap: achromatopsia is specified (M_achrom) but is not among the four modes render_gui_spec offers; whether it is offered is not said.
