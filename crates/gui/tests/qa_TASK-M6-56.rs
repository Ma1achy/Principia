//! QA tests for TASK-M6-56, written from REQ-GUI-415's statement and verify, R-705 and RQ-663's trace, not from the
//! implementation:
//!
//! - With `X ≠ Y`, a typed basis with `Y` on `q₁` or `X` on `q₂` is no tilt of a preset: it reads in its dominant axes'
//!   frame, at a finite zoom, its `σ` whole. The verify's two bases are checked by the numbers: `q₁ = 0.8 e_μ1`,
//!   `q₂ = e_β` is `σ = 0.8` on `(z_μ1, z_β)`, and `q₁ = 0.6 e_α + 0.8 e_μ1`, `q₂ = e_β` is `σ = 1`.
//! - The same holds whichever axes the basis sits on: every ordered pair, every pair of distinct targets, either wrong
//!   vector. A basis a view gives exactly is that view, R-457 E1's map: a `γ` turn puts `Y` on `q₁` lawfully.
//! - Nothing that reads a plane reads an infinite zoom: an exhaustive grid of typed bases.
//! - A zoom edit keeps the plane: `q₁` and `q₂` scaled by one factor, `z₀` untouched, `q₁ ∧ q₂` unchanged as a direction
//!   (compared by Plücker coordinates, not by a projection), on the verify's bases and on fuzzed typed bases, through the
//!   real slider and the real keyboard step.
//! - The panel drawn with no input edits nothing, a zoom past the slider's range included.
//! - A mirrored basis no view gives counts a wrong vector as any other does. With each target on its own vector
//!   REQ-GUI-415 is silent on the axes, and this file asserts none.
//!
//! gui has no route to `negative_control!` (R-187), so each check runs its control (R-176) through [`rejects`]: the
//! verify's two controls are [`outside_ignored`], the reading with `outside` leaving `X` and `Y` out, built here from
//! RQ-663's trace, and `View::with_zoom(z).basis()`, the zoom that rebuilds the basis.
// The file name `qa_TASK-M6-56` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Plane, SliceFrame, LATENT_DIM};
use gui::app::App;
use gui::capture::{click, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::manifold_view::{
    self as mv, frame_edit, Out, Scratch, View, PRESETS, ZOOM, ZOOM_RANGE,
};
use gui::headless::Headless;
use gui::keyboard::keymap::Direction;
use gui::keyboard::scopes::{Adjust, StepKind};
use gui::mock::MockEngine;
use gui::side::MockSide;

const ALPHA: usize = 0;
const BETA: usize = 1;
const Q0: usize = 2;
const Q1: usize = 3;
const Q3: usize = 5;
const MU1: usize = 6;
const MU2: usize = 7;

/// `X = z_q3`, `Y = z_μ1`: RQ-663's frame.
const SPLIT: SliceFrame = SliceFrame {
    normal: Q3 as u32,
    tilt_x: Q3 as u32,
    tilt_y: MU1 as u32,
};

/// One target, `X = Y = z_q3`: there is no wrong vector.
const SHARED: SliceFrame = SliceFrame {
    normal: Q3 as u32,
    tilt_x: Q3 as u32,
    tilt_y: Q3 as u32,
};

type Basis = (Latent, Latent);

fn e(i: usize, k: f64) -> Latent {
    let mut v = [0.0; LATENT_DIM];
    v[i] = k;
    v
}

fn sum(terms: &[(usize, f64)]) -> Latent {
    let mut v = [0.0; LATENT_DIM];
    for &(i, k) in terms {
        v[i] += k;
    }
    v
}

fn plane((q1, q2): Basis) -> Plane {
    Plane {
        z0: [0.0; LATENT_DIM],
        q1,
        q2,
    }
}

fn read(basis: Basis, frame: SliceFrame) -> View {
    View::framed(&plane(basis), Some(&frame))
}

fn check_near(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() < 1e-12, "{what}: {got}, want {want}");
}

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// The reading RQ-663 traces on `main`, `Y` on `q₁` ignored by the preset's test: the preset `(z_α, z_β)`, `σ` the part
/// on `e_a` and `e_X` alone. The verify's first control.
fn outside_ignored((q1, q2): &Basis, frame: SliceFrame) -> View {
    let (a, b, x) = (ALPHA, BETA, frame.tilt_x as usize);
    View {
        preset: Some(0),
        a,
        b,
        d: frame.normal as usize,
        x,
        y: frame.tilt_y as usize,
        scale: q1[a].hypot(q2[a]).hypot(q1[x]),
        tau1: 0.0,
        tau2: 0.0,
        gamma: 0.0,
    }
}

