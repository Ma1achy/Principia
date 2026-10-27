# TASK-M3-39 — Every stepper × regularisation pair: its exact composition, the measured pair table and its warnings (R-561)

- **Milestone:** M3
- **Closes:** REQ-INT-097, REQ-INT-098, REQ-INT-099, REQ-INT-017
- **Depends on:** TASK-M3-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
Every stepper × regularisation pair runs, regularisation off included, and none is forbidden (R-561). Each pair is defined exactly: on Aarseth–Zare's and Heggie's non-separable Hamiltonians KDK is the explicit kick–drift–kick of integrator contract Part 2b and a Yoshida step its composition with dd_integrator §3.2's coefficients, in §3.2's drift order with adjacent half-kicks not merged; logH's pairs are written with its transcribed equations. The kernel carries Part 2b's pair table, each pair measured good, measured bad or not measured, and a measured-bad pair carries its warning text, the data the Run window (TASK-M8-24), the Problems panel and the export sidecar (TASK-M8-32) show. This task writes the criterion for "measured not to work" (a definition, R-72), measures the not-measured regularised pairs by the design's method, and records each verdict in the table.

## References
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/contracts/principia_integrator_contract.md` § "The table gains two rows"
- `docs/design/principia_dd_integrator.md` § "3.2 Occupants"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
- `decisions.md` § "R-560 — The default occupant is Heggie × RK4, the combination every Heggie result was measured with; REQ-GUI-102's default and the Run window change with it"
- `decisions.md` § "R-500 — The capability profile's `reversible` is two flags: `step_reversible`, true for KDK, Yoshida-4, Yoshida-6 and the TTL leapfrog, false for Euler, RK4 and Aarseth–Zare + RK4, the regularised occupants measured; and `run_reversible`, logH-TTL only; both diagnostic routes gate on `step_reversible`, and a negative weight does not break a symmetric step *(closes RQ-369; refines R-162; amends R-57 as to integrator contract Part 6's sentence)*"
- `decisions.md` § "R-567 — The generalised (Störmer–Verlet) leapfrog's growth under Heggie is explained: it does not keep Heggie's Eq. (9), `Σ qᵢ = 0`; Heggie × the generalised leapfrog is measured bad, with its warning, and stays allowed like every other pair *(amends R-561)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `decisions.md` § "R-609 — Neither implicit stepper is selectable: Heggie × the generalised leapfrog is recorded as measured bad for the record, and "stays allowed" means no pair of the built steppers is blocked *(closes RQ-532; reads R-567)*"

## Deliverables
- `crates/kernel/src/regularisation/pairs.rs` — the explicit compositions for the non-separable regularisations (KDK, Yoshida-4, Yoshida-6 on `(u, p, t)` in fictitious time), each pair's composed profile, and the pair table with each pair's status, evidence line and warning text.
- `cargo xtask gate pair-table` — the design's measurement on every regularised pair: order over a halving ladder and the regularised energy over 100 orbits of the figure-eight at a fixed fictitious step, no step control, each regularisation's own time transformation(s) (Heggie's Eq. 20 and Eq. 22, Aarseth–Zare's, logH's logH and TTL modes). *Was:* "both of Heggie's time transformations" (physics review 5605806092 of PR #277, F7).
- Doc change: integrator contract Part 2b, the pair table — the criterion for "measured not to work" with its bounds, deciding one cell, a pair under one time transformation (REQ-INT-099), and each newly measured cell's status and evidence; a cell found bad gains its warning text.
- Doc change: integrator contract Part 2b — logH's pair forms, each stepper under each of logH's time modes, written from logH's equations as TASK-M3-08 transcribes them (REQ-INT-084 covers logH's own equations, not its pairs); the physics reviewer approves before merge (REQ-INT-097; qa review 5607548278 of PR #277, Q1, applied per R-369).

## Acceptance tests
- `cargo test -p kernel pair_definitions` — REQ-INT-097's verify: for each regularised pair under each time transformation, logH's pairs included, one step from a fixed fixture state equals bitwise the written composition evaluated term by term; Heggie × KDK's fitted order on the figure-eight lies in Part 2b note 2's band, 0.8–1.2, under Eq. 20 and Eq. 22, while Heggie × RK4's rounds to 4; the composed profile of Heggie × KDK reads `symplectic` false; controls: a KDK whose drift reads the derivative at `(u, p)` instead of `(u, p½)` differs and fails, a profile copied from KDK's Part 2 row fails, and a Yoshida step with its adjacent half-kicks merged differs under Heggie and fails (REQ-INT-097).
- `cargo xtask gate pair-table` — REQ-INT-098's verify: each regularised pair's order and energy printed under each time transformation with that cell's verdict, none combined across transformations; the kernel's table equals the measured verdicts and integrator contract Part 2b's table, one cell per pair and time transformation; Heggie × RK4 good (order about 4, energy bounded) and Heggie × KDK bad (order in note 2's band 0.8–1.2, diverging) under each of Eq. 20 and Eq. 22, the design's values; Aarseth–Zare × RK4, the port's good, re-measured by the criterion; the table's note 4, Heggie × the generalised leapfrog measured bad (R-567), kept as the design's measurement, that stepper not being built (R-609; *was:* "while that stepper is not built (RQ-532)"); a verdict that changes a status is reported for the table and the review queue; controls: a table marking Heggie × KDK good fails, a measured-bad pair whose warning is empty fails, a table without note 4's warning fails, and a table with one cell for a regularisation that has two time transformations fails (REQ-INT-098).
- `cargo test -p kernel regularisation_compose` — REQ-INT-017's verify, re-run here over every pair, Heggie's and logH's included: each runs, `none` always available; the measured-bad cells (Heggie × KDK under Eq. 20 and Eq. 22) carry their warning and the others none; note 4's warning is in the table; on a synthetic table whose Heggie Eq. 20 and Eq. 22 cells differ for one pair, the warning shown follows the selected time transformation (Eq. 20, then Eq. 22); controls: a selection that blocks a measured-bad pair fails, a measured-bad pair with no warning fails, and a lookup keyed on the pair alone, not the time transformation, fails (REQ-INT-017; qa re-check 5607645188 of PR #277, Q10, applied per R-369; qa review 5607548278 of PR #277, Q8, applied per R-369).
- Review (physics): Part 2b states the criterion with its bounds, and applied to the design's measurements it reads Heggie × RK4 good and Heggie × KDK bad (REQ-INT-099).

## Notes
- R-561 (21 Oct 2026): "don't forbid combos of integrators & regularisers… maybe just have a warning somewhere that they might not work". The design's measurements are prin-rs's `heggie_stepper_probe` (PR #34), kept as a record in `docs/incoming/2026-10-21_gap_sweep_integrator/` (not authority, R-159); the probe is not built here.
- Applied per R-369: the composed occupant's profile is the pair's; the not-measured pairs carry no warning until measured; Euler's pairs run in the debug selection only (REQ-INT-015).
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-099.
- R-567 (21 Oct 2026, the follow-up): Heggie × the generalised (Störmer–Verlet) leapfrog is measured bad, "does not keep Heggie's Eq. (9); the energy breaks once its residual reaches ~2e-2"; neither implicit stepper is selectable or built, and "stays allowed" means no pair of the built steppers is blocked (R-609, RQ-532 (a); *was:* "whether it is a selectable stepper is RQ-532, (a) applied meanwhile, so it is not built here."). Physics review 5605806092 of PR #277: F5 (the Yoshida pairs' order, kicks not merged) and F7 (each regularisation's time transformations; Aarseth–Zare × RK4's status the port's).
