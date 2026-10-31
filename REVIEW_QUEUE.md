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

## RQ-593: A binary-plus-single chart, for Trani et al.'s planar row, Manwadkar et al.'s escape probabilities and Stone & Leigh's closed forms *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-318, REQ-VAL-319, REQ-VAL-321; context: TASK-M3-63)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any new chart a target
needs".*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-23 (was TASK-M3-63). This question
still decides what is built for them, never whether a milestone passes; it stays open.

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
- **28 Oct 2026 (R-636):** stays open. The five-chart build, TASK-M2-36 (and TASK-M2-52 after it), is held until the
  owner rules on the replication batch, RQ-591 to RQ-598 (R-636: "Five-chart build (queue item d): hold until Malachy
  rules on the replication batch; do not start it."); flagged in R-636: those charts come from R-633's split, not from
  this batch.
- **28 Oct 2026 (R-637):** the owner: "Five-chart build: yes, I meant only the replication charts. This morning's
  five-chart split may go ahead." R-636's hold is lifted for TASK-M2-36 and TASK-M2-52. This entry's chart is a replication chart and is not built ahead of
  its ruling (R-594 item 5). Stays open.

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

## RQ-595: A syzygy-symbol field, a fixed-k velocity chart and Chernin et al.'s definitions, for the symbol-sequence and partition maps *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-325, REQ-VAL-326, REQ-VAL-327; context: TASK-M4-24)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, R-594 item 6: "any new chart a target
needs".*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-28 (was TASK-M4-24). This question
still decides what is built for them, never whether a milestone passes; it stays open.

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
- **28 Oct 2026 (R-636):** stays open. The five-chart build, TASK-M2-36 (and TASK-M2-52 after it), is held until the
  owner rules on the replication batch, RQ-591 to RQ-598 (R-636: "Five-chart build (queue item d): hold until Malachy
  rules on the replication batch; do not start it."); flagged in R-636: those charts come from R-633's split, not from
  this batch.
- **28 Oct 2026 (R-637):** the owner: "Five-chart build: yes, I meant only the replication charts. This morning's
  five-chart split may go ahead." R-636's hold is lifted for TASK-M2-36 and TASK-M2-52. This entry's field and chart are replication work and are not built
  ahead of its ruling (R-594 item 5). Stays open.

## RQ-596: The N_scramble and f_col diagnostics *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-318, REQ-VAL-321, REQ-VAL-323; context: TASK-M3-63, TASK-M3-64)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, each adds a diagnostic no task builds, as a new
chart does.*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context tasks are now TASK-M9-23 and TASK-M9-24 (were TASK-M3-63 and
TASK-M3-64). This question still decides what is built for them, never whether a milestone passes; it stays open.

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
- **28 Oct 2026 (R-636):** stays open. The five-chart build, TASK-M2-36 (and TASK-M2-52 after it), is held until the
  owner rules on the replication batch, RQ-591 to RQ-598 (R-636: "Five-chart build (queue item d): hold until Malachy
  rules on the replication batch; do not start it."); flagged in R-636: those charts come from R-633's split, not from
  this batch.
- **28 Oct 2026 (R-637):** the owner: "Five-chart build: yes, I meant only the replication charts. This morning's
  five-chart split may go ahead." R-636's hold is lifted for TASK-M2-36 and TASK-M2-52. Stays open.

## RQ-597: Periodic-orbit continuation over the mass plane, for Li, Li & Liao 2020's stability map *(the human's queued question, R-594 item 6; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-329; context: TASK-M4-25)*

*Queued by R-594 (25 Oct 2026), ported 27 Oct 2026. For the human: yes, it adds a capability no task builds.*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-29 (was TASK-M4-25). This question
still decides what is built for them, never whether a milestone passes; it stays open.

- **File, section:** `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or
  data": LLL-1, "Period, angular momentum and stability contours over (m₁, m₂) for one periodic family … must reproduce
  Li, Li & Liao 2020's on the mass simplex", which needs "periodic-orbit continuation (not planned)" (PLAN.md § 3.2).
- **The question:** build a continuation of a periodic orbit across masses (a Newton–shooting solve per mass point),
  so the mass simplex can show one family's period and stability; or leave LLL-1 waiting.
- **Options seen:** (a) build it, by a later task; (b) no: LLL-1 waits, and the atlas's seven `li2021` rows stay closure
  tests (OC-1).
- **Applied meanwhile:** (b).
- **Waits:** TASK-M4-25.
- **28 Oct 2026 (R-636):** stays open. The five-chart build, TASK-M2-36 (and TASK-M2-52 after it), is held until the
  owner rules on the replication batch, RQ-591 to RQ-598 (R-636: "Five-chart build (queue item d): hold until Malachy
  rules on the replication batch; do not start it."); flagged in R-636: those charts come from R-633's split, not from
  this batch.
- **28 Oct 2026 (R-637):** the owner: "Five-chart build: yes, I meant only the replication charts. This morning's
  five-chart split may go ahead." R-636's hold is lifted for TASK-M2-36 and TASK-M2-52. Stays open.

## RQ-608: The Šuvakov-plane targets that need a chart, now that no chart is built: OC-4 on the plane, AT-D1, AT-D3, HU-1 and HU-2 *(physics; R-636, closing RQ-583; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-311 to REQ-VAL-314, REQ-VAL-316; context: TASK-M4-23, TASK-M8-99)*

*Queued 28 Oct 2026 by R-636's port. For the human: yes, it decides what the M4 and M8 gates hold.*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context tasks are now TASK-M9-27 and TASK-M9-35 (were TASK-M4-23 and
TASK-M8-99). This question still decides what is built for them, never whether a milestone passes; it stays open. Its
"what the M4 and M8 gates hold" is answered for the gates by R-650: no gate holds these targets; what is built for them
is still open.

- **File, section:**
  - `decisions.md` § "R-636 — …", the owner's words: "Šuvakov-plane chart: keep its targets as closures for now; no
    chart build yet."
  - `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data": "a chart
    over the Šuvakov plane … | HU-1, HU-2, AT-D1, AT-D3, OC-4 on the plane"; the rows: OC-4 on the plane (REQ-VAL-311,
    statistical, M8), AT-D1 (REQ-VAL-312, qualitative, M8), AT-D3 (REQ-VAL-313, qualitative, M8), HU-1 (REQ-VAL-314,
    "qualitative; exact at Table 23's minima", M4), HU-2 (REQ-VAL-316, "qualitative + one exact point", M4), each
    with "new: SuvakovPlane" as its chart.
  - `plan/MILESTONES.md`: a milestone's exit gate is every requirement whose `milestone` is that milestone.
- **The gap:** these five targets are maps or statistics over the plane, and each names a chart that R-636 does not
  build. They cannot be Path A closures as written, and as they stand they sit in the M4 and M8 gates with no chart to
  run on.
- **Options seen:** (a) they wait, in their gates, until a chart question is taken up; (b) they leave their gates
  (reported only, or moved to a later milestone) until then; (c) each is reduced to what a closure can test (HU-1's
  Table 23 minima and HU-2's exact point as closures), the rest waiting as in (a) or (b).
- **Applied meanwhile:** nothing is built: no chart, and no map. Each requirement carries `rq: RQ-608` beside RQ-583,
  and TASK-M4-23 and TASK-M8-99 wait on it.
- **Waits:** TASK-M4-23 and TASK-M8-99.
- **28 Oct 2026 (R-639):** the owner: "(1) Šuvakov plane targets: split each into its closure part (stays in M4/M8,
  runs now) and its chart part (moves to gate the task that builds the Šuvakov chart; not dropped). Queue it if the
  split isn't clean for any target." Read target by target in R-639, no split is clean: OC-4 on the plane, AT-D1 and
  AT-D3 have no closure part, and HU-1's and HU-2's closure parts would test what HU-3 already tests (REQ-VAL-234), with
  no control of their own. All five are queued as RQ-618 and left as they are; this entry stays open until RQ-618 is
  answered, and its holds stand.

## RQ-610: Five committed PDFs whose licence is unclear: the Agekyan–Anosova and Anosova translations, the Belgrade gallery's initial-conditions PDF, and Hudomal's MSc thesis *(the owner's licence-audit ruling of 28 Oct 2026; the read-only audit posted on PR #327; context: none built)*

*Queued by the owner (28 Oct 2026): "The 5 "unclear" ones stay for now; queue them." They stay in the repo until
ruled. For the owner: yes, it decides whether five files stay in a public repository.*

- **File, section:** the audit's rows 12 to 16 (PR #327, comment "Read-only licence audit of every committed PDF"):
  - `docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_Ap_4_11_part2.pdf` (sha256 `7d160daf…1c9368`): "No
    copyright line, stamp or licence text found (text layer is OCR only). Missing: any statement of who holds the
    rights to the translation."
  - `docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_SvA_11_1006.pdf` (sha256 `5bb197a9…d2574f8`): "Image
    only … no copyright line, stamp or licence readable by text search". Its README: "ADS scan".
  - `docs/incoming/2026-10-25_replication/papers/anosova_1969_Ap_5_81_part3.pdf` (sha256 `be19ced2…88bcf`): "No
    copyright line, stamp or licence text found. Missing: rights statement."
  - `docs/incoming/2026-10-26_belgrade_gallery/archive/initial-conditions.pdf` (sha256 `5b76cfd6…3ade5b`): the
    gallery's "Use and Copyright" page, as its README quotes: "You may copy and use any images or data from this web
    gallery. Please, credit us by citing this page and our paper [8]." Whether that covers this PDF as a document is
    not stated.
  - `docs/incoming/2026-10-26_orbit_catalogues/hudomal_2015_msc_belgrade.pdf` (sha256 `b30dbad2…c2fb0`): "No
    copyright, stamp or licence in the file (searched all 62 pages)", from https://www.scl.rs/theses/msc_ahudomal.pdf.
- **The question:** for each file, keep it in the repo or move it to `docs/incoming/_local/` as the ten were.
- **Options seen:** (a) keep all five; (b) move all five; (c) per file: for example, keep the gallery's PDF under its
  "copy and use" terms, and move the translations and the thesis.
- **Applied meanwhile:** all five stay, as the owner ruled.
- **Waits:** nothing built.

## RQ-612: GG-1's lifetime T: which definition and which units reproduce the paper's Fig. 1, and whether (3, 43) is a grid offset *(queued by the Brutus pilot; `docs/incoming/2026-10-28_rq598_data/brutus_run_here/REPORT.md` § 2; the data-and-papers entry on PR #309; context: TASK-M3-62)*

*Queued 29 Oct 2026 by the Brutus pilot, as corrected by the physics review of #334. For the owner: yes, it is physics where the choice
changes the result and the corpus does not settle it (R-369). Not yet ready to rule: the neighbour runs in option (a) come first.*

*30 Oct 2026 (R-649, R-650):* GG-1 and BR-1 are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-22 (was TASK-M3-62). This question
decides how they are scored, never whether a milestone passes; it stays open. PR #335's three-point comparison is the
peek log's first row (`docs/verification/peek_log.md`), UNEXPLAINED, with this question among its like-for-like checks.

- **File, section:**
  - Boekholt, Portegies Zwart & Valtonen 2020 (arXiv:2002.04029,
    `docs/incoming/2026-10-25_replication/papers/boekholt_pz_valtonen_2020_MNRAS_493_3932_arXiv2002.04029.pdf`), Appendix A: "We evolve
    each initial realization to the point of a permanent binary-single configuration, or a maximum time of 10,000 time units (Heggie &
    Mathieu 1986). The single body is defined to be permanently escaped if 1) it is separated from the binary center of mass by more than
    10 distance units", and "The lifetime is measured by considering the final snapshot of the forward integration, consisting of the
    permanent binary+single, and then retracing their steps to the moment when the binary+single were closest to the moment of the final
    ejection."
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, GG-1: "lifetime to the last close approach".
  - `fixtures/anosova/region_d.json`: m = (1, 1, 1), bodies 1 and 2 at (∓0.5, 0), the repo's convention (G = 1, at rest).
- **What was found** (ours, run here; `brutus_run_here/hm_units_output.txt`): the pilot ran in m = 1, G = 1, r₁₂ = 1; the paper's times are
  Heggie–Mathieu units (G = 1, M = 1, E = −1/4). The conversion depends on the point, T_HM = T_ours · √(3λ³), λ = (4/9) Σ 1/r_ij, a factor
  of 2.72 to 167 over the grid. Converted, at the three sample points where the paper's two panels agree, ours against Fig. 1 (route 2,
  `transcriptions/gg1_fig1/`) is (1, 54) −0.29 dex, (14, 20) +0.15 dex and (3, 43) −2.45 dex, each unchanged over ε = 10⁻⁴ … 10⁻⁶. In
  Fig. 1, (3, 43) is log₁₀ T = 2.713, an isolated single-cell maximum: all eight neighbours are below it, (3, 42) 1.881, (3, 44) 1.107,
  (2, 43) 0.896, (4, 43) 1.775, (2, 42) 0.920, (2, 44) 0.873, (4, 42) 1.518 and (4, 44) 1.307 (`hm_units_output.txt`). Ours there is 0.262
  in HM units, so even against the nearest short neighbours ((2, 42), (2, 43), (2, 44), (3, 44)) it is 0.61 to 0.85 dex low, the same sign
  as (1, 54)'s −0.29 dex. At (1, 54) ours equals the equilateral free-fall collapse time (1.71 HM), and Fig. 1's 3.3 HM is
  1.9 times that. The pilot's escape radius (10 of its own length units, 13.5 to 210 HM) and cap (1000 of its units) were not the paper's.
