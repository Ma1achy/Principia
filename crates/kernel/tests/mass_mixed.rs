//! The ternary mass plot and the mixed-axis charts (`kernel::chart::mass_simplex`, `kernel::chart::mixed`;
//! chart_reference §1.3, §4.4, §4.5; inverse_encode_contract § "Chart-aware validation"), each check at the f64 and the
//! f32 instantiation natively where it decodes (R-118, R-265):
//! - `ternary_mass_*`: every pixel's masses are `≥ ε_m` and sum to 1; the masses are the shear's, buffered; the
//!   geometry is the Burrau triangle at `ν₀` under every pixel's masses, at rest; the Burrau marker lands at the Burrau
//!   masses; validation is the buffered simplex, decided exactly at the edge (REQ-CHART-026).
//! - `mixed_axis_*`: the `(α, p_ρ)` chart decodes through the shared decoder, its axes the decoder's own `α` and `p_ρ`,
//!   the other six frozen; a latent axis places `z` directly; feasibility is the conjunction of both axes' checks
//!   (REQ-CHART-008).
//! - `per_pixel_mass_stage_*`: each chart whose masses vary per pixel has a decode-stage entry point, whose native call
//!   is the chart's own decode, and a uniform that round-trips its parameters.
//! - `per_pixel_mass_declared_*`: every chart declares `requires_per_pixel_mass`, true exactly for the ternary plot, the
//!   Burrau family, a mixed chart with a mass axis and the latent chart, whose type cannot say (REQ-DEC-006).
//! - `per_pixel_mass_trait_decode_*`: on each chart that declares it and whose masses its type decides, the trait's
//!   `Chart::decode` is the decode stage's `ChartDecode::decode_at`, and its masses vary per pixel (REQ-DEC-006); on the
//!   invariant charts, which declare no per-pixel mass, the trait decode is `decode_at` too.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

mod decode_support;

use decode_support::*;
use kernel::burrau::System;
use kernel::chart::burrau::{embed, RestBlocks};
use kernel::chart::invariant::{Invariant, WarpReal};
use kernel::chart::mass_simplex::{self, masses, system};
use kernel::chart::mixed::{
    Alpha, Beta, Coordinate, PLambdaX, PLambdaY, PRhoX, PRhoY, Quantity, ZAlpha, ZBeta, ZMu1, ZMu2,
    ZQ0, ZQ1, ZQ2, ZQ3,
};
use kernel::chart::shape_sphere::ShapeMap;
use kernel::chart::{
    family_label, BlockTouch, Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart,
    ChartDecode, ChartOut, Constraint, EuclidPlane, InvariantLE, InvariantLK,
    Latent as LatentChart, MassSimplex, Mixed, ShapeSphere, ValidationResult,
};
use kernel::decode::{self, mass, Decoded, Latent};
use kernel::generated::links::*;
use kernel::stage::decode::{
    burrau_defaults, descriptor_words, euclid_plane_defaults, flags, ic_words,
    mass_simplex_defaults, mixed_mu1_alpha_defaults, read_descriptor, read_ic, sample, untag,
    BurrauDefaults, DecodeMode, DecodeUniforms, DecodeVariant, EuclidPlaneDefaults,
    MassSimplexDefaults, MixedMu1AlphaDefaults, UniformChart,
};
use spirv_std::glam::UVec3;
use validation::{negative_control, MASS_SIMPLEX_EPS_M};

/// A precision under test.
trait Q: P + WarpReal {
    const LABEL: &'static str;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
}

/// The register's `ε_m` (chart_reference §4.5).
fn eps_m() -> f64 {
    MASS_SIMPLEX_EPS_M.number()
}

/// The ternary plot at `ν₀` under the register's `ε_m`, at `R`.
fn ternary<R: Q>(nu0: f64) -> MassSimplex<R> {
    MassSimplex {
        nu0: R::of(nu0),
        eps_m: R::of(eps_m()),
    }
}

/// The points of `[0,1]²` checked: a 33 × 33 lattice, edges and corners included, and the points a hair inside each
/// edge.
fn grid() -> Vec<(f64, f64)> {
    let mut t: Vec<f64> = (0..=32).map(|i| i as f64 / 32.0).collect();
    t.extend([1e-12, 1.0 - 1e-12, 0.3]);
    t.iter()
        .flat_map(|&u| t.iter().map(move |&v| (u, v)))
        .collect()
}

/// The shapes `ν₀` checked: the classical `1/2`, its leg swap `1/3`, and three others.
const NUS: [f64; 5] = [0.5, 1.0 / 3.0, 0.2, 0.7, 0.9];

// ── ternary_mass (REQ-CHART-026) ────────────────────────────────────────────────────────────────────────────────

/// The buffered masses at `(u, v)` under `ε`, behind a function pointer so a control runs the check on a broken map.
type MassFn<R> = fn(R, R, R) -> [R; 3];

/// Over the grid, `f`'s masses at `R` are each `≥ ε_m` and sum to 1 within 4 `eps`.
fn check_buffer<R: Q>(f: MassFn<R>) {
    let eps = R::of(eps_m());
    for (u, v) in grid() {
        let m = f(eps, R::of(u), R::of(v));
        for (i, x) in m.iter().enumerate() {
            assert!(
                *x >= eps,
                "mass {i} = {} below ε_m at ({u}, {v}) at {}",
                x.f(),
                R::LABEL
            );
        }
        let sum = m.iter().map(|x| x.f()).sum::<f64>();
        assert!(
            (sum - 1.0).abs() <= 4.0 * R::eps(),
            "the masses sum to {sum} at ({u}, {v}) at {}",
            R::LABEL
        );
    }
}

#[test]
fn ternary_mass_buffered_simplex() {
    check_buffer::<f64>(masses);
    check_buffer::<f32>(masses);
    // The map is §4.5's: the shear `x = u`, `y = (1 − u)·v`, `m₀ = 1 − x − y`, then `(1 − 3ε)·m + ε`, against the
    // formula at f64, and the f32 map within its rounding of it.
    let e = eps_m();
    for (u, v) in grid() {
        let (x, y) = (u, (1.0 - u) * v);
        let raw = [1.0 - x - y, x, y];
        let want = raw.map(|m| (1.0 - 3.0 * e) * m + e);
        let got = masses(e, u, v);
        for i in 0..3 {
            assert!(
                (got[i] - want[i]).abs() <= 4.0 * f64::EPSILON,
                "mass {i} at ({u}, {v}): {} against {}",
                got[i],
                want[i]
            );
            let narrow = masses(e as f32, u as f32, v as f32)[i] as f64;
            assert!(
                (narrow - want[i]).abs() <= 4.0 * f32::EPSILON as f64,
                "f32 mass {i} at ({u}, {v}): {narrow} against {}",
                want[i]
            );
        }
        // The chart's masses are the function's under its own ε_m.
        assert_eq!(ternary::<f64>(0.5).masses(u, v), got);
    }
    // The corners: each mass reaches the buffer and the opposite face `1 − 2ε_m`.
    let corner = |u: f64, v: f64| masses(e, u, v);
    assert_eq!(corner(0.0, 0.0), [1.0 - 2.0 * e, e, e]);
    assert_eq!(corner(1.0, 0.0)[1], (1.0 - 3.0 * e) + e);
    assert_eq!(corner(0.0, 1.0)[2], (1.0 - 3.0 * e) + e);
}

