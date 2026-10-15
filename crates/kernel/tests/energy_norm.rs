//! Energy normalisation `η_E` (`kernel::decode::energy_norm`; dd_decoder §3.7; REQ-DEC-024, REQ-DEC-041), each check
//! at the f64 and the f32 instantiation natively (R-265):
//! - `energy_normalisation_sets_target`: with `E*` set and `η_E` applied, the decoded energy `K_0 + V_0` equals `E*`,
//!   over fuzzed `z` and every registered link; every decode takes dd_decoder §3.7's outcome for its `E*`, `V_0`, rest
//!   and separation; an untouched outcome leaves the momenta bit for bit; no outcome stores NaN.
//! - `energy_normalisation_off`: `None`, or a chart that forbids it, leaves the decode bit for bit (R-25).
//! - `energy_normalisation_zero_target`: `E* = 0` is a target, the energy set to 0, never "off" (R-25).
//! - `energy_normalisation_rest_start`: at `K₀ = 0`, `E* = U` is at the target, untagged; `E* > U` is
//!   `DEGENERATE(ENERGY_UNSCALABLE)`; `E* < U` is `DEGENERATE(ENERGY_INFEASIBLE)`; the momenta stay 0.
//! - `energy_normalisation_infeasible`: `E* < U` with moving bodies is `DEGENERATE(ENERGY_INFEASIBLE)`, the momenta
//!   untouched; the boundary `E* = U` scales to rest, the value one below it is infeasible.
//! - `energy_normalisation_collision_pole`: at `U = −∞`, a pair separation 0, the decode is
//!   `DEGENERATE(ENERGY_UNSCALABLE)` with nothing computed on `∞`.
//! - `energy_normalisation_underflow`: a non-zero momentum whose `K₀` rounds to 0 is scaled to `E*`, not read as rest;
//!   each component of each body counts toward `s`.
//! - `energy_normalisation_keeps_earlier_tag`: a decode the mass block tagged keeps `M01_TINY`.
//!
//! **The tolerances** (applied per R-369: test bounds from the operation count, not calibrated values). The scaled
//! `K = Σ η²p̂ᵢ²/2mᵢ` is formed from `Δ = E* − U` in eight roundings, each relative to its operand (`p̂ = p/s`, the
//! square and the sum of each body's components, the division by `2mᵢ`, the sum over bodies, `Δ/K̂`, the `sqrt` and
//! `η·p̂`), and `K + U` in one more, so `|E − E*| ≤` [`ENERGY_EPS`]` · eps · (K + |U| + |E*|)`, nine. The same bound
//! holds each scaled component to one factor `η` (three roundings, plus `η`'s own read-off), relative to the largest.
//! The worst seen of each is printed: for the energy 1.3 eps at f64 and at f32, for the one factor 1.0 and 0.7.

mod decode_support;

use decode_support::*;
use kernel::decode::degenerate::Degenerate;
use kernel::decode::descriptor;
use kernel::decode::energy_norm::{self, Normalised, Outcome};
use kernel::decode::{self, Decoded, Free, Ic, Latent};
use kernel::generated::links::*;
use proptest::prelude::*;
use validation::negative_control;

/// `|E − E*|` in units of `eps · (K + |U| + |E*|)`, at most: one per rounding (the module's note).
const ENERGY_EPS: f64 = 9.0;

/// A decode entry under a target.
type Entry<R> = fn(&Latent<R>, Option<R>) -> (Decoded<R>, Outcome);

/// The rescale alone.
type Norm<R> = fn(&Ic<R>, R, R, R) -> Normalised<R>;

