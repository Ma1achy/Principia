# TASK-M7-90 — A shader opens as a graph: code edits inside a node come back as that node

- **Milestone:** M7
- **Depends on:** TASK-M7-14, TASK-M7-89

## Goal
Gap 27 of `docs/design/principia_node_sdk.md`: the assembled text keeps each node's function and declarations; editing it, or pasting a shader made of declared functions, gives a graph; only text with no such form stays code.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render`: graph to text to graph is the identity on every library row and preset, control: a declaration dropped from the text; an edit inside one function changes that node only; a shader with no graph form stays code and still draws.
