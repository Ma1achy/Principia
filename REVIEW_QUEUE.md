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

## RQ-583: Whether to build a Šuvakov-plane chart, or keep its targets as Path A closures until a chart question comes up *(physics; the human's queued question; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 9, question 4; PR #305; context: none built)*

*Queued by the human (27 Oct 2026). For the human: yes, it adds or does not add a chart.*

- **File, section:** [`docs/incoming/2026-10-27_validation_plan/PLAN.md` § 9](docs/incoming/2026-10-27_validation_plan/PLAN.md#9-open-questions-for-the-owner),
  question 4.
- **The question (verbatim):** "**Build a Šuvakov-plane chart, or keep its targets as Path A closures until a chart
  question comes up?**"
- **Recommended (the plan's):** "queue the chart question now; 3,109 orbits and the strongest share-back image wait on
  it."
- **Applied meanwhile:** the targets stay Path A closures. Related: RQ-582.
- **27 Oct 2026 (R-630):** the human: "3. Šuvakov plane labelling: the middle body as body 1 (β = 0, reached exactly, R-417), Hudomal eq. (18)'s labelling. Queue it as a chart question with the physics reviewer (§2.3). 4. Queue the Šuvakov-plane chart question now." RQ-582 is closed with that labelling, which this chart question now carries (physics: the chart's physics review checks it against Hudomal eq. (18) and PLAN.md § 2.3). Stays open for the human.
- **27 Oct 2026 (R-632):** the human: "RQ-583 with physics review". Unchanged (R-632); stays open for the human.

## RQ-591: Which replication targets gate their milestone and which are reported only *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5, § 10; REQ-VAL-201 to REQ-VAL-331; context: every task the port adds)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6 asks for it, batched, rather than
decided.*

- **File, section:**
  - `decisions.md` § "R-594 — …", item 6: "Queue for me, batched, rather than deciding: which targets gate a milestone
    and which are reported only".
  - `docs/design/principia_dd_replication.md` § "5. The targets": every new target's requirement, with its milestone.
  - `plan/MILESTONES.md`: a milestone's exit gate is every requirement whose `milestone` is that milestone
    (`plan/tools/milestones.py`).
- **The question:** R-629 made the seven first-batch drafts M3 requirements (REQ-VAL-201 to REQ-VAL-213). For the other
  targets — verification targets that must hold, and replication targets compared with another code's numbers, where
  a disagreement is a finding to queue (rule 4) — which must pass for their milestone's gate, and which are met by
  being run, recorded beside the published number and any disagreement queued?
- **Options seen:** (a) every target gates its milestone, as ported; (b) verification targets (layer V) gate, and
  replication targets (layer R) other than the first batch are reported only; (c) per target, as the human lists them;
  (d) exact targets gate, statistical and qualitative ones are reported.
- **Applied meanwhile:** as ported: each requirement sits in its milestone's list, as every requirement does, and the
  human passes each gate (R-369); a disagreement holds nothing until it is queued and ruled. Nothing is built
  differently either way: the choice decides only what a failing or disagreeing target does to its gate.
- **Waits:** nothing; it is ruled at the gates.

## RQ-592: Targets past the horizon cap: 667 atlas orbits with T > 200, and Boekholt et al. 2020's t = 10⁴ grid *(the human's queued question, R-594 item 6; R-610; `docs/design/principia_dd_replication.md` § 5.2, § 5.10, § 8.2 rule 18; REQ-VAL-201, REQ-VAL-218, REQ-VAL-222, REQ-VAL-295; context: TASK-M3-42, TASK-M3-43, TASK-M3-45, TASK-M3-62)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any target past the horizon
cap".*

- **File, section:**
  - `decisions.md` § "R-610 — …": "`T_horizon ∈ [0.4, 200]`, default 50".
  - `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas": "667 of the 3,916 rows with refined closure below
    1e-9 have T > 200, past R-610's cap" (measured on `docs/incoming/2026-10-26_orbit_catalogues/threebodyorbits_ics.csv`;
    the longest is T = 886.09; none of the 311 free-fall rows or the gallery's 301 is past 200).
  - The same doc, § "5.10 Converged-code references": Boekholt, Portegies Zwart & Valtonen 2020 converged their grid to
    t = 10⁴.
  - § "8.2 Learnt since", rule 18: "A target past 200 is compared only over the systems that end before 200".
- **The question:** R-610 caps the view's horizon at 200. A closure test integrates one period on the validation
  crate's CPU harness, not a view. May a closure test, or a converged-code comparison, run past 200 there, or do
  those targets stay inside the cap?
- **Options seen:** (a) keep the cap everywhere: the 667 long-period rows are not run, and GG-1 compares only systems
  that end before 200 (rule 18); (b) let the validation harness integrate past 200 for closure tests and converged-code
  comparisons only, the view's horizon unchanged; (c) (b) for closure tests only.
- **Applied meanwhile:** (a). OC-1, OC-3 and OC-6 skip and count the 667 rows; GG-1 also waits on RQ-598 for its data.
  The statistical targets whose systems outlive 200 (AA-2 to AA-8, AN-1, V-9, P-2, Q-1 to Q-3, CH-1, MT-1) count
  them as censored at 200, under rule 18.
- **Waits:** GG-1 (REQ-VAL-295, `rq`); the 667 rows of OC-1, which run once (b) or (c) is ruled.

## RQ-593: A binary-plus-single chart, for Trani et al.'s planar row, Manwadkar et al.'s escape probabilities and Stone & Leigh's closed forms *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-318, REQ-VAL-319, REQ-VAL-321; context: TASK-M3-63)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any new chart a target
needs".*

- **File, section:**
  - `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data": TR-1 (the
    inclination-0 row of Trani et al. 2024's map, "as a 1D scan in binary phase"), MK-1 (Manwadkar et al. 2021's
    per-body escape probabilities over eight mass sets) and SL-1 (Stone & Leigh 2019's planar closed forms), each with
    the chart "new: BinaryPlusSingle".
  - `docs/incoming/2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md` § "Sequencing", item 4: "the
    binary-plus-single chart (Trani's row, Manwadkar) … Each is queued as a chart question when its target comes up,
    not built ahead of it." MK-1's row: "their runs are 3D; a planar re-run, not a lookup".
- **The question:** build a chart whose axes are a binary's elements and a third body's approach (phase, impact
  parameter or eccentricity), so the three targets can run; and whether MK-1, whose source runs are 3D, is a
  replication at all when re-run in the plane.
- **Options seen:** (a) build it, as a new chart in the chart reference, by a later task; (b) build it as a Path B
  injector only (initial conditions from binary elements, no map), enough for MK-1 and SL-1 but not TR-1's scan as a
  chart; (c) do not build it: the three targets stay waiting, recorded as such; and, separately for MK-1, (i) keep it,
  stated as a different experiment, or (ii) drop it.
- **Applied meanwhile:** (c); no chart is built. The three requirements carry `rq: RQ-593`.
- **Waits:** TASK-M3-63.

## RQ-594: A side-ratio chart for Parischewsky et al.'s equilateral-triangle experiment, or a tested remap onto ShapeSphere at rest *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-323; context: TASK-M3-64)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any new chart a target
needs".*

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

## RQ-595: A syzygy-symbol field, a fixed-k velocity chart and Chernin et al.'s definitions, for the symbol-sequence and partition maps *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-325, REQ-VAL-326, REQ-VAL-327; context: TASK-M4-24)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any new chart a target
needs".*

- **File, section:** `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or
  data":
  - TS-1, maps of the first 2–4 syzygy symbols over the free-fall plane (Tanikawa & Mikkola 2008, 2015), which need a
    per-pixel syzygy-symbol field: PLAN.md § 4, "a syzygy-symbol field (none planned)";
  - TS-2, first-symbol maps at non-zero angular momentum, whose chart is "new: a fixed-k velocity prescription over
    D11 ∪ D12", the prescription "still to be read from the 2015 PASJ paper (now in the repo)";
  - V-1/V-3/V-7, the L/M/A/H partition and a transition-entropy map, "blocked: no task builds a symbol-sequence field,
    and Chernin et al.'s definitions are not in hand".
- **The question:** build a per-pixel field holding the first few syzygy symbols (and a transition entropy over
  them); a chart applying Tanikawa & Mikkola's fixed-k velocity prescription; and obtain Chernin et al.'s definitions.
- **Options seen:** (a) all three, by later tasks; (b) the symbol field only (TS-1 and, with the definitions, V-7), TS-2
  waiting; (c) none: the three targets wait.
- **Applied meanwhile:** (c); the three requirements carry `rq: RQ-595`.
- **Waits:** TASK-M4-24.

## RQ-596: The N_scramble and f_col diagnostics *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-318, REQ-VAL-321, REQ-VAL-323; context: TASK-M3-63, TASK-M3-64)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, each adds a diagnostic no task builds, as a new
chart does.*

- **File, section:** `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or
  data": SL-1 needs "the cut N_scramble ≥ 2"; PA-1's split "an N_scramble ≥ 4 classifier"; TR-1 "f_col (12 nearest
  neighbours, regular if > 2/3)" as a colouring. PLAN.md § 4: "N_scramble and f_col (PA-1, SL-1, TR-1)" are in no
  milestone.
- **The question:** build a scramble counter (the number of democratic resonance phases before escape) and Trani et
  al.'s f_col colouring, each defined from its paper, so the three targets can be compared.
- **Options seen:** (a) both, by later tasks, each a definition requirement (R-72); (b) N_scramble only; (c) neither:
  the targets' parts that need them wait.
- **Applied meanwhile:** (c).
- **Waits:** TASK-M3-63 and TASK-M3-64.

## RQ-597: Periodic-orbit continuation over the mass plane, for Li, Li & Liao 2020's stability map *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-329; context: TASK-M4-25)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, it adds a capability no task builds.*

- **File, section:** `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or
  data": LLL-1, "Period, angular momentum and stability contours over (m₁, m₂) for one periodic family … must reproduce
  Li, Li & Liao 2020's on the mass simplex", which needs "periodic-orbit continuation (not planned)" (PLAN.md § 3.2).
- **The question:** build a continuation of a periodic orbit across masses (a Newton–shooting solve per mass point),
  so the mass simplex can show one family's period and stability; or leave LLL-1 waiting.
- **Options seen:** (a) build it, by a later task; (b) no: LLL-1 waits, and the atlas's seven `li2021` rows stay closure
  tests (OC-1).
- **Applied meanwhile:** (b).
- **Waits:** TASK-M4-25.

## RQ-598: Data and papers not in the repo: Breen et al. 2020's Brutus runs, Boekholt et al. 2020's per-point lifetimes, and the primaries of the hierarchical stability limits *(the human's queued question, R-594 item 6 and rule 1; `docs/design/principia_dd_replication.md` § 5.10, § 5.12; REQ-VAL-295, REQ-VAL-297, REQ-VAL-330; context: TASK-M3-62, TASK-M3-65)*

*Queued 27 Oct 2026 by R-594's port. For the human: yes, it is a request for third-party data or papers, outside the
repo (R-369).*

- **File, section:** `docs/design/principia_dd_replication.md`:
  - § "5.10 Converged-code references": BR-1, Breen et al. 2020's per-trajectory Brutus runs to t = 10, "not
    released"; GG-1, Boekholt, Portegies Zwart & Valtonen 2020's 1,212 converged points, "figures only; per-point
    lifetimes not published".
  - § "5.12 Targets that wait for a chart, a diagnostic or data": V-11, hierarchical stability limits, whose primaries
    (Valtonen & Karttunen 2006; Mylläri et al. 2018) "are not in the repo".
  - § "8.1 Carried over from the 25 Oct instruction", rule 1: "The source is in the repo."
- **The question:** ask the authors for Breen et al.'s and Boekholt et al.'s data, and fetch the two primaries; or leave
  the three targets waiting. Data received would be committed only under stated terms, otherwise fetched and
  hash-checked (rule 17, as RQ-579).
- **Options seen:** (a) ask for both data sets and fetch both papers; (b) fetch the papers only; (c) none: the targets
  wait. The other fetches the plan names are RQ-584 (Hristov et al.'s free-fall database), RQ-585 (Li & Liao's website)
  and RQ-586 (Tanikawa & Mikkola 2021, Standish 1970, Yasko & Orlov 2015).
- **Applied meanwhile:** (c).
- **Waits:** TASK-M3-62 and TASK-M3-65.

## RQ-599: The occupant matrix census (DRAFT-VAL-08): its §11, five questions for the owner *(the human's queued questions on PR #314; `docs/incoming/2026-10-27_validation_plan/PROPOSAL_occupant_matrix_census_2026-10-27.md` § 11; R-71, R-159, R-561; RQ-577; R-630; context: none built)*

*Queued by the human (27 Oct 2026): "It is a record, not authority." and "Do not build anything: no harness, CI
workflow or runner registration until the ruling." For the human: yes, every question changes what is built (a
harness, CI jobs, a self-hosted runner). No option is recommended here.*

- **File, section:** `docs/incoming/2026-10-27_validation_plan/PROPOSAL_occupant_matrix_census_2026-10-27.md` (sha256
  `28894031…ccacb1f260`), § "11. For the owner to rule"; its axes in § 3, its stages in § 4, its readings and verdicts
  in § 5, its CI and runner plan in § 8. Its header: "**Status: proposed, not in force.** … Nothing here changes a
  default, a gate or a task." Cross-referenced, as dated notes only, from
  `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 7 and the § 3 rows it extends.
- **The questions (verbatim, § 11):**
  1. "Adopt the census as proposed (DRAFT-VAL-08), with stage 0 as the precondition."
  2. "Agree the CI cadence in §8.1 and the Metal runner in §8.2."
  3. "Agree that census verdict thresholds are R-71 calibrations, set once and stated, and never gates."
  4. "Agree that `census/` sits outside `results/`."
  5. "Decide whether stage-1 results feed the RQ-577 ruling, or that ruling stays after M3 regardless ([PR #310's
     ruling])." The bracket replaces the proposal's ruling number, which is PR #310's and not yet on `main`, so
     `plan/check_plan.py` passes on this branch; restore the number once PR #310 (carried by PR #313) has merged.
- **Options as written:** questions 1 to 4 are each adopt or agree as proposed, or not. Question 5's two: (a)
  "stage-1 results feed the RQ-577 ruling"; (b) "that ruling stays after M3 regardless".
- **Found while checking the proposal against the repo (27 Oct 2026), left as questions rather than edits:**
  6. **The stepper axis.** § 3.1 lists "RK4, KDK leapfrog" as the steppers now. In the prin-rs reference (R-159, not
     authority) the KDK leapfrog ships only under logH: `docs/reference/prin-rs/src/integrate/logh/step.rs` :26,
     "`Az + kdk` and `Heggie + kdk` do not exist", and its `Stepper` is `Rk4`, `Kdk`, `Gbs` (:149–157). In the
     contract it is a general stepper: `docs/contracts/principia_integrator_contract.md` Part 2b, "the stepper (KDK,
     Yoshida-4, Yoshida-6, RK4; Euler in the debug selection only, Part 2) are selected separately … and every pair
     runs" (R-561), with Heggie × KDK measured bad. No stepper is built on `main` yet (M3). Does the census cross
     the contract's steppers (adding Yoshida-4 and Yoshida-6, Euler in debug, and KDK under `none`, AZ and Heggie)
     or the reference's? Stage 1's "2 × 2 × 4 × 6 × 6 ≈ 600 cells" counts two.
  7. **GBS.** § 3.2: "Every shipped stepper takes a fixed step in fictitious time … there is no embedded error
     estimate to set a tolerance on." prin-rs's logH ships `Stepper::Gbs` with `gbs_tol`, "Relative tolerance on the
     GBS extrapolation error estimate" (`…/logh/driver.rs` :138–143), and it is not a contract occupant. Is GBS a
     census stepper value, and if so is it in § 3.2's tolerance-ladder group?
  8. **IAS15.** § 3.2 calls IAS15 "already the drift reference in the logH comparison". The only record of it in the
     repo says "**IAS15 is not built.** … its only role here is a reference arm"
     (`docs/reference/prin-rs/docs/NOTES_excerpts.md` :119–121). Is IAS15 to be built for the census, or is that
     clause to be corrected?
  9. **The step-limit axis.** prin-rs's `StepLimit` (`None`, `Reject`, `Predictive`, `AbGrowth`, `Global`) is AZ's
     alone (`…/az/driver.rs` :72–93, its `f` the separate `step_limit_f`, :455); Heggie and logH carry only the
     predictive limit, through `step_limit_f` (`…/heggie/driver.rs` :64, :364; `…/logh/driver.rs` :114, :376), and
     the contract fixes the predictive limit at f = 0.02 (Part 2b, R-160). § 3.1 crosses the step limit "fully" and
     omits `Reject`. Is the step-limit axis AZ-only beyond `None`/`Predictive`, and is `Reject` a value or a
     control?
  10. **The escape rule.** § 3.3's names, `Reference`, `Distance(r)`, `Closure`, are prin-rs's
      `EscapeRule::{Reference, Distance(T), Closure(T)}` (`docs/reference/prin-rs/src/outcome.rs` :225–236). The
      contract has one escape rule, the closure window with `E_rel > 0` and the total mass (Part 7; R-29), and calls
      prin-rs's `M_pair`-only `E_rel` "a physics error that biases toward escape" (R-29). Is the escape-rule sweep a
      harness-only axis over prin-rs's variants, and under which `E_rel`?
  11. **Time transformations.** § 3.1's regularisation values are "none, AZ, Heggie, logH (TTL)". The contract keeps
      one cell per pair and time transformation: Heggie's Eq. 20 and Eq. 22, logH's logH and TTL modes (Part 2b,
      R-161, R-162, R-561). Are these separate values of the axis?
- **Applied meanwhile:** none. Nothing is built: no harness, CI workflow or runner registration (the human, PR #314).
  The proposal's two factual corrections made on PR #314 (the `StepLimit` variants' form; PL-7's horizon placed on
  the precision horizon) change no question here.
- **Waits:** the census harness, its CI jobs (§ 8.1) and the `metal` self-hosted runner (§ 8.2); DRAFT-VAL-08's
  adoption into PLAN.md; question 5 bears on RQ-577's timing.

## RQ-600: The latent Jacobian's central-difference step: "`J_D`'s step rule" is cited, but no step rule for `J_D` is written *(missing value; found by TASK-M6-35; R-550 J1; REQ-DEC-049, REQ-DEC-034, REQ-GUI-400; context: TASK-M6-35, TASK-M6-07, TASK-M8-54)*

*Found by TASK-M6-35's implementer before building. The orchestrator may decide it (R-369: a numerical method's
parameter, not physics) or treat it as a calibration (R-71).*

