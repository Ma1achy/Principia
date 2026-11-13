# TASK-M0-61 — Windows runner: R-986 concurrency, one re-run, time limits

- **Milestone:** M0
- **Closes:** REQ-TOOL-230, REQ-TOOL-231, REQ-TOOL-232
- **Depends on:** TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines, the tests and saved logs included

## Goal
The owner's goal of 12 Nov 2026 for the Windows runner's CI rules the 85-minute run (R-986): "The 85-minute PC run:
merges to main, nightly and dispatch only, with a newer run cancelling an older one; never branch pushes. (Ruling.)"
And its item 5 (R-988): "Unclear stop: one automatic re-run; hang: cancelled by time limit; both logged." This task
builds both on R-711's runner: `win-vulkan` cancels an older run when a newer one starts, on `main` too; a hosted
`win-retry` job re-runs, once, a job on the PC that stopped on a runner-level fault, and logs it; every job on the PC,
and every step of one that builds or runs tests, ends at 1.5 times its measured time, the hang logged and never re-run.

## References
- `decisions.md` § "R-986 — The 85-minute run on the Windows PC, `win-vulkan`, runs on merges to `main`, nightly and by dispatch only, never on a branch push, and a newer run of it cancels an older one, on `main` too *(the owner's goal of 12 Nov 2026 for the Windows runner; amends R-656 for this job; read with R-711, R-859, R-946 and R-970)*"
- `decisions.md` § "R-988 — A job on the Windows PC that stops unclearly is re-run once, automatically, by a hosted job; a hung one ends at a time limit of 1.5 times its measured time; both are logged *(the owner's goal of 12 Nov 2026, item 5; amends R-859's security rules, the owner's job rules of 5 Nov 2026, for this re-run; extends R-714; refines R-970; read with R-711 and R-968)*"
- `decisions.md` § "R-714 — A failed CI job is retried automatically only on a runner-level fault, in at most two retries, never on a real test failure or a flake, the classifier a script tested on saved logs; every CI speed-up step reports disk use and must reduce or hold it; lands in `plan/OPERATIONS.md` § "Merging""
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-859 — From the anchor move, the Windows runner may run, as named measurements and never as required checks: the GPU suites (Vulkan now, DX12 from Phase 2) nightly and on pushes to `main`, gpu-idle, the weekly proof and the 7 Nov heavy-work comparison; benchmarks of record stay on the Mac, Windows numbers a separate named series; moving required or heavy CI work to the PC is not approved and returns through `/rulings-report`; the runner's security rules are unchanged *(refines R-186, R-656 and R-711)*"
- `decisions.md` § "R-970 — A CI run clearly hung or stuck because of a bug is caught and cancelled, on `main` too; every other run on `main` is still never cancelled *(amends R-656)*"

## Deliverables
- `.github/workflows/win-gpu.yml`: `win-vulkan`'s `cancel-in-progress: true`, every other job's left `false`; the
  header's concurrency paragraph and its "No job here is gathered, retried or read by any gate" line rewritten for
  R-986 and R-988; every job's `timeout-minutes`, and every build or test step's, at 1.5 times its measured time.
- `.github/workflows/runner-proof.yml`: its header line likewise, and its job's limit by the same rule.
- `.github/workflows/win-retry.yml` (new): the hosted `win-retry` job, `workflow_run` on `win-gpu` and `runner-proof`
  alone, never a `pull_request` or `pull_request_target` trigger; `actions: write` for the one re-run; the repository
  guard; it checks out `main` and runs no code from the run it reads, as `retry.yml` does.
- `.github/scripts/win_retry.py` (new): the classifier, runner-level fault, hang, cancelled by a newer run, never taken,
  or a failure of the job's own; its decision and the log lines it rests on written to the run's summary.
- The measured times, committed as a table with their run ids before any limit is set (R-649), the file the limits'
  test reads.
- `xtask/tests/ci_self_hosted.rs`: the `cancels` control flipped (a `win-vulkan` that doesn't cancel fails), and a new
  control for another job that does; `check_not_gathered` admitting `win-retry` as the one reader of the runner's
  workflows, the `retried` control (`retry.yml` following `win-gpu.yml`) still failing; the limits check with its
  controls.
- `xtask/tests/ci_win_retry.rs` (new) and `fixtures/win_retry/` (saved logs), with their controls (R-176). Any qa file
  that models the old behaviour changes in this task's qa commit (R-290), each change named on the pull request.
- `plan/OPERATIONS.md` § "The Windows runner's work (R-859)": the built job and the table named.

## Acceptance tests
- `cargo test -p xtask --test ci_self_hosted` — REQ-TOOL-230's and REQ-TOOL-232's verify, and REQ-TOOL-231's part:
  `win-vulkan` cancels and no other job does, the push trigger is `main`'s alone, every limit is 1.5 times its measured
  time, and `win-retry` is the one reader of the runner's workflows (REQ-TOOL-230, REQ-TOOL-231, REQ-TOOL-232).
- `cargo test -p xtask --features controls --test ci_self_hosted` — the negative controls (R-176).
- `cargo test -p xtask --test ci_win_retry` — REQ-TOOL-231's verify, over the saved logs (REQ-TOOL-231).
- `cargo test -p xtask --features controls --test ci_win_retry` — the negative controls (R-176).
- The measured-times table, each job's and step's runs listed with their ids, shown on the pull request before the
  limits it sets.

## Notes
- New (12 Nov 2026), from R-986 and R-988, one task for both (applied per R-369: task scoping), since both change the
  runner's workflows and the one guard test. The rulings pull request that records them changed no workflow.
- R-988 is a reading to confirm: the owner's item 5 was not labelled a ruling, and it contradicts the job rules'
  "No job here is gathered, retried or read by any gate". If the owner reads it otherwise, this task follows the ruling.
- `workflow_run` takes effect only from the default branch, so `win-retry` first runs after the merge. After merge,
  confirm it on the first real unclear stop and report it, as R-714 asks of `retry.yml`.
- A re-run keeps its run's event; whether the PC's hook accepts a re-run's events and what `github.actor` reads on one
  (R-711 item 6) is confirmed on that first re-run. Any PC-side change goes through the owner, one message at a time.
- A re-run of an older `win-vulkan` would cancel the newer one (R-986), so `win-retry` re-runs a job only while no
  newer run of it has started.
