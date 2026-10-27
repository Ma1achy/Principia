# TASK-M8-90 — Lock, compass and time controls on the real engine, and the scrub that re-integrates (R-627; wires TASK-M8-07)

- **Milestone:** M8
- **Closes:** REQ-GUI-404, REQ-GUI-092
- **Depends on:** TASK-M8-07, TASK-M8-89, TASK-M6-21, TASK-M6-22
- **Needs (earlier milestones):** REQ-TOOL-046, REQ-TOOL-047, REQ-SCHED-036, REQ-SCHED-016
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
TASK-M8-07 built the lock, compass and time controls on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-07 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-07 added. It takes the dependencies TASK-M8-07 gave up under R-627, and closes the requirements whose check is on the real engine: REQ-GUI-092.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "08 Lock — the reticle and the pin"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 1 — Playback is the temporal mechanism"
- `decisions.md` § "R-66 — The time scrubber stays *(closes RQ-22)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"

## Deliverables
- The lock, compass and time controls wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-07 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.
- The scrub on the real engine: the playhead `SetField` re-integrating from 0 with the progress indicator TASK-M8-07 shows on the mock (REQ-GUI-092).

## Acceptance tests
- `cargo test -p gui scrub_reintegrates` — scrub from t=40 to t=10: the playhead SetField triggers re-integration from 0 with a progress indicator; no frame store is read (REQ-GUI-092).
- TASK-M8-07's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-07 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-404).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-07's, moved verbatim; TASK-M8-07's Notes are their record.
