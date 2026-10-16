//! QA tests for TASK-M2-07, written from the requirements and their sources, not from the implementation: `C`
//! (`kernel::canon::canonicalise`) and the scale gauge (`kernel::canon::scale_gauge`) on the CPU, the shared source's
//! f64 and f32 instantiations (R-265). The GPU-f32 build is the implementer's `engine/tests/canon_gpu.rs`.
//!
//! Every expected value here comes from an independent construction: a canonical representative built from
//! dd_decoder §3.2's Jacobi reconstruction, the gauge group action of dd_encode §3.3, a plain f64 reference of
//! dd_encode §3.2's steps 1a → 0 → 1b → 2 → 3 written from the text ([`reference`]), and chart_reference §4.2's Burrau
//! construction.
//! - REQ-DEC-002, REQ-ENC-015 (`qa_m207_gauge_quotient`): for a canonical representative `x` and a random gauge
//!   `g = (a, u, θ, s, Π)`, `C(g·x)` lies on the section (CoM 0, `Σp = 0`, `ρ̃` on `+x`, `λ̃_y ≥ −δ_λ`, `I = 1`), equals
//!   `x`, records `I_in = s²` (the CoM-frame `I`, not the origin's) and `λ = 1/s`, mirrors iff `Π` did, and `C` is
//!   idempotent; at f64 and f32.
//! - REQ-DEC-002 (`qa_m207_reference`): on arbitrary physical states, `C` agrees with the f64 reference of the ordered
//!   procedure, its notices included.
//! - REQ-DEC-017 (`qa_m207_deadband_tilde`): the mirror is the test `λ̃_y < −δ_λ` on the mass-weighted `λ̃`, never on
//!   `λ_y`: states with `λ̃_y = k·δ_λ`, `k` straddling `±1`, under rotations, translations, boosts, scales and `Π`, at
//!   masses where `|λ_y| > δ_λ > |λ̃_y|`; the same output twice; f32 on states whose steps are exact there.
//! - dd_encode §5 test 3's half-mirror catch, R-82's full-state rule (`qa_m207_mirror_full_state`): a mirrored output
//!   has every `rᵢ` and every `pᵢ` reflected, so `L_z` flips sign with `lookup_mirrored` and keeps its magnitude.
//! - REQ-DEC-018 (`qa_m207_noop_extreme`): dd_decoder §5 test 5 on decodes at the extreme finite `z` (each control at
//!   `±10³⁰` and at the precision's largest finite value), at f32 and f64: no mirror, rotation angle ≈ 0, the state
//!   unchanged to rounding, the same result twice.
//! - REQ-DEC-023 (`qa_m207_scale_gauge_powers`): dd_decoder §3.5 component by component, `rᵢ/ℓ` and `√ℓ·pᵢ` (the
//!   asymmetric powers chart_reference §0.5 warns of), `I = 1`, the identity on canonical output, idempotent, and the
//!   same map as `C`'s step 0.
//! - REQ-ENC-006 (`qa_m207_burrau_scale_step`): chart_reference §4.2's Burrau (3,4,5) state, built here, at 2× the
//!   canonical scale, checked against `fixtures/states/burrau_2x.json`: `I_in = 4`, `λ = 1/2`, `r ← r/2`, `p ← √2·p`,
//!   `E_canon = 2·E_in`, `L_canon = L_in/√2`, and the timing factor `I_in^{3/4}` read off the dynamics: velocities scale
//!   by `λ·I_in^{3/4}` and accelerations by `λ·I_in^{3/2}`, the similarity `t ← t/I_in^{3/4}` of inverse-encode Part 2.
//! - REQ-ENC-016 (`qa_m207_coincident_pole`): `ρ = 0` with `I_in > 0`, components `(+0, +0)`, `(−0, +0)` and
//!   `(+0, −0)`, raises `lookup_clamped` with `φ = +0` by its bits and the identity rotation (R-13, R-437), at f64 and
//!   f32; at `I_in = 0` (RQ-296 holds the state there) only the flag and `φ` are asserted; a `ρ` off 0 does not clamp.
//!
//! **Not asserted** (held open): when `lookup_rescaled` is raised (RQ-295), and `C`'s state and `λ` at `I_in = 0`
//! (RQ-296).
//!
//! **Tolerances.** Each is an exact identity checked to rounding: a multiple of the precision's epsilon from the
//! number of rounded operations on the path (CoM, rescale, unboost, the rotation from `atan2`/`sin`/`cos`), times the
//! conditioning the constructions bound (`α ∈ [0.15, 1.42]`, so `‖ρ̃‖` and `‖λ̃‖` stay above 0.15; translations within
//! 3× the state's size). Each test prints its measured worst case.
//!
//! **Domain** (pitfalls §10): `C` alone, on states held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use kernel::canon::{canonicalise, scale_gauge, Canonical};
use kernel::decode::{decode, Ic, Latent};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use proptest::prelude::*;

use validation::{negative_control, prop};

/// δ_λ, dd_decoder §3.3 and R-82.
const DELTA: f64 = 1e-12;

/// A precision the shared source runs at.
trait P: LinkReal + PayloadReal + Into<f64> + std::fmt::Debug {}
impl P for f32 {}
impl P for f64 {}

fn eps<R: P>() -> f64 {
    R::epsilon().into()
}

fn narrow<R: P>(x: &Ic<f64>) -> Ic<R> {
    let c = |v: f64| <R as spirv_std::num_traits::NumCast>::from(v).expect("finite");
    Ic {
        m: x.m.map(c),
        r: x.r.map(|v| v.map(c)),
        p: x.p.map(|v| v.map(c)),
    }
}

fn wide<R: P>(x: &Ic<R>) -> Ic<f64> {
    Ic {
        m: x.m.map(Into::into),
        r: x.r.map(|v| v.map(Into::into)),
        p: x.p.map(|v| v.map(Into::into)),
    }
}

/// `C` as a function the checks take, so a control can hand them a wrong one.
type Canon<R> = fn(&Ic<R>) -> Canonical<R>;

fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

/// The worst component gap of `got` from `want`, positions over `want`'s largest `‖rᵢ‖`, momenta over its largest
/// `‖pᵢ‖` (or 1 where all are 0), in `eps`.
fn gap(want: &Ic<f64>, got: &Ic<f64>, eps: f64) -> f64 {
    gap_over(want, got, eps, [0.0; 2])
}

/// [`gap`] with `extra[0]` added to the position scale and `extra[1]` to the momentum scale: the translation and boost
/// a step subtracts, in output units, whose rounding the output carries.
fn gap_over(want: &Ic<f64>, got: &Ic<f64>, eps: f64, extra: [f64; 2]) -> f64 {
    let size = |v: &[[f64; 2]; 3], extra: f64| {
        let s = v.iter().map(|x| norm(*x)).fold(0.0, f64::max) + extra;
        if s == 0.0 {
            1.0
        } else {
            s
        }
    };
    let (sr, sp) = (size(&want.r, extra[0]), size(&want.p, extra[1]));
    let mut worst = 0.0f64;
    for i in 0..3 {
        worst = worst.max((want.m[i] - got.m[i]).abs() / eps);
        for k in 0..2 {
            worst = worst.max((want.r[i][k] - got.r[i][k]).abs() / (sr * eps));
            worst = worst.max((want.p[i][k] - got.p[i][k]).abs() / (sp * eps));
        }
    }
    worst
}

