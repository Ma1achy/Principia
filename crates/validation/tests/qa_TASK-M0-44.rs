//! QA tests for TASK-M0-44's fast-math probe, written from REQ-SYS-074 ("compute shaders must compile under an
//! explicit fast-math setting ... off by default ... on Metal, off must compile the project's own MSL with fast-math off
//! and load it through wgpu's passthrough"; verify: with the setting off, `(x + 0.5) / 255.0` for every x in 0..256 and
//! a fuzzed set of f32 divisions match the CPU's correctly rounded f32 division bit for bit on every backend; on Metal
//! with the setting on, at least one input differs, R-296's Result's 94 columns), from REQ-TOOL-141 (the session header
//! records the setting asked for and each stage's mode as compiled; on lavapipe the setting on compiles as off), and
//! from R-297's Applied note (Vulkan's own path compiles without fast-math and offers no switch). The expectations per
//! backend are written here from those sources, not read from the entry point's own table. The probe kernels are
//! written here, not taken from the harness. Each test registers its negative control (R-176).
//!
//! The probe is stateless (pitfalls §10): it certifies single divisions given identical inputs, not a trajectory.
// The file name `qa_TASK-M0-44` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::fast_math::{CompiledModes, FastMath, StageMode};
use engine::telemetry::session;
use serde_json::Map;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// `(x + 0.5) / 255.0` for each word x, the arithmetic R-296's Result measured, its bits written out.
const QA_HALFWAY: &str = r"
@group(0) @binding(0) var<storage, read> xs: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn qa_halfway(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&xs)) {
        let v = (f32(xs[i]) + 0.5) / 255.0;
        out[i] = bitcast<u32>(v);
    }
}
";

/// `n / d` for the bits of each pair of words.
const QA_DIVIDE: &str = r"
@group(0) @binding(0) var<storage, read> ns: array<u32>;
@group(0) @binding(1) var<storage, read> ds: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn qa_divide(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&ns)) {
        out[i] = bitcast<u32>(bitcast<f32>(ns[i]) / bitcast<f32>(ds[i]));
    }
}
";

/// A reference division: the CPU's correctly rounded one, or a control's stand-in.
type Reference = fn(f32, f32) -> f32;

fn correctly_rounded(n: f32, d: f32) -> f32 {
    n / d
}

/// Multiplication by the divisor's f32 reciprocal: R-296's Result's fast-math arithmetic, one ulp off on 94 of the 256
/// halfway columns.
fn reciprocal(n: f32, d: f32) -> f32 {
    n * (1.0 / d)
}

fn harness() -> GpuHarness {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{}", h.adapter_info());
    h
}

fn is_metal(h: &GpuHarness) -> bool {
    h.adapter_info().backend == wgpu::Backend::Metal
}

/// The halfway inputs, x in 0..256.
fn halfway_inputs() -> Vec<u32> {
    (0..256).collect()
}

/// The indices at which `got` differs, bit for bit, from `reference` on the halfway inputs.
fn halfway_mismatches(got: &[u32], reference: Reference) -> Vec<u32> {
    assert_eq!(
        got.len(),
        256,
        "the halfway probe read back {} words",
        got.len()
    );
    (0..256u32)
        .filter(|&x| got[x as usize] != reference(x as f32 + 0.5, 255.0).to_bits())
        .collect()
}

