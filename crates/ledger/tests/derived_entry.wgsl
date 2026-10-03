// The read side's test entry point (lowering Part 3a; payload §5, §6): appended to the generated fragment unpack layer
// and read side, `crates/render/frag/generated/payload_unpack.wgsl` then `read_side.wgsl`, after the tier's
// `const has_ftle: bool` (the assembler's bake, which the test writes). Each invocation reads one sample through the
// generated unpack of its variant and writes the one read-side member its selector names, an f32 as its bits, a bool
// as 0 or 1. `ledger/tests/derived.rs` and
// `kernel/tests/derived.rs` dispatch it; group 0 is the harness's: selectors, the two variants' buffers, the words, the
// per-sample arguments, then the output.

@group(0) @binding(0) var<storage, read> t_sel: array<u32>;
@group(0) @binding(1) var<storage, read> t_ftle: array<SimStateFTLE>;
@group(0) @binding(2) var<storage, read> t_base: array<SimStateBase>;
@group(0) @binding(3) var<storage, read> t_word: array<vec4<u32>>;
@group(0) @binding(4) var<storage, read> t_args: array<vec4<u32>>;
@group(0) @binding(5) var<storage, read_write> t_out: array<u32>;

// Sample `i` through its variant's unpack. Its two argument rows: (flags, ensemble_spread bits, dt_macro bits,
// delta_0 bits) and (n_renorm, horizon_steps, 0, 0); flag bit 0 asks for the FTLE variant, which only a tier with
// `has_ftle` reads, bit 1 is `has_word`, bit 2 `has_ensemble`.
fn t_read(i: u32) -> SimState {
    let a = t_args[2u * i];
    let b = t_args[2u * i + 1u];
    let params = ReadParams(bitcast<f32>(a.z), bitcast<f32>(a.w), b.x, b.y);
    let has_word = (a.x & 2u) != 0u;
    let has_ensemble = (a.x & 4u) != 0u;
    let spread = bitcast<f32>(a.y);
    if (has_ftle && (a.x & 1u) != 0u) {
        return sim_state_from_ftle(t_ftle[i], t_word[i], has_word, spread, has_ensemble, params);
    }
    return sim_state_from_base(t_base[i], t_word[i], has_word, spread, has_ensemble, params);
}

fn t_bool(b: bool) -> u32 { return select(0u, 1u, b); }

// The member `m` of `s`: the selectors are `MEMBERS`' indices in the tests.
fn t_member(s: SimState, m: u32) -> u32 {
    switch m {
        case 0u: { return bitcast<u32>(s.ftle); }
        case 1u: { return t_bool(s.ftle_valid); }
        case 2u: { return bitcast<u32>(s.diffusion); }
        case 3u: { return t_bool(s.diffusion_slope_valid); }
        case 4u: { return s.total_substeps_log2; }
        case 5u: { return bitcast<u32>(s.t_end_fraction); }
        case 6u: { return bitcast<u32>(s.t_dmin_fraction); }
        case 7u: { return s.orbit_count; }
        case 8u: { return t_bool(s.retrograde); }
        case 9u: { return t_bool(s.is_resolved_outcome); }
        case 10u: { return t_bool(s.is_running); }
        case 11u: { return t_bool(s.is_failed); }
        case 12u: { return t_bool(s.is_finished); }
        case 13u: { return bitcast<u32>(s.ensemble_spread); }
        case 14u: { return s.word.x; }
        case 15u: { return s.word.y; }
        case 16u: { return s.word.z; }
        case 17u: { return s.word.w; }
        case 18u: { return s.total_substeps; }
        case 19u: { return s.t_end_step; }
        case 20u: { return s.state; }
        case 21u: { return bitcast<u32>(s.S); }
        case 22u: { return s.closure_step; }
        case 23u: { return bitcast<u32>(s.d_min); }
        default: { return 0xffffffffu; }
    }
}

@compute @workgroup_size(64)
fn t_derived(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&t_sel)) {
        let sel = t_sel[id.x];
        t_out[id.x] = t_member(t_read(sel >> 8u), sel & 0xffu);
    }
}
