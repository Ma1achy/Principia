//! The virial pair (design `principia_chart_composition.md` § 3's `Q` row, § 9, § 12's `virial_descriptor`;
//! chart_reference §2.1): the virial axis's kind-3 descriptor, its refusals, its forward by mapping and the round trip
//! through the chart's own decode, its label, mark and readout. Each test states its criterion here, written before
//! the first run, and has a control that fails when the property is broken.
//!
//! - `virial_descriptor`: the validator takes `(α, Q)` at a held `L_z ≠ 0`, its vertical axis kind 3, `Q`, solved into
//!   the momenta, reading configuration and masses, not conserved along the flow; it refuses a view's energy target
//!   (hold `E` on the view), and on the registry's path refuses `(α, Q)` under hold `E` (`AxisAndHold`) and `(K, Q)`
//!   (`SameQuantity`); `(α, Q)` lowers to its baked entry. Labels: `Q above the floor` warped at a held `L_z ≠ 0` with
//!   `Q₀ = 0`; `Q` at `Q₀ > 0`, at a held `L_z = 0` and linear. At `Q₀ > 0`, on the crossing chart below, a bottom-row
//!   pixel whose column has `Q_min(U) > Q₀·(1 + 10⁻³)` is marked and one with `Q_min(U) < Q₀·(1 − 10⁻³)` is not, at
//!   least 8 of each; the readout is the state's own `2K/|U|` exactly, and at a marked pixel the axis reads it less
//!   `Q_min(U) − Q₀`. Control: the pair with no descriptor for its virial axis (a kind-2 stand-in in the momenta).
//! - `virial_descriptor_round_trip` and its siblings: `forward(inverse(x)) = x`, the inverse the chart's own decode at
//!   `(u, t)`, `x` its axis value `scale(t)`, on a 65 × 65 grid at f64 and at f32 (the chart built and decoded at that
//!   precision, the state widened to f64), on `α` (the baked chart) and on `z_α`, over the arms: linear
//!   (`[−0.5, 2]`, held 0.2, systems only, the forward also against the decode's own `virial_ratio`); warped `Q₀ = 0`
//!   at held 0 and at held 0.15; warped `Q₀ = 1` at a held `L_z` putting `Q_min(U) = Q₀` at the column `u = ½`
//!   (columns below, at and above `Q₀`: at least 8 below and 8 above on the bottom row, and one within 10⁻³) and at one
//!   putting `Q_min(U) = 0.25` there, where `(½, 0)` reads 1; warped `Q₀ = 0.4` at held 0.
//!   **Criterion:** `|forward(inverse(x)) − x| ≤ 64·eps·(|x| + Q_min(U) + Q₀)`, eps the chart's precision's, in the
//!   ratio's own scale because the forward cancels `K` against the floor near `t = 0`, where `x` itself may be 0.
//!   The budget: the construction's `K` to 8 eps, `U`, `I` and `K` re-evaluated (16), the floor through `L_z` (2 × 4),
//!   doubled and rounded up. Holds read back: `|L_z − held| ≤ 64·eps·(|held| + Σ|rᵢ ∧ pᵢ|)`. The measured worst is
//!   printed per precision.
//!   Controls, one per sibling: a factor changed in the inverse (`Q₀·|U|` for `Q₀·|U|/2`, on `z_α`); the warped row
//!   read as the state's ratio at `Q₀ = 0`; the `Q₀ = 0` form on a `Q₀ > 0` axis where `Q₀ ≥ Q_min(U)`; the row's own
//!   inverse, a scale solve, in place of the decode.

use std::fmt::Debug;

