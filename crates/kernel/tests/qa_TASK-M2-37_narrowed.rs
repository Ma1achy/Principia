//! qa's tests for TASK-M2-37, fix round 7: the warped mass ratio's chart narrowed to `f32`, pixel by pixel, and the two
//! searches `new` runs brought to their ends (chart_reference §2.1: "every pixel of `[0,1]²` is feasible"; "A chart
//! decoded at `f32` holds `m_c` narrowed to `f32` [...] so the range is the part realised at `m_c` both as held and as
//! narrowed"; REQ-CHART-067, REQ-CHART-070). Written from that paragraph, not from the search; each test has its negative
//! control (R-176).
//!
//! - `qa_narrowed_held_near_one`: the slice `softmax_tanh`, `μ_max = 10`, mass controls `(0, 3)`, bodies `(0, 1)`, asked
//!   `[0.5, 2]`, where the held mass rounds to `0.99990463` and the two small masses move by `2.8e-4`, relative: every
//!   pixel of its narrowed chart is valid and decodes at `f32` the masses §2.1 writes
//!   at the narrowed `m_c`. Controls: the range found at the held mass as held alone, whose top row the narrowed chart
//!   refuses; and the masses at the held mass as held, which the `f32` decode does not give.
//! - `qa_narrowed_every_pixel`: the same over a family of slices, both links, every ordered pair, `μ_max` over its
//!   allowed range, with the rows packed against both ends of `t`, where the range's edge is, down to single `f32`s.
//!   Slices where the narrowing moves the small masses by more than `ε_μ/4` are counted, so the family cannot lose them.
//!   `μ_max` includes values `f32` rounds by nearly half a spacing. Control: the range found as held alone.
//! - `qa_narrowed_decode_softmax` and `qa_narrowed_decode_stick`: on that family the `f32` decode gives the masses §2.1
//!   writes, each to `DECODE_TOL`, relative, not only some masses that are positive and sum to 1: a pixel that validates
//!   and decodes another system is no pixel of the chart. Control: `qa_narrowed_decode_holds_narrowed_mass`'s.
//! - `qa_searches_end`: `new` returns on every ask that drives the searches to their ends, with no count of steps to
//!   stop them (the doubles' own spacing does): the widest asks, asks about `q = 1` adjacent in doubles, asks out of
//!   reach, with centres the link does not realise. Whatever it returns lies in the ask. Control: a stand-in search whose
//!   closing test cannot fire, which the watchdog stops.
//!
//! **Values.** `ε_μ` is the register's link clamp (§2.1), the sum held to it as `qa_TASK-M2-37_any_ask.rs` holds it;
//! `DECODE_TOL` is parity_contract §4's relative IC/decode figure, for an `f32` decode against the `f64` formula;
//! `ε_μ/4` is this file's line for "the narrowing bites"; the watchdog's 120 s is far past any build here, a fraction of
//! a second. `μ_max` stays inside REQ-GEN-038's range, capped at 10 (R-597). Every slice, ask and row is test data.

use std::panic::resume_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

/// parity_contract §4, IC/decode, relative.
const DECODE_TOL: f64 = 1.0e-5;

fn eps_mu() -> f64 {
    <f64 as ChartConstants>::EPS_MU
}

fn slice(z6: f64, z7: f64) -> [f64; 8] {
    [0.2, -0.3, 0.1, 0.15, -0.05, 0.25, z6, z7]
}

fn mu(mu_max: f64) -> LinkParams<f64> {
    LinkParams {
        mu_max,
        ..LinkParams::DEFAULT
    }
}

fn warped<S: Selection>(
    z: [f64; 2],
    pair: [u32; 2],
    asked: [f64; 2],
    p: &LinkParams<f64>,
) -> Option<Ratio<S>> {
    Ratio::<S>::new(
        slice(z[0], z[1]),
        [-1.0, 1.0],
        asked,
        pair,
        Mapping::Warped,
        p,
    )
}

/// §2.1's round trip at `q`, at the held mass as held, each mass and the ratio to within `ε_μ/2`: the test's own, reading
/// the link and no search.
fn trip<S: Selection>(pair: [u32; 2], held: f64, q: f64, p: &LinkParams<f64>) -> bool {
    let m_a = (1.0 - held) / (1.0 + q);
    let mut m = [held; 3];
    m[pair[0] as usize] = m_a;
    m[pair[1] as usize] = q * m_a;
    let back =
        <S::MassLink as Link<2, 3>>::forward_at(<S::MassLink as Link<2, 3>>::inverse_at(m, p), p);
    let ratio = back[pair[1] as usize] / back[pair[0] as usize];
    let eps = 0.5 * eps_mu();
    back.iter().zip(m).all(|(&x, m)| (x - m).abs() <= eps * m) && (ratio - q).abs() <= eps * q
}

