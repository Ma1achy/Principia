# TASK-M3-07 — The regularisation axis, `none`, and the no-usable-step marker

- **Milestone:** M3
- **Closes:** REQ-INT-021, REQ-TOOL-039, REQ-TOOL-125
- **Depends on:** TASK-M3-04, TASK-M3-05
- **Needs (earlier milestones):** REQ-DEC-005, REQ-INT-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-9
- **Size:** ~200 lines

## Goal
Regularisation is a second occupant slot independent of the stepper (`none` / AZ / Heggie / logH, integrator contract Part 2b). This task builds the axis: the regularisation slot with Part 2b's four kinds, the composition `Composed<S, R>` implementing `Advance` with `owns_time_mapping` reported for the composed occupant, `none` itself, the data type of Part 2b's pair table (each cell measured good, measured bad or not measured, with its warning), and the per-trajectory `dt_max` diagnostic with its explicit "no usable step" marker: everything that needs no Aarseth–Zare equation. Aarseth–Zare itself is TASK-M3-68. *Was:* "This task builds the axis, `none`, and Aarseth–Zare: two pairs with the reference body from the longest side, re-registering at every sync boundary, stepping in fictitious time `tau` on a fixed tau-schedule until it lands exactly on `t_target` — the final step shrunk by `dtau_eff = min(dtau, max(dt_left − t_so_far, 0)/(A·B))` — with the conditioned Levi-Civita inverse branch. `owns_time_mapping` is reported for the composed occupant; the AZ slot accepts a Mikkola–Tanikawa time-transformed leapfrog while RK4 fills it with `symplectic: false`." (moved to TASK-M3-68).

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `decisions.md` § "R-19 — Change 8 landed in full *(closes RQ-17)*"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"

## Deliverables
- `crates/kernel/src/regularisation/mod.rs` — the `Regularisation` slot with Part 2b's four kinds, composition `Composed<S, R>` implementing `Advance`, `owns_time_mapping = (R != none)`, and `none`.
- The data type of Part 2b's pair table: one cell per stepper × regularisation × time transformation, each measured good, measured bad or not measured, a measured-bad cell carrying its warning (R-561).
- A per-trajectory diagnostic for the largest physical step taken that reports 'no usable step' as an explicit sentinel, never 0.0 (min/max folds reseeded from the first element).

## Acceptance tests
- `cargo test -p kernel regularisation_compose` — every stepper × regularisation pair, `none` included, can be selected and runs; `none` is always available; the pairs the Part 2b table marks measured bad carry its warning and the others none; controls: a selection that blocks a measured-bad pair fails, and a measured-bad pair with no warning fails (REQ-INT-017; R-561). *Was:* "every stepper composes with every regularisation occupant; `none` is always available". Heggie's pairs are built by TASK-M3-08; this task's arm covers the pairs it builds and the table's data type, and TASK-M3-39 measures the table. REQ-INT-017 closes in TASK-M3-39, which re-runs this test over every pair once Heggie's exist (qa review 5607548278 of PR #277, Q8, applied per R-369; *was:* closed here). Applied per R-369 (10 Nov 2026): split: the pairs this task builds are `none`'s; TASK-M3-68 adds Aarseth–Zare's.
- `cargo test -p kernel owns_time_mapping_composed` — enumerate every stepper × regularisation pair: owns_time_mapping == (regularisation != none) (REQ-INT-021). Applied per R-369 (10 Nov 2026): the Aarseth–Zare arm, ADVANCE landing on the sync boundary, moves to TASK-M3-68. *Was:* "…; for AZ, ADVANCE to a sync boundary ends with physical time exactly t_target, and to a target between boundaries at the boundary before it (R-565) (REQ-INT-021)."
- `cargo test -p kernel no_usable_step_sentinel` — a fixture with all steps unusable reports the explicit none sentinel, not 0.0; the 2.209e128 overflow case is reported (REQ-TOOL-039).
- Definition: dt_max's home (occupant diagnostic or ledger field) and its 'no usable step' sentinel written into integrator_contract Part 2c and approved by the physics reviewer (REQ-TOOL-125).

## Notes
- Applied per R-369 (10 Nov 2026): split. TASK-M3-07 was "The regularisation axis and the Aarseth–Zare occupant"; Aarseth–Zare's equations were not in the corpus (RQ-756) and are now transcribed into integrator contract Part 2b § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)". This task keeps what needs none of them; TASK-M3-68 builds Aarseth–Zare and takes REQ-INT-024, REQ-INT-044, REQ-INT-049, REQ-INT-054 and REQ-INT-080, the Aarseth–Zare arms of `regularisation_compose` and `owns_time_mapping_composed`, and REQ-INT-105's Aarseth–Zare proposal.
- REQ-TOOL-039's fixtures are sequences of physical steps fed to the diagnostic, the 2.209e128 case among them, so none needs Aarseth–Zare (applied per R-369, 10 Nov 2026); TASK-M3-68 feeds the diagnostic from Aarseth–Zare's march.
- Gap: `dt_max` has no home in the new ledger; REQ-TOOL-039 is met by the occupant diagnostic above unless a ruling places it in the payload.
- Closes, for gaps the corpus leaves open: REQ-TOOL-125 (R-72 definition) (classification accepted by R-132). *Was:* "REQ-INT-080 (R-71 calibration), REQ-TOOL-125 (R-72 definition)"; REQ-INT-080 moves to TASK-M3-68.
- *11 Nov 2026, applied per R-369 (PR #493's physics review, point D):* TASK-M3-14's shadow march (`CoMarch`, `kernel::driver::benettin`) keeps lockstep with the sample in the steppers' terms: the sample's `N_sub` and the substeps the sample took (`taken`). That covers the steppers only. A regularised occupant that implements `Advance` (`Composed<S, R>`) owns its time mapping, so it states how its shadow keeps lockstep with the sample instead of inheriting `taken` and `n_sub` silently. *Was:* no such note.
