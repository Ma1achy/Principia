//! QA's tests for TASK-M2-38, written from REQ-CHART-060 and REQ-CHART-061 (R-469 P2, A1 to A3, A6; R-471; R-477 C6;
//! R-512), not from the implementation.
//!
//! - **The edge to the bit** (REQ-CHART-060, A3): on a chart written here, one `nav_stop` has never seen (so the rule
//!   is generic over the `Chart` trait and `validate`, no chart carrying its own), whose `validate` fails past a wall
//!   placed at a chosen f64, a path whose fraction `λ` equals the coordinate stops at exactly the last valid f64, the
//!   next f64 failing, within 62 steps, for walls from the smallest subnormal to the f64 just below 1, and for walls
//!   drawn at random over the bit patterns of `(0, 1)`. Control: a bisection that halves `λ` itself (the corrected
//!   reading, physics review F4) misses a wall at small `λ`.
//! - **The report's layer** (A5, inverse_encode_contract § "Chart-aware validation"): layer 3 for decode sanity, layer
//!   2 for every chart-feasibility constraint, none where `Φ` fails; a centre with no view or a non-finite `Φ` counts
//!   as failing (A1).
//! - **The lock** (A2): refused on a pixel that is not a system, the centre unchanged; taken on one that is.
//! - **The latent chart** (A6): nothing stops, at any `z₀` or zoom.
//! - **Tilt** (A2): anchor-preserving, it passes with no centre edit, even from a centre on the edge.
//! - **The fuzz** (REQ-CHART-061, C6): seeded pans and locks through the stop on the charts whose centres can leave the
//!   manifold (the linear `(L_z, E)` and `(L_z, K)` charts, the mass simplex), 1000 per chart, the centre validating
//!   after every edit and the stop binding; control: the stop disabled reaches an invalid centre.
//! - **The 65 × 65 grid** (C6, R-471): warped, every pixel of the invariant charts validates, untilted and under Lean
//!   each axis; linear, infeasible pixels appear and each is classified not a system. Control: the linear grid read
//!   as all valid fails.

use kernel::chart::invariant::Invariant;
use kernel::chart::nav::{nav_stop, Check, Edit, Field, Stop, View};
use kernel::chart::{
    Chart, ChartOut, Constraint, Float, InvariantLE, InvariantLK, Latent, Mapped, Mapping,
    MassSimplex, RestStart, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::decode;
use kernel::generated::links::{Defaults, LinkParams};
use validation::chart_class::forbidden;
use validation::{negative_control, GAMMA_K, K_MAX, MASS_SIMPLEX_EPS_M};

const P: LinkParams<f64> = LinkParams::DEFAULT;

/// R-469 A3: the f64s of `[0, 1]` number fewer than 2⁶², so at most 62 halvings of their bit patterns' interval.
const MAX_STEPS: u32 = 62;

// ── A seeded generator (SplitMix64) ───────────────────────────────────────────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[lo, hi)`.
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

// ── The independent verdict, from the chart alone ─────────────────────────────────────────────────────────────────

/// What fails at a view's centre (A1): `Φ` where there is no view or its output is not finite, else `validate`'s
/// constraint (layers 2 and 3); none where the centre passes.
fn verdict<C: Chart<Param = f64>>(view: Option<View<C>>) -> Option<Check> {
    let view = match view {
        Some(v) => v,
        None => return Some(Check::Phi),
    };
    let [u, v] = view.centre;
    if view
        .chart
        .map::<f64>(u, v)
        .z
        .z
        .iter()
        .any(|x| !x.is_finite())
    {
        return Some(Check::Phi);
    }
    match view.chart.validate_params(u, v, &P) {
        ValidationResult::Valid => None,
        ValidationResult::Project { failed, .. }
        | ValidationResult::Clamp { failed, .. }
        | ValidationResult::Reject { failed } => Some(Check::Failed(failed)),
    }
}

// ── A chart nav_stop has never seen ───────────────────────────────────────────────────────────────────────────────

/// A chart written here: `validate` passes for `u ∈ [lo, hi]` and fails outside with `constraint`; with `nan`, `Φ`
/// returns NaN outside instead, `validate` passing everywhere.
#[derive(Clone, Copy, Debug)]
struct Wall {
    lo: f64,
    hi: f64,
    constraint: Constraint,
    nan: bool,
}

impl Wall {
    fn inside(&self, u: f64) -> bool {
        u >= self.lo && u <= self.hi
    }
}

impl Chart for Wall {
    type Param = f64;

    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        let uf: f64 = u.to_f64().unwrap_or(f64::NAN);
        let fill = if self.nan && !self.inside(uf) {
            F::nan()
        } else {
            v
        };
        ChartOut {
            z: decode::Latent::new([fill; 8]),
        }
    }

    fn name(&self) -> &str {
        "QA wall"
    }

    fn validate(&self, u: f64, _v: f64) -> ValidationResult {
        if self.nan || self.inside(u) {
            ValidationResult::Valid
        } else {
            ValidationResult::Reject {
                failed: self.constraint,
            }
        }
    }
}

