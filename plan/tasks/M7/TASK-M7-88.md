# TASK-M7-88 — The built-ins prove the path: the app run with its shipped nodes as a user's folder, and the lint that keeps it so

- **Milestone:** M7
- **Depends on:** TASK-M7-80, TASK-M7-86, TASK-M7-87

## Goal
The final proof and the lock of `docs/design/principia_node_sdk.md` section 4.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p gui -p engine`: with no shipped sources and the same files as the user's folder, the library, inspectors, legends, pictures and presets are the same; control: one preset that names a shipped id directly.
- `cargo xtask lint` refuses a shipped id or a match on built-in outside the registry and the saved-form reader; control: one planted.
