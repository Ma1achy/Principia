//! Saturate in the CPU evaluator: the squash `p = p_s + Δ·d/(Δ + d)` and the masses evaluated from the remaining
//! distance.
//!
//! **Criteria**, written before the first run. Samples run from the squash's start out to `10³` square widths
//! (`10³` units outward on the unit-width strips, `10³·0.9` on `ν`):
//! - the coordinate is finite and strictly monotone outward, never past `p₀` (`ν` and `θ` decreasing toward `ν₀`,
//!   `θ₀`; `v` increasing on the strips), and every mass is positive;
//! - the binding mass is strictly decreasing and above its floor (`ε_M01`; on a strip edge already below `ε_M01`,
//!   above 0), `M₀₁ ≥ ε_M01` on the strip whose edge has bodies 0 and 1 both below `ε_M01`, no NaN anywhere, every
//!   pixel counted;
//! - across `p_s`, samples `10⁻¹⁰` apart show the binding mass strictly monotone.
//!
//! Controls, each must fail: the unmodified formula (`p₀` at `ε_M01` whatever the edge mass is, `|Δ|`), the raw
//! coordinate difference without `σ`, the masses evaluated at the rounded `p` (sampled to `10¹⁶` where `p` reaches
//! `p₀`), the masses taken as `p₀`'s value plus `δ` times the derivative, and the saturate arm swapped for blank.

use engine::edge::saturate::{
    affine_band, affine_masses, nu_masses, resolve, theta_masses, Band, Squash, EPS_M, EPS_M01,
};
use engine::edge::{Count, Kind};

fn sizes(max: f64) -> Vec<f64> {
    (0..400)
        .map(|i| 1e-9 * (max / 1e-9).powf(f64::from(i) / 399.0))
        .collect()
}

// ── The mass strips (affine masses): the simplex's side and the δm strip ───────────────────────────────────────────

struct Strip {
    name: &'static str,
    m: [f64; 3],
    b: [f64; 3],
    /// The index of the mass whose floor binds first, or `None` where `M₀₁` does.
    binding: Option<usize>,
}

fn strips() -> Vec<Strip> {
    let burrau = [0.4, 0.3, 0.3];
    let blend = |target: [f64; 3], burrau: [f64; 3]| {
        let b = core::array::from_fn(|i| target[i] - burrau[i]);
        (target, b)
    };
    let (t1, b1) = blend([0.5, 0.499999, 1e-6], burrau);
    let (t2, b2) = blend([6e-6, 6e-6, 1.0 - 1.2e-5], [0.4, 0.4, 0.2]);
    let (t3, b3) = blend([0.5, 0.5 - EPS_M01, EPS_M01], burrau);
    vec![
        Strip {
            name: "simplex side",
            m: [0.5, 0.3, 0.2],
            b: [0.1, -0.5, 0.4],
            binding: Some(1),
        },
        Strip {
            name: "edge mass below the fence",
            m: t1,
            b: b1,
            binding: Some(2),
        },
        Strip {
            name: "bodies 0 and 1 below the fence",
            m: t2,
            b: b2,
            binding: None,
        },
        Strip {
            name: "edge mass exactly the fence",
            m: t3,
            b: b3,
            binding: Some(2),
        },
    ]
}

