# TASK-M7-04 — Combiners: Replace-L, Multiply and None as the identity of combine

- **Milestone:** M7
- **Closes:** REQ-COL-009, REQ-COL-038, REQ-COL-023
- **Depends on:** TASK-M7-02, TASK-M7-03
- **Needs (earlier milestones):** REQ-RENDER-009, REQ-RENDER-016
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
The combiner implements Replace-L — base RGB → OKLab, L ← L_min + (L_max − L_min)·b with defaults L_min = 0, L_max = 1 (so the default is L = b, R-77), (a, b) untouched, → RGB — and Multiply (rgb·b in linear space). A bound brightness owns L and the colour occupant contributes hue and chroma only. `combine` treats `None` as its identity per colour_composition §4.1's truth table: C + None → C; None + B → OKLab(L = B, 0, 0) or white·B; None + None → flat mid-grey OKLab(0.6, 0, 0).

## References
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_dd_colouring.md` § "3.5 Combiners"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `decisions.md` § "R-77 — Replace-L, and the state palette *(closes RQ-28)*"
- `docs/gui/principia_render_gui_spec.md` § "13. Invariants"
- `docs/gui/principia_render_gui_spec.md` § "2. The pipeline"

## Deliverables
- `crates/render/shaders/wgsl/frag/combiner/replace_l.wgsl`, `multiply.wgsl` (scanned occupants, the `combine(rgb, b) → vec3` signature).
- `crates/render/src/codegen/combine.rs` — the `Option` handling in the generated `shade()` that applies the truth table.
- `crates/render/src/colour/combine.rs` — Rust mirror for the tests.

## Acceptance tests
- `cargo test -p render combiner_replace_l` — dd_colouring unit test 6: Replace-L leaves (a, b) bit-stable; at default L_min/L_max the output L equals b exactly; a non-default range maps b = 0/1 to L_min/L_max; Multiply preserves channel ratios (REQ-COL-038).
- `cargo test -p render combiner_l_ownership` — Replace-L output's OKLab L equals the brightness-derived L regardless of the colour occupant (REQ-COL-009).
- `cargo test -p render combiner_none_identity` — each of the four colour/brightness combinations under Replace-L and Multiply produces the tabled result (REQ-COL-023).

## Notes
- The monotone-L LUT × Replace-L conflict flag is a UI wiring test; it lands with the node inspector (TASK-M7-23).
