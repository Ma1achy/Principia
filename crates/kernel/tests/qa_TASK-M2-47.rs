//! QA tests for TASK-M2-47, written from the requirements and R-547, not from the implementation: `C`'s step 0
//! (`kernel::canon::canonicalise`, `kernel::canon::scale_step`) and its notices (`kernel::encode::encode`), at f64 and
//! f32 on the CPU. The GPU-f32 side is `engine`'s `qa_TASK-M2-47_gpu`.
//!
//! Every expected value comes from a construction made here, not from the code's own arithmetic:
//! - **Exact-`I` states.** Two families whose `I_in` is a known float, every step of `Σmᵢ‖rᵢ − R_com‖²` exact in any
//!   correct evaluation (powers of two, CoM exactly 0): masses `(¼, ¼, ½)` with `r₀ = (−d−f, −g−h)`,
//!   `r₁ = (−d+f, −g+h)`, `r₂ = (d, g)`, so `I_in = d² + g² + f²/2 + h²/2`, which reaches `1 + τ_I` exactly and
//!   `1 + τ_I + 2ε`; and masses `((1−μ)/2, (1−μ)/2, μ)` with `r₀ = (1, 0)`, `r₁ = (−1, 0)`, `r₂ = 0`, so
//!   `I_in = 1 − μ` exactly, reaching `1 − τ_I` and `1 − τ_I − ε/2`.
//! - **Scaled states.** A generic state `x` and `s·x` (`r ← s·r`, `p ← p/√s`, `s` a power of ten, not of two), whose
//!   canonical form is `C(x)`'s (the scale gauge's orbit, R-547's RQ-360: T1 holds for scalings everywhere).
//! - **Exact differences.** f32 inputs whose pair differences are subnormal, against their differences taken in f64,
//!   where they are exact and normal.
//!
//! The tests:
//! - REQ-ENC-006, REQ-ENC-014, REQ-ENC-035 (`qa_m247_skip_on_canonical_decodes`): canonical-frame decodes of fuzzed `z`
//!   leave step 0 skipped: `λ = 1` exactly, step 0's `r` and `p` bit for bit as they entered it, no `lookup_rescaled`,
//!   and encode raises no notice at all; the measured `|I_in − 1|` is printed against `τ_I`.
//! - REQ-ENC-006, REQ-ENC-014 (`qa_m247_tau_edge`): on the exact-`I` states, step 0 is skipped at `|I_in − 1| = τ_I`
//!   exactly, on either side of 1, and inside it; it runs, raising `lookup_rescaled` with `λ = I_in^{−1/2}`, at
//!   `1 + τ_I + 2ε` and `1 − τ_I − ε/2`, the state leaving `C` at `I = 1`; encode's `lookup_rescaled` agrees.
//! - REQ-ENC-006 (`qa_m247_range`): states with the bodies apart at scales from below to above `I_in`'s float range
//!   (f64 `1e-300` to `1e300`; f32 `1e-37` to `1e37`) leave `C` at `I = 1` to rounding, every output finite,
//!   `lookup_rescaled` and no `lookup_clamped`, `λ = 1/(s√I₀)`, and the same canonical state as the unscaled input,
//!   with `E_canon = √I_in·E_in` and `L_z,canon = L_z,in/I_in^{1/4}`.
//! - REQ-ENC-006, REQ-ENC-014 (`qa_m247_coincident`): three coincident bodies, at the origin and off it, small and far,
//!   with nonzero momenta: divisor 1, `λ = 1`, `p = 0`, `lookup_clamped`, `φ = +0` by its bits, no `lookup_rescaled`,
//!   every output finite (no `1/0` formed); encode reports the coincident pair and no rescale.
//! - REQ-DEC-002 (`qa_m247_subnormal_differences`): f32 states at coordinates near `1e-36` whose inner pair is a
//!   subnormal apart: `ρ ≠ 0`, so no `lookup_clamped`; `φ` the exact difference's angle; the mirror the exact
//!   difference's decision; every output finite at `I = 1`.
//!
//! **Tolerances** (applied per R-369: a test's bound, not a constant of the system). `τ_I` is the register's, read as
//! `IeeeBits::TAU_I` (REQ-ENC-035, provisional until M2). `I = 1` after step 0: 8 eps (each `rᵢ/√I_in` rounded by a
//! few half-ulps, `I` of degree 2 in them). `λ`: 4 eps. The scaled state against the unscaled one: 32 eps on the state
//! (measured up to 8), whose input roundings differ by an ulp per coordinate, through the rotation's conditioning on
//! these states; 16 eps on `E` and `L_z` (measured up to 3.5). `φ`
//! against `atan2` of the exact difference: 4 eps. Each test prints its worst case.
//!
//! **Domain** (pitfalls §10): `C` alone on states held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use kernel::canon::{canonicalise, scale_step, Canonical, Scaled};
use kernel::decode::{decode, Ic, Latent};
use kernel::encode::encode;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

