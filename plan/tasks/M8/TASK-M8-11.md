# TASK-M8-11 — The Trajectory panel: summary, real space, the turning shape sphere, kept orbits

- **Milestone:** M8
- **Closes:** REQ-GUI-088, REQ-GUI-089, REQ-GUI-054, REQ-GUI-157
- **Depends on:** TASK-M8-09, TASK-M8-05
- **Needs (earlier milestones):** REQ-CHART-036, REQ-CHART-018, REQ-INT-003, REQ-VAL-023, REQ-PAY-033, REQ-PAY-047
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The right-hand Trajectory panel shows, for the IC under the cursor or a kept orbit (tabs "under cursor", #1, #2, +), the summary line, real space (CoM frame) beside the shape sphere, the F₂ word, substeps, minimum separation and |ΔE/E|, a playhead, a listen button, Open full viewer… and IC Inspector…, and the kept orbits. The shape-sphere widget draws n(t) as a polyline on S² with the landmarks from the actual masses, turns slowly with visible axes, stops when "turn" is cleared, and toggles to the equirect unwrap; the 3D and unwrapped views share the config-space frame and the orbit control rotates the view, not the data (R-14, R-50). The widget is shared with the Inspector's pane 2.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/gui/src/widgets/shape_sphere.rs` — sphere / unwrapped toggle, landmarks, turn checkbox, view-only rotation.
- `crates/gui/src/widgets/real_space.rs` — CoM-frame body paths.
- `crates/gui/src/explore/trajectory_panel.rs` — tabs, summary, readouts, kept orbits (ViewUI), both Inspector buttons.
- Tests: `sphere_unwrap_agree`; screenshot cases `01_main/trajectory_panel`, `01_main/sphere_unwrapped`, `01_main/turn_off`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (right panel) and `cargo test -p gui trajectory_buttons_open_inspector` — screenshot against 01_main.png's right panel; both buttons open the Inspector window (REQ-GUI-088).
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 05_inspectors` (sphere and unwrapped; turn cleared) — screenshots against 01_main.png and 05_inspectors.png in sphere and unwrapped modes; with 'turn' cleared two frames are identical (REQ-GUI-089).
- `cargo test -p gui sphere_unwrap_agree` — points picked in the 3D view and the unwrap map to the same n; orbiting leaves the stored n(t) unchanged (REQ-GUI-054).
- Proposal: the shape sphere's rotation rate in degrees per second, frame-rate independent; the human confirms it at the M8 gate (REQ-GUI-157).

## Notes
- The turning rate ("slowly") is not given. The listen button's sound is TASK-M8-12.
- Closes, for gaps the corpus leaves open: REQ-GUI-157 (R-71 calibration) (classification accepted by R-132).
