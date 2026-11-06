//! qa's tests for TASK-M2-37, fix round 2: a mass ratio is realised only with the third body's mass at its held value
//! (REQ-CHART-070; chart_reference §2.1: "A ratio is realised when the link's inverse followed by its forward returns
//! its masses `(m_a, m_b, m_c)`, `m_c` at its held value, each to within `ε_μ`, relative"). Written from that
//! paragraph, not from the implementation; each test has its negative control (R-176).
//!
//! The slices are a grid of mass controls `{−3, −1, 0, 1, 3}²` beside a fixed `z_β` slice, every ordered body pair,
//! under `softmax_tanh` at both ends of `μ_max`'s allowed range (`validation::tunables`) and at its default, and under
//! `stick_breaking`. On some of them the logit that sets `m_c` saturates before the ratio's does (for `softmax_tanh`
//! and bodies `(0, 1)`, `ln(m₂/m₀) = ln(m_c·(1 + q)/(1 − m_c))` reaches the clamp before `ln q` does when `m_c > ½`),
//! so the round trip keeps the ratio and moves `m_c`; the controls show the grid contains such slices.
//!
//! - `qa_held_realised_implies_held`: on a grid of `q = 10^{k/50}`, `k ∈ [−300, 300]`, every `q` that
//!   [`MixedMassRatio::realised`] accepts round-trips to `m_c` at the held value, and the ratio `q`, to within `ε_μ`.
//!   Control: the ratio-only round trip, blind to `m_c`, in `realised`'s place.
//! - `qa_held_realised_each_mass`: on that grid, every `q` it accepts returns each of `(m_a, m_b, m_c)` to within
//!   `ε_μ`, relative. Control: the round trip that reads the ratio and `m_c` but not the two swept masses, which a slice
//!   with `m_c` near 1 fools: the ratio and `m_c` hold while `m_a` sits on the link's floor.
//! - `qa_held_realised_not_over_refused`: on that grid, every `q` whose ratio and every mass come back inside `ε_μ/16`
//!   is accepted. Control: `realised` narrowed to `q ≥ 1`.
//! - `qa_held_warped_every_pixel`: every pixel of the warped chart validates and decodes `m_c` at the held value and
//!   the ratio `q(t)`. Control: the range a ratio-only realisation would give, drawn over the same pixels.
//! - `qa_held_linear_refuses`: on the linear option over the asked range, every pixel that validates decodes `m_c` at
//!   the held value and the ratio `q(t)`. Control: the pixels gated by the ratio's round trip alone.
//!
//! **Values.** `ε_μ` is the register's link clamp (§2.1, reused as the round trip's tolerance); `ε_μ/16` is this
//! file's margin for "well inside". Every slice, range and grid here is test data.

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

/// The ordered body pairs `(a, b)`, `q = m_b/m_a`.
const PAIRS: [[u32; 2]; 6] = [[0, 1], [1, 0], [0, 2], [2, 0], [1, 2], [2, 1]];
/// The mass controls of each slice, per coordinate.
const MASS: [f64; 5] = [-3.0, -1.0, 0.0, 1.0, 3.0];
/// The asked ranges.
const ASKED: [[f64; 2]; 2] = [[0.05, 20.0], [1e-4, 1e4]];

/// The slice centre with the mass controls `(z6, z7)`.
fn slice(z6: f64, z7: f64) -> [f64; 8] {
    [0.2, -0.3, 0.1, 0.15, -0.05, 0.25, z6, z7]
}

/// The `softmax_tanh` parameter sets: `μ_max` at its allowed range's ends and at its default.
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

/// §2.1's masses for the ratio `q` at the held `m_c`: `m_a = (1 − m_c)/(1 + q)`, `m_b = q·m_a`, `m_c` the held.
fn masses(pair: [u32; 2], held: f64, q: f64) -> [f64; 3] {
    let m_a = (1.0 - held) / (1.0 + q);
    let mut m = [held; 3];
    m[pair[0] as usize] = m_a;
    m[pair[1] as usize] = q * m_a;
    m
}

fn third(pair: [u32; 2]) -> usize {
    (3 - pair[0] - pair[1]) as usize
}

/// The round trip's errors at `q`.
struct Trip {
    /// The ratio's, relative.
    ratio: f64,
    /// `m_c`'s, relative to the held value.
    m_c: f64,
    /// The sum's.
    sum: f64,
    /// The largest of the three masses' (`m_c`'s among them), relative; `NaN` if any is.
    worst: f64,
    /// Whether every mass came back positive.
    positive: bool,
}

