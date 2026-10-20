# TASK-M6-49 — The GUI track (R-390): Save view as chart on the mock — the button and View entry, the Saved group, provenance, one undo entry, the oblique latent chart reopened bitwise (R-475)

- **Milestone:** M6
- **Closes:** REQ-GUI-239, REQ-GUI-240
- **Depends on:** TASK-M6-48, TASK-M6-45, TASK-M6-46, TASK-M6-47
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines, the tests included

## Goal
Save view as chart (button after Chart builder…, View entry after Debug view): one undo entry switching to a new saved chart in the preset selector's Saved group, "Saved <n>", renamed inline, its provenance shown; the saved charts the GUI's own settings. On the mock a latent view saves as the oblique chart `(z₀, q₁, q₂)` exactly and reopens bitwise at zero tilt; it can be tilted and saved again.

## References
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `decisions.md` § "R-466 — Slice centre, Tilt and Navigate each have a "reset" beside the heading that returns the section to the active chart preset's own values, never literal zero; Home does all three in one command, from a button in the canvas's corner, the Home key in both keyboard modes and View › Home; each is one undoable command, dimmed when already home, and drives the compass live *(extends R-409, R-457, R-458 and R-462)*"
- `decisions.md` § "R-462 — Every panel boundary is a draggable splitter, the canvas taking the space left; each panel has a minimum size, a double-click resets a splitter, the layout is remembered and View › reset layout restores it; resizing the canvas re-frames the figure and never re-integrates it"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/explore/manifold_view/chart.rs` (the button, the Saved group, rename, provenance), `top_bar.rs` (the View entry, R-475 L1), the GUI's settings store for saved charts.
- Tests `save_view_as_chart`, `saved_latent_chart`; a capture case `mock_saved_chart`.

## Acceptance tests
- `cargo test -p gui save_view_as_chart` and `cargo xtask screenshot 01_main` (`mock_saved_chart`) — REQ-GUI-239's verify.
- `cargo test -p gui saved_latent_chart` — REQ-GUI-240's verify (C8).

## Notes
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green.
- **qa's files:** `xtask/tests/qa_TASK-M6-24.rs`'s `check_01_main` (the 01_main case names, "and no other") gains `mock_saved_chart` (and TASK-M6-48's `mock_tilt_map_selector`) in the qa commit of the task that adds the case, as R-468 E9 did for `mock_f3_fit` (applied per R-369).
- **Depends on TASK-M6-46 and TASK-M6-47**: the View entries' order (R-475 L1).
