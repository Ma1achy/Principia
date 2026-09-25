# TASK-M3-26 — Order scaling and the symplectic-vs-not gates

- **Milestone:** M3
- **Closes:** REQ-VAL-030, REQ-VAL-031, REQ-VAL-139
- **Depends on:** TASK-M3-24
- **Needs (earlier milestones):** REQ-VAL-007
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
dd_integrator tests 1 and 2 run as numerical gates: energy error over one Kepler-embedded orbit (`m₂ → ε`) scales as dt² (KDK), dt⁴ (Yoshida-4, RK4) and dt⁶ (Yoshida-6) across a dt ladder; on a long bounded orbit symplectic occupants oscillate about E₀ without secular trend while RK4 and Euler drift, and Euler lights SUSPECT_ENERGY.

## References
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/validation/src/gates/{order_scaling,symplectic}.rs`, `fixtures/gates/kepler-embedded/`, `fixtures/gates/long-bounded/`, each with a control that must fail (e.g. a mislabelled order).

## Acceptance tests
- `cargo xtask gate order-scaling` — dd test 1: the fitted log-log slope of energy error vs dt equals the occupant's order within tolerance, per occupant (REQ-VAL-030).
- `cargo xtask gate symplectic-drift` — dd test 2: no secular trend (fitted slope ≈ 0) for symplectic occupants; secular slope for RK4/Euler; Euler's energy-drift view shows SUSPECT_ENERGY (the pass condition) (REQ-VAL-031).
- Proposal: the order-slope tolerance and the no-secular-trend tolerance, with the fitted slopes per occupant as evidence; the human confirms them at the M3 gate (REQ-VAL-139).

## Notes
- Gap: neither gate's slope tolerance is given by the corpus, and no calibration requirement covers them.
- Closes, for gaps the corpus leaves open: REQ-VAL-139 (R-71 calibration) (classification accepted by R-132).
