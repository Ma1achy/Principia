# TASK-M8-07 — Lock, compass and time controls: the reticle, the pin, the nav cube and the scrubber (08_lock.png, 01_main.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-099
- **Depends on:** TASK-M8-06, TASK-M6-26, TASK-M6-27, TASK-M2-23
- **Needs (earlier milestones):** REQ-GUI-004, REQ-GUI-005, REQ-GUI-006, REQ-GUI-007
- **Reviewers:** code, qa, gui
- **Pitfalls:** PIT-1.7
- **Size:** ~450 lines

## Goal
Locking (K or right-click → lock here) recentres on the point with a SetField on z₀, draws the gold reticle and the compass's gold pin, and shows the "● locked at z_locked" badge with unlock and open in Inspector. The compass sits bottom left, shows the slice plane inside the chart in a fixed view with no orbit, switches between slicing and tilting by its Tilt and Slice buttons and by itself, the slider touched (R-409), tilts when dragged in tilt mode and moves the slice along the plane's normal when dragged in slice mode, and reads out the angles. The time controls offer play, step, a scrubber and speed; scrubbing back sets the playhead and re-integrates progressively with a visible "refining · N%", never replaying stored frames (R-66).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "08 Lock — the reticle and the pin"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 1 — Playback is the temporal mechanism"
- `decisions.md` § "R-66 — The time scrubber stays *(closes RQ-22)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"

## Deliverables
- `crates/gui/src/explore/lock.rs`, extending the track's badge and reticle (TASK-M6-26, R-390) on the mock engine's lock — badge, unlock, open-in-Inspector (opens the window id of TASK-M8-14), reticle mark on the figure.
- `crates/gui/src/explore/compass.rs`, extending the track's compass (TASK-M6-26, R-390) — the nav cube; mode set by the Tilt and Slice buttons and following the last-touched slider group; no orbit (R-409).
- `crates/gui/src/explore/time.rs`, extending the track's Time (TASK-M6-27, R-390) — transport in ViewUI; per-frame playhead write marked no-history; scrub = one coalesced entry; progress from the snapshot.
- Screenshot cases `08_lock/locked`, `01_main/compass_slice`, `01_main/compass_tilt`.

## Acceptance tests
- `cargo xtask screenshot 08_lock` — screenshot against 08_lock.png after K over a point (REQ-GUI-099).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines, every contract field this task adds among its cases (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here; R-627).

## Notes
- TASK-M6-26 also builds the lock badge and the gold reticle on mock data (applied per R-369, review 5434766412 on PR #157); this task wires them to the real engine's lock and keeps REQ-GUI-099.
- R-627 (27 Oct 2026), a correction beside the note above, which stands as its record: the badge and reticle on the real engine's lock are TASK-M8-90's; this task builds them on the mock engine's lock and keeps REQ-GUI-099 (code review 5664955153 of R-627's port, applied per R-369).
- R-390: TASK-M6-26 builds the compass and lock's re-basing on the mock engine and closes REQ-GUI-091, and TASK-M6-27 builds Time; this task depends on both, wires them to the real engine, re-runs REQ-GUI-091's acceptance there, and keeps REQ-GUI-092, REQ-GUI-099 and REQ-GUI-155. (R-627, a correction beside this R-390 note, which stands as its record: REQ-GUI-092 and REQ-GUI-155 are now TASK-M8-90's, as are the wiring to the real engine and REQ-GUI-091's re-run there; this task keeps REQ-GUI-099, on the mock; applied per R-369, physics approval 5664252864 and code review 5664955153 of R-627's port.)
- R-409 (10 Oct 2026): the compass has no orbit, has Tilt and Slice buttons beside its switch by the slider touched, and
  in slice mode a drag moves the slice along the plane's normal. TASK-M6-31 builds it on the mock (REQ-GUI-181,
  REQ-GUI-183), and this task reaches it through TASK-M6-27; it wires the slice-mode drag's `SetField` on `z₀` to the
  real engine and re-runs REQ-GUI-091 as R-409 words it.
