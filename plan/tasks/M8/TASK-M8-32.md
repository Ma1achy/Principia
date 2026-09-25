# TASK-M8-32 — Export & share window: image with pxpack, state snapshot and share link, present mode (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-TOOL-102, REQ-TOOL-103, REQ-GUI-100
- **Depends on:** TASK-M8-30, TASK-M8-31, TASK-M8-05, TASK-M7-32
- **Needs (earlier milestones):** REQ-TOOL-059, REQ-TOOL-065, REQ-TOOL-067, REQ-TOOL-070, REQ-TOOL-071, REQ-TOOL-072
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The Export & share window offers image export (size in multiples of the view, format, embed the view as pxpack, optionally the stain's WGSL) so that opening the picture recreates the view exactly; state export (copy snapshot JSON, save snapshot, load, and a `principia://view?…` share link), each recreating the view; and present mode, which hides all chrome until Esc.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 6 — Sharing: the spec is the object, the video is its shadow"

## Deliverables
- `crates/gui/src/windows/export_share.rs` — calls the exporter (TASK-M8-30) and the M7 embedding.
- `crates/gui/src/present.rs`.
- Tests: `png_pxpack_roundtrip`, `state_share_roundtrip`; screenshot cases `04_windows/export`, `01_main/present`, `01_main/present_esc`.

## Acceptance tests
- `cargo test -p gui png_pxpack_roundtrip` — export a PNG with pxpack, import it: the restored SimConfig + RenderState equal the exported ones field for field (REQ-TOOL-102).
- `cargo test -p gui state_share_roundtrip` — round-trip each of JSON copy, saved file and share link: the state is equal after load (REQ-TOOL-103).
- `cargo xtask screenshot 01_main` (present mode; Esc restores the layout) — screenshot in present mode shows only the figure; Esc restores 01_main.png's layout (REQ-GUI-100).

## Notes
- none
