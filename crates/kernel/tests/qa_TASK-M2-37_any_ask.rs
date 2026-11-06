//! qa's tests for TASK-M2-37, fix round 5: the warped mass ratio's range over drawn slices and asks, past the seed
//! file's fixed grid (chart_reference §2.1: "Warped, `q(t) = q₀·(q₁/q₀)^t`, geometric, over `[q₀, q₁]`: the part of the
//! asked range the link realises at `m_c` and at its active parameters. A chart decoded at `f32` holds `m_c` narrowed to
//! `f32` [...] so the range is the part realised at `m_c` both as held and as narrowed, and where those share no ratio
//! [...] the chart is not offered"). Written from that paragraph, with the realised set scanned by an oracle of the
//! test's own; each test has its negative control (R-176).
//!
//! Drawn: mass controls uniform on `[−12, 12]²` (a third of them on `[−7, −4]²`, where a held mass lies near 1), every
//! ordered body pair, under `softmax_tanh` at `μ_max` drawn over its allowed range and under `stick_breaking`. Asks, for
//! each slice: a sliver about one scanned realised ratio, 0.002 wide in `ln q`; a range holding one end of the scanned
//! realised part; the ends `0` and `1e300`.
//!
//! - `qa_any_ask_offered`: an ask holding any ratio the oracle finds realised, at the held `m_c` and at it narrowed, is
//!   offered, and its range holds each such ratio and stays in the ask, whatever the ask's width and the interval's.
//!   Control: a build that gives up on an unrealised centre.
//! - `qa_any_ask_every_row`: every chart offered holds on each of 65 rows, at `f64` and narrowed to `f32`: the row
//!   validates, the decoded masses are positive, finite and sum to 1 at the chart's own precision, and the `f64` masses
//!   are §2.1's at the held `m_c` to within `ε_μ`, relative. Control: a range past the realised part's ends.
//! - `qa_any_ask_saturated_link`: the same every-row check on controls past the links' saturation, `±20`, `±40`, `±100`
//!   and `±1000`, where a held mass can round to 1 and its two small masses to 0: a zero mass is no system, §2.1's
//!   masses being positive. Control: a range past the realised part's ends.
//!
//! **Values.** `ε_μ` is the register's link clamp (§2.1, reused as the round trip's tolerance); `ε_μ/4` is this file's
//! margin for "well inside"; `0.02` in `ln q` and `±30` are the oracle's scan: step, and reach past any ratio a link at
//! its allowed parameters realises. Every slice, range and seed is test data. The sum at `f32` is held to `ε_μ`, the
//! link's own clamp.

use kernel::chart::energy::Mapping;
use kernel::chart::mixed::ZBeta;
use kernel::chart::physical::MixedMassRatio;
use kernel::chart::{Chart, ValidationResult};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Defaults, Link, LinkParams, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    StickBreaking,
};
use validation::{negative_control, tunables};

/// The selection with `stick_breaking` on the mass block.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

type Ratio<S> = MixedMassRatio<ZBeta, S, f64>;

const PAIRS: [[u32; 2]; 6] = [[0, 1], [1, 0], [0, 2], [2, 0], [1, 2], [2, 1]];
/// Slices drawn per link and parameter.
const DRAWS: u32 = 240;

fn eps_mu() -> f64 {
    <f64 as ChartConstants>::EPS_MU
}

/// A small generator of the test's own, so a failure names its draw.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn on(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

fn slice(z6: f64, z7: f64) -> [f64; 8] {
    [0.2, -0.3, 0.1, 0.15, -0.05, 0.25, z6, z7]
}

/// §2.1's masses for the ratio `q` at the held `m_c`.
fn masses(pair: [u32; 2], held: f64, q: f64) -> [f64; 3] {
    let m_a = (1.0 - held) / (1.0 + q);
    let mut m = [held; 3];
    m[pair[0] as usize] = m_a;
    m[pair[1] as usize] = q * m_a;
    m
}

