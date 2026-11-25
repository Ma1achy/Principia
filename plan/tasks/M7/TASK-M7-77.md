# TASK-M7-77 — The stain editor draws every node from its description: titles, glyphs, controls and choices, no special cases

- **Milestone:** M7
- **Depends on:** TASK-M7-83

## Goal
`crates/gui/src/stain` reads only the description: its tables of titles, the palette special case, the maps line, the choices and the eight-parameter cap go. Every control of the fixed set is drawn and is reachable by keyboard.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p gui`: a built-in and its source pasted as a custom draw the same shapes, for every built-in, with no `{ }` mark; control: a title taken from the id. A fixture with 40 parameters shows all 40.
- Screenshot of one node using every control.
