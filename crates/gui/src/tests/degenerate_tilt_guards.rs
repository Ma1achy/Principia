//! `degenerate_tilt_guards` (REQ-GUI-203; R-451 B11; render_gui_spec § "The compass: look and views (R-451, R-452)",
//! §G4; chart_decoder_contract Part 3, "The lock"): the GUI's guards for a degenerate basis, `in_plane` and
//! `chart_coords`, keep their sine tests and add B11's scale-free test `s = 2√(g₁₁g₂₂ − g₁₂²) / (g₁₁ + g₂₂) < 10⁻⁶`
//! on the plane's own `q₁`, `q₂`. At the four corner points `τ₁ = ±τ₂ = ±90°` they decline, so a locked basis edit
//! sends no `z₀` edit and the mock's figure places nothing; everywhere else, the compass's 0.057° disc included, they
//! return what they did before this task, so a locked drag turns about the pin.
//!
//! Each check is also run, in its test, on an input it must reject, through [`rejects`] (gui has no route to
//! `negative_control!`, `support`'s note).

use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, SimConfig};

use crate::explore::manifold_view::{
    about_the_pin, add, axis, basis_edit, chart_coords, dot, in_plane, plane_s, View, PLANE_S_CUT,
    PRESETS, SLICE_AXIS,
};
use crate::mock::{quads, sample};
use crate::tests::support::{rejects, state};

/// The double-precision rounding unit, as the task states it.
const EPS: f64 = 2.22e-16;

/// The four corner points `(τ₁, τ₂)`, in degrees.
const CORNERS: [(f64, f64); 4] = [(90.0, 90.0), (90.0, -90.0), (-90.0, 90.0), (-90.0, -90.0)];

/// The σ sweep over [0.01, 4]: 41 points, log-spaced, both ends included.
fn sigmas() -> impl Iterator<Item = f64> {
    (0..=40).map(|i| 0.01 * 400f64.powf(f64::from(i) / 40.0))
}

/// The view `(τ₁, τ₂, γ)` in degrees at scale `σ`, in the first preset's frame `(e_a, e_b, e_d)`.
fn view(tau1: f64, tau2: f64, gamma: f64, sigma: f64) -> View {
    let (a, b) = PRESETS[0];
    View {
        preset: Some(0),
        a,
        b,
        d: SLICE_AXIS,
        x: SLICE_AXIS,
        y: SLICE_AXIS,
        scale: sigma,
        tau1: tau1.to_radians(),
        tau2: tau2.to_radians(),
        gamma: gamma.to_radians(),
    }
}

/// `View::basis`'s plane at `(τ₁, τ₂, γ)`, σ, through `z₀`.
fn plane_at(tau1: f64, tau2: f64, gamma: f64, sigma: f64, z0: Latent) -> Plane {
    let (q1, q2) = view(tau1, tau2, gamma, sigma).basis();
    Plane { z0, q1, q2 }
}

/// The latent point with components `(x, y, z)` on the frame `(e_a, e_b, e_d)`.
fn in_frame(x: f64, y: f64, z: f64) -> Latent {
    let (a, b) = PRESETS[0];
    add(
        &add(&axis(a, x), y, &axis(b, 1.0)),
        z,
        &axis(SLICE_AXIS, 1.0),
    )
}

/// The review's anchor `z_locked = (0.1, 0.05, 0.2)` in the frame.
fn review_anchor() -> Latent {
    in_frame(0.1, 0.05, 0.2)
}

/// The guards as they were before this task, their sine tests alone: the failing control's.
fn in_plane_before(plane: &Plane, z: &Latent) -> Option<(f64, f64)> {
    let norm = |v: &Latent| dot(v, v).sqrt();
    let (n1, n2) = (norm(&plane.q1), norm(&plane.q2));
    if n1 == 0.0 || n2 == 0.0 {
        return None;
    }
    let (h1, h2) = (plane.q1.map(|x| x / n1), plane.q2.map(|x| x / n2));
    let g = dot(&h1, &h2);
    let det = 1.0 - g * g;
    if det <= 1e-12 {
        return None;
    }
    let (c1, c2) = (dot(z, &h1), dot(z, &h2));
    Some(((c1 - g * c2) / det, (c2 - g * c1) / det))
}