/// The largest axis of `v`, the first of equals, skipping `not`.
fn largest(v: &Latent, not: Option<usize>) -> usize {
    let mut best: Option<usize> = None;
    for i in (0..LATENT_DIM).filter(|&i| Some(i) != not) {
        if best.is_none_or(|b| v[i].abs() > v[b].abs()) {
            best = Some(i);
        }
    }
    best.expect("eight axes")
}

/// The dominant axes `(a, b)` of a typed basis, from the words: `q₁`'s largest, `q₂`'s largest of the rest.
fn dominant_axes((q1, q2): &Basis) -> (usize, usize) {
    let a = largest(q1, None);
    (a, largest(q2, Some(a)))
}

/// Asserts `view` is the frame of `axes`, no preset's, at scale `sigma`, its zoom finite.
fn check_frame(view: &View, axes: (usize, usize), sigma: f64) {
    assert_eq!((view.a, view.b), axes, "{view:?}");
    assert_eq!(view.preset, None, "{view:?}");
    assert!(view.zoom().is_finite(), "an infinite zoom: {view:?}");
    check_near(view.scale, sigma, "σ");
}

#[test]
fn manifold_view_basis_the_verifys_first_basis_reads_a_finite_zoom_in_its_frame() {
    let basis = (e(MU1, 0.8), e(BETA, 1.0));
    check_frame(&read(basis, SPLIT), (MU1, BETA), 0.8);
    check_near(read(basis, SPLIT).zoom(), -(0.8f64.log2()), "the zoom");
    rejects("the reading with Y left out of the test", || {
        check_frame(&outside_ignored(&basis, SPLIT), (MU1, BETA), 0.8)
    });
    assert_eq!(
        outside_ignored(&basis, SPLIT).zoom(),
        f64::INFINITY,
        "RQ-663's σ = 0"
    );
}

#[test]
fn manifold_view_basis_the_verifys_second_basis_reads_its_sigma_whole() {
    let basis = (sum(&[(ALPHA, 0.6), (MU1, 0.8)]), e(BETA, 1.0));
    check_frame(&read(basis, SPLIT), (MU1, BETA), 1.0);
    rejects("the reading off by 0.8, RQ-663's σ = 0.6", || {
        let wrong = outside_ignored(&basis, SPLIT);
        check_near(wrong.scale, 0.6, "the control's σ");
        check_frame(&wrong, (ALPHA, BETA), 1.0)
    });
}

#[test]
fn manifold_view_basis_a_small_wrong_component_is_counted_on_the_presets_own_axes() {
    // The dominant axes are the preset's, and the basis is still no tilt of it, so it reads as no preset, `σ` whole.
    let y_on_q1 = (sum(&[(ALPHA, 0.8), (MU1, 0.6)]), e(BETA, 1.0));
    check_frame(&read(y_on_q1, SPLIT), (ALPHA, BETA), 1.0);
    let x_on_q2 = (e(ALPHA, 1.0), sum(&[(BETA, 0.8), (Q3, 0.6)]));
    check_frame(&read(x_on_q2, SPLIT), (ALPHA, BETA), 1.0);
    rejects("the same basis with the target on its own vector", || {
        let own = (sum(&[(ALPHA, 0.8), (Q3, 0.6)]), e(BETA, 1.0));
        check_frame(&read(own, SPLIT), (ALPHA, BETA), 1.0)
    });
}

#[test]
fn manifold_view_basis_x_on_q2_reads_as_y_on_q1_does() {
    let basis = (e(ALPHA, 1.0), sum(&[(BETA, 0.6), (Q3, 0.8)]));
    check_frame(&read(basis, SPLIT), (ALPHA, Q3), 1.0);
    rejects("the preset's reading of it", || {
        check_frame(&outside_ignored(&basis, SPLIT), (ALPHA, Q3), 1.0)
    });
}

