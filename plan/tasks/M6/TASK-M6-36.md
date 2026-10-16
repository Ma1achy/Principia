# TASK-M6-36 — The Trajectory panel: a click pins a trajectory, and its views pan, zoom and orbit (R-453)

- **Milestone:** M6
- **Closes:** REQ-GUI-201, REQ-GUI-202
- **Depends on:** TASK-M6-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
On the mock engine, a click on the figure pins that sample's trajectory in the Trajectory panel; hover does not
replace a pinned trajectory, and the cursor moving into the panel never clears it; and the panel's views are
interactive, the real-space view panning and zooming and the shape-sphere view orbiting and zooming (R-453). The
human found the panel unusable because the trajectory changed with every movement of the cursor and vanished when the
cursor reached the panel.

## References
- `decisions.md` § "R-453 — A click on the figure pins that sample's trajectory in the Trajectory panel, and hover never replaces it; the cursor moving into the panel never clears it; the panel's real-space view pans and zooms and its shape-sphere view orbits and zooms"
- `decisions.md` § "R-404 — #174's look choices and key-repeat timings are confirmed as built: the focus ring, the breadcrumb, the `?` overlay, the 500 ms delay and 40 ms interval, and the base steps"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-65 — One Inspector window; the standalone IC Inspector is absorbed *(closes RQ-21)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_trajectory_viewing.md` § "3. Hover trace — projection onto the current (tilted, sliced) plane"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `docs/design/principia_trajectory_viewing.md` § "6. Placement"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/explore/trajectory.rs` — the pin: the first tab showing the pinned sample while one is held,
  reading "pinned", with an unpin button (a scope element); without a pin the last hovered sample kept when the
  pointer leaves the figure (R-453 D2, D3); "+" with the mouse keeping the shown sample; the pin `ViewUI` state, its
  IC re-integrated when the sim key changes (D5).
- `crates/gui/src/explore/manifold_view/navigate.rs` and `crates/gui/src/app.rs` — a primary click with no drag on
  the figure pinning the sample under it (a drag still pans); P in the Figure scope pinning under the pointer, as K
  locks; the figure's drawn path following the pin (D1, D4, D6).
- The views — real space and the unwrapped view: drag pans, the wheel zooms about the pointer; the sphere: drag
  orbits (yaw, pitch clamped ±90°), the wheel zooms, a drag unticking "turn"; a double-click restoring each default;
  each view's state per tab, `ViewUI`, the orbit turning the view, never the data (D7, D8); the Inspector's panes
  getting the same where they are the same widgets (D8).
- `crates/gui/src/keyboard/scopes.rs` — the Trajectory scope's elements gaining unpin and the two views; in a view's
  interaction mode the arrows pan (R-404's `Pan`) or orbit (`Angle`), + and − zoom (`ZoomLog2`), Shift ×10 and Alt
  ×0.1 (D6); the `?` overlay's rows.
- Tests `mock_trajectory_pin` and `mock_trajectory_views`; `mock_hover` updated where it asserts the panel blanking
  off the figure; screenshot case `01_main/mock_trajectory_views`.

## Acceptance tests
- `cargo test -p gui mock_trajectory_pin` — a click pins and the tab reads "pinned"; hovering elsewhere, entering the panel and leaving the window keep it; a pan, a zoom and a slice edit keep it; another click moves it; unpin follows the cursor again; unpinned, entering the panel keeps the last sample; a drag pans and does not pin; P pins over the figure and does nothing off it; "+" with the mouse keeps the shown sample; nothing is added to undo; controls: a panel blanking off the figure, a hover replacing the pin and a pin holding a pixel each fail (REQ-GUI-201).
- `cargo test -p gui mock_trajectory_views` and `cargo xtask screenshot 01_main` (mock_trajectory_views) — drag and wheel pan and zoom real space about the pointer; drag and wheel orbit and zoom the sphere, pitch clamped, "turn" unticked and resumable; double-click restores; the sphere's data unchanged by an orbit; the keyboard steps as named, Shift ×10 and Alt ×0.1; no SetField and no undo entry; controls: a paint-only view, an orbit of the data and another keyboard step each fail (REQ-GUI-202).
- `cargo test -p gui mock_hover` — the hover label and the drawn path still follow the cursor without a pin (§G2's figure, re-checked here).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: hover the figure, click a point, move the pointer
  into the panel and play the orbit; click another point; unpin; drag and scroll on real space and on the sphere,
  double-click; Tab to Trajectory, Enter, the views from the keys; P over the figure.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5. Its look choices (R-390) are recorded in the PR as "applied per R-369" and hold until the human has tried the
  mock (R-416).
- **No camera, no compass orbit.** The views turn and zoom a drawing of one trajectory; §G1's "no camera" naming
  holds, and R-409's "no orbit" is the compass's, untouched (R-453).
- **qa's files (R-290, forced by R-453; R-453 D10).** A qa file asserting the panel blanking off the figure, or a
  click on the figure panning only, asserts what R-453 changes; this task's qa reviewer changes those assertions in the
  task's qa commit, and only those; the PR lists each `M` with its reason.
- **The real engine.** TASK-M8-11 (the real engine's Trajectory panel) is left as it is by this port, PR #226
  editing its file; a later port adds R-453's re-run there.
