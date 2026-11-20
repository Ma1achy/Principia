// The vertex stage of every full-target pass, the colour pass and the three compositor passes (TASK-M1-05): one
// triangle covering the target, so each pixel runs the fragment entry once, at its centre.
@vertex
fn full_target(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}
