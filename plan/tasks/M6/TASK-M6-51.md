# TASK-M6-51 — The GUI track (R-390): a latent view saved at a tilt, tilted again in the orthonormal frame of its projected targets, the stored pair kept (R-475, R-514) ⚑

- **Milestone:** M6
- **Closes:** REQ-GUI-265
- **Depends on:** TASK-M6-49
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~200 lines, the tests included

## Goal
On the mock engine, a latent view saved as a chart at a tilt offers as its tilt targets the named dimensions other than its first parent's axes, each projected off the saved plane and normalised, the vanishing ones dropped; both tilt maps act in the orthonormal frame `(q̂₁, q̂₂⊥, t̂)` and carry the stored pair, so it is kept and the view reopens bitwise (R-514, ruling RQ-404; R-475 L4); its Tilt section and the compass's tilt mode, guarded until now (TASK-M6-49), are enabled. *Was:* "On the mock engine, a latent view saved as a chart at a tilt offers the tilt targets, and each tilt map acts on its saved pair, as RQ-404 rules (R-475 L4); its Tilt section, dimmed meanwhile (TASK-M6-49), is enabled."

## References
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `decisions.md` § "R-514 — A latent view saved at a tilt tilts again in an orthonormal frame: the named dimensions projected off the saved plane and normalised, vanishing ones dropped, are its targets; both maps act in that frame; the stored pair is kept, so the view reopens bitwise *(closes RQ-404; amends R-474, R-475 and R-477)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"

## Deliverables
- R-514's frame, targets and carried pair (I1 to I5) in the GUI's `View::basis` and the Tilt section's target dropdowns (`crates/gui/src/explore/manifold_view/`), and TASK-M6-49's guard lifted. *Was:* "Per RQ-404's ruling, in the GUI's `View::basis` and the Tilt section's target dropdowns (`crates/gui/src/explore/manifold_view/`)."
- Test `saved_tilted_chart_tilt`.

## Acceptance tests
- `cargo test -p gui saved_tilted_chart_tilt` — REQ-GUI-265's verify.

## Notes
- **⚑ Physics review is required**: the tilt maths on a non-orthonormal saved pair; it does not merge on the code reviewer's word alone.
- **A leaf task waiting for RQ-404** (`rq: RQ-404` on REQ-GUI-265): nothing depends on it. *Superseded by the R-514 note below as to the wait; kept as history.* Numbered past the links port's (unmerged) TASK-M6-50 (applied per R-369).
- **R-514 (20 Oct 2026) rules RQ-404:** nothing waits. Decided per R-369 under physics review: the plane's own named dimensions are its first parent's axes (I2); a dimension vanishes at `|t|² ≤ 10⁻¹²`, REQ-GUI-203's threshold (I2); the tilted pair is the stored pair carried by the map, `(M q₁, M q₂)`, which keeps a Lean-saved pair's tilt continuous at `τ = 0` (I4, flagged); C5 is checked on the frame's images (I5). The qa test of TASK-M6-49's guard (REQ-GUI-240's tilt-saved arms) changes in this task's qa commit, the guard being lifted by this ruling (applied per R-369, R-514 I6).
