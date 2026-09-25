# TASK-M7-25 — The Display window: display settings outside the pipeline, CVD on every surface

- **Milestone:** M7
- **Closes:** REQ-COL-026
- **Depends on:** TASK-M7-16, TASK-M7-21, TASK-M7-22, TASK-M6-17
- **Needs (earlier milestones):** REQ-RENDER-042, REQ-GUI-013
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The display stage (style, display scale, gamut clamp, CVD) is a set of settings outside every occupant and preset, never seen by the codegen, edited in the Display window (R-67), and colour-vision simulation applies uniformly to everything displayed: the main render, the sphere preview, the equirect unwrap and the node thumbnails.

## References
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/principia_render_gui_spec.md` § "2. The pipeline"
- `docs/gui/principia_render_gui_spec.md` § "12. Display stage — global, outside the pipeline"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/windows/display.rs` — the Display window (order shown, CVD mode, render→display scale, gamut clamp; style placeholder until TASK-M7-26).
- Display settings held outside `RenderState`'s stain graph; the preview, unwrap and thumbnail paths routed through the display chain.
- Screenshot scenario deutan-everywhere.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (scenario deutan-everywhere) — enable deutan: every surface including thumbnails is simulated; `cargo test -p render preset_has_no_display_state` — no preset serialises CVD (REQ-COL-026).

## Notes
- Which state struct holds the display settings (RenderState or ViewUI) is not stated by the requirement; gui_state_contract §2 governs.
