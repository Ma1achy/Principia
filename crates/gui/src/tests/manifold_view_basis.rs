//! `manifold_view_basis` (REQ-GUI-415; R-705): with `X ≠ Y`, a typed basis with `Y` on `q₁` or `X` on `q₂` reads in its
//! dominant axes' frame, the preset's test counting those components, `σ` read whole; and a zoom edit scales the stored
//! basis, so the plane stays where it is. A view the map gives reads as it did at every `γ`, a mirrored basis whose swapped
//! order reads it back being one of them, and so does a basis with each target on its own vector as typed: a component
//! counts only on the wrong vector as typed, on a reading that does not give the basis back, whichever way round the
//! axes lie. Each check is also run on an input it must reject, through [`rejects`] (gui has no route to
//! `negative_control!`, `support`'s note).

use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Plane, SliceFrame, LATENT_DIM};

use super::support::{click_scope, headless, mock_app, rejects};
use crate::explore::manifold_view::navigate::zoom_to;
use crate::explore::manifold_view::{
    self as mv, axis, basis_edit, dot, frame_edit, Out, Scratch, View, PRESETS, ZOOM,
};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, StepKind};

const ALPHA: usize = 0;
const BETA: usize = 1;
const Q0: usize = 2;
const Q3: usize = 5;
const MU1: usize = 6;
const MU2: usize = 7;

/// The frame of the defects: `X = z_q3`, `Y = z_μ1`.
const SPLIT: SliceFrame = SliceFrame {
    normal: Q3 as u32,
    tilt_x: Q3 as u32,
    tilt_y: MU1 as u32,
};

/// The two bases REQ-GUI-415 names, `Y` on `q₁` both: the first reads `σ = 0` with the fault, and the second is the one
/// a rebuilt basis moves off its plane.
fn typed() -> [(Latent, Latent); 2] {
    [
        (axis(MU1, 0.8), axis(BETA, 1.0)),
        (
            mv::add(&axis(ALPHA, 0.6), 0.8, &axis(MU1, 1.0)),
            axis(BETA, 1.0),
        ),
    ]
}

fn plane((q1, q2): (Latent, Latent)) -> Plane {
    Plane {
        z0: [0.0; LATENT_DIM],
        q1,
        q2,
    }
}

fn read(basis: (Latent, Latent), frame: SliceFrame) -> View {
    View::framed(&plane(basis), Some(&frame))
}

fn check_near(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() < 1e-12, "{what}: {got}, want {want}");
}

/// Asserts `view` is the frame of the axes `(a, b)`, no preset's, at the scale `sigma`, its zoom finite.
fn check_dominant(view: &View, (a, b): (usize, usize), sigma: f64) {
    assert_eq!((view.preset, view.a, view.b), (None, a, b), "{view:?}");
    assert!(view.zoom().is_finite(), "{view:?} reads an infinite zoom");
    check_near(view.scale, sigma, "σ");
}

#[test]
fn manifold_view_basis_y_on_q1_reads_a_finite_zoom_in_that_frame() {
    let [only, _] = typed();
    let view = read(only, SPLIT);
    check_dominant(&view, (MU1, BETA), 0.8);
    assert_eq!((view.tau1, view.tau2, view.gamma), (0.0, 0.0, 0.0));
    rejects("the preset's reading, Y ignored, σ = 0", || {
        check_dominant(
            &View {
                preset: Some(0),
                a: ALPHA,
                b: BETA,
                scale: 0.0,
                ..view
            },
            (MU1, BETA),
            0.8,
        )
    });
}

#[test]
fn manifold_view_basis_q1_reads_its_sigma_whole() {
    let [_, split] = typed();
    check_dominant(&read(split, SPLIT), (MU1, BETA), 1.0);
    // With one target the component is off the frame just the same, and reads as much of σ.
    check_dominant(&read(split, SliceFrame::along(Q3 as u32)), (MU1, BETA), 1.0);
    // A component on neither axes nor target, of a basis no reading on axes takes.
    let q1 = mv::add(&axis(Q0, 0.8), 0.6, &axis(MU2, 1.0));
    check_dominant(&read((q1, axis(BETA, 1.0)), SPLIT), (Q0, BETA), 1.0);
    rejects("σ read from the axes alone", || {
        check_dominant(
            &View {
                scale: 0.8,
                ..read(split, SPLIT)
            },
            (MU1, BETA),
            1.0,
        )
    });
}

