# TASK-M0-03 — Process templates and `cargo xtask pr-check`

- **Milestone:** M0
- **Closes:** REQ-SYS-006, REQ-VAL-001, REQ-VAL-005, REQ-VAL-008
- **Depends on:** TASK-M0-01, TASK-M0-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-4.5
- **Size:** ~480 lines estimated; built at ~678 counted, so `reviews-check` moved to TASK-M0-36 (applied per R-204), leaving ~370

## Goal
Every PR opens on a template that asks what the process rules ask, and CI checks that the PR description answers it: for a design PR, the four drift questions of philosophy §6 (which path is it for; where does each constant come from; could the measurement have failed; does the instrument still say where it stops knowing); for an investigation PR, the `principia_01_pitfalls.md` entry added or updated (observed / assumed / actual / measurement) or the reason none applies (pitfalls §4.5); for a validation record, each error meter's integrated quantity (philosophy §4.5) and each discriminator's dependency on termination (pitfalls §3). The check verifies the answers are present; the physics reviewer judges them. `reviews-complete` (R-175) is TASK-M0-36's.

## References
- `docs/read_first/principia_00_philosophy.md` § "6. How to tell if a decision is drifting"
- `docs/read_first/principia_00_philosophy.md` § "4.5 Verify against the formulation, not just the data"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "4.5 THIS WAS ALREADY ON RECORD"
- `decisions.md` § "Porting rule — a port adds, it never removes a decision"
- `decisions.md` § "R-175 — The reviewers are agents, and a CI check counts their verdicts *(closes H3)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"
- `decisions.md` § "R-180 — pr-check is item-level *(closes A5)*"

## Deliverables
- `.github/pull_request_template.md` — sections: Task and Closes (each requirement id → acceptance command → output); Design questions (the four drift questions); Investigation (pitfalls entry, or why none applies); Validation record, one line per item (R-180): `- meter: <name> — <the quantity the occupant integrates>` and `- discriminator: <name> — <its dependency on termination; recomputed on fully integrated runs where it depends>`; Removed lines (for commits touching `docs/`, `decisions.md` or `plan/`).
- PR labels `design`, `investigation`, `validation` select which sections are mandatory.
- `xtask/src/pr_check.rs` — `cargo xtask pr-check [--event PATH]`: reads the pull_request event JSON (`$GITHUB_EVENT_PATH` in CI) and fails naming every mandatory section that is missing or empty for the PR's labels. In a Validation record it parses each `- meter:` and `- discriminator:` line and fails naming each item whose statement after the dash is empty (R-180). A CI step on `pull_request` events of types `opened`, `edited`, `synchronize`, `reopened`, `labeled` and `unlabeled`, so that it re-runs when a reviewer relabels.
- `xtask/tests/fixtures/pr_*.json` — complete and incomplete bodies per label.
- Negative controls for this task's xtask tests, registered with `negative_control!(test_name, "description", control)` through `xtask`'s dev-dependency on `crates/validation` (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask pr_check_design` — a `design` body missing one of the four drift answers fails naming the question; a complete body passes (REQ-SYS-006).
- `cargo test -p xtask pr_check_validation_meter` — a `validation` body with an error meter that does not state the quantity the occupant integrates fails; review checklist (physics §3): a meter on a non-dynamical quantity (COM drift under an integrator of relative coordinates) is rejected (REQ-VAL-001).
- `cargo test -p xtask pr_check_validation_discriminator` — a `validation` body whose discriminator does not state its dependency on termination fails; review checklist (physics §3): a discriminator that depends on the tested termination (d_min, t_end-truncated statistics) is rejected or recomputed on fully integrated runs (REQ-VAL-005).
- `cargo test -p xtask pr_check_investigation` — an `investigation` body with neither a pitfalls entry nor a stated reason fails (REQ-VAL-008).

## Notes
- The human merges, or a merge bot does once `ci` and `reviews-complete` are green (R-175); branch protection requiring both is human setup (`plan/HUMAN_SETUP.md`).
- The labels are the author's to set; a missing label is a review finding (the reviewer re-labels and the check re-runs).
- No error meter or discriminator exists in M0; these requirements govern every later validation PR, and this task installs the check they are held by.
- Applied per R-204 (a split within the size budget): built at ~678 counted lines (R-211, R-223, R-225), the task is split in two; `reviews-check` and REQ-SYS-066 move to TASK-M0-36 (~305).
