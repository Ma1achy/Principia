//! Warps per axis and the pair maps (design `principia_chart_composition.md` § 4, § 5.1, § 12): `forward(inverse(x)) =
//! x` for each warp and each pair map, at f64 and f32; polar's period and its pole-crossing partner; the linear arm bit
//! for bit today's; a warp not offered on an energy-type axis refused. Every test is named `warps_and_pair_maps_…`, so
//! `cargo test -p kernel warps_and_pair_maps` runs them all, and each has its control.
//!
//! **The linear arm, bit for bit.** [`LINEAR_DIGESTS`] were recorded from the code before the warp word existed
//! (`PRINCIPIA_BLESS_WARP_DIGESTS=1` prints them): FNV-1a 64 over each probe's decoded state at f64 and f32, its
//! encode's chart coordinates, the `(z_μ1, α)` chart's uniform rows and a saved chart's words. The probe's `t` and the
//! ranges are not dyadic and `lo ≠ 0`, so a reordered linear expression, `lo·(1 − t) + hi·t`, moves a bit.

use kernel::chart::mixed::{Alpha, Beta, Mixed, PRhoX, ZMu1, ZQ1};
use kernel::chart::saved::Saved;
use kernel::chart::tilt::{TiltMap, TiltSpec};
use kernel::chart::warp::{AxisWarp, WarpKind};
use kernel::chart::{Chart, Constraint, ValidationResult};
use kernel::decode::Ic;
use kernel::encode::by_kind::{ByKind, Coord};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::stage::decode::UniformChart;
use validation::negative_control;

/// The frozen controls: unequal masses, an interior configuration, nonzero momenta.
const FROZEN: [f64; 8] = [0.3, -0.4, 0.2, -0.5, 0.7, 0.1, 0.12, -0.08];

/// The probe's chart points: none dyadic, two past the unit square.
const PROBE: [(f64, f64); 6] = [
    (0.137, 0.291),
    (0.333, 0.777),
    (0.61, 0.05),
    (0.9071, 0.4429),
    (-0.21, 0.5),
    (0.4, 1.3),
];

/// FNV-1a 64.
#[derive(Clone, Copy)]
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn word(&mut self, w: u64) {
        for b in w.to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn f64(&mut self, x: f64) {
        self.word(x.to_bits());
    }
    fn f32(&mut self, x: f32) {
        self.word(u64::from(x.to_bits()));
    }
}

fn ic64(h: &mut Fnv, ic: &Ic<f64>) {
    for i in 0..3 {
        h.f64(ic.m[i]);
        for k in 0..2 {
            h.f64(ic.r[i][k]);
            h.f64(ic.p[i][k]);
        }
    }
}

fn ic32(h: &mut Fnv, ic: &Ic<f32>) {
    for i in 0..3 {
        h.f32(ic.m[i]);
        for k in 0..2 {
            h.f32(ic.r[i][k]);
            h.f32(ic.p[i][k]);
        }
    }
}

fn coord(h: &mut Fnv, c: Coord<f64>) {
    match c {
        Coord::At(x) => h.f64(x),
        Coord::Annotation => h.word(1),
        Coord::Unplaced => h.word(2),
    }
}

fn validation(h: &mut Fnv, r: ValidationResult) {
    match r {
        ValidationResult::Valid => h.word(10),
        ValidationResult::Clamp { u, v, moved, .. } => {
            h.word(11);
            h.f64(u);
            h.f64(v);
            h.word(u64::from(moved[0]) | u64::from(moved[1]) << 1);
        }
        ValidationResult::Reject { failed } => {
            h.word(12);
            h.word(u64::from(failed == Constraint::AxisDomain));
        }
        _ => h.word(13),
    }
}

/// One mixed chart's digest over [`PROBE`]: the f64 and f32 decodes, the encode's chart coordinates of the f64 decode,
/// the validation and, saved untilted about its centre, its words.
fn digest<H, V>(
    chart: Mixed<H, V, f64>,
    narrow: Mixed<H, V, f32>,
    saved_from: Mixed<H, V, f64>,
) -> u64
where
    H: kernel::chart::mixed::Quantity,
    V: kernel::chart::mixed::Quantity,
{
    let mut h = Fnv::new();
    for &(u, v) in &PROBE {
        let d = Chart::decode::<Defaults, f64>(&chart, u, v);
        ic64(&mut h, &d.ic);
        let n = Chart::decode::<Defaults, f32>(&narrow, u as f32, v as f32);
        ic32(&mut h, &n.ic);
        let e = encode::<Defaults, f64>(&d.ic);
        for c in ByKind::coords::<Defaults, f64>(&chart, &e) {
            coord(&mut h, c);
        }
        validation(&mut h, chart.validate(u, v));
    }
    let tilt = TiltSpec {
        tau1: 0.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: 0,
        y: 1,
    };
    let saved =
        Saved::curved(&saved_from, tilt, [0.5, 0.5], [1.0, 1.0]).expect("a mixed chart saves");
    for w in saved.to_words() {
        h.f64(w);
    }
    h.0
}

