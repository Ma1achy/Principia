# TASK-M4-28 — The kernel cross-backend check: the same IC set marched on NVIDIA Vulkan, Metal, lavapipe and CPU f32, labels and branch-sensitive results compared per backend pair (R-686)

- **Milestone:** M4
- **Closes:** REQ-VAL-348
- **Depends on:** TASK-M4-05, TASK-M4-02, TASK-M4-03, TASK-M4-18, TASK-M4-27
- **Needs (earlier milestones):** REQ-INT-026, REQ-EVT-003, REQ-EVT-005, REQ-VAL-026
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~600 lines

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
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"
- `docs/notes/principia_gpu_determinism_note.md` § "The parity consequence (what shared source does and does not buy)"

## Deliverables
- `cargo xtask gate cross-backend-march`, on TASK-M4-27's record and comparison: `--record` marches the IC set on the job's
  backend, `--compare` over records. The CPU-f32 march is the kernel's own wrapper at f32, which this task drives inside TASK-M4-02's
  harness; that harness alone is `stepOnce` and `decodeOnly` on shared states, fixed inputs that say nothing of a trajectory.
- Two arms, each on all four backends (parity contract §3). **Decoded:** the march from the same chart uniforms, each backend
  decoding its own start state, since TASK-M4-05's march takes no IC buffer. **Shared:** one decoded state's bits as every
  backend's start, stepped by TASK-M4-02's `gpu_step_once`, iterated by this task, and on CPU f32 by the wrapper at f32.
- The IC set, fixed before the first run and the same bits on all four: `fixtures/gates/parity_fixtures` (TASK-M4-18), with its
  non-chaotic and chaotic marking, and a near-decision stratum, `fixtures/gates/cross_backend_near_decision`, of ICs chosen to
  straddle collision, escape and `N_sub` decisions and a uniform grid across a basin boundary. The header states each
  stratum's N, so 0 of N carries its resolution.
- A fixed-input arm: `fixtures/gates/tier_l_boundary_states` (TASK-M4-03) and TASK-M4-02's `synth.rs` generators, on all four
  backends, each row with the same operands' and input's bits as `--replay`, reported per pair as a measurement, so a comparison
  exists where chaos can't reach it. TASK-M4-03 names `macos-15`
  and lavapipe; this arm is what brings REQ-INT-057's decisions to NVIDIA at identical inputs before TASK-M8-41.
- One row per trajectory: the outcome label, `state`, the collision decision and pair, the escape decision and escaper, the
  terminal step and the total substeps, with the decoded start state's bits, so a disagreement already present at `t = 0` is
  counted apart; a digest of each step's branch words (`N_sub`, `state`, the collision and escape decisions) and checkpoint
  states at a stride stated in the header, so `--compare` can find the first step that differs.
- `--replay`: for each disagreement, one backend's state bits at the step before the first difference, re-run from the last
  common checkpoint, go through `gpu_step_once` on every backend, NVIDIA's included, reported per pair. Each backend reads back,
  at the evaluation that forked, the expression's own operands and the input's bits. So the NVIDIA session runs `--record`, then
  `--replay` on the file `--compare` writes. The per-step digests stay branch words.
- The report, written into the gate report: each pair's disagreement count and rate, for each of those results and for any of them, non-chaotic and chaotic
  apart; for each disagreement, the first step that differs and whether the branch operation there agrees on identical
  inputs, as REQ-VAL-079's report does, the kind column of TASK-M4-27, extended to the march's branch inputs from TASK-M4-03's `branch_inputs.rs`.
  The header states the domain, the commit, each adapter line and the compute setting.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo test -p validation cross_backend_march` (no GPU) — over synthetic records: a record against itself reads 0, one flipped
  label reads 1 of N, one changed step count reads 1 of N under that result alone, and the strata sum to the whole; the control is a compare that ignores step counts, which the changed step count rejects (REQ-VAL-348).
- `cargo test -p validation cross_backend_replay` (no GPU) — over synthetic digests, the first differing step is found at its
  step, and the state handed to `--replay` is the same bits for every backend; the control is a digest that drops `N_sub`, which
  a planted `N_sub` difference rejects. The standing, on the march's kinds, as `cross_backend_compare`'s: equal operands and differing
  input bits, no fork, reads "unverified, not supported"; differing operands do not; `other operation` does not. The control is a
  compare that reads only outcomes, which misses the first. A replayed collision fork whose positions differ reads as an upstream
  difference (REQ-VAL-348).
- `cargo xtask gate cross-backend-march` — both arms' records on lavapipe, on Metal and on CPU f32, their comparison and the
  replay, in the PR, with each driver's standing under R-690's condition, every backend pair listed, NVIDIA's as "not run" until its record is posted (REQ-VAL-348).
- `python3 plan/check_plan.py`.

## Notes
- **A measurement under R-649, on tier (a), the GPU explorer.** It has no outside number, so it is neither an M9 target nor a
  peek. It blocks nothing. **No tolerance chosen to make it pass:** the report holds no threshold. Each pair's outcome: PASS is 0
  disagreements; anything else is EXPLAINED DIVERGENCE, with a measurement showing the mechanism, or UNEXPLAINED. Only the
  controls assert.
- **Reading the rate.** Metal against lavapipe is the clean-backend pair, and the NVIDIA pairs are read against it. On a chaotic
  trajectory a rounding difference can change a label with no defect in any branch (pitfalls §10), so a chaotic rate alone
  shows no miscompile; the first-difference replay is what separates the two.
- **A nonzero NVIDIA rate alone is not evidence on `NoContraction`.** The RTX's runtime division differs from correct rounding
  on 19407 of 65536 where Metal off and lavapipe read 0 (RQ-685), and M4 adds divisions to the force loop and the steppers, so
  a march on the card should differ from the first such step whatever contraction does. The discriminating result is differing
  bits at bit-identical operands, in a branch input built only of `+` and `×` (R-547). That points at contraction, reassociation or
  distribution. A difference in a branch input holding a division, `E_rel` or `|Δn̂|`, is not attributable that way.
- **A shared state is not shared operands.** `detect_terminal` runs after the STEP, so the collision test reads `d²` of post-STEP
  positions, which have been through the force loop's divisions. A collision fork in `--replay` has identical inputs at the
  replay's level, and `d²` is `+` and `×`, but if its positions differ it is an upstream difference and convicts no driver.
- **R-690's driver condition** is about bits, not forks, on TASK-M4-27's three levels, at the expression's own operands. A failing driver is recorded **unverified,
  not supported**, in those words. Nothing enforces the condition, and the recording blocks nothing.
- **The NVIDIA record** is a Windows session's run at the PR's head, `--record` with `PRIN_GPU_BACKEND=vulkan`, the file posted to
  #390, under the same hardware label as that card's other runs. It is not a CI job (R-186). Nothing waits on it, and the task
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