/// `chart_coords` before this task.
fn chart_coords_before(plane: &Plane, z: &Latent) -> Option<(f64, f64)> {
    let r = add(z, -1.0, &plane.z0);
    let (g11, g12, g22) = (
        dot(&plane.q1, &plane.q1),
        dot(&plane.q1, &plane.q2),
        dot(&plane.q2, &plane.q2),
    );
    let det = g11 * g22 - g12 * g12;
    if det <= 1e-24 * (g11 * g22).max(f64::MIN_POSITIVE) {
        return None;
    }
    let (c1, c2) = (dot(&r, &plane.q1), dot(&r, &plane.q2));
    let (x, y) = ((g22 * c1 - g12 * c2) / det, (g11 * c2 - g12 * c1) / det);
    Some(((x + 1.0) / 2.0, (y + 1.0) / 2.0))
}

/// `about_the_pin`'s rule for one basis edit, on the guard `chart`: the `z₀` edit it sends first, if any.
fn pivot_edit(
    chart: fn(&Plane, &Latent) -> Option<(f64, f64)>,
    plane: &Plane,
    anchor: &Latent,
    (q1, q2): (Latent, Latent),
) -> Option<Latent> {
    let (s, t) = chart(plane, anchor)?;
    let z0 = add(
        &add(&plane.z0, 2.0 * s - 1.0, &add(&plane.q1, -1.0, &q1)),
        2.0 * t - 1.0,
        &add(&plane.q2, -1.0, &q2),
    );
    (z0 != plane.z0).then_some(z0)
}

/// The skeleton state on `plane`, locked at `anchor`.
fn locked(plane: &Plane, anchor: Latent) -> SimConfig {
    let (mut sim, _) = state(0.0);
    sim.plane = plane.clone();
    sim.lock = Lock {
        locked: true,
        z_locked: anchor,
    };
    sim
}

/// That `pinned`, `about_the_pin`'s answer to the one edit `edit`, is that edit alone: no `SetField` on `z₀`.
fn passes_unchanged(pinned: &[SetField], edit: &SetField, what: &str) {
    assert!(
        !pinned
            .iter()
            .any(|e| matches!(e.edit, Edit::Sim(SimField::Z0(_)))),
        "{what}: a SetField on z₀ was sent: {pinned:?}"
    );
    assert_eq!(pinned, std::slice::from_ref(edit), "{what}: the basis edit");
}

/// That both guards decline `plane` at the review's anchor and at `z₀`.
fn both_decline(plane: &Plane, what: &str) {
    for z in [review_anchor(), plane.z0] {
        assert_eq!(in_plane(plane, &z), None, "{what}: in_plane");
        assert_eq!(chart_coords(plane, &z), None, "{what}: chart_coords");
    }
}

/// That the mock's figure places nothing on `plane`: no quad, no bound, no figure point, no hover path.
fn figure_places_nothing(plane: &Plane, what: &str) {
    assert!(quads::quads(plane).is_empty(), "{what}: quads");
    assert!(
        quads::quads_over(plane, [-1.0, -1.0, 2.0, 2.0]).is_empty(),
        "{what}: quads_over"
    );
    assert!(quads::bounds(plane).is_empty(), "{what}: bounds");
    assert!(
        quads::bounds_over(plane, [-1.0, -1.0, 2.0, 2.0]).is_empty(),
        "{what}: bounds_over"
    );
    assert_eq!(
        quads::on_figure(plane, (0.1, 0.2)),
        None,
        "{what}: on_figure"
    );
    assert!(
        sample::hover_path(plane, [0.5, 0.5]).is_empty(),
        "{what}: hover_path"
    );
}

#[test]
fn degenerate_tilt_guards_decline_at_the_corners() {
    for (t1, t2) in CORNERS {
        for gamma in [0.0, 45.0] {
            for sigma in sigmas() {
                let what = format!("({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}");
                let plane = plane_at(t1, t2, gamma, sigma, [0.0; 8]);
                // B11's measured floor at the corner points: s up to 2.4·10⁻⁸, under √(17ε) ≈ 6.1·10⁻⁸.
                let s = plane_s(&plane.q1, &plane.q2);
                assert!(s < 6.1e-8, "{what}: s = {s} above its rounding floor");
                both_decline(&plane, &what);
                figure_places_nothing(&plane, &what);
                // With γ = 45° the sine tests alone do not fire here (B11: the sine between about 0.27 and 1): the
                // new cut is what declines.
                if gamma == 45.0 {
                    let z = review_anchor();
                    assert!(
                        in_plane_before(&plane, &z).is_some(),
                        "{what}: in_plane's sine test fired"
                    );
                    assert!(
                        chart_coords_before(&plane, &z).is_some(),
                        "{what}: chart_coords' sine test fired"
                    );
                }
            }
        }
    }
    // The control: a plane off the corners is accepted, and the figure places its quads.
    let sound = plane_at(89.0, 90.0, 45.0, 1.0, [0.0; 8]);
    rejects("a sound plane", || both_decline(&sound, "(89°, 90°, 45°)"));
    rejects("a sound plane's figure", || {
        figure_places_nothing(&sound, "(89°, 90°, 45°)")
    });
}

