//! The chart-path object's own properties (TASK-M2-61; design § 17.4, § 17.5, § 17.9; the chart-space model's Theorems
//! 4 and 5; export contract Part 3), each against the direct operation's bits:
//!
//! - `path_ends_bitwise`: a segment's frame at `s = 0` is `A` and at `s = 1` is `B`, the stored views, bit for bit, `−0`
//!   is `+0`, and NaN or `s` outside `[0, 1]` is refused. Control: an end through the family's arithmetic, `from + 1·(to
//!   − from)`, which is not `to` on some pairs.
//! - `path_family_interior`: each family's frame at `s` is the direct operation at its parameter scaled by `s`, bit for
//!   bit, and a slider move's interior tends to its end. Control: the slider move run through the slice family, whose
//!   interior leaves along the step's part off the plane and does not.
//! - `path_record_round_trip`: a frame's record `(A, B, family, s)` reads back bit for bit and gives the same frame; a
//!   record whose `B` its family does not reach is refused. Control: `s` held at f32.
//! - `path_easing_timing_only`: an eased segment shows at `u` the frame the plain one shows at `ease(u)`, bit for bit, and
//!   its record has no easing in it. Controls: an easing applied to a frame's content; a second easing beside the
//!   segment's.
//! - `path_concat_join`: segments concatenate only where they share their join bit for bit; the joined path shows each
//!   segment's frames under that segment's own easing, the join once. Control: a join compared to rounding.
//! - `path_keyframe_bitwise`: keyframes joined by a chain of single-family segments: each keyframe's own frame is that
//!   keyframe's view, bit for bit, and each frame between is its family's. Control: a keyframe's frame through the
//!   family's arithmetic.
//!
//! The criteria are written before the first run: every comparison is of the views' bytes, exactly.

mod path_shim;

use engine::contract::sim_config::Plane;
use engine::nav::directions::hidden;
use engine::nav::BasisVector;
use engine::path::{
    Constructor, Easing, FrameRecord, MonotoneTable, Path, PathError, Segment, Timing, View,
};
use path_shim::sim_of;
use validation::negative_control;

/// The oblique, skewed, non-unit plane with a `−0`.
fn skew() -> Plane {
    Plane {
        z0: [0.3, -0.0, 0.1, -0.2, 0.05, 0.0, 0.4, -0.25],
        q1: [0.7, 0.2, -0.0, 0.1, 0.0, 0.0, 0.05, 0.0],
        q2: [0.037, 0.333, 0.111, -0.074, 0.0, 0.037, 0.0, -0.0],
    }
}

fn view() -> View {
    View::latent(sim_of(&skew()))
}

/// A locked view of the skew plane, locked at `(0.3, 0.8)`.
fn locked() -> View {
    Segment::lock(&view(), 0.3, 0.8)
        .expect("a lock")
        .b()
        .clone()
}

/// A segment of each family stage 1 builds, from the skew view.
fn segments() -> Vec<(&'static str, Segment)> {
    let a = view();
    let dirs = hidden(&skew()).expect("hidden");
    let c = Constructor::new(skew());
    let saved_from = a.clone();
    vec![
        ("pan", Segment::pan(&a, 0.25, -0.125).unwrap()),
        ("zoom", Segment::zoom(&a, 0.3).unwrap()),
        (
            "slice",
            Segment::slice(&a, &dirs[0].direction, 0.5).unwrap(),
        ),
        ("control", Segment::control(&a, 0, 0.91).unwrap()),
        (
            "raw_tilt",
            Segment::raw_tilt(&c, &a, BasisVector::Q1, (0.0, 30.0), &dirs[0].direction).unwrap(),
        ),
        (
            "raw_tilt_pair",
            Segment::raw_tilt_pair(
                &c,
                &a,
                [0.0, 0.0],
                [20.0, -35.0],
                [&dirs[0].direction, &dirs[1].direction],
            )
            .unwrap(),
        ),
        ("lock", Segment::lock(&a, 0.3, 0.8).unwrap()),
        ("snap_back", Segment::snap_back(&locked()).unwrap()),
        ("unlock", Segment::unlock(&locked()).unwrap()),
        (
            "chart_switch",
            Segment::chart_switch(
                &a,
                View::latent(sim_of(&Plane {
                    z0: [0.5; 8],
                    ..skew()
                })),
            ),
        ),
        ("save_view", Segment::save_view(&saved_from).unwrap()),
    ]
}

