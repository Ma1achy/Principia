# TASK-M8-13 — Keyboard scopes, global and in-scope keys, focus ring and breadcrumb (07_keyboard.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-095, REQ-GUI-096, REQ-GUI-097, REQ-GUI-098, REQ-GUI-146
- **Depends on:** TASK-M8-07, TASK-M8-11, TASK-M8-12, TASK-M8-03
- **Needs (earlier milestones):** REQ-GUI-002, REQ-GUI-003
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The GUI is a tree of keyboard scopes with Tab order 1 top bar · 2 Manifold view · 3 Figure · 4 Trajectory · 5 Compass · 6 Time · 7 Legend, and Manifold view's sub-scopes reached with Enter. The global keys follow §G3's table (Tab / Shift+Tab, Enter, Esc, arrows, Shift ×10 / Alt ×0.1, held-key delay-then-repeat with calibrated DAS / ARR, Ctrl+Z from the contract's history, ? over everything). The in-scope keys are Figure (arrows pan, + / − zoom, Space keeps, L listens, K locks), Trajectory, Compass, Time; Legend is read-only. Focus shows only as a ring and the top-bar breadcrumb; the scope lives in ViewUI.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "07 Keyboard — design note, not a screen"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/gui/src/keyboard/{scopes,keymap,repeat}.rs` — the scope tree, the key tables, DAS / ARR repeat.
- `crates/gui/src/explore/breadcrumb.rs` — filled from ViewUI's focus scope.
- Tests: `scope_tab_order`, `global_keys`, `in_scope_keys`; screenshot cases `07_keyboard/before`, `07_keyboard/after`.
- Calibration proposal: the repeat delay and rate, with the reference they follow.

## Acceptance tests
- `cargo test -p gui scope_tab_order` — Tab / Shift+Tab cycles the seven scopes in order; Enter on Manifold view enters Chart (REQ-GUI-095).
- `cargo test -p gui global_keys` — synthetic key events for each row; a held arrow repeats only after the delay; Ctrl+Z calls the contract's undo (REQ-GUI-096).
- `cargo test -p gui in_scope_keys` — synthetic key events per scope produce the stated SetField / ViewUI change; keys in Legend change nothing (REQ-GUI-097).
- `cargo xtask screenshot 07_keyboard` — screenshots before and after moving focus differ only in the ring and the breadcrumb (07_keyboard.png) (REQ-GUI-098).
- Review checklist (gui reviewer) of the DAS / ARR proposal — the proposal gives the delay and rate with the reference they follow (e.g. platform defaults); a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-146).

## Notes
- The base step each arrow applies per field (before Shift ×10 / Alt ×0.1) is not given (raised as a gap for a REVIEW_QUEUE entry).
- Calibrations (R-71) proposed here: REQ-GUI-146. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
