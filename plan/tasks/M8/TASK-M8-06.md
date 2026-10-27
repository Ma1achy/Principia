# TASK-M8-06 — Manifold view panel: Chart, Navigate, depth readout, Slice centre, Tilt, and the figure's axis labels

- **Milestone:** M8
- **Closes:** REQ-GUI-082, REQ-GUI-083, REQ-GUI-084, REQ-GUI-034, REQ-GUI-161, REQ-GUI-345
- **Depends on:** TASK-M8-05, TASK-M6-26, TASK-M6-38, TASK-M2-22, TASK-M2-23, TASK-M2-17, TASK-M2-14, TASK-M2-28
- **Needs (earlier milestones):** REQ-GUI-002, REQ-GUI-003, REQ-GUI-006, REQ-CHART-005, REQ-CHART-012, REQ-CHART-028, REQ-SYS-014, REQ-CHART-002, REQ-RENDER-026
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The left panel is one "Manifold view" group in the order §G2 gives: Chart (preset named by its axes, q₁ / q₂ with edit buttons, Chart builder…, kind), Navigate (centre (u, v) and zoom as log₂, by drag and typing), the depth readout with the event-driven precision warning, Lock, Slice centre (eight named sliders, each with its number box), and Tilt (τ₁ and τ₂ with their target dropdowns, and γ) (R-457). Direction labels say whether a line is a raw-control line or a physical one-quantity direction, evaluated as tangent vectors at the chart centre with the axis's residual convention. The figure's axis labels carry the short axis name and the end values.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Directions are axis kinds — the unifying rule"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"

## Deliverables
- `crates/gui/src/explore/manifold_view/{chart,navigate,depth,centre,slice_tilt}.rs` (the files may carry R-457's names), extending the track's Manifold view group (TASK-M6-26, R-390) — each control emits SetField on the mock engine's SimConfig paths, named as the corpus names them (R-390's "Contract fields"), and has the slot for its per-field warning, filled from mock per-field metadata; TASK-M8-89 moves them to TASK-M8-01's paths and TASK-M8-03's metadata on the real engine.
- `crates/gui/src/explore/direction_label.rs` — raw-control vs physical labelling from the chart axis metadata (REQ-CHART-012).
- `crates/gui/src/explore/axis_labels.rs`, the track's axis labels (TASK-M6-26, R-390), from the mock engine's chart axis metadata — short axis name and range at each end, recomputed after pan.
- `crates/gui/src/explore/manifold_view/chart.rs` also carries the shape-sphere control: the projection selector (equirectangular / the equal-area alternative), shown only when the chart is the shape sphere (R-113), and no hemisphere toggle (R-544).
- Screenshot cases `01_main/shape_sphere_controls`, `01_main/left_panel`, `01_main/axis_labels`, `01_main/direction_labels`.

## Acceptance tests
- `cargo xtask screenshot 01_main` and `cargo test -p gui preset_display_names` — screenshot against 01_main.png; every shipped preset's display name is built from its axis names (REQ-GUI-082).
- `cargo test -p gui invariant_chart_params` — entering (L_z, E) with no lock sets the four frozen controls to z₀'s z_α, z_β, z_μ₁, z_μ₂ bit for bit, with a lock set to the lock's; editing one is one undoable SetField that moves only that parameter; control: a chart entered with its previous parameters kept fails (REQ-GUI-345; R-549, RQ-350).
- `cargo test -p gui navigate_z0_fields` — drag and typed entry on centre u, v, the zoom and each of the eight Slice centre sliders' number boxes emit the same SetField as the slider and round-trip through the snapshot; Navigate has no single-coordinate z₀ box (centre u, v pan within the plane, which edits z₀) (REQ-GUI-083).
- `cargo xtask screenshot 01_main` (axis labels, before and after a pan) — screenshot against 01_main.png; after a pan the end values update (REQ-GUI-084).
- `cargo xtask screenshot 01_main` (direction labels) — slider/direction labels distinguish 'one mass logit' from 'only m₀' (REQ-GUI-034).
- `cargo xtask screenshot 01_main` (shape-sphere controls) — with the shape-sphere chart the Chart section shows the projection selector, which changes the figure, and no hemisphere toggle; with any other chart the selector is not shown (REQ-GUI-161).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines, every contract field this task adds among its cases (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here; R-627).

