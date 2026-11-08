//! Saved charts in the kernel (`kernel::chart::saved`; R-475 S2, S3, L4 to L6; R-513 Q1 to Q3; chart_reference §1.1,
//! "Twelve is the full count; Save view as chart commits"):
//! - `saved_chart_*` (REQ-CHART-073, C8): a latent view at `τ = (30°, 20°)` saved as the oblique chart, its stored
//!   pair kept, and a shape-sphere view tilted 25° into a momentum control saved, each reopened at zero tilt bitwise
//!   at every pixel of a 33 × 33 grid, through the serialisation; a further tilt into a second momentum control is the
//!   block composition `(R ⊕ 1)·(1 ⊕ R′)`; every route of a new tilt's offsets, a panned view included, is `R·R′`
//!   with the saved targets fed to `R`'s inputs; the words read back bit for bit; the refusals; the f32 instantiation.
//! - `saved_chart_parent_*` (REQ-CHART-073; R-475 L5, R-483 H3): the parent is saved by value, its params in the
//!   words: two shape spheres differing only in `z_mass` save to different words, each reproducing its own view and
//!   never the other's; every admitted parent type reads back from its words, each param its own word; charts one
//!   name covers save to different words.
//! - `saved_chart_same_target` (REQ-CHART-077, R-513): a new tilt into the saved target composes the rotations,
//!   `R·R′·(u, v, 0)`, at zoom 1 and zoom 4 alike; into a second control the block form; at `τ′ = 0` the saved view.
//!
//! "To 1 ulp" here: each placed value differs from the reference by at most one ulp of the terms that form it,
//! `ε·Σ|terms|`, a criterion fixed before the run (R-649). Each test registers its negative control (R-176).

use kernel::chart::burrau::{Burrau, EuclidPlane};
use kernel::chart::burrau_acute::{BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz};
use kernel::chart::construction::{HeldK, HeldLz, RestStart};
use kernel::chart::energy::{Mapped, Mapping};
use kernel::chart::invariant::Invariant;
use kernel::chart::invariant::InvariantLK;
use kernel::chart::mass_simplex::MassSimplex;
use kernel::chart::mixed::{Alpha, Beta, Mixed, ZAlpha, ZBeta, ZQ1};
use kernel::chart::physical::{AxisE, AxisK, AxisKE, AxisVirial, MixedEnergy, MixedMassRatio};
use kernel::chart::saved::{
    params_len, Composed, Curved, ParentParams, SaveParent, Saved, PARAMS, WORDS,
};
use kernel::chart::shape_sphere::pixel_centre;
use kernel::chart::tilt::{latent_basis, Hidden, LatentFrame, Placement, Shift};
use kernel::chart::{
    Chart, ChartDecode, Float, InvariantLE, Latent, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec,
    Tilted, ValidationResult,
};
use kernel::decode::Decoded;
use kernel::generated::links::{Defaults, LinkParams};
use validation::negative_control;

/// The view's centre in the parent's own parameters, off `(½, ½)`.
const CENTRE: [f64; 2] = [0.37, 0.61];
/// A panned view's centre on a saved chart: its own parameters moved off the saved centre.
const PANNED: [f64; 2] = [0.42, 0.55];
/// The targets' slider spans.
const SPAN: [f64; 2] = [2.0, 2.0];
/// The grid side.
const N: u32 = 33;
/// The saved target and a second momentum control, on the shape sphere.
const X: u32 = 2;
const SECOND: u32 = 3;

/// The latent chart's frame: the plane `z_q0 × z_q1`, zoom ½, no rotation.
const FRAME: LatentFrame = LatentFrame {
    a: 2,
    b: 3,
    scale: 0.5,
    gamma: 0.0,
};
const Z0: [f64; 8] = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

fn sc(deg: f64) -> (f64, f64) {
    Float::sin_cos(Float::to_radians(deg))
}

fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    }
}

/// The pixel centres of an `n` × `n` window about `centre`, half-width `½/zoom`: at zoom 1 the parent's full range.
fn window(centre: [f64; 2], zoom: f64) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    for j in 0..N {
        for i in 0..N {
            let at = |k, c: f64| c + (pixel_centre(k, N) - 0.5) / zoom;
            points.push([at(i, centre[0]), at(j, centre[1])]);
        }
    }
    points
}

/// The saved tilt of the shape-sphere view: 25° into the momentum control `X`.
fn saved_spec() -> TiltSpec {
    spec(25.0, 0.0, TiltMap::Sheet, X, X)
}

/// The saved curved chart through its words, reopened on the sphere under `new` about `centre`.
fn reopened(
    saved: &TiltSpec,
    new: &TiltSpec,
    centre: [f64; 2],
) -> Tilted<ShapeSphere<f64>, f64, Composed<f64>> {
    let def = Saved::curved(&sphere(), *saved, CENTRE, SPAN).expect("a curved parent");
    let Some(Saved::Curved(curved)) = Saved::from_words(&def.to_words()) else {
        panic!("the words do not read back as a curved chart");
    };
    curved
        .reopen(sphere(), new, centre, SPAN)
        .expect("a composition")
}

// ── The reference: R·R′ as matrices on (a, b, h₀ … h₇) ──────────────────────────────────────────────────────────────

type M = [[f64; 10]; 10];

/// The Givens turn by `deg` from the axis `i` toward `j`: `e_i ↦ cos·e_i + sin·e_j`, `e_j ↦ −sin·e_i + cos·e_j`.
fn givens(m: &mut M, i: usize, j: usize, deg: f64) {
    let (s, c) = sc(deg);
    m[i][i] = c;
    m[j][i] = s;
    m[i][j] = -s;
    m[j][j] = c;
}

/// A stored tilt's rotation on `(a, b, h₀ … h₇)`, written out from its closed forms (R-474 K1, J4): two targets, each
/// axis turned toward its own; Tip as sheet with both angles set, Rodrigues' 3 × 3 rotation about
/// `k = (τ₂ e_a − τ₁ e_b)/m`; a single angle, its axis's turn.
fn rotation(t: &TiltSpec) -> M {
    let mut m = [[0.0; 10]; 10];
    for (k, row) in m.iter_mut().enumerate() {
        row[k] = 1.0;
    }
    let (x, y) = (2 + t.x as usize, 2 + t.y as usize);
    if t.x != t.y {
        givens(&mut m, 0, x, t.tau1);
        givens(&mut m, 1, y, t.tau2);
    } else if t.tau1 != 0.0 && t.tau2 != 0.0 {
        let deg = Float::sqrt(t.tau1 * t.tau1 + t.tau2 * t.tau2);
        let k = [t.tau2 / deg, -t.tau1 / deg, 0.0];
        let (s, c) = sc(deg);
        let cross = [[0.0, -k[2], k[1]], [k[2], 0.0, -k[0]], [-k[1], k[0], 0.0]];
        let at = [0, 1, x];
        for r in 0..3 {
            for q in 0..3 {
                let id = if r == q { c } else { 0.0 };
                m[at[r]][at[q]] = id + s * cross[r][q] + (1.0 - c) * k[r] * k[q];
            }
        }
    } else if t.tau2 == 0.0 {
        givens(&mut m, 0, x, t.tau1);
    } else {
        givens(&mut m, 1, x, t.tau2);
    }
    m
}

