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
it: its units are defined in checkerboard_contract §7 (REQ-RENDER-076) and the ceiling is measured on the M5 render (real integrator, the M1 debug/ramp colour views) at
E + 1 = 8 where the mean reconstruction error crosses ~2 8-bit levels (REQ-VAL-081). Above the ceiling both forms — the hard gate and the ramp of the stale fraction — are captured for the human's choice at the M5 gate (R-128).

## References
- `docs/contracts/principia_checkerboard_contract.md` § "3. The three-state control"
- `docs/contracts/principia_checkerboard_contract.md` § "4. Composition with render-scale (they stack; upscale helps)"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/contracts/principia_checkerboard_contract.md` § "7. Measured imperceptibility (the evidence for the default)"
- `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-128 ✱ — The checkerboard's ceiling form is chosen at the M5 gate *(closes RQ-104)*"

## Deliverables
- `docs/contracts/principia_checkerboard_contract.md` §7: the ceiling's units (REQ-RENDER-076), with the "Removed lines"
  note.
- `crates/engine/src/frame/checkerboard.rs`: live-set-only mask, the three-state setting in `QualitySettings`, the ceiling
  gate.
- `cargo xtask gate checkerboard-ceiling`: mean reconstruction error vs dt at E + 1 = 8, and captures of both forms at the ceiling — the hard gate and the ramp — for the M5 gate (R-128).
- Tests `crates/engine/tests/checkerboard_loop.rs`.

## Acceptance tests
- `cargo test -p engine checkerboard_live_only` — catching-up quads advance every sample every step with checkerboard on (REQ-RENDER-046).
- `cargo test -p engine checkerboard_render_scale` — with render_scale 0.5 the mask is over render pixels; both levers on at once work (REQ-RENDER-047).
- `cargo test -p engine checkerboard_three_state` — default is permanent; motion-only renders static playback full; off never masks (REQ-GUI-009).
- `cargo xtask gate checkerboard-ceiling` — on the M5 render (real integrator, M1 debug/ramp colour views) at E + 1 = 8, measure mean reconstruction error vs dt; ceiling set where it crosses ~2 levels; both forms above it — the hard gate and the ramp — are captured at the ceiling for the human's choice at the M5 gate (R-128) (REQ-VAL-081).
- Review checklist (physics) — the doc states the ceiling's units (sim time per frame, dt, or other) so the §8 measurement records a value in them; the doc change is in this PR and the physics reviewer approves it before merge (REQ-RENDER-076).

## Notes
- Definitions (R-72) this task writes: REQ-RENDER-076.
- The corpus gives the ceiling's existence and proxy (~0.05), not its value on the real system: the gate measures
  it.
- RQ-99 ruled: R-113 — REQ-VAL-081 is split: the ceiling is measured here on the M5 render; the re-confirmation on the real VMF/OKLab mapping is REQ-VAL-145 (M7, TASK-M7-06).
- RQ-104 ruled: R-128 — the task captures both the hard gate and the ramp at the ceiling; the human chooses at the M5 gate.
