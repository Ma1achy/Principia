# Decisions

The log of human rulings. Each entry is dated and names the step that applies it. The open decisions are in
`docs/archive/untangling/DECISIONS_TO_MAKE.md` (step 5; archived, every item ruled). Once a ruling is made it is recorded here.

---

## R-1 — The markdown corpus is the only authority
*24 Sep 2026 · applied in step 3*

Step 3 checks each LaTeX passage against the **current markdown**, not against prin-rs `FINDINGS.md`.
Where the two genuinely conflict, add a `REVIEW_QUEUE.md` entry and don't choose. Citations to FINDINGS
can stay as notes.

## R-2 — The index doesn't cite the unpushed commit `52caf14` *(closes RQ-1)*
*24 Sep 2026 · applied in step 3*

The index shouldn't cite prin-rs `52caf14`, because it exists only on an unpushed branch. Repoint the
citation to `principia_dd_refinement_policy.md` §0.1, which records the same result.

## R-3 — The two PDFs are retired on the same terms as the LaTeX *(closes RQ-3)*
*24 Sep 2026 · applied in step 3*

`sphere_colour_map_spec.pdf` and `com_projection_mini_spec.pdf` are retired. Port what the markdown
relies on into the file that relies on it:
- colour PDF Eq. 5 → `principia_dd_colouring.md` §3.2
- colour PDF §8 (the shape-sphere component → axis convention) → `principia_dd_integrator.md` §3.7
- the COM-projection mini spec → wherever `sec:com_projection` is ported (integrator contract)

Then repoint the references.

## R-4 — "spec-keyed defaults" means the markdown's tier tables *(closes RQ-5)*
*24 Sep 2026 · applied in step 3*

`principia_dd_telemetry_and_tiers.md` line 398 refers to the markdown's tier tables: telemetry §3.5, the
three-axis `eps` / frame-budget / hard-cap model. Repoint it there. **Don't port** the LaTeX
`sec:quality_tiers`, because it predates the move to `eps`.

## R-5 — The chart constants go to the decision sheet *(closes RQ-2)*
*Amended by R-10.*
*Still in force: `α_min` (0.05 in the LaTeX and B25, 0 in the markdown and the tool) goes to the step-5 decision sheet,
listed with audit B25; `μ_max` is settled by R-10.*
*24 Sep 2026 · applied in step 5*

List them with audit B25. `μ_max` is 5 in the LaTeX and 4 in the IC Inspector notes. `α_min` is 0.05 in
the LaTeX and B25, and 0 in the markdown corpus and the tool.

## R-6 — The event priority order goes to the decision sheet *(closes RQ-4)*
*24 Sep 2026 · applied in step 5*

List it with audit B4. It has no LaTeX cross-check. Until it's decided, the markdown's pin in
`principia_dd_integrator.md` §3.6 (collision beats escape) stands.

## R-7 — Step 3's done-check *(closes RQ-6)*
*Amended by the entry "R-7 amended — part (a)'s grep".*
*Still in force: part (a)'s scope and expected results, part (b) and the rerun after step 4; only part (a)'s grep
command is the later entry's.*
*24 Sep 2026 · applied in step 3*

Step 3 is done when both of these hold:

**(a)** Run over the corpus markdown only, after step 2's move:

```
grep -rniwE --include='*.md' "latex|spec\.tex" docs/ --exclude-dir=archive
```

It returns only:
- the retirement ruling in `principia_canonical_spec.md`;
- `principia_spec_pending_changes.md`, which is exempt by name until step 4 archives it.

The root working docs (HANDOFF, REVIEW_QUEUE, decisions, open-questions), `spec_sources/`, `workbench/`
and `.git` are out of scope by construction.

**(b)** `spec_sources/SECTION_MAP.md` is the ledger for everything the grep can't judge ("the spec's …"
phrases). Every T, S and D row carries `done: <commit>` or a REVIEW_QUEUE reference. F and M rows need no
action. R rows are closed in step 4.

**After step 4:** rerun (a) with no exemptions. It returns only the retirement ruling.

## R-8 — The IC Inspector copies
*24 Sep 2026 · applied in step 3*

The `docs/` copies are canonical: `docs/gui/reference/ic_inspector.html` (19 Jul, 39.6 KB) and
`docs/notes/ic_inspector_scratchpad.md` (the later, longer one). The `spec_sources/` copies are older
uploads. They go to `docs/archive/` when `spec_sources/` is archived at the end of step 3.

## R-9 — The toolchain spike findings go to results *(closes RQ-7)*
*24 Sep 2026 · applied in step 3*

`docs/archive/findings.md` → `docs/experiments/results/findings.md` (`git mv`). It is the toolchain spike's
findings, the evidence for the rust-gpu decision. Its INDEX row moves with it.

## R-7 amended — part (a)'s grep
*24 Sep 2026 · applied in step 3*

Part (a)'s grep becomes the following. It drops `-w`, because the filename form has an underscore before "spec":

```
grep -rniE --include='*.md' "\blatex\b|spec(_revised)?\.tex" docs/ --exclude-dir=archive
```

The rest of R-7 is unchanged.

## R-10 — μ_max and q_max are settled *(closes RQ-8, amends R-5)*
*24 Sep 2026 · applied in step 3*

`μ_max = 5` and `q_max = 2`. The "4" was the IC Inspector's value before its correction. The values are written
where the constants are defined (`principia_dd_decoder.md` §3, `principia_chart_decoder_contract.md`), and the
formulae keep the symbols. Only `α_min` (0 or 0.05) stays on the step-5 decision sheet.

## R-11 — The per-body momentum cap is rejected *(closes RQ-9)*
*24 Sep 2026 · applied in step 3*

The LaTeX's optional per-body cap isn't invertible: capping each body, then re-imposing CoM, breaks the T2 round
trip. And `q_max` already bounds the Jacobi momenta. It's recorded under rejected ideas in `principia_00_philosophy.md` §8.

## R-12 — The shape-sphere chart map stays as the markdown has it *(closes RQ-10)*
*Corrected by R-14 (its premise).*
*Superseded in part by R-14 (its axis wording; R-184).*
*Still in force: no polar buffer, the LaTeX's buffer recorded as rejected in chart_reference §3.3; the axes and the
premise are R-14's.*
*24 Sep 2026 · applied in step 3*

θ on `v`, no polar buffer. The LaTeX's buffer rests on a wrong premise: the collision points are on the equator,
not at the poles. It's recorded as rejected in `principia_chart_reference.md` §3.3. In Group B, check the colour
PDF §8 axis convention against this and flag any mismatch.

## R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*
*24 Sep 2026 · applied in step 3*

A looked-up IC takes the same path as any pixel. Bodies within `r_coll` give a t = 0 collision outcome, which is
a real outcome. Exactly coincident bodies can't be represented, and the lookup ladder's range check catches them with
`lookup_clamped`. `principia_inverse_encode_contract.md`'s validation is updated to match.

## Porting rule — a port adds, it never removes a decision
*24 Sep 2026 · applies from step 3 Group B onward*

A port may add content. It may never delete or change a decision the markdown has made, including the prototype-kernel
decisions in the drill-downs and contracts, without a REVIEW_QUEUE entry and a ruling. Each commit message ends with a
"Removed lines" note. For every removed line it says either "reworded, kept at <file:line>" or "stale value, replaced by
<ruling>". A removed decision with neither is a bug.

## R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*
*24 Sep 2026 · applied in step 3*

The convention was validated by the human:

```
u = ‖ρ̃‖² − ‖λ̃‖²,   v = 2 ρ̃·λ̃,   w = 2(ρ̃ ∧ λ̃)   (standard, positive cross)
n = (u, v, w)/I
θ = azimuth in the (u, v) plane, on the horizontal axis, 0..2π
φ = polar angle from +w, on the vertical axis, 0..π; L⁺ (w = +1) at the top
n = (sin φ cos θ, sin φ sin θ, cos φ)
```

Applied as follows:
- chart_reference §3.1: `q` takes the standard sign.
- chart_reference §3.3: the spherical map is rewritten in θ/φ.
- The R-12 note is rewritten. There is no polar buffer, because under this convention the poles are the Lagrange
  points (regular) and every binary collision lies on the equator (w = 0).
- dd_integrator §3.7: the overlay's b̂ landmarks are computed from this formula, not hard-coded. The collision of bodies 0
  and 1 is at n = (−1, 0, 0). Audit decision B18 (mass-weighted positions or fixed 120°) stays open.
- dd_integrator's shape-map tests check the landmarks numerically (BC₀₁ → (−1,0,0), L⁺ → (0,0,+1), all collisions at w = 0,
  equal masses 120° apart) and cross-check `n` against the IC Inspector's JS on random ICs.

## R-15 — `Policy::Tolerance` governs refinement *(closes RQ-13)*
*24 Sep 2026 · applied in step 4*

This follows the INDEX's "current design" and landed pending change 12. The scheduler contract keeps its mechanics and
defers the split decision to `principia_dd_refinement_policy.md`. It's applied in step 4, when change 12 is folded in.

## R-16 — The colour PDF's map lists are ported *(closes RQ-14)*
*24 Sep 2026 · applied in step 3*

The complete Artefact-1 and Artefact-2 map lists go from the retired colour PDF into `principia_colour_composition.md`
(under R-3), and the golden-image tests are pinned to that list. Magenta stays the invalid colour as a plain default,
with no source claimed.

## R-17 — The diffusion sentinel uses the streaming slope *(closes RQ-15)*
*Amended by R-245.*
*Still in force: the diffusion value uses the streaming slope the payload ledger and the payload doc define; it is no
longer a sentinel (R-245).*
*24 Sep 2026 · applied in step 3*

The streaming slope is the one the payload ledger and the payload doc define. `principia_render_contract.md`:79 is updated to cite it.

## R-18 — The agreement value is `spread_event` *(closes RQ-16)*
*24 Sep 2026 · applied in step 3*

`spread_event` is an f16 and is stored, as `principia_dd_generation_root.md`'s ledger defines it. `ensemble_outcome_agreement` is a retired name. The
sampling note cites `spread_event`, and any agreement value is derived from it on the fly.

## R-19 — Change 8 landed in full *(closes RQ-17)*
*24 Sep 2026 · applied in step 4*

`ADVANCE(state, t_now, t_target, params)` is the occupant seam. Part 2b extended it with regularisation as a separate
axis; it didn't replace it. In the integrator contract:
- Part 2a's "PROPOSED CHANGE" becomes "DECIDED (change 8, extended by Part 2b)".
- `owns_time_mapping` is reported for the composed occupant (stepper × regularisation): true whenever the regularisation
  is not `none`.
- The per-substep cadence is a callback passed in.
- Law 18's count bound is on a fixed τ-schedule.

The original rationale text is kept.

## R-20 — No `majority_class` field *(closes RQ-18)*
*24 Sep 2026 · applied in step 4*

The impurity mask compares each sample with `dominant_outcome` at the joint `class ⊕ detail` grain (change 1's
resolution, `principia_dd_generation_root.md` §3.7). The impurity-mask rows in the render contract and in
`principia_debug_tooling_plan.md` say so.

---

## Rulings on the decision sheet (step 5, PR #6)

R-21 to R-59 rule every item on `docs/archive/untangling/DECISIONS_TO_MAKE.md`, in the order of the sheet. The six blockers (R-21 CD-1, R-22 CD-2,
R-29 IE-1, R-36 PL-1, R-37 PL-2, R-41 RS-1) are applied in step 5, one commit each. The defects D1–D3 go with the blockers they belong to
(IE-1 and PL-2, as the sheet ties them). The rest are applied when the build reaches them, as the sheet says for each.

## R-21 — `α_min = 0` *(CD-1)*
*24 Sep 2026 · applied in step 5*

Full sphere; the poles are represented and fenced by the collision detector, the conditioning readout and the SAT flags.
`α_min` is not a numerical guard. The descriptions calling it "a buffer that keeps ‖ρ‖ bounded away from zero" are
reworded. Files: dd_decoder §3, §3.2; chart_reference §0.2; inverse_encode_contract :74–75.

## R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*
*Amended by R-62.*
*Still in force: 0-based body indices and labels, `m0 m1 m2`, and pair id `k` naming the side opposite body `k`, as a
schema event; the Jacobi fields are named by R-62.*
*24 Sep 2026 · applied in step 5*

0-based throughout: `ICDescriptor` fields `m0 m1 m2`, `rho0_mag` / `rho1_mag`, and 0-based display labels. **Pair id `k` names the side
opposite body `k`:** pair 0 = (1, 2), pair 1 = (2, 0), pair 2 = (0, 1). This goes into the payload's pair-id map and every
consumer of `detail` and `dmin_pair`. It changes the ledger, so it is a schema event (R-36). Files: dd_generation_root §3.6, §6;
render_contract :14; dd_decoder §6; dd_simstate_payload §2; chart_reference; dd_integrator §3.7; chart_decoder_contract;
inverse_encode_contract :102; colour_composition :159–170; `ic_inspector.html` labels.

## R-23 — Encode order: CoM subtraction precedes `I` *(CD-3)*
*24 Sep 2026 · recorded; applied before encode, lookup and lock*

The operational order 1a subtract CoM → 0 rescale → 1b subtract boost → 2 rotate → 3 mirror is normative. The contract is
renumbered to match. Files: dd_encode §3.2, §6; inverse_encode_contract Parts 2, 6; canonical_spec :52 and
systems_architecture :149 (wording).

## R-24 — The encode projection metric is confirmed *(CD-4)*
*24 Sep 2026 · recorded; applied before off-curve Burrau encode*

Shape-sphere chordal ⊕ mass-simplex Euclidean, unit weights. Written into inverse_encode_contract Part 5 kind 4. Files:
dd_encode §3.4, §6, test 5.7; inverse_encode_contract :118.

## R-25 — `η_E` is kept, with an explicit off switch *(CD-5)*
*24 Sep 2026 · recorded; applied during the build*

Energy normalisation stays, gated by `forbids_energy_normalisation`. "Off" is an explicit `Option`/flag, never `E* = 0`,
which is a real physical target. Files: dd_decoder §3.7, §6; chart_reference §0.6, :194, :504, :539;
chart_decoder_contract :236; inverse_encode_contract :198; lowering_contract :157.

## R-26 — Every chart declares its domain function *(CD-6)*
*24 Sep 2026 · recorded; applied before the invariant charts and quad-skip*

`validate(u, v) -> ValidationResult` is added to the `Chart` trait (chart_reference §5.1). The quadtree, lookup/lock and the legend
query it. Files: chart_reference §5.1, §5.2; chart_decoder_contract Parts 3, 5; inverse_encode_contract :157–192;
lowering_contract appendix; scheduler_contract (a quad-skip rule); the step-6 GUI docs.

## R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*
*24 Sep 2026 · recorded; applied before any Burrau statistic*

Shape only (the fold) and shape × labelling (the full range) are both kept. A `system_image` value for "covers each shape twice, as two
labelled systems" is added, and the stale "ray-degenerate" description of the Euclid plane is corrected. Files: chart_reference
§4.1, §4.3, §4.5; chart_decoder_contract Part 5; inverse_encode_contract :116, :170, :199; dd_encode :34, test 5;
lowering_contract :160–162.

## R-28 — Momentum decode has no mass unweighting *(CD-8)*
*24 Sep 2026 · recorded; no file changes*

Confirmed as intended (`ic_inspector_scratchpad.md:19`, dd_decoder §3.4). Closes the rest of audit B25.

## R-29 — The escape criterion's undefined parts *(IE-1, amended)*
*Amended by R-61.*
*Still in force: all of it (`E_rel` with the total mass, the provisional 0.4 window, the escaper rule and the to-do);
R-61 defines its "largest separation".*
*24 Sep 2026 · applied in step 5*

- **`E_rel`** is the relative two-body energy of the escaper `b` about the centre of mass of the other two:
  `E_rel = ½|Δv|² − (M_pair + m_b)/d`, with `Δv` and `d` the escaper's velocity and distance relative to that centre of mass.
  It uses the **total** mass. prin-rs's `M_pair`-only form is a physics error that biases toward escape.
- **The window** is 0.4 time units, sampled at sync boundaries (**provisional**).
- **The escaper** is the body with `E_rel > 0` and the largest separation from the other two.
- **To do:** re-validate precision and recall with the corrected energy, and re-measure the `tau` gap. Pitfalls §2.2's
  "383×, untuned" claim is marked "to re-measure" until then.

With D1 and D2 (R-59). Files: integrator_contract Parts 3, 4, 5, 7; dd_integrator §3.6 and the closing line; pitfalls §2.2;
dd_simstate_payload §2, :527.

