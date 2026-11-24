//! Locking does not move the view (chart_decoder_contract Part 4, "Setting the lock" and "The lock's
//! construction, defined"): the lock pins the clicked pixel and edits the lock group alone; tilt and zoom turn about the
//! pin; snap-back undoes the slice only; centre on pin brings the pin to the centre.
//!
//! **The criteria**, written before the first run:
//! - **The lock moves no view.** A lock at an off-centre pixel on the latent chart (three planes), a mixed chart and two
//!   invariant charts leaves `z₀`, the basis and the window bit for bit; on the latent chart `z_locked` is the kernel's
//!   `Φ` at that pixel bit for bit (`engine::lock::point`); on a nonlinear chart the pinned pixel, read through the
//!   unchanged window, decodes the clicked state bit for bit.
//! - **Tilt and zoom turn about the pin.** After each zoom and raw tilt in a chain, on the latent chart, the kernel's `Φ`
//!   at the pin is `z_locked` bit for bit in every component where some `z₀` gives it; a component where none does
//!   (checked by trying every float within 64 ulps of the re-solved `z₀`: the centre's ulp coarser than the pinned
//!   value's, so `z₀ + a·q₁ + b·q₂` steps over it) is within one ulp of the largest of `|z₀|`, `|a·q₁|`, `|b·q₂|` and
//!   `|z_locked|` there, the bound proposed to the owner; the count of such components is printed. On the nonlinear
//!   charts, after each window zoom the pinned
//!   pixel decodes the clicked state bit for bit, and a curved tilt turned about the pin ([`engine::path::View::pivoted`])
//!   decodes it there bit for bit. Where a free view is tilted, the pivot is the kernel's own tilt about the centre, bit
//!   for bit, at every probe pixel.
//! - **Snap-back** after a slice under an off-centre lock on an oblique basis: the pin's address unmoved; the kernel's
//!   `Φ` at the pin is `z_locked` bit for bit wherever a `z₀` gives it, else as the tilt's criterion above; the hidden part of the centre's excursion, `Hᵀ(z₀ − z_locked)` (`H` the
//!   plane's orthonormal hidden directions), within `8·ε·M`, `M = ‖|z_locked| + |a|·|q₁| + |b|·|q₂|‖` the size of the
//!   numbers the re-solve rounds (`a = 2s − 1`, `b = 2t − 1`), the construction's bound (a few ulps per component, `H`
//!   orthonormal so `κ(B)` does not multiply it); the fixture prints its `κ(B)` and `‖δ‖`. Snap-back then centre on pin
//!   is `z₀ = z_locked` bit for bit.
//!
//! **Controls**: a lock that re-centres (the centre-on-pin setting on) fails the first; a tilt about the centre with an
//! off-centre pin (the latent tilt without its re-solve; the curved tilt about the window's centre) fails the second.

mod path_shim;
mod path_support;

use engine::contract::sim_config::{Latent, Plane, Window};
use engine::lock::point;
use engine::lookup::Tolerances;
use engine::nav::directions::hidden;
use engine::nav::BasisVector;
use engine::path::{Constructor, Pin, Segment, View};
use kernel::chart::tilt::{Tilt, TiltMap, TiltSpec, Tiltable, Tilted};
use kernel::chart::ChartDecode;
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::generated::links::Defaults;
use path_shim::sim_of;
use path_support::*;
use validation::negative_control;
use validation::roundtrip::phys;

/// What runs the lock and the tilt: the object, or a control's stand-in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Variant {
    /// The object.
    Object,
    /// The lock with the centre-on-pin setting on: it re-centres.
    #[cfg(feature = "controls")]
    Recentres,
    /// Zooms about the window's centre with the pin off it.
    #[cfg(feature = "controls")]
    ZoomAboutCentre,
    /// Raw tilts about the centre with the pin off it.
    #[cfg(feature = "controls")]
    TiltAboutCentre,
    /// Curved tilts about the window's centre with the pin off it.
    #[cfg(feature = "controls")]
    CurvedAboutCentre,
}

/// Which kind of edit a chain's step is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    Zoom,
    Tilt,
}

/// The off-centre pins of the latent checks.
const PINS: [(f64, f64); 3] = [(0.3, 0.8), (0.71, 0.13), (-0.2, 0.55)];

