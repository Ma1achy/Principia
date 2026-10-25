//! QA tests for TASK-M2-43, written from the requirements it closes (REQ-CHART-081, REQ-CHART-082's M2 half under
//! R-579, REQ-ENC-034) and their sources (R-544; chart_reference §3, §3.1, §3.3; chart_decoder_contract Parts 1, 2, 5;
//! inverse_encode_contract Parts 4, 5, 6), not from the implementation.
//!
//! Every expected value comes from an independent construction: [`reference_n`], chart_reference §3.1's map written
//! from the text on §0.2's mass-weighted Jacobi vectors; [`sphere_point`] and [`exp_point`], §3.3's two chart maps
//! written from the text; [`collision`], a binary-collision landmark placed by §3.1's map on a collinear configuration
//! with the pair together (§3.4's placement); [`jacobi_momenta`], §0.3's Jacobi-to-particle map inverted by hand; [`pi`],
//! the mirror through the `x`-axis, `y → −y` on every `rᵢ` and `pᵢ`.
//!
//! - REQ-CHART-081 (`shape_sphere_signed_decode_qa`): over random masses, held momenta, `I` and `φ_f`, at pixels of both
//!   hemispheres, the poles and the equator included, under both chart maps: the decoded state's shape is the signed
//!   `n(θ, φ)` within REQ-CHART-053's bound; it is canonical in CoM, boost, scale and rotation (`Σmᵢrᵢ = 0`, `Σpᵢ = 0`,
//!   `I = 1`, `ρ` on `+x`) with `λ̃_y` of `n₂`'s sign; its masses are `m_fixed`; its Jacobi momenta are `D_mom`'s
//!   `p_fixed` at `(θ, φ)` and at `(θ, π − φ)`. At f32 the pole rows decode to the pole's own sign, `L⁻` at `t = 0`
//!   (a sign, no tolerance). Controls: `C`'s mirror step run on the decode, a held momentum mirrored below the equator,
//!   and `Φ`'s state handed over without `C`, each fail.
//! - REQ-CHART-082, M2 half (`shape_sphere_mirror_test_qa`): the held coordinates classified (released from rest, and
//!   `p_fixed` with zero `y`-components, mirror-equivariant; a held `p_λ,y ≠ 0` or `p_ρ,y ≠ 0`, "does not apply"),
//!   the case said; `Π·x`, the kernel's `Π` (`kernel::canon::reflect`) on each decoded state, every `rᵢ` and `pᵢ`
//!   reflected bit for bit (`to_bits`), `L_z` negated by value and the shape's `w` negated bitwise where nonzero; in the
//!   equivariant case, each pixel pair's shapes mirror images within REQ-CHART-053's bound at every pixel, and its
//!   states within `ε_phys` in `‖·‖_phys` on the pixels farther than [`DELTA`] from each binary-collision landmark.
//!   Controls: R-141's gauge left in, a momentum rule turning `p_x` below the equator, the symmetry asserted where it
//!   does not apply, the state comparison at `BC₀₁`'s pixel `(½, ½)` of the odd grid, and a configuration-only `Π` on a
//!   state with `p_y ≠ 0`, each fail. The run arms are REQ-CHART-085's (R-579).
//! - REQ-ENC-034 (`encode_shape_sphere_signed_qa`): a random state and its mirror image are placed at their own signed
//!   shapes, `n₂` of opposite signs, `n(u′, v′)` the mirror of `n(u, v)`; decode then lookup returns the pixel on both
//!   hemispheres, both maps; T1 under the mirror holds bitwise for `z` and for the other charts' coordinates; a
//!   lower-hemisphere state with `p_y ≠ 0`, the slice's, under a proper gauge, lands on the chart, decodes back within
//!   `ε_phys` with `L_z`'s sign kept, and its mirror goes to latent `z` unless the slice holds its momenta. Controls: a
//!   lookup leaving the mirror in, T1 on the shape sphere's coordinates, and a landing against the mirrored `C(x)`,
//!   each fail.
//!
//! - REQ-ENC-004's amended T2 on the shape sphere (`t2_per_chart_shape_sphere_qa`): on a grid of both hemispheres,
//!   `D(E(x))`, the free decode of `x`'s `z`, is `x` within `ε_phys` above the equator and `Π·x` below it, the mirror
//!   image, so T2 is taken against `C(x)` there (R-544; inverse_encode_contract Part 1's T2). Control: T2 against `x`
//!   on the lower hemisphere fails.
//!
//! **Tolerances.** Shapes against REQ-CHART-053's `shape_roundtrip_bound`; states in REQ-ENC-024's `ε_phys` under
//! REQ-ENC-029's `‖·‖_phys` ([`phys`]); the canonical section's identities and the held momenta to [`CANON_TOL`], a few
//! roundings of the turn and scale relative to the state's size (applied per R-369, a test decision); bit-level claims
//! bitwise.
//!
//! **Domain** (pitfalls §10): the decode and encode on states held fixed; no trajectory.
//!
//! Each test has registered negative controls (R-176).

#[cfg(feature = "controls")]
use kernel::canon::canonicalise;
use kernel::canon::reflect;
use kernel::chart::mixed::{Beta, PLambdaY};
use kernel::chart::{Burrau, Chart, Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere};
use kernel::decode::{self, mass, momentum, Ic};
use kernel::encode::by_kind::{encode_into, ByKind, Coord, Landing};
use kernel::encode::{encode, Encoded};
use kernel::generated::links::Defaults;
use kernel::shape::shape;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::uniform;
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};
use validation::SHAPE_ROUNDTRIP_BOUND;

const PI: f64 = std::f64::consts::PI;
const EPS: f64 = f64::EPSILON;

/// REQ-CHART-053's `shape_roundtrip_bound` (R-181), provisional until M2.
const ROUNDTRIP: f64 = SHAPE_ROUNDTRIP_BOUND.number();

/// The canonical section's identities and the held momenta: the decode's turn, scale and `C`'s undoing of them, a few
/// roundings each, relative to the state's size (applied per R-369, a test decision).
const CANON_TOL: f64 = 64.0 * EPS;