/// §2.1's round trip at `q` and this held mass, each mass and the ratio to within `eps`, relative. Every mass must be
/// positive: a comparison, so a NaN fails it.
fn trip<S: Selection>(pair: [u32; 2], held: f64, q: f64, eps: f64, p: &LinkParams<f64>) -> bool {
    let m = masses(pair, held, q);
    let back =
        <S::MassLink as Link<2, 3>>::forward_at(<S::MassLink as Link<2, 3>>::inverse_at(m, p), p);
    let ratio = back[pair[1] as usize] / back[pair[0] as usize];
    m.iter().all(|&m| m > 0.0)
        && back.iter().zip(m).all(|(&x, m)| (x - m).abs() <= eps * m)
        && (ratio - q).abs() <= eps * q
}

/// The oracle: realised well inside `ε_μ` at the held mass and at it narrowed.
fn realised<S: Selection>(pair: [u32; 2], held: f64, q: f64, p: &LinkParams<f64>) -> bool {
    let eps = 0.25 * eps_mu();
    trip::<S>(pair, held, q, eps, p) && trip::<S>(pair, f64::from(held as f32), q, eps, p)
}

/// Every scanned ratio the oracle finds realised, in `ln q` order.
fn found<S: Selection>(pair: [u32; 2], held: f64, p: &LinkParams<f64>) -> Vec<f64> {
    (0..=3000)
        .map(|i| (-30.0 + 0.02 * f64::from(i)).exp())
        .filter(|&q| realised::<S>(pair, held, q, p))
        .collect()
}

/// How a check gets its chart.
#[derive(Clone, Copy)]
struct Build {
    /// Refused where the centre is not realised.
    give_up: bool,
    /// The range's ends scaled by these, drawn linearly.
    ends: Option<[f64; 2]>,
}

/// The chart's own build.
const CHART: Build = Build {
    give_up: false,
    ends: None,
};

fn build<S: Selection>(
    how: Build,
    z: [f64; 8],
    pair: [u32; 2],
    asked: [f64; 2],
    p: &LinkParams<f64>,
) -> Option<Ratio<S>> {
    let new = |range, mapping| Ratio::<S>::new(z, [-1.0, 1.0], range, pair, mapping, p);
    let chart = new(asked, Mapping::Warped)?;
    if how.give_up {
        let m = <S::MassLink as Link<2, 3>>::forward_at([z[6], z[7]], p);
        let centre = m[pair[1] as usize] / m[pair[0] as usize];
        if !trip::<S>(pair, chart.held, centre, 0.5 * eps_mu(), p) {
            return None;
        }
    }
    match how.ends {
        Some([a, b]) => new([a * chart.ratio[0], b * chart.ratio[1]], Mapping::Linear),
        None => Some(chart),
    }
}

/// The asks that hold some of `seen`, the scan's realised ratios, ascending.
fn holding(seen: &[f64], d: &mut Draw) -> Vec<[f64; 2]> {
    let (first, last) = (seen[0], seen[seen.len() - 1]);
    let some = seen[(d.next() * seen.len() as f64) as usize % seen.len()];
    vec![
        [some * (-0.001_f64).exp(), some * 0.001_f64.exp()],
        [first * d.on(1e-20, 0.5), first * 1.0001],
        [last * 0.9999, last * d.on(2.0, 1e20)],
        [0.0, some],
        [some, 1.0e300],
        [0.0, 1.0e300],
        [first * 1e-10, last * 1e10],
    ]
}