/// δ_λ, dd_decoder §3.3 and R-82.
const DELTA: f64 = 1e-12;

/// A precision the shared source runs at.
trait P: LinkReal + PayloadReal + IeeeBits + Into<f64> + std::fmt::Debug {
    /// The bits of `self`, widened.
    fn bits(self) -> u64;
}
impl P for f32 {
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}
impl P for f64 {
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

fn eps<R: P>() -> f64 {
    R::epsilon().into()
}

fn of<R: P>(v: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(v).expect("finite")
}

fn narrow<R: P>(x: &Ic<f64>) -> Ic<R> {
    Ic {
        m: x.m.map(of),
        r: x.r.map(|v| v.map(of)),
        p: x.p.map(|v| v.map(of)),
    }
}

fn widen<R: P>(x: &Ic<R>) -> Ic<f64> {
    Ic {
        m: x.m.map(Into::into),
        r: x.r.map(|v| v.map(Into::into)),
        p: x.p.map(|v| v.map(Into::into)),
    }
}

/// `Σmᵢ‖rᵢ − R_com‖²`, in f64.
fn inertia64(x: &Ic<f64>) -> f64 {
    let t: f64 = x.m.iter().sum();
    let c = [0, 1].map(|k| (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>() / t);
    (0..3)
        .map(|i| x.m[i] * ((x.r[i][0] - c[0]).powi(2) + (x.r[i][1] - c[1]).powi(2)))
        .sum()
}

/// `E = Σ‖pᵢ‖²/2mᵢ − Σ mᵢmⱼ/rᵢⱼ` (G = 1), in f64.
fn energy(x: &Ic<f64>) -> f64 {
    let kin: f64 = (0..3)
        .map(|i| (x.p[i][0].powi(2) + x.p[i][1].powi(2)) / (2.0 * x.m[i]))
        .sum();
    let pot: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            x.m[i] * x.m[j]
                / ((x.r[i][0] - x.r[j][0]).powi(2) + (x.r[i][1] - x.r[j][1]).powi(2)).sqrt()
        })
        .sum();
    kin - pot
}

/// `L_z = Σ rᵢ × pᵢ`, in f64.
fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

fn finite_state(x: &Ic<f64>) -> bool {
    x.r.iter()
        .chain(x.p.iter())
        .all(|v| v[0].is_finite() && v[1].is_finite())
}

/// Every output `C` returns is finite: `r`, `p`, `λ`, `φ`, `λ̃_y`.
fn check_finite<R: P>(c: &Canonical<R>, at: &str) {
    let w = widen(&c.ic);
    let lam: f64 = c.lambda.into();
    let phi: f64 = c.phi.into();
    let lty: f64 = c.lambda_t_y.into();
    assert!(
        finite_state(&w) && lam.is_finite() && phi.is_finite() && lty.is_finite(),
        "{at}: an output is not finite: {w:?}, λ = {lam}, φ = {phi}, λ̃_y = {lty}"
    );
}

// ── qa_m247_skip_on_canonical_decodes (REQ-ENC-006, REQ-ENC-014, REQ-ENC-035) ───────────────────────────────────

/// Step 0 skipped on `x`: `λ = 1` exactly, no `lookup_rescaled`, and step 0's `r` and `p` bit for bit its inputs
/// (the CoM-frame positions and the momenta as given).
fn check_skipped<R: P>(x: &Ic<R>, step: &Scaled<R>, c: &Canonical<R>, at: &str) {
    let lam: f64 = c.lambda.into();
    let step_lam: f64 = step.lambda.into();
    assert!(
        lam == 1.0 && step_lam == 1.0 && !c.rescaled && !step.rescaled,
        "{at}: step 0 ran on a state within τ_I: λ = {lam} (step {step_lam}), lookup_rescaled = {}",
        c.rescaled
    );
    check_unchanged(x, step, at);
}

/// Step 0's `r` and `p` are bit for bit the CoM frame's positions and `x`'s momenta.
fn check_unchanged<R: P>(x: &Ic<R>, step: &Scaled<R>, at: &str) {
    for i in 0..3 {
        for k in 0..2 {
            assert!(
                step.r[i][k].bits() == step.centred[i][k].bits()
                    && step.p[i][k].bits() == x.p[i][k].bits(),
                "{at}: step 0 changed the state it should have skipped: body {i}, axis {k}: r {:?} → {:?}, \
                 p {:?} → {:?}",
                step.centred[i][k],
                step.r[i][k],
                x.p[i][k],
                step.p[i][k]
            );
        }
    }
}

