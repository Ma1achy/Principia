# TASK-M8-19 — Stain mode layout: library drawer, canvas with Graph | Code, preview, Problems pane (02_stain.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-127, REQ-GUI-129, REQ-GUI-139, REQ-GUI-140, REQ-GUI-076, REQ-GUI-073
- **Depends on:** TASK-M8-05, TASK-M6-29
- **Needs (earlier milestones):** REQ-RENDER-006
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
Stain mode is the plain node-graph editor over one object — the RenderState stain graph plus display settings — with no separate render-mode, colour-map or debug GUI and no object-based desk. The layout is §1's four surfaces: the collapsible library drawer left (the catalogue, production groups above the debug groups, mapping glyphs, a filter box, and "Your stains" with New, Rename, Import, Export), the canvas centre with its Graph | Code toggle, the live preview (shape sphere or illustrative slice; the default defined in §1) and the node inspector right, and the assembled code and Problems pane below. Debug-tagged registry entries show in the dev GUI and are hidden by `ViewUI.debugVisible = false`. Every slice preview, here and in the Chart builder, keeps the viewport's aspect.

## References
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
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"

## Deliverables
- `crates/gui/src/stain/{mode,library,preview,problems}.rs`, extending the track's Stain layout (TASK-M6-29, R-390) on the real stain graph.
- `crates/gui/src/stain/your_stains.rs` — user stains as serialised graphs (import / export files).
- `crates/render/src/registry/filter.rs` — `debugVisible` filtering on the `category` tag.
- `crates/render/src/registry.rs:65`, the `Category` doc comment: "the polished GUI hides [`Category::Debug`]" reworded to "a replacement GUI, if there ever is one (v2 web's, R-571), hides [`Category::Debug`]", following gui_state_contract §3 (R-571; gui re-check 5608472158 of PR #282, applied per R-369).
- Tests: `debug_visible_filter`, `your_stains_roundtrip`; screenshot cases `02_stain/layout`, `02_stain/preview_{sphere,slice}`, `02_stain/library_filter`, `02_stain/aspect_{square,wide}`, `03_chartbuilder/aspect_{square,wide}`.

## Acceptance tests
- Review checklist (gui reviewer) — every stain surface edits the same RenderState graph; no parallel colour or debug editor exists (REQ-GUI-127).
- `cargo xtask screenshot 02_stain` (both preview modes; an edit updates the preview) — screenshot in both preview modes against 02_stain.png; an edit updates the preview (REQ-GUI-129).
- `cargo xtask screenshot 02_stain` (library; filter narrows rows) — screenshot against 02_stain.png's library; typing in the filter narrows rows (REQ-GUI-139).
- `cargo test -p gui your_stains_roundtrip` — create, rename, export and re-import a stain; it round-trips (REQ-GUI-140).
- Review checklist (gui reviewer) — no Chazy subtitle or stain-desk code path exists; Stain mode opens the Part II graph editor (REQ-GUI-076).
- `cargo xtask screenshot 02_stain` and `cargo xtask screenshot 03_chartbuilder` (aspect cases) — screenshots of 03_chartbuilder and 02_stain layouts: both chart-builder previews and the stain preview are square for a square viewport and match the viewport aspect otherwise (REQ-GUI-073).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines, every contract field this task adds among its cases (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here; R-627).

## Notes
- RQ-100 ruled: R-113 — REQ-GUI-152 (the preview default) moved to M7 (TASK-M7-22); this task lays out the preview §1 now names.
- R-390: TASK-M6-29 builds Stain mode's layout on the mock engine and closes REQ-GUI-128; this task depends on it, re-runs REQ-GUI-128's acceptance on the real engine and keeps every other requirement it closes.
- **R-627 (27 Oct 2026), the GUI track in M8:** this task is built on the mock engine, as R-390's track is: on spare agent slots, never holding up the physics chain, and merging once its reviews pass and CI is green without waiting for the gates of M3 to M6 (the gates of M2 and M7 by R-415's test, recorded in the PR; R-627's flag). Its dependence on TASK-M6-21 and TASK-M6-22, which it had through TASK-M8-01, is satisfied by the mock's precision events and quality tier. A control that needs a contract field adds it as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on both engines. TASK-M8-92 wires it to the real engine after TASK-M6-21 and TASK-M6-22, re-running every acceptance line here on the real engine with the conformance suite on both, and takes the dependencies this task gave up. **What to try.** The PR description has a "What to try" section: the clicks and keys that show what changed, on `cargo run -p gui --features mock`. It builds on the contract's stain graph (`engine::stain`) and the track's Stain editor (TASK-M6-29) on the mock. TASK-M8-18, TASK-M7-14 and TASK-M7-22 move to TASK-M8-92 with the Needs they carried; REQ-GEN-023, whose check reads the real registry's debug tags (TASK-M7-13), moves there with its line, as does the REQ-GUI-128 line re-run on the real engine. The Code view and the Problems pane show the mock's text until TASK-M8-92 wires them. *Was:* "- **Depends on:** TASK-M8-05, TASK-M8-18, TASK-M7-14, TASK-M7-22, TASK-M6-29"; "- **Needs (earlier milestones):** REQ-GUI-021, REQ-GUI-028, REQ-GUI-029, REQ-GUI-031, REQ-GEN-020, REQ-GEN-021, REQ-RENDER-006, REQ-RENDER-057, REQ-GUI-152"; "- **Closes:** REQ-GUI-127, REQ-GUI-129, REQ-GUI-139, REQ-GUI-140, REQ-GUI-076, REQ-GUI-073, REQ-GEN-023". *Was (acceptance lines, moved verbatim):* "- `cargo xtask screenshot 02_stain` — screenshot against 02_stain.png (REQ-GUI-128). Closed by TASK-M6-29 on the mock engine since R-390; this task re-runs it on the real engine." (to TASK-M8-92); "- `cargo test -p render debug_visible_filter` — debugVisible = false removes exactly the debug-tagged entries from the list (REQ-GEN-023)." (to TASK-M8-92).
