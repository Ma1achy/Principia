# TASK-M8-116 — Time-axis mode in Sequence and export sidecars

- **Milestone:** M8
- **Closes:** REQ-GUI-423
- **Depends on:** TASK-M4-37, TASK-M8-115, TASK-M8-77
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
The lane, the cut and the sidecar block.

## References
- `docs/design/principia_chart_composition.md` § "19. Time-axis mode in the GUI (order part D, UI)"

## Deliverables
- `docs/design/principia_sequence.md`, `docs/design/principia_export_details.md`: the words.
- `crates/gui/src/`, `crates/engine/src/`: the lane and the block.

## Acceptance tests
- `cargo test -p gui time_sequence` and `cargo test -p engine time_sidecar` — each and its control (REQ-GUI-423).

## Notes
- Waits on RQ-727.
- Waits on its rulings item: RQ-727. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
