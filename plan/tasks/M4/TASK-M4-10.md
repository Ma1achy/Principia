# TASK-M4-10 — The fixed-timestep frame loop over the flat grid

- **Milestone:** M4
- **Closes:** REQ-SCHED-004, REQ-SCHED-009, REQ-SCHED-013, REQ-SCHED-084
- **Depends on:** TASK-M4-08, TASK-M4-09
- **Needs (earlier milestones):** REQ-SCHED-002, REQ-INT-038, REQ-INT-026, REQ-PAY-051, REQ-TOOL-005
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Time is a live clock. A global playhead advances by a fixed `dt` per sim-step; sim-steps are decoupled from render frames (accumulate wall-clock, take fixed steps), so playback speed is a dt-per-second setting and never the device's frame rate. Every visible sample's `SimState` marches with the playhead under a barrier over the live set (at layer 0, the whole flat grid), and the fragment stage colours the current state. Default playback reaches `t_end` in one minute at 1× via `steps_per_frame = ⌈T / (60 × fps × dt_macro)⌉` with a 0.1×–10× multiplier; which fps the formula reads is defined in the temporal note (R-72). The loop has the progressive (interactive) and blocking (export) barrier policies.

## References
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_temporal_architecture_note.md` § "The decision"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/design/principia_temporal_architecture_note.md` § "Decisions — DECIDED (ratified in conversation; recorded here so they don't evaporate)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 1 — Playback is the temporal mechanism"
- `docs/contracts/principia_export_animation_contract.md` § "Part 5 — Determinism & reproducibility"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_export_animation_contract.md` § "Part 4 — Export: the frame loop in blocking mode"

## Deliverables
- `crates/engine/src/frame_loop.rs`: playhead, fixed-timestep accumulator, barrier over the live set, progressive / blocking policies, speed multiplier.
- Doc change: `docs/design/principia_temporal_architecture_note.md` § Decisions — which fps the formula reads (measured or nominal), with the "Removed lines" note.
- Frame records emitted to the profiler schema (the work each frame did).

## Acceptance tests
- `cargo test -p engine lockstep_playhead` — after k frames every live-set quad's step count equals the playhead's (REQ-SCHED-004).
- `cargo test -p engine fixed_timestep_fps` (proptest) — the same session at simulated 30 and 144 fps: the state at each sim step is bit-identical (REQ-SCHED-009).
- `cargo test -p engine steps_per_frame` — the formula yields the steps per frame, reading the fps the doc defines; the multiplier range is clamped to 0.1–10 (REQ-SCHED-013, REQ-SCHED-084).
- Physics reviewer on the doc change — the temporal note says which fps the formula reads, and the steps-per-frame unit test uses it (REQ-SCHED-084).

## Notes
- REQ-SCHED-084 is a definition (R-72): the doc change is part of this PR and needs the physics reviewer's approval before merge.
- REQ-SCHED-084 is a definition requirement (R-72): measured vs nominal fps is written into the doc, not chosen in code.
