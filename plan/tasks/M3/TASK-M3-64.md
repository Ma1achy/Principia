# TASK-M3-64 — Parischewsky et al.'s equilateral-triangle experiment around Lagrange (PA-1)

- **Milestone:** M3
- **Closes:** REQ-VAL-323, REQ-VAL-324
- **Depends on:** TASK-M3-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets PA-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Escaper-identity maps in side-ratio space around the Lagrange point must reproduce Parischewsky, Trani & Leigh 2023's, and their split, 82.1 % chaotic against 17.9 % regular (N_scramble ≥ 4), must be compared as a distribution (PA-1). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The work on the chart RQ-594 rules and the N_scramble diagnostic RQ-596 rules.
- `crates/validation/src/gates/side_ratio.rs`; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target PA-1` — the chart RQ-594 rules, the split once RQ-596 is; the maps reviewed by the physics reviewer; the split at REQ-VAL-324's level; control: a map at the wrong side ratio range must misplace the regular region (REQ-VAL-323).
- `cargo xtask gate replication --target PA-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-324).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-324.
- Waits on RQ-594, RQ-596: the task starts only once they are ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
