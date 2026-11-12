# TASK-M8-115 — Time-axis mode in the GUI

- **Milestone:** M8
- **Closes:** REQ-GUI-422
- **Depends on:** TASK-M2-58, TASK-M8-113, TASK-M8-108, TASK-M2-98
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Every bullet of the GUI table, with its mock-up.

## References
- `docs/design/principia_chart_composition.md` § "19. Time-axis mode in the GUI (order part D, UI)"

## Deliverables
- `docs/gui/principia_render_gui_spec.md`: G2, G3, G4, G5, G7, G8.
- `crates/gui/src/`: the mode.

## Acceptance tests
- `cargo test -p gui time_mode` — each bullet and its control (REQ-GUI-422).

## Notes
- **Build order (R-863):** stage 5, time-axis charts and the symmetry filter. It depends on stage 4's task, TASK-M8-108, and merges only with its tests green.
- The time-zoom binding is checked against G3's table.
- Waits on its rulings item: RQ-727. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *11 Nov 2026 (the addendum's § A.8, CHANGE; the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; R-940, R-942):* time ticks come from TASK-M2-98's computed axis; a `gauge` end's paper scale is drawn on its own end's side, fading with its share (the addition's § B, B4). It depends on TASK-M2-98 (computed axes, the maths); TASK-M8-130 then draws the computed axes on the figure. *Was (Depends on):* "TASK-M2-58, TASK-M8-113, TASK-M8-108".
