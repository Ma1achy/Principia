//! The blend chart (TASK-M2-90; design § 17.2, § 17.4, § 17.11; R-926 item 3, R-936, R-943, R-944, R-975):
//!
//! - `blend_chart_ends_bitwise`: `s = 0` and `s = 1` are the ends' own pixels bit for bit, through the chart and
//!   through its segment. Control: the ends taken by the block line.
//! - `blend_chart_nests`: a blend's end may be a blend; the outer ends are the inner blend's pixel and the other end's,
//!   and its interior the block line between them; a nested frame's record reads back. Control: the nested end read at
//!   its own `s = 0`.
//! - `blend_chart_navigation_reaches_both_ends`: pan, zoom and turn of a blend are each end's, recursively. Control: a
//!   pan of end `A` alone.
//! - `blend_chart_exposed`: the f64 forward map and decode, as placement and `computeIC` call them. Control: the
//!   decode read against the forward map at another share.
//! - `blend_chart_collision_disc`: a path through the shape sphere's binary collision point gives a small disc of
//!   collisions at `t = 0`, pair and least separation recorded, never blank and never `decode_failed` (R-933).
//!   Control: the disc blanked.
//! - `blend_chart_m01_arm1`, `blend_chart_m01_arm2`: a path whose `M₀₁` dips below `ε_M01` between two valid ends
//!   holds the masses (R-975), with the controls the task names. Control: the blend without the hold.
//! - `blend_chart_fold_indicator`: § 17.11's signed fold indicator reads the fold of a blend toward its mirror image
//!   and none on an unfolded blend. Control: the unsigned indicator.
//! - `blend_chart_round_trip`: an end's state rebuilt from its blocks, non-zero momenta, and the blend just off `s = 0`
//!   at end A. Control: the momenta's inverse with the wrong mass ratio.
//! - `blend_chart_tagged_end`: an end tagged with a reason that carries no system blanks the pixel; one that carries
//!   one blends. Control: the tag dropped.
//! - `blend_chart_held_energy_miss`, `blend_chart_lifted_jump`: measurements, reported, not gates; each asserts only
//!   its fixture, with a control on that.

use engine::contract::sim_config::{Plane, SimConfig, Window};
use engine::path::blend::{
    block_line, fold_indicator, masses_of, state_of, Blend, Blocks, ChartLeaves, End, Flags,
    Forward, LatentLeaves, Leaves, Pixel,
};
use engine::path::{BlendPath, FrameRecord, Map, PathError, Segment, View};
use kernel::canon;
use kernel::chart::invariant::{Invariant, InvariantLK};
use kernel::chart::shape_sphere::{place, ShapeMap, ShapeSphere};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::descriptor;
use kernel::decode::mass::{self, Masses};
use kernel::decode::momentum::Momentum;
use kernel::decode::{energy_norm, Decoded, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_detecting, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{pa_d_min, sd_dmin_pair, sd_state, tm_t_end_step, STATE_COLLISION};
use kernel::physics::energy::energy;
use kernel::physics::pairs::PairGeom;
use validation::negative_control;

mod path_shim;
use path_shim::sim_of;

type Latent = LatentLeaves<Defaults>;

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The skewed latent plane of the path tests.
fn plane_a() -> Plane {
    Plane {
        z0: [0.3, -0.0, 0.1, -0.2, 0.05, 0.0, 0.4, -0.25],
        q1: [0.7, 0.2, -0.0, 0.1, 0.0, 0.0, 0.05, 0.0],
        q2: [0.037, 0.333, 0.111, -0.074, 0.0, 0.037, 0.0, -0.0],
    }
}

/// Another plane, every block moved.
fn plane_b() -> Plane {
    Plane {
        z0: [-0.6, 0.45, -0.3, 0.25, 0.4, -0.15, -0.8, 0.6],
        q1: [0.1, -0.9, 0.2, 0.0, -0.1, 0.05, 0.3, 0.0],
        q2: [0.8, 0.1, 0.0, 0.2, 0.15, 0.0, -0.1, 0.4],
    }
}

fn latent(plane: &Plane) -> View {
    View::latent(sim_of(plane))
}

/// The exponential shape sphere about the collision of bodies 0 and 1, `n₀ = (−1, 0, 0)`, at equal masses from rest.
fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0: [-1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.0, 0.0],
        z_mom: [0.0; 4],
    }
}

/// A view of [`sphere`] through `window`.
fn sphere_view(window: Window) -> View {
    let mut sim: SimConfig = sim_of(&plane_a());
    sim.chart.window = window;
    View::kernel(&sphere(), sim).expect("the shape sphere is a registry chart")
}

/// The latent map's and the shape sphere's leaves.
fn leaves() -> (Latent, ChartLeaves<ShapeSphere<f64>, Defaults>) {
    (
        Latent::new(),
        ChartLeaves::new(sphere()).expect("a registry chart"),
    )
}

/// The `n × n` grid of pixel centres.
fn grid(n: usize) -> Vec<[f64; 2]> {
    let c = |i: usize| (i as f64 + 0.5) / n as f64;
    (0..n * n).map(|k| [c(k % n), c(k / n)]).collect()
}

/// A pixel's state; panics on a blank.
fn state(pixel: &Pixel) -> (&Decoded<f64>, Flags) {
    match pixel {
        Pixel::State { decoded, flags } => (decoded, *flags),
        Pixel::Blank(b) => panic!("a blank pixel: {b:?}"),
    }
}

/// A decode's words: `(m, r, p)` and the tag, by bits.
fn words(d: &Decoded<f64>) -> (Vec<u64>, Option<&'static str>) {
    let ic = &d.ic;
    let w =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten())
            .map(|x| x.to_bits())
            .collect();
    (w, d.degenerate.map(|t| t.name()))
}

// ── blend_chart_ends_bitwise ───────────────────────────────────────────────────────────────────────────────────────
//
// Pass: at every pixel of a 16² grid, the blend at `s = 0` is end A's decode and at `s = 1` end B's, every word and the
// tag; the segment's frames at 0 and 1 are the stored views, by bytes. Reported: how many pixels the block line at
// `s = 1` would have missed, the reason the end rule exists.

/// The pixel the blend shows at `s` at `p`, by the chart's own rule.
fn by_chart(blend: &Blend, s: f64, p: [f64; 2]) -> Pixel {
    blend.at(s).unwrap().decode(&leaves(), p).unwrap()
}

fn check_ends(at: fn(&Blend, f64, [f64; 2]) -> Pixel) {
    let (a, b) = (latent(&plane_a()), latent(&plane_b()));
    let blend = Blend::new(a.clone(), b.clone(), BlendPath::BlockLine, 0.5).unwrap();
    let l = leaves();
    for p in grid(16) {
        let own_a = l.decode(&a, p).unwrap();
        let own_b = l.decode(&b, p).unwrap();
        assert_eq!(
            words(state(&at(&blend, 0.0, p)).0),
            words(&own_a),
            "end A is not bit for bit at {p:?}"
        );
        assert_eq!(
            words(state(&at(&blend, 1.0, p)).0),
            words(&own_b),
            "end B is not bit for bit at {p:?}"
        );
    }
    let seg = Segment::blend(&a, &b, BlendPath::BlockLine);
    assert_eq!(
        seg.frame(0.0).unwrap().bytes(),
        a.bytes(),
        "the segment's frame at 0 is not A"
    );
    assert_eq!(
        seg.frame(1.0).unwrap().bytes(),
        b.bytes(),
        "the segment's frame at 1 is not B"
    );
    assert_eq!(
        seg.frame(-0.0).unwrap().bytes(),
        a.bytes(),
        "the segment's frame at −0 is not A"
    );
}

