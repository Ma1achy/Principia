# TASK-M7-14 — Custom per-node WGSL: async compile, last-valid fallback, the Problems surface and eject

- **Milestone:** M7
- **Closes:** REQ-RENDER-057
- **Depends on:** TASK-M7-12, TASK-M7-13
- **Needs (earlier milestones):** REQ-RENDER-010, REQ-RENDER-011, REQ-RENDER-003
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
A node's occupant can become `custom`: a WGSL source for that node, with schema-driven uniforms, asynchronous compilation, fallback to the last valid pipeline on error with per-node failure isolation, and a compile-status indicator and error surface (the Problems model the stain editor shows). Per-node eject turns a generated node function into editable text among generated peers (stable function boundaries, dependency tracking), its parameter widgets greying out, with revert-to-generated as the way back; whole-slot eject is the same mechanism for a whole occupant.

## References
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/design/principia_colour_composition.md` § "5. Codegen & eject"
- `docs/gui/principia_render_gui_spec.md` § "10. `Graph | Code` — a view toggle, not an eject"
- `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"

## Deliverables
- `crates/render/src/custom.rs` — custom-node compile jobs, last-valid bookkeeping per node, the compile-status and diagnostics model (`Problems`).
- `crates/render/src/codegen/eject.rs` — per-node and whole-slot eject, splice-back among generated peers, revert-to-generated.
- Tests.

## Acceptance tests
- `cargo test -p render custom_node_fallback` — invalid WGSL in one node: the previous pipeline keeps rendering, the error appears in Problems against that node, other nodes are unaffected; eject then revert restores the generated function byte-identically (REQ-RENDER-057).

## Notes
- The GUI side of Problems and the `{ }` badge are drawn in TASK-M7-22 and -24.
