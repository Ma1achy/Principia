//! qa's tests for TASK-M2-37, fix round 4: the warped mass ratio's range when the slice centre's own ratio is not
//! realised (chart_reference §2.1: "Warped, `q(t) = q₀·(q₁/q₀)^t`, geometric, over `[q₀, q₁]`: the part of the asked range
//! the link realises at `m_c` and at its active parameters", "A ratio is realised when the link's inverse followed by
//! its forward returns its masses `(m_a, m_b, m_c)`, `m_c` at its held value, each to within `ε_μ`, relative"). Written
//! from that paragraph, the realised set scanned by an oracle of the test's own; each test has its negative control
//! (R-176).
//!
//! The slices reach the clamps: mass controls `{±10, ±4, ±1.5, 0}²` beside a fixed `z_β` slice, every ordered body
//! pair, under `softmax_tanh` at both ends of `μ_max`'s allowed range and at its default, and under `stick_breaking`,
//! so many centres are not realised.
//!
//! - `qa_seed_offers_a_realised_range`: an asked range holding a realised interval at least 0.2 wide in `ln q` has a
//!   chart. Control: a build that gives up on an unrealised centre. A narrower interval is not asserted: a chart over
//!   it is of no use, and REQ-CHART-070 lets a chart be refused.
//! - `qa_seed_range_is_the_realised_part`: a ratio of the asked range the oracle finds realised is inside the warped
//!   chart's `[q₀, q₁]`, and `[q₀, q₁]` lies inside the asked range. Control: a range cut short.
//! - `qa_seed_every_pixel`: each of 65 rows validates and decodes the three masses of `q(t)` at the held `m_c`, each to
//!   within `ε_μ`, relative. Control: a range past the realised part's ends.
//! - `qa_seed_asked_end_at_zero` and `qa_seed_asked_end_far_out`: moving an asked end out where the link realises
//!   nothing, to 0 (`energy_axes_mass_ratio_saturated_centre`'s own input) or to `1e300` (`new` takes every finite end),
//!   leaves the realised part as it was, so a chart over at least 0.2 in `ln q` stays offered, over the same range to
//!   within `1e-3`, relative. Control: a build that gives up on an unrealised centre.
//!
//! **Values.** `ε_μ` is the register's link clamp (§2.1, reused as the round trip's tolerance); `ε_μ/4` is this file's
//! margin for "well inside"; `0.05` in `ln q` is the oracle's scan step. Every slice and range is test data.

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

fn eps_mu() -> f64 {
    <f64 as ChartConstants>::EPS_MU
}

const PAIRS: [[u32; 2]; 6] = [[0, 1], [1, 0], [0, 2], [2, 0], [1, 2], [2, 1]];
const MASS: [f64; 7] = [-10.0, -4.0, -1.5, 0.0, 1.5, 4.0, 10.0];
const ASKED: [[f64; 2]; 5] = [
    [0.05, 20.0],
    [1e-4, 1e4],
    [1.0, 1000.0],
    [1e-6, 1e-2],
    [1e-3, 1e3],
];

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

/// Each of `got` within `eps`, relative, of `want`.
fn each_within(got: [f64; 3], want: [f64; 3], eps: f64) -> bool {
    got.iter().zip(want).all(|(&g, w)| (g - w).abs() <= eps * w)
}

/// The oracle: §2.1's round trip at `q`, to within `eps`, the ratio read as well.
fn realised<S: Selection>(
    pair: [u32; 2],
    held: f64,
    q: f64,
    eps: f64,
    p: &LinkParams<f64>,
) -> bool {
    let m = masses(pair, held, q);
    let back =
        <S::MassLink as Link<2, 3>>::forward_at(<S::MassLink as Link<2, 3>>::inverse_at(m, p), p);
    let ratio = back[pair[1] as usize] / back[pair[0] as usize];
    each_within(back, m, eps) && (ratio - q).abs() <= eps * q
}

/// The ratios even in `ln q` over `[lo, hi]`, step at most 0.05.
fn scan(lo: f64, hi: f64) -> impl Iterator<Item = f64> {
    let (a, b) = (lo.ln(), hi.ln());
    let n = ((b - a) / 0.05).ceil() as u32;
    (0..=n).map(move |i| {
        (a + (b - a) * f64::from(i) / f64::from(n))
            .exp()
            .clamp(lo, hi)
    })
}

/// How a check gets its chart: the chart's own warped build, or a control's.
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

type Ratio<S> = MixedMassRatio<ZBeta, S, f64>;

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
        if !realised::<S>(pair, chart.held, centre, 0.5 * eps_mu(), p) {
            return None;
        }
    }
    match how.ends {
        Some([a, b]) => new([a * chart.ratio[0], b * chart.ratio[1]], Mapping::Linear),
        None => Some(chart),
    }
}

