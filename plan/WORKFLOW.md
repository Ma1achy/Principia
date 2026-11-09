# Workflow — how a task becomes merged code

How the build runs from `plan/`. The docs stay the authority (plan/MILESTONES.md, "Assumptions"); this file says who
does what, in what order, and what stops the line.
How the orchestrator runs it day to day (dispatch, merging, autonomy, machine resources, and a cloud session's setup
with `scripts/cloud-setup.sh`) is in `plan/OPERATIONS.md` (R-346).

## The unit: one task, one branch, one PR

- Each task in `plan/tasks.yaml` is built on its own branch, `task/<TASK-id>` (e.g. `task/TASK-M3-04`), and merged
  through one PR titled `<TASK-id>: <title>`. Nothing else goes in that PR.
- A task starts only when every task in its **Depends on** list is merged, or early, off a dependency's approved head
  (R-388): when every other dependency has merged and exactly one is unmerged, and every reviewer that dependency's task
  names has approved its head and its CI is green on that head apart from mutants, the task may start (applied per
  R-369: only one unmerged dependency, since branching from two unmerged heads would be stacking, R-362). It branches
  from that head and opens no PR until the dependency merges; then it merges `main` in and opens its PR normally,
  against `main`, so nothing is stacked (R-362). If the dependency changes beyond added tests before it merges, the task
  is re-checked against the merged dependency before its PR opens.
- The PR description lists the requirement ids the task closes and, for each, the acceptance command that
  demonstrates it, with its output. It links the task file.
- A GUI-track task's PR description (R-390; TASK-M6-24 to TASK-M6-29, TASK-M6-30 and TASK-M6-31, R-406 and R-409,
  TASK-M6-32 to TASK-M6-36, R-450 to R-453, TASK-M6-37, R-451 B11, TASK-M6-38 to TASK-M6-44, R-457 to R-463, and TASK-M6-45, R-466) also has a "What to try" section for the human: the clicks and keys that show what changed, on
  `cargo run -p gui --features mock`.
- So does the PR of each of M8's GUI-track tasks (R-627: TASK-M8-05, TASK-M8-06, TASK-M8-07, TASK-M8-13, TASK-M8-19, TASK-M8-20, TASK-M8-23 and TASK-M8-24, and TASK-M8-86 and TASK-M8-96, the v1 web skeleton and its measurements).
  A wiring task's PR (TASK-M8-88 to TASK-M8-95) shows its track task's acceptance lines re-run on the real engine.
- A task that is to be split is split **in the plan first**: new task files and manifest entries, `plan/check_plan.py`
  green, then the work. Whether to split is judgement, not a line count (§ "Task files", R-369).