#[test]
fn blend_chart_ends_bitwise() {
    check_ends(by_chart);
    let (a, b) = (latent(&plane_a()), latent(&plane_b()));
    let l = leaves();
    let pixels = grid(16);
    let missed = pixels
        .iter()
        .filter(|p| {
            let (da, db) = (l.decode(&a, **p).unwrap(), l.decode(&b, **p).unwrap());
            let (line, _) = block_line(&Blocks::of(&da.ic), &Blocks::of(&db.ic), 1.0);
            words(&state_of(&line).0) != words(&db)
        })
        .count();
    println!(
        "blend_chart_ends_bitwise: the block line at s = 1 misses end B's bits on {missed} of {} pixels",
        pixels.len()
    );
}

/// The ends taken by the block line, as a blend with no end rule would.
#[cfg(feature = "controls")]
fn by_line(blend: &Blend, s: f64, p: [f64; 2]) -> Pixel {
    let l = leaves();
    let (da, db) = (
        l.decode(&blend.a, p).unwrap(),
        l.decode(&blend.b, p).unwrap(),
    );
    let (line, _) = block_line(&Blocks::of(&da.ic), &Blocks::of(&db.ic), s);
    Pixel::State {
        decoded: Box::new(state_of(&line).0),
        flags: Flags::default(),
    }
}

negative_control!(
    blend_chart_ends_bitwise,
    "the ends taken by the block line, not selected exactly",
    expected = "is not bit for bit",
    check_ends(by_line)
);

/// End A selected exactly, end B taken by the block line.
#[cfg(feature = "controls")]
fn by_line_at_b(blend: &Blend, s: f64, p: [f64; 2]) -> Pixel {
    match s {
        1.0 => by_line(blend, s, p),
        _ => by_chart(blend, s, p),
    }
}

mod end_b_control {
    use super::*;

    negative_control!(
        blend_chart_ends_bitwise,
        "end B taken by the block line",
        expected = "end B is not bit for bit",
        check_ends(by_line_at_b)
    );
}

// ── blend_chart_nests ──────────────────────────────────────────────────────────────────────────────────────────────
//
// Pass: with the inner blend (A, B) at 0.3 the end of an outer blend toward C (the shape sphere), at every pixel of a
// 12² grid the outer at 0 is the inner's pixel and at 1 C's, bit for bit; the outer at 0.5 is the block line between
// those two states, bit for bit; and the outer's frame record reads back to the same bytes.

fn inner() -> Blend {
    Blend::new(
        latent(&plane_a()),
        latent(&plane_b()),
        BlendPath::BlockLine,
        0.3,
    )
    .unwrap()
}

fn sphere_square() -> View {
    sphere_view(Window {
        centre: [0.5, 0.5],
        half: [0.5, 0.5],
    })
}

/// The pixel the nested end shows, by the chart.
fn nested_end(inner: &Blend, p: [f64; 2]) -> Pixel {
    inner.decode(&leaves(), p).unwrap()
}

fn check_nest(end: fn(&Blend, [f64; 2]) -> Pixel) {
    let inner = inner();
    let c = sphere_square();
    let outer = Blend::new(
        View::blend(inner.clone()),
        c.clone(),
        BlendPath::BlockLine,
        0.5,
    )
    .unwrap();
    let l = leaves();
    for p in grid(12) {
        let own = end(&inner, p);
        let (own, _) = state(&own);
        let at0 = outer.at(0.0).unwrap().decode(&l, p).unwrap();
        assert_eq!(
            words(state(&at0).0),
            words(own),
            "the nested end's pixel is not the inner blend's at {p:?}"
        );
        let own_c = l.decode(&c, p).unwrap();
        let at1 = outer.at(1.0).unwrap().decode(&l, p).unwrap();
        assert_eq!(
            words(state(&at1).0),
            words(&own_c),
            "end C is not bit for bit at {p:?}"
        );
        let (line, _) = block_line(&Blocks::of(&own.ic), &Blocks::of(&own_c.ic), 0.5);
        let mid = outer.decode(&l, p).unwrap();
        assert_eq!(
            words(state(&mid).0),
            words(&state_of(&line).0),
            "the outer interior is not the block line from the inner blend's state at {p:?}"
        );
    }
    let seg = Segment::blend(&View::blend(inner), &c, BlendPath::BlockLine);
    let record = seg.record(0.5).unwrap();
    let read = FrameRecord::read(&record.to_bytes()).unwrap();
    assert_eq!(
        read.to_bytes(),
        record.to_bytes(),
        "a nested record does not read back"
    );
    assert_eq!(
        read.frame().unwrap().bytes(),
        seg.frame(0.5).unwrap().bytes(),
        "a nested record's frame is not the segment's"
    );
}

#[test]
fn blend_chart_nests() {
    check_nest(nested_end);
}

/// The nested end read at its own `s = 0`, its share ignored.
#[cfg(feature = "controls")]
fn nested_at_zero(inner: &Blend, p: [f64; 2]) -> Pixel {
    inner.at(0.0).unwrap().decode(&leaves(), p).unwrap()
}

negative_control!(
    blend_chart_nests,
    "the nested end read at its own s = 0, its share ignored",
    expected = "the nested end's pixel is not the inner blend's",
    check_nest(nested_at_zero)
);

// ── blend_chart_navigation_reaches_both_ends ───────────────────────────────────────────────────────────────────────
//
// Pass: a pan, a zoom and a turn of a blend of the latent map and the shape sphere, and of a nested blend: each end
// of the edited blend is that edit of the end through the chart-path object, by bytes (a pan after a turn the pan
// turned); at `s = 0` and `s = 1` the edited blend's pixel is the edited end's own, bit for bit; at `s = ½` the panned
// blend's pixel `p` is the unpanned blend's pixel `p + d` to `1e-9` in every word (a pan of the screen, both ends
// moved together); at `s = ½` the turned blend's pixel `p` is the unturned blend's pixel at `R(θ)(p − ½) + ½`, the
// turn computed here, to `1e-9` in every word; the segment's pan of a blend view gives the same blend. Controls, one per
// edit: a pan of end A alone, a zoom of end A alone, a turn the wrong way.

const PAN: (f64, f64) = (0.1, -0.05);
const ZOOM: f64 = 0.75;
const TURN: f64 = 0.4;

/// The three edits under test: pan, zoom and turn.
struct Edits {
    pan: fn(&Blend, f64, f64) -> Blend,
    zoom: fn(&Blend, f64) -> Blend,
    turn: fn(&Blend, f64) -> Blend,
}

/// The blend's own edits.
const OWN: Edits = Edits {
    pan: |b, ds, dt| b.pan(ds, dt).unwrap(),
    zoom: |b, k| b.zoom(k).unwrap(),
    turn: |b, r| b.turned(r).unwrap(),
};

