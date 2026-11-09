# TASK-M2-82 — The trajectory interface and its exact fixtures: the Lagrange equilateral homographic solution and the figure-eight (R-918, R-919)

- **Milestone:** M2
- **Closes:** REQ-CHART-173
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
R-918: M2's maths that seems to need the integrator is tested against exactly known trajectories, through a trajectory interface the real integrator plugs into after M2 (TASK-M3-67). This task builds the interface (chart_composition § 18.7) and its two exact three-body fixtures (R-919): the Lagrange equilateral homographic solution in closed form, and the figure-eight, exact by declaration at its period; two-body Kepler only where the interface is generic. The fixtures' constants move to a crate below `engine` and `validation`.

## References
- `decisions.md` § "R-918 — R-917's follow-ups: TASK-M3-14 (the Benettin FTLE shadow) stops and is re-queued after M2's gate; M2's maths that seems to need the integrator is tested against exactly known trajectories through a trajectory interface the real integrator plugs into after M2, flagging only what can't be tested so; CI work outside the order, TASK-M2-38 and TASK-M2-56 finishing, and the gate checklist first are agreed *(no RQ)*"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `docs/design/principia_chart_composition.md` § "18.7 The trajectory interface (R-918, R-919)"
- `docs/design/principia_dd_validation_orbits.md` § "1.2 Figure-eight — the canonical closure test"
- `docs/design/principia_dd_validation_orbits.md` § "1.3 Lagrange and Euler central configurations — the only analytic ones"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"

## Deliverables
- The trajectory interface in `crates/engine` (R-864 item 5): a signed start state and a horizon give the boundary grid as integer indices per occupant at the provider's declared quantum, the state at a boundary, and the terminal (index and kind), the same bits for the same inputs.
- The two fixtures as providers; the figure-eight's constants moved from `crates/validation/src/drift.rs` into a lower crate, `validation` reading them there.

## Acceptance tests
- `cargo test -p engine trajectory_interface` — a row reads the provider's state bitwise; the figure-eight's row at kT₈ equals its row at 0 bitwise; the Lagrange states within the bound derived from the closed form's evaluation, else a calibration (R-71), its criterion written first, set by measurement (R-649/650); the terminal at its index and kind on the homothetic collapse; two reads in either order give one bit pattern; validation's figure_eight() reading the lower crate's constants; controls: a row reading one boundary off fails, a terminal not held fails, a read depending on an earlier read fails; the row-one-boundary-off control and every check that tells boundaries apart run on the Lagrange fixture, at a quantum no multiple of its period, since the figure-eight's boundary rows, at kT₈, all equal its start (REQ-CHART-173).

## Notes
- New with R-918 and R-919 (9 Nov 2026). It may start now (R-917's exceptions: what M2's own tests need).
- A bound the closed form's evaluation can't derive is a calibration (R-71), its criterion written first, set by measurement (R-649/650).
- *9 Nov 2026 (physics review 5834013503 of PR #465):* the figure-eight's boundary rows, at kT₈, all equal its start, so the row-one-boundary-off control, and every check that tells boundaries apart, runs on the Lagrange fixture, at a quantum no multiple of its period. *Was (Acceptance, the end):* "controls: a row reading one boundary off fails, a terminal not held fails, a read depending on an earlier read fails (REQ-CHART-173)."
