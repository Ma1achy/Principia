# TASK-M3-11 — The live shape readout, the lagged n̂ register and the closure fields

- **Milestone:** M3
- **Closes:** REQ-INT-042, REQ-PAY-043, REQ-PAY-055, REQ-PAY-056, REQ-PAY-059, REQ-VAL-055, REQ-INT-082, REQ-INT-087, REQ-INT-088, REQ-INT-089
- **Depends on:** TASK-M3-04, TASK-M3-07
- **Needs (earlier milestones):** REQ-INT-001, REQ-INT-003, REQ-CHART-036, REQ-PAY-009, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~600 lines

## Goal
Each macro-step the shape vector `n = (u, v, w)/I` is derived live from positions only (mass-weighted Jacobi), with no checkpoint array. The escape window's `n̂` from one window earlier is held in a lagged register updated at the occupant's sampling points (macro-step boundaries unregularised, sync boundaries regularised), and the SimState widths include it. `closure_min`/`closure_step` track the running minimum of `|n̂(t) − n̂(0)|` after the shape first departs from `n̂(0)` by more than `δ_dep`, departure latched per sample; `δ_dep` is set by a recorded measurement and the departed bit's placement is written into the ledger (R-37). θ̃ runs in the march with R-389's pole hold kept in `SimState` alone (R-397): the frozen reference in `_reserved` as a u16 code in steps of 2π/65535, 0xFFFF for "no reference", "inside the hold" recomputed from the current state, so a march's θ̃ is bit-identical however it is split into dispatches.

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
- `open-questions.md` § "Pending-changes register"
- `decisions.md` § "R-29 — The escape criterion's undefined parts *(IE-1, amended)*"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `docs/read_first/principia_01_pitfalls.md` § "2.2 The criterion"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-389 — `θ̃` starts at 0, and below a pole radius `r_pole` it holds with a frozen reference, adding the wrapped exit-minus-entry longitude on exit *(closes RQ-223)*"
- `decisions.md` § "R-392 — An IC that starts inside `θ̃`'s pole radius adds no delta at its first exit; `θ̃` counts from the exit longitude *(closes RQ-225)*"
- `decisions.md` § "R-397 — `θ̃`'s frozen pole reference is stored in `_reserved` as a u16, with 0xFFFF for none; `SimState`'s size is unchanged *(closes RQ-226)*"
- `docs/design/principia_dd_simstate_payload.md` § "`_reserved` (u16) — `θ̃`'s frozen pole reference (R-397)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"

## Deliverables
- `crates/kernel/src/driver/shape.rs` — `shape(r, masses)` (no momenta), per-macro-step readout.
- Ledger edits in `crates/ledger`: the lagged `n̂` register row and the departed bit (schema-version change); payload §1 width totals recomputed in `docs/design/principia_dd_simstate_payload.md`.
- `crates/kernel/src/driver/closure.rs` — `closure_min`/`closure_step` registers, departure gate.
- θ̃ in the march (R-397): the reference's encode and decode in `crates/kernel` (the step count and sentinel read from the register); the march's pole hold reading and writing `_reserved`, recomputing the inside test from the current state; ledger edits in `crates/ledger`: the register entries `theta_ref_steps` and `theta_ref_none`, added to the hashed stored-bits entries (schema-version change), and `_reserved`'s meaning in its declaration's comment.
- `crates/validation/src/measure/delta_dep.rs` + report `fixtures/gates/delta-dep/REPORT.md` — `|n̂(t) − n̂(0)|` distributions on periodic-orbit and generic fixtures.

