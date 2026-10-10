# TASK-M1-17 — Conform the debug views to R-399–R-401

- **Milestone:** M1
- **Closes:** REQ-TOOL-162, REQ-COL-064, REQ-RENDER-084, REQ-TOOL-160, REQ-GEN-033
- **Depends on:** TASK-M1-10, TASK-M1-12, TASK-M1-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-8, PIT-3
- **Size:** ~700 lines, the re-rendered goldens included

## Goal
Three rulings of 9 Oct 2026 change how every numeric and field view draws, after TASK-M1-09 (merged) and while
TASK-M1-10, TASK-M1-12 and TASK-M1-13 were being built against the corpus as it stood. This task applies them across
every view those four tasks built. **R-399:** "not yet", an unset `d_min` (R-271, R-280) and a `running` sample (R-96)
alike, is a non-flat style, the neutral grey `DBG_NOT_YET` with a fine dot stipple from the pixel position, drawn by
`debug_not_yet(frag_xy)`, distinct from the invalid hatch's stripes, so it can't collide with any ramp; the stipple's
pattern is proposed here for the M1 gate. **R-400:** a numeric field view whose declared ledger range spans zero is on
`ramp_coolwarm`, centred at zero on the symmetric range `[−M, M]`, derived from the range with no new ledger mark; a
field that can't be negative keeps viridis, and its lower bound is declared in the ledger. **R-401:** each log-scaled
field view without a ledger floor has its own floor, in that field's units, proposed here with evidence for the M1 gate,
replacing the single ε = 2⁻²⁴. **R-403** (closes RQ-261): `K_0` and `V_0` are declared `[0, ∞)` and `(−∞, 0]`, their
scale `lin`, on viridis, mapped monotonically with `V_0`'s most negative value at the dark end; and a field carries the
`diverging` scale only when its declared range spans zero, a check the ledger enforces.

