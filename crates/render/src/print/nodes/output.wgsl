// Output (the lab's `output`): what the canvas shows, its input as it comes.
fn run(uv: vec2<f32>) -> vec4<f32> {
    return tex(in_in, uv);
}
