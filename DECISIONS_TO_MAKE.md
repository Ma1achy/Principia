# Decisions to make

**Ruled 24 Sep 2026: every item below is ruled by R-21 to R-59 in `decisions.md`.** The six blockers are applied in step 5; the rest are applied when the build reaches them.

*Step 5 of the handoff, 24 Sep 2026. Every open item in audit section B and in `open-questions.md` is here, plus the open
items the corpus itself lists (INDEX, refinement policy §7, canonical_spec §11). Anything already ruled is in Appendix A,
with its ruling number. §8 lists defects found while writing the sheet: corpus fixes that need no decision.*

**How to read an item.** The options come first. The recommendation follows, with its reason, and says where the corpus
already recommends something ("*corpus:*") and where the recommendation is mine ("*mine:*"). "Files" lists what changes. The last
line says whether the item is a **blocker** (it must be decided before the build starts, because a day-one component can't be
written to spec without it) or can be **settled during the build** (and before what).

**Evidence from outside the corpus** is marked **[prin-rs]**. It comes from the local prin-rs clone
(`../principia-rs-test`). Under R-1 it isn't an authority, but where it fills a gap or contradicts the corpus the sheet
says so. The ruling is yours.

Blockers at a glance: **CD-1** α_min · **CD-2** body indexing · **IE-1** escape details · **PL-1** schema version ·
**PL-2** `t_min` · **RS-1** `DEBUG_MODE` bits.

---

## 1. Charts and decode

### CD-1 — `α_min`: 0 or 0.05 · *audit B25, R-5/R-10, pending change 6* · **BLOCKER**
- **Options:** (a) `α_min = 0`, full sphere. The poles are represented, and fenced by the collision detector, the conditioning readout
  and the SAT flags. (b) `α_min = 0.05` rad, a polar-cap buffer of about 3° off each pole.
- **Recommendation: (a) 0.** *Corpus:* the IC Inspector notes say "It was never a numerical guard — nothing divides by α; it
  only excised a ~3° polar cap off each pole. Dropped for full-sphere coverage" (`ic_inspector_scratchpad.md:20`), and
  the tool ingests exact poles as finite, SAT-flagged `z`. Every formula already carries the symbol, and 0 is the form
  written out. If (a), reword `dd_decoder.md:67` and `chart_reference.md:56`, which still call it "a buffer that keeps ‖ρ‖
  bounded away from zero". The scratchpad contradicts that description.
- **Files:** dd_decoder §3, §3.2 (:35, :66–70); chart_reference §0.2 (:35, :52, :56); inverse_encode_contract :74–75;
  dd_encode :32 (only if 0.05); `ic_inspector.html` :175, :199, :249 (only if 0.05).
- **Blocker:** the α link is in the core decode, so it needs a number on day one.

### CD-2 — Body indexing · *audit B1, pending change 5* · **BLOCKER**
- **Options:** (a) 0-based throughout: rename `ICDescriptor` `m1 m2 m3 → m0 m1 m2` (and `rho1_mag`, `rho2_mag`) and the 1-based
  display labels. (b) Keep 1-based names and translate from the 0-based decode at the boundary.
- **Recommendation: (a).** *Corpus:* "recommend 0-indexed internally, rename descriptor fields" (`dd_decoder.md:190`); the audit
  agrees. The decode, the softmax reference and the `detail` ids are already 0-based. *Mine:* at the same time, write down
  which pair each pair id 0–2 names (01, 12, 20?). No file says this, and the collision `detail` and `dmin_pair` depend on it.
- **Files:** dd_generation_root §3.6, §6; render_contract :14; dd_decoder §6; dd_simstate_payload :36–37 and §2 (the pair-id
  map); chart_reference :15–18, :472; dd_integrator §3.7 (:201, :214–215); chart_decoder_contract :26, :41, :76, :125, :131,
  :171, :173; inverse_encode_contract :102; colour_composition :159–170; `ic_inspector.html` labels.
- **Blocker:** the ledger's field names are generated on day one. Renaming later is a schema-version change (PL-1) and churns
  every consumer.

### CD-3 — Encode steps 0/1 order · *audit B3* · settle during the build (before encode, lookup and lock)
- **Options:** (a) the operational order 1a subtract CoM → 0 rescale → 1b subtract boost → 2 rotate → 3 mirror, made
  normative. (b) Keep the contract's "0 rescale, 1 translate" numbering, with `I` defined in the CoM frame.
- **Recommendation: (a),** and renumber the contract to match. *Corpus:* "the pin is: *CoM subtraction precedes the I
  computation*", pinned in the shared source (`dd_encode.md:55`, :115). *Mine:* under (b) the contract's own step 0 computes
  `I` before the frame it is defined in exists.
- **Files:** dd_encode §3.2, §6; inverse_encode_contract Part 2 (:33–35), Part 6 (:146–149); canonical_spec :52 and
  systems_architecture :149 (wording only).
- **During the build:** decode is canonical by construction. Encode, lookup, lock and dd_integrator's Burrau golden
  anchor ("via encode", :255) need it.

