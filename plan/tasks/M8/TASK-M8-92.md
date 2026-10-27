# TASK-M8-92 — Stain mode's layout on the real engine: the registry, the Code view and the Problems pane (R-627; wires TASK-M8-19)

- **Milestone:** M8
- **Closes:** REQ-GUI-406, REQ-GEN-023
- **Depends on:** TASK-M8-19, TASK-M8-88, TASK-M8-18, TASK-M7-14, TASK-M7-22, TASK-M6-21, TASK-M6-22
- **Needs (earlier milestones):** REQ-GUI-021, REQ-GUI-028, REQ-GUI-029, REQ-GUI-031, REQ-GEN-020, REQ-GEN-021, REQ-RENDER-057, REQ-GUI-152
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
TASK-M8-19 built Stain mode's layout on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-19 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-19 added. It takes the dependencies TASK-M8-19 gave up under R-627, and closes the requirements whose check is on the real engine: REQ-GEN-023.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "02 Stain — the plain node-graph editor"
- `decisions.md` § "R-571 — The browser: the native app first, then v1 web, the whole native app compiled to run in the browser on the GitHub Pages site, then v2 web, a separate product designed later *(replaces the browser item of the gap sweep's `INSTRUCTION_now.md` backlog; queues RQ-550 and RQ-551)*"
- `docs/gui/principia_render_gui_spec.md` § "0. The core claim"
- `docs/gui/principia_render_gui_spec.md` § "Part II — The stain editor"
- `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces"
- `docs/gui/principia_render_gui_spec.md` § "16. Open / next"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/contracts/principia_gui_state_contract.md` § "7. What a replacement GUI must honour (the teardown contract)"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- Stain mode's layout wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-19 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.
- The library drawer on the real registry (TASK-M7-13), its debug tags filtered (REQ-GEN-023); the Code view and the Problems pane on the real WGSL (TASK-M7-14, TASK-M7-24's when merged); the canvas on TASK-M7-22's; the Chart builder's previews on TASK-M8-18's for REQ-GUI-073's aspect cases.

## Acceptance tests
- `cargo xtask screenshot 02_stain` — screenshot against 02_stain.png (REQ-GUI-128). Closed by TASK-M6-29 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo test -p render debug_visible_filter` — debugVisible = false removes exactly the debug-tagged entries from the list (REQ-GEN-023).
- TASK-M8-19's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-19 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-406).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-19's, moved verbatim; TASK-M8-19's Notes are their record.
