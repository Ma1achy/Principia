# TASK-M6-52 — Deep zooms of the shield: Lehto's 3.125×10⁻⁶ zoom and Valtonen et al.'s three windows (L-2, V-8 (b))

- **Milestone:** M6
- **Closes:** REQ-VAL-293, REQ-VAL-294
- **Depends on:** TASK-M6-08, TASK-M4-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
The replication suite's targets L-2, V-8 (b) run as engine tests (`principia_dd_replication.md`; R-594, R-629). The three-part yellow area of Lehto et al.'s Figs. 2 and 3, and the 3.125×10⁻⁶ zoom, must appear in our deep zoom (L-2). Banded structure must persist in the three zoom windows of Valtonen et al. 2016's Figs. 4.5–4.7 (V-8 (b)).

## References
- `docs/design/principia_dd_replication.md` § "5.9 Region D: maps and statistics"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The zooms rendered and reviewed against the figures.

## Acceptance tests
- `cargo xtask gate replication --target L-2 --render` — the figure beside the paper's; the zoom rendered and reviewed by the physics reviewer against the figures; control: the zoom at a point off the area must not show it; the physics reviewer compares them (REQ-VAL-293).
- `cargo xtask gate replication --target V-8b --render` — the figure beside the paper's; the windows rendered and reviewed by the physics reviewer; control: a window placed in the sea of tranquillity must show no bands; the physics reviewer compares them (REQ-VAL-294).

## Notes
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
