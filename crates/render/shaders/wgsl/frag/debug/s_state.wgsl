// A structural view (debug_tooling_plan §F; colour_composition §6, "§F structural views → `ctx.quad` presets";
// REQ-TOOL-026; TASK-M1-13), the quad-state view: `ctx.quad.state`, dd_generation_root §3.7a's `quad_state`, 0 loaded,
// 1 pending, 2 refinable, 3 terminal, 4 stale, as discrete colours, `dbg_cat(state, 5)` (render contract Part 5): the
// Okabe–Ito palette in its order. A code beyond the enum shows its literal place in the cycle, not the hatch (R-79,
// R-136). It reads quad metadata, not payload. A lane the raster did not place (`shade_sample`) holds the absence NaN's
// bits and draws `debug_invalid`'s hatch.
fn colour(ctx: Ctx) -> vec3<f32> {
    if ctx.quad.state == CTX_ABSENT_BITS {
        return debug_invalid(ctx.frag_xy);
    }
    return dbg_cat(ctx.quad.state, 5u);
}
