// Breaks no-f64: WGSL has no f64.
struct SimStateFTLE {
    r: array<vec2<f32>, 3>,
    p: array<vec2<f32>, 3>,
    packed_a: u32,
}

const SIMSTATE_GROUP: u32 = 1u;
const SIMSTATE_BINDING: u32 = 0u;
const WORD_GROUP: u32 = 1u;
const WORD_BINDING: u32 = 1u;

@group(1) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;
@group(1) @binding(1) var<storage, read> word_buffer: array<vec4<u32>>;

fn sample_read(i: u32) -> vec2<u32> {
    let state = simstate_buffer[i].packed_a;
    let word = word_buffer[i].w;
    return vec2<u32>(state, word);
}
fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 3u); }

fn wide(x: f64) -> f64 { return x; }
