# Decisions

The log of human rulings. Each entry is dated and names the step that applies it. The open decisions are in
`DECISIONS_TO_MAKE.md` (step 5). Once a ruling is made it is recorded here.

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

R-21 to R-59 rule every item on `DECISIONS_TO_MAKE.md`, in the order of the sheet. The six blockers (R-21 CD-1, R-22 CD-2,
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
