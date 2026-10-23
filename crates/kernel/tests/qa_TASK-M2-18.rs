//! QA's tests for TASK-M2-18, the Burrau encode (inverse_encode_contract Part 5 kind 4; dd_encode §3.1, §3.4, §5
//! tests 5 and 7; R-24, R-27), written from the requirements, not from the implementation:
//! - REQ-ENC-011: the physical `(3, 4, 5)` triangle, legs 3 and 4 at their own size, each mass its opposite side,
//!   `(5, 4, 3)/12`, under random gauges: the right angle recovered, `θ = arctan(3/4)`, `ν = sec θ − tan θ = ½`, the
//!   coupling `μ₁ = log(b/c)`, `μ₂ = log(a/c)` holding, and the decode back at the chart point the same triangle to
//!   `ε_phys`; across the family, the closed form inverts `θ(ν)`; and a state whose masses break the coupling is
//!   reported, by the size of the break, and does not land on the curve.
//! - REQ-ENC-017: on the curve the argmin and the closed form agree on `ν`; off it `d(ν*)` is reported as the test's
//!   own metric gives it at `ν*`, the latent fallback is taken beyond the tolerance, never a curve landing, and the
//!   fallback's `z` satisfies T2 (`‖D(z) − C(x)‖_phys ≤ ε_phys`).
//! - REQ-ENC-020: the projected point minimises the shape sphere's chordal ⊕ the mass simplex's Euclidean distance at
//!   unit weights, against a brute-force search under the test's own metric, built from the raw positions with its
//!   own Jacobi weighting and shape map (chart_reference §3.1, §4.2), not from the kernel's. Its control is the same
//!   search under another weighting.
//! - REQ-ENC-022: on a folded chart the leg-swapped twin (bodies 1 and 2 exchanged) is not relabelled: it falls back to
//!   latent `z`, flagged `past_fold`; the isosceles triangle is not past the fold; on the full-range Euclid plane the
//!   twin is its own chart point at `ν′ = (1 − ν)/(1 + ν)`.
//! - Past the span (dd_encode §3.4): a state on the family's curve but past the chart's `ν` span is no chart point; its
//!   `ν*` is the span's edge exactly, flagged.
//!
//! **Tolerances.** The curve tolerance is `k·ε_R` with `k` read from the calibration fixture
//! `fixtures/gates/curve_projection/measurement.toml` (REQ-ENC-026, provisional until the M2 gate); physical
//! round trips are held to `ε_phys` (REQ-ENC-024, provisional until the M2 gate) in Part 4's `‖·‖_phys`. The closed
//! form's scalars (`θ`, `ν` and the coupling, each O(1) and Lipschitz in the positions at O(1)) are held to `ε_phys`
//! too. The brute-force comparison allows only the rounding of one `d²` evaluation, 16 `ε` relative.
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory.

use std::f64::consts::FRAC_PI_4;

use kernel::chart::{BurrauThetaK, ChartDecode};
use kernel::decode::{self as dec, Ic};
use kernel::encode::by_kind::{encode_into_within, ChartEncoded, Coord, Landing};
use kernel::encode::curve::{closed_form, nearest, project, state_point, CurveChart, CurveLanding};
use kernel::encode::{encode, Encoded};
use kernel::generated::links::Defaults;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;
use validation::roundtrip::charts::{burrau_theta_k, euclid_plane};
use validation::roundtrip::{eps_phys, phys, widen, Precision};

// ── The test's own model of the family (chart_reference §4.2, §4.3) ────────────────────────────────────────────────

/// The physical right triangle with legs `a = ‖r₁ − r₀‖` and `b = ‖r₂ − r₀‖`, the right angle at body 0, each mass its
/// opposite side over the perimeter, at rest: the Burrau convention at any size.
fn triangle(a: f64, b: f64) -> Ic<f64> {
    let c = a.hypot(b);
    let s = a + b + c;
    Ic {
        m: [c / s, b / s, a / s],
        r: [[0.0, 0.0], [a, 0.0], [0.0, b]],
        p: [[0.0; 2]; 3],
    }
}

