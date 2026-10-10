# TASK-M8-13 — Keyboard scopes, global and in-scope keys, focus ring and breadcrumb (07_keyboard.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-096, REQ-GUI-097
- **Depends on:** TASK-M8-07, TASK-M8-11, TASK-M8-12, TASK-M8-03, TASK-M6-25, TASK-M6-28
- **Needs (earlier milestones):** REQ-GUI-002, REQ-GUI-003
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The GUI is a tree of keyboard scopes with Tab order 1 top bar · 2 Manifold view · 3 Figure · 4 Trajectory · 5 Compass · 6 Time · 7 Legend · 8 footer (R-405), and Manifold view's sub-scopes reached with Enter. The global keys follow §G3's table in its two modes, navigation and interaction (R-409) (Tab / Shift+Tab, Enter, Esc, arrows, Shift ×10 / Alt ×0.1, held-key delay-then-repeat with calibrated DAS / ARR, Ctrl+Z from the contract's history, ? over everything). The in-scope keys are Figure (arrows pan, + / − zoom, Space keeps, L listens, K locks), Trajectory, Compass (in interaction mode, arrows tilt or move the slice by its mode, Shift ×10 and Alt ×0.1 as everywhere, with no orbit; its Tilt and Slice buttons reached in navigation mode; R-409), Time (Space plays; ← → step on the scrubber); Legend is read-only. A scope's arrow actions act in interaction mode only and its letter and Space shortcuts in either mode, and a Tab lands on the big scope itself (R-409's follow-up answers). Focus shows only as a ring, orange in navigation mode and blue in interaction mode, and the top-bar breadcrumb; the scope and its mode live in ViewUI.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "07 Keyboard — design note, not a screen"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"

## Deliverables
- `crates/gui/src/keyboard/{scopes,keymap,repeat}.rs` — uses and extends the track's scope tree, key tables and DAS / ARR repeat (TASK-M6-25, R-390): wires Ctrl+Z to the contract's undo and the in-scope keys to the real engine's SetField / ViewUI changes.
- `crates/gui/src/explore/breadcrumb.rs` — filled from ViewUI's focus scope.
- Tests: `scope_tab_order`, `global_keys`, `in_scope_keys`; screenshot cases `07_keyboard/before`, `07_keyboard/after`.
- Uses the track's calibration proposal for the repeat delay and rate, with the reference they follow (TASK-M6-25, R-390); the human confirms it at the M8 gate.

## Acceptance tests
- `cargo test -p gui scope_tab_order` — Tab / Shift+Tab cycles the eight scopes in order, the footer last (R-405); Enter on Manifold view enters Chart (REQ-GUI-095). Closed by TASK-M6-25 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo test -p gui global_keys` — synthetic key events for each row in each mode: a Tab landing is on the big scope itself, not its first element, in navigation mode, and Enter into a scope lands in navigation mode on its first element; in navigation mode the arrows move between siblings and adjust nothing; Enter on a value starts interaction mode and the arrows then adjust it, ×10 with Shift and ×0.1 with Alt; Esc returns to navigation mode on the same value, and a second Esc goes up one scope; a scope's letter shortcut acts in both modes; a held arrow repeats only after the delay; Ctrl+Z calls the contract's undo (REQ-GUI-096).
- `cargo test -p gui in_scope_keys` — synthetic key events per scope produce the stated SetField / ViewUI change, in each mode: in navigation mode the arrows move between siblings and no arrow pans the Figure, steps Time or tilts or moves the compass, while + / −, Space, L and K on the Figure and Space on Time act; in interaction mode the arrows pan, step (on Time's scrubber) and tilt or move the slice; on the compass, in each of its modes, the arrows in interaction mode ×10 with Shift and ×0.1 with Alt, and Enter on Tilt and on Slice in navigation mode; no key orbits the compass; keys in Legend change nothing (REQ-GUI-097).
- `cargo xtask screenshot 07_keyboard` — screenshots before and after moving focus differ only in the ring and the breadcrumb, and before and after Enter on a value only in the ring's colour (and the breadcrumb's, per R-409 A6) (07_keyboard.png, read with R-409) (REQ-GUI-098). Closed by TASK-M6-25 on the mock engine since R-390; this task re-runs it on the real engine.
- Review checklist (gui reviewer) of the DAS / ARR proposal — the proposal gives the delay and rate with the reference they follow (e.g. platform defaults); a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-146). Proposed by TASK-M6-25 since R-390, the first task to need the value; this task uses it.
- Proposal: the base step per adjustable field kind and per in-scope arrow action; the human confirms it at the M8 gate (REQ-GUI-158). Proposed by TASK-M6-25 since R-390, the first task to need the value; this task uses it.

## Notes
- The base step each arrow applies per field (before Shift ×10 / Alt ×0.1) is not given.
- Calibrations (R-71) used here: REQ-GUI-146, proposed by TASK-M6-25 since R-390. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- For gaps the corpus leaves open: REQ-GUI-158 (R-71 calibration) (classification accepted by R-132), closed by TASK-M6-25 since R-390 and used here.
- R-390: TASK-M6-25 builds the scope tree, the global keys, the focus ring, the breadcrumb and `?` on the mock engine, closes REQ-GUI-095 and REQ-GUI-098, and proposes REQ-GUI-146 and REQ-GUI-158; this task depends on it, re-runs REQ-GUI-095's and REQ-GUI-098's acceptance on the real engine, uses the proposed values, and keeps REQ-GUI-096 and REQ-GUI-097.
- R-404 (10 Oct 2026): the human confirmed REQ-GUI-146 (500 ms delay, 40 ms interval) and REQ-GUI-158 (the base steps)
  as TASK-M6-25 proposed them, ahead of the M8 gate. This task uses the confirmed values; its two proposal lines have
  nothing left to put to the gate, and the gate lists both as confirmed by R-404.
- R-405 (10 Oct 2026): the footer is big scope 8, after Legend, and the console a window opened from it (REQ-GUI-178,
  built by TASK-M6-28 on the mock, which this task depends on); REQ-GUI-095 reads eight scopes, and this task re-runs
  the eight on the real engine.
- R-409 (10 Oct 2026): the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), every
  landing in navigation mode, Enter starting interaction and Esc returning to navigation and then going up a scope; the
  compass has no orbit, and R-404's `Orbit` step is retired. By the human's follow-up answers (R-409 F1, F3, F4), the
  arrows need Enter and letter and Space shortcuts act in either mode, Shift ×10 and Alt ×0.1 hold on the compass too
  (no separate fine step), and a Tab lands on the big scope itself. TASK-M6-31 builds them on the mock (REQ-GUI-182,
  REQ-GUI-183), and this task reaches it through TASK-M6-28. REQ-GUI-096, REQ-GUI-097 and
  REQ-GUI-098 are reworded; this task's lines for them read as now worded. The base step of the compass's slice-mode
  arrows is REQ-GUI-184, proposed by TASK-M6-31 and confirmed at the M8 gate.
