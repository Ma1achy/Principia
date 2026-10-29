//! Navigation as chart construction (chart_decoder_contract Part 4; chart_reference §1.1; TASK-M2-22): each gesture
//! is a typed `SetField` edit of `SimConfig`'s plane, applied here through the real state store, and every pixel's IC
//! is read through the kernel's latent chart `Φ`.
//!
//! - `nav_ops` (REQ-GUI-002): a slice moves every pixel's IC by the identical hidden step; a pan keeps the plane; a
//!   zoom keeps `z₀`.
//! - `tilt` (REQ-GUI-003): `τ = 0` leaves `q`, `τ = ±90°` gives `±d_target`, `+30°` and `−30°` differ, a curve axis
//!   tilts its tangent at the centre, and the six directions orthogonal to the plane; a pair tilt holds `τ₁` and `τ₂`
//!   together, and re-tilting one replaces its own tilt.
//! - `named_directions` (REQ-CHART-009, REQ-CHART-051): the library returns the three named vectors, the energy vector
//!   moves with the centre, and the morph keeps the anchor through the §4.6 persistence sweep.
//! - `oblique_slice_record` (REQ-CHART-016): the export of an oblique slice carries `q₁, q₂`; two raw tilts give the
//!   second, not the composition.

use engine::contract::canonical;
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use engine::contract::store::StateStore;
use engine::nav::directions::{
    energy_at_fixed_lz, hidden, library, mass_away_from_burrau, morph, morph_source, MorphInto,
    Unavailable,
};
use engine::nav::{
    oblique, off_plane, pan, slice, tilt_tangent, tilt_vector, zoom, BasisVector, ChartConstructor,
    NavError,
};
use kernel::chart::{Chart as _, Latent as LatentChart};
use kernel::decode::descriptor::{kinetic, potential};
use kernel::decode::latent::Latent as Controls;
use kernel::decode::{decode, invariant};
use kernel::generated::links::{
    Defaults, Link, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh, StickBreaking,
};
use validation::negative_control;

/// `ê_k`.
fn e(k: usize) -> Latent {
    let mut v = [0.0; 8];
    v[k] = 1.0;
    v
}

fn dot(a: &Latent, b: &Latent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &Latent) -> f64 {
    dot(a, a).sqrt()
}

fn sub(a: &Latent, b: &Latent) -> Latent {
    std::array::from_fn(|i| a[i] - b[i])
}

fn scale(a: &Latent, k: f64) -> Latent {
    a.map(|x| k * x)
}

fn close(a: &Latent, b: &Latent, tol: f64) -> bool {
    norm(&sub(a, b)) <= tol
}

fn sim(plane: Plane) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
        },
        plane,
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn store(plane: Plane) -> StateStore {
    StateStore::new(
        sim(plane),
        RenderState {
            stain_graph: StainGraph::new(),
            overlays: Overlays::default(),
            hover: Hover::default(),
            palette: Palette {},
            playhead: Playhead { t: 0.0 },
            debug_view: false,
        },
    )
}

/// The plane after `edit`, applied through the real state store.
fn apply(plane: &Plane, edit: SetField) -> Plane {
    let mut s = store(plane.clone());
    s.set_field(edit);
    s.snapshot().sim.plane
}

/// The pixel `(s, t)`'s IC: the kernel's latent chart `Φ` at f64.
fn ic(plane: &Plane, s: f64, t: f64) -> Latent {
    LatentChart {
        z0: plane.z0,
        q1: plane.q1,
        q2: plane.q2,
    }
    .map::<f64>(s, t)
    .z
    .z
}

/// The pixel grid the checks read, past `[0,1]²` on both sides as well.
const GRID: [f64; 6] = [-0.5, 0.0, 0.25, 0.5, 0.75, 1.5];

/// An oblique, zoomed plane through an off-origin centre.
fn oblique_plane() -> Plane {
    let a = [0.3, -1.1, 0.4, 0.2, -0.7, 0.9, 0.05, -0.3];
    let b = [-0.6, 0.2, 1.3, -0.4, 0.1, 0.5, -0.8, 0.25];
    let z0 = [0.4, -0.2, 0.1, -0.3, 0.2, 0.05, 0.6, -0.5];
    let p = oblique(z0, &a, &b).expect("independent draws span a plane");
    Plane {
        z0: p.z0,
        q1: scale(&p.q1, 0.3),
        q2: scale(&p.q2, 0.3),
    }
}

/// Burrau masses `(5, 4, 3)/12` (chart_reference §4.2, `(m, n) = (2, 1)`) as mass controls, with a configuration and
/// moving momenta.
fn burrau_centre() -> Latent {
    let [z6, z7] = <SoftmaxTanh as Link<2, 3>>::inverse([5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0]);
    [0.3, -0.2, 0.1, -0.2, 0.3, 0.05, z6, z7]
}

// ---- nav_ops (REQ-GUI-002) ----

/// Every pixel's IC moved from `before` to `after` by `step`, and `step` is orthogonal to `before`'s plane.
fn check_same_hidden_step(before: &Plane, after: &Plane, step: &Latent) {
    for s in GRID {
        for t in GRID {
            let moved = sub(&ic(after, s, t), &ic(before, s, t));
            assert!(
                close(&moved, step, 1e-12),
                "pixel ({s}, {t}) moved by {moved:?}, not the identical hidden step {step:?}"
            );
        }
    }
    let in_plane = [dot(step, &before.q1), dot(step, &before.q2)];
    assert!(
        in_plane.iter().all(|x| x.abs() <= 1e-12),
        "the step has the in-plane components {in_plane:?}"
    );
}

#[test]
fn nav_ops_slice_moves_every_pixel_by_the_same_hidden_step() {
    let before = oblique_plane();
    let d = hidden(&before).expect("a plane")[2].direction;
    let after = apply(&before, slice(&before, &d, 0.37).expect("a slice"));
    assert_eq!((after.q1, after.q2), (before.q1, before.q2));
    check_same_hidden_step(&before, &after, &scale(&d, 0.37));
}

negative_control!(
    nav_ops_slice_moves_every_pixel_by_the_same_hidden_step,
    "a zoom in place of the slice moves each pixel by a different amount",
    expected = "not the identical hidden step",
    {
        let before = oblique_plane();
        let d = hidden(&before).expect("a plane")[2].direction;
        let after = apply(&before, zoom(&before, 1.0).expect("a zoom"));
        check_same_hidden_step(&before, &after, &scale(&d, 0.37));
    }
);

#[test]
fn nav_ops_slice_steps_by_the_orthogonal_component() {
    let before = oblique_plane();
    let h = hidden(&before).expect("a plane")[0].direction;
    let d: Latent = std::array::from_fn(|i| h[i] + 2.0 * before.q1[i] - before.q2[i]);
    let after = apply(&before, slice(&before, &d, -0.5).expect("a slice"));
    check_same_hidden_step(&before, &after, &scale(&h, -0.5));
}

