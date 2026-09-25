# TASK-M6-22 — Quality in the GUI: the Run window selector, the Custom fields and the arbiter overlay

- **Milestone:** M6
- **Closes:** REQ-GUI-011, REQ-GUI-014, REQ-TOOL-058
- **Depends on:** TASK-M6-15, TASK-M6-17, TASK-M6-18
- **Needs (earlier milestones):** REQ-GUI-001
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The quality selector and Custom fields edit `SimConfig.quality` in the Run window; the arbiter writes `SimConfig.quality` engine-side during settled periods; Custom exposes render_scale (0.25–2.0), N, `MAX_REL_DEPTH`, E, FTLE, motion gating and lock-to-native with the arbiter off; `ViewUI` carries the arbiter debug overlay's visibility, and the overlay shows throughput, current rung, headroom and recent decisions with their reasons.

## References
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/design/principia_quality_device_note.md` § "10. Two sanctities: the user, and observability"
- `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"

## Deliverables
- `crates/gui/src/windows/run_quality.rs`: selector, Custom fields (read-only display of auto's values until touched), device-ceiling maxima with tooltip.
- `crates/gui/src/overlays/arbiter.rs`: the arbiter overlay; `ViewUI.arbiter_overlay: bool` in `crates/engine`.
- Screenshots via `cargo xtask screenshot 04_windows` (layout only, R-68).

## Acceptance tests
- `cargo test -p engine arbiter_writes_simconfig_quality` — arbiter writes arrive as SimConfig.quality changes in the snapshot; the overlay toggle is a ViewUI field (REQ-GUI-011).
- `cargo xtask screenshot 04_windows` — the Custom quality panel shows each control (REQ-GUI-014).
- `cargo xtask screenshot 04_windows` — overlay shows the four items (REQ-TOOL-058).

## Notes
- No artboard shows the Custom quality panel or the arbiter overlay (04_windows' Run window shows only quality, budget, max depth and ensemble) — see Gaps.
