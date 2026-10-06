# TASK-M1-07 — The coordinate convention: the one flip, the UV preset, the coordinate view and picking

- **Milestone:** M1
- **Closes:** REQ-SYS-009, REQ-TOOL-019, REQ-TOOL-027, REQ-GUI-001, REQ-SYS-080, REQ-TOOL-152, REQ-TOOL-153
- **Depends on:** TASK-M1-06
- **Needs (earlier milestones):** REQ-TOOL-004
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~450 lines

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
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `crates/render/shaders/wgsl/lib/coords.wgsl`: the single framebuffer→UV flip, commented as the convention flip.
- `crates/render/src/coords.rs`: the flip's Rust twin, named as the same flip, which picking and the CPU raster call; `flip_twin_matches_wgsl` holds the two equal (RQ-211).
- `crates/render/src/raster.rs` (TASK-M1-06's): its four Y inversions, the WGSL `raster`'s two, `Grid::cell`'s and `tile_pixels`', rewritten to call the named flip of their language (RQ-211).
- `crates/render/src/export.rs`: the image export, the named export seam: it takes the headless readback as returned (rows from the top) and writes the PNG's rows in that order, unreversed; the coordinate note's path-5 flip, which fires only for a Y-up internal image, does not fire for it (REQ-SYS-080; RQ-212).
- `crates/engine/src/picking.rs`: canvas event → post-flip UV (through the Rust twin) → picked quad and the screen's single `ctx.chart.z` (lock, hover and click-inspect share it); per-pixel z arrives with `SimConfig.plane` in M2, through the same function (RQ-216).
- The synthetic harness (`crates/engine/src/synthetic.rs`, TASK-M1-06's) supplies each quad's centre c and half-width h, from its own grid, for the UV preset's reconstruction (RQ-214).
- Presets, as in-code graphs (`engine::stain::StainGraph` values), not editable, with no file format: `uv_screen`, `uv_quad`, and the coordinate view with its δ mode. The serialised preset format and locked loading are TASK-M7-17's (REQ-GUI-028, REQ-GUI-029; RQ-215).
- `xtask/src/golden.rs`: a case field naming WGSL files the runner prepends to the case's shader, by repo-relative path (RQ-210).
- Golden fixtures `fixtures/golden/m1-coords/`, whose fragment prepends `crates/render/shaders/wgsl/lib/coords.wgsl` and calls its flip, and the case's row in `fixtures/golden/BASELINES.md`, citing R-369 (RQ-210).
- Definitions (R-72): the export's orientation in the coordinate note's path 5 (REQ-SYS-080); the δ mode's normalisation and mapping in debug_tooling_plan §F's UV-passthrough row (REQ-TOOL-153).

## Acceptance tests
- Review checklist (code): a grep finds the named framebuffer→UV flip once per language, the WGSL flip in `lib/coords.wgsl` and its Rust twin in `crates/render/src/coords.rs`, and the named export seam, which reverses no rows; no other Y inversion in coordinate code: `raster.rs` calls the named flips, and `embed/search.rs`'s `FlipV`, an image transform the embedding recovery searches, is not coordinate code (REQ-SYS-009).
- `cargo test -p render flip_twin_matches_wgsl` — the WGSL flip, rendered through the headless helper at every row of a target, equals the Rust twin; a twin off by one row fails it (REQ-SYS-009).
- `cargo test -p render export_orientation` — the coordinate view's exported PNG has its top-left pixel at UV (0, 1) and its bottom-left at (0, 0); a row-reversed export fails it (REQ-SYS-080).
- Definition: the export's orientation written into the coordinate note's path 5 and approved by the physics reviewer (REQ-SYS-080).
- `cargo test -p render uv_preset_reconstruction` — sampled quads' reconstructed (u, v) match deep_zoom §1's c + h·(2t − 1), with c and h the synthetic harness's per-quad values; adjacent-sample deltas pass the banding criterion, and its negative fixture, a quad deep enough that the global form u_min + (u_max − u_min)·t bands in f32, fails it (REQ-TOOL-019, REQ-TOOL-152).
- Proposal: the banding criterion and its bound, with the measured adjacent-sample deltas on the flat grid and on the negative fixture; the human confirms it at the M1 gate (REQ-TOOL-152).
- `cargo xtask golden m1-coords` — green increases upward in the coordinate view's golden image, its fragment taking the flip from `lib/coords.wgsl`, which the runner prepends by path (REQ-TOOL-027).
- `cargo test -p render delta_mode` — on sampled quads the δ mode draws (δ_u/h_u + 1)/2 in R and (δ_v/h_v + 1)/2 in G, with B = 0 (REQ-TOOL-153).
- Definition: the δ mode's normalisation by 2^−(ℓ+1) and its (x + 1)/2 mapping written into debug_tooling_plan §F and approved by the physics reviewer (REQ-TOOL-153).
- `cargo test -p engine picking_corners` — click each known corner through the function the GUI will call; the reported UV is that corner (top-left click → v ≈ 1, bottom-left → v ≈ 0), with its quad and the screen's `ctx.chart.z` (REQ-GUI-001, REQ-TOOL-027).

## Notes
- The GUI (egui) is M8; at M1 picking is exercised by synthetic canvas events through the same function the GUI will call, so the M8 GUI inherits the convention rather than re-deriving it.
- A mirrored image is fixed at this seam only — never by a compensating flip elsewhere (coordinate note).
- The δ-mode colour mapping is REQ-TOOL-153's definition (RQ-213); the golden tolerance is REQ-VAL-138's, the runner's one tolerance (`max_step: 0`, confirmed by R-376).
- Picking is checked by `picking_corners` alone: the golden runner binds no `RenderContext` and xtask may not depend on `engine` or `render` (systems_architecture §7.1), so `m1-coords` renders the view only (RQ-210).
- Physics reviews this task for its definition requirements (REQ-SYS-080, REQ-TOOL-153), as it reviews TASK-M1-06's (RQ-212).
- RQ-210 to RQ-216 decided per R-369 (`docs/archive/review_queue/M0.md`).
