# TASK-M6-35 — The compass's Jacobi and Shape views and the physical steer (R-452)

- **Milestone:** M6
- **Closes:** REQ-GUI-199, REQ-GUI-200, REQ-DEC-049, REQ-GUI-346, REQ-GUI-347, REQ-GUI-410, REQ-GUI-411
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
review of both coordinate sets and of the steer's construction before they are built; **R-550 (21 Oct 2026) rules
RQ-341 to RQ-346**, and this task builds what it gives: the latent Jacobian, the full-rank Jacobi and Shape sets, the
state after `η_E`, the fixed-range metric, and the steer by `J⁻¹` with the projector test's refusal.

## References
- `decisions.md` § "R-452 — The compass has four views, Cube (the default), Plücker, Jacobi and Shape, chosen on the compass and independent of its Tilt and Slice modes; a grid view draws the slice plane's Plücker coordinates, latent or pushed through the decoder, and a click on a cell steers the slice"
- `decisions.md` § "R-550 — The compass's physical views and steer: an eight-column f64 central-difference Jacobian `∂x/∂z` of the decoder, read after `η_E`; Jacobi's eight `α, β`, the four Jacobi momenta and two mass fractions, Shape's `θ, φ, θ̇, φ̇, L_z, Ṙ` and the two mass fractions, both of full rank, so the steer is `J⁻¹` with the projector test refusing; each coordinate divided by a fixed reference range *(closes RQ-341, RQ-342, RQ-343, RQ-344, RQ-345 and RQ-346; amends R-452)*"
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
- The latent Jacobian (REQ-DEC-049, a definition, R-72): a CPU, f64, central-difference `∂x/∂z` with 8 columns of
  the decoder's output in the view's coordinates, after canonicalisation, the scale gauge and `η_E`, with `J_D`'s step
  rule (deep_zoom §2), evaluated at `z₀`, or at the hovered pixel's latent point on an affine chart while the pointer is
  over the figure (R-452 C4; R-550 J2). The step is `J_D`'s as R-633 writes it into deep_zoom §2:
  `h_k = ε^{1/3}·max(1, |z_k|)` in f64, the difference divided by the step actually taken, `(z + h) − (z − h)`.
- The Jacobi and Shape views on TASK-M6-34's grid — `u₁ = J q̂₁`, `u₂ = J q̂₂`, the grid over Jacobi's
  `α, β, p_ρₓ, p_ρᵧ, p_λₓ, p_λᵧ, m₀/M, m₁/M` and Shape's `θ, φ, θ̇, φ̇, L_z, Ṙ, m₀/M, m₁/M` (R-550), each divided by
  its fixed reference range (angles by their domain, masses by 1, momenta by `q_max`, Shape's rates and `L_z` by the
  ranges this task writes, REQ-GUI-347, a definition under physics review), the readout's dominant pair and the
  area stretch on the orthonormalised pair; a degenerate pushforward (`J q̂₁ ∧ J q̂₂ = 0`) said, not drawn; the help
  text (the physical views change as the slice moves, the latent one does not; the metric named).
- The physical steer — `q₁ ∝ J⁺ e_i`, `q₂ ∝ J⁺ e_j`, Gram–Schmidt orthonormalised, `J⁺` formed from `J`'s SVD with
  every singular value below `c·σ₁` dropped (R-633; `c` REQ-GUI-410's calibration, starting at `1e-9`, proposed here
  with the measured gap as its basis), so `J⁺ = J⁻¹` where nothing is dropped; refused, its reason logged, where
  the smaller singular value of `[P e_i, P e_j]` (`P = J J⁺`) is below the tolerance this task proposes
  (REQ-GUI-346, a calibration confirmed at the M6 gate), "normal to the energy shell" on an `E*`-normalised chart;
  the Manifold view's sliders following as after the Chart builder's Apply (§G7); an accepted steer a single
  `SetField` on the basis, logged, undone by one Ctrl+Z.
- *Was (before R-550):* the deliverables read the Jacobian, the coordinate sets, the stage and the metric "as RQ-341's
  … RQ-346's rulings give", with `u₁ = J q₁`, `u₂ = J q₂` orthonormalised and the steer by `J⁺` "by RQ-344's
  construction".
- A linear-decoder fixture (a known `J`) for the tests, `E* = 0`, `E*`-unset and saturated-link fixtures for the cut,
  and tests `compass_physical_views`, `mock_compass_steer_physical` and `mock_compass_steer_cut`; screenshot cases `01_main/mock_compass_jacobi` and `01_main/mock_compass_shape`.

