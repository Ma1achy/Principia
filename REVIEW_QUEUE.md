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
- **Deferred:** deferred to M7 by owner 11 Nov 2026 (R-964, the owner: "RQ-391: defer:M7").

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
- **Deferred:** deferred to M3 by owner 11 Nov 2026 (R-957, the owner: "RQ-577: defer:M3").

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
- **Deferred:** deferred to M9 by owner 11 Nov 2026 (R-960, the owner: "RQ-643: defer:M9").

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
- **Deferred:** deferred to M4 by owner 11 Nov 2026 (R-958, the owner: "RQ-629: defer:M4").

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
- **Deferred:** deferred to M9 by owner 11 Nov 2026 (R-961, the owner: "RQ-681: defer:M9").

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
- **Deferred:** deferred to M4 by owner 11 Nov 2026 (R-959, the owner: "RQ-687: defer:M4").

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
- **10 Nov 2026 (R-941):** the owner, in their reply to the report of 10 Nov 2026:
  "RQ-779 (defaults): per rule 2. The mass simplex saturates, like the latent mass plane. I'll confirm the
    rest by eye when the real charts render."
  Settled (R-941): the defaults follow rule 2 (R-932: no edge blank by default; a true edge saturates, a second sheet
  unfolds, a repeating coordinate wraps, the mechanism per edge physics review's call), and the mass simplex's sides
  saturate, not the table's proposed "clamp in the chart's own coordinates". **Still open, so the entry stays:** every
  other row, which the owner confirms or changes by eye when the kernel's charts render (N14's screenshots, row by row).
  Nothing waits on it: N14 builds each row to rule 2 meanwhile.
- **Deferred:** deferred to M2 by owner 11 Nov 2026 (R-963, the owner: "RQ-779: defer:M2").

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
- **Deferred:** deferred to M2 by owner 11 Nov 2026 (R-956, the owner: "RQ-783: defer:M2").

## RQ-786: The domain an untransformed slot input inherits from a field whose manifest range has exactly one finite end *(silence; a one-way door, a default the legend samples over; R-815, R-53; REQ-GEN-019; gui_state_contract § 3; render_gui_spec § 10.1; context: TASK-M7-13, TASK-M8-22, TASK-M8-93)*

*Queued 11 Nov 2026 by TASK-M7-13's fix round, from code review 5852530981 of PR #481, F1; each quote re-read on the
PR's branch. Nothing is chosen here.*

*In plain English: a colour node that does not say what range of values it expects takes that range from the field
plugged into it. The owner ruled what happens when the field's range has two finite ends (use them) and when it has
none (use 0 to 1). Some fields have one finite end only, such as a kinetic energy that is at least 0 with no upper
limit. Nothing written says what range the node takes then. The legend, built later, draws each node over that range,
so the choice changes what the legend shows.*