/// A fuzzed set of f32 divisions, as the bits of dividends and divisors. Two parts:
/// - every quotient `(k + 0.5) / d` and `k / d` for k in 0..512 and odd d in 1..=255: small-integer divisions, the shape
///   R-296 measured, whose quotients' rounding a reciprocal multiply gets wrong on many columns;
/// - 2^15 pairs of normal operands of either sign, random significands, exponents within 2^±50 (xorshift64*, a fixed
///   seed), so every quotient is a normal f32 and subnormal handling stays out of the comparison.
fn fuzzed() -> (Vec<u32>, Vec<u32>) {
    let mut ns = Vec::new();
    let mut ds = Vec::new();
    for k in 0..512u32 {
        for d in (1..=255u32).step_by(2) {
            for n in [k as f32 + 0.5, k as f32] {
                ns.push(n.to_bits());
                ds.push((d as f32).to_bits());
            }
        }
    }
    let mut s = 0x0A44_D1F1_DE5E_ED01_u64;
    let mut next = move || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    let mut operand = move || {
        let r = next();
        let sign = ((r >> 63) as u32) << 31;
        let exp = (127 - 50 + ((r >> 40) % 101) as u32) << 23;
        sign | exp | (r as u32 & 0x007F_FFFF)
    };
    for _ in 0..(1 << 15) {
        ns.push(operand());
        ds.push(operand());
    }
    (ns, ds)
}

/// The indices at which `got` differs, bit for bit, from `reference` on the fuzzed divisions.
fn fuzzed_mismatches(ns: &[u32], ds: &[u32], got: &[u32], reference: Reference) -> Vec<usize> {
    assert_eq!(
        got.len(),
        ns.len(),
        "the fuzzed probe read back {} words",
        got.len()
    );
    (0..ns.len())
        .filter(|&i| got[i] != reference(f32::from_bits(ns[i]), f32::from_bits(ds[i])).to_bits())
        .collect()
}

/// The halfway probe under `setting`: the compute stage's mode as compiled, and the mismatches against `reference`.
fn halfway(h: &GpuHarness, setting: FastMath, reference: Reference) -> (StageMode, Vec<u32>) {
    let xs = halfway_inputs();
    let prepared = h.prepare_with(QA_HALFWAY, "qa_halfway", &[&xs], setting);
    let mode = prepared.mode();
    (mode, halfway_mismatches(&prepared.run(), reference))
}

// ------------------------------------------------------------------------------------------------------------------
// REQ-SYS-074: with the setting off, the probe divides as the CPU does, on every backend
// ------------------------------------------------------------------------------------------------------------------

/// With the setting off, all 256 halfway quotients are the CPU's correctly rounded ones, bit for bit.
fn check_off_halfway(h: &GpuHarness, reference: Reference) {
    let (_, bad) = halfway(h, FastMath::Off, reference);
    eprintln!(
        "qa compute_fast_math: {:?}, setting off: {} of 256 (x + 0.5) / 255.0 differ from the CPU's",
        h.adapter_info().backend,
        bad.len()
    );
    assert!(
        bad.is_empty(),
        "with compute fast-math off, {} of 256 halfway quotients differ from the CPU's correctly rounded division \
         (x = {bad:?})",
        bad.len()
    );
}

#[test]
fn qa_compute_fast_math_off_halfway_exact() {
    check_off_halfway(&harness(), correctly_rounded);
}

negative_control!(
    qa_compute_fast_math_off_halfway_exact,
    "the halfway quotients compared with multiplication by f32(1/255), R-296's 94 columns",
    expected = "with compute fast-math off,",
    check_off_halfway(&harness(), reciprocal)
);

/// With the setting off, every fuzzed division is the CPU's correctly rounded one, bit for bit.
fn check_off_fuzzed(h: &GpuHarness, reference: Reference) {
    let (ns, ds) = fuzzed();
    let got = h.run_wgsl_with(QA_DIVIDE, "qa_divide", &[&ns, &ds], FastMath::Off);
    let bad = fuzzed_mismatches(&ns, &ds, &got, reference);
    eprintln!(
        "qa compute_fast_math: {:?}, setting off: {} of {} fuzzed divisions differ from the CPU's",
        h.adapter_info().backend,
        bad.len(),
        ns.len()
    );
    let first: Vec<_> = bad
        .iter()
        .take(8)
        .map(|&i| {
            (
                f32::from_bits(ns[i]),
                f32::from_bits(ds[i]),
                f32::from_bits(got[i]),
            )
        })
        .collect();
    assert!(
        bad.is_empty(),
        "with compute fast-math off, {} of {} fuzzed divisions differ from the CPU's correctly rounded division \
         (first: {first:?})",
        bad.len(),
        ns.len()
    );
}