### CD-4 — Encode projection metric · *audit B2* · settle during the build (before off-curve Burrau encode)
- **Options:** confirm the pin (shape-sphere chordal ⊕ mass-simplex Euclidean, unit weights), or veto it for other weights or another metric.
- **Recommendation: confirm.** *Corpus:* the drill-down pins it ("both O(1)-normalised", `dd_encode.md` §3.4, "confirm/veto").
  *Mine:* no alternative is on record, and it only affects off-curve encode into the Burrau curve charts. Write the metric into
  inverse_encode_contract Part 5 kind 4 (:118), which leaves `dist` unspecified.
- **Files:** dd_encode §3.4, §6, test 5.7; inverse_encode_contract :118.
- **During the build.**

### CD-5 — Energy normalisation `η_E` · *audit B9* · settle during the build
- **Options:** (a) keep it, flag-gated as specified (`forbids_energy_normalisation`; disabled on invariant-momentum charts).
  (b) Drop it.
- **Recommendation: (a) keep, with one fix.** *Mine:* it is fully specified and enforced by a chart flag, so dropping it touches
  seven files for no gain. **But** "off" is currently signalled by `E* = 0`, which is also a real physical target (zero total
  energy). Make "off" an explicit `Option`/flag, not a value of `E*`.
- **Files:** dd_decoder §3.7, §6; chart_reference §0.6, :194, :504, :539; chart_decoder_contract :236;
  inverse_encode_contract :198; lowering_contract :157.
- **During the build:** optional, and outside the core decode.

### CD-6 — Chart domain functions · *audit B20* · settle during the build (before the invariant charts and quad-skip)
- **Options:** (a) every chart supplies its admissible region as a declared function, which the quadtree, lookup/lock and the legend query. (b) Regions stay implicit (warps, per-pixel tagging, validation at lookup only).
- **Recommendation: (a).** *Corpus:* the contract already registers `validate(u, v) -> ValidationResult` per chart type, and says
  "the quadtree uses the same function to skip quads" (`inverse_encode_contract.md:190–192`). *Mine:* the `Chart` trait in
  `chart_reference.md` §5.1 (:501–505) has no such method, so the trait and the contract disagree. Add it to the trait.
- **Files:** chart_reference §5.1, §5.2; chart_decoder_contract Parts 3, 5; inverse_encode_contract :157–192;
  lowering_contract appendix; scheduler_contract (it has no quad-skip rule yet); the step-6 GUI docs.
- **During the build:** the first chart (latent affine) is valid on the whole square.

### CD-7 — Which set the Burrau survey covers · *pending change 2* · settle during the build (before any Burrau statistic)
- **Options:** (a) shape only (fold the leg swap, `θ ≤ π/4`). (b) Shape × labelling (full range, both leg orderings, which
  are distinct labelled systems under the Burrau mass convention). (c) Keep both charts, each labelled with its quotient.
- **Recommendation: (c).** *Mine:* both answer real questions. The interim rule in `chart_reference.md` §4.5 already says
  to label the quotient and not to take shape fractions from the full-range chart. **But** no `system_image` value can express
  "covers each shape twice, as two labelled systems" (`chart_decoder_contract.md:229–234` has bijective, n-to-1 and ray-degenerate),
  so add one. The contract's "ray-degenerate" description of the Euclid plane is also stale now that §4.5 sweeps `ν` with `m` as an
  annotation axis.
- **Files:** chart_reference §4.1, §4.3, §4.5; chart_decoder_contract Part 5; inverse_encode_contract :116, :170, :199;
  dd_encode :34, test 5; lowering_contract :160–162.
- **During the build:** Burrau is chart #3, and the pipeline is chart-agnostic.

### CD-8 — Momentum decode without mass unweighting · *audit B25 remainder ("§2.6")* · close now
- **Options:** confirm (close it), or switch to mass-weighted momenta.
- **Recommendation: confirm.** *Corpus:* "Momentum decode convention — CONFIRMED correct … *no* `√μ` unweighting. The asymmetry
  with the position path is intended" (`ic_inspector_scratchpad.md:19`), matching `dd_decoder.md` §3.4. No ruling records it yet.
- **Files:** none if confirmed.
- **Not a blocker.**

---

## 2. Integrator and events

### IE-1 — The escape criterion's undefined parts · *pending change 11 (b)–(d)* · **BLOCKER** (for the escape label, `t_end` and outcome colour)
The rule is `ESCAPE ⟺ |Δn̂| over a window < tau AND E_rel > 0` (integrator_contract Part 7). The corpus gives no value or
formula for:
- **(b) The window:** in time units or in steps, and its length. **[prin-rs]** `win = 0.4` time units, realised as 0.406 at sync
  boundaries, with `CLOSURE_TAU = 1e-3` (`reference/escape_criterion.py:23`, `src/outcome.rs:263–270`).
