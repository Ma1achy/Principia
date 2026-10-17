//! QA tests for TASK-M2-15, written from the requirements, not the implementation: encode `E = (block inverses) ∘ C`
//! on the CPU at f64 and f32. The GPU side is `engine`'s `qa_TASK-M2-15_gpu`.
//!
//! - REQ-ENC-007 (`encode_qa_m215_block_inverses`): the forward mates of inverse-encode Part 3 / dd_encode §3.1 are
//!   written here from the docs (softmax ∘ μ_max·tanh, `α = (π/2)σ`, `β = πσ`, `q = q_max(2σ − 1)`, the Jacobi map of
//!   dd_decoder §3.2 and the momentum table of §3.1), never taken from the kernel. Fuzzed interior `z`, put through that
//!   forward map and a random gauge element `g` of dd_encode §3.3 off the mirror band (translation, boost, rotation,
//!   scale `s` with `p` scaled by `s^(−1/2)`, and the mirror on positions *and* momenta), encodes back to `z`, and its
//!   decode is the canonical state in physical units (Part 4). The control mirrors positions only, the half-mirror
//!   §3.3 names.
//! - REQ-ENC-007, REQ-ENC-014 (`encode_qa_m215_clamp_iff`): each `q` component and each mass ratio is driven across its
//!   clamp: the control's bit and `lookup_clamped` are set iff the inverse's argument lies outside the clamp interval
//!   (`|q| > q_max(1 − 2ε_q)`, the `s` clamp; `|μ| > (1 − ε_μ)μ_max`), `|q| > q_max` always clamped and flagged, the
//!   clamped `z` finite and on the clamp's image.
//! - REQ-ENC-007 (`encode_qa_m215_beta_quotient`): `β` alone (R-437, R-438): `β(x, −y)` and `β(x, y)` have the same bits
//!   (`abs`, the mirror's quotient), `β ∈ [0, π̂]`, `(±0, ±0)` gives `+0`, the `−x` half-axis `π̂` at each precision
//!   (the nearest value at f64, rounded toward zero at f32; R-417), never `atan2`'s `−π`.
//! - REQ-ENC-007 (`encode_qa_m215_collinear_beta`): dd_encode §5 test 10 at the three Euler collinear central
//!   configurations of masses `(0.2, 0.3, 0.5)` (solved here by bisection on Euler's condition, so no landmark sits on
//!   the pole), with `λ̃_y ∈ {−1e-13, −0, +0, +1e-13}`; and the pole landmark `m₀ = m₁` on its own.
//! - REQ-DEC-014 (`encode_qa_m215_exact_poles`): `‖λ̃‖ = 0` and `‖ρ̃‖ = 0` exactly, every sign of the zero components:
//!   no NaN, identical on repeat and across the zeros' signs, `lookup_clamped` set, `β` and `φ` R-437's `+0`, and the
//!   CPU-f32 instantiation agreeing with CPU-f64 (angles, notices, decoded state).
//! - REQ-ENC-014 (`encode_qa_m215_notices`): a canonical input raises no notice; a mirrored one only
//!   `lookup_mirrored`; an out-of-range momentum only `lookup_clamped`.
//! - REQ-SYS-015 (`encode_qa_m215_no_wgsl_decode`): no checked-in `.wgsl` file carries the decode's or encode's link
//!   formulae (the verify detail's grep, including `.wgsl`): the fragment decode/encode can only be the translation's
//!   output, which lives under `target/`.
//!
//! **Tolerances** (applied per R-369, test decisions): f64 rounding bounds, each the precision's epsilon times the
//! conditioning the docs give (`d logit/ds = 1/(s(1 − s))`, `d artanh/dt = 1/(1 − t²)`, Part 4) times the input's
//! magnitude after the gauge, with headroom; stated at each constant. `ε_phys` is uncalibrated (REQ-ENC-024), so the
//! "within `ε_phys`" components of test 10 are held to a rounding bound, tighter than any calibrated `ε_phys` can be.
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory.

use std::f64::consts::{FRAC_PI_2, PI};
use std::path::{Path, PathBuf};

use kernel::decode::{decode, Ic, Latent};
use kernel::encode::blocks::beta;
use kernel::encode::{encode, Encoded};
use kernel::generated::links::Defaults;
use validation::negative_control;

// R-10, inverse-encode Part 3, R-82.
const MU_MAX: f64 = 5.0;
const Q_MAX: f64 = 2.0;
const EPS_MU: f64 = 1e-6;
const EPS_Z: f64 = 1e-6;
const EPS_Q: f64 = 1e-6;
const DELTA_LAMBDA: f64 = 1e-12;

/// The latent layout `z_α, z_β | z_q0 … z_q3 | z_μ1, z_μ2` (dd_decoder §3).
const Z_ALPHA: usize = 0;
const Z_BETA: usize = 1;
const Z_Q0: usize = 2;
const Z_MU1: usize = 6;

/// `z` recovered to this, f64, on `|z| ≤ 3`: the gauge's inputs reach `|a| + |r|·s ≤ 5 + 4·4`, so a coordinate carries
/// up to ~20 eps of absolute rounding after `C` undoes it; the controls' conditioning at `|z| = 3` is at most
/// `cosh²(3) ≈ 101` (artanh) and `1/(σ(3)σ(−3)) ≈ 22` (logit); 20 · 101 · 2.2e-16 ≈ 4.5e-13; ×200 headroom.
const Z_TOL_F64: f64 = 1e-10;
/// The canonical state recovered in physical units, f64 (Part 4): positions and momenta are O(1) on the section.
const PHYS_TOL_F64: f64 = 1e-12;
/// Test 10's "within ε_phys" and "plus rounding" terms, f64: a rounding bound, ε_phys being uncalibrated.
const ROUND_F64: f64 = 1e-13;

