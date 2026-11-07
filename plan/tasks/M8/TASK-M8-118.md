# TASK-M8-118 — Hold L_z on any chart, beside energy normalisation

- **Milestone:** M8
- **Closes:** REQ-CHART-102, REQ-CHART-111
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-37, TASK-M2-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
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
- `cargo test -p kernel hold_lz`, `cargo test -p kernel hold_outcomes`, `cargo test -p engine hold_encode` and `cargo test -p engine constraint_axis_refused` — L_z = L*, Σp = 0, both held, the near-rigid fixture, the six outcomes in order, encode on the held decode; off bitwise; each refusal row by name, Some(0) included; controls: η_E alone fails L_z, one projection fails the fixture, outcomes 2 and 3 swapped, the flag cleared fails (REQ-CHART-102).
- `cargo test -p kernel hold_rank` — both rank-loss cases report ray-degenerate, a generic plane doesn't; control: the label removed (REQ-CHART-111).

## Notes
- RQ-722 is for the rest charts. RQ-715 closes only the held-L_z half of RQ-701.
- Free charts take no tilt target on L*; a chart holding L_z by construction moves by value (R-720).
- Waits on its rulings item: RQ-715, RQ-722. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
