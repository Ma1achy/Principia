# TASK-M6-42 — Windows that move, resize, stay on screen and remember their place (R-461)

- **Milestone:** M6
- **Closes:** REQ-GUI-211, REQ-GUI-212
- **Depends on:** TASK-M6-32
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
On the mock engine, every window but the console (Run, Chart builder, Export & share, Display, Profiler, Inspector) is dragged by its title bar, resized from its edges and corners, kept at least partly on screen, and remembers its position and size across sessions (R-461). Today `windows::frame` (`crates/gui/src/windows/mod.rs` lines 204 to 237) pins each with `.fixed_rect` and `.resizable(false)`, and the GUI keeps no settings across sessions.

## References
- `decisions.md` § "R-461 — Every window is dragged by its title bar and resized from its edges and corners, stays at least partly on screen, and remembers its position and size across sessions"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-450 — The console has two scopes, its header bar and its log: opening it lands on the header bar, Enter goes in and ← → cross every header control, × included; Enter on the log scrolls it entry by entry, the whole log reachable *(amends R-405 and R-409 as they apply to the console's structure)*"
- `docs/gui/principia_render_gui_spec.md` § "G5. Windows (`04_windows.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"

## Deliverables
- Movable, resizable windows with minimum sizes and the on-screen clamp (R-461 I1 to I3), the keyboard's window model unchanged (I5).
- The GUI's own settings, kept across sessions (window places now, the layout in TASK-M6-43), never the contract, an unreadable file ignored and logged once (I4).
- Tests `mock_windows_move_resize` and `mock_windows_remembered`.

## Acceptance tests
- cargo test -p gui mock_windows_move_resize (pointer scripts on the mock), for each of the six windows: a drag on the title bar moves it by the drag; a drag on each edge and corner resizes it, clamped at its minimum; a drag that would take it off any side leaves its title bar's height and 64 points of its width inside; shrinking the GUI's window keeps it so; controls: a window fixed in place (today's fixed_rect), one not resizable, and one dragged wholly off screen each fail (REQ-GUI-211).
- cargo test -p gui mock_windows_remembered: a window moved and resized, the GUI's state saved and a new GUI started from it, opens at the same place and size; a remembered place off a smaller screen is clamped; with no settings each window opens at today's place; a corrupt settings file is ignored with one console line; no window place appears in the contract's snapshot or history; control: a GUI that forgets the place across a restart fails (REQ-GUI-212).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: open the Profiler, drag it by its title bar and resize it from a corner; try to drag it off screen; quit and start again: it opens where you left it.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **Storage.** How the settings are stored (a file under the platform's config directory, eframe's persistence, or another) is the task's choice within the repo's rules for dependencies; the PR says which and why.
- **The console** is docked, its height TASK-M6-44's (R-463).
- **Early start (R-388).** It may start off TASK-M6-32's approved head (PR #234).