- **File, section:**
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision", "The latent Jacobian (R-550,
    RQ-341 (a))": "a CPU, f64, central-difference Jacobian `∂x/∂z` of the decoder with **8 columns**, one per latent
    direction (two decode evaluations per direction, the step rule `J_D`'s)".
  - The same section, `J_D` itself: "compute `J_D = ∂D/∂(u,v)` at the centre by **central differences in f64** (two
    decode evals per axis)". No step is given.
  - `plan/requirements.yaml` REQ-DEC-049: "with the step rule of REQ-DEC-034's J_D"; REQ-DEC-034: "J_D is central
    differences of the full chart→IC composite in f64", with no step either.
  - `decisions.md` § "R-550 — …", J1: "with the central-difference rule REQ-DEC-034's `J_D` uses (two decode
    evaluations per direction, deep_zoom §2)"; § "R-596 — …" and `docs/design/principia_lines.md` (item 3's ruling):
    "REQ-DEC-034's step rule", for the trace fade's Jacobian.
  - The corpus's only written step for a central-difference Jacobian is the encode's:
    `docs/contracts/principia_inverse_encode_contract.md` Part 4, "How it is evaluated": "`hₖ = ε_R^{1/3}·max(1,
    |xₖ|)`" (built as `kernel::encode::ingest`, whose `weighted_jacobian`, `W·∂D/∂z`, uses it on the decode too).
- **The question:** which step `hₖ` the latent Jacobian (and `J_D`, which TASK-M6-07 has not built) takes along each
  direction. REQ-DEC-049's verify ("agree with it to the method's truncation bound"; "a one-sided difference fails the
  fixture's bound") needs a stated step to state that bound.
- **Options seen:** (a) `hₖ = ε^{1/3}·max(1, |zₖ|)` in f64, the encode contract's rule, written into deep_zoom §2 as
  `J_D`'s step rule and so the latent Jacobian's and R-596's (the step actually taken, `(z + h) − (z − h)`, as the
  divisor, as `kernel::encode::ingest` does); (b) (a) for the latent Jacobian only, `J_D`'s step left to TASK-M6-07
  with the quad's half-width (the precision-ladder proposal's concern, RQ-577); (c) another rule.
- **Applied meanwhile:** none. TASK-M6-35 waits.

## RQ-601: How `J⁺` is formed in the physical steer's projector test `P = J J⁺`, when the central-difference `J` is never exactly rank-deficient *(missing value; found by TASK-M6-35; R-550 RQ-344 (a2), RQ-345 (b); REQ-GUI-200, REQ-GUI-346; context: TASK-M6-35)*

*Found by TASK-M6-35's implementer before building, with a measurement. For the human or the physics reviewer: it
decides which clicks the steer refuses, on an `E*` chart and at a saturated link or a pole.*

- **File, section:**
  - `decisions.md` § "R-550 — …", RQ-344: "A click is refused, with the reason logged, where the smaller singular value
    of `[P e_i, P e_j]` (`P = J J⁺`) is below a stated tolerance"; RQ-345: "On an `E*`-normalised chart the steer
    refuses a cell whose plane needs the direction normal to the energy shell … RQ-344's (a2) test detects it (`P e`
    loses that direction)".
  - `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)", Jacobi and Shape: "with
    `E*` set every physical Jacobian has rank 7, the direction normal to the energy shell out of reach".
  - `plan/requirements.yaml` REQ-GUI-346: "the bound below which the smaller singular value of [P e_i, P e_j] (P =
    J J⁺) refuses a click", the one calibrated value; nothing states how `J⁺` drops a direction.
