//! Wrap, pole-crossing and the exponential map's repeat in the CPU evaluator.
//!
//! **Criteria**, written before the first run:
//! - periodic: for dyadic `(s, t)` in `[0,1]²`, the pixels at `(s ± 1, t)` and `(s + 2, t)` resolve to `(s, t)` with
//!   the same bits, decode (through the real shape-sphere chart) to the same bits as `(s, t)`, and count as repeats;
//! - pole-crossing: `(s, 1 + δ)` resolves to `(s + ½, 1 − δ)` bitwise, and the chart's own continuation through the
//!   trig, which no resolve touches, agrees with the resolved decode to `1e-12`;
//! - inside `[0,1]²` a pixel comes back with its input bits and counts as itself;
//! - repeat map: untilted, `(r, ê)` with `π < r < 2π` resolves to `(2π − r, −ê)`, counted as a repeat, whose sphere
//!   point is the pixel's own to `1e-12`; tilted it applies at `N_a = N_b = 0` (τ₁ = τ₂ = 0, and Tip as sheet at
//!   τ₁ = 108°, τ₂ = 144°, m = 180°) and not at τ₂ = 30°, where the pixel is its own system, counted;
//! - each control, run through the same check, must fail.

use engine::edge::wrap::{exponential, spherical, Fold};
use engine::edge::{Count, Kind, Resolved};
use kernel::chart::{Chart, ShapeMap, ShapeSphere, TiltMap, TiltSpec};
use kernel::generated::links::Defaults;

fn sphere(map: ShapeMap, scale: f64) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 0.6, 0.8],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, -0.8, 0.6],
        scale,
        inertia: 1.0,
        phase: 0.3,
        z_mass: [0.2, -0.4],
        z_mom: [0.1, -0.2, 0.3, 0.4],
    }
}

fn bits(c: &ShapeSphere<f64>, at: [f64; 2]) -> Vec<u64> {
    let ic = c.decode::<Defaults, f64>(at[0], at[1]).ic;
    let mut v: Vec<u64> = ic.m.iter().map(|x| x.to_bits()).collect();
    v.extend(ic.r.iter().flatten().map(|x| x.to_bits()));
    v.extend(ic.p.iter().flatten().map(|x| x.to_bits()));
    v
}

fn fold(s: Kind, t: Kind) -> Fold {
    Fold {
        s,
        t,
        normal_zero: [true, true],
    }
}

const DYADIC: [f64; 7] = [0.0, 0.125, 0.25, 0.375, 0.5, 0.75, 1.0];

fn same(a: [f64; 2], b: [f64; 2]) -> bool {
    a[0].to_bits() == b[0].to_bits() && a[1].to_bits() == b[1].to_bits()
}

fn check_periodic(resolve: impl Fn([f64; 2]) -> Resolved) {
    let c = sphere(ShapeMap::Spherical, 1.0);
    // `s = 1` is the point `s = 0` drawn twice: inside the square it stays as written.
    for &s in &DYADIC[..6] {
        for &t in &DYADIC {
            let canon = bits(&c, [s, t]);
            for k in [-1.0, 1.0, 2.0] {
                if k == 1.0 && s == 0.0 {
                    continue;
                }
                let r = resolve([s + k, t]);
                assert!(
                    same(r.at, [s, t]),
                    "periodic: ({}, {t}) resolves to {:?}, not ({s}, {t})",
                    s + k,
                    r.at
                );
                assert_eq!(bits(&c, r.at), canon, "periodic: decode differs at s + {k}");
                assert_eq!(r.count, Count::Repeat, "periodic: s + {k} is a repeat");
            }
        }
    }
}

#[test]
fn edge_wrap_periodic() {
    check_periodic(|p| spherical(p, fold(Kind::Wrap, Kind::PoleCrossing)));
}

validation::negative_control!(
    edge_wrap_periodic,
    "the periodic arm swapped for blank",
    expected = "periodic:",
    check_periodic(|p| spherical(p, fold(Kind::Blank, Kind::PoleCrossing)))
);

fn check_pole(resolve: impl Fn([f64; 2]) -> Resolved) {
    let c = sphere(ShapeMap::Spherical, 1.0);
    for &s in &[0.0, 0.125, 0.25, 0.5] {
        // Not `d = ½` at `θ ∈ {0, π}`: that is the decoder's own pole `n_x = ±1`, where its decode is singular.
        for &d in &[0.0625, 0.25, 0.375] {
            let r = resolve([s, 1.0 + d]);
            assert!(
                same(r.at, [s + 0.5, 1.0 - d]),
                "pole: ({s}, 1 + {d}) resolves to {:?}",
                r.at
            );
            assert_eq!(r.count, Count::Repeat, "pole: a repeat");
            let today = c.decode::<Defaults, f64>(s, 1.0 + d).ic;
            let seen = c.decode::<Defaults, f64>(r.at[0], r.at[1]).ic;
            for (a, b) in today.r.iter().flatten().zip(seen.r.iter().flatten()) {
                assert!(
                    (a - b).abs() < 1e-12,
                    "pole: the trig continuation differs at ({s}, 1 + {d}): {a} {b}"
                );
            }
        }
    }
    // Over the pole the other way, and a full period in `t`.
    assert!(
        same(resolve([0.25, -0.25]).at, [0.75, 0.25]),
        "pole: below 0"
    );
    assert!(
        same(resolve([0.25, 2.25]).at, [0.25, 0.25]),
        "pole: a period of 2 in t"
    );
}