/// The entries under test, one per registered link.
fn entries<R: P>() -> [(&'static str, Entry<R>); 4] {
    [
        ("defaults", energy_norm::decode::<Defaults, R>),
        ("stick_breaking", energy_norm::decode::<Stick, R>),
        ("tanh", energy_norm::decode::<Tanhs, R>),
        ("softsign", energy_norm::decode::<Softsigns, R>),
    ]
}

/// The decode without `η_E`, under the selection of the entry named `name`.
fn plain<R: P>(name: &str) -> fn(&Latent<R>) -> Decoded<R> {
    selections::<R>()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, e)| e)
        .expect("a selection of that name")
}

/// The momenta's bits.
fn p_bits<R: P>(p: &[[R; 2]; 3]) -> Vec<u64> {
    p.iter().flatten().map(|x| x.bits()).collect()
}

/// Whether any momentum or descriptor value is NaN.
fn has_nan<R: P>(d: &Decoded<R>) -> bool {
    let fields = ["K_0", "V_0", "virial_ratio", "rho_ratio", "r_min_pair_0"];
    d.ic.p.iter().flatten().any(|x| x.f().is_nan())
        || fields.iter().any(|f| field(&d.descriptor, f).f().is_nan())
}

/// The outcome dd_decoder §3.7 states for the target `e`, the decode's `V_0` and `r_min_pair_0`, and its momenta.
fn expected<R: P>(d: &Decoded<R>, e: R) -> Outcome {
    let v = d.descriptor.V_0.f();
    let at_rest = d.ic.p.iter().flatten().all(|x| x.f() == 0.0);
    if d.descriptor.r_min_pair_0.f() == 0.0 {
        Outcome::Unscalable
    } else if e.f() < v {
        Outcome::Infeasible
    } else if at_rest && e.f() == v {
        Outcome::AtTarget
    } else if at_rest {
        Outcome::Unscalable
    } else {
        Outcome::Scaled
    }
}

/// The tag each outcome raises (dd_decoder §3.7).
fn tag_of(o: Outcome) -> Option<Degenerate> {
    match o {
        Outcome::Infeasible => Some(Degenerate::EnergyInfeasible),
        Outcome::Unscalable => Some(Degenerate::EnergyUnscalable),
        _ => None,
    }
}

// ── energy_normalisation_sets_target (REQ-DEC-024) ──────────────────────────────────────────────────────────────

/// A target relative to the decode's `U`: `U + t·|U|`, `t` in `[−1, 4]`, or exactly 0.
fn target() -> impl Strategy<Value = Option<f64>> {
    prop_oneof![Just(None), (-1.0f64..=4.0).prop_map(Some)]
}

