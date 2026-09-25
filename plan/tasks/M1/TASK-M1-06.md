# TASK-M1-06 — The synthetic payload harness and RenderContext: CPU-filled buffers bound to the fragment

- **Milestone:** M1
- **Closes:** REQ-RENDER-004, REQ-RENDER-007, REQ-RENDER-008, REQ-COL-003, REQ-TOOL-014, REQ-COL-056
- **Depends on:** TASK-M1-01, TASK-M1-02, TASK-M1-05, TASK-M0-02
- **Needs (earlier milestones):** REQ-PAY-001, REQ-PAY-002, REQ-PAY-008, REQ-PAY-012, REQ-SYS-003, REQ-TOOL-004
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
Debug-tooling step 0b's harness: `SimState`, word, `ICDescriptor` and `RenderQuad` buffers filled on the CPU with hand-chosen values, uploaded and bound, and a fragment stage that sees them through `RenderContext` `{sample, ic, quad, uv, screen_uv, time}` and the full `ctx` lane set (screen, chart, quad, tile/sample, payload, validity). Each sample rasterises to exactly its tile (QUAD → N×N SAMPLES → TILE → PIXELS, no interpolation). The logical `RenderSample` is accessor functions over the physical structs — nothing materialises an unpacked copy. "Screen colours a hand-filled buffer" becomes possible here.

## References
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"
- `docs/design/principia_debug_tooling_plan.md` § "Principle"
- `docs/design/principia_debug_tooling_plan.md` § "Build order within Phase 0 (the only forced staggering)"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/engine/src/synthetic.rs`: a builder for CPU-filled payload sets (per-sample setters over the generated pack routines; word buffer; `ICDescriptor`; `RenderQuad`), plus a flat N×N quad layout.
- `crates/render/src/bind.rs`: bind groups for the four buffers + uniforms; generated WGSL `RenderContext` and `ctx` lanes (from the layout definition).
- `crates/render/src/raster.rs`: the sample → tile rasterisation (one sample per tile, no interpolation).
- Headless render-to-texture helper used by every later golden test.

## Acceptance tests
- `cargo test -p render tile_rasterisation` — each sample rasterises to exactly its tile; pixel colours inside a tile come from one sample (REQ-RENDER-004).
- Review checklist (code): no compute pipeline writes an unpacked `RenderSample` buffer; fragment code reads through generated accessors (REQ-RENDER-007).
- `cargo test -p render render_context_members` — the generated WGSL `RenderContext` declares sample, ic, quad, uv, screen_uv, time; a debug view reading each compiles (REQ-RENDER-008).
- `cargo test -p render ctx_lanes` — the generated `ctx` WGSL declares every lane of colour_composition §3; each payload field has a validity accessor (REQ-COL-003).
- Review checklist (qa): a test uploads a CPU-filled SimState/ICDescriptor/RenderQuad set and renders a debug view (`cargo test -p render synthetic_upload_renders`); the plan order follows contract + SDK → debug catalogue → flat compute → quads → adaptive (REQ-TOOL-014).
- Definition: `ctx.tile.uv` added to colour_composition §3's tile/sample lane and approved by the physics reviewer (REQ-COL-056).

## Notes
- `ctx.chart.z` and `chart_id` exist as lanes but charts land in M2; the harness fills them from uniforms.
- render_gui_spec §12.1 draws tile boundaries from `ctx.tile.uv`, which colour_composition §3's tile/sample lane does not list (milestone Gaps); this task declares the lanes §3 lists.
- Closes, for gaps the corpus leaves open: REQ-COL-056 (R-72 definition) (classification accepted by R-132).
