# TASK-M8-21 — Stain node inspector: morphing editors, Advanced ▸ edit shader, Graph | Code, eject

- **Milestone:** M8
- **Closes:** REQ-GUI-136, REQ-GUI-137, REQ-GUI-138, REQ-GUI-043, REQ-GUI-040, REQ-GUI-041, REQ-GUI-042
- **Depends on:** TASK-M8-20, TASK-M7-16, TASK-M7-18, TASK-M7-23, TASK-M7-24
- **Needs (earlier milestones):** REQ-RENDER-057, REQ-GUI-017, REQ-GUI-018, REQ-GUI-019, REQ-COL-021, REQ-COL-030, REQ-RENDER-065
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The node inspector chooses its editor from the node kind and, for colour / brightness, the wired source subtype, per §9's table, morphing with no manual toggle. Every inspector ends with Advanced ▸ edit shader: the node's WGSL editable within its function boundary, the { } badge on, graphical controls greyed, revert-to-generated restoring them; per-node and whole-slot eject into the custom-WGSL occupant with dependency tracking, live compile and last-valid fallback. Graph | Code switches representation only: Graph is offered iff graph structure survives, withdrawn after a whole-pipeline edit that dissolves node boundaries, with no one-way dialog. Blend temperature is one sharpness dial with a hard detent that snaps to nearest; support is a discrete vMF / Voronoi choice. The shape-sphere / equirect preview renders at one mass point and says which. Legibility guidance is advice, never enforced.

## References
- `docs/gui/principia_render_gui_spec.md` § "9. Node inspector — morphing editors"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/gui/principia_render_gui_spec.md` § "10. `Graph | Code` — a view toggle, not an eject"
- `docs/design/principia_colour_composition.md` § "5. Codegen & eject"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `docs/design/principia_colour_composition.md` § "1.1 Family A — site-blend  →  `vec3`"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"

## Deliverables
- `crates/gui/src/stain/inspector/{editors,advanced,eject}.rs`, `crates/gui/src/stain/code_view.rs`.
- `crates/render/src/stain/eject.rs` — per-node and whole-slot eject with dependency tracking, revert.
- `crates/gui/src/stain/inspector/sphere_preview.rs` — mass-point label (slice-centre masses, or the inspected pixel's).
- Tests: `editor_morph_table`, `advanced_edit_revert`, `graph_code_toggle`, `eject_node`, `legibility_advice`; screenshot cases `02_stain/blend_dial`, `02_stain/sphere_preview_mass`.

## Acceptance tests
- `cargo test -p gui editor_morph_table` — for each row, select such a node and assert the editor type; rewiring the source morphs the editor (REQ-GUI-136).
- `cargo test -p gui advanced_edit_revert` — edit a node's WGSL: badge on, controls disabled; revert: generated code and controls restored (REQ-GUI-137).
- `cargo test -p gui graph_code_toggle` — per-node edit: toggle round-trips; whole-pipeline edit that removes a function boundary: Graph is disabled; no modal appears (REQ-GUI-138).
- `cargo test -p render eject_node` — eject one node; peers stay generated and live; revert restores generated code; parameter widgets disabled while ejected (REQ-GUI-043).
- Review checklist (gui reviewer), with `cargo test -p gui legibility_advice` — a categorical → brightness wiring is accepted with an advisory note only (REQ-GUI-040).
- `cargo xtask screenshot 02_stain` (blend dial at the top detent; support toggle) — dial at top detent selects nearest; support toggle switches vMF/Voronoi family (REQ-GUI-041).
- `cargo xtask screenshot 02_stain` (preview label; inspecting a pixel changes it) — preview label shows the mass point; inspecting a pixel changes it (REQ-GUI-042).

## Notes
- none