// ── A small generator, so the cases do not hang on the implementation's ─────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }
}

// ── The forward mates, from the docs ────────────────────────────────────────────────────────────────────────────

fn sigma(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn logit(s: f64) -> f64 {
    (s / (1.0 - s)).ln()
}

/// The physical state, CoM at the origin, of masses `m`, mass-weighted Jacobi vectors `ρ̃`, `λ̃` and Jacobi momenta
/// `p_ρ`, `p_λ` (dd_decoder §3.2; dd_encode §3.1's momentum row: `p₀ = −p_ρ − (m₀/M₀₁)p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`,
/// `p₂ = p_λ`).
fn from_jacobi(
    m: [f64; 3],
    rho_t: [f64; 2],
    lam_t: [f64; 2],
    p_rho: [f64; 2],
    p_lam: [f64; 2],
) -> Ic<f64> {
    let m01 = m[0] + m[1];
    let (srho, slam) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01).sqrt());
    let rho = rho_t.map(|v| v / srho);
    let lam = lam_t.map(|v| v / slam);
    // M = 1: M₀₁·R₀₁ + m₂(R₀₁ + λ) = 0.
    let r01 = lam.map(|v| -m[2] * v);
    let r0 = [0, 1].map(|k| r01[k] - m[1] / m01 * rho[k]);
    let r1 = [0, 1].map(|k| r01[k] + m[0] / m01 * rho[k]);
    let r2 = [0, 1].map(|k| r01[k] + lam[k]);
    let p0 = [0, 1].map(|k| -p_rho[k] - m[0] / m01 * p_lam[k]);
    let p1 = [0, 1].map(|k| p_rho[k] - m[1] / m01 * p_lam[k]);
    Ic {
        m,
        r: [r0, r1, r2],
        p: [p0, p1, p_lam],
    }
}

/// The decode's forward map under the default links, from the docs: the canonical state of `z`.
fn forward(z: [f64; 8]) -> Ic<f64> {
    let (mu1, mu2) = (MU_MAX * z[6].tanh(), MU_MAX * z[7].tanh());
    let w = [1.0, mu1.exp(), mu2.exp()];
    let t: f64 = w.iter().sum();
    let m = w.map(|v| v / t);
    let a = FRAC_PI_2 * sigma(z[0]);
    let b = PI * sigma(z[1]);
    let q = [2, 3, 4, 5].map(|k| Q_MAX * (2.0 * sigma(z[k]) - 1.0));
    from_jacobi(
        m,
        [a.cos(), 0.0],
        [a.sin() * b.cos(), a.sin() * b.sin()],
        [q[0], q[1]],
        [q[2], q[3]],
    )
}

/// The Jacobi coordinates of a state, from its positions and momenta (dd_decoder §3.2; dd_encode §3.1).
#[derive(Debug, Clone, Copy)]
struct Jac {
    m: [f64; 3],
    rho_t: [f64; 2],
    lam_t: [f64; 2],
    p_rho: [f64; 2],
    p_lam: [f64; 2],
}

fn jac(ic: &Ic<f64>) -> Jac {
    let m = ic.m;
    let m01 = m[0] + m[1];
    let (srho, slam) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01).sqrt());
    let [r0, r1, r2] = ic.r;
    let rho_t = [0, 1].map(|k| srho * (r1[k] - r0[k]));
    let lam_t = [0, 1].map(|k| slam * (r2[k] - (m[0] * r0[k] + m[1] * r1[k]) / m01));
    let p_rho = [0, 1].map(|k| ic.p[1][k] + m[1] / m01 * ic.p[2][k]);
    Jac {
        m,
        rho_t,
        lam_t,
        p_rho,
        p_lam: ic.p[2],
    }
}

/// A gauge element of dd_encode §3.3: `rᵢ' = s·R(θ)·Π·rᵢ + a`, `pᵢ' = s^(−1/2)·R(θ)·Π·pᵢ + mᵢ·u`. `mirror_p` false is
/// the half-mirror §3.3 warns of (`Π` forgotten on `p`), for the control.
#[derive(Debug, Clone, Copy)]
struct G {
    a: [f64; 2],
    u: [f64; 2],
    theta: f64,
    s: f64,
    mirror: bool,
    mirror_p: bool,
}

impl G {
    fn random(rng: &mut Rng) -> G {
        G {
            a: [rng.range(-5.0, 5.0), rng.range(-5.0, 5.0)],
            u: [rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)],
            theta: rng.range(-PI, PI),
            s: 4f64.powf(rng.range(-1.0, 1.0)),
            mirror: rng.unit() < 0.5,
            mirror_p: true,
        }
    }

    fn act(&self, x: &Ic<f64>) -> Ic<f64> {
        let (c, sn) = (self.theta.cos(), self.theta.sin());
        let turn = |v: [f64; 2], k: f64, mirror: bool| {
            let y = if mirror { -v[1] } else { v[1] };
            [k * (c * v[0] - sn * y), k * (sn * v[0] + c * y)]
        };
        let r =
            x.r.map(|v| turn(v, self.s, self.mirror))
                .map(|v| [v[0] + self.a[0], v[1] + self.a[1]]);
        let mut p =
            x.p.map(|v| turn(v, self.s.powf(-0.5), self.mirror && self.mirror_p));
        for (i, pi) in p.iter_mut().enumerate() {
            pi[0] += x.m[i] * self.u[0];
            pi[1] += x.m[i] * self.u[1];
        }
        Ic { m: x.m, r, p }
    }
}