fn wall(lo: f64, hi: f64, constraint: Constraint) -> Wall {
    Wall {
        lo,
        hi,
        constraint,
        nan: false,
    }
}

/// The view at centre `(x, ½)`.
fn wall_view(w: Wall) -> impl Fn(f64) -> Option<View<Wall>> {
    move |x| {
        Some(View {
            chart: w,
            centre: [x, 0.5],
        })
    }
}

/// REQ-CHART-060's stop, checked against the chart alone: on the path at its `λ`; its centre validates; where it
/// stopped, the next f64 above `λ` fails with the check reported, within 62 steps; where it did not, the value asked
/// for, taken whole.
fn assert_stop<V, C>(
    what: &str,
    from: V,
    to: V,
    view: &impl Fn(V) -> Option<View<C>>,
    stop: &Stop<V>,
) where
    V: Field + PartialEq + std::fmt::Debug,
    C: Chart<Param = f64>,
{
    assert!(
        stop.value == from.along(to, stop.lambda),
        "{what}: the stop is off the path"
    );
    assert_eq!(
        verdict(view(stop.value)),
        None,
        "{what}: the accepted centre {:?} fails validate",
        stop.value
    );
    assert!(stop.steps <= MAX_STEPS, "{what}: {} steps", stop.steps);
    match stop.stopped {
        None => assert!(
            stop.value == to && stop.lambda == 1.0,
            "{what}: not taken whole"
        ),
        Some(check) => {
            let next = from.along(to, stop.lambda.next_up());
            assert_eq!(
                verdict(view(next)),
                Some(check),
                "{what}: λ = {:e} is not the last valid f64",
                stop.lambda
            );
        }
    }
}

/// The edge cases and seeded walls; `stop` the stop under test.
/// A stop of a path on the wall chart: `nav_stop`, or a control's.
type WallStop = fn(Edit<f64>, &dyn Fn(f64) -> Option<View<Wall>>) -> Stop<f64>;

fn check_edge_to_the_bit(stop: WallStop) {
    let mut edges = vec![
        f64::from_bits(1),
        1e-300,
        f64::MIN_POSITIVE,
        1e-6,
        0.25,
        0.5,
        1.0f64.next_down(),
    ];
    let seed = 0x5EED_2038_u64;
    println!("seed {seed:#x}");
    let mut rng = Rng(seed);
    for _ in 0..300 {
        let bits = rng.next() % 1f64.to_bits();
        edges.push(f64::from_bits(bits.max(1)));
    }
    for edge in edges {
        let w = wall(f64::NEG_INFINITY, edge, Constraint::LzEParabola);
        let view = wall_view(w);
        // from 0 to 1, the coordinate is the fraction itself, so the last valid λ is the wall.
        let s = stop(Edit::Path { from: 0.0, to: 1.0 }, &view);
        assert_stop("wall", 0.0, 1.0, &view, &s);
        assert_eq!(
            s.lambda, edge,
            "the stop at λ = {:e} is not the wall {edge:e}",
            s.lambda
        );
        assert_eq!(s.value, edge);
        assert_eq!(s.stopped, Some(Check::Failed(Constraint::LzEParabola)));
        // The reverse direction: from 1 toward 0 with the wall below.
        let w = wall(edge, f64::INFINITY, Constraint::MassBuffer);
        let view = wall_view(w);
        let s = stop(Edit::Path { from: 1.0, to: 0.0 }, &view);
        assert_stop("reverse wall", 1.0, 0.0, &view, &s);
        assert!(s.value >= edge && s.stopped == Some(Check::Failed(Constraint::MassBuffer)));
    }
}

fn real_stop(edit: Edit<f64>, view: &dyn Fn(f64) -> Option<View<Wall>>) -> Stop<f64> {
    nav_stop(edit, view, &P)
}

