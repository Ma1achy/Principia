# TASK-M0-37 — `reviews-check`: qa's test commits keep an approval, and a PR naming no task passes

- **Milestone:** M0
- **Closes:** REQ-SYS-068
- **Depends on:** TASK-M0-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~120 lines

## Goal
Two amendments to `reviews-complete` (R-175): a role's APPROVE on an earlier commit still counts on the head when every later commit is qa's own `qa: tests for <TASK-id>` commit adding files only under qa's paths (R-260), and a PR whose title names no task passes, printing "no task, no named reviewers" (R-261).

## References
- `decisions.md` § "R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*"
- `decisions.md` § "R-237 — qa's commit may add files under `xtask/tests/`"
- `decisions.md` § "R-260 — qa's approval carries over its own test commit *(amends R-175)*"
- `decisions.md` § "R-261 — A PR whose title names no task passes `reviews-complete` *(amends R-175)*"

## Deliverables
- `xtask/src/reviews_check.rs` — reads the commits after a role's APPROVE (`gh api repos/{owner}/{repo}/pulls/N/commits` and each commit's files) and counts the approval on the head when every later commit's message starts `qa: tests for <TASK-id>` and it only adds files under `crates/*/tests/`, `xtask/tests/` or `fixtures/`; a title with no task id passes with "no task, no named reviewers".
- `xtask/tests/fixtures/reviews_*.json` — an approval followed only by a qa test commit; followed by a qa-titled commit that modifies a file; followed by one that adds outside qa's paths; followed by another commit; a title naming no task.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask reviews_check` — the qa-commit fixture passes; the modifying, outside-path and other-commit fixtures fail naming the role; the no-task title passes printing "no task, no named reviewers" (REQ-SYS-068).

## Notes
- R-260 and R-261 amend R-175 and REQ-SYS-066; TASK-M0-36 built the check as the task file then said.
