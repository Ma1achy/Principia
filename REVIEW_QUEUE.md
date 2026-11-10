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

## RQ-755: A record written before R-919 names `sigmoid_beta`, `tanh_beta` or `softsign_beta` as its `β` link and every entry hash matches: does it recreate through the conversion source, or is it refused? *(silence; a one-way door; R-919 Q1 (c) and Q3 (a), R-898, R-483, R-516, R-521; REQ-TOOL-172, REQ-TOOL-173, REQ-GEN-013; context: TASK-M7-45, TASK-M2-57)*

*Queued 9 Nov 2026 by the chart-core rulings PR (#465), from its physics review 5834538186, item 4, and the
implementer's report on it; each quote below re-read on the PR's head. Nothing is chosen, and nothing in #465 decides
it.*

*In plain English: the owner ruled that the triangle's shape angle β gets one round-the-circle link, and that the three
old half-circle links stay only to convert old saved charts (R-919). Old exported records (the data embedded in an
image or a share link) also name one of those three links, usually the default `sigmoid_beta`. When such a record is
opened and its link's fingerprint still matches, nothing says whether it is converted to the new link and opened, as an
old saved chart is, or refused as too old.*

- **File, section:**
  - `decisions.md` § R-919, Q1 (c): "`sigmoid_beta`, `tanh_beta` and `softsign_beta`, written onto the bounded
    `β ∈ [0, π]`, are not that link: TASK-M2-57 rewrites dd_generation_root § 3.9's `β` rows and REQ-GEN-013's `β`
    codomain to the one periodic link … The three stay as conversion sources only, their forward maps kept for Q3 (a)
    and its control "a value converted by the wrong link fails": not selectable, not counted as `β` links".
  - `decisions.md` § R-919, Q3 (a): "an old saved chart converts from the `β` link it pins where it pins one (R-516),
    else from `sigmoid_beta`, the default, with a notice on load naming the assumption." It names saved charts, not
    records.
  - `decisions.md` § R-898's heading: "every stored latent view, saved chart and golden is converted once … the
    saved-file format is versioned, old files converted on load". It names no record.
  - `decisions.md` § R-516's heading: "A chart preset or a saved chart may optionally pin the links it was designed
    under; loading one whose pin differs from the active links applies the pin with the chart, one command and one
    undo entry, with a notice naming the blocks changed".
  - `decisions.md` § R-483's heading: "a record's decoder is the build's registry plus its links, an unknown or changed
    link refused and never substituted; an old record reads under its own build's defaults".
  - `decisions.md` § R-521's heading: "a mismatch refuses, naming exactly that link, and a match recreates across an
    unrelated schema change".
  - `docs/design/principia_dd_image_embedding.md` § "6. What travels", "The decoder's identity is the build plus the
    config (R-483, X4)": "**An unknown link id**, one the reading build does not have, is "state present, cannot
    recreate""; "when all four match, a schema version other than the reader's refuses nothing on the links' account";
    "**Never substitute.** No default link stands in for one the record names".
  - `plan/requirements.yaml`, REQ-TOOL-172: "a record whose block's entry hash differs from the reading build's entry of
    that id must be refused as "state present, cannot recreate", naming exactly that link; when every block's hash
    matches, a schema version other than the reader's must not refuse the record on the links' account". Its note (9
    Nov 2026, this PR) reads the verify's all-tanh arm with `tanh_beta` as an old record's `β` link checked against
    the conversion source's entry hash: the refusal side only.
- **The silence:** after TASK-M2-57 the reading build still holds an entry for each of the three, with its hash, so a
  pre-R-919 record naming one is neither an unknown link nor, when the hashes match, a changed one. R-521 then says it
  recreates; R-919 says the link isn't selectable, and the record's `z_β` is in the old half-circle coordinates, so it
  can't be decoded as it stands. R-919 Q3 (a) and R-898 convert saved charts and stored views on load, and name no
  record. This touches every record written before TASK-M2-57 merges, since each names some `β` link, by default
  `sigmoid_beta`.
- **The question:** when a record written before R-919 names one of the three conversion sources as its `β` link and
  every entry hash matches, does the reader recreate it, and how?
- **Options seen:**
  - (a) Recreate through the conversion source, with a notice: `z_β` converted from the named link to the periodic link
    once on read (`z_β,new = link_new⁻¹(link_old(z_β,old))`, TASK-M2-57's conversion), the decoded system the same
    within REQ-ENC-024's `ε_phys`, the result labelled converted, as R-919 Q3 (a) treats a saved chart. The record names
    its link, so nothing is assumed or substituted.
  - (b) Refuse as a pre-R-919 record: "state present, cannot recreate", naming the `β` link as retired as a link, the
    conversion sources serving saved charts only.
  - (c) Another, stated by the owner.
- **Recommended:** (a). The record names its link exactly, so the conversion is the one R-919 Q3 (a) already applies
  to a pinned saved chart, with no assumption, and refusing would make every existing shared image unopenable over a
  change R-898 meant to be converted. The measurable alternative is (b), whose cost is every record written before
  TASK-M2-57 merges; the check for (a) is the conversion test (`latent_conversion`'s criterion) run on a recorded
  fixture per conversion source.
- **Applied meanwhile:** nothing. REQ-TOOL-172's verify is unchanged, and its note reads only the refusal side.
- **Waits:** TASK-M7-45, which builds the per-link refuse-or-recreate path on read (REQ-TOOL-172), on its recreate
  arm for a pre-R-919 record. TASK-M2-57 does not wait: it keeps the three forward maps either way.

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
