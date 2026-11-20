//! QA tests for TASK-M2-28, written from the requirements it closes (R-544, R-503, R-554), not from the code.
//!
//! - REQ-CHART-002 (`qa_m228_spherical_whole_sphere`): the spherical map is the whole sphere, every pixel its own
//!   system: on a grid over `[0,1]²` each pixel decodes to the system whose shape, read from the decoded positions by
//!   R-14's map (written out here), is R-14's point `(sin φ cos θ, sin φ sin θ, cos φ)` with `θ = 2πs`,
//!   `φ = π(1 − t)`, the lower pixels below the equator; mirror pixels `(θ, π − φ)` decode to two systems; no two
//!   pixels share one; `system_image` is bijective. Control: R-141's fold, a lower pixel read as its mirror, puts two
//!   pixels on one system, and fails.
//! - REQ-CHART-075 (`qa_m228_exp_cap_bijective`): on caps in either hemisphere, touching the equator and straddling
//!   it, about frames of every orientation and extents past `s√2 = π`, `system_image` is bijective, and the decoded
//!   shape at every pixel short of `r = π` is the exponential map's point `cos r·n₀ + (sin r/r)·d` (written out here),
//!   no two such pixels one system. Controls: R-503's per-pixel fold, and a fixed n = 2, each fail.
//! - REQ-CHART-075 (`qa_m228_exp_count`): on random grids symmetric under `w → −w` (`n₀` on the equator at a random
//!   longitude, `e₂ = ŵ`, odd `N`, every pixel centre more than `10⁻⁶` from `r = π`), the counted total is `N²` when
//!   `s√2 < π`, and the number of pixels with `r ≤ π`, enumerated here from `(u, v)` alone, when
//!   `s√2·(N − 1)/N > π`; on one-hemisphere caps, every pixel. Controls: R-503's fold (undercounts), a fixed n = 2,
//!   and a count including the pixels past `r = π`, each fail.
//! - REQ-CHART-078 (`qa_m228_antipode_once`): grids whose pixel centres land exactly on `r = π` (exactly representable
//!   offsets `±½` at `s = 2π`), about a pole, the other pole and a point of the equator: those pixels decode to `−n₀`
//!   to rounding, finite, and together count once, beside the pixels short of π counting one each and those past it
//!   none (chart_reference §3.3, "Pixels on `r = π`"). Controls: each antipode pixel counted on its own (overcounts)
//!   and the antipode read as the domain's end (undercounts), each fail.
//! - REQ-RENDER-026, REQ-CHART-047 (`qa_m228_hammer_equal_area`): Hammer's equal-area projection, as chart_reference
//!   §3.3 defines it: over a fine grid of display points, the share of the shown points whose chart point lies in a
//!   polar cap of angle `α`, or a band about the equator, is the share of solid angle `(1 − cos α)/2`, both
//!   hemispheres alike, and the shown region is the 2:1 ellipse, `π/4` of the square; the poles land at the top and
//!   bottom of the centre line. Control: the equirectangular display, whose cap share is `α/π`, fails.

use kernel::chart::shape_sphere::{Projection, ShapeMap, ShapeSphere};
use kernel::chart::{Chart, SystemImage};
use kernel::generated::links::Defaults;
use validation::negative_control;

const PI: f64 = std::f64::consts::PI;

// ── Independent formulas ─────────────────────────────────────────────────────────────────────────────────────────

/// R-14's shape-sphere point of the configuration `r` with masses `m` (chart_reference §0.2, §3.1), written out from
/// the docs: `ρ = r₁ − r₀`, `λ = r₂ − r₀₁`, `ρ̃ = √(m₀m₁/M₀₁)·ρ`, `λ̃ = √(m₂M₀₁)·λ` (M = 1), `n = (‖ρ̃‖² − ‖λ̃‖²,
/// 2ρ̃·λ̃, 2ρ̃∧λ̃)/I`.
fn shape_of(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    let m01 = m[0] + m[1];
    let a = (m[0] * m[1] / m01).sqrt();
    let b = (m[2] * m01).sqrt();
    let rho = [a * (r[1][0] - r[0][0]), a * (r[1][1] - r[0][1])];
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let lam = [b * (r[2][0] - c[0]), b * (r[2][1] - c[1])];
    let rr = rho[0] * rho[0] + rho[1] * rho[1];
    let ll = lam[0] * lam[0] + lam[1] * lam[1];
    let i = rr + ll;
    [
        (rr - ll) / i,
        2.0 * (rho[0] * lam[0] + rho[1] * lam[1]) / i,
        2.0 * (rho[0] * lam[1] - rho[1] * lam[0]) / i,
    ]
}