// ── Independent physics, all at f64 ─────────────────────────────────────────────────────────────────────────────

fn com(x: &Ic<f64>) -> [f64; 2] {
    let m: f64 = x.m.iter().sum();
    [0, 1].map(|k| (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>() / m)
}

fn total_p(x: &Ic<f64>) -> [f64; 2] {
    [0, 1].map(|k| (0..3).map(|i| x.p[i][k]).sum::<f64>())
}

/// `I = Σmᵢ‖rᵢ − R_com‖²`, the CoM-frame moment of inertia.
fn inertia_com(x: &Ic<f64>) -> f64 {
    let c = com(x);
    (0..3)
        .map(|i| x.m[i] * ((x.r[i][0] - c[0]).powi(2) + (x.r[i][1] - c[1]).powi(2)))
        .sum()
}

/// `λ̃ = √μ_λ·(r₂ − (m₀r₀ + m₁r₁)/M₀₁)`, `μ_λ = m₂M₀₁` (dd_decoder §3.2).
fn lambda_tilde(x: &Ic<f64>) -> [f64; 2] {
    let m01 = x.m[0] + x.m[1];
    let mu = (x.m[2] * m01).sqrt();
    [0, 1].map(|k| mu * (x.r[2][k] - (x.m[0] * x.r[0][k] + x.m[1] * x.r[1][k]) / m01))
}

fn rho(x: &Ic<f64>) -> [f64; 2] {
    [x.r[1][0] - x.r[0][0], x.r[1][1] - x.r[0][1]]
}

/// `E = Σ‖pᵢ‖²/2mᵢ − Σ mᵢmⱼ/rᵢⱼ`, `G = 1`.
fn energy(x: &Ic<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|i| (x.p[i][0].powi(2) + x.p[i][1].powi(2)) / (2.0 * x.m[i]))
        .sum();
    let u: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| -x.m[i] * x.m[j] / norm([x.r[i][0] - x.r[j][0], x.r[i][1] - x.r[j][1]]))
        .sum();
    k + u
}

/// `L_z = Σ rᵢ × pᵢ`.
fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// Each body's Newtonian acceleration, `G = 1`.
fn accelerations(x: &Ic<f64>) -> [[f64; 2]; 3] {
    [0, 1, 2].map(|i| {
        let mut a = [0.0; 2];
        for j in (0..3).filter(|&j| j != i) {
            let d = [x.r[j][0] - x.r[i][0], x.r[j][1] - x.r[i][1]];
            let r3 = norm(d).powi(3);
            a[0] += x.m[j] * d[0] / r3;
            a[1] += x.m[j] * d[1] / r3;
        }
        a
    })
}

/// A canonical representative (dd_decoder §3.2): masses `m` (`Σ = 1`), `ρ̃ = cos α·(1, 0)`,
/// `λ̃ = sin α·(cos β, sin β)`, unweighted and reconstructed; momenta `p₀`, `p₁` and `p₂ = −p₀ − p₁`.
fn representative(m: [f64; 3], alpha: f64, beta: f64, p0: [f64; 2], p1: [f64; 2]) -> Ic<f64> {
    let m01 = m[0] + m[1];
    let (mu_rho, mu_lambda) = (m[0] * m[1] / m01, m[2] * m01);
    let rho = [alpha.cos() / mu_rho.sqrt(), 0.0];
    let lam = [
        alpha.sin() * beta.cos() / mu_lambda.sqrt(),
        alpha.sin() * beta.sin() / mu_lambda.sqrt(),
    ];
    let r01 = [-m[2] * lam[0], -m[2] * lam[1]];
    Ic {
        m,
        r: [
            [r01[0] - m[1] / m01 * rho[0], r01[1] - m[1] / m01 * rho[1]],
            [r01[0] + m[0] / m01 * rho[0], r01[1] + m[0] / m01 * rho[1]],
            [m01 * lam[0], m01 * lam[1]],
        ],
        p: [p0, p1, [-p0[0] - p1[0], -p0[1] - p1[1]]],
    }
}

/// dd_encode §3.3's gauge `g = (a, u, θ, s, Π)`: `rᵢ' = s·R(θ)·Π·rᵢ + a`, `pᵢ' = s^{−1/2}·R(θ)·Π·pᵢ + mᵢu`.
#[derive(Clone, Copy, Debug)]
struct Gauge {
    a: [f64; 2],
    u: [f64; 2],
    theta: f64,
    s: f64,
    mirror: bool,
}

fn act(g: &Gauge, x: &Ic<f64>) -> Ic<f64> {
    let (sn, cs) = g.theta.sin_cos();
    let map = |v: [f64; 2], k: f64| {
        let y = if g.mirror { -v[1] } else { v[1] };
        [k * (cs * v[0] - sn * y), k * (sn * v[0] + cs * y)]
    };
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let w = map(v, g.s);
            [w[0] + g.a[0], w[1] + g.a[1]]
        }),
        p: [0, 1, 2].map(|i| {
            let w = map(x.p[i], g.s.powf(-0.5));
            [w[0] + x.m[i] * g.u[0], w[1] + x.m[i] * g.u[1]]
        }),
    }
}

/// The ordered procedure of dd_encode §3.2 written from its text, at f64, `M` kept general: 1a `r ← r − R_com`;
/// 0 `I_in = Σmᵢ‖rᵢ‖²`, `r ← r/√I_in`, `p ← I_in^{1/4}·p`, `λ = I_in^{−1/2}`; 1b `p ← p − mᵢP_tot/M`; 2 rotate by
/// `−φ`, `φ = atan2(ρ_y, ρ_x)` and 0 at `ρ = 0` (R-437), `lookup_clamped` there (R-13); 3 mirror the full state iff
/// `λ̃_y < −δ_λ`. Defined for `I_in > 0`.
struct Reference {
    ic: Ic<f64>,
    i_in: f64,
    lambda: f64,
    clamped: bool,
    mirrored: bool,
}

