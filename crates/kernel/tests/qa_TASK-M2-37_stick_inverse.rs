//! qa's tests for TASK-M2-37, fix round 8: `stick_breaking`'s inverse as dd_generation_root §3.9 now writes it, without
//! cancellation against 1, read from that paragraph and not from the tree; and the clamp test that goes with it
//! (`encode::blocks`, "The clamp tests": "the test is the clamp's, bit for bit"; inverse-encode Part 3, "the inverse is by
//! construction lossy and *says so*"). Each test has its negative control (R-176).
//!
//! - `qa_stick_inverse_is_the_table`: §3.9, "That is the table's inverse [...] cancelled by hand": over the simplex, its
//!   clamps included, the registry's inverse is `artanh(clamp(2v − 1, ±(1 − ε_μ)²)/(1 − ε_μ))`, `v = m₁ + m₂` and
//!   `v = m₂/(m₁ + m₂)`, to the table form's own rounding. Control: the table at twice `ε_μ`, which only the clamps tell.
//! - `qa_stick_inverse_corners`: §3.9, "at an exact corner [...] with the table's value", on the host, both floats.
//!   Control: the table's value with the other clamp's sign.
//! - `qa_stick_inverse_f32_trip`: §3.9, "`2v − 1` at a small `v` cancels against 1 the digits an `f32` mass rests on": at
//!   `f32`, `forward ∘ inverse` gives each mass back to `DECODE_TOL`, relative, down to twice the clamp. Control: the
//!   table's form at `f32`, whose loss is why the form changed.
//! - `qa_stick_flag_is_the_clamp`: at `f32` and `f64`, a walk through each of the four clamps: the bit `SimplexClamp`
//!   sets is what the inverse did. Control: the bit of the other control.
//! - `qa_stick_decode_dense`: the chart, on the grid of review 5785451098's finding: decoded at `f32`, every mass is §2.1's
//!   to `DECODE_TOL`. Control: the masses a row further on.
//!
//! **Values.** `ε_μ` is the register's; `DECODE_TOL` is parity_contract §4's relative IC/decode figure, as in
//! `qa_TASK-M2-37_narrowed.rs`, here also the width left out about each clamp, where "the clamp fired" cannot be read off
//! the output at `f32`. The walk's span, 5% about each clamp, and its 4,001 steps are test data, wider than `f32`'s spacing
//! at 1 (`6e-8`) over `ε_μ`. The table's rounding bound, 8 `ε_R`/`v`, is the conditioning of `artanh(2v − 1)`, about
//! `ε_R/4v`, with a margin of 32: no property of the tree under test.

use kernel::chart::energy::Mapping;
use kernel::chart::mixed::ZBeta;
use kernel::chart::physical::MixedMassRatio;
use kernel::chart::Chart;
use kernel::encode::blocks::SimplexClamp;
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Link, LinkParams, LinkReal, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use validation::negative_control;

/// The selection with `stick_breaking` on the mass block.
type Links =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

type Stick = MixedMassRatio<ZBeta, Links, f64>;

/// parity_contract §4, IC/decode, relative.
const DECODE_TOL: f64 = 1.0e-5;

fn eps_mu() -> f64 {
    <f64 as ChartConstants>::EPS_MU
}

/// The masses at the table's `s = m₁ + m₂` and `u = m₂/(m₁ + m₂)`, each limb without cancellation.
fn masses(s: f64, u: f64, one_minus_s: f64, one_minus_u: f64) -> [f64; 3] {
    [one_minus_s, s * one_minus_u, s * u]
}

/// The `(v, 1 − v)` the tests walk: both ends log-spaced from `lo` up to the middle.
fn grid(lo: f64, n: u32) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for k in 0..=n {
        let v = lo * (0.5 / lo).powf(f64::from(k) / f64::from(n));
        out.push((v, 1.0 - v));
        out.push((1.0 - v, v));
    }
    out
}

/// dd_generation_root §3.9's table: `artanh(clamp(2v − 1, ±c²)/c)`, `c = 1 − ε`, `2v − 1` written `v − w` with `w = 1 − v`
/// so the reference itself keeps a small `v`'s digits, and `artanh` written from its `log`.
fn table(v: f64, w: f64, eps: f64) -> f64 {
    let c = 1.0 - eps;
    let x = (v - w).clamp(-c * c, c * c) / c;
    0.5 * ((1.0 + x) / (1.0 - x)).ln()
}

fn check_table(eps: f64) {
    let p = grid(1.0e-9, 240);
    for &(s, one_minus_s) in &p {
        for &(u, one_minus_u) in &p {
            let m = masses(s, u, one_minus_s, one_minus_u);
            let z = <StickBreaking as Link<2, 3>>::inverse(m);
            let want = [table(s, one_minus_s, eps), table(u, one_minus_u, eps)];
            for k in 0..2 {
                let v = [s.min(one_minus_s), u.min(one_minus_u)][k].max(eps);
                let tol = 8.0 * f64::EPSILON / v;
                assert!(
                    (z[k] - want[k]).abs() <= tol,
                    "z[{k}] = {} is not the table's {} at s = {s:e}, u = {u:e}, off by {:e}, past {tol:e} (table)",
                    z[k],
                    want[k],
                    z[k] - want[k]
                );
            }
        }
    }
}