/// Two decodes' words equal to `1e-9` relative.
fn near(x: &Pixel, y: &Pixel, what: &str) {
    let (x, y) = (words(state(x).0).0, words(state(y).0).0);
    for (u, v) in x.iter().zip(&y) {
        let (u, v) = (f64::from_bits(*u), f64::from_bits(*v));
        assert!(
            (u - v).abs() <= 1e-9 * (1.0 + v.abs()),
            "{what}: {u} against {v}"
        );
    }
}

fn rotated([ds, dt]: [f64; 2], turn: f64) -> [f64; 2] {
    let (s, c) = (libm::sin(turn), libm::cos(turn));
    [c * ds - s * dt, s * ds + c * dt]
}

fn pan_view(v: &View, d: [f64; 2]) -> View {
    match &v.map {
        Map::Blend(b) => View::blend(b.pan(d[0], d[1]).unwrap()),
        _ => Segment::pan(v, d[0], d[1]).unwrap().b().clone(),
    }
}

fn check_navigation(edits: &Edits) {
    let pan = edits.pan;
    let l = leaves();
    let a = latent(&plane_a());
    let b = sphere_square();
    let flat = Blend::new(a.clone(), b.clone(), BlendPath::BlockLine, 0.5).unwrap();
    let nested = Blend::new(View::blend(inner()), b.clone(), BlendPath::BlockLine, 0.5).unwrap();
    for blend in [flat, nested] {
        let (ds, dt) = PAN;
        let panned = pan(&blend, ds, dt);
        for (end, view) in [(End::A, &blend.a), (End::B, &blend.b)] {
            assert_eq!(
                panned.end(end).bytes(),
                pan_view(view, [ds, dt]).bytes(),
                "end {end:?} was not panned"
            );
        }
        let zoomed = (edits.zoom)(&blend, ZOOM);
        for (end, view) in [(End::A, &blend.a), (End::B, &blend.b)] {
            let want = match &view.map {
                Map::Blend(inner) => View::blend(inner.zoom(ZOOM).unwrap()),
                _ => Segment::zoom_factor(view, ZOOM).unwrap().b().clone(),
            };
            assert_eq!(
                zoomed.end(end).bytes(),
                want.bytes(),
                "end {end:?} was not zoomed"
            );
        }
        let turned = (edits.turn)(&blend, TURN);
        for p in grid(8) {
            for (s, end) in [(0.0, End::A), (1.0, End::B)] {
                let own = |b: &Blend, q: [f64; 2]| match &b.end(end).map {
                    Map::Blend(inner) => inner.decode(&l, q).unwrap(),
                    _ => Pixel::State {
                        decoded: Box::new(l.decode(b.end(end), q).unwrap()),
                        flags: Flags::default(),
                    },
                };
                for edited in [&panned, &zoomed, &turned] {
                    let shown = edited.at(s).unwrap().decode(&l, p).unwrap();
                    let want = own(edited, edited.address(p));
                    assert_eq!(
                        words(state(&shown).0),
                        words(state(&want).0),
                        "the edited blend at s = {s} is not its edited end at {p:?}"
                    );
                }
            }
            let moved = [p[0] + ds, p[1] + dt];
            near(
                &panned.decode(&l, p).unwrap(),
                &blend.decode(&l, moved).unwrap(),
                &format!("the panned blend's pixel {p:?} is not the blend's pixel {moved:?}"),
            );
            let [x, y] = rotated([p[0] - 0.5, p[1] - 0.5], TURN);
            let under = [0.5 + x, 0.5 + y];
            near(
                &turned.decode(&l, p).unwrap(),
                &blend.decode(&l, under).unwrap(),
                &format!("the turned blend's pixel {p:?} is not the blend's pixel {under:?}"),
            );
        }
        let turned_panned = pan(&turned, ds, dt);
        for (end, view) in [(End::A, &blend.a), (End::B, &blend.b)] {
            assert_eq!(
                turned_panned.end(end).bytes(),
                pan_view(view, rotated([ds, dt], TURN)).bytes(),
                "end {end:?} was not panned along the turned screen"
            );
        }
        let seg = Segment::pan(&View::blend(blend.clone()), ds, dt).unwrap();
        assert_eq!(
            seg.b().bytes(),
            View::blend(blend.pan(ds, dt).unwrap()).bytes(),
            "the segment's pan of a blend view is not the blend's pan"
        );
        assert_eq!(
            seg.frame(0.5).unwrap().bytes(),
            View::blend(blend.pan(0.5 * ds, 0.5 * dt).unwrap()).bytes(),
            "the segment's mid frame is not the blend panned by half"
        );
        let turn = Segment::turn(&View::blend(blend.clone()), TURN).unwrap();
        assert_eq!(
            turn.b().bytes(),
            View::blend(blend.turned(TURN).unwrap()).bytes(),
            "the segment's turn is not the blend's"
        );
    }
    assert_eq!(
        Segment::turn(&a, TURN).unwrap_err(),
        PathError::WrongMap,
        "a turn of a view that is not a blend"
    );
}

#[test]
fn blend_chart_navigation_reaches_both_ends() {
    check_navigation(&OWN);
}

/// A pan of end A alone.
#[cfg(feature = "controls")]
fn pan_a_only(blend: &Blend, ds: f64, dt: f64) -> Blend {
    let a = pan_view(&blend.a, rotated([ds, dt], blend.turn()));
    let mut b = Blend::new(a, blend.b.clone(), blend.path, blend.s()).unwrap();
    if blend.turn() != 0.0 {
        b = b.turned(blend.turn()).unwrap();
    }
    b
}

negative_control!(
    blend_chart_navigation_reaches_both_ends,
    "a pan of end A alone",
    expected = "end B was not panned",
    check_navigation(&Edits {
        pan: pan_a_only,
        ..OWN
    })
);

/// A zoom of end A alone.
#[cfg(feature = "controls")]
fn zoom_a_only(blend: &Blend, k: f64) -> Blend {
    let a = match &blend.a.map {
        Map::Blend(inner) => View::blend(inner.zoom(k).unwrap()),
        _ => Segment::zoom_factor(&blend.a, k).unwrap().b().clone(),
    };
    Blend::new(a, blend.b.clone(), blend.path, blend.s()).unwrap()
}

mod zoom_control {
    use super::*;

    negative_control!(
        blend_chart_navigation_reaches_both_ends,
        "a zoom of end A alone",
        expected = "end B was not zoomed",
        check_navigation(&Edits {
            zoom: zoom_a_only,
            ..OWN
        })
    );
}

mod turn_control {
    use super::*;

    negative_control!(
        blend_chart_navigation_reaches_both_ends,
        "a turn the wrong way",
        expected = "the turned blend's pixel",
        check_navigation(&Edits {
            turn: |b, r| b.turned(-r).unwrap(),
            ..OWN
        })
    );
}

// ── blend_chart_exposed ────────────────────────────────────────────────────────────────────────────────────────────
//
// Pass: on a 12² grid at `s = 0.37`, `forward` gives the blocks and `decode` the state built from them, bit for bit;
// the state is canonical (CoM at the origin and at rest, `I = 1`, `ρ̃` on `+x`, each to `1e-12`) and integrates.

