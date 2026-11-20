//! Encode `E` on the GPU (`kernel::stage::encode`, `engine::decode_dispatch::{encode_gpu, beta_gpu}`; TASK-M2-15): the
//! kernel's f32 build, `target/spirv/kernel.wgsl`, dispatched natively, read back and checked against the same source on
//! the CPU at f64 and f32. CI runs this file in the `gpu-kernel` job, after `cargo xtask build-kernel`, and on Metal and
//! lavapipe.
//! - `encode_beta_half_axis`: `β` on the `−x` half-axis, `λ̃_y = ±0` and `λ̃_x < 0`, is `π̂` by the select, never
//!   `atan2`'s value, on CPU-f64, CPU-f32 and GPU-f32, `±0` operands included; at `λ̃ = (±0, ±0)` it is 0; and collinear
//!   states with body 2 on the `−x` side encode on the GPU as on the CPU (R-438; REQ-ENC-007).
//! - `encode_beta_signed_zero`: `λ̃_x = ±0` with `λ̃_y ≠ 0`: `β` is `π/2` on CPU-f64, CPU-f32 and GPU-f32, Metal's
//!   `atan2(y, −0)` sign kept out by `beta`'s select (pitfalls §11; REQ-ENC-007).
//! - `encode_exact_poles`: configurations with `‖λ̃‖ = 0` and `‖ρ̃‖ = 0` exactly, `±0` components included: no NaN,
//!   the output identical on repeat, bit for bit, and on CPU-f64, CPU-f32 and GPU-f32 to the precision (`z` against
//!   CPU-f32's, the angles and the decoded state against CPU-f64's), the notices identical, and `lookup_clamped` set, the
//!   encode clamp flag (R-133; REQ-DEC-014).
//!
//! **Tolerances** (applied per R-369, a test decision): `z` and the angles agree across backends to [`AGREE_EPS`] f32
//! eps over `1 + |z|`, set from the worst case measured on Metal and lavapipe, each printed with its backend, with
//! headroom: WGSL's `atan2`, `log` and `√` are not correctly rounded, so the backends' last bits can differ; the selects
//! and notices are compared exactly.
//! Against CPU-f64, the comparison is in physical units (dd_encode §3.5; [`check_physical`]).
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory.

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{beta_gpu, encode_gpu};
use kernel::decode::{decode, Ic, Latent};
use kernel::encode::{blocks, encode, Encoded};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::stage::encode::EncodeOut;
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

fn gpu_encode(h: &GpuHarness, ics: &[Ic<f32>]) -> Vec<EncodeOut> {
    encode_gpu(h.device(), h.queue(), &wgsl(), ics, FastMath::default())
        .expect("the encode dispatch")
}

fn gpu_beta(h: &GpuHarness, pairs: &[[f32; 2]]) -> Vec<f32> {
    beta_gpu(h.device(), h.queue(), &wgsl(), pairs, FastMath::default()).expect("the β dispatch")
}

fn narrow(ic: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    }
}

const EPS32: f64 = f32::EPSILON as f64;

/// The cross-backend limit on `z`, the angles and the decoded state, in f32 eps over `1 + |x|` (R-85, R-110; applied per
/// R-369, a test tolerance). Measured worst, each backend's whole run, by [`Worst::check`]'s print:
/// - Metal: 0.5 (`encode_beta_half_axis`) and 1.3 (`encode_exact_poles`) on an Apple M3 Pro, locally; 2.0 and 1.7 on
///   CI's Apple Paravirtual device (`gpu-metal`, run 38972849213).
/// - lavapipe: 21.1 (output 0, `z_α`, at the half-axis states) and 22.4 (output 1, `z_β`, at the turned `‖ρ̃‖ = 0` pole,
///   `β = π/4`) on llvmpipe, LLVM 20.1.2 (`gpu-lavapipe`, runs 38972849213 and 38977653340; first seen failing 16 in
///   jobs 117191742262 and 117191741963).
///
/// 64 is about 2.9 times lavapipe's 22.4. WGSL's own accuracy bounds these states only at about `3.9·10³` eps (`z_α`,
/// `atan2`'s 4096 ULP) and `1.7·10³` (`z_β` at `π/4`), so the limit is the measured spread, not the spec. The failures
/// these tests target are far past it: `−π` for `π̂` moves `z_β` by `2·logit(1 − ε_z) ≈ 27.6`, about `10⁸` eps; the
/// selects and notices are compared exactly.
const AGREE_EPS: f64 = 64.0;

