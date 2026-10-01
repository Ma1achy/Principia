# TASK-M0-45 — Shard the CI test runs and split the long single tests, so each job is back under ~10.5 min

- **Milestone:** M0
- **Closes:** REQ-SYS-077
- **Depends on:** TASK-M0-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
On PR #96 the warm `ci` job took about 16.5 min and `xtask-ci` 11.7–13.3 min, against R-270's ~10.5 min per job
(R-325). The human accepted that overrun for #96 and made this the next M0 task, at high priority (R-336). The tests
that run alone dominate: `qa_cargo_xtask_alias_runs_deps` at 257 s, and the `qa_TASK-M0-24`, `qa_TASK-M0-25` and
`qa_TASK-M0-26` suites at about 100 s each. The nextest runs of `ci` and `xtask-ci` are sharded across parallel jobs,
and since a shard cannot run one test in parallel, those long tests are split into shorter ones, so each CI job's warm
wall-clock time is ~10.5 min or less.

## References
- `decisions.md` § "R-336 — #96's CI overrun is accepted; TASK-M0-45 shards nextest and splits the long single tests *(amends R-270, R-290)*"
- `decisions.md` § "R-325 — CI: the GPU kernel build and its tests run in their own parallel job; ≤ ~10.5 min per job *(amends R-301)*"
- `decisions.md` § "R-270 — TASK-M0-33: qa's one-round exception is granted; the fixture-pool cost is sent back *(amends R-231)*"
- `decisions.md` § "R-231 — After TASK-M0-22, one task speeds up the suite: nextest, stable fixtures, injectable spawn timings"
- `decisions.md` § "R-235 — `qa_cargo_xtask_alias_runs_deps` uses the listing-only form of `cargo xtask ci`; the controls get their own CI job *(closes RQ-150)*"
- `decisions.md` § "R-266 — "Require branches to be up to date" stays off; bypassing is not allowed *(amends HUMAN_SETUP §2)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-335 — qa may narrow `qa_TASK-M0-22_r235.rs`'s `if:` check to the job's own `if:` *(closes RQ-186; amends R-290)*"
- `decisions.md` § "R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only *(amends R-285, R-320)*"

## Deliverables
- `.github/workflows/ci.yml`: the nextest runs of the `ci` and `xtask-ci` jobs run as parallel shards (nextest's
  `--partition`), the shards together running every test the unsharded run did, in both feature sets. Each shard job
  keeps its job's caches and their R-326 save rule, and its key names the shard's job (R-285).
- `.config/nextest.toml` and `xtask/tests/nextest.rs`'s listing check follow the shards, so the CI test steps together
  still list every test `cargo test --workspace -- --list` lists (REQ-VAL-165).
- The splits, made by qa in this task's qa commit (R-336): `qa_cargo_xtask_alias_runs_deps` in
  `xtask/tests/qa_TASK-M0-01.rs`, and the `qa_TASK-M0-24`, `qa_TASK-M0-25` and `qa_TASK-M0-26` suites in
  `crates/validation/tests/`, into tests each short enough not to hold a shard over its target. Every assertion and
  its negative control is kept (R-176), and `qa_cargo_xtask_alias_runs_deps`'s listing-only form stays (REQ-VAL-166).
- The PR shows each CI job's warm wall time, and a cold one, against ~10.5 min, and names the new jobs to add to
  branch protection's required checks.

## Acceptance tests
- Review checklist (code, qa) — `ci.yml` runs the `ci` and `xtask-ci` nextest runs as parallel shards; the shards'
  test lists together equal the unsharded run's, both feature sets (REQ-SYS-077, R-336).
- `cargo nextest run -p xtask --test nextest` — the listing check passes over the sharded steps (REQ-VAL-165, REQ-SYS-077).
- `cargo nextest run -p xtask --test qa_TASK-M0-22_r235` — the controls job is still beside the tests and cannot be skipped or pass
  with a finding (REQ-VAL-166).
- Review checklist (code) — each split moves tests and controls without weakening an assertion; the PR lists each `M`
  and `D` line of qa's commit with its reason (R-290, R-336).
- CI log on the PR head — each CI job's warm wall time is ~10.5 min or less; a job still over it is named in the PR
  and goes back to the human, not accepted silently (REQ-SYS-077, R-325).

## Notes
- High priority: the next M0 task to start once TASK-M0-14 (PR #96) merges (R-336).
- The long tests' files are qa's, and each has implementer commits, so R-290 alone lets neither qa nor the implementer
  change them. R-336 lets qa make the splits in this task's qa commit (applied per R-204 — veto?), as R-335 lets qa
  narrow `qa_TASK-M0-22_r235.rs`. The orchestrator's R-237 check accepts `M` on those files, and `A` for the files
  split from them, in that commit. The implementer does not edit them.
- `qa_TASK-M0-22_r235.rs` reads the `xtask-ci` job's structure, and R-335's exception covers only its `if:` check. If
  sharding would make any other of its checks fail, that goes to REVIEW_QUEUE before the file changes (R-290).
