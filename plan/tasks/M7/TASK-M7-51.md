# TASK-M7-51 — The stain editor's Chart position: the node, the point⟨q⟩ subtype, the base-and-layers inspector, switching and restoring, Problems, the presets and saved stains unchanged (R-542 SM15)

- **Milestone:** M7
- **Closes:** REQ-GUI-335, REQ-GUI-336, REQ-GUI-337, REQ-GUI-338, REQ-GUI-340
- **Depends on:** TASK-M7-48, TASK-M7-49, TASK-M7-17, TASK-M7-22, TASK-M7-23, TASK-M7-24
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~800 lines, the tests included

## Goal
Chart position in the stain editor, as SM15 and the art boards place it: one source node beside the unchanged Shape n̂, the `point⟨q⟩` subtype, the base-and-layers inspector, switching the source or chart and back restoring every setting, the Problems warnings and counts, the seven presets, and every saved stain rendering as before.

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM15. In the stain graph editor"
- `docs/design/principia_state_map.md` § "SM11. Colouring Chart position: a base, then layers"
- `docs/gui/principia_render_gui_spec.md` § "3. The node graph — model"
- `docs/gui/principia_render_gui_spec.md` § "3.1 Node kinds"
- `docs/gui/principia_render_gui_spec.md` § "6. Port typing & wire rules"
- `docs/gui/principia_render_gui_spec.md` § "9. Node inspector — morphing editors"
- `docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs"
- `docs/design/principia_state_map.md` § "SM7. Colour frame and deep zoom"
- `docs/design/principia_state_map.md` § "SM8. Image mode"
- `docs/design/principia_state_map.md` § "SM10. Time, ensemble and fade"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"

## Deliverables
- The inspector's colour frame, Picture choice and upload, and fade controls (`crates/gui/`; REQ-GUI-340); test `state_map_picture_controls`, screenshot `state_map_picture`.
- The node, the inspector, the switch-restore, the Problems rows and the presets (`crates/gui/`).
- Screenshots `state_map_editor`, `state_map_problems`, `state_map_presets`; tests `chart_position_switch`, `state_map_problems`, `state_map_presets`, `saved_stains_unchanged`.

## Acceptance tests
- `cargo xtask screenshot state_map_picture` (board 3's Picture inspector) and `cargo test -p gui state_map_picture_controls` — REQ-GUI-340's verify.
- `cargo xtask screenshot state_map_editor` (boards 1, 3 and 4 of docs/gui/reference/state_map/boards/) and `cargo test -p gui chart_position_switch` — REQ-GUI-335's verify.
- `cargo xtask screenshot state_map_problems` (board 1's Problems rows on the prototype's settings) and `cargo test -p gui state_map_problems` — REQ-GUI-336's verify.
- `cargo xtask screenshot state_map_presets` (board 5) and `cargo test -p gui state_map_presets` — REQ-GUI-337's verify.
- `cargo test -p gui saved_stains_unchanged` — REQ-GUI-338's verify.

## Notes
- **REQ-GUI-340 (gui re-review G6 of PR #270; applied per R-369):** the colour frame, Picture choice and upload, and fade controls go in this task's inspector, moved from TASK-M7-49. The upload is labelled "Your image…", distinct from File › Import picture.
- The gui reviewer reviews against `docs/gui/reference/state_map/boards/` (boards 1 to 5; the human's message: "the GUI ones go with `boards/`").
- **Applied meanwhile:** RQ-497 (a): Site blend ↔ Site blend the only swap, §6's morphing kept between subtypes, the file's "Palette" unqualified until ruled; RQ-499 (a), the code's preset names (Where now, Basin tiles, Picture flow); RQ-502 (a), one colour node for the base and its layers; RQ-505 (a), "none" an ordinary setting.
- **The boards** are read per render GUI spec §G13's R-542 bullet: their new elements only, the shell following `02_stain.png`; board 1's Problems-row buttons and board 4's "Set both to this chart" are board-only, not built (R-542 H14).
- **The Problems counts** come from the engine through the snapshot, by the legend's path (R-467 D5), never computed in the GUI (gui_state_contract §1).