/// One encode's comparable outputs at any precision: `z`, `α`, `β`, and the notices `(coincident, clamped controls,
/// mirrored)`.
#[derive(Clone, Debug, PartialEq)]
struct Out {
    values: Vec<f64>,
    notices: (bool, u32, bool),
}

fn out_of<R: LinkReal + Into<f64>>(e: &Encoded<R>) -> Out {
    let r = &e.report;
    Out {
        values: e
            .z
            .z
            .iter()
            .chain(&[e.blocks.alpha, e.blocks.beta])
            .map(|x| (*x).into())
            .collect(),
        notices: (r.coincident, r.clamped_controls, r.mirrored),
    }
}

fn out_of_gpu(o: &EncodeOut) -> Out {
    let r = &o.report;
    Out {
        values: o
            .z
            .iter()
            .chain(&[o.alpha, o.beta])
            .map(|x| f64::from(*x))
            .collect(),
        notices: (r.coincident, r.clamped_controls, r.mirrored),
    }
}

/// The worst cross-backend difference seen, in f32 eps, and where. A NaN difference is the worst there is: [`Worst::see`]
/// keeps it (pitfalls §9), though [`check_agree`] has asserted every value finite before it gets here.
#[derive(Debug, Default)]
struct Worst {
    eps: f64,
    at: String,
}

impl Worst {
    fn see(&mut self, e: f64, at: impl FnOnce() -> String) {
        if e.is_nan() || e > self.eps {
            self.eps = e;
            self.at = at();
        }
    }

    /// Prints the worst with the backend it was measured on, then holds it to [`AGREE_EPS`]: the whole run is measured
    /// before the limit is applied, so a failing run still prints the spread.
    fn check(&self, h: &GpuHarness, test: &str) {
        let info = h.adapter_info();
        eprintln!(
            "{test}: worst cross-backend difference {:.1} f32 eps on {} ({:?}), at {}",
            self.eps, info.name, info.backend, self.at
        );
        assert!(
            self.eps <= AGREE_EPS,
            "{test}: {:.1} f32 eps on {} ({:?}), over AGREE_EPS = {AGREE_EPS}, at {}",
            self.eps,
            info.name,
            info.backend,
            self.at
        );
    }
}

/// `a` against `b`, both at f32: the notices equal and every value finite in both, asserted; each value's difference in
/// f32 eps over `1 + |b|` goes to `worst`.
fn check_agree(a: &Out, b: &Out, what: &str, worst: &mut Worst) {
    assert_eq!(
        a.notices, b.notices,
        "the notices differ, {what}: {:?} against {:?}",
        a.notices, b.notices
    );
    for (k, (x, y)) in a.values.iter().zip(&b.values).enumerate() {
        assert!(
            x.is_finite() && y.is_finite(),
            "output {k} is not finite, {what}: {x} against {y}"
        );
        let e = (x - y).abs() / (EPS32 * (1.0 + y.abs()));
        worst.see(e, || format!("output {k}, {what}: {x:e} against {y:e}"));
    }
}

/// The decoded state of `z` at f64, its masses, positions and momenta in one list.
fn decoded(z: [f64; 8]) -> Vec<f64> {
    let ic = decode::<Defaults, f64>(&Latent::new(z)).ic;
    ic.m.iter()
        .chain(ic.r.as_flattened())
        .chain(ic.p.as_flattened())
        .copied()
        .collect()
}

