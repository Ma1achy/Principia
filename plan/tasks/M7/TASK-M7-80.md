# TASK-M7-80 — Your shaders: a folder the app reads, their ids and versions, and update

- **Milestone:** M7
- **Depends on:** TASK-M7-83, TASK-M7-84

## Goal
Gaps 14, 16, 17, 19 and 21, import aside: making a node makes its shader here at once; a user folder scanned as ours is, `@id`, `@author`, `@licence`, `@node_version`; saved stains hold the source by value and offer an update; duplicate ids are asked about.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p engine`: a saved stain draws the same after its node's file changes, and offers the update; control: a stain that resolves by id. Two files with one id are refused until one is chosen.
- `cargo test -p engine -p gui`: every control of every shipped node moved to each end of its range leaves the node's shader, its id and your folder as they were, control: a setting written into the source; Save as a node makes one shader whose defaults are the values; an edit to code that is not yours stays in the stain until saved.