negative_control!(
    ternary_mass_buffered_simplex,
    "the shear without the buffer must fail the ε_m floor",
    expected = "below ε_m",
    check_buffer::<f64>(|_, u, v| [1.0 - u - (1.0 - u) * v, u, (1.0 - u) * v])
);

/// The system the geometry under test gives at `ν₀` and masses `m`.
type GeometryFn<R> = fn(R, [R; 3]) -> System<R>;

/// At every grid point and `ν₀`, the decode of `f`'s system at the buffered masses, at rest: the masses are the map's,
/// untagged, every momentum 0; the triangle has its right angle at body 0 and legs in the ratio `a : b` of the Burrau
/// triangle at `ν₀`, within 64 `eps`, at unequal masses (PIT-9).
fn check_geometry<R: Q>(f: GeometryFn<R>) {
    for nu in NUS {
        let chart = ternary::<R>(nu);
        let (a, b) = (1.0 - nu * nu, 2.0 * nu);
        for (u, v) in grid() {
            let m = chart.masses(R::of(u), R::of(v));
            let mut blocks = RestBlocks::new(f(R::of(nu), m));
            let d = decode::decode_with(&mut blocks, &Latent::new([R::zero(); 8]));
            assert_eq!(d.ic.m, m, "the masses at ({u}, {v}), ν₀ = {nu}");
            assert_eq!(d.degenerate, None, "a tag at ({u}, {v}), ν₀ = {nu}");
            assert!(
                d.ic.p.iter().flatten().all(|p| *p == R::zero()),
                "a momentum at ({u}, {v}), ν₀ = {nu}: {:?}",
                d.ic.p
            );
            let r: Vec<[f64; 2]> = d.ic.r.iter().map(|p| [p[0].f(), p[1].f()]).collect();
            let leg1 = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
            let leg2 = [r[2][0] - r[0][0], r[2][1] - r[0][1]];
            let n1 = leg1[0].hypot(leg1[1]);
            let n2 = leg2[0].hypot(leg2[1]);
            let cos = (leg1[0] * leg2[0] + leg1[1] * leg2[1]) / (n1 * n2);
            let tol = 64.0 * R::eps();
            assert!(
                cos.abs() <= tol,
                "no right angle at body 0 at ({u}, {v}), ν₀ = {nu}, {}: cos = {cos}",
                R::LABEL
            );
            assert!(
                ((n1 / n2) / (a / b) - 1.0).abs() <= tol,
                "the legs' ratio {} is not a/b = {} at ({u}, {v}), ν₀ = {nu}, {}",
                n1 / n2,
                a / b,
                R::LABEL
            );
        }
    }
}

#[test]
fn ternary_mass_geometry_at_rest() {
    check_geometry::<f64>(system);
    check_geometry::<f32>(system);
    // The chart's decode is that system's, at rest, through every entry; `Φ` hands over the zero vector.
    for nu in NUS {
        let chart = ternary::<f64>(nu);
        for (u, v) in grid() {
            let mut blocks = RestBlocks::new(system(nu, chart.masses(u, v)));
            let want = decode::decode_with(&mut blocks, &Latent::new([0.0; 8]));
            assert_eq!(mass_simplex::decode(&chart, u, v), want);
            assert_eq!(Chart::decode::<Defaults, f64>(&chart, u, v), want);
            assert_eq!(chart.decode_at::<Tanhs, f64>(u, v), want);
            assert_eq!(sample::<_, Defaults, f64>(&chart, u, v).ic, want.ic);
            assert_eq!(
                chart.map::<f64>(u, v),
                ChartOut {
                    z: Latent::new([0.0; 8])
                }
            );
        }
    }
    // At the Burrau masses the system is §4.2's, `λ` included.
    let burrau = embed(0.5);
    assert_eq!(system(0.5, burrau.m), burrau);
}

negative_control!(
    ternary_mass_geometry_at_rest,
    "the Burrau λ held while the masses move must fail the right angle",
    expected = "no right angle at body 0",
    check_geometry::<f64>(|nu, m| System { m, ..embed(nu) })
);

/// The marker under test.
type MarkerFn = fn(&MassSimplex<f64>) -> [f64; 2];

/// At every `ν₀`, `f`'s marker decodes to the Burrau masses `(c, b, a)/(a+b+c)` within 4 `eps`, and lies inside
/// `[0,1]²`.
fn check_marker(f: MarkerFn) {
    for nu in NUS {
        let chart = ternary::<f64>(nu);
        let [u, v] = f(&chart);
        assert!(
            (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v),
            "the marker ({u}, {v}) at ν₀ = {nu} is outside [0,1]²"
        );
        let got = Chart::decode::<Defaults, f64>(&chart, u, v).ic.m;
        let (a, b, c) = (1.0 - nu * nu, 2.0 * nu, 1.0 + nu * nu);
        let want = [c, b, a].map(|x| x / (a + b + c));
        for i in 0..3 {
            assert!(
                (got[i] - want[i]).abs() <= 4.0 * f64::EPSILON,
                "the marker's mass {i} at ν₀ = {nu} is {}, not the Burrau mass {}",
                got[i],
                want[i]
            );
        }
    }
}

#[test]
fn ternary_mass_burrau_marker() {
    check_marker(MassSimplex::marker);
    // At ν₀ = 1/2, the classical Burrau masses (5, 4, 3)/12.
    let chart = ternary::<f64>(0.5);
    let [u, v] = chart.marker();
    let m = chart.masses(u, v);
    for (got, want) in m.iter().zip([5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0]) {
        assert!((got - want).abs() <= 4.0 * f64::EPSILON, "{m:?}");
    }
}

negative_control!(
    ternary_mass_burrau_marker,
    "a marker that does not undo the buffer must miss the Burrau masses",
    expected = "not the Burrau mass",
    check_marker(|c| {
        let m = embed(c.nu0).m;
        [m[1], m[2] / (1.0 - m[1])]
    })
);

/// The validation under test.
type ValidateFn = fn(&MassSimplex<f64>, f64, f64) -> ValidationResult;

/// A clamp onto `(u, v)` with `moved`, naming the buffered simplex.
fn clamp(u: f64, v: f64, moved: [bool; 2]) -> ValidationResult {
    ValidationResult::Clamp {
        u,
        v,
        moved,
        failed: Constraint::MassBuffer,
    }
}

