# TASK-M0-32 — The two remaining latent test races fixed at their cause

- **Milestone:** M0
- **Closes:** REQ-VAL-164
- **Depends on:** TASK-M0-31
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** small (R-227: "a small task like TASK-M0-31")

## Goal
Two races of the kind R-224 fixed remain in the suite, not yet seen failing: a test rebuilds a binary that sibling tests spawn, and parallel copies of one fixture build into one shared target directory. Each is fixed at its cause, and the workspace suite then passes 20 times in a row under load (R-227).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-208 — TASK-M0-21 is accepted at 762 code lines; later overruns are split first"
- `decisions.md` § "R-224 — The two flaky tests are fixed before TASK-M0-29"
- `decisions.md` § "R-227 — The two remaining latent races are fixed now, in their own task"

## Deliverables
- `cargo build -p xtask` in `crates/validation/tests/support/qa_m0_21_fixture.rs` (~line 35) no longer rewrites the `xtask` binary other tests spawn (`CARGO_BIN_EXE_xtask`): it builds into its own target directory, or the tests use the binary cargo already built.
- Parallel copies of one fixture no longer build into the shared outer target: `xtask/tests/controls.rs` (~line 63), `qa_m0_21_fixture.rs` (~line 143), `support/qa_m0_21.rs` (~line 16), `crates/validation/tests/expected_message.rs` (~line 42) each give a copy its own target directory under `$CARGO_TARGET_TMPDIR`, kept across runs where a fresh one would push a nested build past the 300 s timeout (as TASK-M0-31 found).
- Edits to qa's merged files are limited to the fix, under a one-round exception, with qa reviewing (R-227).

## Acceptance tests
- The whole workspace suite (`PRIN_GPU_BACKEND=metal cargo test --workspace --features validation/controls,prin/controls,xtask/controls`) passes 20 times in a row under a named load; the PR shows the loop, the load and each run's result (REQ-VAL-164).
- Review checklist (code and qa): no test's cargo run rewrites a binary another test spawns; no two parallel fixture copies share a target directory; no retry is added (REQ-VAL-164).

## Notes
- R-227: found by the nested-cargo survey of 27 Sep; the third race of that list (`qa_TASK-M0-26.rs`'s r206 test) is fixed in TASK-M0-31.
- TASK-M0-22 depends on this task, since both edit `xtask/tests/controls.rs`.