#[test]
fn manifold_view_basis_x_on_q2_is_counted_as_y_on_q1_is() {
    let q2 = mv::add(&axis(BETA, 0.6), 0.8, &axis(Q3, 1.0));
    let view = read((axis(ALPHA, 1.0), q2), SPLIT);
    check_dominant(&view, (ALPHA, Q3), 1.0);
    rejects("the preset's reading, X on q₂ ignored", || {
        check_dominant(
            &View {
                preset: Some(0),
                b: BETA,
                ..view
            },
            (ALPHA, Q3),
            1.0,
        )
    });
}

#[test]
fn manifold_view_basis_a_target_on_its_own_vector_stays_a_tilt() {
    let [_, split] = typed();
    // The same basis with `X = z_μ1`: a tilt of the first preset, not a component on the wrong vector.
    let own = SliceFrame {
        tilt_x: MU1 as u32,
        tilt_y: Q3 as u32,
        ..SPLIT
    };
    let view = read(split, own);
    assert_eq!((view.preset, view.a, view.b), (Some(0), ALPHA, BETA));
    check_near(view.scale, 1.0, "σ");
    check_near(view.tau1, 0.8f64.atan2(0.6), "τ₁");
    // One target: no wrong vector, so the tilt on `q₁` and the tilt on `q₂` are both read.
    let (q1, q2) = (
        axis(ALPHA, 1.0),
        mv::add(&axis(BETA, 0.6), 0.8, &axis(Q3, 1.0)),
    );
    let view = read((q1, q2), SliceFrame::along(Q3 as u32));
    assert_eq!((view.preset, view.a, view.b), (Some(0), ALPHA, BETA));
    check_near(view.tau2, 0.8f64.atan2(0.6), "τ₂");
}

/// Asserts `basis` reads as the first preset in [`SPLIT`].
fn check_stays_preset(basis: (Latent, Latent)) {
    assert_eq!(read(basis, SPLIT).preset, Some(0), "{basis:?}");
}

#[test]
fn manifold_view_basis_only_past_the_noise_is_a_component_counted() {
    // The total is 2, so the cut on a component's square is 2·10⁻¹⁸, a component of about 1.41·10⁻⁹.
    for delta in [1e-10, 5e-10] {
        check_stays_preset((
            mv::add(&axis(ALPHA, 1.0), delta, &axis(MU1, 1.0)),
            axis(BETA, 1.0),
        ));
        check_stays_preset((
            axis(ALPHA, 1.0),
            mv::add(&axis(BETA, 1.0), delta, &axis(Q3, 1.0)),
        ));
    }
    let past = (
        mv::add(&axis(ALPHA, 1.0), 2e-9, &axis(MU1, 1.0)),
        axis(BETA, 1.0),
    );
    assert_eq!(read(past, SPLIT).preset, None);
    rejects("a component past the noise", || check_stays_preset(past));
    // The zero basis has nothing to count: `0 > 0` is no component, and it reads the first preset.
    check_stays_preset(([0.0; LATENT_DIM], [0.0; LATENT_DIM]));
}

#[test]
fn manifold_view_basis_a_small_component_is_noise_on_a_reading_that_gives_nothing_back() {
    // Each target on its own vector as typed, sheared so that no reading gives it back: a preset's reading.
    let (q1, q2) = (
        mv::add(&axis(Q0, 0.6), 0.8, &axis(Q3, 1.0)),
        mv::add(&axis(Q0, 0.8), 0.6, &axis(MU1, 1.0)),
    );
    let sheared = |on_q1: f64, on_q2: f64| {
        let view = read(
            (
                mv::add(&q1, on_q1, &axis(MU1, 1.0)),
                mv::add(&q2, on_q2, &axis(Q3, 1.0)),
            ),
            SPLIT,
        );
        (view.preset, view.a)
    };
    assert_eq!(sheared(0.0, 0.0), (Some(1), Q0));
    // A total of about 2: the cut on the component's square is 2·10⁻¹⁸, and these are under it, once each way.
    assert_eq!(sheared(5e-10, 0.0), (Some(1), Q0));
    assert_eq!(sheared(0.0, 5e-10), (Some(1), Q0));
    let past = sheared(2e-9, 0.0);
    assert_ne!(past, (Some(1), Q0));
    rejects("past the noise as typed", || {
        assert_eq!(past, (Some(1), Q0))
    });
}

