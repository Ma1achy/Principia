// The pending hatch (debug_tooling_plan §F; REQ-TOOL-026, REQ-TOOL-124; TASK-M1-13): a post occupant reading `ctx.quad`
// (RQ-239). A pending quad (`quad_state` 1, dd_generation_root §3.7a) is hatched with lines along the anti-diagonal, 2
// px wide every 8 px across `x − y`, in blue #0000FF, 8-bit sRGB, the colour beneath showing between them
// (REQ-TOOL-124; proposed, R-71). It is not `debug_invalid`'s hatch: those stripes run across `x + y`, 4 px each, and
// alternate violet and cyan over the whole pixel, so a pending quad never reads as NaN (PIT-8, R-136). Every other
// state, and a lane the raster did not place, pass unchanged.

// #0000FF, 8-bit sRGB, decoded to linear; the period and the line width, in pixels, across x − y.
const PENDING_SRGB8: vec3<f32> = vec3<f32>(0.0, 0.0, 255.0);
const PENDING_PERIOD: i32 = 8;
const PENDING_LINE: i32 = 2;

fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {
    if ctx.quad.state != 1u {
        return rgb;
    }
    let p = vec2<i32>(floor(ctx.frag_xy));
    // `& (period − 1)` is `x − y` modulo the period, a power of two, for negative differences too.
    let on = ((p.x - p.y) & (PENDING_PERIOD - 1)) < PENDING_LINE;
    return select(rgb, srgb_to_linear(PENDING_SRGB8 / 255.0), on);
}
