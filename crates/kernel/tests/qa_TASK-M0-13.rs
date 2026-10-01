//! QA tests for TASK-M0-13, run on the GPU: the checked-in generated WGSL (`payload_unpack.wgsl`) reads synthetic,
//! hand-filled buffers with known values (debug_tooling_plan "Principle") and is compared with the stored Rust layout
//! and with the corpus's numbers, not with the implementation's.
//! - REQ-PAY-091 (payload §1; R-343): the bytes of a Rust `SimStateFTLE`, `SimStateBase` and `ICDescriptor` (the
//!   authoritative stored layout, lowering Part 2), read through the WGSL structs, give each named member's value; a
//!   second element shows the array stride; `closure_step_reserved` is `closure_step | _reserved << 16` and
//!   `closure_step(w)` returns the Rust `closure_step`, ignoring bits 16–31; `PAYLOAD_SCHEMA_VERSION`'s `.x`/`.y` are
//!   the Rust `u64`'s low/high 32 bits.
//! - REQ-RENDER-001 (render contract Part 5; payload §2, §3, §6; R-343, R-271): every u32 accessor zero-extends (the
//!   u32 overload of `extractBits`: the i32 one sign-extends) at payload §2's bit positions, including fields that
//!   reach bit 31; `pa_d_min_is_unset` is true exactly when bits 16–31 are 0x7c00 (f16 +inf), whatever the low half,
//!   and false for −inf, NaN, 65504 and the subnormal; it agrees with the Rust writer `set_d_min_unset`; the binding
//!   constants are R-343's 1/0 and 1/1 and the Rust ones.
//! - REQ-PAY-016 (payload §3; R-307, R-321, R-324): the WGSL table functions give payload §3's frozen cells for every
//!   symbol input (masked `& 3`) and digit (clamped `min(d, 2)`), and compose: `continuation_index(prev,
//!   continuation_symbol(prev, e)) == e` and `predecessor_symbol(continuation_symbol(prev, e), e) == prev`.
//!
//! Each test has a registered negative control (R-176): the generated text with one rule broken.

use std::path::Path;

use kernel::payload::{self, ICDescriptor, SimStateBase, SimStateFTLE};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn wgsl() -> String {
    let p =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../render/frag/generated/payload_unpack.wgsl");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn run(generated: &str, kernel: &str, entry: &str, inputs: &[&[u32]]) -> Vec<u32> {
    let gpu = GpuHarness::new().expect("a GPU device");
    gpu.run_wgsl(&format!("{generated}\n{kernel}"), entry, inputs)
}

/// A distinct, finite, normal f32 for slot `i` (never a subnormal, which a GPU may flush).
fn f(i: u32) -> f32 {
    let v = 1.25 + i as f32 * 0.75;
    if i.is_multiple_of(2) {
        v
    } else {
        -v
    }
}

/// A distinct u32 for slot `i` with bit 31 and bit 15 set, so a sign-extending read would show.
fn u(i: u32) -> u32 {
    0x8000_8000 | (i.wrapping_mul(0x0101_0101) & 0x7fff_7fff)
}

fn ftle(seed: u32) -> SimStateFTLE {
    let v2 = |k: u32| [[f(k), f(k + 1)], [f(k + 2), f(k + 3)], [f(k + 4), f(k + 5)]];
    SimStateFTLE {
        r: v2(seed),
        p: v2(seed + 6),
        r_sh: v2(seed + 12),
        p_sh: v2(seed + 18),
        S: f(seed + 24),
        theta: f(seed + 25),
        mean_y: f(seed + 26),
        C_ty: f(seed + 27),
        E_0: f(seed + 28),
        Lz_0: f(seed + 29),
        packed_a: u(seed + 30),
        packed_b: u(seed + 31),
        times: u(seed + 32),
        total_substeps: u(seed + 33),
        closure_min: f(seed + 34),
        closure_step: 0x8000 | (seed as u16 * 7 + 3),
        _reserved: 0xc000 | (seed as u16 * 5 + 1),
        // the declared tail padding at f32 is [u32; 0] (R-313, R-86)
        _tail: [],
    }
}

