# TASK-M8-130 — The figure draws the computed axes: the dual ruler, the edge marks, mixed names (R-940, R-942)

- **Milestone:** M8
- **Closes:** REQ-GUI-432
- **Depends on:** TASK-M2-98, TASK-M8-06, TASK-M8-50
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** gui; screenshots

## Goal
The figure's axes are TASK-M2-98's computed axes, drawn by the line system: it changes TASK-M8-06 (the figure's axis labels) and TASK-M8-50 (Lines ▾, the axes and presets) rather than adding a parallel axis path. In a blend a dual ruler, one axis line with the starting chart's real ticks on one side and the target chart's on the other, each fading with its share; on a tilt or rotation within one chart, one number in window units; never a summed number, every true value on hover (R-940). The computed axis is the primary scale on every chart and frame, and the no-ticks rule mid-morph is withdrawn (R-942).

## References
- `decisions.md` § "R-940 — RQ-777, mixed axis numbers: not the summed number; in a blend a dual ruler, one axis line with the starting chart's real ticks on one side and the target chart's on the other, each fading with its share; on a tilt or rotation within one chart, one number in window units; hover gives every true value *(closes RQ-777)*"
- `decisions.md` § "R-942 — The chart addendum's remaining questions, the plan's recommendation on each: the per-edge choice lives in the registry row and, as an override, in the view state, a re-addressing kind off the sim key and a kind that shows new systems on it (RQ-775); the backwards table holds sheet 1 only, a state looked up as itself and as its time reverse (RQ-776); no small term is dropped from an axis's name (RQ-778); the computed axis is the primary scale on every chart and every frame, the no-ticks rule mid-morph withdrawn (RQ-780); the view's centre may cross onto an unfold sheet and stops at a clamped or saturated area (RQ-781); RQ-778 and RQ-780 read with the dual ruler *(closes RQ-775, RQ-776, RQ-778, RQ-780 and RQ-781; amends R-472; refines R-469, R-476 and R-926)*"
- `decisions.md` § "R-944 — The principle: every pixel of a morph frame is a valid system, so a morph frame is a chart, and every feature that works on a chart works on a morph frame: traces, contours, marks, hover, tilt, slice, save; where the design says a mid-morph frame can't do something, that is a gap to close, not a limitation to label; the one accepted exception is area statistics on a mid-morph frame *(no RQ; amends R-899 and R-902; refines R-925)*"
- `decisions.md` § "R-466 — Slice centre, Tilt and Navigate each have a "reset" beside the heading that returns the section to the active chart preset's own values, never literal zero; Home does all three in one command, from a button in the canvas's corner, the Home key in both keyboard modes and View › Home; each is one undoable command, dimmed when already home, and drives the compass live *(extends R-409, R-457, R-458 and R-462)*"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_lines.md` § "8b. Axes"
- `docs/design/principia_chart_composition.md` § "17.5 A frame: valid, reproducible, recorded"
- `docs/design/principia_chart_composition.md` § "17.11 Honesty"

## Deliverables
- In `crates/gui`: the line system's primary scale on each side reads TASK-M2-98's axis; the Ends only preset reads it.
- The marks: a taller tick at a seam and at a fold, constant numbers on a clamp run, a thicker axis line on a saturated stretch, each term's marks fainter by its coefficient, every length in base px (R-583).
- A `gauge` axis's paper scale on its own end's side, fading with its share (the addition's § B, B4).

## Acceptance tests
- `cargo test -p gui computed_axes_drawn` — the line system's primary scale on each side is the computed axis; a mixed name drawn as it gives it; a taller tick at a seam and at a fold, constant numbers on a clamp run, a thicker axis line on a saturated stretch, each term's marks fainter by its coefficient, every length in base px (R-583); the Ends only preset reads the computed axis; no domain words on any axis; in a blend the dual ruler, a `gauge` end's paper scale on its own side fading with its share. Screenshot tests, each with a control: the latent chart at home, at ×1000, at a 30° rotation, at a 40° tilt with a rotation; `InvariantLE` with its spin edges unfolded; a morph paused part-way; a long mixed name never overlaps the Home button's rect (R-466; control: the name drawn untruncated does). No measurement (REQ-GUI-432).

## Notes
- Entered 11 Nov 2026 as N17 of the re-plan's addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`) (applied per R-369: task scoping); its acceptance is the addendum's § C item 6; the addition's § B, B4 and B5. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold, and it is visual work, which waits for M2's gate as well (R-917).
- Both † edges are kept (applied per R-369: task scoping): the acceptance draws on TASK-M8-06's axis labels and Home button and on TASK-M8-50's line-system axes and Ends only preset. The plans name no milestone for it beyond "after M2's gate": it is entered in M8 (applied per R-369: task scoping, listed for veto).
- REQ-GUI-084 (axis labels) and REQ-GUI-246 (the figure's axes) are brought into line with R-940 and R-942 and stay with TASK-M8-06 and TASK-M8-50.
