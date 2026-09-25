# TASK-M4-12 — Checkerboard self-erasing: catch-up at rest, the three-state control and export force-off

- **Milestone:** M4
- **Closes:** REQ-RENDER-029, REQ-RENDER-034, REQ-RENDER-036, REQ-SCHED-008, REQ-VAL-058, REQ-RENDER-078
- **Depends on:** TASK-M4-11
- **Needs (earlier milestones):** REQ-RENDER-004, REQ-SCHED-002
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~420 lines

## Goal
The presentation half of checkerboard. A stale pixel shows only its own value from t − dt (temporal reuse, never spatial interpolation). When the playhead stops — rest, pause, `t_end` / all-terminated — the stale half is marched one dt and promoted at the next barrier, so t = 0 and every quiescent frame is complete and no frame is served checkerboarded except mid-advance. The three-state control (On-permanent default, On-motion-only, Off) is opt-out and forced off in the blocking export loop. The single-playhead invariant holds at every quiescent moment, with a skew of exactly one dt only while advancing. The spread-extrapolation default is measured on the real render and put to the human for the record.

## References
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/contracts/principia_checkerboard_contract.md` § "2. The mechanism: reconstruct-from-previous, resolved through SSAA"
- `docs/contracts/principia_checkerboard_contract.md` § "5. Self-erasing: catch-up at rest, endpoints, and pause"
- `docs/contracts/principia_checkerboard_contract.md` § "6. The single-playhead invariant, relaxed (the one architectural concession)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- `docs/contracts/principia_checkerboard_contract.md` § "3. The three-state control"
- `docs/contracts/principia_checkerboard_contract.md` § "7. Measured imperceptibility (the evidence for the default)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/engine/src/checkerboard.rs`: the setting, the playhead-stationary catch-up (one dt for the stale half, promoted at the barrier), force-off under the blocking policy.
- `crates/render`: the stale-pixel reconstruction reading the pixel's own previous colour/state only.
- `xtask` gate `checkerboard-extrapolation`: reconstruction error with and without spread-extrapolation on the real render, and its cost; the proposal for the default in the PR.

## Acceptance tests
- `cargo test -p engine checkerboard_control` — paused and endpoint frames have zero skew; export frames never checkerboard; toggling off disables it (REQ-RENDER-029).
- `cargo test -p engine checkerboard_catch_up` — pause mid-playback: the next presented frame has every pixel at `t_now`; the t = 0 and `t_end` frames have no stale pixels (REQ-RENDER-036).
- `cargo test -p engine presented_time` (proptest) — with checkerboard Off every presented frame's live quads carry identical t; with checkerboard on the skew is ≤ one dt and zero whenever the playhead is stationary (REQ-SCHED-008).
- Code reviewer plus `cargo test -p render stale_reconstruction` (neighbours poisoned, output unchanged) — the reconstruction shader reads only the pixel's own previous colour/state (REQ-RENDER-034).
- `cargo xtask gate checkerboard-extrapolation` — a recorded decision with the measured error reduction (≈ −69% on the contract's proxy) and its cost; the default is recorded in decisions.md by the human (REQ-VAL-058).
- Definition: spread-extrapolation's value, rate source and gate written into checkerboard_contract §2 and approved by the physics reviewer (REQ-RENDER-078).

## Notes
- Gap: the spread-extrapolation `rate` is not defined, and REQ-VAL-058's "decided" needs a human ruling, not an implementer's choice.
- Closes, for gaps the corpus leaves open: REQ-RENDER-078 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