/// The family at `ν` (Euclid's formulae), at the physical size `size`.
fn family(nu: f64, size: f64) -> Ic<f64> {
    triangle(size * (1.0 - nu * nu), size * 2.0 * nu)
}

/// `x` with bodies 1 and 2 exchanged, positions, momenta and masses.
fn swapped(x: &Ic<f64>) -> Ic<f64> {
    Ic {
        m: [x.m[0], x.m[2], x.m[1]],
        r: [x.r[0], x.r[2], x.r[1]],
        p: [x.p[0], x.p[2], x.p[1]],
    }
}

/// The test's own point of the metric's space: the shape-sphere point of the mass-weighted Jacobi vectors (inner
/// pair (0, 1)), `n = (‖ρ̃‖² − ‖λ̃‖², 2ρ̃·λ̃, 2|ρ̃ ∧ λ̃|)/(‖ρ̃‖² + ‖λ̃‖²)` (chart_reference §3.1, the cross read as its
/// absolute value as the encode's `β` reads `|λ̃_y|`), and the masses over their total.
fn own_point(x: &Ic<f64>) -> ([f64; 3], [f64; 3]) {
    let [m0, m1, m2] = x.m;
    let total = m0 + m1 + m2;
    let m01 = m0 + m1;
    let rho = [x.r[1][0] - x.r[0][0], x.r[1][1] - x.r[0][1]];
    let com = [
        (m0 * x.r[0][0] + m1 * x.r[1][0]) / m01,
        (m0 * x.r[0][1] + m1 * x.r[1][1]) / m01,
    ];
    let lam = [x.r[2][0] - com[0], x.r[2][1] - com[1]];
    let (wr, wl) = ((m0 * m1 / m01).sqrt(), (m01 * m2 / total).sqrt());
    let rt = [wr * rho[0], wr * rho[1]];
    let lt = [wl * lam[0], wl * lam[1]];
    let aa = rt[0] * rt[0] + rt[1] * rt[1];
    let bb = lt[0] * lt[0] + lt[1] * lt[1];
    let i = aa + bb;
    let dot = rt[0] * lt[0] + rt[1] * lt[1];
    let cross = rt[0] * lt[1] - rt[1] * lt[0];
    (
        [(aa - bb) / i, 2.0 * dot / i, 2.0 * cross.abs() / i],
        [m0 / total, m1 / total, m2 / total],
    )
}

/// `d²(ν)` under weights `(1, w)` on the shape and mass terms; R-24's metric is `w = 1`.
fn own_d2(x: &Ic<f64>, nu: f64, w: f64) -> f64 {
    let (n, m) = own_point(x);
    let (ng, mg) = own_point(&family(nu, 1.0));
    let shape: f64 = (0..3).map(|k| (n[k] - ng[k]).powi(2)).sum();
    let mass: f64 = (0..3).map(|k| (m[k] - mg[k]).powi(2)).sum();
    shape + w * mass
}

/// The brute-force minimum of [`own_d2`] over `BRUTE + 1` evenly spaced `ν` on `[lo, hi]`, edges included.
const BRUTE: usize = 20000;

fn brute(x: &Ic<f64>, span: [f64; 2], w: f64) -> (f64, f64) {
    (0..=BRUTE)
        .map(|k| {
            let nu = span[0] + (span[1] - span[0]) * k as f64 / BRUTE as f64;
            (nu, own_d2(x, nu, w))
        })
        .fold(
            (f64::NAN, f64::INFINITY),
            |b, c| if c.1 < b.1 { c } else { b },
        )
}

// ── Plumbing ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// REQ-ENC-026's proposed `k`, read from its calibration fixture, not chosen here.
fn k_curve() -> f64 {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/gates/curve_projection/measurement.toml"
    );
    let text = std::fs::read_to_string(path).expect("the calibration fixture is checked in");
    let line = text
        .lines()
        .find(|l| l.starts_with("k_curve"))
        .expect("the fixture proposes k_curve");
    let value = line.split('=').nth(1).unwrap().split('#').next().unwrap();
    value.trim().parse().expect("k_curve is a number")
}

