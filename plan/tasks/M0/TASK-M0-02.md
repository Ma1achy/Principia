# TASK-M0-02 — `cargo xtask plan-check` in CI, and the plan-order checks

- **Milestone:** M0
- **Closes:** REQ-SYS-007, REQ-SYS-008, REQ-SCHED-001, REQ-TOOL-004
- **Depends on:** TASK-M0-01, TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
`cargo xtask plan-check` runs `plan/check_plan.py` (which also runs `coverage.py`, `milestones.py` and `reviewer_lists.py` with `--check`) and is registered in `cargo xtask ci`, so a plan that loses a requirement, closes one twice, cites a section that doesn't exist — including any section of an archived file, which the citable index does not hold — or disagrees with its task files turns CI red. The plan-order rules the corpus sets are then held by that check and by review of the plan: the quadtree's three layers in order (deep_zoom §3: flat grid → quad cache with ancestor fallback → adaptive refinement), the Phase-0 debug tooling order (0a generation root before 0b/0c, presets as their dependencies land, R-75), and no work item for a parked philosophy §7 item beyond the Real-generic payload.

## References
- `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree"
- `docs/design/principia_debug_tooling_plan.md` § "Build order within Phase 0 (the only forced staggering)"
- `docs/design/principia_debug_tooling_plan.md` § "Principle"
- `docs/design/principia_core_design.md` § "Through-line"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `docs/read_first/principia_00_philosophy.md` § "7.7 The rule for this section"
- `docs/read_first/principia_00_philosophy.md` § "7. Parked — deliberate post-1.0"
- `docs/read_first/principia_INDEX.md` § "Principia — document index and reading order"
- `docs/read_first/principia_INDEX.md` § "Archived — record only, do not implement"
- `decisions.md` § "R-112 — The archived briefs' standing parts are superseded *(closes RQ-81)*"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-179 — Sim data is per-sample payload; QuadReduction is allowed *(closes A4, T3)*"
- `decisions.md` § "R-184 — The minor fixes *(closes G5, G7, G8, A6, A7, A8, C5, C7)*"

## Deliverables
- `xtask/src/plan_check.rs` — `cargo xtask plan-check`: runs `python3 plan/check_plan.py` from the repo root, streams its output, exits with its status; a clear error if `python3` or PyYAML is missing.
- `.github/workflows/ci.yml` — Python and PyYAML set up; `plan-check` registered in `cargo xtask ci`.
- Negative controls for this task's xtask tests, registered with `negative_control!` through `xtask`'s dev-dependency on `crates/validation` (R-176).
- `plan/check_plan.py` — a reference or requirement source into `docs/archive/` or an `ARCHIVE_` file reports "cites archived file <path>" rather than "doesn't exist" (R-184).
- `xtask/tests/plan_check.rs` — runs the check on a temporary copy of `plan/` + the corpus with (a) one task removed from `tasks.yaml`, (b) a requirement source pointing at `docs/archive/principia_ARCHIVE_dd_refinement_criterion_v0.md`, (c) a task reference to an `ARCHIVE_` brief, and asserts a non-zero exit naming the fault each time.
- The R-112 check-and-port record, in the PR description: each standing obligation of the structure-criterion brief's §4–4.6 (the slippy map: breadth-first, frame budget) and the kernel-build brief's §5 (verification gates), with the consolidated-doc section that holds it (deep_zoom §3, scheduler, parity); any obligation no consolidated doc holds is ported into the fitting one in the same PR, as a doc change for review (the INDEX's Archived rows were conformed in step 7).

## Acceptance tests
- `cargo xtask plan-check` — exits 0 on the PR head; runs in CI on every push.
- `cargo test -p xtask plan_check` — a requirement source or task reference into `docs/archive/` or an `ARCHIVE_` brief fails plan-check (REQ-SYS-008: nothing sourced from or citing an archived file).
- Review checklist (code §7) on the R-112 record: every standing obligation of the two briefs maps to a consolidated-doc section or was ported into one (REQ-SYS-008, R-112).
- Review checklist (code §7) on `plan/MILESTONES.md` and `plan/tasks.yaml`: deep_zoom layer 0 is in M4, layer 1 in M5 and layer 2 in M6, in that order, and no M5 task flows sim data GPU→CPU, where sim data is per-sample `SimState` payload and `QuadReduction`, a reduction, is the one permitted automatic return (R-179, R-142) (REQ-SCHED-001).
- Review checklist on `plan/MILESTONES.md` and `plan/tasks.yaml`: the generation root and codegen self-test (0a) are M0; the synthetic harness, field views and the bring-up mode (0b, 0c) are M1; the UV, DECODE and ROUNDTRIP presets land as their dependencies do (R-75); structural views precede the scheduler logic (REQ-TOOL-004).
- Review checklist (code §7) on `plan/tasks.yaml`: no task builds or half-starts extended precision, tiled prebake, dump tiers, headless datasets, the regularisation matrix or the GPU-port lever, other than the Real-generic payload (TASK-M0-14) (REQ-SYS-007).

## Notes
- `plan/check_plan.py` is the checker drafted at step 7; it is at that path.
- REQ-SCHED-001 and REQ-TOOL-004 also say "code history shows" the order: at each later milestone gate the gate review re-reads the history against the same order; this task installs the check that holds the plan.
- RQ-81 ruled: R-112 — the two briefs' standing parts (structure-criterion §4–4.6, kernel-build §5, in `docs/experiments/briefs/`) are superseded by the consolidated docs and never cited; this task checks each obligation is held there and ports any that isn't.
