//! Navigation stops at the edge (R-469 P2, A1 to A3; REQ-CHART-060) and never leaves the manifold under fuzzed edits
//! (R-477 C6; REQ-CHART-061): [`kernel::chart::nav::nav_stop`], the one generic stop over the `Chart` trait and
//! `validate`, on the gallery's charts.
//!
//! **`nav_stop_at_edge_*`** (REQ-CHART-060): a pan on the invariant `(L_z, E)` chart under the linear option, from a
//! feasible centre toward `|L_z| > √(2I(E − U))`, stops at a centre that validates while the next f64 along the path
//! fails, the report naming layer 2 and the constraint; the mass simplex's centre moved toward a zero mass, and its one
//! physical slice, `ν₀`, toward a zero side, stop with every mass `> 0`; a held `K` sliced below 0 stops at `K₀ ≥ 0`
//! (R-720's reading); a tilt and a zoom with no centre edit pass unchanged; a lock on a forbidden pixel is refused; a
//! parameter edit whose boundary lies at `λ ≈ 10⁻⁶` stops at adjacent f64s within 62 steps; on the latent chart nothing
//! stops. Controls: a stop that keeps the requested value, a stop one bisection step short, and a per-chart override.
//!
//! **`nav_fuzz_never_leaves*`** (REQ-CHART-061): on every gallery chart a seeded sequence of pan, slice, zoom, tilt and
//! lock edits, each through the stop, leaves a centre that validates; on the invariant and Burrau `K` charts, warped,
//! every pixel of a 65 × 65 grid over `[0,1]²` whose own `(a, b)` lie in the square validates at every tilt reached,
//! every grid pixel under Lean each axis, with two targets or with one angle set, each angle within ±90° (R-512; past
//! 90°, Tip as sheet's range, a turned window's pixel can leave the square, and the in-square arm holds); under the
//! linear option infeasible pixels appear and each is classified not a system. Controls: the stop disabled, and the
//! linear arm run warped or read as a system.
//!
//! The draws are SplitMix64 from fixed seeds, each printed. `I = 1` throughout (the canonical-frame decode).

use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use kernel::chart::construction::{Held, LzChart, RestChart, HELD_K, NU0};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{
    Alpha, Beta, PLambdaX, PLambdaY, PRhoX, PRhoY, Quantity, ZAlpha, ZBeta, ZMu1, ZMu2, ZQ0, ZQ1,
    ZQ2, ZQ3,
};
use kernel::chart::nav::{nav_stop, Check, Edit, Field, Stop, View};
use kernel::chart::tilt::{latent_basis, Hidden, LatentFrame, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, Constraint, EuclidPlane, HeldK,
    HeldLz, InvariantLE, InvariantLK, Latent, Mapped, Mapping, MassSimplex, Mixed, RestStart,
    ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::decode::Degenerate;
use kernel::generated::links::{Defaults, LinkParams};
use validation::chart_class::forbidden;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

/// The link parameters every check here reads: the defaults.
const P: LinkParams<f64> = LinkParams::DEFAULT;
/// A view's centre off `(½, ½)`.
const C: [f64; 2] = [0.37, 0.61];
/// The targets' slider spans: the Slice centre slider's range, `[−1, 1]` (R-507 U1).
const SPAN: [f64; 2] = [2.0, 2.0];
/// R-469 A3's bound on the bisection: the f64s of `[0, 1]` number fewer than 2⁶².
const MAX_STEPS: u32 = 62;

// ── The independent check ─────────────────────────────────────────────────────────────────────────────────────────

/// What fails at a view's centre, read from the chart alone: `Φ` where there is no view or its output is not finite,
/// else `validate`'s constraint; none where the centre passes.
fn verdict<C: Chart<Param = f64>>(view: Option<View<C>>) -> Option<Check> {
    let Some(View {
        chart,
        centre: [u, v],
    }) = view
    else {
        return Some(Check::Phi);
    };
    if !chart.map::<f64>(u, v).z.z.iter().all(|x| x.is_finite()) {
        return Some(Check::Phi);
    }
    match chart.validate_params(u, v, &P) {
        ValidationResult::Valid => None,
        ValidationResult::Project { failed, .. }
        | ValidationResult::Clamp { failed, .. }
        | ValidationResult::Reject { failed } => Some(Check::Failed(failed)),
    }
}

/// REQ-CHART-060's stop of a path from `from` to `to`, checked against the chart alone: the value is on the path at
/// its `λ`; its centre validates; where it stopped, the next f64 above `λ` fails with the check reported, within 62
/// steps; where it did not, the value is the one asked for.
fn check_stop<V, C>(what: &str, from: V, to: V, view: impl Fn(V) -> Option<View<C>>, stop: &Stop<V>)
where
    V: Field + PartialEq + Debug,
    C: Chart<Param = f64>,
{
    assert!(
        stop.value == from.along(to, stop.lambda),
        "{what}: the stop {:?} is not on the edit's path at λ = {}",
        stop.value,
        stop.lambda
    );
    assert_eq!(
        verdict(view(stop.value)),
        None,
        "{what}: the stopped centre {:?} fails validate",
        stop.value
    );
    match stop.stopped {
        None => assert!(
            stop.value == to && stop.lambda == 1.0,
            "{what}: taken whole at λ = {}, not the value asked for",
            stop.lambda
        ),
        Some(check) => {
            let next = stop.lambda.next_up();
            assert_eq!(
                verdict(view(from.along(to, next))),
                Some(check),
                "{what}: λ = {} is not adjacent to a failing value naming {check:?}",
                stop.lambda
            );
            assert!(
                stop.steps <= MAX_STEPS,
                "{what}: {} bisection steps",
                stop.steps
            );
        }
    }
}

/// A stand-in stop for the controls: the generic bisection, ended one step short: before the last step that moved its
/// valid end, so the end kept is short of the one the full bisection reaches.
fn one_step_short<V, C>(from: V, to: V, view: impl Fn(V) -> Option<View<C>>) -> Stop<V>
where
    V: Field,
    C: Chart<Param = f64>,
{
    let mut ends = vec![(0u64, None)];
    let (mut lo, mut hi, mut failed) = (0u64, 1f64.to_bits(), None);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        match verdict(view(from.along(to, f64::from_bits(mid)))) {
            None => {
                ends.push((lo, failed));
                lo = mid;
            }
            Some(at) => {
                hi = mid;
                failed = Some(at);
            }
        }
    }
    let (lo, failed) = ends.pop().expect("a step that moved the valid end");
    let lambda = f64::from_bits(lo);
    Stop {
        value: from.along(to, lambda),
        lambda,
        steps: 0,
        stopped: failed.or(verdict(view(to))),
    }
}

// ── The charts ────────────────────────────────────────────────────────────────────────────────────────────────────

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

type LeMapped = Mapped<InvariantLE<f64>, Defaults>;
type LkMapped = Mapped<InvariantLK<f64>, Defaults>;

/// `(L_z, E)` under the linear option on both axes (R-471 E2, H4).
fn le_linear() -> LeMapped {
    Mapped::new(InvariantLE(invariant()), [Mapping::Linear; 2])
}

