# TASK-M7-21 — The display chain: style → display scale → gamut clamp → colour-vision simulation → screen

- **Milestone:** M7
- **Closes:** REQ-RENDER-063, REQ-RENDER-069, REQ-COL-043, REQ-RENDER-066, REQ-COL-059
- **Depends on:** TASK-M7-12, TASK-M7-20, TASK-M5-12, TASK-M5-23, TASK-M5-26
- **Needs (earlier milestones):** REQ-RENDER-042, REQ-RENDER-052, REQ-RENDER-038, REQ-RENDER-006, REQ-RENDER-047
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
After the stain, the fixed display chain runs style → render→display scale → gamut clamp → colour-vision simulation → screen (R-67): every stage before the scale at render resolution, the scale step a bilinear upscale below native and a box downsample above (a no-op at native), so the simulation sees the final in-gamut, display-scaled colours. The compositor (blurred backdrop, fresh cover, trace/overlays) sits above the colour stage and blur is a compositor pass; occupants emit layer content, so per-layer CVD equals post-composite CVD for opaque quads.

## References
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "The pixel's life = one ladder traversal"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/principia_render_gui_spec.md` § "12. Display stage — global, outside the pipeline"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `decisions.md` § "R-107 — Apply the RQ-67 follow-ups; GUI_DESIGN_NOTES may be conformed *(closes RQ-67)*"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

- `decisions.md` § "R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*"
## Deliverables
- `crates/render/src/display/chain.rs` — the fixed pass order; style is the identity (plain) until TASK-M7-26.
- `crates/render/shaders/wgsl/compositor/display_scale.wgsl`, `gamut_clamp.wgsl`.
- `fixtures/golden/display-chain/` and the `display-chain` golden suite.

## Acceptance tests
- `cargo test -p render display_chain_order` — with render_scale ≠ 1 and a CVD mode set, the CVD matrix is applied to the post-scale, post-clamp buffer; the scale step is a no-op at native (REQ-RENDER-063).
- `cargo xtask golden display-chain` — CVD simulation is applied to the final in-gamut, display-scaled colours; pass order asserted (REQ-RENDER-069).
- `cargo test -p render cvd_sees_in_gamut` — with an out-of-gamut stain output and CVD on, the CVD stage's input is in gamut and equals gamut_clamp(display_scale(style(stain))) (REQ-COL-043).
- `cargo test -p render cvd_layer_linearity` (proptest) — seam 7 linearity check on random opaque layers: per-layer CVD equals post-composite CVD (REQ-RENDER-066).
- Definition: the gamut-clamp method written into colour_composition §4.3 (identity on in-gamut colours) and approved by the physics reviewer (REQ-COL-059).

## Notes
- Gap: the gamut-clamp method (per-channel clip, or a chroma-reducing clamp in OKLCH) is not given.
- RQ-80 ruled: R-111 — the display chain names `SimState`, not `SimResult` (REQ-COL-043; render_gui_spec and GUI_DESIGN_NOTES renamed in step 7); the vocabulary lint (TASK-M0-16) covers the docs.
- Closes, for gaps the corpus leaves open: REQ-COL-059 (R-72 definition) (classification accepted by R-132).