- R-627 (27 Oct 2026), a correction beside the note above, which stands as its record: wiring the slice-mode drag's `SetField` on `z₀` to the real engine and re-running REQ-GUI-091 as R-409 words it are TASK-M8-90's; this task builds the drag on the mock (code review 5664955153 of R-627's port, applied per R-369).
- **R-627 (27 Oct 2026), the GUI track in M8:** this task is built on the mock engine, as R-390's track is: on spare agent slots, never holding up the physics chain, and merging once its reviews pass and CI is green without waiting for the gates of M3 to M6 (the gates of M2 and M7 by R-415's test, recorded in the PR; R-627's flag). Its dependence on TASK-M6-21 and TASK-M6-22, which it had through TASK-M8-01, is satisfied by the mock's precision events and quality tier. A control that needs a contract field adds it as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on both engines. TASK-M8-90 wires it to the real engine after TASK-M6-21 and TASK-M6-22, re-running every acceptance line here on the real engine with the conformance suite on both, and takes the dependencies this task gave up. **What to try.** The PR description has a "What to try" section: the clicks and keys that show what changed, on `cargo run -p gui --features mock`. REQ-GUI-092 (a scrub re-integrating from 0) moves to TASK-M8-90 with its acceptance line, since it is the real engine's re-integration; the Needs closed by M5 tasks move with it. The mock's scrubber shows a progress percentage; REQ-GUI-155, its definition, moves to TASK-M8-90 with its acceptance line and the two notes on it, since the percentage is the real engine's re-integration (physics review 5664187570 of R-627's port, F1, applied per R-369; *Was:* "REQ-GUI-155's definition stays here, and the mock's scrubber shows its progress percentage."). *Was:* "- **Needs (earlier milestones):** REQ-GUI-004, REQ-GUI-005, REQ-GUI-006, REQ-GUI-007, REQ-TOOL-046, REQ-TOOL-047, REQ-SCHED-036, REQ-SCHED-016"; "- **Closes:** REQ-GUI-092, REQ-GUI-099, REQ-GUI-155". *Was (acceptance lines, moved verbatim):* "- `cargo test -p gui scrub_reintegrates` — scrub from t=40 to t=10: the playhead SetField triggers re-integration from 0 with a progress indicator; no frame store is read (REQ-GUI-092)." (to TASK-M8-90).
- R-627: TASK-M2-23 (merged) is named in Depends on so the Needs it closes stay reachable now that the chain through TASK-M8-05's old dependencies is gone (applied per R-369). *Was:* "- **Depends on:** TASK-M8-06, TASK-M6-26, TASK-M6-27".
- R-627, physics review 5664187570 (F1, applied per R-369): REQ-GUI-155 moved to TASK-M8-90, whose reviewers include physics. *Was:* "- **Closes:** REQ-GUI-099, REQ-GUI-155"; *Was (acceptance line, moved verbatim to TASK-M8-90):* "- Definition: the scrubber's progress percentage written into render_gui_spec §G2 and approved by the physics reviewer (REQ-GUI-155)."; *Was (Notes, moved verbatim to TASK-M8-90):* "- The scrubber's "refining · N%" needs a progress fraction in the snapshot; what it is a fraction of is not stated." and "- Closes, for gaps the corpus leaves open: REQ-GUI-155 (R-72 definition) (classification accepted by R-132)."
- R-627, gui review 5664362471 of R-627's port (G1, applied per R-369): the lock is built on the mock engine's lock; `scrub_reintegrates` is TASK-M8-90's (REQ-GUI-092); and REQ-GUI-091's re-run on the real engine moves to TASK-M8-90 with its acceptance line, as TASK-M8-06, -13, -19 and -20 gave theirs to their wiring tasks. *Was:* "`crates/gui/src/explore/lock.rs`, the track's badge and reticle (TASK-M6-26, R-390) wired to the real engine —"; "- Tests: `scrub_reintegrates`."; *Was (acceptance line, moved verbatim to TASK-M8-90):* "- `cargo xtask screenshot 01_main` (slice and tilt cases) and `cargo xtask screenshot 08_lock` — screenshots against 01_main.png after touching a slice slider, after touching a tilt and after clicking each of the Tilt and Slice buttons, and against 08_lock.png when locked, the buttons and the fixed view read with R-409 (REQ-GUI-091). Closed by TASK-M6-26 on the mock engine since R-390; this task re-runs it on the real engine.".
