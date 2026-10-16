//! The invariant-momentum construction's branches on the GPU, feasibility and seed (`kernel::decode::invariant`; TASK-M2-09): the
//! kernel's f32 build, `target/spirv/kernel.wgsl`, its entry points `decode_invariant_feasibility` and
//! `decode_invariant_construct_defaults` dispatched natively, read back and checked against the same source on the CPU
//! at f32 and at f64 and against an exact integer reference. CI runs this file in the `gpu-kernel` job (lavapipe) and on
//! Metal.
//! - `invariant_infeasible_branch`: `2·I·K* < L_z²` takes one branch on GPU-f32, CPU-f32 and CPU-f64 for identical
//!   inputs, and it is the exact one: triples on both sides of the boundary at every scale, subnormals included; sparse
//!   triples whose difference lands in each limb column; those scaled by powers of two from `2⁻⁶³` to `2⁶⁰` and by
//!   `1e-18` to `1e18`; and the tiny triples, where the limb products of operands not normalised to `[1, 2)` would be
//!   subnormal, which a GPU that flushes subnormals drops (REQ-DEC-022; dd_decoder §5 test 12).
//! - `invariant_infeasible_construct`: the decoder's construction on the GPU, over decoded configurations and
//!   `(L_z, K*)` pairs on both sides of `K* = L_z²/2`, the boundary itself included: tagged infeasible exactly where the
//!   CPU at f32 and f64 tags it, which is exactly `K* < L_z²/2`; every momentum finite, none NaN, none dropped
//!   (REQ-DEC-022).
//! - `invariant_seed_parity`: the decoder's seed on the GPU is R-82's decided on `z_α`, CPU-f64's, at `z_α = 0` and
//!   either side of it to where `cos²α` and `sin²α` are equal to rounding, with unequal masses (PIT-9), and the momenta
//!   agree to f32's rounding (REQ-DEC-021; dd_decoder §5 test 12).
//!
//! **Domain** (pitfalls §10): the construction alone, its inputs held fixed; no trajectory. **PIT-10:** the claim is for
//! identical inputs; the GPU's continuous values (`a`, `w`, `p`) may differ from the CPU's in their last bits.

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use kernel::decode::invariant::{self, construct, infeasible, ExactSign, Outcome};
use kernel::decode::{config, mass};
use kernel::generated::links::Defaults;
use validation::decode_f32_eps::SplitMix;
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn harness() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

/// The words `entry` writes for `input`, `ins` words per sample in and `outs` out, dispatched in chunks of at most
/// 65535 workgroups of 64.
fn gpu(h: &GpuHarness, entry: &str, input: &[u32], ins: usize, outs: usize) -> Vec<u32> {
    use wgpu::util::DeviceExt;
    let d = h.device();
    let wgsl = wgsl();
    let kernel = engine::compute::pipeline(
        d,
        &engine::compute::ComputeShader {
            label: entry,
            wgsl: &wgsl,
            entry,
        },
        FastMath::default(),
    )
    .expect("the entry point's pipeline");
    let bytes = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let mut out = Vec::new();
    for chunk in input.chunks(65535 * 64 * ins) {
        let n = chunk.len() / ins;
        let input = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("invariant input"),
            contents: &bytes(chunk),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("invariant output"),
            contents: &bytes(&vec![u32::MAX; n * outs]),
            usage: wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::COPY_SRC),
        });
        let size = (n * outs * 4) as u64;
        let read = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("invariant readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        let bindings = kernel.bind(d, &[&[&input, &output]]).expect("the bindings");
        let mut encoder = d.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            kernel.set(&mut pass, &bindings);
            pass.dispatch_workgroups((n as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
        h.queue().submit([encoder.finish()]);
        let slice = read.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map"));
        d.poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll");
        let view = slice.get_mapped_range().expect("readback range");
        out.extend(
            view.as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b)),
        );
        drop(view);
        read.unmap();
    }
    out
}

// ── The exact reference ─────────────────────────────────────────────────────────────────────────────────────────

/// `±mant·2^exp` of a finite f64, exactly.
fn parts(x: f64) -> (bool, u128, i64) {
    let b = x.to_bits();
    let field = ((b >> 52) & 0x7FF) as i64;
    let fraction = (b & ((1 << 52) - 1)) as u128;
    let neg = b >> 63 != 0;
    if field == 0 {
        (neg, fraction, -1074)
    } else {
        (neg, fraction | (1 << 52), field - 1075)
    }
}

