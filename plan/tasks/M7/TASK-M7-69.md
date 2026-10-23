# TASK-M7-69 — `device_px` and the screen-anchored sheet at render_scale ≠ 1 and at export, as RQ-538 rules (R-490)

- **Milestone:** M7
- **Closes:** REQ-RENDER-110
- **Depends on:** TASK-M7-55, TASK-M7-60
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
Whichever pixels RQ-538 rules `device_px` lengths and the screen anchoring are counted in, while the Print graph runs before the display scale at render resolution: the CRT, LCD and LED matrix nodes and a sheet-only graph keep their pitch and anchoring at render_scale 0.5, 1 and 2, and at export. Waits on RQ-538; a leaf, so nothing else waits.

## References
- `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size"
- `docs/design/principia_print_graph.md` § "6.3 Anchoring and stability under motion and playback"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the one above)"
- `decisions.md` § "R-490 — The Print graph is deterministic, seeded only; every length is in `ref_px`, `device_px` or `fraction`; the sheet is anchored to the screen; `boil` is off by default"
- `decisions.md` § "R-557 — A Print graph field input takes the stain's window when the stain colours by the same field, else the field's own `range_norm` default; the print stage's time is recorded but never moves the Auto rung, and its post level is the arbiter's motion-regime lever, under Auto only; the human approves PR #268 with its follow-ups *(closes RQ-478, RQ-479 and RQ-495; amends R-488)*"

## Deliverables
- `crates/render/src/print/units.rs` — the `device_px` and anchoring rule RQ-538 rules.
- Test `print_device_px_scale`.

## Acceptance tests
- `cargo test -p render print_device_px_scale` — REQ-RENDER-110's verify (the CRT, LCD and LED matrix nodes and a sheet-only graph rendered at render_scale 0.5, 1 and 2, displayed, agree in pitch and anchoring as RQ-538's ruling says; control: a node counting `device_px` in the wrong pixels fails the pitch arm at render_scale 0.5).

## Notes
- Waits on RQ-538 (REQ-RENDER-110 carries it).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