#[test]
fn manifold_view_basis_a_target_on_its_own_vector_is_still_a_tilt() {
    // `X` on `q₁` and `Y` on `q₂` is what the map gives: R-457 E1. The presets must not lose it.
    let basis = (
        sum(&[(ALPHA, 0.6), (Q3, 0.8)]),
        sum(&[(BETA, 0.8), (MU1, 0.6)]),
    );
    let view = read(basis, SPLIT);
    assert_eq!((view.preset, view.a, view.b), (Some(0), ALPHA, BETA));
    check_near(view.tau1, 0.8f64.atan2(0.6), "τ₁");
    check_near(view.tau2, 0.6f64.atan2(0.8), "τ₂");
    check_near(view.scale, 1.0, "σ");
    // With one target there is no wrong vector: the target on either vector is that tilt.
    let shared = read((sum(&[(ALPHA, 0.6), (Q3, 0.8)]), e(BETA, 1.0)), SHARED);
    assert_eq!((shared.preset, shared.a, shared.b), (Some(0), ALPHA, BETA));
    rejects("a tilt read as a typed basis", || {
        check_frame(&view, (ALPHA, BETA), 1.0)
    });
}

/// Whether a view reads `basis` back, the map's image: such a basis is that view, whatever it holds on which vector.
fn reads_back(view: &View, (q1, q2): &Basis) -> bool {
    let (p1, p2) = view.basis();
    p1.iter()
        .zip(q1)
        .chain(p2.iter().zip(q2))
        .all(|(p, q)| (p - q).abs() < 1e-12)
}

