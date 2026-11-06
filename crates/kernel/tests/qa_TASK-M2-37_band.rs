//! qa's tests for TASK-M2-37, chart_reference §2.1's floor at its two far ends, "the floor is the least normal float
//! §2.2's exact test finds feasible at the held `L_z`, `L_z²/2` at `I = 1` rounded, raised one float where it rounded
//! below, and never subnormal" and "a held `L_z` whose `L_z²` has no finite float at the chart's own precision has no
//! floor, and every pixel of that chart is not a system, §2.2's exact test labelling it and `validate` agreeing at that
//! precision". Written from that paragraph; the floats are recomputed exactly, not by the code under test. Each test
//! has its negative control (R-176).
//!
//! - `qa_energy_axes_warped_floor_exact_f32` and `_f64`: the floor against an exact oracle: `f64` holds an `f32`'s
//!   square exactly, and `BigUint` an `f64`'s, over a stride of every binade and the floats around the square's underflow
//!   and overflow. Control: `l²/2` as rounded.
//! - `qa_energy_axes_warped_floor_band`: around the greatest `L_z` whose square has a finite float, at f32 and at f64,
//!   on `K`, `KE`, `E` and the virial axis: `validate` and the decode at the chart's own precision agree, and they agree
//!   with §2.1: a held value with a floor is a system at every pixel, one with none is not at any, labelled, its momenta
//!   finite. Control: a held value without a floor read as one with.
//! - `qa_energy_axes_warped_floor_no_value`: a held `L_z` that is no number has no floor either: every axis refuses
//!   every pixel, at both precisions, with no panic. Control: a finite spin, a system at every pixel.
//!
//! **Values.** `K_max` and `γ_K` are REQ-CHART-044's placeholders (R-71). Every slice, range and held value is test data.

use kernel::chart::energy::{Mapping, Quantity};
use kernel::chart::invariant::{k_floor, WarpReal};
use kernel::chart::mixed::ZMu1;
use kernel::chart::physical::{AxisE, AxisK, AxisKE, AxisVirial, EnergyAxis, MixedEnergy};
use kernel::chart::{Chart, Constraint, ValidationResult};
use kernel::decode::invariant::{infeasible, Outcome};
use kernel::generated::links::{Defaults, LinkParams, LinkReal};
use kernel::payload::PayloadReal;
use num_bigint::BigUint;
use validation::{negative_control, GAMMA_K, K_MAX};

/// The slice centre (test data; `qa_TASK-M2-37.rs`'s).
const Z0: [f64; 8] = [-0.2, 0.3, 0.1, -0.2, 0.05, 0.15, 0.5, -0.3];
const RANGE: [f64; 2] = [-1.0, 1.0];
/// The virial axis's `Q_max` (test data).
const Q_MAX: f64 = 1.5;

// ── The floor, against an exact oracle ───────────────────────────────────────────────────────────────────────────────

/// The floats tested at f32: a stride over every positive `f32`, and the floats around where `l²/2` meets the least
/// normal float, the top of the band and the greatest finite float.
fn f32_sweep() -> Vec<f32> {
    let around = |l: f32, n: u32| {
        let b = l.to_bits();
        (b - n..=b + n).map(f32::from_bits).collect::<Vec<_>>()
    };
    let mut all: Vec<f32> = (1..0x7F80_0000u32)
        .step_by(1021)
        .map(f32::from_bits)
        .collect();
    all.extend(around(1.0842022e-19, 4096));
    all.extend(around(1.5340e-19, 4096));
    all.extend(around(1.8446743e19, 4096));
    all.extend(around(1.3043817e19, 4096));
    all
}

/// The exact `f32` floor at `l`: `None` where the exact `l²` passes the greatest float.
fn f32_oracle(l: f32) -> Option<f32> {
    let exact = f64::from(l) * f64::from(l);
    if exact > f64::from(f32::MAX) {
        return None;
    }
    let half = exact / 2.0;
    let near = half as f32;
    let up = if f64::from(near) < half {
        f32::from_bits(near.to_bits() + 1)
    } else {
        near
    };
    Some(up.max(f32::MIN_POSITIVE))
}

fn check_exact_f32(floor: fn(f32) -> f32) {
    for l in f32_sweep() {
        for l in [l, -l] {
            let k = floor(l);
            match f32_oracle(l) {
                Some(want) => assert!(
                    k == want,
                    "at f32 L_z = {l:e} the floor {k:e} is not {want:e}"
                ),
                None => assert!(
                    k == f32::MAX / 2.0 && infeasible(1.0, k, l),
                    "at f32 L_z = {l:e}, no finite square, the floor is {k:e}"
                ),
            }
        }
    }
}

#[test]
fn qa_energy_axes_warped_floor_exact_f32() {
    check_exact_f32(k_floor);
}