/// `m·x`, and for each component the sum of the magnitudes of every product that forms it, `size` carrying `x`'s own.
fn apply(m: &M, x: &[f64; 10], size: &[f64; 10]) -> ([f64; 10], [f64; 10]) {
    let mut y = [0.0; 10];
    let mut terms = [0.0; 10];
    for r in 0..10 {
        for q in 0..10 {
            y[r] += m[r][q] * x[q];
            terms[r] += m[r][q].abs() * (x[q].abs() + size[q]);
        }
    }
    (y, terms)
}

/// A pixel's placement: `(a, b)` and the offset on each latent control.
#[derive(Clone, Copy, Debug)]
struct Placed {
    ab: [f64; 2],
    d: [f64; 8],
}

/// The reference placement of the pixel at `(u, v)` of a saved chart (centre `CENTRE`, tilt `saved`) under a new tilt
/// `new` about `centre`: `(u′, v′, e′) = R′·(ũ, ṽ, 0)`, then `R·(u′ + c − s, v′ + c − s, e′)`, every offset in units of
/// its span; with each value's tolerance, one ulp of its terms: `ε` times the sum of the magnitudes of every product
/// and sum that forms it, through both rotations. `span[k]` is the slider span control `k`'s offset is scaled by.
fn reference(
    saved: &TiltSpec,
    new: &TiltSpec,
    centre: [f64; 2],
    span: &[f64; 8],
    u: f64,
    v: f64,
) -> (Placed, Placed) {
    let mut x = [0.0; 10];
    x[0] = u - centre[0];
    x[1] = v - centre[1];
    let (mut y, mut size_y) = apply(&rotation(new), &x, &x.map(f64::abs));
    for k in 0..2 {
        let pan = centre[k] - CENTRE[k];
        y[k] += pan;
        size_y[k] += pan.abs() + y[k].abs();
    }
    let (z, size) = apply(&rotation(saved), &y, &size_y);
    let ab = [CENTRE[0] + z[0], CENTRE[1] + z[1]];
    let tol_ab = [
        f64::EPSILON * (CENTRE[0].abs() + size[0] + ab[0].abs()),
        f64::EPSILON * (CENTRE[1].abs() + size[1] + ab[1].abs()),
    ];
    let mut d = [0.0; 8];
    let mut tol_d = [0.0; 8];
    for k in 0..8 {
        d[k] = span[k] * z[2 + k];
        tol_d[k] = f64::EPSILON * span[k] * size[2 + k];
    }
    (
        Placed { ab, d },
        Placed {
            ab: tol_ab,
            d: tol_d,
        },
    )
}

/// A kernel shift as one offset per latent control.
fn offsets(shift: &Shift<f64>) -> [f64; 8] {
    let mut d = [0.0; 8];
    d[shift.x as usize] += shift.dx;
    if shift.two {
        d[shift.y as usize] += shift.dy;
    }
    d
}

/// Whether `got` is `want` within `tol`, value by value.
fn within(got: &Placed, want: &Placed, tol: &Placed) -> bool {
    let close = |g: f64, w: f64, t: f64| (g - w).abs() <= t;
    (0..2).all(|k| close(got.ab[k], want.ab[k], tol.ab[k]))
        && (0..8).all(|k| close(got.d[k], want.d[k], tol.d[k]))
}

/// How a check forms the pixel's placement: the kernel's, or the control's wrong composition.
#[derive(Clone, Copy, PartialEq)]
enum Got {
    /// The saved chart's own placement.
    Kernel,
    /// RQ-403's option (a): the saved `R` on `(u′, v′, 0)`, and the new offset added to the saved one, `d + d′`.
    Added,
}

/// The placement `got` forms at `(u, v)`, on the saved chart `view` whose tilts are `saved` and `new`.
fn placed(view: &Tilted<ShapeSphere<f64>, f64, Composed<f64>>, got: Got, u: f64, v: f64) -> Placed {
    let composed = view.tilt();
    if got == Got::Added {
        let ([up, vp], new) = composed.new_tilt().place(u, v);
        let (ab, saved) = composed.saved().place(up, vp);
        let mut d = offsets(&saved);
        let added = offsets(&new);
        for k in 0..8 {
            d[k] += added[k];
        }
        return Placed { ab, d };
    }
    let (ab, shift) = composed.place(u, v);
    Placed {
        ab,
        d: offsets(&shift),
    }
}

/// Over the window about `centre` at `zoom`, the saved chart (built from `built`, the saved and new tilts) places each
/// pixel as `R·R′` of `want` to one ulp, and decodes there: the sphere at `(a, b)` with each offset on its control.
fn check_composes(
    built: (TiltSpec, TiltSpec),
    want: (TiltSpec, TiltSpec),
    centre: [f64; 2],
    zoom: f64,
    got: Got,
) {
    let view = reopened(&built.0, &built.1, centre);
    let mut worst = 0.0f64;
    for [u, v] in window(centre, zoom) {
        let have = placed(&view, got, u, v);
        let (expect, tol) = reference(&want.0, &want.1, centre, &[SPAN[0]; 8], u, v);
        assert!(
            within(&have, &expect, &tol),
            "the saved chart at ({u}, {v}) is not the composition R·R′: {have:?} against {expect:?}"
        );
        for k in 0..2 {
            worst = worst.max((have.ab[k] - expect.ab[k]).abs() / tol.ab[k].max(f64::MIN_POSITIVE));
        }
        let mut chart = sphere();
        for (k, d) in have.d.iter().enumerate() {
            if *d != 0.0 {
                let held = chart.map(0.5, 0.5).z.z[k];
                chart = chart.sliced(k as u32, held + d).expect("a hidden control");
            }
        }
        if got == Got::Kernel {
            let [a, b] = have.ab;
            assert!(
                bits(&view.decode::<Defaults, f64>(u, v)) == bits(&chart.decode::<Defaults, f64>(a, b)),
                "the saved chart at ({u}, {v}) does not decode at (a, b) with its offsets on the controls"
            );
        }
    }
    eprintln!("composition at zoom {zoom}: worst (a, b) error {worst:.3} of its tolerance");
}

// ── C8: a latent view saved as the oblique chart, reopened bitwise ──────────────────────────────────────────────────

/// A latent view at `τ = (30°, 20°)` under each map, saved, written, read back and reopened, decodes bitwise at every
/// pixel of a 33 × 33 grid. `orthonormalise` re-orthonormalises the stored pair on reopening (the control: under
/// Lean each axis the pair is skewed, so the chart is another).
fn check_latent(orthonormalise: bool) {
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let (q1, q2) =
            latent_basis(&FRAME, &spec(30.0, 20.0, map, 4, 4)).expect("a target off the plane");
        let view = Latent { z0: Z0, q1, q2 };
        let words = Saved::latent(&view).to_words();
        let mut chart = Saved::from_words(&words)
            .and_then(|s| s.oblique())
            .expect("an oblique chart");
        if orthonormalise {
            let dot = |p: [f64; 8], q: [f64; 8]| p.iter().zip(&q).map(|(x, y)| x * y).sum::<f64>();
            let n1 = dot(chart.q1, chart.q1).sqrt();
            let e1 = chart.q1.map(|x| x / n1);
            let along = dot(chart.q2, e1);
            let mut perp = chart.q2;
            for k in 0..8 {
                perp[k] -= along * e1[k];
            }
            let n2 = dot(perp, perp).sqrt();
            chart.q1 = e1.map(|x| x * n1);
            chart.q2 = perp.map(|x| x / n2 * dot(chart.q2, chart.q2).sqrt());
        }
        for [u, v] in window([0.5, 0.5], 1.0) {
            assert!(
                bits(&chart.decode::<Defaults, f64>(u, v)) == bits(&view.decode::<Defaults, f64>(u, v)),
                "the oblique chart reopened under {map:?} at ({u}, {v}) is not bitwise the saved view"
            );
        }
    }
}