/// The parameters each test reads, the ends excluded.
const S: [f64; 5] = [0.1, 0.25, 1.0 / 3.0, 0.5, 0.875];

// ── path_ends_bitwise ──────────────────────────────────────────────────────────────────────────────────────────────

/// The segment's end at `s = 1` as `end` computes it.
type End = fn(&Segment) -> View;

/// The stored `B`.
fn stored_end(seg: &Segment) -> View {
    seg.frame(1.0).expect("the end")
}

fn check_ends(end: End) {
    for (name, seg) in segments() {
        assert_eq!(
            seg.frame(0.0).unwrap().bytes(),
            seg.a().bytes(),
            "{name}: s = 0 is not A"
        );
        assert_eq!(
            seg.frame(-0.0).unwrap().bytes(),
            seg.a().bytes(),
            "{name}: s = −0 is not A"
        );
        assert_eq!(end(&seg).bytes(), seg.b().bytes(), "{name}: s = 1 is not B");
        for bad in [f64::NAN, 1.0f64.next_up(), -1e-300, f64::INFINITY] {
            assert_eq!(
                seg.frame(bad),
                Err(PathError::Parameter),
                "{name}: s = {bad} is shown"
            );
        }
    }
    // The slider moves of the skew plane's controls to values where `a + (b − a) ≠ b` in f64.
    for (k, to) in [(0, 0.91), (3, 0.91), (6, -1.7), (7, 0.33)] {
        let seg = Segment::control(&view(), k, to).unwrap();
        assert_eq!(
            end(&seg).bytes(),
            seg.b().bytes(),
            "control {k} → {to}: s = 1 is not B"
        );
    }
}

#[test]
fn path_ends_bitwise() {
    check_ends(stored_end);
}

/// The end through the family's arithmetic: a slider move's `from + 1·(to − from)`.
#[cfg(feature = "controls")]
fn end_through_family(seg: &Segment) -> View {
    match seg.family() {
        engine::path::Family::Control { k, from, to } => {
            Segment::control(seg.a(), *k, from + 1.0 * (to - from))
                .unwrap()
                .b()
                .clone()
        }
        _ => seg.b().clone(),
    }
}

negative_control!(
    path_ends_bitwise,
    "an end through the family's arithmetic",
    expected = "s = 1 is not B",
    check_ends(end_through_family)
);

// ── path_family_interior ───────────────────────────────────────────────────────────────────────────────────────────

/// The segment a slider move of control `k` to `to` is run as: its own family, or another.
type SliderRun = fn(&View, usize, f64) -> Segment;

fn slider_own(a: &View, k: usize, to: f64) -> Segment {
    Segment::control(a, k, to).unwrap()
}

