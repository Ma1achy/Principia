//! The mock's fake sample under the cursor and its hover path (R-390; applied per R-369, review 5434766412 on PR
//! #157): for the cursor's figure point, a plausible IC, fate, summary and paths, deterministic in the IC, and the hover
//! path the mock draws into its stand-in, as the real compositor draws the trace, solid near the slice and faint away
//! from it. Placeholder content: the fall-off is a stand-in, not REQ-GUI-156's definition, and the real hover
//! integration is TASK-M8-10's.

use engine::contract::display;
use engine::contract::legend::SWATCHES;
use engine::contract::sample::Sample;
use engine::contract::sim_config::{Latent, Plane};

use crate::explore::manifold_view::{add, chart_coords, dot, norm, point};

/// How many points each fake path has.
pub const PATH_POINTS: usize = 240;
/// How many points the hover path has.
pub const HOVER_POINTS: usize = 160;
/// The hover path's out-of-plane distance, as a fraction of the view's half-span, within which it is drawn solid (the
/// stand-in fade, a look choice, R-390).
pub const NEAR: f64 = 0.04;
/// The distance, past [`NEAR`], over which it fades to [`FAINT`] (the stand-in fade, a look choice, R-390).
pub const FADE: f64 = 0.12;
/// The hover path's opacity far from the slice (the stand-in fade, a look choice, R-390).
pub const FAINT: f32 = 0.22;
/// The hover path's loop size, as a fraction of the view's half-span (placeholder content).
pub const LOOP: f64 = 0.22;
/// How far the loop leaves the plane, as a fraction of its size, before the IC's own spread (placeholder content):
/// enough that part of it is faint and part solid.
pub const OUT_OF_PLANE: f64 = 0.35;

/// A number in `[0, 1)` from `x`: the fractional part of a large sine, a fixed hash (placeholder content).
pub fn hash(x: f64) -> f64 {
    let h = (x.sin() * 43_758.545_3).fract();
    if h < 0.0 {
        h + 1.0
    } else {
        h
    }
}

/// A fixed number in `[0, 1)` for `ic` and `salt`.
fn ic_hash(ic: &Latent, salt: f64) -> f64 {
    let weights = [12.989_8, 78.233, 37.719, 4.581, 19.19, 47.13, 93.989, 61.7];
    let k: f64 = ic.iter().zip(weights).map(|(z, w)| z * w).sum();
    hash(k + salt)
}

/// The outcome classes a fake sample may take, by param: the outcome palette's nine (colour_composition §1.4; R-444),
/// physical outcomes only, no "degenerate" and no "collision @ t=0" (R-445, R-446). Each keeps the slot the fake hash
/// picks that its class had before R-444 where it stayed; the triple outcomes take the two removed classes' slots
/// (placeholder content).
pub const FATES: [&str; 9] = [
    "collision_pair_2",
    "collision_pair_1",
    "collision_pair_0",
    "bounded",
    "triple_ejection",
    "escape_body_0",
    "escape_body_1",
    "escape_body_2",
    "triple_collision",
];

/// The fake sample at the figure point `at` of `plane`, with the playhead at `t` and the horizon `t_max`: its fate is
/// decided once the playhead reaches its `t_end` (`t_max` for `bounded`; no class is decided at `t = 0`, R-445).
pub fn sample(plane: &Plane, at: [f64; 2], t: f64, t_max: f64) -> Sample {
    integrate(point(plane, at[0], at[1]), at, t, t_max)
}

/// The fake sample of the IC `ic`, given at the figure point `at`, with the playhead at `t` and the horizon `t_max`:
/// the mock's `computeIC` (trajectory_viewing §1, §6), deterministic in the IC, so a pinned IC integrates the same on
/// any chart (R-453 D1, D5).
pub fn integrate(ic: Latent, at: [f64; 2], t: f64, t_max: f64) -> Sample {
    let class = FATES[(ic_hash(&ic, 0.0) * FATES.len() as f64) as usize % FATES.len()];
    let end = match class {
        "bounded" => t_max,
        _ => t_max * (0.1 + 0.8 * ic_hash(&ic, 1.0)),
    };
    let decided = t >= end;
    // The fate a person reads: the display name of the class's `state` and `detail` (R-449), "bounded" for bounded.
    let label = SWATCHES
        .iter()
        .find(|s| s.param == class)
        .and_then(|s| display::name_of(s.state, s.detail))
        .unwrap_or_default()
        .to_owned();
    let path_t = end.max(t_max * 0.1);
    let escaping = match class {
        "escape_body_0" => Some(0),
        "escape_body_1" => Some(1),
        "escape_body_2" => Some(2),
        _ => None,
    };
    let (theta0, omega) = (
        std::f64::consts::TAU * ic_hash(&ic, 2.0),
        0.6 + ic_hash(&ic, 3.0),
    );
    let times = (0..PATH_POINTS).map(|i| path_t * i as f64 / (PATH_POINTS - 1) as f64);
    let shape = times
        .clone()
        .map(|tau| {
            let theta = (theta0 + omega * tau).rem_euclid(std::f64::consts::TAU);
            let phi = std::f64::consts::FRAC_PI_2 + 1.1 * (0.37 * tau + theta0).sin();
            [theta, phi]
        })
        .collect();
    let bodies = std::array::from_fn(|k| {
        times
            .clone()
            .map(|tau| {
                let angle = omega * tau + std::f64::consts::TAU * k as f64 / 3.0;
                let mut r = 1.0 + 0.35 * (2.3 * tau + k as f64).sin();
                if escaping == Some(k) {
                    // The escaping body drifts out from 70% of its end on.
                    r += 0.4 * (tau - end * 0.7).max(0.0);
                }
                [r * angle.cos(), 0.7 * r * angle.sin()]
            })
            .collect()
    });
    Sample {
        at,
        ic,
        fate: decided.then_some(label),
        t_end: decided.then_some(end),
        ftle: 0.02 + 0.1 * ic_hash(&ic, 4.0),
        e0: -0.2 - 0.6 * ic_hash(&ic, 5.0),
        f2_word: f2_word(&ic),
        substeps: 1000 + (ic_hash(&ic, 6.0) * 40_000.0) as u64,
        min_separation: 1e-4 * (1.0 + 9.0 * ic_hash(&ic, 7.0)),
        energy_drift: 1e-10 * (1.0 + 9.0 * ic_hash(&ic, 8.0)),
        path_t,
        shape,
        bodies,
    }
}