/// `f` accepts `[0,1]²` and the edge `u = 1`, and clamps every other point onto `[0,1]²`, a point within `ε_m` of an
/// edge included, where the buffered masses are still positive; a non-finite coordinate is rejected.
fn check_validate(f: ValidateFn) {
    let chart = ternary::<f64>(0.5);
    let e = eps_m();
    for (u, v) in grid() {
        assert_eq!(f(&chart, u, v), ValidationResult::Valid, "({u}, {v})");
    }
    for v in [-3.0, -e, 0.5, 1.0 + e, 7.0] {
        assert_eq!(f(&chart, 1.0, v), ValidationResult::Valid, "(1, {v})");
    }
    let cases = [
        ((-e / 2.0, 0.5), clamp(0.0, 0.5, [true, false])),
        ((-1e-300, 0.25), clamp(0.0, 0.25, [true, false])),
        ((0.5, -e / 2.0), clamp(0.5, 0.0, [false, true])),
        ((0.5, 1.0 + e / 2.0), clamp(0.5, 1.0, [false, true])),
        ((1.0 + e / 2.0, 0.5), clamp(1.0, 0.5, [true, false])),
        ((1.5, 1.0), clamp(1.0, 1.0, [true, false])),
        ((1.5, 0.0), clamp(1.0, 0.0, [true, false])),
        ((1.5, 1.5), clamp(1.0, 1.0, [true, true])),
        ((1.5, -0.5), clamp(1.0, 0.0, [true, true])),
        ((-0.5, 2.0), clamp(0.0, 1.0, [true, true])),
    ];
    for ((u, v), want) in cases {
        // Within ε_m of the edge the buffered masses are positive, so only the raw masses' signs decide.
        assert_eq!(f(&chart, u, v), want, "({u}, {v}) must clamp");
        if let ValidationResult::Clamp { u: cu, v: cv, .. } = want {
            assert_eq!(f(&chart, cu, cv), ValidationResult::Valid);
            let d = Chart::decode::<Defaults, f64>(&chart, cu, cv);
            assert!(d.ic.r.iter().flatten().all(|x| x.is_finite()));
        }
    }
    for (u, v) in [
        (f64::NAN, 0.5),
        (0.5, f64::INFINITY),
        (f64::NEG_INFINITY, 0.0),
    ] {
        assert_eq!(
            f(&chart, u, v),
            ValidationResult::Reject {
                failed: Constraint::MassBuffer
            },
            "({u}, {v})"
        );
    }
}

#[test]
fn ternary_mass_validate() {
    check_validate(|c, u, v| c.validate(u, v));
    // The geometry needs ν₀ ∈ (0, 1), whatever the point.
    for nu in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        assert_eq!(
            ternary::<f64>(nu).validate(0.5, 0.5),
            ValidationResult::Reject {
                failed: Constraint::BurrauNu
            },
            "ν₀ = {nu}"
        );
    }
    for nu in [f64::MIN_POSITIVE, 1.0 - f64::EPSILON / 2.0] {
        assert_eq!(
            ternary::<f64>(nu).validate(0.5, 0.5),
            ValidationResult::Valid
        );
    }
    // At f32 parameters the check reads them widened.
    assert_eq!(
        ternary::<f32>(0.5).validate(1.5, 0.5),
        clamp(1.0, 0.5, [true, false])
    );
}

negative_control!(
    ternary_mass_validate,
    "a check of buffered mᵢ > 0 must let a point within ε_m of the edge through",
    expected = "must clamp",
    check_validate(|c, u, v| {
        if !(u.is_finite() && v.is_finite()) {
            return ValidationResult::Reject {
                failed: Constraint::MassBuffer,
            };
        }
        if c.masses(u, v).iter().all(|m| *m > 0.0) || u == 1.0 {
            ValidationResult::Valid
        } else {
            let (cu, cv) = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
            clamp(cu, cv, [cu != u, cv != v])
        }
    })
);

/// At every corner of `[0,1]²`, `chart`'s f32 decode is untagged, its `M₀₁` at least the floor `2ε_m`, which lies
/// above `ε_M01`.
fn check_corners(chart: MassSimplex<f32>) {
    for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let d = Chart::decode::<Defaults, f32>(&chart, u, v);
        assert_eq!(d.degenerate, None, "a tag at the corner ({u}, {v})");
        assert!(
            d.masses.m01.f() >= 2.0 * eps_m() * (1.0 - 1e-6),
            "M₀₁ at ({u}, {v})"
        );
    }
}

#[test]
fn ternary_mass_flags() {
    let chart = ternary::<f64>(0.5);
    assert_eq!(chart.name(), "MassSimplex");
    assert!(chart.requires_per_pixel_mass());
    assert!(!chart.forbids_energy_normalisation());
    check_corners(ternary::<f32>(0.5));
}

negative_control!(
    ternary_mass_flags,
    "the plot with no buffer must reach M01_TINY at a corner",
    expected = "a tag at the corner",
    check_corners(MassSimplex {
        nu0: 0.5,
        eps_m: 0.0
    })
);

// ── mixed_axis (REQ-CHART-008) ──────────────────────────────────────────────────────────────────────────────────

/// The `(α, p_ρ)` chart.
type AlphaPRho<R> = Mixed<Alpha, PRhoX, R>;

/// The frozen controls: unequal masses (PIT-9), and the two coordinates the axes set at values no axis gives.
const FROZEN: [f64; 8] = [3.0, 0.4, -4.0, -0.3, 0.7, 0.2, 0.8, -0.5];

/// The `(α, p_ρ)` chart on `α ∈ [0.05, 1.5]`, `p_ρ,x ∈ [−1.8, 1.8]`, at `R`.
fn alpha_prho<R: Q>() -> AlphaPRho<R> {
    Mixed::new(at(FROZEN), at([0.05, 1.5]), at([-1.8, 1.8]))
}

/// A mixed chart's decode under test.
type MixedDecodeFn<R> = fn(&AlphaPRho<R>, R, R) -> Decoded<R>;

/// Over the grid, `f`'s decode has the decoder's `α` at the horizontal axis's value and `p_ρ,x` at the vertical's,
/// within 8 `eps` of their ranges; every other output of the decoder is the frozen controls'; and at those unequal
/// masses `Σpᵢ = 0` and `p_ρ = (m₀p₁ − m₁p₀)/M₀₁`, the inner pair's Jacobi momentum read back off the particles (PIT-9).
fn check_shared<R: Q>(f: MixedDecodeFn<R>) {
    let chart = alpha_prho::<R>();
    let axes = alpha_prho::<f64>();
    let frozen: [R; 8] = at(FROZEN);
    let masses = mass::decode::<Defaults, R>([frozen[6], frozen[7]]);
    let [beta] = SigmoidBeta::forward([frozen[1]]);
    let q = [frozen[3], frozen[4], frozen[5]].map(|z| SigmoidQ::forward([z])[0]);
    for (u, v) in grid() {
        let d = f(&chart, R::of(u), R::of(v));
        let (alpha, p) = (axes.h(u), axes.v(v));
        let tol = 8.0 * R::eps();
        assert!(
            (d.config.alpha.f() - alpha).abs() <= tol * 1.5,
            "α = {} is not the axis's {alpha} at ({u}, {v}), {}",
            d.config.alpha.f(),
            R::LABEL
        );
        assert!(
            (d.momentum.p_rho[0].f() - p).abs() <= tol * 2.0,
            "p_ρ,x = {} is not the axis's {p} at ({u}, {v}), {}",
            d.momentum.p_rho[0].f(),
            R::LABEL
        );
        assert_eq!(d.masses, masses, "the masses at ({u}, {v})");
        assert_eq!(d.config.beta, beta, "β at ({u}, {v})");
        assert_eq!(
            [d.momentum.q[1], d.momentum.q[2], d.momentum.q[3]],
            q,
            "the frozen momenta at ({u}, {v})"
        );
        let m = d.ic.m.map(|x| x.f());
        let pp: Vec<[f64; 2]> = d.ic.p.iter().map(|x| [x[0].f(), x[1].f()]).collect();
        let scale: f64 = pp.iter().flatten().map(|x| x.abs()).sum::<f64>() + 1.0;
        for (k, p_rho) in d.momentum.p_rho.iter().enumerate() {
            let sum = pp[0][k] + pp[1][k] + pp[2][k];
            assert!(
                sum.abs() <= 8.0 * R::eps() * scale,
                "Σp = {sum} at ({u}, {v})"
            );
            let back = (m[0] * pp[1][k] - m[1] * pp[0][k]) / (m[0] + m[1]);
            assert!(
                (back - p_rho.f()).abs() <= 8.0 * R::eps() * scale,
                "p_ρ read back off the particles is {back}, not {} at ({u}, {v})",
                p_rho.f()
            );
        }
    }
}