use engine::chart_authoring::axis::{AxisKind, Block, InvariantQuantity};
use engine::chart_authoring::lowering::{resolve_spec, Lowering};
use engine::chart_authoring::registry::{AxisSpec, ChartSpec, Registry};
use engine::chart_authoring::validate::{validate, validate_spec, Refusal};
use engine::chart_authoring::virial::{floor, ratio, VirialAxis};
use engine::chart_authoring::{Authored, AxisDescriptor, ChartDescriptor, Residual};
use kernel::chart::energy::{decode_rule, Mapping, Rule};
use kernel::chart::invariant::{self, WarpReal};
use kernel::chart::mixed::{Alpha, Quantity, ZAlpha};
use kernel::chart::physical::{AxisVirial, MixedEnergy};
use kernel::chart::{Chart, ChartOut, Float, ValidationResult};
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::{Defaults, LinkParams, LinkReal};
use kernel::payload::PayloadReal;
use kernel::stage::decode::{DecodeVariant, MixedAlphaVirialDefaults};
use validation::negative_control;

/// The grid side: 65 points `i/64` per axis, edges included.
const N: u32 = 65;
/// The round trip's tolerance, in eps of the chart's precision times `|x| + Q_min(U) + Q₀` (the module's note).
const TOL: f64 = 64.0;
/// The slice centre (test data).
const Z0: [f64; 8] = [-0.2, 0.3, 0.1, -0.2, 0.05, 0.15, 0.5, -0.3];
/// The configuration axis's range: `α` on `[0.2, 1.4]`, and `z_α` on `[−1.5, 1.5]` (test data).
const ALPHA: [f64; 2] = [0.2, 1.4];
const Z_ALPHA: [f64; 2] = [-1.5, 1.5];
/// `γ_K` (test data).
const GAMMA: f64 = 2.0;
/// The asked range with a bottom `Q₀ = 1` (test data).
const ASKED: [f64; 2] = [1.0, 2.5];

/// What a run breaks, for the controls.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Break {
    None,
    /// The virial axis described as a kind-2 axis in the momenta, no invariant.
    NoDescriptor,
    /// `Q₀·|U|` for `Q₀·|U|/2` in the inverse, on `z_α`.
    FactorInInverse,
    /// The warped row at `Q₀ = 0` read as the state's own ratio.
    StateRatioAtZero,
    /// `Q₀ + 2(K − L_z²/2I)/|U|`, the `Q₀ = 0` form, on a `Q₀ > 0` axis.
    ZeroFormAtPositive,
    /// The row's own inverse, a scale solve, for the decode.
    ScaleSolve,
}

/// The scalar a chart is built and decoded at.
trait Prec: WarpReal + LinkReal + PayloadReal + Debug + Copy + 'static
where
    f64: From<Self>,
{
    const EPS: f64;
    fn of(x: f64) -> Self;
}

impl Prec for f64 {
    const EPS: f64 = f64::EPSILON;
    fn of(x: f64) -> Self {
        x
    }
}

impl Prec for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn of(x: f64) -> Self {
        x as f32
    }
}

type Virial<A, R> = MixedEnergy<A, AxisVirial, Defaults, R>;

fn chart<A: Quantity, R: Prec>(
    range: [f64; 2],
    asked: [f64; 2],
    held: f64,
    m: Mapping,
) -> Virial<A, R>
where
    f64: From<R>,
{
    Virial::<A, R>::virial(
        Z0.map(R::of),
        range.map(R::of),
        asked.map(R::of),
        R::of(GAMMA),
        R::of(held),
        m,
    )
    .expect("an offered range")
}

/// The state widened to f64.
fn wide<R: Prec>(d: &Decoded<R>) -> Ic<f64>
where
    f64: From<R>,
{
    Ic {
        m: d.ic.m.map(f64::from),
        r: d.ic.r.map(|v| v.map(f64::from)),
        p: d.ic.p.map(|v| v.map(f64::from)),
    }
}

fn lz_and_scale(ic: &Ic<f64>) -> (f64, f64) {
    let a = |j: usize| ic.r[j][0] * ic.p[j][1] - ic.r[j][1] * ic.p[j][0];
    let s = |j: usize| (ic.r[j][0] * ic.p[j][1]).abs() + (ic.r[j][1] * ic.p[j][0]).abs();
    ((0..3).map(a).sum(), (0..3).map(s).sum())
}

