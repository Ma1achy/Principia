# TASK-M1-09 — Numeric field views: the two-line template, raw debug fields and sentinel styling

- **Milestone:** M1
- **Closes:** REQ-RENDER-022, REQ-RENDER-023, REQ-GEN-012, REQ-TOOL-012, REQ-TOOL-023, REQ-COL-001
- **Depends on:** TASK-M1-08
- **Needs (earlier milestones):** REQ-GEN-001, REQ-GEN-005, REQ-PAY-011
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-8, PIT-3
- **Size:** ~380 lines

## Goal
Every generated numeric field view takes the two-line form — the NaN guard returning `DEBUG_NAN`, then `ramp(range_norm(raw, lo, hi, RANGE_AUTO, u_range))` — with `RANGE_AUTO` a node param shared by the code and the graph node. Debug fields are raw: apart from the NaN guard nothing is masked, so a failed-state 0.0 shows as 0.0. Sentinels are styled, never scaled: the diffusion −1.0 survives pack/unpack bit-exact and renders in the sentinel style, distinct from the NaN absence style; f16-packed scalars round-trip within f16 eps. Every ramp and compaction carries an explicit, per-node-overridable invalid colour defaulting to magenta.

## References
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)"
- `docs/design/principia_debug_tooling_plan.md` § "H. Codegen self-test (the tooling that tests the tooling)"
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `decisions.md` § "R-16 — The colour PDF's map lists are ported *(closes RQ-14)*"
- `docs/gui/principia_render_gui_spec.md` § "13. Invariants"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"

## Deliverables
- `crates/ledger`: the numeric view template emitter (NaN guard + `range_norm` ramp, `RANGE_AUTO` as a node param that round-trips through the generated code).
- `crates/render`: `FieldRamp` / `Compaction` invalid-colour lane (default magenta, per-node override); sentinel styling via `dbg_sentinel` for fields whose ledger entry declares a sentinel.
- Golden fixtures `fixtures/golden/m1-numeric/` (NaN-absent ftle, diffusion −1, forced-failure sample, invalid-colour override).

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — generated source matches the two-line template; toggling `RANGE_AUTO` on the node param updates the generated code and parsing the code back updates the param (REQ-RENDER-022).
- `cargo test -p render debug_fields_raw` — a sample with state = failed and field sentinel 0.0 renders the ramp colour of 0.0, not the invalid colour (REQ-RENDER-023).
- `cargo test -p render diffusion_sentinel` — −1.0 round-trips bit-exact; the catalogue render of −1.0 uses the sentinel style (REQ-GEN-012).
- `cargo xtask golden m1-numeric` — NaN-absent ftle and diffusion = −1 render the two distinct sentinel stylings; a forced-failure sample's d_min renders as its literal 0.0 (REQ-TOOL-012).
- `cargo test -p render f16_scalar_views` — d_min, dE_max, dLz_max pack/unpack within f16 eps; sentinel values render in the sentinel style (REQ-TOOL-023).
- `cargo xtask golden m1-numeric` — invalid pixels (NaN and sentinel) render magenta; overriding the node's invalid colour changes only those pixels; a debug field view shows a failed-state 0.0 as literal 0.0 and a NaN as magenta (REQ-COL-001).

## Notes
- REQ-RENDER-022 names three places `RANGE_AUTO` is edited — the node inspector, the graph node, the code. The node inspector is GUI (M8); at M1 the node param and the code are the two representations, and the inspector binds to the node param when it lands (milestone Gaps).
- render_gui_spec §10.1's guard is `if (raw != raw)` — a NaN self-comparison, which render_contract Part 2 / Part 4 say fast-math may elide (use an exact bitcast test). Which form the generated guard takes is not ruled (milestone Gaps); REQ-RENDER-015's lint (TASK-M1-15) depends on it.
- The magenta value and the NaN hatch are the prelude's (TASK-M1-03) and wait on the same gaps.
- PIT-8: the NaN absence style and the −1 sentinel style must stay distinct — two conditions, never folded into one colour.
- Waits on RQ-82 (`REVIEW_QUEUE.md`): The generated debug NaN guard vs the bitcast rule.
- Waits on RQ-94 (`REVIEW_QUEUE.md`): M1 requirements that need M2, M3, M5 or M8.