/// Checks `entry` at `z` under the target `t` (`None`: 0; `Some(t)`: `U + t·|U|`): the outcome is §3.7's, the tag is
/// the outcome's unless the mass block tagged first, untouched outcomes leave the momenta bit for bit, a scaled
/// outcome's energy is `E*`, and nothing is NaN.
fn check_target<R: P>(name: &str, entry: Entry<R>, z: [f64; 8], t: Option<f64>, worst: &Worst) {
    let zl = latent::<R>(z);
    let before = plain::<R>(name)(&zl);
    let v = before.descriptor.V_0.f();
    let e = R::of(match t {
        None => 0.0,
        Some(t) if v.is_finite() => v + t * v.abs(),
        Some(t) => t,
    });
    let (after, outcome) = entry(&zl, Some(e));
    let want = expected(&before, e);
    assert_eq!(
        outcome,
        want,
        "the outcome at z = {z:?}, E* = {e:?}, {name}, {} is not §3.7's",
        R::NAME
    );
    assert_eq!(
        after.degenerate,
        before.degenerate.or(tag_of(want)),
        "the tag at z = {z:?}, E* = {e:?}, {name}, {}",
        R::NAME
    );
    assert!(
        !has_nan(&after),
        "NaN stored at z = {z:?}, E* = {e:?}, {name}, {}",
        R::NAME
    );
    assert_eq!(
        after.momentum, before.momentum,
        "the momentum block's output moved"
    );
    if want != Outcome::Scaled {
        assert_eq!(
            p_bits(&after.ic.p),
            p_bits(&before.ic.p),
            "momenta moved under the untouched outcome {want:?} at z = {z:?}, {name}, {}",
            R::NAME
        );
        return;
    }
    // One factor for every body: `η` read off the largest decoded component, every other component within
    // `ENERGY_EPS` eps of `η` times its decoded value, relative to the largest scaled component.
    let (pb, pa): (Vec<f64>, Vec<f64>) = before
        .ic
        .p
        .iter()
        .flatten()
        .zip(after.ic.p.iter().flatten())
        .map(|(b, a)| (b.f(), a.f()))
        .unzip();
    let j = (0..6)
        .max_by(|&a, &b| pb[a].abs().total_cmp(&pb[b].abs()))
        .expect("six components");
    let eta = pa[j] / pb[j];
    let top = pa.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    // `η = 0` (`E* = U`) leaves every component 0, one factor for all.
    for i in (0..6).filter(|_| top > 0.0) {
        let off = (pa[i] - eta * pb[i]).abs() / (R::eps() * top);
        worst.see(&format!("one factor at {} (eps)", R::NAME), off);
        assert!(
            off <= ENERGY_EPS,
            "the rescale is not one factor η = {eta:e} for every component at z = {z:?}, {name}, {}: {} → {}",
            R::NAME,
            pb[i],
            pa[i]
        );
    }
    let k = after.descriptor.K_0.f();
    let u = after.descriptor.V_0.f();
    let energy = descriptor::energy(&after.descriptor).f();
    let scale = R::eps() * (k + u.abs() + e.f().abs());
    let err = (energy - e.f()).abs() / scale;
    worst.see(&format!("|E − E*| at {} (eps)", R::NAME), err);
    assert!(
        err <= ENERGY_EPS,
        "the decoded energy {energy:e} is not E* = {e:?} at z = {z:?}, {name}, {}: {err:.2} eps",
        R::NAME
    );
}

fn check_targets<R: P>(entries: [(&'static str, Entry<R>); 4], worst: &Worst) {
    for (name, entry) in entries {
        for z in fixed_z() {
            for t in [
                None,
                Some(-1.0),
                Some(-0.5),
                Some(0.0),
                Some(0.5),
                Some(3.0),
            ] {
                check_target::<R>(name, entry, z, t, worst);
            }
        }
        validation::prop::run(&(any_z(), target()), |(z, t)| {
            check_target::<R>(name, entry, z, t, worst);
            Ok(())
        });
    }
}

#[test]
fn energy_normalisation_sets_target() {
    let worst = Worst::default();
    check_targets::<f64>(entries::<f64>(), &worst);
    check_targets::<f32>(entries::<f32>(), &worst);
    worst.print("energy_normalisation_sets_target");
}

/// The scaled momenta off by 1 %: the energy misses `E*`.
#[cfg(feature = "controls")]
fn off_scale<S: Selection, R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    let (mut d, o) = energy_norm::decode::<S, R>(z, e);
    if o == Outcome::Scaled {
        d.ic.p = d.ic.p.map(|q| q.map(|x| x * R::of(1.01)));
        d.descriptor = descriptor::describe(&d.ic);
    }
    (d, o)
}

/// One body's momentum scaled by a different factor: the energy can still be `E*`, but the rescale is not uniform.
#[cfg(feature = "controls")]
fn skewed<S: Selection, R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    let (mut d, o) = energy_norm::decode::<S, R>(z, e);
    if o == Outcome::Scaled {
        d.ic.p[2] = d.ic.p[2].map(|x| x * R::of(1.01));
    }
    (d, o)
}

negative_control!(
    energy_normalisation_sets_target,
    "a rescale off by 1 % misses E*",
    expected = "the decoded energy",
    check_target::<f64>(
        "defaults",
        off_scale::<Defaults, f64>,
        MOVING,
        Some(1.0),
        &Worst::default()
    )
);