fn enc64(x: &Ic<f64>) -> Encoded<f64> {
    encode::<Defaults, f64>(x)
}

fn enc32(x: &Ic<f64>) -> Encoded<f32> {
    encode::<Defaults, f32>(&narrow(x))
}

fn narrow(x: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: x.m.map(|v| v as f32),
        r: x.r.map(|v| v.map(|c| c as f32)),
        p: x.p.map(|v| v.map(|c| c as f32)),
    }
}

/// The larger of `a` and `b`, or NaN if either is (pitfalls §9): `f64::max` drops a NaN operand, so a fold through it
/// would let a NaN output pass the limit it is held to.
fn worse(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    worse((a[0] - b[0]).abs(), (a[1] - b[1]).abs())
}

/// The folds keep a NaN in any place, so a NaN output fails each `≤` limit they feed (pitfalls §9); `f64::max`'s fold
/// would drop it and pass. `fold` is the fold under test.
fn check_folds_keep_nan(fold: fn(f64, f64) -> f64) {
    for v in [[f64::NAN, 0.0], [0.0, f64::NAN]] {
        assert!(dist(v, [0.0, 0.0]).is_nan(), "dist({v:?}) dropped the NaN");
    }
    for xs in [
        [f64::NAN, 1.0, 2.0],
        [1.0, f64::NAN, 2.0],
        [1.0, 2.0, f64::NAN],
    ] {
        let folded = xs.iter().copied().fold(0.0f64, fold);
        assert!(
            folded.is_nan(),
            "the fold of {xs:?} dropped the NaN, so a limit would pass it: {folded}"
        );
    }
    assert_eq!([1.0, 3.0, 2.0].iter().copied().fold(0.0f64, fold), 3.0);
}

#[test]
fn encode_qa_m215_folds_keep_nan() {
    check_folds_keep_nan(worse);
}

negative_control!(
    encode_qa_m215_folds_keep_nan,
    "a worst-case fold through f64::max must drop the NaN and fail",
    expected = "dropped the NaN",
    check_folds_keep_nan(f64::max)
);

// ── REQ-ENC-007: each block recovers z, under the gauge ───────────────────────────────────────────────────────────

/// Encodes `g·forward(z)` for `n` random interior `z` and `g`, and checks that `z` comes back and that its decode is
/// `forward(z)` in physical units.
fn check_block_inverses(n: usize, seed: u64, half_mirror: bool) {
    let mut rng = Rng(seed);
    let mut worst = (0.0f64, 0.0f64);
    for case in 0..n {
        let z: [f64; 8] = [0; 8].map(|_| rng.range(-3.0, 3.0));
        let x = forward(z);
        let mut g = G::random(&mut rng);
        g.mirror_p = !half_mirror;
        let e = enc64(&g.act(&x));
        for (k, (got, want)) in e.z.z.iter().zip(z).enumerate() {
            let d = (got - want).abs();
            worst.0 = worse(worst.0, d);
            assert!(
                d <= Z_TOL_F64,
                "case {case}: z[{k}] does not recover: {got} for {want} (g = {g:?})"
            );
        }
        let back = decode::<Defaults, f64>(&Latent::new(e.z.z)).ic;
        for i in 0..3 {
            let d = worse(
                worse(dist(back.r[i], x.r[i]), dist(back.p[i], x.p[i])),
                (back.m[i] - x.m[i]).abs(),
            );
            worst.1 = worse(worst.1, d);
            assert!(
                d <= PHYS_TOL_F64,
                "case {case}: body {i}'s decode does not recover the canonical state: {d:e}"
            );
        }
    }
    println!(
        "block inverses: worst |Δz| {:e}, worst physical {:e}",
        worst.0, worst.1
    );
}

#[test]
fn encode_qa_m215_block_inverses() {
    check_block_inverses(4000, 0x2150_0001, false);
}

negative_control!(
    encode_qa_m215_block_inverses,
    "the half-mirror (Π on positions only, dd_encode §3.3) must fail the recovery",
    expected = "does not recover",
    check_block_inverses(4000, 0x2150_0001, true)
);

/// At f32, the same source recovers `z` from `forward(z)` (no gauge) to f32's rounding times the conditioning.
fn check_block_inverses_f32(n: usize, seed: u64, offset: f32) {
    // 2 · 101 · 1.19e-7 ≈ 2.4e-5 (as Z_TOL_F64's derivation, the gauge's 20 replaced by the canonical state's ~2);
    // ×40 headroom.
    const Z_TOL_F32: f64 = 1e-3;
    let mut rng = Rng(seed);
    let mut worst = 0.0f64;
    for case in 0..n {
        let z: [f64; 8] = [0; 8].map(|_| rng.range(-3.0, 3.0));
        let e = enc32(&forward(z));
        for (k, (got, want)) in e.z.z.iter().zip(z).enumerate() {
            let d = (f64::from(got + offset) - want).abs();
            worst = worse(worst, d);
            assert!(
                d <= Z_TOL_F32,
                "case {case}: f32 z[{k}] does not recover: {got} for {want}"
            );
        }
    }
    println!("block inverses f32: worst |Δz| {worst:e}");
}

