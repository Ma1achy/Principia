# The fresh start

This repository was seeded on 20 Nov 2026 from `Ma1achy/principia` at `ea49959` (the old repo, kept as
`principia-archive` with its full history, rulings and PRs). The code, tests, fixtures, design docs and
validation moved over; the process around them didn't.

## What moved, unchanged
All seven crates and their tests; `fixtures/`; `docs/contracts`, `docs/design`, `docs/read_first`,
`docs/notes`, `docs/gui`, `docs/reference`, `docs/incoming` (papers that may be committed), `docs/experiments`,
`docs/readme`; `formal/`; `web/`; the README; the toolchain and cargo config; the Windows runner's workflows
(`win-gpu.yml`, `win-retry.yml`, `runner-proof.yml`) and scripts; the README's progress grid (`progress.yml`,
`scripts/progress_banner.py`, now drawing `plan/tasks.yaml` by checkpoint).

## What changed
- `decisions.md` and `requirements.yaml` are frozen in `docs/history/`. Calibrated constants cite their ruling
  there, and the validation gates read thresholds by requirement id, so the code points there now.
- New: `CLAUDE.md`, `plan/WORKFLOW.md`, `plan/PLAN.md`, `plan/tasks.yaml`, `plan/MACHINES.md`,
  `plan/KNOWN.md`, three agent files, `AGENTS.md`, and a short `ci.yml`.
- `xtask`: the plan, review, PR, gate-report, mutation and Pages modules are gone, with their subcommands.
- `xtask/tests/runner_security.rs` replaces the old 8,800-line workflow-structure test (`ci_self_hosted.rs`) with
  what matters: self-hosted jobs only under push to main, schedule and dispatch; the repo and actor guard; a
  read-only token; no secrets; every action pinned to a commit. Each check has a control. The PC's
  `win-self-check` guard step now runs it.
- Golden baselines: a new row in `fixtures/golden/BASELINES.md` names the PR that made it (`#n`); old rows keep
  citing their ruling, checked against `docs/history/`. A baseline still can't change without a named reason.
- The WSL image's nextest is checked against `ci.yml` (the hosted mutation workflow it was checked against is
  gone). `contain.rs` lost its check that mutation workflows run contained, since there are none.
- Two test arms that checked the old plan's files were cut: `part6_catalogue`'s "a task closes this
  requirement" arm, and one doc in `qa_TASK-M5-01`'s list of docs stating QuadReduction's size.

## Not moved
`workbench/` (prototype scratch), `docs/archive`, most of `plan/` (milestones, OPERATIONS, rulings tooling, the
finished tasks' files; every unfinished product task's file did move, as `plan/tasks/`),
`REVIEW_QUEUE.md`, `open-questions.md`, the six old agent roles, the rulings commands, and 13 workflows (PR
checks, reviews, gates, mutants, nightly, Pages, retry, stand-in soak, formal). The Lean proof's workflow can
come back as on-request when wanted; the proof itself is in `formal/`.

Dropped tests, all in `xtask/tests/`, each testing process tooling or the structure of a workflow that no
longer exists: `ai_review_rules`, `ci_*` (except `ci_partition`, `ci_win_retry`, `ci_wsl_route`,
`ci_wsl_cache`, `ci_win_no_wsl`, which test kept code), `cloud_setup`, `gate_provisional`, `gate_report`,
`gate_report_by_hand`, `gemini_review`, `merge_ready`, `mutants_*`, `nextest`, `pages_workflow`,
`plan_check`, `pr_check`, `progress_banner`, `reviews_check`, `qa_R-375`, `qa_R-485_print_graph_coverage`,
`qa_R-656/662/677/790` (CI tiers), `qa_ci_*`, `qa_tooling_*`, `qa_gemini_*`, `qa_mutants_*`, `qa_ops_*`,
`qa_fix_rq_union_control`, `qa_wsl_cpu_mutants*`, `qa_sampler_*` (the deleted disk sampler),
`qa_TASK-M1-18*` and `qa_TASK-M2-54` (milestone listings of provisional constants), `qa_TASK-M8-86*` (the Pages
deploy), `qa_TASK-M2-49`, `qa_TASK-M6-53`, `qa_TASK-M0-14_*`, `qa_TASK-M0-88` (the old CI's job layout), and the
M0 tooling tasks' qa files (M0-02, 03, 19, 20, 22_r235, 23*, 29, 33, 36, 37, 38, 42, 45, 49, 52, 53, 54, 60, 70, 71, 72,
76; M0-70's subject, `ci_self_hosted`, is gone, and `runner_security` covers the guard it tested). The full list is in the archive's tree at
`ea49959`.

## Architecture, reviewed at import

**Sound:**
- The crate graph is acyclic and layered: `ledger` (generated layouts, the one source of truth for bit layouts
  and constants) → `kernel` (physics, `no_std`, shared by CPU and GPU) and `render` → `engine` → `gui`, `prin`.
  `validation` is the test harness and depends on everything; nothing depends on it except in tests.
- The CPU and GPU run one physics source, so they cannot drift apart by construction.
- The GUI is written against a trait (`EngineSide`, `crates/gui/src/side.rs`) with a mock and a real side.
  That is the seam C1 fills.

**Where it could turn into spaghetti, and what the plan does about it:**
- **The engine has no pipeline.** It holds 20-odd top-level modules, one per old task (lookup, lock, nav,
  decode dispatch, forward table, first slice...), and nothing that turns a `SimConfig` into a field. Left
  alone, the GUI would call `first_slice` and the kernel directly. TASK-C1-01 adds one entry point,
  `engine::field`, behind a backend trait (CPU now, GPU at C4), and the GUI calls only that. TASK-C2-06 groups
  the engine's modules by what they serve.
- **`first_slice` is a measurement harness**, not a renderer. Keep it as the reference that the field backends
  are checked against, not as the path the app uses.
- **The GUI is a third code and two-thirds tests:** about 25k lines of app against about 50k lines of tests,
  almost all on the mock. Fine, but as the real side lands, test the real side, and let the mock's tests shrink
  rather than grow.
- **Large files:** `gui/src/app.rs` (1.6k lines), `gui/src/explore/compass.rs` (2.1k),
  `validation/src/cross_backend.rs` (2.9k). Split when next touched, not before.
- **728 test files**, many one binary each, which makes test builds slow and large on disk. Ledger already uses
  one test binary (`tests/all/`); do the same for the other crates when touching their tests.
- **Comment noise:** 10-17% of source lines are comments, many citing old ruling ids. Readable once you know
  `docs/history/` exists; tidy a passage when you next edit it.
