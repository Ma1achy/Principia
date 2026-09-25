# TASK-M3-27 — The Burrau smoke test and its structural features

- **Milestone:** M3
- **Closes:** REQ-VAL-034, REQ-VAL-127, REQ-VAL-132
- **Depends on:** TASK-M3-24, TASK-M3-25
- **Needs (earlier milestones):** REQ-VAL-016, REQ-ENC-011, REQ-VAL-019
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-10
- **Size:** ~300 lines

## Goal
The classical Burrau rest start resolves to binary plus escaper within the default horizon, asserted as outcome class, escaper identity (the lightest body) and a coarse `t_end` window qualitatively matching Szebehely & Peters (1967), never pointwise. The structural feature set is defined in the validation note (R-72), and the window — given the classical result sits past the default horizon — is calibrated (R-71).

## References
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `docs/design/principia_dd_validation_orbits.md` § "1.5 Burrau itself"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Open sub-questions (settle at implementation)"

## Deliverables
- Doc change: `docs/notes/principia_validation_ground_truth_note.md` — the Burrau feature set and how each is compared structurally (REQ-VAL-127).
- `crates/validation/src/gates/burrau.rs` — Path A run of the Burrau IC, compared with the independent reference's run structurally.
- Calibration proposal for the t_end window and horizon.

## Acceptance tests
- `cargo xtask gate burrau-smoke` — dd test 12: outcome class and escaper (lightest body) match; t_end within the calibrated window (REQ-VAL-132); close-encounter sequence and interaction topology compared structurally (REQ-VAL-034).
- The note lists each feature and how it is compared structurally; physics reviewer approved (REQ-VAL-127).
- The proposal states the window and horizon against Szebehely & Peters (1967) and validation_orbits §1.5; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-VAL-132).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-132.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-127.