#[test]
fn manifold_view_basis_a_mirrored_basis_reads_back_or_reads_its_dominant_axes() {
    let at = |alpha: f64| {
        read(
            (
                mv::add(&axis(MU1, 0.8), alpha, &axis(ALPHA, 1.0)),
                axis(BETA, 1.0),
            ),
            SPLIT,
        )
    };
    // `Y` on `q₁` at a negative determinant, reading back exactly on the other order: a view the map gives, `Y` on its
    // own vector there, so it keeps that reading. From 0 up there is none.
    let exact = at(-0.6);
    assert_eq!((exact.a, exact.b), (BETA, ALPHA));
    check_near(exact.scale, 1.0, "σ");
    check_near(exact.tau2, -(0.8f64.atan2(0.6)), "τ₂");
    check_dominant(&at(0.0), (MU1, BETA), 0.8);
    check_dominant(&at(0.6), (MU1, BETA), 1.0);
    rejects("the exact reading as a typed one", || {
        check_dominant(&exact, (MU1, BETA), 1.0)
    });
    // The same at a sign that reads back in neither order: `Y` still on the wrong vector, and the dominant axes.
    let short = read(
        (
            mv::add(&axis(ALPHA, -0.3), 0.4, &axis(MU1, 1.0)),
            axis(BETA, 1.0),
        ),
        SPLIT,
    );
    check_dominant(&short, (MU1, BETA), 0.5);
    rejects("the reading as typed, Y left out", || {
        check_dominant(
            &View {
                a: ALPHA,
                b: BETA,
                ..short
            },
            (MU1, BETA),
            0.5,
        )
    });
    // And past a swap that would keep more of the tilt: the exact basis with `X` beside it no longer reads back.
    let q1 = mv::add(&axis(MU1, 0.8), -0.6, &axis(ALPHA, 1.0));
    let more = read((mv::add(&q1, 0.3, &axis(Q3, 1.0)), axis(BETA, 1.0)), SPLIT);
    check_dominant(&more, (MU1, BETA), 1.09f64.sqrt());
    rejects("the swap, X read by nothing", || {
        check_dominant(
            &View {
                a: BETA,
                b: ALPHA,
                ..more
            },
            (MU1, BETA),
            1.09f64.sqrt(),
        )
    });
}

#[test]
fn manifold_view_basis_a_tilted_pair_no_preset_names_is_counted_too() {
    // The axes `(z_q0, z_μ2)`, tilted within the targets as a pair H and V picked reads, with `Y` on `q₁`.
    let q1 = mv::add(&axis(Q0, 0.6), 0.8, &axis(MU1, 1.0));
    let view = read((q1, axis(MU2, 1.0)), SPLIT);
    check_dominant(&view, (MU1, MU2), 1.0);
    // The same axes with `X` there are that pair's tilt.
    let q1 = mv::add(&axis(Q0, 0.6), 0.8, &axis(Q3, 1.0));
    let view = read((q1, axis(MU2, 1.0)), SPLIT);
    assert_eq!((view.preset, view.a, view.b), (None, Q0, MU2));
    check_near(view.tau1, 0.8f64.atan2(0.6), "τ₁");
    check_near(view.scale, 1.0, "σ");
}

/// The latent dimensions no frame here takes.
const OFF_TARGETS: [usize; 6] = [ALPHA, BETA, Q0, 3, 4, MU2];

