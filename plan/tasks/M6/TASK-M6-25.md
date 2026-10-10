# TASK-M6-25 — The GUI track (R-390), 2: keyboard navigation through every screen

- **Milestone:** M6
- **Closes:** REQ-GUI-169, REQ-GUI-095, REQ-GUI-098, REQ-GUI-146, REQ-GUI-158
- **Depends on:** TASK-M6-24
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
On the mock engine, the GUI is the tree of keyboard scopes render_gui_spec §G3 gives: Tab order 1 top bar · 2 Manifold
view · 3 Figure · 4 Trajectory · 5 Compass · 6 Time · 7 Legend, with Manifold view's sub-scopes reached with Enter.
Tab and Shift+Tab move between the big scopes, Enter goes in and Esc backs out one level, arrows move between siblings
or adjust a focused value, Shift steps ×10 and Alt ×0.1, held keys delay then repeat (DAS / ARR), and `?` shows the
shortcuts over everything. Focus shows only as a ring on the current scope and the top-bar breadcrumb. The scope lives
in `ViewUI`. The scope tree is built so each later track screen joins it, and every later track task's keys run through
it. This task proposes the repeat delay and rate and the base steps, the first task to need them.

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "07 Keyboard — design note, not a screen"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"

## Deliverables
- `crates/gui/src/keyboard/{scopes,keymap,repeat}.rs` — the scope tree, the global key table, DAS / ARR repeat; a
  registration each later screen uses to join the tree.
- `crates/gui/src/explore/breadcrumb.rs` — filled from `ViewUI`'s focus scope; the focus ring.
- The `?` shortcuts overlay, over everything, closed by Esc.
- Calibration proposals: the repeat delay and rate (REQ-GUI-146) and the base step per adjustable field kind and per
  in-scope arrow action, pan, tilt, orbit and time step (REQ-GUI-158), each with the reference or reasoning it follows;
  used provisionally until the human confirms them at the M8 gate (R-182).
- Tests `mock_keyboard`, `scope_tab_order_mock`; screenshot cases `07_keyboard/mock_focus_before`,
  `07_keyboard/mock_focus_after`, `07_keyboard/mock_shortcuts`.

## Acceptance tests
- `cargo test -p gui mock_keyboard` and `cargo xtask screenshot 07_keyboard` (mock_focus_before, mock_focus_after, mock_shortcuts) — synthetic key events for Tab, Shift+Tab, Enter, Esc, the arrows and the Shift and Alt steps move focus and values as §G3 gives; `?` opens the shortcuts over everything and Esc closes it; screenshots against 07_keyboard.png (REQ-GUI-169).
- `cargo test -p gui scope_tab_order_mock` — Tab / Shift+Tab cycles the seven scopes in order; Enter on Manifold view enters Chart (REQ-GUI-095).
- `cargo xtask screenshot 07_keyboard` (mock_focus_before, mock_focus_after) — screenshots before and after moving focus differ only in the ring and the breadcrumb (07_keyboard.png) (REQ-GUI-098).
- Review checklist (gui reviewer) of the DAS / ARR proposal — the proposal gives the delay and rate with the reference they follow (e.g. platform defaults); a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-146).
- Review checklist (gui reviewer) of the base-step proposal — a table of base steps per field kind and scope (absolute or relative to the field's range or the view), with the reasoning; the human confirms it at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-158).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: Tab and Shift+Tab around the scopes, Enter and Esc,
  arrows with Shift and Alt, `?`, on `cargo run -p gui --features mock`.
- REQ-GUI-095, REQ-GUI-098, REQ-GUI-146 and REQ-GUI-158 moved here from TASK-M8-13 (R-390), which depends on this task,
  re-runs REQ-GUI-095's and REQ-GUI-098's acceptance on the real engine and uses the proposed values. The in-scope keys
  (REQ-GUI-097) and Ctrl+Z from the contract's history with held-key repeat checked end to end (REQ-GUI-096) stay
  TASK-M8-13's; the keys of each later track screen join the tree in that screen's task.
- Each later track task, TASK-M6-26 to TASK-M6-29, extends `mock_keyboard` with its own scopes and adds a
  `07_keyboard` focus case, so the keyboard is checked on every screen that follows (R-390's ORDER item 2; applied per
  R-369, review 5434766412 on PR #157). `mock_keyboard`'s registration API is built for that.
- Calibrations (R-71) proposed here: REQ-GUI-146 and REQ-GUI-158. Each is confirmed by the human at the M8 gate; an
  unconfirmed one blocks that gate.
- Sources and silences as TASK-M6-24's Notes give them (R-390).
- RQ-249 and RQ-250, decided per R-369 (7 Oct 2026): TASK-M6-24 leaves the breadcrumb's slot empty and Help's
  "Keys (?)" disabled; this task fills the slot and enables the entry, which opens the `?` shortcuts. The gui
  reviewer's § 8 keyboard item applies from this task on: it puts TASK-M6-24's top bar (its menus, the mode switch,
  Overlays ▾, Run…, Profiler…, Export…) into the scope tree as scope 1, and `mock_keyboard` covers them.
- R-404 (10 Oct 2026): the human confirmed this task's look choices (the ring, the breadcrumb, the `?` overlay with no
  dimming and its clicks-only pointer capture, the last a reading of "this is good", applied per R-369) and its two calibrations,
  REQ-GUI-146 and REQ-GUI-158, as built, ahead of the M8 gate. TASK-M6-30 rewords the code's "proposed" and "flagged"
  comments to "confirmed by R-404".
- R-405 (10 Oct 2026): the footer becomes big scope 8 and the console a window opened from it (REQ-GUI-178, TASK-M6-28), so
  REQ-GUI-095 now reads eight scopes. This task's acceptance line for it, seven, is its record as merged.
- R-409 (10 Oct 2026): the keyboard gains two modes, navigation (orange ring) and interaction (blue ring); the
  `browsing` model this task built (arrows adjusting a value reached by Enter on its section) is replaced, and R-404's
  `Orbit` step retired. REQ-GUI-096, REQ-GUI-098, REQ-GUI-158 and REQ-GUI-169 are reworded; this task's acceptance lines
  stay as its record as merged. TASK-M6-31 builds the modes and re-checks REQ-GUI-098 and REQ-GUI-169 as now worded.
