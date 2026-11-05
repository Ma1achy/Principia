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

## RQ-623: The agreement presets' tolerance near a binary collision: no factor of the per-operand form covers the fragment-against-compute differences, which grow as (|rᵢ|+|rⱼ|)/rᵢⱼ, up to about 1/eps, as a pair separation falls *(a calibration requirement, R-71, and the form it is a factor of, a definition requirement, R-72; physics; REQ-COL-060, REQ-COL-006; context: TASK-M2-29)*

*Queued 30 Oct 2026 by TASK-M2-29's implementer, from its measurement. For the owner: yes, it is physics where the
choice changes what the agreement preset shows and what its gate certifies, and the corpus does not settle it
(R-369).*

- **File, section:**
  - `plan/requirements.yaml` REQ-COL-060, verify: "the proposal measures, in gpu-metal and gpu-lavapipe, the agreement
    presets' fragment-against-compute differences (|E(fragment-decode) − E₀| and the like) over the M2 charts, and
    states the tolerance that covers them".
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets", the agreement presets' bullet: "The
    fragment shaders may compile with fast-math and the compute shaders don't, by default (R-297), so an agreement
    preset compares within a stated tolerance, a calibration requirement (R-71), not bit-exactly."
  - `crates/render/src/presets/agreement.rs`, the tolerance's form TASK-M2-25 chose (applied per R-369 there):
    `|E − E₀| ≤ FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1))`, each operand's difference bounded by `FACTOR·eps·max(|x|, 1)`.
  - `docs/contracts/principia_inverse_encode_contract.md` Part 4: "every round-trip tolerance is asserted in *physical*
    space, where the composed map is well-conditioned".
  - `decisions.md` § R-544: its conditioning-derived δ exclusion near the shape sphere's collision landmarks, recorded
    as a test detail.
  - `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence": "A QUANTITY THAT DOES NOT
    CONVERGE UNDER REFINEMENT IS MEASURING THE SAMPLING, NOT THE SYSTEM." and "Set thresholds from the observed
    distribution, or from a measured gap."
- **The measurement** (Metal, Apple M3 Pro, branch `task/TASK-M2-29` at fab11679; the `decode-agreement` gate's report,
  each chart at its fixed parameters, the fragment compiled as Metal compiles it, the decode stage with compute
  fast-math off). The worst `|E − E₀|` in eps of the scale `max(|K₀|, 1) + max(|V₀|, 1)`, every chart, by lattice:
  16² per chart 7.9; 128² 15.5; 256² 20.2; 512² 60.4 (`shape_sphere`). The median and 99th percentile do not move
  (`shape_sphere` 2.41 and 4.81 eps at 128², 256² and 512²; `latent` about 0.45 and 8.0); only the worst grows. Every
  sample over 16 eps at 512² lies on `shape_sphere`'s line `v = 0.5`, the collinear equator, at `u ≈ 0.845` and
  `u ≈ 0.040`, beside a binary collision. Zooming on `u = 0.84473, v = 0.5` with 128² cells at depths 9 to 21, the
  worst is 60, 185, 2129, 8606, 19520 (smallest pair separation 9.7·10⁻⁵, `V₀ = −119.7`), 2269, 996 eps; on
  `u = 0.04004`, up to 1735. At the collision itself `V₀` is `−∞`, hatched and excluded (R-436); next to it, the
  difference grows as (|rᵢ|+|rⱼ|)/rᵢⱼ, up to about 1/eps.
- **The mechanism, measured:** the conditioning of `V = −Σ mᵢmⱼ/rᵢⱼ` in f32. A pair separation formed from positions
  of order 1 carries an absolute rounding of order `eps·(|rᵢ| + |rⱼ|)`, so `V` carries `Σ mᵢmⱼ·eps·(|rᵢ| + |rⱼ|)/rᵢⱼ²`,
  which the per-operand scale `eps·|V₀|` underestimates by `~(|rᵢ| + |rⱼ|)/rᵢⱼ`. Any two f32 compilations that round
  differently diverge by that much. It is not fast-math alone: with the decode stage compiled with compute fast-math
  on, beside the fragment, the worst near the same point is still 3086 eps. With the scale widened by that
  conditioning, `eps·(max(|K₀|, 1) + max(|V₀|, 1) + Σ mᵢmⱼ(|rᵢ| + |rⱼ|)/rᵢⱼ²)` from the decode stage's own `(m, r)`, the
  worst over every one of those zoom windows is 1.1 to 5.7.
- **The gap:** REQ-COL-060 asks for "the tolerance that covers" the measured differences over the M2 charts. In the
  form the preset compiles, no factor does: the worst grows as the samples near a collision (pitfalls § 3), so a
  factor that covers the gate's lattices (256 covers every lattice above) still draws pixels near a binary collision
  red, a disagreement that is f32 conditioning, not a translation or addressing fault. The corpus gives the
  tolerance as a calibration and says nothing of its form near a collision.
- **The question:** what the agreement presets' tolerance is where the f32 energy is ill-conditioned, beside a binary
  collision.
- **Options seen:**
  (a) the per-operand form as it is, with a factor that covers the gate's lattices (256 proposed), and the red near a
  collision stated in colour_composition § 6 as expected, as ROUNDTRIP's "bright at clamps/feasibility edges/mirror
  tie = expected" is (expected, tagged: render_contract § "Cross-check views (the seams)");
  (b) a conditioning-aware form, the factor times `eps·(max(|K₀|, 1) + max(|V₀|, 1) + Σ mᵢmⱼ(|rᵢ| + |rⱼ|)/rᵢⱼ²)` from the
  fragment decode's own `(m, r)`, written into colour_composition § 6 as an R-72 definition, physics-reviewed, and the
  factor calibrated in it (measured above at 1.1 to 5.7 near the collisions, with the decode stage's `(m, r)`; its
  value on the lattices still to be measured);
  (c) the per-operand form, and a pixel whose smallest pair separation falls under a threshold drawn in a tagged
  "ill-conditioned" style and left out of the gate, the threshold a calibration;
  (d) another, stated by the owner.
- **Recommended:** (b): it keeps the check meaningful at every finite pixel, keeps its sensitivity to a dispatch
  scramble, and its bound is the measured mechanism rather than a margin. (a) leaves false red beside every collision
  on `shape_sphere`'s equator.
- **Applied meanwhile:** none. The branch holds the measurement (each chart's worst, median and 99th percentile, and
  the excluded count, in the gate's report; the `survey` input; the gate run in `gpu-metal` and `gpu-lavapipe`), the
  threshold still the compiled 256 under REQ-COL-060, provisional. No PR is open.
- **Waits:** TASK-M2-29's proposal and PR (REQ-COL-060).
- **30 Oct 2026:** Decided by the orchestrator per R-369, option (b); listed for the owner's veto at the M2 gate with
  (a) and (c); see PR. Text corrections: ROUNDTRIP's expected brightness is "expected, tagged" (render_contract §
  "Cross-check views (the seams)"); (b)'s 1.1 to 5.7 was measured with the decode stage's `(m, r)`; "grows without
  bound" reads "grows as (|rᵢ|+|rⱼ|)/rᵢⱼ, up to about 1/eps"; inverse_encode_contract Part 4 and R-544's δ are cited.
  Applied: colour_composition § 6's conditioning-aware scale (R-72), the factor proposed in it (R-71), the
  `collision` windows and the per-operand control in the `decode-agreement` gate (TASK-M2-29).

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

## RQ-640: Tanikawa 2000's time unit, for the time columns of its eight tables (proposed T2000-1) *(the owner's request of 29 Oct 2026, "Tanikawa 2000 Collision orbits II (8 tables)"; `docs/incoming/2026-10-29_research_batches/README.md` § 5.6, § 10.5; context: none built)*

*Drafted 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches (that README's § 10.5); filed 1 Nov 2026.*

- **File, section:** `docs/incoming/2026-10-29_research_batches/README.md` § 5.6: "**Units are not stated:** "m1 = m2 =
  m3" (p. 157), with neither the mass value, G nor a time unit", and "so the unit cannot be fixed that way. RQ-640."; §
  9: "Tanikawa 2000's "time" for its triple-collision points is short (0.39–0.49 in Table I) beside Li 2021's T for
  nearby orbits (6.9–11.3), so either the unit or the quantity differs; part of RQ-640." Tanikawa 2000, CeMDA 76, 157,
  p. 157: "m1 = m2 = m3".
- **What is settled without it:** the positions. Equal-mass free fall is scale-free in position, the frame is TSM
  2019's (§ 5.1), and the Fig. 11 point agrees with Li et al. 2021's GT-36 to 1.06e-6.
- **What is not:** the 'time' columns of Tables I–VIII. No Tanikawa 2000 point coincides with a Li 2021 orbit (closest
  1.1e-2, `tanikawa_2000/vs_li2021.txt`), so the unit cannot be fixed from a shared orbit.
- **Read since the draft:** Paper I is now in the record. `docs/incoming/2026-10-28_external_sources/README.md` §
  "Tanikawa, Umehara & Abe 1995": "**Units**, p. 342, quoted: "In the units G = 1, M = 1, AB = 1"", decoded by
  integration as each mass 1; and of its tables, "Time is when the n_min-th encounter occurs". Whether Tanikawa 2000
  keeps both has not been checked.
- **Options seen:** (a) read the normalisation from Paper I (Tanikawa, Umehara & Abe 1995, task 15 item 16) and decode;
  (b) compare positions only, permanently; (c) ask the author.
- **Applied meanwhile:** (b) for now. T2000-1 compares positions; its times wait.

## RQ-641: Broucke, Elipe & Riaguas 2006's rotating Jacobi variables cannot be decoded from the paper *(the owner's request of 29 Oct 2026, "Broucke 2006 (rotating Jacobi variables)"; `docs/incoming/2026-10-29_research_batches/README.md` § 5.11, § 10.5, `broucke_2006/decode_attempt.txt`; context: none built)*

*Drafted 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches (that README's § 10.5); filed 1 Nov
2026. For the owner: (a) is a fetch from publishers, outside the repo.*

- **File, section:** `docs/incoming/2026-10-29_research_batches/broucke_2006/decode_attempt.txt`: "The variables cannot
  be decoded from this paper alone; the equations are in refs [7] (Broucke, Elipe & Riaguas, AIAA 2002-4544) and [10]
  (Broucke & Boggs 1975, CM 11, 13-38), neither in the repo." Broucke et al. 2006, CSF 30, 513, § 3 and Tables 1–3.
- **What is settled:** the transcription (two readings, all differences resolved against the page); Table 2's y/r
  equals Table 1's y to 1e-8; Table 3's f1, f3 follow from a1, a2.
- **What is not:** the map from (y, ẋ, T) to an inertial state. Read as G = 1 with masses 1/3, both Table 1 states have
  E > 0 (unbound); with masses 1, T|E|^{3/2} is 4.08 and 3.45, not the figure-eight's 9.2377; no single G fits both rows.
- **Options seen:** (a) the owner fetches refs. [7] and [10] into `docs/incoming/_local/`, and the decoding is redone;
  (b) leave Broucke 2006 as context.
- **Applied meanwhile:** (b). No target is proposed from it.

## RQ-642: Martynova & Orlov 2014's grid count: 306,346 printed, 306,402 counted *(the owner's request of 29 Oct 2026, ChatGPT's `Martynova_Orlov_2014_figure_initial_conditions.json` against the paper; `docs/incoming/2026-10-29_research_batches/README.md` § 5.9, § 10.5; context: proposed MO14-1)*

*Drafted 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches (that README's § 10.5); filed 1 Nov 2026.*

- **File, section:** Martynova & Orlov 2014, Astron. Rep. 58, 756, p. 760: "We scanned the inner part of the domain D
  in both coordinates with steps Δξ = Δη = 0.001 (the boundaries were not included). In all, we considered 306,346
  versions of the initial conditions. In 306,320 of these cases … decay … In the 26 remaining cases …"; D as bounded
  by the axes and the unit arc about (−0.5, 0) (p. 758).
- **What is settled:** 306,320 + 26 = 306,346, as printed.
- **What is not:** an exact integer count of the grid points strictly inside D at step 0.001 gives 306,402 (306,406
  with the arc included), 56 (60) more than printed (`martynova_orlov_2014/checks.txt`). Which points they left out,
  or whether their grid was offset, the paper does not say. A like-for-like survivor count needs the same grid.
- **Options seen:** (a) compare fractions, 26/306,346 against ours over 306,402, and record the 56 as a stated
  difference; (b) ask the authors which points were dropped; (c) treat the count as UNEXPLAINED in the score.
- **Applied meanwhile:** (a), with the difference stated beside the score.

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
  - `decisions.md` § R-649, item 2, the owner's words: "(a) GPU explorer: fast, f32, interactive; expected to agree statistically or qualitatively only"; the clarification: "External targets bind tier (b) unless they are statistical, where tier (a) is expected to agree statistically."
  - `decisions.md` § R-661: "it is a HYPOTHESIS that Principia will measure, and no paper establishes it."
  - RQ-622, on five statistical targets whose verify names an f64 Result.
- **The question:** R-661 makes tier (a)'s agreement a hypothesis, BZ-1C its test. Until BZ-1C has measured it, a statistical target scored only on f32 can pass or fail for two reasons: the physics, or f32. The choice changes how those results read.
- **Options seen:** (a) score on tier (a) as ruled, each result labelled "f32; agreement with f64 not yet measured", until BZ-1C reports; (b) as (a), and run f64 beside it on a sample sized by the same calculation; (c) statistical targets wait for BZ-1C.
- **Applied meanwhile:** nothing is scored yet, so nothing is applied. R-649's tier assignment is not changed here.
- **Waits:** nothing built.

## RQ-650: Which frame Tanikawa's free-fall papers use: the repo says "the mirror labelling D11 = {(x−0.5)² + y² ≤ 1}", three of the papers say Anosova's region and one prints the mirror *(physics; `docs/incoming/2026-10-25_replication/catalogue.md` line 13; `docs/design/principia_dd_replication.md` line 639; PLAN.md line 707; `docs/incoming/2026-10-28_external_sources/README.md` § 9; context: TM-1, TS-1, TS-2, proposed TSM-1 and TUA-1)*

*Drafted 29 Oct 2026 by the agent ingesting the external sources (`docs/incoming/2026-10-28_external_sources/README.md`
§ "Corrections and contradictions", § 9); filed 1 Nov 2026, with the design doc's line number brought up to date.*

- **File, section, quoted:**
  - `docs/incoming/2026-10-25_replication/catalogue.md` line 13: "Tanikawa's papers use D11 = {x ≥ 0, y ≥ 0,
    (x−0.5)² + y² ≤ 1}, the mirror labelling."
  - `docs/design/principia_dd_replication.md` line 639 and `docs/incoming/2026-10-27_validation_plan/PLAN.md` line 707:
    "Tanikawa uses the mirror labelling D11."
  - Tanikawa, Umehara & Abe 1995, eq. (1): "Let mass points m₂ and m₃ stand still at A(−0.5, 0) and B(+0.5, 0) … m₁
    stand still at a point P(x, y) where (x, y) ∈ D = {(x, y) : x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤ 1}."
  - Tanikawa, Saito & Mikkola 2019, §3: the same D, and "D11 corresponds to Anosova's region".
  - Tanikawa & Mikkola 2008 (arXiv:0802.2465; committed at
    `docs/incoming/2026-10-25_replication/papers/tanikawa_mikkola_2008_arXiv0802.2465.pdf`), p. 3: "We want to extend
    the free fall problem with equal masses (Anosova & Orlov 1986; Tanikawa et al. 1995), in which bodies 2 and 3
    initially stand still at (−0.5, 0) and (0.5, 0), respectively, whereas body 1 stands still at (x, y) ∈ D11 =
    {x ≥ 0, y ≥ 0, (x − 0.5)² + y² ≤ 1}."
  - Tanikawa & Mikkola 2015 (PASJ 67, 115; the owner's copy, local-only, not committed:
    `docs/incoming/_local/tanikawa_mikkola_2015_PASJ_67_115.pdf`), p. 115-2, eq. (9): "Initially, body m2 stands still
    at (−0.5, 0), and body m3 stands still at (0.5, 0) of the (x, y)-plane. Body m1 stands still at any position in
    D = {(x, y) : x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤ 1}."; Fig. 1's caption, p. 115-3: "Anosova's D region is equivalent
    with any of them, say, region D11."
- **What is settled:** the 1995 and 2019 papers use the Agekyan–Anosova frame itself, so TSM-1 and TUA-1 need no remap;
  task 15b's record (`docs/incoming/2026-10-29_research_batches/README.md` § 5.1) finds the same for Li et al. 2021,
  Tanikawa & Umehara 1998, Tanikawa 2000 and Martynova & Orlov 2014, and its § 10.4 holds TM-1's remap wording for the
  owner's confirmation; this entry is the question that confirmation answers.
- **Also read:** Tanikawa & Mikkola 2015 uses Anosova's region, (x + 0.5)², like the 1995 and 2019 papers. Tanikawa
  & Mikkola 2008 prints (x − 0.5)², very likely the source of the repo's "mirror labelling" sentence; it is probably a
  typo, since the paper cites Tanikawa et al. 1995 (whose eq. (1) prints (x + 0.5)²) for this very set-up, and the
  printed region does not keep AB the longest side: (1.4, 0.3) is inside it, with PA = 1.92 > AB = 1.
- **What is not:** whether TM2008's (x − 0.5)² is a typo or the frame its runs used, and which frame Tanikawa &
  Mikkola 2021 (TM-1's source) uses, so whether the rows' "D11 mirror remap" and the general sentence stand. The
  research-batches record (`docs/incoming/2026-10-29_research_batches/README.md`) does state a frame for it: § 10.3:
  "LI21-1, LI25-1, TM23-1 and T2000-1 share one frame with TSM 2019 and TM-1 (§ 5.1 there: same D, no mirror of
  positions)"; § 10.4: "§ 5.1's finding that, for TSM 2019 and these papers, D11 is D with no mirror of positions",
  which it holds for the owner's confirmation; and, as evidence in the data, § 5.4: "Table 2's 11 triple-collision
  points are TSM 2019's Table 1 (and so Li 2021 GT rows)". But § 5.1 quotes no set-up from the 2021 paper: neither its
  list nor its quotes include it.
- **Options seen:** (a) a porter reads Tanikawa & Mikkola 2021's set-up section and corrects the sentence to name the
  papers that use each frame, with TM2008's (x − 0.5)² recorded as printed; (b) drop "mirror" and name D11 as TSM 2019
  and TM2015 define it.
- **Applied meanwhile:** nothing; the proposed rows TSM-1 and TUA-1 state their frame from their own papers.

## RQ-651: Anosova 1986's time unit τ: eq. (1) as printed is not a time *(physics; `docs/incoming/2026-10-28_external_sources/README.md` § "Anosova 1986", § 9; context: proposed AN-2, AN-1)*

*Drafted 29 Oct 2026 by the agent ingesting the external sources (that README's § 9); filed 1 Nov 2026.*

- **File, section, quoted:** Anosova 1986 (Ap&SS 124, 217), p. 218: "The unit of time, τ = 1, is then the mean crossing
  time of a particle through the system"; eq. (1) as printed: τ = G √(Σᵢ Mᵢ Σ_{i≠j} MᵢMⱼ) / (−2E)^{3/2}, the radical
  over both sums; eq. (2): d = G Σ_{i≠j} MᵢMⱼ / 2E. `docs/incoming/2026-10-27_papers/README.md` line 201 records
  Aarseth, Anosova, Orlov & Szebehely 1994b's "τ = G M_t^{1/2}(Σ M_iM_j)/(−2E)^{3/2}".
- **What is not settled:** eq. (1) gives G M^{3/2}/(−2E)^{3/2}, not a time; whether Σ_{i≠j} counts each pair once or
  twice; Table V's escape distance (20d or 30d, p. 229) and its number of runs.
- **Options seen:** (a) read τ as 1994b's (the radical over Σ Mᵢ only, each pair once); (b) treat AN-2 as shape-only
  (compare the normalised distribution up to a free time scale); (c) drop AN-2.
- **Applied meanwhile:** nothing; AN-2 is proposed as waiting on this.

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

## RQ-655: R-632's "a bound" is not true of a start where neither pair has parted *(physics; R-632; `plan/requirements.yaml` REQ-VAL-199; `docs/design/principia_honesty_precision.md` § 6; found by the physics review 5724170840 of PR #363, an aside; context: TASK-M8-76)*

*Queued 31 Oct 2026. Ruled text, restated rather than introduced by PR #363, which it does not hold.*

- **File, section:**
  - REQ-VAL-199: "a pair not parted by the horizon T censored, its t_part taken as T and its start's ratio a bound kept in the median".
  - `docs/design/principia_honesty_precision.md` § 6: "that pair's `t_part` is taken as `T`, the start's ratio is a bound, it stays in the median over all 256".
- **The conflict:** with only the f64 pair unparted, the measured ratio t_part(f32)/T is an upper bound on the true one; with only the f32 pair unparted, T/t_part(f64) is a lower bound. With neither parted the ratio is T/T = 1 before dividing by the prediction, and the true ratio may lie on either side, so it bounds nothing. The gate's median and its censored count are unaffected as written; what such a start's entry means is not.
- **Options seen:** (a) leave the rule and call such a start "unresolved" in the report, counted apart from the one-sided bounds, still in the median over 256; (b) keep the word, as a known looseness; (c) a different treatment of such starts in the median, which changes the statistic.
- **Applied meanwhile:** none: R-632 and REQ-VAL-199 stand as ruled.
- **Waits:** nothing.

## RQ-658: PLAN.md's BR-1 row still says the data is not available, against R-641 and R-642 *(a stale record against a ruling; R-641, R-642; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2; `docs/design/principia_dd_replication.md` § 5.10; REQ-VAL-297; context: TASK-M3-62, TASK-M9-22)*

*Queued 1 Nov 2026 by the orchestrator's filing task, from the library triage's follow-ups.*

- **File, section:**
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, the BR-1 row: Ref. data "not released"; Status "blocked:
    the data is not available".
  - `decisions.md` § "R-641 — …": "**BR-1** (REQ-VAL-297) may be scored against those 180 directly, as the paper's own
    data, not provisionally."; `decisions.md` § "R-642 — …": "BR-1 against the 180 NVM trajectories (pgbreen/NVM @
    a35c378; kept in docs/incoming/_local/, never committed, no licence)".
  - `docs/design/principia_dd_replication.md` § 5.10 already carries this, in its note of 28 Oct 2026: "BR-1 against
    the 180 trajectories of `github.com/pgbreen/NVM` at `a35c378`, never committed".
- **The conflict:** the plan's row predates the rulings and was never given a dated note; its R-631 note in § 3.1 is
  the form such a note has taken. Nothing in the row says that 180 of the paper's ~10⁴ now score directly, or that the
  scoring is in M9 (R-650). The ruled text governs; only the plan's record is stale.
- **Options seen:** (a) a dated note beside the row, leaving it as written, as § 3.1 does for R-631; (b) correct the
  row's Ref. data and Status cells; (c) leave it, the design doc being the current text.
- **Applied meanwhile:** the rulings. No code or target waits.

## RQ-659: REQ-VAL-319 names Manwadkar et al. 2021's Table 1 for the escape probabilities, which are in its Table 2 *(a conflict between a requirement and its source; REQ-VAL-319; `docs/design/principia_dd_replication.md` § 5.12; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2; context: MK-1, TASK-M9-23)*

*Queued 1 Nov 2026 by the orchestrator's filing task, from the library triage's follow-ups.*

- **File, section:**
  - `plan/requirements.yaml`, REQ-VAL-319: "Per-body escape probabilities over Manwadkar et al. 2021's eight mass sets
    must be measured in a planar re-run and compared as distributions with their Table 1 (MK-1)."; its verify detail:
    "Table 1 transcribed twice (rule 2)"; its note: "per-body escape probabilities over eight mass sets (Table 1)".
  - `docs/design/principia_dd_replication.md` § 5.12, the MK-1 row: "per-body escape probabilities over eight mass sets
    (Table 1)".
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, the MK-1 row: Ref. data "Table 1 (not transcribed)".
  - `docs/incoming/2026-10-25_replication/papers/manwadkar_kol_trani_leigh_2021_MNRAS_arXiv2101.03661.pdf`, Table 1:
    "Summary of the three-body simulations sets."; Table 2: "The escape probabilities 𝑃𝑠 and total number of
    realizations 𝑁𝑠 by escaper mass in different three-body systems."
  - `docs/incoming/2026-10-25_replication/catalogue.md` § 5.2: "**Table 1:** Eight mass sets", right of Table 1; it
    gives the probabilities no table.
- **What is settled:** the paper's Table 1 is the mass sets and set-up, and its Table 2 is the probabilities.
- **What is not:** Table 2 prints three cuts: "[1] The entire set, [2] Removing prompt ejections (𝜏 D > 30yrs), [3]
  ergodic subset (𝜏 gap > 10yrs and 𝜏 D > 80yrs)". REQ-VAL-319 names no cut. The paper compares cut 3 with the theories
  ("The probabilities of the ergodic subset are being compared to theoretical formalisms").
- **Options seen:** (a) correct the reference to Table 2, with Table 1 for the mass sets, in the requirement and both
  rows; (b) as (a), and also name the cut; (c) as (a), comparing all three cuts.
- **Applied meanwhile:** nothing. Nothing built waits: MK-1 also waits on RQ-593 and scores in M9.

## RQ-660: PLAN.md's 437 non-zero angular momentum rows count two Rose rows its own list doesn't name, whose L is refinement drift: the named families sum to 435 *(a count against its breakdown; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 2.3, § 7; `docs/design/principia_dd_replication.md` lines 176, 726; `docs/incoming/2026-10-26_orbit_catalogues/threebodyorbits_ics.csv`; `docs/incoming/2026-10-31_library_triage/SUMMARY.md`; context: OC-1)*

*Queued 1 Nov 2026 by the orchestrator's filing task, from the library triage's follow-ups.*

- **File, section:**
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 2.3: "437 with \|L\| > 1e-6".
  - The same, § 7: "OC-1 (437 rows: Simó 344, Broucke, BHH, Sheen, Šuvakov's other orbits)".
  - `docs/design/principia_dd_replication.md` line 176: "437 with |L| > 1e-6; 86 other."; line 726: "OC-1 (437 rows)".
  - `docs/incoming/2026-10-31_library_triage/SUMMARY.md` lines 223-224: "PLAN's 437 L≠0 rows list families summing to
    435; the other 2 are `rose` rows with refinement drift."
  - `docs/incoming/2026-10-31_library_triage/triage.csv`, the `threebodyorbits_ics.csv, family rose` row: "Rose (2016),
    'Geometric phase and periodic orbits of the equal-mass, planar three-body problem with vanishing angular
    momentum'" and "These are general L=0 orbits"; the `atlas-params.json, family rose` row: "The 2 rows with L over
    1e-6 are refinement drift in ref_*, not real nonzero L".
- **Measured, 1 Nov 2026,** on `threebodyorbits_ics.csv`'s `L_cat` (`L_recomp` gives the same): |L| > 1e-6 on 437 rows:
  `simo` 344, `brouckeS` 23, `brouckeA` 16, `brouckeR` 13, `bhhsat` 15, `suvakov_pc` 13, `sheen` 11, and `rose` 2. The
  families § 7 names sum to 435. The two others are `rose2016_t0k4i2_1_1_1` (L = 4.84147e-05) and
  `rose2016_t0k2i4_1_1_1` (L = −3.76993e-06), from a source whose title says vanishing angular momentum. The 413 in
  § 0 holds on the same rows (`closure_refined` < 1e-9). Σ m(x·vy − y·vx) on those two rows' `pub_*` gives 0.0 on both;
  on their `ref_*` it gives 4.841466815563855e-05 and −3.7699259047840883e-06, so `L_cat` and `L_recomp` are the refined
  state's values.
- **What is settled:** the two are L = 0 orbits whose refined state drifted over § 2.3's threshold: the CSV's published
  starts have L = 0 exactly, and the triage reads them the same way. Rose's thesis is not held, so `pub_*` is not
  checked against it here.
- **What is not:** how the plan and the design doc word the count: 437 as the cut measures it, or 435 orbits with
  L ≠ 0.
- **Options seen:** (a) keep 437 as what the cut measures, and add Rose's 2 to § 7's list, named as refinement drift;
  (b) state 435, with the two recorded as L = 0 rows whose refined state is above the cut; (c) a dated note beside
  each, the rows left as written.
- **Applied meanwhile:** nothing. No test depends on the count.

## RQ-661: PLAN.md § 2.1 says the 82 website-only free-fall orbits have no traced initial conditions, and the authors' GitHub table gives them *(a stale record; whether its condition lapses; `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 2.1; R-630 item 8, R-631; `docs/incoming/2026-10-27_hp_ics_freefall/README.md`; REQ-VAL-227; context: LL-5, TASK-M3-46)*