/// The decoded system's shape at `(u, v)`.
fn decoded_shape(chart: &ShapeSphere<f64>, u: f64, v: f64) -> [f64; 3] {
    let d = chart.decode::<Defaults, f64>(u, v);
    shape_of(d.ic.r, d.ic.m)
}

/// R-14's spherical point at `(s, t)`.
fn sphere_point(s: f64, t: f64) -> [f64; 3] {
    let (theta, phi) = (2.0 * PI * s, PI * (1.0 - t));
    [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]
}

/// chart_reference §3.3's exponential map, written out: `d = (2u − 1)s·e₁ + (2v − 1)s·e₂`, `r = ‖d‖`,
/// `n = cos r·n₀ + (sin r/r)·d`. Returns the point and `r`.
fn exp_point(c: &ShapeSphere<f64>, u: f64, v: f64) -> ([f64; 3], f64) {
    let (a, b) = ((2.0 * u - 1.0) * c.scale, (2.0 * v - 1.0) * c.scale);
    let d: [f64; 3] = std::array::from_fn(|k| a * c.e1[k] + b * c.e2[k]);
    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let k = if r == 0.0 { 1.0 } else { r.sin() / r };
    (std::array::from_fn(|i| r.cos() * c.n0[i] + k * d[i]), r)
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The centre of pixel `i` of `n`.
fn centre(i: u32, n: u32) -> f64 {
    (f64::from(i) + 0.5) / f64::from(n)
}

/// A small deterministic generator (splitmix64), so the draws are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn chart(map: ShapeMap, n0: [f64; 3], e1: [f64; 3], e2: [f64; 3], scale: f64) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0,
        e1,
        e2,
        scale,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.2, -0.1],
        z_mom: [0.0; 4],
    }
}

/// The smallest distance between two of `points`, and the pair.
fn closest(points: &[[f64; 3]]) -> (f64, usize, usize) {
    let mut best = (f64::INFINITY, 0, 0);
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let d = dist(points[i], points[j]);
            if d < best.0 {
                best = (d, i, j);
            }
        }
    }
    best
}

// ── REQ-CHART-002: the whole sphere ──────────────────────────────────────────────────────────────────────────────

const SPH: [u32; 2] = [24, 12];

/// The spherical chart's decoded shapes over the `SPH` grid, through `read` (the decode, or a control's fold), with
/// each pixel's expected point; then the assertions.
fn check_whole_sphere(read: fn(&ShapeSphere<f64>, f64, f64) -> [f64; 3], image: SystemImage) {
    let c = chart(
        ShapeMap::Spherical,
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        1.0,
    );
    let mut got = Vec::new();
    let (mut upper, mut lower) = (0, 0);
    for j in 0..SPH[1] {
        for i in 0..SPH[0] {
            let (s, t) = (centre(i, SPH[0]), centre(j, SPH[1]));
            let n = read(&c, s, t);
            let want = sphere_point(s, t);
            assert!(
                dist(n, want) < 1e-9,
                "pixel ({s}, {t}) decodes to shape {n:?}, not its own point {want:?}"
            );
            if t < 0.5 {
                assert!(
                    n[2] < 0.0,
                    "pixel ({s}, {t}) below the equator is not decoded below it"
                );
                lower += 1;
            } else {
                upper += 1;
            }
            got.push(n);
        }
    }
    assert_eq!(
        (upper, lower),
        (144, 144),
        "both hemispheres are on the grid"
    );
    // A pixel and its mirror (θ, π − φ): two systems.
    for j in 0..SPH[1] / 2 {
        for i in 0..SPH[0] {
            let a = got[(j * SPH[0] + i) as usize];
            let b = got[((SPH[1] - 1 - j) * SPH[0] + i) as usize];
            assert!(
                dist(a, b) > 1e-3,
                "pixel ({i}, {j}) and its mirror decode to one system"
            );
        }
    }
    let (d, a, b) = closest(&got);
    assert!(
        d > 1e-3,
        "pixels {a} and {b} decode to one system (shapes {d:e} apart)"
    );
    assert_eq!(
        image,
        SystemImage::Bijective,
        "the spherical map is not bijective"
    );
}

#[test]
fn qa_m228_spherical_whole_sphere() {
    let c = chart(
        ShapeMap::Spherical,
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        1.0,
    );
    check_whole_sphere(decoded_shape, c.system_image());
}