/// The failures of the rule on every ordered axis pair `(a, b)` off every pair of distinct targets, with `(c, s)` on
/// the typed vector: `q₁ = c e_a + s e_Y`, `q₂ = e_b`, or `q₁ = e_a`, `q₂ = c e_b + s e_X`.
fn wrong_vector_failures(read: &dyn Fn(Basis, SliceFrame) -> View) -> Vec<String> {
    let mut failures = Vec::new();
    for (x, y) in (0..LATENT_DIM)
        .flat_map(|x| (0..LATENT_DIM).map(move |y| (x, y)))
        .filter(|(x, y)| x != y)
    {
        let frame = SliceFrame {
            normal: x as u32,
            tilt_x: x as u32,
            tilt_y: y as u32,
        };
        let off = |i: &usize| *i != x && *i != y;
        for (a, b) in (0..LATENT_DIM)
            .filter(off)
            .flat_map(|a| (0..LATENT_DIM).filter(off).map(move |b| (a, b)))
            .filter(|(a, b)| a != b)
        {
            for (c, s) in [
                (0.6, 0.8),
                (0.8, 0.6),
                (-0.6, 0.8),
                (0.6, -0.8),
                (-0.8, 0.6),
                (0.5, 0.8),
                (-0.5, 0.8),
                (0.9, -0.3),
            ] {
                for basis in [
                    (sum(&[(a, c), (y, s)]), e(b, 1.0)),
                    (e(a, 1.0), sum(&[(b, c), (x, s)])),
                ] {
                    let view = read(basis, frame);
                    let whole = mv::dot(&basis.0, &basis.0).sqrt();
                    let scale_ok = (view.scale - whole).abs() < 1e-12;
                    let exact = reads_back(&view, &basis);
                    let frame_ok =
                        (view.a, view.b) == dominant_axes(&basis) && view.preset.is_none();
                    if !scale_ok || !(frame_ok || exact) {
                        failures.push(format!(
                            "X {x} Y {y} on {a} {b} at {c} {s}: read ({}, {}) preset {:?} σ {} γ {}, dominant {:?}",
                            view.a,
                            view.b,
                            view.preset,
                            view.scale,
                            view.gamma,
                            dominant_axes(&basis)
                        ));
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn manifold_view_basis_a_wrong_vector_reads_dominant_on_every_axis_pair() {
    let failures = wrong_vector_failures(&read);
    assert!(
        failures.is_empty(),
        "{} of them, first {:#?}",
        failures.len(),
        &failures[..failures.len().min(6)]
    );
    rejects("the reading with Y left out of the test", || {
        let failures = wrong_vector_failures(&|b, f| outside_ignored(&b, f));
        assert!(failures.is_empty());
    });
}

#[test]
fn manifold_view_basis_the_order_of_the_axes_changes_nothing() {
    // The verify's second basis with `q₂`'s axis below `q₁`'s, as it stands above: nothing of R-705 turns on which axis
    // has the lower index. `q₂ = e_α` is the verify's own shape, a reading no view gives.
    let basis = (sum(&[(BETA, 0.6), (MU1, 0.8)]), e(ALPHA, 1.0));
    assert!(!reads_back(&read(basis, SPLIT), &basis), "no view gives it");
    check_frame(&read(basis, SPLIT), (MU1, ALPHA), 1.0);
    rejects("a preset's axes for that basis", || {
        check_frame(&read(basis, SPLIT), (ALPHA, BETA), 1.0)
    });
}

#[test]
fn manifold_view_basis_a_mirrored_basis_no_view_gives_counts_its_wrong_vector_too() {
    // `Y` on `q₁`, `q₁`'s `e_α` negative: at the length of a view this is one, and anywhere else it is no view.
    let at = |alpha: f64| (sum(&[(ALPHA, alpha), (MU1, 0.8)]), e(BETA, 1.0));
    assert!(reads_back(&read(at(-0.6), SPLIT), &at(-0.6)));
    for alpha in [-0.75, -0.5, -0.1] {
        let basis = at(alpha);
        check_frame(&read(basis, SPLIT), (MU1, BETA), alpha.hypot(0.8));
    }
    // Both targets on the wrong vectors, on a pair turned so that the swap holds more of them than the order as typed:
    // the swap names no dominant axes, and R-705 counts these components whichever reading would keep more.
    let both = (
        sum(&[(ALPHA, 1.0), (BETA, 0.5), (MU1, 0.8)]),
        sum(&[(ALPHA, 2.0), (BETA, 0.5), (Q3, 0.3)]),
    );
    let view = read(both, SPLIT);
    assert!(!reads_back(&view, &both), "no view gives it");
    assert_eq!(
        (view.preset, view.a, view.b),
        (None, ALPHA, BETA),
        "{view:?}"
    );
    assert!(view.zoom().is_finite(), "{view:?}");
    rejects("the swapped pair", || {
        assert_eq!((view.a, view.b), (BETA, ALPHA), "{view:?}")
    });
}

#[test]
fn manifold_view_basis_a_mirrored_basis_with_no_wrong_vector_is_outside_the_requirement() {
    // REQ-GUI-415 names a target on the wrong vector, and here each is on its own: which axes this reads is not its
    // business, so no axes are asserted. What it does state still holds of any basis: a finite zoom, and a zoom edit
    // that keeps the plane.
    let basis = (sum(&[(ALPHA, -0.6), (Q3, 0.8)]), e(BETA, 1.0));
    let view = read(basis, SPLIT);
    assert!(!reads_back(&view, &basis), "no view gives it");
    check_near(view.scale, 1.0, "σ");
    assert!(view.zoom().is_finite(), "{view:?}");
    let mut rig = Rig::new();
    rig.set(basis, SPLIT);
    rig.slide(0.5);
    let after = rig.basis();
    assert_ne!(after, basis, "the slider took no click");
    check_scaled(&basis, &after);
    rejects("the rebuild of a basis no view gives", || {
        check_scaled(&basis, &view.with_zoom(3.0).basis());
    });
}

#[test]
fn manifold_view_basis_a_view_of_the_map_reads_back_at_every_angle() {
    // R-457 E1's map, turned: a `γ` puts `Y` on `q₁` and `X` on `q₂` lawfully, and the reading must not count those.
    let turns = [-1.2, -0.4, 0.0, 0.3, 1.1];
    let tilts = [-1.0, -0.2, 0.0, 0.5, 1.3];
    let mut views = 0;
    for &(a, b) in PRESETS
        .iter()
        .filter(|&&(a, b)| ![Q3, MU1].contains(&a) && ![Q3, MU1].contains(&b))
    {
        for tau1 in tilts {
            for tau2 in tilts {
                for gamma in turns {
                    let view = View {
                        preset: PRESETS.iter().position(|&p| p == (a, b)),
                        a,
                        b,
                        d: Q3,
                        x: Q3,
                        y: MU1,
                        scale: 0.7,
                        tau1,
                        tau2,
                        gamma,
                    };
                    let again = read(view.basis(), SPLIT);
                    assert_eq!(
                        (again.preset, again.a, again.b),
                        (view.preset, a, b),
                        "{view:?}"
                    );
                    check_near(again.scale, 0.7, "σ");
                    check_near(again.tau1, tau1, "τ₁");
                    check_near(again.tau2, tau2, "τ₂");
                    check_near(again.gamma, gamma, "γ");
                    views += 1;
                }
            }
        }
    }
    assert_eq!(views, 2 * 125, "the sweep");
    rejects("a γ's Y on q₁ read as typed", || {
        assert!(outside_ignored(&(e(MU1, 0.8), e(BETA, 1.0)), SPLIT)
            .zoom()
            .is_finite())
    });
}

/// Every basis on `(z_α, z_β, z_q3, z_μ1)` with components in `{0, ±½, ±1}`.
fn grid() -> impl Iterator<Item = Basis> {
    const VALUES: [f64; 5] = [0.0, 0.5, -0.5, 1.0, -1.0];
    const AXES: [usize; 4] = [ALPHA, BETA, Q3, MU1];
    (0..5usize.pow(8)).map(|mut code| {
        let (mut q1, mut q2) = ([0.0; LATENT_DIM], [0.0; LATENT_DIM]);
        for q in [&mut q1, &mut q2] {
            for &axis in &AXES {
                q[axis] = VALUES[code % 5];
                code /= 5;
            }
        }
        (q1, q2)
    })
}

#[test]
fn manifold_view_basis_no_typed_plane_reads_an_infinite_zoom() {
    let mut planes = 0;
    for frame in [SPLIT, SHARED] {
        for basis in grid().filter(|(q1, q2)| mv::plane_s(q1, q2) > mv::PLANE_S_CUT) {
            let zoom = read(basis, frame).zoom();
            assert!(zoom.is_finite(), "zoom {zoom} on {basis:?} in {frame:?}");
            planes += 1;
        }
    }
    assert!(planes > 300_000, "the grid's planes: {planes}");
    rejects("RQ-663's reading", || {
        let basis = (e(MU1, 0.8), e(BETA, 1.0));
        assert!(outside_ignored(&basis, SPLIT).zoom().is_finite());
    });
}

/// `q₁ ∧ q₂`, unit length, its sign that of its largest coordinate: the same for two bases of one plane, as an
/// unoriented direction.
fn plucker((q1, q2): &Basis) -> Vec<f64> {
    let mut p = Vec::new();
    for i in 0..LATENT_DIM {
        for j in i + 1..LATENT_DIM {
            p.push(q1[i] * q2[j] - q1[j] * q2[i]);
        }
    }
    let length = p.iter().map(|c| c * c).sum::<f64>().sqrt();
    let peak = p
        .iter()
        .fold(0.0_f64, |m, c| if c.abs() > m.abs() { *c } else { m });
    p.iter().map(|c| c / length * peak.signum()).collect()
}

/// Asserts `after` is `before` scaled by one positive factor, and returns it: so the same plane, by Plücker coordinates
/// too, and the same oriented shape on it.
fn check_scaled(before: &Basis, after: &Basis) -> f64 {
    for (old, new) in plucker(before).iter().zip(plucker(after)) {
        assert!(
            (old - new).abs() < 1e-9,
            "the plane moved: {before:?} → {after:?}"
        );
    }
    let k = (mv::dot(&after.0, &after.0) + mv::dot(&after.1, &after.1)).sqrt()
        / (mv::dot(&before.0, &before.0) + mv::dot(&before.1, &before.1)).sqrt();
    let parts = before
        .0
        .iter()
        .zip(&after.0)
        .chain(before.1.iter().zip(&after.1));
    for (old, new) in parts {
        assert!(
            (old * k - new).abs() <= 1e-12 * k.max(new.abs()),
            "not one factor {k}: {before:?} → {after:?}"
        );
    }
    k
}

/// `n` typed bases with frames, seeded: each component 0 a third of the time, or in `[−1, 1]`; bases with no plane out.
fn fuzzed(n: usize) -> Vec<(Basis, SliceFrame)> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x9e37_79b9_7f4a_7c15)
    };
    let mut out = Vec::new();
    while out.len() < n {
        let mut vector = || -> Latent {
            std::array::from_fn(|_| {
                let r = next();
                if r % 3 == 0 {
                    0.0
                } else {
                    (r >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
                }
            })
        };
        let basis = (vector(), vector());
        let frame = SliceFrame {
            normal: (next() >> 32) as u32 % LATENT_DIM as u32,
            tilt_x: (next() >> 32) as u32 % LATENT_DIM as u32,
            tilt_y: (next() >> 32) as u32 % LATENT_DIM as u32,
        };
        if mv::plane_s(&basis.0, &basis.1) > 1e-3 {
            out.push((basis, frame));
        }
    }
    out
}

/// The verify's two bases, then `q₂`'s `X` as the third.
fn typed() -> [Basis; 3] {
    [
        (e(MU1, 0.8), e(BETA, 1.0)),
        (sum(&[(ALPHA, 0.6), (MU1, 0.8)]), e(BETA, 1.0)),
        (e(ALPHA, 1.0), sum(&[(BETA, 0.6), (Q3, 0.8)])),
    ]
}

struct Rig {
    app: App<MockSide>,
    headless: Headless,
}

impl Rig {
    fn new() -> Self {
        let mut rig = Self {
            app: App::new(MockSide::new(MockEngine::new(), None), FORMAT),
            headless: Headless::new(SIZE, PIXELS_PER_POINT),
        };
        rig.idle();
        rig
    }

    fn idle(&mut self) {
        for _ in 0..2 {
            let _ = self.headless.frame(&mut self.app, Vec::new());
        }
    }

    fn set(&mut self, basis: Basis, frame: SliceFrame) {
        self.app.set_field(frame_edit(frame, Some(basis)));
        self.idle();
    }

    fn plane(&self) -> Plane {
        self.app.snapshot().sim.plane.clone()
    }

    fn basis(&self) -> Basis {
        let plane = self.plane();
        (plane.q1, plane.q2)
    }

    /// A click on the zoom slider at `share` of its width: the real row.
    fn slide(&mut self, share: f32) {
        let rect = self.app.keyboard.place_of(ZOOM).expect("the zoom row").rect;
        let at = rect.lerp_inside(eframe::egui::vec2(share, 0.5));
        for events in click(at) {
            let _ = self.headless.frame(&mut self.app, events);
        }
        self.idle();
    }
}

#[test]
fn manifold_view_basis_the_zoom_slider_keeps_the_plane() {
    let fuzz = fuzzed(24);
    let cases = typed()
        .into_iter()
        .map(|basis| (basis, SPLIT))
        .chain(fuzz.iter().copied());
    let mut rig = Rig::new();
    let z0 = rig.plane().z0;
    let mut scaled = 0;
    for (case, (basis, frame)) in cases.enumerate() {
        rig.set(basis, frame);
        let before = read(basis, frame);
        rig.slide(0.1 + 0.03 * case as f32);
        let after = rig.basis();
        assert_ne!(after, basis, "the slider took no click");
        let k = check_scaled(&basis, &after);
        assert_eq!(rig.plane().z0, z0, "z₀ moved on a zoom");
        let again = read(after, frame);
        assert_eq!(
            (again.preset, again.a, again.b),
            (before.preset, before.a, before.b),
            "the reading moved on {basis:?}"
        );
        assert!(
            (again.scale / before.scale - k).abs() < 1e-9 * k,
            "σ {} → {} on a factor {k}",
            before.scale,
            again.scale
        );
        scaled += 1;
    }
    assert_eq!(scaled, 3 + 24);
    rejects("the zoom that rebuilds the basis", || {
        let basis = typed()[1];
        let rebuilt = read(basis, SPLIT).with_zoom(3.0).basis();
        check_scaled(&basis, &rebuilt);
    });
}

#[test]
fn manifold_view_basis_the_keyboards_zoom_keeps_the_plane() {
    let mut rig = Rig::new();
    for (basis, frame) in typed().into_iter().map(|b| (b, SPLIT)).chain(fuzzed(24)) {
        rig.set(basis, frame);
        let (mut scratch, mut out) = (Scratch::default(), Out::default());
        let adjust = Adjust {
            scope: ZOOM,
            kind: StepKind::ZoomLog2,
            times: 1.0,
            direction: Direction::Up,
        };
        mv::adjust(&adjust, rig.app.snapshot(), &mut scratch, &mut out);
        let [SetField {
            edit: Edit::Sim(SimField::Basis { q1, q2 }),
            ..
        }] = out.edits[..]
        else {
            panic!("no single basis edit: {:?}", out.edits);
        };
        let k = check_scaled(&basis, &(q1, q2));
        check_near(k, (-0.25f64).exp2(), "a quarter octave in");
    }
    rejects("the zoom that rebuilds the basis, on the arrows", || {
        let basis = typed()[0];
        let view = read(basis, SPLIT);
        check_scaled(&basis, &view.with_zoom(view.zoom() + 0.25).basis());
    });
}

#[test]
fn manifold_view_basis_a_zoom_keeps_the_reading_at_either_end_of_the_range() {
    // The scale is the only thing an edit names: every other part of the reading stays, down to the range's ends.
    for (basis, frame) in fuzzed(300) {
        let view = read(basis, frame);
        for zoom in [ZOOM_RANGE.0, ZOOM_RANGE.1] {
            let k = (view.zoom() - zoom).exp2();
            let again = read((basis.0.map(|c| c * k), basis.1.map(|c| c * k)), frame);
            assert_eq!(
                (again.preset, again.a, again.b, again.x, again.y),
                (view.preset, view.a, view.b, view.x, view.y),
                "{basis:?} in {frame:?}"
            );
            check_near(again.zoom(), zoom, "the zoom asked for");
            check_near(again.tau1, view.tau1, "τ₁");
            check_near(again.tau2, view.tau2, "τ₂");
            check_near(again.gamma, view.gamma, "γ");
        }
    }
    rejects("a reading off by a tilt", || {
        let view = read(typed()[0], SPLIT);
        check_near(view.tau1 + 1.0, view.tau1, "τ₁");
    });
}

#[test]
fn manifold_view_basis_a_panel_left_alone_edits_nothing() {
    // A basis past the slider's range must not be pulled onto it by being drawn: nothing was edited.
    let mut rig = Rig::new();
    for k in [64.0, 1.0, 1e-16] {
        let basis = (sum(&[(ALPHA, 0.6 * k), (MU1, 0.8 * k)]), e(BETA, k));
        rig.set(basis, SPLIT);
        rig.idle();
        assert_eq!(rig.basis(), basis, "the panel drawn edited the basis");
    }
    rejects("a basis moved", || {
        let mut rig = Rig::new();
        let basis = typed()[1];
        rig.set(basis, SPLIT);
        rig.slide(0.5);
        assert_eq!(rig.basis(), basis);
    });
}

#[test]
fn manifold_view_basis_the_other_preset_frames_count_too() {
    // The defect was the preset's test, not the first preset: `(z_q0, z_q1)` is the other preset this frame fits.
    let basis = (sum(&[(Q0, 0.6), (MU1, 0.8)]), e(Q1, 1.0));
    check_frame(&read(basis, SPLIT), (MU1, Q1), 1.0);
    let basis = (e(Q0, 1.0), sum(&[(Q1, 0.6), (Q3, 0.8)]));
    check_frame(&read(basis, SPLIT), (Q0, Q3), 1.0);
    let other = SliceFrame {
        normal: MU2 as u32,
        tilt_x: MU2 as u32,
        tilt_y: Q3 as u32,
    };
    let basis = (sum(&[(ALPHA, 0.6), (Q3, 0.8)]), e(BETA, 1.0));
    check_frame(&read(basis, other), (Q3, BETA), 1.0);
    rejects("that basis in a frame where it is a tilt", || {
        check_frame(&read(basis, SPLIT), (Q3, BETA), 1.0)
    });
}
