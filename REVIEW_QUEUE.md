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
- **Deferred:** deferred to the start of TASK-M7-12, options with a recommendation proposed then by owner 5 Nov 2026 (R-804, the owner: "RQ-675: Defer to TASK-M7-12. Propose options with a recommendation when it starts.").

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
- **Deferred:** deferred to the start of TASK-M7-12, options with a recommendation proposed then by owner 5 Nov 2026 (R-805, the owner: "RQ-676: Defer to TASK-M7-12. As above.").

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
- **Deferred:** deferred to the start of TASK-M7-13, options with a recommendation proposed then by owner 5 Nov 2026 (R-806, the owner: "RQ-677: Defer to TASK-M7-13. As above.").

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
- **5 Nov 2026 (R-720; physics review 5801728548 of #425, P2):** R-507 U1 divides `d` by "that coordinate's slider span, the Slice centre slider's range". R-708 puts a held `K`'s tilt `d` in units of `t`, and a slice of a held `K` stays by value, so a by-value slider has its span in `K`. The same slider then has two units on `Φ_θ,L_z`. The span's definition has to say which, and how they convert.
- **Deferred:** deferred to the start of TASK-M8-48, options with a recommendation proposed then by owner 5 Nov 2026 (R-807, the owner: "RQ-679: Defer to TASK-M8-48. As above.").

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

## RQ-682: Which of the four axis kinds a virial-ratio axis is: R-471 H3 says it is not kind 3, the kinds are a closed set, and kind 2 is a scalar inside one block *(silence: R-471 H3; R-506 W2; chart_decoder_contract § "The four axis kinds — a closed set"; context: TASK-M2-37, TASK-M8-18)*

*Queued 3 Nov 2026 by TASK-M2-37. Nothing in TASK-M2-37 waits: the kernel's virial-ratio axis
(`kernel::chart::physical::AxisVirial`, chart_reference §2.1's new paragraph) decodes and validates, but it has no
authoring descriptor, so the well-posedness validator cannot take a chart with one and no chart offers it.*

- **File, section:**
  - `decisions.md`:13867, § "R-471 — …", H3: "The virial ratio and the mass ratio are not kind 3, so B1 stands for
    them."
  - `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set": kind 2 is a "scalar
    inside one block: `m₀`, `θ`, `φ`", which "inverts into its block"; kind 3 is "`E`, `L_z`, `K`, `KE`, `PE`", which
    "*solves* into a downstream sector, upstream sectors read as fixed".
  - `docs/design/principia_chart_reference.md` § "2.1 …", the virial ratio's paragraph: `K* = Q(t)·|U|/2`, using the
    pixel's own `U`.
- **The silence:** `Q = 2K/|U|` reads the configuration and the masses (through `U`) and solves into the momenta, as a
  kind-3 axis does, but H3 says it is not kind 3. It is not inside one block, so it is not kind 2 as written. No kind
  fits, and the set is closed, so the descriptor a builder chart needs (its kind, the blocks it reads, its downstream
  check and its feasibility flag) is not given.
- **The question:** what kind a virial-ratio axis is.
- **Options seen:** (a) kind 3, with H3 read as being only about B1's extension, which R-506 has since superseded;
  `InvariantQuantity` then gains the virial ratio, solving into the momenta and reading the configuration and the
  masses; (b) kind 2 in the momenta, with a note that its inversion reads the upstream blocks; (c) another reading,
  stated by the owner.
- **Applied meanwhile:** nothing. The engine's descriptors cover the lone `K`, `KE`, `E` and `L_z` axes and the mass
  ratio (kind 2 in the masses, its residual the third body's mass), not the virial ratio.
- **Waits:** the builder's virial-ratio axis (TASK-M8-18).

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

## RQ-703: Whether the quick tier goes ahead of R-683's step (1), as the owner's 4 Nov order says, or after it, as R-683 and R-684 say *(conflict: R-713, R-683, R-684; context: R-716, PR #414)*

*Queued 5 Nov 2026 by the port of the owner's saved orders. For the owner: an order and a ruling disagree on the series' order. Nothing is reworded.*

- **File, section:**
  - `decisions.md` § R-713, the owner's words, 4 Nov 2026: "The quick tier from the CI series is the structural fix, so move it up the series."
  - `decisions.md` § R-684: "**R-683's steps (2) and (4) are ruled**, still in that order, after step (1), the test-binary consolidation".
  - `decisions.md` § R-683, the owner's words: "Then, in order, each as its own tooling PR".
  - `decisions.md` § R-716, the owner's words: "Order: 1 and 4 first (cheap, likely solve disk), then 2 and 3, then the rest." It doesn't say how that list sits with R-683's series; the orchestrator interleaved them, applied per R-369.
- **Observed:** PR #414 merged the quick tier, and `main` at 8636f776 has no step (1) consolidation.
- **The question:** does R-713's order amend R-683 and R-684, and how do R-716's items sit with R-683's steps?
- **Options seen:** (a) R-713 stands, the quick tier ahead of step (1), R-683 and R-684 amended, and the orchestrator's interleaving accepted; (b) R-684's order stands for what remains, step (1) next; (c) another, stated by the owner.
- **Applied meanwhile:** nothing is reworded. Merged work stays merged.
- **Waits:** nothing in the build; only the order of the remaining CI steps.

## RQ-704: Whether a merge of `main` with no conflict stays allowed right before `merge-ready` and at an early start, given the owner's "don't merge main in" *(conflict: R-717, R-676, R-388)*

*Queued 5 Nov 2026 by the port of the owner's saved orders. For the owner: an order and two rulings disagree. Nothing is reworded.*

- **File, section:**
  - `decisions.md` § R-717, the owner's words, 4 Nov 2026: "If it's process only, don't merge main in, and don't re-run CI, just to refresh a branch that has no conflict."
  - `decisions.md` § R-676: "`main` is merged in only on a conflict, when a merged change must be in the tree, or once right before the label, the reviewers then approving the merged head".
  - `plan/OPERATIONS.md` § "Merging": "or once, right before the `merge-ready` label, after the reviewers' approvals and before § "Merging" step 2's merged-tree check."
  - `decisions.md` § R-388 and `plan/WORKFLOW.md` § "The unit: one task, one branch, one PR": "then it merges `main` in and opens its PR normally".
- **The question:** are R-676's "once right before the label" and R-388's merge "our own process", so that they lapse where there is no conflict?
- **Options seen:** (a) both lapse: `main` is merged in only on a real conflict or when a merged change must be in the tree, § "Merging" step 2's merged-tree check standing in; (b) R-676's merge lapses and R-388's stays, its branch being off a head that has since merged; (c) both stay, as named process merges outside the order; (d) another, stated by the owner.
- **Applied meanwhile:** nothing is reworded in R-676, R-388 or the two plan files.
- **Waits:** nothing in the build.

## RQ-705: Whether CI's test binaries take `strip = "debuginfo"`, as candidate 4 lists, given R-374's backtraces *(conflict: R-716, R-374; context: PR #394)*

*Queued 5 Nov 2026 by the port of the owner's saved orders. For the owner: an order and a ruling disagree. Nothing is reworded.*

- **File, section:**
  - `decisions.md` § R-716, the owner's item 4: "`strip = "debuginfo"` for test binaries. Expect large disk and link-time savings; confirm panic backtraces stay usable."
  - `decisions.md` § R-374: "Every CI job sets `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and `CARGO_PROFILE_TEST_DEBUG=line-tables-only`", "Panics keep `file:line` backtraces".
- **Observed:** PR #394's body says the strip half "is not applied, because it can't hold with R-374".
- **The question:** does the candidate stand against R-374, or does R-374's backtrace keep the strip out?
- **Options seen:** (a) R-374 stands, and the strip half of item 4 is dropped; (b) the strip is taken and R-374 amended for test binaries; (c) another, stated by the owner.
- **Applied meanwhile:** PR #394's: no strip.
- **Waits:** nothing in the build.

## RQ-706: Whether caching the kernel's output, candidate 3, is allowed under R-285's "caches only" *(conflict: R-716, R-285; context: R-320, R-656, R-684)*

*Queued 5 Nov 2026 by the port of the owner's saved orders. For the owner: an order and a ruling disagree. Nothing is reworded.*

- **File, section:**
  - `decisions.md` § R-716, the owner's item 3: "Cache the kernel. Key `cargo xtask build-kernel` output on a hash of the kernel crate sources + toolchain; skip the rust-gpu build entirely when unchanged."
  - `decisions.md` § R-285's heading: "CI caches only the cargo registry and the fixture pool, with per-job keys", and its Still-in-force line, which adds the rust-gpu build and R-656's bounded target cache, "and until then no target directory is cached."
- **The question:** does item 3 add the kernel's output to R-285's list?
- **Options seen:** (a) yes: R-285 amended to list it; (b) no: the kernel's output is shared as an artifact within a run, as candidate 2 has it, not cached; (c) another, stated by the owner.
- **Applied meanwhile:** nothing: no kernel-output cache is added by this port.
- **Waits:** candidate 3's pull request.

## RQ-700: A warped virial axis with a range off 0 at a held `L_z ≠ 0`: where its bottom row sits *(physics; silence: the owner's ruling of 4 Nov 2026 on RQ-683; R-506 W2; context: TASK-M2-37)*

*Queued 4 Nov 2026 from the physics review (5775188369, E1) of PR #387, by the orchestrator's decision. For the owner:
it is physics; the two options draw different systems.*

- **File, section:**
  - the owner's ruling of 4 Nov 2026 on RQ-678 and RQ-683: "The floor uses the pixel's own held L_z, additive form K =
    L_z²/2I + K_max·t^γ", and "RQ-683 (#387): same rule for a single energy axis at held spin."
  - `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid", the
    virial ratio's paragraph: "Warped, it runs over the range's feasible part `[Q₀, Q_hi]`, `Q₀ = max(Q_lo, 0)`:
    `Q(t) = Q₀ + (Q_hi − Q₀)·t^γ_K`".
- **The silence:** the ruling's line starts at 0, but a virial axis keeps its asked range, so `Q₀ > 0` is possible. The
  ruling and the corpus do not say how the floor sits under a range off 0. At `Q₀ = 0` both options below agree, and
  both keep every pixel a system.
- **The question:** on a warped virial axis with `Q₀ > 0` at a held `L_z ≠ 0`, what is the pixel's ratio?
- **Options seen:** with `Q_min(U) = L_z²/(I·|U|)`, (a) as built, `K* = L_z²/2I + Q(t)·|U|/2`, the pixel's ratio
  `Q(t) + Q_min(U)`, so the asked bottom `Q₀` is never drawn, even where it is feasible; (b) the floor under the rise
  only, the ratio `max(Q₀, Q_min(U)) + (Q_hi − Q₀)·t^γ_K`, the bottom row at `Q₀` where `Q₀` is feasible and on the
  floor elsewhere.
- **Applied meanwhile:** (a), the ruling's literal form, recorded in chart_reference §2.1 (PR #387).
- **Waits:** nothing: RQ-682 keeps the virial axis off every builder chart.

## RQ-701: Whether and how a `MixedEnergy` chart's held `L_z` or `K` tilts and slices: a hidden coordinate under R-633 that nothing builds or queues *(physics; silence: R-633; the owner's ruling of 4 Nov 2026 on RQ-678 and RQ-683; chart_decoder_contract § "Tilt (basis edit)"; TASK-M2-52's six charts; context: TASK-M2-37)*

*Queued 5 Nov 2026 by TASK-M2-37's fix round, on the orchestrator's decision (applied per R-369), from code review
5778057939 (F1) and physics review 5778075140 (F4) of PR #387; rewritten from code review 5779819090 (F2) and physics
review 5779878848 (F1), which found the first entry short of the contract's clause and of the ruling's lines. For the
owner: yes, it is physics; the tilt changes which systems the chart shows.*

- **File, section:**
  - `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)", "Charts that hold a block by a
    construction": "where it holds a block by a physical construction, its hidden coordinates are that construction's
    physical parameters, tilted and sliced by value as the shape sphere's `p_λ` targets are, `d` added to the physical
    value at the slice centre". `MixedEnergy` holds the momentum block by chart_reference §2.2's construction, its
    `held` the parameter.
  - The owner's ruling of 4 Nov 2026 on RQ-678, the warped mode: "Held energy-type hidden coordinates (R-633 c2) are
    tilted in their warp coordinate in this mode, so held K stays ≥ 0. This amends R-633's "by value" for warped mode
    only."
  - The same ruling, the linear mode: "Rows keep fixed physical values; infeasible cells are "not a system", blank per
    R-470; held coordinates tilt by value."
  - The same ruling, on RQ-683: "RQ-683 (#387): same rule for a single energy axis at held spin."
  - The same ruling, its last line: "If the warped-coordinate tilt of held K turns out not to be well-defined
    somewhere, come back with the case."
  - `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps", the hidden coordinates' table,
    which names only: "`(ν, K)`, `Φ_θ,K` (and `Φ_strip,K`, the same map) | the held `L_z` (0 untilted), a physical
    value" and "`Φ_θ,L_z` | the held `K`, a physical value".
  - `plan/tasks/M2/TASK-M2-52.md` § "Deliverables": "The six charts' hidden coordinates … the held `L_z` of `(ν, K)`
    and `Φ_θ,K`; the held `K` of `Φ_θ,L_z`."
  - `crates/kernel/src/chart/physical.rs`, `MixedEnergy`'s `top`: "`K_max` on `K`, `KE` and `E`, `Q_hi`, the top of the
    asked range, on the virial ratio; not read on `L_z`."
- **The silence:** the principle makes a lone-axis chart's held `L_z`, or the held `K` on an `L_z` axis, a hidden
  coordinate, but the table and TASK-M2-52's six charts do not include `MixedEnergy`. So its tilt and its slice are
  neither built nor queued, and `Hidden::hidden` on the chart lists only the frozen controls. The ruling reaches a
  lone axis through its RQ-683 line. In the linear mode that settles the form: the held value tilts by value, and the
  contract's clause slices it by value too; only the build is missing. In the warped mode the ruling amends "by value"
  for the tilt and does not say what becomes of the slice. Its warp coordinate is not well-defined on these charts as
  they stand, which is the case its last line asks to have brought back:
  - **Held `L_z`, on a `K`, `KE`, `E` or virial axis.** `L_z`'s warp coordinate, `(2s − 1)·L_max(K)`, reads a held `K`,
    and these charts hold none: `K` is what the pixel's axis sets. That rules out that one coordinate, not a tilt: the
    additive floor takes any real `L_z`. The ruling's reason, "so held K stays ≥ 0", does not speak of `L_z`.
  - **Held `K`, on an `L_z` axis.** The warp coordinate, `K = K_max·t^γ_K`, `t₀ = (K₀/K_max)^(1/γ_K)`, is well-formed,
    and is the one the ruling's reason names. It needs a `K_max` this chart does not read: a missing value (R-71), not
    a missing form. `top` is the field that would hold it.

  These two concern different axes, so no one form has to meet both.
- **The question:** in the warped mode, how does a tilt move the held `L_z` on a lone `K`, `KE`, `E` or virial axis,
  and the held `K` on a lone `L_z` axis; and does a slice of the held value move the same way, or by value? In the
  linear mode the form is the ruling's; only the build is open, and it waits on the warped answer so that both arms
  come in together.
- **Options seen, held `L_z`:**
  - (a) By value on the floor: `L_z = L_z₀ + d`, §2.1's floor at the pixel's own tilted `L_z`. Every pixel stays a
    system and no `L_max` is needed. Cost: the ruling's warp sentence then reaches held `K` only, and `|L_z|` has no
    bound but the slider's span.
  - (b) A warp coordinate on a fixed scale, `L_z = (2s − 1)·√(2·I·K_max)`, `s` bounded. Cost: a scale §2.1 does not
    write for these charts, which `K`, `KE` and `E` can read through `K_max` and the virial axis cannot, its `top`
    being `Q_hi`; and `s` needs a rule past its edges.
  - (c) Another, stated by the owner.

  The ruling's port queues the same question and options (a) to (c) for `(ν, K)`, `Φ_θ,K` and `Φ_strip,K`; the answer
  there can be this one.
- **Options seen, held `K`:**
  - (a) The warp line, `K = K_max·t^γ_K`, with `K_max` supplied to the chart. Cost: a value this chart does not hold,
    a calibration requirement under R-71 with REQ-CHART-044's placeholder meanwhile.
  - (b) Another, stated by the owner.
- **Slicing, in every option above:** by value, the contract's clause as it stands, or in the tilt's own coordinate.
  The two agree at an offset of 0 and write different held values away from it.
- **Applied meanwhile:** the held value is neither tilted nor sliced: the chart's hidden coordinates stay its frozen
  controls, and chart_reference §2.1 says so (PR #387). `d = 0` reproduces the untilted chart in every option.
- **Waits:** a tilt or a slice of the held value on a lone-axis chart; nothing else.
- **5 Nov 2026 (R-720, RQ-699 (a)):** the owner has since ruled the same question on `(ν, K)`, `Φ_θ,K` and `Φ_strip,K`: "L_z = L_z0 + d", "R-708's "tilt in the warp coordinate" was meant for held K (which must stay >= 0) and is narrowed to held K only", and "The same rule governs slicing the held L_z as well as tilting it." R-720 names those charts, so it doesn't close this entry, but what it implies here is held `L_z` option (a), tilted and sliced by value. The cost option (a) lists, the warp sentence reaching held `K` only, is now the owner's own reading. Held `K`'s `K_max` on a lone `L_z` axis is still open; its slice is by value (R-720's list).
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

## RQ-708: Whether the running kernel reaches the ionisation gate only after `E_rel > 0`, or the gate fires on its own *(physics; silence: R-29, R-32, R-710; TASK-M3-12 § Goal; `docs/contracts/principia_integrator_contract.md` Part 7; context: physics review 5801941254 of PR #425)*

*Queued 6 Nov 2026 by the port's fix round, from physics review 5801941254 of PR #425, finding N1. For the owner: yes, it is physics; the answer changes when a triple ejection is recorded. Nothing is chosen.*

**In plain English:** a triple ejection is reported as an escape. The task says an escape needs one body unbound from the other two, and then calls a triple ejection a kind of escape. The older ruling lists the triple ejection's own three conditions and doesn't mention that test. So the same run can end at two different times. The label comes out the same; the time it ends, and everything frozen at that moment, does not.

- **File, section:**
  - `plan/tasks/M3/TASK-M3-12.md` § Goal: "Escape is detected post-step on the projected state iff `|Δn̂|` over the 0.4-time-unit window `< tau` AND `E_rel > 0` — two conditions, no receding test, no `r_esc`, no `k_esc`", then "Triple ejection is escape with `detail = 3`, gated on all pairwise energies positive, all separations growing and total `E > 0` read from the stored `E_0`".
  - `decisions.md` § "R-32 — …": "All three pairwise energies > 0, all separations growing, total `E > 0`, evaluated with the escape rule's settling."
  - `docs/contracts/principia_integrator_contract.md` Part 7: "`ESCAPE ⟺ |Δn̂| over a window < tau AND E_rel > 0`."
  - `docs/design/principia_dd_integrator.md`:223: "Triple ejection is `ESCAPE` with `detail = 3`; its gate is ruled by R-32, applied later."
- **The silence:** the first reading has the gate classify a firing that `E_rel > 0` has already allowed. The second has the gate fire on its own, R-32 naming the escape rule's settling and not its `E_rel` test. No file says which. The gate isn't built: no file in `crates/kernel/src` names it. R-710's predicate test holds under either answer.
- **The case that tells them apart:** the equilateral family, unit masses, side `r`, each moving radially outward at speed `u`, with `2/r < u² < 3.08/r`. Every pair is unbound, every separation grows, `E > 0` and `n̂` is settled, while `E_rel < 0` for every body. Example: `u² = 2.5/r₀` gives `E = 0.75/r₀` and `E_rel = 0.75E − 1.214/r(t)`, which turns positive only at `r ≈ 2.16 r₀`. Behind `E_rel > 0` the `detail = 3` firing waits until then; on its own it fires a window after the start. The label agrees, but `t_end` and all that freezes at `t_esc` differ. Only `E > 0` states are affected, so the 8D chart and not rest starts.
- **The question:** does the running kernel reach the ionisation gate only after `E_rel > 0`, or does the gate fire on its own?
- **Options seen:**
  - (a) Behind `E_rel > 0`: the escape rule fires first, as usual, and the gate then says whether the escape is a triple ejection. The firing waits until one body is unbound from the other two.
  - (b) On its own: the gate is a second way to fire, with the escape rule's settling and its own three conditions. The firing comes as soon as those hold, a window after the start in the case above.
  - (c) Another, stated by the owner.
- **Applied meanwhile:** nothing.
- **Waits:** TASK-M3-12's gate (REQ-EVT-006).

## RQ-709: Whether R-791's Newtonian energy reaches the other axes that read `U`: the lone, mixed and Chart-builder `E` axes and the virial-ratio axis *(physics; silence; R-791, R-670, R-654; `docs/design/principia_chart_reference.md` § 2.1; `crates/kernel/src/chart/energy.rs`:303; context: TASK-M9-39, REQ-CHART-092)*

*Queued 6 Nov 2026 from physics review 5805354224 of PR #430, F1. For the owner: yes, it changes results: which start such a chart's pixels decode to at `ε ≠ 0`, or whether those charts run softened at all. Nothing is chosen.*

**In plain English:** the owner ruled that the chart of angular momentum against energy keeps its energy axis Newtonian once softening is on, so a pixel's starting state never depends on the softening length. That ruling names one chart. Other axes read the same potential: an energy axis on its own, on a mixed chart or from the Chart builder, and the virial-ratio axis. As the plan stands, softening would reach them, so their pixels' starts would depend on the softening length, which is what the owner's reason forbids for the first chart. Nothing has run softened yet, because softened runs are refused until the end of the build.

- **File, section, quoted:**
  - `decisions.md` § "R-791 — …", the owner's words: "Its energy axis is the Newtonian energy, so a pixel's starting state never depends on the softening length and toggling softening changes only the physics, never the start."
  - `docs/design/principia_chart_reference.md` § 2.1, "Every energy-type axis is warped by default (R-471)": "`E = U + K_max·t^γ_K`"; and the virial ratio: "The kinetic energy at the pixel is `K* = Q(t)·|U|/2`, using the pixel's own `U` (E3)."
  - `crates/kernel/src/chart/energy.rs`:303, `potential`: "The potential `U` of the masses and configuration, `G = 1` (the descriptor's `V_0`)."
  - `crates/kernel/src/chart/physical.rs`:10: "`E` ([`AxisE`]): `E = U + K*`, that `K*`, `U` the pixel's own potential (E3)."; :14: "`K* = L_z²/2I + Q·|U|/2` on the pixel's own `U`".
  - `docs/design/principia_dd_decoder.md` § 3.6, dated 2 Nov 2026: "At `ε ≠ 0`, `V₀ = − Σ_{i<j} mᵢmⱼ / √(‖rᵢ − rⱼ‖² + ε²)`".
- **The silence:** R-670's refusal covers `Some(E*)` only, and R-791, REQ-CHART-092 and TASK-M9-39 name only `(L_z, E)`. These axes share `potential` with it, and TASK-M9-39 softens that path. So at `ε ≠ 0` their pixels get a start that depends on `ε`, with no label and no readout. The virial axis is the harder case, because R-670 makes the stored `virial_ratio` softened, so a Newtonian axis would disagree with its own readout.
- **The question:** at `ε ≠ 0`, what `U` do these axes read, and do their charts run?
- **Options seen:** (a) Newtonian on every axis that reads `U`, each labelled, with the softened value beside it in the Inspector, as R-791: no start depends on `ε`, and the virial axis differs from the stored `virial_ratio`; (b) these charts refuse `ε ≠ 0` at the view, as with `Some(E*)`: nothing disagrees, and they can't be viewed softened; (c) softened on these axes: each axis is the run's true value, and the start depends on `ε`, against R-791's reason; (d) the `E` axes as (a) and the virial axis as (b); (e) another, stated by the owner.
- **Recommendation:** (a): one rule for every axis that reads `U`, with the owner's reason kept and no chart lost. The virial axis's disagreement is the same one R-791 already labels on the energy axis.
- **Applied meanwhile:** none needed: dispatch refuses `ε ≠ 0` (R-654; TASK-M3-06), so no softened run exists.
- **Waits:** TASK-M9-39 (REQ-INT-111, REQ-SYS-094, REQ-SYS-095). Nothing in M0 to M8 waits.

## RQ-712: Does "a new axis kind" mean a fifth kind, and how far does R-798's "no new chart type" reach? *(conflict: the order against the contract; a one-way door, order item 7; context: all of the plan)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner's order says new abilities arrive as new "axis kinds"; the contract says there are exactly four kinds and that the list is closed. And the ruling that said "no new chart type" was about one paper's experiment. This asks whether it now governs everything.*

- **File, section:**
  - the owner's order of 6 Nov 2026, item 1: "A new capability is a new axis kind, warp or pair map in the builder, not a new chart type (R-798)."
  - `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set": "Burrau introduces no fifth kind." REQ-CHART-004: "Chart axes must be one of the closed set of four kinds".
  - `decisions.md` § R-798, the owner's words: "RQ-594: (a), a tested remap onto the shape sphere at rest. Also add a shape-chart preset … No new chart type." What it decides: "**No new chart type.**", under PA-1's heading.
  - `decisions.md` § R-669: "The Šuvakov-plane chart, `SuvakovPlane`, … is defined (R-72) and built by one M9 task"; REQ-CHART-091: "built as one chart … with its row in the lowering appendix and its baked compute variant".
- **The conflict:** read literally, the order opens the set the contract closes, and takes R-798's sentence beyond PA-1, where R-669 has already ruled a new chart.
- **The question:** is a new capability a new *entry* under the four kinds, and does "no new chart type" hold from here on, generally?
- **Options:**
  - (a) **Recommended.** Four kinds stay. A primitive is a registry entry under one of them; the design finds every primitive fits (§ 3), the virial ratio by RQ-717's answer. "No new chart type" is the rule from now on, for anything not already ruled. `SuvakovPlane` stands as R-669 ruled it, and it can be a baked pair inside the cap.
  - (b) As (a), but `SuvakovPlane` is rebuilt as a builder preset.
  - (c) A fifth kind, "registered", with the contract and REQ-CHART-004 changed.
- **Applied meanwhile:** (a).
- **Owner, 6 Nov:** no fifth axis kind is agreed in principle. That is not a ruling; this entry stays open.

## RQ-713: Is a side-ratio axis in the builder the `SideRatio` chart R-798 refused? *(conflict: the order against R-798; a one-way door; context: TASK-M9-45, REQ-CHART-093)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner ruled that one paper's picture runs by converting side ratios onto the existing shape chart, with no side-ratio chart. The new order lists side ratios as a thing an axis can be. This asks whether both can stand.*

- **File, section:**
  - `decisions.md` § R-798: "**The remap.** PA-1 runs on the shape sphere at rest, reached through a tested remap of side ratios to shape coordinates (rule 6). There is no `SideRatio` chart."
  - the owner's order, item 3: "Each primitive (interior angles, side ratios, Jacobi coordinates, mass ratios, E, Lz, K, virial ratio, latent components, ...) declares".
- **The conflict:** an axis pair of side ratios, offered in the builder, draws what the ruling said does not exist.
- **Options:**
  - (a) **Recommended.** Both stand. PA-1 keeps its remap and TASK-M9-45 is untouched. The builder offers the axis, a registered closed pair (design § 8.3), which is an option in the builder and not a chart type; the same closed form serves as the remap, one source.
  - (b) R-798 wins outright: side ratios are registered forward-only, for the Inspector and contours; no such axis.
- **Applied meanwhile:** the ruling wins, so nothing about PA-1 changes; the axis is listed, and its task (TASK-M4-34) waits on this.

## RQ-714: Region D's definition as a chart *(a one-way door, order item 7; physics: changes what is drawn and counted; R-592; dd_validation_orbits §1.6; context: TASK-M4-06, REQ-RENDER-082, M9's targets)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: region D is the small curved triangle of starting shapes twenty M9 targets use. Its definition as a set of points is settled; what it is as a picture is not: what happens to the pixels outside it, which velocities it starts from, and how a paper's fixed time limit reads when each pixel has been rescaled differently.*

- **Reach (physics note 2, P11):** D is a fundamental domain only at equal masses and `L_z = 0`. At unequal masses the outside is other systems (RQ-728), and under a held `L* ≠ 0` the mirror is lost, so "blank and uncounted" as agreed in principle reaches that case only.
- **File, section:**
  - `docs/design/principia_dd_validation_orbits.md` § "1.6": "**Region D** (I, Fig. 1 and eq. 8): `ξ ≥ 0`, `η ≥ 0`, `(ξ + ½)² + η² ≤ 1`. A point outside it is not a region-D start, and the import refuses it."
  - `docs/contracts/principia_lowering_contract.md`, Appendix, the Anosova row: "physical (m,r,p) construction → **canonicalise entry** → standard pipeline | region D bounds".
  - `docs/contracts/principia_chart_decoder_contract.md` Part 3: "**Inside `[0,1]²` every gallery chart is valid by construction** … so a pixel hatched there is a decoder bug". And `decisions.md` § R-506: "warped to its feasible range by default".
  - `decisions.md` § R-554: "inside `[0,1]²` such a pixel is the domain's end by design … it draws blank and logs no decoder-bug warning", the exponential map's exception.
  - `docs/design/principia_dd_replication.md` § 5.8: "AA-3 | … the Anosova physical-frame entry" with no velocity stated; "AA-2 | … eq. (3) velocities".
- **The silences, four:**
  1. D is not a rectangle, so a square chart either masks, shows the outside, or warps. The outside pixels are real systems: relabelled copies at equal masses and `L_z = 0`, other systems otherwise (the Reach line above).
  2. A mask makes pixels inside the square that are not drawn, which the contract calls a decoder bug on a gallery chart.
  3. AA-3 and AA-5 don't say which velocities.
  4. The kernel decodes at `I = 1`, so each pixel has its own rescale `ℓ`, 0.408 to 0.577 over D at equal masses, and one sim-key `T` is a paper time varying by a factor of 1.68 across the chart there; other masses differ, and `T_c`'s `ℓ_min` is taken at the run's masses.
- **Options on (1), (2):**
  - (a) **Recommended.** A mask. Axes `ξ ∈ [0, ½]`, `η ∈ [0, √3/2]`, linear, a preset of the general third-body pair; outside eq. 8 is the domain's end, blank, out of the count and the total, no warning, R-554's exception extended by name. Pixels stay uniform in `(ξ, η)`, as the papers sample, and the picture is theirs. R-506's warp reads as trivial here: every `(ξ, η)` is a system, so the range is its whole range.
  - (b) The plane, unmasked. D drawn as a contour; statistics filter. No exception needed; the Atlas shows relabelled copies at equal masses and `L_z = 0`, other systems otherwise.
  - (c) A warp onto the square, polar about the left body. Every pixel in D, no exception, R-506 read literally; the measure is not the papers' and the apex is a ray-degenerate edge.
- **Options on (3):** (a) **recommended**, AA-3 and AA-5 read paper I's own runs, so eq. 3; (b) exact rest.
- **Options on (4):** (a) **recommended**, the run reaches the longest pixel's horizon and the comparison cuts each pixel at its own, a tested function (rule 11), the kernel untouched; (b) a per-pixel `T`, a kernel change.
- **Applied meanwhile:** nothing built. The design's § 8 writes (a) in each case.
- **Owner, 6 Nov:** outside region D blank and uncounted is agreed in principle. That is not a ruling; this entry stays open.
- **Waits:** the design's tasks behind TASK-M4-37, and no existing task. TASK-M4-06's golden (REQ-RENDER-082) needs this domain, so its edge to TASK-M4-34 is applied when this is ruled.

## RQ-715: Hold `L_z`, and how it meets `η_E` *(physics, a one-way door; §0.6; R-25, R-670, R-720; closes the held-`L_z` half of RQ-701 only; context: TASK-M8-118)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: a new option holds a chart's spin fixed, as energy normalisation holds energy. Today's energy step scales every velocity, which would change the spin, so when both are on the step has to work differently. That's new physics in the decode, so it's the owner's to rule, though it's off by default. A physics reviewer has checked it: the physics is right, and the formula needs care to be built.*

- **File, section:**
  - `docs/design/principia_chart_reference.md` § "0.6": "`η_E = √((E* − U)/K0)   pᵢ ← η_E·pᵢ`".
  - `docs/contracts/principia_inverse_encode_contract.md` Part 5: "rigid rotation `v^(L) = (L_z/I)·J r` (the *minimal-kinetic-energy* realisation of `L_z`".
  - `docs/contracts/principia_chart_decoder_contract.md`, the curve `v(s) = v⁽ᴸ⁾ + eˢ·(v − v⁽ᴸ⁾)`, "along which `L_z` is exactly constant" (REQ-CHART-051).
  - the owner's order, item 4: "hold Lz fixed on any chart, alongside energy normalisation, with the same refusal rules where Lz is itself an axis".
  - `decisions.md` § R-720: "The held `L_z` of `(ν, K)`, `Φ_θ,K` and `Φ_strip,K` moves by value", and "The same rule governs slicing the held L_z as well as tilting it."
  - RQ-701 (open, queue): "Whether and how a `MixedEnergy` chart's held `L_z` or `K` tilts and slices".
  - PR #422, physics review 5803342655, the note this entry follows; script `docs/incoming/2026-11-06_chart_composition/checks/hold.py`.
- **The silence:** no corpus text says how the two projections compose, what their outcomes are, or what a tilt does to a held `L*` outside R-720's three charts.
- **Options:**
  - (a) **Recommended, amended per the physics review.** Design § 5.3: hold `L_z` adds the rigid rotation; with both held, the scale takes only the part orthogonal to it, `η = √((E* − U − L*²/2I)/K_δ)`, the unique nearest point of the fibre, closed form, and the corpus's own decomposition; off, bitwise today's. Conditions: `K_δ` computed from `δv`, with the second projection (F1, F2), the outcome order (F3) and encode's held decode (F7) written as definitions before the build task (REQ-CHART-108 to REQ-CHART-110); by value inside a chart that holds `L_z` by construction, and no tilt target on a free chart (F4); the wider `ray-degenerate` rank loss (F5); RQ-722 put to the owner (F6).
  - (b) As (a), but both held is refused: hold one at a time. **Not recommended:** the order says "alongside energy normalisation", and only genuine contradictions may be refused. The fibre is non-empty wherever `2K* ≥ L*²`, so this is reduced capability at most, and that's a label.
  - (c) Hold `L_z` solves in the hidden momentum directions, keeping any swept momentum axis's label true. **Not recommended:** it isn't really a solve. Alone it is `Δv ∝ P Jr`, closed form, equal to (a) when no momentum axis is swept, but the hold then depends on which axes are swept, so a tilt changes the IC even at fixed `v`. It fails where `P Jr = 0`, exactly (a)'s rank-loss set, a label turned into a failure, and both held loses further pixels since `E*` has to be met in a smaller subspace. All that to keep one label true.
- **Applied meanwhile:** (a) amended, unbuilt.

## RQ-716: The axis cap, proposed at 4 *(a one-way door, order items 5 and 7; REQ-PERF-006, REQ-PERF-007; lowering Part 4; the start-up object count is issue #427, not this item)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: every "axis pair" compiled into the GPU code makes the one compiled module bigger, and every pipeline parses the whole module. The owner asked for a limit on those pairs, suggesting about 4, with the measured cost. It measures at 4 passing and 8 failing on the Mac and on the Windows machine, which agrees. The limit is a one-way door because plans and charts will lean on it. One new question: start-up on the Windows machine already takes 2.7 s with no pairs at all, over the 2 s this design set, so the owner has to say whether that limit was only ever the Mac's. Whether twelve start-up objects against the contract's ten is right is a separate issue, #427.*

- **File, section:**
  - the owner's order of 6 Nov 2026, item 5: "Cap the number of axis pairs baked into GPU code (propose a number with the measured cost; the owner suggested about 4)."; item 7: "One-way doors (new extension types, any physics or default change, the axis cap, R-798's reading, region D's definition) go to /rulings-report".
  - `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule": "**Chart switching must be instant** → the compute variant is already built (SPIR-V); create its pipeline object for the *active* (occupant, tier, links) at startup — ~10 objects."
  - REQ-PERF-006: "pipeline objects are created from pre-built SPIR-V, eagerly for the active (occupant, tier, links) of every chart at startup and lazily inside the re-integration cost for occupant/tier/link/threshold changes."
  - REQ-PERF-007: "The build-time monomorphisation set must be bounded to the active/plausible variant set, not the full cross-product."
  - `docs/design/principia_chart_composition.md` § 11, the criterion and the measurement.
- **The measurement (criterion written first):** first run, a heavy stand-in: 4 passes, 8 fails. Re-measure, the real region-D pair, interleaved: 4 pairs: module ratio 1.24, one creation 1.23, eager set 1.59 s, gallery 1.23, all within limits; 8 pairs: module 1.48, gallery 1.47, eager 2.35 s, all fail. A pair costs 60 KB of WGSL, so a fifth fails the module test at 1.30: 4 is the ceiling. With the integrator inlined, one pair across 4 occupants fails at 1.43, so the cap holds only with decode as its own entry. That run: one Mac, Metal only, under load.
- **The Windows Vulkan run (criterion unchanged; RTX 2080 SUPER, run 42270588241, 6543782c, 12 arms; byte-identical modules):**

  | `P` | gallery ratio, Mac / Windows | build ratio, Windows | eager `12 + P`, Mac / Windows | start-up (12 gallery), Mac / Windows |
  |---|---|---|---|---|
  | 0 | | | 0.96 s / 2.70 s | 0.96 s / 2.73 s |
  | 1 | 1.06 / 1.05 | 1.06 | 1.10 s / 3.05 s | 1.02 s / 2.87 s |
  | 4 | 1.23 / 1.24 | 1.24 | 1.59 s / 4.40 s | 1.18 s / 3.39 s |
  | 8 | 1.47 / 1.49 | 1.49 | 2.35 s / 6.74 s | 1.41 s / 4.08 s |

  Module ratios are the same exact 1.06, 1.24, 1.48. On every test that moves with `P` the machines agree: 4 passes, 8 fails, so the lower-number rule picks nothing. C1, C2 and C3′ all sit at the limit at 4; C3′'s first repeat alone reads 1.251. **C3's 2 s fails on Windows at every `P`, `P = 0` included.** C3 as written says "on this Mac"; the re-measure also says a criterion isn't relaxed for either machine. On the first reading 4 passes; on the second no count passes and the criterion gives no number. Neither gives a number other than 4.
- **Cautions on that run:** a measurement only, gating nothing (R-649). The owner reports the card may have faulty cores, so the times are one possibly faulty card's; sizes don't depend on it. The first arm ran cold, so compare eager minus gallery within an arm: per pair 176, 246 and 330 ms at `P` = 1, 4, 8, near one gallery pipeline's 239, 282 and 340 ms, and about nothing at `P = 0`. No cache is hiding a pair's cost.
- **The link to #427:** pairs made eager fail the 2 s test, so they are created on first use, which reads REQ-PERF-006's "every chart at startup" as not reaching a composed chart. #427 owns the eager set; if it changes, this is re-read.
- **The questions:** what is the cap, and does it count entries or pairs? Are pair entries created on first use? Does C3's 2 s apply on Vulkan?
- **Options:**
  - (a) **Recommended.** 4 baked entries beyond the gallery, counted as entries (a pair under two link sets is two), decode-only, created on first use; the first use of a pair may take a compile, about 0.25 s to 1 s measured under load on the Mac and about 0.37 s on Windows, and that is accepted as an exception to "every chart at startup". Past the cap a pair runs as CPU batch, labelled. `SuvakovPlane` counts as one of the 4.
  - (b) 3 plus a generic blend entry, if RQ-721 adopts live paths: the blend takes the fourth slot of the module budget. Same module size as (a), a narrower reading of "pairs".
  - (c) 4 eager: every pair at startup, REQ-PERF-006 untouched; start-up on the Mac rises from about 1 s to about 1.6 s and on Windows from 2.7 s to 4.4 s, and #427's gap widens.
  - (d) A cap on entries by a module-size budget (1.25 × today's WGSL) rather than a count: more honest to the cost, harder to read in a task.
- **On the 2 s, recommended:** read it as written, the Mac's number, keep it unchanged, and let #427 set any Vulkan start-up budget, since the gallery fails it with no pairs and the cap can't cure that. This reading is the one under which the proposal passes, so it's flagged as such. If the 2 s applies on Windows, no cap passes, and the gallery itself would have to shrink or go lazy.
- **Applied meanwhile:** (a), with the value in one ledger constant, so a different ruling changes one number and the test `baked_cap`.

## RQ-717: Which kind the virial-ratio axis is *(RQ-682, open in the queue; R-471 H3; a one-way door if ruled as a new kind; context: TASK-M8-119, TASK-M8-18)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the order's second customer is the virial ratio, which compares kinetic and potential energy. The kernel's code for it is written on the warp branch, but the builder can't take it until someone says which of the four kinds it is. That question is already queued, and at a held `L_z ≠ 0` RQ-700 also decides what a row of it means.*

- **File, section:** RQ-682, open in the queue, whose options are (a) kind 3, (b) kind 2 in the momenta, (c) another. `decisions.md` § R-471 H3: "The virial ratio and the mass ratio are not kind 3, so B1 stands for them."
- **Recommendation:** RQ-682 (a). H3's sentence is about B1's extension, which R-506 has superseded; `Q` solves into the momenta and reads the upstream blocks, what kind 3 is.
- **Applied meanwhile:** nothing; the design's virial rows say "by RQ-682".

## RQ-718: What R-669's "no chart" means once any axis pairs with any other *(conflict: the order against R-669; a one-way door; context: TS-2, TR-1, MK-1, SL-1)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner ruled that two particular charts won't be built. The new builder could reach similar pictures by other routes. This asks that the ruling isn't reversed by accident.*

- **File, section:**
  - `decisions.md` § R-669: "no binary-plus-single chart is built, and each reads "not scored: no chart""; "TS-2 (REQ-VAL-326) reads "not scored: no fixed-k velocity chart"".
  - the owner's order: "Any axis may be paired with any other, of any kind".
- **The nearest neighbour (physics note 2):** region D's rest arm under hold `L*` is `(L*/I)·Jr`, a rigid rotation at constant `L`, not constant `k`. It isn't R-669's fixed-`k` chart, and it lands close: RQ-725. Elliptic Lagrange's periapsis start is rigid rotation too, `Q = 1 + e`, on one shape, so not R-669's plane (physics note 3).
- **The conflict:** the registry can leave out binary phase and eccentricity, but `(p_λ,x, p_λ,y)` over a frozen configuration, today's `Mixed`, is a velocity plane, and a user could set it to look like TS-2's.
- **Options:**
  - (a) **Recommended.** R-669 stands: no primitive named for those planes is registered, and a test says so. A user-built plane is a user's chart: it is not the target's chart, it is never scored, and the target's row keeps its words.
  - (b) A named refusal for those planes. Hard to define, since a plane has no name.
- **Applied meanwhile:** (a).

## RQ-719: CPU batch against "not a pre-decoded IC buffer" *(conflict: the order against core_design seam 3; a one-way door; context: TASK-M8-102)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the design rule says each pixel's start is computed on the GPU, since a CPU list couldn't keep up with panning. The owner's order runs numerically solved charts on the CPU. They agree if a batch view isn't live, and this asks for that to be written down.*

- **File, section:**
  - `docs/design/principia_core_design.md` § "3. Kernel eats chart params, decodes in-kernel": "*Not* a pre-decoded IC buffer — pan/zoom/tilt change every pixel's IC every frame, so precompute is impossible at scale."
  - the owner's order, item 3: "numerical (CPU batch only, never in-kernel)".
  - `decisions.md` § R-669: "BZ-1C's starts reach the f32 kernel as a Path B batch, each f64 start rounded once to f32".
- **Options:**
  - (a) **Recommended.** Seam 3 holds for live views. A labelled CPU-batch view is the stated exception, writing the IC buffer the decode stage writes today; a gesture never reaches it.
  - (b) No GPU run at all for such a chart: CPU integrator, f64. Honest, but slow at any useful size.
- **Applied meanwhile:** (a).

## RQ-720: R-594 item 5 against the general options *(conflict: the order against R-594; context: TASK-M8-100, TASK-M8-101)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: an earlier rule said not to build a chart before a target needs it. The order asks for general options, polar and curved slices, that no target needs yet.*

- **File, section:**
  - `decisions.md` § R-594: "a target that needs a chart that doesn't exist waits for that chart. Do not build charts ahead of it."
  - the owner's order, item 4: "polar and log as general options"; "curved slices off the shape sphere (momentum, mass)".
- **Options:**
  - (a) **Recommended.** R-594 item 5 governs charts built *for a target*; builder options are the owner's own order, and none is built for a target.
  - (b) The rule holds: these options wait for a target that needs them.
- **Applied meanwhile:** (a), with the tasks in M8, last, so nothing is built ahead of what the build needs.

## RQ-721: Chart paths: is a path type the owner's generalisation, and do the written rules against morphing give way? *(a one-way door, the owner's consolidated order of 6 Nov; conflicts with export contract Part 3, REQ-TOOL-075, REQ-GUI-374, REQ-GUI-376, REQ-GUI-379, principia_sequence § 3.4; R-520 and R-669 stand; context: TASK-M8-105 to TASK-M8-117, TASK-M8-31, TASK-M8-77)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner wants any chart to turn smoothly into any other, with a morph as a segment of a film instead of a cut, and live like a tilt. The owner set one test first: existing tilt, zoom, lock and Save-view-as-chart must come out bit for bit the same through the general mechanism, or the generalisation isn't adopted. Tested, the single blending formula fails on tilt and on zoom. What passes is narrower: one "path" type with tilt and zoom as branches that run their own code, and morph as another branch. So the owner has to say whether that satisfies the rule. Morphing also contradicts several written rules, which only the owner can change.*

- **What physics says (physics note 2, 6 Nov; a findings note, not a ruling):** the verdict "isn't adopted" stands for the one formula. The path type meets the constraint's letter and not "morph is the general form": two operations pass by not running and two by calling the old code, so the four tests can fail only if dispatch fails, and they say nothing about morph. `Line(A, B_δ)` doesn't tend to the tilt as `δ → 0`, staying 0.016 median off in `n̂`: a tiny extra change to a tilt keyframe switches arms and the mid frames jump. What option (a) adopts is one record, one timing and one rule, passing by construction, not a generalisation. Blocks are right, with `μ`, not `z_μ`. A morph is of addresses, so a mid frame isn't a landscape: a fair reason to keep cut as the default, the owner's call.
- **File, section:**
  - the owner's order of 6 Nov, CHART PATHS: "Morph is the general form and tilt, zoom and chart switch are special cases."; "Constraint: existing tilt, zoom, lock and Save-view-as-chart reproduce bitwise through the general mechanism, a test per operation, or the generalisation isn't adopted."; "pixel -> latent z under chart A and chart B, blended by s in [0,1], then decoded"; "the anchor IC is bitwise fixed at the centre on every frame"; "Live behaves like pan/tilt".
  - `docs/contracts/principia_export_animation_contract.md` § "Part 3 — The keyframe system (retained)": "chart *switches* are cuts or dissolves (two renders composited), never parameter interpolation across chart types."; "No field takes `u` directly and none has a second curve". Part 2 (b) says "the chart morphs".
  - REQ-TOOL-075: "treat chart switches as cuts or dissolves, never parameter interpolation."
  - REQ-GUI-376: "at a chart change, a cut or a dissolve with its length".
  - `docs/design/principia_sequence.md` § 3.3: "A segment whose chart or any link id changes shows a **cut mark** at its second keyframe."; § 3.4: "live playback shows a dissolve as a cut at the second keyframe, as it shows every chart switch".
  - `decisions.md` § R-520, F1: "A changed link id is a cut everywhere, as X7 has it: there is nothing between two links to interpolate."
  - `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)": "Chart-mode switching keeps the anchor at centre"; REQ-GUI-005: "Tilt, zoom and chart-mode switch must preserve the locked anchor exactly".
  - `docs/contracts/principia_inverse_encode_contract.md` Part 1: "`D(E(x)) = C(x)`".
  - `docs/design/principia_chart_composition.md` § 17, the whole design, with its numbers.
- **What can't hold as stated:**
  1. One formula: the blended `z` misses a curved tilt (0.016 median in `n̂`), a general geodesic matches Tip-as-sheet's plane but not its bits (20 of 64 entries), Lean each axis isn't a geodesic, and a zoom through the blend would change the sim key. Bitwise holds only with the arms.
  2. Literal `z`: the shape sphere's lower half and a momentum past `q_max` have no `z`; leaving a lower-hemisphere end jumps by 1.75 median.
  3. The anchor from inverse-encode: `E(D(z)) = z` bitwise on 0 of 20,000; bits hold only from stored words, within one lowering, and only with a locked B that carries them.
  4. "Valid chart": reproducible and decodable, not contract Part 5's well-posedness; a frame folds (32% of pixels at the middle of flat to polar) and takes no statistic.
  5. "On the GPU when both ends are closed-form" narrows to "and baked". "Replaces the cut mark" narrows to what isn't a cut: a saved `cut`, and a changed link id (R-520).
- **The questions:** (1) does the path type meet the constraint, or is the generalisation not adopted? (2) do the written rules give way to a morph? (3) `z` literally, or on blocks? (4) the default for a new chart-change segment? (5) live preview of a morph, against "cuts in live playback"?
- **Options:**
  - (a) **Recommended.** Adopt the framing as a path type: view a point, every edit a path, tilt and zoom as arms running their own code, morph an arm; four tests, one per operation, and if any fails in the build, it stops. State plainly that morph is then not the general form. Change export contract Part 3, REQ-TOOL-075, REQ-GUI-376 and REQ-GUI-379, and § 3.3, 3.4, so a chart change may morph, live and in export. `Path::Line` on blocks. Anchor by stored words. New chart-change segments default to morph where the ends allow, cut and dissolve still offered, old specs untouched. R-520's link-id cut stands; R-669 stands; frames never scored. Mid-morph frames take no baked entry and don't count against the cap.
  - (b) As (a), but keep the rules for live: morph in export only, live playback cuts as today. Smaller, and it drops "live first-class".
  - (c) **Not adopted, by the owner's rule read strictly:** the one-formula generalisation fails, so no path framing. Morph is added only as a third transition of a Sequence segment, export only, beside cut and dissolve, with tilt and zoom untouched.
  - (d) As (a), with literal `z`: lower-hemisphere ends and clamped momenta take a labelled jump at the end frame, and the order's words stand.
- **Recommendation:** (a).
- **Applied meanwhile:** (a), unbuilt. Nothing is implemented until this is answered.

## RQ-722: Does a chart at rest refuse hold `L_z`, or say "not at rest"? *(a choice for the owner beside RQ-715; the order's item 4 and its last block; F6 of the physics review; context: TASK-M4-33, TASK-M8-118)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: some charts are defined as "the bodies start at rest". Holding a non-zero spin on one gives every pixel a spinning start, so the chart stops being what its name says. That's a well-defined set of systems, so strictly nothing contradicts; the question is whether to refuse it anyway, because the chart's identity is at stake, or to allow it with a label.*

- **Beside R-669:** a labelled rest chart under hold `L*` is `(L*/I)·Jr`, a rigid rotation; it isn't the fixed-`k` chart (RQ-725).
- **File, section:**
  - the owner's order, last block: "The only refusals are genuine contradictions (an axis and a constraint on the same quantity; two axes that fix the same quantity; a pair whose joint inverse does not exist)".
  - the owner's order, item 4: "with the same refusal rules where Lz is itself an axis".
  - `docs/design/principia_chart_composition.md` § 6.1, `HoldAgainstConvention`.
  - physics review 5803342655, F6: "`HoldAgainstConvention` … doesn't meet the order's bar as written", and "I lean to the flag, because a chart named for rest that isn't at rest misleads."
- **The facts:** a rest chart under hold `L*` is a well-defined family, each pixel `(L*/I)·Jr`, reachable anyway through the latent chart with `z_q = 0`, which isn't refused. "Rest convention" has no definition today: besides the mass simplex and the Euclid plane, `Φ_strip,δm` (`BurrauThetaDeltaM`) is at rest, as is region D's exact-rest arm, and `forbids_energy_normalisation` is false on all of them, so hold `E` there is a label, and `Some(0)` on a rest chart is a no-op that is refused.
- **Options:**
  - (a) Refuse, by a declared flag on each chart at rest, disclosed as an extra refusal the order's list doesn't name, for the chart's identity. **The physics reviewer leans here.**
  - (b) A label, "not at rest", and no refusal, keeping the order's closed list.
- **Recommendation:** none. The physics reviewer's lean is stated; the design holds no view, and this is the owner's call.
- **Applied meanwhile:** neither: § 6.1 marks the row not settled.

## RQ-723: Inner and outer orbital elements and Mardling–Aarseth: axes, or colours? *(conflict: the owner's order part C against R-668 item 5 and R-669; a one-way door; context: TASK-M8-109, TASK-M8-111, V-11, SL-1, TR-1, MK-1)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner lists orbital elements and a published stability boundary as primitives. Hierarchical systems are parked for a later version by an earlier ruling, and no binary-plus-single chart is built. As chart axes these would reverse those rulings by another name. As colours or contours over a chart they build no chart and reverse nothing. Donnison's variants join them.*

- **File, section:**
  - the owner's order, part C: "inner and outer orbital elements (eccentricities, semi-major axis, phase, periapsis ratio); distance from the Mardling-Aarseth boundary"; "Donnison 1992's (mu, Delta) and (q2/a1, mu*) as variants of the element axes".
  - `decisions.md` § R-669: "no binary-plus-single chart is built, and each reads "not scored: no chart"."
  - `docs/design/principia_dd_replication.md` § 5: "*1 Nov 2026 (R-668, item 5):* hierarchical triples are parked with the v2+ 3D extension."
  - `docs/design/principia_dd_chazy_final_motion.md` § 3.5, the form, and "Not a hierarchy unless `a_in > 0`, `a_out > 0` and `e_out < 1`".
  - `docs/design/principia_chart_composition.md` § 3.1, § 3.2.
- **The findings:** osculating elements exist at `t = 0` for any state by the labelled pair, and mean an orbit only with a clear hierarchy; choosing the pair by nearness adds a seam. Mardling–Aarseth's form **is** in the corpus, so an overlay needs no new source, its domain is its blank. A numerical inverse may exist; what stops it being an axis is R-668's parking, which is enough. *Corrected on physics note 2: 86acb50f said the form was not in the corpus, which was wrong.*
- **Options:**
  - (a) **Recommended.** Forward-only overlay rows for the elements, the periapsis ratio and Mardling–Aarseth, blank off their domain; Donnison's as definition requirements. No axis; nothing scored; R-668 and R-669 stand.
  - (b) As (a), with `e` and `a` as axes at a labelled pair, labelled unscored. It overrides R-668 item 5's reach and needs R-669's words amended.
  - (c) Leave all of them out until the v2+ extension.
- **Applied meanwhile:** (a), unbuilt.

## RQ-724: The Saari split and RQ-680's dilation rows *(RQ-680, open; the owner's order part C; REQ-COL-113, REQ-COL-115, REQ-COL-129; REQ-CHART-051; context: TASK-M8-109, TASK-M7-52, TASK-M7-47)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: an open question asks what the state map's "dilation" targets mean, since no chart defines dilation. The owner's split of kinetic energy into radial, rotational and shape parts gives "dilation" its meaning: the radial part's signed rate. It doesn't by itself give the axis, range and link the open question also needs.*

- **File, section:**
  - RQ-680 (queue): "`(θ, h)`, `h` the scale-free homologous rate `(Σ m r·v)/I · I^¾`", and option (b): "define the two dilation charts (their `h` axis, range and link) and build them as targets".
  - the owner's order, part C: "Saari split of K (radial/dilation, rotational, shape), checked against RQ-680's dilation rows".
  - `docs/design/principia_chart_composition.md` § 3.1.
- **The finding:** `K = K_dil + K_rot + K_shape` holds, `r` and `Jr` orthogonal, to 10⁻¹³, `K_dil = h²/2I`, `K_shape = I|ṅ|²/8`. `Σ m r·v` equals RQ-680's rate only on the `I = 1` gauged state, and SM1 works from the encode of a live state, so the forward must run there or in the scale-free form. SM1 says "never left", so `h` as a target is the whole real line. REQ-COL-129 holds a `t = 0` identity tolerance for `θ × h`, which needs a decode with its residuals stated: `φ`, `L_z`, `K_shape`, masses. `h*` has a closed step keeping `L_z`. `d_E` (REQ-CHART-051) scales `K_dil` and `K_shape` together, no conflict; a `d_h` would be a new named direction.
- **Options:**
  - (a) **Recommended.** RQ-680's option (b): `h` is the registered row in its scale-free form; its range, link and the decode's residuals are a definition (R-72), under physics review; `(θ, h)` and `(ν, h)` state-map targets on CPU batch, no baked entry; a builder axis too.
  - (b) RQ-680's option (a), the targets are the app's charts, and `h` is a builder axis only. It drops REQ-COL-129's two rows, so it's the owner's.
- **Applied meanwhile:** nothing; `h` is listed, and TASK-M7-52 still waits on RQ-680.

## RQ-725: Fixed `k` and symmetric families against R-669 *(the owner's order parts C and E against R-669; a one-way door; context: TASK-M8-111, TASK-M9-40, TS-2, REQ-VAL-326, REQ-CHART-091, RQ-718, RQ-722)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: two listed items sit beside earlier rulings. "Fixed-k" means giving every point a rigid rotation at a chosen ratio of kinetic to potential energy; that is the chart the owner ruled not to build, and its definition is in a paper we hold. The Šuvakov family is already ruled as one chart built by one late task. The neighbours the order lists, such as the third body's speed against escape and Tanikawa & Mikkola's 2023 velocities, aren't those charts.*

- **File, section:**
  - `decisions.md` § R-669: "TS-2 (REQ-VAL-326) reads "not scored: no fixed-k velocity chart""; "The Šuvakov-plane chart, `SuvakovPlane`, at equal masses and at fixed m₃, is defined (R-72) and built by one M9 task, TASK-M9-40".
  - `docs/incoming/2026-10-25_replication/papers/tanikawa_mikkola_2008_arXiv0802.2465.pdf` § 3.1: "we give maximal angular momentum for a given configuration triangle"; § 4.2: "virial coefficient k = T/|U|".
  - the owner's order, part C: "symmetric-IC families: Suvakov-Dmitrasinovic plane (m3 free, and the middle-body-labelled-2 variant), Li & Liao 2017 generalised plane … isosceles choreography (c1..c4), Titov isosceles, Orlov collinear (k, phi) and Mikkola-Hietarinta (R, theta) at fixed E".
- **The findings:** the prescription is the rigid rotation `v = ωJr`, `ω² = 2k|U|/I`, `Q = 2k`, over `D₁₁ ∪ D₁₂`; only its sense isn't printed. A chart of it is what R-669 refuses. `v₂/v_esc` isn't `k`, and the corpus has it: `√chazy_qk[2]`. *Corrected on physics note 2: 86acb50f's premise that the prescription wasn't held was false.* `(α₂, β₁)` isn't `k`. A rest arm under hold `L*` is a rigid rotation at constant `L`, the nearest neighbour, for RQ-718 and RQ-722. Li & Liao 2017's plane is `SuvakovPlane` after `C`. Orlov's `(k, φ)` reads as its polar view at equal masses.
- **Options:**
  - (a) **Recommended.** R-669 stands: no fixed-`k` chart; TS-2 "not scored"; `SuvakovPlane` one M9 task, with Li & Liao's plane and the labelling variant its presets. The third-body ratio, `(α₂, β₁)` and Li–Li–Liao's `(x₁, v₁)` are axes, unscored. A user's rigid-rotation hold is a user's picture under RQ-718. Isosceles families, BHH and Mikkola–Hietarinta take definition requirements (R-72), with no build here.
  - (b) Reverse R-669 for TS-2: build the fixed-`k` chart.
  - (c) A general symmetric-family primitive, with `SuvakovPlane` as its first preset: reverses R-669's "one M9 task".
- **Applied meanwhile:** (a), unbuilt.

## RQ-726: The scattering primitives against R-669's "no binary-plus-single chart" *(conflict: the owner's latest order part C against R-669, closing RQ-593; a one-way door; context: TASK-M8-111, TR-1, MK-1, SL-1, M9-41)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the order lists axes for a binary meeting a single star, and says they feed a binary-plus-single chart. The owner ruled on 1 Nov that no such chart is built. Those axes together are that chart, whatever it's called. The earlier ruling stands until the owner changes it, so nothing is built around it. A stated rule for bringing a star from infinity to a finite start is written down, ready either way.*

- **File, section:**
  - the owner's latest order, part C: "Scattering (binary + single; feeds RQ-593's BinaryPlusSingle chart): v_inf/v_crit …, normalised signed impact parameter b/b_max …, binary phase, orientation angle u …, start-distance ratio d/a with the single at rest …, parabolic encounter … Needs one stated convention: how an "at infinity" state is pulled in to a finite start distance (hyperbolic Kepler), and that distance recorded."
  - `decisions.md` § R-669: "RQ-593's residual, option (c)": "no binary-plus-single chart is built, and each reads "not scored: no chart"."
  - `docs/design/principia_dd_replication.md` § 5.12: "**The binary-plus-single chart** (RQ-593 (c)): not built."
  - `docs/design/principia_chart_composition.md` § 3.2.