#[test]
fn saved_chart_latent_reopens_bitwise() {
    check_latent(false);
}

negative_control!(
    saved_chart_latent_reopens_bitwise,
    "an oblique chart whose skewed pair is re-orthonormalised is another chart, so it must fail",
    expected = "is not bitwise the saved view",
    {
        check_latent(true);
    }
);

// ── C8: a curved view saved, reopened at zero tilt bitwise ──────────────────────────────────────────────────────────

/// The shape-sphere view tilted 25° into a momentum control, and the same view untilted, saved and reopened at
/// `τ′ = 0` (into each target, about the view's centre or a panned one): its decode, `map` and `validate` at every
/// pixel of a 33 × 33 grid are bitwise the saved view's. `lost` reopens with the saved centre lost, `(½, ½)` (the
/// control).
fn check_curved(lost: bool) {
    for saved in [saved_spec(), spec(0.0, 0.0, TiltMap::Sheet, X, X)] {
        let view = Tilted::new(sphere(), Tilt::new(&saved, CENTRE, SPAN)).expect("hidden targets");
        let mut def = Saved::curved(&sphere(), saved, CENTRE, SPAN).expect("a curved parent");
        if lost {
            if let Saved::Curved(c) = &mut def {
                c.centre = [0.5, 0.5];
            }
        }
        let Some(Saved::Curved(curved)) = Saved::from_words(&def.to_words()) else {
            panic!("the words do not read back as a curved chart");
        };
        for (new, centre) in [
            (spec(0.0, 0.0, TiltMap::Sheet, X, X), CENTRE),
            (spec(0.0, 0.0, TiltMap::Lean, SECOND, 6), PANNED),
        ] {
            let chart = curved
                .reopen(sphere(), &new, centre, SPAN)
                .expect("a composition");
            assert!(
                chart.tilt().on() == view.tilt().on,
                "the saved chart reopened at τ′ = 0 takes a tilt path where the saved view takes none, so it is \
                 not bitwise the saved view"
            );
            for [u, v] in window(CENTRE, 1.0) {
                assert!(
                    bits(&chart.decode::<Defaults, f64>(u, v)) == bits(&view.decode::<Defaults, f64>(u, v))
                        && bits(&chart.decode_at_params::<Defaults, f64>(u, v, &LinkParams::DEFAULT))
                            == bits(&view.decode::<Defaults, f64>(u, v))
                        && format!("{:?}", chart.map::<f64>(u, v)) == format!("{:?}", view.map::<f64>(u, v))
                        && chart.validate(u, v) == view.validate(u, v),
                    "the saved chart reopened at τ′ = 0, at ({u}, {v}), is not bitwise the saved view"
                );
            }
        }
    }
}

#[test]
fn saved_chart_curved_reopens_bitwise() {
    check_curved(false);
}

negative_control!(
    saved_chart_curved_reopens_bitwise,
    "a saved chart that loses its slice centre is not the saved view, so it must fail",
    expected = "is not bitwise the saved view",
    {
        check_curved(true);
    }
);

// ── REQ-CHART-073: into a second control, the block form (R ⊕ 1)·(1 ⊕ R′) ───────────────────────────────────────────

/// The new tilt `τ′ = (20°, 0)` into the second momentum control: it turns `a` as the saved tilt does, so the two
/// orders differ (a turn of `b` into it would commute with `R`, the planes `(a, X)` and `(b, Y′)` disjoint).
fn second() -> TiltSpec {
    spec(20.0, 0.0, TiltMap::Sheet, SECOND, SECOND)
}

#[test]
fn saved_chart_block_form() {
    check_composes(
        (saved_spec(), second()),
        (saved_spec(), second()),
        CENTRE,
        1.0,
        Got::Kernel,
    );
}

negative_control!(
    saved_chart_block_form,
    "the 3 × 3 product R·R′, which feeds d′ into R's third input, is not the block form, so it must fail",
    expected = "is not the composition R·R′",
    {
        let into_saved = spec(20.0, 0.0, TiltMap::Sheet, X, X);
        check_composes((saved_spec(), into_saved), (saved_spec(), second()), CENTRE, 1.0, Got::Kernel);
    }
);

#[test]
fn saved_chart_block_order() {
    check_composes(
        (saved_spec(), second()),
        (saved_spec(), second()),
        CENTRE,
        4.0,
        Got::Kernel,
    );
}

negative_control!(
    saved_chart_block_order,
    "the composition in the order R′·R is another, so it must fail",
    expected = "is not the composition R·R′",
    {
        let first = spec(20.0, 0.0, TiltMap::Sheet, SECOND, SECOND);
        let then = spec(25.0, 0.0, TiltMap::Sheet, X, X);
        check_composes(
            (first, then),
            (saved_spec(), second()),
            CENTRE,
            1.0,
            Got::Kernel,
        );
    }
);

// ── Every route of a new tilt's offsets (R-513 Q1) ──────────────────────────────────────────────────────────────────

/// Saved and new tilts over every route: one saved target or two, Tip as sheet with both angles set, a new tilt into
/// the saved target, into the saved second target, into another, and with two targets one of them saved (either one),
/// and a saved view untilted, its `R` the identity; each about the view's centre and a panned one, as `R·R′` to one
/// ulp. `swap` composes each pair the other way (the control).
fn check_routes(swap: bool) {
    let cases = [
        (
            spec(25.0, 0.0, TiltMap::Lean, 2, 2),
            spec(0.0, 20.0, TiltMap::Lean, 2, 2),
        ),
        (
            spec(30.0, 40.0, TiltMap::Sheet, 4, 4),
            spec(15.0, -35.0, TiltMap::Sheet, 4, 4),
        ),
        (
            spec(30.0, 40.0, TiltMap::Sheet, 4, 4),
            spec(-20.0, 10.0, TiltMap::Lean, 6, 4),
        ),
        (
            spec(0.0, -50.0, TiltMap::Sheet, 6, 6),
            spec(35.0, 0.0, TiltMap::Sheet, 6, 6),
        ),
        (
            spec(25.0, -30.0, TiltMap::Lean, 2, 5),
            spec(20.0, 0.0, TiltMap::Sheet, 5, 5),
        ),
        (
            spec(25.0, -30.0, TiltMap::Lean, 2, 5),
            spec(20.0, 40.0, TiltMap::Lean, 5, 2),
        ),
        (
            spec(25.0, 0.0, TiltMap::Lean, 2, 2),
            spec(20.0, 40.0, TiltMap::Lean, 7, 2),
        ),
        (
            spec(-40.0, 0.0, TiltMap::Sheet, 3, 3),
            spec(10.0, 25.0, TiltMap::Sheet, 7, 7),
        ),
        (
            spec(25.0, 0.0, TiltMap::Lean, 2, 2),
            spec(-15.0, 30.0, TiltMap::Sheet, 2, 5),
        ),
        (
            spec(0.0, 0.0, TiltMap::Sheet, 3, 3),
            spec(30.0, -20.0, TiltMap::Lean, 4, 5),
        ),
    ];
    for (saved, new) in cases {
        let built = if swap { (new, saved) } else { (saved, new) };
        for centre in [CENTRE, PANNED] {
            check_composes(built, (saved, new), centre, 1.0, Got::Kernel);
        }
    }
}

