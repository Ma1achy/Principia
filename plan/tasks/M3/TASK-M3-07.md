# TASK-M3-07 — The regularisation axis and the Aarseth–Zare occupant

- **Milestone:** M3
- **Closes:** REQ-INT-021, REQ-INT-024, REQ-INT-044, REQ-INT-049, REQ-INT-054, REQ-TOOL-039, REQ-INT-080, REQ-TOOL-125
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
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"
- `decisions.md` § "R-162 — The reversible occupant is logH's TTL time mode"

- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
- `decisions.md` § "R-565 — The sync grid is a fixed quantum of physical time, `Δ_sync`, the same for every run and independent of the horizon and of frame timing: steps are sized from it, the march stops only on it, the playhead reads boundary states, so live march, catch-up and re-boot reach the same state bitwise; time windows such as the closure window are stated in physical time"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `decisions.md` § "R-563 — The integration floor follows the occupant: with regularisation off the substep machinery is unchanged; under a regularised occupant running out of the step budget makes the result undetermined, never discarded, and the separation floor and the entry-cap hits are counted, non-terminal signals; REQ-SCHED-063 and REQ-SCHED-077 are re-pointed"

## Deliverables
- `crates/kernel/src/regularisation/mod.rs` — the `Regularisation` slot, composition `Composed<S, R>` implementing `Advance`, `owns_time_mapping = (R != none)`.
- `crates/kernel/src/regularisation/az.rs` — AZ transcribed from the reference (`workbench/tb_az.py`, with the overshoot fix of `workbench/tb_az_overshoot_fix.py`), the conditioned Levi-Civita inverse, sync-boundary re-registration counted per trajectory.
- The fixed tau-schedule: the per-sync-interval substep count bound (canonical_spec §9 item 18, R-19) asserted in the driver.
- A per-trajectory diagnostic for the largest physical step taken that reports 'no usable step' as an explicit sentinel, never 0.0 (min/max folds reseeded from the first element).
- `fixtures/gates/lc-inverse/` and `fixtures/gates/az-figure-eight/`.

## Acceptance tests
- `cargo xtask gate separation-floor --propose` — REQ-INT-105's verify for Aarseth–Zare: the proposed `TINY` at f32 and f64 with its evidence (the smallest pre-floor `A` and `B` over the radial collision passage and the close-encounter fixtures, the floored-step counts, whether `TINY·TINY` underflows at f32 and what a doubly floored step then does), and its control (a `TINY` above the passage's smallest pre-floor value floors steps there and is reported); TASK-M3-08 re-runs it on Heggie and logH and closes REQ-INT-105, the human confirming it at the M3 gate (R-563, integrator contract Part 6; physics review 5606990734 of PR #277, P2, applied per R-369). *Was:* "… the human confirms it at the M3 gate and it is recorded in decisions.md; Heggie and logH (TASK-M3-08) floor at the same value (REQ-INT-105; …)", REQ-INT-105 then closed here.
- `cargo test -p kernel regularisation_compose` — every stepper × regularisation pair, `none` included, can be selected and runs; `none` is always available; the pairs the Part 2b table marks measured bad carry its warning and the others none; controls: a selection that blocks a measured-bad pair fails, and a measured-bad pair with no warning fails (REQ-INT-017; R-561). *Was:* "every stepper composes with every regularisation occupant; `none` is always available". Heggie's pairs are built by TASK-M3-08; this task's arm covers the pairs it builds and the table's data type, and TASK-M3-39 measures the table. REQ-INT-017 closes in TASK-M3-39, which re-runs this test over every pair once Heggie's exist (qa review 5607548278 of PR #277, Q8, applied per R-369; *was:* closed here).
- `cargo test -p kernel owns_time_mapping_composed` — enumerate every stepper × regularisation pair: owns_time_mapping == (regularisation != none); for AZ, ADVANCE to a sync boundary ends with physical time exactly t_target, and to a target between boundaries at the boundary before it (R-565) (REQ-INT-021). *Was:* "for AZ, ADVANCE ends with physical time exactly t_target".
- Review (physics): the AZ+RK4 profile has symplectic = false, step_reversible = false and run_reversible = false (R-500; *was:* "reversible = false"); the stepper slot under AZ accepts a Mikkola–Tanikawa leapfrog occupant type (REQ-INT-024).
- `cargo xtask gate lc-inverse` — at rho angle 179.9°: f32 relative error ≈ 5.96e-8 and f64 ≈ 1e-16, vs 2.2e-2 / 3.5e-9 for the unstable form (REQ-INT-044).
- `cargo xtask gate az-final-step` — figure-eight closure after one period ≤ 4.232e-09 at eta = 0.001 (vs 2.818e-03 without the fix); convergence better than first order (REQ-INT-049).
- `cargo test -p kernel tau_schedule_bound` — property test over fuzzed ICs: the substep count per sync interval never exceeds the Law 18 bound computed on the fixed tau-schedule (REQ-INT-054).
- `cargo test -p kernel no_usable_step_sentinel` — a fixture with all steps unusable reports the explicit none sentinel, not 0.0; the 2.209e128 overflow case is reported (REQ-TOOL-039).
- Proposal: the fixed sync quantum Δ_sync (one value, a constant of the build, R-565) and eta for each regularised occupant, with closure/drift evidence over the eta and Δ_sync ladders and integrator contract Part 2a's cost and label-flip evidence, and how the closure window's 0.4 time units map onto the grid; the human confirms them at the M3 gate (REQ-INT-080). *Was:* "n_sync and eta for each regularised occupant, with closure/drift evidence over the eta and n_sync ladders".
- Definition: dt_max's home (occupant diagnostic or ledger field) and its 'no usable step' sentinel written into integrator_contract Part 2c and approved by the physics reviewer (REQ-TOOL-125).

## Notes
- Gap: the sync schedule (`Δ_sync`, `eta`) that defines AZ's sync boundaries and tau-schedule has no value yet: R-565 makes the grid a fixed quantum of physical time, `Δ_sync`, a calibration (integrator contract Part 3's row), and AZ's `dt_left` is `Δ_sync`. *Was:* "the sync schedule (`n_sync`, `eta`) … has no default or owner in integrator contract Part 3".
- Gap: the Mikkola–Tanikawa leapfrog's equations are not in the corpus; this task defines the slot's type only. R-162: Aarseth–Zare with Mikkola–Tanikawa isn't required; the reversible occupant is logH's TTL time mode (TASK-M3-08).
- Gap: `dt_max` has no home in the new ledger; REQ-TOOL-039 is met by the occupant diagnostic above unless a ruling places it in the payload.
- RQ-102 ruled: R-159 to R-163. AZ's reference code, including the predictive step limit (`predictive_dtau`) and the landing clamp, is `docs/reference/prin-rs/src/integrate/az/driver.rs` (reference, not authority, R-159); the equations are integrator_contract Part 2b's (R-160, REQ-INT-084).
- Closes, for gaps the corpus leaves open: REQ-INT-080 (R-71 calibration), REQ-TOOL-125 (R-72 definition) (classification accepted by R-132).
