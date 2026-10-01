# TASK-M0-47 — `prin profile` streams its trace: header first, each frame as it completes, flushed every 60 frames or 1 s

- **Milestone:** M0
- **Closes:** REQ-TOOL-147
- **Depends on:** TASK-M0-18
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
TASK-M0-18's `prin profile --scenario NAME --frames N --json PATH` holds every frame record of the run in memory and
writes the whole trace when the run ends, so a run that crashes or is killed leaves nothing, and its memory grows with
`--frames`. Under R-341 it streams: the header line first, then each frame record as the frame completes, the file
flushed at least every 60 frames or every 1 s, whichever comes first, so a crash loses at most the frames since the
last flush; `leak_flags` and `hot_paths` are appended as the final line at session end (R-286, R-298). The run keeps
no frame record once it is written, so its memory does not grow with `--frames` beyond what the summaries themselves
need. A run killed mid-session leaves an incomplete session, which the reader reports as "session incomplete" (R-298,
R-299; telemetry §5).

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `decisions.md` § "R-341 — `prin profile` streams its trace: the header first, each frame as it completes, flushed every 60 frames or 1 s *(amends R-286, R-298)*"
- `decisions.md` § "R-286 — Profiler traces are JSON Lines: the header, then one compact frame record per line"
- `decisions.md` § "R-298 — TASK-M0-17's items 12 and 15 accepted; a trace with no summary line is valid *(amends R-286)*"
- `decisions.md` § "R-299 — The reader drops a cut-off final line and says how many bytes it dropped *(amends R-298)*"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- `crates/prin/src/profile/run.rs`: the run writes the header line, flushed at once, then each frame record as the
  scenario produces it, flushing at the 60th frame since the last flush or once 1 s has passed since it, whichever
  comes first; the summary line last, when the session ends. No `Vec` of the run's frame records, or other per-frame
  state kept after a frame is written. The flush policy takes its clock from the caller, so a test can drive it.
- `crates/engine/src/contract/profile.rs` (TASK-M0-17's file), if the streaming writer belongs beside `write`: a
  writer that takes the header, then frames one at a time, then the summary, with the same compact lines and the same
  refusals as `write` (telemetry §5). `write` keeps its behaviour.
- `crates/prin/tests/profile.rs`: the tests below, each with a registered negative control (R-176).

## Acceptance tests
- `cargo test -p prin profile_stream_killed` — a `prin profile --scenario synthetic_frames` run, given a frame count
  it cannot finish first, is killed once its trace holds the header and a frame line; the trace grew while the run
  ran, not only at its end; the engine reader returns the header and the frames from 0 in order, and reports the
  session incomplete, "session incomplete", dropping a cut-off last line with its byte count (REQ-TOOL-147, R-341,
  R-298, R-299). Its negative control: a run that writes the trace only at its end leaves no trace to read.
- `cargo test -p prin profile_stream_flush` — over a writer that records its flushes and a clock the test drives: the
  header line is flushed when written; frame lines are flushed at the 60th frame since the last flush, or once 1 s has
  passed since it, whichever comes first, and never later; the summary line is written last; each with a registered
  negative control (a policy that flushes only at 61 frames, one that ignores the clock) (REQ-TOOL-147, R-341).
- `cargo test -p prin profile_file`, `profile_scenario`, `profile_diff`, `profile_show` and `profile_no_gpu` —
  TASK-M0-18's tests still pass: a completed run's trace is the same file as before (REQ-TOOL-002, REQ-TOOL-006).
- Review checklist (perf) — the run keeps no frame record once it is written; the PR reports the run's peak memory at
  two frame counts, one 100 times the other, and the perf reviewer confirms it does not grow with `--frames`
  (REQ-TOOL-147, R-341).

## Notes
- R-341 (1 Oct 2026): the flush is every 60 frames or 1 s, whichever comes first. Applied per R-204 — veto?: the header
  line is flushed as soon as it is written, so a run killed before its first frame flush still leaves a valid trace (a
  header line alone, R-298); the memory bound is judged by the perf reviewer on the two measurements, since the corpus
  gives no number for it.
- Reviewers as TASK-M0-18's, less physics: no definition changes here.
- The kill is `std::process::Child::kill`, so the test runs on every CI runner.
