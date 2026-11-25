# TASK-M7-84 — Stain keys and saved stains by code: a shipped source cannot change under a saved stain

- **Milestone:** M7
- **Depends on:** TASK-M7-83

## Goal
The fragment key hashes a node's code whoever wrote it. A saved node holds id, version, hash and text; the reader takes the old `builtin`, `field` and `custom` forms.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p engine -p render`: a shipped source and its pasted copy have one key; a stain saved before a shipped source changes still draws as it did and offers the update, control: a reader that resolves by id; each old form round-trips.
