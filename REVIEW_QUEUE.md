# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

## RQ-378: Whether "Save view as chart" on a curved chart should also offer a flattened oblique-latent version, where one exists locally *(the human's queued question, R-469 §11, R-475; context: TASK-M2-40, TASK-M8-48)*

*Queued by the human (R-469 §11: "Whether "Save view as chart" on a curved chart should also offer a flattened
oblique-latent version, where one exists locally."). For the human: yes, it changes what Save view as chart builds.*

- **File, section:**
  - `decisions.md` § "R-475 — …", the human's words, S2: on a curved chart the saved chart is "the parent chart, its
    slice centre, and the tilt (map, angles, targets)"; on a latent view, "`(z₀, q₁, q₂)` as an oblique chart".
  - `docs/gui/principia_render_gui_spec.md` § "G2. …", the Chart bullet's Save view as chart.
- **The question:** a curved view near its centre is approximated by its tangent plane, an oblique latent chart
  `(z₀, J q̂₁, J q̂₂)`; whether saving should also offer that flattened chart, and what "exists locally" requires (a
  full-rank Jacobian at the centre, the window small enough).
- **Options seen:** (a) no, the parent chart and its tilt only (S2 as written); (b) yes, a second option "Save as
  flat chart", offered where the decoder's Jacobian at the centre has rank 2 (it needs the derivative RQ-341 raises);
  (c) yes, with the flattened chart shown only as provenance, not offered.
- **Applied meanwhile:** (a). Nothing waits.
- **Parked:** R-510 (20 Oct 2026), the human: "RQ-378: parked until RQ-341." A deferral by ruling: the entry stays
  open, blocked on RQ-341, and returns to the human when RQ-341 is ruled; (a) stays applied and nothing waits.
- **21 Oct 2026:** RQ-341 is ruled (R-550), so R-510's parking has ended and the entry is with the human again. The
  human's batch of 21 Oct leaves it open (R-545); (a) stays applied and nothing waits.
- **21 Oct 2026 (evening):** RQ-341 is ruled (R-550), and the human parked RQ-378 again, with no target (R-568): "yeah
  ok we can park that for now then". The entry stays open, (a) stays applied, nothing waits, and it returns when he
  raises it. *Was:* "RQ-341 is ruled (the 21 Oct batch, PR #274)", before R-550 was on main.
- **25 Oct 2026 (R-625):** the human: "Parked (RQ-378, RQ-386, RQ-391): stay parked." The entry stays open and parked (R-510, R-568); what it applies stays applied, and nothing waits.
- **4 Nov 2026 (R-706):** option (a) applied, and deferred by owner 4 Nov 2026, with no target. The entry stays open.

## RQ-386: Whether tier 2 of the embedding should split further, the ramp window and colour mode above the overlays and lines, if measurement shows tier 2 itself often does not fit at small sizes *(the human's queued question, R-484 RT2; context: TASK-M7-44)*

*Queued by the human (the round-trip rule, "Queue, don't rule"). For the human: yes, it changes what a small image
carries.*

- **File, section:**
  - `decisions.md` § "R-484 — …", the human's words: "Whether tier 2 should split further, for example ramp window and
    colour mode above overlays and lines, if measurement shows tier 2 itself often does not fit at small sizes." and
    RT2's table: "**2. Colour** | `RenderState` as built-ins: colour mode enum and every parameter, the ramp window,
    overlays and the line system."
  - `docs/design/principia_dd_image_embedding.md` § "6. What travels": "**The ramp window is not optional.** An
    auto-ranged ramp manufactures or hides the difference it is meant to show".
  - The same, § "7. Measured capacity and behaviour", to be re-measured with the tiers (R-483 H6, TASK-M7-44).
- **Options seen:** (a) tier 2 whole, as RT2 has it; (b) tier 2a (the stain graph's built-ins, their parameters and the
  ramp window) above tier 2b (overlays, lines, the playhead and hover, `f3_fill`), 2b dropping first; (c) decided by
  TASK-M7-44's measurement: split only if tier 2 fails to fit at 64² or 128² for a typical stain.
- **Applied meanwhile:** (a). Nothing waits; TASK-M7-44's measurement reports tier 2's size so the human can rule with
  it.
- **Parked:** R-522 (20 Oct 2026), the human: "RQ-386 after the capacity re-measure." A deferral by ruling: the entry
  stays open, waiting on TASK-M7-44's capacity re-measure (REQ-TOOL-174), which reports tier 2's own size and, at
  64² and 128², whether tiers 1 and 2 fit; it then returns to the human with those numbers. Meanwhile (a), tier 2
  whole, stands, and nothing else waits.
- **25 Oct 2026 (R-625):** the human: "Parked (RQ-378, RQ-386, RQ-391): stay parked." The entry stays open and parked (R-522); what it applies stays applied, and nothing waits.
- **4 Nov 2026 (R-706):** deferred to TASK-M7-44's measurement by owner 4 Nov 2026. Option (c): split only if that measurement says so; (a) stands until then. The entry stays open.

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

## RQ-594: A side-ratio chart for Parischewsky et al.'s equilateral-triangle experiment, or a tested remap onto ShapeSphere at rest *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-323; context: TASK-M3-64)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any new chart a target
needs".*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-24 (was TASK-M3-64). This question
still decides what is built for them, never whether a milestone passes; it stays open.

- **File, section:** `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or
  data": PA-1, "Escaper-identity maps in side-ratio space around the Lagrange point", chart "new: SideRatio, or
  ShapeSphere at rest by a tested remap".
- **The question:** Parischewsky, Trani & Leigh 2023 map escaper identity over the triangle's side ratios around the
  equilateral (Lagrange) configuration, at rest. Is that a new chart, or the existing ShapeSphere chart at rest
  reached through a tested remap of side ratios to shape coordinates (rule 6)?
- **Options seen:** (a) a remap onto ShapeSphere at rest, no new chart; (b) a new SideRatio chart; (c) neither, PA-1
  waits.
- **Applied meanwhile:** (c); REQ-VAL-323 carries `rq: RQ-594`.
- **Waits:** TASK-M3-64 (and RQ-596 for its chaotic/regular split).
- **28 Oct 2026 (R-636):** stays open. The five-chart build, TASK-M2-36 (and TASK-M2-52 after it), is held until the
  owner rules on the replication batch, RQ-591 to RQ-598 (R-636: "Five-chart build (queue item d): hold until Malachy
  rules on the replication batch; do not start it."); flagged in R-636: those charts come from R-633's split, not from
  this batch.
- **28 Oct 2026 (R-637):** the owner: "Five-chart build: yes, I meant only the replication charts. This morning's
  five-chart split may go ahead." R-636's hold is lifted for TASK-M2-36 and TASK-M2-52. This entry's chart is a replication chart and is not built ahead of
  its ruling (R-594 item 5). Stays open.
- **4 Nov 2026 (R-706):** deferred to M9 by owner 4 Nov 2026. The entry stays open.

## RQ-619: The census's code questions the integrator contract doesn't settle: IAS15 as a drift reference, and the step-limit axis beyond the predictive limit *(R-637: "where the contract is silent, queue the question, don't infer from the prototype"; RQ-599's questions 8 and 9; `docs/incoming/2026-10-27_validation_plan/PROPOSAL_occupant_matrix_census_2026-10-27.md` § 3.1, § 3.2; `docs/contracts/principia_integrator_contract.md` Part 2b; context: none built)*

*Queued 29 Oct 2026 by R-637's port. For the owner: yes, R-637 asks for it, and it decides what the census harness
builds.*

- **File, section** (RQ-599's questions, now in `docs/archive/review_queue/M0.md`, verbatim):
  - **8, IAS15:** "§ 3.2 calls IAS15 "already the drift reference in the logH comparison". The only record of it in the
    repo says "**IAS15 is not built.** … its only role here is a reference arm"
    (`docs/reference/prin-rs/docs/NOTES_excerpts.md` :119–121). Is IAS15 to be built for the census, or is that clause
    to be corrected?"
  - **9, the step-limit axis:** "prin-rs's `StepLimit` (`None`, `Reject`, `Predictive`, `AbGrowth`, `Global`) is AZ's
    alone …; Heggie and logH carry only the predictive limit, through `step_limit_f` …, and the contract fixes the
    predictive limit at f = 0.02 (Part 2b, R-160). § 3.1 crosses the step limit "fully" and omits `Reject`. Is the
    step-limit axis AZ-only beyond `None`/`Predictive`, and is `Reject` a value or a control?"
  - The contract, Part 2b: "**The predictive step limit:** `dτ ≤ f·d_min / (|v_rel|_max·A·B)`, with f = 0.02 (R-160)."
    It names no IAS15, no drift reference for a census, and no step limit but that one.
- **The questions:** (8) whether the census builds IAS15, uses a reference the corpus has, or corrects § 3.2's clause;
  (9) whether the census sweeps a step limit beyond the contract's one, and if so which values, and whether `Reject` is
  a value or a control.
- **Options seen:** for 8, (a) build IAS15 for the census, (b) take the drift reference from what the corpus already
  has (R-33's convergence reference; the inspector's RK45, contract Part 2), correcting § 3.2's clause, (c) drop the
  drift reference from the census; for 9, (a) no step-limit axis: every cell runs the contract's predictive limit at
  f = 0.02, (b) a harness-only axis of the contract's limit and none (`None`), `Reject` a control, (c) another set,
  stated by the owner.
- **Applied meanwhile:** none. Nothing of the census is built.
- **Waits:** the census harness (with RQ-617).
- **4 Nov 2026 (R-706):** deferred to M9 by owner 4 Nov 2026. The entry stays open.

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

## RQ-675: A margin node's two field ins: render_gui_spec § 5 puts them on a derived source, § 3.1 gives a source no input and a brightness one *(a conflict inside the spec; REQ-GUI-024; render_gui_spec § 3.1, § 5, § 15 item 6; context: TASK-M7-12)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flight of TASK-M7-12. Nothing is chosen.*

- **File, section:**
  - `plan/requirements.yaml`:17743, REQ-GUI-024's verify: "a margin node has two field ins". `plan/tasks/M7/TASK-M7-12.md`:42
    names the same in `stain_fanout`.
  - `docs/gui/principia_render_gui_spec.md`:2050-2052, § "5. Sources & multi-source": "**Multi-source mappings expose
    multiple input ports** on the consuming node (e.g. a derived source taking two `field` ins for a `top1 − top2`
    margin, or a `gradient-magnitude` node)."
  - The same file, § "3.1 Node kinds", :2006 and :2008: `source` has inputs "—"; `brightness` has "one `field`". A
    `colour` takes "one or more `field`" (:2007) and a `post` "`vec3` (+ optional `field` ins)" (:2010).
  - The same file, § "15. Settled decisions (record)", item 6 (:2474-2475): "**multi-source = multiple input ports**, not
    multi-wire-per-port."
  - `plan/requirements.yaml`:14247, REQ-COL-015: "the derived operators gradient_magnitude via three evaluations at ±ε
    and topk_margin = d_1 − d_2".
- **Checked on `main`'s code:** `crates/render/src/assemble.rs` follows § 3.1: `declaration` (:557-585) gives a source
  and a combiner no input, a brightness at most one, a colour and a post up to `MAX_INPUTS` (4); `in_ports` (:596-605)
  gives `Kind::Source` none. No occupant under `crates/render/shaders/wgsl/frag/` is a margin (its files are `colour/`'s
  `outcome_state.wgsl`, the three combiners, `debug/` and `post/`'s `edge_line`, `fallback_tint`, `pending_hatch`).
- **The question:** where the margin's two field ins sit, so that `stain_fanout`'s "a margin node has two field ins" has
  a node to test.
- **Options seen:** (a) the margin is a `colour` (or `post`) occupant declaring two `// @input`s, as § 3.1 and § 15
  item 6 allow, and § 5's example is reworded; (b) a derived `source` takes two `field` ins, as § 5 says, and § 3.1's
  `source` row and the assembler change; (c) the margin is REQ-COL-015's `topk_margin`, a derived scalar field read as
  one `field`, and REQ-GUI-024's verify names another multi-input node.
- **Applied meanwhile:** nothing; `main`'s assembler stands.
- **Waits:** TASK-M7-12's `stain_fanout` arm (REQ-GUI-024).
- **4 Nov 2026 (R-706):** deferred to the start of TASK-M7-12 or TASK-M7-13 by owner 4 Nov 2026. Options with a recommendation are proposed then. The entry stays open.

## RQ-676: TASK-M7-12's deliverables against the built assembler: a second walk, an edit-sequence proptest that cannot live in `render`, and a fallback distinct from `#141418` *(a task file against `main`'s code and the corpus; REQ-RENDER-010, REQ-RENDER-072, REQ-RENDER-081; render_gui_spec § 13; R-467; systems_architecture § 7.1; context: TASK-M7-12)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flight of TASK-M7-12. Nothing is chosen.*

- **File, section:**
  - `plan/tasks/M7/TASK-M7-12.md`:33-34, Deliverables: "`crates/render/src/codegen/walk.rs` — graph → `shade()` in
    backbone order, the post chain in declaration order with insert and reorder, the 8-node bound." and
    "`crates/render/src/codegen/fallback.rs` — identity for dangling / absent inputs, the defined fallback shader for an
    invalid graph."
  - The same, :40: "`cargo test -p render stain_always_renderable` (proptest) — fuzz random graph edit sequences: every
    state renders; forced-invalid graphs render the fallback (REQ-RENDER-072)."
  - The same, :44: "Proposal: the invalid-graph fallback (flat grey or error tint) and its sRGB value, distinguishable
    from both-None mid-grey and from the invalid-pixel pattern (REQ-COL-055's hatch, R-132)". REQ-RENDER-081's verify
    (`plan/requirements.yaml`) names the same two.
  - `plan/requirements.yaml`, REQ-RENDER-010's verify: "built-in occupants are loaded through the same assembler entry
    point as a user custom; no second compile path exists".
  - `crates/engine/src/contract/stain/graph.rs`:15-16: "The port rules, the occupants' declarations and the lowering
    are the render crate's, the fragment side's (systems_architecture §7.1: `engine` depends on `render`, never the
    reverse)." `StainGraph` and its edits are `engine`'s (`crates/engine/src/contract/stain/`); `crates/render/Cargo.toml`
    has no `engine` dependency.
  - `docs/gui/principia_render_gui_spec.md`:2449-2451, § "13. Invariants": "by default the hatched invalid pattern
    (R-132) in a debug view and `#141418` in a presentation view, which draws no hatch (R-467)".
- **Checked on `main`'s code:** `crates/render/src/assemble.rs` already walks the graph in backbone order (`shade(ctx)`,
  :1154ff), bounds the live post chain at `MAX_POSTS` = 8 (:120, :697-699) and builds the post chain (:733ff), its git
  history TASK-M1-04's, M1-07's, M1-08's, M1-09's, M1-10's, M1-13's, M2-32's, M2-34's, M7-02's, M7-04's, M7-05's and
  M7-08's; `crates/render/src/codegen/combine.rs` gives None's identity, both-None the
  flat mid-grey (TASK-M7-04). A `codegen/walk.rs` beside them would be a second path to `shade()`.
- **The gaps:** (1) the deliverables name new files for what `assemble.rs` and `combine.rs` hold; (2) a proptest over
  `StainGraph` edit sequences cannot be a `-p render` test, `render` not depending on `engine`; (3) the fallback's
  distinctness list omits the presentation view's `#141418`.
- **Options seen:** (a) re-scope the deliverables to extend `assemble.rs` and `codegen/` (fallback and the remaining
  wiring), no second walk; (b) move the edit-sequence proptest to `cargo test -p engine`, a plan edit of the acceptance
  line and REQ-RENDER-072's crate, or keep it in `render` over `assemble::Stain` values rather than `StainGraph` edits;
  (c) add `#141418` to the fallback's distinctness list in TASK-M7-12 and REQ-RENDER-081's verify. Each may be taken
  alone.
- **Applied meanwhile:** nothing; the task file stands as written.
- **Waits:** TASK-M7-12.
- **4 Nov 2026 (R-706):** deferred to the start of TASK-M7-12 or TASK-M7-13 by owner 4 Nov 2026. Options with a recommendation are proposed then. The entry stays open.

## RQ-677: A slot occupant's default `inputDomains` when no field is wired, and for an unbounded field; and TASK-M7-13's deliverables against the built registry *(a definition requirement, R-72; R-53; REQ-GEN-019; gui_state_contract § 3; context: TASK-M7-13)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flight of TASK-M7-13. Nothing is chosen.*

- **File, section:**
  - `docs/contracts/principia_gui_state_contract.md`:243, § "3. The registry is the scanned filesystem …": "**`inputDomains`
    (R-53):** each node declares the domain of each input it maps from; by default it inherits the manifest's per-field
    domain (the fixed `[lo, hi]` of `range_norm`, render_gui_spec §10.1), and a node that transforms its field declares
    its own."
  - `plan/requirements.yaml`, REQ-GEN-019's verify: "scan the registry: every entry has inputDomains; an untransformed
    node's domain equals the manifest [lo, hi] of its field". `plan/tasks/M7/TASK-M7-13.md`:29 repeats it.
  - `crates/ledger/src/schema.rs`:100-102: "(−∞, ∞): the bounds an entry writes where its range is a gap or provisional
    (dd_generation_root §3.8; R-426) …", `pub const UNBOUNDED: Range`.
  - `crates/ledger/src/gen/numeric.rs`:318-324: "`u_range`'s default: the fixed `(lo, hi)` where both are finite, else
    `(0, 1)`, `range_norm`'s unit interval".
  - `plan/tasks/M7/TASK-M7-13.md`:22-23, Deliverables: "`crates/render/src/registry/scan.rs` — build-time scan (build.rs)
    plus runtime registration for customs …" and "`crates/render/src/registry/entry.rs` — the six-field entry, slot
    validation, inputDomains defaulting from the manifest `[lo, hi]` of `range_norm`."
- **Checked on `main`'s code:** `crates/render/src/registry.rs` (TASK-M1-08, TASK-M1-12) already scans `frag/`, keeps
  the six fields and reads `inputDomains` from each file's `// @input` lines; `registry()` (:146-148) scans at run time
  from `env!("CARGO_MANIFEST_DIR")`. There is no `crates/render/build.rs`. A colour or brightness that declares no input
  gets the default `field` with `domain: None` (`crates/render/src/assemble.rs`, `default_input`).
- **The gap:** an occupant in the registry is not yet wired, so it has no field whose `[lo, hi]` it could inherit at
  scan time; and a field whose range is `Range::UNBOUNDED` has no finite `[lo, hi]` (the generator falls back to
  `(0, 1)`). The corpus defines neither case.
- **Options seen:** (a) the registry records "inherits" for an untransformed input, the domain resolved per wire at
  assembly from the wired field's manifest range, REQ-GEN-019's verify read on a wired node; (b) as (a), with an
  unbounded field's domain `range_norm`'s `(0, 1)`, as the generator's fallback; (c) another definition, stated by the
  owner. On the deliverables: (d) re-scope TASK-M7-13 to extend `registry.rs` and its runtime scan, or (e) keep the
  build-time scan and `build.rs` as written.
- **Applied meanwhile:** nothing; `main`'s registry stands.
- **Waits:** TASK-M7-13.
- **4 Nov 2026 (R-706):** deferred to the start of TASK-M7-12 or TASK-M7-13 by owner 4 Nov 2026. Options with a recommendation are proposed then. The entry stays open.

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

## RQ-680: TASK-M7-52's target charts: SM1's table has `θ × dilation` and `ν × dilation`, which no app chart has *(a definition requirement, R-72; R-542; state_map SM1; REQ-COL-113, REQ-COL-115, REQ-COL-129, REQ-COL-134; context: TASK-M7-52, TASK-M7-47)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flight of TASK-M7-52. Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_state_map.md`:147-148, § "SM1. Definition", the target table: "| `θ × dilation` | `(θ, h)`, `h`
    the scale-free homologous rate `(Σ m r·v)/I · I^¾` | — (never left) |" and "| `ν × dilation` | `(ν, h)`, `ν` the
    nearest point of the right-triangle curve (mirror folded) | past a bounded edge when `ν ∉ [1/32, 31/32]` …".
    The same section, :133-135: "*Transcribed from the prototype (R-542; `smWhere`, `smTarget`).* The prototype's targets
    are its charts".
  - `plan/requirements.yaml`:16609, REQ-COL-129: "θ × h 7e-8 … ν × h 4.9e-4"; REQ-COL-115's verify (:16366) and
    REQ-COL-113's (:16330), TASK-M7-47's `chart_position_identity` and `chart_position_status`, name the same targets.
  - `plan/requirements.yaml`, REQ-COL-134's verify: "over seeded states per target", naming no targets.
- **Checked:** "dilation" appears in neither `docs/design/principia_chart_reference.md` nor
  `docs/gui/principia_render_gui_spec.md` (§ G7, the Chart builder, included), nor under `crates/kernel/src`. The app's
  charts encode through `crates/kernel/src/encode/by_kind.rs`'s `encode_into` (:210), and a curve axis's nearest point is
  `crates/kernel/src/encode/curve.rs`'s `nearest` (:242). The corpus does name the dilation rate as an axis quantity:
  `decisions.md`:18628, the traces' leave table, "A physical quantity (`E`, `L_z`, `K`, a momentum, the dilation rate)",
  and `docs/design/principia_lines.md`:333, § "8. Traces", "a physical quantity (`E`, `L_z`, `K`, a momentum, the
  dilation rate)".