#[test]
fn qa_stick_inverse_is_the_table() {
    check_table(eps_mu());
}

negative_control!(
    qa_stick_inverse_is_the_table,
    "the table at twice the clamp differs from the registry's inverse only past its clamps, so the walk must reach them",
    expected = "(table)",
    check_table(2.0 * eps_mu())
);

/// A float the tests run at, with its way to and from `f64`.
trait Q: LinkReal + std::fmt::Debug {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn spacing() -> f64;
    fn zero_and_one() -> (Self, Self);
}

impl Q for f64 {
    fn of(x: f64) -> f64 {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn spacing() -> f64 {
        f64::EPSILON
    }
    fn zero_and_one() -> (f64, f64) {
        (0.0, 1.0)
    }
}

impl Q for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn spacing() -> f64 {
        f64::from(f32::EPSILON)
    }
    fn zero_and_one() -> (f32, f32) {
        (0.0, 1.0)
    }
}

/// The clamped control's size, `artanh(1 − ε)`, at `R`'s own `ε_μ`, in `f64`: at `f32`, `1 − ε_μ` itself rounds by a
/// hundredth of `ε_μ`, so that form cannot be the reference.
fn saturated<R: Q>() -> f64 {
    let e = R::EPS_MU.f();
    0.5 * ((2.0 - e) / e).ln()
}

fn check_corners<R: Q>(sign: f64) {
    let (zero, one) = R::zero_and_one();
    let sat = saturated::<R>();
    for (m, want) in [
        ([one, zero, zero], [-sat, -sign * sat]),
        ([zero, one, zero], [sat, -sat]),
        ([zero, zero, one], [sat, sat]),
    ] {
        let z = <StickBreaking as Link<2, 3>>::inverse(m);
        for k in 0..2 {
            let off = (z[k].f() - want[k]).abs() / sat;
            assert!(
                off <= 16.0 * R::spacing(),
                "at the corner {m:?} z[{k}] = {:?} is not the table's {:?} (corner)",
                z[k],
                want[k]
            );
        }
    }
}

#[test]
fn qa_stick_inverse_corners() {
    check_corners::<f64>(1.0);
    check_corners::<f32>(1.0);
}

negative_control!(
    qa_stick_inverse_corners,
    "0/0 at m₀ = 1 takes the clamp's low end, so the high end must fail there",
    expected = "(corner)",
    check_corners::<f64>(-1.0)
);

/// The relative miss of each mass through `inverse` and the forward at `f32`, over the simplex down to twice the clamp.
fn worst_trip(inverse: impl Fn([f32; 3]) -> [f32; 2]) -> (f64, [f32; 3]) {
    let p = grid(2.0 * eps_mu(), 160);
    let mut worst = (0.0_f64, [0.0_f32; 3]);
    for &(s, one_minus_s) in &p {
        for &(u, one_minus_u) in &p {
            let wide = masses(s, u, one_minus_s, one_minus_u);
            let m = wide.map(|x| x as f32);
            let back = <StickBreaking as Link<2, 3>>::forward(inverse(m));
            for k in 0..3 {
                let miss = ((f64::from(back[k]) - wide[k]) / wide[k]).abs();
                if miss.is_nan() || miss > worst.0 {
                    worst = (miss, m);
                }
            }
        }
    }
    worst
}

fn check_trip(inverse: impl Fn([f32; 3]) -> [f32; 2]) {
    let (miss, m) = worst_trip(inverse);
    assert!(
        miss <= DECODE_TOL,
        "a mass at {m:?} comes back off by {miss:e}, relative, past {DECODE_TOL:e} (trip)"
    );
}

/// The table's own form at `f32`, `2v − 1` and all.
fn table_f32(m: [f32; 3]) -> [f32; 2] {
    let c = 1.0 - <f32 as ChartConstants>::EPS_MU;
    let cap = c * c;
    let z = |v: f32| ((2.0 * v - 1.0).clamp(-cap, cap) / c).atanh();
    let stick = m[1] + m[2];
    [z(stick), z(m[2] / stick)]
}

#[test]
fn qa_stick_inverse_f32_trip() {
    check_trip(<StickBreaking as Link<2, 3>>::inverse);
}

negative_control!(
    qa_stick_inverse_f32_trip,
    "the table's own form at f32 cancels a small mass's digits against 1, so it must miss",
    expected = "(trip)",
    check_trip(table_f32)
);

/// Masses with control `k`'s `v` beside its clamp: `s` for control 0, `u` for control 1, from the small side, or, with
/// the limbs swapped, from the large; the other control mid-range.
fn beside<R: Q>(k: usize, large: bool, v: f64) -> [R; 3] {
    let (a, b) = if large { (1.0 - v, v) } else { (v, 1.0 - v) };
    let m = if k == 0 {
        masses(a, 0.5, b, 0.5)
    } else {
        masses(0.5, a, 0.5, b)
    };
    m.map(R::of)
}

