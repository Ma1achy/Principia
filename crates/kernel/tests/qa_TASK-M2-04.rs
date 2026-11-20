//! QA tests for TASK-M2-04 (energy normalisation `η_E` with an explicit off switch), written from REQ-DEC-024,
//! REQ-DEC-041, dd_decoder §3.7, chart_reference §0.6 and R-25, R-124, not from the implementation. Each check runs at
//! the f64 and the f32 instantiation (R-265), behind a function pointer, so each negative control (R-176) runs the same
//! check on a deliberately broken `η_E`.
//!
//! - REQ-DEC-024, "with E* set, decoded energy equals E*": over fuzzed `z` with moving bodies and targets
//!   `E* ∈ [U, U + 4|U|]`, and every `E* ≥ 0`, the rescaled IC's energy, evaluated here in f64 from its `(m, r, p)`,
//!   and the descriptor's `K_0 + V_0` both equal `E*`; the momenta are one common factor `η_E ≥ 0` times the decoded
//!   ones (`pᵢ ← η_E·pᵢ`), with `η_E = √((E* − U)/K₀)`; masses and positions untouched; the descriptor describes the
//!   rescaled IC, its `rho_angle` the decode's (R-628: read from the configuration block, which `η_E` does not move).
//! - REQ-DEC-024, "with the option None, momenta are untouched": `None` gives the plain decode bit for bit; a chart
//!   whose `forbids_energy_normalisation` is set gives the same under any `Some(E*)` (chart_reference §0.6).
//! - REQ-DEC-024, "E* = 0 is a real target, not off": `Some(0)` sets the energy to 0; `Some(−0.0)` is the same target
//!   (`−0 = 0`), never off; at the rest start both are `E* > U`, tagged, not off.
//! - REQ-DEC-041 / §3.7's outcomes, at their boundaries: `E* = U` exactly with moving bodies scales to rest, untagged;
//!   one ulp below is `ENERGY_INFEASIBLE` with the momenta untouched; one ulp above scales, untagged. At the rest start
//!   (every `pᵢ = 0`) `E* = U` is untouched and untagged, one ulp above is `ENERGY_UNSCALABLE`, one ulp below
//!   `ENERGY_INFEASIBLE`.
//! - §3.7, "`K₀ = 0` means every `pᵢ` is 0, not that the rounded `K₀` is 0": a subnormal momentum whose `K₀` rounds to
//!   0 is scaled to `E*` along its own direction; a momentum whose `K₀` overflows is scaled to a finite `E*` too.
//! - §3.7, the collision pole `U = −∞` (a separation 0): `ENERGY_UNSCALABLE` under every finite `E*`, momenta
//!   untouched, nothing NaN, ahead of `E* < U` and of the rest start.
//! - §3.7, "a decode the mass block has already tagged keeps that tag".
//!
//! **Fuzzing**: `validation::prop::run`, `prop::CASES` cases (REQ-VAL-151), `z` over `[−4, 4]⁸`.
//!
//! **Tolerances** (no calibration requirement covers `η_E`; each bound is the round-off of the operations §3.7's
//! formula names, counted here, as in the TASK-M2-03 qa tests). The rescaled component `η_E·pᵢ` carries, relative to
//! itself: `E* − U` (½ eps), the kinetic sum `K₀` (squares, sum, division, sum over bodies: 3 eps), `(E* − U)/K₀`
//! (½ eps), the `sqrt` (halving the 4 eps before it, plus ½), the product (½), and a normalising division if one is
//! used (½): at most 3.5 eps. The kinetic energy squares it, 7 eps of `K`, and `K + U` adds ½ eps of the sum. The f64
//! evaluation of `K` here adds 4 roundings at f64. So `|E − E*| ≤ 12 · eps · (K + |U| + |E*|)` ([`ENERGY_ULPS`]), and
//! each component is within `4 · eps · η·s` of `η·pᵢ`, `s` the largest component ([`DIRECTION_ULPS`]).

use std::fmt::Debug;

use kernel::chart::{self, Chart, ChartOut, ValidationResult};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::descriptor::describe;
use kernel::decode::energy_norm::{self, Normalised, Outcome};
use kernel::decode::{decode, Decoded, Free, Ic, Latent};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::{ICDescriptorOf, PayloadReal};
use proptest::prelude::*;
use validation::negative_control;

/// `|E − E*|` in units of `eps · (K + |U| + |E*|)` (the module's note).
const ENERGY_ULPS: f64 = 12.0;

/// `|p'ᵢ − η·pᵢ|` in units of `eps · η · s` (the module's note).
const DIRECTION_ULPS: f64 = 4.0;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

