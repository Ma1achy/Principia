// Replace-L, Principia's default combiner, a built-in occupant of the `combiner` slot (dd_colouring §3.5; R-77): the
// base colour, linear sRGB, to OKLab; L ← L_min + (L_max − L_min)·b; (a, b_ab) untouched; back to linear sRGB. A bound
// brightness owns L and the colour contributes hue and chroma only (render contract Part 4, L-ownership). The defaults
// L_min = 0, L_max = 1 make it L = b (R-77). The result is not clamped: the gamut clamp is the display stage's.
// The CPU mirror is `crates/render/src/colour/combine.rs`.
// @uniform l_min: f32 = 0.0
// @uniform l_max: f32 = 1.0

// The step in OKLab: L replaced, (a, b_ab) returned as given. L_min + (L_max − L_min)·b is evaluated as
// L_min·(1 − b) + L_max·b, the same value, exact at each end: b = 0 gives L_min, b = 1 gives L_max, and at the
// defaults it is b itself.
fn replace_l_lab(lab: vec3<f32>, b: f32) -> vec3<f32> {
    return vec3<f32>(uniforms.l_min * (1.0 - b) + uniforms.l_max * b, lab.y, lab.z);
}

fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return oklab_to_linear(replace_l_lab(linear_to_oklab(rgb), b)); }
