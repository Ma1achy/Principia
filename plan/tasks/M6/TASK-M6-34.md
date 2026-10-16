# TASK-M6-34 — The compass's views: the switcher, the Plücker view, the position readout, the latent steer and the keys (R-452)

- **Milestone:** M6
- **Closes:** REQ-GUI-194, REQ-GUI-195, REQ-GUI-196, REQ-GUI-197, REQ-GUI-198
- **Depends on:** TASK-M6-33
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~600 lines

## Goal
On the mock engine, the compass gets a view switcher on the compass itself, Cube (the default), Plücker, Jacobi and
Shape, independent of its Tilt and Slice modes (R-452). This task builds the switcher, the Plücker view (the slice
plane's Plücker coordinates in the latent chart, an 8 × 8 upper-triangle grid), the position readout under it, the
click on a Plücker cell that steers the slice to that coordinate plane, and the Compass scope's keys for the views. The
Jacobi and Shape views are TASK-M6-35's; here their buttons exist and their views say they wait on physics review.

## References
- `decisions.md` § "R-452 — The compass has four views, Cube (the default), Plücker, Jacobi and Shape, chosen on the compass and independent of its Tilt and Slice modes; a grid view draws the slice plane's Plücker coordinates, latent or pushed through the decoder, and a click on a cell steers the slice"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-404 — #174's look choices and key-repeat timings are confirmed as built: the focus ring, the breadcrumb, the `?` overlay, the 500 ms delay and 40 ms interval, and the base steps"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/explore/compass.rs` (or a `compass/` module) — the view switcher, four buttons beside Tilt and Slice,
  the view the compass's own GUI state (R-452 C1); a grid view ignoring drags.
- The Plücker view — `p_ij` from the decoder's `q₁`, `q₂` orthonormalised (Gram–Schmidt), never from the angles;
  the upper-triangle grid with the coordinate names on both edges, the diagonal empty; the signed two-hue colouring
  with intensity `|p|`; the dominant cell's outline (`|p| > 0.02`); the readout "strongest: z_a × z_b +0.993 · Σ p²
  1.000"; the value on hover as a tooltip beside the compass (C2, C3); the help text.
- The position readout under the Plücker view — `z₀` as signed bars, the slice axis highlighted.
- The latent steer — a click on cell (i, j) sets the preset to `(z_i, z_j)`, τ₁ = τ₂ = γ = 0, zoom and `z₀` kept,
  one `SetField` on the basis, logged in the console, undone by one Ctrl+Z; locked, about the pin (C5).
- `crates/gui/src/keyboard/scopes.rs` — the Compass scope's siblings extended to the compass, Tilt, Slice, Cube,
  Plücker, Jacobi, Shape; Enter on a view button switching the view; in a grid view the compass's interaction moving a
  cell cursor over the upper triangle, Space steering to it, Esc or Enter back (C6); the `?` overlay's rows and the
  breadcrumb.
- The Jacobi and Shape buttons present, their views showing "waits on physics review (RQ-341 to RQ-346)" until
  TASK-M6-35.
- Tests `mock_compass_views`, `compass_plucker`, `compass_position_gauge`, `mock_compass_steer_latent`,
  `mock_compass_view_keys`; screenshot cases `01_main/mock_compass_views` and `01_main/mock_compass_plucker`.

## Acceptance tests
- `cargo test -p gui mock_compass_views` and `cargo xtask screenshot 01_main` (mock_compass_views) — the compass opens on Cube; each view button shows its view and leaves the mode; Tilt and Slice leave the view; touching a slice slider in the Plücker view sets Slice and keeps Plücker; a drag in a grid view and a view switch emit no SetField and add nothing to undo; controls: a view switch that changes the mode and a mode switch that changes the view each fail (REQ-GUI-194).
- `cargo test -p gui compass_plucker` and `cargo xtask screenshot 01_main` (mock_compass_plucker) — Σ p² = 1 within 1e-12 over random, non-orthonormal inputs; an axis-aligned plane lights exactly one cell at ±1; a rotation within the plane leaves every cell unchanged within 1e-12; the outline and the readout name the largest |p|; hover shows a cell's names and value; controls: a grid from the tilt angles and one from an un-orthonormalised basis each fail (REQ-GUI-195).
- `cargo test -p gui compass_position_gauge` — each bar is its z₀ component over the half-range, clamped; the slice axis is highlighted and follows the preset; a slice edit moves its bar and leaves the Plücker grid unchanged; the readout takes no input (REQ-GUI-196).
- `cargo test -p gui mock_compass_steer_latent` — every cell (i, j) steers to a plane lighting (i, j) at ±1, one SetField, logged, the Manifold view showing z_i × z_j with tilts 0, undone exactly by one Ctrl+Z; z₀ kept, the anchor kept when locked; controls: a steer that leaves a tilt, two SetFields and a diagonal click that steers each fail (REQ-GUI-197).
- `cargo test -p gui mock_compass_view_keys` — the seven siblings in order both ways; Enter on each view button switches view in navigation mode; in the Plücker view Enter on the compass starts interaction on the dominant cell, the arrows move over upper cells only, Space steers (undone by one Ctrl+Z), Esc returns; in the Cube view the arrows tilt as before; controls: another sibling order, a cursor off the upper triangle and a Space that steers in navigation mode each fail (REQ-GUI-198).
- `cargo test -p gui mock_compass_keys` — the compass, Tilt and Slice keep their order and keys (REQ-GUI-183, closed by TASK-M6-31, re-checked here with the view buttons after them).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: the four view buttons; tilt with the Plücker
  view open and watch one cell's weight slide along its row; click a cell and Ctrl+Z; move the slice and watch the
  position bars move and the grid stay; Tab to the Compass, Enter, → to Plücker, Enter, ← to the compass, Enter, the
  arrows, Space, Esc.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5. Its look choices (R-390) are recorded in the PR as "applied per R-369" and hold until the human has tried the
  mock (R-416).
  The grid's two hues, its outline and its cell cursor's look are such choices (R-452 C7); the references' teal and
  terracotta on cream are not used.
- **Why apart from the physical views.** The Plücker view, the readout, the latent steer and the keys rest on the
  decoder's basis alone and need none of RQ-341 to RQ-346, so they are built first; TASK-M6-35 adds the physical views
  on this task's grid.