/// A walk through each control's two clamps: each step's clamp bit against whether the inverse returned the clamped
/// value. The bit read is control `k`'s, or with `other` the other control's.
fn check_flag<R: Q>(other: bool) {
    let e = R::EPS_MU.f();
    let end = 0.5 * e * (2.0 - e);
    let sat = saturated::<R>();
    let (mut fired, mut short, mut wrong, mut first) = (0_u32, 0_u32, 0_u32, None);
    for k in 0..2_usize {
        for large in [false, true] {
            for step in -2000_i32..=2000 {
                let d = f64::from(step) * 2.5e-5;
                if d.abs() < DECODE_TOL {
                    continue;
                }
                let m = beside::<R>(k, large, end * (1.0 + d));
                let z = <StickBreaking as Link<2, 3>>::inverse(m)[k].f();
                let target = if large { sat } else { -sat };
                let clamped = (z - target).abs() <= 2.0 * R::spacing() * sat;
                let bit = 1_u32 << if other { 1 - k } else { k };
                let flagged = <StickBreaking as SimplexClamp>::clamps(m) & bit != 0;
                match clamped {
                    true => fired += 1,
                    false => short += 1,
                }
                if clamped != flagged {
                    wrong += 1;
                    first = first.or(Some((k, large, d, m, z)));
                }
            }
        }
    }
    assert!(
        fired > 1000 && short > 1000,
        "the walk stays on one side of the clamp: {fired} past it, {short} short of it"
    );
    assert_eq!(
        wrong,
        0,
        "the bit is not the inverse's clamp on {wrong} of {} steps, first (control, large side, step, m, z) {first:?} \
         (flag)",
        fired + short
    );
}

#[test]
fn qa_stick_flag_is_the_clamp() {
    check_flag::<f64>(false);
    check_flag::<f32>(false);
}

negative_control!(
    qa_stick_flag_is_the_clamp,
    "the other control's bit does not say this control clamped, so read in its place it must fail",
    expected = "(flag)",
    check_flag::<f64>(true)
);

/// The grid of review 5785451098: controls on `[−8, 8]` at quarters, every ordered pair, 2 asks, 33 rows, decoded at `f32`
/// on the chart narrowed to `f32`: each mass §2.1's, at the narrowed `m_c` and the row's ratio, to `DECODE_TOL`. Its two
/// worst slices then, `(−7.25, −0.25)` on bodies `(0, 2)` and `(−6, −6)` on `(1, 2)`, are on the grid. `shift` reads the
/// masses a row further on, so it must fail. Returns the charts decoded.
fn check_dense(shift: f64) -> u32 {
    let params = LinkParams::<f64>::DEFAULT;
    let thin = LinkParams {
        mu_max: params.mu_max as f32,
        q_max: params.q_max as f32,
    };
    let mut charts = 0;
    for i in -32..=32 {
        for j in -32..=32 {
            let frozen = [
                0.2,
                -0.3,
                0.1,
                0.15,
                -0.05,
                0.25,
                f64::from(i) * 0.25,
                f64::from(j) * 0.25,
            ];
            for pair in [[0, 1], [1, 0], [0, 2], [2, 0], [1, 2], [2, 1]] {
                for asked in [[0.5, 2.0], [1.0e-3, 1.0e3]] {
                    let Some(chart) =
                        Stick::new(frozen, [-1.0, 1.0], asked, pair, Mapping::Warped, &params)
                    else {
                        continue;
                    };
                    charts += 1;
                    let narrowed = chart.with_params(|x| x as f32);
                    let wide = narrowed.at_f64();
                    for row in 0..=32 {
                        let v = row as f32 / 32.0;
                        let got = narrowed.decode_params::<Links, f32>(0.5, v, &thin).ic.m;
                        let q = wide.ratio_at(f64::from(v) + shift);
                        let m_a = (1.0 - wide.held) / (1.0 + q);
                        let mut want = [wide.held; 3];
                        want[pair[0] as usize] = m_a;
                        want[pair[1] as usize] = q * m_a;
                        for k in 0..3 {
                            let miss = (f64::from(got[k]) - want[k]).abs() / want[k];
                            assert!(
                                miss <= DECODE_TOL,
                                "z ({i}/4, {j}/4), bodies {pair:?}, asked {asked:?}, row {v}: mass {k} is {} against \
                                 {}, off by {miss:e} (dense)",
                                got[k],
                                want[k]
                            );
                        }
                    }
                }
            }
        }
    }
    charts
}

#[test]
fn qa_stick_decode_dense() {
    let charts = check_dense(0.0);
    assert!(charts >= 10_000, "only {charts} charts");
}

negative_control!(
    qa_stick_decode_dense,
    "a row further on is another ratio, so held to it the decode must fail",
    expected = "(dense)",
    {
        let _ = check_dense(1.0 / 32.0);
    }
);