*Queued 1 Nov 2026 by the orchestrator's filing task, from the library triage's follow-ups.*

- **File, section:**
  - `docs/incoming/2026-10-27_validation_plan/PLAN.md` § 2.1: "That page gives words only, no initial conditions.", and
    "for this plan they are closure tests only until their initial conditions are traced to a source the authors
    publish (the website's word page has none)."
  - `docs/incoming/2026-10-27_hp_ics_freefall/README.md`: "So the 82 website-only orbits, of which the atlas carries 81
    (LL-5), have published initial conditions after all, at 10 decimals, here. … It is recorded here and changes no
    target: LL-5 stays a closure test on the atlas's `ref_*` (rule 13)."
  - `docs/design/principia_dd_replication.md` § 4.1: "it gives the initial conditions, (x, y, T) at 10 decimals,
    for all 316, the 82 website-only orbits included. … LL-5 still tests closure on the atlas's rows only (rule 13)."
  - `decisions.md` § "R-630 — …", item 8: "the 81 atlas rows the paper does not contain are closure tests (PLAN.md's
    LL-5), not published references." REQ-VAL-227: "must close after one period as closure tests only, never cited as
    Li & Liao 2019's published results (rule 13)".
  - `docs/incoming/2026-10-31_library_triage/triage.csv`, the table's row: "PLAN.md 2.1's 'words only, no initial
    conditions' is out of date on this".