- **Options:**
  - (a) **Recommended.** R-669 stands. The scattering rows are definitions and forward-only overlays: nothing with an inverse, no chart, TR-1, MK-1, SL-1 "not scored". The convention stays written.
  - (b) Reverse R-669 on RQ-593: `BinaryPlusSingle` is built as a preset of these rows, on CPU batch, and the three targets score in M9. It takes the convention, with the phase defined at pericentre passage since the flight time from infinity diverges, `b_max`'s definition, and a stated tidal error at `d₀`.
- **Applied meanwhile:** (a), unbuilt.

## RQ-727: Time as an axis: chart mode or fifth kind, and stored rows against "store no history" *(a one-way door; the owner's latest order part D; conflicts with the temporal note, core seam 4 and R-66; agreed in principle: no fifth axis kind; context: TASK-M8-113 to TASK-M8-116)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the owner wants a chart with time up the side: each column is one initial condition run once, and each row a moment. It fits the engine better than expected. Time isn't really a fifth kind of axis, since it says when to read, not what to start from, so it's a mode of the chart; the builder can still list it beside the kinds. The storage is where it hits a rule. Keeping one state per pixel on screen costs what any chart costs. Keeping every step of every column, as "dense output", is the stored history the engine was redesigned to get rid of: 3.8 GB to 76 GB. So we store one row per screen row, and zooming further re-runs from zero and says so. Three more things need saying. Even that amends the note's words. The time range has to come off the cache key, or every time zoom restarts the run, so the caching contract needs an amendment too. And the default integrator can only show time as finely as its fixed sync step, which isn't calibrated yet, so this mode now bears on that value.*

