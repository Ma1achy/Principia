# TASK-M3-48 — Heggie 1974's regularisation targets: the step ratio against Aarseth–Zare (H-1, H-2, DRAFT-VAL-04) and C/3 toward the Lagrange triple collision (H-3, DRAFT-VAL-03)

- **Milestone:** M3
- **Closes:** REQ-VAL-205, REQ-VAL-206, REQ-VAL-207, REQ-VAL-208
- **Depends on:** TASK-M3-24, TASK-M3-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets H-3, H-1, H-2 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Released from rest at (−1, 0), (1, 0), (Δx, √3 + Δy) with equal masses, under the default occupant, a third of the minimum perimeter (C/3) must scale with Δ as Heggie 1974's Table III does: 4e-4, 7e-6, 1e-7 at Δy = 1e-4, 1e-6, 1e-8, and 3e-4, 6e-6 at Δx = 1e-4, 1e-6; the reversal errors in that table are Heggie's integrator's and are not asserted. Under one stepper, the ratio of steps taken by Heggie's regularisation to steps taken by Aarseth–Zare's, at matched final energy error, must be measured for Burrau to t = 16 and for Aarseth & Zare 1974's Example II to t = 0.002, and recorded beside Heggie 1974's Tables I and II (step ratios 147/129, 257/225, 455/397; 48/42, 83/70, 149/124). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.5 Regularisation"
- `docs/design/principia_dd_replication.md` § "6. The first batch"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `fixtures/heggie_1974/tables_i_ii_iii.csv`, Tables I–III transcribed twice from the paper, the two copies and their diff kept, each row with its page.
- `crates/validation/src/gates/heggie_1974.rs`: the step ratio at matched energy error under one stepper, and C/3 per Δ with the slope, √3 entered both ways.
- The calibration proposals for the ratio band and the C/3 tolerance.

## Acceptance tests
- `cargo xtask gate replication --target H-3` — C/3 per Δ and the log-log slope between rungs, recorded beside Heggie's; √3 entered as Heggie did (rounded to 15 decimals) and at full f64, both recorded; the Δ = 0 run (homothetic collapse) reaches its minimum C without a non-finite state; control: the unregularised `none` occupant run alongside; tolerance REQ-VAL-206 (REQ-VAL-205).
- `cargo xtask gate replication --target H-3 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-206).
- `cargo xtask gate replication --target H-1` — steps and |δE/E| per tolerance or eta, both occupants, one stepper; the ratio is the compared quantity (Heggie used RKF7(8), which we do not ship); a disagreement beyond REQ-VAL-208's band is queued in REVIEW_QUEUE.md with both numbers (rule 4) (REQ-VAL-207).
- `cargo xtask gate replication --target H-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-208).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-206, REQ-VAL-208.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Part of the first batch (`principia_dd_replication.md` § 6), ported from PLAN.md § 6's drafts by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
