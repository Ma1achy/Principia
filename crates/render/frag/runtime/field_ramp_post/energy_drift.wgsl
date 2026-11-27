// @version 2
// The styles of a field ramp over the combiner's output: the canvas background for a pixel that is not a system and a
// failed decode, the "not yet" style at the field's unset value, the invalid treatment where the field's value is
// invalid, and every other colour as it is. A lightness draws neither style, so the brightness ramp's post does,
// with the same field and curve.
// @name {field} styles
// @group Post
// @about The background, not-yet and invalid styles of a field's lightness, over the combiner's colour.
// @maps {field}
// @glyph name
// @uniform field: field = ftle scalar
// @control field field "Field" in "Field"
// @uniform curve: u32 = 0 [0, 4] baked
// @control curve choice("lin", "log", "cyclic", "diverging", "flag") "Curve" in "Field"
// @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]
// @uniform INVALID_COLOUR: vec3<f32> = (1.0, 0.0, 1.0)
// @control INVALID_OVERRIDE toggle "Override the invalid colour" in "Invalid"
// @control INVALID_COLOUR colour "Invalid colour" in "Invalid"
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {
    handles_validity();
    let v = ctx.sample.energy_drift;
    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) { return canvas_background(); }
    if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) { return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u); }
    if (!((bitcast<u32>(v) & 0x7f800000u) != 0x7f800000u)) {
        if ((bitcast<u32>(v) & 0x7f800000u) == 0x7f800000u) { note_non_finite(); }
        return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);
    }
    return rgb;
}
