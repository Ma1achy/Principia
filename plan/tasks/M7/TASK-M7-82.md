# TASK-M7-82 — The node author's guide, generated from the grammar, and `prin node check`

- **Milestone:** M7
- **Depends on:** TASK-M7-78, TASK-M7-79

## Goal
Gaps 15 and 22: the guide's reference is generated from the grammar's own tables beside the complete example; `prin node check <file>` runs the app's parser and compiler; the version rule covers the read-side names.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p prin`: the generated reference is current, control: a word added without regenerating; `node check` accepts every built-in and refuses each malformed fixture at its line.