fn inertia(ic: &Ic<f64>) -> f64 {
    let total: f64 = ic.m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|j| ic.m[j] * ic.r[j][a]).sum::<f64>() / total);
    (0..3)
        .map(|j| ic.m[j] * ((ic.r[j][0] - c[0]).powi(2) + (ic.r[j][1] - c[1]).powi(2)))
        .sum()
}

fn potential(ic: &Ic<f64>) -> f64 {
    let mut u = 0.0;
    for a in 0..3 {
        for b in (a + 1)..3 {
            let s = (ic.r[a][0] - ic.r[b][0]).hypot(ic.r[a][1] - ic.r[b][1]);
            u -= ic.m[a] * ic.m[b] / s;
        }
    }
    u
}

/// `Q_min(U) = L_z²/(I·|U|)` at the held `L_z`, on the state's own `U` and `I`.
fn q_min(ic: &Ic<f64>, held: f64) -> f64 {
    held * held / (inertia(ic) * potential(ic).abs())
}

fn grid() -> Vec<[f64; 2]> {
    let at = |i: u32| f64::from(i) / f64::from(N - 1);
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [at(i), at(j)]))
        .collect()
}

/// `|U|` at the column `u = ½`, any row (the configuration does not read the momenta).
fn mid_u<A: Quantity, R: Prec>(range: [f64; 2]) -> f64
where
    f64: From<R>,
{
    let c = chart::<A, R>(range, [0.0, 1.0], 0.0, Mapping::Warped);
    potential(&wide(&c.decode::<Defaults, R>(R::of(0.5), R::zero()))).abs()
}

/// The inverse at `(u, t)`: the chart's decode, or what the control puts in its place.
fn inverse<A: Quantity, R: Prec>(c: &Virial<A, R>, u: R, t: R, b: Break) -> Decoded<R>
where
    f64: From<R>,
{
    let q0 = Float::max(c.bottom, R::zero());
    let under = c.mapping == Mapping::Warped && q0 > R::zero() && c.held != R::zero();
    match b {
        Break::FactorInInverse if under && !A::DERIVED => {
            let two = R::one() + R::one();
            let rule = Rule {
                k0: invariant::k_floor(c.held),
                weight: invariant::kinetic(c.top - q0, c.gamma_k, t) / two,
                bottom_weight: q0,
                u_ref: R::zero(),
                s2: R::zero(),
                lz: c.held,
                lz_on_warp: false,
            };
            decode_rule::<Defaults, R>(rule, &c.map(u, t).z, &LinkParams::DEFAULT).0
        }
        Break::ScaleSolve => {
            // The row's own inverse from the row t' = 1 − t's state, to the ratio the axis's line sets at t.
            let base = c.decode::<Defaults, R>(u, R::one() - t);
            let row = Registry::standard();
            let set = row
                .row("Q")
                .expect("the Q row")
                .invert
                .expect("its inverse");
            let ic = wide(&base);
            let k_line = f64::from(c.scale(t)) * potential(&ic).abs() / 2.0
                + f64::from(invariant::k_floor(c.held));
            let target = 2.0 * k_line / potential(&ic).abs();
            let moved = set(&ic, target).unwrap_or(ic);
            let mut d = base;
            d.ic.p = moved.p.map(|v| v.map(R::of));
            d
        }
        _ => c.decode::<Defaults, R>(u, t),
    }
}

/// The axis's coordinate, or what the control reads in its place.
fn forward(axis: &VirialAxis, ic: &Ic<f64>, b: Break) -> f64 {
    let warped = axis.mapping == Mapping::Warped && axis.held != 0.0;
    let u = potential(ic).abs();
    match b {
        Break::StateRatioAtZero if warped && axis.q0 == 0.0 => ratio(ic),
        Break::ZeroFormAtPositive if warped && axis.q0 > 0.0 => {
            axis.q0 + 2.0 * (engine::chart_authoring::registry::kinetic(ic) - floor(ic)) / u
        }
        _ => axis.forward(ic),
    }
}

