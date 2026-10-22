# TASK-M8-68 — Display › Print, the Print panel: Plain / Print, the slots with live thumbnails and their settings, Boil, the 111-preset gallery in 12 families, Edit graph…, Reset; and each line set's "Through the medium" (R-485, R-525, R-532)

- **Milestone:** M8
- **Closes:** REQ-GUI-358, REQ-GUI-359, REQ-GUI-360
- **Depends on:** TASK-M8-23, TASK-M8-03, TASK-M8-50, TASK-M7-64, TASK-M7-65, TASK-M7-66, TASK-M7-67
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
The Print panel in place of Display › Style, as render_gui_spec § "Display — the last stages" and the design doc §7.1 give it: the Plain / Print toggle; the seven slots, each a dropdown with a live thumbnail and its graph's settings; Boil; the gallery of 111 presets in 12 families ("Poster › Default", never a bare "Default"); Edit graph…; Reset to Plain, one undo entry; the Viewing reason under a screen medium (R-525 V3) and "field not available at this tier". Each edit is one `RenderField::Print` SetField, drags coalescing (R-96). The thumbnail budget is proposed here. Each line set's style panel gains "Through the medium" (§7.3).

## References
- `docs/design/principia_print_graph.md` § "7.1 The Print panel (replaces Display › Style)"
- `docs/design/principia_print_graph.md` § "5. The slot chain: palette and medium compose"
- `docs/design/principia_print_graph.md` § "4.4 Payload fields as inputs"
- `docs/design/principia_print_graph.md` § "7.3 Lines"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-485 — Post-processing is the Print graph, a node graph after the stain and separate from it; it replaces the Display window's "Style", and R-130's v1 styles become presets of it, still available *(supersedes R-130)*"
- `decisions.md` § "R-487 — Lines, traces, axes and grids are drawn over the Print graph by default, and a line set can opt in to being rendered through it (`through_medium`), as the design doc §2 draws R-67's chain *(amends R-67)*"
- `decisions.md` § "R-490 — The Print graph is deterministic, seeded only; every length is in `ref_px`, `device_px` or `fraction`; the sheet is anchored to the screen; `boil` is off by default"
- `decisions.md` § "R-525 — The Viewing slot is off by default while the Medium is a screen medium, and the panel shows the reason *(closes RQ-389)*"
- `decisions.md` § "R-532 — R-130's eight styles are ported as additional Print graph presets beside the lab's 103, 111 in all *(closes RQ-396; amends R-485 and R-486)*"

## Deliverables
- `crates/gui/src/windows/display/print.rs` — the panel, its thumbnails and gallery.
- `crates/gui/src/windows/lines/` — the "Through the medium" box.
- Screenshot cases `print_panel`, `lines_style_through_medium`; tests `print_panel`, `line_through_medium_toggle`.

## Acceptance tests
- `cargo xtask screenshot print_panel (against boards/panel.png with §G13's three departures)` and `cargo test -p gui print_panel` — REQ-GUI-358's verify (the gallery holds 111 presets in 12 families; "Poster › Default" appears and no bare "Default"; picking Linocut fills Medium; Reset is one undo entry; a settings drag is one entry; the Viewing reason shows under CRT television whether or not Viewing is back on; every control is reached from the keyboard; controls: a slot showing node parameters in place of settings, a bare "Default", and a Reset making two entries, each fail).
- Calibration: the PR proposes both with the frame time measured on the reference machine while the gallery fills; a reviewer checks them; the human confirms them at the M8 gate (REQ-GUI-359).
- `cargo xtask screenshot lines_style_through_medium` and `cargo test -p gui line_through_medium_toggle` — REQ-GUI-360's verify (a new contour set shows the box unticked; ticking it is one SetField and one undo entry and the set draws through the Print graph; control: a box that edits the shared style when the set overrides it fails).

## Notes
- The look is `docs/gui/reference/print_graph/boards/panel.png` (reference only, R-485) with render_gui_spec §G13's three departures (the header-strip badge, 111 in 12, Ubuntu not IBM Plex).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