trait Fp: LinkReal + PayloadReal + Debug + Copy + PartialEq + From<f32> + 'static {
    const EPS: f64;
    const LABEL: &'static str;
    /// The smallest positive subnormal.
    const TINY: f64;
    /// A momentum component whose square overflows.
    const HUGE: f64;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn bits(self) -> u64;
    fn up(self) -> Self;
    fn down(self) -> Self;
}

impl Fp for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
    const TINY: f64 = 5e-324;
    const HUGE: f64 = 1e200;
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn up(self) -> Self {
        self.next_up()
    }
    fn down(self) -> Self {
        self.next_down()
    }
}

impl Fp for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    const TINY: f64 = 1.401298464324817e-45;
    const HUGE: f64 = 1e30;
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn bits(self) -> u64 {
        self.to_bits() as u64
    }
    fn up(self) -> Self {
        self.next_up()
    }
    fn down(self) -> Self {
        self.next_down()
    }
}

// ── The entries under test, behind pointers ─────────────────────────────────────────────────────────────────────

/// `η_E` on a decode: [`energy_norm::apply`] with the free blocks.
type Apply<R> = fn(Decoded<R>, Option<R>, bool) -> (Decoded<R>, Outcome);

/// The rescale alone: [`energy_norm::normalise`].
type Norm<R> = fn(&Ic<R>, R, R, R) -> Normalised<R>;

fn real_apply<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    energy_norm::apply(&mut Free::<Defaults>::new(), d, e, forbids)
}

