//! The f32 band's label on the CPU (chart_reference § 2.1, "The f32 band at the floor"; R-876; REQ-CHART-169;
//! `kernel::chart::band`): an edge walk over the f32 charts R-708 covers, `HeldLz` on the `(ν, K)` and `(θ, K)` charts
//! and `HeldK` on `(θ, L_z)`, warped and linear, each against its f64 twin.
//!
//! **The twin.** A pixel is checked against the chart as authored at f64, whose narrowing to f32 the f32 chart is:
//! a refusal that rounding a chart parameter, such as `θ_max = π/4`, causes is a rounding refusal. The walk is also run
//! against the f32 chart's own parameters widened exactly, which isolates the per-pixel arithmetic, and reported.
//!
//! **The walk.** Held `L_z` (or `K`) at 0, 1e-20, 1.5e-19 (where `L_z²/2` meets the least normal f32), 1e-5, 0.3, 1,
//! 7.5 and 1e19, each with its neighbours one and two f32 floats up and down and its negative; `u` on a 33-point grid
//! with the floats next to its ends; `v` the floor row `0`, the first 32 f32 floats above it, the floats next to `1`,
//! and a 65-point grid.
//!
//! **The criterion**, written before the first run:
//! 1. every pixel the f32 chart refuses and its f64 twin accepts is labelled rounding, none unexplained, and its f64
//!    decode is a system (the twin's `validate` passes);
//! 2. no f32-accepted pixel fails its f64 check;
//! 3. at least one rounding refusal is found against the authored twin, so the label is exercised; how many lie on
//!    the floor (the row `v = 0`, refused by the construction's own check, `KAboveLzBound`), and how many remain against the widened twin, is reported, a measurement.
//!
//! **Controls**: a band refusal drawn as the edge (the label `edge` in place of `rounding`); an accepted non-system
//! (every f32 refusal read as accepted).

use kernel::chart::band::{band, Band};
use kernel::chart::construction::{HeldK, HeldLz};
use kernel::chart::energy::Mapping;
use kernel::chart::{Burrau, BurrauThetaK, BurrauThetaLz, Chart, Constraint, ValidationResult};
use validation::negative_control;
use validation::roundtrip::charts;

/// How the walk reads a pixel: as built, or as a control breaks it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Variant {
    Plain,
    #[cfg(feature = "controls")]
    EdgeDrawn,
    #[cfg(feature = "controls")]
    AcceptAll,
}

/// `x` and its neighbours `k` f32 floats up and down, `k = 1, 2`, and their negatives.
fn around(xs: &[f32]) -> Vec<f32> {
    let mut out = Vec::new();
    for &x in xs {
        let b = x.to_bits();
        for k in [0u32, 1, 2] {
            for y in [f32::from_bits(b + k), f32::from_bits(b.saturating_sub(k))] {
                out.extend([y, -y]);
            }
        }
    }
    out.sort_by(f32::total_cmp);
    out.dedup();
    out
}

/// The held values walked.
fn held() -> Vec<f32> {
    around(&[0.0, 1e-20, 1.5e-19, 1e-5, 0.3, 1.0, 7.5, 1e19])
}

/// The `u` walked.
fn us() -> Vec<f32> {
    let mut out: Vec<f32> = (0..=32).map(|k| k as f32 / 32.0).collect();
    out.extend([f32::from_bits(1), f32::from_bits(0x3f7f_ffff)]);
    out
}

/// The `v` walked: the floor row and the floats above it, the top's neighbours and a grid.
fn vs() -> Vec<f32> {
    let mut out: Vec<f32> = (0..32u32).map(f32::from_bits).collect();
    out.extend((0..4u32).map(|k| f32::from_bits(0x3f80_0000 - k)));
    out.extend((0..=64).map(|k| k as f32 / 64.0));
    out
}

/// What the walk found.
#[derive(Default, Debug)]
struct Tally {
    pixels: u64,
    rounding: u64,
    rounding_on_floor: u64,
    edge: u64,
    /// The rounding refusals by the constraint refused, the construction's `KAboveLzBound` the floor's own.
    by_constraint: std::collections::BTreeMap<String, u64>,
}

