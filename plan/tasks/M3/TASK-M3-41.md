# TASK-M3-41 — The regularised integration floor: the step budget as a rate, undetermined on exhaustion, the counted signals and the two energies' reading (R-563, R-564, R-566)

- **Milestone:** M3
- **Closes:** REQ-INT-101, REQ-INT-102, REQ-INT-103, REQ-VAL-195, REQ-INT-106
- **Depends on:** TASK-M3-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines, the tests included

## Goal
Under a regularised occupant the integration floor is a step budget, not a substep cap (R-563). The budget is a rate, steps per unit physical time per ensemble copy (R-564), starting at about 4e4 per unit time per copy, a calibration this task proposes. A copy that runs out of it is undetermined, never discarded and never a physical outcome label; the separation floor and the steps held at the entry cap are counted, non-terminal signals. Under a regularised occupant the per-step suspect flag reads the regularised energy, and the Cartesian energy is a diagnostic read only from runs that did not stop at an event (R-566). With regularisation off nothing changes.

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `docs/design/principia_dd_integrator.md` § "3.5 Invariant monitoring (per `STEP`, post-projection)"
- `docs/design/principia_dd_refinement_policy.md` § "1. The split rule — no exponent in the split test"
- `docs/design/principia_deep_zoom.md` § "Reserved: the integration floor"
- `decisions.md` § "R-563 — The integration floor follows the occupant: with regularisation off the substep machinery is unchanged; under a regularised occupant running out of the step budget makes the result undetermined, never discarded, and the separation floor and the entry-cap hits are counted, non-terminal signals; REQ-SCHED-063 and REQ-SCHED-077 are re-pointed"
- `decisions.md` § "R-564 — A regularised occupant's step budget is a rate, steps per unit physical time per copy; its value starts at about 4e4 per unit time per copy, a calibration"
- `decisions.md` § "R-566 — Under a regularised occupant the energy is named twice: the two-body collision gate (REQ-VAL-045) and any per-step suspect flag read the regularised energy; the Cartesian energy is for diagnostics, read only from runs that did not stop at an event *(refines R-257 as to the radial collision gate's energy)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"

## Deliverables
- `crates/kernel/src/regularisation/budget.rs` — the rate budget, accounted as RQ-527 rules; the exhaustion outcome and the two counters, stored as RQ-526 rules.
- The suspect predicate's energy under a regularised occupant, and the diagnostics' event filter, placed as RQ-528 rules.
- `cargo xtask gate step-budget --propose` — per-copy step rates on the budget's worst targets and the control at `t = 13` and `t = 50`.

## Acceptance tests
- `cargo test -p kernel budget_undetermined` — REQ-INT-101's verify (REQ-INT-101).
- `cargo test -p kernel budget_rate` — REQ-INT-102's verify (REQ-INT-102).
- `cargo xtask gate step-budget --propose` — the proposed rate with per-copy rates (p50, p99, max over copies) on `burrau_nu_k`, `latent_mixed_h3`, `config_stability` and the control `latent_shape` at `t = 13` and `t = 50`, clearing every copy; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-INT-103).
- `cargo test -p kernel energy_named` — REQ-VAL-195's verify (REQ-VAL-195).
- `cargo xtask gate suspect-default-occupant` — REQ-INT-106's verify: Heggie × RK4's regularised drift on dd tests 2 and 7 and the predicates it lights under REQ-INT-085's proposed thresholds, recorded beside REQ-INT-085's record; what it must light is RQ-528 (d) (REQ-INT-106).

## Notes
- Waits on RQ-526 (how exhaustion and the counts are stored and drive the floor), RQ-527 (how the rate is accounted) and RQ-528 (where the regularised drift is stored and how an event-stopped run is drawn). Nothing here is built before they are ruled.
- R-563, R-564, R-566 (21 Oct 2026). The evidence is prin-rs's `budget_horizon_probe` and `results/step_budget/` (PR #34; `docs/incoming/2026-10-21_gap_sweep_integrator/`, a record). `error_ratio` is blind to a truncated run, so the budget's outcome is read directly, never from drift.
- Calibration proposed here (R-71): REQ-INT-103. REQ-INT-106 extends REQ-INT-085's evidence to the default occupant (physics review 5605806092 of PR #277, F6); the separation floor and the entry-cap hit are each occupant's, integrator contract Part 6 (F3), with `TINY` REQ-INT-105's value.
