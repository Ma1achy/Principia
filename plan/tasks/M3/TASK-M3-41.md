# TASK-M3-41 — The regularised integration floor: the step budget as a rate, undetermined on exhaustion, the counted signals and the two energies' reading (R-563, R-564, R-566)

- **Milestone:** M3
- **Closes:** REQ-INT-101, REQ-INT-102, REQ-INT-103, REQ-VAL-195, REQ-INT-106, REQ-PAY-110, REQ-INT-108, REQ-PAY-111, REQ-INT-109, REQ-TOOL-219
- **Depends on:** TASK-M3-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~400 lines, the tests included

## Goal
Under a regularised occupant the integration floor is a step budget, not a substep cap (R-563). The budget is a rate, steps per unit physical time per ensemble copy (R-564), starting at about 4e4 per unit time per copy, a calibration this task proposes. A copy that runs out of it is undetermined, never discarded and never a physical outcome label; the separation floor and the steps held at the entry cap are counted, non-terminal signals. Under a regularised occupant the per-step suspect flag reads the regularised energy, and the Cartesian energy is a diagnostic read only from runs that did not stop at an event (R-566). With regularisation off nothing changes. R-603 to R-605 (25 Oct 2026) rule how: exhaustion a sticky `budget_exhausted` flag beside `saturated` and the two signals counters in a payload word, `saturated` set when either fires; the budget checked every step against `max(N_min, rate · t_elapsed)`, `N_min` a calibration; both energy drifts stored, the suspect flags and the drift view reading the regularised one, an event-stopped run drawn as now with "stopped at event" in its readout, and the energy-suspect threshold recalibrated on Heggie × RK4's regularised drift.

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
- `decisions.md` § "R-603 — A regularised copy's budget exhaustion is a sticky flag bit beside `saturated`, its two counted signals counters in a payload word, `saturated` set when either fires; an undetermined-by-budget footprint is never discarded, never treated as resolved and never split, its remedy finer steps as an explicit re-run *(closes RQ-526; a schema change)*"
- `decisions.md` § "R-604 — A regularised copy's step budget is checked every step: its running total against `max(N_min, rate · t_elapsed)`; `N_min` protects early close encounters and is an M3 calibration *(closes RQ-527; settles R-564's accounting)*"
- `decisions.md` § "R-605 — Both energy drifts are stored: the suspect flags and the drift view read the regularised drift, the Cartesian drift being diagnostic only; an event-stopped run draws as now, its readout saying "stopped at event"; the energy-suspect threshold is recalibrated on Heggie × RK4's regularised drift at the M3 gate, measured first *(closes RQ-528; a schema change)*"
- `docs/design/principia_dd_simstate_payload.md` § "2. Bit layouts (the packed u32s)"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"

## Deliverables
- `crates/kernel/src/regularisation/budget.rs` — the rate budget, checked every step against `max(N_min, rate · t_elapsed)` (R-604); the sticky `budget_exhausted` flag and the two counters (R-603), placed as this task defines them in payload §2 (REQ-PAY-110).
- The regularised drift latches stored beside the Cartesian ones (R-605), their fields defined in payload §1 and §5 (REQ-PAY-111); the suspect predicate and the drift view reading the regularised drift, the diagnostics' event filter, and the event-stopped run's "stopped at event" readout (REQ-TOOL-219).
- `cargo xtask gate step-budget --propose`'s `N_min` (REQ-INT-108) and `cargo xtask gate suspect-default-occupant --propose`, the recalibrated energy-suspect threshold (REQ-INT-109).
- *Was:* "`crates/kernel/src/regularisation/budget.rs` — the rate budget, accounted as RQ-527 rules; the exhaustion outcome and the two counters, stored as RQ-526 rules." and "The suspect predicate's energy under a regularised occupant, and the diagnostics' event filter, placed as RQ-528 rules." (R-603 to R-605)
- `cargo xtask gate step-budget --propose` — per-copy step rates on the budget's worst targets and the control at `t = 13` and `t = 50`.

