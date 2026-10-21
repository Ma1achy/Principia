# TASK-M8-55 — Past the square with the controls shown: pan and zoom past [0,1]², drawn with no gaps, every interaction working there, a repeat resolving to its canonical system, the ✕ and the round trip (R-537 EX1 to EX5, EX9)

- **Milestone:** M8
- **Closes:** REQ-GUI-267, REQ-GUI-268, REQ-GUI-269, REQ-GUI-270
- **Depends on:** TASK-M8-56, TASK-M8-05, TASK-M8-08, TASK-M8-09, TASK-M8-32, TASK-M8-44, TASK-M8-48
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
On the real engine the chart's extension is reached by pan and zoom with the controls shown, not only under F3 (R-537): the view is a window onto the chart's extended parameter plane, the only stop R-469's on the folded centre; wherever it lands it renders the axes' types with no gaps; every interaction works at every visible pixel, a pixel in a repeat resolving to its canonical system, a click on a pixel that is not a system drawing the red ✕ and integrating nothing; and an export reproduces the window, the centre unwrapped on a periodic axis.

## References
- `decisions.md` § "R-537 — The chart's extension past `[0,1]²` is reached by pan and zoom with the controls shown, not only under F3: wherever the view lands it renders with no gaps, every interaction works there, a pixel in a repeat resolving to its canonical system, and the quadtree, refinement and deep zoom work in extended and wrapped regions, one system computed once ⚑ *(amends R-406, R-407 and R-472)*"
- `decisions.md` § "R-469 — A chart never leaves the manifold: every pixel it draws is a valid three-body system or explicitly "not a system", and pan, slice, zoom, tilt and lock are accepted only while the chart's centre passes `validate`, an edit that would cross stopping at the last valid value, by one generic rule in the edit path; undo and the contract record the clamped value, the slider stops and the readout names the check"
- `decisions.md` § "R-470 — A pixel that is not a system draws blank, the view's background, in every view, the outcome view included, never a palette colour; the hatch survives only as the optional Overlays › Chart › forbidden region, off by default, over blank pixels only; NaN, ±∞, `sim_failed`, `decode_failed` and the classification behind the hatch show only in the debug views; R-408's statistics are unchanged *(amends R-407, R-446 and R-467; closes RQ-374)*"
- `decisions.md` § "R-453 — A click on the figure pins that sample's trajectory in the Trajectory panel, and hover never replaces it; the cursor moving into the panel never clears it; the panel's real-space view pans and zooms and its shape-sphere view orbits and zooms"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/contracts/principia_chart_decoder_contract.md` § "The operation table"
- `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- Navigation past the square with the layer shown, the readout's repeat label and canonical coordinates, the ✕ and its readout, and the view state's unwrapped centre, in `crates/gui/` on the real engine, reading TASK-M8-56's canonical addresses.
- Tests `extension_navigation`, `extension_no_gaps`, `extension_pick` and `extension_round_trip`, and the `extension_pick_x` screenshot target.

## Acceptance tests
- `cargo test -p gui extension_navigation` — REQ-GUI-267's verify (R-537 T1, T12).
- `cargo test -p gui extension_no_gaps` — REQ-GUI-268's verify (R-537 T3, T4).
- `cargo test -p gui extension_pick` and `cargo xtask screenshot 01_main` (real engine, `extension_pick_x`) — REQ-GUI-269's verify (R-537 T5, T6; the hover arm, the ✕'s screenshot).
- `cargo test -p gui extension_round_trip` — REQ-GUI-270's verify (R-537 EX9).

## Notes
- **Applied meanwhile (R-537 U4, U6, U7, U9, U14):** RQ-416 (a), Fit the view's own square (R-468 E2); RQ-419 (a), the frames before the first baseline cover excepted; RQ-417 (b), the readout's label only in a debug view; RQ-406 (a), RQ-407 (a), RQ-409 (a); RQ-408's outline is not built; comments and the console's pick wait on RQ-420. A ruling other than these is a follow-up change.
- **The mock** needs nothing (R-537 U2): its stand-in is affine everywhere and already pans past the preset's window with the controls shown.
- **The trace's repeats** (EX6) are TASK-M8-58's, with R-538's markers.
- *Was:* the acceptance line "`cargo test -p gui extension_pick` — REQ-GUI-269's verify (R-537 T5, T6)." (gui review 5594407176 of PR #263, G1, G3; applied per R-369).