/// The round trip on one chart (the module's note); returns the worst `|forward(inverse(x)) − x|` in its tolerance's
/// units, and the bottom row's columns below, at and above `Q₀`.
fn round_trip<A: Quantity, R: Prec>(c: &Virial<A, R>, b: Break) -> (f64, [u32; 3])
where
    f64: From<R>,
{
    let eps = R::EPS;
    let name = format!("{} {}", A::NAME, core::any::type_name::<R>());
    let axis = VirialAxis::of(c);
    let held = axis.held;
    let (mut worst, mut sides) = (0.0f64, [0u32; 3]);
    for [u, t] in grid() {
        let (u, t) = (R::of(u), R::of(t));
        let d = inverse(c, u, t, b);
        if d.degenerate.is_some() {
            assert!(
                axis.mapping == Mapping::Linear,
                "{name}: the warped pixel ({u:?}, {t:?}) is not a system"
            );
            continue;
        }
        let ic = wide(&d);
        // The hold first: a scale solve moves L_z, which is what its control must show.
        let (lz, lz_scale) = lz_and_scale(&ic);
        assert!(
            (lz - held).abs() <= TOL * eps * (held.abs() + lz_scale),
            "{name}: ({u:?}, {t:?}) has L_z = {lz}, not the held {held}"
        );
        let x = f64::from(c.scale(t));
        let qm = q_min(&ic, held);
        let got = forward(&axis, &ic, b);
        let scale = x.abs() + qm + axis.q0;
        worst = worst.max((got - x).abs() / (eps * scale));
        assert!(
            (got - x).abs() <= TOL * eps * scale,
            "{name}: {:?} Q₀ = {}, held {held}, ({u:?}, {t:?}): forward(inverse(x)) = {got}, not x = {x}",
            axis.mapping,
            axis.q0
        );
        if axis.mapping == Mapping::Linear {
            let own = f64::from(d.descriptor.virial_ratio);
            assert!(
                (got - own).abs() <= TOL * eps * own.abs(),
                "{name}: the linear forward {got} is not the decode's virial_ratio {own}"
            );
        }
        if t == R::zero() && axis.q0 > 0.0 {
            let side = if (qm - axis.q0).abs() <= 1e-3 * axis.q0 {
                1
            } else if qm < axis.q0 {
                0
            } else {
                2
            };
            sides[side] += 1;
        }
    }
    (worst, sides)
}

/// Every arm on the configuration axis `A` over `range`, at the precision `R`; the worst in the tolerance's units.
fn arms<A: Quantity, R: Prec>(range: [f64; 2], b: Break) -> f64
where
    f64: From<R>,
{
    let name = format!("{} {}", A::NAME, core::any::type_name::<R>());
    let mut worst = 0.0f64;
    let mut run = |asked: [f64; 2], held: f64, m: Mapping| {
        let c = chart::<A, R>(range, asked, held, m);
        let (w, sides) = round_trip(&c, b);
        worst = worst.max(w);
        (c, sides)
    };
    run([-0.5, 2.0], 0.2, Mapping::Linear);
    run([0.0, 2.0], 0.0, Mapping::Warped);
    run([0.0, 2.0], 0.15, Mapping::Warped);
    run([0.4, 2.0], 0.0, Mapping::Warped);
    let mid = mid_u::<A, R>(range);
    let (_, sides) = run(ASKED, (ASKED[0] * mid).sqrt(), Mapping::Warped);
    assert!(
        sides[0] >= 8 && sides[1] >= 1 && sides[2] >= 8,
        "{name}: the bottom row's columns below, at and above Q₀ are {sides:?}, not at least [8, 1, 8]"
    );
    let (c, _) = run(ASKED, (0.25 * mid).sqrt(), Mapping::Warped);
    // The example: Q₀ = 1 on the column where Q_min(U) = 0.25 reads 1 at t = 0.
    let ic = wide(&inverse(&c, R::of(0.5), R::zero(), b));
    let got = forward(&VirialAxis::of(&c), &ic, b);
    assert!(
        (q_min(&ic, f64::from(c.held)) - 0.25).abs() <= 1e-3
            && (got - 1.0).abs() <= TOL * R::EPS * 1.25,
        "{name}: on the column Q_min(U) = 0.25, t = 0 reads {got}, not 1"
    );
    worst
}

