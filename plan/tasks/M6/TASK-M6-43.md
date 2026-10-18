# TASK-M6-43 — Splitters on every panel boundary, the layout remembered and reset, the canvas re-framed without re-integration (R-462)

- **Milestone:** M6
- **Closes:** REQ-GUI-213, REQ-GUI-214
- **Depends on:** TASK-M6-42
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
On the mock engine, every panel boundary becomes a draggable splitter (the Manifold view, the Trajectory panel, the bottom row's Compass, Time and Legend), the figure's canvas taking the rest, each panel with a minimum size, a double-click resetting a splitter, the layout remembered across sessions and View › reset layout restoring it; resizing the canvas re-frames the figure by R-406's rule and never re-integrates it (R-462). Today the layout is fixed proportions of the artboard (`crates/gui/src/layout.rs` lines 62 to 93) and the View menu holds only "F3 hide".

## References
- `decisions.md` § "R-462 — Every panel boundary is a draggable splitter, the canvas taking the space left; each panel has a minimum size, a double-click resets a splitter, the layout is remembered and View › reset layout restores it; resizing the canvas re-frames the figure and never re-integrates it"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"

## Deliverables
- The five splitters, minimum sizes and double-click reset in the layout (`crates/gui/src/layout.rs`), the ring following each scope's rect (R-462 J1, J2, J3, J6).
- The layout in the GUI's settings (TASK-M6-42's store) and View › reset layout (J3, J5).
- The canvas re-framed by R-406's rule with no `SetField` and the mock's integrated quads kept (J4).
- Tests `mock_layout_splitters`, `mock_layout_remembered` and `mock_layout_reframe`.

## Acceptance tests
- cargo test -p gui mock_layout_splitters (pointer scripts on the mock): a drag on each of the five splitters moves that boundary by the drag and the canvas takes the rest; a drag past a minimum stops there, for every panel and the canvas; a double-click returns that splitter, and only it, to its default; the ring follows a resized scope's rect; controls: a fixed layout (today's), and a splitter that closes the canvas, each fail (REQ-GUI-213).
- cargo test -p gui mock_layout_remembered: a layout changed, saved and restored in a new GUI comes back the same; View › reset layout restores the defaults; mock_layout_reframe: a splitter drag that widens the canvas sends no SetField, keeps the figure point under a fixed screen pixel at the same field point (to 1e-9 in chart coordinates) and leaves the mock's integrated quads in place, only quads in the newly exposed area requested; controls: a canvas resize that sends a SetField, one that stretches the figure, and one that drops the integrated quads each fail (REQ-GUI-214).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: drag the boundary between the Manifold view and the figure: the figure shows more field at the same scale, with no "refining"; double-click it to reset; View › reset layout.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **The real figure** (TASK-M8-05) follows R-462 J4 when it lands; REQ-GUI-214 names the rule.
- **Early start (R-388).** It may start off TASK-M6-42's approved head.