/// The off-centre pins of the nonlinear checks, inside the square.
const CHART_PINS: [(f64, f64); 2] = [(0.3, 0.8), (0.62, 0.21)];

/// `ic`'s words.
fn words(ic: &Ic<f64>) -> Vec<u8> {
    let mut out = Vec::new();
    ic_words(ic, &mut out);
    out
}

/// The lock of `a` at `(s, t)`, by the variant.
fn lock(a: &View, (s, t): (f64, f64), variant: Variant) -> View {
    let seg = match variant {
        #[cfg(feature = "controls")]
        Variant::Recentres => Segment::lock_centred(a, s, t),
        _ => Segment::lock(a, s, t),
    };
    seg.expect("a lock").b().clone()
}

/// The lock of the kernel view `a` on `chart` at `(s, t)`, by the variant, and the clicked state.
fn chart_lock<C>(a: &View, chart: &C, at: (f64, f64), variant: Variant) -> (View, Ic<f64>)
where
    C: ByKind + engine::lock::FrozenControls,
    f64: From<C::Param>,
{
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let (seg, found) = match variant {
        #[cfg(feature = "controls")]
        Variant::Recentres => {
            Segment::lock_on_chart_centred::<Defaults, C, _>(a, chart, at, phys, &tol)
        }
        _ => Segment::lock_on_chart::<Defaults, C, _>(a, chart, at, phys, &tol),
    }
    .expect("a lock");
    (seg.b().clone(), found.ic)
}

/// The view after `edit`, a step of kind `step`, by the variant: where the variant turns that kind about the centre,
/// the pin is stripped before the edit (and put back after it).
fn tilted(a: &View, variant: Variant, step: Step, edit: impl Fn(&View) -> Segment) -> View {
    let about_centre = match (variant, step) {
        #[cfg(feature = "controls")]
        (Variant::ZoomAboutCentre, Step::Zoom) | (Variant::TiltAboutCentre, Step::Tilt) => true,
        _ => false,
    };
    let from = match about_centre {
        true => View {
            pin: None,
            ..a.clone()
        },
        false => a.clone(),
    };
    View {
        pin: a.pin,
        ..edit(&from).b().clone()
    }
}

/// Bit for bit, component by component: how many of `got`'s components are `want`'s.
fn exact(got: &Latent, want: &Latent) -> usize {
    got.iter()
        .zip(want)
        .filter(|(g, w)| g.to_bits() == w.to_bits())
        .count()
}

/// How many components of `Φ(pin)` on `plane` are `want` bit for bit; panics with "the pinned pixel moved" on a
/// component that is not, where a float within 64 ulps of `z₀`'s reaches `want`, or that is further than one ulp of the
/// largest number `Φ` adds there.
fn held_or_unreachable(plane: &Plane, (s, t): (f64, f64), want: &Latent, case: &str) -> usize {
    let got = point(plane, s, t);
    let (a, b) = (s + s - 1.0, t + t - 1.0);
    let mut n = 0;
    for i in 0..8 {
        if got[i].to_bits() == want[i].to_bits() {
            n += 1;
            continue;
        }
        let reach = |z: f64| {
            let mut z0 = plane.z0;
            z0[i] = z;
            point(
                &Plane {
                    z0,
                    ..plane.clone()
                },
                s,
                t,
            )[i]
                .to_bits()
                == want[i].to_bits()
        };
        let mut z = plane.z0[i];
        let mut zs = vec![z];
        for _ in 0..64 {
            z = z.next_up();
            zs.push(z);
        }
        z = plane.z0[i];
        for _ in 0..64 {
            z = z.next_down();
            zs.push(z);
        }
        assert!(
            !zs.into_iter().any(reach),
            "the pinned pixel moved: {case}, component {i} reachable but missed"
        );
        let largest = [plane.z0[i], a * plane.q1[i], b * plane.q2[i], want[i]]
            .iter()
            .fold(0.0f64, |m, x| m.max(x.abs()));
        let ulp = largest.next_up() - largest;
        let off = (got[i] - want[i]).abs();
        eprintln!(
            "  unreachable: {case}, component {i}: off by {off:e} = {:.3} ulp of {largest:e}",
            off / ulp
        );
        assert!(
            off <= ulp,
            "the pinned pixel moved: {case}, component {i} off by {off:e} > {ulp:e}"
        );
    }
    n
}