/// The member values the WGSL must read, by payload §1's member names, in the order the kernel below writes them.
fn ftle_expected(s: &SimStateFTLE, shadow: bool) -> Vec<u32> {
    let mut out = Vec::new();
    let mut pairs = vec![s.r, s.p];
    if shadow {
        pairs.extend([s.r_sh, s.p_sh]);
    }
    for a in pairs {
        for v in a {
            out.extend([v[0].to_bits(), v[1].to_bits()]);
        }
    }
    out.extend([s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0].map(f32::to_bits));
    out.extend([s.packed_a, s.packed_b, s.times, s.total_substeps]);
    out.push(s.closure_min.to_bits());
    out.push(s.closure_step as u32 | (s._reserved as u32) << 16);
    out.push(s.closure_step as u32);
    out
}

/// A kernel reading `ty` elements and writing, per element, each named member's bits, then `closure_step(…)`.
fn struct_kernel(ty: &str, shadow: bool) -> String {
    let mut reads = Vec::new();
    let mut groups = vec!["r", "p"];
    if shadow {
        groups.extend(["r_sh", "p_sh"]);
    }
    for g in groups {
        for j in 0..3 {
            reads.push(format!("bitcast<u32>(s.{g}[{j}].x)"));
            reads.push(format!("bitcast<u32>(s.{g}[{j}].y)"));
        }
    }
    for m in ["S", "theta", "mean_y", "C_ty", "E_0", "Lz_0"] {
        reads.push(format!("bitcast<u32>(s.{m})"));
    }
    for m in ["packed_a", "packed_b", "times", "total_substeps"] {
        reads.push(format!("s.{m}"));
    }
    reads.push("bitcast<u32>(s.closure_min)".into());
    reads.push("s.closure_step_reserved".into());
    reads.push("closure_step(s.closure_step_reserved)".into());
    let n = reads.len();
    let body: String = reads
        .iter()
        .enumerate()
        .map(|(i, r)| format!("    w[{i}] = {r};\n"))
        .collect();
    format!(
        "@group(0) @binding(0) var<storage, read> st: array<{ty}>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_struct(@builtin(global_invocation_id) id: vec3<u32>) {{
    let e = id.x / {n}u;
    let k = id.x % {n}u;
    if e >= arrayLength(&st) || id.x >= arrayLength(&out) {{ return; }}
    let s = st[e];
    var w: array<u32, {n}>;
{body}    out[id.x] = w[k];
}}
"
    )
}

fn ftle_words(s: &SimStateFTLE) -> [u32; 36] {
    // SimStateFTLE is repr(C), all members 4-byte or wider with no padding: its 144 B are 36 initialised words.
    unsafe { std::mem::transmute::<SimStateFTLE, [u32; 36]>(*s) }
}

fn check_ftle(generated: &str) {
    let states = [ftle(0), ftle(40)];
    let mut input: Vec<u32> = states.iter().flat_map(ftle_words).collect();
    // the kernel writes 37 words per element, one more than the 36 stored: pad the input so the output holds them.
    input.extend([0, 0]);
    let got = run(
        generated,
        &struct_kernel("SimStateFTLE", true),
        "t_struct",
        &[&input],
    );
    let want: Vec<u32> = states.iter().flat_map(|s| ftle_expected(s, true)).collect();
    assert_eq!(
        &got[..want.len()],
        &want[..],
        "the WGSL SimStateFTLE does not read the Rust bytes member by member"
    );
}

#[test]
fn qa_gpu_simstate_ftle_reads_the_rust_layout() {
    check_ftle(&wgsl());
}

negative_control!(
    qa_gpu_simstate_ftle_reads_the_rust_layout,
    "packed_a and packed_b exchanged in SimStateFTLE read the wrong bytes",
    expected = "the WGSL SimStateFTLE does not read the Rust bytes member by member",
    check_ftle(&wgsl().replacen(
        "    packed_a: u32,\n    packed_b: u32,",
        "    packed_b: u32,\n    packed_a: u32,",
        1
    ))
);