negative_control!(
    qa_m228_spherical_whole_sphere,
    "R-141's fold, a lower-hemisphere pixel decoded as its mirror, is not the pixel's own point, so it must fail",
    expected = "not its own point",
    {
        check_whole_sphere(
            |c, s, t| {
                let n = decoded_shape(c, s, t);
                [n[0], n[1], n[2].abs()]
            },
            SystemImage::Bijective,
        );
    }
);

// ── REQ-CHART-075: the exponential map's cap ─────────────────────────────────────────────────────────────────────

/// A unit vector, and an orthonormal frame `(n0, e1, e2)` about it turned by `turn`.
fn frame(n0: [f64; 3], turn: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let len = (n0[0] * n0[0] + n0[1] * n0[1] + n0[2] * n0[2]).sqrt();
    let n0 = n0.map(|x| x / len);
    let helper = if n0[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let a = cross(helper, n0);
    let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let a = a.map(|x| x / la);
    let b = cross(n0, a);
    let (c, s) = (turn.cos(), turn.sin());
    let e1 = std::array::from_fn(|k| c * a[k] + s * b[k]);
    let e2 = std::array::from_fn(|k| -s * a[k] + c * b[k]);
    (n0, e1, e2)
}

/// The caps drawn: each with a name for the messages.
fn caps() -> Vec<(String, ShapeSphere<f64>)> {
    let mut out = Vec::new();
    let e = ShapeMap::Exponential;
    // One hemisphere each: about the poles, s√2 < π/2; tilted off them.
    out.push((
        "about +w".into(),
        chart(e, [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.7),
    ));
    out.push((
        "about −w".into(),
        chart(e, [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.7),
    ));
    // Touching the equator from above: the corners at r = π/2 from +w.
    let touch = PI / 2.0 / 2f64.sqrt();
    out.push((
        "touching from above".into(),
        chart(e, [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], touch),
    ));
    // Straddling: about points of the equator.
    out.push((
        "straddling about x̂".into(),
        chart(e, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], 1.0),
    ));
    out.push((
        "straddling, past π".into(),
        chart(e, [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 2.8),
    ));
    let mut rng = Rng(0x2828_2828);
    for k in 0..12 {
        let n0 = [rng.next() - 0.5, rng.next() - 0.5, rng.next() - 0.5];
        let (n0, e1, e2) = frame(n0, 2.0 * PI * rng.next());
        let s = 0.2 + 2.8 * rng.next();
        out.push((
            format!("random cap {k} (s = {s:.3})"),
            chart(e, n0, e1, e2, s),
        ));
    }
    out
}

const CAP_N: u32 = 15;

/// Each cap: bijective by `image`; each pixel short of `r = π` decodes, through `read`, to the exponential map's point,
/// and no two of them to one system. Returns how many caps straddled the equator.
fn check_caps(
    read: fn(&ShapeSphere<f64>, f64, f64) -> [f64; 3],
    image: fn(&ShapeSphere<f64>) -> SystemImage,
) -> u32 {
    let mut straddling = 0;
    for (name, c) in caps() {
        assert_eq!(
            image(&c),
            SystemImage::Bijective,
            "{name}: the cap is not bijective"
        );
        let mut got = Vec::new();
        let (mut up, mut down) = (false, false);
        for j in 0..CAP_N {
            for i in 0..CAP_N {
                let (u, v) = (centre(i, CAP_N), centre(j, CAP_N));
                let (want, r) = exp_point(&c, u, v);
                if r >= PI - 1e-6 {
                    continue;
                }
                let n = read(&c, u, v);
                assert!(
                    dist(n, want) < 1e-9,
                    "{name}: pixel ({u}, {v}) decodes to shape {n:?}, not its own point {want:?}"
                );
                up |= want[2] > 1e-9;
                down |= want[2] < -1e-9;
                got.push(n);
            }
        }
        straddling += u32::from(up && down);
        let (d, a, b) = closest(&got);
        assert!(
            d > 1e-6,
            "{name}: pixels {a} and {b} decode to one system ({d:e} apart)"
        );
    }
    straddling
}

#[test]
fn qa_m228_exp_cap_bijective() {
    let straddling = check_caps(decoded_shape, ShapeSphere::system_image);
    assert!(
        straddling >= 4,
        "only {straddling} caps straddle the equator"
    );
}

negative_control!(
    qa_m228_exp_cap_bijective,
    "R-503's per-pixel fold, a w < 0 pixel read as its mirror, is not the pixel's own point, so it must fail",
    expected = "not its own point",
    {
        check_caps(
            |c, u, v| {
                let n = decoded_shape(c, u, v);
                [n[0], n[1], n[2].abs()]
            },
            ShapeSphere::system_image,
        );
    }
);

mod fixed_n2 {
    use super::*;
    negative_control!(
        qa_m228_exp_cap_bijective,
        "R-141's fixed n = 2 read for the exponential map is not bijective, so it must fail",
        expected = "is not bijective",
        {
            check_caps(decoded_shape, |_| SystemImage::NToOne(2));
        }
    );
}

// ── REQ-CHART-075: the count ─────────────────────────────────────────────────────────────────────────────────────

/// A counted total of `chart` over `nx` × `ny` pixel centres.
type Total = fn(&ShapeSphere<f64>, u32, u32) -> u64;

/// The symmetric grid's chart: `n₀ = (cos α, sin α, 0)` on the equator, `e₁` along it, `e₂ = ŵ`.
fn symmetric(alpha: f64, scale: f64) -> ShapeSphere<f64> {
    chart(
        ShapeMap::Exponential,
        [alpha.cos(), alpha.sin(), 0.0],
        [-alpha.sin(), alpha.cos(), 0.0],
        [0.0, 0.0, 1.0],
        scale,
    )
}

/// `r` at pixel `(i, j)` of `n`, from `(u, v)` alone: `s·√((2u − 1)² + (2v − 1)²)`.
fn r_of(scale: f64, i: u32, j: u32, n: u32) -> f64 {
    let (a, b) = (2.0 * centre(i, n) - 1.0, 2.0 * centre(j, n) - 1.0);
    scale * (a * a + b * b).sqrt()
}

fn check_count(total: Total) {
    let mut rng = Rng(0x0750_0750);
    let (mut inside, mut past) = (0, 0);
    while inside + past < 60 {
        let n = 3 + 2 * (rng.next() * 14.0) as u32;
        let alpha = 2.0 * PI * rng.next();
        let want_past = (inside + past) % 2 == 1;
        let scale = if want_past {
            // s√2·(N − 1)/N > π, up to 3π/2 at the corners.
            let lo = PI / 2f64.sqrt() * f64::from(n) / f64::from(n - 1);
            lo + (3.0 - lo) * rng.next()
        } else {
            PI / 2f64.sqrt() * rng.next()
        };
        if want_past && scale * 2f64.sqrt() * f64::from(n - 1) / f64::from(n) <= PI {
            continue;
        }
        let rs: Vec<f64> = (0..n * n).map(|k| r_of(scale, k % n, k / n, n)).collect();
        if rs.iter().any(|r| (r - PI).abs() <= 1e-6) {
            continue;
        }
        let c = symmetric(alpha, scale);
        let got = total(&c, n, n);
        if want_past {
            let want = rs.iter().filter(|&&r| r <= PI).count() as u64;
            assert!(want < u64::from(n * n), "the grid has a pixel past r = π");
            assert_eq!(
                got, want,
                "N = {n}, s = {scale}: the total is not the number of pixels with r ≤ π"
            );
            past += 1;
        } else {
            assert_eq!(
                got,
                u64::from(n * n),
                "N = {n}, s = {scale}: the straddling cap's total is not N², every pixel once"
            );
            inside += 1;
        }
    }
    // One-hemisphere caps: every pixel counts once.
    for sign in [1.0, -1.0] {
        let c = chart(
            ShapeMap::Exponential,
            [0.0, 0.0, sign],
            [1.0, 0.0, 0.0],
            [0.0, sign, 0.0],
            0.9,
        );
        assert_eq!(
            total(&c, 11, 7),
            77,
            "the cap in w·{sign} ≥ 0: every pixel once"
        );
    }
}

#[test]
fn qa_m228_exp_count() {
    check_count(|c, nx, ny| c.exp_counted_total(nx, ny));
}

/// R-503's withdrawn per-pixel fold: a `w < 0` pixel counts 0 when its mirror `w → −w` is in the cap.
#[cfg(feature = "controls")]
fn folded_total(c: &ShapeSphere<f64>, nx: u32, ny: u32) -> u64 {
    let mut total = 0;
    for j in 0..ny {
        for i in 0..nx {
            let (u, v) = (centre(i, nx), centre(j, ny));
            let (p, r) = exp_point(c, u, v);
            if r > PI {
                continue;
            }
            // The mirror on the symmetric grid is (u, 1 − v); in general, look for it by the point.
            let m = [p[0], p[1], -p[2]];
            let mirrored = (0..ny).any(|jj| {
                (0..nx).any(|ii| {
                    let (q, rq) = exp_point(c, centre(ii, nx), centre(jj, ny));
                    rq <= PI && dist(q, m) < 1e-12
                })
            });
            total += u64::from(!(p[2] < 0.0 && mirrored));
        }
    }
    total
}

negative_control!(
    qa_m228_exp_count,
    "R-503's per-pixel fold undercounts the straddling cap, so it must fail",
    expected = "is not N²",
    {
        check_count(folded_total);
    }
);

mod count_n2 {
    use super::*;
    negative_control!(
        qa_m228_exp_count,
        "a fixed n = 2, every system two pixels, halves the total, so it must fail",
        expected = "is not N²",
        {
            check_count(|c, nx, ny| c.exp_counted_total(nx, ny) / 2);
        }
    );
}

mod count_past {
    use super::*;
    negative_control!(
        qa_m228_exp_count,
        "a count including the pixels past r = π overcounts, so it must fail",
        expected = "not the number of pixels with r ≤ π",
        {
            check_count(|_, nx, ny| u64::from(nx * ny));
        }
    );
}

// ── REQ-CHART-078: the pixels on r = π ───────────────────────────────────────────────────────────────────────────

/// Grids at `s = 2π` whose pixel centres along one axis are `±1/6`, `±1/2` and `±5/6` of the half-width, each exact,
/// with the other axis at its centre line: the `±½` pixels lie exactly on `r = π`, the `±1/6` short of it, the `±5/6`
/// past it. About `+w`, `−w` and `x̂`, offset along `e₁` or along `e₂`.
fn antipode_grids() -> Vec<(String, ShapeSphere<f64>, u32, u32)> {
    let e = ShapeMap::Exponential;
    let mut out = Vec::new();
    for (name, n0, e1, e2) in [
        (
            "about +w",
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ),
        (
            "about −w",
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ),
        ("about x̂", [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ] {
        let c = chart(e, n0, e1, e2, 2.0 * PI);
        out.push((format!("{name}, along e₁"), c, 6, 1));
        out.push((format!("{name}, along e₂"), c, 1, 6));
    }
    out
}

fn check_antipode(total: Total) {
    for (name, c, nx, ny) in antipode_grids() {
        let mut on = 0;
        for j in 0..ny {
            for i in 0..nx {
                let (u, v) = (centre(i, nx), centre(j, ny));
                let (_, r) = exp_point(&c, u, v);
                let kernel_r = c.exp_at(u, v).1;
                assert_eq!(kernel_r, r, "{name}: ({u}, {v})'s r is not the map's own");
                if r == PI {
                    on += 1;
                    let n = decoded_shape(&c, u, v);
                    let anti = c.n0.map(|x| -x);
                    assert!(
                        dist(n, anti) < 1e-12,
                        "{name}: the pixel ({u}, {v}) on r = π decodes to {n:?}, not −n₀"
                    );
                    let d = c.decode::<Defaults, f64>(u, v);
                    assert!(
                        d.ic.r
                            .iter()
                            .flatten()
                            .chain(d.ic.p.iter().flatten())
                            .all(|x| x.is_finite()),
                        "{name}: the pixel ({u}, {v}) on r = π decodes to a non-finite state"
                    );
                }
            }
        }
        assert_eq!(
            on, 2,
            "{name}: the grid does not put two pixel centres exactly on r = π"
        );
        // Two pixels short of π, one each; two on it, once together; two past it, none.
        assert_eq!(
            total(&c, nx, ny),
            3,
            "{name}: the pixels on r = π do not count once together"
        );
    }
}

#[test]
fn qa_m228_antipode_once() {
    check_antipode(|c, nx, ny| c.exp_counted_total(nx, ny));
}

negative_control!(
    qa_m228_antipode_once,
    "each pixel on r = π counted on its own overcounts the one system −n₀, so it must fail",
    expected = "do not count once together",
    {
        check_antipode(|c, nx, ny| {
            let mut t = 0;
            for j in 0..ny {
                for i in 0..nx {
                    t += u64::from(exp_point(c, centre(i, nx), centre(j, ny)).1 <= PI);
                }
            }
            t
        });
    }
);

mod antipode_end {
    use super::*;
    negative_control!(
        qa_m228_antipode_once,
        "the pixels on r = π read as the domain's end drop the one system −n₀, so it must fail",
        expected = "do not count once together",
        {
            check_antipode(|c, nx, ny| {
                let mut t = 0;
                for j in 0..ny {
                    for i in 0..nx {
                        t += u64::from(exp_point(c, centre(i, nx), centre(j, ny)).1 < PI);
                    }
                }
                t
            });
        }
    );
}

// ── REQ-RENDER-026, REQ-CHART-047: Hammer's equal-area projection ────────────────────────────────────────────────

/// The display grid's side: `2M` × `M` points over `[0,1]²`.
const M: u32 = 1000;

/// The worst-case quantisation of a region's share on the grid: a region's boundary crosses at most about
/// `2·(2M) + 2·M` grid cells (the ellipse's and the cap's edges, each monotone in x and y on either side), out of the
/// `(π/4)·2M²` cells the ellipse holds.
fn share_tolerance() -> f64 {
    let m = f64::from(M);
    (6.0 * m) / (PI / 4.0 * 2.0 * m * m)
}

/// Over the display grid through `projection`: the shown share of the square is `π/4`; for each polar cap of angle α
/// about either pole and each band `|ℓ| < β` about the equator, its share of the shown points is its share of solid
/// angle; the poles are drawn at the top and the bottom of the centre line.
fn check_equal_area(projection: Projection) {
    let caps = [PI / 12.0, PI / 6.0, PI / 3.0];
    let bands = [PI / 12.0, PI / 4.0];
    let mut north = [0u64; 3];
    let mut south = [0u64; 3];
    let mut band = [0u64; 2];
    let mut shown = 0u64;
    for yi in 0..M {
        for xi in 0..2 * M {
            let (x, y) = (centre(xi, 2 * M), centre(yi, M));
            let Some([_, t]) = projection.chart_point(x, y) else {
                continue;
            };
            shown += 1;
            let phi = PI * (1.0 - t);
            for (k, &a) in caps.iter().enumerate() {
                north[k] += u64::from(phi < a);
                south[k] += u64::from(phi > PI - a);
            }
            for (k, &b) in bands.iter().enumerate() {
                band[k] += u64::from((PI / 2.0 - phi).abs() < b);
            }
        }
    }
    let tol = share_tolerance();
    let total = f64::from(2 * M * M);
    let s = shown as f64;
    let ellipse = PI / 4.0;
    if projection == Projection::EqualArea {
        assert!(
            (s / total - ellipse).abs() < tol,
            "the shown share {} is not the 2:1 ellipse's π/4",
            s / total
        );
    }
    for (k, &a) in caps.iter().enumerate() {
        let want = (1.0 - a.cos()) / 2.0;
        for (side, got) in [("north", north[k]), ("south", south[k])] {
            let got = got as f64 / s;
            assert!(
                (got - want).abs() < tol,
                "the {side} cap of angle {a:.4} has share {got:.5} of the display, not its solid angle's {want:.5}"
            );
        }
    }
    for (k, &b) in bands.iter().enumerate() {
        let want = b.sin();
        let got = band[k] as f64 / s;
        assert!(
            (got - want).abs() < tol,
            "the band |ℓ| < {b:.4} has share {got:.5} of the display, not its solid angle's {want:.5}"
        );
    }
    // The poles: L⁺ (t = 1) at the top of the centre line, L⁻ (t = 0) at its bottom, for any θ.
    for s in [0.1, 0.5, 0.9] {
        let [x, y] = projection.display_point(s, 1.0);
        assert!(
            (x - 0.5).abs() < 1e-12 && (y - 1.0).abs() < 1e-12,
            "the north pole at θ = 2π·{s} is drawn at ({x}, {y})"
        );
        let [x, y] = projection.display_point(s, 0.0);
        assert!(
            (x - 0.5).abs() < 1e-12 && y.abs() < 1e-12,
            "the south pole at θ = 2π·{s} is drawn at ({x}, {y})"
        );
    }
}

#[test]
fn qa_m228_hammer_equal_area() {
    assert_eq!(
        Projection::default(),
        Projection::Equirectangular,
        "the default is not equirectangular"
    );
    check_equal_area(Projection::EqualArea);
}

negative_control!(
    qa_m228_hammer_equal_area,
    "the equirectangular display gives a polar cap the share α/π, not its solid angle's, so it must fail",
    expected = "not its solid angle's",
    {
        check_equal_area(Projection::Equirectangular);
    }
);
