# TASK-M9-26 — Lehto's lifetime map at full resolution, and the shield's mirror symmetry (L-1, V-8 (a))

- **Milestone:** M9
- **Closes:** REQ-VAL-290, REQ-VAL-291, REQ-VAL-292
- **Depends on:** TASK-M9-20, TASK-M4-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets L-1, V-2/V-8 (a) run as engine tests (`principia_dd_replication.md`; R-594, R-629). Lehto et al.'s lifetime map's morphology — the 1:1 band, the odd resonances, the sea of tranquillity, the trenches and the symmetry — must appear in our lifetime map through the tested lens remap at full resolution, with L-3's statistic and L-5's controls re-measured there (L-1). A full-lens render of the lifetime map must be mirror-symmetric about both axes to rounding (V-2, V-8 (a)). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

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
- The full-resolution lifetime map from the uniform-grid survey, through the lens remap; the mirror-symmetry check; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target L-1 --render` — the figure beside the paper's; the full-resolution map from the uniform-grid survey, reviewed against Fig. 1 by the physics reviewer; L-3's statistic at its calibrated level; control: the map without the lens remap must misplace the band; the physics reviewer compares them (REQ-VAL-290).
- `cargo xtask gate replication --target V-8a` — the render against its two mirror images, pixel by pixel, within REQ-VAL-292; control: a render with one body's mass changed must break the symmetry (REQ-VAL-291).
- `cargo xtask gate replication --target V-8a --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-292).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-292.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M4-22 (milestone M4). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): L-1 (REQ-VAL-290): tier (a); V-2 / V-8 (a) (REQ-VAL-291): tier (b). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M4 gate".