/// Encode's three notices on `x`: `(lookup_clamped, lookup_mirrored, lookup_rescaled)`.
fn notices<R: P>(x: &Ic<R>) -> (bool, bool, bool) {
    let e = encode::<Defaults, R>(x);
    (
        e.report.lookup_clamped(),
        e.report.lookup_mirrored(),
        e.report.lookup_rescaled(),
    )
}

fn canonical_decodes<R: P>(n: usize, seed: u64) -> Vec<Ic<R>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|j| {
            // Every eighth draw takes some controls far out, toward saturation.
            let span = if j % 8 == 0 { 40.0 } else { 4.0 };
            let z = [0; 8].map(|_| of::<R>(f64::from(rng.uniform(-span, span))));
            decode::<Defaults, R>(&Latent::new(z)).ic
        })
        .collect()
}

fn skip_at<R: P>(n: usize) -> f64 {
    let tau: f64 = R::TAU_I.into();
    let mut worst = 0.0f64;
    for (j, x) in canonical_decodes::<R>(n, 0x4747).iter().enumerate() {
        let at = format!("decode {j}");
        let c = canonicalise(x);
        check_skipped(x, &scale_step(x), &c, &at);
        let i_in: f64 = c.i_in.into();
        worst = worst.max((i_in - 1.0).abs() / eps::<R>());
        assert!(
            (i_in - 1.0).abs() <= tau,
            "{at}: |I_in − 1| = {}",
            (i_in - 1.0).abs()
        );
        // An interior decode raises no notice at all; a saturated one may raise a control's `lookup_clamped`
        // (inverse-encode Part 3), never `lookup_rescaled`.
        let got = notices(x);
        let want = if j % 8 == 0 {
            (got.0, false, false)
        } else {
            (false, false, false)
        };
        assert_eq!(
            got, want,
            "{at}: a canonical input raised a notice (lookup_clamped, lookup_mirrored, lookup_rescaled)"
        );
    }
    worst
}

#[test]
fn qa_m247_skip_on_canonical_decodes() {
    let w64 = skip_at::<f64>(4000);
    let w32 = skip_at::<f32>(4000);
    let t64: f64 = f64::TAU_I / f64::EPSILON;
    let t32: f64 = f64::from(f32::TAU_I / f32::EPSILON);
    println!(
        "qa_m247_skip_on_canonical_decodes: 4000 decodes each, worst |I_in − 1| = {w64} eps (f64, τ_I = {t64} eps), \
         {w32} eps (f32, τ_I = {t32} eps); every one skipped bit for bit, no notice"
    );
}

negative_control!(
    qa_m247_skip_on_canonical_decodes,
    "a step 0 that always runs (step 0's map, r/√I_in and I_in^{1/4}·p, applied to the CoM frame) changes a \
     canonical decode, so the bit-for-bit check must fail on it",
    expected = "step 0 changed the state it should have skipped",
    {
        for x in canonical_decodes::<f32>(64, 0x4747) {
            let step = scale_step(&x);
            let (r, p, _) = kernel::canon::similarity(step.centred, x.p, step.i_in);
            let always = Scaled { r, p, ..step };
            check_unchanged(&x, &always, "always run");
        }
    }
);

// ── qa_m247_tau_edge (REQ-ENC-006, REQ-ENC-014) ─────────────────────────────────────────────────────────────────

/// `log₂ v` for `v` a power of two.
fn log2_exact(v: f64) -> i32 {
    let e = v.log2().round() as i32;
    assert!(2f64.powi(e) == v, "{v} is not a power of two");
    e
}

/// The first family at `I_in = 1 + Σ 2^{eₖ}`, each `eₖ` given, at most one even and two odd: `d = 1`, an even `e` as
/// `g = 2^{e/2}`, the odd ones as `f`, `h` with `f²/2 = 2^e`.
fn above<R: P>(exps: &[i32]) -> Ic<R> {
    let (mut f, mut g, mut h) = (0.0f64, 0.0f64, 0.0f64);
    for &e in exps {
        if e % 2 == 0 {
            assert!(g == 0.0, "two even exponents");
            g = 2f64.powi(e / 2);
        } else if f == 0.0 {
            f = 2f64.powi((e + 1) / 2);
        } else {
            assert!(h == 0.0, "three odd exponents");
            h = 2f64.powi((e + 1) / 2);
        }
    }
    let x = Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-1.0 - f, -g - h], [-1.0 + f, -g + h], [1.0, g]],
        p: [[0.3, -0.2], [-0.1, 0.4], [-0.1, -0.1]],
    };
    let n = narrow::<R>(&x);
    assert!(
        widen(&n).r == x.r,
        "the construction is not exact at this precision"
    );
    n
}

