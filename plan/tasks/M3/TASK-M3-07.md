# TASK-M3-07 — The regularisation axis and the Aarseth–Zare occupant

- **Milestone:** M3
- **Closes:** REQ-INT-017, REQ-INT-021, REQ-INT-024, REQ-INT-044, REQ-INT-049, REQ-INT-054, REQ-TOOL-039
- **Depends on:** TASK-M3-04, TASK-M3-05
- **Needs (earlier milestones):** REQ-DEC-005, REQ-INT-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.1, PIT-9
- **Size:** ~500 lines

## Goal
Regularisation is a second occupant slot independent of the stepper (`none` / AZ / Heggie / logH, integrator contract Part 2b). This task builds the axis, `none`, and Aarseth–Zare: two pairs with the reference body from the longest side, re-registering at every sync boundary, stepping in fictitious time `tau` on a fixed tau-schedule until it lands exactly on `t_target` — the final step shrunk by `dtau_eff = min(dtau, max(dt_left − t_so_far, 0)/(A·B))` — with the conditioned Levi-Civita inverse branch. `owns_time_mapping` is reported for the composed occupant; the AZ slot accepts a Mikkola–Tanikawa time-transformed leapfrog while RK4 fills it with `symplectic: false`.

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `decisions.md` § "R-19 — Change 8 landed in full *(closes RQ-17)*"
- `docs/contracts/principia_integrator_contract.md` § "The table gains two rows"
- `docs/design/principia_dd_predictability_horizon.md` § "7.3 The f32 horizon is falsified — and the cause is AZ, not precision"
- `docs/design/principia_dd_validation_orbits.md` § "0.1 The mechanism, and the fix — measured, not proposed"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"

## Deliverables
- `crates/kernel/src/regularisation/mod.rs` — the `Regularisation` slot, composition `Composed<S, R>` implementing `Advance`, `owns_time_mapping = (R != none)`.
- `crates/kernel/src/regularisation/az.rs` — AZ transcribed from the reference (`workbench/tb_az.py`, with the overshoot fix of `workbench/tb_az_overshoot_fix.py`), the conditioned Levi-Civita inverse, sync-boundary re-registration counted per trajectory.
- The fixed tau-schedule: the per-sync-interval substep count bound (canonical_spec §9 item 18, R-19) asserted in the driver.
- A per-trajectory diagnostic for the largest physical step taken that reports 'no usable step' as an explicit sentinel, never 0.0 (min/max folds reseeded from the first element).
- `fixtures/gates/lc-inverse/` and `fixtures/gates/az-figure-eight/`.

## Acceptance tests
- `cargo test -p kernel regularisation_compose` — every stepper composes with every regularisation occupant; `none` is always available (REQ-INT-017).
- `cargo test -p kernel owns_time_mapping_composed` — enumerate every stepper × regularisation pair: owns_time_mapping == (regularisation != none); for AZ, ADVANCE ends with physical time exactly t_target (REQ-INT-021).
- Review (physics): the AZ+RK4 profile has symplectic = false and reversible = false; the stepper slot under AZ accepts a Mikkola–Tanikawa leapfrog occupant type (REQ-INT-024).
- `cargo xtask gate lc-inverse` — at rho angle 179.9°: f32 relative error ≈ 5.96e-8 and f64 ≈ 1e-16, vs 2.2e-2 / 3.5e-9 for the unstable form (REQ-INT-044).
- `cargo xtask gate az-final-step` — figure-eight closure after one period ≤ 4.232e-09 at eta = 0.001 (vs 2.818e-03 without the fix); convergence better than first order (REQ-INT-049).
- `cargo test -p kernel tau_schedule_bound` — property test over fuzzed ICs: the substep count per sync interval never exceeds the Law 18 bound computed on the fixed tau-schedule (REQ-INT-054).
- `cargo test -p kernel no_usable_step_sentinel` — a fixture with all steps unusable reports the explicit none sentinel, not 0.0; the 2.209e128 overflow case is reported (REQ-TOOL-039).

## Notes
- Gap reported: the sync schedule (`n_sync`, `eta`) that defines AZ's sync boundaries and tau-schedule has no default or owner in integrator contract Part 3.
- Gap reported: the Mikkola–Tanikawa leapfrog's equations are not in the corpus; this task defines the slot's type only.
- Gap reported: `dt_max` has no home in the new ledger; REQ-TOOL-039 is met by the occupant diagnostic above unless a ruling places it in the payload.
