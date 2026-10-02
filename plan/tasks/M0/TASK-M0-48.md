# TASK-M0-48 — Tests delete their scratch folders on success and keep them only on failure

- **Milestone:** M0
- **Closes:** REQ-VAL-178
- **Depends on:** TASK-M0-06, TASK-M0-17, TASK-M0-18, TASK-M0-23, TASK-M0-38, TASK-M0-39, TASK-M0-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~260 lines

## Goal
R-333 gave `xtask/tests/qa_TASK-M0-06_edges.rs`'s `scratch_dir` a folder per call, named by the process id and a
per-process counter (qa's commit dc3afb4). No later run reuses that name, and nothing deletes the folder, so the
folders pile up under `CARGO_TARGET_TMPDIR` run after run. Other test helpers make scratch the same way. Under R-342 a
test's scratch folder, or scratch file, is deleted when the test passes and kept, with its path in the failure
output, when the test fails. Under R-359 a negative control passes only when it panics with its expected message
(R-212), so `negative_control!` catches the control's panic, compares the message, deletes the control's scratch only on
a match, and resumes the panic, so libtest's verdict is unchanged.

## References
- `decisions.md` § "R-359 — A negative control's scratch is deleted only when its panic message matches *(amends R-342; applies R-212)*"
- `decisions.md` § "R-342 — Tests delete their scratch folders on success and keep them only on failure *(amends R-290)*"
- `decisions.md` § "R-333 — The `qa_TASK-M0-06_edges` flake: the test and its control get separate scratch folders"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-237 — qa's commit may add files under `xtask/tests/`"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"

## Deliverables
- A scratch guard in each of `xtask`, `prin` and `validation`'s tests (a shared support module per crate): it makes a
  fresh, uniquely named folder, deletes it when the test passes, and keeps it, printing its path, when the test
  panics. Its tests, `scratch_guard`, with registered negative controls (R-176). The guard does not read the thread
  name: a control's panic is not a pass to it (R-359).
- `crates/validation/src/control.rs`: `negative_control!` runs the control inside a catch of its panic, compares the
  caught message with its `expected` one as `#[should_panic(expected = …)]` does (the message contains it, R-212),
  deletes the scratch made in the control only on a match, keeps it and prints its path otherwise, as the guard does
  for a failing test, and then resumes the panic with its original payload, keeping `#[should_panic(expected = …)]`, so
  libtest's verdict is unchanged (R-359). A guard dropped while its control unwinds leaves its path for the macro to
  settle; how it hands it over is the implementation's. The module docs say so.
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
- `cargo test -p validation --features controls scratch_guard_keeps_a_wrong_message_controls_scratch` — a control whose
  panic does not contain its expected message, run as a child test (as `scratch_guard_keeps_a_failing_tests_scratch`
  runs its child, so the suite's own run stays green), fails, keeps its scratch folder, and prints its path in the
  child's output; a control that does not panic keeps its folder too; and after a passing run of the registered
  controls, a control whose panic matches leaves no folder. Its registered negative control: a `negative_control!` that
  deletes the scratch on any panic (#114's design item 2) (REQ-VAL-178, R-359).
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
- R-359 replaces PR #114's design item 2 (applied per R-204 in its PR body: any panic in a `::negative_control` test
  counted as a pass, read from the thread name, and its scratch deleted). A control's scratch is deleted only when its
  panic message matches.
- Applied per R-204, accepted by R-363 (RQ-194, A4) — R-359: a control that does not panic at all fails, so its scratch is kept
  and its path printed, as for a wrong message, since the ruling deletes it only on a match.
- The guard runs in the qa commit's tests too, so qa's files depend on the implementer's support module; qa's commit
  comes after the implementer's, as usual.
