# TASK-M3-37 — Cross-chart consistency: the same state through the shape sphere and the latent chart gives the same z, payload and outcome (R-477 C1) ⚑

- **Milestone:** M3
- **Closes:** REQ-CHART-072
- **Depends on:** TASK-M3-19, TASK-M3-22, TASK-M2-15
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines, the tests included

## Goal
C1: reach one state through two charts, a shape-sphere pixel and the latent pixel its decoded state encodes to, and check the same `z` (to ε_phys) and then the same payload and outcome bit for bit; a mismatched pair differs.

## References
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"

## Deliverables
- Test `cross_chart_consistency`.

## Acceptance tests
- `cargo test -p kernel cross_chart_consistency` — REQ-CHART-072's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **M3**: the outcome needs the integrator (TASK-M3-19, TASK-M3-22).
