# TASK-M7-76 — A node's own source describes it: name, group, glyph and controls, the same for built-in and custom

- **Milestone:** M7
- **Depends on:** none

## Goal
The engine half of the node SDK (`docs/design/principia_node_sdk.md`): the declaration grammar, every built-in declaring itself, and one description for the editor with no branch on built-in or custom.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p engine` and `-p render`: every built-in's source, pasted as a custom, is described the same in every part; control: a branch on the id. Every library row's picture is bit-identical.
