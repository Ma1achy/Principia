# TASK-M8-20 — Stain canvas: selection, wiring gestures, the node palette, node visuals

- **Milestone:** M8
- **Closes:** REQ-GUI-130, REQ-GUI-131, REQ-GUI-132, REQ-GUI-133, REQ-GUI-134, REQ-GUI-135, REQ-GUI-151
- **Depends on:** TASK-M8-19
- **Needs (earlier milestones):** REQ-GUI-021, REQ-GUI-022, REQ-GUI-023, REQ-GUI-024, REQ-GUI-025, REQ-GUI-026, REQ-GUI-027, REQ-GEN-019
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The canvas supports §7's gestures: click-select opening the inspector, click-empty deselect, marquee multi-select, node drag with wires following and multi-selected nodes moving together, space- / middle-drag pan and scroll zoom. While a wire is dragged, compatible in-ports highlight and incompatible ones dim; release on empty canvas opens a node-create menu filtered to what accepts the dragged type. A wire is removed by dragging it off near its head or selecting it and pressing Delete. Right-click opens the categorised node palette (sources by ctx group; colour / brightness / post), whose contents are defined in §7; the library is not a source of single nodes. Field subtype never gates a wire: a categorical field dropped on a gradient colour node is accepted and the inspector morphs to the palette editor, flagging unmappable params. Each node box carries §8's title bar, state glyph, typed colour-coded ports, the { } badge and the selection outline.

## References
- `docs/gui/principia_render_gui_spec.md` § "6. Port typing & wire rules"
- `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/gui/principia_render_gui_spec.md` § "8. Node visuals — glyph-forward"
- `docs/gui/principia_render_gui_spec.md` § "16. Open / next"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/gui/src/stain/canvas/{select,drag,wire,palette}.rs`, `crates/gui/src/stain/node_box.rs`.
- Doc change: `docs/gui/principia_render_gui_spec.md` Part II §7 (the palette's groups and entries).
- Tests: `canvas_gestures`, `wire_drag_filter`, `wire_remove`, `node_palette`, `subtype_morph`; screenshot case `02_stain/nodes`.

## Acceptance tests
- `cargo test -p gui subtype_morph` — rewire a gradient colour node from scalar to categorical: the editor is the palette editor and unmappable params are flagged (REQ-GUI-130).
- `cargo test -p gui canvas_gestures` — scripted pointer events for each gesture produce the stated selection / position change (REQ-GUI-131).
- `cargo test -p gui wire_drag_filter` — drag from a vec3 out-port to empty canvas: the menu lists only post nodes and OUT-compatible inputs; f32 in-ports are dimmed (REQ-GUI-132).
- `cargo test -p gui wire_remove` — both gestures remove the wire and the in-port reverts to None (REQ-GUI-133).
- `cargo test -p gui node_palette` — the palette's source group lists ctx fields by group; the library only loads whole graphs (REQ-GUI-134).
- `cargo xtask screenshot 02_stain` (one hand-edited and one selected node) — screenshot against 02_stain.png with one hand-edited and one selected node (REQ-GUI-135).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions" — §7 lists the palette's groups and entries; the reviewer checks every ctx field and post op is reachable; the physics reviewer approves the doc change before merge (REQ-GUI-151).

## Notes
- The { } badge's hand-edited state comes from TASK-M8-21; this task's screenshot case sets it through the graph model directly.
- Definitions (R-72) written here: REQ-GUI-151. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
