# TASK-M8-13 — Keyboard scopes, global and in-scope keys, focus ring and breadcrumb (07_keyboard.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-096, REQ-GUI-097
- **Depends on:** TASK-M8-07, TASK-M8-11, TASK-M8-12, TASK-M8-03, TASK-M6-25
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
- `crates/gui/src/keyboard/{scopes,keymap,repeat}.rs` — uses and extends the track's scope tree, key tables and DAS / ARR repeat (TASK-M6-25, R-390): wires Ctrl+Z to the contract's undo and the in-scope keys to the real engine's SetField / ViewUI changes.
- `crates/gui/src/explore/breadcrumb.rs` — filled from ViewUI's focus scope.
- Tests: `scope_tab_order`, `global_keys`, `in_scope_keys`; screenshot cases `07_keyboard/before`, `07_keyboard/after`.
- Uses the track's calibration proposal for the repeat delay and rate, with the reference they follow (TASK-M6-25, R-390); the human confirms it at the M8 gate.

## Acceptance tests
- `cargo test -p gui scope_tab_order` — Tab / Shift+Tab cycles the seven scopes in order; Enter on Manifold view enters Chart (REQ-GUI-095). Closed by TASK-M6-25 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo test -p gui global_keys` — synthetic key events for each row; a held arrow repeats only after the delay; Ctrl+Z calls the contract's undo (REQ-GUI-096).
- `cargo test -p gui in_scope_keys` — synthetic key events per scope produce the stated SetField / ViewUI change; keys in Legend change nothing (REQ-GUI-097).
- `cargo xtask screenshot 07_keyboard` — screenshots before and after moving focus differ only in the ring and the breadcrumb (07_keyboard.png) (REQ-GUI-098). Closed by TASK-M6-25 on the mock engine since R-390; this task re-runs it on the real engine.
- Review checklist (gui reviewer) of the DAS / ARR proposal — the proposal gives the delay and rate with the reference they follow (e.g. platform defaults); a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-146). Proposed by TASK-M6-25 since R-390, the first task to need the value; this task uses it.
- Proposal: the base step per adjustable field kind and per in-scope arrow action; the human confirms it at the M8 gate (REQ-GUI-158). Proposed by TASK-M6-25 since R-390, the first task to need the value; this task uses it.

## Notes
- The base step each arrow applies per field (before Shift ×10 / Alt ×0.1) is not given.
- Calibrations (R-71) used here: REQ-GUI-146, proposed by TASK-M6-25 since R-390. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- For gaps the corpus leaves open: REQ-GUI-158 (R-71 calibration) (classification accepted by R-132), closed by TASK-M6-25 since R-390 and used here.
- R-390: TASK-M6-25 builds the scope tree, the global keys, the focus ring, the breadcrumb and `?` on the mock engine, closes REQ-GUI-095 and REQ-GUI-098, and proposes REQ-GUI-146 and REQ-GUI-158; this task depends on it, re-runs REQ-GUI-095's and REQ-GUI-098's acceptance on the real engine, uses the proposed values, and keeps REQ-GUI-096 and REQ-GUI-097.