fn lk_linear() -> LkMapped {
    Mapped::new(InvariantLK(invariant()), [Mapping::Linear; 2])
}

fn simplex(nu0: f64) -> MassSimplex<f64> {
    MassSimplex {
        nu0,
        eps_m: MASS_SIMPLEX_EPS_M.number(),
    }
}

fn theta_range() -> [f64; 2] {
    [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()]
}

fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: theta_range(),
        k: BURRAU_THETA_LZ_K.number(),
        lz_fraction: BURRAU_THETA_LZ_FRACTION.number(),
    }
}

/// The view of `chart` about the centre `c`.
fn at<C>(chart: C, c: [f64; 2]) -> Option<View<C>> {
    Some(View { chart, centre: c })
}

// ── REQ-CHART-060: the linear option's pan ───────────────────────────────────────────────────────────────────────

/// The linear `(L_z, E)` chart's parabola at the row `t = ½`: `L_z = (2s − 1)·√(2·K_max)` against
/// `L_max = √(2·K_max·t)`, so the edge is `s = ½ + ½·√t`.
fn le_edge() -> f64 {
    0.5 + 0.5 * 0.5f64.sqrt()
}

/// The linear `(L_z, E)` chart's view about the centre `c`.
fn le_view(c: [f64; 2]) -> Option<View<LeMapped>> {
    at(le_linear(), c)
}

/// The stop under test of a path on the linear `(L_z, E)` chart.
fn le_stop(from: [f64; 2], to: [f64; 2]) -> Stop<[f64; 2]> {
    nav_stop(Edit::Path { from, to }, le_view, &P)
}

/// A pan from `(½, ½)` toward `s = 1` on the linear `(L_z, E)` chart, taken by `take`, and toward `s = 0` on the linear
/// `(L_z, K)` one.
fn check_linear_pan(take: fn([f64; 2], [f64; 2]) -> Stop<[f64; 2]>) {
    let le = le_view;
    let (from, to) = ([0.5, 0.5], [1.0, 0.5]);
    let stop = take(from, to);
    check_stop("(L_z, E) linear pan", from, to, le, &stop);
    let check = stop.stopped.expect("the pan crosses the parabola");
    assert_eq!(check, Check::Failed(Constraint::LzEParabola));
    assert_eq!(check.layer(), Some(2), "the report names layer 2");
    assert_eq!(stop.value[1], 0.5, "a pan along s keeps t");
    assert!(
        (stop.value[0] - le_edge()).abs() < 1e-12,
        "the stop at s = {} is not the parabola's edge {}",
        stop.value[0],
        le_edge()
    );
    println!(
        "(L_z, E) linear: stopped at s = {:?} (λ = {:?}) after {} steps, {check:?}, layer {:?}",
        stop.value[0],
        stop.lambda,
        stop.steps,
        check.layer()
    );
    let lk = |c: [f64; 2]| at(lk_linear(), c);
    let to = [0.0, 0.5];
    let stop = nav_stop(Edit::Path { from, to }, lk, &P);
    check_stop("(L_z, K) linear pan", from, to, lk, &stop);
    assert_eq!(stop.stopped, Some(Check::Failed(Constraint::KAboveLzBound)));
    assert!((stop.value[0] - (1.0 - le_edge())).abs() < 1e-12);
}

#[test]
fn nav_stop_at_edge_linear_pan() {
    check_linear_pan(le_stop);
}

negative_control!(
    nav_stop_at_edge_linear_pan,
    "a stop that keeps the requested value leaves a centre that fails validate",
    expected = "the stopped centre",
    check_linear_pan(|_, to| Stop {
        value: to,
        lambda: 1.0,
        steps: 0,
        stopped: None
    })
);

mod short {
    use super::*;
    negative_control!(
        nav_stop_at_edge_linear_pan,
        "a stop one bisection step short is not adjacent to the failing value",
        expected = "is not adjacent to a failing value",
        check_linear_pan(|from, to| one_step_short(from, to, le_view))
    );
}

// ── REQ-CHART-060: the mass simplex and the held K ───────────────────────────────────────────────────────────────

/// The decoded masses at a view's centre.
fn masses<C: Chart<Param = f64>>(view: &View<C>) -> [f64; 3] {
    let [u, v] = view.centre;
    view.chart.decode::<Defaults, f64>(u, v).ic.m
}

/// `stop`, or with `kept` the requested value `to` in its place (a control's).
fn keep<V>(stop: Stop<V>, to: V, kept: bool) -> Stop<V> {
    if kept {
        Stop { value: to, ..stop }
    } else {
        stop
    }
}

/// The mass simplex (R-633: its hidden coordinates `ν₀` and the momentum controls): its centre moved toward each zero
/// mass, and `ν₀` sliced toward each zero side, `ν₀ ∉ (0, 1)`. Each stops with every decoded mass `> 0`, the report
/// layer 2's. `kept` keeps the requested values (the control).
fn check_simplex(kept: bool) {
    let chart = RestStart::new(simplex(0.5));
    let pan = |c: [f64; 2]| at(chart, c);
    for to in [[-0.4, C[1]], [1.3, C[1]], [C[0], 1.4]] {
        let stop = keep(nav_stop(Edit::Path { from: C, to }, pan, &P), to, kept);
        let view = pan(stop.value).unwrap();
        let m = masses(&view);
        assert!(
            m.iter().all(|&m| m > 0.0),
            "a move toward a zero mass stops at masses {m:?}"
        );
        check_stop("the mass simplex's centre", C, to, pan, &stop);
        assert_eq!(stop.stopped, Some(Check::Failed(Constraint::MassBuffer)));
    }
    let slice = |nu: f64| chart.sliced(NU0, nu).and_then(|c| at(c, C));
    for to in [-0.5, 1.5] {
        let stop = keep(nav_stop(Edit::Path { from: 0.5, to }, slice, &P), to, kept);
        let view = slice(stop.value).unwrap();
        let m = masses(&view);
        assert!(
            stop.value > 0.0 && stop.value < 1.0 && m.iter().all(|&m| m > 0.0),
            "a slice of ν₀ toward {to} stops at ν₀ = {} with masses {m:?}",
            stop.value
        );
        check_stop("the mass simplex's ν₀", 0.5, to, slice, &stop);
        assert_eq!(stop.stopped, Some(Check::Failed(Constraint::BurrauNu)));
        println!(
            "mass simplex: ν₀ toward {to} stopped at {:?} after {} steps",
            stop.value, stop.steps
        );
    }
}

#[test]
fn nav_stop_at_edge_mass_simplex() {
    check_simplex(false);
}

negative_control!(
    nav_stop_at_edge_mass_simplex,
    "the requested values kept leave a zero mass or a zero side",
    expected = "stops at",
    check_simplex(true)
);

/// The held `K` of `Φ_θ,L_z` sliced toward `K < 0`, bare and held in either mode: the stop holds the centre at
/// `K₀ ≥ 0` (R-720, applied per R-369: "REQ-CHART-060's stop holds the centre at `K₀ ≥ 0`"). `kept` keeps the requested
/// value (the control).
fn check_held_k(kept: bool) {
    let bare = theta_lz();
    let held = HeldK::new(bare, K_MAX.number(), GAMMA_K.number());
    let linear = HeldK {
        mapping: Mapping::Linear,
        ..held
    };
    check_held_k_on("bare", bare, kept);
    check_held_k_on("warped", held, kept);
    check_held_k_on("linear", linear, kept);
}

