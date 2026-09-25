# TASK-M8-07 — Lock, compass and time controls: the reticle, the pin, the nav cube and the scrubber (08_lock.png, 01_main.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-091, REQ-GUI-092, REQ-GUI-099, REQ-GUI-155
- **Depends on:** TASK-M8-06
- **Needs (earlier milestones):** REQ-GUI-004, REQ-GUI-005, REQ-GUI-006, REQ-GUI-007, REQ-TOOL-046, REQ-TOOL-047, REQ-SCHED-036, REQ-SCHED-016
- **Reviewers:** code, qa, gui
- **Pitfalls:** PIT-1.7
- **Size:** ~450 lines

## Goal
Locking (K or right-click → lock here) recentres on the point with a SetField on z₀, draws the gold reticle and the compass's gold pin, and shows the "● locked at z_locked" badge with unlock and open in Inspector. The compass sits bottom left, shows the slice plane inside the chart, switches between slicing and tilting by itself, tilts when the plane is dragged and orbits when the cube is dragged, and reads out the angles. The time controls offer play, step, a scrubber and speed; scrubbing back sets the playhead and re-integrates progressively with a visible "refining · N%", never replaying stored frames (R-66).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "08 Lock — the reticle and the pin"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 1 — Playback is the temporal mechanism"
- `decisions.md` § "R-66 — The time scrubber stays *(closes RQ-22)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/gui/src/explore/lock.rs` — badge, unlock, open-in-Inspector (opens the window id of TASK-M8-14), reticle mark on the figure.
- `crates/gui/src/explore/compass.rs` — the nav cube; mode follows the last-touched slider group.
- `crates/gui/src/explore/time.rs` — transport in ViewUI; per-frame playhead write marked no-history; scrub = one coalesced entry; progress from the snapshot.
- Screenshot cases `08_lock/locked`, `01_main/compass_slice`, `01_main/compass_tilt`.
- Tests: `scrub_reintegrates`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (slice and tilt cases) and `cargo xtask screenshot 08_lock` — screenshots against 01_main.png after touching a slice slider and after touching a tilt, and against 08_lock.png when locked (REQ-GUI-091).
- `cargo test -p gui scrub_reintegrates` — scrub from t=40 to t=10: the playhead SetField triggers re-integration from 0 with a progress indicator; no frame store is read (REQ-GUI-092).
- `cargo xtask screenshot 08_lock` — screenshot against 08_lock.png after K over a point (REQ-GUI-099).
- Definition: the scrubber's progress percentage written into render_gui_spec §G2 and approved by the physics reviewer (REQ-GUI-155).

## Notes
- The scrubber's "refining · N%" needs a progress fraction in the snapshot; what it is a fraction of is not stated.
- Closes, for gaps the corpus leaves open: REQ-GUI-155 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
