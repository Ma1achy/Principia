//! Paths under a lock (TASK-M2-65; design § 17.8; the chart-space model's § 5):
//!
//! - `path_lock_centre_holds`: under a lock whose target holds the anchor, the state at the pin is the stored anchor,
//!   every word, on every frame, ends included, affine and nonlinear, `δ = 0` and `δ ≠ 0`, the pin off the centre and
//!   at it. Control: the select removed.
//! - `path_lock_centre_area`: the area factor of the frame's tangent at the pin stays within 0.5 of the ends' on swapped
//!   and orthogonal planes. Control: offsets blended linearly. Reported: the free block line's area at `s = ½`.
//! - `path_lock_centre_pin_record`: the pin's screen position stored in the lock's record and read back; an old lock on
//!   a degenerate view takes the centre, labelled `assumed`; snap-back returns the stored anchor. Control: the probe
//!   read at `(½, ½)` with an off-centre pin.
//! - `path_lock_centre_drift`: a measurement, the pinned state's drift without the select, the number the lock's
//!   tolerance is proposed from; it asserts only that each drift is within the tolerance in force.
//!
//! Each criterion is written before its run; `LOCK_TOLERANCE` (the ledger's `lock_tolerance`) is the one constant they
//! read.

use engine::contract::sim_config::{Latent, Plane, SimConfig, Window};
use engine::lookup::Tolerances;
use engine::path::anchor::{self, Anchor, LockedEnd, LOCK_TOLERANCE};
use engine::path::blend::{Blend, Blocks, LatentLeaves, Leaves, Pixel};
use engine::path::{BlendPath, FrameRecord, Map, PathError, Pin, Segment, View};
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::Mixed;
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::Defaults;
use validation::negative_control;
use validation::roundtrip::phys;

mod path_shim;
use path_shim::sim_of;

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

/// A unit control direction times `k`.
fn e(i: usize, k: f64) -> Latent {
    let mut v = [0.0; 8];
    v[i] = k;
    v
}

/// The frozen controls of the mixed chart's views.
const FROZEN: Latent = [0.4, -0.3, 0.2, -0.1, 0.3, 0.15, 0.0, -0.4];

/// The mixed chart `(z_μ1, α)` at the hidden controls `z0`.
fn mixed(z0: Latent) -> Mixed<ZMu1, Alpha, f64> {
    Mixed::new(z0, [-1.0, 1.0], [0.25, 1.3])
}

/// A view of the mixed chart through `window`, its hidden controls `FROZEN`.
fn mixed_view(window: Window) -> View {
    let mut sim: SimConfig = sim_of(&Plane {
        z0: FROZEN,
        ..plane_a()
    });
    sim.chart.window = window;
    View::kernel(&mixed(FROZEN), sim).expect("the mixed chart is a registry chart")
}

/// The mixed chart's leaves, the chart rebuilt at each view's own hidden controls, `z₀`, so a slider's move shows.
struct MixedLeaves;

impl Leaves for MixedLeaves {
    fn decode(&self, view: &View, [s, t]: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        if view.map != Map::Kernel(engine::path::KernelMap::of(&mixed(FROZEN))?) {
            return Err(PathError::WrongMap);
        }
        // The test's own convention: a view whose horizontal mapping reads `linear` is of the mixed chart with its axes
        // swapped, `(α, z_μ1)`, the same states in other chart coordinates; a view names its kernel map by registry
        // index alone, and both are the registry's `Mixed`.
        let [u, v] = view.sim.chart.window.at(s, t);
        match view.sim.chart.mapping[0] {
            engine::contract::sim_config::Mapping::Linear => {
                decode_on(view, swapped(view.sim.plane.z0), u, v)
            }
            engine::contract::sim_config::Mapping::Warped => {
                decode_on(view, mixed(view.sim.plane.z0), u, v)
            }
        }
    }
}

/// The mixed chart `(α, z_μ1)`, the axes of [`mixed`] swapped.
fn swapped(z0: Latent) -> Mixed<Alpha, ZMu1, f64> {
    Mixed::new(z0, [0.25, 1.3], [-1.0, 1.0])
}

/// `chart`'s decode at `(u, v)` as `view` shows it: under its own tilt about its pin where it has one.
fn decode_on<C>(view: &View, chart: C, u: f64, v: f64) -> Result<Decoded<f64>, PathError>
where
    C: kernel::chart::tilt::Tiltable<Param = f64>,
{
    match view.tilt.is_some_and(|t| !t.spec().untilted()) {
        true => Ok(kernel::chart::Chart::decode::<Defaults, f64>(
            &view.tilted(chart)?,
            u,
            v,
        )),
        false => Ok(kernel::chart::Chart::decode::<Defaults, f64>(&chart, u, v)),
    }
}

/// The leaves of the tests' charts: the latent map, the mixed chart, the `(L_z, K)` chart and the exponential shape
/// sphere.
type AllLeaves = (
    LatentLeaves<Defaults>,
    (
        MixedLeaves,
        (
            engine::path::blend::ChartLeaves<kernel::chart::InvariantLK<f64>, Defaults>,
            engine::path::blend::ChartLeaves<kernel::chart::ShapeSphere<f64>, Defaults>,
        ),
    ),
);

fn leaves() -> AllLeaves {
    (
        LatentLeaves::new(),
        (
            MixedLeaves,
            (
                engine::path::blend::ChartLeaves::new(invariant_lk()).expect("a registry chart"),
                engine::path::blend::ChartLeaves::new(exp_sphere()).expect("a registry chart"),
            ),
        ),
    )
}

