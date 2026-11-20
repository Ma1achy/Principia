// A test fixture's pass: ink where the picture is dark, the ground where it is light.
fn run(uv: vec2<f32>) -> vec4<f32> {
    let l = dot(tex(in_in, uv).rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let cut = step(p_depth * tex(in_field, uv).r, l + 0.01 * p_seed * p_gouge * px.x * res.x);
    return vec4<f32>(mix(p_ink, tex(in_ground, uv).rgb, cut), 1.0);
}