/// The second family at `I_in = 1 − μ`.
fn below<R: P>(mu: f64) -> Ic<R> {
    let x = Ic {
        m: [(1.0 - mu) / 2.0, (1.0 - mu) / 2.0, mu],
        r: [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]],
        p: [[0.3, -0.2], [-0.1, 0.4], [-0.2, -0.2]],
    };
    let n = narrow::<R>(&x);
    assert!(
        widen(&n).m == x.m,
        "the construction is not exact at this precision"
    );
    n
}

/// The edge states, each with its exact `I_in` and whether step 0 runs (R-547: skipped iff `|I_in − 1| ≤ τ_I`).
fn edge_states<R: P>() -> Vec<(&'static str, Ic<R>, f64, bool)> {
    let tau: f64 = R::TAU_I.into();
    let e = eps::<R>();
    let t = log2_exact(tau);
    let k = log2_exact(e);
    // The least term of the other parity from τ's, at least ε: 2ε where ε's exponent has τ's parity, else ε.
    let extra = if (t - k) % 2 == 0 { k + 1 } else { k };
    vec![
        ("I = 1 exactly", above::<R>(&[]), 1.0, false),
        (
            "I = 1 + τ_I/2",
            above::<R>(&[t - 1]),
            1.0 + tau / 2.0,
            false,
        ),
        ("I = 1 + τ_I", above::<R>(&[t]), 1.0 + tau, false),
        (
            "I just above 1 + τ_I",
            above::<R>(&[t, extra]),
            1.0 + tau + 2f64.powi(extra),
            true,
        ),
        ("I = 1 + 4τ_I", above::<R>(&[t + 2]), 1.0 + 4.0 * tau, true),
        (
            "I = 1 − τ_I/2",
            below::<R>(tau / 2.0),
            1.0 - tau / 2.0,
            false,
        ),
        ("I = 1 − τ_I", below::<R>(tau), 1.0 - tau, false),
        (
            "I just below 1 − τ_I",
            below::<R>(tau + e / 2.0),
            1.0 - tau - e / 2.0,
            true,
        ),
        ("I = 1 − 2τ_I", below::<R>(2.0 * tau), 1.0 - 2.0 * tau, true),
    ]
}

/// Step 0 on one edge state: skipped (bit for bit) or run (`λ = I_in^{−1/2}`, the state at `I = 1`), with
/// `lookup_rescaled` and encode's notice to match.
fn check_edge<R: P>(name: &str, x: &Ic<R>, i_exact: f64, runs: bool) -> f64 {
    let at = format!("{} {name}", std::any::type_name::<R>());
    let c = canonicalise(x);
    let step = scale_step(x);
    let i_in: f64 = c.i_in.into();
    assert!(
        i_in == i_exact,
        "{at}: I_in = {i_in}, the construction's is {i_exact}"
    );
    assert_eq!(
        (c.rescaled, step.rescaled),
        (runs, runs),
        "{at}: lookup_rescaled (C, step 0) is not {runs}"
    );
    assert_eq!(
        encode::<Defaults, R>(x).report.lookup_rescaled(),
        runs,
        "{at}: encode's lookup_rescaled is not {runs}"
    );
    check_finite(&c, &at);
    if !runs {
        check_skipped(x, &step, &c, &at);
        return 0.0;
    }
    let lam: f64 = c.lambda.into();
    // `I_in^{−1/2}` to well below an f64 eps: `1 + δ` with `|δ| ≤ 2^{−17}`, so the series' next term is under `2^{−68}`.
    let d = i_exact - 1.0;
    let want = 1.0 - d / 2.0 + 3.0 * d * d / 8.0 - 5.0 * d * d * d / 16.0;
    let lam_err = (lam - want).abs() / (want * eps::<R>());
    assert!(
        lam_err <= 4.0,
        "{at}: λ = {lam}, not I_in^(−1/2) = {want} ({lam_err} eps)"
    );
    let i_err = (inertia64(&widen(&c.ic)) - 1.0).abs() / eps::<R>();
    assert!(i_err <= 8.0, "{at}: I after C is 1 to {i_err} eps");
    lam_err.max(i_err)
}

#[test]
fn qa_m247_tau_edge() {
    let mut worst = 0.0f64;
    for (name, x, i, runs) in edge_states::<f64>() {
        worst = worst.max(check_edge(name, &x, i, runs));
    }
    for (name, x, i, runs) in edge_states::<f32>() {
        worst = worst.max(check_edge(name, &x, i, runs));
    }
    println!(
        "qa_m247_tau_edge: 9 exact-I states at each precision, skipped at |I_in − 1| ≤ τ_I and run just outside; \
         worst λ or I error where run {worst} eps"
    );
}

negative_control!(
    qa_m247_tau_edge,
    "the state at I_in = 1 + τ_I exactly, expected to run (a strict skip, |I_in − 1| < τ_I), must fail the check",
    expected = "lookup_rescaled (C, step 0) is not true",
    {
        let (name, x, i, _) = edge_states::<f32>().swap_remove(2);
        check_edge(name, &x, i, true);
    }
);

