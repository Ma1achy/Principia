# TASK-M1-09 — Numeric field views: the two-line template, raw debug fields and sentinels shown as their values

- **Milestone:** M1
- **Closes:** REQ-RENDER-022, REQ-RENDER-023, REQ-GEN-012, REQ-TOOL-012, REQ-TOOL-023, REQ-COL-001, REQ-TOOL-137, REQ-COL-053
- **Depends on:** TASK-M1-08
- **Needs (earlier milestones):** REQ-GEN-001, REQ-GEN-005, REQ-PAY-011
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-8, PIT-3
- **Size:** ~650 lines (~250 more if it is the first of TASK-M1-09 and TASK-M1-13 to start and builds RQ-229's harness case kind)

## Goal
Every generated numeric field view takes the two-line form — the NaN guard (the bitcast test against the canonical quiet-NaN pattern, never `raw != raw`, R-114) returning `debug_invalid(frag_xy)` (R-136), then `ramp(range_norm(raw, lo, hi, RANGE_AUTO, u_range))` — with `RANGE_AUTO` a node param shared by the code and the graph node. `raw` is the field's value compacted per its ledger scale, `ramp` is `ramp_viridis`, and `RANGE_AUTO` is a `u32` uniform in the view's header (RQ-231). Debug fields are raw: apart from the NaN guard nothing is masked, so a failed-state 0.0 shows as 0.0. Stored sentinels (`dmin_pair` 3, the word `length` 127) show as their literal values on the ramp, never scaled (R-79's exception, R-136), with one exception: `d_min`'s unset value (f16 +inf, R-271) is drawn in the neutral "not yet" grey of running samples (R-280), whose value this task proposes (REQ-COL-053; RQ-233). An invalid diffusion fit (n < 2) is no sentinel: it reads NaN and gets the hatch, distinct from a valid slope (R-245; RQ-230). NaN alone gets the hatch; f16-packed scalars round-trip within f16 eps. Every ramp and compaction carries an explicit, per-node-overridable invalid colour defaulting to REQ-COL-055's hatched invalid pattern (R-132); this task builds the minimal `FieldRamp`, `ScalarField` and `Compaction` that carry it at M1 (RQ-232).

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
- `decisions.md` § "R-136 — `debug_invalid(frag_xy)` draws the hatch; sentinels show their value *(closes RQ-114)*"
- `decisions.md` § "R-245 — An invalid diffusion fit reads NaN, by a validity predicate; no −1.0 sentinel *(amends R-17, R-136; closes RQ-152)*"
- `decisions.md` § "R-253 — `ftle` reads NaN at `step_count = 0`, by the predicate `step_count ≥ 1`; no sentinel *(closes RQ-154)*"
- `decisions.md` § "R-254 — `ftle` reads NaN whenever `ftle_valid` is false *(refines R-253)*"
- `decisions.md` § "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*"
- `decisions.md` § "R-280 — An unset `d_min` renders in the neutral "not yet" grey *(closes RQ-170)*"
- `decisions.md` § "R-343 — The fragment unpack layer binds `SimStateFTLE` at `@group(1) @binding(0)` and the word buffer at `@group(1) @binding(1)`; WGSL forms of `closure_step` and the schema version *(closes RQ-188)*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-381 — The drift views offer `symlog`, `lin` and `log`, with `symlog` the default; the value fed to `dbg_sentinel` is the compacted value *(closes RQ-206)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default palette"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"

## Deliverables
- `crates/ledger`: the numeric view template emitter (bitcast NaN guard + `range_norm` ramp, `RANGE_AUTO` as a node param that round-trips through the generated code), with RQ-231's scale handling: `raw` is the field's compacted value per its ledger scale — `lin` the identity, `log` the field's floor form as `dbg_log` places it, `cyclic` `fract(x/period)` on the fixed range [0, 1] with `ramp_twilight`, `diverging` a symmetric range; a field whose ledger range is unbounded defaults to `RANGE_AUTO = 1`, its fixed mode that range's finite bound where one exists; `t_end_step` and `t_dmin_step` use `[0, horizon_steps]` (§10.1); `ramp` is `ramp_viridis`. `energy_drift` and `Lz_drift` keep R-381's `symlog` default, so they stay TASK-M1-08's view until TASK-M3-05.
- `crates/render`: the views use TASK-M1-03's `dbg_sentinel` (`present.wgsl`) for the literal placement; the hatch only for NaN (R-136).
- `crates/render/src/colour/field_ramp.rs` (RQ-232): the minimal `FieldRamp` over one `ScalarField` (a payload field with its validity predicate, returning `(value, valid)`), a `lin` `Compaction`, viridis, and the invalid-colour lane (default the REQ-COL-055 pattern, per-node override). TASK-M7-05 and TASK-M7-09 extend these files.
- The running-sample grey (RQ-233): one named constant in the presentation layer (`crates/render/shaders/wgsl/lib/present.wgsl` and its CPU mirror), which TASK-M1-10 reads. The COL-053 proposal: the grey's sRGB, its OKLab lightness and its separation from all nine classes and from the invalid pattern's two colours (REQ-COL-055), attached to the PR for the human's confirmation at the M1 gate; TASK-M1-10's REQ-COL-062 proposal measures the two triple-outcome swatches against it.
- If no earlier task has built it (RQ-229): `cargo xtask golden`'s harness case kind and its `validation` binary. A case's `case.json` names a synthetic scene; the runner spawns the binary, which renders the scene through `render::bind::preset_module` and `upload` and writes the `Rgba32Float` image; the runner quantises and compares it in its own pass as today (REQ-VAL-138, `BASELINES.md` and `repro` unchanged; no crate edge, as `gate` and `screenshot` spawn theirs). Otherwise this task uses the case kind TASK-M1-13 built.
- Golden fixtures `fixtures/golden/m1-numeric/` (NaN-absent ftle, a diffusion with n < 2, a forced-failure sample, an unstepped sample, a valid `d_min`, the `d_min` field ramp, invalid-colour override), as harness cases; each case's `BASELINES.md` row cites R-369 and RQ-229, proposed and confirmed at the M1 gate.

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — generated source matches the two-line template with the bitcast guard and contains no `raw != raw`; each ledger scale (`lin`, `log`, `cyclic`, `diverging`) generates RQ-231's `raw`; a field with an unbounded range defaults to `RANGE_AUTO = 1`; setting `RANGE_AUTO` on the node param writes the header default `@uniform RANGE_AUTO: u32 = <0|1> [0, 1]` in the generated code, and `Declaration::parse` of that header gives the param back (REQ-RENDER-022).
- `cargo test -p render debug_fields_raw` — a sample with state = failed and field sentinel 0.0 renders the ramp colour of 0.0, not the invalid colour (REQ-RENDER-023).
- `cargo test -p render diffusion_invalid_fit` — a diffusion with n < 2 reads NaN and renders the hatch, distinct from a valid slope, which renders on the ramp; the stored sentinels `dmin_pair` 3 and `length` 127 round-trip bit-exact and the catalogue render of each shows its literal value on the ramp (R-136); `d_min`'s unset +∞ is drawn in the grey, not on the ramp (R-280) (REQ-GEN-012).
- `cargo xtask golden m1-numeric` — NaN-absent ftle and a diffusion with n < 2 render hatched (R-245); a forced-failure sample's dE_max and dLz_max render as their literal 0.0; a forced-failure sample and an unstepped sample hold `d_min`'s unset value f16 +inf (R-271), read by its bits (the bit test the harness's validity lane generates, or `pa_d_min_is_unset` on the packed word, never a float comparison; R-343 item 5) and drawn in the neutral "not yet" grey of running samples, not hatched and not on the ramp (R-96, R-280); a NaN in the same view is hatched; a valid d_min sits on the ramp (REQ-TOOL-012, REQ-TOOL-137).
- `cargo test -p render f16_scalar_views` — d_min, dE_max, dLz_max pack/unpack within f16 eps; the stored sentinels render as their literal values on the ramp (R-136), and `d_min`'s unset value in the grey (R-280) (REQ-TOOL-023).
- `cargo xtask golden m1-numeric` — `d_min`'s field ramp (a non-debug `FieldRamp`) renders NaN and the unset sentinel in the invalid pattern; overriding that node's invalid colour changes only those pixels; the debug view of the same field shows the stored values literally, a failed-state 0.0 as literal 0.0, and a NaN as the invalid pattern (REQ-COL-001).
- Review checklist (gui, qa): the proposal shows the grey's OKLab lightness and its separation from bounded #141418, degenerate #ECECF0 and the other seven classes and from the invalid pattern's two colours; the human confirms the value at the M1 gate and it is recorded in decisions.md (REQ-COL-053).

## Notes
- The invalid pattern and the NaN hatch are the prelude's (TASK-M1-03: REQ-COL-055, REQ-TOOL-122).
- PIT-8: NaN and a stored sentinel must stay distinct — two conditions, never folded into one colour: NaN (including an invalid diffusion fit, R-245) gets the hatch; a stored sentinel its literal value on the ramp (R-136); `d_min`'s unset value the grey (R-280).
- RQ-82 ruled: R-114 — the generated guard is the bitcast test against the canonical quiet-NaN pattern (REQ-RENDER-077's bits); `raw != raw` is dropped.
- Applied per R-271 (30 Sep 2026): a failed sample's `d_min` is +inf, not 0.0, so it can't render as a literal 0.0. RQ-170 asked how an unset `d_min` renders; R-280 rules the neutral "not yet" grey of running samples (R-96). PIT-8 still holds: NaN alone gets the hatch.
- RQ-94 ruled: R-113 — REQ-RENDER-022 keeps the template and `RANGE_AUTO` as node parameter ↔ code at M1; the node-inspector leg is in REQ-GUI-136's verify (M8).
- RQ-229 to RQ-233 and RQ-241, decided per R-369 (7 Oct 2026): the golden suites render through a harness case kind (RQ-229); R-245's and R-280's consequences (RQ-230); the template's scales, ranges, ramp and `RANGE_AUTO` uniform (RQ-231); the minimal FieldRamp (RQ-232); REQ-COL-053 moved here from TASK-M1-10, with gui added as a reviewer for its checklist (RQ-233); the References (RQ-241).
- REQ-COL-053 is a calibration (R-71): the value is proposed with evidence here and confirmed by the human at the M1 gate; an unconfirmed calibration blocks the gate.
- `meas` for `auto = true` is TASK-M1-03's: at M1, `u_range` is a `vec2<f32>` uniform filled from a CPU min/max over the synthetic buffer.
