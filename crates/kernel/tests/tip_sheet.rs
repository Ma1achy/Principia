//! Under Tip as sheet a window pixel whose own `(a, b)` leave the chart's square is past the edge (R-512, ruling
//! RQ-402, F1; REQ-CHART-076; chart_decoder_contract § "Tilt (basis edit)"; chart_reference §2.1):
//! - `tip_sheet_in_window`: on the `(L_z, K)` chart, warped, tilted into one target (a mass logit) under Tip as sheet at
//!   `τ = (30°, 12.4°)` and `(−45°, 77.9°)`, over a 65 × 65 grid of the window at zoom 1 about the centre `(½, ½)`:
//!   every pixel whose `(a, b)` lie in the square validates and is drawn; every pixel whose `(a, b)` leave it past an
//!   `s` edge or below `t = 0`, a bounded edge, is not a system and draws the background (the chart classification's
//!   `forbidden`, which a presentation view draws blank, R-470); every pixel past `t = 1` with `s ∈ [0, 1]`, the warp's
//!   continuing side (R-508 V2), is a system, drawn, and decodes bit-identically to the untilted chart sliced to that
//!   pixel's hidden value, the slice centre plus its `d`, at the same `(a, b)`. Each of the three cases past the edge
//!   arises over the two tilts. Run on the chart and on its warped [`Mapped`] form, the dispatch's.
//!
//! Its negative controls (R-176): a draw that reads the chart at the window point, the types acting before the tilt,
//! colours a pixel past a bounded edge; a window scaled to keep every `(a, b)` in the square (RQ-402's option (b))
//! takes no pixel past the edge; a draw that blanks every pixel outside the square blanks one past `t = 1`. Each fails.
//!
//! Until the chart addendum's N14 lands the built charts draw as today (R-932): past a bounded edge, blank.

use kernel::chart::invariant::Invariant;
use kernel::chart::tilt::Tiltable;
use kernel::chart::{
    Chart, InvariantLK, Mapped, Mapping, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::decode::Decoded;
use kernel::generated::links::Defaults;
use validation::chart_class::forbidden;
use validation::{negative_control, GAMMA_K, K_MAX};

/// The hidden coordinate both angles tilt into: the first mass logit, `z[6]`.
const TARGET: u32 = 6;
/// A latent slider's span (R-920): the hidden offset's unit.
const SPAN: f64 = 2.0;
/// The two tilts REQ-CHART-076 names, `(τ₁, τ₂)` in degrees.
const TAUS: [[f64; 2]; 2] = [[30.0, 12.4], [-45.0, 77.9]];
/// The grid's points per side.
const N: u32 = 65;

/// Whether a window pixel is drawn blank: `Some(true)` blank, `Some(false)` drawn, `None` unclassified.
type Draw<C> = fn(&Tilted<C, f64>, f64, f64) -> Option<bool>;

/// The chart's frozen part: the configuration and mass controls and the warp's two parameters, REQ-CHART-044's
/// placeholders (R-71).
fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.2, -0.3],
        z_mass: [0.4, -0.2],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

/// Every field of a decode, each float printed exactly (`{:?}` round-trips a double): equal strings are equal bits.
fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

/// The chart's real draw: the chart classification of the window pixel through the tilted view (R-470).
fn drawn<C: Tiltable<Param = f64>>(view: &Tilted<C, f64>, u: f64, v: f64) -> Option<bool> {
    forbidden(view, u, v)
}

/// The control's draw: the untilted chart classified at the window point, the types acting before the tilt.
#[cfg(feature = "controls")]
fn untilted_draw<C: Tiltable<Param = f64>>(view: &Tilted<C, f64>, u: f64, v: f64) -> Option<bool> {
    forbidden(view.chart(), u, v)
}

/// The control's draw: every pixel whose `(a, b)` leave the square blank, whatever its edge.
#[cfg(feature = "controls")]
fn square_draw<C: Tiltable<Param = f64>>(view: &Tilted<C, f64>, u: f64, v: f64) -> Option<bool> {
    let ([a, b], _) = view.at(u, v);
    let inside = |x: f64| (0.0..=1.0).contains(&x);
    let blank = forbidden(view, u, v)?;
    Some(blank || !(inside(a) && inside(b)))
}

/// How many pixels fell in each case: inside the square, past a bounded edge (`s` or `t = 0`), past `t = 1`.
#[derive(Default)]
struct Tally {
    inside: u32,
    bounded: u32,
    top: u32,
}

