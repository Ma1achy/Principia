# TASK-M0-51 — The profile reader accepts a cut-off last line after the summary line

- **Milestone:** M0
- **Closes:** REQ-TOOL-148
- **Depends on:** TASK-M0-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
Under R-356 a cut-off line after the summary line is valid: the reader keeps the header line, the frames and the
summary line, drops the cut-off bytes and reports them in `Trace::dropped_bytes`, and the session is complete, since its
summary line is present. Today's reader, `engine::contract::profile::read` (`read_lines` in
`crates/engine/src/contract/profile.rs`, TASK-M0-17), holds each line until the next arrives and parses every line that
has one after it as a frame record, so the summary line fails ("line N, a frame record: …") and the file is rejected.
`engine::contract::profile::read` accepts a cut-off last line after the summary line, with a test and a negative
control. A file whose only line is a cut-off header line stays an error that states its bytes, and its test is kept.
`prin profile show` (`crates/prin/src/profile/show.rs`) calls any trace with dropped bytes "session incomplete", so its
notice changes with the reader, with its own test and negative control.

## References
- `decisions.md` § "R-356 — A cut-off line after the summary line is valid; R-297's design bullets and its R-84 and R-116 amendments stand *(amends R-299, R-304)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `decisions.md` § "R-299 — The reader drops a cut-off final line and says how many bytes it dropped *(amends R-298)*"
- `decisions.md` § "R-298 — TASK-M0-17's items 12 and 15 accepted; a trace with no summary line is valid *(amends R-286)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- `crates/engine/src/contract/profile.rs`: `read_lines` tells the summary line from a frame record when the line after
  it is a cut-off last line, and reads the file as R-356 has it: the header line, the frames and the summary line kept,
  `Trace::dropped_bytes` the cut-off line's length, the session complete. `Trace`'s rule that a trace with dropped bytes
  is incomplete (its doc comment and `check_ranges`) gives way: a complete trace may have dropped bytes. `read`'s and
  `Trace::dropped_bytes`' doc comments say so. Every other case reads as it does today, a cut-off header as the only
  line included.
- `crates/prin/src/profile/show.rs`: `prin profile show`'s notice, which says "session incomplete; the last line was
  cut off …" whenever `trace.dropped_bytes > 0`, says "session incomplete" only for an incomplete session; for a
  complete one with dropped bytes it states the bytes dropped after the summary line, without "session incomplete"
  (R-356's applied item). The notice for an incomplete session is unchanged.
- `crates/engine/src/contract/tests/profile_v1.rs` and `crates/prin/tests/profile.rs`: the tests below, each with its
  registered negative control (R-176).

## Acceptance tests
- `cargo test -p engine profile_v1_superset_cut_off_after_summary` — a complete trace (the header line, frame lines,
  and a summary line with `leak_flags` and `hot_paths` set) followed by a last line cut off at several points inside a
  frame-record-shaped and inside a summary-shaped line reads: the header, every frame and both summaries are returned,
  the session is complete and `dropped_bytes` equals the cut-off line's length (REQ-TOOL-148, R-356). Its negative
  control: a reader that parses every line with one after it as a frame record, as TASK-M0-17's does, rejects that
  file.
- `cargo test -p engine profile_v1_superset_cut_off` — a file whose only line is a cut-off header line is still an
  error stating its bytes, and the other cut-off cases read as R-299 has them (REQ-TOOL-148, R-299).
- `cargo test -p prin profile_show_cut_off_after_summary` — `prin profile show`, plain and `--pretty`, on a complete
  trace followed by a cut-off line prints the header, frames and summary line, and its notice states the dropped
  bytes and does not say "session incomplete"; on a trace cut off before its summary line the notice still says
  "session incomplete" (REQ-TOOL-148, R-356). Its negative control: a notice keyed on `dropped_bytes > 0` alone, as
  today's, says "session incomplete" for the complete trace and fails the check.
- `cargo test -p engine profile_v1` and `cargo test -p prin profile` — TASK-M0-17's and TASK-M0-18's tests still pass
  (REQ-TOOL-008), but for the two qa assertions R-356 changes, below.

## Notes
- R-356 (2 Oct 2026). That the session reads complete, with the bytes reported as dropped and not as "session
  incomplete", by the reader and by `prin profile show`, is R-356's applied-per-R-204 item, open in RQ-192; if the
  human vetoes it, this task follows the ruling.
- Two qa assertions test the behaviour R-356 changes: `crates/engine/tests/qa_TASK-M0-17.rs`'s
  `qa_m017_r299_a_malformed_line_ending_in_a_newline_is_an_error` asserts that a cut-off frame line after the summary
  line is an error, and `crates/prin/tests/qa_TASK-M0-18.rs`'s `qa_profile_diff_cut_line_with_newline_is_unreadable`
  that `prin profile diff` exits 2 on that file. Every commit to either file is a qa commit, so qa changes those two
  assertions in this task's qa commit (R-290), and the implementer does not; the PR lists each `M` line with its
  reason, and the code reviewer confirms that they are the only assertions changed, and that the cut-off header case
  and the malformed-line cases beside them are kept.
- Reviewers: code and qa, as for every task; it closes no PERF requirement and touches no frame loop or dispatch.
- Goes to the cloud session with the ready tasks R-355 handed it (R-356, `plan/OPERATIONS.md` § "Away mode").
