// A worked example of a custom colour node that declares itself: every control and the swatches glyph. Paste it into
// a colour node and the editor draws it as it draws a built-in. The descriptive lines are comments; the code is the
// uniforms and `colour` below.
// @version 1
// @name Banded tint
// @group Examples
// @about A field's value as bands of a gradient, tinted, with every control the editor draws.
// @maps its field [lin, 0 to 1]
// @glyph swatches "Tints"
// @uniform strength: f32 = 0.5 [0, 1]
// @control strength slider "Strength" in "Shape" order 3
// @uniform scale: f32 = 1.0 [0.01, 100]
// @control scale log_slider "Scale" in "Shape" order 2
// @uniform bands: i32 = 4 [1, 32]
// @control bands stepper "Bands" in "Shape" order 1
// @uniform invert: u32 = 0 [0, 1]
// @control invert toggle "Invert"
// @uniform mode: u32 = 0 [0, 2]
// @control mode choice("bands", "smooth", "steps") "Mode" order 0
// @uniform mask: u32 = 7
// @control mask flags("red", "green", "blue") "Channels"
// @uniform warm: vec3<f32> = (1.0, 0.5, 0.2)
// @control warm colour "Warm" in "Tints"
// @uniform cool: vec3<f32> = (0.2, 0.4, 1.0)
// @control cool colour "Cool" in "Tints"
// @uniform hue: f32 = 0.0
// @control hue angle "Hue turn"
// @uniform low: vec3<f32> = (0.0, 0.0, 0.0)
// @uniform mid: vec3<f32> = (0.5, 0.5, 0.5)
// @uniform high: vec3<f32> = (1.0, 1.0, 1.0)
// @control low gradient(mid, high) "Ramp"
// @uniform offset: vec2<f32> = (0.0, 0.0)
// @control offset fields "Offset"
// @uniform measured: vec2<f32> = (0.0, 1.0)
// @control measured hidden "Measured range"
fn banded_tint(t: f32) -> vec3<f32> {
    let u = clamp(t * uniforms.scale + uniforms.offset.x, 0.0, 1.0);
    let k = floor(u * f32(uniforms.bands)) / f32(uniforms.bands);
    let v = select(u, k, uniforms.mode != 1u);
    let w = select(v, 1.0 - v, uniforms.invert != 0u);
    let ramp = select(mix(uniforms.mid, uniforms.high, 2.0 * w - 1.0), mix(uniforms.low, uniforms.mid, 2.0 * w), w < 0.5);
    let tint = mix(uniforms.cool, uniforms.warm, 0.5 + 0.5 * cos(uniforms.hue));
    let rgb = mix(ramp, ramp * tint, uniforms.strength);
    let keep = vec3<f32>(f32(uniforms.mask & 1u), f32((uniforms.mask >> 1u) & 1u), f32((uniforms.mask >> 2u) & 1u));
    return rgb * keep + 0.0 * uniforms.measured.x;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return banded_tint(ctx.inputs[0].x);
}