fn reference(x: &Ic<f64>) -> Reference {
    let total_m: f64 = x.m.iter().sum();
    let c = com(x);
    let centred = x.r.map(|v| [v[0] - c[0], v[1] - c[1]]);
    let i_in: f64 = (0..3)
        .map(|i| x.m[i] * (centred[i][0].powi(2) + centred[i][1].powi(2)))
        .sum();
    let root = i_in.sqrt();
    let r = centred.map(|v| [v[0] / root, v[1] / root]);
    let p = x.p.map(|v| [v[0] * root.sqrt(), v[1] * root.sqrt()]);
    let pt = [0, 1].map(|k| p[0][k] + p[1][k] + p[2][k]);
    let p = [0, 1, 2].map(|i| {
        [
            p[i][0] - x.m[i] * pt[0] / total_m,
            p[i][1] - x.m[i] * pt[1] / total_m,
        ]
    });
    let rh = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let clamped = rh[0] == 0.0 && rh[1] == 0.0;
    let phi = if clamped { 0.0 } else { rh[1].atan2(rh[0]) };
    let (s, cs) = phi.sin_cos();
    let rot = |v: [f64; 2]| [cs * v[0] + s * v[1], cs * v[1] - s * v[0]];
    let mut out = Ic {
        m: x.m,
        r: r.map(rot),
        p: p.map(rot),
    };
    let mirrored = lambda_tilde(&out)[1] < -DELTA;
    if mirrored {
        for v in out.r.iter_mut().chain(out.p.iter_mut()) {
            v[1] = -v[1];
        }
    }
    Reference {
        ic: out,
        i_in,
        lambda: 1.0 / root,
        clamped,
        mirrored,
    }
}

// ── qa_m207_gauge_quotient (REQ-DEC-002, REQ-ENC-015) ───────────────────────────────────────────────────────────

/// The quotient's tolerance, in eps: on `I_in` and `λ`, and on the output state over its conditioning `κ`
/// ([`quotient`]).
const QUOTIENT_EPS: f64 = 32.0;
/// The section conditions' tolerance, in eps: the CoM over `Σmᵢ‖rᵢ‖`, `Σp` over `Σ‖pᵢ‖`, `ρ_y` over `‖ρ‖`, `I − 1`.
const SECTION_EPS: f64 = 32.0;

/// A representative and a gauge: masses each ≥ 0.05 before normalising, `α ∈ [0.15, 1.42]`, `β ∈ [0.05, π − 0.05]`,
/// momenta in `[−1, 1]`; the gauge's `s = 10^t`, `t ∈ [−3, 3]`, `θ ∈ [−π, π]`, `‖a‖ ≤ 3s`, `|uₖ| ≤ 2`.
fn cases() -> impl Strategy<Value = (Ic<f64>, Gauge)> {
    (
        proptest::array::uniform3(0.05f64..1.0),
        0.15f64..1.42,
        0.05f64..(std::f64::consts::PI - 0.05),
        proptest::array::uniform4(-1.0f64..1.0),
        (
            -3.0f64..3.0,
            -std::f64::consts::PI..std::f64::consts::PI,
            proptest::array::uniform4(-1.0f64..1.0),
            any::<bool>(),
        ),
    )
        .prop_map(|(w, alpha, beta, q, (t, theta, au, mirror))| {
            let total: f64 = w.iter().sum();
            let x = representative(
                w.map(|v| v / total),
                alpha,
                beta,
                [q[0], q[1]],
                [q[2], q[3]],
            );
            let s = 10f64.powf(t);
            let g = Gauge {
                a: [2.1 * s * au[0], 2.1 * s * au[1]],
                u: [2.0 * au[2], 2.0 * au[3]],
                theta,
                s,
                mirror,
            };
            (x, g)
        })
}

/// The section conditions on `out`, each within [`SECTION_EPS`]; the worst, in eps. `boost` is the input's total
/// momentum in output units, `‖P_tot‖·I_in^{1/4}`, which step 1b subtracts with its rounding.
fn section<R: P>(out: &Canonical<R>, boost: f64) -> Result<f64, String> {
    let e = eps::<R>();
    let x = wide(&out.ic);
    let c = com(&x);
    let size: f64 = (0..3).map(|i| x.m[i] * norm(x.r[i])).sum();
    let pt = total_p(&x);
    let psize: f64 = (x.p.iter().map(|v| norm(*v)).sum::<f64>() + boost).max(f64::MIN_POSITIVE);
    let rh = rho(&x);
    if rh[0] <= 0.0 {
        return Err(format!("ρ̃ is not on +x: ρ = {rh:?}"));
    }
    if lambda_tilde(&x)[1] < -DELTA {
        return Err(format!(
            "λ̃_y = {} < −δ_λ on C's output",
            lambda_tilde(&x)[1]
        ));
    }
    let m = [
        ("the CoM", norm(c) / (size * e)),
        ("Σp", norm(pt) / (psize * e)),
        ("ρ_y", rh[1].abs() / (norm(rh) * e)),
        ("I − 1", (inertia_com(&x) - 1.0).abs() / e),
    ];
    let mut worst = 0.0f64;
    for (what, v) in m {
        if v > SECTION_EPS {
            return Err(format!("{what} is {v} eps off the section"));
        }
        worst = worst.max(v);
    }
    Ok(worst)
}

/// `C(g·x)` against `x`, at the precision `R`: the section, `x` itself, `I_in = s²`, `λ = 1/s`, the mirror iff `Π`,
/// no clamp, and `C` idempotent; the worst gap, in eps.
fn quotient<R: P>(c: Canon<R>, x: &Ic<f64>, g: &Gauge) -> Result<f64, String> {
    let e = eps::<R>();
    let y = narrow::<R>(&act(g, x));
    let out = c(&y);
    // The boost `mᵢu` sums to `u` (Σmᵢ = 1); step 0 multiplies it by `s^{1/2}` before step 1b removes it.
    let boost = norm(g.u) * g.s.sqrt();
    let mut worst = section(&out, boost)?;
    let i_in: f64 = out.i_in.into();
    let lambda: f64 = out.lambda.into();
    let di = (i_in - g.s * g.s).abs() / (g.s * g.s * e);
    if di > QUOTIENT_EPS {
        return Err(format!(
            "I_in = {i_in}, not the CoM-frame s² = {} ({di} eps)",
            g.s * g.s
        ));
    }
    let dl = (lambda * g.s - 1.0).abs() / e;
    if dl > QUOTIENT_EPS {
        return Err(format!("λ = {lambda}, not 1/s = {} ({dl} eps)", 1.0 / g.s));
    }
    if out.mirrored != g.mirror || out.clamped {
        return Err(format!(
            "mirrored {} clamped {} under Π = {}",
            out.mirrored, out.clamped, g.mirror
        ));
    }
    // The error model: a translation of `‖a‖` rounds the positions by `eps·‖a‖`, `‖a‖/s` in output units, and step 2's
    // angle, read off `ρ`, turns every `rᵢ` and `pᵢ` by that over `‖ρ‖`; so the output's gap is `eps` times
    // `κ = (1 + ‖a‖/(s·R))·(1 + R/‖ρ‖)`, `R = maxᵢ‖rᵢ‖` of `x`.
    let big_r = x.r.iter().map(|v| norm(*v)).fold(0.0, f64::max);
    let kappa = (1.0 + norm(g.a) / (g.s * big_r)) * (1.0 + big_r / norm(rho(x)));
    let d = gap_over(x, &wide(&out.ic), e, [norm(g.a) / g.s, boost]) / kappa;
    if d > QUOTIENT_EPS {
        return Err(format!("C(g·x) is {d} eps from x, over κ = {kappa}"));
    }
    let again = c(&out.ic);
    let di = gap(&wide(&out.ic), &wide(&again.ic), e) / kappa;
    if again.mirrored || again.clamped || di > QUOTIENT_EPS {
        return Err(format!(
            "C(C(y)) is {di} eps from C(y), mirrored {} clamped {}",
            again.mirrored, again.clamped
        ));
    }
    worst = worst.max(d).max(di);
    Ok(worst)
}