/// How a check takes the chart's range.
#[derive(Clone, Copy)]
enum Range {
    /// The range `new` found.
    Built,
    /// Outward to the ask, the held mass as held alone: what a range blind to the narrowing gives.
    AsHeld,
}

/// `chart`'s range taken as `how` says, `asked` being what it was built on.
fn ranged<S: Selection>(
    mut chart: Ratio<S>,
    how: Range,
    asked: [f64; 2],
    p: &LinkParams<f64>,
) -> Ratio<S> {
    if let Range::AsHeld = how {
        chart.ratio = [0, 1].map(|i| {
            let (mut ok, mut bad) = (chart.ratio[i].ln(), asked[i].max(f64::MIN_POSITIVE).ln());
            for _ in 0..128 {
                let mid = 0.5 * (ok + bad);
                if trip::<S>(chart.bodies, chart.held, mid.exp(), p) {
                    ok = mid;
                } else {
                    bad = mid;
                }
            }
            ok.exp()
        });
    }
    chart
}

/// The held mass a check holds the `f32` decode to.
#[derive(Clone, Copy)]
enum Target {
    /// None: the decoded masses are not compared with §2.1's.
    Unread,
    /// As narrowed, which an `f32` decode holds (§2.1).
    Narrowed,
    /// As held, which it does not.
    AsHeld,
}

/// §2.1's masses at the ratio `q`, the held mass `held`.
fn masses(pair: [u32; 2], held: f64, q: f64) -> [f64; 3] {
    let m_a = (1.0 - held) / (1.0 + q);
    let mut m = [held; 3];
    m[pair[0] as usize] = m_a;
    m[pair[1] as usize] = q * m_a;
    m
}

/// The rows: 257 across the square, and at each end the floats up to sixteen spacings of `f32` in from it, where the
/// range's edge is.
fn rows() -> Vec<f32> {
    let spacing = f32::EPSILON / 2.0;
    let mut all: Vec<f32> = (0..=256).map(|j| j as f32 / 256.0).collect();
    all.extend((1..=16).map(|k| 1.0 - k as f32 * spacing));
    all.extend((1..=16).map(|k| k as f32 * spacing));
    all.push(f32::MIN_POSITIVE);
    all
}

/// Every pixel of `chart` narrowed to `f32`: valid in every column at the active parameters `p` (§2.1 narrows `m_c`, not
/// them), and its masses at `f32`, on `p` as `f32` reads it, a system, the one §2.1 writes at the `target` held mass.
fn pixels<S: Selection>(chart: &Ratio<S>, at: &str, target: Target, p: &LinkParams<f64>) {
    let narrowed = chart.with_params(|x| x as f32);
    let thin = LinkParams {
        mu_max: p.mu_max as f32,
        q_max: p.q_max as f32,
    };
    let wide = narrowed.at_f64();
    let held = match target {
        Target::Unread => None,
        Target::Narrowed => Some(wide.held),
        Target::AsHeld => Some(chart.held),
    };
    for v in rows() {
        for u in [0.0_f32, 0.25, 0.5, 1.0] {
            let got = narrowed.validate_params(f64::from(u), f64::from(v), p);
            assert!(
                got == ValidationResult::Valid,
                "{at}: the in-square pixel ({u}, {v}) of {:?} is refused on f32: {got:?}",
                chart.ratio
            );
        }
        let m = narrowed.decode_params::<S, f32>(0.5, v, &thin).ic.m;
        let sum: f64 = m.iter().map(|&m| f64::from(m)).sum();
        assert!(
            m.iter().all(|&m| m.is_finite() && m > 0.0) && (sum - 1.0).abs() <= eps_mu(),
            "{at}: at f32 the row {v} decodes the masses {m:?}"
        );
        if let Some(held) = held {
            let want = masses(chart.bodies, held, wide.ratio_at(f64::from(v)));
            assert!(
                m.iter()
                    .zip(want)
                    .all(|(&m, w)| (f64::from(m) - w).abs() <= DECODE_TOL * w),
                "{at}: at f32 the row {v} is no system of the slice: {m:?} against {want:?}"
            );
        }
    }
}

fn check_near_one(how: Range, target: Target) {
    let p = mu(10.0);
    let asked = [0.5, 2.0];
    let chart = warped::<Defaults>([0.0, 3.0], [0, 1], asked, &p).expect("the chart");
    let moved = (f64::from(chart.held as f32) - chart.held).abs() / (1.0 - chart.held);
    assert!(
        moved > 100.0 * eps_mu(),
        "the slice lost what it is here for: the narrowing moves the small masses by {moved}"
    );
    pixels(
        &ranged(chart, how, asked, &p),
        "μ_max 10, z (0, 3), bodies (0, 1)",
        target,
        &p,
    );
}

