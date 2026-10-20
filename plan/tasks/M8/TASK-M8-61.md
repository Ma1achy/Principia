# TASK-M8-61 — The trace and Listen in Explore, 1: the Trajectory panel's tabs and sections, Overlays ▾ › Trace, the keys, the window highlight and the trace's look (R-541 G1 to G4, G9)

- **Milestone:** M8
- **Closes:** REQ-GUI-298, REQ-GUI-299, REQ-GUI-300, REQ-GUI-301, REQ-GUI-307
- **Depends on:** TASK-M8-58, TASK-M8-11, TASK-M8-12, TASK-M8-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~1,000 lines, the tests included

## Goal
R-541's placing of the trace and Listen, G1 to G4 and G9: everything about one trajectory in the Trajectory panel, each kept trajectory a tab with its own previews, trace and sound; the status line, the previews and the Trace, Listen and Numbers sections with their marker, event and effect chips; Overlays ▾'s Trace group; no trace control in the left panel or in Lines ▾; the Figure's T, ⇧T, Y and L; the shape sphere's window highlight while Listen is on; and the trace drawn one way, the defaults with their outline.

## References
- `decisions.md` § "R-541 — The trace and Listen in the dev GUI's Explore mode, as `principia_trace_listen_gui.md` places them: everything about one trajectory in the Trajectory panel, each kept trajectory a tab with its own previews, trace and sound, its Trace, Listen and Numbers sections opening `Trace…` and `Listen…`; a Trace group in Overlays ▾; no trace control in the left panel or in Lines ▾; Listen adding to the bottom band's Time column and nothing else *(amends R-476 and R-538)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/design/principia_lines.md` § "2. Where it sits"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_listen.md` § "4. Follow the head, or the whole run"
- `decisions.md` § "R-538 — The trace's markers and icon sets, as `principia_trace_markers.md` gives them: a star at the start, each end event its own icon in its outcome's colour, "?" for an incomplete run, rings where the state leaves and re-enters the chart's reach, syzygies o–o–o and off by default, off-screen icons pinned faint to the edge, every icon repeated in every image of the chart, never drawn through or started in a pixel that is not a system *(amends R-476)*"

## Deliverables
- The Trajectory panel's tabs, status line and sections, Overlays ▾'s Trace group, the keys and the window highlight (`crates/gui/`); the trace's default look (`crates/render/src/lines/`).
- Tests `trace_one_place`, `trace_keys`, `trajectory_tabs` and `listen_window_highlight`; screenshots `01_main` (trace_overlays, trajectory_panel, trace_look).

## Acceptance tests
- cargo xtask screenshot 01_main (trace_overlays) and cargo test -p gui trace_one_place (R-541 G10 One place): the open Overlays ▾ shows the Trace group after Chart; walking the widget tree finds no trace or sound control outside the Trajectory panel, Overlays ▾ › Trace, Trace…, Listen… and the Time column; a Trace group item, its Trace… control and its chip set one field, one SetField each; control: a stray control (e.g. a trace control surviving in Lines ▾ or the left panel) fails (REQ-GUI-298).
- cargo test -p gui trace_keys: with the focus in the Figure, in each mode, T, ⇧T and Y toggle their Overlays ▾ › Trace fields (one SetField each), L the sound, Space keeps and K locks; each is listed on the ? sheet; none acts while a text field is typed in or with the focus outside the Figure; controls: K keeping, or a key acting outside the Figure, each fail (REQ-GUI-299).
- cargo xtask screenshot 01_main (trajectory_panel: at rest, Trace expanded, Listen expanded) and cargo test -p gui trajectory_tabs (R-541 G10 Tabs): the panel's order and its sections' rows are §G2's; switching tabs switches the trace, the previews and the sound together; Listen's collapsed state survives a restart; control: the sound following the cursor while a kept tab is selected fails (REQ-GUI-300).
- cargo test -p gui listen_window_highlight (R-541 G10 Window highlight): the bright arc covers exactly the samples in windowAt(t) at every head time, under each Window mode; nothing is highlighted with Listen off; control: an off-by-window highlight fails (REQ-GUI-301).
- cargo xtask screenshot 01_main (trace_look): the default trace's line, outline, ticks, arrows, star, head disc and icons as stated, over a light and a dark stain; two kept traces in the palette's first two colours; control: a trace drawn without its outline fails (REQ-GUI-307).

## Notes
- **Applied meanwhile (R-541):** RQ-448 (a), the trace's controls leave Lines ▾, which keeps the rest; RQ-449 (a), K stays lock and Space keep; RQ-460 (a), the three places share one field; RQ-461 (a), the outline's entries are built only once RQ-408 is ruled; RQ-462 (a), the panel keeps FTLE, `E₀`, the playhead and the two buttons; RQ-445 (a), Listen's section remembered per user; RQ-447, R-538's palette. A ruling other than these is a follow-up change.
- TASK-M8-58's trace settings are `Lines`' state (REQ-GUI-271); this task builds the everyday controls and Overlays ▾ › Trace over them, TASK-M8-62 `Trace…`.