fn real_norm<R: Fp>(ic: &Ic<R>, v0: R, r_min: R, e: R) -> Normalised<R> {
    energy_norm::normalise(ic, v0, r_min, e)
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn plain<R: Fp>(z: [f64; 8]) -> Decoded<R> {
    decode::<Defaults, R>(&Latent::new(z.map(R::of)))
}

/// Every float of a decode's IC and descriptor, in bits, with its tag: equality is bit-for-bit.
fn fingerprint<R: Fp>(d: &Decoded<R>) -> (Vec<u64>, Option<Degenerate>) {
    let ic = &d.ic;
    let ds = &d.descriptor;
    let mut v: Vec<u64> = ic.m.iter().map(|x| x.bits()).collect();
    v.extend(ic.r.iter().flatten().map(|x| x.bits()));
    v.extend(ic.p.iter().flatten().map(|x| x.bits()));
    v.extend(
        [
            ds.m0,
            ds.m1,
            ds.m2,
            ds.q_mass,
            ds.rho_mag,
            ds.lambda_mag,
            ds.rho_ratio,
            ds.rho_angle,
            ds.K_0,
            ds.V_0,
            ds.virial_ratio,
            ds.r_min_pair_0,
        ]
        .iter()
        .map(|x| x.bits()),
    );
    (v, d.degenerate)
}

fn p_bits<R: Fp>(p: &[[R; 2]; 3]) -> Vec<u64> {
    p.iter().flatten().map(|x| x.bits()).collect()
}

/// `K = Σ‖pᵢ‖²/2mᵢ` in f64 from the IC's values.
fn kinetic64<R: Fp>(ic: &Ic<R>) -> f64 {
    (0..3)
        .map(|i| {
            let (px, py) = (ic.p[i][0].f(), ic.p[i][1].f());
            (px * px + py * py) / (2.0 * ic.m[i].f())
        })
        .sum()
}

fn largest<R: Fp>(p: &[[R; 2]; 3]) -> f64 {
    p.iter().flatten().map(|x| x.f().abs()).fold(0.0, f64::max)
}

fn at_rest<R: Fp>(p: &[[R; 2]; 3]) -> bool {
    p.iter().flatten().all(|x| x.f() == 0.0)
}

/// Asserts the rescaled IC `after` of the decode `before` reaches `e`: energy within [`ENERGY_ULPS`] from the IC and
/// from the descriptor, the momenta `η·pᵢ` along the decoded ones, masses and positions untouched, the descriptor that
/// of the rescaled IC, its `rho_angle` the decode's (R-628: read from the configuration block's own `(ρ̃, λ̃)`, which
/// `η_E` does not move).
fn assert_reaches<R: Fp>(decode: &Decoded<R>, after: &Decoded<R>, e: R, ctx: &str) {
    let before = &decode.ic;
    assert_eq!(
        (before.m, before.r),
        (after.ic.m, after.ic.r),
        "η_E moved a mass or a position ({ctx})"
    );
    // U is §3.6's V₀, the descriptor's (dd_decoder §3.7: "U = V₀").
    let u = after.descriptor.V_0.f();
    let k = kinetic64(&after.ic);
    let scale = R::EPS * (k + u.abs() + e.f().abs());
    let err = (k + u - e.f()).abs();
    assert!(
        err <= ENERGY_ULPS * scale,
        "the rescaled energy K + U = {} is not E* = {:?}: off by {:.2} eps-units ({ctx})",
        k + u,
        e,
        err / scale
    );
    let err_d = (after.descriptor.K_0.f() + after.descriptor.V_0.f() - e.f()).abs();
    assert!(
        err_d <= ENERGY_ULPS * scale,
        "the descriptor's K_0 + V_0 = {} is not E* = {:?} ({ctx})",
        after.descriptor.K_0.f() + after.descriptor.V_0.f(),
        e
    );
    // R-628: every field but rho_angle is read from the rescaled (m, r, p); rho_angle is read from the configuration
    // block, which η_E does not move, so it is the decode's, bit for bit (dd_decoder §3.6).
    assert_eq!(
        fingerprint(&Decoded {
            descriptor: ICDescriptorOf {
                rho_angle: decode.descriptor.rho_angle,
                ..describe(&after.ic)
            },
            ..*after
        }),
        fingerprint(after),
        "the descriptor does not describe the rescaled IC ({ctx})"
    );
    // pᵢ ← η_E·pᵢ, η_E = √((E* − U)/K₀), evaluated in f64.
    let eta = ((e.f() - u) / kinetic64(before)).sqrt();
    let s = largest(&before.p);
    for (a, b) in after.ic.p.iter().flatten().zip(before.p.iter().flatten()) {
        let want = eta * b.f();
        assert!(
            (a.f() - want).abs() <= DIRECTION_ULPS * R::EPS * eta * s,
            "the rescaled momentum {a:?} is not η_E·p = {want} ({ctx})"
        );
    }
}

/// The `z` of a moving decode: momentum controls non-zero.
fn moving(z: [f64; 8]) -> [f64; 8] {
    let mut z = z;
    if z[2..6].iter().all(|x| x.abs() < 0.05) {
        z[2] = 0.5;
    }
    z
}

fn rest(z: [f64; 8]) -> [f64; 8] {
    let mut z = z;
    z[2..6].copy_from_slice(&[0.0; 4]);
    z
}

fn zs() -> impl Strategy<Value = [f64; 8]> {
    prop::array::uniform8(-4.0f64..=4.0)
}

// ── REQ-DEC-024: with E* set, the decoded energy equals E* ──────────────────────────────────────────────────────

fn check_target<R: Fp>(apply: Apply<R>, z: [f64; 8], t: f64) {
    let before = plain::<R>(moving(z));
    assert!(
        !at_rest(&before.ic.p),
        "the fixture at z = {z:?} is not moving"
    );
    let v = before.descriptor.V_0.f();
    for e in [R::of(v + t * v.abs()), R::of(t), R::of(0.0)] {
        let ctx = format!("z = {z:?}, E* = {e:?}, {}", R::LABEL);
        let (after, outcome) = apply(before, Some(e), false);
        assert_eq!(
            outcome,
            Outcome::Scaled,
            "η_E was not applied at a feasible E* ({ctx})"
        );
        assert_eq!(
            after.degenerate, None,
            "a feasible E* tagged the decode ({ctx})"
        );
        assert_reaches(&before, &after, e, &ctx);
    }
}

fn check_targets<R: Fp>(apply: Apply<R>) {
    validation::prop::run(&(zs(), 0.0f64..=4.0), |(z, t)| {
        check_target::<R>(apply, z, t);
        Ok(())
    });
}

#[test]
fn qa_energy_normalisation_sets_target() {
    check_targets::<f64>(real_apply::<f64>);
    check_targets::<f32>(real_apply::<f32>);
}

/// Aims at `2E* − U`: the energy lands off target.
#[cfg(feature = "controls")]
fn overshoot<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    let u = d.descriptor.V_0;
    real_apply(d, e.map(|e| e + e - u), forbids)
}

negative_control!(
    qa_energy_normalisation_sets_target,
    "an η_E aiming at 2E* − U misses E*",
    expected = "is not E*",
    {
        check_targets::<f64>(overshoot::<f64>);
        check_targets::<f32>(overshoot::<f32>);
    }
);

// ── REQ-DEC-024: None is off, and a forbidding chart is off ─────────────────────────────────────────────────────

fn check_off<R: Fp>(apply: Apply<R>, z: [f64; 8]) {
    for z in [z, rest(z)] {
        let before = plain::<R>(z);
        let v = before.descriptor.V_0;
        let (after, outcome) = apply(before, None, false);
        let ctx = format!("z = {z:?}, {}", R::LABEL);
        assert_eq!(outcome, Outcome::Off, "None is not off ({ctx})");
        assert_eq!(
            fingerprint(&after),
            fingerprint(&before),
            "None changed the decode ({ctx})"
        );
        for e in [R::of(0.0), R::of(-0.0), v, v.down(), v.up(), R::of(3.0)] {
            let (after, outcome) = apply(before, Some(e), true);
            assert_eq!(
                outcome,
                Outcome::Off,
                "a forbidding chart applied η_E at E* = {e:?} ({ctx})"
            );
            assert_eq!(
                fingerprint(&after),
                fingerprint(&before),
                "a forbidding chart changed the decode at E* = {e:?} ({ctx})"
            );
        }
    }
}