fn check_strip(band: Band, from_rounded: bool, widths: f64, sigma: f64) {
    for s in strips() {
        let Some((p_s, p0)) = affine_band(s.m, s.b, band) else {
            panic!("saturate: {} has no squash", s.name);
        };
        let sq = Squash { sigma, p_s, p0 };
        assert!(
            sq.width() > 0.0,
            "saturate: {}: Δ = {} is not positive",
            s.name,
            sq.width()
        );
        let mut last = (f64::NEG_INFINITY, f64::INFINITY);
        // Past `10³` widths the rounded `p` itself reaches `p₀`; only the masses are read there.
        let extreme = widths > 1e3;
        for x in sizes(widths) {
            let at = [0.5, 1.0 + p_s + x];
            let (r, sat) = resolve(
                at,
                1,
                &Squash {
                    sigma,
                    p_s: 1.0 + p_s,
                    p0: 1.0 + p0,
                },
                Kind::Saturate,
                true,
            );
            let sat = sat.expect("past the start");
            assert_eq!(r.count, Count::Saturated, "saturate: {}: counted", s.name);
            let out = r.at[1] - 1.0;
            assert!(
                out.is_finite() && (extreme || out > last.0),
                "saturate: {}: v = {out} is not monotone outward",
                s.name
            );
            assert!(
                extreme || out > 0.0,
                "saturate: {}: v folded inward",
                s.name
            );
            last.0 = out;
            let m = if from_rounded {
                core::array::from_fn(|i| s.m[i] + s.b[i] * out)
            } else {
                affine_masses(s.m, s.b, p0, sat.remaining)
            };
            assert!(
                m.iter().all(|v| v.is_finite() && *v > 0.0),
                "saturate: {}: a mass {m:?} is not positive",
                s.name
            );
            if let Some(i) = s.binding {
                assert!(
                    extreme || m[i] < last.1,
                    "saturate: {}: the binding mass is not decreasing",
                    s.name
                );
                last.1 = m[i];
            }
            if s.binding.is_none() {
                assert!(
                    m[0] + m[1] >= EPS_M01,
                    "saturate: {}: M01 = {} below the fence",
                    s.name,
                    m[0] + m[1]
                );
            }
        }
    }
}

#[test]
fn edge_saturate_strips() {
    check_strip(Band::First, false, 1e3, 1.0);
}

validation::negative_control!(
    edge_saturate_strips,
    "the unmodified formula: p0 at the fence whatever the edge mass is",
    expected = "saturate:",
    check_strip(Band::EpsEverywhere, false, 1e3, 1.0)
);

#[test]
fn edge_saturate_masses_from_remaining() {
    check_strip(Band::First, false, 1e16, 1.0);
}

validation::negative_control!(
    edge_saturate_masses_from_remaining,
    "masses blended at the rounded p",
    expected = "saturate:",
    check_strip(Band::First, true, 1e16, 1.0)
);

#[test]
fn edge_saturate_blank() {
    let sq = Squash {
        sigma: 1.0,
        p_s: 1.0,
        p0: 1.1,
    };
    let (r, _) = resolve([0.5, 2.0], 1, &sq, Kind::Saturate, true);
    assert_eq!(
        r.count,
        Count::Saturated,
        "blank: the saturate arm shows a pixel"
    );
}

validation::negative_control!(
    edge_saturate_blank,
    "the saturate arm swapped for blank",
    expected = "blank:",
    {
        let sq = Squash {
            sigma: 1.0,
            p_s: 1.0,
            p0: 1.1,
        };
        let (r, _) = resolve([0.5, 2.0], 1, &sq, Kind::Blank, true);
        assert_eq!(
            r.count,
            Count::Saturated,
            "blank: the saturate arm shows a pixel"
        );
    }
);

// ── ν and θ edges (decreasing outward, σ = −1) ───────────────────────────────────────────────────────────────────────

/// Which closed form and its start and end.
#[derive(Clone, Copy)]
enum Axis {
    Nu,
    Theta,
}

/// `(p_s, p0)` in the axis's coordinate, the masses at `p₀` with the binding mass exactly `ε_M01`, and the binding index.
fn axis(a: Axis) -> (f64, f64, [f64; 3], usize) {
    let at = |eps: f64| eps / (1.0 - eps);
    match a {
        Axis::Nu => {
            let nu0 = at(EPS_M01);
            let m = [
                (1.0 + nu0 * nu0) / (2.0 * (1.0 + nu0)),
                EPS_M01,
                (1.0 - nu0) / 2.0,
            ];
            (at(EPS_M), nu0, m, 1)
        }
        Axis::Theta => {
            let t0 = at(EPS_M01);
            let m = [
                (1.0 + t0 * t0) / (2.0 * (1.0 + t0)),
                (1.0 - t0) / 2.0,
                EPS_M01,
            ];
            (2.0 * at(EPS_M).atan(), 2.0 * t0.atan(), m, 2)
        }
    }
}