fn check_interior(slider: SliderRun) {
    let a = view();
    let dirs = hidden(&skew()).expect("hidden");
    let c = Constructor::new(skew());
    for s in S {
        let direct = |seg: Segment| seg.b().bytes();
        let pairs = [
            (
                "pan",
                Segment::pan(&a, 0.25, -0.125)
                    .unwrap()
                    .frame(s)
                    .unwrap()
                    .bytes(),
                direct(Segment::pan(&a, s * 0.25, s * -0.125).unwrap()),
            ),
            (
                "zoom",
                Segment::zoom(&a, 0.3).unwrap().frame(s).unwrap().bytes(),
                direct(Segment::zoom(&a, s * 0.3).unwrap()),
            ),
            (
                "slice",
                Segment::slice(&a, &dirs[0].direction, 0.5)
                    .unwrap()
                    .frame(s)
                    .unwrap()
                    .bytes(),
                direct(Segment::slice(&a, &dirs[0].direction, s * 0.5).unwrap()),
            ),
            (
                "raw_tilt",
                Segment::raw_tilt(&c, &a, BasisVector::Q2, (0.0, 45.5), &dirs[3].direction)
                    .unwrap()
                    .frame(s)
                    .unwrap()
                    .bytes(),
                direct(
                    Segment::raw_tilt(&c, &a, BasisVector::Q2, (0.0, s * 45.5), &dirs[3].direction)
                        .unwrap(),
                ),
            ),
        ];
        for (name, frame, op) in pairs {
            assert_eq!(
                frame, op,
                "{name} at s = {s}: not the direct operation at the scaled parameter"
            );
        }
    }
    // A slider move's interior tends to its end: at s = 1 − 2⁻³⁰ within 2⁻²⁸ of B in every control.
    let near = 1.0 - (-30.0f64).exp2();
    for (k, to) in [(0, 0.91), (3, 0.91), (6, -1.7), (7, 0.33)] {
        let frame = slider(&a, k, to).frame(near).unwrap().sim.plane.z0;
        let end = Segment::control(&a, k, to).unwrap().b().sim.plane.z0;
        for i in 0..8 {
            assert!(
                (frame[i] - end[i]).abs() <= (-28.0f64).exp2(),
                "slider {k} → {to}: its interior does not tend to its end at control {i}: {} against {}",
                frame[i],
                end[i]
            );
        }
    }
}

#[test]
fn path_family_interior() {
    check_interior(slider_own);
}

/// The slider move run through the slice family: `z₀ += t·off_plane(ê_k)`, `t = to − from`.
#[cfg(feature = "controls")]
fn slider_as_slice(a: &View, k: usize, to: f64) -> Segment {
    let mut e: engine::contract::sim_config::Latent = [0.0; 8];
    e[k] = 1.0;
    Segment::slice(a, &e, to - a.sim.plane.z0[k]).unwrap()
}

negative_control!(
    path_family_interior,
    "a slider move through the slice family",
    expected = "its interior does not tend to its end",
    check_interior(slider_as_slice)
);

// ── path_record_round_trip ─────────────────────────────────────────────────────────────────────────────────────────

/// How a record's path parameter is held.
type Hold = fn(f64) -> f64;

fn exact(s: f64) -> f64 {
    s
}

fn check_records(hold: Hold) {
    for (name, seg) in segments() {
        for s in [0.0, -0.0, 1.0].into_iter().chain(S) {
            let record = seg.record(hold(s)).unwrap();
            let bytes = record.to_bytes();
            let read = FrameRecord::read(&bytes).unwrap();
            assert_eq!(
                read.to_bytes(),
                bytes,
                "{name} at s = {s}: the record does not read back"
            );
            assert_eq!(
                read.frame().unwrap().bytes(),
                seg.frame(s).unwrap().bytes(),
                "{name} at s = {s}: the record gives another frame"
            );
        }
        assert_eq!(
            seg.record(-0.0).unwrap().s_bits,
            0,
            "{name}: −0 is not recorded as +0"
        );
    }
    // A record whose B its family does not reach is refused.
    let pan = Segment::pan(&view(), 0.25, -0.125).unwrap();
    let mut forged = pan.record(0.5).unwrap();
    forged.b.sim.plane.z0[0] = forged.b.sim.plane.z0[0].next_up();
    let forged = FrameRecord::read(&forged.to_bytes()).unwrap();
    assert_eq!(
        forged.frame(),
        Err(PathError::Rebuilt),
        "a forged B is shown"
    );
    // A replacing tilt's record read back as a raw tilt family is refused: it does not start at its A.
    let dirs = hidden(&skew()).expect("hidden");
    let mut posing = tilt_q2(&pre_tilted()).record(0.5).unwrap();
    posing.family = engine::path::Family::RawTilt {
        committed: Box::new(skew()),
        form: engine::path::TiltForm::Single {
            which: BasisVector::Q2,
            from: 0.0,
            to: 45.5,
            target: dirs[3].direction,
        },
    };
    let posing = FrameRecord::read(&posing.to_bytes()).unwrap();
    assert_eq!(
        posing.frame(),
        Err(PathError::Rebuilt),
        "a raw tilt that does not start at A is shown"
    );
}