/// The round trip's errors at `q`.
fn round_trip<S: Selection>(pair: [u32; 2], held: f64, q: f64, p: &LinkParams<f64>) -> Trip {
    let m = masses(pair, held, q);
    let back =
        <S::MassLink as Link<2, 3>>::forward_at(<S::MassLink as Link<2, 3>>::inverse_at(m, p), p);
    let ratio = back[pair[1] as usize] / back[pair[0] as usize];
    Trip {
        ratio: (ratio - q).abs() / q,
        m_c: (back[third(pair)] - held).abs() / held,
        sum: (back.iter().sum::<f64>() - 1.0).abs(),
        worst: back
            .iter()
            .zip(m)
            .map(|(&x, m)| (x - m).abs() / m)
            .fold(0.0, |w: f64, e| if e > w || e.is_nan() { e } else { w }),
        positive: back.iter().all(|&x| x > 0.0),
    }
}

/// The ratio-only realisation, blind to `m_c` (the controls' predicate).
fn ratio_only<S: Selection>(
    pair: [u32; 2],
    held: f64,
    q: f64,
    eps: f64,
    p: &LinkParams<f64>,
) -> bool {
    if !(q.is_finite() && q > 0.0) {
        return false;
    }
    let t = round_trip::<S>(pair, held, q, p);
    t.positive && t.ratio <= eps && t.sum <= eps
}

/// Every `(slice, pair, p)` under `S`, with the linear chart over `[0.05, 20]` (for its `held`).
fn each<S: Selection>(p: &LinkParams<f64>, mut f: impl FnMut([f64; 8], [u32; 2], f64)) {
    for z6 in MASS {
        for z7 in MASS {
            for pair in PAIRS {
                let z = slice(z6, z7);
                let chart = MixedMassRatio::<ZBeta, S, f64>::new(
                    z,
                    [-1.0, 1.0],
                    [0.05, 20.0],
                    pair,
                    Mapping::Linear,
                    p,
                )
                .expect("a linear chart");
                f(z, pair, chart.held);
            }
        }
    }
}

// ── realised(q) means m_c at the held value ─────────────────────────────────────────────────────────────────────────

/// What a check takes in `realised`'s place: from `realised`'s answer, `q` and the round trip, whether `q` is accepted.
type Gate = fn(bool, f64, &Trip) -> bool;

/// `realised` itself.
const REALISED: Gate = |yes, _, _| yes;

/// One point of the grid, with its gate's answer.
struct Point {
    link: &'static str,
    mu_max: f64,
    z: [f64; 8],
    pair: [u32; 2],
    held: f64,
    q: f64,
    yes: bool,
    trip: Trip,
}

impl Point {
    /// The slice, for a failure's message.
    fn at(&self) -> String {
        format!(
            "{} at μ_max = {}, mass controls ({}, {}), bodies {:?}",
            self.link, self.mu_max, self.z[6], self.z[7], self.pair
        )
    }
}

fn walk_arm<S: Selection>(
    link: &'static str,
    p: &LinkParams<f64>,
    gate: Gate,
    f: &mut impl FnMut(&Point),
) {
    each::<S>(p, |z, pair, held| {
        let chart = MixedMassRatio::<ZBeta, S, f64>::new(
            z,
            [-1.0, 1.0],
            [0.05, 20.0],
            pair,
            Mapping::Linear,
            p,
        )
        .expect("a linear chart");
        for k in -300..=300 {
            let q = 10f64.powf(f64::from(k) / 50.0);
            let trip = round_trip::<S>(pair, held, q, p);
            f(&Point {
                link,
                mu_max: p.mu_max,
                z,
                pair,
                held,
                q,
                yes: gate(chart.realised(q, p), q, &trip),
                trip,
            });
        }
    });
}

/// `f` at every grid point of every slice, under each link and parameter set.
fn walk(gate: Gate, mut f: impl FnMut(&Point)) {
    for p in softmax_params() {
        walk_arm::<Defaults>("softmax_tanh", &p, gate, &mut f);
    }
    walk_arm::<Stick>("stick_breaking", &LinkParams::DEFAULT, gate, &mut f);
}

fn check_realised(gate: Gate) {
    let eps = eps_mu();
    let mut accepted = 0;
    walk(gate, |pt| {
        if pt.yes {
            accepted += 1;
            let t = &pt.trip;
            assert!(
                t.positive && t.ratio <= eps && t.sum <= eps && t.m_c <= eps,
                "{}: realised q = {}, but its round trip moves m_c off the held {} by {} relative (ratio error {})",
                pt.at(),
                pt.q,
                pt.held,
                t.m_c,
                t.ratio
            );
        }
    });
    assert!(accepted > 0, "no ratio realised anywhere");
}

#[test]
fn qa_held_realised_implies_held() {
    check_realised(REALISED);
}

