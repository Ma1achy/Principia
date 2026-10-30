# TASK-M9-36 — The ruled external targets scored: FF-H, the figure-eight's published shape-sphere curve and SP-1 against Szebehely & Peters

- **Milestone:** M9
- **Closes:** REQ-VAL-042, REQ-VAL-127, REQ-VAL-333, REQ-VAL-334, REQ-VAL-335
- **Depends on:** TASK-M3-25, TASK-M3-27, TASK-M3-28, TASK-M3-29
- **Needs (earlier milestones):** REQ-VAL-034, REQ-VAL-126, REQ-VAL-133
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The three external targets that came from the ruled ground-truth suite are scored as measurements (R-649, R-650), each with its tier, its outcome (PASS, EXPLAINED DIVERGENCE or UNEXPLAINED) and its like-for-like checks. FF-H: two or three free-fall (brake) periodic orbits from Hristov et al.'s database (the fixtures TASK-M3-29 commits) return to rest in the same configuration, with the order reported. The figure-eight, integrated at f64, traces its published shape-sphere curve, its tolerance a calibration (R-71). SP-1's structural comparison: the classical Burrau rest start, run to its horizon as TASK-M3-27 runs it, is compared with Szebehely & Peters (1967) on a structural feature set this task defines (R-72), never pointwise. Each binds tier (b), the CPU IC inspector.

## References
- `docs/design/principia_dd_validation_orbits.md` § "1.1 Free-fall periodic orbits — the best fit for this project"
- `docs/design/principia_dd_validation_orbits.md` § "1.2 Figure-eight — the canonical closure test"
- `docs/design/principia_dd_validation_orbits.md` § "1.5 Burrau itself"
- `docs/design/principia_dd_validation_orbits.md` § "3. Proposed suite"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Open sub-questions (settle at implementation)"
- `docs/design/principia_dd_replication.md` § "12.1 What stays continuous, and what moves to M9"
- `docs/design/principia_dd_replication.md` § "12.2 Tiers"
- `docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-650 — External replication becomes its own final milestone, M9, after the core build: it scores every external target, writes up the explained divergences and tightens the GPU path, and exits on its report; the internal correctness tests and the cheap anchor smoke tests (figure-eight closure, SP-1 run to its horizon, SP-2) stay continuous and block a merge as now; source ingest and transcription go on whenever idle; early peeks score a target as information only, in a peek log *(refines R-649; amends R-594, R-629, R-630, R-640 and R-642: where and when the targets are scored)*"

## Deliverables
- Doc change: `docs/notes/principia_validation_ground_truth_note.md` — the Burrau feature set and how each is compared structurally (REQ-VAL-127).
- `fixtures/ground_truth/figure_eight/` (the published shape-sphere curve).
- `crates/validation/src/gates/{free_fall,figure_eight_curve,burrau_structure}.rs`; the calibration proposal for REQ-VAL-334.

## Acceptance tests
- `cargo xtask gate replication --target FF-H` — each orbit returns to rest in the same configuration after one period (tolerance: REQ-VAL-130, calibrated); order reported; the outcome recorded (REQ-VAL-042).
- `cargo xtask gate replication --target figure-eight-curve` — the figure-eight shape-sphere curve against the published one, within REQ-VAL-334; the outcome recorded; control: a Lagrange-triangle run's curve must not match (REQ-VAL-333).
- `cargo xtask gate replication --target figure-eight-curve --propose` — the proposal measures the curve's deviation at f64 across the eta ladder and states the tolerance, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-334).
- `cargo xtask gate replication --target SP-1-structure` — each feature of REQ-VAL-127 beside Szebehely & Peters's; the outcome recorded; control: a run from a start perturbed until its escaper changes must disagree (REQ-VAL-335).
- The note lists each feature and how it is compared structurally; physics reviewer approved (REQ-VAL-127).

## Notes
- Added by R-650 (30 Oct 2026): FF-H from TASK-M3-29; the figure-eight's published curve from TASK-M3-28 (REQ-VAL-047's, split out as REQ-VAL-333 and REQ-VAL-334); SP-1's structural comparison and its feature set from TASK-M3-27 (REQ-VAL-034's, split out as REQ-VAL-335; REQ-VAL-127). The command and file names are this port's, applied per R-369.
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-334.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-127.
- Targets are measurements, never gates (R-649): an outcome other than PASS blocks nothing; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass.
