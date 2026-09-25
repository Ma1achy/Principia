# TASK-M7-17 — The preset library: presets as whole graphs, debug views as locked presets

- **Milestone:** M7
- **Closes:** REQ-GUI-028, REQ-GUI-029, REQ-GUI-030, REQ-COL-028, REQ-RENDER-061, REQ-RENDER-064, REQ-COL-008
- **Depends on:** TASK-M7-01, TASK-M7-08, TASK-M7-10, TASK-M7-12, TASK-M7-13, TASK-M5-18
- **Needs (earlier milestones):** REQ-GEN-011, REQ-RENDER-022, REQ-RENDER-024, REQ-RENDER-025, REQ-RENDER-027, REQ-TOOL-011, REQ-TOOL-019, REQ-RENDER-053
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
A preset is an entire serialised graph that replaces the current graph wholesale on selection, and the catalogue's default composition is that preset payload — one data source, not two. Debug presets load locked (the first edit forks a custom copy), production presets load editable, and preset → graph is one-way. Bivariate presets load with both combiner inputs occupied. The debug views are presets over the one colouring system — field views as FieldRamp presets, structural views as ctx.quad presets, quad boundary lines as a post-chain bandmask on distance-to-quad-edge — forming a section of the library. Uncertainty marking is an optional fuzziness post node reading the exposed spread (the entropy-desaturation rule is not implemented); map gradient magnitude is a post node (screen-space finite difference over the colour buffer), distinct from the sphere FieldRamp gradient. Quantitative presets document their measure; the combiner gives L to a bound brightness.

## References
- `docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)"
- `docs/design/principia_dd_colouring.md` § "6. Deferred / flagged"
- `docs/notes/principia_sampling_msaa_note.md` § "The core move: colour per sample, average last"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `docs/contracts/principia_gui_state_contract.md` § "8. Amendment to the colouring drill-down"
- `docs/design/principia_colour_composition.md` § "7. Preset table & golden-image obligation"

## Deliverables
- `crates/render/presets/{production,debug}/` — preset files, each holding the catalogue metadata and the graph.
- `crates/render/src/presets.rs` — loader; `crates/engine/src/contract/stain/preset.rs` — locked flag, fork-on-edit.
- The M1–M2 debug catalogue (field views, structural views, UV / DECODE / ROUNDTRIP, agreement presets) re-expressed as presets.
- `crates/render/shaders/wgsl/frag/post/fuzziness.wgsl`, `frag/post/map_gradient.wgsl`.
- Bivariate presets n̂ × FTLE, n̂ × diffusion, n̂ × ensemble spread, n̂ × t_end, outcome-state × FTLE.

## Acceptance tests
- `cargo test -p render preset_load` — load a preset: the graph equals the preset data; catalogue metadata and preset graph come from the same file (REQ-GUI-028).
- `cargo test -p engine preset_locked_fork` — edit a loaded debug preset: a custom copy is created and the preset is unchanged; a production preset loads editable (REQ-GUI-029).
- `cargo test -p render preset_bivariate` — load n̂ × FTLE: combiner colour and brightness inputs are both wired (REQ-GUI-030).
- Review (code, gui): no separate debug render path exists; quad boundaries can be overlaid on a normal preset (REQ-COL-028).
- Review (code): no class-colour code path references entropy; the fuzziness overlay is a removable post node (REQ-RENDER-061).
- Review (code): a post node exists for map gradient magnitude; the sphere gradient-magnitude preset is a FieldRamp over the colour node (REQ-RENDER-064).
- Review (physics): quantitative presets document their measure; the combiner gives L to a bound brightness (REQ-COL-008).

## Notes
- Uses TASK-M7-01's debug taxonomy and addressing channel convention for the debug section's groups.
- The §7.1 map presets are TASK-M7-18.
