# TASK-M3-76 — Aarseth–Zare's landing with no tolerance, the state landing with the clock, and its |R₃| floor at TINY (R-998, R-999)

- **Milestone:** M3
- **Closes:** REQ-INT-119, REQ-INT-120
- **Depends on:** TASK-M3-68
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4
- **Size:** ~350 lines, the gate's convergence arm included

## Goal
Two of the owner's answers of 13 Nov 2026 change the built Aarseth–Zare occupant (TASK-M3-68, #502), which takes both
values as settings with no value (`AzParams::land_eps_rel`, `AzParams::r3_floor`):

- **The landing (R-998, RQ-795):** "(b), on one condition: the state must land on the boundary together with the clock
  (the final step is sized or retaken to land), never the clock corrected alone while the state stays overshot. Check
  it by the figure-eight closure's convergence order, not only its closure value." No `LAND_EPS_REL` reaches a branch:
  the interval lands when the last step ends the clock at or past the boundary, and the state lands there with it.
- **The floor (R-999, RQ-796):** "(a)": `|R₃|` in the equations of motion is floored at `TINY` at each working precision,
  as R-953 does for the reference's other `1e-300` floors, its value following REQ-INT-105's proposal.

## References
- `decisions.md` § "R-998 — RQ-795, Aarseth–Zare's landing: option (b), no landing tolerance, on the owner's condition that the state lands on the boundary together with the clock, the final step sized or retaken to land, never the clock corrected alone while the state stays overshot; checked by the figure-eight closure's convergence order, not only its closure value *(closes RQ-795)*"
- `decisions.md` § "R-1008 — TASK-M3-76's failed convergence check stands as reported, and its second criterion, written before running on a fresh η ladder with a stated round-off floor and approved by physics before it runs, must tell round-off from a landing defect: the order fitted only above the floor, an order still "well short of 4" there a landing defect, the clock-only control kept *(on D-5 of the report of 13 Nov 2026, which stands)*"
- `decisions.md` § "R-999 — RQ-796, Aarseth–Zare's floor on `|R₃|`: option (a), the floor is `TINY` at each working precision, as R-953 does for the reference's other `1e-300` floors, its value following REQ-INT-105's proposal *(closes RQ-796)*"
- `decisions.md` § "R-927 — RQ-756, Aarseth–Zare's open points: REQ-INT-049's figure-eight threshold is re-measured under the step rule in force, a calibration; on each point of (5) where the reference code and the corpus disagree, the reference code's choice, the Rust driver being that code, the threshold re-measured under the same code; the owner's (4)(a), REQ-INT-054's bound per sync interval as the step budget over one interval, contradicts R-604's budget, which is cumulative since `t = 0`: flagged, not applied, (4) staying open with three points of (5) whose Rust files are not in the imported set *(on RQ-756, which stays open)*"
- `decisions.md` § "R-953 — RQ-756, Aarseth–Zare's last points: REQ-INT-054's bound per sync interval is R-604's cumulative step budget read at each boundary, the owner's (4)(a-2), with no cap per interval; the three Rust files are imported at `b1e6006`; the predictive limit reads the relative momenta, as the code computes them, its comment's "velocities" being wrong; the inverse map's guards are the floor `TINY`, written down as Part 2c rule 1's one exception; the pairs are taken in the order (0,1), (0,2), (1,2) *(closes RQ-756; amends R-160; refines R-927)*"
- `docs/contracts/principia_integrator_contract.md` § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"