#[test]
fn path_record_round_trip() {
    check_records(exact);
}

/// `s` held at f32.
#[cfg(feature = "controls")]
fn at_f32(s: f64) -> f64 {
    s as f32 as f64
}

negative_control!(
    path_record_round_trip,
    "the path parameter held at f32",
    expected = "the record gives another frame",
    check_records(at_f32)
);

// ── path_easing_timing_only ────────────────────────────────────────────────────────────────────────────────────────

/// A pinned, monotone, non-linear easing.
fn table() -> Easing {
    Easing::Table(
        MonotoneTable::new(vec![
            (0.0, 0.0),
            (0.25, 0.1),
            (0.5, 0.6),
            (0.8, 0.9),
            (1.0, 1.0),
        ])
        .expect("a table"),
    )
}

/// The frame an eased segment shows at playback fraction `u`.
type Shown = fn(&Segment, f64) -> View;

fn shown(seg: &Segment, u: f64) -> View {
    seg.at(u).unwrap()
}

const U: [f64; 7] = [0.0, 0.1, 0.25, 0.4, 0.5, 0.8, 1.0];

fn check_timing(show: Shown) {
    let easing = table();
    for (name, plain) in segments() {
        let eased = plain.clone().with_timing(Timing {
            easing: easing.clone(),
            retime: None,
        });
        for u in U {
            assert_eq!(
                show(&eased, u).bytes(),
                plain.frame(easing.ease(u)).unwrap().bytes(),
                "{name} at u = {u}: easing changed a frame's content"
            );
            let s = easing.ease(u);
            assert_eq!(
                eased.record(s).unwrap().to_bytes(),
                plain.record(s).unwrap().to_bytes(),
                "{name} at s = {s}: the easing is in the record"
            );
        }
        assert_eq!(
            show(&eased, 0.0).bytes(),
            plain.a().bytes(),
            "{name}: easing moved A"
        );
        assert_eq!(
            show(&eased, 1.0).bytes(),
            plain.b().bytes(),
            "{name}: easing moved B"
        );
    }
    // The segment's timing is the one representation a live path, a saved path and an export read (R-901): it reads
    // back bit for bit. No saved-path or export format exists in stage 1 to read it from yet.
    let timing = Timing {
        easing: easing.clone(),
        retime: None,
    };
    let text = serde_json::to_string(&timing).expect("a timing serialises");
    assert_eq!(
        serde_json::from_str::<Timing>(&text).expect("a timing reads back"),
        timing,
        "the timing does not read back"
    );
    assert_eq!(
        MonotoneTable::new(vec![(0.0, 0.0), (0.5, 0.7), (0.4, 0.8), (1.0, 1.0)]),
        Err(PathError::Easing),
        "a table that is not monotone is offered"
    );
}

#[test]
fn path_easing_timing_only() {
    check_timing(shown);
}

/// The easing applied to the frame's content: the shown frame panned by `0.01·ease(u)`.
#[cfg(feature = "controls")]
fn eased_content(seg: &Segment, u: f64) -> View {
    let frame = seg.at(u).unwrap();
    match Segment::pan(&frame, 0.01 * seg.timing().easing.ease(u), 0.0) {
        Ok(p) => p.b().clone(),
        Err(_) => frame,
    }
}