type Pos = [[f64; 2]; 3];

// ── Independent constructions ───────────────────────────────────────────────────────────────────────────────────────

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn norm3(a: [f64; 3]) -> [f64; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    a.map(|x| x / l)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// chart_reference §3.1 from the text, on §0.2's mass-weighted Jacobi vectors at any total mass `M`:
/// `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁/M`, `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`; `a = ‖ρ̃‖²`, `b = ‖λ̃‖²`,
/// `p = ρ̃·λ̃`, `q = ρ̃ₓλ̃_y − ρ̃_yλ̃ₓ`, `I = a + b`, `n = ((a − b)/I, 2p/I, 2q/I)`.
fn reference_n(r: Pos, m: [f64; 3]) -> [f64; 3] {
    let total = m[0] + m[1] + m[2];
    let m01 = m[0] + m[1];
    let (srho, slam) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01 / total).sqrt());
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let lam = [r[2][0] - c[0], r[2][1] - c[1]];
    let (rt, lt) = (
        [srho * rho[0], srho * rho[1]],
        [slam * lam[0], slam * lam[1]],
    );
    let a = rt[0] * rt[0] + rt[1] * rt[1];
    let b = lt[0] * lt[0] + lt[1] * lt[1];
    let i = a + b;
    [
        (a - b) / i,
        2.0 * (rt[0] * lt[0] + rt[1] * lt[1]) / i,
        2.0 * (rt[0] * lt[1] - rt[1] * lt[0]) / i,
    ]
}

/// §3.3's spherical map from the text: `θ = 2πs`, `φ = π(1 − t)`, `n = (sin φ cos θ, sin φ sin θ, cos φ)`.
fn sphere_point(s: f64, t: f64) -> [f64; 3] {
    let (theta, phi) = (2.0 * PI * s, PI * (1.0 - t));
    [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]
}

/// §3.3's exponential map from the text: `d = (2u − 1)·s·e₁ + (2v − 1)·s·e₂`, `n = cos‖d‖·n₀ + sin‖d‖·d/‖d‖`.
fn exp_point(c: &ShapeSphere<f64>, u: f64, v: f64) -> [f64; 3] {
    let (a, b) = ((2.0 * u - 1.0) * c.scale, (2.0 * v - 1.0) * c.scale);
    let d = [0, 1, 2].map(|k| a * c.e1[k] + b * c.e2[k]);
    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if r == 0.0 {
        return c.n0;
    }
    [0, 1, 2].map(|k| r.cos() * c.n0[k] + r.sin() * d[k] / r)
}

/// The chart's point at `(u, v)` by the test's own maps.
fn point(c: &ShapeSphere<f64>, u: f64, v: f64) -> [f64; 3] {
    match c.map {
        ShapeMap::Spherical => sphere_point(u, v),
        ShapeMap::Exponential => exp_point(c, u, v),
    }
}

/// The landmark where bodies `i` and `j` collide (§3.4's placement): §3.1's map on the collinear configuration with the
/// pair at the origin and the third body at `(1, 0)`.
fn collision(m: [f64; 3], i: usize, j: usize) -> [f64; 3] {
    let mut r = [[1.0, 0.0]; 3];
    r[i] = [0.0, 0.0];
    r[j] = [0.0, 0.0];
    reference_n(r, m)
}

/// `Π`, written here: `y → −y` on every `rᵢ` and every `pᵢ`, the masses kept.
fn pi(x: &Ic<f64>) -> Ic<f64> {
    Ic {
        m: x.m,
        r: x.r.map(|v| [v[0], -v[1]]),
        p: x.p.map(|v| [v[0], -v[1]]),
    }
}

/// `L_z = Σ rᵢ × pᵢ`.
fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// §0.3's Jacobi-to-particle map inverted: `p₂ = p_λ` and `p₁ = p_ρ − (m₁/M₀₁)p_λ`, so `p_ρ = p₁ + (m₁/M₀₁)p₂`.
fn jacobi_momenta(x: &Ic<f64>) -> [[f64; 2]; 2] {
    let w1 = x.m[1] / (x.m[0] + x.m[1]);
    [
        [x.p[1][0] + w1 * x.p[2][0], x.p[1][1] + w1 * x.p[2][1]],
        x.p[2],
    ]
}

/// The held momentum's Jacobi components, `D_mom` on the chart's `z_mom` under its masses (chart_reference §3, the
/// held values).
fn held(c: &ShapeSphere<f64>) -> [[f64; 2]; 2] {
    let m = mass::decode::<Defaults, f64>(c.z_mass);
    let h = momentum::decode::<Defaults, f64>(c.z_mom, &m);
    [h.p_rho, h.p_lambda]
}

/// A random shape-sphere chart under `map`: random held masses and momenta, `I ∈ [¼, 4]`, `φ_f ∈ [−π, π]`, and for the
/// exponential map a random orthonormal frame and extent.
fn random_chart(rng: &mut SplitMix, map: ShapeMap) -> ShapeSphere<f64> {
    let n0 = norm3([0, 1, 2].map(|_| uniform(rng, -1.0, 1.0)));
    let e1 = norm3(cross(n0, [0, 1, 2].map(|_| uniform(rng, -1.0, 1.0))));
    ShapeSphere {
        map,
        n0,
        e1,
        e2: cross(n0, e1),
        scale: uniform(rng, 0.2, 2.5),
        inertia: uniform(rng, 0.25, 4.0),
        phase: uniform(rng, -PI, PI),
        z_mass: [uniform(rng, -3.0, 3.0), uniform(rng, -3.0, 3.0)],
        z_mom: [0, 1, 2, 3].map(|_| uniform(rng, -2.0, 2.0)),
    }
}

/// The decode's `ic`, the state handed to the integrator: the kernel's, or a control's.
type DecodeFn = fn(&ShapeSphere<f64>, f64, f64) -> Ic<f64>;

