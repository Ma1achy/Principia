// A field ramp of `mean_y` (colour_composition §1.2; render::colour::field_ramp): a lin compaction, viridis,
// and the invalid lane, the hatched pattern unless INVALID_OVERRIDE is 1 (REQ-COL-001; R-132).
// @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]
// @uniform INVALID_COLOUR: vec3<f32> = (1.0, 0.0, 1.0)
fn colour(ctx: Ctx) -> vec3<f32> {
    let v = ctx.sample.mean_y;
    if (!(bitcast<u32>(v) != 0x7fc00000u)) {
        return select(debug_invalid(ctx.frag_xy), uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);
    }
    return ramp_viridis(range_norm(v, 0.0, 1.0, false, vec2<f32>(0.0, 0.0)));
}
