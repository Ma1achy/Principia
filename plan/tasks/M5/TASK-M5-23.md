# TASK-M5-23 — Checkerboard in the frame loop: live set only, render resolution, three-state control, dt ceiling

- **Milestone:** M5
- **Closes:** REQ-RENDER-046, REQ-RENDER-047, REQ-GUI-009, REQ-VAL-081, REQ-RENDER-076
- **Depends on:** TASK-M5-02, TASK-M5-20, TASK-M5-21
- **Needs (earlier milestones):** REQ-RENDER-029, REQ-RENDER-032, REQ-RENDER-033, REQ-RENDER-034, REQ-RENDER-035, REQ-RENDER-036, REQ-PERF-005
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-3
- **Size:** ~380 lines

## Goal
M4's checkerboard joins the M5 loop: it applies to the live-set march only (catch-up marches advance every
sample every step), runs at render resolution before the render → display upscale and composes with render_scale, and is a
three-state user setting (On-permanent by default, On-motion-only, Off). The per-frame playhead-advance ceiling gates
it: its units are defined in checkerboard_contract §7 (REQ-RENDER-076) and the ceiling is measured on the real render at
E + 1 = 8 where the mean reconstruction error crosses ~2 8-bit levels (REQ-VAL-081).

## References
- `docs/contracts/principia_checkerboard_contract.md` § "3. The three-state control"
- `docs/contracts/principia_checkerboard_contract.md` § "4. Composition with render-scale (they stack; upscale helps)"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/contracts/principia_checkerboard_contract.md` § "7. Measured imperceptibility (the evidence for the default)"
- `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `docs/contracts/principia_checkerboard_contract.md` §7: the ceiling's units (REQ-RENDER-076), with the "Removed lines"
  note.
- `crates/engine/src/frame/checkerboard.rs`: live-set-only mask, the three-state setting in `QualitySettings`, the ceiling
  gate.
- `cargo xtask gate checkerboard-ceiling`: mean reconstruction error vs dt at E + 1 = 8.
- Tests `crates/engine/tests/checkerboard_loop.rs`.

## Acceptance tests
- `cargo test -p engine checkerboard_live_only` — catching-up quads advance every sample every step with checkerboard on (REQ-RENDER-046).
- `cargo test -p engine checkerboard_render_scale` — with render_scale 0.5 the mask is over render pixels; both levers on at once work (REQ-RENDER-047).
- `cargo test -p engine checkerboard_three_state` — default is permanent; motion-only renders static playback full; off never masks (REQ-GUI-009).
- `cargo xtask gate checkerboard-ceiling` — on the real render at E + 1 = 8, measure mean reconstruction error vs dt; ceiling set where it crosses ~2 levels; above it checkerboard is off (or ramped) (REQ-VAL-081).
- Review checklist (physics) — the doc states the ceiling's units (sim time per frame, dt, or other) so the §8 measurement records a value in them; the doc change is in this PR and the physics reviewer approves it before merge (REQ-RENDER-076).

## Notes
- Definitions (R-72) this task writes: REQ-RENDER-076.
- The corpus gives the ceiling's existence and proxy (~0.05), not its value on the real system: the gate measures
  it. checkerboard §8 asks for "the real VMF/OKLAB colour mapping", which is M7 work (see Gaps).
- Whether to ramp the stale fraction instead of a hard gate is "a feel call" (§8), not decided here (see Gaps).
- Waits on RQ-99 (`REVIEW_QUEUE.md`): M5 requirements that need M6, M7 or M8.
- Waits on RQ-104 (`REVIEW_QUEUE.md`): Checkerboard at the dt ceiling: ramp or hard gate ("a feel call").
