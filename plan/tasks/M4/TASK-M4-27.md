# TASK-M4-27 — The kernel cross-backend check, the arm for now: decode's and canon's branch outcomes on NVIDIA Vulkan, Metal, lavapipe and CPU f32, per backend pair (R-686)

- **Milestone:** M4
- **Closes:** REQ-VAL-347
- **Depends on:** TASK-M2-07, TASK-M2-24
- **Needs (earlier milestones):** REQ-DEC-017, REQ-DEC-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~300 lines

## Goal
The part of R-686's check that can run at this commit, on what the GPU runs: decode's and canon's branch outcomes, on
identical inputs, on NVIDIA Vulkan, Metal, lavapipe and CPU f32, with the disagreement rate reported per backend pair and
no tolerance chosen to make it pass. It is a measurement (R-649). **It cannot answer the trajectory question:** these are
fixed inputs through two stages, and the GPU runs no march yet, so a clean table says nothing about collision or escape
decisions along a trajectory, or about whether the missing `NoContraction` matters for the real kernel. TASK-M4-28 does
that.

## References
- `decisions.md` § "R-686 — The kernel cross-backend check is a measurement (R-649): the same IC set on NVIDIA Vulkan, Metal, lavapipe and CPU f32, outcome labels and per-trajectory branch-sensitive results compared, not pixels, the disagreement rate reported per backend pair, no tolerance chosen to make it pass; pixel comparisons across backends go on raw pre-colour values with the outlier tail reported, never 8-bit frame equality, Metal against lavapipe's 161,394 / 221 recorded as the clean-backend baseline; `DECODE_SWITCHOVER`'s spec notes that bitwise agreement must be tested on NVIDIA specifically"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"
- `docs/notes/principia_gpu_determinism_note.md` § "Why branches are special (continuous divergence is fine; branch divergence is not)"

## Deliverables
- `crates/validation/src/cross_backend.rs`: a record, one file per backend, keyed by the commit, the adapter line, the compute
  fast-math setting and a hash of the inputs, with one row per input and its branch outcomes; and the comparison of any
  set of records, every pair: disagreements, inputs compared, the rate, and the inputs that disagreed.
- `cargo xtask gate cross-backend-branches`: `--record` on the job's backend, `--compare` over records. The inputs are those
  `crates/engine/tests/canon_gpu.rs` and `crates/engine/tests/decode_cross_backend.rs` build, seeds fixed. The outcomes are
  canon's branches (skip, coincidence, `ρ = 0`, mirror, `ρ_x` and `ρ_y` zero) and decode's tags (`M01_TINY`,
  `INVARIANT_INFEASIBLE`, the energy tags). The report's header states its domain and that it does not answer the trajectory
  question. The comparison is written into the gate report.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo test -p validation cross_backend_compare` (no GPU) — the comparison over synthetic records: a record against itself
  reads 0 of N, and one flipped outcome reads 1 of N and names the input; a pair over different inputs is refused, not
  compared; the control is a compare that ignores one outcome column, which the flipped case rejects (REQ-VAL-347).
- `cargo xtask gate cross-backend-branches` — records on lavapipe, on Metal and on CPU f32 and their comparison, in the PR,
  each pair's rate and the adapter lines; the header states the domain; every backend pair is listed, NVIDIA's as "not run"
  until its record is posted (REQ-VAL-347).
- `python3 plan/check_plan.py`.

## Notes
- **A measurement under R-649, on tier (a), the GPU explorer.** It has no outside number, so it is neither an M9 target nor a
  peek. It blocks nothing, and no tolerance or threshold enters it. Each pair's outcome: PASS is 0 disagreements;
  anything else is EXPLAINED DIVERGENCE, with a measurement showing the mechanism, or UNEXPLAINED.
- **The NVIDIA record** is a Windows session's run at the PR's head, `--record` with `PRIN_GPU_BACKEND=vulkan`, the file posted to
  #390. It is not a CI job (R-186). Nothing waits on it, and the task closes without it: M4's gate report lists the row as "not
  run" until it is posted, so the run stays visible (`plan/MILESTONES.md` § M4).
- **What is already known:** on this card `decode_` passes 36 of 36 and `canonicaliser_` 28 of 28 (RQ-685). Those are pass
  counts of assertions, the CPU against one backend; this task gives every pair's rate over one input set. The tests in
  `canon_gpu.rs` and `decode_cross_backend.rs` stay as they are and go on asserting.
- A separate task from TASK-M4-28 because a task starts only when its Depends on has merged, and this one must not wait for
  the march. Applied per R-369 (R-686). TASK-M4-28 extends this record.
