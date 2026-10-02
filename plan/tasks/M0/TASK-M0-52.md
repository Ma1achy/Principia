# TASK-M0-52 — The `ci` shards share one test build: a nextest archive built once, so no shard recompiles the tests

- **Milestone:** M0
- **Closes:** REQ-SYS-078
- **Depends on:** TASK-M0-45
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
TASK-M0-45 runs the `ci` job's nextest run as 4 shards of `--partition hash:<k>/4` (R-336, R-366). Each shard spends
~5.5 min on setup and compiling before its first test, so splitting the tests further barely helps, and the human
accepted #117's shard times with ~10.5 min per job kept as the target (R-372). The follow-up R-372 names: build the
tests once and share them. One job builds the workspace's tests with `cargo nextest archive`, once per feature set the
shards run, and uploads the archive; each `ci` shard downloads it and runs its slice from it with `--archive-file`,
compiling no test. The shards together still run every test the unsharded run did.

## References
- `decisions.md` § "R-372 — #117's shard times are accepted, ~10.5 min staying the target; the shards share one test build, as a follow-up task *(amends R-336 and R-366 as they apply to #117)*"
- `decisions.md` § "R-366 — qa updates TASK-M0-45's pinned CI step lines; nextest shards by `hash:<k>/4`; #117's other items stand *(closes RQ-197; amends R-290, R-336 and R-360 as they apply)*"
- `decisions.md` § "R-336 — #96's CI overrun is accepted; TASK-M0-45 shards nextest and splits the long single tests *(amends R-270, R-290)*"
- `decisions.md` § "R-325 — CI: the GPU kernel build and its tests run in their own parallel job; ≤ ~10.5 min per job *(amends R-301)*"
- `decisions.md` § "R-231 — After TASK-M0-22, one task speeds up the suite: nextest, stable fixtures, injectable spawn timings"
- `decisions.md` § "R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only *(amends R-285, R-320)*"
- `decisions.md` § "R-285 — CI caches only the cargo registry and the fixture pool, with per-job keys"
- `decisions.md` § "R-266 — "Require branches to be up to date" stays off; bypassing is not allowed *(amends HUMAN_SETUP §2)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `.github/workflows/ci.yml`: a job that builds the tests once per feature set the `ci` shards run, as a nextest archive
  (`cargo nextest archive --workspace`, with the shards' features and the `ci` profile's settings), and uploads each
  archive as a workflow artifact. Each `ci` shard `needs:` it, downloads its archive, and runs
  `cargo nextest run --archive-file <archive> --partition hash:${{ matrix.shard }}/4`, with no build or test compile of
  its own. The doctests, which nextest does not run, stay in shard 1. Each job's cache keys name its job, with R-326's
  save rule (R-285).
- `xtask/tests/nextest.rs`: the listing check (R-231) and `nextest_ci_shards_together_list_every_test` read the
  archive-based steps, so the shards' test lists together still equal the unsharded run's, both feature sets, each
  check with its negative control.
- The PR shows each `ci` shard's warm wall time, and the archive job's, against ~10.5 min, and names any job to add to
  or remove from branch protection's required checks (R-266), for the human to change.

## Acceptance tests
- `cargo nextest run -p xtask --test nextest` — the listing check and the shards' union check pass over the
  archive-based steps, each with its negative control (REQ-SYS-078).
- Review checklist (code, qa) — `ci.yml` builds the archive once per feature set in one job, every `ci` shard runs from
  it with no `cargo build` or test compile of its own, and the doctests still run in shard 1 (REQ-SYS-078).
- CI log on the PR head — each `ci` shard's warm wall time, and the archive job's, against ~10.5 min; a job still over
  it is named in the PR (REQ-SYS-078, R-325, R-372).

## Notes
- Applied per R-369: R-372 left the follow-up's how and when to the orchestrator. It is this task, at normal priority,
  started once TASK-M0-45 (PR #117) merges, beside the M0 tasks still open, before the M0 gate.
- `xtask-ci`'s shards run `cargo xtask controls`, whose own `cargo test` builds are not nextest's; the archive does not
  cover them, and this task leaves them as they are.
- The qa test files that match the `ci` job's step lines (`xtask/tests/qa_TASK-M0-22_r235.rs`,
  `xtask/tests/support/qa_m0_01.rs`, `xtask/tests/qa_TASK-M0-29.rs` and the qa-only ones R-366 lists) may need their
  step matches moved again. Where this task's change forces that, the exception is decided and recorded in the PR
  under R-369, qa changes only those matches, and the code reviewer confirms nothing else changed.