#[cfg(feature = "controls")]
mod sets_target_skewed {
    use super::*;
    negative_control!(
        energy_normalisation_sets_target,
        "a rescale that is not one factor for every body must fail",
        expected = "is not one factor",
        check_target::<f64>(
            "defaults",
            skewed::<Defaults, f64>,
            MOVING,
            Some(1.0),
            &Worst::default()
        )
    );
}

// ── energy_normalisation_off (REQ-DEC-024, R-25) ────────────────────────────────────────────────────────────────

/// Checks that `entry` under `None`, and under a target on a chart that forbids it, gives the plain decode bit for
/// bit, with the outcome `Off`.
fn check_off<R: P>(name: &str, entry: Entry<R>, forbidding: Entry<R>, z: [f64; 8]) {
    let zl = latent::<R>(z);
    let before = plain::<R>(name)(&zl);
    for (what, run, e) in [
        ("None", entry, None),
        ("a forbidding chart", forbidding, Some(R::of(0.0))),
        ("a forbidding chart", forbidding, Some(R::of(-0.25))),
    ] {
        let (after, outcome) = run(&zl, e);
        assert_eq!(
            outcome,
            Outcome::Off,
            "{what} is not off at z = {z:?}, {}",
            R::NAME
        );
        assert_eq!(
            p_bits(&after.ic.p),
            p_bits(&before.ic.p),
            "momenta moved under {what} at z = {z:?}, {}",
            R::NAME
        );
        assert_eq!(
            after.degenerate, before.degenerate,
            "{what} tagged the decode"
        );
    }
}

/// The default entry on a chart that forbids `η_E`.
fn forbidding<R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    energy_norm::decode_with(&mut Free::<Defaults>::new(), z, e, true)
}

fn check_offs<R: P>(entry: Entry<R>, forbids: Entry<R>) {
    over(any_z(), |z| check_off::<R>("defaults", entry, forbids, z));
}

#[test]
fn energy_normalisation_off() {
    check_offs::<f64>(energy_norm::decode::<Defaults, f64>, forbidding::<f64>);
    check_offs::<f32>(energy_norm::decode::<Defaults, f32>, forbidding::<f32>);
}

/// `None` read as `Some(0)`.
#[cfg(feature = "controls")]
fn none_as_zero<R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    energy_norm::decode::<Defaults, R>(z, Some(e.unwrap_or(R::of(0.0))))
}

/// A forbidding chart's flag ignored.
#[cfg(feature = "controls")]
fn flag_ignored<R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    energy_norm::decode_with(&mut Free::<Defaults>::new(), z, e, false)
}

/// A moving `z`.
const MOVING: [f64; 8] = [0.0, 0.0, 1.0, -0.5, 0.3, 0.7, 0.2, -0.4];

negative_control!(
    energy_normalisation_off,
    "None read as E* = 0 rescales the momenta",
    expected = "None is not off",
    check_off::<f64>("defaults", none_as_zero::<f64>, forbidding::<f64>, MOVING)
);

#[cfg(feature = "controls")]
mod off_flag {
    use super::*;
    negative_control!(
        energy_normalisation_off,
        "a forbidding chart whose flag is ignored rescales the momenta",
        expected = "a forbidding chart is not off",
        check_off::<f64>(
            "defaults",
            energy_norm::decode::<Defaults, f64>,
            flag_ignored::<f64>,
            MOVING
        )
    );
}

// ── energy_normalisation_zero_target (REQ-DEC-024, R-25) ────────────────────────────────────────────────────────