#[test]
fn qa_nav_stop_at_edge_to_the_bit() {
    check_edge_to_the_bit(real_stop);
}

/// The reading physics review F4 corrected: bisection halving `λ` itself, 64 steps. Used only by the control.
#[cfg(feature = "controls")]
fn halving_lambda(edit: Edit<f64>, view: &dyn Fn(f64) -> Option<View<Wall>>) -> Stop<f64> {
    let Edit::Path { from, to } = edit else {
        unreachable!()
    };
    let (mut lo, mut hi, mut failed) = (0.0f64, 1.0f64, verdict(view(to)));
    for _ in 0..64 {
        let mid = 0.5 * (lo + hi);
        match verdict(view(from.along(to, mid))) {
            None => lo = mid,
            Some(c) => {
                hi = mid;
                failed = Some(c);
            }
        }
    }
    Stop {
        value: from.along(to, lo),
        lambda: lo,
        // Reported within the bound, so the control isolates the adjacency check.
        steps: MAX_STEPS,
        stopped: failed,
    }
}

negative_control!(
    qa_nav_stop_at_edge_to_the_bit,
    "a bisection that halves λ itself never reaches adjacent f64s at a wall near λ = 0",
    expected = "is not the last valid f64",
    check_edge_to_the_bit(halving_lambda)
);

// ── The report's layer, and Φ ─────────────────────────────────────────────────────────────────────────────────────

/// inverse_encode_contract § "Chart-aware validation": layer 3, decode sanity; layer 2, chart feasibility.
const LAYERS: [(Constraint, u8); 14] = [
    (Constraint::LzEParabola, 2),
    (Constraint::KNonNegative, 2),
    (Constraint::KAboveLzBound, 2),
    (Constraint::MassPositive, 2),
    (Constraint::MassSum, 2),
    (Constraint::MassBuffer, 2),
    (Constraint::ShapeTheta, 2),
    (Constraint::ShapePhi, 2),
    (Constraint::BurrauNu, 2),
    (Constraint::AxisDomain, 2),
    (Constraint::BurrauTheta, 2),
    (Constraint::DecodedMassPositive, 3),
    (Constraint::ComAtOrigin, 3),
    (Constraint::MomentumZero, 3),
];

fn check_layers(table: &[(Constraint, u8)]) {
    for &(c, layer) in table {
        let view = wall_view(wall(f64::NEG_INFINITY, 0.3, c));
        let s = nav_stop(Edit::Path { from: 0.0, to: 1.0 }, &view, &P);
        assert_stop("layer", 0.0, 1.0, &view, &s);
        let check = s.stopped.expect("the wall stops the edit");
        assert_eq!(check, Check::Failed(c));
        assert_eq!(check.layer(), Some(layer), "{c:?}: the report's layer");
    }
}

#[test]
fn qa_nav_stop_at_edge_reports_layer() {
    check_layers(&LAYERS);
}

negative_control!(
    qa_nav_stop_at_edge_reports_layer,
    "decode sanity read as layer 2 is caught",
    expected = "the report's layer",
    check_layers(&[(Constraint::DecodedMassPositive, 2)])
);

#[test]
fn qa_nav_stop_at_edge_phi_fails() {
    // Φ non-finite past the wall, validate passing everywhere.
    let w = Wall {
        lo: f64::NEG_INFINITY,
        hi: 0.7,
        constraint: Constraint::LzEParabola,
        nan: true,
    };
    let view = wall_view(w);
    let s = nav_stop(Edit::Path { from: 0.1, to: 0.9 }, &view, &P);
    assert_stop("Φ non-finite", 0.1, 0.9, &view, &s);
    assert_eq!(s.stopped, Some(Check::Phi));
    assert_eq!(Check::Phi.layer(), None);
    assert!(
        s.value <= 0.7 && 0.7 - s.value < 1e-15,
        "the stop at {} is not at the wall",
        s.value
    );
    // No view can be built past the wall.
    let none = |x: f64| {
        (x <= 0.4).then_some(View {
            chart: wall(f64::NEG_INFINITY, f64::INFINITY, Constraint::MassSum),
            centre: [x, 0.5],
        })
    };
    let s = nav_stop(Edit::Path { from: 0.0, to: 1.0 }, none, &P);
    assert_stop("no view", 0.0, 1.0, &none, &s);
    assert_eq!(s.stopped, Some(Check::Phi));
    assert_eq!(s.lambda, 0.4);
}

// ── The lock ──────────────────────────────────────────────────────────────────────────────────────────────────────

