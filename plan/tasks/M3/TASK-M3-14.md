# TASK-M3-14 — The Benettin FTLE shadow

- **Milestone:** M3
- **Closes:** REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077
- **Depends on:** TASK-M3-05, TASK-M3-06
- **Needs (earlier milestones):** REQ-PAY-021, REQ-PAY-026, REQ-PAY-032
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
Every sample carries its own Benettin shadow (FTLE tier): one shadow renormalised every `n_renorm` steps (`S += log(δ/δ₀)`, reset to `x + δ₀·δ⃗/δ`), finalised at read or termination with the partial interval (`S_final = S + log(δ_current/δ₀)`, `λ = S_final/(step_count·dt_macro)`), `completed_renorms` derived. Each marching trajectory — base, shadow, ensemble copy — is CoM-projected, monitored and detected independently; shadows never enter the colour/spread pool. δ₀'s direction is defined in dd_integrator §3.8 (R-72), and ‖δ₀‖ and `n_renorm` are calibrated (R-71).

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
- `decisions.md` § "R-924 — TASK-M3-14 (the Benettin FTLE shadow) resumes now *(amends R-918 and R-923 in part)*"
- `decisions.md` § "R-972 — RQ-791, the FTLE when only its shadow reaches a terminal or goes non-finite: (d) + (a); the shadow's detectors monitor only and the shadow marches on through what would be its terminal, and once the shadow's `S` is non-finite `ftle_valid` is false, `ftle` reads NaN and every aggregate drops the sample *(closes RQ-791)*"
- `decisions.md` § "R-976 — RQ-794, what REQ-INT-076 proposes for the FTLE shadow's ‖δ₀‖ and `n_renorm`: option (c), (3e-3, 300, 0.2) stay provisional and stated as unsupported on close-encounter chaotic orbits wherever the FTLE is shown; the pre-registered eight-orbit calibration is re-run unchanged, then `min_regions` ≥ 6 proposed; the M3 gate confirms no final value until the re-run *(closes RQ-794)*"

