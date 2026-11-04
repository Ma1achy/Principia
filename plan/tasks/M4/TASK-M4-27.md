# TASK-M4-27 — The kernel cross-backend check, the arm for now: decode's and canon's branch outcomes on NVIDIA Vulkan, Metal, lavapipe and CPU f32, per backend pair (R-686)

- **Milestone:** M4
- **Closes:** REQ-VAL-347
- **Depends on:** TASK-M2-07, TASK-M2-24, TASK-M2-25
- **Needs (earlier milestones):** REQ-DEC-017, REQ-DEC-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~750 lines

## Goal
The part of R-686's check that can run at this commit, on what the GPU runs: decode's and canon's branch outcomes, on
identical inputs, on NVIDIA Vulkan, Metal, lavapipe and CPU f32, with the disagreement rate reported per backend pair and
no tolerance chosen to make it pass. It is a measurement (R-649). **It cannot answer the trajectory question:** these are
fixed inputs through two stages, and the GPU runs no march yet, so a clean table says nothing about collision or escape
decisions along a trajectory, or about whether the missing `NoContraction` matters for the real kernel. TASK-M4-28 does
that. It is the first test of R-690's driver condition, on fixed inputs.

## References
- `decisions.md` § "R-686 — The kernel cross-backend check is a measurement (R-649): the same IC set on NVIDIA Vulkan, Metal, lavapipe and CPU f32, outcome labels and per-trajectory branch-sensitive results compared, not pixels, the disagreement rate reported per backend pair, no tolerance chosen to make it pass; pixel comparisons across backends go on raw pre-colour values with the outlier tail reported, never 8-bit frame equality, Metal against lavapipe's 161,394 / 221 recorded as the clean-backend baseline; `DECODE_SWITCHOVER`'s spec notes that bitwise agreement must be tested on NVIDIA specifically"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `decisions.md` § "R-690 — R-547's "operations every IEEE backend rounds alike" is stated as a condition on the driver and tested by TASK-M4-27 and TASK-M4-28; it is not enforceable, and a driver that fails is recorded unverified, not supported *(closes RQ-690; amends R-547)*"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"
- `docs/notes/principia_gpu_determinism_note.md` § "Why branches are special (continuous divergence is fine; branch divergence is not)"
- `decisions.md` § "R-437 — The value of every `atan2` at a possible pole of the decode, encode and canonicalisation is defined: 0, for every sign of the zeros, chosen branch-free by a select on the operands against 0, so CPU and GPU agree *(closes RQ-285)*"

## Deliverables
- `crates/validation/src/cross_backend.rs`: a record, one file per backend, keyed by the commit, the adapter line, the driver
  version, the compute fast-math setting, the hardware label if any, given by `--hardware-label`, and a hash of the inputs, with one row per input and branch; and the comparison of any
  set of records, every pair: disagreements, inputs compared, the rate, and the inputs that disagreed. The rate counts inputs: an input
  disagrees if any of its rows' outcomes does, and the denominator is the inputs compared. A row's outcome in the rate is the stage's
  where the stage emits it, otherwise the read-back's, and the row names which.
- `cargo xtask gate cross-backend-branches`: `--record` on the job's backend, `--compare` over records. The inputs are those
  `crates/engine/tests/canon_gpu.rs` and `crates/engine/tests/decode_cross_backend.rs` build, seeds fixed, with #393's from
  `crates/validation/tests/qa_TASK-M2-25_gpu.rs`, written once to `fixtures/gates/cross-backend-branches/inputs/` by a generator this task adds,
  and read from there by every backend's `--record`, since those builders call the platform's `sin_cos` and `powf`, whose bits need not match on Windows,
  and a pair over different inputs is refused. Sweeps built on a GPU's own decodes (`canonicaliser_noop_gpu`) take CPU f32's decodes
  here, for the same reason. The outcomes are canon's branches (skip, coincidence, `ρ = 0`, mirror, `ρ_x` and
  `ρ_y` zero), decode's tags (`M01_TINY`, `INVARIANT_INFEASIBLE`, the energy tags) and decode's pole selects (R-436, R-437, R-438).
  `stage_canon_canonicalise` emits only the skip (`rescaled`), `ρ = 0` (`clamped`) and the mirror, and the decode entries only the tags,
  so the coincidence, `ρ_x` and `ρ_y` zero and the pole selects reach a record only through the read-back below.
  The report's header states its domain and that it does not answer the trajectory
  question. The comparison is written into the gate report.
