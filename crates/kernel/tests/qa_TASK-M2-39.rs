//! QA's tests for TASK-M2-39, written from REQ-CHART-076 (R-512 F1, R-508 V2 and V5, R-472 X3a) and the chart decoder
//! contract § "Tilt (basis edit)", not from the implementation.
//!
//! REQ-CHART-076: under Tip as sheet with one target and both angles set, a window pixel whose own chart parameters
//! `(a, b)` leave the chart's square is past the chart's edge: its axes' types act on `(a, b)` as past the window, so past
//! a bounded edge (a warped `L_z`'s `s` on either side, the warp's `t < 0`) it is not a system and draws blank, and past
//! the energy warp's continuing `t > 1` side it is that side's system.
//!
//! What these tests add to the implementer's `tip_sheet_in_window`:
//! - **The rotation from the contract.** Each pixel's `(a, b, d)` is computed here from the contract's words, Rodrigues'
//!   turn of `span(e_a, e_b)` by `m = √(τ₁² + τ₂²)` about `k = (τ₂ e_a − τ₁ e_b)/m` in `(e_a, e_b, e_X)`, `d` in units of
//!   the target's slider span (R-507), and each pixel is classified by that `(a, b)`, not by the view's own placement.
//! - **"As past the window", pixel by pixel.** A tilted pixel is a system exactly when the untilted chart, sliced to the
//!   pixel's own hidden value (the slice centre plus its `d`), is one at `(a, b)`; where it is, it decodes bit-identically
//!   to that sliced chart at `(a, b)`, and where `d` moved a mass the decode is not the unsliced chart's.
//! - **Every Tip-as-sheet tilt, not only the two the verify names.** A sweep of both-angles-set tilts over each angle's
//!   range `[−180°, 180°]` (the contract's range under Tip as sheet), past the 180° total-tip wrap included, into a
//!   configuration target and a mass target, on both invariant energy charts, `(L_z, K)` and `(L_z, E)` (R-508 V1), each
//!   as itself and as its warped `Mapped` form.
//!
//! Negative controls (R-176), each must fail: the rotation turned the wrong way, at the named tilts and over the sweep;
//! the classification read at the window point, the types acting before the tilt; the reference decode not sliced to the
//! pixel's hidden value; and a draw that blanks the continuing `t > 1` side.

use kernel::chart::invariant::Invariant;
use kernel::chart::tilt::Tiltable;
use kernel::chart::{
    Chart, InvariantLE, InvariantLK, Mapped, Mapping, Tilt, TiltMap, TiltSpec, Tilted,
    ValidationResult,
};
use kernel::decode::Decoded;
use kernel::generated::links::Defaults;
use validation::chart_class::forbidden;
use validation::{negative_control, GAMMA_K, K_MAX};

/// A latent slider's span (R-920: rail `[−1, 1]`), the unit of a latent target's offset `d` (R-507).
const SPAN: f64 = 2.0;
/// The tilts REQ-CHART-076's verify names, `(τ₁, τ₂)` in degrees.
const NAMED: [[f64; 2]; 2] = [[30.0, 12.4], [-45.0, 77.9]];
/// Each angle of the sweep, in degrees, both signs, over Tip as sheet's range `[−180°, 180°]`; the pairs with total tip
/// past 180° exercise the contract's wrap.
const ANGLES: [f64; 10] = [
    -170.0, -120.0, -75.0, -30.0, -8.0, 8.0, 30.0, 75.0, 120.0, 170.0,
];
/// Hidden targets of the invariant charts: a configuration control `z[0]` and a mass logit `z[6]` (their hidden
/// coordinates are `z_cfg` and `z_mass`, chart_decoder_contract § "Tilt (basis edit)", the controls the axes do not write).
const TARGETS: [u32; 2] = [0, 6];
/// A guard far above the rounding of the dozen f64 operations computing `(a, b)` here and in the view (each ~1e-16 on
/// values of order 1): a pixel this close to an edge is not classified, and the two placements must agree to it. It is a
/// floating-point guard, not a physics tolerance.
const GUARD: f64 = 1e-12;

/// A fault injected into the check, for the negative controls; `None` is the check itself.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    /// The contract's rotation turned by `−m`.
    Reversed,
    /// Each pixel classified by its window point `(u, v)`, the types acting before the tilt.
    WindowPoint,
    /// The reference decode read from the unsliced chart, the hidden offset dropped.
    Unsliced,
    /// The expected draw blanks every pixel outside the square, the continuing side included.
    BlankTop,
}

/// The frozen controls and the warp's parameters, REQ-CHART-044's placeholders (R-71).
fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.15, -0.25],
        z_mass: [0.3, -0.1],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

/// Every field of a decode, each float printed exactly (`{:?}` round-trips a double): equal strings are equal bits.
fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