- **Checked:** `data/liliao_github/free-fall-3b-pictures.md` has rows for the three website-only mass sets: 48 + 19 + 15
  = 82, each with x, y, T.
- **The conflict:** § 2.1's "until … traced" reads as a condition, and it is now met, while R-630 item 8 and REQ-VAL-227
  carry no condition. § 3.1's R-631 note corrects the file's location, and leaves the § 2.1 sentence as written.
- **The question:** whether § 2.1 only needs a dated note, or whether the 81 rows may now be tested against the authors'
  own table, which would change REQ-VAL-227.
- **Options seen:** (a) a dated note in § 2.1 pointing at the GitHub table, LL-5 staying as ruled; (b) as (a), and a new
  target comparing all 316 against the authors' table (the triage's NOW-3), a ruling's to add.
- **Applied meanwhile:** (a)'s reading of the target: LL-5 stays closure only, as ruled. No code waits.

## RQ-662: The layout at sizes it allows: the default split puts the bottom row under its minimum at 1280×720, and at the compass's least size the Plücker readout passes the panel *(a conflict between code and its doc; R-462 J2; `crates/gui/src/layout.rs`; found by the gui review of PR #365 (5726156060); context: found beside TASK-M6-35, outside its diff)*

*Queued 1 Nov 2026 by the orchestrator's filing task, on that review's "Outside this PR, for the orchestrator".*

