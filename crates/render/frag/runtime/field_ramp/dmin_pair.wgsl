// @version 2
// A scalar field of the read side drawn as a colour. The field's value is compacted to [0, 1] by the curve over the
// range, the polarity says which end is bright, and the ramp turns it into a colour. Every parameter but the colours
// is written into the code where the node is made, so a ramp is the code the field, the curve and the ramp choose and
// the picture is the one a hand-written ramp of them would draw. Not a system, a failed decode, a failed run, a
// reserved state and every value the field has none draw as the standard treatment says; a value the field leaves
// unset draws the "not yet" style.
// @name {field} ramp
// @group Field ramp
// @about A read-side field on a ramp: its curve, range, polarity and colours are the node's settings.
// @maps {field}
// @glyph name
// @uniform field: field = ftle scalar
// @control field field "Field" in "Field"
// @uniform curve: u32 = 0 [0, 4] baked
// @control curve choice("lin", "log", "cyclic", "diverging", "flag") "Curve" in "Field"
// @uniform scale: f32 = 1.0 baked
// @control scale fields "Scale (ε, period or x₀)" in "Range"
// @uniform range: vec2<f32> = (0.0, 1.0) baked
// @control range fields "Range" in "Range"
// @uniform low_end: u32 = 1 [0, 2] baked
// @control low_end choice("value", "measured", "horizon") "Low end" in "Range"
// @uniform high_end: u32 = 1 [0, 2] baked
// @control high_end choice("value", "measured", "horizon") "High end" in "Range"
// @uniform symmetric: u32 = 0 [0, 1] baked
// @control symmetric toggle "Symmetric about zero" in "Range"
// @uniform polarity: u32 = 0 [0, 1] baked
// @control polarity choice("high is bright", "low is bright") "Polarity" in "Field"
// @uniform ramp: u32 = 4 [0, 5] baked
// @control ramp choice("viridis", "magma", "twilight", "coolwarm", "grey", "gradient") "Ramp" in "Ramp"
// @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]
// @uniform INVALID_COLOUR: vec3<f32> = (1.0, 0.0, 1.0)
// @control INVALID_OVERRIDE toggle "Override the invalid colour" in "Invalid"
// @control INVALID_COLOUR colour "Invalid colour" in "Invalid"
fn colour(ctx: Ctx) -> vec3<f32> {
    handles_validity();
    let v = f32(ctx.sample.dmin_pair);
    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) { return canvas_background(); }
    if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) { return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u); }
    if (!(ctx.sample.dmin_pair != 3u)) {
        return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);
    }
    return ramp_viridis(range_norm(v, 0.0, 1.0, false, vec2<f32>(0.0, 0.0)));
}
