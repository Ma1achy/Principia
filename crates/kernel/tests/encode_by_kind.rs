//! Encode into the active chart by axis kind (`kernel::encode::by_kind`; inverse_encode_contract Parts 5 and 6; dd_encode
//! §3.1, §4 "fibre", §5 test 6; TASK-M2-17):
//! - `encode_axis_kinds_raw_latent`, `_derived`, `_invariant`: a state the chart decodes, under a random gauge, lands
//!   back on the chart at its own point, through kind 1 (latent charts, axis-aligned and oblique), kind 2 (mixed
//!   charts, the ternary mass plot, the shape sphere under both maps, its lower hemisphere at its own signed point under
//!   gauges without the mirror, R-544) and kind 3 (both invariant charts), at unequal masses (PIT-9) (REQ-ENC-008).
//! - `encode_shape_sphere_signed`: the shape sphere's lookup places a state at its signed `n̂`, a state and its mirror
//!   at `(θ, φ)` and `(θ, π − φ)`; decode then lookup returns the pixel on both hemispheres; T1 bitwise for `z` and every
//!   other chart; a lower-hemisphere state whose unmirrored momenta are the slice's lands and decodes back to itself, its
//!   mirror to latent `z` (R-544; REQ-ENC-034).
//! - `encode_axis_kinds_annotation`: an annotation axis gives no coordinate and is not compared (REQ-ENC-008).
//! - `encode_axis_kinds_unaligned`: a state off every chart, another point of the invariant fibre among them, lands in
//!   latent `z`, `E(x)`'s own (REQ-ENC-008).
//! - `encode_axis_kinds_primitives`: each kind's reading at its boundaries: every coordinate's component and derived
//!   value, a dependent plane, the warp's inverse at rest, a NaN residual and the tolerance's edge.
//! - `encode_invariant_reuses_decode`: an entered `(L_z, E)` or `(L_z, K)` and the clicked pixel give one `z`, at f64
//!   and f32, the edges included (REQ-ENC-009).
//! - `encode_infeasible_invariant`, `_feasibility`, `_degenerate`: dd_encode §5 test 6, feasibility before the
//!   construction, project or reject by the contract's metric and criterion, flagged, never NaN; the seed rule's
//!   `DEGENERATE` is the pixel path's (REQ-ENC-010, REQ-ENC-030).
//! - `encode_residual_convention`: `m₀ = 0.4` under hold-`m₁:m₂` and hold-`m₁ = m₂` gives two `z`, each round-tripping
//!   only under its own convention (REQ-CHART-005).
//!
//! **The norm and tolerance** (applied per R-369, a test decision): [`encode_into`] takes Part 4's `‖·‖_phys`
//! (REQ-ENC-029) and `ε_phys` (REQ-ENC-024) from its caller. Here the norm is [`norm`], the largest difference of any
//! mass, position or momentum component, and the tolerance `1e-9` at f64, far above the residual of a state on the
//! chart (printed) and far below that of one off it; coordinates and `z` are compared on the
//! interior, where the links are well conditioned (dd_encode §3.5).
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory.

use std::marker::PhantomData;

use kernel::chart::invariant::{l_max, Invariant, InvariantLE, InvariantLK, WarpReal};
use kernel::chart::mixed::{Alpha, Beta, Coordinate, PLambdaY, PRhoX, ZMu1, ZQ1};
use kernel::chart::{
    Burrau, Chart, ChartDecode, ChartOut, Constraint, Float, Latent, MassSimplex, Mixed, ShapeMap,
    ShapeSphere, ValidationResult,
};
use kernel::decode::config;
use kernel::decode::invariant::{construct_by_norms, construct_seeded, Outcome, Seed};
use kernel::decode::mass::{self, from_masses};
use kernel::decode::{self as dec, Decoded, Ic};
use kernel::encode::by_kind::ChartEncoded;
use kernel::encode::by_kind::{
    component, derived, encode_into, exponential_inverse, feasibility, fibre, fibre_at, invariants,
    m0, plane, spherical_inverse, warp_inverse, ByKind, Coord, Feasibility, HoldEqual, HoldRatio,
    InvariantAxis, Landing, Lookup, Residual, SeedRule,
};
use kernel::encode::{encode, Encoded};
use kernel::generated::links::{Defaults, LinkReal, Selection};
use kernel::payload::PayloadReal;
use kernel::shape::shape;
#[cfg(feature = "controls")]
use kernel::shape::shape_vec;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};
use validation::SHAPE_ROUNDTRIP_BOUND;

/// The landing tolerance at f64 (the module's note).
const TOL: f64 = 1e-9;

/// The tests' norm (the module's note): the largest difference of any mass, position or momentum component, NaN where
/// any is NaN.
fn norm(a: &Ic<f64>, b: &Ic<f64>) -> f64 {
    let mut d = Vec::with_capacity(15);
    for i in 0..3 {
        d.push(a.m[i] - b.m[i]);
        for k in 0..2 {
            d.push(a.r[i][k] - b.r[i][k]);
            d.push(a.p[i][k] - b.p[i][k]);
        }
    }
    d.iter().fold(0.0f64, |w, x| {
        if w.is_nan() || x.is_nan() {
            f64::NAN
        } else {
            w.max(x.abs())
        }
    })
}

/// `x` encoded into `chart` under [`norm`] at the tolerance `tol`.
fn into<C: ByKind<Param = f64>>(chart: &C, x: &Ic<f64>, tol: f64) -> ChartEncoded<f64> {
    encode_into::<Defaults, f64, C, _>(chart, x, norm, tol)
}

/// The frozen controls the charts share: unequal masses (PIT-9), an interior configuration, nonzero momenta.
const SLICE: [f64; 8] = [0.3, -0.4, 0.2, -0.5, 0.7, 0.1, 0.12, -0.08];

fn rng(seed: u64) -> SplitMix {
    SplitMix::new(seed)
}

/// An interior chart point.
fn point(rng: &mut SplitMix) -> (f64, f64) {
    (uniform(rng, 0.1, 0.9), uniform(rng, 0.1, 0.9))
}

/// `chart`'s decode at `(u, v)`, under a random gauge.
fn gauged<C: ChartDecode<Param = f64>>(chart: &C, u: f64, v: f64, rng: &mut SplitMix) -> Ic<f64> {
    let x = chart.decode_at::<Defaults, f64>(u, v).ic;
    Gauge::random(rng, 3.0, 2.0).act(&x)
}

/// `x` encoded into `chart` lands on it at `(u, v)`, its coordinates within `tol_uv`; returns the residual.
fn check_lands<C: ByKind<Param = f64>>(
    chart: &C,
    x: &Ic<f64>,
    (u, v): (f64, f64),
    tol_uv: f64,
) -> f64 {
    let e = into(chart, x, TOL);
    match e.landing {
        Landing::Chart { u: lu, v: lv } => assert!(
            (lu - u).abs() <= tol_uv && (lv - v).abs() <= tol_uv,
            "lands at ({lu}, {lv}), not the chart point ({u}, {v})"
        ),
        Landing::Latent => panic!(
            "a state the chart decodes at ({u}, {v}) lands in latent z (residual {:?})",
            e.residual
        ),
    }
    e.residual.expect("a landing has a residual")
}

/// Decodes from `from` at random interior points and checks each lands on `into` at the same point.
fn sweep<C: ByKind<Param = f64>>(from: &C, into: &C, seed: u64, name: &str) {
    sweep_with(from, into, seed, name, point, |u, v| (u, v));
}

/// [`sweep`] at the points `pick` draws, each landing at `lands(u, v)`.
fn sweep_with<C: ByKind<Param = f64>>(
    from: &C,
    into: &C,
    seed: u64,
    name: &str,
    pick: fn(&mut SplitMix) -> (f64, f64),
    lands: fn(f64, f64) -> (f64, f64),
) {
    let mut r = rng(seed);
    let mut worst = 0.0f64;
    for _ in 0..200 {
        let (u, v) = pick(&mut r);
        let x = gauged(from, u, v, &mut r);
        worst = worst.max(check_lands(into, &x, lands(u, v), 1e-9));
    }
    eprintln!("{name}: worst landing residual {worst:e}");
}

// ── Kind 1 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn unit(k: usize, s: f64) -> [f64; 8] {
    let mut q = [0.0; 8];
    q[k] = s;
    q
}

