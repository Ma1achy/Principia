# TASK-M3-72 — The displacement form of the FTLE shadow, R-974's joint step (REQ-VAL-038), and R-976's calibration re-run

- **Milestone:** M3
- **Closes:** REQ-VAL-038
- **Depends on:** TASK-M3-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The f32 Benettin shadow's displacement form, as R-974 (RQ-793 (a)) rules it: a joint step on the occupant interface, `x` bit-identical to `step(x)` and `δ⃗` advanced through the same stages, the force difference formed from pair separations, plainly in f32 (a1) or cancellation-free (a2) as a pre-registered measurement chooses; the shadow's projection, energy and collision test formed from `x + δ⃗` where they need absolute positions, defined here under physics review; built beside TASK-M3-14's absolute form and tested against it on the known-Lyapunov periodic orbits, switching to displacement only if FTLE accuracy meaningfully improves (REQ-VAL-038). The definition is written into dd_integrator § 3.8 (R-72) as the ruling has it. Nothing named `Displacement` stays that is not one: TASK-M3-14's `Displacement` variant, the `shadow-form` gate's column of that name and `shadow_forms_agree` are replaced or removed once the form is built (physics review 5866988394 of PR #493, N6). It also re-runs REQ-INT-076's pre-registered eight-orbit calibration, unchanged, once this form and R-972's (d) + (a) are built, and proposes `min_regions` ≥ 6 (R-976).

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
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"
- `decisions.md` § "R-974 — RQ-793, the displacement form of the FTLE shadow: option (a), a joint step on the occupant interface; whether the force difference is formed plainly in f32 (a1) or cancellation-free (a2) is chosen by a measurement whose criterion TASK-M3-72 writes before any run, physics-reviewed *(closes RQ-793)*"
- `decisions.md` § "R-976 — RQ-794, what REQ-INT-076 proposes for the FTLE shadow's ‖δ₀‖ and `n_renorm`: option (c), (3e-3, 300, 0.2) stay provisional and stated as unsupported on close-encounter chaotic orbits wherever the FTLE is shown; the pre-registered eight-orbit calibration is re-run unchanged, then `min_regions` ≥ 6 proposed; the M3 gate confirms no final value until the re-run *(closes RQ-794)*"
- `decisions.md` § "R-972 — RQ-791, the FTLE when only its shadow reaches a terminal or goes non-finite: (d) + (a); the shadow's detectors monitor only and the shadow marches on through what would be its terminal, and once the shadow's `S` is non-finite `ftle_valid` is false, `ftle` reads NaN and every aggregate drops the sample *(closes RQ-791)*"

## Deliverables
- `crates/kernel`: the displacement-form shadow, as R-974 rules it, a joint step for each of KDK, Yoshida-4, Yoshida-6, RK4 and the default regularised occupant, in both (a1) and (a2) far enough to be measured, beside the absolute form, which stays the shipped form until the measurement and REQ-PAY-085's calibration say otherwise.
- Doc change: `docs/design/principia_dd_integrator.md` § 3.8 — the displacement form's definition, per R-974 (R-72), with the shadow's projection, energy and collision test from `x + δ⃗`; integrator contract Part 2 and Part 2a, the joint step.
- This file: the criterion that chooses (a1) or (a2), written before any run (R-649) and physics-reviewed.
- `crates/validation/src/measure/benettin.rs` and `fixtures/gates/shadow-form/` — both forms against the known exponent on the periodic-orbit set, at f32 and f64, with a control that the displacement form differs bitwise from the absolute form on some orbit.
- TASK-M3-14's `Displacement` variant, the `shadow-form` gate's column of that name and `shadow_forms_agree` replaced by the real form or removed (physics N6).

## Acceptance tests
- `cargo xtask gate shadow-form` — FTLE error vs the known exponent for both shadow forms on the periodic-orbit set; the switch to displacement is made only if accuracy meaningfully improves, the result recorded (REQ-VAL-038). The measurement is the one RQ-793 names, its criterion written before the run (R-649): the signed gap `λ_T(f32) − λ_T(f64)` on the shadow-form set at ‖δ₀‖ = 1e-5, judged by its magnitude, against the absolute form's +0.023, +0.055, −0.014 and +0.204; whether Burrau at f32 stays finite at ‖δ₀‖ ≤ 1e-5; and a control that fails when the displacement form is bit-identical to the absolute form on every orbit.
- § 3.8 states the displacement form as R-974 rules it; physics reviewer approved (REQ-VAL-038).
- The (a1)/(a2) measurement, by the criterion this file states before any run (R-649), its numbers and the form it chooses recorded; physics reviewer approved (REQ-VAL-038, R-974).
- `cargo xtask gate benettin-calibration` — R-976's re-run: the pre-registered eight-orbit calibration, its grid, meter, selection and tolerance rules unchanged, run after this form and R-972's (d) + (a) are built, proposing `min_regions` ≥ 6 with its evidence; the human confirms REQ-INT-076's values at the M3 gate (REQ-INT-076, closed by TASK-M3-14).

## Notes
- New (12 Nov 2026), applied per R-369 (task re-scoping, physics's recommendation in review 5866988394 of PR #493, § (4)): REQ-VAL-038 and its `shadow-form` acceptance arm moved here from TASK-M3-14, whose other deliverables do not depend on the form. The arm is TASK-M3-14's, unchanged, with the measurement and control RQ-793 names added.
- Waits on RQ-793 (what the displacement form is; a one-way door). Whether a gain is "meaningful" stays REQ-PAY-085's calibration, at M4 (TASK-M4-13, which depends on this task).
- RQ-794 (what REQ-INT-076 proposes for ‖δ₀‖ and `n_renorm`) waits on RQ-793 as well; where its re-run lands is that ruling's.
- *12 Nov 2026 (PR #497's code review 5867138483, notes 1 and 2):* the measurement is written as the signed gap, its figures being signed, and R-649 joins References. *Was:* "`|λ_T(f32) − λ_T(f64)|` on the shadow-form set at ‖δ₀‖ = 1e-5".
- *12 Nov 2026 (R-974, RQ-793 (a); R-976, RQ-794 (c)):* this task no longer waits on RQ-793. The (a1)/(a2) choice is made by a measurement whose criterion this file states before any run, physics-reviewed; both forms are built far enough to measure. REQ-INT-076's re-run lands here (applied per R-369: task scoping), run after the displacement form and R-972's (d) + (a) (TASK-M3-14) are built, since a re-run before them measures nothing new; REQ-INT-076 stays TASK-M3-14's, whose provisional proposal stands until the M3 gate confirms the re-run's. *Was (title):* "The displacement form of the FTLE shadow (REQ-VAL-038, waits on RQ-793)"; *was (Size):* "~400 lines, open until RQ-793's ruling fixes the form"; *was (Goal, the clause):* "as RQ-793's ruling defines it (what is stepped, how the force difference is formed, and what the shadow's own projection, monitor and detector see),"; *was (Goal, the last sentence):* "This task waits on RQ-793; it is not dispatched before the ruling." The two notes above on RQ-793 and RQ-794 are settled by these rulings.
