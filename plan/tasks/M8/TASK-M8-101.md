# TASK-M8-101 — Curved slices off the shape sphere: the momentum and mass spheres

- **Milestone:** M8
- **Closes:** REQ-CHART-105, REQ-CHART-106
- **Depends on:** TASK-M8-100
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
- `cargo test -p kernel exp_map_targets` — L_z and K hold on the momentum sphere, Σm = 1 on the mass sphere, the shape target bitwise; control: a bad frame (REQ-CHART-106).