/// `2·i·k < l²` in exact (integer) arithmetic, for finite `i`, `k`, `l`.
fn reference(i: f64, k: f64, l: f64) -> bool {
    let (ni, mi, ei) = parts(i);
    let (nk, mk, ek) = parts(k);
    let (_, ml, el) = parts(l);
    if mi == 0 || mk == 0 {
        return ml != 0;
    }
    if ni != nk {
        return true;
    }
    let (a, ea) = (mi * mk, ei + ek + 1);
    let (b, eb) = (ml * ml, 2 * el);
    if b == 0 {
        return false;
    }
    let len = |x: u128| 128 - i64::from(x.leading_zeros());
    let (ta, tb) = (len(a) + ea, len(b) + eb);
    if ta != tb {
        return ta < tb;
    }
    if ea >= eb {
        (a << (ea - eb)) < b
    } else {
        a < (b << (eb - ea))
    }
}

// ── The triples ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The positive f32 of exponent field `field` and fraction `fraction`.
fn make(field: u32, fraction: u32) -> f32 {
    f32::from_bits((field << 23) | (fraction & 0x007F_FFFF))
}

/// `x > 0` moved `n` values along the positive axis.
fn step(x: f32, n: i32) -> f32 {
    f32::from_bits((x.to_bits() as i32 + n) as u32)
}

/// Triples `(I, K*, L_z)` around `K* = L_z²/(2I)`, up to 3 values either way, `L_z` and `I` at every scale.
fn near_boundary(g: &mut SplitMix, n: usize) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    while out.len() < n {
        let pick = |g: &mut SplitMix, lo: u64, hi: u64| {
            make(
                (lo + g.next_u64() % (hi - lo + 1)) as u32,
                g.next_u64() as u32,
            )
        };
        let i = match g.next_u64() % 3 {
            0 => 1.0,
            1 => pick(g, 124, 130),
            _ => pick(g, 1, 254),
        };
        let l = pick(g, 0, 254);
        let k0 = (f64::from(l) * f64::from(l) / (2.0 * f64::from(i))) as f32;
        if !(k0.is_finite() && k0 > 0.0) {
            continue;
        }
        let k = step(k0, (g.next_u64() % 7) as i32 - 3);
        if k.is_finite() && k > 0.0 {
            out.push([i, k, l]);
        }
    }
    out
}

/// `L_z = 1 + 2⁻ʲ`, `I = 1`, `K* = ½ + 2⁻ʲ` and its neighbours: `2IK* − L_z² = −2⁻²ʲ`, in each limb column.
fn columns() -> Vec<[f32; 3]> {
    (1..=23)
        .flat_map(|j| {
            let l = make(127, 1 << (23 - j));
            let k = 0.5 + 2f32.powi(-j);
            [[1.0, k, l], [1.0, step(k, 1), l], [1.0, step(k, -1), l]]
        })
        .collect()
}

/// Sparse-bit significands around 1.
fn sparse(g: &mut SplitMix, n: usize) -> Vec<[f32; 3]> {
    let bits =
        |g: &mut SplitMix| (0..g.next_u64() % 4).fold(0u32, |f, _| f | 1 << (g.next_u64() % 23));
    (0..n)
        .map(|_| {
            let (ei, ek) = (
                (126 + g.next_u64() % 3) as u32,
                (125 + g.next_u64() % 3) as u32,
            );
            [make(ei, bits(g)), make(ek, bits(g)), make(127, bits(g))]
        })
        .collect()
}

/// `(I·2^a, K*·2^b, L_z·2^c)` with `a + b = 2c`, which keeps `2IK* − L_z²`'s sign, where every scaled value is exact.
fn scaled(base: &[[f32; 3]], shifts: &[(i32, i32)]) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for t in base {
        for &(a, c) in shifts {
            let s = [a, 2 * c - a, c];
            let exact = [0, 1, 2].map(|n| f64::from(t[n]) * 2f64.powi(s[n]));
            let v = exact.map(|x| x as f32);
            if (0..3).all(|n| v[n].is_finite() && f64::from(v[n]) == exact[n]) {
                out.push(v);
            }
        }
    }
    out
}