fn check_offs<R: Fp>(apply: Apply<R>) {
    validation::prop::run(&zs(), |z| {
        check_off::<R>(apply, z);
        Ok(())
    });
}

#[test]
fn qa_energy_normalisation_none_is_off() {
    check_offs::<f64>(real_apply::<f64>);
    check_offs::<f32>(real_apply::<f32>);
}

/// Reads `None` as the target 0.
#[cfg(feature = "controls")]
fn none_as_zero<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    real_apply(d, Some(e.unwrap_or(R::of(0.0))), forbids)
}

/// Ignores the chart's flag.
#[cfg(feature = "controls")]
fn flag_ignored<R: Fp>(d: Decoded<R>, e: Option<R>, _forbids: bool) -> (Decoded<R>, Outcome) {
    real_apply(d, e, false)
}

negative_control!(
    qa_energy_normalisation_none_is_off,
    "an η_E reading None as E* = 0 is caught",
    expected = "None is not off",
    {
        check_offs::<f64>(none_as_zero::<f64>);
        check_offs::<f32>(none_as_zero::<f32>);
    }
);

#[cfg(feature = "controls")]
mod none_is_off_flag_ignored {
    use super::*;

    negative_control!(
        qa_energy_normalisation_none_is_off,
        "an η_E ignoring forbids_energy_normalisation is caught",
        expected = "a forbidding chart applied η_E",
        {
            check_offs::<f64>(flag_ignored::<f64>);
            check_offs::<f32>(flag_ignored::<f32>);
        }
    );
}

/// The latent chart with `forbids_energy_normalisation` set or not: the flag is the chart's own.
struct Flagged {
    inner: chart::Latent<f32>,
    forbids: bool,
}

impl Chart for Flagged {
    type Param = f32;
    fn map<F: spirv_std::num_traits::Float + From<f32>>(&self, u: F, v: F) -> ChartOut<F> {
        self.inner.map(u, v)
    }
    fn forbids_energy_normalisation(&self) -> bool {
        self.forbids
    }
    fn name(&self) -> &str {
        "Flagged"
    }
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.inner.validate(u, v)
    }
}

/// `D ∘ Φ` with `η_E` through a chart, its own flag deciding: [`energy_norm::decode_chart`].
type ChartEntry<R> = fn(&Flagged, R, R, Option<R>) -> (Decoded<R>, Outcome);

fn real_chart<R: Fp>(c: &Flagged, u: R, v: R, e: Option<R>) -> (Decoded<R>, Outcome) {
    energy_norm::decode_chart::<Defaults, R, _>(c, u, v, e)
}

fn check_chart_flag<R: Fp>(entry: ChartEntry<R>) {
    {
        let inner = chart::Latent {
            z0: [0.3, -0.2, 0.7, -0.4, 0.2, 0.9, 0.1, -0.3],
            q1: [1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0],
        };
        let (u, v) = (R::of(0.3), R::of(0.8));
        let e = R::of(0.0);
        let free = Flagged {
            inner,
            forbids: false,
        };
        let banned = Flagged {
            inner,
            forbids: true,
        };
        let none = entry(&free, u, v, None);
        let (on, outcome) = entry(&free, u, v, Some(e));
        assert_eq!(
            outcome,
            Outcome::Scaled,
            "an allowing chart did not apply η_E ({})",
            R::LABEL
        );
        assert_reaches(&none.0, &on, e, R::LABEL);
        let (off, outcome) = entry(&banned, u, v, Some(e));
        assert_eq!(
            outcome,
            Outcome::Off,
            "a forbidding chart applied η_E ({})",
            R::LABEL
        );
        assert_eq!(
            fingerprint(&off),
            fingerprint(&none.0),
            "a forbidding chart changed the decode"
        );
        assert_eq!(
            fingerprint(&off),
            fingerprint(&chart::decode::<Defaults, R, _>(&banned, u, v))
        );
    }
}

#[test]
fn qa_energy_normalisation_chart_flag() {
    check_chart_flag::<f64>(real_chart::<f64>);
    check_chart_flag::<f32>(real_chart::<f32>);
}

/// Reads every chart as allowing `η_E`.
#[cfg(feature = "controls")]
fn chart_flag_ignored<R: Fp>(c: &Flagged, u: R, v: R, e: Option<R>) -> (Decoded<R>, Outcome) {
    let open = Flagged {
        inner: c.inner,
        forbids: false,
    };
    energy_norm::decode_chart::<Defaults, R, _>(&open, u, v, e)
}

