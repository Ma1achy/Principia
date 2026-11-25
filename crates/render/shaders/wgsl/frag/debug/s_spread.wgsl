// A structural view (debug_tooling_plan §F; colour_composition §6, "§F structural views → `ctx.quad` presets";
// REQ-TOOL-026; TASK-M1-13), the ensemble-spread view: `ctx.quad.spread`, §3.7's `max(spread_shape, spread_event)`,
// each bounded to [0, 1], on Viridis over the fixed [0, 1] (`RANGE_AUTO` 0), or the measured range. It is present iff
// the view contains an ensemble, read as the prelude's `has_ensemble()` (R-145; RQ-239): with no ensemble (E = 0) the
// view draws the absence NaN's hatch, `debug_invalid` (R-136), whatever the quad holds. A lane the raster did not place
// (`shade_sample`) holds the absence NaN and draws the hatch too.
// @name s_spread
// @group Debug · structure
// @about The quad's ensemble spread, present when the view holds an ensemble.
// @maps ctx.quad.spread [lin, 0 to 1]
// @glyph name
// @uniform RANGE_AUTO: u32 = 0 [0, 1]
// @control RANGE_AUTO toggle "Auto range"
// @uniform u_range: vec2<f32> = (0.0, 1.0)
// @control u_range hidden "Measured range"
fn colour(ctx: Ctx) -> vec3<f32> {
    if !has_ensemble() || is_absent_nan(ctx.quad.spread) {
        return debug_invalid(ctx.frag_xy);
    }
    let auto_range = uniforms.RANGE_AUTO != 0u;
    let t = range_norm(ctx.quad.spread, 0.0, 1.0, auto_range, uniforms.u_range);
    return ramp_viridis(t);
}
