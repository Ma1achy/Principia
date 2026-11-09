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

## RQ-679: The slider span of `ν₀`, the held `L_z` and the held `K`, which R-507 U1 divides a hidden offset by, while § G2's Slice centre lists only the eight latent sliders *(a definition requirement, R-72, and a value, R-71; R-507 U1; R-633; render_gui_spec § G2; context: TASK-M8-48, TASK-M2-52)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flights. Nothing is chosen.*

- **File, section:**
  - `decisions.md`:16508-16510, § "R-507 — …", U1: "`d̃` the hidden coordinate's offset divided by that coordinate's
    slider span, the Slice centre slider's range (§G2). After the rotation `ã`, `b̃` are multiplied back by their full
    ranges and `d̃` by its span."
  - `docs/gui/principia_render_gui_spec.md`:317, § "G2. Explore — the everyday view", Slice centre: "eight sliders,
    `z_α, z_β, z_q0…z_q3, z_μ1, z_μ2`, each with its number box beside it."
  - `decisions.md`:27434-27437, § "R-633 — …": "coordinate is ν₀ (the held Burrau-family shape)" and "the (ν, K) chart and
    Φ_θ,K gain the held L_z, and Φ_θ,L_z gains the held K, as physical hidden coordinates".
  - `plan/tasks/M2/TASK-M2-52.md`:30: "the held `L_z` of `(ν, K)` and `Φ_θ,K`; the held `K` of `Φ_θ,L_z`. Each is offered
    as a tilt target and a Slice centre slider, a physical value shown as itself".
- **The gap:** `ν₀`, the held `L_z` and the held `K` are physical hidden coordinates with no latent slider, so § G2 gives
  them no slider and no range, and U1's `d̃` has no span to divide by.
- **Options seen:** (a) each span is the coordinate's feasible range at the view's centre, a definition (R-72) written
  into § G2. Open within (a): the held `K`'s feasible range at the centre is `[L_z²/2I, ∞)`, with no upper end, so (a)
  needs a cap, for example (a1) the invariant warp's `K_max` (REQ-CHART-044) or (a2) `Φ_θ,L_z`'s calibrated `K`
  (REQ-CHART-046); and `ν₀`'s range is either `(0, 1)` or `docs/design/principia_chart_reference.md` § 4.5's
  `[ν_min, ν_max]`, not yet chosen; (b) a fixed span per coordinate, a calibration (R-71); (c) another, stated by the
  owner.
- **Applied meanwhile:** nothing.
- **Waits:** only the span a view supplies, through TASK-M8-48's sliders. TASK-M2-52's kernel work does not wait: the
  span is the caller's input to `Tilt::new(spec, centre, span)` (`crates/kernel/src/chart/tilt.rs`:509-530), used only
  as `d = span·d̃` (`Tilt::offsets`, :555-564), and no TASK-M2-52 acceptance arm depends on its value.
