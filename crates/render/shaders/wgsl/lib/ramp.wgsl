// The ramps of colour_composition §1.2, scalar t ∈ [0, 1] → linear RGB, the colour slot's space: hand-written,
// following the generated prelude (`prelude.wgsl`) at assembly. `lut(name)` is the prelude's own `ramp_<name>`
// (`ramp_viridis`, `ramp_magma`, `ramp_twilight`, `ramp_coolwarm`, `ramp_grey`); this file holds the other three. Each
// clamps t to [0, 1], as the prelude's ramps do. The CPU mirror is `render::colour::field_ramp::Ramp::colour`.

// `lerp(c0, c1)`: c0 at t = 0 to c1 at t = 1, mixed in linear RGB.
fn ramp_lerp(t: f32, c0: vec3<f32>, c1: vec3<f32>) -> vec3<f32> {
    return mix(c0, c1, clamp(t, 0.0, 1.0));
}

// `diverging(c−, c0, c+)`, through a neutral: c− at t = 0, the neutral c0 at t = ½ exactly, c+ at t = 1, each half
// mixed in linear RGB.
fn ramp_diverging(t: f32, lo: vec3<f32>, mid: vec3<f32>, hi: vec3<f32>) -> vec3<f32> {
    let u = clamp(t, 0.0, 1.0);
    return select(mix(mid, hi, 2.0 * u - 1.0), mix(lo, mid, 2.0 * u), u < 0.5);
}

// `bands(field, n, line_col, base)`, a band-mask: the line colour where t falls in one of the n periodic bands over
// [0, 1], each band the first `width` of its period (fract(n·t) < width, 0 < width ≤ 1), the base colour elsewhere.
fn ramp_bands(t: f32, n: f32, width: f32, line_col: vec3<f32>, base: vec3<f32>) -> vec3<f32> {
    return select(base, line_col, fract(n * clamp(t, 0.0, 1.0)) < width);
}
