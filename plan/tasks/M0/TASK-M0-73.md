# TASK-M0-73 — Mutation testing off the merge path: the PC's weekly whole-repo sweep, its backlog file, the gate check, and the job queue refusing mutation runs (R-1031)

- **Milestone:** M0
- **Closes:** REQ-TOOL-246, REQ-TOOL-247, REQ-TOOL-248, REQ-TOOL-249
- **Depends on:** TASK-M0-75
- **Needs (earlier milestones):** none
- **Reviewers:** code
- **Pitfalls:** none
- **Size:** judged by the reviewers alone (R-1031 point 5); a workflow job and its scripts, two small xtask commands and
  their tests, and a refusal in the job queue

## Goal
The owner ruled on 17 Nov 2026 that mutation testing leaves the merge path (R-1031). No pull request waits on a
mutation run; mutation testing becomes a whole-repo sweep on the Windows PC's WSL2 distro, weekly and on request by
`workflow_dispatch`, in the background, yielding to the PC's GPU test runs, a measurement, never a required check and
never red `main`; each sweep's survivors and timeouts go into one backlog file, compared against the last sweep's so
that only new ones are listed, and are triaged into small test-only tasks; a milestone gate needs the latest sweep's
survivors in that milestone's code triaged; the Mac runs no mutation jobs. R-1031's record pull request amended the
rulings and workflow docs. This task builds the machinery and the guard that keeps the merge path and the Mac clear.

