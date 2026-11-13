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
- **11 Nov 2026 (R-957):** deferred to M3 again; the owner: "RQ-577: defer:M3".

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
- **11 Nov 2026 (R-958):** deferred to M4 again; the owner: "RQ-629: defer:M4".

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
- **11 Nov 2026 (R-961):** deferred to M9 again; the owner: "RQ-681: defer:M9".

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
- **11 Nov 2026 (R-959):** deferred to M4 again; the owner: "RQ-687: defer:M4".

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
- **12 Nov 2026:** RQ-801 (records made past the switch to the linearised decoder, and across machines, recreating
  bit for bit) is an addition to this entry, filed from the chart audit the owner ordered ("RQ-783's addition"); it
  is read with this entry at M2 (R-956). This entry is not moved.
- **13 Nov 2026:** RQ-801 is answered, by R-995 (the owner: "RQ-801: (a)."): a record made past the switch names its
  route, exact or patch, and for a patch the quad's level and address, and with R-991 (the pinned maths library) it
  re-runs bitwise on every machine. That settles the addition's patch and cross-machine cases, and records written
  after the switch carry their route ("N2's record contract" above); this entry's own question, records made before
  R-926's route, is unchanged and stays deferred to M2.

## RQ-802: How the precision warning tells saturation from the floor once `J_D` is the dual slope: REQ-GUI-012's test reads "f64 J_D's column along k is 0", which a dual σ′ does not reach until about 745, where the central difference reached 0 past about 37 *(conflict; a one-way door, a definition the warning rests on (R-72), and it extends R-935's words or replaces RQ-553's test; R-935, R-54, R-570, R-599, R-633, R-72; REQ-GUI-012, REQ-SCHED-062, REQ-DEC-034; context: TASK-M6-07, TASK-M6-21, RQ-553, the chart audit's F12)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F12, with both of the audit's reviewers'
corrections; each quote re-read on origin/main 54fa96fc. Option (b)'s slope condition is the owner's definition, 12 Nov
2026, verbatim: "equal f64 ICs at distinct sample points are saturation only if |J_dual,k|·h_633 < ½ ulp_f64(x_k) for
every component (dual slope, R-633's step; no central difference); otherwise they are the f64 floor and warn. Add an arm
past the f64 floor to the measurement. Physics confirms the definition. [RQ-804] inherits it." (the owner's draft id
bracketed as filed). Physics review confirms the definition on the pull request that files this entry. The
recommendation of (b) is the audit's. Nothing is chosen here.*

*In plain English: far from the centre of some charts the picture is genuinely flat, and the program must not warn
"precision has run out" there, because that would be untrue. It tells the two apart by whether the chart's slope is
zero. The new exact slopes are tiny but not zero out there, so the warning would fire falsely. There are two ways to
fix it: go back to estimating the slope for this test, or look directly at whether the exact starting systems are
equal before rounding. Equal is not enough on its own: zoomed deep enough (about depth 48 for ordinary numbers), even
the exact starting systems of neighbouring pixels become equal because precision has run out, not because the picture
is flat. So equal counts as flat only where the exact slope is too small to change any number over the standard
step; otherwise it is precision running out, and the warning is shown. Charts with no exact slopes at all have no answer
under that rule yet; a sub-question asks the owner what they do.*