fn narrow(x: [f64; 8]) -> [f32; 8] {
    x.map(|w| w as f32)
}

fn pair(x: [f64; 2]) -> [f32; 2] {
    x.map(|w| w as f32)
}

/// Both axes of `c` under the warp `kind` (the control's reordered linear one), or `c` itself at [`WarpKind::Linear`].
fn under<H, V, P>(c: Mixed<H, V, P>, kind: WarpKind) -> Mixed<H, V, P>
where
    H: kernel::chart::mixed::Quantity,
    V: kernel::chart::mixed::Quantity,
    P: Copy + Default,
    f64: From<P>,
{
    let w = AxisWarp {
        kind,
        gamma: P::default(),
    };
    match kind {
        WarpKind::Linear => c,
        _ => c.with_warps(w, w).expect("a warp on a finite range"),
    }
}

/// The probe's charts, a latent and a derived axis, two derived axes and a mass axis, each axis at the warp `kind`;
/// the saved words and the uniform are always the linear chart's, so only the arithmetic can move a digest.
fn linear_digests(kind: WarpKind) -> [u64; 4] {
    let (qb_h, qb_v) = ([-1.37, 1.01], [0.43, 2.61]);
    let (ap_h, ap_v) = ([0.31, 1.23], [-0.83, 0.91]);
    let (ma_h, ma_v) = ([-0.67, 0.59], [0.21, 1.37]);
    let ma32 = Mixed::<ZMu1, Alpha, f32>::new(narrow(FROZEN), pair(ma_h), pair(ma_v));
    let mut uniform = Fnv::new();
    for row in ma32.uniform() {
        uniform.f32(row.x);
        uniform.f32(row.y);
        uniform.f32(row.z);
        uniform.f32(row.w);
    }
    let back = Mixed::<ZMu1, Alpha, f32>::from_uniform(&ma32.uniform());
    assert!(back == ma32, "the uniform does not read back");
    let qb = Mixed::<ZQ1, Beta, f64>::new(FROZEN, qb_h, qb_v);
    let ap = Mixed::<Alpha, PRhoX, f64>::new(FROZEN, ap_h, ap_v);
    let ma = Mixed::<ZMu1, Alpha, f64>::new(FROZEN, ma_h, ma_v);
    [
        digest(
            under(qb, kind),
            under(Mixed::new(narrow(FROZEN), pair(qb_h), pair(qb_v)), kind),
            qb,
        ),
        digest(
            under(ap, kind),
            under(Mixed::new(narrow(FROZEN), pair(ap_h), pair(ap_v)), kind),
            ap,
        ),
        digest(
            under(ma, kind),
            under(Mixed::new(narrow(FROZEN), pair(ma_h), pair(ma_v)), kind),
            ma,
        ),
        uniform.0,
    ]
}

/// Recorded before the warp word existed (the module's note).
const LINEAR_DIGESTS: [u64; 4] = [
    0x1d72_88c9_9ed1_0651,
    0x240b_28cf_50ab_9a39,
    0x1a65_ca64_53a3_9334,
    0xd7d7_0628_8ed6_31e2,
];

fn check_linear_bitwise(got: [u64; 4]) {
    if std::env::var_os("PRINCIPIA_BLESS_WARP_DIGESTS").is_some() {
        println!("LINEAR_DIGESTS = {got:#018x?}");
        return;
    }
    assert_eq!(
        got, LINEAR_DIGESTS,
        "the linear warp is not bit for bit today's chart"
    );
}

#[test]
fn warps_and_pair_maps_linear_bitwise() {
    check_linear_bitwise(linear_digests(WarpKind::Linear));
}

negative_control!(
    warps_and_pair_maps_linear_bitwise,
    "the linear arm written lo·(1 − t) + hi·t, equal in exact arithmetic, not in bits",
    expected = "not bit for bit today's chart",
    check_linear_bitwise(linear_digests(WarpKind::ReorderedLinear))
);