negative_control!(
    qa_held_realised_implies_held,
    "the ratio-only round trip in realised's place accepts ratios past the m_c logit's clamp, so it must fail",
    expected = "but its round trip moves m_c off the held",
    check_realised(|_, _, t| t.positive && t.ratio <= eps_mu() && t.sum <= eps_mu())
);

fn check_each_mass(gate: Gate) {
    let eps = eps_mu();
    let mut accepted = 0;
    walk(gate, |pt| {
        if pt.yes {
            accepted += 1;
            assert!(
                pt.trip.worst <= eps,
                "{}: realised q = {}, but its round trip moves a mass by {} relative (m_c by {}, the ratio by {})",
                pt.at(),
                pt.q,
                pt.trip.worst,
                pt.trip.m_c,
                pt.trip.ratio
            );
        }
    });
    assert!(accepted > 0, "no ratio realised anywhere");
}

#[test]
fn qa_held_realised_each_mass() {
    check_each_mass(REALISED);
}

negative_control!(
    qa_held_realised_each_mass,
    "the round trip that reads the ratio and m_c but not the swept masses accepts ratios whose m_a sits on the link's \
     floor, so it must fail",
    expected = "but its round trip moves a mass by",
    check_each_mass(|_, _, t| {
        let eps = eps_mu();
        t.positive && t.ratio <= eps && t.sum <= eps && t.m_c <= eps
    })
);

fn check_not_over_refused(gate: Gate) {
    let well_inside = eps_mu() / 16.0;
    let mut refused = 0;
    walk(gate, |pt| {
        if !pt.yes {
            refused += 1;
            let t = &pt.trip;
            assert!(
                !(t.positive
                    && t.ratio <= well_inside
                    && t.sum <= well_inside
                    && t.worst <= well_inside),
                "{}: q = {} refused, its round trip well inside ε_μ (each mass to {}, the ratio to {})",
                pt.at(),
                pt.q,
                t.worst,
                t.ratio
            );
        }
    });
    assert!(refused > 0, "no ratio refused anywhere");
}

#[test]
fn qa_held_realised_not_over_refused() {
    check_not_over_refused(REALISED);
}

negative_control!(
    qa_held_realised_not_over_refused,
    "realised narrowed to q ≥ 1 refuses ratios whose round trip is well inside ε_μ, so it must fail",
    expected = "refused, its round trip well inside",
    check_not_over_refused(|yes, q, _| yes && q >= 1.0)
);

// ── The warped chart: every pixel at the held m_c ───────────────────────────────────────────────────────────────────

/// The end of the ratio-only realised range from `from` toward `toward`, by bisection in `ln q` (the control's range).
fn ratio_only_edge<S: Selection>(
    pair: [u32; 2],
    held: f64,
    from: f64,
    toward: f64,
    p: &LinkParams<f64>,
) -> f64 {
    let eps = 0.5 * eps_mu();
    if ratio_only::<S>(pair, held, toward, eps, p) {
        return toward;
    }
    let (mut ok, mut bad) = (from.ln(), toward.ln());
    for _ in 0..64 {
        let mid = 0.5 * (ok + bad);
        if ratio_only::<S>(pair, held, mid.exp(), eps, p) {
            ok = mid;
        } else {
            bad = mid;
        }
    }
    ok.exp()
}

/// The pixels `(u, v)`: five columns (the ratio does not depend on `u`) by 65 rows over `[0, 1]`.
fn pixels() -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    for i in 0..5 {
        for j in 0..65 {
            out.push([f64::from(i) / 4.0, f64::from(j) / 64.0]);
        }
    }
    out
}

/// Every pixel of `chart` validates and decodes `m_c` at the held value and the ratio `q(v)`, to within `ε_μ`.
fn check_pixels<S: Selection>(
    link: &str,
    chart: &MixedMassRatio<ZBeta, S, f64>,
    p: &LinkParams<f64>,
    z: [f64; 8],
) {
    let eps = eps_mu();
    let pair = chart.bodies;
    for [u, v] in pixels() {
        let m = chart.decode_params::<S, f64>(u, v, p).ic.m;
        let q = chart.ratio_at::<f64>(v);
        let ratio = m[pair[1] as usize] / m[pair[0] as usize];
        let c = (m[third(pair)] - chart.held).abs() / chart.held;
        assert!(
            c <= eps,
            "{link} at μ_max = {}, mass controls ({}, {}), bodies {pair:?}: the pixel ({u}, {v}) at q = {q} decodes \
             m_c = {}, off the held {} by {c}",
            p.mu_max,
            z[6],
            z[7],
            m[third(pair)],
            chart.held
        );
        assert!(
            (ratio - q).abs() <= eps * q && chart.validate_params(u, v, p) == ValidationResult::Valid,
            "{link} at μ_max = {}, mass controls ({}, {}), bodies {pair:?}: the pixel ({u}, {v}) decodes the ratio \
             {ratio}, not {q}, or does not validate",
            p.mu_max,
            z[6],
            z[7]
        );
    }
}

