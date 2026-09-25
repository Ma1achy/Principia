# TASK-M7-24 — Code views: Pipeline WGSL, Node WGSL and the Problems pane

- **Milestone:** M7
- **Closes:** REQ-GUI-019
- **Depends on:** TASK-M7-14, TASK-M7-22
- **Needs (earlier milestones):** REQ-RENDER-020
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
Every pipeline stage exposes its generated WGSL for reading: the canvas's `Graph | Code` toggle shows the assembled fragment WGSL (prelude, node functions, `shade()`), the node inspector's Advanced section shows the selected node's function, per-node edits show the `{ }` badge, graph view is offered iff the pipeline still has graph structure, and the Problems pane shows compile status and errors from TASK-M7-14.

## References
- `docs/design/principia_colour_composition.md` § "5. Codegen & eject"
- `docs/gui/principia_render_gui_spec.md` § "10. `Graph | Code` — a view toggle, not an eject"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"

## Deliverables
- `crates/gui/src/stain/code_view.rs`, `problems.rs`.
- Screenshot scenarios for the Pipeline WGSL and Node WGSL views.

## Acceptance tests
- `cargo xtask screenshot 02_stain` (scenarios pipeline-wgsl, node-wgsl) — the Node WGSL and Pipeline WGSL views show the generated code (REQ-GUI-019).

## Notes
- The code shown must be the exact text the codegen compiled (TASK-M7-03), not a re-rendering.
