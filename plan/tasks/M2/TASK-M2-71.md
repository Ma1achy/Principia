# TASK-M2-71 — Warps as options, polar and log-polar pair maps, and a statistic's measure (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-104, REQ-CHART-131, REQ-CHART-181
- **Depends on:** TASK-M2-66, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
Power and log as options per axis, polar and log-polar on any pair, each declared extension under R-407 clause 3's review, and a statistic's measure, on the CPU.

## References
- `docs/design/principia_chart_composition.md` § "4. The axis layer: warps"
- `docs/design/principia_chart_composition.md` § "5.1 Pair maps"
- `docs/design/principia_chart_composition.md` § "6.4 Measure: a statistic says what it counts"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `docs/design/principia_chart_composition.md` § "6.5 Chaos measures live in phase space"
- `docs/contracts/principia_scheduler_contract.md` § "Part 2 — Refinement density is not probability density (enforced here, because this is where it would break)"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"

## Deliverables
- `crates/kernel/src/chart/paired.rs`: `Paired<C, M>`.
- The warp word on `Mixed`'s uniform.
- The statistic's weight and label: a named measure, or `per pixel of this chart`.
- The calibration proposal for the warps' and pair maps' round-trip tolerance, its criterion written first, set by measurement (R-649/650) (REQ-CHART-181).

## Acceptance tests
- `cargo test -p kernel warps_and_pair_maps` — forward(inverse(x)) = x for each warp and each pair map; polar's period and its pole-crossing partner; the linear arm bit-for-bit today's; controls: a warp not offered on an energy-type axis is refused, a named mutation per inverse fails (REQ-CHART-104).
- `cargo test -p engine statistic_measure` — a statistic over a warped chart is weighted into a named measure or labelled; a fixture of known fraction is recovered through a power warp; a fraction in a paper's own measure equals the flat grid in the paper's coordinates; controls: no weight and no label fails, the weight inverted fails, a count of quadtree leaves fed in fails (REQ-CHART-131).
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p kernel warps_and_pair_maps`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-181).

## Notes
- Was TASK-M8-100's maths (R-916, 9 Nov 2026). TASK-M8-100 is retired: REQ-CHART-132 went to TASK-M3-14 (R-918), and V1's decode time, the old acceptance's perf clause, to TASK-M4-33's REQ-PERF-116 (applied per R-369).
- Reviewed under RQ-720's reading of R-594 item 5.
- **Build order (R-916):** stage 3.
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.5; R-936, R-944):* REQ-CHART-131 (a statistic's measure) gains: a mid-morph frame admits no area statistic, its `measure` `none`, the two end charts keeping their own. The blend chart is TASK-M2-90's; this task's `statistic_measure` reads the requirement as it now is.
