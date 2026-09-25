# TASK-M7-23 — The node inspector: morphing editors, source availability, the L-conflict flag and the map controls

- **Milestone:** M7
- **Closes:** REQ-GUI-017, REQ-GUI-018, REQ-GUI-020
- **Depends on:** TASK-M7-05, TASK-M7-06, TASK-M7-08, TASK-M7-22, TASK-M6-15
- **Needs (earlier milestones):** REQ-RENDER-013, REQ-RENDER-014, REQ-GUI-015, REQ-RENDER-030
- **Reviewers:** code, qa, gui
- **Pitfalls:** PIT-8
- **Size:** ~450 lines

## Goal
The node inspector of render_gui_spec §9, morphing by node kind and wired source subtype (SiteBlend editor for vector, gradient editor for scalar, palette-swatch editor for categorical, gradient/compaction editor for brightness, combiner choice, post-op controls, source field dropdown). Sources the current quality tier does not populate are greyed out; binding one anyway triggers no recompute, reads NaN at unpack and shows the invalid colour, and the only path to recompute is an explicit tier change (R-79). Pairing a monotone-L LUT with Replace-L raises a conflict flag. Every Artefact-1 map offers the global Invert (v ↦ 255 − v), Blend (linear mix of any two modes) and Auto-rotate controls.

## References
- `docs/contracts/principia_render_contract.md` § "Part 3 — Cache tiers and the recompute rule"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `docs/design/principia_colour_composition.md` § "7.1 The complete map list (R-16)"
- `docs/gui/principia_render_gui_spec.md` § "9. Node inspector — morphing editors"

## Deliverables
- `crates/gui/src/stain/inspector/{mod,site_blend,gradient,palette,compaction,post,source}.rs`.
- `crates/render/src/colour/conflicts.rs` — the monotone-L × Replace-L detector the inspector shows.
- Screenshot scenario for the controls of the `principia_colour_explorer.html` group's maps (R-139).

## Acceptance tests
- `cargo test -p gui source_availability` — at a no-FTLE tier bind brightness to ftle: the dispatch counter stays static; the source is greyed; the unpacked value is NaN and the pixel shows the invalid colour (REQ-GUI-017).
- `cargo test -p gui l_conflict_flag` — dd_colouring unit test 6 wiring test: select a monotone-L LUT + Replace-L; the conflict flag is set; with Multiply it is not (REQ-GUI-018).
- `cargo xtask screenshot 02_stain` (scenario explorer-map-controls) — Invert, Blend and Auto-rotate present on each map of colour_composition §7.1's `principia_colour_explorer.html` group (R-139) (REQ-GUI-020).

## Notes
- PIT-8: the invalid colour for an unavailable source must not be confusable with a data colour.
