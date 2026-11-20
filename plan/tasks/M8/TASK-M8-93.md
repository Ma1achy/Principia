# TASK-M8-93 — The Stain canvas on the real engine (R-627; wires TASK-M8-20)

- **Milestone:** M8
- **Closes:** REQ-GUI-407
- **Depends on:** TASK-M8-20, TASK-M8-92, TASK-M7-11, TASK-M7-12, TASK-M7-13, TASK-M6-21, TASK-M6-22
- **Needs (earlier milestones):** REQ-GUI-021, REQ-GUI-022, REQ-GUI-023, REQ-GUI-024, REQ-GUI-025, REQ-GUI-026, REQ-GUI-027, REQ-GEN-019, REQ-GUI-151
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~120 lines

## Goal
TASK-M8-20 built the Stain canvas on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-20 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-20 added. It takes the dependencies TASK-M8-20 gave up under R-627.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/principia_render_gui_spec.md` § "6. Port typing & wire rules"
- `docs/gui/principia_render_gui_spec.md` § "7. Canvas interactions"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `docs/gui/principia_render_gui_spec.md` § "8. Node visuals — glyph-forward"
- `docs/gui/principia_render_gui_spec.md` § "16. Open / next"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- The Stain canvas wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-20 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.

## Acceptance tests
- `cargo test -p gui canvas_gestures` — scripted pointer events for each gesture produce the stated selection / position change (REQ-GUI-131). Closed by TASK-M6-29 on the mock engine since R-390; this task re-runs it on the real engine.
- TASK-M8-20's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-20 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-407).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-20's, moved verbatim; TASK-M8-20's Notes are their record.