/// `f` at each drawn slice of link `S` with its held mass and realised scan.
fn drawn<S: Selection>(
    p: &LinkParams<f64>,
    seed: u64,
    mut f: impl FnMut([f64; 8], [u32; 2], f64, &[f64], &mut Draw),
) {
    let mut d = Draw(seed);
    for i in 0..DRAWS {
        let z = if i % 3 == 0 {
            slice(d.on(-7.0, -4.0), d.on(-7.0, -4.0))
        } else {
            slice(d.on(-12.0, 12.0), d.on(-12.0, 12.0))
        };
        let pair = PAIRS[i as usize % PAIRS.len()];
        let held = Ratio::<S>::new(z, [-1.0, 1.0], [1.0, 2.0], pair, Mapping::Linear, p)
            .expect("a linear chart")
            .held;
        let seen = found::<S>(pair, held, p);
        f(z, pair, held, &seen, &mut d);
    }
}

fn params(seed: u64) -> Vec<LinkParams<f64>> {
    let mu = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's allowed range");
    let mut d = Draw(seed);
    let mut drawn: Vec<f64> = (0..3).map(|_| d.on(mu.lo, mu.hi)).collect();
    drawn.extend([mu.lo, mu.hi]);
    drawn
        .into_iter()
        .map(|mu_max| LinkParams {
            mu_max,
            ..LinkParams::DEFAULT
        })
        .collect()
}

fn offered_arm<S: Selection>(how: Build, p: &LinkParams<f64>, seed: u64) -> usize {
    let mut tried = 0;
    drawn::<S>(p, seed, |z, pair, _, seen, d| {
        if seen.is_empty() {
            return;
        }
        for asked in holding(seen, d) {
            let inside = || seen.iter().filter(|&&q| asked[0] <= q && q <= asked[1]);
            if inside().next().is_none() {
                continue;
            }
            tried += 1;
            let at = format!(
                "μ_max {}, z ({}, {}), bodies {pair:?}, asked {asked:?}",
                p.mu_max, z[6], z[7]
            );
            let chart = build::<S>(how, z, pair, asked, p)
                .unwrap_or_else(|| panic!("{at}: refused, though the scan finds it realised"));
            let [q0, q1] = chart.ratio;
            assert!(
                asked[0] <= q0 && q1 <= asked[1],
                "{at}: [{q0}, {q1}] leaves the ask"
            );
            for &q in inside() {
                assert!(
                    q0 <= q && q <= q1,
                    "{at}: the realised q = {q} is outside [{q0}, {q1}]"
                );
            }
        }
    });
    tried
}

fn check_offered(how: Build) {
    let mut tried = offered_arm::<Stick>(how, &LinkParams::DEFAULT, 0x9E37_79B9_7F4A_7C15);
    for p in params(0x0123_4567_89AB_CDEF) {
        tried += offered_arm::<Defaults>(how, &p, 0xC2B2_AE3D_27D4_EB4F);
    }
    assert!(tried >= 1000, "only {tried} asks tried");
}

#[test]
fn qa_any_ask_offered() {
    check_offered(CHART);
}

negative_control!(
    qa_any_ask_offered,
    "a build that gives up on an unrealised centre refuses asks the link realises, so it must fail",
    expected = "refused, though the scan",
    check_offered(Build {
        give_up: true,
        ends: None
    })
);

/// Each row of `chart` at the link `S`: validated, at `f64` and narrowed, and its masses a system of the slice.
fn rows<S: Selection>(chart: &Ratio<S>, at: &str, p: &LinkParams<f64>) {
    let pair = chart.bodies;
    let narrowed = chart.with_params(|x| x as f32);
    let thin = LinkParams {
        mu_max: p.mu_max as f32,
        q_max: p.q_max as f32,
    };
    for j in 0..=64 {
        let v = f64::from(j) / 64.0;
        let q = chart.ratio_at::<f64>(v);
        let got = chart.decode_params::<S, f64>(0.5, v, p).ic.m;
        let want = masses(pair, chart.held, q);
        assert!(
            chart.validate_params(0.5, v, p) == ValidationResult::Valid
                && want.iter().all(|&m| m > 0.0)
                && got
                    .iter()
                    .zip(want)
                    .all(|(&g, w)| (g - w).abs() <= eps_mu() * w),
            "{at}: at f64 the row {v}, q = {q}, is no system of the slice: {got:?} against {want:?}"
        );
        assert!(
            narrowed.validate_params(0.5, v, p) == ValidationResult::Valid,
            "{at}: at f32 the row {v} does not validate"
        );
        let wide = narrowed.decode_params::<S, f32>(0.5, v as f32, &thin).ic.m;
        let sum: f64 = wide.iter().map(|&m| f64::from(m)).sum();
        assert!(
            wide.iter().all(|&m| m.is_finite() && m > 0.0) && (sum - 1.0).abs() <= eps_mu(),
            "{at}: at f32 the row {v} decodes the masses {wide:?}"
        );
    }
}