/// How many of the window's two placements of the pin are `want` bit for bit; panics with "the pinned pixel moved"
/// as [`held_or_unreachable`] does.
fn window_held_or_unreachable(
    window: &Window,
    (s, t): (f64, f64),
    want: &[f64; 2],
    case: &str,
) -> usize {
    let got = window.at(s, t);
    let off = [s + s - 1.0, t + t - 1.0];
    let mut n = 0;
    for i in 0..2 {
        if got[i].to_bits() == want[i].to_bits() {
            n += 1;
            continue;
        }
        let reach = |c: f64| {
            let mut centre = window.centre;
            centre[i] = c;
            Window { centre, ..*window }.at(s, t)[i].to_bits() == want[i].to_bits()
        };
        let (mut up, mut down) = (window.centre[i], window.centre[i]);
        for _ in 0..64 {
            (up, down) = (up.next_up(), down.next_down());
            assert!(
                !reach(up) && !reach(down),
                "the pinned pixel moved: {case}, axis {i} reachable but missed"
            );
        }
        let largest = [window.centre[i], off[i] * window.half[i], want[i]]
            .iter()
            .fold(0.0f64, |m, x| m.max(x.abs()));
        let ulp = largest.next_up() - largest;
        let miss = (got[i] - want[i]).abs();
        eprintln!(
            "  unreachable: {case}, axis {i}: off by {miss:e} = {:.3} ulp of {largest:e}",
            miss / ulp
        );
        assert!(
            miss <= ulp,
            "the pinned pixel moved: {case}, axis {i} off by {miss:e} > {ulp:e}"
        );
    }
    n
}

/// The largest relative difference between two states' words, each against the larger of the pair's magnitudes.
fn relative(a: &Ic<f64>, b: &Ic<f64>) -> f64 {
    let flat = |ic: &Ic<f64>| -> Vec<f64> {
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten())
            .copied()
            .collect()
    };
    flat(a)
        .iter()
        .zip(flat(b))
        .map(|(x, y)| match x == &y {
            true => 0.0,
            false => (x - y).abs() / x.abs().max(y.abs()),
        })
        .fold(0.0, f64::max)
}

/// The lock on the latent chart, at each pin on each plane: the view still, `z_locked` the kernel's `Φ` there; then a
/// chain of zooms and raw tilts about the pin, `Φ` at the pin `z_locked` after each.
fn check_latent(variant: Variant) {
    for (pname, plane) in planes() {
        let dirs = hidden(&plane).expect("hidden");
        for at in PINS {
            let a = View::latent(sim(plane.clone()));
            let b = lock(&a, at, variant);
            assert_eq!(
                (b.sim.plane.clone(), b.sim.chart.window),
                (a.sim.plane.clone(), a.sim.chart.window),
                "the lock moved the view: {pname} at {at:?}"
            );
            let z_locked = b.sim.lock.z_locked;
            assert_eq!(
                exact(&z_locked, &point(&plane, at.0, at.1)),
                8,
                "z_locked is not the kernel's Φ at the pixel: {pname} at {at:?}"
            );
            assert_eq!(
                b.pin.map(|p| p.address()),
                Some([at.0, at.1]),
                "the pin is not the clicked pixel"
            );
            let constructor = Constructor::new(plane.clone());
            type Edit<'a> = &'a dyn Fn(&View) -> Segment;
            let steps: [(Step, Edit); 5] = [
                (Step::Tilt, &|v| {
                    Segment::raw_tilt(
                        &constructor,
                        v,
                        BasisVector::Q1,
                        (0.0, 30.0),
                        &dirs[0].direction,
                    )
                    .expect("a tilt")
                }),
                (Step::Zoom, &|v| Segment::zoom(v, 1.5).expect("a zoom")),
                (Step::Zoom, &|v| Segment::zoom(v, -0.7).expect("a zoom")),
                (Step::Tilt, &|v| {
                    Segment::raw_tilt(
                        &constructor,
                        v,
                        BasisVector::Q2,
                        (0.0, -41.5),
                        &dirs[3].direction,
                    )
                    .expect("a tilt")
                }),
                (Step::Zoom, &|v| Segment::zoom(v, 3.25).expect("a zoom")),
            ];
            let mut view = b;
            for (i, (kind, step)) in steps.iter().enumerate() {
                view = tilted(&view, variant, *kind, step);
                let n = held_or_unreachable(
                    &view.sim.plane,
                    at,
                    &z_locked,
                    &format!("{pname} at {at:?}, step {i}"),
                );
                eprintln!(
                    "latent {pname} pin {at:?} step {i}: {n}/8 components of Φ(pin) bit for bit"
                );
            }
        }
    }
}