## Acceptance tests
- `cargo test -p kernel shape_identities` — dd test 9: ‖n‖ = 1; equilateral → n_w = ±1; collinear → n_w = 0; n agrees with the IC Inspector's shapePoint on 5,000 random ICs to f64 round-off after the mirror fold (REQ-INT-042).
- `cargo test -p kernel theta_unwrap_real_orbit` — dd test 8: on a circulating bounded orbit θ̃ has no 2π jumps, orbit_count matches a hand count and retrograde matches the L_z sign (REQ-INT-082).
- `cargo test -p kernel closure_registers` — on a fixture trajectory closure_min matches an offline minimum over the stored shape path and closure_step equals the argmin step (REQ-PAY-043).
- Review (physics): the shape helper's signature takes r (and masses), not p (REQ-PAY-055).
- `cargo test -p kernel closure_departure` — a periodic orbit's closure_min ≈ 0 at closure_step ≈ period/dt_macro; closure before departure is ignored (REQ-PAY-056).
- Review (physics): the SimState layout table lists the lagged n̂ register and payload §1's width totals include it; `cargo test -p kernel lagged_nhat_sampling` — the register updates only at the occupant's sampling points (macro-step or sync boundaries) (REQ-PAY-059).
- `cargo test -p kernel theta_ref_code` — every code round-trips, `encode(decode(c)) = c`; the encode's error is at most π/65535 plus round-off and never yields 0xFFFF; decode lies in (−π, π) (REQ-INT-087).
- `cargo test -p ledger theta_ref_register` — `theta_ref_steps` and `theta_ref_none` are register entries hashed into the schema version; changing either's value changes it (REQ-INT-087).
- `cargo test -p kernel theta_hold_state` — a fresh sample's `_reserved` is 0xFFFF; it is 0xFFFF outside the disc after every step; entry writes the code of the last longitude outside; exit adds `wrap(exit − decode(code))`, ±π adding +π, and writes 0xFFFF; an IC inside adds nothing at its first exit; the inside test is recomputed from the current state; descriptor bits 10–15 stay zero (REQ-INT-088).
- `cargo test -p kernel theta_resume_split` — one call against k ∈ {2, 3, 7} resumed calls at f32 and f64, split on a pole passage's entry step, inside it and on its exit step, and inside the disc for an IC that starts there: θ̃'s bits and `_reserved` are identical; a negative control holding the reference unquantised within a dispatch fails (REQ-INT-089).
- Review (physics): the alignment recheck (REQ-PAY-009) for `_reserved` and for each member this task adds (the lagged `n̂` register): sizes and offsets stated, and payload §1's totals recomputed.
- `cargo xtask gate delta-dep` — measurement report of |n̂(t) − n̂(0)| distributions on periodic-orbit and generic fixtures justifying δ_dep; the ledger row for the departed bit exists and the schema version changes (REQ-VAL-055).

## Notes
- R-113 (RQ-94): REQ-INT-001's dd test 8 on a real orbit is split off as REQ-INT-082 and closed here; the synthetic-path half stays in TASK-M1-11.
- δ_dep must be relative or gap-set (pitfalls §3, 'A threshold on a quantity spanning decades must be relative'). The value is recorded with its measurement; R-37 says the value is set by measurement.
- R-397 (RQ-226), applied per R-369: TASK-M3-11 owns the storage of θ̃'s pole hold as well as θ̃ in the march, since it already edits the ledger with a schema-version change (REQ-VAL-055, REQ-PAY-059); no separate ledger task. The member keeps its name `_reserved` (R-343 and R-351 name its WGSL home `closure_step_reserved`), has no field entry, and gets no read-side accessor or catalogue view. The bring-up pattern keeps `_reserved = 0` (colour_composition Appendix A).
- R-397's quantisation applies wherever the march stores the reference: the hold's helper in `crates/kernel/src/shape.rs` (TASK-M1-11) may be kept, with the march quantising on entry, or changed. A TASK-M1-11 qa assertion R-397 forces to change, and TASK-M0-12's pinned list of the five hashed register entries (now seven), are ruling-forced exceptions to the qa-file rule, listed in the PR with their reasons (R-290, R-369).
- The lagged `n̂` register (REQ-PAY-059) is a new SimState member after R-397's recheck: it costs 8 B per 8 B or less of its width (no free tail remains at f32), and the departed bit's placement does not use `_reserved`.