- **File, section:**
  - `plan/requirements.yaml` REQ-GUI-012 (the precision warning), statement: "a collapse between adjacent samples
    along in-plane direction k being one when f64 J_D's column along k is 0 while those two f64 sample points are
    distinct (RQ-553 (a), decided per R-369, per direction; R-570 item 5, W13)".
  - `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound" (:135–137): "`σ` rounds to 1 in f64 past about 37
    and `tanh` to ±1 past about 19, so `J_D`'s central differences (§2 a) are 0".
  - `decisions.md` § R-935, under RQ-758: "The linearised decoder's `J_D` is the tier-1 (exact, dual numbers) or tier-2
    (implicit) slope where a map has one"; "Central differences at R-633's step stay as the test that checks a dual
    slope away from the flagged sites."
  - `plan/tasks/M6/TASK-M6-07.md` (:43): "RQ-553's per-direction saturation test (REQ-GUI-012, TASK-M6-21) reads exact
    zeros of `J_D` and assumes the step is at least the quad's sample spacing".
  - `decisions.md` § R-599 (collapse judged per axis), its *Applied per R-369*: "'saturated', per axis, is RQ-553's
    per-direction test (f64 `J_D`'s column along the axis 0 while the two f64 sample points are distinct; deep_zoom
    §4)".
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision", R-633's step (`h_633` below):
    "along input coordinate `k`, `h_k = ε^{1/3}·max(1, |x_k|)` in f64, `ε` f64's machine epsilon and `x_k` the
    coordinate stepped (`u` or `v` for `J_D`, …)".
  - `decisions.md` § R-570 (no limit on pan or zoom-out), item 5: "**Precision is not a navigation limit.**"
  - `docs/contracts/principia_canonical_spec.md` § 9, invariant 13 (W13): "**Never blank, never lies, never
    freezes**".
- **The conflict:** RQ-553's test was built on central differences, which read exactly 0 in the saturated far field.
  Under R-935 `J_D` is the dual slope; a dual `σ′` written as `e^{−x}/(1+e^{−x})²` stays non-zero out to about 745. So
  in the saturated field the test finds no saturation and the warning says f32 has run out where the cause is
  saturation, against W13. TASK-M6-07's "the step is at least the quad's sample spacing" has no step to read now.
  R-599's per-axis stop reads the same test.
- **The question:** what test tells a saturation collapse from the genuine floor? And a sub-question, Q-T3: on a
  tier-3 map, which has no slope (R-926 item 9; R-935, under RQ-758: "A tier-3 map gets no one-cell patch"), how is a
  pair of bitwise-equal f64 ICs at distinct f64 sample points classified?
- **Options seen:**
  - **(a) A central-difference `J_D` at R-633's step, for this test only.** Preserves RQ-553's test exactly. Extends
    R-935's "central differences only the check" to one more use, which reads the owner's words (both reviewers).
  - **(b) The direct test on the records, the dual slope deciding equal pairs (both reviewers; the slope condition the
    owner's).** Above `ℓ_switch` the CPU holds every sample's f64 IC, and under RQ-797 (a) past it too. For a
    collapsed adjacent pair along in-plane direction `d`:
    - the f64 sample points equal: the genuine floor (REQ-GUI-012's case (ii)); it warns;
    - the f64 ICs differ while the narrowed records are equal: the genuine f32 floor; it warns;
    - the f64 ICs bitwise equal at distinct f64 sample points: a saturation collapse only if
      `|J_dual,k|·h_633 < ½ ulp_f64(x_k)` for every component `k`, where `J_dual,k` is component `k` of the dual (tier
      1) or implicit (tier 2) slope's column along `d`, `h_633` is R-633's step along `d` (`ε^{1/3}·max(1, |u_d|)` in
      f64), and `ulp_f64(x_k)` is f64's ulp at that component's value; no central difference is taken. Otherwise the
      pair is the f64 floor, and it warns. On a tilted view, where `d` moves several chart coordinates at once, `|u_d|`
      is read as the largest magnitude among the chart coordinates that `d` moves (deep zoom §2's `x_k`, "the
      coordinate stepped", taken as each coordinate the step moves, the largest giving the step); this is the entry's
      reading, for the owner with the definition. Little rides on it: the condition's margin is about `2³⁶` on either
      side (a saturated pair's slope underflows to 0; at the f64 floor with a slope of 1, `h_633 ≈ 2^−17.3` against
      `2⁻⁵³`).

    Without the slope condition, equal f64 ICs at distinct points would always read as saturation, and that misreads
    the f64 floor. Past the depth where adjacent f64 ICs become equal, every pair would read "saturated" and the warning
    would switch off where precision is most exhausted, against W13. That depth is about 48 at O(1) `|x₀|`, and R-570
    item 5 lets navigation reach it. The arithmetic, on the basis of RQ-798's fixture rule: at `N = 16` the
    sample spacing is `2h/N = 2^−(ℓ+4)` (`h = 2^−(ℓ+1)`, deep zoom §1), so adjacent ICs differ by about
    `|∂x_k/∂u|·2^−(ℓ+4)`. f64's ulp at `|x₀,k|` in `[1, 2)` is `2⁻⁵²`, and the two are equal at `ℓ = 48` for a slope of
    1.
    From `ℓ = 49` the spacing is half an ulp or less, and adjacent pairs begin to round equal. In general this is
    `ℓ_k ≈ 48 + log₂(|∂x_k/∂u| / |x₀,k|)` (`49 + …` at `N = 8`), the f64 counterpart of the fixtures' f32
    `19 + …`, to within one level (47 at `|x₀,k|` in `[2, 4)`).

    In the saturated far field the condition holds: at REQ-GUI-012's arm (a), `z₀ = 10⁴·d`, the dual `σ′` underflows to
    0. On an unsaturated field at the f64 floor it fails by many orders: with a slope of 1, `h_633 ≈ 2^−17.3` against
    `½ ulp_f64 = 2⁻⁵³`. The condition is the dual-slope form of "a central difference at R-633's step reads 0" (which
    needs `2·|J|·h_633` below about one ulp), so it keeps RQ-553's meaning (*inferred*; the measurement tests it).
    Exact, and costs one comparison per component on pairs already found equal. It is a definition under R-72; the
    physics reviewer confirms it on the pull request that files this entry, as the owner directed. It replaces the
    `J_D` wording in REQ-GUI-012, R-599's applied note and deep_zoom §4.

    Pairs across a flagged fold: a pair placed symmetrically about a fold REQ-DEC-058 flags can have bitwise-equal f64
    ICs by symmetry at any depth. There the dual slope at each sample is about the slope one spacing from the fold, not
    0, so under (b) as written such a pair reads as the f64 floor and warns; RQ-553's central difference read 0 there
    and counted it as saturation. So at flagged sites (b) changes RQ-553's meaning. The entry does not choose between
    leaving it so and excluding such pairs (as REQ-CHART-130 masks seams from refinement); the measurement lists them
    apart (below). Left so, the cost (physics review 5877369526, F4): a pair equal by symmetry is neither saturation
    nor the floor and occurs at any depth, so it raises the precision warning at shallow depths where nothing has run
    out, wherever a flagged fold lies on a sample-grid symmetry line (a mirror axis at `θ = 0`, for example), against
    W13 ("never lies").
  - **(c) The dual `J_D` as is.** Warns falsely in the saturated field; against W13.
- **Q-T3, the tier-3 gap in (b) (physics review 5877078565 of the pull request that filed this entry, P1):** (b)'s slope
  condition reads `J_dual,k`, the dual (tier 1) or implicit (tier 2) slope, and a tier-3 map has none. Tier-3 maps
  reach the f64 floor (about depth 48) like any other, and can also saturate (a solve composed after saturating links,
  for example). Options seen, none chosen:
  - **(i) The floor: such a pair warns.** Never silent at the floor; but in a saturated tier-3 field it raises a false
    warning, saying precision has run out where the field is flat, against W13 ("never lies").
  - **(ii) A central difference at R-633's step, on tier-3 maps only,** as RQ-800 (a) admits one per sample for the
    measure weight. Keeps RQ-553's meaning on tier 3, at the cost of the owner's "no central difference" on those maps,
    which only the owner can extend. It fails in two directions (physics review 5877369526, F3). Under (ii) the
    condition reduces to `|f(u+h) − f(u−h)| < ulp_f64`, which holds only if the two tier-3 solves agree to an f64 ulp.
    Where saturation comes before the solve (a solve composed after saturating links), the solve's inputs at the two
    samples are bitwise equal, so its outputs are too, and (ii) reads saturation correctly. Where the solve's own
    output saturates, the two solves differ by up to the solve's tolerance `τ`, far above an f64 ulp, so (ii) reads the
    saturated pair as the floor and warns falsely, as (i) does, against W13 ("never lies"). The opposite error, the
    floor read as saturation, which would silence the warning at the floor, needs the central difference to read about
    0 where the true slope is about 1; the `τ/h` error (about `τ/2⁻¹⁷`) is far below 1, so it is the less likely case,
    needing another mechanism such as chaos at R-633's step on a chaotic flow map (RQ-800's silence).
  - **(iii) Another rule, stated by the owner.**

  No recommendation on Q-T3: the audit gives none, and physics review gave none. Until Q-T3 is answered, (b) has no
  answer on a tier-3 map, and RQ-804 (a), which inherits (b)'s definition, inherits Q-T3.
- **Recommended:** (b), with its slope condition, the audit's recommendation (the slope condition the owner's). It
  needs no central difference, so it does not stretch R-935's
  words, and it reads what REQ-GUI-012 already names (the f64 sample points and the records) plus the dual slope R-935
  makes `J_D`. It depends on RQ-797 (a) past `ℓ_switch`; under (b) or (c) of RQ-797 the f64 IC of a patched sample
  is still the CPU's `x₀ + J_D·δ` in f64. It answers tier-1 and tier-2 maps only; Q-T3 is the owner's, with no
  recommendation. Measurable alternative, written before any run:
  - *Arms:* REQ-GUI-012's arms (a) to (e), and an arm (f) past the f64 floor: an unsaturated quad at depths 49 to 52,
    `N = 16`, at a centre where every component with a non-zero in-plane slope reaches its f64 level `ℓ_k` (above,
    `ℓ_k ≈ 48 + log₂(|∂x_k/∂u| / |x₀,k|)`) at or before the arm's depth, for example `|x₀,k|` in `[1, 2)` with an
    in-plane dual slope about 1 (components with a zero in-plane slope are unconstrained), and whose f64 sample points
    stay distinct there (`|u|` and `|v|` of the sample points below `2⁻⁴`, so the spacing `2^−(ℓ+4)`
    stays at least f64's ulp at `u` to depth 52). Each fixture states `|x₀|` and the slopes per component before the
    run; arm (f) is exempt from the fixture rule's f32 admission (RQ-798's fixture rule says why) and admitted by
    these preconditions instead. Arm (f) asserts its precondition first: at least one adjacent pair whose f64 ICs are
    bitwise equal at distinct f64 sample points. Like arms (b) to (e), it may feed the trigger synthetic quad facts,
    here built from the CPU's f64 evaluation at those sample points.
  - *Criterion:* (a) and (b) classify every collapsed pair alike on every arm; the pairs where they differ are counted
    and listed. On arm (f), (b) classifies every such pair as the f64 floor and the warning is raised. If none differ,
    (b) keeps RQ-553's meaning on those arms; any pair that differs shows where the meaning changes, and it goes to the
    owner with that pair. Pairs whose two samples lie on either side of a REQ-DEC-058 flagged site are counted and
    listed apart, with how (a) and (b) each classify them; reported, not judged.
  - *Control on arm (f), failing voids the arm:* the rule without the slope condition (equal f64 ICs at distinct
    points are saturation) reads arm (f)'s pairs as saturated and drops the warning.
  - *Tier-3 arm:* an arm on a tier-3 map, saturated and at the f64 floor, is entered void until Q-T3 is answered; its
    criterion is written then, before its run.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M6-21, TASK-M6-08 (R-599's per-axis stop), TASK-M6-07's note.
- **Deferred:** deferred to M6 by owner 13 Nov 2026 (R-1001, the owner: "RQ-802: (b), with my slope condition; pairs placed symmetrically about a flagged fold are excluded, as seams are. Q-T3: defer:M6.").
- **13 Nov 2026 (code review 5886176062 of #515, C4):** the main question is ruled by R-1001 (option (b), the direct test on the records with the owner's slope condition, pairs placed symmetrically about a flagged fold excluded, as seams are); only Q-T3 (how an equal pair is classified on a tier-3 map, which has no exact slope) is left, deferred to M6.

## RQ-804: REQ-GUI-012's last clause, re-derived: under R-926 a switch at `ℓ_switch` with no collapsed pair costs no precision beyond the narrowing, except where RQ-798's creases are, so the warning it raises claims what is not so *(conflict; a one-way door, it amends R-54's words and what the warning claims; R-54, R-90, R-926 item 11, R-935, R-570; REQ-GUI-012; context: TASK-M6-21, RQ-553, the chart audit's F22 (the physics reviewer's "missed" 4))*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F22, the audit's physics reviewer's missed point 4;
each quote re-read on origin/main 54fa96fc. Filed as the owner directed, 12 Nov 2026: "REQ-GUI-012's re-derivation";
RQ-802's definition is inherited here, as the owner directed ("[RQ-804] inherits it"). The recommendation is the
audit's. Nothing is chosen here.*

*In plain English: the program warns about precision whenever it switches to the centre-plus-slopes shortcut at depth
20, even when nothing has been lost. Under the new chart system, with RQ-798's check of corners and edge midpoints (its
(a″)), that switch loses nothing beyond the final rounding every pixel has anyway, so the warning would say something
untrue. The question is whether to stop warning at a switch where no pixels have merged.*

- **File, section:**
  - `decisions.md` § R-54 (the precision warning is event-driven): "Raised on `DECODE_SWITCHOVER` over visible quads
    and on `AT_F32_FLOOR`."; its refinement line: "every other `DECODE_SWITCHOVER`, an `ℓ_switch` switchover with no
    collapsed pair included, and every other `AT_F32_FLOOR` raises it as here".
  - `plan/requirements.yaml` REQ-GUI-012, statement, its last clause: "as does a switchover at ℓ_switch with no
    collapsed pair (R-90)".
  - `docs/design/principia_deep_zoom.md` § 4 (:158): "A switchover at `ℓ_switch` with no collapsed pair is not a
    saturation collapse and warns as R-54 defines it."
  - `docs/contracts/principia_scheduler_contract.md` Part 4 (:137): "which anchors the precision-critical part in f64
    on the CPU and restores the distinction".
  - `docs/contracts/principia_canonical_spec.md` § 9, invariant 13 (W13): "**Never blank, never lies, never
    freezes**".
- **The conflict:** the warning at an `ℓ_switch` switch was derived when the full decoder ran in f32 on the GPU and the
  switch marked its precision running out. Under R-926 the records above the switch are correctly rounded f64
  evaluations, and below it, on quads where each component stays in one binade and does not change sign (RQ-798's binade
  condition, physics review 5877523301, F5), the patch's records are within 5/4 f32 ulp of the exact ones wherever
  RQ-798 (a″)'s check (four corners and four edge midpoints) admits the patch (5/4 being its interpolation constant
  along an edge, physics review 5877369526, F1; one ulp if it compares at 4/5 of its bound, a variant the owner may
  name), and M-K measures it. Under RQ-798 (a′), four corners only, the records are within one f32 ulp only where each
  component's curvature is semidefinite: where it is indefinite the corners can pass while samples near the edge
  midpoints are off by about `½h²|H_uu|` (physics review 5877078565, P2). So, off quads where a component crosses a
  binade or changes sign, a switch at `ℓ_switch` with no collapsed pair loses nothing a user could see, and a warning
  there says f32 has run out where it has not, against W13. A switch triggered by collapse is, on exact records, the
  floor itself (F11), and still warns unless every collapsed pair is a saturation collapse under RQ-802 (b)'s
  definition.
- **The question:** does a switch at `ℓ_switch` with no collapsed pair still raise the precision warning?
- **Options seen:**
  - **(a) No.** The warning is raised at a collapse (on either route) and at `AT_F32_FLOOR`, saturation excepted by
    RQ-802 (b)'s definition: a collapsed pair whose f64 ICs are bitwise equal at distinct f64 sample points is a
    saturation collapse only if `|J_dual,k|·h_633 < ½ ulp_f64(x_k)` for every component `k` (the dual slope along the
    pair's direction, R-633's step, no central difference); otherwise it is the f64 floor and warns, as do the f32
    floor and coincident sample points. A switch at `ℓ_switch` with no collapse raises none. R-54's "Raised on
    `DECODE_SWITCHOVER`" reads "on a `DECODE_SWITCHOVER` triggered by collapse"; REQ-GUI-012's arm (e) is inverted.
    Safe only with RQ-798 (a″), or with (a′) where every component's curvature is semidefinite; otherwise the crease
    strips, and under (a′) the quads of indefinite curvature, lose precision silently. On a tier-3 map, which has no
    slope, (a) inherits RQ-802's sub-question Q-T3 (how an equal pair is classified there: (i) the floor, warning;
    (ii) a central difference at R-633's step on tier-3 maps only; (iii) another, stated by the owner), with no
    recommendation; until it is answered, (a)'s saturation exception has no answer on a tier-3 map.
  - **(b) Keep it as written.** The warning fires at depth 20 on every view, with nothing lost.
  - **(c) Replace it with an informational route mark** (the route debug view, R-926 item 10), not a precision
    warning.
- **Recommended:** (a), conditional on RQ-798 (a′); (b) if (a′) is not taken; the audit's recommendation. Its condition
  is extended by physics review 5877078565 of the pull request that filed this entry, P2, after the audit: (a) holds
  under RQ-798 (a″), and under (a′) only where every component's curvature is semidefinite; (b) otherwise. The audit
  gave its recommendation before that review found that four corners miss indefinite curvature; the choice is the
  owner's. Measurable alternative, written before any run: RQ-798's M-K's and RQ-797's M-S's share of patched samples
  differing from the exact records by more than the admitting check's figure, one f32 ulp of their own value under (a′)
  or 5/4 under (a″); criterion: if that share is 0 on every admitted quad, a switch with no collapse has nothing to warn
  about.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M6-21; RQ-798 and RQ-802.
- **Deferred:** deferred to M6 by owner 13 Nov 2026 (R-1002, the owner: "RQ-804: (a), given RQ-798 (a″). Q-T3 as in RQ-802: defer:M6.").
- **13 Nov 2026 (code review 5886176062 of #515, C4):** the main question is ruled by R-1002 (option (a): no precision warning at a switch at `ℓ_switch` with no collapse); only Q-T3, inherited from RQ-802, is left, deferred to M6. R-1002's safety on quads whose component drops a binade waits on RQ-811.
- **13 Nov 2026 (R-1006, RQ-811 (b)):** ruled by R-1006: a binade-drop component is compared at the f32 ulp of the quad's smallest sample magnitude, so R-1002's safety holds on those quads; only Q-T3 is left, deferred to M6.


## RQ-815: Proof 2 of the WSL distro fails on WSL's own GPU share and its per-VM interop registration, and the two config fixes both change the `wsl.conf` the owner fixed "exactly as I gave it" *(conflict; a one-way door, it amends the owner's ruling's words; R-915, R-912; REQ-VAL-181; context: run 43418528499, `wsl-proofs`; the TASK-M0-63 dispatch)*

*Queued 13 Nov 2026 by the TASK-M0-63 implementer before any code was written; each quote re-read on `main` at
f9031a72f. Nothing is chosen here.*

*In plain English: the Linux box that runs the Windows PC's CPU tests is meant to be sealed off from Windows. The first
real run of its checks found two holes that Windows opens on its own: the graphics-driver folder on C: is shared in, and
a hook that lets Linux start Windows programs is registered at boot even though interop is off. Each can be closed by
adding a line to the box's settings file, `wsl.conf`, but the owner ruled that file must stay exactly as they wrote it,
and one of the fixes adds a `[boot]` section, which that ruling's text singles out.*

- **File, section:**
  - `decisions.md` § R-915, the owner's words: "WSL stays as set: no sudo, no systemd, and wsl.conf exactly as I gave
    it. Don't add [boot] systemd=true or any sudoers entry, not even a narrow one. A sudo rule for systemd-run amounts
    to full root, and root can mount C: and D:."
  - `decisions.md` § R-912, requirement (2): "automount off (/etc/wsl.conf [automount] enabled=false) and no Windows
    interop; C: and D: not reachable from inside, shown by test".
  - `plan/OPERATIONS.md` § "The Windows runner's work": "`/etc/wsl.conf` exactly as the owner gave it, automount and
    interop off".
  - `scripts/wsl/inside.sh` `isolation()` (proof 2), which fails on "a mount backed by Windows: drvfs, or the 9p and
    virtiofs WSL serves Windows folders over (/usr/lib/wsl/drivers, with GPU support, is the driver store on C:)" and
    on `/proc/sys/fs/binfmt_misc/WSLInterop` being present; `xtask/tests/wsl.rs` `check_image` holds
    `scripts/wsl/wsl.conf` equal to `OWNERS_CONF` and the build free of the word `[boot]`.
- **What run 43418528499 found:** (a) `/usr/lib/wsl/drivers` is 9p-mounted: WSL's GPU share, on by default; the
  owner's `wsl.conf` has no `[gpu]` section. (b) `/proc/sys/fs/binfmt_misc/WSLInterop` is present: WSL2 registers it per
  VM at boot whatever `[interop] enabled` says, and only root can remove it.
- **The conflict:** the dispatched fix adds `[gpu] enabled=false` and a `[boot] command=` that writes `-1` to
  `/proc/sys/fs/binfmt_misc/WSLInterop` (WSL runs it as root at the distro's start, without systemd). Both tighten
  isolation, and neither adds systemd or sudo, but both change the file the owner fixed "exactly as I gave it", and the
  second adds a `[boot]` section and a root command at boot, the area the ruling's own example names. Reading
  "exactly" as "no looser" would reinterpret the owner's words.
- **The question:** may `wsl.conf` gain `[gpu] enabled=false` and a `[boot] command=` removing the `WSLInterop`
  registration, or is proof 2 to be met another way?
- **Options seen:**
  - **(a) Amend R-915's "exactly":** allow both additions (no `systemd=`, no sudo), the image test's exact text and its
    `[boot]` ban updated to the new file and a ban on `systemd=` kept; the manifest re-pinned and the distro
    re-registered with `replace: true`.
  - **(b) `[gpu]` only:** allow `[gpu] enabled=false` (no `[boot]`), and close (b) PC-side or not at all; the
    `WSLInterop` check then keeps failing, so proof 2 cannot pass under (b) alone.
  - **(c) Keep `wsl.conf` as is:** close (a) on the PC (a `.wslconfig` GPU setting, if WSL's version there offers one,
    sent as a PC message) and rule how R-912's "no Windows interop, shown by test" reads with a registration only root
    can remove, for example that no Windows program can be started as `ci` (shown by trying), in place of the entry's
    absence. That redefines the test, which is also the owner's call.
- **Recommended:** (a). Both lines remove a path to Windows that the owner's file was written to close, add no
  privilege `ci` can use, and are checked byte for byte by proof 2's own `cmp` and by the manifest pin. Measurable
  alternative, criterion written before any run (R-649): proof 2 of `wsl-proofs` after a `replace: true` registration
  passes every line of `isolation()` unchanged, with `/usr/lib/wsl/drivers` unmounted and `WSLInterop` absent.
- **Applied meanwhile:** nothing; `scripts/wsl/wsl.conf` and its tests are unchanged.
- **Waits:** the proof-2 half of TASK-M0-63. Its proof-6 fixes (the stdin read, the byte-order mark, lavapipe in the
  hosted `wsl-parity` job, the failure line) and proof 3's evidence count touch no `wsl.conf` and need no ruling.
