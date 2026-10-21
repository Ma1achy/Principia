# TASK-M3-39 — Every stepper × regularisation pair: its exact composition, the measured pair table and its warnings (R-561)

- **Milestone:** M3
- **Closes:** REQ-INT-097, REQ-INT-098, REQ-INT-099
- **Depends on:** TASK-M3-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
Every stepper × regularisation pair runs, regularisation off included, and none is forbidden (R-561). Each pair is defined exactly: on Aarseth–Zare's and Heggie's non-separable Hamiltonians KDK is the explicit kick–drift–kick of integrator contract Part 2b and a Yoshida step its composition with dd_integrator §3.2's coefficients; logH's pairs are written with its transcribed equations. The kernel carries Part 2b's pair table, each pair measured good, measured bad or not measured, and a measured-bad pair carries its warning text, the data the Run window (TASK-M8-24), the Problems panel and the export sidecar (TASK-M8-32) show. This task writes the criterion for "measured not to work" (a definition, R-72), measures the not-measured regularised pairs by the design's method, and records each verdict in the table.

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
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"

## Deliverables
- `crates/kernel/src/regularisation/pairs.rs` — the explicit compositions for the non-separable regularisations (KDK, Yoshida-4, Yoshida-6 on `(u, p, t)` in fictitious time), each pair's composed profile, and the pair table with each pair's status, evidence line and warning text.
- `cargo xtask gate pair-table` — the design's measurement on every regularised pair: order over a halving ladder and the regularised energy over 100 orbits of the figure-eight at a fixed fictitious step, no step control, both of Heggie's time transformations.
- Doc change: integrator contract Part 2b, the pair table — the criterion for "measured not to work" with its bounds (REQ-INT-099), and each newly measured pair's status and evidence; a pair found bad gains its warning text.

## Acceptance tests
- `cargo test -p kernel pair_definitions` — REQ-INT-097's verify: for each regularised pair, one step from a fixed fixture state equals bitwise the written composition evaluated term by term; Heggie × KDK's fitted order on the figure-eight is about 1 while Heggie × RK4's rounds to 4; the composed profile of Heggie × KDK reads `symplectic` false; controls: a KDK whose drift reads the derivative at `(u, p)` instead of `(u, p½)` differs and fails, and a profile copied from KDK's Part 2 row fails (REQ-INT-097).
- `cargo xtask gate pair-table` — REQ-INT-098's verify: each regularised pair's order and energy printed with its verdict; the kernel's table equals the measured verdicts and integrator contract Part 2b's table; Heggie × RK4 good (order about 4, energy bounded) and Heggie × KDK bad (order about 1, diverging), the design's values; a verdict that changes a status is reported for the table and the review queue; controls: a table marking Heggie × KDK good fails, and a measured-bad pair whose warning is empty fails (REQ-INT-098).
- Review (physics): Part 2b states the criterion with its bounds, and applied to the design's measurements it reads Heggie × RK4 good and Heggie × KDK bad (REQ-INT-099).

## Notes
- R-561 (21 Oct 2026): "don't forbid combos of integrators & regularisers… maybe just have a warning somewhere that they might not work". The design's measurements are prin-rs's `heggie_stepper_probe` (PR #34), kept as a record in `docs/incoming/2026-10-21_gap_sweep_integrator/` (not authority, R-159); the probe is not built here.
- Applied per R-369: the composed occupant's profile is the pair's; the not-measured pairs carry no warning until measured; Euler's pairs run in the debug selection only (REQ-INT-015).
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-099.
