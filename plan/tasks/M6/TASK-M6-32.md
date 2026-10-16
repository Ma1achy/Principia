# TASK-M6-32 — The console's two scopes and the keyboard's reach in every window (R-450)

- **Milestone:** M6
- **Closes:** REQ-GUI-189, REQ-GUI-190
- **Depends on:** TASK-M6-28
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
On the mock engine, the console is a window of exactly two scopes, the header bar (All, Warnings, Errors, Info, the
search field, Copy, Clear and ×, in that visual order) and the log, as the human asked after using the dev GUI
(R-450). It opens on the header bar scope in navigation mode; ↑ ↓ and Tab move between the two scopes; Enter on the
header bar goes in and ← → cross every header control, × included, Enter on × closing the console and returning the
focus to its opener; Enter on the log scrolls it by entry, page and to its ends, Shift ×10, over the whole filtered
log. First the intermittent reachability the human saw is reproduced and its cause named; then it is fixed, so that
nothing about the order depends on state; then every other window and scope is audited for the same failure.

## References
- `decisions.md` § "R-450 — The console has two scopes, its header bar and its log: opening it lands on the header bar, Enter goes in and ← → cross every header control, × included; Enter on the log scrolls it entry by entry, the whole log reachable *(amends R-405 and R-409 as they apply to the console's structure)*"
- `decisions.md` § "R-405 — The footer and the console join the keyboard's scope tree, as the top bar does: the footer is big scope 8, and Enter on it opens the console"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-404 — #174's look choices and key-repeat timings are confirmed as built: the focus ring, the breadcrumb, the `?` overlay, the 500 ms delay and 40 ms interval, and the base steps"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- First, a reproduction of the intermittent reachability (the search field, Copy or Clear sometimes unreachable)
  as a failing keyboard-script test, and its cause, named in the PR with the evidence. R-450's context gives the
  orchestrator's reading of the code (an auto-open takes no focus and records no opener; a window is outside the big
  scopes' Tab cycle; a mouse click in the console moves egui's focus, not the keyboard layer's); the task confirms or
  replaces it by reproduction.
- `crates/gui/src/windows/console.rs` (`register`, `show`) and `crates/gui/src/keyboard/scopes.rs` — the console's
  four sections replaced by two scopes, `cs_header` (All, Warnings, Errors, Info, the search field, Copy, Clear, ×) and
  `cs_log`; × a focusable button in the scope tree, Enter or a click on it closing the console through the window
  rule (focus to the opener); the console opening on `cs_header` in navigation mode; ↑ ↓ and Tab between the scopes,
  ← → across the header row stopping at its ends, Esc on a header element up to `cs_header`, Esc on a scope closing
  the window (R-450 A1, A2); the log in interaction mode scrolling by entry, page (the entries its rect shows), Home
  and End, Shift ×10, over the whole filtered list, clamped (A4); an empty list leaving `cs_log` in place (A5); a click
  on a console control moving the keyboard's focus onto it (A3); the breadcrumb's names (A6).
- A reusable reachability check (`crates/gui/src/tests/support.rs` or beside it): walk a scope or window with §G3's
  keys and assert that every drawn control is reached, with a fixture for its negative arm.
- The audit: every other window (Run, Chart builder, Export & share, Display, Profiler, the Inspector frame) and the
  big scopes checked with that walk; each finding listed in the PR and fixed here, or, where the screen is another open
  task's, named against that task (R-450 A7).
- Tests `mock_console_keys` and `mock_reachability`; `scope_tab_order_mock`, `mock_keyboard_windows` and
  `mock_keyboard` updated where they assert the console's four sections; screenshot cases
  `07_keyboard/mock_focus_console_header` and `07_keyboard/mock_focus_console_log` (`mock_focus_console` re-captured).

## Acceptance tests
- `cargo test -p gui mock_console_keys` and `cargo xtask screenshot 07_keyboard` (mock_focus_console_header, mock_focus_console_log) — the console opened from the footer and from Windows › Console lands on the header bar scope in navigation mode; from the landing every header element, the log and × are reached with only ↑ ↓, Tab, Shift+Tab, Enter, Esc and ← →; the search field types after Enter and keeps its text on Esc and Enter; Enter on × and Esc at the top level close the console with the focus on its opener; the log scrolls to its first and last entry with Home, End, Page Up, Page Down and ↑ ↓, Shift ×10, on a list longer than the rows drawn; the breadcrumb reads "Console › Header › Copy" on Copy; every script repeated after an auto-open on error and with a filter that empties the list gives the same order; controls: a landing on All, × outside the scope tree, an order that skips the log when the list is empty, a scroll that stops at the last row drawn and an Esc on a header element that closes the console each fail (REQ-GUI-189).
- `cargo test -p gui mock_reachability` — the reachability walk reaches every drawn control of the console in each of its states and of each window the audit covers; its negative arm (a drawn control left out of the scope tree, and one registered only when a list is non-empty) fails; review checklist (gui reviewer): the PR names the intermittent fault's cause with its reproduction and lists the audit's findings, each fixed or recorded against its task (REQ-GUI-190).
- `cargo test -p gui mock_keyboard scope_tab_order_mock` — the footer is still big scope 8, Enter on it opens the console with the focus inside it, and an auto-open on error takes no focus (REQ-GUI-178, closed by TASK-M6-28, re-checked here as R-450 words the console).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: open the console from the footer and
  from Windows › Console; ← → across the header to ×; Enter on ×; ↓ to the log, Enter, Home, End, Page Down, Shift+↓;
  a filter that empties the list; an error that opens the console by itself, then Tab to the footer and Enter.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5. Its look choices (R-390) are recorded in the PR as "applied per R-369" and hold until the human has tried the
  mock (R-416).
- **qa's files (R-290, forced by R-450; R-450 A9).** `crates/gui/tests/qa_TASK-M6-28.rs` and
  `crates/gui/tests/qa_TASK-M6-28_recheck.rs`, where they assert the console's four sections, its landing on the first
  filter or Esc closing it from a section, assert what R-450 changes. Every earlier commit of each is qa's, so this
  task's qa reviewer changes those assertions in the task's qa commit, and only those; the implementer never edits
  them. The PR lists each `M` with its reason, and the code reviewer confirms that no assertion was weakened but the
  ones R-450 changes.
- **The real engine.** TASK-M8-27 re-runs the console there; R-450's keys are the mock's (REQ-GUI-189, REQ-GUI-190),
  as REQ-GUI-178 is.
