# TASK-M2-42 — The pipeline_runtime teardown SIGSEGV stops: reproduce it, find the cause, fix it, and check for it

- **Milestone:** M2
- **Closes:** REQ-VAL-192
- **Depends on:** TASK-M1-05, TASK-M1-08
- **Needs (earlier milestones):** REQ-RENDER-011
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
`render::pipeline_runtime`'s `node_failure_isolation_unexplained_failure_keeps_the_pipeline`
(`crates/render/tests/pipeline_runtime.rs`, TASK-M1-05) sometimes dies with SIGSEGV after its assertions have passed,
so a `ci` shard goes red until re-run. libtest prints `test … ... ok` and `test result: ok. 1 passed`, and then nextest
reports `(test aborted with signal 11: SIGSEGV)`: the crash is in teardown, not in the test body. It was seen twice,
both in shard 2 (`cargo nextest run … --partition hash:2/4`) on lavapipe (`gpu backend: vulkan (from
PRIN_GPU_BACKEND)`), with three other tests still running beside it:

- PR #235, `ci` run 39048546893, attempt 1, job 117446692905 (17 Oct 2026, head 360d1c75): `SIGSEGV [ 3.719s]
  (519/866) render::pipeline_runtime node_failure_isolation_unexplained_failure_keeps_the_pipeline`.
- PR #254, `ci` run 39427866705, attempt 1, job 118650558546 (20 Oct 2026, head 200512a8): `SIGSEGV [ 4.315s]
  (615/973) render::pipeline_runtime node_failure_isolation_unexplained_failure_keeps_the_pipeline`.

Both passed on re-run (attempt 2). Neither PR touched `crates/render`. As with the r217 flake (R-276, TASK-M0-39) and
the `qa_TASK-M0-06_edges` flake (R-333), the cause is found and fixed: a retry, an `#[ignore]`, a backend skip or a
nextest retry policy is not the fix.

## References
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"
- `decisions.md` § "R-276 — Four follow-ups: the r217 flake, M0-06's wording, conversation resolution, reviews re-run on each review"
- `decisions.md` § "R-333 — The `qa_TASK-M0-06_edges` flake: the test and its control get separate scratch folders"
- `decisions.md` § "R-272 — Throwaway `measure/` branches are allowed; the ubuntu mutants timing runs on one *(closes RQ-164)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"

## Deliverables
- **Reproduce first.** A bounded repeated run on ubuntu-24.04 with lavapipe, as the `ci` job installs it, on a
  `measure/` branch (R-272) or a soak workflow beside `stand-in-soak.yml`: the test alone, then the whole
  `pipeline_runtime` binary under nextest at the CI shard's parallelism, each run with a timeout, each run's exit
  status recorded (a status, or the signal). The PR gives the crash rate it found and the log. If it does not
  reproduce in the bounded runs, the runs are widened (the render crate's tests beside it, as in shard 2) and the PR
  says what was run.
- **Find the cause.** A backtrace or core of the crash (for example `ulimit -c unlimited` with `gdb -batch`, or
  `RUST_BACKTRACE` with a SIGSEGV handler that prints the faulting thread), naming the thread and the frame, and the
  teardown sequence that leads there: what `Rig`, `RenderLoop`, the pipeline cache's async compile work and
  `GpuHarness` drop, and in what order, against what the device and the driver still hold. The PR states the cause.
- **Fix it** in the code that owns the sequence (the test's rig, `crates/render/src/pipeline_cache.rs`, the render
  loop, or `GpuHarness`), so the process exits by status. If the cause lies outside the repo (wgpu or Mesa) and no
  in-repo change removes it, it goes to `REVIEW_QUEUE.md` with the evidence, and the task waits.
- **A regression check that fails on the crash:** `pipeline_teardown` tests in `crates/render/tests/` that run the
  found teardown sequence in a child process (the test binary re-run on one test, as
  `scratch_guard_keeps_a_failing_tests_scratch` runs its child) and fail when the child dies by a signal, naming the
  signal. A registered negative control (R-176) shows the check fails on a child that dies by a signal.
- The test itself is left as it is: no retry, `#[ignore]`, backend skip or nextest retry or `flaky` override for it.

## Acceptance tests
- CI log on the PR, on ubuntu-24.04 lavapipe: `node_failure_isolation_unexplained_failure_keeps_the_pipeline` passes
  500 consecutive runs alone, and the `pipeline_runtime` binary passes 100 consecutive runs under nextest with the CI
  shard's parallelism, each run's exit status recorded and none by signal (REQ-VAL-192).
- `cargo test -p render pipeline_teardown` — the found teardown sequence, in a child process, exits by status; a child
  that dies by a signal fails the check, naming the signal (REQ-VAL-192).
- `cargo xtask controls` — the `pipeline_teardown` control trips (REQ-VAL-192, R-176).
- `cargo test -p render node_failure_isolation` — still passes, its control still trips (REQ-RENDER-011).
- Review checklist (code) — the PR states the cause with its backtrace; the fix is in the sequence, not a retry,
  ignore, skip or retry policy; no assertion of `pipeline_runtime.rs` or of qa's files is weakened (REQ-VAL-192).

## Notes
- Applied per R-369: a tracked task for a recurring CI flake, after R-276 (TASK-M0-39) and R-333; the orchestrator
  decides tests and tooling, so it needs no ruling.
- Applied per R-369: M2, the milestone in progress, since M1's gate has passed (R-432) and a new requirement does not
  join a passed gate.
- Applied per R-369: reviewers code and qa, as TASK-M0-39's were; the fix is at teardown, not in the frame loop's
  per-frame path, so perf does not review. If the fix does change the per-frame path, the code reviewer names perf
  and the orchestrator adds it.
- Applied per R-369: 500 runs alone and 100 runs of the binary, with each run bounded by a timeout. Two failures in a
  few hundred CI runs puts the rate below 1%; the reproduction step measures it, and the PR raises the counts if the
  measured rate needs more runs to show the fix.
- `crates/render/tests/pipeline_runtime.rs` is the implementer's file (TASK-M1-05, TASK-M1-08), so the implementer may
  change its rig; `qa_TASK-M1-05.rs` and `qa_TASK-M1-05_shared.rs` are qa's, and only qa changes them (R-290).
