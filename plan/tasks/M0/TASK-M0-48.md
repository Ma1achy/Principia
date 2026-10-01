# TASK-M0-48 — Tests delete their scratch folders on success and keep them only on failure

- **Milestone:** M0
- **Closes:** REQ-VAL-178
- **Depends on:** TASK-M0-06, TASK-M0-17, TASK-M0-18, TASK-M0-23, TASK-M0-38, TASK-M0-39, TASK-M0-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
R-333 gave `xtask/tests/qa_TASK-M0-06_edges.rs`'s `scratch_dir` a folder per call, named by the process id and a
per-process counter (qa's commit dc3afb4). No later run reuses that name, and nothing deletes the folder, so the
folders pile up under `CARGO_TARGET_TMPDIR` run after run. Other test helpers make scratch the same way. Under R-342 a
test's scratch folder, or scratch file, is deleted when the test passes and kept, with its path in the failure
output, when the test fails.

## References
- `decisions.md` § "R-342 — Tests delete their scratch folders on success and keep them only on failure *(amends R-290)*"
- `decisions.md` § "R-333 — The `qa_TASK-M0-06_edges` flake: the test and its control get separate scratch folders"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-237 — qa's commit may add files under `xtask/tests/`"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- A scratch guard in each of `xtask`, `prin` and `validation`'s tests (a shared support module per crate): it makes a
  fresh, uniquely named folder, deletes it when the test passes, and keeps it, printing its path, when the test
  panics. Its tests, `scratch_guard`, with registered negative controls (R-176).
- The helpers below use it. A search of 1 Oct 2026 found each makes a uniquely named scratch folder or file per call
  (by process id, counter or time) and leaves it behind. The implementer repeats the search at the start and the PR
  names any helper added since.
  - The implementer's files: `crates/prin/tests/profile.rs` (`scratch`), `crates/prin/src/profile/diff.rs`'s unit
    tests (`scratch`, under the system temp folder), `crates/validation/tests/stand_in.rs` (`scratch`),
    `xtask/tests/mutants_no_mutant.rs` (`scratch`).
  - qa's files, which qa changes in this task's qa commit: `xtask/tests/qa_TASK-M0-06_edges.rs` (`scratch_dir`),
    `xtask/tests/qa_TASK-M0-23_r305.rs`, `xtask/tests/qa_TASK-M0-23_shards.rs`, `xtask/tests/qa_TASK-M0-40.rs`,
    `xtask/tests/qa_TASK-M0-38.rs` (`pr_check`'s event files), `crates/prin/tests/qa_TASK-M0-18.rs`,
    `crates/prin/tests/qa_TASK-M0-18_base.rs`, `crates/prin/tests/qa_TASK-M0-18_rulings.rs`,
    `crates/validation/tests/qa_TASK-M0-38.rs` (`fifo`), `crates/validation/tests/qa_TASK-M0-39.rs` (`scratch`).
- A helper with a fixed name reuses and replaces its folder on the next run, so it does not accumulate and is left as
  it is.

## Acceptance tests
- `cargo test -p xtask scratch_guard`, `cargo test -p prin scratch_guard`, `cargo test -p validation scratch_guard` —
  a scratch folder made for a test body that passes is gone afterwards; one made for a body that panics is kept and
  its path printed; each with a registered negative control (a guard that never deletes, one that always deletes)
  (REQ-VAL-178, R-342).
- Each named helper's tests still pass, and after a passing run of them none of their scratch folders or files is left
  under `CARGO_TARGET_TMPDIR` or the system temp folder; the PR shows the listing before and after (REQ-VAL-178).
- Review checklist (code) — qa's commit changes only where scratch is made and when it is deleted, never an
  assertion; the PR lists each `M` line of qa's commit with its reason (R-290, R-342).

## Notes
- Applied per R-204, accepted by R-346: the fix touches qa's files, so qa makes those edits, in this task's qa commit,
  and the implementer does not. R-290 allows it on the files only qa has committed to. Two of them have implementer
  commits, `xtask/tests/qa_TASK-M0-38.rs` (55815e6, 8c22eec) and `crates/validation/tests/qa_TASK-M0-38.rs` (f7becfc),
  so for those this is a named exception to R-290, as R-336's is. The orchestrator's R-237 check accepts `M` on the qa
  files named above in that commit.
- Applied per R-204, accepted by R-346: the scope is the helpers that make a uniquely named folder or file per call;
  fixed-name helpers do not accumulate and are left alone.
- The guard runs in the qa commit's tests too, so qa's files depend on the implementer's support module; qa's commit
  comes after the implementer's, as usual.
