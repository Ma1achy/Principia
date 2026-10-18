# TASK-M6-39 — Live drags both ways: every frame, one undo entry per drag, the compass's mode switched by the slider dragged (R-458)

- **Milestone:** M6
- **Closes:** REQ-GUI-206, REQ-GUI-207
- **Depends on:** TASK-M6-33, TASK-M6-38
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
On the mock engine, every drag is live and one undo entry (R-458). Today a compass drag is previewed in the compass alone and sent once on release, so the sliders and the figure jump at the end (`compass.rs`'s `drag_stopped`, on `main` and in PR #240), while every slider sends an edit a frame with nothing coalescing them, so a slider drag is one undo entry per frame, against R-96. And the compass's mode switches only when a slider's value changes, so it misses a held thumb, a slider at its end and a number being typed. This task makes the contract's history coalesce a drag into one entry (gui_state_contract §2), sends each drag's edit every frame (at most one a frame), logs a drag once, and switches the compass's mode on a drag's first frame, on the reworked compass (R-451, TASK-M6-33).

## References
- `decisions.md` § "R-458 — Drags are live in both directions: a drag on the compass or on any slider applies its edit every frame, at most once a frame, so the sliders, the compass, its readout and the figure follow the drag, and the whole drag is one undo entry; a drag on a Slice centre slider switches the compass to Slice and one on a Tilt slider to Tilt, its plane moving with the drag *(amends R-409 A3 and R-451 B3 as they send one `SetField` per drag)*"
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping *(amends R-409 point 1, the compass's view angle)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- The history's coalescing by drag key in the contract's store (`crates/engine/src/contract/`), on both engines' conformance suite (R-390), and the console's one line per drag.
- Every slider, number box, the compass, the scrubber and the figure's pan sending its edit every frame under one drag key (R-458 F2, F3).
- The compass's mode switched on a drag's first frame by the Slice centre and Tilt controls (R-458 F4), its drawing following `z₀` and the basis every frame (F5).
- Tests `mock_live_compass_drag` and `mock_live_slider_drag`.

## Acceptance tests
- cargo test -p gui mock_live_compass_drag (pointer scripts on the mock): over a slice-mode and a tilt-mode drag of ten frames, the snapshot's z₀ or basis, the slider value and the readout change on every frame the pointer moves and equal the drag's target at that frame; two pointer events in one frame give one edit; the undo depth rises by one over the drag and one Ctrl+Z restores the value before it; the console gains one line for the drag; controls: a drag that sends its edit only on release, and one that pushes an undo entry per frame, each fail (REQ-GUI-206).
- cargo test -p gui mock_live_slider_drag (pointer scripts on the mock): a ten-frame drag on the Slice centre slider of the slice normal changes the snapshot's z₀, the compass's drawn offset and the figure's chart on every frame, the compass in Slice mode from the first frame; the same on τ₁ tilts the drawn plane every frame, the compass in Tilt mode from the first frame; a press on τ₂'s thumb that moves it past 90° (no value change) still switches to Tilt; a click into γ's number box switches to Tilt; each drag (the Slice centre slider, τ₁, the scrubber, the figure's pan) raises the undo depth by one and adds one console line; controls: a switch only on a changed value, a slider that pushes an undo entry per frame, and a scrubber that applies only on release each fail (REQ-GUI-207).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: drag on the cube in Tilt mode and watch τ₁, τ₂, the readout and the figure move with the pointer; drag the Slice centre slider of the slice normal and watch the compass switch to Slice and its plane move; one Ctrl+Z undoes each whole drag.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **PR #240 and R-451 B4.** The reworked compass holds its drag's target in GUI scratch until release, against B4's "the contract's value is the target from the first frame"; this task makes B4 hold (R-458 F1).
- **The gestures** stay as R-451 B3 has them; RQ-368 holds their screen direction, and a ruling on it is a follow-up task, or this one if still open.
- **qa's files (R-290, R-458 F9).** A qa assertion that a compass drag sends nothing until release, or that a slider drag makes one undo entry per frame, asserts what R-458 changes; this task's qa reviewer changes those, and only those.
- **Early start (R-388).** It may start off TASK-M6-38's approved head once TASK-M6-33 has merged.
