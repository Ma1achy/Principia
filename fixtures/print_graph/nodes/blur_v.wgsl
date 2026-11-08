// A test fixture's second pass: down, reading the first.
fn run(uv: vec2<f32>) -> vec4<f32> {
    let d = vec2<f32>(0.0, px.y * p_sigma);
    return 0.5 * (tex(prev, uv - d) + tex(prev, uv + d));
}
