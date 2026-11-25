# TASK-M7-85 — Field sources as one shipped node with a field parameter

- **Milestone:** M7
- **Depends on:** TASK-M7-78, TASK-M7-84

## Goal
The assembler's generated source WGSL becomes one shipped source, `field`, with a `field` parameter; its out-port's subtype follows the parameter.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render`: for every readable field the assembled code is byte-identical to before, control: the subtype taken from the wrong field; old saved sources open.