- The test of R-690's condition, in `cross_backend.rs`. Each row has a **kind** column, from a list this task enumerates from the
  written forms at `crates/kernel/src/canon.rs`, `decode/mass.rs`, `decode/invariant.rs` and `chart/burrau.rs`, and read on what a driver
  can round (R-690). `+ and ×` is floating `+`, `−` and `×`, with what rounds nothing and a rewrite can't move: negation and `abs`,
  exact power-of-two scalings, bit reads and masks, integer operations, selects and comparisons. So canon's and decode's `Wide`
  inputs and `decode/invariant.rs`'s sign reads are this kind. `other operation` holds an operation with an ulp bound rather than
  correct rounding (division, `√`, `sin`, `cos`, `atan2`, a rounding builtin; #393's select, reached through `sin`, is this kind),
  or `fma`, which one driver fuses and another rounds as two operations (R-547, RQ-298). TASK-M4-03's
  `branch_inputs.rs` isn't used: it lists the march's inputs and this task must not wait for it. The list gives each input of the first kind
  the kernel function the arbiter evaluates, called at f32 with no second copy. The comparison gives each such row its level (Notes).
- The read-back, `crates/kernel/src/stage/branch_read.rs`, the source of a row's bits: one compute entry for canon and one for each decode variant the
  inputs run, built into `target/spirv/kernel.wgsl` as `stage_branch_read_*`, with a dispatcher beside `canonicalise_gpu` in
  `crates/engine/src/decode_dispatch.rs`. It is a module of its own because `decode_cross_backend_registry` takes every `stage_decode_` entry for a decode
  variant it must run. No parity-only physics: each branch's input and decision move into a function that its production site and the
  entry both call, with no change of behaviour, and CPU f32's record calls the same functions at f32. For each branch the entry emits
  the outcome, and for kind `+ and ×` the expression's own operands and the input's bits, both sides of a comparison, a `Wide`'s
  mantissa and exponent both. Canon's operands are the sample's `(m, r)` words, and `decode/mass.rs`'s are the masses that entry emitted. Each row
  names the compiled entry its bits and outcome came from.
- What the commands need from the runner, decided per R-369; TASK-M4-28 reuses it. No threshold enters (R-649).
  - **The flags.** `xtask/src/main.rs` and `xtask/src/gate.rs`'s `Which::One` forward what follows the gate's name to validation's `gate` binary: `--record`, `--compare <records>` and `--hardware-label`, and TASK-M4-28's `--replay` and `--replay-state`.
  - **A measurement gate.** `fixtures/gates/cross-backend-branches/gate.json` carries `"measurement": { "requirement": "REQ-VAL-347", "ruling": "R-649" }` in place of `threshold` and `min_regions`, which `GateFile` in `crates/validation/src/gate/mod.rs` then leaves out, and a file with both is refused. Its `inputs` are the control records alone, the only ones with `"expected"`. A record under `--compare` has none and can't fail the gate. `gate <name>` with no flag and `gate --all` run those controls, no GPU; `--list` shows the gate as a measurement.
  - **The directory.** The gate's directory is its name, so the inputs are `fixtures/gates/cross-backend-branches/inputs/`.
  - **The "not run" row.** `fixtures/gates/cross-backend-branches/outstanding.json` lists each outstanding run with its state, and a commit changes it when a run is posted. `xtask/src/gate_report.rs`'s `render` prints a section of those lines for the milestone. They are not `Outcome`s, so they count neither as a pass nor as a failure, and the requirement's own line stays on its controls, as `gate.yml` marks it.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo test -p validation cross_backend_compare` (no GPU) — the comparison over synthetic records: a record against itself
  reads 0 of N, and one flipped outcome reads 1 of N and names the input; a pair over different inputs is refused, not
  compared; the control is a compare that ignores one outcome column, which the flipped case rejects. The standing: a synthetic pair of kind `+ and ×`
  with equal operands and differing input bits, no fork, reads "unverified, not supported", and the pair reads 0 disagreements and not PASS; with equal operands and bits and a
  different outcome it reads the same, as a comparison fork; with differing operands it does not; a `Wide` input with equal operands and only its exponent differing reads "unverified, not
  supported"; of kind `other operation`, an `fma` form included, it does not; at level 2 the driver whose bits differ from the written form on CPU f32 is the one named, and at level 3 the one whose outcome
  differs; two GPUs sharing operands CPU f32's run didn't produce are arbitrated by the host's evaluation at those operands, and the one differing is named; a clean driver reads "no failure in N", with its level 1 and `other operation` counts beside it; a
  driver whose rows are all level 1 reads "no failure in 0"; no record reads "not run"; each standing's domain names the entry its rows were read through. A row whose stage outcome differs while the read-back's
  operands, bits and outcome are equal reads "production-only difference", counts in the rate, takes no level and earns no word; the control is a compare that
  takes the outcome from the stage and the bits from the read-back, which calls that row a comparison fork.
  The control is a compare that reads only outcomes, which misses the first case; the kinds' control is a list that files a `Wide` input under
  `other operation`, which the `Wide` case rejects; N's control is a compare whose N counts all rows, which the "no failure in 0" case
  rejects (REQ-VAL-347).
- `cargo test -p validation cross_backend_read_back` (no GPU) — at f32, on every input of the fixture, each branch's outcome through the read-back's functions is what
  `canonicalise` and the decode give where they emit it, and each kind `+ and ×` input re-evaluated from its emitted operands gives its emitted bits; the
  control is a read-back that emits one operand from the wrong body, which the re-evaluation rejects (REQ-VAL-347).
- `cargo test -p validation measurement_gate` (no GPU) — a `gate.json` with `measurement` and no `threshold` loads and lists; one with both is refused; a control input off its `expected` fails the gate, and a record with no `expected` under `--compare` can't, at any rate; the control is a compare that judges a record against 0, which that case rejects (REQ-VAL-347).
- `cargo test -p xtask gate_report_outstanding` (no GPU) — an outstanding row prints "not run", a posted one what was posted, and the report's pass, fail and exit are the same with or without them; the control is a `render` that makes the row an `Outcome`, which that case rejects (REQ-VAL-347).
- `cargo xtask gate cross-backend-branches` — records on lavapipe, on Metal and on CPU f32 and their comparison, in the PR,
  each pair's rate, the adapter lines and each driver's standing under R-690's condition; the header states the domain; every backend pair is listed, NVIDIA's as "not run"
  until its record is posted (REQ-VAL-347).
- `python3 plan/check_plan.py`.

## Notes
- **R-690's driver condition is about bits, not forks.** An ulp moved in a branch input forks only within about an ulp of a threshold,
  so a test of outcomes alone would read a failing driver as clean. Every row has identical stage inputs, but the kind is the
  expression's own, read at its own operands (`decode/mass.rs`'s block is built on the emitted masses, not on `z`). Three levels, on a row of kind `+ and ×`:
  - level 1, operands differ across the backends compared: an upstream difference, no evidence on the driver;
  - level 2, operands bit-identical, input's bits differ: the driver fails, with or without a fork;
  - level 3, operands and bits identical, outcome differs: a comparison fork, reported as that, and it fails too.

  A failing driver is recorded **unverified, not supported**, in those words. A row of kind `other operation` is reported as that
  kind. Nothing enforces the condition, and the recording blocks nothing.
- **Levels are judged within one named entry** (applied per R-369). Operands, bits and outcome all come from the read-back entry, never bits from
  one shader and an outcome from another. A stage outcome that differs where the read-back's operands, bits and outcome don't is a
  **production-only difference**, under that name: it counts in the rate, it is no level and it earns no word. A rewrite in the
  production compile alone would otherwise read as a comparison fork, and an upstream difference there would convict a driver.
- **From a pair to a driver.** The arbiter is the written form evaluated on the CPU at f32, each operation correctly rounded, not
  the exact expression's correctly rounded value, which the kernel's form isn't (`crates/kernel/src/real.rs`:134). It runs at CPU f32's operands:
  those of its own run's row or, where two GPUs share operands that run didn't produce, those shared operands, which the host evaluates
  with the kernel's form at f32, no second copy (applied per R-369: this names a driver where the record otherwise has none). At level 2
  the driver whose bits differ from it fails; at level 3, the driver whose outcome differs from CPU f32's. Operands that match
  neither are level 1. Each driver's standing reads one of "unverified, not supported", "no failure in N", N shown, never
  "verified" or "supported", or "not run". N counts evaluations, one input's one branch on one driver, of the first kind with operands
  bit-identical to CPU f32's; the rate's denominator is inputs, a different count. Level 1 rows and `other operation` rows never reached the test,
  so each is counted apart, and each driver's line shows, per branch input and entry, N and the level 1 count, with the `other operation` count beside them. "Driver unverified" (RQ-685's ruling) is a run header's word for a driver not measured,
  not one of these. A standing's domain is stated with it: the compute stage, setting off, the adapter line and driver version,
  the compiled entry read, and for NVIDIA its hardware label ("possibly faulty hardware"), so the words attach to that driver on that card.
  A read-back entry is a different shader from the one that gave the row's outcome, and a rewrite depends on the code around the expression:
  differing bits there fail the driver in that entry, and no failure there says nothing of the production pipeline's compile.
- **A measurement under R-649, on tier (a), the GPU explorer.** It has no outside number, so it is neither an M9 target nor a
  peek. It blocks nothing, and no tolerance or threshold enters it. Each pair's outcome: PASS is 0 disagreements with neither driver of the pair recorded unverified, not supported: the rate
  counts outcomes, so bits differing at bit-identical operands of kind `+ and ×` leave it at 0 and are not PASS. Anything else is EXPLAINED DIVERGENCE, with a measurement showing the mechanism, or UNEXPLAINED.
- **The NVIDIA record** is a Windows session's run at the PR's head, `--record` with `PRIN_GPU_BACKEND=vulkan`, the file posted to
  #390. It is not a CI job (R-186). Nothing waits on it, and the task closes without it: M4's gate report lists the row as "not
  run" until it is posted, so the run stays visible (`plan/MILESTONES.md` § M4).
- **What is already known:** on this card `decode_` passes 36 of 36 and `canonicaliser_` 28 of 28 (RQ-685). And #393: on identical
  inputs, setting off, the compute decode stage gives `rho_angle` 0 where f64 gives π/2, inferred to be R-437's `p = q = 0`
  select (RQ-685, RQ-687). The corpus treats such selects as "no branch", so a table of tags alone would read 0 past it; hence
  the select columns. Those are pass
  counts of assertions, the CPU against one backend; this task gives every pair's rate over one input set. The tests in
  `canon_gpu.rs` and `decode_cross_backend.rs` stay as they are and go on asserting.
- A separate task from TASK-M4-28 because a task starts only when its Depends on has merged, and this one must not wait for
  the march. Applied per R-369 (R-686). TASK-M4-28 extends this record.
