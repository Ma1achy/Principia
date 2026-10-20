# TASK-M8-51 — The line system's open questions as RQ-377 rules: the expression syntax and playhead list, the trace fade's distance, label collisions with trace ticks, new contour sets' default style

- **Milestone:** M8
- **Closes:** REQ-RENDER-097, REQ-RENDER-098, REQ-GUI-249, REQ-GUI-250
- **Depends on:** TASK-M7-43, TASK-M8-49, TASK-M8-50
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
Apply RQ-377's rulings: (1) and (2) the expression's surface syntax and the playhead quantities; (3) the trace fade's off-plane distance; (4) contour labels against trace ticks; (6) the default style. Part (5), class edges, has no requirement (nothing waits).

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `docs/design/principia_lines.md` § "11. Open questions"
- `docs/design/principia_lines.md` § "4.1 The expression"
- `docs/design/principia_lines.md` § "7. Labels"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_lines.md` § "6. Smart ink"

## Deliverables
- Per RQ-377's ruling, in `crates/render/src/lines/` and the contour-set editor.
- Tests `contour_syntax`, `label_collisions_traces`, `trace_fade`, `contour_default_style`.

## Acceptance tests
- `cargo test -p render contour_syntax` — REQ-RENDER-097's verify.
- `cargo test -p render label_collisions_traces` — REQ-RENDER-098's verify.
- `cargo test -p render trace_fade` — REQ-GUI-249's verify.
- `cargo test -p gui contour_default_style` — REQ-GUI-250's verify.

## Notes
- **Physics reviews** part (3)'s distance, which needs the encode and a metric (RQ-341, RQ-346).
- **A leaf task waiting for RQ-377**: nothing depends on it.
