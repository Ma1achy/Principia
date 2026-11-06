// Test-only, appended to `noise_source()` by `mock_lattice.rs` (REQ-GUI-412): entry points that run the shipped
// functions and read what they give back as bits, `bitcast<u32>` to `Rgba32Uint`, so no float is canonicalised on the
// way; and the corners the tests must reject. A control takes `cell_corners`' place in the whole module
// (`mock_lattice.rs`'s `control_source`), so `value_noise`, `fbm` and `stand_in_at` run on it as they stand.

// Which fbm the probe follows and how far: the warp, 0 for `warp.x` and 1 for `warp.y`, and the octave.
@group(0) @binding(3) var<uniform> lattice_case: vec4<u32>;

struct LatticeOut {
    @location(0) first: vec4<u32>,
    @location(1) second: vec4<u32>,
};

// The point `value_noise` is handed at the probe's warp and octave, by the shipped float path.
fn lattice_point(xy: vec2<f32>) -> vec2<f32> {
    let points = warp_points(xy, vec2<f32>(0.0, 0.0));
    var q = select(points.x, points.y, lattice_case.x == 1u);
    for (var octave = 0u; octave < lattice_case.y; octave++) {
        q = next_octave(q);
    }
    return q;
}

// The pixel's four corners' bits, and its cell, as `value_noise` gets them.
@fragment
fn fs_lattice_corners(in: VsOut) -> LatticeOut {
    let c = vec2<i32>(floor(lattice_point(field_xy(in))));
    return LatticeOut(bitcast<vec4<u32>>(cell_corners(c)), vec4<u32>(bitcast<vec2<u32>>(c), 0u, 0u));
}

// The shipped field's raw pre-colour values' bits: `n`, `warp.x`, `warp.y`.
@fragment
fn fs_lattice_field(in: VsOut) -> LatticeOut {
    let f = stand_in_at(field_xy(in), vec2<f32>(0.0, 0.0));
    return LatticeOut(bitcast<vec4<u32>>(vec4<f32>(f.n, f.warp, 0.0)), vec4<u32>(0u));
}

// One ulp up, on the bits. No float operation, so it fails everywhere by construction.
fn nudged(v: f32) -> f32 {
    return bitcast<f32>(bitcast<u32>(v) + 1u);
}

// The shipped corners with the one at `c + (1, 0)` nudged: cell N's copy of a corner cell N+1 reads unnudged.
fn nudged_cell_corners(c: vec2<i32>) -> vec4<f32> {
    return vec4<f32>(
        corner(c),
        nudged(corner(c + vec2<i32>(1, 0))),
        corner(c + vec2<i32>(0, 1)),
        corner(c + vec2<i32>(1, 1)),
    );
}

// The float hash this noise had before R-689.
fn old_hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    let r = q + dot(q, q.yx + vec2<f32>(19.19, 47.13));
    return fract(r.x * r.y);
}

// The old hash's four corners, its offsets compile-time and in float, as the old `value_noise` wrote them. Only
// `lattice_old_hash_measurement` reads it: Metal and lavapipe keep these equal, so it is no control there.
fn old_cell_corners(c: vec2<i32>) -> vec4<f32> {
    let i = vec2<f32>(c);
    return vec4<f32>(
        old_hash(i),
        old_hash(i + vec2<f32>(1.0, 0.0)),
        old_hash(i + vec2<f32>(0.0, 1.0)),
        old_hash(i + vec2<f32>(1.0, 1.0)),
    );
}

// `old_hash(i + o)` with the multiply distributed over the offset: `fract(i*k + o*k)` for `fract((i + o)*k)`, the
// rewrite ADR-0001 models (`plan/adr/ADR-0001-mock-noise-integer-lattice.md` § "Context"), by hand.
fn distributed_hash(i: vec2<f32>, o: vec2<f32>) -> f32 {
    let k = vec2<f32>(127.1, 311.7);
    let q = fract(i * k + o * k);
    let r = q + dot(q, q.yx + vec2<f32>(19.19, 47.13));
    return fract(r.x * r.y);
}

// The distributed-offset corners: the cell's own corner is `old_hash(i)`, and the others are not the `old_hash(i)` of
// the cells they are shared with.
fn distributed_cell_corners(c: vec2<i32>) -> vec4<f32> {
    let i = vec2<f32>(c);
    return vec4<f32>(
        old_hash(i),
        distributed_hash(i, vec2<f32>(1.0, 0.0)),
        distributed_hash(i, vec2<f32>(0.0, 1.0)),
        distributed_hash(i, vec2<f32>(1.0, 1.0)),
    );
}