- **File, section:**
  - the owner's latest order, part D: "A chart may take time as one axis, paired with any IC primitive"; "Each column is integrated once and read at each row's time"; "Navigation: zoom/pan in time re-reads stored trajectories (state the memory budget for storing per-column dense output)"; "Chart builder offers time as an axis kind once per chart".
  - `docs/contracts/principia_chart_decoder_contract.md` Part 3, REQ-CHART-004: the four kinds, "a closed set".
  - `docs/design/principia_temporal_architecture_note.md` § "The decision": "**store no history**"; footgun 4: "Viewport cache regrowing into a history buffer".
  - `docs/design/principia_core_design.md` seam 4: "the marching state at the playhead, O(1) in time"; seam 3's rule is on a pre-decoded IC buffer and holds.
  - `docs/contracts/principia_integrator_contract.md`, R-565: "The march stops only on the grid"; Part 3: `dt_macro = max(1e-3, T/65535)`.
  - `docs/gui/principia_render_gui_spec.md` § G2, R-66: "never replays stored frames".
  - `docs/contracts/principia_caching_contract.md`, the invalidation table: "`T`, `dt_macro`, thresholds, `eps` floors | **All sim buffers** (signature change; the march re-boots from `t = 0`)".
  - REQ-INT-080, the calibration of `Δ_sync`; R-560, the default occupant Heggie × RK4; R-602; R-610, `T ≤ 200`.
  - `docs/design/principia_chart_composition.md` §§ 18, 19.
