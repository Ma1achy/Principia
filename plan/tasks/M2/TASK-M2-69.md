# TASK-M2-69 — Hold L_z on any chart, on the CPU (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-102, REQ-CHART-111
- **Depends on:** TASK-M2-66, TASK-M2-37, TASK-M2-52, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
The new constraint, the twin of `η_E`, on the CPU: a uniform, off by default and bitwise today's when off, refused where `L_z` is an axis, with the joint form when both are held. `τ` stays provisional at M2 (R-919, F13 (b)); its GPU evidence and the GPU tags are TASK-M8-118's.

## References
- `docs/design/principia_chart_composition.md` § "5.3 Constraints: hold `E`, and new: hold `L_z`"
- `docs/design/principia_chart_composition.md` § "5.4 What the constraints do to a momentum axis"
- `docs/design/principia_chart_composition.md` § "5.5 Every case where two axes can't be independent"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"

## Deliverables
- `crates/kernel/src/decode/energy_norm.rs`, or its neighbour: the projection; the uniform's switch and value in `crates/kernel/src/stage/decode.rs`.
- `crates/engine/src/chart_validate.rs`: `forbids_lz_hold`, its refusals.
- Encode under a hold: `crates/kernel/src/encode/by_kind.rs` and `crates/engine/src/lookup.rs` reading the held decode, Part 6's step 4.

## Acceptance tests
- `cargo test -p kernel hold_lz`, `cargo test -p kernel hold_outcomes`, `cargo test -p engine hold_encode` and `cargo test -p engine constraint_axis_refused` — L_z = L* to tolerance on every chart allowing it, Σp = 0, both held E* too; off, bitwise; the outcomes in order, at f32 the state the second projection leaves at K̂ = 0, above the floor and on it; under RQ-715's option (i) the outcome tags bit-identical at f32 and f64 on the CPU (the GPU's, REQ-CHART-179), on identical (m, r, v) across a fixture grid on both sides of τ, that state and exactly rigid draws included, no NaN on any arm; under (ii) rigid draws exempt from test 12's tag equality, each float's outcome finite; inverse_encode_contract Part 6's step 4 reads the held decode and a state off {L_z = L*} gets the stated outcome; every row of the refusal table by name; controls: η_E alone with both on fails L_z, under (i) a tag from s = 0 and K̂ = 0 differs on the rigid draws, under (ii) the K̂ = 0 test removed gives NaN, step 4 on the unheld decode lands on a wrong pixel, the flag cleared on (L_z, E) fails (REQ-CHART-102).
- `cargo test -p kernel hold_rank` — a plane containing Jr, one through rest under hold E alone, and one through rest and one through a rigid rotation with both held report it, a generic plane doesn't; controls: the label removed fails; a plane through a rigid rotation under hold E alone reports no rank loss, and reporting it fails (REQ-CHART-111).

## Notes
- Split from TASK-M8-118 by R-916 (9 Nov 2026). TASK-M8-118 keeps REQ-CHART-126 (`τ`) and REQ-CHART-179 (the GPU tags).
- *R-919 (F13 (b)):* `τ` is read at M2 as a provisional value (R-182); its calibration stays TASK-M8-118's, on the GPU evidence.
- **Build order (R-916):** stage 3.
