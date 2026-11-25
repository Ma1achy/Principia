//! Rank loss under the holds (`kernel::decode::hold::plane_image`; chart_composition § 5.4, § 5.5 row 9): a swept
//! velocity plane `v₀ + a·e₁ + b·e₂` is labelled `ray-degenerate` exactly where the held decode maps it onto a curve of
//! systems.
//!
//! **Pass criteria, written before the runs.** The label equals the measured rank loss, `σ_min/σ_max < 10⁻⁶` of the
//! held decode's Jacobian over the plane at `(0.3, 0.2)` by central differences, in every case below; and it is the
//! design document's verdict: reported for a plane containing `Jr` (hold `L_z` alone, both), for one through rest
//! (hold `E` alone, both), and for one through a rigid rotation (both held); not reported for a generic plane under
//! any hold, for one through a rigid rotation under hold `E` alone, and for one through rest under hold `L_z` alone.
//! The design document's ratios are `≤ 10⁻⁹` where rank is lost and `≥ 0.45` where it is kept; the draws here, normalised by the Jacobian's Frobenius norm, are at most `10⁻⁹` and at least `10⁻²`.

mod decode_support;
mod hold_support;

use hold_support::*;
use kernel::chart::SystemImage;
use kernel::decode::energy_norm::normalise;
use kernel::decode::hold::{hold, plane_image};
use kernel::decode::Ic;
use validation::negative_control;

/// The held decode of the velocity `v` at the state `(m, r)`: `E` alone through `η_E`, `L_z` alone or both through the
/// hold, at `L* = 0.5` and, with `E` held, `K* = L*²/2 + 1/2` (`a = 1`).
fn decode(m: &[f64; 3], r: &Bodies, v: &Bodies, e_on: bool, lz_on: bool) -> Bodies {
    let u = potential(m, r);
    let (l, k) = (0.5, 0.125 + 0.5);
    if lz_on {
        return hold(*m, r, v, u, 1.0, l, e_on, u + k).v;
    }
    let p = v.map(|b| b);
    let ic = Ic {
        m: *m,
        r: *r,
        p: [0, 1, 2].map(|i| [m[i] * p[i][0], m[i] * p[i][1]]),
    };
    let out = normalise(&ic, u, 1.0, u + k).p;
    [0, 1, 2].map(|i| [out[i][0] / m[i], out[i][1] / m[i]])
}

/// `σ_min/σ_max` of the Jacobian of the held decode over the plane `v₀ + a·e₁ + b·e₂` at `(a, b) = (0.3, 0.2)`:
/// `σ_min·σ_max` is the area the two columns span, read from `‖j₂ − (j₂·ĵ₁)ĵ₁‖` with no squaring, so a ratio of `10⁻¹⁰`
/// does not cancel.
fn ratio(m: &[f64; 3], r: &Bodies, base: &Bodies, e: [&Bodies; 2], e_on: bool, lz_on: bool) -> f64 {
    let at = |a: f64, b: f64| {
        let v = [0, 1, 2].map(|i| [0, 1].map(|c| base[i][c] + a * e[0][i][c] + b * e[1][i][c]));
        let d = decode(m, r, &v, e_on, lz_on);
        d.iter().flatten().copied().collect::<Vec<f64>>()
    };
    let h = 1e-6;
    let col = |da: f64, db: f64| {
        let (hi, lo) = (at(0.3 + da, 0.2 + db), at(0.3 - da, 0.2 - db));
        hi.iter()
            .zip(&lo)
            .map(|(x, y)| (x - y) / (2.0 * h))
            .collect::<Vec<f64>>()
    };
    let (j1, j2) = (col(h, 0.0), col(0.0, h));
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let n1 = dot(&j1, &j1).sqrt();
    let big = (n1 * n1 + dot(&j2, &j2)).sqrt();
    // A column that is rounding noise beside the other is rank loss: σ_min ≤ ‖j₁‖.
    if n1 <= 1e-12 * big {
        return n1 / big;
    }
    let t = dot(&j2, &j1) / n1;
    let perp: Vec<f64> = j2.iter().zip(&j1).map(|(y, x)| y - t * x / n1).collect();
    let area = n1 * dot(&perp, &perp).sqrt();
    // σ_max ≤ the Frobenius norm and ≥ its half: the ratio is read against the Frobenius norm, within a factor √2.
    (area / big) / big
}

