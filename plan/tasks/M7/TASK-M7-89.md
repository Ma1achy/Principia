# TASK-M7-89 — What can change the picture, proved from the shader: dead controls, inputs, fields and nodes dropped

- **Milestone:** M7
- **Depends on:** TASK-M7-77

## Goal
Gap 26 of `docs/design/principia_node_sdk.md`: a one-sided static proof over naga's IR, per shader and over the graph, in three questions (reach, reach with these settings, what can change the picture), and the dropping it allows.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render`: every uniform, input and field judged dead is set to random values, NaN and infinity included, and the picture does not change by a bit, over every shipped source and a fixture set; controls: branches forgotten, pointer writes forgotten, `0 * x` folded.
- A shipped control that can never matter fails the build; control: one planted.
- `cargo test -p gui`: a control that cannot matter now is dimmed with the setting that would make it matter.
