# TASK-M0-45 — Shard the CI test runs and split the long single tests, so each job is back under ~10.5 min

- **Milestone:** M0
- **Closes:** REQ-SYS-077
- **Depends on:** TASK-M0-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
On PR #96 the warm `ci` job took about 16.5 min and `xtask-ci` 11.7–13.3 min, against R-270's ~10.5 min per job
(R-325). The human accepted that overrun for #96 and made this the next M0 task, at high priority (R-336). The tests
that run alone dominate: `qa_cargo_xtask_alias_runs_deps` at 257 s, and the `qa_TASK-M0-24`, `qa_TASK-M0-25` and
`qa_TASK-M0-26` suites at about 100 s each. The `ci` job's nextest run is sharded across parallel jobs, and since a
shard cannot run one test in parallel, those long tests are split into shorter ones. The `xtask-ci` job runs no
nextest: its time is `cargo xtask controls`'s own `cargo test` runs (RQ-193), so `cargo xtask ci --partition k/4` runs
it as 4 parallel jobs, the controls assigned by a stable hash of the control name (R-360). Each CI job's warm
wall-clock time is then ~10.5 min or less.

## References
- `decisions.md` § "R-360 — `cargo xtask ci --partition k/n` splits the controls into n = 4 parallel jobs, by a stable hash of the control name *(closes RQ-193; amends R-336)*"
- `decisions.md` § "R-366 — qa updates TASK-M0-45's pinned CI step lines; nextest shards by `hash:<k>/4`; #117's other items stand *(closes RQ-197; amends R-290, R-336 and R-360 as they apply)*"
- `decisions.md` § "R-264 — The size budget is a rough heuristic that weighs complexity; M0-09, M3-08 and M5-18 stay whole *(amends R-256, R-211)*"
- `decisions.md` § "R-336 — #96's CI overrun is accepted; TASK-M0-45 shards nextest and splits the long single tests *(amends R-270, R-290)*"
- `decisions.md` § "R-325 — CI: the GPU kernel build and its tests run in their own parallel job; ≤ ~10.5 min per job *(amends R-301)*"
- `decisions.md` § "R-270 — TASK-M0-33: qa's one-round exception is granted; the fixture-pool cost is sent back *(amends R-231)*"
- `decisions.md` § "R-231 — After TASK-M0-22, one task speeds up the suite: nextest, stable fixtures, injectable spawn timings"
- `decisions.md` § "R-235 — `qa_cargo_xtask_alias_runs_deps` uses the listing-only form of `cargo xtask ci`; the controls get their own CI job *(closes RQ-150)*"
- `decisions.md` § "R-266 — "Require branches to be up to date" stays off; bypassing is not allowed *(amends HUMAN_SETUP §2)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-335 — qa may narrow `qa_TASK-M0-22_r235.rs`'s `if:` check to the job's own `if:` *(closes RQ-186; amends R-290)*"
- `decisions.md` § "R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only *(amends R-285, R-320)*"
- `decisions.md` § "R-344 — `δ_λ` and `ε_w` are hashed; #108's four "veto?" items are accepted *(closes RQ-189; amends R-340)*"

## Deliverables
- `.github/workflows/ci.yml`: the `ci` job's nextest runs run as 4 parallel shards, each with nextest's
  `--partition hash:<k>/4`, not `slice:` (R-366), the shards together running every test the unsharded run did, in both feature sets. The `xtask-ci` job becomes 4
  parallel jobs, each running `cargo xtask ci --partition <k>/4` for k = 1 to 4 (R-360). Each shard job keeps its
  job's caches and their R-326 save rule, and its key names the shard's job (R-285).
- `xtask/src/main.rs`, `xtask/src/ci.rs` and `xtask/src/controls.rs`: `cargo xtask ci --partition <k>/<n>` and
  `cargo xtask controls --partition <k>/<n>` (R-360). `main.rs` parses the flag, `ci.rs` passes it to the `controls`
  runner, and `controls.rs` runs the controls in slice k of n: a control's slice is a stable hash of its libtest name
  (`<path>::<test>::negative_control`, as `--list` prints it), modulo n, the same on every run, machine and toolchain
  (a hash written in xtask's source, not std's `DefaultHasher`). The slices are disjoint and together hold every
  control, so every control runs exactly once across the shards, and the findings check (REQ-VAL-147) still covers
  every test, each finding reported by one shard. The controls keep their own cargo invocations (R-231) and run inside
  `cargo xtask ci` (R-235, REQ-VAL-007). Without `--partition`, both commands run every control, as today, and the
  listing-only form (`--list`, R-235) is unchanged.
- The other runners under `--partition` (applied per R-204, accepted by R-365, R-360: build-kernel in every shard): build-kernel
  runs in every shard, before its controls, which read its output; plan-check, the three lints, gate and golden run in
  shard 1 only.
- `.config/nextest.toml` and `xtask/tests/nextest.rs`'s listing check follow the shards, so the CI test steps together
  still list every test `cargo test --workspace -- --list` lists (REQ-VAL-165).
