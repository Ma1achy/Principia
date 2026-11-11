# TASK-M2-80 — Points of interest's maths: the signed point, the nearest chart point, the off-slice distance and the near-match (R-870, R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-172, REQ-GUI-427, REQ-GUI-428
- **Depends on:** TASK-M2-66, TASK-M2-57, TASK-M2-65, TASK-M2-99
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
The maths of R-870's points of interest as engine functions: the signed point, whether a chart or a path frame passes through it within the lock's tolerance, the nearest chart point and the off-slice distance, and the "similar systems" match, with their definitions (R-72) and the near-match tolerance's calibration (R-71). The stars and overlays stay TASK-M8-126's, drawn from these.

## References
- `decisions.md` § "R-870 — Points of interest: any pixel's starting state saved as a named point, shown on every chart and every frame of every path as a filled star where the chart passes through it exactly, otherwise a hollow star at the nearest chart point labelled with its off-slice distance, with optional "similar systems" overlays; display only, never scored, distinct from the lock"
- `docs/design/principia_chart_composition.md` § "10.5 Points of interest"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-854 — Symmetry reductions apply only to how a starting state is described at `t = 0`: trajectories evolve and are shown in full physical space, nothing is folded during a run, and the only operation besides the step is the per-step centre-of-mass projection, energy and `L_z` monitored, never corrected"
- `decisions.md` § "R-899 — The chart-path brief's nine questions: the lock test reads the recorded state at the pin; snap-back undoes the slice only, auto-snap off; jumps under a lock kept and labelled until the sliding anchor exists; the pin's screen position stored, "assumed" at the centre where none can be derived; a pivot record in the view, untilt (a1), amending R-473; points of interest one task in stage 3; their star filled within the lock's tolerance, the off-slice distance shown; marks mid-morph to pixel resolution, the filter off, labelled; the block line built in stage 2 *(amends R-473, and the snap-back of chart_decoder_contract Part 4 and design § 17.8)*"

## Deliverables
- The definitions R-870 leaves unstated, written into chart_composition § 10.5 under physics review (R-72; REQ-GUI-427).
- `crates/engine`: the point stored as a signed state, off the sim key; the pass-through test, nearest point, off-slice distance and near-match.
- The near-match tolerance's proposal, with its evidence, its criterion written first, set by measurement (R-649/650) (REQ-GUI-428).

## Acceptance tests
- `cargo test -p engine points_of_interest_maths` — a point lying in the chart is found at the pixel that decodes to it, within the lock's tolerance, with its off-slice distance; a point off the chart gets the nearest chart point and its distance; a one-property match passes through the point's pixel; across a morph's frames the match is consistent, mid-morph found to pixel resolution; saving a point changes no sim key, payload or statistic; controls: a point one pixel off fails, another property's value fails to pass through it, a point in the sim key fails (REQ-CHART-172).
- Review: chart_composition § 10.5 states each definition: the nearest chart point, the off-slice metric, the near-match measure, marks on a frame with no inverse, the state group holding the points, and the gauge of the property values and the off-slice distance, mirror not quotiented; the physics reviewer approved (REQ-GUI-427).
- Review: the proposal gives the tolerance and its evidence (how many pixels match at it on a stated set of charts and points, against one tighter and one looser value); recorded in decisions.md (REQ-GUI-428).
- *11 Nov 2026 (R-947, R-944):* `cargo test -p engine points_of_interest_maths` — REQ-CHART-172's verify as it now reads: the nearest chart point and the off-slice distance are `place`'s point and residual (TASK-M2-99); mid-morph the point is found to the lock's tolerance under (D), **the condition written in: physics review confirms, else R-899's Q8 (a), "to pixel resolution", is what the test asserts** (REQ-CHART-172). The definitions' review reads REQ-GUI-427 as it now reads (REQ-GUI-427).

## Notes
- Split from TASK-M8-126 by R-916 (9 Nov 2026); TASK-M8-126 keeps REQ-GUI-426, the drawing.
- TASK-M2-57 is in Depends on because a point's signed state on the latent chart needs the full-circle `β` (R-865).
- **Build order (R-916):** stage 3.
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; R-947):* the nearest chart point and the off-slice distance are `place`'s (TASK-M2-99, a new dependency); REQ-GUI-427's "how marks are found on a path frame with no inverse" is defined by chart_composition § 10.6. Mid-morph marks meet the lock's tolerance under (D), applied per R-369 and conditional on physics review: not settled until the physics reviewer confirms; if they disagree it goes to the owner as a queue entry. Where a dated line under Acceptance tests differs from an earlier line for the same requirement, the dated line binds and the earlier one is kept as its *Was*. *Was (Depends on):* "TASK-M2-66, TASK-M2-57, TASK-M2-65".
