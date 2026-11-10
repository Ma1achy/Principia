# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

## RQ-391: Whether the Export Styles' frames (Polaroid border, banknote, poster) become a later "Frame" stage after the Print graph *(the human's queued question, R-485, §0 "separate, later", the message's point 6; context: none until ruled)*

*Queued by the human (point 6, as RQ-388), and §0: "On presentation frames (Polaroid, banknote, posters): separate,
later." For the human: yes, whether there is a Frame stage at all.*

- **File, section:** `docs/design/principia_print_graph.md` § "12. Open questions (for the review queue, not the agent
  to settle)", question 4: "Should the **Export Styles** frames (Polaroid border, banknote, poster) become a later
  "Frame" stage after the Print graph? Ruled out of scope for this bundle."
- **Options:** (a) a Frame stage after the Print graph, its own design later; (b) frames as Print graph presets, a
  sub-graph like any other; (c) frames as an export option only, outside `RenderState`.
- **Applied meanwhile:** nothing is built (§0: "separate, later"); the lab's Polaroid medium (a Photographic node and
  preset) is a medium, not a frame, and is ported with the rest (R-486). Nothing waits.
- **Parked:** R-527 (20 Oct 2026), the human: "RQ-391 later." A deferral by ruling, with no target: the entry
  stays open and returns to the human when the human raises it. Nothing is built; the lab's Polaroid medium stays a
  medium (R-486).
- **25 Oct 2026 (R-625):** the human: "Parked (RQ-378, RQ-386, RQ-391): stay parked." The entry stays open and parked (R-527); what it applies stays applied, and nothing waits.
- **4 Nov 2026 (R-706):** deferred until the exporter is designed, by owner 4 Nov 2026. The entry stays open.
- **Deferred:** deferred to the exporter's design by owner 5 Nov 2026 (R-803, the owner: "RQ-391: Defer until the exporter is designed.").
- **7 Nov 2026 (R-825), restated:** the owner: "RQ-391: defer until the exporter is designed." It stays deferred to the exporter's design.
- **Deferred:** deferred to the exporter's design by owner 8 Nov 2026 (R-890, the owner: "RQ-391: defer until the exporter is designed.").

## RQ-577: The precision-ladder proposal's seven open questions (§6), and the amendments it proposes to R-265, R-572, R-90, canonical spec §1 item 3, deep zoom §2 and its layer table, and integrator contract Part 2 and Part 2b *(the human's queued questions on the 26 Oct incoming proposal; R-265, R-572, R-90, R-561, R-547, R-33; context: none built)*

*Queued by the human, 27 Oct 2026, on `docs/incoming/2026-10-26_precision_ladder/` (PR #297): "A PROPOSAL. Nothing in
it is in force and nothing in it is built until I rule on its §6." and "Do not amend R-265, R-572, R-90, the canonical
spec, deep_zoom or the integrator contract from it." **Nothing in the proposal is in force and nothing in it is built
until the human rules on its §6.** For the human: yes, every question changes what is built. No option is recommended
here.*

- **File, section:** `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md`
  (sha256 `57eca498…852b192d7`), § "6. Rulings touched and open questions"; its design in § "3. The design" (3.1 to
  3.10), its build order in § "4. Build order" and its twelve tests in § "5. Tests (each with what makes it fail)". Its
  header: "**Draft design note, 26 Oct 2026. Not authority.** It proposes changes to rulings in force (R-265, R-572,
  canonical spec §1 item 3, R-90). Nothing here is built until the human rules on §6."
- **The amendments it proposes** (§6, "Rulings it would amend", quoted):
  - **Canonical spec §1 item 3:** "**Canonical spec §1 item 3**, "The GPU is f32-locked". Proposed reading: the GPU's
    native arithmetic is f32 (and f64 where the hardware offers it); extended precision on the GPU is emulated, opt-in,
    and guarded by the canary." In force: `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the
    defining decision)", item 3: "**The GPU is f32-locked** (WebGPU has no f64)."
  - **R-265:** "**R-265**, the kernel is f32 and f64 only and `Dd` is parked. Proposed: the kernel's `Real` gains `df32`,
    `qf32`, `dd64` and `mp(bits)`. R-265's condition for unparking has two clauses, "a precision question needs it and
    Brutus is too slow". The case made here is the Burrau neighbourhood mapped past t ≈ 52 (§2), where a per-pixel Brutus
    run is too slow; the single trajectory does not meet the second clause." In force: `decisions.md` § "R-265 — The
    shared kernel is f32 and f64 only; double-double is parked *(closes RQ-161, amends R-33)*".
  - **R-572:** "**R-572**, "f32 or f64 only" in a Result. Proposed: the ladder, with f64 still the default."; and "**R-572
    and results §3.2**: a Result's uniform grid is a complete tree at one level, run in quads rather than rows (§3.10)."
  - **Integrator contract Part 2 and Part 2b:** "**Integrator contract Part 2 and Part 2b**: Taylor joins the stepper
    axis; precision becomes a third axis; R-561's warn-never-block extends to it." (`docs/contracts/principia_integrator_contract.md`
    § "Part 2 — Occupants and the capability profile" and § "Part 2b — Regularisation is a SECOND swappable axis, not a
    property of the stepper".)
  - **R-90:** "**R-90**, `ℓ_switch = 20` as a fixed bound: per precision (§3.9)." Its proposed rule (§3.9): "`ℓ_switch`
    becomes a per-precision backstop, `ℓ_switch(P)`, never below `b(P)/2`, and is a frozen value of the sim key (Part 2c
    rule 1)."; and "`AT_F32_FLOOR` generalises to `AT_PRECISION_FLOOR(P)`."
  - **Deep zoom §2:** "**Deep zoom §2**, "Floor moves from ~23 to ~50+" with an f32 sum on the GPU: contradicted by the
    prin-rs measurement (§3.9 item 4). This one stands on its own and goes to `REVIEW_QUEUE.md` even if the ladder is not
    adopted." That finding is RQ-578, queued separately as the human asked.
  - **Deep zoom's layer table:** "**Deep zoom's layer table**: the linearised decoder belongs to depth, not to layer 2's
    adaptivity (§3.10)." In force: `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree", layer 2: "CPU→GPU
    adds the linearised-decoder uniforms".
  - Held, not amended, by the proposal's own account: "**Integrator contract Part 2c** is held, not amended"; "**R-547**
    governs the emulated arithmetic's branch inputs and forbids an FMA-based TwoProd on this project's backends (§3.5)";
    "**R-33** is unchanged; §3.8 states why."
- **The seven open questions** (§6, "Open questions for the human", verbatim):
  1. "Unpark extended precision now, or only after M3 (the current integrator milestone) closes?"
  2. "Is the live view allowed the ladder by choice (§3.6), or is it Result-only?"
  3. "Taylor's default order rule: Jorba & Zou's, or a fixed order per tier?"
  4. "What does the profile's `force_evals` mean for Taylor: a measured equivalent, or a new field?"
  5. "Event location by dense output (§3.4): adopt it for Taylor only, or keep events where every stepper samples them,
     so steppers stay comparable?"
  6. "Should a Result above f64 be able to store raw extended state, or rounded fields only?"
  7. "Per-quad precision in the tree: may a deep quad run at a higher rung than its parent, so that only the deep part of
     a view pays for it? It costs the scheduler a precision field per quad and makes neighbouring quads differ in
     arithmetic, which R-547's determinism rules would have to cover."
- **Beside the seven, implied by the amendments:** whether the ladder is adopted at all (the amendments above, each
  accepted, changed or refused), and whether uniform mode becomes "a complete tree at one level" (§3.10), which the
  proposal lists as an amendment but not as a question.
- **Unclear in the record, said here, not resolved:**
  - The proposal's header names four rulings it changes (R-265, R-572, canonical spec §1 item 3, R-90); its README and
    §6 add deep zoom §2 and its layer table, integrator contract Part 2 and Part 2b, and §6 adds results §3.2
    (`docs/design/principia_results.md` § "3.2 Where it runs: the job"). This entry lists every one §6 names.
  - Test 6 (§5) reads `atlas-params.json`, which "**is held outside the repo** until its data terms are confirmed
    (`docs/incoming/2026-10-26_orbit_catalogues/`, unmerged)".
  - §7's sources are "not yet checked against the papers".
- **Applied meanwhile:** none. R-265, R-572, R-90, the canonical spec, deep_zoom and the integrator contract stand as
  written; no requirement or task is added from the proposal.
