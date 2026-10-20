//! QA's tests for TASK-M2-22, written from the requirements it closes and their sources, not from the implementation:
//!
//! - REQ-GUI-002 (chart_decoder_contract Part 4, "The operation table", "Slice"): a slice moves every pixel's IC by the
//!   identical step, orthogonal to the plane; a pan's `Δz₀` lies in `span(q₁, q₂)` and keeps the plane; a zoom keeps
//!   `z₀` and scales `q₁, q₂` by one common, log-stepped factor.
//! - REQ-GUI-003 (Part 4, "Tilt", "Directions are axis kinds"): `q′(τ) = normalise(cos τ·q + sin τ·d)`; `τ = 0` leaves
//!   `q`; `τ = ±90°` gives `±d`; `±30°` differ; `τ` outside `[−90°, 90°]` is refused; the six directions orthogonal to
//!   the plane complete it to ℝ⁸; a curve axis tilts its tangent at the centre.
//! - REQ-CHART-009 / REQ-CHART-051 (Part 4, "Their latent vectors"): the three named directions as unit latent
//!   vectors in their blocks, evaluated at and recomputed with the centre; `d_E` is tangent to the `L_z` level set and
//!   raises `E`, its physical momentum change parallel to `mᵢ(v − (L_z/I)·J r)ᵢ`; `d_m`'s step of `‖z_eq − z₀[6:8]‖`
//!   reaches equal masses; the morph's target leaves the plane and its tilt keeps `z(½,½) = z₀`.
//! - REQ-CHART-016 (chart_reference §1.1): an oblique slice records its orthonormal pair in the export; raw tilts
//!   replace rather than compose.
//!
//! Every IC is read through the kernel's latent chart `Φ`, every edit applied through the real state store.

use engine::contract::canonical;
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice,
};
use engine::contract::store::StateStore;
use engine::nav::directions::{
    energy_at_fixed_lz, hidden, library, mass_away_from_burrau, morph, MorphInto, Unavailable,
};
use engine::nav::{
    oblique, pan, slice, tilt_tangent, tilt_vector, zoom, BasisVector, ChartConstructor, NavError,
};
use kernel::chart::{Chart as _, Latent as LatentChart};
use kernel::decode::decode;
use kernel::decode::descriptor::{kinetic, potential};
use kernel::decode::latent::Latent as Controls;
use kernel::generated::links::{Defaults, Link, SoftmaxTanh};
use validation::negative_control;

// ---- helpers ----

fn dot(a: &Latent, b: &Latent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &Latent) -> f64 {
    dot(a, a).sqrt()
}

fn sub(a: &Latent, b: &Latent) -> Latent {
    std::array::from_fn(|i| a[i] - b[i])
}

fn add_scaled(a: &Latent, k: f64, b: &Latent) -> Latent {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn scale(a: &Latent, k: f64) -> Latent {
    a.map(|x| k * x)
}

fn normalise(a: &Latent) -> Latent {
    scale(a, 1.0 / norm(a))
}

fn e(k: usize) -> Latent {
    let mut v = [0.0; 8];
    v[k] = 1.0;
    v
}

/// A deterministic pseudo-random vector in `[−1, 1]⁸` (a 64-bit LCG), so the sweeps cover generic planes.
fn draw(seed: &mut u64) -> Latent {
    std::array::from_fn(|_| {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    })
}

/// `v` minus its orthogonal projection onto `span(a, b)`, by the 2×2 normal equations (not Gram–Schmidt).
fn residual_off(a: &Latent, b: &Latent, v: &Latent) -> Latent {
    let (g11, g12, g22) = (dot(a, a), dot(a, b), dot(b, b));
    let (r1, r2) = (dot(a, v), dot(b, v));
    let det = g11 * g22 - g12 * g12;
    let x = (r1 * g22 - r2 * g12) / det;
    let y = (r2 * g11 - r1 * g12) / det;
    std::array::from_fn(|i| v[i] - x * a[i] - y * b[i])
}

/// The distance of `v`'s direction from `span(a, b)`: `‖v − proj v‖ / ‖v‖`.
fn off_span(a: &Latent, b: &Latent, v: &Latent) -> f64 {
    norm(&residual_off(a, b, v)) / norm(v)
}

/// Two planes' spans are the same subspace.
fn same_span(p: &Plane, q: &Plane, tol: f64) -> bool {
    off_span(&p.q1, &p.q2, &q.q1) <= tol && off_span(&p.q1, &p.q2, &q.q2) <= tol
}

fn sim(plane: Plane) -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane,
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
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
        },
    )
}

/// The plane after `edit` goes through the state store.
fn apply(plane: &Plane, edit: SetField) -> Plane {
    let mut s = store(plane.clone());
    s.set_field(edit);
    s.snapshot().sim.plane
}

/// Pixel `(s, t)`'s IC, through the kernel's `Φ`.
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

const PIXELS: [f64; 5] = [0.0, 0.1, 0.5, 0.9, 1.0];

/// Generic planes: random centre, random non-orthonormal, non-unit basis.
fn planes() -> Vec<Plane> {
    let mut seed = 0x5eed_2222_u64;
    (0..16)
        .map(|_| {
            let z0 = draw(&mut seed);
            let q1 = scale(&draw(&mut seed), 0.7);
            let q2 = scale(&draw(&mut seed), 0.4);
            Plane { z0, q1, q2 }
        })
        .collect()
}

