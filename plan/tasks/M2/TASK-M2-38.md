# TASK-M2-38 — Navigation stops at the edge: one generic stop over the Chart trait and validate, and the fuzz that never leaves the manifold (R-469, R-477 C6) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-060, REQ-CHART-061
- **Depends on:** TASK-M2-36, TASK-M2-37, TASK-M2-22, TASK-M2-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
One kernel function decides whether a navigation edit keeps the chart's centre valid (`validate`'s layers 2 and 3) and, if not, the last valid value along the edit's path by bisection to adjacent f64 values, with the layer and constraint that stopped it (R-469 P2, A1 to A3). No chart writes its own. A seeded fuzz of navigation edits on every gallery chart never leaves the manifold (C6).

## References
- `decisions.md` § "R-469 — A chart never leaves the manifold: every pixel it draws is a valid three-body system or explicitly "not a system", and pan, slice, zoom, tilt and lock are accepted only while the chart's centre passes `validate`, an edit that would cross stopping at the last valid value, by one generic rule in the edit path; undo and the contract record the clamped value, the slider stops and the readout names the check"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "The operation table"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"

## Deliverables
- `nav_stop` over the `Chart` trait and `validate` (kernel), returning the clamped value and its report.
- Tests `nav_stop_at_edge` and the seeded property test `nav_fuzz_never_leaves`.

## Acceptance tests
- `cargo test -p kernel nav_stop_at_edge` — REQ-CHART-060's verify.
- `cargo test -p kernel nav_fuzz_never_leaves` — REQ-CHART-061's verify (C6, its linear-option arm).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **The edit path** on the real engine is TASK-M8-48 (REQ-GUI-243); on the mock nothing stops (R-469 A6).
- **Physics review 5586890917 of PR #255:** the bisection halves the interval between the ends' f64 bit patterns, at most 62 steps (F4; R-469 A3, REQ-CHART-060); C6's every-pixel arm covers the pixels whose `(a, b)` lie in the chart's square (F1; R-471 H9, REQ-CHART-061; R-512, ruling RQ-402, reads C6 so, and the rest are past the edge, TASK-M2-39). *Was:* "(F1; R-471 H9, REQ-CHART-061; the rest is RQ-402, TASK-M2-39)."
- **R-633 (28 Oct 2026, RQ-603):** TASK-M2-36 is split, and TASK-M2-52 builds tilt and slice on the six construction charts. This task depends on it too: REQ-CHART-060's slice on the mass simplex and REQ-CHART-061's fuzz over every gallery chart, the Burrau `K` charts tilted, need them (applied per R-369). *Was:* "- **Depends on:** TASK-M2-36, TASK-M2-37, TASK-M2-22".