fn kernel_decode(c: &ShapeSphere<f64>, u: f64, v: f64) -> Ic<f64> {
    c.decode::<Defaults, f64>(u, v).ic
}

/// `Φ`'s state, the decode's blocks joined before any canonicalisation (R-178).
#[cfg(feature = "controls")]
fn phi_state(c: &ShapeSphere<f64>, u: f64, v: f64) -> Ic<f64> {
    let d = c.decode::<Defaults, f64>(u, v);
    Ic {
        m: d.masses.m,
        r: d.config.r,
        p: d.momentum.p,
    }
}

// ── REQ-CHART-081: the signed decode ───────────────────────────────────────────────────────────────────────────────

/// The decode on random charts of both maps, at random pixels and, for the spherical map, every fifth draw on a pole
/// row (`t = 0`, `t = 1`) or the equator (`t = ½`), with `BC₀₁`'s pixel `(½, ½)` among them: the shape is the signed
/// `n(u, v)`; the state canonical (CoM at the origin, `Σp = 0`, `I = 1`, `ρ_y = 0` relative to the size, `ρₓ ≥ 0`); `λ̃_y`
/// of `n₂`'s sign where `|n₂|` is above a few roundings; the masses `m_fixed`; the Jacobi momenta `p_fixed` here and,
/// on the spherical map, at `(θ, π − φ)`. Returns the worst shape distance.
fn check_signed(decode: DecodeFn) -> f64 {
    let mut rng = SplitMix::new(0x4381);
    let (mut worst, mut lower, mut poles, mut equator) = (0.0f64, 0u32, 0u32, 0u32);
    for k in 0..3000u32 {
        let map = if k % 4 == 3 {
            ShapeMap::Exponential
        } else {
            ShapeMap::Spherical
        };
        let chart = random_chart(&mut rng, map);
        let (mut s, mut t) = (uniform(&mut rng, 0.0, 1.0), uniform(&mut rng, 0.0, 1.0));
        if map == ShapeMap::Spherical {
            match k % 20 {
                0 => t = 0.0,
                4 => t = 1.0,
                8 => t = 0.5,
                12 => (s, t) = (0.5, 0.5),
                16 => (s, t) = (0.5, 0.0),
                _ => {}
            }
        }
        let n = point(&chart, s, t);
        let x = decode(&chart, s, t);
        assert!(
            x.r.iter()
                .chain(x.p.iter())
                .flatten()
                .all(|v| v.is_finite()),
            "the decode at ({s}, {t}) is not finite: {x:?}"
        );
        let got = reference_n(x.r, x.m);
        let d = dist(got, n);
        assert!(
            d <= ROUNDTRIP,
            "{map:?} at ({s}, {t}): the decoded state's shape is {got:?}, not the signed n {n:?} ({d:e} apart)"
        );
        worst = worst.max(d);
        // Canonical in CoM, boost, scale and rotation.
        let size = (0..3)
            .map(|i| x.m[i] * x.r[i][0].hypot(x.r[i][1]) + x.p[i][0].hypot(x.p[i][1]))
            .sum::<f64>();
        let com = [0, 1].map(|j| (0..3).map(|i| x.m[i] * x.r[i][j]).sum::<f64>());
        let total = [0, 1].map(|j| (0..3).map(|i| x.p[i][j]).sum::<f64>());
        let inertia = (0..3)
            .map(|i| x.m[i] * ((x.r[i][0] - com[0]).powi(2) + (x.r[i][1] - com[1]).powi(2)))
            .sum::<f64>();
        let rho = [x.r[1][0] - x.r[0][0], x.r[1][1] - x.r[0][1]];
        let misses = [
            com[0].abs().max(com[1].abs()) / size,
            total[0].abs().max(total[1].abs()) / size,
            (inertia - 1.0).abs(),
            rho[1].abs() / size,
        ];
        assert!(
            misses.iter().all(|&e| e <= CANON_TOL) && rho[0] >= 0.0,
            "{map:?} at ({s}, {t}), chart I {}: the decoded state is not canonical: CoM, Σp, I − 1, ρ_y {misses:?}, \
             ρ {rho:?}",
            chart.inertia
        );
        let m01 = x.m[0] + x.m[1];
        let lambda_y = x.r[2][1] - (x.m[0] * x.r[0][1] + x.m[1] * x.r[1][1]) / m01;
        assert!(
            n[2].abs() <= CANON_TOL || (lambda_y < 0.0) == (n[2] < 0.0),
            "{map:?} at ({s}, {t}): λ̃_y = {lambda_y:e} against n₂ = {:e}, its sign not kept",
            n[2]
        );
        let m = mass::decode::<Defaults, f64>(chart.z_mass).m;
        assert!(
            x.m == m,
            "the decoded masses {:?} are not m_fixed {m:?}",
            x.m
        );
        let fixed = held(&chart);
        let scale = 1.0 + fixed.iter().flatten().fold(0.0f64, |a, b| a.max(b.abs()));
        let gap = |j: [[f64; 2]; 2]| {
            (0..2)
                .flat_map(|a| (0..2).map(move |b| (a, b)))
                .map(|(a, b)| (j[a][b] - fixed[a][b]).abs())
                .fold(0.0f64, f64::max)
                / scale
        };
        let here = jacobi_momenta(&x);
        let mut there = here;
        if map == ShapeMap::Spherical {
            there = jacobi_momenta(&decode(&chart, s, 1.0 - t));
        }
        assert!(
            gap(here) <= CANON_TOL && gap(there) <= CANON_TOL,
            "{map:?} at ({s}, {t}) and ({s}, {}): the held Jacobi momenta are {here:?} and {there:?}, not p_fixed \
             {fixed:?}",
            1.0 - t
        );
        lower += u32::from(n[2] < -0.01);
        poles += u32::from(map == ShapeMap::Spherical && (t == 0.0 || t == 1.0));
        equator += u32::from(map == ShapeMap::Spherical && t == 0.5);
    }
    assert!(
        lower > 1000 && poles > 400 && equator > 200,
        "the draws miss a regime: {lower} lower-hemisphere, {poles} on a pole row, {equator} on the equator"
    );
    worst
}

