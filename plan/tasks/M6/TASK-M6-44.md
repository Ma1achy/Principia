# TASK-M6-44 — The console's height and maximise toggle, and its open and close within one frame, measured before and after (R-463)

- **Milestone:** M6
- **Closes:** REQ-GUI-215, REQ-GUI-216
- **Depends on:** TASK-M6-32, TASK-M6-43
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
On the mock engine, the console's height is dragged from its top edge and remembered with the layout, a maximise button between Clear and × and the console's shortcut M toggle it to the full height below the top bar, and its open and close each complete within one frame at 60 fps (R-463). The lag is measured before it is fixed, with where the time goes; the PR gives the frame times before and after. A pointer for the measurement, not a finding: while open, the console filters and lays out the whole log every frame (`crates/gui/src/windows/console.rs`, `shown()` and the row grid without `show_rows`), up to 10 000 entries, and the mock logs a line per `SetField`.

## References
- `decisions.md` § "R-463 — The console's height drags and a maximise toggle (button and key M) fills the height below the top bar; its open and close lag is measured first, and both are brought within one frame at 60 fps, with the frame times before and after *(amends R-450 as it lists the header bar's controls)*"
- `decisions.md` § "R-450 — The console has two scopes, its header bar and its log: opening it lands on the header bar, Enter goes in and ← → cross every header control, × included; Enter on the log scrolls it entry by entry, the whole log reachable *(amends R-405 and R-409 as they apply to the console's structure)*"
- `decisions.md` § "R-462 — Every panel boundary is a draggable splitter, the canvas taking the space left; each panel has a minimum size, a double-click resets a splitter, the layout is remembered and View › reset layout restores it; resizing the canvas re-frames the figure and never re-integrates it"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G5. Windows (`04_windows.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "12 Console"

## Deliverables
- The docked console's height splitter and its place in the layout's settings (R-463 K1); the maximise button and M (K2); the header bar's new order and breadcrumb (K3).
- A `--bench console-open-close` mode on the mock (or an equivalent harness) that records the opening and closing frames' times and their breakdown at the three log sizes (K4).
- The fix the measurement points to; the before and after tables in the PR.
- Test `mock_console_height`.

## Acceptance tests
- cargo test -p gui mock_console_height (pointer and keyboard scripts on the mock): a drag on the console's top edge sets its height, clamped at both ends, and the height survives a restart; a click on maximise, and M with the focus in the console, toggle the full height and back; M while typing in the search field types 'm'; → from Clear reaches maximise then ×; controls: a console of fixed height, and an M that types nothing outside the field, each fail (REQ-GUI-215).
- benchmark on the human's Mac (R-186): cargo run -p gui --features mock --release -- --bench console-open-close at the three log sizes, before and after, the frame times of the opening and closing frames and their breakdown in the PR; after the fix each is under 16.7 ms at every size; review checklist (perf reviewer): the breakdown names where the time went before the fix, the fix addresses it, and the after-table meets the target (REQ-GUI-216).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: open and close the console with a long log and see it snap; drag its top edge; press M with the focus in it to maximise and again to restore.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **Benchmarks run on the human's Mac** (R-186): the orchestrator runs the before and after measurement there and the PR carries both tables; the perf reviewer checks the breakdown and the target.
- **qa's files (R-290, R-463 K6).** A qa assertion of the header bar with × right after Clear asserts what K3 changes; this task's qa reviewer changes it, and only it.
- **Early start (R-388).** It may start off TASK-M6-43's approved head once TASK-M6-32 has merged.
