# TASK-M0-02 — `cargo xtask plan-check` in CI, and the plan-order checks

- **Milestone:** M0
- **Closes:** REQ-SYS-007, REQ-SYS-008, REQ-SCHED-001, REQ-TOOL-004
- **Depends on:** TASK-M0-01
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

## Deliverables
- `xtask/src/plan_check.rs` — `cargo xtask plan-check`: runs `python3 plan/check_plan.py` from the repo root, streams its output, exits with its status; a clear error if `python3` or PyYAML is missing.
- `.github/workflows/ci.yml` — Python and PyYAML set up; `plan-check` registered in `cargo xtask ci`.
- `xtask/tests/plan_check.rs` — runs the check on a temporary copy of `plan/` + the corpus with (a) one task removed from `tasks.yaml`, (b) a requirement source pointing at `docs/archive/principia_ARCHIVE_dd_refinement_criterion_v0.md`, (c) a task reference to an `ARCHIVE_` brief, and asserts a non-zero exit naming the fault each time.

## Acceptance tests
- `cargo xtask plan-check` — exits 0 on the PR head; runs in CI on every push.
- `cargo test -p xtask plan_check` — a requirement source or task reference into `docs/archive/` or an `ARCHIVE_` brief fails plan-check (REQ-SYS-008: no requirement sourced from an archived file).
- Review checklist (code §7) on `plan/MILESTONES.md` and `plan/tasks.yaml`: deep_zoom layer 0 is in M4, layer 1 in M5 and layer 2 in M6, in that order, and no M5 task flows sim data GPU→CPU (REQ-SCHED-001).
- Review checklist on `plan/MILESTONES.md` and `plan/tasks.yaml`: the generation root and codegen self-test (0a) are M0; the synthetic harness, field views and the bring-up mode (0b, 0c) are M1; the UV, DECODE and ROUNDTRIP presets land as their dependencies do (R-75); structural views precede the scheduler logic (REQ-TOOL-004).
- Review checklist (code §7) on `plan/tasks.yaml`: no task builds or half-starts extended precision, tiled prebake, dump tiers, headless datasets, the regularisation matrix or the GPU-port lever, other than the Real-generic payload (TASK-M0-14) (REQ-SYS-007).

## Notes
- `plan/check_plan.py` is the checker drafted at step 7; it is at that path.
- REQ-SCHED-001 and REQ-TOOL-004 also say "code history shows" the order: at each later milestone gate the gate review re-reads the history against the same order; this task installs the check that holds the plan.
- See Gaps: the two archived-file exceptions (structure-criterion brief §4–4.6, kernel-build brief §5) live in `docs/experiments/briefs/`, which the citable index does not cover.
- Waits on RQ-81 (`REVIEW_QUEUE.md`): The archived briefs' standing parts aren't citable.
