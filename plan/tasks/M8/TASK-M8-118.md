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
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"

## Deliverables
- `crates/kernel/src/decode/energy_norm.rs`, or its neighbour: the projection.
- `crates/kernel/src/stage/decode.rs`: the uniform's switch and value.
- `crates/engine/src/chart_validate.rs`: `forbids_lz_hold`, its refusals.
- `crates/engine/src/lookup.rs`: encode under a hold, Part 6's step 4 on the held decode, and the stated outcome for a state off `{L_z = L*}`. This task is that code's only home: TASK-M4-32 closes REQ-CHART-110 as a document change.
- Under RQ-715's option (i): `kernel::decode::invariant::rigid`, the bench `hold-rigid-tau`, and the calibration proposal for `τ` (REQ-CHART-126).
- Under RQ-715's option (i): `crates/engine/tests/hold_cross_backend.rs`, package `engine`, with the tests `hold_outcomes_cross_backend`, `hold_outcomes`' GPU arm, and `hold_rigid_tau_tags`, the bench's lavapipe leg; a step running that binary in both the `gpu-metal suites` and `gpu-lavapipe suites` jobs of `.github/workflows/ci.yml`, since those jobs run only the selectors named in their steps; and its entries in `.config/nextest.toml`, as `engine::decode_cross_backend` has.

## Acceptance tests
- `cargo test -p kernel hold_lz`, `cargo test -p kernel hold_outcomes`, `cargo test -p engine hold_encode` and `cargo test -p engine constraint_axis_refused` — L_z = L* to tolerance on every chart allowing it, Σp = 0, both held E* too, the near-rigid fixture, the six outcomes in order, with at f32 the state the second projection leaves at K̂ = 0, above the floor and on it; Part 6's step 4 reads the held decode and a state off {L_z = L*} gets the stated outcome; under RQ-715's option (i) the outcome tags bit-identical at f32 and f64, Metal and lavapipe beside the CPU, on identical (m, r, v) across a fixture grid on both sides of τ, that state and exactly rigid draws included, no NaN on any arm; under (ii) rigid draws exempt from test 12's tag equality, each float's outcome finite; off bitwise; each refusal row by name, Some(0) included; controls: η_E alone with both on fails L_z, one projection fails the fixture, outcomes 2 and 3 swapped, under (i) a tag from s = 0 and K̂ = 0 differs on the rigid draws, under (ii) the K̂ = 0 test removed gives NaN, step 4 on the unheld decode lands on a wrong pixel, the flag cleared on (L_z, E) fails (REQ-CHART-102). The Metal and lavapipe arm of hold_outcomes is `cargo nextest run -p engine --test hold_cross_backend hold_outcomes_cross_backend`, run by the step this task adds to CI's `gpu-metal suites` and `gpu-lavapipe suites` jobs.
- `cargo test -p kernel hold_rank` — a plane containing Jr, one through rest under hold E alone, and one through rest and one through a rigid rotation with both held report ray-degenerate, a generic plane doesn't; controls: the label removed, and a plane through a rigid rotation under hold E alone reporting rank loss (REQ-CHART-111).
- `cargo xtask bench hold-rigid-tau` — under RQ-715's option (i), against a criterion written before measuring (R-649), the proposal shows τ at masses log-uniform to 10⁻³, on the two GPU backends, and the kernel routine's cost. The Metal leg and the cost run on the human's Mac and wait for that run (R-186); the lavapipe leg is a tag measurement, no timing, and runs as `hold_rigid_tau_tags` in that same step of CI's `gpu-lavapipe suites` job (`PRIN_GPU_BACKEND=vulkan`, R-169), where the plan sets lavapipe up. A reviewer checks it, the human confirms it at the M8 gate (REQ-CHART-126). Under option (ii), REQ-CHART-126 retires.

## Notes
- RQ-722 is for the rest charts. RQ-715 closes only the held-L_z half of RQ-701.
- Free charts take no tilt target on L*; a chart holding L_z by construction moves by value (R-720).
- Waits on its rulings item: RQ-715, RQ-722. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
