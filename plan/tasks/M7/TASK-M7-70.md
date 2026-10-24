# TASK-M7-70 — Base px: every mark length of the line system and the Print graph in one unit, scaled by `output_width / base_width`, labels placed at base size, `RenderState.base_width` (R-583)

- **Milestone:** M7
- **Closes:** REQ-RENDER-118
- **Depends on:** TASK-M7-42, TASK-M7-55
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
Every mark length the line system and the Print graph draw is in base px (strokes, dashes, outlines, labels and their offsets, halos and spacing, icons, arrows, ticks, smart ink's radius, every `ref_px` parameter), rendered at `output_width / base_width`, at a scale of 1 on screen, with label placement run at base size and then scaled, so a 4× export is the screen's figure at four times the pixels and places the same labels. `RenderState.base_width` is written by the writer when a view is saved or exported, a headless writer keeping the state's, a record without it reading 1024. `PrintPt` and `LineStyle`'s `units` are gone (R-583).

## References
- `docs/design/principia_export_details.md` § "3.3 One size rule: base px (R-583)"
- `docs/design/principia_lines.md` § "3. The data model"
- `docs/design/principia_lines.md` § "5. Drawing a line"
- `docs/design/principia_lines.md` § "7. Labels"
- `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-583 — One size rule, base px: every mark length (strokes, dashes, outlines, labels, offsets, halos, spacing, icons, arrows, ticks, smart ink's radius and the Print graph's marks) is in base px, rendered at `output_width / base_width`, 1 on screen, labels placed at base size; `device_px` the one listed exception; `PrintPt` retired, a physical size an optional export field with a pt readout *(amends R-476 as to lines §3 and §5's units; amends R-490 as to `ref_px`)*"
- `decisions.md` § "R-490 — The Print graph is deterministic, seeded only; every length is in `ref_px`, `device_px` or `fraction`; the sheet is anchored to the screen; `boil` is off by default"
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"

## Deliverables
- `crates/render/src/lines/` and the Print graph's unit scaling: one `base_px → output px` factor, `output_width / base_width`, read by every mark length; label placement at base size, then scaled.
- `RenderState.base_width: u32` (its tier 2 entry in `tiers.rs`), written by the GUI's save and export writers and kept by a headless writer; 1024 for a record without it.
- Test `base_px_invariance`.

## Acceptance tests
- `cargo test -p render base_px_invariance` — a view with contours and their labels, a trace with icons and arrows, axes with ticks and a ref_px Print graph node, rendered at 1× and at 4× box-downsampled, agrees within REQ-RENDER-112's tolerance on every mark and places the identical set of labels; resizing the figure from 1024 to 1600 logical px leaves every on-screen mark length unchanged; a record without base_width renders as at 1024; a headless export keeps the state's base_width and a GUI save writes the figure's (controls: a mark kept at its screen size in the 4× export, label placement run at the 4× size, and a writer that keeps a stale base_width on a GUI save, each fail) (REQ-RENDER-118).

## Notes
- R-583 (21 Oct 2026, gap sweep PR 2c), from `export_details.md` §3 (c); the human's "line, label and mark sizes scale with the export by one rule ("base px")". TASK-M7-42 keeps REQ-RENDER-088 (no `units` field) and TASK-M7-55 REQ-RENDER-111 (`ref_px` is base px), both amended; this task makes the one rule hold across them.
- **RQ-562 is open** (the writer's `base_width` in the exported `RenderState` against R-484's identity): meanwhile its option (a), the identity on the record written; nothing waits.
- The physical size (mm and DPI) and the export block's `device_px` list are TASK-M8-80's (REQ-TOOL-205, REQ-TOOL-202).
