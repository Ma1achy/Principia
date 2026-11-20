# TASK-M4-07 — The precompile rule and the bounded monomorphisation set

- **Milestone:** M4
- **Closes:** REQ-PERF-006, REQ-PERF-007, REQ-PERF-077
- **Depends on:** TASK-M4-06
- **Needs (earlier milestones):** REQ-INT-005, REQ-TOOL-013, REQ-RENDER-005
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
No compute source is ever compiled at runtime. Every variant in a bounded, explicitly listed monomorphisation set is compiled to SPIR-V at build time; at runtime only pipeline objects are created from it — eagerly for the active (occupant, tier, links) of every chart at startup, so chart switching is instant, and lazily inside the re-integration cost for occupant / tier / link / threshold changes. The set is defined in lowering Part 4 (R-72) and bounded to the active/plausible combinations, not the cross-product.

## References
- `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/kernel/variants.toml` (or equivalent): the explicit variant list the build compiles; the build records its size and build time.
- `crates/engine/src/pipelines.rs`: the pipeline-object cache — eager set at startup, lazy creation on a sim-key change.
- Doc change: `docs/contracts/principia_lowering_contract.md` Part 4 — the monomorphisation set and the rule that bounds it (R-72), with the porting rule's "Removed lines" note.

## Acceptance tests
- `cargo test -p engine pipeline_creation` — a chart switch creates no pipeline; an occupant change creates its pipeline during the re-integration; no source compiler runs at runtime (REQ-PERF-006).
- `cargo test -p kernel variant_list` plus perf review — the build enumerates an explicit variant list and records its size and build time (REQ-PERF-007).
- Physics reviewer on the doc change — lowering Part 4 lists the set and the rule that bounds it; it is not the full cross-product and covers every shipped chart, occupant, tier and link (REQ-PERF-077).

## Notes
- REQ-PERF-077 is a definition (R-72): the doc change is part of this PR and needs the physics reviewer's approval before merge.
- REQ-PERF-077 is a definition requirement (R-72): the doc change merges only with the physics reviewer's approval.