- **File, section:**
  - `crates/gui/src/layout.rs:44-46`: "The bottom row's least height, in points: the compass's cube, buttons and readout,
    and Time's transport, whole (J2; measured: the compass's readout passes the footer below 154.7).", `MIN_BOTTOM_H:
    f32 = 160.0`.
  - The same, `:47-49`: "The compass's least width, in points: its cube and its buttons beside it, and its readout (J2;
    measured: the readout passes the edge below 224, a tilted readout a few characters longer).", `MIN_COMPASS_W: f32 =
    240.0`.
  - `Layout::with_splits`'s doc: "the minimums hold the splitters' drags ([`Layout::range`])", so they hold drags only.
  - `decisions.md` § "R-462 — …", J2: "Each panel's minimum keeps its controls usable (the Manifold view its widest row,
    the compass its cube and buttons, Time its transport)".
  - The gui review of PR #365 (5726156060): "at 1280×720 the default row is 157.3 high, below `MIN_BOTTOM_H`. A default
    split is therefore not held to the minimum in `Layout::with_splits`. At 240 × 160 the Plücker readout also passes
    the panel (883.3 against 876.7), contrary to the doc at `layout.rs:44-48`. Both predate this diff."
- **Checked:** the first by the constants: the default row is `(FOOTER_TOP − BOTTOM_TOP) / ARTBOARD[1]` = 295/1350 of the
  height, so 157.3 at 720, and under 160 for any window under about 732 high. No minimum window size is set
  (`crates/gui/src/cli.rs`'s only size is `with_inner_size`, 1440×900, where the row is 196.7). The second is the
  review's measurement, not repeated here: a docs-only filing runs no build.
- **The question:** what a size below the minimum does. Neither J2 nor the spec says what holds at small windows.
- **Options seen:** (a) hold every split, the default included, to the minimums in `Layout::with_splits`, where the
  window has room; (b) set a least window size at which the default layout meets them; (c) re-measure `MIN_COMPASS_W` and
  `MIN_BOTTOM_H` against the widest readout, for the second part; (d) correct the doc to say they bind drags only.
- **Applied meanwhile:** nothing; the code stands as on `main`. Nothing waits. PR #365's own help text is its own finding,
  apart from this entry.

## RQ-663: With X ≠ Y, a typed basis with Y on q₁ or X on q₂ reads with that component ignored: q₁ = 0.8 e_μ1, q₂ = e_β reads σ = 0, and the next zoom edit rewrites the plane *(a defect in the reading; R-457 E1, R-459 G3, G4; `crates/gui/src/explore/manifold_view/mod.rs`; found by the code review of PR #366 (5726096107); context: found beside TASK-M6-40, outside its diff)*

*Queued 1 Nov 2026 by the orchestrator's filing task, on that review's "Not this PR's: for the orchestrator to file".*

- **File, section:**
  - The code review of PR #366 (5726096107): "With X != Y, a typed basis with Y on q₁ (or X on q₂) has that component
    ignored. σ is then the α part alone: q₁ = 0.6 e_α + 0.8 e_μ1, q₂ = e_β reads σ = 0.6 and off by 0.8. q₁ = 0.8 e_μ1,
    q₂ = e_β reads σ = 0, so the zoom is infinite. The next zoom edit (`navigate.rs`,
    `basis_edit(view.with_zoom(zoom).basis())`) then rewrites the plane."
  - `crates/gui/src/explore/manifold_view/mod.rs`, `View::in_frame`: `let sin1 = cos_g * q1[x] - sin_g * q2[x];`, `let
    sin2 = sin_g * q1[y] + cos_g * q2[y];`, and `scale: cos1.hypot(sin1)`; `View::framed`'s preset test `outside(a, b)`
    leaves out `a`, `b`, `x` and `y` alike, whichever vector carries them; `View::zoom`: `-self.scale.log2()`.
  - `crates/gui/src/explore/manifold_view/navigate.rs`, the zoom row: `basis_edit(view.with_zoom(zoom).basis())`.
  - `decisions.md` § "R-459 — …", G3: "an edit in Chart (the preset, a basis editor) moves H and V with it".
- **Traced on `main`'s code, by reading (no build; X = z_q3, Y = z_μ1):** q₁ = 0.8 e_μ1, q₂ = e_β is not an `axis_pair`
  (6 > 1), and preset (α, β) holds it, as `outside` ignores μ1. So `a` = α, `b` = β; `cos1` = 0, `cos2` = 1, γ = 0; `sin1`
  = q₁[q3] = 0 and `sin2` = q₂[μ1] = 0; σ = `cos1.hypot(sin1)` = 0, and the zoom is +∞. `with_zoom(z).basis()` then gives
  q₁ along e_α, q₂ along e_β, so a zoom edit replaces the typed μ1 direction with α. The plane moves on an edit that names
  only the scale.
- **The question:** how a basis whose X and Y components sit on the wrong vector is read, so that a zoom keeps the plane.
- **Options seen:** (a) make the preset's test count those components, so the basis reads in the dominant axes' frame;
  (b) have zoom scale the stored basis rather than rebuild it; (c) both.
- **Applied meanwhile:** nothing; `main`'s reading stands. PR #366 leaves this reading as it is, its review not blocking
  on it and not waiving it. Nothing built waits.

## RQ-639: The escape window is "provisional" (R-29), and no requirement, task or gate ever confirms it *(physics; calibration; silence; R-29, R-610; `docs/contracts/principia_integrator_contract.md` Part 3; `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema"; REQ-EVT-017, REQ-GUI-103; the code review (5723498808, F2) and physics review (5723511480, call 3) of PR #348; context: TASK-M3-06)*

*Queued 31 Oct 2026 by TASK-M3-06's fix round, in the physics review's text. For the owner: yes, calibration values and
physics where the choice changes results (R-369).*

- **File, section:**
  - `decisions.md` § R-29: "**The window** is 0.4 time units, sampled at sync boundaries (**provisional**)."
  - `docs/contracts/principia_integrator_contract.md` Part 3: "`tau`, escape window | —, 0.4 (provisional)".
  - `docs/read_first/principia_01_pitfalls.md` § 2.2: "`|Δn̂|` is taken over 0.4 time units (**provisional**)".
  - `decisions.md` § R-610: "If the window is recalibrated (M3), the floor, the refusal's reason and the line beside
    the field move with it".
  - `plan/requirements.yaml`, REQ-EVT-017's note: "the window length is marked provisional in R-29; a change must go
    through a ruling."
  - `plan/requirements.yaml`, REQ-GUI-103: "the escape window (0.4 time units, provisional)".
- **The gap:**
  - REQ-EVT-025 calibrates `tau` only, and `plan/MILESTONES.md` § M3 lists "the tau gap" but not the window.
  - TASK-M3-07's proposal covers only "how the closure window's 0.4 time units map onto the grid". TASK-M3-34 measures
    at the fixed window.
  - The register's `escape_window` is settled (citing R-29), so `provisional_until` omits it and the M3 gate never
    blocks on it. `docs/design/principia_dd_generation_root.md` § 3.8 forbids marking it provisional without a
    calibration requirement.
- **The question:** is 0.4 confirmed by measurement at M3, or by ruling as it stands? It changes which trajectories
  fire escape and when, and R-610's horizon floor follows it.
- **Options seen:**
  - (a) A new calibration requirement, confirmed at the M3 gate, proposed by the task that re-validates escape
    (TASK-M3-34), with evidence over a window ladder. The register entry then becomes `proposed(..., <req>, "M3")`.
  - (b) A ruling confirms 0.4. The word "provisional" comes off by a port, and the entry cites that ruling.
- **Second question:** the window is carried and shown in the Run window (REQ-GUI-103), but `validate` accepts any
  finite positive value, and T's floor follows it (`check_horizon`). So what range may it take? That range also bears
  on the smallest `dt_macro` a carried window can reach; TASK-M3-06 meanwhile refuses a `dt_macro` whose smallest
  substep is not a normal f32, per R-369. That refusal bounds the substep only: at its floor the stages' `c·dt` (KDK's
  half-kick, 2⁻¹²⁷ at `N_max` = 64) are still subnormal and a flushing backend reads them as 0, with no branch affected
  and no dynamics at that scale. Every stage size stays normal for a window of at least about 2⁻⁹⁸, and at the window
  of 0.4 it is near 2⁻²⁹. So any lower bound the range sets at or above 2⁻⁹⁸ makes the `dt_macro` refusal unreachable.
  The range is also where a physically meaningful lower bound belongs: f32 rounding absorbs a step far above this
  floor.
- **Applied meanwhile:** 0.4 per R-29, settled in the register, the doc comment saying provisional (TASK-M3-06).
- **Waits:** the answer is needed by the M3 gate. No task waits.

## RQ-624: The Inspector's equator check: `|w|` read on `D(E(x))` is the `β` clamp's image, about π·10⁻⁶, not the 6.8×10⁻¹⁴ R-551 cites, and no check can catch "a state moved off the line by one ulp" *(conflict: the requirement's wording against its ruling's evidence; R-551, R-71, R-72; REQ-VAL-022, REQ-VAL-121; context: TASK-M2-50)*

*Found by TASK-M2-50 before building the shape checks. Nothing is chosen.*