- **The question:** `J` is 8 × 8 and taken by central differences, so in floating point its smallest singular value is
  the difference's error, not 0, where its exact rank is 7. Measured on the default decoder (500 fixed-seed `z` in
  `[−3, 3]⁸`, the Jacobi set over R-550's ranges, step as RQ-600's (a)): `σ₈/σ₁` from 3.9e-6 to 0.46 (median 1.6e-3)
  with `E*` unset, and from 1.1e-17 to 1.9e-12 (median 2.8e-14) with `E* = 0`. With `J⁺` the exact pseudo-inverse of
  the computed `J`, `P = J J⁺ = I` to rounding, `σ_min[P e_i, P e_j] = 1` for every cell, and the test never refuses,
  on an `E*` chart included. A rule for dropping a direction from `J⁺` is needed and is not written; the same holds
  for a saturated link (a column exactly or nearly 0) and the Shape view's poles.
- **Options seen:** (a) `J⁺` from `J`'s SVD with each singular value below `c·σ₁` dropped, `c` a second calibration
  value (R-71) proposed beside REQ-GUI-346 from the measured gap; (b) one value: the same relative cut as
  REQ-GUI-346's tolerance, so a single calibration serves both; (c) on an `E*` chart, `P = I − n̂ n̂ᵀ` exactly, `n̂` the
  energy's gradient in the view's scaled coordinates (the shell's normal, which is `J`'s left null direction), with
  (a)'s cut only for saturation and poles.
