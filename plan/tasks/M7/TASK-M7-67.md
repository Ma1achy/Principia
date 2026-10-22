# TASK-M7-67 — `through_medium` in the line renderer: two passes with complementary filters, the sets drawn through before the Print graph, smart ink reading each pass's figure (R-487, R-531)

- **Milestone:** M7
- **Closes:** REQ-RENDER-114
- **Depends on:** TASK-M7-42, TASK-M7-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
`LineStyle.through_medium` (default false, a `LineRef` override allowed, R-487 K2): the one line renderer called twice with complementary filters, the sets marked through composited onto the stain's output before the Print graph, every other set after the gamut clamp and before the colour-vision simulation (R-531). Axes are a line set, so the switch applies. Smart ink reads the printed figure to the clamp for a set drawn over and the stain's output for a set drawn through (R-531 O1; REQ-RENDER-090 restated). §11 test 6.

## References
- `docs/design/principia_print_graph.md` § "2. Where it sits"
- `docs/design/principia_print_graph.md` § "7.3 Lines"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/design/principia_lines.md` § "2. Where it sits"
- `docs/design/principia_lines.md` § "3. The data model"
- `docs/design/principia_lines.md` § "6. Smart ink"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `decisions.md` § "R-487 — Lines, traces, axes and grids are drawn over the Print graph by default, and a line set can opt in to being rendered through it (`through_medium`), as the design doc §2 draws R-67's chain *(amends R-67)*"
- `decisions.md` § "R-531 — The lines drawn over the Print graph keep R-476 M2's place, after the gamut clamp and before the colour-vision simulation; the lines drawn through sit before the Print graph; the design doc §2's diagram is corrected to match *(closes RQ-395; amends R-487)*"

## Deliverables
- `crates/render/src/lines/` — the `through_medium` field and the second call before the print stage.
- Test `lines_through_medium`.

## Acceptance tests
- `cargo test -p render lines_through_medium` — REQ-RENDER-114's verify (a contour set drawn over renders identically with and without a Print graph; marked through, it differs; axes follow the same switch; smart ink over a printed figure takes its luminance from the printed picture, and through from the stain's output; controls: a single pass that draws every set after the Print graph fails the through arm, and a set drawn over placed before the gamut clamp fails the over arm).

## Notes
- The line set's "Through the medium" box is TASK-M8-68's (REQ-GUI-360).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