/// The same triples at decimal scales, `I` and `K*` by `1e-18 … 1e18`, `L_z` by the square root of their product.
fn decimal(base: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for t in base {
        for e in [-18, -9, -3, 3, 9, 18] {
            let f = 10f64.powi(e);
            let v = [t[0], t[1], t[2]].map(|x| (f64::from(x) * f) as f32);
            if v.iter().all(|x| x.is_finite()) {
                out.push(v);
            }
        }
    }
    out
}

/// The column triples at `2^t` for `t` in `ts`.
fn columns_at(ts: std::ops::RangeInclusive<i32>) -> Vec<[f32; 3]> {
    ts.flat_map(|t| {
        columns()
            .into_iter()
            .map(move |c| c.map(|x| (f64::from(x) * 2f64.powi(-t)) as f32))
    })
    .collect()
}

/// Every family, and its tiny members apart: the column triples at `2^t` for `t` from −40 to −50, where the products
/// of unnormalised limbs fall below f32's smallest normal number but are still exact, so that only a GPU that flushes
/// subnormals drops them. The column triples from `2⁻⁵¹` to `2⁻⁶³` are among the rest.
fn families() -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let mut g = SplitMix::new(209);
    let mut base = near_boundary(&mut g, 20_000);
    base.extend(columns());
    base.extend(sparse(&mut g, 20_000));
    let shifts: Vec<(i32, i32)> = (-63..=60)
        .step_by(7)
        .flat_map(|a| [(a, a), (a, a / 2), (-a, a)])
        .collect();
    let mut all = base.clone();
    all.extend(scaled(&base[..2000], &shifts));
    all.extend(decimal(&base[..2000]));
    let tiny = columns_at(40..=50);
    all.extend(tiny.iter().copied());
    all.extend(columns_at(51..=63));
    (all, tiny)
}

// ── invariant_infeasible_branch ─────────────────────────────────────────────────────────────────────────────────

/// The words of `ts`, three per triple.
fn words(ts: &[[f32; 3]]) -> Vec<u32> {
    ts.iter().flatten().map(|x| x.to_bits()).collect()
}

/// The GPU's branch on each triple: `decode_invariant_feasibility`.
fn gpu_branch(h: &GpuHarness, ts: &[[f32; 3]]) -> Vec<bool> {
    gpu(h, "decode_invariant_feasibility", &words(ts), 3, 1)
        .into_iter()
        .map(|w| {
            assert!(w <= 1, "the GPU wrote {w:#x}, not a branch");
            w == 1
        })
        .collect()
}

/// `gpu`, the GPU's branch on each of `ts`, is CPU-f32's, CPU-f64's on the widened triple, and the exact one.
fn check_branch(ts: &[[f32; 3]], gpu: &[bool]) {
    assert_eq!(ts.len(), gpu.len(), "the GPU's branches");
    for (t, &g) in ts.iter().zip(gpu) {
        let [i, k, l] = *t;
        let want = reference(f64::from(i), f64::from(k), f64::from(l));
        let (c32, c64) = (
            infeasible(i, k, l),
            infeasible(f64::from(i), f64::from(k), f64::from(l)),
        );
        assert!(
            g == want && c32 == want && c64 == want,
            "at (I, K*, L_z) = ({i:e}, {k:e}, {l:e}) the branches differ: GPU-f32 {g}, CPU-f32 {c32}, CPU-f64 {c64}, \
             exact {want}"
        );
    }
}

#[test]
fn invariant_infeasible_branch() {
    let h = harness();
    let (all, tiny) = families();
    let branches = gpu_branch(&h, &all);
    check_branch(&all, &branches);
    let short = branches.iter().filter(|&&b| b).count();
    // The unnormalised form, unflushed, is exact on the tiny triples: its control fails by the flushing alone.
    let kept: Vec<bool> = tiny
        .iter()
        .map(|&[i, k, l]| unnormalised(i, k, l, false))
        .collect();
    check_branch(&tiny, &kept);
    eprintln!(
        "invariant_infeasible_branch: {} triples ({} tiny), {short} infeasible, on {:?}",
        all.len(),
        tiny.len(),
        h.adapter_info().backend
    );
    assert!(
        short > all.len() / 4 && short < 3 * all.len() / 4,
        "{short} of {} infeasible",
        all.len()
    );
}

