# TASK-M7-22 — The stain editor canvas (egui): library, graph canvas, preview and ghosted None

- **Milestone:** M7
- **Closes:** REQ-GUI-031, REQ-COL-022, REQ-GUI-151, REQ-GUI-152
- **Depends on:** TASK-M7-12, TASK-M7-17
- **Needs (earlier milestones):** REQ-RENDER-009
- **Reviewers:** code, qa, gui, physics
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

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/gui/principia_render_gui_spec.md` § "16. Open / next"
## Deliverables
- `crates/gui/src/stain/{mod,canvas,library,node_visuals,preview}.rs`.
- Doc change: `docs/gui/principia_render_gui_spec.md` Part II §7 (the node palette's groups and entries, REQ-GUI-151) and §1 (the Stain preview's default, REQ-GUI-152), closing §16's open items, with the "Removed lines" note — moved here from TASK-M8-20 / TASK-M8-19 by R-113.
- Screenshot scenarios for `cargo xtask screenshot 02_stain`.
- `crates/gui/tests/stain_canvas.rs` — driving the canvas edits against the contract graph.

## Acceptance tests
- `cargo xtask screenshot 02_stain` — against the stain-editor artboard (layout only, R-68); `cargo test -p gui stain_canvas_typed_wiring` — arbitrary typed wiring is accepted and type-mismatched wiring rejected through the canvas (REQ-GUI-031).
- `cargo xtask screenshot 02_stain` (scenario remove-brightness) and `cargo test -p gui stain_remove_undo` — remove the brightness node: it shows ghosted; restore restores; undo covers the SetField (REQ-COL-022).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions" — §7 lists the palette's groups and entries; the reviewer checks every ctx field and post op is reachable; the physics reviewer approves the doc change before merge (REQ-GUI-151).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces" — §1 names the preview default and §16's open item is closed; the physics reviewer approves the doc change before merge (REQ-GUI-152).

## Notes
- Definitions (R-72) written here: REQ-GUI-151, REQ-GUI-152 (render_gui_spec §16's node-palette contents and preview default). Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
- R-68: the artboard sets layout; corpus values win.
- RQ-100 ruled: R-113 — REQ-GUI-151 and REQ-GUI-152 move to M7 and are closed here, where the canvas and preview are built.