#[test]
fn mixed_axis_alpha_prho_shared_decoder() {
    check_shared::<f64>(Chart::decode::<Defaults, f64>);
    check_shared::<f32>(Chart::decode::<Defaults, f32>);
    // `Φ_mixed(u, v) = D(z(u, v))`: the shared decoder on the frozen controls with `z_α` the α link's inverse at the
    // axis's value and `z_q0` the q₀ link's, under each selection, through every entry.
    let chart = alpha_prho::<f64>();
    for (u, v) in grid() {
        let mut z = FROZEN;
        z[0] = SigmoidAlpha::inverse([chart.h(u)])[0];
        z[2] = SigmoidQ::inverse([chart.v(v)])[0];
        assert_eq!(chart.controls::<Defaults, f64>(u, v), z);
        let want = decode::decode::<Defaults, f64>(&Latent::new(z));
        assert_eq!(Chart::decode::<Defaults, f64>(&chart, u, v), want);
        assert_eq!(chart.decode_at::<Defaults, f64>(u, v), want);
        assert_eq!(sample::<_, Defaults, f64>(&chart, u, v).ic, want.ic);
        let mut z = FROZEN;
        z[0] = TanhAlpha::inverse([chart.h(u)])[0];
        z[2] = TanhQ::inverse([chart.v(v)])[0];
        assert_eq!(chart.controls::<Tanhs, f64>(u, v), z);
        let d = Chart::decode::<Tanhs, f64>(&chart, u, v);
        assert_eq!(d, decode::decode::<Tanhs, f64>(&Latent::new(z)));
        assert!(
            (d.config.alpha - chart.h(u)).abs() <= 1e-13,
            "tanh α at ({u}, {v})"
        );
        // `map` places no derived axis: both of this chart's coordinates stay frozen.
        assert_eq!(chart.map::<f64>(u, v).z, Latent::new(FROZEN));
    }
}

negative_control!(
    mixed_axis_alpha_prho_shared_decoder,
    "decoding map's controls, which leave a derived axis frozen, must miss the axis's α",
    expected = "is not the axis's",
    check_shared::<f64>(|c, u, v| decode::decode::<Defaults, f64>(&c.map(u, v).z))
);

/// The `(α, p_ρ)` chart's validation under test.
type MixedValidateFn = fn(&AlphaPRho<f64>, f64, f64) -> ValidationResult;

/// A clamp onto `(u, v)` with `moved`, naming an axis's domain.
fn axis_clamp(u: f64, v: f64, moved: [bool; 2]) -> ValidationResult {
    ValidationResult::Clamp {
        u,
        v,
        moved,
        failed: Constraint::AxisDomain,
    }
}

/// On `α ∈ [−0.3, 1.9]`, `p_ρ,x ∈ [−2.5, 2.5]`, past both domains: `f` is valid where both axes are in their domains and
/// clamps each axis that is not, alone or together, onto where it meets its edge, the clamped point valid and its decode
/// on the edge; a non-finite coordinate is rejected.
fn check_conjunction(f: MixedValidateFn) {
    let chart: AlphaPRho<f64> = Mixed::new(FROZEN, [-0.3, 1.9], [-2.5, 2.5]);
    let edge_u = |alpha: f64| (alpha + 0.3) / 2.2;
    let edge_v = |p: f64| (p + 2.5) / 5.0;
    assert_eq!(f(&chart, 0.5, 0.5), ValidationResult::Valid);
    let lo = edge_u(0.0);
    let hi_u = edge_u(core::f64::consts::FRAC_PI_2);
    let hi_v = edge_v(2.0);
    let lo_v = edge_v(-2.0);
    let cases = [
        ((0.0, 0.5), [true, false], (lo, 0.5)),
        ((1.0, 0.5), [true, false], (hi_u, 0.5)),
        ((0.5, 1.0), [false, true], (0.5, hi_v)),
        ((0.5, 0.0), [false, true], (0.5, lo_v)),
        ((0.0, 1.0), [true, true], (lo, hi_v)),
        ((1.0, 0.0), [true, true], (hi_u, lo_v)),
    ];
    for ((u, v), moved, (cu, cv)) in cases {
        let got = f(&chart, u, v);
        let ValidationResult::Clamp {
            u: gu,
            v: gv,
            moved: gm,
            failed,
        } = got
        else {
            panic!("({u}, {v}) past an axis's domain must clamp, not {got:?}");
        };
        assert_eq!((gm, failed), (moved, Constraint::AxisDomain), "({u}, {v})");
        assert!(
            (gu - cu).abs() <= 1e-12 && (gv - cv).abs() <= 1e-12,
            "({u}, {v}) clamped to ({gu}, {gv}), not ({cu}, {cv})"
        );
        assert_eq!(f(&chart, gu, gv), ValidationResult::Valid, "({gu}, {gv})");
        let d = Chart::decode::<Defaults, f64>(&chart, gu, gv);
        assert!(d
            .ic
            .r
            .iter()
            .chain(&d.ic.p)
            .flatten()
            .all(|x| x.is_finite()));
        assert!((0.0..=core::f64::consts::FRAC_PI_2).contains(&chart.h(gu)));
        assert!((-2.0..=2.0).contains(&chart.v(gv)));
    }
    for (u, v) in [(f64::NAN, 0.5), (0.5, f64::INFINITY)] {
        assert_eq!(
            f(&chart, u, v),
            ValidationResult::Reject {
                failed: Constraint::AxisDomain
            },
            "({u}, {v})"
        );
    }
}