- **The question:** which definition of T (the retrace rule, the escape radius in HM units) makes our Brutus runs and the paper's figure
  agree where the paper's two panels agree; and whether (3, 43) is also a one-cell offset in the initial condition or the grid snap. An
  offset would leave the 0.61 to 0.85 dex remainder above, so an offset and a definition difference may both be present; one does not rule
  out the other.
- **Options seen:** (a) first run all eight neighbours of (3, 43) at ε = 10⁻⁶ with the escape radius and cap in HM units, and seek no
  definition ruling before that: the four short ones ((2, 42), (2, 43), (2, 44), (3, 44)) test whether ours at (3, 43) matches a shifted
  short cell, and the other four ((3, 42) 1.88, (4, 43) 1.78, (4, 42), (4, 44), none of them short) whether ours puts a long-lived cell one
  step over; (b) keep ours, labelled ours, and treat the remaining disagreement as a finding; (c) ask Boekholt for the retrace rule and the
  grid convention, in the email already planned, and also whether Fig. 1's times are in Heggie–Mathieu units: read instead as G = M = 1
  with no energy scaling (r₁₂ = 1 kept), the factor is a constant √3 and the three points are −0.48, −0.32 and −2.72 dex, each further
  from Fig. 1 than the HM reading's −0.29, +0.15 and −2.45, so the HM reading, which is the paper's citation, is applied; (d) test
  candidate conventions on a spread of grid points against route 2, which needs more than the 16-point sample (scaling: the owner's
  go-ahead).
- **Applied meanwhile:** (b). A definition, once ruled, becomes a definition requirement (R-72).
- **Waits:** the point-by-point comparison of route 3 (recomputed) against route 2 (read from the figure) for GG-1.

## RQ-613: Brutus settings the GG-1 paper does not give (`p_max`; a pinned `main.cpp` that may never apply the word length), and the ε = 10⁻⁷⁰ cost, undetermined to orders of magnitude *(queued by the Brutus pilot; `docs/incoming/2026-10-28_rq598_data/brutus_run_here/REPORT.md` § 2b, § 2c; the data-and-papers entry on PR #309; context: TASK-M3-62, TASK-M3-65)*

*Queued 29 Oct 2026 by the Brutus pilot, as corrected by the physics review of #334. For the owner: yes, a missing value (R-71), a question
for the authors, and a decision on whether to spend more probe time before any 10⁻⁷⁰ cost is quoted.*

*30 Oct 2026 (R-649, R-650):* GG-1 and BR-1 are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context tasks are now TASK-M9-22 and TASK-M9-25 (were TASK-M3-62 and
TASK-M3-65). This question decides how they are scored, never whether a milestone passes; it stays open.

- **File, section:**
  - Boekholt et al. 2020, Appendix A: "we fix L_w = 1024 bits"; ε is "the Bulirsch-Stoer tolerance"; no `p_max` is given. § 2: "The average
    wall-clock time of our simulations was about 7 hours, with the longest run taking 1 month to complete."
  - `pgbreen/NvMgeneration` @ `98906f4` (not committed), as the pilot recorded it: README "input file: N time_end time_step tolerance
    word_lenght p_max"; `indat` `3 10.0 0.00390625 1e-11 128 4`; `main.cpp` reads the word length and, as recorded, never applies it
    (`brutus_run_here/stock_lw_test.txt`). Unverified until the source is read again.
- **What was measured** (ours, run here; point (1, 54), L_w = 1024): at ε = 10⁻⁶, `p_max` = 2 costs about 10× the cost of 4; 8 and 16 cost
  0.5 to 0.6× of 4. At p_max = 4 the wall grows ×3.4–4.4 from 10⁻⁶ to 10⁻⁸ and ×5.4 to 10⁻¹⁰: the local slope steepens (three ε values at
  one point; nothing more is shown). At 8 and 16: 6.8 → 10.1 s from 10⁻⁸ to 10⁻¹⁰, and the reversibility test passes at 10⁻⁸ (at 4 only at
  10⁻¹⁰), so `p_max` is an accuracy setting too.
