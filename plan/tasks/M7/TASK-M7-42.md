# TASK-M7-42 — The line engine: its overlay pass, data model and styles, polylines with dashes by arc length, smart ink and labels (R-476, principia_lines.md §2 to §7)

- **Milestone:** M7
- **Closes:** REQ-RENDER-087, REQ-RENDER-088, REQ-RENDER-089, REQ-RENDER-090, REQ-RENDER-091
- **Depends on:** TASK-M7-21, TASK-M7-02
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~1200 lines, the tests included

## Goal
The line system's engine, outside the node graph: its overlay pass after the gamut clamp and before the colour-vision simulation; `LineStyle`, `LabelStyle`, `LineRef`, `ContourSet`, `Trace`, `Axes`, `Lines`; polylines with dashes by arc length, outline then stroke; smart ink (LightDark, KeepHue); labels along lines. No line setting touches the sim key.

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `docs/design/principia_lines.md` § "1. What it is"
- `docs/design/principia_lines.md` § "2. Where it sits"
- `docs/design/principia_lines.md` § "3. The data model"
- `docs/design/principia_lines.md` § "5. Drawing a line"
- `docs/design/principia_lines.md` § "6. Smart ink"
- `docs/design/principia_lines.md` § "7. Labels"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"

## Deliverables
- `crates/render/src/lines/` (the pass, the model, the drawing, smart ink, labels), wired into the display stage after `gamut_clamp`.
- Tests `lines_pass`, `line_model`, `dash_continuity`, `smart_ink`, `line_labels`.

## Acceptance tests
- `cargo test -p render lines_pass` — REQ-RENDER-087's verify (no re-integration).
- `cargo test -p render line_model` — REQ-RENDER-088's verify.
- `cargo test -p render dash_continuity` — REQ-RENDER-089's verify.
- `cargo test -p render smart_ink` — REQ-RENDER-090's verify.
- `cargo test -p render line_labels` — REQ-RENDER-091's verify.

## Notes
- **Where the pass sits** (R-476 M2, flagged): after the gamut clamp, before CVD simulation.
- **The mock gains no line system** (R-476 M6).