- **File, section:**
  - `decisions.md` § "R-815 — An untransformed slot input's default domain is "inherits", resolved per wire at
    assembly, an unbounded field taking `(0, 1)` …": "the registry records "inherits" for an untransformed input, the
    domain resolved per wire at assembly from the wired field's manifest range, and a field whose range is
    `Range::UNBOUNDED` takes `range_norm`'s `(0, 1)`, the generator's own fallback."
  - `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment*
    side; compute occupants are Rust build variants": "an untransformed input's registry entry records "inherits",
    and its domain is resolved per wire at assembly from the wired field's manifest range; a field whose range is
    unbounded (`Range::UNBOUNDED`) takes `range_norm`'s `(0, 1)`, as the generator's fallback does."
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library": "`[lo, hi]` is the ledger range's
    finite ends; a step index's is `[0, horizon_steps]`"; "the one-signed `K_0` `[0, ∞)` and `V_0` `(−∞, 0]` are
    `lin`, on `ramp_viridis`"; and "An end with no finite bound takes the measured end, and a field whose ledger
    range is unbounded defaults to `RANGE_AUTO = 1`".
  - `plan/requirements.yaml` REQ-GEN-019, its verify: "on a wired node its domain equals the manifest [lo, hi] of its
    field, and a field whose range is unbounded takes (0, 1) (R-815)".
  - `crates/ledger/src/schema.rs`, `Range::UNBOUNDED`: both ends `Bound::Unbounded`. A range with one finite end is
    not it.
- **The silence:** R-815 and the contract give `(0, 1)` to `Range::UNBOUNDED` only, and `[lo, hi]` needs two finite
  ends. For a field with one finite end, § 10.1 says what `range_norm` does in a view (the finite end is kept and the
  other is the measured one, per draw), which is not `(0, 1)`. A registry domain resolved at assembly is static: it
  cannot hold an end that is measured per draw. No document says what an untransformed input's domain is on such a
  wire. A range whose non-finite end is written as a closed or open `±∞` rather than `Bound::Unbounded` is in the same
  place.
- **The question:** what domain does an untransformed input have when the field wired to it has exactly one finite
  end?
- **Options seen:**
  - (a) `(0, 1)`, as for an unbounded field: R-815's fallback read as "no finite `[lo, hi]`". Consequences: static, no
    new mechanism; the legend samples `K_0` over `(0, 1)` whatever its values are, and `V_0`, which is never positive, over
    a range it never reaches; the field's one known end is not used.
  - (b) The finite end kept, the other end the measured one, as § 10.1 has `range_norm` do. Consequences: the domain is
    no longer a pair of numbers fixed at assembly. `inputDomains` would need an end that reads "measured", resolved
    per draw from the reduction that fills `u_range`, and the legend (TASK-M8-22) would sample over a range that moves
    with the data, or over the frozen one (R-581) when a range is frozen. R-815's "resolved per wire at assembly" is
    amended for these fields.
  - (c) The wire refused: an untransformed input cannot take a field with one finite end; the node must declare its
    domain (`// @input <name> [<lo>, <hi>]`) to be wired to it. Consequences: static and explicit; the generated views
    are not affected (they read their field from `ctx`, not the wire), but a user's untransformed colour node cannot
    be wired to `K_0` or `V_0` until it declares a domain, a validation error the stain editor must show.
  - (d) Another definition, stated by the owner.
- **Recommended:** (b) in substance, held until the legend's task, with (c) meanwhile: § 10.1 already says what these
  fields' range is everywhere else in the instrument, so a legend drawn over `(0, 1)` for a field whose values are
  nowhere near it would misstate the map; but (b) needs a mechanism this task has no deliverable for. If the owner
  wants a static rule now, (c) is the one that cannot draw a wrong legend.
- **Measurement:** the measurable alternative: when the legend lands (TASK-M8-22), for each manifest field with one
  finite end that a source can be wired to, the share of a reference run's finite values of that field lying inside
  `(0, 1)`. Criterion, written before any run: if every such field has at least 99% of its values inside `(0, 1)`,
  option (a) samples the legend where the data is and is adequate; any field below that shows (a) draws a legend off
  the data, and (b) or (c) is needed. The field count itself is a `cargo test` away:
  `registry_input_domains_resolve_per_wire` asserts the manifest has such fields.
