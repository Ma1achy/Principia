# TASK-M8-131 — Marking clamped and saturated areas: a toggle, off by default (R-939)

- **Milestone:** M8
- **Closes:** REQ-GUI-433
- **Depends on:** TASK-M2-102, TASK-M2-103, TASK-M2-105
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** gui; small

## Goal
An Overlays › Chart toggle that marks the pixels the edge kinds class clamped or saturated, off by default (R-939, RQ-774 (b)).

## References
- `decisions.md` § "R-939 — RQ-774, marking clamped and saturated areas: a toggle, off by default *(closes RQ-774)*"
- `decisions.md` § "R-932 — Chart edges: no blank areas; past every true edge the chart saturates, where there is a second sheet it unfolds onto it, and where the coordinate repeats it wraps; smearing and saturating are the same idea, and which mechanism an edge uses is physics review's call, saturating wherever the edge system itself is not a valid system; RQ-773 option (a): unfold, clamp and saturate become declared kinds per edge, and an unfold sheet is counted, a clamped pixel is not, a saturated pixel only where its system is not shown inside the square *(closes RQ-773; amends R-407, R-470, R-471, R-472, R-503, R-506, R-508, R-512, R-537, R-554, R-597 and R-708; refines R-408)*"
- `decisions.md` § "R-470 — A pixel that is not a system draws blank, the view's background, in every view, the outcome view included, never a palette colour; the hatch survives only as the optional Overlays › Chart › forbidden region, off by default, over blank pixels only; NaN, ±∞, `sim_failed`, `decode_failed` and the classification behind the hatch show only in the debug views; R-408's statistics are unchanged *(amends R-407, R-446 and R-467; closes RQ-374)*"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"

## Deliverables
- The toggle in `crates/gui`, off by default, drawn over the field as R-470's forbidden-region overlay is.
- **A screenshot pair for the owner, dim and hatch:** which of the two marks the area isn't ruled (R-939); the task shows the owner both and they choose. Neither is chosen in the PR.

## Acceptance tests
- `cargo test -p gui mark_clamped_saturated` — an Overlays › Chart toggle, off by default, dimming or hatching exactly the pixels TASK-M2-102, TASK-M2-103 and TASK-M2-105 class clamped or saturated, drawn over the field as R-470's forbidden-region overlay is; control: a toggle that also dims a system pixel inside the square fails; the dim and the hatch shown to the owner as a screenshot pair, their choice recorded. No measurement (REQ-GUI-433).

## Notes
- Entered 11 Nov 2026 as N18 of the re-plan's addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`) (applied per R-369: task scoping); its acceptance is the addendum's § C item 7 and its answers section's N18 bullet (R-939: dim or hatch shown to the owner as a pair). The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold, and it is visual work, which waits for M2's gate as well (R-917).
- The plan names no milestone for it beyond "after N14 and M2's gate": it is entered in M8 (applied per R-369: task scoping, listed for veto).