#[test]
fn mixed_axis_feasibility_conjunction() {
    check_conjunction(|c, u, v| c.validate(u, v));
    // The domains are closed: α at 0 and π/2 and p at ±q_max are valid; one ulp past either is clamped, back inside.
    let exact: AlphaPRho<f64> =
        Mixed::new(FROZEN, [0.0, core::f64::consts::FRAC_PI_2], [-2.0, 2.0]);
    for (u, v) in [(0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0)] {
        assert_eq!(exact.validate(u, v), ValidationResult::Valid, "({u}, {v})");
    }
    assert_eq!(
        exact.validate(1.0f64.next_up(), 0.5),
        axis_clamp(1.0, 0.5, [true, false])
    );
    assert_eq!(
        exact.validate(0.5, -1e-12),
        axis_clamp(0.5, 0.0, [false, true])
    );
    // A reversed range clamps the same way.
    let reversed: AlphaPRho<f64> = Mixed::new(FROZEN, [1.9, -0.3], [2.5, -2.5]);
    for (u, v) in [(0.0, 0.5), (1.0, 0.5), (0.5, 0.0), (0.5, 1.0)] {
        let ValidationResult::Clamp { u: cu, v: cv, .. } = reversed.validate(u, v) else {
            panic!("({u}, {v}) on the reversed chart must clamp");
        };
        assert_eq!(reversed.validate(cu, cv), ValidationResult::Valid);
    }
    // Where the edge's `t` rounds to an `α` a hair past the edge, the clamp steps inward, from either edge, on a rising
    // and a falling range: each clamped point is valid and its `α` in `[0, π/2]`.
    for (lo, hi, u) in [
        (-0.7984536338313981, 2.7864072317121256, 1.0),
        (2.803925140313406, 0.4888151182640976, 1.5),
        (-0.9347690644652089, 2.5847349240453705, 0.0),
        (2.934716228582044, 0.22785132669049601, 0.0),
    ] {
        let chart: Mixed<Alpha, ZQ0, f64> = Mixed::new(FROZEN, [lo, hi], [0.0, 1.0]);
        let edge = if lo + (hi - lo) * u > 1.0 {
            core::f64::consts::FRAC_PI_2
        } else {
            0.0
        };
        let naive = (edge - lo) / (hi - lo);
        let x = lo + (hi - lo) * naive;
        assert!(
            !(0.0..=core::f64::consts::FRAC_PI_2).contains(&x),
            "[{lo}, {hi}]: no rounding past the edge"
        );
        let ValidationResult::Clamp { u: cu, .. } = chart.validate(u, 0.5) else {
            panic!("[{lo}, {hi}] at u = {u} must clamp");
        };
        assert!(
            (0.0..=core::f64::consts::FRAC_PI_2).contains(&chart.h(cu)),
            "[{lo}, {hi}]: {}",
            chart.h(cu)
        );
        assert_eq!(chart.validate(cu, 0.5), ValidationResult::Valid);
        assert!(
            (cu - naive).abs() <= 4.0 * f64::EPSILON,
            "[{lo}, {hi}]: {cu} far from {naive}"
        );
    }
    // A range of no width past the domain has nowhere to clamp to.
    let pinned: AlphaPRho<f64> = Mixed::new(FROZEN, [2.0, 2.0], [0.0, 1.0]);
    assert_eq!(
        pinned.validate(0.5, 0.5),
        ValidationResult::Reject {
            failed: Constraint::AxisDomain
        }
    );
    let unbounded: AlphaPRho<f64> = Mixed::new(FROZEN, [-1e308, 1e308], [0.0, 1.0]);
    assert_eq!(
        unbounded.validate(0.0, 0.5),
        ValidationResult::Reject {
            failed: Constraint::AxisDomain
        }
    );
    let inside: AlphaPRho<f64> = Mixed::new(FROZEN, [1.0, 1.0], [0.0, 1.0]);
    assert_eq!(inside.validate(0.5, 0.5), ValidationResult::Valid);
    // A latent axis takes any finite z, and is refused only where its value is not finite.
    let latent: Mixed<ZBeta, ZQ3, f64> = Mixed::new(FROZEN, [-1e300, 1e300], [-50.0, 50.0]);
    assert_eq!(latent.validate(0.9, -3.0), ValidationResult::Valid);
    assert_eq!(
        latent.validate(1e10, 0.5),
        ValidationResult::Reject {
            failed: Constraint::AxisDomain
        }
    );
    // At f32 parameters the check reads them widened.
    let narrow: AlphaPRho<f32> = Mixed::new(at(FROZEN), [-0.3, 1.9], [-2.5, 2.5]);
    assert!(matches!(
        narrow.validate(0.0, 0.5),
        ValidationResult::Clamp {
            moved: [true, false],
            ..
        }
    ));
}

negative_control!(
    mixed_axis_feasibility_conjunction,
    "a check of the horizontal axis alone must let the vertical axis past its domain",
    expected = "must clamp",
    check_conjunction(
        |c, u, v| Mixed::<Alpha, ZQ0, f64>::new(c.frozen, c.h_range, c.v_range).validate(u, v)
    )
);

/// The `(z_β, z_q3)` chart on `z_β ∈ [−2, 3]`, `z_q3 ∈ [4, −1]`.
type Latents = Mixed<ZBeta, ZQ3, f64>;

/// A latent chart's placement under test.
type PlaceFn = fn(&Latents, f64, f64) -> [f64; 8];

/// Over the grid, `f` places `z_β = −2 + 5u` and `z_q3 = 4 − 5v` and keeps the frozen rest: a latent axis is kind 1,
/// `z_k` itself on its range; the decode is the shared decoder's on that `z`.
fn check_latent(f: PlaceFn) {
    let chart: Latents = Mixed::new(FROZEN, [-2.0, 3.0], [4.0, -1.0]);
    for (u, v) in grid() {
        let mut z = FROZEN;
        z[1] = -2.0 + 5.0 * u;
        z[5] = 4.0 - 5.0 * v;
        assert_eq!(f(&chart, u, v), z, "the controls at ({u}, {v})");
        assert_eq!(
            Chart::decode::<Defaults, f64>(&chart, u, v),
            decode::decode::<Defaults, f64>(&Latent::new(z))
        );
    }
}

#[test]
fn mixed_axis_latent_axes() {
    check_latent(|c, u, v| c.map::<f64>(u, v).z.z);
    check_latent(|c, u, v| c.controls::<Defaults, f64>(u, v));
    // A mass axis and a derived axis: `map` places the mass control, `controls` both.
    let chart: Mixed<ZMu1, Alpha, f64> = Mixed::new(FROZEN, [-1.0, 1.0], [0.2, 1.2]);
    let mut z = FROZEN;
    z[6] = 0.0;
    assert_eq!(chart.map::<f64>(0.5, 0.5).z, Latent::new(z));
    z[0] = SigmoidAlpha::inverse([0.7])[0];
    assert_eq!(chart.controls::<Defaults, f64>(0.5, 0.5), z);
    // Each axis sets its own coordinate and no other.
    fn coordinate<H: Quantity>() -> (Coordinate, bool) {
        (H::COORDINATE, H::DERIVED)
    }
    use Coordinate as C;
    assert_eq!(
        [
            coordinate::<ZAlpha>(),
            coordinate::<ZBeta>(),
            coordinate::<ZQ0>(),
            coordinate::<ZQ1>(),
            coordinate::<ZQ2>(),
            coordinate::<ZQ3>(),
            coordinate::<ZMu1>(),
            coordinate::<ZMu2>(),
            coordinate::<Alpha>(),
            coordinate::<Beta>(),
            coordinate::<PRhoX>(),
            coordinate::<PRhoY>(),
            coordinate::<PLambdaX>(),
            coordinate::<PLambdaY>(),
        ],
        [
            (C::ZAlpha, false),
            (C::ZBeta, false),
            (C::ZQ0, false),
            (C::ZQ1, false),
            (C::ZQ2, false),
            (C::ZQ3, false),
            (C::ZMu1, false),
            (C::ZMu2, false),
            (C::ZAlpha, true),
            (C::ZBeta, true),
            (C::ZQ0, true),
            (C::ZQ1, true),
            (C::ZQ2, true),
            (C::ZQ3, true),
        ]
    );
    // Each derived axis is set through its own link's inverse and its domain is that link's codomain.
    // Each derived axis is set through its own link's inverse; the vertical mass axis, pinned at 0.8, sets `z_μ2`.
    let x = 0.3;
    fn controls<H: Quantity>(x: f64) -> [f64; 8] {
        Mixed::<H, ZMu2, f64>::new(FROZEN, [x, x], [0.8, 0.8]).controls::<Defaults, f64>(0.0, 0.0)
    }
    let place = |k: usize, w: f64| {
        let mut z = FROZEN;
        z[k] = w;
        z[7] = 0.8;
        z
    };
    let q = SigmoidQ::inverse([x])[0];
    assert_eq!(controls::<Beta>(x), place(1, SigmoidBeta::inverse([x])[0]));
    assert_eq!(controls::<PRhoX>(x), place(2, q));
    assert_eq!(controls::<PRhoY>(x), place(3, q));
    assert_eq!(controls::<PLambdaX>(x), place(4, q));
    assert_eq!(controls::<PLambdaY>(x), place(5, q));
    assert_eq!(controls::<ZQ1>(x), place(3, x));
    assert_eq!(
        <Alpha as Quantity>::DOMAIN,
        [0.0, core::f64::consts::FRAC_PI_2]
    );
    assert_eq!(<Beta as Quantity>::DOMAIN, [0.0, core::f64::consts::PI]);
    for d in [
        <PRhoX as Quantity>::DOMAIN,
        <PRhoY as Quantity>::DOMAIN,
        <PLambdaX as Quantity>::DOMAIN,
        <PLambdaY as Quantity>::DOMAIN,
    ] {
        assert_eq!(d, [-2.0, 2.0]);
    }
    assert_eq!(
        <ZMu1 as Quantity>::DOMAIN,
        [f64::NEG_INFINITY, f64::INFINITY]
    );
}

