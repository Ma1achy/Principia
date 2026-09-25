# TASK-M5-21 — Frame loop: the live set, off-loop catch-up and atomic promotion

- **Milestone:** M5
- **Closes:** REQ-SCHED-034, REQ-SCHED-035, REQ-SCHED-037, REQ-SCHED-051
- **Depends on:** TASK-M5-08, TASK-M5-14, TASK-M4-12
- **Needs (earlier milestones):** REQ-SCHED-002, REQ-SCHED-004, REQ-SCHED-008, REQ-SCHED-009, REQ-PERF-010
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-1
- **Size:** ~450 lines

## Goal
The frame loop presents only the barrier-synced live set. Newly revealed or split quads catch up
`0 → playhead` (or resume-point → playhead) off-loop — background, non-blocking, generation-cancelled, on the refinement
compute path — shown via the moving fallback (the blurred live ancestor, animating). Promotion, including the
parent → children swap, is gated on `quad.t == playhead` at a barrier, never on completion, and is atomic: the parent
keeps marching and rendering until the swap frame shows its children at its own t.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 1 — Playback is the temporal mechanism"
- `docs/contracts/principia_scheduler_contract.md` § "Part 8 — Continuous refinement & the live-to-live handoff"
- `docs/design/principia_temporal_architecture_note.md` § "Refinement must be a live-to-live handoff (or it looks broken)"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/design/principia_temporal_architecture_note.md` § "Still open (settle at implementation, or next edit pass)"

## Deliverables
- `crates/engine/src/frame/{loop.rs, live_set.rs, catch_up.rs, promote.rs}`.
- Tests `crates/engine/tests/frame_loop.rs`: slow reveal never delays a present; children finishing early wait; no
  partial sibling set; cancelled generation never promotes; parent t advances every frame during a split. The
  TASK-M5-13 navigation property test re-run at t > 0.

## Acceptance tests
- `cargo test -p engine catch_up_off_loop` — a slow reveal never delays the live-set barrier; the revealed region shows the animating blurred ancestor until promotion (REQ-SCHED-034).
- `cargo test -p engine promote_time_sync_atomic` — children that finish early wait for the barrier with t == playhead; no frame presents a partial set of siblings (REQ-SCHED-035).
- `cargo test -p engine live_to_live_handoff` — during a split the parent region never freezes (its t advances every frame) and the swap frame shows children at the parent's t (REQ-SCHED-037).
- `cargo test -p engine barrier_live_set_only` — a slow catching-up quad never delays a present; promotion only at a barrier with equal t; a cancelled generation never promotes (REQ-SCHED-051).

## Notes
- Promoting on completion instead of time-sync presents mixed time — the patchwork of pitfalls §1 — and the physics
  reviewer checks it.
