# TASK-M6-26 — The GUI track (R-390), 3: the Manifold view group and the compass

- **Milestone:** M6
- **Closes:** REQ-GUI-170, REQ-GUI-171, REQ-GUI-081, REQ-GUI-091
- **Depends on:** TASK-M6-25
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~550 lines

## Goal
On the mock engine, the left panel is one "Manifold view" group, as `01_main.png` and the design notes give it: Chart
(the preset named by its axes, the basis vectors `q₁`, `q₂` with their edit buttons, Chart builder…, the chart's kind),
Navigate (centre `(u, v)`, zoom as log₂, all eight `z₀` values editable by drag and by typing), the depth readout with
its precision warning raised from the snapshot's event fields, Lock, Centre z₀ (eight sliders) and Slice & tilt (the
slice step, `τ₁`, `τ₂`, the rotation `γ`). Every edit is a `SetField` on `z₀` or the basis: navigation is chart
construction, with no camera. The figure's stand-in pans and zooms with it: the mock redraws it for the new chart. Lock
(K, or right-click → lock here) re-bases the sliders to the anchor, each showing anchor plus offset, never freezing
them. The compass sits bottom left, shows the slice plane inside the chart, switches between slicing and tilting by
which slider was touched, carries the gold pin when locked, tilts when the plane is dragged and orbits when the cube is
dragged, and reads out the tilt and rotation angles. Each control joins the keyboard scope tree (TASK-M6-25).

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "08 Lock — the reticle and the pin"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-54 — The precision warning is event-driven *(GU-3 (b))*"
- `decisions.md` § "R-69 — What is undoable *(closes the step-6 open question)*"

## Deliverables
- `crates/gui/src/explore/manifold_view/{mod,chart,navigate,depth,lock,centre,slice_tilt}.rs`.
- `crates/gui/src/explore/compass.rs`.
- The mock's response to navigation: the stand-in redrawn for the new `z₀` and basis; its lock fields and its precision
  event fields, so the depth readout's warning can be raised; any contract field these need, added as the corpus names
  it (R-390's "Contract fields"), with the conformance suite re-run on both engines.
- Tests `mock_manifold_view`, `mock_figure_navigation`; screenshot cases `01_main/mock_manifold_view`,
  `01_main/mock_compass_slice`, `01_main/mock_compass_tilt`, `08_lock/mock_locked`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (mock_manifold_view, mock_compass_slice, mock_compass_tilt), `cargo xtask screenshot 08_lock` (mock_locked) and `cargo test -p gui mock_manifold_view` — against 01_main.png and 08_lock.png; each control emits a SetField on z₀ or the basis, and after lock a slider reads anchor plus offset and moving it emits an excursion (REQ-GUI-170).
- `cargo test -p gui mock_figure_navigation` — a pan and a zoom each emit one SetField on z₀ or the basis and no other edit; the mock's next frame of the stand-in is the previous one shifted or scaled accordingly, within one pixel (REQ-GUI-171).
- `cargo xtask screenshot 01_main` (mock_manifold_view) — screenshot against 01_main.png's left panel: one group, sub-sections in that order, eight named z₀ sliders (REQ-GUI-081).
- `cargo xtask screenshot 01_main` (mock_compass_slice, mock_compass_tilt) and `cargo xtask screenshot 08_lock` (mock_locked) — screenshots against 01_main.png after touching a slice slider and after touching a tilt, and against 08_lock.png when locked (REQ-GUI-091).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: drag and type a z₀ value, pan and zoom the figure,
  touch a slice slider then a tilt and watch the compass, K to lock and move a slider, on
  `cargo run -p gui --features mock`.
- REQ-GUI-081 moved here from TASK-M8-06 and REQ-GUI-091 from TASK-M8-07 (R-390); both depend on this task and re-run
  that acceptance on the real engine. The preset names (REQ-GUI-082), the z₀ round trip through the real snapshot
  (REQ-GUI-083), the axis labels (REQ-GUI-084), the direction labels (REQ-GUI-034), the shape-sphere controls
  (REQ-GUI-161), scrubbing (REQ-GUI-092) and the lock reticle and badge (REQ-GUI-099) stay with those tasks.
- The event-driven precision warning on the real engine is TASK-M6-21's, which depends on this task.
- Sources and silences as TASK-M6-24's Notes give them (R-390).