fn check_round_trip(b: Break) {
    let w64 = arms::<Alpha, f64>(ALPHA, b).max(arms::<ZAlpha, f64>(Z_ALPHA, b));
    let w32 = arms::<Alpha, f32>(ALPHA, b).max(arms::<ZAlpha, f32>(Z_ALPHA, b));
    println!(
        "worst |forward(inverse(x)) − x| / (eps·(|x| + Q_min + Q₀)): f64 {w64:.2}, f32 {w32:.2}; criterion {TOL}"
    );
}

#[test]
fn virial_descriptor_round_trip() {
    check_round_trip(Break::None);
}

negative_control!(
    virial_descriptor_round_trip,
    "a factor changed in the inverse: Q₀·|U| for Q₀·|U|/2",
    expected = "forward(inverse(x))",
    check_round_trip(Break::FactorInInverse)
);

#[test]
fn virial_descriptor_round_trip_above_floor() {
    check_round_trip(Break::None);
}

negative_control!(
    virial_descriptor_round_trip_above_floor,
    "the warped row at Q₀ = 0 and a held L_z ≠ 0 read as the state's own ratio",
    expected = "forward(inverse(x))",
    check_round_trip(Break::StateRatioAtZero)
);

#[test]
fn virial_descriptor_round_trip_bottom() {
    check_round_trip(Break::None);
}

negative_control!(
    virial_descriptor_round_trip_bottom,
    "the Q₀ = 0 form on a Q₀ > 0 axis, where Q₀ ≥ Q_min(U)",
    expected = "forward(inverse(x))",
    check_round_trip(Break::ZeroFormAtPositive)
);

#[test]
fn virial_descriptor_round_trip_holds() {
    check_round_trip(Break::None);
}

negative_control!(
    virial_descriptor_round_trip_holds,
    "the row's own inverse, a scale solve, in place of the chart's decode",
    expected = "not the held",
    check_round_trip(Break::ScaleSolve)
);

// ── The descriptor, the refusals, the label, the mark and the readout ────────────────────────────────────────────────

/// A chart whose descriptor is the test's: the validator reads only the descriptor and `Chart`'s flags.
struct Probe(ChartDescriptor);

impl Chart for Probe {
    type Param = f64;

    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        let c = <F as From<f64>>::from(0.1);
        ChartOut {
            z: kernel::decode::Latent::new([u, v, c, c, c, c, c, c]),
        }
    }

    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "Probe"
    }

    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}

impl Authored for Probe {
    fn descriptor(&self) -> ChartDescriptor {
        self.0.clone()
    }
}