negative_control!(
    invariant_infeasible_branch,
    "the rounded comparison K* < L_z·L_z/(2I) at f32 in the GPU's place must fail",
    expected = "the branches differ",
    {
        let (all, _) = families();
        let rounded: Vec<bool> = all.iter().map(|&[i, k, l]| k < l * l / (2.0 * i)).collect();
        check_branch(&all, &rounded)
    }
);

/// `x`, or where `flushing`, 0 of its sign where it is below f32's smallest normal number, as a GPU that flushes
/// subnormals reads it.
fn flush(x: f32, flushing: bool) -> f32 {
    if flushing && x.abs() < f32::MIN_POSITIVE {
        0.0f32.copysign(x)
    } else {
        x
    }
}

/// The column test without the binades' normalisation, on the operands as given: `2IK* − L_z²` from the operands' own
/// limbs, every product and sum flushed where `flushing`, as a GPU that flushes subnormals does.
fn unnormalised(i: f32, k: f32, l: f32, flushing: bool) -> bool {
    let f = |x: f32| flush(x, flushing);
    let limbs = |x: f32| {
        let high = f32::from_bits(x.to_bits() & 0xFFFF_0000);
        let upper = f32::from_bits(x.to_bits() & 0xFFFF_FF00);
        [high, f(upper - high), f(x - upper)]
    };
    let ([i0, i1, i2], [k0, k1, k2], [l0, l1, l2]) = (limbs(i), limbs(k), limbs(l));
    let m = |a: f32, b: f32| f(a * b);
    let c = [
        2.0 * m(i0, k0) - m(l0, l0),
        2.0 * (m(i0, k1) + m(i1, k0)) - 2.0 * m(l0, l1),
        2.0 * ((m(i0, k2) + m(i1, k1)) + m(i2, k0)) - ((m(l0, l2) + m(l1, l1)) + m(l2, l0)),
        2.0 * (m(i1, k2) + m(i2, k1)) - 2.0 * m(l1, l2),
        2.0 * m(i2, k2) - m(l2, l2),
    ];
    c.into_iter().fold(0.0, |s, x| f(s + f(x))) < 0.0
}

mod flushed {
    use super::*;

    negative_control!(
        invariant_infeasible_branch,
        "the column test on unnormalised operands, its subnormal limb products flushed, must fail on the tiny triples",
        expected = "the branches differ",
        {
            let (_, tiny) = families();
            let flushed: Vec<bool> = tiny.iter().map(|&[i, k, l]| unnormalised(i, k, l, true)).collect();
            check_branch(&tiny, &flushed)
        }
    );
}

// ── invariant_infeasible_construct ──────────────────────────────────────────────────────────────────────────────

/// The construction's samples: decoded configurations (`z_μ₁ z_μ₂ z_α z_β`) by `(L_z, K*)` pairs: `L_z` of at most 12
/// significant bits with `K* = L_z²/2` exactly and one value either side, `K* = 0` and `K* < 0`, and random pairs either
/// side of the boundary.
fn samples() -> Vec<[f32; 6]> {
    let zs: [[f32; 4]; 6] = [
        [0.0; 4],
        [0.8, -0.4, 0.3, -0.7],
        [-0.6, 1.1, -1.2, 1.9],
        [1.5, 0.3, 2.5, 0.2],
        [-1.3, -0.9, -2.0, -2.5],
        [0.4, 2.0, 0.9, 3.0],
    ];
    let mut pairs = Vec::new();
    for l in [0.75f32, -1.375, 0.001953125, 3.5, -0.0625, 1.0, 0.0] {
        let edge = l * l / 2.0;
        pairs.push([l, edge]);
        pairs.push([l, -1.0]);
        if edge > 0.0 {
            pairs.push([l, step(edge, -1)]);
            pairs.push([l, step(edge, 1)]);
            pairs.push([l, 0.0]);
        }
    }
    let mut g = SplitMix::new(9);
    for _ in 0..500 {
        let l = (g.next_u64() >> 40) as f32 / (1u32 << 22) as f32 - 2.0;
        let edge = (f64::from(l) * f64::from(l) / 2.0) as f32;
        if edge > 0.0 {
            pairs.push([l, step(edge, (g.next_u64() % 5) as i32 - 2)]);
        }
    }
    zs.iter()
        .flat_map(|z| {
            pairs
                .iter()
                .map(move |p| [z[0], z[1], z[2], z[3], p[0], p[1]])
        })
        .collect()
}

