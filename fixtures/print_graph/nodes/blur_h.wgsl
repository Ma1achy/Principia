// A test fixture's first pass: across.
fn run(uv: vec2<f32>) -> vec4<f32> {
    let d = vec2<f32>(px.x * p_sigma, 0.0);
    return 0.5 * (tex(in_in, uv - d) + tex(in_in, uv + d));
}