// ---- REQ-GUI-002: slice, pan, zoom ----

/// Every pixel moved by one common step, orthogonal to `before`'s plane, and the basis is untouched.
fn check_slice(before: &Plane, after: &Plane) {
    assert_eq!(after.q1, before.q1, "a slice edited q₁");
    assert_eq!(after.q2, before.q2, "a slice edited q₂");
    let step = sub(&after.z0, &before.z0);
    for s in PIXELS {
        for t in PIXELS {
            let moved = sub(&ic(after, s, t), &ic(before, s, t));
            assert!(
                norm(&sub(&moved, &step)) <= 1e-12,
                "pixel ({s}, {t}) moved by {moved:?}, not the common step {step:?}"
            );
        }
    }
    let along = [dot(&step, &before.q1), dot(&step, &before.q2)];
    assert!(
        along.iter().all(|x| x.abs() <= 1e-12 * (1.0 + norm(&step))),
        "the slice step is not orthogonal to the plane: {along:?}"
    );
}

#[test]
fn qa_nav_ops_slice_is_one_hidden_step_for_every_pixel() {
    let mut seed = 7_u64;
    for p in planes() {
        let d = draw(&mut seed);
        for t in [-2.0, -0.3, 0.25, 1.0] {
            let after = apply(&p, slice(&p, &d, t).expect("a slice"));
            check_slice(&p, &after);
            // z₀ ← z₀ + t·d with d's in-plane part removed: the step is t times d's off-plane residual.
            let want = scale(&residual_off(&p.q1, &p.q2, &d), t);
            assert!(
                norm(&sub(&sub(&after.z0, &p.z0), &want)) <= 1e-12,
                "the slice step is not t·d⊥"
            );
        }
    }
}

/// An exactly hidden direction of `p`, built independently: a random vector's residual off the plane.
fn hidden_of(p: &Plane) -> Latent {
    normalise(&residual_off(&p.q1, &p.q2, &draw(&mut 99_u64)))
}

/// `after`'s centre is `before`'s `z₀ + 0.6·d`.
fn check_hidden_slice(before: &Plane, d: &Latent, after: &Plane) {
    let want = add_scaled(&before.z0, 0.6, d);
    assert!(
        norm(&sub(&after.z0, &want)) <= 1e-12,
        "slice along a hidden d is not z₀ + t·d"
    );
}

#[test]
fn qa_nav_ops_slice_along_a_hidden_direction_is_z0_plus_t_d() {
    for p in planes() {
        let d = hidden_of(&p);
        let after = apply(&p, slice(&p, &d, 0.6).expect("a slice"));
        check_hidden_slice(&p, &d, &after);
    }
}

negative_control!(
    qa_nav_ops_slice_along_a_hidden_direction_is_z0_plus_t_d,
    "a step along q₁ instead of the hidden d is not z₀ + t·d",
    expected = "slice along a hidden d is not z₀ + t·d",
    {
        let p = planes()[0].clone();
        let d = hidden_of(&p);
        let after = Plane {
            z0: add_scaled(&p.z0, 0.6, &normalise(&p.q1)),
            ..p.clone()
        };
        check_hidden_slice(&p, &d, &after);
    }
);

negative_control!(
    qa_nav_ops_slice_is_one_hidden_step_for_every_pixel,
    "a raw z₀ step along a direction with an in-plane part is not orthogonal to the plane",
    expected = "not orthogonal to the plane",
    {
        let p = planes()[0].clone();
        let after = Plane {
            z0: add_scaled(&p.z0, 0.5, &p.q1),
            ..p.clone()
        };
        check_slice(&p, &after);
    }
);

/// `after`'s plane is `before`'s: same basis, and `Δz₀ ∈ span(q₁, q₂)`.
fn check_pan(before: &Plane, after: &Plane) {
    assert_eq!(after.q1, before.q1, "a pan edited q₁");
    assert_eq!(after.q2, before.q2, "a pan edited q₂");
    let dz = sub(&after.z0, &before.z0);
    assert!(norm(&dz) > 0.0, "the pan did not move z₀");
    assert!(
        off_span(&before.q1, &before.q2, &dz) <= 1e-12,
        "Δz₀ leaves span(q₁, q₂): the pan changed the plane"
    );
}

#[test]
fn qa_nav_ops_pan_moves_within_the_plane() {
    for p in planes() {
        for (ds, dt) in [(0.1, 0.0), (0.0, -0.4), (0.3, 0.7), (-1.5, 2.0)] {
            let after = apply(&p, pan(&p, ds, dt).expect("a pan"));
            check_pan(&p, &after);
            // Same slice, a different window: the new centre's IC is one of the old plane's ICs.
            let c = ic(&after, 0.5, 0.5);
            assert!(
                off_span(&p.q1, &p.q2, &sub(&c, &p.z0)) <= 1e-12,
                "the new centre is off the old plane"
            );
        }
    }
}

negative_control!(
    qa_nav_ops_pan_moves_within_the_plane,
    "a slice is not a pan: its Δz₀ leaves the plane",
    expected = "leaves span",
    {
        let p = planes()[1].clone();
        let d = draw(&mut 3_u64);
        check_pan(&p, &apply(&p, slice(&p, &d, 1.0).expect("a slice")));
    }
);

