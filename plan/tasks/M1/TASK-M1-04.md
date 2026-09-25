# TASK-M1-04 — The stain graph lowered to WGSL: backbone, canonical form and generated shade()

- **Milestone:** M1
- **Closes:** REQ-RENDER-009, REQ-RENDER-010, REQ-RENDER-012, REQ-RENDER-016, REQ-RENDER-075, REQ-GEN-027
- **Depends on:** TASK-M1-01, TASK-M1-03, TASK-M0-16
- **Needs (earlier milestones):** REQ-SYS-003, REQ-SYS-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
A stain is a free, typed node graph on the fixed backbone — sources → colour / brightness → combiner → (post)* → OUT, acyclic — and it lowers to one fragment source: the shared prelude, one function per node, and a generated `shade()` that walks the graph. Built-in, debug and custom occupants go through the same single assembler entry point; colour returns linear-RGB `vec3`, brightness `f32` nominal [0,1]; occupants read the fixed read-side `SimState` by plain member access at every tier. The graph's canonical form (node order, wiring, post-chain encoding) is defined in the docs, and the fragment key hashes it.

## References
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"
- `docs/design/principia_colour_composition.md` § "4. Pipeline shape"
- `docs/design/principia_dd_colouring.md` § "1. What it is"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"
- `docs/contracts/principia_render_contract.md` § "Part 3 — Cache tiers and the recompute rule"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/gui/principia_render_gui_spec.md` § "3. The node graph — model"
- `docs/gui/principia_render_gui_spec.md` § "4. The backbone — fixed vs free"
- `docs/gui/principia_render_gui_spec.md` § "6. Port typing & wire rules"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `docs/design/principia_colour_composition.md` § "4.2 Post chain"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/gui/principia_render_gui_spec.md` § "13. Invariants"
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `decisions.md` § "R-53 — Node interfaces declare their input domains *(GU-2 (a))*"

## Deliverables
- `crates/engine`: the stain-graph type (nodes, typed ports, wires, per-node params) and its canonical form.
- `crates/render/src/assemble.rs`: the one assembler entry point — graph → `[prelude][node functions][shade()]`; backbone and acyclicity enforced at construction.
- Doc change (R-72): the canonical form written into `docs/contracts/principia_lowering_contract.md` Part 5 and `docs/contracts/principia_render_contract.md` Part 3, with the porting rule's "Removed lines" note.
- Tests: `crates/render/tests/assemble.rs`.

## Acceptance tests
- Review checklist (code): the generated `shade()` follows the backbone order; no configuration can reorder the backbone or feed post back into colour — a test graph that tries is rejected (REQ-RENDER-009).
- Review checklist (code): built-in occupants load through the same assembler entry point as a user custom; no second compile path exists (REQ-RENDER-010).
- `cargo test -p render custom_reads_any_tier` — a custom occupant reading `sample.ftle`, `sample.ensemble_spread` and `sample.word` compiles and runs at every tier variant (REQ-RENDER-012).
- Review checklist (code): the colour slot signature returns linear-RGB `vec3`; brightness returns `f32` (REQ-RENDER-016).
- Review checklist (physics): the docs state the canonical form; `cargo test -p render canonical_hash` — two wirings of the same graph hash equal and different graphs hash differently (REQ-RENDER-075).
- Definition: the `.wgsl` uniformSchema / inputDomains declaration format written into gui_state_contract §3 and approved by the physics reviewer (REQ-GEN-027).

## Notes
- REQ-RENDER-075 is a definition requirement (R-72): the doc change is part of this PR and the physics reviewer approves it before merge.
- Occupant identity (render_gui_spec §13): dangling inputs fall back to None, so the assembler never produces an unrenderable graph.
- The Replace-L combiner's colour maths is M7; M1 needs only a pass-through combiner for single-colour debug stains.
- Closes, for gaps the corpus leaves open: REQ-GEN-027 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