mod edge_below {
    use super::*;
    negative_control!(
        qa_m247_tau_edge,
        "the state just below 1 − τ_I, expected skipped (a skip one half-ulp wider), must fail the check",
        expected = "lookup_rescaled (C, step 0) is not false",
        {
            let (name, x, i, _) = edge_states::<f64>().swap_remove(7);
            check_edge(name, &x, i, false);
        }
    );
}

// ── qa_m247_range (REQ-ENC-006) ─────────────────────────────────────────────────────────────────────────────────

/// A generic state, masses each ≥ 0.2 before normalising, the bodies at least 0.3 apart, off the origin, moving.
fn generic(rng: &mut SplitMix) -> Ic<f64> {
    loop {
        let w = [0; 3].map(|_| f64::from(rng.uniform(0.2, 1.2)));
        let t: f64 = w.iter().sum();
        let a = [0; 2].map(|_| f64::from(rng.uniform(-1.0, 1.0)));
        let r = [0; 3].map(|_| {
            [
                f64::from(rng.uniform(-1.0, 1.0)) + a[0],
                f64::from(rng.uniform(-1.0, 1.0)) + a[1],
            ]
        });
        let apart = [(0, 1), (0, 2), (1, 2)].iter().all(|&(i, j)| {
            ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt() >= 0.3
        });
        // Off the mirror's band by far, so the scaled and unscaled states mirror alike.
        let c =
            (r[1][0] - r[0][0]) * (r[2][1] - r[0][1]) - (r[1][1] - r[0][1]) * (r[2][0] - r[0][0]);
        if apart && c.abs() > 0.05 {
            return Ic {
                m: w.map(|v| v / t),
                r,
                p: [0; 3].map(|_| {
                    [
                        f64::from(rng.uniform(-0.5, 0.5)),
                        f64::from(rng.uniform(-0.5, 0.5)),
                    ]
                }),
            };
        }
    }
}