fn quotient_at<R: P>(name: &str) {
    let worst = std::cell::Cell::new(0.0f64);
    prop::run(&cases(), |(x, g)| {
        let w = quotient::<R>(canonicalise, &x, &g).map_err(TestCaseError::fail)?;
        worst.set(worst.get().max(w));
        Ok(())
    });
    eprintln!(
        "qa_m207_gauge_quotient {name}: worst {} eps (tolerance {QUOTIENT_EPS}); {}",
        worst.get(),
        prop::cases_status()
    );
}

#[test]
fn qa_m207_gauge_quotient() {
    quotient_at::<f64>("f64");
    quotient_at::<f32>("f32");
}

/// `C` with step 0's `I` taken about the origin, before step 1a: the order R-23 forbids.
#[cfg(feature = "controls")]
fn origin_inertia(x: &Ic<f64>) -> Canonical<f64> {
    let mut c = canonicalise(x);
    c.i_in = (0..3)
        .map(|i| x.m[i] * (x.r[i][0].powi(2) + x.r[i][1].powi(2)))
        .sum();
    c
}

negative_control!(
    qa_m207_gauge_quotient,
    "a C that records I about the origin, not in the CoM frame, must fail",
    expected = "not the CoM-frame",
    {
        let x = representative([0.2, 0.3, 0.5], 0.7, 1.1, [0.1, 0.2], [-0.3, 0.1]);
        let g = Gauge {
            a: [5.0, -4.0],
            u: [0.3, 0.2],
            theta: 0.9,
            s: 2.0,
            mirror: true,
        };
        quotient::<f64>(origin_inertia, &x, &g).unwrap();
    }
);

// ── qa_m207_reference (REQ-DEC-002) ─────────────────────────────────────────────────────────────────────────────

/// Arbitrary physical states: masses ≥ 0.05 before normalising, positions and momenta in `[−5, 5]`, the inner pair at
/// least 0.5 apart so the rotation is well conditioned, and `|λ̃_y|` of the reference's pre-mirror frame away from
/// `δ_λ` (rejected in the run).
fn states() -> impl Strategy<Value = Ic<f64>> {
    (
        proptest::array::uniform3(0.05f64..1.0),
        proptest::array::uniform6(-5.0f64..5.0),
        proptest::array::uniform6(-5.0f64..5.0),
    )
        .prop_map(|(w, r, p)| {
            let total: f64 = w.iter().sum();
            Ic {
                m: w.map(|v| v / total),
                r: [[r[0], r[1]], [r[2], r[3]], [r[4], r[5]]],
                p: [[p[0], p[1]], [p[2], p[3]], [p[4], p[5]]],
            }
        })
        .prop_filter("a well-separated inner pair", |x| norm(rho(x)) > 0.5)
}

const REFERENCE_EPS: f64 = 32.0;

fn against_reference(c: Canon<f64>, x: &Ic<f64>) -> Result<f64, String> {
    let want = reference(x);
    let got = c(x);
    if got.mirrored != want.mirrored || got.clamped != want.clamped {
        return Err(format!(
            "notices mirrored {} clamped {}, the reference's {} {}",
            got.mirrored, got.clamped, want.mirrored, want.clamped
        ));
    }
    let checks = [
        (
            "I_in",
            (got.i_in - want.i_in).abs() / (want.i_in * f64::EPSILON),
        ),
        (
            "λ",
            (got.lambda - want.lambda).abs() / (want.lambda * f64::EPSILON),
        ),
        ("the state", gap(&want.ic, &got.ic, f64::EPSILON)),
    ];
    let mut worst = 0.0f64;
    for (what, d) in checks {
        if d > REFERENCE_EPS {
            return Err(format!("{what} is {d} eps from the reference"));
        }
        worst = worst.max(d);
    }
    Ok(worst)
}

#[test]
fn qa_m207_reference() {
    let worst = std::cell::Cell::new(0.0f64);
    let mirrored = std::cell::Cell::new(0usize);
    prop::run(&states(), |x| {
        let pre = reference(&x);
        // Keep away from the deadband's edge, where the two may round to different sides.
        prop_assume!((lambda_tilde(&pre.ic)[1].abs() - DELTA).abs() > 1e-9);
        mirrored.set(mirrored.get() + usize::from(pre.mirrored));
        let w = against_reference(canonicalise, &x).map_err(TestCaseError::fail)?;
        worst.set(worst.get().max(w));
        Ok(())
    });
    assert!(mirrored.get() > 0, "no state reached the mirror");
    eprintln!(
        "qa_m207_reference: worst {} eps (tolerance {REFERENCE_EPS}); {} mirrored; {}",
        worst.get(),
        mirrored.get(),
        prop::cases_status()
    );
}

/// `C` with the mirror applied to positions only, the config-only mirror dd_encode §5 test 3 exists to catch.
#[cfg(feature = "controls")]
fn config_only_mirror(x: &Ic<f64>) -> Canonical<f64> {
    let mut c = canonicalise(x);
    if c.mirrored {
        for v in c.ic.p.iter_mut() {
            v[1] = -v[1];
        }
    }
    c
}

negative_control!(
    qa_m207_reference,
    "a C that mirrors the positions but not the momenta must fail",
    expected = "the state is",
    {
        let x = Ic {
            m: [0.3, 0.3, 0.4],
            r: [[0.0, 0.0], [2.0, 0.0], [1.0, -1.5]],
            p: [[0.2, 0.4], [-0.1, -0.3], [0.5, 0.1]],
        };
        assert!(reference(&x).mirrored, "the control's state must mirror");
        against_reference(config_only_mirror, &x).unwrap();
    }
);

// ── qa_m207_mirror_full_state (dd_encode §5 test 3; R-82's full-state rule) ──────────────────────────────────────

