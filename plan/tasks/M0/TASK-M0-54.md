# TASK-M0-54 — `ci-gate` gathers `ci-release` (R-809)

- **Milestone:** M0
- **Closes:** REQ-TOOL-223
- **Depends on:** TASK-M0-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~100 lines

## Goal
The owner vetoed D-11 of the report of 5 Nov 2026: "`ci-gate` doesn't gather `ci-release`, so I read that job by eye before each merge". `ci-release` joins the jobs the required check `ci` gathers, so a failing release test fails the check and the read by eye ends.

## References
- `decisions.md` § "R-809 — `ci-gate` gathers `ci-release`: reading that job by eye before each merge is vetoed *(vetoes D-11 of the report of 5 Nov 2026)*"

## Deliverables
- `.github/workflows/ci.yml`: `ci-release` in `ci-gate`'s `needs`, and the comments on `ci-release-archive` and `ci-release` that say they aren't gathered corrected.
- The gate's skip rule: a skip passes only on a docs or data change, where `ci-release` and its archive don't run. A failed `ci-release-archive` skips `ci-release`, and that skip fails the gate.
- `xtask/tests/ci_changes.rs`: its model of `ci-gate`'s `needs` follows, with `ci_gate_gathers_release` and its controls. Any qa file that models the same list changes in this task's qa commit (R-290).

## Acceptance tests
- `cargo test -p xtask ci_gate_gathers_release` — each arm of REQ-TOOL-223's verify (REQ-TOOL-223).
- `cargo test -p xtask --features controls ci_gate_gathers_release` — the negative controls (R-176).
- A merge-candidate CI run on the PR's head shows `ci` gathering both `ci-release` shards.

## Notes
- The quick tier is untouched: `ci-gate` doesn't run on it (`if: needs.changes.outputs.quick != 'true'`).
- Until this merges, the orchestrator goes on reading `ci-release` by eye before each merge, so the job isn't left unread.
- Added by R-809 (5 Nov 2026). This changes CI's behaviour, so the rulings PR that records the veto changes no workflow.