/// The GPU's f32 encode `a` against the CPU's f64 encode `b`, compared in physical units (dd_encode §3.5): the notices
/// equal; `α` and `β` in f32 eps over `1 + |b|`; and the state each `z` decodes to at f64, over `1 + |x|`, each to
/// `worst`. Not `z` itself: at a clamp, `z = logit(1 − ε_z)` reads `1 − ε_z` rounded to each precision, and `logit`'s
/// slope there, `10⁶`, makes f32's rounding of it about `10⁴` f32 eps of `z`, but nothing in the state.
fn check_physical(a: &Out, b: &Out, what: &str, worst: &mut Worst) {
    let angles = |o: &Out| Out {
        values: o.values[8..].to_vec(),
        notices: o.notices,
    };
    check_agree(&angles(a), &angles(b), what, worst);
    let z = |o: &Out| -> [f64; 8] { std::array::from_fn(|k| o.values[k]) };
    for (k, (x, y)) in decoded(z(a)).iter().zip(&decoded(z(b))).enumerate() {
        assert!(
            x.is_finite() && y.is_finite(),
            "the decoded state's component {k} is not finite, {what}: {x} against {y}"
        );
        let e = (x - y).abs() / (EPS32 * (1.0 + y.abs()));
        worst.see(e, || {
            format!("the decoded state's component {k}, {what}: {x:e} against {y:e}")
        });
    }
}

// ── encode_beta_half_axis (REQ-ENC-007; R-438) ──────────────────────────────────────────────────────────────────

/// The `−x` half-axis's operands, `λ̃_y` each sign of zero, `λ̃_x` from `−1` to the smallest normal.
fn half_axis() -> Vec<[f32; 2]> {
    let mut pairs = Vec::new();
    for x in [-1.0f32, -0.6, -1e-3, -1e-20, -f32::MIN_POSITIVE] {
        pairs.push([x, 0.0]);
        pairs.push([x, -0.0]);
    }
    pairs
}

/// The half-axis's `β` on one backend: `π̂`'s bits at every operand.
fn check_half_axis<R: LinkReal>(betas: &[R], pairs: &[[f32; 2]], backend: &str) {
    let pi = R::LIT_7;
    assert_eq!(
        betas.len(),
        pairs.len(),
        "{} β for {} operands on {backend}",
        betas.len(),
        pairs.len()
    );
    for (b, pair) in betas.iter().zip(pairs) {
        assert!(
            b.to_f64().unwrap().to_bits() == pi.to_f64().unwrap().to_bits(),
            "β = {:e}, not π̂ = {:e}, on the −x half-axis at {pair:?} on {backend}",
            b.to_f64().unwrap(),
            pi.to_f64().unwrap()
        );
    }
}

/// Collinear states with body 2 on the `−x` side, `λ̃_y = ±0`: masses `(0.2, 0.3, 0.5)`, `ρ̃ = (0.8, 0)` and
/// `λ̃ = (−0.6, ±0)`, reconstructed by the decoder's own map, with interior momenta.
fn collinear_states() -> Vec<Ic<f64>> {
    use kernel::decode::config::reconstruct;
    use kernel::decode::mass::from_masses;
    use kernel::decode::momentum::to_particles;
    let masses = from_masses([0.2, 0.3, 0.5]);
    [0.0f64, -0.0]
        .iter()
        .map(|y| {
            let (_, _, r) = reconstruct(&masses, [0.8, 0.0], [-0.6, *y]);
            Ic {
                m: masses.m,
                r,
                p: to_particles(&masses, [0.3, -0.2], [-0.4, 0.25]),
            }
        })
        .collect()
}