## References
- `decisions.md` § "R-1031 — Mutation testing leaves the merge path: no pull request waits on a mutation run; mutation testing becomes a whole-repo sweep on the PC's WSL2 distro, weekly and on request, in the background, yielding to the PC's GPU test runs, a measurement, never a required check and never red `main`; each sweep's new survivors and timeouts go into one backlog file and are triaged into small test-only tasks; a milestone gate needs the latest sweep's survivors in its code triaged; the mutant-count split rule no longer splits tasks; the Mac runs no mutation jobs *(the owner's ruling of 17 Nov 2026, outside the reply template; supersedes R-439; amends R-196, R-202, R-302, R-348, R-388, R-656, R-658, R-676, R-677, R-904, R-905, R-909, R-946, R-1014, R-1016 and R-1017; read with R-402, R-903, R-906 and R-912)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-859 — From the anchor move, the Windows runner may run, as named measurements and never as required checks: the GPU suites (Vulkan now, DX12 from Phase 2) nightly and on pushes to `main`, gpu-idle, the weekly proof and the 7 Nov heavy-work comparison; benchmarks of record stay on the Mac, Windows numbers a separate named series; moving required or heavy CI work to the PC is not approved and returns through `/rulings-report`; the runner's security rules are unchanged *(refines R-186, R-656 and R-711)*"
- `decisions.md` § "R-658 — Heavy commands run only through a local job queue the orchestrator owns, `cargo xtask job`: agents submit them and wait, never run them directly; classes merge, fix, build and peek, first in, first out within each; one runner at a fixed concurrency, each job with cargo's jobs and test threads at the core count; jobs time out, stale locks expire and a new runner recovers a dead one's jobs; heavy processes outside the queue are logged; sccache measured again; mutants scoped to the diff and run through the queue *(supersedes R-228)*"
- `decisions.md` § "R-202 — A surviving mutant fails the per-PR job unless it is a listed, justified equivalent *(closes RQ-139)*"
- `decisions.md` § "R-903 — A hung mutant is stopped in minutes: each test gets its own limit in a nextest mutants profile, 3 times its measured baseline, with named overrides for known-slow tests, and a test killed so counts as a timeout, caught; listed equivalent mutants are skipped up front and run by the nightly; the 300-minute limit and 8 shards stay *(amends R-712 and R-348; refines R-202)*"
- `decisions.md` § "R-1016 — Mutation runs move onto the PC once a measurement made first shows them faster there; for mutation runs only, the placements of R-905, R-909, R-912 and R-946 return from R-1014's hold, every condition they set still standing; the rest of RQ-825 stays open and held *(on RQ-825, which stays open)*"
- `decisions.md` § "R-196 — Mutation testing joins the QA gate"
- `decisions.md` § "R-712 — The per-mutant timeout is set from the measured baseline test time, at about 3 times it; 8 shards are kept; a timed-out mutant counts as caught and is listed; #360's 135 unreached mutants go to the nightly run, and the same rule applies to #387 if it hits the limit *(amends R-348, R-376 and R-402 for REQ-VAL-180)*"
- `decisions.md` § "R-906 — CI levers: generated plan files are no longer committed, CI regenerating and checking them; lavapipe's GPU tests may run at a smaller size only once it is shown to reach the same branches and stages, the full size running on the RTX nightly; a test killed by its limit is listed as "timed out (not a survivor)"; the control-inventory tests run once per CI run; the nightly mutation run becomes incremental by fingerprint, every mutant re-tested at least weekly; larger runners: no *(amends R-292 and R-402; refines R-683, R-712, R-718 and R-903)*"
- `decisions.md` § "R-912 — Windows containment, RQ-751: option (b) now, the Job Object proof passing with its two process-visibility gaps accepted and the job-queue mutants staying on GitHub's runners; option (c), CPU tests and mutants in a WSL2 distro under `contain.sh`, built in parallel and never blocking, replaces (b) only once each of its five requirements is proven, the job-queue mutants then free to move to Windows; GPU tests stay native *(closes RQ-751; amends R-905)*"
- `decisions.md` § "R-1004 — RQ-807, the WSL image: (3) and (4), and (i); the cache on E: stays as built, the hosted `wsl-image` job keeps its build in the Actions cache keyed on the recipe, restored on a hit and built only on a miss, proof 5 unchanged, and no credential on the PC *(closes RQ-807; amends R-285 and R-326)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- **The sweep (REQ-TOOL-246).** In `win-gpu.yml` (or a workflow of its own on R-711's runner, its pull request saying
  which and why), a job that runs `cargo mutants` over the workspace at a commit of `main` inside the WSL2 distro under
  `contain.sh`, the way `wsl-cpu` runs today, on a weekly `schedule` and on `workflow_dispatch` only. **Its gate:** it
  runs only under the containment `wsl-cpu` runs under, `contain.sh` in its unprivileged mode, the per-process memory
  cap, the VRAM cap and the canary, dispatch and schedule only, and the signed-commit trust step, at below-normal
  priority overnight, with the 36-hour no-run and failure alert TASK-M0-75 builds first (applied per R-369 in R-1031:
  the sweep must meet R-905's prerequisites in full before it first runs, the dispatched measurement sweep included,
  the conservative reading; it is the owner's own placement, not a per-merge move, so R-946's conditions don't gate it). It
  leaves out the job-queue mutants (`xtask/src/jobq.rs`), which stay on GitHub's runners (R-912), the reason stated,
  and marks in its backlog a survivor that only a GPU test could kill (the distro's GPU share is off). It yields to the
  PC's GPU test runs: a GPU job never waits on it (the mechanism, such as shards short enough to interleave, a
  concurrency rule, or a check that no GPU job is running or queued before each shard, is the pull request's,
  applied per R-369, with its measured effect on a GPU run's start). Its job concludes success whatever it finds; a
  sweep that could not finish says so in its report and artifact. It is added to the runner's named measurements
  (`xtask/tests/ci_self_hosted.rs`) and is never a required check.
- **The alert is TASK-M0-75's (REQ-TOOL-251),** which this task depends on, so it is live on `main` before any sweep
  runs. The sweep's report and artifact say whether it finished its whole run and published its backlog, which is what
  the alert reads.
- **The backlog file (REQ-TOOL-247).** An xtask command that reads a sweep's `mutants.out` and the last completed
  sweep's backlog and writes the new backlog: every survivor and timeout, with file, line, mutation and outcome, those
  new since the last sweep marked new, listed equivalents never new. Where the file lives and how the orchestrator
  fetches it are the pull request's (the PC holds no credential, R-1004, so the sweep publishes it as its artifact).
- **The gate check (REQ-TOOL-248).** An xtask command, `cargo xtask mutants-gate <milestone>` or as named in the pull
  request, that passes only when every survivor of the latest completed sweep in that milestone's code is killed or
  listed in `.cargo/mutants-equivalent.toml` with its reason, and names each one left. How a milestone maps to its code
  is stated and tested.
- **The merge path and the Mac cleared (REQ-TOOL-249).** `cargo xtask job submit` refuses a command that runs
  `cargo mutants`, naming R-1031, and queues nothing; `mutants.yml`'s unused pull-request and merge-ready jobs are
  removed (its header's history kept as a comment or moved to `plan/OPERATIONS.md`); its dispatch-only
  `mutants-fixture` job, which mutates a fixture crate, not the workspace's own code, stays as it is; a test that no job running
  `cargo mutants` can run on a `pull_request`, `push` or `merge_group` event, its workflow's triggers and its own `if:`
  read together (so `wsl-cpu`'s dispatch-only job and a schedule-or-dispatch sweep in `win-gpu.yml` pass), and that the agents' briefs and the workflow docs ask for no mutation
  run before review, a push or a merge.
- `plan/OPERATIONS.md`: how to dispatch a sweep, where its backlog is, and how triage and the gate check run.

## Acceptance tests
- `cargo test -p xtask --test ci_mutants_sweep` and `--features controls` (REQ-TOOL-246, REQ-TOOL-249).
- `cargo test -p xtask --test mutants_backlog` and `--features controls` (REQ-TOOL-247).
- `cargo test -p xtask --test mutants_gate` and `--features controls` (REQ-TOOL-248).
- `cargo test -p xtask --test jobq` and `--features controls`: the refusal and the rest of the queue unchanged
  (REQ-TOOL-249).
- `cargo test -p xtask --test ci_self_hosted` and the runner's other workflow tests, with and without
  `--features controls`: unchanged rules hold.
- `python3 plan/check_plan.py` passes.
- On the PC (a measurement, not a gate): one dispatched sweep's wall time, mutant counts and backlog, reported in the
  pull request, and a GPU run's start time while it ran.

## Notes
- Entered 17 Nov 2026 by R-1031's record pull request (the owner: "Record this, amend the rulings and workflow docs it
  changes"), applied per R-369: task scoping, CI tooling within the ruling's bounds.
- **Not in scope:** the hosted nightly full run (`nightly.yml`'s mutants jobs, REQ-VAL-150, restated), which this task
  does not touch: the owner's answer to RQ-836 (R-1032, 17 Nov 2026) ends it once this task's sweep is live, and
  TASK-M0-74 removes it and adds the job-queue code's small hosted mutation run; `wsl-cpu`'s dispatched in-diff workload, which stays as it is,
  for use on request, off the merge path (applied per R-369 in R-1031). *Was (17 Nov 2026, until R-1032):* "the hosted
  nightly full run (`nightly.yml`'s mutants jobs, REQ-VAL-150), which this task does not touch, RQ-836 asking the owner
  whether it continues".
- The sweep fixes nothing (R-1031 point 3); its findings are triaged into small test-only tasks by the orchestrator.
- *19 Nov 2026 (R-1034, applied by TASK-M0-76; logged in `plan/DECISIONS-LOG.md`):* qa's independent tests are asked only of physics-critical tasks, those naming physics among their reviewers, and this task's list names no physics, so qa no longer reviews it. *Was (Reviewers):* "code, qa".