/// A second easing beside the segment's: `ease(ease(u))`.
#[cfg(feature = "controls")]
fn eased_twice(seg: &Segment, u: f64) -> View {
    seg.at(seg.timing().easing.ease(u)).unwrap()
}

negative_control!(
    path_easing_timing_only,
    "an easing applied to a frame's content",
    expected = "easing changed a frame's content",
    check_timing(eased_content)
);

mod second_easing {
    use super::*;

    #[test]
    fn path_easing_timing_only_twice() {
        check_timing(shown);
    }

    negative_control!(
        path_easing_timing_only_twice,
        "a second easing beside the segment's",
        expected = "easing changed a frame's content",
        check_timing(eased_twice)
    );
}

// ── path_concat_join ───────────────────────────────────────────────────────────────────────────────────────────────

/// Whether two views join.
type Joins = fn(&View, &View) -> bool;

fn bitwise(a: &View, b: &View) -> bool {
    a.bytes() == b.bytes()
}

fn check_concat(joins: Joins) {
    let first = Segment::pan(&view(), 0.25, -0.125).unwrap();
    let second = Segment::zoom(first.b(), 0.3).unwrap().with_timing(Timing {
        easing: table(),
        retime: None,
    });
    let path = Path::new(vec![first.clone(), second.clone()]).expect("a path");
    assert_eq!(
        path.at(0, 0.0).unwrap().bytes(),
        first.a().bytes(),
        "the path does not start at A"
    );
    assert_eq!(
        path.at(0, 1.0).unwrap().bytes(),
        first.b().bytes(),
        "the first segment does not end at its B"
    );
    assert_eq!(
        path.at(1, 0.0).unwrap().bytes(),
        second.a().bytes(),
        "the join is not the shared view"
    );
    assert_eq!(
        path.keyframe(1).unwrap().bytes(),
        first.b().bytes(),
        "the join is not the shared view"
    );
    assert_eq!(
        path.keyframe(2).unwrap().bytes(),
        second.b().bytes(),
        "the path does not end at B"
    );
    for u in [0.1, 0.25, 0.4, 0.8] {
        assert_eq!(
            path.at(0, u).unwrap().bytes(),
            first.at(u).unwrap().bytes(),
            "first segment at {u}"
        );
        assert_eq!(
            path.at(1, u).unwrap().bytes(),
            second.at(u).unwrap().bytes(),
            "second segment at {u}: not its own easing"
        );
    }
    // A segment one ulp off the join does not join.
    let mut off = first.b().clone();
    off.sim.plane.z0[0] = off.sim.plane.z0[0].next_up();
    let apart = Segment::zoom(&off, 0.3).unwrap();
    assert!(!joins(first.b(), apart.a()), "a join one ulp off is a join");
    assert_eq!(
        Path::new(vec![first, apart]),
        Err(PathError::Join),
        "a path joins one ulp off"
    );
}

#[test]
fn path_concat_join() {
    check_concat(bitwise);
}

/// A join compared to rounding: each `z₀` component within `16·ε`.
#[cfg(feature = "controls")]
fn to_rounding(a: &View, b: &View) -> bool {
    let (x, y) = (a.sim.plane.z0, b.sim.plane.z0);
    (0..8).all(|i| (x[i] - y[i]).abs() <= 16.0 * f64::EPSILON)
}

negative_control!(
    path_concat_join,
    "a join compared to rounding",
    expected = "a join one ulp off is a join",
    check_concat(to_rounding)
);

// ── path_keyframe_bitwise ──────────────────────────────────────────────────────────────────────────────────────────

/// A keyframe's own frame, as `own` gives it from its path and its index.
type Own = fn(&Path, usize) -> View;

fn own_frame(path: &Path, k: usize) -> View {
    path.keyframe(k).unwrap()
}

