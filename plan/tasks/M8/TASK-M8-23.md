# TASK-M8-23 — Display window: the last stages and the display overlays (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-101, REQ-GUI-080
- **Depends on:** TASK-M8-05, TASK-M8-19, TASK-M6-28, TASK-M7-20
- **Needs (earlier milestones):** REQ-COL-042
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
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"

## Deliverables
- `crates/gui/src/windows/display.rs` — edits the RenderState display-chain fields and overlay set.
- Screenshot cases `04_windows/display`, `01_main/no_display_bar`, `02_stain/no_display_bar`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (Display window) — screenshot against 04_windows.png's Display window (REQ-GUI-101).
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 02_stain` (no display bar) — screenshots of Explore (01_main.png) and Stain (02_stain.png): no display bar; the Print graph / scale / gamut / CVD are reachable only from the Display window (REQ-GUI-080).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines, every contract field this task adds among its cases (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here; R-627).

## Notes
- R-390: TASK-M6-28 builds the Display window's frame on the mock engine; this task depends on it and keeps REQ-GUI-101 and REQ-GUI-080.
- **R-476 (20 Oct 2026):** REQ-GUI-101 is reworded: the Display window's overlays are class edges and the cursor crosshair; the grid and t_end contours are Lines ▾ presets (TASK-M8-50).
- **R-509 (20 Oct 2026), Y5:** class edges move to the line system too, a Lines ▾ preset (TASK-M8-50); REQ-GUI-101 now reads "The Display window must offer the overlay cursor crosshair; the grid, the t_end contours and the class edges are Lines ▾ presets", so the Display window's one overlay is the cursor crosshair. The Goal's "the overlays grid, class edges, t_end contours and cursor crosshair" and R-476's note above are kept as history (code review 5587720942 of PR #255, F6).
- **R-485 (PR #281, code review 5608337157 F2):** REQ-GUI-080's verify is restated with the Print graph in style's place; the Print panel itself is TASK-M8-68's. *Was:* the Goal's "(style, display scale, gamut clamp, colour-vision simulation)" and the acceptance line's "style / scale / gamut / CVD are reachable only from the Display window".
- **R-627 (27 Oct 2026), the GUI track in M8:** this task is built on the mock engine, as R-390's track is: on spare agent slots, never holding up the physics chain, and merging once its reviews pass and CI is green without waiting for the gates of M3 to M6 (the gates of M2 and M7 by R-415's test, recorded in the PR; R-627's flag). Its dependence on TASK-M6-21 and TASK-M6-22, which it had through TASK-M8-01, is satisfied by the mock's precision events and quality tier. A control that needs a contract field adds it as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on both engines. TASK-M8-94 wires it to the real engine after TASK-M6-21 and TASK-M6-22, re-running every acceptance line here on the real engine with the conformance suite on both, and takes the dependencies this task gave up. **What to try.** The PR description has a "What to try" section: the clicks and keys that show what changed, on `cargo run -p gui --features mock`. It builds on the track's Display window frame (TASK-M6-28) on the mock; TASK-M8-08 moves to TASK-M8-94 with REQ-RENDER-069 from Needs. *Was:* "- **Depends on:** TASK-M8-05, TASK-M8-19, TASK-M8-08, TASK-M6-28"; "- **Needs (earlier milestones):** REQ-RENDER-069, REQ-COL-042".
- R-627: TASK-M7-20 (merged) is named in Depends on so the Needs it closes stay reachable now that the chain through TASK-M8-05's old dependencies is gone (applied per R-369).
