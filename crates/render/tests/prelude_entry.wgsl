// The test entry `tests/prelude.rs` appends to the shared prelude and the presentation layer: each pixel of a
// CASE_WIDTH-wide target evaluates one case, `cases[2i]` and `cases[2i + 1]` for the pixel at index i (row-major),
// and returns the result's bits. Word 0 of a case is its operation; the rest are its arguments' bits.

@group(2) @binding(0) var<storage, read> cases: array<vec4<u32>>;

const CASE_WIDTH: u32 = 64u;

fn rgb(c: vec3<f32>) -> vec4<u32> { return vec4<u32>(bitcast<vec3<u32>>(c), 0u); }

@fragment
fn t_prelude(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = u32(pos.y) * CASE_WIDTH + u32(pos.x);
    let a = cases[2u * i];
    let b = cases[2u * i + 1u];
    let x = bitcast<f32>(a.y);
    switch a.x {
        case 1u: {
            let t = range_norm(x, bitcast<f32>(a.z), bitcast<f32>(a.w), b.x != 0u, bitcast<vec2<f32>>(b.yz));
            return vec4<u32>(bitcast<u32>(t), 0u, 0u, 0u);
        }
        case 2u: { return rgb(ramp_viridis(x)); }
        case 3u: { return rgb(ramp_twilight(x)); }
        case 4u: { return rgb(ramp_grey(x)); }
        case 5u: { return rgb(hue_wheel(x)); }
        case 6u: { return rgb(debug_invalid(pos.xy)); }
        case 7u: { return rgb(dbg_cat(a.y, a.z)); }
        case 8u: { return rgb(dbg_lin(x, bitcast<f32>(a.z), bitcast<f32>(a.w))); }
        case 9u: { return rgb(dbg_log(x, bitcast<f32>(a.z))); }
        case 10u: { return rgb(dbg_flag(a.y != 0u)); }
        case 11u: { return rgb(dbg_hash_u32(a.y)); }
        case 12u: { return rgb(dbg_sentinel(x, pos.xy)); }
        case 13u: { return rgb(srgb_to_linear(bitcast<vec3<f32>>(vec3<u32>(a.y, a.z, a.w)))); }
        case 14u: { return rgb(linear_to_srgb(bitcast<vec3<f32>>(vec3<u32>(a.y, a.z, a.w)))); }
        default: { return vec4<u32>(0xffffffffu); }
    }
}