/// `x` in its rest frame: `rᵢ − R_com`, `pᵢ − mᵢP/M`.
fn rest(x: &Ic<f64>) -> Ic<f64> {
    let t: f64 = x.m.iter().sum();
    let c = [0, 1].map(|k| (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>() / t);
    let q = [0, 1].map(|k| (0..3).map(|i| x.p[i][k]).sum::<f64>());
    Ic {
        m: x.m,
        r: x.r.map(|v| [v[0] - c[0], v[1] - c[1]]),
        p: [0, 1, 2].map(|i| [x.p[i][0] - x.m[i] * q[0] / t, x.p[i][1] - x.m[i] * q[1] / t]),
    }
}

/// `x` scaled by `s`: `r ← s·r`, `p ← p/√s`.
fn scaled(x: &Ic<f64>, s: f64) -> Ic<f64> {
    Ic {
        m: x.m,
        r: x.r.map(|v| v.map(|c| c * s)),
        p: x.p.map(|v| v.map(|c| c / s.sqrt())),
    }
}

/// The largest component gap between two states, in eps.
fn gap(a: &Ic<f64>, b: &Ic<f64>, e: f64) -> f64 {
    a.r.iter()
        .chain(a.p.iter())
        .zip(b.r.iter().chain(b.p.iter()))
        .map(|(u, v)| (u[0] - v[0]).abs().max((u[1] - v[1]).abs()))
        .fold(0.0, f64::max)
        / e
}

/// `C` on `x` scaled by `s`, against `C(x)` at the same precision and the scale gauge's table. The worst, in eps.
fn check_scaled<R: P>(x: &Ic<f64>, s: f64, c: &Canonical<R>, at: &str) -> [f64; 3] {
    let e = eps::<R>();
    check_finite(c, at);
    let i0 = inertia64(&widen(&narrow::<R>(x)));
    assert!(
        c.rescaled && !c.clamped,
        "{at}: (lookup_rescaled, lookup_clamped) = ({}, {}), not (true, false)",
        c.rescaled,
        c.clamped
    );
    let out = widen(&c.ic);
    let i_err = (inertia64(&out) - 1.0).abs() / e;
    assert!(i_err <= 8.0, "{at}: I after C is 1 to {i_err} eps");
    let lam: f64 = c.lambda.into();
    let want = 1.0 / (s * i0.sqrt());
    let lam_err = (lam - want).abs() / (want * e);
    assert!(
        lam_err <= 4.0,
        "{at}: λ = {lam}, not 1/(s√I₀) = {want} ({lam_err} eps)"
    );
    let base = canonicalise(&narrow::<R>(x));
    let state = gap(&widen(&base.ic), &out, e);
    assert!(state <= 32.0, "{at}: C(s·x) is {state} eps from C(x)");
    // E_canon = √I_in·E_in and L_z,canon = L_z,in/I_in^{1/4}: with I_in = s²I₀, E_in = E₀/s, L_z,in = L₀/√s.
    // In the input's rest frame (CoM at the origin, `Σp = 0`), where steps 1a and 1b leave `E` and `L_z` alone.
    let x0 = rest(&widen(&narrow::<R>(x)));
    // A mirrored output has its `L_z` reversed (R-82's full-state reflection).
    let sign = if c.mirrored { -1.0 } else { 1.0 };
    let (e_want, l_want) = (i0.sqrt() * energy(&x0), sign * lz(&x0) / i0.sqrt().sqrt());
    let e_err = (energy(&out) - e_want).abs() / (e * e_want.abs().max(1.0));
    let l_err = (lz(&out) - l_want).abs() / (e * l_want.abs().max(1.0));
    assert!(
        e_err <= 16.0 && l_err <= 16.0,
        "{at}: E_canon {} against √I_in·E_in {e_want} ({e_err} eps), L_z {} against {l_want} ({l_err} eps)",
        energy(&out),
        lz(&out)
    );
    [i_err.max(lam_err), state, e_err.max(l_err)]
}

/// Scales per precision, past `I_in`'s float range at either end (R-547's underflow and overflow states; REQ-ENC-006's
/// verify: f64 below about `1e-162` and above about `1e154`, f32 below about `4e-23` and above about `1e19`).
const SCALES_F64: [f64; 7] = [1e-300, 1e-200, 1e-170, 1e-3, 1e160, 1e200, 1e300];
const SCALES_F32: [f64; 7] = [1e-37, 1e-30, 1e-25, 3e-23, 3e19, 1e30, 1e37];

fn range_at<R: P>(scales: &[f64], seed: u64) -> [f64; 3] {
    let mut rng = SplitMix::new(seed);
    let mut worst = [0.0f64; 3];
    for j in 0..200 {
        let x = generic(&mut rng);
        for &s in scales {
            let at = format!("{} state {j} at s = {s:e}", std::any::type_name::<R>());
            let xs = narrow::<R>(&scaled(&x, s));
            let w = check_scaled(&x, s, &canonicalise(&xs), &at);
            for k in 0..3 {
                worst[k] = worst[k].max(w[k]);
            }
        }
    }
    worst
}

#[test]
fn qa_m247_range() {
    let w64 = range_at::<f64>(&SCALES_F64, 11);
    let w32 = range_at::<f32>(&SCALES_F32, 12);
    println!(
        "qa_m247_range: 200 states at scales {SCALES_F64:?} (f64) and {SCALES_F32:?} (f32): every output finite, \
         lookup_rescaled, I = 1, C(s·x) = C(x); worst (I or λ, the state, E or L_z) {w64:?} eps (f64), {w32:?} eps (f32)"
    );
}

/// A one-step float rescale, the superseded form: `I_in` as one float `Σmᵢ‖cᵢ‖²`, then `r/√I_in` and `I_in^{1/4}·p`,
/// on step 0's own CoM frame.
#[cfg(feature = "controls")]
fn one_step<R: P>(x: &Ic<R>) -> Canonical<R> {
    let step = scale_step(x);
    let c = step.centred;
    let mut i = R::zero();
    for (m, v) in x.m.iter().zip(c.iter()) {
        i = i + *m * (v[0] * v[0] + v[1] * v[1]);
    }
    let (r, p, lambda) = kernel::canon::similarity(c, x.p, i);
    Canonical {
        ic: Ic { m: x.m, r, p },
        lambda,
        rescaled: true,
        ..canonicalise(x)
    }
}

negative_control!(
    qa_m247_range,
    "a one-step float rescale at f64 coordinates near 1e-300 underflows I_in, so the state does not reach I = 1 and \
     the check must fail",
    expected = "I after C is 1 to",
    {
        let x = generic(&mut SplitMix::new(11));
        let xs = narrow::<f64>(&scaled(&x, 1e-300));
        check_scaled(&x, 1e-300, &one_step(&xs), "one step, underflow");
    }
);

mod range_overflow {
    use super::*;
    negative_control!(
        qa_m247_range,
        "a one-step float rescale at f32 coordinates near 1e37 overflows I_in, so an output is not finite and the \
         check must fail",
        expected = "an output is not finite",
        {
            let x = generic(&mut SplitMix::new(12));
            let xs = narrow::<f32>(&scaled(&x, 1e37));
            check_scaled(&x, 1e37, &one_step(&xs), "one step, overflow");
        }
    );
}

// ── qa_m247_coincident (REQ-ENC-006, REQ-ENC-014) ───────────────────────────────────────────────────────────────

/// Three coincident bodies at `q`, with nonzero momenta summing to 0.
fn coincident<R: P>(q: [f64; 2]) -> Ic<R> {
    narrow(&Ic {
        m: [0.25, 0.375, 0.375],
        r: [q, q, q],
        p: [[0.5, -0.25], [-0.25, 0.75], [-0.25, -0.5]],
    })
}

const POINTS: [[f64; 2]; 6] = [
    [0.0, 0.0],
    [0.3, -0.7],
    [-1.0e-20, 3.0e-21],
    [1.0e18, -2.5e17],
    [0.1, 0.2],
    [-7.0, 1.0e-3],
];

/// The coincident select: divisor 1 (no `1/0`, every output finite), `λ = 1`, `p = 0`, `lookup_clamped`, `φ = +0`,
/// and no `lookup_rescaled` (step 0 did not run); encode reports the coincident pair and no rescale.
fn check_coincident<R: P>(x: &Ic<R>, c: &Canonical<R>, at: &str) {
    check_finite(c, at);
    let lam: f64 = c.lambda.into();
    let phi: f64 = c.phi.into();
    assert!(lam == 1.0, "{at}: λ = {lam}, not 1");
    assert!(
        phi.to_bits() == 0,
        "{at}: φ = {phi:e} (bits {:#x}), not +0",
        phi.to_bits()
    );
    assert!(c.clamped, "{at}: no lookup_clamped at ρ = 0");
    assert!(
        !c.rescaled,
        "{at}: lookup_rescaled raised for coincident bodies"
    );
    for (i, v) in widen(&c.ic).p.iter().enumerate() {
        assert!(v[0] == 0.0 && v[1] == 0.0, "{at}: p{i} = {v:?}, not 0");
    }
    let e = encode::<Defaults, R>(x);
    assert!(
        e.report.coincident && e.report.lookup_clamped() && !e.report.lookup_rescaled(),
        "{at}: encode's report {:?} is not the coincident one",
        e.report
    );
}

#[test]
fn qa_m247_coincident() {
    for q in POINTS {
        let x = coincident::<f64>(q);
        check_coincident(&x, &canonicalise(&x), &format!("f64 at {q:?}"));
        let x = coincident::<f32>(q);
        check_coincident(&x, &canonicalise(&x), &format!("f32 at {q:?}"));
    }
}

negative_control!(
    qa_m247_coincident,
    "step 0's map with no select at I_in = 0 forms 1/0, so an output is not finite and the check must fail",
    expected = "an output is not finite",
    {
        let x = coincident::<f32>([0.0, 0.0]);
        let c = canonicalise(&x);
        let lambda = 1.0 / 0.0f32.sqrt();
        let r = c.ic.r.map(|v| v.map(|k| k * lambda));
        check_coincident(
            &x,
            &Canonical {
                ic: Ic { r, ..c.ic },
                lambda,
                ..c
            },
            "no select",
        );
    }
);

// ── qa_m247_subnormal_differences (REQ-DEC-002) ─────────────────────────────────────────────────────────────────

/// f32 states at coordinates near `1e-36`, the inner pair `1..=7` ulps apart on each axis (a subnormal difference:
/// an f32 ulp there is `2^−143`), the third body about `1e-36` off, in any direction; momenta order 1.
fn subnormal_states(n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|_| {
            let base = [0; 2].map(|_| rng.uniform(0.8, 1.6) * 1e-36);
            let mut r1 = base;
            for v in r1.iter_mut() {
                let k = 1 + (rng.next_u64() % 7);
                let up = rng.next_u64().is_multiple_of(2);
                for _ in 0..k {
                    *v = if up { v.next_up() } else { v.next_down() };
                }
            }
            let r2 = [
                base[0] + rng.uniform(-1.0, 1.0) * 1e-36,
                base[1] + rng.uniform(-1.0, 1.0) * 1e-36,
            ];
            let w = [0; 3].map(|_| rng.uniform(0.2, 1.2));
            let t: f32 = w.iter().sum();
            Ic {
                m: w.map(|v| v / t),
                r: [base, r1, r2],
                p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
            }
        })
        .collect()
}