#[test]
fn manifold_view_basis_every_pair_either_way_round_is_decided_by_the_basis() {
    let reads = |q1: Latent, q2: Latent| read((q1, q2), SPLIT);
    let mut pairs = 0;
    for p in OFF_TARGETS {
        for r in OFF_TARGETS.into_iter().filter(|&r| r != p) {
            for sign in [1.0, -1.0] {
                // `Y` on `q₁`: the sort lands on either determinant, and no reading gives this one back.
                let q1 = mv::add(&axis(p, sign * 0.3), 0.4, &axis(MU1, 1.0));
                check_dominant(&reads(q1, axis(r, 1.0)), (MU1, r), 0.5);
                // `X` on `q₂`.
                let q2 = mv::add(&axis(r, sign * 0.3), 0.4, &axis(Q3, 1.0));
                check_dominant(&reads(axis(p, 1.0), q2), (p, Q3), 1.0);
                pairs += 1;
            }
        }
    }
    assert_eq!(pairs, 60);
    // The positive determinant in its typed order that the sort makes negative.
    let q1 = mv::add(&axis(BETA, 0.6), 0.8, &axis(MU1, 1.0));
    let view = reads(q1, axis(ALPHA, 1.0));
    check_dominant(&view, (MU1, ALPHA), 1.0);
    rejects(
        "the sorted pair, Y on its own vector turned by −90°",
        || {
            check_dominant(
                &View {
                    a: ALPHA,
                    b: BETA,
                    ..view
                },
                (MU1, ALPHA),
                1.0,
            )
        },
    );
    // A view the map gives with the axes that way round, `Y` on `q₁` as typed at γ = −90°: it reads back, so it stays.
    let q1 = mv::add(&axis(BETA, -0.6), 0.8, &axis(MU1, 1.0));
    let exact = reads(q1, axis(ALPHA, 1.0));
    assert_eq!((exact.preset, exact.a, exact.b), (Some(0), ALPHA, BETA));
    check_near(exact.gamma, -std::f64::consts::FRAC_PI_2, "γ");
    // Half its length, it gives nothing back, and reads its dominant axes.
    let half = reads(q1.map(|c| c / 2.0), axis(ALPHA, 1.0));
    check_dominant(&half, (MU1, ALPHA), 0.5);
}

#[test]
fn manifold_view_basis_a_mirrored_tie_stays_as_typed() {
    // No tilt for either order to keep, and no reading back: the order as typed, though the dominant axes are the swap's.
    let q1 = mv::add(&axis(Q0, 0.6), 0.8, &axis(MU2, 1.0));
    let q2 = mv::add(&axis(Q0, 1.0), 0.2, &axis(MU2, 1.0));
    let view = read((q1, q2), SPLIT);
    assert_eq!((view.preset, view.a, view.b), (None, Q0, MU2));
    rejects("the tie read on its dominant axes", || {
        assert_eq!((view.preset, view.b, view.a), (None, Q0, MU2))
    });
}

#[test]
fn manifold_view_basis_a_pair_with_no_turn_is_no_tilted_pair() {
    // `q₂` is twice `q₁` on a pair no preset names: a determinant of 0, which reads by its dominant axes, not as the pair.
    let q1 = mv::add(&axis(Q0, 0.5), 1.0, &axis(MU2, 1.0));
    let view = read((q1, q1.map(|c| c * 2.0)), SPLIT);
    assert_eq!((view.preset, view.a, view.b), (None, MU2, Q0));
    rejects("the pair read in its own order", || {
        assert_eq!((view.preset, view.b, view.a), (None, MU2, Q0))
    });
}

/// Asserts `v` reads back from its own basis, angles and scale, with its frame.
fn check_reads_back(v: View) {
    let back = View::framed(&plane(v.basis()), Some(&v.frame()));
    assert_eq!(
        (back.a, back.b, back.x, back.y),
        (v.a, v.b, v.x, v.y),
        "{v:?}"
    );
    assert_eq!(back.preset, v.preset, "{v:?}");
    check_near(back.scale, v.scale, "σ");
    check_near(back.tau1, v.tau1, "τ₁");
    check_near(back.tau2, v.tau2, "τ₂");
    check_near(back.gamma, v.gamma, "γ");
}