negative_control!(
    nav_ops_slice_steps_by_the_orthogonal_component,
    "a centre move with an in-plane part is not a slice",
    expected = "in-plane components",
    {
        let before = oblique_plane();
        let step = scale(&before.q1, 0.5);
        let after = Plane {
            z0: std::array::from_fn(|i| before.z0[i] + step[i]),
            ..before.clone()
        };
        check_same_hidden_step(&before, &after, &step);
    }
);

/// `after` is `before`'s plane panned by `(ds, dt)`: the basis kept, `Δz₀` in the plane, and each pixel's IC the one
/// `(ds, dt)` away before.
fn check_pan(before: &Plane, after: &Plane, ds: f64, dt: f64) {
    assert_eq!(
        (after.q1, after.q2),
        (before.q1, before.q2),
        "the basis changed"
    );
    let off = off_plane(before, &sub(&after.z0, &before.z0)).expect("a plane");
    assert!(norm(&off) <= 1e-12, "the pan left the plane by {off:?}");
    for s in GRID {
        for t in GRID {
            assert!(
                close(&ic(after, s, t), &ic(before, s + ds, t + dt), 1e-12),
                "pixel ({s}, {t}) is not the pixel ({}, {}) before",
                s + ds,
                t + dt
            );
        }
    }
}

#[test]
fn nav_ops_pan_keeps_the_plane() {
    let before = oblique_plane();
    let after = apply(&before, pan(&before, 0.1, -0.2).expect("a pan"));
    check_pan(&before, &after, 0.1, -0.2);
}

negative_control!(
    nav_ops_pan_keeps_the_plane,
    "a slice in place of the pan leaves the plane",
    expected = "the pan left the plane",
    {
        let before = oblique_plane();
        let d = hidden(&before).expect("a plane")[0].direction;
        let after = apply(&before, slice(&before, &d, 0.1).expect("a slice"));
        check_pan(&before, &after, 0.1, -0.2);
    }
);

/// `after` is `before` zoomed by the common factor `k`: `z₀` kept, `q₁` and `q₂` each scaled by `k`.
fn check_zoom(before: &Plane, after: &Plane, k: f64) {
    assert_eq!(after.z0, before.z0, "the zoom moved z₀");
    assert!(
        close(&after.q1, &scale(&before.q1, k), 1e-15),
        "q₁ not scaled by {k}"
    );
    assert!(
        close(&after.q2, &scale(&before.q2, k), 1e-15),
        "q₂ not scaled by {k}"
    );
}

#[test]
fn nav_ops_zoom_keeps_z0() {
    let before = oblique_plane();
    let after = apply(&before, zoom(&before, 1.5).expect("a zoom"));
    check_zoom(&before, &after, 2f64.powf(-1.5));
    let out = apply(&before, zoom(&before, -2.0).expect("a zoom"));
    check_zoom(&before, &out, 4.0);
    let none = apply(&before, zoom(&before, 0.0).expect("a zoom"));
    assert_eq!(none, before);
}

negative_control!(
    nav_ops_zoom_keeps_z0,
    "a pan in place of the zoom moves z₀",
    expected = "the zoom moved z₀",
    {
        let before = oblique_plane();
        let after = apply(&before, pan(&before, 0.1, 0.0).expect("a pan"));
        check_zoom(&before, &after, 1.0);
    }
);

#[test]
fn nav_ops_refuse_what_is_not_finite_or_spans_no_plane() {
    let p = oblique_plane();
    assert_eq!(pan(&p, f64::NAN, 0.0), Err(NavError::NonFinite));
    assert_eq!(pan(&p, 0.0, f64::INFINITY), Err(NavError::NonFinite));
    assert_eq!(slice(&p, &e(0), f64::NAN), Err(NavError::NonFinite));
    assert_eq!(slice(&p, &[f64::NAN; 8], 1.0), Err(NavError::NonFinite));
    assert_eq!(zoom(&p, f64::INFINITY), Err(NavError::NonFinite));
    // 2^−1075 is 0 and 2^1025 is infinite: no plane.
    assert_eq!(zoom(&p, 1075.0), Err(NavError::DegeneratePlane));
    assert_eq!(zoom(&p, -1025.0), Err(NavError::DegeneratePlane));
    assert!(zoom(&p, 1074.0).is_ok() && zoom(&p, -1023.0).is_ok());
    let flat = Plane {
        z0: p.z0,
        q1: p.q1,
        q2: scale(&p.q1, 2.0),
    };
    assert_eq!(slice(&flat, &e(0), 1.0), Err(NavError::DegeneratePlane));
    let zero = Plane {
        z0: p.z0,
        q1: [0.0; 8],
        q2: p.q2,
    };
    assert_eq!(hidden(&zero), Err(NavError::DegeneratePlane));
}

negative_control!(
    nav_ops_refuse_what_is_not_finite_or_spans_no_plane,
    "a finite pan is accepted",
    expected = "NonFinite",
    {
        let p = oblique_plane();
        assert_eq!(pan(&p, 0.0, 0.0), Err(NavError::NonFinite));
    }
);

/// Whether `q₁ = ê₀`, `q₂ = ê₀ + δ·ê₁` spans a plane.
fn spans(delta: f64) -> bool {
    let p = Plane {
        z0: [0.0; 8],
        q1: e(0),
        q2: std::array::from_fn(|i| e(0)[i] + delta * e(1)[i]),
    };
    slice(&p, &e(2), 1.0).is_ok()
}

/// `q₂ = ê₀ + δ·ê₁` with `q₁ = ê₀` spans no plane.
fn check_refused(delta: f64) {
    assert!(!spans(delta), "the plane at δ = {delta} is accepted");
}

#[test]
fn nav_ops_plane_needs_a_conditioned_second_direction() {
    // q₂'s part orthogonal to q₁ must exceed √ε·‖q₂‖, ε the f64 machine epsilon.
    let root = f64::EPSILON.sqrt();
    assert!(spans(1.01 * root), "a plane just above √ε is refused");
    assert!(spans(1e-3));
    check_refused(0.99 * root);
    check_refused(0.0);
}

negative_control!(
    nav_ops_plane_needs_a_conditioned_second_direction,
    "a plane just above √ε spans",
    expected = "is accepted",
    check_refused(1.01 * f64::EPSILON.sqrt())
);

// ---- tilt (REQ-GUI-003) ----

#[test]
fn tilt_zero_leaves_q_unchanged() {
    let q = e(0);
    let d = e(5);
    assert!(close(&tilt_vector(&q, 0.0, &d).expect("a tilt"), &q, 1e-15));
    let p = oblique_plane();
    let c = ChartConstructor::new(p.clone());
    let d = hidden(&p).expect("a plane")[1].direction;
    let after = apply(&p, c.tilt(&p, BasisVector::Q2, 0.0, &d).expect("a tilt"));
    assert!(close(&after.q2, &p.q2, 1e-15), "τ = 0 changed q₂");
    assert!(close(&after.q1, &p.q1, 1e-15), "τ = 0 changed q₁");
    assert_eq!(after.z0, p.z0);
}