/// [`check_held_k`] on one chart holding `K`.
fn check_held_k_on<H: Hidden<Param = f64> + Copy>(name: &str, chart: H, kept: bool) {
    let k0 = theta_lz().k;
    let to = -0.5;
    let view = |k: f64| chart.sliced(HELD_K, k).and_then(|c| at(c, C));
    let stop = keep(nav_stop(Edit::Path { from: k0, to }, view, &P), to, kept);
    assert!(
        stop.value >= 0.0 && stop.stopped.is_some(),
        "the {name} held K sliced toward {to} stops at K₀ = {}, {:?}",
        stop.value,
        stop.stopped
    );
    check_stop(name, k0, to, view, &stop);
    println!(
        "held K, {name}: stopped at K₀ = {:?} after {} steps, {:?}",
        stop.value, stop.steps, stop.stopped
    );
}

#[test]
fn nav_stop_at_edge_held_k() {
    check_held_k(false);
}

negative_control!(
    nav_stop_at_edge_held_k,
    "a held K slice that keeps the requested value leaves K₀ < 0",
    expected = "stops at K₀",
    check_held_k(true)
);

// ── REQ-CHART-060: tilt and zoom, the lock ────────────────────────────────────────────────────────────────────────

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

/// A tilt and a zoom with no centre edit, from a centre on the linear `(L_z, E)` chart's edge: each passes unchanged,
/// taken whole with no step. Tilts under both maps, each in its range, with one target and two, into a mass logit and
/// a configuration control; a zoom on the latent chart in and out by 10⁶. `with_centre` moves the centre with the tilt,
/// past the edge (the control).
fn check_tilt_zoom(with_centre: bool) {
    let edge = nav_stop(
        Edit::Path {
            from: [0.5, 0.5],
            to: [1.0, 0.5],
        },
        |c| at(le_linear(), c),
        &P,
    )
    .value;
    for (map, x, y) in [
        (TiltMap::Sheet, 6, 6),
        (TiltMap::Lean, 6, 6),
        (TiltMap::Sheet, 6, 0),
    ] {
        let view = |w: [f64; 4]| {
            let tilt = Tilt::new(&spec(w[0], w[1], map, x, y), [w[2], w[3]], SPAN);
            Tilted::new(le_linear(), tilt).and_then(|t| at(t, [w[2], w[3]]))
        };
        let from = [0.0, 0.0, edge[0], edge[1]];
        let shift = if with_centre { 0.1 } else { 0.0 };
        let far = if map == TiltMap::Lean { -80.0 } else { -150.0 };
        for to in [
            [30.0, 20.0, edge[0] + shift, edge[1]],
            [far, 75.0, edge[0] + shift, edge[1]],
        ] {
            let stop = nav_stop(Edit::Path { from, to }, view, &P);
            check_stop("a tilt", from, to, view, &stop);
            assert!(
                stop.stopped.is_none() && stop.value == to && stop.steps == 0,
                "a tilt to {to:?} under {map:?} into ({x}, {y}) does not pass unchanged: {stop:?}"
            );
        }
    }
    let frame = |scale: f64| LatentFrame {
        a: 2,
        b: 3,
        scale,
        gamma: 0.0,
    };
    let z0 = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];
    let latent = |scale: f64| {
        let (q1, q2) = latent_basis(&frame(scale), &spec(0.0, 0.0, TiltMap::Sheet, 6, 6))?;
        at(Latent { z0, q1, q2 }, [0.5, 0.5])
    };
    for to in [0.5e-6, 0.5e6] {
        let stop = nav_stop(Edit::Path { from: 0.5, to }, latent, &P);
        check_stop("a zoom", 0.5, to, latent, &stop);
        assert!(stop.stopped.is_none() && stop.value == to && stop.steps == 0);
    }
}

#[test]
fn nav_stop_at_edge_tilt_zoom() {
    check_tilt_zoom(false);
}

negative_control!(
    nav_stop_at_edge_tilt_zoom,
    "a tilt that comes with a centre edit past the edge stops",
    expected = "does not pass unchanged",
    check_tilt_zoom(true)
);

/// A lock on the linear `(L_z, E)` chart: on a forbidden pixel, past the parabola and below `t = 0`, refused with the
/// centre unchanged and the check named; on a pixel that is a system, taken. `as_path` passes the lock as a pan (the
/// control). And an edit from an accepted value that itself fails: taken where the value asked for passes, refused
/// where it fails.
fn check_lock(as_path: bool) {
    let le = |c: [f64; 2]| at(le_linear(), c);
    let edit = |from, to| {
        if as_path {
            Edit::Path { from, to }
        } else {
            Edit::Lock { from, to }
        }
    };
    for to in [[0.95, 0.5], [0.5, -0.2]] {
        let stop = nav_stop(edit(C, to), le, &P);
        assert!(
            stop.value == C && stop.lambda == 0.0 && stop.steps == 0,
            "a lock on the forbidden pixel {to:?} is not refused: {stop:?}"
        );
        assert_eq!(stop.stopped, verdict(le(to)));
        assert!(stop.stopped.is_some());
    }
    let to = [0.6, 0.8];
    let stop = nav_stop(edit(C, to), le, &P);
    assert_eq!(
        stop,
        Stop {
            value: to,
            lambda: 1.0,
            steps: 0,
            stopped: None
        }
    );
    let outside = [0.95, 0.5];
    let stop = nav_stop(Edit::Path { from: outside, to }, le, &P);
    assert!(stop.value == to && stop.stopped.is_none(), "{stop:?}");
    let stop = nav_stop(
        Edit::Path {
            from: outside,
            to: [0.97, 0.5],
        },
        le,
        &P,
    );
    assert!(
        stop.value == outside && stop.lambda == 0.0 && stop.steps == 0,
        "an edit from a failing value to a failing one is not refused: {stop:?}"
    );
    assert_eq!(stop.stopped, verdict(le([0.97, 0.5])));
}

#[test]
fn nav_stop_at_edge_lock() {
    check_lock(false);
}

negative_control!(
    nav_stop_at_edge_lock,
    "a lock passed as a path is stopped short of the pixel, not refused",
    expected = "is not refused",
    check_lock(true)
);

// ── REQ-CHART-060: a boundary at λ ≈ 10⁻⁶ ──────────────────────────────────────────────────────────────────────────