- **Applied meanwhile:** none. TASK-M6-35 waits.

## RQ-603: The hidden coordinates of the charts that hold a block by a construction, not by latent controls: the mass simplex's configuration, the rest start's momenta, and the Burrau charts with no free control *(physics and definition, R-72; found by TASK-M2-36's implementer; R-473 T1, J1, J2; R-554 L2; REQ-CHART-062, REQ-CHART-065; context: TASK-M2-36)*

*Found by TASK-M2-36's implementer, before any code. For the human: yes (physics: what a tilt or a slice of these
charts decodes to changes their pixels; and part (c) is a requirement that cannot be met as written).*

- **File, section:**
  - `decisions.md` § R-473, J1: "A chart's hidden coordinates are the latent controls of the blocks its axes do not
    write (its block-touch sets, chart_decoder_contract Part 5): … on the mass simplex the configuration's two and the
    four momentum controls; on a Burrau-family chart whatever its embed leaves free; … A chart with no free control in
    a block offers none from it."
  - The same, J2: "every hidden coordinate is set and tilted through its latent control (a mass through its logit)";
    T2: "`d` is added to the hidden coordinate's value at the slice centre"; T1: "Tilt and slice are available on
    every chart: latent, shape sphere, mass simplex, Burrau family, invariant charts, and custom and saved charts."
  - `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)", "Tilt on every chart": "A chart's
    hidden coordinates are the latent controls its axes do not write (Part 5's block-touch sets)".
  - `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps", "Ternary mass plot": "Fix the
    geometry at $\nu_0$. … $\Phi_{\mathrm{mass}}(u,v) = (\nu_0,\ (m_0, m_1, m_2),\ \mathrm{rest})$"; the same section
    gives the Euclid plane and the δm strip a rest start, and the `(ν, K)` chart, `Φ_θ,K` and `Φ_θ,L_z` their momenta
    from §2.2's construction at a held `L_z = 0` or `K`.
  - `plan/requirements.yaml` REQ-CHART-062, verify: "over every chart of kernel::chart::REGISTRY (the registry's own
    enumeration), under Lean each axis with one target at τ₁ = τ₂ = 90° and at τ₁ = −90°, τ₂ = 90°, … each chart
    reports ray-degenerate"; REQ-CHART-065, verify: "per gallery chart and per hidden coordinate, a slice edit moves
    only that latent control of z₀".
