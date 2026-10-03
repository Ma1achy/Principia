// Breaks the self-compare rule: a buffer element read twice and compared with itself.
struct Sample { v: f32, w: f32 }

@group(2) @binding(0) var<storage, read> samples: array<Sample>;

fn is_unset(i: u32) -> bool {
    return samples[i].v != samples[i].v; // fires
}
