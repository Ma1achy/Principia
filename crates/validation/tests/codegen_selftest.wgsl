// The codegen self-test's entry point (debug_tooling_plan §H; dd_generation_root §5 test 2): appended to the generated
// fragment unpack layer, `crates/render/frag/generated/payload_unpack.wgsl`, it reads packed words and writes the
// field each invocation's selector names, through the generated accessor only. f32 results are written as their bits.
// Selectors are `FIELDS`' indices in `codegen_selftest.rs`; 11 and 12 are the display fractions, `arg` the
// `horizon_steps` uniform (payload §6). Group 0 is the harness's: words, selectors, args, then the output.

@group(0) @binding(0) var<storage, read> st_words: array<u32>;
@group(0) @binding(1) var<storage, read> st_selectors: array<u32>;
@group(0) @binding(2) var<storage, read> st_args: array<u32>;
@group(0) @binding(3) var<storage, read_write> st_out: array<u32>;

fn st_unpack(w: u32, selector: u32, arg: u32) -> u32 {
    switch selector {
        case 0u: { return sd_state(w); }
        case 1u: { return sd_detail(w); }
        case 2u: { return select(0u, 1u, sd_saturated(w)); }
        case 3u: { return sd_dmin_pair(w); }
        case 4u: { return sd_last_symbol(w); }
        case 5u: { return bitcast<u32>(pa_d_min(w)); }
        case 6u: { return bitcast<u32>(pb_dE_max(w)); }
        case 7u: { return bitcast<u32>(pb_dLz_max(w)); }
        case 8u: { return tm_t_end_step(w); }
        case 9u: { return tm_t_dmin_step(w); }
        case 10u: { return fgw_length_raw(vec4<u32>(0u, 0u, 0u, w)); }
        case 11u: { return bitcast<u32>(tm_t_end_fraction(w, arg)); }
        case 12u: { return bitcast<u32>(tm_t_dmin_fraction(w, arg)); }
        default: { return 0xffffffffu; }
    }
}

@compute @workgroup_size(64)
fn codegen_selftest_unpack(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&st_words)) {
        st_out[id.x] = st_unpack(st_words[id.x], st_selectors[id.x], st_args[id.x]);
    }
}