/// `after` is `before` zoomed by one common factor, `z₀` kept; returns the factor.
fn check_zoom(before: &Plane, after: &Plane) -> f64 {
    assert_eq!(after.z0, before.z0, "the zoom moved z₀");
    let k = norm(&after.q1) / norm(&before.q1);
    for (a, b, name) in [(&after.q1, &before.q1, "q₁"), (&after.q2, &before.q2, "q₂")] {
        assert!(
            norm(&sub(a, &scale(b, k))) <= 1e-12 * norm(a),
            "{name} is not scaled by the common factor {k}"
        );
    }
    k
}

#[test]
fn qa_nav_ops_zoom_is_a_common_log_stepped_scale_keeping_z0() {
    for p in planes() {
        let k1 = check_zoom(&p, &apply(&p, zoom(&p, 1.0).expect("zoom")));
        let k2 = check_zoom(&p, &apply(&p, zoom(&p, 2.0).expect("zoom")));
        let k0 = check_zoom(&p, &apply(&p, zoom(&p, 0.0).expect("zoom")));
        assert_eq!(k0, 1.0, "zero steps changed the scale");
        assert!(k1 != 1.0, "one step did not change the scale");
        // Log-stepped: equal steps multiply the scale by equal factors.
        assert!(
            (k2 - k1 * k1).abs() <= 1e-12,
            "two steps are not the square of one: {k2} vs {k1}²"
        );
        // A step in and a step out return the plane.
        let there = apply(&p, zoom(&p, 1.5).expect("zoom"));
        let back = apply(&there, zoom(&there, -1.5).expect("zoom"));
        assert!(norm(&sub(&back.q1, &p.q1)) <= 1e-12 && norm(&sub(&back.q2, &p.q2)) <= 1e-12);
        // The centre pixel's IC is untouched.
        assert_eq!(ic(&there, 0.5, 0.5), p.z0, "zoom moved the centre pixel");
    }
}

negative_control!(
    qa_nav_ops_zoom_is_a_common_log_stepped_scale_keeping_z0,
    "scaling q₁ alone is not a common zoom",
    expected = "is not scaled by the common factor",
    {
        let p = planes()[2].clone();
        let after = Plane {
            q1: scale(&p.q1, 2.0),
            ..p.clone()
        };
        check_zoom(&p, &after);
    }
);

// ---- REQ-GUI-003: tilt ----

/// The contract's formula, `normalise(cos τ·q + sin τ·d)`, computed here, for a unit `q`.
fn formula(q: &Latent, tau_deg: f64, d: &Latent) -> Latent {
    let t = tau_deg.to_radians();
    normalise(&add_scaled(&scale(q, t.cos()), t.sin(), d))
}

/// A unit plane direction and a unit hidden target at a generic plane.
fn unit_pair(p: &Plane, seed: &mut u64) -> (Latent, Latent) {
    let q = normalise(&p.q1);
    let d = normalise(&residual_off(&p.q1, &p.q2, &draw(seed)));
    (q, d)
}

#[test]
fn qa_tilt_follows_the_contract_formula() {
    let mut seed = 11_u64;
    for p in planes() {
        let (q, d) = unit_pair(&p, &mut seed);
        for tau in [-90.0, -60.0, -30.0, -1.0, 0.0, 1.0, 30.0, 45.0, 89.0, 90.0] {
            let got = tilt_vector(&q, tau, &d).expect("τ in range");
            let want = formula(&q, tau, &d);
            assert!(
                norm(&sub(&got, &want)) <= 1e-12,
                "τ = {tau}: {got:?} is not normalise(cos τ·q + sin τ·d) = {want:?}"
            );
        }
        assert!(norm(&sub(&tilt_vector(&q, 0.0, &d).unwrap(), &q)) <= 1e-15);
        assert!(norm(&sub(&tilt_vector(&q, 90.0, &d).unwrap(), &d)) <= 1e-12);
        assert!(norm(&sub(&tilt_vector(&q, -90.0, &d).unwrap(), &scale(&d, -1.0))) <= 1e-12);
    }
}

negative_control!(
    qa_tilt_follows_the_contract_formula,
    "a tilt that ignores the sign of τ is not the formula",
    expected = "is not normalise",
    {
        let p = planes()[0].clone();
        let (q, d) = unit_pair(&p, &mut 11_u64);
        let got = tilt_vector(&q, 30.0, &d).unwrap();
        let want = formula(&q, -30.0, &d);
        assert!(
            norm(&sub(&got, &want)) <= 1e-12,
            "τ = -30: {got:?} is not normalise(cos τ·q + sin τ·d) = {want:?}"
        );
    }
);

/// The tilted planes `+τ` and `−τ` of `p`'s `q₂` toward `d`, through the chart constructor and the store.
fn plus_minus(p: &Plane, tau: f64, d: &Latent) -> (Plane, Plane) {
    let c = ChartConstructor::new(p.clone());
    let plus = apply(p, c.tilt(p, BasisVector::Q2, tau, d).expect("+τ"));
    let minus = apply(p, c.tilt(p, BasisVector::Q2, -tau, d).expect("−τ"));
    (plus, minus)
}

