// Breaks word-binding: the word is a SimState member, with no buffer of its own.
struct SimStateFTLE {
    r: array<vec2<f32>, 3>,
    p: array<vec2<f32>, 3>,
    packed_a: u32,
    word: vec4<u32>,
}

@group(0) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;

fn sample_state(i: u32) -> SimStateFTLE { return simstate_buffer[i]; }
fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 3u); }
