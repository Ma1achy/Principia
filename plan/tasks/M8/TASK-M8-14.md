# TASK-M8-14 — Inspector window and pane 1: the editable IC on the physical canvas (05_inspectors.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-112, REQ-GUI-113, REQ-GUI-061, REQ-GUI-062, REQ-GUI-063, REQ-GUI-068
- **Depends on:** TASK-M8-09, TASK-M8-11, TASK-M4-16
- **Needs (earlier milestones):** REQ-GUI-008, REQ-TOOL-040, REQ-SYS-016, REQ-GUI-001, REQ-DEC-002, REQ-ENC-008
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The one Inspector window exists and absorbs the standalone IC Inspector (R-65), hosting trajectory_viewing §4's panels per its placement table. Pane 1 shows the IC with draggable bodies 0, 1, 2 and velocity arrows (velocity-only editing) and ghost markers at the playhead. The CoM frame is always on: net momentum is allowed, v_COM is its own vector, each velocity is drawn as the removed boost plus the CoM-frame residual, and encode consumes CoM-frame velocities. Right-click opens the body popover (mass, position, velocity, momentum, distances, share of P, L, E — all editable and synced with dragging); disc radius ∝ ∛m. Clicking a figure pixel (IC Inspector… / Open full viewer…) loads that pixel's IC.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `docs/design/principia_trajectory_viewing.md` § "6. Placement"
- `docs/notes/ic_inspector_scratchpad.md` § "Build notes"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/notes/ic_inspector_scratchpad.md` § "Physical canvas — interaction"
- `docs/notes/ic_inspector_scratchpad.md` § "Whole-system handles — the invariance audit"
- `docs/notes/ic_inspector_scratchpad.md` § "Canonical window (separate panel)"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `docs/notes/ic_inspector_scratchpad.md` § "Bridges to the main view"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/gui/src/windows/inspector/mod.rs` — the window, its tabs (Inspect, Create & locate), panes hosted per the §4 placement table.
- `crates/gui/src/windows/inspector/pane_ic.rs` — the physical canvas, drag, popover, CoM decomposition, auto-fit.
- `crates/engine/src/inspector/ic_edit.rs` — CoM-frame reduction before encode (the engine's actual encode).
- Tests: `com_frame_encode`, `body_popover_sync`, `disc_radius`, `pixel_loads_ic`; screenshot cases `05_inspectors/pane1`, `05_inspectors/fit_large`, `05_inspectors/fit_small`.

## Acceptance tests
- Review checklist (gui reviewer) — each row of trajectory_viewing §4's placement table has its panel in the named host; no separate IC Inspector window exists (REQ-GUI-112).
- `cargo xtask screenshot 05_inspectors` (pane 1; arrow drag edits velocity) — screenshot against 05_inspectors.png; dragging an arrow edits velocity (REQ-GUI-113).
- `cargo test -p engine com_frame_encode` — build an IC with P ≠ 0: v_COM is displayed, the encoded P is 0 to round-off, z equals that of the boost-removed IC (REQ-GUI-061).
- `cargo test -p gui body_popover_sync` — edit mass numerically: the popover and canvas agree; drag the body: the popover position updates (REQ-GUI-062).
- `cargo test -p gui disc_radius` — radii for masses 1 and 8 are in ratio 1 : 2 (REQ-GUI-063).
- `cargo test -p gui pixel_loads_ic` — click a pixel then open the Inspector: pane 1's IC decodes to that pixel's z (REQ-GUI-068).

## Notes
- The Jacobi frame, angles, CoM marks and auto-fit (REQ-GUI-058, REQ-GUI-059) are built in TASK-M8-15 as the body canvas shared by panes 1 and 2 and asserted there on both panes; this task's pane 1 draws the bodies, arrows and ghosts.
- RQ-98 ruled: R-113 — "hosted in the one Inspector window" left REQ-GUI-008 (M3) and is carried by REQ-GUI-112 here.
