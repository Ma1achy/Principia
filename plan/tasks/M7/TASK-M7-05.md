# TASK-M7-05 — Compaction, ramps and the default ramp per field role

- **Milestone:** M7
- **Closes:** REQ-COL-039, REQ-COL-016, REQ-COL-032, REQ-COL-024
- **Depends on:** TASK-M7-03, TASK-M7-04, TASK-M1-09
- **Needs (earlier milestones):** REQ-COL-001, REQ-GEN-001, REQ-GEN-002, REQ-GEN-012, REQ-RENDER-021
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Family B's mapping half: the Compaction forms lin, log (x ≤ 0 → 0 with sentinel styling), cyclic (frac(x/period)), diverging symlog b = ½ + ½·sign(x)·ln(1+|x|/x₀)/ln(1+x_max/x₀) with the ε floor as x₀, and flag; the Ramp kinds lut, lerp, diverging (through a neutral) and bands; each with its explicit invalid colour/value. Each field's compaction scale is taken from the ledger metadata, so a ledger scale change re-styles the view with zero recompute; default ramps follow field role (signed → diverging through neutral, positive → sequential, angle → cyclic, magnitude/diagnostic → greyscale) with per-field polarity (FTLE, diffusion, ensemble spread white = high; `t_end` white = low/early). Any field can occupy either the colour or the brightness role, constrained only by output signature.

## References
- `docs/design/principia_dd_colouring.md` § "3.6 Compaction (payload scalar → b ∈ [0,1]; forms per ledger `scale`)"
- `docs/design/principia_dd_colouring.md` § "6. Deferred / flagged"
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `docs/contracts/principia_gui_state_contract.md` § "8. Amendment to the colouring drill-down"

## Deliverables
- `crates/render/shaders/wgsl/lib/compaction.wgsl`, `lib/ramp.wgsl`.
- `crates/render/src/colour/field_ramp.rs` — the `FieldRamp{field, ramp | compaction}` node for the occupant tree, its invalid colour/value (REQ-COL-001) and its codegen.
- `crates/render/src/colour/defaults.rs` — the default-ramp registry keyed by the ledger `scale` and field role.
- Tests, including the seam-5 re-style test against the dispatch counter.

## Acceptance tests
- `cargo test -p render compaction_forms` — dd_colouring unit test 7: each form monotone on its domain; symlog b(x) + b(−x) = 1 and b(0) = ½ exactly; log styles the −1.0 sentinel instead of ramping it (REQ-COL-039).
- `cargo test -p render default_ramps` — the default ramp registry returns the stated ramp and polarity per field role (REQ-COL-016).
- `cargo test -p render ledger_scale_restyle` — seam 5: change a field's ledger scale; the view re-styles and the dispatch counter is unchanged (REQ-COL-032).
- `cargo test -p render field_either_role` — bind FTLE as brightness and as colour (via a ramp); both compile and render (REQ-COL-024).

## Notes
- The ScalarField sources themselves (payload, geometry-of-n̂, ctx lanes, derived operators) land in TASK-M7-09, which also closes the "every source and ramp/compaction kind constructible" check.