/// A parameter edit, the mass simplex's `ν₀` from `10⁻⁶` to `10⁻⁶ − 1`, whose boundary `ν₀ = 0` lies at `λ ≈ 10⁻⁶`:
/// it stops at adjacent f64s within 62 steps (R-469 A3 as corrected: halving `λ` itself never gets within 64 there).
/// `short` takes the stop one bisection step short (the control).
fn check_tiny(short: bool) {
    let (from, to) = (1e-6, 1e-6 - 1.0);
    let view = |nu: f64| at(simplex(nu), C);
    let stop = if short {
        one_step_short(from, to, view)
    } else {
        nav_stop(Edit::Path { from, to }, view, &P)
    };
    check_stop("ν₀ near 0", from, to, view, &stop);
    assert!(
        (stop.lambda - 1e-6).abs() < 1e-15,
        "the boundary at λ = {}",
        stop.lambda
    );
    assert_eq!(stop.stopped, Some(Check::Failed(Constraint::BurrauNu)));
    println!(
        "ν₀ from 1e-6: stopped at ν₀ = {:?}, λ = {:?}, after {} steps",
        stop.value, stop.lambda, stop.steps
    );
}

#[test]
fn nav_stop_at_edge_tiny_lambda() {
    check_tiny(false);
}

negative_control!(
    nav_stop_at_edge_tiny_lambda,
    "a stop one bisection step short of a boundary at λ ≈ 1e-6 is not adjacent",
    expected = "is not adjacent to a failing value",
    check_tiny(true)
);

// ── REQ-CHART-060: the latent chart, and Φ ─────────────────────────────────────────────────────────────────────────

/// On the latent chart (the decoder accepts any `z`; R-570) nothing stops: pans and slices of `z₀` to `|z| = 10⁴` and a
/// lock far out are taken whole. `simplex` runs the same pans on the mass simplex, which stops (the control). And a
/// centre whose `Φ` fails stops: a pan toward a non-finite `z₀`, and a view that cannot be built past a value.
fn check_latent(on_simplex: bool) {
    let z0 = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];
    let (q1, q2) = latent_basis(
        &LatentFrame {
            a: 2,
            b: 3,
            scale: 0.5,
            gamma: 0.3,
        },
        &spec(25.0, -40.0, TiltMap::Sheet, 6, 6),
    )
    .unwrap();
    let latent = |z0: [f64; 8]| at(Latent { z0, q1, q2 }, [0.5, 0.5]);
    let far = |s: f64| z0.map(|z| z + s * 1e4);
    for to in [
        far(1.0),
        far(-1.0),
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1e4, 1e4],
    ] {
        let stop = if on_simplex {
            let simplex = |c: [f64; 2]| at(simplex(0.5), c);
            let s = nav_stop(
                Edit::Path {
                    from: C,
                    to: [to[0], to[1]],
                },
                simplex,
                &P,
            );
            Stop {
                value: to,
                lambda: s.lambda,
                steps: s.steps,
                stopped: s.stopped,
            }
        } else {
            nav_stop(Edit::Path { from: z0, to }, latent, &P)
        };
        assert!(
            stop.stopped.is_none(),
            "on the latent chart nothing stops, but a pan to {to:?} stopped: {:?}",
            stop.stopped
        );
        check_stop("a latent pan", z0, to, latent, &stop);
        let lock = nav_stop(Edit::Lock { from: z0, to }, latent, &P);
        assert!(lock.value == to && lock.stopped.is_none());
    }
    let mut inf = z0;
    inf[0] = f64::INFINITY;
    let stop = nav_stop(Edit::Path { from: z0, to: inf }, latent, &P);
    assert!(
        stop.value == z0 && stop.stopped == Some(Check::Phi),
        "{stop:?}"
    );
    assert_eq!(Check::Phi.layer(), None, "Φ names no layer");
    check_stop("a pan toward a non-finite z₀", z0, inf, latent, &stop);
    let partial = |x: f64| (x <= 0.3).then(|| at(simplex(0.5), [x, C[1]])).flatten();
    let stop = nav_stop(Edit::Path { from: 0.1, to: 0.9 }, partial, &P);
    check_stop("a view that cannot be built", 0.1, 0.9, partial, &stop);
    assert!(stop.value <= 0.3 && stop.value > 0.3 - 1e-15, "{stop:?}");
    assert_eq!(stop.stopped, Some(Check::Phi));
}

#[test]
fn nav_stop_at_edge_latent() {
    check_latent(false);
}

negative_control!(
    nav_stop_at_edge_latent,
    "the same pans on the mass simplex stop, so the test tells the two kinds of chart apart",
    expected = "on the latent chart nothing stops",
    check_latent(true)
);

// ── REQ-CHART-060: one rule, and the report's layers ──────────────────────────────────────────────────────────────

/// A chart's own remedy in place of the generic stop, registered for one chart by name: `validate`'s projection or
/// clamp of the requested centre, which a chart writing its own rule would take.
fn own_remedy<C: Chart<Param = f64>>(chart: &C, to: [f64; 2]) -> Stop<[f64; 2]> {
    let (value, failed) = match chart.validate(to[0], to[1]) {
        ValidationResult::Project { u, v, failed }
        | ValidationResult::Clamp { u, v, failed, .. } => ([u, v], failed),
        ValidationResult::Reject { failed } => (to, failed),
        ValidationResult::Valid => {
            return Stop {
                value: to,
                lambda: 1.0,
                steps: 0,
                stopped: None,
            }
        }
    };
    Stop {
        value,
        lambda: 1.0,
        steps: 0,
        stopped: Some(Check::Failed(failed)),
    }
}

/// The stop of a centre edit on `chart`, through the registry: the generic stop, or the override registered for the
/// chart's name.
fn stop_for<C: Chart<Param = f64> + Copy>(
    chart: C,
    from: [f64; 2],
    to: [f64; 2],
    overrides: &[&str],
) -> Stop<[f64; 2]> {
    if overrides.contains(&chart.name()) {
        own_remedy(&chart, to)
    } else {
        nav_stop(Edit::Path { from, to }, |c| at(chart, c), &P)
    }
}

/// Every chart's centre edit across its edge is the one rule's: on the stop's path at its `λ`, adjacent to the
/// failing value. A chart whose own `validate` would project or clamp the point (the mass simplex, the linear energy
/// charts, the Burrau family's bounded axes) still stops on the path, not at its remedy. `overrides` registers a
/// chart's own rule by name (the control).
fn check_one_rule(overrides: &[&str]) {
    let to = [1.5, 1.5];
    macro_rules! case {
        ($chart:expr) => {{
            let chart = $chart;
            let stop = stop_for(chart, C, to, overrides);
            check_stop(chart.name(), C, to, |c| at(chart, c), &stop);
            assert!(stop.stopped.is_some(), "{} does not stop", chart.name());
        }};
    }
    case!(le_linear());
    case!(lk_linear());
    case!(simplex(0.5));
    case!(EuclidPlane {
        m_range: [EUCLID_M_MIN.number(), EUCLID_M_MAX.number()],
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
    });
    case!(BurrauThetaK {
        theta_range: theta_range(),
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    });
    case!(Mixed::<Alpha, PRhoX, f64>::new(
        [0.0; 8],
        [0.4, 1.2],
        [-0.5, 0.5]
    ));
}

#[test]
fn nav_stop_at_edge_one_rule() {
    check_one_rule(&[]);
}

negative_control!(
    nav_stop_at_edge_one_rule,
    "a per-chart override registered for the mass simplex stops at its own clamp, off the edit's path",
    expected = "is not on the edit's path",
    check_one_rule(&["MassSimplex"])
);

