# TASK-M8-52 — On the real engine: the Decoder window's Apply re-integrating, the Chart builder's read-only links, and the warp previews (R-478 to R-480)

- **Milestone:** M8
- **Closes:** REQ-GUI-253, REQ-GUI-261, REQ-GUI-264
- **Depends on:** TASK-M6-50, TASK-M8-18, TASK-M8-89, TASK-M8-03, TASK-M4-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
On the real engine: the Decoder window's Apply sends the staged `Links` as one re-integrating, undoable edit, its previews following the live view; the Chart builder shows the active links read-only beside the Domain preview with a link to the Decoder window, and each warped axis has the same preview widget, an energy axis's curve drawn at the centre.

## References
- `decisions.md` § "R-478 — Link functions belong to the decoder, which every chart passes through, not to a chart: they are chosen and tuned in a new Decoder window (Windows › Decoder…); the Manifold view's Chart section shows them in one compact row, tinted amber when any block is off its default; the Chart builder shows them read-only and owns the axis warps; a chart preset carries no links; the instruction's §6 is queued (RQ-380 to RQ-383)"
- `decisions.md` § "R-479 — The Decoder window: one row per block (Masses, Configuration α, Configuration β, Momenta), each with a dropdown of only the compatible registry entries, the link's parameters with their ranges, a preview and a reset; a footer with "Reset all to defaults", "Changing a link re-integrates every pixel" and Apply / Revert, the edits staged and applied as one undo entry; Home and the section resets touch no link; a non-default decoder named in the canvas badge (RQ-387) and in every sidecar and export"
- `decisions.md` § "R-480 — Each link has a preview, about 160 × 110 px in the line system's styles and live while a parameter is dragged: a one-dimensional block's curve with its bounds, the visible range, the centre, its resolution and the default's ghost; the masses' simplex triangle with the image of an even grid of z lines, the view's footprint, the centre and the equal-mass point; the axis warps the same widget; every preview evaluating the generated link functions on the CPU at f64, never a reimplementation"
- `docs/gui/principia_render_gui_spec.md` § "G7a. Decoder (no artboard; R-478 to R-482)"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"

## Deliverables
- The Decoder window wired to the engine.
- The Chart builder's links and warp previews.
- Tests `chartbuilder_links_readonly`, `chartbuilder_warp_preview`, `decoder_real_engine`.

## Acceptance tests
- `cargo test -p gui chartbuilder_links_readonly` — REQ-GUI-253's verify.
- `cargo test -p gui chartbuilder_warp_preview` — REQ-GUI-261's verify.
- `cargo test -p gui decoder_real_engine` — REQ-GUI-264's verify.

## Notes
- Physics reviews the energy axes' warp preview against the per-pixel warp (R-480 F7); this task does not merge on the code reviewer's word alone.
- R-627 (27 Oct 2026): this task runs on the real engine, so where it depended on a task R-627 moves to the mock it depends on that task's real-engine side, which depends on it: TASK-M8-06 by TASK-M8-89 (applied per R-369). *Was:* "- **Depends on:** TASK-M6-50, TASK-M8-18, TASK-M8-06, TASK-M8-03, TASK-M4-21".