fn latent_charts() -> [Latent<f64>; 3] {
    let oblique1 = [0.3, 0.2, -0.1, 0.4, 0.0, 0.2, -0.3, 0.1];
    let oblique2 = [-0.1, 0.4, 0.3, 0.0, 0.2, -0.2, 0.1, 0.3];
    [
        Latent {
            z0: SLICE,
            q1: unit(0, 0.8),
            q2: unit(4, 0.6),
        },
        Latent {
            z0: SLICE,
            q1: unit(6, 0.9),
            q2: unit(1, 0.7),
        },
        Latent {
            z0: SLICE,
            q1: oblique1,
            q2: oblique2,
        },
    ]
}

#[test]
fn encode_axis_kinds_raw_latent() {
    for (n, c) in latent_charts().iter().enumerate() {
        sweep(c, c, 10 + n as u64, &format!("latent chart {n}"));
    }
}

negative_control!(
    encode_axis_kinds_raw_latent,
    "a state from another slice must not land on the chart",
    expected = "lands in latent z",
    {
        let into = latent_charts()[0];
        let mut from = into;
        from.z0[2] += 0.3;
        sweep(&from, &into, 10, "control");
    }
);

// ── Kind 2 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn mixed_ap() -> Mixed<Alpha, PRhoX, f64> {
    Mixed::new(SLICE, [0.3, 1.2], [-0.8, 0.9])
}

fn mixed_qb() -> Mixed<ZQ1, Beta, f64> {
    Mixed::new(SLICE, [-1.5, 1.0], [0.4, 2.6])
}

fn mixed_lm() -> Mixed<PLambdaY, ZMu1, f64> {
    Mixed::new(SLICE, [0.9, -0.6], [-1.0, 1.4])
}

#[test]
fn encode_axis_kinds_derived() {
    sweep(&mixed_ap(), &mixed_ap(), 20, "(α, p_ρ,x)");
    sweep(&mixed_qb(), &mixed_qb(), 21, "(z_q1, β)");
    sweep(&mixed_lm(), &mixed_lm(), 22, "(p_λ,y, z_μ1)");
    let simplex = MassSimplex {
        nu0: 0.4,
        eps_m: 1e-4,
    };
    sweep(&simplex, &simplex, 23, "mass simplex");
    // The edge u = 1, where the shear's v is not read: it lands at v = 0.
    let x = simplex.decode_at::<Defaults, f64>(1.0, 0.7).ic;
    check_lands(&simplex, &x, (1.0, 0.0), 1e-12);
    // The shape sphere: each hemisphere lands at its own signed point (R-544), under gauges without the mirror, whose
    // image is another system there (`encode_shape_sphere_signed`).
    let sphere = shape_sphere(ShapeMap::Spherical);
    let upper = |r: &mut SplitMix| (uniform(r, 0.1, 0.9), uniform(r, 0.55, 0.95));
    let lower = |r: &mut SplitMix| (uniform(r, 0.1, 0.9), uniform(r, 0.05, 0.45));
    sweep_proper(&sphere, 24, "shape sphere, upper", upper);
    sweep_proper(&sphere, 25, "shape sphere, lower", lower);
    let exp = shape_sphere(ShapeMap::Exponential);
    sweep_proper(&exp, 26, "shape sphere, exponential map", point);
}

/// Decodes `chart` at the points `pick` draws and checks each, under a random gauge without the mirror, lands at the
/// same point: the shape sphere's point is the signed `n̂`, so the mirror's image is another system (R-544).
fn sweep_proper<C: ByKind<Param = f64>>(
    chart: &C,
    seed: u64,
    name: &str,
    pick: fn(&mut SplitMix) -> (f64, f64),
) {
    let mut r = rng(seed);
    let mut worst = 0.0f64;
    for _ in 0..200 {
        let (u, v) = pick(&mut r);
        let x = chart.decode_at::<Defaults, f64>(u, v).ic;
        let g = Gauge {
            mirror: false,
            ..Gauge::random(&mut r, 3.0, 2.0)
        };
        worst = worst.max(check_lands(chart, &g.act(&x), (u, v), 1e-9));
    }
    eprintln!("{name}: worst landing residual {worst:e}");
}

/// The shape sphere under `map`, its exponential frame about a point of the upper hemisphere, with an extent that keeps
/// `[0,1]²` on it; the held blocks `SLICE`'s.
fn shape_sphere(map: ShapeMap) -> ShapeSphere<f64> {
    let (c, s) = (0.6f64, 0.8f64);
    ShapeSphere {
        map,
        n0: [s * 0.6, s * 0.8, c],
        e1: [0.8, -0.6, 0.0],
        e2: [-c * 0.6, -c * 0.8, s],
        scale: 0.5,
        inertia: 1.7,
        phase: 0.3,
        z_mass: [SLICE[6], SLICE[7]],
        z_mom: [SLICE[2], SLICE[3], SLICE[4], SLICE[5]],
    }
}

negative_control!(
    encode_axis_kinds_derived,
    "a state whose frozen block differs must not land on the mixed chart",
    expected = "lands in latent z",
    {
        let mut frozen = SLICE;
        frozen[1] = 0.5;
        let from: Mixed<Alpha, PRhoX, f64> = Mixed::new(frozen, [0.3, 1.2], [-0.8, 0.9]);
        sweep(&from, &mixed_ap(), 20, "control");
    }
);

// ── Kind 3 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn frozen_invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [SLICE[0], SLICE[1]],
        z_mass: [SLICE[6], SLICE[7]],
        k_max: 1.5,
        gamma_k: 2.0,
    }
}

/// The chart's invariants at `(u, v)` are the state's, evaluated directly.
fn check_invariants(x: &Ic<f64>, chart: &InvariantLE<f64>, (u, v): (f64, f64)) {
    let inv = invariants(&encode::<Defaults, f64>(x).canonical.ic);
    let t = chart.0.targets(u, v);
    let u_pot = invariants(&chart.decode_at::<Defaults, f64>(u, v).ic).u;
    let e = chart.energy(u_pot, v);
    for (name, got, want) in [("L_z", inv.lz, t.lz), ("K", inv.k, t.k), ("E", inv.e, e)] {
        assert!(
            (got - want).abs() <= 1e-12 * (1.0 + want.abs()),
            "the state's {name} {got} is not the chart's {want}"
        );
    }
}

#[test]
fn encode_axis_kinds_invariant() {
    let le = InvariantLE(frozen_invariant());
    let lk = InvariantLK(frozen_invariant());
    sweep(&le, &le, 30, "(L_z, E)");
    sweep(&lk, &lk, 31, "(L_z, K)");
    let mut r = rng(32);
    for _ in 0..100 {
        let (u, v) = point(&mut r);
        check_invariants(&gauged(&le, u, v, &mut r), &le, (u, v));
    }
}

negative_control!(
    encode_axis_kinds_invariant,
    "another point of the fibre must not land on the invariant chart",
    expected = "lands in latent z",
    {
        let le = InvariantLE(frozen_invariant());
        check_lands(&le, &other_fibre_point(&le, 0.7, 0.6), (0.7, 0.6), 1e-9);
    }
);

/// The state with the chart's `(L_z, K)` at `(u, v)` and configuration, its momenta built on a seed the chart's rule
/// does not take: another point of the fibre.
fn other_fibre_point(chart: &InvariantLE<f64>, u: f64, v: f64) -> Ic<f64> {
    let d = chart.decode_at::<Defaults, f64>(u, v);
    let t = chart.0.targets(u, v);
    let c = construct_seeded(&d.masses, &d.config, Seed::JRho, t.lz, t.k);
    assert_eq!(c.outcome, Outcome::Feasible);
    Ic { p: c.p, ..d.ic }
}

// ── Annotation, and the m₀ axis's chart ───────────────────────────────────────────────────────────────────────────

/// A chart with a kind-2 `m₀` axis under the residual convention `C` on `u` and an annotation axis on `v`: a 1-swept
/// family rendered in 2D (chart_decoder_contract Part 5). Its forward decode and its encode both call `C`.
struct M0Chart<C, P> {
    slice: [P; 8],
    range: [P; 2],
    convention: PhantomData<C>,
}

impl<C, P: Copy> M0Chart<C, P> {
    fn new(slice: [P; 8], range: [P; 2]) -> Self {
        M0Chart {
            slice,
            range,
            convention: PhantomData,
        }
    }
}

impl<C: Residual, P: Copy> M0Chart<C, P> {
    /// The slice's masses, the convention's held point.
    fn held<S: Selection, R: LinkReal + From<P>>(&self) -> [R; 3] {
        let w = <R as From<P>>::from;
        mass::decode::<S, R>([w(self.slice[6]), w(self.slice[7])]).m
    }