#[test]
fn encode_qa_m215_block_inverses_f32() {
    check_block_inverses_f32(2000, 0x2150_0002, 0.0);
}

negative_control!(
    encode_qa_m215_block_inverses_f32,
    "an f32 z off by 2e-3 must fail the recovery",
    expected = "does not recover",
    check_block_inverses_f32(2000, 0x2150_0002, 2e-3)
);

// ── REQ-ENC-007 / REQ-ENC-014: the clamps, iff ────────────────────────────────────────────────────────────────────

/// The canonical state of interior masses and angles with Jacobi momenta `q = (p_ρx, p_ρy, p_λx, p_λy)`.
fn with_q(q: [f64; 4]) -> Ic<f64> {
    let base = forward([0.3, -0.4, 0.0, 0.0, 0.0, 0.0, 0.2, -0.1]);
    let j = jac(&base);
    from_jacobi(j.m, j.rho_t, j.lam_t, [q[0], q[1]], [q[2], q[3]])
}

/// Checks that control `k`'s clamp bit and `lookup_clamped` are set iff `clamped`, and a clamped `z` is finite, on the
/// clamp's image `logit(1 − ε)` with the input's sign.
fn check_clamp(e: &Encoded<f64>, k: usize, clamped: bool, what: &str) {
    let bit = e.report.clamped_controls >> k & 1 == 1;
    assert_eq!(
        bit, clamped,
        "{what}: control {k}'s clamp bit is {bit}, the clamp operated: {clamped}"
    );
    assert_eq!(
        e.report.lookup_clamped(),
        clamped,
        "{what}: lookup_clamped is {}, the clamp operated: {clamped}",
        e.report.lookup_clamped()
    );
    let z = e.z.z[k];
    assert!(z.is_finite(), "{what}: z[{k}] = {z}");
    if clamped && k < Z_MU1 {
        let image = logit(1.0 - EPS_Q);
        assert!(
            (z.abs() - image).abs() <= 1e-6 * image,
            "{what}: clamped z[{k}] = {z} is off the clamp's image ±{image}"
        );
    }
}

fn check_clamp_iff(flip_bit: bool) {
    // The `s` clamp operates iff s = ½(q/q_max + 1) ∉ [ε_q, 1 − ε_q] ⇔ |q| > q_max(1 − 2ε_q); probed 1e-9 relative
    // either side, clear of rounding.
    let edge = Q_MAX * (1.0 - 2.0 * EPS_Q);
    let probes = [
        (0.0, false),
        (1.0, false),
        (edge * (1.0 - 1e-9), false),
        (edge * (1.0 + 1e-9), true),
        (Q_MAX, true),
        (Q_MAX * (1.0 + 1e-12), true),
        (2.5, true),
        (1e3, true),
    ];
    for k in 0..4 {
        for (v, clamped) in probes {
            for sign in [1.0, -1.0] {
                let mut q = [0.3, -0.2, -0.4, 0.25];
                q[k] = sign * v;
                let mut e = enc64(&with_q(q));
                if flip_bit && v == 2.5 {
                    e.report.clamped_controls &= !(1 << (Z_Q0 + k));
                }
                check_clamp(&e, Z_Q0 + k, clamped, &format!("q[{k}] = {}", sign * v));
                if clamped {
                    assert_eq!(
                        e.z.z[Z_Q0 + k].signum(),
                        sign,
                        "q[{k}] = {}: the clamped z has the wrong sign",
                        sign * v
                    );
                }
            }
        }
    }
    // The mass: μ₁ = log(m₁/m₀) against (1 − ε_μ)μ_max (Part 3), m₂ held at m₀.
    for (mu, clamped) in [
        (0.0, false),
        (4.9, false),
        (MU_MAX * (1.0 - EPS_MU) * (1.0 - 1e-9), false),
        (MU_MAX * (1.0 - EPS_MU) * (1.0 + 1e-9), true),
        (5.5, true),
        (-5.5, true),
    ] {
        let w = [1.0, f64::exp(mu), 1.0];
        let t: f64 = w.iter().sum();
        let j = jac(&with_q([0.3, -0.2, -0.4, 0.25]));
        let x = from_jacobi(w.map(|v| v / t), j.rho_t, j.lam_t, j.p_rho, j.p_lam);
        let e = enc64(&x);
        check_clamp(&e, Z_MU1, clamped, &format!("μ₁ = {mu}"));
        if !clamped {
            assert!(
                (e.z.z[Z_MU1] - (mu / MU_MAX).atanh()).abs() <= 1e-9,
                "μ₁ = {mu}: z_μ1 = {} is not artanh(μ₁/μ_max)",
                e.z.z[Z_MU1]
            );
        }
    }
}

#[test]
fn encode_qa_m215_clamp_iff() {
    check_clamp_iff(false);
}

negative_control!(
    encode_qa_m215_clamp_iff,
    "a |q| > q_max input whose clamp bit is cleared must fail",
    expected = "the clamp operated: true",
    check_clamp_iff(true)
);

// ── REQ-ENC-007: β alone (R-437, R-438) ───────────────────────────────────────────────────────────────────────────

