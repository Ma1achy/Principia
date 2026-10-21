//! Kind 4, the Burrau family's encode (`kernel::encode::curve`; inverse_encode_contract Part 5 kind 4; dd_encode §3.1,
//! §3.4, §5 tests 5 and 7; R-24, R-27; TASK-M2-18), at f64 and at the f32 instantiation natively:
//! - `encode_burrau_on_curve`: dd_encode §5 test 5: the physical `(3, 4, 5)`, under a random gauge, encodes to the
//!   masses `(5, 4, 3)/12`, the right angle at body 0, `ν = sec θ − tan θ = ½` and the coupling `μ₁ = log(b/c)`,
//!   `μ₂ = log(a/c)`; decoded back on the Euclid plane it is the same triangle to `ε_phys` (REQ-ENC-011).
//! - `encode_curve_projection_*`: dd_encode §5 test 7: on the curve the argmin and the closed form agree on `ν`, and
//!   the closed form lands, the argmin bypassed; off it `d(ν*)` is reported, the latent fallback is taken beyond the
//!   tolerance and satisfies T2; within it the point is projected; the tolerance's edges; and the calibration's
//!   evidence, `fixtures/gates/curve_projection/` (REQ-ENC-017, REQ-ENC-026).
//! - `encode_curve_composition_*`: kind 4 composed with Part 6 step 4 (dd_encode §3.1, "The composition"): a state
//!   with `d(ν(θ))` and `d(ν*)` beyond the tolerance falls back to latent `z` though its decode at the closed form is
//!   within `ε_phys`; a state within the tolerance of `ν*` only lands at `ν*`, not at `ν(θ)`.
//! - `encode_curve_axis_ends`: a state projected to an end of a curve chart's span reads the axis's end, 0 or 1,
//!   exactly.
//! - `encode_projection_metric_bruteforce`: off-curve fixtures: `ν*` minimises the shape sphere's chordal ⊕ the mass
//!   simplex's Euclidean distance at unit weights, against a brute-force search under the test's own metric, built from
//!   the positions by `kernel::shape::shape`, not from the encode's Jacobi vectors (REQ-ENC-020). Its control, the
//!   same search under another weighting, is the one that shows the projection test can fail (PIT-3).
//! - `encode_burrau_fold`: on a folded chart an input at `θ > π/4`, the leg-swapped twin of a fold shape, is not
//!   relabelled: it falls back to latent `z`, flagged `past_fold`; on the full-range Euclid plane the twin is its own
//!   chart point; the flag reaches the caller in the curve encode (REQ-ENC-022, dd_encode §3.1).
//! - `encode_burrau_by_kind`: every Burrau chart's state, under a random gauge, lands back on its chart at its own
//!   point through `by_kind::encode_into_within` at the curve tolerance, `encode_into` being it at `tol = 0`; past the
//!   curve axis's edge the axis places nothing.
//! - `encode_curve_primitives`: each comparison and select at its boundary: the square's floor, the bounded axis's
//!   edges, the search's edges and its NaN start, the span test, the fold's isosceles tie, and the flat guards; and the
//!   search's golden ratio, correctly rounded at both widths.
//!
//! **Tolerances** (applied per R-369, test decisions): the round trips are held to `ε_phys` (REQ-ENC-024, proposed by
//! the `encode-eps-phys` gate, provisional until the M2 gate) in inverse-encode Part 4's `‖·‖_phys`
//! (`validation::roundtrip::phys`); the projection's tolerance is [`curve_tol`], `k·ε_R` at the proposal of
//! `fixtures/gates/curve_projection/measurement.toml` (REQ-ENC-026, provisional until the M2 gate); the closed form's
//! scalars to [`scalar_tol`], 16 `ε_R`.
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory.

use std::f64::consts::FRAC_PI_4;
use std::path::{Path, PathBuf};

use kernel::chart::burrau::{embed, isosceles_nu};
use kernel::chart::burrau_acute::{nu_of, theta_of};
use kernel::chart::Float;
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, ChartDecode, EuclidPlane,
};
use kernel::decode::{self as dec, Ic};
use kernel::encode::by_kind::{
    encode_into, encode_into_within, ByKind, ChartEncoded, Coord, Landing,
};
use kernel::encode::curve::{
    bounded, closed_form, curve_distance2, curve_point, distance2, floor, nearest, project, sq,
    state_point, ClosedForm, CurveChart, CurveLanding, Point,
};
use kernel::encode::{encode, Encoded};
use kernel::generated::links::Defaults;
use kernel::real::IeeeBits;
use kernel::shape::shape;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys, trip, widen, Precision};
use validation::{BURRAU_THETA_MAX, BURRAU_THETA_MIN, GAMMA_K, K_MAX};

/// The proposed `k` of the projection's tolerance `k·ε_R` (REQ-ENC-026), the fixture's `k_curve`.
const K_CURVE: f64 = 128.0;

/// The projection's tolerance at `R`: [`K_CURVE`]`·ε_R`.
fn curve_tol<R: Precision>() -> R {
    R::of(K_CURVE * R::EPS)
}

/// The closed form's scalars' tolerance at `R`: 16 `ε_R`.
fn scalar_tol<R: Precision>() -> f64 {
    16.0 * R::EPS
}

fn rng(seed: u64) -> SplitMix {
    SplitMix::new(seed)
}

/// `x` rounded to `R`.
fn narrow<R: Precision>(x: &Ic<f64>) -> Ic<R> {
    Ic {
        m: x.m.map(R::of),
        r: x.r.map(|v| v.map(R::of)),
        p: x.p.map(|v| v.map(R::of)),
    }
}

/// `E(x)` at `R`.
fn enc<R: Precision>(x: &Ic<f64>) -> Encoded<R> {
    encode::<Defaults, R>(&narrow::<R>(x))
}

/// `γ(ν)` at rest, at f64.
fn burrau(nu: f64) -> Ic<f64> {
    let s = embed(nu);
    Ic {
        m: s.m,
        r: s.r,
        p: [[0.0; 2]; 3],
    }
}

/// `x` under a random gauge.
fn gauged(x: &Ic<f64>, rng: &mut SplitMix) -> Ic<f64> {
    Gauge::random(rng, 3.0, 2.0).act(x)
}

/// The Euclid plane on its default range, at `R`.
fn euclid<R: Precision>() -> EuclidPlane<R> {
    EuclidPlane {
        m_range: [R::of(1.0), R::of(32.0)],
        nu_range: [R::of(1.0 / 32.0), R::of(31.0 / 32.0)],
    }
}

/// `Φ_θ,K` at the register's proposed `θ` range and warp (REQ-CHART-044, REQ-CHART-045).
fn theta_k<R: Precision>() -> BurrauThetaK<R> {
    BurrauThetaK {
        theta_range: [
            R::of(BURRAU_THETA_MIN.number()),
            R::of(BURRAU_THETA_MAX.number()),
        ],
        k_max: R::of(K_MAX.number()),
        gamma_k: R::of(GAMMA_K.number()),
    }
}

/// `‖·‖_phys` at `R`, widened, for `encode_into`.
fn norm<R: Precision>(a: &Ic<R>, b: &Ic<R>) -> R {
    R::of(phys(&widen(a), &widen(b)))
}

/// `x` encoded into `chart` at `R` under `‖·‖_phys`, `ε_phys` and the curve tolerance [`curve_tol`].
fn into<R: Precision, C: ByKind<Param = R>>(chart: &C, x: &Ic<f64>) -> ChartEncoded<R>
where
    f64: From<R>,
{
    encode_into_within::<Defaults, R, C, _>(
        chart,
        &narrow::<R>(x),
        norm::<R>,
        R::of(eps_phys::<R>()),
        curve_tol::<R>(),
    )
}

fn f<R: Precision>(x: R) -> f64 {
    x.into()
}

// ── Test 5: the physical (3, 4, 5) ─────────────────────────────────────────────────────────────────────────────────