- **Waits:** nothing built waits; no task builds the ladder. RQ-578 (deep zoom §2's floor) does not wait on this entry.
- **27 Oct 2026 (the verification and replication plan, PR #305):** [`docs/incoming/2026-10-27_validation_plan/PLAN.md` § 9](docs/incoming/2026-10-27_validation_plan/PLAN.md#9-open-questions-for-the-owner), question 10, is this entry's question and is queued here, not as a new entry: "**RQ-577, the precision ladder: rule before or after M3 closes?** Recommended: after, as the proposal's own §6 question 1 allows; PL-8's first half runs on R-33's reference at M3 without it." The plan also corrects test 6's row count (PLAN.md § 2.2: 937 Hristov rows, not 1,260; a dated note in the proposal). Stays queued (the human, 27 Oct 2026: "The other §9 questions stay queued as RQs."). Not applied.
- **27 Oct 2026 (R-630):** the human: "10. RQ-577 (precision ladder): rule after M3 closes. Leave it queued; PL-8's first half runs on R-33's reference at M3 without it." Stays open until M3 closes; nothing applied.
- **27 Oct 2026 (R-632):** the human: "Unchanged: RQ-577 after M3 (R-630)". Unchanged (R-632); stays open until M3 closes. RQ-578, which this entry did not hold, is closed by R-632: a ~50+ floor needs an f64 sum and integration, on the CPU or under this entry's ladder, after M3.
- **28 Oct 2026 (R-636):** the owner: "Still open, paste verbatim to Malachy: the precision-ladder proposal's seven
  questions with the rulings each would amend; the census draft's five questions; the replication batch's gate/report
  list and the missing data/papers list." Stays open; the orchestrator puts it to the owner verbatim.
- **28 Oct 2026 (R-637):** the owner: "RQ-577: still after M3. Record these as Malachy's leanings on the item, not
  rulings: (1) after M3; (2) Result-only first, live later by choice; (3) Jorba & Zou order rule, order recorded per
  Result; (4) new field, no "measured equivalent"; (5) events stay where every stepper samples them, dense output
  optional and reported separately; (6) rounded fields only unless a checkpoint need appears; (7) per-quad precision
  later, after R-547 covers it, uniform per Result first." Recorded here as the owner's leanings on the seven questions,
  in their order, **not rulings**: nothing is applied and nothing is built. The census's stage-1 results attach to this
  entry as evidence only (R-637, closing RQ-599). Stays open until M3 closes (R-630).
- **4 Nov 2026 (R-706):** deferred to its own session by owner 4 Nov 2026. The entry stays open.
- **Deferred:** deferred to M3 by owner 5 Nov 2026 (R-795, the owner: "RQ-577: defer:M3").
- **7 Nov 2026 (R-821), restated:** the owner: "RQ-577: defer:M3". It stays deferred to M3.
- **Deferred:** deferred to M3 by owner 8 Nov 2026 (R-880, the owner: "RQ-577: defer:M3").

## RQ-643: The collinear statistical targets' stops lie past R-610's horizon cap in the integrator's units (proposed OR02-1, OR03-1, MH89-1) *(physics review of the 29 Oct research-batches ingest, finding F4; `docs/incoming/2026-10-29_research_batches/README.md` § 10.3, § 10.5, `horizons_integrator_units.txt`; context: none built)*

*Drafted 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches (that README's § 10.5); filed 1 Nov 2026.*

- **File, section:** `docs/contracts/principia_integrator_contract.md` Part 5: "a **single global horizon `T` is
  legitimate across every pixel**" and "`T ∈ [0.4, 200]` (R-610, closing RQ-555): … the cap stays at 200 until long
  horizons are measured for cost"; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 8 rule 18: "Respect the
  horizon cap; queue, don't raise it. … A target past 200 (Boekholt et al. 2020's t = 10⁴) is compared only over the
  systems that end before 200."
- **What is settled:** the conversion. Each source's fixed-energy stop, brought to M = G = I = 1 by the inverse-encode
  contract's Part 2, is a per-start horizon: Orlov 2002's t = 100 is 60.3 to 5.94×10⁴, past 200 for 44 of its 99 k
  values; Orlov 2003's t = 1000 is 36.1 to 9.95×10⁵, past 200 in 443 of its 475 (ε1, ε2, k) cells; Mikkola &
  Hietarinta 1989's lifetimes reach 9×10⁷ of their units, and the cap is their t = 0.119 to 334, so Table 3's last
  3/8 lie past it at every start (`docs/incoming/2026-10-29_research_batches/horizons_integrator_units.txt`).
- **What is not:** how rule 18's "compared only over the systems that end before 200" applies here. The three papers
  publish fractions over the whole grid (Orlov 2002 Table 1; Orlov 2003's table, by (ε1, ε2)) or over every break-up
  (Mikkola & Hietarinta 1989 Tables 2–3), not per start, so the source has no subset ending before 200 to compare
  with.
- **Options seen:** (a) a ruling raising the cap for these runs once long horizons are measured for cost, R-610's
  condition; (b) compare only what the cap reaches and the source states, e.g. Mikkola & Hietarinta 1989 Table 3's
  t1–t4 at the starts where they fall inside 200; (c) leave the past-cap parts blocked by the horizon, as MO14-1's
  long-time part is.
- **Applied meanwhile:** (c). The rows state the conversion and which parts the cap blocks; no new cap is chosen.
- **4 Nov 2026 (R-706):** deferred to M9 by owner 4 Nov 2026. The entry stays open.
- **Deferred:** deferred to M9, option (a) only, if the cost measurement allows, with option (b) applied now (R-800) by owner 5 Nov 2026 (R-800, the owner: "RQ-643: (b) now. Revisit (a) at M9 if the cost measurement allows.").
- **Deferred:** deferred to M9 (R-823) by owner 7 Nov 2026 (R-823, the owner: "RQ-643: defer:M9").
- **8 Nov 2026 (R-886):** deferred to M9 again, the third time; the owner: "RQ-643: defer:M9".

## RQ-629: WGSL and Vulkan bound `sin` and `cos` only by an absolute error, where the decode's `cos α` near `π/2` needs a relative one *(a note for future backends; REQ-ENC-024, REQ-RENDER-025; R-297; inverse-encode contract Part 4; dd_decoder §3.2; context: RQ-628, TASK-M2-26, PR #343)*

*Queued 30 Oct 2026 from TASK-M2-26's fix round, as RQ-628's decision directs. No failure today; nothing waits.*

- **File, section:**
  - The orchestrator's decision on RQ-628 (30 Oct 2026, applied per R-369), its last bullet: "Related, filed
    separately: WGSL/Vulkan bound sin/cos only by an absolute error of 2^-11 on [-pi, pi], yet cos(alpha) near pi/2
    needs relative accuracy. There is no failure today; this gets a REVIEW_QUEUE note so a future backend doesn't
    surprise us. It is out of (a)'s scope."
  - `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi":
    "`ρ̃ = R̃ cos α · (1, 0)`", and "`‖ρ̃‖ = cos α`, so **small α = LARGE inner-pair separation**; `α → π/2` = tight
    inner binary + distant third body (hierarchical)."
  - `docs/contracts/principia_inverse_encode_contract.md` § "Part 4 — Conditioning: assert in physical units, never in
    z": "`ε_phys = k·ε_R` … one `k` for both precisions: the interior residual is rounding, the same count of `ε_R` at
    f64 and f32", and "the `ROUNDTRIP` preset (Part 8) use[s] it unchanged".
- **What is seen:** the fragment decode forms `‖ρ̃‖` from the backend's `cos α`. WGSL, and Vulkan for GLSL.std.450,
  bound `sin` and `cos` only by an absolute error of `2⁻¹¹` on `[−π, π]`. Near `α = π/2`, `cos α` is small (0.034 for
  RQ-628's light body), so an absolute bound allows a relative error in `‖ρ̃‖` of `2⁻¹¹/cos α`, far past `ε_phys`, as
  `atan2`'s did in RQ-628. The backends in use do much better than the bound: RQ-628's check that moved each
  other builtin, `sin` and `cos` among them, by 8 `ε_R` left the worst sample below 0.27 `ε_phys`, and with the
  kernel's own `atan2` qa's `qa_m226_latent_slices_dark_and_within_eps_phys` passes on Metal (lavapipe's run is PR
  #343's CI).
- **The gap:** nothing in the corpus bounds a backend's `sin` and `cos` relative to their value, which `ε_phys`
  assumes at the fragment's decode. RQ-628's option (a) replaced only `atan2` (the kernel's own, in integer
  arithmetic); `sin` and `cos` stay the backend's.
- **The question:** whether a future backend (another driver, WebGPU in a browser) is held to a relative accuracy for
  `sin` and `cos` near `π/2`, or the decode's `cos α` is formed so that it needs none (for instance as `sin(π/2 − α)`
  from an exact `π/2 − α`, or by the kernel's own function, as `atan2` now is).
- **Options seen:** (a) nothing now: a note, acted on when a backend fails ROUNDTRIP's `ε_phys` this way; (b) a
  per-backend check of `cos` near `π/2` against f64 in the GPU suite, so a new backend that does worse is caught
  before ROUNDTRIP's acceptance; (c) the kernel's own `sin` and `cos` at f32, as `atan2` (a larger change, beyond
  RQ-628's scope).
- **Applied meanwhile:** none; nothing is built for it.
- **Waits:** nothing.
- **4 Nov 2026 (R-696):** deferred to M4 by owner 4 Nov 2026, with RQ-687's option (a), decided on the cross-backend check. The entry stays open.
- **Deferred:** deferred to M4 by owner 5 Nov 2026 (R-796, the owner: "RQ-629: defer:M4").
- **Deferred:** deferred to M4, option (c) with RQ-687 on the cross-backend numbers, option (b) built at M4 by TASK-M4-27 (R-822) by owner 7 Nov 2026 (R-822, the owner: "RQ-629: (b) at M4; (c) ruled with RQ-687 on the cross-backend numbers. defer:M4
  RQ-679: defer:M8
  RQ-687: defer:M4
  RQ-696: (c).").
- **Deferred:** deferred to M4 by owner 8 Nov 2026 (R-881, the owner: "RQ-629: defer:M4").

## RQ-637: Near a zero `E_0` or `L_z`, `symlog`'s neutral band can't give what ruling (d) also asks, an exactly neutral fresh sample: the band is relative to the reference, and the rounding isn't *(physics; R-652, the owner's ruling (d); `docs/contracts/principia_render_contract.md` § "Part 5", "The neutral band"; physics review 5723029285 of PR #360, Q1; REQ-TOOL-149; context: TASK-M3-05)*

*Queued 31 Oct 2026 by TASK-M3-05's fix round. For the owner: yes, it is physics, it changes which real drifts a view
shows. Code unchanged; TASK-M3-05's PR doesn't wait on it, the ruling's band being built as ruled.*

- **File, section:**
  - `decisions.md` § R-652, the owner's ruling (d): "give symlog a neutral band of k·f32-ulp relative to |E₀| (and |L| for
    L_z), with k justified in writing (rounding-level differences aren't physics)".
  - The same ruling: "add a test that a fresh sample renders exactly neutral on every backend".
  - `docs/contracts/principia_render_contract.md` Part 5: "At or near a zero reference the band narrows to nothing".
- **The conflict:** a fresh sample's rounding scales with `K + |V|` for the energy and `S = Σ(|xᵢp_{y,i}| + |yᵢp_{x,i}|)`
  for `L_z`, not with `|E_0|` or `|Lz_0|`. Near a cancellation the band is narrow while the rounding is not, so both can't
  hold. `drift_view_neutral_fresh_sample_renders_exactly_neutral` passes on the samples inside its stated domain (R-652,
  "The test's domain"), at least 4 of 8.
- **Evidence:**
  - It errs toward showing a tint: the band never hides more than `k·2⁻²³·|reference|`.
  - dd test 2's fresh figure-eight has `L_z ≈ 0` with moving bodies, so its band is empty and it is outside "exactly
    neutral". The physics reviewer's estimate, not a render: a fragment rounding of about 1e-7 at `ε_L = 1e-6` gives
    `s ≈ 0.1`, `t ≈ 0.54`, a visible tint that depends on the backend.
  - In the other direction, RK4 at f64 on dd test 7 (ΔE −9.65e-7, relative 5.6e-6) is true truncation drift and now reads
    neutral under `symlog`; `log` has no band and shows it, and `dE_max` holds it.
- **The question:** whether the band stays relative to the reference, or scales by the rounding itself.
- **Options seen:** (a) keep it as ruled, with the limit recorded (render contract Part 5); (b) scale the band by the
  rounding, `k'·2⁻²³·(K + |V|)` and `k'·2⁻²³·S`, with `k'` from the bound: 15 with a CPU reference, 22 with a GPU one for
  the energy; 5 and 6 for `L_z`. This closes the gap near zero and hides less at small `K/|E_0|`, where 128 is 8.5 times
  the bound; it departs from the ruling's words;
  (c) another, stated by the owner.
- **Applied meanwhile:** (a). Nothing waits.

## RQ-638: `symlog`'s neutral band is in the three drift debug views; a field ramp on `energy_drift` or `Lz_drift` has none, so a fresh sample's rounding still shows as colour there *(physics; R-652, the owner's ruling (d); `docs/design/principia_colour_composition.md` § "6. Debug views as presets"; code review 5723219417 of PR #360, C2; REQ-TOOL-149; context: TASK-M3-05)*

*Queued 31 Oct 2026 by TASK-M3-05's fix round. For the owner: yes, it changes what a real stain shows. Code unchanged;
the scope is stated in render contract Part 5 and R-652, applied per R-369, and TASK-M3-05's PR doesn't wait on it.*

- **File, section:**
  - `decisions.md` § R-652, the owner's ruling (d), its reason: "every t = 0 pixel is a fresh sample, so Metal's ulp noise
    rendering as colour would appear on real data too. Fix the view."
  - `docs/design/principia_colour_composition.md` § 6: "A field view *is* `colour = FieldRamp{field, recommended ramp}`",
    "`f_edrift` = `energy_drift · diverging · symlog`".
- **The gap in the code:** `render::colour::defaults::default_ramp` gives these fields `Compaction::Diverging { x0 }`,
  which is `compact_symlog(x, x0)` in `crates/render/shaders/wgsl/lib/compaction.wgsl`, and `qa_field_either_role_any_field`
  (`crates/render/tests/qa_TASK-M7-05.rs`) renders a field ramp on `energy_drift`. No band is on that path, so a stain's
  field ramp still draws a fresh sample's rounding as backend-dependent colour at `x₀ = 10⁻⁶`. No golden exercises it.
- **The question:** whether "Fix the view" reaches the field-ramp path.
- **Options seen:** (a) extend the band to it, where the drift is read, since `compact_symlog` has no reference and also
  serves `E_0` and `Lz_0`, with the fresh-sample test extended; (b) leave it there, the band being the debug views' alone,
  a field ramp showing the rounding; (c) another, stated by the owner.
- **Applied meanwhile:** (b), the code unchanged, the scope stated. Nothing waits.

## RQ-681: TASK-M9-01: no task builds the scoring runner it calls, AT-D6's twins and slice planes are undefined, and OC-2's state and precision are unsettled *(silence, R-72; R-71; R-649, R-650; REQ-VAL-214 to REQ-VAL-217; dd_replication § 4.3, § 8; context: TASK-M9-01, TASK-M3-27, RQ-660)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flight of TASK-M9-01. Nothing is chosen.*

- **File, section:**
  - (a) `plan/tasks/M9/TASK-M9-01.md`:35-38 call `cargo xtask gate replication --target OC-2` (and `AT-D6`, each with
    `--propose`); `plan/tasks/M3/TASK-M3-27.md`:38-39 call `--target PL-8a`. `docs/design/principia_dd_replication.md`
    :712-713: "**The gate** is one command, `cargo xtask gate replication --target <id>`, with `--propose` for a
    calibration's proposal and `--render` …", and :714-716 (R-649, R-650): "it reports the outcome (§ 12.3) and blocks
    nothing". `xtask/src/main.rs`:45-48: "gate (<gate> | --all | --list) … fails naming each input whose outcome is not
    its expected one"; `xtask/src/gate.rs` takes no `--target` or `--propose`.
  - (b) `plan/requirements.yaml`, REQ-VAL-216: "with scaled twins (rows equal up to a rescaling) landing on one point …
    each slice family's residual off its fitted plane printed". `docs/incoming/2026-10-27_validation_plan/PLAN.md`:249
    cites "`ATP` D6, §5 steps 2–3", and :50 gives `ATP` as "`/mnt/user-data/outputs/atlas_into_principia_plan.md`
    (**outside the repo**)".
  - (c) `plan/requirements.yaml`, REQ-VAL-214: "recomputed from its initial conditions … must match the row's catalogued
    `E_cat` and `L_cat` to their printed precision"; REQ-VAL-215: one tolerance on E and L.
    `docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md`:24-25: "`pub_*` as the source
    paper published them and `ref_*` as the site re-converged them. E and L are recomputed from `ref_*` as a check on
    the layout." RQ-660 measured on two Rose rows that `L_cat` is the refined state's (Σ m(x·vy − y·vx)
    4.841466815563855e-05 and −3.7699259047840883e-06 from `ref_*`, 0.0 on both from `pub_*`).
- **Measured, 3 Nov 2026,** on `threebodyorbits_ics.csv` (3,943 rows): `E_cat` is printed with 1 to 8 decimals (1 on
  343 rows, all −4.5 or −1.0; 6 on 2,872). `L_cat` is "0.0" on 402 rows, a plain decimal of 3 to 8 places on 434, and in
  exponent form on 3,107 (magnitudes 8.2e-35 to 4.8e-5, up to 17 significant digits). E and L recomputed (G = 1, the
  masses as given) on all 3,943 rows: from `ref_*`, E matches `E_cat` to at most 4.9e-7 relative on every row, and the
  worst |L − `L_cat`| is 5.0e-6; from `pub_*`, typical rows are off by about 1e-7 relative in E, but the worst is off by
  2.8e-1, 16 rows by more than 1e-3 (family V 2, li_IIC 2, III 2, IVa 3, IVb 2, IVc 2, simo 1, II 1, sheen 1), and the
  worst |L − `L_cat`| is 2.3e-2.
- **The gaps:** (a) no task's deliverables build the replication scoring command, with `--target`, `--propose` and the
  three outcomes; (b) which rows are scaled twins (which rescaling, found how), and the fitted plane's coordinates and
  fit, are undefined, and their source is outside the repo; (c) whether OC-2 recomputes from `pub_*` or `ref_*`, and what
  "printed precision" means for a value printed to one decimal or in exponent form, against one tolerance.
- **Options seen:** (a) a deliverable in TASK-M3-27 (the earlier milestone that calls it) or TASK-M9-01, or a new owning task;
  (b) definitions written by TASK-M9-01 (R-72), physics-reviewed, or the ATP draft brought into the repo as a record;
  (c) `ref_*`, the state RQ-660 found `L_cat` matches, or `pub_*` with the drift reported (on the 16 rows above, more
  than refinement drift: up to 2.8e-1 relative in E); and a tolerance per row from its printed digits, or one tolerance
  with the coarse rows reported apart, or an absolute tolerance on L (most `L_cat` are numerically zero, 3,107 in
  exponent form at most 4.84147e-5 in magnitude and 402 reading 0.0, where a count of printed digits means nothing).
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M9-01; TASK-M3-27 for (a).
- **4 Nov 2026 (R-706):** deferred to M9 by owner 4 Nov 2026. The entry stays open.
- **Deferred:** deferred to M9, parts (b) and (c), with part (a) ruled, a deliverable of TASK-M3-27 (R-794) by owner 5 Nov 2026 (R-794, the owner: "RQ-681: (a) as a deliverable of TASK-M3-27, the earlier user. Parts (b) and (c) stay deferred to M9.").
- **Deferred:** deferred to M9, its remaining parts, with part (c) state ruled ref_* (R-824) by owner 7 Nov 2026 (R-824, the owner: "RQ-681: (c) ref_*, rerun the figures at M9; the rest defer:M9.").
- **Deferred:** deferred to M9 by owner 8 Nov 2026 (R-887, the owner: "RQ-681: defer:M9").

## RQ-687: Whether the kernel takes its own f32 `sin` and `cos`, as `arctan.rs` does for `atan2` (RQ-628), given that it can change CPU bits *(physics; a decision that changes what is built; R-297, R-649; REQ-DEC-043; RQ-628, RQ-629; issues #390, #393, #401; context: the Windows RTX 2080 Super run, RQ-685)*

*Queued 4 Nov 2026 by the orchestrator's filing task, from the Windows GPU run (#390), as #393 says it must be: "That
can change CPU bits, so it goes to `REVIEW_QUEUE.md` for the owner's decision." For the owner. Nothing is chosen. The
run's record, the 13 failures and the hardware-health arm are in RQ-685.*

- **File, section:**
  - `crates/kernel/src/chart/shape_sphere.rs`:162: `let k = select(r == F::zero(), F::one(), Float::sin(r) / r);`. The note
    at :52-55 claims each select "moves the output by rounding only". `crates/render/wgsl/decode.wgsl`:745 is the same
    line in the fragment, generated from `crates/kernel/src/stage/fragment.rs` (its header), so it follows the source.
  - Every f32 `sin` and `cos` that reaches the GPU, through `stage/decode.rs` and `stage/fragment.rs`:
    `shape_sphere.rs`:147 and :149-151 (`spherical`), :161-162 (`exponential`; `cos(r)` at :161 is under the same
    absolute bound), :225-226 (`inverse`); `chart/burrau_acute.rs`:72 (`nu_of`, which the Burrau θ charts' decodes call);
    `decode/config.rs`:62-65. Only :162 is measured to fail.
  - Not on the GPU: `chart/tilt.rs`:356-357, :376 and :479, f64 at this commit, which no stage takes; `shape_sphere.rs`:589-592,
    `hammer_forward`, f64.
  - `crates/kernel/src/arctan.rs`:1-3: "at f64, the CPU's alone, libm's `atan2`, unchanged bit for bit; at f32, the GPU's
    precision, [`atan2_f32`], the kernel's own, in integer arithmetic."
  - `REVIEW_QUEUE.md` RQ-628, the 30 Oct note: decided by the orchestrator per R-369, option (a), which replaced `atan2` at
    f32. No ruling in `decisions.md` names RQ-628.
  - `REVIEW_QUEUE.md` RQ-629, still open: "WGSL, and Vulkan for GLSL.std.450, bound `sin` and `cos` only by an absolute
    error of `2⁻¹¹` on `[−π, π]`", and its option "(c) the kernel's own `sin` and `cos` at f32, as `atan2` (a larger
    change, beyond RQ-628's scope)". Its "No failure today" no longer holds, though this failure is `sin` near 0, not the
    `cos α` near `π/2` RQ-629 is about.
- **The note's claim fails for `rho_angle`:** the failure below is a move of π/2, not rounding. By #393's diagnosis
  (inferred) it is R-437's `p = q = 0` select, taken on the card and not on Metal or lavapipe, on identical inputs with
  the setting off. The cause is `sin`, so RQ-685's division measurement doesn't cover it; its reading (b) does. How the two
  rulings depend on each other is stated once, in RQ-685.
- **Measured (#393):**
  - `qa_m225_kernel_rho_angle_at_the_lambda_pole` on the card: "n₀ = [1.0, 0.0, 0.0], φ_f = 0 at [0.5, 0.5005]: the kernel
    decode stage's rho_angle 0e0 is 8388608 eps from Φ's f64 angle 1.5707963267948966e0, over REQ-DEC-043's 256". The line
    is identical on 6 of 6 runs.
  - Passes on lavapipe (main runs 41692855335, 41683257838) and on Metal (local job 1380).
  - With the sinc in the built `kernel.wgsl` replaced by `1f` at its two sites, a temporary edit to the gitignored
    build output, restored and hash-checked, the test passes on the card. On Metal, forcing that one `sin` to 0 gives the
    card's exact message. Unedited, `decode_preset_shape_poles` passes on the card.
  - Not verified: the card's `sin` near 0 itself; whether the final SPIR-V keeps `Sin`. The diagnosis's reading is that
    `sin(1.00005e-7)` returning 0 conforms to the absolute bound; the isolation is by the edit, and the edit is not a fix.
  - **Past `[−π, π]` (#401).** `spherical` has `θ = 2π·s` (:144-145), up to 6.2831855 (`2·P::PI()`), on the user-reachable
    map, outside the range RQ-629's bound covers; from memory, unchecked, the specs set no bound there.
    `decode_cross_backend`, `stage_decode_shape_sphere_defaults n₀ = ±1 poles`, η_E on: worst 28408617.97 eps against f64
    and 28408618.00 against native f32, `rho_angle`, at sample 2 (1, 0.5), identical on 3 of 3 runs. The row is outside
    REQ-DEC-043's domain, reported, not held.
  - **That sample is a pole.** `crates/engine/tests/decode_cross_backend.rs`:928-932: "`n₀ = +1` and `λ̃ = 0`", reached
    only to each precision's rounding, "and `1/ρ` or the angle of a rounding-sized `λ̃` amplifies it". True `‖λ̃‖` there is
    about 9.0e-8 (computed), so these eps are the direction of a vector of that length, not a measure of how far the
    trig is off. Metal's worst in the same domain is at the same sample: 139986730.45 eps against f64, `rho_ratio`.
    Inferred from that ratio: Metal's `‖λ̃‖` is about 18 times shorter than true, so `sin` and `cos` aren't relatively
    accurate there on Metal either. Lavapipe's worst is 723878.23 eps (`K_0`), at (0.5, 0.5).
  - **A correction to #393's diagnosis (comment 6622072753).** #393 reads that line as: the card "returned π̂ for
    `rho_angle`, forcing `sin(6.2831855) < 0` and `cos(1.5707964)` near 0", and says "the card's trig is near-flat there".
    - **The true values,** computed in f64 on those f32 arguments: `sin(6.2831855)` = +1.7484556e-7 and `cos(1.5707964)` =
      −4.371139e-8. The argument is `2·P::PI()`, from π's nearest f32, and lies above 2π. It isn't twice the corpus's π̂,
      rounded toward zero (R-417), where `sin` is about −3.0e-7. π is taken at the parameters' f32 type
      (`shape_sphere.rs`:75, :292), so true trig gives `rho_angle` −0.2449786631 at f64 and −0.24497867 at f32.
    - **"Returned π̂" checks.** `error` scales by `max(|x64|, 1)`, and π̂ = 3.1415925 (`pi_image`,
      `crates/kernel/src/decode/descriptor.rs`:116-121, `angle`'s upper limit) reproduces both figures, 28408617.97 and
      28408618.00.
    - **The path to `sin < 0`, inferred.** `rho_angle` isn't `β`. `inverse` forms `p` from `sin θ` and `q` from `cos φ`,
      takes `β` from the kernel's integer `atan2(q, p)`, then the backend's `sin ψ` and `cos ψ` (:226), and `angle` takes
      the backend's `Float::atan2` (`descriptor.rs`:144). π̂ needs `cos ψ < 0`, so `|β| > π/2`, so `p < 0`: a negative `sin`
      where the true one is positive. #393 doesn't say that sign is wrong.
    - **It also needs `|q| ≪ |p|,** about 1e-7 relative, up to the card's own `sin ψ`. The true `cos` won't do: at `sin` =
      −1e-7 it gives `rho_angle` −2.7295. So either the card's `cos(1.5707964)`, an in-range argument, is far smaller than
      the true value, or its `sin(6.2831855)` is far larger in magnitude. The line can't tell which.
    - All of this stays inferred: the card's `sin`, `cos` and `atan2` were not read.
    - **What is left of #393's reading.** This corrects its wording. Its conclusion stands only as: the card's values
      there differ from the true ones by at least the sign of a 1.75e-7 `sin`. The size of the error isn't bounded by this
      line, and "near-flat" isn't shown. Whether the range is the cause isn't shown either: the wrong sign takes an
      absolute error of only 1.75e-7 or more, about 0.04% of the in-range bound `2⁻¹¹`, and the `sin` here is near 0 as well.
  - Related and unsettled (#400): `shape_sphere` fuzz, worst 52.05 eps vs f64 on the RTX, 28.55 on Metal, 17.12 on
    lavapipe, against a tolerance of 256. This `sin` is one candidate cause and RQ-685's division another; the logs can't
    separate them.
- **Reach:** `ShapeMap::Exponential`, the map that failed, is built only in `validation` and tests, so that failure is not
  reachable by a user, as traced in #393. The spherical map has the same exposure at :149-151 and is reachable, but its
  exact poles aren't pixel centres; that stays open, as does how far deep zoom reaches it. On that map `θ` runs outside the
  bounded range; what #401's line shows at its sample is stated above. Beyond the decode, the physics reach is
  unmeasured, as RQ-685 says.
- **What each option would change:**
  - (a), built as `arctan.rs` is: at f32, on every backend, the decode's `sin` and `cos`; f64 keeps libm's bits, and the
    CPU's f32 bits change to the kernel's. Its per-pixel cost in compute and fragment is unmeasured. A generated table
    would bring a calibration of its own, as REQ-GEN-044 did for `atan2`.
  - (b): `exponential` is shared generic `F` code, so a new form changes f64 and f32 CPU bits near 0 unless it is gated
    by precision. `cos(r)` at :161 stays under the same bound.
  - (c): no bits change.
  - Which fixtures and tolerances record those bits has not been checked.
- **The question:** does the kernel take its own f32 `sin` and `cos`, given that this can change CPU bits?
- **Options seen:** (a) yes, both, in integer arithmetic as `arctan.rs`, for every GPU site above; (b) only the sinc: a form
  of `sin r / r` at `shape_sphere.rs`:162 that doesn't read the backend's `sin` near 0, `decode.wgsl` following from its
  source, leaving the same class at the other sites; (c) neither: #393 stays a platform finding, and RQ-629's option (a) or (b) stands; (d)
  another, stated by the owner.
- **Applied meanwhile:** nothing. Every assertion stays, and nothing is tuned or loosened (R-649).
- **Waits:** nothing in the build loop: the test passes on CI's lavapipe. RQ-629's options wait with this one.
- **4 Nov 2026 (R-696):** option (b) ruled: a sinc form at `shape_sphere.rs`:162 that never reads the backend's `sin` near 0, gated to f32 (REQ-DEC-053, TASK-M2-53). Option (a) deferred to M4 by owner 4 Nov 2026, decided on the cross-backend check (TASK-M4-27 and TASK-M4-28, R-686); the entry stays open for it.
- **Deferred:** deferred to M4's cross-backend measurement, option (a), with option (b) ruled (R-797) by owner 5 Nov 2026 (R-797, the owner: "RQ-687: (b) now, as ruled on 4 Nov: fix the sinc site, gated to f32. Option (a) is deferred to M4's cross-backend measurement.").
- **Deferred:** deferred to M4, option (a) on the cross-backend measurement (R-822) by owner 7 Nov 2026 (R-822, the owner: "RQ-629: (b) at M4; (c) ruled with RQ-687 on the cross-backend numbers. defer:M4
  RQ-679: defer:M8
  RQ-687: defer:M4
  RQ-696: (c).").
- **Deferred:** deferred to M4 by owner 8 Nov 2026 (R-883, the owner: "RQ-687: defer:M4").

## RQ-738: RQ-736's correction list and the conflicts the filter order leaves *(carried from RQ-736, closed by R-853 on 7 Nov 2026; the owner's model against the corpus; a one-way door where an item changes what is built; context: TASK-M4-34, TASK-M8-120, TASK-M4-37)*

*Filed 7 Nov 2026 by the rulings PR of that day. RQ-736 closed on what the owner ruled (R-853, R-855); these two lists were not answered by any of it, so they move here unchanged, so they aren't lost. Nothing in them is applied but the "reading held" each states.*

*In plain English: the owner ruled that the measured and ruled documents win over the chart-space model, and the model was corrected. A few of the model's sentences still say otherwise, and the owner's symmetry filter order meets the corpus in nine places. Each is listed with what stands meanwhile.*

- **File, section:** `docs/archive/review_queue/M0.md`, RQ-736, from which both lists are moved word for word; each item quotes its own files and sections.
- **Applied meanwhile:** what each item states as standing meanwhile, or its "reading held"; nothing else.
- **Not settled by the revised model: a correction list for the owner.** What stands meanwhile on each: the corpus and § 17. The model's file is untouched in PR #422 beyond the owner's copy. Recommended on each: correct the wording at the next revision; only item 1 has options.
  1. **Which tilt.** Model § 0.2: "**Affine tilt is the ruled lean-each-axis map (R-474), not a Grassmannian geodesic.**" `docs/contracts/principia_chart_decoder_contract.md` § "Two tilt maps": "**Tip as sheet**, the default (`sheet`)", "**Lean each axis**, an option (`lean`)", with the sheet "turned rigidly". So the default tilt is a rigid rotation, and lean is the option. (a) **Recommended.** § 0.2 reads "R-474's two tilt maps, each tested against its own formula", which the contract and `path_tilt_bitwise` already say. (b) Lean each axis becomes the default, which amends R-474.
  2. **Body text § 0 overrides but doesn't rewrite.** Model § 1: "Every chart Principia has fits this"; § 2: "**Rule: always blend in `Z`, never in physical coordinates.**"; § 7: "**Proved** (Thm 1): requires blending in `Z` and the decoder being total"; § 9: "blend in latent space, never in physical space"; § 8: "never the morph itself, which is continuous in `Z` (Thm 1)". § 0: "\"Always blend in Z\" is withdrawn."
  3. **`L_z`'s sign, still in the body.** Model § 6b, Theorem 11's limits: "(`L_z²E`, the signs of `E` and `L_z`)"; § 7: "the scale-free L_z²E and the signs". § 0.5: "`L_z²E` and the sign of `E` only". Also code F3: the model's "`T = 0` recovers the ordinary chart" holds within `ε_phys`, not bitwise (model § 8: "`E(D(z)) ≠ z` bitwise").
  4. **Held values mid-morph (physics B5, the rest).** Model Corollary 2: "Mid-morph frames must not claim a held value." Right for the corollary's mixed case; too wide for one `E*` at both ends, which design § 17.11 holds. Corollary 1's "interpolated as a display choice": design § 17.11, a blank end "resolves at the end frame" (R-470).
  5. **`E`'s discontinuities (physics B2, the rest).** Model § 8: "The encode `E` has genuine discontinuities: the mirror deadband (`λ̃_y = 0`) and collision degeneracies." Filed under floating point, and they're exact-arithmetic facts.
  6. **Physics B6.** Model § 6b: "many-to-one, with no inverse even locally"; § 7: "**Impossible**: many-to-one". Every axis quantity is many-to-one on 8D, and where `d_min` or `t_end` is smooth the implicit function theorem gives a local inverse. The true reasons are discontinuity, fractal level sets and no tractable section. The exclusion stands as a design rule: "excluded" for "Impossible".
  7. **Physics B7.** Model Theorem 11's limits: "`φ_T` is continuous only where the backward trajectory survives `[0, T]` without a triple collision or an escape." Escape doesn't break the flow's continuity, and binary collisions regularise, so that mask is an app limit. "All of §2–§6 applies" needs "off the mirror seam" if the start goes through `Enc` (design § 10.3).
  8. **Physics B8.** Model Theorem 3: "same endpoints and same tangent at `s = 0`" holds to `O(θ²)` only; "0.197 at `τ = 1.2`" is in no file of the repo, and design § 17.3 gives 0.016 median, 0.124 worst.
  9. **Physics B9.**
     - Model § 3: "`Q' = [e₂ e₁]`, which is the same plane with its axes swapped (a 90° rotation in the plane plus a mirror)": not a tilt in `(q_i, t)` but `R(π)` in another plane. The stronger fact is that every tilt fails: the midpoint area of `L` is `cos(θ/2)`.
     - Model § 3, "Pan by `a`", "Zoom by factor `k > 0`": pan and zoom-out leave `U`, so `φ` needs Part 3's extension.
     - Model § 3b: "every theorem holds on `U \ Σ_φ`"; § 7: "Continuity proved everywhere": a morph needs `Σ_φ ∪ Σ_ψ`.
     - Model § 3b: "This is a path whenever `A` is continuous along it": `A` depends on `s`, and a blend can lose rank: `(K, E) → (U, E)` gives axis 1 `= E/2` at `s = ½`.
     - Model § 8: "Seams belong to charts, not to paths": fails for that family.
     - Model § 9: "the target's nearest admissible point": nearest holds only for coordinate constraints.
     - `E` names both the energy and encode (model § 2, § 8).
- **Conflicts the filter order and the ruling leave, none resolved here. What stands meanwhile on each: the design's reading as stated, unbuilt.**
  1. **R-544's words.** `decisions.md` § R-544's title: "no hemisphere toggle, no remap and no fold at the equator"; `docs/contracts/principia_chart_decoder_contract.md`: "nothing is folded or hidden", "The shape sphere has no one-hemisphere view (R-544)". Against the order: "the half shape sphere at rest come out of this filter". The filter is such a toggle, as a view. Reading held: the owner's later order governs and R-544's words want amending in the rulings PR.
  2. **RQ-714 and what it touches.** Its (a), "A mask", agreed in principle; `docs/design/principia_dd_validation_orbits.md` § 1.6: "A point outside it is not a region-D start, and the import refuses it."; the lowering Appendix: "region D bounds", and its row "physical (m,r,p) construction → **canonicalise entry**"; REQ-CHART-095 and REQ-CHART-100. Against "region D presets render the full disc". Reading held: the import's refusal stands (an import isn't a chart), and the preset's domain is RQ-714's question, answered by the filter order. On the Appendix's row, reading held: the entry is the one seam, taken with `C`'s step 3 not taken, as R-544 takes it for the shape sphere; identical on D.
  3. **"Full disc" (corrected by physics review 5812620255, P2).** Eq. 8's D is `ξ ≥ 0, η ≥ 0, (ξ + ½)² + η² ≤ 1`, area 0.307. Its four images under the mirror and the 1↔2 swap are the lens `(|ξ| + ½)² + η² ≤ 1`, area 1.228, four times D. At equal masses at rest D is the fundamental domain of the mirror × S₃, twelve elements, under the gauge `A₁A₂ = 1`: the lens is where 1–2 is the longest side, and every rest point outside it is the 1↔3 or 2↔3 image, rescaled, of a lens point. A square chart's box `ξ ∈ [−½, ½]`, `η ∈ [−√3/2, √3/2]`, area 1.732, 5.64 times D, has corners outside the lens. Options, none picked here:
     - (a) The lens, the box's corners masked: the filter with the mirror and the 1↔2 swap gives D. It is a mask, which the order removed.
     - (b) The box drawn whole, the mirror and the 1↔2 swap declared: the filter leaves the box's quadrant `ξ, η ≥ 0`, not D. `(0.5, 0.8)` has `(|ξ| + ½)² + η² = 1.64`: outside D, and all four images outside the lens, so never blanked.
     - (c) "Full disc" read literally, any window holding the lens, the mirror and all three swaps declared, as the order's "body-label swaps where the masses make them exact" allows: every pixel outside the lens has its partner in the domain, the filter gives exactly D with no mask, and the order's words hold as written. At eq. 3 and at unequal masses only the mirror holds (conflict 4, § 8.8).
     - What stands meanwhile: nothing built; design § 8.4 states the three.
  4. **Eq. 3's velocities.** "Region D's quarter disc … come out of this filter", but the near-rest start of `principia_dd_validation_orbits.md` § 1.6 has `ẋ ≠ 0`, so the swap's image isn't on the chart and the correctness rule keeps `ξ < 0`. AA-2 and AA-7 run eq. 3, and AA-3 and AA-5 do by RQ-714's (3)(a). So on those presets the filter gives `η ≥ 0`, not D. Options: (a) `ξ ≥ 0` shown by the label `reduced domain` of the ruling above, a separate crop; (b) the half is drawn; (c) the eq. 3 chart declares the swap as approximate. Recommended (b), since it needs no exception to the correctness rule.
  5. **"Bitwise".** The order's test: "filtered + its mirror reproduces the full chart bitwise". `docs/design/principia_chart_reference.md`: "bitwise where the integration is mirror-exact, else to rounding (R-544 §4) … the two pixels of a pair decode to mirror images only to the decode's rounding". Ensemble copies' offsets aren't mirror-symmetric either. Reading held: the test's fixture decodes its pairs to exact mirrors, one copy; the alternative is a pixel-by-pixel comparison from the exactly mirrored state `Π·x`.
  6. **The angle pair and Parischewsky's pair.** The ruling: the presets "that inherit its limit (Anosova half-disc, Parischewsky, the angle pair)"; the order: "region D presets render the full disc". Their axes are unsigned, so neither has a partner or a full disc. Reading held: part 2's label stands for those two, as `reduced domain`, with no filter and no one-click full disc.
  7. **R-408's statistics on a filtered view.** The order: "a display option applied after rendering", so the legend's shares stay those of the computed chart, while the papers' fractions are of D. Reading held: shares of the computed chart, a paper's own fraction by `in_region_d` (design § 6.4).
  8. **Order of building.** TASK-M4-34's preset and TASK-M4-06's golden are M4, the filter M8. Reading held: the preset renders the window conflict 3 rules, D as a contour, until TASK-M8-120 lands, and the golden reads `in_region_d`. It inherits conflict 3's choice: on (a) a mask is built in M4 after all, on (b) and (c) none.
  9. **Unequal masses.** The ruling: "state what the preset does there (RQ-728 links)". RQ-728 settles the labelling; no document says how a paper's D at a Li & Liao mass set is shown once the mask is gone. Reading held: a contour on the half the mirror leaves (design § 8.8). A gap.
- **Waits:** TASK-M4-34's preset and TASK-M8-120 (conflicts 3 and 4, through RQ-714's open part); the correction list waits for the owner's next revision of the model.
- **8 Nov 2026 (R-868):** the owner: "4. RQ-738: the model's wording follows R-474 (tip-as-sheet default, lean-each-axis the other map); I'll correct the model." Their corrected model of that day replaces `docs/design/principia_chart_space_model.md` byte for byte. That settles the correction list's item 1, as its option (a): § 0.2 now names R-474's two maps, Tip as sheet the default. **Still open:** the correction list's items 2 to 9, which the owner's copy doesn't change (its other changes are § 0.6, R-865, and a new § 0.7, R-866 and R-867), and all nine conflicts the filter order leaves. The entry stays open for them.
- **8 Nov 2026 (R-878, R-871):** the owner: "RQ-738: the readings held stand for conflicts 1, 2 and 4 to 9; conflict 3 as in my RQ-714 answer. Items 2 to 9: send me the list and I'll correct the model." Applied: the nine conflicts are settled, 1, 2 and 4 to 9 by their readings held (conflict 1's amends R-544's words and chart_decoder_contract's "no one-hemisphere view", kept as a "Was"), and 3 by RQ-714's answer, its option (c), the full disc read literally (R-871). **Still open:** the correction list's items 2 to 9, which the owner corrects in the model from the list the orchestrator sent on 8 Nov, and two items this port adds to that list: **10.** the model's § 5, "Setting `δ ← 0` restores `φ(0) = a` exactly", and § 7's "Snap-back is exact", against the brief's Q2 (a), snap-back undoing the slice only (R-899); **11.** the model's § 1, "`Z = R^8`", Theorem 1's proof ("`Z = R^8` is convex") and § 4 ("`Z` is convex"), against RQ-750 (c), `z_β` periodic (R-898). Nothing waits on them: the corpus and the rulings stand meanwhile. The entry stays open for items 2 to 11.

## RQ-782: The smallest `r_coll` the collision test actually honours is undefined: below the f32 spacing of the positions it decides "the bodies' f32 positions coincide", not `‖rᵢ − rⱼ‖ < r_coll` *(silence, a missing definition, R-72; a one-way door; integrator_contract Part 7, dd_integrator § 3.6, R-547; REQ-EVT-003, REQ-EVT-004; physics review 5845121432 of PR #474, F2; context: TASK-M3-09)*

*Queued 10 Nov 2026 by TASK-M3-09's fix round (#474), from its physics review 5845121432, finding F2; each quote below
re-read on the PR's head. Nothing is chosen, and #474 decides nothing here; the default `r_coll = 10⁻³` is
unaffected.*

*In plain English: the collision radius `r_coll` is a user slider that the docs say is valid all the way down to 0. But
the collision test measures the distance between bodies from their positions rounded to single precision, whose
spacing near the bodies' usual size (about 1) is about `6·10⁻⁸`. So for an `r_coll` below about `10⁻⁷` the test can
only tell "the rounded positions are the same" from "they differ", and it may call a collision at a true separation
larger than `r_coll`. The closest-approach distance `d_min` has the same limit. Nothing says where this floor is or
what the instrument does below it.*

- **File, section:**
  - `docs/contracts/principia_integrator_contract.md` § "Part 7", "`r_coll` is user-exposed (sim-key), not a fixed
    constant": "A slider from "regularise aggressively" (large) to "refuse to regularise" (→0), all valid, answering
    different questions".
  - The same section, "Coupling to `N_max`": "the usable minimum `r_coll` is bounded below by what the substepper
    resolves before the cap". It names the substep cap's floor, not the positions' resolution.
  - `docs/design/principia_dd_integrator.md` § "3.6 Detectors": "**Collision:** `min_{i<j} ‖rᵢ − rⱼ‖² < r_coll²` …
    Uses the same squared min-separation the substepper buckets on and `d_min` displays (as its √) — one value, three
    consumers."
  - The same file, § "3.3 Substep law — and the determinism pin, made concrete", with
    `docs/notes/principia_gpu_determinism_note.md` § "The discipline", rule 2 (R-547): the squared separation the
    substepper buckets on is formed from each position cast to f32, so it is position-quantised.
- **The silence:** the collision test must read the substepper's `d²` (§ 3.6), and that `d²` is formed from f32
  positions, so its resolution is the f32 spacing at the bodies' coordinates: about `6·10⁻⁸` for `|x|` in `[0.5, 1)`,
  `1.2·10⁻⁷` in `[1, 2)` (the `I = 1` gauge, Part 5). For an `r_coll` below that spacing, a pair collides exactly when
  its f32 positions coincide on both axes, which can be at a true separation larger than `r_coll`, and `d_min`, the
  latch's `√`, cannot resolve below it either. Part 7 calls every `r_coll` down to 0 valid and never states the
  smallest one honoured. TASK-M3-09 (#474) builds the test as § 3.6 says, and for every `r_coll > 0` gives a threshold
  of at least f32's least normal, so coincident f32 positions always collide (physics F1).
- **The question:** what is the smallest `r_coll` the instrument honours as `‖rᵢ − rⱼ‖ < r_coll`, and what does it do
  with a requested `r_coll` below it?
- **Options seen:**
  - (a) State the floor in Part 7 as a definition: below the f32 position spacing at the pair's coordinates,
    `r_coll` means "collision at f32 coincidence", and `d_min` is resolved only to that spacing; the slider stays valid
    to 0, and the floor is recorded with the run.
  - (b) Dispatch refuses an `r_coll` below a stated floor (for example `2⁻²³`, the spacing at `|x| = 1`), or accepts it
    and flags every output of the run as below the collision test's resolution.
  - (c) Another, stated by the owner (for example a collision test on f64 separations, which § 3.6's "one value, three
    consumers" and R-547 currently rule out).
- **Recommended:** (a). It changes no result: it states what the test already computes, keeps Part 7's "all valid"
  slider, and the default `10⁻³` lies four decades above the floor. The measurable alternative is (b), whose check is a
  dispatch test at the stated floor and one below it; its cost is refusing or flagging a slider setting Part 7 calls
  valid.
- **Applied meanwhile:** nothing beyond TASK-M3-09's physics F1 fix: every `r_coll > 0` has a threshold of at least
  `2⁻¹²⁶`, so f32-coincident bodies collide; dispatch accepts every finite, non-negative `r_coll` whose f32 square is
  finite.
- **Waits:** nothing built waits; TASK-M3-09 (#474) does not wait. A task that builds the `r_coll` slider or its
  dispatch-side flag would take the answer.

## RQ-756: Aarseth–Zare's equations of motion, its Levi-Civita inverse, its reference-body rule and its step control are not in the corpus; and the figure-eight gate and the Law 18 bound assume a schedule R-565 and Part 6 replaced *(silence and conflict; R-160, R-159, R-565, R-563, R-19, R-72; REQ-INT-044, REQ-INT-049, REQ-INT-054, REQ-INT-021, REQ-INT-024; context: TASK-M3-07)*

*Queued 10 Nov 2026 by TASK-M3-07's implementer, before any code; each quote re-read on `origin/main` 3338e336.
Nothing is chosen here. In plain English: the task builds the Aarseth–Zare regularisation (a way of integrating
through close encounters by changing coordinates and time), but the rules it would implement are written only in the
imported prin-rs reference files, which are reference and not authority; and two of its tests were measured under an
older way of stepping that later rulings replaced.*

- **File, section:**
  1. **The equations are not in the corpus.** `plan/tasks/M3/TASK-M3-07.md` § Notes: "AZ's reference code, including
     the predictive step limit (`predictive_dtau`) and the landing clamp, is
     `docs/reference/prin-rs/src/integrate/az/driver.rs` (reference, not authority, R-159); the equations are
     integrator_contract Part 2b's (R-160, REQ-INT-084)." But `decisions.md` § R-160: "The Heggie, logH and TTL
     equations, the step control, and the predictive step limit … are written into integrator_contract Part 2b", and
     REQ-INT-084: "Part 2b must carry the Heggie, logH and TTL equations of motion" — Aarseth–Zare is not named.
     `docs/contracts/principia_integrator_contract.md` § "Part 2b", § "The equations and the step control (R-160,
     R-161)" gives none of: the two-pair Hamiltonian (`Γ = A·B·(H − E)` is quoted only as a withdrawn mechanism,
     `docs/design/principia_dd_predictability_horizon.md` § 7.3, and Part 2a's "the `|u|^2 |p_lambda|^2` term"), its
     derivatives, the mass factors `μ₁, μ₂`, the reference-body rule beyond Part 2b's table cell "two pairs, reference
     body from the longest side" (which body, and the tie rule), or the conditioned Levi-Civita inverse (§ 7.3 gives
     only the unstable `u0 = sqrt((|rho| + rho.x)/2)`, `u1 = rho.y/(2 u0)`, and "the conditioned branch", not its
     form or which sign branch it takes). The task's Deliverables say "AZ transcribed from the reference
     (`workbench/tb_az.py`, with the overshoot fix of `workbench/tb_az_overshoot_fix.py`)", which is not normative
     (R-159; `CLAUDE.md` § Authority). The imported set holds all of it (`docs/reference/prin-rs/reference/tb_az.py`,
     `tb_lc.py`, `src/integrate/az/driver.rs`), but nothing rules it into the contract, as R-160 did for Heggie and
     logH.
  2. **Which `dτ` rule the gate figures were measured under.** `docs/design/principia_dd_validation_orbits.md` § 0.1:
     "`dtau = eta·dt_left/(A·B)` is fixed per sub-interval", and its table (4.232e-09 at `eta = 0.001`) is REQ-INT-049's
     threshold. Integrator contract Part 6 (R-563), the entry-cap bullet: "the step-size rule in force sizes each step
     as `η·Δ_sync / (dt/dτ)` from the current state and caps it at the interval's entry size", with Part 2b's predictive
     limit after it — a different schedule from the one the threshold was measured on.
  3. **"After one period" against the grid.** REQ-INT-049's verify: "figure-eight closure after one period"
     (`T = 6.32591398`, § 0). Integrator contract Part 2a § "The sync grid is a fixed quantum of physical time
     (R-565)": "`ADVANCE(state, t_now, t_target, params)` advances to the latest boundary at or before `t_target`; it
     never stops between boundaries", boundaries `t = k·Δ_sync`. Unless `Δ_sync` divides the period, ADVANCE cannot
     end on it; § 0.1's runs used `dt_left = T/n_sync`.
  4. **The Law 18 bound has no formula.** REQ-INT-054's verify: "the substep count per sync interval never exceeds the
     Law 18 bound computed on the fixed τ-schedule"; `docs/contracts/principia_canonical_spec.md` § 9 item 18 and
     R-19 say only "the count bound is on a fixed `tau`-schedule". Under Part 6's per-step rule with the predictive
     limit (`dτ ≤ f·d_min / (|v_rel|_max·A·B)`), a step can be cut without a lower limit, so the count per interval is
     bounded only by the step budget (Part 3's `step_budget_rate`, `N_min`), whose values are calibrations not yet
     proposed (REQ-INT-103, REQ-INT-108).
- **The question:** (1) may TASK-M3-07 transcribe Aarseth–Zare's Hamiltonian and equations of motion, its conditioned
  Levi-Civita inverse, its reference-body rule and its step control from the imported set into integrator contract
  Part 2b, cited (Aarseth & Zare 1974; the prin-rs files), physics-reviewed and confirmed at the M3 gate, as R-160 did
  for Heggie and logH? (2) is REQ-INT-049's gate run under Part 6's per-step rule with the threshold re-measured, or
  under § 0.1's fixed `dτ`? (3) does the closure gate run with `dt_left = T/n` (a gate-only grid) or on the `Δ_sync`
  grid with the period's remainder handled how? (4) what is the Law 18 bound per sync interval: the step budget's
  share of the interval, an entry-cap count (`⌈1/η⌉` plus the cuts), or another?
- **Options seen:** (1)(a) yes, a ruling like R-160 naming TASK-M3-07 and a requirement for it (or REQ-INT-084
  widened); (1)(b) another task transcribes them first and TASK-M3-07 waits. (2)(a) Part 6's rule, the 4.232e-09
  figure re-measured and proposed per R-71; (2)(b) § 0.1's fixed rule as a gate-only mode. (3)(a) the gate's own
  `dt_left = T/n`; (3)(b) the `Δ_sync` grid, closure read at the last boundary at or before `T` against the reference
  trajectory there. (4)(a) the step budget over one interval, `max(N_min, step_budget_rate·Δ_sync)`; (4)(b) an
  occupant-declared count.
- **Recommended:** (1)(a), since the imported set was brought in to be transcribed (R-159) and R-160 is the
  precedent; (2)(a) with (3)(a), so the landing clamp is tested on its own and the threshold is the rule in force's; and
  (4)(a), since R-604 already accounts the budget every step. Measurable alternative for (2): run the figure-eight
  ladder under both rules and record both.
- **Narrowed 10 Nov 2026:** transcription done in PR #471 per CLAUDE.md § Authority (applied per R-369, R-160 the
  precedent; the owner, 10 Nov 2026, of the integrator tasks to take: "Fine: the physics itself. Steppers,
  regularisation, step control, conservation and convergence tests, the CPU reference, and TASK-M2-82 (the trajectory
  interface with its exact fixtures, the Lagrange solution and the figure-eight, R-918), which has no dependencies."):
  integrator contract Part 2b § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step
  control (transcribed)", physics-reviewed in PR #471 and confirmed at the M3 gate. Question (1) is answered by it.
  Question (3) is decided, applied per R-369 as test mechanics: the figure-eight closure gate runs on its own sync
  interval `T/n` with `n = 32`, the reference's comparable figure (`driver.rs:399`, "measured over one period at
  `n_sync = 32`"), so `ADVANCE` lands on the period under R-565's rule that it stops only on a boundary, and the gate
  reports its re-registration count with its result (PIT-4); the gate's grid is a fixture's, and the march's `Δ_sync`
  is untouched. TASK-M3-07 is split: the regularisation axis stays TASK-M3-07, and Aarseth–Zare is TASK-M3-68. Open:
  - (2) REQ-INT-049's threshold, 4.232e-09 at `η = 0.001`, was measured under § 0.1's fixed `dτ`, not Part 6's per-step
    rule (R-563). Recommended: TASK-M3-68 re-measures it under the rule in force, at the gate's `n = 32`, and proposes
    it as a calibration (R-71); the value is the owner's at the M3 gate. Measurable alternative: run the ladder under
    both rules and record both.
  - (4) REQ-INT-054's bound per sync interval has no formula. Recommended: the step budget over one interval,
    `max(N_min, step_budget_rate·Δ_sync)`, a recommendation only, its values REQ-INT-103's and REQ-INT-108's.
  - (5) Found in the transcription, where the reference and the corpus disagree; nothing is chosen:
    1. **The landing secant.** `docs/design/principia_dd_validation_orbits.md` § 0.1: "Not yet RK4's fourth — a residual
       lower-order term remains, most plausibly the LC re-registration at sync boundaries". `driver.rs:419-421`:
       "`clamp_final_step` sizes the landing step from `dt/dtau` read *before* the step. That is a first-order
       predictor, so its landing residual is `O(h^2)` and it caps the observable convergence order at two", and its
       default `land_iterate: true`, `land_max_iters: 4` (`driver.rs:476-477`) retakes the landing step by a secant.
       The corpus names the clamp only (REQ-INT-049; Part 6, "the landing clamp"). Is the secant part of Aarseth–Zare's
       step control here? It moves REQ-INT-049's measurement.
    2. **The gate's figures.** § 0.1's table: "0.001 | 2.818e-03 | **4.232e-09**". `driver.rs:398-400`: "closure error
       `2.976e-3` against `3.595e-9` at `eta = 1e-3`, and the convergence order across `eta in [0.02, 0.001]` **1.13
       against 3.06**", at `n_sync = 32`.
    3. **The unstable inverse at 179.9°.** `docs/design/principia_dd_predictability_horizon.md` § 7.3 and REQ-INT-044's
       verify: "reference (unstable) | 3.5e-9 | **2.2e-2**" at 179.9°. `tb_lc.py:67-69`: "angle 135 170 179 179.9
       179.99 / f64 2.2e-16 3.9e-15 1.9e-13 4.0e-12 3.5e-09 / f32 1.8e-07 2.3e-06 9.8e-05 2.2e-02 1.7e-04", so f64's
       3.5e-9 is at 179.99° and 179.9°'s is 4.0e-12.
    4. **What couples Γ.** Integrator contract Part 2a: "`Gamma` is **not separable** (the `|u|^2 |p_lambda|^2` term
       couples position and momentum)". That term is the single-pair Levi-Civita Γ's (`tb_lc.py:19`, "Gamma =
       |p_u|^2/(8 mu_rho) + |u|^2 [ |p_lam|^2/(2 mu_lam) + V_pert - E ] - G m_i m_j"); Aarseth–Zare's two-pair Γ has no
       `p_λ`, and couples through `B|p1|^2/(8 mu1) + A|p2|^2/(8 mu2) + (L1 p1).(L2 p2)/(4 m_a)` (`tb_az.py:29`). The
       conclusion, not separable, holds either way.
    5. **"From the longest side".** Part 2b's table: "two pairs, reference body from the longest side". `tb_az.py:146`:
       "Reference body = the one NOT in the longest side". The transcription states the code's rule.
    6. **`v_rel` in the predictive limit.** R-160: "`dτ ≤ f·d_min / (|v_rel|_max·A·B)`". `driver.rs:608`:
       "`phys_from_state` returns `(R1, R2, V1, V2)`", but the AZ system it calls (`src/integrate/az/system.rs`) is not
       in the imported set, and the NumPy `phys_from_state` it was transcribed from returns the conjugate momenta
       (`tb_az.py:80-83`, "P1 = _Lmat_apply(u1, p1)/(2*A[:, None])"). Velocities or momenta?
    7. **The `1e-300` guards.** `tb_lc.py:98`, `102-103` and `tb_az.py:78-79`, `224-225` test and floor at the literal
       `1e-300`, which underflows at f32; integrator contract Part 2c rule 1: "NO PRECISION-DEPENDENT CONSTANT IN A
       CONTROL-FLOW DECISION", and Part 6 floors `A` and `B` at `TINY`, as `driver.rs:716-717` does. The Rust inverse map
       is not imported, so what it tests against is unknown.
    8. **The tie-break's source.** The Rust selector (`src/integrate/az/reference_body.rs`) is not imported; the
       tie-break transcribed is `tb_az.py:148`'s `np.argmax`, first maximum in `(0,1), (0,2), (1,2)` order.
    9. **Where the landing secant stops.** `tb_az.py:303-306` re-tests the landing on every retake: "need =
       (np.isfinite(s).all(axis=1) & (~done) / & (s[:, 8] >= dt_left - tol) / & np.isfinite(got) & (got > 0.0) / &
       (np.abs(got - want) > tol))", so a retake that lands short ends the secant. `driver.rs:1086` does not: "if
       !got.is_finite() || got <= T::zero() || (got - want).abs() <= tol_abs { break; }", retaking up to
       `land_max_iters` times. Which stop is the rule? It moves the landing step and REQ-INT-049's measurement.
- **Applied meanwhile:** the transcription and the split above (10 Nov 2026, applied per R-369). TASK-M3-07 builds the
  axis, which needs none of it. *Was:* "none. TASK-M3-07 waits; nothing is built."
- **Waits:** TASK-M3-68 (REQ-INT-049 on (2) and (5) 1-2 and 9, REQ-INT-054 on (4), REQ-INT-044 on (5) 3 and 7,
  REQ-INT-024's profile on (5) 4, the predictive limit on (5) 6). *Was:* "TASK-M3-07 (REQ-INT-044, REQ-INT-049, REQ-INT-054, and the
  Aarseth–Zare arms of REQ-INT-021 and REQ-INT-024)."

## RQ-748: REQ-RENDER-102 and REQ-RENDER-104 test a `linocut` with a `depth` parameter and `ground` and `field` inputs, which the lab's linocut does not have *(record conflict between a verify and the reference; TASK-M7-54, PR for REQ-RENDER-101 to REQ-RENDER-104; context: the task that ports the lab's linocut)*

*Queued 8 Nov 2026 by TASK-M7-54. A record correction, two-way under R-369 (§ "When to ask the human"): the orchestrator
and the reviewers can settle it. It blocks nothing now: no linocut ships in TASK-M7-54, and its tests use a fixture.*

**In plain English:** two Print graph tests describe a "linocut" node with a depth setting and two inputs called
`ground` and `field`, and say its defaults are the lab's. The lab's real linocut has none of those: its knobs are ink,
gouge, amount and seed, its inputs are `in`, `flow` and `paper`, and the `{ ground: "paper", field: "white" }` defaults
belong to the lab's "Unfinished" node. TASK-M7-54 tests against a stand-in linocut built to the tests' description, so
nothing is wrong today; but the task that ports the real linocut must follow the lab (R-486), not these tests.

- **File, section:**
  - `plan/requirements.yaml`, REQ-RENDER-102's verify: "an unconnected `ground` input of `linocut` takes `paper` and an
    unconnected `field` input `white`, as the lab's defaults declare"; REQ-RENDER-104's verify: "a fixture setting
    `linocut.depth` outside its range".
  - `docs/design/principia_print_graph.md` § "4.1 A graph is data", its example: `"cut": { "type": "linocut",
    "params": { "depth": "$depth", "seed": "$seed" }, "in": { "in": "img", "flow": "fl", "paper": "pap" } }`; § "4.2
    Port types": "An unconnected input takes the default its node type declares (the lab's `defaults: { ground:
    "paper", field: "white" }`)."
  - The reference, `docs/gui/reference/print_graph/catalogue/node_catalogue.json`, `linocut`: inputs `in`, `flow`,
    `paper`; params `ink`, `gouge`, `amount`, `seed`; and the lab (`reference/print_graph_lab/index.html`), `linocut`:
    `defaults: { paper: "paper" }`, the `{ ground: "paper", field: "white" }` defaults being `unfinished`'s.
- **The question:** whether the two verifies are reworded to name a stand-in (or `unfinished`, whose defaults they
  quote), so the linocut port is not read as needing a `depth` or `ground` and `field`.
- **Options seen:** (a) reword the verifies to "a fixture node" with those ports and that parameter, as TASK-M7-54
  built them; (b) name `unfinished` for the defaults and a node the lab gives a ranged parameter for the range; (c) leave
  the text, with this entry as the record.
- **Applied meanwhile:** TASK-M7-54's tests use `fixtures/print_graph/nodes/linocut.json`, a stand-in with the shape the
  verifies ask of it (§4.1's `depth` and `seed`, §4.2's `ground` and `field` with the quoted defaults), stated in it and
  in `fixtures/README.md` not to be the lab's linocut. Nothing ships from it.
- **Waits:** nothing now. The linocut port task reads this entry.

## RQ-771: On the invariant `(L_z, E)` and `(L_z, K)` charts, what "unfold" past the spin edge and "the nearest edge system" mean, now that the kernel's construction has no second sheet *(silence; a one-way door, a physics definition; the owner's direction of 10 Nov on extensions, R-407, R-508, R-72; REQ-CHART-054, REQ-ENC-030; context: the addendum's N12, N13, N14 in `plan/charts/m2-chart-addendum-extensions-axes.md`)*

*Queued 10 Nov 2026 by the chart addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`), each quote re-read on
`origin/main` a5f0aa88. The owner's direction is not a ruling; nothing is chosen here.*

*In plain English: the prototype draws the energy chart past its "spin at its maximum" edge as a second sheet, where the
system's in-and-out motion runs the other way. The kernel has no such sheet: it always takes the outward one. So the
sheet, and what "the nearest system on the edge" means on the real chart, need defining before anything is built.*

- **File, section:**
  - The addendum (`orch/charts/prototype-2026-11-10/addendum-extensions-axes-2026-11-10.md`, on the orchestrator's
    drive, cited by path, not a ruling) § 1.2's table: "L_z × E, spin at ± its maximum | unfold onto the contracting
    sheet (radial rate changes sign); period 4 in L_z / L_max" and "L_z × E, kinetic energy below 0 | clamp to K = 0";
    § 1.4: "On the real energy chart the feasibility edge depends on shape and is curved. "Unfold" and "nearest edge
    system" both need a definition there."
  - `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction", (iii): "a = √(2·(K* −
    Lz²/(2I)))" and "vᵢ = vᵢ^(L) + a·wᵢ"; `crates/kernel/src/decode/invariant.rs:12`: "(iii) K* < K_min → infeasible;
    else a = √(2(K* − K_min)),  vᵢ = vᵢ⁽ᴸ⁾ + a·wᵢ,  pᵢ = mᵢvᵢ".
  - `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid":
    "**Past the unit square (R-407, R-508): `s` is bounded, and `t` bounded below `t = 0` and continuing past `t = 1`**".
  - `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation", "The nearest valid point" (R-72;
    REQ-ENC-030): "The nearest valid point is the one at the same `K*`, and so at the same `E`, on the parabola, on the
    side of the entered `L_z`" and "Where `K* < 0` … The nearest feasible point is then the parabola's apex, `L_z = 0`
    at `K* = 0`, `E = U`: the rest start."; § "A qualitatively different IC": "At `K* = 0` with `L_z ≠ 0` the
    parabola's only point is `L_z = 0`, so the pair is rejected (`L_z`'s class would change). At `K* < 0` the apex keeps
    `L_z`'s class only where `L_z = 0`. … So that pair is projected to the rest start, and every other pair there is
    rejected." and "A chart point below the invariant charts' apex, `t < 0`, has no entered `L_z`: `L_z(s, t)` has no
    real value there. So projecting it to `t = 0` changes no class, and the projection stands."
  - RQ-764, the momentum seed (does the expansion seed `w = r/√I` replace R-82's rule), with the re-plan's N10,
    "the seed conversion", and its § D item 24, "The seed `w = r/√I`" (`plan/charts/m2-chart-system-replan.md`).
- **The gap:** the kernel's exact feasibility test (`invariant.rs:273`, `2·I·K* < L_z²` decided exactly) is binary:
  feasible or not. It gives no second sheet: `a` is always the non-negative root (`invariant.rs:532`), so the
  construction always takes `+a·w`. The prototype's "contracting sheet" is `−a·w`, a new definition (R-72) and a new
  extension type under R-407 clause 3's physics review. Its "radial rate" is the seed's. Under R-82's seed `w` is the
  inner pair's dilation `(ρ̇, λ̇) = (ρ, 0)` where `z_α ≤ 0` and the third body's `(0, λ)` elsewhere (`invariant.rs:455`),
  so "contracting" names a different pair on either side of `z_α = 0`; under (a) or (b) of RQ-764, the momentum
  seed, with the re-plan's N10 built, `w = r/√I` and "contracting" names the whole system's dilation, `I` decreasing. N13 comes
  after N10, and its "sheet 1 bitwise today's" baseline is `main`'s construction at N13's start: N10's under RQ-764's
  (a) or (b), R-82's under its (c). Three facts frame the answer. On the default warped chart the edges are
  straight (`s = 0, 1` at every `t`, `L_z = (2s − 1)·L_max(t)`, chart_reference §2.1), and the `t = 0` row is the rest
  start, `L_max(0) = 0`: one system on an untilted, unsliced chart, a line of different rest systems, one per pixel,
  under a tilt or slice that moves the configuration or masses (R-471 E3); only under the linear option (R-471 E2) is
  the edge the curved parabola `|L_z| = √(2I(E − U))`, read per pixel with that pixel's own `U` and `I`. REQ-ENC-030
  defines the nearest edge system on every pixel of the warped charts (past `s = 0, 1` the rigid rotation at
  `±L_max(t)`, keeping the sign; below `t = 0` the rest start, "the projection stands"), but not everywhere under the
  linear option or on `Φ_θ,L_z`: there it rejects `K* ≤ 0` with `L_z ≠ 0`, since the rest start's `L_z = 0` is
  another class, the only one that can reach a triple collision (Sundman). And the sheet `−a·w` at `L_z` is exactly
  the time reversal (`p → −p`) of the chart's own system at `−L_z`, since `vᵢ^(L)` is linear in `L_z` and `a` depends on
  `L_z²`, for any seed `w` that does not depend on `L_z` (both seed rules): the unfolded side shows the time-reversed
  chart, shifted by one in `s`.
- **The question:** (i) is the second sheet `−a·w` adopted as the invariant charts' unfold, and on which mapping; (ii)
  what does "the nearest edge system" mean for a clamp where REQ-ENC-030 rejects, that is under the linear option and
  on `Φ_θ,L_z`, below the parabola's floor at a non-zero `L_z` (`K* ≤ 0`, `L_z ≠ 0`); (iii) on the Burrau `Φ_θ,L_z`
  chart, which uses the same construction (chart_reference §4.5), the same?
- **Options seen for (ii)**, below the floor at `L_z ≠ 0` (elsewhere the clamp is REQ-ENC-030's projection):
  - (ii-1) The rest start, in spite of REQ-ENC-030's reject. One system per pixel's configuration; it draws the
    `L_z = 0` class (the only one that can reach a triple collision) over pixels whose own `L_z` is not zero, so the
    clamp changes the class the pixel's coordinates name, which lookup itself refuses.
  - (ii-2) A vertical clamp at the pixel's own `L_z` to `E = U + L_z²/(2I)` (`K = L_z²/(2I)` on `(L_z, K)`), the rigid
    rotation at that `L_z`, as § A.11's held-`L_z` row has it. It keeps `L_z`'s class, and on the linear chart it is a
    line of systems, one per column. It moves `E`, so it is not REQ-ENC-030's metric (fixed `E`) but a new definition
    (R-72) under R-407 clause 3's review; and where `L_z² ≥ −2IU` it lifts `E` to `0` or above, changing `E`'s class,
    which REQ-ENC-030's second classifier forbids for a projection.
  - (ii-3) Blank there: no new definition; the linear option then has a blank region the direction does not want.
- **Options seen:**
  - (a) Sheet 2 is `vᵢ = vᵢ^(L) − a·wᵢ`. Warped: past `s = 1`, `s' = 2 − s` on sheet 2, period 2 in `s` (4 in
    `L_z/L_max`, the prototype's figure); linear option: at the pixel's own `E`, `L_z` reflected about the parabola, the
    sheet flipped, the same period in `L_z/L_max(E)`. Clamp: REQ-ENC-030's nearest valid point where it projects (fixed
    `E`, `L_z` onto the parabola where `K* > 0`; the rest start where `K* < 0` and `L_z = 0`, and on the warp below
    `t = 0`), (ii)'s answer where it rejects. `Φ_θ,L_z` the same. Consequences: real, distinct systems, each the time reverse of
    one in the square; a sheet word in the record and in the decode's inputs; R-408's primary range for `s` becomes two
    sheets; the backwards table must tell the sheets apart (RQ-776).
  - (b) As (a) on the warped mapping only (the default, straight edges); under the linear option the parabola clamps by
    REQ-ENC-030 and nothing unfolds. Simpler per-pixel arithmetic; the linear option keeps its present meaning.
  - (c) No second sheet: the spin edges clamp by REQ-ENC-030 or stay blank. No new physics definition; the prototype's
    unfold is not built.
- **Recommended:** (a), with physics review of the sheet's definition (R-407 clause 3): it is the only kind past this
  edge that shows real, distinct systems, and its time-reversal identity gives an exact, bitwise test (the addendum's
  N13) and a backwards-table shortcut. (b) is the measurable fallback if the per-pixel reflection under a tilt
  misbehaves. For (ii), (ii-2) where it keeps both classes (`L_z² < −2IU`) and (ii-3) elsewhere, since it is the only
  choice that never draws a system of another class; the owner's to choose, after physics review.
- **Measurement:** N13's distinctness measurement on `InvariantLE` at the register's defaults (256 × 256 over `s ∈ [1,
  2]`): the share of pixels whose decoded state equals a state of the square on the same grid, positions bitwise
  equal (the held configuration on both) and momenta matching when `‖Δp‖₂ ≤ 10⁻¹²·‖p‖₂` over all six components (a zero `p` only
  exactly); predicted only the fold columns and the `t = 0` row; its control, sheet 1 mirrored in `s` (a repeat),
  matches 100%; void if the default configuration's seed is degenerate (`a = 0`, the sheets equal by construction).
  Criterion written in the addendum before any run.
- **Applied meanwhile:** nothing. The charts stay as chart_reference §5.4 has them: `s` bounded, `t` bounded below 0.
- **Waits:** the addendum's N12 (the definitions) and N13 (the sheet).

## RQ-772: Which edges of which real charts have a genuine continuation (a second sheet, or real systems the chart does not show), and which can only repeat, clamp, saturate or stay blank *(silence; a one-way door, physics definitions under R-407 clause 3; the owner's direction of 10 Nov on extensions, R-407, R-408, R-508, R-554, R-879; REQ-CHART-054, REQ-CHART-055; context: N12, N14)*

*Queued 10 Nov 2026 by the chart addendum; quotes re-read on `origin/main` a5f0aa88. A per-edge table is proposed in
`plan/charts/m2-chart-addendum-extensions-axes.md` § A.11 for the physics reviewer and the owner; nothing is chosen.*

*In plain English: for every edge of every chart we ship, is there something real past it (new systems, or the same
systems seen again), or is it a true end where the picture can only stop, smear the edge outward, squash, or stay
empty?*

- **File, section:**
  - The addendum § 1.1: "Unfold is preferred wherever a second sheet exists, because it is the only non-periodic kind
    that keeps the slice a genuine surface. Clamp is for true edges, where nothing exists beyond."
  - `decisions.md` § R-407, clause 3 (as applied): "**No invented continuations.** An axis gets anything beyond
    bounded only by declaring it, and any new extension type needs physics review."
  - `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)": the
    table of the existing charts' types (latent affine; sphere θ periodic, φ pole-crossing; exponential map bounded;
    invariant warp `s` bounded, `t` bounded below 0 and affine past 1; mass simplex affine with negative masses not a
    system; the Burrau family per axis).
  - `decisions.md` § R-554's heading: "The shape sphere's exponential map past `r = π` is the domain's end by design,
    blank and unwarned".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square", "Where the extension matters": the
    exponential map's "square covers only a disc of radius `s√2` about `n₀`, so past its square a pixel is classified
    not a system and draws the background, though the sphere holds systems there".
  - `docs/design/principia_chart_composition.md` § 18 (time as an axis): "`t_lo` is new; integration still starts at
    0." Nothing says what a time axis shows before `t = 0`.
- **The gap:** the corpus types every shipped axis, but chose bounded wherever a continuation would repeat systems or
  run off the domain; it never asks whether a second sheet exists. The owner's direction asks for an extension on every
  edge, unfold where a sheet exists.
- **The question:** for each edge in § A.11's table, which kind is genuine (wrap, pole-crossing, a new repeat map, an
  unfold onto a sheet, affine up to the domain's end) and which can only clamp, saturate or stay blank?
- **Options seen:**
  - (a) Adopt § A.11's classification, each new declaration physics-reviewed: genuine continuations are the existing
    periodic and pole-crossing axes; the exponential map's square edges (affine to `r = π`) and `r = π` (a repeat,
    `r ↦ 2π − r` in the opposite direction, no new systems); the invariant charts' and `Φ_θ,L_z`'s spin edges (RQ-771's
    sheet); the Burrau `ν` and `θ` edges up to the domain's end (twins past the leg-swap point repeat, R-27); the time
    axis before 0 (the trajectory's past, by a backward run, new compute; R-879's final-condition charts run backward
    already). True ends: the invariant `t < 0` (the rest start: one system untilted and unsliced, a line of rest
    systems under a tilt or slice that moves the configuration or masses), the floor of a held `L_z`, the mass
    simplex's sides and the δm strip's zero-mass end, the Burrau `K` below 0.
  - (b) Only the invariant spin edges and the time axis get a continuation; every other bounded edge clamps.
  - (c) Keep today's types; the direction's extension on every edge is met by clamp alone where the type is bounded.
- **Recommended:** (a) as the physics reviewer's starting list, the owner confirming which to build.
- **Measurement:** per declared continuation, the distinctness share of N13's form (new systems vs repeats), so a
  repeat is counted once under R-408 and a new sheet is not mistaken for one.
- **Applied meanwhile:** nothing.
- **Waits:** N12, N14.

## RQ-773: Does the direction "a chart never has a blank area" amend R-407's bounded default and "no invented continuations", R-470's blank and R-554's blank past `r = π`, and how do clamped, saturated and unfolded pixels count under R-408 *(conflict; a one-way door, amending rulings; the owner's direction of 10 Nov, R-407, R-408, R-470, R-508, R-537, R-554; REQ-CHART-054, REQ-CHART-056, REQ-GUI-268; context: N12, N14)*

*Queued 10 Nov 2026 by the chart addendum; quotes re-read on `origin/main` a5f0aa88. A direction is not a ruling, so
nothing in force changes until the owner rules.*

*In plain English: today an edge with nothing past it draws empty background, by rulings. The owner now wants no empty
areas. That needs the rulings changed, and a rule for whether the smeared or squashed areas count in statistics.*

- **File, section:**
  - The addendum § 1, "Direction (owner):": "a chart never has a blank area. The 2D slice should always be looking at
    systems, so every edge of a chart's primary range has a declared extension."
  - `decisions.md` § R-407, clause 1 (as applied): "**An axis that declares nothing is bounded.**"; clause 3: "any new
    extension type needs physics review"; its applied text: "P4 (a clamped extension) does not arise, since no axis is
    clamped".
  - `decisions.md` § R-470's heading: "A pixel that is not a system draws blank, the view's background, in every view,
    the outcome view included, never a palette colour".
  - `decisions.md` § R-537, EX4 (the applied text): "The only blank pixels are not-a-system pixels (EX2)."
  - `decisions.md` § R-408 (the owner's words): "count a visible pixel only if every axis is inside its primary
    range."
- **The conflict:** clamp and saturate are not among R-407's four types, and "no invented continuations" requires a
  declaration under physics review for each; bounded, the default, is blank by R-470. A clamped area shows edge systems
  again, a line smeared over an area, which R-408 must not count twice; a saturated area shows systems nearly alike; an
  unfolded sheet shows new systems that R-408's primary ranges (one period, pole to pole, unbounded, the domain) do not
  cover.
- **The question:** (i) does the owner's direction amend R-407 (adding unfold, clamp and saturate as declared kinds, per
  edge, each under physics review) and R-470, R-554 and EX4 (blank only where "blank" is the declared kind or validity
  fails)? (ii) what is each new kind's primary range for R-408's count?
- **Options seen:**
  - (a) Amend as asked: seven kinds per edge (affine, wrap = periodic, pole-crossing, unfold, clamp, saturate, blank =
    bounded), and, where RQ-772 declares one, a reflection or a new repeat map, each a new kind under R-407's clause 3;
    blank stays a kind and validity failures stay "not a system". Counting: an unfold sheet is inside the primary range
    (new systems, counted); a clamped pixel is outside it (an edge system again, not counted, as a repeat is); a
    saturated pixel is counted where its system is not shown inside the square and not counted otherwise, the rule
    declared per edge with the kind.
  - (b) As (a), but no extension pixel past the primary square ever counts: statistics are the square's alone, simple
    and unchanged from today for every chart but the latent.
  - (c) The direction stands only for presentation: blank pixels draw the clamp's colour, the classification and
    statistics unchanged.
- **Recommended:** (a): it keeps R-408's "each system once" exactly and gives the unfold's new systems their count.
- **Measurement:** REQ-CHART-079's test extended: panning by one period, or across a clamp edge, changes neither
  "forbidden in view" nor W7's counts; an unfold adds exactly its sheet's systems.
- **Applied meanwhile:** nothing; R-407, R-408, R-470 and R-554 stand as written.
- **Waits:** N12, N14, and through them TASK-M8-87 and TASK-M8-55.

## RQ-774: Are clamped or saturated areas marked in the picture *(silence; a one-way door, a look; the owner's direction of 10 Nov, R-470, R-552; REQ-GUI-348; context: N18)*

*Queued 10 Nov 2026 by the chart addendum. The addendum records that the owner has not asked for a mark.*

*In plain English: where the picture is a smear of the edge, or a squashed coordinate, should you see that it is, or
does it look like the rest of the chart?*

- **File, section:**
  - The addendum § 1.4: "Whether a clamped or saturated area is marked in the picture (dimmed or hatched). The owner
    has not asked for it."; § 2.2, "Direction (owner):": "no "wraps", "folds back", "saturated" or "one system" text …
    The numbers and ticks carry it."
  - `decisions.md` § R-470's heading: "the hatch survives only as the optional Overlays › Chart › forbidden region, off
    by default, over blank pixels only".
  - `plan/requirements.yaml` REQ-GUI-348: "Overlays ▾'s Chart group must hold an "outline the chart's primary square"
    toggle … on by default".
- **The question:** is a clamped or saturated area marked, and how?
- **Options seen:**
  - (a) No mark in a presentation view: the ticks carry it (direction § 2.2), the primary-square outline (REQ-GUI-348, on
    by default) shows where the chart ends, and the debug view names each pixel's kind.
  - (b) (a) plus an Overlays › Chart toggle, off by default, dimming or hatching clamped and saturated pixels, as R-470's
    forbidden-region overlay does.
  - (c) Always dimmed.
- **Recommended:** (b): nothing changes by default, and the honest view is one toggle away.
- **Measurement:** none decides it; a screenshot pair per option on the latent, energy and simplex charts for the
  owner to look at.
- **Applied meanwhile:** nothing.
- **Waits:** N18 (built only under (b) or (c)).

## RQ-775: Where a chart's per-edge extension choice lives (the registry, the view state, or both), and whether it is in the sim key *(silence; a one-way door, what the cache keys; the owner's direction of 10 Nov, R-92, R-407, R-472; REQ-CHART-096, REQ-CHART-068, REQ-SCHED-101; context: N14, TASK-M2-66)*

*Queued 10 Nov 2026 by the chart addendum; quotes re-read on `origin/main` a5f0aa88.*

*In plain English: if each edge's behaviour can be switched, is the switch part of the chart's definition, of the
current view, or both; and does switching it throw away work already computed?*

- **File, section:**
  - The addendum § 1.2, "Direction (owner):": "the extension kind is a choice per edge, with a default. The owner picks
    the defaults by looking."; "The prototype has no switch between kinds yet; each edge has one hard-coded kind."
  - `decisions.md` § R-407, clause 1: "A custom chart inherits its extension from its axes, so nothing is written per
    chart." and § R-472: "a tilted, sliced or saved chart extends by its axes' types, nothing written per chart".
  - `decisions.md` § R-92: "In-plane pan and zoom re-address; slicing out of the plane, tilting and rotating
    re-integrate; lock changes neither."
  - `plan/requirements.yaml` REQ-CHART-068: "a chart parameter (warped | linear) in SimConfig, a sim key".
- **The question:** (i) the registry, the view state, or both; (ii) in the sim key or not?
- **Options seen:**
  - (a) Both: each registry row and each map declares, per edge, the kinds allowed and the default; the view state holds
    an override per edge, saved with the view. A kind whose pixels show systems computed elsewhere (wrap,
    pole-crossing, clamp) re-addresses to the canonical quads and is off the sim key, as repeats are (R-537 EX7); a kind
    that shows new systems (unfold, saturate) changes the quads' systems and is on it.
  - (b) The registry only: one kind per edge per chart, no switch; nothing in the key beyond today's.
  - (c) The view state only, always in the sim key.
- **Recommended:** (a): it keeps R-407's inheritance (defaults from the row), gives the owner's per-edge choice, and
  re-integrates only when a pixel's system changes.
- **Measurement:** a test that switching clamp to blank requests and integrates no quad, and unfold to clamp drops the
  sheet's quads from the request set; criterion: zero new integrations for re-addressing kinds.
- **Applied meanwhile:** nothing.
- **Waits:** N14, TASK-M2-66.

## RQ-776: What the backwards table does on an unfolded chart *(silence; a one-way door, accuracy; the owner's direction of 10 Nov; R-926 item 7 and R-925 item 2, the backwards table; context: N15)*

*Queued 10 Nov 2026 by the chart addendum. The backwards table (running state to chart position, for Chart position,
the state map and the round-trip presets) is R-926 item 7's (the owner's chart-system decisions), built by the
re-plan's N7 and N9 (`plan/charts/m2-chart-system-replan.md`).*

*In plain English: colouring by "where on the chart is this system now" needs a lookup from a state back to a chart
point. With two sheets, a state could belong to either; which one does the lookup return?*

- **File, section:**
  - The addendum § 1.4: "What the backwards table does on an unfolded chart: two sheets share a region of state space
    only at the fold, so this should be benign, but it is unmeasured."
  - `plan/charts/m2-chart-addendum-extensions-axes.md` § A.11 and RQ-771: sheet 2 at `L_z` is the time reversal of the
    square's system at `−L_z`.
- **The question:** does the key carry the sheet, does the search span both sheets, or does a state fold onto sheet 1?
- **Options seen:**
  - (a) The table holds sheet 1 only; a state is looked up as itself and as its time reverse, the better match kept and
    its sheet recorded. Half the table; exact by the reversal identity, provided the table's state metric is invariant
    under `p → −p` (a mass-weighted Euclidean metric is; a metric that is not would make the two lookups disagree).
  - (b) A table per sheet, searched both.
  - (c) Sheet 1 only, the second sheet never returned: Chart position is then wrong on the unfolded side.
- **Recommended:** (a).
- **Measurement:** N15's: wrong share and round-trip error, near the fold apart from the rest, criterion written in the
  addendum before any run.
- **Applied meanwhile:** nothing.
- **Waits:** N15, and the re-plan's N7.

## RQ-777: Is a weighted-mix axis value, which adds unlike quantities, acceptable mid-blend, and how are its units shown *(silence; a one-way door, what an axis claims; the owner's direction of 10 Nov on computed axes; REQ-GUI-084, REQ-GUI-246; context: N16, N17)*

*Queued 10 Nov 2026 by the chart addendum.*

*In plain English: halfway through a morph, or on a tilted view, an axis can be part angle and part momentum. The
prototype labels it with a weighted sum of the two numbers. Is a number that adds an angle to a ratio acceptable, and
how should units appear?*

- **File, section:**
  - The addendum § 2, "Direction (owner):": "An axis that is no longer a pure coordinate says what it is made of. One
    axis per edge, including in a blend."; § 2.1: "A mixed value adds unlike quantities (an angle and a ratio, say). It
    is consistent, and exact at the ends, but mid-blend it is a construction, not a physical quantity."; its example
    name: "`0.52·[z_α shape] + 0.36·[q₀ inner pₓ] + 0.12·[β angle ÷ π]`".
  - `plan/requirements.yaml` REQ-GUI-084: "The figure's axis labels must carry the short axis name and the range at
    each end."
  - `docs/design/principia_lines.md` § "8b. Axes": "**Primary scale.** The chart's own coordinates: latent units on a
    latent chart, physical units on a physical axis."
- **The question:** is the summed value shown, and in what units?
- **Options seen:**
  - (a) The prototype's: `Σ coef_k·value_k` in each term's own units, the name listing each term with its unit; the
    hover readout gives each term's own value, the exact numbers. The coefficients are dimensionless, but `value_k` is
    not, so the axis's numbers depend on the unit chosen for each coordinate: `β` in radians and `β ÷ π` (the
    addendum's own example) give different tick values, and different places for the round values, in the same window.
    The number on a mixed axis is then a definition fixed by those unit choices, not only a label.
  - (b) Each term divided by its window scale (the unit of RQ-762, the window's per-coordinate scale), so the sum is
    dimensionless "window units"; the name says so. `Σ coef_k·value_k/scale_k` does not depend on the unit chosen for
    any coordinate, since `scale_k` carries the same unit as `value_k`.
  - (c) No sum: the edge carries the dominant term's own ticks, the others listed in the name with their coefficients.
- **Recommended:** (a), with the readout carrying the exact per-term values; (c) is the measurable alternative, judged
  by the owner on N16's sweep.
- **Measurement:** N16's sweep reports terms per edge and name length over a morph and a tilt; none decides the units.
- **Applied meanwhile:** nothing.
- **Waits:** N16, N17.

## RQ-778: How a computed axis's terms are chosen: the share's scale, and whether small terms are dropped *(silence; a one-way door, a tolerance; the owner's direction of 10 Nov, R-507, R-920, R-71; context: N16)*

*Queued 10 Nov 2026 by the chart addendum.*

*In plain English: the prototype names an axis by the coordinates it moves, weighted by how much, and leaves out any
under 2%. That 2% is a tolerance nobody has set, and the weighting needs a unit per coordinate.*

- **File, section:**
  - The addendum § 2.1: "**Share of the direction:** share_k = (slope_k / scale_k)², normalised to sum to 1 over the
    chart's coordinates. Terms under 2% are dropped."
  - `decisions.md` § R-507's heading: "the screen axis in units of the axis's full range at zoom 1, the hidden
    coordinate in its slider span"; RQ-762, the window's per-coordinate scale.
- **The gap:** `scale_k` is the window's per-coordinate scale, open as RQ-762; the 2% is a hand-set tolerance (R-71: a
  missing value is a calibration). A tilt of 8° gives a share of `sin²8° ≈ 1.9%`, so the axis is named pure while the
  view moves a second coordinate.
- **The question:** (i) is `scale_k` the window's scale as RQ-762 rules it; (ii) are small terms dropped, and
  at what share?
- **Options seen:**
  - (a) No drop: every non-zero term is named; the display shortens the name and the readout shows it whole.
  - (b) Drop below a calibrated share (R-71), the dropped total shown on the axis as a number.
  - (c) The prototype's 2%.
- **Recommended:** (a): nothing claims purity it lacks; the long-name problem is layout (N17).
- **Measurement:** N16's sweep: terms per edge and name length, reported; criterion: reported only, written before the
  run.
- **Applied meanwhile:** nothing.
- **Waits:** N16; RQ-762, the window's per-coordinate scale.

## RQ-779: The default extension kind of each edge, for the owner to confirm by eye *(the owner's call by looking; a one-way door, defaults; the owner's direction of 10 Nov; R-407, R-508, R-898; context: N14)*

*Queued 10 Nov 2026 by the chart addendum. The addendum's table is the prototype's current settings, "which the owner
has seen and not objected to (to be confirmed as defaults)"; it is not decided.*

*In plain English: for each chart edge, which behaviour is on by default. The owner said they would pick by looking.*

- **File, section:**
  - The addendum § 1.2, "Direction (owner):": "where a chart has no fully valid extension (the mass simplex, L × E), the
    extension kind is a choice per edge, with a default. The owner picks the defaults by looking."
  - The addendum § 1.2's table (prototype settings): "L_z × E, spin at ± its maximum | unfold onto the contracting
    sheet …"; "L_z × E, kinetic energy below 0 | clamp to K = 0"; "latent masses | saturate (already the behaviour; the
    owner is happy with it)"; "latent shape z_α, latent momenta | saturate"; "β, longitude | wrap"; "polar angle |
    unfold (over the pole)"; "time before 0 | clamp to the starting state".
  - `docs/design/principia_chart_reference.md` § 5.4's table (today's types).
- **The question:** confirm or change each row's default:

  | Edge | Prototype now | Today (§ 5.4) | Proposed default |
  |---|---|---|---|
  | `(L_z, E)`, `(L_z, K)`: spin at ±max (`s` edges) | unfold | bounded, blank | unfold (RQ-771 (a)) |
  | `(L_z, E)`, `(L_z, K)`: below `K = 0` (`t < 0`) | clamp to `K = 0` | bounded, blank | clamp: on the warp the rest start, one system untilted and unsliced; under a tilt or slice that moves the configuration or masses, a line of different rest systems smeared downward (the addendum's "streaks") |
  | latent masses, shape `z_α`, momenta | saturate | affine; the links saturate (REQ-CHART-087) | unchanged: affine, saturating by the links |
  | latent `β`, sphere longitude θ | wrap | `β` periodic (R-898); θ periodic | unchanged |
  | sphere polar angle φ | unfold over the pole | pole-crossing | unchanged |
  | exponential map: square edges, `r = π` | (not in the prototype) | bounded; past `π` blank (R-554) | affine to `r = π`, then the repeat (RQ-772 (a)) |
  | mass simplex sides | (not in the prototype) | affine, negative masses blank | clamp in the chart's own coordinates |
  | Burrau `ν`, `θ`, `m`; `K` below 0; δm | (not in the prototype) | bounded, or affine then blank | clamp, until RQ-772 declares a continuation |
  | time before 0 | clamp to the start | silent | clamp to the start |
- **Options seen:** (a) the proposed column; (b) the prototype's column where it has a row, blank elsewhere; (c) the
  owner's picks, row by row.
- **Recommended:** (a), for the owner to look at in the prototype (`chart-slice.html`, version 21) and, once built, on
  the kernel's charts.
- **Measurement:** none; screenshots per row from N14, side by side with today's blank.
- **Applied meanwhile:** nothing.
- **Waits:** N14.

## RQ-780: Do computed axes replace the line system's primary scale and the composition's "no ticks mid-morph" rule *(conflict; a one-way door, what an axis shows; the owner's direction of 10 Nov on computed axes, R-476, R-850; REQ-GUI-246, REQ-GUI-084, REQ-GUI-422; context: N16, N17, TASK-M8-50, TASK-M8-06)*

*Queued 10 Nov 2026 by the chart addendum; quotes re-read on `origin/main` a5f0aa88.*

*In plain English: the design says a frame in the middle of a morph shows no ticks, and that a warped axis's ticks sit
at round values of its physical quantity. The owner now wants ticks computed from the view everywhere, morphs
included. Which rule wins, and where do they combine?*

- **File, section:**
  - The addendum § 2, "Direction (owner):": "axis names, tick positions and tick values are derived algorithmically,
    and stay correct through pan, zoom, rotation, tilt, slice, blend and morph."; § 2.1: "It is linear in s, so ticks sit
    at round values by exact arithmetic, with no sampling."
  - `docs/design/principia_chart_composition.md` § 17 (line 1946): "axes carry no ticks, only "A, B, `s`"."
  - `docs/design/principia_lines.md` § "8b. Axes": "A warped axis's ticks fall at round values of the quantity, so they
    are unevenly spaced." and "A secondary scale is refused where the quantity is not monotonic along the edge."
  - `docs/design/principia_chart_reference.md` § 2.1 (R-471 E3): "**The warp is evaluated per pixel,** with that
    pixel's own `U`, `I` and masses".
- **The conflict:** § 17 shows no ticks on a morph frame; the direction asks for them. The prototype's exactness holds
  only for a coordinate linear in the window; a warped quantity, or `E` read with each pixel's own `U` under a tilt, is
  not linear along an edge, so its ticks need § 8b's solve, which is sampling or root-finding. An unfolded axis's
  numbers rise and fall, which § 8b's monotonic rule refuses for a secondary scale.
- **The question:** (i) does the computed axis become § 8b's primary scale on every chart and every frame, § 17's
  no-ticks rule withdrawn; (ii) are warped and per-pixel quantities placed by § 8b's solve; (iii) may a primary scale
  be non-monotonic at an unfold; (iv) the solve's tolerance, which § 8b does not state: a missing value, so a
  calibration requirement (R-71), "N16's tick tolerance", one-way and the owner's, never the implementer's.
  Proposed form, for the owner: a tick's position error in base px (R-583), the unit its mark is drawn in; the owner
  sets the value at the milestone gate's calibration batch, or ties it to an existing calibrated tolerance.
- **Options seen:**
  - (a) Yes to all three: the computed axis is the primary scale; coordinates linear in the window get exact
    arithmetic, the rest § 8b's solve; an unfold's turning point is a marked fold, not a refusal; secondary scales stay
    as § 8b has them.
  - (b) Computed axes on affine charts and frames only; warped charts keep § 8b; morph frames keep "A, B, `s`".
  - (c) Keep both texts as they are.
- **Recommended:** (a).
- **Measurement:** N16's tests (ends exact; ticks at round values to 4 ulps on linear coordinates; solved ticks within
  N16's tick tolerance once (iv) sets it, the test waiting until then; the contour-on-ticks exactness test of lines
  § 12).
- **Applied meanwhile:** nothing.
- **Waits:** N16, N17.

## RQ-781: Where the view's centre may go once every edge extends: onto an unfold sheet, into a clamped or saturated area *(conflict; a one-way door, navigation; the owner's direction of 10 Nov, R-469, R-537; REQ-CHART-060; context: TASK-M2-38, N14)*

*Queued 10 Nov 2026 by the chart addendum; quotes re-read on `origin/main` a5f0aa88.*

*In plain English: today dragging stops when the centre of the view would leave valid systems, though the edges of the
picture can show past it. With extensions everywhere, may the centre cross onto the second sheet, or into a smeared
area?*

- **File, section:**
  - `decisions.md` § R-469's heading: "pan, slice, zoom, tilt and lock are accepted only while the chart's centre passes
    `validate`, an edit that would cross stopping at the last valid value".
  - `decisions.md` § R-537, EX3 (the applied text): "**Bounded charts** (the warp, the mass simplex, the Burrau family):
    the centre stops at the domain's edge, but the window may still show blank past it."
  - `plan/requirements.yaml` REQ-CHART-060: "an edit that would cross must stop at the last valid value along its own
    path".
- **The question:** may the centre (and a lock) sit on an unfold sheet, in a clamped area, in a saturated area?
- **Options seen:**
  - (a) An unfold sheet is valid ground: `validate` reads the sheet, and the centre crosses the fold as it crosses a
    pole; a clamped or saturated area is not: the centre stops at the edge (R-469 unchanged there), the window showing
    the extension past it.
  - (b) The centre stops at every primary-range edge, sheets included.
  - (c) The centre goes anywhere the picture does, a clamped centre reading as its edge system.
- **Recommended:** (a): it keeps R-469's "last valid value" for every true end and treats a real sheet as the
  pole-crossing axis is treated.
- **Measurement:** TASK-M2-38's fuzz extended: no edit leaves the centre on a clamped or saturated pixel, and a pan
  across the fold lands on sheet 2 with `validate` passing.
- **Applied meanwhile:** nothing; R-469 stands.
- **Waits:** TASK-M2-38, N14.
## RQ-758: Do central-difference slopes survive R-926's three slope tiers: as the linearised decoder's `J_D`, as a fourth tier, or only as a test's check? *(conflict; a one-way door; R-926 item 9 and item 11, R-633, R-113; REQ-DEC-034, REQ-DEC-036, REQ-DEC-042, REQ-GEN-018; context: TASK-M6-07, TASK-M5-04)*

*Queued 10 Nov 2026 by the M2 chart re-plan (`plan/charts/m2-chart-system-replan.md`); each quote re-read on
`origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: a chart's "slopes" say how fast the starting system changes as you move across the picture; deep
zoom needs them to shrink a tiny quad to a centre plus slopes. The owner ruled three kinds of slope (exact, implicit,
none). The plan's requirements, written earlier, compute slopes another way, by nudging each coordinate a little
either side ("central differences"). Which one the deep-zoom decoder uses changes its numbers.*

- **File, section:**
  - `decisions.md` § R-926, item 9: "Slopes come in three tiers: exact (dual numbers), implicit, none (per-sample
    difference). A map that cannot give exact slopes is still a valid chart." Item 11: "A small quad's table collapses
    to one cell — that IS the linearised decoder (centre + slopes)."
  - `plan/requirements.yaml`, REQ-DEC-034: "The linearised decode must replace the decode with x₀ + J_D·δ, where J_D is
    central differences of the full chart→IC composite in f64 with deep_zoom §2's step rule … chart-agnostic with no
    per-axis-kind special-casing, and |det J_D| must double as the measure weight." REQ-DEC-036 says the same of
    TASK-M5-04's CPU side.
  - `decisions.md` § R-633's heading: "Every central-difference Jacobian of the decode takes the encode contract's step".
  - `plan/tasks/M6/TASK-M6-07.md` § Goal: "`J_D = ∂(physical)/∂(u,v)` by central differences in f64 over the whole
    chart→IC composite (Φ → decode → canonicalise)".
- **The conflict:** R-926 names exact dual-number slopes as the first tier and gives no tier to central differences;
  REQ-DEC-034 and REQ-DEC-036 rule central differences for `J_D`, and `|det J_D|` is the measure weight (REQ-GEN-018).
  The orchestrator's code reading (10 Nov) found that a dual type needs one implementation of four traits (`IeeeBits`,
  `ExactSign`, `WarpReal`, `LinkReal`) and that its slope is wrong or one-sided at about a dozen decode sites (the
  feasibility kink, `warped_k` at `d = 0`, the exponential map's `r = 0`, the poles, the seed switches), each needing a
  flag set at the site.
- **The question:** for a tier-1 map, is the linearised decoder's `J_D` the dual-number slope (REQ-DEC-034 and
  REQ-DEC-036 rewritten), and what are central differences then: removed, kept as the check, or a tier for maps with
  no exact slope?
- **Options seen:**
  - (a) `J_D` is the tier-1 or tier-2 slope where a map has one; a tier-3 map gets no one-cell patch and every sample
    is evaluated on the CPU, its record carrying its difference from the quad's centre ("per-sample difference");
    central differences at R-633's step stay as the test that checks a dual slope away from flagged sites.
    REQ-DEC-034, REQ-DEC-036 and TASK-M6-07 are rewritten; REQ-DEC-042's `h²` fit and REQ-GEN-018 stand.
  - (b) As (a), but central differences are kept as the slope of a map with no exact slope, a fourth tier. Cheaper deep
    zoom for tier-3 maps; it adds a tier the owner did not name.
  - (c) Central differences stay `J_D` everywhere (REQ-DEC-034 as written); dual slopes serve the lock and the area
    factor only. No rewrite; R-926 item 9 then has no reader in the decoder.
- **Recommended:** (a). It reads item 9 and item 11 together as written, keeps a central-difference check that can
  fail, and makes a tier-3 map pay in CPU evaluations, not in an unruled tier.
- **Measurement:** the plan's slope measurement (§ B, "slopes"): per map, the dual slope against a central difference
  at R-633's step `h_k = ε^{1/3}·max(1, |x_k|)`, away from the flagged sites, within that step's stated truncation and
  rounding bound (each of order `ε^{2/3}`), with a control (the sign flipped on a column well above that bound) that
  must fail; and the one-cell patch's depth with each slope against per-sample
  f64 evaluation.
- **Applied meanwhile:** nothing. REQ-DEC-034 and REQ-DEC-036 stand as written; no task is dispatched (the chart
  pause, R-923).
- **Waits:** the slope task of the plan and TASK-M6-07's rewrite; TASK-M5-04's CPU side.

## RQ-759: The lock's re-solve after a map change: is it continuous, in what metric, and does it keep R-593's offer, R-469's stop and the stored-words select? *(conflict and silence; a one-way door; R-926 item 4, R-849, R-593, R-469, R-866, R-867, R-902; REQ-CHART-146, REQ-CHART-161, REQ-CHART-060, REQ-CHART-010, REQ-CHART-113; context: TASK-M2-62, TASK-M2-65, TASK-M2-73)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: a lock pins one system to one pixel. The owner ruled that after a change of map the lock finds the
new position by least squares. Earlier rulings ask for more: that the pinned system slides smoothly (never jumps) when
the new chart can't show it, that "nearest" is defined, that an impossible centre is only offered a re-centre, and that
an edit stops at the last valid value. Least squares from a few starting guesses can jump between answers.*

- **File, section:**
  - `decisions.md` § R-926, item 4: "The lock is one constraint: "this pixel shows this system". Window edits re-solve
    the centre; a map change re-solves by least squares."
  - `decisions.md` § R-849, the rider: "the anchor slides continuously to its nearest point in B (define "nearest": e.g.
    keep shape and masses where B allows, ease the held-fixed coordinates to B's values along the path) … The jump
    stays only for time-axis to IC-only charts."
  - `plan/requirements.yaml`, REQ-CHART-146: ""Nearest point in B" … must be defined, the chart-space model's Theorem 8
    its construction".
  - `decisions.md` § R-593's heading: "Lock validation gives one notice, never one per pixel: it offers a re-centre, to
    the nearest feasible point of the lock's own centre, only when that centre has gone infeasible".
  - `decisions.md` § R-469's heading: "pan, slice, zoom, tilt and lock are accepted only while the chart's centre passes
    `validate`, an edit that would cross stopping at the last valid value".
  - `docs/design/principia_chart_composition.md` § "17.8 Lock: a based homotopy": "With stored words | **Holds.** … a
    sample at chart coordinate exactly the centre takes them by a comparison"; "Offset alone isn't bitwise … and the
    select is what gives the bits."
  - The prototype design (`orch/charts/prototype-2026-11-10/chart-system-prototype-design.md` § 4, a proposal): "solve
    `P' = argmin_P ‖φ'(P) − X*‖` by Levenberg–Marquardt with a finite-difference Jacobian from four seeds … If `min > 0`
    the new map cannot show `X*`; the pin becomes `φ'(P')`".
- **The conflict:** R-926 rules least squares; it does not say in which metric (the prototype's is the unweighted
  state norm, not REQ-CHART-146's Theorem 8 construction), whether the result must move continuously as the path moves
  (R-849), whether an impossible centre is re-solved automatically or offered (R-593), whether a window edit that would
  leave the valid systems stops (R-469) or re-solves, nor whether the pin's pixel keeps the stored state's words bit for
  bit (§ 17.8, REQ-CHART-113's ends). R-849's time-axis to IC-only jump also meets R-926's time as a coordinate.
- **The question:** what the least-squares re-solve must guarantee beyond item 4's words.
- **Options seen:**
  - (a) Least squares in the metric REQ-CHART-146 defines (Theorem 8's retraction), continued from the previous frame's
    solution along a path so the anchor moves continuously (R-849 kept), a fresh multi-seed solve only at a cut; R-593's
    offer kept for a centre that has gone infeasible on its own map; R-469's stop kept for window edits; the stored-words
    select kept at the pin; R-849's time-axis jump dropped, time being a coordinate the re-solve includes. Continuity
    and "nearest" can't both hold everywhere: at a cut locus of the nearest-point map, where two points of B are
    equally near, a continuous anchor stops being the global nearest (the prototype's generic note § 5: a sliding
    anchor needs a contractible constraint set). Under (a) continuity wins: the anchor follows the local minimiser
    through a cut locus, labelled "not nearest" while another point is nearer, and does not jump there. The followed
    local minimiser can also end: it merges with a saddle and disappears (a fold of the minimiser, likely further
    along once the anchor has passed a cut locus onto the branch that is not the global one). There no continuous
    anchor exists, and under (a) the path **stops** at the last frame where the followed minimiser exists, labelled
    "the followed system ends here", as R-469's stop does for window edits; it never jumps silently, and going on is
    the user's choice, which re-pins (a new lock, not a jump under the old one).
  - (a′) As (a), but nearest wins at a cut locus: the anchor jumps to the new global nearest point there, a labelled
    jump, as R-849's "no labelled jumps under the lock" does not allow.
  - (b) As (a) but R-593 becomes automatic: the re-solve itself moves the centre to the nearest feasible point, with
    the one notice.
  - (c) The prototype's rule: an unweighted least squares from seeds each time, the pin becoming the nearest system it
    finds; no continuity promise, a jump labelled when the basin changes.
- **Recommended:** (a). It keeps R-849's "never jumps" by stopping where the followed minimiser ends, gives up
  global nearness only at a cut locus and says so,
  and reads item 4 as the solve's method, not its guarantees; (b) is the smaller change to R-593 if the owner prefers fewer prompts.
- **Measurement:** G8's slide check, written before the run (`plan/MILESTONES.md`, G8): along a morph into rest and
  Burrau charts, halve the frame step twice; the largest anchor step must fall to at most 0.6 of the previous each
  time; plus the share of frames where a four-seed solve changes basin against the continued solve; and a fixture
  that crosses a cut locus (a path whose target B holds two equally near points to the anchor at one frame, for example
  a rest chart reached from a state symmetric under the swap of bodies 1 and 2), on which (a) must stay continuous and
  label "not nearest", and (a′) must jump once, labelled; and a fixture that ends the followed minimiser (the same path
  continued past the cut locus until the followed branch merges with a saddle), on which (a) must stop at the last
  frame where the branch exists, labelled, with no anchor step larger than the slide check allows before it.
- **Applied meanwhile:** nothing; TASK-M2-62 and TASK-M2-65 are paused with the other chart tasks (R-923).
- **Waits:** TASK-M2-62, TASK-M2-65 and the solver core of TASK-M2-73, as the plan places them.

## RQ-760: What becomes of design § 17.2's other path arms, the mass block, statistics on a blend and folds no axis declares, now that a morph blends the two charts' states? *(conflict and silence; a one-way door; R-926 item 3, R-925 item 4, R-850, R-853, R-408, R-407, R-898; REQ-CHART-112, REQ-CHART-113, REQ-CHART-131, REQ-CHART-150; context: TASK-M2-60, TASK-M2-65, TASK-M2-71, TASK-M2-81)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: the owner ruled that a morph mixes the two charts' systems directly, and that a blend is itself a
chart. The design had other ways to go between charts: mixing one axis's value, bending the pair map, mixing view
fields. It is not said whether those survive, whether "linear in masses" means the masses or their logarithms as the
design has it, whether area statistics may be taken over a blend, or which copy counts when a blend shows one system
twice.*

- **File, section:**
  - `decisions.md` § R-926, item 3: "A morph is a blend of two charts' states (linear in masses, momenta and time; slerp
    on shape). Both ends are exactly the end charts." § R-925, item 4: ""Morph" means the system blend. A blend is a
    chart."
  - `docs/design/principia_chart_composition.md` § "17.2 The mechanism": "**The rule: blend at the last layer where the
    two ends differ, and run everything downstream once, on the blend.**", with the arms `Path::AxisBlend`,
    `Path::PairMap`, `Path::PerField`, and the line's block "mass `μ_k = ln(m_k/m₀)` `a + s(b − a)`".
  - Same file, § "17.6 Per-axis and pair-map morphs": "between two pixels at `E* = −0.1`, the blended state misses by
    0.054 median and 2.1 worst, against 1.4·10⁻¹⁶ with the step run after the blend."
  - Same file, § "17.5 A frame: valid, reproducible, recorded": "A frame's `system_image` is unknown, since a blend may
    fold (§ 17.6), so `measure` is `none`, no statistic is admitted (`admit_statistic` refuses)".
  - `plan/requirements.yaml`, REQ-CHART-112: "a path type with arms for tilt, zoom, plane rotation, per-axis blend, pair
    map, line and per-field, blending at the last layer where the ends differ".
  - `decisions.md` § R-408's heading: "Area statistics count each system once, through the axis types".
  - The prototype design § 13, items 4 and 5 (a proposal): "The `α×β→` sphere blend is still 0.4–1.0% wrong backwards
    and the cause is not found." "A fold that is not a declared extension has no rule for the true copy."
- **What R-925 and R-926 settle:** a morph blends states; a turn of the window within one map, `Path::Tilt`,
  `Path::Zoom` and § 17.7's plane rotation, is a tilt or a window edit, not a morph; a blend is a chart; the ends are
  the end charts exactly (REQ-CHART-113's exact selection of `s = 0` and `s = 1` gives that, and stays). R-850 already
  says mid-morph frames never claim held values, so § 17.6's held-`E*` miss is a property of the morph, labelled.
- **The question:** (i) whether `AxisBlend`, `PairMap`'s bend and `PerField` survive, and as what; (ii) whether
  "linear in masses" is § 17.2's `μ_k = ln(m_k/m₀)`; (iii) whether a blend chart admits area statistics, and under what
  measure; (iv) which copy is the system's position where a blend folds and no axis declares an extension.
- **Options seen:**
  - (i-a) Retired as morphs and offered as maps: a chart whose axis is the blend of two quantities, or the bent pair
    map, is a map like any other, navigable and lockable. (i-b) Kept as morph families beside the state blend. (i-c)
    Retired outright.
  - (ii-a) The mass block in `μ`, as § 17.2 and REQ-CHART-112 have it. (ii-b) Linear in the masses `m_k` themselves,
    which reaches every ratio too but blends differently.
  - (iii-a) No statistic on a blend chart (§ 17.5 kept): the measure stays `none`. (iii-b) Statistics admitted where
    a signed fold indicator keeps one sign over the view (no fold), refused elsewhere. No such indicator exists yet:
    the Gram area factor is unsigned (§ 17.11: "a fold reads as a zero crossing, which a coarse grid can step over"),
    so (iii-b) needs one defined first (R-72), for example the sign of the 2 × 2 Jacobian determinant of a fixed 2D
    key of the swept state composed with the chart, and it sees only folds the sample grid resolves.
  - (iv-a) No true copy: a folded pixel's backwards answer is the nearest one, labelled, and folded pixels are excluded
    from statistics. (iv-b) A rule declared per blend (for example the copy continuous with `s = 0`).
- **Recommended:** (i-a), (ii-a), (iii-b) once its indicator is defined (else (iii-a)), and (iv-a): they keep "morph" to the one meaning the owner gave, keep the
  design's measured mass block, and keep R-408's "count each system once" honest on a blend.
- **Measurement:** per blend pair on the kernel's charts (latent to the shape sphere's exponential map, `(L_z, E)` to the
  sphere), the folded share by the signed fold indicator (once defined, R-72) and the backwards wrong share at `t = 0`, written as the plan's open item
  "blend wrong backwards"; the `α × β` to sphere blend re-measured on the kernel's latent chart before any cause is
  sought.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M2-60's contract text, TASK-M2-65's line and TASK-M2-81's flags, as the plan places them.

## RQ-761: Time as a coordinate on the integrator's integer grid: a per-sample stop index, `bounded`, a playhead moved backwards, and the column march *(conflict and silence; a one-way door; R-926 items 2 and 3, R-925 item 5, R-565, R-602, R-842, R-849, R-92; REQ-INT-038, REQ-SCHED-010, REQ-CHART-123, REQ-CHART-174; context: TASK-M2-83, TASK-M4-05, TASK-M2-82)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: the owner made time an ordinary coordinate of every chart, so a picture can tilt into time and a
morph can run while time plays. The integrator, though, only stops on a fixed grid of times counted in whole steps, and
today every sample of a dispatch stops at the same time. How a sample with its own time stops, what its "ran to the
end" flag means, and what playing backwards does are not said.*

- **File, section:**
  - `decisions.md` § R-926, item 2: "Time is the last coordinate of every map. The playhead is the window centre along
    time." Item 1: "An axis change is a tilt to 90 degrees." § R-925, item 5: "A morph supports time running, and time
    is a supported chart axis."
  - `decisions.md` § R-565's heading: "steps are sized from it, the march stops only on it, the playhead reads boundary
    states, so live march, catch-up and re-boot reach the same state bitwise".
  - `plan/requirements.yaml`, REQ-INT-038: "The horizon test must be an integer macro-step counter against
    `horizon_steps = ⌈T/dt_macro⌉`, never accumulated float time."
  - `docs/design/principia_chart_composition.md` § "18.2 Against the core": "Rows are kept as **integer boundary
    indices**, since `⌊(k·10⁻³)/10⁻³⌋ ≠ k` for 731 of the 65,535 `k` in f64"; § "18.5 Cost per column", C1: "The state
    word differs by design: `bounded` belongs to the horizon".
  - `plan/requirements.yaml`, REQ-CHART-123: "run each column once from t = 0 through the trajectory interface (§ 18.7),
    read rows as integer boundary indices per occupant … and cut to an IC-only chart."
  - The prototype design § 5 (a proposal): "The GPU integrates each sample until `t ≥ max(0, T + τ)`."
- **What R-925 and R-926 settle:** time is a coordinate of every map, not a chart mode (R-842 amended in part); a tilt
  may take it (REQ-CHART-174's "time never a tilt target" goes); a morph blends time linearly and may run with time
  playing (REQ-CHART-123's cut to an IC-only chart goes).
- **The question:** how a sample's own target time meets R-565's grid and REQ-INT-038's counter, and what `bounded`,
  backwards play and the column march become.
- **Options seen:**
  - (a) A per-sample stop index `k_stop ≤ horizon_steps`, a `u32` in the record, the host mapping each target time to
    the latest boundary at or before it in integers on the occupant's grid; `T` stays the per-dispatch uniform and
    `bounded` stays the horizon's; a target below the current state re-boots from `t = 0` (R-565 bitwise); a
    time-axis chart keeps the column march as an implementation that gives the same bits. A sample stopped at its own
    `k_stop` needs a label, one of:
    - (a-i) a new state, "reached its time": a sixth label beside escape, collision, bounded, the failures and running in
      canonical spec § 9's totality invariant ("every pixel gets a labelled output (escape/collision/bounded,
      sim_failed/decode_failed, or running)") and in the payload's state enum, each consumer of the enum (the
      outcome legend, the statistics, export) handling it;
    - (a-ii) no new label: `k_stop` is the sample's own horizon and the sample is labelled `bounded` there, which
      changes `bounded`'s meaning from "reached the dispatch's horizon `T`" to "reached its own target" (§ 18.5 C1's
      "`bounded` belongs to the horizon").
  - **The staircase, under either (a):** on a view tilted toward time, once a pixel's step in time falls below the
    grid's spacing (`Δ_sync`, or the macro-step on an unregularised occupant), neighbouring rows read the same boundary,
    and the picture shows bands whose edges are level sets of the stop index, not dynamics (`principia_01_pitfalls.md`
    § 1.7: "Smooth, clean geometric boundaries in a chaotic field … are level sets of something smooth — usually a
    termination time"; § 1.2 on quantised end times). The time-axis mode already says so for its rows (§ 18.2, "Time
    zoom floors at one boundary"); a tilted view needs the same: its zoom along time floors at one boundary, or the
    bands are drawn with a label.
  - (b) A per-sample float horizon `t ≥ T + τ`, as the prototype: off R-565's grid, so live march and re-boot no
    longer meet bitwise.
  - (c) Per-dispatch horizons only: one dispatch per distinct target time, the per-sample case going through the
    trajectory interface.
- **Recommended:** (a) with (a-i), and the zoom along time floored at one boundary. It is R-565 and REQ-INT-038
  applied to a per-sample target, keeps the bitwise re-boot and `bounded`'s meaning, and needs one word in the record; (c) costs a dispatch per target time on a space-time slice.
- **Measurement:** § 18.5's C1 re-run with per-sample stops (every row bitwise against a run with that row's boundary
  as horizon, `bounded` excepted by design); a zoom-in-time check on a view tilted 90° toward time, the rows per
  boundary counted at each zoom and the floor shown to stop the zoom (or label the bands) where a row's time step
  falls below one boundary; and the cost of a morph with time running, every sample re-integrated per
  frame, measured against the frame budget on the Mac.
- **Applied meanwhile:** nothing. TASK-M4-05's interface is not built.
- **Waits:** TASK-M4-05 (the integrator's entry and its record), TASK-M2-83 (re-scoped by the plan).

## RQ-762: The window's per-coordinate scale: R-507's and R-920's units as `Σ`, and the scale of time and of coordinates that have no slider *(silence; a one-way door, a physics definition; R-926 item 1, R-507, R-920, R-474, R-92; REQ-CHART-188, REQ-CHART-064; context: TASK-M2-61, TASK-M2-63, TASK-M2-36)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: turning the window between two coordinates needs a rule for how far one unit of each goes, or the
angle means nothing. The rulings already give units for the screen axes and the sliders. Time, and any coordinate with
no slider, has none.*

- **File, section:**
  - `decisions.md` § R-926, item 1: "A chart is a map (named coordinates -> system state + time) composed with a flat
    window (centre + basis). Pan, zoom, rotate, tilt and slice are edits of the window."
  - `decisions.md` § R-507's heading: "the screen axis in units of the axis's full range at zoom 1, the hidden coordinate
    in its slider span, so no unit depends on zoom; the latent chart's tilt stays the decoder's basis edit".
  - `decisions.md` § R-920: "A hidden coordinate's slider span is the full range at zoom 1 of the screen axis that
    carries the same coordinate on the registry's own charts, read at that chart's home window".
  - The prototype design § 1 (a proposal): "Each coordinate `k` declares a scale `σ_k` and a home value `h_k` …
    `G = diag(1/σ_k²)`, time excluded."
- **The silence:** R-507 and R-920 give each screen axis and each slider a unit, which is a per-coordinate scale, but
  time has no slider span and no zoom-1 range, the latent chart's tilt is "the decoder's basis edit" with no scale, and
  a map's coordinate that is neither an axis nor a slider on any registry chart has none. A tilt toward time (R-926,
  items 1 and 2) needs one.
- **The question:** is the window's scale `σ_k` R-507's and R-920's unit for every coordinate that has one, and what is
  it for time and for the rest?
- **Options seen:**
  - (a) `σ_k` = R-507's and R-920's unit wherever defined (the latent coordinates' span 2, uniform, so the latent tilt is
    unchanged); time's `σ` is the chart's home time window, the span of `[0, T]` at zoom 1; any other coordinate
    declares its own `σ` with its map, a definition each under physics review (R-72).
  - (b) As (a), but time's `σ` is a fixed physical constant (for example `Δ_sync` times a count), the same on every
    chart.
  - (c) No metric: tilts toward a coordinate with no unit are refused. Conflicts with R-925's "no refusals" spirit and
    R-926 item 1.
- **Recommended:** (a). It adds no unit the rulings don't already have, and time's unit then behaves as R-920's spans
  do, the same fraction of a home window on every coordinate.
- **Measurement:** R-920's own checks reused for time (a 90° tilt into time at zoom 1 equals the time axis's home
  extent, to rounding; a pan at fixed `τ` leaves the span bitwise unchanged), written before any run.
- **Applied meanwhile:** nothing.
- **Waits:** the window type of TASK-M2-61's revised sketch; TASK-M2-63's held-coordinate tilts.

## RQ-763: Does R-864's "bitwise reproduction first" still bind stage 1 in full, now that every operation becomes an edit of a general window? *(conflict; a one-way door; R-864 item 3, R-894, R-917, R-919, R-926 item 1; REQ-CHART-147, REQ-CHART-113, REQ-CHART-099, REQ-CHART-156; context: TASK-M2-61, TASK-M2-64)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: before changing any arithmetic, the plan was to rebuild every one of today's view operations
inside the new object so that each gives exactly the same bits as today, then merge them into one per operation. With
the window now one general centre-and-basis, about 35 of today's routines would be rebuilt exactly only to be thrown
away, and a general rotation gives different last bits on about one angle in nine.*

- **File, section:**
  - `decisions.md` § R-864, item 3 (its heading): "today's operations re-expressed as named families of it and their old
    routes deleted; bitwise reproduction first".
  - `decisions.md` § R-894's heading: "rounding-only differences resolved to one canonical formula with a documented bit
    change and a conversion test … bit-for-bit reproduction is stage 1's safety net".
  - `plan/MILESTONES.md` § "M2 — The chart system: charts, decode and encode, and the non-visual chart core", G1:
    "Every existing chart and view operation through the composition, bit for bit (R-864 item 3; TASK-M2-61)".
  - `decisions.md` § R-926, item 1: "Pan, zoom, rotate, tilt and slice are edits of the window."
  - The chart review (`orch/charts/prototype-2026-11-10/chart-review-2026-11-10.md` § 5, point 3, a proposal): "Measure
    the `≟` pairs first, and only freeze a separate leaf where a pair differs."
- **The conflict:** R-926 makes every operation an edit of `(c, Q)`; R-864 item 3 and G1 require today's arithmetic
  reproduced bitwise through the new object before any change. The orchestrator's reconciliation (10 Nov) counts 35
  stage-1 leaves, of which a generic rotation of `Q` reproduces today's shared-basis tilt bitwise on about 89% of angles
  (type sketch v10, physics note A1, not re-measured).
- **The question:** does stage 1 still reproduce every one of today's operations bitwise through the new types, or only
  those the generic edit reproduces, the rest converted at once with a recorded bit change?
- **Options seen:**
  - (a) As written: stage 1 reproduces all of today's leaves bitwise in the `(map, c, Q)` types, and TASK-M2-64
    converts them with documented bit changes (R-894).
  - (b) Measure first: each of today's operations against its generic edit; a bitwise-equal one is one leaf at once; a
    differing one is converted in stage 1 with R-894's documented bit change and conversion test, never kept as a
    temporary leaf.
  - (c) No bitwise stage: every operation converted at once, conversion tests throughout.
- **Recommended:** (b). It keeps R-864's safety net (every change measured and recorded) and drops only the scaffolding
  the measurement shows is not needed; it is the review's point 3, which the owner has not ruled.
- **Measurement:** the D2 pair measurement, criterion written before the run: per operation, at f64 and f32, the share
  of a fixed grid of inputs on which today's route and the generic `(c, Q)` edit agree bitwise.
- **Applied meanwhile:** nothing; R-864 item 3 and G1 stand as written.
- **Waits:** TASK-M2-61 (and its type sketch, revised from v10 for the owner under R-864 item 1 and R-919).

## RQ-764: The momentum seed: does the expansion seed `w = r/√I` replace R-82's largest-norm rule, everywhere or as a named option? *(silence; a one-way door, physics that changes every invariant chart's image; R-82, R-865, R-849; REQ-DEC-021; context: TASK-M2-57, TASK-M2-61, TASK-M2-85)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: on charts that fix the energy, the leftover energy is put into the bodies' motion along one
"seed" direction. Today's rule picks the largest of four seeds, and the choice flips across a line in the picture, so
the picture shows a vertical edge that is not physics. The independent review proposes a seed with no flip (a uniform
expansion). It reads as the owner's decision, but no ruling records it.*

- **File, section:**
  - `decisions.md` § R-82: "Seed: among seeds with ‖w⁽²⁾‖²_m > ε_w, take the largest norm; break ties by seed order."
  - The chart review (`orch/charts/prototype-2026-11-10/chart-review-2026-11-10.md` § 2, "The seed seam: remove it,
    don't declare it (owner's decision)", a proposal): "**`w = r/√I` has no seam.** … it changes every invariant and
    mixed-energy chart's image where `K* > K_min`, not only at the seam, so it is a recorded conversion."
  - The prototype design § 13, item 9 (a proposal): "The seed: the composed map uses rigid rotation plus uniform
    expansion. Not ruled."
  - Code: `crates/kernel/src/decode/invariant.rs:313` (the seeds) and `crates/kernel/src/chart/burrau.rs:110` (Burrau's
    seed select).
- **The silence:** neither R-925 nor R-926 names the seed; no ruling on `main` records `w = r/√I`. Its seam sits in the
  slope sites R-926 item 9 must flag (the seed switch at `z_α = 0` and at `ν = 1/3`).
- **The question:** does `w = r/√I` replace R-82's seed rule, and where?
- **Options seen:**
  - (a) Everywhere, the invariant charts and Burrau's included: a recorded conversion of every image where
    `K* > K_min` (before TASK-M2-61's baseline), `NO_QUALIFYING_SEED` unreachable for `I > 0`, R-82's rule kept as a named
    option with its seam declared.
  - (b) On the invariant charts only; Burrau keeps R-82's seed with a declared seam.
  - (c) R-82 stands; the seams are declared and flagged as one-sided slopes.
- **Recommended:** (a), as the review argues; with a check of the invariant charts' default pin, the equal-mass Lagrange
  triangle, where every pixel then starts a homographic orbit whose break-up is set by rounding, labelled.
- **Measurement:** the seed jump across `z_α = 0` on the kernel's `(L_z, E)` chart before and after (the review's 1.9545
  predicted and measured), and the invariant `(L_z, K)` matrix case run under the legacy option and recorded (R-649).
- **Applied meanwhile:** nothing; R-82 stands.
- **Waits:** the seed conversion of the plan, which must land before TASK-M2-61 records its baseline.

## RQ-765: With the integrator taking only records, what are the GPU decode entries, R-837's cap of baked entries and the lowering labels for? *(conflict; a one-way door; R-926 items 5 and 6, R-925 item 1, R-837, R-116, R-864 item 5; REQ-PERF-116, REQ-CHART-098, REQ-INT-070, REQ-SYS-022, REQ-DEC-007, REQ-COL-006, REQ-RENDER-027; context: TASK-M2-06, TASK-M4-05, TASK-M4-27, TASK-M4-33, TASK-M4-34, TASK-M2-66)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: the graphics card can run the chart's own decoding today, as a separate step used by tests and by
the debug pictures. The owner ruled that in the product the card only integrates systems the CPU has prepared. It is not
said whether that GPU decoding step stays (for tests and the fragment's pictures) or goes, nor what becomes of the
limit on how many such decoders are compiled, or of each chart's "lowering" label.*

- **File, section:**
  - `decisions.md` § R-926, item 5: "The CPU evaluates charts in f64. The GPU only integrates records. No chart code in
    the integrator."
  - `decisions.md` § R-837's heading: "The axis cap is 4 baked entries beyond the gallery, counted as entries,
    decode-only, created on first use, a pair past the cap running as CPU batch".
  - `plan/requirements.yaml`, REQ-INT-070: "The kernel must take the chart description … as a uniform and per sample map
    → z → decode → (m, r, p) → integrate in-kernel, never from a pre-decoded IC buffer, keeping per-frame CPU cost O(1)."
  - `plan/requirements.yaml`, REQ-CHART-098: "A validated chart must carry its lowering (uniform affine, baked entry or
    CPU batch) … and resolve must return the baked entry for a baked (H, V, M) and CPU batch for any other."
  - `plan/requirements.yaml`, REQ-PERF-116: "The closed-form axis pairs baked into the GPU kernel beyond the gallery must
    not exceed the ledger's cap".
  - Code: `crates/kernel/src/stage/decode.rs:771` (the GPU decode entries), `crates/engine/src/decode_dispatch.rs:3`
    (its callers are tests and validation gates).
- **What R-926 settles:** REQ-INT-070's in-kernel decode, core seam 3 and R-839 go; the integrator decodes nothing.
  R-925 keeps the fragment's generated code where a chart has closed-form code.
- **The question:** what remains of the compute-side GPU decode entries, R-837's cap and REQ-CHART-098's three
  lowerings.
- **Options seen:**
  - (a) The GPU decode, canon and encode compute entries stay as test entries: the cross-backend parity checks
    (TASK-M4-27's) and the agreement presets that compare the fragment's generated code; R-837's cap and REQ-PERF-116
    are retired as moot; REQ-CHART-098's label becomes the fragment route, "closed form" or "table", never a refusal.
  - (b) As (a), but R-837's cap is re-read as a cap on the fragment's generated chart sections, which every pipeline
    parses whole.
  - (c) The compute decode entries are removed with their tests. Conflicts with R-908's "product-correctness tests are
    never retired" where they check the fragment's generated code.
- **Recommended:** (a), with (b)'s cap measured before it is retired: the fragment module's size and compile time with
  every chart's section is what R-837's measurement was about.
- **Measurement:** the fragment pipeline's compile time and module size with all generated chart sections, on the Mac
  and on lavapipe, against C3's 2 s start-up limit (R-872), criterion as R-837's measurement wrote it.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M4-05's rewrite, TASK-M2-66's labels, TASK-M4-33 and TASK-M4-34's GPU sides.

## RQ-766: A large quad's forward table: one node per sample, evaluated exactly, or a coarser grid the GPU interpolates; and when it becomes a one-cell patch *(silence; a one-way door, it changes the starting states; R-926 items 6 and 11, R-925 item 3, R-90, R-599, R-632; REQ-DEC-033, REQ-DEC-005, REQ-DEC-051, REQ-SCHED-024; context: TASK-M5-04, TASK-M6-07, TASK-M6-08)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: the CPU works out the starting system for each quad and sends it with the quad. It can do that for
every sample exactly, or for a coarser grid that the graphics card fills in between. Filling in changes the starting
systems slightly. When a quad is small enough to be sent as just a centre and its slopes is ruled already (R-90); the
prototype used a different trigger.*

- **File, section:**
  - `decisions.md` § R-926, item 6: "Lookup tables are PER QUAD: a forward table (window position -> state) built on
    the CPU, bundled with the quad and dispatched with it." Item 11: "Grids for large quads, one-cell patches for small
    ones." § R-925, item 3: "The forward direction needs no table: the CPU evaluates the chart per quad and the result is
    bundled with the quad when it is sent to the GPU."
  - `decisions.md` § R-90: "Switch to the linearised decoder when the full decoder's adjacent samples give
    bitwise-identical ICs, with ℓ_switch = 20 as an upper bound (whichever comes first)."
  - `plan/requirements.yaml`, REQ-DEC-005: "The integrator must have exactly one input type, the decoder's canonical
    (mᵢ, rᵢ, pᵢ) in the dimensionless system G = M = I = 1".
  - The prototype's lookup-table notes (`lut-exploration-2026-11-10.md` § 5, a proposal): one patch "is within a sample
    from about ×64 to ×256 in".
- **The silence:** R-925 item 3 says the forward direction needs no table; R-926 item 6 says each quad carries a
  forward table and item 11 says "grids for large quads". Neither says whether a grid has a node per sample.
- **The question:** is a large quad's forward table one node per sample (each start the CPU's f64 evaluation rounded
  once), or coarser and interpolated on the GPU; and does R-90's trigger decide the switch to the one-cell patch?
- **Options seen:**
  - (a) One node per sample above R-90's switch, exact; the one-cell patch, a centre and slopes, below it, R-90's
    trigger unchanged; the record the integrator reads is REQ-DEC-005's `(m, r, p)`. R-90's "adjacent samples give
    bitwise-identical ICs" compares the **narrowed f32 records**, the ICs the integrator actually starts from, as it
    did when the full decode ran in f32. Comparing the f64 evaluations instead would almost never fire before
    `ℓ_switch = 20` (f64 neighbours stay distinct to about level 48), so the switch would always be the upper bound,
    and a quad between the f32 collapse and level 20 would keep sending identical f32 records; the choice moves the
    switch depth and with it the starting states. (a-f64), the f64 comparison, is the alternative.
  - (b) A coarser grid the GPU interpolates bilinearly at every depth, a node spacing per chart: fewer CPU evaluations,
    starting states off by the interpolation error.
  - (c) (a) with the prototype's switch: a patch once it is within a sample of the exact evaluation, measured per chart.
- **Recommended:** (a), comparing the narrowed f32 records. It is the only option where a starting state is the chart's own system to rounding, which
  REQ-DEC-005 and every round-trip gate assume; (c)'s trigger can be measured against R-90 without changing it.
- **Measurement:** on the kernel's latent and exponential-map sphere charts, the CPU evaluations per frame under (a)
  against the frame budget; (b)'s interpolation error against the sample spacing; the depth at which R-90's trigger fires
  on the f32 records and on the f64 evaluations, against the prototype's trigger.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M5-04's quad request and TASK-M6-07, as the plan rewrites them.

## RQ-767: How accurate must the table route be outside a morph, and may it refine on the chart's generated forward code? *(silence; a one-way door, a tolerance; R-926 items 7 and 8, R-925 items 1 and 2, R-116, R-542, R-919 F11; REQ-ENC-024, REQ-ENC-029, REQ-CHART-158; context: Chart position's build and the decode and round-trip presets)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: where a chart has no backwards formula, the picture finds a system's place on it from a table.
The owner said an approximate answer is fine during a morph. Outside a morph, nothing says how close the answer must
be, or whether the table only gives a first guess that the chart's own forward code then sharpens.*

- **File, section:**
  - `decisions.md` § R-926, item 7: "It is a baked backwards table with a table search as the fallback. Approximate is
    acceptable during a morph."
  - `decisions.md` § R-919, F11: "blend frames, which have no inverse (R-899 Q8), found to pixel resolution."
  - The prototype design § 7.5 (a proposal): the search refines "by damped Gauss–Newton on the bilinear table".
  - `docs/design/principia_colour_composition.md` § "3. The `ctx` contract": every chart's `Φ` "is evaluated
    fragment-side, each through its own generated section".
- **The silence:** the accuracy of the backwards answer on a plain chart with no closed-form inverse is not stated; a
  search refined on the bilinear table is limited by the table's spacing, while a refinement on the chart's generated
  forward code, which every kernel chart has, drives the residual in state to f32 rounding for a state that lies on
  the chart's image (one decoded from a pixel); the error in chart coordinates is then about that residual divided by
  `σ_min(J)`, the chart Jacobian's smallest singular value. A running or saved state generally lies off the chart's
  two-dimensional image, so even a converged solve leaves a residual: its distance off the chart, which no solver
  removes. The state map treats it as a projection: `docs/design/principia_state_map.md` § "SM1. Definition" (lines
  108 to 117): "`P_T` is the target chart's projection, the one the trace uses (`principia_trajectory_viewing.md` §3;
  trace follow-up F1)." (Codex review thread on PR #473.) SM1 defines no distance in state space: its table reads
  `P_T`'s coordinates through the encode on charts that have a closed-form read, and its only nearest point is a curve
  axis's, § "SM1. Definition" (lines 154 to 155): "the targets are the app's own charts, through `encode_into` and
  `curve::nearest`." For an off-image state on a chart with no closed-form inverse it is silent on which nearest point
  is the answer and on the metric that distance is measured in; both are definitions owed (R-72).
- **The question:** what accuracy the table route must reach outside a morph, and whether it may refine against the
  generated forward code; and, for an off-image state on a chart with no closed-form inverse, which nearest point is
  the answer and in what state-space metric `W` (definitions, R-72).
- **Options seen:**
  - (a) The table seeds the search; Newton steps run on the chart's generated forward code where the chart has it
    (every kernel chart, and a blend of two such charts), so the residual in state reaches f32 rounding outside a
    morph; the table alone only where no forward code exists. The criterion is stated apart for the two kinds of
    state:
    - *On the chart's image* (a state known to lie on it, such as one decoded from a pixel): the residual in state
      reaches f32 rounding. The answer is not f32-accurate in chart coordinates everywhere: its error is about the
      residual over `σ_min(J)`, which is unbounded at the sites a chart flags (the poles, `r = 0`, the feasibility
      boundary). The bound is a calibration (R-71), stated in chart coordinates with its conditioning, as
      `ε_res / σ_min(J)` with `ε_res` the residual tolerance, reported beside `σ_min(J)` at each sample, and a sample
      whose `σ_min(J)` falls below a stated floor is labelled ill-conditioned rather than held to the bound.
    - *Off the chart's image* (running and saved states): with `f` the chart's generated forward code from chart
      coordinates `u` to the encoded state `x`, the converged residual `r = f(u) − x` is the projection distance
      `‖r‖_W`, `P_T`'s distance off the chart, reported as a measured quantity and never read as solver error. Two
      definitions it rests on are owed (R-72), as the silence says, and neither is chosen here: (i) which nearest
      point counts as the answer for an off-image state on a chart with no closed-form inverse (SM1's `P_T` reads
      coordinates through the encode where a closed-form inverse exists, and its only nearest point today is
      `curve::nearest` on a curve axis); (ii) the state-space metric `W` it is measured in. The encoded state mixes
      shape, momentum and mass coordinates in different units, so every number reported here (`‖r‖_W`, `‖J‖_W`,
      `σ_min(J)` in `W`, the optimality) depends on `W`, and the first-order optimality measure is `JᵀWr`, not `Jᵀr`.
      Convergence is judged apart from the distance, by first-order optimality: `‖JᵀWr‖ / (‖J‖_W·‖r‖_W)`, or the
      Newton step in chart coordinates, below a stated tolerance, a calibration (R-71). That ratio is 0/0 as
      `‖r‖_W → 0`, so a sample with `‖r‖_W ≤ ε_res` (every running state at `t = 0`, and any later one as close) is
      judged by the on-chart criterion above instead, so that it is binned deterministically. Beside each sample, the
      chart-coordinate error bound `‖JᵀWr‖ / λ_min(H)`, with `H = JᵀWJ + Σᵢ (Wr)ᵢ ∇²fᵢ` the objective's Hessian
      off the image (`Σᵢ rᵢ ∇²fᵢ` when `W` is the identity); the residual-curvature term is kept because `H` loses
      positive definiteness once `‖r‖_W` nears the image's radius of curvature, even where `σ_min(J)` is healthy.
      `λ_min(H)` is reported with it, and the ill-conditioned floor applies to `λ_min(H)`: a sample below a stated
      floor, a calibration (R-71), is labelled ill-conditioned rather than held to the bound, and a sample with
      `λ_min(H) ≤ 0`, where the nearest point is not unique, is reported apart. The off-chart projection distance is
      the quantity the trace's residual will be defined against when that work is planned.

    Each criterion is written first.
  - (b) The table alone, labelled approximate with its bound, the table's spacing, everywhere.
  - (c) Pixel resolution everywhere, as R-919 F11 has it for blend frames.
- **Recommended:** (a). It costs a few forward evaluations per pixel, which the prototype's Metal figures (7 to 20 ns a
  sample for the search) suggest is cheap, and makes "approximate" true only where the owner allowed it.