/// f32's π̂: the image of π rounded toward zero (R-417).
fn pi_hat_f32() -> f32 {
    let nearest = PI as f32;
    if f64::from(nearest) > PI {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

fn check_beta_quotient(beta64: fn([f64; 2]) -> f64) {
    let mut rng = Rng(0x2150_0003);
    let pairs: Vec<[f64; 2]> = (0..20000)
        .map(|i| {
            let mag = 10f64.powf(rng.range(-14.0, 2.0));
            let x = rng.range(-1.0, 1.0) * mag;
            let y = match i % 5 {
                0 => 0.0,
                1 => -0.0,
                2 => rng.range(-1.0, 1.0) * DELTA_LAMBDA,
                _ => rng.range(-1.0, 1.0) * mag,
            };
            [x, y]
        })
        .collect();
    for [x, y] in pairs.iter().copied() {
        let (b, bm) = (beta64([x, y]), beta64([x, -y]));
        assert!(
            b.to_bits() == bm.to_bits(),
            "β({x:e}, {y:e}) = {b} but β({x:e}, {:e}) = {bm}: not the mirror's quotient",
            -y
        );
        assert!((0.0..=PI).contains(&b), "β({x:e}, {y:e}) = {b} ∉ [0, π]");
        if y == 0.0 && x < 0.0 {
            assert!(b == PI, "β({x:e}, {y:e}) = {b}, not π̂ on the −x half-axis");
        }
        let b32 = beta([x as f32, y as f32]);
        let bm32 = beta([x as f32, -(y as f32)]);
        assert!(
            b32.to_bits() == bm32.to_bits(),
            "f32 β({x:e}, {y:e}) = {b32} but {bm32} mirrored: not the mirror's quotient"
        );
        assert!(
            (0.0..=pi_hat_f32()).contains(&b32),
            "f32 β({x:e}, {y:e}) = {b32} ∉ [0, π̂]"
        );
        if (y as f32) == 0.0 && (x as f32) < 0.0 {
            assert!(
                b32.to_bits() == pi_hat_f32().to_bits(),
                "f32 β({x:e}, {y:e}) = {b32}, not π̂ on the −x half-axis"
            );
        }
    }
    for x in [0.0, -0.0] {
        for y in [0.0, -0.0] {
            let b = beta64([x, y]);
            assert!(
                b.to_bits() == 0.0f64.to_bits(),
                "β at the pole ({x:?}, {y:?}) = {b:?}, not R-437's 0"
            );
            let b32 = beta([x as f32, y as f32]);
            assert!(
                b32.to_bits() == 0.0f32.to_bits(),
                "f32 β at the pole ({x:?}, {y:?}) = {b32:?}, not R-437's 0"
            );
        }
    }
}

#[test]
fn encode_qa_m215_beta_quotient() {
    check_beta_quotient(beta::<f64>);
}

negative_control!(
    encode_qa_m215_beta_quotient,
    "β read as atan2(λ̃_y, λ̃_x), without |·|, must fail",
    expected = "not the mirror's quotient",
    check_beta_quotient(|[x, y]| y.atan2(x))
);

// ── REQ-ENC-007: dd_encode §5 test 10 ─────────────────────────────────────────────────────────────────────────────

/// Euler's collinear central configuration of masses `m` with body `order[1]` in the middle: positions on a line,
/// `x[order[0]] = 0`, `x[order[1]] = 1`, `x[order[2]] = 1 + t`, `t` solving `(a₃ − a₂)/t = a₂ − a₁` (accelerations
/// proportional to the distance from the CoM) by bisection.
fn euler(m: [f64; 3], order: [usize; 3]) -> [f64; 3] {
    let pos = |t: f64| {
        let mut x = [0.0; 3];
        x[order[1]] = 1.0;
        x[order[2]] = 1.0 + t;
        x
    };
    let acc = |x: [f64; 3], i: usize| -> f64 {
        (0..3)
            .filter(|&j| j != i)
            .map(|j| m[j] * (x[j] - x[i]).signum() / (x[j] - x[i]).powi(2))
            .sum()
    };
    let g = |t: f64| {
        let x = pos(t);
        let (a1, a2, a3) = (acc(x, order[0]), acc(x, order[1]), acc(x, order[2]));
        (a3 - a2) / t - (a2 - a1)
    };
    let (mut lo, mut hi) = (1e-6, 1e6);
    assert!(
        g(lo).signum() != g(hi).signum(),
        "Euler's root is not bracketed"
    );
    for _ in 0..200 {
        let mid = (lo * hi).sqrt();
        if g(mid).signum() == g(lo).signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    pos(lo)
}

/// The canonical Jacobi coordinates (`ρ̃` on `+x`, `I = 1`) of the collinear positions `x`.
fn collinear_jacobi(m: [f64; 3], x: [f64; 3]) -> (f64, f64) {
    let m01 = m[0] + m[1];
    let rho = x[1] - x[0];
    let lam = x[2] - (m[0] * x[0] + m[1] * x[1]) / m01;
    let (rt, lt) = ((m[0] * m[1] / m01).sqrt() * rho, (m[2] * m01).sqrt() * lam);
    let i = (rt * rt + lt * lt).sqrt();
    // ρ̃ to +x: a half turn, if needed, flips both.
    let s = rho.signum() / i;
    (rt * s, lt * s)
}

const P_RHO: [f64; 2] = [0.3, -0.2];
const P_LAM: [f64; 2] = [-0.4, 0.25];

fn check_collinear(m: [f64; 3], tamper: bool) {
    let ys = [-1e-13, -0.0, 0.0, 1e-13];
    let mut sides = (0, 0);
    for middle in 0..3 {
        let order = match middle {
            0 => [1, 0, 2],
            1 => [0, 1, 2],
            _ => [0, 2, 1],
        };
        let (rt, lx) = collinear_jacobi(m, euler(m, order));
        assert!(
            lx.abs() > 1e-3,
            "landmark {middle}: λ̃_x = {lx:e}, on the pole; choose other masses"
        );
        let lam_norm = lx.abs();
        let mut zs: Vec<[f64; 8]> = Vec::new();
        for y in ys {
            let x = from_jacobi(m, [rt, 0.0], [lx, y], P_RHO, P_LAM);
            let mut e = enc64(&x);
            if tamper {
                e.blocks.beta = y.atan2(lx);
            }
            let b = e.blocks.beta;
            let what = format!("landmark {middle} (λ̃_x = {lx:.4}), λ̃_y = {y:?}");
            if lx < 0.0 {
                // β = atan2(|λ̃_y|, λ̃_x) is π − |λ̃_y|/‖λ̃‖ to first order.
                assert!(
                    (b - PI).abs() <= y.abs() / lam_norm + ROUND_F64,
                    "{what}: β = {b}, not π (body 2 on the −x side)"
                );
                if y == 0.0 {
                    assert!(b == PI, "{what}: β = {b}, not π̂ by the select");
                }
            } else {
                assert!(
                    (0.0..=y.abs() / lam_norm + ROUND_F64).contains(&b),
                    "{what}: β = {b}, not 0 (body 2 on the +x side)"
                );
            }
            assert!(e.report.lookup_clamped(), "{what}: lookup_clamped not set");
            assert!(
                e.report.clamped_controls >> Z_BETA & 1 == 1,
                "{what}: s_β not reported clamped"
            );
            assert!(!e.report.mirrored, "{what}: mirrored inside the deadband");
            zs.push(e.z.z);
            // The round trip, in mass-weighted Jacobi coordinates.
            let j = jac(&decode::<Defaults, f64>(&Latent::new(e.z.z)).ic);
            assert!(
                j.lam_t[0].signum() == lx.signum(),
                "{what}: the decode moved body 2 to the other side: λ̃_x = {}",
                j.lam_t[0]
            );
            let dy = (j.lam_t[1] - y).abs();
            assert!(
                dy <= PI * EPS_Z * lam_norm + y.abs() + ROUND_F64,
                "{what}: |Δλ̃_y| = {dy:e} over π·ε_z·‖λ̃‖ + |λ̃_y|"
            );
            let dx = (j.lam_t[0] - lx).abs();
            assert!(
                dx <= lam_norm * (1.0 - (PI * EPS_Z).cos()) + ROUND_F64,
                "{what}: |Δλ̃_x| = {dx:e} over ‖λ̃‖(1 − cos(π·ε_z))"
            );
            let rest = (0..3)
                .map(|i| (j.m[i] - m[i]).abs())
                .chain([
                    dist(j.rho_t, [rt, 0.0]),
                    dist(j.p_rho, P_RHO),
                    dist(j.p_lam, P_LAM),
                ])
                .fold(0.0f64, worse);
            assert!(
                rest <= ROUND_F64,
                "{what}: the masses, ρ̃ or the momenta move by {rest:e}"
            );
        }
        for (k, z) in zs.iter().enumerate().skip(1) {
            for c in 0..8 {
                assert!(
                    (z[c] - zs[0][c]).abs() <= ROUND_F64 * (1.0 + z[c].abs()),
                    "landmark {middle}: λ̃_y = {:?} and {:?} encode to different z[{c}]: {} and {}",
                    ys[0],
                    ys[k],
                    zs[0][c],
                    z[c]
                );
            }
        }
        if lx < 0.0 {
            sides.0 += 1;
        } else {
            sides.1 += 1;
        }
    }
    assert!(
        sides.0 > 0 && sides.1 > 0,
        "the landmarks cover only one side: {sides:?}"
    );
}

/// The pole landmark: `m₀ = m₁` with body 2 at the inner pair's centre of mass, `λ̃ = 0` exactly. Dyadic masses and
/// symmetric positions keep every product exact, so `λ̃_x` is 0 whatever `C` rounds.
fn check_pole_landmark(tamper: bool) {
    let m = [0.25, 0.25, 0.5];
    for y in [-1e-13, -0.0, 0.0, 1e-13] {
        // λ̃_y = √μ_λ·λ_y, μ_λ = m₂M₀₁ = 1/4.
        let x = Ic {
            m,
            r: [[-1.0, 0.0], [1.0, 0.0], [0.0, 2.0 * y]],
            p: [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]],
        };
        let mut e = enc64(&x);
        if tamper && y == 0.0 {
            // atan2(−0, −0) = −π: the value R-437's select exists to avoid.
            e.blocks.beta = (-0.0f64).atan2(-0.0);
        }
        let b = e.blocks.beta;
        if y == 0.0 {
            assert!(
                b.to_bits() == 0.0f64.to_bits(),
                "pole, λ̃_y = {y:?}: β = {b:?}, not R-437's 0 by the select"
            );
        } else {
            assert!(
                (b - FRAC_PI_2).abs() <= 4.0 * f64::EPSILON,
                "pole, λ̃_y = {y:?}: β = {b}, not π/2"
            );
        }
        assert!(
            e.report.lookup_clamped(),
            "pole, λ̃_y = {y:?}: lookup_clamped not set"
        );
        assert!(
            e.z.z.iter().all(|v| v.is_finite()),
            "pole, λ̃_y = {y:?}: z = {:?}",
            e.z.z
        );
    }
}

#[test]
fn encode_qa_m215_collinear_beta() {
    check_collinear([0.2, 0.3, 0.5], false);
}

negative_control!(
    encode_qa_m215_collinear_beta,
    "β read as atan2(λ̃_y, λ̃_x) must fail on the −x side at λ̃_y < 0",
    expected = "not π (body 2 on the −x side)",
    check_collinear([0.2, 0.3, 0.5], true)
);

negative_control!(
    encode_qa_m215_collinear_beta_pole,
    "β taken from atan2(−0, −0) at the pole must fail",
    expected = "not R-437's 0 by the select",
    check_pole_landmark(true)
);

#[test]
fn encode_qa_m215_collinear_beta_pole() {
    check_pole_landmark(false);
}

// ── REQ-DEC-014: the exact α-poles ────────────────────────────────────────────────────────────────────────────────

const ZEROS: [f64; 2] = [0.0, -0.0];

/// `‖λ̃‖ = 0` exactly: `m₀ = m₁`, body 2 at the inner pair's centre, every sign of the zero components.
fn lambda_pole_states() -> Vec<Ic<f64>> {
    let mut v = Vec::new();
    for a in ZEROS {
        for b in ZEROS {
            for c in ZEROS {
                v.push(Ic {
                    m: [0.25, 0.25, 0.5],
                    r: [[-1.0, a], [1.0, b], [c, a]],
                    p: [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]],
                });
            }
        }
    }
    v
}

/// `‖ρ̃‖ = 0` exactly: bodies 0 and 1 coincide, every sign of the zero components of `ρ`.
fn rho_pole_states() -> Vec<Ic<f64>> {
    let mut v = Vec::new();
    for a in ZEROS {
        for b in ZEROS {
            for c in ZEROS {
                v.push(Ic {
                    m: [0.25, 0.25, 0.5],
                    r: [[-1.0, a], [-1.0, b], [1.0, c]],
                    p: [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]],
                });
            }
        }
    }
    v
}