/// Checks that `Some(0)` at the moving `z` is scaled, its energy 0 to the tolerance, and its momenta moved from the
/// plain decode's.
fn check_zero<R: P>(entry: Entry<R>, z: [f64; 8]) {
    let zl = latent::<R>(z);
    let before = decode::decode::<Defaults, R>(&zl);
    let (after, outcome) = entry(&zl, Some(R::of(0.0)));
    assert_eq!(
        outcome,
        Outcome::Scaled,
        "E* = 0 read as off at z = {z:?}, {}",
        R::NAME
    );
    assert_ne!(
        p_bits(&after.ic.p),
        p_bits(&before.ic.p),
        "E* = 0 read as off: the momenta did not move"
    );
    let k = after.descriptor.K_0.f();
    let u = after.descriptor.V_0.f();
    let energy = descriptor::energy(&after.descriptor).f();
    assert!(
        energy.abs() <= ENERGY_EPS * R::eps() * (k + u.abs()),
        "E* = 0 gave the energy {energy:e} at {}",
        R::NAME
    );
}

#[test]
fn energy_normalisation_zero_target() {
    for z in [MOVING, [0.4, -1.1, -2.0, 0.1, 0.9, -0.3, 0.0, 0.0]] {
        check_zero::<f64>(energy_norm::decode::<Defaults, f64>, z);
        check_zero::<f32>(energy_norm::decode::<Defaults, f32>, z);
    }
}

/// `Some(0)` read as off, the bug R-25 rules out.
#[cfg(feature = "controls")]
fn zero_as_off<R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    let e = e.filter(|e| e.f() != 0.0);
    energy_norm::decode::<Defaults, R>(z, e)
}

negative_control!(
    energy_normalisation_zero_target,
    "E* = 0 read as off must fail",
    expected = "E* = 0 read as off",
    check_zero::<f64>(zero_as_off::<f64>, MOVING)
);

// ── energy_normalisation_rest_start (REQ-DEC-041) ───────────────────────────────────────────────────────────────

/// The rest start: the golden IC's `z = 0`, and a configuration away from it with `z_q = 0`.
const RESTS: [[f64; 8]; 2] = [[0.0; 8], [0.7, -1.2, 0.0, 0.0, 0.0, 0.0, 0.5, -0.3]];

/// Checks that at each rest start `E* = U` is `AtTarget`, the value above `U` `Unscalable` and the value below it
/// `Infeasible`, each tagged as §3.7 states and each leaving the momenta 0.
fn check_rest<R: PS>(entry: Entry<R>) {
    for z in RESTS {
        let zl = latent::<R>(z);
        let before = decode::decode::<Defaults, R>(&zl);
        assert!(
            before.ic.p.iter().flatten().all(|x| x.f() == 0.0),
            "z = {z:?} is not at rest"
        );
        let u = before.descriptor.V_0;
        for (e, want, tag) in [
            (u, Outcome::AtTarget, None),
            (
                R::next_up(u),
                Outcome::Unscalable,
                Some(Degenerate::EnergyUnscalable),
            ),
            (
                R::of(0.0),
                Outcome::Unscalable,
                Some(Degenerate::EnergyUnscalable),
            ),
            (
                R::next_down(u),
                Outcome::Infeasible,
                Some(Degenerate::EnergyInfeasible),
            ),
        ] {
            let (after, outcome) = entry(&zl, Some(e));
            assert_eq!(
                (outcome, after.degenerate),
                (want, tag),
                "the rest start at z = {z:?}, E* = {e:?}, U = {u:?}, {}",
                R::NAME
            );
            assert_eq!(
                p_bits(&after.ic.p),
                p_bits(&before.ic.p),
                "the rest start's momenta moved"
            );
            assert!(!has_nan(&after), "NaN stored at the rest start");
        }
    }
}

/// `next_up` and `next_down` at each precision.
trait Step: Sized {
    fn next_up(self) -> Self;
    fn next_down(self) -> Self;
}

impl Step for f64 {
    fn next_up(self) -> Self {
        f64::next_up(self)
    }
    fn next_down(self) -> Self {
        f64::next_down(self)
    }
}

impl Step for f32 {
    fn next_up(self) -> Self {
        f32::next_up(self)
    }
    fn next_down(self) -> Self {
        f32::next_down(self)
    }
}

