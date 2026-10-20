# TASK-M2-39 — T4's units for a tilt mixing a chart parameter with a hidden coordinate, as RQ-375 rules (R-473) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-064
- **Depends on:** TASK-M2-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
Apply RQ-375's ruling on the common scale for a tilt between a chart parameter and a hidden coordinate (the human's proposal: each screen axis in half-widths of the window, the hidden coordinate in its slider span).

## References
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"

## Deliverables
- The scale in the tilt's params, per RQ-375.
- Test `tilt_units`.

## Acceptance tests
- `cargo test -p kernel tilt_units` — REQ-CHART-064's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **A leaf task waiting for RQ-375**: nothing depends on it; TASK-M2-36 builds the meanwhile reading.
