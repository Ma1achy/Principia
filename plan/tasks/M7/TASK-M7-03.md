# TASK-M7-03 — The occupant algebra: typed expression trees, the parameter schema and readable WGSL codegen

- **Milestone:** M7
- **Closes:** REQ-COL-011, REQ-GEN-022, REQ-COL-031
- **Depends on:** TASK-M7-02, TASK-M1-05
- **Needs (earlier milestones):** REQ-RENDER-005, REQ-RENDER-006, REQ-RENDER-010, REQ-RENDER-020, REQ-RENDER-075
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
A colour occupant is an expression tree over the two primitive families and the combinators; every node outputs `vec3` or `f32` and only the root is constrained by the slot's signature. The tree compiles to deterministic, readable WGSL: one named function per node with an identity stable across recompiles, a comment carrying the node name and its parameter values, parameters bound as uniforms (a slider edit rebinds, never recompiles) and calls into one shared WGSL library. The parameter schema enforces the adopted ranges L ∈ [0.35, 0.90], C ∈ [0.05, 0.22], κ ∈ [0.5, 12], f ∈ [2, 14], N ∈ [12, 96], ks ∈ [1, 20], s ∈ [0, 1].

## References
- `docs/design/principia_colour_composition.md` § "1. Primitive algebra"
- `docs/design/principia_colour_composition.md` § "5. Codegen & eject"
- `docs/design/principia_colour_composition.md` § "8. Amendments to other docs"
- `docs/design/principia_colour_composition.md` § "1.1 Family A — site-blend  →  `vec3`"

## Deliverables
- `crates/engine/src/contract/stain/occupant.rs` — the expression-tree type and its type-checker (node output type `vec3 | f32`, root checked against the slot); the family and combinator node variants are filled in by TASK-M7-05 to -10.
- `crates/render/src/codegen/mod.rs` — tree → WGSL emitter into the M1 template `[prelude][node functions][shade()]`: per-node function names derived from the node id, a header comment per function (node name, parameter values), emission order from the canonical graph form (REQ-RENDER-075).
- `crates/render/src/codegen/schema.rs` — the uniform schema with the adopted parameter ranges and clamping.
- Tests under `crates/engine/src/contract/tests/` and `crates/render/tests/`.

## Acceptance tests
- `cargo test -p engine occupant_typecheck` — a tree with an f32 node feeding a vec3 root type-checks; an f32 root in the colour slot fails type-checking (REQ-COL-011).
- `cargo test -p render codegen_deterministic` — compiling the same graph twice yields byte-identical WGSL; function names survive an unrelated node edit; each function carries its name/params comment; a slider change rebinds uniforms and the pipeline-compile counter is unchanged (REQ-GEN-022).
- `cargo test -p render param_schema_ranges` — the schema clamps each of L, C, κ, f, N, ks, s to its adopted range (REQ-COL-031).

## Notes
- The M1 assembler (REQ-RENDER-005, -010) is the one compile path; this task adds the tree front end, not a second path.