fn check_differ(a: &Plane, b: &Plane) {
    assert!(
        !same_span(a, b, 1e-6),
        "+τ and −τ give the same plane: the sign was lost"
    );
}

#[test]
fn qa_tilt_sign_matters() {
    let mut seed = 5_u64;
    for p in planes() {
        let (_, d) = unit_pair(&p, &mut seed);
        let (plus, minus) = plus_minus(&p, 30.0, &d);
        check_differ(&plus, &minus);
        // Both are anchor-preserving: the centre stays at z₀.
        assert_eq!(plus.z0, p.z0);
        assert_eq!(minus.z0, p.z0);
    }
}

negative_control!(
    qa_tilt_sign_matters,
    "the same tilt twice gives the same plane",
    expected = "the sign was lost",
    {
        let p = planes()[0].clone();
        let (_, d) = unit_pair(&p, &mut 5_u64);
        let (plus, _) = plus_minus(&p, 30.0, &d);
        check_differ(&plus, &plus);
    }
);

const OUT_OF_RANGE: [f64; 6] = [
    -90.000001,
    90.000001,
    180.0,
    -135.0,
    f64::NAN,
    f64::INFINITY,
];

/// `tilt` refuses every `τ` outside `[−90°, 90°]`.
fn check_refuses(tilt: &dyn Fn(f64) -> Result<Latent, NavError>) {
    for tau in OUT_OF_RANGE {
        assert!(
            tilt(tau).is_err(),
            "τ = {tau} outside [−90°, 90°] was accepted"
        );
    }
}

#[test]
fn qa_tilt_refuses_tau_outside_plus_minus_ninety() {
    let p = planes()[0].clone();
    let (q, d) = unit_pair(&p, &mut 1_u64);
    check_refuses(&|tau| tilt_vector(&q, tau, &d));
    for tau in OUT_OF_RANGE {
        let c = ChartConstructor::new(p.clone());
        assert!(c.tilt(&p, BasisVector::Q1, tau, &d).is_err(), "τ = {tau}");
    }
    assert_eq!(tilt_vector(&q, 91.0, &d), Err(NavError::TauOutOfRange));
}

negative_control!(
    qa_tilt_refuses_tau_outside_plus_minus_ninety,
    "a tilt that clamps τ into [−90°, 90°] accepts an out-of-range τ",
    expected = "was accepted",
    {
        let p = planes()[0].clone();
        let (q, d) = unit_pair(&p, &mut 1_u64);
        check_refuses(&|tau| Ok(formula(&q, tau.clamp(-90.0, 90.0), &d)));
    }
);

/// A curve through latent space, `γ(ν)`, bending, and its tangent by central difference.
fn gamma(nu: f64) -> Latent {
    [
        nu.cos(),
        nu.sin(),
        0.3 * nu * nu,
        0.0,
        0.1 * nu,
        0.0,
        0.0,
        0.0,
    ]
}

fn tangent(nu: f64) -> Latent {
    let h = 1e-6;
    scale(&sub(&gamma(nu + h), &gamma(nu - h)), 0.5 / h)
}

/// Tilting the tangent `g` gives the contract formula applied to `γ′(ν₀)`, at every `τ`.
fn check_curve_tilt(nu0: f64, g: &Latent, d: &Latent) {
    let at = normalise(&tangent(nu0));
    for tau in [-45.0, 0.0, 20.0, 90.0] {
        let got = normalise(&tilt_tangent(g, 1.0, tau, d).expect("in range"));
        let want = formula(&at, tau, d);
        assert!(
            norm(&sub(&got, &want)) <= 1e-9,
            "ν₀ = {nu0}, τ = {tau}: not the tilt of γ′(ν₀)"
        );
    }
}

#[test]
fn qa_tilt_curve_axis_rotates_the_tangent_at_the_centre() {
    let d = e(6);
    for nu0 in [-0.8, 0.0, 0.4, 1.3] {
        let g = tangent(nu0);
        check_curve_tilt(nu0, &g, &d);
        // At τ = 0 it is the tangent at ν₀ and not at another point of the curve.
        let elsewhere = normalise(&tangent(nu0 + 0.5));
        let got = normalise(&tilt_tangent(&g, 1.0, 0.0, &d).unwrap());
        assert!(norm(&sub(&got, &elsewhere)) > 1e-3);
    }
}

negative_control!(
    qa_tilt_curve_axis_rotates_the_tangent_at_the_centre,
    "tilting the tangent at another point of the curve is not the tilt of γ′(ν₀)",
    expected = "not the tilt of γ′(ν₀)",
    check_curve_tilt(0.4, &tangent(0.9), &e(6))
);