/// The keyframes and their path: a pan, a zoom, a slider's move and a raw tilt, then 45 small pans, 49 segments in all
/// (a count for which `k/n` is not exact in f64).
fn keyframe_path() -> (Vec<View>, Path) {
    let mut segments = vec![Segment::pan(&view(), -0.3, 0.45).unwrap()];
    let last = |segments: &Vec<Segment>| segments.last().unwrap().b().clone();
    segments.push(Segment::zoom(&last(&segments), -2.5).unwrap());
    segments.push(Segment::control(&last(&segments), 7, 0.33).unwrap());
    // The committed plane is the slider's end, so the tilt starts at its A.
    let tilt_from = last(&segments);
    let c = Constructor::new(tilt_from.sim.plane.clone());
    let dirs = hidden(&tilt_from.sim.plane).expect("hidden");
    segments.push(
        Segment::raw_tilt(
            &c,
            &tilt_from,
            BasisVector::Q1,
            (0.0, -12.25),
            &dirs[5].direction,
        )
        .unwrap(),
    );
    for i in 0..45 {
        let ds = 0.01 * (i as f64 + 1.0) / 7.0;
        segments.push(Segment::pan(&last(&segments), ds, -ds / 3.0).unwrap());
    }
    let mut keyframes = vec![segments[0].a().clone()];
    keyframes.extend(segments.iter().map(|s| s.b().clone()));
    (keyframes, Path::new(segments).expect("a path"))
}

fn check_keyframes(own: Own) {
    let (keyframes, path) = keyframe_path();
    assert_eq!(path.segments().len(), 49);
    assert!(
        matches!(
            path.segments()[3].family(),
            engine::path::Family::RawTilt { .. }
        ),
        "the keyframes' tilt is not a raw tilt family"
    );
    for (k, key) in keyframes.iter().enumerate() {
        assert_eq!(
            own(&path, k).bytes(),
            key.bytes(),
            "keyframe {k}'s own frame is not its view"
        );
    }
    for (i, seg) in path.segments().iter().enumerate().take(6) {
        for s in S {
            assert_eq!(
                path.at(i, s).unwrap().bytes(),
                seg.frame(s).unwrap().bytes(),
                "segment {i} at {s}: not its family's frame"
            );
        }
    }
}

#[test]
fn path_keyframe_bitwise() {
    check_keyframes(own_frame);
}

/// A keyframe's frame addressed by one fraction of the whole path, `u = k/n`, split back into a segment and a place.
#[cfg(feature = "controls")]
fn own_by_fraction(path: &Path, k: usize) -> View {
    let n = path.segments().len();
    let u = k as f64 / n as f64;
    match u == 1.0 {
        true => path.at(n - 1, 1.0).unwrap(),
        false => {
            let i = ((u * n as f64) as usize).min(n - 1);
            path.at(i, u * n as f64 - i as f64).unwrap()
        }
    }
}

negative_control!(
    path_keyframe_bitwise,
    "a keyframe addressed by one fraction of the whole path",
    expected = "own frame is not its view",
    check_keyframes(own_by_fraction)
);

// ── path_tilt_starts_at_a ──────────────────────────────────────────────────────────────────────────────────────────
//
// A raw tilt's interior starts at `A`: raw tilts replace rather than compose, so a tilt from a plane that already
// carries a raw tilt of the other vector jumps there, and is a cut (`TiltReplaces`), not a raw tilt family. The
// criterion: every segment's frame at `s = 2⁻³⁰` is within `10⁻⁶` of `A` in each basis component. Control: the cut's
// record read back as a raw tilt family, whose interior jumps.

/// The raw tilt of `q₂` from the committed skew plane, from `A` as given.
fn tilt_q2(a: &View) -> Segment {
    let dirs = hidden(&skew()).expect("hidden");
    Segment::raw_tilt(
        &Constructor::new(skew()),
        a,
        BasisVector::Q2,
        (0.0, 45.5),
        &dirs[3].direction,
    )
    .unwrap()
}

