# TASK-M8-11 — The Trajectory panel: summary, real space, the turning shape sphere, kept orbits

- **Milestone:** M8
- **Closes:** REQ-GUI-088, REQ-GUI-089, REQ-GUI-054
- **Depends on:** TASK-M8-09, TASK-M8-87, TASK-M6-27, TASK-M2-33
- **Needs (earlier milestones):** REQ-CHART-036, REQ-CHART-018, REQ-INT-003, REQ-VAL-023, REQ-PAY-033, REQ-PAY-047, REQ-GUI-188
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
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"

## Deliverables
- `crates/gui/src/widgets/shape_sphere.rs` — sphere / unwrapped toggle, landmarks, turn checkbox, view-only rotation.
- `crates/gui/src/widgets/real_space.rs` — CoM-frame body paths.
- `crates/gui/src/explore/trajectory.rs` — tabs, summary, readouts, kept orbits (ViewUI), both Inspector buttons.
- Tests: `sphere_unwrap_agree`; screenshot cases `01_main/trajectory_panel`, `01_main/sphere_unwrapped`, `01_main/turn_off`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (right panel) and `cargo test -p gui trajectory_buttons_open_inspector` — screenshot against 01_main.png's right panel; both buttons open the Inspector window (REQ-GUI-088).
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 05_inspectors` (sphere and unwrapped; turn cleared) — screenshots against 01_main.png and 05_inspectors.png in sphere and unwrapped modes; with 'turn' cleared two frames are identical (REQ-GUI-089).
- `cargo test -p gui sphere_unwrap_agree` — points picked in the 3D view and the unwrap map to the same n; orbiting leaves the stored n(t) unchanged; the figure-eight run through a collinear configuration draws n(t) across φ = π/2 at the signed n̂ in both views, with the canonical bodies shown and hidden; control: a path drawn from the canonical representative's n (|n₂|), which bounces at the equator, fails (REQ-GUI-054, R-544 item 8).
- Proposal: the shape sphere's rotation rate in degrees per second, frame-rate independent; the human confirms it at the M8 gate (REQ-GUI-157). Proposed by TASK-M6-27 since R-390, the first task to need the value; this task uses it.

## Notes
- The turning rate ("slowly") is not given. The listen button's sound is TASK-M8-12.
- For gaps the corpus leaves open: REQ-GUI-157 (R-71 calibration) (classification accepted by R-132), closed by TASK-M6-27 since R-390 and used here.
- R-390: TASK-M6-27 builds the Trajectory panel's frame on the mock engine and proposes the rotation rate (REQ-GUI-157, the first task to need it); this task depends on it, uses that value, fills the panel from the real engine and keeps REQ-GUI-088, REQ-GUI-089 and REQ-GUI-054.
- The Trajectory panel's file is `crates/gui/src/explore/trajectory.rs`, the one TASK-M6-27 creates for the panel's
  frame (its Deliverables name `crates/gui/src/explore/{time,legend,trajectory}.rs`); this task fills it, applied per
  R-369 (TASK-M1-14's pre-flight; was `trajectory_panel.rs`).
- R-449 (16 Oct 2026): the shape sphere's landmark labels, the kept orbits' rows and the summary name the bodies 1-based through TASK-M2-33's conversion function: `BC₀₁`, `BC₁₂`, `BC₂₀` are shown `BC₁₂`, `BC₂₃`, `BC₁₃` (dd_integrator §3.7). The Euler landmarks: `E_j` is the collinear configuration with body `j` in the middle, shown `E_{j+1}` through the same function, its antipodality to the other two bodies' collision stated for equal masses only (R-549, RQ-334; REQ-GUI-054). *Was:* "The Euler landmarks' index is not defined; this task writes how they are shown (R-72), its reviewers deciding per R-369: RQ-334."
- R-541 G4 (21 Oct 2026): the kept orbits become the header's tabs, each with its own previews, trace and sound
  (REQ-GUI-088, restated); this task builds the tabs and the panel's items as REQ-GUI-088 has them, and TASK-M8-61 the
  order, the status line and the Trace and Numbers sections over them (REQ-GUI-326), TASK-M8-64 the Listen section
  (REQ-GUI-334; PR #267's split, *was:* "the Trace, Listen and Numbers sections over them (REQ-GUI-326)"). What G4 leaves out (FTLE,
  `E₀`, the playhead, the two buttons) is kept: RQ-462 (a), ruled so by R-556.
- R-544 (21 Oct 2026) item 8: "Show the canonical bodies" may stay as a view of the bodies, but must not fold the path; REQ-GUI-054 gains the syzygy arm (gui review 5603356729 of PR #273, G4, applied per R-369).
- R-627 (27 Oct 2026): this task runs on the real engine, so where it depended on a task R-627 moves to the mock it depends on that task's real-engine side, which depends on it: TASK-M8-05 by TASK-M8-87 (applied per R-369). *Was:* "- **Depends on:** TASK-M8-09, TASK-M8-05, TASK-M6-27, TASK-M2-33".