#[test]
fn qa_compute_fast_math_off_fuzzed_exact() {
    check_off_fuzzed(&harness(), correctly_rounded);
}

negative_control!(
    qa_compute_fast_math_off_fuzzed_exact,
    "the fuzzed divisions compared with multiplication by the divisor's f32 reciprocal",
    expected = "with compute fast-math off,",
    check_off_fuzzed(&harness(), reciprocal)
);

/// The fuzzed set's own shape: its size, every operand and every CPU quotient a normal f32 (or zero for k = 0), and the
/// reciprocal multiply differing from correct rounding on some of it, so the set can tell the two apart.
fn check_fuzzed_set(ns: &[u32], ds: &[u32]) {
    assert_eq!(
        (ns.len(), ds.len()),
        (512 * 128 * 2 + (1 << 15), 512 * 128 * 2 + (1 << 15)),
        "the fuzzed set is not the stated size"
    );
    for (&n, &d) in ns.iter().zip(ds) {
        let q = f32::from_bits(n) / f32::from_bits(d);
        assert!(
            f32::from_bits(d).is_normal() && (q.is_normal() || q == 0.0),
            "the fuzzed set holds a division outside the normal range: {} / {}",
            f32::from_bits(n),
            f32::from_bits(d)
        );
    }
    let separating = ns
        .iter()
        .zip(ds)
        .filter(|(&n, &d)| {
            let (n, d) = (f32::from_bits(n), f32::from_bits(d));
            (n / d).to_bits() != reciprocal(n, d).to_bits()
        })
        .count();
    assert!(
        separating > 0,
        "the fuzzed set does not tell a reciprocal multiply from correct rounding"
    );
}

#[test]
fn qa_compute_fast_math_fuzzed_set_shape() {
    let (ns, ds) = fuzzed();
    check_fuzzed_set(&ns, &ds);
}

negative_control!(
    qa_compute_fast_math_fuzzed_set_shape,
    "a set with a subnormal divisor",
    expected = "the fuzzed set holds a division outside the normal range",
    {
        let (ns, mut ds) = fuzzed();
        ds[7] = 1;
        check_fuzzed_set(&ns, &ds);
    }
);

// ------------------------------------------------------------------------------------------------------------------
// REQ-SYS-074: the switch acts on Metal; on Vulkan, on compiles as off (R-297's Applied note)
// ------------------------------------------------------------------------------------------------------------------

/// Under `setting`, the compute stage compiles as `mode` and the halfway probe differs from the CPU's correctly rounded
/// division on at least one input when `differs`, on none otherwise.
fn check_acts(h: &GpuHarness, setting: FastMath, mode: StageMode, differs: bool) {
    let (got, bad) = halfway(h, setting, correctly_rounded);
    eprintln!(
        "qa compute_fast_math: {:?}, setting {setting:?}, compute compiled {got:?}: {} of 256 halfway quotients \
         differ from the CPU's",
        h.adapter_info().backend,
        bad.len()
    );
    assert_eq!(
        (got, !bad.is_empty()),
        (mode, differs),
        "the switch does not act as R-297 states: setting {setting:?} compiled {got:?} with {} of 256 differing",
        bad.len()
    );
}

/// What R-297 states each setting compiles to on this backend: on Metal, off is fast-math off and on is fast-math on,
/// which differs from correct rounding (R-296's Result); on Vulkan, both compile off and match it.
fn check_switch_per_backend(h: &GpuHarness, flip: bool) {
    let metal = is_metal(h);
    for setting in [FastMath::Off, FastMath::On] {
        let on = metal && setting == FastMath::On;
        let on = on != flip;
        let mode = if on { StageMode::On } else { StageMode::Off };
        check_acts(h, setting, mode, on);
    }
}

#[test]
fn qa_compute_fast_math_switch_per_backend() {
    check_switch_per_backend(&harness(), false);
}

