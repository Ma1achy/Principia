# TASK-M8-20 — Stain canvas: selection, wiring gestures, the node palette, node visuals

- **Milestone:** M8
- **Closes:** REQ-GUI-130, REQ-GUI-132, REQ-GUI-133, REQ-GUI-134, REQ-GUI-135
- **Depends on:** TASK-M8-19, TASK-M6-29
- **Needs (earlier milestones):** none
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
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"

## Deliverables
- `crates/gui/src/stain/canvas/{select,drag,wire,palette}.rs`, extending the track's canvas and its gestures (TASK-M6-29, R-390), and `crates/gui/src/stain/node_box.rs`.
- Tests: `canvas_gestures`, `wire_drag_filter`, `wire_remove`, `node_palette`, `subtype_morph`; screenshot case `02_stain/nodes`.

## Acceptance tests
- `cargo test -p gui subtype_morph` — rewire a gradient colour node from scalar to categorical: the editor is the palette editor and unmappable params are flagged (REQ-GUI-130).
- `cargo test -p gui wire_drag_filter` — drag from a vec3 out-port to empty canvas: the menu lists only post nodes and OUT-compatible inputs; f32 in-ports are dimmed (REQ-GUI-132).
- `cargo test -p gui wire_remove` — both gestures remove the wire and the in-port reverts to None (REQ-GUI-133).
- `cargo test -p gui node_palette` — the palette's source group lists ctx fields by group; the library only loads whole graphs (REQ-GUI-134).
- `cargo xtask screenshot 02_stain` (one hand-edited and one selected node) — screenshot against 02_stain.png with one hand-edited and one selected node (REQ-GUI-135).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines, every contract field this task adds among its cases (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here; R-627).

## Notes
- The { } badge's hand-edited state comes from TASK-M8-21; this task's screenshot case sets it through the graph model directly.
- RQ-100 ruled: R-113 — REQ-GUI-151 (the palette contents) moved to M7 (TASK-M7-22); this task implements the palette §7 now defines.
- R-390: TASK-M6-29 builds the canvas gestures on the mock engine and closes REQ-GUI-131; this task depends on it, re-runs REQ-GUI-131's acceptance on the real stain graph and keeps every other requirement it closes.
- R-627 (27 Oct 2026), a correction beside the note above, which stands as its record: re-running REQ-GUI-131's acceptance on the real stain graph is TASK-M8-93's; this task builds the gestures on the mock (code review 5664955153 of R-627's port, applied per R-369).
- **R-627 (27 Oct 2026), the GUI track in M8:** this task is built on the mock engine, as R-390's track is: on spare agent slots, never holding up the physics chain, and merging once its reviews pass and CI is green without waiting for the gates of M3 to M6 (the gates of M2 and M7 by R-415's test, recorded in the PR; R-627's flag). Its dependence on TASK-M6-21 and TASK-M6-22, which it had through TASK-M8-01, is satisfied by the mock's precision events and quality tier. A control that needs a contract field adds it as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on both engines. TASK-M8-93 wires it to the real engine after TASK-M6-21 and TASK-M6-22, re-running every acceptance line here on the real engine with the conformance suite on both, and takes the dependencies this task gave up. **What to try.** The PR description has a "What to try" section: the clicks and keys that show what changed, on `cargo run -p gui --features mock`. It builds on the contract's stain graph (`engine::stain`), its kinds, ports and wire rules, on the mock; the Needs closed by TASK-M7-11, TASK-M7-12, TASK-M7-13 and TASK-M7-22 move to TASK-M8-93, with the REQ-GUI-131 line re-run on the real engine. *Was:* "- **Needs (earlier milestones):** REQ-GUI-021, REQ-GUI-022, REQ-GUI-023, REQ-GUI-024, REQ-GUI-025, REQ-GUI-026, REQ-GUI-027, REQ-GEN-019, REQ-GUI-151". *Was (acceptance lines, moved verbatim):* "- `cargo test -p gui canvas_gestures` — scripted pointer events for each gesture produce the stated selection / position change (REQ-GUI-131). Closed by TASK-M6-29 on the mock engine since R-390; this task re-runs it on the real engine." (to TASK-M8-93).