#[test]
fn encode_beta_half_axis() {
    let h = harness();
    let pairs = half_axis();
    let wide: Vec<[f64; 2]> = pairs.iter().map(|p| p.map(f64::from)).collect();
    check_half_axis(&gpu_beta(&h, &pairs), &pairs, "GPU-f32");
    check_half_axis(
        &pairs.iter().map(|p| blocks::beta(*p)).collect::<Vec<f32>>(),
        &pairs,
        "CPU-f32",
    );
    check_half_axis(
        &wide.iter().map(|p| blocks::beta(*p)).collect::<Vec<f64>>(),
        &pairs,
        "CPU-f64",
    );
    // The pole, each sign of each zero: 0 on every backend.
    let poles = [[0.0f32, 0.0], [-0.0, 0.0], [0.0, -0.0], [-0.0, -0.0]];
    let at_poles = gpu_beta(&h, &poles);
    assert_eq!(
        at_poles.len(),
        poles.len(),
        "{} β for {} poles",
        at_poles.len(),
        poles.len()
    );
    for (b, pair) in at_poles.iter().zip(&poles) {
        assert_eq!(*b, 0.0, "β = {b:e}, not 0, at the pole {pair:?} on GPU-f32");
    }
    assert!(gpu_beta(&h, &[]).is_empty(), "β of no operands");
    // Collinear states through the whole encode: the GPU's `β` is `π̂`, and its encode is the CPU's.
    let states = collinear_states();
    let narrowed: Vec<Ic<f32>> = states.iter().map(narrow).collect();
    let mut worst = Worst::default();
    for (o, (x64, x32)) in gpu_encode(&h, &narrowed)
        .iter()
        .zip(states.iter().zip(&narrowed))
    {
        assert_eq!(
            o.beta.to_bits(),
            f32::from_bits(0x4049_0FDA).to_bits(),
            "the GPU's encode gives β = {:e}, not π̂, at {x32:?}",
            o.beta
        );
        let g = out_of_gpu(o);
        check_agree(
            &g,
            &out_of(&encode::<Defaults, f32>(x32)),
            &format!("GPU-f32 against CPU-f32 at {x32:?}"),
            &mut worst,
        );
        check_physical(
            &g,
            &out_of(&encode::<Defaults, f64>(x64)),
            &format!("GPU-f32 against CPU-f64 at {x32:?}"),
            &mut worst,
        );
    }
    worst.check(&h, "encode_beta_half_axis");
}

negative_control!(
    encode_beta_half_axis,
    "atan2's value where the zero's sign is read as −0, as WGSL may read it, −π, must fail the select's π̂",
    expected = "not π̂",
    {
        let pairs = [[-0.6f32, 0.0]];
        check_half_axis(&[(-0.0f32).atan2(-0.6)], &pairs, "atan2");
    }
);

// ── encode_beta_signed_zero (REQ-ENC-007; pitfalls §11) ─────────────────────────────────────────────────────────

/// `β`'s limit off `π/2`, in f32 eps of `π/2` (applied per R-369, a test tolerance): a backend reading `λ̃_x = −0`'s
/// sign turns `β` by `π`, `2²³` eps; WGSL's `atan2` is not correctly rounded, and 4 eps leaves it room.
const SIGNED_ZERO_EPS: f64 = 4.0;

/// The `y` axis's operands, `λ̃_x` each sign of zero, `λ̃_y` of each sign from 3 to the smallest normal: `β = π/2`.
fn y_axis() -> Vec<[f32; 2]> {
    let mut pairs = Vec::new();
    for y in [3.0f32, 1.0, -1.0, 0.5, 1e-3, -1e-20, f32::MIN_POSITIVE] {
        pairs.push([-0.0, y]);
        pairs.push([0.0, y]);
    }
    pairs
}

/// `β` is `π/2` within [`SIGNED_ZERO_EPS`] `eps` of it at each operand; returns the worst miss in `eps`.
fn check_y_axis<R: LinkReal>(betas: &[R], pairs: &[[f32; 2]], eps: f64, backend: &str) -> f64 {
    assert_eq!(
        betas.len(),
        pairs.len(),
        "{} β for {} operands on {backend}",
        betas.len(),
        pairs.len()
    );
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut worst = 0.0f64;
    for (b, pair) in betas.iter().zip(pairs) {
        let b = b.to_f64().unwrap();
        let err = (b - quarter).abs() / (eps * quarter);
        assert!(
            err <= SIGNED_ZERO_EPS,
            "β = {b:e}, not π/2 ({err:.3e} eps), on the y axis at {pair:?} on {backend}"
        );
        worst = worst.max(err);
    }
    worst
}