#[test]
fn saved_chart_composes_every_route() {
    check_routes(false);
}

negative_control!(
    saved_chart_composes_every_route,
    "each composition taken in the order R′·R is another, so it must fail",
    expected = "is not the composition R·R′",
    {
        check_routes(true);
    }
);

// ── The spans each offset is scaled by (R-513 Q2) ───────────────────────────────────────────────────────────────────

/// The saved tilt's slider spans, unequal, and the new tilt's, unequal and unlike them.
const SAVED_SPAN: [f64; 2] = [1.5, 2.5];
const NEW_SPAN: [f64; 2] = [0.7, 3.0];

/// With unequal spans, each offset is scaled by the span R-513 Q2 names for it, at every pixel of the window about the
/// view's centre and a panned one, to one ulp: an offset out of `R` on a saved target by that target's saved span,
/// whatever span the new tilt names for it, and a new offset on another target by the new tilt's span for it (code
/// review 5816802860 of PR #447, C3); a control no tilt names, span 0, never moves. Each route's spans are written out
/// per control. `swap` builds the chart with the saved and new spans exchanged (the control).
fn check_spans(swap: bool) {
    let (built_saved, built_new) = if swap {
        (NEW_SPAN, SAVED_SPAN)
    } else {
        (SAVED_SPAN, NEW_SPAN)
    };
    // (saved tilt, new tilt, the span of each control's offset)
    let routes = [
        // Two saved targets, the new tilt into the first: X's offset by X's saved span, Y's by Y's, the new span unread.
        (
            spec(25.0, -30.0, TiltMap::Lean, 2, 3),
            spec(20.0, 0.0, TiltMap::Sheet, 2, 2),
            [(2, 1.5), (3, 2.5)],
        ),
        // Two saved targets, the new tilt into the second.
        (
            spec(25.0, -30.0, TiltMap::Lean, 2, 3),
            spec(0.0, 20.0, TiltMap::Sheet, 3, 3),
            [(2, 1.5), (3, 2.5)],
        ),
        // One saved target, the new tilt's first target saved and its second another: that one by its new span.
        (
            spec(25.0, 0.0, TiltMap::Sheet, 2, 2),
            spec(20.0, 10.0, TiltMap::Lean, 2, 4),
            [(2, 1.5), (4, 3.0)],
        ),
        // One saved target, the new tilt's first target another, by its new span, and its second saved.
        (
            spec(25.0, 0.0, TiltMap::Sheet, 2, 2),
            spec(20.0, 10.0, TiltMap::Lean, 4, 2),
            [(2, 1.5), (4, 0.7)],
        ),
    ];
    for (saved, new, spans) in routes {
        let mut span = [0.0; 8];
        for (k, s) in spans {
            span[k] = s;
        }
        let def = Saved::curved(&sphere(), saved, CENTRE, built_saved).expect("a curved parent");
        let Some(Saved::Curved(curved)) = Saved::from_words(&def.to_words()) else {
            panic!("the words do not read back as a curved chart");
        };
        for centre in [CENTRE, PANNED] {
            let view = curved
                .reopen(sphere(), &new, centre, built_new)
                .expect("a composition");
            for [u, v] in window(centre, 1.0) {
                let have = placed(&view, Got::Kernel, u, v);
                let (expect, tol) = reference(&saved, &new, centre, &span, u, v);
                assert!(
                    within(&have, &expect, &tol),
                    "the saved chart at ({u}, {v}) scales an offset by another span: {have:?} against {expect:?}"
                );
            }
        }
    }
}

#[test]
fn saved_chart_spans() {
    check_spans(false);
}

negative_control!(
    saved_chart_spans,
    "the saved and new tilts' spans exchanged scale each offset by another span, so it must fail",
    expected = "scales an offset by another span",
    {
        check_spans(true);
    }
);

// ── REQ-CHART-077: into the saved target, the rotations compose (R-513) ─────────────────────────────────────────────

/// The new tilt `τ′ = (0, 20°)` into the saved target: non-parallel to the saved `τ = (25°, 0)`, so the two orders
/// differ.
fn same() -> TiltSpec {
    spec(0.0, 20.0, TiltMap::Sheet, X, X)
}

/// R-513 at zoom 1 and zoom 4: into the saved target `R·R′·(u, v, 0)` to one ulp at every pixel of a 33 × 33 grid, the
/// decode the sphere's there with `d` on the saved control; into a second control the block form; at `τ′ = 0` the
/// saved view, bitwise. `got` forms the placement (the control's: the offsets added).
fn check_same_target(got: Got) {
    for zoom in [1.0, 4.0] {
        check_composes(
            (saved_spec(), same()),
            (saved_spec(), same()),
            CENTRE,
            zoom,
            got,
        );
        let into_second = spec(0.0, 20.0, TiltMap::Sheet, SECOND, SECOND);
        check_composes(
            (saved_spec(), into_second),
            (saved_spec(), into_second),
            CENTRE,
            zoom,
            got,
        );
    }
    check_curved(false);
}

#[test]
fn saved_chart_same_target() {
    check_same_target(Got::Kernel);
}

negative_control!(
    saved_chart_same_target,
    "the offsets added, d + d′ (RQ-403's option (a)), are not the rotations composed, so it must fail",
    expected = "is not the composition R·R′",
    {
        check_same_target(Got::Added);
    }
);

#[test]
fn saved_chart_same_target_order() {
    check_composes(
        (saved_spec(), same()),
        (saved_spec(), same()),
        CENTRE,
        1.0,
        Got::Kernel,
    );
}

negative_control!(
    saved_chart_same_target_order,
    "into the saved target the order R′·R is another rotation, so it must fail",
    expected = "is not the composition R·R′",
    {
        let first = spec(0.0, 20.0, TiltMap::Sheet, X, X);
        check_composes(
            (first, saved_spec()),
            (saved_spec(), same()),
            CENTRE,
            1.0,
            Got::Kernel,
        );
    }
);

// ── R's inputs: the third column of a rotation ──────────────────────────────────────────────────────────────────────