/// The `(L_z, K)` chart of the lock's tests, at a frozen configuration and masses.
fn invariant_lk() -> kernel::chart::InvariantLK<f64> {
    kernel::chart::InvariantLK(kernel::chart::invariant::Invariant {
        z_cfg: [-0.2, 0.5],
        z_mass: [0.1, 0.3],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

/// The exponential shape sphere holding the masses and momenta of the affine lock's pinned latent point (`plane_a` at
/// `(0.3, 0.65)`), centred off its shape: so it holds that state, at a chart point its re-solve must find.
fn exp_sphere() -> kernel::chart::ShapeSphere<f64> {
    let z = engine::lock::point(&plane_a(), 0.3, 0.65);
    let n = [0.36, -0.48, 0.8];
    let e1 = [0.8, 0.6, 0.0];
    let e2 = [
        n[1] * e1[2] - n[2] * e1[1],
        n[2] * e1[0] - n[0] * e1[2],
        n[0] * e1[1] - n[1] * e1[0],
    ];
    kernel::chart::ShapeSphere {
        map: kernel::chart::ShapeMap::Exponential,
        n0: n,
        e1,
        e2,
        scale: 2.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [z[6], z[7]],
        z_mom: [z[2], z[3], z[4], z[5]],
    }
}

/// The latent view of `plane`, locked at `(s, t)` and recorded.
fn locked_latent(plane: &Plane, (s, t): (f64, f64)) -> View {
    let view = View::latent(sim_of(plane));
    let locked = Segment::lock(&view, s, t).expect("a lock").b().clone();
    anchor::record(&locked, &leaves()).expect("a recorded anchor")
}

/// The mixed view through `window`, locked at `(s, t)` on the chart and recorded.
fn locked_mixed(window: Window, (s, t): (f64, f64)) -> View {
    let view = mixed_view(window);
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let (seg, _) =
        Segment::lock_on_chart::<Defaults, _, _>(&view, &mixed(FROZEN), (s, t), phys, &tol)
            .expect("a lock on the mixed chart");
    anchor::record(seg.b(), &leaves()).expect("a recorded anchor")
}

/// The mixed chart's home window and one panned and zoomed from it.
fn home() -> Window {
    Window {
        centre: [0.1, 0.7],
        half: [0.6, 0.3],
    }
}

fn moved() -> Window {
    Window {
        centre: [-0.25, 0.85],
        half: [0.45, 0.2],
    }
}

/// A pixel's state; panics on a blank.
fn state(pixel: Pixel) -> Decoded<f64> {
    match pixel {
        Pixel::State { decoded, .. } => *decoded,
        Pixel::Blank(b) => panic!("a blank pixel: {b:?}"),
    }
}

/// The words of a state, as the anchor stores them.
fn words(ic: &Ic<f64>) -> [u64; 15] {
    Anchor::of(ic, false).words
}

/// The pin's address of a locked view.
fn pin(view: &View) -> [f64; 2] {
    anchor::pin_address(view).expect("a locked view").0
}

/// The frames the tests read: `s = k/16`, ends included.
fn shares() -> impl Iterator<Item = f64> {
    (0..=16).map(|k| k as f64 / 16.0)
}

// ── path_lock_centre_holds ─────────────────────────────────────────────────────────────────────────────────────────
//
// Pass: the target holds the anchor (`LockedEnd::Holds`), and at every frame `s = k/16`, ends included, the state at
// the pin is the stored anchor, all fifteen words bit for bit.

/// One hold pair: `a` locked and recorded, `b` the target as asked.
struct Pair {
    name: &'static str,
    a: View,
    b: View,
}

fn hold_pairs() -> Vec<Pair> {
    let affine = |name, at, plane: Plane| Pair {
        name,
        a: locked_latent(&plane_a(), at),
        b: View::latent(sim_of(&plane)),
    };
    // δ ≠ 0: a slice after the lock, then the lock's record taken again (the pin's current state).
    let sliced = {
        let a = locked_latent(&plane_a(), (0.3, 0.65));
        let b = Segment::slice(&a, &e(5, 1.0), 0.15)
            .expect("a slice")
            .b()
            .clone();
        anchor::record(&b, &leaves()).expect("recorded again")
    };
    let mixed_sliced = {
        let a = locked_mixed(home(), (0.35, 0.6));
        let b = Segment::control(&a, 4, a.sim.plane.z0[4] + 0.2)
            .expect("a slider's move")
            .b()
            .clone();
        anchor::record(&b, &leaves()).expect("recorded again")
    };
    let moved_of = |v: &View| {
        let mut b = v.clone();
        b.sim.chart.window = moved();
        b.pin = None;
        b.anchor = None;
        b.sim.lock.locked = false;
        b
    };
    let invariant = {
        let mut sim = sim_of(&plane_a());
        sim.chart.window = Window::SQUARE;
        let view = View::kernel(&invariant_lk(), sim).expect("a registry chart");
        let tol = Tolerances::register(128.0 * f64::EPSILON);
        let (seg, _) = Segment::lock_on_chart::<Defaults, _, _>(
            &view,
            &invariant_lk(),
            (0.4, 0.55),
            phys,
            &tol,
        )
        .expect("a lock on the (L_z, K) chart");
        anchor::record(seg.b(), &leaves()).expect("a recorded anchor")
    };
    let sphere = {
        let mut sim = sim_of(&plane_a());
        sim.chart.window = Window::SQUARE;
        View::kernel(&exp_sphere(), sim).expect("a registry chart")
    };
    vec![
        Pair {
            name: "an invariant chart to latent (through z_locked, lookup's inverse)",
            a: invariant,
            b: View::latent(sim_of(&plane_b())),
        },
        Pair {
            name: "latent to the exponential shape sphere (through the re-solve)",
            a: locked_latent(&plane_a(), (0.3, 0.65)),
            b: sphere,
        },
        affine("affine, pin at the centre", (0.5, 0.5), plane_b()),
        affine("affine, pin off the centre", (0.3, 0.65), plane_b()),
        Pair {
            name: "affine, δ ≠ 0",
            a: sliced,
            b: View::latent(sim_of(&plane_b())),
        },
        Pair {
            name: "nonlinear to latent",
            a: locked_mixed(home(), (0.35, 0.6)),
            b: View::latent(sim_of(&plane_b())),
        },
        Pair {
            name: "nonlinear to nonlinear, another window",
            b: moved_of(&locked_mixed(home(), (0.35, 0.6))),
            a: locked_mixed(home(), (0.35, 0.6)),
        },
        Pair {
            name: "nonlinear to nonlinear, δ ≠ 0",
            b: moved_of(&mixed_sliced),
            a: mixed_sliced,
        },
    ]
}

/// The segment of a hold pair; panics unless the target holds the anchor.
fn held(p: &Pair) -> Segment {
    let (seg, end) = Segment::locked_blend(&p.a, &p.b, BlendPath::BlockLine, &leaves(), phys, 64)
        .unwrap_or_else(|e| panic!("{}: no locked path: {e:?}", p.name));
    assert_eq!(
        end,
        LockedEnd::Holds,
        "{}: the target does not hold",
        p.name
    );
    seg
}

fn check_holds(p: &Pair, seg: &Segment, frame: impl Fn(View) -> View) {
    let anchor = p.a.anchor.expect("recorded");
    let at = pin(&p.a);
    for s in shares() {
        let view = frame(seg.frame(s).expect("a frame"));
        let shown = state(anchor::state_at(&view, &leaves(), at).expect("a pixel"));
        assert_eq!(
            words(&shown.ic),
            anchor.words,
            "{}: the pin's words differ from the stored anchor at s = {s}",
            p.name
        );
    }
}

#[test]
fn path_lock_centre_holds() {
    for p in hold_pairs() {
        check_holds(&p, &held(&p), |v| v);
    }
}

/// The frame with its lock's offset form `form`.
#[cfg(feature = "controls")]
fn with_form(view: View, form: anchor::OffsetForm) -> View {
    match &view.map {
        Map::Blend(blend) => View::blend((**blend).clone().with_form(form)),
        _ => view,
    }
}

negative_control!(
    path_lock_centre_holds,
    "the select removed: the pin shows the blend's own rounding of the anchor",
    expected = "differ from the stored anchor",
    for p in hold_pairs() {
        let form = anchor::OffsetForm {
            select: false,
            ..Default::default()
        };
        check_holds(&p, &held(&p), |v| with_form(v, form));
    }
);

// ── path_lock_centre_area ──────────────────────────────────────────────────────────────────────────────────────────
//
// Pass: on swapped and on orthogonal planes, at `s = ¼, ½, ¾`, the area factor of the frame's tangent at the pin, `A_s`,
// against the ends' interpolated geometrically, `A₀^{1−s}·A₁^s`, has `|A_s/(A₀^{1−s}A₁^s) − 1| < 0.5`. The tangent is
// read off the blocks by a central difference of step `h = 2⁻¹⁶` in the address, the shape by the log map at the pin.

/// The area factor of `view`'s tangent at `at`.
fn area(view: &View, at: [f64; 2]) -> f64 {
    let h = 2f64.powi(-16);
    let base = Blocks::of(&state(anchor::state_at(view, &leaves(), at).expect("a pixel")).ic);
    let col = |axis: usize| -> [f64; 9] {
        let (mut p, mut m) = (at, at);
        p[axis] += h;
        m[axis] -= h;
        let bp = Blocks::of(&state(anchor::state_at(view, &leaves(), p).expect("a pixel")).ic);
        let bm = Blocks::of(&state(anchor::state_at(view, &leaves(), m).expect("a pixel")).ic);
        let (op, om) = (anchor::log(&base, &bp), anchor::log(&base, &bm));
        std::array::from_fn(|i| (op[i] - om[i]) / (2.0 * h))
    };
    let (u, v) = (col(0), col(1));
    let dot = |a: &[f64; 9], b: &[f64; 9]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    (dot(&u, &u) * dot(&v, &v) - dot(&u, &v) * dot(&u, &v))
        .max(0.0)
        .sqrt()
}

/// The swapped and orthogonal pairs, the pin at the centre and off it.
fn area_pairs() -> Vec<(&'static str, Plane, Plane, (f64, f64))> {
    let z0 = [0.2, 0.9, 0.1, -0.1, 0.05, 0.1, 0.1, -0.1];
    let p = |q1, q2| Plane { z0, q1, q2 };
    vec![
        (
            "swapped",
            p(e(0, 0.6), e(1, 0.6)),
            p(e(1, 0.6), e(0, 0.6)),
            (0.5, 0.5),
        ),
        (
            "swapped, pin off the centre",
            p(e(0, 0.6), e(1, 0.6)),
            p(e(1, 0.6), e(0, 0.6)),
            (0.4, 0.6),
        ),
        (
            "orthogonal",
            p(e(0, 0.6), e(1, 0.6)),
            p(e(2, 0.6), e(3, 0.6)),
            (0.5, 0.5),
        ),
    ]
}

fn check_area(form: impl Fn(View) -> View) -> Vec<(String, f64)> {
    let mut seen = Vec::new();
    for (name, pa, pb, at) in area_pairs() {
        let a = locked_latent(&pa, at);
        let pair = Pair {
            name,
            a,
            b: View::latent(sim_of(&pb)),
        };
        let seg = held(&pair);
        let pin = pin(&pair.a);
        let (a0, a1) = (area(seg.a(), pin), area(seg.b(), pin));
        for s in [0.25, 0.5, 0.75] {
            let view = form(seg.frame(s).expect("a frame"));
            let ratio = area(&view, pin) / (a0.powf(1.0 - s) * a1.powf(s));
            seen.push((format!("{name} at s = {s}"), ratio));
            assert!(
                (ratio - 1.0).abs() < 0.5,
                "{name}: the area factor at the pin is {ratio} of the ends' at s = {s}"
            );
        }
    }
    seen
}

#[test]
fn path_lock_centre_area() {
    for (what, ratio) in check_area(|v| v) {
        println!("path_lock_centre_area: {what}: A_s/(A₀^(1−s)A₁^s) = {ratio:.6}");
    }
    // Reported, not a gate: the free block line's area at s = ½ on the swapped planes, why the offset form exists.
    let (_, pa, pb, _) = area_pairs().remove(0);
    let free = Blend::new(
        View::latent(sim_of(&pa)),
        View::latent(sim_of(&pb)),
        BlendPath::BlockLine,
        0.5,
    )
    .expect("a blend");
    let a0 = area(&View::latent(sim_of(&pa)), [0.5, 0.5]);
    println!(
        "path_lock_centre_area: the free block line on swapped planes at s = ½: {:.6} of the ends'",
        area(&View::blend(free), [0.5, 0.5]) / a0
    );
}

negative_control!(
    path_lock_centre_area,
    "offsets blended linearly: swapped planes squash to no area at s = ½",
    expected = "the area factor at the pin is",
    {
        let form = anchor::OffsetForm {
            transport: false,
            ..Default::default()
        };
        check_area(|v| with_form(v, form));
    }
);

// ── path_lock_centre_pin_record ────────────────────────────────────────────────────────────────────────────────────
//
// Pass: (1) a lock off the centre, recorded, written in a frame record and read back, keeps its pin's address and its
// anchor bit for bit, and its anchor is the state at that address; (2) an old lock with no pin on a degenerate plane
// (`q₂ = q₁`) records its anchor at the centre, labelled `assumed`, and an old lock with no pin on a plane derives its
// address from `z_locked`, not assumed; (3) after a slice, snap-back and recording again give the first anchor's words.

fn check_pin_record(probe: impl Fn(&View) -> [f64; 2]) {
    let a = locked_latent(&plane_a(), (0.3, 0.65));
    let anchor = a.anchor.expect("recorded");
    let record = FrameRecord {
        a: a.clone(),
        b: a.clone(),
        family: engine::path::Family::Cut {
            label: engine::path::CutLabel::Lock,
        },
        s_bits: 0,
        align: None,
        retime_hash: None,
        anchor_words: None,
    };
    let back = FrameRecord::read(&record.to_bytes()).expect("the record reads back");
    assert_eq!(back.a.pin, a.pin, "the pin's address does not read back");
    assert_eq!(back.a.anchor, a.anchor, "the anchor does not read back");
    let shown = state(anchor::state_at(&back.a, &leaves(), probe(&back.a)).expect("a pixel"));
    let raw = state(Pixel::State {
        decoded: Box::new(leaves().decode(&back.a, probe(&back.a)).expect("a decode")),
        flags: Default::default(),
    });
    assert_eq!(
        (words(&shown.ic), words(&raw.ic)),
        (anchor.words, anchor.words),
        "the state at the probe is not the recorded anchor"
    );
    // (2) Old locks: no pin stored.
    let mut degenerate = View::latent(sim_of(&Plane {
        q2: plane_a().q1,
        ..plane_a()
    }));
    degenerate.sim.lock.locked = true;
    degenerate.sim.lock.z_locked = plane_a().z0;
    let r = anchor::record(&degenerate, &leaves()).expect("an old lock recorded");
    assert_eq!(
        (r.pin, r.anchor.map(|a| a.assumed)),
        (Some(Pin::Centre), Some(true)),
        "an old lock on a degenerate view does not take the centre, assumed"
    );
    let mut old = View::latent(sim_of(&plane_a()));
    old.sim.lock.locked = true;
    old.sim.lock.z_locked = engine::lock::point(&plane_a(), 0.3, 0.65);
    let (address, assumed) = anchor::pin_address(&old).expect("locked");
    assert!(
        !assumed && (address[0] - 0.3).abs() < 1e-12 && (address[1] - 0.65).abs() < 1e-12,
        "an old lock's address is not derived from z_locked: {address:?}, assumed {assumed}"
    );
    // (3) Snap-back.
    let sliced = Segment::slice(&a, &e(5, 1.0), 0.15)
        .expect("a slice")
        .b()
        .clone();
    let back = Segment::snap_back(&sliced).expect("snap-back").b().clone();
    let again = anchor::record(&back, &leaves()).expect("recorded again");
    assert_eq!(
        again.anchor.map(|x| x.words),
        Some(anchor.words),
        "snap-back does not return the stored anchor"
    );
}

#[test]
fn path_lock_centre_pin_record() {
    check_pin_record(pin);
}

negative_control!(
    path_lock_centre_pin_record,
    "a probe at (½, ½) with an off-centre pin reads another state",
    expected = "is not the recorded anchor",
    check_pin_record(|_| [0.5, 0.5])
);

// ── path_lock_centre_drift ─────────────────────────────────────────────────────────────────────────────────────────
//
// A measurement. On each hold pair, at every frame `s = k/64`, the pin's state without the select (the offset form's
// own value at the pin, the frame's `block_line(a, B(pin), s)`) against the stored anchor in `‖·‖_phys`, and the target's
// own pin state against it; the worst of each, in units of `ε`, printed. Its assert is that each is within the tolerance
// in force, the measurement's own fixture check.

fn drift(p: &Pair, against: &Anchor) -> (f64, f64) {
    let seg = held(p);
    let at = pin(&p.a);
    let anchor = against.ic();
    let raw = |v: &View| leaves().decode(v, at).expect("a decode").ic;
    let target = phys(&anchor, &raw(seg.b()));
    let mut worst = 0.0f64;
    for k in 1..64 {
        let s = k as f64 / 64.0;
        let view = seg.frame(s).expect("a frame");
        let Map::Blend(blend) = &view.map else {
            panic!("a mid frame is a blend")
        };
        let lock = blend.lock().expect("a locked blend");
        assert!(lock.hold, "{}: a mid frame that does not hold", p.name);
        let (line, _) = engine::path::blend::block_line(
            &p.a.anchor.expect("recorded").blocks(),
            &Blocks::of(&raw(&blend.b)),
            s,
        );
        let (shown, _) = engine::path::blend::state_of(&line);
        worst = worst.max(phys(&anchor, &shown.ic));
    }
    (worst, target)
}

fn check_drift(against: impl Fn(&Pair) -> Anchor) {
    for p in hold_pairs() {
        let (frames, target) = drift(&p, &against(&p));
        println!(
            "path_lock_centre_drift: {}: frames {:.3} ε, target's own pin {:.3} ε",
            p.name,
            frames / f64::EPSILON,
            target / f64::EPSILON
        );
        assert!(
            frames <= LOCK_TOLERANCE && target <= LOCK_TOLERANCE,
            "{}: the pinned state drifts past the lock's tolerance: {frames:e}, {target:e}",
            p.name
        );
    }
}

#[test]
fn path_lock_centre_drift() {
    check_drift(|p| p.a.anchor.expect("recorded"));
}

negative_control!(
    path_lock_centre_drift,
    "the drift read against another state, the anchor of a lock elsewhere",
    expected = "drifts past the lock's tolerance",
    check_drift(|_| locked_latent(&plane_a(), (0.7, 0.2))
        .anchor
        .expect("recorded"))
);

// ── The sliding anchor ─────────────────────────────────────────────────────────────────────────────────────────────

use engine::path::blend::ChartLeaves;
use kernel::chart::{Burrau, HeldLz, MassSimplex, RestStart};
use ledger::constants::{EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX, MASS_SIMPLEX_EPS_M};

/// The mass simplex at rest, `ν₀ = ½`: every momentum 0 and one shape, so it holds no anchor with momenta.
fn simplex() -> RestStart<MassSimplex<f64>, f64> {
    RestStart::new(MassSimplex {
        nu0: 0.5,
        eps_m: MASS_SIMPLEX_EPS_M.number(),
    })
}

/// The `(ν, K)` Burrau chart at the register's defaults, holding `L_z = 0`.
fn nu_k() -> HeldLz<Burrau<f64>, f64> {
    HeldLz::new(Burrau {
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    })
}

/// A constructed `B` (REQ-CHART-146's, where none of G9's pairs crosses a cut locus or ends): its state at the chart
/// point `(u, v)` built from blocks about the anchor `a`, the masses `a`'s, the shape turned from `a`'s by
/// `θ₁(1 − u)/2`, `p_ρ` moved by `P₂(1 + u)/2` and `p_λ` by `(κ(u² − 1), v)`. Along a slide the frame's pin state is
/// `block_line(a, B(u, v), s)`, whose distance to `a` is `s²(P² + κ²(u²−1)² + v²) + chord(sθ)²`: the shape term's weight
/// falls as `s` grows, so the minimiser moves with `s`.
///
/// - The cut locus (`κ = 3`, `θ₁ = 2.6`, `P₂ = √5.2`): two wells, `u ≈ 0.93` nearer at `s → 0` and `u ≈ −0.98` nearer at
///   `s = 1`, equally near near `s ≈ 0.7`.
/// - The ending (`κ = 0`, `θ₁ = 3`, `P₂ = 2`, no state where `u < 0.34`): one minimiser, at `u ≈ 0.385` as `s → 0` and
///   `0.30` at `s = 1`, that runs into the edge of `B`'s states near `s ≈ 0.75`.
#[derive(Clone, Copy)]
struct Constructed {
    a: Blocks,
    theta: f64,
    p: f64,
    kappa: f64,
    edge: f64,
}

impl Constructed {
    fn cut_locus(a: &Anchor) -> Self {
        Constructed {
            a: a.blocks(),
            theta: 2.6,
            p: 5.2f64.sqrt(),
            kappa: 3.0,
            edge: f64::NEG_INFINITY,
        }
    }

    fn ending(a: &Anchor) -> Self {
        Constructed {
            a: a.blocks(),
            theta: 3.0,
            p: 2.0,
            kappa: 0.0,
            edge: 0.34,
        }
    }

    /// The blocks at `(u, v)`.
    fn blocks(&self, u: f64, v: f64) -> Blocks {
        let a = &self.a;
        // A unit vector orthogonal to `n̂_a`, then `n̂_a` turned toward it.
        let pick = if a.n[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let d = pick[0] * a.n[0] + pick[1] * a.n[1] + pick[2] * a.n[2];
        let w: [f64; 3] = std::array::from_fn(|i| pick[i] - d * a.n[i]);
        let nw = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
        let angle = self.theta * (1.0 - u) / 2.0;
        Blocks {
            mu: a.mu,
            n: std::array::from_fn(|i| angle.cos() * a.n[i] + angle.sin() * w[i] / nw),
            p_rho: [a.p_rho[0] + self.p * (1.0 + u) / 2.0, a.p_rho[1]],
            p_lambda: [
                a.p_lambda[0] + self.kappa * (u * u - 1.0),
                a.p_lambda[1] + v,
            ],
        }
    }
}

impl Leaves for Constructed {
    fn decode(&self, view: &View, [s, t]: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        if view.map != Map::Kernel(engine::path::KernelMap::of(&mixed(FROZEN))?) {
            return Err(PathError::WrongMap);
        }
        let [u, v] = view.sim.chart.window.at(s, t);
        let (mut d, _) = engine::path::blend::state_of(&self.blocks(u, v));
        if u < self.edge {
            d.ic.m[0] = f64::NAN;
        }
        Ok(d)
    }
}

/// The locked anchor of the slide tests: the skewed plane, pinned off the centre, a state with momenta.
fn slide_anchor() -> View {
    locked_latent(&plane_a(), (0.3, 0.65))
}

fn kernel_view<C: kernel::chart::Chart>(chart: &C, window: Window) -> View {
    let mut sim = sim_of(&plane_a());
    sim.chart.window = window;
    View::kernel(chart, sim).expect("a registry chart")
}

/// The constructed `B`'s view: `u ∈ [−1, 2]`, `v ∈ [−1, 1]`.
fn constructed_view() -> View {
    kernel_view(
        &mixed(FROZEN),
        Window {
            centre: [0.5, 0.0],
            half: [1.5, 1.0],
        },
    )
}

/// The pin's state on each frame of `n` steps over `[0, 1]`, read through `frame`, as blocks.
fn pin_track<L: Leaves>(
    seg: &Segment,
    leaves: &L,
    at: [f64; 2],
    n: usize,
    frame: &dyn Fn(&Segment, f64) -> View,
) -> Vec<[f64; 9]> {
    (0..=n)
        .map(|k| {
            let view = frame(seg, k as f64 / n as f64);
            Blocks::of(&state(anchor::state_at(&view, leaves, at).expect("a pixel")).ic).vector()
        })
        .collect()
}

/// The largest step between consecutive frames.
fn largest_step(track: &[[f64; 9]]) -> f64 {
    track
        .windows(2)
        .map(|w| {
            (0..9)
                .map(|i| (w[1][i] - w[0][i]).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0.0, f64::max)
}

/// G8's criterion: the largest step between frames falls to at most 0.6 of the previous each time the frame step is
/// halved, twice over (16, 32 and 64 frames).
fn check_continuous<L: Leaves>(
    name: &str,
    seg: &Segment,
    leaves: &L,
    at: [f64; 2],
    frame: &dyn Fn(&Segment, f64) -> View,
) {
    let steps: Vec<f64> = [16, 32, 64]
        .iter()
        .map(|n| largest_step(&pin_track(seg, leaves, at, *n, frame)))
        .collect();
    println!("path_lock_centre_slide: {name}: largest steps at 16, 32, 64 frames: {steps:?}");
    for w in steps.windows(2) {
        assert!(
            w[1] <= 0.6 * w[0],
            "{name}: the slide jumps: its largest step {} does not fall to 0.6 of {} as the frame step halves",
            w[1],
            w[0]
        );
    }
}

/// The segment's own frame.
fn own(seg: &Segment, s: f64) -> View {
    seg.frame(s).expect("a frame")
}

// ── path_lock_centre_slide ─────────────────────────────────────────────────────────────────────────────────────────
//
// Pass, on a rest `B` and a Burrau `B` (no cut locus on either): the target can't hold the anchor and the anchor slides
// (`LockedEnd::Slides`, not labelled "not nearest"); at `s = 0` the pin shows the stored anchor bit for bit; at `s = 1`
// it shows `B`'s nearest point to the anchor, the restart rule's minimiser: the same minimiser, its chart point within
// the solver core's agreement `10⁻⁶·h` of the restart's (`h` the window's half-width), the distance of the two states in
// `‖·‖_phys` reported; between, the slide is continuous by G8's criterion.
//
// The first criterion at `s = 1` was the two states within the lock's tolerance in `‖·‖_phys`. Its first run measured
// 9.4·10⁻¹² (42 427 ε) on the rest chart: two solves of the same minimiser off `B`'s image (the anchor 0.53 from `B`)
// agree only to the precision of the central-difference slopes they stop on, `ε^{2/3}`, and an optimality tolerance of
// `10⁻¹⁴` makes the solve fail to converge at all. So the identity of the minimiser is asked as the solver core asks it.

fn rest_and_burrau() -> Vec<(&'static str, Segment, LockedEnd, View)> {
    let a = slide_anchor();
    let rest = (
        LatentLeaves::<Defaults>::new(),
        ChartLeaves::<_, Defaults>::new(simplex()).expect("a chart"),
    );
    let burrau = (
        LatentLeaves::<Defaults>::new(),
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let b_rest = kernel_view(&simplex(), Window::SQUARE);
    let b_burrau = kernel_view(&nu_k(), Window::SQUARE);
    let (s1, e1) = Segment::locked_blend(&a, &b_rest, BlendPath::BlockLine, &rest, phys, 64)
        .expect("a locked path to the rest chart");
    let (s2, e2) = Segment::locked_blend(&a, &b_burrau, BlendPath::BlockLine, &burrau, phys, 64)
        .expect("a locked path to Burrau");
    vec![("rest", s1, e1, b_rest), ("Burrau", s2, e2, b_burrau)]
}

fn check_slide(frame: &dyn Fn(&Segment, f64) -> View) {
    let a = slide_anchor();
    let at = pin(&a);
    let anchor = a.anchor.expect("recorded");
    let rest = (
        LatentLeaves::<Defaults>::new(),
        ChartLeaves::<_, Defaults>::new(simplex()).expect("a chart"),
    );
    let burrau = (
        LatentLeaves::<Defaults>::new(),
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    for (name, seg, end, b) in rest_and_burrau() {
        assert_eq!(
            end,
            LockedEnd::Slides { not_nearest: false },
            "{name}: not a slide"
        );
        let leaves: &dyn Fn(&View, [f64; 2]) -> Decoded<f64> = &|v, x| match name {
            "rest" => state(anchor::state_at(v, &rest, x).expect("a pixel")),
            _ => state(anchor::state_at(v, &burrau, x).expect("a pixel")),
        };
        let first = leaves(&frame(&seg, 0.0), at);
        assert_eq!(
            words(&first.ic),
            anchor.words,
            "{name}: s = 0 is not the stored anchor"
        );
        let nearest = match name {
            "rest" => anchor::nearest(&b, &rest, at, &anchor),
            _ => anchor::nearest(&b, &burrau, at, &anchor),
        }
        .expect("a nearest point");
        let end = frame(&seg, 1.0);
        let last = leaves(&end, at);
        let miss = phys(&nearest.ic, &last.ic);
        let h = b.sim.chart.window.half[0]
            .abs()
            .max(b.sim.chart.window.half[1].abs());
        let x = end.sim.chart.window.at(at[0], at[1]);
        let apart = (x[0] - nearest.x[0]).hypot(x[1] - nearest.x[1]);
        println!(
            "path_lock_centre_slide: {name}: s = 1 against B's nearest point: chart points {apart:e} apart, states {:.3} ε; the anchor's distance to B {:.6}",
            miss / f64::EPSILON,
            phys(&anchor.ic(), &nearest.ic)
        );
        assert!(
            apart <= 1e-6 * h,
            "{name}: s = 1 is not B's nearest point: {apart:e} apart"
        );
        match name {
            "rest" => check_continuous(name, &seg, &rest, at, frame),
            _ => check_continuous(name, &seg, &burrau, at, frame),
        }
    }
}

#[test]
fn path_lock_centre_slide() {
    check_slide(&own);
}

negative_control!(
    path_lock_centre_slide,
    "a labelled jump where the anchor can't exist in B: A's frame until s = 1, then B",
    expected = "the slide jumps",
    check_slide(&|seg, s| match s < 1.0 {
        true => seg.a().clone(),
        false => seg.b().clone(),
    })
);

#[test]
fn path_lock_centre_slide_no_snap() {
    check_slide(&own);
}

negative_control!(
    path_lock_centre_slide_no_snap,
    "an unlabelled snap to B's nearest point at one mid-path frame",
    expected = "the slide jumps",
    check_slide(&|seg, s| match s == 0.5 {
        true => seg.b().clone(),
        false => own(seg, s),
    })
);

// ── path_lock_centre_slide_cut_locus ───────────────────────────────────────────────────────────────────────────────
//
// Pass, on the constructed cut-locus `B`: the anchor slides; at the first frame the followed minimiser is the nearest
// (not labelled), it is labelled "not nearest" at `s = 1` and at every frame from the first one labelled on; at `s = 1`
// the pin shows the followed minimiser, not `B`'s nearest point (the two states more than the lock's tolerance apart in
// `‖·‖_phys`, and the followed one farther from the anchor); the slide is continuous by G8's criterion.

fn cut_locus() -> (View, Constructed, Segment, LockedEnd) {
    let a = slide_anchor();
    let b = Constructed::cut_locus(&a.anchor.expect("recorded"));
    let leaves = (LatentLeaves::<Defaults>::new(), b);
    let (seg, end) = Segment::locked_blend(
        &a,
        &constructed_view(),
        BlendPath::BlockLine,
        &leaves,
        phys,
        64,
    )
    .expect("a locked path to the constructed B");
    (a, b, seg, end)
}

fn check_cut_locus(frame: &dyn Fn(&Segment, f64) -> View) {
    let (a, b, seg, end) = cut_locus();
    let leaves = (LatentLeaves::<Defaults>::new(), b);
    let at = pin(&a);
    let anchor = a.anchor.expect("recorded");
    assert_eq!(
        end,
        LockedEnd::Slides { not_nearest: true },
        "the cut locus is not labelled"
    );
    let engine::path::Family::Slide { track, .. } = seg.family() else {
        panic!("not a slide")
    };
    let first_on = track.nodes.iter().position(|n| n.not_nearest);
    println!(
        "path_lock_centre_slide_cut_locus: labelled \"not nearest\" from s = {:?}; the followed point at s = 1: {:?}",
        first_on.map(|k| track.nodes[k].s),
        track.nodes.last().map(|n| n.x)
    );
    let k = first_on.expect("a frame labelled not nearest");
    assert!(
        k > 0 && track.nodes[k..].iter().all(|n| n.not_nearest),
        "the label is not on from the cut locus to the end"
    );
    let nearest =
        anchor::nearest(&constructed_view(), &leaves, at, &anchor).expect("a nearest point");
    let last = state(anchor::state_at(&frame(&seg, 1.0), &leaves, at).expect("a pixel"));
    let (to_followed, to_nearest) = (
        phys(&anchor.ic(), &last.ic),
        phys(&anchor.ic(), &nearest.ic),
    );
    println!(
        "path_lock_centre_slide_cut_locus: the anchor's distance to the followed point {to_followed:.6}, to B's nearest {to_nearest:.6}"
    );
    assert!(
        phys(&nearest.ic, &last.ic) > LOCK_TOLERANCE && to_followed > to_nearest,
        "s = 1 jumped to the nearer point at the cut locus"
    );
    check_continuous("cut locus", &seg, &leaves, at, frame);
}

#[test]
fn path_lock_centre_slide_cut_locus() {
    check_cut_locus(&own);
}

negative_control!(
    path_lock_centre_slide_cut_locus,
    "a jump to the nearer point at the cut locus: from the first frame labelled on, B placed at its nearest point",
    expected = "jumped to the nearer point",
    {
        let (a, b, seg, _) = cut_locus();
        let leaves = (LatentLeaves::<Defaults>::new(), b);
        let at = pin(&a);
        let near = anchor::nearest(&constructed_view(), &leaves, at, &a.anchor.expect("recorded"))
            .expect("a nearest point");
        let engine::path::Family::Slide { track, .. } = seg.family().clone() else {
            panic!("not a slide")
        };
        let cut = track.nodes.iter().find(|n| n.not_nearest).map(|n| n.s).expect("labelled");
        check_cut_locus(&|seg, s| match s >= cut {
            true => {
                let end = anchor::placed(&constructed_view(), at, near.x);
                View::blend(
                    Blend::new(seg.a().clone(), end, BlendPath::BlockLine, s).expect("a blend"),
                )
            }
            false => own(seg, s),
        })
    }
);

// ── path_lock_centre_slide_ends ────────────────────────────────────────────────────────────────────────────────────
//
// Pass, on the constructed ending `B`: the path stops (`LockedEnd::Ended { s }`, `0 < s < 1`, labelled "the followed
// system ends here"); its last frame, the segment's `B`, is the blend at that `s`, its pin showing that frame's state;
// the slide up to it is continuous by G8's criterion; and no frame of the segment reads the path past the stop.

fn ending() -> (View, Constructed, Segment, LockedEnd) {
    let a = slide_anchor();
    let b = Constructed::ending(&a.anchor.expect("recorded"));
    let leaves = (LatentLeaves::<Defaults>::new(), b);
    let (seg, end) = Segment::locked_blend(
        &a,
        &constructed_view(),
        BlendPath::BlockLine,
        &leaves,
        phys,
        64,
    )
    .expect("a locked path to the constructed B");
    (a, b, seg, end)
}

fn check_ends(frame: &dyn Fn(&Segment, f64) -> View) {
    let (a, b, seg, end) = ending();
    let leaves = (LatentLeaves::<Defaults>::new(), b);
    let at = pin(&a);
    let LockedEnd::Ended { s: stop } = end else {
        panic!("the path does not stop: {end:?}")
    };
    println!("path_lock_centre_slide_ends: the followed system ends after s = {stop}");
    assert!(
        stop > 0.0 && stop < 1.0,
        "the stop is not inside the path: {stop}"
    );
    let Map::Blend(last) = &seg.b().map else {
        panic!("the last frame is not a blend")
    };
    assert_eq!(last.s(), stop, "the last frame is not the stop's");
    // Frames past the stop: each frame of the segment reads the path at `s·stop`, so none reaches past it.
    for k in 0..=16 {
        let s = k as f64 / 16.0;
        let view = frame(&seg, s);
        let shown = anchor::state_at(&view, &leaves, at).expect("a pixel");
        assert!(
            matches!(shown, Pixel::State { .. }),
            "the path runs on past the followed minimiser's end: no state at s = {s}"
        );
        if let Map::Blend(blend) = &view.map {
            assert!(
                blend.s() <= stop,
                "the path runs on past the followed minimiser's end: s = {}",
                blend.s()
            );
        }
    }
    check_continuous("ending", &seg, &leaves, at, frame);
}

#[test]
fn path_lock_centre_slide_ends() {
    check_ends(&own);
}

negative_control!(
    path_lock_centre_slide_ends,
    "the path running on past the followed minimiser's end: the blend carried on to s = 1",
    expected = "runs on past the followed minimiser's end",
    check_ends(&|seg, s| {
        let Map::Blend(last) = &seg.b().map else {
            panic!("not a blend")
        };
        View::blend(last.at(s).expect("a share"))
    })
);

// ── path_lock_centre_slide_go_on ───────────────────────────────────────────────────────────────────────────────────
//
// Pass: going on from the stop needs the user's choice, which re-pins: from the stopped frame as it is, no locked path
// can be built (it holds no anchor); recorded again, its anchor is the stopped frame's pin state, a new pin in the lock's
// record, different from the first anchor; from it a locked path to the rest chart is built.

fn check_go_on(repin: impl Fn(&View) -> View) {
    let (a, b, seg, _) = ending();
    let leaves = (LatentLeaves::<Defaults>::new(), b);
    let stopped = seg.b().clone();
    assert!(
        matches!(
            Segment::locked_blend(
                &stopped,
                &constructed_view(),
                BlendPath::BlockLine,
                &leaves,
                phys,
                8
            ),
            Err(PathError::NotBuilt(_))
        ),
        "going on from the stop without re-pinning"
    );
    let again = repin(&stopped);
    let at = pin(&a);
    let shown = state(anchor::state_at(&stopped, &leaves, at).expect("a pixel"));
    assert!(
        again.anchor.is_some_and(
            |x| x.words == words(&shown.ic) && x.words != a.anchor.expect("recorded").words
        ),
        "going on from the stop without a new pin recorded"
    );
}

#[test]
fn path_lock_centre_slide_go_on() {
    check_go_on(|v| {
        let leaves = (
            LatentLeaves::<Defaults>::new(),
            Constructed::ending(&slide_anchor().anchor.expect("recorded")),
        );
        anchor::record(v, &leaves).expect("re-pinned")
    });
}

negative_control!(
    path_lock_centre_slide_go_on,
    "going on from the stop with the first pin's record carried over",
    expected = "without a new pin recorded",
    check_go_on(|v| View {
        anchor: slide_anchor().anchor,
        ..v.clone()
    })
);

// ── The arms that wait on other tasks ──────────────────────────────────────────────────────────────────────────────

/// Region D's slide (F9): its arm is TASK-M2-68's, the region D chart pair not yet on main.
#[test]
#[ignore = "waits on TASK-M2-68"]
fn path_lock_centre_slide_region_d() {
    panic!("region D's chart pair is TASK-M2-68's");
}

negative_control!(
    path_lock_centre_slide_region_d,
    "waits on TASK-M2-68",
    expected = "region D",
    panic!("region D's chart pair is TASK-M2-68's")
);

/// A time-axis chart to an IC-only chart slides with no jump: TASK-M2-83's pair, not yet on main.
#[test]
#[ignore = "waits on TASK-M2-83"]
fn path_lock_centre_slide_time_axis() {
    panic!("the time-axis pair is TASK-M2-83's");
}

negative_control!(
    path_lock_centre_slide_time_axis,
    "waits on TASK-M2-83",
    expected = "time-axis pair",
    panic!("the time-axis pair is TASK-M2-83's")
);

// ── The owner's gate test: lock → morph A → B → tilt → slice → morph back ─────────────────────────────────────────

use engine::path::LatentTilt;
use kernel::chart::tilt::{TiltMap, TiltSpec};

/// Where the morph is paused for the tilt and the slice.
const MID: f64 = 0.4;

/// One gate pair: its locked start, its target, and each end's own tilt and slice.
struct Gate {
    name: &'static str,
    a: View,
    b: View,
    tilt_a: fn(&View) -> Segment,
    tilt_b: fn(&View) -> Segment,
    slice_a: fn(&View) -> Segment,
    slice_b: fn(&View) -> Segment,
}

/// The latent frame `(e_a, e_b)` at the scale `σ`, untilted.
fn frame_plane(z0: Latent, a: usize, b: usize, scale: f64) -> Plane {
    Plane {
        z0,
        q1: e(a, scale),
        q2: e(b, scale),
    }
}

/// R-474's tilt of a latent end in its own frame `(e_a, e_b)`, Tip as sheet toward its hidden `x`, about the pin.
fn latent_tilt(v: &View, a: u32, b: u32, x: u32) -> Segment {
    let tilt = LatentTilt {
        a,
        b,
        scale: 0.5,
        from: [0.0, 0.0, 0.0],
    };
    let spec = TiltSpec {
        tau1: 20.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x,
        y: x,
    };
    Segment::latent_tilt(v, &tilt, &spec, 0.0).expect("a latent tilt")
}

/// The mixed chart's own tilt, toward its hidden `β` (`z[1]`), about the pin.
fn mixed_tilt(v: &View) -> Segment {
    let spec = TiltSpec {
        tau1: 20.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: 1,
        y: 1,
    };
    Segment::curved_tilt(v, &mixed(v.sim.plane.z0), &spec).expect("a curved tilt")
}

const GATE_Z0: Latent = [0.25, 0.8, 0.1, -0.15, 0.05, 0.1, 0.2, -0.1];

fn gates() -> Vec<Gate> {
    let a_plane = frame_plane(GATE_Z0, 0, 1, 0.5);
    let b_plane = frame_plane([-0.3, 0.5, 0.2, 0.1, -0.2, 0.15, -0.4, 0.3], 2, 3, 0.5);
    let moved_of = |v: &View| {
        let mut b = v.clone();
        b.sim.chart.window = moved();
        b.pin = None;
        b.anchor = None;
        b.sim.lock.locked = false;
        b
    };
    let mixed_a = locked_mixed(home(), (0.35, 0.6));
    vec![
        Gate {
            name: "affine",
            a: locked_latent(&a_plane, (0.3, 0.65)),
            b: View::latent(sim_of(&b_plane)),
            tilt_a: |v| latent_tilt(v, 0, 1, 5),
            tilt_b: |v| latent_tilt(v, 2, 3, 6),
            slice_a: |v| Segment::slice(v, &e(4, 1.0), 0.2).expect("a slice"),
            slice_b: |v| Segment::slice(v, &e(4, 1.0), 0.2).expect("a slice"),
        },
        Gate {
            name: "nonlinear",
            b: moved_of(&mixed_a),
            a: mixed_a,
            tilt_a: mixed_tilt,
            tilt_b: mixed_tilt,
            slice_a: |v| Segment::control(v, 4, v.sim.plane.z0[4] + 0.2).expect("a slider's move"),
            slice_b: |v| Segment::control(v, 4, v.sim.plane.z0[4] + 0.2).expect("a slider's move"),
        },
    ]
}

/// The gate path's segments, in order: the morph to `MID`, the tilt, the slice and the morph back.
struct Run {
    segments: Vec<(&'static str, Segment, f64)>,
}

fn blend_of(v: &View) -> &Blend {
    match &v.map {
        Map::Blend(b) => b,
        _ => panic!("not a blend view"),
    }
}

fn run(g: &Gate) -> Run {
    let (morph, end) = Segment::locked_blend(&g.a, &g.b, BlendPath::BlockLine, &leaves(), phys, 64)
        .unwrap_or_else(|e| panic!("{}: no locked path: {e:?}", g.name));
    assert_eq!(
        end,
        LockedEnd::Holds,
        "{}: the target does not hold",
        g.name
    );
    let v1 = morph.frame(MID).expect("the paused frame");
    let blend = blend_of(&v1);
    let tilt = Segment::on_ends(&v1, &(g.tilt_a)(&blend.a), &(g.tilt_b)(&blend.b))
        .expect("the tilt mid-morph");
    let v2 = tilt.b().clone();
    let ends = blend_of(&v2);
    let slice = Segment::locked_slice(&v2, &(g.slice_a)(&ends.a), &(g.slice_b)(&ends.b), &leaves())
        .expect("the slice mid-morph");
    let v3 = slice.b().clone();
    let back = Segment::share(&v3, 0.0).expect("the morph back");
    Run {
        segments: vec![
            ("morph", morph, MID),
            ("tilt", tilt, 1.0),
            ("slice", slice, 1.0),
            ("back", back, 1.0),
        ],
    }
}

/// The state the frame `view` shows at `at`, and, on a locked mid frame that holds, what the pin would show without the
/// select: the block line between the pinned state and `B`'s own pin state, rebuilt (the drift the select removes).
fn pinned_and_unselected(view: &View, at: [f64; 2]) -> (Ic<f64>, Ic<f64>, Option<Ic<f64>>) {
    let shown = state(anchor::state_at(view, &leaves(), at).expect("a pixel")).ic;
    let Map::Blend(blend) = &view.map else {
        return (shown, shown, None);
    };
    let a_pin = state(anchor::state_at(&blend.a, &leaves(), at).expect("a pixel")).ic;
    let pinned = blend
        .lock()
        .and_then(|l| l.anchor)
        .map_or(a_pin, |x| x.ic());
    if blend.s() == 0.0 || blend.s() == 1.0 {
        return (shown, pinned, None);
    }
    let b = match blend.rehold() {
        true => {
            anchor::rehold(&blend.a, &blend.b, &leaves(), phys, blend.s())
                .expect("re-held")
                .0
        }
        false => blend.b.clone(),
    };
    let raw = leaves().decode(&b, at).expect("a decode").ic;
    let (line, _) =
        engine::path::blend::block_line(&Blocks::of(&pinned), &Blocks::of(&raw), blend.s());
    (
        shown,
        pinned,
        Some(engine::path::blend::state_of(&line).0.ic),
    )
}

/// The drift measured along a gate run, frame by frame: the shown state against the pinned state, and without the
/// select; `frames` frames per segment.
fn check_round_trip(
    g: &Gate,
    run: &Run,
    frames: usize,
    frame: &dyn Fn(&Segment, f64) -> View,
) -> Vec<f64> {
    let at = pin(&g.a);
    let mut unselected = Vec::new();
    for (what, seg, upto) in &run.segments {
        for k in 0..=frames {
            let u = upto * k as f64 / frames as f64;
            let view = frame(seg, u);
            let (shown, pinned, raw) = pinned_and_unselected(&view, at);
            let drift = phys(&pinned, &shown);
            assert!(
                drift <= LOCK_TOLERANCE,
                "{}: {what} at u = {u}: the pin drifts {:.3} ε from the pinned state, past the lock's tolerance",
                g.name,
                drift / f64::EPSILON
            );
            if let Some(raw) = raw {
                let d = phys(&pinned, &raw);
                assert!(
                    d <= LOCK_TOLERANCE,
                    "{}: {what} at u = {u}: without the select the pin drifts {:.3} ε, past the lock's tolerance",
                    g.name,
                    d / f64::EPSILON
                );
                unselected.push(d);
            }
        }
    }
    unselected
}

/// R1: the return to `s = 0` after the mid-morph tilt and slice, bitwise the same tilt and slice applied directly to `A`
/// about the pin, at every pixel of an 8² grid and the pin.
fn check_r1(g: &Gate, run: &Run) {
    let back = run.segments[3].1.b();
    let direct = (g.slice_a)((g.tilt_a)(&g.a).b()).b().clone();
    let direct = anchor::record(&direct, &leaves()).expect("recorded");
    let mut probes: Vec<[f64; 2]> = (0..64)
        .map(|k| {
            [
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            ]
        })
        .collect();
    probes.push(pin(&g.a));
    for p in probes {
        let x = state(anchor::state_at(back, &leaves(), p).expect("a pixel")).ic;
        let y = state(anchor::state_at(&direct, &leaves(), p).expect("a pixel")).ic;
        assert_eq!(
            words(&x),
            words(&y),
            "{}: R1 fails: the return differs from the direct edit at {p:?}",
            g.name
        );
    }
    assert_eq!(
        blend_of(back).a.bytes(),
        direct.bytes(),
        "{}: R1 fails: the return's view is not the direct edit's",
        g.name
    );
}

/// The worst, the median and the 95th percentile of `xs`, in units of `ε`.
fn summary(xs: &[f64]) -> (f64, f64, f64) {
    let mut v: Vec<f64> = xs.iter().map(|x| x / f64::EPSILON).collect();
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    (v[v.len() - 1], at(0.5), at(0.95))
}

// Pass, on the affine and the nonlinear pair: on every frame of the morph to `s = 0.4`, the tilt mid-morph, the slice
// mid-morph and the morph back (16 frames each), the state at the pin within the lock's tolerance of the pinned state in
// `‖·‖_phys`; and R1, the return bitwise the direct tilt and slice of `A` about the pin. Reported, the measurement the
// lock's tolerance is proposed from: the drift without the select, frame by frame (64 frames per segment), its worst,
// median and 95th percentile per pair.
#[test]
fn path_lock_round_trip() {
    for g in gates() {
        let run = run(&g);
        check_round_trip(&g, &run, 16, &own);
        let drift = check_round_trip(&g, &run, 64, &own);
        let (worst, median, p95) = summary(&drift);
        println!(
            "path_lock_round_trip: {}: the drift without the select over {} frames: worst {worst:.3} ε, median {median:.3} ε, 95th percentile {p95:.3} ε",
            g.name,
            drift.len()
        );
        check_r1(&g, &run);
    }
}

negative_control!(
    path_lock_round_trip,
    "the lock dropped on a blend segment: the morph's frames blend the ends' own pins freely",
    expected = "past the lock's tolerance",
    for g in gates() {
        let run = run(&g);
        check_round_trip(&g, &run, 16, &|seg, u| {
            let view = own(seg, u);
            match &view.map {
                Map::Blend(b) if b.s() > 0.0 && b.s() < 1.0 => {
                    let mut a = b.a.clone();
                    a.anchor = None;
                    let mut bb = b.b.clone();
                    bb.anchor = None;
                    let b0 =
                        Blend::new(a, g.b.clone(), BlendPath::BlockLine, b.s()).expect("a blend");
                    View::blend(b0)
                }
                _ => view,
            }
        });
    }
);

// ── path_mid_morph_tilt ────────────────────────────────────────────────────────────────────────────────────────────
//
// Pass: the tilt mid-morph reaches both ends, each in its own hidden coordinates and tilt map: each end of the tilted
// blend is, bit for bit, that end's own tilt about the pin; and the morph resumes from its parameter: the morph back's
// first frame is the paused blend's share, and its frames run that share down to 0.

fn check_own_coordinates(tilt_b: &dyn Fn(&View, &Gate) -> Segment) {
    for g in gates() {
        let (morph, _) =
            Segment::locked_blend(&g.a, &g.b, BlendPath::BlockLine, &leaves(), phys, 64)
                .expect("a path");
        let v1 = morph.frame(MID).expect("paused");
        let blend = blend_of(&v1);
        let tilt =
            Segment::on_ends(&v1, &(g.tilt_a)(&blend.a), &tilt_b(&blend.b, &g)).expect("the tilt");
        let ends = blend_of(tilt.b());
        assert_eq!(
            ends.a.bytes(),
            (g.tilt_a)(&blend.a).b().bytes(),
            "{}: end A is not its own tilt",
            g.name
        );
        assert_eq!(
            ends.b.bytes(),
            (g.tilt_b)(&blend.b).b().bytes(),
            "{}: end B is not tilted in its own hidden coordinates",
            g.name
        );
        assert_eq!(ends.s(), MID, "{}: the tilt moved the share", g.name);
    }
}

#[test]
fn path_mid_morph_tilt() {
    check_own_coordinates(&|v, g| (g.tilt_b)(v));
}

negative_control!(
    path_mid_morph_tilt,
    "a hidden coordinate invented for the blend: B tilted by A's tilt, in A's coordinates",
    expected = "not tilted in its own hidden coordinates",
    check_own_coordinates(&|v, g| match g.name {
        "affine" => latent_tilt(v, 0, 1, 5),
        _ => {
            let spec = TiltSpec {
                tau1: 20.0,
                tau2: 0.0,
                map: TiltMap::Sheet,
                x: 2,
                y: 2,
            };
            Segment::curved_tilt(v, &mixed(v.sim.plane.z0), &spec).expect("a curved tilt")
        }
    })
);

fn check_resumes(frame: &dyn Fn(&Segment, f64) -> View) {
    for g in gates() {
        let run = run(&g);
        let back = &run.segments[3].1;
        for k in 0..=8 {
            let u = k as f64 / 8.0;
            let want = MID * (1.0 - u);
            let got = blend_of(&frame(back, u)).s();
            assert!(
                (got - want).abs() <= 4.0 * f64::EPSILON,
                "{}: the morph does not resume from its parameter: share {got} at u = {u}, not {want}",
                g.name
            );
        }
    }
}

#[test]
fn path_mid_morph_tilt_resumes() {
    check_resumes(&own);
}

negative_control!(
    path_mid_morph_tilt_resumes,
    "a morph restarted from s = 0",
    expected = "does not resume from its parameter",
    check_resumes(&|seg, u| {
        let Map::Blend(b) = &seg.a().map else {
            panic!("not a blend")
        };
        View::blend(b.at(u).expect("a share"))
    })
);

// ── saved_file_stage2_conversion ───────────────────────────────────────────────────────────────────────────────────
//
// Pass: every stage-1 record of the fixtures (a free view, a lock on the affine chart off the centre, a lock on the
// mixed chart, an old lock with no pin, a free morph frame whose ends are locked views) converts on load: each locked
// view gains its lock's record, the converted record is stage 2's, and at every pixel of an 8² grid and at the pin each
// converted view decodes to the same system as the old one within `ε_phys` in `‖·‖_phys`; a stage-2 record reads back
// and writes again bit for bit; a stage-1 record's bytes are what stage 1 wrote (no version), a free record unchanged by
// conversion. Read through `reader`, which the control replaces.

fn stage1_records() -> Vec<(&'static str, FrameRecord)> {
    let rec = |a: View, b: View| FrameRecord {
        a,
        b,
        family: engine::path::Family::Cut {
            label: engine::path::CutLabel::Lock,
        },
        s_bits: 0,
        align: None,
        retime_hash: None,
        anchor_words: None,
    };
    let strip = |v: &View| View {
        anchor: None,
        ..v.clone()
    };
    let free = View::latent(sim_of(&plane_a()));
    let affine = strip(&locked_latent(&plane_a(), (0.3, 0.65)));
    let nonlinear = strip(&locked_mixed(home(), (0.35, 0.6)));
    let mut old = View::latent(sim_of(&plane_a()));
    old.sim.lock.locked = true;
    old.sim.lock.z_locked = engine::lock::point(&plane_a(), 0.3, 0.65);
    let morph = View::blend(
        Blend::new(
            affine.clone(),
            View::latent(sim_of(&plane_b())),
            BlendPath::BlockLine,
            0.4,
        )
        .expect("a blend"),
    );
    vec![
        ("free", rec(free.clone(), free)),
        ("affine lock", rec(View::latent(sim_of(&plane_a())), affine)),
        ("nonlinear lock", rec(mixed_view(home()), nonlinear)),
        ("an old lock, no pin", rec(old.clone(), old)),
        ("a morph frame of a locked view", rec(morph.clone(), morph)),
    ]
}

/// The 8² grid and the pin of a view.
fn probes(v: &View) -> Vec<[f64; 2]> {
    let mut p: Vec<[f64; 2]> = (0..64)
        .map(|k| {
            [
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            ]
        })
        .collect();
    if let Some((at, _)) = anchor::pin_address(v) {
        p.push(at);
    }
    p
}

fn check_conversion(reader: &dyn Fn(&[u8]) -> Result<FrameRecord, PathError>) {
    let eps_phys = Tolerances::register(0.0).eps_phys;
    for (name, old) in stage1_records() {
        let bytes = old.to_bytes();
        assert_eq!(
            old.version(),
            engine::path::Version::One,
            "{name}: not a stage-1 record"
        );
        assert!(
            !String::from_utf8_lossy(&bytes).contains("\"version\""),
            "{name}: stage 1's bytes carry a version"
        );
        let new = reader(&bytes)
            .unwrap_or_else(|e| panic!("{name}: an old record does not convert on load: {e:?}"));
        let locked = [&old.a, &old.b]
            .iter()
            .any(|v| v.sim.lock.locked && !matches!(v.map, Map::Blend(_)));
        assert_eq!(
            new.version() == engine::path::Version::Two,
            locked,
            "{name}: the converted record's version"
        );
        let mut worst = 0.0f64;
        for (o, n) in [(&old.a, &new.a), (&old.b, &new.b)] {
            for p in probes(n) {
                let x = state(anchor::state_at(o, &leaves(), p).expect("old")).ic;
                let y = state(anchor::state_at(n, &leaves(), p).expect("new")).ic;
                worst = worst.max(phys(&x, &y));
            }
        }
        println!(
            "saved_file_stage2_conversion: {name}: the converted record's worst distance {:.3} ε",
            worst / f64::EPSILON
        );
        assert!(
            worst <= eps_phys,
            "{name}: the converted record decodes to another system: {worst:e}"
        );
        let again = FrameRecord::read(&new.to_bytes()).expect("a stage-2 record reads");
        assert_eq!(
            again.to_bytes(),
            new.to_bytes(),
            "{name}: a stage-2 record does not read back bit for bit"
        );
        if !locked {
            assert_eq!(
                new.to_bytes(),
                bytes,
                "{name}: a free record changed in conversion"
            );
        }
    }
}

#[test]
fn saved_file_stage2_conversion() {
    check_conversion(&|b| FrameRecord::read_converted(b, &leaves()));
}

/// `bytes` with `"version": 2` written in, as if stage 2 had written them.
#[cfg(feature = "controls")]
fn as_stage2(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8(bytes.to_vec()).expect("utf-8");
    text.replacen('{', "{\"version\":2,", 1).into_bytes()
}

negative_control!(
    saved_file_stage2_conversion,
    "an old record read as the new version",
    expected = "does not convert on load",
    check_conversion(&|b| FrameRecord::read_converted(&as_stage2(b), &leaves()))
);

// Pass: a record whose version is neither stage 1's (none) nor stage 2's (2) is refused, read either way.
fn check_unknown(reader: &dyn Fn(&[u8]) -> Result<FrameRecord, PathError>) {
    let (_, old) = stage1_records().remove(1);
    let text = String::from_utf8(old.to_bytes()).expect("utf-8");
    for v in [0, 1, 3, 99] {
        let bytes = text.replacen('{', &format!("{{\"version\":{v},"), 1);
        assert!(
            reader(bytes.as_bytes()).is_err(),
            "an unknown version, {v}, is accepted"
        );
    }
}

#[test]
fn saved_file_stage2_conversion_unknown() {
    check_unknown(&|b| FrameRecord::read_converted(b, &leaves()));
}

negative_control!(
    saved_file_stage2_conversion_unknown,
    "an unknown version accepted: the version dropped before reading",
    expected = "is accepted",
    check_unknown(&|b| {
        let text = String::from_utf8(b.to_vec()).expect("utf-8");
        let cut = text.find(',').expect("a field") + 1;
        FrameRecord::read_converted(format!("{{{}", &text[cut..]).as_bytes(), &leaves())
    })
);

// ── curved_pivot_chart_switch ──────────────────────────────────────────────────────────────────────────────────────
//
// Pass: a lock at the centre of a tilted mixed view (a curved chart, its pivot record set), recorded; a switch to a named
// target that holds the pinned state (the mixed chart through another window, tilted its own way) and back: on each
// chart the pinned pixel's own decode (no select) is within the lock's tolerance of the pinned state in `‖·‖_phys`, the
// lock's record unchanged bit for bit (the anchor's words), each switch labelled `SwitchHolds`; a switch to a named
// target that can't hold it (the mass simplex at rest): labelled `SwitchSlides`, the pin showing the target's nearest
// point to the pinned state (no select: its own decode, more than the tolerance from the anchor, within the solve's
// agreement of the restart's point), the record unchanged.

type SwitchLeaves = (
    LatentLeaves<Defaults>,
    (
        MixedLeaves,
        ChartLeaves<RestStart<MassSimplex<f64>, f64>, Defaults>,
    ),
);

fn switch_leaves() -> SwitchLeaves {
    (
        LatentLeaves::new(),
        (MixedLeaves, ChartLeaves::new(simplex()).expect("a chart")),
    )
}

/// The tilted mixed view through `window`, its own tilt `tau` toward `β`.
fn tilted_mixed(window: Window, tau: f64) -> View {
    let v = mixed_view(window);
    let spec = TiltSpec {
        tau1: tau,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: 1,
        y: 1,
    };
    Segment::curved_tilt(&v, &mixed(FROZEN), &spec)
        .expect("a tilt")
        .b()
        .clone()
}

/// The tilted mixed view locked at its centre and recorded: its pivot record set.
fn pivoted_lock() -> View {
    let v = tilted_mixed(home(), 20.0);
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let (seg, _) =
        Segment::lock_on_chart::<Defaults, _, _>(&v, &mixed(FROZEN), (0.5, 0.5), phys, &tol)
            .expect("a lock");
    let locked = anchor::record(seg.b(), &switch_leaves()).expect("recorded");
    assert!(
        locked.pivot.is_some(),
        "the lock on a tilted curved view sets no pivot record"
    );
    locked
}

/// The raw decode of `v` at its pin: the chart's own state there, the select not applied.
fn raw_at_pin(v: &View) -> Ic<f64> {
    switch_leaves().decode(v, pin(v)).expect("a decode").ic
}

fn check_switch(after: &dyn Fn(View, &View) -> View) {
    let a = pivoted_lock();
    let record = a.anchor.expect("recorded");
    // The holder: the mixed chart with its axes swapped, through its own window, tilted its own way.
    let holder = {
        let mut v = tilted_mixed(moved(), -15.0);
        v.sim.chart.mapping[0] = engine::contract::sim_config::Mapping::Linear;
        v.sim.chart.window = Window {
            centre: [0.75, 0.1],
            half: [0.3, 0.6],
        };
        View {
            pin: None,
            anchor: None,
            pivot: None,
            ..v
        }
    };
    let there = Segment::locked_switch(&a, &holder, &switch_leaves(), phys).expect("the switch");
    let b = after(there.b().clone(), &a);
    let back = Segment::locked_switch(&b, &a, &switch_leaves(), phys).expect("the switch back");
    for (what, seg, v) in [("there", &there, &b), ("back", &back, back.b())] {
        assert_eq!(
            seg.family(),
            &engine::path::Family::Cut {
                label: engine::path::CutLabel::SwitchHolds
            },
            "{what}: not labelled as holding"
        );
        let miss = phys(&record.ic(), &raw_at_pin(v));
        println!("curved_pivot_chart_switch: {what}: the pinned pixel's own decode {:.3} ε from the pinned state", miss / f64::EPSILON);
        assert!(
            miss <= LOCK_TOLERANCE,
            "{what}: the pinned pixel does not decode the pinned state: {miss:e}"
        );
        assert_eq!(
            v.anchor.map(|x| x.words),
            Some(record.words),
            "{what}: the lock's record changed"
        );
    }
}

#[test]
fn curved_pivot_chart_switch() {
    check_switch(&|b, _| b);
}

negative_control!(
    curved_pivot_chart_switch,
    "the pivot carried in chart coordinates across the switch",
    expected = "does not decode the pinned state",
    check_switch(&|b, a| View {
        pivot: a.pivot,
        ..b
    })
);

fn check_switch_slides(claim: bool) {
    let a = pivoted_lock();
    let record = a.anchor.expect("recorded");
    let rest = kernel_view(&simplex(), Window::SQUARE);
    let seg = Segment::locked_switch(&a, &rest, &switch_leaves(), phys).expect("the switch");
    assert_eq!(
        seg.family(),
        &engine::path::Family::Cut {
            label: engine::path::CutLabel::SwitchSlides
        },
        "not labelled as sliding"
    );
    let mut b = seg.b().clone();
    if claim {
        b.anchor = b.anchor.map(|x| Anchor { slid: false, ..x });
    }
    assert_eq!(
        b.anchor.map(|x| x.words),
        Some(record.words),
        "the lock's record changed"
    );
    let shown = state(anchor::state_at(&b, &switch_leaves(), pin(&b)).expect("a pixel")).ic;
    let near = anchor::nearest(&rest, &switch_leaves(), pin(&a), &record).expect("a nearest point");
    let (from_anchor, from_nearest) = (phys(&record.ic(), &shown), phys(&near.ic, &shown));
    println!(
        "curved_pivot_chart_switch_slides: the pin {from_anchor:.6} from the pinned state, {:.3} ε from the target's nearest point",
        from_nearest / f64::EPSILON
    );
    assert!(
        from_anchor > LOCK_TOLERANCE && from_nearest <= 1e-6,
        "the pinned state is claimed on a target that can't hold it"
    );
}

#[test]
fn curved_pivot_chart_switch_slides() {
    check_switch_slides(false);
}

negative_control!(
    curved_pivot_chart_switch_slides,
    "the pinned state claimed on a target that can't hold it",
    expected = "claimed on a target that can't hold it",
    check_switch_slides(true)
);

// ── saved_blend_tilt_slice ─────────────────────────────────────────────────────────────────────────────────────────
//
// Pass, on the gate's affine and nonlinear pairs, under a lock and without: the frame at `s = 0.4` saved (a frame record)
// and reopened, then tilted by 20° (each end its own tilt) and sliced by 0.1 of the slider span (each end in its own
// hidden coordinates; under a lock `B` re-held), decodes at every pixel of an 8² grid and at the pin to the live morph's
// states at `s = 0.4` after the same tilt and slice, bit for bit; without a lock each end has turned about its own
// centre (each end bit for bit its own tilt with no pin); and the tilted frame saved and reopened again equals itself,
// bytes and states.

use engine::path::{SliderSpan, LATENT_SPAN};

/// A frame `v` saved and reopened: written as a frame record and read back.
fn saved(v: &View) -> View {
    let record = FrameRecord {
        a: v.clone(),
        b: v.clone(),
        family: engine::path::Family::Commit,
        s_bits: 0,
        align: None,
        retime_hash: None,
        anchor_words: None,
    };
    FrameRecord::read(&record.to_bytes())
        .expect("the frame reads back")
        .a
}

/// The paused frame of a gate pair, locked or free.
fn paused(g: &Gate, locked: bool) -> View {
    match locked {
        true => {
            let (morph, _) =
                Segment::locked_blend(&g.a, &g.b, BlendPath::BlockLine, &leaves(), phys, 64)
                    .expect("a path");
            morph.frame(MID).expect("paused")
        }
        false => {
            let free = |v: &View| {
                let mut v = v.clone();
                v.sim.lock = engine::contract::sim_config::Lock {
                    locked: false,
                    z_locked: [0.0; 8],
                };
                v.pin = None;
                v.anchor = None;
                v.pivot = None;
                v
            };
            View::blend(
                Blend::new(free(&g.a), free(&g.b), BlendPath::BlockLine, MID).expect("a blend"),
            )
        }
    }
}

/// 0.1 of an end's slider span along its hidden `z[4]`: a latent end's slice, the mixed chart's slider.
fn tenth_slice(v: &View) -> Segment {
    match v.map {
        Map::Latent => Segment::slice(v, &e(4, 1.0), 0.1 * LATENT_SPAN).expect("a slice"),
        _ => {
            let span = mixed(v.sim.plane.z0)
                .slider_span(4)
                .expect("a hidden coordinate");
            Segment::control(v, 4, v.sim.plane.z0[4] + 0.1 * span).expect("a slider's move")
        }
    }
}

/// The tilt and the slice of a blend view, each to both ends.
fn tilt_and_slice(g: &Gate, v: &View, locked: bool) -> Result<View, PathError> {
    let ends = blend_of(v);
    let tilted = Segment::on_ends(v, &(g.tilt_a)(&ends.a), &(g.tilt_b)(&ends.b))?
        .b()
        .clone();
    let ends = blend_of(&tilted);
    let (sa, sb) = (tenth_slice(&ends.a), tenth_slice(&ends.b));
    Ok(match locked {
        true => Segment::locked_slice(&tilted, &sa, &sb, &leaves())?
            .b()
            .clone(),
        false => Segment::on_ends(&tilted, &sa, &sb)?.b().clone(),
    })
}

fn states_of(v: &View, at: [f64; 2]) -> Vec<[u64; 15]> {
    let mut p: Vec<[f64; 2]> = (0..64)
        .map(|k| {
            [
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            ]
        })
        .collect();
    p.push(at);
    p.iter()
        .map(|x| words(&state(anchor::state_at(v, &leaves(), *x).expect("a pixel")).ic))
        .collect()
}

fn check_saved_blend(
    edit: &dyn Fn(&Gate, &View, bool) -> Result<View, PathError>,
    turned_about: &dyn Fn(&Gate, &View) -> Segment,
) {
    for g in gates() {
        for locked in [true, false] {
            let what = format!("{}, {}", g.name, if locked { "locked" } else { "free" });
            let live = paused(&g, locked);
            let at = pin(&g.a);
            let reopened = saved(&live);
            assert_eq!(
                reopened.bytes(),
                live.bytes(),
                "{what}: the saved frame does not reopen as itself"
            );
            let want = tilt_and_slice(&g, &live, locked).expect("the live edit");
            let got = edit(&g, &reopened, locked)
                .unwrap_or_else(|e| panic!("{what}: the reopened frame refuses the tilt: {e:?}"));
            assert_eq!(
                states_of(&got, at),
                states_of(&want, at),
                "{what}: the reopened frame's states are not the live morph's"
            );
            let again = saved(&got);
            assert_eq!(
                again.bytes(),
                got.bytes(),
                "{what}: the tilted frame does not reopen as itself"
            );
            assert_eq!(
                states_of(&again, at),
                states_of(&got, at),
                "{what}: the tilted frame reopened decodes otherwise"
            );
            if !locked {
                // Each end turned about its own centre: the tilt keeps the state at that end's own centre pixel, bit
                // for bit, and the frame's own route tilts end B so.
                let ends = blend_of(&live);
                let routed = Segment::on_ends(&live, &(g.tilt_a)(&ends.a), &(g.tilt_b)(&ends.b))
                    .expect("a tilt");
                let turned = turned_about(&g, &ends.b).b().clone();
                let centre = |v: &View| {
                    words(&state(anchor::state_at(v, &leaves(), [0.5, 0.5]).expect("a pixel")).ic)
                };
                assert!(
                    blend_of(routed.b()).b.bytes() == (g.tilt_b)(&ends.b).b().bytes()
                        && centre(&turned) == centre(&ends.b)
                        && centre(&blend_of(routed.b()).a) == centre(&ends.a),
                    "{what}: end B has not turned about its own centre"
                );
            }
        }
    }
}

#[test]
fn saved_blend_tilt_slice() {
    check_saved_blend(&|g, v, locked| tilt_and_slice(g, v, locked), &|g, b| {
        (g.tilt_b)(b)
    });
}

negative_control!(
    saved_blend_tilt_slice,
    "a tilt of the blended state's own encode: the reopened frame tilted as one latent plane, the ends' mean",
    expected = "are not the live morph's",
    check_saved_blend(
        &|g, v, locked| {
            let ends = blend_of(v);
            let mean = |a: &Latent, b: &Latent| -> Latent { std::array::from_fn(|i| 0.5 * (a[i] + b[i])) };
            let (pa, pb) = (&ends.a.sim.plane, &ends.b.sim.plane);
            let one = View::latent(sim_of(&Plane { z0: mean(&pa.z0, &pb.z0), q1: mean(&pa.q1, &pb.q1), q2: mean(&pa.q2, &pb.q2) }));
            let _ = (g, locked);
            Ok(latent_tilt(&one, 0, 1, 5).b().clone())
        },
        &|g, b| (g.tilt_b)(b)
    )
);

#[test]
fn saved_blend_tilt_slice_refuses_nothing() {
    check_saved_blend(&|g, v, locked| tilt_and_slice(g, v, locked), &|g, b| {
        (g.tilt_b)(b)
    });
}

negative_control!(
    saved_blend_tilt_slice_refuses_nothing,
    "the reopened frame refusing the tilt, as a blend view did before its ends were reached",
    expected = "refuses the tilt",
    check_saved_blend(
        &|_, v, _| Segment::slice(v, &e(4, 1.0), 0.1).map(|s| s.b().clone()),
        &|g, b| (g.tilt_b)(b)
    )
);

#[test]
fn saved_blend_tilt_slice_own_centres() {
    check_saved_blend(&|g, v, locked| tilt_and_slice(g, v, locked), &|g, b| {
        (g.tilt_b)(b)
    });
}

negative_control!(
    saved_blend_tilt_slice_own_centres,
    "without a lock, both ends turned about one shared point: B's tilt about A's centre pixel's address",
    expected = "has not turned about its own centre",
    check_saved_blend(&|g, v, locked| tilt_and_slice(g, v, locked), &|g, b| {
        let shared = View { pin: Some(Pin::at(b, 0.3, 0.65)), ..b.clone() };
        (g.tilt_b)(&shared)
    })
);

// ── path_lock_round_trip_slides ────────────────────────────────────────────────────────────────────────────────────
//
// Pass, on a sliding pair (the affine lock to the mass simplex at rest, which holds no state with momenta): the gate's
// path, lock → morph to `s = 0.4` (the anchor sliding) → tilt mid-morph (end A its own tilt about the pin; the rest
// chart's end, untilted here, held as it is by a zoom of 0) → slice mid-morph (end A sliced, the rest chart's end placed
// at its nearest point to A's new pinned state) → morph back; the pin's state continuous along the morph by G8's
// criterion, and R1: the return to `s = 0` bitwise the direct tilt and slice of A about the pin.

type RestLeaves = (
    LatentLeaves<Defaults>,
    ChartLeaves<RestStart<MassSimplex<f64>, f64>, Defaults>,
);

fn rest_leaves() -> RestLeaves {
    (
        LatentLeaves::new(),
        ChartLeaves::new(simplex()).expect("a chart"),
    )
}

fn check_round_trip_slides(frame: &dyn Fn(&Segment, f64) -> View) {
    let a_plane = frame_plane(GATE_Z0, 0, 1, 0.5);
    let a = locked_latent(&a_plane, (0.3, 0.65));
    let at = pin(&a);
    let rest = kernel_view(&simplex(), Window::SQUARE);
    let (morph, end) =
        Segment::locked_blend(&a, &rest, BlendPath::BlockLine, &rest_leaves(), phys, 64)
            .expect("a locked path");
    assert!(
        matches!(end, LockedEnd::Slides { .. }),
        "the rest chart holds the anchor: {end:?}"
    );
    // The morph, by G8's criterion, up to the pause: frames read along [0, MID].
    let paused_track = |n: usize| -> Vec<[f64; 9]> {
        (0..=n)
            .map(|k| {
                let v = frame(&morph, MID * k as f64 / n as f64);
                Blocks::of(&state(anchor::state_at(&v, &rest_leaves(), at).expect("a pixel")).ic)
                    .vector()
            })
            .collect()
    };
    let steps: Vec<f64> = [16, 32, 64]
        .iter()
        .map(|n| largest_step(&paused_track(*n)))
        .collect();
    println!("path_lock_round_trip_slides: largest steps at 16, 32, 64 frames: {steps:?}");
    for w in steps.windows(2) {
        assert!(
            w[1] <= 0.6 * w[0],
            "the slide jumps: {} against {}",
            w[1],
            w[0]
        );
    }
    let v1 = morph.frame(MID).expect("paused");
    let ends = blend_of(&v1);
    let tilt_a = |v: &View| latent_tilt(v, 0, 1, 5);
    let slice_a = |v: &View| Segment::slice(v, &e(4, 1.0), 0.2).expect("a slice");
    let tilt = Segment::on_ends(
        &v1,
        &tilt_a(&ends.a),
        &Segment::zoom(&ends.b, 0.0).expect("a zoom of 0"),
    )
    .expect("the tilt mid-morph");
    let v2 = tilt.b().clone();
    let ends = blend_of(&v2);
    // The rest chart's end sliced in its own `z[4]`, a momentum control, which it holds at rest: its states do not move.
    let slice_b =
        Segment::control(&ends.b, 4, ends.b.sim.plane.z0[4] + 0.2).expect("a slider's move");
    let slice = Segment::locked_slice(&v2, &slice_a(&ends.a), &slice_b, &rest_leaves());
    let slice = match slice {
        Ok(s) => s,
        Err(e) => panic!("the slice mid-morph on a sliding pair: {e:?}"),
    };
    let back = Segment::share(slice.b(), 0.0).expect("the morph back");
    let direct = anchor::record(slice_a(tilt_a(&a).b()).b(), &rest_leaves()).expect("recorded");
    let mut probes: Vec<[f64; 2]> = (0..64)
        .map(|k| {
            [
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            ]
        })
        .collect();
    probes.push(at);
    for p in probes {
        let x = state(anchor::state_at(back.b(), &rest_leaves(), p).expect("a pixel")).ic;
        let y = state(anchor::state_at(&direct, &rest_leaves(), p).expect("a pixel")).ic;
        assert_eq!(
            words(&x),
            words(&y),
            "R1 fails on the sliding pair at {p:?}"
        );
    }
}

#[test]
fn path_lock_round_trip_slides() {
    check_round_trip_slides(&own);
}

negative_control!(
    path_lock_round_trip_slides,
    "a labelled jump on the sliding pair: A's frame until the pause, then the paused frame",
    expected = "the slide jumps",
    check_round_trip_slides(&|seg, s| match s < MID {
        true => seg.a().clone(),
        false => own(seg, s),
    })
);

// ── path_lock_tolerance_sweep ──────────────────────────────────────────────────────────────────────────────────────
//
// A measurement, the inexact holds the lock's tolerance must cover: over an 8² grid of pins, (1) a lock on the `(L_z, K)`
// chart, its target the latent chart re-centred on `z_locked` (lookup's inverse), and (2) a lock on the affine chart, its
// target the exponential shape sphere holding that pin's masses and momenta, re-centred by the re-solve; each target's
// own pin state against the pinned state in `‖·‖_phys`, its worst, median and 95th percentile printed. It asserts that
// each is within the tolerance in force, the measurement's own fixture check.

/// The exponential shape sphere holding the masses and momenta of the latent point `z`.
fn sphere_for(z: &Latent) -> kernel::chart::ShapeSphere<f64> {
    kernel::chart::ShapeSphere {
        z_mass: [z[6], z[7]],
        z_mom: [z[2], z[3], z[4], z[5]],
        ..exp_sphere()
    }
}

fn sweep(against: &dyn Fn(&Anchor) -> Ic<f64>) -> (Vec<f64>, Vec<f64>) {
    let grid: Vec<(f64, f64)> = (0..64)
        .map(|k| {
            (
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            )
        })
        .collect();
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let mut invariant = Vec::new();
    let mut sphere = Vec::new();
    let mut skipped = Vec::new();
    let mut sim = sim_of(&plane_b());
    sim.chart.window = Window::SQUARE;
    let lk = View::kernel(&invariant_lk(), sim.clone()).expect("a registry chart");
    for &(s, t) in &grid {
        // (1) The (L_z, K) chart to latent; a pin whose lock is refused (not a system there) is counted.
        let locked =
            Segment::lock_on_chart::<Defaults, _, _>(&lk, &invariant_lk(), (s, t), phys, &tol);
        if locked.is_err() {
            skipped.push((s, t));
        }
        if let Ok((seg, _)) = locked {
            let a = anchor::record(seg.b(), &leaves()).expect("recorded");
            let b = View::latent(sim_of(&plane_b()));
            let (z0, _) = engine::path::hold_plane(&b.sim.plane, [s, t], &a.sim.lock.z_locked);
            let mut held = b.clone();
            held.sim.plane.z0 = z0;
            let shown = leaves().decode(&held, [s, t]).expect("a decode").ic;
            invariant.push(phys(&against(&a.anchor.expect("recorded")), &shown));
        }
        // (2) Latent to the exponential shape sphere.
        let a = locked_latent(&plane_a(), (s, t));
        let z = engine::lock::point(&plane_a(), s, t);
        let leaves = (
            LatentLeaves::<Defaults>::new(),
            ChartLeaves::<_, Defaults>::new(sphere_for(&z)).expect("a chart"),
        );
        let mut sim = sim_of(&plane_a());
        sim.chart.window = Window::SQUARE;
        let b = View::kernel(&sphere_for(&z), sim).expect("a registry chart");
        let anchor = a.anchor.expect("recorded");
        let found = anchor::nearest(&b, &leaves, [s, t], &anchor).expect("a nearest point");
        sphere.push(phys(&against(&anchor), &found.ic));
    }
    println!(
        "path_lock_tolerance_sweep: (L_z, K) pins whose lock is refused: {} {skipped:?}",
        skipped.len()
    );
    (invariant, sphere)
}

fn check_sweep(against: &dyn Fn(&Anchor) -> Ic<f64>) {
    let (invariant, sphere) = sweep(against);
    for (name, xs) in [
        ("(L_z, K) to latent", &invariant),
        ("latent to the exponential sphere", &sphere),
    ] {
        let (worst, median, p95) = summary(xs);
        println!(
            "path_lock_tolerance_sweep: {name}: {} pins, the target's own pin: worst {worst:.3} ε, median {median:.3} ε, 95th percentile {p95:.3} ε",
            xs.len()
        );
        assert!(
            xs.iter().all(|x| *x <= LOCK_TOLERANCE),
            "{name}: a target's own pin is past the lock's tolerance: worst {worst:.3} ε"
        );
    }
}

#[test]
fn path_lock_tolerance_sweep() {
    check_sweep(&|a| a.ic());
}

negative_control!(
    path_lock_tolerance_sweep,
    "the targets read against another state, the anchor of a lock elsewhere",
    expected = "past the lock's tolerance",
    check_sweep(&|_| locked_latent(&plane_a(), (0.7, 0.2))
        .anchor
        .expect("recorded")
        .ic())
);

// ── path_lock_round_trip_returns ───────────────────────────────────────────────────────────────────────────────────
//
// Pass (R1 on its own): the gate's return to `s = 0`, at every pixel of an 8² grid and at the pin, bitwise the direct tilt
// and slice of A about the pin; read through `back`, which the control replaces.

fn check_returns(back: &dyn Fn(&Segment) -> View) {
    for g in gates() {
        let run = run(&g);
        let returned = back(&run.segments[3].1);
        let direct = (g.slice_a)((g.tilt_a)(&g.a).b()).b().clone();
        let direct = anchor::record(&direct, &leaves()).expect("recorded");
        let mut probes: Vec<[f64; 2]> = (0..64)
            .map(|k| {
                [
                    (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                    (k / 8) as f64 / 8.0 + 1.0 / 16.0,
                ]
            })
            .collect();
        probes.push(pin(&g.a));
        for p in probes {
            let x = state(anchor::state_at(&returned, &leaves(), p).expect("a pixel")).ic;
            let y = state(anchor::state_at(&direct, &leaves(), p).expect("a pixel")).ic;
            assert_eq!(
                words(&x),
                words(&y),
                "{}: R1 fails: the return differs from the direct edit at {p:?}",
                g.name
            );
        }
    }
}

#[test]
fn path_lock_round_trip_returns() {
    check_returns(&|seg| seg.b().clone());
}

negative_control!(
    path_lock_round_trip_returns,
    "the return read just off s = 0, at a share of 2⁻²⁰",
    expected = "R1 fails",
    check_returns(&|seg| View::blend(blend_of(seg.b()).at(2f64.powi(-20)).expect("a share")))
);

// ── path_lock_centre_near_ends ─────────────────────────────────────────────────────────────────────────────────────
//
// Pass, on the hold pairs with the pin off the centre and on swapped planes: off the pin, a locked frame just inside each
// end shows that end's own pixels: at every pixel of an 8² grid, the frame at `s = 2⁻²⁰` within `10⁻⁴` of A's state in
// `‖·‖_phys`, and at `s = 1 − 2⁻²⁰` within `10⁻⁴` of B's (re-centred) state; the offset form's transported frame and its
// remainder both meet the ends.

fn check_near_ends(form: &dyn Fn(View) -> View) {
    let z0 = [0.2, 0.9, 0.1, -0.1, 0.05, 0.1, 0.1, -0.1];
    let pairs = vec![
        Pair {
            name: "affine, pin off the centre",
            a: locked_latent(&plane_a(), (0.3, 0.65)),
            b: View::latent(sim_of(&plane_b())),
        },
        Pair {
            name: "swapped",
            a: locked_latent(
                &Plane {
                    z0,
                    q1: e(0, 0.6),
                    q2: e(1, 0.6),
                },
                (0.4, 0.6),
            ),
            b: View::latent(sim_of(&Plane {
                z0,
                q1: e(1, 0.6),
                q2: e(0, 0.6),
            })),
        },
    ];
    let h = 2f64.powi(-20);
    for p in pairs {
        let seg = held(&p);
        let at = pin(&p.a);
        let mut worst = 0.0f64;
        for k in 0..64 {
            let c = [
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            ];
            if c == at {
                continue;
            }
            for (s, end) in [(h, seg.a()), (1.0 - h, seg.b())] {
                let x = state(
                    anchor::state_at(&form(seg.frame(s).expect("a frame")), &leaves(), c)
                        .expect("a pixel"),
                )
                .ic;
                let y = state(anchor::state_at(end, &leaves(), c).expect("a pixel")).ic;
                worst = worst.max(phys(&x, &y));
            }
        }
        println!(
            "path_lock_centre_near_ends: {}: the worst distance to the nearer end {worst:e}",
            p.name
        );
        assert!(
            worst <= 1e-4,
            "{}: a frame just inside an end does not show that end: {worst:e}",
            p.name
        );
    }
}

#[test]
fn path_lock_centre_near_ends() {
    check_near_ends(&|v| v);
}

negative_control!(
    path_lock_centre_near_ends,
    "the transport run from B's frame to A's",
    expected = "does not show that end",
    check_near_ends(&|v| with_form(
        v,
        anchor::OffsetForm {
            reversed: true,
            ..Default::default()
        }
    ))
);

// ── path_lock_round_trip_slides_slice ──────────────────────────────────────────────────────────────────────────────
//
// Pass, on the sliding pair of `path_lock_round_trip_slides` (the affine lock to the mass simplex at rest), across the
// slice mid-morph: no frame of the slice segment is labelled as holding the pinned state (the rest chart can't hold a
// state with momenta), and the pin's state is continuous across the segment by G8's criterion (16, 32 and 64 frames),
// the slide continuing rather than snapping. Read through `form`, which the control replaces.

fn sliding_slice() -> (View, Segment) {
    let a = locked_latent(&frame_plane(GATE_Z0, 0, 1, 0.5), (0.3, 0.65));
    let rest = kernel_view(&simplex(), Window::SQUARE);
    let (morph, _) =
        Segment::locked_blend(&a, &rest, BlendPath::BlockLine, &rest_leaves(), phys, 64)
            .expect("a locked path");
    let v1 = morph.frame(MID).expect("paused");
    let ends = blend_of(&v1);
    let tilt = Segment::on_ends(
        &v1,
        &latent_tilt(&ends.a, 0, 1, 5),
        &Segment::zoom(&ends.b, 0.0).expect("a zoom of 0"),
    )
    .expect("the tilt mid-morph");
    let v2 = tilt.b().clone();
    let ends = blend_of(&v2);
    let slice_a = Segment::slice(&ends.a, &e(4, 1.0), 0.2).expect("a slice");
    let slice_b =
        Segment::control(&ends.b, 4, ends.b.sim.plane.z0[4] + 0.2).expect("a slider's move");
    let slice = Segment::locked_slice(&v2, &slice_a, &slice_b, &rest_leaves())
        .expect("the slice mid-morph");
    (a, slice)
}

fn check_slide_through_slice(form: &dyn Fn(View, f64) -> View) {
    let (a, slice) = sliding_slice();
    let at = pin(&a);
    let track = |n: usize| -> Vec<[f64; 9]> {
        (0..=n)
            .map(|k| {
                let u = k as f64 / n as f64;
                let v = form(slice.frame(u).expect("a frame"), u);
                let held = blend_of(&v).holds(&rest_leaves()).expect("a hold");
                assert!(
                    !held,
                    "a frame of the sliding pair's slice is labelled as holding the pinned state"
                );
                Blocks::of(&state(anchor::state_at(&v, &rest_leaves(), at).expect("a pixel")).ic)
                    .vector()
            })
            .collect()
    };
    let steps: Vec<f64> = [16, 32, 64]
        .iter()
        .map(|n| largest_step(&track(*n)))
        .collect();
    println!("path_lock_round_trip_slides_slice: largest steps at 16, 32, 64 frames: {steps:?}");
    for w in steps.windows(2) {
        assert!(
            w[1] <= 0.6 * w[0],
            "the slide jumps across the slice: {} against {}",
            w[1],
            w[0]
        );
    }
}

#[test]
fn path_lock_round_trip_slides_slice() {
    check_slide_through_slice(&|v, _| v);
}

negative_control!(
    path_lock_round_trip_slides_slice,
    "the slice's frames forced to hold the pinned state, as the re-hold did before it checked",
    expected = "labelled as holding",
    check_slide_through_slice(&|v, _| with_form(
        v,
        anchor::OffsetForm {
            forced_hold: true,
            ..Default::default()
        }
    ))
);

// ── path_lock_centre_slide_nearest ─────────────────────────────────────────────────────────────────────────────────
//
// `path_lock_centre_slide`'s check again, to host the control of its `s = 1` assert: the end placed at another chart
// point than B's nearest, which must fail it.

#[test]
fn path_lock_centre_slide_nearest() {
    check_slide(&own);
}

negative_control!(
    path_lock_centre_slide_nearest,
    "s = 1 placed at another chart point of B, 0.1 of the window from its nearest point",
    expected = "is not B's nearest point",
    check_slide(&|seg, s| match s == 1.0 {
        true => {
            let b = seg.b();
            let at = pin(b);
            let x = b.sim.chart.window.at(at[0], at[1]);
            let h = b.sim.chart.window.half;
            anchor::place(b, at, [x[0] + 0.1 * h[0], x[1] + 0.1 * h[1]])
        }
        false => own(seg, s),
    })
);

// ── path_lock_centre_latent_hold_checked ───────────────────────────────────────────────────────────────────────────
//
// Pass (physics review 6017434260, blocking 1): a lock on the mixed chart, a slider's move (an excursion, which leaves
// `z_locked` stale), recorded again, then a locked morph to a latent target: either the path is refused, or, where it is
// labelled as holding, the target's own decode at the pin (no select) is within the lock's tolerance of the stored
// anchor in `‖·‖_phys`. The check runs on what `outcome` gives; the control gives the old behaviour, the target re-centred
// on the stale `z_locked` and labelled as holding.

/// A locked morph's outcome: its end `B` and how it met the anchor.
type Outcome = Result<(View, LockedEnd), PathError>;

fn check_latent_hold(outcome: &dyn Fn(&View, &View) -> Outcome) {
    let a = locked_mixed(home(), (0.35, 0.6));
    let moved = Segment::control(&a, 4, a.sim.plane.z0[4] + 0.2)
        .expect("a slider's move")
        .b()
        .clone();
    let a = anchor::record(&moved, &leaves()).expect("recorded again");
    let target = View::latent(sim_of(&plane_b()));
    match outcome(&a, &target) {
        Ok((b, LockedEnd::Holds)) => {
            let at = pin(&a);
            let raw = leaves()
                .decode(&View { anchor: None, ..b }, at)
                .expect("a decode")
                .ic;
            let miss = phys(&a.anchor.expect("recorded").ic(), &raw);
            assert!(
                miss <= LOCK_TOLERANCE,
                "labelled Holds while the target's own pin is {miss:e} from the pinned state"
            );
        }
        Ok((_, end)) => println!("path_lock_centre_latent_hold_checked: {end:?}"),
        Err(PathError::Refused(why)) => {
            println!("path_lock_centre_latent_hold_checked: refused: {why}");
            assert_eq!(
                why, "the latent end does not hold the anchor's latent point",
                "refused for another reason"
            );
        }
        Err(e) => panic!("the locked morph failed otherwise than by refusing the hold: {e:?}"),
    }
}

#[test]
fn path_lock_centre_latent_hold_checked() {
    check_latent_hold(&|a, b| {
        Segment::locked_blend(a, b, BlendPath::BlockLine, &leaves(), phys, 64)
            .map(|(seg, end)| (seg.b().clone(), end))
    });
}

negative_control!(
    path_lock_centre_latent_hold_checked,
    "the old check, read through the select: the target re-centred on the stale z_locked and labelled as holding",
    expected = "labelled Holds while",
    check_latent_hold(&|a, b| {
        let at = pin(a);
        let mut out = b.clone();
        out.sim.plane.z0 = engine::path::hold_plane(&b.sim.plane, at, &a.sim.lock.z_locked).0;
        out.anchor = a.anchor;
        out.sim.lock = a.sim.lock.clone();
        out.pin = a.pin;
        Ok((out, LockedEnd::Holds))
    })
);

// ── path_lock_share_from_slice ─────────────────────────────────────────────────────────────────────────────────────
//
// Pass (code review 6017789955, blocking 1): on the sliding pair's slice, a morph resumed from a frame inside it (`s` of
// the slice 0.5, resumed to a share of 0.9) shows the same states whether or not that frame was drawn first, bitwise at
// the pin on the share's frames u = 0.25, 0.5 and 1; and a share frame saved and reopened shows the live one's state at
// the pin bitwise. `at` builds the blend at a share; the control keeps the drawn frame's cached re-solve.

fn check_share_from_slice(at: &dyn Fn(&Blend, f64) -> Blend) {
    let (a, slice) = sliding_slice();
    let pin_at = pin(&a);
    let shown = slice.frame(0.5).expect("a frame");
    state(anchor::state_at(&shown, &rest_leaves(), pin_at).expect("drawn"));
    let fresh = slice.frame(0.5).expect("a frame");
    let words_at =
        |v: &View| words(&state(anchor::state_at(v, &rest_leaves(), pin_at).expect("a pixel")).ic);
    for u in [0.25, 0.5, 1.0] {
        let share = 0.4 + u * (0.9 - 0.4);
        let live = View::blend(at(blend_of(&shown), share));
        let cold = View::blend(at(blend_of(&fresh), share));
        assert_eq!(
            words_at(&live),
            words_at(&cold),
            "a resumed share differs by whether its frame was drawn first, u = {u}"
        );
        assert_eq!(
            words_at(&saved(&live)),
            words_at(&live),
            "a saved share frame reopens otherwise, u = {u}"
        );
    }
}

#[test]
fn path_lock_share_from_slice() {
    check_share_from_slice(&|b, s| b.at(s).expect("a share"));
}

negative_control!(
    path_lock_share_from_slice,
    "the drawn frame's cached re-solve kept across the share",
    expected = "differs by whether its frame was drawn first",
    check_share_from_slice(&|b, s| b.at_keeping_cache(s).expect("a share"))
);

// ── path_lock_round_trip_slides_slice_continuous ───────────────────────────────────────────────────────────────────
//
// `path_lock_round_trip_slides_slice`'s check again, to host a control that reaches its continuity assert alone: one mid
// frame of the slice with B snapped elsewhere (0.2 of the window from its nearest point), unlabelled as holding. (A snap
// to the nearest point itself measured no jump here: at the share 0.4 it is near the slide's own placement.)

#[test]
fn path_lock_round_trip_slides_slice_continuous() {
    check_slide_through_slice(&|v, _| v);
}

negative_control!(
    path_lock_round_trip_slides_slice_continuous,
    "an unlabelled snap of B at one mid frame of the slice, no hold",
    expected = "the slide jumps across the slice",
    check_slide_through_slice(&|v, u| {
        if u != 0.5 {
            return v;
        }
        let b = blend_of(&v);
        let (a_end, b_end) = (b.a.clone(), b.b.clone());
        let at = pin(&a_end);
        let shown = state(anchor::state_at(&a_end, &rest_leaves(), at).expect("a pixel")).ic;
        let near = anchor::nearest(&b_end, &rest_leaves(), at, &Anchor::of(&shown, false))
            .expect("a nearest point");
        let h = b_end.sim.chart.window.half;
        let snapped = anchor::placed(&b_end, at, [near.x[0] + 0.2 * h[0], near.x[1] + 0.2 * h[1]]);
        View::blend(b.with_ends(a_end, snapped).reholding(false))
    })
);

// ── path_lock_tolerance_window: the lock's tolerance over the whole window on the gate's pairs ────────────────────────
//
// A measurement, the evidence the lock's tolerance is confirmed from. Its criterion, written before its first run:
//
// - **The proposal** is the smallest power of two, in units of `ε` (f64), at or above twice the worst drift measured
//   anywhere in the sweep ([`proposal`]). The test prints it; it does not set `LOCK_TOLERANCE`, which the owner confirms.
// - **No refusals a correct lock should hold:** on every pin, every pair runs through without an error, and a pair whose
//   target holds the pinned state holds it on every frame (its `Blend::holds` true, the select showing the stored words).
//   Every refusal is listed with its pin, its stage and its reason, and the test fails on any.
// - **The fixture check:** every drift is within the tolerance in force, as the gate test asks.
//
// The sweep: pins on a 16 × 16 grid of the window's addresses `k/15`, its corners and edges included; on each pair, the
// gate path at each pin (lock there, the morph to `s = 0.4`, the tilt mid-morph, the slice mid-morph, the morph back),
// every frame of each segment (`FRAMES` per segment) read at the pin. What is read:
//
// - **A frame that holds** (`Blend::holds`): the drift without the select, the frame's `block_line(pinned, B(pin), s)` of
//   the pinned state and `B`'s own decode at the pin (re-held, on a slice's frames), rebuilt into a state, against the
//   pinned state in `‖·‖_phys`; and `B`'s own decode at the pin against it (the target's own pin).
// - **A frame that slides:** the pin's shown state against the slide's prescribed state, `block_line(pinned, B_x(pin), s)`
//   with `B_x` the frame's placement of `B`, in `‖·‖_phys`. The frame's pin is built by that same formula, so this
//   reads 0 unless the pin's pixel is built otherwise; it is reported, not argued.
// - **An end** (`s = 0`, `A` itself): `A`'s own decode at the pin against the stored anchor.
//
// The pairs (the gate's G9 list): affine (latent to latent) and nonlinear (mixed chart to mixed chart), the gate test's
// own; latent to the exponential shape sphere; the mixed chart to the mass simplex at rest and to Burrau, both sliding.
// One extra row, the `(L_z, K)` chart to latent, the pair PR #59's worst number (3.509 ε) came from, runs the morph only:
// its latent end can't be re-held to a nonlinear start's pinned state (`anchor::rehold` refuses, `NotBuilt`), so it takes
// no slice, and its frames run the whole morph, `s` from 0 to 1.
//
// The plan is unclear here (which tilt and slice a kernel map's end takes); chose `path_lock_round_trip_slides`' own:
// a kernel map's end read through `ChartLeaves`, which refuses a tilted curved view, takes a zoom of 0 for its tilt and a
// slider's move of `z[4]` for its slice (its states do not move), so only `A` is tilted. On the sphere pair `A` is the
// latent frame `(e₀, e₂)`, tilted toward `z[5]` and sliced along `e₁`, a shape control: the plan is unclear here too;
// chose a slice the sphere can follow (it holds the masses and momenta fixed and moves only the shape), so the pair
// stays a hold pair through the slice and its re-held frames are measured as holds.

// ── The rerun with the re-hold fixed: its criteria, written before it ran ────────────────────────────────────────────
//
// The two fixes: the solver's residual target decoupled from `LOCK_TOLERANCE` (a tighter target of its own), and
// `anchor::rehold` deciding its hold in `‖·‖_phys` as `anchor::recentre` does. Main's run (PR #69): worst 65.825 ε, the
// sphere's re-held slice frames; 124 refusals, all mixed to Burrau. The criteria:
//
// - **C1, the solver target from its floor.** With the residual target at 0, the residual each hold solve stops at (the
//   re-hold's and the nearest point's, on the hold pairs at the sweep's pins) is its floor. The target is the smallest
//   power of two in `ε` at or above twice the worst floor. Pass: it is below `LOCK_TOLERANCE` (64 ε); if not, the target
//   goes to the owner, not into the code.
// - **C2, no hold lost.** At that target, no hold pair (affine, nonlinear, the sphere, `(L_z, K)` to latent) refuses or
//   has a frame that slides: a hold decided in `‖·‖_phys` must not turn a frame main held into a slide, since a sliding
//   frame reads 0 here and would hide it.
// - **C3, the worst and its distribution**, reported per pair, the target's own pin included. Every holding frame is now
//   within `LOCK_TOLERANCE` in `‖·‖_phys` by construction; the number that matters is how far under.
// - **C4, a fixed point.** `T` = the rule's output at 64. Rerun with `LOCK_TOLERANCE` set to `T` (a local edit of the
//   ledger, reverted, as PR #69 did). Pass: the rule gives `T` again there, and nothing the sweep holds at 64 refuses
//   or slides at `T`.
// - **C5, the slides unchanged.** The rest and Burrau pairs' refusal count and per-pin readings as on main, the
//   shared solver settings notwithstanding; any change is listed.
// - **C6, the Burrau refusals' cause.** Each ended slide's failure kind (the follow's continuity test or a probe,
//   `Ended`; or the frame's direct solve, `Stalled`, `IterationCap`, `IllConditioned`), its `s` and the first frame's
//   residual, tabulated. A correct end is a continuity ending whose `s` holds as the frame count doubles (within a
//   frame); an end that is a direct solve's failure, or whose `s` moves with the frame count, is a failed follow, a
//   defect.

/// The grid's side: pins at `k/(SWEEP_SIDE − 1)`, corners and edges included.
const SWEEP_SIDE: usize = 16;

/// The frames read per segment of the gate path. The plan is unclear here ("every frame of the morph", which has no
/// fixed count); chose the gate test's own check of 16 (17 frames, ends included) rather than its 64-frame measurement,
/// because 256 pins × six pairs at 64 would take over 20 minutes in a debug build (16 takes about 8½).
const SWEEP_FRAMES: usize = 16;

/// The criterion's rule: the smallest power of two, in units of `ε`, at or above twice `worst` (itself in `ε`); 1 where
/// the worst is 0.
fn proposal(worst: f64) -> f64 {
    let mut k = 1.0;
    while k < 2.0 * worst {
        k *= 2.0;
    }
    k
}

/// The pins: a `side × side` grid of the window's addresses, its corners and edges included.
fn sweep_pins(side: usize) -> Vec<[f64; 2]> {
    let at = |k: usize| k as f64 / (side - 1) as f64;
    (0..side * side)
        .map(|k| [at(k % side), at(k / side)])
        .collect()
}

/// What a pair's path ends in at one pin.
#[derive(Default)]
struct PinReading {
    /// The worst drift over the pin's frames, with its segment and `u`.
    worst: (f64, &'static str, f64),
    /// The target's own pin at its worst over the pin's holding frames, with its segment and `u`.
    own: (f64, &'static str, f64),
    /// The worst drift in each segment.
    segments: Vec<(&'static str, f64)>,
    /// Frames read that hold, and that slide.
    holds: usize,
    slides: usize,
}

impl PinReading {
    fn take(&mut self, drift: f64, own: f64, what: &'static str, u: f64) {
        if drift > self.worst.0 {
            self.worst = (drift, what, u);
        }
        if own > self.own.0 {
            self.own = (own, what, u);
        }
        match self.segments.iter_mut().find(|(w, _)| *w == what) {
            Some((_, x)) => *x = x.max(drift),
            None => self.segments.push((what, drift)),
        }
    }
}

/// One refusal: the pair, the pin, the stage and the reason.
type Refusal = (&'static str, [f64; 2], &'static str, String);

/// One gate step: an end's edit, fallible.
type EndEdit<'a> = &'a dyn Fn(&View) -> Result<Segment, PathError>;

/// One pair of the sweep at one pin: the locked start, the target, whether the target should hold, the ends' tilts and
/// slices (`None`: the morph only, over `[0, 1]`).
struct SweepCase<'a> {
    a: View,
    b: View,
    should_hold: bool,
    edits: Option<[EndEdit<'a>; 4]>,
}

/// One segment of the gate path: its name, the segment and how far it runs.
type GateSegment = (&'static str, Segment, f64);

/// The segments of the gate path at one pin, or the morph alone: `(name, segment, upto)`.
fn sweep_path<L: Leaves>(
    c: &SweepCase,
    leaves: &L,
) -> Result<(LockedEnd, Vec<GateSegment>), (&'static str, String)> {
    let (morph, end) = Segment::locked_blend(&c.a, &c.b, BlendPath::BlockLine, leaves, phys, 64)
        .map_err(|e| ("the locked morph", format!("{e:?}")))?;
    let Some([tilt_a, tilt_b, slice_a, slice_b]) = c.edits else {
        return Ok((end, vec![("morph", morph, 1.0)]));
    };
    if let LockedEnd::Ended { s } = end {
        if s < MID {
            return Err((
                "the locked morph",
                format!("ended at s = {s}, before the pause"),
            ));
        }
    }
    let err = |what: &'static str| move |e: PathError| (what, format!("{e:?}"));
    let v1 = morph.frame(MID).map_err(err("the pause"))?;
    let ends = blend_of(&v1);
    let tilt = Segment::on_ends(
        &v1,
        &tilt_a(&ends.a).map_err(err("A's tilt"))?,
        &tilt_b(&ends.b).map_err(err("B's tilt"))?,
    )
    .map_err(err("the tilt mid-morph"))?;
    let v2 = tilt.b().clone();
    let ends = blend_of(&v2);
    let slice = Segment::locked_slice(
        &v2,
        &slice_a(&ends.a).map_err(err("A's slice"))?,
        &slice_b(&ends.b).map_err(err("B's slice"))?,
        leaves,
    )
    .map_err(err("the slice mid-morph"))?;
    let back = Segment::share(slice.b(), 0.0).map_err(err("the morph back"))?;
    Ok((
        end,
        vec![
            ("morph", morph, MID),
            ("tilt", tilt, 1.0),
            ("slice", slice, 1.0),
            ("back", back, 1.0),
        ],
    ))
}

/// One frame read at the pin `at`, against `against` (the stored state the drift is read from, the control's switch):
/// `(holds, drift, the target's own pin)`; `None` on an end. A holding frame whose select does not show the pinned
/// state is an error.
fn sweep_frame<L: Leaves>(
    view: &View,
    leaves: &L,
    at: [f64; 2],
    against: &dyn Fn(Ic<f64>) -> Ic<f64>,
) -> Result<(bool, f64, f64), String> {
    let pixel = |v: &View| match anchor::state_at(v, leaves, at) {
        Ok(Pixel::State { decoded, .. }) => Ok(decoded.ic),
        Ok(Pixel::Blank(b)) => Err(format!("a blank pin: {b:?}")),
        Err(e) => Err(format!("{e:?}")),
    };
    let raw = |v: &View| {
        leaves
            .decode(v, at)
            .map(|d| d.ic)
            .map_err(|e| format!("{e:?}"))
    };
    let shown = pixel(view)?;
    let Map::Blend(blend) = &view.map else {
        // An end: its own decode at the pin against its stored anchor.
        let stored = view.anchor.map_or(shown, |x| x.ic());
        return Ok((true, phys(&against(stored), &raw(view)?), 0.0));
    };
    let pinned = match blend.lock().and_then(|l| l.anchor) {
        Some(x) => x.ic(),
        None => pixel(&blend.a)?,
    };
    let s = blend.s();
    if s == 0.0 || s == 1.0 {
        let end = blend.end(match s == 0.0 {
            true => engine::path::blend::End::A,
            false => engine::path::blend::End::B,
        });
        return Ok((true, phys(&against(pinned), &raw(end)?), 0.0));
    }
    let holds = blend.holds(leaves).map_err(|e| format!("{e:?}"))?;
    let b = match blend.rehold() {
        true => {
            anchor::rehold(&blend.a, &blend.b, leaves, phys, s)
                .map_err(|e| format!("{e:?}"))?
                .0
        }
        false => blend.b.clone(),
    };
    let own = raw(&b)?;
    let (line, _) = engine::path::blend::block_line(&Blocks::of(&pinned), &Blocks::of(&own), s);
    let line = engine::path::blend::state_of(&line).0.ic;
    match holds {
        true => {
            if words(&shown) != words(&pinned) {
                return Err("a holding frame's pin is not the pinned state".into());
            }
            Ok((
                true,
                phys(&against(pinned), &line),
                phys(&against(pinned), &own),
            ))
        }
        false => Ok((false, phys(&line, &shown), 0.0)),
    }
}

/// One pair at one pin: the reading, or the refusal.
fn sweep_case<L: Leaves>(
    c: &SweepCase,
    leaves: &L,
    at: [f64; 2],
    frames: usize,
    against: &dyn Fn(Ic<f64>) -> Ic<f64>,
) -> Result<PinReading, (&'static str, String)> {
    let (end, segments) = sweep_path(c, leaves)?;
    if c.should_hold && end != LockedEnd::Holds {
        return Err((
            "the locked morph",
            format!("the target does not hold: {end:?}"),
        ));
    }
    let mut r = PinReading::default();
    for (what, seg, upto) in &segments {
        for k in 0..=frames {
            let u = upto * k as f64 / frames as f64;
            let view = seg
                .frame(u)
                .map_err(|e| (*what, format!("frame u = {u}: {e:?}")))?;
            let (holds, drift, own) = sweep_frame(&view, leaves, at, against)
                .map_err(|e| (*what, format!("u = {u}: {e}")))?;
            // A non-finite reading is a refusal, never dropped: `phys` returns NaN where a state has one, and the
            // reductions below compare with `>`, which a NaN never passes.
            if !drift.is_finite() || !own.is_finite() {
                return Err((
                    *what,
                    format!("u = {u}: a non-finite drift ({drift}, the target's own pin {own})"),
                ));
            }
            if c.should_hold && !holds {
                return Err((*what, format!("u = {u}: a frame of a hold pair slides")));
            }
            match holds {
                true => r.holds += 1,
                false => r.slides += 1,
            }
            r.take(drift, own, what, u);
        }
    }
    Ok(r)
}

/// The sweep's pair names, in the table's order.
const SWEEP_PAIRS: [&str; 7] = [
    "affine (latent to latent)",
    "nonlinear (mixed to mixed)",
    "latent to the exponential sphere",
    "mixed to the rest chart (slides)",
    "mixed to Burrau (slides)",
    "(L_z, K) to latent (morph only)",
    "the control's: latent to the rest chart, asked to hold",
];

/// The mixed view through `window`, locked at `(s, t)` on the chart and recorded; the refusal's reason where it fails.
fn try_locked_mixed(window: Window, (s, t): (f64, f64)) -> Result<View, String> {
    let view = mixed_view(window);
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let (seg, _) =
        Segment::lock_on_chart::<Defaults, _, _>(&view, &mixed(FROZEN), (s, t), phys, &tol)
            .map_err(|e| format!("{e:?}"))?;
    anchor::record(seg.b(), &leaves()).map_err(|e| format!("{e:?}"))
}

/// The latent view of `plane`, locked at `(s, t)` and recorded; the refusal's reason where it fails.
fn try_locked_latent(plane: &Plane, (s, t): (f64, f64)) -> Result<View, String> {
    let view = View::latent(sim_of(plane));
    let locked = Segment::lock(&view, s, t).map_err(|e| format!("{e:?}"))?;
    anchor::record(locked.b(), &leaves()).map_err(|e| format!("{e:?}"))
}

/// The sweep over `pins` on every pair in `pairs` (indices into [`SWEEP_PAIRS`]): each pair's readings by pin, and the
/// refusals.
#[allow(clippy::type_complexity)]
fn tolerance_window(
    pins: &[[f64; 2]],
    pairs: &[usize],
    frames: usize,
    against: &dyn Fn(Ic<f64>) -> Ic<f64>,
) -> (Vec<Vec<([f64; 2], PinReading)>>, Vec<Refusal>) {
    let tilt_affine = |v: &View| {
        let spec = TiltSpec {
            tau1: 20.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 5,
            y: 5,
        };
        let tilt = LatentTilt {
            a: 0,
            b: 1,
            scale: 0.5,
            from: [0.0, 0.0, 0.0],
        };
        Segment::latent_tilt(v, &tilt, &spec, 0.0)
    };
    let tilt_affine_b = |v: &View| {
        let spec = TiltSpec {
            tau1: 20.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 6,
            y: 6,
        };
        let tilt = LatentTilt {
            a: 2,
            b: 3,
            scale: 0.5,
            from: [0.0, 0.0, 0.0],
        };
        Segment::latent_tilt(v, &tilt, &spec, 0.0)
    };
    let tilt_sphere_a = |v: &View| {
        let spec = TiltSpec {
            tau1: 20.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 5,
            y: 5,
        };
        let tilt = LatentTilt {
            a: 0,
            b: 2,
            scale: 0.5,
            from: [0.0, 0.0, 0.0],
        };
        Segment::latent_tilt(v, &tilt, &spec, 0.0)
    };
    let tilt_mixed = |v: &View| {
        let spec = TiltSpec {
            tau1: 20.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 1,
            y: 1,
        };
        Segment::curved_tilt(v, &mixed(v.sim.plane.z0), &spec)
    };
    let no_tilt = |v: &View| Segment::zoom(v, 0.0);
    let slice_e4 = |v: &View| Segment::slice(v, &e(4, 1.0), 0.2);
    let slice_e1 = |v: &View| Segment::slice(v, &e(1, 1.0), 0.2);
    let slider_z4 = |v: &View| Segment::control(v, 4, v.sim.plane.z0[4] + 0.2);
    let affine_a = frame_plane(GATE_Z0, 0, 1, 0.5);
    let affine_b = frame_plane([-0.3, 0.5, 0.2, 0.1, -0.2, 0.15, -0.4, 0.3], 2, 3, 0.5);
    let sphere_a = frame_plane(GATE_Z0, 0, 2, 0.5);
    let moved_of = |v: &View| {
        let mut b = v.clone();
        b.sim.chart.window = moved();
        b.pin = None;
        b.anchor = None;
        b.sim.lock.locked = false;
        b
    };
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let lk = {
        let mut sim = sim_of(&plane_b());
        sim.chart.window = Window::SQUARE;
        View::kernel(&invariant_lk(), sim).expect("a registry chart")
    };
    let rest = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(simplex()).expect("a chart"),
    );
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let mut readings: Vec<Vec<([f64; 2], PinReading)>> = pairs.iter().map(|_| Vec::new()).collect();
    let mut refusals: Vec<Refusal> = Vec::new();
    for &at in pins {
        let st = (at[0], at[1]);
        for (slot, &p) in pairs.iter().enumerate() {
            let name = SWEEP_PAIRS[p];
            let outcome = match p {
                0 => try_locked_latent(&affine_a, st)
                    .map_err(|e| ("the lock", e))
                    .and_then(|a| {
                        let c = SweepCase {
                            a,
                            b: View::latent(sim_of(&affine_b)),
                            should_hold: true,
                            edits: Some([&tilt_affine, &tilt_affine_b, &slice_e4, &slice_e4]),
                        };
                        sweep_case(&c, &leaves(), at, frames, against)
                    }),
                1 => try_locked_mixed(home(), st)
                    .map_err(|e| ("the lock", e))
                    .and_then(|a| {
                        let c = SweepCase {
                            b: moved_of(&a),
                            a,
                            should_hold: true,
                            edits: Some([&tilt_mixed, &tilt_mixed, &slider_z4, &slider_z4]),
                        };
                        sweep_case(&c, &leaves(), at, frames, against)
                    }),
                2 => try_locked_latent(&sphere_a, st)
                    .map_err(|e| ("the lock", e))
                    .and_then(|a| {
                        let sphere = sphere_for(&engine::lock::point(&sphere_a, at[0], at[1]));
                        let leaves = (
                            LatentLeaves::<Defaults>::new(),
                            ChartLeaves::<_, Defaults>::new(sphere).expect("a chart"),
                        );
                        let c = SweepCase {
                            a,
                            b: kernel_view(&sphere, Window::SQUARE),
                            should_hold: true,
                            edits: Some([&tilt_sphere_a, &no_tilt, &slice_e1, &slider_z4]),
                        };
                        sweep_case(&c, &leaves, at, frames, against)
                    }),
                3 | 4 => try_locked_mixed(home(), st)
                    .map_err(|e| ("the lock", e))
                    .and_then(|a| {
                        let edits: [EndEdit; 4] = [&tilt_mixed, &no_tilt, &slider_z4, &slider_z4];
                        match p {
                            3 => sweep_case(
                                &SweepCase {
                                    a,
                                    b: kernel_view(&simplex(), Window::SQUARE),
                                    should_hold: false,
                                    edits: Some(edits),
                                },
                                &rest,
                                at,
                                frames,
                                against,
                            ),
                            _ => sweep_case(
                                &SweepCase {
                                    a,
                                    b: kernel_view(&nu_k(), Window::SQUARE),
                                    should_hold: false,
                                    edits: Some(edits),
                                },
                                &burrau,
                                at,
                                frames,
                                against,
                            ),
                        }
                    }),
                6 => try_locked_latent(&affine_a, st)
                    .map_err(|e| ("the lock", e))
                    .and_then(|a| {
                        let c = SweepCase {
                            a,
                            b: kernel_view(&simplex(), Window::SQUARE),
                            should_hold: true,
                            edits: None,
                        };
                        sweep_case(&c, &rest_leaves(), at, frames, against)
                    }),
                _ => Segment::lock_on_chart::<Defaults, _, _>(&lk, &invariant_lk(), st, phys, &tol)
                    .map_err(|e| ("the lock", format!("{e:?}")))
                    .and_then(|(seg, _)| {
                        anchor::record(seg.b(), &leaves())
                            .map_err(|e| ("the lock", format!("{e:?}")))
                    })
                    .and_then(|a| {
                        let c = SweepCase {
                            a,
                            b: View::latent(sim_of(&plane_b())),
                            should_hold: true,
                            edits: None,
                        };
                        sweep_case(&c, &leaves(), at, frames, against)
                    }),
            };
            match outcome {
                Ok(r) => readings[slot].push((at, r)),
                Err((stage, why)) => refusals.push((name, at, stage, why)),
            }
        }
    }
    (readings, refusals)
}

/// The `q` quantile of `xs` (sorted in place), nearest rank.
fn quantile(xs: &mut [f64], q: f64) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[((xs.len() - 1) as f64 * q).round() as usize]
}

/// The sweep's check (the criterion above) on the `side × side` grid: prints the table and each pair's map of per-pin
/// worsts.
fn check_tolerance_window(
    side: usize,
    pairs: &[usize],
    frames: usize,
    against: &dyn Fn(Ic<f64>) -> Ic<f64>,
) {
    check_tolerance(&sweep_pins(side), side, pairs, frames, against)
}

/// The sweep's check on `pins`, the map drawn on the `side × side` grid (none where `side` is 0).
fn check_tolerance(
    pins: &[[f64; 2]],
    side: usize,
    pairs: &[usize],
    frames: usize,
    against: &dyn Fn(Ic<f64>) -> Ic<f64>,
) {
    let (readings, refusals) = tolerance_window(pins, pairs, frames, against);
    let eps = f64::EPSILON;
    let mut worst_anywhere = 0.0f64;
    println!(
        "path_lock_tolerance_window: {} pins, {frames} frames per segment; drift in ε (f64), per pin its worst over its frames",
        pins.len()
    );
    println!(
        "| pair | pins read | frames hold / slide | max | p99 | median | worst at (pin, segment, u) | by segment, max | target's own pin: max, p99, median | its max at |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    for (slot, &p) in pairs.iter().enumerate() {
        let rs = &readings[slot];
        if rs.is_empty() {
            println!("| {} | 0 | | | | | | | | |", SWEEP_PAIRS[p]);
            continue;
        }
        let mut per_pin: Vec<f64> = rs.iter().map(|(_, r)| r.worst.0 / eps).collect();
        let max = quantile(&mut per_pin, 1.0);
        let p99 = quantile(&mut per_pin, 0.99);
        let median = quantile(&mut per_pin, 0.5);
        let (at, r) = rs
            .iter()
            .max_by(|x, y| x.1.worst.0.total_cmp(&y.1.worst.0))
            .expect("a reading");
        let mut owns: Vec<f64> = rs.iter().map(|(_, r)| r.own.0 / eps).collect();
        let own = quantile(&mut owns, 1.0);
        let (own_at, own_r) = rs
            .iter()
            .max_by(|x, y| x.1.own.0.total_cmp(&y.1.own.0))
            .expect("a reading");
        let mut segs: Vec<(&str, f64)> = Vec::new();
        for (_, r) in rs {
            for &(w, x) in &r.segments {
                match segs.iter_mut().find(|(v, _)| *v == w) {
                    Some((_, y)) => *y = y.max(x),
                    None => segs.push((w, x)),
                }
            }
        }
        let segs: Vec<String> = segs
            .iter()
            .map(|(w, x)| format!("{w} {:.3}", x / eps))
            .collect();
        let (holds, slides) = rs
            .iter()
            .fold((0, 0), |(h, s), (_, r)| (h + r.holds, s + r.slides));
        println!(
            "| {} | {} | {holds} / {slides} | {max:.3} | {p99:.3} | {median:.3} | ({:.4}, {:.4}), {}, u = {:.4} | {} | {own:.3}, {:.3}, {:.3} | ({:.4}, {:.4}), {}, u = {:.4} |",
            SWEEP_PAIRS[p],
            rs.len(),
            at[0],
            at[1],
            r.worst.1,
            r.worst.2,
            segs.join("; "),
            quantile(&mut owns, 0.99),
            quantile(&mut owns, 0.5),
            own_at[0],
            own_at[1],
            own_r.own.1,
            own_r.own.2
        );
        worst_anywhere = worst_anywhere.max(max).max(own);
    }
    for (slot, &p) in pairs.iter().enumerate().filter(|_| side > 1) {
        println!(
            "path_lock_tolerance_window: {}: per pin, the worst of its drift and its target's own pin (ε), rows t = 1 down to 0, columns s = 0 to 1; '-' a refusal",
            SWEEP_PAIRS[p]
        );
        for row in (0..side).rev() {
            let line: Vec<String> = (0..side)
                .map(|col| {
                    let at = [
                        col as f64 / (side - 1) as f64,
                        row as f64 / (side - 1) as f64,
                    ];
                    readings[slot]
                        .iter()
                        .find(|(x, _)| *x == at)
                        .map_or("   -".into(), |(_, r)| {
                            format!("{:4.1}", r.worst.0.max(r.own.0) / eps)
                        })
                })
                .collect();
            println!("  {}", line.join(" "));
        }
    }
    println!(
        "path_lock_tolerance_window: the worst drift anywhere {worst_anywhere:.3} ε; the proposal (the smallest power of two at or above twice it) {} ε; the tolerance in force {} ε",
        proposal(worst_anywhere),
        LOCK_TOLERANCE / eps
    );
    println!("path_lock_tolerance_window: refusals: {}", refusals.len());
    for (pair, at, stage, why) in &refusals {
        println!("  {pair} at ({:.4}, {:.4}), {stage}: {why}", at[0], at[1]);
    }
    assert!(
        refusals.is_empty(),
        "the sweep refused {} locks a correct lock should hold, the first: {:?}",
        refusals.len(),
        refusals.first()
    );
    assert!(
        worst_anywhere * eps <= LOCK_TOLERANCE,
        "a drift of {worst_anywhere:.3} ε is past the lock's tolerance in force"
    );
}

// Pass: the criterion above, on the 16 × 16 grid, every pair. About 6 minutes in a debug build, so ignored in CI's
// shards; run it with `CARGO_PROFILE_TEST_DEBUG=0 cargo test -p engine --test path_lock -- path_lock_tolerance_window
// --exact --include-ignored --nocapture` (about 2 minutes with `--release`, the same bits). Its siblings below run in CI
// on the window's corners. It passes since the solver's polish is judged on `‖P r‖` (PR #79): no refusal (main: 124 on
// mixed to Burrau, failed follows), and the worst reading 34.244 ε, the sphere's re-held slice frames.
#[test]
#[ignore = "about 6 minutes: cargo test -p engine --test path_lock -- path_lock_tolerance_window --exact --include-ignored --nocapture"]
fn path_lock_tolerance_window() {
    check_tolerance_window(SWEEP_SIDE, &[0, 1, 2, 3, 4, 5], SWEEP_FRAMES, &|x| x);
}

negative_control!(
    path_lock_tolerance_window,
    "the drift read against another state: every stored state moved by 2⁻⁴⁰ in its first mass (on the window's \
     corners; Burrau left out, as when its followed minimiser ended before the pause there and was counted as a refusal \
     first)",
    expected = "past the lock's tolerance",
    check_tolerance_window(2, &[0, 1, 2, 3, 5], 4, &|mut x| {
        x.m[0] += 2f64.powi(-40);
        x
    })
);

// ── path_lock_tolerance_window_fine: the re-solving pairs at 64 frames ──────────────────────────────────────────────
//
// Criterion C7, added to the rerun's before it ran: 16 frames per segment gave lower bounds on the sphere's re-hold
// (PR #69: 65.8 ε at 16, 66.6 at 64), so the two hold pairs whose `B` re-solves (nonlinear; latent to the exponential
// sphere) are swept again at 64 frames per segment on the 16 × 16 pins. Pass: the sweep's own criterion (no refusal,
// every reading within the tolerance in force), and the rule's output there is the 16-frame sweep's (C4's `T`, 128).

#[test]
#[ignore = "about a minute: cargo test -p engine --test path_lock -- path_lock_tolerance_window_fine --exact --include-ignored --nocapture"]
fn path_lock_tolerance_window_fine() {
    check_tolerance_window(SWEEP_SIDE, &[1, 2], 64, &|x| x);
}

negative_control!(
    path_lock_tolerance_window_fine,
    "the drift read against another state: every stored state moved by 2⁻⁴⁰ in its first mass (on the window's corners)",
    expected = "past the lock's tolerance",
    check_tolerance_window(2, &[1, 2], 64, &|mut x| {
        x.m[0] += 2f64.powi(-40);
        x
    })
);

// ── path_lock_tolerance_window_refusals ────────────────────────────────────────────────────────────────────────────
//
// The sweep's refusal count on its own, to host a control that reaches it: the window's four corners on the three hold
// pairs of the gate path, 4 frames per segment. Pass: no refusal. Control: a target that can't hold (the rest chart)
// asked to hold, which must be counted.

#[test]
fn path_lock_tolerance_window_refusals() {
    check_tolerance_window(2, &[0, 1, 2], 4, &|x| x);
}

negative_control!(
    path_lock_tolerance_window_refusals,
    "a target that can't hold asked to hold: latent to the rest chart",
    expected = "a correct lock should hold",
    check_tolerance_window(2, &[0, 6], 4, &|x| x)
);

// ── path_lock_tolerance_rule ───────────────────────────────────────────────────────────────────────────────────────
//
// The criterion's rule at its boundaries. Pass: the smallest power of two at or above twice the worst: 0 → 1, 0.5 → 1,
// just above 0.5 → 2, PR #59's 3.509 → 8, 4 → 8, just above 4 → 16, 32 → 64. Control: four times the worst, PR #59's
// rule, which gives 16 at 3.509 (it first fails at 0.5, giving 2 for 1).

fn check_rule(rule: &dyn Fn(f64) -> f64) {
    let above = |x: f64| f64::from_bits(x.to_bits() + 1);
    for (worst, want) in [
        (0.0, 1.0),
        (0.5, 1.0),
        (above(0.5), 2.0),
        (3.509, 8.0),
        (4.0, 8.0),
        (above(4.0), 16.0),
        (32.0, 64.0),
    ] {
        assert_eq!(
            rule(worst),
            want,
            "the rule gives {} at a worst of {worst}",
            rule(worst)
        );
    }
}

#[test]
fn path_lock_tolerance_rule() {
    check_rule(&proposal);
}

negative_control!(
    path_lock_tolerance_rule,
    "PR #59's rule, four times the worst",
    expected = "the rule gives",
    check_rule(&|w| proposal(2.0 * w))
);

// ── path_lock_tolerance_window_finite ──────────────────────────────────────────────────────────────────────────────
//
// The sweep's non-finite readings on their own, to host a control that reaches them: the window's four corners on the
// three hold pairs, 4 frames per segment. Pass: every reading finite, no refusal. Control: every stored state read with a
// NaN first mass, whose drift `phys` returns as NaN; it must be counted as a refusal, not dropped by the reductions.

#[test]
fn path_lock_tolerance_window_finite() {
    check_tolerance_window(2, &[0, 1, 2], 4, &|x| x);
}

negative_control!(
    path_lock_tolerance_window_finite,
    "every stored state read with a NaN first mass",
    expected = "a non-finite drift",
    check_tolerance_window(2, &[0, 1, 2], 4, &|mut x| {
        x.m[0] = f64::NAN;
        x
    })
);

// ── path_lock_solve_floor: criterion C1's measurement ──────────────────────────────────────────────────────────────
//
// Criterion C1 above, measured: on the two hold pairs whose ends re-solve (nonlinear, mixed to mixed; latent to the
// exponential sphere) at the sweep's 16 × 16 pins and 16 frames per segment, each nearest-point and re-hold solve is
// run again from its own seeds with a residual target of 0 (`probe::Switches::floor`), and the residual it stops at is
// its floor. The path itself runs as on main (the old target, `LOCK_TOLERANCE`, and the old block-residual hold), so the
// same solves run as on main. A "hold solve" is one whose solve at the old target converged within `LOCK_TOLERANCE`:
// a solve whose target is off `B`'s image stops at the projection distance, which is no floor. Pass: the target the
// rule gives from the worst floor (the smallest power of two in `ε` at or above twice it, `proposal`) is below
// `LOCK_TOLERANCE`.

/// One hold solve under the floor switch: which, its residual at the old target, and its floor's report.
#[cfg(feature = "controls")]
type FloorSolve = (&'static str, f64, kernel::chart::lock_solve::Report<f64>);

/// One pair's solves at `pins` under the floor switch: each hold solve, the count left out and the refusals.
#[cfg(feature = "controls")]
fn solve_floors(pins: &[[f64; 2]], pair: usize) -> (Vec<FloorSolve>, usize, usize) {
    use engine::path::anchor::probe::{self, Switches};
    let switches = Switches {
        target: Some(LOCK_TOLERANCE),
        block_hold: true,
        floor: true,
        ..Switches::default()
    };
    let ((_, refusals), log) = probe::with(switches, || {
        tolerance_window(pins, &[pair], SWEEP_FRAMES, &|x| x)
    });
    let mut out = Vec::new();
    let mut left_out = 0;
    for w in log.windows(2) {
        let (solve, floor) = (&w[0], &w[1]);
        let paired = matches!(
            (solve.what, floor.what),
            ("nearest", "nearest floor") | ("rehold", "rehold floor")
        );
        if !paired {
            continue;
        }
        match solve.report.converged() && solve.report.residual <= LOCK_TOLERANCE {
            true => out.push((solve.what, solve.report.residual, floor.report)),
            false => left_out += 1,
        }
    }
    (out, left_out, refusals.len())
}

/// C1 at `pins`: the floor read where `floor`, else (the control) the residual each solve stopped at, at the old target.
#[cfg(feature = "controls")]
fn check_solve_floor(pins: &[[f64; 2]], floor: bool) {
    let eps = f64::EPSILON;
    let mut worst = 0.0f64;
    for pair in [1, 2] {
        let (solves, left_out, refusals) = solve_floors(pins, pair);
        for what in ["nearest", "rehold"] {
            let mut kinds: Vec<(String, usize, u32)> = Vec::new();
            for (_, _, r) in solves.iter().filter(|x| x.0 == what) {
                let k = format!("{:?}", r.status);
                match kinds.iter_mut().find(|(v, _, _)| *v == k) {
                    Some((_, n, it)) => {
                        *n += 1;
                        *it = (*it).max(r.iterations);
                    }
                    None => kinds.push((k, 1, r.iterations)),
                }
            }
            let worst = solves
                .iter()
                .filter(|x| x.0 == what)
                .max_by(|a, b| a.2.residual.total_cmp(&b.2.residual));
            println!(
                "path_lock_solve_floor: {}: {what}: the floor solves stop (status, count, most iterations): {kinds:?}; the worst floor's: {:?}",
                SWEEP_PAIRS[pair],
                worst.map(|x| (x.2.residual / eps, x.2.status, x.2.iterations, x.2.optimality))
            );
        }
        let solves: Vec<_> = solves
            .into_iter()
            .map(|(w, at, f)| (w, at, if floor { f.residual } else { at }))
            .collect();
        for what in ["nearest", "rehold"] {
            let mut floors: Vec<f64> = solves
                .iter()
                .filter(|x| x.0 == what)
                .map(|x| x.2 / eps)
                .collect();
            let mut in_force: Vec<f64> = solves
                .iter()
                .filter(|x| x.0 == what)
                .map(|x| x.1 / eps)
                .collect();
            if floors.is_empty() {
                println!(
                    "path_lock_solve_floor: {}: {what}: no hold solve",
                    SWEEP_PAIRS[pair]
                );
                continue;
            }
            assert!(
                floors.iter().all(|f| f.is_finite()),
                "a floor that is not finite"
            );
            let max = quantile(&mut floors, 1.0);
            println!(
                "path_lock_solve_floor: {}: {what}: {} hold solves; floor max {max:.3} ε, p99 {:.3}, median {:.3}; at the old target: max {:.3}, median {:.3}",
                SWEEP_PAIRS[pair],
                floors.len(),
                quantile(&mut floors, 0.99),
                quantile(&mut floors, 0.5),
                quantile(&mut in_force, 1.0),
                quantile(&mut in_force, 0.5),
            );
            worst = worst.max(max);
        }
        println!(
            "path_lock_solve_floor: {}: {left_out} solves left out (not within the tolerance at the old target); {refusals} refusals",
            SWEEP_PAIRS[pair]
        );
    }
    let target = proposal(worst);
    println!(
        "path_lock_solve_floor: the worst floor {worst:.3} ε; the target (the smallest power of two at or above twice it) {target} ε; the lock's tolerance {} ε",
        LOCK_TOLERANCE / eps
    );
    assert!(
        target * eps < LOCK_TOLERANCE,
        "the solver's target from its floor, {target} ε, is not below the lock's tolerance"
    );
}

#[cfg(feature = "controls")]
#[test]
#[ignore = "about 20 seconds: cargo test -p engine --features controls --test path_lock -- path_lock_solve_floor --exact --include-ignored --nocapture"]
fn path_lock_solve_floor() {
    check_solve_floor(&sweep_pins(SWEEP_SIDE), true);
}

negative_control!(
    path_lock_solve_floor,
    "the residual each solve stopped at, at the old target, read as its floor (main's stopping point), at the sphere's \
     worst pin (0.8, 0.6)",
    expected = "is not below the lock's tolerance",
    check_solve_floor(&[grid_pin(12, 9)], false)
);

// ── path_lock_burrau_ends: criterion C6's measurement ──────────────────────────────────────────────────────────────
//
// Criterion C6 above, measured: at each of the sweep's 16 × 16 pins, mixed to Burrau's locked morph at 64 frames (the
// sweep's track), and where its followed minimiser ends, again at 128 and 256. Per ended pin: the failure's kind at each
// frame count (the follow's own report: `Ended` is the continuity test or a probe inside the frame; `Stalled`,
// `IterationCap`, `IllConditioned`, `NonFinite` the frame's direct solve), its `s`, and the first frame's residual. A
// correct end is an `Ended` whose `s` holds within one frame of 64 as the frame count doubles; any other is a failed
// follow, a defect. A measurement: it prints the table and the count of each verdict, and fails only if a pin that
// ends at 64 has no failed follow in its log (the log would then not be the slide's).

/// The followed minimiser's end on mixed to Burrau at `at` over `frames` frames: `None` where it does not end, else the
/// failed follow's status and `s`, the first frame's residual, and the failed follow's optimality and residual.
#[cfg(feature = "controls")]
fn burrau_end(at: [f64; 2], frames: usize) -> Option<BurrauEnd> {
    burrau_end_logged(at, frames, true)
}

/// [`burrau_end`], the solves logged where `logged` (the control's switch: with no log, an end has no cause to read).
#[cfg(feature = "controls")]
fn burrau_end_logged(at: [f64; 2], frames: usize, logged: bool) -> Option<BurrauEnd> {
    use engine::path::anchor::probe::{self, Switches};
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let a = try_locked_mixed(home(), (at[0], at[1])).ok()?;
    let b = kernel_view(&nu_k(), Window::SQUARE);
    let run = || Segment::locked_blend(&a, &b, BlendPath::BlockLine, &burrau, phys, frames);
    // Main's slide, so the control's pin ends as on main.
    let main = main_slide();
    let (out, log) = match logged {
        true => probe::with(Switches::default(), run),
        false => (probe::with(main, run).0, Vec::new()),
    };
    let (_, end) = out.ok()?;
    let LockedEnd::Ended { .. } = end else {
        return None;
    };
    let first = log
        .iter()
        .find(|x| x.what == "slide first")
        .map_or(f64::NAN, |x| x.report.residual);
    let failed = log
        .iter()
        .find(|x| x.what == "follow" && !x.report.converged())
        .expect("an ended slide's failed follow in the log");
    Some((
        failed.report.status,
        failed.s,
        first,
        failed.report.optimality,
        failed.report.residual,
    ))
}

/// A Burrau end: the failed follow's status and `s`, the first frame's residual, the failed follow's optimality
/// (`‖P r‖/‖r‖`, against the spec's `10⁻⁸`) and residual.
#[cfg(feature = "controls")]
type BurrauEnd = (kernel::chart::lock_solve::Status, f64, f64, f64, f64);

#[cfg(feature = "controls")]
#[test]
#[ignore = "about a minute: cargo test -p engine --features controls --test path_lock -- path_lock_burrau_ends --exact --include-ignored --nocapture"]
fn path_lock_burrau_ends() {
    let eps = f64::EPSILON;
    let mut verdicts: Vec<(&str, usize)> = Vec::new();
    let mut kinds: Vec<(String, usize)> = Vec::new();
    let mut frames_at: Vec<(usize, usize)> = Vec::new();
    let mut optimality: Vec<f64> = Vec::new();
    println!("| pin | at 64: kind, s, optimality | at 128: kind, s, optimality | at 256: kind, s, optimality | first frame's residual (ε) | verdict |");
    println!("|---|---|---|---|---|---|");
    for at in sweep_pins(SWEEP_SIDE) {
        let Some(e64) = burrau_end(at, 64) else {
            continue;
        };
        let e128 = burrau_end(at, 128);
        let e256 = burrau_end(at, 256);
        let show = |e: &Option<BurrauEnd>| match e {
            Some((k, s, _, o, _)) => format!("{k:?}, {s:.6}, {o:.2e}"),
            None => "does not end".into(),
        };
        let ended = |e: &Option<BurrauEnd>| {
            matches!(
                e,
                Some((
                    kernel::chart::lock_solve::Status::Failed(
                        kernel::chart::lock_solve::Failure::Ended
                    ),
                    _,
                    _,
                    _,
                    _
                ))
            )
        };
        let holds =
            |e: &Option<BurrauEnd>| e.is_some_and(|(_, s, ..)| (s - e64.1).abs() <= 1.0 / 64.0);
        let verdict = match ended(&Some(e64)) && ended(&e128) && ended(&e256) {
            true if holds(&e128) && holds(&e256) => "a correct end",
            true => "a failed follow: its s moves with the frame count",
            false => "a failed follow: a direct solve's failure, or no end at a finer count",
        };
        println!(
            "| ({:.4}, {:.4}) | {} | {} | {} | {:.3} | {verdict} |",
            at[0],
            at[1],
            show(&Some(e64)),
            show(&e128),
            show(&e256),
            e64.2 / eps
        );
        match verdicts.iter_mut().find(|(v, _)| *v == verdict) {
            Some((_, n)) => *n += 1,
            None => verdicts.push((verdict, 1)),
        }
        let kind = format!("{:?}", e64.0);
        let frame = (e64.1 * 64.0).round() as usize;
        match frames_at.iter_mut().find(|(f, _)| *f == frame) {
            Some((_, n)) => *n += 1,
            None => frames_at.push((frame, 1)),
        }
        optimality.push(e64.3);
        match kinds.iter_mut().find(|(v, _)| *v == kind) {
            Some((_, n)) => *n += 1,
            None => kinds.push((kind, 1)),
        }
    }
    println!("path_lock_burrau_ends: by kind at 64 frames: {kinds:?}");
    println!("path_lock_burrau_ends: verdicts: {verdicts:?}");

    frames_at.sort();
    println!("path_lock_burrau_ends: the failed frame's index at 64 frames (k in s = k/64), and how many: {frames_at:?}");
    if !optimality.is_empty() {
        println!(
            "path_lock_burrau_ends: the failed follow's optimality at 64 frames: min {:.2e}, median {:.2e}, max {:.2e} (the spec's tolerance 1e-8)",
            quantile(&mut optimality, 0.0),
            quantile(&mut optimality, 0.5),
            quantile(&mut optimality, 1.0)
        );
    }
}

// ── path_lock_rehold_target: the solver's own target ───────────────────────────────────────────────────────────────
//
// The re-hold's solve stops at its own target (`anchor::SOLVE_TOLERANCE`, 32 ε from C1), not at the lock's tolerance,
// so a frame it holds is not held at the tolerance it is judged by. At the pin where main read its worst, latent to the
// exponential sphere at (0.8, 0.6) (65.825 ε, the slice's first step), the gate path at 16 frames per segment. Pass: no
// frame of the hold pair slides or is refused, and every reading is within `LOCK_TOLERANCE` in `‖·‖_phys`. Control:
// the solver's target set back to `LOCK_TOLERANCE` (main's), under which that pin's re-held frames stop short of the
// pinned state and, decided in `‖·‖_phys`, slide.

/// The sweep's pin `(k/15, j/15)`.
fn grid_pin(k: usize, j: usize) -> [f64; 2] {
    sweep_pins(SWEEP_SIDE)[j * SWEEP_SIDE + k]
}

#[test]
fn path_lock_rehold_target() {
    check_tolerance(&[grid_pin(12, 9)], 0, &[2], SWEEP_FRAMES, &|x| x);
}

negative_control!(
    path_lock_rehold_target,
    "the solver's residual target set back to the lock's tolerance, main's",
    expected = "a correct lock should hold",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            target: Some(LOCK_TOLERANCE),
            ..Switches::default()
        };
        probe::with(switches, || {
            check_tolerance(&[grid_pin(12, 9)], 0, &[2], SWEEP_FRAMES, &|x| x)
        });
    }
);

// ── path_lock_rehold_norm: the hold decided in the norm ────────────────────────────────────────────────────────────
//
// `anchor::rehold` decides its hold as `anchor::recentre` does: the re-placed `B`'s own state at the pin against `A`'s
// pinned state, in the norm it is given, against `LOCK_TOLERANCE`. The nonlinear pair (mixed to mixed) at (0.8, 0.6),
// the slice mid-morph at its middle frame. Pass: in `‖·‖_phys` it holds, its own state within the tolerance; in a norm
// that reads every state twice the tolerance further off, it does not hold (the frame slides). Control: the hold decided
// by the solver's block residual (main's rule), which never reads the norm and holds the frame either way.

/// The nonlinear pair's slice mid-morph at `at`: the gate path's third segment.
fn nonlinear_slice(at: [f64; 2]) -> Segment {
    let tilt_mixed = |v: &View| {
        let spec = TiltSpec {
            tau1: 20.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 1,
            y: 1,
        };
        Segment::curved_tilt(v, &mixed(v.sim.plane.z0), &spec)
    };
    let slider_z4 = |v: &View| Segment::control(v, 4, v.sim.plane.z0[4] + 0.2);
    let a = try_locked_mixed(home(), (at[0], at[1])).expect("the lock");
    let mut b = a.clone();
    b.sim.chart.window = moved();
    b.pin = None;
    b.anchor = None;
    b.sim.lock.locked = false;
    let c = SweepCase {
        a,
        b,
        should_hold: true,
        edits: Some([&tilt_mixed, &tilt_mixed, &slider_z4, &slider_z4]),
    };
    let (_, segments) = sweep_path(&c, &leaves()).expect("the gate path");
    segments
        .into_iter()
        .find(|(w, _, _)| *w == "slice")
        .expect("the slice")
        .1
}

fn check_rehold_norm() {
    let at = grid_pin(12, 9);
    let view = nonlinear_slice(at)
        .frame(0.5)
        .expect("the slice's middle frame");
    let blend = blend_of(&view);
    let s = blend.s();
    let (b, held) =
        anchor::rehold(&blend.a, &blend.b, &leaves(), phys, s).expect("the re-hold in ‖·‖_phys");
    let shown = match anchor::state_at(&blend.a, &leaves(), at).expect("A's pin") {
        Pixel::State { decoded, .. } => decoded.ic,
        Pixel::Blank(b) => panic!("A's pin is blank: {b:?}"),
    };
    let own = leaves().decode(&b, at).expect("B's own pin").ic;
    let d = phys(&shown, &own);
    println!(
        "path_lock_rehold_norm: s = {s}: held {held}, B's own pin {:.3} ε from A's",
        d / f64::EPSILON
    );
    assert!(held, "the re-hold does not hold a frame B can hold");
    assert!(
        d <= LOCK_TOLERANCE,
        "a held frame's own pin is {:.3} ε off",
        d / f64::EPSILON
    );
    let far = |x: &Ic<f64>, y: &Ic<f64>| phys(x, y) + 2.0 * LOCK_TOLERANCE;
    let (_, held) =
        anchor::rehold(&blend.a, &blend.b, &leaves(), far, s).expect("the re-hold's slide");
    assert!(!held, "the re-hold holds a frame its norm refuses");
}

/// The affine pair's slice mid-morph at `at` (latent to latent): the gate path's third segment.
fn affine_slice(at: [f64; 2]) -> Segment {
    let tilt = |a: u32, b: u32, x: u32| {
        move |v: &View| {
            let spec = TiltSpec {
                tau1: 20.0,
                tau2: 0.0,
                map: TiltMap::Sheet,
                x,
                y: x,
            };
            let tilt = LatentTilt {
                a,
                b,
                scale: 0.5,
                from: [0.0, 0.0, 0.0],
            };
            Segment::latent_tilt(v, &tilt, &spec, 0.0)
        }
    };
    let (tilt_a, tilt_b) = (tilt(0, 1, 5), tilt(2, 3, 6));
    let slice_e4 = |v: &View| Segment::slice(v, &e(4, 1.0), 0.2);
    let affine_a = frame_plane(GATE_Z0, 0, 1, 0.5);
    let affine_b = frame_plane([-0.3, 0.5, 0.2, 0.1, -0.2, 0.15, -0.4, 0.3], 2, 3, 0.5);
    let c = SweepCase {
        a: try_locked_latent(&affine_a, (at[0], at[1])).expect("the lock"),
        b: View::latent(sim_of(&affine_b)),
        should_hold: true,
        edits: Some([&tilt_a, &tilt_b, &slice_e4, &slice_e4]),
    };
    let (_, segments) = sweep_path(&c, &leaves()).expect("the gate path");
    segments
        .into_iter()
        .find(|(w, _, _)| *w == "slice")
        .expect("the slice")
        .1
}

/// The latent branch: a latent `B` held where its own pin is within the tolerance, refused where the norm says not.
fn check_rehold_norm_latent() {
    let view = affine_slice(grid_pin(12, 9))
        .frame(0.5)
        .expect("the slice's middle frame");
    let blend = blend_of(&view);
    let s = blend.s();
    let (_, held) = anchor::rehold(&blend.a, &blend.b, &leaves(), phys, s)
        .expect("the latent re-hold in ‖·‖_phys");
    assert!(held, "the latent re-hold does not hold a frame B can hold");
    let far = |x: &Ic<f64>, y: &Ic<f64>| phys(x, y) + 2.0 * LOCK_TOLERANCE;
    match anchor::rehold(&blend.a, &blend.b, &leaves(), far, s) {
        Err(PathError::Refused(_)) => {}
        other => panic!(
            "the latent re-hold holds a frame its norm refuses: {:?}",
            other.map(|x| x.1)
        ),
    }
}

#[test]
fn path_lock_rehold_norm() {
    check_rehold_norm();
    check_rehold_norm_latent();
}

negative_control!(
    path_lock_rehold_norm,
    "the hold decided by the solver's block residual, main's rule",
    expected = "the re-hold holds a frame its norm refuses",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            block_hold: true,
            ..Switches::default()
        };
        probe::with(switches, check_rehold_norm);
    }
);

// The latent branch on its own, under the same switch: main held a latent `B` unchecked.
#[test]
fn path_lock_rehold_norm_latent() {
    check_rehold_norm_latent();
}

negative_control!(
    path_lock_rehold_norm_latent,
    "a latent end held unchecked, main's rule",
    expected = "the latent re-hold holds a frame its norm refuses",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            block_hold: true,
            ..Switches::default()
        };
        probe::with(switches, check_rehold_norm_latent);
    }
);

negative_control!(
    path_lock_burrau_ends,
    "the solves not logged: an end at (0, 0), whose followed minimiser ends at 64 frames under main's slide, has no \
     failed follow to read",
    expected = "an ended slide's failed follow in the log",
    {
        burrau_end_logged([0.0, 0.0], 64, false);
    }
);

// ── The follow at a slide's first frames: criteria, written before the fix ran ───────────────────────────────────────
//
// PR #72's C6: on mixed to Burrau, 132 of the 256 pins' slides stop at 64 frames, every one a failed follow (71 the
// frame's direct solve `Stalled` at first-order optimality 1.4e-8 to 2.2e-7 against 1e-8, 61 `Ended` by the continuity
// test or a probe), and the failing frame moves toward the start as frames are added; 124 are refused by the sweep (115
// at the morph, 9 at the slice's re-hold, whose slide solve stalls the same way). `anchor::slide` reads any failed
// follow as the path's end.
//
// **The diagnosis** (`path_lock_slide_stall`, a measurement): one variable changed at a time from main's solve, the
// 132 pins at 64 frames, a per-pin table of each run (the stop's kind, frame, optimality, conditioning), each run's
// prediction written here before it ran.
//
// - **D0, main.** Prediction: the 132 of PR #72's C6, the same kinds and frames.
// - **D1, the residual's form.** The slide's frame state is `block_line(a, B_x, s)` and its target the anchor `a`, so
//   the solver forms `r = (a + s(B_x − a)) − a`: an absolute rounding of `ε|a|` against a residual of size `s·|B_x − a|`,
//   and central-difference slopes whose rounding is `ε|a|/h` against slopes of size `s·|∂B|`. Both relative errors grow
//   as `1/s`, so the solve's noise floor in `‖P r‖/‖r‖` grows as the frame nears `s = 0`, the stall's signature. The
//   variable: the residual formed as `block_line(a, B_x, s) − a` directly (`s(B_x − a)` on the linear blocks, the great
//   circle's offset `(cos sθ − 1)a + sin sθ·ŵ` with `cos sθ − 1 = −2 sin²(sθ/2)` on `n̂`), target 0: the same function in
//   exact arithmetic, with no cancellation. Prediction, if this is the cause: no `Stalled` follow is left, the `Ended`
//   at the 2nd to 6th frame are gone, and the remaining stops (if any) sit at a fixed `s`; if not the cause, the count
//   of stops within 10% of D0's.
// - **D2, the damping's ceiling** (`1e12` to `1e16`, main's residual form). Prediction: the stops unchanged within 10%,
//   because a stall here is the polish failing to lower `‖Jᵀr‖` (noise), not the damping running out.
// - **D3, the seed** (each follow seeded by the restart rule's point at its frame, not the last frame's point; main's
//   residual form). Prediction: the stalls unchanged within 10%: they are at the minimiser, seeded there or not.
// - **Read, not varied:** the optimality measure is already relative (`‖P r‖/‖r‖`, a sine, the same whatever the scale
//   of `r` or of either coordinate), so its scaling is not varied; `JᵀJ`'s conditioning at each stop is tabulated, to
//   see whether a null direction (as `s → 0`) is present.
//
// **D0–D3 read** (`path_lock_slide_stall`): D0 reproduces PR #72's C6 exactly (132 stops, 71 `Stalled`, 61 `Ended`, the
// same frames). D1 misses its prediction as the whole cause (121 stops, 74 `Stalled`, 47 `Ended`), but its stops no
// longer pile at the 2nd frame (9 there against 43) and its worst optimality falls from 2.22e-7 to 2.59e-8: the
// cancellation is the part that moves the stop toward `s = 0`. D2 changes nothing (132, bit for bit the same table).
// D3 has no stop, but it seeds each frame at the restart rule's own minimiser there, so the frame's direct solve does
// not move and the continuity test passes on a zero step: it shows a minimiser exists at every frame, and bypasses the
// test rather than passing it. Read, not varied: `JᵀJ`'s conditioning at the stops is 2.5e-7 to 1e-5, a near-null
// direction, through which the slopes' error reaches the optimality measure amplified by about `1/√conditioning`
// (`‖P r‖² = gᵀ(JᵀJ)⁻¹g`): central differences are good to about `ε^{2/3} ≈ 4e-11`, and `4e-11/√2.5e-7 ≈ 8e-8`, the
// stalls' range. Two more runs, one variable each, predictions written before they ran:
//
// - **D4, the slopes** (a five-point central difference at its own step `ε^{1/5}·h`, good to about `ε^{4/5} ≈ 3e-13`,
//   main's residual form). Prediction: the stops fall well below D0's, those left at the first frames (≤ 6), where
//   main's cancelling form still costs `1/s`.
//   *Changed before D4 ran in this test, after a scratch probe at three pins (reported in PR #79):* the step `ε^{1/5}·h`
//   failed the rest chart's slide at (0, 0), which main takes through, and measuring the slopes at the stops showed why
//   and what the stall's slope error is: the three-point difference at `ε^{1/3}·h` is off by `1.5e-7` (Burrau) and
//   `4e-8` (rest) relative, falling 4× per halving of the step, so truncation, not the `ε^{2/3}` rounding assumed; the
//   five-point difference at the same step `ε^{1/3}·h` agrees with itself to about `1e-11`, and a wider stencil meets
//   the rest chart's curvature. D4 and D5 take the five-point difference at `ε^{1/3}·h`; the predictions stand.
// - **D5, both** (D1's residual and D4's slopes). Prediction: no `Stalled`, and no `Ended` before the 7th frame; any
//   stop left is at a fixed `s`, for F2 to judge.
//
// **Added before any fix run was read** (the first F-sweep run was stopped unread when the suite showed this): the
// constructed ending (`path_lock_centre_slide_ends`), a true end at the edge of `B`'s states, arrives as the frame's
// direct solve stalling (`Stalled` at `s = 51/64`), not as `Ended`. So a direct solve's failure is classified by
// `anchor::leaves_states`: where the frame's Gauss–Newton step from the last frame's point leaves `B`'s states, the
// minimiser has run into their edge, an end; any other direct failure is `FollowFailed`. (Two first forms, an end
// wherever a bisection of the frame stepped toward the failure, with and without this test, were replaced before any
// sweep ran: the first read main's spurious stall at (0, 0) as an end in `path_lock_failed_follow`, and at the
// constructed edge no bisected half converges at all, so "stepped toward" picks the wrong one of the two.) F1 reads
// "no slide stops on `FollowFailed`"; F2 is judged on every stop whatever its tag, and also asks that the brackets
// `[k/n, (k+1)/n]` of the last good frame at 64, 128 and 256 share a point (an end drifting toward 0 by less than a
// frame per count fails it).
//
// **The fix's criteria** (on the cause D1–D3 find; the runs `path_lock_slide_follows_sweep` and the sweeps below):
//
// - **F1, no stall and no spurious continuity failure.** On mixed to Burrau at the sweep's 256 pins, at 64, 128 and 256
//   frames, no slide stops on a direct solve's failure (`Stalled`, `IterationCap`, `IllConditioned`, `NonFinite`).
// - **F2, every remaining stop a true end:** an `Ended` at 64, 128 and 256 frames alike whose `s` is within one frame of
//   the coarsest count's (`|s_n − s_64| ≤ 1/64`), and not at the 2nd frame of any count (an end there moves with the
//   frame count by construction, the old classifier's blind spot).
// - **F3, a failed follow is not an end.** A direct solve's failure in a follow is its own outcome, `PathError::
//   FollowFailed`, refused and named so, never `LockedEnd::Ended`.
// - **F4, the slides that went through before go through the same way.** Every pin of mixed to the rest chart and to
//   Burrau whose slide reached `s = 1` on main (`LockedEnd::Slides`) reaches it again with the same `not_nearest`, its
//   track's every node within the continuity test's agreement (`10⁻⁶·h`) of main's.
// - **F5, no hold lost:** #72's C2 (no hold pair refuses or slides, at 16 frames), C5 (the rest pair's refusals and
//   per-pin readings as on main) and C7 (the re-solving pairs at 64 frames) rerun, as written there.
// - **Reported:** how many of the 124 become pins the sweep reads through, true ends, or remaining failures; C1's floor
//   rerun and the sizing rule's value and its margins. `LOCK_TOLERANCE` (64) and `lock_solve_tolerance` (32) are not
//   changed here; proposed values go to the owner.
//
// **Read** (PR #79 carries the output): the cause is the solver's polish, judged on `‖Jᵀr‖` while convergence reads
// `‖P r‖` (M2; D6 no stop where D0 has 132). With it alone fixed, F1, F2 and F4 pass at 64, 128 and 256 frames (all 256
// Burrau pins go through, no end, every node within `1.85·10⁻⁷·h` of main's), and F5: the whole sweep refuses nothing,
// every other pair's row and per-pin map identical to main's. D1's and D4's changes are not the cause and are not taken
// (with them, F4 fails at 29 mirror ties, `path_lock_slide_first_choice`).

// ── path_lock_slide_stall: the diagnosis (D0–D3 above) ─────────────────────────────────────────────────────────────

/// Where a slide on mixed to Burrau stops: the frame count, the stop's frame index `k` (`s = k/frames`), the failed
/// solve's report, the residual's relative rounding there (`‖r_main − r‖/‖r‖`, main's cancelling form against the
/// offset formed directly) and main's slopes' relative error there (its three-point difference against a five-point one
/// of the offset formed directly), or `None` where the slide reaches `s = 1`.
#[cfg(feature = "controls")]
type SlideStop = Option<(usize, kernel::chart::lock_solve::Report<f64>, f64, f64)>;

/// Mixed to Burrau's locked morph at `at` over `frames` frames under `switches`: where its slide stops (any solve in the
/// slide that fails: the first frame's restart, a follow, or, read as main read it, a follow ending). `Err` where the
/// lock itself refuses.
#[cfg(feature = "controls")]
fn slide_stop(
    at: [f64; 2],
    frames: usize,
    switches: engine::path::anchor::probe::Switches,
) -> Result<SlideStop, String> {
    use engine::path::anchor::probe;
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let a = try_locked_mixed(home(), (at[0], at[1]))?;
    let b = kernel_view(&nu_k(), Window::SQUARE);
    let (_, log) = probe::with(switches, || {
        Segment::locked_blend(&a, &b, BlendPath::BlockLine, &burrau, phys, frames)
    });
    let Some(failed) = log
        .iter()
        .find(|x| matches!(x.what, "slide first" | "follow") && !x.report.converged())
    else {
        return Ok(None);
    };
    let k = (failed.s * frames as f64).round() as usize;
    // The residual at the stop, both ways: the frame's state less the anchor, and the offset formed directly.
    let anchor = a.anchor.expect("a recorded anchor").blocks();
    let placed = anchor::placed(&b, pin(&a), failed.report.x);
    let own = Blocks::of(
        &state(anchor::state_at(&placed, &burrau, pin(&a)).map_err(|e| format!("{e:?}"))?).ic,
    );
    let main: [f64; 9] = {
        let line = engine::path::blend::block_line(&anchor, &own, failed.s)
            .0
            .vector();
        let a = anchor.vector();
        std::array::from_fn(|i| line[i] - a[i])
    };
    let offset = engine::path::blend::line_offset(&anchor, &own, failed.s);
    let norm = |v: [f64; 9]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
    let rounding = norm(std::array::from_fn(|i| main[i] - offset[i])) / norm(offset);
    // The slopes' error at the stop: main's three-point difference of main's residual against a five-point one of the
    // offset formed directly, relative to the latter (`‖·‖` over both columns).
    let value = |x: [f64; 2], cancelling: bool| -> Option<[f64; 9]> {
        let v = anchor::placed(&b, pin(&a), x);
        let own = Blocks::of(&decoded_ic(anchor::state_at(&v, &burrau, pin(&a)).ok()?)?);
        Some(match cancelling {
            true => {
                let line = engine::path::blend::block_line(&anchor, &own, failed.s)
                    .0
                    .vector();
                let a = anchor.vector();
                std::array::from_fn(|i| line[i] - a[i])
            }
            false => engine::path::blend::line_offset(&anchor, &own, failed.s),
        })
    };
    let half = b.sim.chart.window.half[0]
        .abs()
        .max(b.sim.chart.window.half[1].abs());
    let x = failed.report.x;
    let slopes = |cancelling: bool, k: &[(f64, f64)], h: f64| -> Option<[[f64; 9]; 2]> {
        let mut out = [[0.0; 9]; 2];
        for (axis, col) in out.iter_mut().enumerate() {
            for &(at, w) in k {
                let mut p = x;
                p[axis] += at * h;
                let v = value(p, cancelling)?;
                for i in 0..9 {
                    col[i] += w * v[i] / h;
                }
            }
        }
        Some(out)
    };
    let three = [(1.0, 0.5), (-1.0, -0.5)];
    let five = [
        (-2.0, 1.0 / 12.0),
        (-1.0, -8.0 / 12.0),
        (1.0, 8.0 / 12.0),
        (2.0, -1.0 / 12.0),
    ];
    let slope_error = match (
        slopes(true, &three, f64::EPSILON.cbrt() * half),
        slopes(false, &five, f64::EPSILON.powf(0.2) * half),
    ) {
        (Some(j3), Some(j5)) => {
            let n =
                |j: &[[f64; 9]; 2]| (j[0].iter().chain(&j[1]).map(|x| x * x).sum::<f64>()).sqrt();
            let d: [[f64; 9]; 2] =
                std::array::from_fn(|c| std::array::from_fn(|i| j3[c][i] - j5[c][i]));
            n(&d) / n(&j5)
        }
        _ => f64::NAN,
    };
    Ok(Some((k, failed.report, rounding, slope_error)))
}

/// A pixel's state, `None` on a blank one.
#[cfg(feature = "controls")]
fn decoded_ic(pixel: Pixel) -> Option<Ic<f64>> {
    match pixel {
        Pixel::State { decoded, .. } => Some(decoded.ic),
        Pixel::Blank(_) => None,
    }
}

/// Main's slide, emulated through the probe: its residual and slopes, a failed follow read as an end, and `recentre`'s
/// hold on `placed_at`'s decode.
#[cfg(feature = "controls")]
fn main_slide() -> engine::path::anchor::probe::Switches {
    engine::path::anchor::probe::Switches {
        polish_unscaled: true,
        failed_is_end: true,
        placed_at_hold: true,
        ..Default::default()
    }
}

/// The diagnosis's premise: under `switches` the slide at `at` stops as PR #72 measured main's at (0, 0), its follow
/// stalling at the 2nd of 64 frames.
#[cfg(feature = "controls")]
fn check_reproduces(at: [f64; 2], switches: engine::path::anchor::probe::Switches) {
    let stop = slide_stop(at, 64, switches).expect("a lock");
    assert!(
        matches!(
            stop,
            Some((
                2,
                kernel::chart::lock_solve::Report {
                    status: kernel::chart::lock_solve::Status::Failed(
                        kernel::chart::lock_solve::Failure::Stalled
                    ),
                    ..
                },
                ..
            ))
        ),
        "main's stall not reproduced at ({}, {}): {stop:?}",
        at[0],
        at[1]
    );
}

// Its check, before the measurement: D0's switches reproduce PR #72's stall at (0, 0). Control: the slide as it is now,
// which goes through there.
#[cfg(feature = "controls")]
#[test]
#[ignore = "a few minutes: cargo test -p engine --features controls --test path_lock -- path_lock_slide_stall --exact --include-ignored --nocapture"]
fn path_lock_slide_stall() {
    use engine::path::anchor::probe::Switches;
    let main = main_slide();
    check_reproduces([0.0, 0.0], main);
    let runs: [(&str, Switches); 8] = [
        ("D0 main", main),
        (
            "D1 the offset formed directly",
            Switches {
                offset: true,
                ..main
            },
        ),
        (
            "D2 the damping's ceiling 1e16",
            Switches {
                ceiling: Some(1e16),
                ..main
            },
        ),
        (
            "D3 each follow seeded by the restart rule at its frame",
            Switches {
                restart_seed: true,
                ..main
            },
        ),
        (
            "D4 five-point slopes",
            Switches {
                five_point: true,
                ..main
            },
        ),
        (
            "D5 the offset formed directly and five-point slopes",
            Switches {
                offset: true,
                five_point: true,
                ..main
            },
        ),
        (
            "D6 the polish judged on ‖P r‖ alone (the fix)",
            Switches {
                polish_unscaled: false,
                ..main
            },
        ),
        (
            "D7 all three",
            Switches {
                offset: true,
                five_point: true,
                polish_unscaled: false,
                ..main
            },
        ),
    ];
    let pins = sweep_pins(SWEEP_SIDE);
    let mut table: Vec<([f64; 2], Vec<SlideStop>)> = Vec::new();
    for at in &pins {
        let row: Vec<SlideStop> = runs
            .iter()
            .map(|(_, sw)| slide_stop(*at, 64, *sw).unwrap_or(None))
            .collect();
        if row.iter().any(|x| x.is_some()) {
            table.push((*at, row));
        }
    }
    let cell = |x: &SlideStop| {
        match x {
        None => "reaches 1".to_string(),
        Some((k, r, rounding, slope)) => format!(
            "{:?} at k = {k}, opt {:.2e}, cond {:.2e}, it {}, r {:.2e}, rounding {:.1e}, slopes {:.1e} (/√cond {:.1e})",
            r.status,
            r.optimality,
            r.conditioning,
            r.iterations,
            r.residual,
            rounding,
            slope,
            slope / r.conditioning.sqrt()
        ),
    }
    };
    println!(
        "| pin | {} |",
        runs.iter().map(|r| r.0).collect::<Vec<_>>().join(" | ")
    );
    println!("|---|{}", "---|".repeat(runs.len()));
    for (at, row) in &table {
        println!(
            "| ({:.4}, {:.4}) | {} |",
            at[0],
            at[1],
            row.iter().map(cell).collect::<Vec<_>>().join(" | ")
        );
    }
    for (i, (name, _)) in runs.iter().enumerate() {
        let mut kinds: Vec<(String, usize)> = Vec::new();
        let mut frames: Vec<(usize, usize)> = Vec::new();
        let mut opt: Vec<f64> = Vec::new();
        let mut rounding: Vec<f64> = Vec::new();
        let mut cond: Vec<f64> = Vec::new();
        for (_, row) in &table {
            let Some((k, r, ro, _)) = &row[i] else {
                continue;
            };
            let kind = format!("{:?}", r.status);
            match kinds.iter_mut().find(|(v, _)| *v == kind) {
                Some((_, n)) => *n += 1,
                None => kinds.push((kind, 1)),
            }
            match frames.iter_mut().find(|(f, _)| f == k) {
                Some((_, n)) => *n += 1,
                None => frames.push((*k, 1)),
            }
            opt.push(r.optimality);
            rounding.push(*ro);
            cond.push(r.conditioning);
        }
        frames.sort();
        let stops = opt.len();
        println!(
            "path_lock_slide_stall: {name}: {stops} stops; by kind {kinds:?}; by frame {frames:?}"
        );
        if stops > 0 {
            println!(
                "path_lock_slide_stall: {name}: optimality min {:.2e}, median {:.2e}, max {:.2e}; the residual's rounding min {:.1e}, median {:.1e}, max {:.1e}; conditioning min {:.2e}, median {:.2e}",
                quantile(&mut opt, 0.0),
                quantile(&mut opt, 0.5),
                quantile(&mut opt, 1.0),
                quantile(&mut rounding, 0.0),
                quantile(&mut rounding, 0.5),
                quantile(&mut rounding, 1.0),
                quantile(&mut cond, 0.0),
                quantile(&mut cond, 0.5),
            );
        }
    }
}

// ── path_lock_solve_below_lock: the re-solve's target below the lock's tolerance ────────────────────────────────────
//
// `anchor::SOLVE_TOLERANCE < LOCK_TOLERANCE`, so a hold is never judged at the point its solve stopped (PR #72's
// review). The engine also asserts it at compile time, through the same `anchor::solve_below_lock`. Pass: it holds of
// the constants in force. Control: the target set to the lock's tolerance (main before PR #72) fails.

fn check_solve_below_lock(solve: f64) {
    assert!(
        anchor::solve_below_lock(solve, LOCK_TOLERANCE),
        "the re-solve's target {} ε is not below the lock's tolerance {} ε",
        solve / f64::EPSILON,
        LOCK_TOLERANCE / f64::EPSILON
    );
}

#[test]
fn path_lock_solve_below_lock() {
    check_solve_below_lock(anchor::SOLVE_TOLERANCE);
}

negative_control!(
    path_lock_solve_below_lock,
    "the re-solve's target set to the lock's tolerance, as before PR #72",
    expected = "is not below the lock's tolerance",
    check_solve_below_lock(LOCK_TOLERANCE)
);

// ── path_lock_recentre_place: recentre judges the state its view shows ──────────────────────────────────────────────
//
// `anchor::recentre` decides a kernel map's hold on the state the view it returns shows at the pin (`place`'s window,
// its own decode), as `rehold` does, not on the nearest-point solve's own decode through `placed_at`'s window, which
// differs from it by up to 7.3 ε on the sphere (PR #72). Latent to the exponential sphere, at the first of the sweep's
// pins (row by row from (0.8, 0.6)) where the two decodes differ in `‖·‖_phys` from the anchor: a norm that reads 0 at
// exactly the shown state's distance and ∞ elsewhere, so only a hold judged on the shown state holds. Pass: the end
// holds. Control: the hold judged on `placed_at`'s decode, main's, slides.

/// The sphere pair's leaves: latent and the exponential shape sphere.
type SphereLeaves = (
    LatentLeaves<Defaults>,
    ChartLeaves<kernel::chart::ShapeSphere<f64>, Defaults>,
);

/// The sphere pair at the first pin where `place`'s and `placed_at`'s decodes differ: the locked start, the end, the
/// leaves and the shown state's distance from the anchor.
fn recentre_case() -> (View, View, SphereLeaves, f64) {
    let sphere_a = frame_plane(GATE_Z0, 0, 2, 0.5);
    let mut pins = sweep_pins(SWEEP_SIDE);
    pins.rotate_left(9 * SWEEP_SIDE + 12);
    for at in pins {
        let Ok(a) = try_locked_latent(&sphere_a, (at[0], at[1])) else {
            continue;
        };
        let sphere = sphere_for(&engine::lock::point(&sphere_a, at[0], at[1]));
        let leaves = (
            LatentLeaves::<Defaults>::new(),
            ChartLeaves::<_, Defaults>::new(sphere).expect("a chart"),
        );
        let b = kernel_view(&sphere, Window::SQUARE);
        let anchor = a.anchor.expect("a recorded anchor");
        let Ok(found) = anchor::nearest(&b, &leaves, at, &anchor) else {
            continue;
        };
        let shown = View {
            anchor: None,
            ..anchor::place(&b, at, found.x)
        };
        let Ok(own) = leaves.decode(&shown, at) else {
            continue;
        };
        let (d_shown, d_solver) = (phys(&anchor.ic(), &own.ic), phys(&anchor.ic(), &found.ic));
        if d_shown != d_solver && d_shown <= LOCK_TOLERANCE {
            println!(
                "path_lock_recentre_place: at ({:.4}, {:.4}): the shown state {:.3} ε from the anchor, the solver's own decode {:.3} ε",
                at[0],
                at[1],
                d_shown / f64::EPSILON,
                d_solver / f64::EPSILON
            );
            return (a, b, leaves, d_shown);
        }
    }
    panic!("no pin where the two decodes differ");
}

fn check_recentre_place() {
    let (a, b, leaves, d_shown) = recentre_case();
    let only_shown = |x: &Ic<f64>, y: &Ic<f64>| match phys(x, y) == d_shown {
        true => 0.0,
        false => f64::INFINITY,
    };
    let out = anchor::recentre(&a, &b, &leaves, only_shown, 4);
    assert!(
        matches!(out, Ok(anchor::Recentred::Holds(_))),
        "recentre judged a state its view does not show: {:?}",
        out.map(|r| matches!(r, anchor::Recentred::Holds(_)))
    );
}

#[test]
fn path_lock_recentre_place() {
    check_recentre_place();
}

negative_control!(
    path_lock_recentre_place,
    "the hold judged on the nearest-point solve's own decode (placed_at's window), main's",
    expected = "recentre judged a state its view does not show",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            placed_at_hold: true,
            ..Switches::default()
        };
        probe::with(switches, check_recentre_place);
    }
);

// ── path_lock_failed_follow: a failed follow is refused, not read as an end ─────────────────────────────────────────
//
// Criterion F3. `anchor::slide` reads a follow whose frame's own solve fails (`Stalled`, `IterationCap`,
// `IllConditioned`, `NonFinite`) as the solver's failure, `PathError::FollowFailed`, never as the followed minimiser's
// end (`LockedEnd::Ended`, the continuity test's). Mixed to Burrau at (0, 0), 64 frames, its slide solved as main solved
// it (`main_slide`: its residual, its slopes and the polish's acceptance), under which its follow stalls at the 2nd frame
// (PR #72's C6). Pass: the
// locked morph is refused with `FollowFailed`. Control: a failed follow read as an end, main's, gives `Ended`.

#[cfg(feature = "controls")]
fn check_failed_follow(failed_is_end: bool) {
    use engine::path::anchor::probe::{self, Switches};
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let a = try_locked_mixed(home(), (0.0, 0.0)).expect("a lock");
    let b = kernel_view(&nu_k(), Window::SQUARE);
    let switches = Switches {
        failed_is_end,
        placed_at_hold: false,
        ..main_slide()
    };
    let (out, _) = probe::with(switches, || {
        Segment::locked_blend(&a, &b, BlendPath::BlockLine, &burrau, phys, 64)
    });
    let out = out.map(|(_, end)| end);
    println!("path_lock_failed_follow: {out:?}");
    assert!(
        !matches!(out, Ok(LockedEnd::Ended { .. })),
        "a failed follow read as an end: {out:?}"
    );
    assert!(
        matches!(out, Err(PathError::FollowFailed { .. })),
        "the failed follow not refused as one: {out:?}"
    );
}

#[cfg(feature = "controls")]
#[test]
fn path_lock_failed_follow() {
    check_failed_follow(false);
}

negative_control!(
    path_lock_failed_follow,
    "a failed follow read as the followed minimiser's end, main's",
    expected = "a failed follow read as an end",
    check_failed_follow(true)
);

// ── path_lock_rehold_slide_stall: the slice refusals' measurement ───────────────────────────────────────────────────
//
// PR #72's 9 refusals at the slice mid-morph on mixed to Burrau, measured then in a scratch run, committed here: at each
// of the sweep's 256 pins, the Burrau pair's gate path at 16 frames per segment, with the slide's residual formed as
// main formed it (and a failed follow read as an end, as main read it) and as it is formed now. Per pin refused at the
// slice: each `"rehold slide"` solve that failed, its `s`, status and optimality (main's run emulated through the probe,
// `main_slide`: `recentre`'s old hold decode included). A measurement: it fails only if a
// slice refusal has no failed re-hold slide solve in its log (the log would then not be the refusal's).

/// The Burrau pair's gate path at `at` under `switches`: the slice's refusal, if any, and the re-hold slide solves that
/// failed (`s`, status, optimality).
#[cfg(feature = "controls")]
#[allow(clippy::type_complexity)]
fn rehold_slide_stalls(
    at: [f64; 2],
    switches: engine::path::anchor::probe::Switches,
    logged: bool,
) -> (
    Option<String>,
    Vec<(f64, kernel::chart::lock_solve::Status, f64)>,
) {
    use engine::path::anchor::probe;
    let run = || tolerance_window(&[at], &[4], SWEEP_FRAMES, &|x| x).1;
    let (refusals, log) = match logged {
        true => probe::with(switches, run),
        false => (probe::with(switches, run).0, Vec::new()),
    };
    let slice = refusals
        .into_iter()
        .find(|r| matches!(r.2, "the slice mid-morph" | "slice"))
        .map(|r| r.3);
    let failed = log
        .iter()
        .filter(|x| x.what == "rehold slide" && !x.report.converged())
        .map(|x| (x.s, x.report.status, x.report.optimality))
        .collect();
    (slice, failed)
}

#[cfg(feature = "controls")]
fn check_rehold_slide_stall(pins: &[[f64; 2]], logged: bool) {
    use engine::path::anchor::probe::Switches;
    let main = main_slide();
    for (name, sw) in [
        ("main's slide", main),
        ("the slide as now", Switches::default()),
    ] {
        let mut refused = 0;
        let mut opt: Vec<f64> = Vec::new();
        for &at in pins {
            let (slice, failed) = rehold_slide_stalls(at, sw, logged);
            let Some(why) = slice else { continue };
            refused += 1;
            println!(
                "path_lock_rehold_slide_stall: {name}: ({:.4}, {:.4}) refused at the slice ({why}); failed re-hold slide solves (s, status, optimality): {failed:?}",
                at[0], at[1]
            );
            assert!(
                !failed.is_empty(),
                "a slice refusal with no failed re-hold slide solve in its log"
            );
            opt.extend(failed.iter().map(|f| f.2));
        }
        print!("path_lock_rehold_slide_stall: {name}: {refused} pins refused at the slice");
        match opt.is_empty() {
            true => println!(),
            false => println!(
                "; the failed solves' optimality min {:.2e}, max {:.2e} (the spec's 1e-8)",
                quantile(&mut opt, 0.0),
                quantile(&mut opt, 1.0)
            ),
        }
    }
}

#[cfg(feature = "controls")]
#[test]
#[ignore = "a few minutes: cargo test -p engine --features controls --test path_lock -- path_lock_rehold_slide_stall --exact --include-ignored --nocapture"]
fn path_lock_rehold_slide_stall() {
    check_rehold_slide_stall(&sweep_pins(SWEEP_SIDE), true);
}

negative_control!(
    path_lock_rehold_slide_stall,
    "the solves not logged, at (7/15, 4/5), refused at the slice under main's slide: its refusal has no failed solve to \
     read",
    expected = "a slice refusal with no failed re-hold slide solve in its log",
    check_rehold_slide_stall(&[grid_pin(7, 12)], false)
);

negative_control!(
    path_lock_slide_stall,
    "the slide as it is now in place of main's, at (0, 0), where it goes through",
    expected = "main's stall not reproduced",
    check_reproduces([0.0, 0.0], engine::path::anchor::probe::Switches::default())
);

// ── path_lock_slide_follows_sweep: criteria F1, F2 and F4 ────────────────────────────────────────────────────────────
//
// As written above, before the fix ran. At the sweep's 256 pins: mixed to Burrau re-centred at 64, 128 and 256 frames
// (F1: no follow fails; F2: every stop an `Ended` at all three counts, its last frame's `s` within 1/64 of the 64-frame
// count's and past the first frame at each), and mixed to the rest chart and to Burrau at 64 frames against main's
// residual and main's reading of a failed follow (F4: every slide that reached `s = 1` on main reaches it again, with
// the same `not_nearest` at `s = 1`, every node within `10⁻⁶·h` of main's). Prints the per-pin table and the tallies.

/// A slide's outcome at one pin: its track (ended or through), or the refusal.
#[cfg(feature = "controls")]
type SlideOutcome = Result<anchor::Track, PathError>;

/// Mixed locked at `at`, re-centred on `b` through `leaves` over `frames` frames: the slide's outcome (`None` where the
/// lock refuses or `b` holds).
#[cfg(feature = "controls")]
fn slide_outcome<L: Leaves>(
    at: [f64; 2],
    b: &View,
    leaves: &L,
    frames: usize,
) -> Option<SlideOutcome> {
    let a = try_locked_mixed(home(), (at[0], at[1])).ok()?;
    match anchor::recentre(&a, b, leaves, phys, frames) {
        Ok(anchor::Recentred::Slides { track, .. }) => Some(Ok(track)),
        Ok(anchor::Recentred::Holds(_)) => None,
        Err(e) => Some(Err(e)),
    }
}

#[cfg(feature = "controls")]
fn show_outcome(o: &Option<SlideOutcome>) -> String {
    match o {
        None => "holds or no lock".into(),
        Some(Ok(t)) if t.ended => format!("ends, last s = {:.6}", t.end()),
        Some(Ok(t)) => format!("through, not nearest {}", t.not_nearest(1.0)),
        Some(Err(e)) => format!("{e:?}"),
    }
}

#[cfg(feature = "controls")]
#[test]
#[ignore = "about 20 minutes: cargo test -p engine --features controls --test path_lock -- path_lock_slide_follows_sweep --exact --include-ignored --nocapture"]
fn path_lock_slide_follows_sweep() {
    check_slide_follows(&sweep_pins(SWEEP_SIDE));
}

// The same criteria with D7's three changes (the polish's, the offset formed directly and five-point slopes), the fix as
// it stood when this sweep first ran: a measurement, reported, not a criterion of the fix. It fails F4 at 29 pins, each
// a mirror tie (`path_lock_slide_first_choice`); the polish alone, the fix as it is, passes every criterion.
#[cfg(feature = "controls")]
#[test]
#[ignore = "a few minutes in release: cargo test --release -p engine --features controls --test path_lock -- path_lock_slide_follows_sweep_three --exact --include-ignored --nocapture"]
fn path_lock_slide_follows_sweep_three() {
    use engine::path::anchor::probe;
    probe::with(three_changes(), || {
        check_slide_follows(&sweep_pins(SWEEP_SIDE))
    });
}

/// D7's three changes: the polish's (in force), the slide's residual formed directly and its slopes five-point.
#[cfg(feature = "controls")]
fn three_changes() -> engine::path::anchor::probe::Switches {
    engine::path::anchor::probe::Switches {
        offset: true,
        five_point: true,
        ..Default::default()
    }
}

negative_control!(
    path_lock_slide_follows_sweep_three,
    "main's polish too, at (0, 0)",
    expected = "F1: 1 pins' follows fail",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            failed_is_end: false,
            ..main_slide()
        };
        probe::with(switches, || check_slide_follows(&[[0.0, 0.0]]));
    }
);

negative_control!(
    path_lock_slide_follows_sweep,
    "the slide as main formed it (its residual and slopes; a failed follow refused as now), at (0, 0)",
    expected = "F1: 1 pins' follows fail",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            failed_is_end: false,
            ..main_slide()
        };
        probe::with(switches, || check_slide_follows(&[[0.0, 0.0]]));
    }
);

/// F1, F2 and F4 at `pins`.
#[cfg(feature = "controls")]
fn check_slide_follows(pins: &[[f64; 2]]) {
    use engine::path::anchor::probe;
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let rest = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(simplex()).expect("a chart"),
    );
    let b_burrau = kernel_view(&nu_k(), Window::SQUARE);
    let b_rest = kernel_view(&simplex(), Window::SQUARE);
    let h = b_burrau.sim.chart.window.half[0]
        .abs()
        .max(b_burrau.sim.chart.window.half[1].abs());
    let h_rest = b_rest.sim.chart.window.half[0]
        .abs()
        .max(b_rest.sim.chart.window.half[1].abs());
    let main = main_slide();
    let mut f1: Vec<String> = Vec::new();
    let mut f2: Vec<String> = Vec::new();
    let mut f4: Vec<String> = Vec::new();
    let (mut through, mut ends, mut failed) = (0, 0, 0);
    let mut worst_node = 0.0f64;
    println!(
        "| pin | Burrau, main at 64 | at 64 | at 128 | at 256 | rest, main at 64 | rest at 64 |"
    );
    println!("|---|---|---|---|---|---|---|");
    for &at in pins {
        let (main_b, _) = probe::with(main, || slide_outcome(at, &b_burrau, &burrau, 64));
        let (main_r, _) = probe::with(main, || slide_outcome(at, &b_rest, &rest, 64));
        let now: Vec<Option<SlideOutcome>> = [64, 128, 256]
            .iter()
            .map(|&n| slide_outcome(at, &b_burrau, &burrau, n))
            .collect();
        let now_r = slide_outcome(at, &b_rest, &rest, 64);
        println!(
            "| ({:.4}, {:.4}) | {} | {} | {} | {} | {} | {} |",
            at[0],
            at[1],
            show_outcome(&main_b),
            show_outcome(&now[0]),
            show_outcome(&now[1]),
            show_outcome(&now[2]),
            show_outcome(&main_r),
            show_outcome(&now_r)
        );
        let pin_name = format!("({:.4}, {:.4})", at[0], at[1]);
        // F1 and F2, Burrau at 64, 128 and 256.
        if now.iter().any(|o| matches!(o, Some(Err(_)))) {
            failed += 1;
            f1.push(format!(
                "{pin_name}: {:?}",
                now.iter().map(show_outcome).collect::<Vec<_>>()
            ));
        } else if now.iter().any(|o| matches!(o, Some(Ok(t)) if t.ended)) {
            ends += 1;
            let s64 = match &now[0] {
                Some(Ok(t)) if t.ended => Some(t.end()),
                _ => None,
            };
            let true_end = s64.is_some_and(|s64| {
                now.iter().zip([64.0, 128.0, 256.0]).all(|(o, n)| {
                    matches!(o, Some(Ok(t)) if t.ended && (t.end() - s64).abs() <= 1.0 / 64.0 && t.end() > 1.0 / n)
                })
            });
            // The brackets `[s_n, s_n + 1/n]` of the last good frame share a point.
            let shared = now
                .iter()
                .zip([64.0, 128.0, 256.0])
                .filter_map(|(o, n)| match o {
                    Some(Ok(t)) => Some((t.end(), t.end() + 1.0 / n)),
                    _ => None,
                })
                .fold((f64::NEG_INFINITY, f64::INFINITY), |(lo, hi), (a, b)| {
                    (lo.max(a), hi.min(b))
                });
            let true_end = true_end && shared.0 <= shared.1;
            if !true_end {
                f2.push(format!(
                    "{pin_name}: {:?}",
                    now.iter().map(show_outcome).collect::<Vec<_>>()
                ));
            }
        } else if now.iter().all(|o| matches!(o, Some(Ok(_)))) {
            through += 1;
        }
        // F4: what went through on main goes through the same way.
        for (name, before, after, hh) in [
            ("Burrau", &main_b, &now[0], h),
            ("rest", &main_r, &now_r, h_rest),
        ] {
            let Some(Ok(tb)) = before else { continue };
            if tb.ended {
                continue;
            }
            match after {
                Some(Ok(ta))
                    if !ta.ended
                        && ta.not_nearest(1.0) == tb.not_nearest(1.0)
                        && ta.nodes.len() == tb.nodes.len() =>
                {
                    let d = ta
                        .nodes
                        .iter()
                        .zip(&tb.nodes)
                        .map(|(x, y)| {
                            ((x.x[0] - y.x[0]).powi(2) + (x.x[1] - y.x[1]).powi(2)).sqrt()
                        })
                        .fold(0.0f64, f64::max);
                    worst_node = worst_node.max(d / hh);
                    if d > 1e-6 * hh {
                        f4.push(format!(
                            "{name} {pin_name}: a node {:.3e}·h from main's",
                            d / hh
                        ));
                    }
                }
                other => f4.push(format!(
                    "{name} {pin_name}: through on main, now {}",
                    show_outcome(other)
                )),
            }
        }
    }
    println!("path_lock_slide_follows_sweep: Burrau: {through} pins through at all three counts, {ends} that end, {failed} with a failed follow");
    println!(
        "path_lock_slide_follows_sweep: F4: the largest node distance from main's {:.3e}·h",
        worst_node
    );
    println!("path_lock_slide_follows_sweep: F1 failures: {}", f1.len());
    for x in &f1 {
        println!("  {x}");
    }
    println!("path_lock_slide_follows_sweep: F2 failures: {}", f2.len());
    for x in &f2 {
        println!("  {x}");
    }
    println!("path_lock_slide_follows_sweep: F4 failures: {}", f4.len());
    for x in &f4 {
        println!("  {x}");
    }
    assert!(f1.is_empty(), "F1: {} pins' follows fail", f1.len());
    assert!(
        f2.is_empty(),
        "F2: {} stops that are not true ends",
        f2.len()
    );
    assert!(
        f4.is_empty(),
        "F4: {} slides that went through before do not now",
        f4.len()
    );
}