/// The report's layer: 3 for decode sanity, 2 for every other constraint, none for `Φ`. `swap` reads `ComAtOrigin` as
/// layer 2 (the control).
fn check_layers(swap: bool) {
    use Constraint::*;
    let layer3 = [DecodedMassPositive, ComAtOrigin, MomentumZero];
    let layer2 = [
        LzEParabola,
        KNonNegative,
        KAboveLzBound,
        MassPositive,
        MassSum,
        MassBuffer,
        ShapeTheta,
        ShapePhi,
        BurrauNu,
        AxisDomain,
        BurrauTheta,
    ];
    for c in layer3 {
        let want = if swap && c == ComAtOrigin { 2 } else { 3 };
        assert_eq!(Check::Failed(c).layer(), Some(want), "{c:?}'s layer");
    }
    for c in layer2 {
        assert_eq!(Check::Failed(c).layer(), Some(2), "{c:?}'s layer");
    }
    assert_eq!(Check::Phi.layer(), None);
}

#[test]
fn nav_stop_at_edge_layers() {
    check_layers(false);
}

negative_control!(
    nav_stop_at_edge_layers,
    "a decode-sanity constraint read as layer 2 fails",
    expected = "ComAtOrigin's layer",
    check_layers(true)
);

// ── REQ-CHART-061: the fuzz ───────────────────────────────────────────────────────────────────────────────────────

/// The seeded draws: SplitMix64 from a fixed seed, uniform on `[−1, 1]`.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }

    /// An index below `n`.
    fn index(&mut self, n: usize) -> usize {
        (((self.next() + 1.0) * 0.5 * n as f64) as usize).min(n - 1)
    }
}

/// The fuzz's seed.
const SEED: u64 = 0x5EED_0C06;
/// The edits per gallery chart: R-477 C6's "≥ 1000".
const EDITS: u32 = 1000;
/// The grid side of C6's every-pixel arm: 65 × 65 over `[0,1]²`, `i/64` for `i ∈ 0..=64`.
const GRID: u32 = 64;

/// A navigation edit's kind.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Pan,
    Slice,
    Zoom,
    Tilt,
    Lock,
}

/// The kinds in proportion: pan and lock the most, as they are what reaches an edge.
const KINDS: [Kind; 10] = [
    Kind::Pan,
    Kind::Pan,
    Kind::Pan,
    Kind::Slice,
    Kind::Slice,
    Kind::Zoom,
    Kind::Tilt,
    Kind::Lock,
    Kind::Lock,
    Kind::Lock,
];

/// What C6's grid checks at each tilt reached.
#[derive(Clone, Copy, PartialEq)]
enum Grid {
    None,
    /// Every pixel whose `(a, b)` lie in the square validates; every pixel under Lean, two targets or one angle, each
    /// angle within ±90°, where `(a, b)` lie between the pixel and the centre. Past 90° (Tip as sheet's range) a single
    /// turn, or each of two, takes the window over, and a pixel can leave the square (R-512).
    Warped,
    /// The infeasible pixels, each classified not a system; with `as_system` each read as a system (a control).
    Linear {
        as_system: bool,
    },
}

/// One fuzz run's tally.
#[derive(Clone, Debug, Default)]
struct Tally {
    edits: u32,
    stopped: u32,
    refused: u32,
    tilts: u32,
    grid_checked: u64,
    infeasible: u64,
    faults: Vec<String>,
}

impl Tally {
    fn fault(&mut self, s: String) {
        if self.faults.len() < 8 {
            self.faults.push(s);
        }
    }

    fn count<V>(&mut self, stop: &Stop<V>, lock: bool) {
        self.edits += 1;
        if stop.stopped.is_some() {
            if lock || stop.lambda == 0.0 {
                self.refused += 1;
            } else {
                self.stopped += 1;
            }
        }
    }
}

/// The stop under test, or with `off` the requested value taken (the control's disabled stop).
fn stop_or_take<V: Field, C: Chart<Param = f64>>(
    edit: Edit<V>,
    view: impl Fn(V) -> Option<View<C>>,
    off: bool,
) -> Stop<V> {
    let stop = nav_stop(edit, view, &P);
    match (off, edit) {
        (true, Edit::Path { to, .. } | Edit::Lock { to, .. }) => Stop {
            value: to,
            lambda: 1.0,
            steps: 0,
            stopped: None,
        },
        (false, _) => stop,
    }
}

/// A curved gallery chart's hidden coordinates' values at the slice centre.
trait Nav: Tiltable<Param = f64> + Copy + Send + Sync + 'static {
    fn value(&self, k: u32) -> f64;
}

impl Nav for ShapeSphere<f64> {
    fn value(&self, k: u32) -> f64 {
        match k {
            2..=5 => self.z_mom[(k - 2) as usize],
            _ => self.z_mass[(k - 6) as usize],
        }
    }
}

fn invariant_value(inv: &Invariant<f64>, k: u32) -> f64 {
    match k {
        0 | 1 => inv.z_cfg[k as usize],
        _ => inv.z_mass[(k - 6) as usize],
    }
}

impl Nav for InvariantLE<f64> {
    fn value(&self, k: u32) -> f64 {
        invariant_value(&self.0, k)
    }
}

impl Nav for InvariantLK<f64> {
    fn value(&self, k: u32) -> f64 {
        invariant_value(&self.0, k)
    }
}

impl Nav for LeMapped {
    fn value(&self, k: u32) -> f64 {
        self.chart.value(k)
    }
}

impl Nav for LkMapped {
    fn value(&self, k: u32) -> f64 {
        self.chart.value(k)
    }
}

impl<R> Nav for RestStart<R, f64>
where
    R: RestChart<Param = f64> + Held<f64> + Send + Sync + 'static,
{
    fn value(&self, k: u32) -> f64 {
        self.held(k).expect("a hidden coordinate")
    }
}

impl<R> Nav for HeldLz<R, f64>
where
    R: LzChart<Param = f64> + Send + Sync + 'static,
{
    fn value(&self, k: u32) -> f64 {
        self.held(k).expect("a hidden coordinate")
    }
}

impl Nav for HeldK<f64> {
    fn value(&self, k: u32) -> f64 {
        self.held(k).expect("a hidden coordinate")
    }
}

impl<H: Quantity + Send + Sync + 'static, V: Quantity + Send + Sync + 'static> Nav
    for Mixed<H, V, f64>
{
    fn value(&self, k: u32) -> f64 {
        self.frozen[k as usize]
    }
}

/// The tilt maps and targets a run takes: one target under each map, and two targets where the chart has two hidden
/// coordinates.
fn configs(hidden: &[u32]) -> Vec<(TiltMap, u32, u32)> {
    let mut out = vec![
        (TiltMap::Sheet, hidden[0], hidden[0]),
        (
            TiltMap::Lean,
            hidden[hidden.len() - 1],
            hidden[hidden.len() - 1],
        ),
    ];
    if hidden.len() > 1 {
        out.push((TiltMap::Sheet, hidden[0], hidden[hidden.len() - 1]));
    }
    out
}

/// A curved chart's view state: the chart at its slice centre, the view's centre in its own parameters, its tilt and
/// the window's width.
struct Curved<C> {
    chart: C,
    centre: [f64; 2],
    spec: TiltSpec,
    width: f64,
}

