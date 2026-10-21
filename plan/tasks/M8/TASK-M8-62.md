# TASK-M8-62 — The trace and Listen in Explore, 2: the Trace… and Listen… windows (R-541 G5, G6)

- **Milestone:** M8
- **Closes:** REQ-GUI-328, REQ-GUI-329
- **Depends on:** TASK-M8-61, TASK-M8-58, TASK-M8-59, TASK-M8-60
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
The two windows of R-541 G5 and G6: `Trace…`, every trace setting of both demos with its default under a live preview strip, for all traces or the current tab; and `Listen…`, every sound setting of `principia_listen.md` in two columns with the readout row, the instrument menu grouped by family.

## References
- `decisions.md` § "R-541 — The trace and Listen in the dev GUI's Explore mode, as `principia_trace_listen_gui.md` places them: everything about one trajectory in the Trajectory panel, each kept trajectory a tab with its own previews, trace and sound, its Trace, Listen and Numbers sections opening `Trace…` and `Listen…`; a Trace group in Overlays ▾; no trace control in the left panel or in Lines ▾; Listen adding to the bottom band's Time column and nothing else *(amends R-476 and R-538)*"
- `docs/gui/principia_render_gui_spec.md` § "Trace… — every trace setting (R-541 G5)"
- `docs/gui/principia_render_gui_spec.md` § "Listen… — every sound setting (R-541 G6)"
- `docs/gui/principia_render_gui_spec.md` § "G5. Windows (`04_windows.png`)"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_listen.md` § "8. Instruments"
- `decisions.md` § "R-540 — Listen, the trajectory heard as sound, as `principia_listen.md` gives it and its demo's code transcribes it: the spectrum of the motion of the triangle's shape plus its rotation, complex so that the winding's sign is the stereo side, pitched by the crossing time so that it is scale-free, following the head or played whole, every moving run heard; the instrument voices it and changes nothing measured; events and effects; its build waits on RQ-440 and RQ-442 *(extends R-109)*"

## Deliverables
- The `Trace…` and `Listen…` windows (`crates/gui/`), moving and resizing as every window does (R-461).
- The instrument menu's family assignment, written here (R-72; RQ-459), into `docs/design/principia_listen.md` §8 with the porting rule's "Removed lines" note.
- Tests `trace_window`, `trace_settings_keys` and `listen_window`; screenshots `trace_window` and `listen_window`.

## Acceptance tests
- cargo xtask screenshot trace_window: every control of §G5's Trace… table present with its default, the preview strip matching the settings, all traces / current tab, reset and save as default; cargo test -p gui trace_window: each control's edit is one undo entry leaving the sim key unchanged, and a current-tab edit leaves the other tabs' traces unchanged; control: a trace setting of the demo missing from the window fails; cargo test -p gui trace_settings_keys: Tab and Enter reach each of Trace…'s controls in §G5's table order in navigation mode, each changed only in interaction mode (render_gui_spec §G3, R-409); controls: a setting outside the scope tree, out of the table's order, or changed in navigation mode, each fail (REQ-GUI-328).
- cargo xtask screenshot listen_window (instrument menu open): every control of §G5's Listen… table with the code's default, the two columns, the readout row and the grouped menu, each family holding its count; cargo test -p gui listen_window: each control sets its Listen field and none touches the sim key but Horizon's extension; control: a sound setting of principia_listen.md missing fails (REQ-GUI-329).

## Notes
- **Applied meanwhile (R-541):** RQ-454, the code's defaults; RQ-459 (a), this task writes the families; RQ-461, the outline's entry waits on RQ-408; RQ-447, R-538's palette. A ruling other than these is a follow-up change.
- The gui reviewer checks the family assignment against the counts (pure 5, keys 3, struck 10, bowed and blown 7, voices 4, synths 8, textures 2).
- **Merging main (7daf689b, PR #263), applied per R-369:** REQ-GUI-271's keyboard reach and order arm (gui review 5594407176 of PR #263, G3) moves here with the controls (RQ-448 (a)), read on §G5's table order. *Was:* "- Tests `trace_window` and `listen_window`; screenshots `trace_window` and `listen_window`." and the trace_window acceptance line ending "control: a trace setting of the demo missing from the window fails (REQ-GUI-328)."
- **RQ-465, (a) applied meanwhile:** `Trace…`'s footer, the whole trace drawn when nothing plays, is the default length's; a `Window` or `LastDt` length is kept as chosen (RQ-427 (a)).
