# TASK-M1-07 — The coordinate convention: the one flip, the UV preset, the coordinate view and picking

- **Milestone:** M1
- **Closes:** REQ-SYS-009, REQ-TOOL-019, REQ-TOOL-027, REQ-GUI-001
- **Depends on:** TASK-M1-06
- **Needs (earlier milestones):** REQ-TOOL-004
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~320 lines

## Goal
The earliest view of all: one internal orientation (bottom-left origin, Y-up) and exactly one named flip at the framebuffer↔UV boundary (`v = 1 − frag_coord.y/H`), mirrored once at image export. On top of it, the UV fragment preset (quad-local `ctx.quad.uv → RG`, and `ctx.screen.uv → RG`), the UV-passthrough coordinate view (u → red, v → green, plus a quad-local δ mode), and the pointer-picking path that flips canvas coordinates the same way before computing the picked quad or z — with the picking cross-check.

## References
- `docs/design/principia_coordinate_conventions_note.md` § "Principia — coordinate conventions"
- `docs/design/principia_coordinate_conventions_note.md` § "The one-line rule"
- `docs/design/principia_coordinate_conventions_note.md` § "Code paths that must honour the single convention (each is a separate path)"
- `docs/design/principia_debug_tooling_plan.md` § "A. Kernel debug dispatch modes (skip integration; reuse payload slots as scratch)"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `docs/design/principia_debug_tooling_plan.md` § "Build order within Phase 0 (the only forced staggering)"
- `docs/design/principia_coordinate_conventions_note.md` § "The catch: a UV-passthrough debug view (debug-first)"
- `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"

## Deliverables
- `crates/render/shaders/wgsl/lib/coords.wgsl`: the single framebuffer→UV flip, commented as the convention flip.
- `crates/render/src/export.rs`: the single export flip (named) for image output.
- `crates/engine/src/picking.rs`: canvas event → post-flip UV → picked quad / z (lock, hover and click-inspect share it).
- Presets (serialised graphs, locked): `uv_screen`, `uv_quad`, and the coordinate view with its δ mode.
- Golden fixtures `fixtures/golden/m1-coords/`.

## Acceptance tests
- Review checklist (code): a grep finds one framebuffer→UV flip and one export flip, both named; no other Y inversion in coordinate code (REQ-SYS-009).
- `cargo test -p render uv_preset_reconstruction` — sampled quads' reconstructed (u, v) match c ± h·(2t−1); adjacent-sample deltas vary smoothly, not step-quantised (REQ-TOOL-019).
- `cargo xtask golden m1-coords` — green increases upward in the coordinate view's golden image; picking returns the expected corners (top-left click → v ≈ 1, bottom-left → v ≈ 0) (REQ-TOOL-027).
- `cargo test -p engine picking_corners` — click each known corner; the reported UV/IC coordinate is that corner (REQ-GUI-001).

## Notes
- The GUI (egui) is M8; at M1 picking is exercised by synthetic canvas events through the same function the GUI will call, so the M8 GUI inherits the convention rather than re-deriving it.
- A mirrored image is fixed at this seam only — never by a compensating flip elsewhere (coordinate note).
- The δ-mode colour mapping and the exact golden tolerance come from the golden runner's defaults (M0).