/// The contract's Tip as sheet (chart_decoder_contract § "Tilt (basis edit)", R-474): the window offset `(du, dv, 0)`
/// turned by `m = √(τ₁² + τ₂²)` degrees about the in-plane unit axis `k = (τ₂, −τ₁, 0)/m`, by Rodrigues' formula
/// `v cos m + (k × v) sin m + k (k·v)(1 − cos m)`, giving `(Δa, Δb, d/span)`. A wrap of the total tip by 360° is the same
/// rotation, so none is applied here.
fn rodrigues(tau1: f64, tau2: f64, du: f64, dv: f64, reversed: bool) -> [f64; 3] {
    let m = (tau1 * tau1 + tau2 * tau2).sqrt();
    let (k1, k2) = (tau2 / m, -tau1 / m);
    let angle = if reversed { -m } else { m }.to_radians();
    let (s, c) = angle.sin_cos();
    let dot = k1 * du + k2 * dv;
    // k × (du, dv, 0) with k = (k1, k2, 0): (0, 0, k1 dv − k2 du).
    let cross_x = k1 * dv - k2 * du;
    [
        du * c + k1 * dot * (1.0 - c),
        dv * c + k2 * dot * (1.0 - c),
        cross_x * s,
    ]
}

/// What the pixel's own `(a, b)` say it is: inside the square, past a bounded edge (`s` either side, `t < 0`), or past
/// the continuing `t > 1` side with `s ∈ [0, 1]`; `None` within [`GUARD`] of an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Inside,
    Bounded,
    Top,
}

fn case(a: f64, b: f64) -> Option<Case> {
    let near = |x: f64| x.abs() < GUARD || (x - 1.0).abs() < GUARD;
    if near(a) || near(b) {
        return None;
    }
    let inside = |x: f64| (0.0..=1.0).contains(&x);
    Some(if !inside(a) || b < 0.0 {
        Case::Bounded
    } else if b > 1.0 {
        Case::Top
    } else {
        Case::Inside
    })
}

/// How many pixels fell in each case, and how many past `t = 1` the hidden offset visibly moved.
#[derive(Default)]
struct Tally {
    inside: u32,
    bounded: u32,
    top: u32,
    moved: u32,
}

