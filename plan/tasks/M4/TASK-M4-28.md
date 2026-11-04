# TASK-M4-28 — The kernel cross-backend check: the same IC set marched on NVIDIA Vulkan, Metal, lavapipe and CPU f32, labels and branch-sensitive results compared per backend pair (R-686)

- **Milestone:** M4
- **Closes:** REQ-VAL-348
- **Depends on:** TASK-M4-05, TASK-M4-02, TASK-M4-03, TASK-M4-09, TASK-M4-18, TASK-M4-27
- **Needs (earlier milestones):** REQ-INT-026, REQ-EVT-003, REQ-EVT-005, REQ-VAL-026
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~900 lines

## Goal
The owner's evidence for whether the missing `NoContraction` matters for the real kernel (R-686). The same IC set runs on
NVIDIA Vulkan, Metal, lavapipe and CPU f32, and the check compares outcome labels and each trajectory's branch-sensitive
results, collision and escape decisions and step counts, not pixels. It reports the disagreement rate per backend pair,
with no tolerance chosen to make it pass. It is a measurement under R-649. TASK-M4-05 delivers the march it runs. The rate
alone does not answer the owner's question; the bits at identical operands do, and they test R-690's driver condition (Notes).

## References
- `decisions.md` § "R-686 — The kernel cross-backend check is a measurement (R-649): the same IC set on NVIDIA Vulkan, Metal, lavapipe and CPU f32, outcome labels and per-trajectory branch-sensitive results compared, not pixels, the disagreement rate reported per backend pair, no tolerance chosen to make it pass; pixel comparisons across backends go on raw pre-colour values with the outlier tail reported, never 8-bit frame equality, Metal against lavapipe's 161,394 / 221 recorded as the clean-backend baseline; `DECODE_SWITCHOVER`'s spec notes that bitwise agreement must be tested on NVIDIA specifically"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `decisions.md` § "R-690 — R-547's "operations every IEEE backend rounds alike" is stated as a condition on the driver and tested by TASK-M4-27 and TASK-M4-28; it is not enforceable, and a driver that fails is recorded unverified, not supported *(closes RQ-690; amends R-547)*"
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `decisions.md` § "R-58 — The non-Metal parity run gates Paper 2, not the build *(TO-2)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "Tier S — structural only (asserting the pointwise gap would assert a falsehood)"
- `docs/contracts/principia_parity_contract.md` § "3. The load-bearing discipline: never accumulate before comparing"
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"
- `docs/notes/principia_gpu_determinism_note.md` § "The parity consequence (what shared source does and does not buy)"

## Deliverables
- `cargo xtask gate cross-backend-march`, on TASK-M4-27's record and comparison: `--record` marches the IC set on the job's
  backend, `--compare` over records. The CPU-f32 march is the kernel's own wrapper at f32, which this task drives inside TASK-M4-02's
  harness; that harness alone is `stepOnce` and `decodeOnly` on shared states, fixed inputs that say nothing of a trajectory.
- Two arms, each on all four backends (parity contract §3). **Decoded:** the march from the same chart uniforms, each backend
  decoding its own start state, through TASK-M4-05's survey entry, each IC one texel of its own quad, since that march takes no IC buffer. **Shared:** CPU f32's decoded state's bits as every
  backend's start, marched by the pass entry below, and on CPU f32 by the same body at f32. An IC held as `(m, r, p)` with no chart point runs on the shared arm alone, and the report says so.