/// The physical classical Burrau problem: the right angle at body 0, legs 3 and 4, each mass its opposite side,
/// `(5, 4, 3)`, over the total, at rest.
fn classical(m: [f64; 3]) -> Ic<f64> {
    Ic {
        m,
        r: [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]],
        p: [[0.0; 2]; 3],
    }
}

const TWELFTHS: [f64; 3] = [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0];

/// dd_encode §5 test 5 on the `(3, 4, 5)` triangle with masses `m`, under a random gauge at `R`: the canonical masses
/// are `m`, and the rest is the closed form's and the decode's.
fn check_classical<R: Precision>(m: [f64; 3], rng: &mut SplitMix)
where
    f64: From<R>,
{
    let x = gauged(&classical(m), rng);
    let e = enc::<R>(&x);
    let c = widen(&e.canonical.ic);
    let tol = scalar_tol::<R>();
    for i in 0..3 {
        assert!(
            (c.m[i] - m[i]).abs() <= tol,
            "the masses {:?} are not {m:?}",
            c.m
        );
    }
    let d = |p: [f64; 2], q: [f64; 2]| [p[0] - q[0], p[1] - q[1]];
    let (leg_a, leg_b) = (d(c.r[1], c.r[0]), d(c.r[2], c.r[0]));
    let cos = (leg_a[0] * leg_b[0] + leg_a[1] * leg_b[1])
        / (leg_a[0].hypot(leg_a[1]) * leg_b[0].hypot(leg_b[1]));
    assert!(
        cos.abs() <= tol,
        "the right angle is not recovered: cos = {cos:e}"
    );
    let cf = closed_form(&e.canonical.ic);
    let theta = f(cf.theta);
    assert!(
        (theta - 0.75f64.atan()).abs() <= tol,
        "θ = {theta} is not arctan(3/4)"
    );
    let secant = 1.0 / theta.cos() - theta.tan();
    assert!(
        (f(cf.nu) - secant).abs() <= tol && (f(cf.nu) - 0.5).abs() <= tol,
        "ν = {} is not sec θ − tan θ = {secant} = ½",
        f(cf.nu)
    );
    assert!(
        f(cf.coupling[0]).abs() <= tol && f(cf.coupling[1]).abs() <= tol,
        "the mass coupling μ₁ = log(b/c), μ₂ = log(a/c) fails by {:?}",
        cf.coupling.map(f)
    );
    assert!(!cf.past_fold, "(3, 4, 5) is inside the fold, θ < π/4");
    let back = into(&euclid::<R>(), &x);
    let residual = back.residual.map(f);
    assert!(
        matches!(back.landing, Landing::Chart { .. })
            && residual.is_some_and(|r| r <= eps_phys::<R>()),
        "decoded back, the triangle is not C(x) to ε_phys: residual {residual:?}"
    );
    let span = euclid::<R>().nu_span::<R>();
    let p = project(&e, span, curve_tol::<R>());
    assert!(
        matches!(p.landing, CurveLanding::OnCurve { .. }) && p.projection.is_none(),
        "the closed form is not on the curve: d = {:e}",
        f(p.d)
    );
}

#[test]
fn encode_burrau_on_curve() {
    let mut r = rng(5);
    for _ in 0..64 {
        check_classical::<f64>(TWELFTHS, &mut r);
        check_classical::<f32>(TWELFTHS, &mut r);
    }
}

negative_control!(
    encode_burrau_on_curve,
    "the (3, 4, 5) triangle with its legs' masses exchanged, (5, 3, 4)/12, breaks the coupling",
    expected = "the mass coupling",
    check_classical::<f64>([TWELFTHS[0], TWELFTHS[2], TWELFTHS[1]], &mut rng(5))
);

// ── Test 7: the projection ─────────────────────────────────────────────────────────────────────────────────────────

/// On the curve at `ν` under a random gauge at `R`: the closed form lands on the curve, the argmin bypassed; and the
/// argmin over the span, run anyway, agrees with `closed(ν)`, the closed form's `ν` as `closed` reads it, within
/// [`curve_tol`] in `ν`, at `d(ν*) ≤ tol`.
fn check_agrees<R: Precision>(nu: f64, rng: &mut SplitMix, closed: fn(&ClosedForm<R>) -> R)
where
    f64: From<R>,
{
    let e = enc::<R>(&gauged(&burrau(nu), rng));
    let span = euclid::<R>().nu_span::<R>();
    let tol = curve_tol::<R>();
    let p = project(&e, span, tol);
    assert!(
        matches!(p.landing, CurveLanding::OnCurve { .. }) && p.projection.is_none(),
        "ν = {nu}: the closed form is off the curve, d = {:e}",
        f(p.d)
    );
    let star = nearest(&state_point(&e.canonical.ic), span);
    let gap = (f(star.nu) - f(closed(&p.closed))).abs();
    assert!(
        gap <= f(tol) && star.d2 <= tol * tol,
        "ν = {nu}: the argmin {} and the closed form {} disagree by {gap:e}",
        f(star.nu),
        f(closed(&p.closed))
    );
}

#[test]
fn encode_curve_projection_agrees() {
    let mut r = rng(7);
    for _ in 0..512 {
        let nu = uniform(&mut r, 1.0 / 32.0, 31.0 / 32.0);
        check_agrees::<f64>(nu, &mut r, |c| c.nu);
        check_agrees::<f32>(nu, &mut r, |c| c.nu);
    }
}

negative_control!(
    encode_curve_projection_agrees,
    "a closed form of tan θ in place of sec θ − tan θ must disagree with the argmin",
    expected = "disagree",
    check_agrees::<f64>(0.3, &mut rng(7), |c| c.theta.tan())
);

/// Off the curve: `γ(ν)` with each position moved by up to `delta` at `R`. `d(ν*)` is reported, finite and the
/// projection's own; beyond the tolerance the landing is latent `z`, which decodes back to `C(x)` within `ε_phys`, and
/// whose round trip satisfies T2.
fn check_falls_back<R: Precision>(nu: f64, delta: f64, rng: &mut SplitMix, tol: R)
where
    f64: From<R>,
{
    let mut x = burrau(nu);
    for body in &mut x.r {
        body[0] += delta * uniform(rng, -1.0, 1.0);
        body[1] += delta * uniform(rng, -1.0, 1.0);
    }
    let e = enc::<R>(&gauged(&x, rng));
    let span = euclid::<R>().nu_span::<R>();
    let p = project(&e, span, tol);
    assert!(
        matches!(p.landing, CurveLanding::Latent),
        "ν = {nu}, δ = {delta}: d = {:e} beyond the tolerance {:e} must fall back to latent z",
        f(p.d),
        f(tol)
    );
    let star = p.projection.expect("off the curve, the argmin runs");
    assert!(
        p.d.is_finite() && p.d == star.distance() && p.d > tol,
        "d(ν*) is not reported: {:e}",
        f(p.d)
    );
    let back = dec::decode::<Defaults, R>(&e.z);
    let residual = phys(&widen(&e.canonical.ic), &widen(&back.ic));
    assert!(
        residual <= eps_phys::<R>(),
        "the latent fallback does not decode to C(x): {residual:e}"
    );
    let t2 = trip::<Defaults, R>(&back.ic).residual;
    assert!(
        t2 <= eps_phys::<R>(),
        "the latent fallback fails T2: {t2:e}"
    );
}

#[test]
fn encode_curve_projection_fallback() {
    let mut r = rng(17);
    for delta in [1e-2, 1e-1] {
        for _ in 0..128 {
            let nu = uniform(&mut r, 0.1, 0.9);
            check_falls_back::<f64>(nu, delta, &mut r, curve_tol());
            check_falls_back::<f32>(nu, delta, &mut r, curve_tol());
        }
    }
}

negative_control!(
    encode_curve_projection_fallback,
    "a tolerance of 1 admits every off-curve point, so the fallback is not taken",
    expected = "must fall back to latent z",
    check_falls_back::<f64>(0.4, 1e-2, &mut rng(17), 1.0)
);