// ── path_lock_line_offset: the slide's residual is the frame's offset ───────────────────────────────────────────────
//
// `blend::line_offset(a, b, s)` is `block_line(a, b, s) − a`, formed without the cancellation: the slide solves on it,
// the frames show `block_line`. On each of the great circle's branches (general with `a·b ≥ 0` and `< 0`, `b = a`, the
// antipodal tie and a near-tie inside `√ε`) at `s` from `10⁻³` to 0.9, with masses and momenta moved too. Pass: every
// component within `16·ε·max(1, |a|, |b|)` of `block_line(a, b, s) − a`, whose own rounding is `ε·|a|`. Control: the
// offset taken at `1 − s`.

fn check_line_offset(offset: &dyn Fn(&Blocks, &Blocks, f64) -> [f64; 9]) {
    let unit = |v: [f64; 3]| {
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|x| x / n)
    };
    let a = Blocks {
        mu: [0.3, -0.7],
        n: unit([0.2, -0.5, 0.8]),
        p_rho: [1.1, -0.4],
        p_lambda: [-0.6, 0.9],
    };
    let tilt = |n: [f64; 3]| Blocks {
        mu: [0.1, 0.4],
        n,
        p_rho: [-0.2, 0.5],
        p_lambda: [0.7, 1.3],
    };
    let near_tie = unit([-a.n[0] + 1e-9, -a.n[1], -a.n[2]]);
    let cases = [
        ("general, a·b ≥ 0", tilt(unit([0.4, -0.1, 0.9]))),
        ("general, a·b < 0", tilt(unit([-0.6, 0.7, -0.2]))),
        ("b = a", tilt(a.n)),
        ("the antipodal tie", tilt(a.n.map(|x| -x))),
        ("a near-tie", tilt(near_tie)),
    ];
    for (name, b) in &cases {
        for s in [1e-3, 1.0 / 64.0, 0.25, 0.5, 0.9] {
            let line = engine::path::blend::block_line(&a, b, s).0.vector();
            let av = a.vector();
            let got = offset(&a, b, s);
            let scale = av
                .iter()
                .chain(&b.vector())
                .fold(1.0f64, |m, x| m.max(x.abs()));
            for i in 0..9 {
                let want = line[i] - av[i];
                assert!(
                    (got[i] - want).abs() <= 16.0 * f64::EPSILON * scale,
                    "the slide's residual is not the frame's offset: {name}, s = {s}, component {i}: {} against {want}",
                    got[i]
                );
            }
        }
    }
}

