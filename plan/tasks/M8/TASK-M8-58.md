# TASK-M8-58 — The trace's markers and icon sets, its defaults, syzygies, off-screen pins, repeats, not-a-system handling and computeIC's guards (R-538; R-537 EX6) ⚑

- **Milestone:** M8
- **Closes:** REQ-GUI-271, REQ-GUI-272, REQ-GUI-273, REQ-GUI-274, REQ-GUI-275, REQ-GUI-276, REQ-INT-096
- **Depends on:** TASK-M8-49, TASK-M8-50, TASK-M8-51, TASK-M8-55
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
The trace's markers are R-538's: the four icon sets (Principia the default), the end icons in their outcome colours, "?" for an incomplete run, the reach rings, syzygies (off by default) placed on `w = 0` in collinear order, off-screen icons pinned faint to the edge, every icon in every image of an extended or wrapped chart (R-537 EX6), never drawn through or started in a pixel that is not a system, TM2's defaults, and `computeIC`'s two guards.

## References
- `decisions.md` § "R-538 — The trace's markers and icon sets, as `principia_trace_markers.md` gives them: a star at the start, each end event its own icon in its outcome's colour, "?" for an incomplete run, rings where the state leaves and re-enters the chart's reach, syzygies o–o–o and off by default, off-screen icons pinned faint to the edge, every icon repeated in every image of the chart, never drawn through or started in a pixel that is not a system *(amends R-476)*"
- `decisions.md` § "R-537 — The chart's extension past `[0,1]²` is reached by pan and zoom with the controls shown, not only under F3: wherever the view lands it renders with no gaps, every interaction works there, a pixel in a repeat resolving to its canonical system, and the quadtree, refinement and deep zoom work in extended and wrapped regions, one system computed once ⚑ *(amends R-406, R-407 and R-472)*"
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `decisions.md` § "R-509 — `principia_lines.md` §11's six questions are ruled: contours in the stain's expression syntax; the playhead reads any ledger-declared payload field; the trace fade uses the proxy where it is defined, else no fade and says so, the real distance waiting on RQ-341; trace labels win; class edges move to the line system; Screen contour the default, smart ink opt-in *(closes RQ-377; amends R-476)*"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_lines.md` § "3. The data model"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- The icon sets and markers, the syzygy detection and placement, edge pinning, repeats and the not-a-system checks in the trace renderer (`crates/render/src/lines/`), the trace settings in Lines ▾ (`crates/gui/`), and the guards in `computeIC` (the CPU kernel instance).
- Tests `trace_defaults`, `trace_markers`, `trace_syzygy`, `trace_edge_pins`, `trace_repeats`, `trace_not_a_system` and `compute_ic_guards`.

## Acceptance tests
- `cargo test -p render trace_defaults` — REQ-GUI-271's verify (R-538 T1).
- `cargo test -p render trace_markers` — REQ-GUI-272's verify (R-538 T2).
- `cargo test -p render trace_syzygy` — REQ-GUI-273's verify (R-538 T3, T4).
- `cargo test -p render trace_edge_pins` — REQ-GUI-274's verify (R-538 T5).
- `cargo test -p render trace_repeats` — REQ-GUI-275's verify (R-538 T6, T7, T12).
- `cargo test -p render trace_not_a_system` — REQ-GUI-276's verify (R-538 T8, T9, T10).
- `cargo test -p kernel compute_ic_guards` — REQ-INT-096's verify (R-538 T11).

## Notes
- **⚑ Physics review is required**: the syzygy test on `w`, the guards and the forbidden-sample count; it does not merge on the code reviewer's word alone.
- **Applied meanwhile (R-538 I8, I13):** RQ-412 to RQ-415's option (a); RQ-418 (a), a non-finite step ends the run `sim_failed`, its trace ending with "?"; RQ-417 (b) for the readout. A ruling other than these is a follow-up change.
- **REQ-GUI-245** (TASK-M8-49) no longer draws the markers; this task replaces them. The icons are figure marks (R-539).