type LeMapped = Mapped<InvariantLE<f64>, Defaults>;
type LkMapped = Mapped<InvariantLK<f64>, Defaults>;

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, 0.6],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

fn le(mapping: Mapping) -> LeMapped {
    Mapped::new(InvariantLE(invariant()), [mapping; 2])
}

fn lk(mapping: Mapping) -> LkMapped {
    Mapped::new(InvariantLK(invariant()), [mapping; 2])
}

fn at<C>(chart: C) -> impl Fn([f64; 2]) -> Option<View<C>>
where
    C: Copy,
{
    move |c| Some(View { chart, centre: c })
}

/// A lock, or with `lock` false a pan (the control's).
fn lock_or_pan<V>(lock: bool, from: V, to: V) -> Edit<V> {
    if lock {
        Edit::Lock { from, to }
    } else {
        Edit::Path { from, to }
    }
}

fn check_lock(lock: bool) {
    let edit = |from, to| lock_or_pan(lock, from, to);
    // The wall chart: a lock past the wall is refused, the centre unchanged.
    let view = wall_view(wall(f64::NEG_INFINITY, 0.5, Constraint::MassPositive));
    let s = nav_stop(edit(0.2, 0.9), &view, &P);
    assert!(
        s.value == 0.2 && s.stopped == Some(Check::Failed(Constraint::MassPositive)),
        "a lock on a pixel that is not a system is not refused: {s:?}"
    );
    let s = nav_stop(edit(0.2, 0.45), &view, &P);
    assert_eq!(s.value, 0.45, "a lock on a system is taken");
    // The linear (L_z, E) chart: a pixel far outside the parabola, then a feasible one.
    let v = at(le(Mapping::Linear));
    let from = [0.5, 0.6];
    let forbidden_px = [0.02, 0.1];
    assert!(
        verdict(v(forbidden_px)).is_some(),
        "the fixture's pixel is forbidden"
    );
    let s = nav_stop(lock_or_pan(lock, from, forbidden_px), &v, &P);
    assert!(
        s.value == from && s.stopped.is_some(),
        "a lock on a pixel that is not a system is not refused: {s:?}"
    );
}

#[test]
fn qa_nav_stop_at_edge_lock() {
    check_lock(true);
}

negative_control!(
    qa_nav_stop_at_edge_lock,
    "a lock passed as a pan is stopped part-way, not refused with the centre unchanged",
    expected = "is not refused",
    check_lock(false)
);

// ── The latent chart: nothing stops ───────────────────────────────────────────────────────────────────────────────

fn check_latent<C: Chart<Param = f64> + Copy>(other: Option<C>) {
    let mut rng = Rng(0x001A_7E47_u64);
    let draw = |rng: &mut Rng, s: f64| core::array::from_fn::<f64, 8, _>(|_| rng.range(-s, s));
    for i in 0..200 {
        let q1 = draw(&mut rng, 3.0);
        let q2 = draw(&mut rng, 3.0);
        let from = draw(&mut rng, 5.0);
        let to = draw(&mut rng, if i % 2 == 0 { 50.0 } else { 5.0 });
        let view = |z0: [f64; 8]| {
            Some(View {
                chart: Latent { z0, q1, q2 },
                centre: [0.5, 0.5],
            })
        };
        let s = nav_stop(Edit::Path { from, to }, view, &P);
        assert!(
            s.stopped.is_none() && s.value == to && s.steps == 0,
            "a pan on the latent chart stopped: {s:?}"
        );
        // Zoom about the centre, in and out by 10⁶.
        let zoom = |k: f64| {
            Some(View {
                chart: Latent { z0: from, q1, q2 }.zoomed(k),
                centre: [0.5, 0.5],
            })
        };
        for to in [1e-6, 1e6] {
            let s = nav_stop(Edit::Path { from: 1.0, to }, zoom, &P);
            assert!(
                s.stopped.is_none() && s.value == to && s.steps == 0,
                "a zoom stopped: {s:?}"
            );
        }
    }
    if let Some(chart) = other {
        let v = at(chart);
        let s = nav_stop(
            Edit::Path {
                from: [0.5, 0.5],
                to: [1.5, 0.5],
            },
            &v,
            &P,
        );
        assert!(
            s.stopped.is_none(),
            "a pan on the latent chart stopped: {s:?}"
        );
    }
}

#[test]
fn qa_nav_stop_at_edge_latent_nothing_stops() {
    check_latent::<LeMapped>(None);
}