/// The six hidden directions of `p` with `p`'s two span ℝ⁸, orthonormally and off the plane.
fn check_six(p: &Plane, six: &[Latent]) {
    assert_eq!(six.len(), 6);
    for (i, a) in six.iter().enumerate() {
        assert!((norm(a) - 1.0).abs() <= 1e-12, "direction {i} is not unit");
        assert!(
            dot(a, &p.q1).abs() <= 1e-12 * norm(&p.q1)
                && dot(a, &p.q2).abs() <= 1e-12 * norm(&p.q2),
            "direction {i} is not orthogonal to the plane"
        );
        for (j, b) in six.iter().enumerate().skip(i + 1) {
            assert!(
                dot(a, b).abs() <= 1e-12,
                "directions {i} and {j} are not orthogonal"
            );
        }
    }
    // Together with the plane they span ℝ⁸: every ê_k is in their span.
    let u1 = normalise(&p.q1);
    let u2 = normalise(&add_scaled(&p.q2, -dot(&p.q2, &u1), &u1));
    let frame = [u1, u2];
    for k in 0..8 {
        let ek = e(k);
        let captured: f64 = frame
            .iter()
            .chain(six.iter())
            .map(|b| dot(&ek, b).powi(2))
            .sum();
        assert!(
            (captured - 1.0).abs() <= 1e-10,
            "ê_{k} is not in span(plane, six): {captured}"
        );
    }
}

#[test]
fn qa_tilt_offers_six_directions_completing_the_plane() {
    for p in planes() {
        let six: Vec<Latent> = hidden(&p)
            .expect("a plane")
            .iter()
            .map(|h| h.direction)
            .collect();
        check_six(&p, &six);
    }
    // An axis-aligned plane: the six are the other raw axes, up to sign.
    let p = Plane {
        z0: [0.0; 8],
        q1: e(0),
        q2: e(3),
    };
    let six = hidden(&p).expect("a plane");
    for k in [1, 2, 4, 5, 6, 7] {
        assert!(
            six.iter()
                .any(|h| (dot(&h.direction, &e(k)).abs() - 1.0).abs() <= 1e-15),
            "ê_{k} is not offered on an axis-aligned plane"
        );
    }
}

negative_control!(
    qa_tilt_offers_six_directions_completing_the_plane,
    "six directions that include a plane direction are not orthogonal to it",
    expected = "not orthogonal to the plane",
    {
        let p = planes()[3].clone();
        let mut six: Vec<Latent> = hidden(&p)
            .expect("a plane")
            .iter()
            .map(|h| h.direction)
            .collect();
        six[5] = normalise(&p.q1);
        check_six(&p, &six);
    }
);

// ---- REQ-CHART-009 / REQ-CHART-051: the named directions ----

/// Burrau masses `(5, 4, 3)/12` (chart_reference §4.2) as default mass controls.
fn burrau_mass() -> [f64; 2] {
    <SoftmaxTanh as Link<2, 3>>::inverse([5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0])
}

/// Generic centres with Burrau masses and moving momenta.
fn centres() -> Vec<Latent> {
    let mut seed = 0xbeef_u64;
    let [z6, z7] = burrau_mass();
    (0..12)
        .map(|_| {
            let mut z = scale(&draw(&mut seed), 0.8);
            z[6] = z6;
            z[7] = z7;
            z
        })
        .collect()
}

fn decoded(z: &Latent) -> kernel::decode::Ic<f64> {
    decode::<Defaults, f64>(&Controls::new(*z)).ic
}

fn energy_lz(z: &Latent) -> (f64, f64) {
    let d = decoded(z);
    let lz: f64 = (0..3)
        .map(|i| d.r[i][0] * d.p[i][1] - d.r[i][1] * d.p[i][0])
        .sum();
    (kinetic(&d) + potential(&d), lz)
}

/// `mᵢ·(vᵢ − (L_z/I)·J rᵢ)`, the contract's physical momentum change, computed here from the decoded IC.
fn physical_dp(z: &Latent) -> [f64; 6] {
    let d = decoded(z);
    let v: [[f64; 2]; 3] = std::array::from_fn(|i| [d.p[i][0] / d.m[i], d.p[i][1] / d.m[i]]);
    let inertia: f64 = (0..3)
        .map(|i| d.m[i] * (d.r[i][0].powi(2) + d.r[i][1].powi(2)))
        .sum();
    let lz: f64 = (0..3)
        .map(|i| d.m[i] * (d.r[i][0] * v[i][1] - d.r[i][1] * v[i][0]))
        .sum();
    let w = lz / inertia;
    let mut out = [0.0; 6];
    for i in 0..3 {
        // J r = (−r_y, r_x).
        out[2 * i] = d.m[i] * (v[i][0] - w * -d.r[i][1]);
        out[2 * i + 1] = d.m[i] * (v[i][1] - w * d.r[i][0]);
    }
    out
}

/// The decoded momenta's derivative along `d` at `z`, by central difference.
fn momentum_derivative(z: &Latent, d: &Latent) -> [f64; 6] {
    let h = 1e-6;
    let plus = decoded(&add_scaled(z, h, d)).p;
    let minus = decoded(&add_scaled(z, -h, d)).p;
    std::array::from_fn(|k| (plus[k / 2][k % 2] - minus[k / 2][k % 2]) / (2.0 * h))
}

fn check_unit_in_block(v: &Latent, block: std::ops::Range<usize>, name: &str) {
    assert!(
        (norm(v) - 1.0).abs() <= 1e-12,
        "{name} is not a unit vector"
    );
    for k in (0..8).filter(|k| !block.contains(k)) {
        assert_eq!(v[k], 0.0, "{name} is non-zero outside its block at {k}");
    }
}

