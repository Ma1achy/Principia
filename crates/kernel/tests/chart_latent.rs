//! The `Chart` trait and the latent chart (`kernel::chart`; chart_reference §1.1, §5.1, §5.2), each check at the f64
//! and the f32 instantiation of `map` natively (R-118, R-265): a chart held at f64 mapped at f64, as on the CPU, and
//! one held at f32 mapped at f32, as in the kernel.
//! - `latent_affine_centre_and_corners`: `z(½, ½) = z₀` and the corners are `z₀ ± q₁ ± q₂`, each component exactly,
//!   over fixed and fuzzed `z₀, q₁, q₂`. The chart is built by a struct literal of `z0`, `q1` and `q2` alone, which
//!   compiles only while those are its only fields: it has no `s_u`, `s_v` (REQ-CHART-003, R-83).
//! - `latent_affine_zoom_common_factor`: zoom scales `q₁` and `q₂` by one common factor and keeps `z₀`
//!   (REQ-CHART-003, R-83).
//! - `latent_axis_aligned_equals_alpha_beta`: the slice `q₁ = ê_α`, `q₂ = ê_β` through the shared decoder equals,
//!   pixel by pixel and bitwise, the direct `(α, β)` sweep of chart_reference §5.2, its ICs built from `(α, β)`
//!   through §0.2, under every registered link (REQ-CHART-030, REQ-CHART-050).
//! - `latent_validate_accepts_every_point`: the latent chart's `validate` is `Valid` inside `[0,1]²` and past it
//!   (R-26; inverse_encode_contract § "Chart-aware validation": layer 2 adds nothing for it).
//! - `latent_name_in_dump_header`: the dump header carries the chart's name (REQ-CHART-028).
//! - `latent_allows_energy_normalisation`: the latent chart does not forbid energy normalisation (chart_reference
//!   §0.6).

mod decode_support;

use decode_support::*;
use kernel::chart::{self, Chart, ChartOut, Latent, ValidationResult};
use kernel::decode::{config, mass, momentum, Decoded, Ic};
use kernel::generated::links::*;
use validation::{negative_control, prop};

/// A chart's `map` at `R`, behind a function pointer, so a control runs the same check on a broken one.
type Map<R> = fn(&Latent<R>, R, R) -> ChartOut<R>;

/// The latent chart's own `map`.
fn map<R: P>(chart: &Latent<R>, u: R, v: R) -> ChartOut<R> {
    chart.map(u, v)
}

/// The chart `(z0, q1, q2)` at `R`, by a struct literal of exactly those fields.
fn latent_chart<R: P>(z0: [f64; 8], q1: [f64; 8], q2: [f64; 8]) -> Latent<R> {
    Latent {
        z0: at(z0),
        q1: at(q1),
        q2: at(q2),
    }
}

/// Three latent vectors `(z₀, q₁, q₂)` over the whole domain, each control at most 10³⁸ in magnitude so that
/// `z₀ ± q₁ ± q₂` stays finite at f32.
fn any_chart() -> impl proptest::strategy::Strategy<Value = ([f64; 8], [f64; 8], [f64; 8])> {
    (any_z(), any_z(), any_z())
}

/// Runs `check` on fixed charts (each fixed `z` as `z₀`, `q₁` and `q₂` in turn, the others the next fixed `z`) and on
/// fuzzed ones.
fn over_charts(check: impl Fn([f64; 8], [f64; 8], [f64; 8])) {
    let fixed = fixed_z();
    for (i, z) in fixed.iter().enumerate() {
        let w = fixed[(i + 1) % fixed.len()];
        check(*z, w, w);
        check(w, *z, w);
        check(w, w, *z);
    }
    prop::run(&any_chart(), |(z0, q1, q2)| {
        check(z0, q1, q2);
        Ok(())
    });
}

// ── latent_affine_centre_and_corners (REQ-CHART-003) ──────────────────────────────────────────────────────────

