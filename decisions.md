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
*24 Sep 2026 · applied in step 5*

List them with audit B25. `μ_max` is 5 in the LaTeX and 4 in the IC Inspector notes. `α_min` is 0.05 in
the LaTeX and B25, and 0 in the markdown corpus and the tool.

## R-6 — The event priority order goes to the decision sheet *(closes RQ-4)*
*24 Sep 2026 · applied in step 5*

List it with audit B4. It has no LaTeX cross-check. Until it's decided, the markdown's pin in
`principia_dd_integrator.md` §3.6 (collision beats escape) stands.

## R-7 — Step 3's done-check *(closes RQ-6)*
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
*25 Sep 2026 · applied in step 7*

`parity_contract` governs. Branch decisions are identical on identical inputs, per step. Labels on chaotic trajectories may
differ across precisions. Reword the integrator and determinism texts to match. Tier S asserts the outcome class only on
non-chaotic fixtures.

## R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*
*25 Sep 2026 · applied in step 7*

Native in-process `wgpu` sets the Tier-N tolerances. Dawn CI is dropped. Real browsers are checked against those tolerances
with the browser build (M8).

## R-86 — The payload doc governs the eight payload items *(closes RQ-37)*
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

§4.1's gate stands: the cross-check runs only for t < t_max(f32). REQ-VAL-070's gate reads the GPU measurement of
t_max(f32) (REQ-VAL-071); R-35's change-10 re-run supplies the f64 figure and the method. t_max annotates refinement; it
doesn't gate it.

## R-94 — The GUI snapshot is ~10 Hz *(closes RQ-45)*
*25 Sep 2026 · applied in step 7*

Throttled (caching Part 6a). egui redraws at frame rate from the latest snapshot. Conform systems_architecture §3.

## R-95 — After escape fires *(closes RQ-48, in part)*
*25 Sep 2026 · applied in step 7 · amended by R-103*

Once escape fires, `state` reads escape and `t_end` is fixed. Time averages (FTLE's S/T and the like) freeze at t_esc. Any
march for the pitfall §2.4 checks runs only in the validation harness, on its own state; in production `done` is set when
escape fires and the loop ends, and the payload never sees that march. The window is sampled at macro-step
boundaries for unregularised occupants and at sync boundaries for regularised ones. Re-validate against check 2's
independent ground truth, with the legacy t = 30 set kept as a comparison.

## R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*
*25 Sep 2026 · applied in step 7 · last bullet withdrawn by R-106*

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

Through render_gui_spec's "listen". Its open points (the reference pitch, θ alone or stereo θ/φ, the whole or a
windowed spectrum) become R-71 / R-72 calibration and definition requirements. The rest of the file stays working notes.

---

*Checkpoint B of step 7 (PR #8). Rulings R-110 to R-133, 25 Sep 2026. RQ-102 and RQ-103 (the prin-rs occupants and
fixtures) wait for the human. ✱ marks a design ruling the human may veto.*

## R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*
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
*25 Sep 2026 · applied in step 8*
*Dormant since R-186 (26 Sep 2026): no self-hosted runner exists. The text stands, and applies again when one is added.*

GPU jobs on the self-hosted runner run only for pushes and for PRs from this repository:
`if: github.event.pull_request.head.repo.full_name == github.repository` (or the event is a push). Fork PRs get the CPU
and lavapipe jobs only. The runner runs under its own macOS user account. Outside contributors need approval before any
workflow runs.

## R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*
*25 Sep 2026 · applied in step 8*

GitHub won't let one account approve its own PR, so each reviewer posts a PR review headed `VERDICT: APPROVE <role>` or
`VERDICT: CHANGES <role>`. A CI check, `reviews-complete`, passes only when every role the task file names has approved
on the latest commit. The human merges, or a merge bot does once all the checks are green.

## R-176 — Controls come before the tests that need them *(closes G3, S2)*
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
