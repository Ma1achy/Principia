// Two co-located symptoms with separate causes (pitfalls §8), for the repro tests: a magenta disc of radius `disc`
// pixels centred at (32, 32), and a white column at x = `stripe`, over a red ramp R = x / 63.
override disc: f32 = 0.0;
override stripe: f32 = -1.0;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let c = pos.xy - vec2<f32>(32.0, 32.0);
    if (dot(c, c) < disc * disc) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (floor(pos.x) == stripe) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    return vec4<f32>(floor(pos.x) / 63.0, 0.0, 0.0, 1.0);
}
