# TASK-M8-95 — The Run window on the real engine, and the stepper pairs' warnings (R-627; wires TASK-M8-24)

- **Milestone:** M8
- **Closes:** REQ-GUI-409, REQ-GUI-351, REQ-GUI-352
- **Depends on:** TASK-M8-24, TASK-M8-88, TASK-M3-39, TASK-M6-21, TASK-M6-22
- **Needs (earlier milestones):** REQ-INT-026, REQ-EVT-015, REQ-GUI-011, REQ-GUI-014, REQ-GUI-015, REQ-PERF-065, REQ-INT-098
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
TASK-M8-24 built the Run window on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-24 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-24 added. It takes the dependencies TASK-M8-24 gave up under R-627, and closes the requirements whose check is on the real engine: REQ-GUI-351, REQ-GUI-352.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-575 — The horizon stays 50 by default and can be raised or lowered, not floored at 50 *(amends R-132 and the Run window's `T_horizon ∈ [50, 200]`; queues RQ-555)*"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"
- `decisions.md` § "R-107 — Apply the RQ-67 follow-ups; GUI_DESIGN_NOTES may be conformed *(closes RQ-67)*"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/design/principia_quality_device_note.md` § "The auto → custom "show your work" flow"
- `decisions.md` § "R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*"
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `decisions.md` § "R-560 — The default occupant is Heggie × RK4, the combination every Heggie result was measured with; REQ-GUI-102's default and the Run window change with it"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"

## Deliverables
- The Run window wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-24 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.
- The stepper pair's warning beside the selectors and in the Problems panel RQ-525 names, from TASK-M3-39's measured pair table (REQ-GUI-351, REQ-GUI-352).

## Acceptance tests
- `cargo test -p gui occupant_pair_warning` — selecting Heggie × KDK shows the integrator contract's Part 2b warning text beside the selectors and the run proceeds; selecting Heggie × RK4 shows none; every pair of the production steppers is reachable, and the Euler pairs only in the debug selection (REQ-INT-015); controls: a selector that disables a measured-bad pair fails, a warning shown for a not-measured pair fails, and a production selector offering an Euler pair fails (REQ-GUI-351; R-561; qa review 5607548278 of PR #277, Q6, applied per R-369).
- `cargo test -p gui problems_pair_warning` — with Heggie × KDK bound the Problems panel RQ-525 names lists the pair's warning, and with Heggie × RK4 it lists none; control: a binding change that leaves the old warning listed fails (REQ-GUI-352; R-561; waits on RQ-525, the rest of the task does not).
- TASK-M8-24's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-24 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-409).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-24's, moved verbatim; TASK-M8-24's Notes are their record.
- REQ-GUI-352's line waits on RQ-525 (the Problems panel it names), as it did in TASK-M8-24; the rest of this task does not.