#[test]
fn manifold_view_basis_a_turned_view_keeps_its_reading() {
    // `γ` puts `Y` on `q₁` and `X` on `q₂` on every view the map gives: it is the turned-back vectors that count.
    let angles = [-1.2, -0.4, 0.3, 1.1];
    let mut turned = None;
    for (&(a, b), preset) in PRESETS.iter().take(2).zip(0..) {
        for tau1 in angles {
            for tau2 in angles {
                for gamma in angles {
                    let v = View {
                        preset: Some(preset),
                        a,
                        b,
                        d: MU2,
                        x: MU2,
                        y: MU1,
                        scale: 0.75,
                        tau1,
                        tau2,
                        gamma,
                    };
                    check_reads_back(v);
                    turned = Some(v);
                }
            }
        }
    }
    let v = turned.expect("views");
    // On a pair no preset names, as H and V pick them.
    check_reads_back(View {
        preset: None,
        a: Q0,
        b: MU2,
        d: Q3,
        x: Q3,
        y: MU1,
        ..v
    });
    rejects("a view that reads another's angles", || {
        check_near(v.gamma, v.gamma + 0.1, "γ")
    });
}

#[test]
fn manifold_view_basis_equal_axes_read_gamma_on_the_first() {
    // Both axes hold 1: the first's column gives `γ = 0`, the second's would give 90°.
    let view = read(
        (
            mv::add(&axis(ALPHA, 1.0), 1.0, &axis(BETA, 1.0)),
            [0.0; LATENT_DIM],
        ),
        SPLIT,
    );
    assert_eq!(view.gamma, 0.0);
}

/// The basis `edit` sets.
fn edited(edit: &SetField) -> (Latent, Latent) {
    match &edit.edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => (*q1, *q2),
        other => panic!("not a basis edit: {other:?}"),
    }
}

/// How far `v` is off the span of `q1`, `q2`, as a share of its length.
fn off_span(v: &Latent, (q1, q2): (&Latent, &Latent)) -> f64 {
    let (g11, g12, g22) = (dot(q1, q1), dot(q1, q2), dot(q2, q2));
    let det = g11 * g22 - g12 * g12;
    let (c1, c2) = (dot(v, q1), dot(v, q2));
    let (x, y) = ((g22 * c1 - g12 * c2) / det, (g11 * c2 - g12 * c1) / det);
    let rest = mv::add(&mv::add(v, -x, q1), -y, q2);
    dot(&rest, &rest).sqrt() / dot(v, v).sqrt()
}

/// Asserts the basis `after` spans the plane of `before`, each in the other's span.
fn check_same_plane(before: (Latent, Latent), after: (Latent, Latent)) {
    let (old, new) = ((&before.0, &before.1), (&after.0, &after.1));
    for off in [
        off_span(&after.0, old),
        off_span(&after.1, old),
        off_span(&before.0, new),
        off_span(&before.1, new),
    ] {
        assert!(
            off < 1e-9,
            "the plane moved, {off} off: {before:?} → {after:?}"
        );
    }
}

/// `n` typed bases and frames, seeded: each component in `[−1, 1]`, some on no axis at all, and `X ≠ Y` on most.
fn fuzzed(n: usize) -> Vec<((Latent, Latent), SliceFrame)> {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    (0..n)
        .map(|_| {
            let component = |seed: u64| {
                if seed.is_multiple_of(3) {
                    0.0
                } else {
                    seed as f64 / u64::MAX as f64 * 2.0 - 1.0
                }
            };
            let q1 = std::array::from_fn(|_| component(next()));
            let q2 = std::array::from_fn(|_| component(next()));
            let target = |seed: u64| (seed % LATENT_DIM as u64) as u32;
            let frame = SliceFrame {
                normal: target(next()),
                tilt_x: target(next()),
                tilt_y: target(next()),
            };
            ((q1, q2), frame)
        })
        .collect()
}

/// The sound bases of [`fuzzed`]: those spanning a plane.
fn fuzzed_planes(n: usize) -> Vec<((Latent, Latent), SliceFrame)> {
    fuzzed(n)
        .into_iter()
        .filter(|((q1, q2), _)| mv::plane_s(q1, q2) > 1e-3)
        .collect()
}

#[test]
fn manifold_view_basis_a_zoom_edit_keeps_the_plane() {
    let cases: Vec<_> = typed()
        .into_iter()
        .map(|basis| (basis, SPLIT))
        .chain(fuzzed_planes(400))
        .collect();
    assert!(cases.len() > 300, "the fuzz's bases: {}", cases.len());
    for (basis, frame) in cases {
        let view = read(basis, frame);
        for zoom in [-3.0, 0.5, 20.0] {
            let after = edited(&zoom_to(&plane(basis), view, view.with_zoom(zoom)));
            check_same_plane(basis, after);
            let scaled = read(after, frame);
            assert_eq!((scaled.a, scaled.b), (view.a, view.b), "{basis:?}");
            check_near(scaled.zoom(), zoom, "the zoom asked for");
        }
    }
    let basis = typed()[1];
    let view = read(basis, SPLIT);
    rejects("a zoom that rebuilds the basis", || {
        check_same_plane(basis, view.with_zoom(1.0).basis())
    });
}

