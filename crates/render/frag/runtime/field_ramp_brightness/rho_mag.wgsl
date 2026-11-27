// @version 2
// A scalar field of the read side as a lightness: compacted to [0, 1] by the curve over the range, the polarity
// applied. A blank, unset or invalid sample returns 0; its style is drawn by the ramp's post, which takes the same
// field and curve. Every parameter is written into the code where the node is made.
// @name {field} lightness
// @group Brightness
// @about A read-side field as a lightness; the ramp's post draws its styles.
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
fn brightness(ctx: Ctx) -> f32 {
    handles_validity();
    let v = ctx.ic.rho_mag;
    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) { return 0.0; }
    if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) { return 0.0; }
    if (!((bitcast<u32>(v) & 0x7f800000u) != 0x7f800000u)) {
        if ((bitcast<u32>(v) & 0x7f800000u) == 0x7f800000u) { note_non_finite(); }
        return 0.0;
    }
    return range_norm(v, 0.0, 1.0, false, vec2<f32>(0.0, 0.0));
}