negative_control!(
    tilt_zero_leaves_q_unchanged,
    "a tilt of 1° changes q",
    expected = "τ = 0 changed q₂",
    {
        let p = oblique_plane();
        let c = ChartConstructor::new(p.clone());
        let d = hidden(&p).expect("a plane")[1].direction;
        let after = apply(&p, c.tilt(&p, BasisVector::Q2, 1.0, &d).expect("a tilt"));
        assert!(close(&after.q2, &p.q2, 1e-15), "τ = 0 changed q₂");
    }
);

/// `tilt_vector` of the unit `q` at `tau` toward `d` is `want`.
fn check_tilt(q: &Latent, tau: f64, d: &Latent, want: &Latent) {
    let got = tilt_vector(q, tau, d).expect("a tilt");
    assert!(
        close(&got, want, 1e-15),
        "τ = {tau}° gave {got:?}, not {want:?}"
    );
}

#[test]
fn tilt_ninety_gives_plus_or_minus_the_target() {
    let q = e(1);
    let d = e(6);
    check_tilt(&q, 90.0, &d, &d);
    check_tilt(&q, -90.0, &d, &scale(&d, -1.0));
    // A target of any length is its direction.
    check_tilt(&q, 90.0, &scale(&d, 3.0), &d);
}

negative_control!(
    tilt_ninety_gives_plus_or_minus_the_target,
    "−90° does not give +d_target",
    expected = "τ = -90° gave",
    check_tilt(&e(1), -90.0, &e(6), &e(6))
);

/// The planes `span(a.q₁, a.q₂)` and `span(b.q₁, b.q₂)` differ: some basis vector of `b` leaves `a`'s plane.
fn check_planes_differ(a: &Plane, b: &Plane) {
    let off = [&b.q1, &b.q2].map(|q| norm(&off_plane(a, q).expect("a plane")));
    assert!(off.iter().any(|x| *x > 1e-3), "the same plane: {off:?}");
}

/// The committed plane `p` with `q₂` tilted by `tau` toward `d`.
fn tilted(p: &Plane, tau: f64, d: &Latent) -> Plane {
    let c = ChartConstructor::new(p.clone());
    apply(p, c.tilt(p, BasisVector::Q2, tau, d).expect("a tilt"))
}

#[test]
fn tilt_plus_and_minus_thirty_give_different_planes() {
    let p = oblique_plane();
    let d = hidden(&p).expect("a plane")[3].direction;
    let plus = tilted(&p, 30.0, &d);
    let minus = tilted(&p, -30.0, &d);
    check_planes_differ(&plus, &minus);
    check_planes_differ(&p, &plus);
    // Both keep the centre.
    assert_eq!(ic(&plus, 0.5, 0.5), p.z0);
    assert_eq!(ic(&minus, 0.5, 0.5), p.z0);
}

negative_control!(
    tilt_plus_and_minus_thirty_give_different_planes,
    "one plane is not different from itself",
    expected = "the same plane",
    {
        let p = oblique_plane();
        let d = hidden(&p).expect("a plane")[3].direction;
        let plus = tilted(&p, 30.0, &d);
        check_planes_differ(&plus, &plus.clone());
    }
);

/// A curve through the latent space, `γ(ν) = (cos ν, sin ν, ν², 0, …)`, and its tangent `γ′(ν)`.
fn gamma_prime(nu: f64) -> Latent {
    [-nu.sin(), nu.cos(), 2.0 * nu, 0.0, 0.0, 0.0, 0.0, 0.0]
}

/// The tilt of the curve axis at the centre `ν₀` by `tau` toward `d` is the rotation of the unit tangent there.
fn check_curve_tilt(tangent: &Latent, nu0: f64, tau: f64, d: &Latent) {
    let at = gamma_prime(nu0);
    let t_hat = scale(&at, 1.0 / norm(&at));
    let (sin, cos) = tau.to_radians().sin_cos();
    let want: Latent = std::array::from_fn(|i| cos * t_hat[i] + sin * d[i]);
    let want = scale(&want, 0.5 / norm(&want));
    let got = tilt_tangent(tangent, 0.5, tau, d).expect("a tilt");
    assert!(
        close(&got, &want, 1e-14),
        "the curve axis did not tilt its tangent at ν₀ = {nu0}: {got:?}"
    );
}

#[test]
fn tilt_curve_axis_rotates_its_tangent_at_the_centre() {
    let nu0 = 0.7;
    let d = e(4);
    for tau in [-90.0, -30.0, 0.0, 30.0, 90.0] {
        check_curve_tilt(&gamma_prime(nu0), nu0, tau, &d);
    }
}

negative_control!(
    tilt_curve_axis_rotates_its_tangent_at_the_centre,
    "the tangent at another point of the curve is not the centre's",
    expected = "did not tilt its tangent",
    check_curve_tilt(&gamma_prime(0.2), 0.7, 30.0, &e(4))
);

#[test]
fn tilt_range_is_minus_ninety_to_ninety() {
    let (q, d) = (e(0), e(3));
    assert!(tilt_vector(&q, 90.0, &d).is_ok());
    assert!(tilt_vector(&q, -90.0, &d).is_ok());
    assert_eq!(tilt_vector(&q, 90.000001, &d), Err(NavError::TauOutOfRange));
    assert_eq!(
        tilt_vector(&q, -90.000001, &d),
        Err(NavError::TauOutOfRange)
    );
    assert_eq!(tilt_vector(&q, f64::NAN, &d), Err(NavError::NonFinite));
    assert_eq!(
        tilt_tangent(&q, f64::NAN, 0.0, &d),
        Err(NavError::NonFinite)
    );
    assert_eq!(tilt_vector(&q, 10.0, &[0.0; 8]), Err(NavError::ZeroVector));
    // cos 45°·q̂ + sin 45°·(−q̂) = 0: no direction.
    assert_eq!(
        tilt_vector(&q, 45.0, &scale(&q, -1.0)),
        Err(NavError::ZeroVector)
    );
}

negative_control!(
    tilt_range_is_minus_ninety_to_ninety,
    "90° itself is in range",
    expected = "TauOutOfRange",
    assert_eq!(
        tilt_vector(&e(0), 90.0, &e(3)).map(|_| ()),
        Err(NavError::TauOutOfRange)
    )
);

#[test]
fn tilt_keeps_the_zoom_and_refuses_a_flat_plane() {
    let p = oblique_plane();
    let d = hidden(&p).expect("a plane")[0].direction;
    let after = tilted(&p, 50.0, &d);
    assert!(
        (norm(&after.q2) - 0.3).abs() <= 1e-15,
        "the tilt changed |q₂|"
    );
    assert!(
        (norm(&after.q1) - 0.3).abs() <= 1e-15,
        "the tilt changed |q₁|"
    );
    // Tilting q₂ fully into q₁'s direction leaves no plane.
    let c = ChartConstructor::new(p.clone());
    assert_eq!(
        c.tilt(&p, BasisVector::Q2, 90.0, &p.q1),
        Err(NavError::DegeneratePlane)
    );
}