/// `d_E` at `z` is tangent to the `L_z` level set, raises `E`, and moves the momenta along the contract's `δp`.
fn check_energy(z: &Latent, d: &Latent) {
    let h = 1e-5;
    let (ep, lp) = energy_lz(&add_scaled(z, h, d));
    let (em, lm) = energy_lz(&add_scaled(z, -h, d));
    let (de, dl) = ((ep - em) / (2.0 * h), (lp - lm) / (2.0 * h));
    assert!(dl.abs() <= 1e-7, "L_z changes along d_E: dL_z = {dl}");
    assert!(de > 0.0, "E does not rise along d_E: dE = {de}");
    let got = momentum_derivative(z, d);
    let want = physical_dp(z);
    let cos = got.iter().zip(&want).map(|(a, b)| a * b).sum::<f64>()
        / (got.iter().map(|x| x * x).sum::<f64>().sqrt()
            * want.iter().map(|x| x * x).sum::<f64>().sqrt());
    assert!(
        cos >= 1.0 - 1e-8,
        "the momentum change along d_E is not parallel to m(v − (L_z/I)·J r): cos = {cos}"
    );
}

#[test]
fn qa_named_directions_energy_is_the_rotation_free_velocity_at_fixed_lz() {
    for z in centres() {
        let d = energy_at_fixed_lz::<Defaults>(&z).expect("available at a moving centre");
        check_unit_in_block(&d, 2..6, "d_E");
        check_energy(&z, &d);
    }
}

negative_control!(
    qa_named_directions_energy_is_the_rotation_free_velocity_at_fixed_lz,
    "a raw momentum axis is not the energy direction at fixed L_z",
    expected = "L_z changes along d_E",
    {
        let z = centres()[0];
        check_energy(&z, &e(3));
    }
);

/// The energy vectors at two different centres differ.
fn check_moved(da: &Latent, db: &Latent) {
    assert!(
        norm(&sub(da, db)) > 1e-3,
        "the energy-at-fixed-L_z vector did not move with the centre"
    );
}

#[test]
fn qa_named_directions_are_recomputed_with_the_centre() {
    let cs = centres();
    let (a, b) = (cs[0], cs[1]);
    let da = energy_at_fixed_lz::<Defaults>(&a).unwrap();
    let db = energy_at_fixed_lz::<Defaults>(&b).unwrap();
    check_moved(&da, &db);
    // The library at a plane through each centre carries that centre's vectors.
    for z in [a, b] {
        let p = Plane {
            z0: z,
            q1: e(0),
            q2: e(1),
        };
        let lib = library::<Defaults>(&p);
        assert_eq!(lib.energy, energy_at_fixed_lz::<Defaults>(&z));
        assert_eq!(lib.mass, mass_away_from_burrau::<Defaults>(&z));
        assert!(lib.morph_mass.is_ok() && lib.morph_momentum.is_ok());
        assert_eq!(lib.hidden.as_ref().map(|h| h.len()), Ok(6));
    }
}

negative_control!(
    qa_named_directions_are_recomputed_with_the_centre,
    "an energy vector frozen at the first centre does not move with the centre",
    expected = "did not move with the centre",
    {
        let da = energy_at_fixed_lz::<Defaults>(&centres()[0]).unwrap();
        check_moved(&da, &da);
    }
);

/// `d_m` at `z`: a unit mass-block vector whose step of `‖z_eq − z₀[6:8]‖` decodes to equal masses.
fn check_mass(z: &Latent, d: &Latent) {
    check_unit_in_block(d, 6..8, "d_m");
    let z_eq = <SoftmaxTanh as Link<2, 3>>::inverse([1.0 / 3.0; 3]);
    let dist = ((z_eq[0] - z[6]).powi(2) + (z_eq[1] - z[7]).powi(2)).sqrt();
    let m = decoded(&add_scaled(z, dist, d)).m;
    assert!(
        m.iter().all(|x| (x - 1.0 / 3.0).abs() <= 1e-12),
        "the mass line does not reach equal masses: {m:?}"
    );
    // Away from Burrau: the step moves the masses toward equal, the largest mass falls.
    let m0 = decoded(z).m;
    let m1 = decoded(&add_scaled(z, 0.1 * dist, d)).m;
    assert!(
        m1[0] < m0[0],
        "the mass direction does not move toward equal masses"
    );
}

#[test]
fn qa_named_directions_mass_line_from_burrau_reaches_equal_masses() {
    for z in centres() {
        let d = mass_away_from_burrau::<Defaults>(&z).expect("Burrau masses are unequal");
        check_mass(&z, &d);
    }
    let mut eq = centres()[0];
    let z_eq = <SoftmaxTanh as Link<2, 3>>::inverse([1.0 / 3.0; 3]);
    eq[6] = z_eq[0];
    eq[7] = z_eq[1];
    assert_eq!(
        mass_away_from_burrau::<Defaults>(&eq),
        Err(Unavailable::EqualMasses),
        "a direction to equal masses from equal masses was returned"
    );
}

negative_control!(
    qa_named_directions_mass_line_from_burrau_reaches_equal_masses,
    "the reversed mass direction moves away from equal masses",
    expected = "does not reach equal masses",
    {
        let z = centres()[0];
        let d = mass_away_from_burrau::<Defaults>(&z).unwrap();
        check_mass(&z, &scale(&d, -1.0));
    }
);

