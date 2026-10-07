# TASK-M6-29 — The GUI track (R-390), 6: the Stain node-graph editor

- **Milestone:** M6
- **Closes:** REQ-GUI-175, REQ-GUI-128, REQ-GUI-131
- **Depends on:** TASK-M6-28
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~550 lines

## Goal
On the mock engine, Stain mode is the plain node-graph editor `02_stain.png` and Part II give: the library drawer
(collapsible) left, the graph canvas centre with its `Graph · Pipeline WGSL · Node WGSL` toggle, the preview and the
node inspector right, and the assembled code and the Problems pane under the canvas. The nodes and wires are fake ones
the mock serves; the preview is live, rendered by the mock, square for a square viewport, and changes when the graph is
edited. The canvas takes the gestures of Part II §7: click-select opening the node's inspector, click on empty canvas
deselecting, marquee multi-select, dragging nodes with their wires following and a multi-selection moving together,
space-drag or middle-drag to pan, scroll to zoom. Every graph edit is a `SetField`. Each surface joins the keyboard
scope tree.

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "02 Stain — the plain node-graph editor"
- `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces"
- `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions"
- `docs/gui/principia_render_gui_spec.md` § "8. Node visuals — glyph-forward"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"

## Deliverables
- `crates/gui/src/stain/{mod,canvas,library,preview,code_view,problems}.rs`: the layout and the canvas gestures over the
  mock's fake graph.
- The mock's content: a plausible graph of fake nodes and wires, library rows, assembled code text, Problems entries, and
  a live preview it re-renders on each graph edit; any contract field the graph edits need, added as the corpus names it
  (R-390's "Contract fields"), with the conformance suite re-run on both engines.
- Tests `mock_stain_preview`, `canvas_gestures_mock`, and `mock_keyboard` extended; screenshot cases
  `02_stain/mock_stain`, `02_stain/mock_stain_selected`, `07_keyboard/mock_focus_stain`.
- Stain mode's scopes in TASK-M6-25's scope tree: in Stain mode the big scopes in Tab order are the top bar, the
  library drawer, the graph canvas, the preview and node inspector, and the code and Problems pane; Enter goes into one,
  and arrows move between its rows or nodes (render_gui_spec §G3 gives Explore's order only; applied per R-369, review
  5434766412 on PR #157).

## Acceptance tests
- `cargo xtask screenshot 02_stain` (mock_stain, mock_stain_selected) and `cargo test -p gui mock_stain_preview` — against 02_stain.png; an edit to the graph is a SetField and the mock's preview changes; the preview is square for a square viewport (REQ-GUI-175).
- `cargo xtask screenshot 02_stain` (mock_stain) — screenshot against 02_stain.png (REQ-GUI-128).
- `cargo test -p gui canvas_gestures_mock` — scripted pointer events for each gesture produce the stated selection / position change (REQ-GUI-131).
- `cargo test -p gui mock_keyboard` (extended with this task's scopes) and `cargo xtask screenshot 07_keyboard` (mock_focus_stain) — every control this task adds joins the scope tree: in Stain mode, Tab and Shift+Tab reach the top bar, the library drawer, the graph canvas, the preview and node inspector, and the code and Problems pane in that order, and Enter reaches each control inside them (a library row, a node, a node inspector field, the Graph · Pipeline WGSL · Node WGSL toggle); Esc backs out one level; the focus ring is drawn on the focused control and the top-bar breadcrumb names its path; arrows move between siblings and adjust a focused value by its proposed base step, ×10 with Shift and ×0.1 with Alt; the screenshot shows the ring and the breadcrumb as 07_keyboard.png draws them (REQ-GUI-169, closed by TASK-M6-25, re-checked here for this screen, applied per R-369, review 5434766412 on PR #157).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: the Explore / Stain switch, selecting and dragging
  nodes, a marquee, panning and zooming the canvas, the Graph · Pipeline WGSL · Node WGSL toggle, an edit changing the
  preview, on `cargo run -p gui --features mock`.
- REQ-GUI-128 moved here from TASK-M8-19 and REQ-GUI-131 from TASK-M8-20 (R-390); both depend on this task and re-run
  that acceptance on the real engine. The real stain graph, its typing and the ghosted None (TASK-M7-22), the node
  inspector (TASK-M7-23, TASK-M8-21), the code views on real WGSL (TASK-M7-24), wiring, the palette and node visuals
  (TASK-M8-20) and the rest of Stain mode (TASK-M8-19) stay with their tasks. TASK-M7-22 depends on this task and builds
  on its canvas.
- Sources and silences as TASK-M6-24's Notes give them (R-390).
- RQ-249, decided per R-369 (7 Oct 2026): TASK-M6-24's mode switch changes a `ViewUI` mode and shows an empty Stain
  page frame; this task fills it.