fn base(seed: u32) -> SimStateBase {
    let s = ftle(seed);
    SimStateBase {
        r: s.r,
        p: s.p,
        S: s.S,
        theta: s.theta,
        mean_y: s.mean_y,
        C_ty: s.C_ty,
        E_0: s.E_0,
        Lz_0: s.Lz_0,
        packed_a: s.packed_a,
        packed_b: s.packed_b,
        times: s.times,
        total_substeps: s.total_substeps,
        closure_min: s.closure_min,
        closure_step: s.closure_step,
        _reserved: s._reserved,
        _tail: [],
    }
}

fn check_base(generated: &str) {
    let states = [base(3), base(50)];
    let mut input: Vec<u32> = states
        .iter()
        .flat_map(|s| unsafe { std::mem::transmute::<SimStateBase, [u32; 24]>(*s) })
        .collect();
    input.extend([0, 0]);
    let got = run(
        generated,
        &struct_kernel("SimStateBase", false),
        "t_struct",
        &[&input],
    );
    let want: Vec<u32> = [ftle(3), ftle(50)]
        .iter()
        .flat_map(|s| ftle_expected(s, false))
        .collect();
    assert_eq!(
        &got[..want.len()],
        &want[..],
        "the WGSL SimStateBase does not read the Rust bytes member by member"
    );
}

#[test]
fn qa_gpu_simstate_base_reads_the_rust_layout() {
    check_base(&wgsl());
}

negative_control!(
    qa_gpu_simstate_base_reads_the_rust_layout,
    "a SimStateBase whose closure_step_reserved precedes closure_min reads byte 88 for it",
    expected = "the WGSL SimStateBase does not read the Rust bytes member by member",
    {
        let g = wgsl();
        let at = g.find("struct SimStateBase").expect("SimStateBase");
        let (head, tail) = g.split_at(at);
        let tail = tail.replacen(
            "    closure_min: f32,\n    closure_step_reserved: u32,",
            "    closure_step_reserved: u32,\n    closure_min: f32,",
            1,
        );
        check_base(&format!("{head}{tail}"));
    }
);

fn check_ic(generated: &str) {
    let names = [
        "m0",
        "m1",
        "m2",
        "q_mass",
        "rho_mag",
        "lambda_mag",
        "rho_ratio",
        "rho_angle",
        "K_0",
        "V_0",
        "virial_ratio",
        "r_min_pair_0",
    ];
    let mk = |k: u32| ICDescriptor {
        m0: f(k),
        m1: f(k + 1),
        m2: f(k + 2),
        q_mass: f(k + 3),
        rho_mag: f(k + 4),
        lambda_mag: f(k + 5),
        rho_ratio: f(k + 6),
        rho_angle: f(k + 7),
        K_0: f(k + 8),
        V_0: f(k + 9),
        virial_ratio: f(k + 10),
        r_min_pair_0: f(k + 11),
        _pad: [u(k), u(k + 1), u(k + 2), u(k + 3)],
        _tail: [],
    };
    let ds = [mk(0), mk(20)];
    let input: Vec<u32> = ds
        .iter()
        .flat_map(|d| unsafe { std::mem::transmute::<ICDescriptor, [u32; 16]>(*d) })
        .collect();
    let body: String = names
        .iter()
        .enumerate()
        .map(|(i, n)| format!("    w[{i}] = bitcast<u32>(s.{n});\n"))
        .collect();
    let kernel = format!(
        "@group(0) @binding(0) var<storage, read> st: array<ICDescriptor>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_ic(@builtin(global_invocation_id) id: vec3<u32>) {{
    let e = id.x / 12u;
    let k = id.x % 12u;
    if e >= arrayLength(&st) {{ return; }}
    let s = st[e];
    var w: array<u32, 12>;
{body}    out[id.x] = w[k];
}}
"
    );
    let got = run(generated, &kernel, "t_ic", &[&input]);
    let want: Vec<u32> = ds
        .iter()
        .flat_map(|d| {
            [
                d.m0,
                d.m1,
                d.m2,
                d.q_mass,
                d.rho_mag,
                d.lambda_mag,
                d.rho_ratio,
                d.rho_angle,
                d.K_0,
                d.V_0,
                d.virial_ratio,
                d.r_min_pair_0,
            ]
            .map(f32::to_bits)
        })
        .collect();
    assert_eq!(
        &got[..24],
        &want[..],
        "the WGSL ICDescriptor does not read the Rust bytes member by member"
    );
}

