# TASK-M7-05 — Compaction, ramps and the default ramp per field role

- **Milestone:** M7
- **Closes:** REQ-COL-039, REQ-COL-016, REQ-COL-032, REQ-COL-024, REQ-COL-065, REQ-COL-066
- **Depends on:** TASK-M7-03, TASK-M7-04, TASK-M1-09, TASK-M1-17
- **Needs (earlier milestones):** REQ-COL-001, REQ-GEN-001, REQ-GEN-002, REQ-GEN-012, REQ-RENDER-021
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Family B's mapping half: the Compaction forms lin, log (x ≤ 0 → 0 with sentinel styling), cyclic (frac(x/period)), diverging symlog b = ½ + ½·sign(x)·ln(1+|x|/x₀)/ln(1+x_max/x₀) with the ε floor as x₀, and flag; the Ramp kinds lut, lerp, diverging (through a neutral) and bands; each with its explicit invalid colour/value. Each field's compaction scale is taken from the ledger metadata, so a ledger scale change re-styles the view with zero recompute; default ramps follow field role (signed, read as R-400 reads it: a field whose declared ledger range spans zero, `lo < 0 < hi`, either end possibly unbounded → diverging through neutral, positive → sequential, angle → cyclic, magnitude/diagnostic → greyscale) with per-field polarity (FTLE, diffusion, ensemble spread white = high; `t_end` white = low/early). Any field can occupy either the colour or the brightness role, constrained only by output signature.