/// A model of Metal's own `atan2` at its default math mode (naga passes the call through; pitfalls §11), consistent
/// with the outputs measured at the four sites, not taken from Metal's implementation: `atan(y/x)`, turned by `π` to
/// `y`'s side where `x < 0`; a `−0` `x` is not below 0, so `atan2(y, −0)` is `atan(∓∞)`, the opposite sign of `π/2` to
/// IEEE's.
#[cfg(feature = "controls")]
fn metal_atan2(y: f32, x: f32) -> f32 {
    let t = (y / x).atan();
    match (x < 0.0, y >= 0.0) {
        (true, true) => t + std::f32::consts::PI,
        (true, false) => t - std::f32::consts::PI,
        _ => t,
    }
}

#[test]
fn encode_beta_signed_zero() {
    let h = harness();
    let pairs = y_axis();
    let wide: Vec<[f64; 2]> = pairs.iter().map(|p| p.map(f64::from)).collect();
    let worst = check_y_axis(&gpu_beta(&h, &pairs), &pairs, EPS32, "GPU-f32");
    check_y_axis(
        &pairs.iter().map(|p| blocks::beta(*p)).collect::<Vec<f32>>(),
        &pairs,
        EPS32,
        "CPU-f32",
    );
    check_y_axis(
        &wide.iter().map(|p| blocks::beta(*p)).collect::<Vec<f64>>(),
        &pairs,
        f64::EPSILON,
        "CPU-f64",
    );
    let info = h.adapter_info();
    eprintln!(
        "encode_beta_signed_zero: worst GPU-f32 β {worst:.2} f32 eps of π/2 on {} ({:?})",
        info.name, info.backend
    );
}

negative_control!(
    encode_beta_signed_zero,
    "β from Metal's atan2 with λ̃_x's zero sign kept, which turns β by π at λ̃_x = −0, must fail",
    expected = "not π/2",
    {
        let pairs = y_axis();
        let betas: Vec<f32> = pairs
            .iter()
            .map(|[x, y]| metal_atan2(y.abs(), *x))
            .collect();
        check_y_axis(&betas, &pairs, EPS32, "Metal's atan2");
    }
);

// ── encode_exact_poles (REQ-DEC-014; R-133, R-437) ──────────────────────────────────────────────────────────────

/// States at the exact `α`-poles, masses `(¼, ¼, ½)`, every product of a mass and a coordinate exact, so `C` keeps
/// them on the pole: `‖λ̃‖ = 0`, body 2 at the inner pair's centre of mass, `r₀ = (−1, ±0)`, `r₁ = (1, ±0)`,
/// `r₂ = (±0, ±0)`; and `‖ρ̃‖ = 0`, the inner pair coincident, `r₀ = r₁ = (−1, ±0)`, `r₂ = (1, ∓0)`, and turned,
/// `r₀ = r₁ = (−½, −½)`, `r₂ = (½, ½)`. Momenta interior.
fn pole_states() -> Vec<(Ic<f64>, &'static str)> {
    let m = [0.25, 0.25, 0.5];
    let p = [[0.1, -0.2], [0.3, 0.05], [-0.2, 0.075]];
    let mut states = Vec::new();
    for (y0, y2) in [(0.0, 0.0), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        for x2 in [0.0, -0.0] {
            states.push((
                Ic {
                    m,
                    r: [[-1.0, y0], [1.0, y0], [x2, y2]],
                    p,
                },
                "‖λ̃‖ = 0",
            ));
        }
        states.push((
            Ic {
                m,
                r: [[-1.0, y0], [-1.0, y0], [1.0, y2]],
                p,
            },
            "‖ρ̃‖ = 0",
        ));
    }
    states.push((
        Ic {
            m,
            r: [[-0.5, -0.5], [-0.5, -0.5], [0.5, 0.5]],
            p,
        },
        "‖ρ̃‖ = 0, turned",
    ));
    states
}