// ── Round trips: the criteria, written before the first run ─────────────────────────────────────────────────────────
//
// Each case draws its parameters and a point, and measures `forward(inverse(x)) − x` in units of the precision's
// epsilon times the case's scale. The criteria come from the rounding each formula does, not from a run:
//
// - linear, `t = (x − lo)/w`, `x′ = lo + w·t`, `w = hi − lo`: two roundings in `t`, two in `x′`, each at most
//   `eps·M`, `M = max(|lo|, |hi|)`: **`|x′ − x| ≤ 4·eps·M`**.
// - power, `s = (x − lo)/w`, `t = s^(1/γ)`, `x′ = lo + w·t^γ`: `s` to `2 eps`, each `pow` an ulp, `1/γ`'s rounding
//   `eps·|ln s|·s` (at most `eps/e` absolute), the `γ`-th power multiplying `t`'s relative error by `γ`; in `x` at most
//   `eps·(2M + w·s·(3 + γ) + 0.37·w)`, `w ≤ 2M`: **`|x′ − x| ≤ (10 + 2γ)·eps·M`**.
// - log, `t = ln(x/lo)/ln(hi/lo)`, `x′ = lo·(hi/lo)^t`: `x/lo` to `eps`, the two `ln`s and the division each a relative
//   `eps` on `t·ln(hi/lo)`, which `pow` turns into a relative error of the result times `|ln(x/lo)| ≤ |ln(hi/lo)|`:
//   **`|x′ − x| ≤ (4 + 4·|ln(hi/lo)|)·eps·|x|`**.
// - polar, `(a, b) → (r, ϕ) → (u, v) → (r′, ϕ′) → (a′, b′)`: `r` to a relative `eps` and back through the radius's
//   affine map (`4 eps·R`, `R = max(|r₀|, |r₁|)`), `ϕ` to an absolute `eps·Φ`, `Φ = max(|ϕ₀|, |ϕ₁|) + 2π`, the turn
//   into `[ϕ₀, ϕ₀ + 2π)` and the angle's affine map back (`4 eps·Φ`), `sin` and `cos` an ulp, the centre added (`eps·C`,
//   `C = max |c|`): **`‖(a′, b′) − (a, b)‖∞ ≤ 8·eps·(C + R·(1 + Φ))`**.
// - log-polar: polar's, with `ln(r/r₀)` and `r₀·e^τ` adding a relative `eps·(2 + |τ|)` to `r`, `T = max(|τ₀|, |τ₁|)`:
//   **`‖(a′, b′) − (a, b)‖∞ ≤ 8·eps·(C + R·(2 + Φ + T))`**, `R = r₀·e^{max τ}`.
// - affine: `(a, b)` itself, **bit for bit**.
// - polar's period and partner, `M(u, v + 2π/Δϕ)` and `M(partner(u, v))` against `M(u, v)`: the shifted `v` and the
//   partner's `−r` each a rounding at the shifted angle's scale, `sin` and `cos` taken at up to `Φ + π`:
//   **`‖Δ‖∞ ≤ 8·eps·(C + R·(2 + Φ + 2π))`**.
//
// The sample: [`DRAWS`] draws per case per precision, SplitMix64 from fixed seeds; the measured worst of each is
// printed (`--nocapture`) and reported on the pull request.

use kernel::chart::energy::{offered, Quantity as EnergyQuantity, Refused};
use kernel::chart::paired::{Affine, LogPolar, PairExtension, PairMap, Paired, Polar, Pole};
use kernel::chart::Float;
use kernel::real::Real;
use spirv_std::num_traits::FloatConst;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::uniform;

/// Draws per case per precision.
const DRAWS: usize = 100_000;

/// The precision a case runs at.
trait Width: Real + Float + From<f32> + Into<f64> + core::fmt::Debug {
    const EPS: f64;
    fn of(x: f64) -> Self;
}

impl Width for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl Width for f64 {
    const EPS: f64 = f64::EPSILON;
    fn of(x: f64) -> f64 {
        x
    }
}

/// A named mutation of one inverse (§ 12: each fires on its own row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mutation {
    None,
    /// Linear: `(x − lo)/hi`, the width dropped.
    LinearOverHi,
    /// Power: `s^γ` for `s^(1/γ)`.
    PowerExponent,
    /// Log: `ln(x)/ln(hi/lo)`, `lo` dropped.
    LogDropsLo,
    /// Polar: `atan2(x, y)` for `atan2(y, x)`.
    PolarSwapped,
    /// Log-polar: `ln(r)` for `ln(r/r₀)`.
    LogPolarDropsR0,
    /// Affine: `(b, a)` for `(a, b)`.
    AffineSwapped,
}

/// The warp `w`'s inverse at `x` on `[lo, hi]`, the shipped one or its named mutation.
fn warp_inverse<R: Width>(w: AxisWarp<R>, range: [R; 2], x: R, m: Mutation) -> R {
    let [lo, hi] = range;
    match (w.kind, m) {
        (WarpKind::Linear, Mutation::LinearOverHi) => (x - lo) / hi,
        (WarpKind::Power, Mutation::PowerExponent) => ((x - lo) / (hi - lo)).power(w.gamma),
        (WarpKind::Log, Mutation::LogDropsLo) => x.logarithm() / (hi / lo).logarithm(),
        _ => w.position(range, x),
    }
}

