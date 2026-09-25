# TASK-M3-28 — Figure-eight closure and Lagrange rigidity

- **Milestone:** M3
- **Closes:** REQ-VAL-041, REQ-VAL-047, REQ-VAL-130, REQ-VAL-133
- **Depends on:** TASK-M3-24
- **Needs (earlier milestones):** REQ-VAL-020, REQ-ENC-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The exact-case ground truth runs: the figure-eight (T = 6.32591398, E = −1.2871419918, L_z = 0, Σp = 0) integrated one period reports `|dr|` across an eta/step ladder with the fitted convergence order, which must match the occupant's; the figure-eight traces its published shape-sphere curve; the Lagrange circular triangle stays equilateral at every sampled t. The closure gate's threshold and order tolerance (including whether AZ+RK4's ~third order is an expected failure) and the exact-case tolerances and structural windows are calibrated.

## References
- `docs/design/principia_dd_validation_orbits.md` § "3. Proposed suite"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"
- `docs/design/principia_dd_validation_orbits.md` § "1.2 Figure-eight — the canonical closure test"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"
- `docs/design/principia_dd_validation_orbits.md` § "1.3 Lagrange and Euler central configurations — the only analytic ones"
- `docs/notes/principia_validation_ground_truth_note.md` § "Open sub-questions (settle at implementation)"
- `docs/design/principia_dd_validation_orbits.md` § "0.1 The mechanism, and the fix — measured, not proposed"

## Deliverables
- `crates/validation/src/gates/{figure_eight,lagrange}.rs`, `fixtures/ground_truth/figure_eight/` (published shape-sphere curve).
- Calibration proposals: closure |dr| threshold and order tolerance per occupant; the exact-case and structural tolerances.

## Acceptance tests
- `cargo xtask gate figure-eight-closure` — closure |dr| per eta and fitted order; fails if the order is below the occupant's (threshold and tolerance: REQ-VAL-133, calibrated) (REQ-VAL-041).
- `cargo xtask gate exact-cases` — Lagrange circular: equilateral residual at every sampled t within the calibrated f64 tolerance (REQ-VAL-130); the figure-eight shape-sphere curve matches the published one (REQ-VAL-047).
- `cargo xtask gate exact-cases --propose` — the proposal measures each exact case at f64 and states its tolerance, and states each structural window; the human confirms them at the M3 gate and they are recorded in decisions.md (REQ-VAL-130).
- `cargo xtask gate figure-eight-closure --propose` — |dr| and fitted order per occupant across the eta/step ladder, with the threshold and tolerance; the human confirms them at the M3 gate and they are recorded in decisions.md (REQ-VAL-133).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-130, REQ-VAL-133.