#[test]
fn qa_gpu_icdescriptor_reads_the_rust_layout() {
    check_ic(&wgsl());
}

negative_control!(
    qa_gpu_icdescriptor_reads_the_rust_layout,
    "an ICDescriptor with 3 words of padding has a 60 B stride and reads the second element wrong",
    expected = "the WGSL ICDescriptor does not read the Rust bytes member by member",
    check_ic(&wgsl().replacen("_pad: array<u32, 4>,", "_pad: array<u32, 3>,", 1))
);

// ---------------------------------------------------------------------------------------------------------------
// closure_step(w): bits 0–15, bits 16–31 ignored (R-343).

fn check_closure_step(generated: &str) {
    let mut ws: Vec<u32> = Vec::new();
    for hi in [0u32, 0x0001, 0x8000, 0xffff, 0x1234] {
        for lo in [0u32, 1, 0x7fff, 0x8000, 0xffff, 0xabcd] {
            ws.push(hi << 16 | lo);
        }
    }
    let kernel = "@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_cs(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&a) { out[id.x] = closure_step(a[id.x]); }
}
";
    let got = run(generated, kernel, "t_cs", &[&ws]);
    for (w, g) in ws.iter().zip(&got) {
        assert_eq!(
            *g,
            w & 0xffff,
            "closure_step({w:#010x}) is {g:#x}, not its low 16 bits"
        );
    }
}

#[test]
fn qa_gpu_closure_step_is_the_low_16_bits() {
    check_closure_step(&wgsl());
}

negative_control!(
    qa_gpu_closure_step_is_the_low_16_bits,
    "a closure_step reading the high half must fail",
    expected = "not its low 16 bits",
    check_closure_step(&wgsl().replace(
        "fn closure_step(w: u32) -> u32 { return extractBits(w, 0u, 16u); }",
        "fn closure_step(w: u32) -> u32 { return extractBits(w, 16u, 16u); }"
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// The u32 accessors at payload §2/§3's bit positions, zero-extended (REQ-RENDER-001's u32 extractBits).

/// Payload §2/§3/§6, by bit position: (accessor call on `x`, expected from `x`).
type Expect = fn(u32) -> u32;
const ACCESSORS: [(&str, Expect); 18] = [
    ("sd_state(x)", |x| x & 0b111),
    ("sd_detail(x)", |x| (x >> 3) & 0b11),
    ("select(0u, 1u, sd_saturated(x))", |x| (x >> 5) & 1),
    ("sd_dmin_pair(x)", |x| (x >> 6) & 0b11),
    ("sd_last_symbol(x)", |x| (x >> 8) & 0b11),
    ("tm_t_end_step(x)", |x| x & 0xffff),
    ("tm_t_dmin_step(x)", |x| x >> 16),
    (
        "fgw_length_raw(vec4<u32>(0xffffffffu, 0xffffffffu, 0xffffffffu, x))",
        |x| x >> 25,
    ),
    (
        "select(0u, 1u, fgw_truncated(vec4<u32>(0u, 0u, 0u, x)))",
        |x| (x >> 25 == 127) as u32,
    ),
    (
        "select(0u, 1u, fgw_reduced_length_valid(vec4<u32>(0u, 0u, 0u, x)))",
        |x| (x >> 25 != 127) as u32,
    ),
    (
        "fgw_retained_prefix_length(vec4<u32>(0u, 0u, 0u, x))",
        |x| if x >> 25 == 127 { 76 } else { x >> 25 },
    ),
    // payload §2: 0–2 resolved outcomes, 3 running, 4–5 failed, 6–7 reserved = finished and untrusted.
    ("select(0u, 1u, sd_is_resolved_outcome(x))", |x| {
        (x & 7 <= 2) as u32
    }),
    ("select(0u, 1u, sd_is_running(x))", |x| (x & 7 == 3) as u32),
    ("select(0u, 1u, sd_is_failed(x))", |x| (x & 7 >= 4) as u32),
    ("select(0u, 1u, sd_is_finished(x))", |x| (x & 7 != 3) as u32),
    ("select(0u, 1u, sd_last_symbol_valid(x))", |x| {
        (x >= 1 && x != 127) as u32
    }),
    // floor(log2 total), 0 for a total ≤ 1 (render contract Part 5, R-86).
    ("total_substeps_log2(x)", |x| {
        if x <= 1 {
            0
        } else {
            31 - x.leading_zeros()
        }
    }),
    ("select(0u, 1u, pa_d_min_is_unset(x))", |x| {
        (x >> 16 == 0x7c00) as u32
    }),
];

fn words() -> Vec<u32> {
    let mut v: Vec<u32> = (0..256).collect();
    for b in 0..32 {
        v.extend([1u32 << b, !(1u32 << b), (1u32 << b).wrapping_sub(1)]);
    }
    for k in 0..128u32 {
        v.push(k << 25 | 0x00ff_ffff);
    }
    for h in [
        0x7c00u32, 0xfc00, 0x7c01, 0x7e00, 0xfe00, 0x7bff, 0x0001, 0x0000, 0x3c00, 0xffff,
    ] {
        for l in [0u32, 0x7c00, 0xffff, 0x03ff] {
            v.push(h << 16 | l);
        }
    }
    // a deterministic spread over the whole range
    let mut x = 0x9e37_79b9u32;
    for _ in 0..512 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        v.push(x);
    }
    v
}

fn check_accessors(generated: &str) {
    let xs = words();
    for (i, (call, expect)) in ACCESSORS.iter().enumerate() {
        let kernel = format!(
            "@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_acc_{i}(@builtin(global_invocation_id) id: vec3<u32>) {{
    if id.x < arrayLength(&a) {{ let x = a[id.x]; out[id.x] = {call}; }}
}}
"
        );
        let got = run(generated, &kernel, &format!("t_acc_{i}"), &[&xs]);
        for (x, g) in xs.iter().zip(&got) {
            assert_eq!(
                *g,
                expect(*x),
                "WGSL `{call}` at x = {x:#010x} is {g:#x}, not {:#x}",
                expect(*x)
            );
        }
    }
}

