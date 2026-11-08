# TASK-M0-56 — The heavy-work comparison: the Windows runner against hosted runners (R-859)

- **Milestone:** M0
- **Closes:** REQ-TOOL-225
- **Depends on:** TASK-M0-55
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The owner's order of 7 Nov 2026, which R-859 allows from the anchor move: is the Windows runner faster for the heavy
work? On one task branch, at one commit, run the GPU suites (`codegen_selftest_pack_unpack_kernel_gpu` among them)
and one mutant shard holding `projection_parity.rs` or `drift.rs` on the PC and on hosted runners, and compare wall
time and results, mutant for mutant. It is a measurement, never a gate (R-649); nothing required moves. The result and
a fallback design go to the next `/rulings-report` as the one-way question of moving required work.

## References
- `decisions.md` § "R-859 — From the anchor move, the Windows runner may run, as named measurements and never as required checks: the GPU suites (Vulkan now, DX12 from Phase 2) nightly and on pushes to `main`, gpu-idle, the weekly proof and the 7 Nov heavy-work comparison; benchmarks of record stay on the Mac, Windows numbers a separate named series; moving required or heavy CI work to the PC is not approved and returns through `/rulings-report`; the runner's security rules are unchanged *(refines R-186, R-656 and R-711)*"
- `decisions.md` § "R-861 — The heavy-work comparison starts by `workflow_dispatch` on its task branch; the runner's security rules are unchanged; the 7 Nov order's "push trigger" was an error *(closes RQ-743; corrects R-859)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-812 — Two-way doors are decided and shown afterwards, by measurement where one can settle it, CI and build tooling among them; one-way doors are asked through `/rulings-report` only while work carries on; when unsure, a door is one-way *(amends R-369)*"
- `decisions.md` § "R-712 — The per-mutant timeout is set from the measured baseline test time, at about 3 times it; 8 shards are kept; a timed-out mutant counts as caught and is listed; #360's 135 unreached mutants go to the nightly run, and the same rule applies to #387 if it hits the limit *(amends R-348, R-376 and R-402 for REQ-VAL-180)*"

## Deliverables
- The criterion, written first: what "clearly faster" means in wall time, and that the results must be identical,
  committed to the PR before the first run of either side and never changed after (R-649, R-812).
- The run on the PC: a measurement job in its own workflow, keeping every runner security rule (R-711 items 1, 5 and
  6; the hook's allow-listed events, no fork run, an idle runner with nothing alongside), started by `workflow_dispatch`
  on the task branch, dispatched from the Mac (R-861). The guard test, `xtask/tests/ci_self_hosted.rs`, admits it by
  name, with no exception to its trigger checks.
- The same work on hosted runners at the same commit: the GPU suites, and the same mutant shard's mutants.
- `cargo xtask heavy-compare`: reads both sides' test outcomes and mutant outcomes and reports each side's wall time,
  each test whose outcome differs, and each mutant whose outcome (caught, missed, unviable, timed out, R-712) differs or
  that one side lacks.
- The record: both runs' ids, the commit, the driver version, the wall times, the comparison's output, and the verdict
  against the criterion, with R-719's hardware-suspect label on any Windows-only, non-reproducing failure.
- A `REVIEW_QUEUE.md` entry for the next `/rulings-report`: the numbers and the fallback design, the PC running the
  heavy jobs first and the required check taking the PC's result if it arrives within a time limit, the hosted
  runners running as now otherwise, so the PC is never a single point of failure. It is filed whatever the verdict; if
  the PC isn't clearly faster with identical results, the entry says so and proposes no move.

## Acceptance tests
- `cargo test -p xtask heavy_compare` — identical outcomes report identical; the comparison's arms (REQ-TOOL-225).
- `cargo test -p xtask --features controls heavy_compare` — the negative controls: one mutant moved from caught to
  missed, one test passing on one side only, and a mutant on one side only, each reported as not identical (R-176).
- `cargo test -p xtask --test ci_self_hosted` — the runner's guard checks hold with the comparison's job admitted.
- The run itself, on the PC and on hosted runners, with its record and the criterion's commit preceding both runs, in
  the PR (REQ-TOOL-225).

## Notes
- Starts after the anchor move. The 7 Nov order's "via push trigger" was an error of wording (R-861): no push trigger
  on a task branch.
- The verdict moves nothing: required or heavy CI work stays on hosted runners until the owner rules on the queue
  entry (R-859).
- Lavapipe on the hosted side and NVIDIA Vulkan on the PC are different backends; a test whose outcome differs is a
  finding of the comparison, handled as R-719 says, and no tolerance or test changes to make the two agree (R-649).