negative_control!(
    qa_energy_axes_warped_floor_exact_f32,
    "l²/2 as rounded, with no step up or least normal float, is not the least feasible normal float",
    expected = "is not",
    check_exact_f32(|l| l * l / 2.0)
);

/// `x`, positive and finite, as `m·2^e`.
fn parts(x: f64) -> (BigUint, i32) {
    let bits = x.to_bits();
    let raw = ((bits >> 52) & 0x7FF) as i32;
    let frac = bits & ((1 << 52) - 1);
    let (m, e) = if raw == 0 {
        (frac, -1074)
    } else {
        (frac | (1 << 52), raw - 1075)
    };
    (BigUint::from(m), e)
}

/// Whether `2·k ≥ l²`, exactly.
fn holds(k: f64, l: f64) -> bool {
    let (mk, ek) = parts(k);
    let (ml, el) = parts(l);
    let (ek, el2) = (ek + 1, 2 * el);
    let low = ek.min(el2);
    (mk << (ek - low) as usize) >= (&ml * &ml) << (el2 - low) as usize
}

/// The floats tested at f64.
fn f64_sweep() -> Vec<f64> {
    let around = |l: f64, n: u64| {
        let b = l.to_bits();
        (b - n..=b + n).map(f64::from_bits).collect::<Vec<_>>()
    };
    let mut all: Vec<f64> = (1..0x7FF0_0000_0000_0000u64)
        .step_by(0x0000_7FF3_0000_1235)
        .map(f64::from_bits)
        .collect();
    all.extend(around(f64::MAX.sqrt(), 512));
    all.extend(around((f64::MAX / 2.0).sqrt(), 512));
    all.extend(around(f64::MIN_POSITIVE.sqrt(), 512));
    all.extend(around((2.0 * f64::MIN_POSITIVE).sqrt(), 512));
    all
}

fn check_exact_f64(floor: fn(f64) -> f64) {
    for l in f64_sweep() {
        let k = floor(l);
        if holds(f64::MAX / 2.0, l) {
            assert!(
                k.is_normal(),
                "at L_z = {l:e} the floor {k:e} is not normal"
            );
            assert!(
                holds(k, l),
                "at L_z = {l:e} the floor {k:e} is below L_z²/2"
            );
            let less = f64::from_bits(k.to_bits() - 1);
            assert!(
                !less.is_normal() || !holds(less, l),
                "at L_z = {l:e} the floor {k:e} is not the least"
            );
        } else {
            assert!(
                k == f64::MAX / 2.0 && infeasible(1.0, k, l),
                "at L_z = {l:e}, no finite square, the floor is {k:e}"
            );
        }
    }
}

#[test]
fn qa_energy_axes_warped_floor_exact_f64() {
    check_exact_f64(k_floor);
}

negative_control!(
    qa_energy_axes_warped_floor_exact_f64,
    "l²/2 as rounded, with no step up or least normal float, is not the floor",
    expected = "at L_z",
    check_exact_f64(|l| l * l / 2.0)
);

// ── The band ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A float a chart is held at.
trait Held: WarpReal + LinkReal + PayloadReal + core::fmt::LowerExp {
    fn of(x: f64) -> Self;
}

impl Held for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl Held for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// The lone axis `Q` beside `z_μ₁` at the held `L_z = held`, warped, at the chart's precision `P`.
fn chart<Q: EnergyAxis, P: Held>(held: P) -> MixedEnergy<ZMu1, Q, Defaults, P>
where
    f64: From<P>,
{
    let top = if Q::QUANTITY == Quantity::VirialRatio {
        Q_MAX
    } else {
        K_MAX.number()
    };
    MixedEnergy::new(
        Z0.map(P::of),
        RANGE.map(P::of),
        P::of(top),
        P::of(GAMMA_K.number()),
        held,
        Mapping::Warped,
    )
}

/// `validate` and the decode at the chart's own precision, on a coarse grid of `[0,1]²`, each as §2.1 says: `system`,
/// valid, feasible and finite at every pixel; else refused and labelled, its momenta finite.
fn check_axis<Q: EnergyAxis, P: Held>(held: P, system: bool)
where
    f64: From<P>,
    MixedEnergy<ZMu1, Q, Defaults, P>: Chart<Param = P>,
{
    let name = Q::QUANTITY;
    let c = chart::<Q, P>(held);
    for u in [0.0, 0.5, 1.0] {
        for v in [0.0, 0.5, 1.0] {
            let valid = c.validate(u, v);
            let (d, outcome) = c.decode_from(c.frozen, P::of(u), P::of(v), &LinkParams::DEFAULT);
            let finite = d.ic.p.iter().flatten().all(|x| x.is_finite());
            assert!(
                finite,
                "{name:?} at the held L_z = {held:e}: ({u}, {v}) decodes momenta that are not finite: {outcome:?}"
            );
            if system {
                assert!(
                    valid == ValidationResult::Valid && outcome == Outcome::Feasible,
                    "{name:?} at the held L_z = {held:e}: ({u}, {v}) is not a system: {valid:?}, {outcome:?}"
                );
            } else {
                assert!(
                    valid
                        == ValidationResult::Reject {
                            failed: Constraint::KAboveLzBound
                        }
                        && outcome == Outcome::Infeasible,
                    "{name:?} at the held L_z = {held:e}: ({u}, {v}) is a system: {valid:?}, {outcome:?}"
                );
            }
        }
    }
}

