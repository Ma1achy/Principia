// A field ramp of `E_0` (colour_composition §1.2; render::colour::field_ramp): the `lin` compaction, polarity high,
// then `lut(viridis)`, with the invalid lane, #141418 unless INVALID_OVERRIDE is 1, and the canvas background
// for a pixel that is not a system (REQ-COL-001, REQ-COL-108; R-467, R-511).
// @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]
// @uniform INVALID_COLOUR: vec3<f32> = (1.0, 0.0, 1.0)
fn colour(ctx: Ctx) -> vec3<f32> {
    let v = ctx.sample.E_0;
    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) { return canvas_background(); }
    if (!((bitcast<u32>(v) & 0x7f800000u) != 0x7f800000u)) {
        if ((bitcast<u32>(v) & 0x7f800000u) == 0x7f800000u) { note_non_finite(); }
        return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);
    }
    return ramp_viridis(range_norm(v, 0.0, 1.0, false, vec2<f32>(0.0, 0.0)));
}
