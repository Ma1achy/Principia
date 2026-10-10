# TASK-M6-30 — The GUI track (R-390), 7: F3 fills the window with the figure (R-406); #174's values marked confirmed (R-404)

- **Milestone:** M6
- **Closes:** REQ-GUI-179
- **Depends on:** TASK-M6-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
On the mock engine, F3 hiding the egui layer makes the figure fill the whole window (R-406). The stand-in shows more of
the field at the same scale in every direction around the shown figure's rect, each point of the field at the screen
position it has in the shown layout, so nothing moves when F3 toggles and the shown view is never stretched. The mock's
fake quads, and their bounds when the tile-bounds overlay is on (TASK-M6-27), cover the window too, the extra quads at
the shown quads' size. F3 again returns the figure to its rect in the shown layout. Neither sends a `SetField` or adds
an undo entry: the hidden state is `ViewUI`'s, and `z₀`, the basis and the zoom are unchanged. The code's comments that
call #174's values proposed or flagged now read confirmed by R-404.

## References
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-404 — #174's look choices and key-repeat timings are confirmed as built: the focus ring, the breadcrumb, the `?` overlay, the 500 ms delay and 40 ms interval, and the base steps"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/layout.rs` and `crates/gui/src/app.rs`: while the layer is hidden the figure's rect is the window,
  and the canvas is given the shown layout's placement of the field (the shown figure's rect), so it draws each point
  where the shown layout puts it.
- `crates/gui/src/mock/canvas.rs` and `crates/gui/src/mock/noise.wgsl`: the stand-in, the fake quads and their bounds
  drawn over any rect at a given placement of the field, never stretched to the rect.
- The comments in `crates/gui/src/keyboard/repeat.rs`, `crates/gui/src/keyboard/scopes.rs` and
  `crates/gui/src/explore/breadcrumb.rs` that call the delay, the interval, the base steps, the ring's colour and its
  rounding proposed, provisional until the M8 gate or flagged for the human read "confirmed by R-404"; no value
  changes.
- Test `f3_fills_window_mock`; `f3_toggle_mock` updated (its clear-colour check outside the figure goes, its
  identical-over-the-rect check stays); screenshot case `01_main/mock_f3_off` re-captured.

## Acceptance tests
- `cargo test -p gui f3_fills_window_mock` and `cargo xtask screenshot 01_main` (mock_shell, mock_f3_off) — with F3 off no pixel of the window is the clear colour; over the shown figure's rect the F3-off capture is pixel-identical to the F3-on one; outside it the capture equals the stand-in drawn at the same placement over the whole window; the tile-bounds overlay, on, draws the extra quads at the shown quads' size; F3 again restores the shown capture pixel for pixel; the undo depth is unchanged and no SetField is sent; controls: the shown view stretched to the window fails the check over the rect, and a figure left in its rect fails the coverage check (REQ-GUI-179).
- `cargo test -p gui f3_toggle_mock` and `cargo xtask screenshot 01_main` (mock_shell, mock_f3_off, mock_warning) — the figure identical underneath over the shown figure's rect with F3 on and off; a raised warning and error change the footer's counts and nothing over the figure (REQ-GUI-168, closed by TASK-M6-24, re-checked here under R-406).
- `cargo test -p gui mock_keyboard` — F3 still hides and shows the layer as TASK-M6-25 built it, the keyboard taking no key while hidden and the focus kept (REQ-GUI-169, closed by TASK-M6-25, re-checked here).
- Review checklist (code reviewer) of the marks — the three files' comments name R-404 as confirming the values, and `DELAY_MS`, `INTERVAL_MS`, `StepKind::base`, `RING_WIDTH`, `RING_ROUNDING` and the ring's colour are unchanged (R-404; REQ-GUI-146 and REQ-GUI-158, closed by TASK-M6-25).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: F3 with the figure panned and zoomed, the tile
  bounds on, and F3 again, on `cargo run -p gui --features mock`.
- **Why this task, and why after TASK-M6-27.** The fill needs the stand-in to pan and zoom (TASK-M6-26) and the fake
  quads and their bounds (TASK-M6-27). TASK-M6-26 is being built as R-406 is recorded and is not changed; this task
  follows TASK-M6-27, so the whole figure is in place (applied per R-369, R-406).
- **RQ-248 revised.** TASK-M6-24 kept the figure in its shown rect with F3 off and the rest of the window the clear
  colour (RQ-248, per R-369); R-406 revises that. The window title's "mock engine" stays.
- **qa's file, changed under R-406 (R-290).** `crates/gui/tests/qa_TASK-M6-24.rs`'s
  `qa_capture_figure_identical_under_f3_warning_and_console` asserts that with F3 off everything outside the figure's
  rect is the clear colour, which R-406 reverses. Every earlier commit of the file is qa's, so this task's qa reviewer
  changes that assertion in the task's qa commit; its identical-over-the-rect check stays. The implementer never edits
  it, and until that commit the test fails on the implementer's head, as the PR says. The PR lists the change with its
  reason, and the code reviewer confirms that nothing else was weakened.
- **The real engine** is REQ-GUI-180, closed by TASK-M8-05, which depends on this task and waits on RQ-262 (how the
  view extends past the chart's `[0,1]²` and what a chart shows outside its domain). Nothing here waits on it: the
  stand-in is noise defined everywhere, with no chart mapping, so no physics review.
- Sources and silences as TASK-M6-24's Notes give them (R-390).