fn bits64(e: &Encoded<f64>) -> Vec<u64> {
    let mut w: Vec<u64> = e.z.z.iter().map(|v| v.to_bits()).collect();
    w.extend([e.blocks.alpha.to_bits(), e.blocks.beta.to_bits()]);
    w.extend([
        u64::from(e.report.lookup_clamped()),
        u64::from(e.report.mirrored),
        u64::from(e.report.clamped_controls),
    ]);
    w
}

fn bits32(e: &Encoded<f32>) -> Vec<u32> {
    let mut w: Vec<u32> = e.z.z.iter().map(|v| v.to_bits()).collect();
    w.extend([e.blocks.alpha.to_bits(), e.blocks.beta.to_bits()]);
    w.extend([
        u32::from(e.report.lookup_clamped()),
        u32::from(e.report.mirrored),
        e.report.clamped_controls,
    ]);
    w
}

/// Checks one pole family: `alpha` its α, `rho_pole` whether `φ` is the select's.
fn check_poles(name: &str, states: &[Ic<f64>], alpha: f64, rho_pole: bool, nan_at: Option<usize>) {
    let first64 = bits64(&enc64(&states[0]));
    let first32 = bits32(&enc32(&states[0]));
    for (i, x) in states.iter().enumerate() {
        let what = format!("{name} state {i} ({:?})", x.r);
        let mut e = enc64(x);
        if nan_at == Some(i) {
            e.z.z[Z_ALPHA] = f64::NAN;
        }
        let f = enc32(x);
        let finite =
            e.z.z
                .iter()
                .chain([&e.blocks.alpha, &e.blocks.beta])
                .all(|v| v.is_finite())
                && f.z
                    .z
                    .iter()
                    .chain([&f.blocks.alpha, &f.blocks.beta])
                    .all(|v| v.is_finite());
        assert!(
            finite,
            "{what}: a NaN or infinity at the pole: {e:?} / {f:?}"
        );
        assert_eq!(
            bits64(&e),
            bits64(&enc64(x)),
            "{what}: f64 not identical on repeat"
        );
        assert_eq!(
            bits32(&f),
            bits32(&enc32(x)),
            "{what}: f32 not identical on repeat"
        );
        assert_eq!(
            bits64(&e),
            first64,
            "{what}: f64 depends on the zeros' signs"
        );
        assert_eq!(
            bits32(&f),
            first32,
            "{what}: f32 depends on the zeros' signs"
        );
        assert!(
            e.report.lookup_clamped(),
            "{what}: f64 lookup_clamped not set"
        );
        assert!(
            f.report.lookup_clamped(),
            "{what}: f32 lookup_clamped not set"
        );
        assert!(
            e.report.clamped_controls >> Z_ALPHA & 1 == 1
                && f.report.clamped_controls >> Z_ALPHA & 1 == 1,
            "{what}: s_α at the pole is not reported clamped"
        );
        assert!(
            (e.blocks.alpha - alpha).abs() <= 4.0 * f64::EPSILON,
            "{what}: α = {}, not {alpha}",
            e.blocks.alpha
        );
        assert!(
            (f64::from(f.blocks.alpha) - alpha).abs() <= 4.0 * f64::from(f32::EPSILON),
            "{what}: f32 α = {}, not {alpha}",
            f.blocks.alpha
        );
        // The cross-precision check, in physical units and on the notices (Part 4).
        assert_eq!(
            (
                e.report.lookup_clamped(),
                e.report.mirrored,
                e.report.clamped_controls,
                e.report.coincident
            ),
            (
                f.report.lookup_clamped(),
                f.report.mirrored,
                f.report.clamped_controls,
                f.report.coincident
            ),
            "{what}: the notices differ between f64 and f32"
        );
        assert!(
            (e.blocks.beta - f64::from(f.blocks.beta)).abs() <= 4.0 * f64::from(f32::EPSILON),
            "{what}: β differs between f64 ({}) and f32 ({})",
            e.blocks.beta,
            f.blocks.beta
        );
        if rho_pole {
            assert!(e.report.coincident, "{what}: ρ = 0 not reported (R-13)");
            assert!(
                e.canonical.phi.to_bits() == 0.0f64.to_bits()
                    && f.canonical.phi.to_bits() == 0.0f32.to_bits(),
                "{what}: φ = {:?} / {:?}, not R-437's 0",
                e.canonical.phi,
                f.canonical.phi
            );
        } else {
            assert!(
                e.blocks.beta.to_bits() == 0.0f64.to_bits()
                    && f.blocks.beta.to_bits() == 0.0f32.to_bits(),
                "{what}: β = {:?} / {:?}, not R-437's 0",
                e.blocks.beta,
                f.blocks.beta
            );
        }
    }
}

