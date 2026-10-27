# TASK-M8-99 — The Šuvakov plane rendered, at equal masses and at the eight values of m₃, and OC-4 there (AT-D1, AT-D3, OC-4 on the Šuvakov plane)

- **Milestone:** M8
- **Closes:** REQ-VAL-311, REQ-VAL-312, REQ-VAL-313
- **Depends on:** TASK-M8-97
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets OC-4 on the Šuvakov plane, AT-D1, AT-D3 run as engine tests (`principia_dd_replication.md`; R-594, R-629). OC-4's comparison must be made for the atlas orbits on the Šuvakov plane, at equal masses (1,759 rows) and at the eight values of m₃ (1,350 rows), on a chart over that plane (OC-4 on the Šuvakov plane). The Šuvakov plane must be rendered at equal masses with all 1,759 atlas orbits on it marked and the stable ones highlighted, on the papers' axes (AT-D1). The Šuvakov plane must be rendered at each of the eight values of m₃ the atlas holds (0.5, 0.75, 2, 4, 5, 8, 10, 1.001), each with its orbits marked (AT-D3).

## References
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "4.3 The atlas's slice families"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- Result runs on the chart RQ-583 rules, in RQ-582's labelling; no chart is built here ahead of those rulings.

## Acceptance tests
- `cargo xtask gate replication --target OC-4s` — as OC-4, on the Šuvakov-plane chart once RQ-583 (and its labelling, RQ-582) is ruled; agreement at OC-4's calibrated level; control as OC-4's (REQ-VAL-311).
- `cargo xtask gate replication --target AT-D1 --render` — the figure beside the paper's; a Result (R-572) at f64; reviewed by the physics reviewer; the atlas's credit line on the figure; control: each marker's IC re-encoded must land on its marker; the physics reviewer compares them (REQ-VAL-312).
- `cargo xtask gate replication --target AT-D3 --render` — the figure beside the paper's; as AT-D1, per value of m₃; the physics reviewer compares them (REQ-VAL-313).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Waits on RQ-582, RQ-583: the task starts only once they are ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