- **File, section:**
  - `decisions.md` § "R-551 — …", RQ-475, the human's words: "Equilateral → `|w − 1|` and collinear `|w|` lie within
    an R-71 calibrated bound, with the measured 2.2×10⁻¹⁶ and 6.8×10⁻¹⁴ as evidence." Those two numbers were measured
    on `C`'s output: `docs/archive/review_queue/M0.md` § "RQ-475", "Measured on the f64 encode, `w` of `C`'s output
    through `kernel::shape::shape`".
  - `plan/requirements.yaml` REQ-VAL-022, verify: "max |w − 1| and max |w|, read on the canonical state D(E(x))
    (where C's mirror puts L⁻ at w = +1), within REQ-VAL-121's bounds (measured 2.2×10⁻¹⁶ and 6.8×10⁻¹⁴)".
  - `plan/tasks/M2/TASK-M2-50.md` § "Deliverables" and § "Acceptance tests": "max `|w − 1|` and max `|w|`, read on the
    canonical state `D(E(x))`"; and "controls: a state moved off the line by one ulp is caught".
  - The wording "read on the canonical state D(E(x))" came from the physics review of PR #274, P3, applied per R-369:
    "Add "read on the canonical state" (or `D(E(x))`) to both, so the test can't be written against the input." It
    offered the two as one state; at the equator they differ.
- **Measured** (f64 encode and decode, the defaults links; RQ-475's families: 2000 of each per seed, through random
  gauge elements):
  - Pole, equal-mass equilateral, `L⁺` and `L⁻`: max `|w − 1|` = 2.2×10⁻¹⁶ on `C(x)` and on `D(E(x))` alike (seeds 1,
    2, 3). No `w` is negative on either, so `C`'s mirror puts `L⁻` at `w = +1` on both.
  - Equator, collinear with masses from `z_μ` on `[−4, 4)²`: max `|w|` on `C(x)` = 6.8×10⁻¹⁴, 2.4×10⁻¹⁴ and
    1.2×10⁻¹³ (seeds 1, 2, 3); on `D(E(x))` 3.14156×10⁻⁶, 3.14159×10⁻⁶ and 3.14159×10⁻⁶. Every collinear `z_β` is at
    its clamp image, so `D` gives `β = π·ε_z`, and `w = sin 2α · sin(π·ε_z)` ≤ π·10⁻⁶. `D(E(x)) = C(x)` holds only
    off the clamps (REQ-ENC's left inverse, "modulo gauge"), and the equator is on one.
  - One ulp off the line (each collinear state's body 2 moved in `y` by one ulp of its `x`, then the same gauge
    elements): `z_β` at its clamp image with its bit set in 2000 of 2000, so the bit-for-bit check passes it; max `|w|`
    7.2×10⁻¹⁴ on `C(x)`, the same as the states on the line, and π·10⁻⁶ on `D(E(x))`. A physical one-ulp move is below
    both the clamp (`β < π·ε_z` is clamped) and the gauge's rounding, so neither check catches it.
- **The question:** (1) on which state the equator's `|w|` is read, and so what its bound measures; (2) what the
  control "a state moved off the line by one ulp is caught" means, since no check can catch a physical one-ulp move.
- **Options seen:**
  - (1a) On `C(x)`, "the canonical state" of P3's first wording, the state R-551's evidence was measured on: the
    bound is proposed from about 10⁻¹³ (it tests `C`'s rounding off the line); the pole check reads the same state.
  - (1b) On `D(E(x))`, as written: the bound is proposed from π·ε_z, and 6.8×10⁻¹⁴ is no longer its evidence; the
    check then tests the clamp's image, which the bit-for-bit `z_β` check already pins.
  - (1c) Both: `|w − 1|` on either (they agree), `|w|` on `C(x)` judged, on `D(E(x))` reported.
  - (2a) The control moves `z_β` one ulp off its clamp image (the equator in `z`): the bit-for-bit check fails it,
    which shows the comparison is exact and has no tolerance.
  - (2b) The control moves the state off the line by enough to leave the clamp (`β` past π·ε_z), and the bit-for-bit
    check fails it; "one ulp" is reworded.
  - (2c) Both (2a) and (2b).
- **Applied meanwhile:** nothing. The Randomise arm (REQ-VAL-193), the `z_β` bit-for-bit check and the pole's
  `|w − 1|` do not depend on the answer.
- **Waits:** REQ-VAL-022's equator `|w|` check and its one-ulp control, and REQ-VAL-121's equator bound (TASK-M2-50).
- **30 Oct 2026:** Decided by the orchestrator per R-369; see PR #346 (TASK-M2-50). Test and verify-text matters, no
  physics change: (1a) both shape checks are read on `C(x)`, `D(E(x))`'s `|w|` reported unjudged as the clamp image,
  sin2α·sin(π·ε_z); (2a) with an in-clamp-band control (a collinear state about 10³ ulps off the line, caught by
  `|w|` on `C(x)` while `z_β` still clamps) and the bound-tightened control; the one-ulp control dropped. REQ-VAL-022's
  verify and TASK-M2-50 corrected, `rq: [RQ-624]` removed from REQ-VAL-022 and REQ-VAL-121.

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

## RQ-635: The arctangent generator's working precision, 100 fraction bits: a value the corpus doesn't give, and its admissibility class *(a calibration requirement, R-71; REQ-GEN-044; REQ-SYS-005, REQ-SYS-082; R-427; philosophy §4.2; context: RQ-628, TASK-M2-26, PR #343)*

*Queued 31 Oct 2026 from TASK-M2-26's fix round, on code review 5720675453. For the M2 gate's calibration batch; nothing
waits on it before the gate.*

- **File, section:**
  - `decisions.md` § "R-71 — A missing value becomes a calibration requirement": "Every value the corpus doesn't give
    becomes a **calibration** requirement in the milestone that needs it."
  - `plan/requirements.yaml`, REQ-SYS-082: "every other function there that returns a numeric literal must be moved into
    the register or named in its doc as ledger or interface data".
  - `docs/read_first/principia_00_philosophy.md` § "4.2 Every constant must be derived or absent": "A quantity is
    admissible if it is bounded by its own achievable maximum, fixed by a conservation law, or expressed in canonical
    units."
  - `crates/ledger/src/gen/arctan.rs`, `sum_bits()`, before this fix: returned the literal 100, its doc citing nothing.
- **What it is:** the units, `2⁻¹⁰⁰`, in which the generator sums each `atan(k/8)` in integer arithmetic before rounding
  to the Q0.64 and Q2.62 words it writes. Only the generator reads it. The emitted file doesn't hold it.
- **The gap:** no document gives this precision, and RQ-628's decision doesn't name it. It is a choice, not a
  derivation: 100 is one value on a measured plateau, picked as a round number with room on both sides.
- **The measurement** (the generator's sum in exact integers, each product checked against `2¹²⁸`, for every bit count
  from 65 up): every count from 72 to 121 writes the same nine `ATAN_EIGHTHS`, `HALF_PI` and `PI`. At 71, three of the
  eleven differ. At 122 a product passes `2¹²⁸` and the `u128` arithmetic overflows. The sum's error at 100 bits: under 2
  units per term, each term carrying its predecessor's truncation (`eₙ₊₁ < eₙ/2 + 1`), which with at most 100 terms and
  a dropped tail under 4 units is about 204 units; the measured worst is 90.05 units (`k = 8`, 95 terms), against the
  `2³⁶` units the table keeps.
- **The question:** confirm 100 at the M2 gate, or another value in 72 to 121, which writes the same file; and whether
  the class stands.
- **The class:** proposed as `canonical-units`, a pure number with no physical unit, as `curve_grid_log2` is. It is not
  a clean fit, and no class is. `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry
  must carry)" reads: "`achievable-maximum` covers a format's achievable extreme: a value fixed by a number format's
  limits", and 100 isn't fixed by `u128`'s limit, only bounded by it, though `curve_golden_steps`, a count where a
  further step resolves nothing, holds that class. No conservation law applies.
- **Options seen:** (a) confirm 100 as proposed; (b) a value derived from the plateau's edge, 72, which has no margin
  against a change of the sum; (c) the largest, 121, which has no margin against overflow.
- **Applied meanwhile (per R-369, R-71, R-427):** `arctan_sum_bits` = 100 in the register, marked provisional until M2,
  citing REQ-GEN-044; `sum_bits()` reads it. Not hashed into the schema version: it decides no stored bit.
- **Waits:** the M2 gate only (`gate-provisional --milestone M2` blocks until it is confirmed). TASK-M2-26 proceeds.

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

## RQ-657: REQ-PERF-115 and TASK-M8-96 name the native comparison as `cargo run -p gui --features mock`, which is opt-level 0, while the site they compare it with is built `--release` *(silence: the requirement names no build; R-627; REQ-PERF-115; the perf review of PR #368 (5726255918), F1; context: TASK-M8-86, TASK-M8-96)*

*Queued from the perf review of PR #368. Wording only: what was measured is settled under R-369 below, and the requirement is not changed here.*

- **File, section:**
  - `plan/requirements.yaml`, REQ-PERF-115, `verify.detail`: "beside `cargo run -p gui --features mock` on the same Mac by the same method (TASK-M8-86's frame statistics)".
  - `plan/tasks/M8/TASK-M8-96.md` § "Acceptance tests", the REQ-PERF-115 line: "beside `cargo run -p gui --features mock` on the same Mac by the same method".
  - `plan/tasks/M8/TASK-M8-86.md` § "Deliverables": "The same scripted pan and frame-rate window in the native mock app (`cargo run -p gui --features mock`)".
- **The silence:** the root `Cargo.toml` has no `[profile.*]`, so that command runs unoptimised, and `xtask/src/web_skeleton.rs` builds the site with `--release`. On one head and window, native idle, the perf review read unoptimised work at about 3 to 7 times release's (work less wait p95, 4.75 to 4.91 ms against 0.72 to 1.41 ms; load-affected, size and direction only). The frame rate on these runs moved between about 60 and 120 in either build, so fps alone does not show the build; this is perf review 5726291237, F3, the correction of 5726255918, F1, whose halved-rate reading it withdraws.
- **The question:** whether the three lines should name `--release`, for example `cargo run -p gui --features mock --release`.
- **Options seen:** (a) add `--release` to the measured native row in the REQ-PERF-115 and TASK-M8-96 lines, leaving TASK-M8-86's line, which names the app, as it is; (b) leave the wording, the report's `build` field and this entry being the record.
- **Applied meanwhile:** applied per R-369, the measured native row uses `--release`. The report and the panel name the build (`build`, "debug" or "release"), and PR #368's "What to try" gives the command. No requirement or task text changes for this entry.
- **Waits:** nothing built waits. TASK-M8-96's perf reader measures with `--release` and checks the `build` field.

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

## RQ-678: On the tilted Burrau `K` charts, R-471 E3's "every pixel feasible under every tilt" against R-633 c2's refused cells *(physics; R-471 E3; R-633 c2; REQ-CHART-061, REQ-CHART-067; context: TASK-M2-38, TASK-M2-52)*

*Queued 3 Nov 2026 by the orchestrator's filing task, from the read-only pre-flight of TASK-M2-38. For the owner: yes,
it is physics, the two rulings give different results on one chart.*

- **File, section:**
  - `decisions.md`:13831, § "R-471 — …", E3: "Per-pixel evaluation keeps every pixel feasible under every tilt." Its port,
    :13846-13847: "so a tilt or slice that moves a hidden coordinate (masses, shape) keeps every pixel feasible under
    every tilt."
  - `plan/requirements.yaml`, REQ-CHART-067: kind-3 axes warped by default "wherever they appear (the invariant charts,
    the Burrau family's K axes, …) … so every pixel inside [0,1]² is feasible by construction".
  - `decisions.md`:27436-27438, § "R-633 — …", the owner's (c) c2: "the (ν, K) chart and Φ_θ,K gain the held L_z, and
    Φ_θ,L_z gains the held K, as physical hidden coordinates. Cells with no momentum construction at the tilted value
    (K < L_z²/2I) read as refused, through the tilt's existing ray-degenerate path."
  - `plan/requirements.yaml`:2950, REQ-CHART-061's verify: "on the invariant and Burrau K charts, warped, every grid
    pixel whose (a, b) lie in the square validates at every tilt".
  - `decisions.md`:27541-27543, R-633's note: TASK-M2-38 runs "REQ-CHART-061's fuzz over every gallery chart, the Burrau
    `K` charts tilted".
- **The conflict:** with the held `L_z` tilted in, a warped `K` axis that does not read `L_z` reaches pixels with
  `K < L_z²/2I`, which c2 refuses; REQ-CHART-061 requires every in-square pixel there to validate. E3 names masses and
  shape as the hidden coordinates it covers; c2 adds a momentum one.
- **The question:** on the tilted `(ν, K)` and `Φ_θ,K` charts (and `Φ_θ,L_z` tilted into `K`), does every in-square pixel
  validate (E3), or may cells be refused (c2)?
- **Options seen:** (a) E3 holds: on `(ν, K)` and `Φ_θ,K` the `K` warp takes the pixel's own held `L_z` (its floor
  `L_z²/2I`); on `Φ_θ,L_z`, whose `L_z` axis is not that warp but `docs/design/principia_chart_reference.md` § 4.5's
  linear band `L_z(v) = (2v − 1)·λ_L·L_max(K)`, `L_max(K) = √(2IK)` is taken at the pixel's own tilted held `K`. On
  `(ν, K)` and `Φ_θ,K`, c2's refusal is then reached only under the linear option if the floored warp is additive, such
  as `K = L_z²/2I + K_max·t^γ`; a form that keeps `K ≤ K_max` still has no feasible pixel once `L_z²/2I > K_max`, so the
  form is part of the option. On `Φ_θ,L_z` the tilt moves the held `K` itself (`K = K_centre + d`, `d = span·d̃`,
  `crates/kernel/src/chart/tilt.rs`:555-564; the offset is added to its target at :596, on `main` so far only for the
  latent controls; TASK-M2-52 adds the held `K` as a target), and the band gives `L_z²/2I = (2v − 1)²·λ_L²·K ≤ K` for
  every `K ≥ 0` (`v ∈ [0, 1]`, `λ_L ∈ (0, 1]`, § 4.5); so there a tilt that drives the held `K` below 0
  (`d < −K_centre`) is refused under either option and whatever the floor form, as `√(2IK)` has no value and §2.2's
  construction has none ("`K ≥ 0` always", `decisions.md`:18628). Open within (a), not chosen: whether the held `K`'s
  tilt on `Φ_θ,L_z` is bounded or warped to stay at or above 0 (which would give E3 there too), or those cells stay
  refused; (b) c2 holds: refused cells occur under tilt, warped or not, and REQ-CHART-061's arm on those charts counts a
  refused cell as not a failure (E3 and REQ-CHART-067 read as covering masses and shape); (c) another, stated by the
  owner.
- **Applied meanwhile:** TASK-M2-52 builds c2 now, as R-633 words it (its refused-cell arm), and is reworked under (a)
  (applied per R-369 by the orchestrator: TASK-M2-52 is not held; it proceeds on the corpus as written).
- **Waits:** TASK-M2-38 (REQ-CHART-061's tilted Burrau `K` arm); TASK-M2-52's rework under (a) (it proceeds meanwhile,
  above), as it builds c2's refusal, option (b): its Deliverables' refused cell (`plan/tasks/M2/TASK-M2-52.md`:32, "on
  the three Burrau charts, a cell whose tilted value leaves no momentum construction (`K < L_z²/2I`, §2.2's terminal
  case) is no system") and `tilt_every_chart`'s arm (:37) "cells with `K < L_z²/2I` refused … and every other cell
  valid", with its control, on the warped `(ν, K)` and `Φ_θ,K`. Under (a) that `K` decode and that test change.

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

## RQ-684: R-360 has every control run across the `xtask-ci` shards, through `cargo xtask controls`; three now run in `ci-workspace` under nextest *(conflict: R-360 and REQ-SYS-077 against PR #395's item (f), decided per R-369; the code review (5763833425, F3) and qa review (5763914449, Q1) of PR #395; context: `tooling/ci-one-build`)*

*Queued by the implementer of PR #395, on the orchestrator's decision; recorded for the owner. Nothing waits.*

- **File, section:**
  - `decisions.md` § "R-360 — `cargo xtask ci --partition k/n` splits the controls into n = 4 parallel jobs, by a stable
    hash of the control name", Applied: "The n slices are disjoint and together hold every control, so every control
    runs exactly once across the shards", and "The controls still run through `cargo xtask controls`'s own cargo
    invocations (R-231), inside `cargo xtask ci` (R-235, REQ-VAL-007)."
  - `plan/requirements.yaml` REQ-SYS-077, statement before PR #395: "every control run exactly once across the shards
    (R-360)"; verify detail before PR #395: "whose CI logs together run each control once (R-360)".
- **The conflict:** PR #395's item (f) takes the three controls in `xtask/tests/nextest.rs` that list the workspace in
  its default feature set out of the `xtask-ci` shards (`controls::IN_CI_WORKSPACE`, `Partition::runs`), since in a shard
  they cost a second whole build of the workspace's tests (run 41756812074: 48 GB of `target/debug/deps`, shards 2 and
  4). They run as a step of `ci-workspace`, through `cargo nextest run --features controls`, not through
  `cargo xtask controls`. So each control still runs once on every code run, but not every one across the shards, and
  those three are judged by libtest's `#[should_panic(expected)]`, not by `cargo xtask controls`. `cargo xtask ci`
  without `--partition`, as in `gate.yml` and `nightly.yml`, and `cargo xtask controls` locally still run all of them.
- **Applied meanwhile:** the move stands, decided per R-369 as a CI decision by the orchestrator on PR #395, since each
  control still runs exactly once per code run, and a control that stops failing its test reds the required `ci`
  through `ci-workspace`. REQ-SYS-077's statement and verify detail are amended to say so (PR #395, its `*Was:*` kept in
  the requirement). R-360's text in `decisions.md`, and `plan/CURRENT_RULES.md`, are unchanged: R-360 stays for the owner.

## RQ-685: What R-297's "bit-identity" covers with the compute setting off: branch decisions only, or every Vulkan driver's continuous results, division the one operation measured *(physics; conflict: R-297, REQ-SYS-074's verify and the probe tests against REQ-INT-057 and parity contract § 2; R-84, R-649; issues #390, #396; context: the Windows RTX 2080 Super run at `e1df88fa`)*

*Queued 4 Nov 2026 by the orchestrator's filing task, from the Windows GPU run (#390), the entry REQ-SYS-074's verify
asks for ("a mismatch with the setting off is a REVIEW_QUEUE entry"). For the owner: yes, it changes what the corpus
claims, what the probe tests assert and what the header records. Nothing is chosen. This entry also holds the run's
record, which RQ-686 and RQ-687 share.*

- **The run:**
  - Commit `e1df88fa`, NVIDIA GeForce RTX 2080 SUPER, Vulkan, driver 621.01, Windows 26200.9711. Metal (Apple Paravirtual
    device) and lavapipe (llvmpipe, LLVM 20.1.2, Mesa 25.2.8) are CI run 41744485022, the same commit;
    `qa_m225_kernel_rho_angle_at_the_lambda_pole` on Metal is a local job (1380), since CI's `gpu-metal` doesn't select
    it. Classification: #390, comment 6629109407.
  - Windows GPU results are measurements (R-649), never gates. Nothing was re-baselined, and no fixture, tolerance or
    test was changed.
  - **13 Windows test failures. None fails on Metal, lavapipe and the RTX together.** 10 are NVIDIA-Vulkan-specific
    (#396 six, #397 two, #399 one, #393 one); 2 are our refusal of a discrete adapter (#398:
    `compute_fast_math_defaults_off`, `qa_compute_fast_math_header_is_the_dispatch`); 1 is both (#398 and #396:
    `compute_fast_math_switch_acts`). Three `build_kernel` failures, Windows process failures (os error 193), are not
    among the 13 and are not GPU results.
  - **The hardware-health arm is clean.** The owner reported that the card may have faulty shader cores, and held these
    findings until a determinism control ran. `cargo xtask golden --all`, 6 runs: all 453 output files identical. The 13
    failing tests, 5 reruns each: 65 of 65 failed, one distinct output per test. `decode_` and
    `cargo xtask gate decode-agreement`, 3 runs: 0 differing lines. No `nvlddmkm` event in any run window. The owner
    released the findings. The Windows session's caveat stands: repeating on one card rules out an intermittent fault,
    not a fixed one, and only agreement with another GPU or backend separates those. DX12 on this card is the planned
    second discriminator (#390, Phase 2) and has not run.
  - **The physics reach is unmeasured.** No GPU march was run at this commit: `spirv(compute` appears only in
    `crates/kernel/src/` `bringup.rs`, `toolchain.rs`, `decode/invariant.rs` and `stage/{canon,decode,encode,fragment}.rs`,
    so nothing here integrates a trajectory on a GPU, and nothing here shows how far these differences reach.
- **File, section:**
  - `decisions.md` § "R-297 — …", the human's words: "Off: bit-identity and identical branch decisions hold (R-84,
    REQ-INT-057)."
  - `decisions.md` § "R-84 — …", which R-297 cites for that claim: "`parity_contract` governs. Branch decisions are
    identical on identical inputs, per step."
  - `decisions.md`:3213-3216, R-297's Still in force line: "In the browser build: … bit-identity with the setting off no
    longer holds there, each stage's compiled mode is recorded as "unknown", and runs are held to R-85's Tier-N tolerances
    (R-303)." § "R-303 — …", Applied: "Browser runs are held to R-85's Tier-N tolerances, measured, not bit-exact".
  - `docs/contracts/principia_parity_contract.md`:154, § 4: "**Off:** bit-identity and identical branch decisions hold
    as this contract states them (R-84; Tier B)".
  - The same file, :97, § 2, Tier B: "a GPU's `pow`/`sqrt`/`div` and libm's differ by 1–3 ulp *in every math mode*".
  - The same file, :145, § 4, source 2 of the tolerance: "**cross-backend f32 variance** — the kernel run through native
    `wgpu` vs the user's browser/driver can produce slightly different f32 results for the same kernel; shader translation
    and driver math differ across backends."
  - `docs/notes/principia_gpu_determinism_note.md`:62, § "The parity consequence", the Standing caveat: "The spike's
    bit-identity evidence is **Metal-only**: on the test machine both the native and the browser legs funnelled to the
    same MSL compiler, which is *why* even continuous words matched. Branch-word bit-identity across a **different naga
    backend + driver** (Vulkan / D3D12) is the one thing the spike could not witness".
  - `plan/requirements.yaml`, REQ-VAL-099 (R-58): "The parity gate must be run on a non-Metal backend (Vulkan or D3D12)
    before any Paper-2 numbers are produced; it gates Paper 2, not the build." R-110's Still in force line: "lavapipe as
    the second backend on every commit, meeting M4's two-backend check, and a real non-Metal GPU gating Paper 2 (R-58)".
  - `plan/requirements.yaml`, REQ-INT-057: "the wrapper's branch decisions made from the current state (substep bucket
    N_sub, cap decision, collision, escape, horizon, SIM_FAILED and terminal) must be bit-identical across the CPU and
    every GPU backend on identical inputs, per step, while continuous values (positions, momenta, energy, trajectory,
    and the outcome class they decide near a chaotic boundary) are left free to diverge and are never reconciled".
  - `plan/requirements.yaml`, REQ-SYS-074's verify: "in the gpu-metal and gpu-lavapipe jobs … compares each bit for bit
    with the CPU's correctly rounded f32 division and reports the mismatches per backend; it must show none on either
    backend … a mismatch with the setting off is a REVIEW_QUEUE entry".
  - `crates/validation/src/gpu.rs`:978-979: "bit for bit, on every backend"; `crates/validation/tests/qa_TASK-M0-44.rs`:3-4
    the same, wider than the requirement's two jobs.
  - `crates/kernel/src/canon.rs`:67-69: "`√`, `/` and `atan2` carry ulp bounds in WGSL, not correct rounding; `fma` is
    not fused on every backend (CI's lavapipe evaluates it as a rounded product and an add, RQ-298), nor is `a·b + c`
    left uncontracted on every one".
  - `decisions.md`:3282, R-297's Applied note: "Where wgpu's own path compiles without fast-math and offers no switch
    (Vulkan, on lavapipe in CI), off is that path". `crates/engine/src/compute.rs`:58: `Api::Vulkan =>
    Some(all(StageMode::Off))`, a table entry; `crates/engine/src/contract/fast_math.rs`:24: "Compiled with fast-math
    off." REQ-TOOL-141: each stage's mode "as compiled on the running backend". Nothing observes the driver.
- **Measured (#396), bit-for-bit differences, the setting off but for the "Metal on" column:**

  | Probe | Against | RTX | Metal off | Metal on | lavapipe | CPU reciprocal |
  |---|---|--:|--:|--:|--:|--:|
  | `(x + 0.5) / 255.0`, 256 | correct rounding | **191** | 0 | 191 | 0 | 191 |
  | Harness fuzz, 65536 | correct rounding | **19407** | 0 | 20646 | 0 | 17762 |
  | QA fuzz, 163840 | correct rounding | **50411** | 0 | not run | 0 | 44457 |
  | `(x + 0.5) / 255.0`, 256 | `n * (1.0 / d)` | **0** | 191 | not run | 191 | 0 |
  | QA fuzz, 163840 | `n * (1.0 / d)` | **22519** | 44457 | not run | 44457 | 0 |

  The CPU column is a reproduction of the same inputs. With the constant divisor, the card's 191 values of x are exactly
  the CPU reciprocal's set, by list comparison, with the same hash on all 12 occurrences across the runs. With a runtime
  divisor it matches neither: the first eight mismatches are `0.5 / d` for d = 33, 37, 61, 67, 75, 83, 97, 123, each one
  ulp from correct. With the setting on, the card has no measurement: each test that loops over the settings stopped at
  off.
- **Read from the code:** naga 30.0.1 writes a plain `OpFDiv`, and its SPIR-V backend has no `NoContraction`.
- **Inferred, not measured:** the driver folds a constant divisor into a multiply; the card's runtime arithmetic is not
  identified.
- **From memory, unchecked:** Vulkan and WGSL bound `x / y` at 2.5 ULP, neither promises correct rounding, and no Vulkan
  switch asks for it. The eight posted quotients are within that bound; no worst case over the fuzzed sets was printed.
  Also unchecked, from the physics reviewer's memory: SPV_KHR_float_controls2's `FPFastMathMode` has an `AllowRecip` bit,
  which could forbid the reciprocal rewrite without giving correct rounding, and naga doesn't emit it. If so, the
  constant and runtime cases differ here.
- **Reach:**
  - **Branch decisions: a gap, not a clean result.** REQ-INT-057's are the wrapper's, and no GPU has run any of them: the
    wrapper has no compute entry point at this commit, and `detect` (`crates/kernel/src/driver/cadence.rs`:71) is a stub
    until TASK-M3-09 and TASK-M3-12. What is measured on this card is canon's and decode's branches (R-547), on identical
    inputs: `decode_` 36 of 36 and `canonicaliser_` 28 of 28.
  - **Division and `√` in a branch input are excluded by rule, and the rule isn't met in writing yet.** Rule 1 of
    `docs/notes/principia_gpu_determinism_note.md`: "No runtime `pow`/`sqrt`/`div` decides control flow". REQ-INT-064 and
    R-34 (corrected by R-547) require `|Δn̂|` and `E_rel` to be "built only from operations every IEEE backend rounds
    alike", in a fixed written form, and `plan/tasks/M4/TASK-M4-03.md`:45 asks for "no runtime `pow`/`sqrt`/`div` feeding
    a decision". But escape is on REQ-INT-057's list, and its `E_rel` has a division: `E_rel = ½|Δv|² − (M_pair + m_b)/d`
    (R-29; `docs/contracts/principia_integrator_contract.md`:737). So has `|Δn̂|`: `n̂(r, m) = (u, v, w)/I`
    (`docs/design/principia_dd_simstate_payload.md`:529), and `crates/kernel/src/shape.rs`:51-69 divides by `m01` and `i`;
    its magnitude is also a `√` unless it is compared squared, and `shape.rs`:64 has a `√` in `s`, feeding `p` and `q`.
    `E_rel`'s `d` is itself a `√`. The integrator contract, :608, calls both "shared branches
    whose inputs are multi-op arithmetic". Neither written form exists, and nothing read here reconciles the two
    formulas with the rule.
  - **So the risk is open, not absent.** A driver whose quotients or roots are one ulp off is the case where
    `(M_pair + m_b)/d` could flip `E_rel > 0` near zero, or `n̂` could flip `|Δn̂| < tau`. Neither is measured: neither
    input is on any GPU at this commit.
  - **One same-input difference the division measurement doesn't cover; reading (b) does.** RQ-687's failure: on identical inputs, setting off, in the
    compute decode stage, the card and not Metal or lavapipe gives `rho_angle` 0 where f64 gives π/2. The cause is `sin`,
    isolated on the card, not division. By #393's diagnosis (inferred) the card takes R-437's `p = q = 0` select where
    the others don't. This entry uses "branch decision" for the wrapper's, REQ-INT-057's, and
    `docs/design/principia_dd_decoder.md`:76-77 and :313 treat such selects as choosing a value ("no branch"), so by the
    letter this isn't one. `crates/kernel/src/chart/shape_sphere.rs`:52-55 does call it "a backend fork", so it is
    disclosed here.
  - **Trajectories:** not measurable, as above. But division is already on the GPU in the continuous decode, canon and
    encode outputs (for example `crates/kernel/src/decode/config.rs`:78-84 and `canon.rs`:405), and these set every GPU
    trajectory's start. The measured decode reach is #400's `shape_sphere` fuzz, 52.05, 28.55 and 17.12 eps on the RTX,
    Metal and lavapipe against 256, cause unsettled (RQ-687). The divisions M4 will add include `physics/force.rs`:19-21,
    `occupant/kdk.rs`:65-74, `occupant/rk4.rs`, `driver/advance.rs`:125 and `driver/welford.rs`; the default occupant is
    Heggie × RK4 (`docs/contracts/principia_integrator_contract.md`:116, R-560).
- **The conflict, which stays open:**
  - **That it means branch decisions:** R-84, which R-297 cites for this claim, says "`parity_contract` governs" and
    speaks only of branch decisions; parity § 4 says "as this contract states them"; REQ-INT-057 leaves continuous values
    free. Read so, the corpus comes close to settling question (1).
  - **That it means more:** the human's "bit-identity and identical branch decisions" names two things, and R-297's
    Still in force line sets bit-identity against tolerances: in the browser it "no longer holds", and runs are "measured,
    not bit-exact", which says little if native bit-identity were only about branch decisions. REQ-SYS-074's verify
    asks for every division's bits on its two named jobs; the two test comments widen that, and have no authority beyond
    the requirement.
  - **What weighs on the first side for continuous words:** parity § 4 expects "driver math" to differ, and the Standing
    caveat explains the spike's matching continuous words by one shared compiler. So the two sides aren't even: nothing
    in the corpus's design text promises continuous bits across drivers.
  - Neither side settles it, so question (1) is the owner's. The card fails the second reading. On the first, only
    canon's and decode's branches are measured, on this card, and REQ-INT-057's are unmeasured on every GPU. "Compiled
    Off" on Vulkan records wgpu's path, not what the driver did.
- **The questions:** (1) is "bit-identity" with the setting off a claim about branch decisions, or about every Vulkan
  driver's continuous results, division the one operation measured? (2) what should the header record where a driver rewrites division with no switch?
- **Options seen:** (a) branch decisions and the Tier B fields only, which holds only if `E_rel` and `|Δn̂|` get
  written forms that take no driver's division or `√`, a gap still open and untested on any GPU; the probe then stays a
  measurement on its two named jobs, the "every backend" comments narrow, and the header keeps `StageMode::Off` with its
  doc saying what it records, wgpu's path, no driver observed; (b) every continuous result on every driver, division the
  one operation measured: this card fails R-297. The reading reaches past division: `√` (`physics/pairs.rs`'s `norm` and
  `cube`, which feed `force.rs`:19-21), contraction of `a·b + c`, and the builtins (`canon.rs`:67-69; parity :97). So the
  project's own division wouldn't deliver it, and the probe passing on Metal and lavapipe doesn't show continuous
  bit-identity there either. It needs the project's own arithmetic for each, or a list of supported drivers, and no
  switch is known to give correct rounding (from memory, as above). The cost of a software division in the force loop
  is unmeasured. The header records `Off` only where that is shown, and `Unknown` elsewhere;
  (c) bit-identity is claimed only on the drivers measured to give it, and any other Vulkan driver records
  `StageMode::Unknown`, as `Api::Dx12` and `Api::Webgpu` do at `compute.rs`:59 and as R-303 does for the browser. R-303
  also disables the Run window's control and holds runs to Tier-N only, and whether that carries over to every native
  Vulkan driver but lavapipe is part of this option; (d) another, stated by the owner.
- **What (a) and (c) leave:** the payload compatibility signature and the sim key carry the compute setting and no driver
  (`docs/contracts/principia_caching_contract.md`:14, :36). So the same sim key gives different continuous bits per
  driver, and `SimState = f(IC, sim key)` (`docs/contracts/principia_canonical_spec.md`:82, :116) then holds per driver.
- **How this ruling and RQ-687's depend on each other:** under option (b), RQ-687's `sin` failure is also a failure of
  R-297, so RQ-687's option (c), neither, stands only with (b)'s list of supported drivers.
- **Measurements that would settle more:** both fuzzed sets' result words on the card (worst ulp, and whether the result
  is a function of `d` times `n`); the halfway probe with 255.0 read from a buffer, beside the literal; the setting on;
  the same probe on another Vulkan driver. RQ-690's probe, `(a + K) * k` in the compute stage, is one more.
- **Applied meanwhile:** nothing. No test, tolerance or comment changed.
- **Waits:** nothing in the build loop: CI's Metal and lavapipe jobs are green at this commit, and no Windows job gates
  anything. The ruling bears on M4's GPU march and on REQ-VAL-099's parity gate, which a real non-Metal GPU serves and
  lavapipe doesn't (R-110).

## RQ-686: The near-tie reference on a Vulkan driver other than lavapipe: one reference per backend means per API, and the RTX shares lavapipe's API but not its arithmetic *(physics; silence: R-296, R-376, REQ-VAL-176 and REQ-VAL-138 name no Vulkan driver but lavapipe; R-649; issues #390, #397, #399; context: the Windows RTX 2080 Super run, RQ-685)*

*Queued 4 Nov 2026 by the orchestrator's filing task, from the Windows GPU run (#390). For the owner: yes, it changes
what a golden near a tie is compared with. Nothing is chosen. The run's record, the 13 failures and the
hardware-health arm are in RQ-685.*

- **File, section:**
  - `plan/requirements.yaml`, REQ-VAL-176: "must keep one reference per backend, and the runner must compare it only with
    the reference for the backend it rendered on, naming that backend (R-269, R-287, R-296)"; its parenthesis names the
    cause: "a value within an ulp of a tie can differ where the display shaders compile with fast-math, Metal via wgpu".
  - `decisions.md` § "R-296 — …": "goldens near a tie use per-backend references", and its Result: "on lavapipe, R =
    `(x + 0.5) / 255.0` matches correctly rounded f32 division; on Metal, it matches `(x + 0.5) × f32(1/255)`, because
    wgpu 30 compiles MSL with the default `MTLCompileOptions`, which has fast-math on".
  - `decisions.md`:5654, § "R-376 — …": REQ-VAL-138 is "0 steps, on every native backend CI renders on (lavapipe, Metal)",
    and "a golden near a tie keeps one reference per backend"; R-296's Still in force line: "the half-way fixture's
    references are confirmed, no longer proposed".
  - `docs/contracts/principia_parity_contract.md`:151, § 4: "a value within an ulp of a tie can differ on a backend whose
    display shaders compile with fast-math (Metal via wgpu), so goldens near a tie use per-backend references (R-296)".
  - `xtask/src/golden.rs`:50-52: per-backend references are "named by the backend (`metal`, `vulkan`)".
    `fixtures/golden/BASELINES.md`:28: `quantise/halfway@vulkan`, "on lavapipe".
- **Measured (#397, #399):**

  | | RTX 2080 Super (Vulkan) | Metal | lavapipe (Vulkan) |
  |---|---:|---:|---:|
  | `quantise/halfway` RGB sha256, first 16 hex | `db057b4b817dd461` | `db057b4b817dd461` | `4a66e83dbb876baa` |
  | against its own API's reference | **FAIL**, step 1, 24064 of 65536 px | step 0 | step 0 |
  | Exact f32 ties in R | 16896 px, 66 columns | 16896 px, 66 columns | 65280 px, 255 columns |
  | `quantise/halfway_automatic` RGB sha256, first 16 hex | `d30f56643f06a081` | `7c0760e5a4b04ec7` | `4a66e83dbb876baa` |
  | against its own API's reference | **FAIL**, step 1, 35396 px | step 0 | step 0 |
  | Quantised against automatic | 57328 px, step 1 | 8704 px, step 1 | 0 px |

  - Both hashes are the same on all 6 golden runs. The tolerance is REQ-VAL-138's 0. The other 149 golden cases are
    byte-identical on all three backends.
  - `quantise/halfway`: the card renders Metal's bytes. `24064 = 94 × 256`, R-296's 94 columns. A CPU run of
    `(x + 0.5) × f32(1/255)` reproduces the 94 columns and the 66 tie columns, and correct rounding gives lavapipe's 255.
    So the cause reads as RQ-685's division, in the fragment stage, with no fast-math flag set by anything we control.
    That is inferred, as in RQ-685: what is measured is the hash equal to Metal's, and the reproduction is on the CPU.
    `golden_halfway_quantise_rounds_half_to_even` passed, so the rounding is correct.
  - `quantise/halfway_automatic`, the control case: against `vulkan.png` 35396 px (R 32512, G 3072, B 2779); against
    `metal.png` 65302 px (R 65280, G 3072, B 2779). Every difference is one step, the render lower than the reference.
    Inferred, not measured: not the division. `floor(v × 255 + o)` with one fitted parameter, `o ≈ 0.4556`, gives 35355 and
    57287 px against 35396 and 57328, which #399 reads as a conversion biased by about 0.044 of a level. The golden path
    doesn't use this output.
  - **A qualification of #399's fit.** #399's fit was made before the per-channel counts were posted, and the channels
    don't bear it out. Recomputed on `halfway.wgsl`'s formula with CPU `exp` and `pow`, at `o = 0.4556`: G 2560 and B 3214
    against the measured 3072 and 2779. R gives 32512 at each of the eleven offsets tried from 0.001 to 0.499, so R carries
    no information on the bias and shows only that its ties round down. The size rests on G and B, and they disagree: G
    gives 3072 at `o ≈ 0.445`, a bias of about 0.055, and B comes nearest at about 0.465, a bias of about 0.035. So the
    totals fit better than the channels do, and no single bias is supported. The card's `exp` and `pow` may differ from
    the CPU's, and these numbers can't separate those.
- **The gap:** the corpus keys a near-tie reference by "backend", the runner keys it by API, and the corpus gives Metal's
  fast-math as the reason a case differs. `vulkan.png` was made on lavapipe, and on this card the same API gives Metal's
  bytes for the quantised case and bytes of its own for the control. No document says what a near-tie case is compared
  with on a Vulkan driver that isn't lavapipe, and R-376's value is stated for the backends CI renders on.
- **The question:** what reference applies on a Vulkan driver other than lavapipe?
- **Options seen:** (a) references keyed by driver, not API: the `vulkan` references are lavapipe's alone, and another
  driver gets its own, under a gate decision (R-110's "no re-baselining without a gate decision"). It must say what
  "driver" keys on, the name or the version too, since the rewrite would be a driver compiler's, and no CI job could hold
  such a reference, so it couldn't be checked per commit; (b) near-tie cases are held only on the backends CI renders on (R-376's
  scope), and on any other driver their result is reported, not compared to pass; (c) a driver may match any of the
  existing references, which passes `quantise/halfway` here and not `quantise/halfway_automatic`; (d) another, stated by
  the owner. Each of (a) to (c) leaves REQ-VAL-176's text to be reworded.
- **Applied meanwhile:** nothing. Nothing is re-baselined.
- **Waits:** nothing in the build loop: every golden case passes on CI's Metal and lavapipe jobs.

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

## RQ-698: `.claude/commands/*.md` joins `plan/tools/rulings.py`'s scanned files once the ruling those commands cite is on `main` *(a follow-up of PR #409, the rulings commands' tooling; code review 5771543937)*

*Filed 4 Nov 2026 by PR #409's fix round, so the follow-up outlasts the PR. Nothing here is for the owner: a mechanical step, decided per R-369, which waits only on a merge.*

- **File, section:** `plan/tools/rulings.py`, `REF_GLOBS`: "The files whose R-n and RQ-n references must resolve", which names `.claude/agents/*.md` and not `.claude/commands/*.md`; `.claude/commands/rulings-apply.md` § "1. Whose words, and may this run" cites the standing rule on `main` being red, which is on `rulings/2026-11-03-port3` and not yet on `main`.
- **The gap:** an R-n or RQ-n reference in the two commands is checked by nothing, so one can go stale. Added now, the glob fails `plan/check_plan.py` on that citation.
- **Options seen:** (a) add `.claude/commands/*.md` to `REF_GLOBS` in the first PR after `rulings/2026-11-03-port3` merges, and drop this entry's note from `.claude/commands/rulings-apply.md` that the rule is on that branch.
- **Applied meanwhile:** none: the commands' references are read by review only.
- **Waits:** `rulings/2026-11-03-port3`'s merge.
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

## RQ-697: What the ionisation gate's total `E > 0` reads and by what form, at `ε = 0` as well as `ε ≠ 0` *(physics; silence; R-32, R-34, R-547, R-654, R-670; `docs/contracts/principia_integrator_contract.md` Part 4, Part 7; `docs/contracts/principia_parity_contract.md` Tier B; context: TASK-M9-39, TASK-M3-12, TASK-M4-03, REQ-EVT-006)*

*Queued 4 Nov 2026 from physics review 5771573790 of PR #408, F8. For the owner: yes, it changes results: whether a state beside `E = 0` is labelled ionised, on every backend alike. Nothing is chosen.*

- **File, section, quoted:**
  - `decisions.md` § "R-32 — …": "All three pairwise energies > 0, all separations growing, total `E > 0`, evaluated with the escape rule's settling."
  - `plan/requirements.yaml`, REQ-EVT-006: "gated on all three pairwise relative energies positive, all three separations growing and total `E > 0`, evaluated with the escape rule's settling."
  - `decisions.md` § "R-654 — …", the owner's words: "Energy and drift are defined under the softened potential."
  - `decisions.md` § "R-34 — …": "every branch input that consumes multi-op arithmetic, the change-11 `|Δn̂|` and `E_rel` included, is enumerated rather than relying on a backend switch"; "such an input is built only from operations every IEEE backend rounds alike, with no intermediate that can be subnormal (R-547)."
  - `docs/contracts/principia_integrator_contract.md` Part 4: "a branch decision may depend only on comparisons against compile-time constants, or against values every backend receives bit-identically (the sim key's threshold table, R-502), and single-rounded arithmetic — never a runtime transcendental."
  - `docs/contracts/principia_parity_contract.md` Tier B: "a GPU's `pow`/`sqrt`/`div` and libm's differ by 1–3 ulp *in every math mode*".
  - `plan/tasks/M4/TASK-M4-03.md`, Goal: "`d²` for the substep bucket and collision, the windowed `|Δn̂|` and `E_rel` for escape, the cap and horizon decisions, and the rest."
  - `docs/design/principia_dd_decoder.md` § 3.6: "`E₀` is **derived** from it (`K₀ + V₀`), not stored (R-86)."; "`E₀` here must agree with the kernel's `E_0` at t=0 — that agreement is a cross-check view, not an assumption."
  - `decisions.md` § "R-133 — …": "the decode stage's E₀ = K₀ + V₀ (R-86), not SimState.E_0."
  - `docs/design/principia_dd_simstate_payload.md`: "`E_0`, `Lz_0` (drift references)"; `docs/contracts/principia_integrator_contract.md` Part 1 gives its formula, `E_0 = Σ ‖pᵢ‖²/2mᵢ − G Σ mᵢmⱼ/‖rᵢ − rⱼ‖`.
- **The silence:** the corpus does not say whether the gate reads the live `E` or the stored `E_0`, or by what form it decides the sign, and no list names it as a branch input. The condition is not redundant: positive pair energies give only `E > −Σ (mᵢmⱼ/rᵢⱼ)·(m_k/M)`, so total `E` can still be negative. Live, `E > 0` at `ε ≠ 0` is `K > Σ mᵢmⱼ/√(dᵢⱼ² + ε²)`, with three different radicals, and at `ε = 0` it is `K > Σ mᵢmⱼ/dᵢⱼ`, with three `1/d` terms. Two quantities share the name. The decode's `E₀` is derived, `K₀ + V₀`, and not stored (`principia_dd_decoder.md` § 3.6; R-86). The stored one is the kernel's `E_0`, a `SimState` field formed at `t = 0` from integrator contract Part 1's energy, and it is the stored energy a gate would read (R-133 keeps the two apart). It takes the same three terms, so its sign can differ across backends on identical inputs near `E = 0`, and `K_0 + V_0` from the `ICDescriptor` is a third reading with the same sign problem. The live `E` and `E_0` also differ along a run, by the drift.
- **The question:** which quantity does the gate's total `E > 0` read, and by what form is its sign decided, so that the decision agrees across backends? The answer reaches the `ε = 0` gate, which TASK-M3-12 builds, as well as TASK-M9-39's.
- **Options seen:** (a) the live `E`, the decision an enumerated exception to the comparison-only rule, a backend difference beside `E = 0` recorded and reported; (b) the stored `E_0`, with the same exception; (c) `E_0`'s sign computed once on the host at f64 and given to every backend as a bit-identical value, a value the gate compares against rather than computes, which names no exception but needs another wall changed: `E_0` differs per sample across a quad, so it takes one value per sample sent to the GPU and a host f64 decode of every sample, against W12 (`plan/reviewers/physics.md` § 2: "No per-sample payload, tree or reduction array crosses GPU↔CPU") and the Dispatch row of `docs/design/principia_systems_architecture.md` § 3 (uniforms "per frame / per quad"; R-502's table, the precedent, is per sim key), so it stands only if the owner also changes that wall; (d) R-32 changed so that the gate has no total `E` condition, which changes which states it labels; (e) another, stated by the owner. A radical-free sign test for the live `E` exists in principle, isolating one radical and comparing squares for each of the three, but at f32 its high degree in `d` hits R-547's subnormal bar and overflow at ordinary separations.
- **Applied meanwhile:** nothing: the gate is not built (no `crates/` file names it), and dispatch refuses `ε ≠ 0` (R-654; TASK-M3-06), so no softened run exists.
- **Waits:** TASK-M9-39 (REQ-INT-111, REQ-SYS-094, REQ-SYS-095); TASK-M3-12 (REQ-EVT-006), which builds the gate and cannot write its third condition at `ε = 0` without this ruling; and TASK-M4-03 (REQ-INT-061), which writes every kernel branch input's form into `branch_inputs.rs`, what it and REQ-INT-061 hold of this input following the ruling. REQ-EVT-006 is not the whole of the reach: those two tasks are it at `ε = 0`, and what depends on them waits through them. *Was:* "their waiting is for the ruling to say" (applied per R-369).
- **Notes** (physics review 5771861276 of PR #408, for the port of the ruling): (1) `softened_tier_l`, REQ-INT-111's verify and TASK-M4-03's note and REQ-INT-061 now defer to this entry, and under option (a) or (b) those lines are re-read and take the ruling's exception, since the decision can then fork; all three tasks wait here, so nothing is built against them first. (2) TASK-M3-12's `ionisation_gate`, with its fixtures `three_way_unbound` and `one_bound_pair`, has no state with all pairs unbound, all separating and `E < 0`, so dropping the `E > 0` test passes it. The port adds that state as a control, if the condition stands: unit masses on an equilateral triangle of side `r`, each moving radially outward at speed `u` with `u²` just above `4/(3r)`, giving all pairs unbound and `E ≈ −1/r`. (3) REQ-INT-057 and REQ-VAL-059 require escape decisions and the terminal label bit-identical across backends, and ionisation is `escape` with `detail = 3`, so under option (a) or (b) they need the ruling's exception too.

## RQ-702: What refresh rate a browser run started from the measurement panel's button should read, when the rate was measured once at page load *(silence; TASK-M8-86, REQ-PERF-115; `plan/tasks/M8/TASK-M8-86.md` § "Deliverables")*

*Queued 5 Nov 2026 from Codex thread `PRRT_kwDOUpUQ5c6z3aWc` of PR #368, code review 5779560269 F1 and gui review 5779587987 G3. The native half of the same finding is fixed there. Nothing is chosen.*

- **File, section, quoted:**
  - `plan/tasks/M8/TASK-M8-86.md` § "Deliverables", the frame statistics: "The display's refresh rate is measured apart from the app's work: in the browser, an empty `requestAnimationFrame` loop on the main thread, run for 2 s before the warm-up with the app idle, its median interval".
  - `web/skeleton/loader.js:35`: "The empty requestAnimationFrame loop that measures the display, run before the app starts, so the app's frames cannot slow it."
  - `web/skeleton/runner/common.mjs:24`: `url.searchParams.set("run", run);`
- **The silence:** the task's "before the warm-up" does not say how long before. The loop runs once, as the page loads and before the app starts, and the rate it gives stands for the session.
- **What a panel-button run reads:** that same rate, taken at page load however long ago, with nothing to mark it. A window moved to another display in between reads a stale rate, and its "capped" marks read that rate. `display_changed` is null in the browser, which watches nothing: the worker layout has no window rect, and the rate cannot be read again without another idle loop, which needs the app idle.
- **Why `?run=` runs are unaffected:** the runner scripts start every run by that parameter (`common.mjs:24`), so the loader takes it with the idle loop's stamps, the run starts at the app's first step, and its warm-up follows the loop at once. Every measurement TASK-M8-96 takes goes this way. Nothing moves the window in them.
- **Options seen:** (a) keep it, and state in the panel that a button run reads the page-load rate; (b) run the idle loop again before a button run's warm-up, the app paused for it; (c) mark a button run in the report, so that it is not read as a measurement.
- **Applied meanwhile:** none: the rate stands as measured at page load, and measured runs go through `?run=`.
- **Waits:** nothing. TASK-M8-96 uses `?run=` only; TASK-M8-37, if it keeps the panel, meets this question.