fn check_descriptor(b: Break) {
    let pair = chart::<Alpha, f64>(ALPHA, ASKED, 0.15, Mapping::Warped);
    let mut d = pair.descriptor();
    if b == Break::NoDescriptor {
        d.axes[1] = AxisDescriptor::derived(
            "Q",
            Block::Momentum,
            None,
            Residual {
                pins: 0,
                statement: "none",
            },
        );
    }
    let v = d.axes[1];
    let inv = v.invariant_meta();
    validate(Probe(d.clone()), None).expect("the validator takes the virial pair's descriptor");
    validate(pair, None).expect("the validator takes the virial pair");
    assert!(
        v.kind() == Some(AxisKind::CrossBlockInvariant)
            && inv.is_some_and(|i| i.quantity() == InvariantQuantity::Q
                && i.sector() == Block::Momentum
                && !i.conserved_along_flow()),
        "the virial axis is not kind 3, Q into the momenta, unconserved: {v:?}"
    );
    assert!(
        matches!(validate(pair, Some(-0.5)), Err(Refusal::View(_))),
        "a view's energy target on the virial pair is not refused"
    );
    // The registry's path: refused under hold E and beside K; (α, Q) lowers to its baked entry.
    let registry = Registry::standard();
    let spec = |h: &str, v: &str| ChartSpec::new(AxisSpec::pick(h), AxisSpec::pick(v), Z0);
    let mut held_e = spec("α", "Q");
    held_e.constraints.hold_e = Some(-0.5);
    assert!(
        matches!(
            validate_spec(&registry, &held_e),
            Err(Refusal::AxisAndHold { .. })
        ),
        "(α, Q) under hold E is not refused AxisAndHold"
    );
    assert!(
        matches!(
            validate_spec(&registry, &spec("K", "Q")),
            Err(Refusal::SameQuantity { .. })
        ),
        "(K, Q) is not refused SameQuantity"
    );
    let r = resolve_spec(&validate_spec(&registry, &spec("α", "Q")).expect("(α, Q) validates"));
    assert_eq!(
        r.lowering,
        Lowering::Baked {
            chart: "Mixed",
            entry: MixedAlphaVirialDefaults::ENTRY
        },
        "(α, Q) does not lower to its baked entry"
    );
    // Labels.
    let label = |asked: [f64; 2], held: f64, m: Mapping| {
        VirialAxis::of(&chart::<Alpha, f64>(ALPHA, asked, held, m)).label()
    };
    assert_eq!(
        label([0.0, 2.0], 0.15, Mapping::Warped),
        "Q above the floor"
    );
    assert_eq!(label(ASKED, 0.15, Mapping::Warped), "Q");
    assert_eq!(label([0.0, 2.0], 0.0, Mapping::Warped), "Q");
    assert_eq!(label([0.0, 2.0], 0.15, Mapping::Linear), "Q");
    // The mark and the readout, on the crossing chart's bottom row.
    let c = chart::<Alpha, f64>(
        ALPHA,
        ASKED,
        (ASKED[0] * mid_u::<Alpha, f64>(ALPHA)).sqrt(),
        Mapping::Warped,
    );
    let axis = VirialAxis::of(&c);
    let (mut marked, mut unmarked) = (0, 0);
    for i in 0..N {
        let u = f64::from(i) / f64::from(N - 1);
        let ic = wide(&c.decode::<Defaults, f64>(u, 0.0));
        let qm = q_min(&ic, axis.held);
        let own = 2.0 * engine::chart_authoring::registry::kinetic(&ic) / potential(&ic).abs();
        assert!(
            axis.readout(&ic) == ratio(&ic),
            "the readout is not the state's own ratio"
        );
        assert!(
            (axis.readout(&ic) - own).abs() <= 1e-14 * own,
            "the readout {} is not 2K/|U| = {own}",
            axis.readout(&ic)
        );
        if qm > ASKED[0] * (1.0 + 1e-3) {
            marked += 1;
            assert!(
                axis.marked(&ic),
                "u = {u}: Q_min(U) = {qm} > Q₀ is not marked"
            );
            let want = own - (qm - ASKED[0]);
            assert!(
                (axis.forward(&ic) - want).abs() <= TOL * f64::EPSILON * (own + qm),
                "u = {u}: the marked pixel reads {}, not its ratio less Q_min − Q₀ = {want}",
                axis.forward(&ic)
            );
        } else if qm < ASKED[0] * (1.0 - 1e-3) {
            unmarked += 1;
            assert!(!axis.marked(&ic), "u = {u}: Q_min(U) = {qm} ≤ Q₀ is marked");
        }
    }
    assert!(
        marked >= 8 && unmarked >= 8,
        "{marked} marked and {unmarked} unmarked columns, not 8 of each"
    );
}

#[test]
fn virial_descriptor() {
    check_descriptor(Break::None);
}

negative_control!(
    virial_descriptor,
    "the virial pair with no descriptor for its virial axis: a kind-2 stand-in in the momenta",
    expected = "the validator takes the virial pair's descriptor",
    check_descriptor(Break::NoDescriptor)
);