fn tol<R: Precision>() -> f64 {
    k_curve() * R::EPS
}

fn narrow<R: Precision>(x: &Ic<f64>) -> Ic<R> {
    Ic {
        m: x.m.map(R::of),
        r: x.r.map(|v| v.map(R::of)),
        p: x.p.map(|v| v.map(R::of)),
    }
}

fn enc<R: Precision>(x: &Ic<f64>) -> Encoded<R> {
    encode::<Defaults, R>(&narrow::<R>(x))
}

fn f<R: Precision>(x: R) -> f64 {
    x.into()
}

fn norm<R: Precision>(a: &Ic<R>, b: &Ic<R>) -> R {
    R::of(phys(&widen(a), &widen(b)))
}

fn into<R, C>(chart: &C, x: &Ic<f64>) -> ChartEncoded<R>
where
    R: Precision + From<C::Param>,
    C: kernel::encode::by_kind::ByKind,
    f64: From<C::Param>,
{
    encode_into_within::<Defaults, R, C, _>(
        chart,
        &narrow::<R>(x),
        norm::<R>,
        R::of(eps_phys::<R>()),
        R::of(tol::<R>()),
    )
}

/// T2 for `E(x)`'s latent `z`: the free decode of `z` is `C(x)` to `ε_phys`.
fn check_t2<R: Precision>(e: &Encoded<R>, what: &str) {
    let back = dec::decode::<Defaults, R>(&e.z);
    let r = phys(&widen(&e.canonical.ic), &widen(&back.ic));
    assert!(
        r <= eps_phys::<R>(),
        "{what}: the latent fallback breaks T2, residual {r:e} > ε_phys {:e}",
        eps_phys::<R>()
    );
}

// ── REQ-ENC-011: the physical (3, 4, 5) ────────────────────────────────────────────────────────────────────────────

/// dd_encode §5 test 5 on the physical triangle with legs `(a, b)` at their own size, expecting `ν = nu`, under a
/// random gauge, at `R`.
fn check_345<R: Precision>(legs: [f64; 2], nu: f64, rng: &mut SplitMix)
where
    f64: From<R>,
{
    let x = Gauge::random(rng, 3.0, 2.0).act(&triangle(legs[0], legs[1]));
    let e = enc::<R>(&x);
    let eps = eps_phys::<R>();
    let cf = closed_form(&e.canonical.ic);
    let theta = f(cf.theta);
    assert!(
        (theta - (legs[0] / legs[1]).atan()).abs() <= eps,
        "θ = {theta} is not the acute angle arctan(a/b)"
    );
    let secant = 1.0 / theta.cos() - theta.tan();
    assert!(
        (f(cf.nu) - secant).abs() <= eps,
        "ν = {} is not sec θ − tan θ = {secant}",
        f(cf.nu)
    );
    assert!(
        (f(cf.nu) - nu).abs() <= eps,
        "ν = {} is not the expected {nu}",
        f(cf.nu)
    );
    let c = widen(&e.canonical.ic);
    let (b, a, cc) = (
        (c.r[2][0] - c.r[0][0]).hypot(c.r[2][1] - c.r[0][1]),
        (c.r[1][0] - c.r[0][0]).hypot(c.r[1][1] - c.r[0][1]),
        (c.r[2][0] - c.r[1][0]).hypot(c.r[2][1] - c.r[1][1]),
    );
    let mu = [(c.m[1] / c.m[0]).ln(), (c.m[2] / c.m[0]).ln()];
    let own = [mu[0] - (b / cc).ln(), mu[1] - (a / cc).ln()];
    for k in 0..2 {
        assert!(
            (f(cf.coupling[k]).abs() - own[k].abs()).abs() <= eps,
            "the reported coupling {:?} is not the state's own break {own:?}",
            cf.coupling.map(f)
        );
    }
    // The chart: the Euclid plane, decoded back at the chart point.
    let chart = euclid_plane::<R>();
    let back = into::<R, _>(&chart, &x);
    let (u, v) = match back.landing {
        Landing::Chart { u, v } => (u, v),
        Landing::Latent => panic!("(3, 4, 5) does not land on the Euclid plane"),
    };
    let d = chart.decode_at::<Defaults, R>(u, v).ic;
    let r = phys(&widen(&e.canonical.ic), &widen(&d));
    assert!(
        r <= eps,
        "decoded back, the triangle is not C(x) to ε_phys: {r:e}"
    );
    let dm = widen(&d);
    let twelfths = [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0];
    for (m, want) in dm.m.iter().zip(twelfths) {
        assert!(
            (m - want).abs() <= eps,
            "the decoded masses {:?} are not (5, 4, 3)/12",
            dm.m
        );
    }
    let l1 = [dm.r[1][0] - dm.r[0][0], dm.r[1][1] - dm.r[0][1]];
    let l2 = [dm.r[2][0] - dm.r[0][0], dm.r[2][1] - dm.r[0][1]];
    let cos = (l1[0] * l2[0] + l1[1] * l2[1]) / (l1[0].hypot(l1[1]) * l2[0].hypot(l2[1]));
    assert!(
        cos.abs() <= eps,
        "the right angle is not recovered at body 0: cos = {cos:e}"
    );
}