/// The CPU's decoder construction at `R` on a sample, its masses and configuration decoded at `R`, its seed decided on
/// `z_α` (`construct`).
fn cpu<R: ExactSign + From<f32>>(s: &[f32; 6]) -> kernel::decode::invariant::Construction<R> {
    let w = s.map(<R as From<f32>>::from);
    let masses = mass::decode::<Defaults, R>([w[0], w[1]]);
    let config = config::decode::<Defaults, R>([w[2], w[3]], &masses);
    construct(&masses, &config, w[2], w[4], w[5])
}

/// `out`, the GPU's words for `samples`: each tag is CPU-f32's and CPU-f64's and the exact `K* < L_z²/2`'s, and every
/// momentum is finite.
fn check_construct(samples: &[[f32; 6]], out: &[u32]) {
    let n = invariant::construct_out_words();
    assert_eq!(out.len(), samples.len() * n, "the GPU's words");
    let mut short = 0;
    for (s, w) in samples.iter().zip(out.chunks(n)) {
        let at = format!("at z = {:?}, (L_z, K*) = ({:e}, {:e})", &s[..4], s[4], s[5]);
        let want = if reference(1.0, f64::from(s[5]), f64::from(s[4])) {
            Outcome::Infeasible
        } else {
            Outcome::Feasible
        };
        short += usize::from(want == Outcome::Infeasible);
        let (c32, c64) = (cpu::<f32>(s), cpu::<f64>(s));
        let words = [
            invariant::outcome_word(want),
            invariant::outcome_word(c32.outcome),
            invariant::outcome_word(c64.outcome),
        ];
        assert!(
            w[6] == words[0] && words[1] == words[0] && words[2] == words[0],
            "the tag {at}: GPU-f32 {}, CPU-f32 {}, CPU-f64 {}, exact {}",
            w[6],
            words[1],
            words[2],
            words[0]
        );
        assert!(w[7] <= 3, "no seed qualified {at}: {}", w[7]);
        let p: Vec<f32> = w[..6].iter().map(|b| f32::from_bits(*b)).collect();
        assert!(
            p.iter().all(|x| x.is_finite()),
            "a non-finite momentum on the GPU {at}: {p:?}"
        );
    }
    assert!(
        short > 0 && short < samples.len(),
        "{short} of {} infeasible",
        samples.len()
    );
}

#[test]
fn invariant_infeasible_construct() {
    let h = harness();
    let s = samples();
    let input: Vec<u32> = s.iter().flatten().map(|x| x.to_bits()).collect();
    let out = gpu(
        &h,
        "decode_invariant_construct_defaults",
        &input,
        invariant::construct_in_words(),
        invariant::construct_out_words(),
    );
    check_construct(&s, &out);
}

negative_control!(
    invariant_infeasible_construct,
    "a GPU tag of feasible at K* one value below L_z²/2 must fail",
    expected = "the tag",
    {
        let h = harness();
        let s = samples();
        let input: Vec<u32> = s.iter().flatten().map(|x| x.to_bits()).collect();
        let n = invariant::construct_out_words();
        let mut out = gpu(
            &h,
            "decode_invariant_construct_defaults",
            &input,
            invariant::construct_in_words(),
            n,
        );
        out[2 * n + 6] = invariant::outcome_word(Outcome::Feasible);
        check_construct(&s, &out)
    }
);

mod construct_nan {
    use super::*;

    negative_control!(
        invariant_infeasible_construct,
        "a GPU momentum of NaN at an infeasible pixel must fail",
        expected = "a non-finite momentum",
        {
            let h = harness();
            let s = samples();
            let input: Vec<u32> = s.iter().flatten().map(|x| x.to_bits()).collect();
            let n = invariant::construct_out_words();
            let mut out = gpu(
                &h,
                "decode_invariant_construct_defaults",
                &input,
                invariant::construct_in_words(),
                n,
            );
            out[n + 3] = f32::NAN.to_bits();
            check_construct(&s, &out)
        }
    );
}

// ── invariant_seed_parity ───────────────────────────────────────────────────────────────────────────────────────