    /// `m₀` at `u`.
    fn m0_at<R: Float + From<P>>(&self, u: R) -> R {
        let w = <R as From<P>>::from;
        w(self.range[0]) + (w(self.range[1]) - w(self.range[0])) * u
    }

    /// The forward axis's latent point at `u`: the slice with its mass block at `m₀(u)` under `C`.
    fn controls<S: Selection, R: LinkReal + From<P>>(&self, u: R) -> [R; 8] {
        let mut z = self.slice.map(<R as From<P>>::from);
        let [a, b] = C::controls::<S, R>(self.m0_at(u), self.held::<S, R>());
        z[6] = a;
        z[7] = b;
        z
    }
}

impl<C: Residual, P: Copy> Chart for M0Chart<C, P> {
    type Param = P;

    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut {
            z: dec::Latent::new(self.slice.map(<F as From<P>>::from)),
        }
    }

    fn name(&self) -> &str {
        C::STATEMENT
    }

    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }

    fn decode<S, R>(&self, u: R, _v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        dec::decode::<S, R>(&dec::Latent::new(self.controls::<S, R>(u)))
    }
}

impl<C: Residual, P: Copy> ChartDecode for M0Chart<C, P> {
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }
}

impl<C: Residual, P: Copy> ByKind for M0Chart<C, P> {
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let w = <R as From<P>>::from;
        let (lo, hi) = (w(self.range[0]), w(self.range[1]));
        [Coord::At((m0(e) - lo) / (hi - lo)), Coord::Annotation]
    }
}

const M0_RANGE: [f64; 2] = [0.1, 0.9];

/// The annotation axis gives no coordinate, and the landing's `v` is 0 whatever the state's chart `v`.
fn check_annotation<C: ByKind<Param = f64>>(chart: &C, x: &Ic<f64>) {
    let e = into(chart, x, TOL);
    assert_eq!(
        e.coords[1],
        Coord::Annotation,
        "the annotation axis gave a coordinate"
    );
    match e.landing {
        Landing::Chart { v, .. } => assert_eq!(v, 0.0, "the annotation axis's v was read"),
        Landing::Latent => panic!("the state lands in latent z (residual {:?})", e.residual),
    }
}

#[test]
fn encode_axis_kinds_annotation() {
    let chart: M0Chart<HoldRatio, f64> = M0Chart::new(SLICE, M0_RANGE);
    let mut r = rng(40);
    for _ in 0..50 {
        let (u, v) = point(&mut r);
        let x = gauged(&chart, u, v, &mut r);
        check_annotation(&chart, &x);
        check_lands(&chart, &x, (u, 0.0), 1e-9);
    }
}

/// A chart reading its annotation axis as a coordinate: the control's mistake.
#[cfg(feature = "controls")]
struct ReadsAnnotation(M0Chart<HoldRatio, f64>);

#[cfg(feature = "controls")]
impl Chart for ReadsAnnotation {
    type Param = f64;
    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        self.0.map(u, v)
    }
    fn name(&self) -> &str {
        "reads its annotation axis"
    }
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.0.validate(u, v)
    }
}

#[cfg(feature = "controls")]
impl ChartDecode for ReadsAnnotation {
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<f64>,
    {
        self.0.decode_at::<S, R>(u, v)
    }
}

#[cfg(feature = "controls")]
impl ByKind for ReadsAnnotation {
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<f64>,
    {
        let [u, _] = self.0.coords::<S, R>(e);
        [u, Coord::At(R::one() / (R::one() + R::one()))]
    }
}

negative_control!(
    encode_axis_kinds_annotation,
    "an annotation axis read as a coordinate must fail",
    expected = "the annotation axis gave a coordinate",
    {
        let chart = ReadsAnnotation(M0Chart::new(SLICE, M0_RANGE));
        let x = chart.decode_at::<Defaults, f64>(0.4, 0.3).ic;
        check_annotation(&chart, &x);
    }
);

// ── Off every chart ────────────────────────────────────────────────────────────────────────────────────────────────

/// A generic state: random positive masses, positions and momenta.
fn generic(rng: &mut SplitMix) -> Ic<f64> {
    let m = [
        uniform(rng, 0.2, 1.0),
        uniform(rng, 0.2, 1.0),
        uniform(rng, 0.2, 1.0),
    ];
    let total = m[0] + m[1] + m[2];
    let mut v = || [uniform(rng, -1.0, 1.0), uniform(rng, -1.0, 1.0)];
    Ic {
        m: m.map(|x| x / total),
        r: [v(), v(), v()],
        p: [v(), v(), v()],
    }
}

/// `x` lands in latent `z` on `chart`, `E(x)`'s own `z`.
fn check_latent<C: ByKind<Param = f64>>(chart: &C, x: &Ic<f64>) {
    let e = into(chart, x, TOL);
    assert_eq!(
        e.encoded,
        encode::<Defaults, f64>(x),
        "the fallback is not E(x)"
    );
    assert!(
        e.landing == Landing::Latent && e.residual.is_none_or(|d| d > TOL || d.is_nan()),
        "a state off the chart lands on it, residual {:?}",
        e.residual
    );
}

#[test]
fn encode_axis_kinds_unaligned() {
    let le = InvariantLE(frozen_invariant());
    let lk = InvariantLK(frozen_invariant());
    let m0c: M0Chart<HoldEqual, f64> = M0Chart::new(SLICE, M0_RANGE);
    let mut r = rng(50);
    for _ in 0..100 {
        let x = generic(&mut r);
        for c in latent_charts() {
            check_latent(&c, &x);
        }
        check_latent(&mixed_ap(), &x);
        check_latent(&mixed_qb(), &x);
        check_latent(&mixed_lm(), &x);
        check_latent(&le, &x);
        check_latent(&lk, &x);
        check_latent(&m0c, &x);
        check_latent(
            &MassSimplex {
                nu0: 0.4,
                eps_m: 1e-4,
            },
            &x,
        );
        check_latent(&shape_sphere(ShapeMap::Spherical), &x);
        check_latent(&shape_sphere(ShapeMap::Exponential), &x);
    }
    for (u, v) in [(0.7, 0.6), (0.2, 0.9), (0.5, 0.3)] {
        check_latent(&le, &other_fibre_point(&le, u, v));
    }
}

negative_control!(
    encode_axis_kinds_unaligned,
    "a state on the chart must fail the off-chart check",
    expected = "a state off the chart lands on it",
    {
        let le = InvariantLE(frozen_invariant());
        check_latent(&le, &le.decode_at::<Defaults, f64>(0.4, 0.5).ic);
    }
);

// ── Each kind's reading at its boundaries ─────────────────────────────────────────────────────────────────────────