fn row_arm<S: Selection>(how: Build, p: &LinkParams<f64>, seed: u64) -> usize {
    let mut charts = 0;
    drawn::<S>(p, seed, |z, pair, _, seen, d| {
        if seen.is_empty() {
            return;
        }
        for asked in holding(seen, d) {
            let at = format!(
                "μ_max {}, z ({}, {}), bodies {pair:?}, asked {asked:?}",
                p.mu_max, z[6], z[7]
            );
            if let Some(chart) = build::<S>(how, z, pair, asked, p) {
                rows(&chart, &at, p);
                charts += 1;
            }
        }
    });
    charts
}

fn check_rows(how: Build) {
    let mut charts = row_arm::<Stick>(how, &LinkParams::DEFAULT, 0xDEAD_BEEF_0BAD_F00D);
    for p in params(0xFEED_FACE_CAFE_BEEF) {
        charts += row_arm::<Defaults>(how, &p, 0x0F1E_2D3C_4B5A_6978);
    }
    assert!(charts >= 1000, "only {charts} charts drawn");
}

#[test]
fn qa_any_ask_every_row() {
    check_rows(CHART);
}

negative_control!(
    qa_any_ask_every_row,
    "a range past the realised part's ends draws ratios the link does not realise, so it must fail",
    expected = "is no system of the slice",
    check_rows(Build {
        give_up: false,
        ends: Some([0.9, 1.1])
    })
);

/// Each chart's rows at the extreme controls.
fn saturated_arm<S: Selection>(how: Build, p: &LinkParams<f64>) -> usize {
    const EXTREME: [f64; 11] = [
        -1000.0, -100.0, -40.0, -20.0, -12.0, 0.0, 12.0, 20.0, 40.0, 100.0, 1000.0,
    ];
    let mut charts = 0;
    for z6 in EXTREME {
        for z7 in EXTREME {
            for pair in PAIRS {
                for asked in [
                    [0.0, 1.0e300],
                    [1.0e-3, 1.0e3],
                    [1.0e-30, 1.0],
                    [1.0, 1.0e30],
                ] {
                    let at = format!(
                        "μ_max {}, z ({z6}, {z7}), bodies {pair:?}, asked {asked:?}",
                        p.mu_max
                    );
                    if let Some(chart) = build::<S>(how, slice(z6, z7), pair, asked, p) {
                        rows(&chart, &at, p);
                        charts += 1;
                    }
                }
            }
        }
    }
    charts
}

fn check_saturated(how: Build) {
    let mut charts = saturated_arm::<Stick>(how, &LinkParams::DEFAULT);
    for p in params(0x1357_9BDF_0246_8ACE) {
        charts += saturated_arm::<Defaults>(how, &p);
    }
    assert!(charts > 0, "no chart drawn");
}

#[test]
fn qa_any_ask_saturated_link() {
    check_saturated(CHART);
}

negative_control!(
    qa_any_ask_saturated_link,
    "a range past the realised part's ends draws ratios the link does not realise, so it must fail",
    expected = "is no system of the slice",
    check_saturated(Build {
        give_up: false,
        ends: Some([0.9, 1.1])
    })
);
