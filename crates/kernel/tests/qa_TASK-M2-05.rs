//! QA tests for TASK-M2-05 (the `Chart` trait, `ValidationResult` and the latent affine chart), written from the
//! requirements and chart_reference §1.1, §5.1, §5.2 and §5.4, chart_decoder_contract Part 3, inverse_encode_contract
//! § "Chart-aware validation" and R-26, R-83, R-118, not from the implementation. Each check runs behind a function
//! pointer or a generic, so its negative control (R-176) runs the same check on a deliberately broken variant.
//!
//! - REQ-CHART-003, R-83: on dyadic `z₀, q₁, q₂` and dyadic `(u, v)`, where every product and sum is exact at f32 and
//!   f64, `Φ(u, v) = z₀ + (2u − 1)q₁ + (2v − 1)q₂` componentwise and exactly, over `[0,1]²` and past every edge
//!   (§5.4: the latent axes are affine, the formula continues), at f64 parameters mapped at f64, f32 at f32 and f32
//!   widened to f64; zoom by `k` keeps `z₀`, scales `q₁` and `q₂` by the one `k`, and so maps every pixel to
//!   `z₀ + k·((2u − 1)q₁ + (2v − 1)q₂)`; the chart's storage is exactly `z₀, q₁, q₂` (no `s_u`, `s_v`).
//! - REQ-CHART-030, REQ-CHART-050: the slice `q₁ = ê_α, q₂ = ê_β` is a direct `(α, β)` sweep: at every pixel the
//!   chart's decode equals, bitwise, the shared decoder applied to `z₀` with `z_α = z₀_α + (2u − 1)` and
//!   `z_β = z₀_β + (2v − 1)`, and its `α`, `β` are the selection's α and β links of those two controls; and, as a sweep
//!   of `(α, β)` alone, the masses and momenta are `z₀`'s at every pixel, `α` depends on `u` only and rises with it,
//!   `β` on `v` only and rises with it, and the centre pixel is `D(z₀)`. Under every registered link family.
//! - REQ-CHART-028: a probe chart written here implements the trait's four methods; a generic over `Chart` reaches all
//!   four; `map` from the one source runs at f32 and f64; `validate` takes f64; the dump header stamps the chart's own
//!   `name()` (the probe's, not a constant); `forbids_energy_normalisation` defaults to false and an override is read.
//! - REQ-CHART-015: `chart::decode` is `D ∘ Φ` through the shared decoder, for a chart that is not the latent one.
//! - REQ-CHART-043, R-26: the four `ValidationResult` variants, each failure naming its failed constraint, round-trip
//!   through a chart's `validate`; the latent chart's `validate` is `Valid` at every pixel inside and past `[0,1]²`,
//!   and its decoded masses there are positive (layer 3's mass check holds where `Valid` is returned).
//!
//! **Tolerances.** None: every assertion is exact (`==`), on values chosen so the arithmetic is exact, or on bitwise
//! equality the definition (chart_reference §5.2, REQ-CHART-050) states.

use kernel::chart::{self, Chart, ChartOut, Constraint, Latent, ValidationResult};
use kernel::decode::{self, Decoded};
use kernel::generated::links::*;
use kernel::payload::PayloadReal;
use spirv_std::num_traits::Float;
use validation::negative_control;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

trait Fp:
    LinkReal + PayloadReal + Float + core::fmt::Debug + Copy + PartialEq + From<Self> + 'static
{
    fn of(x: f64) -> Self;
}

impl Fp for f64 {
    fn of(x: f64) -> Self {
        x
    }
}

impl Fp for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
}

// Every link family, as in the registry (the default mass link with each angle/momentum family, and the stick-breaking
// mass link).
type Sigmoids = Defaults;
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type Tanhs = Select<SoftmaxTanh, TanhAlpha, TanhBeta, TanhQ, TanhQ, TanhQ, TanhQ>;
type Softsigns =
    Select<StickBreaking, SoftsignAlpha, SoftsignBeta, SoftsignQ, SoftsignQ, SoftsignQ, SoftsignQ>;

