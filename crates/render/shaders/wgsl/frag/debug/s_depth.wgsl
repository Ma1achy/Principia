// A structural view (debug_tooling_plan §F; colour_composition §6, "§F structural views → `ctx.quad` presets";
// REQ-TOOL-026; TASK-M1-13), the quad-depth heatmap: `ctx.quad.depth`, the quadtree depth ℓ, on Viridis
// (colour_composition §6). The depth has no upper bound, so by default (`RANGE_AUTO` 1) the ramp spans `u_range`, the
// measured range; `RANGE_AUTO` 0 fixes its low end at 0, its high end still the measured. It reads quad metadata, not
// payload. A lane the raster did not place (`shade_sample`) holds the absence NaN, or its bits in a u32 member, and
// draws `debug_invalid`'s hatch (R-136).
// @name s_depth
// @group Debug · structure
// @about The quad's depth in the quadtree.
// @maps ctx.quad.depth [lin, measured range]
// @glyph name
// @uniform RANGE_AUTO: u32 = 1 [0, 1]
// @control RANGE_AUTO toggle "Auto range"
// @uniform u_range: vec2<f32> = (0.0, 1.0)
// @control u_range hidden "Measured range"
fn colour(ctx: Ctx) -> vec3<f32> {
    if ctx.quad.depth == CTX_ABSENT_BITS {
        return debug_invalid(ctx.frag_xy);
    }
    let auto_range = uniforms.RANGE_AUTO != 0u;
    let t = range_norm(f32(ctx.quad.depth), 0.0, uniforms.u_range.y, auto_range, uniforms.u_range);
    return ramp_viridis(t);
}
