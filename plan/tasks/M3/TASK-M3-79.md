# TASK-M3-79 — Heggie's landing-order check: the figure-eight convergence order under RQ-823's criterion

- **Milestone:** M3
- **Closes:** REQ-INT-122
- **Gate condition:** must pass before the M3 gate (R-1013, the owner's condition, 14 Nov 2026)
- **Depends on:** TASK-M3-77
- **Waits on:** RQ-823 (the criterion for Heggie's landing-order check)
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, physics, qa
- **Pitfalls:** PIT-4
- **Size:** ~200 lines, the test and its control (taken up from `task/TASK-M3-79`), the criterion's record and the PR's
  run

Split from TASK-M3-77, 14 Nov 2026, applied per R-369 (task scoping, the orchestrator's call; physics review 5904361846
of #530 left it to the orchestrator): TASK-M3-77's convergence-order check, run once under its approved criterion,
read FAIL (round-off), and a second criterion is RQ-823's, a one-way question. Physics: neither REQ-INT-122 nor
REQ-INT-084's Heggie part closes, and the failing test does not land on `main`, until the check passes.

The owner ruled the split, 14 Nov 2026 (R-1013, verbatim): "TASK-M3-77 split: yes. TASK-M3-77 merges closing
REQ-INT-123; the order check moves to TASK-M3-79 closing REQ-INT-122. Condition: TASK-M3-79 must pass before the M3
gate." This task closes REQ-INT-122, in place of TASK-M3-77. REQ-INT-084's Heggie part is checked here, but REQ-INT-084 itself is closed by
TASK-M3-75 once logH's part is in and this task has passed. RQ-823 is the source of the criterion until it is ruled;
then its ruling in `decisions.md` is. The landing the check measures is Part 2b's Heggie step 8.

## Goal
R-1007's check of Heggie's landing is made: Heggie × RK4's figure-eight closure and its convergence order across η,
under the criterion RQ-823's ruling names, written and physics-approved before its one run, with the clock-only landing
as its control (REQ-INT-122; REQ-INT-084's Heggie part).

## References
- `decisions.md` § "R-1007 — RQ-812, Heggie's landing: option (a), one landing rule for all three methods; Heggie's occupant lands under R-998's rule, with no landing tolerance, the state landing on the boundary with the clock, the final step sized or retaken, checked by the figure-eight closure's convergence order *(closes RQ-812; extends R-998 and R-994)*"
- `decisions.md` § "R-998 — RQ-795, Aarseth–Zare's landing: option (b), no landing tolerance, on the owner's condition that the state lands on the boundary together with the clock, the final step sized or retaken to land, never the clock corrected alone while the state stays overshot; checked by the figure-eight closure's convergence order, not only its closure value *(closes RQ-795)*"
- `decisions.md` § "R-1008 — TASK-M3-76's failed convergence check stands as reported, and its second criterion, written before running on a fresh η ladder with a stated round-off floor and approved by physics before it runs, must tell round-off from a landing defect: the order fitted only above the floor, an order still "well short of 4" there a landing defect, the clock-only control kept *(on D-5 of the report of 13 Nov 2026, which stands)*"
- `docs/contracts/principia_integrator_contract.md` § "The equations and the step control (R-160, R-161)"
- `docs/contracts/principia_integrator_contract.md` § "Heggie: the equations, the time transformations, the maps and the step control (transcribed)"

## Deliverables
- `crates/kernel/tests/heggie.rs` — `heggie_landing_order` and its clock-only control (the clamp on, the secant's and
  the bisection's caps 0), under the criterion RQ-823's ruling names. The test as built and run on #530 is on the local
  branch `task/TASK-M3-79` at 58ee2abef, under TASK-M3-77's first criterion; it is taken up and changed only as the
  ruling's criterion says.
- The criterion in the pull request, written before any run and approved by physics before it runs (R-649), and the
  run's output.

## Acceptance tests
- `cargo test -p kernel heggie_landing_order` — Heggie × RK4's figure-eight closure and its convergence order under
  RQ-823's criterion: the round-off floor stated before the run, the order fitted only above it, at least 3 rungs, PASS
  within 0.5 of 4, below 3.0 a landing defect, otherwise unexplained (the bands of the first criterion, kept by every
  option RQ-823 lists); control: the clock-only landing reads a landing defect (REQ-INT-122; REQ-INT-084's Heggie part).
  It runs once; nothing is tuned or re-laddered after it (R-649).

## Notes
- The first criterion's run (job 5918, on #530): measured floor 8.882e-13, `F = 1e-12`, fitted above 1e-11; the landing
  1 rung of 3 above it (η = 0.02 at 4.022e-11), FAIL (round-off); the clock-only control order 2.209, a landing defect.
  RQ-823 quotes it in full, with physics' options (A) `T/8` with the predictive limit set not to bind (recommended),
  (B) `T/32` with η in [0.2, 0.02], (C) accept the run.