- **What the code holds** (`crates/kernel/src/chart/`): the latent chart, the shape sphere (`z_mass`, `z_mom`), the
  invariant charts (`z_cfg`, `z_mass`) and the mixed charts (`frozen`) hold their unwritten blocks as latent controls,
  so J1's hidden coordinates are well defined there. The mass simplex holds no latent control: its configuration is
  the Burrau triangle's *positions* at `ν₀`, held while the masses vary, and its momenta are a rest start set to 0
  directly. The Euclid plane and the δm strip hold a rest start likewise. The `(ν, K)` chart, `Φ_θ,K` and `Φ_θ,L_z`
  have every block written: the embed ties the configuration and masses, and the `K` or `L_z` axis with the held
  invariant and R-82's seed writes the momentum block.
- **The questions:**
  - **(a) The mass simplex's configuration.** Its latent configuration controls `(z_α, z_β)` are not constants of the
    chart: the held triangle's mass-weighted Jacobi angles change with each pixel's masses. So "the hidden
    coordinate's value at the slice centre" is not one number, and a tilt that adds `d` to the centre's `z_α` and
    holds it across the window would not reduce to the untilted chart as `d → 0` (every pixel but the centre would
    leave the triangle). What does a tilt or a slice into `z_α` or `z_β` decode to on the mass simplex?
  - **(b) The rest start's momenta** (the mass simplex, the Euclid plane, the δm strip). Is the rest start the four
    momentum controls at 0, so that a tilt or slice adds `d` to a control from 0 and the momenta are `D_mom` of it
    under the pixel's masses? (Every registered momentum link maps 0 to 0: `q_max·(2σ(0) − 1)`, `q_max·tanh 0`,
    `q_max·0/(1 + 0)`.)
  - **(c) The charts with no free control.** By J1's own rule the `(ν, K)` chart, `Φ_θ,K` and `Φ_θ,L_z` offer no hidden
    coordinate, so they cannot be tilted or sliced, against T1's "every chart … Burrau family" and REQ-CHART-062's
    corner arm "over every chart of kernel::chart::REGISTRY", which needs each chart tilted with one target.