/// `(ρ, λ̃_y)` of `x` from its pair differences taken in f64, where an f32 difference is exact and normal: `ρ = r₁ − r₀`,
/// and `λ̃_y = √(m₂M₀₁)·(ρ̂ × Λ)/(M₀₁√I)`, `Λ = m₀(r₂ − r₀) + m₁(r₂ − r₁)`, `I = Σ_{i<j}mᵢmⱼ‖rⱼ − rᵢ‖²` (`Σm`
/// normalised), the canonical frame's `λ̃_y` (dd_decoder §3.2, R-82), `ρ̂ = (1, 0)` at `ρ = 0`.
fn exact_branch_inputs(x: &Ic<f32>) -> ([f64; 2], f64) {
    let w = widen(x);
    let t: f64 = w.m.iter().sum();
    let m = w.m.map(|v| v / t);
    let d = |i: usize, j: usize| [w.r[j][0] - w.r[i][0], w.r[j][1] - w.r[i][1]];
    let rho = d(0, 1);
    let lam = [0, 1].map(|k| m[0] * d(0, 2)[k] + m[1] * d(1, 2)[k]);
    let n2 = |v: [f64; 2]| v[0] * v[0] + v[1] * v[1];
    let i = m[0] * m[1] * n2(d(0, 1)) + m[0] * m[2] * n2(d(0, 2)) + m[1] * m[2] * n2(d(1, 2));
    let norm = n2(rho).sqrt();
    let hat = if norm == 0.0 {
        [1.0, 0.0]
    } else {
        [rho[0] / norm, rho[1] / norm]
    };
    let m01 = m[0] + m[1];
    let cross = hat[0] * lam[1] - hat[1] * lam[0];
    (rho, (m[2] * m01).sqrt() * cross / (m01 * i.sqrt()))
}

