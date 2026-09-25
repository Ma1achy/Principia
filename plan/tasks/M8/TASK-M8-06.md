# TASK-M8-06 — Manifold view panel: Chart, Navigate, depth readout, Centre z₀, Slice & tilt, and the figure's axis labels

- **Milestone:** M8
- **Closes:** REQ-GUI-081, REQ-GUI-082, REQ-GUI-083, REQ-GUI-084, REQ-GUI-034, REQ-GUI-161
- **Depends on:** TASK-M8-05
- **Needs (earlier milestones):** REQ-GUI-002, REQ-GUI-003, REQ-GUI-006, REQ-GUI-012, REQ-CHART-005, REQ-CHART-012, REQ-CHART-028, REQ-SYS-014, REQ-CHART-002, REQ-RENDER-026
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The left panel is one "Manifold view" group in the order §G2 gives: Chart (preset named by its axes, q₁ / q₂ with edit buttons, Chart builder…, kind), Navigate (centre (u, v), zoom as log₂, the eight z₀ values by drag and typing), the depth readout with the event-driven precision warning, Lock, Centre z₀ (eight named sliders), and Slice & tilt (slice step, τ₁, τ₂, γ). Direction labels say whether a line is a raw-control line or a physical one-quantity direction, evaluated as tangent vectors at the chart centre with the axis's residual convention. The figure's axis labels carry the short axis name and the end values.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Directions are axis kinds — the unifying rule"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/gui/src/explore/manifold_view/{chart,navigate,depth,centre,slice_tilt}.rs` — each control emits SetField on the SimConfig paths from TASK-M8-01; each shows the per-field warning from TASK-M8-03.
- `crates/gui/src/explore/direction_label.rs` — raw-control vs physical labelling from the chart axis metadata (REQ-CHART-012).
- `crates/gui/src/explore/axis_labels.rs` — short axis name and range at each end, recomputed after pan.
- `crates/gui/src/explore/manifold_view/chart.rs` also carries the shape-sphere controls: the projection selector (equirectangular / the equal-area alternative) and the hemisphere toggle, shown only when the chart is the shape sphere (R-113).
- Screenshot cases `01_main/shape_sphere_controls`, `01_main/left_panel`, `01_main/axis_labels`, `01_main/direction_labels`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (left panel) — screenshot against 01_main.png's left panel: one group, sub-sections in that order, eight named z₀ sliders (REQ-GUI-081).
- `cargo xtask screenshot 01_main` and `cargo test -p gui preset_display_names` — screenshot against 01_main.png; every shipped preset's display name is built from its axis names (REQ-GUI-082).
- `cargo test -p gui navigate_z0_fields` — drag and typed entry on each of the eight z₀ fields both emit the same SetField and round-trip through the snapshot (REQ-GUI-083).
- `cargo xtask screenshot 01_main` (axis labels, before and after a pan) — screenshot against 01_main.png; after a pan the end values update (REQ-GUI-084).
- `cargo xtask screenshot 01_main` (direction labels) — slider/direction labels distinguish 'one mass logit' from 'only m₀' (REQ-GUI-034).
- `cargo xtask screenshot 01_main` (shape-sphere controls) — with the shape-sphere chart the Chart section shows the projection selector and the hemisphere toggle and each changes the figure; with any other chart neither is shown (REQ-GUI-161).

## Notes
- R-113 (RQ-95 M2-G5): the shape-sphere projection selector and hemisphere toggle live in the Manifold view's Chart section, shown when the chart is the shape sphere (REQ-GUI-161); the M2 goldens (TASK-M2-28) verify the render side.
- The Lock sub-group's badge and the compass are TASK-M8-07; this task leaves the Lock slot wired to the lock SetFields.
