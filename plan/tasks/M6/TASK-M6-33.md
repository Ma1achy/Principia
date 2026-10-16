# TASK-M6-33 — The compass's cube as the nav-cube reference draws it (R-451)

- **Milestone:** M6
- **Closes:** REQ-GUI-191, REQ-GUI-192, REQ-GUI-193
- **Depends on:** TASK-M6-31
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~550 lines

## Goal
On the mock engine, the compass's cube is drawn and behaves as `docs/gui/reference/principia_nav_cube.html` draws its
cube (R-451), transcribed from the file's code, in the dev GUI's dark-mode colours: isometric, the edges depth-sorted,
the slice its true cross-section polygon, Lambert-shaded, with an in-plane grid and an outline, the reference's edge
labels and readout, its drag rates and its easing. What R-409 settled stays: no orbit, the Tilt and Slice buttons and
the switch by the slider touched, Tilt the default, the gold pin, §G3's keys. The plane drawn is the decoder's, as
`View::basis` builds it, not the reference's axis-angle map (RQ-340 holds that difference).

## References
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-404 — #174's look choices and key-repeat timings are confirmed as built: the focus ring, the breadcrumb, the `?` overlay, the 500 ms delay and 40 ms interval, and the base steps"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"

## Deliverables
- `crates/gui/src/explore/compass.rs` — the yaw-and-pitch `project` replaced by the reference's isometric
  `P(x,y,z)` and fit (render_gui_spec § "The compass: look and views (R-451, R-452)"); the edges depth-sorted with the
  reference's opacity and width laws; the square slice (`PLANE_HALF`) and the dashed untilted plane replaced by the
  cross-section polygon of the plane with `[−1,1]³` (3 to 6 vertices, ordered), slice mode untilted at its depth and
  tilt mode as `View::basis` tilts it in the frame `(e_a, e_b, e_d)` (R-451 B2), sorted among the edges by its depth,
  Lambert-shaded (light, view, ambient and depth term as the reference's), with the in-plane grid clipped to it and
  the outline; the labels and the corner numbers (R-451 B1); the readout per mode (B5); the drags at the reference's
  rates, slice mode along R-409 A3's normal (B3); the easing in the drawing only (B4); the gold pin when locked.
- The cross-section as a pure function (plane in the cube's frame → ordered polygon), unit-tested apart from the
  drawing.
- Golden-image fixtures: renders of the reference at the compass's size and the cases' parameters, with a note of
  how each was made (R-451 B8), and the comparison of geometry and labelling (vertices, edge order, label text and
  placement, within a stated tolerance), not colour.
- Tests `compass_cross_section`, `mock_compass_cube`; `mock_compass` and `mock_compass_modes` updated where they
  assert the old projection, the square slice or the drag rates; screenshot cases `01_main/mock_compass_cube_slice`,
  `01_main/mock_compass_cube_tilt_0`, `01_main/mock_compass_cube_tilt_rect`, `01_main/mock_compass_cube_tilt_hex`, and
  `01_main/mock_compass_tilt`, `01_main/mock_compass_slice` and `08_lock/mock_locked` re-captured.

## Acceptance tests
- `cargo xtask screenshot 01_main` (mock_compass_cube_slice, mock_compass_cube_tilt_0, mock_compass_cube_tilt_rect, mock_compass_cube_tilt_hex) — golden images of the cube in each mode and at 0°, a single tilt (a rectangle) and two tilts cutting six faces (a hexagon), compared with the reference's renders at the same size and parameters in geometry and labelling, not colour; no cream colour appears in any capture; controls: a yaw-and-pitch projection, a fixed-square slice and a label on the wrong edge each fail (REQ-GUI-191).
- `cargo test -p gui compass_cross_section` — over a sweep of τ₁, τ₂ ∈ [−90°, 90°], γ and depths in [−1, 1] the polygon has 3 to 6 vertices, each on the plane and inside the cube within 1e-12, convex and ordered; untilted it is the rectangle at the depth; the drawn plane's normal equals that of `View::basis`'s (q₁, q₂) in the cube's frame within 1e-12; controls: a fixed-square polygon and the reference's axis-angle normal at τ₁ = τ₂ = 30° each fail (REQ-GUI-192).
- `cargo test -p gui mock_compass_cube` — the readouts per mode; a half-width horizontal drag in slice mode emits one SetField on z₀ of half the range along the normal, a vertical one nothing; in tilt mode a full-width drag sets τ₁ to the clamp and a quarter-height drag up raises τ₂ by 45°; the SetField carries the target and the drawn value eases by the law at dt = 1/60 s and at the 0.05 s cap; controls: an unclamped drag, an eased value sent through the contract and a slice-mode vertical drag that moves the slice each fail (REQ-GUI-193).
- `cargo test -p gui mock_compass_modes mock_compass_keys` and `cargo xtask screenshot 08_lock` (mock_locked) — the Tilt and Slice buttons, the switch by the slider touched, Tilt the default, no orbit, the compass's keys and the gold pin as R-409 has them (REQ-GUI-181 and REQ-GUI-183, closed by TASK-M6-31, re-checked here).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: the compass in Tilt, dragging across and
  up; Slice and a horizontal drag; a tilt that makes a hexagon; lock (K) and the gold pin; watch the values ease.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5. Its look choices (R-390) are recorded in the PR as "applied per R-369" and hold until the human has tried the
  mock (R-416).
  The colours (the fill's two tones, the edges', the grid's and the outline's) are such choices; the reference's cream
  palette is not used anywhere.
- **The tilt mapping (RQ-340).** The drawing follows the decoder's mapping, as R-451 says; RQ-340 is open for the
  human and this task does not wait. If it is ruled for the reference's two targets, tilt mode's labels follow the
  reference (R-451 B1) in a follow-up task.
- **qa's files (R-290, forced by R-451; R-451 B10).** `crates/gui/tests/qa_TASK-M6-26.rs` and
  `crates/gui/tests/qa_TASK-M6-31.rs`, where they assert the compass's yaw-and-pitch view, its square slice or its
  drag rates, assert what R-451 changes; this task's qa reviewer changes those assertions in the task's qa commit, and
  only those, every earlier commit of each being qa's; the implementer never edits them; the PR lists each `M` with its
  reason, and the code reviewer confirms no other assertion was weakened.
- **The real engine.** TASK-M8-07 re-runs the compass there as REQ-GUI-091 words it; R-451's drawing is the mock's
  GUI code, which the real engine's GUI shares.