- **Options seen:**
  - (a1) Per pixel: the tilt adds `d` to the pixel's own pre-image of the held triangle at its masses (an encode per
    pixel), so `d = 0` is the untilted chart to rounding; a slice sets the centre's control and the chart then holds
    `(z_α, z_β)` in place of the triangle, a different chart. (a2) The mass simplex offers only the momentum controls,
    its configuration being held by `ν₀`, as the Burrau family's is by its embed (J1's "a chart with no free control in
    a block offers none from it" read for the configuration). (a3) The configuration's hidden coordinate is `ν₀`
    itself, a physical value, as R-554 L5 makes the shape sphere's `p_λ` targets.
  - (b1) Yes: the rest start is `z_mom = 0`, and each momentum control is a hidden coordinate from 0. (b2) No momentum
    target on a rest-start chart.
  - (c1) Those three charts offer no tilt and no slice, and REQ-CHART-062's corner arm and REQ-CHART-065 run over the
    registry's charts that offer a hidden coordinate. (c2) They gain a hidden coordinate that is a physical value, the
    held `L_z` (on `(ν, K)` and `Φ_θ,K`) or `K` (on `Φ_θ,L_z`), tilted by its value as R-554 L5's `p_λ`.
- **Applied meanwhile:** none; nothing is built.
- **Waits:** TASK-M2-36, on these six charts: REQ-CHART-062's every-chart arms (C2, the 30° tilt, the anchor, the
  corner over the registry) and REQ-CHART-065's per-chart slice. The rest of the task's tilt path, on the latent,
  shape-sphere, invariant and mixed charts (REQ-CHART-063, REQ-CHART-064, REQ-CHART-066 and those charts' arms of
  REQ-CHART-062 and REQ-CHART-065), does not turn on the answer; the task stopped whole, per the escalation rule,
  for the orchestrator to say whether that part proceeds.