- **Findings:** a mode needs no `AxisKind`; "axis kind" in the builder is the selector's word. Rows read boundary states: no new landing logic. `n_sync` no longer exists (R-565). For an unregularised occupant, past `T = 65.535` the macro-step moves with `T`, so raising `T` there is a new run; a regularised occupant has no such line. Visible rows cost `payload_mem`, 315 MB per copy at 1080p, 348 MB with each row's own word: a reduced word isn't append-only, so no length recovers it; macro-step dense output is 3.79 GB per copy at `T = 13`, 14.6 GB at the default `T = 50`, 19.1 GB at `T = 200`.
- **Findings of physics note 3 (a findings note, not a ruling):**
  - *The window comes off the sim key.* `T` stays on it as the march's top; `(t_lo, t_hi, warp, rows)` is a view. Raising `T` as a continuation **amends the caching contract's `T` row**, for regularised occupants always and for unregularised ones while `dt_macro` is unchanged, up to R-610's cap.
  - *The default occupant resolves time only to `Δ_sync`.* At `T = 13`, `H = 1080`: 130 distinct rows at 0.1, drawn as bands 8 rows tall. Time zoom floors at one boundary; re-integrating can't go finer. **This mode now bears on REQ-INT-080's value**, whose cost is 2% at 0.1 and about 50% at 0.0125. The cost measured so far is on an unregularised stand-in; **the re-measure on Heggie × RK4 is owed.**
  - *Every option amends the temporal note.* `H` past states of one trajectory is history in its words, bounded by `payload_mem`.
  - *Stored by distinct boundary,* the default occupant's every boundary at `T = 50` is 36 MB to 1.17 GB per copy, so the size conflict is the unregularised occupants'.
