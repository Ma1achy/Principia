# TASK-M6-19 — The sea_fraction(eps) estimator

- **Milestone:** M6
- **Closes:** REQ-REF-031, REQ-REF-044, REQ-REF-049
- **Depends on:** TASK-M6-03, TASK-M6-17
- **Needs (earlier milestones):** REQ-REF-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
A cheap `sea_fraction(eps)` estimator, computable before any descent, built after the tier controller (R-48), that feeds the tier inversion. It is checked against the full-cache footprint-spread CDF on the tolerance-study fixtures; the task proposes its accuracy target with the tier-derivation error that error costs (R-71).

## References
- `docs/design/principia_dd_refinement_policy.md` § "5.1 `sea_fraction` is the regime test, and it is computable before any descent"
- `docs/design/principia_dd_refinement_policy.md` § "7. Open"
- `decisions.md` § "R-48 — The `sea_fraction` estimator is built after the tier controller *(RS-8)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "v0.5 means the NUMBERS are guesses; the SHAPE is not"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/engine/src/refine/sea_fraction.rs`: the estimator, implementing the trait TASK-M6-16's inversion reads.
- `crates/validation/src/sea_fraction.rs` + `cargo xtask gate sea-fraction` (estimator vs full-cache CDF; tier-derivation error).

## Acceptance tests
- `cargo test -p engine sea_fraction_estimator` (tolerance: REQ-REF-044 (calibrated)) — estimator's sea_fraction matches the full-cache footprint-spread CDF on the tolerance-study fixtures within a recorded tolerance (REQ-REF-031).
- `cargo xtask gate sea-fraction` — decisions.md records the target with its evidence: estimator sea_fraction against the full-cache footprint-spread CDF on the tolerance-study fixtures, and the tier-derivation error that error costs; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-REF-044).
- Definition: the sea_fraction estimator's method written into refinement_policy §5.1 and approved by the physics reviewer (REQ-REF-049).

## Notes
- Calibration proposed: REQ-REF-044. The estimator's method is not given by the corpus ("the concrete unbuilt next step") — see Gaps.
- Closes, for gaps the corpus leaves open: REQ-REF-049 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