- Two occupants, each through both arms, applied per R-369: the default, Heggie × RK4 (R-560), since that is the real kernel; and Yoshida-4, named in the header, since `N_sub`, `N_max` and the bucket exist only without regularisation (integrator contract § "Part 6 — Three boundaries stated honestly (deferred work lands here)"). The record's key gains the occupant and the sim key (`dt_macro`, the horizon, `r_coll`, `N_max` or the step budget), and a pair over different ones is refused, as a pair over different inputs is. Below, a **pass** is one macro-step, or under Heggie one sync interval (R-565).
- The pass entry. `gpu_step_once` is one STEP (parity contract § "1. The principle"): no `N_sub`, projection or detection, so iterating it isn't a march, and choosing `N_sub`
  on the host would put the CPU's branch in the GPU's record. So a compute entry in `crates/kernel/src/stage/branch_read.rs`, dispatched by `gpu_macro_pass` in `crates/validation/src/harness/`, calls TASK-M4-09's one loop body on its one carried-state type, with no second list of registers (applied per R-369). It takes and returns that carried state, and also returns the pass's branch words. Under Yoshida-4 those are `N_sub`, `state` and the collision and escape decisions. Under Heggie there is no `N_sub`: they are the steps taken, the budget flag, R-603's two counts, `state`, the collision decision and escape's sync-boundary sample.
- The IC set, fixed before the first run and the same bits on all four: `fixtures/gates/parity_fixtures` (TASK-M4-18), with its
  non-chaotic and chaotic marking, and a near-decision stratum, `fixtures/gates/cross_backend_near_decision`, of ICs chosen to
  straddle collision and escape decisions on both occupants and `N_sub` decisions on Yoshida-4, and a uniform grid across a basin boundary, each a chart and a point, all the survey entry takes. The header states each
  stratum's N, so 0 of N carries its resolution.
- A fixed-input arm: `fixtures/gates/tier_l_boundary_states` (TASK-M4-03) and TASK-M4-02's `synth.rs` generators, their output written once to
  `fixtures/gates/cross_backend_fixed_inputs` so every backend reads the same bits, each state with the registers `detect_terminal` reads, on all four
  backends, read at the given state (below), reported per pair as a measurement, so a comparison
  exists where chaos can't reach it. `synth.rs`'s feasibility-edge states are decode's, and go through TASK-M4-27's decode read-back entries. TASK-M4-03 names `macos-15`
  and lavapipe; this arm is what brings REQ-INT-057's decisions to NVIDIA at identical inputs before TASK-M8-41.
- One row per trajectory: the outcome label, `state`, the collision decision and pair, the escape decision and escaper, the
  terminal step and the total substeps, from the final `SimState`, with the decoded start state's bits, read through `gpu_decode_only` since the survey entry overwrites them
  in place, so a disagreement already present at `t = 0` is counted apart.
- A digest of each pass's branch words and carried-state checkpoints at a stride stated in the header, so `--compare` can find the first pass that differs. They come from
  `gpu_macro_pass` on both arms, since the survey entry emits neither: on the decoded arm each backend re-marches from its own decoded start state. Where a pair's survey rows differ and its passes' rows don't, that is TASK-M4-27's production-only difference; where one backend's survey row and passes' row
  differ, the report says so.
- `--replay`: for each disagreement, `--compare` names the first pass `k` that differs and a source, the pair's first backend in the order CPU f32, lavapipe, Metal, NVIDIA, so NVIDIA is
  never a source. `--replay-state`, on the source's backend, re-runs from its last checkpoint before `k` and writes the carried state at `k − 1`. Those same bits then go through the
  after-the-STEP read on every backend, NVIDIA's included, reported per pair. For an NVIDIA pair the order is a round trip: Windows `--record`, posted; `--compare` and `--replay-state` on the source's machine; then a second Windows run, `--replay` on that file.
  The per-pass digests stay branch words.
- The read-back, two more entries in TASK-M4-27's module, dispatched from `crates/validation/src/harness/`, no parity-only physics, each with a fixed slot per evaluation up to a bound per occupant: `N_max` under Yoshida-4, the pass's step budget under Heggie (`step_budget_rate`, `N_min`). An evaluation past the last slot is counted and reported on the driver's line, never dropped. For every branch input in `branch_inputs.rs`, each
  emits the outcome, and for kind `+ and ×` the expression's own operands and the input's bits, a `Wide`'s mantissa and exponent both. Each row names its entry, as TASK-M4-27's record does, and a standing's domain carries it. Which read is which:
  - **At the given state**, for the fixed-input arm: `substep_bucket`'s `d²` and `detect_terminal`'s branch inputs evaluated on the given state, through the kernel's own code, with no STEP, projection or invariant
    update before them; the bucket under Yoshida-4 alone, since Heggie has none. Their operands are the fixture's bits, and `synth.rs`'s straddling states stay on their thresholds.
  - **After the STEP**, for `--replay`: one pass as `gpu_macro_pass` runs it, under Yoshida-4 the bucket's `d²` at its start, and `detect_terminal`'s inputs after each STEP, on positions that have been through the force loop, the evaluation that forked.