/// The triangle of `γ(ν₀)` with the masses of `γ(ν₀ + ε)`: the closed form reads `ν₀` off the triangle, and the argmin
/// finds a nearer point, `d(ν*) < d(ν₀)` (the shape point moves with the masses, so `ν*` need not lie between the two). At a tolerance `share` of the way from `d(ν*)` to `d(ν₀)`
/// the point is projected to `ν*`, `d(ν*)` reported.
fn check_projects(nu: f64, eps: f64, share: f64) {
    let x = Ic {
        m: embed(nu + eps).m,
        ..burrau(nu)
    };
    let e = enc::<f64>(&x);
    let span = euclid::<f64>().nu_span::<f64>();
    let d0 = project(&e, span, 0.0).closed_d2.sqrt();
    let star = nearest(&state_point(&e.canonical.ic), span).distance();
    let tol = star + share * (d0 - star);
    let p = project(&e, span, tol);
    match p.landing {
        CurveLanding::Projected { nu: at } => assert!(
            p.d == star && star < d0 && (at - nu).abs() < 0.1,
            "projected to {at}, d = {:e}, not nearer than ν₀'s {d0:e}",
            p.d
        ),
        other => panic!("ν₀ = {nu}: at tolerance {tol:e}, not projected: {other:?}"),
    }
}

#[test]
fn encode_curve_projection_within() {
    for nu in [0.15, 0.3, 0.5, 0.7, 0.85] {
        check_projects(nu, 1e-3, 0.5);
    }
}

negative_control!(
    encode_curve_projection_within,
    "below d(ν*) the point is not projected",
    expected = "not projected",
    check_projects(0.5, 1e-3, -0.5)
);

/// The tolerance's edges, `d² ≤ tol²` both ways, at `R`: on the curve, the first draw whose closed-form `d` squares
/// back to its `d²` exactly (`IeeeBits::product`, the landing's form) lands on the curve at `tol = d` and not one float
/// below; off it (body 2 moved by `10⁻³`), the first such draw's `d(ν*)` is projected at `tol = d(ν*)` and falls back
/// one float below. `strict` replaces `≤` by `<` in the expectation at `tol = d`, for the control.
fn check_edges<R: Precision + Below>(seed: u64, strict: bool)
where
    f64: From<R>,
{
    let mut r = rng(seed);
    let span = euclid::<R>().nu_span::<R>();
    for on in [true, false] {
        let mut found = false;
        for _ in 0..4096 {
            let mut x = gauged(&burrau(uniform(&mut r, 0.1, 0.9)), &mut r);
            if !on {
                x.r[2][0] += 1e-3;
            }
            let e = enc::<R>(&x);
            let probe = project(&e, span, R::zero());
            let Some(star) = probe.projection else {
                continue;
            };
            if !on && probe.closed_d2 <= star.d2 + star.d2 {
                continue;
            }
            let d2 = if on { probe.closed_d2 } else { star.d2 };
            let d = Float::sqrt(d2);
            if d.product(d) != d2 || d2 == R::zero() {
                continue;
            }
            let (at, below) = (project(&e, span, d), project(&e, span, d.next_below()));
            if on {
                assert!(
                    matches!(at.landing, CurveLanding::OnCurve { .. }) != strict,
                    "at tol = d = {:e} the landing is {:?}",
                    f(d),
                    at.landing
                );
                assert!(
                    !matches!(below.landing, CurveLanding::OnCurve { .. }),
                    "below d the closed form lands: {:?}",
                    below.landing
                );
            } else {
                assert!(
                    matches!(at.landing, CurveLanding::Projected { .. }) != strict,
                    "at tol = d = {:e} the landing is {:?}",
                    f(d),
                    at.landing
                );
                assert!(
                    matches!(below.landing, CurveLanding::Latent),
                    "below d(ν*) the landing is {:?}",
                    below.landing
                );
            }
            found = true;
            break;
        }
        assert!(found, "no draw squares back exactly (on the curve: {on})");
    }
}

/// The float below `self`.
trait Below {
    fn next_below(self) -> Self;
}

impl Below for f64 {
    fn next_below(self) -> f64 {
        f64::from_bits(self.to_bits() - 1)
    }
}

impl Below for f32 {
    fn next_below(self) -> f32 {
        f32::from_bits(self.to_bits() - 1)
    }
}

#[test]
fn encode_curve_projection_edges() {
    check_edges::<f64>(23, false);
    check_edges::<f32>(23, false);
}

negative_control!(
    encode_curve_projection_edges,
    "a strict tolerance test would refuse d = tol",
    expected = "at tol = d",
    check_edges::<f64>(23, true)
);

// ── The calibration's evidence (REQ-ENC-026) ───────────────────────────────────────────────────────────────────────

/// The evidence directory.
const DIR: &str = "fixtures/gates/curve_projection";

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DIR)
}

/// The off-curve perturbations' sizes.
const DELTAS: [f64; 4] = [1e-6, 1e-4, 1e-2, 1e-1];

/// The near-tolerance perturbations' sizes, in the precision's tolerance `k·ε_R`.
const SCALES: [f64; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

/// The on-curve and off-curve draws.
const ON: usize = 16384;
const OFF: usize = 1024;

/// The seeds of the on-curve maxima's stability, and the draws at each.
const SEEDS: [u64; 8] = [27, 28, 29, 30, 31, 32, 33, 34];
const ON_SEED: usize = 4096;

/// The on-curve distribution's quantiles.
const QUANTILES: [(f64, &str); 5] = [
    (0.5, "q50"),
    (0.9, "q90"),
    (0.99, "q99"),
    (0.999, "q999"),
    (1.0, "max"),
];

/// The value at the quantile `q` of `sorted`, ascending: its `⌊(n − 1)·q⌋`th entry.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * q) as usize]
}

/// A curve chart's encode at `R` under the curve tolerance, as `into` does it, for the measurement's spans.
fn lands<R: Precision, C: CurveChart<Param = R>>(chart: &C, x: &Ic<f64>) -> bool
where
    f64: From<R>,
{
    matches!(into(chart, x).landing, Landing::Chart { .. })
}