## R-30 — Event precedence is by time *(IE-2)*
*Amended by R-339:* its date line's "R-6's pin stands until then" no longer holds: dd_integrator §3.6's pin is marked
superseded by this ruling's time ordering.
*Still in force: all of it; only its date line's "R-6's pin stands until then" is spent (R-339).*
*24 Sep 2026 · recorded; applied with the label writer (R-6's pin stands until then)*

sim_failed first; then whichever event came first, with a tie going to collision; triple ejection as a detail of escape; then running,
then bounded. Triple collision and triple ejection are placed explicitly. Files: dd_integrator §3.6 (:176–184), test 6, §6, closing line;
integrator_contract Part 7 (:390); parity_contract :39, :193; payload §2.

## R-31 — Escape doesn't terminate until pitfalls §2.4's three checks pass *(IE-3)*
*24 Sep 2026 · recorded; applied during the build*

Keep integrating after escape. Record prin-rs's check 1 as passed and run checks 2 and 3. Specify what `state` holds when
escape has fired but the march continues. Files: pitfalls §2.4; integrator_contract Part 7; payload §2; dd_integrator §3.6.

## R-32 — The ionisation gate is pairwise-unbound, separating and total `E > 0`, with settling *(IE-4 (c))*
*24 Sep 2026 · recorded; applied before outcome data at scale*

All three pairwise energies > 0, all separations growing, total `E > 0`, evaluated with the escape rule's settling. Its place in
the R-30 precedence is written with it. The old-data question is closed: no stored dump recorded two-pair collisions as binary.
Files: integrator_contract Part 7 (:388); dd_integrator §3.6 (:161), tests §5; parity_contract :40.

## R-33 — The independent convergence reference is Brutus-style *(IE-5, amended)*
*Amended by R-265.*
*Still in force: the independent reference is CPU arbitrary precision with convergence gating, and RK45 stays the
inspector's reference; that reference also serves as the screen, and double-double is parked (R-265).*
*24 Sep 2026 · recorded; applied before the integration-floor probe and Burrau ground truth*

CPU arbitrary precision with convergence gating: raise the precision and tighten the tolerance until the result stops changing.
Double-double is a fast screen only. RK45 stays the inspector's reference. Files: canonical_spec §7, §11 (:91, :151);
validation_ground_truth_note :99; parity_contract :13; systems_architecture :45; integrator_contract :87, :242, :370;
dd_integrator :92; spike_brief :61.

## R-34 — FMA: explicit fma at every branch input, enumerated *(IE-6)*
*24 Sep 2026 · recorded; applied before the parity gate on a second backend*

Rule 6 extends to every branch input that consumes multi-op arithmetic, the change-11 `|Δn̂|` and `E_rel` included. The branch
inputs are enumerated rather than relying on a backend switch. Files: integrator_contract Part 2c, Part 4; gpu_determinism_note rules 2 and 6;
parity_contract Tier B.

## R-35 — The change-10 cross-checks are re-run and the NumPy reference patched *(IE-7)*
*24 Sep 2026 · recorded; applied before any further integrator measurement*

The divergence-vs-horizon table is regenerated or retired as part of the re-run. Files: dd_validation_orbits §0.1, §5;
dd_predictability_horizon; `workbench/tb_az.py`.

## R-36 — Schema version = content hash of the ledger *(PL-1)*
*24 Sep 2026 · applied in step 5*

Files: dd_generation_root §4, §5 test 7, §6; caching_contract :12; render_contract :59, :182; payload :161, :276.

## R-37 — `t_min` is a departure threshold on the shape sphere *(PL-2 (b))*
*24 Sep 2026 · applied in step 5 (the rule); the value is set by measurement*

The closure minimum starts once `|n̂(t) − n̂(0)|` has first exceeded a stated threshold, which is to be measured.
`closure_min` / `closure_step` are added to the ledger (D3, R-59). Files: payload §1; dd_generation_root §3.4;
dd_validation_orbits §6; integrator_contract.

## R-38 — Per-pair views are out of v1 until `dominant_pair` is specified *(PL-3)*
*24 Sep 2026 · recorded; applied before any per-pair view*

The three open specifications in symbolic_dynamics_contract §1–4 are written before any per-pair view ships. Word storage and
`S_word` proceed. Files: symbolic_dynamics_contract §1–4; dd_generation_root :45, :87–90; debug_tooling_plan :51;
render_contract :104; payload §5, :468; canonical_spec :152.

## R-39 — `FULL_RETENTION` keeps bit 4, owned by the measurement path *(PL-4, amended)*
*24 Sep 2026 · recorded; applied in step 6*

The owner is the measurement path: the Measure tool and matched N/2N renders, a uniform grid with every sample retained.
Step 6's GUI docs specify it. Files: scheduler_contract Parts 2, 5; the step-6 GUI docs.

## R-40 — Memory tiers and the quality device key off `eps` *(PL-5)*
*24 Sep 2026 · recorded; applied before the auto controller and hard cap*

The eps / frame-budget / hard-cap axes are folded into `QualitySettings` and the ladder, and the stale 136/88 B widths recomputed (D6).
Files: quality_device_note; memory_tiers; canonical_spec :79; systems_architecture :63, :163; telemetry §3.5, §4.

## R-41 — `DEBUG_MODE` uses the baked variants, not flag bits *(RS-1 (b))*
*24 Sep 2026 · applied in step 5*

Drop "in dispatch flags" from the render contract and the debug plan. Bits 6–7 stay reserved. Files: render_contract :202;
debug_tooling_plan :26; scheduler_contract Part 5 (reserved-bit note).

## R-42 — The `alpha_area` defects: tell empty from full by `n_unresolved`; refuse the floor on a negative exponent *(RS-2 (b)+(c))*
*24 Sep 2026 · recorded; applied with refine*

`alpha_lo` stays at 0.005. The ledger's stale `alpha` row is fixed with it (D4). Files: refinement_policy §2, §2.2, §7;
dd_generation_root §3.7; INDEX :165; pitfalls :372.

## R-43 — `N = 16` vs 8 is measured; the thread-count inconsistencies are fixed now *(RS-3)*
*24 Sep 2026 · recorded; applied before WebGPU deployment*

Files: systems_architecture §5.5; memory_tiers §4 (the N column); scheduler_contract :141; lowering_contract :138;
quality_device_note.

## R-44 — The camera is wired; scheduler Part 6 is reconciled with policy §0.1 *(RS-4)*
*24 Sep 2026 · recorded; applied during the build*

Close "camera not wired into priority". Reconcile `P_complexity = 1 − coherence`, "never compute off-screen" (:132) and
"offscreen → stop" (:134) with the policy, and say how the order in view enters. Files: scheduler_contract Part 6.

## R-45 — The frontier margin is derived from the refill rate, and a widened margin is the baseline to beat *(RS-5)*
*24 Sep 2026 · recorded; applied during the build*

Files: refinement_policy §0.1, §7; scheduler_contract Parts 6, 7; INDEX :170.

## R-46 — Relevance is computed relative to the camera or quad centre *(RS-6)*
*24 Sep 2026 · recorded; applied before any deep-zoom demo*

Using deep_zoom §1's centre-plus-half-width pattern. Files: scheduler_contract Part 6; deep_zoom §1; INDEX :171.

## R-47 — `Undetermined` is reported, terminal and flagged, until a chart shows it non-zero *(RS-7)*
*24 Sep 2026 · recorded; applied during the build*

Files: refinement_policy §6; telemetry §3.5, §4; scheduler_contract Part 6; quality_device_note.

## R-48 — The `sea_fraction` estimator is built after the tier controller *(RS-8)*
*24 Sep 2026 · recorded; applied during the build*

Files: refinement_policy §5.1, §7; telemetry §3.5, §4.

## R-49 — The refinement policy's open measurements are taken at the first refine milestone *(RS-9)*
*24 Sep 2026 · recorded; applied at the first refine milestone*

Depth beyond level 6, a calibrated grid / `tau`, and merging under real camera motion. Files: refinement_policy §5, §7.

## R-50 — The shape-sphere collision landmarks are mass-weighted *(CO-1 (a))*
*24 Sep 2026 · recorded; applied before the physics-overlay occupant*

Fix the two HTML references that disagree. Files: dd_integrator §3.7, test 9; dd_colouring :90; colour_composition
:197–209, :481; trajectory_viewing :78, :84; `principia_gui_mock.html` :784, :937; `principia_colour_presets.html` :131–139.

## R-51 — The OKLab coefficients are checked against Ottosson *(CO-2)*
*24 Sep 2026 · recorded; applied before the colour constants enter the shared source*

Files: dd_colouring §3.1, only if a digit is wrong.

## R-52 — Undo lives in the state contract *(GU-1 (a))*
*24 Sep 2026 · recorded; applied in step 6*

Undo/redo is a history of typed `setField` edits, shared by every GUI. Files: gui_state_contract §2, §7; colour_composition :264;
render_gui_spec; caching_contract :133–134.

## R-53 — Node interfaces declare their input domains *(GU-2 (a))*
*24 Sep 2026 · recorded; applied in step 6*

The manifest's per-field domain is the default a node inherits. Files: gui_state_contract §3, §4; render_gui_spec §6, §10.1;
colour_composition §1.2, §3; dd_colouring :119; debug_tooling_plan :43–44.

## R-54 — The precision warning is event-driven *(GU-3 (b))*
*24 Sep 2026 · recorded; applied in step 6*

Raised on `DECODE_SWITCHOVER` over visible quads and on `AT_F32_FLOOR`. Files: deep_zoom §2; scheduler_contract Part 4;
the step-6 GUI docs; gui_state_contract §2.

## R-55 — Cursor bias is `P_focus` centred on the pointer *(GU-4 (a))*
*24 Sep 2026 · recorded; applied in step 6*

`P_focus` centres on the pointer instead of the viewport centre while the pointer is in view, with a weight and a decay to be written. Files: scheduler_contract Part 6;
scratchpad_pointer_channels :132–134; caching_contract :133.

## R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*
*24 Sep 2026 · recorded; the measurement struct lands with the first frame loop, the schema in step 6*

At the top level: telemetry §2's frame record and its five stages. Beneath them: nested scopes, GPU passes, allocations and events.
Files: dd_telemetry_and_tiers §2, §5, §5.5, §7; debug_tooling_plan; the step-6 GUI docs.

## R-57 — The Yoshida-6 coefficients are checked against Yoshida (1990), and the `w₂ < 0` label fixed *(TO-1)*
*24 Sep 2026 · recorded; applied before Yoshida-6 enters the shared source*

Files: dd_integrator :81–90, §6; integrator_contract :366.

## R-58 — The non-Metal parity run gates Paper 2, not the build *(TO-2)*
*24 Sep 2026 · recorded; applied before Paper-2 numbers*

Files: canonical_spec §2, §11; gpu_determinism_note and parity_contract if branch words fork.

## R-59 — Fix D1 to D6 as listed *(sheet §8)*
*24 Sep 2026 · D1–D3 applied in step 5 with R-29 and R-37; D4–D6 applied with R-42, R-27 and R-40*

- D1: the stale escape remnants (integrator_contract :318, :340, :357; dd_integrator :283).
- D2: payload :527's conflation of `closure_min` with the windowed `|Δn̂|`.
- D3: the ledger's missing closure fields.
- D4: the ledger's `alpha` row.
- D5: `has_redundant_hemisphere` at inverse_encode_contract :199.
- D6: the 136/88 B widths.

## R-60 — No t = 0 escape outcome *(closes RQ-19)*
*24 Sep 2026 · applied in step 5*

The windowed escape rule (R-29) can't fire at t = 0. The only valid t = 0 terminal is a collision (within `r_coll`). The
wording applied with R-29 in integrator_contract Part 5 and payload §2 stands.

## R-61 — The escaper's separation is its distance to the other two's centre of mass *(amends R-29)*
*24 Sep 2026 · applied in step 5*

"Largest separation from the other two" means the largest distance `d` to the centre of mass of the other two, the same
`d` as in `E_rel`. Files: integrator_contract Part 7; dd_integrator §3.6; pitfalls §2.2.

## R-62 — `rho_mag` and `lambda_mag` *(amends R-22)*
*24 Sep 2026 · applied in step 5*

The `ICDescriptor` fields for the Jacobi vectors `ρ` and `λ` are `rho_mag` and `lambda_mag`, not `rho0_mag` / `rho1_mag`: a
trailing digit reads as a body index. Files: dd_generation_root §3.6; render_contract :14, :174.

## R-63 — The continuation table is hashed with the ledger *(confirms R-36's application)*
*24 Sep 2026 · applied in step 5*

As written in payload §3: any change to the continuation table changes the schema version automatically.

## R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*
*25 Sep 2026 · applied in step 6*

render_gui_spec Part II stands. gui_state_contract §5's four-slot, fixed-wiring description predates Part II and is
updated to the graph. Files: gui_state_contract §5, §7; render_gui_spec Part II header.

## R-65 — One Inspector window; the standalone IC Inspector is absorbed *(closes RQ-21)*
*25 Sep 2026 · applied in step 6*

The IC Inspector and the trajectory viewer are one Inspector window (the notes, §05). trajectory_viewing §4's panels are
hosted there and in Explore's Trajectory side panel; trajectory_viewing says where each panel lives. The IC Inspector's
reference HTML stays in `docs/gui/reference/` as prior art. Files: trajectory_viewing §4, §6; render_gui_spec §G2, §G8;
ic_inspector_scratchpad (build notes); INDEX.

## R-66 — The time scrubber stays *(closes RQ-22)*
*25 Sep 2026 · applied in step 6*

It sets the display time and re-integrates progressively; it never replays stored frames. The export contract's "no scrub"
applies to exported animations only, and one sentence there says so. Files: export_animation_contract Part 1;
render_gui_spec §G2.

## R-67 — The display chain *(closes RQ-23)*
*25 Sep 2026 · applied in step 6*

stain → style → display scale → gamut clamp → colour-vision simulation → screen. The simulation sees the final in-gamut
colours. The corpus's top display bar is replaced by the Display window plus the Overlays menu (the approved design).
Files: colour_composition §4.3 and the §7 summary; render_gui_spec §G2, §G5, Part II §1, §12, §15.

## R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*
*25 Sep 2026 · applied in step 6*

The corpus's palette hex codes, the substep cap (default 64) and the sound mapping win. The Run window exposes the parameters
the contracts define, under their contract names. The rule is added to the header line of render_gui_spec §G13. Files:
render_gui_spec §G2, §G5, §G13.

## R-69 — What is undoable *(closes the step-6 open question)*
*25 Sep 2026 · applied in step 6*

`SimConfig` and `RenderState` edits are undoable, including navigation (it edits `z₀` and the basis) and lock / unlock (chart
construction). `ViewUI`-only state — open panels, focus, selection, the kept-orbit list — is not. Files: gui_state_contract
§2.

---

*Checkpoint A of step 7 (PR #8). Rulings R-70 to R-96, 25 Sep 2026.*

## R-70 — Where two docs conflict, the later consolidated doc wins
*25 Sep 2026 · applied in step 7*

The other doc is conformed to it. The pairs: the payload doc over scattered payload text; `colour_composition` over
dd_colouring's per-mode sections; `parity_contract` over older determinism wording; the refinement policy over the
scheduler's stop rules.

## R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*
*25 Sep 2026 · applied in step 7*

Every value the corpus doesn't give becomes a **calibration** requirement in the milestone that needs it. The task proposes
the value with its evidence; a reviewer checks it; the human confirms it at the milestone gate; and it is recorded in
`decisions.md`. No value is invented silently.

## R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*
*25 Sep 2026 · applied in step 7*

Missing definitions (κ(z), `xi`, the legal state transitions, the Welford `y`, the τ tie rule, …) are written as doc changes
by the task that needs them, and reviewed by the physics reviewer before merging.

## R-73 — Apply the whole ruling-follow-up checklist now *(closes RQ-56)*
*25 Sep 2026 · applied in step 7*

All of RQ-56, including the items its rulings had scheduled for later: R-23, R-33, R-40 / D6, R-42 / D4, and D5.

## R-74 — The research phases are settled by the vertical slice *(closes RQ-25)*
*25 Sep 2026 · applied in step 7*

canonical_spec §11 governs; philosophy §7.8 is marked superseded. One M3 validation requirement is added: the logH
falsification check of the re-registration mechanism.

## R-75 — The kernel keeps one debug mode *(closes RQ-26)*
*25 Sep 2026 · applied in step 7*

`colour_composition` governs. The kernel keeps only Appendix A's single bring-up mode; UV / DECODE / ROUNDTRIP become
fragment presets. Conform render_contract Part 6, lowering :52 and debug_tooling_plan §A.

## R-76 — Stability × Hue is deleted *(closes RQ-27)*
*25 Sep 2026 · applied in step 7*

As `colour_composition` §4.1 says. Remove it from §7, from §7.1 (the golden list) and from render_contract Part 4; mark
dd_colouring §3.4 superseded.

## R-77 — Replace-L, and the state palette *(closes RQ-28)*
*25 Sep 2026 · applied in step 7*

Replace-L is L = B: the range form with defaults L_min = 0, L_max = 1. The range form stays as the general case. The state
palette is `colour_composition` §1.4's nine canonical classes. The Okabe–Ito / golden-angle rule stays for other categorical
fields.

## R-78 — Real Viénot and Brettel colour-vision simulation *(closes RQ-29)*
*25 Sep 2026 · applied in step 7*

Implement real Viénot (protan, deutan) and Brettel (tritan) through LMS, with golden values from a published reference
implementation. dd_colouring §3.8's matrices are replaced.

## R-79 — NaN and sentinels *(closes RQ-30)*
*25 Sep 2026 · applied in step 7*

Storage never holds NaN. A blown-up sample stores the defined failed-state values. A tier-absent (derived) field reads NaN
at unpack. Every colouring maps NaN or a sentinel to its invalid colour. Debug fields are the stated exception: they show
literal stored values, and NaN still goes to the invalid colour.

## R-80 — Samples per footprint *(closes RQ-31)*
*25 Sep 2026 · applied in step 7*

A footprint has E+1 samples, `copy_index` 0..E. Copy 0 is the un-jittered centre; copies 1..E are Halton points 1..E,
centred (minus ½) and scaled to the footprint.

## R-81 — The embedded record uses the contract names *(closes RQ-32)*
*25 Sep 2026 · applied in step 7*

Rewrite the embedded record against the contract names and the 8-D latent. Drop `jitter_frac` (the footprint fixes the
offsets, R-80). Bump the embedding version.

## R-82 — One mirror test, one seed rule *(closes RQ-33)*
*25 Sep 2026 · applied in step 7*

One mirror test, in encode's frame: mirror iff λ̃_y < −δ_λ (λ̃ = √μ_λ·λ), δ_λ = 1e-12; |λ̃_y| ≤ δ_λ → no mirror. Seed:
among seeds with ‖w⁽²⁾‖²_m > ε_w, take the largest norm; break ties by seed order. Conform all four texts.

## R-83 — The slice scale lives in q *(closes RQ-34)*
*25 Sep 2026 · applied in step 7*

The scale lives in `q` (a common zoom). chart_reference drops `s_u` and `s_v`.

## R-84 — Branch decisions across precisions *(closes RQ-35)*
*Amended by R-297.*
*Still in force: all of it with the compute shaders' fast-math off, the default; with it on, an opt-in, parity is
measured, not exact (R-297).*
*25 Sep 2026 · applied in step 7*

`parity_contract` governs. Branch decisions are identical on identical inputs, per step. Labels on chaotic trajectories may
differ across precisions. Reword the integrator and determinism texts to match. Tier S asserts the outcome class only on
non-chaotic fixtures.

## R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*
*25 Sep 2026 · applied in step 7*

Native in-process `wgpu` sets the Tier-N tolerances. Dawn CI is dropped. Real browsers are checked against those tolerances
with the browser build (M8).

## R-86 — The payload doc governs the eight payload items *(closes RQ-37)*
*Amended by R-313.*
*Still in force: all eight items; `ICDescriptor`'s 64 B with explicit padding is its f32 instantiation, and its
width follows `Real` (R-313).*
*25 Sep 2026 · applied in step 7*

- `times` is exact u16 only; dispatch refuses a configuration with ⌈T/dt⌉ > 65535.
- Phase state is vec2-grouped.
- The debug plan's tables are regenerated from the ledger.
- `ICDescriptor` is 64 B with explicit padding; E₀ is derived (K₀ + V₀), not stored.
- Descriptor bits 8–9 are `last_symbol`; 10–15 are reserved.
- `saturated` ⟺ N_sub == N_max.
- The complexity proxy is ⌊log₂⌋, with 0 for a total ≤ 1.
- One name per accessor, the payload §6 names (`fgw_retained_prefix_length`), with `tm_t_dmin` and `fgw_length_raw` in the
  unpack layer.

## R-87 — `failed_fraction` is retired *(closes RQ-38)*
*25 Sep 2026 · applied in step 7*

Retired in favour of `error_ratio`; its references are removed. "Indeterminate" is read from `error_ratio`.

## R-88 — What stops in-view refinement *(closes RQ-39)*
*25 Sep 2026 · applied in step 7*

Above the screen floor, in-view quads must split (policy §0.1). The criterion may supersample below it. `MAX_REL_DEPTH` caps
only that supersampling depth (`MAX_REL_DEPTH` ≥ the screen floor). Conform scheduler Part 4 and memory_tiers §2.

## R-89 — Depth and E are not on the sim key *(closes RQ-40)*
*25 Sep 2026 · applied in step 7*

Depth is a scheduler knob, not on the sim key. E changes live: copies are cached per `copy_index`, and the nominal's key
excludes E. The ladder has ~8–12 rungs in total.

## R-90 — The decoder switchover trigger *(closes RQ-41)*
*25 Sep 2026 · applied in step 7*

Switch to the linearised decoder when the full decoder's adjacent samples give bitwise-identical ICs, with ℓ_switch = 20 as
an upper bound (whichever comes first). lowering's `SWITCH` = ℓ_switch.

## R-91 — The temporal accumulators feed "unresolved" *(closes RQ-42)*
*25 Sep 2026 · applied in step 7*

Under the same `eps`: a footprint is unresolved if its spread exceeds `eps` now, or its latched running maximum ever did.
θ_s, θ_max, θ_trend and the trend signal are dropped.

## R-92 — What the sim key holds of navigation *(closes RQ-43)*
*25 Sep 2026 · applied in step 7*

The sim key holds the slice plane (z₀'s out-of-plane part, span{q₁, q₂}, the in-plane orientation). In-plane pan and zoom
re-address; slicing out of the plane, tilting and rotating re-integrate; lock changes neither. Conform canonical_spec §4 / §8
and systems_architecture.

## R-93 — The f32 predictability horizon gates the cross-check only *(closes RQ-44)*
*25 Sep 2026 · applied in step 7 · corrected by R-105 · amended by R-119*
*Still in force: all of it, as its text now reads: conformed by R-105 (the re-run is R-35's) and R-119 (`t_max(f32)` is
the GPU measurement).*

§4.1's gate stands: the cross-check runs only for t < t_max(f32). REQ-VAL-070's gate reads the GPU measurement of
t_max(f32) (REQ-VAL-071); R-35's change-10 re-run supplies the f64 figure and the method. t_max annotates refinement; it
doesn't gate it.

## R-94 — The GUI snapshot is ~10 Hz *(closes RQ-45)*
*25 Sep 2026 · applied in step 7*

Throttled (caching Part 6a). egui redraws at frame rate from the latest snapshot. Conform systems_architecture §3.

## R-95 — After escape fires *(closes RQ-48, in part)*
*25 Sep 2026 · applied in step 7 · amended by R-103*
*Still in force: all of it, as its text now reads: conformed by R-103 (escape ends the production loop; the §2.4 march
runs in the harness).*

Once escape fires, `state` reads escape and `t_end` is fixed. Time averages (FTLE's S/T and the like) freeze at t_esc. Any
march for the pitfall §2.4 checks runs only in the validation harness, on its own state; in production `done` is set when
escape fires and the loop ends, and the payload never sees that march. The window is sampled at macro-step
boundaries for unregularised occupants and at sync boundaries for regularised ones. Re-validate against check 2's
independent ground truth, with the legacy t = 30 set kept as a comparison.

## R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*
*25 Sep 2026 · applied in step 7 · last bullet withdrawn by R-106*
*Still in force: every bullet but the last: the palette readings, pointer_channels normative only where cited, the
properties popover and the disc radius ∝ ∛m, one undo entry per drag, and transport in `ViewUI`; the last bullet is
withdrawn, the link ids being the chart's link functions (R-106).*

- Palette reading: "degenerate" = `decode_failed`; "collision at start" = collision with `t_end_step == 0`; `running` shows
  neutral grey; `sim_failed` shows the invalid colour.
- pointer_channels is normative only where render_gui_spec, trajectory_viewing or a ruling cites it.
- The Inspector's right-click properties popover and the disc radius ∝ ∛m are in.
- Undo coalesces a drag into one entry.
- Transport (play / pause / speed / loop) moves from `SimConfig` to `ViewUI`: not undoable, not on the sim key.
- ~~Link ids are specified when the v2 research tools are built.~~ *Withdrawn by R-106.*

---

*Revised checkpoint A of step 7 (PR #8). Rulings R-97 to R-109, 25 Sep 2026. The reviewer also accepted the
interpretations reported with the revision: `tm_t_dmin_step` with no alias; the achromatopsia matrix stays;
`quad.collapsed` is kept and defined where R-90 lands; `rEsc`, `eta` and `nSync` are dropped from the embedded
record; `running`'s neutral grey is a calibration requirement; REQ-VAL-130 and REQ-VAL-133, if one obligation,
become one (the other retired, citing it).*

## R-97 — Quad addresses live in the slice plane *(closes RQ-57)*
*25 Sep 2026 · applied in step 7*

Quad addresses are `(level, i, j)` in the slice plane's own frame, relative to the plane anchor: `z₀`'s value at the
last re-integrating event (R-92). In-plane pan and zoom change which addresses are requested, never the addresses
themselves. Conform deep_zoom §1.

## R-98 — `MAX_REL_DEPTH` caps every split beyond the screen floor *(closes RQ-58)*
*25 Sep 2026 · applied in step 7*

Off-screen policy splits included.

## R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*
*25 Sep 2026 · applied in step 7*

The latch is per footprint; a quad is unresolved if any of its footprints is. The running mean, the first-divergence
time and `S_word` are diagnostics, not split inputs. The latch lives with the resident quad: when the cache evicts or
merges it, the latch goes too, so it never pins memory.

## R-100 — No per-cell Halton rotation *(closes RQ-60)*
*25 Sep 2026 · applied in step 7*

Copy positions are fixed by the pixel alone (R-80), which keeps recreate-from-image exact.

## R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*
*25 Sep 2026 · applied in step 7*

The GUI's clock advances `RenderState`'s playhead each frame through a `SetField` marked "no history". Playback never
enters undo; a manual scrub is one coalesced entry (R-96).

## R-102 — The ensemble isn't a baked variant *(closes RQ-62)*
*25 Sep 2026 · applied in step 7*

`copy_index` is a uniform, and each copy is the same kernel dispatched again (R-89).

## R-103 — Escape ends the production loop; the §2.4 checks run in the harness *(closes RQ-63 and RQ-69)*
*25 Sep 2026 · applied in step 7*

In production, `done` is set when escape fires and the loop ends. The post-escape march for the pitfall §2.4 checks
runs only in the validation harness, which keeps its own state; the payload never sees it. R-95's text is conformed.

## R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*
*25 Sep 2026 · applied in step 7 · corrected by R-141*
*Still in force: `DoubleCover` and its meaning, each shape covered twice as two labelled systems; its site is the
full-range Burrau chart (R-157), not the shape sphere (R-141).*

"Covers each shape twice, as two labelled systems." Lowering's shape-sphere row names it.

## R-105 — R-93's re-run is R-35's *(closes RQ-65)*
*25 Sep 2026 · applied in step 7*

Confirmed: R-93's "R-84 re-run" means R-35's change-10 re-run. R-93's text is corrected.

## R-106 — The link ids are the chart's link functions *(closes RQ-66)*
*25 Sep 2026 · applied in step 7*

The link ids in `SimConfig` and on the sim key are the chart's link functions (per-block registry entries), which were
already correct. R-96's last bullet is withdrawn. Linked views for side by side are a separate `ViewUI` item, specified
with the v2 research tools.

## R-107 — Apply the RQ-67 follow-ups; GUI_DESIGN_NOTES may be conformed *(closes RQ-67)*
*25 Sep 2026 · applied in step 7*

`GUI_DESIGN_NOTES.md` may be edited where a ruling contradicts it; each edit is marked "conformed to R-n". The decisions
override it.

## R-108 — Must-split above the floor is the at-rest target *(closes RQ-68)*
*25 Sep 2026 · applied in step 7*

During a gesture the frame budget governs: ancestors show, so there are never blanks, and completeness resumes at rest.

## R-109 — Only pointer_channels §4 is normative *(closes RQ-70)*
*25 Sep 2026 · applied in step 7 · amended by R-144*
*Still in force: pointer_channels §4 is normative through render_gui_spec's "listen", its open points become R-71 and
R-72 requirements, and the rest stays working notes; §3 is normative too (R-144).*

Through render_gui_spec's "listen". Its open points (the reference pitch, θ alone or stereo θ/φ, the whole or a
windowed spectrum) become R-71 / R-72 calibration and definition requirements. The rest of the file stays working notes.

---

*Checkpoint B of step 7 (PR #8). Rulings R-110 to R-133, 25 Sep 2026. RQ-102 and RQ-103 (the prin-rs occupants and
fixtures) wait for the human. ✱ marks a design ruling the human may veto.*

## R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*
*Amended by R-186.*
*Still in force: the unit, property, numerical-gate and native golden suites on every commit; GUI screenshots on GUI PRs
and at the gates; lavapipe as the second backend on every commit, meeting M4's two-backend check, and a real non-Metal
GPU gating Paper 2 (R-58); Chrome stable and Safari for REQ-VAL-116; goldens rendered with native wgpu offscreen from
M1, checked at M8 by the Playwright suite against the same baselines within tolerance, with no re-baselining without a
gate decision; the runners and the benchmarks' schedule are R-186's.*
*25 Sep 2026 · applied in step 7*

- Unit, property, numerical-gate and native golden suites run on every commit. Benchmarks run nightly and at each
  milestone gate. GUI screenshots run on GUI PRs and at the gates.
- GPU CI: a self-hosted Apple-silicon runner (Metal), plus lavapipe as the second backend on every commit. lavapipe
  satisfies M4's two-backend check; a real non-Metal GPU gates Paper 2 (R-58).
- Browsers for REQ-VAL-116: Chrome stable and Safari.
- Goldens render with native wgpu offscreen from M1. At M8 the Playwright browser suite checks against the same
  baselines within tolerance; no re-baselining without a gate decision.

## R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*
*25 Sep 2026 · applied in step 7*

Rename `SimResult` → `SimState` in the display chain (render_gui_spec :199, GUI_DESIGN_NOTES under R-107,
REQ-COL-043), and `M` → `n_renorm` in the uniform echo. The vocabulary lint covers code and docs, excluding passages
wrapped in `<!-- retired-terms -->` … `<!-- /retired-terms -->`.

## R-112 — The archived briefs' standing parts are superseded *(closes RQ-81)*
*25 Sep 2026 · applied in step 7*

The archived briefs' standing parts are superseded by the consolidated docs (deep_zoom §3, scheduler, parity).
TASK-M0-02 checks that each standing obligation exists in a consolidated doc, and ports any that doesn't. Then conform
INDEX and REQ-SYS-008. Archives are never cited.

## R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*
*25 Sep 2026 · applied in step 7*

Every proposed fix and split in RQ-93 to RQ-100 is accepted as written, with these choices:
- RQ-93, QuadReduction: option (b), sizing at M5.
- REQ-VAL-135 moves to M3 (the runner and the gate stay in M0).
- RQ-99, M5-2: option (a), REQ-DEC-036 moves to M5.
- M5-3 and M5-5: the proposed splits. The native frame loop runs on a dedicated render thread.
- RQ-95, M2-G5: the M2 goldens as proposed. The M8 projection selector and hemisphere toggle live in the Manifold
  view's Chart section, shown when the chart is the shape sphere.

## R-114 — The debug NaN guard is the bitcast test *(closes RQ-82)*
*25 Sep 2026 · applied in step 7*

The debug NaN guard is the bitcast test against the canonical quiet-NaN pattern. Drop `raw != raw`.

## R-115 — The raw `state` view keeps six colours *(closes RQ-83)*
*25 Sep 2026 · applied in step 7*

The raw `state` debug view keeps a six-colour `dbg_cat` palette. R-77 governs the outcome palette (state ⊕ detail)
only.

## R-116 — The fragment decode and encode are generated from the one source *(closes RQ-84)*
*Amended by R-297.*
*Still in force: all of it; the agreement presets compare within a stated tolerance, a calibration requirement, not
bit-exactly (R-297).*
*25 Sep 2026 · applied in step 7*

The fragment WGSL decode and encode are generated from the one Rust source (rust-gpu → SPIR-V → WGSL translation),
never hand-written. The agreement presets check translation. Conform colour_composition §3 and §6: two compilation
paths of one source.

## R-117 — The lowering appendix's shape-sphere row uses (θ, φ) *(closes RQ-85)*
*25 Sep 2026 · applied in step 7*

The lowering appendix's shape-sphere row is conformed to R-14's (θ, φ) map.

## R-118 — Φ is generic over the float type *(closes RQ-86)*
*25 Sep 2026 · applied in step 7*

Φ is generic over the float type (lowering Part 2): chart_reference §5.1 reads `map<F: Float>`. `validate()` stays
CPU-side f64.

## R-119 — `t_max(f32)` is the GPU measurement *(closes RQ-87)*
*25 Sep 2026 · applied in step 7*

REQ-VAL-070's gate reads REQ-VAL-071's GPU measurement. The R-35 re-run supplies the f64 figure and the method.
Conform R-93 and predictability §4.1.

## R-120 — Eviction takes the lowest cost-weighted resistance first *(closes RQ-88)*
*25 Sep 2026 · applied in step 7*

Eviction takes the lowest cost-weighted resistance first (scheduler Part 6, caching Part 7). The pinned classes are
unchanged. Conform telemetry's "deepest first" passages.

## R-121 — The physics overlay isn't baked *(closes RQ-89)*
*25 Sep 2026 · applied in step 7*

The physics overlay isn't baked: it's a per-fragment occupant. The hoist, when masses are constant over the slice, is
an allowed optimisation, not a bake tier. Conform render_contract Parts 3 and 4.

## R-122 — The reference HTML files are the colour oracle *(closes RQ-90 and RQ-101)*
*25 Sep 2026 · applied in step 7 · amended by R-151*
*Still in force: all of it except cubehelix, whose reference is the analytic form (R-151).*

The two reference HTML files (`docs/gui/reference/principia_colour_explorer.html` and `principia_colour_presets.html`)
are named the golden oracle in colour_composition §7. Formulas follow the oracle: the explorer's blob weight and its
sequential clamped mix; conform dd_colouring §3.4. LUT data comes from the published matplotlib tables (viridis,
cividis, plasma, magma, inferno, twilight, cubehelix) and Moreland's table for cool-warm. The Principia palette's
stops come from the explorer.

## R-123 — Achromatopsia is a fifth Display mode *(closes RQ-91)*
*25 Sep 2026 · applied in step 7*

Achromatopsia is offered in the Display window as a fifth mode.

## R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*
*25 Sep 2026 · applied in step 7*

Apply all three now (R-25, R-50, R-102 in systems_architecture §5.5). This unblocks TASK-M4-05 and TASK-M4-06.

## R-125 — The branch-cut convention is M3's, transcribed from the literature *(closes RQ-96)*
*25 Sep 2026 · applied in step 7*

REQ-PAY-070 moves to M3, closed by TASK-M3-16. Its convention (the a/b branch-cut assignment and the crossing sign)
is transcribed from the literature (Montgomery; Šuvakov–Dmitrašinović) and verified by REQ-VAL-043 against the
published braid classes. REQ-PAY-071 and REQ-PAY-072 are v2: per-pair views are out of v1 (PL-3, R-38). Retire them
from the v1 plan with that citation.

Transcribed with citations, physics-reviewed, confirmed at the gate.

## R-126 — The Euler landmarks are the Euler central configurations *(closes RQ-106)*
*25 Sep 2026 · applied in step 7*

The Euler landmarks are the Euler central configurations (the collinear relative equilibria: roots of Euler's quintic
in the mass ratios), mapped through the shape map. Transcribed with citation. Equal masses reduce to the antipodes of
b̂. Physics-reviewed, confirmed at the gate.

## R-127 — Periodic-orbit stability is Floquet; the Poincaré sections are three *(closes RQ-109)*
*25 Sep 2026 · applied in step 7*

Periodic-orbit stability uses the monodromy matrix's Floquet multipliers. A fold is where a multiplier crosses +1;
period-doubling, where one crosses −1. Poincaré sections offered: syzygy crossings (w = 0), a chosen shape-sphere great
circle, and a Jacobi-coordinate hyperplane. Transcribed with citations, physics-reviewed, confirmed at the gate.

## R-128 ✱ — The checkerboard's ceiling form is chosen at the M5 gate *(closes RQ-104)*
*25 Sep 2026 · applied in step 7 · design; the human may veto*

The task captures both the hard gate and the ramp at the ceiling; the human chooses at the M5 gate.

## R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*
*25 Sep 2026 · applied in step 7 · design; the human may veto*

Custom quality fields: in the Run window, under "quality: Custom". Target-utilisation ceiling: Run window. Arbiter
overlay: a Profiler tab. Passive logging: a Profiler switch, with its indicator in the footer. Until the M8 dev GUI
they're checked by presence only, not layout.

## R-130 ✱ — The styles are the poster's *(closes RQ-107)*
*25 Sep 2026 · applied in step 7 · design; the human may veto*

The styles are defined by the poster's implementation (`workbench/principia_poster_both_sides.html` and its press
module): "watercolour & pencil" is its painted treatment; "print" is the paper.design halftone presets with its patches
(per-plate slip, cell ceiling). v1 offers plain, watercolour & pencil, and the seven print presets.

## R-131 — The video encoders *(closes RQ-108)*
*25 Sep 2026 · applied in step 7*

Native: PNG frames, GIF, and MP4 through a system ffmpeg when present. Browser: PNG frames (zipped), GIF via a wasm
encoder, and MP4/WebM through WebCodecs where supported.

## R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*
*25 Sep 2026 · applied in step 7*

- REQ-COL-055: the invalid colour must collide with no palette entry. Propose a hatched pattern, not a flat colour.
- REQ-INT-081: `dt_macro = max(1e-3, T/65535)`. T keeps [50, 200], with a default of 50; u16 indices stay exact
  (R-86).
- REQ-PERF-086: Ultra and Extreme cap N at 16, and scale through E and render scale (the values are calibrated).

## R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*
*Amended by R-297.*
*Still in force: all seven interpretations, except that "f32 noise" in REQ-COL-006 is read as REQ-DEC-043's calibrated
f32 decode factor only until TASK-M2-29 calibrates the agreement presets' tolerance under fragment fast-math, which the
agreement gate then uses (REQ-COL-060, R-297). The rest of that interpretation stands: "Tier-N tolerance" in
REQ-TOOL-029 is read as REQ-DEC-043's factor until REQ-VAL-064 sets Tier N, and the agreement preset compares against
the decode stage's E₀ = K₀ + V₀ (R-86), not SimState.E_0.*
*25 Sep 2026 · applied in step 7*

All seven interpretations in RQ-111 are accepted.

---

*Rulings on RQ-112 to RQ-118 (found while applying R-110 to R-133), 25 Sep 2026.*

## R-134 — When the Playwright suite and the aggregate survey run *(closes RQ-112)*
*25 Sep 2026 · applied in step 7*

From M8, the Playwright browser suite runs nightly, on GUI and colour PRs, and at each gate. The aggregate survey runs
nightly and before release (parity §6).

## R-135 — The across-copy reduction is its own resolve pass *(closes RQ-113)*
*25 Sep 2026 · applied in step 7*

The across-copy reduction is its own resolve pass. It runs after all E+1 copy dispatches for a quad complete, reads
their SimState slices, and writes the footprint resolve and the QuadReduction fields. The "serial copies improve load
balance" subsection keeps its text as a "Was" note (R-102), not deleted.

## R-136 — `debug_invalid(frag_xy)` draws the hatch; sentinels show their value *(closes RQ-114)*
*Amended by R-245.*
*Still in force: `debug_invalid(frag_xy)` draws the hatch, a stored sentinel shows as its value, and only NaN gets the
hatch; `diffusion` is no longer a sentinel (R-245).*
*25 Sep 2026 · applied in step 7*

`DEBUG_NAN` becomes a function, `debug_invalid(frag_xy: vec2<f32>) -> vec3<f32>`, which draws the hatch from the pixel
position. The two stay distinct: debug views show a stored sentinel such as −1.0 as its literal value on the ramp
(R-79's exception), and only NaN gets the hatch.

## R-137 — The Ultra and Extreme rows are provisional *(closes RQ-115)*
*25 Sep 2026 · applied in step 7*

Mark the Ultra and Extreme rows, and the memory totals that depend on them, as provisional pending R-132's calibration.
Recompute the totals when the values land.

## R-138 — `dt_macro` is derived, shown read-only *(closes RQ-116)*
*25 Sep 2026 · applied in step 7*

`dt_macro` is derived, not editable. The Run window shows it read-only, with its rule: `max(1e-3, T/65535)`.

## R-139 — Turbo is Google's table; the §7.1 labels name the oracle files *(closes RQ-117)*
*25 Sep 2026 · applied in step 7*

Turbo uses Google's published Turbo table (Mikhailov 2019, Apache-2.0). Rename the §7.1 group labels to the two oracle
HTML files (R-122).

## R-140 — The four readings are accepted *(closes RQ-118)*
*25 Sep 2026 · applied in step 7*

All four readings in RQ-118 are accepted.

---

*Rulings on RQ-71 to RQ-78 (open since step 7's first pass) and RQ-119 to RQ-126 (checkpoint B's plan pass),
25 Sep 2026.*

## R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*
*25 Sep 2026 · applied in step 7 · amended by R-157*
*Still in force: the shape sphere is 2-to-1 over its φ hemispheres, and `DoubleCover` is retired for the shape sphere
only; it stays the full-range Burrau chart's value (R-157).*

The shape sphere's φ hemispheres are reflection-equivalent: the canonical decode gauges `λ̃_y → −λ̃_y`, so both
hemispheres decode to the same system. Its `system_image` is the existing n-to-1 category with n = 2 ("2-to-1 over the φ
hemispheres"). `DoubleCover` is retired for the shape sphere; it stays the full-range Burrau chart's value (corrected by R-157). Conform chart_decoder Part 5,
lowering :159, chart_reference :349 and inverse_encode :202.

## R-142 — The latch is evaluated on the GPU; only its verdict returns *(closes RQ-72)*
*25 Sep 2026 · applied in step 7*

The latch is evaluated on the GPU, in the resolve pass (R-135), and its state stays in GPU-resident per-quad memory.
`QuadReduction` carries only the verdict: the count of unresolved footprints, latched ones included. `QuadReduction` stays
the sole automatic return.

## R-143 — The live tree contains the static tree *(closes RQ-73)*
*25 Sep 2026 · applied in step 7*

The "bitwise the static tree at the horizon" claim is withdrawn. It now reads: the live tree contains the static tree at
the horizon, and they are equal when footprint spreads are monotone in time. A merge doesn't drop a latch while the quad
is resident (R-99). A calibration requirement measures the latch's cost (the extra resident quads) on the named slices.

## R-144 — pointer_channels §3 is normative too *(closes RQ-74, amends R-109)*
*25 Sep 2026 · applied in step 7*

pointer_channels §3 (responsiveness) is normative beside §4, since it already superseded trajectory_viewing §1. The
banner stays. R-109 now reads §3 and §4.

## R-145 — The fragment side reads `has_ensemble` as a uniform *(closes RQ-75)*
*25 Sep 2026 · applied in step 7*

The fragment side reads `has_ensemble` as a uniform, not a bake, like the compute side (R-102). Toggling E never
re-bakes. E = 0 still reads `ensemble_spread` as NaN.

## R-146 — The crate layout is confirmed *(closes RQ-76)*
*25 Sep 2026 · applied in step 7*

The crate layout is confirmed as proposed. `web/` uses Vitest for unit tests and Playwright for the browser suites.

## R-147 — The R-97 to R-109 follow-ups are applied *(closes RQ-77)*
*25 Sep 2026 · applied in step 7*

Apply both. The sampling note's heading becomes "The sampling pattern: deterministic Halton offsets" (its citations are
re-pointed). chart_reference :468 records R-27 (both charts kept), with its application still "before any Burrau
statistic".

## R-148 — The validation harness renders the "off" image *(closes RQ-78)*
*25 Sep 2026 · applied in step 7*

The `stop_on_escape` "off" image is rendered by the validation harness, whose own march continues past escape. The
regression (REQ-EVT-014, tolerance REQ-EVT-023) stands there.

## R-149 — The colour suite runs on Chromium and WebKit *(closes RQ-119)*
*25 Sep 2026 · applied in step 7*

The colour suite runs on the R-110 pair: Playwright's Chromium and WebKit (Playwright's Safari engine).

## R-150 — REQ-INT-048's GPU arm leaves M3 *(closes RQ-120)*
*25 Sep 2026 · applied in step 7*

REQ-INT-048's GPU arm is dropped from M3, like the other four; REQ-VAL-072 covers it at M4.

## R-151 — Cubehelix's reference is the analytic form *(closes RQ-121, amends R-122)*
*25 Sep 2026 · applied in step 7*

Cubehelix's reference is the analytic form with dd_colouring's parameters (s = 0.5, λ = 1.5, h = 1). The matplotlib
cubehelix function, called with the same parameters, is a cross-check only.

## R-152 — A minimal Profiler window holds the Arbiter tab at M6 *(closes RQ-122)*
*25 Sep 2026 · applied in step 7*

TASK-M6-22 builds a minimal Profiler window shell holding the Arbiter tab; TASK-M8-28 fills in the rest.

## R-153 — The debug and live-march views have golden images of their own *(closes RQ-123)*
*25 Sep 2026 · applied in step 7*

The debug views (TASK-M1-12) and live-march views (TASK-M3-22) are checked against golden images of their own, recorded
at the gate, not against artboards.

## R-154 — REQ-DEC-036 is verified over a depth sweep at M5 *(closes RQ-124)*
*25 Sep 2026 · applied in step 7*

At M5, REQ-DEC-036 is verified over a depth sweep. The check at the actual switchover depth joins REQ-DEC-037 at M6.

## R-155 — One GIF encoder for both builds *(closes RQ-125)*
*25 Sep 2026 · applied in step 7*

One GIF encoder for both builds: the Rust `gif` crate (MIT or Apache-2.0), with `color_quant` for palettes, compiled to
wasm for the browser.

## R-156 — The plan-pass readings are accepted *(closes RQ-126)*
*Reversed in part by R-183 (its size exemption).*
*Still in force: RQ-126's other readings, including removing TASK-M6-13; TASK-M0-06's size exemption is reversed
(R-183).*
*25 Sep 2026 · applied in step 7*

All readings in RQ-126 are accepted, including removing TASK-M6-13 and letting TASK-M0-06 run slightly over the size
guideline.

---

*Rulings on RQ-127 and RQ-128 (found while applying R-141 to R-156), 25 Sep 2026.*

## R-157 — `DoubleCover` stays, for the full-range Burrau chart *(closes RQ-127, amends R-141)*
*25 Sep 2026 · applied in step 7*

`DoubleCover` stays. It is the `system_image` of the full-range Burrau chart (R-27): there the leg swap relabels the
bodies, so each shape is covered twice, as two labelled systems. R-141 applies to the shape sphere only, which is n-to-1
with n = 2, its hemispheres reflection-equivalent. R-141's text is corrected ("no current chart has two labelled
systems" is wrong), and `DoubleCover`'s definition is restored where R-141 marked it "Was", naming the Burrau chart as
its site. chart_reference §4.5 :477 stands. REQ-CHART-014 and REQ-CHART-025 are unblocked.

## R-158 — The six readings are accepted *(closes RQ-128)*
*25 Sep 2026 · applied in step 7*

All six readings in RQ-128 are accepted.

---

*Rulings on RQ-102 and RQ-103, from the prin-rs survey (prin-rs commit `8600d45`), 25 Sep 2026. With them checkpoint B
and PR #8 are ready for final approval. ✱ marks a design ruling the human may veto.*

## R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*
*25 Sep 2026 · applied in step 7*

prin-rs is the human's own repo; importing is authorised. The minimal text and code set is copied into
`docs/reference/prin-rs/`, pinned to `8600d45`, with a README: "reference, not authority (R-1). Transcribe into the
contracts and cite; never cite these files as normative." The set:
- `src/integrate/heggie/*`, `src/integrate/logh/*`, the predictive step-limit code;
- `src/grid.rs` (slices), `src/ensemble/stats.rs`, `src/testing.rs` (the pulse);
- `examples/integrator_gallery.rs`, `wedge_census.rs`, `logh_arms.rs`;
- `FINDINGS.md` and the `NOTES.md` excerpts the corpus cites;
- `tools/xcheck/` and `reference/*.py`.

Images and `results/` are not imported; they are cited by commit (`8600d45`, and `70cfbc4` for the original 256² data).

## R-160 — The integrator equations are transcribed into integrator_contract Part 2b
*25 Sep 2026 · applied in step 7*

The Heggie, logH and TTL equations, the step control, and the predictive step limit
(`dτ ≤ f·d_min / (|v_rel|_max·A·B)`, f = 0.02) are written into integrator_contract Part 2b, with citations: Heggie 1974;
Mikkola & Tanikawa 1999; Preto & Tremaine 1999. Physics-reviewed, confirmed at the M3 gate.

## R-161 ✱ — Heggie's default time transformation is the measured one, Eq. 22 at n = 3/2
*25 Sep 2026 · applied in step 7 · design; the human may veto*

Heggie's default time transformation is the one measured: Eq. 22 at n = 3/2 (`dτ = S^{3/2}/(R₁R₂R₃) dt`), as prin-rs's
code runs it. `FINDINGS.md:96`'s Eq. 20 statement is a documentation error, noted in the reference README. Eq. 20 stays
as a selectable time mode. M3's re-run of the 32-case matrix confirms the default.

## R-162 — The reversible occupant is logH's TTL time mode
*25 Sep 2026 · applied in step 7*

The reversible occupant is logH's TTL time mode (Mikkola–Tanikawa), as prin-rs built it. Aarseth–Zare with
Mikkola–Tanikawa isn't required. integrator_contract Part 2a ("required") and REQ-VAL-044 are conformed to "a
reversible occupant exists: logH-TTL". `NOTES.md:2573`'s verdict (it's built, validated, and loses on accuracy) is
recorded as a prior finding.

## R-163 — The wedge ablation is re-run on Heggie
*25 Sep 2026 · applied in step 7*

REQ-INT-052 stays on the shipping configuration. M3 re-runs the wedge ablation (the three switches, and
`wedge_census.rs`'s density: at least 25% pale pixels in a 9×9 window at 1024²) on Heggie, with Aarseth–Zare kept for
comparison.

## R-164 — +0.305 and −0.082 are correlations, not controls
*25 Sep 2026 · applied in step 7*

+0.305 and −0.082 are FTLE–drift Spearman correlations, not control ICs; the Aarseth–Zare value is a null result
(`NOTES.md:2077`). Every requirement and RQ that calls them controls is rewritten to cite them as prior findings. There
is no gate on them.

## R-165 — The 32-case figure is 3915 → 74
*25 Sep 2026 · applied in step 7*

Quote 3915 → 74, the current regeneration at `8600d45`. Note the original 3916 → 73 at `70cfbc4`.

## R-166 — The fixtures
*25 Sep 2026 · applied in step 7*

- (a) The slice definitions are transcribed from `src/grid.rs` into `fixtures/slices.toml`: `config_stability`,
  `config_basin`, the five Burrau regions, `tilt_plambda`, `preset_shape`. Each corpus mention of "the config slice" is
  resolved by context; any that stay ambiguous go to REVIEW_QUEUE.
- (b) The 32-case matrix → `fixtures/case_matrix.toml`, from `integrator_gallery.rs:138-176`. err>10 means
  `error_ratio > 10` (`stats.rs:71-88`).
- (c) The legacy t = 30 set isn't stored data. M3's validation harness regenerates it: the config slices at t = 30
  under the legacy classifier, whose code is imported as reference. The result is checked in as a fixture.
- (d) BodyPlane: M3 records the Python reference's output (`tools/xcheck`, `reference/*.py`) as the fixture.
  REQ-VAL-028 reads: bit-exact where the operation order is identical (parity_contract), otherwise within a calibrated
  tolerance (R-71).
- (e) The moving pulse is a synthetic field (`src/testing.rs:162`), not a chart slice. The corpus wording is corrected,
  and its definition is transcribed into `fixtures/`.

## R-167 — The prin-rs licence is not blocking
*25 Sep 2026 · applied in step 7*

The human may add one to prin-rs later.

---

*Rulings on the cold-read review (`docs/archive/untangling/REVIEW_cold_read.md`, PR #9), 25 Sep 2026. The finding ids (G1, A1, …) are the
review's. Applied in step 8.*

## R-168 — REQ-INT-014 covers the built occupants; Aarseth–Zare + TTL is an allowed future occupant *(follows R-162)*
*25 Sep 2026 · applied in step 8 · fills the unused number before R-169*

The number R-168 was never used in step 7. It is taken here by the human, issued with the cold-read rulings.
Aarseth–Zare + time-transformed leapfrog isn't built in v1 (R-162). REQ-INT-014 is narrowed to the occupants that are built.
It isn't retired, and TASK-M3-02 still closes it. integrator_contract Part 2a keeps the row as an allowed future occupant,
not a requirement.

## R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*
*Amended by R-186.*
*Still in force: TASK-M0-04's GPU jobs (Metal, and ubuntu with lavapipe), `PRIN_GPU_BACKEND=metal|vulkan`, and an
install step for every toolchain in the task that first needs it; the Metal job runs on GitHub-hosted `macos-15`, not a
self-hosted runner (R-186).*
*25 Sep 2026 · applied in step 8*

TASK-M0-04 adds the GPU CI jobs: a macOS arm64 self-hosted job (Metal) and an ubuntu job with `mesa-vulkan-drivers`
(lavapipe). `GpuHarness` picks its backend from `PRIN_GPU_BACKEND=metal|vulkan`. Every toolchain gets an install step
in the task that first needs it: `.nvmrc` for Node (TASK-M2-08), apt for Mesa (TASK-M0-04), `rust-toolchain.toml` for the
rust-gpu pin (TASK-M0-14).

## R-170 — The crate map *(closes G2)*
*25 Sep 2026 · applied in step 8 · the map awaits the human's confirmation*

Before TASK-M0-01, systems_architecture §7 gains a "Crate map": each node of its graph assigned to a crate (`kernel`,
`ledger`, `engine`, `render`, `gui`, `validation`, `prin`, `xtask`). The agent drafts it from §7 and R-146; the human
confirms it; REQ-SYS-004's lint checks the crate graph against it.

## R-171 — The convergence gate, defined *(closes A1, A2, T2)*
*25 Sep 2026 · applied in step 8*

- Samples are ordered coarse → fine: strides 32, 4, 1, 0, where stride 0 is unstrided (the finest).
- r_k = |x_k − x_{k−1}| / |x_{k−1}|.
- Pass iff r_k is strictly decreasing and the finest r_k is below the threshold.

The canonical sequence (strides 32, 4, 1, 0: 0.2153, 0.4423, 0.5494, 0.0947) gives r = 1.054, 0.242, 0.828. It fails,
because r isn't strictly decreasing. (The ruling as issued quoted the absolute steps 0.227, 0.107, 0.455; the human
confirmed the relative values are the ones recorded.) M0 uses a provisional threshold of 0.1, named against REQ-VAL-135
and marked provisional until M3 calibrates it. A passing fixture with strictly shrinking steps is added.

## R-172 — There is no contract crate *(closes C1, C4)*
*25 Sep 2026 · applied in step 8*

The typed surfaces live in `crates/engine/src/contract/` (R-146). TASK-M0-01 and TASK-M0-08 are fixed, and so are the
stale "pending" notes.

## R-173 — Tau is split into a provisional and a confirmed value *(closes C2, C3, S3, G6)*
*25 Sep 2026 · applied in step 8*

- REQ-EVT-024 (TASK-M3-12) is the provisional tau, stated in the task file and taken inside the provisional range
  7.04e-05 … 2.70e-02. prin-rs used `CLOSURE_TAU = 1e-3` (`src/outcome.rs:263` at `8600d45`), so that value is used and
  cited. It is inside the range, and near its geometric mean (≈ 1.4e-03).
- REQ-EVT-025 (TASK-M3-34), added, is the confirmed tau, inside the re-measured gap. (The ruling as issued named them
  024a and 024b; ids take the next free number, and the human confirmed this numbering.)

TASK-M3-12's stale note is removed.

## R-174 — The self-hosted runner runs only this repository's code *(closes H1)*
*Amended by R-186.*
*Still in force: outside contributors need approval before any workflow runs (the approval setting R-186 keeps); the
fork guard (GPU jobs only for pushes and this repository's PRs, fork PRs getting the CPU and lavapipe jobs) and the
runner's own macOS user account are dormant while no self-hosted runner exists (R-186), and apply again when one is
added.*
*25 Sep 2026 · applied in step 8*
*Dormant since R-186 (26 Sep 2026): no self-hosted runner exists. The text stands, and applies again when one is added.*

GPU jobs on the self-hosted runner run only for pushes and for PRs from this repository:
`if: github.event.pull_request.head.repo.full_name == github.repository` (or the event is a push). Fork PRs get the CPU
and lavapipe jobs only. The runner runs under its own macOS user account. Outside contributors need approval before any
workflow runs.

## R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*
*Amended by R-260 and R-261.*
*Still in force: the `VERDICT` review headings, `reviews-complete` passing only when every named role has approved on
the latest commit, and the merge by the human or a bot; R-260 carries an approval over qa's own test commit, and R-261
passes a PR naming no task.*
*25 Sep 2026 · applied in step 8*

GitHub won't let one account approve its own PR, so each reviewer posts a PR review headed `VERDICT: APPROVE <role>` or
`VERDICT: CHANGES <role>`. A CI check, `reviews-complete`, passes only when every role the task file names has approved
on the latest commit. The human merges, or a merge bot does once all the checks are green.

## R-176 — Controls come before the tests that need them *(closes G3, S2)*
*Amended by R-199.*
*Still in force: TASK-M0-02 and TASK-M0-03 depend on TASK-M0-04 and register controls for their xtask tests, controls
reach xtask tests through a dev-dependency on `crates/validation`, and crates without the `controls` feature are
skipped; the name matching is R-199's.*
*25 Sep 2026 · applied in step 8*

TASK-M0-02 and TASK-M0-03 depend on TASK-M0-04, and register controls for their xtask tests. Controls reach xtask tests
through a dev-dependency on `crates/validation`. Tests are matched to their controls by a shared test-name attribute.
Crates without the `controls` feature are skipped, not failed.

## R-177 — Cadence *(closes G4, C6)*
*25 Sep 2026 · applied in step 8*

- `ci.yml` runs `cargo xtask ci` on every push, and that includes plan-check.
- `nightly.yml` (scheduled) runs bench and the survey.
- A GUI PR is one touching `crates/gui/**` or `docs/gui/**`; screenshots run on those.
- `gate.yml` (`workflow_dispatch`, input: milestone) runs every suite and writes the gate report.

Bench is removed from the per-commit `ci`.

## R-178 — The shape-sphere round trip composes Φ alone *(closes A3)*
*25 Sep 2026 · applied in step 8*

TASK-M2-08's round-trip gate composes Φ alone. The canonicaliser's fold is checked separately, on the upper hemisphere
(R-141).

## R-179 — Sim data is per-sample payload; QuadReduction is allowed *(closes A4, T3)*
*25 Sep 2026 · applied in step 8*

"Sim data" means per-sample `SimState` payload. `QuadReduction` is a reduction: it is the one permitted automatic
GPU→CPU return (R-142), and it is allowed.

## R-180 — pr-check is item-level *(closes A5)*
*25 Sep 2026 · applied in step 8*

The PR template gives each error meter and discriminator its own line (`- meter: <name> — <statement>`,
`- discriminator: <name> — <statement>`), and pr-check parses those lines.

## R-181 — The shape-sphere round-trip bound *(closes T1)*
*25 Sep 2026 · applied in step 8*

TASK-M2-08's round trip is bounded by ≤ 1e-13 absolute, as a provisional calibration confirmed at the M2 gate.

## R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*
*25 Sep 2026 · applied in step 8*

TASK-M3-12's seven escape fixtures are defined in the task, with their parameters, as an R-72 definition. Any tolerance
a PR proposes is used by CI provisionally until its gate confirms it.

## R-183 — TASK-M0-06 is split *(closes S1; reverses R-156's size exemption)*
*25 Sep 2026 · applied in step 8*

TASK-M0-06 splits into the golden and repro runner (TASK-M0-06) and the screenshot runner (TASK-M0-20, the next free id).

## R-184 — The minor fixes *(closes G5, G7, G8, A6, A7, A8, C5, C7)*
*25 Sep 2026 · applied in step 8*

- TASK-M3-12 cites gpu_determinism_note rule 6 and integrator_contract Part 4.
- The TASK-M2-08 fixture: `ic_inspector.html`'s script is extracted with a pinned script and run on the `.nvmrc` Node
  version, and CI reads the checked-in fixture without regenerating it.
- The OKLab reference is Ottosson's published reference code, with its URL and revision pinned in the task.
- Open-question citations point at specific entries.
- The screenshot runner's argument is `<suite>`.
- TASK-M0-02 includes the `check_plan.py` change that names archive faults.
- TASK-M2-08 says R-14 supersedes R-12's axis wording.
- The INDEX file count is updated.

## R-185 — The crate map is confirmed; kernel → ledger is a build-dependency only *(closes TASK-M0-00)*
*26 Sep 2026 · applied in step 8*

The crate map in systems_architecture §7.1 is confirmed, with one change: kernel's dependency on ledger is a
build-dependency only (the ledger generates code into the kernel at build time). The kernel stays no_std so rust-gpu
can compile it.

## R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*
*26 Sep 2026 · applied in step 8*

Start with GitHub-hosted runners, and no self-hosted runner.
- Per-commit CI: `ubuntu-latest` for the CPU suites and the lavapipe (`mesa-vulkan-drivers`) GPU suites; `macos-15`
  (GitHub-hosted, Apple silicon, paravirtual Metal) for the Metal correctness suites only.
- Benchmarks and every performance gate run on the human's own Mac via `prin profile`, at milestone gates and on
  demand, never on hosted runners. The nightly workflow runs the CPU and lavapipe suites and the survey, not benchmarks.
- TASK-M0-04 adds a first check: on `macos-15`, wgpu finds a Metal adapter and the M0 golden fixture renders within
  tolerance. If it fails or is flaky, stop and raise a REVIEW_QUEUE entry proposing the self-hosted runner (the agent
  then scripts the setup; the human approves it).
- R-174's fork guard is no longer needed while no self-hosted runner exists. Its text is kept, marked dormant until one
  is added.
- HUMAN_SETUP.md drops runner registration. What remains is the approval setting and branch protection.

*Placement (applied in step 8):* the golden-image runner and its M0 fixture (`fixtures/golden/selftest/`) are built
in TASK-M0-06, which depends on TASK-M0-04, and the fixture's tolerance is REQ-VAL-138, calibrated there. So the check
is split. TASK-M0-04 checks, on `macos-15`, that wgpu finds a Metal adapter and that the harness's M0 fixture
(the identity dispatch) round-trips bit-exact. TASK-M0-06 runs `golden selftest` on `macos-15` within REQ-VAL-138's
tolerance. The same stop-and-raise rule governs both.


## R-187 — kernel and ledger may take validation as a dev-dependency; validation never depends on prin *(closes RQ-129)*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

1. Yes: kernel and ledger may take validation as a dev-dependency only (§7.1's "any (dev-dependency only) →
   validation" holds for every crate except gui). Never as a normal or build dependency; the no_std kernel and
   rust-gpu builds never see it. Condition: in kernel and ledger, any test that uses validation must be an integration
   test (tests/), not a unit test inside src/, because the dev-dependency cycle gives unit tests two copies of the
   crate. xtask deps enforces it.
2. No: validation may not depend on prin. Where validation needs the CLI, it runs the built binary as a separate
   process.

Fix §7.1 line 324 to say "ledger depends on nothing, kernel on nothing but ledger — normal and build dependencies;
dev-dependencies per line 322".

*Applied (TASK-M0-01):* §7.1's "any (dev-dependency only) → `validation`" row read "any", with no gui exception; the
ruling's parenthetical says it holds "for every crate except gui". The ruling's words are applied: the row now reads
"any except `gui`", and `xtask deps` forbids `gui` → `validation` in every kind. Under R-176, gui's tests then have no
route to `negative_control!`; a crate without the `controls` feature is skipped, not failed. The `validation` row
(line 321) now excludes `prin` as well as `gui`. Line 324 cites "the `validation` row above" rather than a line number.

## R-188 — TASK-M0-01 is accepted over its size; the source scan also follows `include!`
*Amended by R-189 (its item 2).*
*Still in force: item 1, TASK-M0-01's size acceptance; item 2 is gone (R-189, then R-191).*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

Asked in review of PR #16, the human chose:
1. "Accept, record it": PR #16 stays one PR, at about three times TASK-M0-01's ~450-line budget (+1507 / −6 at
   4250b46, not counting `Cargo.lock`, fixtures and qa's files). The overage comes from R-187's source scan, which the
   task did not have when it was sized. `plan/WORKFLOW.md` § "Task files" ("One task is one reviewable PR") is waived
   for this task only.
2. "Yes, close it": in kernel and ledger `src/`, `cargo xtask deps` follows `include!` string paths as it follows
   `#[path]`, and scans the file; a path it can't resolve (built with `concat!`, `env!` and the like) fails the check.

## R-189 — kernel and ledger `src/` use neither `#[path]` nor `include!` *(amends R-188 item 2)*
*Amended by R-190; superseded by R-191.*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

Asked in review of PR #16 how to close an `include!` or `#[path]` inside a `macro_rules!` body (rustc resolves it at
the call site), the human chose "Forbid #[path]/include!": in kernel and ledger `src/`, `#[path]` (including under
`cfg_attr`) and `include!` are forbidden outright, and `cargo xtask deps` fails on any occurrence, naming the file and
line. The R-187 scan then covers the `.rs` files under `src/` only, and no longer follows `#[path]` or `include!` into
other files. R-188 item 2 ("follows `include!` string paths as it follows `#[path]`") is replaced by this. The
check that kernel's and ledger's targets sit under `src/` stays. qa gets a one-round exception to update or remove its
own tests that expect a `#[path]` or `include!` to be followed.

## R-190 — kernel `src/` may include the ledger's generated code from `OUT_DIR`; everything else `include`-shaped fails *(amends R-189)*
*Superseded by R-191.*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

R-189 forbade every `include!` in kernel `src/`, which also forbade the usual route by which R-185's generated code
("the ledger generates code into the kernel at build time") reaches the kernel. Asked in review of PR #16, the human
chose "Allow the OUT_DIR form":
- In kernel `src/`, exactly one form is allowed: `include!(concat!(env!("OUT_DIR"), "/<literal>.rs"))`, at item level,
  not inside a macro body and not through an alias. Ledger `src/` allows no `include!` at all, as R-189 has it.
- Everything else fails `cargo xtask deps` in kernel and ledger `src/`: any other `include` identifier (which covers
  `use std::include as …` and `include` passed to a macro), any `#[path]` (as R-189), and any attribute whose contents
  include a macro variable (`#[$a]`, `#[$($t)*]`, `#[cfg_attr(…, $a)]`).
- qa's suggested rule, failing on any `path` followed by `=` anywhere, is not adopted. It would catch ordinary bindings
  such as `let path = …`. The macro-variable attribute rule closes the same bypass.

## R-191 — R-187's integration-test condition is checked by compiling, not by reading tokens *(amends R-189, R-190; closes RQ-130)*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

The token scan behind R-187's condition ("in kernel and ledger, any test that uses validation must be an integration
test") was bypassed in review of PR #16 again and again by macro constructions: aliases, metavariable attributes,
attributes assembled from `tt` fragments (RQ-130), shadowed builtins. Asked whether to replace it, the human chose
"Compile check":
- `cargo xtask deps` copies the workspace to a temporary directory, removes `validation` from kernel's and ledger's
  dev-dependencies there, and runs `cargo check -p kernel -p ledger --lib --tests`. Any use of the validation crate
  by a unit test, whatever the route (alias, macro, `#[path]`, `include!`), fails to compile, and the check fails,
  showing the compiler's error.
- The token rules of R-189 and R-190 are lifted: `#[path]`, `include!` (the `OUT_DIR` form included) and attributes
  holding macro variables are no longer forbidden by `xtask deps`. A local item named `validation` is allowed again.
  The token scanner is removed. The check that kernel's and ledger's targets sit under `src/` stays.
- RQ-130 is moot and closed by this ruling.
- qa gets a one-round exception to replace or remove its own token-level tests with compile-level ones.

*Applied (TASK-M0-01, 42ccd3c):* the command as written, `cargo check -p kernel -p ledger --lib --tests`, would also
compile kernel's and ledger's integration tests, which R-187 allows to use `validation`. The ruling's intent is applied
instead: in the temporary copy, kernel's and ledger's integration-test, example and bench targets are left out, so only
the lib and bin unit tests are compiled without `validation`. Which features the check compiles is RQ-131.

## R-192 — The compile check builds with `--all-features` *(closes RQ-131)*
*Superseded by R-194.*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

Asked which features R-191's compile check builds, the human chose "--all-features": the check runs with
`--all-features`, so a unit test behind any feature of kernel or ledger is compiled without `validation`. If kernel or
ledger ever gains mutually exclusive features, that is ruled on then.

## R-193 — The compile check is host-only; that is its known limit *(closes RQ-132)*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

"Accept host-only, and record it as the check's known limit. Kernel and ledger unit tests run on the host by nature
(the SPIR-V target can't run a test harness), so the check sees every unit test that can exist. Any platform-gated
unit test that ever appears gets ruled on then."

*Applied note:* "the host" is not one platform. The check runs on Linux in CI, while tests also run on the human's
Mac, so a unit test gated on `target_os = "macos"` would run there with `validation` linked, unseen by the Linux
check. The check therefore sees every unit test that can exist on Linux. The ruling stands: such a test is ruled on
when it appears.

## R-194 — The compile check builds three feature sets in two profiles; doctests may use validation *(amends R-192; closes RQ-133)*
*26 Sep 2026 · applied in TASK-M0-01 (PR #16)*

Asked in review of PR #16, the human chose:
1. "3×2 matrix + known limit": the check runs with `--no-default-features`, with default features and with
   `--all-features`, each in the dev and the release profile. Any other cfg combination (for example a test gated on
   feature `a` on and `b` off) is a known limit, ruled on if it ever appears, as R-193 does for platforms.
2. "Yes, like integration tests": a kernel or ledger doctest may use `validation`. rustdoc compiles each doctest as a
   separate crate that links the library from outside, so R-187's two-copies problem cannot arise. The check does not
   compile doctests.

## R-195 — CI checks formatting and lints
*26 Sep 2026*

"CI runs cargo fmt --check and cargo clippy --workspace --all-targets -- -D warnings on every push."

*Applied (PR #17, merged as 1caad55; recorded under R-292):* in `.github/workflows/ci.yml` and REQ-SYS-067. The
toolchain step installs `rustfmt` and `clippy`, and two steps run before `cargo build --workspace`: `cargo fmt --all
--check` and `cargo clippy --workspace --all-targets -- -D warnings`. The workflow's header comment cites R-195. One
`cargo fmt --all` commit formatted the workspace, qa's test files included. Clippy's one finding, a
`clippy::type_complexity` in `xtask/tests/deps.rs`, became RQ-134, which R-197 rules.

## R-197 — Who may fix, suppress or configure a lint *(closes RQ-134)*
*26 Sep 2026*

RQ-134: "option (a). Factor the tuple in xtask/tests/deps.rs into a small named struct; no allow, no config change."

Standing rule, "so this doesn't recur":
- A lint fix that doesn't change behaviour (renaming, restructuring, simplifying) may be made by the implementer and
  approved by the code reviewer. It doesn't go to REVIEW_QUEUE.
- Suppressing a lint (`#[allow]`) needs a comment giving the reason, and the code reviewer's explicit approval of that
  suppression.
- Changing lint configuration (`clippy.toml`, `[lints]` tables) needs a ruling.

*Recorded before R-196, which was given earlier and is recorded with TASK-M0-04.*

## R-196 — Mutation testing joins the QA gate
*Amended by R-302.*
*Still in force: per PR, `cargo mutants --in-diff` on the changed code, sharded across parallel CI jobs under R-302;
nightly, a full run written as a report; every surviving mutant in a PR's diff is a qa finding; the three exclusions;
a REVIEW_QUEUE entry, not a dropped run, if per-PR runs prove impractical.*
*26 Sep 2026 · given before R-197, recorded with TASK-M0-04*

"mutation testing joins the QA gate.
- M0-04 adds cargo-mutants to CI: per PR, `cargo mutants --in-diff` on the changed code (with a per-PR time limit);
  nightly, a full run written as a report.
- Every surviving mutant in a PR's diff is a QA finding: kill it with a test, or justify it as equivalent in the
  review. QA's checklist gains that line.
- Excluded: generated code, GPU-only (spirv-gated) paths and xtask's own harness plumbing.
- If the time limit makes per-PR runs impractical, raise it in REVIEW_QUEUE rather than dropping it."

*Open when recorded:* the per-PR time limit has no value; it becomes a calibration requirement (R-71). The nightly
workflow is TASK-M0-19's deliverable, so the nightly run cannot land before it. Where R-196's work goes is RQ-135.

*Applied per R-204 — veto? (RQ-162, 30 Sep 2026):* cargo-mutants excludes only by file glob or mutant-name regex, so
"GPU-only (spirv-gated) paths" is met by a marker: every item gated on `target_arch = "spirv"` also carries
`#[cfg_attr(test, mutants::skip)]`, and a missing marker fails closed. "xtask's own harness plumbing" is
`xtask/src/main.rs` and `xtask/src/codegen.rs` only; xtask's checks stay mutated.

## R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*
*26 Sep 2026 · applied in TASK-M0-04*

Asked in RQ-135, the human chose "Accept the split":
- TASK-M0-04 keeps the GPU harness (`gpu.rs`, `PRIN_GPU_BACKEND`, the identity and `extractBits` self-tests, the
  `metal_hosted_probe`, the `gpu-metal` and `gpu-lavapipe` CI jobs) and `prop.rs`, and closes REQ-SYS-065.
- TASK-M0-21: the control registry and `cargo xtask controls`, with its fixture tests; not yet in `cargo xtask ci`.
- TASK-M0-22: controls for every test merged before it, and `controls` registered in `cargo xtask ci`; closes
  REQ-VAL-007.
- TASK-M0-23: R-196's per-PR `cargo mutants --in-diff` job, its exclusions, the per-PR time limit as a calibration
  requirement, and qa's checklist line.
- R-196's nightly full run joins TASK-M0-19, which creates `nightly.yml`.
- TASK-M0-02 and TASK-M0-03 depend on TASK-M0-22 instead of TASK-M0-04.

## R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*
*Amended by R-212.*
*Still in force: no test-name attribute and no proc-macro crate; `negative_control!` names its test, and `cargo xtask
controls` pairs tests and controls by that name; R-212 adds the expected panic message.*
*26 Sep 2026 · applied in TASK-M0-04*

Asked in RQ-136, the human chose "Name in the macro": there is no test-name attribute and no proc-macro crate.
`negative_control!(test_name, "description", control)` names its test, and `cargo xtask controls` pairs tests and
controls by that name. R-176's "Tests are matched to their controls by a shared test-name attribute" is replaced by
this; its dev-dependency route to `crates/validation` and the skipping of crates without the `controls` feature stand.

## R-200 — TASK-M0-22 is accepted at ~650 lines; TASK-M0-16 depends on it *(closes RQ-137)*
*Superseded in part by R-209 (its size acceptance).*
*Still in force: TASK-M0-16 depends on TASK-M0-22, so TASK-M0-16 to TASK-M0-18 register their own controls; the size
acceptance gave way to R-209's split.*
*26 Sep 2026 · applied in TASK-M0-04*

Asked in RQ-137, the human chose "Accept ~650, mechanical": TASK-M0-22 stays one PR over the size budget, as
repetitive registration of one control per test, as R-188 did for TASK-M0-01. And "Yes, depend on M0-22":
TASK-M0-16 depends on TASK-M0-22, so M0-16 to M0-18 register their own controls and TASK-M0-22 covers only the
tests merged before it (TASK-M0-01's, TASK-M0-04's and TASK-M0-21's).

## R-201 — A kernel or ledger unit test's control is registered from that crate's `tests/`, by name *(closes RQ-138)*
*26 Sep 2026 · applied in TASK-M0-04*

Asked in RQ-138, the human chose "Control in tests/, by name": because R-187 keeps `validation` out of kernel's and
ledger's unit tests, a unit test there has its control registered from the same crate's integration tests
(`tests/`), paired by the test's name (R-199). `cargo xtask controls` pairs tests and controls across a crate's
targets.

## R-202 — A surviving mutant fails the per-PR job unless it is a listed, justified equivalent *(closes RQ-139)*
*26 Sep 2026 · applied in TASK-M0-04*

Asked in RQ-139, the human chose "Fail; list justified ones": the per-PR `cargo mutants --in-diff` job goes red on
any surviving mutant not in a checked-in list of equivalent mutants. Each entry carries a one-line justification,
and the code and qa reviewers approve it, as R-197 does for lint suppressions.

## R-203 — The shared proptest case count is a calibration requirement, 256 provisional *(closes RQ-140)*
*26 Sep 2026 · applied in TASK-M0-04*

Asked in RQ-140, the human chose "Calibration, 256 provisional": the case count is a calibration requirement (R-71)
closed by TASK-M0-04. 256 is the proposed value, used provisionally and marked so (R-182); the human confirms or
changes it at the M0 gate.

## R-204 — When to ask the human
*Amended by R-264 (its "exceeding budgets").*
*Still in force: all of it, except that size and budgets are no longer a question for the human (R-264).*
*26 Sep 2026 · given as "R-198", which was already taken (the TASK-M0-04 split); recorded as R-204*

"Standing rule: when to ask me.

Don't ask me when the answer follows from an existing ruling or the docs. That covers:
- sequencing and dependencies (like M0-16 → M0-22 under R-176);
- splits within the size budget;
- mechanical consequences of a ruling;
- wording and citation fixes;
- anything where your recommendation is just "apply R-n".
Apply it, and record it in decisions.md as "applied per R-n: <what>" (or in the PR description), so I can see it and
veto it later.

Ask me only for genuine choices:
- physics or conventions;
- design and GUI behaviour;
- numeric values and calibrations;
- conflicts the rulings don't settle;
- scope or cost trade-offs (dropping or deferring anything, exceeding budgets);
- anything irreversible.

Batch what isn't blocking: collect those questions and ask them once, when the PR is ready, not one at a time.

If you're unsure which kind it is, apply the recommendation, mark it "applied per R-198 — veto?" in the PR, and carry
on."

*Applied note:* the marker is written "applied per R-204 — veto?", the number this rule is recorded under.

## R-205 — TASK-M0-04 is accepted at 570 code lines
*26 Sep 2026 · applied in TASK-M0-04*

Asked when PR #18 was ready (R-204, exceeding a budget), the human chose "Accept as one PR": TASK-M0-04's 570 code
lines, against the ~500 budget and its ~380 estimate, stay one PR. The overage is the inline controls each test
carries until TASK-M0-21's registry exists, rustfmt wrapping, and R-203's added test.

## R-207 — Export trace also writes the Chrome Trace Event format
*27 Sep 2026*

"Export trace also writes the Chrome Trace Event format (openable in Perfetto and chrome://tracing), alongside
profiler schema v1: CPU scopes as complete events, GPU passes on their own track, counters as counter events. Add it
to REQ-TOOL-098's task (M8), with a test that a captured trace loads and round-trips its span count. Apply without
asking; it's an addition, not a choice."

*Applied:* `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender" and `docs/gui/principia_render_gui_spec.md`
§ "Profiler" name the second format; a new requirement, closed by TASK-M8-28 (which closes REQ-TOOL-098), carries it
and its test.

## R-206 — An unset `PRIN_GPU_BACKEND` defaults by platform
*27 Sep 2026*

"When PRIN_GPU_BACKEND is unset, default by platform: metal on macOS, vulkan elsewhere. An explicit value still
overrides, and an unknown value is still an error. CI keeps setting it explicitly. Log which backend was chosen, so a
test run always says what it ran on."

*Applied:* R-169 says only that `GpuHarness` picks its backend from `PRIN_GPU_BACKEND`; the "unset is an error" rule
was TASK-M0-04's (its Deliverables and its `gpu_backend_env` acceptance line), and those lines are amended here.
REQ-SYS-065's statement gains the default. The code, and the merged tests that assert an unset value fails (qa's
among them, under a one-round exception as a mechanical consequence, R-204), change in the same PR.

## R-208 — TASK-M0-21 is accepted at 762 code lines; later overruns are split first
*Amended by R-211 and R-264.*
*Still in force: TASK-M0-21's size acceptance; the three `cargo xtask controls` rules it confirmed (a doctest counts as
a test without a control; a control covers exactly one test, and a name with more tests than controls fails naming them;
`negative_control` is a reserved test name); and the `deps.rs` fix, each test workspace with its own `CARGO_TARGET_DIR`,
shown to pass with one outside the repo; its split-first rule is R-264's orchestrator choice now.*
*27 Sep 2026 · applied in TASK-M0-21*

"Merge-level decisions for PR #19:
- Size overrun accepted (record as R-208). From now on, if a task looks set to exceed ~500 lines, propose a split in
  REVIEW_QUEUE before implementing it.
- All three "applied per R-204" items stand.
- Fix the flaky xtask/tests/deps.rs tests in the R-206/R-207 PR: give each test workspace its own CARGO_TARGET_DIR (a
  temp dir per test), so they never share the outer build directory. Add a test showing it passes with
  CARGO_TARGET_DIR set outside the repo.
Then R-206/R-207, then TASK-M0-22."

*Applied:* TASK-M0-21's 762 code lines (fixtures and qa's tests excluded), against the ~500 budget, stay one PR. The
three items confirmed are PR #19's "applied per R-204 — veto?" rules for `cargo xtask controls`, in a crate that
declares the `controls` feature: a doctest counts as a test without a control; a control covers exactly one test, and
a name with more tests than controls fails naming them; `negative_control` is a reserved test name. So TASK-M0-22 turns
the `ignore` example in `crates/validation/src/control.rs` into a non-test block. The `deps.rs` fix lands in the
R-206/R-207 PR.

## R-209 — TASK-M0-22 is split three ways, each part closing its own requirement *(closes RQ-141)*
*27 Sep 2026 · applied in TASK-M0-22*

Asked in RQ-141, the human chose "Split, new reqs": the three-way split the implementer proposed, where each of the
two earlier parts gets a new requirement of its own, so every task still closes one (`plan/WORKFLOW.md` § "Task
files").

*Applied:* the earlier parts take new ids, so every task that depends on TASK-M0-22 still waits for the last part:
- TASK-M0-24 (part a): controls for the implementer's tests merged before TASK-M0-22 (validation's `gpu` and `prop`
  unit tests; xtask's `ci.rs`, `controls.rs`, `deps.rs`), xtask's `controls` feature and validation dev-dependency,
  and the `control.rs` example made a non-test block (R-208). Closes REQ-VAL-152.
- TASK-M0-25 (part b): controls for qa's validation and prin tests, and the child-mode move (R-210). Closes
  REQ-VAL-153.
- TASK-M0-22 (part c): controls for qa's xtask tests, and `controls` registered in `cargo xtask ci`. Still closes
  REQ-VAL-007, and now depends on TASK-M0-24 and TASK-M0-25.
Supersedes R-200's size acceptance; R-200's dependency of TASK-M0-16 on TASK-M0-22 stands.

## R-210 — Subprocess bodies leave libtest *(closes RQ-142)*
*27 Sep 2026 · applied in TASK-M0-25*

Asked in RQ-142, the human chose "Move out of libtest": a subprocess body a test spawns is not itself a `#[test]`. It
lives in a `harness = false` test target (or a bin) whose `main` the parent spawns, so `cargo xtask controls` never
lists it. The three qa child-mode helpers (`qa_child_open_harness`, `qa_child_failing_property`,
`qa_child_count_cases`) move there, under a one-round exception to edit qa's merged files, like R-206's, with qa
reviewing.

## R-211 — TASK-M0-24 and TASK-M0-25 accepted; the size budget counts implementation only *(closes RQ-143, RQ-144)*
*Amended by R-223 and R-264.*
*Still in force: the size budget counts implementation code and the implementer's own tests, leaving out qa's commits
and `negative_control!` blocks, and TASK-M0-24's and TASK-M0-25's sizes stand; which lines count is R-225's, and whether
to split is the orchestrator's (R-264).*
*27 Sep 2026 · applied in TASK-M0-22*

"Both sizes accepted. From now on the ~500-line budget counts implementation code and the implementer's own tests
only; QA test commits and negative_control! blocks don't count. Pre-split only if implementation alone looks set to
exceed ~500 (this applies to TASK-M0-22)."

*Applied:* TASK-M0-24 (638 lines, PR #23) and TASK-M0-25 (610 lines, PR #24) stay one PR each. `plan/WORKFLOW.md`
§ "Task files" and its split rule say what the budget counts. It amends R-208's split-first rule, which now applies
to the counted lines.

## R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*
*27 Sep 2026 · applied in TASK-M0-22*

"negative_control! takes the expected panic message (should_panic(expected = …)), so a control must trip its intended
assertion. Convert existing controls in M0-22."

*Applied:* R-199's form gains the expected message; how it is written in the call is the implementation's (the task
says). Every control registered before TASK-M0-22 is converted there. A new requirement carries it, since
REQ-VAL-147 is closed.

## R-213 — R-210's exception extends to `qa_r206_harness_opens_the_selected_backend` *(closes RQ-146)*
*27 Sep 2026 · applied in TASK-M0-22*

"R-210's exception extends to qa_r206_harness_opens_the_selected_backend."

*Applied:* its child path moves out of libtest, into the `qa_child` bin TASK-M0-25 created, under the same one-round
exception to edit qa's merged file (`crates/validation/tests/qa_R-206.rs`), with qa reviewing. TASK-M0-25 is merged,
so TASK-M0-22 does it.

## R-214 — Children are spawned through one helper with a timeout *(closes RQ-147)*
*Amended by R-217.*
*Still in force: one shared spawn helper in `crates/validation`, used by every test that spawns a child, which on
timeout kills and fails naming the child, its timeout a provisional calibration confirmed at the M0 gate (R-71); the
kill takes the whole process group and the timeout is 300 s (R-217).*
*27 Sep 2026 · applied in TASK-M0-22*

"One shared spawn helper with a timeout; on timeout it kills the child and fails naming it. 120 s provisional,
calibration confirmed at the M0 gate. Tests that spawn children use it."

*Applied:* the helper lives in `crates/validation`. Every test that spawns a child process uses it, including qa's
merged files, under a one-round exception limited to replacing the spawn call. The 120 s value is a calibration
requirement (R-71), provisional in CI until the human confirms it at the M0 gate (R-182).

## R-215 — The veto items on PRs #23 and #24 stand; duplicated controls and copied checks are consolidated
*Extended by R-218.*
*Still in force: all of it; R-218 extends its shared-module rule to inputs a control shares with its test.*
*27 Sep 2026 · applied in TASK-M0-22*

"All stand. M0-22 also removes the inline controls duplicated by registered ones, and moves checks copied between test
files into a shared test-support module."

*Applied:* the items that stand: `--features controls` on the gpu-metal and gpu-lavapipe jobs; `metal_hosted_probe`'s
control asserting the other platform's expectation; inline controls kept beside registered ones (until TASK-M0-22
removes the duplicates); controls copying their test's check (until TASK-M0-22 moves the copies into the shared
module). Editing qa's merged files to use the shared module is limited to replacing the copied check with a call, under
a one-round exception, with qa reviewing.

## R-216 — TASK-M0-22 is split three ways under R-211 *(closes RQ-148)*
*27 Sep 2026 · applied in TASK-M0-22*

Asked in RQ-148, the human chose "Three tasks": parts 1 and 2, part 4, and parts 3 and 5 of the implementer's split.
Each earlier part gets a requirement of its own, as R-209 did, and the last keeps REQ-VAL-007.

*Applied:*
- TASK-M0-26 (parts 1 and 2, ~350 counted lines): the spawn helper, every spawn moved to it and R-213's move
  (REQ-VAL-155, REQ-VAL-156), and the expected messages (REQ-VAL-154, narrowed to R-212; its R-215 half moves to the
  two new requirements below).
- TASK-M0-27 (part 4, ~540, about half deletions): qa's copied checks moved into shared test-support modules
  (REQ-VAL-157).
- TASK-M0-22 (parts 3 and 5, ~285): the implementer's duplicated inline controls removed and its copied checks
  replaced (REQ-VAL-158), the controls for qa's 33 xtask tests and `controls` in `cargo xtask ci` (REQ-VAL-007). It
  depends on TASK-M0-26 and TASK-M0-27, so every task waiting on it still waits for all three.

## R-217 — TASK-M0-26's size accepted; a timed-out child's whole process group dies; the timeout is 300 s provisional *(amends R-214)*
*27 Sep 2026 · applied in TASK-M0-26*

"On PR #27: size accepted. The timeout item is vetoed; fix it before merging:
- Spawn every child in its own process group (process_group(0) on Unix). On timeout, kill the whole group: SIGTERM,
  then after a 5 s grace SIGKILL, and reap it. Nothing a child started may survive its timeout.
- Add a test: a child that starts a grandchild and then hangs. After the timeout, both are gone, and no process from
  the group remains.
- The 120 s provisional is too tight (a cold run was killed at 120 s). Raise it to 300 s provisional, covering cold
  builds with margin. It's still confirmed at the M0 gate, from cold and warm measurements on CI and my Mac.
The error-return item stands."

*Applied note:* the human gave this as "R-216", which already records TASK-M0-22's split (RQ-148); it is recorded here
as R-217. "Size accepted" is PR #27's size as it merges: 532 counted lines at the head reviewed, plus the fix this ruling requires, against the ~500 budget. The timeout item vetoed is PR
#27's "applied per R-204 — veto?" item 2 (kill only the direct child); item 1 (the helper returns `io::Result`)
stands. R-214's "kills the child" now means the child's whole process group, and its 120 s becomes 300 s, still
provisional (R-71, R-182). REQ-VAL-155 and REQ-VAL-156 carry both; TASK-M0-26 applies them.

## R-218 — Inputs a control shares with its test live in the shared module too *(extends R-215)*
*27 Sep 2026 · applied in TASK-M0-22*

"R-215's shared-module rule extends to inputs a control must share with its test (shader text, fixtures, constants),
not just checks. M0-22 moves the duplicated GPU shader text in qa_TASK-M0-04_controls.rs into the shared support
module, under the same exception for QA's files."

*Applied:* REQ-VAL-159 (new) carries it, closed by TASK-M0-22. R-215's one-round exception for editing qa's merged
files covers this move: a copied input is replaced by a reference to the shared one, and nothing else changes, with qa
reviewing. An input that is the control's own (a mutated kernel, a contaminated input) is not a copy and stays with the
control.

## R-219 — Reviewers never share a checkout
*27 Sep 2026 · applied in CLAUDE.md and `.claude/agents/`*

"Reviewers never share a checkout. The orchestrator gives each reviewer its own git worktree and its own
CARGO_TARGET_DIR, and removes them afterwards. Add this to CLAUDE.md and the agent definitions."

*Applied:* each reviewer runs in a worktree the orchestrator makes at the PR head (detached), with its own
`CARGO_TARGET_DIR`, both named in the dispatch. The read-only checks (`git status --porcelain`, HEAD unmoved) and qa's
one-commit check run in that worktree; the orchestrator pushes qa's commit from it
(`git push origin HEAD:task/<TASK-id>`), then removes the worktree and the target directory. Process only
(section_notes); no requirement changes.

## R-220 — TASK-M0-27's size accepted; its veto item stands
*27 Sep 2026 · applied in TASK-M0-27*

"PR #29: size accepted (R-217), and the veto item stands."

*Applied note:* the human cited this as "R-217", which already records TASK-M0-26's ruling; it is recorded here as
R-220, after the two rulings the human numbered in the same message. The size is PR #29's as it merged, against the
~500 budget (R-211). The item that stands is PR #29's "applied per R-204 — veto?" item 1: `qa_TASK-M0-21_r2.rs` uses the
shared fixture and xtask helpers instead of its own variant.

## R-221 — TASK-M0-22 is split four ways under R-208 *(closes RQ-149)*
*27 Sep 2026 · applied in TASK-M0-22*

Asked in RQ-149, the human chose "Split": the four-way split RQ-149 lists as option 2. Each earlier part gets a
requirement of its own, as R-209 and R-216 did, and the last keeps REQ-VAL-007.

*Applied:*
- TASK-M0-28 (part (a), ~400 counted lines): the implementer's duplicated inline controls removed and its copied gpu and
  prop checks replaced (REQ-VAL-158), and the shader text shared (REQ-VAL-159, R-218).
- TASK-M0-29 (part (b1), ~480): qa's helpers and inline checks in `xtask/tests/qa_TASK-M0-01.rs` and `_live` moved into
  shared test-support modules, moves only (REQ-VAL-160).
- TASK-M0-30 (part (b2), ~420): the same for `_r191`, `_r193` and `_r194` (REQ-VAL-161).
- TASK-M0-22 (part (c), ~60): the 33 controls and `controls` in `cargo xtask ci` (REQ-VAL-007). It depends on
  TASK-M0-28, -29 and -30, so every task waiting on it still waits for all four.
The edits to qa's merged files in TASK-M0-29 and TASK-M0-30 are limited to replacing a helper or an inline check with a
call to the shared module, under R-215's one-round exception, with qa reviewing.

## R-222 — TASK-M0-28's size accepted; its veto items stand
*27 Sep 2026 · applied in TASK-M0-28*

"PR #31: size accepted, and all three veto items stand."

*Applied note:* PR #31 counted 572 lines against the ~500 budget (R-211), 379 of them deletions. The items that stand:
two removed inline controls broader than the registered ones that replace them
(`deps_a_unit_test_of_a_binary_with_test_false_fails`, one manifest form of three; `deps.rs:251`'s exactly-one-edge
check); `words()` kept as the controls' own input; `metal_hosted_probe`'s control asserting the other platform's
expectation through the test's own check.

## R-223 — The size budget counts added and changed lines, not pure deletions *(amends R-211)*
*Superseded by R-225.*
*27 Sep 2026 · applied in plan/WORKFLOW.md*

"So size stops needing a ruling every PR: the ~500-line budget counts added and changed lines only. Pure deletions
don't count (R-211's exclusions still apply). A PR within budget under this rule needs no size question."

*Applied:* a PR's counted size is its added lines (`+` lines of the diff; a changed line shows as one `-` and one `+`,
and counts once, by its `+`), in implementation code and the implementer's own tests, leaving out qa's commits and
`negative_control!` blocks (R-211). A PR within ~500 on that count raises no size question; a task is pre-split
(R-208) only if that count looks set to exceed ~500.

## R-224 — The two flaky tests are fixed before TASK-M0-29
*Amended by R-230.*
*Still in force: both flaky tests fixed at their cause, not by retries, in TASK-M0-31 before TASK-M0-29; the proof is
R-230's, 5 full runs under normal load and green CI.*
*27 Sep 2026 · applied in TASK-M0-31*

"Fix both flaky tests next, before TASK-M0-29, as one small follow-up:
- qa_m0_26_a_control_without_an_expected_message_does_not_compile: give the test and its control separate target
  directories, as R-208 did for deps.rs.
- qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on: find the actual cause of the failure under load
  (timing, shared resources, a timeout), fix that, and show the reason in the PR. Don't just add retries.
- Prove each fix: run the whole workspace suite 20 times under load, with no failures.
Then TASK-M0-29."

*Applied:* TASK-M0-31 (new, REQ-VAL-162) carries it, and TASK-M0-29 depends on it. Both tests are in qa's merged files
(`crates/validation/tests/qa_TASK-M0-26.rs`, `qa_TASK-M0-25.rs`), so the fix edits them under a one-round exception
limited to what the fix needs, with qa reviewing, as R-215's did.

## R-225 — Lines moved verbatim don't count toward the size budget; TASK-M0-30's size accepted *(amends R-223)*
*27 Sep 2026 · applied in plan/WORKFLOW.md and TASK-M0-30*

Asked whether code moved verbatim counts toward the budget (TASK-M0-30 counted 657 added lines under R-223, nearly all
of them qa's helpers moved into support modules), the human chose "Moves don't count": accept TASK-M0-30, and from now on
lines moved verbatim — the same text, shown by the diff as a deletion and an addition — count like deletions; only new
or changed lines count.

*Applied:* a PR's counted size is its added lines, less those that are the same text as a line the same diff deletes
(a verbatim move), in implementation code and the implementer's own tests, leaving out qa's commits and
`negative_control!` blocks (R-211, R-223). A line moved and then edited counts. PR #33 (TASK-M0-30) is accepted on size.

## R-226 — The suite stops re-running every control; folded into TASK-M0-22
*27 Sep 2026 · applied in TASK-M0-22*

Asked whether to stop the suite running every control twice (the survey on 27 Sep found `controls_on_this_workspace_skips_gui`, `xtask/tests/controls.rs:167`, re-running every control in the workspace through `xtask controls`, and qa's `qa_m0_24_every_registered_control_makes_its_test_fail`, `crates/validation/tests/qa_TASK-M0-24.rs:271`, doing a smaller re-run — together about a third of the suite's time), the human chose "Fold into M0-22": once TASK-M0-22 makes `cargo xtask controls` its own CI step, those two tests check only what they claim, through a listing-only mode, without re-running every control.

*Applied:* REQ-VAL-163 (new) carries it, closed by TASK-M0-22. The CI step must fail on any finding, so no coverage is lost. qa's merged `qa_TASK-M0-24.rs` changes only as far as this needs, under a one-round exception, with qa reviewing, as R-215's did.

## R-227 — The two remaining latent races are fixed now, in their own task
*Amended by R-230.*
*Still in force: the two remaining latent races fixed at their cause in TASK-M0-32; the proof is R-230's.*
*27 Sep 2026 · applied in TASK-M0-32*

Asked about three latent races of R-224's kind, none yet seen failing, the human chose "One task now": fixed at their cause, in a small task like TASK-M0-31, with a loaded multi-run proof.

*Applied:* the first race (`qa_TASK-M0-26.rs`'s r206 test spawning `qa_child` while sibling cargo runs rewrite it) is inside R-224's exception for that file, and qa's review of PR #35 found it blocking, so TASK-M0-31 fixes it. The other two go to TASK-M0-32 (new, REQ-VAL-164): `cargo build -p xtask` in `crates/validation/tests/support/qa_m0_21_fixture.rs:35` rewriting the `xtask` binary while sibling tests spawn it; and parallel copies of one fixture building into the shared outer target (`xtask/tests/controls.rs:63`, `qa_m0_21_fixture.rs:143`, `support/qa_m0_21.rs:16`, `crates/validation/tests/expected_message.rs:42`). The proof is R-224's: 20 whole-workspace runs in a row under load. Edits to qa's merged files are limited to the fix, under a one-round exception, with qa reviewing. TASK-M0-32 depends on TASK-M0-31, and TASK-M0-22 depends on TASK-M0-32 (both edit `xtask/tests/controls.rs`).

## R-228 — Each agent builds and tests with four jobs and four test threads
*28 Sep 2026 · applied in CLAUDE.md and the local build environment*

"Each agent: CARGO_BUILD_JOBS=4 and RUST_TEST_THREADS=4."

*Applied note:* the human's message of 28 Sep gave five speed items under "R-228 to R-231". They are recorded in its
order: this one; the reviewers' scope and the qa-only re-check together as R-229; the proofs as R-230; the task as R-231.
The cap is set in the shared agent environment file (outside the repo); CLAUDE.md records it. Process only.

## R-229 — Reviewers run the tests their diff touches; CI runs the full suite
*28 Sep 2026 · applied in CLAUDE.md, plan/WORKFLOW.md and `.claude/agents/`*

"Reviewers run the tests their diff touches plus dependents; CI runs the full suite. Full local runs only for
cross-cutting changes." "If the only new commit since an approval is QA's test-only commit, the code reviewer re-checks
that commit alone."

*Applied:* a reviewer runs the acceptance commands, the test targets the diff adds or changes and those that depend on
what it changes (with and without the controls features), fmt, clippy and the `cargo xtask` tools, and checks that CI is
green on the head. A full local run is for a cross-cutting change (the workspace manifest, CI, the spawn helper, the
controls machinery, a support module many targets include). An acceptance command that is itself a whole-suite run
(R-230's proof) still runs. Process only.

## R-230 — A flaky-test fix is proved by 5 full runs under normal load and green CI *(amends R-224, R-227)*
*28 Sep 2026 · applied in TASK-M0-31 and TASK-M0-32*

"Proofs of flaky-test fixes: 5 full runs under normal load plus green CI. Apply this to #35 now."

*Applied:* R-224's and R-227's "20 times in a row under load" becomes 5 consecutive full-workspace runs under normal load,
with no failures, plus CI green on the head. REQ-VAL-162 and REQ-VAL-164 and the two task files carry it.

## R-231 — After TASK-M0-22, one task speeds up the suite: nextest, stable fixtures, injectable spawn timings
*Amended by R-270.*
*Still in force: one task, TASK-M0-33, after TASK-M0-22: cargo-nextest in CI and locally (doctests through `cargo test
--doc`, `cargo xtask controls` keeping its own cargo invocations), fixtures written only when changed, a build directory
never used by a test and its control at once (R-224, read per fixture type under R-270, as TASK-M0-33 applied it),
injectable spawn-helper timings with the calibrated values (REQ-VAL-156) unchanged, no test dropped, and edits to qa's
merged files limited to what these need; fixtures share one build directory per fixture type, not per copy, cached on
CI (R-270).*
*28 Sep 2026 · applied in TASK-M0-33*

"After TASK-M0-22, one task: cargo-nextest (CI and local), fixtures written only when changed with a target dir per
fixture, and injectable spawn-helper timings."

*Applied:* TASK-M0-33 (new, REQ-VAL-165), depending on TASK-M0-22. No test is dropped: doctests still run through
`cargo test --doc`, and `cargo xtask controls` keeps its own cargo invocations. Each fixture's target directory is never
shared between a test and its control (R-224). The spawn helper's calibrated values (REQ-VAL-156) are unchanged. Edits
to qa's merged files are limited to what these need, under a one-round exception, with qa reviewing.
*Applied note:* the human had also chosen "Lighter debug info" (`[profile.dev] debug = "line-tables-only"`) a few
minutes earlier; this later list leaves it out, so it is not in the task.

## R-232 — TASK-M0-30's veto items stand
*28 Sep 2026 · applied in TASK-M0-30*

"#33: both veto items stand. Merge it."

*Applied note:* the items are r194's constants moved into its support module as shared inputs (R-218), and inputs
written inside test functions left in qa's files. Numbered R-232 and R-233 because the human's rule in the same message
is R-234.

## R-233 — TASK-M0-31's veto items stand; the speed rulings' numbering stands; debug info stays at the default
*28 Sep 2026 · applied in TASK-M0-31*

"#35: all three veto items stand. Merge it." "The R-228 to R-231 numbering is fine." "Leave debug info at the default;
builds on the SSD make disk a non-issue." "The 300 s timeout is noted for the M0 gate."

*Applied note:* the items are the test's and its control's target directories kept across runs rather than fresh per
call; `--no-fail-fast` in the heavy-load loop only; `cargo xtask deps` not re-run for a test-only change. R-231's
applied note leaving out lighter debug info stands. REQ-VAL-156's 300 s is confirmed at the M0 gate, with the load
measurements from PRs #33 and #35.

## R-234 — A PR with veto items may be merged overnight when each item is accepted and none is a substantive choice
*28 Sep 2026 · applied in overnight runs*

"So veto items don't stall overnight runs: you may merge a PR with "applied per R-204 — veto?" items if every named
reviewer explicitly accepted each item AND every item is test infrastructure, process, sequencing or mechanical. Hold it
(keep going on other tasks) if any item touches physics or conventions, numeric values or calibrations, design or GUI
behaviour, scope or deferrals, or if the reviewers disagree. List every item merged this way in the summary, for me to
veto afterwards."

*Applied:* the overnight merge conditions otherwise stand (every named reviewer approves the head, CI green,
`check_plan.py` passes, within budget, no REVIEW_QUEUE entry needing the human). Process only.

## R-235 — `qa_cargo_xtask_alias_runs_deps` uses the listing-only form of `cargo xtask ci`; the controls get their own CI job *(closes RQ-150)*
*29 Sep 2026 · applied in TASK-M0-22*

"R-235 (RQ-150): option (c). qa_cargo_xtask_alias_runs_deps uses the listing-only form of cargo xtask ci; CI's own step
still runs every control. Also move cargo xtask controls into its own CI job running in parallel with the tests. Get
qa's explicit word on veto items 4 and 5, then merge #39 under R-234 if everything holds."

*Applied:* RQ-150 numbered its options 1–4; "(c)" is read as the third, the listing-only form, which the ruling's own
text describes. REQ-VAL-166 (new) carries it, closed by TASK-M0-22. qa's merged `qa_TASK-M0-01.rs` (or its support
module) changes only as far as this needs, under a one-round exception, with qa reviewing, as R-226's did.
*Applied per R-235 — veto?:* `cargo xtask ci`'s only runner is `controls`, and REQ-VAL-007 requires `controls` to run
inside `cargo xtask ci` on every push (R-198). So the `cargo xtask ci` step moves out of the `ci` job into a job of its
own, running beside the tests, rather than a second job running `cargo xtask controls` beside it; the controls run
once per push either way.
*Also in the same message:* the "veto?" items merged under R-234 on PRs #37 and #38 all stand.

## R-236 — A failing control's output is kept, and the `deps.rs:1205` control checks its compile error first
*Amended by R-244.*
*Still in force: the `deps.rs:1205` control first asserts the expected compile error, and a failing control's output is
kept; R-244 keeps it in `xtask/src/controls.rs`'s findings.*
*29 Sep 2026 · applied in TASK-M0-34*

"R-236: a small task, run in parallel: qa_TASK-M0-24.rs keeps the failing control's output in its message (reworded
from "leaves it passing"), and the deps.rs:1205 control first asserts the expected compile error."

*Applied:* TASK-M0-34 (new, REQ-VAL-167). *Applied per R-204 (sequencing):* it depends on TASK-M0-22, which also edits
`qa_TASK-M0-24.rs`, and runs in parallel with the other tasks ready after it. The edit to qa's merged file is limited
to the message, under a one-round exception, with qa reviewing.

## R-237 — qa's commit may add files under `xtask/tests/`
*Amended by R-290.*
*Still in force: qa's commit touches only `crates/*/tests/`, `xtask/tests/` and `fixtures/`; besides adding files there,
it may modify or delete a file only qa has committed to (R-290).*
*29 Sep 2026 · applied in CLAUDE.md and `.claude/agents/qa-reviewer.md`*

"R-237: qa's commit paths are crates/*/tests/ and xtask/tests/."

*Applied:* the orchestrator's check on qa's commit accepts `A` lines under `crates/*/tests/`, `xtask/tests/` or
`fixtures/`. `fixtures/` is kept: the ruling names the test directories, and nothing in it removes the fixtures path.

## R-238 — The old prin-impl scratch directory is deleted
*29 Sep 2026 · applied 29 Sep*

"R-238: delete the old scratch directory in /private/tmp."

*Applied:* `/private/tmp/claude-501/-Users-malachy-src-prin-impl/c1185d1d-…/` (4.6 GB) was deleted. Outside the repo.

## R-239 — Parallel work waits while swap is above 4 GB or memory pressure is high
*Superseded by R-252.*
*29 Sep 2026 · applied in CLAUDE.md*

"R-239: memory limit for parallel work. Don't start a new build or reviewer if swap in use is above 4 GB or memory
pressure is high; wait for running work to finish. Log swap alongside free disk in the summary."

*Applied:* before each dispatch the orchestrator reads `sysctl vm.swapusage` and `memory_pressure`, alongside free
disk. Process only.

## R-240 — TASK-M0-07 is split in two *(closes RQ-151)*
*29 Sep 2026 · applied in TASK-M0-07 and TASK-M0-35*

Asked in RQ-151, the human chose "Split in two": (a) the schema, the metadata gate, the generator driver, `cargo xtask
codegen` and REQ-GEN-024's definition; (b) the static layout check and its tests.

*Applied:* TASK-M0-07 keeps (a) and closes REQ-GEN-002 and REQ-GEN-024. TASK-M0-35 (new) takes (b), closes REQ-GEN-003
and REQ-GEN-028 (R-242), and depends on TASK-M0-07; its code is already written, and it opens once TASK-M0-07 merges.
*Applied per R-204 (sequencing):* TASK-M0-09 depends on TASK-M0-35 too, since its word buffer is the first real layout
the static check guards; TASK-M0-08 (the constants register) needs only TASK-M0-07. *Applied per R-204:* TASK-M0-07's
reviewers gain `physics`, which REQ-GEN-024 and its acceptance line require ("approved by the physics reviewer").

## R-241 — `xtask` may depend on `ledger`, for `cargo xtask codegen` *(amends systems_architecture §7.1)*
*29 Sep 2026 · applied in systems_architecture §7.1 and TASK-M0-07*

Asked whether `cargo xtask codegen` should run a ledger binary through `cargo run` (no new edge) or depend on `ledger`,
the human chose "Add xtask → ledger edge".

*Applied:* §7.1's allowed-edge table gains `xtask` → `ledger` (a normal dependency), realising `cargo xtask codegen`.
`cargo xtask deps`'s edge table gains it in TASK-M0-07, and the ledger `codegen` binary is dropped. REQ-SYS-004 cites
this ruling. `ledger` still depends on nothing.

## R-242 — The layout check's range and width rules *(closes RQ-151's rules)*
*Amended by R-247 and R-248.*
*Still in force: a `Range` end is closed, open or unbounded, the unsigned width rule, the unnamed-entry report,
`ledger::layout()` empty until transcribed, and a field type the check doesn't recognise failing; the signed rule waits
for the first signed type (R-247), and floats take exact widths (R-248).*
*29 Sep 2026 · applied in TASK-M0-07 and TASK-M0-35*

Asked whether to accept the rules the implementer applied where §3.8 and §5 are silent, the human chose "Accept all,
plus: the width check also covers signed integer fields if the ledger has any (min ≥ −2^(w−1), max ≤ 2^(w−1) − 1), and a
field type the check doesn't recognise fails rather than passing unchecked."

*Applied:* accepted — a `Range` end is closed, open or unbounded; the width check on an unsigned (u-bits) field requires
the least value ≥ 0 and the greatest ≤ 2^w − 1, and an unbounded end fits no width; an entry with no name is reported
as "entry #i (unnamed)"; `ledger::layout()` stays empty until the §3 entries are transcribed. Added — a signed integer
field of width w requires the least value ≥ −2^(w−1) and the greatest ≤ 2^(w−1) − 1; a field type the width check does
not recognise fails the check, naming the field and its type. REQ-GEN-028 (new) carries the width rules, closed by
TASK-M0-35; the range ends and the unnamed-entry report are TASK-M0-07's.

## R-243 — TASK-M0-07 is accepted at ~725 counted lines
*29 Sep 2026 · applied in PR #42*

Asked whether to accept PR #42 at 725 counted lines against R-240's ~560 (the excess: R-241's `xtask` → `ledger` edge,
~50, and the fixes code, qa and physics asked for, ~115), the human chose "Accept, merge". Merged as c1fd103.

## R-244 — TASK-M0-34's first deliverable moves to `xtask/src/controls.rs` *(amends R-236; closes RQ-155)*
*29 Sep 2026 · applied in TASK-M0-34*

Asked in RQ-155, the human chose "Move it to controls.rs": `xtask/src/controls.rs`'s `ControlPasses` / `WrongPanic`
findings carry the control's own output and drop "leaves it passing"; the one-round exception extends to the qa files
that assert the old wording. R-236's intent holds: the cause isn't lost next time.

*Applied:* REQ-VAL-167 is reworded to match. The exception covers `xtask/tests/qa_TASK-M0-22.rs`,
`crates/validation/tests/qa_TASK-M0-21.rs` and `xtask/tests/controls.rs`, limited to the assertions on that wording,
with qa reviewing.

## R-245 — An invalid diffusion fit reads NaN, by a validity predicate; no −1.0 sentinel *(amends R-17, R-136; closes RQ-152)*
*29 Sep 2026 · applied in the docs listed below*

Asked in RQ-152, the human chose "Validity predicate": drop the value sentinel; validity is read at derive time from
`n ≥ 2` (§3.5 already says invalid for `n < 2`), and an invalid fit reads NaN like R-79's tier-absent fields.

*Applied per R-245 — veto? (mechanical consequences):* `diffusion` is derived at read (§3.5 stores only `mean_y`,
`C_ty`), so storage still never holds NaN (R-79 stands). Every place that named the −1.0 sentinel now names the
predicate `n ≥ 2` and NaN: generation-root §3.4, §3.5, §3.8's worked entry and §5 item 5; simstate_payload §§ the
`n < 2` guard, the derived table and the WGSL helper's comment; the render contract's sentinel paragraph, generation
note and field table; debug_tooling_plan's field row; colour_composition's validity lane. R-136's rule stands for any
stored sentinel; `diffusion` is no longer one, and an invalid fit gets the hatch as NaN. R-17's streaming slope stands.
REQ-INT-041, REQ-PAY-030, REQ-PAY-039, REQ-GEN-012, REQ-RENDER-028 and REQ-TOOL-012 are reworded to match. The
non-normative GUI mockups under `docs/gui/reference/` are not edited.

## R-246 — The current drifts are derived at read; the render contract's "stored" means kept *(closes RQ-153)*
*29 Sep 2026 · applied in the render contract and generation-root §3.5, §3.8*

Asked in RQ-153, the human chose "§3.1: derived": the final drift is derived at read; the max is an accumulator. The
render contract's "stored" is reworded to "kept".

*Applied:* render contract Part 6's "Drift shape" row, generation-root §3.5's accumulator list and §3.8's two worked
entries are reworded to cite this ruling.

## R-247 — The signed-width rule waits for the first signed type *(amends R-242; closes RQ-156 in part)*
*29 Sep 2026 · applied in REQ-GEN-028 and TASK-M0-35*

Asked in RQ-156, the human chose "Defer to first signed type": §3.8 is left alone; the signed case is dropped from
REQ-GEN-028's test and TASK-M0-35's acceptance line. R-242's signed rule stays recorded and lands with the first signed
type.

## R-248 — Float types at a packed location: exact width; an f16 range lies within f16's finite range *(amends R-242; closes RQ-156)*
*29 Sep 2026 · applied in generation-root §3.8, REQ-GEN-028 and TASK-M0-35*

"Exact width, no range-vs-width test: f16-pair and fixed16 need exactly 16 bits, f32 exactly 32; a vector or anything
else at a packed location still fails. Plus: an f16 field's declared range must lie within f16's finite range
(±65504). An unbounded end fails unless the ledger entry states its overflow behaviour (saturate or ±inf)."

*Applied per R-248 — veto?:* §3.8 gains an optional key, `overflow: saturate | inf`, where the entry states it. "An
f16 field" is read as an `f16-pair` field wherever it sits, and each `f16-pair` component of a vector (range applies
per component, §3.8); `fixed16` is not binary16, so the f16 range rule doesn't apply to it. REQ-GEN-028 carries the
rules, closed by TASK-M0-35. TASK-M0-09 gains a note: `d_min` (`f16`, range "> 0", §3.4) has an unbounded upper end,
so its entry must state its overflow behaviour or bound its range.

## R-249 — TASK-M0-08 is accepted at ~756 counted lines in one PR
*29 Sep 2026 · applied in TASK-M0-08*

Asked whether to split TASK-M0-08 (built at 756 counted lines against ~300 estimated) into the register with its gate
(~420) and `cargo xtask lint constants` (~336), or to accept it whole, the human chose "Accept at 756".

*Applied per R-245 (mechanical):* the task file's register list drops "the diffusion sentinel −1.0", and the two
remaining places that still called an invalid diffusion fit a sentinel (generation-root §3.4's Welford note,
simstate_payload's `n < 2` guard) now say NaN, which R-245's list missed.

## R-250 — A threshold must sit between populations of its own distribution; no numeric percentile bound
*29 Sep 2026 · applied in TASK-M0-08*

Asked how to read REQ-VAL-006's "an extreme percentile (tau_display at the 0.4th percentile) fails", on which qa (a
numeric bound) and physics (a structural rule, from pitfalls §3's "the same defect as `tau_display` at the 0.4th
percentile" and philosophy §4.2's "the 0.4th percentile of its own distribution") disagreed on PR #47, the human chose
"Structural (physics)": the threshold must be finite and sit between populations of its own distribution, with at least
one population wholly below and one wholly above; its percentile is derived from the populations' counts. A threshold
inside any population fails, at whatever percentile. There is no numeric bound.

*Applied:* REQ-VAL-006's verify detail is reworded to match. qa's test in PR #47 that encodes the numeric reading
(`qa_constants_threshold_tau_display_at_the_0_4th_percentile_fails_whatever_its_populations`) is rewritten by the
implementer to the structural reading, under a one-round exception, with qa reviewing.

## R-251 — TASK-M0-08 accepted at ~918; its register items, with the hash covering value, type and class
*29 Sep 2026 · applied in TASK-M0-08, TASK-M0-12 and `plan/reviewers/physics.md`*

Asked whether to accept PR #47 at 918 counted lines (against R-249's ~756; the excess from the fix round the reviewers
asked for) and its four non-mechanical items, the human chose "Accept at 918" and answered:

"2, 5, 6 accepted. 2: confirm 76 is its representation's maximum, or reclass it. 4: accepted, but the hash covers each
constant's value, type and class, not its citation text. 6: also add to the physics reviewer's checklist that any
non-trivial numeric literal in kernel or engine code must be a register constant or justified in the review."

*Applied:*
- Item 2 (all four constants classed "bounded by its own achievable maximum"): accepted; PR #47 confirms, with the
  physics reviewer, that 76 (the reduced word's capacity in 121 bits, payload §3) is its representation's maximum, or
  reclasses it.
- Item 4 (the hash scope): accepted as scoped to the entries that decide stored bits, but the hash covers each such
  constant's value, type and class, not its citation text. PR #47's §3.8 text is corrected before merge. TASK-M0-12
  tests that a value, type or class edit to a hashed entry changes the schema version, that a citation-only edit does
  not, and that an entry that doesn't decide stored bits leaves it unchanged (from PR #47's physics review; applied per
  R-72 and REQ-SYS-063, closing the gap its note pointed to).
- Item 5 (a pending value keeps its class; pending exactly when citing a calibration requirement): accepted.
- Item 6 (the lint's scope): accepted; `plan/reviewers/physics.md` § 9 gains the item the human worded.
*Applied per R-204 — veto? (sequencing):* TASK-M0-12's reviewers gain `physics`, since what the schema version covers
is now a physics definition (the physics reviewer's suggestion).
*Accepted by the human (29 Sep, with R-252):* "#49: accept the veto item (physics as a reviewer on TASK-M0-12). Merge
#49, then #47."

## R-252 — The memory limit reads memory pressure, not swap *(amends R-239)*
*Amended by R-277.*
*Still in force: log the memory-pressure level, not swap, in summaries (kept by R-295); the dispatch limits are
R-277's.*
*29 Sep 2026 · applied in CLAUDE.md*

"R-252, amends R-239: the memory limit uses memory pressure, not swap size. macOS keeps swap allocated after memory
frees up, so swap over-reports. Check `memory_pressure` (or vm_stat's pressure level): start new builds or reviewers
only when pressure is normal; hold new work at warning; at critical, finish the running work only. Log the pressure
level instead of swap in summaries."

*Applied:* before each dispatch the orchestrator reads the kernel's pressure level (`sysctl
kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical) with `memory_pressure`'s free percentage,
alongside free disk. R-239's 4 GB swap limit no longer applies. Process only.

## R-253 — `ftle` reads NaN at `step_count = 0`, by the predicate `step_count ≥ 1`; no sentinel *(closes RQ-154)*
*Superseded by R-254.*
*29 Sep 2026 · applied in the docs listed below, REQ-INT-043 and REQ-TOOL-012*

Asked in RQ-154, the human answered: "RQ-154: accepted as recommended. ftle reads NaN, with validity predicate
step_count ≥ 1, no sentinel (same form as R-245). Record it as R-253."

*Applied:* before the first step `ftle = S_final/(step_count·dt)` is 0/0; it reads NaN there, never a stored or value
sentinel. `ftle` is derived at read, so storage still never holds NaN (R-79 stands). The predicate `step_count ≥ 1` is
the `n > 0` clause `ftle_valid` already has (payload §6; REQ-PAY-032), and `ftle_valid` stands unchanged: it also
requires `completed_renorms > 0` before the value counts as usable. The docs that describe `ftle`'s value now say so:
payload §5's derived table and FTLE note, generation-root §3.4's row and §3.8's entry, the render contract's field
table, and debug_tooling_plan's field row. REQ-INT-043 gains the read at `step_count = 0`; REQ-TOOL-012 names it among
the NaN a field view hatches.

## R-254 — `ftle` reads NaN whenever `ftle_valid` is false *(refines R-253)*
*29 Sep 2026 · applied in the docs listed below, REQ-INT-043, REQ-PAY-032 and REQ-TOOL-012*

"R-254, refining R-253 (fold into #53 if it hasn't merged): ftle reads NaN whenever ftle_valid is false, i.e. before
the first completed renorm as well as at step_count = 0. One rule, so no consumer ever sees a meaningless 0.0 alongside
a false validity flag."

*Applied:* `ftle` reads NaN exactly when `ftle_valid` (payload §6) is false: the tier off (as R-79 already had), a
failed sample, `step_count = 0`, or no completed renorm. The docs R-253 touched now state the one rule, and the
lowering contract's `sample.ftle` row says the accessor returns it.
*Applied per R-204 — veto? (reading of "whenever"):* `ftle_valid` is also false for a failed sample, and the lowering
contract said per-sample failure surfaces as the defined failed-state values, never NaN (R-79). R-254 is applied as
worded, so a failed sample's `ftle` reads NaN too; the lowering contract names `ftle` as the one exception. R-79's
storage rule stands: `ftle` is derived at read, never stored, so storage still never holds NaN, and a failed sample's
stored fields keep their defined failed-state values.
*Accepted by the human (29 Sep, with R-255):* "The failed-sample item on #53 stands: a failed sample's ftle reads NaN
(R-254 as applied)."

## R-255 — Every aggregate over `ftle` excludes samples by `ftle_valid`, never by NaN propagation *(condition on R-254)*
*29 Sep 2026 · applied in the lowering and render contracts, REQ-PAY-032, REQ-RENDER-015 and `plan/reviewers/physics.md`*

"The failed-sample item on #53 stands: a failed sample's ftle reads NaN (R-254 as applied). Condition, recorded with it
as R-255: every aggregate over ftle (footprint means and spreads, quad reductions, histograms, statistics) excludes
samples by ftle_valid explicitly and never relies on NaN propagation, since one NaN poisons a sum or mean, and WGSL's
min/max/clamp with NaN operands are implementation-defined (a parity hazard). Add this to the physics reviewer's
checklist."

*Applied:* the lowering contract's Part 3a and the render contract's validity bullet state the rule. REQ-PAY-032 carries
it for every consumer and REQ-RENDER-015's check covers the fragment side; `plan/reviewers/physics.md` § 7 gains the
item, so every task that aggregates `ftle` is checked against it.

## R-256 — TASK-M0-09 is accepted at ~1,000 counted lines in one PR; TASK-M0-10 keeps only pack/unpack
*Amended by R-264.*
*Still in force: veto items (a) and (b), and TASK-M0-10 keeps only pack/unpack; TASK-M0-09's size is R-264's (1,084
counted lines, one PR).*
*29 Sep 2026 · applied in TASK-M0-09 and TASK-M0-10*

"1. TASK-M0-09 accepted at ~1,000 lines, one PR. Veto items (a) and (b) stand. Update TASK-M0-10's task file so it
keeps only pack/unpack."

*Applied:* TASK-M0-09 (built at ~1,000 counted lines: ~473 implementation, ~529 the implementer's own tests) is one PR.
Its veto items stand: (a) it transcribes the interiors of `packed_a`, `packed_b` and `times` (the descriptor fields
included), so the static check sees every declared bit; (b) §3.4's `delta_E_max_abs` and `delta_Lz_max_abs` are the
payload's `dE_max` and `dLz_max` (R-70, R-86). TASK-M0-10 now builds the pack/unpack/insert emitters, the accessors
and `roundtrip_ctl` over the entries TASK-M0-09 transcribed.

## R-257 — The briefs' unheld obligations: two ported, one superseded, one not standing, the kernel gates ported with values *(closes RQ-157)*
*29 Sep 2026 · applied in TASK-M0-02 (PR #54)*

"2. RQ-157: all five as recommended (1 superseded, 2 and 3 ported with calibration and definition requirements, 4 not
standing, 5 ported with values into dd_validation_orbits §2). #54's veto item 1 stands: current docs win where they
contradict a brief."

*Applied:* (1) the 2:1 balance constraint and `neighbour(i, dir)` are superseded: scheduler Part 3's split predicate
governs, and one sample per tile names no crack. (2) the persistent frontier and its permanent from-scratch
cross-check are ported, with N, the frames between cross-checks, a calibration requirement (R-71). (3) "zoom-out
recomputes ≈ 0 quads" is ported as a benchmark with its tolerance a calibration requirement, and ranking on a quad's
visible part is ported with `P_visible` a definition requirement (R-72). (4) the reporting items are experiment
reporting, not standing obligations. (5) kernel-build §5's gates are ported, with their values, into
`dd_validation_orbits` § 2. Where a consolidated doc contradicts a brief, the doc wins and nothing is ported (PR #54's
veto item 1). The ports land in PR #54 (TASK-M0-02), docs first, then the requirements.

## R-258 — The convergence gate's region minimum is a calibration, its fixtures' counts are "not recorded", and scatter is defined *(closes RQ-159)*
*Amended by R-273.*
*Still in force: all three items, but the region minimum is calibrated at M3 (R-273), not at the M0 gate.*
*29 Sep 2026 · applied in TASK-M0-05*

"3. RQ-159: all three as recommended."

*Applied:* (1) the number of regions the convergence gate declares is a calibration requirement (R-71), closed by
TASK-M0-05 and confirmed at the M0 gate; until then the report prints the count it saw and marks the minimum "not yet
calibrated". (2) the two fixtures record their region count as "not recorded", and a conclusion drawn from an
unrecorded count is flagged like one from too few regions. (3) scatter, for the convergence gate, is a definition
requirement (R-72): the r_k sequence per region, and the min–max spread of the final r across regions.

## R-259 — The vocabulary lint matches four retired ideas as phrases; `spawn::TIMEOUT` becomes `SPAWN_TIMEOUT`; it scans `.md` and `.html` *(closes RQ-160)*
*29 Sep 2026 · applied in TASK-M0-16 and REQ-SYS-002*

"4. RQ-160: all three as recommended, including SPAWN_TIMEOUT with a one-round exception for qa's file."

*Applied:* (1) the four retired ideas are matched as case-insensitive phrases in code and docs: "checkpoint count",
"ensemble shadow(s)", `fround` (with `Math.fround`), "TypeScript layout constant" / "TS layout constant"; the
identifier terms stay whole, case-sensitive identifiers (R-140). (2) `validation::spawn::TIMEOUT` is renamed
`SPAWN_TIMEOUT` in TASK-M0-16, and qa's `crates/validation/tests/qa_TASK-M0-26.rs` follows under a one-round
exception, qa reviewing. (3) the lint scans `.md` and `.html` under `docs/`, excluding `docs/archive/` and
`docs/reference/`; `docs/experiments/` holds no `.md` contract.

## R-260 — qa's approval carries over its own test commit *(amends R-175)*
*29 Sep 2026 · applied in REQ-SYS-066 and TASK-M0-37*

"5. reviews-complete: an approval still counts if the only later commit is qa's own "qa: tests for <task>" commit,
adding files only under qa's paths."

*Applied:* a role's APPROVE on an earlier commit counts on the head when every commit after it is a commit titled
`qa: tests for <TASK-id>` that adds files only under qa's paths (`crates/*/tests/`, `xtask/tests/`, `fixtures/`,
R-237). `reviews-check` implements it in TASK-M0-37.

## R-261 — A PR whose title names no task passes `reviews-complete` *(amends R-175)*
*29 Sep 2026 · applied in REQ-SYS-066 and TASK-M0-37*

"6. reviews-complete: a PR naming no task passes, with "no task, no named reviewers"."

*Applied:* a PR whose title names no task id passes `reviews-complete` and prints "no task, no named reviewers".
`reviews-check` implements it in TASK-M0-37.

## R-262 — Builds move to the internal disk, three agents at most
*29 Sep 2026 · process*

"7. Move builds to the internal disk, three agents at most. Keep the disk rules (≥25 GB target, never start work below
15 GB)."

*Applied:* new worktrees and target directories go on the internal disk; at most three agents build at once; free disk
stays ≥ 25 GB where possible and no work starts below 15 GB. Work already on the external SSD finishes there. Process
only.

## R-263 — §3.8 gains the optional key `floor?: <sim-key parameter>` *(closes RQ-158)*
*29 Sep 2026 · applied in generation-root §3.8 and TASK-M0-09*

"8. RQ-158: add the optional §3.8 key floor?: <sim-key parameter>."

*Applied:* §3.8's schema gains `floor?: <sim-key parameter>`; `energy_drift` carries `floor: eps_E` and `Lz_drift`
`floor: eps_L` (§3.4), set in TASK-M0-09.

*Also in the same message:* "The REQ-PAY-002 split and the M0-03 → M0-36 split both stand. I'll add pr-check (and ci
and reviews-complete, if they aren't yet) to the required checks once #56 merges." Recorded under R-256's PR as
applied: REQ-PAY-002 keeps the SimState and ICDescriptor part (TASK-M0-09); RenderQuad's field set moves to TASK-M1-06
and the WGSL layout comparison to TASK-M0-13, each as a split-off requirement.

## R-264 — The size budget is a rough heuristic that weighs complexity; M0-09, M3-08 and M5-18 stay whole *(amends R-256, R-211)*
*30 Sep 2026 · applied to PR #59, TASK-M3-08 and TASK-M5-18*

"yes i accept it being larger"

"2/ again doesn't matter that they are over budget, 500 line budget is too strict"

*Applied:* PR #59 (TASK-M0-09) stays one PR at 1,084 counted lines (R-211, R-223, R-225). R-256 accepted ~1,000; the
extra ~80 are R-263's `floor` key, its gate and its tests. TASK-M3-08 (~520, with REQ-VAL-170/171) and TASK-M5-18
(~510, with REQ-VAL-172/173), both raised by PR #54's RQ-157 ports, stay one task each: no split, and REQ-VAL-170/171
do not move to TASK-M3-07.

"budget is a rough heruistic, it should consider the task complexity ALSO"

*Applied:* no new figure. The ~500-line budget stays as a rough heuristic, and whether a task is one reviewable PR
also weighs its complexity, not its counted lines alone. `plan/WORKFLOW.md` § "Task files" gains a line saying so.

"also you don't have to keep nagging me if it goes over budget, you delagate and split or decide to accept and keep as one
pr. as long as the work gets done & that nothing is skipped, deffered or drifts. the questions for me should be actually
ambiguious non specified requirements, features, or problems. not the size of a pr"

*Applied:* size is no longer a question for the human (amends R-208, R-211 and R-204's "exceeding budgets"). The
orchestrator decides whether an oversized task splits in the plan or stays one PR, and records the choice in the PR
description. The limit on that choice: nothing is skipped, deferred or drifts. A split moves every requirement to a
named task, and deferral still needs a human ruling. The self-merge condition "the PR is within budget" is met by this
recorded choice.

## R-265 — The shared kernel is f32 and f64 only; double-double is parked *(closes RQ-161, amends R-33)*
*30 Sep 2026 · applied in canonical_spec §1, §7, §11; core_design; systems_architecture §1; REQ-INT-004; REQ-SYS-007; TASK-M3-01*

"RQ-161: option 2, park it (R-265). The shared kernel is instantiated for f32 and f64 only. The precision reference is
R-33's CPU arbitrary-precision integrator (TASK-M3-25), which also serves as the screen. TASK-M3-01 and REQ-INT-004 drop
the Dd instantiation; core_design and canonical_spec §1 are conformed; REQ-SYS-007 names only the R-33 reference;
philosophy §7.1/§7.7 stay as they are. Record Dd under parked ideas: it returns only if a precision question needs it
and Brutus is too slow."

*Applied:* canonical_spec §1 items 2 and 3 and its vehicle rationale; core_design's inspector line and substrate
paragraph. REQ-INT-004 and TASK-M3-01 lose the `Dd` instantiation (f32 and f64 only). REQ-SYS-007 names R-33's
reference (TASK-M3-25) as the one extended-precision build before 1.0, beside §7.1's payload genericity.

*Applied per R-265 — veto?:* R-33's "double-double is a fast screen only" is amended where it is quoted
(canonical_spec §7 :91 and §11 :153, systems_architecture §1 :45) by an added note that the reference itself serves as
the screen. systems_architecture :45's inspector witness "(f64 or double-double)" reads "(f64; double-double parked,
R-265)". Measured records stay as they are: dd_integrator :124's verified bit-identity across CPU-double-double,
experiments/results/findings.md and the spike brief.

*Parked:* **the shared kernel's double-double (`Dd`) instantiation.** What it buys: f64-plus precision on the same
source as the survey. Why it waits: R-33's independent reference covers extended precision. It returns only if a
precision question needs it and the Brutus-style reference is too slow. Recorded here, since philosophy §7.1 and §7.7
stay as they are (§7.1 already parks extended precision; this is its kernel-instantiation row).

## R-266 — "Require branches to be up to date" stays off; bypassing is not allowed *(amends HUMAN_SETUP §2)*
*30 Sep 2026 · applied in plan/HUMAN_SETUP.md §2*

"R-266: "Require branches to be up to date" stays off: CI runs on each PR merged with main, and every push to main runs
CI again. Amend HUMAN_SETUP.md §2 to match. I'm turning on "Do not allow bypassing", so the required checks bind every
merge, including yours."

*Applied:* HUMAN_SETUP §2 now says the up-to-date requirement stays off, with the reason, and that bypassing is not
allowed. The required checks on `main` are `ci`, `pr-check`, `reviews-complete`, `xtask-ci`, `gpu-metal` and
`gpu-lavapipe`. From this ruling on, no merge, the orchestrator's included, lands with a required check red.

## R-267 — Every merged "veto?" item stands; the #38 flake item is closed; three follow-ups become one task
*30 Sep 2026 · applied in TASK-M0-38 (new), REQ-SYS-069 to REQ-SYS-071*

"All merged "veto?" items stand, including #37, #38, #50, #56, #57 and #62, and your own applications on #61 and #63.
The two #38 flake fixes are already done by TASK-M0-34 (#50); close that item. The three queued follow-ups
(parse_wrong_panics with an embedded header, the "Text file busy" failure, pr-check passing a nameless meter line)
become one small low-priority task. Check whether "Text file busy" has the same cause as the earlier
executable-replacement race."

*Applied:* the items stand as merged: PR #37's four, #38's two, #50's two, #56's five, #57's two, #62's five, #61's
three R-265 applications and #63's two RQ-162 applications. The #38 `deps.rs` control-flake item is closed by
TASK-M0-34 (R-236, REQ-VAL-167). TASK-M0-38 is new: code and qa, low priority, depending only on merged tasks.

*The "Text file busy" check (orchestrator's diagnosis, recorded for TASK-M0-38):* not the same cause. CI run
36634340042 attempt 1 (PR #56, ubuntu): `qa_m022_list_runs_no_control` failed with "xtask: cannot run cargo metadata:
Text file busy (os error 26)". Each `qa_TASK-M0-22.rs` test writes its own stand-in `cargo` script and runs xtask
against it, four at once. That is the Linux fork/exec race: while one thread has its script open for writing, another
thread forks to spawn a child, which inherits the write descriptor until it execs, and an exec of the first script in
that window fails with ETXTBSY. The #38 race was different: tests shared one binary (`debug/xtask`) that one of them
rewrote while others ran it, fixed by the `WORKSPACE_RUN` Mutex. Nothing is shared here, so that Mutex cannot cover it;
it shows on Linux only.

## R-268 — The overnight "veto?" items stand; #70's item 2 and #71's items 1, 6 and 12 are accepted
*30 Sep 2026 · applied in PRs #70 and #71*

"All merged "veto?" items and the overnight R-204 applications stand. #70 item 2 accepted. #71 items 1 and 6 accepted;
item 12 accepted once code's re-check agrees."

*Applied:* the items stand as merged on #54, #59, #68, #72 and #75, and so do the overnight R-204 applications: RQ-162
(#63) and RQ-168 (the stand-in soak installs Mesa, #71's item 13). PR #70's item 2 (`SetField` edits only `SimConfig`
and `RenderState`, and `Snapshot` leaves `ViewUI` out) is accepted. So are PR #71's item 1 (golden opens its own wgpu
device, with its backend rule kept in step with the harness's by a test) and item 6 (repro reports every line a case
declares, and refuses a case that declares none). Item 12 (what `golden --list` checks) is accepted once the code
reviewer's re-check at 76fd151 agrees.

## R-269 — REQ-VAL-138 across backends: measure lavapipe, then zero steps or one reference per backend
*Amended by R-287.*
*Still in force: its measurement and result, and one reference per backend as the fallback for a case whose bytes still
differ; otherwise goldens share one reference (R-287).*
*30 Sep 2026 · applied in REQ-VAL-138; the measurement runs on a `measure/` branch (R-272)*

"REQ-VAL-138: throwaway measure/ branches are allowed (push, measure, delete). Get the lavapipe max-step. If 0: zero
steps across backends for M1 goldens. If not: one reference per backend."

*Applied:* the values between 8-bit levels that PR #71's Metal check rendered (Metal max step 0 between two renders)
are rendered on lavapipe on a `measure/` branch and compared with Metal's. If the max step is 0, REQ-VAL-138's 0 steps
holds across backends for the M1 goldens, against one reference. If it isn't, each golden case keeps one reference per
backend, and the runner picks the one for the backend it renders on. The result and the branch's deletion are recorded
under this ruling.

*Result (30 Sep 2026, CI run 36719287172, branch `measure/r269-lavapipe-step`, deleted afterwards, no PR):* PR #71's
fragment (R = (x+0.5)/255, exactly half-way between levels; G = exp(−y/40); B = ((x+y)/510)^2.2; 256×256, Rgba8Unorm)
rendered through the golden runner. Lavapipe against itself over two renders: max step 0. Metal against itself: 0, and
the hosted Metal render matches a local M3 Pro byte for byte. Lavapipe against Metal: max step 1, in R only, on 32768
of 65536 pixels (every even x); G and B are identical. Metal rounds each exact tie up, and lavapipe rounds it to even.
The max step isn't 0, so under this ruling each golden case keeps one reference per backend, and the runner picks the
one for the backend it renders on (REQ-VAL-176, TASK-M0-43).

## R-270 — TASK-M0-33: qa's one-round exception is granted; the fixture-pool cost is sent back *(amends R-231)*
*Amended by R-301, R-325 and R-336.*
*Still in force: qa's one-round exception; fixture copies share one build directory per fixture type; CI caches the
pool between runs; the local pool ~5 GB. The ~10.5 min target applies to each CI job's warm wall-clock time (R-325);
R-301's measurement, about 10m28.5s per warm `ci` run as PR #85 measured it, stands as a record. R-336 accepts PR
#96's overrun of it, and TASK-M0-45 brings each job back under it.*
*30 Sep 2026 · applied in TASK-M0-33 (PR #74)*

"#74: one-round exception granted for qa_TASK-M0-33.rs. Cost sent back: share build dirs per fixture type, not per
copy, and cache the pool on CI. Targets: CI job no slower than before (~10.5 min), local pool ~5 GB."

*Applied:* qa may modify `crates/validation/tests/qa_TASK-M0-33.rs` in one commit, which fixes the test that misses
cargo's coloured output. The orchestrator's R-237 check accepts `M` on that one file, once. Fixture copies share one
build directory per fixture type, not per copy, and CI caches the pool between runs. PR #74 shows two measurements:
the `ci` job's wall time is no slower than before (~10.5 min), and the local pool is ~5 GB. REQ-VAL-165's "a warm second
run … rebuilds nothing" is read per fixture type.

## R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*
*30 Sep 2026 · applied in docs/design/principia_dd_simstate_payload.md §1, TASK-M0-10*

"RQ-163: d_min's unset value is +inf (the minimum of an empty set); stored values clamp to f16's smallest positive
subnormal, so 0.0 never appears; readers treat +inf as unset."

*Applied:* a failed sample's `d_min` half, and any sample's before its first step, holds f16 +inf (bits `0x7C00`). A
valid `d_min` below f16's smallest positive subnormal (2⁻²⁴ ≈ 5.96e-8) is stored as that subnormal. `dE_max` and
`dLz_max` keep their 0.0 failed-state value (payload §1), which is the maximum of an empty set of magnitudes.
*Applied per R-204 — veto? (mechanical consequences, found in payload §1):* WGSL makes `pack2x16float` indeterminate
outside binary16's finite range and lets it flush subnormals to zero. So the `d_min` packer writes the +inf and
subnormal bit patterns itself, not through `pack2x16float`. Readers test the unset value by its bits (`0x7C00` in bits
16–31 of `packed_a`), not by a float comparison, which WGSL's finite-math rules would leave indeterminate. A GPU reader
that flushes a stored subnormal on unpack sees 0 for display only; the unset test never confuses the two.

## R-272 — Throwaway `measure/` branches are allowed; the ubuntu mutants timing runs on one *(closes RQ-164)*
*30 Sep 2026 · applied in TASK-M0-23 (PR #65)*

"RQ-164: covered by the measure/ branch rule; run the ubuntu mutants timing."

*Applied:* a `measure/<what>` branch may be pushed so that CI takes a measurement, and is deleted right after. No PR is
opened from it, and the number is recorded in the PR or here. TASK-M0-23 takes REQ-VAL-149's ubuntu timings this way.

## R-273 — REQ-VAL-168's region minimum is calibrated at M3 *(closes RQ-165, amends R-258)*
*30 Sep 2026 · applied in REQ-VAL-168, TASK-M0-05, TASK-M3-34*

"RQ-165: the region minimum is calibrated at M3; the runner prints "not yet calibrated" until then."

*Applied:* REQ-VAL-168 moves to M3. TASK-M3-34, which proposes the convergence gate's threshold (REQ-VAL-135), also
proposes the region minimum with its evidence, and the human confirms it at the M3 gate. Until then the gate prints the
count it saw and "minimum not yet calibrated", which TASK-M0-05 (merged) already does.

## R-274 — The screenshot runner reaches `gui` through a headless capture mode it spawns *(closes RQ-166)*
*30 Sep 2026 · applied in systems_architecture §7.1, a new GUI requirement, TASK-M6-22*

"RQ-166: gui gets a headless capture mode, spawned by the runner (no crate edge)."

*Applied:* `gui` ships a headless capture mode. The screenshot runner spawns it as a separate process, which renders a
named window offscreen and writes the PNG and the AccessKit names. The runner compares or checks presence, as `cargo
xtask gate` spawns validation's binary. No crate depends on `gui` (§7.1 unchanged). A case's `surface` field names the
kind (`data` today, `gui` for these). TASK-M6-22, the first task with a GUI screenshot requirement, builds the mode.

## R-275 — A control clipped out of the visible surface isn't present *(closes RQ-167)*
*30 Sep 2026 · applied in REQ-TOOL-134, TASK-M0-40*

"RQ-167: a control clipped out of the visible surface isn't present."

*Applied:* a presence check counts a control only if it is in egui's tree and its rect intersects the visible surface.
A case that needs a control below the fold scrolls to it first. TASK-M0-40 changes the runner (PR #72 merged the
tree-only check).

## R-276 — Four follow-ups: the r217 flake, M0-06's wording, conversation resolution, reviews re-run on each review
*30 Sep 2026 · applied in TASK-M0-39 and TASK-M0-40 (new), TASK-M0-06, plan/HUMAN_SETUP.md §2*

"Yes: the qa_r217 flake task (proper process group, not perl setpgrp), the M0-06 wording fix, conversation resolution
in HUMAN_SETUP §2, and a pull_request_review trigger so reviews-complete re-runs on each review."

*Applied:*
- **TASK-M0-39 (new).** It fixes `qa_TASK-M0-26_r217.rs`'s two controls, which flake under CI load: the out-of-group
  grandchild leaves the process group only when perl runs `setpgrp`, and the 1 s timeout can fire first. The
  grandchild is put in its own process group at spawn, not by perl.
- **TASK-M0-06.** "through the harness" becomes "on its own headless wgpu device": §7.1 lets xtask reach `validation`
  only as a dev-dependency (R-268 accepts #71's item 1).
- **HUMAN_SETUP §2.** It records "Require conversation resolution before merging", which is on.
- **Reviews re-run.** *Flagged:* `reviews.yml` already triggers on `pull_request_review`. What blocks a merge is the
  earlier `pull_request`-event run: it fails before any review and stays a separate failed check suite (R-266's
  no-bypass). *Applied per R-204 — veto?:* TASK-M0-40 has the `pull_request_review` run re-run that stale
  `pull_request` run for the same head, so one review turns both green.

## R-277 — Agents: two at memory-pressure warning, three at normal *(amends R-252)*
*30 Sep 2026 · applied in the orchestrator's loop*

"Two agents while memory pressure sits at warning; three at normal."

*Applied:* at `kern.memorystatus_vm_pressure_level` 2 (warning), at most two agents run; at 1 (normal), three. At 4
(critical), only running work finishes (R-252).

## R-278 — The f16 subnormal floor is an achievable maximum, beside `f16_finite_max` *(closes RQ-169)*
*30 Sep 2026 · applied in TASK-M0-10 (PR #78): dd_generation_root §3.8, the constants register, REQ-PAY-092*

The human's rulings message of 30 Sep said "Rulings (R-272 onward)"; R-272 to R-277 were already taken, so its rulings
are recorded here as R-278 to R-285, in the message's order.

"RQ-169: option 1. The f16 subnormal floor joins the register as an achievable maximum ("the format's achievable
extreme"), beside f16_finite_max; update §3.8's class wording, hash sentence and schema version accordingly."

*Applied:* in TASK-M0-10. §3.8's `achievable-maximum` class also covers a number format's achievable extreme. An
`f16_min_subnormal` row (2⁻²⁴) sits beside `f16_finite_max`. The hash sentence names the floor and counts five
entries, and the schema version changes with the hashed ledger. The emitter reads the floor from the register.

## R-279 — A fixture type is a fixture source set; `xtask` is one *(TASK-M0-33, veto item 11)*
*30 Sep 2026 · applied in TASK-M0-33 (PR #74, merged)*

"#74 item 11: accepted. Merge #74."

*Applied:* #74 merged at 08999ec. The fixture pool keeps one build directory per fixture source set, and the `xtask`
build is one of them (R-270).

## R-280 — An unset `d_min` renders in the neutral "not yet" grey *(closes RQ-170)*
*30 Sep 2026 · applied in payload §1, debug_tooling_plan §B, REQ-TOOL-012, REQ-TOOL-137 and TASK-M1-09*

"RQ-170: option 3: an unset d_min renders in the neutral "not yet" style, the same grey as running samples (R-96).
Not the invalid hatch, not the top of the ramp."

*Applied:* a field view that reads `d_min`'s unset bits (`0x7C00`, R-271) draws the neutral grey that `running`
samples show (R-96). It doesn't draw the invalid hatch, which stays NaN's (PIT-8), and it doesn't place +inf on the
ramp. The other sentinels still show as their literal values on the ramp (R-136).

## R-281 — TASK-M0-10's veto items: 1 and 7 accepted; item 2 vetoed in part
*30 Sep 2026 · applied in TASK-M0-10 (PR #78) and REQ-PAY-092*

"#78: items 1 and 7 accepted. Item 2 vetoed in part: the packer never stores NaN (R-79) and never silently rewrites a
negative value. Both are debug_assert! failures; in release, a NaN stores the unset bits and a negative value clamps
to the floor, and each case increments a telemetry counter."

*Applied:* in TASK-M0-10. The `d_min` packer's `debug_assert!` fails on a NaN or a negative input. In release, a NaN
stores `0x7C00` and a negative value stores the floor, `0x0001`, and each case increments a telemetry counter. The
corpus names no such counter, and a GPU-side count has to cross the membrane, which the corpus doesn't settle, so the
counter's definition waited on RQ-171 (filed on PR #78), and R-288 settles it.

## R-282 — TASK-M0-17's design items accepted
*30 Sep 2026 · applied in TASK-M0-17 (PR #79)*

"#79: items 3, 4, 5, 6, 10, 11, F1 and F2 accepted."

*Applied:* profiler schema v1 keeps them as PR #79 wrote them. Item 3: a batch render's `present` is null, in both
places. Item 4: the header shapes. Item 5: the `frame` index key. Item 6: the nested section shapes. Item 10: only
`camera_delta > 0` is normative. Item 11: the three memory pools are disjoint. F1: `leak_flags` and `hot_paths`,
null until M8. F2: `live_memory`, a per-frame snapshot of each pool by type.

## R-283 — The process choices stand; the add-only rule is raised, not exempted again; #80 merges
*30 Sep 2026 · applied in the orchestrator's loop*

"Process choices: all stand (physics on #79, qa's edits on #74/#78, the renumbering). If qa keeps needing to edit its
own files, raise the add-only rule itself in REVIEW_QUEUE rather than exempting it again." "#80: merge it."

*Applied:* physics stays a reviewer of TASK-M0-17. qa's `M` lines on #74 and #78 stand, and the queue renumbering
(RQ-169 → RQ-170) stands. The next time qa needs to change a file of its own, the orchestrator doesn't grant another
exception: it files R-237's add-only rule in REVIEW_QUEUE. PR #80 merged as 7e5526a.

## R-284 — `cargo xtask codegen` writes a generated file only when its content changes
*30 Sep 2026 · applied in REQ-TOOL-138 and TASK-M0-41 (new)*

"codegen rewriting unchanged files: small task, medium priority. Write generated.rs only when its content changes."

*Applied:* a new task, TASK-M0-41. `cargo xtask codegen` compares each generated file with what's on disk and writes
it only when the content differs, so an unchanged file keeps its mtime and forces no rebuild. Medium priority.

## R-285 — CI caches only the cargo registry and the fixture pool, with per-job keys
*Amended by R-320 and R-326.*
*Still in force: no whole target directories cached; per-job keys; the cargo registry and the fixture pool cached;
R-320 adds the rust-gpu build (`~/.cache/rust-gpu`), keyed on the pinned toolchain version; R-326 saves caches only on
pushes to `main`, and pull-request runs restore only.*
*30 Sep 2026 · applied in REQ-SYS-073 and TASK-M0-42 (new)*

"CI cache: cache only the cargo registry and the fixture pool, not whole target dirs, with per-job keys, to stay well
under GitHub's 10 GB limit."

*Applied:* a new task, TASK-M0-42. CI workflows stop caching whole target dirs (`Swatinem/rust-cache` caches `target`
by default). They cache the cargo registry and the fixture pool (R-270), each under a key naming its job, so the
repo's Actions cache stays well under GitHub's 10 GB limit.

## R-286 — Profiler traces are JSON Lines: the header, then one compact frame record per line
*Amended by R-298 and R-341.*
*Still in force: the header on the first line, then one compact frame record per line, never pretty-printed;
pretty-printing on demand (`prin profile show --pretty`, or `jq`); R-298 adds the summary line after the frames, and
makes a trace that lacks it valid; R-341 makes `prin profile` stream the file in that order, flushed at least every 60
frames or 1 s.*
*30 Sep 2026 · applied in telemetry §5 and TASK-M0-17 (PR #79), REQ-TOOL-120, REQ-TOOL-139 (new) and TASK-M0-18*

"R-286: profiler traces are JSON Lines: the header on the first line, then one compact frame record per line.
Pretty-printing is on demand (prin profile show --pretty, or jq). This meets §5's "readable" and "bounded size"
together. Apply it in #79's fix round."

*Applied:* a trace file's first line is the session header and each later line is one compact frame record. The
writer never pretty-prints. TASK-M0-17 writes the format in telemetry §5 and applies it in PR #79's fix round.
`prin profile show --pretty` joins TASK-M0-18, which builds `prin profile` (REQ-TOOL-139).

## R-287 — Fragment output quantises in the shader, so goldens share one reference across backends *(amends R-269)*
*Amended by R-296.*
*Still in force: fragment output quantises in the shader, rounding half to even, and a golden case whose bytes agree
across backends keeps one reference; explicit quantisation makes exact ties identical, not values within an ulp of a
tie, so a golden near a tie keeps one reference per backend (R-296).*
*30 Sep 2026 · applied in parity contract §4, REQ-VAL-176 and TASK-M0-43*

"R-287: fragment output quantises explicitly in the shader (round half to even, then store), not through the
backend's automatic float-to-unorm conversion, so every backend writes identical bytes. Once that lands, goldens
return to one reference across backends (amends R-269; per-backend references stay the fallback if any case still
differs). Fold it into TASK-M0-43."

*Applied:* TASK-M0-43 now quantises each fragment output channel in the shader, rounding half to even, before the
store, so no backend's float-to-unorm conversion decides a tie (R-269's measured difference). Each golden case then
keeps one reference for every backend. A case whose bytes still differ between backends keeps one reference per
backend, as R-269 ruled, and the PR names it.

## R-288 — R-281's counters: two per-frame atomic u32 counters in telemetry §2, on the existing readback *(closes RQ-171)*
*Amended by R-294.*
*Still in force: two u32 per-frame counters in telemetry §2, `dmin_nan_unset` (a NaN `d_min` stored as unset) and
`dmin_negative_floored` (a negative `d_min` clamped), incremented by the `d_min` packer in release builds too, not by
`roundtrip_ctl`'s repack; read back asynchronously on the profiler/telemetry readback, a frame or two late and never
stalling a frame, on no new GPU→CPU channel (`QuadReduction` unchanged, R-142); and profiler schema v1's frame record
carries both keys; they belong to the frame, never a static: a per-frame struct the caller passes in on the CPU, a
buffer bound and reset per frame on the GPU (R-294).*
*30 Sep 2026 · applied in telemetry §2 and TASK-M0-10 (PR #78), and profiler schema v1's frame record in TASK-M0-17 (PR #79)*

"RQ-171: option (a), recorded as R-288. Two per-frame atomic u32 counters (NaN d_min stored as unset; negative d_min
clamped) in telemetry §2. They ride on the existing profiler/telemetry readback, not a new GPU→CPU channel
(QuadReduction stays the sole automatic return of simulation data, R-142), and are read back asynchronously with a
frame or two of latency, never stalling the frame. They're counted in release builds too; that's their purpose."

*Applied:* telemetry §2 gains `dmin_nan_unset` and `dmin_negative_floored`, u32 counts per frame, the names RQ-171's
option (a) gave. The `d_min` packer increments them in release builds as well as debug ones. They come back on the
profiler/telemetry readback, asynchronously and a frame or two late, and never stall a frame; `QuadReduction` is
unchanged (R-142). `roundtrip_ctl`'s repack is an observation, not a store, so it doesn't count (RQ-171 option (a)).
Profiler schema v1's frame record carries both keys, as §2's superset (R-56).

## R-289 — Rulings reach agents only in the opening prompt of a fresh dispatch
*30 Sep 2026 · standing practice, applied in the orchestrator's loop*

"Standing practice (R-289): rulings reach agents only in the opening prompt of a fresh dispatch, never as a mid-task
message. If a ruling lands while an agent is mid-task, let it finish its current step and stop, then re-dispatch fresh
with the ruling."

*Applied:* on PR #79, rulings R-286 and R-288 were relayed to a running implementer as messages. The permission
classifier treated them as possible instruction poisoning and blocked the edits, and the agent stopped without a
change. From now on, a ruling that lands mid-task waits: the agent finishes its current step and stops, and a fresh
agent is dispatched with the ruling, in the human's words, in its opening prompt. #79's R-286 and R-288 round is
re-dispatched that way.

## R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*
*Amended by R-335, R-336 and R-342.*
*Still in force: all of it; R-335, R-336 and R-342 each name test files, with implementer commits, that qa may change
under a ruling (`qa_TASK-M0-22_r235.rs`'s `if:` check; TASK-M0-45's splits of the long tests; TASK-M0-48's scratch
cleanup in `xtask/tests/qa_TASK-M0-38.rs` and `crates/validation/tests/qa_TASK-M0-38.rs`, accepted by R-346).*
*30 Sep 2026 · recorded; the orchestrator's check on qa's commit takes it*

"RQ-172: option 1 (R-290). qa may modify or delete test files that only qa has ever committed to (checked with git
log). Every such change is listed in the PR with its reason, and the code reviewer confirms no assertion was weakened
except where a ruling changed the behaviour it tests. The implementer still never edits qa's files."

*Applied:* besides `A` lines, the orchestrator's check on qa's commit (R-237) accepts `M` and `D` lines on a test file
whose every earlier commit, by `git log`, is a qa commit ("qa: tests for …"). The PR lists each such change with its
reason, and the code reviewer confirms that no assertion was weakened, except where a ruling changed the behaviour it
tests. The implementer still never edits a file of qa's. It is recorded with R-292, which changes nothing else in
CLAUDE.md: CLAUDE.md's qa paragraph and `.claude/agents/qa-reviewer.md` still state R-237's add-only form.

## R-291 — TASK-M0-40's three veto items stand
*30 Sep 2026 · applied in TASK-M0-40 (PR #82)*

"#82: all three veto items stand."

*Applied:* PR #82's three "Applied per R-204 — veto?" items stand as the PR wrote them. Design: a named node with no
bounds counts as clipped, not present, and so does one whose rect only touches the surface edge (R-275). Process: the
re-run of a stale `reviews` run is a step inside the `reviews-complete` job, so the list of required checks doesn't
change. Test infra: the new tests build their own one-case suite in a temp dir (`CARGO_TARGET_TMPDIR`), so
`screenshot --all` in CI is unchanged.

## R-292 — Forward lines on amended rulings, a generated CURRENT_RULES.md, and a review queue of open entries only
*Amended by R-293.*
*Still in force: all six items and their Applied choices, except which rulings CURRENT_RULES.md leaves out and how it
shows the rest (R-293).*
*30 Sep 2026 · applied in decisions.md, REVIEW_QUEUE.md, docs/archive/review_queue/, plan/check_plan.py, plan/tools/,
plan/CURRENT_RULES.md and CLAUDE.md*

"decisions.md hygiene (R-292), one small PR, no rulings changed:
1. Every ruling amended, superseded or corrected by a later one gets an "Amended by R-n" (or "Superseded by R-n") line
   directly under its heading. There are 22 today, including R-5, R-12, R-17, R-110, R-175, R-176, R-211, R-214,
   R-223, R-239, R-252 and R-256.
2. check_plan.py fails if a ruling says it amends/supersedes/corrects/replaces R-n and R-n lacks the matching forward
   line.
3. Generate plan/CURRENT_RULES.md from decisions.md: every standing rule as currently in force, grouped (authority,
   physics conventions, values, process, CI), each citing its ruling. It's generated only, rebuilt by a script, and
   check_plan fails if it's stale. CLAUDE.md points agents to it for current rules, and to decisions.md for history.
4. Add R-195's missing "Applied:" note (PR #17).
Record R-290 (RQ-172) and R-291 (#82's veto items) in the same PR."

The human's addition:

"5. REVIEW_QUEUE.md holds open entries only. Every entry with a ruling moves, unchanged, to docs/archive/review_queue/
in one file per era: untangling.md (RQ-1 to ~128) and M0.md (the rest); later milestones get their own file after each
gate. IDs never change. check_plan.py resolves RQ-n and R-n references in the live files and the archive, and fails on
any it can't find.
6. decisions.md stays whole: it's the law as well as its history. Agents read CURRENT_RULES.md for rules in force, and
decisions.md only for why. Revisit splitting it per milestone (numbers unchanged) at a gate, if it becomes unwieldy."

*Applied:*
- Item 1: 38 rulings gain a forward line directly under the heading ("*Amended by R-m.*", or "Superseded in part",
  "Corrected", "Reversed in part", "Refined" or "Extended by R-m"). The count is 38, not 22: a scan of every
  ruling's heading and text for "amends", "supersedes", "corrects" and "replaces" R-n finds 33, and "reverses",
  "refines" and "extends" 3 more (R-156, R-253, R-215); R-7 (its "R-7 amended" entry) and R-237 (R-290) make 38.
  R-93, R-95, R-104, R-109, R-122 and R-141 already carried theirs in their date line and keep it there.
- Item 2: `plan/check_plan.py` fails when a ruling says it amends, supersedes, corrects or replaces R-n, and R-n has no
  matching forward line in its heading block. Reverses, refines and extends are checked the same way (applied per
  R-204 — veto?). The checks are in `plan/tools/rulings.py`.
- Items 3 and 6: `plan/tools/current_rules.py` generates `plan/CURRENT_RULES.md` from this file and
  `plan/rule_groups.yaml`, and `check_plan.py` fails if it is stale. Applied per R-204 — veto?: the groups live in
  that mapping file rather than in a line under each heading; a sixth group, "Design and architecture", holds the
  architecture, payload, scheduler, render, colour, GUI and tooling rulings; each ruling's group is a judgement from
  its text; and the one-off acts are listed last, under "One-off acts (history only)". A ruling superseded outright
  would be left out; none is today. CLAUDE.md points agents to CURRENT_RULES.md, and to this file for history.
- Item 4: R-195's Applied note (PR #17).
- Item 5: `REVIEW_QUEUE.md` keeps RQ-173, the only open entry. The ruled entries moved unchanged to
  `docs/archive/review_queue/untangling.md` (RQ-1 to RQ-128, tagged "step 1" to "step 7") and `M0.md` (RQ-129 onward,
  tagged build, plan, calibration and the like from RQ-129's "(build, TASK-M0-01)"). RQ-169 and RQ-171, filed on PR
  #78 and cited here by R-278 and R-288, were not on main; they are copied into `M0.md` unchanged from that branch.
  `check_plan.py` resolves every R-n and RQ-n in the live files and the archive, and `coverage.py` takes its RQ ids
  from the queue and its archive.

## R-293 — CURRENT_RULES.md shows each rule's current form: superseded rulings leave it, partly amended ones say what still stands *(amends R-292)*
*Amended by R-295.*
*Still in force: all of it; R-295 changes only its application to R-252, which is amended, not superseded, by R-277.*
*30 Sep 2026 · applied in decisions.md, plan/tools/, plan/check_plan.py, plan/CURRENT_RULES.md, CLAUDE.md and
`.claude/agents/qa-reviewer.md`*

"#84: veto items 1–4 stand. Item 5 changes (R-293): CURRENT_RULES.md shows every rule's current form. Audit each
"Amended by" pair: where the later ruling fully replaces the earlier (e.g. R-239 → R-252 → R-277), mark the earlier
"Superseded by R-n" and drop it from the digest. Where only part is replaced, the earlier entry carries one line,
"Still in force: …", drafted by the agent and checked in review. Also update CLAUDE.md's qa paragraph and
.claude/agents/qa-reviewer.md to R-290. Merge #84 once that's done and green."

*Applied:*
- PR #84's "applied per R-204 — veto?" items 1 to 4 stand as R-292's Applied note has them: the verb set, the group
  mapping file, the sixth group and the one-off acts listed last. Item 5 (a ruling is left out only on an outright
  "Superseded by") changes as below.
- The audit covers every forward pair: 53, on 46 rulings (the 51 pairs R-292 recorded, and R-288 and R-292's pairs
  here). Where the later ruling replaces the whole of the earlier, the earlier's forward line now reads "Superseded by
  R-n", and the ruling leaves CURRENT_RULES.md: R-189 and R-190 (by R-191; R-190 had amended R-189 in part), R-192
  (by R-194), R-223 (by R-225), R-239 (by R-252), R-252 (by R-277) and R-253 (by R-254). Each of the other 39 carries
  one line under its forward line (or date line), "*Still in force: ….*", drafted per R-293 and checked in review.
  The PR lists every pair with its classification.
- `plan/tools/current_rules.py` gives a partly amended ruling's "Still in force" line in its digest entry, then the
  rulings that amend it. `plan/check_plan.py` fails if a ruling with a forward line that isn't superseded outright has
  no "Still in force" line, if one superseded outright has one, or if CURRENT_RULES.md lists a ruling superseded
  outright. A "Superseded by R-m" line also answers R-m's claim on the ruling, whatever its verb ("amends",
  "refines").
- CLAUDE.md's qa paragraph and `.claude/agents/qa-reviewer.md` state R-290's rule, citing it; nothing else in either
  changes.

*Applied per R-204 — veto?:* R-252 is marked superseded by R-277, as the human's example chain has it. R-277 restates
R-252's limits (two agents at warning, three at normal, only running work at critical) but not its "Log the pressure
level instead of swap in summaries", which leaves the digest with R-252. R-253 is marked superseded by R-254, which it
"refines": `ftle` reads NaN whenever `ftle_valid` is false, which includes `step_count = 0`, so nothing of R-253 stands
apart from R-254.

## R-294 — R-288's counters belong to the frame; no mutable statics in the kernel *(amends R-288; closes RQ-174)*
*30 Sep 2026 · applied in TASK-M5-28 and REQ-TOOL-140; PR #78 (TASK-M0-10) applies the CPU half*

"#78: the program-wide static is vetoed (R-294). R-288's counters belong to the frame, never global state: on the CPU,
the packer's caller passes in a per-frame counters struct and reads it back; on the GPU, an atomic u32 buffer bound
per frame and reset each frame, read back asynchronously with the telemetry readback. No mutable statics in the
kernel. RQ-174 (GPU binding and readback) becomes part of the task that builds the telemetry readback; state which task
in the ruling."

*Applied:* on the CPU, the `d_min` packer counts into a per-frame counters struct that its caller passes in and reads
back after the frame. PR #78 (TASK-M0-10) drops its process-wide `DMIN_COUNTERS` static, and the kernel holds no
mutable static. On the GPU, `dmin_nan_unset` and `dmin_negative_floored` are an atomic u32 storage buffer, bound to
each frame's dispatch and reset each frame, and read back asynchronously with the telemetry readback (R-288), never
stalling a frame; their values go into that frame's record. REQ-TOOL-140 (new) carries the GPU half. RQ-174 moves to
`docs/archive/review_queue/M0.md` with its Ruling line.

*Applied per R-204 — veto? (which task):* TASK-M5-28, the frame record, percentiles and the bounded telemetry file. No
task builds a GPU telemetry readback under that name: TASK-M5-14 builds only the dispatch queue's `QuadReduction`
readback and the measurement path, and TASK-M6-16 times the GPU with timestamp queries. TASK-M5-28 builds the frame
record (telemetry §2), where the counters belong, so it binds the buffer, reads it back and writes the values into the
record. Its Depends on reaches TASK-M4-05, the compute kernel whose march packs `d_min`, through TASK-M5-21, so no
dependency is added.

## R-295 — R-252's summary logging stays in force; two instruction-file edits *(amends R-293)*
*30 Sep 2026 · applied in decisions.md, plan/CURRENT_RULES.md, CLAUDE.md and `.claude/agents/code-reviewer.md`*

"R-252: "log the pressure level in summaries" stays in CURRENT_RULES.md as its "Still in force" line.
Instruction files: both edits approved. Add R-290's no-weakened-assertion check to .claude/agents/code-reviewer.md,
and point CLAUDE.md's memory bullet at the current rule (R-277) instead of R-239 and R-252."

*Applied:* R-293's veto item on R-252 changes. R-252 is amended by R-277, not superseded. Its forward line now reads
"Amended by R-277", and a "Still in force" line keeps its logging rule: summaries log the memory-pressure level, not
swap. R-252 returns to the digest with that line. R-239 stays superseded by R-252. `.claude/agents/code-reviewer.md`
gains R-290's check: where qa's commit modifies or deletes a test file that only qa has committed to, the code reviewer
confirms that no assertion was weakened, except where a ruling changed the behaviour it tests. CLAUDE.md's
memory-pressure bullet cites R-277 in place of R-239 and R-252. The same message asked for the full text of #79's
items 12 and 15 before ruling on them; they stay open.

## R-296 — R-269's half-way fixture keeps one reference per backend; explicit quantisation makes exact ties identical, not values near one *(closes RQ-175; amends R-287)*
*30 Sep 2026 · applied in parity contract §4, REQ-VAL-176 and TASK-M0-43*

"R-296 (closes RQ-175): option 1. R-269's fixture keeps one reference
image per backend, R-287's own fallback. Reword TASK-M0-43's acceptance
line and REQ-VAL-176: the evidence is that lavapipe's quantised bytes
equal its automatic ones, and that both backends round exact ties to
even. Qualify parity contract §4: explicit quantisation makes exact ties
identical, but values within an ulp of a tie can differ on backends
whose display shaders compile with fast-math (Metal via wgpu), so
goldens near a tie use per-backend references."

*Applied:* RQ-175's option 1. R-269's half-way fixture keeps one reference per backend, the fallback R-287 names, and
TASK-M0-43's PR names it. TASK-M0-43's first acceptance line and REQ-VAL-176's verify detail now take as the evidence
that lavapipe's quantised bytes equal its automatic ones, and that at exact f32 ties both backends round to even; the
fixture renders max step 0 on each backend against its own reference, and the control still shows max step 1 between
the backends. Parity contract §4 qualifies "byte-exact across backends by construction": explicit quantisation makes
exact ties identical, but a value within an ulp of a tie can differ on a backend whose display shaders compile with
fast-math (Metal via wgpu), so a golden near a tie keeps one reference per backend. The rest of R-287 stands: fragment
output quantises in the shader, and a golden case whose bytes agree across backends keeps one reference. RQ-175 moves
unchanged to `docs/archive/review_queue/M0.md`, with its Ruling line.

*Applied per R-204 — veto? (plan):* REQ-VAL-176's statement is qualified the same way: every backend writes identical
bytes at exact ties, and a case whose bytes still differ, a golden near a tie among them, keeps one reference per
backend. TASK-M0-43's last acceptance line (`cargo xtask golden --all`) reads "against one reference per case, or one
per backend for a case the PR names", since the half-way fixture is now such a case. Its title is unchanged.

*Result (the measurement RQ-175 reported, recorded here so the corpus can cite it; 30 Sep 2026, CI run 36736481929,
branch `measure/m043-lavapipe-halfway`, deleted afterwards, no PR):* R-269's fragment rendered through TASK-M0-43's
runner on hosted Metal and lavapipe, the runner's own shader scaling each channel to 0..255, rounding half to even and
storing `k / 255` in the `Rgba8Unorm` target. With the automatic conversion (the control), Metal against lavapipe is max
step 1 on 32768 of 65536 pixels, every even x, in R only: R-269's result, reproduced. Quantised in the shader, Metal
against lavapipe is still max step 1, on 24064 of 65536 pixels (94 of the 256 columns, all even x), in R only; G and B
are identical. The rounding itself agrees: lavapipe's quantised render is byte-identical to its automatic one, and at
the columns where both backends' f32 `R × 255` is exactly x + 0.5, both round to even. Hosted Metal matches a local
M3 Pro byte for byte. The cause is the fragment's own arithmetic, before any rounding: on lavapipe, R =
`(x + 0.5) / 255.0` matches correctly rounded f32 division; on Metal, it matches `(x + 0.5) × f32(1/255)`, because
wgpu 30 compiles MSL with the default `MTLCompileOptions`, which has fast-math on, and offers no switch to turn it off.
On those 94 columns the two values are one ulp apart, on opposite sides of x + 0.5, so the rounded levels differ by
one.

## R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*
*Amended by R-303.*
*Still in force: all of it on native backends. In the browser build: the compute setting stays explicit, off by default,
on the sim key, in pxpack and recorded in the header as asked for, and display stages may keep fast-math on; bit-identity
with the setting off no longer holds there, each stage's compiled mode is recorded as "unknown", and runs are held to
R-85's Tier-N tolerances (R-303).*
*30 Sep 2026 · applied in parity contract §4, render contract Part 3, caching contract Parts 1 and 2, gui_state_contract
§2, dd_image_embedding §6, telemetry §5, render_gui_spec §G5 and colour_composition §6; REQ-INT-057; REQ-SYS-074 and
REQ-TOOL-141 in TASK-M0-44 (new), REQ-COL-060 in TASK-M2-29 (new), REQ-SCHED-097 in TASK-M4-08, REQ-PERF-094 and
REQ-VAL-177 in TASK-M4-20 (new), REQ-TOOL-142 in TASK-M7-31, and REQ-GUI-163 and REQ-GUI-164 in TASK-M8-43 (new)*

"R-297 (fast-math, per shader stage):
- Compute shaders (the simulation): fast-math is an explicit, recorded
  setting, off by default and on as an opt-in optimisation; the project
  never inherits it silently. Off: bit-identity and identical branch
  decisions hold (R-84, REQ-INT-057). On Metal that means compiling our
  own MSL with fast-math off and loading it through wgpu's passthrough,
  for the compute pipelines only. On: allowed for speed; parity becomes
  "measured, not exact", with on-versus-off differences measured and
  reported. The compute setting is part of the sim key, recorded in
  pxpack, and shown in the profiler and the Run window.
- Vertex and fragment shaders (display): fast-math may stay on.
- Exception: fragment paths that recompute physics to check agreement
  (the DECODE/ROUNDTRIP presets, R-116) compare within a stated
  tolerance, a calibration requirement, not bit-exactly.
- Record the per-stage mode in the telemetry header. File the work as
  tasks where it belongs (the compute passthrough path in M0/M4, the Run
  window control in M8), and measure the performance difference so
  turning it on is an informed choice."

*Applied:*
- Docs. Parity contract §4 states the per-stage rule: compute fast-math is off by default, an explicit setting that
  is recorded and never inherited; off, bit-identity and identical branch decisions hold (R-84, REQ-INT-057); on,
  parity is measured, not exact. On Metal, off is the project's own MSL compiled with fast-math off and loaded through
  wgpu's passthrough, for the compute pipelines only. The display stages may keep fast-math on; the DECODE and
  ROUNDTRIP presets compare within a stated tolerance. The setting joins the sim key (render contract Part 3) and the
  payload compatibility signature with its blast-radius row (caching contract Parts 1 and 2), the embedded record's
  sim fields (dd_image_embedding §6), and the Profiler and Run window (render_gui_spec §G5). Telemetry §5's header
  records the compute setting asked for and each stage's mode as compiled: compute, vertex and fragment.
  colour_composition §6 gives the agreement presets their tolerance.
- R-84 and R-116 carry forward lines: R-84's guarantee holds with compute fast-math off, and the agreement presets
  R-116 describes compare within a tolerance. REQ-INT-057 is qualified: "with the compute shaders' fast-math off, the
  default (R-297)".
- Plan. REQ-SYS-074 (the explicit setting, default off, and the Metal passthrough for the compute pipelines only) and
  REQ-TOOL-141 (the compute setting and each stage's compiled mode in the session header) go to a new task, TASK-M0-44. REQ-SCHED-097 (the setting on
  the sim key) joins TASK-M4-08, which builds the sim key. REQ-PERF-094 (the march's speed, on against off, a
  benchmark on the human's Mac, R-186) and REQ-VAL-177 (the on-versus-off differences, measured and reported) go to a
  new task, TASK-M4-20. REQ-TOOL-142 (the setting in the embedded record) joins TASK-M7-31, which builds what
  travels. REQ-GUI-163 (the Run window control) and REQ-GUI-164 (the Profiler's compute setting) go to a new M8 task,
  TASK-M8-43. REQ-COL-060, the agreement presets' tolerance (the DECODE view's fragment decode against the compute
  kernel), is a calibration requirement (R-71), closed by a new task, TASK-M2-29.

*Applied per R-204 — veto?:*
- Plan (where the passthrough goes): a new M0 task, TASK-M0-44, after TASK-M0-14 (the first kernel dispatched on the
  GPU) and TASK-M0-19 (the session header), so every compute pipeline from M0 on is built through it. No existing
  task takes it: TASK-M0-14 is ~450 lines already, and TASK-M4-01 and TASK-M4-06 come after M0's compute dispatches.
  TASK-M4-06's variant table builds its pipelines through TASK-M0-44's entry point.
- Plan (the other placements): the sim key in TASK-M4-08 and the embedded record in TASK-M7-31, the tasks that build
  them; the Run window control and the Profiler line in a new TASK-M8-43, after TASK-M8-24 and TASK-M8-28 (TASK-M8-28
  is already over budget); the benchmark and the difference report together in TASK-M4-20, after TASK-M4-19's
  kernel benchmarks; the tolerance in TASK-M2-29, after TASK-M2-25 builds the agreement presets and their gate.
- Design (ROUNDTRIP's tolerance): ROUNDTRIP's residual is computed wholly in the fragment, `encode(decode(z))` with
  nothing from the compute shader (colour_composition §6), and it already has a stated tolerance, itself a
  calibration requirement: ε_phys (REQ-ENC-024; debug_tooling_plan §A; REQ-RENDER-025). So R-297's "stated tolerance"
  for ROUNDTRIP is ε_phys, checked with the fragment compiled as each backend compiles it, and REQ-COL-060 covers the
  agreement presets only. A ROUNDTRIP residual above ε_phys under fragment fast-math is a REVIEW_QUEUE entry, not a
  second tolerance.
- Design (where the setting lives): it is a `SimConfig` field (gui_state_contract §2), since it is on the sim key and
  every Run window field is a `SimConfig` field (render_gui_spec §G5).
- Design (what is recorded): the header records both the compute setting asked for, the sim key's, and the mode each
  stage was compiled with on the running backend. Where wgpu's own path compiles without fast-math and offers no
  switch (Vulkan, on lavapipe in CI), off is that path, and on compiles the same way: the header records the setting
  on and the compute stage compiled off.
- Design (what the Profiler shows): the compute setting, as R-297 asks, with the compute stage's compiled mode beside
  it where the two differ, so a run on lavapipe with the setting on doesn't read as fast-math on. The vertex and
  fragment modes are recorded in the header but not shown in the Profiler, since R-297 asks only that the compute
  setting be shown there.
- Design (the difference report): `cargo xtask gate fast-math-diff`, run in the `gpu-metal` job; it reports branch-word
  forks, the largest continuous-word differences and the outcome-class fractions, on against off, and its control is
  off against off, exactly zero.
- GUI: the Run window control and the Profiler line have no artboard, so they are checked by presence only until the
  M8 dev GUI (R-129).
- Amendment: R-297 amends R-84 (branch decisions are identical only with compute fast-math off) and R-116 (the
  agreement presets compare within a tolerance), and each carries a forward line.
- Amendment (R-133), a consequence of R-297, not a mechanical one: REQ-COL-006's agreement gate moves, from TASK-M2-29
  on, off REQ-DEC-043's calibrated f32 decode factor, which R-133 accepted as its tolerance until REQ-VAL-064 sets
  Tier N, and onto REQ-COL-060's calibrated tolerance. The fragment decode may compile with fast-math and the kernel's
  decode doesn't, so the agreement presets need the stated tolerance R-297 asks for. REQ-DEC-043's factor stays the
  tolerance for the fragment decode against the f64 `decodeOnly()` (REQ-TOOL-029). R-133 carries a forward line, and
  REQ-COL-006's note says the same.

*Flagged, not applied:* `principia_gpu_determinism_note.md` § "The mechanism (measured, not inferred — the
attribution was overturned by a controlled test)" records that turning Metal's fast-math off
did not restore `N_sub`'s determinism (the cause was transcendental latitude, in every math mode), and that the switch
"is not available in a browser regardless". R-297 leaves the first as it stands: branch decisions stay comparison-only
(Tier B), whatever the math mode. The second is open: what the compute
setting means in the browser build (M8), where WebGPU gives no fast-math control, isn't settled; it is asked in the PR. It is
filed as RQ-177.

## R-298 — TASK-M0-17's items 12 and 15 accepted; a trace with no summary line is valid *(amends R-286)*
*Amended by R-299 and R-341.*
*Still in force: items 12 and 15 as accepted; a trace with no final summary line, its last line a frame record or the
header line, is valid: the reader returns the frames, reports `leak_flags` and `hot_paths` as absent with "session
incomplete", and never rejects the file for it; `prin profile query --live` works on an in-progress trace. R-299
replaces only the Applied note's rule that a last line cut off inside its JSON object is rejected. R-341 has
`prin profile` append the summary line as the final line at session end, after frames streamed as they complete.*
*30 Sep 2026 · applied in telemetry §5, REQ-TOOL-008, REQ-TOOL-101 and TASK-M0-17 (PR #79)*

"This is from me. #79: items 12 and 15 accepted (R-298), with one
condition on 15: a trace with no final summary line (a crashed or
still-running session) is valid. The reader returns the frames, reports
leak_flags and hot_paths as absent with "session incomplete", and never
rejects the file for it; prin profile query --live works on an
in-progress trace. Add a test for a truncated trace."

*Applied:* item 12 stands: two `by_kind` entries in one pool with the same `kind`, or two `allocations` entries in one
stage with the same `kind` and `pool`, are refused by the writer and rejected by the reader, the fourth reader-only
exception in telemetry §5. Item 15 stands: `leak_flags` and `hot_paths` take the file's last line, after the frames,
so R-286's "each later line is one compact frame record" holds for every line but that one. Its condition: telemetry
§5 now says a trace whose last line is a frame record, or the header line when no frame was recorded, is a session
that ended before its summary line, and is valid. `engine::contract::profile::read` returns its header and frames, and
reports `leak_flags` and `hot_paths` as absent, with "session incomplete". REQ-TOOL-008 gains the behaviour and a
truncated-trace test in TASK-M0-17. REQ-TOOL-101, closed by TASK-M8-28, gains "`prin profile query --live` works on an
in-progress trace (no summary line yet)".

*Applied per R-204 — veto?:* R-298 is in the "design" group of `plan/rule_groups.yaml`, beside R-282 and R-286. A
header line alone counts as an incomplete session with no frames, since it too is "a trace with no final summary line".
A last line cut off inside its JSON object is still rejected: the ruling covers a missing summary line, not a partial
line, and §5 doesn't say otherwise. The type shape and the rest are in PR #79.

## R-299 — The reader drops a cut-off final line and says how many bytes it dropped *(amends R-298)*
*30 Sep 2026 · applied in telemetry §5, REQ-TOOL-008 and TASK-M0-17 (PR #79)*

"#79 item b (R-299): the reader drops an unterminated final line that doesn't parse, reports the session incomplete,
and states how many bytes it dropped. A malformed line ending in a newline stays an error. Implementer change plus
re-checks, then merge."

*Applied:* PR #79's veto item b (R-298's Applied note: "A last line cut off inside its JSON object is still rejected")
changes. Telemetry §5 now says a last line with no newline after it that is not one complete JSON value is the part of
a line a session was writing when it stopped: the reader drops it, reads the lines before it as the trace, reports the
session incomplete ("session incomplete", R-298), and states the number of bytes it dropped. A last line with no
newline that is complete JSON is read as before, R-298's rules unchanged: a summary line or a frame record, or an
error. A line that ends in a newline and is not the object its place calls for stays an error, wherever it is.
`engine::contract::profile::read` gives the count as `Trace::dropped_bytes`; REQ-TOOL-008 gains the behaviour and its
test in TASK-M0-17.

*Applied per R-204 — veto?:* "doesn't parse" is read as "is not one complete JSON value": a compact JSON object cut
anywhere before its closing brace never is, while a complete JSON object with no newline after it, whatever its keys, is
a written line and not a cut one, so it is read as it is today. Because the dropped line held the last place, the line
before it keeps a frame's place: a summary line followed by a cut-off line is an error, since the writer writes nothing
after the summary line. A file whose only line is a cut-off header line has no header line to read, and stays an error,
as an empty file is; the error states the bytes. R-299 is in the "design" group of `plan/rule_groups.yaml`, beside
R-286 and R-298.

## R-300 — #78's items 11–13 are accepted; `DminCounters`' fields are private
*30 Sep 2026 · applied in TASK-M0-10 (PR #78)*

"#78 items 11–13 accepted (R-300), with 13 amended: DminCounters' fields are private, with increment, read and reset
methods; workers still share &DminCounters."

*Applied:* PR #78's veto items 11 (`set_d_min_counted` removed) and 12 (`PackedA::pack(&self, counters)`, with
`roundtrip_ctl` on a scratch pair) stand. Item 13 changes: `DminCounters`' two counters are private fields, reached
through methods that increment, read and reset them. Workers still share one `&DminCounters` per frame, so R-294 holds:
the caller owns the counters and no kernel static remains.

## R-301 — TASK-M0-42's CI cost is accepted *(amends R-270)*
*Amended by R-325.*
*Still in force: the measurement, about 10m28.5s per warm `ci` run on PR #85; R-325 makes ~10.5 min the target for
each CI job again.*
*30 Sep 2026 · applied in TASK-M0-42 (PR #85) and REQ-SYS-073*

"#85: the ~1.5 min cost is accepted (R-301). You may delete the old v0-rust-* Actions caches."

*Applied:* R-285's caching (registry and fixture pool only, per-job keys) costs about 1.5 min per warm `ci` run, 8m56.5s
before against 10m28.5s after, and about 1m42s per warm `xtask-ci` run, as PR #85 measured. That cost is accepted, and
R-270's "no slower than before (~10.5 min)" gives way to it as the `ci` target: REQ-SYS-073's second acceptance is met by
this ruling, not by a run under ~10.5 min. When this was recorded, GitHub had already evicted the old `v0-rust-*`
caches. The ten `v0-rust-*` entries then present were all in current use by PR #65's branch, so none was deleted.

## R-302 — Per-PR mutation runs are sharded across parallel CI jobs; the nightly full run is the backstop *(closes RQ-176; amends R-196)*
*30 Sep 2026 · applied in REQ-VAL-148, REQ-VAL-149 and TASK-M0-23 (PR #65)*

"RQ-176, ahead of #65's proposal (R-302): per-PR mutation runs are sharded across parallel CI jobs (cargo mutants
--shard k/n), each with a timeout, and the full nightly run stays the backstop. #65's proposal sets n and the
per-shard limit, calibrated at the M0 gate."

*Applied:* the per-PR `cargo mutants --in-diff` job runs as n parallel CI jobs, shard k of n each
(`--shard k/n`), each under its own time limit. REQ-VAL-149's calibration becomes the pair: the shard count n and the
per-shard time limit. PR #65 (TASK-M0-23) proposes both with its evidence, and the human confirms them at the M0 gate
(R-182). The nightly full run (REQ-VAL-150) stays as it is, the backstop for anything a per-PR shard misses. RQ-176 was
filed on PR #65's branch; it is archived here, unchanged, with this ruling, and PR #65 drops it from REVIEW_QUEUE.md when
it merges main. RQ-176's question whether 180 minutes stands meanwhile is answered by the same proposal: #65 replaces
its one provisional limit with n and the per-shard limit.

## R-303 — In the browser build, each stage's compiled fast-math mode is "unknown" *(closes RQ-177; amends R-297)*
*30 Sep 2026 · applied in telemetry §5, render_gui_spec § "Run — from the top bar", REQ-GUI-163, REQ-VAL-116 and
REQ-TOOL-143 (new) in TASK-M8-37*

"RQ-177 (R-303): accepted as recommended. In the browser the compiled mode is recorded as "unknown", the Run window
control is disabled, and runs are held to R-85's tolerances."

*Applied:* RQ-177's option 1. In the browser build, where WebGPU offers no fast-math control and no passthrough, the
session header records the compute setting as asked for, and the compiled mode of each stage (compute, vertex and
fragment) as "unknown". The Run window shows the compute fast-math control disabled, with a note that the browser
chooses the mode. Browser runs are held to R-85's Tier-N tolerances, measured, not bit-exact (REQ-VAL-116). The sim key
and pxpack keep the setting, so a view made in the browser opens natively with it.
*Applied per R-204, confirmed by R-304:* the header's browser rule is a new requirement, REQ-TOOL-143, closed by
TASK-M8-37 (the browser build), since REQ-TOOL-141 is an M0 requirement on native backends.

## R-304 — The "veto?" items on #78, #79, #89 and #90 stand
*30 Sep 2026 · applied in PRs #78, #79, #89 and #90*

"All stand: #78 items 15–16; #79 items f–i; #89's REQ-TOOL-143 (a new M8 requirement closed by TASK-M8-37); #90's 2 s
post-SIGKILL bound and its reuse of TASK-M0-26's id and reviewers. Merge each once its reviews and CI are green."

*Applied:* PR #78's item 15 (`DminCounters::reset` takes `&mut self`, so only the owner resets the pair, never while a
worker holds it) and item 16 (the increment methods take telemetry §2's counter names) stand. So do PR #79's items f
(an unterminated last line that is complete JSON of the wrong shape is rejected, not dropped), g (a cut-off line after
the summary line is an error), h (a cut-off header as the only line is an error stating its bytes) and i (#79's
ruling on cut-off lines is in the design group). REQ-TOOL-143 stands as recorded under R-303. PR #90's 2 s bound on the wait for the process group
after SIGKILL, and its reuse of TASK-M0-26's id and reviewers for a defect fix in that task's merged code, stand. Each
PR merges once its named reviewers approve its head and CI is green.

## R-305 — #65's provisional mutation values and items 10–13 stand; `mutants-check` becomes a required check on `main`
*30 Sep 2026 · applied in PR #65 (TASK-M0-23): `.github/workflows/mutants.yml` and `plan/HUMAN_SETUP.md` §2*

"#65 (R-305):
- Provisional values stand: n = 8 shards, 120 min per shard, confirmed or replaced at the M0 gate.
- Items 10–13 stand.
- mutants-check becomes a required status check on main. First make sure it runs on every PR and passes trivially when
the diff has no mutants (docs-only and rulings PRs), so no PR can wait on a check that never reports. Then add it to
main's branch protection yourself; I authorise that one settings change.
- The comment-width nit: fix it in the next PR that touches that file.
Then merge #65 and resume queued work, TASK-M0-43 first."

*Applied:* REQ-VAL-149's pair, n = 8 shards and 120 minutes per shard, stays in `mutants.yml` as proposed and marked
provisional, until the human confirms or replaces it at the M0 gate (R-71, R-182, R-302). PR #65's "Applied per R-204 —
veto?" items 10–13 stand: round-robin sharding; one `mutants-check` job over every shard, beside each shard's own check;
the fixture self-test as its own job, `mutants-fixture`; and `fail-fast: false` on the shard matrix. `mutants-check`
becomes a required status check on `main` once it reports on every pull request: PR #65 makes it run on every pull
request whatever its diff, and pass when the diff has no mutant, whether the diff changes no Rust source (a docs-only or
rulings PR) or changes Rust source with nothing to mutate. Once #65 merges, the orchestrator adds `mutants-check` to
`main`'s branch protection, the one settings change the human authorised here, and `plan/HUMAN_SETUP.md` §2 lists it.
The comment-width nit (`xtask/src/mutants_check.rs` line 3, over 120 columns) is fixed by the next PR that touches that
file.

## R-306 — `QuadReduction`'s member list is ledger data at M0; the struct is built at M5 *(closes RQ-178)*
*1 Oct 2026 · applied in PR #97: TASK-M0-11, TASK-M5-01, REQ-PAY-019, REQ-PAY-077*

"RQ-178: as recommended. M0 records §3.7's member list as ledger data and tests REQ-PAY-019 on it; building the struct
moves to TASK-M5-01 (R-113)."

*Applied:* RQ-178's option 1. TASK-M0-11 transcribes generation-root §3.7's `QuadReduction` members into
`crates/ledger/src/payload.rs` as ledger data, each with its name and §3.7 type, not as §3.8 entries and not emitted;
its REQ-PAY-019 test checks that list (`spread_event` is f16; there is no `ensemble_outcome_agreement`). The generated
struct, the members' §3.8 entries and `class_histogram`'s placement are TASK-M5-01's, with REQ-PAY-075 and REQ-PAY-077
(R-113). RQ-178's sub-question, how M5 types the f16 members (`f16-pair` at packed 16-bit locations, or a new §3.8
`f16` type), was not ruled; it is filed as RQ-182 for M5 and blocks nothing in M0. Numbered from R-306: the ruling
block said "R-299 onward", but R-299 to R-305 were already taken.

## R-307 — `continuation_index` holds 3 where `next` is `prev`'s inverse *(closes RQ-179)*
*1 Oct 2026 · applied in PR #97: payload §3, generation-root §3.3, REQ-PAY-016, TASK-M0-11, TASK-M0-13*

"RQ-179: the four unset continuation_index cells hold 3 ("invalid"), matching dmin_pair."

*Applied:* payload §3 states that the four cells of `continuation_index[prev][next]` where `next = inverse(prev)` hold
3, which is no digit, as `dmin_pair`'s 3 means unset/invalid (payload §2), and gives the whole 4 × 4 table derived from
`cont_symbol`. The table is frozen and hashed with the ledger (R-36, R-63), so its Rust and WGSL emissions both carry
the 3s: TASK-M0-11 (Rust) and TASK-M0-13 (WGSL, REQ-PAY-016) assert them. Numbered from R-306: the ruling block said
"R-299 onward", but R-299 to R-305 were already taken.

## R-308 — A session that opens no GPU writes `api: "none"`, its GPU fields null *(closes RQ-180)*
*1 Oct 2026 · applied in PR #97: telemetry §5, REQ-TOOL-144 (new), REQ-TOOL-006, TASK-M0-18*

"RQ-180: a session that opens no GPU writes api: "none", with the other GPU fields null. Readers accept that. Never
open an adapter just to fill the header."

*Applied:* telemetry §5 adds "none" to `backend.api`'s values. A session that opens no GPU writes `backend.api` "none"
and `null` for `backend.driver`, `device.gpu`, `device.gpu_cores`, `device.memory` and `precision`, the fields RQ-180
names as the GPU's (its model, its driver, its memory split and its f32/f64 support); `device.cpu` and
`device.cpu_cores` are written as always. The typed form (`engine::contract::profile`) and `profile_v1.json` accept it;
TASK-M0-18 makes that change to TASK-M0-17's files, and M0's `prin profile` opens no adapter. REQ-TOOL-144 holds it.
Numbered from R-306: the ruling block said "R-299 onward", but R-299 to R-305 were already taken.

## R-309 — `SimConfig` and `RenderState` have one canonical serialisation; the profiler header's `config` uses it *(closes RQ-181)*
*Amended by R-318.*
*Still in force: `SimConfig` and `RenderState` are serialisable; the header's `config` is `{"scenario", "frames",
"sim", "render"}`; one canonical serialisation shared with snapshot JSON, share links and pxpack; R-318 makes it JCS
(RFC 8785), replacing the definition TASK-M0-18 was to write.*
*1 Oct 2026 · applied in PR #97: gui_state_contract §2, telemetry §5, render_gui_spec § "Export & share",
image_embedding §6, export_animation Part 6, REQ-TOOL-002, REQ-TOOL-145 and REQ-TOOL-146 (new), TASK-M0-18,
TASK-M8-32*

"RQ-181: as recommended (SimConfig and RenderState serialisable; config = {"scenario", "frames", "sim", "render"}),
with a canonical serialisation: stable key order and exact number formatting, shared with snapshot JSON, share links
and pxpack."

*Applied:* RQ-181's option 1. TASK-M0-18 makes the M0 skeleton's `SimConfig`, `RenderState` and their groups
serialisable (its Deliverables gain `crates/engine/src/contract/{sim_config,render_state}.rs`), and `prin profile`
writes the header's `config` as `{"scenario": NAME, "frames": N, "sim": SimConfig, "render": RenderState}`.
gui_state_contract §2 states that the two structs have one canonical serialisation, with a stable key order and exact
number formatting, shared by the profiler header's `config`, snapshot JSON, share links and pxpack. The corpus doesn't
say what that key order and number formatting are, so they are a definition requirement (R-72), REQ-TOOL-145: TASK-M0-18,
the first task that writes the serialisation, writes the definition into gui_state_contract §2, reviewed by the physics
reviewer (applied per R-72 — veto?). REQ-TOOL-146 (M8, TASK-M8-32) holds snapshot JSON, share links and pxpack to it.
Numbered from R-306: the ruling block said "R-299 onward", but R-299 to R-305 were already taken.

## R-310 — The "veto?" items on #94 and #95, and physics on TASK-M0-18, stand
*1 Oct 2026 · applied in PRs #94 and #95 and in TASK-M0-18's Reviewers*

"#94: both veto items stand. Merge #94.
#95's check in ledger::gen, and physics on TASK-M0-18: both stand."

*Applied:* PR #94's two items stand: the self-test cases keep their one analytic reference on lavapipe too (R-287), and
`screenshot.rs` is untouched, since no M1 renderer exists and it isn't on the golden path. #94 was merged
(7b935c2) before this entry was written, on the human's instruction. PR #95's item stands: the write-only-when-changed
compare lives in `ledger::gen::run_with_register`, so `.cargo/mutants.toml`'s reason for excluding
`xtask/src/codegen.rs` holds; #95 merged as 842012f. TASK-M0-18's Reviewers gain physics, because REQ-TOOL-119's
acceptance line and R-72 ask the physics reviewer to approve its definition (and, now, REQ-TOOL-145's). Numbered from
R-306: the ruling block said "R-299 onward", but R-299 to R-305 were already taken.

## R-311 — #97's "veto?" items stand; #97 merges once CI is green and its reviewer is done
*1 Oct 2026 · applied in PR #97 (merged as 7c3978d)*

"#97: all three veto items stand. Merge overnight once CI is green and the reviewer is done."

*Applied:* PR #97's items stand: R-308's "other GPU fields" are `backend.driver`, `device.gpu`, `device.gpu_cores`,
`device.memory` and all of `precision`, with `device.cpu` and `device.cpu_cores` still required; TASK-M0-18 writes
REQ-TOOL-145's definition (key order and number formatting) and the physics reviewer approves it (R-72); REQ-TOOL-146's
check sits in TASK-M8-32. The fourth "veto?" line, added by #97's review fixes after this ruling was asked for
(TASK-M0-19's probe fills the GPU fields only from an adapter the run already opened, per R-308), was accepted by the
code reviewer as a mechanical consequence of R-308 and merged with the rest, under R-234. #97 merged as 7c3978d once
the code reviewer approved its head (8e5e5e1) and CI was green.

## R-312 — §3.8 gains an `f16` type: one half-float at a packed 16-bit location, under R-248's rules *(closes RQ-182)*
*1 Oct 2026 · applied in generation-root §3.8, REQ-GEN-030 and TASK-M5-01*

"RQ-182 (ruled now, applies at M5): §3.8 gains a single f16 type for one half-float at a 16-bit location. f16-pair
stays two halves. The f16 type follows R-243's rules: exactly 16 bits, and a declared range within f16's finite range,
with overflow behaviour stated for unbounded ends."

*Applied:* the ruling cites R-243, which is TASK-M0-07's size acceptance; the rules it quotes are R-248's ("f16-pair and
fixed16 need exactly 16 bits … an f16 field's declared range must lie within f16's finite range (±65504). An unbounded
end fails unless the ledger entry states its overflow behaviour"), so R-248 is applied, and the rules are the ones the
human wrote either way. §3.8's `type` gains `f16`; a paragraph beside R-248's states it. REQ-GEN-030 (M5) carries it,
closed by TASK-M5-01, which types `QuadReduction`'s f16 members with it. *Applied per R-204 — veto?:* "at a 16-bit
location" is read as a packed location of exactly 16 bits only: `f16` is not a `scalar-index` type (a scalar index is
a 32-bit slot) and not a `vector` component type (a vector of halves stays `vector(f16-pair, k)`). R-248's
static-check rules for `f16-pair` (REQ-GEN-028) are unchanged.

## R-313 — `ICDescriptor` follows `Real`; R-86's 64 B is its f32 instantiation *(closes RQ-185; amends R-86)*
*1 Oct 2026 · applied in generation-root §3.6, dd_simstate_payload §1, render contract, debug_tooling_plan §E,
dd_decoder, REQ-PAY-017, REQ-PAY-087, TASK-M0-14 and TASK-M0-11*

"RQ-185: ICDescriptor follows Real. R-86's 64 B is its f32 instantiation."

*Applied:* `ICDescriptor`'s twelve float fields have the width of `Real`, as `SimState`'s widening fields do, so the
payload (`SimState` + `ICDescriptor`, systems_architecture) is a function of `Real` throughout (REQ-PAY-017). R-86's
"64 B with explicit padding" (12 × f32 = 48 B plus 16 B declared padding) is the f32 instantiation. The f64 and
DoubleF64 sizes and padding are not given by the corpus; they join REQ-PAY-087's definition, which TASK-M0-14 writes
into generation-root §3.6 for the physics reviewer to approve (R-72). `E₀ = K₀ + V₀` is then formed at `Real` width on
every path. TASK-M0-14 (PR #96) makes `ICDescriptor` generic with a layout row per precision; REQ-PAY-017 stays closed
by TASK-M0-14. PR #96's §1 sentence "`ICDescriptor` [is] not generic" is superseded. RQ-185 was filed in PR #101.

## R-314 — The `rustc-check-cfg` declaration for `spirv` in `crates/kernel/build.rs` is accepted *(closes RQ-184)*
*1 Oct 2026 · applied in TASK-M0-14 (PR #96)*

"RQ-184: accept rustc-check-cfg for spirv in crates/kernel/build.rs (no blanket allow)."

*Applied:* this is the ruling R-197 asks for to change lint configuration. `crates/kernel/build.rs` declares
`cargo::rustc-check-cfg=cfg(target_arch, values("spirv"))`, naming one expected value and leaving `unexpected_cfgs` on
for every other cfg. No `#[allow(unexpected_cfgs)]` or `#![allow(unexpected_cfgs)]` is used for it, at any scope.
RQ-184 was filed in PR #101.

## R-315 — `n_unresolved` is a u16 `QuadReduction` member, like `valid_sample_count` *(closes RQ-183)*
*1 Oct 2026 · applied in generation-root §3.7, REQ-REF-052 (new), TASK-M0-11 and TASK-M5-01*

"RQ-183: n_unresolved is u16, like valid_sample_count."

*Applied:* §3.7's temporal-accumulators table gains the row `n_unresolved` (u16), the latch's verdict (R-142): the
count of the quad's unresolved footprints, latched ones included, at most N² as `valid_sample_count` is. TASK-M0-11's
ledger member list (R-306) types it u16; TASK-M5-01 builds it into the struct (REQ-REF-052). RQ-183 was filed on PR
#99's branch (`task/TASK-M0-11`); it is archived from there, and #99's own REVIEW_QUEUE.md copy goes when #99 next
merges `main`.

## R-316 — #96's `closure_min` widening with `Real` stands
*1 Oct 2026 · applied in TASK-M0-14 (PR #96)*

"#96: closure_min widening with Real stands."

*Applied:* PR #96's "Applied per R-204 — veto?" item stands: `closure_min` widens with `Real` (f64 at f64), so a
persisted and resumed `SimState` keeps the running minimum at the march's own width (the physics reviewer's resume
argument). REQ-PAY-087's definition in dd_simstate_payload §1 carries it.

## R-317 — #98's `f16` restriction stands; `f16` is storage-only
*1 Oct 2026 · applied in generation-root §3.8, REQ-GEN-030 and TASK-M5-01*

"#98: the f16 restriction (packed 16-bit locations only) stands. f16 arithmetic in WGSL is optional in WebGPU, so f16
stays storage-only."

*Applied:* R-312's reading stands: `f16` is a type for a packed 16-bit location only, not a `scalar-index` type or a
`vector` component type (PR #98, merged as 2ade719). `f16` is storage-only: a generated read accessor widens it to f32
(`unpack2x16float`, core WGSL), and no generated code does arithmetic in binary16 or needs WGSL's optional
`shader-f16` feature (`enable f16`). §3.8 says so beside R-312's paragraph, and REQ-GEN-030 checks it.

## R-318 — The canonical serialisation is JCS (RFC 8785) *(amends R-309)*
*Amended by R-322.*
*Still in force: JCS (RFC 8785) — sorted keys, its number format, its test vectors; −0.0 written `0`; R-322 replaces
the per-value reading of "integers beyond 2^53" with a per-field rule.*
*1 Oct 2026 · applied in gui_state_contract §2, telemetry §5, REQ-TOOL-145 and TASK-M0-18 (PR #100)*

"#100: the canonical serialisation is JCS (RFC 8785): sorted keys and its number format, using its test vectors. −0.0
serialises as 0 (accepted). Integers beyond 2^53 (e.g. seeds) serialise as strings. Supersedes #100's bespoke number
rules."

*Applied:* `SimConfig` and `RenderState` serialise as RFC 8785 JSON Canonicalization Scheme text: object members sorted
by their names' UTF-16 code units (RFC 8785 §3.2.3), no insignificant whitespace, strings escaped as §3.2.2.2 gives,
and numbers in its format (ECMAScript's Number-to-String, §3.2.2.3). RFC 8785's published test vectors are the
acceptance test. −0.0 is written `0`, as JCS writes it. An integer whose magnitude exceeds 2^53 (a seed, say) is
written as a JSON string of its decimal digits. REQ-TOOL-145 becomes the ruled definition, so it is no longer a
definition TASK-M0-18 writes or the physics reviewer approves (REQ-TOOL-119's still is). PR #100's own key order (UTF-8
bytes) and number layout (ryu, positional for exponents −5 to 15) are superseded; for its ASCII keys the order is the
same. R-309's other text stands. *Applied per R-204, then ruled by R-322:* "integers beyond 2^53" was first read per
value (an integer field writing a number when |n| ≤ 2^53 and a string above it); R-322 rules it per field instead: a
u64 field is always a string, every other number a number.

## R-319 — An out-of-range input to the continuation tables is a `debug_assert!` failure; in release it returns 3 *(vetoes #99's item 8)*
*Amended by R-321.*
*Replaced in part by R-321 (its release behaviour).*
*Still in force: PR #99's item 8 stays vetoed (no input yields "the last cell" as a fallback); each input is
`debug_assert!`-ed. R-321 replaces the release behaviour: the functions are total, each input masked to 2 bits.*
*1 Oct 2026 · applied in payload §3, REQ-PAY-016, TASK-M0-11 (PR #99) and TASK-M0-13*

"#99 item 8 vetoed: an out-of-range input to the continuation tables is a debug_assert! failure; in release it returns 3
("invalid", R-307), never the last cell."

*Applied:* PR #99's item 8 (an input past the last code returns the last cell) is vetoed. Each generated
continuation-table function (`inverse`, `continuation_symbol`, `predecessor_symbol`, `continuation_index`) checks its
inputs with `debug_assert!`; a release build given an out-of-range input returns 3, never a table cell. Payload §3
states it beside R-307's table, and REQ-PAY-016 tests both builds. *Applied per R-204 — veto?:* the ruling calls 3
"invalid" (R-307), which holds for `continuation_index` only; symbol codes are a=0, A=1, b=2, B=3 (payload §3), so 3
from `inverse`, `continuation_symbol` or `predecessor_symbol` is the valid symbol `B`. Flagged to the human, who ruled
R-321: the functions are total (each input `debug_assert!`-ed < 4, then masked `& 3`), replacing this release
behaviour for all four.

## R-320 — CI caches the rust-gpu build, keyed on the pinned toolchain version *(amends R-285)*
*Amended by R-326.*
*Still in force: the rust-gpu build is cached under a key naming its job and the pinned toolchain version; R-326
saves it only on pushes to `main`.*
*1 Oct 2026 · applied in REQ-SYS-073, REQ-SYS-075 (new) and TASK-M0-14 (PR #96)*

"R-285 amended: cache the rust-gpu build (~/.cache/rust-gpu), keyed on the pinned toolchain version."

*Applied:* every CI job that builds the kernel restores and saves `~/.cache/rust-gpu` under a key naming the pinned
toolchain version (`rust-toolchain.toml`'s channel), so a toolchain bump starts a fresh cache. The rest of R-285 stands.
TASK-M0-14 (PR #96), which brings kernel builds into CI, makes the change (REQ-SYS-075).

## R-321 — The continuation-table functions are total: each input is debug-asserted < 4, then masked to 2 bits *(amends R-319)*
*Amended by R-324.*
*Still in force: the four functions are total in Rust and WGSL; each symbol input is `debug_assert!`-ed < 4, then
masked `& 3`; `continuation_index` returns 3 only for its inverse cells (R-307). R-324 sets the digit argument's rule.*
*1 Oct 2026 · applied in payload §3, REQ-PAY-016, TASK-M0-11 (PR #99) and TASK-M0-13*

"R-319 amended (R-321): the continuation-table functions are total. Each input is debug_assert!-ed to be < 4, then
masked to 2 bits (& 3), in Rust and WGSL, so no out-of-range return value exists and nothing is unspecified.
continuation_index returns 3 only for its genuine invalid cells (the inverse cases, R-307). This replaces R-319's
release behaviour for all four functions."

*Applied:* each generated table function (`inverse`, `continuation_symbol`, `predecessor_symbol`,
`continuation_index`), in Rust and in WGSL, `debug_assert!`s each input < 4 (Rust; WGSL has no `debug_assert!`) and
then reads the table at `input & 3`. A release build, and WGSL, given an input ≥ 4 return the cell at `input & 3`;
there is no separate out-of-range value. `continuation_index` returns 3 only for its four inverse cells (R-307).
R-319's release behaviour (return 3) is replaced for all four; its `debug_assert!` and its veto of #99's item 8 stand.
Payload §3, REQ-PAY-016, TASK-M0-11's `continuation_table_out_of_range` and TASK-M0-13's
`continuation_table_wgsl_out_of_range` follow. *Flagged to the human:* the digit argument of
`continuation_symbol` and `predecessor_symbol` is defined for 0–2 (payload §3); "< 4, then masked & 3" leaves a digit of
3, for which the tables have no row. Ruled by R-324: the digit is `debug_assert!`-ed < 3, then clamped with `min(d, 2)`.

## R-322 — R-318's integers rule is per field: u64 fields are always strings *(amends R-318)*
*1 Oct 2026 · applied in gui_state_contract §2, REQ-TOOL-145 and TASK-M0-18 (PR #100)*

"R-318's integers rule is per field (R-322): u64 fields (e.g. seeds) always serialise as strings; all other numbers as
numbers. A field's type never depends on its value."

*Applied:* in the canonical serialisation (JCS, R-318), a field of type u64 (a seed, say) is always written as a JSON
string of its decimal digits, whatever its value; every other numeric field is written as a JSON number in JCS's
format. R-318's per-value reading (number up to 2^53, string above) is replaced. gui_state_contract §2, REQ-TOOL-145
and TASK-M0-18's `canonical_jcs` line follow (a u64 field of 0 and of 2^53 + 1 both write strings; another integer
field writes a number).

## R-323 — #100's physics findings accepted: the diff threshold is exact; no frames exits 2; a cut-off trace says so
*Amended by R-328.*
*Still in force: the exact threshold; "session incomplete" with the dropped bytes, still compared; R-328 extends "no
frame records exits 2" from NEW to either file.*
*1 Oct 2026 · applied in REQ-TOOL-119 and TASK-M0-18 (PR #100)*

"#100's physics findings accepted: the regression threshold compares exactly (100 → 107 at --threshold 7% is not a
regression; add that test); a NEW trace with no frames exits 2; a cut-off trace prints "session incomplete" with the
dropped bytes (as R-298)."

*Applied:* REQ-TOOL-119's definition, which TASK-M0-18 writes into render_gui_spec § "Profiler" (PR #100; not yet on
`main`), gains three rules. (1) A scope regresses when its NEW p95 exceeds its BASE p95 by more than P%, compared
exactly, not by rounded floating-point arithmetic: 100 → 107 at `--threshold 7%` is not a regression, and that case is
tested. (2) A NEW trace with no frame records exits 2, as an unreadable file does. (3) A cut-off or incomplete trace
prints "session incomplete" and the bytes dropped. The ruling cites R-298, which makes a trace with no summary line
valid; the dropped-bytes count is R-299's (the reader drops a cut-off final line and says how many bytes it dropped),
so both apply. *Confirmed by the human* (a later message, 1 Oct 2026): "What matters is the behaviour, and that's unchanged: a
cut-off trace is reported as "session incomplete" with its dropped bytes, never silently compared over fewer frames."
So an incomplete NEW is compared and sets the exit code as usual, always with the "session incomplete" notice and the
dropped bytes, never silently. On the citation, the human wrote: "Right, thanks for catching it. Apply R-299." R-298
gives the incomplete session; R-299 gives the dropped bytes. REQ-TOOL-119's
verify detail and TASK-M0-18's `profile_diff` line follow; physics still approves the written definition (R-72).

## R-324 — The digit argument of `continuation_symbol` and `predecessor_symbol` is debug-asserted < 3, then clamped with `min(d, 2)` *(completes R-321)*
*1 Oct 2026 · applied in payload §3, REQ-PAY-016, TASK-M0-11 (PR #99) and TASK-M0-13*

"Digit 3 in continuation_symbol and predecessor_symbol (R-324, completing R-321): the digit is debug_assert!-ed to be
< 3, then clamped with min(d, 2), in Rust and WGSL. Total and branch-free, no new table row. The debug assertion is the
protection; the clamp only keeps release builds well-defined."

*Applied:* in Rust and WGSL, `continuation_symbol` and `predecessor_symbol` read their digit argument as `min(d, 2)`;
a Rust debug build given a digit ≥ 3 fails a `debug_assert!` first (WGSL has none). A release build, and WGSL, given a
digit ≥ 3 read the digit-2 row. Their symbol argument keeps R-321's rule (`debug_assert!` < 4, then `& 3`). No table
row is added. Payload §3, REQ-PAY-016, TASK-M0-11's `continuation_table_out_of_range` and TASK-M0-13's
`continuation_table_wgsl_out_of_range` follow.

## R-325 — CI: the GPU kernel build and its tests run in their own parallel job; ≤ ~10.5 min per job *(amends R-301)*
*1 Oct 2026 · applied in REQ-SYS-073, REQ-SYS-076 (new) and TASK-M0-14 (PR #96)*

"CI: move the GPU kernel build and its tests into their own parallel job; target ≤ ~10.5 min wall-clock per job
(R-270)."

*Applied:* `cargo xtask build-kernel` and the tests that need the built kernel leave the `ci` job for a job of their
own, which runs in parallel with it. R-270's ~10.5 min is the target again, now for each CI job's warm wall-clock
time, not for `ci` alone. R-301's measurement stands (about 10m28.5s per warm `ci` run on PR #85), but its "the
~10.5 min target gives way" no longer does. PR #96 measured the warm `ci` job at 17 min 30 s with the kernel build in
it, against `main`'s 12 min 51 s. TASK-M0-14 (PR #96) makes the split, and its PR shows every job's warm wall time
against ~10.5 min. A job still over ~10.5 min after the split is reported to the human, not accepted silently. A new
job is not a required status check until the human adds it to branch protection (R-266, HUMAN_SETUP §2); the PR says
so.

## R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only *(amends R-285, R-320)*
*Amended by R-337.*
*Still in force: every cache step restores in every run and saves only in a run on a push to `main`; R-337 makes a
workflow that never runs on a push to `main` restore, read-only, a key a `ci.yml` job saves there.*
*1 Oct 2026 · applied in REQ-SYS-073, REQ-SYS-075 and TASK-M0-14 (PR #96)*

"Actions cache: caches are saved only on pushes to main; PR jobs restore only. The rust-gpu cache keeps its own key."

*Applied:* every cache step in every workflow restores in every run, and saves only in a run triggered by a push to
`main`. A pull-request run, and a push to any other branch, restores and never saves. GitHub scopes caches by branch,
and a pull-request run can restore what `main` saved. The rust-gpu cache keeps its own key, `rust-gpu-<job>-<os>-<channel>`
(R-320). This answers PR #96's measurement that the repository's Actions cache held 11.13 GiB, over GitHub's 10 GB
limit, most of it fixture-pool entries of about 1 GiB saved per branch. REQ-SYS-073 and REQ-SYS-075 follow.
*Applied per R-204 — veto?:* the rule reaches every workflow, including the cache steps TASK-M0-42 wrote, and
TASK-M0-14 (PR #96), which already edits the workflows' cache steps, makes the change in all of them; REQ-SYS-073
stays closed by TASK-M0-42.
*Veto not exercised, 1 Oct 2026:* "The veto marks on R-326, R-329 and R-332 stand." A note, not a ruling.

## R-327 — The profiler header's frame count is u32, so it is a JSON number *(applies R-322)*
*1 Oct 2026 · applied in telemetry §5, REQ-TOOL-002 and TASK-M0-18 (PR #100)*

"#100 item 11: the frame count is typed u32, so it serialises as a number under R-322."

*Applied:* the frame count, `prin profile --frames N` and the header's `config.frames`, is typed u32. R-322 makes
only a u64 field a string, so `frames` is a JSON number. PR #100 typed it u64 and wrote it as a number (its veto item
11); its fix pass types it u32. Telemetry §5, REQ-TOOL-002 and TASK-M0-18 follow.

## R-328 — `prin profile diff` given a BASE or a NEW with no frame records exits 2 *(amends R-323)*
*1 Oct 2026 · applied in REQ-TOOL-119 and TASK-M0-18 (PR #100)*

"#100 item 13 stands: empty BASE or NEW exits 2. Widen the plan text to "either file"."

*Applied:* R-323's "a NEW trace with no frame records exits 2" now covers either file. A BASE with no frame records
can't be compared either, and a gate that passed on an empty BASE could not fail after a crashed baseline run. PR #100
applied it for BASE per R-204, following the physics reviewer's finding 2, and this ruling confirms it.
REQ-TOOL-119's verify detail and TASK-M0-18's `profile_diff` and definition lines read "either file"; render_gui_spec
§ "Profiler" cites R-328 for it.

## R-329 — The header's core count is `cpu_cores_available`; no `usize` in a serialised type
*1 Oct 2026 · applied in telemetry §5, gui_state_contract §2, REQ-TOOL-002, REQ-TOOL-145 and TASK-M0-18 (PR #100)*

"#100 item 5: the field is named cpu_cores_available (cores this process may use); add the machine's total if cheap.
Item 14: no usize in serialised types; use explicit u32/u64."

*Applied:* the session header's `device.cpu_cores` is renamed `device.cpu_cores_available`: the number of cores this
process may use (`std::thread::available_parallelism`), a count of at most 2^32 − 1. *Applied per R-204 — veto?:* the
machine's total is a second key, `device.cpu_cores_total`, a count of at most 2^32 − 1, or `null` where the platform
doesn't report it cheaply. A key that is sometimes missing would break telemetry §5's rule that every key is present,
an absent value `null`. No serialised type has a `usize` field: a count or size is an explicit u32 or u64, so its
width, and under R-322 its JSON type, never depends on the platform. PR #100's fix pass renames the key in the typed
form, the JSON Schema and the tests (TASK-M0-17's files, as R-308's change was). Telemetry §5, gui_state_contract §2,
REQ-TOOL-002, REQ-TOOL-145 and TASK-M0-18 follow.
*Veto not exercised, 1 Oct 2026:* "The veto marks on R-326, R-329 and R-332 stand." A note, not a ruling.

## R-330 — #99's veto item 5 stands: §3.7's "f16 × 2" is `escape_time_min` and `escape_time_max`
*1 Oct 2026 · applied in TASK-M0-11 (PR #99)*

"#99 item 5 accepted (escape_time_min, escape_time_max)."

*Applied:* §3.7's "f16 × 2" stays two named f16 members, `escape_time_min` and `escape_time_max`, as PR #99 wrote it.
PR #99 merged on this ruling. Changes no requirement.

## R-331 — #96's veto item 11 stands: `ICDescriptor`'s `_pad` keeps 16 B (64 / 112 / 208 B)
*1 Oct 2026 · applied in TASK-M0-14 (PR #96)*

"#96 item 11 accepted (16-byte padding; 64/112/208 B)."

*Applied:* `ICDescriptor`'s `_pad` stays 16 B at every `Real` width, so the descriptor is 64 B at f32, 112 B at f64 and
208 B at DoubleF64 (generation-root §3.6, R-313), as PR #96 wrote it. Changes no requirement.

## R-332 — `QuadReduction` is the sole automatic return of simulation data; the telemetry readback is not simulation data *(closes RQ-173)*
*1 Oct 2026 · applied in the render contract Part 1, systems_architecture §3, REQ-SYS-033, REQ-SYS-036 and TASK-M5-30*

"RQ-173: option 1. REQ-SYS-033 reads "the sole automatic return of simulation data"; telemetry readback (R-288) isn't
simulation data."

*Applied:* REQ-SYS-033's statement reads "the sole automatic GPU-to-CPU return of simulation data", and its verify
detail admits R-288's per-frame telemetry readback, which is not simulation data. The render contract Part 1 says the
same. TASK-M5-30's Goal and its REQ-SYS-033 checklist line follow. *Applied per R-332 — veto?:* REQ-SYS-036's audit
("finds only the reduction readback and the two sanctioned pulls") read the same way as REQ-SYS-033's, so it also
admits the telemetry readback, and its statement names the `QuadReduction` "the sole automatic GPU→CPU return of
simulation data"; systems_architecture §3's membrane table, REQ-SYS-036's source, says the same, and TASK-M5-30's
REQ-SYS-036 checklist line follows. A veto reverts all three together. RQ-173 moves to
`docs/archive/review_queue/M0.md`.
*Veto not exercised, 1 Oct 2026:* "The veto marks on R-326, R-329 and R-332 stand." A note, not a ruling.

## R-333 — The `qa_TASK-M0-06_edges` flake: the test and its control get separate scratch folders
*1 Oct 2026 · applied in a follow-up PR on TASK-M0-06*

"The qa_TASK-M0-06_edges flake: a small fix giving the test and its control separate scratch folders."

*Applied:* `qa_m006_symptom_each_channel_mean` in `xtask/tests/qa_TASK-M0-06_edges.rs` and its negative control each
get a scratch folder of their own. The test failed once with "copied: NotFound" (seen on PR #96) and passed on three
re-runs: the two shared a folder. The file is qa's, so qa makes the change (R-290), in a follow-up PR on TASK-M0-06,
as R-217's was on TASK-M0-26. Changes no requirement.

## R-334 — check_plan.py proves every still-open item in `open-questions.md` maps to a requirement or a REVIEW_QUEUE entry
*1 Oct 2026 · applied in `open-questions.md` and `plan/check_plan.py` (a PR of its own)*

"open-questions.md: make check_plan.py prove every still-open item in it maps to a requirement or a REVIEW_QUEUE
entry (ionisation gate, δ_dep and the departed bit, the change-10 re-runs, alpha_area, camera priority, the Burrau
quotient, the Yoshida-6/OKLab checks). Anything unmapped is filed, not left as prose."

*Applied:* each still-open item in `open-questions.md` names the requirement or the open REVIEW_QUEUE entry that
carries it. An item that has neither is filed first, as a requirement through `plan/tools/reqio.py` or as a
REVIEW_QUEUE entry, and then named. `plan/check_plan.py` fails when a still-open item names neither, or names a
requirement that doesn't exist or is retired, or a REVIEW_QUEUE entry that isn't open. The items the ruling lists: the
ionisation gate, `δ_dep` and the per-sample departed bit, the change-10 re-runs, the two `alpha_area` defects, the
camera not wired into priority, the Burrau quotient, and the Yoshida-6 and OKLab transcription checks. It lands in a
PR of its own, after this one.

## R-335 — qa may narrow `qa_TASK-M0-22_r235.rs`'s `if:` check to the job's own `if:` *(closes RQ-186; amends R-290)*
*1 Oct 2026 · applied in TASK-M0-14 (PR #96)*

"1 yes." (RQ-186's recommended option 1.)

*Applied:* R-326's cache save in the `xtask-ci` job is a step under a step-level `if:`, and
`check_controls_job_beside_the_tests` in `xtask/tests/qa_TASK-M0-22_r235.rs` (line 115) rejects an `if:` at any indent
in that job. qa may change that one check so that its `if:` test reads only the job's own `if:` (indent 4), not a
step's; its `continue-on-error:` test and the rest of the file stay as they are, and the code reviewer confirms that
nothing else changed. This is a named exception to R-290 for this file, whose history has implementer commits
(9405734, 3b84aa5), on R-290's own ground: a ruling, R-326, changed the behaviour it tests. qa makes the change in
PR #96's qa commit, and the orchestrator's R-237 check accepts `M` on this file in that commit. REQ-VAL-166 is
unchanged: the controls job still cannot be skipped or pass with a finding. RQ-186, filed on PR #96's branch, is
archived unchanged in `docs/archive/review_queue/M0.md` with this ruling's port, so that its id resolves on `main`;
#96's next fix pass deletes its open copy from `REVIEW_QUEUE.md`. Changes no requirement.

## R-336 — #96's CI overrun is accepted; TASK-M0-45 shards nextest and splits the long single tests *(amends R-270, R-290)*
*1 Oct 2026 · applied in REQ-SYS-073, REQ-SYS-076, REQ-SYS-077 (new), TASK-M0-14 (PR #96) and TASK-M0-45 (new)*

"2 B: accept the overrun on #96, and make the sharding the next M0 task at high priority. It should also split the
long single tests (the 257 s one, and the M0-24/25/26 suites), since sharding can't parallelise inside one test."

*Applied:* PR #96's warm wall times, the `ci` job at about 16.5 min (18.7 cold) and `xtask-ci` at 11.7–13.3 min
(19.5 cold), over R-270's ~10.5 min per job (R-325), are accepted for PR #96: its wall-time checks (REQ-SYS-073,
REQ-SYS-076) are met by this ruling, not by a run under ~10.5 min. The target stays ~10.5 min for each CI job's warm
wall-clock time. A new task, TASK-M0-45, depending on TASK-M0-14 and the next M0 task to start, at high priority,
brings each job back under it:
- it shards the nextest runs of the `ci` and `xtask-ci` jobs across parallel jobs, the shards together running every
  test the unsharded run did;
- it splits the long single tests that bound a job, since a shard cannot run one test in parallel:
  `qa_cargo_xtask_alias_runs_deps` in `xtask/tests/qa_TASK-M0-01.rs` (257 s on PR #96), and the `qa_TASK-M0-24`,
  `qa_TASK-M0-25` and `qa_TASK-M0-26` suites in `crates/validation/tests/` (about 100 s each).

REQ-SYS-077 (new) carries it, and qa's finding on PR #96 against REQ-SYS-073 is answered by it. A new job is not a
required status check until the human adds it to branch protection (R-266); TASK-M0-45's PR names the jobs to add.
*Applied per R-204, accepted by R-344:* the files holding those tests are qa's, and each has implementer commits, so
R-290 bars both qa and the implementer from them. qa makes the splits, in TASK-M0-45's qa commit, as a named exception
to R-290 for those files, as R-335 is for `qa_TASK-M0-22_r235.rs`; a split moves tests and their controls without
changing an assertion, and the code reviewer confirms that none is weakened. The orchestrator's R-237 check accepts `M`
on those files, and `A` for the files split from them, in that commit.

## R-337 — Workflows that run only on pull requests restore, read-only, the caches a `ci.yml` job saves on `main` *(amends R-326)*
*1 Oct 2026 · applied in REQ-SYS-073, REQ-SYS-075 and TASK-M0-14 (PR #96)*

"3 a."

*Applied:* `mutants.yml`, `pr-check.yml`, `reviews.yml`, `screenshot.yml` and `stand-in-soak.yml` never run on a push
to `main`, so under R-326 they never save a cache, and a restore under a key of their own finds nothing. Each restores,
read-only, the key a `ci.yml` job saves on `main` for what it needs: the cargo registry under the key a `ci.yml` job
saves it under, and, in a job that builds the kernel, the rust-gpu build under `gpu-kernel`'s key
(`rust-gpu-gpu-kernel-<os>-<channel>`, R-320). Each of `mutants.yml`'s 8 shards built the rust-gpu backend cold on
PR #96, about 4 min each. None of them saves. A cache that is saved still names the job that saves it (R-285); a
restore-only step names the saving job's key, as `xtask-ci`'s restore of `ci`'s fixture pool does. The comments PR #96
wrote at `pr-check.yml:28`, `reviews.yml:39`, `screenshot.yml:34` and `stand-in-soak.yml:42`, "it restores what `main`
saved", were false of a key nothing saves; they are corrected to name the `ci.yml` key each step restores. TASK-M0-14
(PR #96), which makes R-326's change in every workflow, makes this one. REQ-SYS-073 and REQ-SYS-075 follow.

## R-338 — Parked is not open: `open-questions.md`'s audit section D needs no mapping *(closes RQ-187)*
*1 Oct 2026 · applied in `open-questions.md` and `plan/check_plan.py` (PR #106), and in `open-questions.md` and `REVIEW_QUEUE.md` (PR #108)*

"4 yes. 5 ok."

*Applied:* the items of `open-questions.md`'s "Audit section D — deliberately parked" are decisions to wait, each
parked with its reason in its owning file (philosophy §7.7). They are not still-open items under R-334: section D is
marked parked, carries no *Carried by* note, and `plan/check_plan.py`'s R-334 check does not read it. PR #106 (R-334's
PR) merged with section D still naming RQ-187 and RQ-187 still open, so PR #108 applies it: section D is marked
"Parked, not open (R-338)", with no *Carried by* note, `open-questions.md`'s opening note says a parked item names no
carrier, and RQ-187's open copy is deleted from `REVIEW_QUEUE.md`. RQ-187, filed on #106's branch, is archived unchanged
in `docs/archive/review_queue/M0.md` with this ruling's port, so that its id resolves on `main`.
Noted with it, not a ruling of its own ("5 ok"): PR #106's veto item, which marks `open-questions.md`'s corpus-defects
section, D1–D6, "Closed: fixed by R-59" (applied per R-334), stands; the human did not veto it. Changes no requirement.

## R-339 — dd_integrator §3.6's terminal-priority pin is superseded by R-30's time ordering
*1 Oct 2026 · applied in dd_integrator §3.6, §5 test 6, §6 and its closing line, and REQ-EVT-020*

"6 yes: R-30's time ordering wins, so mark the block superseded."

*Applied:* `docs/design/principia_dd_integrator.md` §3.6's block "Terminal taxonomy & priority (pinned here —
confirm/veto)" ordered SIM_FAILED > COLLISION > ESCAPE > BOUNDED within one `STEP`: the markdown's pin that R-6 kept
until the order was decided. R-30 decided it by time (REQ-EVT-020): sim_failed first; then whichever event came
first, a same-time tie going to collision; triple ejection as a detail of escape; then running; then bounded. The
block stays, marked "superseded by R-30, kept for the record", as change 11's escape gates above it are, and so does
the paragraph after it that argues for the pin. What R-30 leaves alone still stands: labels are mutually exclusive,
there is no timeout state, `MAX_SUBSTEPS` is not a terminal condition, and the order is deterministic, identical on
CPU and GPU and in the shared physics source. §5 test 6's "per §3.6 priority" and the closing line's "Collision beats
escape" cite R-30's same-time tie, and §6's "Priority-order pin" entry is struck through and marked settled by R-30,
as R-14's entry there is. REQ-EVT-020 cites this ruling.

## R-340 — The schema version hashes each link registry entry's semantic content, not its prose *(applies R-251)*
*Amended by R-344.*
*Still in force: the link registry is hashed into the schema version, each entry by its semantic content (its name,
constraint, forward, inverse, log-det, ε clamps and the parameters its functions read, by value), not its sampling
note, and every entry, whether or not a block uses it by default; R-344 hashes the registry's chart constants that no
link reads (`δ_λ`, `ε_w`) too, by value, and accepts the name and every-entry items.*
*1 Oct 2026 · applied in generation-root §3.8 and §3.9, REQ-GEN-031 and REQ-GEN-032 (new), TASK-M0-46 (new) and
TASK-M2-01*

"7 hash it, without waiting for an RQ: the link registry's definitions determine how chart coordinates decode, so
changing them changes what cached and saved results mean. That's exactly what the schema version is for. As with
R-251, hash each link's semantic content, not its prose."

*Applied:* TASK-M0-12 (merged) computes `PAYLOAD_SCHEMA_VERSION` over the layout entries and words, the continuation
table, the code assignments, `QuadReduction`'s member list and the register entries that decide stored bits, but not
the link registry (generation-root §3.9), though §2 names "the canonicalised §3 ledger". The registry joins the hashed
ledger (R-36). §3.9 says what an entry is: "Each entry ships **forward, inverse, log-det, ε clamps, and the sampling
note**", under the constraint its row names. The semantic content hashed is:
- the entry's constraint, the block codomain it maps onto (§3.9's first column);
- its forward, inverse and log-det;
- its ε clamps, and each parameter its forward, inverse or log-det reads (`μ_max` in the simplex link, R-10), by value.

Its sampling note is prose: §3.9 gives it to "the robustness-sweep picker", and no decode or encode reads it, so it is
not hashed, as R-251 leaves out a constant's citation. Changing a hashed member of any entry, or adding or removing an
entry, changes the schema version; a sampling-note-only edit does not. *Applied per R-204, accepted by R-344:* the
entry's name, the link id provenance records (chart contract § "Integrity: the link is part of the experiment"), is
hashed too, as TASK-M0-12 hashes each layout entry's and register constant's name; and every entry is hashed, whether or
not a block uses it by default, as the human's words are "each link's". How a function is written in the hashed bytes, a
canonical form with no formatting-dependent bytes, is a definition TASK-M0-46 writes into §3.9 (R-72, REQ-GEN-032),
approved by the physics reviewer. A new task, TASK-M0-46, depending on TASK-M0-12, builds it in `crates/ledger`, with
the physics reviewer as for TASK-M0-12 (R-251), and tests it over test entries; the registry's own entries are
TASK-M2-01's, hashed as they land. `PAYLOAD_SCHEMA_VERSION`'s value changes when TASK-M0-46 lands, since the canonical
serialisation gains the registry's table, and again when TASK-M2-01 adds its entries; under R-36 that is the intent.
Filed: RQ-189, whether the registry's chart constants that no link entry reads (`δ_λ`, `ε_w`) are hashed too.
TASK-M0-46 waited for it; R-344 closes it: they are hashed.

## R-341 — `prin profile` streams its trace: the header first, each frame as it completes, flushed every 60 frames or 1 s *(amends R-286, R-298)*
*1 Oct 2026 · applied in telemetry §5, REQ-TOOL-147 (new) and TASK-M0-47 (new)*

"R-341: prin profile streams its trace. The header is written first, then each frame record as it completes, flushed at
least every N frames (or every second) so a crash loses at most that much; leak_flags and hot_paths are appended as the
final line at session end (R-286, R-298). Memory use must not grow with --frames beyond what the summaries themselves
need. Add a test that kills a run mid-session and reads the partial trace as "session incomplete"."

"R-341's flush: every 60 frames or 1 s, whichever comes first."

*Applied:* TASK-M0-18 (merged, PR #100) has `prin profile` hold every frame record of the run in memory and write the
whole trace when the run ends, so a run that dies writes nothing and its memory grows with `--frames`. Telemetry §5
said only that a writer "can stream the frames". Now `prin profile` writes the header line first, then each frame
record as the frame completes, and flushes the file at least every 60 frames or every 1 s, whichever comes first, so a
run that crashes or is killed loses at most the frames since the last flush. The summary line, `leak_flags` and
`hot_paths`, is appended as the final line when the session ends (R-286, R-298). The run's memory does not grow with
`--frames` beyond what the summaries themselves need: it keeps no frame record once the record is written. A trace
left by a killed run is the incomplete session R-298 and R-299 define, and the reader reports it "session incomplete".
Telemetry §5 says so. REQ-TOOL-147 (new, M0) carries it, closed by a new task, TASK-M0-47, depending on TASK-M0-18,
with the code, qa and perf reviewers, as TASK-M0-18 has, less physics, since no definition changes. Its test kills a
run mid-session and reads the partial trace as "session incomplete". *Applied per R-204, accepted by R-346:* the header
line is flushed as soon as it is written, so a run killed before its first frame flush still leaves a trace R-298
accepts (a header line alone), and a crash loses at most the frames, never the header; the memory bound is checked by
the perf reviewer against the run's peak memory at two frame counts, which the PR reports, since the corpus gives no
number for it.
*Veto not exercised, 1 Oct 2026:* "The five R-204 items on R-341–R-343 stand." (R-346 (C)): the header line flushed as
soon as it is written; the memory bound judged by the perf reviewer at two frame counts. A note, not a ruling.

## R-342 — Tests delete their scratch folders on success and keep them only on failure *(amends R-290)*
*1 Oct 2026 · applied in REQ-VAL-178 (new) and TASK-M0-48 (new)*

"File the M0-06 scratch-folder accumulation as a small task: tests delete their scratch folders on success and keep
them only on failure."

*Applied:* R-333 gave `xtask/tests/qa_TASK-M0-06_edges.rs`'s `scratch_dir` a folder per call, named by the process id
and a per-process counter (qa's commit dc3afb4). No later run reuses that name, and nothing deletes the folder, so the
folders pile up under `CARGO_TARGET_TMPDIR` run after run. A test's scratch folder, or scratch file, is deleted when the
test passes and kept, with its path in the failure output, when the test fails. REQ-VAL-178 (new, M0) carries it, closed
by a new task, TASK-M0-48, depending on TASK-M0-06, with the code and qa reviewers. *Applied per R-204, accepted by
R-346:* the task covers each helper that makes a fresh, uniquely named scratch folder or file per call (by process id,
counter or time), which a later run never reuses: a search of 1 Oct 2026 found them in `xtask/tests/`
(`qa_TASK-M0-06_edges`, `qa_TASK-M0-23_r305`, `qa_TASK-M0-23_shards`, `qa_TASK-M0-38`, `qa_TASK-M0-40`,
`mutants_no_mutant`), `crates/prin/tests/` (`profile`, `qa_TASK-M0-18`, `qa_TASK-M0-18_base`, `qa_TASK-M0-18_rulings`),
`crates/prin/src/profile/diff.rs`'s unit tests, and `crates/validation/tests/` (`qa_TASK-M0-38`, `qa_TASK-M0-39`,
`stand_in`). A helper with a fixed name reuses and replaces its folder on the next run, so it does not accumulate and is
left as it is. qa makes the edits to qa's files in TASK-M0-48's qa commit: R-290 allows it on the files only qa has
committed to, and for the two whose history has implementer commits, `xtask/tests/qa_TASK-M0-38.rs` (55815e6, 8c22eec)
and `crates/validation/tests/qa_TASK-M0-38.rs` (f7becfc), this is a named exception to R-290, as R-336's is; the change
alters only where scratch is made and when it is deleted, never an assertion, and the code reviewer confirms that. The
implementer makes the edits to the rest.
*Veto not exercised, 1 Oct 2026:* "The five R-204 items on R-341–R-343 stand." (R-346 (C)): the scope limited to the
helpers that make a uniquely named scratch folder or file per call; the named R-290 exception for the two
`qa_TASK-M0-38.rs` files. A note, not a ruling.

## R-343 — The fragment unpack layer binds `SimStateFTLE` at `@group(1) @binding(0)` and the word buffer at `@group(1) @binding(1)`; WGSL forms of `closure_step` and the schema version *(closes RQ-188)*
*1 Oct 2026 · applied in render contract Part 5 "Unpack layer", lowering contract Part 3a, payload §1, REQ-RENDER-001,
REQ-PAY-091 and TASK-M0-13 (PR #107)*

"RQ-188 (R-343): 1b, not 1a: SimStateFTLE at @group(1) @binding(0) and the word buffer at @group(1) @binding(1),
leaving group 0 for the assembler's per-frame uniforms. Emit the group/binding numbers as generated constants from one
table. Reading only through sample_state()/sample_word() stands. 2a and 3a accepted. Items 4–6 accepted; for item 5,
note in the docs that unset-checks on values read in fragment shaders test bit patterns, never isinf/isnan, because
fast-math (R-297) may optimise those away."

*Applied:* RQ-188's options 1b, 2a and 3a, and PR #107's three items applied per R-204, which are accepted and lose
their "veto?" marks.
- **Bindings (1b).** The generated fragment-side unpack layer declares the full tier's stored buffers:
  `@group(1) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;` and
  `@group(1) @binding(1) var<storage, read> word_buffer: array<vec4<u32>>;`. Group 0 is left to the assembler's
  per-frame uniforms. The group and binding numbers are generated constants from one ledger table, the one source of
  the numbers the WGSL attributes carry. Each buffer is read only through a generated function of the sample index,
  `sample_state(i)` and `sample_word(i)`, the same `i` for both (per copy); nothing else indexes either buffer. M1's
  per-tier assembly (lowering Part 3a) owns the feature-off variants (`SimStateBase`, no word binding).
- **`closure_step` (2a).** WGSL has no u16 and `enable f16` is banned (R-86), so the u16 `closure_step` and u16
  `_reserved` at byte 140 are one WGSL member, `closure_step_reserved: u32`: `closure_step` in bits 0–15, `_reserved`
  in bits 16–31, read through `fn closure_step(w: u32) -> u32 { return extractBits(w, 0u, 16u); }`. The stored layout,
  and the Rust `closure_step: u16` and `_reserved: u16`, are unchanged.
- **Schema version (3a).** WGSL has no u64, so the generated WGSL carries `const PAYLOAD_SCHEMA_VERSION: vec2<u32>`,
  `.x` the low 32 bits of the Rust `u64` and `.y` the high 32, the order `unpack2x16float` uses.
- **Item 4, accepted:** no WGSL setter is emitted; the fragment side only reads.
- **Item 5, accepted:** the generated WGSL emits `PA_D_MIN_UNSET = 0x7c00u` and `pa_d_min_is_unset`, so a shader tests
  `d_min`'s unset value by its bits (R-271). With it, as the human asks, the render contract's WGSL traps say that an
  unset-check on a value read in a fragment shader tests the bit pattern, never `isinf` or `isnan` (or a float
  comparison standing in for them), because fast-math (R-297) may optimise those away.
- **Item 6, accepted:** `fgw_reduced_length_valid` and `fgw_reduced_length` are emitted; `ftle_valid` and the tier
  interface (`has_<feature>`, lowering Part 3a) are left to M1.

The render contract's "Unpack layer" and its WGSL traps say all of this; lowering Part 3a gains a note on the
bindings, and payload §1's WGSL read view a note on `closure_step`'s WGSL form, their text kept. REQ-RENDER-001 and
REQ-PAY-091 cite this ruling, and TASK-M0-13 (PR #107) builds it. RQ-188, filed on #107's branch, is archived unchanged
in `docs/archive/review_queue/M0.md` with this ruling's port, so its id resolves on `main`; #107's fix pass deletes its
open copy and drops REQ-RENDER-001's and REQ-PAY-091's `rq: RQ-188`.
*Applied per R-204, accepted by R-346:* the table's constants are named `SIMSTATE_GROUP`, `SIMSTATE_BINDING`,
`WORD_GROUP` and `WORD_BINDING`, and are emitted to both targets, the WGSL (whose `@group`/`@binding` attributes carry
the same numbers) and the generated Rust, which the host's bind group layout reads (generation-root §1: one source, two
targets). They are not hashed into the schema version: a binding number decides no stored bit's meaning (generation-root
§3.8 "The hash").
*Veto not exercised, 1 Oct 2026:* "The five R-204 items on R-341–R-343 stand." (R-346 (C)): the binding constants'
names, `SIMSTATE_GROUP`, `SIMSTATE_BINDING`, `WORD_GROUP` and `WORD_BINDING`; their emission to the WGSL and the
generated Rust; and their not being hashed. A note, not a ruling.

## R-344 — `δ_λ` and `ε_w` are hashed; #108's four "veto?" items are accepted *(closes RQ-189; amends R-340)*
*1 Oct 2026 · applied in generation-root §3.9, REQ-GEN-031, TASK-M0-46, TASK-M0-45, TASK-M2-01 and R-336's and R-340's
notes*

"#108: RQ-189 yes (δ_λ and ε_w are hashed: they change how coordinates decode). qa making R-336's test splits under a
named R-290 exception: accepted. R-340 hashing every link and its name: accepted. TASK-M2-01 depending on M0-46:
accepted."

*Applied:* RQ-189's option 1. The registry's chart constants that no link reads, `δ_λ` (decode's mirror tie-break)
and `ε_w` (its seed-selection floor), are hashed into the schema version by value, as the links' own parameters are:
each changes how a chart coordinate decodes, R-340's reason. Every chart constant the registry holds is hashed, whether
or not a link reads it. Generation-root §3.9's "The hash", REQ-GEN-031 and TASK-M0-46 say so; TASK-M0-46 hashes the
registry's constants table with its entries and no longer waits. RQ-189 moves to `docs/archive/review_queue/M0.md`.
PR #108's four items applied per R-204 are accepted, and their "veto?" marks are dropped: qa makes R-336's test splits
in TASK-M0-45's qa commit, a named exception to R-290; R-340 hashes each entry's name; it hashes every entry, whether
or not a block uses it by default; and TASK-M2-01 depends on TASK-M0-46 and needs REQ-GEN-031.

## R-345 — Merged branches are deleted, with their worktrees and target directories
*1 Oct 2026 · applied in `plan/WORKFLOW.md` § "The review loop" and CLAUDE.md § Git*

"add that to be part of the workflow, merged branches are deleted."

*Applied:* said as every merged branch had just been pruned, remote and local. After a PR merges, its remote branch
and its local branch are deleted, its worktrees and their target directories are removed, and stale remote refs and
worktree entries are pruned (`git fetch --prune`, `git worktree prune`). The rule covers only branches fully merged
into `main`. A branch that is the base of an open PR is not deleted until that child PR is retargeted to `main`, since
deleting a base branch closes its child PRs (CLAUDE.md § Git, "Merging stacked PRs"). `plan/WORKFLOW.md`'s merge step
and CLAUDE.md § Git say so. Process only (section_notes); no requirement changes.

## R-346 — The orchestrator's manual is `plan/OPERATIONS.md`; cloud sessions start with `scripts/cloud-setup.sh`, which reads every pin from CI's files
*Amended by R-347.*
*Still in force: all of it, except how the script installs cargo-nextest and cargo-mutants: R-347 downloads them
prebuilt, with `cargo install --locked` only as the fallback. Its items applied per R-204 are ruled by R-347.*
*1 Oct 2026 · applied in `plan/OPERATIONS.md`, `scripts/cloud-setup.sh`, `xtask/tests/cloud_setup.rs`, CLAUDE.md §
"How work runs", `plan/WORKFLOW.md`, RQ-190 (closed by R-347), and R-341–R-343's notes*

(A) "Before I move work to cloud sessions: write everything your local memory holds that a fresh session needs into
the repo (CLAUDE.md, plan/WORKFLOW.md, or a new plan/OPERATIONS.md): the overnight rules, merge order, dispatch rule
(R-289), size practice, disk/memory limits, how to run reviewers, and anything else you'd tell a new orchestrator. Mark
which parts are Mac-specific (SSD paths, APFS clones, env.sh, memory_pressure) and what a Linux cloud machine should do
instead. One PR. Then finish the work in flight and stop at a clean point: nothing half-applied, every open question
recorded in the repo."

(B) "Also add scripts/cloud-setup.sh: installs exactly what CI's Linux jobs install (Rust stable, the pinned rust-gpu
nightly, Mesa lavapipe, cargo-nextest, cargo-mutants, Python + PyYAML), sets PRIN_GPU_BACKEND=vulkan, and runs
check_plan.py as a smoke test. Document in plan/OPERATIONS.md that cloud sessions run it first."

(C) "The five R-204 items on R-341–R-343 stand. One change to the handoff PR: scripts/cloud-setup.sh reads every pin
from the same source CI uses (rust-toolchain file(s), and the workflow files' version pins), never hard-coded copies,
so it can't drift when #96 changes the nightly. Add a check that the script and CI agree."

(D) "This is from me. I'm moving orchestration to Claude Code cloud sessions. Prepare the repo so a fresh session needs
nothing from your local memory:
1. plan/OPERATIONS.md: everything a new orchestrator needs that isn't already in CLAUDE.md, plan/WORKFLOW.md or
CURRENT_RULES.md: the overnight rules, merge order (retarget before deleting), the dispatch rule (R-289), size practice
(R-264), the disk/memory limits, reviewer worktrees (R-219), the measure/ branch rule (R-272), and anything else you'd
tell a new orchestrator. Mark what is Mac-specific (SSD paths, APFS clones, env.sh, memory_pressure, Metal, local perf
runs) and what a Linux cloud machine does instead.
2. scripts/cloud-setup.sh: installs exactly what CI's Linux jobs install (Rust stable, the pinned rust-gpu nightly,
Mesa lavapipe, cargo-nextest, cargo-mutants, Python + PyYAML), sets PRIN_GPU_BACKEND=vulkan, and runs check_plan.py as
a smoke test.
3. CLAUDE.md points to OPERATIONS.md.
Then finish the work in flight and stop at a clean point: nothing half-applied, every open question in REVIEW_QUEUE.
Report what's open."

*Applied:* four messages of 1 Oct 2026, recorded as one process ruling.
- **`plan/OPERATIONS.md` (new)** holds what the orchestrator's local memory and session notes held that a new session
  needs and `CLAUDE.md`, `plan/WORKFLOW.md` and `plan/CURRENT_RULES.md` don't say: start-up, dispatch (R-289), reviewer
  worktrees (R-219) and re-checks (R-229, R-260), the check on qa's commit (R-237, R-290 and its named exceptions),
  merging and merge order (R-266, R-345), away mode (the human's limits of 27 and 28 Sep 2026, R-234), size (R-264),
  asking the human (R-204), resources (R-228, R-252, R-262, R-277), paths and warm builds, Metal and perf (R-186),
  `measure/` branches (R-272), pitfalls and logs. Each rule cites its ruling; a practice with no ruling of its own
  carries its date and stands under this one. Where memory held a fact a later ruling replaced, the ruling's form is
  written: the internal disk, not the SSD (R-262); memory pressure, not swap (R-252); three agents at normal pressure
  and two at warning (R-277); the `measure/` rule as R-272. Mac-only parts are marked, with what a Linux cloud machine
  does instead.
- **`scripts/cloud-setup.sh` (new)** installs what CI's Linux jobs (`runs-on: ubuntu-*`) install, and reads each item
  and version from the files CI reads: the `dtolnay/rust-toolchain@<ref>` steps and their `components:`, the root
  `rust-toolchain.toml` (or `rust-toolchain`) that CI's bare `rustup toolchain install` steps read, the
  `taiki-e/install-action` steps' `tool:` pins, the `apt-get install` and `pip install` lines, the `actions/setup-python`
  steps' `python-version:`, and `PRIN_GPU_BACKEND` from the jobs' `env:`. It holds no version of its own. It refuses to
  run when a Linux job installs by a means it doesn't know, naming the step. It exports `PRIN_GPU_BACKEND`, prints how to
  keep it and cargo's PATH, and runs `python3 plan/check_plan.py`; `--dry-run` prints its plan. Running it again skips
  what is installed; apt runs as root or through sudo.
- **The check (C) asks for** is `xtask/tests/cloud_setup.rs`, which `cargo nextest run --workspace` runs in CI's `ci`
  job, each test with its negative control (R-176). It reads CI's Linux jobs and the root toolchain file itself, and
  fails if the script's dry run names an item CI doesn't install or misses one it does, if any item's version differs,
  if the script's text holds a version literal or one of CI's pins, if the dry run doesn't follow a changed pin, or if
  the script accepts an install step it doesn't know.
- **Pointers:** CLAUDE.md § "How work runs" and `plan/WORKFLOW.md`'s opening point to `plan/OPERATIONS.md` and the
  script. `plan/HUMAN_SETUP.md` covers repository settings, not a machine's setup, and gains none.
- **(C)'s first sentence**, "The five R-204 items on R-341–R-343 stand", is applied once `main` (#108) is merged in:
  R-341, R-342 and R-343 each carry a "*Veto not exercised, 1 Oct 2026:*" note naming their items, and those items'
  "— veto?" marks become "accepted by R-346" in R-341–R-343's notes, R-290's "Still in force" line, telemetry §5,
  TASK-M0-13, TASK-M0-47 and TASK-M0-48. No requirement note carried one.
- **Flagged:** (B) and (D) name "Rust stable" and "the pinned rust-gpu nightly" both. Read from CI's files, as (C) asks,
  `main`'s Linux jobs install stable (`dtolnay/rust-toolchain@stable`) and pin no nightly, and #96 replaces every such
  step with `rustup toolchain install`, which installs the nightly its `rust-toolchain.toml` pins. So the script installs
  stable until #96 merges and the nightly after, and stable then only if a Linux job still asks for it. Checked against
  #96's workflows and toolchain file as well as `main`'s.
- *Applied per R-204, ruled by R-347 (RQ-190):* where CI uses an action, the script does the same with rustup, `cargo
  install --locked <tool>@<version>` (the action downloads a prebuilt binary of that version) and `python3 -m pip
  install`, falling back to `--user` and then `--break-system-packages` where the system Python refuses; it installs
  rustup when the machine has none, and requires `git`, `curl` and `cc`, which CI's runner image has; a `python3` of
  another minor version than CI's warns rather than fails. `plan/OPERATIONS.md`'s Linux readings of memory pressure (PSI
  or `free`, with thresholds), its cap of `nproc` / 4 agents, its disk thresholds (the Mac's), its log for a cloud
  session, and its untested sccache note are recommendations, marked so there.

Process only (section_notes); no requirement changes.

## R-347 — RQ-190's items stand; cargo-nextest and cargo-mutants are downloaded prebuilt, with `cargo install --locked` only as the fallback *(closes RQ-190; amends R-346)*
*1 Oct 2026 · applied in `scripts/cloud-setup.sh`, `xtask/tests/cloud_setup.rs`, `plan/OPERATIONS.md` and R-346's
notes*

"RQ-190: items 2–7 accepted as written. Item 1 accepted with one change: install cargo-nextest from its official
prebuilt installer at the pinned version, and cargo-mutants via cargo-binstall (prebuilt) at the pinned version; fall
back to cargo install --locked only if a download fails."

*Applied:* RQ-190's items 2–7 stand as R-346 applied them: a `python3` of another minor version than CI's warns; the
Linux readings of memory pressure (PSI or `free`, with their thresholds); at most `nproc` / 4 agents; the Mac's disk
thresholds; a cloud session's log in its scratch directory, with its summary as its final message and a PR comment;
and the untested sccache note. `plan/OPERATIONS.md` drops their *recommended* marks and cites this ruling. Item 1
stands, rustup, pip and the required `git`, `curl` and `cc` as written, except for the two cargo tools:
- **cargo-nextest** comes from its official prebuilt installer, `https://get.nexte.st/<version>/<platform>` (`linux` on
  x86_64, `linux-arm` on aarch64), unpacked into `$CARGO_HOME/bin`.
- **cargo-mutants** comes through `cargo binstall --no-confirm --disable-strategies compile cargo-mutants@<version>`.
  cargo-binstall, when the machine has none, is installed first from its official prebuilt installer
  (`install-from-binstall-release.sh` in `cargo-bins/cargo-binstall`).
- Each is at the version CI pins, still read from CI's files, never written in the script (R-346 (C)). Each falls
  back to `cargo install --locked <tool>@<version>` only if its download fails: a failed `curl`, a failed
  `cargo binstall`, or a platform get.nexte.st has no build for.
- `scripts/cloud-setup.sh --dry-run` prints each item's install method as a fourth word, the two tools as
  `prebuilt:get.nexte.st,fallback:cargo-install` and `prebuilt:cargo-binstall,fallback:cargo-install`.
- `xtask/tests/cloud_setup.rs` gains a test, with its negative control (R-176): the dry run names those routes, and the
  script's install function, run with `curl`, `tar`, `cargo` and `uname` stubbed, downloads each tool at CI's pin and
  runs `cargo install --locked` only after a failed download. Sourcing the script defines its functions and installs
  nothing, so the test can run that function alone.
- `plan/OPERATIONS.md` § "Start here" says how the two tools are installed, and that a cloud machine needs network
  access to get.nexte.st and GitHub's releases.

*Applied per R-347:* binstall's own build-from-source strategy is disabled, so a failed download falls back to
`cargo install --locked` as the ruling says, and not to binstall's unlocked build. CI pins no cargo-binstall version, so
its installer's current release is used. A cargo tool CI adds later other than these two is built with
`cargo install --locked`, as item 1 had it. RQ-190 moves to `docs/archive/review_queue/M0.md`. Process only
(section_notes); no requirement changes.

## R-348 — Mutants runs get a per-mutant timeout and a per-process memory cap on test processes; both values are calibrated
*Amended by R-352.*
*Still in force: all of it; R-352 accepts its items applied per R-204 and settles how a local run applies the caps: on
macOS a local run gets the per-mutant timeout only, and CI's Linux runners enforce both caps.*
*1 Oct 2026 · applied in REQ-VAL-179, REQ-VAL-180 and REQ-VAL-181 (new), TASK-M0-49 (new) and TASK-M0-19*

The human's message of 1 Oct 2026 numbered its first two rulings R-347 and R-348. R-347 was already taken (it closes
RQ-190), so the message's four rulings are recorded here as R-348 to R-351, in the message's order (R-278). Applied per
R-204, accepted by R-352.

"R-347: mutants runs get two caps: a per-mutant timeout via cargo-mutants' timeout setting (a hang is recorded as a
timeout, not a dead shard), and a per-process memory cap on test processes (ulimit -v or prlimit) so a runaway
allocation kills one test, not the runner. The values are calibration requirements (R-71)."

*Applied:* every `cargo mutants` run, the per-PR shards (`mutants.yml`, REQ-VAL-148, R-302), the nightly full run
(REQ-VAL-150) and a run made locally (`plan/OPERATIONS.md` § "Reviewers"), runs under two caps:
- **A per-mutant timeout,** set through cargo-mutants' timeout setting. A mutant whose tests hang past it is recorded
  in the run's `outcomes.json` as a timeout, and the shard goes on to its next mutant and finishes, rather than hanging
  until R-302's per-shard limit cuts it off. `cargo xtask mutants-check` already lists a timed-out mutant as "timed out
  (not a survivor)" (R-202).
- **A per-process memory cap on the test processes,** set with `ulimit -v` or `prlimit`, so a mutant that allocates
  without bound kills its own test process, not cargo-mutants or the runner. A test the cap kills fails, as any
  failing test does; the unmutated baseline runs under the same cap.
- **The values** are calibration requirements (R-71): REQ-VAL-180, the per-mutant timeout, and REQ-VAL-181, the
  per-process memory cap. The task that needs them proposes each with its evidence, CI uses it provisionally (R-182),
  and the human confirms it at the M0 gate. REQ-VAL-179 (new, M0) carries the caps themselves. A new task, TASK-M0-49,
  closes all three, with the code and qa reviewers.

*Applied per R-204, accepted by R-352:*
- REQ-VAL-179 is a requirement for the caps beside the two calibrations, so that the caps are checked as well as
  valued, as R-342's REQ-VAL-178 carries its rule.
- TASK-M0-49 depends on TASK-M0-14 (PR #96), which rewrites `mutants.yml`'s toolchain, cache and kernel-build steps:
  the caps are written and measured on the workflow as #96 leaves it, and the two PRs don't clash.
- The nightly full run does not exist yet: TASK-M0-19 writes it. TASK-M0-19 now depends on TASK-M0-49, and its
  nightly run applies the same two caps at the same values, read from the one place TASK-M0-49 keeps them.
- **Local runs too.** The human's words say "mutants runs", not CI's alone, so a local run, which
  `plan/OPERATIONS.md` allows when one has to run, uses the same caps at the same values, read from the same place.
  macOS is not known to enforce `ulimit -v` (RLIMIT_AS) and has no `prlimit`, so on the Mac a local run gets the
  timeout only, and CI's Linux runners enforce both caps (R-352); TASK-M0-49's PR says how a local run applies each
  cap.

## R-349 — Agents never delete or modify anything outside the repository and its build and scratch directories without asking first, caches included
*1 Oct 2026 · applied in CLAUDE.md § "How work runs" and `plan/OPERATIONS.md` § "Resources"*

"R-348: agents never delete or modify anything outside the repo and its build/scratch directories without asking
first, caches included. Add it to CLAUDE.md and OPERATIONS.md."

*Applied:* an agent, the orchestrator included, asks the human before it deletes or modifies anything outside the
repository and its build and scratch directories, and caches are no exception. CLAUDE.md § "How work runs" and
`plan/OPERATIONS.md` § "Resources" ("Cleaning disk") say so. Numbered R-349: the human numbered it R-348, which this
message's first ruling now holds (R-348's note).

*Applied per R-204, accepted by R-352:* how the rule is read where the words leave it open.
- **Inside:** the repository's checkouts (the main checkout and every worktree), their target directories (the
  seed's and a mutants run's `<target>-mutants` among them) and the session's scratch directory. Everything else is
  outside: among the caches, `~/.cargo` (its registry, git checkouts and installed tools), `~/.rustup`, the rust-gpu
  cache (`~/.cache/rust-gpu`, `~/Library/Caches/rust-gpu` on the Mac) and the repository's Actions caches on GitHub;
  and the system temp folder outside the scratch directory, the shell's and git's configuration, and the external
  SSD's folders.
- **What `plan/OPERATIONS.md` names as the orchestrator's** is inside wherever it lives: the away-mode log on the Mac
  (`/Users/malachy/principia-ssd/overnight-log.md`, § "Logs"). The SSD's other folders, the human's own, stay
  outside.
- **A build's own cache writes** are part of running the build, not an agent's change: cargo filling its registry,
  rustup installing the toolchain `rust-toolchain.toml` pins, and `cargo xtask build-kernel` building rust-gpu's
  backend in cargo-gpu's cache (R-350). An agent's own deletion or edit of a cache asks first, such as emptying the
  rust-gpu cache for a cold run, as PR #96's cold check did, clearing `~/.cargo`, removing a toolchain, or deleting an
  Actions cache entry.
- **What a build, test or tool writes in its normal course** outside the repository is part of running it, like a
  build's cache writes: the files and folders tests and tools make through `std::env::temp_dir()` (as
  `xtask/tests/deps.rs`, `xtask/src/deps.rs`, `crates/validation/src/spawn.rs` and `crates/prin/src/profile/diff.rs`
  do), and cargo-mutants' temporary copy of the tree. An agent's own deletion or edit there asks first.
- **`scripts/cloud-setup.sh`** installs toolchains, apt and pip packages and cargo tools outside the repository. A
  cloud session runs it as R-346 and R-347 order, which is the asking for what it installs; anything it does beyond
  them asks first.

## R-350 — #96's veto items stand, the exact rust-gpu pin among them, with the backend built from `xtask/rust-gpu-backend.lock`
*1 Oct 2026 · applied in TASK-M0-14 (PR #96)*

"#96's veto items all stand, including the exact rust-gpu pin with the backend built from
xtask/rust-gpu-backend.lock."

*Applied:* every item PR #96 (TASK-M0-14) applied per R-204 and left open stands as the PR wrote it, and loses its
"veto?" mark. Items 1, 2, 6 and 11 of its list were ruled before: R-313, R-316, R-314 and R-331.
- **The list's items 3, 4, 5, 7, 8, 9, 10 and 12:** declared tail padding (`_tail: [u32; n]`, empty at f32) rather
  than any reordering; the GPU entry point named `toolchain_pack_unpack` in WGSL; the mis-built variant is naga's WGSL
  with one field's read offset shifted, asserted to exist exactly once; `Cargo.lock` holds glam at 0.32.1; in
  `cargo xtask ci`, build-kernel runs as its own process through `$CARGO`; in `mutants.yml` the kernel build step is
  the cargo call the alias expands to; payload §1's new block is a bold paragraph, not a `###` heading; `is_real`
  keeps naming the `SimState` structs and `is_generic` names every struct generic over `Real`.
- **The two rust-cache items of its R-337 pass:** `pr-check.yml` and `reviews.yml` set `CARGO_TERM_COLOR: always`, so
  their rust-cache key matches the `ci` job's; and every PR-only job restores the `ci` job's registry key.
- **The exact pin (3e2ba72):** `cargo-gpu-install` and `spirv-std` are required at `=0.10.0-alpha.1`, and
  `cargo xtask build-kernel` builds rust-gpu's backend itself, in cargo-gpu's crate in its cache, from `BACKEND_TOML`
  (`rustc_codegen_spirv` at `=0.10.0-alpha.1`) and `xtask/rust-gpu-backend.lock`, the lockfile of a backend build on
  R-169's pinned nightly-2026-04-11. A cold build can no longer resolve rust-gpu 0.10.0, which needs a later nightly;
  the nightly is not moved.
- **86741e9:** a cached backend is used only if its crate's `Cargo.lock` equals `xtask/rust-gpu-backend.lock` byte for
  byte; any other is rebuilt, and after the install build-kernel refuses one that differs.

PR #96's description marks each of these "Ruled, R-350 (stands)". Numbered R-350: the human's message gave this item
no number (R-348's note). Changes no requirement.

## R-351 — #107's `closure_step_reserved` offsets stand; the bit-pattern unset check becomes a `cargo xtask lint` rule over fragment-stage WGSL
*Amended by R-352.*
*Still in force: all of it; R-352 widens the lint to a float compared with itself and to comparisons against
finite-max stand-ins used as inf checks, and accepts its items applied per R-204.*
*1 Oct 2026 · applied in TASK-M0-13 (PR #107), REQ-PAY-091, REQ-RENDER-083 (new), TASK-M0-50 (new) and the render
contract's "Unpack layer"*

"#107's offset correction stands. The bit-pattern unset check becomes an automated lint as a small task: `cargo xtask
lint` fails on isinf/isnan, or comparisons against inf/NaN constants, in fragment-stage WGSL (R-343, R-297). The
checklist line stays as a backup."

*Applied:*
- **The offsets.** PR #107's correction stands: `closure_step_reserved` is at byte 140 in `SimStateFTLE` and at byte
  92 in `SimStateBase`, which drops the 48 B shadow (payload §1). The 140 that R-343's note and REQ-PAY-091 give is
  `SimStateFTLE`'s. #107's `wgsl_layouts` test asserts both. REQ-PAY-091's verify line names both offsets. #107's
  task-file line (de28d04) exists only on its branch, so its "(…; applied per R-204 — veto?)" mark becomes "ruled by
  R-351" in #107's next fix pass.
- **The lint.** `cargo xtask lint` fails on `isinf` or `isnan`, or on a comparison against an inf or NaN constant, in
  fragment-stage WGSL, naming the file, the line and the rule, because fast-math (R-297) may optimise those away and an
  unset-check tests the bit pattern (R-343). REQ-RENDER-083 (new, M0) carries it. A new task, TASK-M0-50, closes it,
  depends on TASK-M0-13 (PR #107), which writes the WGSL lint, and has the code and qa reviewers. The review-checklist
  grep in REQ-RENDER-001's verify line stays, as the backup. The render contract's "Unpack layer" gains a sentence
  saying so.
- PR #107's description marks both items ruled. Numbered R-351: the human's message gave this item no number (R-348's
  note).

*Applied per R-204, accepted by R-352:*
- "Fragment-stage WGSL" is every WGSL file under `crates/render/frag/`, generated or written by hand. Today that is
  `crates/render/frag/generated/payload_unpack.wgsl`.
- The rule joins `cargo xtask lint wgsl`, the WGSL lint TASK-M0-13 writes and `cargo xtask ci` runs, rather than a new
  subcommand.
- An inf or NaN constant includes a constant expression that evaluates to one, such as a `bitcast<f32>` of an inf or
  NaN bit pattern.

*Flagged, not resolved:* the render contract's WGSL traps and REQ-RENDER-001 also name two float comparisons that
stand in for `isnan` and `isinf`: a self-comparison, `x != x`, and `x > 65504.0`. Neither compares against an inf or
NaN constant, so under the human's words the lint does not cover them, and the checklist grep alone checks them.
RQ-191 asks whether the lint should cover them too. R-352 rules that it does (RQ-191's option (b)).

## R-352 — RQ-191's thirteen items stand; the fragment-stage lint also fails on a float compared with itself and on comparisons against finite-max stand-ins *(closes RQ-191; amends R-348 and R-351)*
*1 Oct 2026 · applied in REQ-RENDER-083, REQ-VAL-179, REQ-VAL-180, REQ-VAL-181, TASK-M0-50, TASK-M0-49, TASK-M0-19,
the render contract's "Unpack layer", CLAUDE.md § "How work runs", `plan/OPERATIONS.md` § "Reviewers" and
§ "Resources", and R-348's, R-349's and R-351's notes*

"RQ-191: (b). The fragment-stage lint also fails on a float compared with itself (x != x, x == x) and on comparisons
against 65504.0 or other finite-max stand-ins used as inf checks, since fast-math may fold or break them (R-297); the
fix is a bit-pattern test (R-343). TASK-M0-50 grows to match. All 13 veto items stand, including item 5 (on macOS,
local mutants runs get the timeout only; CI's Linux runners enforce both caps)."

*Applied:*
- **Option (b).** R-351's rule in `cargo xtask lint wgsl` also fails, naming the file, the line and the rule, on a float
  compared with itself (`x != x`, `x == x`) and on a comparison against 65504.0 or another finite-max stand-in used as
  an inf check, in every WGSL file under `crates/render/frag/`, because fast-math (R-297) may fold or break them. The
  fix the lint's message names is a bit-pattern test (R-343), as `pa_d_min_is_unset` does. REQ-RENDER-083's statement
  and verify line widen to match (reqio). TASK-M0-50 grows: its goal, deliverables, fixtures (each new rule's failing
  fixtures, and clean fixtures as its negative controls), acceptance tests and size. The render contract's "Unpack
  layer" sentence names what the lint covers, and REQ-RENDER-001's checklist grep stays as the backup.
- **All 13 items stand**, as RQ-191 listed them. Their "veto?" marks become "accepted by R-352" (in R-348's, R-349's
  and R-351's notes, CLAUDE.md, `plan/OPERATIONS.md`, the render contract, REQ-VAL-179, TASK-M0-19, TASK-M0-49 and
  TASK-M0-50), the wording otherwise unchanged:
  1. The message's rulings are recorded as R-348 to R-351, since R-347 was taken (R-278).
  2. R-348: REQ-VAL-179 carries the caps themselves, beside the calibrations REQ-VAL-180 and REQ-VAL-181.
  3. R-348: TASK-M0-49 waits for TASK-M0-14 (#96), which rewrites `mutants.yml`.
  4. R-348: TASK-M0-19's nightly full run depends on TASK-M0-49 and applies the same caps at the same values.
  5. R-348: "mutants runs" includes a local run, under the same caps at the same values, each where its machine
     enforces it.
  6. R-349: what is inside (the checkouts and worktrees, their target directories, the session's scratch directory)
     and what is outside (`~/.cargo`, `~/.rustup`, the rust-gpu cache, the Actions caches, the system temp folder,
     shell and git configuration, the SSD's folders).
  7. R-349: the away-mode log on the SSD, named in `plan/OPERATIONS.md` as the orchestrator's, is inside.
  8. R-349: a build's own cache writes are part of the build; an agent's own deletion or edit of a cache asks first.
  9. R-349: what a build, test or tool writes in its normal course under `std::env::temp_dir()`, and cargo-mutants'
     temporary copy of the tree, are part of running it; an agent's own deletion or edit there asks first.
  10. R-349: `scripts/cloud-setup.sh`'s installs are asked for by R-346 and R-347; anything beyond them asks first.
  11. R-351: "fragment-stage WGSL" is every WGSL file under `crates/render/frag/`.
  12. R-351: the rule joins `cargo xtask lint wgsl`, not a new subcommand.
  13. R-351: an inf or NaN constant includes a constant expression that evaluates to one (a `bitcast<f32>` of its bit
      pattern).
- **Item 5, as ruled:** on macOS a local mutants run gets the per-mutant timeout only; CI's Linux runners enforce both
  caps. Where #110 said a Mac run "may" get the timeout only (R-348's note, REQ-VAL-179, `plan/OPERATIONS.md` §
  "Reviewers"), it now says so as ruled, and TASK-M0-49 no longer has to find out whether the Mac enforces
  `ulimit -v`.
- RQ-191 moves to `docs/archive/review_queue/M0.md`.

*Applied per R-204 — veto?:* what the human's words leave open.
- **The finite-max stand-ins** are, as the lint's definite list:
  - binary16's largest finite value, 65504 (generation root §3.8's register entry `f16_finite_max`, payload §1's pack
    clamp), in any spelling: `65504.0`, `65504.`, `6.5504e4`, `65504.0f`, `65504h` among them;
  - binary32's largest finite value, 3.40282347e38 (`0x7f7fffff`), in any spelling;
  - either as a `bitcast<f32>` of its bit pattern (`bitcast<f32>(0x477fe000u)`, `bitcast<f32>(0x7f7fffffu)`), or any
    other constant expression that evaluates to it, as item 13 reads an inf or NaN constant;
  - either value negated (`-65504.0`, `-3.40282347e38`), the same stand-in for −inf;
  - a comparison with any of these fails, in either operand order and with any of `<`, `<=`, `>`, `>=`, `==`, `!=`.
  The lint reads naga's IR, where a literal's spelling is gone, so every spelling of the same value is caught. The
  corpus names 65504 (`f16_finite_max`) but not binary32's largest finite as a sentinel, and the check plan requires no
  definition requirement for a lint's list, so the list lives in REQ-RENDER-083 and TASK-M0-50.
- **A float compared with itself** is a comparison whose two operands are the same expression: the same naga
  expression, or structurally equal expressions reading the same `let`, argument, variable or buffer element; a float
  scalar or vector. Besides `==` and `!=`, the human's words, the lint also fails on `<`, `<=`, `>` and `>=` of an
  expression with itself, which fast-math may fold just the same (`x <= x` is false only for NaN).