/// One pole's encode at one precision: `lookup_clamped` set, `α` the pole's (0, or `π/2` to rounding), `β` 0 by the
/// select at `‖λ̃‖ = 0`, the coincident notice iff `‖ρ̃‖ = 0`, and no NaN.
fn check_pole(o: &Out, kind: &str, at: &str) {
    let (coincident, controls, _) = o.notices;
    assert!(
        coincident || controls != 0,
        "lookup_clamped is not set at {kind}, {at}"
    );
    assert!(
        o.values.iter().all(|v| !v.is_nan()),
        "a NaN at {kind}, {at}: {:?}",
        o.values
    );
    let (alpha, beta) = (o.values[8], o.values[9]);
    if kind == "‖λ̃‖ = 0" {
        assert!(!coincident, "the coincident notice at {kind}, {at}");
        assert_eq!(alpha, 0.0, "α = {alpha:e} at {kind}, {at}");
        assert_eq!(
            beta.to_bits(),
            0.0f64.to_bits(),
            "β = {beta:e}, not +0 by the select, at {kind}, {at}"
        );
    } else {
        assert!(coincident, "no coincident notice at {kind}, {at}");
        assert!(
            (alpha - std::f64::consts::FRAC_PI_2).abs() <= 4.0 * EPS32,
            "α = {alpha:e}, not π/2, at {kind}, {at}"
        );
    }
}

#[test]
fn encode_exact_poles() {
    let h = harness();
    let states = pole_states();
    let narrowed: Vec<Ic<f32>> = states.iter().map(|(x, _)| narrow(x)).collect();
    let gpu = gpu_encode(&h, &narrowed);
    let again = gpu_encode(&h, &narrowed);
    let mut worst = Worst::default();
    for (i, ((x64, kind), x32)) in states.iter().zip(&narrowed).enumerate() {
        let at = format!("state {i}, r = {:?}", x64.r);
        let (g, cpu32, cpu64) = (
            out_of_gpu(&gpu[i]),
            out_of(&encode::<Defaults, f32>(x32)),
            out_of(&encode::<Defaults, f64>(x64)),
        );
        for (o, backend) in [(&g, "GPU-f32"), (&cpu32, "CPU-f32"), (&cpu64, "CPU-f64")] {
            check_pole(o, kind, &format!("{at} on {backend}"));
        }
        assert_eq!(
            out_of_gpu(&again[i]),
            g,
            "the GPU's encode differs on repeat at {at}"
        );
        assert_eq!(
            out_of(&encode::<Defaults, f64>(x64)),
            cpu64,
            "the CPU-f64 encode differs on repeat at {at}"
        );
        check_agree(
            &g,
            &cpu32,
            &format!("GPU-f32 against CPU-f32 at {at}"),
            &mut worst,
        );
        check_physical(
            &g,
            &cpu64,
            &format!("GPU-f32 against CPU-f64 at {at}"),
            &mut worst,
        );
    }
    assert!(gpu_encode(&h, &[]).is_empty(), "the encode of no states");
    eprintln!("encode_exact_poles: {} states", states.len());
    worst.check(&h, "encode_exact_poles");
}

negative_control!(
    encode_exact_poles,
    "an off-pole state, its body 2 off the inner pair's centre, read as the ‖λ̃‖ = 0 pole must fail",
    expected = "lookup_clamped is not set",
    {
        let x = Ic {
            m: [0.25, 0.25, 0.5],
            r: [[-1.0, 0.0], [1.0, 0.0], [0.3, 0.4]],
            p: [[0.0; 2]; 3],
        };
        check_pole(
            &out_of(&encode::<Defaults, f64>(&x)),
            "‖λ̃‖ = 0",
            "an off-pole state",
        );
    }
);
