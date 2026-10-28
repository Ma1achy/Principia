# TASK-M4-23 — Hudomal's return-proximity map over the Šuvakov plane and the butterfly III zoom (HU-1, HU-2)

- **Milestone:** M4
- **Closes:** REQ-VAL-314, REQ-VAL-315, REQ-VAL-316, REQ-VAL-317
- **Depends on:** TASK-M4-17, TASK-M3-53, TASK-M3-11
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets HU-1, HU-2 run as engine tests (`principia_dd_replication.md`; R-594, R-629). The return-proximity map over (p₁, p₂) ∈ [0.3, 0.5] × [0.1, 0.5] must show Hudomal's bright candidate spots and dark collision lines (Fig. 8), with its minima at Table 23's points (HU-1). The butterfly III zoom must show three candidates, the middle one at (0.405916, 0.230163) with T = 13.866 (HU-2). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M4 gate (R-71).

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
- The maps on the chart RQ-583 rules, in RQ-582's labelling; no chart is built here ahead of those rulings.
- The calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target HU-1` — the closure field on the Šuvakov-plane chart once RQ-583 is ruled; each Table 23 minimum within REQ-VAL-315; the map reviewed by the physics reviewer; control: a map at m₃ = 2 must not show the equal-mass minima (REQ-VAL-314).
- `cargo xtask gate replication --target HU-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M4 gate and recorded in `decisions.md` (REQ-VAL-315).
- `cargo xtask gate replication --target HU-2` — as HU-1, zoomed; the middle candidate's position and period within REQ-VAL-317 (REQ-VAL-316).
- `cargo xtask gate replication --target HU-2 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M4 gate and recorded in `decisions.md` (REQ-VAL-317).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M4 gate, then recorded in decisions.md): REQ-VAL-315, REQ-VAL-317.
- Waits on RQ-583: the task starts only once it is ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5). RQ-582 is closed by R-630 item 3: the middle body is body 1, at β = 0, Hudomal eq. (18)'s labelling.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