- The splits, made by qa in this task's qa commit (R-336): `qa_cargo_xtask_alias_runs_deps` in
  `xtask/tests/qa_TASK-M0-01.rs`, and the `qa_TASK-M0-24`, `qa_TASK-M0-25` and `qa_TASK-M0-26` suites in
  `crates/validation/tests/`, into tests each short enough not to hold a shard over its target. Every assertion and
  its negative control is kept (R-176), and `qa_cargo_xtask_alias_runs_deps`'s listing-only form stays (REQ-VAL-166).
- The CI step matches, made by qa in this task's qa commit (R-366, a named exception to R-290): in
  `xtask/tests/qa_TASK-M0-22_r235.rs` and `xtask/tests/support/qa_m0_01.rs`, qa changes only the matches of the
  unsharded steps to the exact sharded forms, `cargo nextest run --workspace --partition hash:${{ matrix.shard }}/4`
  and `cargo xtask ci --partition ${{ matrix.shard }}/4`, with their controls' edit targets moved to match; nothing
  else in the two files changes. The three qa-only files that match those lines (`qa_TASK-M0-14_r335.rs`,
  `qa_TASK-M0-33.rs`, `qa_TASK-M0-14_rust_gpu_cache.rs`) change the same way under R-290. The implementer does not
  edit any of them.
- The PR shows each CI job's warm wall time, and a cold one, against ~10.5 min, and names the new jobs to add to
  branch protection's required checks.

## Acceptance tests
- `cargo nextest run -p xtask controls_partition` — the 4 slices of a list of control names are disjoint, together
  hold every name, and give a fixed name the same slice on every run; with its negative control, a partition that
  drops a name or slices by crate (REQ-SYS-077, R-360).
- CI log on the PR head — the 4 `xtask-ci` jobs' `controls` lines together run each control exactly once, the same
  controls the unpartitioned `cargo xtask controls` runs (REQ-SYS-077, R-360).
- Review checklist (code, qa) — `ci.yml` runs the `ci` job's nextest runs as parallel shards, the shards' test lists
  together equal to the unsharded run's, both feature sets (REQ-SYS-077, R-336); and `xtask-ci` as 4 jobs of
  `cargo xtask ci --partition <k>/4` (REQ-SYS-077, R-360).
- `cargo nextest run -p xtask --test nextest` — the listing check passes over the sharded steps (REQ-VAL-165, REQ-SYS-077).
- `cargo nextest run -p xtask --test qa_TASK-M0-22_r235` — the controls job is still beside the tests and cannot be skipped or pass
  with a finding (REQ-VAL-166).
- Review checklist (code) — each split moves tests and controls without weakening an assertion; the PR lists each `M`
  and `D` line of qa's commit with its reason (R-290, R-336).
- Review checklist (code) — in `qa_TASK-M0-22_r235.rs` and `support/qa_m0_01.rs`, qa's commit changes only the CI step
  matches and their controls' edit targets, to the exact sharded forms; nothing else in them changed (R-366).
- CI log on the PR head — each CI job's warm wall time is ~10.5 min or less; a job still over it is named in the PR
  and goes back to the human, not accepted silently (REQ-SYS-077, R-325). If shards are still over ~10.5 min after
  qa's commit, the long tests are split further, first `qa_cargo_xtask_alias_runs_deps`, in preference to raising n
  (R-366).

## Notes
- High priority: the next M0 task to start once TASK-M0-14 (PR #96) merges (R-336).
- RQ-193 is ruled by R-360 (2 Oct 2026): option 1, sliced by a stable hash of the control name, n = 4. The branch's
  open copy of RQ-193 and REQ-SYS-077's `rq: [RQ-193]` go in its next fix pass; the rulings PR archived the entry.
- Size: R-360's xtask edits take the task from ~250 to ~400 lines, past the budget. It stays one task: the
  partition, the workflow's jobs and the check that every control runs once are one change, and the human left the
  size to the orchestrator (R-360, "Size is your call"; R-264).
- The long tests' files are qa's, and each has implementer commits, so R-290 alone lets neither qa nor the implementer
  change them. R-336 lets qa make the splits in this task's qa commit (applied per R-204, accepted by R-344), as R-335
  lets qa narrow `qa_TASK-M0-22_r235.rs`. The orchestrator's R-237 check accepts `M` on those files, and `A` for the
  files split from them, in that commit. The implementer does not edit them.
- `qa_TASK-M0-22_r235.rs` reads the `xtask-ci` job's structure, and R-335's exception covers only its `if:` check. If
  sharding would make any other of its checks fail, that goes to REVIEW_QUEUE before the file changes (R-290).
- That happened: RQ-197, ruled by R-366 (2 Oct 2026, option 1). qa's commit may change only the CI step matches in
  `xtask/tests/qa_TASK-M0-22_r235.rs` and `xtask/tests/support/qa_m0_01.rs`, a named exception to R-290 as R-335 and
  R-336 are, and the code reviewer confirms nothing else in them changed. The orchestrator's R-237 check accepts `M`
  on those two files in that commit.
- R-366 also settles PR #117's items applied per R-204: 4 `ci` shards, the `ci-checks` job, the doctests in shard 1,
  the gate jobs named `ci` and `xtask-ci`, and the shards' shared cache keys stand; nextest's `slice:` partition
  becomes `hash:<k>/4`. If a shard is still over ~10.5 min after qa's commit, split the long tests further, first
  `qa_cargo_xtask_alias_runs_deps`, rather than raising n.
