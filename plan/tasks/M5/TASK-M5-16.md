# TASK-M5-16 — Ensemble copies: Halton offsets and copies as scheduler leaves

- **Milestone:** M5
- **Closes:** REQ-SCHED-038, REQ-SCHED-039, REQ-SCHED-041, REQ-PAY-062, REQ-PAY-068, REQ-RENDER-043
- **Depends on:** TASK-M5-04, TASK-M4-18
- **Needs (earlier milestones):** REQ-INT-039, REQ-INT-065, REQ-INT-069, REQ-RENDER-031, REQ-PERF-012, REQ-VAL-056
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~350 lines

## Goal
A footprint has E+1 samples, `copy_index` 0..E: copy 0 at the un-jittered centre, copies 1..E at Halton (2,3)
points 1..E, centred (minus ½) and scaled to the footprint — deterministic, time and chart_id excluded, no per-cell
rotation, identical on CPU and GPU. Every copy is the same full `SimState` (with its own Benettin shadow), an extra array
entry and never an extra field, dispatched as the same kernel with `copy_index` as a uniform; `ENSEMBLE_ENABLED` is
checked only for nominal samples, so a quad dispatches at most N²·(E+1) trajectories. No per-sample ensemble tag exists.
Spread is not stored and shadows never enter the SSAA or spread pools.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 9 — Ensemble / SSAA sampling (dispatch rules)"
- `docs/notes/principia_sampling_msaa_note.md` § "No recursion: structural uniformity ≠ role uniformity"
- `docs/notes/principia_sampling_msaa_note.md` § "Amendments this makes"
- `docs/notes/principia_sampling_msaa_note.md` § "The sampling pattern: deterministic Halton offsets"
- `docs/notes/principia_sampling_msaa_note.md` § "Ensemble copies ARE the SSAA samples"
- `docs/design/principia_dd_colouring.md` § "3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/notes/principia_sampling_msaa_note.md` § "Uniform samples: every sample is a full, normal SimState"
- `docs/notes/principia_sampling_msaa_note.md` § "Stability metrics: FTLE and diffusion are SimState fields; spread is a resolve-stage reduction"
- `docs/contracts/principia_scheduler_contract.md` § "`QuadRequest.flags`"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/design/principia_temporal_architecture_note.md` § "Stability metrics are just running accumulators (they get simpler)"
- `docs/design/principia_temporal_architecture_note.md` § "Still open (settle at implementation, or next edit pass)"
- `decisions.md` § "R-80 — Samples per footprint *(closes RQ-31)*"
- `decisions.md` § "R-100 — No per-cell Halton rotation *(closes RQ-60)*"
- `decisions.md` § "R-102 — The ensemble isn't a baked variant *(closes RQ-62)*"

## Deliverables
- `crates/kernel/src/ensemble.rs`: Halton (2,3) offsets from `copy_index` (shared source, f32 and f64).
- `crates/engine/src/dispatch/ensemble.rs`: nominal/copy dispatch, the no-recursion rule.
- Tests `crates/kernel/tests/ensemble.rs` (CPU/GPU offset parity, frame invariance) and
  `crates/engine/tests/ensemble_dispatch.rs` (trajectory count).

## Acceptance tests
- `cargo test -p engine ensemble_no_recursion` — with ENSEMBLE_ENABLED and E = 3, a quad dispatches N²·(E+1) trajectories, never more (REQ-SCHED-038).
- `cargo test -p kernel halton_offsets` — copy 0's offset is (0, 0); copies 1..E equal the reference Halton (2,3) points 1..E minus ½, scaled to the footprint, in every footprint; identical across frames and CPU/GPU (REQ-SCHED-039).
- `cargo test -p kernel copies_are_leaves` — copy offsets equal the centred Halton(2,3) points 1..E; copy 0 is at the centre; copies never spawn ensembles (REQ-SCHED-041).
- Review checklist (code) — one SimState type for nominal and copy samples; copies are extra array entries; each has its own shadow (REQ-PAY-062).
- Review checklist (code) — SimState has no ensemble tag field (REQ-PAY-068).
- `cargo test -p kernel ssaa_pool` — copy offsets equal centred Halton (2,3) points 1..E with copy 0 at the centre; no stored spread field; shadows excluded from resolve (REQ-RENDER-043).

## Notes
- The offset parity test is stateless (inputs held fixed); it establishes offset identity only, never that copy
  outcomes agree across backends (pitfalls §10).
