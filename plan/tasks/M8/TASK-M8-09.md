# TASK-M8-09 — The hover integration: computeIC in the inspector worker, step budget, and the projection onto (q₁, q₂)

- **Milestone:** M8
- **Closes:** REQ-GUI-051, REQ-GUI-053, REQ-GUI-056, REQ-TOOL-085, REQ-TOOL-094, REQ-PERF-072, REQ-PERF-075, REQ-PERF-078
- **Depends on:** TASK-M8-01, TASK-M8-04
- **Needs (earlier milestones):** REQ-SYS-017, REQ-GUI-001, REQ-SYS-009, REQ-DEC-032, REQ-TOOL-028, REQ-SCHED-022, REQ-SYS-030, REQ-CHART-028
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-3
- **Size:** ~500 lines

## Goal
Hover and click map the pointer through the single Y-flip, decode the IC on the CPU, and integrate it with one f64 `computeIC` returning the full state r(t), p(t), in a dedicated inspector worker separate from the render loop: debounced, cancel-on-move with a latest-wins generation counter, never awaited on the main thread, under a step budget while the pointer moves (a partial result marked incomplete) that lifts on dwell. The trace is the state projected onto the chart's actual q₁, q₂ through the forward map, never onto a dominant named coordinate. It reads no payload and does no GPU readback, and the one result feeds both the trace and sonification. All of it is ViewUI-side and reaches the engine only through `computeIC(chart, uv, simKey)`.

## References
- `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"
- `docs/design/principia_trajectory_viewing.md` § "3. Hover trace — projection onto the current (tilted, sliced) plane"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/design/principia_trajectory_viewing.md` § "6. Placement"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_render_contract.md` § "Part 7 — The hover trace (per-IC trajectory overlay)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/design/principia_systems_architecture.md` § "The return paths"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"

## Deliverables
- `crates/engine/src/inspector/worker.rs` — the inspector worker (native: a thread; browser: the second wasm context, TASK-M8-38), with a generation counter, debounce and cancel.
- `crates/engine/src/inspector/trajectory.rs` — `Trajectory { t, r, p, complete }` from `computeIC`, with the step budget and the dwell release.
- `crates/engine/src/inspector/project.rs` — `project(state, q)` through the chart's forward map.
- Tests: `pointer_flip_decode`, `hover_projection` (proptest), `hover_no_gpu_read`, `hover_shared_result`, `hover_budget_incomplete`.
- Calibration proposal: the step budget and the dwell time, with the total_substeps distribution (p1–p99) of hover ICs and the per-frame cost against the 60 fps frame.

## Acceptance tests
- `cargo test -p engine pointer_flip_decode` — pointer at the top edge decodes v ≈ 1 (top of the manifold); the trajectory holds r and p at every step (REQ-GUI-051).
- `cargo test -p engine hover_projection` — a conserved axis (E, L_z) gives a dot; tilting it toward a live direction grows a line then a curve; changing the slice shifts the trace without changing its shape (REQ-GUI-053).
- Review checklist (gui reviewer) — trajectory-viewing code calls only computeIC; its state lives in ViewUI (REQ-GUI-056).
- `cargo test -p engine hover_no_gpu_read` — hover a pixel: the trace equals computeIC(decoded IC) projected onto the chart basis; no GPU buffer is read (REQ-TOOL-085).
- `cargo test -p engine hover_shared_result` — hover issues no GPU readback; trace and sound consume the same computeIC result (REQ-TOOL-094).
- Review checklist (perf reviewer), with `cargo xtask bench hover-in-flight` — rapid hover moves issue at most one completed integration per settle; frame time unaffected by an in-flight trace (REQ-PERF-072).
- `cargo test -p engine hover_budget_incomplete` — a near-collision IC under motion returns a partial trajectory flagged incomplete; after dwell the full trajectory arrives (REQ-PERF-075).
- `cargo xtask bench hover-step-budget` — the proposal shows the total_substeps distribution (p1 to p99) of hover ICs and the per-frame cost of the chosen budget against the 60 fps frame; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-PERF-078).

## Notes
- REQ-PERF-072, REQ-PERF-075 and REQ-PERF-078 wait on RQ-74 (whether pointer_channels §3's per-frame-during-motion budget or trajectory_viewing §1's "one integration per settled hover" is normative). The deliverables follow the requirements as written; the PR can't merge until RQ-74 is ruled.
- Calibrations (R-71) proposed here: REQ-PERF-078. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
