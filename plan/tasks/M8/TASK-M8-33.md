# TASK-M8-33 — Import picture, saved views, and record a time sweep (09_importrecord.png)

- **Milestone:** M8
- **Closes:** REQ-TOOL-104, REQ-TOOL-105, REQ-TOOL-106, REQ-TOOL-107
- **Depends on:** TASK-M8-32, TASK-M8-08
- **Needs (earlier milestones):** REQ-TOOL-061, REQ-TOOL-063, REQ-TOOL-066, REQ-TOOL-071, REQ-SYS-011
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1
- **Size:** ~450 lines

## Goal
Import picture (drop a Principia PNG or File › Import picture…) reads the pxpack, shows what is stored and what differs from now, and offers restore this view, only its stain, and open side by side. Saved views are pxpack snapshots with a thumbnail and "go", plus "Save this view", kept in the store §G9 defines (TASK-M8-08). Record a time sweep takes a time range, a frame count, the per-frame quality, a size at the view's aspect, GIF / PNG frames / MP4, overlays on or off, with pxpack in every frame, and integrates each frame to its own t through the blocking exporter — never by scrubbing.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "09 Import picture · saved views · record a sweep"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 4 — Export: the frame loop in blocking mode"

## Deliverables
- `crates/gui/src/windows/import_picture.rs`, `saved_views.rs`, `record_sweep.rs`.
- Golden suite `record-sweep` (a recorded frame at t equals a fresh blocking render at t).
- Tests: `import_diff_stain_only`, `saved_view_go`, `record_frames_pxpack`; screenshot case `09_importrecord/layout`.

## Acceptance tests
- `cargo test -p gui import_diff_stain_only` and `cargo xtask screenshot 09_importrecord` — import a PNG exported with a different stain: the diff lists the stain; 'only its stain' changes RenderState's stain and nothing else; screenshot against 09_importrecord.png (REQ-TOOL-104).
- `cargo test -p gui saved_view_go` — save a view, change it, press go: the state equals the saved one (REQ-TOOL-105).
- `cargo test -p engine record_frames_pxpack` — record 5 frames as PNG frames: each carries a pxpack whose time is its own frame time; size aspect equals the view's (REQ-TOOL-106).
- `cargo xtask golden record-sweep` — a recorded frame at t equals a fresh blocking render at t pixel for pixel (REQ-TOOL-107).

## Notes
- "Open side by side" opens the v2 side-by-side view of TASK-M8-36 once it lands; until then the action is asserted to request it.
