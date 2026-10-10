# TASK-M6-28 — The GUI track (R-390), 5: the windows: Run, Chart builder, Export & share, Display, Profiler, Console, the Inspector frame

- **Milestone:** M6
- **Closes:** REQ-GUI-174, REQ-GUI-126, REQ-GUI-178
- **Depends on:** TASK-M6-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~650 lines

## Goal
On the mock engine, the windows look, feel and behave as their artboards and the design notes give them, their frames
and controls real and their content plausible data from the mock, each opened from its top-bar or menu entry and joining
the keyboard scope tree: Run (horizon, integrator, escape settings, quality, budget, recompute and cancel, under the
contract's names, no "tolerance" field); Chart builder (the axis kinds, presets, the domain preview and the quick
render, both square for a square viewport, Revert / Apply); Export & share (image with pxpack, snapshot JSON, share
link, present mode); Display (the fixed order of the last stages, the display overlays); Profiler (the window and its
tabs, with the mock's plausible trace); Console (the footer opened: severity, time, source and message columns,
filters, copy and clear, opening by itself on an error); and the Inspector window's frame (Inspect and Create & locate,
the three panes and the one timeline).
The footer and the console join the scope tree too (R-405): the footer is big scope 8, after Legend, and Enter on it
opens the console with the focus inside; the console is a window under the window rule below.

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "12 Console"
- `docs/gui/principia_render_gui_spec.md` § "G5. Windows (`04_windows.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
- `decisions.md` § "R-152 — A minimal Profiler window holds the Arbiter tab at M6 *(closes RQ-122)*"
- `decisions.md` § "R-405 — The footer and the console join the keyboard's scope tree, as the top bar does: the footer is big scope 8, and Enter on it opens the console"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"

## Deliverables
- `crates/gui/src/windows/{run,chart_builder,export,display,profiler,console,inspector}.rs`, registered in the window
  registry the Windows menu lists.
- The mock's content for them: plausible run settings, a chart's domain and quick render, a profiler trace in profiler
  schema v1's shape, console entries of each severity, an IC and its trajectory for the Inspector frame; any contract
  field a control needs, added as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on
  both engines.
- Screenshot cases `04_windows/mock_run`, `04_windows/mock_export`, `04_windows/mock_display`,
  `04_windows/mock_profiler`, `03_chartbuilder/mock_chart_builder`, `12_console/mock_console`,
  `05_inspectors/mock_inspector`, `07_keyboard/mock_focus_window`; tests `mock_windows_open`, and `mock_keyboard`
  extended.
