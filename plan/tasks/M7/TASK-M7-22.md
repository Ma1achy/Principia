# TASK-M7-22 — The stain editor canvas (egui): library, graph canvas, preview and ghosted None

- **Milestone:** M7
- **Closes:** REQ-GUI-031, REQ-COL-022
- **Depends on:** TASK-M7-12, TASK-M7-17
- **Needs (earlier milestones):** REQ-RENDER-009
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The Stain mode of the dev GUI: the four surfaces of render_gui_spec Part II §1 (library drawer, graph canvas with its `Graph · Pipeline WGSL · Node WGSL` toggle, node inspector host, preview), the canvas interactions of §7 (select, marquee, move, wire with compatible-port highlighting, drag-off-pin type-filtered create menu, delete with OUT/combiner refusal, pan/zoom) and the glyph-forward node visuals of §8, over the contract-crate graph, so the stain editor is a free, typed node graph (R-64). Removing the colour or brightness node sets None and renders it ghosted; every edit is a SetField, so undo covers it.

## References
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces"
- `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions"
- `docs/gui/principia_render_gui_spec.md` § "8. Node visuals — glyph-forward"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "02 Stain — the plain node-graph editor"

## Deliverables
- `crates/gui/src/stain/{mod,canvas,library,node_visuals,preview}.rs`.
- Screenshot scenarios for `cargo xtask screenshot 02_stain`.
- `crates/gui/tests/stain_canvas.rs` — driving the canvas edits against the contract graph.

## Acceptance tests
- `cargo xtask screenshot 02_stain` — against the stain-editor artboard (layout only, R-68); `cargo test -p gui stain_canvas_typed_wiring` — arbitrary typed wiring is accepted and type-mismatched wiring rejected through the canvas (REQ-GUI-031).
- `cargo xtask screenshot 02_stain` (scenario remove-brightness) and `cargo test -p gui stain_remove_undo` — remove the brightness node: it shows ghosted; restore restores; undo covers the SetField (REQ-COL-022).

## Notes
- Gap: render_gui_spec §16's "Node palette contents" (the right-click palette's source fields and post ops) and the preview default (sphere vs illustrative slice) are still open; R-96 did not rule them.
- R-68: the artboard sets layout; corpus values win.
- Waits on RQ-100 (`REVIEW_QUEUE.md`): Existing requirements closed after the task that needs them.