- **Options:**
  - (a) **Recommended.** A chart mode, no fifth kind, **Time** in the builder's selector. `T` on the sim key, the window off it. Store by distinct boundary over `[t_lo, T]`, at most `K = H`, with `2H` a labelled extra under the tier budget; a finer window re-integrates from `t = 0`, progressively, as R-66, down to one boundary, where zoom stops; more through a Result (R-572), on disk. Time to IC-only is a cut. It amends the temporal note for visible rows, bounded by `payload_mem`, and the caching contract's `T` row for continuation.
  - (b) As (a), with dense `r`, `p` per column in a capped budget, for the trace and the viewer only. A wider amendment of the note, and a cache class.
  - (c) Full dense `SimState`, reversing "store no history". It needs a ruling that reverses the temporal note, and doesn't run below a discrete GPU.
- **Applied meanwhile:** (a), unbuilt.

## RQ-728: Region D at unequal masses: the labelled mask, Anosova's half-disc, and its reach *(physics; the owner's latest order A5; RQ-714; context: TASK-M8-110, AN-1, LL-1 to LL-3, CH-1)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the plane is geometric at every mass, as the owner says, and the exact map to the sphere is written. But "the longest side is 1" doesn't hold for the 1969 unequal-mass paper: it abandons region D for a half-disc of radius 10. And at unequal masses D holds only one of six labellings, so a count over D isn't a count over all triangles.*