/// Every coordinate's component, the derived values, a dependent plane, the warp's inverse at rest, a NaN residual,
/// and the landing tolerance's edge.
fn check_primitives(z_shift: f64) {
    let z: [f64; 8] = std::array::from_fn(|k| k as f64 + z_shift);
    let all = [
        Coordinate::ZAlpha,
        Coordinate::ZBeta,
        Coordinate::ZQ0,
        Coordinate::ZQ1,
        Coordinate::ZQ2,
        Coordinate::ZQ3,
        Coordinate::ZMu1,
        Coordinate::ZMu2,
    ];
    for (k, c) in all.into_iter().enumerate() {
        assert_eq!(
            component(&z, c),
            k as f64,
            "component {c:?} reads the wrong control"
        );
    }
    let x = mixed_ap().decode_at::<Defaults, f64>(0.3, 0.6).ic;
    let e = encode::<Defaults, f64>(&x);
    let j = e.blocks.jacobi;
    let want = [
        Some(e.blocks.alpha),
        Some(e.blocks.beta),
        Some(j.p_rho[0]),
        Some(j.p_rho[1]),
        Some(j.p_lambda[0]),
        Some(j.p_lambda[1]),
        None,
        None,
    ];
    for (c, w) in all.into_iter().zip(want) {
        assert_eq!(
            derived(&e, c),
            w,
            "derived {c:?} is not the block inverse's value"
        );
    }
    // The Gram solve: exact components on an axis pair, an oblique pair recovered, a dependent pair refused.
    let d = [0.5, 0.0, 0.0, -0.25, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(
        plane(&d, &unit(0, 1.0), &unit(3, 1.0)),
        Some((0.5, -0.25)),
        "the axis pair's components"
    );
    let (q1, q2) = (
        [1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    );
    let d: [f64; 8] = std::array::from_fn(|k| 2.0 * q1[k] - 3.0 * q2[k]);
    let (a, b) = plane(&d, &q1, &q2).expect("an independent pair");
    assert!(
        (a - 2.0).abs() < 1e-15 && (b + 3.0).abs() < 1e-15,
        "oblique components ({a}, {b})"
    );
    assert_eq!(
        plane(&d, &q1, &q1.map(|q| 2.0 * q)),
        None,
        "a dependent pair placed a point"
    );
    // A near-dependent pair whose Gram product `g₁₁g₂₂` does not fit the format: the determinant's fixed form keeps its
    // rounding error, so the first component, an exact quotient there, is exactly 2.
    let t = 2f64.powi(-14);
    let (n1, n2) = (
        [1.0, t, t, 0.0, 0.0, 0.0, 0.0, 0.0],
        [1.0, t, -t, 0.0, 0.0, 0.0, 0.0, 0.0],
    );
    let dn: [f64; 8] = std::array::from_fn(|k| 2.0 * n1[k] - 3.0 * n2[k]);
    let (an, _) = plane(&dn, &n1, &n2).expect("an independent pair");
    assert!(
        (an - 2.0).abs() < 1e-12,
        "a near-dependent pair's component {an}"
    );
    // Dependent directions whose Gram products round: the determinant's fixed form is exactly 0 there.
    let qa = [0.1, 0.2, 0.3, 0.7, -0.6, 0.35, 0.9, -0.15];
    assert_eq!(
        plane(&d, &qa, &qa),
        None,
        "a rounded dependent pair placed a point"
    );
    let flat = Latent {
        z0: SLICE,
        q1,
        q2: q1,
    };
    let e_flat = into(&flat, &x, TOL);
    assert_eq!(
        e_flat.coords,
        [Coord::Unplaced, Coord::Unplaced],
        "a dependent plane's coordinates"
    );
    assert!(
        e_flat.landing == Landing::Latent && e_flat.residual.is_none(),
        "a dependent plane lands"
    );
    // The warp inverted: at rest `s = ½` (no `L_max` to divide by), `t = 0`; elsewhere `K(t)` and `L_z(s, t)` back.
    let rest = invariants(&Ic {
        p: [[0.0; 2]; 3],
        ..x
    });
    assert_eq!(
        warp_inverse(1.5, 2.0, &rest),
        (0.5, 0.0),
        "the warp's inverse at rest"
    );
    let inv = kernel::encode::by_kind::Invariants {
        lz: -0.3,
        k: 0.375,
        u: -1.0,
        e: -0.625,
    };
    let (s, t) = warp_inverse(1.5, 2.0, &inv);
    assert!(
        (t - 0.5).abs() < 1e-15 && ((2.0 * s - 1.0) * l_max(0.375) + 0.3).abs() < 1e-15,
        "warp ({s}, {t})"
    );
    // The shape sphere's inverses: θ taken into [0, 2π), the pole's θ selected as 0, the exponential centre.
    let pi = std::f64::consts::PI;
    let (s_neg, t_eq) = spherical_inverse([0.5f64.cos(), -(0.5f64.sin()), 0.0], pi);
    assert!(
        (s_neg - (1.0 - 0.5 / (2.0 * pi))).abs() < 1e-15 && (t_eq - 0.5).abs() < 1e-15,
        "θ < 0: ({s_neg}, {t_eq})"
    );
    assert_eq!(
        spherical_inverse([0.0, -0.0, 1.0], pi),
        (0.0, 1.0),
        "the pole"
    );
    assert_eq!(spherical_inverse([-1.0, 0.0, 0.0], pi), (0.5, 0.5), "θ = π");
    assert_eq!(
        spherical_inverse([-1.0, -0.0, 0.0], pi),
        (0.5, 0.5),
        "θ = π at n₁ = −0"
    );
    let (s_y, t_y) = spherical_inverse([0.0, 1.0, 0.0], pi);
    assert!(
        (s_y - 0.25).abs() < 1e-15 && (t_y - 0.5).abs() < 1e-15,
        "n₀ = 0 off the pole: ({s_y}, {t_y})"
    );
    let frame = ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    assert_eq!(
        exponential_inverse([0.0, 0.0, 1.0], frame, 0.5),
        (0.5, 0.5),
        "the exponential centre"
    );
    let (eu, ev) = exponential_inverse([0.25f64.sin(), 0.0, 0.25f64.cos()], frame, 0.5);
    assert!(
        (eu - 0.75).abs() < 1e-15 && (ev - 0.5).abs() < 1e-15,
        "the exponential map's ({eu}, {ev})"
    );
    // A NaN residual lands in latent `z`.
    let le = InvariantLE(frozen_invariant());
    let xs = gauged(&le, 0.3, 0.7, &mut rng(60));
    let e_nan =
        encode_into::<Defaults, f64, _, _>(&le, &xs, |_: &Ic<f64>, _: &Ic<f64>| f64::NAN, TOL);
    assert!(
        e_nan.landing == Landing::Latent && e_nan.residual.is_some_and(f64::is_nan),
        "a NaN residual landed"
    );
    // The landing's tolerance: a residual exactly at it lands; just below it does not.
    let at = into(&le, &xs, TOL).residual.expect("a placed point");
    assert!(
        at > 0.0,
        "the gauged state's residual is 0, so the edge is not tested"
    );
    assert!(
        matches!(into(&le, &xs, at).landing, Landing::Chart { .. }),
        "at tol"
    );
    let below = at.next_down();
    assert_eq!(into(&le, &xs, below).landing, Landing::Latent, "below tol");
    // A non-finite coordinate places nothing: a mixed axis of no width.
    let narrow: Mixed<Alpha, PRhoX, f64> = Mixed::new(SLICE, [0.7, 0.7], [-0.8, 0.9]);
    let e_narrow = into(&narrow, &x, TOL);
    assert!(
        e_narrow.landing == Landing::Latent && e_narrow.residual.is_none(),
        "a NaN coordinate landed"
    );
}

#[test]
fn encode_axis_kinds_primitives() {
    check_primitives(0.0);
}

negative_control!(
    encode_axis_kinds_primitives,
    "a shifted control vector must fail the component read",
    expected = "reads the wrong control",
    check_primitives(1.0)
);

// ── Kind 3 (b): encode reuses decode ──────────────────────────────────────────────────────────────────────────────

/// A precision, with its f64 conversions.
trait P: LinkReal + PayloadReal + WarpReal + From<f32> {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn eps() -> f64;
}

impl P for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn eps() -> f64 {
        f64::EPSILON
    }
}

impl P for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn eps() -> f64 {
        f32::EPSILON as f64
    }
}

fn frozen_f32() -> Invariant<f32> {
    let f = frozen_invariant();
    Invariant {
        z_cfg: f.z_cfg.map(|x| x as f32),
        z_mass: f.z_mass.map(|x| x as f32),
        k_max: f.k_max as f32,
        gamma_k: f.gamma_k as f32,
    }
}

/// How the entered pair is encoded: `fibre` itself, or a control's replacement.
type Entry<R, C> = fn(&C, R, R) -> Lookup<R>;

/// At every point of a grid over `[0,1]²`, edges included: the clicked pixel's `z` (the chart's decode, then `E`) and
/// the entered pair's (`entry` on the pixel's `L_z` and vertical value) agree within `tol` eps per component of
/// `1 + |z|`, past the edge's allowance `EDGE·√|ΔK*|`. Returns the worst, in eps, and the worst `|Δz|/(1 + |z|)` per
/// `√|ΔK*|` where `ΔK* ≠ 0`.
fn reuses<R: P, C>(
    chart: &C,
    vertical: impl Fn(R, R) -> R,
    entry: Entry<R, C>,
    tol: f64,
) -> (f64, f64)
where
    C: InvariantAxis<Param = f32> + ChartDecode,
{
    let (mut worst, mut edge) = (0.0f64, 0.0f64);
    let n = 24;
    for i in 0..=n {
        for k in 0..=n {
            let (u, v) = (R::of(i as f64 / n as f64), R::of(k as f64 / n as f64));
            let d = chart.decode_at::<Defaults, R>(u, v);
            let pixel = encode::<Defaults, R>(&d.ic).z.z;
            let t = chart.invariant().targets(u, v);
            let u_pot = invariants(&Ic {
                p: [[R::zero(); 2]; 3],
                ..d.ic
            })
            .u;
            let y = vertical(u_pot, v);
            // The entered `K*` is the pixel's `K(t)` only to the rounding of `y` (`E = U + K` loses `|U|·eps`), which the
            // mixing amplitude `a = √(2(K* − L_z²/2))` magnifies to its square root at the parabola's edge.
            let dk = (C::kinetic(y, u_pot).f() - t.k.f()).abs();
            assert!(
                dk <= 4.0 * R::eps() * (1.0 + u_pot.f().abs()),
                "the entered K* is {dk:e} from the pixel's"
            );
            let Lookup::Encoded { z, .. } = entry(chart, t.lz, y) else {
                panic!(
                    "the pixel ({}, {}) is not encoded from its own pair",
                    u.f(),
                    v.f()
                );
            };
            for j in 0..8 {
                let rel = (z[j].f() - pixel[j].f()).abs() / (1.0 + pixel[j].f().abs());
                let err = (rel - EDGE * dk.sqrt()).max(0.0) / R::eps();
                worst = worst.max(err);
                if dk > 0.0 {
                    edge = edge.max(rel / dk.sqrt());
                }
                assert!(
                    err <= tol,
                    "z[{j}] {} entered differs from the pixel's {} by {err:.1} eps at ({}, {})",
                    z[j].f(),
                    pixel[j].f(),
                    u.f(),
                    v.f()
                );
            }
        }
    }
    (worst, edge)
}