#[test]
fn qa_encode_burrau_345() {
    let mut rng = SplitMix::new(0x345);
    for _ in 0..64 {
        check_345::<f64>([3.0, 4.0], 0.5, &mut rng);
        check_345::<f32>([3.0, 4.0], 0.5, &mut rng);
    }
    // The (3, 4, 5) at the historical size, scaled tenfold: the closed form is scale-free.
    check_345::<f64>([30.0, 40.0], 0.5, &mut rng);
}

negative_control!(
    qa_encode_burrau_345,
    "the legs swapped, (4, 3, 5), is another curve point, ν = 1/3, past the fold",
    expected = "is not the expected",
    check_345::<f64>([4.0, 3.0], 0.5, &mut SplitMix::new(0x345))
);

/// Across the family, at a random physical size and gauge: the closed form inverts `θ(ν)`, the coupling holds, and
/// `past_fold` is exactly `a > b`.
fn check_family(nus: &[f64], expect: impl Fn(f64) -> f64) {
    let mut rng = SplitMix::new(11);
    let eps = eps_phys::<f64>();
    for &nu in nus {
        let size = uniform(&mut rng, -3.0, 3.0).exp2();
        let x = Gauge::random(&mut rng, 3.0, 2.0).act(&family(nu, size));
        let cf = closed_form(&enc::<f64>(&x).canonical.ic);
        let want = expect(nu);
        assert!(
            (cf.nu - want).abs() <= eps,
            "at ν = {nu}: the closed form gives ν = {}, not {want}",
            cf.nu
        );
        let (a, b) = (1.0 - nu * nu, 2.0 * nu);
        assert!(
            (cf.theta - a.atan2(b)).abs() <= eps,
            "at ν = {nu}: θ is not arctan(a/b)"
        );
        // `log(b/c)` and `log(a/c)` carry a side's relative error: ε_phys over the side's share of `c`.
        let c = 1.0 + nu * nu;
        let bound = [eps * c / b, eps * c / a];
        assert!(
            (0..2).all(|k| cf.coupling[k].abs() <= bound[k]),
            "at ν = {nu}: the coupling fails on the curve, {:?} beyond {bound:?}",
            cf.coupling
        );
        assert_eq!(cf.past_fold, a > b, "at ν = {nu}: past_fold is not a > b");
    }
}

#[test]
fn qa_encode_burrau_closed_form_family() {
    let nus: Vec<f64> = (1..50).map(|k| k as f64 / 50.0).collect();
    check_family(&nus, |nu| nu);
}

