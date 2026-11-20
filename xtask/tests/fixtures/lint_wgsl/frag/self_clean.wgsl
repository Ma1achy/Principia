// Every float rule holds. The near misses an over-broad self-compare rule would fail: two different floats, two
// different buffer elements, a variable compared with its own earlier value after it is reassigned, and an unset
// value tested by its bits.
struct Sample { v: f32, w: f32 }

@group(2) @binding(0) var<storage, read> samples: array<Sample>;

const PA_D_MIN_UNSET: u32 = 0x7c00u;

fn differ(x: f32, y: f32) -> bool {
    return x != y;
}

fn components_differ(a: vec2<f32>) -> bool {
    return a.x == a.y;
}

fn elements_differ(i: u32, j: u32) -> bool {
    return samples[i].v == samples[j].v;
}

fn moved(start: f32) -> bool {
    var v = start;
    let old = v;
    v = v + 1.0;
    return old != v;
}

fn pa_d_min_is_unset(w: u32) -> bool { return extractBits(w, 16u, 16u) == PA_D_MIN_UNSET; }
