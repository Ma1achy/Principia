# TASK-M6-35 — The compass's Jacobi and Shape views and the physical steer (R-452)

- **Milestone:** M6
- **Closes:** REQ-GUI-199, REQ-GUI-200
- **Depends on:** TASK-M6-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
On the mock engine, the compass's Jacobi and Shape views push the slice plane through the decoder's Jacobian at `z₀`
(or at the hovered pixel) and draw the Plücker grid of the pushed-forward plane over eight physical coordinates, with
the dominant pair and the area stretch in the readout (R-452, 3.2); a click on a physical cell steers the slice to its
latent preimage, `q ∝ J⁺ e`, refused and logged where the preimage is no 2-plane (3.4). The human asked for physics
review of both coordinate sets and of the steer's construction before they are built: **this task waits on RQ-341
to RQ-346** (which Jacobian, the two coordinate sets, the steer, the energy normalisation's stage and the physical
metric), and builds what their rulings give.

## References
- `decisions.md` § "R-452 — The compass has four views, Cube (the default), Plücker, Jacobi and Shape, chosen on the compass and independent of its Tilt and Slice modes; a grid view draws the slice plane's Plücker coordinates, latent or pushed through the decoder, and a click on a cell steers the slice"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `docs/design/principia_deep_zoom.md` § "Couplings to the contract"
- `docs/design/principia_chart_reference.md` § "0.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_chart_reference.md` § "0.3 Momentum — free Jacobi momenta"
- `docs/design/principia_chart_reference.md` § "3.1 Forward map (already implemented as `shape_vec`)"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- The Jacobian the views read, as RQ-341's ruling defines it (proposed: a CPU, f64, central-difference `∂x/∂z` of
  the decoder's output in the view's coordinates at a latent point), evaluated at `z₀`, or at the hovered pixel's
  latent point on an affine chart while the pointer is over the figure (R-452 C4).
- The Jacobi and Shape views on TASK-M6-34's grid — `u₁ = J q₁`, `u₂ = J q₂` orthonormalised, the grid over the
  coordinates RQ-342's and RQ-343's rulings give, in the metric RQ-346's gives, read before or after `η_E` as RQ-345's
  gives, the readout's dominant pair and the area stretch (on orthonormalised `q̂`, as RQ-341 proposes); a degenerate
  pushforward (`J q₁ ∧ J q₂ = 0`) said, not drawn; the help text (the physical views change as the slice moves, the
  latent one does not).
- The physical steer — `q₁ ∝ J⁺ e_i`, `q₂ ∝ J⁺ e_j`, Gram–Schmidt orthonormalised, by RQ-344's construction (with a
  full-rank set, RQ-342 and RQ-343 point 6, `J⁺ = J⁻¹`; otherwise RQ-344's rank test, its tolerance, and how the
  Manifold view's sliders follow a general plane); a refused click logged with its reason; an accepted one a single
  `SetField` on the basis, logged, undone by one Ctrl+Z.
- A linear-decoder fixture (a known `J`) for the tests, and tests `compass_physical_views` and
  `mock_compass_steer_physical`; screenshot cases `01_main/mock_compass_jacobi` and `01_main/mock_compass_shape`.

## Acceptance tests
- `cargo test -p gui compass_physical_views` and `cargo xtask screenshot 01_main` (mock_compass_jacobi, mock_compass_shape) — on the linear-decoder fixture the grid equals the normalised bivector of J q₁ ∧ J q₂ within 1e-12 and the stretch the Gram factor; on the decoder a slice edit changes the physical grid where J depends on z₀ and leaves the Plücker grid unchanged; the hovered pixel's J is used while the pointer is over the figure; a degenerate pushforward shows its message; controls: the reference's synthetic Jacobian and un-orthonormalised u₁, u₂ each fail; review checklist (physics reviewer): the Jacobian, the coordinate sets, the stage read and the physical metric are as RQ-341 to RQ-343, RQ-345 and RQ-346's rulings give them (REQ-GUI-199).
- `cargo test -p gui mock_compass_steer_physical` — on the fixture a click on a physical cell with e_i, e_j in J's range steers to a plane whose pushforward lights (i, j) at ±1 within RQ-344's tolerance; a rank-deficient case is refused with a console line and no SetField; each steer is one SetField, logged, undone by one Ctrl+Z; controls: a steer accepted on a rank-deficient case and one without Gram–Schmidt each fail; review checklist (physics reviewer): the construction is RQ-344's, RQ-345's and RQ-346's rulings' (REQ-GUI-200).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: the Jacobi and Shape views; move the slice and
  watch them turn while the Plücker view stays; hover the figure; click a physical cell, then Ctrl+Z; a cell outside
  the Jacobian's range, refused, and its console line.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5. Its look choices (R-390) are recorded in the PR as "applied per R-369" and hold until the human has tried the
  mock (R-416).
- **Waits on RQ-341 to RQ-346** (the human's physics review, R-452; RQ-345 and RQ-346 filed in the fix round of
  PR #227). The task does not start until they are ruled;
  their rulings may change this file's deliverables and acceptance (a port of the ruling does so first).
- **The physics reviewer** checks the Jacobian, the coordinate sets and the steer's construction against the
  rulings, as R-452 asks.
