# TASK-M3-14 — The Benettin FTLE shadow

- **Milestone:** M3
- **Closes:** REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038
- **Depends on:** TASK-M3-05, TASK-M3-06
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

## Notes
- 'Within tolerance' for dd test 10 is set by the REQ-INT-076 proposal (calibrated).
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-INT-076.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-077.
