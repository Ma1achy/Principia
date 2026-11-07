# TASK-M8-115 — Time-axis mode in the GUI

- **Milestone:** M8
- **Closes:** REQ-GUI-422
- **Depends on:** TASK-M4-37, TASK-M8-113
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
- Waits on RQ-727. The time-zoom binding is checked against G3's table.
- Waits on its rulings item: RQ-727. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
