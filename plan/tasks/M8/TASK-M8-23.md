# TASK-M8-23 — Display window: the last stages and the display overlays (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-101, REQ-GUI-080
- **Depends on:** TASK-M8-05, TASK-M8-19, TASK-M8-08
- **Needs (earlier milestones):** REQ-RENDER-069, REQ-COL-042
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
The Display window holds the fixed last stages (style, display scale, gamut clamp, colour-vision simulation) and the overlays grid, class edges, t_end contours and cursor crosshair. There is no global display bar on Explore or Stain: these controls are reachable only from the Display window and the Overlays menu (R-67).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"

## Deliverables
- `crates/gui/src/windows/display.rs` — edits the RenderState display-chain fields and overlay set.
- Screenshot cases `04_windows/display`, `01_main/no_display_bar`, `02_stain/no_display_bar`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (Display window) — screenshot against 04_windows.png's Display window (REQ-GUI-101).
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 02_stain` (no display bar) — screenshots of Explore (01_main.png) and Stain (02_stain.png): no display bar; style / scale / gamut / CVD are reachable only from the Display window (REQ-GUI-080).

## Notes
- none