/// R-512 F1 on `chart` under Tip as sheet into `target` at each of `taus`, over an `n × n` grid of the window at zoom 1
/// about the centre `(½, ½)`, with `fault` injected.
fn check<C>(chart: C, target: u32, taus: &[[f64; 2]], n: u32, fault: Fault, tally: &mut Tally)
where
    C: Tiltable<Param = f64> + Copy,
{
    assert!(chart.hidden(target), "z[{target}] is not hidden");
    let held = chart.map(0.5, 0.5).z.z[target as usize];
    for &[tau1, tau2] in taus {
        let spec = TiltSpec {
            tau1,
            tau2,
            map: TiltMap::Sheet,
            x: target,
            y: target,
        };
        let view = Tilted::new(chart, Tilt::new(&spec, [0.5, 0.5], [SPAN, SPAN]))
            .expect("the target is a hidden coordinate");
        for j in 0..n {
            for i in 0..n {
                let grid = |k: u32| f64::from(k) / f64::from(n - 1);
                let (u, v) = (grid(i), grid(j));
                let [da, db, dd] =
                    rodrigues(tau1, tau2, u - 0.5, v - 0.5, fault == Fault::Reversed);
                let (a, b, d) = (0.5 + da, 0.5 + db, SPAN * dd);
                let ([va, vb], shift) = view.at(u, v);
                let shift = shift.expect("a tilt is on");
                let at = format!(
                    "z[{target}] τ = ({tau1}°, {tau2}°), pixel ({u}, {v}) at (a, b, d) = ({a}, {b}, {d})"
                );
                assert!(
                    (a - va).abs() <= GUARD
                        && (b - vb).abs() <= GUARD
                        && (d - shift.dx).abs() <= GUARD,
                    "{at}: the view places it at ({va}, {vb}, {}), not the contract's Tip as sheet",
                    shift.dx
                );
                let (ca, cb) = if fault == Fault::WindowPoint {
                    (u, v)
                } else {
                    (a, b)
                };
                let Some(kind) = case(ca, cb) else { continue };
                let valid = view.validate(u, v) == ValidationResult::Valid;
                let blank = forbidden(&view, u, v);
                // "As past the window": the untilted chart sliced to this pixel's own hidden value, read at (a, b).
                let sliced = chart
                    .sliced(target, held + shift.dx)
                    .expect("the target slices");
                let own_valid = sliced.validate(va, vb) == ValidationResult::Valid;
                assert_eq!(
                    valid, own_valid,
                    "{at}: the tilted pixel's validity is not the sliced chart's at (a, b)"
                );
                match kind {
                    Case::Bounded => {
                        tally.bounded += 1;
                        assert!(!valid, "{at}: past a bounded edge, it is a system");
                        assert_eq!(blank, Some(true), "{at}: past a bounded edge, it is drawn");
                    }
                    Case::Top | Case::Inside => {
                        if kind == Case::Top {
                            tally.top += 1;
                        } else {
                            tally.inside += 1;
                        }
                        let expect_blank = fault == Fault::BlankTop && kind == Case::Top;
                        assert!(valid, "{at}: {kind:?}, it is not a system");
                        assert_eq!(
                            blank,
                            Some(expect_blank),
                            "{at}: {kind:?}, its draw is not a system's"
                        );
                        let reference = if fault == Fault::Unsliced {
                            chart
                        } else {
                            sliced
                        };
                        let own = reference.decode::<Defaults, f64>(va, vb);
                        let tilted = view.decode::<Defaults, f64>(u, v);
                        assert_eq!(
                            bits(&tilted),
                            bits(&own),
                            "{at}: {kind:?}, the decode is not the chart's at (a, b) with its hidden value"
                        );
                        if kind == Case::Top && target == 6 && shift.dx.abs() > 1e-6 {
                            let unsliced = chart.decode::<Defaults, f64>(va, vb);
                            if bits(&unsliced) != bits(&tilted) {
                                tally.moved += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Every pixel of every tilt in `taus` on the two invariant energy charts, each as itself and warped, into each of
/// [`TARGETS`]; then each case arises, and past `t = 1` the hidden offset moved the decode somewhere.
fn run(taus: &[[f64; 2]], n: u32, fault: Fault) {
    let mut tally = Tally::default();
    for target in TARGETS {
        let lk = InvariantLK(invariant());
        check(lk, target, taus, n, fault, &mut tally);
        check(
            Mapped::<_, Defaults>::new(lk, [Mapping::Warped; 2]),
            target,
            taus,
            n,
            fault,
            &mut tally,
        );
        let le = InvariantLE(invariant());
        check(le, target, taus, n, fault, &mut tally);
        check(
            Mapped::<_, Defaults>::new(le, [Mapping::Warped; 2]),
            target,
            taus,
            n,
            fault,
            &mut tally,
        );
    }
    assert!(tally.inside > 0, "no pixel's (a, b) lies in the square");
    assert!(
        tally.bounded > 0,
        "no pixel leaves the square past a bounded edge"
    );
    assert!(tally.top > 0, "no pixel leaves the square past t = 1");
    assert!(
        tally.moved > 0,
        "past t = 1 no hidden offset moved the decode"
    );
}

/// Every both-angles-set pair of [`ANGLES`].
fn sweep() -> Vec<[f64; 2]> {
    ANGLES
        .iter()
        .flat_map(|&t1| ANGLES.iter().map(move |&t2| [t1, t2]))
        .collect()
}

/// The verify's two tilts over its 65 × 65 grid, on both charts and both targets, classified by the contract's rotation.
#[test]
fn qa_tip_sheet_named_tilts() {
    run(&NAMED, 65, Fault::None);
}

/// Every both-angles-set pair of [`ANGLES`] over a 17 × 17 grid: R-512 F1 holds for Tip as sheet, not only at two tilts.
#[test]
fn qa_tip_sheet_every_tilt() {
    run(&sweep(), 17, Fault::None);
}

mod reversed {
    use super::*;
    negative_control!(
        qa_tip_sheet_named_tilts,
        "the rotation turned the wrong way places pixels off the view's own placement, so it must fail",
        expected = "not the contract's Tip as sheet",
        run(&NAMED, 65, Fault::Reversed)
    );
    negative_control!(
        qa_tip_sheet_every_tilt,
        "over the sweep, the rotation turned the wrong way places pixels off the view's own placement, so it must fail",
        expected = "not the contract's Tip as sheet",
        run(&sweep(), 17, Fault::Reversed)
    );
}

mod window_point {
    use super::*;
    negative_control!(
        qa_tip_sheet_named_tilts,
        "classifying at the window point, the types acting before the tilt, must fail",
        expected = "Inside, it is not a system",
        run(&NAMED, 65, Fault::WindowPoint)
    );
}

mod unsliced {
    use super::*;
    negative_control!(
        qa_tip_sheet_named_tilts,
        "a reference decode that drops the pixel's hidden offset differs from the tilted decode, so it must fail",
        expected = "the decode is not the chart's at (a, b) with its hidden value",
        run(&NAMED, 65, Fault::Unsliced)
    );
}

mod blank_top {
    use super::*;
    negative_control!(
        qa_tip_sheet_named_tilts,
        "a draw that blanks the continuing t > 1 side must fail",
        expected = "Top, its draw is not a system's",
        run(&NAMED, 65, Fault::BlankTop)
    );
}