/// A free view of the kernel map `chart` through `window`.
fn chart_view<C: kernel::chart::Chart>(chart: &C, window: Window) -> View {
    let mut s = sim(Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    });
    s.chart.window = window;
    View::kernel(chart, s).expect("a registry chart")
}

/// The curved tilts of the nonlinear checks, into the chart's hidden coordinates `x` and `y`.
fn specs(x: u32, y: u32) -> [TiltSpec; 3] {
    [
        TiltSpec {
            tau1: 25.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x,
            y: x,
        },
        TiltSpec {
            tau1: 25.0,
            tau2: -40.0,
            map: TiltMap::Sheet,
            x,
            y: x,
        },
        TiltSpec {
            tau1: -15.0,
            tau2: 35.0,
            map: TiltMap::Lean,
            x,
            y,
        },
    ]
}

/// The targets' slider spans of the nonlinear checks.
const SPAN: [f64; 2] = [2.0, 2.0];

/// The lock on a nonlinear chart, at each pin: the view still, the pinned pixel the clicked state; then window zooms
/// about the pin, the pinned pixel the clicked state after each, and curved tilts about the pin, the pinned pixel the
/// clicked state under each.
fn check_chart<C>(name: &str, chart: &C, [x, y]: [u32; 2], variant: Variant)
where
    C: ByKind + engine::lock::FrozenControls + ChartDecode + Tiltable<Param = f64> + Copy,
    f64: From<<C as kernel::chart::Chart>::Param>,
{
    for at in CHART_PINS {
        let a = chart_view(chart, window([0.45, 0.55], 0.25));
        let (b, clicked) = chart_lock(&a, chart, at, variant);
        assert_eq!(
            (b.sim.plane.clone(), b.sim.chart.window),
            (a.sim.plane.clone(), a.sim.chart.window),
            "the lock moved the view: {name} at {at:?}"
        );
        let pinned = |v: &View| {
            let [u, w] = v.sim.chart.window.at(at.0, at.1);
            chart.decode_at::<Defaults, f64>(u, w).ic
        };
        assert_eq!(
            words(&pinned(&b)),
            words(&clicked),
            "the pinned pixel is not the clicked state"
        );
        let clicked_at = b.sim.chart.window.at(at.0, at.1);
        let mut view = b;
        for (i, octaves) in [1.0, -0.5, 2.25].iter().enumerate() {
            view = tilted(&view, variant, Step::Zoom, |v| {
                Segment::zoom(v, *octaves).expect("a zoom")
            });
            let case = format!("{name} at {at:?}, zoom {i}");
            let n = window_held_or_unreachable(&view.sim.chart.window, at, &clicked_at, &case);
            let same = words(&pinned(&view)) == words(&clicked);
            eprintln!(
                "{name} pin {at:?} zoom {i}: {n}/2 of the chart point bit for bit; pinned pixel's state bit for bit \
                 {same}, its largest relative difference {:.3e}",
                relative(&pinned(&view), &clicked)
            );
            assert!(
                n < 2 || same,
                "the pinned pixel moved: {case}, its chart point exact and its state not"
            );
        }
        for (i, spec) in specs(x, y).iter().enumerate() {
            let [u, w] = view.sim.chart.window.at(at.0, at.1);
            let got = match variant {
                #[cfg(feature = "controls")]
                Variant::CurvedAboutCentre => {
                    let t = Tilt::new(spec, view.sim.chart.window.centre, SPAN);
                    Tilted::new(*chart, t)
                        .expect("a tilt")
                        .decode_at::<Defaults, f64>(u, w)
                        .ic
                }
                _ => {
                    let p = view.pivoted(spec, SPAN).expect("a pivot");
                    Tilted::placed(*chart, p)
                        .expect("a tilt")
                        .decode_at::<Defaults, f64>(u, w)
                        .ic
                }
            };
            let same = words(&got) == words(&clicked);
            eprintln!("{name} pin {at:?} curved tilt {i}: pinned pixel bit for bit {same}");
            assert!(
                same,
                "the pinned pixel moved: {name} at {at:?}, curved tilt {i}"
            );
        }
    }
}