negative_control!(
    qa_nav_stop_at_edge_latent_nothing_stops,
    "the same check on a linear (L_z, E) chart sees a stop",
    expected = "on the latent chart stopped",
    check_latent(Some(le(Mapping::Linear)))
);

// ── Tilt passes with no centre edit ───────────────────────────────────────────────────────────────────────────────

fn check_tilt(centre_shift: f64) {
    let chart = le(Mapping::Linear);
    // A centre on the edge: the pan toward s = 1 stopped there.
    let edge = nav_stop(
        Edit::Path {
            from: [0.5, 0.5],
            to: [1.0, 0.5],
        },
        at(chart),
        &P,
    );
    assert!(edge.stopped.is_some());
    let c = edge.value;
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let view = |w: [f64; 3]| {
            let spec = TiltSpec {
                tau1: w[0],
                tau2: w[1],
                map,
                x: 6,
                y: 6,
            };
            let centre = [w[2], c[1]];
            Tilted::new(chart, Tilt::new(&spec, centre, [2.0, 2.0]))
                .map(|t| View { chart: t, centre })
        };
        let from = [0.0, 0.0, c[0]];
        for to in [
            [25.0, -40.0, c[0] + centre_shift],
            [-70.0, 60.0, c[0] + centre_shift],
        ] {
            let s = nav_stop(Edit::Path { from, to }, view, &P);
            assert!(
                s.stopped.is_none() && s.value == to,
                "a tilt under {map:?} to {to:?} did not pass: {s:?}"
            );
        }
    }
}

#[test]
fn qa_nav_stop_at_edge_tilt_passes() {
    check_tilt(0.0);
}

negative_control!(
    qa_nav_stop_at_edge_tilt_passes,
    "a tilt that comes with a centre edit past the edge stops",
    expected = "did not pass",
    check_tilt(0.05)
);

// ── REQ-CHART-061: the fuzz ───────────────────────────────────────────────────────────────────────────────────────

/// ≥ 1000 edits per chart (REQ-CHART-061's verify).
const EDITS: u32 = 1000;

/// Seeded pans (three in four) and locks from a valid centre to one drawn over `[−0.5, 1.5]²`, each through the stop,
/// or taken whole with `disabled`; the centre must validate after each. Returns (stops, refusals).
fn fuzz<C: Chart<Param = f64> + Copy>(
    name: &str,
    chart: C,
    start: [f64; 2],
    seed: u64,
    disabled: bool,
) -> (u32, u32) {
    println!("{name}: seed {seed:#x}, {EDITS} edits");
    let view = at(chart);
    assert_eq!(verdict(view(start)), None, "{name}: the start is valid");
    let mut rng = Rng(seed);
    let mut centre = start;
    let (mut stops, mut refusals) = (0, 0);
    for i in 0..EDITS {
        let to = [rng.range(-0.5, 1.5), rng.range(-0.5, 1.5)];
        let lock = rng.next().is_multiple_of(4);
        let edit = if lock {
            Edit::Lock { from: centre, to }
        } else {
            Edit::Path { from: centre, to }
        };
        let s = if disabled {
            Stop {
                value: to,
                lambda: 1.0,
                steps: 0,
                stopped: None,
            }
        } else {
            nav_stop(edit, &view, &P)
        };
        if s.stopped.is_some() {
            if s.value == centre {
                refusals += 1;
            } else {
                stops += 1;
            }
        }
        centre = s.value;
        assert_eq!(
            verdict(view(centre)),
            None,
            "{name}: edit {i} left the manifold at {centre:?}"
        );
    }
    println!("{name}: {stops} stopped, {refusals} refused");
    (stops, refusals)
}

