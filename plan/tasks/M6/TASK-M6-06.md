# TASK-M6-06 — Merging under a live playhead: coarsening, resident count and merge memory

- **Milestone:** M6
- **Closes:** REQ-REF-026, REQ-REF-027, REQ-REF-028, REQ-REF-029, REQ-REF-035
- **Depends on:** TASK-M6-02, TASK-M6-03, TASK-M6-04, TASK-M5-11, TASK-M5-28
- **Needs (earlier milestones):** REQ-SCHED-032, REQ-SCHED-037, REQ-SCHED-050, REQ-TOOL-050
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~420 lines

## Goal
The tree coarsens under a live playhead: a parent whose four children are leaves that did not split this boundary merges back when it is `Keep` or `Floor`; the children become `Decision::Merged` and their footprints' latches go with them, as they do on eviction (R-99). `QuadTree::resident` is tracked apart from `quads_computed`. A merged parent keeps its exponents with a separate expiry flag (never cleared by deletion) that expires when its structured weight leaves a factor of two; capped leaves stay re-decidable and are re-tested every boundary.

## References
- `docs/design/principia_dd_refinement_policy.md` § "3. Merging — the split rule read backwards"
- `decisions.md` § "R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*"
- `docs/design/principia_temporal_architecture_note.md` § "Continuous refinement — temporal accumulators + spatial coherence"
- `docs/design/principia_temporal_architecture_note.md` § "Decisions — DECIDED (ratified in conversation; recorded here so they don't evaporate)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/contracts/principia_caching_contract.md` § "Part 7 — The current-state cache (resume points, hard-capped)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 8 — Continuous refinement & the live-to-live handoff"
- `decisions.md` § "R-91 — The temporal accumulators feed "unresolved" *(closes RQ-42)*"

## Deliverables
- `crates/engine/src/refine/merge.rs`: the boundary pass (merge test, `Merged` marking, latch drop via TASK-M6-03's store), merge memory with `expired: bool`.
- `QuadTree::resident` and `quads_computed` counters, both reported in the frame record.
- Cache eviction path drops the latches of evicted quads.
- Golden suite `fixtures/golden/merge_pulse/` (moving-pulse fixture) run by `cargo xtask golden merge-pulse`.

## Acceptance tests
- `cargo xtask golden merge-pulse` — moving-pulse fixture: resident count rises and falls (≈37 → 85 → 37 → 69), merged children hold no latch, and the final tree is bitwise the static tree at the horizon (REQ-REF-026).
- `cargo test -p engine resident_vs_computed` — after merges, resident < quads_computed and both are reported (REQ-REF-027).
- `cargo test -p engine merge_memory_expiry` — expired memory is distinguishable from absent memory (not an assert_ne!); expiry does not revert to first-time flooring (421 vs 645 quads regression) (REQ-REF-028).
- `cargo test -p engine capped_leaf_redecidable` — pulse fixture: the tree coarsens to the static 69 quads rather than 149 (REQ-REF-029).
- `cargo test -p engine latch_dropped_on_evict_merge` — a latched footprint keeps its quad split while resident; evict or merge the quad: its latches are gone and no latch memory remains; revisiting the region re-discovers the split from fresh state (REQ-REF-035).

## Notes
- Waits on **RQ-73** (does the merge still reproduce the static tree now that the latch keeps a once-unresolved footprint unresolved?) — REQ-REF-026's "bitwise the static tree" assertion and REQ-REF-035 depend on the ruling. Also carries RQ-72 through REQ-REF-035.
- PIT-9: expired memory vs absent memory must be told apart by state, not by an `assert_ne!`.