/// The worst of one warp's round trip at `R` over [`DRAWS`], in units of its criterion; panics past 1.
fn warp_case<R: Width>(kind: WarpKind, seed: u64, m: Mutation) -> f64
where
    f64: From<R>,
{
    let mut rng = SplitMix::new(seed);
    let mut worst = 0.0f64;
    let mut worst_eps = 0.0f64;
    for _ in 0..DRAWS {
        let (lo, hi, gamma) = match kind {
            WarpKind::Log => {
                let lo = 10f64.powf(uniform(&mut rng, -3.0, 3.0));
                let mut hi = 10f64.powf(uniform(&mut rng, -3.0, 3.0));
                if (hi / lo).ln().abs() < 0.1 {
                    hi = lo * 2.0;
                }
                (lo, hi, 0.0)
            }
            _ => {
                let lo = uniform(&mut rng, -10.0, 10.0);
                let mut hi = uniform(&mut rng, -10.0, 10.0);
                if (hi - lo).abs() < 0.1 {
                    hi = lo + 1.0;
                }
                let mut g = uniform(&mut rng, 0.2, 5.0);
                if (g - 1.0).abs() < 0.05 {
                    g = 1.5;
                }
                (lo, hi, g)
            }
        };
        let range = [R::of(lo), R::of(hi)];
        let warp = match kind {
            WarpKind::Power => AxisWarp::power(R::of(gamma)).expect("γ > 0"),
            WarpKind::Log => AxisWarp::<R>::log(),
            _ => AxisWarp::<R>::linear(),
        };
        let s = R::of(uniform(&mut rng, 0.0, 1.0));
        // Off the forward's own grid: `lo + (hi − lo)·s` is a value the linear round trip returns exactly at f64.
        let x = range[0] * (R::one() - s) + range[1] * s;
        let back = warp.value(range, warp_inverse(warp, range, x, m));
        let e: f64 = (back - x).into().abs();
        let (lo, hi, x64): (f64, f64, f64) = (range[0].into(), range[1].into(), x.into());
        let big = lo.abs().max(hi.abs());
        let g: f64 = warp.gamma.into();
        let (bound, scale) = match kind {
            WarpKind::Power => ((10.0 + 2.0 * g) * R::EPS * big, R::EPS * big),
            WarpKind::Log => (
                (4.0 + 4.0 * (hi / lo).ln().abs()) * R::EPS * x64.abs(),
                R::EPS * x64.abs(),
            ),
            _ => (4.0 * R::EPS * big, R::EPS * big),
        };
        let ratio = if e.is_nan() { f64::INFINITY } else { e / bound };
        worst = worst.max(ratio);
        worst_eps = worst_eps.max(if e.is_nan() { f64::INFINITY } else { e / scale });
    }
    let under = if m == Mutation::None {
        String::new()
    } else {
        format!(", inverse mutated {m:?}")
    };
    println!(
        "{kind:?} at {}{under}: worst {worst:.3} of the criterion ({worst_eps:.2} eps of its scale)",
        core::any::type_name::<R>()
    );
    worst
}

fn check_warps(mutated: Mutation) {
    let cases = [
        (WarpKind::Linear, Mutation::LinearOverHi),
        (WarpKind::Power, Mutation::PowerExponent),
        (WarpKind::Log, Mutation::LogDropsLo),
    ];
    for (i, (kind, named)) in cases.into_iter().enumerate() {
        let m = if named == mutated {
            named
        } else {
            Mutation::None
        };
        let seed = 0x5741_5250 + i as u64;
        for worst in [
            warp_case::<f64>(kind, seed, m),
            warp_case::<f32>(kind, seed, m),
        ] {
            assert!(
                worst <= 1.0,
                "{kind:?}'s round trip off: {worst} of its criterion"
            );
        }
        // Each named mutation fires on its own warp (the shipped inverse being checked above).
        if mutated == Mutation::None {
            assert!(
                warp_case::<f64>(kind, seed, named) > 1.0,
                "{named:?} does not fire on {kind:?}"
            );
        }
    }
}

#[test]
fn warps_and_pair_maps_warp_round_trip() {
    check_warps(Mutation::None);
}

negative_control!(
    warps_and_pair_maps_warp_round_trip,
    "power's inverse mutated: s^γ for s^(1/γ)",
    expected = "round trip off",
    check_warps(Mutation::PowerExponent)
);

/// A pair-map case: the map and the scale of its criterion (the criteria above), at `R`.
struct PairCase<R> {
    name: &'static str,
    forward: Box<dyn Fn(R, R) -> [R; 2]>,
    inverse: Box<dyn Fn(R, R) -> [R; 2]>,
    bound: f64,
}