trait PS: P + Step {}
impl<R: P + Step> PS for R {}

#[test]
fn energy_normalisation_rest_start() {
    check_rest::<f64>(energy_norm::decode::<Defaults, f64>);
    check_rest::<f32>(energy_norm::decode::<Defaults, f32>);
}

/// The rest start read through `K₀ = 0` and the formula alone: every rest start is "at the target".
#[cfg(feature = "controls")]
fn rest_untagged<R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    let (mut d, o) = energy_norm::decode::<Defaults, R>(z, e);
    if o == Outcome::Unscalable {
        d.degenerate = None;
        return (d, Outcome::AtTarget);
    }
    (d, o)
}

negative_control!(
    energy_normalisation_rest_start,
    "a rest start above U left untagged must fail",
    expected = "the rest start at z",
    check_rest::<f64>(rest_untagged::<f64>)
);

// ── energy_normalisation_infeasible (REQ-DEC-041) ───────────────────────────────────────────────────────────────

/// Checks, at the moving `z`, that `E* = U` scales to rest, untagged, its energy `U`; that the value below `U` is
/// `Infeasible`, tagged, its momenta untouched; and that the value above `U` scales.
fn check_infeasible<R: PS>(norm: Norm<R>, z: [f64; 8]) {
    let zl = latent::<R>(z);
    let d = decode::decode::<Defaults, R>(&zl);
    let (u, r) = (d.descriptor.V_0, d.descriptor.r_min_pair_0);
    let at = norm(&d.ic, u, r, u);
    assert_eq!(
        at.outcome,
        Outcome::Scaled,
        "E* = U is not scaled at {}",
        R::NAME
    );
    assert!(
        at.p.iter().flatten().all(|x| x.f() == 0.0),
        "E* = U did not scale to rest at {}: {:?}",
        R::NAME,
        at.p
    );
    let below = norm(&d.ic, u, r, R::next_down(u));
    assert_eq!(
        (below.outcome, below.outcome.tag()),
        (Outcome::Infeasible, Some(Degenerate::EnergyInfeasible)),
        "E* one below U is not infeasible at {}",
        R::NAME
    );
    assert_eq!(
        p_bits(&below.p),
        p_bits(&d.ic.p),
        "an infeasible E* moved the momenta"
    );
    let far = norm(&d.ic, u, r, u + u);
    assert_eq!(
        far.outcome,
        Outcome::Infeasible,
        "E* = 2U is not infeasible"
    );
    let above = norm(&d.ic, u, r, R::next_up(u));
    assert_eq!(
        above.outcome,
        Outcome::Scaled,
        "E* one above U is not scaled at {}",
        R::NAME
    );
}

#[test]
fn energy_normalisation_infeasible() {
    check_infeasible::<f64>(energy_norm::normalise::<f64>, MOVING);
    check_infeasible::<f32>(energy_norm::normalise::<f32>, MOVING);
}

/// Feasibility as `E* > U`: `E* = U` read as infeasible.
#[cfg(feature = "controls")]
fn strict<R: P>(ic: &Ic<R>, v0: R, r_min: R, e: R) -> Normalised<R> {
    let n = energy_norm::normalise(ic, v0, r_min, e);
    if e == v0 {
        return Normalised {
            p: ic.p,
            outcome: Outcome::Infeasible,
        };
    }
    n
}

/// `E* < U` clamped to `η_E = 0`, untagged.
#[cfg(feature = "controls")]
fn clamped<R: P>(ic: &Ic<R>, v0: R, r_min: R, e: R) -> Normalised<R> {
    energy_norm::normalise(ic, v0, r_min, if e < v0 { v0 } else { e })
}

negative_control!(
    energy_normalisation_infeasible,
    "feasibility as E* > U must fail at E* = U",
    expected = "E* = U is not scaled",
    check_infeasible::<f64>(strict::<f64>, MOVING)
);