## RQ-604: Refinement policy §1's "an unusable copy" has no payload predicate, so no reduction can tell an undetermined footprint whose spread is finite *(definition, R-72; found by physics review 5679524618 of PR #318, F4; dd_refinement_policy §1, dd_simstate_payload §1 and "VERTICAL-SLICE ADDITIONS", dd_generation_root §3.7; R-603, REQ-PAY-110, REQ-PAY-109; context: TASK-M5-32, TASK-M3-41)*

*Found while fixing physics review 5679524618 of PR #318 (TASK-M5-32), F4. A missing definition, so it becomes a
definition requirement (R-72) written by the task that needs it. For the human: no; the orchestrator and the physics
reviewer decide it (R-369), unless the definition changes what a footprint's state means physically.*

- **File, section:**
  - `docs/design/principia_dd_refinement_policy.md` § "1. The split rule — no exponent in the split test" (:91):
    "∨  the footprint is undetermined  (non-finite spread, or an unusable copy)"; and (:99): "The other undetermined
    footprints (a non-finite spread, an unusable copy of another kind) split as above."
  - `docs/design/principia_dd_simstate_payload.md` § "VERTICAL-SLICE ADDITIONS" (:736): "A quad with **every
    footprint budget-exhausted and all 512 copies flagged unusable** reads `ensemble_spread` **finite on every one**."
    This reports a prin-rs measurement and defines no payload flag.
  - `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct" (:71-73): "`budget_word : u32,
    // R-603's two counters (separation-floor hits, entry-cap hits) and, since sample_descriptor's 16 bits are all
    assigned (R-545), the sticky budget_exhausted bit. Bits, widths and saturation: TASK-M3-41, in §2 (REQ-PAY-110).`"
  - `docs/design/principia_dd_generation_root.md` § "Temporal accumulators (scheduler Part 8)", the
    `running_mean_divergence` definition (TASK-M5-32, PR #318): "A footprint undetermined with a finite
    `spread_shape`, by an unusable copy (policy §1) or by a regularised copy's step budget (R-603 …), contributes
    that `spread_shape`: the payload carries no predicate for an unusable copy, and R-603's `budget_exhausted` bit is
    placed by TASK-M3-41 (REQ-PAY-110), so neither is read here."
- **The gap:**
  - No ledger entry, payload bit or `state` code is defined as "unusable copy", so neither the resolve pass nor a
    reduction can evaluate policy §1's second undetermined case by a bit test.
  - Reading it as `state ∈ {sim_failed, decode_failed}` (payload §2, :271, :286) is plausible, but the corpus doesn't
    say so, and policy §1 :99's "an unusable copy of another kind" suggests the budget-exhausted copy is one kind
    among several.
- **R-603's case is not part of this gap:** its sticky `budget_exhausted` bit arrives with TASK-M3-41 (REQ-PAY-110),
  which places it in `budget_word`. Once that merges, the bit is readable.
- **Options seen:**
  - (a) Define "unusable copy" as `state ∈ {sim_failed, decode_failed}` or `budget_exhausted`, a bit test on
    `sample_descriptor`'s `state` and `budget_word`, written into refinement policy §1 and payload §2.
  - (b) Define a dedicated per-copy flag. A schema change: `sample_descriptor`'s 16 bits are all assigned (R-545), so
    it would go in `budget_word` or a new word.
  - (c) Drop "unusable copy" from policy §1, keeping only the non-finite spread and R-603's budget case.
- **What waits on it:** policy §1's `unresolved(f)` as built (whichever task builds the resolve pass's undetermined
  test), and any widening of `running_mean_divergence`'s `d = 1` from a non-finite spread to every undetermined
  footprint (physics review 5679524618's option (b)).
- **Applied meanwhile:** TASK-M5-32 applies option (a) of that review, per R-369. `d = 1` applies only to a
  non-finite `spread_shape`, and a finite undetermined footprint contributes its `spread_shape`. Nothing else waits.