negative_control!(
    qa_compute_fast_math_switch_per_backend,
    "each setting required to compile to, and divide as, the other mode",
    expected = "the switch does not act as R-297 states",
    check_switch_per_backend(&harness(), true)
);

/// The default setting is off, and the harness's default dispatch divides exactly as setting off does: on Metal, where
/// on differs, it would show (REQ-SYS-074, "off by default").
fn check_default(h: &GpuHarness, default: FastMath) {
    assert_eq!(
        default,
        FastMath::Off,
        "the compute fast-math default is not off"
    );
    let xs = halfway_inputs();
    let by_default = h.run_wgsl(QA_HALFWAY, "qa_halfway", &[&xs]);
    let off = h.run_wgsl_with(QA_HALFWAY, "qa_halfway", &[&xs], FastMath::Off);
    assert_eq!(
        by_default, off,
        "the compute fast-math default is not off: the default dispatch divides otherwise"
    );
    assert!(
        halfway_mismatches(&by_default, correctly_rounded).is_empty(),
        "the compute fast-math default is not off: the default dispatch is not correctly rounded"
    );
}

#[test]
fn qa_compute_fast_math_default_off() {
    check_default(&harness(), FastMath::default());
}

negative_control!(
    qa_compute_fast_math_default_off,
    "the setting on taken as the default",
    expected = "the compute fast-math default is not off",
    check_default(&harness(), FastMath::On)
);

// ------------------------------------------------------------------------------------------------------------------
// REQ-TOOL-141 at the seam with the entry point: the header records what was compiled, and the probe shows it
// ------------------------------------------------------------------------------------------------------------------

/// The modes R-297 states for this backend: on Metal, compute as the setting asks and the display stages on, wgpu's
/// default compile; on Vulkan, every stage off.
fn stated(metal: bool, setting: FastMath) -> CompiledModes {
    if metal {
        CompiledModes {
            compute: if setting == FastMath::On {
                StageMode::On
            } else {
                StageMode::Off
            },
            vertex: StageMode::On,
            fragment: StageMode::On,
        }
    } else {
        CompiledModes {
            compute: StageMode::Off,
            vertex: StageMode::Off,
            fragment: StageMode::Off,
        }
    }
}

/// For each setting, the header written for the harness's own adapter records the setting asked for and the modes
/// R-297 states; its compute mode is the dispatch's, and it reads on exactly when the probe differs from correct
/// rounding. `adapter` is the session adapter the header is written for.
fn check_header_is_the_dispatch(h: &GpuHarness, adapter: Option<&session::Adapter>) {
    let host = session::Host {
        cpu: "qa".to_owned(),
        cpu_cores_available: 1,
        cpu_cores_total: None,
        ram_bytes: Some(1 << 30),
    };
    for setting in [FastMath::Off, FastMath::On] {
        let header = session::header_with_fast_math(
            adapter,
            &host,
            session::build("qa", "dev", ""),
            Map::new(),
            setting,
        )
        .expect("no header");
        let (mode, bad) = halfway(h, setting, correctly_rounded);
        let want = stated(is_metal(h), setting);
        assert_eq!(
            (header.fast_math.setting, header.fast_math.compiled),
            (setting, Some(want)),
            "the header does not record the dispatch as compiled"
        );
        assert_eq!(
            (mode, mode == StageMode::On),
            (want.compute, !bad.is_empty()),
            "the header does not record the dispatch as compiled: the probe ran {mode:?} with {} of 256 differing",
            bad.len()
        );
    }
}

#[test]
fn qa_compute_fast_math_header_is_the_dispatch() {
    let h = harness();
    let adapter = h.session_adapter().expect("no session adapter").clone();
    check_header_is_the_dispatch(&h, Some(&adapter));
}

negative_control!(
    qa_compute_fast_math_header_is_the_dispatch,
    "the header written as if no GPU were opened, so it records nothing compiled",
    expected = "the header does not record the dispatch as compiled",
    check_header_is_the_dispatch(&harness(), None)
);