/// `map(½, ½) = z₀` and each corner `(u, v) ∈ {0, 1}²` is `z₀ + (2u − 1)q₁ + (2v − 1)q₂`, componentwise and exactly.
fn check_corners<R: P>(map: Map<R>, z0: [f64; 8], q1: [f64; 8], q2: [f64; 8]) {
    let chart = latent_chart::<R>(z0, q1, q2);
    let half = R::of(0.5);
    assert_eq!(
        map(&chart, half, half).z.z,
        chart.z0,
        "z(½, ½) is not z₀ for {chart:?}"
    );
    for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let got = map(&chart, R::of(u), R::of(v)).z.z;
        let want: [R; 8] = core::array::from_fn(|k| {
            let a = if u == 1.0 { chart.q1[k] } else { -chart.q1[k] };
            let b = if v == 1.0 { chart.q2[k] } else { -chart.q2[k] };
            chart.z0[k] + a + b
        });
        assert_eq!(
            got, want,
            "the corner ({u}, {v}) is not z₀ ± q₁ ± q₂ for {chart:?}"
        );
    }
}

fn check_corners_both(m64: Map<f64>, m32: Map<f32>) {
    over_charts(|z0, q1, q2| {
        check_corners::<f64>(m64, z0, q1, q2);
        check_corners::<f32>(m32, z0, q1, q2);
    });
}

#[test]
fn latent_affine_centre_and_corners() {
    check_corners_both(map::<f64>, map::<f32>);
}

/// A map that places `(u − ½)` rather than `(2u − 1)`: the centre holds, the corners don't.
#[cfg(feature = "controls")]
fn half_scale<R: P>(chart: &Latent<R>, u: R, v: R) -> ChartOut<R> {
    let half = R::of(0.5);
    chart.map(half + (u - half) * half, half + (v - half) * half)
}

negative_control!(
    latent_affine_centre_and_corners,
    "a map placing (u − ½) for (2u − 1) misses the corners",
    expected = "is not z₀ ± q₁ ± q₂",
    check_corners_both(half_scale::<f64>, half_scale::<f32>)
);

// ── latent_affine_zoom_common_factor (REQ-CHART-003) ──────────────────────────────────────────────────────────

/// A zoom at `R`, behind a function pointer.
type Zoom<R> = fn(&Latent<R>, R) -> Latent<R>;

fn zoom<R: P>(chart: &Latent<R>, k: R) -> Latent<R> {
    chart.zoomed(k)
}

/// The zoom factors: in and out, powers of two and not.
const ZOOMS: [f64; 5] = [0.5, 0.0009765625, 0.7, 2.0, 3.0];

/// Zoom by `k` keeps `z₀` and scales `q₁` and `q₂` each by `k`, componentwise.
fn check_zoom<R: P>(zoom: Zoom<R>, z0: [f64; 8], q1: [f64; 8], q2: [f64; 8]) {
    let chart = latent_chart::<R>(z0, q1, q2);
    for k in ZOOMS.map(R::of) {
        let zoomed = zoom(&chart, k);
        assert_eq!(zoomed.z0, chart.z0, "zoom by {k:?} moved z₀ of {chart:?}");
        assert_eq!(
            zoomed.q1,
            chart.q1.map(|q| q * k),
            "zoom by {k:?}: q₁ is not scaled by the common factor, {chart:?}"
        );
        assert_eq!(
            zoomed.q2,
            chart.q2.map(|q| q * k),
            "zoom by {k:?}: q₂ is not scaled by the common factor, {chart:?}"
        );
    }
}

fn check_zoom_both(z64: Zoom<f64>, z32: Zoom<f32>) {
    over_charts(|z0, q1, q2| {
        check_zoom::<f64>(z64, z0, q1, q2);
        check_zoom::<f32>(z32, z0, q1, q2);
    });
}

#[test]
fn latent_affine_zoom_common_factor() {
    check_zoom_both(zoom::<f64>, zoom::<f32>);
}

/// A per-axis zoom: `q₁` by `k`, `q₂` by `k²`.
#[cfg(feature = "controls")]
fn per_axis<R: P>(chart: &Latent<R>, k: R) -> Latent<R> {
    let mut z = chart.zoomed(k);
    z.q2 = z.q2.map(|q| q * k);
    z
}