/// On `x` and `C(x)`: `L_z(C(x)) = ±L_z(x)·λ^{1/2}`, `L_z(x)` taken in `x`'s CoM frame at zero total momentum, minus exactly when mirrored (`L_canon = L_in/I_in^{1/4}`); every
/// `pᵢ` of a mirrored output is the reflection of the unmirrored one.
fn lz_sign(c: Canon<f64>, x: &Ic<f64>) -> Result<(), String> {
    let out = c(x);
    // `L_in` in the input's CoM frame at zero total momentum, the `L_z` the scale step's table transforms.
    let (cm, pt) = (com(x), total_p(x));
    let internal = Ic {
        m: x.m,
        r: x.r.map(|v| [v[0] - cm[0], v[1] - cm[1]]),
        p: [0, 1, 2].map(|i| [x.p[i][0] - x.m[i] * pt[0], x.p[i][1] - x.m[i] * pt[1]]),
    };
    let want = if out.mirrored { -1.0 } else { 1.0 } * lz(&internal) * out.lambda.sqrt();
    let got = lz(&out.ic);
    let scale: f64 = (0..3).map(|i| norm(out.ic.r[i]) * norm(out.ic.p[i])).sum();
    if (got - want).abs() > 64.0 * f64::EPSILON * scale {
        return Err(format!(
            "L_z(C(x)) = {got}, not {want}, mirrored {}",
            out.mirrored
        ));
    }
    Ok(())
}

#[test]
fn qa_m207_mirror_full_state() {
    // λ̃_y below −δ_λ by far, at three gauges each.
    let x = representative([0.25, 0.35, 0.4], 0.8, -1.2, [0.3, -0.2], [0.1, 0.4]);
    let mut seen = [false; 2];
    for g in [
        Gauge {
            a: [0.0; 2],
            u: [0.0; 2],
            theta: 0.0,
            s: 1.0,
            mirror: false,
        },
        Gauge {
            a: [3.0, 1.0],
            u: [0.5, -0.5],
            theta: 2.2,
            s: 7.0,
            mirror: false,
        },
        Gauge {
            a: [-1.0, 2.0],
            u: [-0.2, 0.1],
            theta: -0.4,
            s: 0.2,
            mirror: true,
        },
    ] {
        let y = act(&g, &x);
        let out = canonicalise(&y);
        seen[usize::from(out.mirrored)] = true;
        lz_sign(canonicalise, &y).unwrap();
        // The mirror of the unmirrored canonical frame: compare with the reference's full-state mirror.
        let want = reference(&y);
        assert_eq!(out.mirrored, want.mirrored, "the mirror decision at {g:?}");
        let d = gap(&want.ic, &out.ic, f64::EPSILON);
        assert!(
            d <= REFERENCE_EPS,
            "C's state is {d} eps from the full-state mirror at {g:?}"
        );
    }
    assert_eq!(
        seen,
        [true, true],
        "the gauges must reach both sides of the mirror"
    );
}

negative_control!(
    qa_m207_mirror_full_state,
    "a config-only mirror keeps L_z's sign and must fail",
    expected = "not",
    {
        let x = representative([0.25, 0.35, 0.4], 0.8, -1.2, [0.3, -0.2], [0.1, 0.4]);
        assert!(canonicalise(&x).mirrored, "the control's state must mirror");
        lz_sign(config_only_mirror, &x).unwrap();
    }
);

// ── qa_m207_deadband_tilde (REQ-DEC-017) ────────────────────────────────────────────────────────────────────────

/// The `k` of `λ̃_y = k·δ_λ`: straddling `±1` and inside, with 0.
const KS: [f64; 9] = [-3.0, -1.1, -0.9, -0.5, 0.0, 0.5, 0.9, 1.1, 3.0];

/// Masses with `√μ_λ = √(m₂M₀₁) = 0.3`, so `|λ_y| = |λ̃_y|/0.3`: every `|k| ≥ 0.5` here has `|λ_y| > δ_λ`.
const LIGHT: [f64; 3] = [0.5, 0.4, 0.1];

/// The output `λ̃_y` of `C` and its mirror flag, against what the test on `λ̃_y` gives: the frame's
/// `λ̃_y = (Π ? −k : k)·δ_λ`, mirrored iff below `−δ_λ`, and then `+|λ̃_y|`; read to `ABS` absolute. Twice the same.
fn deadband(c: Canon<f64>, k: f64, g: &Gauge) -> Result<(), String> {
    const ABS: f64 = 1e-14;
    let x = representative(
        LIGHT,
        0.9,
        (k * DELTA / 0.9f64.sin()).asin(),
        [0.2, -0.1],
        [0.05, 0.3],
    );
    let y = act(g, &x);
    let out = c(&y);
    if c(&y) != out {
        return Err(format!("two results at k = {k}"));
    }
    let frame = if g.mirror { -k } else { k } * DELTA;
    let mirror = frame < -DELTA;
    if out.mirrored != mirror {
        return Err(format!(
            "the mirror is {} at λ̃_y = {frame:e}, k = {k}, {g:?}",
            out.mirrored
        ));
    }
    let want = if mirror { -frame } else { frame };
    let got = lambda_tilde(&out.ic)[1];
    if (got - want).abs() > ABS {
        return Err(format!(
            "the output's λ̃_y is {got:e}, not {want:e}, at k = {k}, {g:?}"
        ));
    }
    Ok(())
}

#[test]
fn qa_m207_deadband_tilde() {
    let m01 = LIGHT[0] + LIGHT[1];
    assert!(((LIGHT[2] * m01).sqrt() - 0.3).abs() < 1e-15, "√μ_λ = 0.3");
    let mut n = 0;
    for k in KS {
        for (j, theta) in [0.0, 0.37, 1.9, -2.8, std::f64::consts::FRAC_PI_2]
            .into_iter()
            .enumerate()
        {
            for mirror in [false, true] {
                let s = [1.0, 0.25, 6.0][j % 3];
                let g = Gauge {
                    a: [0.7 * s, -1.3 * s],
                    u: [0.4, -0.25],
                    theta,
                    s,
                    mirror,
                };
                deadband(canonicalise, k, &g).unwrap();
                n += 1;
            }
        }
    }
    // f32: states whose steps 1a, 1b and 2 are exact and step 0 is 1 to an ulp (`I = ½·2 = 1`), λ̃_y = −a = kδ_λ
    // with masses (¼, ¼, ½), √μ_λ = ½, so λ_y = 2kδ_λ.
    for k in KS {
        let a = (-k * DELTA) as f32;
        let x = Ic {
            m: [0.25f32, 0.25, 0.5],
            r: [
                [-std::f32::consts::SQRT_2, a],
                [std::f32::consts::SQRT_2, a],
                [0.0, -a],
            ],
            p: [[0.0; 2]; 3],
        };
        let out = canonicalise(&x);
        assert_eq!(canonicalise(&x), out, "f32: two results at k = {k}");
        let lt: f64 = f64::from(out.lambda_t_y);
        let pre = if out.mirrored { -lt } else { lt };
        assert_eq!(
            out.mirrored,
            k < -1.0,
            "f32: the mirror is {} at λ̃_y = {k}δ_λ (C read {pre:e})",
            out.mirrored
        );
        // r₂'s y follows the flag: −a before, +a after the mirror (to step 0's relative ulp).
        let r2y = f64::from(out.ic.r[2][1]);
        let want = if out.mirrored {
            f64::from(a)
        } else {
            -f64::from(a)
        };
        assert!(
            (r2y - want).abs() <= 4.0 * f64::from(f32::EPSILON) * want.abs(),
            "f32: r₂'s y is {r2y:e}, not {want:e}, at k = {k}"
        );
        n += 1;
    }
    eprintln!("qa_m207_deadband_tilde: {n} states");
}

