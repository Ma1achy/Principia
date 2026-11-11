# TASK-M2-73 — Numerical inverses and the general numerical solve, on the CPU (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-107, REQ-CHART-145, REQ-CHART-183
- **Depends on:** TASK-M2-66, TASK-M2-67, TASK-M2-68, TASK-M2-71, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
Numerical inverses on the CPU: a pair with a numerical inverse runs in f64, its branch rule a pure function of the pixel, its branch recorded and its seam reported; and the general numerical solve for a typed expression with no registered inverse (R-849). The seam's jitter and refinement, the payload's slots and the browser build stay TASK-M8-102's.

## References
- `docs/design/principia_chart_composition.md` § "7.2 CPU batch"
- `docs/design/principia_chart_composition.md` § "7.3 A numerical inverse: branch and seam"
- `decisions.md` § "R-849 — A typed combination with no registered inverse gets a general numerical solve on the CPU, recording its branch rule, marking its seams and labelled CPU-only; and under the lock a chart path never jumps: the anchor slides continuously to its nearest point in a constrained chart *(closes RQ-733; on RQ-721, closed by R-850)*"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"

## Deliverables
- `crates/engine/src/chart_batch.rs`: the solver, its rule and seam; the branch in the IC record.
- A general numerical solve: its branch rule recorded, seams marked, labelled CPU-only, convergence reported per pixel, failures tagged and never dropped (REQ-CHART-145).
- The calibration proposal for a numerical inverse's round-trip tolerance, its criterion written first, set by measurement (R-649/650) (REQ-CHART-183).

## Acceptance tests
- `cargo test -p engine numerical_branch_seam` — the (area, perimeter) pair reports its rule, per-pixel branch and seam; the same pixels in two orders give one bit pattern; forward(inverse(x)) = x on every written sample; a fixture of known solvable samples solves, with no failures; controls: a rule reading a neighbour fails, the pair with no rule is refused by the registry, the solve's target mutated to F(z) + x writes a wrong root and fails (REQ-CHART-107).
- `cargo test -p engine general_numerical_solve` — a typed expression with two solutions takes the recorded branch at every pixel and marks the seam where the rule flips; each pixel carries its convergence; a non-converging pixel is tagged and present in the payload; the view is labelled CPU-only; controls: a failure dropped fails; a seam left unmarked fails (REQ-CHART-145).
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p engine numerical_branch_seam`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-183).

## Notes
- Split from TASK-M8-102 by R-916 (9 Nov 2026); TASK-M8-102 keeps REQ-CHART-129, REQ-CHART-130 and REQ-CHART-155.
- **Build order (R-916):** stage 3.
- *11 Nov 2026 (the re-plan's § A.1, CHANGE; the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4):* split: its solver core is TASK-M2-94's, built before TASK-M2-65 for the lock's re-solve; this task keeps the rest. "CPU batch" and "CPU-only" lose their meaning: every chart is CPU-evaluated, and the fragment reaches it by table (R-925).