- **4 Nov 2026 (R-706):** deferred to the start of TASK-M8-48 by owner 4 Nov 2026. Options with a recommendation are proposed then. The entry stays open.
- **5 Nov 2026 (R-720; physics review 5801728548 of #425, P2):** R-507 U1 divides `d` by "that coordinate's slider span, the Slice centre slider's range". R-708 puts a held `K`'s tilt `d` in units of `t`, and a slice of a held `K` stays by value, so a by-value slider has its span in `K`. The same slider then has two units on `Φ_θ,L_z`. The span's definition has to say which, and how they convert.
- **Deferred:** deferred to the start of TASK-M8-48, options with a recommendation proposed then by owner 5 Nov 2026 (R-807, the owner: "RQ-679: Defer to TASK-M8-48. As above.").
- **Deferred:** deferred to M8, the start of TASK-M8-48 (R-822) by owner 7 Nov 2026 (R-822, the owner: "RQ-629: (b) at M4; (c) ruled with RQ-687 on the cross-backend numbers. defer:M4
  RQ-679: defer:M8
  RQ-687: defer:M4
  RQ-696: (c).").

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

## RQ-701: Whether and how a `MixedEnergy` chart's held `L_z` or `K` tilts and slices: a hidden coordinate under R-633 that nothing builds or queues *(physics; silence: R-633; the owner's ruling of 4 Nov 2026 on RQ-678 and RQ-683; chart_decoder_contract § "Tilt (basis edit)"; TASK-M2-52's six charts; context: TASK-M2-37)*

*Queued 5 Nov 2026 by TASK-M2-37's fix round, on the orchestrator's decision (applied per R-369), from code review
5778057939 (F1) and physics review 5778075140 (F4) of PR #387; rewritten from code review 5779819090 (F2) and physics
review 5779878848 (F1), which found the first entry short of the contract's clause and of the ruling's lines. For the
owner: yes, it is physics; the tilt changes which systems the chart shows.*

- **File, section:**
  - `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)", "Charts that hold a block by a
    construction": "where it holds a block by a physical construction, its hidden coordinates are that construction's
    physical parameters, tilted and sliced by value as the shape sphere's `p_λ` targets are, `d` added to the physical
    value at the slice centre". `MixedEnergy` holds the momentum block by chart_reference §2.2's construction, its
    `held` the parameter.
  - The owner's ruling of 4 Nov 2026 on RQ-678, the warped mode: "Held energy-type hidden coordinates (R-633 c2) are
    tilted in their warp coordinate in this mode, so held K stays ≥ 0. This amends R-633's "by value" for warped mode
    only."
  - The same ruling, the linear mode: "Rows keep fixed physical values; infeasible cells are "not a system", blank per
    R-470; held coordinates tilt by value."
  - The same ruling, on RQ-683: "RQ-683 (#387): same rule for a single energy axis at held spin."
  - The same ruling, its last line: "If the warped-coordinate tilt of held K turns out not to be well-defined
    somewhere, come back with the case."
  - `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps", the hidden coordinates' table,
    which names only: "`(ν, K)`, `Φ_θ,K` (and `Φ_strip,K`, the same map) | the held `L_z` (0 untilted), a physical
    value" and "`Φ_θ,L_z` | the held `K`, a physical value".
  - `plan/tasks/M2/TASK-M2-52.md` § "Deliverables": "The six charts' hidden coordinates … the held `L_z` of `(ν, K)`
    and `Φ_θ,K`; the held `K` of `Φ_θ,L_z`."
  - `crates/kernel/src/chart/physical.rs`, `MixedEnergy`'s `top`: "`K_max` on `K`, `KE` and `E`, `Q_hi`, the top of the
    asked range, on the virial ratio; not read on `L_z`."
- **The silence:** the principle makes a lone-axis chart's held `L_z`, or the held `K` on an `L_z` axis, a hidden
  coordinate, but the table and TASK-M2-52's six charts do not include `MixedEnergy`. So its tilt and its slice are
  neither built nor queued, and `Hidden::hidden` on the chart lists only the frozen controls. The ruling reaches a
  lone axis through its RQ-683 line. In the linear mode that settles the form: the held value tilts by value, and the
  contract's clause slices it by value too; only the build is missing. In the warped mode the ruling amends "by value"
  for the tilt and does not say what becomes of the slice. Its warp coordinate is not well-defined on these charts as
  they stand, which is the case its last line asks to have brought back:
  - **Held `L_z`, on a `K`, `KE`, `E` or virial axis.** `L_z`'s warp coordinate, `(2s − 1)·L_max(K)`, reads a held `K`,
    and these charts hold none: `K` is what the pixel's axis sets. That rules out that one coordinate, not a tilt: the
    additive floor takes any real `L_z`. The ruling's reason, "so held K stays ≥ 0", does not speak of `L_z`.
  - **Held `K`, on an `L_z` axis.** The warp coordinate, `K = K_max·t^γ_K`, `t₀ = (K₀/K_max)^(1/γ_K)`, is well-formed,
    and is the one the ruling's reason names. It needs a `K_max` this chart does not read: a missing value (R-71), not
    a missing form. `top` is the field that would hold it.

  These two concern different axes, so no one form has to meet both.
- **The question:** in the warped mode, how does a tilt move the held `L_z` on a lone `K`, `KE`, `E` or virial axis,
  and the held `K` on a lone `L_z` axis; and does a slice of the held value move the same way, or by value? In the
  linear mode the form is the ruling's; only the build is open, and it waits on the warped answer so that both arms
  come in together.
- **Options seen, held `L_z`:**
  - (a) By value on the floor: `L_z = L_z₀ + d`, §2.1's floor at the pixel's own tilted `L_z`. Every pixel stays a
    system and no `L_max` is needed. Cost: the ruling's warp sentence then reaches held `K` only, and `|L_z|` has no
    bound but the slider's span.
  - (b) A warp coordinate on a fixed scale, `L_z = (2s − 1)·√(2·I·K_max)`, `s` bounded. Cost: a scale §2.1 does not
    write for these charts, which `K`, `KE` and `E` can read through `K_max` and the virial axis cannot, its `top`
    being `Q_hi`; and `s` needs a rule past its edges.
  - (c) Another, stated by the owner.

  The ruling's port queues the same question and options (a) to (c) for `(ν, K)`, `Φ_θ,K` and `Φ_strip,K`; the answer
  there can be this one.
- **Options seen, held `K`:**
  - (a) The warp line, `K = K_max·t^γ_K`, with `K_max` supplied to the chart. Cost: a value this chart does not hold,
    a calibration requirement under R-71 with REQ-CHART-044's placeholder meanwhile.
  - (b) Another, stated by the owner.
- **Slicing, in every option above:** by value, the contract's clause as it stands, or in the tilt's own coordinate.
  The two agree at an offset of 0 and write different held values away from it.
- **Applied meanwhile:** the held value is neither tilted nor sliced: the chart's hidden coordinates stay its frozen
  controls, and chart_reference §2.1 says so (PR #387). `d = 0` reproduces the untilted chart in every option.
- **Waits:** a tilt or a slice of the held value on a lone-axis chart; nothing else.
- **5 Nov 2026 (R-720, RQ-699 (a)):** the owner has since ruled the same question on `(ν, K)`, `Φ_θ,K` and `Φ_strip,K`: "L_z = L_z0 + d", "R-708's "tilt in the warp coordinate" was meant for held K (which must stay >= 0) and is narrowed to held K only", and "The same rule governs slicing the held L_z as well as tilting it." R-720 names those charts, so it doesn't close this entry, but what it implies here is held `L_z` option (a), tilted and sliced by value. The cost option (a) lists, the warp sentence reaching held `K` only, is now the owner's own reading. Held `K`'s `K_max` on a lone `L_z` axis is still open; its slice is by value (R-720's list).
- **Deferred:** deferred to M8, the start of TASK-M8-48, its form ruled by R-818 (option (a) for each, slicing by value, K_max as for RQ-710) by owner 7 Nov 2026 (R-818, the owner: "RQ-701: (a) for each, slicing by value; K_max ruled with RQ-710. defer:M8
  RQ-708: (a).").

## RQ-711: What a saved preset's base point, centre and window are when its chart is not two latent coordinate axes at one scale *(physics; silence; REQ-GUI-108, REQ-GUI-224, REQ-GUI-417, R-466, R-475, R-798; `docs/gui/principia_render_gui_spec.md` § G2, § G7; context: TASK-M8-18)*

*Queued 6 Nov 2026 from physics review 5805243152 of PR #429 (TASK-M6-45), finding F1. Not chosen here. The definition now stops where it is true, and the mock's six presets are all inside it. This entry covers saved presets; R-798's shipped preset is outside it.*

**In plain English:** every chart preset has a home view, built from a starting point, a centre and a zoom scale. Those definitions work when the preset's two axes are plain latent coordinates at the same scale, which all six mock presets are. A user can also save other charts: skewed axes, axes at different scales, axes that are physical quantities, curved charts, or a tilted view. For those, the formula gives no home view, or gives one that differs from the chart that was saved, so Home would change the chart. The saved-presets task needs an answer before it can store these values.

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` § G2 "Resets and Home": "The preset's **home view** is `z₀ = b`, `q₁ = σ_h e_a`, `q₂ = σ_h e_b`."
  - `docs/gui/principia_render_gui_spec.md` § G2, Save view as chart, as R-475 has it, which commits the view as a named chart, storing "a latent view as the oblique chart `(z₀, q₁, q₂)`" and a curved chart as "its parent chart, slice centre and tilt".
  - `docs/gui/principia_render_gui_spec.md` § G7: "**Each axis has a kind:** latent direction (any mix of the eight `z` components, normalised; a normalise button shows `|q|`), physical quantity (…), or Burrau dimension (…)".
  - `plan/requirements.yaml` REQ-GUI-108: "a saved preset must store, with its axes, its home values: its base point, its centre and its window"; its verify: "the section resets and Home of the saved preset read back exactly".
- **The silence, case by case:**
  - Unequal scales, `|q₁| ≠ |q₂|`: one `σ_h` cannot express the chart, and Home would change it.
  - Non-orthogonal axes: `u = b·e_a`, `v = b·e_b` are not the base point's coordinates on the axes, and they disagree with Navigate's own row, which inverts the Gram matrix.
  - A physical-quantity or Burrau axis, or a curved chart: there is no `e_a`, so neither the centre nor the window is defined. R-798's equilateral-point shape-chart preset (REQ-GUI-417) is a curved chart too, but a shipped preset, not a saved one: its map and window are TASK-M9-45's, and it does not wait on this entry.
  - A tilted view: is the saved pair the new axes, or the parent's? § G2's three sections and R-466 K2's "six dimensions off them" depend on the answer.
- **The question:** for each of these charts, what are the home view, the centre and the window, and what does the zoom row read against?
- **Options seen:** (a) a preset stores its whole saved triple `(z₀, q₁, q₂)` as its home view, with the zoom row reading against a scale still to be named; (b) only charts within § G2's formulae store home values, and the resets and Home are dimmed on any other, as with no active preset (R-466 K3); (c) a definition per axis kind.
- **Applied meanwhile:** § G2 and § G7 bound the formulae to a preset whose axes are two latent coordinate axes at one scale. All six mock presets qualify, so no code changes.
- **Waits:** REQ-GUI-108's home values for any other chart, in TASK-M8-18; the home values of region D's preset, in TASK-M4-34, and of the paper-frame presets, in TASK-M8-112.
- **7 Nov 2026 (physics review 5810950891 of PR #422, F1):** this entry also covers the shipped design presets, region D's (TASK-M4-34, REQ-CHART-100) and the paper-frame ones (TASK-M8-112, REQ-CHART-122), which sit on non-latent axes; REQ-GUI-224 asks each chart preset's home values. They are proposed, not built, and they aren't R-798's exception. Only their home values wait on this entry.
- **7 Nov 2026 (R-820):** the owner: "RQ-711: (a)." Option (a) is applied: a saved preset stores its whole saved triple `(z₀, q₁, q₂)` as its home view (`principia_render_gui_spec.md` § G2, REQ-GUI-108). **Left open:** the option's own "the zoom row's scale still to be named". The entry stays open for that one value.

## RQ-714: Region D's definition as a chart *(a one-way door, order item 7; physics: changes what is drawn and counted; R-592; dd_validation_orbits §1.6; context: TASK-M4-06, REQ-RENDER-082, M9's targets)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: region D is the small curved triangle of starting shapes twenty M9 targets use. Its definition as a set of points is settled; what it is as a picture is not: what happens to the pixels outside it, which velocities it starts from, and how a paper's fixed time limit reads when each pixel has been rescaled differently.*

- **Reach (physics note 2, P11):** D is a fundamental domain only at equal masses and `L_z = 0`. At unequal masses the outside is other systems (RQ-728), and under a held `L* ≠ 0` the mirror is lost, so "blank and uncounted" as agreed in principle reaches that case only.
- **File, section:**
  - `docs/design/principia_dd_validation_orbits.md` § "1.6": "**Region D** (I, Fig. 1 and eq. 8): `ξ ≥ 0`, `η ≥ 0`, `(ξ + ½)² + η² ≤ 1`. A point outside it is not a region-D start, and the import refuses it."
  - `docs/contracts/principia_lowering_contract.md`, Appendix, the Anosova row: "physical (m,r,p) construction → **canonicalise entry** → standard pipeline | region D bounds".
  - `docs/contracts/principia_chart_decoder_contract.md` Part 3: "**Inside `[0,1]²` every gallery chart is valid by construction** … so a pixel hatched there is a decoder bug". And `decisions.md` § R-506: "warped to its feasible range by default".
  - `decisions.md` § R-554: "inside `[0,1]²` such a pixel is the domain's end by design … it draws blank and logs no decoder-bug warning", the exponential map's exception.
  - `docs/design/principia_dd_replication.md` § 5.8: "AA-3 | … the Anosova physical-frame entry" with no velocity stated; "AA-2 | … eq. (3) velocities".
- **The silences, four:**
  1. D is not a rectangle, so a square chart either masks, shows the outside, or warps. The outside pixels are real systems: relabelled copies at equal masses and `L_z = 0`, other systems otherwise (the Reach line above).
  2. A mask makes pixels inside the square that are not drawn, which the contract calls a decoder bug on a gallery chart.
  3. AA-3 and AA-5 don't say which velocities.
  4. The kernel decodes at `I = 1`, so each pixel has its own rescale `ℓ`, 0.408 to 0.577 over D at equal masses, and one sim-key `T` is a paper time varying by a factor of 1.68 across the chart there; other masses differ, and `T_c`'s `ℓ_min` is taken at the run's masses.
- **Overtaken for the presets (RQ-736):** the owner's filter order, 7 Nov 2026: "region D presets render the full disc, with the filter on by default for the paper-frame presets only." So (b) with a display filter, and (a) stays the record of what was agreed in principle. The conflict with (a) is listed there.
- **Options on (1), (2):**
  - (a) **Recommended.** A mask. Axes `ξ ∈ [0, ½]`, `η ∈ [0, √3/2]`, linear, a preset of the general third-body pair; outside eq. 8 is the domain's end, blank, out of the count and the total, no warning, R-554's exception extended by name. Pixels stay uniform in `(ξ, η)`, as the papers sample, and the picture is theirs. R-506's warp reads as trivial here: every `(ξ, η)` is a system, so the range is its whole range.
  - (b) The plane, unmasked. D drawn as a contour; statistics filter. No exception needed; the Atlas shows relabelled copies at equal masses and `L_z = 0`, other systems otherwise.
  - (c) A warp onto the square, polar about the left body. Every pixel in D, no exception, R-506 read literally; the measure is not the papers' and the apex is a ray-degenerate edge.
- **Options on (3):** (a) **recommended**, AA-3 and AA-5 read paper I's own runs, so eq. 3; (b) exact rest.
- **Options on (4):** (a) **recommended**, the run reaches the longest pixel's horizon and the comparison cuts each pixel at its own, a tested function (rule 11), the kernel untouched; (b) a per-pixel `T`, a kernel change.
- **Applied meanwhile:** nothing built. The design's § 8 writes (a) in each case.
- **Owner, 6 Nov:** outside region D blank and uncounted is agreed in principle. That is not a ruling; this entry stays open.
- **Waits:** the design's tasks behind TASK-M4-37, and no existing task. TASK-M4-06's golden (REQ-RENDER-082) needs this domain, so its edge to TASK-M4-34 is applied when this is ruled.
- **7 Nov 2026 (R-845, R-851):** the owner: "RQ-714: outside masked (a); time (a). Velocities: offer what each region-D target uses, as presets,
  with the default chosen by which most targets use; report the count before fixing it." Applied: time (a); velocities, exact rest the default and eq. 3 the second preset, on the count (R-851, the owner's later order: "Region D default: exact rest, eq. 3 as the second preset, as counted."). **Left open, the outside:** the reply's "outside masked (a)" (12:16) and the owner's later filter order (13:40, R-855), "region D presets render the full disc, with the filter on by default for the paper-frame presets only", differ in what is built, a mask in M4 or none, and nothing says which governs, nor what "full disc" is (RQ-738, from RQ-736's conflict 3). So neither is applied; TASK-M4-06's edge to TASK-M4-34 waits with this, and RQ-728's "labelled mask" is read with it.

## RQ-716: The axis cap, proposed at 4 *(a one-way door, order items 5 and 7; REQ-PERF-006, REQ-PERF-007; lowering Part 4; the start-up object count is issue #427, not this item)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: every "axis pair" compiled into the GPU code makes the one compiled module bigger, and every pipeline parses the whole module. The owner asked for a limit on those pairs, suggesting about 4, with the measured cost. It measures at 4 passing and 8 failing on the Mac and on the Windows machine, which agrees. The limit is a one-way door because plans and charts will lean on it. One new question: start-up on the Windows machine already takes 2.7 s with no pairs at all, over the 2 s this design set, so the owner has to say whether that limit was only ever the Mac's. Whether twelve start-up objects against the contract's ten is right is a separate issue, #427.*

- **File, section:**
  - the owner's order of 6 Nov 2026, item 5: "Cap the number of axis pairs baked into GPU code (propose a number with the measured cost; the owner suggested about 4)."; item 7: "One-way doors (new extension types, any physics or default change, the axis cap, R-798's reading, region D's definition) go to /rulings-report".
  - `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule": "**Chart switching must be instant** → the compute variant is already built (SPIR-V); create its pipeline object for the *active* (occupant, tier, links) at startup — ~10 objects."
  - REQ-PERF-006: "pipeline objects are created from pre-built SPIR-V, eagerly for the active (occupant, tier, links) of every chart at startup and lazily inside the re-integration cost for occupant/tier/link/threshold changes."
  - REQ-PERF-007: "The build-time monomorphisation set must be bounded to the active/plausible variant set, not the full cross-product."
  - `docs/design/principia_chart_composition.md` § 11, the criterion and the measurement.
- **The measurement (criterion written first):** first run, a heavy stand-in: 4 passes, 8 fails. Re-measure, the real region-D pair, interleaved: 4 pairs: module ratio 1.24, one creation 1.23, eager set 1.59 s, gallery 1.23, all within limits; 8 pairs: module 1.48, gallery 1.47, eager 2.35 s, all fail. A pair costs 60 KB of WGSL, so a fifth fails the module test at 1.30: 4 is the ceiling. With the integrator inlined, one pair across 4 occupants fails at 1.43, so the cap holds only with decode as its own entry. That run: one Mac, Metal only, under load.
- **The Windows Vulkan run (criterion unchanged; RTX 2080 SUPER, run 42270588241, 6543782c, 12 arms; byte-identical modules):**

  | `P` | gallery ratio, Mac / Windows | build ratio, Windows | eager `12 + P`, Mac / Windows | start-up (12 gallery), Mac / Windows |
  |---|---|---|---|---|
  | 0 | | | 0.96 s / 2.70 s | 0.96 s / 2.73 s |
  | 1 | 1.06 / 1.05 | 1.06 | 1.10 s / 3.05 s | 1.02 s / 2.87 s |
  | 4 | 1.23 / 1.24 | 1.24 | 1.59 s / 4.40 s | 1.18 s / 3.39 s |
  | 8 | 1.47 / 1.49 | 1.49 | 2.35 s / 6.74 s | 1.41 s / 4.08 s |

  Module ratios are the same exact 1.06, 1.24, 1.48. On every test that moves with `P` the machines agree: 4 passes, 8 fails, so the lower-number rule picks nothing. C1, C2 and C3′ all sit at the limit at 4; C3′'s first repeat alone reads 1.251. **C3's 2 s fails on Windows at every `P`, `P = 0` included.** C3 as written says "on this Mac"; the re-measure also says a criterion isn't relaxed for either machine. On the first reading 4 passes; on the second no count passes and the criterion gives no number. Neither gives a number other than 4.
- **Cautions on that run:** a measurement only, gating nothing (R-649). The owner reports the card may have faulty cores, so the times are one possibly faulty card's; sizes don't depend on it. The first arm ran cold, so compare eager minus gallery within an arm: per pair 176, 246 and 330 ms at `P` = 1, 4, 8, near one gallery pipeline's 239, 282 and 340 ms, and about nothing at `P = 0`. No cache is hiding a pair's cost.
- **The link to #427:** pairs made eager fail the 2 s test, so they are created on first use, which reads REQ-PERF-006's "every chart at startup" as not reaching a composed chart. #427 owns the eager set; if it changes, this is re-read.
- **The questions:** what is the cap, and does it count entries or pairs? Are pair entries created on first use? Does C3's 2 s apply on Vulkan?
- **Options:**
  - (a) **Recommended.** 4 baked entries beyond the gallery, counted as entries (a pair under two link sets is two), decode-only, created on first use; the first use of a pair may take a compile, about 0.25 s to 1 s measured under load on the Mac and about 0.37 s on Windows, and that is accepted as an exception to "every chart at startup". Past the cap a pair runs as CPU batch, labelled. `SuvakovPlane` counts as one of the 4.
  - (b) 3 plus a generic blend entry, if RQ-721 adopts live paths: the blend takes the fourth slot of the module budget. Same module size as (a), a narrower reading of "pairs".
  - (c) 4 eager: every pair at startup, REQ-PERF-006 untouched; start-up on the Mac rises from about 1 s to about 1.6 s and on Windows from 2.7 s to 4.4 s, and #427's gap widens.
  - (d) A cap on entries by a module-size budget (1.25 × today's WGSL) rather than a count: more honest to the cost, harder to read in a task.
- **On the 2 s, recommended:** read it as written, the Mac's number, keep it unchanged, and let #427 set any Vulkan start-up budget, since the gallery fails it with no pairs and the cap can't cure that. This reading is the one under which the proposal passes, so it's flagged as such. If the 2 s applies on Windows, no cap passes, and the gallery itself would have to shrink or go lazy.
- **Applied meanwhile:** (a), with the value in one ledger constant, so a different ruling changes one number and the test `baked_cap`.
- **7 Nov 2026 (R-837):** the owner: "RQ-716: (a)." Option (a) is applied: 4 baked entries beyond the gallery, counted as entries, decode-only, created on first use, CPU batch past the cap. **Left open:** whether C3's 2 s start-up limit applies on Vulkan, where the gallery alone takes 2.7 s; the answer doesn't say, and the cap passes only on the reading that the 2 s is the Mac's, which stands meanwhile, #427 owning any Vulkan budget.

## RQ-729: Paper frames the library doesn't settle *(a silence; the owner's latest order part E; RQ-650, open; context: TASK-M8-112)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: most targets' coordinate frames can now be written as presets. A few details aren't in anything we hold. Four the order listed turned out to be readable after all. The rest should wait unbuilt rather than be guessed.*

- **File, section:** `docs/design/principia_chart_composition.md` § 20, its gaps table, each with its path; the order's list: "Tanikawa-Mikkola's fixed-k velocity prescription and D12; Hristov 2025's conventions; Valtonen 2005's rotation parameter w; Martynova 2009; Boyd & McMillan 1993; Hut 1983; Heggie & Sweatman 1991's full setup; Henon 1974's A; the L/M/A/H boundary definitions."
- **The finding for RQ-650:** Tanikawa & Mikkola 2008 prints `D11 = {x ≥ 0, y ≥ 0, (x − 0.5)² + y² ≤ 1}`; its Fig. 3 draws `D₁₁` as Anosova's region and `D₁₂` beside it inside the right circle. So the printed set is exactly `D₁₁ ∪ D₁₂`: neither a mirror nor plainly a typo, a reading RQ-650's record doesn't have. It doesn't rule RQ-650.
- **Options:**
  - (a) **Recommended.** Presets where read, tested against the paper. Each remaining gap stays unbuilt, and a target that needs it reads "not scored" with the gap named; no frame is guessed.
  - (b) Fetch or read the held copies first, then write all the presets at once.
- **Applied meanwhile:** (a), unbuilt.

## RQ-731: REQ-VAL-326's pointer to the 2015 paper *(a corpus error; `plan/requirements.yaml`, REQ-VAL-326; context: TS-2, TASK-M9-28)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the requirement behind target TS-2 says the fixed-k prescription is read from the 2015 paper. The 2015 paper is free-fall and doesn't have it; the 2008 paper does.*

- **File, section:** REQ-VAL-326's prescription "read from the 2015 PASJ paper"; `docs/design/principia_dd_replication.md` TS-2: "Tanikawa & Mikkola 2008, Fig. 7"; physics note 2 of 6 Nov, RQ-725. Also: TS-2's "virial ratio 0 to 0.7" is `k = T/|U|`, not `Q`; `Q = 2k`.
- **Options:**
  - (a) **Recommended.** Change the pointer to Tanikawa & Mikkola 2008 § 3.1 and § 4.2, and note `k` against `Q`. TS-2 stays "not scored" (R-669).
  - (b) Leave it, since TS-2 isn't scored.
- **Applied meanwhile:** nothing.

## RQ-732: Older acceptance and verify lines that give `cargo test` two filters: 2 in the invalid form, 5 more in prose, not the 43 a grep finds *(a record correction, a two-way door under R-812; Codex's P2 on PR #422, thread PRRT_kwDOUpUQ5c60yEWq; context: TASK-M6-32, TASK-M6-33, REQ-CHART-070, REQ-PAY-097, REQ-PAY-098, REQ-GUI-178)*

*Queued 7 Nov 2026 from PR #422, for the orchestrator to decide: the correction is reversible, so R-812 doesn't send it to the owner. PR #422 fixed its own 12 lines to `cargo test -p engine -- <a> <b>`.*

**In plain English:** `cargo test` takes one test-name filter before `--`, so a command naming two fails before any test runs. A few older lines in the plan name two. The grep for the form finds 43 lines on `main`, but most of those are sentences that carry on after one test name; the lines to fix are 7.

- **File, section:**
  - The form, as `cargo test --help` gives it and Codex's P2 quotes it: "`cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]`"; "invoking the first command produces `unexpected argument 'path_zoom_bitwise' found`".
  - The count: `grep -rnE 'cargo test -p [a-z_-]+ [a-z0-9_]+ [a-z0-9_]+' plan --include='*.md' --include='*.yaml'` gives 43 lines on `main` at c4dd9d98, and 45 with PR #422's two new lines, both of them the "two commands" kind below. The 43 are:
    - 2 in the invalid form, both already-merged tasks' acceptance lines: `plan/tasks/M6/TASK-M6-32.md`:59, "`cargo test -p gui mock_keyboard scope_tab_order_mock`", and `plan/tasks/M6/TASK-M6-33.md`:61, "`cargo test -p gui mock_compass_modes mock_compass_keys`".
    - 5 naming two filters in prose, which fail the same way if run as written: `plan/requirements.yaml`:3223 (REQ-CHART-070), "cargo test -p kernel pe_axis and physical_axis_warps"; `plan/requirements.yaml`:8909 (REQ-PAY-097) and :8929 (REQ-PAY-098), "cargo test -p ledger declared_ranges and numeric_view_template pass with it"; `plan/requirements.yaml`:21351 (REQ-GUI-178), "cargo test -p gui mock_keyboard and scope_tab_order_mock"; and `plan/reviewers/gui.md`:129, its generated copy.
    - 29 of two separate commands, already valid, as "cargo test -p gui f3_fill_modes and cargo xtask screenshot 01_main".
    - 7 of one filter and then a sentence, already valid, as "cargo test -p render outcome_palette_separation computes them".
- **The question:** should the 7 lines become the valid form, `cargo test -p <crate> -- <a> <b>`?
- **Options seen:**
  - (a) **Recommended.** Rewrite the 2 task lines and the 4 verify details to that form in a follow-up plan PR, through `plan/tools/reqio.py` for the details, and regenerate the fifth, `plan/reviewers/gui.md`:129. No test changes, and the 36 valid lines stay as they are.
  - (b) Rewrite only the 2 task lines, and leave the prose, which reads as a description rather than a command.
- **Applied meanwhile:** nothing. PR #422's own lines are fixed; these older lines are unchanged.
- **Waits:** nothing. TASK-M6-32 and TASK-M6-33 are merged.

## RQ-734: Final-condition charts: build them, and on what terms? *(a one-way door: what the instrument computes, and the reading of the owner's model; the owner's review of PR #422, 7 Nov 2026; context: `docs/design/principia_chart_composition.md` § 10.3, no task)*

*Queued 7 Nov 2026 from PR #422, for the owner's rulings report. Nothing is built until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner's model shows you can pick where a system is at a later time and work backwards to the start that gets there, by flipping the velocities and running forward. That gives a new kind of chart. The theorem is about the equations, and our default integrator doesn't run backwards exactly, so the app measures the miss at each pixel and masks the bad ones; an integrator that does run backwards exactly is an option. Starts also get rescaled to a standard size, and the design currently bars anything "measured after t = 0".*

- **File, section:**
  - `docs/design/principia_chart_space_model.md` Theorem 11: "A chart whose axes are state functions evaluated at time `T`" and § 0.5: "The scale-free invariant overlap is `L_z²E` and the sign of `E` only"; § 6b: "`E` and `L_z` themselves are independent of `T` only before the `I = 1` rescale."; "Theorem 11 is exact for the true flow; the app's integrators aren't run-reversible."; "That's an option to rule on, not something assumed."
  - the owner on the model, 7 Nov 2026: "Add "final-condition" charts (Theorem 11, time-reversal: flip momenta, integrate forward T, flip back) as a design item for the rulings report, with the check that conserved axes are T-independent."
  - `docs/contracts/principia_integrator_contract.md` Part 2, the reversibility flags (R-500): "`run_reversible` says the bound occupant's whole run is time-symmetric, as the reversibility test binds it: only logH's TTL time mode (R-162)"; "The default occupant is Heggie × RK4 (R-560)".
  - `docs/design/principia_chart_composition.md` § 3.1, the Rejected table: "anything measured after `t = 0` | results, not axes".
  - `docs/design/principia_chart_composition.md` § 10.3 and § 3.3's flag.
- **The questions and options:**
  - **Whether.** (a) **Recommended.** Keep it as a design item and build it after the M8 charts, as a CPU batch chart, `T` on the sim key, a mask for runs that don't survive, and the three checks of § 10.3. (b) Don't build it: the rejection stands as written.
  - **The occupant.** Theorem 11 is exact for the true flow, and in the app it's an approximate inverse. In both options the round-trip miss is measured per pixel, reported, and masked where it's large. (a) **Recommended. The default occupant, approximate inverse.** Signed `C` of the forward state at `T' = T/I_in^{3/4}` is compared with `D(ψ(u))` (the contract's `C` would mirror a lower-half `ψ(u)` and show a miss the size of the mirror on correct physics), and the miss is the integrator's error amplified by chaos. (b) **An exactly reversible integrator.** A time-symmetric integrator with a reversible step rule for this chart. logH's TTL mode is the only occupant the contract flags `run_reversible`, so this departs from the default (R-560); rounding and chaos still leave a miss.
  - **The gauge.** `F_{−T}` leaves `I ≠ 1`, and `C` rescales by a per-pixel `s`. (a) **Recommended.** Keep `I = 1`: the chart's start is an ordinary one, the `T`-independence check reads `L_z²E` and the sign of `E` after signed `C` as before it, `E` and `L_z` only before the rescale, and the sign of `L_z` after signed `C` too, flipped only on the `Enc` arm, the time reached on a pixel is per pixel as in § 8.6, and the chart carries `paper scale differs`. (b) Skip the rescale for this chart: `T` is one number across it, and it's an exception to `C` that every later stage must then carry.
  - **The mirror half.** Applied from the owner's rider to RQ-736: a start found stays in the signed state, taking § 17.2's signed `C`, with no mirror, clamp or `z`, and the chart fills the IC from the state. This item confirms that reading of Theorem 11's `Enc`.
  - **The rejection.** (a) **Recommended.** Narrow "anything measured after `t = 0`" to event reads and functionals, so a state function at a fixed `T` can be an axis of this chart. (b) Leave it, which goes with option (b) of the first question.
- **Measured and reasoned:** all of this is reasoned. No such chart has been run, and the extra run's cost, one integration of length `T` per sample, isn't measured. The scaling of `E`, `L` and `L²E` under `C` is algebra.
- **Applied meanwhile:** the rejection stands; nothing is built.
- **Waits:** nothing. No task of the design depends on it. Read at TASK-M4-37.
- **7 Nov 2026 (R-852):** the owner's order on the model's § 6b: "Add "final-condition" charts (Theorem 11, time-reversal: flip momenta, integrate forward T, flip back) as a design item for the rulings report, with the check that conserved axes are T-independent." It stays this design item, for the next rulings report; nothing is built.

## RQ-735: What does the browser build get of chart composition? *(a one-way door: what the product is, and scope under R-571; the owner's review of PR #422, 7 Nov 2026; context: `docs/design/principia_chart_composition.md` § 7.5, TASK-M8-102, TASK-M8-96)*

*Queued 7 Nov 2026 from PR #422, for the owner's rulings report. Nothing is built for either option until ruled (the owner's order of 6 Nov, A6).*

*In plain English: most of the new chart axes have no fast formula for the graphics card, so they run on the main processor, a route called CPU batch. The browser version either leaves those out, or runs the same Rust code compiled to WebAssembly, slower. The owner's rule for the browser says "the whole native app", which points at the second.*

- **File, section:**
  - `docs/contracts/principia_canonical_spec.md` (R-571): "v1 web: the whole native app, compiled to run in the browser, with the same interface, the same features and the same stains"; "where the browser cannot do something the native app does, v1 web says so in the app, and does not get a different design".
  - `docs/contracts/principia_caching_contract.md`, "Measured on Pages first (R-627)": "whether SharedArrayBuffer and wasm threads then actually work, in each browser of R-110's pair".
  - `docs/design/principia_chart_composition.md` § 13, V24: "Every § 3.1 and § 3.2 row is CPU batch"; § 7.5.
  - the owner's review of PR #422, 7 Nov 2026: "Web: state what the browser build gets. Closed-form GPU charts only, or a WASM CPU route for numerical axes and CPU-only morphs."
- **The question:** does the browser build run CPU batch?
- **Options:**
  - (a) Closed-form GPU charts only. The baked pairs run; every other chart says in the app that the browser can't. Nothing to build, and most of the new primitives aren't in the browser.
  - (b) **Recommended.** A WASM CPU route too: numerical axes, unbaked closed forms and CPU-only morphs run as CPU batch in the engine's worker, labelled and not live. It's the same code, so it adds a target and its tests, not a design.
- **Measured and reasoned:** nothing is measured in a browser. § 11's measurements are Metal and Vulkan, not WebGPU; TASK-M8-96's test of wasm threads on Pages hasn't run, so CPU batch's thread count there is unknown; and bit-for-bit agreement of a wasm f64 solve with a native one isn't measured. The recommendation is reasoned from R-571's words and V24. If TASK-M8-96 finds one thread only, (b) stands and is slower.
- **Applied meanwhile:** nothing: § 7.5 states both, and neither is built.
- **Waits:** nothing. The native tasks go ahead either way. Read at TASK-M4-37.

## RQ-737: V-11's primary states "f g = 1" at e_in = i = m3 = 0, while its own eq. (11) gives 0.7 there: which holds for the prograde limit? *(conflict inside a source; the owner's 31 Oct request, "V-11 Part I"; `docs/incoming/2026-10-31_research_library/myllari_2018/README.md`; REQ-VAL-330; context: TASK-M9-25 (was TASK-M3-65); V-11 parked by R-668 item 5)*

*Queued 31 Oct 2026 by the library ingest as RQ-639, and filed 7 Nov 2026 as RQ-737, RQ-639 being already taken on
`main`. For the owner only if the physics review can't settle it: it scales the prograde number by (1/0.7)^(1/3) = 1.126
at low e_in. It blocks nothing now: V-11 is parked with hierarchical triples by R-668 item 5 (1 Nov 2026).*

**In plain English:** the V-11 stability paper (Mylläri et al. 2018) says in words that its stability factor is 1 for a
circular, flat, prograde orbit with a light third body, but its own printed formula gives 0.7 there. The printed formula
matches the paper's own figures, so the sentence looks like the slip. The answer moves V-11's prograde comparison number by
about 13 %. V-11 is parked for now (R-668, the owner's 1 Nov ruling that parks hierarchical-triple targets), so nothing
waits on the answer.

- **File, section:**
  - Mylläri et al. 2018, MNRAS 476, 830 (local only), abstract, p. 830, and p. 836: "At the limit of ein = i = m3 = 0, f g = 1."
  - The same paper, eq. (11), p. 833, as printed: f(e_in, cos i) = {1 − ⅔e_in[1 − ½e_in²] − 0.3 cos i [1 − ½e_in +
    2 cos i (1 − (5/2)e_in^(3/2) − cos i)]}, which gives f(0, 1) = 0.7, and f = 1 at cos i = 0 (i = 90°).
  - `docs/incoming/2026-10-31_research_library/myllari_2018/formula_check.txt`.
- **The question:** whether V-11 scores against eq. (11) as printed, or treats the stated limit as the paper's intent.
  Eq. (11) as printed reproduces Fig. 5's curves at cos i = +1 (3.15, 3.97, 4.79 against about 3.2, 4.0, 4.75, by eye), so the
  sentence looks the likelier slip; it may mean i = 90°.
  At the limit itself (e_in = 0, cos i = +1), eq. (11) as printed gives 2.37, 2.99, 3.61 at e_out = 0.3 (eq. (13),
  `formula_check.txt`), against about 2.35, 3.0, 3.6 read by eye from Figs. 7 and 8; the stated limit (a factor of 1.126)
  would give about 2.67, 3.37, 4.07. Fig. 7's caption says e_out = 0, where eq. (13) gives 2.24, 2.82, 3.40, so the match
  holds only if Fig. 7's curves are drawn for e_out = 0.3, as Fig. 8's are (`myllari_2018/README.md`).
- **Options seen:** (a) eq. (11) as printed; (b) the stated limit, f g = 1 at e_in = i = m3 = 0; (c) another, stated by
  the owner.
- **Applied meanwhile:** eq. (11) as printed, in the record. Nothing is built; REQ-VAL-330's text and TASK-M9-25 are
  unchanged. V-11 (REQ-VAL-330, the hierarchical-triple stability target) is parked by R-668 item 5 (1 Nov 2026), the
  owner's words: "5: hierarchical triples out of scope for now; park them with the v2+ 3D extension; shrink the M9 list
  accordingly." It reads "not scored: parked" in the M9 report (`decisions.md` § R-668, "Item 5"), so the question
  blocks nothing now.
- **Waits:** nothing now: V-11 is parked by R-668 item 5. Its prograde comparison number waits on this only if the
  parking is lifted and V-11 is scored.

## RQ-746: A saved curved chart tilted again into its own saved target, where the saved tilt is Lean each axis with one target and both angles set: what is `R`'s third input? *(physics; silence; R-513 Q1, R-474, R-475 L5; context: TASK-M2-40, REQ-CHART-077, TASK-M8-48)*

*Queued 8 Nov 2026 by TASK-M2-40's implementer, for the physics reviewer and, if they can't settle it from the corpus, the owner's rulings report. A one-way door if it reaches the owner: it changes what a saved chart computes.*

**In plain English:** when you save a tilted curved view and tilt it again into the same hidden coordinate, R-513 says the two rotations compose: the new tilt's offset on that coordinate is fed into the saved rotation as its third input. That needs the saved rotation's third column. Every saved tilt has one except a "Lean each axis" tilt with one target and both angles set, which isn't a rotation: its two tilted axes are skewed, so nothing says where the third input goes.

- **File, section:**
  - `decisions.md` § R-513, Q1: "The new tilt's offset on the saved target enters `R`'s third input"; and its ruling, "`(a, b, d) = R·(u′, v′, d′)`, that is `R·R′·(u, v, 0)`".
  - `decisions.md` § R-474 and `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)", "Two tilt maps": "**Lean each axis**, an option (`lean`), is the formula above per axis, `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_X`, skewed (`u₁·u₂ = sin τ₁ sin τ₂`)." Its map gives `R·e_a` and `R·e_b` only; with one target and both angles set it is no rotation, so `R·e_X` is not defined.
  - `decisions.md` § R-513, Q3: "The saved target is offered again in the Tilt section's target dropdowns of a saved curved chart".
- **Options seen:**
  - (a) **Recommended**: keep R-513's ruled form `R·R′` literally, with `R = [û₁ û₂ t̂]`, `R`'s third input `t̂` the unit normal of the saved plane, `û₁ × û₂` normalised, the sign toward `+e_X`. From R-514 it borrows only `t̂` (a target projected off the saved plane and normalised); it does not apply the new tilt as R-514 does (option (b)). The composition is then no longer a rotation, as the saved Lean map isn't; a single `τ′` tips one saved axis toward `t̂` and leaves the other where it was. At the edge of Lean's range, `|τ₁|` or `|τ₂|` = 90° with the other angle set (R-474 R8's dead zone), `û₁ × û₂ = (−s₁c₂, −c₁s₂, c₁c₂)` has no `e_X` component (`c₁c₂ = 0`), so "toward `+e_X`" fixes no sign and (a) takes the cross product's own; at the corner `|τ₁| = |τ₂| = 90°` the cross product is 0 and (a) has no frame (R-554's `ray-degenerate`).
  - (b) **R-514's frame action**, carried over from latent saved charts (R-514 I1, I3, I4): the new tilt acts as the rotation `M = F·R′·Fᵀ` in the orthonormal frame `F = (q̂₁, q̂₂⊥, t̂)` Gram–Schmidt makes of the saved axes and the target, `q̂₁ = û₁`, `q̂₂⊥` the second axis made orthogonal to the first, and carries the stored pair: `(a, b, d) = M·(u·û₁ + v·û₂)`. The new tilt stays an isometry of the scaled 3-space acting on the saved plane, but the result is not of `R·R′` form: with `U = [û₁ û₂]` skewed, `Fᵀ·U` is upper-triangular and does not commute with `R′`, so `U·R′ ≠ F·R′·Fᵀ·U`. At the dead-zone edge `e_X` lies in the saved plane, so its projection vanishes and R-514 I2 drops it as a target: the saved target is not offered at all; at the corner `|τ₁| = |τ₂| = 90°` (b) has no frame either (R-554's `ray-degenerate`).
  - (c) Refuse the case: a saved Lean view with one target and both angles set doesn't offer its saved target to a new tilt (Q3 holding everywhere else).
  - (d) Another, stated by the owner.
- **Worked numbers** (physics review 5816819537 of PR #447, F1, checked by the implementer): a saved Lean each axis view at `τ = (30°, 40°)`, one target `X`, then `τ′ = (20°, 0)` into `X`, at the pixel `(u, v) = (0.3, 0.7)` from the centre, in `(a, b, d_X)`: option (a) gives `(0.2026, 0.4759, 0.6628)`; option (b) gives `(0.1598, 0.4307, 0.7099)`. Both are continuous, and both are the saved view at `τ′ = 0`; they are two different answers.
- **Applied meanwhile:** the kernel refuses that composition (`kernel::chart::saved::Composed::new` returns `None`; `TiltSpec::inputs` returns `None` for that tilt), computing nothing it can't justify. A new tilt into any other target of such a chart composes as the block form, which needs no third input, and every other saved tilt composes as R-513 rules.
- **Waits:** only that case: a new tilt into the saved target of a saved curved chart whose tilt is Lean each axis, one target, both angles set. REQ-CHART-077's own verify (a 25° single-angle saved tilt) doesn't reach it. The case is split from REQ-CHART-077 as REQ-CHART-149, carrying `rq: RQ-746`, closed by TASK-M2-56 once ruled; TASK-M8-48's target dropdowns don't offer the saved target in that case meanwhile (physics review 5816819537 of PR #447, F2).
## RQ-744: R-863's build order: the tasks that fit no stage or straddle two, and the stage-3 edges the milestones and the existing edges refuse *(the owner's order of 8 Nov 2026, R-863; a one-way door where an answer moves a task out of its milestone's gate, otherwise a reading of the owner's order; `docs/design/principia_chart_composition.md` § 15; context: TASK-M4-32, TASK-M4-33, TASK-M4-34, TASK-M7-72, TASK-M8-100, TASK-M8-102 to TASK-M8-107, TASK-M8-112, TASK-M8-117, TASK-M8-118, TASK-M8-122, TASK-M8-123, TASK-M8-124)*

*Filed 8 Nov 2026 by the rulings PR that ports R-863 and R-864 (PR #445). The orchestrator's instruction: where a task
fits no stage or straddles two, don't choose. Nothing below is given a stage edge.*

*In plain English: the owner set five stages for building the chart-composition tasks. Most tasks fall into one stage
and now wait on the stage before. Some fall into none, one falls into three, and three stage-3 tasks can't be made to wait
on stage 2: two sit in an earlier milestone (M4), which may not wait on M8 work, and all three are already needed by
stage 2's task through the chain the design drew. And Save view as chart's tasks aren't ordered against stage 1. (Stage
2's sliding anchor, which had no task when this was filed, now has one: the 7 Nov rulings, merged since, give it to
stage 2's task.) The owner says where each goes.*

- **File, section:** `decisions.md` § R-863, the owner's words: "build in this order: (1) the path object and the bitwise
  tests that today's pan, zoom, tilt, slice, lock and Save-view-as-chart reproduce exactly through it, before any new
  capability; (2) locked paths (based morph, then the sliding anchor) with the test that the centre IC is constant; (3)
  the primitive registry and composed axes, each primitive with its round-trip test; (4) morph families, easing and
  Sequence segments; (5) time-axis charts and the symmetry filter. Each stage merges only with the previous stage's tests
  green."; `docs/design/principia_chart_composition.md` § 15, "Tasks, milestones chosen by what depends on them";
  `plan/check_plan.py`: a task may not depend "forward across milestones", and the graph may have no cycle.
- **Applied:** stage 1 TASK-M8-124 (new); stage 2 TASK-M8-117; stage 3 TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101,
  TASK-M8-109, TASK-M8-110, TASK-M8-111, TASK-M8-119; stage 4 TASK-M8-108; stage 5 TASK-M8-113 to TASK-M8-116 and
  TASK-M8-120. Edges: TASK-M8-117 on TASK-M8-124; TASK-M8-101, -109, -110, -111 and -119 on TASK-M8-117; TASK-M8-108 on
  every stage-3 task; every stage-5 task on TASK-M8-108.
- **(A) Stage-3 tasks with no edge to stage 2.** TASK-M4-33 (the registry) and TASK-M4-34 (region D's pair) are in M4,
  TASK-M8-117 in M8, so the edge would depend forward across milestones. And TASK-M4-33, TASK-M4-34 and TASK-M8-100 (warps
  and pair maps) are upstream of TASK-M8-117 already: TASK-M8-117 → TASK-M8-106 → TASK-M8-102 (CPU batch) → TASK-M4-33,
  TASK-M4-34, TASK-M8-100. So today stage 2 waits on these three stage-3 tasks, against the order. TASK-M4-34 also holds
  TASK-M4-06 once RQ-714 rules (TASK-M4-37's Deliverables), so moving it behind stage 2 would hold M4's golden behind the
  path work.
- **(B) Straddles: TASK-M8-106** (the path type, the line on blocks, export through CPU batch). Its path type and its
  bitwise tests (`path_tilt_bitwise`, `path_zoom_bitwise`, `path_lock_bitwise`, `path_save_view_bitwise`, REQ-CHART-113)
  are stage 1, now TASK-M8-124's too; its line on blocks is the morph a based morph (stage 2) needs, or one of stage 4's
  families; its re-timing, alignment and export are stage 4. Its edge to TASK-M8-102 is what puts (A)'s three tasks
  before stage 2.
- **(C) No stage names them:** TASK-M4-32 (the composition in the contracts, docs), TASK-M8-105 (chart paths in the
  contracts, docs), TASK-M8-102 (CPU batch, numerical inverses), TASK-M8-103 (the builder's panels), TASK-M8-104 (1D
  charts), TASK-M8-107 (live paths: the blend entry; R-863 names no live stage), TASK-M8-112 (paper frames as presets),
  TASK-M8-118 (hold `L_z`) and TASK-M7-72 (contours of any primitive).
  - *8 Nov 2026, on merging `main` (PR #443):* TASK-M8-122 (the fixed-`k` chart) and TASK-M8-123 (the binary-plus-single
    scattering chart), R-847's, wait behind TASK-M4-37 too. They are not PR #422's tasks, so R-863's words don't name
    them; both already depend on stage-3 tasks (TASK-M8-109 and TASK-M8-111) and on TASK-M8-102. No stage edge is made;
    the options below place them with (C)'s tasks.
  - *8 Nov 2026 (R-870):* TASK-M8-126, points of interest, joins this list: the owner's stages don't name it. It
    depends on TASK-M8-124 (stage 1) and TASK-M8-108 (stage 4), so it can't be built before stage 4, but no stage edge is
    made for it.
- **(D) Stage 2's sliding anchor has no task on `main`.** The owner's stage 2 is "based morph, then the sliding anchor".
  TASK-M8-117 is the based morph with the *stored* anchor: its `path_lock_centre` asserts "`anchor not in B` labelled
  and unbased", and design § 17.8 (`docs/design/principia_chart_composition.md`) says "Its sliding anchor (Theorem 8,
  "the RQ-721 rider") is RQ-721's, not applied here: until then `anchor not in B` stands". The sliding anchor is the
  owner's RQ-721 rider in the open rulings PR #443 ("no labelled jumps under the lock. Where the anchor can't exist in
  chart B ... the anchor slides continuously to its nearest point in B"), not yet on `main`. Until that answer is
  ported, with a task that builds it in stage 2 and replaces TASK-M8-117's `anchor not in B` arm, stage 2 is incomplete,
  and stage 3 waits only on its based-morph half.
  - *Moot, 8 Nov 2026, on merging `main` (PR #443):* the rider is R-849, now on `main`, and it gives the sliding anchor
    to TASK-M8-117 (REQ-CHART-115's acceptance, and REQ-CHART-146, the definition of B's nearest point, under physics
    review), in place of `anchor not in B`. Stage 2 is that one task; nothing is left for the owner here.
- **(E) Save view as chart's tasks are not ordered against stage 1.** The owner's stage 1 names "today's ...
  Save-view-as-chart", and R-864 item 2 says "No parallel code paths". On `main` no Save view as chart code exists: its
  routes are TASK-M6-49 and TASK-M6-51 (the mock, R-475, R-514) and TASK-M8-48 (the real engine, curved views; R-475),
  none merged, and TASK-M8-124's Depends on names none of them. If TASK-M8-48 lands after TASK-M8-124, it builds a Save
  view and tilt and slice route beside the object, and TASK-M8-124's bitwise baseline, "today's code", misses it. Either
  edge is a choice: TASK-M8-124 after TASK-M8-48 holds stage 1 behind M8's GUI work; TASK-M8-48 after TASK-M8-124 holds
  TASK-M8-48 behind TASK-M4-37 and the owner's answer to R-864's sketch, and builds Save view as chart as a family of
  the object. TASK-M6-49 and TASK-M6-51, in M6, can only come before TASK-M8-124 by an edge (an M6 task may not depend
  forward on an M8 one), or after it by dispatch.
- **Options:**
  - (a) Split and re-place: TASK-M8-106 keeps only what is past stage 1 (its line, re-timing, alignment and export),
    drops REQ-CHART-113's tests to TASK-M8-124, and TASK-M8-117 depends on TASK-M8-124 and TASK-M2-23 and not on it,
    so stage 2 no longer waits on stage 3; TASK-M4-33 and TASK-M4-34 move to M8 (new ids, their requirements leaving
    M4's gate, TASK-M4-06's edge then from M8), or stay in M4 under (b); (C)'s tasks placed by the stage they serve:
    TASK-M4-32 before stage 3 and TASK-M8-105 before stage 1 (docs first), TASK-M8-102 and TASK-M8-118 in stage 3,
    TASK-M8-107 and TASK-M8-103, TASK-M8-104 and TASK-M8-112 in stage 4 or 5 as the owner says, TASK-M7-72 in stage 3.
  - (b) Keep the milestones: the stage-3 tasks in M4 keep their place, and the order across M4 and M8 is kept by
    dispatch, the orchestrator releasing a stage's M4 tasks only once the previous stage has merged, not by edges.
    (B) and (C) as in (a).
  - (c) Another, stated by the owner.
  - For (E): (i) TASK-M8-124 depends on TASK-M6-49, TASK-M6-51 and TASK-M8-48, its baseline covering their routes;
    (ii) TASK-M8-48 depends on TASK-M8-124 and builds Save view as chart on curved views through the object, TASK-M6-49
    and TASK-M6-51 before stage 1 by edge; (iii) another, stated by the owner.
- **Recommendation:** (a) for (B), since it is the split that makes the stage order hold by edges; for (A), (b), since
  moving TASK-M4-33 and TASK-M4-34 out of M4 changes M4's gate; for (E), (i), since "today's" reads as the routes that
  exist when stage 1 starts and R-475 is ruled, and stage 1 already waits on TASK-M4-37. A recommendation, not applied.
- **Applied meanwhile:** the edges above; none for (A), (B), (C) or (E); (D) moot by R-849, recorded in R-863, design
  § 15 and TASK-M8-117's Notes. Every task here stays behind TASK-M4-37.
  - *8 Nov 2026 (R-866, applied per R-369):* TASK-M8-125, locking without moving the view, is in stage 1, after
    TASK-M8-124's bitwise re-expression, and TASK-M8-117 depends on it.
- **Waits:** nothing can start yet: all of it waits behind TASK-M4-37 and, for TASK-M8-124, the owner's answer to R-864's
  type sketch.

## RQ-745: The acute-angle Burrau charts show `θ ∈ (0, π/4]` only, one labelling of each shape, the leg swap folded out: does that stand under the symmetry rule and the filter? *(a one-way door: what the instrument shows; R-27 against the owner's symmetry rule and symmetry-filter order of 7 Nov 2026, R-854 and R-855; `crates/kernel/src/chart/burrau_acute.rs`; context: REQ-CHART-024, REQ-CHART-025, REQ-CHART-027, TASK-M8-120)*

*Filed 8 Nov 2026 by the rulings PR that ports R-863 and R-864 (PR #445), from the orchestrator's read of the built
charts against the 7 Nov rulings.*

*In plain English: three built Burrau charts sweep the triangle's acute angle only up to 45°, because the angle and its
complement give the same triangle with its two legs swapped. So each shape appears once and its swapped twin never does.
The owner's 7 Nov symmetry rule says a symmetry may be shown once only by an optional filter after rendering, never by
removing starting states or shrinking a chart. An older ruling (R-27) keeps these folded charts on purpose, labelled
"shape only", beside the full-range charts. The owner says which stands for these charts.*

- **File, section:** `crates/kernel/src/chart/burrau_acute.rs`:6-12, the module note: "`θ ∈ (0, π/4]` is horizontal, …
  `θ` folds out the leg swap (`θ` and `π/2 − θ` are the same triangle with the legs swapped), so each chart covers each
  shape once: [`SystemImage::Bijective`], the quotient "shape only" ([`ChartLabel`])."; :89-92, `in_fold`, "Layer 2's
  `θ ∈ (0, π/4]`"; :121-124, `fold_label`, "The acute-angle charts' label: `name`, bijective, "shape only" (the module's note)"; :144,
  `reject_early`, which rejects a `θ(u)` outside the fold.
- **Against:** `decisions.md` § R-854, the owner's symmetry rule of 7 Nov 2026, the owner's words: "Where a symmetry is
  used at all (counting mirror pairs once, the IC constructor showing that a triangle and its flip are mirror images),
  it is a declared, optional view or statistic, never a removal of starting states or angles. Every chart shows the full
  range of the angles it sweeps."; and `decisions.md` § R-855, the owner's symmetry-filter order of 7 Nov 2026, the
  owner's words: "Region D's quarter disc, the Anosova half-disc and the half shape sphere at rest come out of this
  filter rather than out of reduced chart domains, so the papers' figures are reproducible with the full disc still
  computed underneath."
- **For:** `decisions.md` § R-27 (24 Sep 2026): "Shape only (the fold) and shape × labelling (the full range) are both
  kept."; `docs/design/principia_chart_reference.md` § 4.5.
- **Options:** (a) keep them as R-27 has them, labelled "shape only", beside the full-range `ν` charts; (b) re-express
  them as the full chart through the symmetry filter (TASK-M8-120), the leg swap shown once only when the filter is on;
  (c) another, stated by the owner.
- **Applied meanwhile:** nothing. The charts stay as built, labelled "shape only" (R-27); no code or doc changes.
- **Waits:** nothing built waits; under (b) a task re-expresses the three charts once TASK-M8-120's filter exists.
- **8 Nov 2026, the owner's direction (R-869):** "5. Burrau acute charts: unfold the leg swap; show both labellings, with the reduced view from the symmetry filter (R-855). Add it to the review queue with that as my direction." That is option (b). The owner asks for it to be queued with that as their direction, so the entry stays open for their ruling at the next rulings report, where R-27's "shape only" fold charts would give way. No task is added and nothing changes meanwhile.
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

## RQ-739: Remove the decoder's mirror step, letting latent `β` run over the full circle? *(the owner's proposal of 7 Nov 2026 with the symmetry rule, R-854; a one-way door: it changes the decoder; R-82, R-438, R-544, R-408; context: no task)*

*Filed 7 Nov 2026 by the rulings PR of that day, for the next rulings report. The owner's own words make it one-way, and "Mirror-step removal stays a proposal until ruled: don't apply it, just tag the sections it would touch." Nothing is applied; the sections it would touch carry the marker "[mirror-step proposal, 7 Nov 2026 (R-854; RQ-739)]".*

*In plain English: today a starting shape and its mirror image decode through one canonical half, with a special case for the shape sphere. The owner proposes dropping that step, so every chart uses one decode path and a lower-half start gets its own latent coordinates. The owner asked for five things to be reported before deciding.*

- **File, section:** the owner's symmetry rule of 7 Nov 2026, its proposal (R-854, which quotes it whole); `docs/design/principia_chart_composition.md` § 6.6.
- **The owner's proposal, verbatim (7 Nov 2026):**

  > Proposed change, for the rulings report with a recommendation (it changes the decoder, so
  > one-way): remove the decoder's mirror step entirely and let the latent beta run over the full
  > circle, so every chart, the shape sphere included, uses one decode path with no special case.
  > Report before deciding:
  > 1. Bitwise check: every existing preset, saved chart and golden test with beta in [0, pi]
  >    reproduces unchanged (expected, since the mirror step never fires there); list any that don't.
  > 2. What becomes unnecessary: the mirror step, its deadband rules (R-82, R-438) and the shape
  >    sphere's separate decode path (R-544's skip).
  > 3. What needs a new option: R-408's count-each-system-once statistics become "mirror pairs counted
  >    once" as an optional statistic, never by dropping starts.
  > 4. Encode: a lower-half state now encodes to beta in (pi, 2 pi) instead of a mirrored state; list
  >    every stored z, IC save file or test that relied on the old mirrored encode.
  > 5. Cost: latent charts covering both halves have twice the area in beta; state the effect on
  >    default windows.

- **Tagged sections (the order's item 4):** `docs/design/principia_dd_decoder.md` § 3.3; `docs/design/principia_dd_encode.md` § 3.2; `docs/contracts/principia_inverse_encode_contract.md` Part 1; `docs/design/principia_chart_reference.md` § 0.4 and § 3; `docs/contracts/principia_chart_decoder_contract.md` Part 2 and Part 5; `docs/contracts/principia_canonical_spec.md` § 3; and `docs/design/principia_chart_composition.md` §§ 3, 6.2, 8.4, 10.3, 10.4, 17.2 and 17.8 (PR #422).
- **Owed before a recommendation:** the five reports the proposal lists, each measured or read, its criterion written first (R-649): the bitwise check over every preset, saved chart and golden test with `β ∈ [0, π]`; what becomes unnecessary (the mirror step, R-82's and R-438's deadband, R-544's skip); R-408's statistics as an optional "mirror pairs counted once"; every stored `z`, IC save file or test that relied on the mirrored encode; and the default windows' cost of twice the area in `β`.
- **Options:** (a) remove the mirror step, as proposed, once the five reports are in; (b) keep it, the symmetry rule (R-854) and the filter (R-855) standing on the signed `C` they already read.
- **A dependency, one-way (7 Nov 2026, the physics review of PR #443, P2):** R-853's answer to RQ-736's first open question, the owner's words: "Colour later states and draw traces in the signed, unfolded state; traces are continuous through syzygies." Against it, `docs/design/principia_dd_decoder.md` § 3.2's link `β = π·σ(z_β)`, with `β ∈ [0, π]`, and `docs/design/principia_lines.md` § "8. Traces", the table row: "`z_β` at a syzygy | It runs to ±∞ and comes back: a jump at infinity, not a sample off the chart". At a syzygy `β` is 0 or π, so `z_β` runs to ±∞, and a signed lower-half state has no `z_β` under the present link; under this entry's `β ∈ (0, 2π)` a crossing at `β = 0` would still wrap. So on any chart showing `z_β` a signed trace can't be continuous through a syzygy; the latent momenta chart can. TASK-M7-52's definition (R-72, how a lower-half state reads on a latent target) needs latent coordinates beyond `β ∈ [0, π]`, which is this entry's item 4. Not resolved here: REQ-COL-163's test is scoped to the latent momenta chart, and the `lines.md` row is tagged as waiting.
- **Applied meanwhile:** nothing. The decoder and its tests are unchanged.
- **Waits:** nothing built; no task depends on it.
- **8 Nov 2026 (R-865):** the owner ruled the removal: "1. RQ-739: remove the decoder's mirror step; latent beta runs over the full circle. Conditional on the bitwise check (every preset, saved chart and golden with beta in [0, pi] reproduces unchanged); if anything doesn't, bring the list back before proceeding. This also fixes the latent-chart traces jumping at syzygies. Add a task for it." That is option (a), built by TASK-M2-57. The entry was moved to the archive by PR #449's first head and is reopened by its fix round (code review 5817349158, F1), since the answer doesn't settle all of it (`.claude/commands/rulings-apply.md` § 2). **Still open:** the proposal's item 3, R-408's count-each-system-once statistics as an optional "mirror pairs counted once" statistic, never by dropping starts; R-408's count spans `β ∈ (π, 2π)` once the step goes, and nothing rules the statistic. Flagged for the next rulings report. The link question the condition meets is RQ-750's.

## RQ-740: Does R-708's "every pixel is a system under every tilt" allow an f32 chart to refuse a pixel inside the band where f32 rounding, not the chart, decides? *(a one-way door: what the instrument computes; R-708, R-471; the report of 7 Nov 2026, "Open from today's merges, to be filed as entries"; context: TASK-M2-52 (PR #388, merged))*

*Filed 7 Nov 2026 by the rulings PR of that day. The report put this to the owner before it had an entry, and the owner's answer asked for a measurement first, so it is filed here as an ask.*

*In plain English: on the tilted held-value charts, the chart is promised to show a real three-body system at every pixel inside its square. The warp puts the feasibility limit on the bottom row, where the held constraint is just met. A pixel on or near that floor never leaves the square, but at f32 (single precision) it can decode to a state just past the constraint (for example `K` just below `L_z²/2I`, or a layer-3 check failing on the f32 decode) and be refused, though at f64 the same pixel is a system. The question is whether such a refusal breaks the promise.*

- **File, section:** `decisions.md` § "R-708 — The tilted Burrau `K` charts follow R-471's two modes: …", the owner's words: "Warped mode (E1, the default): option (a). Every pixel is a system under every tilt."; `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid".
- **The report's item (7 Nov 2026):** "Whether "every pixel" allows refusing inside the f32 band. R-708 (held energy's warp) has no phrase for that band. Warped invariant charts at f32 refuse 0 of 3.16 million pixels; `HeldLz`, `HeldK` and Burrau at f32 aren't covered."
- **Needed:** whether a refusal inside the f32 band is allowed under R-708's "every pixel", or such a pixel must be shown; the measurement the owner asked for comes first.
- **Applied meanwhile:** nothing. `Tilted::validate` on `main` (TASK-M2-52, PR #388) refuses where its f32 checks fail, as built.
- **Waits:** nothing built; no task depends on it.
- **Asked, 7 Nov 2026:** the owner: "f32 band: measure HeldLz, HeldK and Burrau at f32 first and report." Flagged for the next report.
- **7 Nov 2026:** the measurement the owner asked for was run that day, its criterion written before the first run (R-649), on PR #388's head 5e3f45be over `HeldLz`, `HeldK` and Burrau's charts at f32 against f64, and reported to the owner the same day; the criterion and the logs are in the orchestrator's notes (`orch/scratch/f32band/`, outside the repository). The question stays open for the owner's ruling on it.

## RQ-741: What does a warped virial axis at a held `L_z ≠ 0` say it reads when its asked bottom `Q₀ > 0`? *(a one-way door: what the instrument claims; R-817, R-506; Codex on PR #443, 7 Nov 2026; context: TASK-M8-119, REQ-CHART-103)*

*Filed 7 Nov 2026 by the rulings PR of that day, from Codex's finding on PR #443 (`plan/tasks/M8/TASK-M8-119.md`). Nothing is chosen.*

*In plain English: the virial ratio compares kinetic and potential energy. On a spinning chart it can't go below a floor that depends on each pixel's shape. Today the axis is labelled "Q above the floor", because it reads the ratio less that floor. The owner's ruling of 7 Nov (R-817) puts the floor under the rise only, so where the asked bottom of the range is above the floor, the axis now reads the ratio itself. On the other columns it reads the asked bottom plus the excess above the floor. The label "Q above the floor" stays true only when the range starts at 0. What the axis is labelled when it starts above 0 is not said anywhere.*

- **File, section:**
  - `decisions.md` § "R-817 — A warped virial axis with an asked bottom `Q₀ > 0` at a held `L_z ≠ 0` puts the floor under the rise only: …": "the pixel's ratio is `max(Q₀, Q_min(U)) + (Q_hi − Q₀)·t^γ_K`, so the bottom row is the asked bottom `Q₀` where `Q₀` is feasible and the floor elsewhere, and the rows span the asked range."
  - `docs/design/principia_chart_composition.md` § "9. Then virial": "the label at `L_z ≠ 0` is `Q above the floor`, with the state's own `Q` in the readout."
  - `plan/requirements.yaml` REQ-CHART-103, as it stood: "a warped axis at a held L_z ≠ 0 labelled Q above the floor".
- **The silence:** the axis's coordinate is chart_reference § 2.1's `Q(t) = Q₀ + (Q_hi − Q₀)·t^γ_K`, so its forward, the inverse of R-817's decode, is `Q₀ + 2(K − max(Q₀·|U|/2, L_z²/2I))/|U|`: the state's own ratio `2K/|U|` on a column where `Q₀ ≥ Q_min(U) = L_z²/(I·|U|)`, and that ratio less `Q_min(U) − Q₀` on the rest. At `Q₀ = 0` it is `2(K − L_z²/2I)/|U|`, the built axis, and `Q above the floor` is exact. At `Q₀ > 0` no file names the label, and one label can't be exact on both kinds of column.
- **The question:** on a warped virial axis at a held `L_z ≠ 0` with `Q₀ > 0`, what is the axis labelled?
- **Options seen:** (a) `Q`, the ratio, with the columns where `Q₀` is infeasible marked (as the floor is) and the state's own ratio in the readout; (b) `Q above the floor` throughout, its meaning stated as `Q₀` plus the excess above the floor; (c) a label per column kind, `Q` where `Q₀` is feasible and `Q above the floor` elsewhere; (d) another, stated by the owner.
- **Applied meanwhile:** REQ-CHART-103 and TASK-M8-119 take R-817's forward and its round trip, and `Q above the floor` at `Q₀ = 0` only, both settled by R-817 and § 2.1; they name this entry for the label at `Q₀ > 0`.
- **Waits:** TASK-M8-119's label at `Q₀ > 0` only; its descriptor, forward and decode go ahead.

## RQ-747: Where are the latent chart's tilt angles stored, when R-474 says "angles are stored as set" and TASK-M6-48 adds no angle field? *(a one-way door: what the instrument computes and records; R-474 R3, K3, K15; REQ-GUI-233; `docs/contracts/principia_gui_state_contract.md` § 2; context: TASK-M6-48, TASK-M6-49, TASK-M6-56, TASK-M8-124)*

*Filed 8 Nov 2026 by the rulings PR of that day, from the physics review of the chart-path design work. Nothing is
chosen.*

*In plain English: under the new default tilt map, "Tip as sheet", each of the two tilt angles can go to ±180°, and
the owner's ruling says the angles are stored as set. But the task that builds it adds nowhere to store them: the GUI
works the angles out again from the view's axes each time. Past a total tip of 180° that gives back different, wrapped
angles from the ones set, so the sliders show other numbers, and the next keyboard step tilts to a different plane from
the one that stepping the set angles would give. Where the angles live also decides whether undo, saved views and the
chart-path object see them.*

- **File, section:**
  - `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, …": R3, "Angles are stored as set and **wrapped only in
    total tip, at 360°** … This is the same rotation, so nothing changes."; K3, "the stored angles are as set, and the
    rotation is computed from them, wrapped internally"; K15, "Under Tip as sheet an angle may be stored past ±90°, which
    Lean each axis's range does not hold. Choosing Lean each axis clamps each such angle to ±90°, its sign kept, in the
    same history entry as the map change".
  - `plan/requirements.yaml` REQ-GUI-233, verify: "at τ = (100°, 30°) under Sheet the readout holds "same plane as
    (−72.4°, −21.7°), mirrored" and the stored angles are bit-identical after it".
  - `plan/tasks/M6/TASK-M6-48.md` § Deliverables: "`SimConfig`'s `tilt_map` in the contract
    (`crates/engine/src/contract/sim_config.rs`), default Sheet, its absence read as Lean." No angle field.
  - `docs/contracts/principia_gui_state_contract.md` § 2: "`SimConfig`'s slice frame gains `tilt_map: Sheet | Lean`
    (`SliceFrame { normal, tilt_x, tilt_y, tilt_map }`)", where `tilt_x` and `tilt_y` are the targets
    (`crates/engine/src/contract/sim_config.rs`, `SliceFrame`); § 2's heading, "The editable state is the entire coupling
    surface", with "every knob is a typed field on the state"; and § 2, "A curved chart's tilt (R-473) is part of
    `SimConfig`'s chart params: the angles, the targets (indices of its hidden coordinates) and the map".
- **The silence:** with no angle field, the angles are read back from the basis (`View::framed`,
  `crates/gui/src/explore/manifold_view/mod.rs`). A view with total tip `m > 180°` reads back as its wrapped angles:
  `(150°, 150°)` reads back as `(−104.6°, −104.6°)`. The sliders show those, and a +1° keyboard step gives
  `(−103.6°, −104.6°)`, a different plane from that of `(151°, 150°)`, which wraps to `(−104.4°, −103.7°)`. So R3's
  "computed from the stored angles" does not hold, and at `m = 180°` the read-back also loses the split between the tip
  axis and the in-plane turn `γ`. No file says where the stored angles live.
- **The question:** where are the latent chart's tilt angles stored?
- **Options seen:**
  - (a) In `SimConfig`, beside `tilt_map`, always written: chart construction, undone in the same entry as the basis,
    in saved views and exports (R-474 R5). It amends TASK-M6-48's Deliverables and gui_state_contract § 2's `SliceFrame`
    line, and changes the canonical text and its `qa_TASK-M0-18` pins once more (TASK-M6-48 changes them already). The
    chart-path object's starting point carries them, since it holds `SimConfig`. Its precedent: § 2 already puts a curved
    chart's angles in `SimConfig`'s chart params; only the latent chart lacks the field. Its cost: every route that
    writes the basis other than through the angles needs a rule for the stored angles, or the two disagree. Those routes
    are the typed basis (TASK-M6-56), the compass steers and picks, the presets, the Tilt reset and Home; the rule could
    be "re-derive by the read-back" or "reset to the preset's".
  - (b) In the GUI's own scratch state, outside `SimConfig`. No schema change, but undo, saved views and exports don't
    carry them; it conflicts with § 2's "every knob is a typed field on the state"; and the chart-path object's starting
    point would miss them, so a path's starting point would be incomplete. K15's "one Ctrl+Z restores the map and the
    angles together" presumes the angles are in the undoable state, which (b) leaves them out of.
  - (c) The status quo, TASK-M6-48 as its file stands: no stored angles, the read-back standing under either map. R3's,
    K3's and REQ-GUI-233's "stored angles" are then read as the read-back's, which amends R3's wording and accepts the
    divergence above past `m = 180°`.
  - (d) Another, stated by the owner.
- **Recommended:** (a). It is the one option under which R3 holds as worded and every path sees the angles the user
  set. A recommendation, not applied.
- **Applied meanwhile:** none. The chart-path stage 1 (TASK-M8-124) decides nothing on it: its baseline freezes
  whichever behaviour lands, and the task that stores the angles re-freezes it.
- **Waits:** TASK-M6-48 (its file's Waits line), and through it TASK-M6-49's Save view, which records the view, and
  TASK-M8-124's baseline.

## RQ-749: Once locking leaves the pin off the view's centre (R-866), what does stage 2's "the centre IC is constant" test read: the centre pixel or the pin? *(a one-way door: what the lock's test claims; R-863 item 2 against R-866; `docs/design/principia_chart_composition.md` § 17.8; REQ-CHART-115; context: TASK-M8-117, TASK-M8-125)*

*Filed 8 Nov 2026 by the rulings PR that ports R-865 to R-870, where porting R-866 met R-863's words. Nothing is
chosen.*

*In plain English: the owner's build order says the locked-paths stage is proved by a test that the centre's starting
state stays constant along the path. The owner's new lock rule leaves the view where it is when you lock, so the locked
point is usually not at the centre of the view but wherever you clicked. The test as written reads the centre. The owner
says whether it should read the locked point instead.*

- **File, section:**
  - `decisions.md` § R-863, the owner's words: "(2) locked paths (based morph, then the sliding anchor) with the test
    that the centre IC is constant".
  - `docs/design/principia_chart_composition.md` § "17.8 Lock: a based homotopy": "The test reads a probe at `(½, ½)`,
    since an even grid has no pixel there."; and "**With an excursion** (`δ ≠ 0`): the pin is the current `z₀`, as
    "turns about the pin"; the words are that point's".
  - `plan/requirements.yaml` REQ-CHART-115, which reads the centre three times: its statement has the anchor's IC words
    "taken at the centre on every frame", "the centre pixel follow it" for the sliding anchor, and "the area factor at
    the centre within 0.5 of the ends'"; its verify has "a probe at (½, ½) reads one set of words on every frame, ends
    included, affine and nonlinear, δ = 0 and δ ≠ 0".
- **Against:** `decisions.md` § R-866, the owner's words: "The pin is set at the clicked pixel with an initial in-plane
  excursion equal to that pixel's offset from the view centre; tilt and zoom turn about the pin, so the pinned pixel
  stays fixed on screen."; and `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)":
  "where `δ` has an in-plane part the pin is off-centre".
- **The conflict:** after R-866 a fresh lock has an in-plane excursion wherever the click was off the centre, so the
  anchor sits at the pin, not at `(½, ½)`. § 17.8 reads the anchor's words at the centre and calls the current `z₀` the
  pin under an excursion; the contract and R-866 put the pin at the clicked pixel. Which the stage-2 test holds constant
  changes what TASK-M8-117 builds and asserts.
- **Options seen** (each settles all three of REQ-CHART-115's centre reads; *added 8 Nov 2026, physics review
  5817348709 of PR #449, P3*):
  - (a) The test reads the pin: the anchor's stored state at the pin's pixel, constant on every frame; R-863's "centre"
    read as the lock's centre, the pin. The sliding anchor's follower is the pin's pixel, and the area factor is taken
    at the pin.
  - (b) The test reads the view's centre, as written: constant where the pin is at the centre (after "centre on pin", or
    a click at the centre); an off-centre lock is tested by R-866's pin staying fixed on screen instead. The slide and
    the area factor are tested at the centre on centred locks only, and an off-centre lock has neither tested.
  - (c) Both: the pin's state constant on every frame, and the centre's too where the pin is centred; the slide and the
    area factor at the pin, and also at the centre where the pin is centred.
  - (d) Another, stated by the owner.
- **Recommended:** (a), since R-866 puts the anchor at the pin and a based homotopy is based at the anchor. A
  recommendation, not applied.
- **Applied meanwhile:** nothing. REQ-CHART-115 and § 17.8 stand as written; TASK-M8-125 builds R-866 either way.
- **Waits:** TASK-M8-117's `path_lock_centre`, for what it reads off the centre (REQ-CHART-115 carries `rq: RQ-749`).

## RQ-750: R-865's bitwise condition can't hold under any full-circle link for latent `β`: migrate the stored `z`, carry the lower half in another coordinate, or make `z_β` periodic? *(a one-way door: what the instrument computes and every stored latent view; R-865 against `docs/design/principia_dd_decoder.md` § 3.2; physics review 5817348709 of PR #449, P1; context: TASK-M2-57, REQ-DEC-055, REQ-DEC-056, TASK-M8-126)*

*Filed 8 Nov 2026 by the fix round of the rulings PR that ports R-865 (PR #449), from its physics review. Nothing is
chosen: it is the owner's "bring the list back" case, found before the task starts.*

*In plain English: the owner ruled that the decoder stops folding mirror images, with the shape angle β running round
the whole circle, on condition that every existing preset, saved chart and golden image comes out bit for bit the same.
But the latent coordinate already maps every value onto half the circle, so any way of stretching it to the whole circle
moves every stored latent view. The condition fails on all of them before anything is built. The owner says which way
out to take.*

- **File, section:**
  - `decisions.md` § R-865, the owner's words: "remove the decoder's mirror step; latent beta runs over the full circle.
    Conditional on the bitwise check (every preset, saved chart and golden with beta in [0, pi] reproduces unchanged); if
    anything doesn't, bring the list back before proceeding. This also fixes the latent-chart traces jumping at
    syzygies."; and the owner's proposal of 7 Nov 2026 (RQ-739, item 1): "every existing preset, saved chart and golden
    test with beta in [0, pi] reproduces unchanged (expected, since the mirror step never fires there)".
  - `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi": the link
    `β = π·σ(z_β)`, which maps all of ℝ onto `(0, π)`.
- **The conflict:** the owner's premise holds of the mirror step: on canonical-frame decodes with `β ∈ [0, π]` step 3
  never fires (dd_decoder § 3.3; its no-op test, § 5.5), so removing the step alone changes no bit. It doesn't hold of
  the link "full circle" needs. Since `π·σ` already uses all of ℝ for `(0, π)`, no link reaching `(π, 2π)` agrees with it
  on every `z_β`: `2π·σ(z_β)`, a periodic `z_β`, or any other moves the decoded state of every stored latent `z`, so the
  check fails on each latent preset's `z₀`, each saved latent chart and each latent golden's image hash.
- **Traces at syzygies (the owner's "this also fixes"):** under a σ-type link only the syzygies at `β = π` become
  continuous; a syzygy at the link's seam, `β = 0` with `z_β → −∞`, still jumps on any chart showing `z_β`, unless
  `z_β` is made periodic (option (c)). *(This corrects RQ-739's physics note of 7 Nov, which had a crossing at `β = 0`
  wrapping under `β ∈ (0, 2π)`, and folds it here.)*
- **The question:** which way out, given that the check as worded fails on every latent item?
- **Options seen:**
  - (a) Migrate the stored `z` to the new link. That isn't "unchanged": IC words stay bitwise only where a migrated `z′`
    happens to land on the old `β`.
  - (b) Keep `β = π·σ(z_β)` and carry the lower half in another coordinate. Every stored `z` is unchanged, but it isn't
    "β runs over the full circle".
  - (c) Make `z_β` periodic. Every syzygy is continuous, but it changes the latent chart's premise of a convex `ℝ⁸`, and
    the stored `z` change as under (a).
  - (d) Another, stated by the owner.
- **Recommended:** none. The owner's words send this case back to them.
- **Applied meanwhile:** nothing. The decoder and its tests are unchanged; TASK-M2-57 records its baseline only once
  this is ruled.
- **Waits:** TASK-M2-57 (its link definition, REQ-DEC-056, and so the task; `rq: RQ-750` on REQ-DEC-055 and
  REQ-DEC-056), and through it TASK-M8-126's latent points.

## RQ-751: Two of `contain.sh check`'s assertions have no Job Object equivalent: does the Windows proof pass without them? *(a one-way door: what the containment proof claims, and so what may be placed on Windows; the owner's CI shape order of 8 Nov 2026, step 1, and its placement rule; `scripts/contain.sh` § `check`; REQ-VAL-181; context: the tooling PR "Windows containment for tests and mutants (Job Object)")*

*Filed 8 Nov 2026 by the implementer of that PR, which builds `.github/scripts/win_contain.ps1` and its dispatch-only
job, `win-gpu.yml`'s `win-contain`. Nothing is chosen.*

*In plain English: on Linux, CI's containment hides every outside process from the contained run, so a mutant that
signals "everything" can't reach the runner. Windows has nothing that hides processes: a Job Object (what the owner
asked for) kills the whole tree when it closes, caps processes and memory, and stops a process breaking out, but a
process inside can still see an outside process and kill it by its id. So the Windows proof can show every other
scenario of the Linux check, not these two. The owner says whether that is enough, and for what.*

- **File, section:**
  - The owner's CI shape order of 8 Nov 2026, step 1: "Windows containment: run tests and mutants inside a Job Object
    with kill-on-close, a process limit and an 8 GiB per-process memory cap (REQ-VAL-181's equivalent of prlimit). Prove
    it with the same kill test contain.sh check uses. Until proven, mutants of xtask/src/jobq.rs stay on GitHub-hosted
    runners." And its placement rule: "Containment, memory cap and the 36 h no-run alert are prerequisites for any job
    placed on Windows."
  - `scripts/contain.sh` § `check`, the probe run inside: `if kill -0 "$1" 2> /dev/null; then exit 42; fi` (the
    outside process must be unseen) and `kill -9 -1 2> /dev/null`, after which the outside process must be alive.
- **The conflict:** the Job Object the owner names gives kill-on-close, the process limit, the per-process commit cap,
  and no breakaway; `win_contain.ps1 check` proves each, and maps `kill -9 -1` to Windows' broadcast, CTRL+C to the
  whole console, which a console of the job's own keeps inside. It can't give a PID namespace: inside the job, the
  outside process is visible (contain.sh's 42) and, being the same user's, can be opened and terminated by its id. The
  check measures both and reports them as gaps, never killing anything; its exit status covers the Job Object's
  scenarios only. So "the same kill test" passes in full only with these two waived or met another way.
- **Context for the choice:** `xtask/src/jobq.rs`, the module whose mutants the order names, is `#[cfg(unix)]`
  (`xtask/src/lib.rs`), as are its tests (`xtask/tests/jobq.rs`): on Windows it isn't compiled, so its mutants can't be
  killed there whatever the containment, and the process-group signalling that made containment necessary on Linux
  (`kill(-1)`) has no Windows code path. Nothing in the code base terminates a process by an id it didn't start.
- **The question:** does the Windows proof count as passed, for placing jobs on Windows, with 42 and the kill by pid
  as known gaps?
- **Options seen:**
  - (a) Yes: the Job Object's scenarios are the proof; 42 and the kill by pid are recorded as gaps of the platform.
  - (b) Yes for jobs but not for `jobq.rs`'s mutants, which stay hosted for good (they can't run on Windows anyway).
  - (c) No: stronger isolation first (a separate restricted account or token, or a sandbox), which needs work at the PC
    and a design of its own.
- **Recommendation:** (b). A measurable alternative: (c), judged by the same check with 42 and the kill by pid turned
  into failing scenarios.
- **Applied meanwhile:** nothing is placed on Windows. The check is built and runs by `workflow_dispatch` only; its
  first run is the orchestrator's, once the runner accepts jobs.
- **Waits:** the switch of the slow tier and the mutation run to the Windows runner (the order's steps 2 to 5).
