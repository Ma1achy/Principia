# TASK-M3-67 — After M2: the trajectory interface on the real integrator, and the outcome-class arms of the conversions (R-916, R-918)

- **Milestone:** M3
- **Closes:** REQ-CHART-177
- **Depends on:** TASK-M2-82, TASK-M2-64, TASK-M2-65, TASK-M3-17, TASK-M3-19, TASK-M3-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
After M2's gate (R-918): the real integrator plugs into the trajectory interface as a provider, at R-565's fixed sync quantum, and the outcome-class arms M2 couldn't read pass on integrated outcomes: R-894's consolidation and R-900's saved-file conversion keep each case's outcome class.

## References
- `decisions.md` § "R-918 — R-917's follow-ups: TASK-M3-14 (the Benettin FTLE shadow) stops and is re-queued after M2's gate; M2's maths that seems to need the integrator is tested against exactly known trajectories through a trajectory interface the real integrator plugs into after M2, flagging only what can't be tested so; CI work outside the order, TASK-M2-38 and TASK-M2-56 finishing, and the gate checklist first are agreed *(no RQ)*"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-894 — Right after stage 1 and before stage 2, a consolidation step collapses the chart-path leaves to one per operation: bitwise-equal pairs merged with no bit change, rounding-only differences resolved to one canonical formula with a documented bit change and a conversion test, the GUI routes thin input adapters, the same-frame slips fixed; bit-for-bit reproduction is stage 1's safety net, not a permanent rule; the dispatch classes sit in the GUI's `dispatch.rs`, the path in `chart_space` *(the report's D-43, which stands; refines R-863 and R-864)*"
- `decisions.md` § "R-900 — Stage 2's saved-file rewrite is handled as RQ-750: a versioned format, old files converted on load, a conversion test in place of the bitwise check, and any golden hashes regenerated with a record of why *(refines R-864)*"

## Deliverables
- The default integrator as a trajectory-interface provider.
- The outcome-class arms of `path_canonical_conversion` and the saved-file conversion, on integrated runs.

## Acceptance tests
- `cargo test -p engine trajectory_interface_integrator` and `cargo test -p engine -- path_canonical_conversion_outcome saved_file_stage2_outcome` — the interface's tests re-run with the default integrator as provider; every changed or converted case's outcome class equal on integrated runs, the excluded set near the landmarks stated; controls: a provider whose row reads between boundaries fails, a conversion that flips a class fails (REQ-CHART-177).

## Notes
- New with R-916 and R-918 (9 Nov 2026), from F5. Waits for M2's gate (R-917).
- Depends on TASK-M3-40 because the Goal's provider runs at R-565's fixed sync quantum, which TASK-M3-40 builds (REQ-INT-100).