#[test]
fn qa_gpu_accessors_zero_extend_at_payload_bit_positions() {
    check_accessors(&wgsl());
}

negative_control!(
    qa_gpu_accessors_zero_extend_at_payload_bit_positions,
    "tm_t_dmin_step on the i32 overload sign-extends bit 31 and must fail",
    expected = "WGSL `tm_t_dmin_step(x)` at x = 0xfffffffe is 0xffffffff, not 0xffff",
    check_accessors(&wgsl().replace(
        "fn tm_t_dmin_step(w: u32) -> u32 { return extractBits(w, 16u, 16u); }",
        "fn tm_t_dmin_step(w: u32) -> u32 { return bitcast<u32>(extractBits(bitcast<i32>(w), 16u, 16u)); }"
    ))
);

/// `pa_d_min_is_unset` agrees with the Rust writer: every word `set_d_min_unset` writes reads unset; the same words
/// with any other high half, −inf's and NaN's included, do not.
fn check_unset_vs_writer(generated: &str) {
    let base = words();
    let mut xs: Vec<u32> = base.iter().map(|&w| payload::set_d_min_unset(w)).collect();
    let others: Vec<u32> = base
        .iter()
        .flat_map(|&w| [0xfc00u32, 0x7e00, 0x7c01, 0x7bff, 0x0001].map(|h| (w & 0xffff) | h << 16))
        .collect();
    let n = xs.len();
    xs.extend(&others);
    let kernel = "@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_unset(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&a) { out[id.x] = select(0u, 1u, pa_d_min_is_unset(a[id.x])); }
}
";
    let got = run(generated, kernel, "t_unset", &[&xs]);
    for (i, (x, g)) in xs.iter().zip(&got).enumerate() {
        let want = (i < n) as u32;
        assert_eq!(
            *g, want,
            "WGSL pa_d_min_is_unset({x:#010x}) is {g}, not {want}"
        );
        assert_eq!(
            payload::pa_d_min_is_unset(*x) as u32,
            want,
            "Rust pa_d_min_is_unset({x:#010x})"
        );
    }
}