negative_control!(
    tilt_keeps_the_zoom_and_refuses_a_flat_plane,
    "a unit tilt of a zoomed basis vector loses the zoom",
    expected = "the tilt changed |q₂|",
    {
        let p = oblique_plane();
        let d = hidden(&p).expect("a plane")[0].direction;
        let q2 = tilt_tangent(&p.q2, 1.0, 50.0, &d).expect("a tilt");
        assert!((norm(&q2) - 0.3).abs() <= 1e-15, "the tilt changed |q₂|");
    }
);

/// The six hidden directions of `p` are orthonormal and orthogonal to its plane.
fn check_hidden(p: &Plane) {
    let h = hidden(p).expect("a plane");
    for (i, a) in h.iter().enumerate() {
        for (j, b) in h.iter().enumerate() {
            let want = if i == j { 1.0 } else { 0.0 };
            let got = dot(&a.direction, &b.direction);
            assert!((got - want).abs() <= 1e-12, "hidden {i}·{j} = {got}");
        }
        for q in [&p.q1, &p.q2] {
            let got = dot(&a.direction, q);
            assert!(
                got.abs() <= 1e-12,
                "hidden {i} is not orthogonal to the plane: {got}"
            );
        }
    }
}

#[test]
fn tilt_offers_the_six_directions_orthogonal_to_the_plane() {
    let axis = Plane {
        z0: [0.0; 8],
        q1: e(0),
        q2: scale(&e(3), 0.5),
    };
    let h = hidden(&axis).expect("a plane");
    assert_eq!(h.map(|x| x.source), [1, 2, 4, 5, 6, 7]);
    for x in h {
        assert_eq!(x.direction, e(x.source));
    }
    check_hidden(&axis);
    check_hidden(&oblique_plane());
}

negative_control!(
    tilt_offers_the_six_directions_orthogonal_to_the_plane,
    "the plane's own direction is not orthogonal to it",
    expected = "is not orthogonal to the plane",
    {
        let axis = Plane {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(3),
        };
        // The plane the directions are checked against holds ê₁, one of the directions.
        let h = hidden(&axis).expect("a plane");
        let other = Plane {
            q2: h[0].direction,
            ..axis
        };
        for q in [&other.q1, &other.q2] {
            let got = dot(&h[0].direction, q);
            assert!(
                got.abs() <= 1e-12,
                "hidden 0 is not orthogonal to the plane: {got}"
            );
        }
    }
);

/// `after` holds both angles: `q₁` is the committed `p.q1` turned by `tau1` toward `d1`, `q₂` the committed `p.q2`
/// turned by `tau2` toward `d2`, each alone.
fn check_both_held(
    p: &Plane,
    after: &Plane,
    (tau1, d1): (f64, &Latent),
    (tau2, d2): (f64, &Latent),
) {
    let want1 = tilt_vector(&p.q1, tau1, d1).expect("q₁'s tilt");
    let want2 = tilt_vector(&p.q2, tau2, d2).expect("q₂'s tilt");
    assert!(
        close(&after.q2, &want2, 1e-15),
        "q₂ lost its tilt: {:?}, want {want2:?}",
        after.q2
    );
    assert!(
        close(&after.q1, &want1, 1e-15),
        "q₁ is not its own tilt: {:?}, want {want1:?}",
        after.q1
    );
}

#[test]
fn tilt_pair_holds_both_angles() {
    let p = oblique_plane();
    let h = hidden(&p).expect("a plane");
    let (d1, d2) = (h[1].direction, h[4].direction);
    let c = ChartConstructor::new(p.clone());
    // τ₂ = 30° first, then τ₁ = 20° with τ₂ held.
    let first = apply(&p, c.tilt_pair(&p, (0.0, &d1), (30.0, &d2)).expect("τ₂"));
    let alone = tilted(&p, 30.0, &d2);
    assert!(close(&first.q1, &alone.q1, 1e-15) && close(&first.q2, &alone.q2, 1e-15));
    let both = apply(
        &first,
        c.tilt_pair(&first, (20.0, &d1), (30.0, &d2))
            .expect("τ₁, τ₂"),
    );
    check_both_held(&p, &both, (20.0, &d1), (30.0, &d2));
    check_planes_differ(&p, &both);
    assert_eq!(ic(&both, 0.5, 0.5), p.z0);
    assert_eq!(c.committed(), &p);
}

negative_control!(
    tilt_pair_holds_both_angles,
    "a one-vector tilt of q₁ drops q₂'s tilt",
    expected = "q₂ lost its tilt",
    {
        let p = oblique_plane();
        let h = hidden(&p).expect("a plane");
        let (d1, d2) = (h[1].direction, h[4].direction);
        let c = ChartConstructor::new(p.clone());
        let first = tilted(&p, 30.0, &d2);
        let one = apply(
            &first,
            c.tilt(&first, BasisVector::Q1, 20.0, &d1).expect("τ₁"),
        );
        check_both_held(&p, &one, (20.0, &d1), (30.0, &d2));
    }
);

/// The pair `(τ₁ = −35°, τ₂ = 30°)` of `p`, after an earlier pair `(20°, 30°)`: `got`; the pair alone `alone`.
fn retilted(p: &Plane, c: &ChartConstructor, d: [&Latent; 3]) -> (Plane, Plane, Plane) {
    let earlier = apply(
        p,
        c.tilt_pair(p, (20.0, d[0]), (30.0, d[1])).expect("earlier"),
    );
    let got = apply(
        &earlier,
        c.tilt_pair(&earlier, (-35.0, d[2]), (30.0, d[1]))
            .expect("re-tilt"),
    );
    let alone = apply(
        p,
        c.tilt_pair(p, (-35.0, d[2]), (30.0, d[1])).expect("alone"),
    );
    (earlier, got, alone)
}

#[test]
fn tilt_pair_retilting_q1_replaces_its_tilt() {
    let p = oblique_plane();
    let h = hidden(&p).expect("a plane");
    let c = ChartConstructor::new(p.clone());
    let (earlier, got, alone) =
        retilted(&p, &c, [&h[1].direction, &h[4].direction, &h[2].direction]);
    // The composition: −35° of the already tilted q₁, q₂ unchanged.
    let composed = apply(
        &earlier,
        ChartConstructor::new(earlier.clone())
            .tilt_pair(&earlier, (-35.0, &h[2].direction), (0.0, &h[1].direction))
            .expect("composed"),
    );
    check_replaced(&got, &alone, &composed);
    check_both_held(&p, &got, (-35.0, &h[2].direction), (30.0, &h[4].direction));
}

