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