negative_control!(
    qa_energy_normalisation_chart_flag,
    "a chart entry ignoring the chart's own flag is caught",
    expected = "a forbidding chart applied η_E",
    {
        check_chart_flag::<f64>(chart_flag_ignored::<f64>);
        check_chart_flag::<f32>(chart_flag_ignored::<f32>);
    }
);

// ── REQ-DEC-024: E* = 0 is a real target; −0.0 is the same target ───────────────────────────────────────────────

fn check_zero<R: Fp>(apply: Apply<R>, z: [f64; 8]) {
    let before = plain::<R>(moving(z));
    let (none, _) = apply(before, None, false);
    for e in [R::of(0.0), R::of(-0.0)] {
        let ctx = format!("z = {z:?}, E* = {e:?}, {}", R::LABEL);
        let (after, outcome) = apply(before, Some(e), false);
        assert_ne!(outcome, Outcome::Off, "E* = 0 was read as off ({ctx})");
        assert_eq!(
            outcome,
            Outcome::Scaled,
            "E* = 0 on a bound IC was not applied ({ctx})"
        );
        assert_reaches(&before, &after, e, &ctx);
        let (k0, u0) = (kinetic64(&before.ic), before.descriptor.V_0.f());
        if (k0 + u0).abs() > 1e3 * R::EPS * (k0 + u0.abs()) {
            assert_ne!(
                p_bits(&after.ic.p),
                p_bits(&none.ic.p),
                "E* = 0 left the momenta as None does ({ctx})"
            );
        }
    }
    // At the rest start E* = 0 > U: a target, tagged, never off.
    let still = plain::<R>(rest(z));
    for e in [R::of(0.0), R::of(-0.0)] {
        let (after, outcome) = apply(still, Some(e), false);
        assert_eq!(
            outcome,
            Outcome::Unscalable,
            "E* = {e:?} at the rest start is not ENERGY_UNSCALABLE (z = {z:?}, {})",
            R::LABEL
        );
        assert_eq!(after.degenerate, Some(Degenerate::EnergyUnscalable));
    }
}

fn check_zeros<R: Fp>(apply: Apply<R>) {
    validation::prop::run(&zs(), |z| {
        check_zero::<R>(apply, z);
        Ok(())
    });
}

#[test]
fn qa_energy_normalisation_zero_target() {
    check_zeros::<f64>(real_apply::<f64>);
    check_zeros::<f32>(real_apply::<f32>);
}

/// Reads a zero target, of either sign, as off: the `E* = 0` sentinel R-25 forbids.
#[cfg(feature = "controls")]
fn zero_as_off<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    real_apply(d, e.filter(|e| e.f() != 0.0), forbids)
}

/// Reads `−0.0` as off: a sign-bit test.
#[cfg(feature = "controls")]
fn negative_zero_as_off<R: Fp>(
    d: Decoded<R>,
    e: Option<R>,
    forbids: bool,
) -> (Decoded<R>, Outcome) {
    real_apply(
        d,
        e.filter(|e| !(e.f() == 0.0 && e.f().is_sign_negative())),
        forbids,
    )
}

negative_control!(
    qa_energy_normalisation_zero_target,
    "an η_E reading E* = 0 as off is caught",
    expected = "was read as off",
    {
        check_zeros::<f64>(zero_as_off::<f64>);
        check_zeros::<f32>(zero_as_off::<f32>);
    }
);

#[cfg(feature = "controls")]
mod zero_target_negative_zero {
    use super::*;

    negative_control!(
        qa_energy_normalisation_zero_target,
        "an η_E reading E* = −0.0 as off is caught",
        expected = "E* = -0.0",
        {
            check_zeros::<f64>(negative_zero_as_off::<f64>);
            check_zeros::<f32>(negative_zero_as_off::<f32>);
        }
    );
}

// ── REQ-DEC-041 / §3.7: the feasibility boundary E* = U and the rest start ───────────────────────────────────────

