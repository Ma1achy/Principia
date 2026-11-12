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

## RQ-797: Where the one-cell patch `x₀ + J_D·δ` is expanded into the integrator's records: R-926 item 5 and REQ-INT-116 keep chart arithmetic off the GPU, while REQ-SCHED-024, REQ-DEC-036, REQ-DEC-052, TASK-M5-04 and TASK-M6-07 have the GPU form it; and R-935's tier-3 words read as a centre-plus-difference record *(conflict; a one-way door, it sets CPU and GPU bits and reads the owner's words in R-926 item 5 and R-935; R-926 items 5 and 11, R-935, R-90, R-632, R-686, R-142; REQ-INT-115, REQ-INT-116, REQ-SCHED-024, REQ-DEC-035, REQ-DEC-036, REQ-DEC-051, REQ-DEC-052, REQ-TOOL-158, REQ-PAY-069; context: TASK-M2-87, TASK-M4-05, TASK-M5-04, TASK-M6-07, TASK-M6-08, the chart audit's F1)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F1, with the audit's physics and code reviewers'
corrections; each quote re-read on origin/main 54fa96fc. Q1 (a) and Q2 (i) are recommended by the owner, 12 Nov 2026:
"patch location (recommend CPU f64)", and "[RQ-797] Q2: recommend (i). The owner doesn't recall the intent of R-935's
words; (i) keeps the GPU free of chart arithmetic, which is R-926's purpose. Note that forming the sample's chart input
quad-locally on the CPU (centre + offset, in f64) is compatible with (i)." (the owner's draft id bracketed as filed).
Nothing is chosen here.*

*In plain English: past zoom depth 20, each square's starting systems come from one centre point plus its slopes, a
small sum per pixel. The owner ruled that the graphics card only runs the simulation and does no chart arithmetic, but
older requirements and tasks still have the graphics card doing that sum. Where the sum is done decides whether the
pictures come out the same bit for bit on every machine, and how much data goes to the graphics card. A second, smaller
question: the owner's words for charts with no exact slopes say each pixel's record carries "its difference from the
quad's centre", which reads as the same sum done on the graphics card. The owner doesn't recall what those words were
meant to say; the recommendation reads them so that the graphics card still does no chart arithmetic: the CPU works out
each such pixel's own starting system, and may use the centre plus the pixel's offset as its input.*