/// The seed check's samples: `z_α` at 0, either side of it to `3e-7` (where `cos²α` and `sin²α` are equal to f32's
/// rounding) and beyond, each with random unequal masses and `β`, at a feasible `(L_z, K*)`.
fn seed_samples() -> Vec<[f32; 6]> {
    let mut g = SplitMix::new(43);
    let mut unit = || (g.next_u64() >> 40) as f32 / (1u32 << 24) as f32;
    let mut alphas = vec![0.0f32, -0.0, f32::from_bits(1), -f32::from_bits(1)];
    for x in [
        1e-30f32, 1e-12, 1e-9, 1e-8, 3e-8, 1e-7, 3e-7, 1e-6, 1e-2, 0.5, 2.0, 6.0,
    ] {
        alphas.extend([x, -x]);
    }
    let mut zs: Vec<f32> = alphas.iter().flat_map(|&a| [a; 40]).collect();
    zs.extend((0..8000).map(|_| (2.0 * unit() - 1.0) * 3e-7));
    zs.into_iter()
        .map(|a| {
            let lz = 2.0 * unit() - 1.0;
            let k = lz * lz / 2.0 + 0.05 + unit();
            [
                8.0 * unit() - 4.0,
                8.0 * unit() - 4.0,
                a,
                8.0 * unit() - 4.0,
                lz,
                k,
            ]
        })
        .collect()
}

/// `out`, the GPU's words for `samples`: each seed is R-82's decided on `z_α`, `(ρ, 0)` (word 0) where `z_α ≤ 0` and
/// `(0, λ)` (word 1) elsewhere, CPU-f64's; and the momenta are CPU-f64's to f32's rounding, not a fork's O(1)
/// (dd_decoder §5 test 12).
fn check_seeds(samples: &[[f32; 6]], out: &[u32]) {
    let n = invariant::construct_out_words();
    assert_eq!(out.len(), samples.len() * n, "the GPU's words");
    for (s, w) in samples.iter().zip(out.chunks(n)) {
        let at = format!("at z = {:?}, (L_z, K*) = ({:e}, {:e})", &s[..4], s[4], s[5]);
        let want = u32::from(s[2] > 0.0);
        let c64 = cpu::<f64>(s);
        assert!(
            w[7] == want && invariant::seed_word(c64.seed) == want,
            "the seed {at}: GPU-f32 {}, CPU-f64 {}, R-82 on z_α {want}",
            w[7],
            invariant::seed_word(c64.seed)
        );
        let size: f64 = c64.p.iter().flatten().map(|x| x.abs()).sum();
        let apart: f64 = w[..6]
            .iter()
            .zip(c64.p.iter().flatten())
            .map(|(b, x)| (f64::from(f32::from_bits(*b)) - x).abs())
            .sum();
        assert!(
            apart <= 1e-4 * size,
            "the momenta {at} differ by {apart:e} of {size:e}, GPU-f32 against CPU-f64"
        );
    }
}

/// The GPU's construction words for `samples`.
fn gpu_construct(samples: &[[f32; 6]]) -> Vec<u32> {
    let h = harness();
    let input: Vec<u32> = samples.iter().flatten().map(|x| x.to_bits()).collect();
    gpu(
        &h,
        "decode_invariant_construct_defaults",
        &input,
        invariant::construct_in_words(),
        invariant::construct_out_words(),
    )
}

#[test]
fn invariant_seed_parity() {
    let s = seed_samples();
    check_seeds(&s, &gpu_construct(&s));
}

negative_control!(
    invariant_seed_parity,
    "seeds and momenta from the rule on the computed norms, at f32, must fork from z_α's near z_α = 0",
    expected = "the seed",
    {
        let s = seed_samples();
        let n = invariant::construct_out_words();
        let mut out = gpu_construct(&s);
        for (x, w) in s.iter().zip(out.chunks_mut(n)) {
            let masses = mass::decode::<Defaults, f32>([x[0], x[1]]);
            let config = config::decode::<Defaults, f32>([x[2], x[3]], &masses);
            let c = invariant::construct_by_norms(&masses, &config, x[4], x[5]);
            for (o, p) in w.iter_mut().zip(c.p.iter().flatten()) {
                *o = p.to_bits();
            }
            w[7] = invariant::seed_word(c.seed);
        }
        check_seeds(&s, &out)
    }
);