- **Measurement:** on the kernel's charts at `t = 0` and after a run, for (a) and (b), each binned by `σ_min(J)` with
  the flagged sites reported apart, the on-chart and off-chart samples binned and reported apart: for on-chart samples
  (states decoded from pixels, at `t = 0`), the round-trip error in cells, the wrong share and the state residual
  against `ε_res`; for off-chart samples (running states after the run, and saved states, with `‖r‖_W > ε_res`),
  also binned by `λ_min(H)`, the projection distance `‖r‖_W` (median and worst), the first-order optimality
  `‖JᵀWr‖ / (‖J‖_W·‖r‖_W)` at the answer, the share that converged, the wrong share (an answer whose `‖r‖_W` exceeds
  the reference's by more than a stated margin in `W`, a calibration (R-71); the reference is the best converged
  answer over a multi-start refinement on the chart's forward code, seeded from every table cell), the bound
  `‖JᵀWr‖ / λ_min(H)` beside `λ_min(H)` and `σ_min(J)`, and the samples with `λ_min(H) ≤ 0` counted apart; and the
  per-sample cost on Metal; against a criterion written before the run.
- **Applied meanwhile:** nothing.
- **Waits:** the backwards-table task of the plan; Chart position's build.

## RQ-768: `ctx.chart.z` is a forward value in the corpus; R-926 item 7 lists it among the backwards direction's needs *(conflict in reading; a one-way door, a reading of the owner's words; R-926 item 7, R-116, R-72; REQ-COL-057, REQ-COL-006; context: the colour nodes that read `ctx.chart.z`)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: `ctx.chart.z` is the value of the chart's coordinates at the pixel, which a colour node can read.
The corpus works it out going forwards, from the pixel. The owner's list puts it with the needs of the backwards
direction (from a running system to its place on a chart). Either the owner means the forward value, served by tables
where a chart has no shader code, or a new backwards value.*

- **File, section:**
  - `decisions.md` § R-926, item 7: "The backwards direction (running state -> chart position) exists ONLY for the
    fragment-side needs: chart-position colouring / the state map, ctx.chart.z, and the decode/round-trip debug
    presets."
  - `docs/design/principia_colour_composition.md` § "3. The `ctx` contract": "`ctx.chart.z` is the eight controls `Φ`
    hands the shared decoder at the pixel's chart point, `ctx.chart.slice_uv` … The fragment obtains it … by evaluating
    the chart's own `D ∘ Φ` in WGSL at the pixel's `ctx.chart.slice_uv`".
- **The conflict:** in the corpus `ctx.chart.z` is computed forwards from the pixel and needs no inverse.
- **The question:** which reading the owner means.
- **Options seen:**
  - (a) Item 7 lists the fragment side's chart needs: `ctx.chart.z` stays forward, from the generated code where the
    chart has it and from the quad's forward table where it has none. R-926 item 6's forward table maps window
    position to state, and `ctx.chart.z` is the controls `Φ` hands the decoder, not the state, so on a table-route
    chart (a) needs one of two sources:
    - (a-table) the forward table carries `z` beside the state at each node (eight more f32 words a node), the
      fragment interpolating it as it does the state, with the table's spacing its error; `z` keeps
      colour_composition § 3's meaning;
    - (a-window) `z` is the flat window's output, `c + Q·u`, affine and exact with no table, which is the map's named
      coordinates at the pixel; it equals the decoder's controls only on a chart whose map takes the controls as its
      named coordinates, so on any other chart it changes what `ctx.chart.z` means.
  - (b) `ctx.chart.z` becomes, or gains a sibling that is, the running state's chart position, read backwards.
- **Recommended:** (a-table), which changes nothing in colour_composition § 3 but the table fallback; (a-window) is
  cheaper and exact, and is the alternative if the owner reads `ctx.chart.z` as the window's coordinates.
- **Measurement:** none needed; a reading.
- **Applied meanwhile:** the plan reads (a) and marks it "reading, open".
- **Waits:** the colour work that builds `ctx.chart.z` on a table-route chart.

## RQ-769: May the registry and composed-axis tasks (stage 3) run beside stages 1 and 2? *(conflict; a one-way door; R-863, R-916, R-917; context: TASK-M2-66 to TASK-M2-76)*

*Queued 10 Nov 2026 by the M2 chart re-plan; each quote re-read on `origin/main` 054e7f72. Nothing is chosen here.*

*In plain English: the chart work was ordered in five stages, each starting after the last. The registry of
primitives, the warps, the holds and the curved slices use nothing from the lock or the path object, so they could be
built in parallel, roughly halving the chain of about twenty tasks in a row.*

- **File, section:**
  - `decisions.md` § R-863's heading: "built in five stages, each merging only with the previous stage's tests green".
  - `plan/tasks.yaml`: TASK-M2-66 to TASK-M2-76 each depend on TASK-M2-65, by stage order only.
  - The chart review (`chart-review-2026-11-10.md` § 5, point 1, a proposal): "Run stage 3 beside stages 1–2, not after
    them … This amends R-863's "each stage after the last", so it is the owner's call."
- **The question:** may stage 3's tasks that touch no lock run beside stages 1 and 2, joining at the tasks that do
  (region D's slide arm, points of interest, path flags)?
- **Options seen:**
  - (a) Two tracks: TASK-M2-66, M2-67, M2-69 to M2-72, M2-74 and M2-76 depend on TASK-M2-61's baseline only; the arms
    that touch a lock (TASK-M2-68's slide, M2-73's general solve, M2-80, M2-81) keep their stage-2 edges.
  - (b) R-863's order as written.
- **Recommended:** (a).
- **Measurement:** none; the critical path, counted from `plan/tasks.yaml` under each option.
- **Applied meanwhile:** nothing; the plan's task list shows both orders.
- **Waits:** `plan/tasks.yaml`'s rewrite after the one-way items are answered.

## RQ-770: Does R-926 item 6's "just recompute the chart and re-integrate" keep R-92's cache re-addressing for an in-plane pan or zoom? *(conflict in reading; a one-way door, a reading of the owner's words; R-926 item 6, R-92; the caching contract's invalidation table; context: the scheduler and cache tasks of M5, TASK-M2-61)*

*Queued 10 Nov 2026 by the M2 chart re-plan, from code review 5844040504 of PR #473, finding 1; each quote re-read on
the PR's head. Nothing is chosen here.*

*In plain English: today, sliding or zooming the picture within the same plane reuses the squares already worked out,
only asking for new ones at the edges. The owner's new rule says pan and zoom "just recompute the chart and
re-integrate". Read literally, every pan would throw the cache away; read as the owner's point about there being no
backwards lookup on that path, the cache stays.*

- **File, section:**
  - `decisions.md` § R-926, item 6: "Pan, zoom, tilt, slice and morph just recompute the chart and re-integrate. There
    is no inverse anywhere in that path."
  - `decisions.md` § R-92: "In-plane pan and zoom re-address; slicing out of the plane, tilting and rotating
    re-integrate; lock changes neither."
  - `docs/contracts/principia_caching_contract.md`, the invalidation table: "In-plane pan, zoom, lock | **Nothing.** Pan
    and zoom re-address which quads are asked for on the same slice plane"; and below it: "**in-plane pan and zoom
    re-address; slicing out of the plane, tilting and rotating re-integrate — and neither invalidates.**"
- **The conflict:** item 6 lists pan and zoom with tilt and slice as edits that recompute and re-integrate; R-92 and the
  caching contract keep every quad already integrated on the same plane, computing only the quads a pan or zoom newly
  asks for.
- **The question:** does an in-plane pan or zoom still re-address the cache, new quads alone being evaluated on the CPU
  and integrated?
- **Options seen:**
  - (a) Yes: item 6 says how a changed view is computed (the CPU re-evaluates, the GPU re-integrates, no inverse), and
    a quad already integrated on the same plane is reused, as R-92 and the caching contract say. A tilt, slice, morph
    or rotation is a new plane and re-integrates, as today.
  - (b) No: every pan and zoom re-evaluates and re-integrates every visible quad; R-92's re-addressing and the caching
    contract's row are amended.
- **Recommended:** (a). It is how the plan reads item 6 (`plan/charts/m2-chart-system-replan.md` § A.3, R-92's row),
  and (b) would make a pan cost a whole frame of CPU evaluation and integration (M-A's whole-screen rate).
- **Measurement:** none decides a reading; for the cost, M-A's whole-screen rate against the in-motion rate (plan
  § B.4, item 14).
- **Applied meanwhile:** the plan reads (a) and marks it open; `docs/design/principia_core_design.md` § 3's dated note
  says the point is open.
- **Waits:** the scheduler and cache tasks that follow R-926 (TASK-M5-04 among them).

## RQ-783: A record made before R-926's f64 decode route lands recreates bitwise only through today's f32 decode: R-921 and REQ-TOOL-172 require bitwise, and R-926 item 5 replaces that decode *(conflict; a one-way door, it changes what the instrument claims about reproducing records; R-921, R-926 item 5, R-521, R-919, R-483; REQ-TOOL-172; context: TASK-M7-45, TASK-M2-57, TASK-M2-61, TASK-M4-05, the plan's N1 and N2)*

*Queued 10 Nov 2026 by the M2 chart re-plan, from physics review 5847854223 of PR #473, P1; each quote re-read on the
PR's head 55e6462d. Nothing is chosen here.*

*In plain English: a saved picture stores where each pixel sits on the chart, not the starting system itself, so
re-running it works the starting system out again. The owner ruled that a picture saved before the `β` link changed
must re-run bit for bit, because a chaotic run drifts apart if its starting system's last bits move. The new chart
system works starting systems out a different way, in double precision on the CPU and then rounded, which gives
different last bits from today's way on some pixels. So a picture saved before the switch re-runs bit for bit only if
the old way is kept for it.*

- **File, section:**
  - `decisions.md` § R-921 (a record made under a retired `β` link recreates bitwise), the owner's words: "Recreating
    a record made under a retired β link decodes its stored latent coordinates through that exact link (kept for
    conversion) to the original physical starting state, bitwise, and runs from there"; "No conversion on recreation:
    converting β to the new coordinates "to rounding" can change the starting state's last bits, and a chaotic run
    then won't reproduce."; "Test: a record made before TASK-M2-57 recreates bitwise after it lands."
  - `plan/requirements.yaml` REQ-TOOL-172 (a record refused naming exactly a changed link, and the retired-link
    recreate arm), its statement's last sentence: "A record made before TASK-M2-57 whose β link is one R-919 retired
    as a link (sigmoid_beta, tanh_beta or softsign_beta), every block's hash matching, must recreate by decoding its
    stored latent coordinates through that exact retired link to the original physical starting state, bitwise,
    labelled "made under a retired link", never converting z_β to the periodic link on recreation (R-921)."; its
    verify: "read from fixtures/retired_beta_links/pre_m2_57.json, every bit of which TASK-M2-57 writes through its
    baseline commit's decoder, before any link changes" and "each pixel's starting state decoded through the named
    retired link equal bit for bit to the fixture's decoded starting state, the state the writing build decoded".
  - `plan/tasks/M2/TASK-M2-57.md` (the decoder's mirror step removed, latent `β` periodic), Deliverables:
    "`fixtures/retired_beta_links/pre_m2_57.json`, written through today's decoder".
  - `decisions.md` § R-926 (the owner's eleven chart-system decisions), item 5, the owner's words: "The CPU evaluates
    charts in f64. The GPU only integrates records. No chart code in the integrator."; and *What it supersedes*:
    "every view, live or not, is filled from the CPU's f64 evaluation narrowed to the integrator's records".
  - `decisions.md` § R-521 (a record recreates when every link's own hash matches), the option it decides: "a match
    recreates, even across an unrelated schema change, if nothing else in the record's tier 1 changed meaning."
  - `plan/charts/m2-chart-system-replan.md` § C, "Compared by bound": "The record difference between today's f32
    decode and the f64-then-narrow path is the size of the conversion R-926 makes, which TASK-M2-61's baseline must
    record."
- **The conflict:** records store latent coordinates only (R-921), so recreation runs the decode again. The fixture's
  expected states are today's decode, the f32 decode (§ C's `sample_with::<…, f32>`). Under R-926 item 5 the product
  route becomes the CPU's f64 evaluation narrowed to `Ic<f32>`, which differs from the f32 decode in the last bits
  wherever the f32 decode is not the correctly rounded value of the f64 one (§ C measures how often). TASK-M7-45 (the
  decoder's identity per link), in M7, runs REQ-TOOL-172's retired-link arm after that route has landed (the plan's N1,
  N2 and TASK-M4-05 CHANGE): decoded through the retained retired link but on the new route, a recreated state misses
  the fixture's bits wherever the two routes differ. R-521's "a match recreates" has the same gap for every record
  written before the switch, retired link or not: the switch changes no registry entry, so every link's hash still
  matches and the record recreates, from a starting state whose last bits may differ from the one its writing build
  ran.
- **The question:** does a record written before R-926 item 5's route lands recreate bitwise, and through which
  decode?
- **Options seen:**
  - (a) Keep today's f32 decode as a source for recreation, as R-919 Q1 (c) (the three retired `β` links kept as
    conversion sources) keeps the retired links: a record written before the switch decodes through its writing
    build's route, the f32 decode, bitwise, and runs from there; a record written after it uses the f64-then-narrow
    route. The record must say which route wrote it (a route marker beside the entry hashes, or the build hash and
    schema version it already carries); which is an implementation choice under R-521. Consequences: the f32 decode
    stays product code for recreation (it stays anyway as test and parity entries under RQ-765 (a), the GPU decode
    entries kept for tests; TASK-M2-06 KEEP in the plan); R-926 item 5's "The CPU evaluates charts in f64" gains a
    named exception for recreating old records; R-921, REQ-TOOL-172 and TASK-M7-45 stand as written, the arm run
    through the retained route.
  - (b) Give up bitwise recreation of records written before the switch:
    - (b1) recreate through the new route within a stated tolerance of the writing build's state, in f32 ulps per
      component, a calibration (R-71, a missing value becomes a calibration requirement), labelled "recreated under a
      changed decode"; or
    - (b2) refuse, "state present, cannot recreate", naming the changed decode, as R-521 refuses a changed link.
    Consequences: R-921's "bitwise" and REQ-TOOL-172's last sentence are amended; the owner's reason in R-921, a
    chaotic run reproducing only from the same bits, holds only for records written after the switch; (b1) recreates
    runs that may end differently, labelled, and (b2) loses those records' recreation altogether. One decode route
    remains.
- **Recommended:** (a). It keeps the owner's own reason in R-921, that a chaotic run reproduces only from the same
  bits, at the cost of keeping code that R-919 Q1 (c) and RQ-765 (a) already keep for other reasons; (b) trades every
  pre-switch record's reproducibility for one fewer route.
- **Measurement:** the measurable alternative, the plan's first slice (§ C, N1): per chart, the share of samples where
  the f64-then-narrow record differs from today's f32 decode, and, run on the fixture's pixels once TASK-M2-57 has
  written it, how many of the fixture's starting states would miss their bits; its control arm gives the outcome flips
  that difference causes against the one-ulp chaos baseline. Criterion, written before the run: if no record differs,
  on the fixture's pixels and on N1's grids, the two routes agree on those inputs and the choice bears only on inputs
  outside them; any differing record shows the conflict is real, and the question stands as written.
- **Applied meanwhile:** nothing. R-921, REQ-TOOL-172, TASK-M7-45 and TASK-M2-57's fixture stand as written; the plan
  lists the item in § A (REQ-TOOL-172 and TASK-M7-45, "waits on RQ-783") and § C, and recommends (a).
- **Waits:** TASK-M7-45's retired-link arm; N2's record contract (whether a record carries its decode route);
  TASK-M4-05's switch to records.