/// Every stored tilt that is a rotation gives `R·e_X` (and with two targets `R·e_Y`) as the reference rotation's
/// column, to one ulp; Lean each axis with one target and both angles set gives none (RQ-746). `flip` negates the
/// input, a reflection's (the control).
fn check_inputs(flip: f64) {
    let rotations = [
        spec(0.0, 0.0, TiltMap::Sheet, 4, 4),
        spec(35.0, 0.0, TiltMap::Lean, 4, 4),
        spec(0.0, -60.0, TiltMap::Sheet, 4, 4),
        spec(30.0, 40.0, TiltMap::Sheet, 4, 4),
        spec(-120.0, 60.0, TiltMap::Sheet, 4, 4),
        spec(30.0, 40.0, TiltMap::Lean, 4, 6),
        spec(-70.0, 15.0, TiltMap::Sheet, 6, 4),
    ];
    for t in rotations {
        let r = rotation(&t);
        let [ix, iy] = t.inputs().expect("a rotation").map(|c| c.map(|x| flip * x));
        let (x, y) = (2 + t.x as usize, 2 + t.y as usize);
        let rows = [0, 1, x, y];
        for (k, &row) in rows.iter().enumerate().take(if t.x == t.y { 3 } else { 4 }) {
            assert!(
                (ix[k] - r[row][x]).abs() <= 4.0 * f64::EPSILON,
                "R·e_X of {t:?} is not the rotation's column: {ix:?}"
            );
            if t.x != t.y {
                assert!(
                    (iy[k] - r[row][y]).abs() <= 4.0 * f64::EPSILON,
                    "R·e_Y of {t:?} is not the rotation's column: {iy:?}"
                );
            }
        }
        if t.x == t.y {
            assert!(
                iy == [0.0, 0.0, 0.0, flip],
                "R·e_Y of {t:?} is not the rotation's column: {iy:?}"
            );
        }
    }
    assert!(
        spec(30.0, 40.0, TiltMap::Lean, 4, 4).inputs().is_none(),
        "R·e_X of a skewed Lean each axis view is defined"
    );
}

#[test]
fn saved_chart_inputs() {
    check_inputs(1.0);
}

negative_control!(
    saved_chart_inputs,
    "a reflection's third column is not the rotation's, so it must fail",
    expected = "is not the rotation's column",
    {
        check_inputs(-1.0);
    }
);

// ── The serialisation ───────────────────────────────────────────────────────────────────────────────────────────────

/// The definitions `alter`ed after writing read back bit for bit, and every malformed word list reads as `None`.
fn check_words(alter: fn(&mut [f64; WORDS])) {
    let mut z0 = Z0;
    z0[1] = -0.0;
    z0[5] = f64::MIN_POSITIVE / 8.0;
    z0[7] = 1e300;
    let oblique = Saved::latent(&Latent {
        z0,
        q1: [core::f64::consts::PI; 8],
        q2: [-1.0 / 3.0; 8],
    });
    let curved = Saved::curved(
        &sphere(),
        spec(-0.0, 1.0 / 7.0, TiltMap::Lean, 7, 2),
        [-0.25, 1e-310],
        [0.1, 3.5],
    )
    .expect("a curved parent");
    for def in [oblique, curved] {
        let mut words = def.to_words();
        alter(&mut words);
        let back = Saved::from_words(&words).expect("a definition");
        let same = back.to_words().map(f64::to_bits) == def.to_words().map(f64::to_bits);
        assert!(
            same && back == def,
            "{def:?} does not read back bit for bit"
        );
    }
    let good = curved.to_words();
    let set = |k: usize, value: f64| {
        let mut w = good;
        w[k] = value;
        w
    };
    for (bad, what) in [
        (set(0, 2.0), "a kind past curved"),
        (set(0, -0.0), "a negative-zero kind"),
        (set(1, 0.0), "the latent chart as a curved parent"),
        (set(1, 11.0), "a parent past the registry"),
        (set(1, 1.5), "a parent that is no index"),
        (set(6, 2.0), "a map past Lean each axis"),
        (set(7, -1.0), "a negative target"),
        (set(8, 4294967296.0), "a target past u32"),
        (set(4, f64::NAN), "a NaN angle"),
        (set(2, f64::INFINITY), "an infinite centre"),
        (set(11, 0.5), "a form that is no index"),
        (set(11, -1.0), "a negative form"),
    ] {
        assert!(
            Saved::from_words(&bad).is_none(),
            "{what} reads as a definition"
        );
    }
    for (edge, what) in [
        (set(1, 10.0), "the registry's last chart as the parent"),
        (set(7, 4294967295.0), "the last u32 as a target"),
    ] {
        assert!(
            Saved::from_words(&edge).is_some(),
            "{what} does not read as a definition"
        );
    }
    let mut nan = oblique.to_words();
    nan[13] = f64::NAN;
    assert!(
        Saved::from_words(&nan).is_none(),
        "a NaN vector reads as a definition"
    );
    for k in [25, WORDS - 1] {
        let mut padded = oblique.to_words();
        padded[k] = -0.0;
        assert!(
            Saved::from_words(&padded).is_none(),
            "an oblique chart's padding word other than +0 reads as a definition"
        );
    }
    assert!(
        Saved::curved(
            &Latent {
                z0: Z0,
                q1: Z0,
                q2: Z0
            },
            saved_spec(),
            CENTRE,
            SPAN
        )
        .is_none(),
        "a latent view saves as a curved chart"
    );
    assert!(
        oblique.oblique().is_some() && curved.oblique().is_none(),
        "only an oblique definition reopens as the latent chart"
    );
}

#[test]
fn saved_chart_words() {
    check_words(|_| {});
}

negative_control!(
    saved_chart_words,
    "an angle rounded through f32 on the way is not the definition, so it must fail",
    expected = "does not read back bit for bit",
    {
        check_words(|w| w[5] = f64::from(w[5] as f32));
    }
);

// ── Parents: a tilted chart is no parent ────────────────────────────────────────────────────────────────────────────

/// A type, probed at run time for the bounds below: method resolution takes a by-value arm, `true`, only where the
/// type meets its bound, and falls back to the autoref arm, `false`, where it does not.
struct Probe<T>(core::marker::PhantomData<T>);

trait SaveParentArm {
    fn save_parent(&self) -> bool {
        true
    }
}
impl<T: SaveParent> SaveParentArm for Probe<T> {}
trait SaveParentMiss {
    fn save_parent(&self) -> bool {
        false
    }
}
impl<T> SaveParentMiss for &Probe<T> {}

trait ChartArm {
    fn chart(&self) -> bool {
        true
    }
}
impl<T: Chart> ChartArm for Probe<T> {}
trait ChartMiss {
    fn chart(&self) -> bool {
        false
    }
}
impl<T> ChartMiss for &Probe<T> {}

/// Whether the type `$t` meets `SaveParent`, and whether it meets `Chart`.
macro_rules! meets {
    ($t:ty) => {{
        let probe = &Probe::<$t>(core::marker::PhantomData);
        (probe.save_parent(), probe.chart())
    }};
}

/// `Saved::curved` takes a bare chart as a parent, the shape sphere and the latent chart (which it refuses at run
/// time), and refuses by type a reopened saved chart and a tilted view, which name their parent, so saving them would
/// drop the saved `R` (code review 5816802860 of PR #447, C2), and what is no chart. `bound_chart` reads the bound as
/// `Chart`, which every chart meets (the control: the bound before the fix).
fn check_parents(bound_chart: bool) {
    let taken = |(save_parent, chart): (bool, bool)| if bound_chart { chart } else { save_parent };
    let admitted = [
        (taken(meets!(ShapeSphere<f64>)), "the shape sphere"),
        (taken(meets!(Latent<f64>)), "the latent chart"),
    ];
    let refused = [
        (
            taken(meets!(Tilted<ShapeSphere<f64>, f64, Composed<f64>>)),
            "a reopened saved chart, its saved R dropped",
        ),
        (
            taken(meets!(Tilted<ShapeSphere<f64>, f64>)),
            "a tilted view, its tilt dropped",
        ),
        (taken(meets!(TiltSpec)), "a tilt, which is no chart"),
    ];
    for (taken, what) in admitted {
        assert!(taken, "{what} is not taken as a saved chart's parent");
    }
    for (taken, what) in refused {
        assert!(!taken, "{what}, is taken as a saved chart's parent");
    }
    let reopened = reopened(&saved_spec(), &second(), CENTRE);
    assert!(
        reopened.name() == "ShapeSphere" && reopened.tilt().saved().on,
        "a reopened saved chart does not name its parent under its saved tilt, so the bound guards nothing"
    );
}