/// At f32, on the CPU, every pixel of the spherical map's pole rows decodes to that pole's side: `w < 0` at `t = 0`
/// (`L⁻`), `w > 0` at `t = 1`, `s` on a grid with `½` among it, where `sin θ` rounds below 0 (a sign, no tolerance).
fn check_poles_f32() {
    let mut rng = SplitMix::new(0x4382);
    for _ in 0..16 {
        let c = random_chart(&mut rng, ShapeMap::Spherical);
        let c32 = ShapeSphere::<f32> {
            map: ShapeMap::Spherical,
            n0: c.n0.map(|x| x as f32),
            e1: c.e1.map(|x| x as f32),
            e2: c.e2.map(|x| x as f32),
            scale: c.scale as f32,
            inertia: c.inertia as f32,
            phase: c.phase as f32,
            z_mass: c.z_mass.map(|x| x as f32),
            z_mom: c.z_mom.map(|x| x as f32),
        };
        for i in 0..=16u8 {
            let s = f32::from(i) / 16.0;
            for (t, sign) in [(0.0f32, -1.0f32), (1.0, 1.0)] {
                let x = c32.decode::<Defaults, f32>(s, t).ic;
                let w = shape(x.r, x.m)[2];
                assert!(
                    w * sign > 0.5,
                    "at f32 the pole row ({s}, {t}) decodes to w = {w}, not the pole's side {sign}"
                );
            }
        }
    }
}

#[test]
fn shape_sphere_signed_decode_qa() {
    let worst = check_signed(kernel_decode);
    check_poles_f32();
    eprintln!(
        "shape_sphere_signed_decode_qa: worst ‖n(decode) − n(θ, φ)‖ {worst:.3e} ({:.1} ε) over 3000 pixels of both \
         maps and hemispheres, poles and equator included",
        worst / EPS
    );
}

negative_control!(
    shape_sphere_signed_decode_qa,
    "R-141's gauge left in, C's mirror step run on the decode, lands every lower pixel at (θ, π − φ), so it must fail",
    expected = "not the signed n",
    {
        check_signed(|c, u, v| canonicalise(&kernel_decode(c, u, v)).ic);
    }
);

mod signed_held_mirrored {
    use super::*;
    negative_control!(
        shape_sphere_signed_decode_qa,
        "a held momentum mirrored below the equator (p_y negated in the body frame where n₂ < 0) differs from the upper \
         twin's, so it must fail",
        expected = "held Jacobi momenta",
        {
            check_signed(|c, u, v| {
                let x = kernel_decode(c, u, v);
                if point(c, u, v)[2] < 0.0 {
                    Ic {
                        p: x.p.map(|q| [q[0], -q[1]]),
                        ..x
                    }
                } else {
                    x
                }
            });
        }
    );
}

mod signed_skipping_c {
    use super::*;
    negative_control!(
        shape_sphere_signed_decode_qa,
        "Φ's state handed over without C is at the chart's I and turned by φ_f, off the section, so it must fail",
        expected = "is not canonical",
        {
            check_signed(phi_state);
        }
    );
}

// ── REQ-CHART-082, M2 half (R-579): the mirror test ────────────────────────────────────────────────────────────────

/// The case the mirror test is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    /// Every held coordinate is its own mirror image: the masses, and `p_fixed` with zero `y`-components.
    Equivariant,
    /// A held momentum has a `y`-component: the hemispheres are different systems.
    DoesNotApply,
}

/// Classifies what the chart holds fixed (R-544 item 5): the masses are mirror-invariant, so `p_fixed` decides.
fn classify(c: &ShapeSphere<f64>) -> Case {
    let h = held(c);
    if h[0][1] == 0.0 && h[1][1] == 0.0 {
        Case::Equivariant
    } else {
        Case::DoesNotApply
    }
}

/// The odd grid, `GRID × GRID` pixels `(i, j)/(GRID − 1)`: `(½, ½)`, `BC₀₁`'s pixel, its own pair, is on it, and
/// `1 − t` is a grid value exactly.
const GRID: u32 = 33;

/// The state comparison's chordal distance from each binary-collision landmark. At `I = 1` near `BC₀₁`,
/// `‖ρ̃‖² = (1 + n₀)/2` and the chordal distance `d` has `d² = 2(1 + n₀)`, so `‖ρ̃‖ = d/2`; `λ̃`'s direction is
/// conditioned as `ulp/‖ρ̃‖` (§3.2), so a rounding-level difference in `n̂` turns the state by up to `2·ulp/d`. At
/// `δ = 1/8` that is `16 ulp ≤ ε_phys = 64 ulp` (REQ-ENC-024): `ulp/δ`-scale `≤ ε_phys` is `δ`'s reason. Applied to all
/// three landmarks.
const DELTA: f64 = 1.0 / 8.0;

/// `Π` under test: the kernel's, or a control's.
type MirrorFn = fn(&Ic<f64>) -> Ic<f64>;

fn kernel_pi(x: &Ic<f64>) -> Ic<f64> {
    reflect(x, true)
}

