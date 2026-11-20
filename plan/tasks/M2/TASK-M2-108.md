# TASK-M2-108 — The first slice's integrated arms on the default occupant, Heggie × RK4, once its joint march is built (R-926)

- **Milestone:** M2
- **Closes:** REQ-DEC-057
- **Depends on:** TASK-M2-86
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** a re-run of TASK-M2-86's harness on another occupant, with its report; little new code

## Goal
TASK-M2-86 built the first slice's harness and measured it, but the integrated arms ran on KDK. The task asks for "the default occupant", which is Heggie × RK4 at every tier (R-560, R-606), and no Heggie × RK4 occupant was built when TASK-M2-86 ran (RQ-818). This task closes REQ-DEC-057 by running the same harness, generic over the occupant (`engine::first_slice::outcome::<A>`), on the default occupant once TASK-M3-75 builds Heggie's joint march and seam. It keeps every predicate and measure TASK-M2-86 reports, and reads the integrated arms against their control on the default occupant.

Split from TASK-M2-86 (applied per R-369, 15 Nov 2026, task scoping: the task re-scoped to match the built code; no requirement dropped or deferred). The orchestrator decided it in the fix round on PR #541 (the physics review's F5 and qa's Q2). REQ-DEC-057 moved here from TASK-M2-86. RQ-818 stays open for the owner. If it is ruled otherwise, this task follows the ruling.

## References
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"

## Deliverables
- TASK-M2-86's harness (`crates/engine/tests/first_slice.rs`, `engine::first_slice`) run with the default occupant, Heggie × RK4, in place of KDK: predicate (4), the integrated arm and its control arm, on REQ-DEC-057's grids. The KDK runs stay beside it, labelled as the stand-in they were.
- The report: every measure of TASK-M2-86's, with the integrated arms on the default occupant. They run at a horizon written before the run, chosen so the narrowed marches can end in more than one outcome class (TASK-M2-86 used the cap dispatch admits, `T_HORIZON_MAX` = 200, R-610). Each grid's arm is read by the criterion TASK-M2-86 used, also written before the run: the control's flip share is above 0 on at least one grid of each chart, and on a grid where it is 0, the arm is reported as uninformative at that horizon, not as "0 flips".

## Acceptance tests
- `cargo test -p engine first_slice` — REQ-DEC-057's acceptance as TASK-M2-86 states it, with the integrated arms and predicate (4) on the default occupant: predicates (1) to (4), PASS required; the comparison, the branch selects, the integrated outcome class and `t_end_step` after `T` read against the control arm (R-84), and the neighbouring-pair shares, measured and reported, not asserted; the outcome PASS, EXPLAINED DIVERGENCE or UNEXPLAINED by the criterion TASK-M2-86 states. The Mac only; no Windows runner job (REQ-DEC-057).

## Notes
- Entered 15 Nov 2026 in the fix round of PR #541 (applied per R-369: task scoping).
- **Waits for TASK-M3-75**, which builds Heggie's joint march and seam, **and for TASK-M3-12**, which labels escape (until it merges nothing writes `escape`, so a march that does not collide ends bounded whatever the occupant, and the arm can't read an escape flip): this task isn't dispatched before both merge. The edges aren't in `plan/tasks.yaml`, because `check_plan` admits no dependency on a later milestone's task, and REQ-DEC-057 is an M2 requirement, so an M3 task can't close it. The orchestrator holds the dispatch. RQ-818 (`REVIEW_QUEUE.md`, open) is the owner's question about the default occupant.
- What TASK-M2-86 measured on KDK, the stand-in, at `T = 200`, the key's `r_coll = 1e-3` and no escape labelled (TASK-M3-12 not merged): the control arm registers flips on one grid of the latent chart (2^10 about the origin: class flips 2 of 256, against today's decode's 17 of 256) and one of the `(L_z, E)` chart (home: 1 of 255, against 2 of 255), the only non-bounded outcomes collisions; on every other grid, and on every grid of the shape sphere, no march collided and every one ran to the horizon bounded, so the arm there is uninformative. The cause is the escape gap: nothing writes `escape` until TASK-M3-12, so a march that does not collide ends bounded whatever the occupant. (An earlier reading, every march bounded at `T = 4`, 50, 100 and 200, came from marches at `r_coll = 0`, where no collision fires; the physics review on PR #541 found it, and it is withdrawn.) The default occupant's report, after TASK-M3-12, says which grids read.
- *17 Nov 2026 (R-1027, RQ-818 (b)):* the owner ruled option (b): KDK's runs stand as the labelled stand-in, and this
  task re-runs the same harness on the default occupant, Heggie × RK4, once TASK-M3-75 and TASK-M3-12 merge. RQ-818 is
  closed; the dispatch still waits for those two tasks, as above.
