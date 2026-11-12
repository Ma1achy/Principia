# TASK-M3-72 — The displacement form of the FTLE shadow (REQ-VAL-038, waits on RQ-793)

- **Milestone:** M3
- **Closes:** REQ-VAL-038
- **Depends on:** TASK-M3-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, open until RQ-793's ruling fixes the form

## Goal
The f32 Benettin shadow's displacement form, as RQ-793's ruling defines it (what is stepped, how the force difference is formed, and what the shadow's own projection, monitor and detector see), built beside TASK-M3-14's absolute form and tested against it on the known-Lyapunov periodic orbits, switching to displacement only if FTLE accuracy meaningfully improves (REQ-VAL-038). The definition is written into dd_integrator § 3.8 (R-72) as the ruling has it. Nothing named `Displacement` stays that is not one: TASK-M3-14's `Displacement` variant, the `shadow-form` gate's column of that name and `shadow_forms_agree` are replaced or removed once the form is built (physics review 5866988394 of PR #493, N6). This task waits on RQ-793; it is not dispatched before the ruling.

## References
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"
- `docs/design/principia_dd_integrator.md` § "3.4 COM projection (per `STEP`; the policy is integrator contract Part 1)"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `crates/kernel`: the displacement-form shadow, as RQ-793's ruling defines it, beside the absolute form, which stays the shipped form until the measurement and REQ-PAY-085's calibration say otherwise.
- Doc change: `docs/design/principia_dd_integrator.md` § 3.8 — the displacement form's definition, per RQ-793's ruling (R-72).
- `crates/validation/src/measure/benettin.rs` and `fixtures/gates/shadow-form/` — both forms against the known exponent on the periodic-orbit set, at f32 and f64, with a control that the displacement form differs bitwise from the absolute form on some orbit.
- TASK-M3-14's `Displacement` variant, the `shadow-form` gate's column of that name and `shadow_forms_agree` replaced by the real form or removed (physics N6).

## Acceptance tests
- `cargo xtask gate shadow-form` — FTLE error vs the known exponent for both shadow forms on the periodic-orbit set; the switch to displacement is made only if accuracy meaningfully improves, the result recorded (REQ-VAL-038). The measurement is the one RQ-793 names, its criterion written before the run (R-649): `|λ_T(f32) − λ_T(f64)|` on the shadow-form set at ‖δ₀‖ = 1e-5, against the absolute form's +0.023, +0.055, −0.014 and +0.204; whether Burrau at f32 stays finite at ‖δ₀‖ ≤ 1e-5; and a control that fails when the displacement form is bit-identical to the absolute form on every orbit.
- § 3.8 states the displacement form as RQ-793's ruling defines it; physics reviewer approved (REQ-VAL-038).

## Notes
- New (12 Nov 2026), applied per R-369 (task re-scoping, physics's recommendation in review 5866988394 of PR #493, § (4)): REQ-VAL-038 and its `shadow-form` acceptance arm moved here from TASK-M3-14, whose other deliverables do not depend on the form. The arm is TASK-M3-14's, unchanged, with the measurement and control RQ-793 names added.
- Waits on RQ-793 (what the displacement form is; a one-way door). Whether a gain is "meaningful" stays REQ-PAY-085's calibration, at M4 (TASK-M4-13, which depends on this task).
- RQ-794 (what REQ-INT-076 proposes for ‖δ₀‖ and `n_renorm`) waits on RQ-793 as well; where its re-run lands is that ruling's.