fn check_all<P: Held>(held: P, system: bool)
where
    f64: From<P>,
    MixedEnergy<ZMu1, AxisK, Defaults, P>: Chart<Param = P>,
    MixedEnergy<ZMu1, AxisKE, Defaults, P>: Chart<Param = P>,
    MixedEnergy<ZMu1, AxisE, Defaults, P>: Chart<Param = P>,
    MixedEnergy<ZMu1, AxisVirial, Defaults, P>: Chart<Param = P>,
{
    check_axis::<AxisK, P>(held, system);
    check_axis::<AxisKE, P>(held, system);
    check_axis::<AxisE, P>(held, system);
    check_axis::<AxisVirial, P>(held, system);
}

/// The greatest `f32` whose exact square is at most the greatest float, by [`f32_oracle`].
fn top_f32() -> f32 {
    let mut l = f32::from_bits(f32::MAX.sqrt().to_bits() + 4);
    while f32_oracle(l).is_none() {
        l = f32::from_bits(l.to_bits() - 1);
    }
    l
}

/// The greatest `f64` whose exact square is at most the greatest float, by [`holds`].
fn top_f64() -> f64 {
    let mut l = f64::from_bits(f64::MAX.sqrt().to_bits() + 4);
    while !holds(f64::MAX / 2.0, l) {
        l = f64::from_bits(l.to_bits() - 1);
    }
    l
}

/// The band at both precisions; `wrong`, the control, reads the first held value past the top as a system.
fn check_band(wrong: bool) {
    let top32 = top_f32();
    for k in 0..4 {
        let l = f32::from_bits(top32.to_bits() - k * k * 64);
        check_all(l, true);
        check_all(-l, true);
    }
    check_all(top32 * 0.75, true);
    check_all(top32 * 0.5, true);
    check_all(f32::from_bits(top32.to_bits() + 1), wrong);
    check_all(f32::from_bits(top32.to_bits() + 4096), false);
    check_all(-f32::from_bits(top32.to_bits() + 1), false);

    let top64 = top_f64();
    for k in 0..4 {
        let l = f64::from_bits(top64.to_bits() - k * k * 1024);
        check_all(l, true);
        check_all(-l, true);
    }
    check_all(top64 * 0.75, true);
    check_all(top64 * 0.5, true);
    check_all(f64::from_bits(top64.to_bits() + 1), false);
    check_all(f64::from_bits(top64.to_bits() + (1 << 20)), false);
}

#[test]
fn qa_energy_axes_warped_floor_band() {
    check_band(false);
}

negative_control!(
    qa_energy_axes_warped_floor_band,
    "the first held value with no finite square has no floor, so it is not a system",
    expected = "is not a system",
    check_band(true)
);

/// Each axis at the held `held`, at the chart's own precision, refuses every pixel of the grid.
fn check_no_value_axis<Q: EnergyAxis, P: Held>(held: P)
where
    f64: From<P>,
    MixedEnergy<ZMu1, Q, Defaults, P>: Chart<Param = P>,
{
    let c = chart::<Q, P>(held);
    for u in [0.0, 0.5, 1.0] {
        for v in [0.0, 0.5, 1.0] {
            assert!(
                c.validate(u, v)
                    == ValidationResult::Reject {
                        failed: Constraint::KAboveLzBound
                    },
                "{:?} at the held L_z = {held:e}: ({u}, {v}) is not refused",
                Q::QUANTITY
            );
        }
    }
}

fn check_no_value(values32: &[f32], values64: &[f64]) {
    for &l in values32 {
        check_no_value_axis::<AxisK, f32>(l);
        check_no_value_axis::<AxisKE, f32>(l);
        check_no_value_axis::<AxisE, f32>(l);
        check_no_value_axis::<AxisVirial, f32>(l);
    }
    for &l in values64 {
        check_no_value_axis::<AxisK, f64>(l);
        check_no_value_axis::<AxisKE, f64>(l);
        check_no_value_axis::<AxisE, f64>(l);
        check_no_value_axis::<AxisVirial, f64>(l);
    }
}

#[test]
fn qa_energy_axes_warped_floor_no_value() {
    check_no_value(
        &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY],
        &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY],
    );
}

negative_control!(
    qa_energy_axes_warped_floor_no_value,
    "a finite spin has its floor, a system at every pixel, so it is not refused",
    expected = "is not refused",
    check_no_value(&[0.3], &[0.3])
);