fn check_warped_arm<S: Selection>(link: &str, p: &LinkParams<f64>, blind: bool) -> usize {
    let mut built = 0;
    each::<S>(p, |z, pair, held| {
        for asked in ASKED {
            let chart = if blind {
                // The range a ratio-only realisation gives from the centre's ratio, drawn over the same pixels.
                let m = <S::MassLink as Link<2, 3>>::forward_at([z[6], z[7]], p);
                let centre = m[pair[1] as usize] / m[pair[0] as usize];
                let lo = ratio_only_edge::<S>(pair, held, centre, asked[0], p);
                let hi = ratio_only_edge::<S>(pair, held, centre, asked[1], p);
                if lo >= hi {
                    continue;
                }
                MixedMassRatio::<ZBeta, S, f64>::new(
                    z,
                    [-1.0, 1.0],
                    [lo, hi],
                    pair,
                    Mapping::Linear,
                    p,
                )
            } else {
                MixedMassRatio::<ZBeta, S, f64>::new(
                    z,
                    [-1.0, 1.0],
                    asked,
                    pair,
                    Mapping::Warped,
                    p,
                )
            };
            if let Some(chart) = chart {
                built += 1;
                check_pixels(link, &chart, p, z);
            }
        }
    });
    built
}

fn check_warped(blind: bool) {
    let mut built = 0;
    for p in softmax_params() {
        built += check_warped_arm::<Defaults>("softmax_tanh", &p, blind);
    }
    built += check_warped_arm::<Stick>("stick_breaking", &LinkParams::DEFAULT, blind);
    // The grid's slices nearly all realise some part of the asked range; a warp that built none would test nothing.
    assert!(built >= 1000, "only {built} warped charts built");
}

#[test]
fn qa_held_warped_every_pixel() {
    check_warped(false);
}

negative_control!(
    qa_held_warped_every_pixel,
    "the range a ratio-only realisation gives reaches past the m_c logit's clamp, its pixels decoding m_c off the \
     held value, so it must fail",
    expected = "off the held",
    check_warped(true)
);

// ── The linear option: a pixel that validates is at the held m_c ───────────────────────────────────────────────────

fn check_linear_arm<S: Selection>(link: &str, p: &LinkParams<f64>, blind: bool) -> (usize, usize) {
    let eps = eps_mu();
    let (mut taken, mut refused) = (0, 0);
    each::<S>(p, |z, pair, held| {
        for asked in ASKED {
            let chart = MixedMassRatio::<ZBeta, S, f64>::new(
                z,
                [-1.0, 1.0],
                asked,
                pair,
                Mapping::Linear,
                p,
            )
            .expect("a linear chart");
            for j in 0..65 {
                let v = f64::from(j) / 64.0;
                let u = 0.5;
                let q = chart.ratio_at::<f64>(v);
                let gate = if blind {
                    ratio_only::<S>(pair, held, q, eps, p)
                } else {
                    chart.validate_params(u, v, p) == ValidationResult::Valid
                };
                if !gate {
                    refused += 1;
                    continue;
                }
                taken += 1;
                let m = chart.decode_params::<S, f64>(u, v, p).ic.m;
                let ratio = m[pair[1] as usize] / m[pair[0] as usize];
                let c = (m[third(pair)] - held).abs() / held;
                assert!(
                    c <= eps && (ratio - q).abs() <= eps * q,
                    "{link} at μ_max = {}, mass controls ({}, {}), bodies {pair:?}: the linear pixel at q = {q} is \
                     taken, decoding m_c = {} against the held {held} ({c} relative), ratio {ratio}",
                    p.mu_max,
                    z[6],
                    z[7],
                    m[third(pair)]
                );
            }
        }
    });
    (taken, refused)
}

fn check_linear(blind: bool) {
    let (mut taken, mut refused) = (0, 0);
    for p in softmax_params() {
        let (t, r) = check_linear_arm::<Defaults>("softmax_tanh", &p, blind);
        taken += t;
        refused += r;
    }
    let (t, r) = check_linear_arm::<Stick>("stick_breaking", &LinkParams::DEFAULT, blind);
    taken += t;
    refused += r;
    assert!(
        taken > 0 && refused > 0,
        "{taken} pixels taken, {refused} refused: the grid exercises one side only"
    );
}

#[test]
fn qa_held_linear_refuses() {
    check_linear(false);
}

negative_control!(
    qa_held_linear_refuses,
    "pixels gated by the ratio's round trip alone include ratios whose round trip moves m_c, so it must fail",
    expected = "is taken, decoding m_c",
    check_linear(true)
);
