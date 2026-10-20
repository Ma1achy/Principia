# TASK-M7-43 — Contours: the expression's variables and totality, evaluation on the decode, levels, marching squares, and energy contours that follow the active mapping (R-476, principia_lines.md §4) ⚑

- **Milestone:** M7
- **Closes:** REQ-RENDER-092, REQ-RENDER-093, REQ-RENDER-094, REQ-RENDER-095, REQ-RENDER-096
- **Depends on:** TASK-M7-42, TASK-M2-37, TASK-M2-25
- **Needs (earlier milestones):** REQ-CHART-067, REQ-CHART-069
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
Contours of any expression over §4.1's variables, total; starting-state contours evaluated on the decode at screen resolution (GPU pass, CPU fallback), playhead contours on the quads' payload; levels by count, step or list; marching squares, no segment over a non-finite or not-a-system cell; energy contours through the decode with the per-pixel warp, true level sets on a tilted plane (L4).

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `docs/design/principia_lines.md` § "4.1 The expression"
- `docs/design/principia_lines.md` § "4.2 Evaluation"
- `docs/design/principia_lines.md` § "4.3 Levels"
- `docs/design/principia_lines.md` § "4.4 Extraction"
- `docs/design/principia_lines.md` § "10. How it fits today's other decisions"

## Deliverables
- `crates/render/src/lines/contours/` (the expression tree, evaluation, levels, extraction).
- Tests `contour_expr`, `contour_eval`, `contour_levels`, `contour_extract`, `energy_contours`.

## Acceptance tests
- `cargo test -p render contour_expr` — REQ-RENDER-092's verify.
- `cargo test -p render contour_eval` — REQ-RENDER-093's verify.
- `cargo test -p render contour_levels` — REQ-RENDER-094's verify.
- `cargo test -p render contour_extract` — REQ-RENDER-095's verify (exactness, totality).
- `cargo test -p render energy_contours` — REQ-RENDER-096's verify (L4).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers L4, the energy contours on a tilted plane (R-476 M7).
- **RQ-377 (1), (2)** wait (REQ-RENDER-097, TASK-M8-51); meanwhile §4.1's tree and its named fields.
