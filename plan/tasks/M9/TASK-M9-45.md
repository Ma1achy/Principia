# TASK-M9-45 — The side-ratio remap onto the shape sphere at rest, its preset on the equilateral point, and the two side ratios in the Inspector (R-798)

- **Milestone:** M9
- **Closes:** REQ-CHART-093, REQ-GUI-417
- **Depends on:** TASK-M3-34, TASK-M8-16, TASK-M8-48
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
Put PA-1's side ratios on the shape sphere at rest, and make the region around the equilateral (Lagrange) point one click away, as the owner asked with RQ-594's answer. A tested remap takes side ratios to shape coordinates, for TASK-M9-24 to run PA-1 on. A shape-chart preset centred on that point joins the chart presets, and the Inspector readout shows the two side ratios. No new chart type.

## References
- `decisions.md` § "R-798 — PA-1 runs on the shape sphere at rest through a tested remap, with no new chart type; a shape-chart preset centred on the equilateral point shows the two side ratios in the Inspector *(closes RQ-594)*"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"

## Deliverables
- The tested remap of side ratios to shape coordinates, in `crates/validation/src/gates/side_ratio.rs`'s reach, one source for TASK-M9-24 and the readout (REQ-CHART-093).
- The preset, among the chart presets, with its base point, centre and window (R-466's definition of a preset).
- The Inspector readout's two side ratios on that preset, read from the remap (REQ-CHART-093).
- The written definitions below, each in `principia_render_gui_spec.md` § G2.

## Acceptance tests
- `cargo test -p validation side_ratio_remap` — the remap's round trip, the equilateral point at the pole, each decoded triangle's side ratios; control: two sides swapped fails (REQ-CHART-093).
- `cargo test -p gui equilateral_preset` — the preset's centre, no chart kind added, and the two side ratios against the remap, each with its negative control (REQ-GUI-417).

## Notes
- Written by this task (R-72, R-71), proposed in its PR: which pair of side ratios the readout shows, following PA-1's paper; which of the two mirror points the preset centres on; and the preset's window.
- Added by R-798 (5 Nov 2026, RQ-594 (a)), applied per R-369 as its own task. A build task, not a target: it is not scored, and TASK-M9-24, the target, depends on it.
