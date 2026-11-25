# TASK-M7-83 — One registry of stain nodes: every shipped source named by its id, the second table gone

- **Milestone:** M7
- **Depends on:** TASK-M7-76

## Goal
`assemble::BUILTINS` goes; a graph names any source the scan found by its id, so the debug views are no longer placed as custom text, titled "custom". Nothing in a description says built-in or custom.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render -p engine`: every library row's assembled code is byte-identical (`declared_code`); every row that loads holds shipped occupants only, control: one row placed as text.
- Saved stains in the old form open and draw the same.