/// The skew view with `q₁` already tilted by 30° from the committed plane.
fn pre_tilted() -> View {
    let dirs = hidden(&skew()).expect("hidden");
    Segment::raw_tilt(
        &Constructor::new(skew()),
        &view(),
        BasisVector::Q1,
        (0.0, 30.0),
        &dirs[0].direction,
    )
    .unwrap()
    .b()
    .clone()
}

/// The segment as shown: itself, or its record read back with another family.
type AsShown = fn(Segment) -> Box<dyn Fn(f64) -> View>;

fn itself(seg: Segment) -> Box<dyn Fn(f64) -> View> {
    Box::new(move |s| seg.frame(s).unwrap())
}

fn check_starts(shown: AsShown) {
    assert!(
        matches!(
            tilt_q2(&view()).family(),
            engine::path::Family::RawTilt { .. }
        ),
        "a tilt from the committed plane is not a raw tilt family"
    );
    let replaced = tilt_q2(&pre_tilted());
    assert_eq!(
        replaced.family(),
        &engine::path::Family::Cut {
            label: engine::path::CutLabel::TiltReplaces
        },
        "a tilt that replaces another is not a cut"
    );
    for seg in [tilt_q2(&view()), replaced] {
        let a = seg.a().sim.plane.clone();
        let frame = shown(seg)((-30.0f64).exp2()).sim.plane;
        for i in 0..8 {
            assert!(
                (frame.q1[i] - a.q1[i]).abs() <= 1e-6 && (frame.q2[i] - a.q2[i]).abs() <= 1e-6,
                "a raw tilt's interior does not start at A, component {i}"
            );
        }
    }
}

#[test]
fn path_tilt_starts_at_a() {
    check_starts(itself);
}

/// The frame as the direct operation at the scaled angle, whatever the family: the raw tilt to `s·45.5°`.
#[cfg(feature = "controls")]
fn as_raw_tilt(seg: Segment) -> Box<dyn Fn(f64) -> View> {
    let dirs = hidden(&skew()).expect("hidden");
    let a = seg.a().clone();
    Box::new(move |s| {
        Segment::raw_tilt(
            &Constructor::new(skew()),
            &a,
            BasisVector::Q2,
            (0.0, s * 45.5),
            &dirs[3].direction,
        )
        .unwrap()
        .b()
        .clone()
    })
}

negative_control!(
    path_tilt_starts_at_a,
    "a replacing tilt shown as the direct operation at the scaled angle",
    expected = "does not start at A",
    check_starts(as_raw_tilt)
);

// ── path_fence_not_built ───────────────────────────────────────────────────────────────────────────────────────────
//
// Stage 1's fence (R-863): a family stage 1 types and does not build refuses every frame between its ends. Read from a
// record, since no constructor builds one. Control: the blend's frame taken as `A`, as a cut's is.

/// The frame a record names between its ends.
type Between = fn(&FrameRecord) -> Result<View, PathError>;

fn named(r: &FrameRecord) -> Result<View, PathError> {
    r.frame()
}

fn check_fence(between: Between) {
    let pan = Segment::pan(&view(), 0.25, -0.125).unwrap();
    for family in [
        engine::path::Family::WindowEase,
        engine::path::Family::R474Tilt,
        engine::path::Family::PlaneRotation,
        engine::path::Family::Blend(engine::path::BlendPath::BlockLine),
    ] {
        let mut record = pan.record(0.5).unwrap();
        record.family = family.clone();
        let read = FrameRecord::read(&record.to_bytes()).unwrap();
        assert!(
            matches!(between(&read), Err(PathError::NotBuilt(_))),
            "{family:?} is given behaviour in stage 1"
        );
    }
}

#[test]
fn path_fence_not_built() {
    check_fence(named);
}

/// The blend's frame taken as `A`, as a cut's is.
#[cfg(feature = "controls")]
fn as_cut(r: &FrameRecord) -> Result<View, PathError> {
    Ok(r.a.clone())
}

