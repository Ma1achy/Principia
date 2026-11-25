# TASK-M7-81 — Other people's shaders: import, allowed before they run, loops bounded, a lost GPU recovered

- **Milestone:** M7
- **Depends on:** TASK-M7-14

## Goal
Gap 20: a source from outside the machine does not compile or run until allowed, its slot's default drawn and the node saying so; loops are bounded by a check on the parsed module; a lost device brings the app back with the node off.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render`: an imported source is not compiled before it is allowed, control: the gate open; an unbounded loop is refused at its line, control: the check removed; a forced device loss comes back with that node off.