negative_control!(
    tilt_pair_retilting_q1_replaces_its_tilt,
    "a composition is not the re-tilt alone",
    expected = "two raw tilts composed",
    {
        let p = oblique_plane();
        let h = hidden(&p).expect("a plane");
        let c = ChartConstructor::new(p.clone());
        let (earlier, _, alone) =
            retilted(&p, &c, [&h[1].direction, &h[4].direction, &h[2].direction]);
        let composed = apply(
            &earlier,
            ChartConstructor::new(earlier.clone())
                .tilt_pair(&earlier, (-35.0, &h[2].direction), (0.0, &h[1].direction))
                .expect("composed"),
        );
        check_replaced(&composed, &alone, &composed);
    }
);

#[test]
fn tilt_pair_keeps_each_zoom_and_refuses_what_tilt_refuses() {
    let p = oblique_plane();
    let h = hidden(&p).expect("a plane");
    let (d1, d2) = (h[0].direction, h[3].direction);
    let c = ChartConstructor::new(p.clone());
    // A current plane whose two vectors have different lengths since the commit.
    let current = Plane {
        q1: scale(&p.q1, 2.0),
        q2: scale(&p.q2, 0.5),
        ..p.clone()
    };
    let after = apply(
        &current,
        c.tilt_pair(&current, (50.0, &d1), (-40.0, &d2))
            .expect("a tilt"),
    );
    assert!(
        (norm(&after.q1) - 0.6).abs() <= 1e-15,
        "the tilt changed |q₁|: {}",
        norm(&after.q1)
    );
    assert!(
        (norm(&after.q2) - 0.15).abs() <= 1e-15,
        "the tilt changed |q₂|: {}",
        norm(&after.q2)
    );
    // Both vectors fully along one direction leave no plane, either sign.
    for (t1, t2) in [(90.0, 90.0), (-90.0, 90.0), (90.0, -90.0)] {
        assert_eq!(
            c.tilt_pair(&p, (t1, &d1), (t2, &d1)),
            Err(NavError::DegeneratePlane),
            "({t1}, {t2})"
        );
    }
    // Just inside, a plane.
    assert!(c.tilt_pair(&p, (89.0, &d1), (90.0, &d1)).is_ok());
    // Each angle's range is [−90°, +90°], on either vector.
    for (t1, t2) in [(90.0 + 1e-9, 0.0), (0.0, -90.0 - 1e-9), (f64::NAN, 0.0)] {
        assert!(
            c.tilt_pair(&p, (t1, &d1), (t2, &d2)).is_err(),
            "({t1}, {t2}) accepted"
        );
    }
    assert!(c.tilt_pair(&p, (-90.0, &d1), (90.0, &d2)).is_ok());
}

negative_control!(
    tilt_pair_keeps_each_zoom_and_refuses_what_tilt_refuses,
    "a tilt at the committed lengths loses the zoom since the commit",
    expected = "the tilt changed |q₁|",
    {
        let p = oblique_plane();
        let d = hidden(&p).expect("a plane")[0].direction;
        let after = tilt_vector(&p.q1, 50.0, &d).expect("a tilt");
        assert!(
            (norm(&after) - 0.6).abs() <= 1e-15,
            "the tilt changed |q₁|: {}",
            norm(&after)
        );
    }
);

// ---- named_directions (REQ-CHART-009, REQ-CHART-051) ----

/// The Burrau-centred plane: a configuration axis and a momentum axis, zoomed.
fn burrau_plane() -> Plane {
    Plane {
        z0: burrau_centre(),
        q1: scale(&e(0), 0.5),
        q2: scale(&e(4), 0.5),
    }
}

/// `v` is a unit vector zero outside `block`.
fn check_unit_in(v: &Latent, block: std::ops::Range<usize>, name: &str) {
    assert!(
        (norm(v) - 1.0).abs() <= 1e-12,
        "{name} is not a unit vector"
    );
    for (i, x) in v.iter().enumerate() {
        assert!(
            block.contains(&i) || *x == 0.0,
            "{name} has z[{i}] = {x}, outside its block"
        );
    }
}

#[test]
fn named_directions_library_returns_the_three_vectors() {
    let p = burrau_plane();
    let lib = library::<Defaults>(&p);
    check_unit_in(&lib.mass.expect("the mass direction"), 6..8, "d_m");
    check_unit_in(&lib.energy.expect("the energy direction"), 2..6, "d_E");
    for m in [lib.morph_mass, lib.morph_momentum] {
        let m = m.expect("the morph");
        assert_eq!(m.source, BasisVector::Q1);
        assert!((norm(&m.target) - 1.0).abs() <= 1e-12);
        assert!(norm(&off_plane(&p, &m.target).expect("a plane")) > 1.0 - 1e-12);
    }
    assert_eq!(lib.hidden, hidden(&p));
}

negative_control!(
    named_directions_library_returns_the_three_vectors,
    "the energy direction is not in the mass block",
    expected = "outside its block",
    check_unit_in(
        &energy_at_fixed_lz::<Defaults>(&burrau_centre()).expect("d_E"),
        6..8,
        "d_E"
    )
);

/// `a` and `b` differ: the vector moved with the centre.
fn check_moved(a: &Latent, b: &Latent) {
    assert!(
        norm(&sub(a, b)) > 1e-3,
        "the energy vector did not move: {a:?}"
    );
}

#[test]
fn named_directions_energy_vector_moves_with_the_centre() {
    let z = burrau_centre();
    let mut moved = z;
    moved[0] += 0.8;
    moved[1] -= 0.5;
    let at = |z: &Latent| energy_at_fixed_lz::<Defaults>(z).expect("d_E");
    check_moved(&at(&z), &at(&moved));
}

negative_control!(
    named_directions_energy_vector_moves_with_the_centre,
    "the same centre gives the same vector",
    expected = "did not move",
    {
        let at = |z: &Latent| energy_at_fixed_lz::<Defaults>(z).expect("d_E");
        check_moved(&at(&burrau_centre()), &at(&burrau_centre()));
    }
);

/// `(E, L_z)` of the IC `z` decodes to.
fn energy_and_lz(z: &Latent) -> (f64, f64) {
    let d = decode::<Defaults, f64>(&Controls::new(*z));
    let lz: f64 = (0..3)
        .map(|i| d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0])
        .sum();
    (kinetic(&d.ic) + potential(&d.ic), lz)
}

/// Along `d` from `z`, `E` rises at first order and `L_z` holds.
fn check_energy_direction(z: &Latent, d: &Latent) {
    let h = 1e-5;
    let (e_plus, l_plus) = energy_and_lz(&std::array::from_fn(|i| z[i] + h * d[i]));
    let (e_minus, l_minus) = energy_and_lz(&std::array::from_fn(|i| z[i] - h * d[i]));
    let de = (e_plus - e_minus) / (2.0 * h);
    let dl = (l_plus - l_minus) / (2.0 * h);
    assert!(
        dl.abs() <= 1e-7,
        "L_z changed along the direction: dL_z = {dl}"
    );
    assert!(de > 1e-3, "E did not rise along the direction: dE = {de}");
}