/// `C` whose mirror test reads `λ_y = λ̃_y/√μ_λ`: the defect REQ-DEC-017 names.
#[cfg(feature = "controls")]
fn lambda_y_test(x: &Ic<f64>) -> Canonical<f64> {
    let mut c = canonicalise(x);
    // Undo C's own mirror, then mirror on λ_y.
    if c.mirrored {
        for v in c.ic.r.iter_mut().chain(c.ic.p.iter_mut()) {
            v[1] = -v[1];
        }
    }
    let m01 = x.m[0] + x.m[1];
    let ly = lambda_tilde(&c.ic)[1] / (x.m[2] * m01).sqrt();
    c.mirrored = ly < -DELTA;
    if c.mirrored {
        for v in c.ic.r.iter_mut().chain(c.ic.p.iter_mut()) {
            v[1] = -v[1];
        }
    }
    c
}

negative_control!(
    qa_m207_deadband_tilde,
    "a mirror test on λ_y, not λ̃_y, mirrors inside the deadband and must fail",
    expected = "the mirror is true",
    {
        let g = Gauge {
            a: [0.0; 2],
            u: [0.0; 2],
            theta: 0.37,
            s: 1.0,
            mirror: false,
        };
        deadband(lambda_y_test, -0.9, &g).unwrap();
    }
);

// ── qa_m207_noop_extreme (REQ-DEC-018) ──────────────────────────────────────────────────────────────────────────

/// The no-op's tolerance on the state, in eps: step 1a's and step 0's rounding on a state already at CoM 0, `I = 1`.
const NOOP_EPS: f64 = 8.0;

/// dd_decoder §5 test 5 on `C(x)`: no mirror, `|φ| ≤ NOOP_EPS·eps`, the state within `NOOP_EPS`, the same twice.
fn noop<R: P>(c: Canon<R>, x: &Ic<R>, at: &str) -> Result<f64, String> {
    let out = c(x);
    if c(x) != out {
        return Err(format!("two results at {at}"));
    }
    if out.mirrored {
        return Err(format!("the mirror was taken at {at}"));
    }
    let phi: f64 = out.phi.into();
    if phi.abs() > NOOP_EPS * eps::<R>() {
        return Err(format!("the rotation angle is {phi:e} at {at}"));
    }
    let d = gap(&wide(x), &wide(&out.ic), eps::<R>());
    if d > NOOP_EPS {
        return Err(format!("C moved the decode by {d} eps at {at}"));
    }
    Ok(d)
}

/// `z` with one control at each extreme finite value, the others at fixed interior values; and `(z_α, z_β)` together at
/// every pair of the extremes.
fn extreme_z<R: P>() -> Vec<[R; 8]> {
    let c = |v: f64| <R as spirv_std::num_traits::NumCast>::from(v).expect("finite");
    let big: f64 = R::max_value().into();
    let ext = [-big, -1e30, 1e30, big];
    let base = [0.3, -0.7, 0.4, -0.2, 0.9, -1.1, 0.5, -0.6];
    let mut out = Vec::new();
    for k in 0..8 {
        for e in ext {
            let mut z = base;
            z[k] = e;
            out.push(z.map(c));
        }
    }
    for a in ext {
        for b in ext {
            let mut z = base;
            z[0] = a;
            z[1] = b;
            out.push(z.map(c));
        }
    }
    out
}

fn noop_at<R: P>(name: &str) {
    let mut worst = 0.0f64;
    for (j, z) in extreme_z::<R>().into_iter().enumerate() {
        let x = decode::<Defaults, R>(&Latent::new(z)).ic;
        let at = format!("{name} z = {z:?} (case {j})");
        worst = worst.max(noop::<R>(canonicalise, &x, &at).unwrap());
    }
    eprintln!("qa_m207_noop_extreme {name}: worst {worst} eps (tolerance {NOOP_EPS})");
}

#[test]
fn qa_m207_noop_extreme() {
    noop_at::<f64>("f64");
    noop_at::<f32>("f32");
}

negative_control!(
    qa_m207_noop_extreme,
    "a C that turns a canonical decode by a quarter turn must fail test 5",
    expected = "the rotation angle is",
    {
        let x =
            decode::<Defaults, f64>(&Latent::new([0.3, -0.7, 0.4, -0.2, 0.9, -1.1, 0.5, -0.6])).ic;
        noop::<f64>(
            |x| {
                let mut c = canonicalise(x);
                c.phi = std::f64::consts::FRAC_PI_2;
                c
            },
            &x,
            "a turned C",
        )
        .unwrap();
    }
);

// ── qa_m207_scale_gauge_powers (REQ-DEC-023) ────────────────────────────────────────────────────────────────────

const GAUGE_EPS: f64 = 16.0;

/// dd_decoder §3.5 on a CoM-frame state `x` scaled by `s` (`r·s`, `p·s^{−1/2}`, so `I = s²·I(x)`): component by
/// component `rᵢ/ℓ` and `√ℓ·pᵢ`, `ℓ = √I`; `I = 1` after; `E·ℓ` and `L_z/√ℓ`; applied again, the identity; and the
/// same map as `C`'s step 0 on that state. The worst, in eps.
fn powers(gauge: fn(&Ic<f64>) -> Ic<f64>, x: &Ic<f64>, s: f64) -> Result<f64, String> {
    let e = f64::EPSILON;
    let y = Ic {
        m: x.m,
        r: x.r.map(|v| v.map(|c| c * s)),
        p: x.p.map(|v| v.map(|c| c / s.sqrt())),
    };
    let l = inertia_com(&y).sqrt();
    let want = Ic {
        m: y.m,
        r: y.r.map(|v| v.map(|c| c / l)),
        p: y.p.map(|v| v.map(|c| c * l.sqrt())),
    };
    let g = gauge(&y);
    let checks = [
        ("the components", gap(&want, &g, e)),
        ("I − 1", (inertia_com(&g) - 1.0).abs() / e),
        (
            "E",
            (energy(&g) - l * energy(&y)).abs() / (e * energy(&g).abs().max(1.0)),
        ),
        (
            "L_z",
            (lz(&g) - lz(&y) / l.sqrt()).abs() / (e * lz(&g).abs().max(1.0)),
        ),
        ("the gauge twice", gap(&g, &gauge(&g), e)),
        ("C's step 0", gap(&canonicalise(&y).ic, &g, e)),
    ];
    let mut worst = 0.0f64;
    for (what, d) in checks {
        if d > GAUGE_EPS {
            return Err(format!("{what}: {d} eps at s = {s}"));
        }
        worst = worst.max(d);
    }
    Ok(worst)
}