/// The mirror test on `chart` (REQ-CHART-082's M2 half): classifies the chart and reports the case. On every decoded
/// state of the grid, `mirror(x)` against [`pi`]: the masses and every `rᵢ`, `pᵢ` bit for bit, `L_z` negated by value,
/// the shape's `n₀`, `n₁` bitwise and `w` negated bitwise where nonzero, by value at zero; Π's own property, checked in
/// both cases. In the equivariant case, or with `assert_anyway`, each pair's shapes `n(Π·decode(θ, φ))` and
/// `n(decode(θ, π − φ))` within REQ-CHART-053's bound, and the pair's states within `ε_phys` in `‖·‖_phys` where `n̂`
/// is farther than `delta` from each landmark. Returns the case and the worst state residual in `ε`.
fn mirror_test(
    chart: &ShapeSphere<f64>,
    decode: DecodeFn,
    mirror: MirrorFn,
    delta: f64,
    assert_anyway: bool,
) -> (Case, f64) {
    let case = classify(chart);
    let m = mass::decode::<Defaults, f64>(chart.z_mass).m;
    let landmarks = [collision(m, 0, 1), collision(m, 1, 2), collision(m, 2, 0)];
    let eps = eps_phys::<f64>();
    let last = f64::from(GRID - 1);
    let mut worst = 0.0f64;
    for i in 0..GRID {
        for j in 0..GRID {
            let (s, t) = (f64::from(i) / last, f64::from(j) / last);
            let x = decode(chart, s, t);
            let y = mirror(&x);
            let want = pi(&x);
            let bits = |a: &Ic<f64>| {
                (
                    a.m.map(f64::to_bits),
                    a.r.map(|v| v.map(f64::to_bits)),
                    a.p.map(|v| v.map(f64::to_bits)),
                )
            };
            let (nx, ny) = (shape(x.r, x.m), shape(y.r, y.m));
            let w_ok = if nx[2] == 0.0 {
                ny[2] == 0.0
            } else {
                ny[2].to_bits() == (-nx[2]).to_bits()
            };
            assert!(
                bits(&y) == bits(&want)
                    && lz(&y) == -lz(&x)
                    && ny[0].to_bits() == nx[0].to_bits()
                    && ny[1].to_bits() == nx[1].to_bits()
                    && w_ok,
                "Π·x is not x's mirror at ({s}, {t}): x {x:?}, Π·x {y:?}"
            );
            if case == Case::DoesNotApply && !assert_anyway {
                continue;
            }
            let twin = decode(chart, s, 1.0 - t);
            let gap = dist(shape(y.r, y.m), shape(twin.r, twin.m));
            assert!(
                gap <= ROUNDTRIP,
                "the pair ({s}, {t}), ({s}, {}): shapes are not mirror images, {gap:e} apart",
                1.0 - t
            );
            let n = sphere_point(s, t);
            if landmarks.iter().all(|&b| dist(n, b) > delta) {
                let r = phys(&twin, &y);
                assert!(
                    r <= eps,
                    "the pair ({s}, {t}), ({s}, {}): states differ by {r:e} in ‖·‖_phys, beyond ε_phys {eps:e}",
                    1.0 - t
                );
                worst = worst.max(r / EPS);
            }
        }
    }
    (case, worst)
}

/// A spherical chart at random masses, `I` and `φ_f`, holding `z_mom`.
fn holding(rng: &mut SplitMix, z_mom: [f64; 4]) -> ShapeSphere<f64> {
    ShapeSphere {
        z_mom,
        ..random_chart(rng, ShapeMap::Spherical)
    }
}

/// Released from rest.
const REST: [f64; 4] = [0.0; 4];
/// `p_ρ` and `p_λ` along `x` only.
const ALONG_X: [f64; 4] = [0.7, 0.0, -0.5, 0.0];
/// A held `p_λ,y ≠ 0`.
const P_LAMBDA_Y: [f64; 4] = [0.2, 0.0, 0.3, 0.6];
/// A held `p_ρ,y ≠ 0`.
const P_RHO_Y: [f64; 4] = [0.0, -0.4, 0.1, 0.0];

#[test]
fn shape_sphere_mirror_test_qa() {
    let mut rng = SplitMix::new(0x4383);
    let mut worst = 0.0f64;
    for _ in 0..3 {
        for (z, want) in [
            (REST, Case::Equivariant),
            (ALONG_X, Case::Equivariant),
            (P_LAMBDA_Y, Case::DoesNotApply),
            (P_RHO_Y, Case::DoesNotApply),
        ] {
            let chart = holding(&mut rng, z);
            let (case, w) = mirror_test(&chart, kernel_decode, kernel_pi, DELTA, false);
            assert_eq!(case, want, "the case of the held z_mom {z:?}");
            let report = match case {
                Case::Equivariant => "mirror-equivariant: pairs checked",
                Case::DoesNotApply => "does not apply: no symmetry asserted",
            };
            eprintln!("shape_sphere_mirror_test_qa: z_mom {z:?}: {report}");
            worst = worst.max(w);
        }
    }
    eprintln!(
        "shape_sphere_mirror_test_qa: worst pair state residual {worst:.2} ε against ε_phys {} ε, δ = {DELTA}",
        eps_phys::<f64>() / EPS
    );
}

negative_control!(
    shape_sphere_mirror_test_qa,
    "R-141's gauge left in folds each pair onto one shape, equal, not mirror images, so it must fail",
    expected = "shapes are not mirror images",
    {
        let chart = holding(&mut SplitMix::new(11), ALONG_X);
        mirror_test(
            &chart,
            |c, u, v| canonicalise(&kernel_decode(c, u, v)).ic,
            kernel_pi,
            DELTA,
            false,
        );
    }
);

mod mirror_px_turned {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test_qa,
        "a momentum rule turning p_x below the equator makes the pair's states not mirror images, so it must fail",
        expected = "states differ",
        {
            let chart = holding(&mut SplitMix::new(12), ALONG_X);
            mirror_test(
                &chart,
                |c, u, v| {
                    let z = c.z_mom;
                    let c2 = ShapeSphere {
                        z_mom: if sphere_point(u, v)[2] < 0.0 {
                            [-z[0], z[1], -z[2], z[3]]
                        } else {
                            z
                        },
                        ..*c
                    };
                    kernel_decode(&c2, u, v)
                },
                kernel_pi,
                DELTA,
                false,
            );
        }
    );
}

mod mirror_asserted_anyway {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test_qa,
        "the symmetry asserted where a held p_λ,y ≠ 0 makes the hemispheres different systems fails on the pairs",
        expected = "states differ",
        {
            let chart = holding(&mut SplitMix::new(13), P_LAMBDA_Y);
            mirror_test(&chart, kernel_decode, kernel_pi, DELTA, true);
        }
    );
}