negative_control!(
    mixed_axis_latent_axes,
    "a placement with the axes swapped must fail",
    expected = "the controls at",
    check_latent(|c, u, v| c.map::<f64>(v, u).z.z)
);

/// The block-touch set under test.
type TouchFn = fn(Coordinate) -> BlockTouch;

/// `f` puts `z_α`, `z_β` in the configuration block, `z_q0` … `z_q3` in the momentum block and `z_μ1`, `z_μ2` in the
/// mass block, each in one block only.
fn check_touches(f: TouchFn) {
    let touch = |config, momentum, mass| BlockTouch {
        config,
        momentum,
        mass,
    };
    use Coordinate as C;
    for (c, want) in [
        (C::ZAlpha, touch(true, false, false)),
        (C::ZBeta, touch(true, false, false)),
        (C::ZQ0, touch(false, true, false)),
        (C::ZQ1, touch(false, true, false)),
        (C::ZQ2, touch(false, true, false)),
        (C::ZQ3, touch(false, true, false)),
        (C::ZMu1, touch(false, false, true)),
        (C::ZMu2, touch(false, false, true)),
    ] {
        assert_eq!(f(c), want, "the block of {c:?}");
    }
}

#[test]
fn mixed_axis_metadata() {
    let a = alpha_prho::<f64>();
    assert_eq!(a.name(), "Mixed");
    assert!(!a.forbids_energy_normalisation());
    let touch = |config, momentum, mass| BlockTouch {
        config,
        momentum,
        mass,
    };
    let axes = a.axes();
    assert_eq!(
        (axes[0].touches, axes[1].touches),
        (touch(true, false, false), touch(false, true, false))
    );
    assert!(!axes[0].annotation && !axes[1].annotation);
    assert_eq!(family_label(axes), "2-swept chart");
    let m: Mixed<ZMu1, ZQ2, f64> = Mixed::new(FROZEN, [0.0, 1.0], [0.0, 1.0]);
    assert_eq!(
        (m.axes()[0].touches, m.axes()[1].touches),
        (touch(false, false, true), touch(false, true, false))
    );
    check_touches(Coordinate::touches);
    // Equality reads every parameter; the label names both axes.
    let b = a;
    assert_eq!(a, b);
    assert_ne!(a, Mixed::new([0.0; 8], a.h_range, a.v_range));
    assert_ne!(a, Mixed::new(a.frozen, [0.0, 1.0], a.v_range));
    assert_ne!(a, Mixed::new(a.frozen, a.h_range, [0.0, 1.0]));
    let label = format!("{a:?}");
    assert!(
        label.starts_with("Mixed")
            && label.contains("\"alpha\"")
            && label.contains("\"p_rho_x\"")
            && label.contains("frozen")
            && label.contains("h_range")
            && label.contains("v_range"),
        "{label}"
    );
}

negative_control!(
    mixed_axis_metadata,
    "a block-touch set that puts the mass controls in the momentum block must fail",
    expected = "the block of ZMu1",
    check_touches(|c| {
        let t = c.touches();
        BlockTouch {
            momentum: t.momentum | t.mass,
            mass: false,
            ..t
        }
    })
);

// ── per_pixel_mass_declared (REQ-DEC-006) ───────────────────────────────────────────────────────────────────────

/// A chart that declares nothing beyond the trait's required items.
struct Undeclared;

impl Chart for Undeclared {
    type Param = f64;
    fn map<F: spirv_std::num_traits::Float + From<f64>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut {
            z: Latent::new([F::zero(); 8]),
        }
    }
    fn name(&self) -> &str {
        "Undeclared"
    }
    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}

#[test]
fn per_pixel_mass_declared_by_every_chart() {
    let invariant = Invariant {
        z_cfg: [0.0, 0.0],
        z_mass: [0.3, -0.2],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    let sphere = ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.2],
        z_mom: [0.0; 4],
    };
    let latent = LatentChart {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    let declared = [
        (
            "MassSimplex",
            ternary::<f64>(0.5).requires_per_pixel_mass(),
            true,
        ),
        (
            "Burrau",
            Burrau {
                nu_range: [0.1, 0.9],
                k_max: 1.0,
                gamma_k: 2.0,
            }
            .requires_per_pixel_mass(),
            true,
        ),
        (
            "EuclidPlane",
            EuclidPlane {
                m_range: [1.0, 32.0],
                nu_range: [0.1, 0.9],
            }
            .requires_per_pixel_mass(),
            true,
        ),
        (
            "BurrauThetaK",
            theta_k::<f64>().requires_per_pixel_mass(),
            true,
        ),
        (
            "BurrauThetaLz",
            theta_lz::<f64>().requires_per_pixel_mass(),
            true,
        ),
        (
            "BurrauThetaDeltaM",
            delta_m::<f64>().requires_per_pixel_mass(),
            true,
        ),
        (
            "Mixed (z_μ1, α)",
            Mixed::<ZMu1, Alpha, f64>::new(FROZEN, [0.0, 1.0], [0.1, 1.0])
                .requires_per_pixel_mass(),
            true,
        ),
        (
            "Mixed (α, z_μ2)",
            Mixed::<Alpha, ZMu2, f64>::new(FROZEN, [0.1, 1.0], [0.0, 1.0])
                .requires_per_pixel_mass(),
            true,
        ),
        (
            "Mixed (z_μ1, z_μ2)",
            Mixed::<ZMu1, ZMu2, f64>::new(FROZEN, [0.0, 1.0], [0.0, 1.0]).requires_per_pixel_mass(),
            true,
        ),
        (
            "Mixed (α, p_ρ)",
            alpha_prho::<f64>().requires_per_pixel_mass(),
            false,
        ),
        (
            "Mixed (z_β, z_q3)",
            Mixed::<ZBeta, ZQ3, f64>::new(FROZEN, [0.0, 1.0], [0.0, 1.0]).requires_per_pixel_mass(),
            false,
        ),
        ("Latent", latent.requires_per_pixel_mass(), true),
        ("ShapeSphere", sphere.requires_per_pixel_mass(), false),
        (
            "InvariantLE",
            InvariantLE(invariant).requires_per_pixel_mass(),
            false,
        ),
        (
            "InvariantLK",
            InvariantLK(invariant).requires_per_pixel_mass(),
            false,
        ),
        (
            "the trait's default",
            Undeclared.requires_per_pixel_mass(),
            true,
        ),
    ];
    for (name, got, want) in declared {
        assert_eq!(got, want, "{name} declares requires_per_pixel_mass = {got}");
    }
    // Each declaration is the masses' behaviour, wherever the chart's type decides it.
    fn behaves<C: Chart<Param = f64>>(chart: &C) {
        check_flag_is_masses(chart, chart.requires_per_pixel_mass());
    }
    behaves(&ternary::<f64>(0.5));
    behaves(&Mixed::<ZMu1, Alpha, f64>::new(
        FROZEN,
        [-1.0, 1.0],
        [0.1, 1.4],
    ));
    behaves(&Mixed::<Alpha, ZMu2, f64>::new(
        FROZEN,
        [0.1, 1.4],
        [-1.0, 1.0],
    ));
    behaves(&alpha_prho::<f64>());
    behaves(&Mixed::<ZBeta, ZQ3, f64>::new(
        FROZEN,
        [0.0, 1.0],
        [0.0, 1.0],
    ));
    behaves(&sphere);
    behaves(&Burrau {
        nu_range: [0.1, 0.9],
        k_max: 1.0,
        gamma_k: 2.0,
    });
    behaves(&theta_k::<f64>());
    behaves(&theta_lz::<f64>());
    behaves(&delta_m::<f64>());
    behaves(&EuclidPlane {
        m_range: [1.0, 32.0],
        nu_range: [0.1, 0.9],
    });
}

