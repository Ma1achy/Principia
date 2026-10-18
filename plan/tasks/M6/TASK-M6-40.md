# TASK-M6-40 — The cube's H, V and D buttons, the third axis that follows the gesture, and the "4D: see Plücker" badge (R-459)

- **Milestone:** M6
- **Closes:** REQ-GUI-208, REQ-GUI-209
- **Depends on:** TASK-M6-34, TASK-M6-38
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
On the mock engine, the compass's Cube view labels its edges with buttons in both modes, H (chart x), V (chart y) and the third axis (D, the slice normal, in Slice mode), each opening a dropdown of the dimensions not on the other two, each pick one undoable edit, H and V in step with Chart's horizontal q₁ and vertical q₂ (R-459). In Tilt mode the third axis shows what the gesture tilts into (X while τ₁ is dragged, Y while τ₂ is; at rest X if τ₁ ≠ 0, else Y if τ₂ ≠ 0, else the slice normal), in the active gesture's colour, the cube drawing that gesture's plane exactly; both targets are always in the readout; with both angles set and X ≠ Y a "4D: see Plücker" badge switches the compass to the Plücker view.

## References
- `decisions.md` § "R-459 — The cube's H, V and D axis labels are buttons, each a dropdown of latent dimensions, no dimension twice, each pick one undoable edit, H and V in step with Chart's horizontal q₁ and vertical q₂; in Tilt mode the third axis shows what the gesture tilts into, and a "4D: see Plücker" badge appears when both angles are set into different targets *(amends R-451 B1's labels)*"
- `decisions.md` § "R-457 — The Manifold view's Navigate section loses its grid of eight `z₀` boxes; "Centre z₀" is renamed "Slice centre" and "Slice & tilt" "Tilt"; each tilt angle tilts into a latent dimension the user picks, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", the defaults today's one shared target; the step slider is removed *(amends render_gui_spec §G2's Navigate and Slice & tilt, R-409 A3's "the slice step" and R-451 B1's single tilt target)*"
- `decisions.md` § "R-458 — Drags are live in both directions: a drag on the compass or on any slider applies its edit every frame, at most once a frame, so the sliders, the compass, its readout and the figure follow the drag, and the whole drag is one undo entry; a drag on a Slice centre slider switches the compass to Slice and one on a Tilt slider to Tilt, its plane moving with the drag *(amends R-409 A3 and R-451 B3 as they send one `SetField` per drag)*"
- `decisions.md` § "R-452 — The compass has four views, Cube (the default), Plücker, Jacobi and Shape, chosen on the compass and independent of its Tilt and Slice modes; a grid view draws the slice plane's Plücker coordinates, latent or pushed through the decoder, and a click on a cell steers the slice"
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping *(amends R-409 point 1, the compass's view angle)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- The H, V and D buttons and their dropdowns on R-451 B1's edges (`crates/gui/src/explore/compass.rs`), H and V setting the axis pair as the preset does, D the slice frame's normal (R-459 G1 to G4).
- The third axis's rule, its colour and the gesture's plane drawn exactly (G5 to G7); the badge and its click (G8); the Compass scope's new siblings (G9).
- Tests `mock_cube_axis_buttons` and `mock_cube_third_axis`, and the screenshot cases `mock_compass_cube_third_axis` and `mock_compass_cube_badge`.

## Acceptance tests
- cargo test -p gui mock_cube_axis_buttons: the three buttons sit on R-451 B1's edges with 'H z_α', 'V z_β', 'D z_q3' at the default; each dropdown lists the dimensions not on the other two; picking H = z_q0 sets q₁ ∝ e_q0, resets τ₁, τ₂, γ to 0 and keeps zoom and z₀, Chart's horizontal q₁ reading z_q0; editing q₂ in Chart to e_q1 moves V to z_q1; picking D = z_μ1 sets the slice frame's normal and, at τ₁ = 20° with X = D, re-tilts q₁ into e_μ1 in one entry; each pick is undone by one Ctrl+Z; a Plücker steer onto z_q2 × z_q3 with D = z_q3 moves D by the slice-axis rule; keyboard: Tab to the Compass, Enter, → to H, Enter, ↓, Enter picks; controls: a dropdown that offers V's dimension in H's list, and a D pick that makes two entries, each fail (REQ-GUI-208).
- cargo test -p gui mock_cube_third_axis: with X = z_q3, Y = z_μ1, the third axis reads D in Slice mode; in Tilt mode with τ₁ = τ₂ = 0 it reads the slice normal, with τ₁ = 20° X, with τ₁ = 0 and τ₂ = 15° Y; during a τ₂ slider drag with τ₁ = 20° it reads Y, back to X when the drag ends; a compass drag moving τ₁ by 10° and τ₂ by 30° reads Y; its label's colour is the active gesture's; at τ₁ = 20°, τ₂ = 15° the cube's polygon equals the independent computation of View::basis at (20°, 0°) in (e_a, e_b, e_q3) and the badge is shown, a click on it setting the view to Plücker; with X = Y = z_q3 at the same angles no badge is shown and the polygon is R-451 B2's; the readout names both targets in every case; controls: a third axis that stays on the slice normal in Tilt mode, a badge shown with X = Y, and a readout naming one target each fail; cargo xtask screenshot compass (mock_compass_cube_third_axis, mock_compass_cube_badge) (REQ-GUI-209).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: pick H = z_q0 on the cube and watch Chart's horizontal q₁ follow; set τ₁ into z_q3 and τ₂ into z_μ1, both non-zero, and see the third axis follow the slider you drag and the "4D: see Plücker" badge; click it.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **The gesture colours** (slice, τ₁, τ₂) are dark-mode look choices under R-390, listed in the PR (R-459 G7).
- **qa's files (R-290, R-459 G11).** A qa assertion of R-451 B1's single tilt-mode label, or of the unnamed edges, asserts what R-459 changes; this task's qa reviewer changes those, and only those.
- **Early start (R-388).** It may start off the approved head of whichever of TASK-M6-34 and TASK-M6-38 merges last, the other merged.