#[test]
fn named_directions_energy_rises_at_fixed_lz() {
    for z in [burrau_centre(), oblique_plane().z0] {
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E");
        check_energy_direction(&z, &d);
    }
}

negative_control!(
    named_directions_energy_rises_at_fixed_lz,
    "a raw momentum direction changes L_z",
    expected = "L_z changed",
    check_energy_direction(&burrau_centre(), &e(3))
);

/// The pull-back, through the default momentum links, of the `(L_z, K)` construction's momenta at `(lz, k)`, minus
/// `z0`'s momentum controls, normalised: the construction's latent chord from `z0`.
fn construction_chord(z0: &Latent, lz: f64, k: f64) -> Latent {
    let d = decode::<Defaults, f64>(&Controls::new(*z0));
    let c = invariant::construct(&d.masses, &d.config, z0[0], lz, k);
    let (m, p) = (d.masses.m, c.p);
    let q = [
        (m[0] * p[1][0] - m[1] * p[0][0]) / d.masses.m01,
        (m[0] * p[1][1] - m[1] * p[0][1]) / d.masses.m01,
        p[2][0],
        p[2][1],
    ];
    let mut v = [0.0; 8];
    for (k, qk) in q.iter().enumerate() {
        v[2 + k] = <SigmoidQ as Link<1, 1>>::inverse([*qk])[0] - z0[2 + k];
    }
    scale(&v, 1.0 / norm(&v))
}

/// `d` is the construction's direction at `z0`.
fn check_seed_direction(d: &Latent, chord: &Latent) {
    assert!(
        close(d, chord, 1e-5),
        "d_E {d:?} is not the construction's seed direction {chord:?}"
    );
}

#[test]
fn named_directions_energy_at_rest_is_the_construction_seed() {
    for z_alpha in [-0.4, 0.0, 0.6] {
        let mut z = burrau_centre();
        z[0] = z_alpha;
        z[2..6].fill(0.0);
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E at rest");
        check_seed_direction(&d, &construction_chord(&z, 0.0, 1e-12));
    }
}

negative_control!(
    named_directions_energy_at_rest_is_the_construction_seed,
    "the other seed's direction is not the one taken",
    expected = "is not the construction's seed direction",
    {
        let mut z = burrau_centre();
        z[0] = -0.4;
        z[2..6].fill(0.0);
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E at rest");
        // The seed is decided on z_α: at z_α = 0.6 the construction takes (0, λ), not (ρ, 0).
        let mut other = z;
        other[0] = 0.6;
        check_seed_direction(&d, &construction_chord(&other, 0.0, 1e-12));
    }
);

/// The centre whose momenta are the rigid rotation at `L_z = lz`, `K = K_min = L_z²/2`, at the configuration control
/// `z_alpha`.
fn rigid_rotation(z_alpha: f64, lz: f64) -> Latent {
    let mut z = burrau_centre();
    z[0] = z_alpha;
    let d = decode::<Defaults, f64>(&Controls::new(z));
    let c = invariant::construct(&d.masses, &d.config, z[0], lz, lz * lz / 2.0);
    let m = d.masses.m;
    let q = [
        (m[0] * c.p[1][0] - m[1] * c.p[0][0]) / d.masses.m01,
        (m[0] * c.p[1][1] - m[1] * c.p[0][1]) / d.masses.m01,
        c.p[2][0],
        c.p[2][1],
    ];
    for (k, qk) in q.iter().enumerate() {
        z[2 + k] = <SigmoidQ as Link<1, 1>>::inverse([*qk])[0];
    }
    z
}

#[test]
fn named_directions_energy_on_the_rigid_rotation_is_the_construction_seed() {
    // Its K − K_min is rounding, so the field is the seed's, the construction's K tangent there.
    for z_alpha in [-0.3, 0.3] {
        let z = rigid_rotation(z_alpha, 0.2);
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E on the rotation");
        check_seed_direction(&d, &construction_chord(&z, 0.2, 0.02 + 1e-12));
    }
}

negative_control!(
    named_directions_energy_on_the_rigid_rotation_is_the_construction_seed,
    "the other seed's direction is not the one taken on the rotation",
    expected = "is not the construction's seed direction",
    {
        let z = rigid_rotation(-0.3, 0.2);
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E on the rotation");
        let mut other = z;
        other[0] = 0.3;
        check_seed_direction(&d, &construction_chord(&other, 0.2, 0.02 + 1e-12));
    }
);

#[test]
fn named_directions_mass_line_reaches_equal_masses() {
    let z = burrau_centre();
    let d = mass_away_from_burrau::<Defaults>(&z).expect("d_m");
    check_unit_in(&d, 6..8, "d_m");
    let length = (z[6] * z[6] + z[7] * z[7]).sqrt();
    let end: Latent = std::array::from_fn(|i| z[i] + length * d[i]);
    let m = decode::<Defaults, f64>(&Controls::new(end)).masses.m;
    for mi in m {
        assert!(
            (mi - 1.0 / 3.0).abs() <= 1e-12,
            "the line ends at the masses {m:?}"
        );
    }
    let mut equal = z;
    equal[6..8].fill(0.0);
    assert_eq!(
        mass_away_from_burrau::<Defaults>(&equal),
        Err(Unavailable::EqualMasses)
    );
}

negative_control!(
    named_directions_mass_line_reaches_equal_masses,
    "the energy direction does not lead to equal masses",
    expected = "outside its block",
    check_unit_in(
        &energy_at_fixed_lz::<Defaults>(&burrau_centre()).expect("d_E"),
        6..8,
        "d_m"
    )
);

/// The mass selection a stick-breaking link makes, the rest the defaults.
type StickSelection =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// Under `S`, the mass line from `z` ends at equal masses.
fn check_mass_line<S: kernel::generated::links::Selection>(z: &Latent) {
    let d = mass_away_from_burrau::<S>(z).expect("d_m");
    let start = <S::MassLink as Link<2, 3>>::forward([z[6], z[7]]);
    let length = {
        let eq = <S::MassLink as Link<2, 3>>::inverse([1.0 / 3.0; 3]);
        ((eq[0] - z[6]).powi(2) + (eq[1] - z[7]).powi(2)).sqrt()
    };
    let end = <S::MassLink as Link<2, 3>>::forward([z[6] + length * d[6], z[7] + length * d[7]]);
    for mi in end {
        assert!(
            (mi - 1.0 / 3.0).abs() <= 1e-12,
            "the line ends at the masses {end:?}, from {start:?}"
        );
    }
}

#[test]
fn named_directions_mass_line_reaches_equal_masses_under_any_mass_link() {
    check_mass_line::<Defaults>(&burrau_centre());
    check_mass_line::<StickSelection>(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.7, -0.4]);
}