/// A Burrau-like slice: `q₁` in the configuration block, `q₂` in the momentum block.
fn burrau_plane(z0: Latent) -> Plane {
    Plane {
        z0,
        q1: scale(&normalise(&[0.6, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]), 0.5),
        q2: scale(&normalise(&[0.0, 0.1, 0.0, 1.0, 0.2, 0.0, 0.0, 0.0]), 0.5),
    }
}

fn check_morph_sweep(p: &Plane, source: BasisVector, target: &Latent) {
    let c = ChartConstructor::new(p.clone());
    for tau in [-90.0, -45.0, 0.0, 10.0, 60.0, 90.0] {
        let after = apply(p, c.tilt(p, source, tau, target).expect("tilt"));
        assert_eq!(ic(&after, 0.5, 0.5), p.z0, "τ = {tau}: the anchor moved");
        let q = match source {
            BasisVector::Q1 => after.q1,
            BasisVector::Q2 => after.q2,
        };
        if tau == 90.0 || tau == -90.0 {
            let want = scale(target, tau.signum());
            assert!(
                norm(&sub(&normalise(&q), &want)) <= 1e-12,
                "τ = {tau}: the source is not committed to ±target"
            );
        }
        if tau == 0.0 {
            assert!(
                same_span(&after, p, 1e-12),
                "τ = 0 is not the starting slice"
            );
        }
    }
}

#[test]
fn qa_named_directions_morph_leaves_the_plane_and_keeps_the_anchor() {
    for z in centres() {
        let p = burrau_plane(z);
        for into in [MorphInto::Mass, MorphInto::Momentum] {
            let m = morph::<Defaults>(&p, into).expect("a morph");
            assert_eq!(
                m.source,
                BasisVector::Q1,
                "the source is not the configuration direction"
            );
            assert!(
                (norm(&m.target) - 1.0).abs() <= 1e-12,
                "the target is not unit"
            );
            assert!(
                dot(&m.target, &p.q1).abs() <= 1e-12 && dot(&m.target, &p.q2).abs() <= 1e-12,
                "the morph target lies partly in the plane"
            );
            // The target is the named direction's off-plane part.
            let named = match into {
                MorphInto::Mass => mass_away_from_burrau::<Defaults>(&z).unwrap(),
                MorphInto::Momentum => energy_at_fixed_lz::<Defaults>(&z).unwrap(),
            };
            let want = normalise(&residual_off(&p.q1, &p.q2, &named));
            assert!(
                norm(&sub(&m.target, &want)) <= 1e-10,
                "the target is not the named direction's off-plane part"
            );
            check_morph_sweep(&p, m.source, &m.target);
        }
        // The configuration direction is q₂ when q₂ carries it.
        let swapped = Plane {
            z0: p.z0,
            q1: p.q2,
            q2: p.q1,
        };
        assert_eq!(
            morph::<Defaults>(&swapped, MorphInto::Mass).unwrap().source,
            BasisVector::Q2
        );
    }
    // Refused where neither basis vector has a configuration component.
    let none = Plane {
        z0: centres()[0],
        q1: e(2),
        q2: e(4),
    };
    assert_eq!(
        morph::<Defaults>(&none, MorphInto::Mass).map(|m| m.source),
        Err(NavError::NoConfiguration)
    );
}

negative_control!(
    qa_named_directions_morph_leaves_the_plane_and_keeps_the_anchor,
    "a slice along the morph target moves the anchor",
    expected = "the anchor moved",
    {
        let p = burrau_plane(centres()[0]);
        let m = morph::<Defaults>(&p, MorphInto::Mass).unwrap();
        let after = apply(&p, slice(&p, &m.target, 0.2).unwrap());
        assert_eq!(ic(&after, 0.5, 0.5), p.z0, "τ = 0: the anchor moved");
    }
);

// ---- REQ-CHART-016: the oblique record and raw-tilt replacement ----

fn check_export(sim: &SimConfig, q1: &Latent, q2: &Latent) {
    let text = canonical::to_string(sim).expect("serialises");
    let json: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let got1: Latent = serde_json::from_value(json["plane"]["q1"].clone()).expect("q1");
    let got2: Latent = serde_json::from_value(json["plane"]["q2"].clone()).expect("q2");
    assert!(
        got1 == *q1 && got2 == *q2,
        "the exported state does not carry the oblique pair"
    );
}

#[test]
fn qa_oblique_slice_record_is_orthonormal_and_exported() {
    let mut seed = 0x0b1_u64;
    for _ in 0..12 {
        let (a, b) = (draw(&mut seed), draw(&mut seed));
        let z0 = draw(&mut seed);
        let p = oblique(z0, &a, &b).expect("a plane");
        assert_eq!(p.z0, z0);
        assert!((norm(&p.q1) - 1.0).abs() <= 1e-14 && (norm(&p.q2) - 1.0).abs() <= 1e-14);
        assert!(
            dot(&p.q1, &p.q2).abs() <= 1e-14,
            "the pair is not orthogonal"
        );
        assert!(
            off_span(&a, &b, &p.q1) <= 1e-12 && off_span(&a, &b, &p.q2) <= 1e-12,
            "the pair does not span the oblique plane"
        );
        // Set through the store, exported from its snapshot.
        let mut s = store(planes()[0].clone());
        s.set_field(SetField {
            edit: engine::contract::set_field::Edit::Sim(
                engine::contract::set_field::SimField::Basis { q1: p.q1, q2: p.q2 },
            ),
            no_history: false,
        });
        check_export(&s.snapshot().sim, &p.q1, &p.q2);
    }
}