/// `f` at every slice, pair and asked range under `S`, given the held mass.
fn slices<S: Selection>(p: &LinkParams<f64>, mut f: impl FnMut([f64; 8], [u32; 2], [f64; 2], f64)) {
    for z6 in MASS {
        for z7 in MASS {
            let z = slice(z6, z7);
            for pair in PAIRS {
                let held = Ratio::<S>::new(z, [-1.0, 1.0], [1.0, 2.0], pair, Mapping::Linear, p)
                    .expect("a linear chart")
                    .held;
                for asked in ASKED {
                    f(z, pair, asked, held);
                }
            }
        }
    }
}

/// The scanned ratios of `asked` the oracle finds realised, well inside `ε_μ`.
fn found<S: Selection>(
    pair: [u32; 2],
    held: f64,
    asked: [f64; 2],
    p: &LinkParams<f64>,
) -> Vec<f64> {
    scan(asked[0], asked[1])
        .filter(|&q| realised::<S>(pair, held, q, 0.25 * eps_mu(), p))
        .collect()
}

/// Whether the centre of the slice `z` is not realised.
fn saturated<S: Selection>(z: [f64; 8], pair: [u32; 2], held: f64, p: &LinkParams<f64>) -> bool {
    let m = <S::MassLink as Link<2, 3>>::forward_at([z[6], z[7]], p);
    let centre = m[pair[1] as usize] / m[pair[0] as usize];
    !realised::<S>(pair, held, centre, 0.5 * eps_mu(), p)
}

fn offers_arm<S: Selection>(how: Build, p: &LinkParams<f64>) -> usize {
    let mut seeded = 0;
    slices::<S>(p, |z, pair, asked, held| {
        let found = found::<S>(pair, held, asked, p);
        if found.len() < 5 {
            return;
        }
        assert!(
            build::<S>(how, z, pair, asked, p).is_some(),
            "μ_max {}, z ({}, {}), bodies {pair:?}, asked {asked:?}: refused though {} scanned ratios are realised, \
             from {}",
            p.mu_max,
            z[6],
            z[7],
            found.len(),
            found[0]
        );
        seeded += usize::from(saturated::<S>(z, pair, held, p));
    });
    seeded
}

fn range_arm<S: Selection>(how: Build, p: &LinkParams<f64>) -> usize {
    let mut seeded = 0;
    slices::<S>(p, |z, pair, asked, held| {
        let Some(chart) = build::<S>(how, z, pair, asked, p) else {
            return;
        };
        let at = format!(
            "μ_max {}, z ({}, {}), bodies {pair:?}, asked {asked:?}",
            p.mu_max, z[6], z[7]
        );
        let [q0, q1] = chart.ratio;
        for q in found::<S>(pair, held, asked, p) {
            assert!(
                q0 <= q && q <= q1,
                "{at}: the realised q = {q} is outside the range [{q0}, {q1}]"
            );
        }
        assert!(
            asked[0] <= q0 && q1 <= asked[1],
            "{at}: the range [{q0}, {q1}] leaves the asked range"
        );
        seeded += usize::from(saturated::<S>(z, pair, held, p));
    });
    seeded
}

/// A chart of `asked` over at least 0.2 in `ln q`, its end `out` moved out where the range stops short of it (that end
/// the link's edge, so moving it changes nothing), to `far`.
fn far_arm<S: Selection>(how: Build, p: &LinkParams<f64>, out: usize, far: f64) -> usize {
    let mut seeded = 0;
    slices::<S>(p, |z, pair, asked, held| {
        let Some(chart) = build::<S>(CHART, z, pair, asked, p) else {
            return;
        };
        let [q0, q1] = chart.ratio;
        if (q1 / q0).ln() < 0.2 || chart.ratio[out] == asked[out] {
            return;
        }
        let mut moved = asked;
        moved[out] = far;
        let Some(wide) = build::<S>(how, z, pair, moved, p) else {
            panic!(
                "μ_max {}, z ({}, {}), bodies {pair:?}: asked {asked:?} is offered, over [{q0}, {q1}], and asked \
                 {moved:?} is refused",
                p.mu_max, z[6], z[7]
            );
        };
        let same = wide
            .ratio
            .iter()
            .zip([q0, q1])
            .all(|(got, want)| (got - want).abs() <= 1.0e-3 * want);
        assert!(
            same,
            "μ_max {}, z ({}, {}), bodies {pair:?}: asked {moved:?} draws {:?}, not [{q0}, {q1}]",
            p.mu_max, z[6], z[7], wide.ratio
        );
        seeded += usize::from(saturated::<S>(z, pair, held, p));
    });
    seeded
}

