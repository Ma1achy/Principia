# TASK-M0-36 — `cargo xtask reviews-check` and the `reviews-complete` check

- **Milestone:** M0
- **Closes:** REQ-SYS-066
- **Depends on:** TASK-M0-01, TASK-M0-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~305 counted lines (split from TASK-M0-03, applied per R-204)

## Goal
`reviews-complete` (R-175): the reviewers are agents posting from one account, so each posts a PR review headed `VERDICT: APPROVE <role>` or `VERDICT: CHANGES <role>`, and the check passes only when every role the task file names has approved on the latest commit.

## References
- `decisions.md` § "R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"

## Deliverables
- `xtask/src/reviews_check.rs` — `cargo xtask reviews-check [--pr N]`: reads the task id from the PR title, the Reviewers field from `plan/tasks/<Mn>/<TASK-id>.md`, and the PR's reviews (`gh api repos/{owner}/{repo}/pulls/N/reviews`); a role counts as approved when its latest review whose body starts `VERDICT: APPROVE <role>` was submitted on the head commit and no later `VERDICT: CHANGES <role>` exists; fails naming each missing role. `.github/workflows/reviews.yml` runs it as the `reviews-complete` check on `pull_request` and `pull_request_review` events (R-175).
- `xtask/tests/fixtures/reviews_*.json` — review lists: all roles approved on head; one role approved on an older commit; an APPROVE superseded by a later CHANGES; a missing role.
- Negative controls for this task's xtask tests, registered with `negative_control!(test_name, "description", control)` through `xtask`'s dev-dependency on `crates/validation` (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask reviews_check` — the all-approved fixture passes; the stale-commit, superseded and missing-role fixtures each fail naming the role (REQ-SYS-066).

## Notes
- Split from TASK-M0-03, applied per R-204 (a split within the size budget): TASK-M0-03 was built at ~678 counted lines; its `reviews-check` part (~305) is this task. The code exists on TASK-M0-03's branch and moves here.
- The human merges, or a merge bot does once `ci` and `reviews-complete` are green (R-175); branch protection requiring both is human setup (`plan/HUMAN_SETUP.md`).