/// A fake F₂ word of five letters, `a`, `b` and their inverses (placeholder content).
fn f2_word(ic: &Latent) -> String {
    let letters = ["a", "b", "a⁻¹", "b⁻¹"];
    (0..5)
        .map(|i| letters[(ic_hash(ic, 10.0 + i as f64) * 4.0) as usize % 4])
        .collect::<Vec<_>>()
        .join(" ")
}

/// One point of the hover path: where it falls on the figure, `(s, t)`, and its opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    /// The figure point, `(s, t)`, `t` Y-up.
    pub at: [f64; 2],
    /// Its opacity: 1 near the slice, falling to [`FAINT`] away from it.
    pub alpha: f32,
}

/// The opacity of a point `distance` off the slice, `half_span` the view's half-span: solid within [`NEAR`] of it,
/// fading linearly over [`FADE`] to [`FAINT`].
pub fn alpha(distance: f64, half_span: f64) -> f32 {
    let d = distance / half_span;
    let f = ((d - NEAR) / FADE).clamp(0.0, 1.0);
    (1.0 - f * (1.0 - f64::from(FAINT))) as f32
}

/// A unit direction out of `plane`'s span: the axis with the largest part off both basis vectors, made orthogonal to
/// them.
pub fn out_of_plane(plane: &Plane) -> Option<Latent> {
    let (q1, q2) = (&plane.q1, &plane.q2);
    let n1 = norm(q1);
    if n1 == 0.0 {
        return None;
    }
    let e1 = q1.map(|x| x / n1);
    let r2 = add(q2, -dot(q2, &e1), &e1);
    let m2 = norm(&r2);
    if m2 == 0.0 {
        return None;
    }
    let e2 = r2.map(|x| x / m2);
    // The axis whose part off the span is largest, the last of equals: of eight axes and a span of two, at least one
    // has a part of length √(6/8) or more, so the residual is never zero.
    let mut best = (0.0, [0.0; 8]);
    for i in 0..8 {
        let axis: Latent = std::array::from_fn(|j| if j == i { 1.0 } else { 0.0 });
        let r = add(&add(&axis, -dot(&axis, &e1), &e1), -dot(&axis, &e2), &e2);
        let m = norm(&r);
        if m >= best.0 {
            best = (m, r);
        }
    }
    let (m, r) = best;
    Some(r.map(|x| x / m))
}

/// The hover path of the IC at `at` on `plane`: a fake `z(τ)` through the manifold, a loop about the IC that leaves the
/// plane and comes back, projected onto the chart (render_gui_spec §G2), each point's opacity by its distance off the
/// slice. Empty for a degenerate basis.
pub fn hover_path(plane: &Plane, at: [f64; 2]) -> Vec<PathPoint> {
    let Some(n) = out_of_plane(plane) else {
        return Vec::new();
    };
    let ic = point(plane, at[0], at[1]);
    let half_span = norm(&plane.q1);
    let size = LOOP * half_span;
    let tilt = OUT_OF_PLANE * (0.5 + ic_hash(&ic, 20.0));
    (0..HOVER_POINTS)
        .filter_map(|i| {
            let tau = std::f64::consts::TAU * i as f64 / (HOVER_POINTS - 1) as f64;
            let distance = size * tilt * (1.0 - tau.cos());
            let z = add(
                &add(
                    &add(&ic, size * tau.sin(), &plane.q1.map(|x| x / half_span)),
                    size * 0.6 * (2.0 * tau).sin(),
                    &plane.q2.map(|x| x / norm(&plane.q2)),
                ),
                distance,
                &n,
            );
            let (s, t) = chart_coords(plane, &z)?;
            Some(PathPoint {
                at: [s, t],
                alpha: alpha(distance, half_span),
            })
        })
        .collect()
}