#[cfg(feature = "controls")]
mod infeasible_clamped {
    use super::*;
    negative_control!(
        energy_normalisation_infeasible,
        "E* < U clamped to rest, untagged, must fail",
        expected = "E* one below U is not infeasible",
        check_infeasible::<f64>(clamped::<f64>, MOVING)
    );
}

// ── energy_normalisation_collision_pole (REQ-DEC-041, R-436) ────────────────────────────────────────────────────

/// Checks that `norm` at a separation of 0, `V_0 = −∞`, is `Unscalable` and leaves the momenta, for each target, and
/// that the decode at f64's collision pole is so tagged.
fn check_collision<R: P>(norm: Norm<R>) {
    let d = decode::decode::<Defaults, R>(&latent::<R>(MOVING));
    for e in [-2.0, 0.0, 3.0] {
        let n = norm(&d.ic, R::of(f64::NEG_INFINITY), R::of(0.0), R::of(e));
        assert_eq!(
            n.outcome,
            Outcome::Unscalable,
            "the collision pole is not unscalable at E* = {e}, {}",
            R::NAME
        );
        assert_eq!(
            p_bits(&n.p),
            p_bits(&d.ic.p),
            "the collision pole moved the momenta"
        );
    }
}

/// A `z` at f64's collision pole: `α` saturated at the image of π/2, where the emitted `r₀` and `r₁` coincide
/// (dd_decoder §3.6), found by a grid search over `β` and the masses.
const POLE: [f64; 8] = [60.0, -8.0, 1.0, -0.5, 0.3, 0.7, 0.0, 0.5];

#[test]
fn energy_normalisation_collision_pole() {
    check_collision::<f64>(energy_norm::normalise::<f64>);
    check_collision::<f32>(energy_norm::normalise::<f32>);
    let z = POLE;
    let plain = decode::decode::<Defaults, f64>(&latent::<f64>(z)).descriptor;
    assert_eq!(
        (plain.r_min_pair_0, plain.V_0),
        (0.0, f64::NEG_INFINITY),
        "z = {z:?} is not at the collision pole"
    );
    let (d, o) = energy_norm::decode::<Defaults, f64>(&latent::<f64>(z), Some(-1.0));
    assert_eq!(
        (o, d.degenerate),
        (Outcome::Unscalable, Some(Degenerate::EnergyUnscalable)),
        "the decode at the collision pole z = {z:?}"
    );
    assert!(!has_nan(&d), "NaN stored at the collision pole");
}

/// The separation ignored: `V_0 = −∞` read as a value.
#[cfg(feature = "controls")]
fn no_pole<R: P>(ic: &Ic<R>, v0: R, _r_min: R, e: R) -> Normalised<R> {
    energy_norm::normalise(ic, v0, R::of(1.0), e)
}

negative_control!(
    energy_normalisation_collision_pole,
    "a rescale that ignores the separation must fail at the pole",
    expected = "the collision pole is not unscalable",
    check_collision::<f64>(no_pole::<f64>)
);

// ── energy_normalisation_underflow (REQ-DEC-041) ────────────────────────────────────────────────────────────────

/// An IC with equal masses and a triangle, its momenta `p`.
fn ic_with<R: P>(p: [[f64; 2]; 3]) -> Ic<R> {
    let third = R::of(1.0 / 3.0);
    Ic {
        m: [third; 3],
        r: [[-0.5, 0.0], [0.5, 0.0], [0.0, 0.8]].map(|q| q.map(R::of)),
        p: p.map(|q| q.map(R::of)),
    }
}