impl<C: Nav> Curved<C> {
    fn tilted(&self, chart: C, spec: &TiltSpec, centre: [f64; 2]) -> Option<View<Tilted<C, f64>>> {
        Tilted::new(chart, Tilt::new(spec, centre, SPAN)).and_then(|t| at(t, centre))
    }

    fn view(&self) -> Option<View<Tilted<C, f64>>> {
        self.tilted(self.chart, &self.spec, self.centre)
    }
}

/// C6's grid at the current view: the warped arm's every pixel, or the linear arm's infeasible ones.
fn check_grid<C: Nav>(state: &Curved<C>, grid: Grid, t: &mut Tally, at_edit: u32) {
    let view = state.view().expect("the view").chart;
    let s = state.spec;
    let within = s.tau1.abs() <= 90.0 && s.tau2.abs() <= 90.0;
    let every = within && (s.map == TiltMap::Lean || s.x != s.y || s.tau1 == 0.0 || s.tau2 == 0.0);
    for j in 0..=GRID {
        for i in 0..=GRID {
            let (u, v) = (
                f64::from(i) / f64::from(GRID),
                f64::from(j) / f64::from(GRID),
            );
            let ([a, b], _) = view.at(u, v);
            let square = (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b);
            match grid {
                Grid::Warped if every || square => {
                    t.grid_checked += 1;
                    let r = view.validate(u, v);
                    if r != ValidationResult::Valid {
                        t.fault(format!(
                            "edit {at_edit}: the warped pixel ({u}, {v}) at (a, b) = ({a}, {b}) under {s:?} about {:?} is {r:?}",
                            state.centre
                        ));
                    }
                }
                Grid::Linear { as_system } => {
                    t.grid_checked += 1;
                    let d = view.decode::<Defaults, f64>(u, v);
                    if d.degenerate == Some(Degenerate::InvariantInfeasible) {
                        t.infeasible += 1;
                        let blank = !as_system && forbidden(&view, u, v) != Some(false);
                        if !blank {
                            t.fault(format!(
                                "edit {at_edit}: an infeasible pixel ({u}, {v}) under {s:?} is classified a system"
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// A seeded fuzz of `edits` navigation edits on a curved chart under the tilt `config`, each through the stop (or, with
/// `off`, taken as asked): after each, the view's centre validates; after each tilt, C6's grid is checked.
fn fuzz_curved<C: Nav>(
    chart: C,
    config: (TiltMap, u32, u32),
    seed: u64,
    edits: u32,
    off: bool,
    grid: Grid,
) -> Tally {
    let hidden: Vec<u32> = (0..16).filter(|&k| chart.hidden(k)).collect();
    let (map, x, y) = config;
    let mut state = Curved {
        chart,
        centre: [0.5, 0.5],
        spec: spec(0.0, 0.0, map, x, y),
        width: 1.0,
    };
    let mut values = [0.0; 16];
    for &k in &hidden {
        values[k as usize] = chart.value(k);
    }
    let mut d = Draw(seed);
    let mut t = Tally::default();
    if verdict(state.view()).is_some() {
        t.fault(format!(
            "the starting centre fails: {:?}",
            verdict(state.view())
        ));
        return t;
    }
    if grid != Grid::None {
        check_grid(&state, grid, &mut t, 0);
    }
    for n in 1..=edits {
        let kind = KINDS[d.index(KINDS.len())];
        let draws = [d.next(), d.next(), d.next()];
        let k = hidden[d.index(hidden.len())];
        match kind {
            Kind::Pan => {
                let from = state.centre;
                let to = [
                    from[0] + 0.75 * state.width * draws[0],
                    from[1] + 0.75 * state.width * draws[1],
                ];
                let stop = stop_or_take(
                    Edit::Path { from, to },
                    |c| state.tilted(state.chart, &state.spec, c),
                    off,
                );
                t.count(&stop, false);
                state.centre = stop.value;
            }
            Kind::Slice => {
                let from = values[k as usize];
                let to = from + 1.5 * draws[0];
                let view = |x: f64| {
                    state
                        .chart
                        .sliced(k, x)
                        .and_then(|c| state.tilted(c, &state.spec, state.centre))
                };
                let stop = stop_or_take(Edit::Path { from, to }, view, off);
                t.count(&stop, false);
                values[k as usize] = stop.value;
                state.chart = state
                    .chart
                    .sliced(k, stop.value)
                    .expect("a hidden coordinate");
            }
            Kind::Zoom => {
                let from = state.width;
                let to = from * 2f64.powf(2.0 * draws[0]);
                let view = |_w: f64| state.view();
                let stop = stop_or_take(Edit::Path { from, to }, view, off);
                t.count(&stop, false);
                state.width = stop.value;
            }
            Kind::Tilt => {
                let range = if map == TiltMap::Lean { 90.0 } else { 180.0 };
                let one = draws[2];
                let to = [
                    if one < -0.5 { 0.0 } else { range * draws[0] },
                    if one > 0.5 { 0.0 } else { range * draws[1] },
                ];
                let from = [state.spec.tau1, state.spec.tau2];
                let view = |a: [f64; 2]| {
                    state.tilted(state.chart, &spec(a[0], a[1], map, x, y), state.centre)
                };
                let stop = stop_or_take(Edit::Path { from, to }, view, off);
                t.count(&stop, false);
                t.tilts += 1;
                state.spec = spec(stop.value[0], stop.value[1], map, x, y);
            }
            Kind::Lock => {
                let pixel = [
                    state.centre[0] + 0.5 * state.width * draws[0],
                    state.centre[1] + 0.5 * state.width * draws[1],
                ];
                let (to, _) = state.view().expect("the view").chart.at(pixel[0], pixel[1]);
                let from = state.centre;
                let stop = stop_or_take(
                    Edit::Lock { from, to },
                    |c| state.tilted(state.chart, &state.spec, c),
                    off,
                );
                t.count(&stop, true);
                state.centre = stop.value;
            }
        }
        if let Some(check) = verdict(state.view()) {
            t.fault(format!(
                "edit {n} ({kind:?}) leaves the manifold: the centre {:?} under {:?} fails {check:?}",
                state.centre, state.spec
            ));
            return t;
        }
        if kind == Kind::Tilt && grid != Grid::None {
            check_grid(&state, grid, &mut t, n);
        }
    }
    t
}

/// A seeded fuzz on a latent frame (`Latent` with `(q₁, q₂)` from the frame and the tilt): pan and lock move `z₀` in
/// the plane, a slice moves one hidden component, a zoom the frame's scale, a tilt the basis. Nothing stops (R-570),
/// and the centre validates after each edit.
fn fuzz_latent(frame: LatentFrame, z0: [f64; 8], seed: u64, edits: u32) -> Tally {
    let hidden: Vec<u32> = (0..8).filter(|&k| frame.hidden(k)).collect();
    let (x, y) = (hidden[0], hidden[hidden.len() - 1]);
    let mut d = Draw(seed);
    let mut t = Tally::default();
    let (mut frame, mut z0, mut s) = (frame, z0, spec(0.0, 0.0, TiltMap::Sheet, x, y));
    let make = |frame: &LatentFrame, s: &TiltSpec, z0: [f64; 8]| {
        let (q1, q2) = latent_basis(frame, s)?;
        at(Latent { z0, q1, q2 }, [0.5, 0.5])
    };
    for n in 1..=edits {
        let kind = KINDS[d.index(KINDS.len())];
        let draws = [d.next(), d.next(), d.next()];
        let k = hidden[d.index(hidden.len())];
        let chart = make(&frame, &s, z0).expect("the view").chart;
        let stopped = match kind {
            Kind::Pan | Kind::Lock => {
                let to = chart
                    .map::<f64>(0.5 + 0.75 * draws[0], 0.5 + 0.75 * draws[1])
                    .z
                    .z;
                let edit = if matches!(kind, Kind::Pan) {
                    Edit::Path { from: z0, to }
                } else {
                    Edit::Lock { from: z0, to }
                };
                let stop = nav_stop(edit, |z| make(&frame, &s, z), &P);
                z0 = stop.value;
                stop.stopped
            }
            Kind::Slice => {
                let mut to = z0;
                to[k as usize] += 2.0 * draws[0];
                let stop = nav_stop(Edit::Path { from: z0, to }, |z| make(&frame, &s, z), &P);
                z0 = stop.value;
                stop.stopped
            }
            Kind::Zoom => {
                let to = frame.scale * 2f64.powf(2.0 * draws[0]);
                let view = |scale: f64| make(&LatentFrame { scale, ..frame }, &s, z0);
                let stop = nav_stop(
                    Edit::Path {
                        from: frame.scale,
                        to,
                    },
                    view,
                    &P,
                );
                frame.scale = stop.value;
                stop.stopped
            }
            Kind::Tilt => {
                let to = [
                    180.0 * draws[0],
                    if draws[2] > 0.5 {
                        0.0
                    } else {
                        180.0 * draws[1]
                    },
                ];
                let view = |a: [f64; 2]| make(&frame, &spec(a[0], a[1], TiltMap::Sheet, x, y), z0);
                let stop = nav_stop(
                    Edit::Path {
                        from: [s.tau1, s.tau2],
                        to,
                    },
                    view,
                    &P,
                );
                s = spec(stop.value[0], stop.value[1], TiltMap::Sheet, x, y);
                stop.stopped
            }
        };
        t.edits += 1;
        if let Some(check) = stopped.or_else(|| verdict(make(&frame, &s, z0))) {
            t.fault(format!(
                "latent edit {n} ({kind:?}) stopped or left: {check:?}"
            ));
            return t;
        }
    }
    t
}

// ── The gallery ───────────────────────────────────────────────────────────────────────────────────────────────────

/// One run: a name and the work.
type Job = (String, Box<dyn Fn() -> Tally + Send + Sync>);

/// The curved chart `chart` under each of its tilt configurations, `edits` edits each.
fn curved_jobs<C: Nav>(
    jobs: &mut Vec<Job>,
    name: &str,
    chart: C,
    edits: u32,
    off: bool,
    grid: Grid,
) {
    let hidden: Vec<u32> = (0..16).filter(|&k| chart.hidden(k)).collect();
    let cs = configs(&hidden);
    let per = edits.div_ceil(cs.len() as u32);
    for (i, config) in cs.into_iter().enumerate() {
        let seed = SEED ^ (jobs.len() as u64) << 8 ^ i as u64;
        jobs.push((
            format!("{name} under {config:?}"),
            Box::new(move || fuzz_curved(chart, config, seed, per, off, grid)),
        ));
    }
}

fn shape(map: ShapeMap, scale: f64) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    }
}

fn register_invariant() -> Invariant<f64> {
    Invariant {
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
        ..invariant()
    }
}

fn nu_k() -> Burrau<f64> {
    Burrau {
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

fn theta_k() -> BurrauThetaK<f64> {
    BurrauThetaK {
        theta_range: theta_range(),
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

/// The mixed-axis chart `Mixed<H, V>` at ranges inside its codomains, the other six controls seeded.
fn mixed_pair<H, V>(jobs: &mut Vec<Job>, edits: u32, off: bool)
where
    H: Quantity + Send + Sync + 'static,
    V: Quantity + Send + Sync + 'static,
{
    let range = |domain: [f64; 2], derived: bool| {
        if derived {
            let [lo, hi] = domain;
            [lo + (hi - lo) / 4.0, hi - (hi - lo) / 4.0]
        } else {
            [-1.0, 1.0]
        }
    };
    let chart = Mixed::<H, V, f64>::new(
        [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4],
        range(H::DOMAIN, H::DERIVED),
        range(V::DOMAIN, V::DERIVED),
    );
    curved_jobs(
        jobs,
        &format!("mixed ({}, {})", H::NAME, V::NAME),
        chart,
        edits,
        off,
        Grid::None,
    );
}

/// Every unordered pair of the module's axes that set two different coordinates, as `Mixed<H, V>` (gallery_sweep's).
macro_rules! mixed_pairs {
    ($jobs:expr, $e:expr, $off:expr; $head:tt $($tail:tt)*) => {
        $( mixed_pairs!(@cross $jobs, $e, $off; $head $tail); )*
        mixed_pairs!($jobs, $e, $off; $($tail)*);
    };
    ($jobs:expr, $e:expr, $off:expr;) => {};
    (@cross $jobs:expr, $e:expr, $off:expr; [$($h:ident)*] $v:tt) => {
        $( mixed_pairs!(@row $jobs, $e, $off; $h $v); )*
    };
    (@row $jobs:expr, $e:expr, $off:expr; $h:ident [$($v:ident)*]) => {
        $( mixed_pair::<$h, $v>($jobs, $e, $off); )*
    };
}

/// The edits per mixed-axis pair, each pair a chart of its own.
const MIXED_EDITS: u32 = EDITS;

/// Every gallery chart (gallery_sweep's list, R-467 D11), warped, at f64: the latent chart and its flat slices, the
/// shape sphere's three maps, the invariant charts, the Euclid plane, `(ν, K)`, `Φ_θ,K` (and `Φ_strip,K`, the same map),
/// `Φ_θ,L_z`, `Φ_θ,δm` (and `Φ_strip,δm`), the mass simplex, and the mixed-axis pairs.
fn gallery(off: bool) -> Vec<Job> {
    let mut jobs: Vec<Job> = Vec::new();
    if !off {
        let z0 = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];
        for a in 0..8 {
            for b in a + 1..8 {
                let frame = LatentFrame {
                    a,
                    b,
                    scale: 1.0,
                    gamma: 0.0,
                };
                let seed = SEED ^ u64::from(a * 8 + b);
                jobs.push((
                    format!("flat slice ({a}, {b})"),
                    Box::new(move || fuzz_latent(frame, [0.0; 8], seed, EDITS)),
                ));
            }
        }
        let frame = LatentFrame {
            a: 1,
            b: 4,
            scale: 0.3,
            gamma: 0.7,
        };
        jobs.push((
            "latent chart".into(),
            Box::new(move || fuzz_latent(frame, z0, SEED ^ 0xA7, EDITS)),
        ));
    }
    let warped = if off { Grid::None } else { Grid::Warped };
    for (name, map, scale) in [
        ("shape sphere, spherical", ShapeMap::Spherical, 0.0),
        (
            "shape sphere, exponential π/4",
            ShapeMap::Exponential,
            std::f64::consts::FRAC_PI_4,
        ),
        (
            "shape sphere, exponential π/2",
            ShapeMap::Exponential,
            std::f64::consts::FRAC_PI_2,
        ),
    ] {
        curved_jobs(&mut jobs, name, shape(map, scale), EDITS, off, Grid::None);
    }
    curved_jobs(
        &mut jobs,
        "(L_z, E)",
        InvariantLE(register_invariant()),
        EDITS,
        off,
        warped,
    );
    curved_jobs(
        &mut jobs,
        "(L_z, K)",
        InvariantLK(register_invariant()),
        EDITS,
        off,
        warped,
    );
    let plane = EuclidPlane {
        m_range: [EUCLID_M_MIN.number(), EUCLID_M_MAX.number()],
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
    };
    curved_jobs(
        &mut jobs,
        "Euclid plane",
        RestStart::new(plane),
        EDITS,
        off,
        Grid::None,
    );
    curved_jobs(&mut jobs, "(ν, K)", HeldLz::new(nu_k()), EDITS, off, warped);
    curved_jobs(
        &mut jobs,
        "Φ_θ,K",
        HeldLz::new(theta_k()),
        EDITS,
        off,
        warped,
    );
    let theta_lz = HeldK::new(theta_lz(), K_MAX.number(), GAMMA_K.number());
    curved_jobs(&mut jobs, "Φ_θ,L_z", theta_lz, EDITS, off, Grid::None);
    let delta_m = BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
    };
    curved_jobs(
        &mut jobs,
        "Φ_θ,δm",
        RestStart::new(delta_m),
        EDITS,
        off,
        Grid::None,
    );
    curved_jobs(
        &mut jobs,
        "mass simplex",
        RestStart::new(simplex(0.5)),
        EDITS,
        off,
        Grid::None,
    );
    let jobs_ref = &mut jobs;
    mixed_pairs!(jobs_ref, MIXED_EDITS, off;
        [ZAlpha Alpha] [ZBeta Beta] [ZQ0 PRhoX] [ZQ1 PRhoY] [ZQ2 PLambdaX] [ZQ3 PLambdaY] [ZMu1] [ZMu2]);
    jobs
}

/// The linear option's run (R-471 E2): the invariant and Burrau `K` charts, and `Φ_θ,L_z`, the same edits.
fn linear_gallery(mapping: Mapping, grid: Grid) -> Vec<Job> {
    let mut jobs: Vec<Job> = Vec::new();
    let le = Mapped::<_, Defaults>::new(InvariantLE(register_invariant()), [mapping; 2]);
    let lk = Mapped::<_, Defaults>::new(InvariantLK(register_invariant()), [mapping; 2]);
    curved_jobs(&mut jobs, "(L_z, E)", le, EDITS, false, grid);
    curved_jobs(&mut jobs, "(L_z, K)", lk, EDITS, false, grid);
    let nu = HeldLz {
        mapping,
        ..HeldLz::new(nu_k())
    };
    let th = HeldLz {
        mapping,
        ..HeldLz::new(theta_k())
    };
    curved_jobs(&mut jobs, "(ν, K)", nu, EDITS, false, grid);
    curved_jobs(&mut jobs, "Φ_θ,K", th, EDITS, false, grid);
    let lz = HeldK {
        mapping,
        ..HeldK::new(theta_lz(), K_MAX.number(), GAMMA_K.number())
    };
    curved_jobs(&mut jobs, "Φ_θ,L_z", lz, EDITS, false, grid);
    jobs
}

/// The worker threads, at most (R-228's four).
const WORKERS: usize = 4;

/// Runs the jobs on up to [`WORKERS`] threads, the tallies in the jobs' order.
fn run(jobs: Vec<Job>) -> Vec<(String, Tally)> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(vec![None; jobs.len()]);
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(WORKERS);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((name, job)) = jobs.get(i) else {
                    break;
                };
                let tally = job();
                out.lock().unwrap()[i] = Some((name.clone(), tally));
            });
        }
    });
    out.into_inner()
        .unwrap()
        .into_iter()
        .map(|t| t.expect("every job ran"))
        .collect()
}

/// Prints each run's line and fails naming every fault.
fn report(runs: &[(String, Tally)]) {
    println!("seed {SEED:#x}");
    for (name, t) in runs {
        println!(
            "{name}: {} edits, {} stopped, {} refused, {} tilts, {} grid pixels checked, {} infeasible",
            t.edits, t.stopped, t.refused, t.tilts, t.grid_checked, t.infeasible
        );
    }
    let faults: Vec<String> = runs
        .iter()
        .flat_map(|(name, t)| t.faults.iter().map(move |f| format!("{name}: {f}")))
        .collect();
    assert!(
        faults.is_empty(),
        "{} faults:\n{}",
        faults.len(),
        faults.join("\n")
    );
}

/// The warped fuzz over every gallery chart; `off` disables the stop (the control).
fn check_fuzz(off: bool) {
    let runs = run(gallery(off));
    report(&runs);
    let stopped: u32 = runs.iter().map(|(_, t)| t.stopped + t.refused).sum();
    assert!(stopped > 0, "no edit of the fuzz reached an edge");
}

#[test]
fn nav_fuzz_never_leaves() {
    check_fuzz(false);
}

negative_control!(
    nav_fuzz_never_leaves,
    "the fuzz with the stop disabled reaches an invalid centre",
    expected = "leaves the manifold",
    check_fuzz(true)
);

/// The linear option's arm (R-471 E2, A7): the same run, the stop keeping the centre feasible, finds infeasible pixels
/// inside `[0,1]²`, each classified not a system. `mapping` warped finds none (a control); `grid` read as a system
/// misclassifies them (a control).
fn check_linear(mapping: Mapping, grid: Grid) {
    let runs = run(linear_gallery(mapping, grid));
    report(&runs);
    for chart in ["(L_z, E)", "(L_z, K)", "(ν, K)", "Φ_θ,K", "Φ_θ,L_z"] {
        let found: u64 = runs
            .iter()
            .filter(|(name, _)| name.starts_with(chart))
            .map(|(_, t)| t.infeasible)
            .sum();
        assert!(
            found > 0,
            "the linear {chart} run finds no infeasible pixel inside [0,1]²"
        );
    }
}

#[test]
fn nav_fuzz_never_leaves_linear() {
    check_linear(Mapping::Linear, Grid::Linear { as_system: false });
}

negative_control!(
    nav_fuzz_never_leaves_linear,
    "the same run warped finds no infeasible pixel",
    expected = "finds no infeasible pixel",
    check_linear(Mapping::Warped, Grid::Linear { as_system: false })
);

mod as_system {
    use super::*;
    negative_control!(
        nav_fuzz_never_leaves_linear,
        "an infeasible pixel read as a system fails",
        expected = "is classified a system",
        check_linear(Mapping::Linear, Grid::Linear { as_system: true })
    );
}
