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
## RQ-628: ROUNDTRIP's interior residual on a backend whose `atan2` is approximate: lavapipe's goes past `ε_phys` *(a calibration requirement, R-71; REQ-ENC-024, REQ-RENDER-025; R-297; inverse-encode contract Part 4, Part 8; context: TASK-M2-26, PR #343)*

*Queued 30 Oct 2026 from TASK-M2-26's fix round, on CI's lavapipe runs of `qa_m226_latent_slices_dark_and_within_eps_phys`.
For the owner: yes, it is REQ-ENC-024's calibration, and the options change what is built.*

- **File, section:**
  - `plan/tasks/M2/TASK-M2-26.md` § "Acceptance tests": "`cargo xtask golden roundtrip-preset` in `gpu-metal` and
    `gpu-lavapipe` — ROUNDTRIP render of the latent and (L_z, E) charts, with the fragment compiled as the backend
    compiles it (fast-math on Metal, R-297): residual below ε_phys (REQ-ENC-024) except at tagged-expected regions".
  - The same file, § "Notes": "If the residual under fragment fast-math exceeds ε_phys, that is a REVIEW_QUEUE entry,
    not a second tolerance (applied per R-204, accepted by R-358)."
  - `docs/contracts/principia_inverse_encode_contract.md` § "Part 4 — Conditioning: assert in physical units, never in
    z": "`ε_phys = k·ε_R` … one `k` for both precisions: the interior residual is rounding, the same count of `ε_R` at
    f64 and f32", and "the `ROUNDTRIP` preset (Part 8) use[s] it unchanged".
  - `decisions.md` § "R-297 — …": "fragment paths that recompute physics to check agreement (the DECODE/ROUNDTRIP
    presets, R-116) compare within a stated tolerance, a calibration requirement, not bit-exactly."
  - `crates/ledger/src/constants.rs`, `EPS_PHYS`: `k = 64`, proposed from the `encode-eps-phys` gate's CPU
    measurements (worst interior 25.64 `ε_R`, f32).
- **What was seen:** on CI's lavapipe runners (runs 41003051251 and 41003055485), slice 6 of the qa test (seed
  `0x0343_2026`, a latent slice reaching 20 along random directions), sample 458 at `(0.328, 0.453)`: untagged, ratio
  1.053 `ε_phys`. Every control is deep inside its clamp (margins 21 420, 398, 26 149, 17 297, 58 202, 281 223, 284,
  123 763 clamp-`ε`), no mirror, no clamp, no degenerate decode, so nothing should tag it. The same trip on the CPU at
  f32 gives 0.004 `ε_phys`; on Metal 0.034 (slice 6's worst untagged 0.26).
- **The cause:** the state has a light body (`m₀ = 0.0044`), so `|ρ̃| = 0.034 ≪ |λ̃| = 1.0` and
  `α = atan2(|λ̃|, |ρ̃|)` sits 0.034 below `π/2`. The decode builds `ρ̃` from `cos α`, so an absolute error `δα` in the
  encode's `α` comes back as a relative error `δα / cos α` in `ρ`, about 30× here. Mesa lowers `atan2` (for lavapipe)
  to a degree-11 odd polynomial whose leading coefficient is 0.9999793; for `|y/x| > 1` the angle is `π/2 − P(x/y)`,
  so near `π/2` it is off by about `2.07·10⁻⁵·cos α` (about 174 `ε_R` relative to `cos α`). That gives
  `Δρ/ρ ≈ 2.07·10⁻⁵`, `Δr₀ ≈ 1.05·10⁻⁵`, `/R_x = 7.7·10⁻⁶ ≈ 1.01 ε_phys`, the 1.053 CI reports. A check on Metal
  agrees: the same view with `atan2` replaced by that lowering, in WGSL, gives 1.18 `ε_phys` at sample 458 and seven
  untagged samples past `ε_phys` on slice 6. Changing each other builtin by 8 `ε_R` in turn (exp, log, tanh, atanh,
  sin, cos, pow, sqrt) leaves sample 458 below 0.27; `atan2` alone takes it to 2. WGSL's and Vulkan's accuracy for
  `atan2` is 4096 ULP, so this is a conforming backend. The ROUNDTRIP code is not at fault: it runs the generated
  encode as R-116 requires, and the pixel is interior by every margin.
- **The gap:** `ε_phys` was calibrated on the CPU, where the interior residual is rounding, and Part 4 assumes that.
  In the fragment it is also the backend's builtin accuracy (`atan2`, amplified by `1/cos α` near a light body),
  which no document bounds. The task requires `ε_phys` on lavapipe, and its note forbids a second tolerance without a
  ruling.
- **The question:** how ROUNDTRIP's interior is held to a tolerance on a backend whose builtins are only as accurate
  as WGSL requires.
- **Options seen:**
  - (a) The one Rust source gives `α` and `β` an `atan2` of its own, a stated polynomial accurate to a few `ε_R` (as
    R-437 made the poles backend-independent), so the fragment trip no longer depends on the backend's `atan2`. This
    changes the generated encode (TASK-M2-25's), and the compute kernel too unless the change is fragment-only.
  - (b) A stated fragment tolerance for ROUNDTRIP, `k_frag·ε_R` with `k_frag` calibrated on both backends (a second
    calibration requirement, R-71).
  - (c) `ε_phys` re-calibrated to cover the fragment on both backends, which loosens T2 on the CPU as well.
  - (d) The lavapipe run drops from ROUNDTRIP's acceptance, or lavapipe is held to a looser stated bound (dropping or
    changing an acceptance clause).
- **Applied meanwhile:** none. No tolerance is changed, and qa's test is unchanged.
- **Waits:** TASK-M2-26 (PR #343): its required test fails on the lavapipe runners.
- **30 Oct 2026:** Decided by the orchestrator per R-369, option (a); see PR #343; the owner's veto listed. Built in
  PR #343's fix round: at f32 the encode's `α` and `β`, the spherical and exponential inverses and the shape sphere
  decode's `β` take the kernel's own `atan2` (`crates/kernel/src/arctan.rs`), in integer arithmetic, so every backend
  gives the CPU's bits; the f64 path keeps libm's, bit for bit. No tolerance is changed, and qa's test is unchanged.

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
