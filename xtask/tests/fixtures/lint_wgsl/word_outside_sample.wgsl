// Breaks sample-only: `peek_word` indexes word_buffer, which only sample_word reads (R-343).
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

fn sample_state(i: u32) -> SimStateFTLE { return simstate_buffer[i]; }
fn sample_word(i: u32) -> vec4<u32> { return word_buffer[i]; }
fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 3u); }

fn peek_word(i: u32) -> vec4<u32> { return word_buffer[i]; }
