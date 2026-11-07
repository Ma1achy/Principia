# TASK-M7-72 — Contours of any registered primitive

- **Milestone:** M7
- **Closes:** REQ-RENDER-121
- **Depends on:** TASK-M4-37, TASK-M7-43, TASK-M4-33
- **Needs (earlier milestones):** REQ-CHART-096
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
The figure's Tier 1: a contour of any registered primitive's forward, display-only and never scored.

## References
- `docs/design/principia_chart_composition.md` § "10.2 Tier 1 overlays"
- `docs/design/principia_lines.md` § "4.1 The expression"

## Deliverables
- `crates/render/src/lines/contours/`: the new variables and `acos`.

## Acceptance tests
- `cargo test -p render contour_primitives` — each forward on the decode; the figure's two families at exact angles; no scored field reached; control: an angle off by 1° (REQ-RENDER-121).

## Notes
- Waits on the owner's ruling of the design doc as a whole, no single item mapping here (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