## Deliverables
- Part 2b's text, physics-reviewed: how the secant stops with no tolerance so that the state and the clock land on the
  boundary together, the final step sized or retaken to land, never the clock set alone while the state is overshot (a
  definition, R-72, under R-998); each replaced line of steps 6 and 7 kept as *Was*. Its limits (R-998's dated line):
  no constant other than an integer iteration cap (rule 3) reaches a branch; the wrapper's clock is never set to the
  boundary while the stored state's own clock differs from it; if no stop meets both, the residual goes to
  `REVIEW_QUEUE.md` (a calibration under R-71, or an RQ) and is never chosen in the pull request.
- `crates/kernel/src/regularisation/az.rs`: the landing as written; `AzParams::land_eps_rel` removed, so no landing
  tolerance reaches a branch at either precision; `AzParams::r3_floor` replaced by `TINY` at the working precision.
- The gates and tests that state the reference's values (`1e-5`, `1e-14`; `1e-300` and no floor at f32) as their run
  condition, marked with RQ-795 and RQ-796, moved to the rule in force, each marker removed.
- `cargo xtask gate az-final-step`'s convergence arm (below).

## Acceptance tests
- `cargo xtask gate az-final-step` — the figure-eight closure on the gate's own sync interval (T/32) under the landing
  above: its closure value at η = 0.001 as REQ-INT-049 has it, and its convergence order across η in [0.02, 0.001],
  fitted on the ladder; criterion, written in the PR before any run and physics-reviewed (R-649): the fitted order
  against the order expected for the stepper (RK4) within a stated margin, with its reasoning; control: a landing that
  sets the clock to the boundary while the state stays where the overshooting step left it shows its order fall and
  fails (REQ-INT-119).
  *13 Nov 2026 (R-1008, the report's D-5, which stands, with the owner's rider):* the first criterion's FAIL (order 2.46
  against "within 0.5 of 4") stands as reported. The second criterion is written before running, on a fresh η ladder
  not yet run, with its round-off floor stated beforehand, and physics approves it before it runs. The owner's rider
  binds it: (1) it must tell round-off from a landing defect; (2) the order is fitted only over step sizes above the
  stated round-off floor; (3) if the order there is still "well short of 4" (the owner's words; the criterion states its
  own threshold before running), it is treated as a landing defect, a residual of order h² from sizing the final step,
  not as round-off; (4) the clock-only control stays in.
- `cargo test -p kernel az_landing_state_with_clock` — on a fixture interval whose last step overshoots, the stored
  state is the state at the boundary, not the overshot one, and the clock reads the boundary; no landing tolerance is
  read; control: the clock-only correction fails (REQ-INT-119).
- `cargo test -p kernel az_r3_floor_tiny` — at f32 the `|R₃|` term reads `max(|R₃|, TINY)` with `TINY` the f32 value, not
  0; at f64 the f64 value; control: a floor of 0 at f32 fails on a state with `|R₃|` below `TINY` (REQ-INT-120).

## Notes
- Entered 13 Nov 2026 by R-998 and R-999 (applied per R-369: task scoping; no open task builds Aarseth–Zare's step
  control, and the two answers change the same occupant's settings).
- `crates/kernel/tests/qa_TASK-M3-68.rs` sets `land_eps_rel` and `r3_floor` in its parameters (its lines 77–78 and
  648–649 at `main` 256384ad); removing the setting forces a change to that qa file, which R-998 and R-999 force (an
  exception to R-290's rule that only qa edits qa's files, decided by the orchestrator and the reviewers per R-369 and
  listed on the PR).
- logH lands under the same rule (R-994), in the task that builds logH; Heggie's landing is that task's and TASK-M3-08's,
  and R-998 names Aarseth–Zare only.
- *13 Nov 2026 (physics review 5886173032 and code review 5886176062 of #515):* Heggie's landing is RQ-812, open (its reference lands with the per-precision
  `LAND_EPS_REL` in branches, which rule 1 forbids); it is not this task's. The secant stop's limits are stated in the
  Deliverables, from R-998's dated line.
- *13 Nov 2026 (R-1007, RQ-812 (a)):* ruled by R-1007: one landing rule for all three methods, so Heggie lands under
  R-998's rule too, in TASK-M3-08's step-control part (REQ-INT-084); it is still not this task's.
- `TINY`'s values are REQ-INT-105's calibration, confirmed at the M3 gate.
