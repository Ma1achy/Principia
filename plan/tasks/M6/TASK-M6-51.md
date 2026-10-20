# TASK-M6-51 — The GUI track (R-390): a latent view saved at a tilt, tilted again, as RQ-404 rules (R-475) ⚑

- **Milestone:** M6
- **Closes:** REQ-GUI-265
- **Depends on:** TASK-M6-49
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~200 lines, the tests included

## Goal
On the mock engine, a latent view saved as a chart at a tilt offers the tilt targets, and each tilt map acts on its saved pair, as RQ-404 rules (R-475 L4); its Tilt section, dimmed meanwhile (TASK-M6-49), is enabled.

## References
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"

## Deliverables
- Per RQ-404's ruling, in the GUI's `View::basis` and the Tilt section's target dropdowns (`crates/gui/src/explore/manifold_view/`).
- Test `saved_tilted_chart_tilt`.

## Acceptance tests
- `cargo test -p gui saved_tilted_chart_tilt` — REQ-GUI-265's verify.

## Notes
- **⚑ Physics review is required**: the tilt maths on a non-orthonormal saved pair; it does not merge on the code reviewer's word alone.
- **A leaf task waiting for RQ-404** (`rq: RQ-404` on REQ-GUI-265): nothing depends on it. Numbered past the links port's (unmerged) TASK-M6-50 (applied per R-369).
