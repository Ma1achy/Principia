# TASK-M0-41 — `cargo xtask codegen` writes a generated file only when its content changes

- **Milestone:** M0
- **Closes:** REQ-TOOL-138
- **Depends on:** TASK-M0-07, TASK-M0-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~60 lines

## Goal
`cargo xtask codegen` rewrites `generated.rs` even when nothing in it changed, so its modification time moves and every
local build that depends on it rebuilds. It writes a generated file only when the content differs from what is on
disk (R-284). Medium priority (R-284).

## References
- `decisions.md` § "R-284 — `cargo xtask codegen` writes a generated file only when its content changes"
- `decisions.md` § "R-241 — `xtask` may depend on `ledger`, for `cargo xtask codegen` *(amends systems_architecture §7.1)*"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"

## Deliverables
- `xtask/src/codegen.rs`: before writing each generated file, compare the new content with the file on disk; write only when they differ, and report which files were written and which were unchanged.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask codegen_unchanged` — running codegen twice leaves every generated file's modification time unchanged on the second run; changing one ledger entry rewrites only the files it affects (REQ-TOOL-138).

## Notes
- Depends on TASK-M0-10, which changes the emitter, so the two don't edit `codegen.rs`'s outputs at once.