/// One precision's measurement: `(key, value, what)`.
fn measure<R: Precision>(label: &str, out: &mut Vec<(String, f64, String)>)
where
    f64: From<R>,
{
    let mut r = rng(26);
    let tol = curve_tol::<R>();
    let spans = [
        ("euclid", euclid::<R>().nu_span::<R>()),
        ("fold", theta_k::<R>().nu_span::<R>()),
    ];
    for (name, span) in spans {
        let (lo, hi) = (f(span[0]), f(span[1]));
        let (mut closed, mut star, mut gap, mut accepted) =
            (Vec::with_capacity(ON), 0.0f64, 0.0f64, 0usize);
        for _ in 0..ON {
            let nu = uniform(&mut r, lo, hi);
            let x = gauged(&burrau(nu), &mut r);
            let e = enc::<R>(&x);
            let p = state_point(&e.canonical.ic);
            let cf = closed_form(&e.canonical.ic);
            let n = nearest(&p, span);
            closed.push(f(Float::sqrt(curve_distance2(&p, cf.nu))) / R::EPS);
            star = star.max(f(n.distance()) / R::EPS);
            gap = gap.max((f(n.nu) - f(cf.nu)).abs() / R::EPS);
            accepted += usize::from(if name == "euclid" {
                lands(&euclid::<R>(), &x)
            } else {
                lands(&theta_k::<R>(), &x)
            });
        }
        closed.sort_by(f64::total_cmp);
        for (q, qn) in QUANTILES {
            out.push((
                format!("on_{name}_{label}_d_closed_{qn}"),
                quantile(&closed, q),
                format!("d(ν(θ)) on the curve at the quantile {q}, {name} span, {label}, in ε_R"),
            ));
        }
        out.push((
            format!("on_{name}_{label}_d_star_max"),
            star,
            format!("largest d(ν*) on the curve, {name} span, {label}, in ε_R"),
        ));
        out.push((
            format!("on_{name}_{label}_nu_gap_max"),
            gap,
            format!("largest |ν* − ν(θ)| on the curve, {name} span, {label}, in ε_R"),
        ));
        out.push((
            format!("on_{name}_{label}_step4_frac"),
            accepted as f64 / ON as f64,
            format!("on the curve, {name} span, {label}: the share step 4 lands on the chart at tol and ε_phys"),
        ));
    }
    for seed in SEEDS {
        let mut r = rng(seed);
        for (name, span) in spans {
            let (lo, hi) = (f(span[0]), f(span[1]));
            let mut worst = 0.0f64;
            for _ in 0..ON_SEED {
                let e = enc::<R>(&gauged(&burrau(uniform(&mut r, lo, hi)), &mut r));
                let cf = closed_form(&e.canonical.ic);
                let p = state_point(&e.canonical.ic);
                worst = worst.max(f(Float::sqrt(curve_distance2(&p, cf.nu))) / R::EPS);
            }
            out.push((
                format!("on_{name}_{label}_d_closed_max_seed{seed}"),
                worst,
                format!("largest d(ν(θ)) on the curve, {name} span, {label}, seed {seed}, {ON_SEED} draws, in ε_R"),
            ));
        }
    }
    let chart = euclid::<R>();
    let span = chart.nu_span::<R>();
    for delta in DELTAS {
        let mut d = Vec::with_capacity(OFF);
        for _ in 0..OFF {
            let nu = uniform(&mut r, 0.1, 0.9);
            let mut x = burrau(nu);
            for body in &mut x.r {
                body[0] += delta * uniform(&mut r, -1.0, 1.0);
                body[1] += delta * uniform(&mut r, -1.0, 1.0);
            }
            let s = uniform(&mut r, -1.0, 1.0) * delta / 4.0;
            let t = uniform(&mut r, -1.0, 1.0) * delta / 4.0;
            x.m = [x.m[0] + s, x.m[1] - s / 2.0 + t, x.m[2] - s / 2.0 - t];
            let e = enc::<R>(&gauged(&x, &mut r));
            d.push(f(nearest(&state_point(&e.canonical.ic), span).distance()));
        }
        d.sort_by(f64::total_cmp);
        for (q, name) in [(0.0, "min"), (0.5, "median"), (1.0, "max")] {
            out.push((
                format!("off_{delta:e}_{label}_d_{name}"),
                quantile(&d, q),
                format!("off the curve, moved by up to δ = {delta:e}: the {name} d(ν*), {label}"),
            ));
        }
        let within = d.iter().filter(|&&v| v <= f(tol)).count();
        out.push((
            format!("off_{delta:e}_{label}_within_frac"),
            within as f64 / OFF as f64,
            format!("off the curve, moved by up to δ = {delta:e}: the share with d(ν*) ≤ tol, accepted by kind 4, {label}"),
        ));
    }
    for scale in SCALES {
        let (mut within, mut accepted, mut refused) = (0usize, 0usize, 0usize);
        for _ in 0..OFF {
            let x = gauged(
                &perturbed(uniform(&mut r, 0.1, 0.9), scale * f(tol), &mut r),
                &mut r,
            );
            let e = enc::<R>(&x);
            let p = project(&e, span, tol);
            if matches!(p.landing, CurveLanding::Latent) {
                refused +=
                    usize::from(residual_at(&e, chart.at_nu(p.closed.nu)) <= eps_phys::<R>());
            } else {
                within += 1;
                accepted += usize::from(lands(&chart, &x));
            }
        }
        let share = |n: usize| n as f64 / OFF as f64;
        out.push((
            format!("near_{scale:e}_{label}_within_frac"),
            share(within),
            format!("moved by up to {scale} tol: the share kind 4 accepts, d ≤ tol, {label}"),
        ));
        out.push((
            format!("near_{scale:e}_{label}_step4_frac"),
            share(accepted),
            format!("moved by up to {scale} tol: the share kind 4 accepts and step 4 lands on the chart, {label}"),
        ));
        out.push((
            format!("near_{scale:e}_{label}_refused_frac"),
            share(refused),
            format!("moved by up to {scale} tol: the share kind 4 refuses whose closed form step 4 alone would land, {label}"),
        ));
    }
}

/// The whole measurement, both precisions.
fn measurement() -> Vec<(String, f64, String)> {
    let mut out = vec![(
        "k_curve".to_string(),
        K_CURVE,
        "the proposed tolerance's k: d(ν*) ≤ k·ε_R lands (REQ-ENC-026, provisional until the M2 gate)".to_string(),
    )];
    measure::<f64>("f64", &mut out);
    measure::<f32>("f32", &mut out);
    out
}

fn text(values: &[(String, f64, String)]) -> String {
    let mut s = String::from(
        "# The curve projection's calibration evidence (REQ-ENC-026; R-71, R-24): d(ν*) under the R-24 metric, shape\n\
         # sphere chordal ⊕ mass simplex Euclidean at unit weights, on and off the Burrau curve, and the proposed\n\
         # tolerance k·ε_R, provisional until the M2 gate. Measured by crates/kernel/tests/encode_burrau.rs,\n\
         # encode_curve_projection_calibration; regenerate with PRIN_WRITE_CURVE_PROJECTION=1. On the curve: γ(ν) at\n\
         # rest under a random gauge, ν uniform on each span, 16384 draws at seed 26, d(ν(θ))'s quantiles in ε_R, and\n\
         # the share step 4 lands (dd_encode §3.1, the composition, at tol and ε_phys); its maximum at seeds 27 to 34,\n\
         # 4096 draws each. The test holds each on-curve value below k and at most twice its value here. Off it: γ(ν),\n\
         # ν uniform on [0.1, 0.9], each position moved by up to δ per component and the masses by up to δ/4 along\n\
         # (1, −½, −½) and up to δ/4 along (0, 1, −1), under a random gauge, 1024 draws per δ: the least, median and\n\
         # largest d(ν*), compared at a relative 1e-6 plus k·ε_R, and the share within tol. Near the tolerance: each\n\
         # position component and the masses along (1, 0, −1) and (0, 1, −1) moved by up to a multiple of tol, 1024\n\
         # draws per multiple: the shares kind 4 accepts, kind 4 and step 4 land, and kind 4 refuses where step 4 alone\n\
         # would land at the closed form. Every share is compared within 0.02.\n",
    );
    for (k, v, what) in values {
        s.push_str(&format!("{k} = {v:e}  # {what}\n"));
    }
    s
}

fn held(text: &str, k: &str) -> f64 {
    let line = text
        .lines()
        .find(|l| l.split('=').next().is_some_and(|key| key.trim() == k))
        .unwrap_or_else(|| panic!("{DIR}: no `{k}`"));
    line.split('=')
        .nth(1)
        .and_then(|r| r.split('#').next())
        .map(str::trim)
        .and_then(|r| r.parse().ok())
        .unwrap_or_else(|| panic!("{DIR}: `{k}` holds no number"))
}

