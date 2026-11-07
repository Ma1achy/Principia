# TASK-M8-118 — Hold L_z on any chart, beside energy normalisation

- **Milestone:** M8
- **Closes:** REQ-CHART-102, REQ-CHART-111, REQ-CHART-126
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-37, TASK-M2-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The new constraint, the twin of `η_E`: a uniform, off by default and bitwise today's when off, refused where `L_z` is an axis, with the joint form when both are held.

## References
- `docs/design/principia_chart_composition.md` § "5.3 Constraints: hold `E`, and new: hold `L_z`"
- `docs/design/principia_chart_composition.md` § "5.4 What the constraints do to a momentum axis"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"

## Deliverables
- `crates/kernel/src/decode/energy_norm.rs`, or its neighbour: the projection.
- `crates/kernel/src/stage/decode.rs`: the uniform's switch and value.
- `crates/engine/src/chart_validate.rs`: `forbids_lz_hold`, its refusals.

## Acceptance tests
- `cargo test -p kernel hold_lz`, `cargo test -p kernel hold_outcomes`, `cargo test -p engine hold_encode` and `cargo test -p engine constraint_axis_refused` — L_z = L*, Σp = 0, both held, the near-rigid fixture, the six outcomes in order, with at f32 the state the second projection leaves at K̂ = 0, above the floor and on it, encode on the held decode; under RQ-715's option (i) the outcome tags bit-identical at f32 and f64, Metal and lavapipe beside the CPU, on identical (m, r, v) across a fixture grid on both sides of τ, that state and exactly rigid draws included, no NaN on any arm; under (ii) rigid draws exempt from test 12's tag equality, each float's outcome finite; off bitwise; each refusal row by name, Some(0) included; controls: η_E alone fails L_z, one projection fails the fixture, outcomes 2 and 3 swapped, under (i) a tag from s = 0 and K̂ = 0 differs on the rigid draws, under (ii) the K̂ = 0 test removed gives NaN, the flag cleared fails (REQ-CHART-102).
- `cargo test -p kernel hold_rank` — a plane containing Jr, one through rest under hold E alone, and one through rest and one through a rigid rotation with both held report ray-degenerate, a generic plane doesn't; controls: the label removed, and a plane through a rigid rotation under hold E alone reporting rank loss (REQ-CHART-111).
- `cargo xtask bench hold-rigid-tau` — under RQ-715's option (i), against a criterion written before measuring (R-649), the proposal shows τ at masses log-uniform to 10⁻³, on the two GPU backends, and the kernel routine's cost; a reviewer checks it, the human confirms it at the M8 gate; it waits for the human's Mac (REQ-CHART-126). Under option (ii), REQ-CHART-126 retires.

## Notes
- RQ-722 is for the rest charts. RQ-715 closes only the held-L_z half of RQ-701.
- Free charts take no tilt target on L*; a chart holding L_z by construction moves by value (R-720).
- Waits on its rulings item: RQ-715, RQ-722. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