- Each window's scope in TASK-M6-25's scope tree: an open window is a scope that takes focus when it opens; Tab and
  Shift+Tab move between its sections and Enter goes into one; Esc backs out one level, and from the window's top level
  returns focus to the scope that opened it (render_gui_spec §G3 is silent on windows; applied per R-369, review
  5434766412 on PR #157).
- The footer and the console in the scope tree (R-405; render_gui_spec §G3, §G12): the footer is big scope 8, after
  Legend, Tab wrapping from it to the top bar; Enter on it opens the console, as its click does, with the focus on the
  console's first filter. The console is a window under the rule above: opened by the footer or from Windows › Console
  it takes the focus; its sections, in Tab order, are the filters (all, warnings, errors, info), the text filter, copy
  and clear, and the entry list; Tab and Shift+Tab move between them, inside the console, and the arrow keys within a
  section in navigation mode, the entry list's ↑ and ↓ scrolling it only in interaction mode, after Enter on it, Esc
  returning to navigation mode on the list (R-409 F2, amending R-405), Enter acting on a filter, copy or clear as its
  click; the text filter landing in navigation mode, Enter on it starting typing (interaction mode), in which the
  layer stands aside and every key goes to the field, Ctrl+Z undoing the text edit and `?` typing a character, but Esc
  and Enter (back to navigation mode, the text kept) and Tab and Shift+Tab (moving on) (applied per R-369, R-409 G4,
  G5); Tab and Shift+Tab from interaction mode anywhere in the console moving to its next or previous section in
  navigation mode, staying in the console (R-409 G6); Esc in navigation mode closes the console and returns the
  focus to whatever opened it, the footer or Windows › Console; the console opening by itself on an error does not
  take the focus.
  `scope_tab_order_mock` extended to eight; screenshot case `07_keyboard/mock_focus_console`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (mock_run, mock_export, mock_display, mock_profiler), `cargo xtask screenshot 03_chartbuilder` (mock_chart_builder), `cargo xtask screenshot 12_console` (mock_console), `cargo xtask screenshot 05_inspectors` (mock_inspector) and `cargo test -p gui mock_windows_open` — against their artboards; the Chart builder's two previews are square for a square viewport; each window opens from its top-bar or menu entry (REQ-GUI-174).
- `cargo xtask screenshot 12_console` (mock_console) — screenshot against 12_console.png; raising an error opens it (REQ-GUI-126).
- `cargo test -p gui mock_keyboard` (extended with this task's scopes) and `cargo xtask screenshot 07_keyboard` (mock_focus_window) — every control this task adds joins the scope tree: each window, opened from its entry, takes focus as a scope, Tab and Shift+Tab move between its sections and Enter reaches each control (the Run window's fields, the Chart builder's axis kinds and presets, Export's formats, Display's settings, the Profiler's tabs, the Console's filters, the Inspector's panes); every landing is in navigation mode, Enter on an element starts interaction mode and Enter on a button acts as its click, Esc in interaction mode returns to navigation mode on the same element, and Esc in navigation mode goes up one scope (R-409), and Esc in navigation mode at the window's top level returns focus to the scope that opened it; the focus ring is drawn on the focused control, orange in navigation mode and blue in interaction mode, and the top-bar breadcrumb names its path; arrows move between siblings in navigation mode and adjust a focused value in interaction mode by its confirmed base step (R-404), ×10 with Shift and ×0.1 with Alt; the screenshot shows the ring and the breadcrumb as 07_keyboard.png draws them (REQ-GUI-169, closed by TASK-M6-25, re-checked here for this screen as R-409 words it, applied per R-369, review 5434766412 on PR #157).
- `cargo test -p gui mock_keyboard`, `cargo test -p gui scope_tab_order_mock` and `cargo xtask screenshot 07_keyboard` (mock_focus_console) — Tab from Legend reaches the footer and Tab from the footer the top bar; Shift+Tab from no focus starts at the footer; Enter on the footer opens the console with the focus on its first filter, in navigation mode; Tab and Shift+Tab move between the console's sections without leaving it, the arrows move within a section and Enter on clear clears it; Tab to the text filter lands in navigation mode, where a letter types nothing; Enter on it starts typing in interaction mode, letters, Space and the arrows go to the field, Esc returns to navigation mode on it with the text kept, Enter likewise, and Tab while typing moves to the next section in navigation mode; while typing, ? appears in the field and does not open the shortcuts overlay, and Ctrl+Z undoes the text edit and not the contract's history; Tab to the entry list lands in navigation mode, where ↑ and ↓ do not scroll it; Enter on it starts interaction mode and ↑ and ↓ scroll it; Esc returns to navigation mode on the list; Tab from the entry list in interaction mode moves to the console's next section in navigation mode and does not leave the console; Esc closes the console with the focus on the footer; Windows › Console opens it with the focus on its first filter, and Esc closes it with the focus on Windows › Console; a raised error opens the console without moving the focus; controls: a tree without the footer scope (Tab from Legend wraps to the top bar), a console that does not take the focus on Enter, a console landing in interaction mode, an entry list that scrolls in navigation mode, a text filter that types on landing, an Esc while typing that clears the text or closes the console, a ? while typing that opens the overlay, and a Tab from the entry list in interaction mode that leaves the console each fail; the screenshot shows the ring on a console control, in navigation mode's colour, and the breadcrumb naming the console (REQ-GUI-178, R-405, R-409 F2, G4, G5 and G6; REQ-GUI-095's eight scopes, closed by TASK-M6-25, re-checked here).
- `cargo test -p gui mock_keyboard` (console_from_menu) — Windows › Console opens the console with the focus on its first filter; Tab moves between its sections; Esc closes it with the focus back on Windows › Console; control: an Esc that returns the focus to the footer fails (REQ-GUI-178, R-405 as applied per R-369, gui review 5478696211, G2).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: Run…, Profiler…, Export… from the top bar, the Chart
  builder from the Manifold view, an error raised to open the Console, IC Inspector… from the Trajectory panel, on
  `cargo run -p gui --features mock`.
- REQ-GUI-126 moved here from TASK-M8-27 (R-390); TASK-M8-27 depends on this task, re-runs its acceptance on the real
  engine's telemetry stream and keeps REQ-TOOL-108. Every other window requirement stays with its task, each of which
  depends on this one: the Run window's contract fields and behaviour (TASK-M8-24, TASK-M6-22), the Chart builder
  (TASK-M8-18), Export & share (TASK-M8-32), Display (TASK-M8-23, TASK-M7-25), the Profiler (TASK-M8-28, and its Arbiter
  tab, TASK-M6-22 under R-152), and the Inspector (TASK-M8-14 onward).
- Values follow R-68: the Run window's defaults and names are the contract's, not the artboard's (§G13).
- Sources and silences as TASK-M6-24's Notes give them (R-390).
- RQ-245 and RQ-249, decided per R-369 (7 Oct 2026): TASK-M6-24 draws the Windows menu with each window and Console
  disabled, Run…, Profiler… and Export… disabled, and a console window frame listing the log entries (REQ-GUI-177) as
  plain rows; this task enables each entry as it builds its window, and builds the console's layout, filters, copy
  and clear and its opening on an error over that frame, re-capturing `01_main/mock_warning` to show them. The
  console's "clear" resets the footer's warning and error counts, which count from the session's start until then.
- **Present mode (R-406, R-407).** This task builds Export & share's present mode, hiding all chrome until Esc; making its
  figure fill the window, as F3 off does, is TASK-M6-30's, which depends on this task (applied per R-369, R-407, A3).
- **R-405 (10 Oct 2026).** The footer is big scope 8 and the console a window under the window rule (REQ-GUI-178).
  Applied per R-369 (R-405): "the bottom bar" is read as the footer, and the console follows the window rule, Tab
  between its sections, the arrows within one, Esc closing it and returning the focus to the footer or Windows ›
  Console, whichever opened it (gui review 5478696211, G2). What to try gains Tab to the footer, Enter, Tab and the
  arrows in the console, Esc, and Windows › Console then Esc.
- **qa's files, changed under R-405 (R-290).** `crates/gui/tests/qa_TASK-M6-25.rs`'s
  `qa_scope_tab_order_mock_seven_big_scopes_cycle` (its `BIG` list and its module doc's quote of REQ-GUI-095's verify)
  asserts seven scopes. Every earlier commit of the file is qa's, so this task's qa reviewer changes it to eight in the
  task's qa commit; the implementer never edits it, and until that commit the test fails on the implementer's head, as
  the PR says. The PR lists each change with its reason, and the code reviewer confirms that no assertion was weakened
  but the count R-405 changes. `qa_TASK-M6-25_recheck.rs` needs no change: it asserts no scope count and does not take
  Legend as the last big scope (gui review 5478696211).
- **R-409 (10 Oct 2026).** Every window, the footer and the console follow the two keyboard modes TASK-M6-31 builds
  (through TASK-M6-27): each landing, the console's first filter included, is in navigation mode; Enter on a button
  acts as its click; Esc in navigation mode at a window's top level is the window rule's Esc. The window rule's
  deliverable above reads with them. REQ-GUI-178 says so (applied per R-369, R-409 points 4 and 5).
- **R-409's follow-up answer (F2, 10 Oct 2026).** The human chose "Enter first" for the console's entry list: it lands
  in navigation mode, Enter starts interaction, ↑ and ↓ then scroll it, and Esc returns to navigation mode on it. This
  amends R-405 in part ("↑ and ↓ scroll the entry list"); the deliverable and REQ-GUI-178's line read with it. What to
  try gains Tab to the entry list, ↓ (no scroll), Enter, ↑ ↓, Esc.
