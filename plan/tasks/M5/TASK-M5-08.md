# TASK-M5-08 — The current-state cache: lookup, resume points, preview vs refined, no history

- **Milestone:** M5
- **Closes:** REQ-SCHED-016, REQ-SCHED-029, REQ-SCHED-030, REQ-SCHED-049, REQ-PAY-060, REQ-SCHED-099
- **Depends on:** TASK-M5-06, TASK-M5-07, TASK-M4-10
- **Needs (earlier milestones):** REQ-SCHED-002, REQ-SCHED-004, REQ-SCHED-009, REQ-PAY-005, REQ-PAY-057, REQ-TOOL-011
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1, PIT-10
- **Size:** ~400 lines

## Goal
The quad cache holds current state only, one timestep per entry: lookup is identity + signature match with the
entry's `t` as resume metadata. A cached state is presentable only at its stored `t` and otherwise resumes
`t_cached → playhead`, landing bit-identical to the never-evicted state. Preview (`PREVIEW_MODE`, reduced T) and refined
payloads of one quad are distinct entries; neither satisfies a lookup for the other; sharpen-on-idle is a new entry,
never an in-place edit. The engine stores no trajectory history; a time scrub sets the display time and re-integrates.
It also defines render_contract Part 6's quad fields cache-valid flag, which R-424 moved here from the M1 checklist,
decides whether a per-quad schema version belongs with it, and adds what it defines to the field-view catalogue
(REQ-SCHED-099, R-72).

## References
- `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"
- `docs/contracts/principia_caching_contract.md` § "Part 7 — The current-state cache (resume points, hard-capped)"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/contracts/principia_scheduler_contract.md` § "Part 5 — Preview vs refine is a second sim key"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_temporal_architecture_note.md` § "The decision"
- `docs/design/principia_temporal_architecture_note.md` § "What changes vs what doesn't"
- `docs/design/principia_temporal_architecture_note.md` § "Decisions — DECIDED (ratified in conversation; recorded here so they don't evaporate)"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_dd_generation_root.md` § "3.7a `RenderQuad` — the CPU-written quad record"
- `decisions.md` § "R-424 — The quad fields row's flags: contains-ensemble, contains-FTLE and the schema version are checked at M1 against `has_ensemble()`, `has_ftle` and `PAYLOAD_SCHEMA_VERSION`; sim-failed moves to the `QuadReduction` task and cache-valid to the M5 cache task *(closes RQ-275)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/engine/src/cache/{mod.rs, lookup.rs, resume.rs}`.
- Property tests `crates/engine/tests/cache.rs`: evict at random t and resume (binary fields bit-identical); resident
  bytes constant over a long march; scrub back re-integrates from t = 0.
- Unit tests for preview/refined separation and sharpen-on-idle.
- The cache-valid definition and the per-quad schema version decision (REQ-SCHED-099, R-424) in
  `docs/design/principia_dd_generation_root.md` §3.7a, with the commit's "Removed lines" note; the quad fields view's
  cache-valid cell (and the per-quad schema version's, if added) in the generated catalogue.

## Acceptance tests
- `cargo test -p engine cache_resume_exact` — evict at random t and resume: binary fields bit-identical to the uninterrupted run (REQ-SCHED-016).
- `cargo test -p engine cache_preview_refined` — insert preview and refined payloads for one quad; assert distinct keys and that full-quality lookups miss on a preview-only entry and vice versa (REQ-SCHED-029).
- `cargo test -p engine cache_sharpen_on_idle` — after idle, the refined payload is a new cache entry and the preview entry's bytes are unchanged (REQ-SCHED-030).
- `cargo test -p engine cache_preview_never_refined` — a preview-valid entry is not returned for a refined lookup of the same quad (REQ-SCHED-049).
- `cargo test -p engine cache_no_history` — GPU bytes resident stay constant as the playhead advances over a long march; scrubbing back re-integrates from t = 0 (REQ-PAY-060).
- Review checklist (physics) — dd_generation_root §3.7a defines what cache-valid holds per drawn quad and its source (caching contract Part 1's identity and signature match, or otherwise), and states whether a per-quad schema version belongs with it, with the reason (it can differ from the build's `PAYLOAD_SCHEMA_VERSION` only once quads can come from an older build's cache); the doc change is in this PR and the physics reviewer approves it before merge (REQ-SCHED-099).
- Review checklist (qa) — render_contract Part 6's quad fields row's cache-valid cell, and the per-quad schema version's if it is added, is ticked against the generated catalogue on a synthetic payload, a valid and an invalid quad drawing differently (REQ-SCHED-099).

## Notes
- A resumed state at a different `t` presented beside live quads is the patchwork of pitfalls §1 (mixed time
  strata); the presentable-only-at-`t` rule is its guard, and the physics reviewer checks it.
- Definitions (R-72) this task writes: REQ-SCHED-099.
- **R-424 (13 Oct 2026) rules RQ-275** (TASK-M1-19's pre-flight): "cache-valid moves to the M5 cache task, which also
  decides whether per-quad schema version belongs with cache-valid (it only matters once quads can come from an older
  build's cache)". This task is that task (applied per R-369): its lookup is identity + signature match, the check
  cache-valid reports, against TASK-M5-06's signature. At M1 the schema version cell is checked against the build's
  `PAYLOAD_SCHEMA_VERSION` (REQ-TOOL-011).