fn check_boundary<R: Fp>(apply: Apply<R>, z: [f64; 8]) {
    // Moving bodies.
    let before = plain::<R>(moving(z));
    let u = before.descriptor.V_0;
    let ctx = format!("z = {z:?}, U = {u:?}, {}", R::LABEL);
    let (at, outcome) = apply(before, Some(u), false);
    assert_eq!(
        outcome,
        Outcome::Scaled,
        "E* = U with moving bodies is not scaled ({ctx})"
    );
    assert_eq!(at.degenerate, None, "E* = U tagged the decode ({ctx})");
    assert!(
        at_rest(&at.ic.p),
        "E* = U did not scale to rest: {:?} ({ctx})",
        at.ic.p
    );
    let (below, outcome) = apply(before, Some(u.down()), false);
    assert_eq!(
        outcome,
        Outcome::Infeasible,
        "E* one ulp below U is not ENERGY_INFEASIBLE ({ctx})"
    );
    assert_eq!(
        below.degenerate,
        Some(Degenerate::EnergyInfeasible),
        "the tag below U ({ctx})"
    );
    assert_eq!(
        p_bits(&below.ic.p),
        p_bits(&before.ic.p),
        "an infeasible decode did not keep its momenta ({ctx})"
    );
    assert_eq!(
        fingerprint(&below).0,
        fingerprint(&before).0,
        "an infeasible decode changed ({ctx})"
    );
    let (above, outcome) = apply(before, Some(u.up()), false);
    assert_eq!(
        outcome,
        Outcome::Scaled,
        "E* one ulp above U is not scaled ({ctx})"
    );
    assert_eq!(
        above.degenerate, None,
        "E* one ulp above U tagged the decode ({ctx})"
    );
    assert!(
        !at_rest(&above.ic.p),
        "E* one ulp above U gave rest ({ctx})"
    );
    assert_reaches(&before, &above, u.up(), &ctx);

    // The rest start.
    let still = plain::<R>(rest(z));
    assert!(at_rest(&still.ic.p), "z_q = 0 is not the rest start");
    let u = still.descriptor.V_0;
    let ctx = format!("rest, z = {z:?}, U = {u:?}, {}", R::LABEL);
    for (e, want, tag) in [
        (u, Outcome::AtTarget, None),
        (
            u.up(),
            Outcome::Unscalable,
            Some(Degenerate::EnergyUnscalable),
        ),
        (
            u.down(),
            Outcome::Infeasible,
            Some(Degenerate::EnergyInfeasible),
        ),
        (
            R::of(1.0),
            Outcome::Unscalable,
            Some(Degenerate::EnergyUnscalable),
        ),
    ] {
        let (after, outcome) = apply(still, Some(e), false);
        assert_eq!(outcome, want, "the rest start at E* = {e:?} ({ctx})");
        assert_eq!(
            after.degenerate, tag,
            "the rest start's tag at E* = {e:?} ({ctx})"
        );
        assert_eq!(
            fingerprint(&after).0,
            fingerprint(&still).0,
            "the rest start changed at E* = {e:?} ({ctx})"
        );
    }
}

fn check_boundaries<R: Fp>(apply: Apply<R>) {
    validation::prop::run(&zs(), |z| {
        check_boundary::<R>(apply, z);
        Ok(())
    });
}

#[test]
fn qa_energy_normalisation_feasibility_boundary() {
    check_boundaries::<f64>(real_apply::<f64>);
    check_boundaries::<f32>(real_apply::<f32>);
}

/// Reads `E* = U` as infeasible: `E* ≤ U`.
#[cfg(feature = "controls")]
fn closed_infeasible<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    match e {
        Some(e) if e == d.descriptor.V_0 && !forbids => {
            let (mut out, _) = real_apply(d, Some(e.down()), forbids);
            out.degenerate = d.degenerate.or(Some(Degenerate::EnergyInfeasible));
            (out, Outcome::Infeasible)
        }
        _ => real_apply(d, e, forbids),
    }
}

/// Tags the rest start at `E* = U` too.
#[cfg(feature = "controls")]
fn rest_always_tagged<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    let (mut out, outcome) = real_apply(d, e, forbids);
    if outcome == Outcome::AtTarget {
        out.degenerate = Some(Degenerate::EnergyUnscalable);
        return (out, Outcome::Unscalable);
    }
    (out, outcome)
}

negative_control!(
    qa_energy_normalisation_feasibility_boundary,
    "an η_E reading E* = U as infeasible is caught",
    expected = "E* = U with moving bodies is not scaled",
    {
        check_boundaries::<f64>(closed_infeasible::<f64>);
        check_boundaries::<f32>(closed_infeasible::<f32>);
    }
);

#[cfg(feature = "controls")]
mod feasibility_boundary_rest_tagged {
    use super::*;

    negative_control!(
        qa_energy_normalisation_feasibility_boundary,
        "an η_E tagging the rest start at E* = U is caught",
        expected = "the rest start at E*",
        {
            check_boundaries::<f64>(rest_always_tagged::<f64>);
            check_boundaries::<f32>(rest_always_tagged::<f32>);
        }
    );
}

// ── §3.7: K₀ = 0 means every pᵢ is 0, not that the rounded K₀ is 0 ───────────────────────────────────────────────