/// Whether `chart`'s masses vary over the grid is `flag`, the declaration under check.
fn check_flag_is_masses<C: Chart<Param = f64>>(chart: &C, flag: bool) {
    let at = |u, v| Chart::decode::<Defaults, f64>(chart, u, v).ic.m;
    let varies = grid().into_iter().any(|(u, v)| at(u, v) != at(0.5, 0.5));
    assert_eq!(
        varies,
        flag,
        "{}'s masses vary: {varies}, its declaration {flag}",
        chart.name()
    );
}

negative_control!(
    per_pixel_mass_declared_by_every_chart,
    "a mass-axis chart's masses checked against a declaration of false must fail",
    expected = "masses vary: true",
    check_flag_is_masses(
        &Mixed::<Alpha, ZMu2, f64>::new(FROZEN, [0.1, 1.4], [-1.0, 1.0]),
        false
    )
);

// ── per_pixel_mass_trait_decode (REQ-DEC-006) ───────────────────────────────────────────────────────────────────

/// A chart's decode under the default links, behind a function pointer so a control runs the check on a broken one.
type TraitDecodeFn<C, R> = fn(&C, R, R) -> Decoded<R>;

/// Over the grid at `R`, `f` (the trait's [`Chart::decode`] under check) is [`ChartDecode::decode_at`], IC, label and
/// descriptor, compared by their `Debug` forms, which round-trip every float and its sign, NaN included.
fn check_trait_decode<R: Q, C: ChartDecode<Param = R>>(chart: &C, f: TraitDecodeFn<C, R>) {
    for (u, v) in grid() {
        let (u, v) = (R::of(u), R::of(v));
        let got = format!("{:?}", f(chart, u, v));
        let want = format!("{:?}", ChartDecode::decode_at::<Defaults, R>(chart, u, v));
        assert!(
            got == want,
            "{}'s Chart::decode is not its decode_at at ({}, {}) at {}",
            chart.name(),
            u.f(),
            v.f(),
            R::LABEL
        );
    }
}

/// `chart` declares `requires_per_pixel_mass`, and its trait [`Chart::decode`] gives more than one mass triple over
/// the grid at `R`.
fn check_trait_masses_vary<R: Q, C: Chart<Param = R>>(chart: &C) {
    assert!(
        chart.requires_per_pixel_mass(),
        "{} does not declare requires_per_pixel_mass",
        chart.name()
    );
    let mut masses: Vec<String> = grid()
        .into_iter()
        .map(|(u, v)| {
            format!(
                "{:?}",
                Chart::decode::<Defaults, R>(chart, R::of(u), R::of(v)).ic.m
            )
        })
        .collect();
    masses.sort();
    masses.dedup();
    assert!(
        masses.len() > 1,
        "{}'s Chart::decode gives {} mass triple(s) over the grid at {}",
        chart.name(),
        masses.len(),
        R::LABEL
    );
}

/// The `(ν, K)` chart checked.
fn nu_k<R: Q>() -> Burrau<R> {
    Burrau {
        nu_range: [R::of(0.1), R::of(0.9)],
        k_max: R::of(1.0),
        gamma_k: R::of(2.0),
    }
}

/// The acute-angle range checked, inside `(0, π/4]`.
fn theta_range<R: Q>() -> [R; 2] {
    [R::of(0.2), R::of(core::f64::consts::FRAC_PI_4)]
}

/// The `(θ, K)` chart checked.
fn theta_k<R: Q>() -> BurrauThetaK<R> {
    BurrauThetaK {
        theta_range: theta_range(),
        k_max: R::of(1.0),
        gamma_k: R::of(2.0),
    }
}

/// The `(θ, L_z)` chart checked.
fn theta_lz<R: Q>() -> BurrauThetaLz<R> {
    BurrauThetaLz {
        theta_range: theta_range(),
        k: R::of(0.5),
        lz_fraction: R::of(0.9),
    }
}

/// The δm strip checked, toward equal masses.
fn delta_m<R: Q>() -> BurrauThetaDeltaM<R> {
    BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<R>::equal_masses(),
    }
}

/// The Euclid plane checked.
fn euclid<R: Q>() -> EuclidPlane<R> {
    EuclidPlane {
        m_range: [R::of(1.0), R::of(32.0)],
        nu_range: [R::of(0.1), R::of(0.9)],
    }
}

/// The mixed charts with a mass axis: on the horizontal axis, on the vertical, and on both.
type MassAxisCharts<R> = (
    Mixed<ZMu1, Alpha, R>,
    Mixed<Alpha, ZMu2, R>,
    Mixed<ZMu1, ZMu2, R>,
);

/// The mixed charts with a mass axis checked.
fn mass_axis_charts<R: Q>() -> MassAxisCharts<R> {
    (
        Mixed::new(at(FROZEN), at([-1.0, 1.0]), at([0.1, 1.4])),
        Mixed::new(at(FROZEN), at([0.1, 1.4]), at([-1.0, 1.0])),
        Mixed::new(at(FROZEN), at([-1.0, 1.0]), at([-1.0, 1.0])),
    )
}

/// [`check_trait_decode`] on every chart that declares `requires_per_pixel_mass` and whose masses its type decides,
/// at `R`.
fn check_trait_decodes<R: Q>()
where
    f64: From<R>,
{
    let (h, v, both) = mass_axis_charts::<R>();
    check_trait_decode(&ternary::<R>(0.5), Chart::decode::<Defaults, R>);
    check_trait_decode(&nu_k::<R>(), Chart::decode::<Defaults, R>);
    check_trait_decode(&euclid::<R>(), Chart::decode::<Defaults, R>);
    check_trait_decode(&theta_k::<R>(), Chart::decode::<Defaults, R>);
    check_trait_decode(&theta_lz::<R>(), Chart::decode::<Defaults, R>);
    check_trait_decode(&delta_m::<R>(), Chart::decode::<Defaults, R>);
    check_trait_decode(&h, Chart::decode::<Defaults, R>);
    check_trait_decode(&v, Chart::decode::<Defaults, R>);
    check_trait_decode(&both, Chart::decode::<Defaults, R>);
}

