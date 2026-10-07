# TASK-M8-27 — Console: the footer opened, on the telemetry stream (12_console.png)

- **Milestone:** M8
- **Closes:** REQ-TOOL-108
- **Depends on:** TASK-M8-05, TASK-M8-26, TASK-M8-03, TASK-M6-28
- **Needs (earlier milestones):** REQ-TOOL-002, REQ-TOOL-008, REQ-GUI-012
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The console is the footer opened: severity, time, source and message columns; filters (all, warnings, errors, info, text); copy and clear; it opens automatically on an error. It reads the same telemetry stream as the profiler, with sources including the stain, the integrator, the quadtree, the contract (every SetField logged, one entry each) and the app; console and profiler events share ids.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "12 Console"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/engine/src/telemetry/events.rs` — the event stream with ids; the contract logs each applied SetField.
- Uses and extends the track's `crates/gui/src/windows/console.rs` (TASK-M6-28, R-390): fed from the real engine's telemetry stream.
- Tests: `setfield_logged`; screenshot cases `12_console/open`, `12_console/auto_open_on_error`.

## Acceptance tests
- `cargo xtask screenshot 12_console` — screenshot against 12_console.png; raising an error opens it (REQ-GUI-126). Closed by TASK-M6-28 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo test -p engine setfield_logged` — each SetField produces one console entry with source 'contract'; console and profiler events share ids (REQ-TOOL-108).

## Notes
- R-390: TASK-M6-28 builds the console on the mock engine and closes REQ-GUI-126; this task depends on it, re-runs REQ-GUI-126's acceptance on the real engine's telemetry stream and keeps REQ-TOOL-108.