/// The worst `z` disagreement a precision allows, in eps of `1 + |z|`: the pixel path's `C` and block inverses against
/// the entered path's frozen controls (applied per R-369, a test decision set over the measured worst, printed).
const REUSE_EPS: f64 = 64.0;

/// The bound on `|Δz|/(1 + |z|)` per `√|ΔK*|` at the parabola's edge (applied per R-369, as `REUSE_EPS`): `dz/da` is
/// bounded on the interior of the momentum links, and `|Δa| ≤ √(2|ΔK*|)`.
const EDGE: f64 = 2.0;

fn entered<R: P, C>(chart: &C, lz: R, y: R) -> Lookup<R>
where
    C: InvariantAxis<Param = f32>,
{
    fibre::<Defaults, R, C>(chart, lz, y)
}

#[test]
fn encode_invariant_reuses_decode() {
    let le = InvariantLE(frozen_f32());
    let lk = InvariantLK(frozen_f32());
    let le_e = |u_pot, v| le.energy(u_pot, v);
    let worst = [
        (
            "(L_z, E) f64",
            reuses::<f64, _>(&le, le_e, entered, REUSE_EPS),
        ),
        (
            "(L_z, K) f64",
            reuses::<f64, _>(&lk, |_, v| lk.0.targets(0.0, v).k, entered, REUSE_EPS),
        ),
        (
            "(L_z, E) f32",
            reuses::<f32, _>(&le, |u_pot, v| le.energy(u_pot, v), entered, REUSE_EPS),
        ),
        (
            "(L_z, K) f32",
            reuses::<f32, _>(&lk, |_, v| lk.0.targets(0.0f32, v).k, entered, REUSE_EPS),
        ),
    ];
    for (name, (w, edge)) in worst {
        eprintln!(
            "encode_invariant_reuses_decode {name}: worst {w:.1} eps, edge {edge:.3} per √ΔK*"
        );
    }
}

/// The control's entry: an independent fibre choice, the construction on the seed the chart's rule does not take.
#[cfg(feature = "controls")]
fn other_seed(chart: &InvariantLE<f32>, lz: f64, y: f64) -> Lookup<f64> {
    let z = chart.0.frozen::<f64>().z;
    let masses = mass::decode::<Defaults, f64>([z[6], z[7]]);
    let config = config::decode::<Defaults, f64>([z[0], z[1]], &masses);
    let u_pot = invariants(&Ic {
        m: masses.m,
        r: config.r,
        p: [[0.0; 2]; 3],
    })
    .u;
    let flipped = SeedRule::Decoded { z_alpha: -z[0] };
    let k = y - u_pot;
    fibre_at::<Defaults, f64>(
        &masses,
        &config,
        flipped,
        (lz, k),
        (Constraint::LzEParabola, Constraint::LzEParabola),
    )
}

negative_control!(
    encode_invariant_reuses_decode,
    "a fibre choice other than the construction's must give another z",
    expected = "entered differs from the pixel's",
    {
        let le = InvariantLE(frozen_f32());
        reuses::<f64, _>(&le, |u_pot, v| le.energy(u_pot, v), other_seed, REUSE_EPS);
    }
);

// ── Kind 3 (b): feasibility first ─────────────────────────────────────────────────────────────────────────────────

/// The frozen potential of an `(L_z, E)` chart.
fn frozen_u(chart: &InvariantLE<f64>) -> f64 {
    let d = chart.decode_at::<Defaults, f64>(0.5, 0.5);
    invariants(&Ic {
        p: [[0.0; 2]; 3],
        ..d.ic
    })
    .u
}

/// The decoded state of an encoded lookup.
fn decoded(z: [f64; 8]) -> Ic<f64> {
    dec::decode::<Defaults, f64>(&dec::Latent::new(z)).ic
}

/// dd_encode §5 test 6 on `(L_z, E)` with `K* = E − U < L_z²/2`: projected onto the parabola at the same `E`, on the
/// entered side, flagged with the parabola, finite; its decode has the projected `L_z` and the entered `E`.
fn check_projected(chart: &InvariantLE<f64>, lz: f64, e: f64, result: Lookup<f64>) {
    let k = e - frozen_u(chart);
    let Lookup::Encoded {
        z,
        lz: lp,
        k: kp,
        projected,
        ..
    } = result
    else {
        panic!("the infeasible pair ({lz}, {e}) was not projected: {result:?}");
    };
    assert_eq!(
        projected,
        Some(Constraint::LzEParabola),
        "the projection of ({lz}, {e}) is not flagged"
    );
    assert!(
        z.iter().all(|x| x.is_finite()),
        "the projection of ({lz}, {e}) has a non-finite z: {z:?}"
    );
    assert_eq!(kp, k, "the projection moved the energy");
    assert_eq!(
        lp,
        lz.signum() * l_max(k),
        "the projection is not onto the parabola on L_z's side"
    );
    let inv = invariants(&decoded(z));
    assert!(
        (inv.lz - lp).abs() <= 1e-12 && (inv.e - e).abs() <= 1e-12,
        "decodes to ({}, {})",
        inv.lz,
        inv.e
    );
}

