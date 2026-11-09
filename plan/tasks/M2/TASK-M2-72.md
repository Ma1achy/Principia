# TASK-M2-72 — Curved slices off the shape sphere: the momentum and mass spheres

- **Milestone:** M2
- **Closes:** REQ-CHART-105, REQ-CHART-106, REQ-CHART-182
- **Depends on:** TASK-M2-58, TASK-M2-71, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Define (R-72) and build the two other 2-spheres in the state, under § 3.3's exponential map, the shape sphere's bits unchanged.

## References
- `docs/design/principia_chart_composition.md` § "5.2 Curved slices off the shape sphere"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"

## Deliverables
- The chart reference's section.
- `ExpMap<Target>` in `crates/kernel/src/chart/`.

## Acceptance tests
- Physics reviewer: the section, its frames and seams (REQ-CHART-105).
- `cargo test -p kernel exp_map_targets` — L_z and K hold on the momentum sphere; on a wide mass chart every unblanked pixel has all components positive and √mᵢ = nᵢ, each outside pixel blank and uncounted, no two unblanked pixels giving one mass set; the shape target bitwise; `forward(inverse(x)) = x` on each target; controls: a bad frame, squaring with the mask removed, mutation: named per target (REQ-CHART-106).
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p kernel exp_map_targets`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-182).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. It depends on stage 2's task, TASK-M2-65, and merges only with its tests green.
- Waits on its rulings item: RQ-720. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-101; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Closes: REQ-CHART-105, REQ-CHART-106".
- *9 Nov 2026 (R-916):* REQ-CHART-182, the round-trip tolerance § 12 gives only as measured worsts, is this task's calibration, its criterion written first, set by measurement (R-649/650), confirmed at the M2 gate (G5).
