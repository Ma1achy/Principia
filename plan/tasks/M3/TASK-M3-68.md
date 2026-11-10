# TASK-M3-68 — The Aarseth–Zare occupant

- **Milestone:** M3
- **Closes:** REQ-INT-024, REQ-INT-044, REQ-INT-049, REQ-INT-054, REQ-INT-080, REQ-INT-114
- **Depends on:** TASK-M3-07
- **Needs (earlier milestones):** REQ-DEC-005, REQ-INT-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.1, PIT-9
- **Size:** ~400 lines

## Goal
Aarseth–Zare fills TASK-M3-07's regularisation slot, built to integrator contract Part 2b § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)": two pairs about the reference body opposite the longest side, re-registering at every sync boundary, stepping in fictitious time `tau` under the transcribed step control until it lands exactly on `t_target` — the final step shrunk by `dtau_eff = min(dtau, max(dt_left − t_so_far, 0)/(A·B))` — with the conditioned Levi-Civita inverse branch. The AZ slot accepts a Mikkola–Tanikawa time-transformed leapfrog while RK4 fills it with `symplectic: false`. Split from TASK-M3-07 (applied per R-369, 10 Nov 2026), whose Goal this was.

## References
- `docs/contracts/principia_integrator_contract.md` § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/contracts/principia_integrator_contract.md` § "The equations and the step control (R-160, R-161)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `decisions.md` § "R-19 — Change 8 landed in full *(closes RQ-17)*"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"
- `docs/contracts/principia_integrator_contract.md` § "The table gains two rows"
- `docs/design/principia_dd_predictability_horizon.md` § "7.3 The f32 horizon is falsified — and the cause is AZ, not precision"
- `docs/design/principia_dd_validation_orbits.md` § "0.1 The mechanism, and the fix — measured, not proposed"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `decisions.md` § "R-162 — The reversible occupant is logH's TTL time mode"
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)"
- `decisions.md` § "R-565 — The sync grid is a fixed quantum of physical time, `Δ_sync`, the same for every run and independent of the horizon and of frame timing: steps are sized from it, the march stops only on it, the playhead reads boundary states, so live march, catch-up and re-boot reach the same state bitwise; time windows such as the closure window are stated in physical time"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `decisions.md` § "R-927 — RQ-756, Aarseth–Zare's open points: REQ-INT-049's figure-eight threshold is re-measured under the step rule in force, a calibration; REQ-INT-054's bound per sync interval is the step budget over one interval; on each point of (5) where the reference code and the corpus disagree, the reference code's choice, the Rust driver being that code, the threshold re-measured under the same code; three points whose Rust files are not in the imported set stay open *(on RQ-756, which stays open)*"
- `decisions.md` § "R-563 — The integration floor follows the occupant: with regularisation off the substep machinery is unchanged; under a regularised occupant running out of the step budget makes the result undetermined, never discarded, and the separation floor and the entry-cap hits are counted, non-terminal signals; REQ-SCHED-063 and REQ-SCHED-077 are re-pointed"

## Deliverables
- `crates/kernel/src/regularisation/az.rs` — Aarseth–Zare built to integrator contract Part 2b § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)": the reference-body rule with its tie-break, `Γ`'s equations of motion, the conditioned Levi-Civita inverse, the step control, and sync-boundary re-registration counted per trajectory. *Was (in TASK-M3-07):* "AZ transcribed from the reference (`workbench/tb_az.py`, with the overshoot fix of `workbench/tb_az_overshoot_fix.py`), the conditioned Levi-Civita inverse, sync-boundary re-registration counted per trajectory."
- The fixed tau-schedule: the per-sync-interval substep count bound (canonical_spec §9 item 18, R-19) asserted in the driver.
- Aarseth–Zare's march feeds each step's physical increment to TASK-M3-07's `dt_max` diagnostic (the transcription's step 8).
- `fixtures/gates/lc-inverse/` and `fixtures/gates/az-figure-eight/`.

## Acceptance tests
- `cargo xtask gate separation-floor --propose` — REQ-INT-105's verify for Aarseth–Zare: the proposed `TINY` at f32 and f64 with its evidence (the smallest pre-floor `A` and `B` over the radial collision passage and the close-encounter fixtures, the floored-step counts, whether `TINY·TINY` underflows at f32 and what a doubly floored step then does), and its control (a `TINY` above the passage's smallest pre-floor value floors steps there and is reported); TASK-M3-08 re-runs it on Heggie and logH and closes REQ-INT-105, the human confirming it at the M3 gate (R-563, integrator contract Part 6; physics review 5606990734 of PR #277, P2, applied per R-369). *Was:* "… the human confirms it at the M3 gate and it is recorded in decisions.md; Heggie and logH (TASK-M3-08) floor at the same value (REQ-INT-105; …)", REQ-INT-105 then closed here. Moved from TASK-M3-07 (applied per R-369, 10 Nov 2026).
- `cargo test -p kernel regularisation_compose` — its Aarseth–Zare arm: every stepper × Aarseth–Zare pair can be selected and runs, and carries the Part 2b table's warning if the table marks it measured bad and none otherwise (REQ-INT-017's arm; R-561). Moved from TASK-M3-07, whose arm covered "the pairs it builds" (applied per R-369, 10 Nov 2026).
- `cargo test -p kernel owns_time_mapping_composed` — its Aarseth–Zare arm: for AZ, ADVANCE to a sync boundary ends with physical time exactly t_target, and to a target between boundaries at the boundary before it (R-565). Moved from TASK-M3-07 (REQ-INT-021, which closes there; applied per R-369, 10 Nov 2026). *Was:* "for AZ, ADVANCE ends with physical time exactly t_target".
- Review (physics): the AZ+RK4 profile has symplectic = false, step_reversible = false and run_reversible = false (R-500; *was:* "reversible = false"); the stepper slot under AZ accepts a Mikkola–Tanikawa leapfrog occupant type (REQ-INT-024).
- `cargo xtask gate lc-inverse` — at rho angle 179.9°: f32 relative error ≈ 5.96e-8 and f64 ≈ 1e-16, vs 2.2e-2 / 4.0e-12 for the unstable form, the reference's own table, whose 3.5e-9 is its f64 figure at 179.99° (REQ-INT-044; R-927, RQ-756 (5) 3). *Was:* "… vs 2.2e-2 / 3.5e-9 for the unstable form (REQ-INT-044). The unstable form's f64 figure at 179.9° is RQ-756 (5) 3's."
- `cargo xtask gate az-final-step --propose` — REQ-INT-114's verify: the figure-eight closure after one period at eta = 0.001 and `n = 32`, under integrator contract Part 6's per-step rule and the transcribed step control with the landing secant, the convergence ladder, and the closure without the landing fix beside it, each with its re-registration count and whether the predictive limit bound any step; the threshold proposed from the measured closure with its margin; the human confirms it at the M3 gate (R-927, RQ-756 (2)(a)).
- `cargo xtask gate az-final-step` — figure-eight closure after one period at most REQ-INT-114's calibrated threshold at eta = 0.001, the closure without the landing fix reported beside it; convergence better than first order (REQ-INT-049; R-927). The gate runs on its own sync interval `T/n` with `n = 32`, so `ADVANCE` lands on the period under R-565, and it reports its re-registration count with its result (PIT-4). *Was:* "… closure after one period ≤ 4.232e-09 at eta = 0.001 (vs 2.818e-03 without the fix) … the threshold and the step rule it is measured under are RQ-756 (2), (5) 1-2 and (5) 9."
- `cargo test -p kernel tau_schedule_bound` — property test over fuzzed ICs: the substep count per sync interval never exceeds the Law 18 bound, the step budget over one interval, `max(N_min, step_budget_rate·Δ_sync)` (REQ-INT-054; R-927, RQ-756 (4)(a)). *Was:* "… the Law 18 bound computed on the fixed tau-schedule (REQ-INT-054). The bound's formula is RQ-756 (4)."
- Proposal: the fixed sync quantum Δ_sync (one value, a constant of the build, R-565) and eta for each regularised occupant, with closure/drift evidence over the eta and Δ_sync ladders and integrator contract Part 2a's cost and label-flip evidence, and how the closure window's 0.4 time units map onto the grid; the human confirms them at the M3 gate (REQ-INT-080). *Was:* "n_sync and eta for each regularised occupant, with closure/drift evidence over the eta and n_sync ladders". Moved from TASK-M3-07 (applied per R-369, 10 Nov 2026).

## Notes
- Applied per R-369 (10 Nov 2026): split from TASK-M3-07, which keeps the regularisation axis, `none`, the pair table's data type and the `dt_max` marker. Each acceptance arm and requirement above moved from TASK-M3-07 unchanged except where marked.
- R-927 (10 Nov 2026) answers RQ-756 (2), (4) and (5) 1 to 5 and 9: the threshold is re-measured under the rule in force (REQ-INT-114), the bound is the step budget over one interval, and the landing secant, the gate's `n = 32` and the per-step rule stand as driver.rs and the contract have them. RQ-756 stays open on (5) 6, 7 and 8 only: the predictive limit's `v_rel`, the guard the inverse map tests against, and the `1e-300` guards and the tie-break, whose reference files aren't imported; each is marked *(RQ-756)* in the transcription, and what touches those three waits on it. *Was:* "RQ-756 is open on REQ-INT-049's threshold (2), REQ-INT-054's bound (4), and eight points where the reference set and the corpus disagree (5) … The arms they touch wait on it."
- Gap: the sync schedule (`Δ_sync`, `eta`) that defines AZ's sync boundaries and tau-schedule has no value yet: R-565 makes the grid a fixed quantum of physical time, `Δ_sync`, a calibration (integrator contract Part 3's row), and AZ's `dt_left` is `Δ_sync`. *Was:* "the sync schedule (`n_sync`, `eta`) … has no default or owner in integrator contract Part 3".
- Gap: the Mikkola–Tanikawa leapfrog's equations are not in the corpus; this task defines the slot's type only. R-162: Aarseth–Zare with Mikkola–Tanikawa isn't required; the reversible occupant is logH's TTL time mode (TASK-M3-08).
- RQ-102 ruled: R-159 to R-163. Aarseth–Zare's equations and step control, including the predictive step limit and the landing clamp, are integrator contract Part 2b's, transcribed (RQ-756). *Was (in TASK-M3-07):* "AZ's reference code, including the predictive step limit (`predictive_dtau`) and the landing clamp, is `docs/reference/prin-rs/src/integrate/az/driver.rs` (reference, not authority, R-159); the equations are integrator_contract Part 2b's (R-160, REQ-INT-084)."
- Closes, for gaps the corpus leaves open: REQ-INT-080 (R-71 calibration) (classification accepted by R-132).