/// The fixture `text` is the measurement `values`' evidence: the proposal is the tests' `k`; each share is the
/// fixture's within 0.02, a platform rounding a draw near an edge the other way; each on-curve value is below `k` and
/// at most twice the fixture's; each off-curve value is the fixture's within a relative 10⁻⁶ plus `k·ε_R` of its
/// precision, a platform's `sin`, `cos` and `atan2` rounding differently; and each off-curve minimum lies above `k·ε_R`,
/// so those draws fall back: at f64 from `δ = 10⁻⁶`, at f32 from `δ = 10⁻²`.
fn check_measurement(text: &str, values: &[(String, f64, String)]) {
    for (k, v, _) in values {
        let h = held(text, k);
        if k == "k_curve" {
            assert!(*v == h, "{DIR}: k_curve {h} is not the tests' {v}");
        } else if k.ends_with("_frac") {
            assert!((v - h).abs() <= 0.02, "{k} = {v}, the fixture holds {h}");
        } else if k.starts_with("on_") {
            assert!(*v < K_CURVE, "{k} = {v} is not below k = {K_CURVE}");
            assert!(*v <= 2.0 * h, "{k} = {v} is above twice the fixture's {h}");
        } else {
            let eps = if k.contains("_f32_") {
                f32::EPSILON as f64
            } else {
                f64::EPSILON
            };
            assert!(
                (v - h).abs() <= 1e-6 * h.abs() + K_CURVE * eps,
                "{k} = {v:e}, the fixture holds {h:e}"
            );
        }
    }
    for (label, eps, deltas) in [
        ("f64", f64::EPSILON, &DELTAS[..]),
        ("f32", f32::EPSILON as f64, &DELTAS[2..]),
    ] {
        for delta in deltas {
            let min = held(text, &format!("off_{delta:e}_{label}_d_min"));
            assert!(
                min > K_CURVE * eps,
                "δ = {delta:e}, {label}: d(ν*) = {min:e} does not fall back"
            );
        }
    }
}

#[test]
fn encode_curve_projection_calibration() {
    let values = measurement();
    let path = dir().join("measurement.toml");
    if std::env::var_os("PRIN_WRITE_CURVE_PROJECTION").is_some() {
        std::fs::create_dir_all(dir()).expect("the evidence directory");
        std::fs::write(&path, text(&values)).expect("the measurement");
    }
    let held =
        std::fs::read_to_string(&path).expect("fixtures/gates/curve_projection/measurement.toml");
    for (k, v, _) in &values {
        eprintln!("{k} = {v:e}");
    }
    check_measurement(&held, &values);
}

negative_control!(
    encode_curve_projection_calibration,
    "a fixture proposing another k is not the tests' evidence",
    expected = "is not the tests'",
    {
        let values = measurement();
        let held =
            std::fs::read_to_string(dir().join("measurement.toml")).expect("the measurement");
        check_measurement(
            &held.replace("k_curve = 1.28e2", "k_curve = 6.4e1"),
            &values,
        );
    }
);

// ── The metric against a brute-force search (REQ-ENC-020) ──────────────────────────────────────────────────────────

/// The test's own metric at mass weight `w`: the shape point of the positions by `kernel::shape::shape`, `w` as
/// `|w|`, its chordal distance squared, plus `w` times the masses' Euclidean distance squared.
fn own_d2(x: &Ic<f64>, nu: f64, w: f64) -> f64 {
    let y = burrau(nu);
    let (a, b) = (shape(x.r, x.m), shape(y.r, y.m));
    let (a, b) = ([a[0], a[1], a[2].abs()], [b[0], b[1], b[2].abs()]);
    let shape2: f64 = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum();
    let mass2: f64 = (0..3).map(|i| (x.m[i] - y.m[i]).powi(2)).sum();
    shape2 + w * mass2
}

/// The off-curve fixtures: `γ(ν₀)` with body 2 moved by `(dx, dy)` and the masses moved by `dm` along
/// `(1, −½, −½)`.
const FIXTURES: [(f64, f64, f64, f64); 8] = [
    (0.2, 0.05, -0.03, 0.02),
    (0.35, -0.1, 0.04, -0.03),
    (0.5, 0.02, 0.1, 0.05),
    (0.5, -0.2, -0.05, -0.04),
    (0.65, 0.07, 0.07, 0.03),
    (0.75, -0.03, 0.15, -0.02),
    (0.85, 0.1, -0.1, 0.04),
    (0.1, 0.0, 0.05, -0.05),
];

/// Two far states, `d² > 1`, whose `d²` has a second local minimum on the Euclid span (found by a random search, 2 in
/// 200 000): a search that leaves the best grid point's two cells can settle in the other basin.
const FAR: [Ic<f64>; 2] = [
    Ic {
        m: [0.6590993899962303, 0.260895262143094, 0.08000534786067577],
        r: [
            [0.0, 0.0],
            [-0.05432953293357334, 0.8274629309567318],
            [0.3088102163100235, -1.9664279320973064],
        ],
        p: [[0.0; 2]; 3],
    },
    Ic {
        m: [0.17094085574060783, 0.6699798483787551, 0.15907929588063704],
        r: [
            [0.0, 0.0],
            [-1.4689826507511716, 1.244462576487301],
            [1.4715565077088297, -0.9332744531024302],
        ],
        p: [[0.0; 2]; 3],
    },
];

/// The fixture `i`: [`FIXTURES`]' perturbations, then the [`FAR`] states.
fn fixture(i: usize) -> Ic<f64> {
    if i >= FIXTURES.len() {
        return FAR[i - FIXTURES.len()];
    }
    let (nu, dx, dy, dm) = FIXTURES[i];
    let mut x = burrau(nu);
    x.r[2][0] += dx;
    x.r[2][1] += dy;
    x.m = [x.m[0] + dm, x.m[1] - dm / 2.0, x.m[2] - dm / 2.0];
    x
}

/// Every fixture's index.
fn fixtures() -> std::ops::Range<usize> {
    0..FIXTURES.len() + FAR.len()
}

/// The fixture `i`'s `ν*`, under a random gauge, minimises the test's metric at weight `w` over the Euclid span,
/// against a brute-force search on 200 001 points: its value no more than the search's least plus `10⁻¹²`, and within
/// two of the search's steps of the search's point.
fn check_minimises(i: usize, w: f64, rng: &mut SplitMix) {
    let x = fixture(i);
    let e = enc::<f64>(&gauged(&x, rng));
    let c = e.canonical.ic;
    let [lo, hi] = euclid::<f64>().nu_span::<f64>();
    let star = nearest(&state_point(&c), [lo, hi]);
    let n = 200_000;
    let step = (hi - lo) / n as f64;
    let (brute, least) = (0..=n)
        .map(|k| lo + step * k as f64)
        .map(|nu| (nu, own_d2(&c, nu, w)))
        .fold((lo, f64::INFINITY), |b, t| if t.1 < b.1 { t } else { b });
    let at = own_d2(&c, star.nu, w);
    assert!(
        at <= least + 1e-12 && (star.nu - brute).abs() <= 2.0 * step,
        "fixture {i}: ν* = {} (d² {at:e}) does not minimise the metric: the search finds {brute} (d² {least:e})",
        star.nu
    );
    assert!(
        (star.d2 - own_d2(&c, star.nu, 1.0)).abs() <= 1e-14,
        "fixture {i}: the reported d² {:e} is not the metric's {:e}",
        star.d2,
        own_d2(&c, star.nu, 1.0)
    );
}

#[test]
fn encode_projection_metric_bruteforce() {
    let mut r = rng(20);
    for i in fixtures() {
        check_minimises(i, 1.0, &mut r);
    }
}

negative_control!(
    encode_projection_metric_bruteforce,
    "under a mass weight of 4 the unit-weight ν* is not the minimiser",
    expected = "does not minimise",
    {
        let mut r = rng(20);
        for i in fixtures() {
            check_minimises(i, 4.0, &mut r);
        }
    }
);

// ── The fold (REQ-ENC-022) ─────────────────────────────────────────────────────────────────────────────────────────

/// `x` with bodies 1 and 2 exchanged: the leg swap, a relabelling.
fn swapped(x: &Ic<f64>) -> Ic<f64> {
    Ic {
        m: [x.m[0], x.m[2], x.m[1]],
        r: [x.r[0], x.r[2], x.r[1]],
        p: [x.p[0], x.p[2], x.p[1]],
    }
}