negative_control!(
    qa_oblique_slice_record_is_orthonormal_and_exported,
    "the export of a store never given the pair does not carry it",
    expected = "does not carry the oblique pair",
    {
        let p = oblique([0.0; 8], &draw(&mut 1_u64), &draw(&mut 2_u64)).unwrap();
        check_export(&sim(planes()[0].clone()), &p.q1, &p.q2);
    }
);

fn check_not_composed(got: &Plane, second_alone: &Plane) {
    assert!(
        norm(&sub(&got.q1, &second_alone.q1)) <= 1e-12
            && norm(&sub(&got.q2, &second_alone.q2)) <= 1e-12,
        "two raw tilts composed rather than the second replacing the first"
    );
}

#[test]
fn qa_oblique_slice_record_raw_tilts_replace() {
    for p in planes() {
        let six = hidden(&p).unwrap();
        let (d1, d2) = (six[0].direction, six[2].direction);
        let c = ChartConstructor::new(p.clone());
        for (w1, w2) in [
            (BasisVector::Q1, BasisVector::Q1),
            (BasisVector::Q2, BasisVector::Q2),
            (BasisVector::Q1, BasisVector::Q2),
        ] {
            let first = apply(&p, c.tilt(&p, w1, 35.0, &d1).unwrap());
            let got = apply(&first, c.tilt(&first, w2, -20.0, &d2).unwrap());
            let alone = apply(&p, c.tilt(&p, w2, -20.0, &d2).unwrap());
            check_not_composed(&got, &alone);
            // And it differs from the composition.
            let composed = apply(
                &first,
                ChartConstructor::new(first.clone())
                    .tilt(&first, w2, -20.0, &d2)
                    .unwrap(),
            );
            assert!(
                !same_span(&got, &composed, 1e-6),
                "the check cannot tell replace from compose"
            );
        }
    }
}

negative_control!(
    qa_oblique_slice_record_raw_tilts_replace,
    "a constructor committed to the first tilt composes",
    expected = "two raw tilts composed",
    {
        let p = planes()[0].clone();
        let six = hidden(&p).unwrap();
        let first = apply(
            &p,
            ChartConstructor::new(p.clone())
                .tilt(&p, BasisVector::Q1, 35.0, &six[0].direction)
                .unwrap(),
        );
        let mut c = ChartConstructor::new(p.clone());
        c.commit(first.clone());
        let got = apply(
            &first,
            c.tilt(&first, BasisVector::Q1, -20.0, &six[2].direction)
                .unwrap(),
        );
        let alone = apply(
            &p,
            ChartConstructor::new(p.clone())
                .tilt(&p, BasisVector::Q1, -20.0, &six[2].direction)
                .unwrap(),
        );
        check_not_composed(&got, &alone);
    }
);

/// `d_E` at a centre at rest, where every direction on the level set raises `K` only at second order: still a unit
/// momentum-block vector, `L_z` held to first order, `E` raised on both sides.
fn check_at_rest(z: &Latent, d: &Latent) {
    check_unit_in_block(d, 2..6, "d_E at rest");
    let (e0, l0) = energy_lz(z);
    assert!(l0.abs() <= 1e-15, "the centre is not at rest: L_z = {l0}");
    for h in [1e-3, -1e-3] {
        let (eh, lh) = energy_lz(&add_scaled(z, h, d));
        assert!(eh > e0, "E does not rise along d_E at rest (h = {h})");
        assert!(
            lh.abs() <= 100.0 * h * h,
            "L_z changes at first order along d_E at rest: {lh} at h = {h}"
        );
    }
}

#[test]
fn qa_named_directions_energy_at_rest_and_saturated() {
    for z in centres() {
        let mut rest = z;
        rest[2..6].copy_from_slice(&[0.0; 4]);
        let p = decoded(&rest).p;
        assert!(
            p.iter().flatten().all(|x| x.abs() <= 1e-15),
            "z[2:6] = 0 is not at rest: {p:?}"
        );
        let d = energy_at_fixed_lz::<Defaults>(&rest).expect("defined at rest");
        check_at_rest(&rest, &d);
        // A saturated momentum control has no direction: reported unavailable, never a vector.
        let mut saturated = z;
        saturated[2] = 1e3;
        assert_eq!(
            energy_at_fixed_lz::<Defaults>(&saturated),
            Err(Unavailable::SaturatedMomentum),
            "a saturated control gave a direction"
        );
    }
}

negative_control!(
    qa_named_directions_energy_at_rest_and_saturated,
    "a rotation direction at rest changes L_z at first order",
    expected = "L_z changes at first order",
    {
        let mut rest = centres()[0];
        rest[2..6].copy_from_slice(&[0.0; 4]);
        // Pure relative rotation of the Jacobi pair: δp_ρ ⟂ r_ρ, built in the momentum block's ρ part.
        let d = decoded(&rest);
        let rho = [d.r[1][0] - d.r[0][0], d.r[1][1] - d.r[0][1]];
        let mut rot = [0.0; 8];
        rot[2] = -rho[1];
        rot[3] = rho[0];
        check_at_rest(&rest, &normalise(&rot));
    }
);