#[test]
fn degenerate_tilt_guards_return_todays_values_off_the_corners() {
    // (τ₁, τ₂, γ) in degrees, and B11's s where the task states it.
    let tilts: [(f64, f64, f64, Option<f64>); 12] = [
        (90.0, 89.9, 0.0, Some(1.75e-3)),
        (90.0, 89.97, 45.0, Some(5.2e-4)),
        (90.0, 89.999, 45.0, Some(1.75e-5)),
        (90.0, 89.97, 0.0, Some(5.2e-4)),
        (90.0, 89.999, 30.0, Some(1.75e-5)),
        (-89.97, 90.0, 45.0, None),
        (89.0, 90.0, 45.0, None),
        (90.0, 0.0, 45.0, None),
        (0.0, 0.0, 0.0, None),
        (30.0, 0.0, 0.0, None),
        (45.0, 45.0, 0.0, None),
        (-60.0, 20.0, 30.0, None),
    ];
    let z0 = in_frame(0.02, -0.03, 0.05);
    let points = [review_anchor(), z0, in_frame(-0.7, 0.4, 0.3)];
    for (t1, t2, gamma, s_stated) in tilts {
        // The zoom's deepest scale, 2⁻⁴⁸ (ZOOM_RANGE), among them: a sound basis that small is kept.
        for sigma in [2f64.powi(-48), 0.01, 1.0, 4.0] {
            let what = format!("({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}");
            let plane = plane_at(t1, t2, gamma, sigma, z0);
            let s = plane_s(&plane.q1, &plane.q2);
            assert!(s >= PLANE_S_CUT, "{what}: s = {s} under the cut");
            if let Some(stated) = s_stated {
                assert!(
                    (s - stated).abs() <= 0.01 * stated,
                    "{what}: s = {s}, the task's {stated}"
                );
            }
            for z in &points {
                let (now, before) = (in_plane(&plane, z), in_plane_before(&plane, z));
                assert!(now.is_some(), "{what}: in_plane declined");
                assert_eq!(now, before, "{what}: in_plane");
                let (now, before) = (chart_coords(&plane, z), chart_coords_before(&plane, z));
                assert!(now.is_some(), "{what}: chart_coords declined");
                assert_eq!(now, before, "{what}: chart_coords");
            }
        }
    }
    // The control: at a corner, today's values are not returned.
    let corner = plane_at(90.0, 90.0, 45.0, 1.0, z0);
    rejects("a corner", || {
        assert_eq!(
            chart_coords(&corner, &points[0]),
            chart_coords_before(&corner, &points[0])
        )
    });
}

#[test]
fn degenerate_tilt_guards_keep_a_typed_basis_of_unequal_norms() {
    // q₁ = e_x, q₂ = 10⁻⁴ e_y: orthogonal, norm ratio r = 10⁻⁴, s = 2r/(1 + r²) ≈ 2·10⁻⁴, under the compass's 10⁻³ cut.
    let plane = Plane {
        z0: [0.0; 8],
        q1: axis(0, 1.0),
        q2: axis(1, 1e-4),
    };
    let s = plane_s(&plane.q1, &plane.q2);
    assert!((s / (2e-4 / (1.0 + 1e-8)) - 1.0).abs() <= 1e-12, "s = {s}");
    let mut z = [0.0; 8];
    (z[0], z[1]) = (0.5, 2e-5);
    let check = |plane: &Plane| {
        assert_eq!(chart_coords(plane, &z), Some((0.75, 0.6)), "chart_coords");
        assert_eq!(in_plane(plane, &z), Some((0.5, 2e-5)), "in_plane");
    };
    check(&plane);
    // The control: a 10⁻³ cut, the compass's, would refuse this basis.
    rejects("the compass's cut", || {
        assert!(s >= 1e-3, "s = {s} under the compass's cut")
    });
    // And with the norm ratio under 5·10⁻⁷, s < 10⁻⁶: the guards decline, a basis edit sends no z₀ edit.
    let thin = Plane {
        q2: axis(1, 4e-7),
        ..plane.clone()
    };
    assert!(plane_s(&thin.q1, &thin.q2) < PLANE_S_CUT);
    both_decline(&thin, "q₂ = 4·10⁻⁷ e_y");
    rejects("a basis under the cut", || check(&thin));

    // The cut's boundary: q₂ = r e_y with r = 5.00000000000125·10⁻⁷ reads s = 10⁻⁶ exactly, and the guards return
    // values there (s ≥ the cut); one ulp less of r reads s under it, and they decline.
    let r = 5.00000000000125e-7;
    let at = Plane {
        q2: axis(1, r),
        ..plane.clone()
    };
    assert_eq!(plane_s(&at.q1, &at.q2), PLANE_S_CUT);
    let mut w = [0.0; 8];
    (w[0], w[1]) = (0.5, 0.2 * r);
    assert!(in_plane(&at, &w).is_some(), "in_plane at the cut");
    assert!(chart_coords(&at, &w).is_some(), "chart_coords at the cut");
    let under = Plane {
        q2: axis(1, f64::from_bits(r.to_bits() - 1)),
        ..plane.clone()
    };
    assert!(plane_s(&under.q1, &under.q2) < PLANE_S_CUT);
    assert_eq!(in_plane(&under, &w), None, "in_plane under the cut");
    assert_eq!(chart_coords(&under, &w), None, "chart_coords under the cut");
    rejects("the basis under the cut", || {
        assert!(chart_coords(&under, &w).is_some())
    });
}

