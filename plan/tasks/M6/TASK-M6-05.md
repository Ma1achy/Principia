# TASK-M6-05 — error_ratio as the trust flag, and the refinement gate / integrator tolerance pair

- **Milestone:** M6
- **Closes:** REQ-REF-018, REQ-PAY-080, REQ-PAY-081, REQ-PAY-082, REQ-VAL-088, REQ-VAL-089
- **Depends on:** TASK-M6-03, TASK-M6-04
- **Needs (earlier milestones):** REQ-REF-003, REQ-REF-004, REQ-INT-026, REQ-INT-028
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~380 lines

## Goal
`error_ratio` becomes the estimator trust flag (R-87): its departure from 1.0 marks estimator failure and triggers acquiring a third scale; there is no `failed_fraction`. The task writes how `error_ratio` enters `ensemble_spread` and "unresolved" (R-72, dd_generation_root §3.7), proposes the departure threshold (R-71), and proposes the refinement gate threshold and the integrator tolerance as one pair from a sweep that includes the ledger's eta = 0.005 and 0.02 cases (R-71). Frame certification uses `error_ratio_max`; `alpha_E` certifies nothing.

## References
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `decisions.md` § "R-87 — `failed_fraction` is retired *(closes RQ-38)*"
- `docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_generation_root.md` § "Open"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_predictability_horizon.md` § "7.1 `alpha_E` is structurally blind to the measurement horizon — my experiment could not work"
- `docs/design/principia_dd_predictability_horizon.md` § "7.4 The `t=40` failure is a rate, and mostly an aggregation artefact"

## Deliverables
- Doc change: `docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors" replaces "how error_ratio enters is OPEN" with the rule (REQ-PAY-081).
- `crates/engine/src/refine/trust.rs`: the boolean trust flag from `error_ratio` and the calibrated threshold; third-scale acquisition hook in the refinement walk.
- `crates/validation/src/trust_bar.rs` + `cargo xtask gate error-ratio-threshold` (RC §7.19c cases, false-alarm rate) and `cargo xtask gate gate-eta-pair` (the sweep).
- A CI grep in `xtask` that no `failed_fraction` identifier and no acceptance gate on `alpha_E` exists.

## Acceptance tests
- `cargo test -p engine error_ratio_third_scale` — an ensemble whose error_ratio departs from 1.0 triggers third-scale acquisition; no failed_fraction field or reference exists (REQ-REF-018).
- `cargo xtask gate error-ratio-threshold` — the threshold separates exact-dynamics footprints from integration-error footprints on the measured cases (RC §7.19c), with its false-alarm rate reported; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-PAY-080).
- Review checklist (physics) — the ledger replaces "how error_ratio enters is OPEN" with the rule, consistent with R-87 (indeterminate is read from error_ratio); the doc change is merged with the physics reviewer's approval (REQ-PAY-081).
- `cargo xtask gate gate-eta-pair` — a sweep over the pair (the ledger's eta = 0.005 and 0.02 cases included) showing the chosen pair reaches the trust bar; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-PAY-082).
- `cargo xtask gate gate-eta-pair` (the recorded (threshold, eta) pair) — a recorded measurement fixes (threshold, eta) together (REQ-VAL-088).
- Review checklist (physics) — no acceptance gate uses alpha_E; frame certification uses error_ratio_max (REQ-VAL-089).

## Notes
- Calibrations proposed: REQ-PAY-080 (error_ratio threshold), REQ-PAY-082 (gate threshold + integrator tolerance pair; REQ-VAL-088 is closed by the same recorded measurement).
- Definition written: REQ-PAY-081, consistent with R-87 (indeterminate is read from `error_ratio`).
- "Acquire a third scale" is named but not specified (which scale, when) — see Gaps.