fn check_exposed(forward_at: f64) {
    let blend = Blend::new(
        latent(&plane_a()),
        latent(&plane_b()),
        BlendPath::BlockLine,
        0.37,
    )
    .unwrap();
    let probe = blend.at(forward_at).unwrap();
    let l = leaves();
    for p in grid(12) {
        let Forward::Point { blocks, .. } = probe.forward(&l, p).unwrap() else {
            panic!("a blank pixel at {p:?}")
        };
        let pixel = blend.decode(&l, p).unwrap();
        let (decoded, _) = state(&pixel);
        assert_eq!(
            words(decoded),
            words(&state_of(&blocks).0),
            "the decode is not the forward map's state at {p:?}"
        );
        let ic = &decoded.ic;
        let com: Vec<f64> = (0..2)
            .map(|k| (0..3).map(|i| ic.m[i] * ic.r[i][k]).sum())
            .collect();
        let total: Vec<f64> = (0..2).map(|k| (0..3).map(|i| ic.p[i][k]).sum()).collect();
        let inertia: f64 = (0..3)
            .map(|i| ic.m[i] * (ic.r[i][0].powi(2) + ic.r[i][1].powi(2)))
            .sum();
        assert!(
            com.iter().chain(&total).all(|x| x.abs() < 1e-12) && (inertia - 1.0).abs() < 1e-12,
            "the decode is not canonical at {p:?}: {com:?} {total:?} {inertia}"
        );
        let rho_y = ic.r[1][1] - ic.r[0][1];
        assert!(
            rho_y.abs() < 1e-12 && ic.r[1][0] > ic.r[0][0],
            "ρ is not on +x at {p:?}"
        );
        let out =
            integrate_detecting::<Kdk, f64>(ic, &uniforms(), CollisionRadius::from_r_coll(R_COLL));
        assert!(
            out.r.iter().flatten().all(|x| x.is_finite()),
            "the decode does not integrate"
        );
    }
}

#[test]
fn blend_chart_exposed() {
    check_exposed(0.37);
}

negative_control!(
    blend_chart_exposed,
    "the decode read against the forward map at another share",
    expected = "the decode is not the forward map's state",
    check_exposed(0.38)
);

// ── blend_chart_round_trip ──────────────────────────────────────────────────────────────────────────────────────────
//
// Pass: on a 12² grid of both latent planes, whose momenta are not zero (at least one end word above 0.05 in size),
// every word of the state rebuilt from an end's blocks is the end's own to `1e-12·(1 + |x|)`; and the blend at
// `s = 10⁻¹²` is end A's state to `1e-9·(1 + |x|)` (continuous at the end). Control: the momenta's inverse taken with
// `m₀/M₀₁` in place of `m₁/M₀₁`.

/// An end's blocks with the momenta inverted with `m₀/M₀₁` in place of `m₁/M₀₁`.
#[cfg(feature = "controls")]
fn swapped_blocks(ic: &Ic<f64>) -> Blocks {
    let mut b = Blocks::of(ic);
    let w0 = ic.m[0] / mass::from_masses(ic.m).m01;
    let p2 = ic.p[2];
    b.p_rho = [ic.p[1][0] + w0 * p2[0], ic.p[1][1] + w0 * p2[1]];
    b
}

fn check_round_trip(blocks_of: fn(&Ic<f64>) -> Blocks) {
    let l = leaves();
    let (a, b) = (latent(&plane_a()), latent(&plane_b()));
    let blend = Blend::new(a.clone(), b.clone(), BlendPath::BlockLine, 1e-12).unwrap();
    let mut largest = 0.0_f64;
    for p in grid(12) {
        for end in [&a, &b] {
            let own = l.decode(end, p).unwrap();
            largest = own
                .ic
                .p
                .iter()
                .flatten()
                .fold(largest, |m, x| m.max(x.abs()));
            let back = state_of(&blocks_of(&own.ic)).0;
            for (u, v) in words(&back).0.iter().zip(&words(&own).0) {
                let (u, v) = (f64::from_bits(*u), f64::from_bits(*v));
                assert!(
                    (u - v).abs() <= 1e-12 * (1.0 + v.abs()),
                    "the round trip misses at {p:?}: {u} against {v}"
                );
            }
        }
        let own = Pixel::State {
            decoded: Box::new(l.decode(&a, p).unwrap()),
            flags: Flags::default(),
        };
        near(
            &blend.decode(&l, p).unwrap(),
            &own,
            &format!("the blend at s = 1e-12 is not end A at {p:?}"),
        );
    }
    assert!(
        largest > 0.05,
        "the fixture's momenta are too small: {largest}"
    );
}

#[test]
fn blend_chart_round_trip() {
    check_round_trip(Blocks::of);
}

negative_control!(
    blend_chart_round_trip,
    "the momenta's inverse taken with m₀/M₀₁ in place of m₁/M₀₁",
    expected = "the round trip misses",
    check_round_trip(swapped_blocks)
);

// ── blend_chart_tagged_end ─────────────────────────────────────────────────────────────────────────────────────────
//
// Pass (§ 17.11's table): with end A's decode tagged by each of the decoder's five reasons in turn, its momenta not 0,
// the pixel at `s = ½` is blank for `M01_TINY` and `NO_QUALIFYING_SEED` and a state for `ENERGY_INFEASIBLE`,
// `ENERGY_UNSCALABLE` and `INVARIANT_INFEASIBLE` (its rigid-rotation arm). On the invariant `(L_z, K)` chart itself:
// end A past the overflow edge, `t = 3·10²⁰⁵`, decodes `INVARIANT_INFEASIBLE` with every momentum 0 and the pixel is
// blank; end A past `L_max`, `s = 1.5`, decodes `INVARIANT_INFEASIBLE` with momenta and the pixel is a state. Control:
// the tag dropped on its way into the blend.

/// The latent map's leaves with end A's decode (its plane's `z₀[0]` is plane A's) tagged `tag`.
struct Tagged {
    tag: Degenerate,
    keep: bool,
}

impl Leaves for Tagged {
    fn decode(&self, view: &View, p: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        let mut d = Latent::new().decode(view, p)?;
        if self.keep && view.sim.plane.z0 == plane_a().z0 {
            d.degenerate = Some(self.tag);
        }
        Ok(d)
    }
}

fn check_tagged(keep: bool) {
    let blend = Blend::new(
        latent(&plane_a()),
        latent(&plane_b()),
        BlendPath::BlockLine,
        0.5,
    )
    .unwrap();
    for tag in kernel::decode::degenerate::ALL {
        let pixel = blend.decode(&Tagged { tag, keep }, [0.5, 0.5]).unwrap();
        let blank = matches!(tag, Degenerate::M01Tiny | Degenerate::NoQualifyingSeed);
        assert_eq!(
            matches!(pixel, Pixel::Blank(_)),
            blank,
            "an end tagged {} gives {pixel:?}",
            tag.name()
        );
    }
}

/// The invariant `(L_z, K)` chart at its defaults: the configuration and masses frozen at 0, `K_max = 1`, `γ_K = 3/2`.
fn invariant() -> InvariantLK<f64> {
    InvariantLK(Invariant {
        z_cfg: [0.0, 0.0],
        z_mass: [0.0, 0.0],
        k_max: 1.0,
        gamma_k: 1.5,
    })
}

