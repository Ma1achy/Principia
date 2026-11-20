// The golden runner's self-test (TASK-M0-06; R-186): an analytic gradient over a 256x256 target. At pixel (x, y),
// whose centre @builtin(position) gives as (x + 0.5, y + 0.5), the output is
//   R = min(x + step, 255) / 255,  G = y / 255,  B = floor((x + y) / 2) / 255,
// so each channel is k / 255 for an integer k and the Rgba8Unorm target stores k exactly: the reference, at
// step = 0, is computed from these formulas, not rendered. `step` is the one variable a repro or the tolerance's
// evidence changes: at step = 1 the render moves by one 8-bit step, and must fail.
override step: f32 = 0.0;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<u32>(pos.xy);
    let r = min(f32(p.x) + step, 255.0);
    return vec4<f32>(r / 255.0, f32(p.y) / 255.0, f32((p.x + p.y) / 2u) / 255.0, 1.0);
}
