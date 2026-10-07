# TASK-M1-13 — Structural views over RenderQuad and the boundary overlay

- **Milestone:** M1
- **Closes:** REQ-TOOL-026, REQ-RENDER-024, REQ-TOOL-124
- **Depends on:** TASK-M1-07, TASK-M1-08, TASK-M1-09
- **Needs (earlier milestones):** REQ-PAY-002, REQ-SCHED-001, REQ-SYS-003
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
Structural views read quad metadata, not payload, over a synthetic `RenderQuad` set before any scheduler fills it: quad-depth, quad-state (loaded / pending / refinable / terminal / stale), coherence/impurity, ensemble spread (present iff contains-ensemble, read as the prelude's `has_ensemble()`, R-145), suspect fraction, priority score, cache age / ancestor gap, leaf outlines, fallback tint and pending hatch. They read `ctx.quad`, which this task extends with colour_composition §3's quad members, and the tile boundaries read `ctx.tile.uv` (RQ-238). The fallback tint (`ancestor_gap > 0`) and the pending hatch (`quad_state == 1`) are post occupants reading `ctx.quad`, not the Tier-3 path, which is TASK-M5-29's (RQ-239). The Tier-1 boundary overlay is an ordinary post node drawing from the quad-local and tile-local uv with the fwidth-based `edge_line`: constant pixel width at any quad size, depth and zoom, antialiased, serialising with the graph with editable width, opacity, colour and level.

## References
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/design/principia_dd_generation_root.md` § "3.7a `RenderQuad` — the CPU-written quad record"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `decisions.md` § "R-145 — The fragment side reads `has_ensemble` as a uniform *(closes RQ-75)*"
- `decisions.md` § "R-136 — `debug_invalid(frag_xy)` draws the hatch; sentinels show their value *(closes RQ-114)*"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- The stain-context extension (RQ-238): `CtxQuad` gains colour_composition §3's quad members, filled in `render::bind::preset_module` from `quad_read(r.quad)` by `QUAD_LANE`'s mapping, and a `CtxTile { uv }` lane is filled from `r.tile_uv`; `shade_sample` fills both with the absence NaN, as TASK-M1-07 does for its lanes.
- `crates/render/shaders/wgsl/frag/post/edge_line.wgsl` (post occupant) and presets for quad / tile boundaries.
- `crates/render/shaders/wgsl/frag/debug/`: `s_depth`, `s_state`, `s_impurity`, … as `ctx.quad` presets; the spread view reads `has_ensemble()` and draws the absence NaN's hatch when it is false (RQ-239).
- `crates/render/shaders/wgsl/frag/post/`: the fallback tint (`ancestor_gap > 0`) and the pending hatch (`quad_state == 1`) as post occupants reading `ctx.quad` (RQ-239), styled by REQ-TOOL-124's definition.
- Synthetic `RenderQuad` sets in `crates/engine/src/synthetic.rs` covering every quad-state and depths 3 and 20, with their deep_zoom §1 frames.
- The golden suites use `cargo xtask golden`'s harness case kind, which TASK-M1-09 builds (RQ-229, as amended per code review 5438179638).
- Golden fixtures `fixtures/golden/m1-structural/`, as harness cases, each case's `BASELINES.md` row citing R-369 and RQ-229, proposed and confirmed at the M1 gate; the structural debug views' cases also go in `fixtures/golden/debug-views/` (RQ-237).

## Acceptance tests
- `cargo xtask golden m1-structural` — each structural view renders from a synthetic RenderQuad set; asserted values match the CPU structs; with `has_ensemble()` false the spread view draws the hatch (REQ-TOOL-026).
- `cargo xtask golden m1-structural` — the depth-3 and depth-20 sets, drawn with their deep_zoom frames at two raster quad sizes (for example 16 and 64 px), give boundary lines of the same measured pixel width across all four, and a thresholded-uv control (`u < 0.01`) fails that check (REQ-RENDER-024).
- `cargo test -p engine boundary_overlay_roundtrip` — serialise the graph holding the boundary overlay, load it, and get an equal graph and an equal render key (REQ-RENDER-024).
- Definition (physics): the pending-hatch pattern and the fallback tint written into debug_tooling_plan §F, each differing from `debug_invalid`'s hatch in pattern and in colours, with REQ-COL-055's no-collision measurement rerun with them, and approved by the physics reviewer (REQ-TOOL-124).

## Notes
- `ctx.tile.uv` and `ctx.quad.uv` are defined in colour_composition §3's "The within-cell coordinates `ctx.quad.uv` and `ctx.tile.uv` (R-72; REQ-COL-056)" paragraph (TASK-M1-06).
- The pending-hatch pattern and the fallback-tint colour are not given by the corpus (milestone Gaps); REQ-TOOL-124 defines them, and the pending hatch must not read as NaN's (PIT-8, R-136).
- The quad-state test at M1 is structural only; the "state transitions legal" assertion of debug plan §F needs the scheduler (M5).
- Closes, for gaps the corpus leaves open: REQ-TOOL-124 (R-72 definition) (classification accepted by R-132).
- RQ-229 and RQ-237 to RQ-241, decided per R-369 (7 Oct 2026): the harness case kind (RQ-229); the shared `debug-views` suite (RQ-237); the stain-context extension (RQ-238); the Tier-3 path left with TASK-M5-29 and `has_ensemble()` for "contains-ensemble" (RQ-239); physics as a reviewer, the two-raster-size width test, the persistence unit test and a hatch distinct from `debug_invalid` (RQ-240); the References (RQ-241).
- The orchestrator resolves PR #160's and PR #161's conflict over the stain's `Ctx` when the second of them merges, before this task starts (RQ-238). This task cannot start early (R-388) while more than one of its dependencies is unmerged; once TASK-M1-07 and TASK-M1-08 have merged, it may start off TASK-M1-09's approved head.
