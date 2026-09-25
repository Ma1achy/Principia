# TASK-M5-09 — Eviction: cost-weighted LRU with t_cached, and the pinned classes

- **Milestone:** M5
- **Closes:** REQ-SCHED-019, REQ-SCHED-032, REQ-SCHED-079, REQ-PERF-022
- **Depends on:** TASK-M5-08
- **Needs (earlier milestones):** REQ-SCHED-002
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-9
- **Size:** ~330 lines

## Goal
The cache evicts by cost-weighted LRU whose cost combines `computeCostMs` and `t_cached` by a formula written
into caching_contract Part 7 (REQ-SCHED-079). The baseline cover, its visible ancestor chain and the backdrop identity's
leaf cover are pinned as frozen states and never evicted; the coarse ancestors the fill draws during motion always
survive.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- `docs/contracts/principia_caching_contract.md` § "Part 7 — The current-state cache (resume points, hard-capped)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Eviction order, and the trap in it"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.2a Budgeting before allocating"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `docs/contracts/principia_caching_contract.md` Part 7: the eviction cost formula (REQ-SCHED-079), with the
  "Removed lines" note.
- `crates/engine/src/cache/evict.rs`: the cost, the pinned classes, the victim order.
- Unit and property tests `crates/engine/tests/evict.rs`.

## Acceptance tests
- `cargo test -p engine evict_pinned_survive` — under memory pressure pinned entries survive; deeper-t entries outlive shallower ones (REQ-SCHED-019).
- `cargo test -p engine evict_cheaper_first` — with two equally-old quads of different computeCostMs under cache pressure, the cheaper is evicted first (REQ-SCHED-032).
- Review checklist (physics) — the doc states the cost formula; the eviction unit tests (cheaper-first, deeper-t outlives shallower) follow from it; the doc change is in this PR and the physics reviewer approves it before merge (REQ-SCHED-079).
- `cargo test -p engine evict_keeps_ancestors` — under eviction, every visible region keeps an ancestor; the pinned chain survives (REQ-PERF-022).

## Notes
- Definitions (R-72) this task writes: REQ-SCHED-079.
- Blocked on a corpus conflict (see Gaps): telemetry §6.2 says drop the **deepest** cached quads first, while
  scheduler Part 6 makes deep, expensive quads **resist** eviction (∝ `computeCostMs`). The formula cannot be written
  until the human rules on the order.