/// The golden IC's masses and positions with the momenta `p`.
fn golden_with<R: Fp>(p: [[f64; 2]; 3]) -> Ic<R> {
    let g = plain::<R>([0.0; 8]).ic;
    Ic {
        m: g.m,
        r: g.r,
        p: p.map(|b| b.map(R::of)),
    }
}

fn check_tiny<R: Fp>(norm: Norm<R>) {
    let (t, h) = (R::TINY, R::HUGE);
    for p in [
        [[t, 0.0], [-t, 0.0], [0.0, 0.0]],
        [[0.0, 0.0], [0.0, t], [0.0, -t]],
        [[0.0, -t], [0.0, 0.0], [0.0, t]],
        [[h, 0.0], [-h, 0.0], [0.0, 0.0]],
        [[0.0, h], [h, 0.0], [-h, -h]],
    ] {
        let ic = golden_with::<R>(p);
        assert!(!at_rest(&ic.p), "the fixture {p:?} is at rest");
        let d = describe(&ic);
        let k0 = d.K_0.f();
        assert!(
            k0 == 0.0 || k0.is_infinite(),
            "the fixture {p:?}'s K₀ = {k0} neither rounds to 0 nor overflows ({})",
            R::LABEL
        );
        for e in [R::of(0.0), R::of(-0.1), R::of(2.0)] {
            let ctx = format!("p = {p:?}, K₀ = {k0}, E* = {e:?}, {}", R::LABEL);
            let n = norm(&ic, d.V_0, d.r_min_pair_0, e);
            assert_eq!(
                n.outcome,
                Outcome::Scaled,
                "a non-zero momentum was not scaled ({ctx})"
            );
            let after = Ic { p: n.p, ..ic };
            let u = d.V_0.f();
            let k = kinetic64(&after);
            let scale = R::EPS * (k + u.abs() + e.f().abs());
            assert!(
                (k + u - e.f()).abs() <= ENERGY_ULPS * scale,
                "a non-zero momentum was not scaled to E*: K + U = {} ({ctx})",
                k + u
            );
            // Along its own direction: the zero components stay zero, the signs and ratios kept.
            for (a, b) in after.p.iter().flatten().zip(p.iter().flatten()) {
                if *b == 0.0 {
                    assert_eq!(a.f(), 0.0, "a zero component was moved ({ctx})");
                } else {
                    assert_eq!(
                        a.f().signum(),
                        b.signum(),
                        "a component's sign changed ({ctx})"
                    );
                }
            }
            let s = largest(&after.p);
            for (a, b) in after.p.iter().flatten().zip(p.iter().flatten()) {
                let want = s * (b / p.iter().flatten().fold(0.0f64, |m, x| m.max(x.abs())));
                assert!(
                    (a.f() - want).abs() <= DIRECTION_ULPS * R::EPS * s,
                    "the rescaled momentum is not along the decoded one ({ctx})"
                );
            }
        }
    }
    // Exactly zero is the rest start.
    let ic = golden_with::<R>([[0.0, -0.0], [-0.0, 0.0], [0.0, 0.0]]);
    let d = describe(&ic);
    let n = norm(&ic, d.V_0, d.r_min_pair_0, R::of(0.0));
    assert_eq!(
        n.outcome,
        Outcome::Unscalable,
        "exact rest at E* > U ({})",
        R::LABEL
    );
}

#[test]
fn qa_energy_normalisation_tiny_momentum() {
    check_tiny::<f64>(real_norm::<f64>);
    check_tiny::<f32>(real_norm::<f32>);
}

/// Reads the rounded `K₀ = 0` as the rest start.
#[cfg(feature = "controls")]
fn rounded_rest<R: Fp>(ic: &Ic<R>, v0: R, r_min: R, e: R) -> Normalised<R> {
    if describe(ic).K_0.f() == 0.0 && e != v0 && e > v0 && r_min != R::of(0.0) {
        return Normalised {
            p: ic.p,
            outcome: Outcome::Unscalable,
        };
    }
    real_norm(ic, v0, r_min, e)
}

negative_control!(
    qa_energy_normalisation_tiny_momentum,
    "an η_E reading a rounded-to-zero K₀ as the rest start is caught",
    expected = "a non-zero momentum was not scaled",
    {
        check_tiny::<f64>(rounded_rest::<f64>);
        check_tiny::<f32>(rounded_rest::<f32>);
    }
);

// ── §3.7: the collision pole U = −∞ ─────────────────────────────────────────────────────────────────────────────

/// The golden IC with body 1 moved onto body 0, moving.
fn collided<R: Fp>(p: [[f64; 2]; 3]) -> Ic<R> {
    let mut ic = golden_with::<R>(p);
    ic.r[1] = ic.r[0];
    ic
}