## Deliverables
- `crates/kernel/src/driver/benettin.rs` — the shadow march through the same wrapper cadence (own projection), renormalisation, finalisation at read.
- Doc change: `docs/design/principia_dd_integrator.md` §3.8 — δ₀'s direction, deterministic and the same on CPU and GPU (definition, REQ-INT-077).
- `crates/validation/src/measure/benettin.rs` + `fixtures/gates/benettin-calibration/` and `fixtures/gates/shadow-form/` — dd test 10 at candidate (‖δ₀‖, n_renorm), f32 and f64; displacement vs absolute shadow FTLE error on known-Lyapunov periodic orbits. *11 Nov 2026, applied per R-369 (record correction, PR #493's code and qa reviews):* the fixtures sit under the two gates' names, as the acceptance commands name them. *Was:* "`crates/validation/src/measure/benettin.rs` + `fixtures/gates/benettin/` — dd test 10 at candidate (‖δ₀‖, n_renorm), f32 and f64; displacement vs absolute shadow FTLE error on known-Lyapunov periodic orbits."
- Calibration proposal for ‖δ₀‖ and `n_renorm` with the evidence, for decisions.md at the M3 gate.

## Acceptance tests
- `cargo test -p kernel shadow_not_in_pool` — dd test 11: shadows are structurally excluded from the colour/spread pool (REQ-INT-035).
- `cargo test -p kernel shadow_own_projection` — base + shadow with deliberately different roundoff: each computes its own R_com; FTLE unchanged vs a reference that projects independently (REQ-INT-039).
- `cargo test -p kernel benettin_ftle` — dd test 10: halving δ₀ and doubling n_renorm leaves λ_T unchanged within tolerance; λ_T ≈ 0 on the Kepler-embedded orbit, large on Burrau; reading between renorm boundaries is not biased low (REQ-INT-043).
- `cargo xtask gate benettin-calibration` — the proposal shows dd test 10 at the proposed ‖δ₀‖ and n_renorm at f32 and f64; the human confirms the values at the M3 gate and they are recorded in decisions.md (REQ-INT-076).
- §3.8 fixes δ₀'s direction deterministically (same on CPU and GPU); physics reviewer approved (REQ-INT-077).

## Notes
- 'Within tolerance' for dd test 10 is set by the REQ-INT-076 proposal (calibrated).
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-INT-076.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-077.
- *9 Nov 2026 (R-916):* REQ-CHART-132 (the FTLE field's chart independence) added, and this task re-queued after M2's gate (R-918); the header follows plan/tasks.yaml. *Was:* "Closes: REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038"; "Depends on: TASK-M3-05, TASK-M3-06".
- *9 Nov 2026 (R-918):* stopped and re-queued after M2's gate; it gains REQ-CHART-132 (the FTLE field's chart independence, from the retired TASK-M8-100), since that needs the Benettin shadow's initialisation (F4), and TASK-M2-71 in Depends on.
- Applied per R-369 (10 Nov 2026), following R-924: this task resumes now (R-924 lifts R-918's stop); following it, REQ-CHART-132 and its `ftle_chart_independent` arm, which need TASK-M2-71's warped chart, move to TASK-M3-69, and TASK-M2-71 leaves Depends on. TASK-M3-17 depends on this task, and its `escape_freezes_accumulators` asserts REQ-PAY-058's derived-FTLE arm on this task's shadow. R-924 joins References. *Was:* "Closes: REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038, REQ-CHART-132"; "Depends on: TASK-M3-05, TASK-M3-06, TASK-M2-71"; "- `cargo test -p engine ftle_chart_independent` — a linear chart, a warped one at a pixel where the warp's value is exact, and a third with a different tangent through the same state, a tilted pair, decode one bit pattern and give one lambda_T bit for bit, read on copy 0; control: delta_0's direction taken from a chart tangent fails (REQ-CHART-132)." (now TASK-M3-69's, unchanged).
- Applied per R-369 (12 Nov 2026; task re-scoping, physics's recommendation in review 5866988394 of PR #493, § (4)): REQ-VAL-038 and its `shadow-form` acceptance arm move to TASK-M3-72, the displacement form of the FTLE shadow, which waits on RQ-793 (what that form is; PR #493's `Displacement` is the absolute form by construction). This task's other deliverables do not depend on it. The Deliverables' "displacement vs absolute shadow FTLE error" clause is left as written, PR #493 having amended that line: what this task builds of it is the absolute form's measurement and the `shadow-form` gate as PR #493 has them, the baseline TASK-M3-72 measures against. REQ-INT-076's proposal waits on RQ-794 (what it proposes, given the six-orbit sweep). *Was:* "Closes: REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038"; in the Goal, "The displacement-vs-absolute f32 shadow choice is tested on known-Lyapunov orbits."; in the Acceptance tests, "- `cargo xtask gate shadow-form` — FTLE error vs the known exponent for both shadow forms on the periodic-orbit set; the switch to displacement is made only if accuracy meaningfully improves, the result recorded (REQ-VAL-038)." (now TASK-M3-72's, unchanged, its measurement added).
- *12 Nov 2026 (R-972, RQ-791 (d) + (a)):* this task builds both: a shadow's detectors monitor only, the shadow marching on through what would be its terminal (dd_integrator §3.4), and `ftle_valid`'s clause, false once the shadow's `S` is non-finite, with the mark that records it defined here under physics review (payload §6; R-79: storage never holds NaN). `shadow_own_projection` (REQ-INT-039) gains REQ-INT-039's two arms at a nonzero `r_coll`: a shadow-only collision, where the shadow marches on and the value stays valid, and a non-finite shadow `S`, where `ftle_valid` is false and `ftle` reads NaN; REQ-PAY-032's new clause is checked by the second.
- *12 Nov 2026 (R-976, RQ-794 (c)):* REQ-INT-076's proposal stays (3e-3, 300, 0.2), marked provisional and stated as unsupported on close-encounter chaotic orbits, with the eight-orbit evidence; the M3 gate confirms no final value until the re-run, which is TASK-M3-72's (applied per R-369: task scoping).