- **The gap:** which charts are "each target chart" for the one projection: SM1's targets are the prototype's, two of
  them with an axis the app does not define.
- **Options seen:** (a) the targets are the app's charts, through `encode_into` and `curve::nearest`, SM1's dilation
  rows and REQ-COL-113's, -115's and -129's θ × h and ν × h read as the prototype's record. (a) leaves REQ-COL-129's
  `θ × h` and `ν × h` tolerances, and TASK-M7-47's arms on them, without a target, so it drops requirement content and
  is the owner's call; (b) define the two dilation charts (their `h` axis, range and link) and build them as targets;
  (c) another, stated by the owner.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M7-52 (and TASK-M7-47's `θ × h` and `ν × h` arms through it).
- **4 Nov 2026 (R-706):** deferred to M9 by owner 4 Nov 2026. The entry stays open.

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
- **4 Nov 2026 (R-696):** option (b) ruled: a sinc form at `shape_sphere.rs`:162 that never reads the backend's `sin` near 0, gated to f32 (REQ-DEC-053, TASK-M2-53). Option (a) deferred to M4 by owner 4 Nov 2026, decided on the cross-backend check; the entry stays open for it.

## RQ-688: `stand-in-soak` is red on `main`: three `qa_m022_*` tests reach the 600 s timer on run 1 of 50, and the cause is not known *(silence: a failed verify with no cause; REQ-SYS-070, R-267; REQ-VAL-156, R-376; R-649; context: run 41781905241, PR #394)*

*Queued 4 Nov 2026 by the orchestrator's filing task, from run 41781905241. This entry asks for the cause, not a timer
change: no timer was touched (R-649). Nothing is chosen.*

- **File, section:**
  - `plan/requirements.yaml`, REQ-SYS-070's verify: "qa_TASK-M0-22.rs's stand-in-cargo tests pass in 50 consecutive
    parallel runs on ubuntu-latest, with the run log kept on the PR".
  - `.github/workflows/stand-in-soak.yml`: 50 runs of `cargo test -p xtask --test qa_TASK-M0-22 --test qa_TASK-M0-22_r235
    --test ci --test qa_TASK-M0-36 --test qa_TASK-M0-37`, stopping at the first failure, on a pull request that changes
    the listed paths and on `workflow_dispatch`, never on a push.
  - `crates/validation/src/spawn.rs`:20-22: `SPAWN_TIMEOUT`, 600 s, REQ-VAL-156, confirmed by R-376.
- **Measured:**
  - Run 41781905241, `workflow_dispatch` on `main` at `e1df88fa`, job 126415145558, step "50 consecutive parallel runs":
    `run 1 of 50 failed`. In `qa_TASK-M0-22`, 1 passed and 3 failed, finished in 600.09 s. The three:
    `qa_m022_ci_fails_on_a_control_that_leaves_its_test_passing`, `qa_m022_ci_fails_on_a_test_without_control` and
    `qa_m022_ci_passes_when_every_control_trips`. Each panics at `xtask/tests/qa_TASK-M0-22.rs`:67: "child
    `…/target/debug/xtask ci` (pid 6193) was killed: it outlived the 600 s timeout" (pids 6193, 6200, 6196).
  - The failure is a timeout. "Text file busy", REQ-SYS-070's own subject, is nowhere in the log. The `ci` test binary in
    the same run passed, 5 of 5 in 88.03 s.
  - The same three tests fail the same way, on run 1, at 600.08 to 600.09 s, on PR #394's heads: runs 41769443878
    (`5e867800`), 41781911548 (`bd2b4c12`) and 41794663752 (`ef6ad489`). `e1df88fa`'s workflow has no step
    "Dependencies without debug info", which those heads add, so that step is not the cause.
  - The last green run, 41101800019 at `ffbd3f28`, 30 Oct: 50 of 50, on the same four tests 544.52 s on run 1 and 249.00
    to 253.57 s on the other 49 (median 250.53 s). So run 1 was already within 56 s of the timer.
  - Under nextest, one at a time, the three pass on `main` at `e1df88fa` (run 41744485022): 95.051 s, 95.446 s and 163.929 s.
- **Later runs:** there is no later `stand-in-soak` run on `main`, because the workflow doesn't run on a push. `ci` on
  `main` is green at `e1df88fa`, `39a766c9`, `d0062a5c` and `99881a7b`. Run 41821036802 at `437d281d` hadn't finished at 12:12 UTC on 4 Nov: 12 jobs passed, none failed, 7 not finished. PR
  #394's last run, 41808766194 at `b5bf2f67` (`main` merged in, with #395 and #403), started at 09:35 and was still in
  progress at 12:12 UTC, in step "50 consecutive parallel runs" since 09:37:32, 2 h 34 min on, where the failed runs took
  14 to 17 minutes (13 min 32 s to 17 min 07 s by the runs API). That suggests it passed run 1. Its log can't be read until
  it ends, so this isn't known. PR #394 merged meanwhile.
- **Read from the code, not measured:** of `xtask ci`'s runners, those that call `cargo` go to the test's stand-in, which
  answers from canned text; only a call naming `golden_harness` reaches the real cargo
  (`xtask/tests/qa_TASK-M0-22.rs`:45-47; `xtask/src/golden.rs`:1226-1234), and the golden runner renders on lavapipe. The
  soak's build step doesn't build that binary, so on run 1 the three `xtask ci` children, side by side under libtest,
  would share its cold build. `.config/nextest.toml`:31-33 records the same contention in `ci` shards, and the group
  `xtask-cargo` that takes them one at a time there; the soak runs without it. This fits run 1's cost but doesn't explain
  what moved it past 600 s between 30 Oct and 4 Nov.
- **The question:** what made these tests go from 544.52 s to past 600 s on run 1, and is it still so on `437d281d`?
- **Would settle it:** run 41808766194's log when it ends; a `workflow_dispatch` of the soak on current `main`; the time of
  each `xtask ci` runner on a cold target, with the harness build apart.
- **Applied meanwhile:** nothing. No timer is touched.
- **Waits:** REQ-SYS-070's verify on `main`. A PR that changes the workflow's paths meets this run; no build task waits.
- **4 Nov 2026 (R-707):** ruled: "find the cause; timer untouched". The entry stays open until that work is done; it is in hand on another branch.
- **4 Nov 2026, diagnosis (the-locksmith agent), not yet proven:** not a code, build-profile or disk regression. Run 1 is
  the cold run, and it fails on the slowest runner band (step "List" at 85 s or more). 5 of 6 soak runs on 4 Nov were red.
  - **Results:** run 41808766194 (`b5bf2f67` on `99881a7b`) succeeded 50 of 50, run 1 451.04 s, warm median 260.2 s. Run
    41821036802 (`ci` at `437d281d`) succeeded. New red run 41845250868 (job 126626947510, `9133ef1b` on `5ef0124a`):
    three tests at 600.08 s on run 1. Correction: `ci` at `5ef0124a`, run 41830399926, succeeded.
  - **Read from the runs:** three parallel `xtask ci` children under plain `cargo test`, each 600 s; nextest's `xtask-cargo`
    group does not apply. The workload grew: warm M0-22 187, then 250, then 260 s.
  - **Hypotheses, none proven:** H1, the cargo build lock on the cold `golden_harness`; H2, llvmpipe's shader compile, cold;
    H3, the bands are different hardware. The slow band's growth past 600 s is not explained by the logged phases.
  - **Fix proposed:** a serial warm-up after the build in `stand-in-soak.yml` (`xtask golden --all`), with the same
    environment so the fingerprint holds; the timer and the 50 parallel runs unchanged. `qa_TASK-M0-42` and
    `qa_TASK-M0-14_rust_gpu_cache` read the workflow.
  - **The entry stays open:** the ruling was "find the cause; timer untouched", and the cause is proven only when a run
    with List at 85 s or more passes 50 of 50.

## RQ-689: `jobq_a_finished_jobs_group_is_killed`'s control races perl's `setpgrp` against the runner's group kill *(a flaky control; REQ-VAL-147; R-658; context: `xtask/tests/jobq.rs` `finished_group_case`, run 41785718756, PR #394)*

*Queued 4 Nov 2026 by the orchestrator's filing task. Nothing is chosen.*

- **File, section:** `xtask/tests/jobq.rs`:572-591, `finished_group_case`, with `escape = true`:
  `perl -e 'setpgrp(0,0); exec @ARGV' sleep 60 & echo $! > child.pid; exit 0`. The control at :599-604 expects "a
  finished job's group outlived it". The test's queue sets `tick_ms = 50` (:45). `xtask/src/jobq.rs`:1258-1260, in
  `Runner::pass`: `if running.reaped.is_none() && leader_exited(running.child.id()) { … signal_group(pgid,
  rustix::process::Signal::KILL);`.
- **What is racy, from the code:** the script writes the pid and exits at once, and nothing makes it wait for perl. The
  background perl is in the job's process group until it reaches `setpgrp(0,0)`, so that call races perl's own start-up
  against the next pass, at most about 50 ms after the leader exits. If the pass wins, the SIGKILL reaches perl in the
  group, `alive(child)` is false, the assertion holds, and the control doesn't panic, so the case it means to show never
  arises. `xtask controls` then reports that the test has no working control. The race is in the control only: without
  `escape`, `sleep 60` never leaves the group and dies either way.
- **Evidence it fails:** one failure found. Run 41785718756 (`ci`, PR #394, `4e6e455c`, attempt 1), `xtask-ci (2)`, job
  126429139482: "xtask: test `jobq_a_finished_jobs_group_is_killed`: its control did not make it fail, so the test may not
  be able to (philosophy §4.4)"; "test did not panic as expected at crates/validation/src/control.rs:94:16"; "xtask ci:
  [3/10] controls FAILED: 1 test(s) without a control that makes them fail (REQ-VAL-147)". PR #394 didn't touch this
  code. That the race is what fired there is read from the code: the log doesn't show perl's timing.
- **Searched:** the last 25 failed `ci` runs, back to 1 Nov 21:22 UTC, show that one run only, and the 1431 local queue
  logs show none. So the rate is low, and it is not measured.
- **The gap:** the control can fail with nothing wrong, on any PR whose `xtask-ci` shard holds it.
- **Options seen:** (a) the job's script waits, with a bound like `GATE`'s, until the child has left the group (a file
  perl writes after `setpgrp`, or the child's `pgid` read back), then exits, so the kill always finds the case; a test-only
  change; (b) another, stated by a reviewer or the owner.
- **Applied meanwhile:** nothing.
- **Waits:** nothing.
- **4 Nov 2026 (R-707):** ruled: option (a), "you fix (test-only)". The entry stays open until that work is done; it is in hand on another branch.

## RQ-693: What the `(L_z, E)` chart's `E` axis means at `ε ≠ 0`: R-670's refusal covers `Some(E*)`, and this chart has no `E*` *(physics; silence; R-670, RQ-656 (6); `docs/design/principia_chart_reference.md` § 2.1, § 2.2; `docs/design/principia_dd_decoder.md` § 3.6, § 3.7; context: TASK-M9-39, REQ-VAL-015)*

*Queued 4 Nov 2026 from physics review 5771183527 of PR #408, F1. For the owner: yes, it changes results: which ICs such a chart's pixels decode to, or whether the chart runs softened at all. Nothing is chosen.*

- **File, section, quoted:**
  - `decisions.md` § "R-670 — …", the owner's words: "(6)(iii)"; its reading: "A view refuses `ε ≠ 0` with any `Some(E*)`, as a forbidding chart refuses any `Some(E*)` (dd_decoder § 3.7, R-25): `η_E` never runs on a softened decode, so a pixel's IC never depends on `ε`".
  - `docs/design/principia_dd_decoder.md` § 3.7: "**Under softening (R-670, RQ-656 (6)(iii), 2 Nov 2026).** A view refuses `ε ≠ 0` with any `Some(E*)`, `Some(0)` included, as a forbidding chart refuses it below: `η_E` never runs on a softened decode, so a pixel's IC does not depend on `ε`".
  - `docs/design/principia_dd_decoder.md` § 3.7: "**It must be disabled** on the invariant-momentum charts $(L_z, E)$ and $(L_z, K)$, where energy is a chart coordinate or is fixed by the momentum construction."
  - `docs/design/principia_chart_reference.md` § 2.1: "`E = U + K` with `K ≥ 0`, so `E ≥ U`", "`|Lz| ≤ √(2·I·K) = √(2·I·(E − U))`", "The feasible region is the interior of a parabola with apex at `(Lz, E) = (0, U)` — the rest start.", and the warp's "`E(t)     = U + K(t)`".
  - `docs/design/principia_chart_reference.md` § 2.2, "(iii) Mix to the target kinetic energy": "`K* = E − U          (or K(t) directly on the (Lz,K) chart)`".
  - `docs/design/principia_dd_decoder.md` § 3.7: "$U = V_0$ and $K_0$ are §3.6's, of the decoded $(m, r, p)$"; § 3.6, dated 2 Nov 2026: "At `ε ≠ 0`, `V₀ = − Σ_{i<j} mᵢmⱼ / √(‖rᵢ − rⱼ‖² + ε²)`".
  - `plan/requirements.yaml`, REQ-VAL-015, verify: "on an (L_z,E) chart, E_0 is constant along the L_z axis and monotone along E to within f32 rounding of the target".
- **The silence:** RQ-656 (6) asked about `η_E`, and R-670 rules that. A `(L_z, E)` chart forbids `Some(E*)`, so no energy target reaches it; its energy is the axis, and its `U` is the fixed configuration's potential. § 3.7 reads `U` as `V₀`, and R-670 softens `V₀` at `ε ≠ 0`. Nothing says which `U` the chart's axis, feasible set and `K*` read at `ε ≠ 0`, and R-670's "a pixel's IC never depends on `ε`" is stated of the refusal, which does not cover this chart. The `(L_z, K)` chart has `K* = K(t)` and reads no `U` in its `K*`.
- **The question:** at `ε ≠ 0`, what does the `(L_z, E)` chart's `E` axis read, and does the chart run?
- **Options seen:** (a) `U` softened: the axis is the run's softened `E₀`, and REQ-VAL-015 holds as written, but the IC and the feasible set then depend on `ε`, against R-670's stated reason; (b) `U` Newtonian: the IC does not depend on `ε`, but the run's softened `E₀` is not the axis value, the region under the parabola no longer means what § 2.1 says, and REQ-VAL-015 needs restating; (c) the chart refuses `ε ≠ 0`, at the view level, as with `Some(E*)`, which nothing says; (d) another, stated by the owner.
- **Applied meanwhile:** none needed: dispatch refuses `ε ≠ 0` (R-654; TASK-M3-06), so no softened run exists.
- **Waits:** TASK-M9-39 (REQ-INT-111, REQ-SYS-094, REQ-SYS-095). Nothing in M0 to M8 waits.

## RQ-694: What Chazy's `pair_kup` and `out_kup` read at `ε ≠ 0`: R-670 softens the classifier's `E` and leaves the `K/U` its labels read *(physics; silence; R-670, RQ-656 (3); `docs/design/principia_dd_chazy_final_motion.md` § 3.1, § 3.2; context: TASK-M9-39, TASK-M7-35)*

*Queued 4 Nov 2026 from physics review 5771183527 of PR #408, F2, and code review 5771184172, C2. For the owner: yes, it changes results: which label a softened run commits. Nothing is chosen.*

- **File, section, quoted:**
  - `decisions.md` § "R-670 — …", the owner's words: "656 (a)", option (a) of RQ-656 (1) to (3): "Chazy's `E` softened as the ruling's energy."
  - `docs/design/principia_dd_chazy_final_motion.md` § 3.1: "*2 Nov 2026 (R-670, RQ-656 (3), option (a)):* at `ε ≠ 0`, `E` is the run's `E_0` under the softened potential (R-654); "unsoftened" is the `ε = 0` case."
  - `docs/design/principia_dd_chazy_final_motion.md` § 3.1: "**A pair `(i, j)`'s two-body energies** (`pair_kup`): reduced mass `μ = m_i m_j / (m_i + m_j)`, `Δr = r_j − r_i`, `Δv = v_j − v_i`, `d = |Δr|`; kinetic `K = ½ μ |Δv|²`, potential magnitude `U = m_i m_j / d` (positive)".
  - `docs/design/principia_dd_chazy_final_motion.md` § 3.1: "**The outer body `b`'s two-body energies** (`out_kup`), against the centre of mass of the other two, `(i, j)`: … `μ = m_b M_p / (m_b + M_p)`, `K = ½ μ |Δv|²`, `U = m_b M_p / d`, `v_r = Δr·Δv / d`."
  - `docs/design/principia_dd_chazy_final_motion.md` § 3.2: "**H** | every pair separation > `h_far · R_0`, every pair receding, every pair `K/U > 1 + ε`", with HP, P, HE and PE reading `K/U` too, and all five committed. That table's `ε` is the parabolic band's (§ 3.1), not softening's.
- **The silence:** RQ-656 (3) asked of `E` only, and R-670 softens `E`. Nothing rules the pair and outer two-body energies. Once the refusal is lifted the classifier runs on every softened run, with a softened `E` beside a Newtonian `U`, unless a ruling says otherwise, and the committed labels can change. A softened `out_kup` is not a term of the dynamics' `V`, since `b`'s potential there is pairwise, about each body and not about the pair's centre of mass; R-670 (4) reads `E_rel`, which has the same shape, as `d` read as `√(d² + ε²)` in the potential term.
- **The question:** at `ε ≠ 0`, what do `pair_kup`'s and `out_kup`'s `U` read, and do the committed labels stand on a softened run?
- **Options seen:** (a) both soften, `U = m_i m_j / √(d² + ε²)` and `U = m_b M_p / √(d² + ε²)`, as R-670 (4) reads `E_rel`, the separations, `v_r` and the kinetic terms geometric; (b) both stay Newtonian as written, `E` alone softened; (c) the classifier's two-body labels do not run at `ε ≠ 0`, the run reading only the labels that take `E`; (d) another, stated by the owner.
- **Applied meanwhile:** none needed: dispatch refuses `ε ≠ 0` (R-654; TASK-M3-06), so no softened run exists.
- **Waits:** TASK-M9-39 (REQ-INT-111, REQ-SYS-094, REQ-SYS-095). Nothing in M0 to M8 waits.

## RQ-696: Whether BR-1 runs from the authors' own initial conditions stand in for R-638's seeded draw *(the part of RQ-615 R-701 does not reach; R-638; `docs/incoming/2026-10-28_rq598_data/README.md` § 4 task 4; context: TASK-M9-22)*

*Queued 4 Nov 2026 by R-701's port. For the owner: yes, it changes which ensemble BR-1 is scored on, and the corpus does not settle it (R-369).*

- **File, section:**
  - `docs/archive/review_queue/M0.md`, RQ-615's note of 28 Oct 2026: "**Still open:** whether runs from the authors' initial conditions stand in for the seeded draw, which R-638 asks for ("BR-1 Brutus ensemble on Breen's stated set-up now, fixed recorded seed")".
  - `docs/incoming/2026-10-28_rq598_data/README.md` § 4 task 4: "Draw the initial positions from a **fixed, recorded seed**."
  - `decisions.md` § R-701, the owner's words: "RQ-615: (a) local only, plus (c) ask Breen when he replies."
- **The silence:** RQ-615's options (a) and (c) are about the files: how they are held, and asking for a licence. Neither says whether an ensemble run from their starting points replaces the seeded draw.
- **Options seen:** (a) runs from the authors' starts stand in, for the 180 they cover; (b) R-638's seeded draw stays required, and the pilot stays a pilot; (c) both, each labelled.
- **Applied meanwhile:** R-638 stands, and the seeded draw is not run.
- **Waits:** a BR-1 ensemble beyond the 12-run pilot.

## RQ-699: What "tilted in their warp coordinate" means for the held `L_z` of `(ν, K)`, `Φ_θ,K` and `Φ_strip,K` *(physics; silence: R-708; R-471 E1; R-633 c2; REQ-CHART-062; context: TASK-M2-52, PR #388)*

*Queued 4 Nov 2026 by R-708's port. For the owner: yes, it is physics; the answer changes which systems a tilt into the
held `L_z` shows. R-708's last line says to come back with such a case. Nothing is chosen.*

- **File, section:**
  - `decisions.md` § "R-708 — …", the owner's words: "Held energy-type hidden coordinates (R-633 c2) are tilted in their
    warp coordinate in this mode, so held K stays ≥ 0."
  - `decisions.md` § "R-633 — …", the owner's (c) c2: "the (ν, K) chart and Φ_θ,K gain the held L_z, and Φ_θ,L_z gains
    the held K, as physical hidden coordinates."
  - `decisions.md` § "R-471 — …", the heading: "Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`)".
  - `docs/design/principia_chart_reference.md` § 2.1: "`L_z = (2s − 1)·L_max` with `L_max = √(2·I·K)` at the kinetic
    energy the chart holds fixed (as §4.5's `Φ_θ,L_z`)."
- **The silence:** `L_z` is energy-type, and the held `L_z` is one of R-633 c2's two held coordinates, so the sentence
  reaches it. Its reason, "so held K stays ≥ 0", speaks only of `K`. `L_z`'s warp coordinate is `s`, but §2.1's line
  needs a `K` the chart holds fixed, and these charts hold none: `K` is the pixel's axis, and its floor reads the held
  `L_z`. So on these charts `s` has no `L_max` to scale by.
- **The question:** in warped mode, how does a tilt move the held `L_z` on these charts?
- **Options seen:** (a) by value, as R-633 wrote it: `L_z = L_z₀ + d`, the sentence reaching held `K` only. R-708's
  additive floor takes any real `L_z`, so every pixel stays a system; `|L_z|` has no bound but the slider's span. (b) In
  a warp coordinate on a fixed scale, such as `L_z = (2s − 1)·√(2·I·K_max)`, `s` bounded: `|L_z|` is then bounded, at the
  cost of a scale that §2.1 doesn't write for these charts, and `s` needs a rule past its edges. (c) Another, stated by
  the owner.
- **Applied meanwhile:** nothing. R-708's `K` floor, the "no refused cells" test and the linear arm are the same in
  every option; `d = 0` reproduces the untilted chart in every option.
- **Waits:** TASK-M2-52's warped tilt into the held `L_z`, and so PR #388, which R-708 holds until both arms are in.