/// `C` on `x` against its exact branch inputs `(ρ, λ̃_y)`: `lookup_clamped` iff `ρ = 0`; `φ` within 4 eps of
/// `atan2(ρ_y, ρ_x)`; the mirror iff `λ̃_y < −δ_λ` (only where `λ̃_y` is clear of `−δ_λ` by `10⁻³`); every output
/// finite at `I = 1`. The worst `φ` error, in f32 eps.
fn check_exact_branches(c: &Canonical<f32>, exact: ([f64; 2], f64), at: &str) -> f64 {
    let (rho, lty) = exact;
    let pole = rho[0] == 0.0 && rho[1] == 0.0;
    assert_eq!(
        c.clamped, pole,
        "{at}: lookup_clamped is not ρ = 0, the exact ρ being {rho:?}"
    );
    check_finite(c, at);
    let i_err = (inertia64(&widen(&c.ic)) - 1.0).abs() / eps::<f32>();
    assert!(i_err <= 8.0, "{at}: I after C is 1 to {i_err} eps");
    let phi_want = if pole { 0.0 } else { rho[1].atan2(rho[0]) };
    let phi_err = (f64::from(c.phi) - phi_want).abs() / (eps::<f32>() * phi_want.abs().max(1.0));
    assert!(
        phi_err <= 4.0,
        "{at}: φ = {}, the exact ρ's is {phi_want} ({phi_err} eps)",
        c.phi
    );
    if (lty + DELTA).abs() > 1e-3 * DELTA {
        assert_eq!(
            c.mirrored,
            lty < -DELTA,
            "{at}: lookup_mirrored is not the exact decision, λ̃_y = {lty:e}"
        );
    }
    phi_err
}

#[test]
fn qa_m247_subnormal_differences() {
    let mut worst = 0.0f64;
    let (mut mirrored, mut pole) = (0, 0);
    let states = subnormal_states(4000, 0x5ab);
    for (j, x) in states.iter().enumerate() {
        let c = canonicalise(x);
        worst = worst.max(check_exact_branches(
            &c,
            exact_branch_inputs(x),
            &format!("state {j}"),
        ));
        mirrored += usize::from(c.mirrored);
    }
    // The same states with the inner pair made coincident: the guard's other side.
    for (j, x) in states.iter().take(400).enumerate() {
        let y = Ic {
            r: [x.r[0], x.r[0], x.r[2]],
            ..*x
        };
        let c = canonicalise(&y);
        check_exact_branches(&c, exact_branch_inputs(&y), &format!("coincident pair {j}"));
        pole += usize::from(c.clamped);
    }
    assert!(
        mirrored > 0,
        "no state mirrored, so the mirror arm was never reached"
    );
    println!(
        "qa_m247_subnormal_differences: 4000 f32 states with subnormal inner-pair differences ({mirrored} mirrored) \
         and 400 with the pair coincident ({pole} clamped): the guard, φ and the mirror the exact differences'; worst φ \
         {worst} eps"
    );
}

negative_control!(
    qa_m247_subnormal_differences,
    "the inner pair's difference as a lone f32 subtraction flushed to zero reads ρ = 0, so the guard check must fail",
    expected = "lookup_clamped is not ρ = 0",
    {
        for x in subnormal_states(64, 0x5ab) {
            let flushed = |a: f32, b: f32| {
                let d = b - a;
                if d.is_subnormal() {
                    0.0
                } else {
                    d
                }
            };
            let rho = [
                flushed(x.r[0][0], x.r[1][0]),
                flushed(x.r[0][1], x.r[1][1]),
            ];
            let mut c = canonicalise(&x);
            c.clamped = rho[0] == 0.0 && rho[1] == 0.0;
            check_exact_branches(&c, exact_branch_inputs(&x), "flushed");
        }
    }
);
