# TASK-M6-22 — Quality in the GUI: the Run window selector, the Custom fields and the arbiter overlay

- **Milestone:** M6
- **Closes:** REQ-GUI-011, REQ-GUI-014, REQ-TOOL-058
- **Depends on:** TASK-M6-15, TASK-M6-17, TASK-M6-18
- **Needs (earlier milestones):** REQ-GUI-001
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The quality selector and Custom fields (under "quality: Custom") edit `SimConfig.quality` in the Run window (R-129); the arbiter writes `SimConfig.quality` engine-side during settled periods; Custom exposes render_scale (0.25–2.0), N, `MAX_REL_DEPTH`, E, FTLE, motion gating and lock-to-native with the arbiter off; `ViewUI` carries the arbiter debug overlay's visibility, and the overlay, in the Profiler window's Arbiter tab (R-129) — at M6 a minimal Profiler window shell holding only that tab, which TASK-M8-28 fills in (R-152) — shows throughput, current rung, headroom and recent decisions with their reasons.

## References
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/design/principia_quality_device_note.md` § "10. Two sanctities: the user, and observability"
- `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"
- `decisions.md` § "R-152 — A minimal Profiler window holds the Arbiter tab at M6 *(closes RQ-122)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"

## Deliverables
- `crates/gui/src/windows/run_quality.rs`: selector, Custom fields (read-only display of auto's values until touched), device-ceiling maxima with tooltip.
- `crates/gui/src/overlays/arbiter.rs`: the arbiter overlay as the Arbiter tab of a minimal Profiler window shell (`crates/gui/src/windows/profiler.rs`, R-152), which TASK-M8-28 fills in; `ViewUI.arbiter_overlay: bool` in `crates/engine`.
- Screenshots via `cargo xtask screenshot 04_windows`, presence only — no layout comparison until the M8 dev GUI (R-129).

## Acceptance tests
- `cargo test -p engine arbiter_writes_simconfig_quality` — arbiter writes arrive as SimConfig.quality changes in the snapshot; the overlay toggle is a ViewUI field (REQ-GUI-011).
- `cargo xtask screenshot 04_windows` — presence only (R-129): the Run window's "quality: Custom" section shows each control (REQ-GUI-014).
- `cargo xtask screenshot 04_windows` — presence only (R-129): the Profiler tab shows the four items (REQ-TOOL-058).

## Notes
- RQ-105 ruled: R-129 — the Custom quality fields live in the Run window under "quality: Custom" and the arbiter overlay in a Profiler tab; with no artboard they are checked by presence only until the M8 dev GUI.
- RQ-122 ruled: R-152 — this task builds a minimal Profiler window shell holding the Arbiter tab; TASK-M8-28 fills in the rest.