mod mirror_at_bc01 {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test_qa,
        "the state comparison at BC₀₁'s pixel (½, ½), where λ̃'s direction is the rounding's, must fail",
        expected = "states differ",
        {
            let chart = holding(&mut SplitMix::new(14), REST);
            mirror_test(&chart, kernel_decode, kernel_pi, 0.0, false);
        }
    );
}

mod mirror_configuration_only {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test_qa,
        "a configuration-only Π, positions reflected and momenta kept, on a state with p_y ≠ 0 fails Π's check",
        expected = "is not x's mirror",
        {
            let chart = holding(&mut SplitMix::new(15), P_LAMBDA_Y);
            mirror_test(
                &chart,
                kernel_decode,
                |x| Ic {
                    p: x.p,
                    ..reflect(x, true)
                },
                DELTA,
                false,
            );
        }
    );
}

// ── REQ-ENC-034: the signed lookup, T1, and the landing ────────────────────────────────────────────────────────────

/// A generic state: masses in `[0.2, 1]` normalised, positions and momenta in `[−1, 1]²`.
fn generic(rng: &mut SplitMix) -> Ic<f64> {
    let m = [0, 1, 2].map(|_| uniform(rng, 0.2, 1.0));
    let total = m[0] + m[1] + m[2];
    let mut v = || [uniform(rng, -1.0, 1.0), uniform(rng, -1.0, 1.0)];
    Ic {
        m: m.map(|x| x / total),
        r: [v(), v(), v()],
        p: [v(), v(), v()],
    }
}

/// A lookup onto the shape sphere: the kernel's `coords` of `E(x)`, or a control's.
type LookupFn = fn(&ShapeSphere<f64>, &Ic<f64>) -> (f64, f64);

fn kernel_lookup(c: &ShapeSphere<f64>, x: &Ic<f64>) -> (f64, f64) {
    match c.coords::<Defaults, f64>(&encode::<Defaults, f64>(x)) {
        [Coord::At(u), Coord::At(v)] => (u, v),
        other => panic!("the shape sphere placed no point: {other:?}"),
    }
}

/// The spherical map inverted from the text: `θ = atan2(n₁, n₀)` on `[0, 2π)`, `φ = acos n₂`, `s = θ/2π`,
/// `t = 1 − φ/π`.
#[cfg(feature = "controls")]
fn sphere_inverse(n: [f64; 3]) -> (f64, f64) {
    let theta = n[1].atan2(n[0]).rem_euclid(2.0 * PI);
    (theta / (2.0 * PI), 1.0 - n[2].clamp(-1.0, 1.0).acos() / PI)
}

/// The spherical chart of the lookup tests: held values from `z_mom`, `I = 1.7`, `φ_f = 0.6`.
fn lookup_chart(z_mom: [f64; 4]) -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.7,
        phase: 0.6,
        z_mass: [0.4, -0.7],
        z_mom,
    }
}

/// An exponential chart whose cap straddles the equator, about a point of it.
fn straddling(z_mom: [f64; 4]) -> ShapeSphere<f64> {
    let n0 = norm3([0.3, -0.9, 0.05]);
    let e1 = norm3(cross(n0, [0.0, 0.0, 1.0]));
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1,
        e2: cross(n0, e1),
        scale: 1.1,
        ..lookup_chart(z_mom)
    }
}

/// The held momentum of the landing tests, `p_y ≠ 0` in both Jacobi momenta.
const SLICE_MOM: [f64; 4] = [0.3, -0.5, -0.2, 0.4];

/// `lookup` on random states and their mirror images: each placed at its own signed shape within REQ-CHART-053's
/// bound, `n₂` of opposite signs, `n(u′, v′)` the mirror `(n₀, n₁, −n₂)` of `n(u, v)`.
fn check_lookup(lookup: LookupFn) -> u32 {
    let chart = lookup_chart(SLICE_MOM);
    let mut rng = SplitMix::new(0x4384);
    let mut lower = 0;
    for _ in 0..600 {
        let x = generic(&mut rng);
        let y = pi(&x);
        let (nx, ny) = (reference_n(x.r, x.m), reference_n(y.r, y.m));
        let (u, v) = lookup(&chart, &x);
        let (um, vm) = lookup(&chart, &y);
        let (px, py) = (sphere_point(u, v), sphere_point(um, vm));
        assert!(
            dist(px, nx) <= ROUNDTRIP && dist(py, ny) <= ROUNDTRIP,
            "x is placed at {px:?}, its shape {nx:?}; Π·x at {py:?}, its shape {ny:?}: not the state's signed shape"
        );
        assert!(
            dist(py, [px[0], px[1], -px[2]]) <= ROUNDTRIP
                && (nx[2].abs() <= CANON_TOL || (px[2] < 0.0) != (py[2] < 0.0)),
            "x at ({u}, {v}) and Π·x at ({um}, {vm}) are not at (θ, φ) and (θ, π − φ)"
        );
        lower += u32::from(nx[2] < -0.01);
    }
    assert!(lower > 200, "{lower} of 600 states below the equator");
    lower
}

/// Decode then lookup on `chart`: every pixel of an interior grid, both hemispheres, returns a point within
/// REQ-CHART-053's bound of itself, on its own side of the equator where `|n₂|` is above a few roundings. Returns the
/// lower-hemisphere pixels visited.
fn check_decode_lookup(chart: &ShapeSphere<f64>) -> u32 {
    let mut lower = 0;
    for i in 1..24 {
        for j in 1..24 {
            let (s, t) = (f64::from(i) / 24.0, f64::from(j) / 24.0);
            let want = point(chart, s, t);
            let x = chart.decode::<Defaults, f64>(s, t).ic;
            let (u, v) = kernel_lookup(chart, &x);
            let got = point(chart, u, v);
            assert!(
                dist(got, want) <= ROUNDTRIP
                    && (want[2].abs() <= CANON_TOL || (got[2] < 0.0) == (want[2] < 0.0)),
                "{:?}: decode then lookup at ({s}, {t}) returns ({u}, {v}), {got:?} against {want:?}",
                chart.map
            );
            lower += u32::from(want[2] < -0.01);
        }
    }
    lower
}