fn check_collision<R: Fp>(norm: Norm<R>, apply: Apply<R>) {
    for p in [
        [[0.3, -0.1], [-0.2, 0.4], [-0.1, -0.3]],
        [[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
    ] {
        let ic = collided::<R>(p);
        let d = describe(&ic);
        assert!(
            d.V_0.f() == f64::NEG_INFINITY && d.r_min_pair_0.f() == 0.0,
            "the fixture is not at the collision pole: V_0 = {:?}, r_min_pair_0 = {:?}",
            d.V_0,
            d.r_min_pair_0
        );
        for e in [
            R::of(0.0),
            R::of(-0.0),
            R::of(-1e30),
            R::of(1e30),
            R::of(-0.5),
            R::of(5.0),
        ] {
            let ctx = format!("p = {p:?}, E* = {e:?}, {}", R::LABEL);
            let n = norm(&ic, d.V_0, d.r_min_pair_0, e);
            assert_eq!(
                n.outcome,
                Outcome::Unscalable,
                "the collision pole is not ENERGY_UNSCALABLE ({ctx})"
            );
            assert_eq!(
                p_bits(&n.p),
                p_bits(&ic.p),
                "the collision pole changed the momenta ({ctx})"
            );
            // Through a decode: the descriptor the rest of the decoder wrote, the tag added.
            let mut base = plain::<R>([0.0; 8]);
            base.ic = ic;
            base.descriptor = d;
            let (after, outcome) = apply(base, Some(e), false);
            assert_eq!(
                outcome,
                Outcome::Unscalable,
                "the collision pole through a decode ({ctx})"
            );
            assert_eq!(
                after.degenerate,
                Some(Degenerate::EnergyUnscalable),
                "the pole's tag ({ctx})"
            );
            assert!(
                after.ic.p.iter().flatten().all(|x| !x.f().is_nan())
                    && !after.descriptor.K_0.f().is_nan(),
                "the collision pole stored NaN ({ctx})"
            );
        }
    }
}

#[test]
fn qa_energy_normalisation_collision_pole() {
    check_collision::<f64>(real_norm::<f64>, real_apply::<f64>);
    check_collision::<f32>(real_norm::<f32>, real_apply::<f32>);
}

/// Misses the pole: reads the separation as non-zero.
#[cfg(feature = "controls")]
fn pole_missed<R: Fp>(ic: &Ic<R>, v0: R, _r_min: R, e: R) -> Normalised<R> {
    real_norm(ic, v0, R::of(1.0), e)
}

negative_control!(
    qa_energy_normalisation_collision_pole,
    "an η_E that does not test the separation is caught",
    expected = "the collision pole is not ENERGY_UNSCALABLE",
    {
        check_collision::<f64>(pole_missed::<f64>, real_apply::<f64>);
        check_collision::<f32>(pole_missed::<f32>, real_apply::<f32>);
    }
);

// ── §3.7: a decode the mass block has already tagged keeps that tag ─────────────────────────────────────────────

fn check_earlier<R: Fp>(apply: Apply<R>) {
    for z in [
        [0.4, 0.2, 0.5, -0.3, 0.1, 0.7, 0.2, -0.1],
        rest([0.4, 0.2, 0.0, 0.0, 0.0, 0.0, 0.2, -0.1]),
    ] {
        let mut d = plain::<R>(z);
        d.degenerate = Some(Degenerate::M01Tiny);
        let u = d.descriptor.V_0;
        for e in [u.down(), u.up(), R::of(0.0), R::of(1.0)] {
            let (after, _) = apply(d, Some(e), false);
            assert_eq!(
                after.degenerate,
                Some(Degenerate::M01Tiny),
                "a decode tagged M01_TINY lost its tag at E* = {e:?} (z = {z:?}, {})",
                R::LABEL
            );
        }
    }
}

#[test]
fn qa_energy_normalisation_keeps_earlier_tag() {
    check_earlier::<f64>(real_apply::<f64>);
    check_earlier::<f32>(real_apply::<f32>);
}

/// Overwrites the earlier tag with η_E's.
#[cfg(feature = "controls")]
fn later_wins<R: Fp>(d: Decoded<R>, e: Option<R>, forbids: bool) -> (Decoded<R>, Outcome) {
    let earlier = d.degenerate;
    let (mut out, outcome) = real_apply(
        Decoded {
            degenerate: None,
            ..d
        },
        e,
        forbids,
    );
    out.degenerate = outcome.tag().or(earlier);
    (out, outcome)
}

negative_control!(
    qa_energy_normalisation_keeps_earlier_tag,
    "an η_E overwriting M01_TINY is caught",
    expected = "lost its tag",
    {
        check_earlier::<f64>(later_wins::<f64>);
        check_earlier::<f32>(later_wins::<f32>);
    }
);