#[test]
fn per_pixel_mass_trait_decode_is_decode_at() {
    check_trait_decodes::<f64>();
    check_trait_decodes::<f32>();
}

negative_control!(
    per_pixel_mass_trait_decode_is_decode_at,
    "the trait's default decode on the (ν, K) chart, the shared decoder on Φ's zero vector, must fail the check",
    expected = "Burrau's Chart::decode is not its decode_at",
    check_trait_decode(&nu_k::<f64>(), |c, u, v| decode::decode::<Defaults, f64>(
        &c.map(u, v).z
    ))
);

/// [`check_trait_masses_vary`] on the same charts at `R`.
fn check_trait_masses<R: Q>()
where
    f64: From<R>,
{
    let (h, v, both) = mass_axis_charts::<R>();
    check_trait_masses_vary(&ternary::<R>(0.5));
    check_trait_masses_vary(&nu_k::<R>());
    check_trait_masses_vary(&euclid::<R>());
    check_trait_masses_vary(&theta_k::<R>());
    check_trait_masses_vary(&theta_lz::<R>());
    check_trait_masses_vary(&delta_m::<R>());
    check_trait_masses_vary(&h);
    check_trait_masses_vary(&v);
    check_trait_masses_vary(&both);
}

#[test]
fn per_pixel_mass_trait_decode_varies() {
    check_trait_masses::<f64>();
    check_trait_masses::<f32>();
}

negative_control!(
    per_pixel_mass_trait_decode_varies,
    "a declared chart whose masses are held, the latent chart with no sweep, must fail the check",
    expected = "Latent's Chart::decode gives 1 mass triple(s)",
    check_trait_masses_vary(&LatentChart {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    })
);

/// The invariant charts' frozen part checked: a mass point off the equal masses.
fn invariant_part<R: Q>() -> Invariant<R> {
    Invariant {
        z_cfg: [R::of(0.4), R::of(-0.3)],
        z_mass: [R::of(0.3), R::of(-0.2)],
        k_max: R::of(1.0),
        gamma_k: R::of(2.0),
    }
}

/// [`check_trait_decode`] on the invariant charts at `R`: they declare no per-pixel mass, but their trait decode is
/// still the construction's.
fn check_invariant_trait_decodes<R: Q>() {
    check_trait_decode(
        &InvariantLE(invariant_part::<R>()),
        Chart::decode::<Defaults, R>,
    );
    check_trait_decode(
        &InvariantLK(invariant_part::<R>()),
        Chart::decode::<Defaults, R>,
    );
}

#[test]
fn per_pixel_mass_trait_decode_invariant() {
    check_invariant_trait_decodes::<f64>();
    check_invariant_trait_decodes::<f32>();
}

negative_control!(
    per_pixel_mass_trait_decode_invariant,
    "the trait's default decode on the (L_z, E) chart, the shared decoder on Φ's frozen controls, must fail the check",
    expected = "InvariantLE's Chart::decode is not its decode_at",
    check_trait_decode(&InvariantLE(invariant_part::<f64>()), |c, u, v| {
        decode::decode::<Defaults, f64>(&c.map(u, v).z)
    })
);

// ── per_pixel_mass_stage ────────────────────────────────────────────────────────────────────────────────────────

/// A decode stage compute entry point, called natively.
type Entry = fn(UVec3, &DecodeUniforms, &[f32], &mut [u32], &mut [u32], &mut [u32]);

/// `chart`'s uniform round-trips it, and `entry`, called natively over a few points and three invocations past them,
/// writes at each point the stage's `sample` of `V::Chart`: its `(m, r, p)`, label and descriptor.
fn check_stage<V: DecodeVariant>(entry: Entry, chart: V::Chart)
where
    V::Chart: PartialEq + core::fmt::Debug,
{
    let rows = chart.uniform();
    assert_eq!(
        V::Chart::from_uniform(&rows),
        chart,
        "the uniform does not round-trip the chart"
    );
    let uv: Vec<f32> = [
        [0.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.7],
        [1.0, 0.3],
        [0.25, 0.9],
        [0.5, 0.5],
    ]
    .into_iter()
    .flatten()
    .collect();
    let n = uv.len() / 2;
    let uniforms = DecodeUniforms {
        chart: rows,
        flags: [flags(DecodeMode::Full), 0, 0, 0].into(),
    };
    let mut ic = vec![0; n * ic_words()];
    let mut tags = vec![u32::MAX; n];
    let mut descriptors = vec![0; n * descriptor_words()];
    for i in 0..n as u32 + 3 {
        entry(
            UVec3::new(i, 0, 0),
            &uniforms,
            &uv,
            &mut ic,
            &mut tags,
            &mut descriptors,
        );
    }
    for i in 0..n {
        let want = sample::<V::Chart, V::Links, f32>(&chart, uv[2 * i], uv[2 * i + 1]);
        let got = (
            read_ic(&ic, i),
            untag(tags[i]),
            read_descriptor(&descriptors, i),
        );
        assert!(
            got.0 == want.ic && got.1 == Ok(want.degenerate) && got.2 == want.descriptor,
            "sample {i}: the entry point's decode is not the chart's"
        );
    }
}

/// The ternary plot as the kernel holds it.
fn stage_ternary() -> MassSimplex<f32> {
    ternary::<f32>(0.4)
}

#[test]
fn per_pixel_mass_stage_entry_points() {
    check_stage::<MassSimplexDefaults>(mass_simplex_defaults, stage_ternary());
    check_stage::<BurrauDefaults>(
        burrau_defaults,
        Burrau {
            nu_range: [0.1, 0.8],
            k_max: 1.5,
            gamma_k: 2.0,
        },
    );
    check_stage::<EuclidPlaneDefaults>(
        euclid_plane_defaults,
        EuclidPlane {
            m_range: [2.0, 16.0],
            nu_range: [0.2, 0.7],
        },
    );
    check_stage::<MixedMu1AlphaDefaults>(
        mixed_mu1_alpha_defaults,
        Mixed::new(at(FROZEN), [-1.5, 1.0], [0.2, 1.3]),
    );
    assert_eq!(
        [
            MassSimplexDefaults::ENTRY,
            BurrauDefaults::ENTRY,
            EuclidPlaneDefaults::ENTRY,
            MixedMu1AlphaDefaults::ENTRY,
        ],
        [
            "stage_decode_mass_simplex_defaults",
            "stage_decode_burrau_defaults",
            "stage_decode_euclid_plane_defaults",
            "stage_decode_mixed_mu1_alpha_defaults",
        ]
    );
}

negative_control!(
    per_pixel_mass_stage_entry_points,
    "the latent chart's entry point on the ternary plot's uniform is not the chart's decode",
    expected = "the entry point's decode is not the chart's",
    check_stage::<MassSimplexDefaults>(kernel::stage::decode::latent_defaults, stage_ternary())
);