#[test]
fn saved_chart_parents() {
    check_parents(false);
}

negative_control!(
    saved_chart_parents,
    "a parent bounded as any chart takes a reopened saved chart, dropping its R, so it must fail",
    expected = "is taken as a saved chart's parent",
    {
        check_parents(true);
    }
);

// ── Refusals ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Reopenings that must be refused: a new tilt into the target of a saved Lean view with one target and both angles
/// set (RQ-746), a composition onto three hidden coordinates, a parent other than the saved one, a target that is not
/// hidden; and two that must not. `admitted` is checked as refused (the control).
fn check_refused(admitted: bool) {
    let lean = spec(30.0, 40.0, TiltMap::Lean, 4, 4);
    let curved = |t: TiltSpec| match Saved::curved(&sphere(), t, CENTRE, SPAN) {
        Some(Saved::Curved(c)) => c,
        _ => panic!("a curved parent"),
    };
    let reopen = |c: &Curved, new: TiltSpec| c.reopen(sphere(), &new, CENTRE, SPAN).is_some();
    let refused = [
        (
            reopen(&curved(lean), spec(10.0, 0.0, TiltMap::Sheet, 4, 4)),
            "into a skewed R's target",
        ),
        (
            reopen(&curved(lean), spec(10.0, 5.0, TiltMap::Lean, 5, 4)),
            "into a skewed R's target as a second one",
        ),
        (
            reopen(&curved(saved_spec()), spec(10.0, 5.0, TiltMap::Lean, 5, 6)),
            "onto three controls",
        ),
        (
            reopen(
                &curved(spec(10.0, 5.0, TiltMap::Lean, 2, 3)),
                spec(10.0, 0.0, TiltMap::Sheet, 4, 4),
            ),
            "onto three controls",
        ),
        (
            reopen(&curved(saved_spec()), spec(10.0, 0.0, TiltMap::Sheet, 0, 0)),
            "into a control the axes write",
        ),
        (
            curved(saved_spec())
                .reopen(
                    InvariantLE(invariant()),
                    &spec(0.0, 0.0, TiltMap::Sheet, X, X),
                    CENTRE,
                    SPAN,
                )
                .is_some(),
            "on another parent",
        ),
    ];
    let admits = [
        (
            reopen(&curved(lean), spec(10.0, 0.0, TiltMap::Sheet, 5, 5)),
            "a skewed R tilted into another target",
        ),
        (
            reopen(&curved(saved_spec()), spec(10.0, 5.0, TiltMap::Lean, X, 6)),
            "two new targets, one saved",
        ),
    ];
    for (taken, what) in refused {
        assert!(!taken, "a reopening {what} was not refused");
    }
    for (taken, what) in admits {
        assert!(taken != admitted, "a reopening {what} was not refused");
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

#[test]
fn saved_chart_refusals() {
    check_refused(false);
}

negative_control!(
    saved_chart_refusals,
    "an admitted reopening checked as refused must fail",
    expected = "was not refused",
    {
        check_refused(true);
    }
);

// ── Entries: the mass flag, and the f32 instantiation ───────────────────────────────────────────────────────────────

/// The saved chart's entries: per-pixel masses only where a tilt moves a mass logit, the name the parent's; and at f32
/// (R-118, J5), its params rounded, each placed value within four f32 ulps of the f64 placement `against`'s (the
/// control's another composition), and a system decoded there.
fn check_entries(against: TiltSpec) {
    let view = reopened(&saved_spec(), &second(), CENTRE);
    assert_eq!(
        view.name(),
        "ShapeSphere",
        "a saved chart's name is not its parent's"
    );
    let into_mass = reopened(
        &saved_spec(),
        &spec(15.0, 0.0, TiltMap::Sheet, 6, 6),
        CENTRE,
    );
    assert!(
        !sphere().requires_per_pixel_mass()
            && !view.requires_per_pixel_mass()
            && into_mass.requires_per_pixel_mass(),
        "a saved chart's mass flag does not follow its tilts"
    );
    let narrow = Composed::new(&saved_spec(), CENTRE, SPAN, &same(), CENTRE, SPAN)
        .expect("a composition")
        .at_precision::<f32>();
    let (u, v) = (0.81, 0.23);
    let (ab, wide) = reopened(&saved_spec(), &against, CENTRE).tilt().place(u, v);
    let (ab32, shift32) = narrow.place(u as f32, v as f32);
    let close =
        |w: f64, n: f32| (w as f32 - n).abs() <= 4.0 * f32::EPSILON * (w.abs() as f32).max(1.0);
    assert!(
        close(ab[0], ab32[0])
            && close(ab[1], ab32[1])
            && close(wide.dx, shift32.dx)
            && shift32.x == wide.x,
        "the f32 composition is not the f64 one rounded: {ab:?} {ab32:?} {wide:?} {shift32:?}"
    );
    let chart32 = Tilted::placed(
        ShapeSphere::<f32> {
            map: ShapeMap::Spherical,
            n0: [1.0, 0.0, 0.0],
            e1: [0.0, 1.0, 0.0],
            e2: [0.0, 0.0, 1.0],
            scale: 1.0,
            inertia: 1.0,
            phase: 0.0,
            z_mass: [0.3, -0.4],
            z_mom: [0.2, -0.1, 0.15, 0.05],
        },
        narrow,
    )
    .expect("hidden targets");
    let d = chart32.decode_at::<Defaults, f32>(u as f32, v as f32);
    assert!(
        d.ic.m.iter().all(|m| m.is_finite() && *m > 0.0)
            && chart32.validate(u, v) == ValidationResult::Valid,
        "the f32 saved chart does not decode a system: {d:?}"
    );
}

#[test]
fn saved_chart_entries() {
    check_entries(same());
}

negative_control!(
    saved_chart_entries,
    "the f32 composition compared with another composition's f64 placement must fail",
    expected = "is not the f64 one rounded",
    {
        check_entries(second());
    }
);

// ── The parent by value (R-475 L5, R-483 H3; physics review 5816908974 of PR #447, P1) ───────────────────────────────

/// The first word of a curved chart's parent: its form, then its params.
const HEAD: usize = WORDS - PARAMS;

/// The curved chart the words hold.
fn curved_of(words: &[f64; WORDS]) -> Curved {
    match Saved::from_words(words) {
        Some(Saved::Curved(c)) => c,
        _ => panic!("the words do not read back as a curved chart"),
    }
}

/// The words `parent` saves to, under the saved tilt.
fn words_of<C: SaveParent + ParentParams>(parent: &C) -> [f64; WORDS] {
    Saved::curved(parent, saved_spec(), CENTRE, SPAN)
        .expect("a curved parent")
        .to_words()
}

/// Two shape spheres differing only in `z_mass`, saved under the saved tilt: their words differ, and each set, opened
/// from the definition alone at `τ′ = 0`, reproduces its own saved view bitwise at every pixel of a 33 × 33 grid and
/// never the other's; reopening on the other parent is refused. `drop` drops the params from the words (the
/// control).
fn check_params(drop: bool) {
    let a = ShapeSphere {
        z_mass: [0.5, -0.3],
        ..sphere()
    };
    let b = ShapeSphere {
        z_mass: [0.0, 0.0],
        ..sphere()
    };
    let words = [a, b].map(|p| {
        let mut w = words_of(&p);
        if drop {
            w[HEAD..].fill(0.0);
        }
        w
    });
    assert!(
        words[0].map(f64::to_bits) != words[1].map(f64::to_bits),
        "two parents differing only in z_mass give the same words"
    );
    let untilted = spec(0.0, 0.0, TiltMap::Sheet, X, X);
    for (own, other, words) in [(a, b, &words[0]), (b, a, &words[1])] {
        let curved = curved_of(words);
        let chart = curved
            .open::<ShapeSphere<f64>>(&untilted, CENTRE, SPAN)
            .expect("the definition opens");
        let view = |p: ShapeSphere<f64>| {
            Tilted::new(p, Tilt::new(&saved_spec(), CENTRE, SPAN)).expect("hidden targets")
        };
        let (own_view, other_view) = (view(own), view(other));
        for [u, v] in window(CENTRE, 1.0) {
            let got = bits(&chart.decode::<Defaults, f64>(u, v));
            assert!(
                got == bits(&own_view.decode::<Defaults, f64>(u, v)),
                "the words of a parent do not reproduce its own view at ({u}, {v})"
            );
            assert!(
                got != bits(&other_view.decode::<Defaults, f64>(u, v)),
                "the words of a parent reproduce the other parent's view at ({u}, {v})"
            );
        }
        assert!(
            curved.reopen(own, &untilted, CENTRE, SPAN).is_some()
                && curved.reopen(other, &untilted, CENTRE, SPAN).is_none(),
            "reopening does not refuse a parent whose params are not the saved ones"
        );
    }
}

#[test]
fn saved_chart_parent_params() {
    check_params(false);
}

negative_control!(
    saved_chart_parent_params,
    "words that drop the parent's params cannot tell the two parents apart, so it must fail",
    expected = "give the same words",
    {
        check_params(true);
    }
);

/// `x` moved by one ulp: another finite value, and no index.
fn bump(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// The parent `c`, saved, reads back from its words (`alter`ed after writing) as itself, its words bit for bit; each
/// of its `params_len` words moved by one ulp reads as another parent whose words are those, except its `codes`
/// words, a type's code, an enum or an index, which read as no parent; and every word past them is `+0`, any other
/// value refused.
fn check_parent<C>(c: C, codes: usize, what: &str, alter: fn(&mut [f64; WORDS]))
where
    C: SaveParent + ParentParams + Copy + PartialEq + core::fmt::Debug,
{
    let mut words = words_of(&c);
    alter(&mut words);
    let back = curved_of(&words).parent::<C>();
    let same = back.is_some_and(|b| words_of(&b).map(f64::to_bits) == words.map(f64::to_bits));
    assert!(
        back == Some(c) && same,
        "{what} does not read back as its parent: {back:?}"
    );
    let params = HEAD + 1..HEAD + 1 + params_len(&c);
    let mut refused = 0;
    for k in params.clone() {
        let mut moved = words;
        moved[k] = bump(words[k]);
        match curved_of(&moved).parent::<C>() {
            Some(p) => assert!(
                p != c && words_of(&p).map(f64::to_bits) == moved.map(f64::to_bits),
                "{what}: a change of its word {k} is not a change of one param"
            ),
            None => refused += 1,
        }
    }
    assert_eq!(
        refused, codes,
        "{what}: {refused} of its words read as no parent, not its {codes} codes"
    );
    for k in params.end..WORDS {
        let mut extra = words;
        extra[k] = 1.0;
        assert!(
            words[k].to_bits() == 0 && curved_of(&extra).parent::<C>().is_none(),
            "{what}: its word {k}, past its params, is not +0 or is read"
        );
    }
}

/// The frozen controls of the mixed charts below.
const FROZEN: [f64; 8] = [0.31, -0.52, 0.13, -0.24, 0.45, -0.06, 0.67, -0.38];

fn invariant_b() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.21, -0.43],
        z_mass: [0.65, -0.17],
        k_max: 1.9,
        gamma_k: 1.25,
    }
}

fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: [0.2, 0.7],
        k: 1.3,
        lz_fraction: 0.6,
    }
}

/// Every parent type `Saved::curved` admits, each through `check_parent` with `alter`.
fn check_every_parent(alter: fn(&mut [f64; WORDS])) {
    check_parent(
        ShapeSphere {
            map: ShapeMap::Exponential,
            n0: [0.6, 0.0, 0.8],
            e1: [0.0, 1.0, -0.25],
            e2: [-0.8, 0.125, 0.6],
            scale: 0.9,
            inertia: 1.1,
            phase: 0.3,
            z_mass: [0.5, -0.3],
            z_mom: [0.2, -0.1, 0.15, 0.05],
        },
        1,
        "the shape sphere",
        alter,
    );
    check_parent(InvariantLE(invariant_b()), 0, "(L_z, E)", alter);
    check_parent(InvariantLK(invariant_b()), 0, "(L_z, K)", alter);
    check_parent(
        Mapped::<_, Defaults>::new(
            InvariantLE(invariant_b()),
            [Mapping::Linear, Mapping::Warped],
        ),
        2,
        "(L_z, E) mapped",
        alter,
    );
    check_parent(
        Mapped::<_, Defaults>::new(
            InvariantLK(invariant_b()),
            [Mapping::Warped, Mapping::Linear],
        ),
        2,
        "(L_z, K) mapped",
        alter,
    );
    check_parent(theta_lz(), 0, "Φ_θ,L_z", alter);
    check_parent(
        HeldK {
            chart: theta_lz(),
            k_max: 2.5,
            gamma_k: 1.7,
            mapping: Mapping::Linear,
        },
        1,
        "Φ_θ,L_z with its held K",
        alter,
    );
    check_parent(
        HeldLz {
            chart: Burrau {
                nu_range: [0.1, 0.9],
                k_max: 1.2,
                gamma_k: 1.4,
            },
            lz: 0.3,
            mapping: Mapping::Linear,
        },
        1,
        "(ν, K) with its held L_z",
        alter,
    );
    check_parent(
        HeldLz {
            chart: BurrauThetaK {
                theta_range: [0.15, 0.65],
                k_max: 1.6,
                gamma_k: 2.1,
            },
            lz: -0.4,
            mapping: Mapping::Warped,
        },
        1,
        "Φ_θ,K with its held L_z",
        alter,
    );
    let z_mom = [0.11, -0.22, 0.33, -0.44];
    check_parent(
        RestStart {
            chart: MassSimplex {
                nu0: 0.4,
                eps_m: 0.05,
            },
            z_mom,
        },
        0,
        "the mass simplex at rest",
        alter,
    );
    check_parent(
        RestStart {
            chart: EuclidPlane {
                m_range: [0.5, 2.0],
                nu_range: [0.2, 0.8],
            },
            z_mom,
        },
        0,
        "the Euclid plane at rest",
        alter,
    );
    check_parent(
        RestStart {
            chart: BurrauThetaDeltaM {
                theta_range: [0.3, 0.75],
                m_target: [0.2, 0.3, 0.5],
            },
            z_mom,
        },
        0,
        "Φ_θ,Δm at rest",
        alter,
    );
    check_parent(
        Mixed::<Alpha, ZQ1, f64>::new(FROZEN, [0.1, 1.2], [-0.7, 0.9]),
        2,
        "a mixed chart",
        alter,
    );
    check_parent(
        MixedEnergy::<Beta, AxisE, Defaults, f64>::new(
            FROZEN,
            [0.4, 2.2],
            1.8,
            1.3,
            0.35,
            Mapping::Linear,
        ),
        3,
        "a mixed energy chart",
        alter,
    );
    check_parent(
        MixedEnergy::<ZAlpha, AxisVirial, Defaults, f64>::virial(
            FROZEN,
            [-0.4, 0.6],
            [-0.5, 2.0],
            1.5,
            0.2,
            Mapping::Linear,
        )
        .expect("an increasing range"),
        3,
        "a mixed virial-ratio chart",
        alter,
    );
    check_parent(
        MixedMassRatio::<ZBeta, Defaults, f64>::new(
            FROZEN,
            [0.25, 2.75],
            [0.5, 2.0],
            [0, 2],
            Mapping::Linear,
            &LinkParams::DEFAULT,
        )
        .expect("two bodies, an increasing range"),
        4,
        "a mixed mass-ratio chart",
        alter,
    );
}