negative_control!(
    named_directions_mass_line_reaches_equal_masses_under_any_mass_link,
    "the line along the energy direction does not reach equal masses",
    expected = "the line ends at the masses",
    {
        let z = burrau_centre();
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E");
        let end = <SoftmaxTanh as Link<2, 3>>::forward([z[6] + d[6], z[7] + d[7]]);
        for mi in end {
            assert!(
                (mi - 1.0 / 3.0).abs() <= 1e-12,
                "the line ends at the masses {end:?}"
            );
        }
    }
);

/// The centre at the configuration control `z_alpha` whose momenta are the `(L_z, K)` construction's with `seed`, and
/// the latent chord from the same construction at `K_min` to it, normalised: its momentum direction off the rotation.
fn seeded_centre(z_alpha: f64, seed: invariant::Seed, lz: f64, k: f64) -> (Latent, Latent) {
    let mut z = burrau_centre();
    z[0] = z_alpha;
    let d = decode::<Defaults, f64>(&Controls::new(z));
    let controls = |kk: f64| {
        let c = invariant::construct_seeded(&d.masses, &d.config, seed, lz, kk);
        let m = d.masses.m;
        let q = [
            (m[0] * c.p[1][0] - m[1] * c.p[0][0]) / d.masses.m01,
            (m[0] * c.p[1][1] - m[1] * c.p[0][1]) / d.masses.m01,
            c.p[2][0],
            c.p[2][1],
        ];
        q.map(|qk| <SigmoidQ as Link<1, 1>>::inverse([qk])[0])
    };
    let (at, rotation) = (controls(k), controls(lz * lz / 2.0));
    z[2..6].copy_from_slice(&at);
    let mut chord = [0.0; 8];
    for i in 0..4 {
        chord[2 + i] = at[i] - rotation[i];
    }
    (z, scale(&chord, 1.0 / norm(&chord)))
}

#[test]
fn named_directions_energy_near_the_rotation_is_the_velocity_off_it() {
    // (K − K_min)/K = 10⁻⁹ > ε_w, so the field is δv, here along the seed (0, λ), not the decoded seed (ρ, 0).
    let (lz, k_min) = (0.05, 0.05 * 0.05 / 2.0);
    let (z, off) = seeded_centre(-0.4, invariant::Seed::Lambda, lz, k_min * (1.0 + 1e-9));
    let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E");
    check_seed_direction(&d, &off);
}

negative_control!(
    named_directions_energy_near_the_rotation_is_the_velocity_off_it,
    "the decoded seed's direction is not δv there",
    expected = "is not the construction's seed direction",
    {
        let (lz, k_min) = (0.05, 0.05 * 0.05 / 2.0);
        let (z, _) = seeded_centre(-0.4, invariant::Seed::Lambda, lz, k_min * (1.0 + 1e-9));
        let (_, rho) = seeded_centre(-0.4, invariant::Seed::Rho, lz, k_min * (1.0 + 1e-9));
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("d_E");
        check_seed_direction(&d, &rho);
    }
);

/// One step of the persistence sweep: `after` keeps `before`'s anchor `z(½,½)` exactly and its non-source vector.
fn check_anchor_kept(before: &Plane, after: &Plane) {
    assert_eq!(ic(after, 0.5, 0.5), before.z0, "the anchor moved");
    assert_eq!(
        after.q2, before.q2,
        "the morph moved the other basis vector"
    );
}

#[test]
fn named_directions_morph_supports_the_persistence_test() {
    let p = burrau_plane();
    let c = ChartConstructor::new(p.clone());
    for into in [MorphInto::Mass, MorphInto::Momentum] {
        let m = morph::<Defaults>(&p, into).expect("the morph");
        assert_eq!(m.source, BasisVector::Q1);
        let mut last = p.clone();
        for step in 0..=9 {
            let tau = 10.0 * f64::from(step);
            let after = apply(
                &p,
                c.tilt(&p, m.source, tau, &m.target).expect("a morph step"),
            );
            check_anchor_kept(&p, &after);
            // Continuous: a 10° step turns q₁ by 10° at its length 0.5.
            let turn = norm(&sub(&after.q1, &last.q1));
            assert!(
                turn <= 0.5 * 10f64.to_radians() + 1e-12,
                "the morph jumped {turn} at {tau}°"
            );
            last = after;
        }
        assert!(
            close(&last.q1, &scale(&m.target, 0.5), 1e-15),
            "τ = 90° is not the target"
        );
    }
}

negative_control!(
    named_directions_morph_supports_the_persistence_test,
    "a slice in place of the morph moves the anchor",
    expected = "the anchor moved",
    {
        let p = burrau_plane();
        let m = morph::<Defaults>(&p, MorphInto::Mass).expect("the morph");
        let after = apply(&p, slice(&p, &m.target, 0.1).expect("a slice"));
        check_anchor_kept(&p, &after);
    }
);