#[test]
fn path_lock_line_offset() {
    check_line_offset(&engine::path::blend::line_offset);
}

negative_control!(
    path_lock_line_offset,
    "the offset taken at 1 − s",
    expected = "the slide's residual is not the frame's offset",
    check_line_offset(&|a, b, s| engine::path::blend::line_offset(a, b, 1.0 - s))
);

// ── path_lock_slide_follows: the follow converges at a slide's first frames ─────────────────────────────────────────
//
// The fix's guard in CI (criteria F1 and F2, at two of the pins PR #72's C6 measured stalling at the 2nd frame: (0, 0)
// and (1/15, 0)). Mixed to Burrau at 64 and 256 frames. Pass: no follow fails (`FollowFailed`), and where the slide ends
// it ends at the same `s` within 1/64 at both counts, past the first frame. Control: the slide as main formed it (its
// residual and its slopes), whose follow fails at the 2nd frame.

fn check_slide_follows_at(pins: &[[f64; 2]]) {
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let b = kernel_view(&nu_k(), Window::SQUARE);
    for &at in pins {
        let a = try_locked_mixed(home(), (at[0], at[1])).expect("a lock");
        let ends: Vec<Option<f64>> = [64usize, 256]
            .iter()
            .map(|&n| {
                let end = Segment::locked_blend(&a, &b, BlendPath::BlockLine, &burrau, phys, n)
                    .map(|x| x.1);
                println!(
                    "path_lock_slide_follows: ({}, {}) at {n} frames: {end:?}",
                    at[0], at[1]
                );
                match end {
                    Ok(LockedEnd::Ended { s }) => {
                        assert!(s > 1.0 / n as f64, "an end at the first frame: {s}");
                        Some(s)
                    }
                    Ok(_) => None,
                    Err(e) => panic!(
                        "the follow failed at ({}, {}), {n} frames: {e:?}",
                        at[0], at[1]
                    ),
                }
            })
            .collect();
        if let [Some(s64), Some(s256)] = ends[..] {
            assert!(
                (s64 - s256).abs() <= 1.0 / 64.0,
                "an end that moves with the frame count: {s64} at 64, {s256} at 256"
            );
        }
        assert!(
            ends[0].is_some() == ends[1].is_some(),
            "an end at one frame count and not the other: {ends:?}"
        );
    }
}