/// Asserts every component of `basis` is finite.
fn check_finite((q1, q2): &(Latent, Latent)) {
    assert!(q1.iter().chain(q2).all(|c| c.is_finite()), "{q1:?} {q2:?}");
}

#[test]
fn manifold_view_basis_a_zoom_with_no_scale_to_scale_takes_the_views_basis() {
    let edits = |basis: (Latent, Latent)| {
        let view = View::of(&plane(basis));
        let to = view.with_zoom(1.0);
        (edited(&zoom_to(&plane(basis), view, to)), to.basis())
    };
    // `σ = 0`: no basis, and a basis with no plane, `q₁` a multiple of `q₂` on `e_b`. Scaled, each is NaN.
    for basis in [
        ([0.0; LATENT_DIM], [0.0; LATENT_DIM]),
        (axis(BETA, 0.5), axis(BETA, 1.0)),
    ] {
        let (got, want) = edits(basis);
        assert_eq!(got, want);
        check_finite(&got);
    }
    // `σ` past `f64`: the factor is 0, and scaled the basis would be no basis.
    let huge = mv::add(&axis(ALPHA, f64::MAX), f64::MAX, &axis(MU1, 1.0));
    assert_eq!(
        View::of(&plane((huge, axis(BETA, 1.0)))).scale,
        f64::INFINITY
    );
    let (got, want) = edits((huge, axis(BETA, 1.0)));
    assert_eq!(got, want);
    rejects("the zero basis scaled by the infinite factor", || {
        let zero = [0.0; LATENT_DIM].map(|c: f64| c * f64::INFINITY);
        check_finite(&(zero, zero))
    });
}

/// The app on the mock at the typed `basis`, in [`SPLIT`], with the panel drawn.
fn typed_app(
    basis: (Latent, Latent),
) -> (
    crate::app::App<crate::side::MockSide>,
    crate::headless::Headless,
) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(frame_edit(SPLIT, Some(basis)));
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

#[test]
fn manifold_view_basis_the_zoom_row_keeps_the_plane() {
    for basis in typed() {
        let (mut app, mut h) = typed_app(basis);
        // A click on the slider's middle is a zoom far from this one's.
        click_scope(&mut h, &mut app, ZOOM);
        let plane = app.snapshot().sim.plane.clone();
        assert_ne!((plane.q1, plane.q2), basis, "the slider took the click");
        check_same_plane(basis, (plane.q1, plane.q2));
    }
    rejects("the plane a rebuilt basis gives", || {
        let basis = typed()[1];
        check_same_plane(basis, read(basis, SPLIT).with_zoom(1.0).basis())
    });
}

/// Asserts `edits` is one edit, `basis` at half its size: an octave in.
fn check_halved(edits: &[SetField], basis: (Latent, Latent)) {
    let halved = (basis.0.map(|c| c / 2.0), basis.1.map(|c| c / 2.0));
    assert_eq!(edits, [basis_edit(halved)]);
}

#[test]
fn manifold_view_basis_the_keyboards_zoom_keeps_the_plane() {
    for basis in typed() {
        let (app, _) = typed_app(basis);
        let (mut scratch, mut out) = (Scratch::default(), Out::default());
        let adjust = Adjust {
            scope: ZOOM,
            kind: StepKind::ZoomLog2,
            times: 4.0,
            direction: Direction::Up,
        };
        mv::adjust(&adjust, app.snapshot(), &mut scratch, &mut out);
        check_halved(&out.edits, basis);
    }
    rejects("the keyboard's zoom rebuilding the basis", || {
        let basis = typed()[1];
        let view = read(basis, SPLIT);
        check_halved(
            &[basis_edit(view.with_zoom(view.zoom() + 1.0).basis())],
            basis,
        )
    });
}
