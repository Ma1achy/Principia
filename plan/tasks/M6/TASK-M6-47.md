# TASK-M6-47 — The GUI track (R-390): F3 fills by Extend or Fit, chosen in View for the session; Fit scales the view's square to the window's shorter side, never stretched, the rest background (R-468)

- **Milestone:** M6
- **Closes:** REQ-GUI-229
- **Depends on:** TASK-M6-46, TASK-M6-30
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
On the mock engine, View gains the row **F3 fills by: Extend / Fit**, two radio options after F3 hide, Extend at each launch and the choice kept for the session in `ViewUI` (R-468). Extend is today's fill (REQ-GUI-179). Fit scales the view's square uniformly to the window's shorter side, centred, never stretched, the rest of the window the clear colour, the fake quads chosen at the new screen scale; it sends no `SetField` and adds no undo entry. Present mode follows the mode. F3 again returns the shown layout and the view in either mode, nothing moved.

## References
- `decisions.md` § "R-468 — With the egui layer hidden, F3 fills the window by one of two modes: Extend, the default and R-406's, more of the field at the same scale, or Fit, the view's square scaled to the window's shorter side, never stretched, the rest canvas background; chosen in View › "F3 fills by: Extend / Fit" for the session, a view change and never an edit *(amends R-406)*"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-467 — Forbidden pixels and non-finite values are debug-only: a presentation view draws a pixel that is not part of the chart as the canvas background, with no hatch, key or share, and every failure and non-finite value `#141418`; one switchable debug view, View › Debug view, shows the hatch, `debug_invalid` and a NaN / Inf style, each with its key and count *(amends R-446, and R-407 clause 2 as it applies to display; closes RQ-319)*"
- `decisions.md` § "R-462 — Every panel boundary is a draggable splitter, the canvas taking the space left; each panel has a minimum size, a double-click resets a splitter, the layout is remembered and View › reset layout restores it; resizing the canvas re-frames the figure and never re-integrates it"
- `decisions.md` § "R-466 — Slice centre, Tilt and Navigate each have a "reset" beside the heading that returns the section to the active chart preset's own values, never literal zero; Home does all three in one command, from a button in the canvas's corner, the Home key in both keyboard modes and View › Home; each is one undoable command, dimmed when already home, and drives the compass live *(extends R-409, R-457, R-458 and R-462)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- The row in `crates/gui/src/explore/top_bar.rs`, after F3 hide and before Debug view; `ViewUI`'s `F3Fill { Extend, Fit }`, the GUI's own state.
- Keyboard reach (§G3; gui review 5585209592 of PR #253, G7): the row and its two options registered as scopes of the View menu's scope tree, as `HIDE` is, Enter on an option choosing it.
- Fit's placement in the mock canvas (`crates/gui/src/mock/canvas.rs`, `Placement`): the scale `k = min(W, H) / w`, centred, the window outside the square the clear colour; the fake quads (`crates/gui/src/mock/quads.rs`) split on `frac = quad width / (shown figure width × k)`, `k = 1` outside Fit, so the shown layout's and Extend's quads, and the `mock_tile_bounds` capture, stay unchanged and only Fit refines (R-468 E3; gui review 5585209592 of PR #253, G6). *Was: "split by screen points, not figure fractions, so they refine at `k`".*
- Present mode in Fit.
- Tests `f3_fill_modes`; a capture case `mock_f3_fit`.

## Acceptance tests
- `cargo test -p gui f3_fill_modes` and `cargo xtask screenshot 01_main` (`mock_f3_fit`) — as REQ-GUI-229's verify: Extend pixel-identical over the shown rect; Fit's side equal to the window's shorter side within one pixel, one scale, its extent ratio 1 : 1, centred, the rest the clear colour, finer fake quads; F3 twice in both modes returning the layout and `(z₀, q₁, q₂)` bit for bit and the capture pixel-identical, no `SetField`; present mode in Fit equal to F3 in Fit; under Extend the hook's region background in the presentation view and hatched in the debug view; the shown layout's and Extend's quads and the `mock_tile_bounds` capture unchanged; the row and its options reached by the keyboard, Enter choosing; with its controls (REQ-GUI-229).

## Notes
- **What to try.** `cargo run -p gui --features mock`; View › F3 fills by: Fit; press F3: the square fills the window's height, bands of background either side; F3 again: the layout is as it was. Quit and relaunch: Extend again.
- **Depends on TASK-M6-46**: both add a View entry, and the Extend check reads View › Debug view.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green.
- **Flagged (R-468 E2, E4, E6, applied per R-369):** "the chart square" read as the view's square; "remembered per session" as for the session; present mode follows the mode.
- **qa's files (R-290, R-468 E9; gui review 5585209592 of PR #253, G2).** `xtask/tests/qa_TASK-M6-24.rs`:118-150, `check_01_main` (the 01_main case names "and no other"), must change in this task's qa commit only, gaining `mock_f3_fit`; the implementer's head is red on it until then. *Was: "No qa assertion is known to assert what R-468 changes".*