#[test]
fn qa_m207_scale_gauge_powers() {
    let mut worst = 0.0f64;
    for (j, (alpha, beta)) in [
        (0.3, 0.4),
        (0.8, 1.7),
        (1.3, 2.9),
        (std::f64::consts::FRAC_PI_4, 1.0),
    ]
    .into_iter()
    .enumerate()
    {
        let x = representative([0.2, 0.45, 0.35], alpha, beta, [0.4, -0.3], [-0.2, 0.5]);
        for s in [1.0, 1e-3, 0.37, 4.0, 2.5e3] {
            worst =
                worst.max(powers(scale_gauge, &x, s).unwrap_or_else(|f| panic!("state {j}: {f}")));
        }
    }
    eprintln!("qa_m207_scale_gauge_powers: worst {worst} eps (tolerance {GAUGE_EPS})");
}

negative_control!(
    qa_m207_scale_gauge_powers,
    "a scale gauge with the symmetric powers, pᵢ/ℓ, must fail",
    expected = "the components",
    {
        let x = representative([0.2, 0.45, 0.35], 0.8, 1.7, [0.4, -0.3], [-0.2, 0.5]);
        powers(
            |y| {
                let mut g = scale_gauge(y);
                let l = inertia_com(y).sqrt();
                g.p = y.p.map(|v| v.map(|c| c / l));
                g
            },
            &x,
            4.0,
        )
        .unwrap();
    }
);

// ── qa_m207_burrau_scale_step (REQ-ENC-006) ─────────────────────────────────────────────────────────────────────

/// chart_reference §4.2 at `(m, n) = (2, 1)`: `a = 3`, `b = 4`, `c = 5`, `r₀ = 0`, `r₁ = (a/c, 0)`, `r₂ = (0, b/c)`,
/// `m = (c, b, a)/(a + b + c)`; its CoM-frame `I`.
fn burrau() -> (Ic<f64>, f64) {
    let (a, b, c) = (3.0, 4.0, 5.0);
    let x = Ic {
        m: [c / 12.0, b / 12.0, a / 12.0],
        r: [[0.0, 0.0], [a / c, 0.0], [0.0, b / c]],
        p: [[0.0; 2]; 3],
    };
    let i = inertia_com(&x);
    (x, i)
}

/// The Burrau state at 2× the canonical scale: `r·2/√I`, the origin kept at body 0.
fn burrau_2x() -> Ic<f64> {
    let (x, i) = burrau();
    let k = 2.0 / i.sqrt();
    Ic {
        m: x.m,
        r: x.r.map(|v| v.map(|c| c * k)),
        p: x.p,
    }
}

const STEP_EPS: f64 = 16.0;

/// Encode test 4's row on `x` (at `I_in = 4`) and `C(x)`, at precision `R`: `I_in = 4·I_canon`, `λ = 1/2`,
/// `r ← (r − R_com)/2` and `p ← √2·p` (no rotation needed: the Burrau `ρ` is on `+x` and `λ̃_y > 0`),
/// `E_canon = 2·E_in`, `L_canon = L_in/√2`, and the time scale `T = I_in^{3/4} = 2^{3/2}`: velocities `λT·v` and
/// accelerations `λT²·a`. The worst, in eps.
fn row<R: P>(c: Canon<R>, x: &Ic<f64>) -> Result<f64, String> {
    let e = eps::<R>();
    let xr = narrow::<R>(x);
    let xin = wide(&xr);
    let out = c(&xr);
    let o = wide(&out.ic);
    let t = 2f64.powf(1.5);
    let lam = 0.5;
    let cm = com(&xin);
    let want = Ic {
        m: xin.m,
        r: xin.r.map(|v| [(v[0] - cm[0]) * lam, (v[1] - cm[1]) * lam]),
        p: xin.p.map(|v| v.map(|q| q * 2f64.sqrt())),
    };
    let vel = |s: &Ic<f64>| [0, 1, 2].map(|i| s.p[i].map(|q| q / s.m[i]));
    let (vin, vout) = (vel(&xin), vel(&o));
    let (ain, aout) = (accelerations(&xin), accelerations(&o));
    let vs: f64 = vin
        .iter()
        .map(|v| norm(*v))
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    let as_: f64 = ain.iter().map(|v| norm(*v)).fold(0.0, f64::max);
    let mut dv = 0.0f64;
    let mut da = 0.0f64;
    for i in 0..3 {
        for k in 0..2 {
            dv = dv.max((vout[i][k] - lam * t * vin[i][k]).abs() / (lam * t * vs * e));
            da = da.max((aout[i][k] - lam * t * t * ain[i][k]).abs() / (lam * t * t * as_ * e));
        }
    }
    let i_in: f64 = out.i_in.into();
    let lambda: f64 = out.lambda.into();
    let ecale = energy(&o).abs();
    let checks = [
        ("velocities × λ·I_in^(3/4)", dv),
        ("accelerations × λ·I_in^(3/2)", da),
        ("I_in = 4", (i_in - 4.0).abs() / (4.0 * e)),
        (
            "I_in = 4·I_canon",
            (i_in - 4.0 * inertia_com(&o)).abs() / (4.0 * e),
        ),
        ("λ = 1/2", (lambda - 0.5).abs() / (0.5 * e)),
        ("the state", gap(&want, &o, e)),
        (
            "E_canon = 2·E_in",
            (energy(&o) - 2.0 * energy(&xin)).abs() / (ecale * e),
        ),
        (
            "L_canon = L_in/√2",
            (lz(&o) - lz(&xin) / 2f64.sqrt()).abs()
                / (e * (0..3)
                    .map(|i| norm(o.r[i]) * norm(o.p[i]))
                    .sum::<f64>()
                    .max(f64::MIN_POSITIVE)),
        ),
    ];
    if out.mirrored || out.clamped {
        return Err(format!(
            "a notice besides λ: mirrored {} clamped {}",
            out.mirrored, out.clamped
        ));
    }
    let mut worst = 0.0f64;
    for (what, d) in checks {
        if d > STEP_EPS {
            return Err(format!("{what}: {d} eps off"));
        }
        worst = worst.max(d);
    }
    Ok(worst)
}

