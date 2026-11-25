# TASK-M7-79 — What a node needs, read from its source, and the standard treatment of invalid pixels

- **Milestone:** M7
- **Depends on:** TASK-M7-89

## Goal
Gaps 7, 8 and 9: needs (FTLE, spread, word, measured range, time, quad data) and purity in n̂ come from TASK-M7-89's proof; the assembler wraps every node with the invalid, not-a-system and not-yet-run treatment unless the source says it handles validity.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render`: the derived needs of every built-in match a list written by hand; a custom with no validity code draws an invalid pixel as invalid, control: the wrapper removed; every built-in's picture is bit-identical.