negative_control!(
    qa_encode_burrau_closed_form_family,
    "ν read as tan θ, not sec θ − tan θ, is not the family's ν",
    expected = "the closed form gives",
    check_family(&[0.3, 0.5], |nu| (1.0 - nu * nu) / (2.0 * nu))
);

/// A state whose masses break the coupling: the (3, 4, 5) with the legs' masses exchanged, `(5, 3, 4)/12`. The break
/// is `|log(4/3)|` in each `μ`, and the state is not on the curve: it lands nowhere on it at the curve tolerance.
fn check_broken_coupling(m: [f64; 3]) {
    let mut x = triangle(3.0, 4.0);
    x.m = m;
    let e = enc::<f64>(&x);
    let cf = closed_form(&e.canonical.ic);
    let want = (4.0f64 / 3.0).ln();
    for k in 0..2 {
        assert!(
            (cf.coupling[k].abs() - want).abs() <= eps_phys::<f64>(),
            "the coupling break {:?} is not ±log(4/3)",
            cf.coupling
        );
    }
    let p = project(&e, euclid_plane::<f64>().nu_span::<f64>(), tol::<f64>());
    assert!(
        p.landing == CurveLanding::Latent,
        "a state off the coupling lands on the curve: {:?}",
        p.landing
    );
}

#[test]
fn qa_encode_burrau_coupling_break() {
    check_broken_coupling([5.0 / 12.0, 3.0 / 12.0, 4.0 / 12.0]);
}

negative_control!(
    qa_encode_burrau_coupling_break,
    "the Burrau masses themselves have no break",
    expected = "is not ±log(4/3)",
    check_broken_coupling([5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0])
);

// ── REQ-ENC-017: the projection, dd_encode §5 test 7 ───────────────────────────────────────────────────────────────

/// On the curve: the argmin and the closed form agree on `ν` to the curve tolerance, and the closed form lands.
fn check_on_curve<R: Precision>(nu: f64, rng: &mut SplitMix, offset: f64)
where
    f64: From<R>,
{
    let x = Gauge::random(rng, 3.0, 2.0).act(&family(nu, 1.0));
    let e = enc::<R>(&x);
    let span = euclid_plane::<R>().nu_span::<R>();
    let star = nearest(&state_point(&e.canonical.ic), span);
    let cf = closed_form(&e.canonical.ic);
    let t = tol::<R>();
    let gap = (f(star.nu) + offset - f(cf.nu)).abs();
    assert!(
        gap <= t,
        "at ν = {nu}: argmin and closed form disagree by {gap:e} > tol {t:e}"
    );
    let p = project(&e, span, R::of(t));
    assert!(
        matches!(p.landing, CurveLanding::OnCurve { .. }),
        "at ν = {nu}: an exact curve point does not land on the curve, d = {:e}",
        f(p.d)
    );
}

#[test]
fn qa_encode_curve_projection_on_curve() {
    let mut rng = SplitMix::new(7);
    for k in 0..256 {
        let nu = 0.04 + 0.92 * (k as f64 + 0.5) / 256.0;
        check_on_curve::<f64>(nu, &mut rng, 0.0);
        check_on_curve::<f32>(nu, &mut rng, 0.0);
    }
}

negative_control!(
    qa_encode_curve_projection_on_curve,
    "an argmin off by 1e-6 in ν is caught",
    expected = "argmin and closed form disagree",
    check_on_curve::<f64>(0.5, &mut SplitMix::new(7), 1e-6)
);

/// The family at `ν`, each position moved by up to `delta` per component and the masses by up to `delta/4`
/// (renormalised), under a random gauge.
fn off_curve(nu: f64, delta: f64, rng: &mut SplitMix) -> Ic<f64> {
    let mut x = family(nu, 1.0);
    for r in x.r.iter_mut() {
        for c in r.iter_mut() {
            *c += uniform(rng, -delta, delta);
        }
    }
    for m in x.m.iter_mut() {
        *m *= 1.0 + uniform(rng, -delta / 4.0, delta / 4.0);
    }
    let s: f64 = x.m.iter().sum();
    x.m = x.m.map(|m| m / s);
    Gauge::random(rng, 3.0, 2.0).act(&x)
}

