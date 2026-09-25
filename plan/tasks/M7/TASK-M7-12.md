# TASK-M7-12 — Graph to fragment shader: the walk, None identity, the post chain and the always-renderable fallback

- **Milestone:** M7
- **Closes:** REQ-RENDER-071, REQ-GUI-027, REQ-RENDER-072, REQ-RENDER-068, REQ-GUI-024, REQ-COL-025, REQ-RENDER-081
- **Depends on:** TASK-M7-04, TASK-M7-11
- **Needs (earlier milestones):** REQ-RENDER-005, REQ-RENDER-009, REQ-RENDER-011, REQ-RENDER-075
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
Codegen walks the stain graph — source(s) → optional colour and optional brightness (None = identity) → the required combiner → a possibly-empty ordered chain of at most 8 `vec3 → vec3` post nodes → OUT — to produce the fragment shader's `shade()`. Dangling in-ports left by a deleted mid-graph node render as None and evaluate via occupant identity; the graph is evaluated on every change and is always renderable, a genuinely invalid graph showing a defined fallback and never crashing. One source fans out to many consumers and multi-source nodes expose several input ports. Colour occupants are data, never structure: they cannot alter the topology outside the stain graph, and L is owned by the bound brightness metric.

## References
- `docs/gui/principia_render_gui_spec.md` § "2. The pipeline"
- `docs/gui/principia_render_gui_spec.md` § "4. The backbone — fixed vs free"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions"
- `docs/gui/principia_render_gui_spec.md` § "13. Invariants"
- `docs/design/principia_systems_architecture.md` § "0. The ladder — the organising abstraction"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "The pixel's life = one ladder traversal"
- `docs/gui/principia_render_gui_spec.md` § "5. Sources & multi-source"
- `docs/gui/principia_render_gui_spec.md` § "3.1 Node kinds"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/design/principia_colour_composition.md` § "4.2 Post chain"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/src/codegen/walk.rs` — graph → `shade()` in backbone order, the post chain in declaration order with insert and reorder, the 8-node bound.
- `crates/render/src/codegen/fallback.rs` — identity for dangling / absent inputs, the defined fallback shader for an invalid graph.
- Tests under `crates/render/tests/`, rendering through the M0 GPU test harness.

## Acceptance tests
- `cargo test -p render stain_codegen_wiring` — graphs with each of colour / brightness absent and with 0, 1, 3 posts compile to a fragment shader whose shade() follows the wiring (REQ-RENDER-071).
- `cargo test -p render stain_delete_mid_node` — delete the colour node of a two-slot graph: the output equals greyscale-of-brightness (REQ-GUI-027).
- `cargo test -p render stain_always_renderable` (proptest) — fuzz random graph edit sequences: every state renders; forced-invalid graphs render the fallback (REQ-RENDER-072).
- Review (code): colour occupants cannot alter the pipeline topology outside the stain graph; L is owned by the bound brightness metric (REQ-RENDER-068).
- `cargo test -p render stain_fanout` — picking a vector field sets subtype vector; one source wired to a colour and a brightness node renders both; a margin node has two field ins (REQ-GUI-024).
- `cargo test -p render post_chain_order` — a 9th post node is rejected; reordering two post nodes changes the output accordingly (REQ-COL-025).
- Proposal: the invalid-graph fallback (flat grey or error tint) and its sRGB value, distinguishable from both-None mid-grey and invalid magenta; the human confirms it at the M7 gate (REQ-RENDER-081).

## Notes
- Gap: render_gui_spec §13 gives the invalid-graph fallback as "flat grey / error tint" — which one, and its value, are not given (see the milestone report).
- Closes, for gaps the corpus leaves open: REQ-RENDER-081 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
