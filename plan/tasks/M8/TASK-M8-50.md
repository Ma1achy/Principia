# TASK-M8-50 — Lines ▾, its editors, the axes and presets, Grid, t_end and class edges out of Overlays, and lines in the exported view state (R-476 L2, §8b, §9; R-509)

- **Milestone:** M8
- **Closes:** REQ-GUI-246, REQ-GUI-247, REQ-GUI-248
- **Depends on:** TASK-M7-42, TASK-M7-43, TASK-M8-87, TASK-M8-08, TASK-M8-94, TASK-M8-03
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
Lines ▾ beside Overlays ▾ with its count and `Alt+L`; the contour-set editor and the style library; the figure's axes as lines (Screen the default, Print, Ends only); the Grid and t_end presets, which leave Overlays ▾ and the Display window on the mock and the real engine; `RenderState` gains `Lines`, its `Overlays` losing `grid` and `t_end_contours`; lines saved with the view, undoable, never re-integrating.

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/design/principia_lines.md` § "2. Where it sits"
- `docs/design/principia_lines.md` § "3. The data model"
- `docs/design/principia_lines.md` § "8b. Axes"
- `docs/design/principia_lines.md` § "9. Persistence, undo, provenance"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/explore/` (Lines ▾, its editors), `axis_labels.rs` (the axes), `overlays_menu.rs` and `crates/gui/src/windows/display.rs` (the toggles removed), `crates/engine/src/contract/render_state.rs` (`Lines`; `Overlays` without `grid`, `t_end_contours`), `store.rs` and its canonical tests.
- Tests `line_axes`, `lines_menu`, `lines_provenance`; capture cases `mock_lines_dimmed` (the mock's top bar with Lines ▾ dimmed, and its Overlays and Display window without the three toggles) and `lines_menu` (real engine: the menu, its count, `Alt+L`, the presets drawing). *Was:* "a capture case `mock_lines_menu` (the mock's Overlays without the two toggles)".

## Acceptance tests
- `cargo test -p gui line_axes` and `cargo xtask screenshot 01_main` — REQ-GUI-246's verify.
- `cargo test -p gui lines_menu`, `cargo xtask screenshot 01_main` (`mock_lines_dimmed`) and the real-engine capture `lines_menu` — REQ-GUI-247's verify.
- `cargo test -p gui lines_provenance` — REQ-GUI-248's verify (§12 provenance, no re-integration).

## Notes
- **qa's files (R-290, R-476 M12; applied per R-369, a ruling forcing the change).** These must change in this task's qa commit and only there, each `M` line listed with its reason: `crates/engine/tests/qa_TASK-M0-18.rs` (`RENDER_TEXT` :62-66, its doc :55), `crates/prin/tests/qa_TASK-M0-18.rs` (`expected_config_text` :165-171) and `crates/gui/tests/qa_TASK-M6-27.rs` (:951-952, :1016-1021, :1058-1062); and `xtask/tests/qa_TASK-M6-24.rs`'s `check_01_main` gains `mock_lines_dimmed` (named for what it shows; gui review G5). The implementer's own files follow (R-476 M12's list).
- **The mock gains no line system** (R-476 M6): it drops the three toggles and shows Lines ▾ dimmed, its count 0, with a tooltip, `Alt+L` doing nothing there (gui review G5 of PR #255). *Was:* "it only drops the two toggles".
- **Class edges (R-509 Y5):** the preset Class edges, contours of the stain's categorical field; `Overlays` loses `class_edges` too, so the canonical text holds eleven toggles. The qa files above change in the same lines for it (R-476 M12's list, its reason extended), `qa_TASK-M6-27.rs`'s Stain group (:952) going; the code reviewer confirms no other assertion is weakened.
- **Keyboard and captures (R-476 M9, M13; gui review G5 to G7 of PR #255):** `Alt+L` is global and never the Figure's L; Lines ▾ follows Overlays ▾ in the top bar's scope; the editors are windows. REQ-GUI-247's verify takes the mock capture `mock_lines_dimmed` and a real-engine capture `lines_menu`; `check_01_main` gains `mock_lines_dimmed` (*was:* `mock_lines_menu`). REQ-GUI-078's Lines ▾ clause is REQ-GUI-247's.
- **R-541 (21 Oct 2026):** Lines ▾ lists no trace: the traces' controls are Overlays ▾ › Trace's, the Trajectory
  panel's and `Trace…`'s (TASK-M8-61, TASK-M8-62). REQ-GUI-247 is restated (*was:* "it lists the contour sets, traces
  and axes with their visibility"). Lines ▾ stays for the rest of the line system: R-556 rules RQ-448 (a),
  Lines ▾ kept for contours, axes, grids, the presets and class edges.
- R-484 J4 (20 Oct 2026): the line system is tier 2 of the embedding.
- R-627 (27 Oct 2026): this task runs on the real engine, so where it depended on a task R-627 moves to the mock it depends on that task's real-engine side, which depends on it: TASK-M8-05 by TASK-M8-87; TASK-M8-23 by TASK-M8-94 (applied per R-369). *Was:* "- **Depends on:** TASK-M7-42, TASK-M7-43, TASK-M8-05, TASK-M8-08, TASK-M8-23, TASK-M8-03".
