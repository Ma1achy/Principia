# TASK-M8-42 — Screen sweep: every artboard screen present, every schema field exposed

- **Milestone:** M8
- **Closes:** REQ-GUI-069, REQ-GUI-038
- **Depends on:** TASK-M8-05, TASK-M8-06, TASK-M8-07, TASK-M8-08, TASK-M8-10, TASK-M8-11, TASK-M8-13, TASK-M8-14, TASK-M8-15, TASK-M8-16, TASK-M8-17, TASK-M8-18, TASK-M8-19, TASK-M8-20, TASK-M8-21, TASK-M8-22, TASK-M8-23, TASK-M8-24, TASK-M8-25, TASK-M8-27, TASK-M8-28, TASK-M8-32, TASK-M8-33, TASK-M8-34, TASK-M8-35, TASK-M8-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
The dev GUI provides every screen in the spec's artboard table, with Explore and Stain the only two modes and everything else a window or a tool, each compared for layout against its artboard (values illustrative, R-68). A field-coverage test lists every SimConfig and RenderState field and the control that edits it, and the GUI adds no state of its own to them.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Dev GUI (egui) — design notes for the artboards"
- `docs/gui/principia_render_gui_spec.md` § "Principia — Dev GUI (egui / F3)"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `xtask` screenshot suite `all` running every case of `01_main` … `12_console` added by the earlier tasks.
- `crates/gui/tests/field_coverage.rs` — schema paths from TASK-M8-01 ↔ controls registered by the window tasks.

## Acceptance tests
- `cargo xtask screenshot all` (01_main.png … 12_console.png) and `cargo test -p gui mode_switch_two_modes` — one screenshot per screen compared for layout with 01_main.png … 12_console.png (values are illustrative, R-68); the mode switch offers exactly Explore / Stain (REQ-GUI-069).
- Review checklist (gui reviewer), with `cargo test -p gui field_coverage` — a field-coverage test lists every schema field and the control that edits it (REQ-GUI-038).

## Notes
- none