/// `Φ_θ,K`'s state at `(u, v)` and its leg-swapped twin, at `R`, each under a random gauge: the state lands on the
/// chart, inside the fold; the twin, at `θ > π/4`, is flagged `past_fold`, is not relabelled onto the chart, and falls
/// back to latent `z`, by `encode_into_within` and by the projection both, the flag in the curve encode it reports; at
/// rest (`v = 0`) the twin is the full-range Euclid plane's own chart point, flagged too. When `twin` is false the twin
/// is replaced by the state itself, for the control.
fn check_fold<R: Precision>(u: f64, v: f64, rng: &mut SplitMix, twin: bool)
where
    f64: From<R>,
{
    let chart = theta_k::<f64>();
    let x = chart.decode_at::<Defaults, f64>(u, v).ic;
    let y = if twin { swapped(&x) } else { x };
    let own = into(&theta_k::<R>(), &gauged(&x, rng));
    assert!(
        matches!(own.landing, Landing::Chart { .. }),
        "the fold's own state does not land: {:?}",
        own.residual
    );
    let e = enc::<R>(&gauged(&y, rng));
    let cf = closed_form(&e.canonical.ic);
    assert!(
        cf.past_fold && f(cf.theta) > FRAC_PI_4,
        "θ = {} is not past the fold",
        f(cf.theta)
    );
    let t = into(&theta_k::<R>(), &gauged(&y, rng));
    assert!(
        matches!(t.landing, Landing::Latent) && t.coords[0] == Coord::Unplaced,
        "the twin lands on the folded chart: {:?}",
        t.landing
    );
    assert!(
        t.curve.is_some_and(|c| c.closed.past_fold),
        "the twin's curve encode does not carry past_fold to the caller"
    );
    let p = project(&e, theta_k::<R>().nu_span::<R>(), curve_tol::<R>());
    assert!(
        matches!(p.landing, CurveLanding::Latent),
        "the twin is projected onto the fold: {:?}",
        p.landing
    );
    if v == 0.0 {
        let full = into(&euclid::<R>(), &gauged(&y, rng));
        assert!(
            matches!(full.landing, Landing::Chart { .. })
                && full.curve.is_some_and(|c| c.closed.past_fold),
            "on the full range the twin is not its own chart point, flagged: {:?}",
            full.residual
        );
    }
}

#[test]
fn encode_burrau_fold() {
    let mut r = rng(22);
    for _ in 0..64 {
        let (u, v) = (uniform(&mut r, 0.05, 0.9), uniform(&mut r, 0.05, 0.95));
        for v in [0.0, v] {
            check_fold::<f64>(u, v, &mut r, true);
            check_fold::<f32>(u, v, &mut r, true);
        }
    }
}

negative_control!(
    encode_burrau_fold,
    "the fold's own state is not past the fold",
    expected = "is not past the fold",
    check_fold::<f64>(0.5, 0.0, &mut rng(22), false)
);

// ── Into every Burrau chart ────────────────────────────────────────────────────────────────────────────────────────

/// `from`'s state at random interior points, under a random gauge, lands on `into` at the same point within `1e-9`, at
/// f64.
fn sweep<C: ByKind<Param = f64>>(from: &C, onto: &C, seed: u64, name: &str, annotation: bool) {
    let mut r = rng(seed);
    for _ in 0..128 {
        let (u, v) = (uniform(&mut r, 0.05, 0.95), uniform(&mut r, 0.05, 0.95));
        let x = gauged(&from.decode_at::<Defaults, f64>(u, v).ic, &mut r);
        let e = into(onto, &x);
        match e.landing {
            Landing::Chart { u: lu, v: lv } => assert!(
                (annotation || (lu - u).abs() <= 1e-9) && (lv - v).abs() <= 1e-9,
                "{name}: lands at ({lu}, {lv}), not ({u}, {v})"
            ),
            Landing::Latent => panic!(
                "{name}: the state at ({u}, {v}) lands in latent z: {:?}",
                e.residual
            ),
        }
    }
}

fn nu_k() -> Burrau<f64> {
    Burrau {
        nu_range: [0.05, 0.9],
        k_max: 1.0,
        gamma_k: 2.0,
    }
}

fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: [0.2, FRAC_PI_4],
        k: 0.5,
        lz_fraction: 0.9,
    }
}

fn delta_m() -> BurrauThetaDeltaM<f64> {
    BurrauThetaDeltaM {
        theta_range: [0.2, FRAC_PI_4],
        m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
    }
}

#[test]
fn encode_burrau_by_kind() {
    sweep(&nu_k(), &nu_k(), 30, "(ν, K)", false);
    sweep(&euclid::<f64>(), &euclid::<f64>(), 31, "Euclid plane", true);
    sweep(&theta_k::<f64>(), &theta_k::<f64>(), 32, "Φ_θ,K", false);
    sweep(&theta_lz(), &theta_lz(), 33, "Φ_θ,L_z", false);
    sweep(&delta_m(), &delta_m(), 34, "δm strip", false);
    // Past the curve axis's edge the axis places nothing at the curve tolerance: the (ν, K) chart's ν = 0.95 > ν_max,
    // while ν = 0.85, inside [0.05, 0.9], is placed by the same call.
    let e = enc::<f64>(&burrau(0.95));
    let (c, _) = nu_k().coords_within::<Defaults, f64>(&e, curve_tol());
    assert!(c[0] == Coord::Unplaced, "ν past the span is placed: {c:?}");
    let e = enc::<f64>(&burrau(0.85));
    let (c, _) = nu_k().coords_within::<Defaults, f64>(&e, curve_tol());
    assert!(
        matches!(c[0], Coord::At(t) if (t - 0.8 / 0.85).abs() <= 1e-9),
        "ν inside the span is not placed: {c:?}"
    );
    // `encode_into` is `encode_into_within` at `tol = 0`.
    let x = gauged(&burrau(0.5), &mut rng(35));
    let eps = eps_phys::<f64>();
    assert_eq!(
        encode_into::<Defaults, f64, _, _>(&euclid::<f64>(), &x, norm::<f64>, eps),
        encode_into_within::<Defaults, f64, _, _>(&euclid::<f64>(), &x, norm::<f64>, eps, 0.0)
    );
}

negative_control!(
    encode_burrau_by_kind,
    "a (ν, K) state does not land on a chart of another K_max at its own point",
    expected = "not (",
    {
        let other = Burrau {
            k_max: 2.0,
            ..nu_k()
        };
        sweep(&nu_k(), &other, 30, "control", false);
    }
);

// ── The primitives at their boundaries ─────────────────────────────────────────────────────────────────────────────

/// `sq` keeps `floor()` and takes the float below it as 0; `floor()` is `2⁻³¹` at f32 and `2⁻²⁵⁵` at f64.
fn check_floor<R: Precision + Below>(expected: f64) {
    let t = floor::<R>();
    assert!(f(t) == expected, "floor {} is not {expected:e}", f(t));
    assert!(
        sq(t) == t * t && sq(t) > R::zero(),
        "sq(floor) = {} is not floor²",
        f(sq(t))
    );
    assert!(sq(-t) == t * t, "sq(−floor) is not floor²");
    assert!(
        sq(t.next_below()) == R::zero(),
        "sq below the floor is not 0"
    );
    assert!(sq(R::of(3.0)) == R::of(9.0), "sq(3) is not 9");
}

/// `M·2^-bits`, `M` an integer, is `(√5 − 1)/2` correctly rounded: `(M ∓ ½)·2^-bits` brackets the root of
/// `x² + x − 1`, in exact integer arithmetic, `x = m·2^-(bits + 1)` scaled by `2^(2·bits + 2)`.
fn rounds_golden(m: i128, bits: u32) -> bool {
    let p = |k: i128| k * k + k * (1i128 << (bits + 1)) - (1i128 << (2 * bits + 2));
    p(2 * m - 1) < 0 && p(2 * m + 1) > 0
}