/// R-512 F1 on `chart` at each of [`TAUS`] under Tip as sheet into [`TARGET`], over the window scaled by `scale` about
/// its centre `(½, ½)` (1 the window at zoom 1), each pixel drawn by `draw`; then each case past the edge arises.
fn check<C>(chart: C, scale: f64, draw: Draw<C>)
where
    C: Tiltable<Param = f64> + Copy,
{
    let held = chart.map(0.5, 0.5).z.z[TARGET as usize];
    let mut tally = Tally::default();
    for [tau1, tau2] in TAUS {
        let spec = TiltSpec {
            tau1,
            tau2,
            map: TiltMap::Sheet,
            x: TARGET,
            y: TARGET,
        };
        let view = Tilted::new(chart, Tilt::new(&spec, [0.5, 0.5], [SPAN, SPAN]))
            .expect("the mass logit is a hidden coordinate");
        for j in 0..N {
            for i in 0..N {
                let at = |k: u32| 0.5 + scale * (f64::from(k) / f64::from(N - 1) - 0.5);
                let (u, v) = (at(i), at(j));
                let ([a, b], shift) = view.at(u, v);
                let shift = shift.expect("a tilt is on");
                let blank = draw(&view, u, v);
                let at_ab =
                    format!("({u}, {v}) at (a, b) = ({a}, {b}) under τ = ({tau1}°, {tau2}°)");
                if !(0.0..=1.0).contains(&a) || b < 0.0 {
                    tally.bounded += 1;
                    assert_ne!(
                        view.validate(u, v),
                        ValidationResult::Valid,
                        "{at_ab}, past a bounded edge, validates"
                    );
                    assert_eq!(
                        blank,
                        Some(true),
                        "{at_ab}, past a bounded edge, is drawn, not blank"
                    );
                } else if b > 1.0 {
                    tally.top += 1;
                    assert_eq!(
                        view.validate(u, v),
                        ValidationResult::Valid,
                        "{at_ab}, past t = 1, is no system"
                    );
                    assert_eq!(blank, Some(false), "{at_ab}, past t = 1, is drawn blank");
                    let sliced = chart
                        .sliced(TARGET, held + shift.dx)
                        .expect("the mass logit slices");
                    let own = sliced.decode::<Defaults, f64>(a, b);
                    let tilted = view.decode::<Defaults, f64>(u, v);
                    assert!(
                        own.degenerate.is_none(),
                        "{at_ab}, past t = 1, decodes degenerate"
                    );
                    assert_eq!(
                        bits(&tilted),
                        bits(&own),
                        "{at_ab}, past t = 1, is not the untilted chart's decode sliced to its hidden value"
                    );
                } else {
                    tally.inside += 1;
                    assert_eq!(
                        view.validate(u, v),
                        ValidationResult::Valid,
                        "{at_ab}, in the square, is no system"
                    );
                    assert_eq!(blank, Some(false), "{at_ab}, in the square, is not drawn");
                }
            }
        }
    }
    assert!(tally.inside > 0, "no pixel's (a, b) lies in the square");
    assert!(
        tally.bounded > 0,
        "no pixel leaves the square past a bounded edge"
    );
    assert!(tally.top > 0, "no pixel leaves the square past t = 1");
}

/// The `(L_z, K)` chart, warped: the chart itself and its [`Mapped`] form with both axes warped.
fn tip_sheet(
    scale: f64,
    own: Draw<InvariantLK<f64>>,
    mapped: Draw<Mapped<InvariantLK<f64>, Defaults>>,
) {
    let chart = InvariantLK(invariant());
    check(chart, scale, own);
    check(Mapped::new(chart, [Mapping::Warped; 2]), scale, mapped);
}

#[test]
fn tip_sheet_in_window() {
    tip_sheet(1.0, drawn, drawn);
}

negative_control!(
    tip_sheet_in_window,
    "a draw that reads the chart at the window point colours a pixel past a bounded edge, so it must fail",
    expected = "past a bounded edge, is drawn, not blank",
    tip_sheet(1.0, untilted_draw, untilted_draw)
);

mod scaled_window {
    use super::*;
    negative_control!(
        tip_sheet_in_window,
        "a window scaled to keep every (a, b) in the square (RQ-402's option (b)) takes no pixel past the edge, so it \
         must fail",
        expected = "no pixel leaves the square past a bounded edge",
        tip_sheet(0.5, drawn, drawn)
    );
}

mod top_blank {
    use super::*;
    negative_control!(
        tip_sheet_in_window,
        "a draw that blanks every pixel outside the square blanks one past t = 1, so it must fail",
        expected = "past t = 1, is drawn blank",
        tip_sheet(1.0, square_draw, square_draw)
    );
}
