// R-269's half-way fragment (TASK-M0-43; R-269, R-287): at pixel (x, y) of a 256x256 target,
//   R = (x + 0.5) / 255,  G = exp(-y / 40),  B = ((x + y) / 510)^2.2,
// so R lies exactly half-way between the 8-bit levels x and x + 1, the tie a backend's float-to-unorm conversion
// breaks its own way (R-269's measurement: Metal rounds it up, lavapipe to even). Quantised in the runner's shader,
// half to even (R-287), every backend stores the same level; through the automatic conversion (`halfway_automatic`,
// the control) they differ by one step in R.
@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<f32>(vec2<u32>(pos.xy));
    let r = (p.x + 0.5) / 255.0;
    let g = exp(-p.y / 40.0);
    let b = pow((p.x + p.y) / 510.0, 2.2);
    return vec4<f32>(r, g, b, 1.0);
}