fn zero_arm<S: Selection>(how: Build, p: &LinkParams<f64>) -> usize {
    far_arm::<S>(how, p, 0, 0.0)
}

fn top_arm<S: Selection>(how: Build, p: &LinkParams<f64>) -> usize {
    far_arm::<S>(how, p, 1, 1.0e300)
}

fn softmax_params() -> Vec<LinkParams<f64>> {
    let mu = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's allowed range");
    [mu.lo, LinkParams::DEFAULT.mu_max, mu.hi]
        .into_iter()
        .map(|mu_max| LinkParams {
            mu_max,
            ..LinkParams::DEFAULT
        })
        .collect()
}

/// Every link and parameter set through `arm`, its count of charts from an unrealised centre at least 100, so the
/// grid reaches the seed.
macro_rules! every_link {
    ($arm:ident, $how:expr) => {{
        let mut seeded = $arm::<Stick>($how, &LinkParams::DEFAULT);
        for p in softmax_params() {
            seeded += $arm::<Defaults>($how, &p);
        }
        assert!(
            seeded >= 100,
            "only {seeded} charts from an unrealised centre"
        );
    }};
}

fn check_offers(how: Build) {
    every_link!(offers_arm, how);
}

#[test]
fn qa_seed_offers_a_realised_range() {
    check_offers(CHART);
}

negative_control!(
    qa_seed_offers_a_realised_range,
    "a build that gives up on an unrealised centre refuses ranges the link realises, so it must fail",
    expected = "refused though",
    check_offers(Build {
        give_up: true,
        ends: None
    })
);

fn check_range(how: Build) {
    every_link!(range_arm, how);
}

#[test]
fn qa_seed_range_is_the_realised_part() {
    check_range(CHART);
}

negative_control!(
    qa_seed_range_is_the_realised_part,
    "a range cut short leaves realised ratios out, so it must fail",
    expected = "is outside the range",
    check_range(Build {
        give_up: false,
        ends: Some([1.0, 0.9])
    })
);

fn check_zero(how: Build) {
    every_link!(zero_arm, how);
}

#[test]
fn qa_seed_asked_end_at_zero() {
    check_zero(CHART);
}

negative_control!(
    qa_seed_asked_end_at_zero,
    "a build that gives up on an unrealised centre refuses the moved range, so it must fail",
    expected = "is refused",
    check_zero(Build {
        give_up: true,
        ends: None
    })
);

fn check_top(how: Build) {
    every_link!(top_arm, how);
}

#[test]
fn qa_seed_asked_end_far_out() {
    check_top(CHART);
}

negative_control!(
    qa_seed_asked_end_far_out,
    "a build that gives up on an unrealised centre refuses the moved range, so it must fail",
    expected = "is refused",
    check_top(Build {
        give_up: true,
        ends: None
    })
);

fn pixels_arm<S: Selection>(how: Build, p: &LinkParams<f64>) -> usize {
    let eps = eps_mu();
    let mut drawn = 0;
    slices::<S>(p, |z, pair, asked, held| {
        let Some(chart) = build::<S>(how, z, pair, asked, p) else {
            return;
        };
        for j in 0..=64 {
            let v = f64::from(j) / 64.0;
            let q = chart.ratio_at::<f64>(v);
            let got = chart.decode_params::<S, f64>(0.5, v, p).ic.m;
            assert!(
                chart.validate_params(0.5, v, p) == ValidationResult::Valid
                    && each_within(got, masses(pair, held, q), eps),
                "μ_max {}, z ({}, {}), bodies {pair:?}, asked {asked:?}: the row {v} at q = {q} is not a system of the \
                 slice, masses {got:?}",
                p.mu_max,
                z[6],
                z[7]
            );
            drawn += 1;
        }
    });
    drawn
}

fn check_pixels(how: Build) {
    let mut drawn = pixels_arm::<Stick>(how, &LinkParams::DEFAULT);
    for p in softmax_params() {
        drawn += pixels_arm::<Defaults>(how, &p);
    }
    assert!(drawn > 0, "no pixel drawn");
}

#[test]
fn qa_seed_every_pixel() {
    check_pixels(CHART);
}

negative_control!(
    qa_seed_every_pixel,
    "a range past the realised part's ends draws ratios the link does not realise, so it must fail",
    expected = "is not a system of the",
    check_pixels(Build {
        give_up: false,
        ends: Some([0.9, 1.1])
    })
);
