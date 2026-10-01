// Breaks vec2-groups: r is not array<vec2<f32>, 3>.
struct SimStateFTLE {
    r: array<f32, 6>,
    p: array<vec2<f32>, 3>,
    packed_a: u32,
}

@group(0) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;
@group(0) @binding(1) var<storage, read> word_buffer: array<vec4<u32>>;

fn sample_state(i: u32) -> SimStateFTLE { return simstate_buffer[i]; }
fn sample_word(i: u32) -> vec4<u32> { return word_buffer[i]; }
fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 3u); }