- **File, section:**
  - the owner's latest order, A5: "The papers' native frame is the apex position (xi, eta) with A1=(-1/2,0), A2=(1/2,0), longest side = 1 (TARGETS.md:475; replication/catalogue.md 1.1), and at unequal masses (Li & Liao, Anosova 1969) it is geometric, not COM-centred."
  - `docs/incoming/_local/anosova_1969_Ap_5_81_part3.pdf`, p. 81: "the domain D does not include all the possible configurations"; "a semicircle of radius 10 units of distance and centered on the origin".
  - `docs/incoming/2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf`, § II.
  - `docs/incoming/2026-10-27_papers/TARGETS.md` LL-1's caveat: "each mass set's points lie in one labelled sector".
  - `docs/design/principia_dd_replication.md` § 3: AN-1's chart, "the Anosova physical-frame entry at a mass point".
  - `docs/design/principia_chart_composition.md` § 8.8.
- **Options:**
  - (a) **Recommended.** The plane is geometric at every mass, `ξ + iη = d + λ/ρ`. Region D is a labelled mask, one sector at unequal masses, and "blank and uncounted" reaches equal masses at `L_z = 0`. AN-1 is its own preset, a half-disc of radius 10, with A₁A₂ = 1 not the longest side; its placement is derived, not printed: at (3, 3, 1) "the coordinate axis is the axis of symmetry" holds only with the equal masses at `A₁, A₂`, so `C` is `m₃`, and both signs of `ξ′` make the rest immaterial to a statistic. A statistic over D at unequal masses says "one labelling".
  - (b) As (a), with every unequal-mass statistic run over all six sectors.