#[test]
fn qa_gpu_pa_d_min_is_unset_agrees_with_the_rust_writer() {
    check_unset_vs_writer(&wgsl());
}

negative_control!(
    qa_gpu_pa_d_min_is_unset_agrees_with_the_rust_writer,
    "an unset test that ignores the sign bit reads f16 -inf as unset",
    expected = "is 1, not 0",
    check_unset_vs_writer(&wgsl().replace(
        "return extractBits(w, 16u, 16u) == PA_D_MIN_UNSET;",
        "return extractBits(w, 16u, 15u) == PA_D_MIN_UNSET;"
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// Schema version halves and binding constants, read on the GPU (REQ-PAY-091, REQ-RENDER-001; R-343).

fn check_constants(generated: &str) {
    let kernel = "@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_consts(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x != 0u { return; }
    out[0] = PAYLOAD_SCHEMA_VERSION.x;
    out[1] = PAYLOAD_SCHEMA_VERSION.y;
    out[2] = SIMSTATE_GROUP;
    out[3] = SIMSTATE_BINDING;
    out[4] = WORD_GROUP;
    out[5] = WORD_BINDING;
    out[6] = PA_D_MIN_UNSET + 0u * a[0];
}
";
    let got = run(generated, kernel, "t_consts", &[&[0; 7]]);
    let v = payload::PAYLOAD_SCHEMA_VERSION;
    assert_eq!(
        [got[0], got[1]],
        [v as u32, (v >> 32) as u32],
        "PAYLOAD_SCHEMA_VERSION is not (.x low, .y high) of the Rust u64"
    );
    let rust = [
        payload::SIMSTATE_GROUP,
        payload::SIMSTATE_BINDING,
        payload::WORD_GROUP,
        payload::WORD_BINDING,
    ];
    assert_eq!(
        &got[2..6],
        &rust,
        "the WGSL binding constants are not the Rust ones"
    );
    assert_eq!(
        &got[2..6],
        &[1, 0, 1, 1],
        "the binding constants are not R-343's"
    );
    assert_eq!(
        got[6],
        payload::PA_D_MIN_UNSET,
        "PA_D_MIN_UNSET differs from the Rust one"
    );
}

#[test]
fn qa_gpu_schema_version_and_binding_constants() {
    check_constants(&wgsl());
}

negative_control!(
    qa_gpu_schema_version_and_binding_constants,
    "the version's halves exchanged must fail",
    expected = "PAYLOAD_SCHEMA_VERSION is not (.x low, .y high) of the Rust u64",
    {
        let v = payload::PAYLOAD_SCHEMA_VERSION;
        let right = format!(
            "vec2<u32>({:#010x}u, {:#010x}u)",
            v as u32,
            (v >> 32) as u32
        );
        let swapped = format!(
            "vec2<u32>({:#010x}u, {:#010x}u)",
            (v >> 32) as u32,
            v as u32
        );
        let g = wgsl();
        assert!(
            g.contains(&right),
            "the control's pattern `{right}` is not in the WGSL"
        );
        check_constants(&g.replace(&right, &swapped));
    }
);

// ---------------------------------------------------------------------------------------------------------------
// The continuation tables (REQ-PAY-016): payload §3's frozen cells, masked and clamped, and their composition.

const INV: [u32; 4] = [1, 0, 3, 2];
const CONT: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
const CI: [[u32; 4]; 4] = [[0, 3, 1, 2], [3, 0, 2, 1], [1, 2, 0, 3], [2, 1, 3, 0]];

fn check_tables(generated: &str) {
    let ins: Vec<u32> = (0..12)
        .chain([255, 0x8000_0003, u32::MAX - 2, u32::MAX])
        .collect();
    let (a, b): (Vec<u32>, Vec<u32>) = ins
        .iter()
        .flat_map(|&x| ins.iter().map(move |&y| (x, y)))
        .unzip();
    let kernel = "@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read> b: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_tab(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&a) { return; }
    let x = a[id.x];
    let y = b[id.x];
    let n = continuation_symbol(x, y);
    out[id.x] = inverse(x)
        | (n << 4u)
        | (predecessor_symbol(x, y) << 8u)
        | (continuation_index(x, y) << 12u)
        | (continuation_index(x, n) << 16u)
        | (predecessor_symbol(n, y) << 20u);
}
";
    let got = run(generated, kernel, "t_tab", &[&a, &b]);
    for i in 0..a.len() {
        let (x, y) = (a[i], b[i]);
        let (s, d) = ((x & 3) as usize, y.min(2) as usize);
        let n = CONT[d][s];
        let want = INV[s]
            | n << 4
            | CONT[d][s] << 8
            | CI[s][(y & 3) as usize] << 12
            // the composition: the index of the continuation is the (clamped) digit, its predecessor the symbol
            | (d as u32) << 16
            | (s as u32) << 20;
        assert_eq!(
            got[i], want,
            "WGSL tables at ({x}, {y}) give {:#08x}, not payload §3's {want:#08x}",
            got[i]
        );
    }
}

#[test]
fn qa_gpu_continuation_tables_are_payload_section_3() {
    check_tables(&wgsl());
}

negative_control!(
    qa_gpu_continuation_tables_are_payload_section_3,
    "a digit wrapped mod 3, not clamped to 2, reads digit 0's row at 3",
    expected = "WGSL tables at (0, 3)",
    check_tables(&wgsl().replace(
        "fn continuation_symbol(prev: u32, e: u32) -> u32 { return CONT_SYMBOL[min(e, 2u)][prev & 3u]; }",
        "fn continuation_symbol(prev: u32, e: u32) -> u32 { return CONT_SYMBOL[e % 3u][prev & 3u]; }"
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// f16 pairs read through core unpack2x16float (REQ-RENDER-001): on a device opened with no features (so no
// shader-f16), `pa_d_min` reads the high half, `pb_dE_max` the low and `pb_dLz_max` the high, as binary16 values.

/// IEEE 754 binary16 `h` as f32, for normal and zero halves (sign, 5-bit exponent biased 15, 10-bit mantissa).
fn half(h: u32) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = (h >> 10) & 0x1f;
    let m = (h & 0x3ff) as f32;
    if e == 0 {
        assert_eq!(h & 0x7fff, 0, "only zero and normal halves are used here");
        return sign * 0.0;
    }
    sign * (1.0 + m / 1024.0) * 2f32.powi(e as i32 - 15)
}

fn check_f16_pairs(generated: &str) {
    let halves = [
        0x0000u32, 0x3c00, 0xc000, 0x7bff, 0xfbff, 0x0400, 0x3555, 0x5640, 0x8000, 0x2e66,
    ];
    let mut xs = Vec::new();
    for &hi in &halves {
        for &lo in &halves {
            xs.push(hi << 16 | lo);
        }
    }
    for (call, pick) in [("pa_d_min", 16u32), ("pb_dE_max", 0), ("pb_dLz_max", 16)] {
        let kernel = format!(
            "@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn t_{call}(@builtin(global_invocation_id) id: vec3<u32>) {{
    if id.x < arrayLength(&a) {{ out[id.x] = bitcast<u32>({call}(a[id.x])); }}
}}
"
        );
        let got = run(generated, &kernel, &format!("t_{call}"), &[&xs]);
        for (x, g) in xs.iter().zip(&got) {
            let want = half((x >> pick) & 0xffff);
            assert_eq!(
                f32::from_bits(*g),
                want,
                "WGSL {call}({x:#010x}) is {}, not {want}",
                f32::from_bits(*g)
            );
        }
    }
}

#[test]
fn qa_gpu_f16_pairs_read_through_unpack2x16float() {
    check_f16_pairs(&wgsl());
}

negative_control!(
    qa_gpu_f16_pairs_read_through_unpack2x16float,
    "pb_dE_max reading the high half must fail",
    expected = "WGSL pb_dE_max(",
    check_f16_pairs(&wgsl().replace(
        "fn pb_dE_max(w: u32) -> f32 { return unpack2x16float(w).x; }",
        "fn pb_dE_max(w: u32) -> f32 { return unpack2x16float(w).y; }"
    ))
);