#[test]
fn path_lock_slide_follows() {
    check_slide_follows_at(&[[0.0, 0.0], [1.0 / 15.0, 0.0]]);
}

negative_control!(
    path_lock_slide_follows,
    "the slide's residual and slopes as main formed them",
    expected = "the follow failed",
    {
        use engine::path::anchor::probe::{self, Switches};
        let switches = Switches {
            failed_is_end: false,
            ..main_slide()
        };
        probe::with(switches, || check_slide_follows_at(&[[0.0, 0.0]]));
    }
);

// ── path_lock_slide_margins: the margins, measured where the follow still fails ─────────────────────────────────────
//
// D5's fix as first committed (the offset formed directly; five-point slopes at `ε^{1/3}·h`) still fails at the first
// frames at 128 and 256 frames: the F-sweep's first nine rows were read before that run was stopped. At these Burrau
// minimisers the window is `[0, 1]²` and the point sits at `u ≈ −0.03`, about 0.01 from `B`'s mass edge, where `μ₁` is
// log-steep (`|∂B/∂u| ≈ 250`) and `B` barely moves along `v` (`|∂B/∂v| ≈ 0.09`): `JᵀJ`'s conditioning ≈ 1e-7. So the
// margins are measured directly, not by counting stops, at six frames where the follow fails now (the frame's `s` and
// the last converged point before it, under the current slide). Predictions, written before the run:
//
// - **M1, the optimality floor:** the frame's solve from the last point with tolerance 0 stops (its least optimality)
//   within 3× of `1e-8` (3e-9 to 3e-8): the solve is precision-limited, not lost.
// - **M2, the polish's acceptance:** `lock_solve::polish` accepts a step where `‖Jᵀr‖²` falls, while convergence is
//   judged on the scale-invariant `‖P r‖² = gᵀ(JᵀJ)⁻¹g`; with columns 2000× apart, `‖Jᵀr‖²` is the steep column's and
//   its rounding hides the weak one. With the polish accepting where `‖P r‖²` falls instead (one variable), if this is
//   the cause the floor falls at least 10× at every point.
// - **Then D6 and D7** (`path_lock_slide_stall`, one more variable each, predictions written after M1–M3 were read and
//   before D6 and D7 ran): D6, the polish
//   judged on `‖P r‖` alone (main's residual and slopes): if M2 holds, the `Stalled` stops fall well below D0's, the
//   ones left at the first frames, where main's residual still cancels; D7, all three: no `Stalled`, and no `Ended`
//   before the 7th frame.
// - **M3, the scatter:** solves of the frame from seeds `10⁻⁶·h` apart (and from the restart rule's point) converge to
//   points spread by more than the continuity test's floor, `10⁻⁷·h`, where the follow ended: the test reads scatter as
//   travel.

