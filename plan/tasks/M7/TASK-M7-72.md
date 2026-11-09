# TASK-M7-72 — Contours of any registered primitive

- **Milestone:** M7
- **Closes:** REQ-RENDER-121
- **Depends on:** TASK-M2-66, TASK-M7-43
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
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- Waits on the owner's ruling of the design doc as a whole, no single item mapping here (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-873):* **Build order:** stage 3 of R-863's order, released by dispatch after stage 2 (TASK-M2-65), its milestone kept (RQ-744 (A) (b)); no edge to the M8 task is made. TASK-M2-81 and TASK-M8-108, stage 4, depend on it.
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M4-37, TASK-M7-43, TASK-M4-33".
