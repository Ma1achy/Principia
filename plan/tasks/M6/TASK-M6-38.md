# TASK-M6-38 — The Navigate, Slice centre and Tilt sections, and a target per tilt angle (R-457)

- **Milestone:** M6
- **Closes:** REQ-GUI-204, REQ-GUI-205
- **Depends on:** TASK-M6-33
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
On the mock engine, the Manifold view's Navigate section loses its grid of eight `z₀` boxes, which repeated the Slice centre sliders (`manifold_view/navigate.rs`, the grid at lines 98 to 123 at `ab87a879`); "Centre z₀" becomes **Slice centre** and "Slice & tilt" **Tilt**; Tilt loses its step slider, which set `z₀`'s component on the slice axis, the same value as that dimension's Slice centre slider; and each tilt angle gets its own target, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", dropdowns of the dimensions not in the plane, defaulting to today's one shared target, so the defaults give today's basis exactly (R-457). The targets, with the slice normal, become `SimConfig`'s slice frame, undoable and read by no kernel (gui_state_contract §2); the compass's readout names both targets.

## References
- `decisions.md` § "R-457 — The Manifold view's Navigate section loses its grid of eight `z₀` boxes; "Centre z₀" is renamed "Slice centre" and "Slice & tilt" "Tilt"; each tilt angle tilts into a latent dimension the user picks, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", the defaults today's one shared target; the step slider is removed *(amends render_gui_spec §G2's Navigate and Slice & tilt, R-409 A3's "the slice step" and R-451 B1's single tilt target)*"
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping *(amends R-409 point 1, the compass's view angle)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- The Navigate, Slice centre and Tilt sections as R-457 gives them (`crates/gui/src/explore/manifold_view/`), the breadcrumb and the sub-scope names following.
- `View::basis` with a target per angle, `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, then `γ` as now; the two dropdowns and their keys (R-457 E5).
- `SliceFrame { normal, tilt_x, tilt_y }` in `SimConfig` (`crates/engine/src/contract/`), on both engines' conformance suite (R-390), with its `SetField` path; a pick that edits the slice frame and the basis is one history entry.
- The compass's readout naming both targets with their angles (on TASK-M6-33's compass).
- Tests `mock_manifold_sections` and `tilt_targets`.

## Acceptance tests
- cargo test -p gui mock_manifold_sections: the Navigate section's widgets are centre u, v, zoom, the depth readout, the precision warning and the lock row, and no widget there edits a z₀ component; Slice centre has eight sliders, each with a number box that edits the same component; Tilt has τ₁, τ₂ and γ with number boxes and no widget that edits z₀; the breadcrumb on a Tilt slider reads 'Manifold view › Tilt › τ₁' and Enter on Manifold view then ↓ reaches Chart, Navigate, Slice centre and Tilt in that order; controls: a panel with the eight z₀ boxes under Navigate, a Tilt section with a step slider, and the old names each fail; cargo xtask screenshot 01_main (mock_manifold_view) (REQ-GUI-204).
- cargo test -p gui tilt_targets: with the default targets, View::basis equals today's map at a sweep of (τ₁, τ₂, γ, σ) to 1e-15; with X = z_q3, Y = z_μ1, τ₁ = 30°, τ₂ = 20°, γ = 0, q₁ = σ(cos 30° e_a + sin 30° e_q3) and q₂ = σ(cos 20° e_b + sin 20° e_μ1), and q₁·q₂ = 0; each dropdown lists exactly the six dimensions not e_a, e_b; at τ₁ = 30° a change of X from z_q3 to z_q2 sends one history entry whose basis tilts q₁ into e_q2, and one Ctrl+Z restores the basis and X = z_q3; at τ₁ = 0 a change of X sends one entry on the slice frame and no basis edit, undone by one Ctrl+Z; a preset change to a plane containing X resets X to the slice normal and the angles to 0; the readout reads both targets with their angles; controls: a dropdown that offers e_a, a target change that makes two history entries, a target change that does not re-tilt at a non-zero angle, and an undo that restores the basis but not the target each fail (REQ-GUI-205).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: open the Manifold view: Navigate has no z₀ boxes; the sections read Slice centre and Tilt; pick "tilt τ₁ into z_μ1", drag τ₁ and watch the figure tilt into the mass; Ctrl+Z restores it.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **RQ-340.** With `X = Y` the map is the code's (option (a), as applied); the task does not change it. With `X ≠ Y` it is the contract's two-target form. B11's corner points arise only with `X = Y`.
- **The cube's labels and the badge** are TASK-M6-40's (R-459); this task gives the compass only the readout's two targets.
- **qa's files (R-290, R-457 E9).** A qa assertion of the Navigate grid, the step slider, the old names or the shared target asserts what R-457 changes; this task's qa reviewer changes those, and only those, and the PR lists each.
- **Early start (R-388).** It may start off TASK-M6-33's approved head (PR #240) and opens its PR once that merges.
