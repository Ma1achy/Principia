# TASK-M6-14 — Symbolic spread S_word: the word-reduction diagnostic and its measurement

- **Milestone:** M6
- **Closes:** REQ-REF-014, REQ-REF-037, REQ-REF-042, REQ-PAY-084, REQ-VAL-087, REQ-VAL-093, REQ-VAL-095
- **Depends on:** TASK-M6-03
- **Needs (earlier milestones):** REQ-PAY-036, REQ-PAY-067, REQ-SCHED-042, REQ-INT-072, REQ-RENDER-051
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~420 lines

## Goal
The third resolve-stage reduction exists: over a footprint's E+1 per-copy words, `S_word` is computed on whole raw reduced words (no pair attribution), shown and measured as a diagnostic and never read by the split (R-99). The task writes the metric into the sampling note (R-72), measures where `S_word` fires relative to outcome impurity and the branch-cut crossing distribution, and proposes the additivity criterion (R-71).

## References
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "Downstream consumers waiting on this contract"
- `docs/notes/principia_sampling_msaa_note.md` § "Symbolic spread — the free-group word as a third resolve-stage reduction"
- `decisions.md` § "R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_scheduler_contract.md` § "Part 8 — Continuous refinement & the live-to-live handoff"

## Deliverables
- Doc change: `docs/notes/principia_sampling_msaa_note.md` § "Symbolic spread — the free-group word as a third resolve-stage reduction" — one metric and its range (REQ-REF-042).
- `crates/engine/src/resolve/s_word.rs` (or the resolve-stage reduction in the kernel): `S_word` per footprint from the word buffer.
- `crates/validation/src/s_word_survey.rs` + `cargo xtask gate s-word-survey` on the tolerance-study fixtures and reference charts (crossing distribution, correlation with impurity and position spread, split unchanged with/without `S_word`).

## Acceptance tests
- Review checklist (code) — S_word implementation uses no per-pair helper (REQ-REF-014).
- `cargo test -p engine s_word_diagnostic` — a footprint whose copies share an outcome but differ in word yields nonzero S_word; the split decision is identical with S_word forced to any value (REQ-REF-037).
- Review checklist (physics) — the sampling note names one metric and its range; the reviewer checks it depends only on whole raw reduced words; the doc change is merged with the physics reviewer's approval (REQ-REF-042).
- `cargo xtask gate s-word-survey --criterion` — the measured crossing distribution and the criterion applied to it; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-PAY-084).
- `cargo xtask gate s-word-survey` — record where S_word fires relative to outcome impurity on the tolerance-study fixtures; the split's output is unchanged with S_word present or absent (REQ-VAL-087).
- `cargo xtask gate s-word-survey --crossings` — record crossing-count distribution and its correlation with outcome impurity on reference charts (REQ-VAL-093).
- `cargo xtask gate s-word-survey --reference` — record where S_word fires relative to outcome impurity and position spread on a reference survey (REQ-VAL-095).

## Notes
- Definition written: REQ-REF-042. Calibration proposed: REQ-PAY-084.
