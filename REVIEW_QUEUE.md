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