#[test]
fn qa_narrowed_held_near_one() {
    check_near_one(Range::Built, Target::Narrowed);
}

negative_control!(
    qa_narrowed_held_near_one,
    "a range found at the held mass as held alone reaches ratios the narrowed mass does not realise, so it must fail",
    expected = "is refused on f32",
    check_near_one(Range::AsHeld, Target::Narrowed)
);

#[test]
fn qa_narrowed_decode_holds_narrowed_mass() {
    check_near_one(Range::Built, Target::Narrowed);
}

negative_control!(
    qa_narrowed_decode_holds_narrowed_mass,
    "the f32 decode holds the mass as narrowed, so held to the mass as held it must fail, though every pixel validates",
    expected = "is no system of the slice",
    check_near_one(Range::Built, Target::AsHeld)
);

/// The family at the link `S`: `(charts, charts the narrowing bites)`.
fn family<S: Selection>(how: Range, target: Target, p: &LinkParams<f64>) -> (u32, u32) {
    const CONTROLS: [f64; 13] = [
        -6.0, -4.0, -3.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 6.0,
    ];
    const ASKS: [[f64; 2]; 3] = [[0.5, 2.0], [1.0e-3, 1.0e3], [0.0, 1.0e300]];
    let (mut charts, mut bites) = (0, 0);
    let mut turn = 0_u32;
    for z6 in CONTROLS {
        for z7 in CONTROLS {
            for pair in PAIRS {
                for asked in ASKS {
                    let Some(chart) = warped::<S>([z6, z7], pair, asked, p) else {
                        continue;
                    };
                    let shift = (f64::from(chart.held as f32) - chart.held).abs();
                    let bitten = shift / (1.0 - chart.held) > 0.25 * eps_mu();
                    turn += 1;
                    if !bitten && !turn.is_multiple_of(29) {
                        continue;
                    }
                    let at = format!(
                        "μ_max {}, z ({z6}, {z7}), bodies {pair:?}, asked {asked:?}, held {}",
                        p.mu_max, chart.held
                    );
                    pixels(&ranged(chart, how, asked, p), &at, target, p);
                    charts += 1;
                    bites += u32::from(bitten);
                }
            }
        }
    }
    (charts, bites)
}

/// `μ_max` over REQ-GEN-038's range, with two values `f32` rounds by nearly half a spacing, up and down, which the `f32`
/// decode reads narrowed.
fn mu_maxes() -> Vec<f64> {
    let range = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's allowed range");
    let half = 4.0 * f64::from(f32::EPSILON);
    vec![
        range.lo,
        range.default,
        0.5 * (range.lo + range.hi),
        range.hi,
        9.0 + 0.999 * half,
        9.0 + 1.001 * half,
    ]
}

fn check_every_pixel(how: Range) {
    let (mut charts, mut bites) = family::<Stick>(how, Target::Unread, &LinkParams::DEFAULT);
    for mu_max in mu_maxes() {
        let (c, b) = family::<Defaults>(how, Target::Unread, &mu(mu_max));
        charts += c;
        bites += b;
    }
    assert!(
        charts >= 500 && bites >= 100,
        "only {charts} charts, {bites} of them bitten by the narrowing"
    );
}

#[test]
fn qa_narrowed_every_pixel() {
    check_every_pixel(Range::Built);
}

negative_control!(
    qa_narrowed_every_pixel,
    "a range found at the held mass as held alone reaches ratios the narrowed mass does not realise, so it must fail",
    expected = "is refused on f32",
    check_every_pixel(Range::AsHeld)
);

#[test]
fn qa_narrowed_decode_softmax() {
    for mu_max in mu_maxes() {
        family::<Defaults>(Range::Built, Target::Narrowed, &mu(mu_max));
    }
}

#[test]
fn qa_narrowed_decode_stick() {
    family::<Stick>(Range::Built, Target::Narrowed, &LinkParams::DEFAULT);
}

// ── The searches' ends ───────────────────────────────────────────────────────────────────────────────────────────────

/// Runs `work` on a thread of its own, which ticks at each step; panics where no tick comes within `limit`, and passes a
/// panic of `work` on.
fn watched(limit: Duration, work: impl FnOnce(Sender<()>) + Send + 'static) {
    let (tick, ticks) = channel();
    let worker = thread::spawn(move || work(tick));
    loop {
        match ticks.recv_timeout(limit) {
            Ok(()) => {}
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => panic!("the search did not end within {limit:?}"),
        }
    }
    if let Err(panic) = worker.join() {
        resume_unwind(panic);
    }
}