/// The bits of a chart point.
fn coord_bits(c: [Coord<f64>; 2]) -> [Option<u64>; 2] {
    c.map(|k| match k {
        Coord::At(x) => Some(x.to_bits()),
        _ => None,
    })
}

/// T1 under the mirror on one chart: `E(x)` and `E(Π·x)` give one chart point, bit for bit.
fn check_t1<C: ByKind<Param = f64>>(name: &str, chart: &C, xs: &[Ic<f64>]) {
    for x in xs {
        let (a, b): (Encoded<f64>, Encoded<f64>) =
            (encode::<Defaults, f64>(x), encode::<Defaults, f64>(&pi(x)));
        assert!(
            coord_bits(chart.coords::<Defaults, f64>(&a))
                == coord_bits(chart.coords::<Defaults, f64>(&b)),
            "T1 fails on {name}: x and Π·x are two chart points"
        );
    }
}

/// T1 for `z` and the other charts under the mirror, bitwise: `E(Π·x).z` is `E(x).z`, and each chart reads one point.
fn check_t1_others(xs: &[Ic<f64>]) {
    for x in xs {
        let (a, b) = (encode::<Defaults, f64>(x), encode::<Defaults, f64>(&pi(x)));
        assert!(
            a.z.z.map(f64::to_bits) == b.z.z.map(f64::to_bits),
            "T1 fails on z: {:?} against {:?}",
            a.z.z,
            b.z.z
        );
    }
    check_t1(
        "a latent chart",
        &Latent {
            z0: [0.1, -0.2, 0.3, 0.0, 0.2, -0.1, 0.4, -0.3],
            q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        },
        xs,
    );
    check_t1(
        "(β, p_λ,y)",
        &Mixed::<Beta, PLambdaY, f64>::new([0.2; 8], [-1.0, 1.0], [-0.5, 0.5]),
        xs,
    );
    check_t1(
        "the mass simplex",
        &MassSimplex {
            nu0: 0.3,
            eps_m: 1e-3,
        },
        xs,
    );
    check_t1(
        "(ν, K)",
        &Burrau {
            nu_range: [0.05, 0.4],
            k_max: 1.0,
            gamma_k: 2.0,
        },
        xs,
    );
}

/// A proper gauge, written here: turn by `a`, move by `b`, boost by `w` (`pᵢ += mᵢw`), no mirror.
fn proper(x: &Ic<f64>, a: f64, b: [f64; 2], w: [f64; 2]) -> Ic<f64> {
    let (sn, cs) = a.sin_cos();
    let turn = |v: [f64; 2]| [cs * v[0] - sn * v[1], sn * v[0] + cs * v[1]];
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let t = turn(v);
            [t[0] + b[0], t[1] + b[1]]
        }),
        p: [0, 1, 2].map(|i| {
            let t = turn(x.p[i]);
            [t[0] + x.m[i] * w[0], t[1] + x.m[i] * w[1]]
        }),
    }
}

/// Where a state lands on the shape sphere: the kernel's `encode_into` at `‖·‖_phys` and `ε_phys`, or a control's.
type LandFn = fn(&ShapeSphere<f64>, &Ic<f64>) -> Landing<f64>;

fn kernel_land(c: &ShapeSphere<f64>, x: &Ic<f64>) -> Landing<f64> {
    encode_into::<Defaults, f64, _, _>(c, x, phys, eps_phys::<f64>()).landing
}

/// The landing (inverse_encode_contract Part 6, step 4, on the shape sphere): a lower-hemisphere `x` with `p_y ≠ 0`,
/// the chart's own decode, its masses and unmirrored Jacobi momenta the slice's, as is and under a proper gauge, lands
/// on the chart at its pixel and decodes back to `x` within `ε_phys`, `L_z`'s sign kept; `Π·x`, the slice unchanged,
/// goes to latent `z`. On a slice whose momenta are their own mirror images, `Π·x` lands at `(θ, π − φ)`. Returns the
/// worst residual in `ε`.
fn check_landing(land: LandFn) -> f64 {
    let chart = lookup_chart(SLICE_MOM);
    let mut rng = SplitMix::new(0x4385);
    let mut worst = 0.0f64;
    for _ in 0..300 {
        let (s, t) = (uniform(&mut rng, 0.05, 0.95), uniform(&mut rng, 0.05, 0.45));
        let x = chart.decode::<Defaults, f64>(s, t).ic;
        assert!(
            sphere_point(s, t)[2] < 0.0 && x.p.iter().any(|q| q[1] != 0.0) && lz(&x) != 0.0,
            "({s}, {t}) is not a lower-hemisphere state with p_y ≠ 0"
        );
        let g = proper(
            &x,
            uniform(&mut rng, -PI, PI),
            [uniform(&mut rng, -3.0, 3.0), uniform(&mut rng, -3.0, 3.0)],
            [uniform(&mut rng, -2.0, 2.0), uniform(&mut rng, -2.0, 2.0)],
        );
        for y in [x, g] {
            let Landing::Chart { u, v } = land(&chart, &y) else {
                panic!("the lower-hemisphere state at ({s}, {t}) lands in latent z");
            };
            let back = chart.decode::<Defaults, f64>(u, v).ic;
            let d = phys(&x, &back);
            assert!(
                d <= eps_phys::<f64>() && dist(sphere_point(u, v), sphere_point(s, t)) <= ROUNDTRIP,
                "the state at ({s}, {t}) lands at ({u}, {v}) and decodes back {d:e} from itself"
            );
            assert!(
                (lz(&back) < 0.0) == (lz(&x) < 0.0),
                "L_z's sign at ({s}, {t}) is not kept: {} against {}",
                lz(&back),
                lz(&x)
            );
            worst = worst.max(d / EPS);
        }
        assert!(
            land(&chart, &pi(&x)) == Landing::Latent,
            "Π·x of the state at ({s}, {t}) lands on the chart, whose slice holds x's momenta, not Π·x's"
        );
    }
    // A slice holding momenta along x only, their own mirror images: Π·x is the state at (θ, π − φ) and lands there.
    let even = lookup_chart([0.3, 0.0, -0.2, 0.0]);
    for (s, t) in [(0.3, 0.2), (0.7, 0.35), (0.15, 0.1)] {
        let x = even.decode::<Defaults, f64>(s, t).ic;
        let Landing::Chart { u, v } = land(&even, &pi(&x)) else {
            panic!("Π·x of ({s}, {t}), the slice holding its momenta, lands in latent z");
        };
        assert!(
            dist(sphere_point(u, v), sphere_point(s, 1.0 - t)) <= ROUNDTRIP,
            "Π·x of ({s}, {t}) lands at ({u}, {v}), not (θ, π − φ)"
        );
    }
    worst
}