#[test]
fn qa_m207_burrau_scale_step() {
    let (base, i) = burrau();
    assert!(
        (i - 0.2).abs() < 1e-15,
        "§4.2's hypotenuse-normalised Burrau triangle has I = 1/5, got {i}"
    );
    let x = burrau_2x();
    assert!(
        (inertia_com(&x) - 4.0).abs() < 1e-14,
        "2× scale is I_in = 4"
    );
    // The checked-in fixture is this construction.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/states/burrau_2x.json");
    let j: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("fixtures/states/burrau_2x.json"),
    )
    .expect("JSON");
    for b in 0..3 {
        assert_eq!(j["m"][b].as_f64(), Some(base.m[b]), "the fixture's m{b}");
        for k in 0..2 {
            let f = j["r"][b][k].as_f64().expect("a number");
            assert!(
                (f - x.r[b][k]).abs() <= 2.0 * f64::EPSILON * x.r[b][k].abs(),
                "the fixture's r{b}[{k}] = {f}, built {}",
                x.r[b][k]
            );
            assert_eq!(j["p"][b][k].as_f64(), Some(0.0), "the fixture is at rest");
        }
    }
    // At rest, and with momenta of zero sum (so the E, L_z and velocity rows carry kinetic terms).
    let moving = Ic {
        p: [[0.3, -0.1], [-0.2, 0.25], [-0.1, -0.15]],
        ..x
    };
    for (what, s) in [("at rest", x), ("moving", moving)] {
        let w64 = row::<f64>(canonicalise, &s).unwrap_or_else(|f| panic!("f64 {what}: {f}"));
        let w32 = row::<f32>(canonicalise, &s).unwrap_or_else(|f| panic!("f32 {what}: {f}"));
        eprintln!("qa_m207_burrau_scale_step {what}: worst {w64} eps (f64), {w32} eps (f32); tolerance {STEP_EPS}");
    }
}

negative_control!(
    qa_m207_burrau_scale_step,
    "a step 0 that leaves the momenta unscaled must fail the velocity row (the timing factor)",
    expected = "velocities",
    {
        let x = Ic {
            p: [[0.3, -0.1], [-0.2, 0.25], [-0.1, -0.15]],
            ..burrau_2x()
        };
        row::<f64>(
            |y| {
                let mut c = canonicalise(y);
                c.ic.p = c.ic.p.map(|v| v.map(|q| q / 2f64.sqrt()));
                c
            },
            &x,
        )
        .unwrap();
    }
);

// ── qa_m207_coincident_pole (REQ-ENC-016) ───────────────────────────────────────────────────────────────────────

/// States with `ρ = 0` exactly after steps 1a and 0, `I_in > 0`, named by `ρ`'s zeros: `r₀ = r₁` bit for bit gives
/// `(+0, +0)`; `r₀ = (+0, 0)`, `r₁ = (−0, 0)` with body 2 on the y axis keeps `R_com,x = +0`, so `ρ_x = −0 − (+0) = −0`;
/// and the transpose gives `ρ_y = −0`.
fn poles() -> Vec<(&'static str, Ic<f64>)> {
    let m = [0.3, 0.25, 0.45];
    let p = [[0.1, -0.3], [0.2, 0.05], [-0.25, 0.4]];
    vec![
        (
            "(+0, +0)",
            Ic {
                m,
                r: [[0.7, -1.2], [0.7, -1.2], [-0.4, 2.0]],
                p,
            },
        ),
        (
            "(−0, +0)",
            Ic {
                m,
                r: [[0.0, 0.0], [-0.0, 0.0], [0.0, 1.5]],
                p,
            },
        ),
        (
            "(+0, −0)",
            Ic {
                m,
                r: [[0.0, 0.0], [0.0, -0.0], [1.5, 0.0]],
                p,
            },
        ),
    ]
}

/// At `ρ = 0`: `lookup_clamped`, `φ = +0` by its bits, and the output the reference's with no rotation (R-437:
/// `R(−φ)` the identity), its own mirror flag applied; at precision `R`.
fn pole<R: P>(c: Canon<R>, name: &str, x: &Ic<f64>) -> Result<(), String> {
    let xr = narrow::<R>(x);
    let out = c(&xr);
    let phi: f64 = out.phi.into();
    if !out.clamped {
        return Err(format!("no lookup_clamped at ρ = {name}"));
    }
    if phi.to_bits() != 0.0f64.to_bits() {
        return Err(format!("φ = {phi} at ρ = {name}, not +0"));
    }
    let want = reference(&wide(&xr));
    if !want.clamped || want.mirrored != out.mirrored {
        return Err(format!(
            "the reference disagrees at ρ = {name}: mirrored {} vs {}",
            want.mirrored, out.mirrored
        ));
    }
    let d = gap(&want.ic, &wide(&out.ic), eps::<R>());
    if d > REFERENCE_EPS {
        return Err(format!(
            "the rotation at ρ = {name} is not the identity: {d} eps"
        ));
    }
    Ok(())
}

#[test]
fn qa_m207_coincident_pole() {
    for (name, x) in poles() {
        // The construction reaches the signed zero it is named for.
        let r = reference(&x);
        assert!(r.clamped && r.i_in > 0.0, "{name}: ρ = 0 with I_in > 0");
        pole::<f64>(canonicalise, name, &x).unwrap();
        pole::<f32>(canonicalise, name, &x).unwrap();
    }
    // I_in = 0, every body at one point (RQ-296 holds the state and λ): only the flag and φ.
    for (sx, sy) in [(0.0, 0.0), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        let x = Ic {
            m: [0.2, 0.3, 0.5],
            r: [[0.0, 0.0], [sx, sy], [0.0, 0.0]],
            p: [[0.1, 0.0], [0.0, 0.2], [-0.1, -0.2]],
        };
        let c64 = canonicalise(&x);
        let c32 = canonicalise(&narrow::<f32>(&x));
        assert!(
            c64.clamped && c32.clamped,
            "no lookup_clamped at I_in = 0, ρ = ({sx}, {sy})"
        );
        assert_eq!(
            c64.phi.to_bits(),
            0.0f64.to_bits(),
            "f64 φ at I_in = 0, ρ = ({sx}, {sy})"
        );
        assert_eq!(
            c32.phi.to_bits(),
            0.0f32.to_bits(),
            "f32 φ at I_in = 0, ρ = ({sx}, {sy})"
        );
    }
    // ρ off 0: the inner pair one representable step apart in each direction, both precisions: no clamp.
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        let base = poles()[0].1;
        let mut x = base;
        x.r[1] = [x.r[0][0] + dx * 1e-6, x.r[0][1] + dy * 1e-6];
        assert!(
            !canonicalise(&x).clamped,
            "f64 clamped at ρ = ({dx}, {dy})·1e-6"
        );
        assert!(
            !canonicalise(&narrow::<f32>(&x)).clamped,
            "f32 clamped at ρ = ({dx}, {dy})·1e-6"
        );
    }
}

negative_control!(
    qa_m207_coincident_pole,
    "a C whose φ is atan2 at the pole, π for a −0 ρ_x, turns the state by a half turn and must fail",
    expected = "φ = 3.14",
    {
        let (name, x) = poles()[1];
        pole::<f64>(
            |y| {
                let mut c = canonicalise(y);
                let rh = [y.r[1][0] - y.r[0][0], y.r[1][1] - y.r[0][1]];
                c.phi = rh[1].atan2(rh[0]);
                c
            },
            name,
            &x,
        )
        .unwrap();
    }
);