/// One pixel, its label read as `variant` reads it, checked against criteria 1 and 2.
fn pixel<N: Chart, W: Chart>(
    narrow: &N,
    wide: &W,
    u: f32,
    v: f32,
    variant: Variant,
    t: &mut Tally,
) {
    #[allow(unused_mut)]
    let mut b = band(narrow, wide, u, v);
    match variant {
        Variant::Plain => {}
        #[cfg(feature = "controls")]
        Variant::EdgeDrawn => {
            if let Band::Rounding(c) = b {
                b = Band::Edge(c);
            }
        }
        #[cfg(feature = "controls")]
        Variant::AcceptAll => b = Band::System,
    }
    let wide_ok = wide.validate(f64::from(u), f64::from(v)) == ValidationResult::Valid;
    t.pixels += 1;
    match b {
        Band::System => assert!(
            wide_ok,
            "an f32-accepted pixel is not a system at f64: {} at ({u:e}, {v:e})",
            narrow.name()
        ),
        Band::Rounding(c) => {
            assert!(
                wide_ok,
                "a rounding refusal whose f64 decode is not a system"
            );
            *t.by_constraint.entry(format!("{c:?}")).or_default() += 1;
            t.rounding += 1;
            // On the floor: the floor row, refused by the floor's own check, the construction's.
            t.rounding_on_floor += u64::from(v == 0.0 && c == Constraint::KAboveLzBound);
        }
        Band::Edge(_) | Band::Unexplained(_) => {
            assert!(
                !wide_ok,
                "an f32 refusal whose f64 twin is a system is not labelled rounding: {} at ({u:e}, {v:e}), {b:?}",
                narrow.name()
            );
            t.edge += 1;
        }
    }
}

/// Which f64 chart a pixel is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Twin {
    /// The chart as authored at f64: the band's definition.
    Authored,
    /// The f32 chart's parameters widened: the per-pixel arithmetic alone.
    Widened,
}

fn walk(variant: Variant) {
    let t = walk_against(variant, Twin::Authored);
    assert!(t.rounding > 0, "the walk found no rounding refusal: {t:?}");
    let widened = walk_against(variant, Twin::Widened);
    println!("f32_band_label: against the widened parameters, {widened:?}");
}

fn walk_against(variant: Variant, twin: Twin) -> Tally {
    let mut t = Tally::default();
    for mapping in [Mapping::Warped, Mapping::Linear] {
        for h in held() {
            // The f64 twin: the chart as authored at f64, whose narrowing the f32 chart is (`Twin::Authored`), or the
            // f32 chart's own parameters widened exactly (`Twin::Widened`), which isolates the per-pixel arithmetic.
            let (bu, tk, tl) = (
                charts::burrau::<f32>(),
                charts::burrau_theta_k::<f32>(),
                charts::burrau_theta_lz::<f32>(),
            );
            let w = |x: [f32; 2]| x.map(f64::from);
            let (bu64, tk64, tl64) = match twin {
                Twin::Authored => (
                    charts::burrau::<f64>(),
                    charts::burrau_theta_k::<f64>(),
                    charts::burrau_theta_lz::<f64>(),
                ),
                Twin::Widened => (
                    Burrau {
                        nu_range: w(bu.nu_range),
                        k_max: f64::from(bu.k_max),
                        gamma_k: f64::from(bu.gamma_k),
                    },
                    BurrauThetaK {
                        theta_range: w(tk.theta_range),
                        k_max: f64::from(tk.k_max),
                        gamma_k: f64::from(tk.gamma_k),
                    },
                    BurrauThetaLz {
                        theta_range: w(tl.theta_range),
                        k: f64::from(tl.k),
                        lz_fraction: f64::from(tl.lz_fraction),
                    },
                ),
            };
            let mut a = HeldLz::new(bu);
            let mut a64 = HeldLz::new(bu64);
            let mut b = HeldLz::new(tk);
            let mut b64 = HeldLz::new(tk64);
            (a.lz, a.mapping, b.lz, b.mapping) = (h, mapping, h, mapping);
            (a64.lz, a64.mapping) = (f64::from(h), mapping);
            (b64.lz, b64.mapping) = (f64::from(h), mapping);
            let mut k = HeldK::new(tl, 1.0f32, 2.0f32);
            (k.chart.k, k.mapping) = (h, mapping);
            let mut k64 = HeldK::new(tl64, 1.0f64, 2.0f64);
            (k64.chart.k, k64.mapping) = (f64::from(h), mapping);
            for &u in &us() {
                for &v in &vs() {
                    pixel(&a, &a64, u, v, variant, &mut t);
                    pixel(&b, &b64, u, v, variant, &mut t);
                    // `HeldK`'s rows are `L_z(u)` at the held `K`: walk it transposed too, its column edges at `u`.
                    pixel(&k, &k64, u, v, variant, &mut t);
                    pixel(&k, &k64, v, u, variant, &mut t);
                }
            }
        }
    }
    println!("f32_band_label: against the {twin:?} twin, {t:?}");
    t
}

#[test]
fn f32_band_label() {
    walk(Variant::Plain);
}

negative_control!(
    f32_band_label,
    "a band refusal drawn as the edge",
    expected = "is not labelled rounding",
    walk(Variant::EdgeDrawn)
);

#[test]
fn f32_band_label_accepted() {
    walk(Variant::Plain);
}

negative_control!(
    f32_band_label_accepted,
    "an accepted non-system: every f32 refusal read as accepted",
    expected = "an f32-accepted pixel is not a system at f64",
    walk(Variant::AcceptAll)
);