fn check_fuzz(disabled: bool) {
    let (s, r) = fuzz(
        "(L_z, E) linear",
        le(Mapping::Linear),
        [0.5, 0.5],
        0xC6_0001,
        disabled,
    );
    assert!(s > 0 && r > 0, "the fuzz never reached the edge");
    let (s, r) = fuzz(
        "(L_z, K) linear",
        lk(Mapping::Linear),
        [0.5, 0.5],
        0xC6_0002,
        disabled,
    );
    assert!(s > 0 && r > 0, "the fuzz never reached the edge");
    let simplex = RestStart::new(MassSimplex {
        nu0: 0.5,
        eps_m: MASS_SIMPLEX_EPS_M.number(),
    });
    let (s, r) = fuzz("mass simplex", simplex, [0.4, 0.3], 0xC6_0003, disabled);
    assert!(s > 0 && r > 0, "the fuzz never reached the edge");
    // And on the mass simplex every decoded mass stays positive at the final centre's neighbours too: checked per edit
    // by validate above; check the decode once more at fresh valid stops.
    let view = at(simplex);
    let mut rng = Rng(0xC6_0004);
    for _ in 0..200 {
        let to = [rng.range(-1.0, 2.0), rng.range(-1.0, 2.0)];
        let s = nav_stop(
            Edit::Path {
                from: [0.4, 0.3],
                to,
            },
            &view,
            &P,
        );
        let m = simplex.decode::<Defaults, f64>(s.value[0], s.value[1]).ic.m;
        assert!(
            m.iter().all(|&m| m > 0.0),
            "a stop at {:?} has masses {m:?}",
            s.value
        );
    }
}

#[test]
fn qa_nav_fuzz_never_leaves() {
    check_fuzz(false);
}

negative_control!(
    qa_nav_fuzz_never_leaves,
    "the fuzz with the stop disabled reaches an invalid centre",
    expected = "left the manifold",
    check_fuzz(true)
);

// ── REQ-CHART-061: the 65 × 65 grid, warped and linear ────────────────────────────────────────────────────────────

const N: u32 = 65;

fn grid() -> impl Iterator<Item = [f64; 2]> {
    (0..N).flat_map(|j| (0..N).map(move |i| [i as f64 / (N - 1) as f64, j as f64 / (N - 1) as f64]))
}

/// Warped, every grid pixel validates; linear, infeasible pixels appear and each is classified not a system.
/// `linear_all_valid` asserts the linear grid all valid instead (the control).
fn check_grid(linear_all_valid: bool) {
    fn all_valid<C: Chart<Param = f64>>(name: &str, chart: &C) {
        for [u, v] in grid() {
            assert_eq!(
                chart.validate_params(u, v, &P),
                ValidationResult::Valid,
                "{name}: pixel ({u}, {v}) fails"
            );
            assert_eq!(
                forbidden(chart, u, v),
                Some(false),
                "{name}: pixel ({u}, {v}) not a system"
            );
        }
    }
    all_valid("(L_z, E) warped", &le(Mapping::Warped));
    all_valid("(L_z, K) warped", &lk(Mapping::Warped));
    // Under Lean each axis with one target every grid pixel stays in the square (R-512), so all must validate.
    for (t1, t2) in [(30.0, -45.0), (-80.0, 10.0), (60.0, 0.0)] {
        let spec = TiltSpec {
            tau1: t1,
            tau2: t2,
            map: TiltMap::Lean,
            x: 6,
            y: 6,
        };
        let tilt = Tilt::new(&spec, [0.5, 0.5], [2.0, 2.0]);
        let t = Tilted::new(le(Mapping::Warped), tilt).expect("a hidden target");
        all_valid("(L_z, E) warped, Lean", &t);
        let t = Tilted::new(lk(Mapping::Warped), tilt).expect("a hidden target");
        all_valid("(L_z, K) warped, Lean", &t);
    }
    for (name, infeasible) in [
        ("(L_z, E) linear", linear_grid(&le(Mapping::Linear))),
        ("(L_z, K) linear", linear_grid(&lk(Mapping::Linear))),
    ] {
        println!("{name}: {infeasible} of {} grid pixels not a system", N * N);
        if linear_all_valid {
            assert_eq!(
                infeasible, 0,
                "{name}: the linear grid has pixels that are not a system"
            );
        } else {
            assert!(infeasible > 0, "{name}: no infeasible pixel inside [0,1]²");
        }
    }
}

/// The count of grid pixels that fail validate, each asserted classified not a system.
fn linear_grid<C: Chart<Param = f64>>(chart: &C) -> u32 {
    let mut n = 0;
    for [u, v] in grid() {
        let valid = chart.validate_params(u, v, &P) == ValidationResult::Valid;
        if !valid {
            n += 1;
        }
        assert_eq!(
            forbidden(chart, u, v),
            Some(!valid),
            "pixel ({u}, {v}): the classification disagrees with validate"
        );
    }
    n
}

#[test]
fn qa_nav_fuzz_never_leaves_grid() {
    check_grid(false);
}

negative_control!(
    qa_nav_fuzz_never_leaves_grid,
    "the linear grid read as all systems fails",
    expected = "has pixels that are not a system",
    check_grid(true)
);