fn polar_draw(rng: &mut SplitMix, pole: Pole) -> Polar<f64> {
    let centre = [uniform(rng, -2.0, 2.0), uniform(rng, -2.0, 2.0)];
    let radius = match pole {
        Pole::Bounded => [uniform(rng, 0.0, 1.0), uniform(rng, 1.0, 3.0)],
        Pole::Crossing => [uniform(rng, -3.0, -0.5), uniform(rng, 0.5, 3.0)],
    };
    let p0 = uniform(rng, -core::f64::consts::PI, core::f64::consts::PI);
    let angle = [p0, p0 + uniform(rng, 0.5, core::f64::consts::TAU)];
    Polar {
        centre,
        radius,
        angle,
        pole,
    }
}

fn log_polar_draw(rng: &mut SplitMix) -> LogPolar<f64> {
    let p0 = uniform(rng, -core::f64::consts::PI, core::f64::consts::PI);
    LogPolar {
        centre: [uniform(rng, -2.0, 2.0), uniform(rng, -2.0, 2.0)],
        r0: uniform(rng, 0.1, 10.0),
        log_radius: [uniform(rng, -3.0, 0.0), uniform(rng, 0.5, 3.0)],
        angle: [p0, p0 + uniform(rng, 0.5, core::f64::consts::TAU)],
    }
}

fn polar_at<R: Width>(p: &Polar<f64>) -> Polar<R> {
    Polar {
        centre: p.centre.map(R::of),
        radius: p.radius.map(R::of),
        angle: p.angle.map(R::of),
        pole: p.pole,
    }
}

fn log_polar_at<R: Width>(p: &LogPolar<f64>) -> LogPolar<R> {
    LogPolar {
        centre: p.centre.map(R::of),
        r0: R::of(p.r0),
        log_radius: p.log_radius.map(R::of),
        angle: p.angle.map(R::of),
    }
}

fn max_abs(x: [f64; 2]) -> f64 {
    x[0].abs().max(x[1].abs())
}

/// `ϕ` in `[ϕ₀, ϕ₀ + 2π)` from `(x, y)` with the arguments swapped: polar's named mutation.
fn swapped_inverse<R: Width>(p: &Polar<R>, a: R, b: R) -> [R; 2] {
    let (x, y) = (a - p.centre[0], b - p.centre[1]);
    let r = x.hypotenuse(y);
    let tau = R::of(core::f64::consts::TAU);
    let phi = x.arctan2(y);
    let phi = phi - ((phi - p.angle[0]) / tau).floor() * tau;
    [
        (r - p.radius[0]) / (p.radius[1] - p.radius[0]),
        (phi - p.angle[0]) / (p.angle[1] - p.angle[0]),
    ]
}

/// A case of each map at `R`, drawn from `rng`, its inverse mutated where `m` names it.
fn pair_cases<R: Width + FloatConst>(rng: &mut SplitMix, m: Mutation) -> Vec<PairCase<R>>
where
    f64: From<R>,
{
    let eps = R::EPS;
    let mut out = Vec::new();
    for pole in [Pole::Bounded, Pole::Crossing] {
        let d = polar_draw(rng, pole);
        let p = polar_at::<R>(&d);
        let big_r = max_abs(d.radius);
        let phi = max_abs(d.angle) + core::f64::consts::TAU;
        out.push(PairCase {
            name: if pole == Pole::Bounded {
                "polar, bounded"
            } else {
                "polar, pole-crossing"
            },
            forward: Box::new(move |u, v| p.forward(u, v)),
            inverse: Box::new(move |a, b| {
                if m == Mutation::PolarSwapped {
                    swapped_inverse(&p, a, b)
                } else {
                    p.inverse(a, b)
                }
            }),
            bound: 8.0 * eps * (max_abs(d.centre) + big_r * (1.0 + phi)),
        });
    }
    let d = log_polar_draw(rng);
    let l = log_polar_at::<R>(&d);
    let big_r = d.r0 * d.log_radius[1].exp();
    let phi = max_abs(d.angle) + core::f64::consts::TAU;
    let t = max_abs(d.log_radius);
    out.push(PairCase {
        name: "log-polar",
        forward: Box::new(move |u, v| l.forward(u, v)),
        inverse: Box::new(move |a, b| {
            let [u, v] = l.inverse(a, b);
            if m == Mutation::LogPolarDropsR0 {
                let r = (a - l.centre[0]).hypotenuse(b - l.centre[1]);
                let tau = r.logarithm();
                [
                    (tau - l.log_radius[0]) / (l.log_radius[1] - l.log_radius[0]),
                    v,
                ]
            } else {
                [u, v]
            }
        }),
        bound: 8.0 * eps * (max_abs(d.centre) + big_r * (2.0 + phi + t)),
    });
    let affine = Affine::<R>::default();
    out.push(PairCase {
        name: "affine",
        forward: Box::new(move |u, v| affine.forward(u, v)),
        inverse: Box::new(move |a, b| {
            if m == Mutation::AffineSwapped {
                [b, a]
            } else {
                affine.inverse(a, b)
            }
        }),
        bound: 0.0,
    });
    out
}