/// Off the curve at `R`, with the projection run at `tol_run`: `d` is reported, and is the test's own distance at the
/// reported `ν*` (at f64), no smaller than the brute force's; beyond the curve tolerance the landing is latent `z`,
/// never the curve, and that `z` satisfies T2.
fn check_off<R: Precision>(x: &Ic<f64>, tol_run: f64) -> bool
where
    f64: From<R>,
{
    let e = enc::<R>(x);
    let chart = euclid_plane::<R>();
    let span = chart.nu_span::<R>();
    let p = project(&e, span, R::of(tol_run));
    let t = tol::<R>();
    // `d` is at `ν*` where the argmin ran, at the closed form's `ν` where it was bypassed.
    let at = f(p.projection.map_or(p.closed.nu, |q| q.nu));
    let d = f(p.d);
    assert!(d.is_finite(), "d(ν*) is not reported: {d}");
    if R::EPS == f64::EPSILON {
        let own = own_d2(x, at, 1.0).sqrt();
        assert!(
            (d - own).abs() <= t,
            "the reported d(ν*) = {d:e} is not the metric's at ν* = {at}: {own:e}"
        );
    }
    if d > t {
        assert!(
            p.landing == CurveLanding::Latent,
            "d(ν*) = {d:e} > tol {t:e}, yet the landing is {:?}, not latent z",
            p.landing
        );
        let c = into::<R, _>(&chart, x);
        assert!(
            c.landing == Landing::Latent && matches!(c.coords[1], Coord::Unplaced),
            "beyond the tolerance the chart still places the state: {:?}",
            c.landing
        );
        check_t2::<R>(&e, "off the curve");
        return true;
    }
    false
}

#[test]
fn qa_encode_curve_projection_fallback() {
    let mut rng = SplitMix::new(17);
    let mut fell_back = 0;
    for &delta in &[1e-3, 1e-2, 1e-1] {
        for _ in 0..48 {
            let nu = uniform(&mut rng, 0.1, 0.9);
            let x = off_curve(nu, delta, &mut rng);
            fell_back += usize::from(check_off::<f64>(&x, tol::<f64>()));
            fell_back += usize::from(check_off::<f32>(&x, tol::<f32>()));
        }
    }
    // Every state moved by δ ≥ 1e-3 lies far beyond k·ε_R at both precisions: the fallback branch ran each time.
    assert_eq!(
        fell_back,
        2 * 3 * 48,
        "some off-curve states were not beyond the tolerance"
    );
}

negative_control!(
    qa_encode_curve_projection_fallback,
    "a projection that ignores the tolerance (run at tol = 1) lands an off-curve state on the curve",
    expected = "not latent z",
    {
        check_off::<f64>(&off_curve(0.5, 1e-2, &mut SplitMix::new(17)), 1.0);
    }
);

/// Within the tolerance: a state moved off the curve by a few ε, the projection run at `tol_run`, lands on the curve,
/// at `ν` within the tolerance of its own.
fn check_within(tol_run: f64) {
    let mut rng = SplitMix::new(19);
    let t = tol::<f64>();
    for _ in 0..64 {
        let nu = uniform(&mut rng, 0.1, 0.9);
        let x = off_curve(nu, 4.0 * f64::EPSILON, &mut rng);
        let e = enc::<f64>(&x);
        let p = project(&e, euclid_plane::<f64>().nu_span::<f64>(), tol_run);
        let at = match p.landing {
            CurveLanding::OnCurve { nu } | CurveLanding::Projected { nu } => nu,
            CurveLanding::Latent => {
                panic!("a state within 4 ε of γ({nu}) falls back, d = {:e}", p.d)
            }
        };
        assert!(p.d <= t, "landed at d = {:e} > tol", p.d);
        assert!((at - nu).abs() <= t, "landed at ν = {at}, not γ({nu})'s");
    }
}