#[test]
fn encode_qa_m215_exact_poles() {
    check_poles("λ̃ = 0", &lambda_pole_states(), 0.0, false, None);
    check_poles("ρ̃ = 0", &rho_pole_states(), FRAC_PI_2, true, None);
}

negative_control!(
    encode_qa_m215_exact_poles,
    "a NaN in one pole state's z must fail",
    expected = "a NaN or infinity at the pole",
    check_poles("λ̃ = 0", &lambda_pole_states(), 0.0, false, Some(5))
);

// ── REQ-ENC-014: the notices ──────────────────────────────────────────────────────────────────────────────────────

fn check_notices(forget_mirror: bool) {
    let mut rng = Rng(0x2150_0004);
    for case in 0..500 {
        let z: [f64; 8] = [0; 8].map(|_| rng.range(-3.0, 3.0));
        let x = forward(z);
        let e = enc64(&x);
        assert!(
            !e.report.mirrored && !e.report.lookup_clamped(),
            "case {case}: a canonical input raised a notice: {:?}",
            e.report
        );
        let mirrored = Ic {
            m: x.m,
            r: x.r.map(|v| [v[0], -v[1]]),
            p: x.p.map(|v| [v[0], -v[1]]),
        };
        let mut e = enc64(&mirrored);
        if forget_mirror {
            e.report.mirrored = false;
        }
        assert!(
            e.report.lookup_mirrored() && !e.report.lookup_clamped(),
            "case {case}: a mirrored input did not raise lookup_mirrored alone: {:?}",
            e.report
        );
    }
    let e = enc64(&with_q([2.5, -0.2, -0.4, 0.25]));
    assert!(
        e.report.lookup_clamped() && !e.report.mirrored,
        "|q| > q_max did not raise lookup_clamped alone: {:?}",
        e.report
    );
}