#[test]
fn encode_infeasible_invariant() {
    let le = InvariantLE(frozen_invariant());
    let u_pot = frozen_u(&le);
    for (lz, k) in [
        (1.0, 0.3),
        (-1.0, 0.3),
        (2.5, 1e-3),
        (-0.2, 0.0199),
        (4.0, 2.0),
    ] {
        assert!(k < lz * lz / 2.0);
        let e = u_pot + k;
        check_projected(&le, lz, e, fibre::<Defaults, f64, _>(&le, lz, e));
    }
    // K* = 0 with L_z ≠ 0, and K* < 0 with L_z ≠ 0: the projection would change L_z's class, so rejected.
    for (lz, k) in [(0.5, 0.0), (-0.5, -0.0), (0.5, -0.2), (-1e-300, -1.0)] {
        assert_eq!(
            fibre::<Defaults, f64, _>(&le, lz, u_pot + k),
            Lookup::Rejected {
                failed: Constraint::LzEParabola
            },
            "({lz}, K* = {k}) is not rejected"
        );
    }
    // K* < 0 at L_z = 0: projected to the apex, the rest start.
    let apex = fibre::<Defaults, f64, _>(&le, 0.0, u_pot - 0.4);
    let Lookup::Encoded {
        z,
        lz,
        k,
        projected: Some(Constraint::LzEParabola),
        ..
    } = apex
    else {
        panic!("L_z = 0 below U is not projected to the apex: {apex:?}");
    };
    assert_eq!((lz, k), (0.0, 0.0), "the apex");
    assert!(
        invariants(&decoded(z)).k <= 1e-24,
        "the apex is not a rest start"
    );
    // A non-finite entry is rejected, never a NaN.
    for (lz, e) in [
        (f64::NAN, -1.0),
        (0.3, f64::NAN),
        (f64::INFINITY, 0.0),
        (0.0, f64::NEG_INFINITY),
    ] {
        assert_eq!(
            fibre::<Defaults, f64, _>(&le, lz, e),
            Lookup::Rejected {
                failed: Constraint::LzEParabola
            },
            "({lz}, {e}) is not rejected"
        );
    }
    // (L_z, K) names its two constraints.
    let lk = InvariantLK(frozen_invariant());
    let lk_at = |lz, k| fibre::<Defaults, f64, _>(&lk, lz, k);
    assert_eq!(
        lk_at(0.5, -0.1),
        Lookup::Rejected {
            failed: Constraint::KNonNegative
        },
        "(L_z, K) K < 0"
    );
    assert_eq!(
        lk_at(0.5, 0.0),
        Lookup::Rejected {
            failed: Constraint::KAboveLzBound
        },
        "(L_z, K) K = 0"
    );
    assert_eq!(
        lk_at(f64::NAN, 0.1),
        Lookup::Rejected {
            failed: Constraint::KAboveLzBound
        },
        "(L_z, K) NaN L_z"
    );
    assert_eq!(
        lk_at(0.1, f64::NAN),
        Lookup::Rejected {
            failed: Constraint::KNonNegative
        },
        "(L_z, K) NaN K"
    );
    let Lookup::Encoded { projected, k, .. } = lk_at(1.0, 0.3) else {
        panic!("(L_z, K) not projected")
    };
    assert_eq!(
        (projected, k),
        (Some(Constraint::KAboveLzBound), 0.3),
        "(L_z, K) projection"
    );
    let Lookup::Encoded { projected, k, .. } = lk_at(0.0, -0.3) else {
        panic!("(L_z, K) apex")
    };
    assert_eq!(
        (projected, k),
        (Some(Constraint::KNonNegative), 0.0),
        "(L_z, K) apex"
    );
    // A feasible pair is encoded unprojected, its decode at the pair. A momentum past q_max is clamped and flagged, in
    // the momentum bits alone.
    let Lookup::Encoded {
        z,
        projected: None,
        clamped: 0,
        ..
    } = fibre::<Defaults, f64, _>(&le, 0.2, u_pot + 0.5)
    else {
        panic!("a feasible pair is not encoded as entered");
    };
    let inv = invariants(&decoded(z));
    assert!(
        (inv.lz - 0.2).abs() < 1e-12 && (inv.k - 0.5).abs() < 1e-12,
        "a feasible pair decodes to ({}, {})",
        inv.lz,
        inv.k
    );
    let Lookup::Encoded { clamped, .. } = fibre::<Defaults, f64, _>(&le, 0.0, u_pot + 400.0) else {
        panic!("a fast pair is not encoded");
    };
    assert!(
        clamped != 0 && clamped & !0b11_1100 == 0,
        "a momentum past q_max flags {clamped:#b}"
    );
}

negative_control!(
    encode_infeasible_invariant,
    "a pair encoded unflagged must fail the projection check",
    expected = "was not projected",
    {
        let le = InvariantLE(frozen_invariant());
        let e = frozen_u(&le) + 0.3;
        check_projected(
            &le,
            1.0,
            e,
            Lookup::Rejected {
                failed: Constraint::LzEParabola,
            },
        );
    }
);

/// A feasibility test at f64: the kernel's, or a control's replacement.
type Test = fn(f64, f64, Constraint, Constraint) -> Feasibility<f64>;

/// The feasibility test `test`'s arms at their boundaries, for the constraints `low`, `wide`.
fn check_feasibility(lz: f64, test: Test) {
    let (low, wide) = (Constraint::KNonNegative, Constraint::KAboveLzBound);
    let edge = lz * lz / 2.0;
    assert_eq!(edge * 2.0, lz * lz, "the fixture's boundary is not exact");
    assert_eq!(
        test(lz, edge, low, wide),
        Feasibility::Valid,
        "K* = L_z²/2 is feasible"
    );
    assert_eq!(
        test(-lz, edge, low, wide),
        Feasibility::Valid,
        "K* = L_z²/2, L_z < 0"
    );
    let below = edge.next_down();
    assert_eq!(
        test(lz, below, low, wide),
        Feasibility::Projected {
            lz: l_max(below),
            k: below,
            failed: wide
        },
        "one ulp below the parabola"
    );
    assert_eq!(
        test(-lz, below, low, wide),
        Feasibility::Projected {
            lz: -l_max(below),
            k: below,
            failed: wide
        },
        "one ulp below the parabola, L_z < 0"
    );
    let tiny = f64::from_bits(1);
    assert_eq!(
        test(lz, tiny, low, wide),
        Feasibility::Projected {
            lz: l_max(tiny),
            k: tiny,
            failed: wide
        },
        "the least positive K*"
    );
    assert_eq!(
        test(0.0, 0.0, low, wide),
        Feasibility::Valid,
        "the apex is feasible"
    );
    assert_eq!(
        test(-0.0, -0.0, low, wide),
        Feasibility::Valid,
        "the apex at −0"
    );
    assert_eq!(
        test(lz, 0.0, low, wide),
        Feasibility::Rejected { failed: wide },
        "K* = 0, L_z ≠ 0"
    );
    assert_eq!(
        test(lz, -tiny, low, wide),
        Feasibility::Rejected { failed: low },
        "K* < 0, L_z ≠ 0"
    );
    assert_eq!(
        test(0.0, -tiny, low, wide),
        Feasibility::Projected {
            lz: 0.0,
            k: 0.0,
            failed: low
        },
        "K* < 0 at L_z = 0"
    );
    assert_eq!(
        test(f64::INFINITY, 1.0, low, wide),
        Feasibility::Rejected { failed: wide },
        "∞ L_z"
    );
    assert_eq!(
        test(0.0, f64::INFINITY, low, wide),
        Feasibility::Rejected { failed: low },
        "∞ K*"
    );
    assert_eq!(
        feasibility(0.5f32, 0.125f32, low, wide),
        Feasibility::Valid,
        "f32 boundary"
    );
}

#[test]
fn encode_infeasible_invariant_feasibility() {
    check_feasibility(0.75, feasibility);
}

negative_control!(
    encode_infeasible_invariant_feasibility,
    "a test taking the parabola itself as infeasible must fail",
    expected = "K* = L_z²/2 is feasible",
    check_feasibility(0.75, strict)
);

/// The control's test: `K* ≤ L_z²/2` taken as infeasible, the boundary on the wrong side.
#[cfg(feature = "controls")]
fn strict(lz: f64, k: f64, low: Constraint, wide: Constraint) -> Feasibility<f64> {
    if k == lz * lz / 2.0 && lz != 0.0 {
        Feasibility::Projected {
            lz,
            k,
            failed: wide,
        }
    } else {
        feasibility(lz, k, low, wide)
    }
}

/// The configuration where every body coincides: no seed has a norm, so the rule degenerates.
fn coincident() -> config::Config<f64> {
    config::Config {
        alpha: 0.0,
        beta: 0.0,
        rho_t: [0.0; 2],
        lambda_t: [0.0; 2],
        rho: [0.0; 2],
        lambda: [0.0; 2],
        r: [[0.0; 2]; 3],
    }
}

/// The fibre at `config` under the rule on computed norms is `DEGENERATE` exactly where the pixel path's construction
/// is.
fn check_degenerate(config: &config::Config<f64>) {
    let masses = from_masses([0.2, 0.5, 0.3]);
    let pixel = construct_by_norms(&masses, config, 0.0, 0.5);
    let wide = (Constraint::LzEParabola, Constraint::LzEParabola);
    let got = fibre_at::<Defaults, f64>(&masses, config, SeedRule::ByNorms, (0.0, 0.5), wide);
    assert_eq!(
        pixel.outcome == Outcome::Degenerate,
        got == Lookup::Degenerate,
        "the fibre's DEGENERATE is not the pixel path's ({:?} against {got:?})",
        pixel.outcome
    );
    assert_eq!(
        got,
        Lookup::Degenerate,
        "the configuration is not DEGENERATE: {got:?}"
    );
}

#[test]
fn encode_infeasible_invariant_degenerate() {
    let masses = from_masses([0.2, 0.5, 0.3]);
    check_degenerate(&coincident());
    // Infeasibility takes precedence over the seed rule, as on the pixel path: checked before construction.
    let wide = (Constraint::LzEParabola, Constraint::LzEParabola);
    assert_eq!(
        fibre_at::<Defaults, f64>(&masses, &coincident(), SeedRule::ByNorms, (1.0, 0.0), wide),
        Lookup::Rejected {
            failed: Constraint::LzEParabola
        },
        "an infeasible pair at a degenerate configuration"
    );
    // A decoded configuration never degenerates, under either rule.
    let config = config::decode::<Defaults, f64>([0.3, -0.4], &masses);
    for rule in [SeedRule::ByNorms, SeedRule::Decoded { z_alpha: 0.3 }] {
        let got = fibre_at::<Defaults, f64>(&masses, &config, rule, (0.0, 0.5), wide);
        assert!(
            matches!(got, Lookup::Encoded { .. }),
            "a decoded configuration gives {got:?}"
        );
    }
}

