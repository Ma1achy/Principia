# TASK-M1-13 — Structural views over RenderQuad and the boundary overlay

- **Milestone:** M1
- **Closes:** REQ-TOOL-026, REQ-RENDER-024, REQ-TOOL-124
- **Depends on:** TASK-M1-07, TASK-M1-08
- **Needs (earlier milestones):** REQ-PAY-002, REQ-SCHED-001, REQ-SYS-003
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Structural views read quad metadata, not payload, over a synthetic `RenderQuad` set before any scheduler fills it: quad-depth, quad-state (loaded / pending / refinable / terminal / stale), coherence/impurity, ensemble spread (present iff contains-ensemble), suspect fraction, priority score, cache age / ancestor gap, leaf outlines, fallback tint and pending hatch. The Tier-1 boundary overlay is an ordinary post node drawing from the quad-local and tile-local uv with the fwidth-based `edge_line`: constant pixel width at any quad size, depth and zoom, antialiased, serialising with the graph with editable width, opacity, colour and level.

## References
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/render/shaders/wgsl/frag/post/edge_line.wgsl` (post occupant) and presets for quad / tile boundaries.
- `crates/render/shaders/wgsl/frag/debug/`: `s_depth`, `s_state`, `s_impurity`, … as `ctx.quad` presets; the Tier-3 tile debug shader path (small CPU→GPU per-visible-quad buffer, postprocess) for the scheduler-verdict overlays.
- Synthetic `RenderQuad` sets in `crates/engine/src/synthetic.rs` covering every quad-state and depths 3 and 20.
- Golden fixtures `fixtures/golden/m1-structural/`.

## Acceptance tests
- `cargo xtask golden m1-structural` — each structural view renders from a synthetic RenderQuad set; asserted values match the CPU structs (REQ-TOOL-026).
- `cargo xtask golden m1-structural` — boundary lines at depths 3 and 20 have the same pixel width in the golden images; the overlay persists across save / load of the graph (REQ-RENDER-024).
- Definition: the pending-hatch pattern and the fallback tint written into debug_tooling_plan §F and approved by the physics reviewer (REQ-TOOL-124).

## Notes
- `ctx.tile.uv` (render_gui_spec §12.1) is not in colour_composition §3's lane list (milestone Gaps; carried from TASK-M1-06).
- The pending-hatch pattern and the fallback-tint colour are not given by the corpus (milestone Gaps).
- The quad-state test at M1 is structural only; the "state transitions legal" assertion of debug plan §F needs the scheduler (M5).
- Closes, for gaps the corpus leaves open: REQ-TOOL-124 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