/// The pivot of a free view is the kernel's tilt about the centre, bit for bit, at every probe pixel.
fn check_free_pivot<C>(chart: &C, [x, y]: [u32; 2])
where
    C: ChartDecode + Tiltable<Param = f64> + Copy,
    f64: From<<C as kernel::chart::Chart>::Param>,
{
    let view = chart_view(chart, window([0.45, 0.55], 0.25));
    for spec in specs(x, y) {
        let ours =
            Tilted::placed(*chart, view.pivoted(&spec, SPAN).expect("a pivot")).expect("a tilt");
        let kernel = Tilted::new(*chart, Tilt::new(&spec, view.sim.chart.window.centre, SPAN))
            .expect("a tilt");
        for t in PROBE {
            for s in PROBE {
                let [u, w] = view.sim.chart.window.at(s, t);
                assert_eq!(
                    words(&ours.decode_at::<Defaults, f64>(u, w).ic),
                    words(&kernel.decode_at::<Defaults, f64>(u, w).ic),
                    "a free view's pivot is not the kernel's tilt about the centre"
                );
            }
        }
    }
}

fn check_charts(variant: Variant) {
    check_chart("mixed", &mixed(), [2, 3], variant);
    check_chart("invariant (L_z, E)", &invariant_le(), [0, 6], variant);
    check_chart("invariant (L_z, K)", &invariant_lk(), [1, 7], variant);
}

// Each guard has its own test and control, so each control is shown failing its own arm: the lock that re-centres on
// the latent chart and on the nonlinear charts; the raw tilt and the zoom about the centre on the latent chart (the
// chain's first step is a tilt); the window zoom and the curved tilt about the centre on the nonlinear charts.

#[test]
fn lock_moves_no_view() {
    check_latent(Variant::Object);
    check_free_pivot(&mixed(), [2, 3]);
    check_free_pivot(&invariant_le(), [0, 6]);
}

negative_control!(
    lock_moves_no_view,
    "a lock that re-centres the view, on the latent chart",
    expected = "the lock moved the view",
    check_latent(Variant::Recentres)
);

#[test]
fn lock_moves_no_view_on_charts() {
    check_charts(Variant::Object);
}

negative_control!(
    lock_moves_no_view_on_charts,
    "a lock that re-centres the view, on the nonlinear charts",
    expected = "the lock moved the view: mixed",
    check_charts(Variant::Recentres)
);

#[test]
fn lock_moves_no_view_tilt_about_pin() {
    check_latent(Variant::Object);
}

negative_control!(
    lock_moves_no_view_tilt_about_pin,
    "a raw tilt about the centre with the pin off it",
    expected = "the pinned pixel moved: axis at (0.3, 0.8), step 0",
    check_latent(Variant::TiltAboutCentre)
);

#[test]
fn lock_moves_no_view_zoom_about_pin() {
    check_latent(Variant::Object);
}

negative_control!(
    lock_moves_no_view_zoom_about_pin,
    "a zoom about the centre with the pin off it",
    expected = "the pinned pixel moved: axis at (0.3, 0.8), step 1",
    check_latent(Variant::ZoomAboutCentre)
);

#[test]
fn lock_moves_no_view_chart_zoom_about_pin() {
    check_charts(Variant::Object);
}

negative_control!(
    lock_moves_no_view_chart_zoom_about_pin,
    "a window zoom about the centre with the pin off it",
    expected = "the pinned pixel moved: mixed at (0.3, 0.8), zoom 0",
    check_charts(Variant::ZoomAboutCentre)
);

#[test]
fn lock_moves_no_view_curved_tilt_about_pin() {
    check_charts(Variant::Object);
}

negative_control!(
    lock_moves_no_view_curved_tilt_about_pin,
    "a curved tilt about the window's centre with the pin off it",
    expected = "the pinned pixel moved: mixed at (0.3, 0.8), curved tilt 0",
    check_charts(Variant::CurvedAboutCentre)
);

// ── Snap-back and centre on pin ────────────────────────────────────────────────────────────────────────────────────