- **Applied meanwhile:** nothing is chosen. `render::registry::resolve` returns `Err(Unruled)`, an error naming this
  entry, for an untransformed input wired to a range that neither has two finite ends nor is `Range::UNBOUNDED`, and
  `Entry::domain` gives no domain there; the two ruled arms are built as ruled (TASK-M7-13, PR #481). Nothing shipped
  samples a domain yet: the assembler does not call `resolve`, and the first readers are the legend (TASK-M8-22) and
  the Stain canvas on the real engine (TASK-M8-93), both of which need REQ-GEN-019.
- **Waits:** REQ-GEN-019's arm for a field with one finite end (`rq: RQ-786`); TASK-M8-22's legend over such a wire;
  TASK-M8-93 where it wires one. TASK-M7-13's other arms do not wait.
## RQ-785: At what picture size P2's and P4's pixels are counted: the owner's "more than 8 pixels apart" names none *(silence, a missing definition, R-72; a one-way door, what a reported measurement means; R-951, R-947, R-583, R-649; context: the re-plan addition's measurement task, TASK-M8-49, TASK-M8-58)*

*Queued 11 Nov 2026 with the plan PR for the chart re-plan's addition, as the orchestrator's answer to the first
hand-back asked; each quote below re-read on that PR's branch. Nothing is chosen here.*

*In plain English: before the trace tasks are built, the owner wants two measurements of how a trajectory's trace
behaves on a half-morphed chart. One counts how often two neighbouring points of the trace land "more than 8 pixels
apart"; the other gives a jump "in pixels". A pixel is a different distance on a small picture and on a large one, and
the owner's words do not say how big the picture is. Only the owner may say which size they meant.*

- **File, section:**
  - `decisions.md` § R-951 (the two measurements, P2 and P4), the owner's words: "on the same fixtures and blend pairs,
    at s = 0.25, 0.5 and 0.75, the share of neighbouring trace vertices that land more than 8 pixels apart on the blend,
    set against the same share on each end chart's own trace" and "Do not pick thresholds or fallbacks yourself."
  - `decisions.md` § R-951, *What it decides*, on P2: "the on-screen jump between an end chart's own trace (`s = 0`) and
    the blend's (`s = 10⁻⁶`), median and worst, in pixels per vertex".
  - `decisions.md` § R-583 (one size rule, base px), *What it decides*: "**`RenderState.base_width`** is the figure's
    width in logical px (the scale factor removed) when saved or exported; a spec with no view (the reel, headless) and
    older records take 1024."
  - `docs/design/principia_chart_composition.md` § "17.12 Performance", the one place the chart-path design names a
    picture size for a measurement: "one coarse cover's chart side (1920×1080 at 1/8 linear, 32,400 samples)".
- **The silence:** the measurements run headless, on fixtures with no window. Nothing in the owner's words, in R-951 or
  in the trajectory fixtures (R-919's F18, TASK-M2-82) says how many pixels wide and high the chart's window is drawn
  when a vertex's place is turned into pixels, nor whether "pixels" are R-583's base px. The share P4 reports changes
  with that size: the same trace has fewer vertex pairs "more than 8 pixels apart" on a smaller picture.
- **The question:** at what window size and resolution are P2's and P4's pixels counted?
- **Options seen:**
  - (a) Base px, as R-583 has every mark length: the chart's window drawn 1024 base px wide, the width R-583 gives a
    spec with no view. The corpus names that width and no height for a headless spec, so the owner would still state the
    shape: square, 1024 × 1024, or another.
  - (b) The picture the owner looks at: 1920 × 1080, the size design § 17.12's coarse-cover measurement is written at.
  - (c) Reported at several sizes rather than one, each labelled: P2 and P4 at each of 512 × 512, 1024 × 1024 and 1920 ×
    1080, so the owner reads the share where they would look at the trace.
  - (d) Another size, stated by the owner.
- **Recommended:** (a) with a square window, because base px is the corpus's one size rule and a trace's marks are
  already sized in it, so "8 pixels" then means the same length on screen and in any export. Against it: the owner may
  have meant the screen they morph on, which is (b). The measurable alternative is (c): the report gives both
  measurements at all three sizes, and the owner reads the one they meant; it costs three placements of each trace
  instead of one, and no choice.
- **Applied meanwhile:** nothing. P2 and P4 are not run, and no size is assumed; the measurement task carries a
  definition requirement (R-72) for the size, and R-951's "8 pixels" stands as the owner wrote it.
- **Waits:** the re-plan addition's measurement task (P2 and P4), and through its report TASK-M8-49 and TASK-M8-58,
  which R-951 holds until the owner has seen both.
- **11 Nov 2026, three further questions on the same set-up, asked with the first** (the code review of PR #482, review
  5852985982, F2; the physics review of the same PR, review 5852979675, its point 10). Each is a silence in the same
  place, a missing definition (R-72) only the owner can state, since each changes what their measurement means.
  REQ-CHART-203 (the set-up's definitions) carries them and waits on this entry, as REQ-CHART-202 (the picture size)
  does. Nothing is chosen, and nothing is run until all are answered.
  - **Question 2: what a seeded pixel contributes.** `plan/charts/m2-chart-replan-addition.md` § "A.5 The
    measurements: P2 and P4, reported and not gates; and the proposals nothing adopts": "**16 seeded pixels per
    pair.** The seed is written into N20's task file before any run." and "what a seeded pixel contributes (the draft
    has "over the trajectory fixtures … and 16 seeded pixels per chart pair" and says no more)". The owner's words
    (R-951) have "on the same fixtures and blend pairs" and name no seeded pixel. *Options seen:* (2a) a pinned
    trace: the system the pixel shows on the starting chart, integrated once and placed again at every `s`, measured
    as the two fixtures are; (2b) a hover trace: the blend's own system at that pixel, another trajectory at each
    `s`, as a hover shows while a morph plays; (2c) its starting state only, one vertex on the surface, the
    off-surface numbers coming from the two fixtures alone; (2d) another, stated by the owner. *Recommended:* (2a),
    because P2 sets one trajectory at `s = 0` against the same trajectory at `s = 10⁻⁶`, which needs a fixed system.
    The measurable alternative: the report gives (2a) and (2b) side by side, labelled.
  - **Question 3: whether P2 is also read at the other end.** `decisions.md` § R-951, *What it decides*, defines P2
    at one end: "the on-screen jump between an end chart's own trace (`s = 0`) and the blend's (`s = 10⁻⁶`)". The
    owner's words there: "the whole trace may jump as a morph starts or ends." *Options seen:* (3a) at `s = 0` only,
    as R-951 defines it; (3b) at both ends, `s = 1` against `s = 1 − 10⁻⁶` as well, each reported apart; (3c)
    another, stated by the owner. *Recommended:* (3b): "starts or ends" names both, and it costs one more placement of
    each trace. The measurable alternative is the same report: both are given and the owner reads the one they
    meant.
  - **Question 4: the trace's vertex spacing in time, and the window.** The physics review: "RQ-785 or REQ-CHART-203
    should also name the trace's vertex spacing in time and the window (home, zoom 1?): both move P4's share." P4
    counts "neighbouring trace vertices", and nothing says how far apart in time two neighbours are, nor which
    window of each chart the trace is drawn in. *Options seen:* (4a) the trajectory fixture's own output steps
    (TASK-M2-82), each chart in its home window at zoom 1; (4b) vertices at equal steps of time, the step stated by
    the owner, in the home window; (4c) reported at more than one spacing, each labelled (the fixture's own steps,
    and every second and every fourth of them), in the home window; (4d) another spacing or window, stated by the
    owner. *Recommended:* (4a), the trace as the fixture gives it, with (4c) the measurable alternative: it shows how
    far P4's share moves with the spacing, at the cost of three placements of each trace.
  - **Applied meanwhile:** nothing, as above. **Waits:** as above.

## RQ-787: What a blend's interior pixel shows when its blended masses raise `M01_TINY` between two valid ends: no ruling says, and there "every in-between frame is a valid system" does not hold *(silence, a missing definition, R-72; a one-way door, it changes what a chart draws; R-943, R-926 item 3, R-936, R-932, R-597, R-467, R-944; REQ-CHART-189; physics review 5852979675 of PR #482, P1; context: TASK-M2-90, TASK-M2-81)*

*Queued 11 Nov 2026 by the fix round on the plan PR for the chart re-plan's addition (#482), from its physics review
5852979675, finding P1; each quote below re-read on that PR's branch. Nothing is chosen here.*

*In plain English: a morph moves every pixel's system from one chart's to the other's, and the rule is that every frame
on the way shows a valid system. The masses are moved in a way that can, on some pairs of charts, squeeze two of the
three bodies' masses to almost nothing partway through, although both ends are fine. The decoder cannot build a system
there, so the pixel would go blank in the middle of the morph. The owner's rules say what happens at a chart's edge in
that case, and say nothing about the middle of a morph. Only the owner may say what such a pixel shows.*

- **File, section:**
  - `decisions.md` § R-943 (the general rule for a morph), its heading: "any other path is allowed if both ends are
    exactly the end charts, every in-between frame is a valid system and it is continuous".
  - `decisions.md` § R-936 (what a state blend leaves open), its heading: "the mass block blends the mass coordinates,
    the logs of the mass ratios".
  - `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)", the
    tag table: "`M01_TINY` | the mass block: `M₀₁ < ε_M01` (§3.1), on the emitted masses of every chart, a chart that
    sets its masses directly included".
  - `decisions.md` § R-932 (chart edges), the owner's words: "which mechanism is used on a given edge is physics
    review's call (saturate wherever the edge system itself is not a valid system, such as a zero mass)."
  - `decisions.md` § R-597, its heading: "under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` is genuinely
    degenerate and draws as not a system, with the reason in the readout", which R-932 turns into a saturation at
    that link's edge.
  - `docs/design/principia_chart_composition.md` § "17.11 Honesty", the bullet "Out of feasible", its dated note,
    case 2, which sets out the case.
- **The case, from the physics review:** masses blend linearly in `μ_k = ln(m_k/m₀)`. Then `M₀₁ = g/(1 + g)` with
  `g(s) = e^{−μ₂(s)} + e^{μ₁(s) − μ₂(s)}`, a sum of exponentials of linear functions: convex in `s`, so it can dip
  below both ends. Example: end A `μ = (−L, 0)` (`M₀₁ ≈ ½`), end B `μ = (L, L)` (`M₀₁ ≈ ½`); at `s = ½`,
  `μ = (0, L/2)` and `M₀₁ = 2/(2 + e^{L/2})`, below `10⁻⁵` for `L > 24.4`. Both ends are valid pixels of a chart that
  sets its masses directly (the edge-reaching simplex link). Latent-link ends cannot reach it (`|μ| ≤ 5` gives
  `g ≥ e^{−5}`).
- **The general condition, from the physics re-check (review 5853427038 of PR #482, P9; 11 Nov 2026):** the example is
  right but is not the typical case, and no extreme mass is needed. With `r_k = m_k/m₂` the blend gives
  `r_k(s) = r_kA^(1−s)·r_kB^s`, and `g = r₀ + r₁`. Take two ends on or just outside the band's edge, `g = g*`, whose
  light pair has the ratios `q` and `1/q`: then `g(½) = g*·2√q/(1 + q)`, below `g*` for every `q ≠ 1`. In masses: end A
  ∝ `(1, 0.01, 10⁵)` and end B ∝ `(0.01, 1, 10⁵)` both have `M₀₁ = 1.01·10⁻⁵`; the midpoint ∝ `(0.1, 0.1, 10⁵)` has
  `M₀₁ = 2·10⁻⁶`. A brute-force search over ends with every mass at least `10⁻⁹` and `M₀₁ ≥ 10⁻⁵` finds midpoints down
  to `7·10⁻⁷`. It reaches saturated ends too: a saturated end blends its saturated state (condition (i) of
  chart_composition § 17.11's table), which sits on the band's edge, so wherever both ends saturate a pixel to states
  with different `m₁/m₀`, the interior dips for every `0 < s < 1`. The region can be the whole overlap of the two
  ends' saturated bands, not a few deep-zoom pixels. Latent-link ends still cannot reach it.
- **The silence:** R-932 and R-597 say what a chart draws at its own edge where `M₀₁` is tiny, and each edge has a
  declared kind. A blend's interior pixel is on no edge, and no edge kind applies to it. R-943 requires every
  in-between frame to be a valid system, and on the default path of the mass block (R-936) this pixel is not one. No
  ruling says what it shows.
- **The question:** what does a blend's interior pixel show where its blended masses give `M₀₁ < ε_M01` and both ends
  are valid?
- **Options seen:**
  - (a) It saturates, as rule 2 treats a zero mass at an edge (R-932, with R-597): the pixel shows a valid system and
    is never blank. What it saturates to is then to state, and R-932 leaves the mechanism to physics review: the
    masses held at the band's edge with the rest of the blend unchanged, or the pixel's own system at the nearest `s`
    on its path where `M₀₁` is still large enough. *11 Nov 2026, physics review's condition on any mechanism (review
    5853427038, P9):* it is continuous in `s` and across pixels, and equals the plain blend where the dip begins and
    ends (R-943's continuity). Holding the blended `m₁ : m₀` and lifting the pair to `M₀₁ = ε_M01`, the rest of the
    blend unchanged, meets it. "The pixel's own system at the nearest `s` where `M₀₁` is still large enough" **fails
    continuity** and is not a mechanism physics review would pass: at the middle of the dip the shown system jumps
    from the entry state to the exit state.
  - (b) It is blank, a validity failure (R-467): tagged `M01_TINY`, drawn as not a system with the reason in the
    readout, as R-597 had it. A blank then opens mid-path on these pairs, an exception to R-943's "every in-between
    frame is a valid system" and to R-944 that the owner would be accepting.
  - (c) Another path for the mass block on such a pair: R-943 allows any path whose in-between frames are all valid,
    so the masses could take a path that does not dip (straight on the simplex, in the masses themselves) where the
    log-ratio path would. That changes R-936's answer for those pairs. *11 Nov 2026, two facts from physics review
    (review 5853427038, P9):* a path straight in the masses never dips (`M₀₁` is then linear in `s`); but applied only
    to the pairs that dip it makes a seam between pixels on the two paths, so (c) is clean only if it replaces R-936's
    log-ratio path for every pair.
  - (d) Another, stated by the owner.
- **Recommended:** (a), because the owner's rule 2 names the same failure at an edge ("saturate wherever the edge
  system itself is not a valid system, such as a zero mass") and R-943 asks for a valid system on every frame; the
  mechanism would then go to physics review, as R-932 has it. Against it: no ruling yet reaches inside a blend, and (c)
  removes the case rather than covering it. The measurable alternative: TASK-M2-90's fixture reports, on the pairs of
  shipped charts that set their masses directly, the share of pixels and the range of `s` where `M₀₁ < ε_M01`, and the
  owner chooses with that number. The dip needs no extreme mass: it opens wherever two ends on or just outside the
  band's edge differ in `m₁/m₀`, saturated ends included (the general condition above), and the share is not known
  until measured, so the measurement matters more. *Corrected 11 Nov 2026 (the physics re-check, review 5853427038,
  P9). Was:* "the case needs `L > 24.4` in the example, so the share may be small."
- **Applied meanwhile:** nothing new. The decoder's rule stands as written: such a pixel is tagged `M01_TINY`, never
  dropped and never NaN. The blend chart's fixture (REQ-CHART-189, TASK-M2-90), in its three arms (the `L > 24.4`
  example; the band with modest ratios; both ends saturated), asserts the tag and that both ends are
  valid, and asserts nothing about what is drawn; no saturation of a blend's interior pixel is built. Nothing is
  built at all while R-923's hold stands.
- **Waits:** REQ-CHART-189's arm for this pixel (`rq: RQ-787`), and through it TASK-M2-90 (the blend chart) for that
  pixel alone; TASK-M2-81's `path_blank_ends` reads the answer for a blend's interior pixel.
  *Added 11 Nov 2026 (the code review of PR #482, review 5854054751, N6):* the fixture's arm with both ends saturated
  is now REQ-CHART-211's (a blend of two saturated pixels raises the tag; `rq: RQ-787`), closed by TASK-M2-103
  (saturate in the CPU evaluator), which builds the saturated pixels it blends; so REQ-CHART-211 and, through it,
  TASK-M2-103 wait for that pixel alone as well. REQ-CHART-189 and TASK-M2-90 keep the other two arms. No option
  changes.