#[test]
fn qa_encode_curve_projection_within() {
    check_within(tol::<f64>());
}

negative_control!(
    qa_encode_curve_projection_within,
    "a projection that lands only exact curve points (run at tol = 0) drops a state a few ε off the curve to latent z",
    expected = "falls back",
    check_within(0.0)
);

// ── REQ-ENC-020: the metric against a brute-force search ───────────────────────────────────────────────────────────

/// Off-curve fixtures: the shape of one curve point with the masses of another (so the shape and the mass terms pull
/// `ν*` apart and the weighting decides it), and random off-curve states.
fn metric_fixtures() -> Vec<Ic<f64>> {
    let mut out = Vec::new();
    for &(shape_nu, mass_nu) in &[
        (0.3, 0.7),
        (0.7, 0.3),
        (0.2, 0.5),
        (0.6, 0.85),
        (0.45, 0.15),
    ] {
        let mut x = family(shape_nu, 1.0);
        x.m = family(mass_nu, 1.0).m;
        out.push(x);
    }
    let mut rng = SplitMix::new(23);
    for _ in 0..12 {
        let nu = uniform(&mut rng, 0.1, 0.9);
        out.push(off_curve(nu, 0.1, &mut rng));
    }
    out
}

/// `ν*` minimises the metric with weights `(1, w)`: its value there is no greater than at any brute-force point, up to
/// the rounding of one evaluation.
fn check_minimises(x: &Ic<f64>, w: f64) {
    let e = enc::<f64>(x);
    let span = euclid_plane::<f64>().nu_span::<f64>();
    let star = nearest(&state_point(&e.canonical.ic), span);
    let (bnu, bf) = brute(x, span, w);
    let at = own_d2(x, star.nu, w);
    assert!(
        at <= bf * (1.0 + 16.0 * f64::EPSILON) + f64::MIN_POSITIVE,
        "ν* = {} does not minimise the metric: d² there {at:e} > {bf:e} at ν = {bnu}",
        star.nu
    );
    let reported = star.d2;
    assert!(
        (reported - own_d2(x, star.nu, 1.0)).abs() <= tol::<f64>(),
        "the reported d² {reported:e} is not the unit-weight metric's at ν*"
    );
}

#[test]
fn qa_encode_projection_metric_bruteforce() {
    for x in metric_fixtures() {
        check_minimises(&x, 1.0);
    }
}

negative_control!(
    qa_encode_projection_metric_bruteforce,
    "under a mass weight of 4, the unit-weight ν* is not the minimiser on the mixed fixture",
    expected = "does not minimise the metric",
    check_minimises(&metric_fixtures()[0], 4.0)
);

/// Past the span (dd_encode §3.4): a state on the family's curve at `ν` below the Euclid plane's `1/32` is no chart
/// point: not on the curve, `ν*` the span's lower edge exactly, flagged, and beyond the tolerance it is latent.
fn check_past_span(nu: f64) {
    let x = family(nu, 1.0);
    let e = enc::<f64>(&x);
    let span = euclid_plane::<f64>().nu_span::<f64>();
    let p = project(&e, span, tol::<f64>());
    let Some(proj) = p.projection else {
        panic!(
            "at ν = {nu} the closed form lands, {:?}, not the edge",
            p.landing
        )
    };
    assert!(
        proj.nu == span[0] && proj.edge,
        "the nearest point past the lower edge is ν* = {}, edge {}, not the edge {} exactly",
        proj.nu,
        proj.edge,
        span[0]
    );
    assert_eq!(
        p.landing,
        CurveLanding::Latent,
        "a point past the span lands on the chart"
    );
}

#[test]
fn qa_encode_curve_past_span() {
    check_past_span(0.01);
    check_past_span(0.02);
}

negative_control!(
    qa_encode_curve_past_span,
    "a state inside the span is not projected to its edge",
    expected = "not the edge",
    check_past_span(0.2)
);

// ── REQ-ENC-022: the fold ──────────────────────────────────────────────────────────────────────────────────────────