#[test]
fn encode_shape_sphere_signed_qa() {
    let lower = check_lookup(kernel_lookup);
    let spherical = check_decode_lookup(&lookup_chart(SLICE_MOM));
    let exponential = check_decode_lookup(&straddling(SLICE_MOM));
    assert!(
        spherical > 200 && exponential > 100,
        "decode then lookup visits {spherical} and {exponential} lower-hemisphere pixels"
    );
    let mut rng = SplitMix::new(0x4386);
    let xs: Vec<Ic<f64>> = (0..300).map(|_| generic(&mut rng)).collect();
    check_t1_others(&xs);
    let worst = check_landing(kernel_land);
    eprintln!(
        "encode_shape_sphere_signed_qa: {lower} lower-hemisphere states and their mirrors placed at their signed shapes; \
         {spherical} and {exponential} lower pixels return themselves; landing worst {worst:.2} ε (ε_phys {} ε)",
        eps_phys::<f64>() / EPS
    );
}

negative_control!(
    encode_shape_sphere_signed_qa,
    "a lookup reading the shape of C(x) without undoing the mirror lands every lower-hemisphere state on the upper",
    expected = "not the state's signed shape",
    {
        check_lookup(|_, x| {
            let c = canonicalise(x).ic;
            sphere_inverse(reference_n(c.r, c.m))
        });
    }
);

mod signed_t1_on_sphere {
    use super::*;
    negative_control!(
        encode_shape_sphere_signed_qa,
        "T1 applied to the shape sphere's coordinates, where x and Π·x are two points, must fail",
        expected = "T1 fails on the shape sphere",
        {
            let mut rng = SplitMix::new(0x4386);
            let xs: Vec<Ic<f64>> = (0..300).map(|_| generic(&mut rng)).collect();
            check_t1("the shape sphere", &lookup_chart(SLICE_MOM), &xs);
        }
    );
}

mod signed_landing_mirrored {
    use super::*;
    negative_control!(
        encode_shape_sphere_signed_qa,
        "a landing test against the mirrored C(x) sends every lower-hemisphere state to latent z, so it must fail",
        expected = "lands in latent z",
        {
            check_landing(|c, x| {
                let (u, v) = kernel_lookup(c, x);
                let d = phys(
                    &canonicalise(x).ic,
                    &c.decode::<Defaults, f64>(u, v).ic,
                );
                if d <= eps_phys::<f64>() {
                    Landing::Chart { u, v }
                } else {
                    Landing::Latent
                }
            });
        }
    );
}

// ── REQ-ENC-004 as R-544 amends it: T2 on the shape sphere ─────────────────────────────────────────────────────────

/// The state T2 compares `D(E(x))` with: `C(x)`'s physical content, `x` above the equator and [`pi`]`(x)` below, or a
/// control's.
type ReferenceFn = fn(&Ic<f64>, f64) -> Ic<f64>;

/// T2 on the spherical chart over the pixels `(i, j)/24` of both hemispheres, the poles' and the equator's rows left
/// out (on the equator `λ̃_y` is a rounding, in R-82's deadband, where `C` may or may not mirror): `D(E(x))` within
/// `ε_phys` of `reference(x, n₂)` in `‖·‖_phys`. Returns the lower-hemisphere pixels and the worst residual in `ε`.
fn check_t2_sphere(reference: ReferenceFn) -> (u32, f64) {
    let chart = lookup_chart(SLICE_MOM);
    let (mut lower, mut worst) = (0u32, 0.0f64);
    for i in 0..24 {
        for j in 1..24 {
            let (s, t) = (f64::from(i) / 24.0, f64::from(j) / 24.0);
            if j == 12 {
                continue;
            }
            let x = chart.decode::<Defaults, f64>(s, t).ic;
            let n2 = sphere_point(s, t)[2];
            let back = decode::decode::<Defaults, f64>(&encode::<Defaults, f64>(&x).z).ic;
            let r = phys(&reference(&x, n2), &back);
            assert!(
                r <= eps_phys::<f64>(),
                "T2 at ({s}, {t}), n₂ = {n2:.3}: D(E(x)) is {r:e} from the reference in ‖·‖_phys"
            );
            worst = worst.max(r / EPS);
            lower += u32::from(n2 < 0.0);
        }
    }
    (lower, worst)
}

#[test]
fn t2_per_chart_shape_sphere_qa() {
    let (lower, worst) = check_t2_sphere(|x, n2| if n2 < 0.0 { pi(x) } else { *x });
    assert!(lower > 200, "{lower} lower-hemisphere pixels");
    eprintln!(
        "t2_per_chart_shape_sphere_qa: D(E(x)) is x above the equator and Π·x below, worst {worst:.2} ε (ε_phys {} ε), \
         {lower} lower pixels",
        eps_phys::<f64>() / EPS
    );
}

negative_control!(
    t2_per_chart_shape_sphere_qa,
    "T2 against x itself on the lower hemisphere, where D(E(x)) is the mirror image, must fail",
    expected = "from the reference",
    {
        check_t2_sphere(|x, _| *x);
    }
);