#[test]
fn edge_wrap_pole_crossing() {
    check_pole(|p| spherical(p, fold(Kind::Wrap, Kind::PoleCrossing)));
}

validation::negative_control!(
    edge_wrap_pole_crossing,
    "the pole-crossing arm swapped for blank",
    expected = "pole:",
    check_pole(|p| spherical(p, fold(Kind::Wrap, Kind::Blank)))
);

fn check_inside(resolve: impl Fn([f64; 2]) -> Resolved) {
    for &s in &DYADIC {
        for &t in &DYADIC {
            let r = resolve([s, t]);
            assert!(same(r.at, [s, t]), "inside: ({s}, {t}) moved to {:?}", r.at);
            assert_eq!(r.count, Count::System, "inside: ({s}, {t}) is a system");
            assert_eq!(r.applied, None, "inside: no kind acts");
        }
    }
}

#[test]
fn edge_wrap_inside_unchanged() {
    check_inside(|p| spherical(p, fold(Kind::Wrap, Kind::PoleCrossing)));
    check_inside(|p| exponential(p, 1.5, Kind::Repeat, [true, true]));
}

validation::negative_control!(
    edge_wrap_inside_unchanged,
    "a resolver that folds every pixel by a period",
    expected = "inside:",
    check_inside(|p| Resolved {
        at: [p[0] + 0.5, p[1]],
        ..Resolved::own(p)
    })
);

/// A canonical point at radius `rho` along angle `phi` of the exponential map of extent `scale`, as a pixel.
fn pixel(scale: f64, rho: f64, phi: f64) -> [f64; 2] {
    [
        (rho * phi.cos() / scale + 1.0) / 2.0,
        (rho * phi.sin() / scale + 1.0) / 2.0,
    ]
}

const SCALE: f64 = 1.5;

fn check_repeat(
    spec: TiltSpec,
    want_repeat: bool,
    resolve: impl Fn([f64; 2], [bool; 2]) -> Resolved,
) {
    let c = sphere(ShapeMap::Exponential, SCALE);
    let normal_zero = spec.normal_zero();
    for (r, phi) in [(4.0, 0.7), (5.5, 2.1), (3.4, -1.2)] {
        let p = pixel(SCALE, r, phi);
        // The same system, from the pixel's side: radius 2π − r, direction reversed.
        let q = pixel(SCALE, std::f64::consts::TAU - r, phi + std::f64::consts::PI);
        let out = resolve(p, normal_zero);
        if want_repeat {
            assert_eq!(out.count, Count::Repeat, "repeat: r = {r} is a repeat");
            for k in 0..2 {
                assert!(
                    (out.at[k] - q[k]).abs() < 1e-14,
                    "repeat: r = {r} resolves to {:?}, not {:?}",
                    out.at,
                    q
                );
            }
            let a = c.point::<f64>(p[0], p[1]);
            let b = c.point::<f64>(out.at[0], out.at[1]);
            for k in 0..3 {
                assert!(
                    (a[k] - b[k]).abs() < 1e-12,
                    "repeat: the sphere point differs at r = {r}"
                );
            }
            assert_eq!(
                bits(&c, out.at),
                bits(&c, out.at),
                "repeat: decode of the resolved point"
            );
        } else {
            assert_eq!(
                out.count,
                Count::System,
                "repeat: tilted, r = {r} is its own system"
            );
            assert!(same(out.at, p), "repeat: tilted, r = {r} is not moved");
        }
    }
}

fn spec(tau1: f64, tau2: f64) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map: TiltMap::Sheet,
        x: 0,
        y: 0,
    }
}

fn arm(p: [f64; 2], nz: [bool; 2]) -> Resolved {
    exponential(p, SCALE, Kind::Repeat, nz)
}

#[test]
fn edge_wrap_repeat_map() {
    check_repeat(spec(0.0, 0.0), true, arm);
}

validation::negative_control!(
    edge_wrap_repeat_map,
    "the repeat arm swapped for blank",
    expected = "repeat: r",
    check_repeat(spec(0.0, 0.0), true, |p, nz| exponential(
        p,
        SCALE,
        Kind::Blank,
        nz
    ))
);

/// Tip as sheet at m = 180° leaves `N_a = N_b = 0`, exactly: the map applies.
#[test]
fn edge_wrap_repeat_map_m180() {
    check_repeat(spec(108.0, 144.0), true, arm);
}

validation::negative_control!(
    edge_wrap_repeat_map_m180,
    "N_a = N_b = 0 tested on computed floating-point values drops the map at m = 180 degrees",
    expected = "repeat: r",
    check_repeat(spec(108.0, 144.0), true, |p, _| {
        let (t1, t2) = (108.0f64, 144.0f64);
        let m = (t1 * t1 + t2 * t2).sqrt().to_radians();
        let na = -t1.to_radians() * m.sin() / m;
        let nb = -t2.to_radians() * m.sin() / m;
        arm(p, [na == 0.0, nb == 0.0])
    })
);

/// At τ₂ = 30° the tilt adds an offset along `b`: the pixel is its own system, counted.
#[test]
fn edge_wrap_repeat_map_tilted() {
    check_repeat(spec(0.0, 30.0), false, arm);
}

validation::negative_control!(
    edge_wrap_repeat_map_tilted,
    "the repeat kept at tau2 = 30 degrees",
    expected = "repeat: tilted",
    check_repeat(spec(0.0, 30.0), false, |p, _| arm(p, [true, true]))
);