- The report, written into the gate report: each pair's disagreement count and rate, for each of those results and for any of them, non-chaotic and chaotic
  apart; for each disagreement, the first step that differs and whether the branch operation there agrees on identical
  inputs, as REQ-VAL-079's report does, the kind column of TASK-M4-27, extended to the march's branch inputs from TASK-M4-03's `branch_inputs.rs`.
  The header states the domain, the commit, each adapter line and the compute setting, NVIDIA's replay as its own row, "not run" until posted, so a record posted with its replay outstanding reads as that, and that N comes from the fixed-input arm and the replays. Each driver's line shows, per branch input and entry, N and the level 1 count.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo test -p validation cross_backend_march` (no GPU) — over synthetic records: a record against itself reads 0, one flipped
  label reads 1 of N, one changed step count reads 1 of N under that result alone, and the strata sum to the whole; the control is a compare that ignores step counts, which the changed step count rejects (REQ-VAL-348).
- `cargo test -p validation cross_backend_replay` (no GPU) — over synthetic digests, the first differing pass is found, under both occupants, a planted budget-flag difference under Heggie among them, and the state handed to `--replay` is the same bits for every backend; a pair over different occupants or sim keys is refused; an evaluation past the last slot reads as a counted overflow; the control is a digest that drops `N_sub`, which
  a planted `N_sub` difference under Yoshida-4 rejects. The standing, on the march's kinds, as `cross_backend_compare`'s: equal operands and differing
  input bits, no fork, reads "unverified, not supported", on a `Wide`'s exponent alone too, and the pair reads 0 disagreements and not PASS; differing operands do not; `other operation` does not; two GPUs sharing operands CPU f32's run didn't produce are arbitrated by the host's evaluation at those operands, and the one differing is named; a driver whose rows are all level 1 reads "no failure in 0", and N's control is a compare whose N counts all rows, which that case rejects; under Yoshida-4 a driver with N > 0 at the bucket and level 1 at collision shows 0 at collision under its own name, as under Heggie one with N > 0 at the given-state collision read and level 1 after the STEP shows 0 there, and the control is a line that sums the branch inputs, which those cases reject. The control is a
  compare that reads only outcomes, which misses the first. A replayed collision fork whose positions differ reads as an upstream
  difference (REQ-VAL-348).
- `cargo test -p validation cross_backend_passes` (no GPU) — at f32, on Yoshida-4 and on Heggie × RK4, iterated from an IC, the loop body gives `integrate`'s `SimState` bit for bit, and from a checkpoint's carried state it gives the same digests as a run through it; a near-collision state read at the given state has the fixture's bits as its operands and stays straddling `r_coll²`; the control is that state read after the STEP, whose operands the case rejects (REQ-VAL-348).
- `cargo xtask gate cross-backend-march` — both arms' records on lavapipe, on Metal and on CPU f32, their comparison and the
  replay, in the PR, with each driver's standing under R-690's condition, every backend pair listed, NVIDIA's as "not run" until its record is posted (REQ-VAL-348).
- `python3 plan/check_plan.py`.

## Notes
- **A measurement under R-649, on tier (a), the GPU explorer.** It has no outside number, so it is neither an M9 target nor a
  peek. It blocks nothing. **No tolerance chosen to make it pass:** the report holds no threshold. Each pair's outcome: PASS is 0 disagreements with neither driver of the pair recorded unverified, not supported: the rate
  counts outcomes, so bits differing at bit-identical operands of kind `+ and ×` leave it at 0 and are not PASS. Anything else is EXPLAINED DIVERGENCE, with a measurement showing the mechanism, or UNEXPLAINED. Only the
  controls assert.
- **Reading the rate.** Metal against lavapipe is the clean-backend pair, and the NVIDIA pairs are read against it. On a chaotic
  trajectory a rounding difference can change a label with no defect in any branch (pitfalls §10), so a chaotic rate alone
  shows no miscompile; the first-difference replay is what separates the two.
- **A nonzero NVIDIA rate alone is not evidence on `NoContraction`.** The RTX's runtime division differs from correct rounding
  on 19407 of 65536 where Metal off and lavapipe read 0 (RQ-685), and M4 adds divisions to the force loop and the steppers, so
  a march on the card should differ from the first such step whatever contraction does. The discriminating result is differing
  bits at bit-identical operands, in a branch input of kind `+ and ×` (R-547; R-690). That points at contraction, reassociation or
  distribution. A difference in a branch input of kind `other operation` is not attributable that way.
- **The kind is read from `branch_inputs.rs`'s written form, not assumed.** `E_rel = ½|Δv|² − (M_pair + m_b)/d` is the physics definition, not the branch input's written form, which TASK-M4-03 writes and proves. By REQ-INT-064 and REQ-INT-061, `E_rel` and `|Δn̂|` are meant to be of kind `+ and ×`, so their operands are read like `d²`'s. A written form that holds a division or `fma` is reported as `other operation` and named as departing from REQ-INT-064.
- **A shared state is not shared operands.** `detect_terminal` runs after the STEP, so the collision test reads `d²` of post-STEP
  positions, which have been through the force loop's divisions. A collision fork in `--replay` has identical inputs at the
  replay's level, and `d²` is `+` and `×`, but if its positions differ it is an upstream difference and convicts no driver. That is why the fixed-input arm reads `detect_terminal`'s inputs on the given state, with no STEP.
- **R-690's driver condition** is about bits, not forks, on TASK-M4-27's three levels, at the expression's own operands, with its
  arbiter, the written form on CPU f32, each operation correctly rounded, its three words, its N and its domain, the compiled entry read included (TASK-M4-27's Notes). N here comes from the rows that read operands, the fixed-input arm's and the replays'; a trajectory's row and digests count in the rate, not in N.
  The fixed-input arm gives most of it, since its read at the given state holds operands identical at every site. In a replay on a card whose division differs, only Yoshida-4's bucket `d²` is sure of
  that, and under Heggie no site is: `detect_terminal`'s inputs are expected at level 1, and the per-input counts show it. Levels are judged within one entry, with TASK-M4-27's production-only difference. A failing driver is recorded **unverified,
  not supported**, in those words. Nothing enforces the condition, and the recording blocks nothing.
- **The NVIDIA record** is a Windows session's run at the PR's head, `--record` with `PRIN_GPU_BACKEND=vulkan`, the file posted to
  #390, under the same hardware label as that card's other runs. A record too big to attach goes as a committed file on a branch named on the issue. Its replay is a second run, in the order the `--replay` bullet gives, and blocks nothing either. It is not a CI job (R-186). Nothing waits on it, and the task
  closes without it: M4's gate report lists the row as "not run" until it is posted, though the owner's question rests on that
  row (`plan/MILESTONES.md` § M4). It doesn't stand in for R-58's parity run, REQ-VAL-099, which gates Paper 2; whether a driver recorded unverified,
  not supported can serve that gate is RQ-695.
- **Why generic ICs are not enough.** A 1-ulp difference flips a branch only within about an ulp of its threshold, so a small
  non-chaotic set reads 0 of N with a defect present (order of magnitude, assuming 10³ crossings per trajectory: 2.4e-4 per
  trajectory), and the chaotic stratum reads high with none. Hence the near-decision stratum and the fixed-input arm, which add
  to R-686's "same IC set" and change none of it.
- **Why TASK-M4-18 is a dependency:** it owns the fixture set and the label-flip report on Metal and lavapipe. This task adds the
  other two backends, the per-pair table and the per-trajectory results, and does not change that report.
- Applied per R-369 (R-686): its own task, not a part of the fix task, TASK-M6-53, since the mock's fix needs no march.
