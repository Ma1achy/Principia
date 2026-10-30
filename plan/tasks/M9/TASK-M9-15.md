# TASK-M9-15 — The prototype's integrator and classifier numbers, reproduced or explained (tier D: D-1, D-2, D-4, D-5)

- **Milestone:** M9
- **Closes:** REQ-VAL-299, REQ-VAL-300, REQ-VAL-301, REQ-VAL-302, REQ-VAL-303, REQ-VAL-304, REQ-VAL-305
- **Depends on:** TASK-M3-28, TASK-M3-32, TASK-M3-34, TASK-M3-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets D-1, D-2, D-4, D-5 run as engine tests (`principia_dd_replication.md`; R-594, R-629). The figure-eight's fitted closure order under the default occupant must be reported beside the prototype's 2.40 with the landing clamp and 1.03 without, and under Aarseth–Zare beside 2.08 and 1.06; a difference must be explained by a measurement, the prototype's number never taken as ground truth (D-1). The default occupant's drift-field change under doubled re-registration on `config_stability` at 256² must be reported beside the prototype's chord, 4.832e-2 under Heggie against 5.162e-1 under Aarseth–Zare, the difference explained by a measurement where they differ (D-2). f32 against f64 outcome labels on near-field, 32 × 32, t = 13, under Heggie × RK4 on the CPU, must be measured and reported beside the prototype's Aarseth–Zare figures: 1022 of 1024 agreeing, 2 pixels with |dE/E| > 1, and 152 of 1024 flipping on the unconditioned branch (D-4). No escape must fire on the near-field chart by t = 13, as the prototype found (D-5). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.11 The prototype's results (tier D)"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "1.2 Figure-eight — the canonical closure test"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `crates/validation/src/gates/prototype.rs`: each prototype number beside ours, under the default occupant and, where the prototype measured it there, Aarseth–Zare.
- A report naming each difference and the measurement that explains it; the calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target D-1` — REQ-VAL-041's ladder, re-read; each order beside the prototype's; agreement within REQ-VAL-300, or the measurement that explains the difference recorded; control: the run with the landing clamp disabled must read the lower order (REQ-VAL-299).
- `cargo xtask gate replication --target D-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-300).
- `cargo xtask gate replication --target D-2` — REQ-VAL-052's protocol, re-read; within REQ-VAL-302 or explained; control: the Aarseth–Zare run must read the larger chord (REQ-VAL-301).
- `cargo xtask gate replication --target D-2 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-302).
- `cargo xtask gate replication --target D-4` — the kernel at f32 and f64 on the CPU; label agreement within REQ-VAL-304, or the difference explained; control: an f32 run with the conditioning disabled must flip more labels (REQ-VAL-303).
- `cargo xtask gate replication --target D-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-304).
- `cargo xtask gate replication --target D-5` — the escape count over the near-field grid at t = 13 is 0; control: the same grid run to the default horizon must show escapes (REQ-VAL-305).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-300, REQ-VAL-302, REQ-VAL-304.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-55 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each binds is § "12.2 Tiers"'s: (b). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate".