- **Applied meanwhile:** (a), unbuilt.

## RQ-729: Paper frames the library doesn't settle *(a silence; the owner's latest order part E; RQ-650, open; context: TASK-M8-112)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: most targets' coordinate frames can now be written as presets. A few details aren't in anything we hold. Four the order listed turned out to be readable after all. The rest should wait unbuilt rather than be guessed.*

- **File, section:** `docs/design/principia_chart_composition.md` § 20, its gaps table, each with its path; the order's list: "Tanikawa-Mikkola's fixed-k velocity prescription and D12; Hristov 2025's conventions; Valtonen 2005's rotation parameter w; Martynova 2009; Boyd & McMillan 1993; Hut 1983; Heggie & Sweatman 1991's full setup; Henon 1974's A; the L/M/A/H boundary definitions."
- **The finding for RQ-650:** Tanikawa & Mikkola 2008 prints `D11 = {x ≥ 0, y ≥ 0, (x − 0.5)² + y² ≤ 1}`; its Fig. 3 draws `D₁₁` as Anosova's region and `D₁₂` beside it inside the right circle. So the printed set is exactly `D₁₁ ∪ D₁₂`: neither a mirror nor plainly a typo, a reading RQ-650's record doesn't have. It doesn't rule RQ-650.
- **Options:**
  - (a) **Recommended.** Presets where read, tested against the paper. Each remaining gap stays unbuilt, and a target that needs it reads "not scored" with the gap named; no frame is guessed.
  - (b) Fetch or read the held copies first, then write all the presets at once.