// ── Fixtures: dyadic charts, so the affine formula is exact at f32 ──────────────────────────────────────────────

/// Dyadic `(z₀, q₁, q₂)`: small multiples of 1/8, so every `(2u − 1)·q` with `u` a multiple of 1/16 and every sum below
/// is exact at f32 and f64.
const CHARTS: [([f64; 8], [f64; 8], [f64; 8]); 3] = [
    (
        [0.0; 8],
        [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        [0.5, -1.25, 0.375, 2.0, -0.75, 0.125, 1.5, -2.5],
        [0.25, -0.5, 1.0, 0.0, -1.75, 0.625, 0.0, 3.0],
        [-1.0, 0.75, 0.0, -0.125, 0.5, 2.25, -0.375, 0.0],
    ),
    (
        [-3.0, 0.0, 1.0, -1.0, 0.25, 0.5, -0.5, 0.75],
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
    ),
];

/// Pixel coordinates: multiples of 1/16 over `[−½, 3/2]`, so inside `[0,1]²` and past every edge.
fn coords() -> Vec<f64> {
    (-8..=24).map(|i| f64::from(i) / 16.0).collect()
}

fn chart_at<R: Fp>(z0: [f64; 8], q1: [f64; 8], q2: [f64; 8]) -> Latent<R> {
    Latent {
        z0: z0.map(R::of),
        q1: q1.map(R::of),
        q2: q2.map(R::of),
    }
}

// ── REQ-CHART-003: the affine slice ─────────────────────────────────────────────────────────────────────────────

/// `Φ` of a latent chart held at `P`, evaluated at `F`, behind a function pointer.
type MapFn<P, F> = fn(&Latent<P>, F, F) -> [F; 8];

fn real_map<P: Copy, F: Float + From<P>>(c: &Latent<P>, u: F, v: F) -> [F; 8] {
    c.map(u, v).z.z
}

/// `Φ(u, v) = z₀ + (2u − 1)q₁ + (2v − 1)q₂` exactly, the right side evaluated here in f64 (exact on the dyadic grid).
fn check_affine<P: Fp, F: Fp + From<P>>(map: MapFn<P, F>) {
    for (z0, q1, q2) in CHARTS {
        let c = chart_at::<P>(z0, q1, q2);
        for &u in &coords() {
            for &v in &coords() {
                let got = map(&c, F::of(u), F::of(v));
                let want: [F; 8] = core::array::from_fn(|k| {
                    F::of(z0[k] + (2.0 * u - 1.0) * q1[k] + (2.0 * v - 1.0) * q2[k])
                });
                assert!(
                    got == want,
                    "Φ({u}, {v}) is not z₀ + (2u − 1)q₁ + (2v − 1)q₂: {got:?} against {want:?}"
                );
            }
        }
    }
}

fn check_affine_all(m64: MapFn<f64, f64>, m32: MapFn<f32, f32>, m32_64: MapFn<f32, f64>) {
    check_affine::<f64, f64>(m64);
    check_affine::<f32, f32>(m32);
    check_affine::<f32, f64>(m32_64);
}

#[test]
fn qa_latent_map_is_the_affine_slice() {
    check_affine_all(real_map, real_map, real_map);
}

/// A map with a per-axis factor on the horizontal axis (`s_u = 2`), which R-83 removed.
#[cfg(feature = "controls")]
fn per_axis_map<P: Copy, F: Float + From<P>>(c: &Latent<P>, u: F, v: F) -> [F; 8] {
    let half = F::one() / (F::one() + F::one());
    // (2u' − 1) = 2(2u − 1) for u' = 2u − ½.
    let u2 = u + u - half;
    c.map(u2, v).z.z
}

negative_control!(
    qa_latent_map_is_the_affine_slice,
    "a map with a per-axis factor s_u = 2 is not the affine slice",
    expected = "is not z₀ + (2u − 1)q₁ + (2v − 1)q₂",
    check_affine_all(per_axis_map, per_axis_map, per_axis_map)
);

/// A zoom, behind a function pointer.
type ZoomFn<P> = fn(&Latent<P>, P) -> Latent<P>;

fn real_zoom<P: Fp>(c: &Latent<P>, k: P) -> Latent<P> {
    c.zoomed(k)
}

/// Zoom by `k` keeps `z₀`, scales `q₁` and `q₂` by the one `k`, and the zoomed chart maps every pixel to
/// `z₀ + k·((2u − 1)q₁ + (2v − 1)q₂)`: the centre stays, and the view shrinks or grows about it uniformly.
fn check_zoom<P: Fp>(zoom: ZoomFn<P>) {
    for (z0, q1, q2) in CHARTS {
        let c = chart_at::<P>(z0, q1, q2);
        for k in [1.0, 0.5, 2.0, 0.25, 4.0] {
            let z = zoom(&c, P::of(k));
            assert!(z.z0 == c.z0, "zoom by {k} moved z₀");
            let s1: [P; 8] = q1.map(|q| P::of(k * q));
            let s2: [P; 8] = q2.map(|q| P::of(k * q));
            assert!(
                z.q1 == s1 && z.q2 == s2,
                "zoom by {k} does not scale q₁ and q₂ by one common factor: {:?}, {:?}",
                z.q1,
                z.q2
            );
            for &u in &coords() {
                for &v in &coords() {
                    let got = z.map(P::of(u), P::of(v)).z.z;
                    let want: [P; 8] = core::array::from_fn(|i| {
                        P::of(z0[i] + k * ((2.0 * u - 1.0) * q1[i] + (2.0 * v - 1.0) * q2[i]))
                    });
                    assert!(
                        got == want,
                        "zoom by {k} does not scale q₁ and q₂ by one common factor at ({u}, {v})"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_latent_zoom_is_common_scale_on_q() {
    check_zoom::<f64>(real_zoom);
    check_zoom::<f32>(real_zoom);
}

/// A zoom that scales only `q₁`.
#[cfg(feature = "controls")]
fn q1_only<P: Fp>(c: &Latent<P>, k: P) -> Latent<P> {
    let mut z = *c;
    z.q1 = c.q1.map(|q| q * k);
    z
}

negative_control!(
    qa_latent_zoom_is_common_scale_on_q,
    "a zoom scaling q₁ alone is not a common scale",
    expected = "does not scale q₁ and q₂ by one common factor",
    {
        check_zoom::<f64>(q1_only);
        check_zoom::<f32>(q1_only);
    }
);

/// The chart's storage is exactly three latent vectors, so it has no `s_u`, `s_v` (R-83). The struct literal in
/// [`chart_at`] names exactly `z0`, `q1`, `q2`; the size check also fails if a field is added beside them.
fn check_storage(sizes: &[(usize, usize)]) {
    for &(got, want) in sizes {
        assert_eq!(got, want, "the latent chart holds more than z₀, q₁, q₂");
    }
}

#[test]
fn qa_latent_has_no_per_axis_factors() {
    check_storage(&[
        (core::mem::size_of::<Latent<f64>>(), 3 * 8 * 8),
        (core::mem::size_of::<Latent<f32>>(), 3 * 8 * 4),
    ]);
}

/// A chart with `s_u`, `s_v` beside `z₀, q₁, q₂`.
#[cfg(feature = "controls")]
#[allow(dead_code)] // The control reads only its size.
struct WithScales {
    z0: [f64; 8],
    q1: [f64; 8],
    q2: [f64; 8],
    s_u: f64,
    s_v: f64,
}

negative_control!(
    qa_latent_has_no_per_axis_factors,
    "a chart with s_u, s_v fields is caught",
    expected = "the latent chart holds more than z₀, q₁, q₂",
    check_storage(&[(core::mem::size_of::<WithScales>(), 3 * 8 * 8)])
);

// ── REQ-CHART-030, REQ-CHART-050: the axis-aligned slice is the direct (α, β) sweep ─────────────────────────────

fn e(k: usize) -> [f64; 8] {
    let mut v = [0.0; 8];
    v[k] = 1.0;
    v
}

/// Slice centres `z₀`, each well inside every link's unsaturated range so `α` and `β` rise strictly pixel to pixel.
const CENTRES: [[f64; 8]; 3] = [
    [0.0; 8],
    [0.3, -0.7, 0.5, -1.1, 0.9, 0.2, 0.4, -0.6],
    [-1.2, 1.4, -0.3, 0.8, -0.5, 1.6, -1.0, 0.7],
];

/// A chart's `D ∘ Φ` under `S`, behind a function pointer.
type ChartDecode<R> = fn(&Latent<R>, R, R) -> Decoded<R>;

/// The direct sweep's `z` at `(u, v)`: `z₀` with `z_α = z₀_α + (2u − 1)` and `z_β = z₀_β + (2v − 1)` (§5.2's
/// definition), evaluated here.
fn direct_z<R: Fp>(z0: [R; 8], u: f64, v: f64) -> [R; 8] {
    let mut z = z0;
    z[0] = z0[0] + R::of(2.0 * u - 1.0);
    z[1] = z0[1] + R::of(2.0 * v - 1.0);
    z
}

fn check_sweep<S: Selection, R: Fp>(
    decode_chart: ChartDecode<R>,
    link: &str,
    q1: [f64; 8],
    q2: [f64; 8],
) {
    let cs = coords();
    for z0f in CENTRES {
        let c = chart_at::<R>(z0f, q1, q2);
        let z0 = c.z0;
        let centre = decode::decode::<S, R>(&decode::Latent::new(z0));
        let mut alpha_of_u: Vec<Option<R>> = vec![None; cs.len()];
        let mut beta_of_v: Vec<Option<R>> = vec![None; cs.len()];
        for (i, &u) in cs.iter().enumerate() {
            for (j, &v) in cs.iter().enumerate() {
                let got = decode_chart(&c, R::of(u), R::of(v));
                // Bitwise against the shared decoder on the direct sweep's z.
                let want = decode::decode::<S, R>(&decode::Latent::new(direct_z(z0, u, v)));
                assert!(
                    (got.config.alpha, got.config.beta, got.ic) == (want.config.alpha, want.config.beta, want.ic),
                    "under {link}, pixel ({u}, {v}): the slice is not the direct (α, β) sweep, z₀ = {z0f:?}"
                );
                // α and β are the selection's links of the two controls.
                let za = direct_z(z0, u, v);
                let a = <S::AlphaLink as Link<1, 1>>::forward::<R>([za[0]])[0];
                let b = <S::BetaLink as Link<1, 1>>::forward::<R>([za[1]])[0];
                assert!(
                    got.config.alpha == a && got.config.beta == b,
                    "under {link}, pixel ({u}, {v}): the slice is not the direct (α, β) sweep (links)"
                );
                // A sweep of (α, β) alone: masses and momenta are z₀'s at every pixel.
                assert!(
                    got.ic.m == centre.ic.m && got.ic.p == centre.ic.p,
                    "under {link}, pixel ({u}, {v}): the slice is not the direct (α, β) sweep: mass or momentum moved"
                );
                // α depends on u alone, β on v alone.
                match alpha_of_u[i] {
                    None => alpha_of_u[i] = Some(got.config.alpha),
                    Some(a0) => assert!(
                        a0 == got.config.alpha,
                        "under {link}: the slice is not the direct (α, β) sweep: α moves with v at u = {u}"
                    ),
                }
                match beta_of_v[j] {
                    None => beta_of_v[j] = Some(got.config.beta),
                    Some(b0) => assert!(
                        b0 == got.config.beta,
                        "under {link}: the slice is not the direct (α, β) sweep: β moves with u at v = {v}"
                    ),
                }
                if u == 0.5 && v == 0.5 {
                    assert!(
                        got.ic == centre.ic,
                        "under {link}: the slice is not the direct (α, β) sweep: the centre pixel is not D(z₀)"
                    );
                }
            }
        }
        // α rises with u (q₁ = +ê_α), β with v (q₂ = +ê_β).
        for w in alpha_of_u.windows(2) {
            assert!(
                w[0].unwrap() < w[1].unwrap(),
                "under {link}: the slice is not the direct (α, β) sweep: α does not rise with u"
            );
        }
        for w in beta_of_v.windows(2) {
            assert!(
                w[0].unwrap() < w[1].unwrap(),
                "under {link}: the slice is not the direct (α, β) sweep: β does not rise with v"
            );
        }
    }
}

fn check_sweep_all(q1: [f64; 8], q2: [f64; 8]) {
    fn each<S: Selection>(link: &str, q1: [f64; 8], q2: [f64; 8]) {
        check_sweep::<S, f64>(chart::decode::<S, f64, Latent<f64>>, link, q1, q2);
        check_sweep::<S, f32>(chart::decode::<S, f32, Latent<f32>>, link, q1, q2);
    }
    each::<Sigmoids>("sigmoid", q1, q2);
    each::<Stick>("stick_breaking", q1, q2);
    each::<Tanhs>("tanh", q1, q2);
    each::<Softsigns>("softsign", q1, q2);
}

#[test]
fn qa_latent_axis_aligned_is_direct_alpha_beta_sweep() {
    check_sweep_all(e(0), e(1));
}

negative_control!(
    qa_latent_axis_aligned_is_direct_alpha_beta_sweep,
    "the slice q₁ = ê_α, q₂ = ê_μ1 is not the (α, β) sweep",
    expected = "the slice is not the direct (α, β) sweep",
    check_sweep_all(e(0), e(6))
);

/// A second control: the slice zoomed out by 2 (`q₁ = 2ê_α`, `q₂ = 2ê_β`) is not the sweep the definition names (its
/// `(u, v) → (α, β)` map differs), though it covers the same axes.
mod zoomed_sweep {
    use super::*;
    negative_control!(
        qa_latent_axis_aligned_is_direct_alpha_beta_sweep,
        "the slice q₁ = 2ê_α, q₂ = 2ê_β is not the direct sweep",
        expected = "the slice is not the direct (α, β) sweep",
        check_sweep_all(e(0).map(|x| 2.0 * x), e(1).map(|x| 2.0 * x))
    );
}

// ── REQ-CHART-028, REQ-CHART-015, REQ-CHART-043: the trait, through a chart that is not the latent one ─────────

/// A probe chart: a fixed `z` (any `(u, v)` maps to it), its own name, forbids energy normalisation, and a `validate`
/// that returns each `ValidationResult` variant on its own region.
struct Probe<P> {
    z: [P; 8],
}

const PROBE_NAME: &str = "QaProbe-7f3a";

impl<P: Copy> Chart for Probe<P> {
    type Param = P;

    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut {
            z: decode::Latent::new(self.z.map(<F as From<P>>::from)),
        }
    }

    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        PROBE_NAME
    }

    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        if u < 0.0 {
            ValidationResult::Reject {
                failed: Constraint::BurrauNu,
            }
        } else if u > 1.0 {
            ValidationResult::Project {
                u: 1.0,
                v,
                failed: Constraint::LzEParabola,
            }
        } else if v > 1.0 {
            ValidationResult::Clamp {
                u,
                v: 1.0,
                moved: [false, true],
                failed: Constraint::ShapePhi,
            }
        } else {
            ValidationResult::Valid
        }
    }
}

/// What a chart must report through the trait.
struct Expect {
    name: &'static str,
    forbids: bool,
    /// `(u, v)` and the expected `validate`.
    validations: Vec<((f64, f64), ValidationResult)>,
}

fn probe_expect() -> Expect {
    Expect {
        name: PROBE_NAME,
        forbids: true,
        validations: vec![
            ((0.5, 0.5), ValidationResult::Valid),
            (
                (-0.1, 0.5),
                ValidationResult::Reject {
                    failed: Constraint::BurrauNu,
                },
            ),
            (
                (1.5, 0.25),
                ValidationResult::Project {
                    u: 1.0,
                    v: 0.25,
                    failed: Constraint::LzEParabola,
                },
            ),
            (
                (0.5, 1.5),
                ValidationResult::Clamp {
                    u: 0.5,
                    v: 1.0,
                    moved: [false, true],
                    failed: Constraint::ShapePhi,
                },
            ),
        ],
    }
}

/// The failed constraint a result names, or none for `Valid`.
fn failed(r: &ValidationResult) -> Option<Constraint> {
    match *r {
        ValidationResult::Valid => None,
        ValidationResult::Project { failed, .. }
        | ValidationResult::Clamp { failed, .. }
        | ValidationResult::Reject { failed } => Some(failed),
    }
}

/// A dump-header writer, behind a function pointer.
type HeaderFn<C> = fn(&mut String, &C) -> core::fmt::Result;

/// The four methods through a generic bound: `map` (at f64 for a chart held at f64, and from the same source at f32
/// for one held at f32, [`map_both`]), `name` stamped in the dump header, `forbids_energy_normalisation`, and
/// `validate` at f64 naming the failed constraint.
fn check_trait<C: Chart<Param = f64>>(c: &C, header: HeaderFn<C>, x: &Expect) {
    let validate: fn(&C, f64, f64) -> ValidationResult = C::validate;
    let _: ChartOut<f64> = c.map(0.25f64, 0.75f64);
    assert_eq!(c.name(), x.name, "the chart's name() is not its own");
    let mut out = String::new();
    header(&mut out, c).expect("writing to a String does not fail");
    assert!(
        out.contains(x.name),
        "the dump header does not carry the chart's name: {out:?}"
    );
    assert_eq!(
        c.forbids_energy_normalisation(),
        x.forbids,
        "forbids_energy_normalisation is not the chart's own"
    );
    for &((u, v), want) in &x.validations {
        let got = validate(c, u, v);
        assert_eq!(
            got, want,
            "validate({u}, {v}) is not the chart's own result"
        );
        assert_eq!(
            failed(&got),
            failed(&want),
            "validate({u}, {v}) names another constraint"
        );
    }
}

/// `map` from the one generic source at f32 (the kernel's instantiation, parameters held at f32) and at f64 (the CPU's),
/// through the trait bound alone.
fn map_both<C32: Chart<Param = f32>, C64: Chart<Param = f64>>(
    c32: &C32,
    c64: &C64,
) -> (ChartOut<f32>, ChartOut<f64>) {
    (c32.map(0.25f32, 0.75f32), c64.map(0.25f64, 0.75f64))
}

#[test]
fn qa_chart_trait_four_methods_and_header() {
    let (a, b) = map_both(&Probe { z: [0.5f32; 8] }, &Probe { z: [0.5f64; 8] });
    assert!(
        a.z.z == [0.5f32; 8] && b.z.z == [0.5f64; 8],
        "the probe's map is not its own"
    );
    let (a, b) = map_both(
        &chart_at::<f32>(CHARTS[1].0, CHARTS[1].1, CHARTS[1].2),
        &chart_at::<f64>(CHARTS[1].0, CHARTS[1].1, CHARTS[1].2),
    );
    assert!(
        a.z.z.map(f64::from) == b.z.z,
        "the latent chart's f32 and f64 maps differ on a dyadic chart"
    );
    check_trait(
        &Probe { z: [0.0; 8] },
        chart::write_dump_header,
        &probe_expect(),
    );
    // The latent chart through the same generic: its own name, no energy refusal, Valid.
    let l = chart_at::<f64>(CENTRES[1], e(0), e(1));
    check_trait(
        &l,
        chart::write_dump_header,
        &Expect {
            name: "Latent",
            forbids: false,
            validations: vec![((0.5, 0.5), ValidationResult::Valid)],
        },
    );
}

/// A header writer with the name fixed to the latent chart's.
#[cfg(feature = "controls")]
fn fixed_header<C: Chart>(out: &mut String, _c: &C) -> core::fmt::Result {
    use core::fmt::Write;
    writeln!(out, "chart: Latent")
}

negative_control!(
    qa_chart_trait_four_methods_and_header,
    "a dump header that writes a constant name misses the probe's",
    expected = "the dump header does not carry the chart's name",
    check_trait(&Probe { z: [0.0; 8] }, fixed_header, &probe_expect())
);

mod trait_validate_control {
    use super::*;
    // The latent chart judged by the probe's validations: its Valid past the edge is not the probe's Reject, so the
    // check tells charts' validate apart.
    negative_control!(
        qa_chart_trait_four_methods_and_header,
        "the latent chart does not give the probe's validate results",
        expected = "is not the chart's own result",
        check_trait(
            &chart_at::<f64>(CENTRES[1], e(0), e(1)),
            chart::write_dump_header,
            &Expect {
                name: "Latent",
                forbids: false,
                validations: probe_expect().validations,
            },
        )
    );
}

/// `chart::decode` is the shared decoder on `Φ`'s output, for a chart that is not the latent one (REQ-CHART-015).
fn check_shared_decode(dec: fn(&Probe<f64>, f64, f64) -> Decoded<f64>) {
    for z in CENTRES {
        let p = Probe { z };
        let want = decode::decode::<Defaults, f64>(&decode::Latent::new(z));
        for (u, v) in [(0.0, 0.0), (0.5, 0.5), (1.0, 0.25), (-0.5, 1.5)] {
            assert!(
                dec(&p, u, v) == want,
                "chart::decode is not the shared decoder on Φ's output at ({u}, {v})"
            );
        }
    }
}

#[test]
fn qa_chart_decode_is_shared_decoder_on_phi() {
    check_shared_decode(chart::decode::<Defaults, f64, Probe<f64>>);
}

negative_control!(
    qa_chart_decode_is_shared_decoder_on_phi,
    "a decode under another link selection is not D ∘ Φ under the defaults",
    expected = "chart::decode is not the shared decoder on Φ's output",
    check_shared_decode(chart::decode::<Tanhs, f64, Probe<f64>>)
);

// ── R-26: the latent chart's validate, and layer 3 where it says Valid ─────────────────────────────────────────

type LatentValidate = fn(&Latent<f64>, f64, f64) -> ValidationResult;

/// The latent chart's `validate` is `Valid` at every pixel inside and past `[0,1]²` (layer 2 adds nothing for it;
/// layer 1 is never a pixel's, chart_decoder_contract Part 3), including far past the edge; and where it says
/// `Valid`, the decoded masses are positive (layer 3's mass check), under every link family.
fn check_latent_validate(validate: LatentValidate) {
    let mut far = coords();
    far.extend([-1.0e3, 1.0e3, -1.0e6, 1.0e6]);
    for (z0, q1, q2) in CHARTS {
        let c = chart_at::<f64>(z0, q1, q2);
        for &u in &far {
            for &v in &far {
                let r = validate(&c, u, v);
                assert_eq!(
                    r,
                    ValidationResult::Valid,
                    "the latent chart's validate refused ({u}, {v})"
                );
                fn masses<S: Selection>(c: &Latent<f64>, u: f64, v: f64) -> [f64; 3] {
                    chart::decode::<S, f64, Latent<f64>>(c, u, v).ic.m
                }
                for m in [
                    masses::<Sigmoids>(&c, u, v),
                    masses::<Stick>(&c, u, v),
                    masses::<Tanhs>(&c, u, v),
                    masses::<Softsigns>(&c, u, v),
                ] {
                    assert!(
                        m.iter().all(|&x| x > 0.0),
                        "validate is Valid at ({u}, {v}) but a decoded mass is not positive: {m:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_latent_validate_valid_everywhere() {
    check_latent_validate(Latent::validate);
}

/// A `validate` that applies the hypercube to the chart point (layer 1, which is never a pixel's).
#[cfg(feature = "controls")]
fn hypercube(c: &Latent<f64>, u: f64, v: f64) -> ValidationResult {
    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
        c.validate(u, v)
    } else {
        ValidationResult::Clamp {
            u: u.clamp(0.0, 1.0),
            v: v.clamp(0.0, 1.0),
            moved: [!(0.0..=1.0).contains(&u), !(0.0..=1.0).contains(&v)],
            failed: Constraint::MassBuffer,
        }
    }
}

negative_control!(
    qa_latent_validate_valid_everywhere,
    "a validate clamping to [0,1]² refuses the affine continuation",
    expected = "the latent chart's validate refused",
    check_latent_validate(hypercube)
);
