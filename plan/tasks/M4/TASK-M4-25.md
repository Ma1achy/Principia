# TASK-M4-25 — Li, Li & Liao's mass-plane contours for one periodic family (LLL-1)

- **Milestone:** M4
- **Closes:** REQ-VAL-329
- **Depends on:** TASK-M4-17, TASK-M2-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The replication suite's targets LLL-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Period, angular momentum and stability contours over (m₁, m₂) for one periodic family must reproduce Li, Li & Liao 2020's on the mass simplex (LLL-1).

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
- The contours on the mass simplex, by the continuation RQ-597 rules.

## Acceptance tests
- `cargo xtask gate replication --target LLL-1 --render` — the figure beside the paper's; periodic-orbit continuation once RQ-597 is ruled; reviewed by the physics reviewer; the atlas's 7 `li2021` rows are OC-1's; control: a family continued from a wrong seed must not match; the physics reviewer compares them (REQ-VAL-329).

## Notes
- Waits on RQ-597: the task starts only once it is ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