- **What was estimated, and why it is undetermined:** from the same two points (10⁻⁸, 10⁻¹⁰ at p_max 8), a model linear in −log₁₀ε gives
  about 14,000 forward core-hours for the ε = 10⁻⁷⁰ panel, and a geometric one about 3×10⁷, on #334's weights. Both are curves through two
  points. Capped-order Bulirsch–Stoer (the stated p_max mechanism, itself unverified) should eventually cost a power of ε: with the order
  capped, the per-step cost levels off and the growth moves into the number of steps. The geometric model (186–333 s per step) puts it in
  the per-step cost instead, contradicting that mechanism, and the linear model does not include it either; neither 14,000 nor 3×10⁷
  bounds the cost. The ×11.5 step factor used (attributed to `get_eta`) is unverified, and
  the step law is documented only for ε > 10⁻⁵⁰. The weights (Fig. 1's lifetimes) are in HM units and the rates in ours: converted, they
  are 4.5× smaller with black points at 1000, and the paper's black points run to 10⁴ HM units, not 1000. The paper's "about 7 hours" is an
  average over every ε of its convergence runs, not a per-10⁻⁷⁰ cost. #334's figure of 6,000 to 14,000 core-hours is withdrawn.
- **The question:** which `p_max` the paper's runs used, and whether the authors' runs used a `main.cpp` that applied L_w; and whether to run
  one more probe (p_max 8 and 16 at, say, 10⁻²⁰ and 10⁻³⁰ on the same point, recording the number of steps and the number of
  Bulirsch–Stoer stages per step separately) before any 10⁻⁷⁰ cost is quoted.
- **Options seen:** (a) `p_max` = 4, the repo's sample inputs; (b) 8, cheaper and as accurate in our probe; (c) ask Boekholt and Breen in the
  planned emails, and use (a) until they answer; separately, (d) approve the 10⁻²⁰ / 10⁻³⁰ probe, or (e) quote no 10⁻⁷⁰ cost until the
  authors answer.
- **Applied meanwhile:** (a), with our own driver setting L_w explicitly; no 10⁻⁷⁰ cost is quoted as an estimate. A value, once ruled,
  becomes a calibration requirement (R-71).
- **Waits:** any plan to compute the ε = 10⁻⁷⁰ panel.

## RQ-615: Some of the authors' BR-1 trajectories are public, with no licence: how may they be used *(queued by the Brutus pilot; `docs/incoming/2026-10-28_rq598_data/README.md` § 5; the data-and-papers entry on PR #309; context: TASK-M3-62)*

*Queued 29 Oct 2026. For the owner: yes, a licence and process question outside the repo (R-369).*

*30 Oct 2026 (R-649, R-650):* GG-1 and BR-1 are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-22 (was TASK-M3-62). This question
decides how they are scored, never whether a milestone passes; it stays open.

- **File, section:**
  - `docs/incoming/2026-10-28_rq598_data/README.md` § 2 on BR-1: "No data statement and no code or data release." § 5 corrects it:
    `github.com/pgbreen/NVM` @ `a35c378`, `trainex/data/run10–99.dat` and `validex/data/run9910–9999.dat`, each row `x1 y1 x2 y2` at
    t = k/256, the first row the initial condition, with no licence file. The pilot counted 180 files (136 full, 44 truncated); the count is
    not verified from committed material. The paper's ensemble is about 10,000, so this is under 2% of it.
  - § 4 task 4: "Draw the initial positions from a **fixed, recorded seed**." The pilot instead ran 12 of the authors' own initial
    conditions, on instructions recorded in § 5 as quoted, not as a ruling.
- **What was done:** a copy was kept, gitignored, in `docs/incoming/_local/nvm_a35c378/` on the machine that ran the pilot; the scripts
  read it there. Nothing from it is committed: no coordinate, no row, no value computed from the files alone, and none of our trajectories
  (row 1 of each is the authors' initial condition). What is committed is run names, row counts, and our differences against their rows.
- **The question:** whether that use is enough; whether BR-1 may be graded against these trajectories; and whether runs from the authors'
  initial conditions replace § 4 task 4's seeded draw.
- **Options seen:** (a) as now: local only, the repo says where to fetch them, nothing derived is committed; (b) the repo's test fetches
  them at test time and checks a hash; (c) ask Breen for a licence or written permission first; (d) do not use them, and go back to § 4
  task 4's seeded draw.
- **Applied meanwhile:** (a), and the seeded draw of § 4 task 4 is not run.
- **Waits:** a BR-1 ensemble beyond the 12-run pilot.
- **28 Oct 2026 (R-641, R-642), two of its three questions answered:** the 180 trajectories stay local and are never
  committed, their per-file sha256 table and citation recorded (`docs/incoming/2026-10-28_rq598_data/README.md` § 6,
  `nvm_a35c378_sha256.txt`): option (a) as applied; and BR-1 may be graded against them directly, not provisionally,
  for those 180 (R-641: "these 180 are the paper's own data, so BR-1 may be scored against them directly (not
  provisional) for those 180"). **Still open:** whether runs from the authors' initial conditions stand in for the
  seeded draw, which R-638 asks for ("BR-1 Brutus ensemble on Breen's stated set-up now, fixed recorded seed"); the
  owner's "Resume." message of 28 Oct 2026 names the pilot only ("pilot BR-1 from the NVM starting points"). The seeded
  draw is still not run.

## RQ-616: BR-1's convergence criterion: is the "phase distance < 0.1" on δ or on δ² *(a definition requirement, R-72; `docs/incoming/2026-10-28_rq598_data/brutus_run_here/REPORT.md` § 3; the data-and-papers entry on PR #309; context: TASK-M3-62)*

*Queued 29 Oct 2026 from the physics review of #334. For the owner: yes, a definition the corpus does not settle, and it changes the
converged fraction BR-1 is scored on.*

*30 Oct 2026 (R-649, R-650):* GG-1 and BR-1 are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-22 (was TASK-M3-62). This question
decides how they are scored, never whether a milestone passes; it stays open.

- **File, section:** Breen et al. 2020 (arXiv:1910.07291,
  `docs/incoming/2026-10-25_replication/papers/breen_etal_2020_MNRAS_494_2465_arXiv1910.07291.pdf`), Appendix A, eq. (A1):
  δ²_{a,b} = (1/12) Σ_{i=1}^{3} Σ_{j=1}^{4} (q_{a,i,j} − q_{b,i,j})², then: "Note δ is equivalent to the mean squared error of the phase space
  coordinate. Converged solutions were identified when the average phase distance was < 0.1 over the duration of a simulation."
  `docs/incoming/2026-10-28_rq598_data/README.md` § 2: "converged when the phase distance δ (their eq. A1) stays below 0.1".
- **The ambiguity:** by (A1), δ is a root mean square, but the next sentence calls δ the mean squared error, which is δ². So the threshold
  0.1 may apply to the time-average of δ, of δ², or to √(time-average of δ²). On our 12 runs against the authors' (positions only, their
  files hold no velocities), √(time-mean MSE) < 0.1 holds for 10 of 12, and time-mean MSE < 0.1 for all 12 (`brutus_run_here/br1/br1_table.md`).
  That comparison is not the authors' test, which compares two of their own (ε, L_w) runs; but the same ambiguity sets the converged fraction
  BR-1 is compared against ("over 90%").
- **The question:** which quantity the < 0.1 threshold applies to.
- **Options seen:** (a) √ of the time-mean of δ² (δ as written in A1, RMS); (b) the time-mean of δ² (the "mean squared error" sentence);
  (c) the time-mean of δ; (d) ask Breen.
- **Applied meanwhile:** none chosen; both (a) and (b) are reported side by side. Once ruled, it becomes a definition requirement (R-72).
- **Waits:** BR-1's converged-fraction comparison.

## RQ-617: The occupant matrix census's CI cadence: the owner's words and the proposal's § 8.1 and § 8.2 differ *(the owner's request in R-637, "check §8.1 says this; if not, show me the text"; RQ-599's question 2; `docs/incoming/2026-10-27_validation_plan/PROPOSAL_occupant_matrix_census_2026-10-27.md` § 8.1, § 8.2; context: none built)*

*Queued 29 Oct 2026 by R-637's port. For the owner: yes, he asked to be shown the text; it decides what CI and the Mac
run. No option is recommended here.*

- **File, section:**
  - `decisions.md` § "R-637 — …", the owner's words: "Metal runner nightly/weekly on the Mac, lavapipe in cloud per PR
    (check §8.1 says this; if not, show me the text)."
  - The proposal, § "8.1 Cloud CI", verbatim:

    | job | trigger | contents |
    |---|---|---|
    | census-smoke | every PR touching the kernel, stepper or regularisation | stage 0, plus the default's immediate neighbours on stage-1 fixtures |
    | census-stage1 | nightly | the full stage 1, CPU and lavapipe |
    | census-stage2 | weekly, sharded by fixture | the corpus and the maps, CPU and lavapipe, cached |

  - The proposal, § "8.2 The owner's Mac", verbatim: "It is registered as a self-hosted runner, labelled `metal`,
    running the GPU columns weekly or on demand." and "It only runs when the Mac is awake and online. A missed week is
    recorded as not run, never as a pass."
- **Where they differ:**
  1. **Per PR.** The owner's words run lavapipe per PR. § 8.1 runs one job per PR, `census-smoke`, and only on PRs
     touching the kernel, stepper or regularisation; it does not say whether that job runs lavapipe. Lavapipe runs in
     `census-stage1`, nightly, and `census-stage2`, weekly.
  2. **The Metal runner.** The owner's words: "nightly/weekly on the Mac". § 8.2: "weekly or on demand", not nightly.
  - § 8.1's "Cloud CI" is CI's hosted Linux runners; a cloud session (R-645, R-647, R-648) is not involved either way.
- **The question:** which cadence the census runs at: per PR, nightly and weekly on CI's runners, and on the Mac.
- **Options seen:** (a) § 8.1 and § 8.2 as written; (b) the owner's words as written: lavapipe on every PR (the job
  and the PRs it runs on to be stated), the Metal runner nightly or weekly; (c) § 8.1 as written, with the Metal runner
  nightly.
- **Applied meanwhile:** none. Nothing of the census is built: no harness, CI job or runner registration.
- **Waits:** the census's CI jobs and the `metal` self-hosted runner.

## RQ-618: The Šuvakov-plane targets whose split into a closure part and a chart part isn't clean: all five, OC-4 on the plane, AT-D1, AT-D3, HU-1 and HU-2 *(physics; R-639, "Queue it if the split isn't clean for any target"; RQ-608; `docs/design/principia_dd_replication.md` § 5.12; REQ-VAL-311 to REQ-VAL-314, REQ-VAL-316; context: TASK-M4-23, TASK-M8-99)*

*Queued 29 Oct 2026 by R-639's port. For the owner: yes, R-639 asks for it, and it decides what the M4 and M8 gates
hold and whether a chart task is added.*

*30 Oct 2026 (R-649, R-650):* the targets here are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context tasks are now TASK-M9-27 and TASK-M9-35 (were TASK-M4-23 and
TASK-M8-99). This question still decides what is built for them, never whether a milestone passes; it stays open. Its
"what the M4 and M8 gates hold" is answered for the gates by R-650: no gate holds these targets; what is built for them
is still open.

- **File, section:**
  - `decisions.md` § "R-639 — …", the owner's words: "(1) Šuvakov plane targets: split each into its closure part
    (stays in M4/M8, runs now) and its chart part (moves to gate the task that builds the Šuvakov chart; not dropped).
    Queue it if the split isn't clean for any target."
  - `plan/requirements.yaml`, each target's verify, verbatim:
    - OC-4 on the Šuvakov plane, REQ-VAL-311 (M8, TASK-M8-99): "as OC-4, on the Šuvakov-plane chart once RQ-583 (and
      its labelling, RQ-582) is ruled; agreement at OC-4's calibrated level; control as OC-4's". OC-4, REQ-VAL-309,
      its statement (not its verify): "On a chart through each atlas orbit not on the Šuvakov plane, the payload's
      leaf decisions, spread and `error_ratio` must place linearly stable orbits inside resolved regular islands and
      unstable ones on boundaries".
    - AT-D1, REQ-VAL-312 (M8, TASK-M8-99): "a Result (R-572) at f64; reviewed by the physics reviewer; the atlas's
      credit line on the figure; control: each marker's IC re-encoded must land on its marker".
    - AT-D3, REQ-VAL-313 (M8, TASK-M8-99): "as AT-D1, per value of m₃".
    - HU-1, REQ-VAL-314 (M4, TASK-M4-23): "the closure field on the Šuvakov-plane chart once RQ-583 is ruled; each
      Table 23 minimum within REQ-VAL-315; the map reviewed by the physics reviewer; control: a map at m₃ = 2 must not
      show the equal-mass minima". REQ-VAL-315's evidence: "the map's pixel pitch and Table 23's six digits, measured".
    - HU-2, REQ-VAL-316 (M4, TASK-M4-23): "as HU-1, zoomed; the middle candidate's position and period within
      REQ-VAL-317". REQ-VAL-317's evidence: "the zoom's pixel pitch and the printed digits".
