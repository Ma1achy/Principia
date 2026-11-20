// A structural view (debug_tooling_plan §F; colour_composition §6, "§F structural views → `ctx.quad` presets";
// REQ-TOOL-026; TASK-M1-13), the priority view: `ctx.quad.priority`, the scheduler's `w_v·P_v + w_z·P_z + w_c·P_c +
// w_f·P_f` (debug_tooling_plan §F), on Viridis over the measured range: neither end is bounded. It reads quad metadata,
// not payload. A lane the raster did not place (`shade_sample`) holds the absence NaN, or its bits in a u32 member, and
// draws `debug_invalid`'s hatch (R-136).
// @uniform RANGE_AUTO: u32 = 1 [0, 1]
// @uniform u_range: vec2<f32> = (0.0, 1.0)
fn colour(ctx: Ctx) -> vec3<f32> {
    if is_absent_nan(ctx.quad.priority) {
        return debug_invalid(ctx.frag_xy);
    }
    let auto_range = uniforms.RANGE_AUTO != 0u;
    let t = range_norm(ctx.quad.priority, uniforms.u_range.x, uniforms.u_range.y, auto_range, uniforms.u_range);
    return ramp_viridis(t);
}
