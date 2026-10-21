# TASK-M8-64 — Listen in Explore, 1: the Trajectory panel's Listen section, the Figure's L and the window highlight (R-541 G2, G4)

- **Milestone:** M8
- **Closes:** REQ-GUI-334, REQ-GUI-327
- **Depends on:** TASK-M8-61, TASK-M8-12, TASK-M8-59, TASK-M8-60
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
Listen's half of R-541 G2 and G4, split from TASK-M8-61 so that the trace's GUI does not wait on Listen's RQs: the Trajectory panel's Listen section between Trace and Numbers, collapsed until first used and then remembered per user; each kept tab's own sound, switching with the tab; the Figure's L; and the shape-sphere preview's window highlight while Listen is on. **It waits on RQ-440 and RQ-442** through TASK-M8-12.

## References
- `decisions.md` § "R-541 — The trace and Listen in the dev GUI's Explore mode, as `principia_trace_listen_gui.md` places them: everything about one trajectory in the Trajectory panel, each kept trajectory a tab with its own previews, trace and sound, its Trace, Listen and Numbers sections opening `Trace…` and `Listen…`; a Trace group in Overlays ▾; no trace control in the left panel or in Lines ▾; Listen adding to the bottom band's Time column and nothing else *(amends R-476 and R-538)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/design/principia_listen.md` § "4. Follow the head, or the whole run"
- `decisions.md` § "R-540 — Listen, the trajectory heard as sound, as `principia_listen.md` gives it and its demo's code transcribes it: the spectrum of the motion of the triangle's shape plus its rotation, complex so that the winding's sign is the stereo side, pitched by the crossing time so that it is scale-free, following the head or played whole, every moving run heard; the instrument voices it and changes nothing measured; events and effects; its build waits on RQ-440 and RQ-442 *(extends R-109)*"

## Deliverables
- The Listen section in TASK-M8-61's panel, the tab's sound, the Figure's L and the window highlight (`crates/gui/`).
- Tests `listen_section` and `listen_window_highlight`; screenshot `01_main` (trajectory_panel_listen).

## Acceptance tests
- cargo xtask screenshot 01_main (trajectory_panel_listen: Listen collapsed, Listen expanded) and cargo test -p gui listen_section (R-541 G10 Tabs, its sound half): the section's rows and controls are §G2's; switching tabs switches the sound with the trace and the previews; Listen's collapsed state survives a restart and is not in the contract's state; with the focus in the Figure, in each mode, L toggles the sound and is on the ? sheet, and acts neither with the focus outside the Figure nor while a text field is typed in; Tab and Enter reach the section's controls in §G2's order (♪, the instrument, ⋯; expanded, volume, listen mode, loudness, the event and effect chips) in navigation mode, each changed only in interaction mode (§G3, R-409); controls: the sound following the cursor while a kept tab is selected, the state reset on restart or kept in the contract's state, L acting outside the Figure, and a control unreachable, out of order or changed in navigation mode, each fail (REQ-GUI-334).
- cargo test -p gui listen_window_highlight (R-541 G10 Window highlight): the bright arc covers exactly the samples in windowAt(t) at every head time, under each Window mode; nothing is highlighted with Listen off; control: an off-by-window highlight fails (REQ-GUI-327).

## Notes
- **Split (code review 5595371779 of PR #267, F2; the orchestrator's decision, applied per R-369):** REQ-GUI-334 is new, from REQ-GUI-325's L and REQ-GUI-326's Listen section, tab sound and remembered state; REQ-GUI-327 moves here from TASK-M8-61.
- **Applied meanwhile (R-541):** RQ-445 (a), Listen's section remembered per user in the GUI's own settings; RQ-449 (a), K stays lock. A ruling other than these is a follow-up change.
