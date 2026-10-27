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