/// The six frames: pin, frame count.
#[cfg(feature = "controls")]
const MARGIN_CASES: [([f64; 2], usize); 6] = [
    ([0.0, 0.0], 64),
    ([0.0, 0.0], 256),
    ([1.0 / 15.0, 0.0], 128),
    ([2.0 / 15.0, 0.0], 64),
    ([3.0 / 15.0, 0.0], 64),
    ([6.0 / 15.0, 0.0], 64),
];

#[cfg(feature = "controls")]
#[test]
#[ignore = "about a minute: cargo test -p engine --features controls --test path_lock -- path_lock_slide_margins --exact --include-ignored --nocapture"]
fn path_lock_slide_margins() {
    measure_margins(&MARGIN_CASES, true);
}

/// The margins at `cases`, the slide's solves logged where `logged` (the control's switch: with no log, there is no
/// failed follow to measure at).
#[cfg(feature = "controls")]
fn measure_margins(cases: &[([f64; 2], usize)], logged: bool) {
    use engine::path::anchor::probe::{self, Switches};
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let b = kernel_view(&nu_k(), Window::SQUARE);
    let h = b.sim.chart.window.half[0]
        .abs()
        .max(b.sim.chart.window.half[1].abs());
    let mut read = 0;
    for &(at, frames) in cases {
        let a = try_locked_mixed(home(), (at[0], at[1])).expect("a lock");
        let anchor_ = a.anchor.expect("an anchor");
        // Where D5's slide (the residual and slopes fixed, the polish as main's) failed, the frames M1–M3 were taken at.
        let d5 = Switches {
            offset: true,
            five_point: true,
            polish_unscaled: true,
            ..Switches::default()
        };
        let (out, log) = probe::with(d5, || anchor::slide(&b, &burrau, pin(&a), &anchor_, frames));
        let log = match logged {
            true => log,
            false => Vec::new(),
        };
        let Some(k) = log
            .iter()
            .position(|x| x.what == "follow" && !x.report.converged())
        else {
            println!(
                "path_lock_slide_margins: ({:.4}, {:.4}) at {frames}: no failed follow ({:?})",
                at[0],
                at[1],
                out.map(|t| t.ended)
            );
            continue;
        };
        let failed = log[k];
        let prev = log[..k]
            .iter()
            .rev()
            .find(|x| matches!(x.what, "follow" | "slide first") && x.report.converged())
            .expect("a converged frame before it");
        let (s, x) = (failed.s, prev.report.x);
        let frame = |sw: Switches, seed: [f64; 2], at_s: f64| {
            probe::with(sw, || {
                anchor::slide_frame(&b, &burrau, pin(&a), &anchor_, at_s, seed)
            })
            .0
        };
        let floor = Switches {
            target: Some(0.0),
            optimality: Some(0.0),
            offset: true,
            five_point: true,
            ..Switches::default()
        };
        let m1 = frame(
            Switches {
                polish_unscaled: true,
                ..floor
            },
            x,
            s,
        );
        let m2 = frame(floor, x, s);
        let spread = |at_s: f64| {
            let seeds = [
                x,
                [x[0] + 1e-6 * h, x[1]],
                [x[0], x[1] + 1e-6 * h],
                [x[0] - 1e-6 * h, x[1] - 1e-6 * h],
            ];
            let got: Vec<[f64; 2]> = seeds
                .iter()
                .map(|&seed| frame(d5, seed, at_s))
                .filter(|r| r.converged())
                .map(|r| r.x)
                .collect();
            let mut d = 0.0f64;
            for p in &got {
                for q in &got {
                    d = d.max(((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt());
                }
            }
            (got.len(), d / h)
        };
        let (n_prev, d_prev) = spread(prev.s);
        let (n_s, d_s) = spread(s);
        println!(
            "path_lock_slide_margins: ({:.4}, {:.4}) at {frames}, s = {s:.6} ({:?}, opt {:.2e}, cond {:.2e}): M1 floor {:.2e} ({:?}, it {}); M2 floor {:.2e} ({:?}, it {}); M3 spread/h at the last frame {d_prev:.2e} ({n_prev} of 4 converged), at s {d_s:.2e} ({n_s} of 4); the travel/h of the last frame {:.2e}; the continuity floor/h 1e-7",
            at[0],
            at[1],
            failed.report.status,
            failed.report.optimality,
            failed.report.conditioning,
            m1.optimality,
            m1.status,
            m1.iterations,
            m2.optimality,
            m2.status,
            m2.iterations,
            prev.report.travel / h,
        );
        read += 1;
    }
    assert!(
        read > 0,
        "no failed follow to measure at any of the six frames"
    );
}

negative_control!(
    path_lock_slide_margins,
    "the slide's solves not logged, at (0, 0) at 64 frames, where its follow fails: nothing to measure",
    expected = "no failed follow to measure",
    measure_margins(&MARGIN_CASES[..1], false)
);

// ── path_lock_slide_first_choice: F4's 29 pins, read ────────────────────────────────────────────────────────────────
//
// F4 failed at 29 Burrau pins when the fix was D7's three changes (`path_lock_slide_follows_sweep_three`): each goes
// through as on main, on a track about `2h` from main's. A measurement of why, written before it ran: at each, the
// first frame's restart rule (`"slide first"`) under main's solve and under the three changes, its kept seed's index and
// residual; and main's solve from the three changes' kept seed. If main's choice is a stall's artefact, the three
// changes' kept point has the lower residual, and main's solve from that seed stalled short of converging there.
//
// Read: the prediction is refuted. At all 29 the two kept points are mirror images in `v`, `(u, −v)` and `(u, v)`, their
// residuals equal to a few `10⁻¹⁵` (relative) and their states at the pin within `2.3·10⁻⁸` of each other in the
// blocks; main's solve from the other seed converges there too (at 27; `IllConditioned` at 2). On Burrau the first
// frame's minimiser comes as a mirror pair, and the restart rule's strict "least residual" picks between the two by
// rounding, on main as under the three changes. The polish alone, the fix as it is, leaves every pick as main's (F4
// passes). Its check, written after that run: at each of the 29 the two points show the same state (within `10⁻⁶` in
// the blocks) and their residuals agree within `10⁻¹²` (relative). Control: main's first point set against the three
// changes' at another frame (`s = 1/2`), a different state.

/// At each of `pins`: main's first point against the three changes' at the frame `other` (the first, `1/64`, but for the
/// control): the largest distance between their states in the blocks, and between their residuals (relative).
#[cfg(feature = "controls")]
fn first_choice(pins: &[[f64; 2]], other: f64) -> (f64, f64) {
    use engine::path::anchor::probe::{self, Switches};
    let burrau = (
        MixedLeaves,
        ChartLeaves::<_, Defaults>::new(nu_k()).expect("a chart"),
    );
    let b = kernel_view(&nu_k(), Window::SQUARE);
    let (mut far, mut unequal) = (0.0f64, 0.0f64);
    for &at in pins {
        let a = try_locked_mixed(home(), (at[0], at[1])).expect("a lock");
        let anchor_ = a.anchor.expect("an anchor");
        let frame = |sw: Switches, at_s: f64| {
            let (_, log) = probe::with(sw, || anchor::slide(&b, &burrau, pin(&a), &anchor_, 64));
            log.into_iter()
                .find(|x| match at_s == 1.0 / 64.0 {
                    true => x.what == "slide first",
                    false => x.what == "follow" && x.s == at_s,
                })
                .expect("a frame")
                .report
        };
        let (m, n) = (
            frame(main_slide(), 1.0 / 64.0),
            frame(three_changes(), other),
        );
        let index = |r: &kernel::chart::lock_solve::Report<f64>| match r.branch {
            kernel::chart::lock_solve::Branch::Restart { index, .. } => index,
            _ => u32::MAX,
        };
        let seed = match n.branch {
            kernel::chart::lock_solve::Branch::Restart { seed, .. } => seed,
            _ => n.x,
        };
        let (from_seed, _) = probe::with(main_slide(), || {
            anchor::slide_frame(&b, &burrau, pin(&a), &anchor_, 1.0 / 64.0, seed)
        });
        let blocks_at = |x: [f64; 2]| {
            let v = anchor::placed(&b, pin(&a), x);
            Blocks::of(&state(anchor::state_at(&v, &burrau, pin(&a)).expect("a state")).ic).vector()
        };
        let (bm, bn) = (blocks_at(m.x), blocks_at(n.x));
        let apart = (0..9).map(|i| (bm[i] - bn[i]).powi(2)).sum::<f64>().sqrt();
        let rel = ((m.residual - n.residual) / m.residual).abs();
        println!(
            "path_lock_slide_first_choice: ({:.4}, {:.4}): main keeps seed {} at {:?}, r = {:.6e}; the three changes keep seed {} at {:?}, r = {:.6e}; main's solve from that seed {:?}; residuals {rel:.3e} apart (relative), states {apart:.3e} in the blocks",
            at[0],
            at[1],
            index(&m),
            m.x,
            m.residual,
            index(&n),
            n.x,
            n.residual,
            from_seed.status,
        );
        far = far.max(apart);
        unequal = unequal.max(rel);
    }
    println!(
        "path_lock_slide_first_choice: at {} pins, the states at most {far:.3e} apart, the residuals {unequal:.3e}",
        pins.len()
    );
    (far, unequal)
}

#[cfg(feature = "controls")]
fn check_first_choice(pins: &[[f64; 2]], other: f64) {
    let (far, unequal) = first_choice(pins, other);
    assert!(
        far <= 1e-6 && unequal <= 1e-12,
        "main's first point and the three changes' are not one state: {far:.3e} apart, residuals {unequal:.3e}"
    );
}

#[cfg(feature = "controls")]
#[test]
#[ignore = "seconds in release: cargo test --release -p engine --features controls --test path_lock -- path_lock_slide_first_choice --exact --include-ignored --nocapture"]
fn path_lock_slide_first_choice() {
    let pins: Vec<[f64; 2]> = [
        (9, 0),
        (14, 0),
        (15, 0),
        (14, 1),
        (11, 2),
        (14, 2),
        (11, 3),
        (11, 4),
        (12, 4),
        (14, 4),
        (11, 5),
        (12, 5),
        (13, 5),
        (12, 6),
        (12, 7),
        (13, 7),
        (10, 8),
        (11, 10),
        (12, 10),
        (11, 11),
        (11, 12),
        (12, 12),
        (13, 12),
        (10, 13),
        (12, 13),
        (10, 14),
        (14, 14),
        (10, 15),
        (12, 15),
    ]
    .iter()
    .map(|&(k, j)| grid_pin(k, j))
    .collect();
    check_first_choice(&pins, 1.0 / 64.0);
}

negative_control!(
    path_lock_slide_first_choice,
    "main's first point set against the three changes' at s = 1/2, a different state, at (0.6, 0)",
    expected = "are not one state",
    check_first_choice(&[grid_pin(9, 0)], 0.5)
);
