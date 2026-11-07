# TASK-M6-55 — A least window size, and the layout's minimums re-measured (R-705)

- **Milestone:** M6
- **Closes:** REQ-GUI-414
- **Depends on:** TASK-M6-35
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~120 lines

## Goal
R-705 takes RQ-662's options (b) and (c). The default split puts the bottom row under its minimum at 1280×720, 157.3 against 160, and no least window is set; and at the compass's least size the Plücker readout was seen to pass the panel. A least window size makes the default layout meet the minimums, and the two constants are measured again.

## References
- `decisions.md` § "R-705 — REQ-PERF-115 and TASK-M8-96 name `--release`; a least window size is set, and the layout's minimums are re-measured; a typed basis's off-frame components are counted and zoom scales the stored basis *(closes RQ-657, RQ-662 and RQ-663)*"
- `decisions.md` § "R-462 — Every panel boundary is a draggable splitter, the canvas taking the space left; each panel has a minimum size, a double-click resets a splitter, the layout is remembered and View › reset layout restores it; resizing the canvas re-frames the figure and never re-integrates it"

## Deliverables
- `crates/gui/src/cli.rs`: `with_min_inner_size` at the least size, derived from the layout's constants and the default row's 295/1350 of the height, every panel minimum considered, not only the bottom row's, which alone gives about 732 high.
- `crates/gui/src/layout.rs`: `MIN_COMPASS_W` and `MIN_BOTTOM_H` re-measured against the widest readout, each doc stating what was measured.
- Tests beside `Layout`.

## Acceptance tests
- `cargo test -p gui layout_least_window` — REQ-GUI-414's verify, with its control.
- Review checklist (gui reviewer) — a screenshot at the least size shows every panel whole (REQ-GUI-414).

## Notes
- Added by R-705 (4 Nov 2026, RQ-662 (b), (c)). `Layout::with_splits`'s behaviour is unchanged: the minimums still hold drags, RQ-662's (a) and (d) not being taken.
- A gui task runs its mutants under R-439's limits.