## Acceptance tests
- `cargo test -p gui compass_physical_views` and `cargo xtask screenshot 01_main` (mock_compass_jacobi, mock_compass_shape) — on the linear-decoder fixture the grid equals the normalised bivector of J q₁ ∧ J q₂ within 1e-12 and the stretch the Gram factor; on the decoder a slice edit changes the physical grid where J depends on z₀ and leaves the Plücker grid unchanged; the hovered pixel's J is used while the pointer is over the figure; a degenerate pushforward shows its message; controls: the reference's synthetic Jacobian and un-orthonormalised u₁, u₂ each fail; review checklist (physics reviewer): the Jacobian, the coordinate sets, the stage read and the physical metric are R-550's (REQ-GUI-199).
- `cargo test -p gui compass_latent_jacobian` — REQ-DEC-049's verify (the eight columns, the stage after `η_E`, the stretch on the orthonormalised pair).
- The reference ranges (REQ-GUI-347), the steer's tolerance (REQ-GUI-346) and the pole's radius (REQ-GUI-411): written into render_gui_spec's compass section and the PR's calibration record; review checklist (physics reviewer).
- `cargo test -p gui mock_compass_steer_cut` — REQ-GUI-410's verify: at `c = 1e-9`, the `E* = 0` fixture refuses the shell-normal direction at every sampled `z`, the `E*`-unset fixture refuses none, a saturated-link fixture refuses; controls: no cut, and a cut at `1e-3`, each fail; the cut and its measured gap in the PR's calibration record; review checklist (physics reviewer) (R-633).
- `cargo test -p gui mock_compass_steer_physical` — on the fixture a click on a physical cell with e_i, e_j in J's range steers to a plane whose pushforward lights (i, j) at ±1 within REQ-GUI-346's tolerance; a cell needing the energy shell's normal on an `E*` chart is refused with "normal to the energy shell" and no SetField; each steer is one SetField, logged, undone by one Ctrl+Z; controls: a steer accepted on a refused case and one without Gram–Schmidt each fail; review checklist (physics reviewer): the construction is R-550's (REQ-GUI-200).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: the Jacobi and Shape views; move the slice and
  watch them turn while the Plücker view stays; hover the figure; click a physical cell, then Ctrl+Z; a cell outside
  the Jacobian's range, refused, and its console line.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5. Its look choices (R-390) are recorded in the PR as "applied per R-369" and hold until the human has tried the
  mock (R-416).
- **No longer waits:** R-550 (21 Oct 2026) rules RQ-341 to RQ-346, the human's physics review (R-452), and its port
  has brought this file's deliverables and acceptance in line. *Was:* "**Waits on RQ-341 to RQ-346** (the human's
  physics review, R-452; RQ-345 and RQ-346 filed in the fix round of PR #227). The task does not start until they are
  ruled; their rulings may change this file's deliverables and acceptance (a port of the ruling does so first)."
- **The physics reviewer** checks the Jacobian, the coordinate sets and the steer's construction against the
  rulings, as R-452 asks.
- **R-633 (28 Oct 2026; RQ-600 (a), RQ-601 (a)):** the task no longer waits on RQ-600 or RQ-601. The latent Jacobian's
  step is `J_D`'s, written into deep_zoom §2, so REQ-DEC-049's truncation bound is stated against it. `J⁺` drops each
  singular value below `c·σ₁`, `c` a second calibration (REQ-GUI-410, added to Closes), starting at `1e-9` on the
  measured gap; the steer reads `J⁺` where the cut drops one (applied per R-369; the physics reviewer checks it). The
  analytic energy-normal projector (RQ-601 (c)) is noted for later and not built. *Was (deliverables):* "The physical
  steer — `q₁ ∝ J⁻¹ e_i`, `q₂ ∝ J⁻¹ e_j`, Gram–Schmidt orthonormalised;" and "tests `compass_physical_views` and
  `mock_compass_steer_physical`".
- **REQ-GUI-411 (R-71; the code review of PR #365, F4):** naming a refusal for the pole and the readout's pole line need
  a radius on `sin φ`, and the corpus sets none, so it is a calibration this task proposes, measured in
  `mock_compass_steer_cut`. *Was (Closes):* "REQ-GUI-199, REQ-GUI-200, REQ-DEC-049, REQ-GUI-346, REQ-GUI-347,
  REQ-GUI-410". *Was (acceptance):* "The reference ranges (REQ-GUI-347) and the steer's tolerance (REQ-GUI-346):
  written".
