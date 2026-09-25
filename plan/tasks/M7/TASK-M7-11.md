# TASK-M7-11 — The stain graph model: node kinds, typed ports, wire rules and RenderState serialisation

- **Milestone:** M7
- **Closes:** REQ-GUI-021, REQ-GUI-022, REQ-GUI-023, REQ-GUI-025, REQ-GUI-026, REQ-GUI-016
- **Depends on:** TASK-M7-01, TASK-M7-03, TASK-M2-23
- **Needs (earlier milestones):** REQ-RENDER-009, REQ-RENDER-075, REQ-GUI-007
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The stain is a data object in the engine crate's contract module: nodes {id, kind, inputs, outputs, params} of kinds source, colour, brightness, combiner, post and OUT, with typed directional ports carrying `field` (subtyped scalar / vector / categorical), `vec3` or `f32`, each kind's ports as render_gui_spec §3.1 tabulates. OUT and combiner are fixed singletons (add, delete and duplicate refused); the free interior takes any number of source, colour, brightness and post nodes wired subject only to type compatibility and acyclicity; a wire attaches only on matching types (field subtype is not a gate); an in-port takes exactly one wire (last write wins) and out-ports fan out without bound. Node ids, wires and per-node params serialise with `RenderState`, and picking a node's occupant is one `SetField`.

## References
- `docs/gui/principia_render_gui_spec.md` § "3. The node graph — model"
- `docs/gui/principia_render_gui_spec.md` § "3.1 Node kinds"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/gui/principia_render_gui_spec.md` § "4. The backbone — fixed vs free"
- `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/gui/principia_render_gui_spec.md` § "6. Port typing & wire rules"
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"

## Deliverables
- `crates/engine/src/contract/stain/graph.rs`, `ports.rs` — the graph, node kinds, port types.
- `crates/engine/src/contract/stain/edit.rs` — add / delete / duplicate / wire / unwire / reorder as typed edits with refusal reasons; each accepted edit is a `SetField` (R-52 undo).
- `RenderState` carrying the graph (serde), replacing the pre-R-64 four-slot object.
- Unit and property tests under `crates/engine/src/contract/tests/`.

## Acceptance tests
- `cargo test -p engine stain_node_kinds` — construct each kind; its port list and types equal §3.1's row (REQ-GUI-021).
- `cargo test -p engine stain_backbone_singletons` — delete / duplicate / add on OUT and combiner are refused; the graph always has exactly one of each (REQ-GUI-022).
- `cargo test -p engine stain_wires_acyclic` (proptest) — random wire sequences: the graph stays acyclic, every accepted wire is type-compatible, and a cycle-closing wire is refused (REQ-GUI-023).
- `cargo test -p engine stain_wire_types` — vec3 → f32 and field → vec3 drops are refused and leave the graph unchanged; matching drops attach; a categorical field onto a gradient colour node is accepted (REQ-GUI-025).
- `cargo test -p engine stain_port_arity` — a second wire dropped on an in-port removes the first; an out-port holds five wires (REQ-GUI-026).
- `cargo test -p engine stain_renderstate_roundtrip` — round-trip a graph through RenderState; choosing an occupant emits one SetField (REQ-GUI-016).

## Notes
- The canonical graph form (REQ-RENDER-075, M1) is the serialised order; this task must not define a second one.