/// The invariant chart's leaves, with its tag kept or dropped.
struct InvariantLeaves {
    chart: ChartLeaves<InvariantLK<f64>, Defaults>,
    keep: bool,
}

impl Leaves for InvariantLeaves {
    fn decode(&self, view: &View, p: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        let mut d = match self.chart.decode(view, p) {
            Err(PathError::WrongMap) => Latent::new().decode(view, p)?,
            other => other?,
        };
        if !self.keep {
            d.degenerate = None;
        }
        Ok(d)
    }
}

/// The pixel at `s = ½` of the blend from the invariant chart's view centred at the chart point `centre` to plane B.
fn invariant_mid(centre: [f64; 2], keep: bool) -> (Decoded<f64>, Pixel) {
    let mut sim = sim_of(&plane_a());
    sim.chart.window = Window {
        centre,
        half: [1e-3, 1e-3],
    };
    let a = View::kernel(&invariant(), sim).expect("a registry chart");
    let leaves = InvariantLeaves {
        chart: ChartLeaves::new(invariant()).unwrap(),
        keep: true,
    };
    let own = leaves.decode(&a, [0.5, 0.5]).unwrap();
    let blend = Blend::new(a, latent(&plane_b()), BlendPath::BlockLine, 0.5).unwrap();
    let pixel = blend
        .decode(&InvariantLeaves { keep, ..leaves }, [0.5, 0.5])
        .unwrap();
    (own, pixel)
}

fn check_invariant_arms(keep: bool) {
    let (own, pixel) = invariant_mid([0.5, 3e205], keep);
    assert_eq!(
        own.degenerate,
        Some(Degenerate::InvariantInfeasible),
        "the fixture is not tagged"
    );
    assert!(
        own.ic.p.iter().flatten().all(|x| *x == 0.0),
        "the overflow arm has momenta"
    );
    assert!(
        matches!(pixel, Pixel::Blank(_)),
        "the overflow arm blends: {pixel:?}"
    );
    let (own, pixel) = invariant_mid([1.5, 0.1], keep);
    assert_eq!(
        own.degenerate,
        Some(Degenerate::InvariantInfeasible),
        "the rotation fixture is not tagged"
    );
    assert!(
        own.ic.p.iter().flatten().any(|x| *x != 0.0),
        "the rotation arm has no momenta"
    );
    assert!(
        matches!(pixel, Pixel::State { .. }),
        "the rotation arm is blank"
    );
}

#[test]
fn blend_chart_tagged_end() {
    check_tagged(true);
    check_invariant_arms(true);
}

mod invariant_control {
    use super::*;

    negative_control!(
        blend_chart_tagged_end,
        "the invariant chart's tag dropped on its way into the blend",
        expected = "the overflow arm blends",
        check_invariant_arms(false)
    );
}

negative_control!(
    blend_chart_tagged_end,
    "the tag dropped on its way into the blend",
    expected = "an end tagged M01_TINY gives",
    check_tagged(false)
);

// ── blend_chart_retime_refused ─────────────────────────────────────────────────────────────────────────────────────
//
// Pass: a blend segment, and the pan of a blend view, carrying a retime table (not built) refuse their interior frame
// with `NotBuilt("retiming")`, as every other family does. Control: the segment without the table, which shows a frame.

fn check_retime(retime: Option<u64>) {
    let (a, b) = (latent(&plane_a()), latent(&plane_b()));
    let timing = engine::path::Timing {
        retime,
        ..Default::default()
    };
    let blend = Segment::blend(&a, &b, BlendPath::BlockLine).with_timing(timing.clone());
    let view = View::blend(Blend::new(a, b, BlendPath::BlockLine, 0.5).unwrap());
    let pan = Segment::pan(&view, 0.1, 0.0).unwrap().with_timing(timing);
    for (what, seg) in [("the blend", blend), ("the blend view's pan", pan)] {
        assert_eq!(
            seg.frame(0.5).err(),
            Some(PathError::NotBuilt("retiming")),
            "{what} does not refuse a retime table"
        );
    }
}

#[test]
fn blend_chart_retime_refused() {
    check_retime(Some(7));
}

negative_control!(
    blend_chart_retime_refused,
    "the segments without the retime table",
    expected = "the blend does not refuse a retime table",
    check_retime(None)
);

// ── blend_chart_collision_disc ─────────────────────────────────────────────────────────────────────────────────────
//
// Ends: the shape sphere about the collision of bodies 0 and 1, `n₀ = (−1, 0, 0)`, seen through two small windows either
// side of it, so the great circle between the ends' pixels passes through the collision point near the frame's centre
// at `s = ½`. Pass, on a 33² grid at `s = ½`: every pixel a state, finite and untagged (never blank, never
// `decode_failed`); every pixel whose least pair separation is below `0.9·r_coll` a collision at `t = 0` (no step taken),
// its `dmin_pair` the closest pair and its `d_min` that separation to f16's half-ulp (floored at f16's least subnormal); every pixel above `1.1·r_coll`
// not a collision at `t = 0`; the centre pixel inside, and at least 5 pixels and at most half the grid inside (a small
// disc). The band between is reported, not asserted: the detector reads `d²` quantised.

const R_COLL: f64 = 1e-3;

fn uniforms() -> WrapperUniforms<f64> {
    let law = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: 0.01,
        horizon_steps: 4,
        n_max: law.n_max,
        thresholds: thresholds(&law).expect("the default substep law generates"),
    }
}

/// The disc's frame: the blend of the two windows at `s = ½`.
fn disc_blend() -> Blend {
    let window = |x: f64| Window {
        centre: [x, 0.5],
        half: [1.6e-3, 1.6e-3],
    };
    Blend::new(
        sphere_view(window(0.4)),
        sphere_view(window(0.6)),
        BlendPath::BlockLine,
        0.5,
    )
    .unwrap()
}

/// The pair separations `(d₀, d₁, d₂)`, pair `k` the one opposite body `k` (`PairGeom`'s order).
fn separations(ic: &Ic<f64>) -> [f64; 3] {
    let r = ic.r;
    let d = |i: usize, j: usize| ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt();
    [d(1, 2), d(2, 0), d(0, 1)]
}

/// The disc's pixels, by the chart.
fn disc_pixels() -> Vec<Pixel> {
    let blend = disc_blend();
    let l = leaves();
    grid(33)
        .into_iter()
        .map(|p| blend.decode(&l, p).unwrap())
        .collect()
}

