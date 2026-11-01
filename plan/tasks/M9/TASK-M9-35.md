# TASK-M9-35 — The Šuvakov plane rendered, at equal masses and at the eight values of m₃, and OC-4 there (AT-D1, AT-D3, OC-4 on the Šuvakov plane)

- **Milestone:** M9
- **Closes:** REQ-VAL-311, REQ-VAL-312, REQ-VAL-313
- **Depends on:** TASK-M9-33
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
- `decisions.md` § "R-639 — The Šuvakov-plane targets split into a closure part, which stays in M4 or M8 and runs now, and a chart part, which moves to gate the task that builds the Šuvakov chart and is not dropped, a target whose split isn't clean being queued; a read-only licence audit of every committed PDF; none of the five targets splits cleanly, so all five are queued *(answers RQ-608)*"
- `decisions.md` § "R-653 — Five map targets and B-2 are scored on tier (a), their f64 Result or CPU FTLE run beside it and recorded as information, an explained divergence between the two being a precision-tier finding *(closes RQ-622; refines R-649)*"

## Deliverables
- Result runs on the chart RQ-583 rules, in RQ-582's labelling; no chart is built here ahead of those rulings.

## Acceptance tests
- `cargo xtask gate replication --target OC-4s` — as OC-4, on the Šuvakov-plane chart once RQ-583 (and its labelling, RQ-582) is ruled; agreement at OC-4's calibrated level; control as OC-4's; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-311).
- `cargo xtask gate replication --target AT-D1 --render` — the figure beside the paper's; scored on tier (a): the GPU explorer's f32 output; reviewed by the physics reviewer; the f64 Result (R-572) run beside it and recorded as information, a divergence between the two a precision-tier finding; the atlas's credit line on the figure; control: each marker's IC re-encoded must land on its marker; the physics reviewer compares them; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-312).
- `cargo xtask gate replication --target AT-D3 --render` — the figure beside the paper's; as AT-D1, per value of m₃; the physics reviewer compares them; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649) (REQ-VAL-313).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Waits on RQ-583: the task starts only once it is ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5). RQ-582 is closed by R-630 item 3: the middle body is body 1, at β = 0, Hudomal eq. (18)'s labelling.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *28 Oct 2026 (R-636, closing RQ-583):* the owner: "Šuvakov-plane chart: keep its targets as closures for now; no chart build yet." No chart is built, and this task's targets need one; it waits on RQ-608, which asks what they do meanwhile.
- *28 Oct 2026 (R-639):* the owner ruled that each Šuvakov-plane target splits into a closure part, kept here and run now, and a chart part, which gates the task that builds the chart, and that a target whose split isn't clean is queued. None of this task's targets splits cleanly (R-639), so they are queued as RQ-618 and stay as they are; the task still waits on RQ-608, now with RQ-618.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M8-99 (milestone M8). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): OC-4 on the Šuvakov plane (REQ-VAL-311): tier (a); AT-D1 (REQ-VAL-312): tier (a); AT-D3 (REQ-VAL-313): tier (a) (R-653, RQ-622 (c): scored on tier (a), the f64 Result beside it as information; *was:* "its scoring on an f64 Result waits on RQ-622"). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). A wait on a ruling or data below holds this task's scoring, never a milestone. R-639's chart part, "which moves to gate the task that builds the Šuvakov chart", gates nothing now (R-649): it is scored when that chart exists, and RQ-618 stays open on what is built.