#[test]
fn degenerate_tilt_guards_s_is_scale_free() {
    let bases = [
        view(90.0, 89.97, 45.0, 1.0).basis(),
        view(90.0, 89.999, 30.0, 1.0).basis(),
        view(-60.0, 20.0, 30.0, 1.0).basis(),
        view(90.0, 90.0, 45.0, 1.0).basis(),
        (axis(0, 1.0), axis(1, 1e-4)),
        (
            [0.3, -0.1, 0.7, 0.2, -0.5, 0.05, 0.9, -0.4],
            [0.1, 0.6, -0.2, 0.3, 0.25, -0.8, 0.1, 0.45],
        ),
    ];
    let scaled =
        |(q1, q2): &(Latent, Latent), k: f64| plane_s(&q1.map(|x| k * x), &q2.map(|x| k * x));
    for (i, basis) in bases.iter().enumerate() {
        let s = plane_s(&basis.0, &basis.1);
        // A power of two scales exactly; any other scale moves s² by rounding alone, B11's 17ε for each of the two.
        for k in [0.25, 2.0, 1024.0, 2f64.powi(-48)] {
            assert_eq!(scaled(basis, k), s, "basis {i}, k = {k}");
        }
        for k in [0.01, 3.0, 1e3, 7.3e-9] {
            let sk = scaled(basis, k);
            assert!(
                (sk * sk - s * s).abs() <= 2.0 * 17.0 * EPS,
                "basis {i}, k = {k}: {sk} against {s}"
            );
        }
    }
    // s ≤ sin θ, and the zero basis reads 0.
    let (q1, q2) = &bases[5];
    let cos = dot(q1, q2) / (dot(q1, q1) * dot(q2, q2)).sqrt();
    assert!(plane_s(q1, q2) <= (1.0 - cos * cos).sqrt());
    assert_eq!(plane_s(&[0.0; 8], &[0.0; 8]), 0.0);
    // A parallel pair reads s at its rounding floor, never NaN: a g₁₁g₂₂ − g₁₂² rounded below 0 reads 0.
    let mut negative = 0;
    for k in 1..=200 {
        let q2 = q1.map(|x| x * (0.37 + 0.031 * f64::from(k)));
        let (g11, g12, g22) = (dot(q1, q1), dot(q1, &q2), dot(&q2, &q2));
        negative += usize::from(g11 * g22 - g12 * g12 < 0.0);
        let s = plane_s(q1, &q2);
        assert!((0.0..6.1e-8).contains(&s), "k = {k}: s = {s}");
    }
    assert!(negative > 0, "no parallel pair rounded below 0");
    // The control: scaling one vector alone changes s.
    rejects("one vector scaled", || {
        let (q1, q2) = &bases[2];
        assert_eq!(plane_s(&q1.map(|x| 2.0 * x), q2), plane_s(q1, q2));
    });
}