#[test]
fn encode_curve_primitives() {
    check_floor::<f32>(2f64.powi(-31));
    check_floor::<f64>(2f64.powi(-255));

    // The golden ratio on [½, 1): the significand over 2^24 at f32 and over 2^53 at f64.
    let g32 = <f32 as IeeeBits>::INV_PHI;
    let g64 = <f64 as IeeeBits>::INV_PHI;
    assert!((0.5..1.0).contains(&g32) && (0.5..1.0).contains(&g64));
    assert!(rounds_golden(
        i128::from(g32.to_bits() & 0x7F_FFFF | 0x80_0000),
        24
    ));
    assert!(rounds_golden(
        i128::from(g64.to_bits() & 0xF_FFFF_FFFF_FFFF | 1 << 52),
        53
    ));
    assert!(!rounds_golden(
        i128::from(g64.to_bits() & 0xF_FFFF_FFFF_FFFF | 1 << 52) + 1,
        53
    ));

    // The bounded axis: [0, 1] placed, its edges included; one float past either edge not.
    assert_eq!(bounded(0.0f64), Coord::At(0.0));
    assert_eq!(bounded(1.0f64), Coord::At(1.0));
    assert_eq!(bounded(-f64::from_bits(1)), Coord::Unplaced);
    assert_eq!(bounded(1.0f64 + f64::EPSILON), Coord::Unplaced);
    assert_eq!(bounded(f64::NAN), Coord::Unplaced);

    // The metric: unit weights, each term once.
    let p = Point {
        n: [1.0, 0.0, 0.0],
        m: [0.5, 0.25, 0.25],
    };
    let q = Point {
        n: [0.0, 1.0, 0.0],
        m: [0.25, 0.5, 0.25],
    };
    assert_eq!(distance2(&p, &q), 2.0 + 0.125);
    assert_eq!(distance2(&p, &p), 0.0);

    // The search's edges: the curve's own edge points are found at the edge, flagged; an interior one is not.
    let span = [0.2f64, 0.6];
    for (nu, edge) in [(0.2, true), (0.6, true), (0.41, false)] {
        let s = nearest(&curve_point(nu), span);
        assert!(s.edge == edge, "ν = {nu}: edge {} is not {edge}", s.edge);
        assert!((s.nu - nu).abs() <= 1e-14, "ν = {nu}: found {}", s.nu);
    }
    // A span of one point: that point, an edge at both ends.
    let one = nearest(&curve_point(0.3), [0.5, 0.5]);
    assert!(one.nu == 0.5 && one.edge, "a one-point span: {one:?}");
    // Past the span the nearest point is its edge.
    assert_eq!(nearest(&curve_point(0.8), span).nu, 0.6);
    assert_eq!(nearest(&curve_point(0.1), span).nu, 0.2);
    // A NaN at the search's start (ν = 0, whose triangle has a zero leg) is not kept.
    assert!(curve_distance2(&curve_point(0.3), 0.0f64).is_nan());
    let s = nearest(&curve_point(0.3), [0.0, 0.5]);
    assert!(
        (s.nu - 0.3).abs() <= 1e-14 && !s.edge,
        "the NaN start is kept: {s:?}"
    );

    // The span test, both edges inclusive: a closed form at the span's edge lands on the curve; a span that starts one
    // float above it does not hold it, and the projection runs.
    let e = enc::<f64>(&burrau(0.5));
    let nu = closed_form(&e.canonical.ic).nu;
    let tol = curve_tol::<f64>();
    for span in [[nu, 0.9], [0.1, nu]] {
        let p = project(&e, span, tol);
        assert!(
            matches!(p.landing, CurveLanding::OnCurve { .. }),
            "{span:?}: {:?}",
            p.landing
        );
    }
    for span in [
        [f64::from_bits(nu.to_bits() + 1), 0.9],
        [0.1, nu.next_below()],
    ] {
        let p = project(&e, span, tol);
        assert!(
            p.projection.is_some(),
            "{span:?}: the closed form past the span lands on the curve"
        );
    }

    // The fold's tie: equal legs are not past it; a longer `a` is.
    let iso = Ic {
        m: [0.4, 0.3, 0.3],
        r: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        p: [[0.0; 2]; 3],
    };
    assert!(!closed_form(&iso).past_fold, "equal legs are past the fold");
    let long = Ic {
        r: [[0.0, 0.0], [1.0 + f64::EPSILON, 0.0], [0.0, 1.0]],
        ..iso
    };
    assert!(
        closed_form(&long).past_fold,
        "a longer leg a is not past the fold"
    );
    assert!(
        (closed_form(&iso).nu - isosceles_nu()).abs() <= 1e-15,
        "the isosceles closed form is not √2 − 1"
    );
    assert!((theta_of(nu_of(0.5f64)) - 0.5).abs() <= 1e-15);

    // Each curve chart's span is the ν its decode gives at the curve axis's ends, ordered, and `at_nu` inverts it.
    fn ends<C: CurveChart<Param = f64>>(c: &C, nu: [f64; 2], name: &str) {
        let span = c.nu_span::<f64>();
        assert!(
            (span[0] - nu[0]).abs() <= 1e-15 && (span[1] - nu[1]).abs() <= 1e-15,
            "{name}: the span {span:?} is not {nu:?}"
        );
        let t = [c.at_nu(span[0]), c.at_nu(span[1])];
        let mut sorted = t;
        sorted.sort_by(f64::total_cmp);
        assert!(
            sorted[0].abs() <= 1e-12 && (sorted[1] - 1.0).abs() <= 1e-12,
            "{name}: the span's ends are at {t:?}, not 0 and 1"
        );
    }
    let fold = |c: [f64; 2]| [nu_of(c[1]), nu_of(c[0])];
    ends(&nu_k(), [0.05, 0.9], "(ν, K)");
    ends(&euclid::<f64>(), [1.0 / 32.0, 31.0 / 32.0], "Euclid plane");
    ends(
        &theta_k::<f64>(),
        fold(theta_k::<f64>().theta_range),
        "Φ_θ,K",
    );
    ends(&theta_lz(), fold(theta_lz().theta_range), "Φ_θ,L_z");
    let mid = theta_lz().at_nu(nu_of(0.5f64));
    assert!(
        (mid - (0.5 - 0.2) / (FRAC_PI_4 - 0.2)).abs() <= 1e-12,
        "Φ_θ,L_z: θ = ½ is at {mid}"
    );
    let mid = theta_k::<f64>().at_nu(nu_of(0.5f64));
    let range = theta_k::<f64>().theta_range;
    assert!(
        (mid - (0.5 - range[0]) / (range[1] - range[0])).abs() <= 1e-12,
        "Φ_θ,K: θ = ½ is at {mid}"
    );

    // The flat guards: Φ_θ,L_z at K = 0, whose band has no width, reads v = ½; the δm strip whose target is the
    // closed form's own Burrau masses reads v = 0.
    let rest = enc::<f64>(&burrau(0.5));
    let flat = BurrauThetaLz {
        k: 0.0,
        ..theta_lz()
    };
    assert_eq!(flat.coords::<Defaults, f64>(&rest)[1], Coord::At(0.5));
    let own = BurrauThetaDeltaM {
        m_target: embed(closed_form(&rest.canonical.ic).nu).m,
        ..delta_m()
    };
    assert_eq!(own.coords::<Defaults, f64>(&rest)[1], Coord::At(0.0));
}

negative_control!(
    encode_curve_primitives,
    "the f32 floor is not the f64 one",
    expected = "is not",
    check_floor::<f32>(2f64.powi(-255))
);

// ── Kind 4 composed with step 4 (dd_encode §3.1, "The composition") ──────────────────────────────────────────────────

/// `γ(ν)` with each position component moved by up to `s` and the masses by up to `s` within the simplex, in two
/// random directions, (1, 0, −1) and (0, 1, −1) mixed.
fn perturbed(nu: f64, s: f64, rng: &mut SplitMix) -> Ic<f64> {
    let mut x = burrau(nu);
    for body in &mut x.r {
        body[0] += s * uniform(rng, -1.0, 1.0);
        body[1] += s * uniform(rng, -1.0, 1.0);
    }
    let (a, b) = (s * uniform(rng, -1.0, 1.0), s * uniform(rng, -1.0, 1.0));
    x.m = [x.m[0] + a, x.m[1] + b, x.m[2] - (a + b)];
    x
}

