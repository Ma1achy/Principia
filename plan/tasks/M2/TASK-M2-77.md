# TASK-M2-77 — Paper frames as presets

- **Milestone:** M2
- **Closes:** REQ-CHART-122, REQ-CHART-186
- **Depends on:** TASK-M2-58, TASK-M2-75, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Each paper's frame as a preset over the registry, tested against its coordinates.

## References
- `docs/design/principia_chart_composition.md` § "20. Paper frames as presets (order part E)"

## Deliverables
- `crates/engine/src/chart_authoring/presets.rs`: the frames.
- `crates/engine/tests/preset_paper_coords.rs`.

## Acceptance tests
- `cargo test -p engine preset_paper_coords` — `forward(inverse(x)) = x` on each row, and each row and its control, mutation: a sign flipped in the inverse on one side only (REQ-CHART-122).
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p engine preset_paper_coords`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-186).

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- RQ-650 is outside TASK-M2-58's gate, so nothing in Depends on holds it: D₁₁'s preset doesn't start until RQ-650 is ruled, by this note. If RQ-729 rules for the presets, TASK-M9-20, -22 and -28 take them, by edges TASK-M2-58 applies then; until it does, those tasks' own functions stand.
- Each preset's base point, centre and window (R-466's definition of a preset) are undefined on these pairs, which aren't two latent coordinate axes: RQ-711, read at TASK-M2-58, or a task that defines them under R-72 as TASK-M9-45 does (design § 20). Only those home values wait: the pairs, the masks and `preset_paper_coords` do not.
- Each paper-frame preset declares the filter on, as TASK-M8-120 reads it (design § 10.4; RQ-736).
- Waits on its rulings item: RQ-729. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *8 Nov 2026 (R-873):* **Build order:** stage 3 of R-863's order, after stage 2 (TASK-M2-65), placed by R-873 (RQ-744 (A) (a) and (C)); TASK-M2-81 and TASK-M8-108, stage 4, depend on it. The note above that no stage edge is made is overtaken. *Was:* "- **Depends on:** TASK-M2-58, TASK-M2-75"
- *8 Nov 2026 (R-884):* RQ-729 is ruled (a): presets where the papers are read, each tested against its paper; each remaining gap stays unbuilt, its target "not scored" with the gap named. TASK-M2-58 applies its edges (TASK-M9-20, -22 and -28 gain this task). R-888 (RQ-711): a saved non-latent preset's zoom row reads 1× at its saved view.
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-112; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Closes: REQ-CHART-122".
- *9 Nov 2026 (R-916):* REQ-CHART-186, the round-trip tolerance § 12 gives only as measured worsts, is this task's calibration, its criterion written first, set by measurement (R-649/650), confirmed at the M2 gate (G5).