## Acceptance tests
- `cargo test -p kernel budget_undetermined` — REQ-INT-101's verify (REQ-INT-101).
- `cargo test -p kernel budget_rate` — REQ-INT-102's verify (REQ-INT-102).
- `cargo xtask gate step-budget --propose` — the proposed rate with per-copy rates (p50, p99, max over copies) on `burrau_nu_k`, `latent_mixed_h3`, `config_stability` and the control `latent_shape` at `t = 13` and `t = 50`, clearing every copy; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-INT-103).
- `cargo test -p kernel energy_named` — REQ-VAL-195's verify (REQ-VAL-195).
- `cargo xtask gate suspect-default-occupant` — REQ-INT-106's verify: Heggie × RK4's regularised drift on dd tests 2 and 7 and the predicates it lights under REQ-INT-085's proposed thresholds, recorded beside REQ-INT-085's record, the evidence for REQ-INT-109 (REQ-INT-106). *Was:* "what it must light is RQ-528 (d)" (R-605).
- `cargo xtask gate step-budget --propose` — with the rate, the per-copy running step totals (p50, p99, max over copies) at each copy's earliest close encounter on the budget's worst targets `burrau_nu_k`, `latent_mixed_h3` and `config_stability` and the clean control `latent_shape`, and the proposed `N_min`, no copy exhausted before its first close encounter is passed (qa review 5644656634 of PR #293, Q5; *was:* "with the rate, the per-copy running step totals at each copy's earliest close encounter and the proposed `N_min`"); the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-INT-108).
- `cargo xtask gate suspect-default-occupant --propose` — REQ-INT-109's verify: the energy-suspect threshold recalibrated on REQ-INT-106's measured regularised drift, with which predicates light at it; at that value, what Euler, RK4, KDK, Yoshida-4 and Yoshida-6 light on dd tests 2 and 7 at each precision, and whether REQ-INT-085's and REQ-VAL-031's pass conditions (Euler lighting `SUSPECT_ENERGY` on test 2) still hold, the value replacing REQ-INT-085's one threshold; the human confirms it at the M3 gate (REQ-INT-109; physics review 5644374613 of PR #293, F2).
- `cargo test -p render drift_view_regularised` — REQ-TOOL-219's verify, the render arm: the drift view's input bitwise the regularised latch, and the event-stopped pixel carrying its stopped-at-event status for the readout (control: the status dropped fails) (REQ-TOOL-219).
- `cargo test -p gui drift_readout_stopped_at_event` — REQ-TOOL-219's verify, the gui arm: the event-stopped pixel's hover readout in the energy-drift view reads "stopped at event" (control: a readout without the words fails) (REQ-TOOL-219; qa review 5644656634 of PR #293, Q1, applied per R-369: the readout is the gui crate's, so gui joins the reviewers). *Was:* "— REQ-TOOL-219's verify (REQ-TOOL-219)."
- Review checklist (physics and code) — payload §2 places `budget_exhausted` and the two counters (REQ-PAY-110), and payload §1 and §5 the regularised drift fields (REQ-PAY-111), each with the ledger's hash and the schema version changed; approved before merge.

## Notes
- R-603, R-604 and R-605 (25 Oct 2026) rule RQ-526, RQ-527 and RQ-528; nothing waits. *Was:* "Waits on RQ-526 (how exhaustion and the counts are stored and drive the floor), RQ-527 (how the rate is accounted) and RQ-528 (where the regularised drift is stored and how an event-stopped run is drawn). Nothing here is built before they are ruled."
- Definitions written here (R-72; physics and code reviewers approve before merge): REQ-PAY-110, REQ-PAY-111 (schema changes, the version following from the ledger's hash, R-36). Calibrations proposed here: REQ-INT-108, REQ-INT-109 (after REQ-INT-106's measurement).
- R-563, R-564, R-566 (21 Oct 2026). The evidence is prin-rs's `budget_horizon_probe` and `results/step_budget/` (PR #34; `docs/incoming/2026-10-21_gap_sweep_integrator/`, a record). `error_ratio` is blind to a truncated run, so the budget's outcome is read directly, never from drift.
- Calibration proposed here (R-71): REQ-INT-103. REQ-INT-106 extends REQ-INT-085's evidence to the default occupant (physics review 5605806092 of PR #277, F6); the separation floor and the entry-cap hit are each occupant's, integrator contract Part 6 (F3), with `TINY` REQ-INT-105's value.