## References
- `decisions.md` § "R-399 — "Not yet" is a non-flat style: the neutral grey with a fine dot stipple from the pixel position, distinct from the invalid hatch's stripes *(closes RQ-259; amends R-96 and R-280)*"
- `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*"
- `decisions.md` § "R-403 — `K_0` and `V_0` are `lin` on viridis over `[0, ∞)` and `(−∞, 0]`; a field carries the `diverging` scale only when its declared range spans zero *(closes RQ-261)*"
- `decisions.md` § "R-401 — Each log-scaled field view has its own floor, in that field's units, proposed with evidence for the M1 gate; the single ε = 2⁻²⁴ is replaced"
- `decisions.md` § "R-398 — `N` is a power of two at every tier and setting, so the sample coordinates are dyadic, as the quadtree's are *(closes RQ-258)*"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default palette"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state block"
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-122 — The reference HTML files are the colour oracle *(closes RQ-90 and RQ-101)*"
- `decisions.md` § "R-136 — `debug_invalid(frag_xy)` draws the hatch; sentinels show their value *(closes RQ-114)*"
- `decisions.md` § "R-263 — §3.8 gains the optional key `floor?: <sim-key parameter>` *(closes RQ-158)*"
- `decisions.md` § "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*"
- `decisions.md` § "R-280 — An unset `d_min` renders in the neutral "not yet" grey *(closes RQ-170)*"
- `decisions.md` § "R-381 — The drift views offer `symlog`, `lin` and `log`, with `symlog` the default; the value fed to `dbg_sentinel` is the compacted value *(closes RQ-206)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `crates/render/shaders/wgsl/lib/present.wgsl` and its CPU mirror: `debug_not_yet(frag_xy: vec2<f32>) -> vec3<f32>`
  beside `DBG_NOT_YET`, the grey with a fine dot stipple from `⌊frag_xy⌋`, its pattern proposed (R-71; REQ-COL-064);
  `DBG_NOT_YET` keeps REQ-COL-053's proposed value.
- Every view that draws "not yet" calls it, never the flat grey: the generated `d_min` numeric debug view and the
  `d_min` `FieldRamp` (TASK-M1-09's, `crates/ledger` and `crates/render/src/colour/field_ramp.rs`), the outcome
  palette's `running` class (TASK-M1-10's `outcome_state.wgsl`), and any view TASK-M1-12 or TASK-M1-13 built that
  draws the grey (REQ-TOOL-162).
- `crates/ledger/data/lut/coolwarm.txt`: Moreland's cool-warm table (R-122), its source and version named in the file,
  as `viridis.txt` and `twilight.txt` are; the prelude's `ramp_coolwarm(t)` reads it as `ramp_viridis` reads its
  table, returning linear RGB, its midpoint the table's neutral (render_gui_spec §10.1).
- `crates/ledger`: the numeric view template takes its ramp from the field's declared range, `ramp_coolwarm` on the
  symmetric `[−M, M]` (fixed and auto) where the range spans zero, `ramp_viridis` otherwise, the cyclic field
  `ramp_twilight` and the drifts R-381's view, unchanged; no new ledger member (REQ-RENDER-084). The ledger declares the
  lower bound of each field the corpus or its own definition makes non-negative, citing the line for each (the mass
  fractions and `|ρ|` of render_gui_spec §10.1, `virial_ratio = 2K₀/|V₀|`, and the like). `K_0` is declared `[0, ∞)`
  and `V_0` `(−∞, 0]`, each citing dd_decoder §3.6, and their scale is `lin`, no longer `diverging`
  (`crates/ledger/src/payload.rs`:174–175 at `2844365`; R-403): both on `ramp_viridis`, mapped monotonically,
  increasing in the value in either mode, `V_0`'s most negative value at the dark end, `t = 0`. Neither range has two
  finite ends, so both views' headers keep the default `RANGE_AUTO = 1` (`crates/ledger/src/gen/numeric.rs`:154–155,
  render_gui_spec §10.1). Under auto, `t = 0` is the measured minimum (`K_0`'s minimum, `V_0`'s most negative
  value); under fixed, `t = 0` is `K_0`'s declared 0 and `V_0`'s measured minimum, an unbounded end taking the
  measured end, and a positive `V_0` clamps to `t = 1` (R-403's "The map is the template's own").
- `crates/ledger`: generation refuses an entry whose scale is `diverging` and whose declared range does not span zero,
  naming the field and its range, beside the metadata gate (REQ-GEN-002); every `diverging` entry left (`energy_drift`,
  `Lz_drift`, `E_0`, `Lz_0`) spans zero (REQ-GEN-033; dd_generation_root §3.8).
- `crates/ledger`: one log floor per log field without a ledger floor (`d_min`, `dE_max`, `dLz_max`, `closure_min`,
  `rho_ratio`, `r_min_pair_0`), each a named presentation constant in that field's units, read by the template; not
  §3.8's `floor?` key, which R-263 gives a sim-key parameter. The REQ-TOOL-160 proposal: each floor, its units, and its
  evidence (where the field's stored values fall under it on M1's fixtures, and why it sits there), attached to the PR.
  The floors for `dE_max` and `dLz_max` are absolute, in the stored fields' own normalised units, since the corpus
  defines both as absolute maxima (R-401's note), and the proposal says so. `closure_min` is a dimensionless chord on
  the unit shape sphere, and its f32 precision floor of about 1e-7 (dd_simstate_payload § 1) is evidence for its
  floor.
- The REQ-COL-064 proposal: the stipple's pattern over the grey, its distinctness from the hatch, its lightness
  contrast, and that no flat ramp colour reproduces a 4 × 4 block of it, attached to the PR. It states the pixel space
  of `frag_xy` (physical or logical pixels) and shows the stipple resolved at the Mac's display scale factor.
- Every golden the three rulings change re-rendered, `m1-numeric` and `m1-outcome` and any of TASK-M1-12's and
  TASK-M1-13's whose views change, each `BASELINES.md` row citing the ruling, proposed and confirmed at the M1 gate.
- `crates/render/tests/uv_absolute_banding.rs`: the two comments (lines 14 and 177 at `ea1921d`) that call RQ-258 open
  cite R-398 instead; no assertion changes.

## Acceptance tests
- `cargo test -p render not_yet_style` — over any 4 × 4 pixel block `debug_not_yet` takes at least two colours, its
  mean is within the proposal's stated distance of `DBG_NOT_YET`, and it differs from `debug_invalid` at every pixel of
  the block (REQ-TOOL-162).
- `cargo xtask golden m1-numeric` — a forced-failure and an unstepped sample draw `d_min` in the "not yet" style in
  the debug view and the `FieldRamp`; a NaN is hatched; a valid `d_min` sits on the ramp (R-280, R-399; REQ-TOOL-162).
- `cargo xtask golden m1-outcome` — a running sample draws the "not yet" style; the nine classes and the two triple
  outcomes render as before (REQ-TOOL-162).
- `cargo test -p ledger numeric_view_template` — a zero-spanning field generates `ramp_coolwarm` on `[−M, M]`, fixed
  and auto, 0 at `t = ½`; a field with `lo ≥ 0` generates `ramp_viridis`; the cyclic field keeps `ramp_twilight` and
  the drifts R-381's view; the ledger gains no member (REQ-RENDER-084); every log view places raw at `dbg_log`'s form
  with its own field's floor, and two log fields with different floors place the same `|x|` differently (REQ-TOOL-160);
  `K_0` and `V_0` generate `ramp_viridis`, increasing in the value in both modes, and both headers default to
  `RANGE_AUTO = 1`; under auto, `t = 0` is the measured minimum (`K_0`'s minimum, `V_0`'s most negative value); under
  fixed, `t = 0` is `K_0`'s declared 0 and `V_0`'s measured minimum, and a positive `V_0` clamps to `t = 1`; `V_0`'s
  most negative value is at `t = 0` in both modes (REQ-RENDER-084, R-403).
- `cargo test -p ledger diverging_range` — every `diverging` entry of the payload ledger declares a range that spans
  zero, and `K_0` and `V_0` are `lin` over `[0, ∞)` and `(−∞, 0]`; negative controls: a fixture entry with scale
  `diverging` over `[0, ∞)`, and one over `(−∞, 0]`, each makes generation fail naming the field and its range; one
  over `(−∞, ∞)` generates (REQ-GEN-033).
- `cargo test -p ledger declared_ranges` — each field the corpus or its definition makes non-negative declares
  `lo ≥ 0`, with its citation (REQ-RENDER-084).
- `cargo test -p render ramp_coolwarm` — the ramp matches the checked-in table at its stops, interpolates between them
  as `ramp_viridis` does, and `t = ½` is the table's neutral (REQ-RENDER-084).
- `cargo xtask golden m1-numeric` — a zero-spanning field (`E_0`, `Lz_0`, `C_ty`) shows its 0 at the neutral and its
  two signs on the two sides; a non-negative field is on viridis; `K_0` and `V_0` are on viridis, not cool-warm, in
  the default (auto) mode, `K_0`'s minimum and `V_0`'s most negative value dark, drawn from a physical `V_0 ≤ 0`, each
  with a `BASELINES.md` row citing R-403 (REQ-RENDER-084).
- Review checklist (gui, qa): the REQ-COL-064 proposal gives the stipple's pattern over REQ-COL-053's grey, shows it is
  dots, not the hatch's stripes, gives its lightness contrast, and shows that no flat colour of viridis, twilight,
  grey or cool-warm reproduces a 4 × 4 block of it; it states `frag_xy`'s pixel space and shows the stipple resolved
  at the Mac's display scale factor; the human confirms it at the M1 gate (REQ-COL-064).
- Review checklist (physics): the REQ-TOOL-160 proposal gives each floor with its units and evidence, the floors
  for `dE_max` and `dLz_max` stated as absolute; each declared
  non-negative range cites its line; the human confirms the floors at the M1 gate (REQ-TOOL-160, REQ-RENDER-084).
- `cargo test -p ledger`, `cargo test -p render` and `cargo xtask golden` — every other view renders as before.

## Notes
- R-399, R-400 and R-401 (9 Oct 2026) are the rulings this task applies; applied per R-369 by the orchestrator:
  TASK-M1-10, TASK-M1-12 and TASK-M1-13 finish against the corpus as dispatched, and this one task conforms every view
  after them. Kept as one task (applied per R-369): the three rulings touch the same template, presentation layer and
  goldens, and three tasks would re-render the same goldens three times.
- `ramp_coolwarm` is not a look value: render_gui_spec §10.1 names it and R-122 gives its data, Moreland's table; the
  table's version is the one Moreland publishes for the cool-warm map, named in the file. TASK-M7-07's cool-warm LUT
  reads the same table.
- R-403 (9 Oct 2026) ruled RQ-261, which held this task's merge: `K_0` and `V_0` are `lin` over `[0, ∞)` and
  `(−∞, 0]` on viridis, `V_0`'s most negative value at the dark end, and a `diverging` field's range spans zero
  (REQ-GEN-033). Nothing holds the merge now. R-403 also confirmed R-401's absolute floors for `dE_max` and `dLz_max`.
- Synthetic payloads that give `K_0` or `V_0` an off-sign value only to tell the offsets apart (for example
  `crates/engine/tests/synthetic.rs`:146–147, `V_0: 10.0`) are layout fixtures, not ICs. Where one meets the new
  declared ranges (a test that reads the range, or a golden that draws the value), the implementer lists it in the PR.
  Drawn, such a value is no `V_0` the physics produces: in auto mode it is placed by the measured range like any value
  (alone, the degenerate range reads `t = 0`), and in fixed mode a positive `V_0` clamps to `t = 1`, looking the same
  as `V_0 = 0`. So the `K_0` and `V_0` goldens use a physical `V_0 ≤ 0` (and `K_0 ≥ 0`), never such a fixture.
- Applied per R-369 (gui review 5472865383, F1): open-ended ranges keep the auto default; the mapping is stated per
  mode. Changing `K_0`'s or `V_0`'s default mode would change render_gui_spec §10.1's rule and is a look choice for the
  human (R-390); R-403 does not make it.
- The qa tests of TASK-M1-09, TASK-M1-10, TASK-M1-12 and TASK-M1-13 that pin the flat grey, viridis on a zero-spanning
  field or the single ε (for example `crates/ledger/tests/qa_TASK-M1-09.rs` and `crates/render/tests/qa_TASK-M1-09.rs`)
  are ruling-forced changes (R-399, R-400, R-401): qa makes them, and the PR lists each with the assertion it changes
  and the ruling that forces it (R-290, R-369). The implementer's own tests (`not_yet_grey.rs`, `numeric_views.rs`,
  `numeric_view_template.rs`) change with the code.
- REQ-TOOL-160 moved here from TASK-M1-09, which built the single ε R-401 replaces. REQ-COL-053 (the grey's value)
  stays TASK-M1-09's; its proposal stands, and this task's stipple is measured over it.
- R-398 asks nothing of this task beyond the two comments above; TASK-M1-16's `N = 6` fixture stays a negative control.
- A declared lower bound can make a field's range fully bounded (e.g. `[0, 1]`); under render_gui_spec §10.1 that field then switches from `RANGE_AUTO = 1` to its fixed range. The implementer lists each such field in the PR and the reviewers check each (code review 5472057070 of PR #175).
- **R-400's scope over TASK-M1-12's hand-written signed views** (the R-388 pre-flight, 10 Oct 2026; grounds corrected
  per physics review 5480220637 of PR #185). R-400's rule is keyed on "a numeric field view whose field's declared
  ledger range spans zero", and its "What does not change" bullet keeps "the drift views … R-381's view". For each of
  TASK-M1-12's views (at its head `bc663a1c`) whose comment says "a signed field on viridis, as the corpus stands
  (RQ-260 open)":
  - `crates/render/shaders/wgsl/frag/debug/derived/energy_drift.wgsl`, the current drift `H(r, p) − E_0`, is the
    `energy_drift` field's view, a drift view, so R-400 does not move it to `ramp_coolwarm`. As built it passes the raw
    `ΔE` to `dbg_sentinel` on viridis, with no `eps_E` compaction: that interim placement is not R-381's view, and is
    the defect TASK-M3-05 names ("a raw drift ≪ 1 lands at the ramp's middle", TASK-M3-05.md:45). Its compaction
    (R-381's `symlog`, `lin` and `log`) and its ramp (`f_edrift`'s `diverging(c−,c0,c+)`, viridis only through the
    palette swap) are TASK-M3-05's to build; this task leaves the view as it is.
  - The generated `n` view, the '‖·‖ as scalar' reduction (`crates/render/frag/debug/generated/n.wgsl`), shows `‖n‖`.
    Read literally, R-400 would put it on `ramp_coolwarm`, since `n`'s declared range is `[−1, 1]`. Applied per
    R-369: it keeps viridis, because the quantity shown, `‖n‖`, cannot be negative ("fields that can't be negative keep
    viridis", applied to the quantity the view shows rather than to the vector field's range). A display-ramp scope
    choice; it changes no result.
  - `crates/render/shaders/wgsl/frag/debug/live_shape.wgsl`'s mode 2, `‖n‖ − 1`, is a signed error but not a ledger
    field, so R-400's range rule does not reach it. Applied per R-369: it stays on viridis as built. Mode 0
    (`dbg_dircos3`) and mode 1 (`ramp_twilight`) are no ramp of a declared range and do not change.
  Only the numeric template moves fields to `ramp_coolwarm`. This task rewords those comments' "RQ-260 open" to cite
  R-400 and the grounds above (and, for `energy_drift.wgsl`, that its R-381 view is TASK-M3-05's); no assertion
  changes. The human may veto either choice applied per R-369; a veto would be a ruling, applied by a follow-up task.
- **The cool-warm table already exists** (the R-388 pre-flight): `crates/render/tests/data/lut/coolwarm.txt` holds
  Moreland's 33-stop CoolWarmFloat33 table, as matplotlib 3.8.0 transcribes it, its source and version named in the
  file, sRGB-encoded, each stop at `k/32`, so stop 16 is at `t = ½`, the neutral. The deliverable
  `crates/ledger/data/lut/coolwarm.txt` takes the same table (R-122), so the two copies agree, and `ramp_coolwarm(t)`
  reads it as `ramp_viridis` reads its table, returning linear RGB.