negative_control!(
    encode_infeasible_invariant_degenerate,
    "a configuration whose seeds qualify must fail the DEGENERATE check",
    expected = "the configuration is not DEGENERATE",
    {
        let masses = from_masses([0.2, 0.5, 0.3]);
        check_degenerate(&config::decode::<Defaults, f64>([0.3, -0.4], &masses));
    }
);

// ── Kind 2: the residual convention ───────────────────────────────────────────────────────────────────────────────

/// `m₀ = 0.4` entered on the `m₀` axis under conventions `A` and `B` at the slice `slice`: two `z`, and each decoded
/// state lands back at `m₀ = 0.4` on its own convention's chart and in latent `z` on the other's.
fn check_conventions<A: Residual, B: Residual>(slice: [f64; 8]) {
    let a: M0Chart<A, f64> = M0Chart::new(slice, M0_RANGE);
    let b: M0Chart<B, f64> = M0Chart::new(slice, M0_RANGE);
    let u = (0.4 - M0_RANGE[0]) / (M0_RANGE[1] - M0_RANGE[0]);
    let (za, zb) = (
        a.controls::<Defaults, f64>(u),
        b.controls::<Defaults, f64>(u),
    );
    assert!(
        (za[6] - zb[6]).abs() + (za[7] - zb[7]).abs() > 1e-3,
        "the two conventions give the same z ({za:?})"
    );
    for (z, own) in [(za, A::STATEMENT), (zb, B::STATEMENT)] {
        let x = Gauge::random(&mut rng(70), 2.0, 1.0).act(&decoded(z));
        assert!((x.m[0] - 0.4).abs() < 1e-15, "m₀ is {}", x.m[0]);
        let (on_a, on_b) = (into(&a, &x, TOL).landing, into(&b, &x, TOL).landing);
        let (mine, other) = if own == A::STATEMENT {
            (on_a, on_b)
        } else {
            (on_b, on_a)
        };
        let Landing::Chart { u: lu, .. } = mine else {
            panic!("m₀ = 0.4 under {own} does not round-trip under its own convention");
        };
        assert!((lu - u).abs() < 1e-12, "lands at u = {lu}, not {u}");
        assert_eq!(
            other,
            Landing::Latent,
            "m₀ = 0.4 under {own} round-trips under the other convention"
        );
        let e = encode::<Defaults, f64>(&x);
        assert!(
            (e.z.z[6] - z[6]).abs() < 1e-12 && (e.z.z[7] - z[7]).abs() < 1e-12,
            "the encoded mass block {:?} is not {own}'s",
            &e.z.z[6..8]
        );
    }
}

#[test]
fn encode_residual_convention() {
    check_conventions::<HoldRatio, HoldEqual>(SLICE);
    assert_ne!(
        HoldRatio::STATEMENT,
        HoldEqual::STATEMENT,
        "the conventions' labels"
    );
    // Each convention's masses: hold-ratio keeps the slice's m₁:m₂ and hold-equal sets m₁ = m₂, both summing to 1.
    let held = [0.2, 0.5, 0.3];
    let r = HoldRatio::masses(0.4, held);
    assert!(
        (r[1] / r[2] - 0.5 / 0.3).abs() < 1e-15 && (r.iter().sum::<f64>() - 1.0).abs() < 1e-15,
        "{r:?}"
    );
    assert_eq!(
        HoldEqual::masses(0.4, held),
        [0.4, 0.3, 0.3],
        "hold m₁ = m₂"
    );
}

negative_control!(
    encode_residual_convention,
    "a slice where the conventions coincide must fail the distinct-z check",
    expected = "the two conventions give the same z",
    {
        // `z_μ1 = z_μ2` holds m₁ = m₂ at the slice, so hold-ratio and hold-equal agree.
        let mut slice = SLICE;
        slice[7] = slice[6];
        check_conventions::<HoldRatio, HoldEqual>(slice);
    }
);

// ── The shape sphere's signed point (R-544; REQ-ENC-034) ───────────────────────────────────────────────────────────

/// A lookup onto the shape sphere: the chart point `(u, v)` of an encoded state.
type Lookup2 = fn(&ShapeSphere<f64>, &Encoded<f64>) -> (f64, f64);

/// The kernel's lookup, [`ByKind::coords`].
fn signed_lookup(chart: &ShapeSphere<f64>, e: &Encoded<f64>) -> (f64, f64) {
    match chart.coords::<Defaults, f64>(e) {
        [Coord::At(u), Coord::At(v)] => (u, v),
        c => panic!("the shape sphere placed no point: {c:?}"),
    }
}

/// `Π·x`, the full-state mirror.
fn mirrored(x: &Ic<f64>) -> Ic<f64> {
    kernel::canon::reflect(x, true)
}

/// `L_z = Σ rᵢ × pᵢ`.
fn angular(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// The shape sphere, spherical map, with the masses and momenta `slice` holds; `I = 1.3` and `φ_f = −0.4`.
fn sphere_holding(slice: [f64; 8]) -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.3,
        phase: -0.4,
        z_mass: [slice[6], slice[7]],
        z_mom: [slice[2], slice[3], slice[4], slice[5]],
    }
}

/// The exponential map about a point of the equator, so its cap straddles both hemispheres.
fn straddling() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0: [0.6, 0.8, 0.0],
        e1: [-0.8, 0.6, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 0.9,
        ..sphere_holding(SLICE)
    }
}

/// `lookup` on random generic states and their mirrors: `x` and `Π·x` land at `(θ, φ)` and `(θ, π − φ)`, `n₂` of
/// opposite signs, each the state's own signed shape within `shape_roundtrip_bound` (REQ-CHART-053).
fn check_signed_points(lookup: Lookup2) {
    let chart = sphere_holding(SLICE);
    let mut r = rng(0x0544);
    let mut lower = 0;
    for _ in 0..500 {
        let x = generic(&mut r);
        let (u, v) = lookup(&chart, &encode::<Defaults, f64>(&x));
        let (um, vm) = lookup(&chart, &encode::<Defaults, f64>(&mirrored(&x)));
        for (y, at) in [(x, (u, v)), (mirrored(&x), (um, vm))] {
            let n = chart.point(at.0, at.1);
            let own = shape(y.r, y.m);
            let d = (0..3).map(|k| (n[k] - own[k]).powi(2)).sum::<f64>().sqrt();
            assert!(
                d <= SHAPE_ROUNDTRIP_BOUND.number(),
                "the lookup lands at {at:?}, n {n:?}, not the state's signed shape {own:?} ({d:e} apart)"
            );
        }
        assert!(
            (um - u).abs() <= 1e-12 && (vm - (1.0 - v)).abs() <= 1e-12,
            "x lands at ({u}, {v}) and Π·x at ({um}, {vm}), not (θ, π − φ)"
        );
        lower += usize::from(shape(x.r, x.m)[2] < 0.0);
    }
    assert!(lower > 150, "{lower} of 500 states below the equator");
}

/// Decode then lookup on `chart`: every interior pixel of both hemispheres returns itself within `1e-12`, and lands
/// there in `‖·‖_phys` within `ε_phys`.
fn check_decode_lookup(chart: &ShapeSphere<f64>, lookup: Lookup2) -> usize {
    let mut lower = 0;
    for i in 1..20 {
        for j in 1..20 {
            let (u, v) = (f64::from(i) / 20.0, f64::from(j) / 20.0);
            let x = chart.decode_at::<Defaults, f64>(u, v).ic;
            let (lu, lv) = lookup(chart, &encode::<Defaults, f64>(&x));
            assert!(
                (lu - u).abs() <= 1e-12 && (lv - v).abs() <= 1e-12,
                "decode then lookup at ({u}, {v}) returns ({lu}, {lv})"
            );
            let e = encode_into::<Defaults, f64, _, _>(chart, &x, phys, eps_phys::<f64>());
            assert!(
                matches!(e.landing, Landing::Chart { .. }),
                "the decode at ({u}, {v}) lands in latent z, residual {:?}",
                e.residual
            );
            lower += usize::from(chart.point(u, v)[2] < 0.0);
        }
    }
    lower
}

/// The bits of a chart point.
fn coord_bits(c: [Coord<f64>; 2]) -> [Option<u64>; 2] {
    c.map(|k| match k {
        Coord::At(x) => Some(x.to_bits()),
        _ => None,
    })
}