/// A mass-ratio chart's bodies read back up to the last body, `2`, and refuse a body past it, `3`.
fn check_bodies(last: f64) {
    let chart = MixedMassRatio::<ZBeta, Defaults, f64>::new(
        FROZEN,
        [0.25, 2.75],
        [0.5, 2.0],
        [0, 2],
        Mapping::Linear,
        &LinkParams::DEFAULT,
    )
    .expect("two bodies, an increasing range");
    let words = words_of(&chart);
    let body = HEAD + 1 + 1 + 8 + 2 + 2 + 1;
    assert!(
        words[body].to_bits() == 2.0f64.to_bits(),
        "the second body's word is not where the layout puts it"
    );
    let mut past = words;
    past[body] = last + 1.0;
    let mut at = words;
    at[body] = last;
    assert!(
        curved_of(&at)
            .parent::<MixedMassRatio<ZBeta, Defaults, f64>>()
            .is_some()
            && curved_of(&past)
                .parent::<MixedMassRatio<ZBeta, Defaults, f64>>()
                .is_none(),
        "a body past the last reads as a mass-ratio chart, or the last does not"
    );
}

#[test]
fn saved_chart_parent_round_trip() {
    check_every_parent(|_| {});
    check_bodies(2.0);
}

negative_control!(
    saved_chart_parent_round_trip,
    "words that drop the parent's params read back as no parent or another, so it must fail",
    expected = "does not read back as its parent",
    {
        check_every_parent(|w| w[HEAD + 1..].fill(0.0));
    }
);

mod bodies {
    use super::*;
    negative_control!(
        saved_chart_parent_round_trip,
        "a body checked past the last, 3, is refused, so it must fail",
        expected = "a body past the last",
        {
            check_bodies(3.0);
        }
    );
}

/// Two charts `name()` cannot tell apart save to different words, and each one's words read as no chart of the
/// other's type; `masked` compares the words with the form and the first two params, where the codes are, masked
/// (the control).
fn check_pair<A, B>(a: &A, b: &B, what: &str, masked: bool)
where
    A: SaveParent + ParentParams,
    B: SaveParent + ParentParams,
{
    assert_eq!(a.name(), b.name(), "{what}: the premise, one name");
    let mask = |mut w: [f64; WORDS]| {
        if masked {
            w[HEAD..HEAD + 3].fill(0.0);
        }
        w.map(f64::to_bits)
    };
    assert!(
        mask(words_of(a)) != mask(words_of(b)),
        "{what} give the same words"
    );
    assert!(
        curved_of(&words_of(a)).parent::<B>().is_none()
            && curved_of(&words_of(b)).parent::<A>().is_none(),
        "{what}: one's words read as the other"
    );
}

fn check_variants(masked: bool) {
    let mixed = Mixed::<ZAlpha, ZBeta, f64>::new(FROZEN, [0.1, 1.2], [-0.7, 0.9]);
    check_pair(
        &mixed,
        &Mixed::<Alpha, ZBeta, f64>::new(FROZEN, [0.1, 1.2], [-0.7, 0.9]),
        "a latent axis and the derived axis on its coordinate",
        masked,
    );
    check_pair(
        &mixed,
        &Mixed::<ZBeta, ZAlpha, f64>::new(FROZEN, [0.1, 1.2], [-0.7, 0.9]),
        "the axes swapped",
        masked,
    );
    let energy = |gamma_k| {
        MixedEnergy::<ZAlpha, AxisK, Defaults, f64>::new(
            FROZEN,
            [0.1, 1.2],
            1.8,
            gamma_k,
            0.35,
            Mapping::Warped,
        )
    };
    check_pair(
        &energy(1.3),
        &MixedEnergy::<ZAlpha, AxisKE, Defaults, f64>::new(
            FROZEN,
            [0.1, 1.2],
            1.8,
            1.3,
            0.35,
            Mapping::Warped,
        ),
        "K and KE",
        masked,
    );
    check_pair(
        &mixed,
        &energy(1.3),
        "a mixed chart and a mixed energy chart",
        masked,
    );
    check_pair(
        &energy(0.0),
        &MixedMassRatio::<ZAlpha, Defaults, f64>::new(
            FROZEN,
            [0.1, 1.2],
            [0.5, 2.0],
            [0, 1],
            Mapping::Linear,
            &LinkParams::DEFAULT,
        )
        .expect("two bodies, an increasing range"),
        "a mixed energy chart and a mixed mass-ratio chart",
        masked,
    );
    check_pair(
        &InvariantLE(invariant_b()),
        &Mapped::<_, Defaults>::new(InvariantLE(invariant_b()), [Mapping::Warped; 2]),
        "(L_z, E) and its warped mapped chart",
        masked,
    );
    check_pair(
        &theta_lz(),
        &HeldK {
            chart: theta_lz(),
            k_max: 0.0,
            gamma_k: 0.0,
            mapping: Mapping::Warped,
        },
        "Φ_θ,L_z and its held K at zero",
        masked,
    );
    assert!(
        curved_of(&words_of(&InvariantLE(invariant_b())))
            .parent::<InvariantLK<f64>>()
            .is_none(),
        "(L_z, E)'s words read as (L_z, K), its layout but another parent"
    );
}

#[test]
fn saved_chart_parent_variants() {
    check_variants(false);
}

negative_control!(
    saved_chart_parent_variants,
    "words without the form and the axis codes cannot tell the variants apart, so it must fail",
    expected = "give the same words",
    {
        check_variants(true);
    }
);
