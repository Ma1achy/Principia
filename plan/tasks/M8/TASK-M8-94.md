# TASK-M8-94 — The Display window on the real engine: the last stages and the display overlays (R-627; wires TASK-M8-23)

- **Milestone:** M8
- **Closes:** REQ-GUI-408
- **Depends on:** TASK-M8-23, TASK-M8-88, TASK-M8-92, TASK-M8-08, TASK-M7-21, TASK-M6-21, TASK-M6-22
- **Needs (earlier milestones):** REQ-RENDER-069
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
TASK-M8-23 built the Display window on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-23 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-23 added. It takes the dependencies TASK-M8-23 gave up under R-627.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces"
- `docs/gui/principia_render_gui_spec.md` § "15. Settled decisions (record)"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"

## Deliverables
- The Display window wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-23 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.
- The Display window's controls on the real display chain (TASK-M7-21) and its overlays on TASK-M8-08's tiers.

## Acceptance tests
- TASK-M8-23's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-23 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-408).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-23's, moved verbatim; TASK-M8-23's Notes are their record.