/// Checks that a momentum whose `K₀` rounds to 0, in each component of each body and of each sign, is scaled to
/// `E*`, and that `s` is its magnitude.
fn check_underflow<R: P>(norm: Norm<R>, tiny: f64) {
    for k in 0..6 {
        for sign in [1.0, -1.0] {
            let mut p = [[0.0; 2]; 3];
            p[k / 2][k % 2] = sign * tiny;
            let ic = ic_with::<R>(p);
            assert_eq!(
                energy_norm::largest(&ic.p).f(),
                R::of(tiny).f(),
                "s misses component {k} of {p:?}"
            );
            assert_eq!(
                descriptor::kinetic(&ic).f(),
                0.0,
                "K₀ of {p:?} does not round to 0 at {}",
                R::NAME
            );
            let d = descriptor::describe(&ic);
            let e = R::of(-0.25);
            let n = norm(&ic, d.V_0, d.r_min_pair_0, e);
            assert_eq!(
                n.outcome,
                Outcome::Scaled,
                "an underflowed momentum {p:?} is read as rest at {}",
                R::NAME
            );
            let out = Ic { p: n.p, ..ic };
            let k0 = descriptor::kinetic(&out).f();
            let u = d.V_0.f();
            let err = (k0 + u - e.f()).abs() / (R::eps() * (k0 + u.abs() + e.f().abs()));
            assert!(
                err <= ENERGY_EPS,
                "the energy after an underflowed momentum {p:?} misses E* by {err:.2} eps"
            );
        }
    }
}

#[test]
fn energy_normalisation_underflow() {
    check_underflow::<f64>(energy_norm::normalise::<f64>, 1e-170);
    check_underflow::<f32>(energy_norm::normalise::<f32>, 1e-25);
}

/// Rest read from the rounded `K₀ = 0`.
#[cfg(feature = "controls")]
fn rounded_rest<R: P>(ic: &Ic<R>, v0: R, r_min: R, e: R) -> Normalised<R> {
    if descriptor::kinetic(ic).f() == 0.0 {
        return Normalised {
            p: ic.p,
            outcome: Outcome::Unscalable,
        };
    }
    energy_norm::normalise(ic, v0, r_min, e)
}

negative_control!(
    energy_normalisation_underflow,
    "rest read from the rounded K₀ must fail",
    expected = "is read as rest",
    check_underflow::<f64>(rounded_rest::<f64>, 1e-170)
);

// ── energy_normalisation_keeps_earlier_tag (REQ-DEC-041) ────────────────────────────────────────────────────────

/// Checks that a `z` the mass block tags `M01_TINY` keeps that tag under an infeasible and an unscalable target.
fn check_earlier<R: P>(entry: Entry<R>) {
    let z = fixed_z()
        .into_iter()
        .find(|z| {
            decode::decode::<Stick, R>(&latent::<R>(*z)).degenerate == Some(Degenerate::M01Tiny)
        })
        .expect("a fixed z the stick_breaking mass block tags");
    let zl = latent::<R>(z);
    let u = decode::decode::<Stick, R>(&zl).descriptor.V_0;
    for e in [u + u, R::of(1.0)] {
        let (d, _) = entry(&zl, Some(e));
        assert_eq!(
            d.degenerate,
            Some(Degenerate::M01Tiny),
            "the mass block's tag was replaced at z = {z:?}, E* = {e:?}, {}",
            R::NAME
        );
    }
}

#[test]
fn energy_normalisation_keeps_earlier_tag() {
    check_earlier::<f64>(energy_norm::decode::<Stick, f64>);
    check_earlier::<f32>(energy_norm::decode::<Stick, f32>);
}

/// The outcome's tag over the mass block's.
#[cfg(feature = "controls")]
fn last_tag<R: P>(z: &Latent<R>, e: Option<R>) -> (Decoded<R>, Outcome) {
    let (mut d, o) = energy_norm::decode::<Stick, R>(z, e);
    d.degenerate = o.tag().or(d.degenerate);
    (d, o)
}

negative_control!(
    energy_normalisation_keeps_earlier_tag,
    "η_E's tag over the mass block's must fail",
    expected = "the mass block's tag was replaced",
    check_earlier::<f64>(last_tag::<f64>)
);
