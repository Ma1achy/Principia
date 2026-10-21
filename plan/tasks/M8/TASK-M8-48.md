# TASK-M8-48 — On the real engine: tilt and slice on every chart, the energy axes' mapping control, the navigation stop in the edit path, and Save view as chart on curved views (R-469, R-471, R-473, R-475) ⚑

- **Milestone:** M8
- **Closes:** REQ-GUI-241, REQ-GUI-242, REQ-GUI-243, REQ-GUI-244
- **Depends on:** TASK-M8-06, TASK-M8-03, TASK-M6-48, TASK-M6-49, TASK-M2-36, TASK-M2-37, TASK-M2-38, TASK-M2-40, TASK-M8-18
- **Needs (earlier milestones):** REQ-CHART-060, REQ-CHART-062, REQ-CHART-068, REQ-CHART-073, REQ-CHART-077
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
The real engine's Manifold view tilts and slices every chart (targets the chart's hidden coordinates, the rotation on its own parameters, mass readouts), offers each energy-type axis's mapping, passes every navigation `SetField` through the kernel's stop before recording it (the clamped value in the history, the slider stopped, the readout's "stopped at the edge" line), and saves a curved view as a chart that reopens bitwise.

## References
- `decisions.md` § "R-469 — A chart never leaves the manifold: every pixel it draws is a valid three-body system or explicitly "not a system", and pan, slice, zoom, tilt and lock are accepted only while the chart's centre passes `validate`, an edit that would cross stopping at the last valid value, by one generic rule in the edit path; undo and the contract record the clamped value, the slider stops and the readout names the check"
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `decisions.md` § "R-513 — A new tilt of a saved curved chart into its own saved target composes the rotations, `(a, b, d) = R·R′·(u, v, 0)`, the saved `R` acting on `(u′, v′, d′)`; the saved target is offered again *(closes RQ-403; amends R-473 and R-475)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_chart_decoder_contract.md` § "The operation table"

## Deliverables
- The edit path's stop in the real engine's contract (`crates/engine/src/contract/`), the Manifold view's sections on every chart, the mapping control, the readout line, Save view as chart for curved views.
- Tests `real_tilt_every_chart`, `energy_axis_mapping_control`, `nav_stop_edit_path`, `saved_curved_chart`.

## Acceptance tests
- `cargo test -p gui real_tilt_every_chart` and `cargo xtask screenshot 01_main` — REQ-GUI-241's verify.
- `cargo test -p gui energy_axis_mapping_control` — REQ-GUI-242's verify.
- `cargo test -p gui nav_stop_edit_path` — REQ-GUI-243's verify.
- `cargo test -p gui saved_curved_chart` — REQ-GUI-244's verify (C8).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **RQ-378** (a flattened oblique-latent save of a curved chart) waits on nothing; meanwhile not offered.
- **R-510 (20 Oct 2026):** RQ-378 is parked until RQ-341; nothing waits. **Review round 1 of PR #255 (applied per R-369):** on a curved chart the Slice centre sliders its axes set are read-only and dimmed (gui review G2; R-473 J11, REQ-GUI-241); the mapping rows follow Save view as chart in the keyboard order and offer `KE` (G7, G9; R-471 H10, REQ-GUI-242); a saved curved chart's target dropdowns do not offer its saved target (RQ-403).
- **R-513 (20 Oct 2026) rules RQ-403:** a saved curved chart's target dropdowns offer its saved target too, a new tilt into it composing the rotations in the kernel (REQ-CHART-077, TASK-M2-40, which this task depends on already); the round-1 note's "do not offer its saved target (RQ-403)" no longer applies.
- **R-537 EX3 (20 Oct 2026):** the stop is judged on the centre after the axes' types fold it, so a pan across a seam or a pole never stops; with the controls shown the view may sit past the chart's square (TASK-M8-55, REQ-GUI-267). The stop itself is unchanged.
- R-484 J9 (20 Oct 2026): filling the chart params clears T2's `Chart {}`; T2 is TASK-M8-53's.
