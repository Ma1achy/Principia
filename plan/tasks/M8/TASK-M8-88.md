# TASK-M8-88 — The Explore shell on the real engine: egui-wgpu on the engine's device, the status line, the footer and `budget-bound` (R-627; wires TASK-M8-05)

- **Milestone:** M8
- **Closes:** REQ-GUI-402, REQ-GUI-070, REQ-GUI-078
- **Depends on:** TASK-M8-05, TASK-M6-21, TASK-M6-22, TASK-M8-03, TASK-M8-04, TASK-M6-20
- **Needs (earlier milestones):** REQ-TOOL-050, REQ-TOOL-057, REQ-GUI-010
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
TASK-M8-05 built the Explore shell on the mock engine (R-627; R-390's track). This task wires that work to the real engine, after TASK-M6-21 (the precision events) and TASK-M6-22 (the quality tier), whose stand-ins on the mock it replaces: the screen reads the real engine's snapshot and sends its edits to the real state store, every acceptance line of TASK-M8-05 passes on the real engine, and the contract conformance suite passes on both engines, covering every contract field TASK-M8-05 added. It takes the dependencies TASK-M8-05 gave up under R-627, and closes the requirements whose check is on the real engine: REQ-GUI-070, REQ-GUI-078.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `decisions.md` § "R-446 — "Degenerate" leaves the outcome view: black `#141418` means nothing has happened yet, up to the playhead, and bounded, running and every failure draw it; failures show in the debug views; forbidden chart regions keep the chart contract's hatch, decided from the chart's validity *(supersedes R-96, R-399 and R-441 as they apply to the outcome view)*"
- `decisions.md` § "R-467 — Forbidden pixels and non-finite values are debug-only: a presentation view draws a pixel that is not part of the chart as the canvas background, with no hatch, key or share, and every failure and non-finite value `#141418`; one switchable debug view, View › Debug view, shows the hatch, `debug_invalid` and a NaN / Inf style, each with its key and count *(amends R-446, and R-407 clause 2 as it applies to display; closes RQ-319)*"
- `decisions.md` § "R-468 — With the egui layer hidden, F3 fills the window by one of two modes: Extend, the default and R-406's, more of the field at the same scale, or Fit, the view's square scaled to the window's shorter side, never stretched, the rest canvas background; chosen in View › "F3 fills by: Extend / Fit" for the session, a view change and never an edit *(amends R-406)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `decisions.md` § "R-508 — An energy chart's `t > 1` side continues: past `t = 1` the warp's `K > K_max` is drawn and counted as an affine side, while past `t < 0`, and the `s` axis past either edge, the warp stays bounded *(closes RQ-376; amends R-471)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"

## Deliverables
- The Explore shell wired to the real engine: it reads the real engine's snapshot and sends its `SetField`s and requests to `StateStore` (`crates/engine/src/contract/store.rs`), in place of the mock, when the app runs without `--features mock`.
- Each contract field TASK-M8-05 added (R-390's "Contract fields"), implemented by the real engine as far as the conformance suite reaches (R-390's "The real engine's side"), with a conformance case in `crates/engine/src/contract/conformance.rs` for any field no case covers, run on both engines.
- `crates/gui/src/render_host.rs` — egui-wgpu `Renderer` constructed from the engine's `wgpu::Device` / `Queue`, painting after the engine's pass on the same surface texture.
- The status line on the real engine's Snapshot: the frame record's summary, filled by the frame loop (the RQ-243 note of TASK-M8-05; `FrameSummary`, `None` until now), the binding axis for `budget-bound` (TASK-M6-20) and the undo depth; the footer's counts, latest message and memory from the real engine's log and frame record.
- `xtask` screenshot case `01_main/budget_bound`, on the real engine.
- `crates/engine/src/contract/canvas.rs`'s module comment reworded to name this task, not TASK-M8-05, as the owner of the real engine's canvas implementation (REQ-GUI-070) (qa review 5665189777 of R-627's port, nit, applied per R-369).

## Acceptance tests
- `cargo xtask screenshot 01_main` (F3 on / off cases) and Review checklist (gui reviewer) on the egui-wgpu construction — screenshots with F3 on and off against 01_main.png; review that egui-wgpu is constructed from the engine's device/queue, not a second context (REQ-GUI-070). The F3-off clause, the figure identical over the shown figure's rect, is run by TASK-M8-87, which places the real canvas.
- `cargo xtask screenshot 01_main` — screenshot against 01_main.png: dark theme, Ubuntu for text, Ubuntu Mono for numbers/code (REQ-GUI-075). Closed by TASK-M6-24 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo xtask screenshot 01_main` (top bar, budget-bound case) and `cargo test -p gui status_undo_depth` — screenshot against 01_main.png's top bar; force the budget to bind and check 'budget-bound' appears; make two edits and check the undo depth reads 2 (REQ-GUI-078).
- TASK-M8-05's acceptance lines re-run on the real engine (`cargo run -p gui`, and `cargo xtask screenshot` through the headless capture on the real engine), each with its output in the PR, and `cargo test -p engine conformance` and `cargo test -p gui conformance` — the conformance suite passes on both engines, every contract field TASK-M8-05 added among its cases; control: a contract field the screen reads that no case covers fails the review (REQ-GUI-402).

## Notes
- R-627 (27 Oct 2026): a wiring task, small by design. It is not a track task: it runs on the real engine and waits for the gates as any engine-side task does (R-415's test where it would merge early). The acceptance lines above that name an earlier task's requirement were TASK-M8-05's, moved verbatim; TASK-M8-05's Notes are their record.
- TASK-M8-87, the figure past `[0,1]²` with the axis extension types and the area statistics, depends on this task.
- *Was (qa review 5665189777 of R-627's port, F5, applied per R-369; the F3-off clause moved verbatim to TASK-M8-87 with a control):* "- `cargo xtask screenshot 01_main` (F3 on / off cases) and Review checklist (gui reviewer) on the egui-wgpu construction — screenshots with F3 on and off against 01_main.png: the figure is identical underneath, over the shown figure's rect, with a stain that reads no screen-lane field (R-406, R-407 A4); review that egui-wgpu is constructed from the engine's device/queue, not a second context (REQ-GUI-070).".
- physics review 5665205331 of R-627's port, P1 (applied per R-369): physics joins the reviewers, since the real frame loop fills the frame summary and may revise REQ-GUI-176's definition (`quad_count` as the leaf count, fps, GPU and heap memory), which needs physics review before merge; its owner before R-627, TASK-M8-05, had physics. *Was:* "- **Reviewers:** code, qa, gui".