fn check_disc(pixels: Vec<Pixel>) {
    let n = 33;
    let (mut inside, mut band) = (0, 0);
    for (k, pixel) in pixels.iter().enumerate() {
        let Pixel::State { decoded, .. } = pixel else {
            panic!("a pixel of the collision disc is blank: {k}")
        };
        let ic = decoded.ic;
        assert!(
            ic.m.iter()
                .chain(ic.r.iter().flatten())
                .chain(ic.p.iter().flatten())
                .all(|x| x.is_finite())
                && decoded.degenerate.is_none(),
            "pixel {k} is not a finite untagged state"
        );
        let d = separations(&ic);
        let (pair, least) = (0..3).fold(
            (0, f64::INFINITY),
            |b, i| if d[i] < b.1 { (i, d[i]) } else { b },
        );
        let out =
            integrate_detecting::<Kdk, f64>(&ic, &uniforms(), CollisionRadius::from_r_coll(R_COLL));
        // At `t = 0`: labelled collision with no step taken (a collision in the first macro-step also ends at
        // `t_end_step = 0`, but after substeps).
        let at_t0 = sd_state(out.packed_a) == STATE_COLLISION
            && tm_t_end_step(out.times) == 0
            && out.total_substeps == 0;
        if least < 0.9 * R_COLL {
            inside += 1;
            assert!(
                at_t0,
                "pixel {k}, least separation {least:e}, is not a collision at t = 0"
            );
            assert_eq!(
                sd_dmin_pair(out.packed_a) as usize,
                pair,
                "pixel {k} records another pair"
            );
            let stored = pa_d_min(out.packed_a) as f64;
            // The latch floors a separation below f16's least subnormal, 2⁻²⁴, to it (the latch's oracle).
            let want = least.max(2f64.powi(-24));
            assert!(
                (stored - want).abs() <= want * 2f64.powi(-11) + 2f64.powi(-25),
                "pixel {k} records d_min {stored:e}, not its least separation {least:e}"
            );
        } else if least > 1.1 * R_COLL {
            assert!(
                !at_t0,
                "pixel {k}, least separation {least:e}, collides at t = 0"
            );
        } else {
            band += 1;
        }
    }
    let centre = (n / 2) * n + n / 2;
    let Pixel::State { decoded, .. } = &pixels[centre] else {
        unreachable!()
    };
    let least = separations(&decoded.ic)
        .into_iter()
        .fold(f64::INFINITY, f64::min);
    assert!(
        least < 0.9 * R_COLL,
        "the centre pixel is not in the disc: {least:e}"
    );
    assert!(
        (5..=n * n / 2).contains(&inside),
        "the disc is not small: {inside} of {}",
        n * n
    );
    println!(
        "blend_chart_collision_disc: {inside} of {} pixels inside 0.9·r_coll, each a collision at t = 0; {band} in the \
         band to 1.1·r_coll, not asserted; centre least separation {least:.3e}",
        n * n
    );
}

#[test]
fn blend_chart_collision_disc() {
    check_disc(disc_pixels());
}

/// The disc blanked, as a blend that refused the collision pole would leave it.
#[cfg(feature = "controls")]
fn blanked() -> Vec<Pixel> {
    disc_pixels()
        .into_iter()
        .map(|p| match &p {
            Pixel::State { decoded, .. }
                if separations(&decoded.ic)
                    .into_iter()
                    .fold(f64::INFINITY, f64::min)
                    < R_COLL =>
            {
                Pixel::Blank(engine::path::blend::Blank {
                    end: End::A,
                    why: engine::path::blend::NoState::NonFinite,
                })
            }
            _ => p,
        })
        .collect()
}

negative_control!(
    blend_chart_collision_disc,
    "the collision disc blanked",
    expected = "a pixel of the collision disc is blank",
    check_disc(blanked())
);

// ── blend_chart_m01_arm1, blend_chart_m01_arm2 ─────────────────────────────────────────────────────────────────────
//
// A map that sets its masses directly: each end one simplex point at every pixel, through the shared entry for a chart
// that sets masses (its `M01_TINY` test applies), a fixed shape, at rest. Pass, on each arm: both ends valid, their
// decodes untagged and the blend's ends the same; at the midpoint the plain blend's masses would raise `M01_TINY`, and
// the blend's pixel is a state, finite and untagged, flagged held, with `M₀₁ ≥ ε_M01` and within four ulps of it, the
// blended `m₁/m₀` kept to `1e-12`, and its shape and momenta the plain blend's to `1e-12`; along `s = k/64` the held
// `M₀₁` is `max(plain M₀₁, ε_M01)` to four ulps of `ε_M01` (continuous, equal to the plain blend where the dip begins
// and ends). Each control: no pixel along `s = k/64` held or tagged.

/// A shape for the direct map: off every collision.
const SHAPE: [f64; 3] = [0.3, -0.4, 0.866_025_403_784_438_6];

/// The direct map's decode of the simplex point `m` (normalised): mass, the shape [`SHAPE`] placed at `I = 1`, rest.
fn direct(m: [f64; 3]) -> Decoded<f64> {
    let sum = m[0] + m[1] + m[2];
    let masses = mass::from_masses(m.map(|x| x / sum));
    let norm = SHAPE.iter().map(|x| x * x).sum::<f64>().sqrt();
    let config = place(SHAPE.map(|x| x / norm), 1.0, 0.0, &masses);
    let zero = [[0.0; 2]; 3];
    let ic = canon::canonicalise(&Ic {
        m: masses.m,
        r: config.r,
        p: zero,
    })
    .ic;
    Decoded {
        ic,
        masses,
        config,
        momentum: Momentum {
            q: [0.0; 4],
            p_rho: [0.0; 2],
            p_lambda: [0.0; 2],
            p: zero,
        },
        descriptor: descriptor::describe_decoded(&ic, &config),
        degenerate: masses.degenerate,
    }
}

/// The direct map's leaves: the end whose plane's `z₀[0]` is 0 shows `a`, 1 shows `b`.
struct Direct {
    a: [f64; 3],
    b: [f64; 3],
}

impl Leaves for Direct {
    fn decode(&self, view: &View, _: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        match view.sim.plane.z0[0] {
            0.0 => Ok(direct(self.a)),
            1.0 => Ok(direct(self.b)),
            _ => Err(PathError::WrongMap),
        }
    }
}

fn direct_ends() -> (View, View) {
    let mut a = plane_a();
    a.z0[0] = 0.0;
    let mut b = plane_a();
    b.z0[0] = 1.0;
    (latent(&a), latent(&b))
}

/// The pixel at `s`, as the blend shows it.
fn held(blend: &Blend, leaves: &Direct, s: f64) -> Pixel {
    blend.at(s).unwrap().decode(leaves, [0.5, 0.5]).unwrap()
}

/// The plain blend's masses at `s`: no hold.
fn plain(blend: &Blend, leaves: &Direct, s: f64) -> (Blocks, Masses<f64>) {
    let Forward::Point { blocks, .. } = blend.at(s).unwrap().forward(leaves, [0.5, 0.5]).unwrap()
    else {
        panic!("a blank")
    };
    (blocks, masses_of(blocks.mu))
}

/// The pixel at `s` with the plain blend's masses, as a blend without the hold would show it.
#[cfg(feature = "controls")]
fn unheld(blend: &Blend, leaves: &Direct, s: f64) -> Pixel {
    let (blocks, masses) = plain(blend, leaves, s);
    let config = place(blocks.n, 1.0, 0.0, &masses);
    let p = kernel::decode::momentum::to_particles(&masses, blocks.p_rho, blocks.p_lambda);
    let ic = canon::canonicalise(&Ic {
        m: masses.m,
        r: config.r,
        p,
    })
    .ic;
    Pixel::State {
        decoded: Box::new(Decoded {
            ic,
            masses,
            config,
            momentum: Momentum {
                q: [0.0; 4],
                p_rho: blocks.p_rho,
                p_lambda: blocks.p_lambda,
                p,
            },
            descriptor: descriptor::describe_decoded(&ic, &config),
            degenerate: masses.degenerate,
        }),
        flags: Flags::default(),
    }
}