/// One plane: its name, the state's draw and the plane's directions.
struct Plane {
    name: &'static str,
    base: Bodies,
    e: [Bodies; 2],
}

/// The planes of the design document's § 5.4, at one state.
fn planes(m: &[f64; 3], r: &Bodies, rng: &mut Rng) -> Vec<Plane> {
    let w = jr(r);
    let generic = velocity(m, rng);
    let e0 = velocity(m, rng);
    let e1 = velocity(m, rng);
    let rest = [[0.0; 2]; 3];
    let rigid = w.map(|b| b.map(|x| 1.3 * x));
    vec![
        Plane {
            name: "generic",
            base: generic,
            e: [e0, e1],
        },
        Plane {
            name: "contains Jr",
            base: generic,
            e: [w, e1],
        },
        Plane {
            name: "through rest",
            base: rest,
            e: [e0, e1],
        },
        Plane {
            name: "through a rigid rotation",
            base: rigid,
            e: [e0, e1],
        },
    ]
}

/// The label for each plane and hold, read against the measured ratio and the design document's verdict.
fn check_rank(label: fn(&[f64; 3], &Bodies, &Plane, bool, bool) -> SystemImage) {
    let mut rng = Rng(9);
    let (mut worst_lost, mut least_kept) = (0.0_f64, f64::INFINITY);
    for trial in 0..20 {
        let (m, r) = state(&mut rng);
        for plane in planes(&m, &r, &mut rng) {
            for (hold_name, e_on, lz_on) in [
                ("E", true, false),
                ("L_z", false, true),
                ("both", true, true),
            ] {
                let measured = ratio(&m, &r, &plane.base, [&plane.e[0], &plane.e[1]], e_on, lz_on);
                let lost = measured < 1e-6;
                if lost {
                    worst_lost = worst_lost.max(measured);
                } else {
                    least_kept = least_kept.min(measured);
                }
                let verdict = matches!(
                    (plane.name, hold_name),
                    ("contains Jr", "L_z" | "both")
                        | ("through rest", "E" | "both")
                        | ("through a rigid rotation", "both")
                );
                assert_eq!(
                    lost, verdict,
                    "measured: the plane {} under hold {hold_name} has ratio {measured:e} (trial {trial})",
                    plane.name
                );
                let said = label(&m, &r, &plane, e_on, lz_on) == SystemImage::RayDegenerate;
                assert_eq!(
                    said, lost,
                    "label: the plane {} under hold {hold_name} {} (ratio {measured:e}, trial {trial})",
                    plane.name,
                    if lost { "loses rank and is not reported" } else { "keeps rank and is reported" }
                );
            }
        }
    }
    println!(
        "ratio where rank is lost: at most {worst_lost:e}; where kept: at least {least_kept:e}"
    );
}

fn reported(m: &[f64; 3], r: &Bodies, p: &Plane, e_on: bool, lz_on: bool) -> SystemImage {
    plane_image(*m, r, &p.base, [&p.e[0], &p.e[1]], e_on, lz_on)
}

#[test]
fn hold_rank_labels() {
    check_rank(reported);
}

/// The label removed.
#[cfg(feature = "controls")]
fn removed(_: &[f64; 3], _: &Bodies, _: &Plane, _: bool, _: bool) -> SystemImage {
    SystemImage::Bijective
}

/// Rank loss reported for a plane through a rigid rotation under hold `E` alone as well: the centre of the scale is rest
/// there, not the rotation.
#[cfg(feature = "controls")]
fn rigid_under_e(m: &[f64; 3], r: &Bodies, p: &Plane, e_on: bool, lz_on: bool) -> SystemImage {
    if p.name == "through a rigid rotation" && e_on && !lz_on {
        return SystemImage::RayDegenerate;
    }
    reported(m, r, p, e_on, lz_on)
}

negative_control!(
    hold_rank_labels,
    "the label removed fails to report a plane containing Jr",
    expected = "loses rank and is not reported",
    check_rank(removed)
);

#[cfg(feature = "controls")]
mod rigid_under_e_alone {
    use super::*;
    negative_control!(
        hold_rank_labels,
        "a plane through a rigid rotation under hold E alone reporting rank loss fails",
        expected = "keeps rank and is reported",
        check_rank(rigid_under_e)
    );
}