## Notes
- R-113 (RQ-95 M2-G5): the shape-sphere projection selector and hemisphere toggle live in the Manifold view's Chart section, shown when the chart is the shape sphere (REQ-GUI-161); the M2 goldens (TASK-M2-28) verify the render side.
- R-544 (21 Oct 2026) removes the hemisphere toggle, and withdraws R-412's remap with it: the shape sphere chart is the whole sphere. The projection selector's placement stands. TASK-M8-44 no longer depends on this task.
- The Lock sub-group's badge and the compass are TASK-M8-07; this task leaves the Lock slot wired to the lock SetFields.
- R-390: TASK-M6-26 builds the Manifold view group on the mock engine and closes REQ-GUI-081; this task depends on it, wires the group to the real engine and re-runs REQ-GUI-081's acceptance there, keeping every other requirement it closes.
- TASK-M6-26 also builds the figure's axis labels on mock data (applied per R-369, review 5434766412 on PR #157); this task wires them to the real chart's ranges and keeps REQ-GUI-084.
- R-457 (18 Oct 2026): Navigate loses its eight z₀ values, Centre z₀ and Slice & tilt become Slice centre and Tilt, each tilt angle gets its target dropdown, and the slice step is removed. TASK-M6-38 builds this on the mock; this task depends on it and wires it to the real engine. REQ-GUI-081's and REQ-GUI-083's acceptance lines above follow the reworded requirements.
- **R-473, R-474 (20 Oct 2026):** tilt and slice on every chart on the real engine are TASK-M8-48's (REQ-GUI-241); the figure's axis labels become the line system's axes (REQ-GUI-246, TASK-M8-50), this task's being the Ends only preset meanwhile.
- R-478 W2 (20 Oct 2026): the Chart section's Links row and the decoder badge are TASK-M6-50's on the mock; keep them on the real engine.
- **R-627 (27 Oct 2026), the GUI track in M8:** this task is built on the mock engine, as R-390's track is: on spare agent slots, never holding up the physics chain, and merging once its reviews pass and CI is green without waiting for the gates of M3 to M6 (the gates of M2 and M7 by R-415's test, recorded in the PR; R-627's flag). Its dependence on TASK-M6-21 and TASK-M6-22, which it had through TASK-M8-01, is satisfied by the mock's precision events and quality tier. A control that needs a contract field adds it as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on both engines. TASK-M8-89 wires it to the real engine after TASK-M6-21 and TASK-M6-22, re-running every acceptance line here on the real engine with the conformance suite on both, and takes the dependencies this task gave up. **What to try.** The PR description has a "What to try" section: the clicks and keys that show what changed, on `cargo run -p gui --features mock`. The REQ-GUI-081 line re-run on the real engine moves to TASK-M8-89 with REQ-GUI-012 from Needs. *Was:* "- **Needs (earlier milestones):** REQ-GUI-002, REQ-GUI-003, REQ-GUI-006, REQ-GUI-012, REQ-CHART-005, REQ-CHART-012, REQ-CHART-028, REQ-SYS-014, REQ-CHART-002, REQ-RENDER-026". *Was (acceptance lines, moved verbatim):* "- `cargo xtask screenshot 01_main` (left panel) — screenshot against 01_main.png's left panel, read with R-457: one group, sub-sections in that order, eight named Slice centre sliders, no z₀ values under Navigate and no step under Tilt (REQ-GUI-081). Closed by TASK-M6-26 on the mock engine since R-390; this task re-runs it on the real engine." (to TASK-M8-89).
- R-627: TASK-M2-22, TASK-M2-23, TASK-M2-17, TASK-M2-14, TASK-M2-28 (merged) are named in Depends on so the Needs they close stay reachable now that the chain through TASK-M8-05's old dependencies is gone (applied per R-369). *Was:* "- **Depends on:** TASK-M8-05, TASK-M6-26, TASK-M6-38".
- R-627, gui review 5664362471 of R-627's port (G1, applied per R-369): the deliverables say what this track task builds on the mock; TASK-M8-01 and TASK-M8-03 are no longer dependencies, and the real-engine part (TASK-M8-01's SetField paths, TASK-M8-03's per-field metadata and warning, the axis labels on the real chart) is TASK-M8-89's. *Was:* "each control emits SetField on the SimConfig paths from TASK-M8-01; each shows the per-field warning from TASK-M8-03."; "`crates/gui/src/explore/axis_labels.rs`, the track's axis labels (TASK-M6-26, R-390) wired to the real chart —".