- CI runs on every push (`ci.yml`): the build, `cargo test` and `cargo xtask ci`, which includes `cargo xtask plan-check`
  (`plan/check_plan.py`) (R-177). R-656 reads "every push" and "every commit" here as every push to `main` and every
  pull-request head (a pull request's pushes run through `pull_request`, once each), with its skips as R-662 scopes
  them: a head that changes none of Principia's code runs the cheap checks (plan-check, the vocabulary lint, the atlas
  guard) in place of the build and tests, and one that changes no GPU-path file skips the GPU suites; every required check still reports, under its own name (`plan/OPERATIONS.md` § "Merging"). The other suites run at the frequency the corpus gives them (`docs/contracts/principia_parity_contract.md` §6, and
  `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"): unit, property, numerical-gate and native golden suites (the sim-parity
  and codegen suites among them) on every commit; benchmarks and every performance gate on the human's own Mac via
  `prin profile`, at each milestone gate and on demand, never on hosted runners (R-186); GUI screenshots on GUI
  PRs and at the gates; from M8 the Playwright browser suite nightly, on GUI and colour PRs and at each gate, and the
  aggregate survey nightly and before release (R-134). GPU CI runs on GitHub-hosted runners (R-186): lavapipe
  (`mesa-vulkan-drivers`) on `ubuntu-24.04`, pinned (R-375), and `macos-15` (Apple silicon, paravirtual Metal) for the Metal
  correctness suites only, both on every commit. The one self-hosted runner is R-711's, for measurements only and in no required job; from the anchor move it may run the GPU suites nightly and on pushes to `main`, gpu-idle, the weekly proof and the heavy-work comparison, each a named measurement, and no required or heavy CI work moves to it (R-859, `plan/OPERATIONS.md` § "The Windows runner's work (R-859)"); and lavapipe satisfies M4's two-backend check, and a real
  non-Metal GPU gates Paper 2 (R-58). A red CI blocks review.
- The workflows (R-177): `ci.yml` on every push; `nightly.yml` (scheduled) runs the CPU and lavapipe suites and, from M8, the survey, not benchmarks (R-186);
  `screenshot.yml` on GUI PRs, which are PRs touching `crates/gui/**` or `docs/gui/**`; `gate.yml` (`workflow_dispatch`,
  input `milestone`) runs every suite and writes the gate report the milestone checkpoint reviews.
- Whatever its CI frequency, a task's PR shows every one of its acceptance commands run, with their output. A
  `benchmark` or performance-gate command runs on the human's Mac (R-186): the PR includes that run's `prin profile`
  output, or waits for it.

## Task files

Each task is `plan/tasks/<milestone>/<TASK-id>.md`, listed in `plan/tasks.yaml`; the two agree exactly (checked by
`plan/check_plan.py`). Task ids are `TASK-<Mn>-<nn>`, in build order within the milestone.
- **One task is one reviewable PR.** ~500 counted lines is a planning guide for a task file's Size line.
- **No budget gate (R-369):** size is judgement, never a question for the human. A PR that is big because its task is
  big merges normally; one that is big because of sloppy or bloated work is fixed or split, and the reviewers judge
  which. The PR records the reason for its size. Nothing is skipped, deferred or drifts: a split moves every
  requirement to a named task (R-264).
- **What the budget counts (R-211, R-223, R-225):** added and changed lines (a diff's `+` lines) in implementation code
  and the implementer's own tests. Pure deletions, lines moved verbatim (the same text deleted elsewhere in the same
  diff), qa's test commits and `negative_control!` blocks don't count. The count is a measure, not a gate (R-369).
- **Every live requirement is closed by exactly one task, and every task closes at least one.** A task's milestone
  is never later than the milestones of the requirements it closes.
- **Calibration requirements** (R-71) are closed by the task that needs the value. Its deliverable is the proposal
  with evidence, and the human confirms the value at the milestone gate, asked through `/rulings-report` (R-812).
- **Definition requirements** (R-72) are closed by the task that needs the definition. Its deliverable is the doc
  change, reviewed by the physics reviewer.
- **The header fields** are Milestone, Closes, Depends on, Needs (earlier milestones), Reviewers, Pitfalls and Size.
  - **Depends on** names the tasks that must be merged first, including tasks in earlier milestones. A task may start
    earlier, off the approved head of its one unmerged dependency, every other dependency merged, but opens its PR only
    once that dependency has merged (R-388, applied per R-369; § "The unit: one task, one branch, one PR").
  - **Needs** lists the earlier-milestone requirement ids the task builds on. The task closing each one is reachable
    through Depends on.
- **The sections** are Goal, References, Deliverables, Acceptance tests and Notes.
  - Each References bullet is `` `file` § "heading" ``, and the heading must exist.
  - Every closed requirement appears in at least one acceptance line, with its verify method's threshold and fixture.
    Where the corpus gives no threshold, the line names the calibration requirement instead.
- **Reviewers:**
  - `code` and `qa` review every task.
  - `physics` reviews when the task touches any of these: decode, encode, the charts, the integrator, events, payload
    physics fields, a numerical gate, an R-71 calibration of a physical value, an R-72 definition, or a pitfall
    regression.
  - `gui` reviews when the task closes a GUI-area requirement or any `GUI screenshot` verify.
  - `perf` reviews when the task closes a PERF-area requirement or any `benchmark` verify, or touches the frame loop
    or dispatch.
- **Pitfalls** are `PIT-<n>` or `PIT-<n>.<m>`, the section numbers of `docs/read_first/principia_01_pitfalls.md`.

## Conventions: crates and runners (R-146)

This layout is confirmed by R-146. The workspace sits under `crates/`, next to `docs/`.

**Crates:**
- `kernel`: the physics source, compiled twice: f32 SPIR-V → WGSL, and native f64.
- `ledger`: the generation root, including the ledger and the pack/unpack generator.
- `engine`: the scheduler, quadtree, cache, frame loop, dispatch, and the CPU reference driver. It also holds the
  typed contract surfaces, which are defined once in the engine crate (`docs/contracts/principia_gui_state_contract.md`
  §1).
- `render`: the fragment side and the display chain.
- `gui`: the egui dev GUI.
- `validation`: the validation harness.
- `prin`: the CLI.

**Other directories:**
- `xtask`: the runners.
- `web/`: v1 web's page and shell, the whole native app compiled to run in the browser (R-571; *was:* "the browser
  product"). Its unit tests run under Vitest (`npm --prefix web test -- <filter>`, the package's test
  script running `vitest run`), and its browser suites under Playwright (R-146).
- `fixtures/golden/` and `fixtures/gates/`: the test fixtures.
- `reel/specs/`: the spotlight reel's curated specs, `NN_name.principia-seq.json`, which `cargo xtask reel` renders at
  release into the film and its manifest, build outputs that are not checked in (R-578, applied per R-369).

**Each verify method has its runner:**

| Verify method | Runner |
|---|---|
| `unit test` | `cargo test -p <crate>` |
| `property test` | `cargo test -p <crate>`, using proptest |
| `golden image` | `cargo xtask golden <suite>`, rendered with native wgpu offscreen from M1 against the baselines in `fixtures/golden/`; at M8 the Playwright browser suite, on Chromium and WebKit (R-149), checks against the same baselines within tolerance, and no baseline is re-baselined without a gate decision (R-110) |
| `numerical gate` | `cargo xtask gate <gate>`, with fixtures in `fixtures/gates/` |
| `benchmark` | `cargo xtask bench <bench>`, run on the human's own Mac (compared with `prin profile diff`), never on a hosted runner (R-186) |
| `GUI screenshot` | `cargo xtask screenshot <suite>` (native wgpu offscreen; on GUI PRs and at the gates, R-110), compared against `docs/gui/design/NN_*.png` for layout only (R-68); a surface with no artboard is checked by presence only until the M8 dev GUI (`decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*") |
| `review checklist` | the named reviewer's checklist, or a CI lint that installs the check |

`cargo xtask plan-check` runs `plan/check_plan.py`.

**Running the suite locally (R-231):** the suite runs through [cargo-nextest](https://nexte.st), pinned to the version
CI installs, and doctests, which nextest does not run, through `cargo test --doc`:

```sh
cargo install cargo-nextest --version 0.9.146 --locked
cargo nextest run --workspace
cargo test --workspace --doc
```

`cargo xtask ci` runs every control and the other per-push checks (R-235). (This was the README's "Running the tests"
until the README of 18 Oct 2026, which has no build instructions; `xtask/tests/nextest.rs` checks this run against CI's
pinned version.)

### Agents' test runs: cargo nextest

Every agent, and the orchestrator, runs tests locally with `cargo nextest run` in place of `cargo test`, as R-231 and
CI already do. Applied per R-369 and R-812 on 9 Nov 2026, against a criterion written before measuring (at least 10%
faster than `cargo test` on the same warm target, by more than the spread between rounds): the test phase took kernel
151 / 129 s → 85 / 65 s and ledger 84 / 107 s → 74 / 46 s, 37–46% faster, with no result changed. `cargo test` runs a
crate's test binaries one after another; nextest runs every test of every binary at once, one process per test, as CI.
- **The same arguments.** `cargo nextest run` takes `cargo test`'s package, target, feature and profile arguments as
  they are (`-p`, `--workspace`, `--lib`, `--test <name>`, `--features`, `--release`) and its positional name filters
  (`cargo nextest run -p gui mock_keyboard`). After `--`, libtest's `--exact`, `--skip`, `--ignored` and
  `--include-ignored` work as they are; `-- --nocapture` becomes `--no-capture`, and `-- --test-threads N` becomes
  `--test-threads N`. Run it on the default profile, with no `NEXTEST_PROFILE` set (a mutants run's `mutants` profile
  aside, `plan/OPERATIONS.md` § "Reviewers"); it runs every test, and keeps `.config/nextest.toml`'s overrides that run
  the tests that run cargo alone.
- **Doctests.** nextest runs none. Where `cargo test` would have run them (no `--lib`, `--test` or `--bins` selection)
  and the crate has a library, also run `cargo test --doc` with the same package, features and filter. `prin` has no
  library, and `--doc` fails on it.
- **A task's acceptance commands.** A task file's `cargo test …` acceptance command is run as its nextest equivalent,
  plus its `--doc` run where the line above asks for one. The PR shows the command actually run, with its output,
  beside the task's command it stands for (`cargo test -p kernel --test roundtrip`, run as `cargo nextest run -p
  kernel --test roundtrip`). The task files are not rewritten; this rule covers them, and the reviewer checklists'
  `cargo test` lines alike.
- **A filter that selects nothing.** nextest fails a run that selects no test (exit 4), where `cargo test` passes with
  0 tests. Don't pass `--no-tests=pass`: a command that selects nothing tests nothing. Fix the filter; if the task's
  own command selects nothing, that is an escalation (§ "Escalation").
- **The built kernel first.** The tests that read the built kernel (the `gpu-kernel` profile's list in
  `.config/nextest.toml`: engine's and validation's `*_gpu` tests, `decode_cross_backend`, `codegen_selftest`,
  `bringup`, `toolchain` and the rest) need `cargo xtask build-kernel` run first, through the queue, in the same
  worktree. It writes `target/spirv/kernel.spv` and `target/spirv/kernel.wgsl` under the worktree's root, whatever
  `CARGO_TARGET_DIR` is; they are gitignored, so a fresh worktree has neither, and a change under `crates/kernel/`
  needs it run again. Without them those tests fail with "target/spirv/kernel.wgsl: run `cargo xtask build-kernel`
  first".
- **What stays on `cargo test`:**
  - doctests, `cargo test --doc` (above);
  - the stand-in soak (`stand-in-soak.yml`, REQ-SYS-070): its 50 runs of `cargo test -p xtask --test <name>` test the
    fork/exec race among one binary's tests run as threads of one process, which nextest's process per test would
    remove (`xtask/tests/nextest.rs`, `LIBTEST_SOAK`); a local repeat of it runs the workflow's own command;
  - the `cargo test` that tools and tests run themselves, as a child: `cargo xtask controls`, which keeps its own cargo
    invocations (R-231); the test listings `cargo test <args> -- --list` (`xtask/tests/nextest.rs`'s no-test-dropped
    check, `gate.yml`'s listing, REQ-SYS-079); and the tests that run `cargo test` as their subject (validation's
    `qa_TASK-M0-24`, `qa_TASK-M0-25` and `qa_TASK-M0-26`, and xtask's `ci_debug_info`, among them). An agent runs
    those tests themselves under nextest, as CI does; what they spawn is not rewritten.

  The tests that re-run their own binary by name with libtest's arguments (`current_exe()` with `--exact` and
  `--test-threads 1`: `xtask/tests/lint_no_alloc.rs`, `lint_wgsl_rules.rs`, `lint_compute_pipelines.rs` and
  `qa_ops_golden_harness_ci.rs`; `crates/render/tests/pipeline_teardown.rs` and `qa_TASK-M2-42.rs`;
  `crates/validation/tests/qa_TASK-M0-33_type.rs`, `qa_TASK-M0-48.rs` and `support/scratch_guard.rs`;
  `crates/prin/tests/qa_TASK-M0-18.rs`), and those that took in-process locks under `cargo test` (`WORKSPACE_TARGET`,
  `WORKSPACE_RUN`), run under nextest as they do in CI, so they are not on this list.

## The review loop

1. **The implementer** opens the PR once the task's acceptance tests pass locally and it has run `cargo mutants
   --in-diff` on its own diff against `origin/main` and tested the boundary of every comparison and match arm it adds
   (R-388; `plan/OPERATIONS.md` § "Reviewers", its **Mutants** item). Since the owner's CI model (R-908, R-909) no
   pull-request run mutates, so that local run is the pull request's mutation check. *Was:* "CI's mutants gate stays
   the authority."
2. **Every reviewer named in the task** (`plan/reviewers/<name>.md`) reviews against their checklist and the task's
   References. The reviewers are agents (R-175): each posts a PR review headed `VERDICT: APPROVE <role>` or
   `VERDICT: CHANGES <role>`, since GitHub won't let one account approve its own PR. `code` and `qa` review every task; `physics`, `gui` and `perf` when the task names them.
   Reviewers never share a checkout: each gets its own git worktree at the PR head and its own `CARGO_TARGET_DIR`,
   which the orchestrator makes before dispatch and removes afterwards (R-219).
   A reviewer runs the acceptance commands and the tests the diff touches plus their dependents, and checks CI is green
   on the head; a full local run is only for a cross-cutting change (R-229). A head's green is the PR tier's
   (`plan/OPERATIONS.md` § "What CI runs on a pull request: the PR tier"); the full suite runs on each merge to `main`.
   *Was (R-684):* "Before `merge-ready`, a code head's green is the quick tier's, the full-only checks pending; the
   full suite then runs on the approved head."
3. **Findings cite file and line** — of the diff, or of the doc section a finding rests on (`file` § "section"). A
   finding without a citation isn't actionable and is returned to its author.
4. **The implementer fixes** each finding and replies on the finding with the fixing commit.
5. **Every reviewer re-checks** — not only the one who raised a finding: a fix can break another reviewer's check.
   Each reviewer approves explicitly, with a new `VERDICT: APPROVE <role>` review on the latest commit.
6. Merge when `ci` and `reviews-complete` are green: `reviews-complete` (`cargo xtask reviews-check`) passes only when
   every role the task file names has approved on the latest commit. The orchestrator merges (R-369;
   `plan/OPERATIONS.md` § "Autonomy (R-369)").
7. After the merge, the branch is cleaned up (R-345): GitHub's "Automatically delete head branches" deletes its remote
   branch, and the orchestrator confirms it is gone rather than deleting it (R-357); its local branch is deleted, its
   worktrees and their target directories are removed, and stale remote refs and worktree entries are pruned
   (`git fetch --prune`, `git worktree prune`). Only a branch fully merged into `main` is deleted. PRs are not
   stacked; if a stack is ever needed, the child PR is retargeted to `main` before the PR below it merges, because
   merging deletes the lower branch at once and deleting a base branch closes its child PRs (CLAUDE.md § Git, R-362).

**Advisory bot reviews (R-651, a standing rule; R-910).** Codex and Gemini CLI review alongside the named reviewers,
advisory only: they never replace the named reviewers' approvals, and `reviews-complete` doesn't count them. Copilot is
never requested (R-910). The orchestrator asks each bot for a fresh review (`@codex review`, `@gemini-cli /review`):
- **before the merge, on the final head:** step 6 waits for a review from each bot on the exact SHA being merged, and
  re-requests a bot whose last review is on an older head;
- **after every substantive push:** new logic, physics or algebra, a fix responding to review findings (whatever it
  changes), or a rebase or merge that changes code; only a docs-only typo push that answers no finding needs none;
- **periodically on a long-running PR:** at least once a working day while it is open, and every few pushes on a
  large one, so problems surface partway through rather than at the end.

Under limits the order is pre-merge, then post-push, then periodic. A bot is unavailable when it hasn't reviewed the
exact SHA within its limit, counted from the request (Codex 25 minutes, Gemini CLI 30), errors, or refuses
on quota or rate limit; it is noted on the PR and the merge goes ahead with the others; the owner is told if pre-merge
reviews start being skipped. Every bot finding is closed out before the merge: verified against the code, the spec or
the physics, then fixed (the reply names the commit), answered with why it doesn't apply (wrong, out of scope, already
covered), or moved to a tracked task or RQ; then its thread is resolved. No thread is resolved without a reply, a wrong
finding is answered, never "fixed" to silence it, and a PR with an unresolved bot thread does not merge
(`plan/OPERATIONS.md` § "Merging").

## Human checkpoints: the milestone gates

A milestone exits only at a human checkpoint. Before it:
- every task of the milestone is merged, and its exit gate (`plan/MILESTONES.md`) passes in CI: every requirement
  listed, and every earlier gate still green;
- every **calibration** requirement of the milestone (`kind: calibration`, R-71) has its proposed value, the evidence
  and the reviewer's check in the PR that closed it; **the human confirms each value at the gate**, and it is then
  recorded in `decisions.md`. Until then CI uses the proposed value provisionally, and the test or gate report marks
  it provisional (R-182). An unconfirmed calibration blocks the gate. A proposed value in the constants register's
  crates (`kernel`, `ledger`, `engine`) is a register entry marked provisional until its gate, read from the register,
  never a function in its place (R-427; dd_generation_root §3.8), and the gate report lists those entries;
- every **definition** requirement (`kind: definition`, R-72) has its doc change merged with the physics reviewer's
  approval.
The human reviews the gate report and either passes the milestone or rules on what blocks it; the question reaches them
through `/rulings-report`, the gate report its evidence, and work outside the gate carries on (R-812). No work in the
next milestone merges before the gate passes (tasks may be prepared on branches), except by R-415's test: a task may merge
before an earlier milestone's gate if its dependencies have merged and it relies on no value or behaviour that gate
calibrates or could change; the PR records the test as applied (`plan/OPERATIONS.md` § "Autonomy (R-369)").
R-390's GUI track (TASK-M6-24 to TASK-M6-29, with TASK-M6-30 and TASK-M6-31 from R-406 and R-409, and TASK-M6-32 to
TASK-M6-36 from R-450 to R-453, TASK-M6-37 from R-451 B11, TASK-M6-38 to TASK-M6-44 from R-457 to R-463, and TASK-M6-45 from R-466) is the exception:
it runs now, in parallel with the physics and renderer chain, and each of its tasks merges once its reviews pass and CI
is green, without waiting for the gates of M1 to M5.
R-627 (27 Oct 2026) extends the exception to M8's GUI-track tasks (TASK-M8-05, TASK-M8-06, TASK-M8-07, TASK-M8-13, TASK-M8-19, TASK-M8-20, TASK-M8-23 and TASK-M8-24, and TASK-M8-86 and TASK-M8-96, the v1
web skeleton and its measurements): each merges once its reviews pass and CI is green without waiting for the gates of M3 to M6; the gates of M2
and M7, which R-627 does not name, by R-415's test above, recorded in the PR. Their wiring tasks, TASK-M8-88 to
TASK-M8-95, run on the real engine and wait for the gates as any other task does.
*9 Nov 2026 (R-917, R-918):* M2 is the whole non-visual chart system (R-916), and its gate, "fully working and
tested", is the checklist G1 to G14 in `plan/MILESTONES.md` § "M2 — The chart system: charts, decode and encode, and the
non-visual chart core", the owner's lock-with-morphs test among it. Until it passes, no new integrator or visual task
is dispatched: work in flight or approved finishes, and fixes, CI and tooling work and what M2's own tests need go
ahead. The GUI track's exception above (R-390, R-627) doesn't hold for new dispatch; a track task already started
finishes. R-415's test doesn't bring forward a task R-917 holds.
*30 Oct 2026 (R-649, R-650):* a replication or verification target is a measurement, never a gate: an external
target's outcome (PASS, EXPLAINED DIVERGENCE or UNEXPLAINED) blocks no merge and no milestone. The internal
correctness tests, the GPU-vs-CPU agreement checks within their stated f32 tolerances and the anchor smoke tests
stay in their milestones' gates and block as before. M9, external replication, follows M8 and has no gate: it exits
on its report (REQ-VAL-340), which the owner reviews, and the calibrations of its targets are confirmed by the human
with that report, not at a gate. Scoring waits for M9, and R-415's test above does not bring an M9 task forward.
Source ingest and transcription (data PRs under `docs/incoming/`) merge whenever their reviews pass. An early peek
scores a target as information only, in `docs/verification/peek_log.md` (`docs/design/principia_dd_replication.md`
§ "12.5 Peeks"), never a gate.

## Escalation

Nothing is chosen by the implementer or a reviewer where the corpus doesn't decide:
- **Reviewers disagree**, and the corpus doesn't settle it → a `REVIEW_QUEUE.md` entry citing both positions and the
  sections they rest on; the human rules; the ruling is recorded in `decisions.md`.
- **A corpus conflict or silence turns up** during a task (two sections disagree; a value, name or behaviour the task
  needs isn't given) → a `REVIEW_QUEUE.md` entry with file, section and the quoted text; the affected requirement gets
  `rq:` in `plan/requirements.yaml`. A value becomes a calibration requirement (R-71) and a definition a definition
  requirement (R-72) once ruled so; otherwise the task waits.
- **A design change** goes into the docs first, with an RQ and a ruling (the porting rule); then the plan follows;
  then the code.
An entry's form, which the plan check holds it to: a heading `## RQ-n: <question>`, and in its body a
`- **File, section:**` field and a `- **Applied meanwhile:**` field (`none`, if nothing is).
The task stays open, blocked, until the human rules. Work continues on other tasks. An open entry reaches the human
through `/rulings-report`, never as a blocking prompt; an entry that is a two-way door isn't
held for them, and the orchestrator and the reviewers decide it and report it (R-812).

## No deferral

Nothing is deferred silently. Deferring anything — a requirement, an acceptance test, a finding, a part of a task —
needs a `decisions.md` entry made by the human. A reviewer finding cannot be waived by the implementer or by another
reviewer. A requirement leaves a gate only by a ruling (retired with `retired: <ruling>`, never deleted).

## Keeping the plan true

- `plan/requirements.yaml` changes through `plan/tools/reqio.py` patches or by hand, never by renumbering; ids are
  permanent.
- `plan/coverage.md` and the MILESTONES gate blocks are generated (`plan/tools/coverage.py`, `plan/tools/milestones.py`)
  and checked in CI.
- Every commit that changes `docs/`, `decisions.md` or `plan/` ends with its "Removed lines" note: each removed line
  is "reworded, kept at <file:line>" or "stale value, replaced by <ruling>".