#[test]
fn named_directions_morph_source_is_the_configuration_direction() {
    let plane = |q1: Latent, q2: Latent| Plane {
        z0: burrau_centre(),
        q1,
        q2,
    };
    assert_eq!(morph_source(&plane(e(2), e(1))), Ok(BasisVector::Q2));
    assert_eq!(morph_source(&plane(e(0), e(1))), Ok(BasisVector::Q1));
    let mixed: Latent = [0.6, 0.0, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(morph_source(&plane(mixed, e(1))), Ok(BasisVector::Q2));
    assert_eq!(morph_source(&plane(e(1), mixed)), Ok(BasisVector::Q1));
    // The share is of each vector's own length: a long, mostly momentum q₁ against a short configuration q₂.
    assert_eq!(
        morph_source(&plane(scale(&mixed, 2.0), scale(&e(1), 0.5))),
        Ok(BasisVector::Q2)
    );
    assert_eq!(
        morph_source(&plane(e(2), e(6))),
        Err(NavError::NoConfiguration)
    );
    assert_eq!(morph_source(&plane([0.0; 8], e(1))), Ok(BasisVector::Q2));
    assert_eq!(
        morph::<Defaults>(&plane(e(2), e(6)), MorphInto::Mass),
        Err(NavError::NoConfiguration)
    );
}

negative_control!(
    named_directions_morph_source_is_the_configuration_direction,
    "a momentum axis is not the configuration direction",
    expected = "assertion",
    assert_eq!(
        morph_source(&Plane {
            z0: [0.0; 8],
            q1: e(2),
            q2: e(1)
        }),
        Ok(BasisVector::Q1)
    )
);

#[test]
fn named_directions_unavailable_where_they_give_no_direction() {
    let z = burrau_centre();
    let d_m = mass_away_from_burrau::<Defaults>(&z).expect("d_m");
    let holds_mass = Plane {
        z0: z,
        q1: e(0),
        q2: d_m,
    };
    assert_eq!(
        morph::<Defaults>(&holds_mass, MorphInto::Mass),
        Err(NavError::Unavailable(Unavailable::InPlane))
    );
    let mut saturated = z;
    saturated[2] = 1e3;
    assert_eq!(
        energy_at_fixed_lz::<Defaults>(&saturated),
        Err(Unavailable::SaturatedMomentum)
    );
    let mut equal = burrau_plane();
    equal.z0[6..8].fill(0.0);
    assert_eq!(
        morph::<Defaults>(&equal, MorphInto::Mass),
        Err(NavError::Unavailable(Unavailable::EqualMasses))
    );
    let mut far = z;
    far[6] = f64::NAN;
    assert_eq!(
        mass_away_from_burrau::<Defaults>(&far),
        Err(Unavailable::NonFinite)
    );
}

negative_control!(
    named_directions_unavailable_where_they_give_no_direction,
    "a Burrau centre has a mass direction",
    expected = "EqualMasses",
    assert_eq!(
        mass_away_from_burrau::<Defaults>(&burrau_centre()).map(|_| ()),
        Err(Unavailable::EqualMasses)
    )
);

// ---- oblique_slice_record (REQ-CHART-016) ----

/// The export of `sim` records `plane`'s `q₁` and `q₂` exactly.
fn check_recorded(sim: &SimConfig, plane: &Plane) {
    let text = canonical::to_string(sim).expect("SimConfig serialises");
    let json: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    for (name, q) in [("q1", &plane.q1), ("q2", &plane.q2)] {
        let got: Latent =
            serde_json::from_value(json["plane"][name].clone()).expect("eight numbers");
        assert_eq!(&got, q, "the export does not record {name}");
    }
}

#[test]
fn oblique_slice_record_exports_q1_and_q2() {
    let a = [0.3, -1.1, 0.4, 0.2, -0.7, 0.9, 0.05, -0.3];
    let b = [-0.6, 0.2, 1.3, -0.4, 0.1, 0.5, -0.8, 0.25];
    let p = oblique([0.0; 8], &a, &b).expect("a plane");
    assert!((norm(&p.q1) - 1.0).abs() <= 1e-15 && (norm(&p.q2) - 1.0).abs() <= 1e-15);
    assert!(dot(&p.q1, &p.q2).abs() <= 1e-15);
    assert!(close(&p.q1, &scale(&a, 1.0 / norm(&a)), 1e-15));
    check_recorded(&sim(p.clone()), &p);
    assert_eq!(
        oblique([0.0; 8], &a, &scale(&a, -2.0)),
        Err(NavError::DegeneratePlane)
    );
    assert_eq!(
        oblique([0.0; 8], &[0.0; 8], &b),
        Err(NavError::DegeneratePlane)
    );
    assert_eq!(oblique([f64::NAN; 8], &a, &b), Err(NavError::NonFinite));
}

negative_control!(
    oblique_slice_record_exports_q1_and_q2,
    "an export of another plane does not record the oblique pair",
    expected = "the export does not record",
    {
        let a = [0.3, -1.1, 0.4, 0.2, -0.7, 0.9, 0.05, -0.3];
        let b = [-0.6, 0.2, 1.3, -0.4, 0.1, 0.5, -0.8, 0.25];
        let p = oblique([0.0; 8], &a, &b).expect("a plane");
        check_recorded(&sim(burrau_plane()), &p);
    }
);

/// Two raw tilts gave `got`: the second alone, `second`, not the composition `composed`.
fn check_replaced(got: &Plane, second: &Plane, composed: &Plane) {
    assert!(
        close(&got.q1, &second.q1, 1e-15) && close(&got.q2, &second.q2, 1e-15),
        "two raw tilts composed: {got:?} is not the second tilt alone"
    );
    check_planes_differ(second, composed);
}

#[test]
fn oblique_slice_record_raw_tilts_replace() {
    let p = oblique_plane();
    let h = hidden(&p).expect("a plane");
    let c = ChartConstructor::new(p.clone());
    let first = apply(
        &p,
        c.tilt(&p, BasisVector::Q2, 30.0, &h[0].direction)
            .expect("tilt 1"),
    );
    let both = apply(
        &first,
        c.tilt(&first, BasisVector::Q1, 40.0, &h[1].direction)
            .expect("tilt 2"),
    );
    let second = apply(
        &p,
        c.tilt(&p, BasisVector::Q1, 40.0, &h[1].direction)
            .expect("alone"),
    );
    let composed = apply(
        &first,
        ChartConstructor::new(first.clone())
            .tilt(&first, BasisVector::Q1, 40.0, &h[1].direction)
            .expect("composed"),
    );
    check_replaced(&both, &second, &composed);
    assert_eq!(c.committed(), &p);
}

negative_control!(
    oblique_slice_record_raw_tilts_replace,
    "a composition is not the second tilt alone",
    expected = "two raw tilts composed",
    {
        let p = oblique_plane();
        let h = hidden(&p).expect("a plane");
        let c = ChartConstructor::new(p.clone());
        let first = apply(
            &p,
            c.tilt(&p, BasisVector::Q2, 30.0, &h[0].direction)
                .expect("tilt 1"),
        );
        let second = apply(
            &p,
            c.tilt(&p, BasisVector::Q1, 40.0, &h[1].direction)
                .expect("alone"),
        );
        let composed = apply(
            &first,
            ChartConstructor::new(first.clone())
                .tilt(&first, BasisVector::Q1, 40.0, &h[1].direction)
                .expect("composed"),
        );
        check_replaced(&composed, &second, &composed);
    }
);

#[test]
fn oblique_slice_record_commit_starts_the_next_tilt() {
    let p = oblique_plane();
    let h = hidden(&p).expect("a plane");
    let mut c = ChartConstructor::new(p.clone());
    let first = apply(
        &p,
        c.tilt(&p, BasisVector::Q2, 30.0, &h[0].direction)
            .expect("tilt 1"),
    );
    c.commit(first.clone());
    assert_eq!(c.committed(), &first);
    let again = apply(
        &first,
        c.tilt(&first, BasisVector::Q2, 0.0, &h[1].direction)
            .expect("τ = 0"),
    );
    assert!(
        close(&again.q2, &first.q2, 1e-15),
        "the commit did not start the next tilt"
    );
}

negative_control!(
    oblique_slice_record_commit_starts_the_next_tilt,
    "without the commit, τ = 0 returns to the first plane",
    expected = "the commit did not start the next tilt",
    {
        let p = oblique_plane();
        let h = hidden(&p).expect("a plane");
        let c = ChartConstructor::new(p.clone());
        let first = apply(
            &p,
            c.tilt(&p, BasisVector::Q2, 30.0, &h[0].direction)
                .expect("tilt 1"),
        );
        let again = apply(
            &first,
            c.tilt(&first, BasisVector::Q2, 0.0, &h[1].direction)
                .expect("τ = 0"),
        );
        assert!(
            close(&again.q2, &first.q2, 1e-15),
            "the commit did not start the next tilt"
        );
    }
);
