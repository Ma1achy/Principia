# TASK-M8-23 — Display window: the last stages and the display overlays (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-101, REQ-GUI-080
- **Depends on:** TASK-M8-05, TASK-M8-19, TASK-M8-08, TASK-M6-28
- **Needs (earlier milestones):** REQ-RENDER-069, REQ-COL-042
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
The Display window holds the fixed last stages (the Print graph in style's place, R-485; display scale, gamut clamp, colour-vision simulation) and the overlays grid, class edges, t_end contours and cursor crosshair. There is no global display bar on Explore or Stain: these controls are reachable only from the Display window and the Overlays menu (R-67).

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
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 02_stain` (no display bar) — screenshots of Explore (01_main.png) and Stain (02_stain.png): no display bar; the Print graph / scale / gamut / CVD are reachable only from the Display window (REQ-GUI-080).

## Notes
- R-390: TASK-M6-28 builds the Display window's frame on the mock engine; this task depends on it and keeps REQ-GUI-101 and REQ-GUI-080.
- **R-476 (20 Oct 2026):** REQ-GUI-101 is reworded: the Display window's overlays are class edges and the cursor crosshair; the grid and t_end contours are Lines ▾ presets (TASK-M8-50).
- **R-509 (20 Oct 2026), Y5:** class edges move to the line system too, a Lines ▾ preset (TASK-M8-50); REQ-GUI-101 now reads "The Display window must offer the overlay cursor crosshair; the grid, the t_end contours and the class edges are Lines ▾ presets", so the Display window's one overlay is the cursor crosshair. The Goal's "the overlays grid, class edges, t_end contours and cursor crosshair" and R-476's note above are kept as history (code review 5587720942 of PR #255, F6).
- **R-485 (PR #281, code review 5608337157 F2):** REQ-GUI-080's verify is restated with the Print graph in style's place; the Print panel itself is TASK-M8-68's. *Was:* the Goal's "(style, display scale, gamut clamp, colour-vision simulation)" and the acceptance line's "style / scale / gamut / CVD are reachable only from the Display window".