/// `a·b`.
fn dot(a: &Latent, b: &Latent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `κ(B)` of `B = [q₁ q₂ | H]`, `H` orthonormal and orthogonal to the plane: the singular values are `q`'s Gram
/// matrix's square roots and six ones.
fn kappa(plane: &Plane) -> f64 {
    let (g11, g12, g22) = (
        dot(&plane.q1, &plane.q1),
        dot(&plane.q1, &plane.q2),
        dot(&plane.q2, &plane.q2),
    );
    let (m, d) = (
        (g11 + g22) / 2.0,
        (((g11 - g22) / 2.0).powi(2) + g12 * g12).sqrt(),
    );
    let (hi, lo) = ((m + d).sqrt(), (m - d).max(0.0).sqrt());
    hi.max(1.0) / lo.min(1.0)
}

/// Snap-back after a slice under an off-centre lock on the oblique (skew) plane: the pin unmoved, `Φ(pin) = z_locked`
/// bit for bit, the centre's hidden excursion within the construction's bound; snap-back then centre on pin is
/// `z₀ = z_locked` bit for bit.
fn check_snap_back(snap: fn(&View) -> View) {
    let plane = planes()
        .into_iter()
        .find(|(n, _)| *n == "skew")
        .expect("the skew plane")
        .1;
    let dirs = hidden(&plane).expect("hidden");
    for at in PINS {
        let locked = lock(&View::latent(sim_of(&plane)), at, Variant::Object);
        let z_locked = locked.sim.lock.z_locked;
        let sliced = Segment::slice(&locked, &dirs[1].direction, 0.37)
            .expect("a slice")
            .b()
            .clone();
        let sliced = Segment::slice(&sliced, &dirs[4].direction, -0.21)
            .expect("a slice")
            .b()
            .clone();
        let delta: Latent = std::array::from_fn(|i| sliced.sim.plane.z0[i] - z_locked[i]);
        let back = snap(&sliced);
        assert_eq!(
            back.pin.map(|p| p.address()),
            sliced.pin.map(|p| p.address()),
            "snap-back moved the pin"
        );
        let held = held_or_unreachable(
            &back.sim.plane,
            at,
            &z_locked,
            &format!("snap-back at {at:?}"),
        );
        let (a, b) = (at.0 + at.0 - 1.0, at.1 + at.1 - 1.0);
        let m: f64 = (0..8)
            .map(|i| {
                (z_locked[i].abs() + a.abs() * plane.q1[i].abs() + b.abs() * plane.q2[i].abs())
                    .powi(2)
            })
            .sum::<f64>()
            .sqrt();
        let excursion: Latent = std::array::from_fn(|i| back.sim.plane.z0[i] - z_locked[i]);
        let h = dirs
            .iter()
            .map(|d| dot(&d.direction, &excursion).powi(2))
            .sum::<f64>()
            .sqrt();
        let bound = 8.0 * f64::EPSILON * m;
        eprintln!(
            "snap-back skew pin {at:?}: κ(B) = {:.4}, ‖δ‖ = {:.4}, Φ(pin) bit for bit {held}/8, ‖Hᵀ(z₀ − z_locked)‖ = \
             {h:.3e} against 8·ε·M = {bound:.3e}",
            kappa(&plane),
            dot(&delta, &delta).sqrt()
        );
        assert!(
            h <= bound,
            "snap-back left a hidden excursion {h:e} > {bound:e} at {at:?}"
        );
        let centred = Segment::centre_on_pin(&back)
            .expect("centre on pin")
            .b()
            .clone();
        assert_eq!(
            exact(&centred.sim.plane.z0, &z_locked),
            8,
            "snap-back then centre on pin is not z₀ ← z_locked at {at:?}"
        );
        assert_eq!(
            centred.pin,
            Some(Pin::Centre),
            "centre on pin left the pin off the centre"
        );
    }
}

/// Snap-back through the object.
fn snap_object(v: &View) -> View {
    Segment::snap_back(v).expect("snap-back").b().clone()
}

#[test]
fn lock_moves_no_view_snap_back() {
    check_snap_back(snap_object);
}

/// Today's snap-back before the pin, `z₀ ← z_locked`: it re-centres.
#[cfg(feature = "controls")]
fn snap_recentres(v: &View) -> View {
    let mut out = v.clone();
    out.sim.plane.z0 = v.sim.lock.z_locked;
    out
}

negative_control!(
    lock_moves_no_view_snap_back,
    "snap-back that re-centres, z₀ ← z_locked",
    expected = "the pinned pixel moved: snap-back",
    check_snap_back(snap_recentres)
);

// ── The pinned view through the path object ────────────────────────────────────────────────────────────────────────
//
// The criterion: under an off-centre pin, a zoom's and a raw tilt's interior frame at `s` is the direct operation at the
// scaled parameter, bit for bit (one formula, the family's); a pinned view's frame record reads back to the same frame,
// bit for bit, the pin in its bytes. Control: the frame of a pinned zoom compared with the zoom of the view stripped of
// its pin, which does not turn about the pin.

/// The pinned skew view, locked at `PINS[0]`.
fn pinned() -> View {
    let plane = planes()
        .into_iter()
        .find(|(n, _)| *n == "skew")
        .expect("skew")
        .1;
    lock(&View::latent(sim_of(&plane)), PINS[0], Variant::Object)
}

fn check_interior(direct_from: fn(&View) -> View) {
    let a = pinned();
    let dirs = hidden(&a.sim.plane).expect("hidden");
    let constructor = Constructor::new(a.sim.plane.clone());
    for s in [0.25, 0.5, 0.875] {
        let zoom = Segment::zoom(&a, 2.0).expect("a zoom");
        let direct = Segment::zoom(&direct_from(&a), s * 2.0).expect("a zoom");
        assert_eq!(
            zoom.frame(s).expect("a frame").bytes(),
            View {
                pin: a.pin,
                ..direct.b().clone()
            }
            .bytes(),
            "a pinned zoom's interior frame is not the zoom at the scaled parameter, s = {s}"
        );
        let tilt = Segment::raw_tilt(
            &constructor,
            &a,
            BasisVector::Q1,
            (0.0, 40.0),
            &dirs[2].direction,
        )
        .expect("a tilt");
        let direct = Segment::raw_tilt(
            &constructor,
            &direct_from(&a),
            BasisVector::Q1,
            (0.0, s * 40.0),
            &dirs[2].direction,
        )
        .expect("a tilt");
        assert_eq!(
            tilt.frame(s).expect("a frame").bytes(),
            View {
                pin: a.pin,
                ..direct.b().clone()
            }
            .bytes(),
            "a pinned tilt's interior frame is not the tilt at the scaled parameter, s = {s}"
        );
        let record = zoom.record(s).expect("a record");
        let read =
            engine::path::FrameRecord::read(&record.to_bytes()).expect("a record reads back");
        assert_eq!(
            read.frame().expect("a frame").bytes(),
            zoom.frame(s).expect("a frame").bytes(),
            "a pinned view's record does not read back to its frame"
        );
        assert!(
            String::from_utf8_lossy(&a.bytes()).contains("\"pin\""),
            "the pin is not in the view's bytes"
        );
    }
}

/// The view as it is.
fn as_is(v: &View) -> View {
    v.clone()
}

#[test]
fn lock_moves_no_view_interior_frames() {
    check_interior(as_is);
}

/// The view stripped of its pin.
#[cfg(feature = "controls")]
fn unpinned(v: &View) -> View {
    View {
        pin: None,
        ..v.clone()
    }
}

negative_control!(
    lock_moves_no_view_interior_frames,
    "a pinned zoom compared with the zoom of the view stripped of its pin",
    expected = "a pinned zoom's interior frame is not the zoom at the scaled parameter",
    check_interior(unpinned)
);

// ── The sweep: how often the pin is exact ──────────────────────────────────────────────────────────────────────────
//
// A measurement, not a gate on the rate: random planes (unit-scale and deep), pins and chains of zooms and raw tilts
// about the pin, counting the components of `Φ(pin)` that are `z_locked` bit for bit and the worst miss. The criterion it
// does gate is the latent check's: a component that misses is unreachable and within one ulp of the largest number `Φ`
// adds there.

/// A small deterministic generator (SplitMix64).
struct Rng(u64);

impl Rng {
    fn u(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn latent(&mut self, scale: f64) -> Latent {
        std::array::from_fn(|_| (2.0 * self.u() - 1.0) * scale)
    }
}

#[test]
fn lock_moves_no_view_sweep() {
    sweep(Variant::Object);
}

negative_control!(
    lock_moves_no_view_sweep,
    "the sweep's zooms and tilts about the centre with the pin off it",
    expected = "the pinned pixel moved: sweep chain",
    sweep(Variant::ZoomAboutCentre)
);

/// The sweep, its zooms by the variant.
fn sweep(variant: Variant) {
    let mut rng = Rng(62);
    let (mut exact_n, mut total) = (0usize, 0usize);
    let mut chains_exact = 0usize;
    let mut worst_state = 0.0f64;
    let state =
        |z: &Latent| kernel::decode::decode::<Defaults, f64>(&kernel::decode::Latent::new(*z)).ic;
    const CHAINS: usize = 400;
    for c in 0..CHAINS {
        let scale = match c % 4 {
            3 => (-20.0f64).exp2(),
            _ => 1.0,
        };
        let plane = Plane {
            z0: rng.latent(1.0),
            q1: rng.latent(scale),
            q2: rng.latent(scale),
        };
        let Ok(dirs) = hidden(&plane) else { continue };
        let at = (rng.u() * 1.4 - 0.2, rng.u() * 1.4 - 0.2);
        let locked = lock(&View::latent(sim_of(&plane)), at, Variant::Object);
        let z_locked = locked.sim.lock.z_locked;
        let constructor = Constructor::new(plane.clone());
        let mut view = locked;
        let mut all = true;
        for i in 0..4 {
            view = match i % 2 {
                0 => {
                    let octaves = rng.u() * 6.0 - 3.0;
                    tilted(&view, variant, Step::Zoom, |v| {
                        Segment::zoom(v, octaves).expect("a zoom")
                    })
                }
                _ => {
                    let (tau, k) = (rng.u() * 120.0 - 60.0, (rng.u() * 6.0) as usize % 6);
                    tilted(&view, variant, Step::Tilt, |v| {
                        Segment::raw_tilt(
                            &constructor,
                            v,
                            BasisVector::Q2,
                            (0.0, tau),
                            &dirs[k].direction,
                        )
                        .expect("a tilt")
                    })
                }
            };
            let n = held_or_unreachable(
                &view.sim.plane,
                at,
                &z_locked,
                &format!("sweep chain {c}, step {i}"),
            );
            exact_n += n;
            total += 8;
            worst_state = worst_state.max(relative(
                &state(&point(&view.sim.plane, at.0, at.1)),
                &state(&z_locked),
            ));
            all &= n == 8;
        }
        chains_exact += usize::from(all);
    }
    eprintln!(
        "sweep: {exact_n}/{total} components of Φ(pin) bit for bit ({:.2}%); {chains_exact}/{CHAINS} chains exact \
         throughout; every miss unreachable and within one ulp; the pinned pixel's decoded state's largest relative \
         difference from z_locked's {worst_state:.3e}",
        100.0 * exact_n as f64 / total as f64
    );
}

// ── Snap-back on a nonlinear chart ─────────────────────────────────────────────────────────────────────────────────
//
// The criterion: on the mixed chart locked off-centre, after a slider's move of a hidden control, snap-back gives
// `z₀ = z_locked` bit for bit, the window and the pin's address unmoved. Control: a snap-back that keeps the move.

fn check_chart_snap_back(snap: fn(&View) -> View) {
    let chart = mixed();
    let a = chart_view(&chart, window([0.45, 0.55], 0.25));
    let (locked, _) = chart_lock(&a, &chart, CHART_PINS[0], Variant::Object);
    let moved = Segment::control(&locked, 2, 0.9)
        .expect("a move")
        .b()
        .clone();
    let back = snap(&moved);
    assert_eq!(
        exact(&back.sim.plane.z0, &locked.sim.lock.z_locked),
        8,
        "snap-back on a chart left the excursion"
    );
    assert_eq!(
        back.sim.chart.window, locked.sim.chart.window,
        "snap-back on a chart moved the window"
    );
    assert_eq!(
        back.pin.map(|p| p.address()),
        locked.pin.map(|p| p.address()),
        "snap-back on a chart moved the pin"
    );
}

#[test]
fn lock_moves_no_view_chart_snap_back() {
    check_chart_snap_back(snap_object);
}

/// A snap-back that keeps the move.
#[cfg(feature = "controls")]
fn snap_keeps(v: &View) -> View {
    v.clone()
}

negative_control!(
    lock_moves_no_view_chart_snap_back,
    "a snap-back that keeps the move",
    expected = "snap-back on a chart left the excursion",
    check_chart_snap_back(snap_keeps)
);
