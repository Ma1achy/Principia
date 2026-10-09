// The fallback ancestor tint (debug_tooling_plan §F; REQ-TOOL-026, REQ-TOOL-124; TASK-M1-13): a post occupant reading
// `ctx.quad` (RQ-239). A quad whose `ancestor_gap` is above 0 draws an ancestor's payload in its place
// (dd_generation_root §3.7a), and is tinted: its colour mixed 0.4 of the way (its opacity) toward pink #FF69FF, 8-bit
// sRGB, a colour that collides with no palette entry and is neither of `debug_invalid`'s (REQ-TOOL-124; proposed,
// R-71). A flat tint, no pattern, so the data beneath still reads. A quad drawn from its own payload, and a lane the
// raster did not place, pass unchanged.

// #FF69FF, 8-bit sRGB, decoded to linear, and the tint's opacity.
const FALLBACK_TINT_SRGB8: vec3<f32> = vec3<f32>(255.0, 105.0, 255.0);
const FALLBACK_OPACITY: f32 = 0.4;

fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {
    let gap = ctx.quad.ancestor_gap;
    if gap == 0u || gap == CTX_ABSENT_BITS {
        return rgb;
    }
    return mix(rgb, srgb_to_linear(FALLBACK_TINT_SRGB8 / 255.0), FALLBACK_OPACITY);
}
