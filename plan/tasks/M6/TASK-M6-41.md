# TASK-M6-41 — Focus and keyboard navigation apart: a mouse-opened window takes the focus with no ring; keyboard navigation starts only on keys (R-460)

- **Milestone:** M6
- **Closes:** REQ-GUI-210
- **Depends on:** TASK-M6-32
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
On the mock engine, opening the console, the Chart builder, the Profiler or any window with the mouse turns keyboard navigation on. The cause, read from the code at `ab87a879` for this task to confirm by reproduction: there is no keyboard-navigation-off state; the ring is drawn whenever the focus path is set (`crates/gui/src/app.rs` lines 920 to 930, `draw_ring`), and `Keyboard::open_window` (`crates/gui/src/keyboard/mod.rs` line 434) moves the path into every window however it was opened; a mouse-opened menu does the same (lines 382 to 385). R-450 A3 is not the cause; it is built on PR #234. This task adds the GUI's own keyboard-navigation flag beside the focus (R-460): it starts only on keyboard input (Tab, Shift+Tab, the arrows; Enter only reveals the ring), a mouse open takes the focus with no ring, and the ring and the breadcrumb's path are drawn only while it is on. It fixes the one mechanism, not each window.

## References
- `decisions.md` § "R-460 — Focus and keyboard navigation are separate: a window opened with the mouse takes the focus but shows no ring and leaves keyboard navigation off; keyboard navigation starts only on keyboard input; a test per entry point, with a keyboard control arm *(amends R-409 point 4's "always in one of two modes" and R-450's landing "the whole scope highlighted", for a window opened with the mouse; reads R-456's blue ring on a clicked text field as shown only while keyboard navigation is on)*"
- `decisions.md` § "R-456 — A click on a text input starts typing, in interaction mode; a click anywhere else lands in navigation mode *(amends R-450 A3, and R-409's landing rule and G4, for text inputs)*"
- `decisions.md` § "R-450 — The console has two scopes, its header bar and its log: opening it lands on the header bar, Enter goes in and ← → cross every header control, × included; Enter on the log scrolls it entry by entry, the whole log reachable *(amends R-405 and R-409 as they apply to the console's structure)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-405 — The footer and the console join the keyboard's scope tree, as the top bar does: the footer is big scope 8, and Enter on it opens the console"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "07 Keyboard — design note, not a screen"

## Deliverables
- The keyboard-navigation flag on `Keyboard` (`crates/gui/src/keyboard/`), set by Tab, Shift+Tab, the arrows and Enter (R-460 H1), cleared by a mouse open (H2), never by a click otherwise until RQ-367 is ruled (H3).
- `draw_ring` and the breadcrumb drawing the focus only while it is on; `open_window`, `close_window` and `follow_menus` keep moving the focus as now.
- Test `mock_keyboard_nav_state`: one script per entry point with its keyboard control arm (H5).

## Acceptance tests
- cargo test -p gui mock_keyboard_nav_state (pointer and keyboard scripts on the mock), one script per entry point: the footer, Windows › each of Run, Chart builder, Export & share, Display, Profiler, Console and Inspector, the top bar's Run…, Profiler… and Export…, the Manifold view's Chart builder… and open in Inspector, and the Trajectory panel's IC Inspector… and Open full viewer…: opened by a click with no key pressed, the focus is on the window's landing and keyboard navigation is off, no ring drawn; control arm for each: the same window opened by keys (Enter on the footer, a menu entry chosen by Enter, a button reached by Tab and pressed by Enter) has keyboard navigation on and the ring on its landing, so a GUI that never turns it on fails; after a mouse open, Tab turns it on and moves on from the landing, and Enter turns it on without acting; a menu opened by a click shows no ring; Esc closes a mouse-opened window with it still off; a click on a console control or the figure leaves it as it was, off or on; control: a GUI that turns it on at a mouse open (today's) fails (REQ-GUI-210).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: click the footer: the console opens with no ring; press Tab: the ring appears and moves on; close it, open the Profiler from the top bar with the mouse: no ring; open it again from the Windows menu by keys: the ring is on its first section.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **RQ-367** holds the human's unconfirmed recommendation (a click or drag turns keyboard navigation off). Meanwhile a click never turns it on and leaves it as it was (H3); a ruling for (a), (b) or (ii) is applied here if the task is still open, else by a follow-up.
- **The cause** goes in the PR, confirmed or replaced by reproduction, as the human asked ("Find the actual cause and report it").
- **Existing tests** that assert the focus's path and mode after a mouse open (`mock_windows_open.rs`, `qa_TASK-M6-28.rs`) stay true; qa assertions that a mouse open shows the ring are changed by this task's qa reviewer only (R-290, H7).
- **Early start (R-388).** It may start off TASK-M6-32's approved head (PR #234).