negative_control!(
    latent_affine_zoom_common_factor,
    "a zoom scaling q₂ by another factor than q₁ must fail",
    expected = "q₂ is not scaled by the common factor",
    check_zoom_both(per_axis::<f64>, per_axis::<f32>)
);

// ── latent_axis_aligned_equals_alpha_beta (REQ-CHART-030, REQ-CHART-050) ──────────────────────────────────────

/// `ê_k`.
fn basis(k: usize) -> [f64; 8] {
    let mut e = [0.0; 8];
    e[k] = 1.0;
    e
}

/// The pixel coordinates: a 17 × 17 grid over `[0,1]²` and a row and column past each edge.
fn pixels() -> Vec<f64> {
    let mut c: Vec<f64> = (0..=16).map(|i| f64::from(i) / 16.0).collect();
    c.extend([-0.25, 1.25]);
    c
}

/// chart_reference §5.2's direct `(α, β)` sweep at `(u, v)` about `z₀`: `α` and `β` through the selection's links from
/// `z₀_α + (2u − 1)` and `z₀_β + (2v − 1)`, the IC built from them through §0.2 (`ρ̃`, `λ̃`, unweighted and
/// reconstructed), the masses and momenta from `z₀`'s mass and momentum blocks.
fn direct<S: Selection, R: P>(z0: [R; 8], u: R, v: R) -> (R, R, Ic<R>) {
    let one = R::of(1.0);
    let (alpha, beta) = config::angles::<S, R>([z0[0] + (u + u - one), z0[1] + (v + v - one)]);
    let masses = mass::decode::<S, R>([z0[6], z0[7]]);
    let (rho_t, lambda_t) = config::hyperspherical(alpha, beta);
    let (_, _, r) = config::reconstruct(&masses, rho_t, lambda_t);
    let p = momentum::decode::<S, R>([z0[2], z0[3], z0[4], z0[5]], &masses).p;
    (alpha, beta, Ic { m: masses.m, r, p })
}

/// The axis-aligned chart's decode, `D ∘ Φ`, at `R`, behind a function pointer.
type ChartDecode<R> = fn(&Latent<R>, R, R) -> Decoded<R>;

/// Every pixel of the slice `(z₀, q₁, q₂)` decodes, under `S`, bitwise to the direct sweep's `(α, β)` and IC.
fn check_axis_aligned<S: Selection, R: P>(
    decode: ChartDecode<R>,
    link: &str,
    z0: [f64; 8],
    q1: [f64; 8],
    q2: [f64; 8],
) {
    let chart = latent_chart::<R>(z0, q1, q2);
    for &u in &pixels() {
        for &v in &pixels() {
            let (u, v) = (R::of(u), R::of(v));
            let got = decode(&chart, u, v);
            let (alpha, beta, ic) = direct::<S, R>(chart.z0, u, v);
            assert!(
                (got.config.alpha, got.config.beta, got.ic) == (alpha, beta, ic),
                "pixel ({u:?}, {v:?}) under {link}: the slice's IC differs from the direct (α, β) sweep's, \
                 z₀ = {z0:?}: {:?} against {:?}",
                (got.config.alpha, got.config.beta, got.ic),
                (alpha, beta, ic)
            );
        }
    }
}

/// The check at both precisions under every registered link, the slice given by its `(q₁, q₂)`.
fn check_axis_aligned_all(q1: [f64; 8], q2: [f64; 8]) {
    fn each<S: Selection>(link: &str, q1: [f64; 8], q2: [f64; 8]) {
        over_z(|z0| {
            check_axis_aligned::<S, f64>(chart::decode::<S, f64, Latent<f64>>, link, z0, q1, q2);
            check_axis_aligned::<S, f32>(chart::decode::<S, f32, Latent<f32>>, link, z0, q1, q2);
        });
    }
    each::<Defaults>("defaults", q1, q2);
    each::<Stick>("stick_breaking", q1, q2);
    each::<Tanhs>("tanh", q1, q2);
    each::<Softsigns>("softsign", q1, q2);
}

/// Runs `check` on the fixed `z₀` and on fuzzed ones.
fn over_z(check: impl Fn([f64; 8])) {
    over(any_z(), check);
}

