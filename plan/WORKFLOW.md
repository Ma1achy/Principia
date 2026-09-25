# Workflow — how a task becomes merged code

How the build runs from `plan/`. The docs stay the authority (plan/MILESTONES.md, "Assumptions"); this file says who
does what, in what order, and what stops the line.

## The unit: one task, one branch, one PR

- Each task in `plan/tasks.yaml` is built on its own branch, `task/<TASK-id>` (e.g. `task/TASK-M3-04`), and merged
  through one PR titled `<TASK-id>: <title>`. Nothing else goes in that PR.
- A task starts only when every task in its **Depends on** list is merged.
- The PR description lists the requirement ids the task closes and, for each, the acceptance command that
  demonstrates it, with its output. It links the task file.
- A task that turns out bigger than one reviewable PR (roughly 500 lines of change) is split **in the plan first**:
  new task files and manifest entries, `plan/check_plan.py` green, then the work.
- CI runs on every push: the build, `cargo test` and `cargo xtask plan-check` (`plan/check_plan.py`). The other
  suites run at the frequency the corpus gives them (`docs/contracts/principia_parity_contract.md` §6, and
  `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"): unit, property, numerical-gate and native golden suites (the sim-parity
  and codegen suites among them) on every commit; benchmarks nightly and at each milestone gate; GUI screenshots on GUI
  PRs and at the gates; from M8 the Playwright browser suite nightly, on GUI and colour PRs and at each gate, and the
  aggregate survey nightly and before release (R-134). GPU CI is a self-hosted Apple-silicon runner (Metal)
  plus lavapipe as the second backend, both on every commit; lavapipe satisfies M4's two-backend check, and a real
  non-Metal GPU gates Paper 2 (R-58). A red CI blocks review.
- Whatever its CI frequency, a task's PR shows every one of its acceptance commands run, with their output.

## Task files

Each task is `plan/tasks/<milestone>/<TASK-id>.md`, listed in `plan/tasks.yaml`; the two agree exactly (checked by
`plan/check_plan.py`). Task ids are `TASK-<Mn>-<nn>`, in build order within the milestone.
- **One task is one reviewable PR:** roughly ≤ 500 lines of change.
- **Every live requirement is closed by exactly one task, and every task closes at least one.** A task's milestone
  is never later than the milestones of the requirements it closes.
- **Calibration requirements** (R-71) are closed by the task that needs the value. Its deliverable is the proposal
  with evidence, and the human confirms the value at the milestone gate.
- **Definition requirements** (R-72) are closed by the task that needs the definition. Its deliverable is the doc
  change, reviewed by the physics reviewer.
- **The header fields** are Milestone, Closes, Depends on, Needs (earlier milestones), Reviewers, Pitfalls and Size.
  - **Depends on** names the tasks that must be merged first, including tasks in earlier milestones.
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
- `web/`: the browser product. Its unit tests run under Vitest (`npm --prefix web test -- <filter>`, the package's test
  script running `vitest run`), and its browser suites under Playwright (R-146).
- `fixtures/golden/` and `fixtures/gates/`: the test fixtures.

**Each verify method has its runner:**

| Verify method | Runner |
|---|---|
| `unit test` | `cargo test -p <crate>` |
| `property test` | `cargo test -p <crate>`, using proptest |
| `golden image` | `cargo xtask golden <suite>`, rendered with native wgpu offscreen from M1 against the baselines in `fixtures/golden/`; at M8 the Playwright browser suite, on Chromium and WebKit (R-149), checks against the same baselines within tolerance, and no baseline is re-baselined without a gate decision (R-110) |
| `numerical gate` | `cargo xtask gate <gate>`, with fixtures in `fixtures/gates/` |
| `benchmark` | `cargo xtask bench <bench>` |
| `GUI screenshot` | `cargo xtask screenshot <artboard>` (native wgpu offscreen; on GUI PRs and at the gates, R-110), compared against `docs/gui/design/NN_*.png` for layout only (R-68); a surface with no artboard is checked by presence only until the M8 dev GUI (`decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*") |
| `review checklist` | the named reviewer's checklist, or a CI lint that installs the check |

`cargo xtask plan-check` runs `plan/check_plan.py`.

## The review loop

1. **The implementer** opens the PR once the task's acceptance tests pass locally.
2. **Every reviewer named in the task** (`plan/reviewers/<name>.md`) reviews against their checklist and the task's
   References. The reviewers are agents (R-175): each posts a PR review headed `VERDICT: APPROVE <role>` or
   `VERDICT: CHANGES <role>`, since GitHub won't let one account approve its own PR. `code` and `qa` review every task; `physics`, `gui` and `perf` when the task names them.
3. **Findings cite file and line** — of the diff, or of the doc section a finding rests on (`file` § "section"). A
   finding without a citation isn't actionable and is returned to its author.
4. **The implementer fixes** each finding and replies on the finding with the fixing commit.
5. **Every reviewer re-checks** — not only the one who raised a finding: a fix can break another reviewer's check.
   Each reviewer approves explicitly, with a new `VERDICT: APPROVE <role>` review on the latest commit.
6. Merge when `ci` and `reviews-complete` are green: `reviews-complete` (`cargo xtask reviews-check`) passes only when
   every role the task file names has approved on the latest commit. The human merges, or a merge bot does (R-175).

## Human checkpoints: the milestone gates

A milestone exits only at a human checkpoint. Before it:
- every task of the milestone is merged, and its exit gate (`plan/MILESTONES.md`) passes in CI: every requirement
  listed, and every earlier gate still green;
- every **calibration** requirement of the milestone (`kind: calibration`, R-71) has its proposed value, the evidence
  and the reviewer's check in the PR that closed it; **the human confirms each value at the gate**, and it is then
  recorded in `decisions.md`. An unconfirmed calibration blocks the gate;
- every **definition** requirement (`kind: definition`, R-72) has its doc change merged with the physics reviewer's
  approval.
The human reviews the gate report and either passes the milestone or rules on what blocks it. No work in the next
milestone merges before the gate passes (tasks may be prepared on branches).

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
The task stays open, blocked, until the human rules. Work continues on other tasks.

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