- **(c) `E_rel`:** (i) the escaper's specific energy relative to the barycentre of the other two; (ii) the μ-weighted outer
  Jacobi `E_out` of the old detector; (iii) pairwise. **[prin-rs]** uses (i) as `½|dv|² − M_pair/d`, with `M_pair` only, not
  `M_pair + m_b`, and `d` floored at 1e-12. *Mine, a physics flag and not a choice:* the relative two-body orbit's energy
  uses the total mass. With `M_pair` alone the escaper looks more unbound than the relative orbit is. Decide this
  deliberately.
- **(d) The escaping body's id:** (i) the Jacobi outer body; (ii) the body not in the tightest pair; (iii) the body whose criterion fires. **[prin-rs]**
  FINDINGS :382–384: "The escaping-body label is the lowest firing index, not the escaping body … Transcribed, not
  corrected."
- **And a contradiction to rule on:** the corpus says `tau` "sits in a 383× gap and is not tuned" (pitfalls §2.2).
  **[prin-rs]** FINDINGS :360–364 says the gap "is at best 6.8× in this build, and near-field shows none at all".
- **Recommendation:** *mine:* (b) and (c) as the landed prin-rs criterion has them, because that is what produced the measured 100% precision
  / 96.3% recall, **after** a deliberate check of the `M_pair` vs `M_pair + m_b` term. For (d), define the escaper as the body
  whose `E_rel > 0` and whose separation from the other two is largest, which fixes the index-order defect. Re-validate `tau`
  on this build before calling it untuned.
- **Files:** integrator_contract Part 7 (:387), plus the stale remnants in §8 (D1); dd_integrator §3.6 (:155–161), test 5, closing
  line; pitfalls §2.2; payload §2 and :527; parity_contract :40.
- **Blocker:** the escape label can't be written to spec without (b)–(d).

### IE-2 — Event priority · *audit B4, R-6* · settle during the build, together with IE-1 and IE-3 (R-6's pin stands until then)
- **Options:** (a) keep the same-step pin `SIM_FAILED > COLLISION > ESCAPE > BOUNDED`. (b) Precedence by time: sim_failed first, then
  whichever event came first, with a tie going to collision, then triple ejection as a detail of escape, then running, then bounded.
- **Recommendation: (b).** *Mine:* the new escape fires late (t≈10) and may not terminate (IE-3), so a collision and an escape can
  land in different steps, and a same-step rule can't order them. **[prin-rs]** measured the case: "990 of 996 trajectories that
  fired both arms escape first and be labelled collision — 42.97% of the slice" (FINDINGS :376–380). Also place triple
  collision and triple ejection explicitly. The corpus gives neither a precedence.
- **Files:** dd_integrator §3.6 (:176–184), test 6, §6, closing line; integrator_contract Part 7 (:390); parity_contract :39,
  :193; payload §2 only if the rule becomes time-based.
- **During the build:** the label writer can use R-6's pin until this lands.

### IE-3 — Does escape terminate integration? · *pending change 11 (a)* · settle during the build
- **Options:** (a) terminate on escape; (b) keep integrating; (c) a per-mode toggle (the legacy reference froze "only in basin mode").
- **Recommendation: (b) until the three checks of pitfalls §2.4 pass.** *Corpus:* "escape must not terminate integration until
  §2.4's three checks pass" (canonical_spec :176, payload :495). **[prin-rs]** reports check 1 passed ("1.0000 are still unbound at
  +1, +2, +4, +8 boundaries and at `3·t_max`", FINDINGS :356–358). Record that, and run checks 2 and 3. *Mine:* also decide what
  `state` holds when escape has fired but the march continues. The enum makes escape and running mutually exclusive
  (payload :154).
- **Files:** pitfalls §2.4; integrator_contract Part 7; payload §2; dd_integrator §3.6.
- **During the build:** the corpus's default (keep integrating) is usable.

### IE-4 — The ionisation (triple ejection) gate · *pending change 7* · settle during the build (before outcome data at scale)
- **Options:** (a) the proposed gate: all three pairwise relative energies > 0 and all separations growing. (b) (a) plus total
  `E > 0`. (c) (b) evaluated with the escape rule's settling, not instantaneously.