type Show = fn(&Blend, &Direct, f64) -> Pixel;

fn check_ends_valid(blend: &Blend, leaves: &Direct) {
    for (s, m) in [(0.0, leaves.a), (1.0, leaves.b)] {
        let own = direct(m);
        assert!(
            own.degenerate.is_none(),
            "end at s = {s} is not valid: {:?}",
            own.masses
        );
        let shown = held(blend, leaves, s);
        assert_eq!(
            words(state(&shown).0),
            words(&own),
            "the blend's end at s = {s} is not the end's own"
        );
    }
}

/// The arm: the midpoint held, and the hold continuous along the path.
fn check_dip(leaves: &Direct, show: Show) {
    let (a, b) = direct_ends();
    let blend = Blend::new(a, b, BlendPath::BlockLine, 0.5).unwrap();
    check_ends_valid(&blend, leaves);
    let eps = mass::m01_epsilon::<f64>();
    let ulps = |x: f64| (x - eps).abs() / (eps.next_up() - eps);
    let (blocks, masses) = plain(&blend, leaves, 0.5);
    assert!(
        masses.degenerate.is_some(),
        "the fixture's midpoint does not dip: {masses:?}"
    );
    let mid = show(&blend, leaves, 0.5);
    let (decoded, flags) = state(&mid);
    assert!(
        decoded.degenerate.is_none() && decoded.masses.degenerate.is_none(),
        "the midpoint is tagged M01_TINY"
    );
    assert!(flags.m01_held, "the midpoint is not flagged held");
    let m = decoded.masses.m;
    let m01 = decoded.masses.m01;
    assert!(
        m01 >= eps && ulps(m01) <= 4.0,
        "the held M₀₁ {m01:e} is not at the guard"
    );
    let ratio = libm::exp(blocks.mu[0]);
    assert!(
        ((m[1] / m[0]) - ratio).abs() <= 1e-12 * ratio,
        "the held m₁/m₀ {} is not the blend's {ratio}",
        m[1] / m[0]
    );
    let got = Blocks::of(&decoded.ic);
    for (x, y) in got
        .n
        .iter()
        .chain(&got.p_rho)
        .chain(&got.p_lambda)
        .zip(blocks.n.iter().chain(&blocks.p_rho).chain(&blocks.p_lambda))
    {
        assert!(
            (x - y).abs() <= 1e-12,
            "the hold moved the shape or momenta: {x} against {y}"
        );
    }
    let mut lifted = 0;
    for k in 1..64 {
        let s = k as f64 / 64.0;
        let (_, plain_masses) = plain(&blend, leaves, s);
        let pixel = show(&blend, leaves, s);
        let (d, f) = state(&pixel);
        assert!(
            d.degenerate.is_none(),
            "the pixel at s = {s} is tagged M01_TINY"
        );
        let want = plain_masses.m01.max(eps);
        let off = (d.masses.m01 - want).abs() / (eps.next_up() - eps);
        assert!(
            off <= 4.0,
            "the held M₀₁ at s = {s} is {:e}, not max(plain, ε) = {want:e}",
            d.masses.m01
        );
        lifted += f.m01_held as usize;
    }
    println!(
        "the dip: plain midpoint M₀₁ {:e}, held {m01:e}; {lifted} of 63 shares held",
        masses.m01
    );
}

/// A control: no pixel along the path held or tagged.
fn check_no_dip(leaves: &Direct, show: Show) {
    let (a, b) = direct_ends();
    let blend = Blend::new(a, b, BlendPath::BlockLine, 0.5).unwrap();
    check_ends_valid(&blend, leaves);
    let mut least = f64::INFINITY;
    for k in 1..64 {
        let s = k as f64 / 64.0;
        let pixel = show(&blend, leaves, s);
        let (d, f) = state(&pixel);
        assert!(
            d.degenerate.is_none() && !f.m01_held,
            "the control's pixel at s = {s} is held or tagged"
        );
        least = least.min(d.masses.m01);
    }
    println!("the control: least M₀₁ along the path {least:e}");
}

/// Arm (1): end A `μ = (−L, 0)`, end B `μ = (L, L)`, `μ_k = ln(m_k/m₀)`, as masses.
fn arm1(l: f64) -> Direct {
    Direct {
        a: [1.0, libm::exp(-l), 1.0],
        b: [1.0, libm::exp(l), libm::exp(l)],
    }
}

fn arm1_checks(show: Show) {
    check_dip(&arm1(30.0), show);
    check_no_dip(&arm1(10.0), show);
}

#[test]
fn blend_chart_m01_arm1() {
    arm1_checks(held);
}

negative_control!(
    blend_chart_m01_arm1,
    "the blend without the hold",
    expected = "the midpoint is tagged M01_TINY",
    arm1_checks(unheld)
);

fn arm2_checks(show: Show) {
    check_dip(
        &Direct {
            a: [1.0, 0.01, 1e5],
            b: [0.01, 1.0, 1e5],
        },
        show,
    );
    // q = 1: g constant along the path in exact arithmetic.
    check_no_dip(
        &Direct {
            a: [0.505, 0.505, 1e5],
            b: [0.505, 0.505, 1e5],
        },
        show,
    );
    // The same path with the light pair lifted: M₀₁ = 1.01·10⁻⁴ at the ends, 2.0·10⁻⁵ at the midpoint.
    check_no_dip(
        &Direct {
            a: [1.0, 0.01, 1e4],
            b: [0.01, 1.0, 1e4],
        },
        show,
    );
}

#[test]
fn blend_chart_m01_arm2() {
    arm2_checks(held);
}

negative_control!(
    blend_chart_m01_arm2,
    "the blend without the hold",
    expected = "the midpoint is tagged M01_TINY",
    arm2_checks(unheld)
);

// ── blend_chart_fold_indicator ─────────────────────────────────────────────────────────────────────────────────────
//
// Pass: on a 24² probe grid at `s = ½`, the blend of a latent view and its mirror image (`q₁ → −q₁`, so pixel `(u, v)`
// of B is pixel `(1 − u, v)` of A, and the frame is folded along `u = ½`) reads a folded share of at least 0.25; the
// blend of the same view and its pan by 0.1 reads 0.

fn check_folds(read: fn(engine::path::blend::Folds) -> f64) {
    let a = latent(&plane_a());
    let mut mirror = plane_a();
    mirror.q1 = mirror.q1.map(|x| -x);
    let l = leaves();
    let folded = Blend::new(a.clone(), latent(&mirror), BlendPath::BlockLine, 0.5).unwrap();
    let folds = fold_indicator(&folded, &l, 24).unwrap();
    let share = read(folds.clone());
    println!(
        "blend_chart_fold_indicator: the mirror fixture's folded share {share:.3}, undefined {:.3}",
        folds.undefined
    );
    assert!(share >= 0.25, "the mirror fixture reads no fold: {share}");
    let panned = Segment::pan(&a, 0.1, 0.0).unwrap().b().clone();
    let flat = Blend::new(a, panned, BlendPath::BlockLine, 0.5).unwrap();
    let flat = read(fold_indicator(&flat, &l, 24).unwrap());
    assert_eq!(flat, 0.0, "the unfolded blend reads a fold");
}

