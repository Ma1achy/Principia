# TASK-M0-38 — Three harness follow-ups: embedded libtest headers, ETXTBSY on stand-in executables, nameless meter lines

- **Milestone:** M0
- **Closes:** REQ-SYS-069, REQ-SYS-070, REQ-SYS-071
- **Depends on:** TASK-M0-03, TASK-M0-22, TASK-M0-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
Three small defects found in review, gathered into one low-priority task (R-267). `parse_wrong_panics` takes a
`---- x stdout ----` string inside a control's own output as a new section. The stand-in-`cargo` tests in
`qa_TASK-M0-22.rs` can fail on Linux with "Text file busy". `cargo xtask pr-check` passes a meter line that names nothing.

## References
- `decisions.md` § "R-267 — Every merged "veto?" item stands; the #38 flake item is closed; three follow-ups become one task"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-180 — pr-check is item-level *(closes A5)*"
- `decisions.md` § "R-227 — The two remaining latent races are fixed now, in their own task"

## Deliverables
- `xtask/src/controls.rs` — `parse_wrong_panics` (:146-164) ends a control's section only at a libtest section header
  at the start of a line, so an embedded `---- x stdout ----` inside the control's output stays in that control's note.
- The ETXTBSY fix. R-267 records the diagnosis: CI run 36634340042 attempt 1 failed `qa_m022_list_runs_no_control`
  with "cannot run cargo metadata: Text file busy (os error 26)". It is the Linux fork/exec race: one thread holds its
  stand-in script open for writing while another thread's fork inherits the descriptor, so the exec fails. The fix is
  that writing a stand-in executable and spawning children never overlap across the test binary's threads, for example
  one process-wide lock held across each stand-in write and around spawns, or writing every stand-in before any spawn.
  It goes in the shared place (`crates/validation`'s spawn helper, or a helper qa's files call), not by retrying, and
  covers every test that writes an executable, not only qa_TASK-M0-22.rs.
- `xtask/src/pr_check.rs` — a `- meter:` or `- discriminator:` line with nothing before its dash fails, naming the line.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask controls` — a control whose output embeds `---- x stdout ----` keeps its whole note (REQ-SYS-069).
- CI log on the PR head: the stand-in-cargo tests pass 50 consecutive parallel runs on ubuntu-latest (REQ-SYS-070).
- `cargo test -p xtask pr_check` — `- meter: — COM drift` fails naming the line; a named meter passes (REQ-SYS-071).

## Notes
- An edit to qa's merged `qa_TASK-M0-22.rs`, if the ETXTBSY fix needs one there, follows R-227's exception.
- Low priority (R-267): dispatch when nothing on the M0 critical path is waiting for a slot.
