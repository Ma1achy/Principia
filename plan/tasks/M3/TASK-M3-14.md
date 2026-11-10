# TASK-M3-14 — The Benettin FTLE shadow

- **Milestone:** M3
- **Closes:** REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038, REQ-CHART-132
- **Depends on:** TASK-M3-05, TASK-M3-06, TASK-M2-71, TASK-M3-17
- **Needs (earlier milestones):** REQ-PAY-021, REQ-PAY-026, REQ-PAY-032
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
Every sample carries its own Benettin shadow (FTLE tier): one shadow renormalised every `n_renorm` steps (`S += log(δ/δ₀)`, reset to `x + δ₀·δ⃗/δ`), finalised at read or termination with the partial interval (`S_final = S + log(δ_current/δ₀)`, `λ = S_final/(step_count·dt_macro)`), `completed_renorms` derived. Each marching trajectory — base, shadow, ensemble copy — is CoM-projected, monitored and detected independently; shadows never enter the colour/spread pool. δ₀'s direction is defined in dd_integrator §3.8 (R-72), and ‖δ₀‖ and `n_renorm` are calibrated (R-71). The displacement-vs-absolute f32 shadow choice is tested on known-Lyapunov orbits.

## References
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"
- `docs/design/principia_dd_integrator.md` § "3.4 COM projection (per `STEP`; the policy is integrator contract Part 1)"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"

## Deliverables
- `crates/kernel/src/driver/benettin.rs` — the shadow march through the same wrapper cadence (own projection), renormalisation, finalisation at read.
- Doc change: `docs/design/principia_dd_integrator.md` §3.8 — δ₀'s direction, deterministic and the same on CPU and GPU (definition, REQ-INT-077).
- `crates/validation/src/measure/benettin.rs` + `fixtures/gates/benettin/` — dd test 10 at candidate (‖δ₀‖, n_renorm), f32 and f64; displacement vs absolute shadow FTLE error on known-Lyapunov periodic orbits.
- Calibration proposal for ‖δ₀‖ and `n_renorm` with the evidence, for decisions.md at the M3 gate.

## Acceptance tests
- `cargo test -p kernel shadow_not_in_pool` — dd test 11: shadows are structurally excluded from the colour/spread pool (REQ-INT-035).
- `cargo test -p kernel shadow_own_projection` — base + shadow with deliberately different roundoff: each computes its own R_com; FTLE unchanged vs a reference that projects independently (REQ-INT-039).
- `cargo test -p kernel benettin_ftle` — dd test 10: halving δ₀ and doubling n_renorm leaves λ_T unchanged within tolerance; λ_T ≈ 0 on the Kepler-embedded orbit, large on Burrau; reading between renorm boundaries is not biased low (REQ-INT-043).
- `cargo xtask gate benettin-calibration` — the proposal shows dd test 10 at the proposed ‖δ₀‖ and n_renorm at f32 and f64; the human confirms the values at the M3 gate and they are recorded in decisions.md (REQ-INT-076).
- §3.8 fixes δ₀'s direction deterministically (same on CPU and GPU); physics reviewer approved (REQ-INT-077).
- `cargo xtask gate shadow-form` — FTLE error vs the known exponent for both shadow forms on the periodic-orbit set; the switch to displacement is made only if accuracy meaningfully improves, the result recorded (REQ-VAL-038).
- `cargo test -p engine ftle_chart_independent` — a linear chart, a warped one at a pixel where the warp's value is exact, and a third with a different tangent through the same state, a tilted pair, decode one bit pattern and give one lambda_T bit for bit, read on copy 0; control: delta_0's direction taken from a chart tangent fails (REQ-CHART-132).
- `cargo test -p kernel ftle_frozen_at_escape` — on a fixture escaper at the FTLE tier, its escape firing off a renorm boundary, S advances while running and is unchanged by later dispatches after the escape firing step (TASK-M3-17's latch), and the derived FTLE (REQ-PAY-021: S_final/(step_count·dt_macro), the partial renorm interval finalised from the shadow's δ at the firing step) is frozen at t_esc (the derived-FTLE arm REQ-PAY-058 names; that requirement is closed by TASK-M3-17); control: a plain S/t variant, the partial interval left unfinalised, fails (REQ-PAY-058).

## Notes
- 'Within tolerance' for dd test 10 is set by the REQ-INT-076 proposal (calibrated).
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-INT-076.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-077.
- *9 Nov 2026 (R-916):* REQ-CHART-132 (the FTLE field's chart independence) added, and this task re-queued after M2's gate (R-918); the header follows plan/tasks.yaml. *Was:* "Closes: REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038"; "Depends on: TASK-M3-05, TASK-M3-06".
- *9 Nov 2026 (R-918):* stopped and re-queued after M2's gate; it gains REQ-CHART-132 (the FTLE field's chart independence, from the retired TASK-M8-100), since that needs the Benettin shadow's initialisation (F4), and TASK-M2-71 in Depends on.
- Applied per R-369 (10 Nov 2026): the derived-FTLE arm of REQ-PAY-058 ("the derived FTLE equals S/T at t_esc") moved here from TASK-M3-17's `escape_freezes_accumulators` as `ftle_frozen_at_escape`, so that TASK-M3-17 no longer waits for this stopped task; it needs TASK-M3-17's escape latch, so TASK-M3-17 joins Depends on and R-95 joins References. *Was:* "Depends on: TASK-M3-05, TASK-M3-06, TASK-M2-71".
- Applied per R-369 (10 Nov 2026): record correction, the shorthand named REQ-PAY-021's formula; physics review 5836486519. The fixture escaper fires off a renorm boundary, with a plain S/t control. *Was:* "the derived FTLE equals S/T at t_esc (the derived-FTLE arm REQ-PAY-058 names; that requirement is closed by TASK-M3-17) (REQ-PAY-058)"; "on a fixture escaper at the FTLE tier, S advances".