negative_control!(
    path_fence_not_built,
    "a not-built family's frame taken as a cut's",
    expected = "is given behaviour in stage 1",
    check_fence(as_cut)
);

// ── path_saved_view_edits_refused ──────────────────────────────────────────────────────────────────────────────────
//
// Save view as chart gives a view of the saved chart; in stage 1 its view is not edited: its definition holds the
// oblique chart's vectors by value beside the window, and an edit would leave them disagreeing. The criterion: every
// window edit, a slider move, a lock and saving again on a saved view are refused, and saving moves no pixel. Control:
// the saved view taken as a latent one, whose pan is accepted.

/// The view an edit of the saved view acts on.
type AsEdited = fn(&View) -> View;

fn as_is(v: &View) -> View {
    v.clone()
}

fn check_saved(edited: AsEdited) {
    let saved = Segment::save_view(&view()).unwrap().b().clone();
    assert_eq!(saved.sim, view().sim, "Save view as chart moved the view");
    let a = edited(&saved);
    let dirs = hidden(&skew()).expect("hidden");
    let tilt = Segment::raw_tilt(
        &Constructor::new(skew()),
        &a,
        BasisVector::Q1,
        (0.0, 30.0),
        &dirs[0].direction,
    );
    let refusals = [
        ("pan", Segment::pan(&a, 0.25, -0.125).err()),
        ("zoom", Segment::zoom(&a, 0.3).err()),
        ("slice", Segment::slice(&a, &dirs[0].direction, 0.5).err()),
        ("control", Segment::control(&a, 0, 0.91).err()),
        ("lock", Segment::lock(&a, 0.3, 0.8).err()),
        ("save again", Segment::save_view(&a).err()),
        ("raw tilt", tilt.err()),
    ];
    for (name, refused) in refusals {
        assert!(
            matches!(refused, Some(PathError::NotBuilt(_))),
            "{name} of a saved chart's view is not refused: {refused:?}"
        );
    }
}

#[test]
fn path_saved_view_edits_refused() {
    check_saved(as_is);
}

/// The saved view taken as a latent one.
#[cfg(feature = "controls")]
fn as_latent(v: &View) -> View {
    View::latent(v.sim.clone())
}

negative_control!(
    path_saved_view_edits_refused,
    "the saved view taken as a latent one",
    expected = "of a saved chart's view is not refused",
    check_saved(as_latent)
);

// ── path_jump_under_lock_labelled ──────────────────────────────────────────────────────────────────────────────────
//
// A chart switch from a locked view is a cut labelled `JumpUnderLock`, its non-conformance with R-849 stated until
// stage 2's sliding anchor; from a free view it is labelled `ChartSwitch`. Control: the switch from a locked view
// taken from its free copy.

/// The view a switch starts from, given the locked one.
type StartFrom = fn(&View) -> View;

fn locked_as_is(v: &View) -> View {
    v.clone()
}

fn check_jump(from: StartFrom) {
    let target = View::latent(sim_of(&Plane {
        z0: [0.5; 8],
        ..skew()
    }));
    let switch = Segment::chart_switch(&from(&locked()), target.clone());
    assert_eq!(
        switch.family(),
        &engine::path::Family::Cut {
            label: engine::path::CutLabel::JumpUnderLock
        },
        "a jump under a lock is not labelled"
    );
    assert_eq!(
        Segment::chart_switch(&view(), target).family(),
        &engine::path::Family::Cut {
            label: engine::path::CutLabel::ChartSwitch
        },
        "a free chart switch is labelled a jump under a lock"
    );
}

#[test]
fn path_jump_under_lock_labelled() {
    check_jump(locked_as_is);
}

/// The locked view's free copy.
#[cfg(feature = "controls")]
fn freed(v: &View) -> View {
    Segment::unlock(v).unwrap().b().clone()
}

negative_control!(
    path_jump_under_lock_labelled,
    "the switch from a locked view taken from its free copy",
    expected = "a jump under a lock is not labelled",
    check_jump(freed)
);