- **Applied meanwhile:** (a), unbuilt.

## RQ-730: Hill's threshold at unequal masses *(physics; the owner's order part C; a definition (R-72); context: TASK-M8-109)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the Hill-stability limit on `L²E` is one number at equal masses, 25/972. At unequal masses it is three, one for each body in the middle of the Euler line, and the allowed region splits in stages. So "distance from the threshold" has to say which. All choices agree at equal masses. Hill-stable also means no exchange, not bound: the third body can still leave.*

- **File, section:** the owner's order, part C: "distance from the Hill-stability threshold on Lz^2*E (Marchal/Saari/Zare)"; physics note 2 of 6 Nov, P17: at `m = (.6, .3, .1)` the thresholds are 0.011599, 0.012261, 0.012917; `docs/design/principia_chart_composition.md` § 3.1.
- **Options:**
  - (a) **Recommended.** The labelled pair: pair `(i, j)` is cut off from the third iff `c > max(f_i, f_j)`, so it says something about named bodies, as the chart's labelled pair does. Blank at `E ≥ 0`, `ε ≠ 0`.
  - (b) `max` over all three: the threshold for any pair being cut off.
  - (c) The median `f_k`: where the region first disconnects.
- **Applied meanwhile:** none chosen; the row is a definition requirement.

## RQ-731: REQ-VAL-326's pointer to the 2015 paper *(a corpus error; `plan/requirements.yaml`, REQ-VAL-326; context: TS-2, TASK-M9-28)*

*Queued 7 Nov 2026 from the chart composition design, PR #422, for the owner's rulings report. Nothing is implemented until ruled (the owner's order of 6 Nov, A6).*

*In plain English: the requirement behind target TS-2 says the fixed-k prescription is read from the 2015 paper. The 2015 paper is free-fall and doesn't have it; the 2008 paper does.*

- **File, section:** REQ-VAL-326's prescription "read from the 2015 PASJ paper"; `docs/design/principia_dd_replication.md` TS-2: "Tanikawa & Mikkola 2008, Fig. 7"; physics note 2 of 6 Nov, RQ-725. Also: TS-2's "virial ratio 0 to 0.7" is `k = T/|U|`, not `Q`; `Q = 2k`.
- **Options:**
  - (a) **Recommended.** Change the pointer to Tanikawa & Mikkola 2008 § 3.1 and § 4.2, and note `k` against `Q`. TS-2 stays "not scored" (R-669).
  - (b) Leave it, since TS-2 isn't scored.
- **Applied meanwhile:** nothing.