- **Why no split is clean:** OC-4 on the plane, AT-D1 and AT-D3 hold no closure: OC-4 reads an island criterion from
  the payload on a chart, and AT-D1 and AT-D3 render the plane, so the closure part of each is empty. HU-1's and HU-2's
  closure parts, Table 23's points and the middle candidate run as closures, would test that each orbit closes, which
  HU-3 already tests (REQ-VAL-234, TASK-M3-53, M3), while their tolerances are calibrated from a pixel pitch and their
  only control is a map; dividing them changes what the part tests and leaves it no control.
- **The question:** what each target becomes under R-639.
- **Options seen:** (a) each moves whole to gate a new task, "build the Šuvakov-plane chart", which waits on a ruling
  to start (R-636, RQ-583), its closure part empty or (for HU-1 and HU-2) already HU-3's; (b) as (a) for OC-4 on the
  plane, AT-D1 and AT-D3, and HU-1 and HU-2 each gain a closure part with a control of its own (Table 23's minima, and
  the middle candidate at (0.405916, 0.230163), T = 13.866, as Path A closures), kept in M4; (c) leave all five as they
  are, under RQ-608.
- **Applied meanwhile:** (c). The five requirements are unchanged and carry `rq: RQ-608`; no chart task is added.
- **Waits:** TASK-M4-23 and TASK-M8-99.

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

## RQ-620: GG-1's per-point reference panel: Fig. 1 has two panels, ε = 10⁻⁶ and ε = 10⁻⁷⁰, that differ point by point *(a definition requirement, R-72; physics; R-642 item 1; `docs/design/principia_dd_replication.md` § 5.10, § 8.1 rule 8; `docs/incoming/2026-10-28_rq598_data/README.md` § 2, § 4, § 6; the physics review of PR #336, F1; REQ-VAL-295; context: TASK-M3-62)*

*Queued 29 Oct 2026 by R-642's port, on the physics review of PR #336. For the owner: yes, it is physics where the
choice changes the result and the corpus does not settle it (R-369).*

*30 Oct 2026 (R-649, R-650):* GG-1 and BR-1 are measurements, never gates, scored in M9
(`docs/design/principia_dd_replication.md` § 12). The context task is now TASK-M9-22 (was TASK-M3-62). This question
decides how they are scored, never whether a milestone passes; it stays open. PR #335's three-point comparison is the
peek log's first row (`docs/verification/peek_log.md`), UNEXPLAINED, with this question among its like-for-like checks.

- **File, section:**
  - `decisions.md` § "R-642 — …", the owner's words, item 1: "GG-1 against the Fig. 1 vector extraction at its stated
    precision (~0.012 dex, black = censored >1000)."
  - `docs/design/principia_dd_replication.md` § "5.10 Converged-code references", GG-1's row: "1,212 points at spacing
    1/64, converged with Brutus to t = 10⁴; lifetime to the last close approach", compared "per point where reliable
    (rule 8: a converged code); KS otherwise".
  - The same doc, § "8.1 Carried over from the 25 Oct instruction", rule 8: "A chaotic result is compared as a
    distribution, never pixel for pixel. The exception is a result computed with a converged code (Brutus, clean
    numerical simulation). Those are trajectories, and can be compared per point."
  - `docs/incoming/2026-10-28_rq598_data/README.md` § 2: "The per-point lifetimes appear only in Fig. 1 (two panels:
    ε = 10⁻⁶ left, ε = 10⁻⁷⁰ right; colour log₁₀ T on 0–3; black for T > 1000). The paper's K–S test gives p = 0.72
    between the two panels."
  - The same README, § 4, task 3: "The two panels themselves differ point by point (the paper says so), so compare
    like ε with like."
  - `docs/incoming/2026-10-28_rq598_data/transcriptions/gg1_fig1/README.md` § "Files": `gg1_fig1_left_eps1e-6.csv`,
    "Left panel, ε = 10⁻⁶: 1212 rows", and `gg1_fig1_right_eps1e-70.csv`, "Right panel, ε = 10⁻⁷⁰: 1212 rows".
- **The gap:** R-642's port wrote GG-1's scoring as "all 1212 points per panel". Neither the owner's words nor the
  corpus say which panel is the per-point reference for our lifetimes, or for a reproduction ("vs Brutus, run here"),
  nor what the other panel is for. Read literally, our lifetimes would be scored per point against both panels, which
  differ point by point.
- **The question:** which panel GG-1 scores per point against, for our runs and for a reproduction, and what role the
  other panel takes.
- **Options seen:** (a) the ε = 10⁻⁷⁰ panel per point, the ε = 10⁻⁶ panel by KS only; (b) each panel per point only
  against a run at like ε (a reproduction at ε = 10⁻⁶ against the left, at 10⁻⁷⁰ against the right), our lifetimes
  against both by KS; (c) per point only where the two panels agree within one LUT step (a reliability mask), KS
  elsewhere; (d) another, stated by the owner.
- **Applied meanwhile:** none. No panel is chosen. REQ-VAL-295's note, R-642's entry (a forward line under its item 1),
  `principia_dd_replication.md` § 5.10, the RQ-598 README § 6 and TASK-M3-62's note point here.
- **Waits:** GG-1's per-point scoring against the Fig. 1 extraction (TASK-M3-62). BR-1 does not wait on it.

## RQ-621: The control REQ-DEC-011 and REQ-VAL-017 ask of `M₀₁` cannot fail: an unfused or flushed `fma(m₁, 1, m₀)` rounds exactly as the fused one *(conflict: a requirement's control against arithmetic; R-547, RQ-298; REQ-DEC-011, REQ-VAL-017; context: TASK-M2-48)*

*Found by TASK-M2-48's implementer, before building, 30 Oct 2026. For the orchestrator (R-369: tests), except option
(b), which changes the branch input's definition and is the human's (physics).*

- **File, section:**
  - `plan/requirements.yaml`, REQ-DEC-011's verify: "control: the explicit fma(m₁, 1, m₀) form, unfused, flips the tag
    on lavapipe's evaluation of it".
  - `plan/tasks/M2/TASK-M2-48.md` § "Acceptance tests", `cargo test -p kernel decode_mass`: "control: the unfused
    `fma(m₁, 1, m₀)` evaluation flips the tag (REQ-DEC-011)".
  - `plan/requirements.yaml`, REQ-VAL-017's verify, and the same task's `cargo test -p engine decode_parity`: "control:
    on the same fuzzed inputs the superseded forms (the explicit fma(m₁, 1, m₀), a plain float subtract or compare),
    evaluated on the CPU under an unfused or flush-to-zero emulation, give a different tag from the unflushed CPU on at
    least one input".
  - `docs/design/principia_dd_decoder.md` § "3.1 Mass", "The branch input": "`m₁·1` is exact, so it is the same single
    rounding of `m₀ + m₁`"; and "Its premise": "Folding `fma(x, 1, y)` to `x + y` is exact and standard … after it, a
    contracting backend can rebuild `fma(t₇, t₁₃, m₀)` and flip the tag as above."
