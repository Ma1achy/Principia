// A test fixture's pass: lifted blacks.
fn run(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(tex(in_in, uv).rgb * 0.9 + vec3<f32>(0.05 + 0.0 * p_seed), 1.0);
}