/// `D(Φ(c))` on the Euclid plane at the curve coordinate `t`, against `C(x)`, in `‖·‖_phys`.
fn residual_at<R: Precision>(e: &Encoded<R>, t: R) -> f64
where
    f64: From<R>,
{
    let back = euclid::<R>().decode_at::<Defaults, R>(R::zero(), t).ic;
    phys(&widen(&e.canonical.ic), &widen(&back))
}

/// Case (i), kind 4 (b) before step 4: the first seeded draw near the curve, at `R`, with `d(ν(θ)) > tol` and
/// `d(ν*) > tol`, `tol` the proposal's, whose decode at the closed form's `ν` is within `ε_phys` of `C(x)`. Encoded into
/// the Euclid plane at the tolerance `at`, it falls back to latent `z`: the curve axis places nothing, and the curve
/// encode, reported, is latent. The closed form's coordinate alone would have landed (the precondition).
fn check_beyond<R: Precision>(seed: u64, at: R)
where
    f64: From<R>,
{
    let mut r = rng(seed);
    let chart = euclid::<R>();
    let tol = curve_tol::<R>();
    for _ in 0..65536 {
        let x = perturbed(uniform(&mut r, 0.04, 0.2), 3.0 * f(tol), &mut r);
        let e = enc::<R>(&x);
        let p = project(&e, chart.nu_span::<R>(), tol);
        let cf = p.closed;
        if p.closed_d2 <= tol * tol || !matches!(p.landing, CurveLanding::Latent) {
            continue;
        }
        if residual_at(&e, chart.at_nu(cf.nu)) > eps_phys::<R>() {
            continue;
        }
        let w = encode_into_within::<Defaults, R, _, _>(
            &chart,
            &narrow::<R>(&x),
            norm::<R>,
            R::of(eps_phys::<R>()),
            at,
        );
        assert!(
            matches!(w.landing, Landing::Latent)
                && w.coords[1] == Coord::Unplaced
                && w.curve
                    .is_some_and(|c| matches!(c.landing, CurveLanding::Latent)),
            "d(ν(θ)) = {:e} and d(ν*) beyond tol = {:e}: lands on the chart, {:?} at {:?}",
            f(p.d),
            f(tol),
            w.landing,
            w.coords
        );
        return;
    }
    panic!("no draw beyond tol within ε_phys");
}

#[test]
fn encode_curve_composition_beyond() {
    check_beyond::<f64>(41, curve_tol());
    check_beyond::<f32>(41, curve_tol());
}

negative_control!(
    encode_curve_composition_beyond,
    "a tolerance of 1, the closed form's coordinate whatever d is, lands the state on the chart",
    expected = "lands on the chart",
    check_beyond::<f64>(41, 1.0)
);

/// Case (ii), kind 4 (b)'s `ν*`: the first seeded draw near the curve, at `R`, with `d(ν*) ≤ tol < d(ν(θ))`, `tol`
/// the proposal's, whose decode at `ν*` is within `ε_phys` of `C(x)`. Encoded into the Euclid plane, it lands on the
/// chart at `ν*`'s coordinate, which is not `ν(θ)`'s, and the curve encode, reported, is projected there. `closed`
/// expects `ν(θ)`'s coordinate instead, for the control.
fn check_at_star<R: Precision>(seed: u64, closed: bool)
where
    f64: From<R>,
{
    let mut r = rng(seed);
    let chart = euclid::<R>();
    let tol = curve_tol::<R>();
    for _ in 0..65536 {
        let x = perturbed(uniform(&mut r, 0.1, 0.9), f(tol), &mut r);
        let e = enc::<R>(&x);
        let p = project(&e, chart.nu_span::<R>(), tol);
        let CurveLanding::Projected { nu: star } = p.landing else {
            continue;
        };
        let (at_star, at_closed) = (chart.at_nu(star), chart.at_nu(p.closed.nu));
        if at_star == at_closed || residual_at(&e, at_star) > eps_phys::<R>() {
            continue;
        }
        let w = into(&chart, &x);
        let expected = if closed { at_closed } else { at_star };
        assert!(
            w.landing
                == Landing::Chart {
                    u: R::zero(),
                    v: expected
                }
                && w.curve.is_some_and(|c| c.landing == p.landing),
            "d(ν*) = {:e} ≤ tol < d(ν(θ)): lands at {:?}, not ν*'s {} (ν(θ)'s {})",
            f(p.d),
            w.landing,
            f(at_star),
            f(at_closed)
        );
        return;
    }
    panic!("no draw within tol of ν* only");
}

#[test]
fn encode_curve_composition_projected() {
    check_at_star::<f64>(43, false);
    check_at_star::<f32>(43, false);
}

negative_control!(
    encode_curve_composition_projected,
    "the closed form's coordinate is not where an off-curve state within tol lands",
    expected = "not ν*'s",
    check_at_star::<f64>(43, true)
);

/// A state on the curve just past each end of `chart`'s span, `8 ε_R` in `ν`, is projected to that end within
/// [`curve_tol`], and the curve axis, the coordinate `axis`, reads the end's own coordinate, 0 at `t = 0` and 1 at
/// `t = 1`, exactly, at `R`.
fn check_axis_ends<R: Precision, C: CurveChart<Param = R>>(chart: &C, axis: usize, name: &str)
where
    f64: From<R>,
{
    let tol = curve_tol::<R>();
    let span = chart.nu_span::<R>();
    for (t, end) in chart.nu_ends::<R>().into_iter().enumerate() {
        let out = if end == span[0] { -1.0 } else { 1.0 };
        let e = enc::<R>(&burrau(f(end) + out * 8.0 * R::EPS));
        let (c, curve) = chart.coords_within::<Defaults, R>(&e, tol);
        let curve = curve.expect("a curve chart reports its curve encode");
        assert!(
            curve.landing == CurveLanding::Projected { nu: end }
                && c[axis] == Coord::At(R::of(t as f64)),
            "{name}: past the end t = {t}, ν = {}, the axis reads {:?}, landing {:?}",
            f(end),
            c[axis],
            curve.landing
        );
    }
}

fn axis_ends<R: Precision>()
where
    f64: From<R>,
{
    let wide = |c: Burrau<f64>| Burrau {
        nu_range: c.nu_range.map(R::of),
        k_max: R::of(c.k_max),
        gamma_k: R::of(c.gamma_k),
    };
    // Φ_θ,L_z on [0.2, 0.7]: an end away from the isosceles, where at f32 the curve's d is flat to a float.
    let lz = |c: BurrauThetaLz<f64>| BurrauThetaLz {
        theta_range: [R::of(c.theta_range[0]), R::of(0.7)],
        k: R::of(c.k),
        lz_fraction: R::of(c.lz_fraction),
    };
    check_axis_ends(&wide(nu_k()), 0, "(ν, K)");
    check_axis_ends(&euclid::<R>(), 1, "Euclid plane");
    check_axis_ends(&theta_k::<R>(), 0, "Φ_θ,K");
    check_axis_ends(&lz(theta_lz()), 0, "Φ_θ,L_z");
}

#[test]
fn encode_curve_axis_ends() {
    axis_ends::<f64>();
    axis_ends::<f32>();
}

negative_control!(
    encode_curve_axis_ends,
    "past the span by more than tol allows, the state is not projected to the end",
    expected = "past the end",
    {
        let chart = theta_k::<f64>();
        let e = enc::<f64>(&burrau(chart.nu_span::<f64>()[1] + 1e-3));
        let (c, curve) = chart.coords_within::<Defaults, f64>(&e, curve_tol::<f64>());
        assert!(
            curve.is_some_and(|k| matches!(k.landing, CurveLanding::Projected { .. }))
                && c[0] == Coord::At(0.0),
            "past the end by 10⁻³ the axis reads {:?}",
            c[0]
        );
    }
);
