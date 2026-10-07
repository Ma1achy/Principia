// The UV-passthrough coordinate view (debug_tooling_plan §F; REQ-TOOL-027; TASK-M1-07): each pixel's post-flip UV,
// u → red, v → green, B = 0, so green increases upward and red rightward. The flip is not written here: the runner
// prepends `crates/render/shaders/wgsl/lib/coords.wgsl` (the case's `prepend`, RQ-210), and this fragment calls its
// `frag_uv`, the convention's one flip.

override W: f32;
override H: f32;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_uv(pos.xy, vec2<f32>(W, H));
    return vec4<f32>(uv, 0.0, 1.0);
}