#[test]
fn degenerate_tilt_guards_locked_at_a_corner_send_no_z0_edit() {
    // The review's case: z₀ = 0, z_locked = (0.1, 0.05, 0.2), the corner (90°, 90°, γ = 45°), σ = 1, a tilt drag to
    // (89°, 90°, γ = 45°).
    let corner = plane_at(90.0, 90.0, 45.0, 1.0, [0.0; 8]);
    let drag = view(89.0, 90.0, 45.0, 1.0).basis();
    let edit = basis_edit(drag);
    let sim = locked(&corner, review_anchor());
    passes_unchanged(
        &about_the_pin(vec![edit.clone()], &sim),
        &edit,
        "the review's case",
    );
    // The failing control: today's guards do not fire here. Their pivot is rounding, t ≈ −2.887·10¹⁴, and the drag's
    // z₀ edit is about (−7.13·10¹², 0, 6.22·10¹⁰) in the frame.
    let (_, t) = chart_coords_before(&corner, &review_anchor()).expect("today's guard fires");
    assert!((t / -2.887e14 - 1.0).abs() < 1e-3, "t = {t}");
    let z0 = pivot_edit(chart_coords_before, &corner, &review_anchor(), drag)
        .expect("today's guards send a z₀ edit");
    let (a, b) = PRESETS[0];
    let frame = [z0[a], z0[b], z0[SLICE_AXIS]];
    assert!((frame[0] / -7.13e12 - 1.0).abs() < 1e-3, "{frame:?}");
    assert!(frame[1].abs() < 1e-3, "{frame:?}");
    assert!((frame[2] / 6.22e10 - 1.0).abs() < 1e-3, "{frame:?}");
    let before = vec![
        SetField {
            edit: Edit::Sim(SimField::Z0(z0)),
            no_history: false,
        },
        edit.clone(),
    ];
    rejects("today's guards", || {
        passes_unchanged(&before, &edit, "today's guards")
    });

    // Likewise at each corner, γ = 0 and 45°, over the σ sweep, the drag one degree back off τ₁'s ±90°.
    for (t1, t2) in CORNERS {
        for gamma in [0.0, 45.0] {
            for sigma in sigmas() {
                let what = format!("({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}");
                let plane = plane_at(t1, t2, gamma, sigma, [0.0; 8]);
                let edit = basis_edit(view(t1 - t1.signum(), t2, gamma, sigma).basis());
                let pinned = about_the_pin(vec![edit.clone()], &locked(&plane, review_anchor()));
                passes_unchanged(&pinned, &edit, &what);
            }
        }
    }
}

#[test]
fn degenerate_tilt_guards_locked_inside_the_disc_turn_about_the_pin() {
    // (τ₂, γ) in degrees, and the task's stated tolerance 17ε(1/sin²θ + 1/sin²θ′).
    for (t2, gamma, stated) in [
        (89.97, 45.0, 7.6e-15),
        (89.97, 0.0, 1.4e-8),
        (89.999, 30.0, 3.1e-6),
    ] {
        let what = format!("(90°, {t2}°, γ = {gamma}°)");
        let plane = plane_at(90.0, t2, gamma, 1.0, [0.0; 8]);
        let after = view(89.0, t2, gamma, 1.0).basis();
        let sin = |(q1, q2): &(Latent, Latent)| {
            let cos = dot(q1, q2) / (dot(q1, q1) * dot(q2, q2)).sqrt();
            (1.0 - cos * cos).sqrt()
        };
        let (sin_t, sin_t2) = (sin(&(plane.q1, plane.q2)), sin(&after));
        let tolerance = 17.0 * EPS * (1.0 / (sin_t * sin_t) + 1.0 / (sin_t2 * sin_t2));
        assert!(
            (tolerance / stated - 1.0).abs() < 0.1,
            "{what}: tolerance {tolerance}, the task's {stated}"
        );
        // The pin z_locked = 0.3q₁ + 0.2q₂, chart point (0.65, 0.60).
        let anchor = add(&add(&plane.z0, 0.3, &plane.q1), 0.2, &plane.q2);
        let edit = basis_edit(after);
        let pinned = about_the_pin(vec![edit.clone()], &locked(&plane, anchor));
        let [first, second] = pinned.as_slice() else {
            panic!("{what}: a z₀ edit, then the basis edit: {pinned:?}");
        };
        let Edit::Sim(SimField::Z0(z0)) = first.edit else {
            panic!("{what}: the z₀ edit first: {pinned:?}");
        };
        assert_eq!(*second, edit, "{what}: the basis edit as it was");
        let moved = Plane {
            z0,
            q1: after.0,
            q2: after.1,
        };
        let held = |plane: &Plane| {
            let (s, t) = chart_coords(plane, &anchor).expect("a plane");
            let off = (s - 0.65).hypot(t - 0.60);
            assert!(
                off <= tolerance,
                "{what}: the pin moved {off}, over {tolerance}"
            );
        };
        held(&moved);
        // The control: the drag with no z₀ edit, the pivot declined, moves the pin off its chart point.
        rejects("the pivot declined", || {
            held(&Plane {
                z0: plane.z0,
                q1: after.0,
                q2: after.1,
            })
        });
    }
}