- **File, section:**
  - `decisions.md` § R-926 (the new chart system; the CPU evaluates charts, the GPU only integrates), the owner's
    words, item 5: "The CPU evaluates charts in f64. The GPU only integrates records. No chart code in the
    integrator."; item 11: "A small quad's table collapses to one cell — that IS the linearised decoder (centre +
    slopes). Grids for large quads, one-cell patches for small ones."
  - `decisions.md` § R-935 (the re-plan's eleven answers), under RQ-758 (slopes): "A tier-3 map gets no one-cell
    patch: every sample is evaluated on the CPU, its record carrying its difference from the quad's centre."
  - `plan/requirements.yaml` REQ-INT-116 (the integrator entry never decodes), verify: "the integrator entry's
    source, and everything it calls, contains no chart map, no link and no decode call".
  - `plan/requirements.yaml` REQ-SCHED-024 (deep zoom's precision split), statement: "give the GPU only quad-local
    relative arithmetic x = x₀ + J_D·δ in f32, passed as per-quad uniforms"; verify: "kernel computes only x₀ +
    J_D·δ".
  - `plan/requirements.yaml` REQ-DEC-036 (`x₀` and `J_D` on the CPU), statement: "the GPU must evaluate x = x₀ +
    J_D·δ without running the nonlinear decode pipeline".
  - `plan/requirements.yaml` REQ-DEC-051 (the f32 floor calibration), statement: "x = x₀ + J_D·δ formed in f32".
  - `plan/requirements.yaml` REQ-DEC-052 (the f32 floor gate), verify, its control: "a measurement that reads the f64
    sum x₀ + J_D·δ in place of the f32 IC the GPU integrates misses it and fails".
  - `plan/tasks/M6/TASK-M6-07.md` (the linearised decoder), Goal (:12): "the GPU evaluates `x = x₀ + J_D·δ` in f32
    and never runs the nonlinear pipeline"; Deliverables (:30): "`crates/kernel/src/decode/linear.rs`: the f32 linear
    path `x₀ + J_D·δ` in the shared source".
  - `plan/tasks/M5/TASK-M5-04.md` (the quad request), Goal (:15–16): "the kernel's only use of them is `x₀ + J_D·δ`
    in f32".
  - `docs/contracts/principia_lowering_contract.md` § "Part 1 — What lowering is" (:20): "the CPU evaluates them in
    f64 and sends each quad its forward table, from which the integrator's records are formed". It does not say
    where they are formed.
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision" (:76): "A few fused multiply-adds
    (continuous — the decode is not a branch input, so FMA latitude here is honest divergence, not a determinism
    concern)"; (:87): "But `x = x₀ + J_D·δ` is an absolute f32 number of order `|x₀|`, and the f32 integrator on the
    GPU starts from it".
  - `docs/contracts/principia_scheduler_contract.md` Part 4 (:137), the dated note: "bitwise switchover on CI
    backends stays blocking (R-686, the owner's acceptance)."
  - `decisions.md` § R-142 (only the verdict returns from the GPU): "`QuadReduction` stays the sole automatic
    return."
- **The conflict:** R-926 item 5 and REQ-INT-116 leave the GPU no chart arithmetic, and the lowering contract forms
  records from the forward table without saying where. REQ-SCHED-024, REQ-DEC-036, REQ-DEC-051, REQ-DEC-052,
  TASK-M5-04 and TASK-M6-07 have the GPU form `x₀ + J_D·δ` in f32, TASK-M6-07 in a decode module. If the sum is
  called from the integrator entry, TASK-M6-07 fails TASK-M4-05's lint (REQ-INT-116); if it is a separate GPU pass,
  no task builds it. A GPU sum follows each backend's FMA contraction (deep zoom §2:76), and R-90's switch and
  `AT_F32_FLOOR` branch on the bitwise equality of its results (scheduler Part 4:137–138), which is why R-686 tests it
  on NVIDIA. Detecting collapse from GPU-formed records needs a readback, against R-142. Separately, R-935's tier-3
  wording ("its record carrying its difference from the quad's centre") reads as a centre-plus-difference record,
  which needs a GPU-side add on tier-3 quads too; a difference carried in f32 and summed in f32 before an absolute f32
  integrator buys no depth (deep zoom §2:87).
- **The question:** Q1, does the CPU or the GPU expand the patch into records? Q2, what does a tier-3 record carry?
- **Options seen:**
  - **Q1 (a) The CPU expands in f64 and narrows once.** The GPU reads only records, so R-926 item 5 and REQ-INT-116
    hold literally. Each record is the correctly rounded value of its f64 sum (TASK-M2-86's predicate (3), within half
    an f32 ulp), the same on every backend once RQ-799 (a) pins the maths library, with no quad seam beyond the
    `O(h²)` linearisation (F10). Collapse and floor detection run on the CPU, with no readback (R-142). R-686's NVIDIA
    run then measures the integrator only; its words stand. Costs: about 30 FMAs per patched sample, in place of the
    exact evaluation (about 85 ns, composition §17.12:2496) it replaces, so the CPU cost per patched sample falls; and
    `N²` records uploaded per patched quad, as for an exact quad, where (b) uploads one `x₀` and `J_D` (about 45 floats,
    scheduler Part 5:171). Rewritten from the answer: REQ-SCHED-024, REQ-DEC-035, REQ-DEC-036, REQ-DEC-051,
    REQ-DEC-052's control (the f64 sum, narrowed, becomes the measured path), REQ-TOOL-158, REQ-PAY-069, REQ-DEC-033
    ("on the full nonlinear decoder"), scheduler Part 4:138's definition of `AT_F32_FLOOR`, deep zoom §2:75–76,
    TASK-M5-04, TASK-M6-07 and TASK-M6-08. A patched sample's descriptor, tag and branch words must come from
    somewhere: under RQ-798 (a′) a quad whose samples could differ from the centre's in tag, branch or sheet is
    exact per sample, so the centre's words are right on every patched quad; TASK-M2-87's contract states it.
  - **Q1 (b) A separate GPU pass expands into a record buffer before the integrator entry.** REQ-INT-116 holds and the
    upload stays one `x₀` and `J_D` per quad. Records follow each backend's FMA contraction, so they are not bitwise
    across backends, and R-686 applies to them. Floor detection needs a readback, against R-142. A new task is needed.
  - **Q1 (c) The integrator entry calls the patch,** as TASK-M6-07 and TASK-M6-08 have it today. R-926 item 5 and
    REQ-INT-116 would need amending, and (b)'s bit and readback issues remain.
  - **Q2 (i) A tier-3 record holds each sample's own f64 evaluation, narrowed once,** the same as an exact record; "its
    difference from the quad's centre" is read as what the CPU knows and may show (the table debug view, R-926 item
    10), not what the record carries. Consistent with Q1 (a). The sample's chart input may be formed quad-locally on
    the CPU, the quad's centre plus the sample's offset, in f64, before the evaluation; that is compatible with (i), the
    difference from the centre then entering the CPU's evaluation, not the record or the GPU. A reading of the owner's
    words, so the owner's.
  - **Q2 (ii) A tier-3 record carries centre plus difference, summed on the GPU.** The words read literally. Needs Q1
    (b) or (c) for tier-3 quads, with their bit issues, and buys no depth (deep zoom §2:87).
- **Recommended:** Q1 (a), recommended by the owner, 12 Nov 2026, and Q2 (i), also recommended by the owner, 12 Nov 2026
  (their words in this entry's header). On Q2: the owner doesn't recall what R-935's words "its record carrying its
  difference from the quad's centre" were meant to say, and (i) keeps the GPU free of chart arithmetic, which is the
  purpose of R-926 (item 5: "The GPU only integrates records. No chart code in the integrator."). Forming the sample's
  chart input quad-locally on the CPU (centre plus offset, in f64) is compatible with (i). Q2 still reads the owner's
  words, so it is the owner's to rule. Measurable alternative for Q1, written before any run: bytes uploaded per patched
  quad under (a) and (b), reported as their ratio (expected about `N²·136 B / 180 B`, about 190 at `N = 16`, *inferred*)
  and as a share of the tier's hard cap on bytes (REQ-PERF-040); and M-S, the audit's seam measurement, run under (a)
  and (b): patch records against exact records on both sides of 1,000 quad edges, on the latent and `(L_z, E)` charts at
  depths 21 to 25, at fixtures as RQ-798's fixture rule states them, the seam difference per component reported as a
  ratio to the in-quad adjacent difference at the same depth, and the outcome-flip rate across seams against the in-quad
  rate and the one-ulp chaos baseline (R-84); positive control, one quad's `x₀` offset by two ulps must show as a seam,
  or the run is void.
- **Applied meanwhile:** nothing. The superseded lines are marked *Was* in the chart audit's record-corrections PR (its
  F7 and F9), without choosing an option.
- **Waits:** TASK-M2-87 (the record contract), TASK-M4-05 (flat-grid compute), TASK-M5-04, TASK-M6-07, TASK-M6-08; the
  parts of the chart audit's record corrections F7, F9 and F20 that depend on this answer.

## RQ-798: A patched quad that straddles a crease: R-90 patches every quad past depth 20, "whichever comes first", and nothing consumes REQ-DEC-058's kink flag, so tags, branches and sheets come out as the centre's, and continuous values are wrong across a strip hundreds of quads wide near a square-root edge *(silence; a one-way door, it amends R-90's "whichever comes first" with a named exception, and changes CPU bits; R-90, R-935, R-937, R-926 item 11; REQ-DEC-058, REQ-INT-115, REQ-CHART-129, REQ-CHART-130; context: TASK-M2-87, TASK-M2-88, TASK-M6-08, the chart audit's F2)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F2, with the audit's physics reviewer's option (a′)
and its code reviewer's additions; each quote re-read on origin/main 54fa96fc. (a′) is recommended by the owner, 12 Nov
2026: "creases (recommend a′)". Nothing is chosen here.*

*In plain English: deep down, each square gets the centre-plus-slopes shortcut. Where a square straddles a crease in
the chart, such as the edge of the allowed region, a fold, a pole or a switch between formulas, the shortcut gives the
wrong kind of system on part of the square. Near an edge shaped like a square root it is also wrong in value, not just
in kind, over a band hundreds of squares wide. The proposed fix checks each square's four corners against the exact
answer before trusting the shortcut. Four corners can miss a saddle-shaped error, which cancels at every corner; a
variant also checks the midpoints of the square's four edges, which catches it, at twice the cost.*

- **File, section:**
  - `decisions.md` § R-90 (the decoder switchover trigger): "Switch to the linearised decoder when the full decoder's
    adjacent samples give bitwise-identical ICs, with ℓ_switch = 20 as an upper bound (whichever comes first)."
  - `decisions.md` § R-935, under RQ-766 (a large quad's forward table): "One node per sample above R-90's switch,
    exact; the one-cell patch, a centre and slopes, below it".
  - `plan/requirements.yaml` REQ-DEC-058 (each map declares its slope tier), statement: "a tier-1 or tier-2 map must
    set a flag at each kink site (the feasibility boundary, `k = 0`, `warped_k` at `d = 0`, `normalise`'s edges, the
    exponential map's `r = 0`, the poles, the seed switches)".
  - `plan/requirements.yaml` REQ-INT-115 (the record contract): "the record as REQ-DEC-005's `(m, r, p)`, its
    descriptor and tag words, a branch word and the stop index".
  - `plan/requirements.yaml` REQ-CHART-129 (seams flagged per footprint): "a footprint whose copies straddle a
    declared seam, a branch or a composed chart's mask must be flagged per pixel, in the resolve pass"; REQ-CHART-130
    (seams masked from refinement): "Declared seams must be masked from the refinement criterion".
  - `decisions.md` § R-937 (the invariant charts' second sheet): "the spin edges unfold onto the time-reversed sheet".
  - `plan/tasks/M2/TASK-M2-86.md` (the first slice), Acceptance (:28): "(3) each narrowed component is the correctly
    rounded value of its f64 evaluation (within half an f32 ulp of it)"; and "per component in f32 ulps at `max(1,
    |x|)`".
- **The silence:** REQ-DEC-058 requires the kink flags; nothing consumes them (in the tasks only TASK-M2-86:25 and
  TASK-M2-88 mention kinks). R-90 patches every quad past 20. A one-cell patch gives every sample the centre's tag,
  branch and sheet, so along each kink curve, at every depth past 20, `ENERGY_INFEASIBLE` and `INVARIANT_INFEASIBLE`
  come out wrong at the `(L_z, E)` feasibility edge, the patch keeps sheet 1 across R-937's unfold line, and branch
  selects come out wrong at the poles. The continuous error is wider than the tags. Near a `√` edge, `f = √d`; a patch
  of half-width `h` centred at distance `d` has a relative linearisation error of about `(h/d)²/8`. Holding that under
  half an f32 ulp of the component's own value (predicate (3)'s bound, `2⁻²⁴` relative) needs `(h/d)² ≤ 2⁻²¹`, so
  `d ≳ 2^10.5·h ≈ 1,450·h`: about `2¹¹` half-widths (about 720 quad widths) each side of the edge (the physics
  reviewer's arithmetic, *inferred*). Every figure here is in the component's own ulp, a relative unit, and only `h/d`
  enters, so the strip is scale-invariant: the same number of half-widths at every depth. The poles behave alike
  (curvature about `1/r²`).
- **The question:** when does a quad past R-90's switch take the patch, and when is it exact per sample?
- **Options seen:**
  - **(a) Footprint and centre test.** A quad whose footprint meets a flagged site, or whose corner and centre
    evaluations differ in tag, branch or sheet, is exact per sample at any depth. Fixes the tags where a flagged site
    lies in the footprint; a tag sliver between sample points can still be missed by the corner test, which the
    footprint test carries (the code reviewer). Leaves the wide strip of continuous values wrong by more than the
    narrowing. R-90 gains a named exception.
  - **(a′) The four-corner check plus the footprint test (the physics reviewer's).** A quad takes the patch only if
    the exact f64 evaluation at each of its four corners and the patch's f64 prediction there differ by at most half
    an f32 ulp of the component's own value in every component (predicate (3)'s bound), agree in tag, branch and
    sheet, and its footprint meets no flagged site (REQ-DEC-058's flags, consumed by extending REQ-CHART-129's
    per-footprint flag and REQ-CHART-130's `Seam` mask). Otherwise it is exact per sample. Cost: 4 evaluations against
    `N² = 256` at `N = 16`, 1/64 of an exact quad. Covers kinks, folds and sheets, and the high-curvature strip where
    the curvature has one sign (semidefinite, as at the `√` edge above), subject to the binade condition stated under
    (a″) below; keeps R-926 item 11's "one-cell patches for
    small ones" wherever the patch is right. R-90 gains a named exception, and the record's bits change on the quads
    it sends back to exact. **The gap (physics review 5877078565 of the pull request that filed this entry, P2):** four
    corners do not bound the patch's error where a component's curvature is indefinite. The error is `½·δᵀHδ + O(δ³)`,
    at the corners `(±h, ±h)` it is `½h²(H_uu + H_vv ± 2H_uv)`, so a component with `H_uu = −H_vv` and `H_uv` small
    (any harmonic component) passes every corner while samples near the edge midpoints are off by about `½h²|H_uu|`.
    For example, on a log-polar pair `(s, θ)`, `x = e^s·cos θ` has `H_ss = x` and `H_θθ = −x`, and at `θ = 0`
    `H_sθ = 0`: every corner passes while the samples along the `s` and `θ` axes are off by `½h²·x`. Under (a′) such
    quads take the patch with that error. Its scale (physics review 5877369526, F2): the miss is `½h²·|H_uu|/|x|`
    relative to the component's own value. On the log-polar example that is `½h²`, about `2⁻⁴⁵` at depth 21
    (`h = 2⁻²²`), roughly `2⁻²¹` of an f32 ulp, harmless at every depth from 21 to 30. The gap bites only where
    `½h²·|H_uu|/|x|` reaches an f32 ulp of the component's own value (`2⁻²⁴` relative), where the curvature is large
    against the value: near a point where a component and its gradient both vanish with saddle-shaped curvature
    (`x ≈ u² − v²`, whose miss `h²` against `|x| ≈ r²` reaches `2⁻²⁴` within about `r ≲ 2¹²·h`, some `2¹²`
    half-widths of the point), or in an indefinite strip beside a singular site, as the `√` strip is beside its edge
    (*inferred*). It also bites along a component's zero lines where the gradient is not zero (physics review
    5877523301, F6): where the curvature in the grid axes is nearly trace-free with a small mixed term, for example
    `x = g·u + c(u² − v²)`, the corners miss and `|x|` falls to 0 along the line, so the gap opens on strips, not only
    at points, about `2²⁴·½h²|H_uu|/|∇x|` wide, some `2·|H_uu|/|∇x|` half-widths at depth 21, narrowing with depth
    (*inferred*); the binade condition under (a″) applies to these quads too. The log-polar position components never
    bite: `e^s·cos θ` and `e^s·sin θ` are harmonic, at their zero lines `H_ss = H_θθ = 0` and the curvature is purely
    mixed, which the corners catch, and at their extrema the miss is about `½h²` relative (physics review 5877523301).
  - **(a″) The four corners and the four edge midpoints, plus the footprint test (physics review 5877078565, P2).** As
    (a′), with the exact f64 evaluation and the patch's f64 prediction compared at the four edge midpoints as well as
    the four corners, under the same bound, tag, branch and sheet agreement and footprint test. The midpoints give
    `H_uu` and `H_vv` and the corners `H_uv`, so the quadratic error over the square is bounded, for example by
    `½h²(|H_uu| + 2|H_uv| + |H_vv|)`. Admission bounds the square at 5/4 of the bound checked at the eight points, not
    at the bound itself (physics review 5877369526, F1): along each edge the error is a quadratic checked at three
    points, the two corners and the midpoint, and the interpolation constant on nodes `{−1, 0, 1}` is `5/4`, reached at
    `t = ½`. For example, in units of the bound, `½h²H_uu = 1`, `h²H_uv = 1` and `½h²H_vv = −1` pass all eight points,
    while on the edge `u = h` the error is `1 + s − s²` (`s = v/h`), `5/4` at `s = ½`. So where the patch is admitted
    its records are within 5/4 f32 ulp of the exact ones, where (a′) on semidefinite curvature gives one (a convex error
    peaks at a vertex); comparing at 4/5 of the bound would restore one ulp, a variant the owner may name. **The binade
    condition (physics review 5877523301, F5), for (a′) and (a″) alike:** both bounds are in the own-value ulp of the
    checked points, which varies across a quad, so they hold per sample only while each component's samples stay in the
    checked points' binade; the semidefinite bound is not exact per sample, as an earlier review read it, but holds only
    under that uniform tolerance. Where a component's magnitude drops a binade inside the quad, interior samples get up
    to 2 (semidefinite, (a′)) or 5/2 ((a″)) of their own ulp. Where a component changes sign inside the quad, its own
    ulp at the samples nearest the zero is arbitrarily small, and the error against it is unbounded whatever the
    curvature's sign: for `x = g·u + ½H·u²` on a quad straddling `u = 0` at depth 30 (`h = 2⁻³¹`), the corners sit at
    `|x| ≈ |g|h` and pass whenever `h|H|/|g| < 2⁻²⁴`, while a sample at `|x| ≪ |g|h` still carries about `½H·δu²`
    against `2⁻²⁴·|x|`. Variants the owner may name: comparing at the quad's smallest own-value ulp, estimated from the
    patch as `|x₀| − |J|·√2·h`; and, where that estimate is at or below 0, so the component may cross zero, exact
    evaluation per sample or an absolute floor. Cost: 8 evaluations against `N² = 256` at `N = 16`, 1/32 of an exact
    quad. Covers kinks, folds, sheets and the high-curvature strip of either sign. R-90 gains a named exception, and the
    record's bits change on the quads it sends back to exact.
  - **(b) Keep the patch, and set each sample's tag, branch and sheet by a cheap exact predicate per sample.**
    Continuous values stay linearised, wrong across the strip.
  - **(c) Keep the patch and label the strip.** Wrong classifications are drawn and only labelled.
- **Recommended:** (a′), recommended by the owner, 12 Nov 2026 (their words in this entry's header). The owner
  recommended (a′) before physics review 5877078565 found that four corners miss indefinite curvature; (a″) closes it.
  The choice between (a′) and (a″) is the owner's. Measurable alternative, measurement M-K, criterion written before
  any run:
  - *Fixture rule (this entry's and RQ-797's, RQ-801's, RQ-802's and RQ-805's):* a fixture past R-90's switch states,
    before its run, `|x₀,k|` and the in-plane slopes `|∂x_k/∂u|`, `|∂x_k/∂v|` from the f64 evaluation for each of the
    record's 15 components, and for each the deepest level at which it still separates adjacent samples,
    `ℓ_k ≈ 19 + log₂(|∂x_k/∂u| / |x₀,k|)` at `N = 16` (`20 + …` at `N = 8`), to within one level (deep zoom § 1's
    `h = 2^−(ℓ+1)` and sample spacing `2h/N` against f32's ulp at `|x₀,k|`, *inferred*). A depth is admitted when,
    along each in-plane axis, some component varying along it has `ℓ_k` at or past that depth, R-632's rule as
    REQ-TOOL-159's note has it ("use coordinates where the measured f32 floor (REQ-DEC-051, quoted with |x₀|) admits
    depth 30"); an arm at a depth no component admits is reported void, never passed.
  - *Setup:* the `(L_z, E)` chart at depths 21 to 30, `N = 16`, the CPU expansion of RQ-797 (a). Straddling set: 512
    quads whose footprint meets the feasibility edge. Away set: 512 quads at distance at least `2¹¹·h` from it, the
    bound above. Indefinite set: 512 quads at depths 21 to 30 centred within about `2¹²·h` of a point where a component
    and its gradient both vanish with saddle-shaped curvature, or on a strip along a component's zero line where the
    grid-axis curvature is nearly trace-free with a small mixed term (the gap's strips above), the fixture stating
    before its run, under the fixture rule, that `½h²·|H_uu|` reaches an f32 ulp of that component's own value there; it
    is needed because the feasibility edge's curvature is semidefinite and the two sets above alone would pass a
    corners-only check that is wrong elsewhere. The log-polar example would not serve: its miss is about `2⁻²¹` of an
    f32 ulp, so it reports no difference under (a′) or (a″). Whether any chart in the corpus has such a point or strip
    past depth 20 is not established here; searching for both is M-K's first step, and if none exists the run says so,
    the indefinite set is void, and (a′)'s gap is exercised by control (iii) alone, which the owner should know when
    weighing (a′) against (a″). The set's quads that cross a component's zero are reported apart, as PASS below says,
    in the quad's smallest own-value ulp. The fixture follows the fixture rule above. Patch records and exact narrowed
    records compared sample by sample, as are the held quantities `E` and `L_z` of the patch records against `E*` and
    `L_z*`.
  - *Ulp convention:* the component's own f32 ulp, because near the edge `|p| ∝ √d` is small and `max(1, |x|)` would
    hide its error. The strip bound, PASS and the controls are all in this one unit; each difference is also reported,
    not judged, at `max(1, |x|)`, TASK-M2-86:28's unit, for comparison only.
  - *PASS under (a′) or (a″):* on every quad the check admits on which each component's samples stay in the checked
    points' binade, no sample differs in tag, branch or sheet, and no component differs by more than one f32 ulp of its
    own value (half an ulp each side, predicate (3)) under (a′), or 5/4 f32 ulp under (a″), (a″)'s interpolation
    constant above. Admitted quads on which a component crosses a binade or changes sign are reported apart, each with
    its worst difference in the quad's smallest own-value ulp, and are not judged against a bound the check cannot give
    them (the binade condition above). The share of
    straddling and of away quads the check sends to exact is reported.
  - *Positive controls, any failing voids the run:* (i) one sample's tag flipped on purpose in an admitted quad is
    detected; (ii) one quad's `x₀` offset by two f32 ulps of the component's own value is rejected by the check;
    (iii) a planted saddle-shaped second-order error `κ(δu² − δv²)` in one quad's patch, above the bound at the edge
    midpoints and 0 at the corners, is rejected by (a″)'s check. (ii) is first-order and cannot exercise this. Under
    (a′), (iii) is admitted by construction: that arm reports it as (a′)'s gap, and its rejection is asserted only
    under (a″).
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M2-87 (the record contract), TASK-M2-88's flag consumer, TASK-M6-08.

## RQ-799: Pin every transcendental reachable from the CPU chart evaluator: on the host, `Float::sin`, `Float::atan2` and the rest call std and so the platform's maths library, so a record is bitwise only per machine *(conflict; a one-way door, CPU bits; R-926 item 5, R-502, R-521, R-921, R-197, R-931; REQ-CHART-099, REQ-CHART-155, REQ-TOOL-172, REQ-INT-091; context: TASK-M2-67, TASK-M2-86, TASK-M2-89, the chart audit's F3)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F3, with the audit's code reviewer's list of
functions and its physics reviewer's measurement; each quote re-read on origin/main 54fa96fc. (a) is recommended by the
owner, 12 Nov 2026: "the libm pin (recommend a)". Nothing is chosen here.*

*In plain English: the CPU now works out every pixel's starting system. It uses the computer's own maths library,
which differs very slightly between Mac, Linux, Windows and the browser, so a saved picture could re-run one bit
differently on another machine. The project already fixed this in three places by using a pinned software maths
library; this would do the same for every maths function the chart code can reach, with a test so none slips back.*

- **File, section:**
  - `crates/kernel/src/real.rs` (:49–50), the doc comment on `arctan2`: "libm's by default, so the f64 path is libm's
    bit for bit"; the default body (:59): `Float::atan2(self, x)`; `sinc`'s default (:71): `Float::sin(self) / self`.
  - spirv-std 0.10.0-alpha.1's `Cargo.toml` (the cargo registry, lines 61–63):
    `[target.'cfg(not(target_arch = "spirv"))'.dependencies.num-traits]`, `default-features = true`. In num-traits
    0.2.19 (`src/float.rs:835`, `:847`) the `std` implementation is selected when `std` is on, and the `libm` path only
    under `all(not(std), libm)`.
  - `crates/kernel/Cargo.toml` (:16–21): "computes its `pow` in f64 through a pinned pure-software libm … with
    `force-soft-floats`, so no platform libm, compiler intrinsic or architecture-specific instruction computes it, and
    the native and wasm hosts give the same bits"; `libm = { version = "=0.2.16", default-features = false, features =
    ["force-soft-floats"] }`. The same pin is at `crates/engine/Cargo.toml:27` and `crates/ledger/Cargo.toml:11`, and
    `crates/engine/src/trajectory.rs:277` and `:376–393` call `libm::sin` and `libm::cos`.
  - Chart code calling the platform library on the host: `Float::powf` at `chart/invariant.rs:187`,
    `construction.rs:946`, `physical.rs:739`, `burrau.rs:557`; `Float::asin` at `shape_sphere.rs:671`; `Float::ln` at
    `burrau.rs:638`; inherent `f64::ln` at `physical.rs:922–923` and `:962`; `Float::sin`, `cos`, `sin_cos` and
    `atan2` throughout `shape_sphere.rs`, `tilt.rs`, `burrau.rs` and `burrau_acute.rs`; `Float::hypot` at
    `chart/tilt.rs:239`, `:288`, `:294`, `:342` and `:438`; `Float::ln_1p` and `Float::exp_m1` in the generated links'
    `softplus` and `inv_softplus` (`generated/links.rs:219`, `:225`); `Float::cbrt` for the encode's step
    (`encode/ingest.rs:116`, `:152`); and `Float::cosh`, which the link generator emits for `sech2`
    (`crates/ledger/src/gen/links.rs:671`).
  - `decisions.md` § R-502 (the `N_sub` table, pinned libm): "A pinned pure-software libm computes the `pow` (in
    f64, rounded once to f32), so the native and the wasm hosts give identical bits".
  - `decisions.md` § R-921 (records under a retired link recreate bitwise): "to the original physical starting
    state, bitwise, and runs from there".
  - `plan/requirements.yaml` REQ-CHART-099 (charts reopen bitwise): "Every existing chart, gallery preset and saved
    chart must reopen bitwise under the composition"; REQ-CHART-155 (the browser CPU route): "whether a wasm f64 solve
    matches the native one bit for bit is measured and reported".
  - `docs/design/principia_ic_save.md` (:82–84): "a residual arises only from another platform's maths library
    (`atan2` in `C`'s step 2, the chart inverses) or another build".
  - `decisions.md` § R-197 (who may configure a lint): changing lint configuration (`clippy.toml`, `[lints]` tables)
    needs a ruling.
- **The conflict:** under R-926 item 5 every record is the CPU's f64 evaluation, narrowed. On the host every `Float::`
  transcendental, and every inherent `f64` one, goes through std to the platform library (macOS, glibc, Windows, wasm).
  Where two platforms' f64 values differ by one f64 ulp, narrowing flips at an f32 rounding midpoint with probability
  about `2⁻²⁹` per differing component (one f64 ulp in the `2²⁹` f64 ulps of an f32 ulp); the overall rate is (the share
  of calls that differ by platform) × (the amplification through the chart) × that, *inferred*. Rare, but not zero, so
  R-521's and R-921's recreation, REQ-CHART-099's digest written on the Mac and checked on Linux CI, REQ-CHART-155's
  native-to-wasm comparison and the export contract's re-render (:156) hold only per machine. The lock and placement
  solves, the backwards search and R-952's contours call the evaluator, so they inherit it too (iteration counts and
  branches can change). ic_save :82–84 already concedes the residual for chart inverses; R-926 extends it to every
  pixel. The comment at `real.rs:49–50` reads as if the pinned crate were used, when the default goes through std.
- **The question:** does every transcendental reachable from the CPU evaluator go through the pinned libm, enforced?
- **Options seen:**
  - **(a) Pin, enforced.** Every f64 transcendental reachable from the CPU evaluator (sin, cos, sin_cos, tan, atan2,
    asin, acos, exp, exp_m1, ln, ln_1p, powf, hypot, cbrt, sinh, cosh, tanh and any added later) goes through libm
    0.2.16 with `force-soft-floats` (libm's `expm1` and `log1p` for `exp_m1` and `ln_1p`), through the `Real` trait's
    methods as `arctan2` and `sinc` already are. `sqrt` is excluded: IEEE 754 requires it correctly rounded, so every
    platform gives the same bits. Enforced by a test in the `discipline_lint` style: every `Float::` or inherent `f64`
    transcendental in the evaluator's source outside the `Real` impl fails, `sqrt` excepted, with a control (a planted
    `Float::sin` fails); a clippy `disallowed-methods` entry would be lint configuration and need a
    ruling (R-197), so the test is the route. Records become identical on every host. It changes today's f64 bits on
    some inputs, so a fixture written from the f64 path becomes a recorded conversion (R-931). A software
    implementation may be slower, which M-A then measures.
  - **(b) Keep the platform library,** and state that bitwise claims hold per machine and per OS. That amends what
    R-521 and R-921 claim across machines.
- **Recommended:** (a), recommended by the owner, 12 Nov 2026 (their words in this entry's header), before any record is
  written on the new route. Measurable alternative, criterion written before any run:
  - *Setup:* the first slice's three charts and grids (TASK-M2-86). Count records that differ bitwise, at f64 before
    narrowing and after narrowing, between the std path and the pinned path on the Mac, and, under each path, across
    four machines: the Mac, Linux, Windows and wasm (the wasm build, as REQ-CHART-155 compares a wasm f64 solve with the
    native one), each against the Mac. The wasm runtime and the Windows host are named before the run.
  - *PASS:* under the pinned path the count against the Mac is 0 on Linux, Windows and wasm, at both levels. The
    std/pinned counts and the std path's cross-machine counts are reported as the size of the change.
  - *Control:* a one-ulp perturbation of `sin` gives a non-zero count at the f64 level, or the harness is void. (At the
    narrowed level it flips about `2⁻²⁹` of components, about 0.03 expected over the slice's roughly `1.8·10⁷`
    components, so it cannot serve there; the physics reviewer's arithmetic.)
- **Applied meanwhile:** nothing. The `real.rs:49–50` comment is corrected as a two-way edit, apart from this entry,
  saying what the default does today.
- **Waits:** TASK-M2-67, TASK-M2-86, TASK-M2-89.

## RQ-800: The measure weight: REQ-DEC-034 and REQ-VAL-094 take it from "the same J_D the linear decode uses", but under R-926 a `J_D` exists only on patched quads, varies across a shallow quad, and a tier-3 map has none *(silence; a one-way door, what statistics claim, and it extends R-935's "central differences only the check"; R-935, R-633, R-926 items 9 and 11, R-936; REQ-DEC-034, REQ-VAL-094, REQ-GEN-018, REQ-DEC-058; context: TASK-M6-09, TASK-M8-128, the chart audit's F13)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F13, with the audit's physics reviewer's widening;
each quote re-read on origin/main 54fa96fc. (a) is recommended by the owner, 12 Nov 2026: "the tier-3 measure weight
(recommend per-sample)". Nothing is chosen here.*

*In plain English: statistics such as "what fraction of this area escapes" need each pixel weighted by how much the
chart stretches the space of systems there. That weight came from the deep-zoom slopes, worked out once per square. The
new chart system has those slopes only on the smallest squares, and charts with no exact slopes have none at all. The
proposal is a weight per pixel from each pixel's own slopes, with a check for the charts that have to estimate them.*

- **File, section:**
  - `plan/requirements.yaml` REQ-DEC-034 (the linearised decode), statement: "and |det J_D| must double as the
    measure weight".
  - `plan/requirements.yaml` REQ-VAL-094 (the per-quad measure weight), statement: "The per-quad measure weight for
    quantitative claims must be |det J_D| taken from the same J_D the linear decode uses."
  - `plan/tasks/M6/TASK-M6-09.md` (the measure weight), Goal (:12): "weighted by `|det J_D|` — taken from the same
    `J_D` the linear decode uses, computed once"; Acceptance (:30): "measure weight equals |det J_D| for the quad".
  - `decisions.md` § R-935, under RQ-758: "A tier-3 map gets no one-cell patch"; "Central differences at R-633's step
    stay as the test that checks a dual slope away from the flagged sites."
  - `docs/design/principia_deep_zoom.md` § "Couplings to the contract", b: "The CPU already computes `J_D` per deep
    quad for the linear decode — take `|det J_D|` and the per-quad measure weight is free."
  - `plan/requirements.yaml` REQ-GEN-018 (`|det J_D|` against the link derivatives), verify: "log √det(J_DᵀJ_D) where
    J_D is not square".
  - `plan/requirements.yaml` REQ-DEC-058, verify, M-B's criterion: "agreement within the central difference's error
    at that step, its truncation `h_k²·|f'''|/6` plus its rounding `ε·|f|/h_k`".
- **The silence:** the weight is defined from the patch's `J_D`, once per quad. Under R-926 item 11 and R-935 a `J_D`
  exists only on patched quads (past `ℓ_switch`); on exact quads above it, where `det J` varies across the quad, one
  weight per quad is wrong; tier-1 and tier-2 maps give a dual slope per sample; a tier-3 map has no slope. A central
  difference of a tier-3 map (a solve or an integration) carries an error of about `τ/h` from the solve's tolerance `τ`,
  and for a chaotic flow map (TASK-M8-128's final-condition charts) it is not reliable at R-633's step. "det" here is
  the Gram area factor `√det(J_DᵀJ_D)` (REQ-GEN-018's form; design §17.11), not a determinant.
- **The question:** where does the measure weight come from, per sample or per quad, and on a tier-3 map?
- **Options seen:**
  - **(a) Per sample.** Each sample's weight is the Gram area factor of its own slope: the dual (tier 1) or implicit
    (tier 2) slope at the sample; on a tier-3 map, a central difference at R-633's step per sample, labelled "measure
    by central difference", admitted per map only if it passes the Richardson check below. Extends R-935's "central
    differences only the check" to the measure on tier 3. REQ-DEC-034's clause, REQ-VAL-094 and TASK-M6-09 are
    rewritten per sample.
  - **(b) Per quad, as written,** from the patch's `J_D` where one exists, and per sample from the dual slope
    elsewhere; tier 3 as (c). Two definitions of one weight.
  - **(c) Tier-3 charts give area statistics only on the uniform measurement grid (`FULL_RETENTION`), unweighted,**
    labelled as chart-area fractions.
  - **(d) No measure-weighted statistics on tier-3 charts.** A refusal, against the no-refusal spirit of R-925 and
    R-926 item 8.
- **Recommended:** (a), recommended by the owner, 12 Nov 2026 (their words in this entry's header). Measurable
  alternative, written before any run:
  - *Per tier-1 and tier-2 map:* per-sample weight against the per-quad weight on shallow quads, the spread of their
    ratio reported.
  - *Per tier-3 map:* a Richardson check, the weight at `h` and `h/2`, `h` R-633's step; criterion: their difference,
    as a ratio of the weight, within M-B's bound form (REQ-DEC-058: `h²·|f'''|/6 + ε·|f|/h`) plus the solve's `τ/h`,
    each term stated per map before the run. A map that fails takes (c), labelled. Positive control: an error planted
    in one map's evaluation, of 100 times that bound (M-B's control factor), must fail the check.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M6-09; TASK-M8-128's statistics.

## RQ-801: An addition to RQ-783 for its M2 answer (R-956): records made past `ℓ_switch` recreate through the exact evaluation, not the patch that was integrated, and across machines through another maths library *(conflict; a one-way door, what a record claims to reproduce; R-521, R-921, R-926, R-956; REQ-TOOL-172; context: RQ-783, TASK-M2-87, TASK-M7-45, the chart audit's F15)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F15, with the audit's physics reviewer's sizing;
each quote re-read on origin/main 54fa96fc. Filed as the owner directed, 12 Nov 2026: "RQ-783's addition". The
recommendation is the audit's. Nothing is chosen here; read with RQ-783 at M2, where RQ-783 carries a dated line
pointing here.*

*In plain English: RQ-783 asks whether pictures saved before the new chart system re-run bit for bit. Two more cases
need the same answer. A pixel deep in a zoom was simulated from the centre-plus-slopes shortcut, but re-opening it
works the system out exactly, so it can differ in the last bit. And on another kind of computer the maths library
can differ (RQ-799). The size is small in smooth regions, about one pixel in a hundred thousand at depth 21 by
estimate.*

- **File, section:**
  - `REVIEW_QUEUE.md` § RQ-783 (pre-switch records re-running bit for bit), the question: "does a record written
    before R-926 item 5's route lands recreate bitwise, and through which decode?"; Deferred: "deferred to M2 by owner
    11 Nov 2026 (R-956, the owner: "RQ-783: defer:M2")".
  - `docs/design/principia_ic_save.md` (:15–16): "Integration enters only through `computeIC(chart, uv, simKey)`";
    (:37): `source:    "built" | { pixel: { chart, uv } }`.
  - `docs/design/principia_honesty_precision.md` (:191–192): "the f32 run is the survey kernel's one-pixel dispatch
    through the atlas's decode path, bitwise what coloured the pixel".
  - `docs/design/principia_trajectory_viewing.md` (:135): "In regular regions, agreement certifies the pipeline
    (disagreement = a real bug)."
  - `decisions.md` § R-921: "to the original physical starting state, bitwise, and runs from there".
- **The conflict:** a record stores `(chart, uv)`, and recreation is an exact f64 evaluation. Past `ℓ_switch` the
  integrated IC was the patch's, which depends on the quad's level and centre, so an exact recreate can differ from it.
  honesty_precision :191–192's bitwise claim then fails. trajectory_viewing :135 compares outcome class in regular
  regions, which a one-ulp IC difference does not flip, so it does not misfire (the physics reviewer). Size: on smooth
  patched quads about `h²‖D″‖/ulp` per component, about `10⁻⁵` of samples at depth 21, *inferred*, at a fixture whose
  `|x₀|` per component admits depth 21 (RQ-798's fixture rule). Under RQ-798 (a′) the crease strips are
  exact per sample, so they recreate. Across machines, RQ-799 decides.
- **The question:** does a record made past `ℓ_switch` recreate the IC that was integrated, bitwise?
- **Options seen:**
  - **(a) The record names its route** (exact or patch) and, for a patch, the quad's level and address, so recreation
    re-forms the patch and reproduces the integrated IC bitwise; the inspector's one-pixel trace uses the quad's
    record. With RQ-799 (a), across machines too.
  - **(b) The exact CPU evaluation is canonical.** Recreation and the inspector use it; a patched pixel's integrated IC
    is documented as within the linearisation error of the canonical one; honesty_precision's "bitwise what coloured
    the pixel" is amended past `ℓ_switch`.
  - **(c) As (b), labelled per record** "recreated from the exact evaluation".
- **Recommended:** (a), with RQ-799 (a), the audit's recommendation. Measurable alternative, written before any run: on
  patched quads at depths 21 to 25, at a fixture stating `|x₀|` per component (RQ-798's fixture rule), the share of
  samples whose exact recreate differs bitwise from the patch record, and the outcome flips that causes against the
  one-ulp chaos baseline (R-84). Criterion: any non-zero share shows the question is real; the share is reported against
  the `10⁻⁵` estimate.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M2-87 (whether a record carries its route), TASK-M7-45, RQ-783's M2 answer.

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
    run. Arm (f) asserts its precondition first: at least one adjacent pair whose f64 ICs are bitwise equal at distinct
    f64 sample points. Like arms (b) to (e), it may feed the trigger synthetic quad facts, here built from the CPU's f64
    evaluation at those sample points.
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

## RQ-803: The collapse test fires late: R-90 and `AT_F32_FLOOR` need every word of adjacent records bitwise equal, while the dominant components are step-quantised levels earlier, undetected *(silence; a one-way door if it becomes a criterion, it amends R-90's trigger and what the floor warning claims; R-90, R-935, R-599, R-632, R-433, R-54; REQ-SCHED-062, REQ-DEC-033, REQ-DEC-052, REQ-TOOL-152, REQ-GUI-012; context: TASK-M5-04, TASK-M6-08, the chart audit's F21 (the physics reviewer's "missed" 3))*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F21, the audit's physics reviewer's missed point 3;
each quote re-read on origin/main 54fa96fc. Filed as the owner directed, 12 Nov 2026: "the collapse-test lateness". The
recommendation is the audit's. Nothing is chosen here.*

*In plain English: the program decides that precision has run out only when two neighbouring pixels' starting systems
are identical in every one of their numbers. Long before that, the biggest numbers can already be stuck in steps, so
neighbouring pixels differ only in their smallest numbers and the picture can show stripes, with no warning. The
question is whether to keep the strict test, or also count the earlier stepping.*

- **File, section:**
  - `decisions.md` § R-90: "Switch to the linearised decoder when the full decoder's adjacent samples give
    bitwise-identical ICs"; refined by R-935: "the trigger's "bitwise-identical ICs" compares the narrowed f32 records".
  - `docs/contracts/principia_scheduler_contract.md` Part 4 (:138): "Only when the *linearised* decoder's samples
    collapse to bitwise-identical ICs … is f32 genuinely exhausted".
  - `plan/requirements.yaml` REQ-TOOL-152 (the banding criterion, confirmed by R-433): "a delta departing from 2h/N by
    more than 1/16 of the step is banded".
  - `docs/design/principia_deep_zoom.md` § 1 (:62–65): "A run of reconstructed coordinates is banded, step-quantised,
    when an adjacent-sample delta departs from the exact step `2h/N` by more than **1/16 of the step** … the criterion
    holds while the step spans at least 16 ulps."
- **The silence:** the banding criterion is defined for the chart coordinate `u`, never for the IC's components. A
  record's components quantise one by one as the depth grows: the dominant ones first, levels before the last component
  collapses. R-90's switch and `AT_F32_FLOOR` fire only when every word is equal, so the quantised stretch is
  undetected, and REQ-GUI-012's warning comes late there. (On a 1/16 criterion a component quantises about four levels
  before it collapses, `log₂ 16`, *inferred*.)
- **The question:** does the switch or the floor stop also fire on per-component quantisation of the records?
- **Options seen:**
  - **(a) Keep R-90's bitwise test as written,** and report beside REQ-DEC-052's first collapsing depth the first depth
    at which any component, along each in-plane axis, fails REQ-TOOL-152's 1/16 criterion. Nothing changes in what
    is computed; the gap is measured. Adding a reported column is two-way.
  - **(b) Add per-component quantisation as a trigger:** a component whose adjacent deltas along an axis fail the 1/16
    criterion counts as collapsed along that axis, judged per axis as R-599 judges collapse. Amends R-90's trigger and
    R-54's floor warning; changes when quads stop.
  - **(c) Quantisation raises the precision warning only,** without changing the switch or the stop. Amends R-54.
- **Recommended:** (a) now, the audit's recommendation, with (b) the measurable alternative: if (a)'s reported gap
  exceeds one level at any stated
  `|x₀|` on the first slice's charts, (b) or (c) comes back to the owner with those numbers. The criterion is written
  here, before the run.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M5-04 (REQ-DEC-052's gate, where the column is reported), TASK-M6-08.

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

## RQ-805: The fragment's backwards answer and `ctx.chart.z` past `ℓ_switch`: both are absolute f32 values, which R-395 rules out for the forward coordinate there, so Chart position's View frame, the round-trip and decode presets and `ctx.chart.z` stop resolving samples from about depth 20, and the answer sooner on most charts *(silence; a one-way door, it changes what the fragment computes and shows and reads R-947's "the same function's generated WGSL"; R-926 items 7 and 11, R-945, R-947, R-935, R-395, R-394, R-570, R-90; REQ-CHART-199, REQ-COL-134, REQ-RENDER-125, REQ-RENDER-126, REQ-TOOL-158, REQ-COL-063, REQ-COL-057; context: TASK-M2-99, TASK-M7-47, TASK-M7-52, TASK-M7-74, RQ-797, the chart audit's F23)*

*Queued 12 Nov 2026 from the chart audit the owner ordered, finding F23, item 1, on the owner's pass of that day on
the backwards direction ("file any one-way part with the other eight"); each quote re-read on origin/main 54fa96fc. The
recommendation is the audit's. Nothing is chosen here.*

*In plain English: to colour a pixel by where its system is now, or to show the round-trip check, the graphics card
works out a chart position from the system. The graphics card only has single precision, and it returns that position
as one absolute number. Zoomed in past about level 20 (sooner on most charts, because the solve itself loses a few
digits) neighbouring pixels get the same answer, so these views go blocky. The owner already ruled the same problem away
for the forward direction by working relative to each square's centre (R-395). The question is whether the backwards
answer, and the chart value shaders read (`ctx.chart.z`), get the same treatment.*

- **File, section:**
  - `decisions.md` § R-926 (the new chart system), the owner's words, item 7: "The backwards direction (running state ->
    chart position) exists ONLY for the fragment-side needs: chart-position colouring / the state map, ctx.chart.z, and
    the decode/round-trip debug presets."; item 11: "A small quad's table collapses to one cell — that IS the linearised
    decoder (centre + slopes)."
  - `decisions.md` § R-947 (one placement function), and `docs/design/principia_chart_composition.md` § 10.6
    (:1503–1505): "The fragment reads the same function's generated WGSL where the chart has closed-form code, and the
    backwards table with its search where it has none".
  - `decisions.md` § R-935 (the re-plan's eleven answers), under RQ-767: "The table seeds the search, and Newton steps
    run on the chart's generated forward code where the chart has it"; under RQ-768: `ctx.chart.z` "stays the forward
    value of colour_composition § 3. On a table-route chart the forward table carries `z` beside the state at each node
    (eight more f32 words a node)".
  - `decisions.md` § R-395 (no banding, absolute to `ℓ_switch`, quad-local beyond), the owner's words: "no banding
    beyond it through the per-quad local coordinates"; its decision: "**Beyond ℓ_switch**, the coordinate is carried as
    the per-quad local coordinates, the quad's centre `c` (f64 on the CPU) and the f32 offset `δ` … never as an absolute
    f32 `u`"; its sweep: "N = 16 to ℓ = 19, collapsing at ℓ = 20".
  - `plan/requirements.yaml` REQ-TOOL-158 (quads past `ℓ_switch` on quad-local coordinates): "in the kernel and in the
    fragment's UV preset". It covers the forward coordinate only.
  - `docs/design/principia_colour_composition.md` § 3 (:452–454): "Formed in f32 it is an absolute coordinate, which
    R-395 keeps free of banding only up to the deep-zoom switchover (ℓ_switch, R-90); past it a fragment works in the
    quad's local coordinates"; (:460–464) `ctx.chart.z` is obtained "by evaluating the chart's own `D ∘ Φ` in WGSL at
    the pixel's `ctx.chart.slice_uv`".
  - `docs/design/principia_state_map.md` SM7 (:416–422): the View frame, "`s = tanh((x − view centre)/view
    half-width)`", "is needed at depth"; "the state is f32. The `t = 0` identity holds to **4e-7 (latent), 1.5e-6
    (sphere), 5e-6 (simplex), 2.7e-8 (momenta), 7e-8 (θ × h), 1.9e-6 (tilted E, F)** in chart units".
- **The silence:** R-395 and REQ-TOOL-158 keep the forward coordinate quad-local past `ℓ_switch`; nothing does so for
  the fragment's backwards answer or for `ctx.chart.z`. WGSL has no f64, so both fragment routes return an absolute f32
  point. At `|u|` in `[0.5, 1)` and `N = 16` a correctly rounded answer stops separating adjacent samples at depth 20
  (R-395's arithmetic), about `2^(ℓ−20)` samples sharing each answer past it; the f32 solve is not correctly rounded,
  and SM7's identity errors reach one sample spacing at about depth 17 (latent), 15 (sphere, tilted E, F) and 14
  (simplex) (*inferred*, F23, assuming SM7's chart units are the quadtree's units, a root quad of width 1 and a sample
  spacing of `2^−(ℓ+4)` at `N = 16`: 4e-7 is `2^−21.25`, so depth 17; if a chart's home window spans `W` chart units,
  every one of these depths shifts by `log₂ W`). The View frame's `x − view centre` then shows blocks; the round-trip
  preset reads the floor. On a closed-form chart `ctx.chart.z` is `D ∘ Φ` at an absolute `ctx.chart.slice_uv`, which
  R-395 rules out past `ℓ_switch`, and on a table-route chart its nodes are absolute f32, so it collapses near 20 at
  O(1) `z`. A backwards answer can never beat its input: the f32 state separates samples only to `ℓ_k ≈ 19 +
  log₂(|∂x_k/∂u| / |x₀,k|)`, so a quad-local answer gains depth past ~20 only where a component is small relative to its
  slope, and gains the solve's lost digits everywhere.
- **The question:** Q1, past `ℓ_switch`, is the fragment's backwards answer absolute f32 or quad-local? Q2, is
  `ctx.chart.z` past `ℓ_switch` one absolute f32 value, or does it gain a quad-local lane?
- **Options seen:**
  - **Q1 (a) Absolute, as now.** Nothing changes; the floor is stated in SM7 (a two-way note) and reported by M-E.
    Chart position's View frame and the round-trip and decode presets go blocky from about depth 14 to 20 by chart.
    R-570 (precision never limits navigation) is kept for navigation, not for these views.
  - **Q1 (b) Quad-local past `ℓ_switch`, absolute beyond the quad's patch.** For a state near its own quad, the fragment
    solves `δ = J⁺(X − x₀)`, the one-cell patch run backwards (R-926 item 11), and draws `(c_q − c_view) + δ` with `c_q
    − c_view` formed in f64 on the CPU. `x₀` is sent as two f32 words a component (high and low) so its rounding is no
    per-quad seam, about 62 f32 words, 248 B, per visible quad (*inferred*); under RQ-797 (a) this is new upload, under
    (b) the uniforms exist. Validity: the solve's residual in state within `X`'s f32 rounding (R-935's on-image
    criterion), else the absolute route. A tier-3 map has no `J_D` and keeps the absolute route. Reads R-947's "the same
    function's generated WGSL": past `ℓ_switch` the fragment places by a linear solve from the one source's `x₀` and
    `J_D`, not by its generated code. The gain is near `t = 0` and in regular regions; most running states leave their
    quad and take the absolute route.
  - **Q1 (c) The CPU places every visible sample with `place` and uploads offsets.** Exact to the lock's tolerance at
    any depth; about one solve a sample a frame while time runs, several forward evaluations each at about 85 ns
    (composition §17.12:2496), far past a frame for a whole screen (*inferred*). Not viable while time runs.
  - **Q2 (i) `ctx.chart.z` stays one absolute f32 vec8.** It collapses near depth 20 at O(1) `z`, stated in colour § 3
    (a two-way note).
  - **Q2 (ii) A quad-local `z` lane past `ℓ_switch`, beside the absolute one:** `z₀` per quad from the CPU's f64 (high
    and low words) and an f32 offset per pixel, `J_z·δ`, as R-395 does for `ctx.chart.slice_uv`. A change to colour §
    3's context that every shader can read; the absolute lane stays for shaders that need no depth.
- **Recommended:** Q1 (b) and Q2 (ii), the audit's recommendation: the forward rule the owner already made (R-395)
  applied to the backwards answer and to `z`. Measurable alternative, written before any run: M-E, the audit's
  measurement of the fragment against `place`, run first under (a): per route (the generated encode, the table with
  Newton steps on generated forward code, the table alone, and a morph frame on both blend pairs), at depths 0, 10, 16,
  20, 21, 25 and 30, at fixtures stating `|x₀|` and the slopes per component (RQ-798's fixture rule) and `|u|` per
  axis, 64 × 64 samples at `t = 0` and at a stated `t > 0`, the distance between the fragment's point and `place`'s
  reported in sample spacings, median and worst per route and depth; positive controls, either failing voids the run: a
  fragment answer offset by one spacing at depth 10 shows a distance of at least one spacing, and the fragment's
  algorithm run in f64 on the CPU stays within one spacing at depth 25 where the f32 run does not. If, at
  every fixture, the first depth at which the worst distance to `place` exceeds one sample spacing is within one level
  of that fixture's forward floor `ℓ_k`, the absolute answer already reaches what its input allows and (b) adds nothing:
  (a) is then recommended. Otherwise (b), whose own M-E arm is predicted within one level of `ℓ_k`.
- **Applied meanwhile:** nothing. Two-way notes in SM7 and colour § 3, apart from this entry, state the floor without
  choosing.
- **Waits:** TASK-M7-74 (M-E's quad-local arm), TASK-M7-47 (the View frame past `ℓ_switch`), TASK-M7-52 (the
  generated projection's reading past `ℓ_switch`); RQ-797 (whether the per-quad `x₀` and `J_D` are uploaded at all).

## RQ-795: Aarseth–Zare's landing tolerance `LAND_EPS_REL`: the step control branches on it, rule 1 forbids it in a branch, and the corpus gives it no value *(conflict and silence, a missing value, R-71; a one-way door, it changes when a sync interval counts as landed and so what the occupant computes, at both precisions; Part 2b steps 6 and 7, Part 2c rule 1; R-927, R-953; REQ-INT-049, REQ-INT-114; context: TASK-M3-68)*

*Queued 11 Nov 2026 by the implementer of TASK-M3-68 (the Aarseth–Zare occupant); each quote re-read on `main` at
50f406e4. Nothing is chosen here.*

*In plain English: Aarseth–Zare marches each sync interval in small steps of a stretched time, and stops when the
clock has reached the interval's end "closely enough". The contract's transcription of the step control says how close
with a tolerance, `LAND_EPS_REL`, in three places, but the contract's own rule 1 forbids exactly that constant in a
branch, and it gives no value for it. The march cannot be built without some landing test, so the choice of test (and,
if it is a tolerance, its value) is the owner's.*

- **File, section:**
  - `docs/contracts/principia_integrator_contract.md` § "Aarseth–Zare: the equations, the reference body, the
    Levi-Civita inverse and the step control (transcribed)", step 6, "The landing secant": "When the step just taken
    reaches the boundary (`t_s ≥ dt_left − dt_left·LAND_EPS_REL`) … stop if `got` is not finite, `got ≤ 0`, or
    `|got − want| ≤ dt_left·LAND_EPS_REL`"; and step 7, "The exit": "The march ends when
    `t_s ≥ dt_left − dt_left·LAND_EPS_REL`, or the state is not finite, or the budget is spent".
  - The same section, after step 8: "`SYNC_EPS` and `LAND_EPS_REL` are the reference's precision-dependent constants
    that Part 2c rule 1 forbids in a branch; their values are not part of this transcription."
  - The same contract, § "Rules the new kernel must hold by construction", rule 1: "prin-rs gives `f32` and `f64`
    different `SYNC_EPS` (1e-6 / 1e-15) and `LAND_EPS_REL` (1e-5 / 1e-14), **both sitting directly in branch
    conditions.** … If a tolerance must scale with precision, it belongs where the *value* is used, never where the
    *branch* is taken."; and R-953's note there: "Rule 1's text stands for every other constant: `SYNC_EPS` and
    `LAND_EPS_REL` stay forbidden in a branch."
- **The conflict:** steps 6 and 7 branch on `LAND_EPS_REL`; rule 1 and R-953 forbid it in a branch; neither says what
  the landing test is instead. **The silence:** no value is given (the values rule 1 quotes are prin-rs's, described as
  the defect).
- **`SYNC_EPS` does not need a ruling:** TASK-M3-68 replaces the reference's `t ≥ t_target − SYNC_EPS` (step 1) by
  integer sync-boundary indices, so no tolerance reaches that branch (applied per R-369, an implementation choice that
  changes no result on the grid R-565 fixes).
- **Options (measurable):** (a) one tolerance for both precisions, set where the value is used, as rule 1 allows, a
  calibration with its value proposed (R-71); (b) no tolerance: the interval lands when the secant's last retake ends
  the interval clock at or past `dt_left`, the clamp then setting the remainder exactly, so the branch compares `t_s`
  with `dt_left` alone; (c) the reference's per-precision values, which needs rule 1 amended. Each changes the
  trajectory by at most the landing residual, which the `az-final-step` gate measures; the figure-eight closure under
  each can be run on request.
- **Applied meanwhile:** TASK-M3-68 takes the tolerance as a parameter of the occupant's configuration
  (`AzParams::land_eps_rel`), with no value in the kernel and no production configuration; the gates and tests that
  run the occupant state the reference's values (`1e-5`, `1e-14`) as their run condition, marked with this entry.
- **Waits:** the occupant's production configuration (its `AzConfig`), with REQ-INT-080 and REQ-INT-105's values;
  TASK-M3-68 itself does not wait.

## RQ-796: Aarseth–Zare's floor on `|R₃|` in the equations of motion has an f64 value only, `1e-300`, which is 0 at f32: the corpus gives no f32 value *(silence, a missing value, R-71; a one-way door, it changes what the occupant computes at f32 near a close approach of the unregularised side; Part 2b § "The equations of motion in fictitious time `τ`"; R-953; REQ-INT-105; code review 5873667061 of PR #502, C3; context: TASK-M3-68, TASK-M3-08)*

*Queued 12 Nov 2026 by the implementer of TASK-M3-68 (the Aarseth–Zare occupant), fix round 1; each quote re-read on
`main` at 416e5cd7. Nothing is chosen here.*

*In plain English: Aarseth–Zare removes the singularity of two of the three pairs, but the third side, between the two
bodies other than the reference body, keeps its `1/|R₃|` term. The equations guard it with a tiny floor so that a
division by zero cannot happen. The contract gives that floor as `1e-300`, a double-precision number; in single
precision, which the GPU runs, `1e-300` rounds to exactly 0, so there is no floor at all. The contract gives no
single-precision value, so which value to use there is the owner's.*

- **File, section:** `docs/contracts/principia_integrator_contract.md` § "Aarseth–Zare: the equations, the reference
  body, the Levi-Civita inverse and the step control (transcribed)", "The equations of motion in fictitious time `τ`":
  "on the state `(u₁, p₁, u₂, p₂, t)` with `E` held, `A` and `B` here not floored and `|R₃|` floored at `1e-300`
  (`tb_az.py:114-118`)"; the same section, on the reference body: "Hence the floor on `|R₃|` (`tb_az.py:118`) and
  `|R₂ − R₁|` in the predictive limit's `d_min`".
- **The neighbouring rule:** the same section, R-953's note on the inverse map's guards: "`system.rs` floors at `TINY`
  too, where `tb_az.py` floors at `1e-300`: `A` and `B` in `phys_from_state` (`system.rs:93-94`) and the three
  separations in `energy_phys` (`system.rs:109-111`)". `TINY` is "one value per working precision, a calibration (R-71,
  REQ-INT-105)", the owner's numbers `1e-37` at f32 and `1e-300` at f64. The note names no floor for `|R₃|` in the
  equations of motion, so the transcription's `1e-300` stands there, an f64 value.
- **The silence:** `1e-300` is below f32's least subnormal (about `1.4e-45`), so at f32 it is `0`, and no f32 value is
  given.
- **Options (measurable):** (a) the floor is `TINY` at each width (`1e-37` at f32, `1e-300` at f64 in the reference's
  values), as R-953 maps `tb_az.py`'s other `1e-300` floors onto `TINY`, its value then following REQ-INT-105's
  proposal; (b) a calibration of its own per width, proposed from the least pre-floor `|R₃|` over the close-encounter
  fixtures, as REQ-INT-105's verify does for `A` and `B`; (c) no floor at f32, a non-finite `|R₃|` term then caught by
  the march's explicit finiteness test (Part 2b, step 7). Each can be measured on `separation-floor --propose`'s
  fixtures: the floored-step count and the least `|R₃|` reached at f32.
- **Recommendation:** (a), as the reading most consistent with R-953, which treats `tb_az.py`'s `1e-300` floors as
  `TINY` at each width.
- **Applied meanwhile:** TASK-M3-68's kernel takes the floor as a parameter of the occupant's configuration
  (`AzParams::r3_floor`), with no value in the kernel and no production configuration. Its tests and gates run with
  `1e-300` at f64 and **no floor (0) at f32**, and say so in their module notes and in `separation-floor --propose`'s
  output, until this entry is ruled.
- **Waits:** the occupant's production configuration (its `AzConfig`), with REQ-INT-080's and REQ-INT-105's values;
  TASK-M3-68 itself does not wait.

## RQ-806: Which `sim_failed` detail code a regularised march's non-finite exit writes, when the state it stores is the latched finite one *(silence and conflict, a meaning, R-72; a one-way door, it changes what the instrument records for a failed sample; payload §2, §8; integrator contract Part 2b step 7, Part 7; R-30; physics review 5879658175 of PR #502, P1; context: TASK-M3-68, TASK-M3-18)*

*Queued 12 Nov 2026 from physics review 5879658175 on PR #502 (TASK-M3-68, the Aarseth–Zare occupant); each quote
re-read on `main` at 54fa96fc. Nothing is chosen here.*

*In plain English: when a regularised march breaks down numerically, it stops and keeps its last good state, and the
sample is marked failed with a small code saying what went wrong: 0 for a NaN in the state, 1 for an infinity in the
state, 2 for a blown-up derived quantity such as the energy or force. What broke is the march's own internal state,
which held NaN or infinity, but what is stored is the last good state, which holds neither. So the code can be read off
the stored state (2), off the broken internal state (0 or 1), or off the cause. Which one the instrument records is the
owner's.*

- **File, section:**
  - `docs/design/principia_dd_simstate_payload.md` § "2. Bit layouts (the packed u32s)" (:296): "**`sim_failed`**
    (state 4, numerical breakdown *during integration*): `0` = NaN in state; `1` = Inf/overflow in state; `2` =
    non-finite derived quantity (energy/force blew up); `3` = reserved."
  - The same file, § "8. Build-time settles" (:753): "**Failure `detail` enum** — the sim_failed/decode_failed category
    codes (§2) are drawn from the integrator contract's actual failure modes; confirm they match the contract's
    enumeration when it's finalised."
  - `docs/contracts/principia_integrator_contract.md` § "Aarseth–Zare: the equations, the reference body, the
    Levi-Civita inverse and the step control (transcribed)", step 7, "The exit": "The march ends when
    `t_s ≥ dt_left − dt_left·LAND_EPS_REL`, or the state is not finite, or the budget is spent"; and, for an entry
    size that is not finite: "The entry size is then infinite and the step's state is not finite".
  - The same contract, Part 7 (:1093): "`sim_failed` (NaN/Inf during integration) and `decode_failed` … are lifecycle
    states".
  - The latch, as built (`crates/kernel/src/regularisation/az.rs`, `End::NonFinite`): "The state was not finite at the
    explicit test (Part 2b, step 7): the copy stopped, its state latched at the last finite one."
- **The silence:** the codes are defined on "the state", and a regularised march has two: the regularised `(Q, P, t)`
  the step-7 test rejected, non-finite, and the stored Cartesian state, finite by construction. Payload §8 leaves the
  codes open, and TASK-M3-18 owns their cross-check against the contract's failure enumeration.
- **Options:**
  - (i) 2, read on the stored state, as built (`failed_detail` on the latched state);
  - (ii) 0 or 1, read on the first non-finite regularised state the step-7 test rejected (0 if any component is NaN,
    else 1);
  - (iii) by cause: 2 where the step size `h` was not finite, else 0 or 1 read on the stepped state as in (ii).
- **Measured** (the `az_doubly_degenerate_is_finite` fixture, f32, three bodies within `3e-38` at rest, TASK-M3-68's
  test values; a throwaway probe, not committed, run through the job queue as job 5298 on 3ae7ae9e): the entry size `dτ_entry` is
  `+∞`, so the step size `h` is `+∞`; RK4's step from the registered state gives all nine components of
  `(u₁, p₁, u₂, p₂, t)` NaN (bits `0x7fc00000`), none infinite. So this fixture writes **2** under (i), **0** under (ii)
  and **2** under (iii). Under (i) and (iii) the code is the same here; they differ on a finite step whose stepped state
  overflows.
- **Measured over the separation-floor and close-encounter fixtures at f32** (physics review 5880652590's measurable
  alternative; a throwaway probe, not committed, job 5312 on 2424506a; each fixture as `separation-floor --propose` runs
  it, Aarseth–Zare + RK4 at `η = 0.01`, `Δ_sync = 0.125`, the reference's `TINY` and landing tolerance, no `|R₃|`
  floor at f32):

  | fixture | end at f32 | steps | (i) | (ii) | (iii) |
  |---|---|---|---|---|---|
  | the radial collision passage, to `t = 2` | marching, landed | 2528 | — | — | — |
  | test 7's close-encounter hierarchy, to `t = 10` | marching, landed | 8463 | — | — | — |
  | Burrau's (3, 4, 5) from rest, to `t = 20` | marching, landed | 22097 | — | — | — |
  | the doubly degenerate state (`separation-floor --propose`'s, unit masses) | non-finite after 1 step | 1 | 2 | 0 | 2 |
  | `az_doubly_degenerate_is_finite`'s fixture (masses `1/3`) | non-finite after 1 step | 1 | 2 | 0 | 2 |

  None of the three close-encounter fixtures fails at f32, so no option writes a detail there. In both degenerate
  states the step size `h` was not finite: the entry size and `c₀` were both unusable. So **(i) and (iii) never differ
  on these fixtures**, and (ii) differs from both (0 against 2).
- **Recommendation (physics review 5880652590 of PR #502):** option (iii), by cause. Payload §8 (:753) says the codes
  "are drawn from the integrator contract's actual failure modes", and Part 2b step 7 names this failure's mode as an
  entry size that is not finite, a derived quantity, which is payload §2's code 2. (iii) records that mode, and records 0
  or 1 when a finite step overflows the state. (i) writes 2 for every regularised failure, so a later state overflow
  cannot be told apart from it. (ii) writes 0 in the doubly degenerate case, although the step size failed first. Its
  measurable alternative is above: (i) and (iii) agree on every fixture, so (i) is adequate for now.
- **Applied meanwhile:** 2, option (i), as built by TASK-M3-68 (`crates/kernel/src/driver/wrapper.rs` `integrate_with`,
  its doc comment marking the detail interim until this entry is ruled), as TASK-M3-05's interim writer was.
- **Waits:** TASK-M3-18 (the one `sim_failed` writer and the cross-check of payload §2's codes); TASK-M3-68 itself does
  not wait.
