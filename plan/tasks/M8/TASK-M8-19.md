# TASK-M8-19 — Stain mode layout: library drawer, canvas with Graph | Code, preview, Problems pane (02_stain.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-127, REQ-GUI-128, REQ-GUI-129, REQ-GUI-152, REQ-GUI-139, REQ-GUI-140, REQ-GUI-076, REQ-GUI-073, REQ-GEN-023
- **Depends on:** TASK-M8-05, TASK-M8-18, TASK-M7-14, TASK-M7-22
- **Needs (earlier milestones):** REQ-GUI-021, REQ-GUI-028, REQ-GUI-029, REQ-GUI-031, REQ-GEN-020, REQ-GEN-021, REQ-RENDER-006, REQ-RENDER-057
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
Stain mode is the plain node-graph editor over one object — the RenderState stain graph plus display settings — with no separate render-mode, colour-map or debug GUI and no object-based desk. The layout is §1's four surfaces: the collapsible library drawer left (the catalogue, production groups above the debug groups, mapping glyphs, a filter box, and "Your stains" with New, Rename, Import, Export), the canvas centre with its Graph | Code toggle, the live preview (shape sphere or illustrative slice; the default defined in §1) and the node inspector right, and the assembled code and Problems pane below. Debug-tagged registry entries show in the dev GUI and are hidden by `ViewUI.debugVisible = false`. Every slice preview, here and in the Chart builder, keeps the viewport's aspect.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "02 Stain — the plain node-graph editor"
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

## Deliverables
- `crates/gui/src/stain/{mode,library,preview,problems}.rs`.
- `crates/gui/src/stain/your_stains.rs` — user stains as serialised graphs (import / export files).
- `crates/render/src/registry/filter.rs` — `debugVisible` filtering on the `category` tag.
- Doc change: `docs/gui/principia_render_gui_spec.md` Part II §1 (the default preview) and §16 (the open item closed).
- Tests: `debug_visible_filter`, `your_stains_roundtrip`; screenshot cases `02_stain/layout`, `02_stain/preview_{sphere,slice}`, `02_stain/library_filter`, `02_stain/aspect_{square,wide}`, `03_chartbuilder/aspect_{square,wide}`.

## Acceptance tests
- Review checklist (gui reviewer) — every stain surface edits the same RenderState graph; no parallel colour or debug editor exists (REQ-GUI-127).
- `cargo xtask screenshot 02_stain` — screenshot against 02_stain.png (REQ-GUI-128).
- `cargo xtask screenshot 02_stain` (both preview modes; an edit updates the preview) — screenshot in both preview modes against 02_stain.png; an edit updates the preview (REQ-GUI-129).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "1. Layout — four surfaces" — §1 names the default and §16's open item is closed; the physics reviewer approves the doc change before merge (REQ-GUI-152).
- `cargo xtask screenshot 02_stain` (library; filter narrows rows) — screenshot against 02_stain.png's library; typing in the filter narrows rows (REQ-GUI-139).
- `cargo test -p gui your_stains_roundtrip` — create, rename, export and re-import a stain; it round-trips (REQ-GUI-140).
- Review checklist (gui reviewer) — no Chazy subtitle or stain-desk code path exists; Stain mode opens the Part II graph editor (REQ-GUI-076).
- `cargo xtask screenshot 02_stain` and `cargo xtask screenshot 03_chartbuilder` (aspect cases) — screenshots of 03_chartbuilder and 02_stain layouts: both chart-builder previews and the stain preview are square for a square viewport and match the viewport aspect otherwise (REQ-GUI-073).
- `cargo test -p render debug_visible_filter` — debugVisible = false removes exactly the debug-tagged entries from the list (REQ-GEN-023).

## Notes
- REQ-GUI-152 is a definition (R-72) that its requirement routes to the physics reviewer.
- Definitions (R-72) written here: REQ-GUI-152. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
- Waits on RQ-100 (`REVIEW_QUEUE.md`): Existing requirements closed after the task that needs them.
