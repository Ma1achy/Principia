# TASK-M3-11 — The live shape readout, the lagged n̂ register and the closure fields

- **Milestone:** M3
- **Closes:** REQ-INT-042, REQ-PAY-043, REQ-PAY-055, REQ-PAY-056, REQ-PAY-059, REQ-VAL-055, REQ-INT-082
- **Depends on:** TASK-M3-04, TASK-M3-07
- **Needs (earlier milestones):** REQ-INT-001, REQ-INT-003, REQ-CHART-036, REQ-PAY-009, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~450 lines

## Goal
Each macro-step the shape vector `n = (u, v, w)/I` is derived live from positions only (mass-weighted Jacobi), with no checkpoint array. The escape window's `n̂` from one window earlier is held in a lagged register updated at the occupant's sampling points (macro-step boundaries unregularised, sync boundaries regularised), and the SimState widths include it. `closure_min`/`closure_step` track the running minimum of `|n̂(t) − n̂(0)|` after the shape first departs from `n̂(0)` by more than `δ_dep`, departure latched per sample; `δ_dep` is set by a recorded measurement and the departed bit's placement is written into the ledger (R-37).

## References
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_dd_validation_orbits.md` § "6. The closure field — from validation to discovery"
- `decisions.md` § "R-37 — `t_min` is a departure threshold on the shape sphere *(PL-2 (b))*"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `open-questions.md` § "Open questions"
- `decisions.md` § "R-29 — The escape criterion's undefined parts *(IE-1, amended)*"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `docs/read_first/principia_01_pitfalls.md` § "2.2 The criterion"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/kernel/src/driver/shape.rs` — `shape(r, masses)` (no momenta), per-macro-step readout.
- Ledger edits in `crates/ledger`: the lagged `n̂` register row and the departed bit (schema-version change); payload §1 width totals recomputed in `docs/design/principia_dd_simstate_payload.md`.
- `crates/kernel/src/driver/closure.rs` — `closure_min`/`closure_step` registers, departure gate.
- `crates/validation/src/measure/delta_dep.rs` + report `fixtures/gates/delta-dep/REPORT.md` — `|n̂(t) − n̂(0)|` distributions on periodic-orbit and generic fixtures.

## Acceptance tests
- `cargo test -p kernel shape_identities` — dd test 9: ‖n‖ = 1; equilateral → n_w = ±1; collinear → n_w = 0; n agrees with the IC Inspector's shapePoint on 5,000 random ICs to f64 round-off after the mirror fold (REQ-INT-042).
- `cargo test -p kernel theta_unwrap_real_orbit` — dd test 8: on a circulating bounded orbit θ̃ has no 2π jumps, orbit_count matches a hand count and retrograde matches the L_z sign (REQ-INT-082).
- `cargo test -p kernel closure_registers` — on a fixture trajectory closure_min matches an offline minimum over the stored shape path and closure_step equals the argmin step (REQ-PAY-043).
- Review (physics): the shape helper's signature takes r (and masses), not p (REQ-PAY-055).
- `cargo test -p kernel closure_departure` — a periodic orbit's closure_min ≈ 0 at closure_step ≈ period/dt_macro; closure before departure is ignored (REQ-PAY-056).
- Review (physics): the SimState layout table lists the lagged n̂ register and payload §1's width totals include it; `cargo test -p kernel lagged_nhat_sampling` — the register updates only at the occupant's sampling points (macro-step or sync boundaries) (REQ-PAY-059).
- `cargo xtask gate delta-dep` — measurement report of |n̂(t) − n̂(0)| distributions on periodic-orbit and generic fixtures justifying δ_dep; the ledger row for the departed bit exists and the schema version changes (REQ-VAL-055).

## Notes
- R-113 (RQ-94): REQ-INT-001's dd test 8 on a real orbit is split off as REQ-INT-082 and closed here; the synthetic-path half stays in TASK-M1-11.
- δ_dep must be relative or gap-set (pitfalls §3, 'A threshold on a quantity spanning decades must be relative'). The value is recorded with its measurement; R-37 says the value is set by measurement.
