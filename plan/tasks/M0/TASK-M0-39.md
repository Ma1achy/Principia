# TASK-M0-39 — The r217 controls stop flaking: the grandchild gets its own process group at spawn

- **Milestone:** M0
- **Closes:** REQ-VAL-175
- **Depends on:** TASK-M0-26, TASK-M0-33
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~80 lines

## Goal
Two controls in `crates/validation/tests/qa_TASK-M0-26_r217.rs` sometimes fail to trip under CI load, so `xtask-ci`
goes red until re-run (PRs #71 and #74, 30 Sep). The out-of-group grandchild leaves the process group only once perl
runs `setpgrp`, and the 1 s timeout can fire before that, so SIGTERM still reaches it and the control passes (R-276).

## References
- `decisions.md` § "R-276 — Four follow-ups: the r217 flake, M0-06's wording, conversation resolution, reviews re-run on each review"
- `decisions.md` § "R-217 — TASK-M0-26's size accepted; a timed-out child's whole process group dies; the timeout is 300 s provisional *(amends R-214)*"
- `decisions.md` § "R-227 — The two remaining latent races are fixed now, in their own task"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"

## Deliverables
- The r217 grandchild is spawned into its own process group at spawn (for example `std::os::unix::process::CommandExt::process_group(0)` on the grandchild's own spawn), not by perl's `setpgrp`, so it is out of the group before the timeout can fire.
- The edit to qa's merged `qa_TASK-M0-26_r217.rs` is limited to that, under R-227's exception, with qa reviewing.
- A 50-run soak of the two tests and their controls on ubuntu-latest (the stand-in-soak workflow, or a `measure/` branch, R-272), with the log kept on the PR.

## Acceptance tests
- CI log on the PR head: `qa_r217_a_grandchild_holding_the_output_is_gone_after_the_timeout` and `qa_r217_a_hanging_child_and_its_grandchild_are_gone_after_the_timeout`, with their controls, pass 50 consecutive runs under the parallel suite on ubuntu-latest (REQ-VAL-175).
- `cargo xtask controls` — both controls trip (REQ-VAL-175).

## Notes
- Depends on TASK-M0-33, which gives these tests a short grace (R-231), so the fix lands on its timings.
- qa traced the cause on PR #71 (review at 98dcdf4): `qa_TASK-M0-26_r217.rs:137-144`.
