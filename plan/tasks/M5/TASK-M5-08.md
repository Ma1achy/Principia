# TASK-M5-08 — The current-state cache: lookup, resume points, preview vs refined, no history

- **Milestone:** M5
- **Closes:** REQ-SCHED-016, REQ-SCHED-029, REQ-SCHED-030, REQ-SCHED-049, REQ-PAY-060
- **Depends on:** TASK-M5-06, TASK-M5-07, TASK-M4-10
- **Needs (earlier milestones):** REQ-SCHED-002, REQ-SCHED-004, REQ-SCHED-009, REQ-PAY-005, REQ-PAY-057
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1, PIT-10
- **Size:** ~400 lines

## Goal
The quad cache holds current state only, one timestep per entry: lookup is identity + signature match with the
entry's `t` as resume metadata. A cached state is presentable only at its stored `t` and otherwise resumes
`t_cached → playhead`, landing bit-identical to the never-evicted state. Preview (`PREVIEW_MODE`, reduced T) and refined
payloads of one quad are distinct entries; neither satisfies a lookup for the other; sharpen-on-idle is a new entry,
never an in-place edit. The engine stores no trajectory history; a time scrub sets the display time and re-integrates.

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

## Deliverables
- `crates/engine/src/cache/{mod.rs, lookup.rs, resume.rs}`.
- Property tests `crates/engine/tests/cache.rs`: evict at random t and resume (binary fields bit-identical); resident
  bytes constant over a long march; scrub back re-integrates from t = 0.
- Unit tests for preview/refined separation and sharpen-on-idle.

## Acceptance tests
- `cargo test -p engine cache_resume_exact` — evict at random t and resume: binary fields bit-identical to the uninterrupted run (REQ-SCHED-016).
- `cargo test -p engine cache_preview_refined` — insert preview and refined payloads for one quad; assert distinct keys and that full-quality lookups miss on a preview-only entry and vice versa (REQ-SCHED-029).
- `cargo test -p engine cache_sharpen_on_idle` — after idle, the refined payload is a new cache entry and the preview entry's bytes are unchanged (REQ-SCHED-030).
- `cargo test -p engine cache_preview_never_refined` — a preview-valid entry is not returned for a refined lookup of the same quad (REQ-SCHED-049).
- `cargo test -p engine cache_no_history` — GPU bytes resident stay constant as the playhead advances over a long march; scrubbing back re-integrates from t = 0 (REQ-PAY-060).

## Notes
- A resumed state at a different `t` presented beside live quads is the patchwork of pitfalls §1 (mixed time
  strata); the presentable-only-at-`t` rule is its guard, and the physics reviewer checks it.