fn masses(a: Axis, x0: f64, at0: [f64; 3], d: f64) -> [f64; 3] {
    match a {
        Axis::Nu => nu_masses(x0, at0, d),
        Axis::Theta => theta_masses(x0, at0, d),
    }
}

fn direct(a: Axis, x: f64) -> [f64; 3] {
    match a {
        Axis::Nu => [
            (1.0 + x * x) / (2.0 * (1.0 + x)),
            x / (1.0 + x),
            (1.0 - x) / 2.0,
        ],
        Axis::Theta => {
            let t = (x / 2.0).tan();
            [
                (1.0 + t * t) / (2.0 * (1.0 + t)),
                (1.0 - t) / 2.0,
                t / (1.0 + t),
            ]
        }
    }
}

/// The binding mass's derivative along the axis at `p₀`.
fn slope(a: Axis, x0: f64) -> f64 {
    match a {
        Axis::Nu => 1.0 / ((1.0 + x0) * (1.0 + x0)),
        Axis::Theta => {
            let t0 = (x0 / 2.0).tan();
            0.5 * (1.0 + t0 * t0) / ((1.0 + t0) * (1.0 + t0))
        }
    }
}

/// `linear`: the masses as `p₀`'s value plus `δ` times the derivative there.
fn check_edge(a: Axis, sigma: f64, linear: bool) {
    let (x_s, x0, at0, bind) = axis(a);
    // Outward is `σ·x`: on these edges the coordinate decreases outward, σ = −1.
    let sq = Squash {
        sigma,
        p_s: sigma * x_s,
        p0: sigma * x0,
    };
    assert!(
        sq.width() > 0.0,
        "edge: Δ = {} is not positive (σ left out)",
        sq.width()
    );
    let mut last = (f64::INFINITY, f64::INFINITY);
    for d in sizes(900.0) {
        let (r, sat) = resolve([x_s - d, 0.5], 0, &sq, Kind::Saturate, true);
        let sat = sat.expect("past the start");
        let c = r.at[0];
        assert!(
            c.is_finite() && c < last.0 && c > x0,
            "edge: coordinate {c} is not monotone toward {x0}"
        );
        last.0 = c;
        let m = if linear {
            let slope = slope(a, x0);
            let mut m = at0;
            m[bind] += slope * sat.remaining;
            m
        } else {
            masses(a, x0, at0, sat.remaining)
        };
        assert!(
            m[bind] < last.1 && m[bind] > EPS_M01,
            "edge: the binding mass {} is not decreasing above the fence",
            m[bind]
        );
        assert!(m.iter().all(|v| *v > 0.0), "edge: a mass is not positive");
        last.1 = m[bind];
    }
    // Straddling p_s at spacing 1e-10.
    let mut prev = f64::INFINITY;
    for k in -20..=20 {
        let x = x_s - f64::from(k) * 1e-10; // k < 0: the affine side, k > 0 past p_s
        let m = match sq.at(x) {
            None => direct(a, x)[bind],
            Some(s) => {
                if linear {
                    at0[bind] + slope(a, x0) * s.remaining
                } else {
                    masses(a, x0, at0, s.remaining)[bind]
                }
            }
        };
        assert!(
            m < prev,
            "edge: the binding mass is not monotone across p_s at k = {k}: {m} after {prev}"
        );
        prev = m;
    }
}

#[test]
fn edge_saturate_nu_and_theta() {
    check_edge(Axis::Nu, -1.0, false);
    check_edge(Axis::Theta, -1.0, false);
}

validation::negative_control!(
    edge_saturate_nu_and_theta,
    "Δ taken as the raw coordinate difference, σ left out",
    expected = "edge: Δ",
    check_edge(Axis::Nu, 1.0, false)
);

#[test]
fn edge_saturate_nu_linearised_masses() {
    check_edge(Axis::Nu, -1.0, false);
}

validation::negative_control!(
    edge_saturate_nu_linearised_masses,
    "masses as p0's value plus delta times its derivative",
    expected = "edge:",
    check_edge(Axis::Nu, -1.0, true)
);
