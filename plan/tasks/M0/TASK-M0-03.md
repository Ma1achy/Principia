# TASK-M0-03 — Process templates and `cargo xtask pr-check`

- **Milestone:** M0
- **Closes:** REQ-SYS-006, REQ-VAL-001, REQ-VAL-005, REQ-VAL-008, REQ-SYS-066
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-4.5
- **Size:** ~480 lines

## Goal
Every PR opens on a template that asks what the process rules ask, and CI checks that the PR description answers it: for a design PR, the four drift questions of philosophy §6 (which path is it for; where does each constant come from; could the measurement have failed; does the instrument still say where it stops knowing); for an investigation PR, the `principia_01_pitfalls.md` entry added or updated (observed / assumed / actual / measurement) or the reason none applies (pitfalls §4.5); for a validation record, each error meter's integrated quantity (philosophy §4.5) and each discriminator's dependency on termination (pitfalls §3). The check verifies the answers are present; the physics reviewer judges them. Beside it, `reviews-complete` (R-175): the reviewers are agents posting from one account, so each posts a PR review headed `VERDICT: APPROVE <role>` or `VERDICT: CHANGES <role>`, and the check passes only when every role the task file names has approved on the latest commit.

## References
- `docs/read_first/principia_00_philosophy.md` § "6. How to tell if a decision is drifting"
- `docs/read_first/principia_00_philosophy.md` § "4.5 Verify against the formulation, not just the data"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "4.5 THIS WAS ALREADY ON RECORD"
- `decisions.md` § "Porting rule — a port adds, it never removes a decision"
- `decisions.md` § "R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*"

## Deliverables
- `.github/pull_request_template.md` — sections: Task and Closes (each requirement id → acceptance command → output); Design questions (the four drift questions); Investigation (pitfalls entry, or why none applies); Validation record (error meters: the quantity the occupant integrates; discriminators: dependency on termination, recomputed on fully integrated runs where it depends); Removed lines (for commits touching `docs/`, `decisions.md` or `plan/`).
- PR labels `design`, `investigation`, `validation` select which sections are mandatory.
- `xtask/src/pr_check.rs` — `cargo xtask pr-check [--event PATH]`: reads the pull_request event JSON (`$GITHUB_EVENT_PATH` in CI) and fails naming every mandatory section that is missing or empty for the PR's labels; a CI step on pull_request events.
- `xtask/tests/fixtures/pr_*.json` — complete and incomplete bodies per label.
- `xtask/src/reviews_check.rs` — `cargo xtask reviews-check [--pr N]`: reads the task id from the PR title, the Reviewers field from `plan/tasks/<Mn>/<TASK-id>.md`, and the PR's reviews (`gh api repos/{owner}/{repo}/pulls/N/reviews`); a role counts as approved when its latest review whose body starts `VERDICT: APPROVE <role>` was submitted on the head commit and no later `VERDICT: CHANGES <role>` exists; fails naming each missing role. `.github/workflows/reviews.yml` runs it as the `reviews-complete` check on `pull_request` and `pull_request_review` events (R-175).
- `xtask/tests/fixtures/reviews_*.json` — review lists: all roles approved on head; one role approved on an older commit; an APPROVE superseded by a later CHANGES; a missing role.

## Acceptance tests
- `cargo test -p xtask pr_check_design` — a `design` body missing one of the four drift answers fails naming the question; a complete body passes (REQ-SYS-006).
- `cargo test -p xtask pr_check_validation_meter` — a `validation` body with an error meter that does not state the quantity the occupant integrates fails; review checklist (physics §3): a meter on a non-dynamical quantity (COM drift under an integrator of relative coordinates) is rejected (REQ-VAL-001).
- `cargo test -p xtask pr_check_validation_discriminator` — a `validation` body whose discriminator does not state its dependency on termination fails; review checklist (physics §3): a discriminator that depends on the tested termination (d_min, t_end-truncated statistics) is rejected or recomputed on fully integrated runs (REQ-VAL-005).
- `cargo test -p xtask pr_check_investigation` — an `investigation` body with neither a pitfalls entry nor a stated reason fails (REQ-VAL-008).
- `cargo test -p xtask reviews_check` — the all-approved fixture passes; the stale-commit, superseded and missing-role fixtures each fail naming the role (REQ-SYS-066).

## Notes
- The human merges, or a merge bot does once `ci` and `reviews-complete` are green (R-175); branch protection requiring both is human setup (`plan/HUMAN_SETUP.md`).
- The labels are the author's to set; a missing label is a review finding (the reviewer re-labels and the check re-runs).
- No error meter or discriminator exists in M0; these requirements govern every later validation PR, and this task installs the check they are held by.