#[test]
fn blend_chart_fold_indicator() {
    check_folds(|f| f.folded);
}

/// The unsigned indicator: `|σ|`, which sees no side.
#[cfg(feature = "controls")]
fn unsigned(f: engine::path::blend::Folds) -> f64 {
    let defined: Vec<i8> = f
        .sigma
        .iter()
        .filter(|s| **s != 0)
        .map(|s| s.abs())
        .collect();
    let r = f.sigma[f.reference].abs();
    defined.iter().filter(|s| **s != r).count() as f64 / defined.len().max(1) as f64
}

negative_control!(
    blend_chart_fold_indicator,
    "the unsigned indicator",
    expected = "the mirror fixture reads no fold",
    check_folds(unsigned)
);

// ── blend_chart_held_energy_miss (measurement, reported, not a gate) ───────────────────────────────────────────────
//
// Both ends hold `E* = −0.1` (the decoder's `η_E`), two latent planes at equal masses (their mass block 0, where most
// shapes have `U < E*`); the state blend at `s = ½` holds nothing, and its
// miss `|E − E*|` over the pixels feasible at both ends is reported, median and worst, against § 17.6's 0.054 median
// and 2.1 worst. Mid-morph frames never claim held values (R-850). Asserted only that the ends hold `E*` to `1e-9`, so
// the miss is the blend's own, and that some pixels are measured.

const E_STAR: f64 = -0.1;

/// The latent map's leaves with `η_E` toward the target, or with none (the control's ends, which hold nothing).
struct Held(Option<f64>);

impl Leaves for Held {
    fn decode(&self, view: &View, [s, t]: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        let plane = &view.sim.plane;
        let chart = kernel::chart::Latent {
            z0: plane.z0,
            q1: plane.q1,
            q2: plane.q2,
        };
        Ok(energy_norm::decode_chart::<Defaults, f64, _>(&chart, s, t, self.0).0)
    }
}

fn energy_of(ic: &Ic<f64>) -> f64 {
    energy(&ic.m, &ic.p, &PairGeom::new(&ic.r))
}

/// `plane` at equal masses: its mass block 0 in `z₀`, `q₁` and `q₂`.
fn equal_masses(mut plane: Plane) -> Plane {
    for v in [&mut plane.z0, &mut plane.q1, &mut plane.q2] {
        v[6] = 0.0;
        v[7] = 0.0;
    }
    plane
}

#[test]
fn blend_chart_held_energy_miss() {
    measure_energy_miss(&Held(Some(E_STAR)));
}

negative_control!(
    blend_chart_held_energy_miss,
    "the ends decoded with no energy target, so they hold nothing",
    expected = "an end does not hold E*",
    measure_energy_miss(&Held(None))
);

fn measure_energy_miss(held: &Held) {
    let (a, b) = (
        latent(&equal_masses(plane_a())),
        latent(&equal_masses(plane_b())),
    );
    let blend = Blend::new(a.clone(), b.clone(), BlendPath::BlockLine, 0.5).unwrap();
    let mut miss = Vec::new();
    for p in grid(32) {
        let (da, db) = (held.decode(&a, p).unwrap(), held.decode(&b, p).unwrap());
        if da.degenerate.is_some() || db.degenerate.is_some() {
            continue;
        }
        for d in [&da, &db] {
            assert!(
                (energy_of(&d.ic) - E_STAR).abs() < 1e-9,
                "an end does not hold E*"
            );
        }
        let pixel = blend.decode(held, p).unwrap();
        miss.push((energy_of(&state(&pixel).0.ic) - E_STAR).abs());
    }
    assert!(!miss.is_empty(), "no pixel is feasible at both ends");
    miss.sort_by(f64::total_cmp);
    println!(
        "blend_chart_held_energy_miss (measurement, not a gate): |E − E*| at s = ½ over {} pixels feasible at both \
         ends: median {:.4}, worst {:.4} (design § 17.6: 0.054 median, 2.1 worst)",
        miss.len(),
        miss[miss.len() / 2],
        miss[miss.len() - 1]
    );
}

// ── blend_chart_lifted_jump (measurement, reported, not a gate) ────────────────────────────────────────────────────
//
// The re-plan's open item O5: the jump in `n̂` between neighbouring pixels at mid-morph on the per-pixel shortest arc,
// where end B's pixels are near the antipodes of end A's (the shape sphere's exponential map out to `π`, end B's
// window about the antipode). Reported: the largest and median neighbour jump at `s = ½` and the pixels on the tie.
// Asserted only that the fixture reaches the antipodal tie, so the measurement is of the case O5 names.

#[test]
fn blend_chart_lifted_jump() {
    measure_lifted_jump(1.0);
}

negative_control!(
    blend_chart_lifted_jump,
    "end B's window moved off the antipode, onto end A's",
    expected = "the fixture reaches no antipodal tie",
    measure_lifted_jump(0.5)
);

/// The measurement with end B's window centred at `(b, ½)` of the chart (1 the antipode of end A's centre).
fn measure_lifted_jump(b: f64) {
    let mut chart = sphere();
    chart.scale = std::f64::consts::PI;
    let l = (
        Latent::new(),
        ChartLeaves::<_, Defaults>::new(chart).expect("a registry chart"),
    );
    let window = |x: f64| Window {
        centre: [x, 0.5],
        half: [0.02, 0.02],
    };
    let blend = Blend::new(
        sphere_view(window(0.5)),
        sphere_view(window(b)),
        BlendPath::BlockLine,
        0.5,
    )
    .unwrap();
    let n = 33;
    let mut ties = 0;
    let points: Vec<[f64; 3]> = grid(n)
        .into_iter()
        .map(|p| match blend.forward(&l, p).unwrap() {
            Forward::Point {
                blocks,
                antipodal_tie,
            } => {
                ties += antipodal_tie as usize;
                blocks.n
            }
            Forward::Blank(b) => panic!("a blank: {b:?}"),
        })
        .collect();
    let angle = |a: [f64; 3], b: [f64; 3]| {
        let c = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f64>();
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        libm::atan2(cross.iter().map(|x| x * x).sum::<f64>().sqrt(), c)
    };
    let mut jumps = Vec::new();
    for j in 0..n {
        for i in 0..n {
            if i + 1 < n {
                jumps.push(angle(points[j * n + i], points[j * n + i + 1]));
            }
            if j + 1 < n {
                jumps.push(angle(points[j * n + i], points[(j + 1) * n + i]));
            }
        }
    }
    jumps.sort_by(f64::total_cmp);
    assert!(ties > 0, "the fixture reaches no antipodal tie");
    println!(
        "blend_chart_lifted_jump (measurement, not a gate): neighbour jump in n̂ at s = ½ on the per-pixel shortest arc: \
         largest {:.4} rad, median {:.4} rad; {ties} pixels on the antipodal tie (re-plan O5's prototype: 3.14 rad \
         against 0.002 lifted)",
        jumps[jumps.len() - 1],
        jumps[jumps.len() / 2]
    );
}