#[test]
fn encode_qa_m215_notices() {
    check_notices(false);
}

negative_control!(
    encode_qa_m215_notices,
    "a mirror taken but not reported must fail",
    expected = "did not raise lookup_mirrored alone",
    check_notices(true)
);

// ── REQ-SYS-015: no hand-written WGSL decode or encode ────────────────────────────────────────────────────────────

/// The decode's and encode's link formulae and constants, as a WGSL transcription would name them.
const DECODE_TOKENS: [&str; 10] = [
    "mu_max",
    "q_max",
    "eps_mu",
    "eps_z",
    "eps_q",
    "alpha_min",
    "artanh",
    "atanh",
    "logit",
    "softmax",
];

/// The tokens of `DECODE_TOKENS` that `text` carries, case-insensitively.
fn decode_tokens(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    DECODE_TOKENS
        .iter()
        .copied()
        .filter(|t| lower.contains(t))
        .collect()
}

fn wgsl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        if path.is_dir() {
            wgsl_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "wgsl") {
            out.push(path);
        }
    }
}

fn check_no_wgsl_decode(files: &[(String, String)]) {
    let hits: Vec<String> = files
        .iter()
        .filter_map(|(path, text)| {
            let t = decode_tokens(text);
            (!t.is_empty()).then(|| format!("{path}: {t:?}"))
        })
        .collect();
    assert!(
        hits.is_empty(),
        "a checked-in .wgsl file carries decode/encode formulae: {hits:?}"
    );
}

#[test]
fn encode_qa_m215_no_wgsl_decode() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for top in ["crates", "xtask", "fixtures"] {
        wgsl_files(&root.join(top), &mut files);
    }
    assert!(
        files.len() > 50,
        "the scan found only {} .wgsl files: it is not reading the tree",
        files.len()
    );
    let texts: Vec<(String, String)> = files
        .iter()
        .map(|p| (p.display().to_string(), std::fs::read_to_string(p).unwrap()))
        .collect();
    check_no_wgsl_decode(&texts);
}

negative_control!(
    encode_qa_m215_no_wgsl_decode,
    "a hand-written WGSL mass inverse must be found",
    expected = "carries decode/encode formulae",
    check_no_wgsl_decode(&[(
        "frag/encode.wgsl".into(),
        "fn z_mu(m1: f32, m0: f32) -> f32 { return atanh(log(m1 / m0) / MU_MAX); }".into()
    )])
);