#[test]
fn latent_axis_aligned_equals_alpha_beta() {
    check_axis_aligned_all(basis(0), basis(1));
}

negative_control!(
    latent_axis_aligned_equals_alpha_beta,
    "the slice with its axes swapped, q₁ = ê_β and q₂ = ê_α, is not the direct sweep",
    expected = "the slice's IC differs from the direct (α, β) sweep's",
    check_axis_aligned_all(basis(1), basis(0))
);

// ── latent_validate_accepts_every_point (R-26) ────────────────────────────────────────────────────────────────

/// A chart's `validate`, behind a function pointer.
type Validate = fn(&Latent<f64>, f64, f64) -> ValidationResult;

fn validate(chart: &Latent<f64>, u: f64, v: f64) -> ValidationResult {
    chart.validate(u, v)
}

/// The latent chart's `validate` is `Valid` at every pixel, inside `[0,1]²` and past every edge.
fn check_validate(validate: Validate) {
    over_charts(|z0, q1, q2| {
        let chart = latent_chart::<f64>(z0, q1, q2);
        for &u in &pixels() {
            for &v in &pixels() {
                assert_eq!(
                    validate(&chart, u, v),
                    ValidationResult::Valid,
                    "the latent chart refused ({u}, {v}), though layer 2 adds no constraint to it"
                );
            }
        }
    });
}

#[test]
fn latent_validate_accepts_every_point() {
    check_validate(validate);
}

/// A `validate` that applies the hypercube to the chart point, layer 1, which is the encode path's.
#[cfg(feature = "controls")]
fn unit_square(chart: &Latent<f64>, u: f64, v: f64) -> ValidationResult {
    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
        chart.validate(u, v)
    } else {
        ValidationResult::Reject {
            failed: chart::Constraint::MassPositive,
        }
    }
}

negative_control!(
    latent_validate_accepts_every_point,
    "a validate that rejects past [0,1]² must fail",
    expected = "though layer 2 adds no constraint to it",
    check_validate(unit_square)
);

// ── latent_name_in_dump_header (REQ-CHART-028) ────────────────────────────────────────────────────────────────

/// A dump-header writer, behind a function pointer.
type Header = fn(&mut String, &Latent<f64>) -> core::fmt::Result;

fn header(out: &mut String, chart: &Latent<f64>) -> core::fmt::Result {
    chart::write_dump_header(out, chart)
}

/// The header is the line `chart: Latent`.
fn check_header(header: Header) {
    let chart = latent_chart::<f64>([0.0; 8], basis(0), basis(1));
    let mut out = String::new();
    header(&mut out, &chart).expect("writing to a String does not fail");
    assert_eq!(chart.name(), "Latent", "the latent chart's name");
    assert_eq!(
        out, "chart: Latent\n",
        "the dump header does not stamp the chart's name"
    );
}

#[test]
fn latent_name_in_dump_header() {
    check_header(header);
}

/// A header writer that leaves the name out.
#[cfg(feature = "controls")]
fn nameless(out: &mut String, _chart: &Latent<f64>) -> core::fmt::Result {
    use core::fmt::Write;
    writeln!(out, "chart:")
}

negative_control!(
    latent_name_in_dump_header,
    "a header without the chart's name must fail",
    expected = "the dump header does not stamp the chart's name",
    check_header(nameless)
);

// ── latent_allows_energy_normalisation (chart_reference §0.6) ────────────────────────────────────────────────

/// The latent chart, whose energy is no chart coordinate, does not forbid energy normalisation.
fn check_energy(forbids: bool) {
    assert!(
        !forbids,
        "the latent chart forbids energy normalisation, though energy is not one of its coordinates"
    );
}

#[test]
fn latent_allows_energy_normalisation() {
    let chart = latent_chart::<f64>([0.0; 8], basis(0), basis(1));
    check_energy(chart.forbids_energy_normalisation());
}

negative_control!(
    latent_allows_energy_normalisation,
    "a chart that forbids energy normalisation must fail",
    expected = "the latent chart forbids energy normalisation",
    check_energy(true)
);