## References
- `docs/design/principia_dd_colouring.md` § "3.6 Compaction (payload scalar → b ∈ [0,1]; forms per ledger `scale`)"
- `docs/design/principia_dd_colouring.md` § "6. Deferred / flagged"
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `docs/contracts/principia_gui_state_contract.md` § "8. Amendment to the colouring drill-down"
- `decisions.md` § "R-399 — "Not yet" is a non-flat style: the neutral grey with a fine dot stipple from the pixel position, distinct from the invalid hatch's stripes *(closes RQ-259; amends R-96 and R-280)*"
- `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*"
- `decisions.md` § "R-403 — `K_0` and `V_0` are `lin` on viridis over `[0, ∞)` and `(−∞, 0]`; a field carries the `diverging` scale only when its declared range spans zero *(closes RQ-261)*"
- `decisions.md` § "R-401 — Each log-scaled field view has its own floor, in that field's units, proposed with evidence for the M1 gate; the single ε = 2⁻²⁴ is replaced"
- `decisions.md` § "R-263 — §3.8 gains the optional key `floor?: <sim-key parameter>` *(closes RQ-158)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"

## Deliverables
- `crates/render/shaders/wgsl/lib/compaction.wgsl`, `lib/ramp.wgsl`.
- `crates/render/src/colour/field_ramp.rs` — extends TASK-M1-09's minimal `FieldRamp` (one `ScalarField`, a `lin` `Compaction`, viridis and the invalid lane; RQ-232) to the `FieldRamp{field, ramp | compaction}` node for the occupant tree, its invalid colour/value (REQ-COL-001) and its codegen.
- `crates/render/src/colour/defaults.rs` — the default-ramp registry keyed by the ledger `scale` and field role.
- `docs/design/principia_dd_colouring.md` §3.6: `x_max` and `x₀` for each `diverging` field (REQ-COL-065, R-72), with
  the commit's "Removed lines" note.
- Tests, including the seam-5 re-style test against the dispatch counter.

## Acceptance tests
- `cargo test -p render compaction_forms` — dd_colouring unit test 7: each form monotone on its domain; symlog b(x) + b(−x) = 1 and b(0) = ½ exactly; log styles the −1.0 sentinel instead of ramping it (REQ-COL-039; waits on RQ-265).
- `cargo test -p render default_ramps` — the default ramp registry returns the stated ramp and polarity per field role (REQ-COL-016).
- `cargo test -p render ledger_scale_restyle` — seam 5: change a field's ledger scale; the view re-styles and the dispatch counter is unchanged (REQ-COL-032; waits on RQ-265).
- `cargo test -p render field_either_role` — bind FTLE as brightness and as colour (via a ramp); both compile and render (REQ-COL-024).
- Review checklist (physics) — dd_colouring §3.6 defines, for each `diverging` field (`energy_drift`, `Lz_drift`, `E_0`, `Lz_0`), what `x_max` is and how `x₀` is chosen, in that field's units, the drifts' `x₀` being their floors `eps_E` and `eps_L`; the doc change is in this PR and the physics reviewer approves it before merge (REQ-COL-065; waits on RQ-265).
- Proposal: `E_0`'s and `Lz_0`'s `x₀`, each in its field's units, and any fixed `x_max` REQ-COL-065 gives, each with its evidence; the physics reviewer checks each and the human confirms them at the M7 gate (REQ-COL-066).

## Notes
- The ScalarField sources themselves (payload, geometry-of-n̂, ctx lanes, derived operators) land in TASK-M7-09, which also closes the "every source and ramp/compaction kind constructible" check.
- **Depends on TASK-M1-17** (applied per R-369; the R-388 pre-flight, 10 Oct 2026). TASK-M1-17 builds
  `debug_not_yet` (R-399), declares the non-negative lower bounds that make R-400's "spans zero" readable from the
  ledger, and moves `K_0` and `V_0` to `lin` (R-403); both tasks edit `crates/render/src/colour/field_ramp.rs`. R-399
  (decisions.md:6900–6903): "**Where it is drawn:** every field view that shows `d_min`'s unset value (the generated
  numeric debug view and the `d_min` `FieldRamp`, TASK-M1-09's), the outcome palette's `running` class (TASK-M1-10's
  occupant), and every later view that shows "not yet" (TASK-M7-05's and TASK-M7-09's extensions of `FieldRamp`
  inherit it)." So this task's `FieldRamp` draws "not yet" through `debug_not_yet(frag_xy)`, never a flat grey.
- **"Signed" is R-400's** (decisions.md:6969–6971): "colour_composition §1.2's default for signed fields (TASK-M7-05's
  registry, REQ-COL-016) reads "signed" the same way: a field whose declared range spans zero. §1.2 gains a sentence;
  REQ-COL-016 gains R-400 and a note and loses `rq: RQ-260`." The registry keys "signed" on the declared range, with no
  signed-or-not mark; `K_0` and `V_0` are one-signed and `lin` (R-403).
- **REQ-COL-065 (definition, R-72; the R-388 pre-flight).** dd_colouring §3.6's diverging form uses `x_max`, which the
  corpus defines nowhere, and takes "the ε floor as `x₀`" for the signed drifts, which settles theirs: `eps_E` and
  `eps_L` (§3.8's `floor?`, R-263), which R-401 leaves unchanged (R-401 replaced the single ε only for the log-scaled
  views with no ledger floor). R-403 leaves `E_0` and `Lz_0` `diverging` with no floor
  (`crates/ledger/src/payload.rs`:132–133 at `c683714`), so their `x₀` is open. This task writes the form's rules into
  §3.6 (what `x_max` is, how `x₀` is chosen per field), the physics reviewer approving that definition (applied per
  R-369: physics joins this task's reviewers for REQ-COL-065 and REQ-COL-066 only); the numbers the corpus does not
  give, `E_0`'s and `Lz_0`'s `x₀` and any fixed `x_max`, are REQ-COL-066 (calibration, R-71), proposed with evidence
  and confirmed by the human at the M7 gate, as R-401 classes a per-field floor (physics review 5480220637 of PR #185).
- **RQ-265 (open).** The debug views' `symlog` and `log` placements (render contract presentation layer, R-381;
  render_gui_spec §10.1, R-401) differ from §3.6's forms, and §3.6's `log` needs `lo > 0` where the ledger declares
  `lo = 0`. Three lines wait on the ruling, each requirement carrying `rq: RQ-265`: REQ-COL-032 (the scale the view
  re-styles from), REQ-COL-039 (the forms, §3.6's `log` not buildable as written over `lo = 0`) and REQ-COL-065
  (whether `x_max` exists is what option (c) decides). REQ-COL-016, REQ-COL-024 and REQ-COL-066 do not wait.
