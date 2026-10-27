# TASK-M8-89 — The Manifold view panel on the real engine (R-627; wires TASK-M8-06)

- **Milestone:** M8
- **Closes:** REQ-GUI-403
- **Depends on:** TASK-M8-06, TASK-M8-88, TASK-M6-21, TASK-M6-22
- **Needs (earlier milestones):** REQ-GUI-012
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
TASK-M8-06 built the Manifold view panel on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-06 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-06 added. It takes the dependencies TASK-M8-06 gave up under R-627.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Directions are axis kinds — the unifying rule"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"

## Deliverables
- The Manifold view panel wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-06 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.

## Acceptance tests
- `cargo xtask screenshot 01_main` (left panel) — screenshot against 01_main.png's left panel, read with R-457: one group, sub-sections in that order, eight named Slice centre sliders, no z₀ values under Navigate and no step under Tilt (REQ-GUI-081). Closed by TASK-M6-26 on the mock engine since R-390; this task re-runs it on the real engine.
- TASK-M8-06's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-06 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-403).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-06's, moved verbatim; TASK-M8-06's Notes are their record.
