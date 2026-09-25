# TASK-M1-09 — Numeric field views: the two-line template, raw debug fields and sentinels shown as their values

- **Milestone:** M1
- **Closes:** REQ-RENDER-022, REQ-RENDER-023, REQ-GEN-012, REQ-TOOL-012, REQ-TOOL-023, REQ-COL-001
- **Depends on:** TASK-M1-08
- **Needs (earlier milestones):** REQ-GEN-001, REQ-GEN-005, REQ-PAY-011
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-8, PIT-3
- **Size:** ~380 lines

## Goal
Every generated numeric field view takes the two-line form — the NaN guard (the bitcast test against the canonical quiet-NaN pattern, never `raw != raw`, R-114) returning `debug_invalid(frag_xy)` (R-136), then `ramp(range_norm(raw, lo, hi, RANGE_AUTO, u_range))` — with `RANGE_AUTO` a node param shared by the code and the graph node. Debug fields are raw: apart from the NaN guard nothing is masked, so a failed-state 0.0 shows as 0.0. Sentinels show as their literal values on the ramp, never scaled (R-79's exception, R-136): the diffusion −1.0 survives pack/unpack bit-exact and renders as −1.0 on the ramp, distinct from NaN, which alone gets the hatch; f16-packed scalars round-trip within f16 eps. Every ramp and compaction carries an explicit, per-node-overridable invalid colour defaulting to REQ-COL-055's hatched invalid pattern (R-132).

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
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-114 — The debug NaN guard is the bitcast test *(closes RQ-82)*"
- `decisions.md` § "R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*"

## Deliverables
- `crates/ledger`: the numeric view template emitter (bitcast NaN guard + `range_norm` ramp, `RANGE_AUTO` as a node param that round-trips through the generated code).
- `crates/render`: `FieldRamp` / `Compaction` invalid-colour lane (default the REQ-COL-055 pattern, per-node override); `dbg_sentinel(x, frag_xy)` for fields whose ledger entry declares a sentinel: the literal value on the ramp, the hatch only for NaN (R-136).
- Golden fixtures `fixtures/golden/m1-numeric/` (NaN-absent ftle, diffusion −1, forced-failure sample, invalid-colour override).

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — generated source matches the two-line template with the bitcast guard and contains no `raw != raw`; toggling `RANGE_AUTO` on the node param updates the generated code and parsing the code back updates the param (REQ-RENDER-022).
- `cargo test -p render debug_fields_raw` — a sample with state = failed and field sentinel 0.0 renders the ramp colour of 0.0, not the invalid colour (REQ-RENDER-023).
- `cargo test -p render diffusion_sentinel` — −1.0 round-trips bit-exact; the catalogue render of −1.0 shows the literal value −1.0 on the ramp (R-136) (REQ-GEN-012).
- `cargo xtask golden m1-numeric` — NaN-absent ftle renders hatched and diffusion = −1 as its literal ramp value (R-136); a forced-failure sample's d_min renders as its literal 0.0 (REQ-TOOL-012).
- `cargo test -p render f16_scalar_views` — d_min, dE_max, dLz_max pack/unpack within f16 eps; sentinel values render as their literal values on the ramp (R-136) (REQ-TOOL-023).
- `cargo xtask golden m1-numeric` — invalid pixels (NaN and sentinel) render the invalid pattern; overriding the node's invalid colour changes only those pixels; a debug field view shows a failed-state 0.0 as literal 0.0 and a NaN as the invalid pattern (REQ-COL-001).

## Notes
- The invalid pattern and the NaN hatch are the prelude's (TASK-M1-03: REQ-COL-055, REQ-TOOL-122).
- PIT-8: NaN absence and the −1 sentinel must stay distinct — two conditions, never folded into one colour: NaN gets the hatch, −1 its literal value on the ramp (R-136).
- RQ-82 ruled: R-114 — the generated guard is the bitcast test against the canonical quiet-NaN pattern (REQ-RENDER-077's bits); `raw != raw` is dropped.
- RQ-94 ruled: R-113 — REQ-RENDER-022 keeps the template and `RANGE_AUTO` as node parameter ↔ code at M1; the node-inspector leg is in REQ-GUI-136's verify (M8).