/// The asks that bring the two searches to their ends.
fn hard_asks() -> Vec<[f64; 2]> {
    let up = |x: f64, n: u32| (0..n).fold(x, |x, _| x.next_up());
    let down = |x: f64, n: u32| (0..n).fold(x, |x, _| x.next_down());
    vec![
        [0.0, f64::MAX],
        [f64::MIN, f64::MAX],
        [f64::MIN_POSITIVE, f64::MAX],
        [5e-324, f64::MAX],
        [1.0e300, f64::MAX],
        [f64::MAX.next_down(), f64::MAX],
        [1.0e-300, 1.0e-299],
        [5e-324, 1.0e-320],
        [-2.0, -1.0],
        [-1.0, 0.0],
        [down(1.0, 1), 1.0],
        [1.0, up(1.0, 1)],
        [down(1.0, 1), up(1.0, 1)],
        [down(1.0, 2), up(1.0, 1)],
        [down(1.0, 9), up(1.0, 9)],
        [1.0, 1.0 + 1.0e-15],
        [1.0 - 1.0e-15, 1.0],
        [up(2.0, 1), up(2.0, 2)],
        [down(100.0, 3), 100.0],
    ]
}

/// One build on each hard ask at the link `S`: whatever it returns lies in the ask, with `q₀ < q₁`.
fn build_each<S: Selection>(tick: &Sender<()>, p: &LinkParams<f64>) {
    const CENTRES: [[f64; 2]; 9] = [
        [0.0, 0.0],
        [0.0, 3.0],
        [10.0, 0.0],
        [-20.0, 0.0],
        [0.0, 20.0],
        [-1.925, 10.0],
        [40.0, 40.0],
        [-1000.0, 1000.0],
        [1000.0, -1000.0],
    ];
    for z in CENTRES {
        for pair in PAIRS {
            for asked in hard_asks() {
                if let Some(chart) = warped::<S>(z, pair, asked, p) {
                    let [q0, q1] = chart.ratio;
                    assert!(
                        asked[0] <= q0 && q0 < q1 && q1 <= asked[1],
                        "z {z:?}, bodies {pair:?}, asked {asked:?}: the range [{q0}, {q1}]"
                    );
                }
                tick.send(()).expect("the watchdog");
            }
        }
    }
}

#[test]
fn qa_searches_end() {
    let range = tunables()
        .iter()
        .find(|t| t.entry == "softmax_tanh" && t.param == "mu_max")
        .expect("μ_max's allowed range");
    let (lo, default, hi) = (range.lo, range.default, range.hi);
    watched(Duration::from_secs(120), move |tick| {
        build_each::<Stick>(&tick, &LinkParams::DEFAULT);
        for mu_max in [lo, default, hi] {
            build_each::<Defaults>(&tick, &mu(mu_max));
        }
    });
}

/// `softmax_tanh` reaches `|ln q|` at most `2·μ_max`, so an ask out of reach is refused, which takes `seed` through every
/// step to its closing test where the centre is not realised (`z = (10, 0)` at `μ_max = 5`) and answers `None`.
#[test]
fn qa_searches_end_on_nothing_realised() {
    watched(Duration::from_secs(120), |tick| {
        let p = LinkParams::DEFAULT;
        for z in [[10.0, 0.0], [0.0, 20.0], [40.0, 40.0], [1000.0, -1000.0]] {
            for pair in PAIRS {
                for asked in [[1.0e300, f64::MAX], [f64::MIN_POSITIVE, 1.0e-300]] {
                    assert!(
                        warped::<Defaults>(z, pair, asked, &p).is_none(),
                        "z {z:?}, bodies {pair:?}: a chart over {asked:?}"
                    );
                    tick.send(()).expect("the watchdog");
                }
            }
        }
    });
}

/// A trisection whose closing test cannot fire: `<=` for `<`, so it holds at `lo = hi`, the bracket no longer moving.
fn never_closes(stop: &AtomicBool) {
    let (mut lo, hi) = (1.0_f64, 1.0_f64);
    while !stop.load(Ordering::Relaxed) {
        let third = (hi - lo) / 3.0;
        let near = lo + third;
        if !(lo <= near && hi - third <= hi) {
            return;
        }
        lo = near;
    }
}

negative_control!(
    qa_searches_end,
    "a search that does not end must fail the watchdog, or the test could not tell a hang from an end",
    expected = "did not end",
    {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        struct Stop(Arc<AtomicBool>);
        impl Drop for Stop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let _stop = Stop(stop);
        watched(Duration::from_secs(1), move |_tick| never_closes(&flag));
    }
);
