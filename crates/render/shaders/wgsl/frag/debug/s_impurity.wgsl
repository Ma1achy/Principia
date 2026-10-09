// A structural view (debug_tooling_plan §F; colour_composition §6, "§F structural views → `ctx.quad` presets";
// REQ-TOOL-026; TASK-M1-13), the impurity view: `ctx.quad.impurity`, §3.7's `outcome_impurity`, `1 − max(class
// fraction)`, on Magma (colour_composition §6) over the fixed [0, 1] (`RANGE_AUTO` 0), or the measured range. It reads
// quad metadata, not payload. A lane the raster did not place (`shade_sample`) holds the absence NaN, or its bits in a
// u32 member, and draws `debug_invalid`'s hatch (R-136).
// @uniform RANGE_AUTO: u32 = 0 [0, 1]
// @uniform u_range: vec2<f32> = (0.0, 1.0)
fn colour(ctx: Ctx) -> vec3<f32> {
    if is_absent_nan(ctx.quad.impurity) {
        return debug_invalid(ctx.frag_xy);
    }
    let auto_range = uniforms.RANGE_AUTO != 0u;
    let t = range_norm(ctx.quad.impurity, 0.0, 1.0, auto_range, uniforms.u_range);
    return ramp_magma(t);
}
