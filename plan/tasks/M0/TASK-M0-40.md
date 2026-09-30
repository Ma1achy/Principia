# TASK-M0-40 — Presence means visible, and a review re-runs the stale reviews check

- **Milestone:** M0
- **Closes:** REQ-TOOL-136, REQ-SYS-072
- **Depends on:** TASK-M0-20, TASK-M0-37
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
Two small follow-ups from the human's rulings of 30 Sep. The screenshot runner's presence check counts every AccessKit
name egui produced, so a control clipped out of view passes (R-275). And a `pull_request_review` run of
reviews-complete doesn't clear the failed `pull_request`-event run for the same head, so a merge waits on a manual
re-run (R-276).

## References
- `decisions.md` § "R-275 — A control clipped out of the visible surface isn't present *(closes RQ-167)*"
- `decisions.md` § "R-276 — Four follow-ups: the r217 flake, M0-06's wording, conversation resolution, reviews re-run on each review"
- `decisions.md` § "R-266 — "Require branches to be up to date" stays off; bypassing is not allowed *(amends HUMAN_SETUP §2)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
- `decisions.md` § "R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*"

## Deliverables
- `xtask/src/screenshot.rs` — presence counts a control only if its AccessKit node's rect intersects the visible surface (R-275); the report names a control that is in the tree but clipped.
- `.github/workflows/reviews.yml` — on `pull_request_review`, after its own check, the job re-runs the latest `pull_request`-event reviews run for the same head (it needs `actions: write`), so one passing review turns both check suites green.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask screenshot` — a control laid out below a 120×20 surface fails presence naming it as clipped; the same control inside the surface passes (REQ-TOOL-136).
- On this task's own PR: after the last named approval, both reviews-complete runs for the head pass and mergeStateStatus is CLEAN with no manual re-run (REQ-SYS-072).

## Notes
- The reviews change is applied per R-204 — veto?: `reviews.yml` already triggers on `pull_request_review` (R-276's flag); what blocks a merge is the earlier failed `pull_request`-event run, which is a separate check suite.
