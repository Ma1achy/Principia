# TASK-M6-22 — Quality in the GUI: the Run window selector, the Custom fields and the arbiter overlay

- **Milestone:** M6
- **Closes:** REQ-GUI-011, REQ-GUI-014, REQ-TOOL-058
- **Depends on:** TASK-M6-15, TASK-M6-17, TASK-M6-18, TASK-M0-20, TASK-M6-24, TASK-M6-28
- **Needs (earlier milestones):** REQ-GUI-001
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The quality selector and Custom fields (under "quality: Custom") edit `SimConfig.quality` in the Run window (R-129); the arbiter writes `SimConfig.quality` engine-side during settled periods; Custom exposes render_scale (0.25–2.0), N, `MAX_REL_DEPTH`, E, FTLE, motion gating and lock-to-native with the arbiter off; `ViewUI` carries the arbiter debug overlay's visibility, and the overlay, in the Profiler window's Arbiter tab (R-129) — the Profiler window that TASK-M6-28 builds on the mock engine, whose Arbiter tab this task fills, and TASK-M8-28 the rest (R-152, amended in part by R-390) — shows throughput, current rung, headroom and recent decisions with their reasons.

## References
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/design/principia_quality_device_note.md` § "10. Two sanctities: the user, and observability"
- `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"
- `decisions.md` § "R-152 — A minimal Profiler window holds the Arbiter tab at M6 *(closes RQ-122)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
- `decisions.md` § "R-274 — The screenshot runner reaches `gui` through a headless capture mode it spawns *(closes RQ-166)*"
- `decisions.md` § "R-275 — A control clipped out of the visible surface isn't present *(closes RQ-167)*"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"

## Deliverables
- `crates/gui/src/windows/run_quality.rs`: selector, Custom fields (read-only display of auto's values until touched), device-ceiling maxima with tooltip.
- `crates/gui/src/overlays/arbiter.rs`: the arbiter overlay, filling the Arbiter tab of the Profiler window that TASK-M6-28 builds (`crates/gui/src/windows/profiler.rs`, R-152 as amended in part by R-390), which TASK-M8-28 fills in; `ViewUI.arbiter_overlay: bool` in `crates/engine`.
- Uses `gui`'s headless capture mode (R-274, amended in part by R-390), which TASK-M6-24 builds (REQ-GUI-162): the `gui` entry point the screenshot runner spawns, which renders a named window offscreen and writes the PNG and its AccessKit names (with rects, for R-275's visibility check), and the runner's `gui` surface kind that spawns it. This task's screenshots run through it, and it re-runs REQ-GUI-162's acceptance on the real engine. No crate depends on `gui`.
- Screenshots via `cargo xtask screenshot 04_windows`, presence only — no layout comparison until the M8 dev GUI (R-129).

## Acceptance tests
- `cargo test -p engine arbiter_writes_simconfig_quality` — arbiter writes arrive as SimConfig.quality changes in the snapshot; the overlay toggle is a ViewUI field (REQ-GUI-011).
- `cargo xtask screenshot 04_windows` — presence only (R-129): the Run window's "quality: Custom" section shows each control (REQ-GUI-014).
- `cargo xtask screenshot 04_windows` — presence only (R-129): the Profiler tab shows the four items (REQ-TOOL-058).
- `cargo test -p xtask screenshot_gui_surface` — a case with surface kind `gui` spawns the capture mode for a named window and gets its PNG and names back; `cargo xtask deps` shows no edge into `gui` (REQ-GUI-162). Closed by TASK-M6-24 on the mock engine since R-390; this task re-runs it on the real engine.

## Notes
- RQ-105 ruled: R-129 — the Custom quality fields live in the Run window under "quality: Custom" and the arbiter overlay in a Profiler tab; with no artboard they are checked by presence only until the M8 dev GUI.
- RQ-122 ruled: R-152 — a minimal Profiler window holds the Arbiter tab at M6, and TASK-M8-28 fills in the rest. Amended in part by R-390: TASK-M6-28 builds the Profiler window's frame and tabs, and this task fills its Arbiter tab.
- R-390: REQ-GUI-162, the headless capture mode, is closed by TASK-M6-24, whose screenshots need it first; the Run window's frame and the Profiler window with its tabs are built on the mock engine by TASK-M6-28. This task depends on both: it adds the quality selector and Custom fields to that Run window and fills the Arbiter tab (R-152), keeping REQ-GUI-011, REQ-GUI-014 and REQ-TOOL-058, and re-runs REQ-GUI-162's acceptance.