/// On the folded `Φ_θ,K`: the original at `ν` lands at its own `θ`, not past the fold; its leg-swapped twin (bodies 1
/// and 2 exchanged) is flagged `past_fold`, is not relabelled onto the chart, falls back to latent `z` satisfying T2,
/// and lands on the full-range Euclid plane at `ν′ = (1 − ν)/(1 + ν)`.
fn check_fold(nu: f64, build_twin: bool, twin: bool, rng: &mut SplitMix) {
    let g = Gauge::random(rng, 3.0, 2.0);
    let original = family(nu, 1.0);
    let x = g.act(&if build_twin {
        swapped(&original)
    } else {
        original
    });
    let folded: BurrauThetaK<f64> = burrau_theta_k::<f64>();
    let c = into::<f64, _>(&folded, &x);
    let curve = c.curve.expect("a curve chart reports its curve encode");
    if !twin {
        let theta = (1.0 - nu * nu).atan2(2.0 * nu);
        let want = (theta - 0.2) / (FRAC_PI_4 - 0.2);
        assert!(
            !curve.closed.past_fold,
            "at ν = {nu}: a fold shape is flagged past the fold"
        );
        match c.landing {
            Landing::Chart { u, .. } => assert!(
                (u - want).abs() <= eps_phys::<f64>(),
                "at ν = {nu}: lands at u = {u}, not θ's {want}"
            ),
            Landing::Latent => {
                panic!("at ν = {nu}: a fold shape does not land on the folded chart")
            }
        }
        return;
    }
    assert!(
        curve.closed.past_fold,
        "at ν = {nu}: the twin is not flagged past_fold"
    );
    assert!(
        c.landing == Landing::Latent && curve.landing == CurveLanding::Latent,
        "at ν = {nu}: the twin is relabelled onto the folded chart: {:?}",
        c.landing
    );
    check_t2::<f64>(&c.encoded, "the fold's twin");
    let euclid = euclid_plane::<f64>();
    let e = into::<f64, _>(&euclid, &x);
    let nu_twin = (1.0 - nu) / (1.0 + nu);
    let want = (nu_twin - 1.0 / 32.0) / (30.0 / 32.0);
    assert!(
        e.curve.expect("a curve chart").closed.past_fold,
        "the twin's flag does not reach the caller on the full-range chart"
    );
    match e.landing {
        Landing::Chart { v, .. } => assert!(
            (v - want).abs() <= eps_phys::<f64>(),
            "at ν = {nu}: the twin lands at v = {v} on the Euclid plane, not ν′'s {want}"
        ),
        Landing::Latent => {
            panic!("at ν = {nu}: the twin is not its own point on the full-range chart")
        }
    }
}

#[test]
fn qa_encode_burrau_fold() {
    let mut rng = SplitMix::new(29);
    // The fold's shapes inside the folded chart's θ range [0.2, π/4] (ν ∈ [√2 − 1, ν(0.2) ≈ 0.818]), away from the
    // isosceles end, where only a twin within the tolerance of the edge may be projected.
    for k in 0..32 {
        let nu = 0.45 + 0.36 * k as f64 / 31.0;
        check_fold(nu, false, false, &mut rng);
        check_fold(nu, true, true, &mut rng);
    }
    // The isosceles triangle, a = b, is not past the fold.
    let iso = std::f64::consts::SQRT_2 - 1.0;
    let cf = closed_form(&enc::<f64>(&triangle(1.0, 1.0)).canonical.ic);
    assert!(
        !cf.past_fold,
        "the isosceles triangle is flagged past the fold"
    );
    assert!(
        (cf.nu - iso).abs() <= eps_phys::<f64>(),
        "the isosceles ν is not √2 − 1"
    );
}

negative_control!(
    qa_encode_burrau_fold,
    "a fold shape checked as if it were the twin is not flagged past the fold",
    expected = "is not flagged past_fold",
    check_fold(0.6, false, true, &mut SplitMix::new(29))
);
