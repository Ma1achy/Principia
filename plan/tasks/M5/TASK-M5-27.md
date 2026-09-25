# TASK-M5-27 — Precompiled pipelines: no compile on the gesture path

- **Milestone:** M5
- **Closes:** REQ-RENDER-040, REQ-SYS-035
- **Depends on:** TASK-M5-21, TASK-M4-07
- **Needs (earlier milestones):** REQ-PERF-006, REQ-PERF-007, REQ-RENDER-010, REQ-RENDER-011
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~220 lines

## Goal
A gesture never reaches the compiler: pipelines are created with the async creation APIs, the finite variant
set is precompiled at startup, the last valid pipeline is kept while a new one compiles, moving values are uniforms, and
re-integrating changes compile lazily within their own cost.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"
- `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"

## Deliverables
- `crates/engine/src/pipelines.rs`: startup precompile of the lowering table's variants; async creation; last-valid
  fallback.
- A CI lint (`cargo xtask lint gesture-compile`) that fails if a gesture handler reaches pipeline creation.

## Acceptance tests
- Review checklist (code) — no synchronous pipeline creation on the gesture path (REQ-RENDER-040).
- Review checklist (code) — no gesture handler triggers pipeline compilation; all instant-toggle variants are precompiled at load (REQ-SYS-035).

## Notes
- Both requirements are review checklists; the lint installs the check so it holds after this PR.