- **The conflict:** unfused, `fma(m₁, 1, m₀)` is `round(round(m₁·1) + m₀)`, and `m₁·1` is exact, so it is
  `round(m₀ + m₁)`: bit for bit the fused value, on every input. Flushed, it differs only where `m₁` or `m₀` is
  subnormal, and then `M₀₁` is below `2^−125`, far under `ε_M01 = 10⁻⁵`, so the tag is the same. So the control the two
  requirements name cannot flip `M01_TINY` on any masses, and a test written to it can never fail. Lavapipe's unfused
  `fma` (RQ-298's measurement) gives the right tag for `M₀₁`. The hazard §3.1 describes is the other one: a backend
  that folds the `×1` and then contracts `m₀ + t₇·t₁₃` into a fused `fma(t₇, t₁₃, m₀)`, which flips 45 of 40 000 f32
  onset crossing points and 77 at f64 (`decode_m01_tiny_contraction`'s control already shows it). The other superseded
  forms REQ-VAL-017 names seem unlikely to flip a tag on test 12's fuzzed inputs either: a plain compare is unchanged
  by an unfused emulation, and the decoded states have no subnormal pair differences or products for a flush to
  change. That last point is my reading, not measured.
- **The question:** what the control for `M₀₁`, and test 12's collective control, should be.
- **Options seen:**
  - (a) The control emulates the superseded form as a folding, contracting backend evaluates it: `fma(t₇, t₁₃, m₀)`
    fused, at TASK-M2-02's onset crossings. It must flip the tag there, and the rewritten form must not. In both
    requirements' verify and the task's two acceptance lines, "unfused" becomes "with the `×1` folded and the add
    contracted (fused)". Test 12's inputs gain the onset crossings, so that "the same fuzzed inputs" include points
    where one rounding decides the tag.
  - (b) Decide the tag on the exact `m₀ + m₁ < ε` rather than on its one rounded sum. Then the explicit form, fused or
    not, flips the tag where `round(m₀ + m₁) = ε` but the exact sum is below `ε`, and the control as written holds.
    This changes §3.1's "one rounded sum" and REQ-DEC-011's "M₀₁ formed …", and it breaks the oracle of
    `crates/kernel/tests/qa_TASK-M2-02_m01_sum.rs` (`qa_m01_one_rounded_sum`, `qa_m01_crossings`: "its tag is that
    sum's"), a qa file that only a ruling may change (R-290).
  - (c) Drop the control for `M₀₁`, and keep the form's proof (physics review) and the cross-backend comparison at the
    onset crossings. REQ-VAL-017's control is then met by whichever superseded form does flip on test 12's inputs, or
    reworded to say which.
- **Applied meanwhile:** none. TASK-M2-48 is not built. The form planned is unaffected by the choice between (a) and
  (c): `M₀₁` as the one rounded sum, formed from mantissas in `[1, 2)` with integer exponents (`kernel::canon`'s `Wide`
  addition, R-547's pattern). Only the controls wait.
- **Waits:** TASK-M2-48's two acceptance controls (REQ-DEC-011, REQ-VAL-017).
- **30 Oct 2026:** Decided by the orchestrator per R-369 (a test decision), option (a); see PR (TASK-M2-48). The
  control is the folded-and-fused evaluation, `fma(t₇, t₁₃, m₀)` fused at f32 and f64 on the CPU, at TASK-M2-02's
  onset crossings, which decoder test 12's inputs gain; REQ-VAL-017's emulation list gains "contracted (fused)";
  REQ-DEC-011's and REQ-VAL-017's verify text and TASK-M2-48's two acceptance lines drop "unfused" and "on lavapipe".
  The tag rule, `ε_M01` (REQ-DEC-039) and R-547's form are unchanged. Nothing waits.

## RQ-622: Five statistical or qualitative map targets whose verify names a Result at f64: are they scored on tier (a), the f32 GPU explorer, as R-649's confirmed reading says, or on the f64 Result their verify names *(physics; R-649 item 2 and the owner's confirmation of 30 Oct 2026; `docs/design/principia_dd_replication.md` § 12.2; REQ-VAL-308, REQ-VAL-309, REQ-VAL-311, REQ-VAL-312, REQ-VAL-313; and B-2, REQ-VAL-246, added below; context: TASK-M9-33, TASK-M9-34, TASK-M9-35, TASK-M9-16)*

*Queued 30 Oct 2026 by the fix round of R-649's port (PR #342), applying the owner's confirmation of the tier reading.
For the owner: yes, it is physics where the choice changes which instrument a target is scored on, and the corpus
does not settle it (R-369). Numbered RQ-622: the number before it is already held by the branch
`task/TASK-M2-48` (`plan/OPERATIONS.md` § "Merging", **Numbers**).*

- **File, section:**
  - `decisions.md` § "R-649 — …", the owner's words, item 2: "(a) GPU explorer: fast, f32, interactive; expected to
    agree statistically or qualitatively only, with error_ratio flagging what it can't resolve; (b) CPU IC inspector:
    f64/high precision; exact targets (collision points, periods, converged states) bind here."
  - The same entry, his confirmation of 30 Oct 2026: "Tier reading confirmed: statistical targets are scored on tier
    (a), exact targets on tier (b)."
  - `decisions.md` § "R-572 — Result: a view's region run at full precision, f32 or f64 only, on a uniform grid, as a
    batch job in the app or by `prin batch` …".
  - `plan/requirements.yaml`, REQ-VAL-308 (AT-D2), verify: "one Result per mass set (R-572), f64, uniform; each orbit's
    pixel against REQ-VAL-137's threshold"; REQ-VAL-309 (OC-4 off the Šuvakov plane), verify: "Result runs (R-572) at
    f64; the island criterion read from the payload; agreement at REQ-VAL-310's level"; REQ-VAL-311 (OC-4 on the
    plane), verify: "as OC-4"; REQ-VAL-312 (AT-D1), verify: "a Result (R-572) at f64; reviewed by the physics
    reviewer"; REQ-VAL-313 (AT-D3), verify: "as AT-D1, per value of m₃".
- **The gap:** the five are statistical (OC-4 on and off the plane) or qualitative over a map (AT-D1, AT-D2, AT-D3), so
  the confirmed reading scores them on tier (a), the f32 interactive GPU explorer. Their verify, written before R-649,
  scores them on a Result at f64, a batch run that is neither the interactive f32 explorer nor the CPU IC inspector.
  Changing the verify to f32 changes what is scored; keeping it scores a statistical target off tier (a).
- **The question:** on what are these five scored.
- **Options seen:** (a) on tier (a): the verify's run becomes the GPU explorer's f32 output, and the f64 Result is
  dropped from the scoring; (b) the verify as written: the f64 Result is the scoring run, recorded as a stated
  exception to the tier rule; (c) both: scored on tier (a), with the f64 Result run beside it and recorded as
  information (an explained divergence between the two would be a precision-tier finding); (d) another, stated by the
  owner.
- **Applied meanwhile:** the five are listed as tier (a) by the rule, "its f64 Result: RQ-622", in § 12.2 and in their
  notes; their verify is unchanged. Nothing in M0 to M8 depends on it.
- **Added (30 Oct 2026, fix round of PR #342, Codex thread on § 12.2's B-2 row): B-2, REQ-VAL-246.** The same
  question for one statistical target whose statement names the CPU:
  - `plan/requirements.yaml`, REQ-VAL-246 (B-2), statement: "The Lyapunov exponent of Burrau's problem, measured with
    the CPU FTLE, must be compared loosely with λ₁₀ = 0.101 ± 0.005 (τ_L = 4.3 ± 0.2), recorded as a fit over every ζ,
    relativistic runs included, and not a Newtonian-only number (B-2)." Its compare is "statistical (loose)", so the
    confirmed reading puts it on (a), and § 12.2's table row reads "(a)".
  - `docs/design/principia_dd_replication.md` § "12.2 Tiers": "A single chaotic trajectory set beside a figure (AA-4,
    V-6, SP-1's structural comparison) is (b)." B-2's published λ₁₀ is "fitted over all ζ, relativistic runs
    included" (REQ-VAL-246's note): an FTLE fit over many runs from one start, the Burrau pixel, with a statistical
    compare. Whether the clause reaches B-2 is part of the question, not a given.
  - The gap: by its compare B-2 is (a), the f32 GPU explorer, by the owner's confirmed reading; its statement names the
    CPU FTLE. Which instrument measures the exponent changes the number scored.
  - The question: does § 12.2's single-trajectory clause reach B-2, a fit over many runs from one start; and is B-2
    scored on (a), on the CPU FTLE its statement names, or on both.
  - Options seen: (a) on tier (a), the GPU explorer's FTLE, with the statement's "measured with the CPU FTLE" changed
    to match; (b) on the CPU FTLE, as the statement says, recorded as a stated exception to the tier rule or as the
    single-trajectory clause's reach; (c) both: scored on tier (a), with the CPU FTLE run beside it and recorded as
    information (an explained divergence between the two would be a precision-tier finding); (d) another, stated by
    the owner.
  - Applied meanwhile: § 12.2's row reads "(a); its CPU FTLE: RQ-622"; REQ-VAL-246's statement is unchanged and it is
    held on this entry; TASK-M9-16's note and TASK-M9-37's "not scored" list name the hold. TASK-M9-16 still scores
    CH-1, B-1 and V-6.
- **Waits:** the scoring of REQ-VAL-308, REQ-VAL-309, REQ-VAL-311, REQ-VAL-312 and REQ-VAL-313 in M9 (TASK-M9-33,
  TASK-M9-34, TASK-M9-35), and any peek of them; and B-2's (REQ-VAL-246, TASK-M9-16), and any peek of it.
- *31 Oct 2026, R-661:* the quoted "expected to agree statistically" is a hypothesis Principia will measure, and no paper establishes it. RQ-649 asks what a score on tier (a) says meanwhile; this entry's question stands.

## RQ-630: R-554 L3's and the chart contract's summary sentences on Lean each axis say `N_a` and `N_b` vanish together on a tilted view only at the corner, where L3's own closed form also gives both zero at Lean `(±180, 0)`, `(0, ±180)` and `(±180, ±180)` *(conflict: a ruling's summary against its own formula; R-554 L3, R-474 R2; `docs/contracts/principia_chart_decoder_contract.md` § "Repeats and the canonical point"; the physics review of PR #349 (5710611197), P2; REQ-SCHED-101; context: TASK-M2-36, TASK-M8-56)*

*Queued 30 Oct 2026 by TASK-M2-36's fix round 2, on the physics review of PR #349. A wording conflict, not a physics
question: the code follows the formula, and the physics reviewer agrees it is the physically consistent reading.*

- **File, section:**
  - `decisions.md` § "R-554 — …", L3: "under Lean each axis `N ∝ (−sin τ₁ cos τ₂, −cos τ₁ sin τ₂, cos τ₁ cos τ₂)`,
    so on a tilted view they vanish together only at the corner `|τ₁| = |τ₂| = 90°`, where there is no normal and
    RQ-471 applies no fold, so no pole map either".
  - The same L3, before it: "the pole maps' condition `N_a = 0` and `N_b = 0` is decided exactly on the stored angles
    from each map's closed form".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Repeats and the canonical point": "decided exactly on the
    stored angles (Tip as sheet `m == 180`; Lean each axis only untilted, its one-target corner ray-degenerate with no
    fold; two targets iff `τ₁, τ₂ ∈ {0, ±180}`)".
  - `decisions.md` § "R-474 — …", R2: "A single tilt is identical under both maps."
- **The conflict:** L3's formula gives `N_a = −sin τ₁ cos τ₂ = 0` and `N_b = −cos τ₁ sin τ₂ = 0` whenever
  `sin τ₁ = sin τ₂ = 0`, i.e. at Lean `(±180, 0)`, `(0, ±180)` and `(±180, ±180)`, all tilted views, with
  `N = (0, 0, cos τ₁ cos τ₂) ≠ 0`. There the plane is mirrored and `d ≡ 0`, so the pole maps keep `d`. Lean with one
  angle at ±180 is the same rotation as Tip as sheet at `m = 180` (R-474 R2), where the contract applies the pole
  maps. The summary sentences ("only at the corner", "only untilted") say otherwise.
- **The question:** whether the summary sentences are reworded to match the formula ("Lean each axis iff
  `sin τ₁ = sin τ₂ = 0`, its one-target corner ray-degenerate with no fold"), or the formula's other zeros are excluded.
- **Options seen:** (a) reword L3's and the contract's summaries to the formula's zeros (the code's reading);
  (b) keep the summaries and exclude Lean's ±180 zeros from the pole maps, so that Lean `(180, 0)` and Tip as sheet
  `(180, 0)`, the same rotation, count differently; (c) another, stated by the owner.
- **Applied meanwhile:** the formula, as L3's "decided exactly on the stored angles from each map's closed form" says:
  `TiltSpec::normal_zero` (`crates/kernel/src/chart/tilt.rs`) gives `[true, true]` at Lean `(±180, 0)` and
  `(0, ±180)` (TASK-M2-36, PR #349).
- **Waits:** nothing; the summaries' rewording only.

## RQ-631: On a tilted shape sphere, the spherical map's period 2 in `t` makes `(s + k, t + 2j)` the same system as `(s, t)` wherever `N_a·k + 2·N_b·j = 0` for whole `(k, j) ≠ 0`, which R-554 L3's rule counts twice *(physics; silence; R-554 L3, RQ-472 (a), R-408; `docs/contracts/principia_chart_decoder_contract.md` § "Repeats and the canonical point"; the physics review of PR #349 (5710611197), P3; REQ-CHART-062, REQ-SCHED-101; context: TASK-M2-36, TASK-M8-56)*

*Queued 30 Oct 2026 by TASK-M2-36's fix round 2, on the physics review of PR #349. For the owner: yes, it is physics
where the choice changes the result (which pixels count) and the corpus does not settle it (R-369).*

*30 Oct 2026: Ruled by the owner, R-655 (option (a): "#349 (a): extend the repeat rule to t's period 2, with
tests.").*

- **File, section:**
  - `decisions.md` § "R-554 — …", L3: "the pole maps' condition `N_a = 0` and `N_b = 0` is decided exactly on the
    stored angles from each map's closed form".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Repeats and the canonical point": "On a tilted view a
    repeat is the same system only where the type group's map leaves the tilt's hidden offset `d` unchanged"; "the pole
    maps apply on a tilted view iff `N_a = 0` and `N_b = 0` for chart_reference §3.3's `N`, … and otherwise the view
    is affine along that axis and draws no image (R-554, ruling RQ-472 (a))".
  - `decisions.md` § "R-408 — Area statistics count each system once, …".
- **The silence:** the spherical map repeats every 1 in `s` (θ's period) and every 2 in `t` (two pole maps and θ's
  period composed): `n(s, t + 2) = n(s, t)`. On a tilted view `d` depends on `(a, b)` through `N_a·a + N_b·b`, so
  `(s + k, t + 2j)` keeps `d`, and is the same system as `(s, t)`, whenever `N_a·k + 2·N_b·j = 0` for whole numbers
  `(k, j) ≠ 0`. Cases: `N_b = 0` with `N_a ≠ 0` (e.g. `τ₁` alone), where `t` and `t + 2` repeat; Lean `45°/45°`
  (`N_a = N_b`), where `(s − 2, t + 2)` repeats `(s, t)`. The ruled rule ("pole maps iff `N_a = N_b = 0`, otherwise
  affine") counts each such pair twice, against R-408. They lie two chart units out, so a view reaches them only zoomed
  out.
- **The question:** whether these lattice repeats are folded, or the ruled rule stands with the exception stated.
- **Options seen:** (a) fold the lattice of repeats: a pixel counts only at its canonical point under every
  `(k, j)` with `N_a·k + 2·N_b·j = 0`, decided exactly on the stored angles; (b) keep the ruled rule and state the
  exception in L3 and the contract (such pixels counted twice, two chart units out); (c) another, stated by the owner.
- **Applied meanwhile:** the ruled text (R-554 L3): `Tilted::pixel` (`crates/kernel/src/chart/tilt.rs`) uncounts a
  pixel past θ's period iff `N_a = 0` and one over a pole iff `N_a = N_b = 0`, and counts the lattice repeats
  (TASK-M2-36, PR #349).
- **Waits:** nothing built waits; a fold, if ruled, is a follow-up to TASK-M2-36's `Tilted::pixel` and TASK-M8-56.

## RQ-632: Which cuts carry `a` and `b`: two arcs ending at a Lagrange pole cannot reproduce the published classes *(physics; R-125; `docs/contracts/principia_symbolic_dynamics_contract.md` § 1; REQ-PAY-070; context: TASK-M3-16)*

*Queued 30 Oct 2026 by TASK-M3-16, transcribing the convention under R-125. For the owner: yes, it is physics where
the choice changes the result (the word every sample stores) and the corpus does not settle it (R-369).*

- **File, section:**
  - `docs/contracts/principia_symbolic_dynamics_contract.md` § "1. Generator ↔ branch-cut convention": "Fix,
    normatively: which two branch cuts correspond to generators `a` and `b` (arc from which collision point to the
    Lagrange reference pole); the crossing-direction sign convention (which crossing direction is the generator vs its
    inverse) — must match the half-open sign convention in payload §3 / the integrator contract."
  - `decisions.md` § "R-125 — …": "Its convention (the a/b branch-cut assignment and the crossing sign) is transcribed
    from the literature (Montgomery; Šuvakov–Dmitrašinović) and verified by REQ-VAL-043 against the published braid
    classes."
  - The literature, `docs/incoming/2026-10-25_replication/papers/`: Šuvakov & Dmitrašinović, PRL 110, 114301 (2013),
    p. 1: "A stereographic projection of this sphere onto a plane, using one of the punctures as the "north pole"
    effectively removes that puncture to infinity … That leads to the aforementioned free group on two letters (a, b),
    where (for definiteness) a denotes a clockwise full turn around the right-hand-side puncture, and b denotes the
    counterclockwise full turn around the other puncture". Dmitrašinović, Hudomal, Shibayama & Sugita, J. Phys. A 51,
    315101 (2018), Appendix B ("Montgomery's topological identification method"), Fig. 7: "Three two-body collision
    points (solid red) lie on a meridian (dashed circle), with one of them being at the north pole"; and: "a simple
    loop around the third ("infinite") puncture on the shape sphere corresponds to aB". Its Appendix C: "make the
    substitution a = 12, A = 21, b = 32, B = 23", the digits being "crossings of the equator on the shape sphere, in
    one of three corresponding segments on the said equator, where the index of the body passing between the other two
    is used as a symbol". (Montgomery's own paper, Nonlinearity 11, 363 (1998), is not in the corpus; Appendix B is
    the authors' account of his method.)
- **The gap:** the literature's "north pole" is a collision point, not a Lagrange pole. Two cuts from two collision
  points to a Lagrange pole `L±` (an equilateral shape, not a puncture) leave the cut sphere a punctured disc, not a
  disc: a loop around the third collision point crosses neither cut and reads as the empty word, though the
  literature's class is `aB` (the BHH family's); and a small loop around the pole crosses both and reads as a
  nonempty word, though it is contractible. So no assignment of `a` and `b` to the arcs §1 names reproduces the
  published classes (REQ-VAL-043). The literature fixes the generators as loops, with Appendix C's reading in
  syzygies; it names no cuts. Two smaller points ride on the choice: the literature's bodies 1, 2, 3 against ours 0, 1, 2
  (R-630 labels the Šuvakov plane's middle body our body 1), and which hemisphere is the literature's upper half-plane,
  the start of `a = 12`, which its figures alone fix and on which the PRL and the gallery differ for 6 of 15 orbits by
  the `(a, b) ↔ (A, B)` exchange (`docs/incoming/2026-10-26_belgrade_gallery/README.md`, cross-check item 3).
- **The question:** which two cuts carry `a` and `b`, the body labelling they are read in, and which side of each is
  negative, so that the positive sense (`d < 0`, then `d ≥ 0`; integrator contract Part 5) is the generator.
- **Options seen:**
  - (a) **The literature's own cuts, on the equator** (Appendix C). With the literature's body 1 and body 2 meeting at
    the puncture `a` circles and bodies 2 and 3 at `b`'s, the third collision, of bodies 1 and 3, at infinity: cut `a`
    is the equator segment where body 1 is between the others, from the 1–2 collision to the 1–3; cut `b` the segment
    where body 3 is between, from the 2–3 collision to the 1–3. A crossing of segment 1 out of the hemisphere where `a`
    starts is `a`, into it `A`; segment 3 likewise for `b`, `B`; segment 2 is not a cut. The word read so is Appendix
    C's syzygy reading letter for letter. `d` is the triangle's orientation (`n_w`, `L⁺` positive), and which segment
    is crossed is a second, comparison-only test of which body is between at the crossing. The two cuts lie on one
    great circle, so one substep crosses at most one of them, and the `τ`-sort never meets two.
  - (b) **Arcs through the Lagrange poles, to the third collision.** Cut `a` from the 1–2 collision through `L⁺` to the
    1–3 collision, cut `b` from the 2–3 collision through `L⁻` to the 1–3: each isotopic to (a)'s segment with no
    puncture between, so a closed orbit reads the same class as under (a). It keeps §1's Lagrange poles; each cut is
    two quarter-meridians, each with a hemisphere test; the cuts meet only at the 1–3 collision, near which one substep
    can cross both, so the `τ`-sort is live there.
  - (c) **Three cuts, one from each collision to `L⁺`** (§1's arcs, made a disc by the third), a crossing of the third
    appending the punctured-sphere relation's two letters: needs §2's relation, which R-125 put in v2.
  - (d) Another, stated by the owner.
  For each: the body map (literal, `1, 2, 3 → 0, 1, 2`, or R-630's, the middle body ours 1), and the side `a`
  starts on (proposed: `L⁺`'s hemisphere, bodies 0 → 1 → 2 anticlockwise, dd_integrator §3.7), which REQ-VAL-043's
  comparison up to the `(a, b) ↔ (A, B)` exchange cannot tell apart.
- **Applied meanwhile:** none. TASK-M3-16's detector (`crates/kernel/src/word/crossing.rs`) takes each cut's signed
  distances as its input and applies the integrator contract's τ tie rule (Part 5), which does not depend on the cuts'
  shape; no cut geometry is built and nothing reads a word from a trajectory.
- **Waits:** the signed distances from the state and §1's convention (REQ-PAY-070, TASK-M3-16), and REQ-VAL-043
  (TASK-M3-30).
- **30 Oct 2026:** Decided by the orchestrator per R-369 (consequence of R-125), option (a); confirmed at the M3 gate;
  see PR (TASK-M3-16). The cuts are the collision equator's segments with body 0 between (`a`, `P₀₁`–`P₀₂`) and body 2
  between (`b`, `P₁₂`–`P₀₂`), Appendix C's bodies 1, 2, 3 ours 0, 1, 2 (R-630); `d = −(r₁ − r₀) × (r₂ − r₀)`, so `−n_w`,
  and a generator crosses from `L⁺`'s hemisphere into `L⁻`'s ("`d` is `n_w`, `L⁺` positive" in option (a) above is
  corrected); the segment classifier is an R-72 definition (REQ-PAY-113). Applied in symbolic_dynamics_contract §1 and
  integrator contract Part 5. Nothing waits on it; REQ-VAL-043 stays TASK-M3-30's.
- **31 Oct 2026:** Physics review 5720379930 of PR #358 holds option (a) under R-369, the owner confirming it at the M3
  gate (R-125, REQ-PAY-070); it re-derives the sign from Appendix C and names two sub-choices that are not literature
  and change stored words, the hemisphere and the outer-body order, for the gate record. It reopens this entry if
  REQ-VAL-043 shows the kernel's substeps too coarse for (a). **Provenance:** §1's first read of the ten orbits was a
  throwaway script of 30 Oct, never in the repo. What stands in for it is `fixtures/crossings/prl_orbits.py`, adapted
  from it; `crossing_body_relabelling` and `qa_m316_prl_fixture_against_appendix_c`, which check the outer-body claim.

## RQ-634: On a tilted shape sphere a pole map's image also keeps `d` on lines of measure zero, `N_a(½ + k) + N_b(2j − 2t) = 0`, which no ruling names: R-554 L3 counts such a pixel and its image as two systems, and R-408 counts each system once *(physics; silence; R-554 L3, R-655, R-408; `docs/contracts/principia_chart_decoder_contract.md` § "Repeats and the canonical point"; the physics review of PR #349 (5719740344), P2; context: TASK-M2-36, TASK-M8-56)*

*Queued 30 Oct 2026 by TASK-M2-36's fix round, on the physics review of PR #349. For the owner: yes, it is physics
where the choice changes the result (which pixels count) and the corpus does not settle it (R-369).*

- **File, section:**
  - `decisions.md` § "R-554 — …", L3: "the pole maps' condition `N_a = 0` and `N_b = 0` is decided exactly on the
    stored angles from each map's closed form".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Repeats and the canonical point": "the pole maps apply on a
    tilted view iff `N_a = 0` and `N_b = 0` for chart_reference §3.3's `N`, decided exactly on the stored angles", and
    "On a tilted view a repeat is the same system only where the type group's map leaves the tilt's hidden offset `d`
    unchanged".
  - `decisions.md` § "R-408 — Area statistics count each system once, …".
  - `decisions.md` § "R-655 — …": "a pixel `(s + k, t + 2j)` with `N_a·k + 2·N_b·j = 0` for integers `(k, j) ≠ 0` is the
    same system as `(s, t)`", which covers the type group's translations and not its pole maps.
- **The silence:** a pole map's image `(s + ½ + k, −t + 2j)` has the same `n̂` as `(s, t)`. On a tilted view it also
  keeps `d` on the line `N_a(½ + k) + N_b(2j − 2t) = 0`. Under Lean `45°/45°`, `(a, ¼)` and `(a + ½, −¼)` have one shape
  and one `d`; with `N_a = 0`, `N_b ≠ 0`, the pole rows `t = j` are one system at each `d`. So the contract's "same
  system only where the map leaves `d` unchanged" holds on these lines, while its "iff `N_a = 0` and `N_b = 0`" and
  R-554 L3 make the view affine there and count both. No ruling names these lines.
- **The question:** whether these lines are identified, or R-554 L3 stands with the exception stated.
- **Options seen:** (a) keep L3 and state these lines as a named exception of measure zero, each pixel counted, as
  § 3.3 states the circle's identification; (b) fold them as R-655 folds its lattice, one count per system, decided
  exactly on the stored angles (the line's `t` is a computed coordinate, so what "exactly" means on it needs stating);
  (c) another, stated by the owner.
- **Applied meanwhile:** the ruled text, R-554 L3: `Tilted::pixel` (`crates/kernel/src/chart/tilt.rs`) applies no pole
  map unless `N_a = N_b = 0`, so such pixels count as distinct (TASK-M2-36, PR #349). No code changes for this entry.
- **Waits:** nothing built waits; a fold, if ruled, is a follow-up to TASK-M2-36's `Tilted::pixel` and TASK-M8-56.

## RQ-644: Whether the record should stop calling Boekholt & Portegies Zwart 2015's method an analogue of "never discard", given that its unbiased result is stated with the worst runs removed *(a conflict between a paper and two lines of our record; context: task 15e, BZ-1)*

*Queued 31 Oct 2026 by task 15e. Wording of a record; no target or code depends on it.*

- **File, section:**
  - `docs/incoming/2026-10-25_replication/catalogue.md` § 6.1: "A direct analogue of the project's "never discard a copy" and `error_ratio` rules".
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, BZ-1: "Its method is what REQ-VAL-199 and "never discard a copy" already apply".
  - `docs/design/principia_honesty_precision.md` line 278: "**Never discard:** a failed or budget-exhausted run is shown as such, with its end time (R-563)."
  - The paper, abstract: "unbiased, as long as the time-step parameter, η ≤ 2⁻⁵ and when simulations which violate energy conservation by more than 10% are excluded"; passages in `docs/incoming/2026-10-31_fetch_list/README.md`.
- **The question:** whether to add a dated note to those two lines saying the analogue is the paper's ensemble comparison and not its energy cut.
- **Options seen:** (a) add the note, and if BZ-1's method is ever run, report the whole ensemble and the cut side by side, as the paper's Fig. 14 does; (b) leave the lines as they are.
- **Applied meanwhile:** the lines are unchanged; the conflict is stated in the fetch list's README. BZ-1 stays out of scope as a replication.
- **Waits:** nothing.
- *31 Oct 2026: Ruled by the owner, R-661: "Fix the three overstatements you found (PLAN's BZ-1 row, catalogue.md §6.1, the
  replication instruction) via the queue, with Removed-lines notes." Those lines are reworded (RQ-646), and BZ-1 is reopened
  on the cold sets, any cut shown beside the uncut ensemble with the count removed.*

## RQ-645: Two citations on the owner's 31 Oct fetch list do not verify: "Barrio et al. 2009, PRE 79, 036208" and "Tanikawa & Mikkola 2008, PASJ 60, 883" *(a missing source; context: task 15e)*

*Queued 31 Oct 2026 by task 15e. For the owner, who may know the paper meant.*

- **File, section:**
  - `docs/incoming/2026-10-31_fetch_list/README.md` § "The sources", the last row and the Tanikawa & Mikkola row.
  - Crossref's record of doi:10.1103/PhysRevE.79.036208: Barré & Yamaguchi 2009, "Small traveling clusters in attractive and repulsive Hamiltonian mean-field models", not a three-body paper.
  - doi:10.1093/pasj/60.4.883 returns 404 at doi.org. The paper we already have under that authorship and year, arXiv:0802.2465, has an ASP Conference Series header, and `docs/incoming/2026-10-25_replication/catalogue.md` § 1.4 calls it a precursor of PASJ 67, 115 (2015).
- **The question:** which papers were meant. Neither a Crossref search nor a web search found a Barrio 2009 paper in PRE on the planar three-body problem; the nearest is Barrio, Borczyk & Breiter 2009, "Spurious structures in chaos indicators maps", Chaos, Solitons & Fractals 40, 1697, doi:10.1016/j.chaos.2007.09.084, which was not read, so it is not offered as the same work.
- **Options seen:** (a) the owner gives the paper, if it exists; (b) both drop off the list, with arXiv:0802.2465 standing for Tanikawa & Mikkola 2008.
- **Applied meanwhile:** nothing fetched for Barrio; arXiv:0802.2465 stands for Tanikawa & Mikkola 2008.
- **Waits:** nothing; no target names either citation.

## RQ-646: Three lines of our record overstate Boekholt & Portegies Zwart 2015 *(a conflict between a paper and our record; R-661; `docs/incoming/2026-10-31_precision_set/README.md`; context: BZ-1, REQ-VAL-199)*

*Queued and closed 31 Oct 2026, by R-661.*

- **File, section:**
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, BZ-1: "Its method is what REQ-VAL-199 and "never discard a copy" already apply"; "out of scope as a replication: 3D Plummer ICs".
  - `docs/incoming/2026-10-25_replication/catalogue.md` § 6.1: "the ensemble statistics agree (KS tests) if the step parameter is ≤ 2⁻⁵ and runs with |dE/E| > 0.1 are rejected".
  - `docs/incoming/2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md` § C: "This is the empirical basis of the f32/f64 honesty story, and of "never discard a copy"".
  - The paper, abstract: "unbiased, as long as the time-step parameter, η ≤ 2⁻⁵ and when simulations which violate energy conservation by more than 10% are excluded"; Fig. 6's caption: "For η < 2⁻⁵, the distributions are not significantly different."
- **The conflict:** the first line reads a cut as never discarding; Fig. 14's side-by-side is compatible, and the "unbiased" claim holds only after the cut. "3D Plummer" does not hold of the cold sets. The second merges the KS claim with the error-distribution claim. The third cites a paper with no f32 run as the basis for f32.
- **Options seen:** reword, with the old text kept; or leave them.
- **Applied meanwhile:** the lines stand until the ruling below.
- *Ruled by the owner, R-661: reworded in PLAN.md and `catalogue.md`, each with a "Was"; the 25 Oct instruction is a dated record, not restated in force elsewhere, so its text stays and a bracketed correction follows it.*

## RQ-647: Whether to re-size REQ-VAL-199's 256 starts by the power argument, and at what cost *(the owner's other option; R-661 item 3; REQ-VAL-199; `docs/incoming/2026-10-31_precision_set/README.md` § "The power calculation"; context: TASK-M8-76)*

*Queued 31 Oct 2026 for the owner, by R-661's port. Applied meanwhile: state the effect size (R-369). It changes what is run, so it is his.*

- **File, section:** `decisions.md` § R-661, item (3): "REQ-VAL-199: re-size its 256 starts by the same power argument, or state the effect size it can detect." REQ-VAL-199: "over all 256 chaotic starts"; `decisions.md` § R-610: "The cap stays at 200 until long horizons are measured for cost."
- **What 256 can detect:** a two-sample KS at 5%, D_c = 1.358 √(2/256) = 0.12; at 80% power, a fraction difference of 0.12 at p = 0.5 and 0.074 at p = 0.1. The gate's own statistic, a median of ratios, has a 95% half-width of 0.154 of the ratios' spread if they are normal; distribution-free, ranks 112 to 145, the quantiles 0.44 to 0.57, with c bounds of one direction moving the affected end out by c ranks, unbounded at 112 or more. Differences of a few percent in outcomes are out of its sight, and the gate never compares outcomes.
- **Options seen, each in starts × three runs (f32, f64, f64′) × T = 200:**
  - (a) keep 256 and state the effect size: 1.5 × 10⁵ time units, D_c 0.12. The gate stays a test of the parting ratio, and outcome statistics are BZ-1C's.
  - (b) n = 4,096: 16 times the runs, 2.5 × 10⁶ time units, D_c 0.030, median half-width 0.038 of the spread.
  - (c) n = 10⁴: 39 times the runs, 6 × 10⁶ time units, D_c 0.019, median half-width 0.025 of the spread.
- **The cost, as far as it is known:** the multiples above are exact, the CPU seconds are not: R-610 sets a cap "until long horizons are measured for cost" and gives no budget figure, and no time per unit is measured here, so none is invented. (b) and (c) are also a change to a statement R-600 fixed ("all 256 starts").
- **Applied meanwhile:** (a). REQ-VAL-199 states its effect size and keeps its size.
- **Waits:** nothing. A resize, if ruled, changes REQ-VAL-199 and TASK-M8-76.

## RQ-648: Whether the trust line's two marks should keep citing Boekholt & Portegies Zwart 2015, whose 10⁻³ and 10⁻¹ measure something other than a shape-sphere chord *(physics; a citation against the paper; R-574; `docs/design/principia_honesty_precision.md` § 3; R-661's search; context: TASK-M8-76)*

*Queued 31 Oct 2026 by R-661's port. Not an f32 claim, so R-661's rewording does not reach it; the text is the owner's (R-574), so it is left.*

- **File, section:**
  - `docs/design/principia_honesty_precision.md` § 3 and § 5 (the owner's words, R-574): "**Faithful until**: f32 and f64 within a shape-sphere chord of **10⁻³**, the reliability threshold of Boekholt & Portegies Zwart (2015, 2020, 2023), stated as keeping the difference in the linear regime"; "**Parted at**: chord **0.1**, the decorrelation threshold (separation at the system's own scale: Boekholt 2015, Hernandez et al. 2020)".
  - The 2015 paper, § 2.3: converged means the first p decimal places of every phase-space coordinate agree, usually p = 3; § 5.4: a normalised phase-space distance δ of about 10⁻¹ separates same-trajectory from diverged.
- **The question:** both 2015 numbers are in phase space, for f64 against a converged code, and the marks are chords on the shape sphere for f32 against f64. Are the marks to read "measured here, at values chosen by analogy with", rather than "the threshold of"?
- **Options seen:** (a) leave the words; (b) add a dated line that the values are chosen by analogy, not transferred, the marks themselves unchanged (they are measured, shown and editable).
- **Applied meanwhile:** (a). The values, 10⁻³ and 0.1, do not change under either.
- **Waits:** nothing.

## RQ-649: What a statistical target scored on tier (a) alone says, while f32's statistics are an unmeasured hypothesis *(physics; R-661 against R-649 item 2; context: BZ-1C, M9's statistical targets)*

*Queued 31 Oct 2026 by R-661's port. It blocks nothing.*

- **File, section:**
  - `decisions.md` § R-649, item 2, the owner's words: "(a) GPU explorer: fast, f32, interactive; expected to agree statistically or qualitatively only"; his clarification: "External targets bind tier (b) unless they are statistical, where tier (a) is expected to agree statistically."
  - `decisions.md` § R-661: "it is a HYPOTHESIS that Principia will measure, and no paper establishes it."
  - RQ-622, on five statistical targets whose verify names an f64 Result.
- **The question:** R-661 makes tier (a)'s agreement a hypothesis, BZ-1C its test. Until BZ-1C has measured it, a statistical target scored only on f32 can pass or fail for two reasons: the physics, or f32. The choice changes how those results read.
- **Options seen:** (a) score on tier (a) as ruled, each result labelled "f32; agreement with f64 not yet measured", until BZ-1C reports; (b) as (a), and run f64 beside it on a sample sized by the same calculation; (c) statistical targets wait for BZ-1C.
- **Applied meanwhile:** nothing is scored yet, so nothing is applied. R-649's tier assignment is not changed here.
- **Waits:** nothing built.

## RQ-652: Which event BZ-1C's third side is compared on: BPZ's escape criterion or Principia's *(physics; R-661; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, BZ-1C; `docs/design/principia_dd_replication.md` rule 9; R-29; context: BZ-1C)*

*Queued 31 Oct 2026 by R-661's port (physics review 5724115938 of PR #363, F2). The two ids before this one are named by `docs/incoming/2026-10-28_external_sources/README.md`, so this takes the next free id (applied per R-369).*

- **File, section:**
  - Boekholt & Portegies Zwart 2015 § 4: "We stop the simulations when the system is dissolved into a permanent binary and an escaper"; the escaper has "E > 0", "r > 2 rvirial" and "r · v > 0".
  - `docs/design/principia_dd_replication.md` rule 9: "Record the escape criterion as data, with its parameters and its clock."
  - The records hold each run's final state and whole-number stop time only, on BPZ's criterion. R-29's criterion cannot be applied to a state they don't hold.
- **The question:** f32 against f64 can use one classifier. The records can only be read on BPZ's. Which event do the three-way lifetimes and end states use?
- **Options seen:** (a) apply BPZ's criterion to all three sides, testing Principia's states as BPZ's code did, and report R-29's beside it for f32 against f64 only; (b) R-29 for f32 and f64, the records as they are, the difference labelled and not scored as precision; (c) both for the two arms, so the criteria's own difference is measured on the same runs.
- **Applied meanwhile:** none; BZ-1C is a proposal. Both criteria are recorded whichever is chosen.
- **Waits:** BZ-1C's scoring against the records. f32 against f64 does not wait.

## RQ-653: Which energy a BZ-1C energy cut uses: regularised or Cartesian *(physics; R-661; R-566; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, BZ-1C; context: BZ-1C)*

*Queued 31 Oct 2026 by R-661's port (physics review 5724115938 of PR #363, F2).*

- **File, section:**
  - Boekholt & Portegies Zwart 2015 § 6.1: "a relative energy conservation |∆E/E| > 1, or |∆E/E| > 0.1", on Hermite's Cartesian state.
  - `decisions.md` § R-566: "after a close passage the Cartesian read-out is 280× to 7100× worse than the" regularised energy.
  - `decisions.md` § R-661: "show any energy cut beside the uncut ensemble with the removed count".
- **The question:** the removed count depends on the energy. The Cartesian one is like-for-like with BPZ's but is R-566's diagnostic; the regularised one is what Principia's gates read. At what stage it is read also differs.
- **Also open, inside (a):** R-566 does not say whether an escape stop is an "event". Its evidence is a collision and a run parked at one, and `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)" lists terminal events as "a collision, a non-finite state, the step budget run out", with escape "sampled at the boundaries". If escape counts, (a) reads only censored runs, can cut nothing among the runs that ended, and is not like-for-like. If it does not, (a) reads the Cartesian state at an escape stop, as BPZ's does. Either way a collision stop has no reading under (a).
- **Options seen:** (a) the Cartesian read-out, only from runs that did not stop at an event, as R-566 reads it, like-for-like with BPZ only if escape is not such an event; (b) the regularised energy; (c) both cuts shown, each with its count.
- **Applied meanwhile:** none; each run records both.
- **Waits:** any energy cut in BZ-1C. The uncut ensemble does not wait.

## RQ-654: How BZ-1C's starts reach the f32 GPU kernel: Path B or a new list-of-starts chart *(the human's, by `docs/design/principia_dd_replication.md` § 2; R-594 item 5; R-661; context: BZ-1C)*

*Queued 31 Oct 2026 by R-661's port (physics review 5724115938 of PR #363, F7).*

- **File, section:**
  - `docs/design/principia_dd_replication.md` § 2: ""Path B" (a known IC encoded, or injected physically, with no chart …), or "new: <name>" for a chart no task builds … each such chart is a question for the human".
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, BZ-1C: 2 × 10⁴ starts, f32 on the GPU, f64 on the CPU.
- **The question:** Path B is a known IC with no chart. Whether it can carry 2 × 10⁴ arbitrary f32 starts through the GPU kernel, as a survey dispatch does, is not stated in the corpus; and the choice bears on the result, since an encoded start's rounding is part of what f32 runs. The rest-start D-plane charts reach these starts' shapes only with a rounding of their own.
- **Options seen:** (a) Path B, if a batch of injected starts can be dispatched, f64 values rounded once to f32; (b) a new list-of-starts chart, built when BZ-1C is adopted; (c) the Anosova entry, reaching the equal-mass rest starts by encode, with its rounding measured, and no 1:2:4 until a mass point is added.
- **Applied meanwhile:** none; nothing is built.
- **Waits:** BZ-1C.

## RQ-655: R-632's "a bound" is not true of a start where neither pair has parted *(physics; R-632; `plan/requirements.yaml` REQ-VAL-199; `docs/design/principia_honesty_precision.md` § 6; found by the physics review 5724170840 of PR #363, an aside; context: TASK-M8-76)*

*Queued 31 Oct 2026. Ruled text, restated rather than introduced by PR #363, which it does not hold.*

- **File, section:**
  - REQ-VAL-199: "a pair not parted by the horizon T censored, its t_part taken as T and its start's ratio a bound kept in the median".
  - `docs/design/principia_honesty_precision.md` § 6: "that pair's `t_part` is taken as `T`, the start's ratio is a bound, it stays in the median over all 256".
- **The conflict:** with only the f64 pair unparted, the measured ratio t_part(f32)/T is an upper bound on the true one; with only the f32 pair unparted, T/t_part(f64) is a lower bound. With neither parted the ratio is T/T = 1 before dividing by the prediction, and the true ratio may lie on either side, so it bounds nothing. The gate's median and its censored count are unaffected as written; what such a start's entry means is not.
- **Options seen:** (a) leave the rule and call such a start "unresolved" in the report, counted apart from the one-sided bounds, still in the median over 256; (b) keep the word, as a known looseness; (c) a different treatment of such starts in the median, which changes the statistic.
- **Applied meanwhile:** none: R-632 and REQ-VAL-199 stand as ruled.
- **Waits:** nothing.
