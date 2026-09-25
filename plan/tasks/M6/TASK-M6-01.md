# TASK-M6-01 — The refinement decision and the quad lifecycle: terminal vs refinable

- **Milestone:** M6
- **Closes:** REQ-REF-033, REQ-REF-034, REQ-SCHED-064, REQ-REF-011, REQ-REF-047
- **Depends on:** TASK-M5-17, TASK-M5-30
- **Needs (earlier milestones):** REQ-VAL-080, REQ-REF-009, REQ-TOOL-116, REQ-SCHED-047
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~300 lines

## Goal
The engine's quadtree gets the `Decision` enum with exactly the eight variants of refinement_policy §6 and the lifecycle mapping from each stop reason to `ReadyRefinable` or terminal. Only the true floors (`AT_F32_FLOOR`, the integration floor) and `Decision::Undetermined` make a quad terminal and cached as a quad fact; everything kept by the policy or screen-floored at this zoom stays refinable and is revisited when budget frees or on zoom-in. `Undetermined` is reported, terminal and flagged (R-47), with no second refinement axis. The scheduler never dequeues a terminal quad. This is the frame every later M6 task decides inside.

## References
- `docs/design/principia_dd_refinement_policy.md` § "6. Decision variants, and the second budget line"
- `decisions.md` § "R-47 — `Undetermined` is reported, terminal and flagged, until a chart shows it non-zero *(RS-7)*"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `docs/contracts/principia_scheduler_contract.md` § "Part 4 — Terminal vs refinable, tied to the two floors"
- `decisions.md` § "R-88 — What stops in-view refinement *(closes RQ-39)*"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `decisions.md` § "R-98 — `MAX_REL_DEPTH` caps every split beyond the screen floor *(closes RQ-58)*"
- `decisions.md` § "R-108 — Must-split above the floor is the at-rest target *(closes RQ-68)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/engine/src/refine/decision.rs`: `enum Decision { Keep, Split, Floor, ScreenFloor, Undetermined, Collapsed, Merged, BalanceForced }` and the stop-reason record carried as CPU quad metadata (not in `QuadReduction`).
- `crates/engine/src/quadtree/lifecycle.rs`: the terminal/refinable mapping (`is_terminal` true iff `AT_F32_FLOOR`, integration floor or `Undetermined`); the refinable queue never admits a terminal quad; a terminal flag carried for the stop breakdown.
- Screen-floored quads re-enter the refinable set when the zoom changes (the floor is not a stored fact); zoom-out un-stops them.
- Tests in `crates/engine/tests/lifecycle.rs`: exhaustive `match` over the enum; the state-machine table test; an undetermined-footprint fixture.

## Acceptance tests
- `cargo test -p engine decision_variants` — exhaustive match over the enum compiles against this list (REQ-REF-033).
- `cargo test -p engine undetermined_terminal` — an undetermined footprint yields a terminal flagged quad and appears in the stop breakdown (REQ-REF-034).
- `cargo test -p engine lifecycle_state_machine` — lifecycle state machine test: each stop reason maps to terminal/refinable as stated; scheduler never dequeues a terminal quad (REQ-SCHED-064).
- `cargo test -p engine terminal_iff_true_floor` — quad-state is terminal iff AT_F32_FLOOR or the integration floor is hit; zooming out un-stops screen-floored quads (REQ-REF-011).
- Definition: each Decision variant's producing stop (AT_F32_FLOOR, the integration floor, BalanceForced) written into refinement_policy §6 and approved by the physics reviewer (REQ-REF-047).

## Notes
- The legal state-transition table is REQ-TOOL-116's definition (M5, debug_tooling_plan §F); this task's lifecycle must follow it, not redefine it.
- `Merged` is produced by TASK-M6-06. Which variants record `AT_F32_FLOOR` and the integration floor, and what produces `BalanceForced`, are not stated — see the REVIEW_QUEUE entries below.
- Closes, for gaps the corpus leaves open: REQ-REF-047 (R-72 definition) (classification accepted by R-132).