/// T1 under the mirror on `chart`: `x` and `Π·x` give the same chart point, bit for bit.
fn check_t1<C: ByKind<Param = f64>>(name: &str, chart: &C, xs: &[Ic<f64>]) {
    for x in xs {
        let (a, b) = (
            encode::<Defaults, f64>(x),
            encode::<Defaults, f64>(&mirrored(x)),
        );
        assert!(
            coord_bits(chart.coords::<Defaults, f64>(&a))
                == coord_bits(chart.coords::<Defaults, f64>(&b)),
            "T1 fails on {name}: x and Π·x encode to two chart points"
        );
    }
}

/// T1 for `z` and every other chart, under the mirror, bit for bit: `E(Π·x)` and `E(x)` are one `z`, and every chart
/// but the shape sphere reads one point off it.
fn check_t1_others(xs: &[Ic<f64>]) {
    for x in xs {
        let (a, b) = (
            encode::<Defaults, f64>(x),
            encode::<Defaults, f64>(&mirrored(x)),
        );
        assert!(
            a.z.z.map(f64::to_bits) == b.z.z.map(f64::to_bits)
                && a.canonical.mirrored != b.canonical.mirrored,
            "T1 fails on z: x and Π·x encode to {:?} and {:?}",
            a.z.z,
            b.z.z
        );
    }
    for (n, c) in latent_charts().iter().enumerate() {
        check_t1(&format!("latent chart {n}"), c, xs);
    }
    check_t1("(α, p_ρ,x)", &mixed_ap(), xs);
    check_t1("(z_q1, β)", &mixed_qb(), xs);
    check_t1("(p_λ,y, z_μ1)", &mixed_lm(), xs);
    check_t1(
        "the mass simplex",
        &MassSimplex {
            nu0: 0.4,
            eps_m: 1e-4,
        },
        xs,
    );
    check_t1("(L_z, E)", &InvariantLE(frozen_invariant()), xs);
    check_t1("(L_z, K)", &InvariantLK(frozen_invariant()), xs);
    check_t1(
        "(ν, K)",
        &Burrau {
            nu_range: [0.05, 0.4],
            k_max: 1.0,
            gamma_k: 2.0,
        },
        xs,
    );
}

/// Where a lower-hemisphere state lands on the shape sphere: the kernel's [`encode_into`] at Part 4's norm and
/// `ε_phys`, or a control's.
type Land = fn(&ShapeSphere<f64>, &Ic<f64>) -> Landing<f64>;

/// The kernel's landing.
fn land(chart: &ShapeSphere<f64>, x: &Ic<f64>) -> Landing<f64> {
    encode_into::<Defaults, f64, _, _>(chart, x, phys, eps_phys::<f64>()).landing
}

/// The landing on the shape sphere chart (inverse_encode_contract Part 6, "On the shape sphere chart"): a
/// lower-hemisphere `x` with `p_y ≠ 0`, the chart's own decode, its masses and unmirrored Jacobi momenta the slice's,
/// and a turned, scaled, moved and boosted copy of it, land on the chart at its pixel and decode back to `x` within
/// `ε_phys` in `‖·‖_phys`, `L_z`'s sign kept; `Π·x`, the slice unchanged, lands in latent `z`. Returns the worst
/// residual in `ε`.
fn check_landing(land: Land) -> f64 {
    let chart = sphere_holding(SLICE);
    let mut r = rng(0x0545);
    let mut worst: f64 = 0.0;
    for _ in 0..200 {
        let (u, v) = (uniform(&mut r, 0.05, 0.95), uniform(&mut r, 0.05, 0.45));
        let x = chart.decode_at::<Defaults, f64>(u, v).ic;
        assert!(
            chart.point(u, v)[2] < 0.0 && x.p.iter().any(|p| p[1] != 0.0),
            "({u}, {v}) is not a lower-hemisphere state with p_y ≠ 0"
        );
        let g = Gauge {
            mirror: false,
            ..Gauge::random(&mut r, 2.0, 1.5)
        };
        for y in [x, g.act(&x)] {
            match land(&chart, &y) {
                Landing::Chart { u: lu, v: lv } => {
                    let back = chart.decode_at::<Defaults, f64>(lu, lv).ic;
                    let d = phys(&x, &back);
                    assert!(
                        (lu - u).abs() <= 1e-9 && (lv - v).abs() <= 1e-9 && d <= eps_phys::<f64>(),
                        "the state at ({u}, {v}) lands at ({lu}, {lv}) and decodes back {d:e} from itself"
                    );
                    assert!(
                        angular(&back).signum() == angular(&x).signum() && angular(&x) != 0.0,
                        "L_z's sign at ({u}, {v}) is not kept: {} against {}",
                        angular(&back),
                        angular(&x)
                    );
                    worst = worst.max(d / f64::EPSILON);
                }
                Landing::Latent => {
                    panic!("the lower-hemisphere state at ({u}, {v}) lands in latent z")
                }
            }
        }
        assert_eq!(
            land(&chart, &mirrored(&x)),
            Landing::Latent,
            "Π·x of the state at ({u}, {v}) lands on the chart, whose slice holds x's momenta"
        );
    }
    worst
}

#[test]
fn encode_shape_sphere_signed() {
    check_signed_points(signed_lookup);
    let spherical = check_decode_lookup(&sphere_holding(SLICE), signed_lookup);
    let exponential = check_decode_lookup(&straddling(), signed_lookup);
    assert!(
        spherical > 150 && exponential > 100,
        "decode then lookup visits {spherical} and {exponential} lower-hemisphere pixels"
    );
    let mut r = rng(0x0546);
    let xs: Vec<Ic<f64>> = (0..200).map(|_| generic(&mut r)).collect();
    check_t1_others(&xs);
    let worst = check_landing(land);
    // With a slice whose momenta are their own mirror images, Π·x is the state at (θ, π − φ) and lands there.
    let even = sphere_holding([SLICE[0], SLICE[1], 0.4, 0.0, -0.3, 0.0, SLICE[6], SLICE[7]]);
    let x = even.decode_at::<Defaults, f64>(0.3, 0.2).ic;
    match land(&even, &mirrored(&x)) {
        Landing::Chart { u, v } => assert!(
            (u - 0.3).abs() <= 1e-9 && (v - 0.8).abs() <= 1e-9,
            "Π·x lands at ({u}, {v}), not (0.3, 0.8)"
        ),
        Landing::Latent => panic!("Π·x, the slice holding its momenta, lands in latent z"),
    }
    eprintln!(
        "encode_shape_sphere_signed: {spherical} and {exponential} lower-hemisphere pixels return themselves; the \
         lower-hemisphere landing's worst residual {worst:.2} ε (ε_phys {} ε)",
        eps_phys::<f64>() / f64::EPSILON
    );
}

negative_control!(
    encode_shape_sphere_signed,
    "a lookup reading shape_vec of C(x) without undoing the mirror lands every lower-hemisphere state on the upper, so \
     it must fail",
    expected = "not the state's signed shape",
    {
        check_signed_points(|c, e| {
            let j = &e.blocks.jacobi;
            let pi = std::f64::consts::PI;
            match c.map {
                ShapeMap::Spherical => spherical_inverse(shape_vec(j.rho_t, j.lambda_t), pi),
                ShapeMap::Exponential => unreachable!("the control's chart is spherical"),
            }
        });
    }
);

mod signed_t1_on_the_sphere {
    use super::*;
    negative_control!(
        encode_shape_sphere_signed,
        "T1 applied to the shape sphere's coordinates, where x and Π·x are two points, must fail",
        expected = "T1 fails on the shape sphere",
        {
            let mut r = rng(0x0546);
            let xs: Vec<Ic<f64>> = (0..200).map(|_| generic(&mut r)).collect();
            check_t1("the shape sphere", &sphere_holding(SLICE), &xs);
        }
    );
}

mod signed_landing_folded {
    use super::*;
    negative_control!(
        encode_shape_sphere_signed,
        "a landing test against the mirrored C(x) sends every lower-hemisphere state to latent z, so it must fail",
        expected = "lands in latent z",
        {
            check_landing(|c, x| {
                let e = encode::<Defaults, f64>(x);
                let (u, v) = signed_lookup(c, &e);
                let d = phys(&e.canonical.ic, &c.decode_at::<Defaults, f64>(u, v).ic);
                if d <= eps_phys::<f64>() {
                    Landing::Chart { u, v }
                } else {
                    Landing::Latent
                }
            });
        }
    );
}