- **Recommendation: (c).** **[prin-rs]** already requires total `E > 0` ("three pairwise-unbound instants can occur transiently in
  a bound system", `src/outcome.rs:380–398`). *Mine:* the old escape gate failed on exactly such transients, so an instantaneous test
  risks the same failure. Also decide its place in IE-2.
- **Old data:** close it. **[prin-rs]** counted pairs from the start (`98c49cc`), so no stored dump recorded two-pair collisions as
  binary. The dumps are stale for the escape and precedence reasons instead.
- **Files:** integrator_contract Part 7 (:388); dd_integrator §3.6 (:161), tests §5; parity_contract :40.
- **During the build:** it needs `E > 0`, which isn't reachable from rest, so the day-one Burrau and free-fall charts never hit it.

### IE-5 — The independent convergence reference: which integrator · *audit B8, canonical_spec §11* · settle during the build (before the integration-floor probe and Burrau ground truth)
- **Options:** (i) RK45; (ii) a separately written double-double integrator; (iii) CPU arbitrary precision (MPFR-style);
  (iv) Brutus-style convergence gating.
- **Recommendation: (ii).** *Mine:* the role needs **independence and** precision beyond f64 (canonical_spec :91: "double-double/
  arbitrary … immune to shared-source bugs"). RK45 is f64. Say explicitly that RK45 is the inspector's reference and not this one,
  because integrator_contract :87 calls RK45 "the adaptive high-precision reference". core_design :9's "double-double on the *same*
  kernel" is the precision reference, not this.
- **Files:** canonical_spec §7, §11 (:91, :151); validation_ground_truth_note :99; parity_contract :13; systems_architecture
  :45; integrator_contract :87, :242, :370; dd_integrator :92; spike_brief :61.
- **During the build.**

### IE-6 — FMA contraction per backend · *audit B17* · settle during the build (before the parity gate on a second backend)
- **Options:** (a) find a per-backend no-contract control; (b) explicit `fma` or single-rounded ops at every branch input
  (the existing rule 6), with contraction accepted elsewhere as honest divergence; (c) enumerate every branch that consumes multi-op
  arithmetic.
- **Recommendation: (b) + (c).** *Corpus:* rule 6 already hardens `d²` (`gpu_determinism_note.md:44`, :52). *Mine:* the change-11
  inputs `|Δn̂|` and `E_rel` are multi-op branch inputs, and the `k_esc` counter that absorbed single-step flicker is gone, so they
  need the same treatment. Enumerate the branch inputs rather than rely on a backend switch that may not exist.
- **Files:** integrator_contract Part 2c (:152–157), Part 4 (:338–340); gpu_determinism_note rules 2 and 6; parity_contract Tier B.
- **During the build.**

### IE-7 — The cross-checks invalidated by change 10 · *pending change 10* · settle during the build (before any further integrator measurement)
- **Options:** re-run them, patching the NumPy reference, or record them as waived or superseded.
- **Recommendation: re-run and patch.** *Corpus:* "re-run the Python cross-check and the divergence-vs-horizon table after
  adopting it" (`dd_validation_orbits.md:195`). No record says it was done. `workbench/tb_az.py:207` still clips only the time
  accumulator; the fix exists only as `tb_az_overshoot_fix.py`. The divergence-vs-horizon table exists only in an archived brief,
  so decide whether to regenerate it or retire it.
- **Files:** dd_validation_orbits §0.1, §5; dd_predictability_horizon (any figures re-measured); the reference scripts.
- **During the build.**

---

## 3. Payload

### PL-1 — Schema version = content hash of the ledger · *audit B6* · **BLOCKER** (cheap)
- **Options:** a hand-bumped integer, or a hash of the canonicalised §3 table.
- **Recommendation: the hash.** *Corpus:* "deriving the version … makes the failure impossible rather than merely forbidden.
  *Recommendation to adopt.*" (`dd_generation_root.md:452`); the audit agrees.
- **Files:** dd_generation_root §4, §5 test 7, §6; caching_contract :12; render_contract :59, :182; payload :161, :276.
- **Blocker:** the generator is the root of the build DAG, and the version feeds the cache signature from day one.

### PL-2 — `t_min` for the closure minimum · *pending change 9* · **BLOCKER** (for the integrate kernel)
- **Options:** (a) a stated fraction of the system size in canonical units (`√I = 1`); (b) a departure threshold on the shape
  sphere: the minimum starts once `|n̂(t) − n̂(0)|` has first exceeded a stated value.
- **Recommendation: (b), with the value to be set by measurement.** *Mine:* closure is measured on the unit shape sphere, which is
  already scale-free, so "a fraction of the system size" (payload :118–120) has no clear referent there. An angular departure is
  gauge-covariant by construction. Also add `closure_min` / `closure_step` to the ledger (§8 D3).
- **Files:** payload §1 (:69, :118–120); dd_generation_root §3.4; dd_validation_orbits §6; integrator_contract (kernel semantics).
- **Blocker:** the kernel writes `closure_min` every step and needs the rule.

### PL-3 — `dominant_pair` attribution · *audit B7, canonical_spec §11* · settle during the build (before any per-pair view)
- **Options:** specify the three open items (which branch cuts are `a`/`b` and the crossing sign; the thrice-punctured-sphere
  relation; a deterministic map to 01/02/12), or leave per-pair quantities out of v1.
- **Recommendation:** *mine:* leave per-pair views out of v1 and specify all three before any per-pair view ships. Word storage and
  `S_word` aren't blocked (`symbolic_dynamics_contract.md:33`, :42).
- **Files:** symbolic_dynamics_contract §1–4; dd_generation_root :45, :87–90; debug_tooling_plan :51; render_contract :104;
  payload §5, :468; canonical_spec :152.
- **During the build.**

### PL-4 — `FULL_RETENTION` has no owner · *step 3 open question* · settle during the build
- **Options:** (a) write an owner, most likely the uniform-grid measuring/export path; (b) return bit 4 to reserved.
- **Recommendation: (b) until an owner is written.** *Mine:* nothing consumes it, and a flag with no described path is what
  step 3 had to chase. Reinstate it with an owner when the export path needs it.
- **Files:** scheduler_contract Part 5 (:120), Part 2 (:36); export_animation or caching contract, if (a).
- **During the build.**

### PL-5 — Memory tiers and quality device, keyed off `eps` · *audit B10, pending change 9* · settle during the build (before the auto controller and hard cap)
- **Options:** fold the eps / frame-budget / hard-cap axes into `QualitySettings` and the ladder, or leave the banner-only state.
- **Recommendation: fold them.** *Corpus:* the audit says "Do it before they're implemented"; telemetry §3.5 has the three-axis model
  and R-4 points there. Recompute the stale widths as part of it: `memory_tiers.md` :28, :90, §4 (:156–167),
  `canonical_spec.md:79` and `systems_architecture.md:63`, :163 still use 136/88 B (now 144/96).
- **Files:** quality_device_note (settings struct, §3 ladder); memory_tiers; canonical_spec :79; systems_architecture :63, :163;
  telemetry §3.5, §4.
- **During the build:** refinement keys on `eps` directly, and the tier numbers are guesses by design (audit D).

---

## 4. Refinement and scheduling

### RS-1 — `DEBUG_MODE` in `QuadRequest.flags` · *step 3 open question* · **BLOCKER** (cheap)
- **Options:** (a) assign reserved bits 6–7 (four values); (b) rely on the lowering contract's baked variants and drop "in dispatch
  flags" from the render contract and the debug plan.
- **Recommendation: (b).** *Mine:* the lowering contract already bakes the debug modes as pre-built variants (`lowering_contract.md:52`),
  so no runtime branch needs a flag. Keep bits 6–7 reserved.
- **Files:** scheduler_contract Part 5; render_contract :202; debug_tooling_plan :26; lowering_contract :52, if (a).
- **Blocker:** the debug catalogue comes first in the build order (render_contract :204).

### RS-2 — `alpha_area`'s two defects · *audit B11, pending change 12* · settle during the build (refine ships with them otherwise)
- **Options:** (a) `alpha_lo = 0` (measured: +49% quads, none stopped); (b) keep `0.005`, and tell empty masks from full ones by
  `n_unresolved`; (c) refuse the floor when the exponent goes negative (`d = 2 − α > 2`).
- **Recommendation: (b) + (c).** *Mine:* both are "a check whose reachable output set does not include the failure it guards
  against" (pitfalls :372). `n_unresolved` is already computed, and a negative exponent is impossible in the plane. (a) gives up
  the stop rule's whole purpose. Also update the ledger's stale `alpha` row (§8 D4).
- **Files:** refinement_policy §2, §2.2, §7; dd_generation_root §3.7 (:269–273); INDEX :165; pitfalls :372.
- **During the build:** the policy is "settled enough to build on", with known defects.

### RS-3 — `N = 16` vs 8 on WebGPU · *audit B14* · settle during the build (before WebGPU deployment)
- **Options:** N = 16 where the 256-invocation ceiling allows it; force N = 8 on WebGPU; worker tiles (rejected unless profiled).
- **Recommendation: measure.** *Corpus:* "measure it rather than assume" (`systems_architecture.md:220`). *Mine:* fix the
  inconsistencies now whatever the answer. memory_tiers' N 24 and 32 need 576 and 1024 threads at one thread per texel, over
  WebGPU's 256. `lowering_contract.md:138` says "N×N workgroups per quad" against "ONE WORKGROUP PER QUAD"
  (systems_architecture :170).
- **Files:** systems_architecture §5.5; memory_tiers §4 N column; scheduler_contract :141; lowering_contract :138;
  quality_device_note.
- **During the build.**

### RS-4 — The priority formula and off-screen guard against policy §0.1 · *the "camera not wired" item* · settle during the build
- **Options:** close "camera not wired into priority" as done, and reconcile scheduler Part 6's remaining mechanics with §0.1, or
  leave Part 6 as it is.
- **Recommendation: close and reconcile.** *Corpus:* "the camera now reaches the scheduler" (INDEX :158), so the open-questions entry
  is out of date. Two Part 6 lines still predate R-15: `P_complexity = 1 − coherence` with "never compute off-screen" (:132), and the guard "offscreen → stop" (:134), which contradict
  the policy's "off screen … the criterion decides depth, adequacy read as met". Whatever the ruling, also say whether the
  order in view (§0.1, "the criterion decides ORDER") enters through `P_complexity`.
- **Files:** scheduler_contract Part 6 (:132, :134); open-questions.
- **During the build.**

### RS-5 — Frontier scoping · *audit B15* · settle during the build
- **Options:** a naive cull (measured worse); a margin derived from the refill rate; velocity extrapolation; refining the swept path.
- **Recommendation: a margin derived from the refill rate, with a plain widened margin as the baseline to beat.** *Corpus:* "The margin must be
  derived from the refill rate" (refinement_policy :271); "Option 3 is the honest baseline and the others must beat it"
  (the retained slippy-map brief §4, :315).
- **Files:** refinement_policy §0.1, §7; scheduler_contract Parts 6, 7; INDEX :170.
- **During the build:** an unbounded frontier wastes work but converges.

### RS-6 — Relevance arithmetic in global UV · *audit B16* · settle during the build (before any deep-zoom demo)
- **Options:** compute relevance relative to the camera or the quad centre; or leave it in global UV.
- **Recommendation: relative.** *Corpus:* "Take before any deep-zoom demo" (INDEX :171). *Mine:* use the same centre-plus-half-width
  pattern `deep_zoom.md` §1 already uses for GPU sample positions.
- **Files:** scheduler_contract Part 6 (`P_visible`, `P_focus`); deep_zoom §1; INDEX :171.
- **During the build:** the defect shows only past about depth 40.

### RS-7 — `Decision::Undetermined` has no budget line · *audit B13* · settle during the build
- **Options:** add a second refinement axis and budget line (finer `eta`); or report Undetermined as terminal-and-flagged until
  a chart needs more.
- **Recommendation: report and flag for now.** *Mine:* it "measured exactly 0.0000 in all 36 cells" under the shipped step control
  (refinement_policy :248). Add the axis when a chart shows it non-zero.
- **Files:** refinement_policy §6; telemetry §3.5, §4; scheduler_contract Part 6; quality_device_note.
- **During the build.**

### RS-8 — A cheap `sea_fraction` estimator · *audit B12* · settle during the build
- **Options:** none framed yet; the corpus names it as "the concrete unbuilt next step" (refinement_policy :221).
- **Recommendation:** *mine:* build it after the tier controller exists (PL-5), which is what consumes it.
- **Files:** refinement_policy §5.1, §7; telemetry §3.5, §4, :388.
- **During the build.**

### RS-9 — The refinement policy's own open measurements · *refinement_policy §7* · settle during the build
Not on the audit's list, but open in the policy doc:
- **Depth beyond level 6:** "Every tree in the tolerance study is capped; nothing is known past it" (:263).
- **A calibrated grid / `tau`:** "All 36 cells ran shipped defaults; a calibrated `tau` moves several by an order" (:264–265).
- **Merging under real camera motion:** "measured on a pulse fixture but not under real camera motion" (:266–267).
- **Recommendation:** measure all three in the build's first refine milestone. None of them changes the design as written.
- **Files:** refinement_policy §5, §7.

---

## 5. Colour

### CO-1 — Shape-sphere collision landmarks: mass-weighted or fixed 120° · *audit B18* · settle during the build (before the physics-overlay occupant)
- **Options:** (a) mass-weighted positions, computed from the shape map with the current masses (R-14); (b) fixed 120° on the equator.
- **Recommendation: (a).** *Corpus:* "Convention call — RESOLVED: keep mass-weighted. The mass-weighted landmarks *are* the actual
  collision loci … Geometric/pinned landmarks would be a *different* chart" (`ic_inspector_scratchpad.md:25`), and colour_composition
  §2 (:197–209) already assumes the landmarks move with the masses. If (a), `principia_gui_mock.html` :784, :937 (fixed 120°) and
  `principia_colour_presets.html` :131–139 (a heuristic skew) are wrong.
- **Files:** dd_integrator §3.7 (:201, :211–212), test 9; dd_colouring :90; colour_composition :197–209, :481;
  trajectory_viewing :78, :84; the HTML references above.
- **During the build.**

### CO-2 — OKLab coefficients checked against Ottosson · *audit C* · settle during the build (before the colour constants enter the shared source)
- **Options:** run the check (there is no design choice).
- **Recommendation: run it.** *Corpus:* `dd_colouring.md:180`. The corpus is internally consistent: every HTML copy is a subset of the 33
  values in dd_colouring §3.1. It hasn't been checked against the reference.
- **Files:** dd_colouring §3.1 only, unless a digit is wrong.

---

## 6. GUI

None of these blocks the build. They concern the GUI, which step 6 writes up.

### GU-1 — Where undo lives · *audit B21*
- **Options:** (a) an undo/redo history over typed `setField` edits, owned by the state contract and shared by every GUI; (b) each GUI keeps its own.
- **Recommendation: (a).** *Mine:* the contract is already "subscribe/emit … typed edits (`setField(path, value)`)"
  (`gui_state_contract.md:50`), which is exactly the log undo needs, and colour_composition :264 assumes typed edits for undo.
  A per-GUI history would be rebuilt by every replacement GUI.
- **Files:** gui_state_contract §2, §7; colour_composition :264; render_gui_spec; caching_contract :133–134 if the history crosses the worker boundary.

### GU-2 — Node interfaces declare their input domains · *audit B19*
- **Options:** (a) the node/occupant interface declares its input domains, extending `uniformSchema` and port types, and the legend is generated from
  them; (b) domains stay per-field in the manifest, and the legend is derived from the source field and the `range_norm` mode.
- **Recommendation: (a), with (b)'s manifest domain as the default a node inherits.** *Mine:* a generated legend needs to know what
  the node maps from, and a node can transform its field's domain. The manifest already declares per-field domains
  (render_gui_spec :246–249).
- **Files:** gui_state_contract §3, §4 (:77, :85–111); render_gui_spec §6, §10.1; colour_composition §1.2, §3; dd_colouring :119;
  debug_tooling_plan :43–44.

### GU-3 — The deep-zoom precision warning's thresholds · *audit B22*
- **Options:** (a) fixed depth thresholds (near `ℓ_switch = 20` and about 50); (b) event-driven, raised when `DECODE_SWITCHOVER` fires on
  visible quads and when `AT_F32_FLOOR` is hit.
- **Recommendation: (b).** *Corpus preference:* "The more robust trigger is *adaptive* rather than depth-thresholded"
  (deep_zoom :57), and the absolute floor "is not a policy constant" (scheduler_contract :74).
- **Files:** deep_zoom §2; scheduler_contract Part 4; the step-6 GUI docs; gui_state_contract §2 if it needs a state field.

### GU-4 — Cursor bias · *audit B24*
- **Options:** (a) specify it, as `P_focus` centred on the pointer instead of the viewport centre while the pointer is in view, with a weight and a
  decay; (b) drop it.
- **Recommendation: (a).** *Mine:* it is cheap, since the pointer is already in a shared buffer (caching_contract :133) and `P_focus`
  already exists (scheduler_contract :132). It is the one attention signal the pointer-channels note builds on. It is named only in
  an unratified note (`scratchpad_pointer_channels.md:132–134`), so (b) costs nothing if you'd rather not.
- **Files:** scheduler_contract Part 6; scratchpad_pointer_channels :132–134; caching_contract :133.

### GU-5 — Profiler schema v1 · *audit B23* · partly: the measurement struct is needed from the first frame loop
- **Options:** (a) a new schema built from scopes, GPU passes, counters, allocations and events; (b) formalise telemetry §2's
  frame record as v1. Either way, pick the stage list (telemetry's five stages or the mock's fine/coarse sets) and the format.
- **Recommendation: (b), in JSON.** *Mine:* telemetry §2 already defines the record and says profiling is "part of the render loop's
  contract, always present" (:189–190), and §5 wants one self-describing file with one parser. JSON meets that. Take
  telemetry's five stages; the mock's lists are finer views of them.
- **Files:** dd_telemetry_and_tiers §2, §5, §5.5, §7; debug_tooling_plan; the step-6 GUI docs.
- **Build:** the struct goes in with the first frame loop; the serialised schema can follow.

---

## 7. Tooling and verification

### TO-1 — Yoshida-6 coefficients checked against Yoshida (1990) · *audit C* · settle during the build (before Yoshida-6 enters the shared source)
- **Options:** run the check (there is no design choice).
- **Recommendation: run it,** and fix the label at `integrator_contract.md:366` ("Yoshida's `w₂ < 0`"), which uses the
  LaTeX's Yoshida-4 labels. In dd_integrator the negative weight is `w₀` (Yoshida-4) and `w₁` (Yoshida-6). The markdown and the archived
  LaTeX agree to 15 digits, and `w₀ = 1 − 2(w₁+w₂+w₃)` checks.
- **Files:** dd_integrator :81–90, §6; integrator_contract :366.
- **Not a blocker:** Yoshida-6 is the Ultra–Extreme occupant, not the default.

### TO-2 — The non-Metal parity run · *canonical_spec §2, §11* · settle during the build (before Paper-2 numbers)
- **Options:** only when and where to run it (Vulkan or D3D12), and whether it gates the build or only Paper 2.
- **Recommendation: gate Paper 2, not the build.** *Corpus:* "run the gate on a non-Metal backend (Vulkan/D3D12) before trusting the
  survey for Paper 2" (canonical_spec :41). It needs the native parity test to exist first. If branch words fork, IE-6 becomes urgent.
- **Files:** canonical_spec §2, §11 (closing it); gpu_determinism_note, parity_contract if it forks.

---

## 8. Corpus defects found while writing the sheet (no decision needed; fix during the build)

These aren't choices. The corpus contradicts itself, and the fix follows from rulings already made. They're listed so
nothing is dropped. The same list is in `open-questions.md`.

- **D1. Stale escape remnants from the change-11 fold.** Step 4 folded the new rule into Part 7 and §3.6 but missed:
  integrator_contract Part 3's `R_esc, k_esc` row (:318), Part 4's `c_esc` bullet (:340), Part 5's t = 0 escape test
  "(outward + positive outer-energy, not merely beyond `R_esc`)" (:357), and dd_integrator's closing line "The counter
  forgives a flickering gate" (:283). Fold them together with IE-1.
- **D2. `payload.md:527` conflates two quantities.** It calls `closure_min` (`|n̂(t) − n̂(0)|`, since t = 0) "the same quantity as
  the escape criterion's settling test", which is a windowed `|Δn̂|`. No lagged `n̂` register is specified for the window.
  Settle with IE-1.
- **D3. The ledger lacks the closure fields.** `closure_min` and `closure_step` are in payload §1 but not in dd_generation_root
  §3.4. Adding them is a schema event (PL-1).
- **D4. The ledger's `alpha` row is from the α era.** dd_generation_root :271 defines `alpha` as "scaling exponent of
  `ensemble_spread`, parent↔child", not `alpha_area` (RS-2).
- **D5. `inverse_encode_contract.md:199`** still uses `has_redundant_hemisphere`, which `system_image` replaced
  (chart_decoder_contract :229).
- **D6. The 136/88 B widths** in canonical_spec :79 and systems_architecture :63, :163 (see PL-5).

---

## Appendix A — Already decided (left off the sheet)

| ruling | settles |
|---|---|
| R-1 | the markdown corpus is the only authority |
| R-2 | the INDEX cites refinement policy §0.1, not the unpushed commit |
| R-3 | the colour and COM-projection PDFs are retired; what the corpus uses is ported |
| R-4 | "spec-keyed defaults" means telemetry §3.5's tier tables |
| R-5 | the chart constants go to this sheet (amended by R-10) |
| R-6 | event priority goes to this sheet; the collision-beats-escape pin stands meanwhile (**still open: IE-2**) |
| R-7 | step 3's done-check (met; rerun after step 4 returns only the retirement ruling) |
| R-8 | the IC Inspector copies in `docs/` are canonical |
| R-9 | the toolchain spike's findings live in `experiments/results/` |
| R-10 | **audit B25 in part:** `μ_max = 5`, `q_max = 2` (**α_min still open: CD-1**) |
| R-11 | the per-body momentum cap is rejected |
| R-12 | the shape-sphere chart map has no polar buffer |
| R-13 | lookup has no coincident-bodies rejection |
| R-14 | **audit B5:** the shape-sphere component → axis convention (**landmark positions still open: CO-1**) |
| R-15 | **pending change 12:** `Policy::Tolerance` governs refinement |
| R-16 | the colour PDF's map lists are ported; the golden tests are pinned to them; magenta is a plain default |
| R-17 | the diffusion sentinel uses the streaming slope |
| R-18 | the agreement value is `spread_event`; `ensemble_outcome_agreement` is retired |
| R-19 | **pending change 8:** `ADVANCE` is the occupant seam; `owns_time_mapping`; the τ-schedule count bound |
| R-20 | the impurity mask compares with `dominant_outcome` at the joint grain; no `majority_class` |
| porting rule | a port adds and never removes a decision without a ruling; every commit ends with a "Removed lines" note |

## Appendix B — Not on the sheet by design

- **Deliberately parked (audit D),** each with its reason in its owning file: `00_philosophy.md` §7 (post-1.0); reversibility
  replay and KS regularisation (integrator contract Part 6); cross-chart payload sharing (caching contract Part 3); quantised
  checkpoints (moot under lockstep); batch literature import (validation phase); tier-table numbers (guesses by design,
  calibrated from telemetry).
- **Handled by later steps:** the GUI design formalisation (audit A4, canonical_spec §11) is step 6; the build plan (audit A5,
  canonical_spec §11) is step 7.

## Appendix C — Coverage check

Every audit-B item and every `open-questions.md` item, and where it is.

| source | where |
|---|---|
| B1 | CD-2 |
| B2 | CD-4 |
| B3 | CD-3 |
| B4 | IE-2 (R-6's pin interim) |
| B5 | R-14 |
| B6 | PL-1 |
| B7 | PL-3 |
| B8 | IE-5 |
| B9 | CD-5 |
| B10 | PL-5 |
| B11 | RS-2 |
| B12 | RS-8 |
| B13 | RS-7 |
| B14 | RS-3 |
| B15 | RS-5 |
| B16 | RS-6 |
| B17 | IE-6 |
| B18 | CO-1 |
| B19 | GU-2 |
| B20 | CD-6 |
| B21 | GU-1 |
| B22 | GU-3 |
| B23 | GU-5 |
| B24 | GU-4 |
| B25 | R-10 (μ_max, q_max); CD-1 (α_min); CD-8 (momentum weighting) |
| open-q: `DEBUG_MODE` bits | RS-1 |
| open-q: `FULL_RETENTION` owner | PL-4 |
| open-q: change 1 | closed (R-20) |
| open-q: change 2, the Burrau survey set | CD-7 |
| open-q: change 3, 4 | closed (folded in step 3; moot) |
| open-q: change 5 | CD-2 |
| open-q: change 6 | CD-1 |
| open-q: change 7, the ionisation gate and old data | IE-4 |
| open-q: change 8 | closed (R-19) |
| open-q: change 9, `t_min` and memory_tiers §4 | PL-2, PL-5 |
| open-q: change 10, the cross-checks | IE-7 |
| open-q: change 11, termination / window / `E_rel` / escaper id | IE-3, IE-1 |
| open-q: change 12, the `alpha_area` defects and the camera | RS-2, RS-4 |
| open-q: audit C | TO-1, CO-2 |
| open-q: audit D | Appendix B |
| open-q: the dangling refs, findings.md, the change-11 fold, the R-15/R-18 fold | closed (steps 3–4) |
| canonical_spec §11: the convergence reference, `dominant_pair`, the non-Metal run | IE-5, PL-3, TO-2 |
| refinement policy §7: depth > 6, the calibrated grid, the live playhead | RS-9 |