/// The worst round trip of each pair map at `R` over [`DRAWS`] parameter draws, one point each, in units of its
/// criterion, by name.
fn pair_worst<R: Width + FloatConst>(seed: u64, m: Mutation) -> Vec<(&'static str, f64)>
where
    f64: From<R>,
{
    let mut rng = SplitMix::new(seed);
    let mut worst: Vec<(&'static str, f64, f64)> = Vec::new();
    for _ in 0..DRAWS {
        let (u, v) = (uniform(&mut rng, 0.0, 1.0), uniform(&mut rng, 0.0, 1.0));
        for case in pair_cases::<R>(&mut rng, m) {
            let [a, b] = (case.forward)(R::of(u), R::of(v));
            let [iu, iv] = (case.inverse)(a, b);
            let [a2, b2] = (case.forward)(iu, iv);
            let e = f64::from(a2 - a).abs().max(f64::from(b2 - b).abs());
            let ratio = if e.is_nan() {
                f64::INFINITY
            } else if case.bound == 0.0 {
                if a2.to_f64_bits() == a.to_f64_bits() && b2.to_f64_bits() == b.to_f64_bits() {
                    0.0
                } else {
                    f64::INFINITY
                }
            } else {
                e / case.bound
            };
            match worst.iter_mut().find(|w| w.0 == case.name) {
                Some(w) => {
                    w.1 = w.1.max(ratio);
                    w.2 = w.2.max(e / R::EPS);
                }
                None => worst.push((case.name, ratio, e / R::EPS)),
            }
        }
    }
    let under = if m == Mutation::None {
        String::new()
    } else {
        format!(", inverses mutated {m:?}")
    };
    for (name, ratio, eps) in &worst {
        println!(
            "{name} at {}{under}: worst {ratio:.3} of the criterion ({eps:.2} eps absolute)",
            core::any::type_name::<R>()
        );
    }
    worst.into_iter().map(|w| (w.0, w.1)).collect()
}

/// A value's bits, widened, for the affine case's bitwise comparison.
trait Bits {
    fn to_f64_bits(self) -> u64;
}
impl<R: Into<f64>> Bits for R {
    fn to_f64_bits(self) -> u64 {
        self.into().to_bits()
    }
}

fn check_pairs(mutated: Mutation) {
    let named = [
        ("polar, bounded", Mutation::PolarSwapped),
        ("polar, pole-crossing", Mutation::PolarSwapped),
        ("log-polar", Mutation::LogPolarDropsR0),
        ("affine", Mutation::AffineSwapped),
    ];
    let seed = 0x5041_4952;
    for (name, worst) in pair_worst::<f64>(seed, mutated)
        .into_iter()
        .chain(pair_worst::<f32>(seed, mutated))
    {
        assert!(
            worst <= 1.0,
            "{name}'s round trip off: {worst} of its criterion"
        );
    }
    if mutated == Mutation::None {
        for m in [
            Mutation::PolarSwapped,
            Mutation::LogPolarDropsR0,
            Mutation::AffineSwapped,
        ] {
            let fired = pair_worst::<f64>(seed, m);
            for (name, n) in named {
                if n == m {
                    let w = fired
                        .iter()
                        .find(|w| w.0 == name)
                        .expect("each case runs")
                        .1;
                    assert!(w > 1.0, "{m:?} does not fire on {name}");
                }
            }
        }
    }
}

#[test]
fn warps_and_pair_maps_pair_round_trip() {
    check_pairs(Mutation::None);
}

negative_control!(
    warps_and_pair_maps_pair_round_trip,
    "polar's inverse mutated: atan2's arguments swapped",
    expected = "round trip off",
    check_pairs(Mutation::PolarSwapped)
);

/// Polar's period and its pole-crossing partner at f64 and f32: `M(u, v + period)` and `M(partner(u, v))` are
/// `M(u, v)` within the criterion above; the declared extensions. `partner` is the shipped one, or, for the control,
/// `(−r, ϕ)` with the turn by `π` dropped.
fn check_period_partner(drop_turn: bool) {
    let mut rng = SplitMix::new(0x504f_4c45);
    let mut worst = [0.0f64; 2];
    for _ in 0..DRAWS {
        let d = polar_draw(&mut rng, Pole::Crossing);
        assert_eq!(
            d.extensions(),
            [PairExtension::PoleCrossing, PairExtension::Periodic]
        );
        assert_eq!(
            polar_draw(&mut rng, Pole::Bounded).extensions(),
            [PairExtension::Bounded, PairExtension::Periodic]
        );
        let (u, v) = (uniform(&mut rng, 0.0, 1.0), uniform(&mut rng, 0.0, 1.0));
        let [pu, pv] = if drop_turn {
            [d.partner(u, v)[0], v]
        } else {
            d.partner(u, v)
        };
        let shifted = v + d.v_period();
        for (i, eps) in [f64::EPSILON, f32::EPSILON as f64].into_iter().enumerate() {
            let bound = 8.0
                * eps
                * (max_abs(d.centre)
                    + max_abs(d.radius) * (2.0 + max_abs(d.angle) + 2.0 * core::f64::consts::TAU));
            let e = |x: [f64; 2], y: [f64; 2]| (x[0] - y[0]).abs().max((x[1] - y[1]).abs());
            let (at, period, partner) = if i == 0 {
                (d.forward(u, v), d.forward(u, shifted), d.forward(pu, pv))
            } else {
                let p = polar_at::<f32>(&d);
                let w = |x: [f32; 2]| x.map(f64::from);
                (
                    w(p.forward(u as f32, v as f32)),
                    w(p.forward(u as f32, shifted as f32)),
                    w(p.forward(pu as f32, pv as f32)),
                )
            };
            worst[i] = worst[i]
                .max(e(period, at) / bound)
                .max(e(partner, at) / bound);
        }
    }
    println!(
        "polar's period and partner: worst {:.3} at f64, {:.3} at f32 of the criterion",
        worst[0], worst[1]
    );
    for w in worst {
        assert!(
            w <= 1.0,
            "polar's period or partner is not the same point: {w} of the criterion"
        );
    }
}

#[test]
fn warps_and_pair_maps_period_partner() {
    check_period_partner(false);
}

negative_control!(
    warps_and_pair_maps_period_partner,
    "the partner without its turn by π",
    expected = "not the same point",
    check_period_partner(true)
);

/// Power and log refused on every energy-type quantity by name, linear offered on every quantity with a warp written;
/// `accept` is the rule checked, the shipped [`offered`] or, for the control, one that reads only whether a warp is
/// written.
fn check_offered(accept: fn(EnergyQuantity, WarpKind) -> Result<(), Refused>) {
    for q in EnergyQuantity::ALL {
        for kind in [WarpKind::Linear, WarpKind::Power, WarpKind::Log] {
            let got = accept(q, kind);
            let want = if q == EnergyQuantity::PE {
                Err(Refused::NoWarpWritten(q))
            } else if kind != WarpKind::Linear && q.energy_type() {
                Err(Refused::WarpNotOffered(q, kind))
            } else {
                Ok(())
            };
            assert_eq!(
                got, want,
                "{kind:?} on {q:?}: accepted on an energy-type axis or refused wrongly"
            );
        }
    }
    // A warp's own domain: log on a range not positive, and power at γ ≤ 0, have no chart.
    let chart = Mixed::<ZQ1, Beta, f64>::new(FROZEN, [-1.0, 1.0], [0.4, 2.6]);
    assert!(chart
        .with_warps(AxisWarp::log(), AxisWarp::linear())
        .is_none());
    assert!(chart
        .with_warps(AxisWarp::linear(), AxisWarp::log())
        .is_some());
    assert!(AxisWarp::power(0.0f64).is_none() && AxisWarp::power(-1.0f64).is_none());
    assert!(AxisWarp::power(f64::NAN).is_none() && AxisWarp::power(2.0f64).is_some());
}

#[test]
fn warps_and_pair_maps_energy_refused() {
    check_offered(offered);
}

negative_control!(
    warps_and_pair_maps_energy_refused,
    "the rule reading only whether a warp is written: power accepted on E",
    expected = "accepted on an energy-type axis",
    check_offered(|q, _| kernel::chart::energy::written(q))
);

/// A pair map one ulp off the identity in `u`, the affine arm's control.
#[cfg(feature = "controls")]
#[derive(Clone, Copy)]
struct OneUlp;

#[cfg(feature = "controls")]
impl PairMap for OneUlp {
    type Param = f64;
    fn name(&self) -> &'static str {
        "one ulp"
    }
    fn extensions(&self) -> [PairExtension; 2] {
        [PairExtension::Affine; 2]
    }
    fn forward<R: Real + Float + From<f64>>(&self, u: R, v: R) -> [R; 2] {
        [u + u * R::epsilon(), v]
    }
    fn inverse<R: Real + Float + From<f64>>(&self, a: R, b: R) -> [R; 2] {
        [a, b]
    }
    fn area<R: Real + Float + From<f64>>(&self, _u: R, _v: R) -> R {
        R::one()
    }
    fn bound(&self, u: f64, v: f64) -> [f64; 2] {
        [u, v]
    }
}

/// `Paired<C, M>` decodes, validates and encodes as the chart itself, bit for bit, over [`PROBE`].
fn check_affine_bitwise<M: PairMap<Param = f64>>(pair: M) {
    let chart = Mixed::<Alpha, PRhoX, f64>::new(FROZEN, [0.31, 1.23], [-0.83, 0.91]);
    let paired = Paired::new(chart, pair);
    for &(u, v) in &PROBE {
        let (mut a, mut b) = (Fnv::new(), Fnv::new());
        ic64(&mut a, &Chart::decode::<Defaults, f64>(&chart, u, v).ic);
        ic64(&mut b, &Chart::decode::<Defaults, f64>(&paired, u, v).ic);
        validation(&mut a, chart.validate(u, v));
        validation(&mut b, paired.validate(u, v));
        let e = encode::<Defaults, f64>(&Chart::decode::<Defaults, f64>(&chart, u, v).ic);
        for (x, y) in ByKind::coords::<Defaults, f64>(&chart, &e)
            .into_iter()
            .zip(ByKind::coords::<Defaults, f64>(&paired, &e))
        {
            coord(&mut a, x);
            coord(&mut b, y);
        }
        assert_eq!(
            a.0, b.0,
            "the affine pair map is not the chart's own at ({u}, {v})"
        );
    }
}

#[test]
fn warps_and_pair_maps_affine_bitwise() {
    check_affine_bitwise(Affine::<f64>::default());
}

negative_control!(
    warps_and_pair_maps_affine_bitwise,
    "a pair map one ulp off the identity",
    expected = "not the chart's own",
    check_affine_bitwise(OneUlp)
);

/// A warped mixed chart end to end at f64: each probe's decode encodes back onto its point (`|Δu|, |Δv| ≤ 10⁻⁹`, the
/// encode tests' landing tolerance), past a power axis's `t = 0` and a log axis's `t = 1` validation clamps onto them,
/// and the warp words read back from the uniform and from a saved chart. `linear_encode` reads the encode's
/// coordinates through the linear inverse, the control's.
fn check_warped_chart(linear_encode: bool) {
    let chart = Mixed::<ZQ1, Beta, f64>::new(FROZEN, [-1.37, 1.01], [0.43, 2.61])
        .with_warps(AxisWarp::power(2.5).expect("γ > 0"), AxisWarp::log())
        .expect("log on a positive range");
    let read = if linear_encode {
        Mixed::<ZQ1, Beta, f64>::new(FROZEN, [-1.37, 1.01], [0.43, 2.61])
    } else {
        chart
    };
    for &(u, v) in PROBE.iter().take(4) {
        let x = Chart::decode::<Defaults, f64>(&chart, u, v).ic;
        let e = encode::<Defaults, f64>(&x);
        match ByKind::coords::<Defaults, f64>(&read, &e) {
            [Coord::At(eu), Coord::At(ev)] => assert!(
                (eu - u).abs() <= 1e-9 && (ev - v).abs() <= 1e-9,
                "the warped chart's encode lands at ({eu}, {ev}), not ({u}, {v})"
            ),
            c => panic!("the warped chart's encode does not place ({u}, {v}): {c:?}"),
        }
    }
    match chart.validate(-0.2, 1.3) {
        ValidationResult::Clamp { u, v, moved, .. } => {
            assert!(
                u == 0.0 && v == 1.0 && moved == [true, true],
                "clamped to ({u}, {v})"
            )
        }
        r => panic!("past the warps' bounds is not clamped: {r:?}"),
    }
    let narrow32 = Mixed::<ZMu1, Alpha, f32>::new(narrow(FROZEN), [-0.67, 0.59], [0.21, 1.37])
        .with_warps(AxisWarp::linear(), AxisWarp::power(0.7).expect("γ > 0"))
        .expect("finite ranges");
    assert!(
        Mixed::<ZMu1, Alpha, f32>::from_uniform(&narrow32.uniform()) == narrow32,
        "the warp words do not read back from the uniform"
    );
    let tilt = TiltSpec {
        tau1: 0.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: 0,
        y: 1,
    };
    let saved = Saved::curved(&chart, tilt, [0.5, 0.5], [1.0, 1.0]).expect("a mixed chart saves");
    let words = saved.to_words();
    let reread = Saved::from_words(&words).expect("the words read back");
    let parent = match reread {
        Saved::Curved(c) => c.parent::<Mixed<ZQ1, Beta, f64>>(),
        Saved::Oblique(_) | Saved::Chained(_) => None,
    };
    assert!(
        parent == Some(chart),
        "the warp words do not read back from a saved chart"
    );
}

#[test]
fn warps_and_pair_maps_warped_chart() {
    check_warped_chart(false);
}

negative_control!(
    warps_and_pair_maps_warped_chart,
    "the encode read through the linear inverse on a warped chart",
    expected = "the warped chart's encode lands at",
    check_warped_chart(true)
);
