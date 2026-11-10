//! The collision detector, the dispatch-time pass and the `dmin_pair` latch (`kernel::detect`; integrator_contract
//! Part 5, Part 7; dd_integrator §3.6; payload §2; R-13, R-22, R-60):
//! - `t0_detectors_*`: at dispatch, before the first step, an IC inside `r_coll` is `collision` at `t_end_step = 0`, an
//!   IC already escaping is not an escape at `t = 0`, and `decode_failed` is the decoder's failure only: a non-finite
//!   decode output (any one of its fifteen values) or a `DEGENERATE` tag (REQ-EVT-002).
//! - `collision_detector_*`: dd test 6, a head-on pair collides with its 0-based pair id, the side opposite the third
//!   body, for each of the three pairs; two pairs inside `r_coll` in one step is `detail = 3`, never a binary label;
//!   the test is strict at `d² = r_coll²`; the threshold is `r_coll²` rounded once to f32, and for `r_coll > 0` a square
//!   below f32's least normal raised to it, so coincident bodies collide under every positive `r_coll` (REQ-EVT-003).
//! - `collision_shared_separation`: the bucket reads the detector's `d²` of the step, the same bits as the substep law's
//!   own form, so one computation serves both (REQ-EVT-004).
//! - `collision_t0_lookup`: a decoded IC with a pair inside `r_coll` is `COLLISION_T0` with the opposite-side pair id
//!   and `t_event = 0`, and the same IC entered via lookup is not rejected and gets the same label (REQ-EVT-009).
//! - `lookup_t0_collision_dispatch`: a lookup fixture with bodies within the default `r_coll`, entered off the canonical
//!   gauge, reaches dispatch and is labelled `collision`, `t_end_step = 0` (REQ-ENC-033).
//! - `dmin_pair_latch*`: on a fly-by, `dmin_pair` is the pair of least separation over the run, neither the IC's nor
//!   the last state's closest; a run whose cadence latches nothing keeps the sentinel 3; a failed run writes 3
//!   (REQ-PAY-047).
//!
//! **Domain** (pitfalls §10): the detectors' decisions on the states given, which are identical on every backend given
//! identical positions; the head-on and fly-by marches are KDK at f64, short, and what they show is the detector's
//! reading of the march, not the march's accuracy.

use kernel::decode::degenerate::ALL;
use kernel::decode::{decode, Degenerate, Ic};
use kernel::detect::collision::{classify, CollisionRadius, DminLatch, Separations, Verdict};
use kernel::detect::t0::{t0, T0};
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::{WordCadence, WrapperCadence};
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::substep::d2_quantised;
use kernel::driver::wrapper::{
    integrate, integrate_detecting, integrate_with, monitored, WrapperUniforms,
};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_dmin_pair, sd_state, tm_t_end_step, FreeGroupWord, SimStateBaseOf,
    SD_DMIN_PAIR_SENTINEL, STATE_BOUNDED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
    STATE_RUNNING, STATE_SIM_FAILED,
};
use validation::drift::{integrate_fixture, Fixture};
use validation::negative_control;

/// Equal masses, `Σmᵢ = 1`.
const THIRD: f64 = 1.0 / 3.0;

/// The default `r_coll`, `10⁻³` (integrator_contract Part 3's table).
const R_COLL_DEFAULT: f64 = 1e-3;

/// An IC of equal masses at positions `r` and momenta `p`.
fn ic(r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> Ic<f64> {
    Ic {
        m: [THIRD; 3],
        r,
        p,
    }
}

/// The sim key's loop at `dt_macro = 10⁻³` for `h` macro-steps, the default substep law.
fn uniforms(h: u32) -> WrapperUniforms<f64> {
    let law = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: 1e-3,
        horizon_steps: h,
        n_max: law.n_max,
        thresholds: thresholds(&law).expect("the default substep law generates"),
    }
}

/// The f64 distance of each R-22 pair of `r`, pair `k` the side opposite body `k`: the test's own reference.
fn distances(r: &[[f64; 2]; 3]) -> [f64; 3] {
    let d = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    [d(r[1], r[2]), d(r[2], r[0]), d(r[0], r[1])]
}

/// The third body of the pair `(a, b)`, whose index is the pair's R-22 id.
fn third(a: usize, b: usize) -> usize {
    3 - a - b
}

// ── t0_detectors ─────────────────────────────────────────────────────────────────────────────────────────────────

/// An IC with bodies 0 and 1 `5·10⁻⁴` apart, body 2 far: inside the default `r_coll`, pair 2.
fn inside() -> Ic<f64> {
    ic(
        [[-0.00025, -0.3], [0.00025, -0.3], [0.0, 0.6]],
        [[0.1, 0.0], [-0.1, 0.0], [0.0, 0.0]],
    )
}

/// An IC already escaping: body 2 far from the bound pair 0–1 and leaving fast.
fn escaping() -> Ic<f64> {
    ic(
        [[-0.1, -2.0], [0.1, -2.0], [0.0, 4.0]],
        [[0.0, -1.0], [0.0, -1.0], [0.0, 2.0]],
    )
}

/// The dispatch-time label `label` gives, under the default `r_coll`: inside `r_coll` is `collision` with pair 2 and
/// the march takes no step, `t_end_step` 0; already escaping is `running`, never `escape`; a non-finite value in any of
/// the fifteen slots, alone and beside a tag, is `decode_failed` 0; each `DEGENERATE` reason is `decode_failed` with its
/// code, inside `r_coll` too.
fn check_t0(label: fn(&Ic<f64>, Option<Degenerate>, &CollisionRadius) -> T0) {
    let radius = CollisionRadius::from_r_coll(R_COLL_DEFAULT);
    let at = label(&inside(), None, &radius);
    assert_eq!(
        (at.state, at.detail),
        (STATE_COLLISION, 2),
        "an IC inside r_coll is not collision with pair 2 at t = 0"
    );
    assert!(at.terminal(), "a t = 0 collision is not terminal");
    let run = integrate_detecting::<Kdk, f64>(&inside(), &uniforms(50), radius);
    assert_eq!(
        (sd_state(run.packed_a), sd_detail(run.packed_a)),
        (STATE_COLLISION, 2),
        "the march from an IC inside r_coll is not collision with pair 2"
    );
    assert_eq!(
        (tm_t_end_step(run.times), run.total_substeps),
        (0, 0),
        "the march from an IC inside r_coll stepped"
    );
    assert_eq!(
        (run.r, run.p),
        (inside().r, inside().p),
        "the march from an IC inside r_coll moved it"
    );

    let esc = label(&escaping(), None, &radius);
    assert_ne!(esc.state, STATE_ESCAPE, "an escaping IC is escape at t = 0");
    assert_eq!(
        esc,
        T0 {
            state: STATE_RUNNING,
            detail: 0
        },
        "an escaping IC does not proceed to the march"
    );
    assert!(!esc.terminal(), "a running label is terminal");
    let run = integrate_detecting::<Kdk, f64>(&escaping(), &uniforms(1), radius);
    assert_eq!(
        sd_state(run.packed_a),
        STATE_BOUNDED,
        "an escaping IC's first step is labelled other than bounded"
    );

    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for i in 0..3 {
            let mut slots: Vec<(String, Ic<f64>)> = Vec::new();
            let mut m = escaping();
            m.m[i] = bad;
            slots.push((format!("m[{i}]"), m));
            for j in 0..2 {
                let mut r = escaping();
                r.r[i][j] = bad;
                slots.push((format!("r[{i}][{j}]"), r));
                let mut p = escaping();
                p.p[i][j] = bad;
                slots.push((format!("p[{i}][{j}]"), p));
            }
            for (slot, x) in slots {
                for tag in [None, Some(Degenerate::M01Tiny)] {
                    let got = label(&x, tag, &radius);
                    assert_eq!(
                        (got.state, got.detail),
                        (STATE_DECODE_FAILED, 0),
                        "{bad} in {slot}, tag {tag:?}: not decode_failed with a non-finite decode output"
                    );
                }
            }
        }
    }
    for reason in ALL {
        for x in [escaping(), inside()] {
            let got = label(&x, Some(reason), &radius);
            assert_eq!(
                (got.state, got.detail),
                (STATE_DECODE_FAILED, reason.detail()),
                "{}: not decode_failed with its code",
                reason.name()
            );
        }
    }
}

#[test]
fn t0_detectors() {
    check_t0(t0);
}

negative_control!(
    t0_detectors,
    "a t = 0 pass folding a collision at t = 0 into decode_failed",
    expected = "is not collision with pair 2 at t = 0",
    check_t0(|x, tag, radius| {
        let got = t0(x, tag, radius);
        if got.state == STATE_COLLISION {
            T0 {
                state: STATE_DECODE_FAILED,
                detail: 1,
            }
        } else {
            got
        }
    })
);

/// `T0::packed_a` writes `state` and `detail` and keeps every other bit.
#[test]
fn t0_detectors_packed_a() {
    let label = T0 {
        state: STATE_COLLISION,
        detail: 3,
    };
    let w = label.packed_a(0xffff_ffc0);
    assert_eq!(
        (sd_state(w), sd_detail(w), w & !0x1f),
        (STATE_COLLISION, 3, 0xffff_ffc0),
        "packed_a did not write state and detail alone"
    );
}

negative_control!(
    t0_detectors_packed_a,
    "a label written over the other bits",
    expected = "packed_a did not write state and detail alone",
    {
        let w = T0 {
            state: STATE_COLLISION,
            detail: 3,
        }
        .packed_a(0);
        assert_eq!(
            (sd_state(w), sd_detail(w), w & !0x1f),
            (STATE_COLLISION, 3, 0xffff_ffc0),
            "packed_a did not write state and detail alone"
        );
    }
);

// ── collision_detector ───────────────────────────────────────────────────────────────────────────────────────────

/// dd test 6's head-on pair: bodies `a` and `b` `0.1` apart on one line, closing at `0.6`, the third far off it.
fn head_on(a: usize, b: usize) -> Ic<f64> {
    let c = third(a, b);
    let mut r = [[0.0; 2]; 3];
    let mut p = [[0.0; 2]; 3];
    r[a] = [-0.05, -0.3];
    r[b] = [0.05, -0.3];
    r[c] = [0.0, 0.6];
    p[a] = [0.1, 0.0];
    p[b] = [-0.1, 0.0];
    ic(r, p)
}

/// The head-on pair `(a, b)` marched under the default `r_coll` collides mid-run with `expected` as its `detail`.
fn check_head_on(a: usize, b: usize, expected: u32) {
    let h = 2000;
    let out = integrate_detecting::<Kdk, f64>(
        &head_on(a, b),
        &uniforms(h),
        CollisionRadius::from_r_coll(R_COLL_DEFAULT),
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_COLLISION,
        "head-on ({a}, {b}) is not a collision"
    );
    assert_eq!(
        sd_detail(out.packed_a),
        expected,
        "head-on ({a}, {b}): collision detail is not pair {expected}"
    );
    let t = tm_t_end_step(out.times);
    assert!(
        t > 0 && t < h,
        "head-on ({a}, {b}) stopped at step {t}, not mid-run"
    );
    let d = distances(&out.r);
    assert!(
        d[expected as usize] < R_COLL_DEFAULT,
        "head-on ({a}, {b}): the pair stopped at {} is not inside r_coll",
        d[expected as usize]
    );
}

#[test]
fn collision_detector_head_on() {
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        check_head_on(a, b, third(a, b) as u32);
    }
}

negative_control!(
    collision_detector_head_on,
    "a pair named by its first body instead of the side opposite the third",
    expected = "collision detail is not pair",
    check_head_on(0, 1, 0)
);

/// The positions of three bodies on a line `s` apart, body 1 in the middle: pairs 0 and 2 at `s`, pair 1 at `2s`.
fn collinear(s: f64) -> [[f64; 2]; 3] {
    [[0.0, 0.0], [s, 0.0], [2.0 * s, 0.0]]
}

/// `verdict` on the counting cases under `r_coll = 0.01`: none inside, each single pair, two pairs (a near-triple)
/// and all three, by the positions' separations and through the wrapper's cadence after one step.
fn check_count(verdict: fn(&Separations, &CollisionRadius) -> Verdict) {
    let r_coll = 0.01;
    let radius = CollisionRadius::from_r_coll(r_coll);
    let far = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert!(
        !verdict(&Separations::of(&far), &radius).fired,
        "no pair inside r_coll fired"
    );
    for k in 0..3 {
        // Bodies k + 1 and k + 2 (mod 3) close, body k far: pair k.
        let mut r = [[0.0; 2]; 3];
        r[(k + 1) % 3] = [0.5, 0.5];
        r[(k + 2) % 3] = [0.5 + 0.5 * r_coll, 0.5];
        r[k] = [-0.5, -0.5];
        let v = verdict(&Separations::of(&r), &radius);
        assert_eq!(
            (v.fired, v.detail),
            (true, k as u32),
            "one pair, pair {k}, inside r_coll"
        );
    }
    let two = collinear(0.6 * r_coll);
    let v = verdict(&Separations::of(&two), &radius);
    assert_eq!(
        (v.fired, v.detail),
        (true, 3),
        "two pairs inside r_coll: never a binary label"
    );
    let all = [
        [0.0, 0.0],
        [0.5 * r_coll, 0.0],
        [0.25 * r_coll, 0.4 * r_coll],
    ];
    let v = verdict(&Separations::of(&all), &radius);
    assert_eq!(
        (v.fired, v.detail),
        (true, 3),
        "three pairs inside r_coll: never a binary label"
    );
}

#[test]
fn collision_detector_count() {
    check_count(classify);
}

negative_control!(
    collision_detector_count,
    "a test of 'any pair below r_coll' first, which steals the triple into the binary arm",
    expected = "never a binary label",
    check_count(|s, radius| {
        let k = s.d2.iter().position(|&d| d < radius.squared());
        Verdict {
            fired: k.is_some(),
            detail: k.unwrap_or(0) as u32,
        }
    })
);

/// Two pairs entering `r_coll` in one step, through the wrapper's cadence: the step's label is `detail = 3`, written as
/// `collision` over `running`, and the cadence is done.
#[test]
fn collision_detector_two_pairs_in_one_step() {
    let r_coll = 0.01;
    let start = ic(collinear(0.1), [[0.0; 2]; 3]);
    let mut cadence = WrapperCadence::with_radius(
        &start.m,
        &start.r,
        &start.p,
        CollisionRadius::from_r_coll(r_coll),
    );
    assert!(!cadence.done(), "the cadence is done before the step");
    let mut s = State {
        m: start.m,
        r: collinear(0.6 * r_coll),
        p: start.p,
    };
    let mut g = Geom::new(&s.m, &s.r);
    cadence.after_step(&mut s, &mut g);
    assert!(
        cadence.done(),
        "two pairs inside r_coll did not end the run"
    );
    let out = cadence.finish(SimStateBaseOf::<f64>::default());
    assert_eq!(
        (sd_state(out.packed_a), sd_detail(out.packed_a)),
        (STATE_COLLISION, 3),
        "two pairs inside r_coll in one step: never a binary label"
    );
}

negative_control!(
    collision_detector_two_pairs_in_one_step,
    "the same step read at r_coll = 0, where nothing collides",
    expected = "did not end the run",
    {
        let start = ic(collinear(0.1), [[0.0; 2]; 3]);
        let mut cadence = WrapperCadence::new(&start.m, &start.r, &start.p);
        let mut s = State {
            m: start.m,
            r: collinear(0.006),
            p: start.p,
        };
        let mut g = Geom::new(&s.m, &s.r);
        cadence.after_step(&mut s, &mut g);
        assert!(
            cadence.done(),
            "two pairs inside r_coll did not end the run"
        );
    }
);

/// A fired verdict stays: a state outside `r_coll` after the collision leaves it, its detail kept.
#[test]
fn collision_detector_stays_fired() {
    let r_coll = 0.01;
    let start = ic(collinear(0.6 * r_coll), [[0.0; 2]; 3]);
    let mut cadence = WrapperCadence::with_radius(
        &start.m,
        &start.r,
        &start.p,
        CollisionRadius::from_r_coll(r_coll),
    );
    assert_eq!(
        cadence.detector().collision(),
        Verdict {
            fired: true,
            detail: 3
        },
        "the IC's triple did not fire at t = 0"
    );
    let mut s = State {
        m: start.m,
        r: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        p: start.p,
    };
    let mut g = Geom::new(&s.m, &s.r);
    cadence.after_step(&mut s, &mut g);
    assert_eq!(
        cadence.detector().collision(),
        Verdict {
            fired: true,
            detail: 3
        },
        "a later state outside r_coll cleared the collision"
    );
    assert!(cadence.done(), "the cadence is not done after a collision");
}

negative_control!(
    collision_detector_stays_fired,
    "a collision read off the later state alone",
    expected = "cleared the collision",
    {
        let radius = CollisionRadius::from_r_coll(0.01);
        let far = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        assert_eq!(
            classify(&Separations::of(&far), &radius),
            Verdict {
                fired: true,
                detail: 3
            },
            "a later state outside r_coll cleared the collision"
        );
    }
);

/// The comparison is strict, for each pair: a pair at `d² = r_coll²` exactly does not collide, and one whose `d²` is
/// the next f32 below does; `r_coll = 0` fires on nothing, coincident bodies included.
fn check_boundary(fires: fn(f32, &[[f64; 2]; 3]) -> bool) {
    for k in 0..3 {
        // Bodies k + 1 and k + 2 (mod 3) 0.5 apart, body k far: pair k's quantised d² is 0.25 exactly.
        let mut r = [[0.0; 2]; 3];
        r[(k + 1) % 3] = [0.0, 0.0];
        r[(k + 2) % 3] = [0.5, 0.0];
        r[k] = [10.0, 10.0];
        assert_eq!(
            Separations::of(&r).d2[k],
            0.25,
            "the fixture's d² is not 0.25"
        );
        assert!(!fires(0.25, &r), "d² = r_coll² collided, pair {k}");
        assert!(
            fires(0.25f32.next_up(), &r),
            "d² < r_coll² did not collide, pair {k}"
        );
    }
    let coincident = [[0.5, 0.5], [0.5, 0.5], [10.0, 10.0]];
    assert!(!fires(0.0, &coincident), "r_coll = 0 collided");
}

#[test]
fn collision_detector_boundary() {
    check_boundary(|r2, r| classify(&Separations::of(r), &CollisionRadius::from_squared(r2)).fired);
    assert_eq!(
        CollisionRadius::none(),
        CollisionRadius::from_squared(0.0),
        "none is not r_coll = 0"
    );
    let out = integrate::<Kdk, f64>(&inside(), &uniforms(1));
    assert_ne!(
        sd_state(out.packed_a),
        STATE_COLLISION,
        "the run at r_coll = 0 collided"
    );
}

negative_control!(
    collision_detector_boundary,
    "a non-strict comparison, d² ≤ r_coll²",
    expected = "d² = r_coll² collided",
    check_boundary(|r2, r| Separations::of(r).d2.iter().any(|&d| d <= r2))
);

/// The threshold of an `r_coll`: `r_coll²` in f64 rounded once to f32; for `r_coll > 0`, a square at f32's least
/// normal kept, and one below it, subnormal or rounded to 0, raised to that least normal, never written as a subnormal
/// or 0; `r_coll = 0`, 0.
fn check_threshold(of: fn(f64) -> f32) {
    assert_eq!(of(0.5), 0.25, "r_coll 0.5");
    assert_eq!(of(R_COLL_DEFAULT), 1e-6f64 as f32, "r_coll 10⁻³");
    assert_eq!(of(0.1), (0.1f64 * 0.1) as f32, "r_coll 0.1 squared in f64");
    assert_eq!(of(0.0), 0.0, "r_coll 0");
    let least_normal = 2f64.powi(-63);
    assert_eq!(of(least_normal), f32::MIN_POSITIVE, "r_coll² = 2⁻¹²⁶ kept");
    assert_eq!(
        of(least_normal.next_up()),
        ((least_normal.next_up() * least_normal.next_up()) as f32),
        "r_coll² just above 2⁻¹²⁶ not its own square"
    );
    assert_eq!(
        of(2f64.powi(-64)),
        f32::MIN_POSITIVE,
        "r_coll² = 2⁻¹²⁸, subnormal, not raised to 2⁻¹²⁶"
    );
    assert_eq!(
        of(f64::MIN_POSITIVE),
        f32::MIN_POSITIVE,
        "r_coll² rounded to 0 not raised to 2⁻¹²⁶"
    );
}

#[test]
fn collision_detector_threshold() {
    check_threshold(|r| CollisionRadius::from_r_coll(r).squared());
}

negative_control!(
    collision_detector_threshold,
    "a threshold that keeps a subnormal square",
    expected = "subnormal, not raised",
    check_threshold(|r| (r * r) as f32)
);

/// Bodies whose f32 positions coincide (`d² = 0`) collide under every positive `r_coll`, those whose square is
/// subnormal or rounds to 0 included (integrator_contract Part 5, "t=0 terminals are real outcomes"), and not under
/// `r_coll = 0`.
fn check_coincident(radius: fn(f64) -> CollisionRadius) {
    let coincident = [[0.5, 0.5], [0.5, 0.5], [10.0, 10.0]];
    let fires = |r_coll: f64| classify(&Separations::of(&coincident), &radius(r_coll)).fired;
    for r_coll in [
        2f64.powi(-63),
        2f64.powi(-63).next_down(),
        2f64.powi(-64),
        1e-30,
        f64::MIN_POSITIVE,
    ] {
        assert!(
            fires(r_coll),
            "coincident bodies did not collide at r_coll = {r_coll:e}"
        );
    }
    assert!(!fires(0.0), "coincident bodies collided at r_coll = 0");
}

#[test]
fn collision_detector_threshold_coincident() {
    check_coincident(CollisionRadius::from_r_coll);
}

negative_control!(
    collision_detector_threshold_coincident,
    "a threshold that writes a square below f32's least normal as 0",
    expected = "coincident bodies did not collide",
    check_coincident(|r| {
        let r2 = (r * r) as f32;
        CollisionRadius::from_squared(if r2 < f32::MIN_POSITIVE { 0.0 } else { r2 })
    })
);

/// A cadence that keeps nothing: no projection, no detector, never done; its bucket `d²` is the trait's default.
struct Bare;

impl Cadence<f64> for Bare {
    fn after_step(&mut self, _state: &mut State<f64>, _geom: &mut Geom<f64>) {}
    fn done(&self) -> bool {
        false
    }
}

// ── collision_shared_separation ──────────────────────────────────────────────────────────────────────────────────

/// The wrapper's cadence hands the bucket the `d²` its detector formed for the last state, the IC's before the first
/// step: the same bits as the substep law's own `d2_quantised` of that state, whatever positions it is asked about.
fn check_shared(bucket: fn(&WrapperCadence<f64>, &[[f64; 2]; 3]) -> f32) {
    let start = head_on(0, 1);
    let mut cadence = WrapperCadence::with_radius(
        &start.m,
        &start.r,
        &start.p,
        CollisionRadius::from_r_coll(R_COLL_DEFAULT),
    );
    let elsewhere = [[0.0, 0.0], [3.0, 0.0], [0.0, 3.0]];
    assert_eq!(
        bucket(&cadence, &elsewhere).to_bits(),
        d2_quantised(&start.r).to_bits(),
        "before the first step the bucket's d² is not the IC's"
    );
    let mut s = State {
        m: start.m,
        r: [[0.125, -0.25], [0.375, 0.5], [-0.75, 0.0625]],
        p: start.p,
    };
    let mut g = Geom::new(&s.m, &s.r);
    cadence.after_step(&mut s, &mut g);
    assert_eq!(
        bucket(&cadence, &elsewhere).to_bits(),
        d2_quantised(&s.r).to_bits(),
        "after a step the bucket's d² is not the projected state's"
    );
    assert_eq!(
        cadence.detector().last(),
        Separations::of(&s.r),
        "the detector's separations are not the projected state's"
    );
    let mut bare = Bare;
    bare.after_step(&mut s, &mut g);
    assert_eq!(
        bare.bucket_d2(&elsewhere).to_bits(),
        d2_quantised(&elsewhere).to_bits(),
        "a cadence that forms no d² does not hand the bucket the positions' own"
    );
    let want = bucket(&cadence, &elsewhere);
    let mut words = [FreeGroupWord::default()];
    let word = WordCadence::new(&mut cadence, &s.r, 0, &mut words[..]);
    assert_eq!(
        word.bucket_d2(&elsewhere).to_bits(),
        want.to_bits(),
        "the word's cadence does not hand on its inner cadence's d²"
    );
}

#[test]
fn collision_shared_separation() {
    check_shared(|c, r| c.bucket_d2(r));
    for r in [
        [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        [[0.3, -0.1], [0.3, -0.1], [0.0, 0.9]],
        [[1.0, 1.0], [-1.0, 2.0], [0.25, -3.0]],
    ] {
        assert_eq!(
            Separations::of(&r).min().to_bits(),
            d2_quantised(&r).to_bits(),
            "the detector's least d² of {r:?} is not the bucket's"
        );
    }
}

negative_control!(
    collision_shared_separation,
    "a bucket that forms its own d² of the positions it is given",
    expected = "before the first step the bucket's d² is not the IC's",
    check_shared(|_, r| d2_quantised(r))
);

// ── collision_t0_lookup ──────────────────────────────────────────────────────────────────────────────────────────

/// A state with bodies 1 and 2 `0.004` apart, about `0.0056` once rescaled to `I = 1`, body 0 far: pair 0 under
/// `r_coll = 0.01`.
fn close_pair_state() -> Ic<f64> {
    ic(
        [[-1.0, 0.1], [0.5, -0.048], [0.5, -0.052]],
        [[0.0, 0.2], [0.1, -0.1], [-0.1, -0.1]],
    )
}

/// The label of the IC decoded from a state's `z` and of the same IC entered via lookup, by `label`: both
/// `COLLISION_T0` with pair 0, the side opposite body 0, at `t_event = 0`; lookup does not reject it.
fn check_lookup(label: fn(&Ic<f64>, Option<Degenerate>, &CollisionRadius) -> T0) {
    let r_coll = 0.01;
    let radius = CollisionRadius::from_r_coll(r_coll);
    let z = encode::<Defaults, f64>(&close_pair_state()).z;
    let decoded = decode::<Defaults, f64>(&z);
    let d = distances(&decoded.ic.r);
    assert!(
        d[0] < r_coll && d[1] > r_coll && d[2] > r_coll,
        "the decoded IC's pairs are not pair 0 alone inside r_coll: {d:?}"
    );
    let a = label(&decoded.ic, decoded.degenerate, &radius);
    assert_eq!(
        (a.state, a.detail),
        (STATE_COLLISION, 0),
        "the decoded IC is not COLLISION_T0 with pair 0"
    );
    let run = integrate_detecting::<Kdk, f64>(&decoded.ic, &uniforms(10), radius);
    assert_eq!(
        (
            sd_state(run.packed_a),
            sd_detail(run.packed_a),
            tm_t_end_step(run.times)
        ),
        (STATE_COLLISION, 0, 0),
        "the decoded IC's run is not a collision of pair 0 at t_event = 0"
    );

    let entered = encode::<Defaults, f64>(&decoded.ic);
    assert!(
        !entered.report.lookup_clamped(),
        "lookup clamped bodies within r_coll"
    );
    let looked_up = decode::<Defaults, f64>(&entered.z);
    let b = label(&looked_up.ic, looked_up.degenerate, &radius);
    assert_eq!(b, a, "the IC entered via lookup is labelled otherwise");
}

#[test]
fn collision_t0_lookup() {
    check_lookup(t0);
}

negative_control!(
    collision_t0_lookup,
    "a t = 0 pass that names the pair by the lower of its bodies",
    expected = "is not COLLISION_T0 with pair 0",
    check_lookup(|x, tag, radius| {
        let got = t0(x, tag, radius);
        if got.state == STATE_COLLISION && got.detail == 0 {
            T0 {
                state: STATE_COLLISION,
                detail: 1,
            }
        } else {
            got
        }
    })
);

// ── lookup_t0_collision_dispatch ─────────────────────────────────────────────────────────────────────────────────

/// A lookup fixture: bodies 0 and 1 `6·10⁻⁴` apart, `8.5·10⁻⁴` at `I = 1`, inside the default `r_coll`, entered off
/// the canonical gauge (offset and boosted).
fn lookup_fixture() -> Ic<f64> {
    let (offset, boost) = ([3.0, -2.0], [0.5, 0.25]);
    let base = ic(
        [[-0.5, 0.0003], [-0.5, -0.0003], [1.0, 0.0]],
        [[0.05, 0.1], [-0.05, 0.1], [0.0, -0.2]],
    );
    Ic {
        m: base.m,
        r: base.r.map(|q| [q[0] + offset[0], q[1] + offset[1]]),
        p: base
            .p
            .map(|q| [q[0] + THIRD * boost[0], q[1] + THIRD * boost[1]]),
    }
}

/// The lookup fixture, encoded (not rejected), decoded and dispatched through `dispatch`: labelled `collision` with pair
/// 2 at `t_end_step = 0`, having taken no step.
fn check_dispatch(dispatch: fn(&Ic<f64>) -> SimStateBaseOf<f64>) {
    let entered = encode::<Defaults, f64>(&lookup_fixture());
    assert!(
        !entered.report.lookup_clamped(),
        "lookup rejected bodies within r_coll"
    );
    let decoded = decode::<Defaults, f64>(&entered.z);
    let radius = CollisionRadius::from_r_coll(R_COLL_DEFAULT);
    let label = t0(&decoded.ic, decoded.degenerate, &radius);
    assert_eq!(
        (label.state, label.detail),
        (STATE_COLLISION, 2),
        "the looked-up IC does not reach dispatch as a t = 0 collision"
    );
    let out = dispatch(&decoded.ic);
    assert_eq!(
        sd_state(out.packed_a),
        STATE_COLLISION,
        "the dispatched lookup is not labelled collision"
    );
    assert_eq!(
        (
            sd_detail(out.packed_a),
            tm_t_end_step(out.times),
            out.total_substeps
        ),
        (2, 0, 0),
        "the dispatched lookup is not pair 2 at t_end_step = 0"
    );
}

#[test]
fn lookup_t0_collision_dispatch() {
    check_dispatch(|x| {
        integrate_detecting::<Kdk, f64>(
            x,
            &uniforms(100),
            CollisionRadius::from_r_coll(R_COLL_DEFAULT),
        )
    });
}

negative_control!(
    lookup_t0_collision_dispatch,
    "a dispatch that marches without the t = 0 pass (r_coll = 0)",
    expected = "is not labelled collision",
    check_dispatch(|x| integrate::<Kdk, f64>(x, &uniforms(100)))
);

// ── dmin_pair_latch ──────────────────────────────────────────────────────────────────────────────────────────────

/// A fly-by: body 2 leaves body 1, `0.3` away, fast, and passes body 0 about `0.03` off. The closest pair at the start
/// is pair 0, bodies 1 and 2; at the pass pair 1, bodies 2 and 0; and at the end, body 2 far off, pair 2.
fn fly_by() -> Ic<f64> {
    ic(
        [[0.0, 0.0], [-1.5, 0.0], [-1.2, 0.03]],
        [[-0.5, 0.0], [-0.5, 0.0], [1.0, 0.0]],
    )
}

/// A run of the fly-by at `r_coll = 0` (nothing collides), with every state the march passed through.
struct Run {
    out: SimStateBaseOf<f64>,
    states: Vec<[[f64; 2]; 3]>,
}

/// The wrapper's cadence, recording each state it leaves.
struct Recording {
    inner: WrapperCadence<f64>,
    states: Vec<[[f64; 2]; 3]>,
}

impl Cadence<f64> for Recording {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
        self.states.push(state.r);
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
}

/// The fly-by, marched for `1.5` time units.
fn run_fly_by() -> Run {
    let x = fly_by();
    let mut rec = Recording {
        inner: WrapperCadence::new(&x.m, &x.r, &x.p),
        states: vec![x.r],
    };
    let out = integrate_with::<Kdk, f64, _>(&x, &uniforms(1500), &mut rec);
    Run {
        out: monitored(out, &rec.inner),
        states: rec.states,
    }
}

/// The pair `pair_of` reads off the run is the pair of least f64 separation over every state of it, the IC's
/// included, which is neither the IC's closest pair nor the last state's (so the fixture tells them apart).
fn check_latch(pair_of: fn(&Run) -> u32) {
    let run = run_fly_by();
    let closest = |r: &[[f64; 2]; 3]| {
        let d = distances(r);
        (0..3).fold(0, |k, j| if d[j] < d[k] { j } else { k })
    };
    let mut best = (f64::INFINITY, 0);
    for r in &run.states {
        let k = closest(r);
        let d = distances(r)[k];
        if d < best.0 {
            best = (d, k);
        }
    }
    let first = closest(&run.states[0]);
    let last = closest(run.states.last().expect("the run stepped"));
    assert!(
        best.1 != first && best.1 != last,
        "the fixture does not tell the minimum apart: pair {} at {}, first {first}, last {last}",
        best.1,
        best.0
    );
    assert_eq!(
        sd_state(run.out.packed_a),
        STATE_BOUNDED,
        "the fly-by did not run to its horizon"
    );
    assert_eq!(
        pair_of(&run),
        best.1 as u32,
        "dmin_pair is not the pair of minimum separation"
    );
}

#[test]
fn dmin_pair_latch() {
    check_latch(|run| sd_dmin_pair(run.out.packed_a));
}

negative_control!(
    dmin_pair_latch,
    "a dmin_pair read off the last state instead of latched",
    expected = "dmin_pair is not the pair of minimum separation",
    check_latch(|run| Separations::of(run.states.last().expect("stepped"))
        .closest()
        .1)
);

/// The sentinel: a run whose cadence latches nothing keeps `dmin_pair` 3, as the wrapper writes it, and a run whose
/// monitor failed writes 3 over the latched pair, unset as `d_min` is.
#[test]
fn dmin_pair_latch_sentinel() {
    let x = fly_by();
    let out = integrate_with::<Kdk, f64, _>(&x, &uniforms(10), &mut Bare);
    assert_eq!(
        sd_dmin_pair(out.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "dmin_pair's initial value is not 3"
    );
    let (failed, _) = integrate_fixture::<Kdk, f32>(&Fixture {
        name: "momenta whose squares overflow at f32",
        ic: Ic {
            m: [0.25, 0.25, 0.5],
            r: [[-0.5, -0.25], [0.5, -0.25], [0.0, 0.25]],
            p: [[3e19, 0.0], [-3e19, 0.0], [0.0, 0.0]],
        },
        horizon: 0.002,
    });
    assert_eq!(
        sd_state(failed.packed_a),
        STATE_SIM_FAILED,
        "the run did not fail"
    );
    assert_eq!(
        sd_dmin_pair(failed.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "a failed run's dmin_pair is not unset"
    );
    let finished = run_fly_by();
    assert_ne!(
        sd_dmin_pair(finished.out.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "a finished run's dmin_pair is unset"
    );
}

negative_control!(
    dmin_pair_latch_sentinel,
    "a failed run whose latched pair is kept",
    expected = "a failed run's dmin_pair is not unset",
    {
        let finished = run_fly_by();
        assert_eq!(
            sd_dmin_pair(finished.out.packed_a),
            SD_DMIN_PAIR_SENTINEL,
            "a failed run's dmin_pair is not unset"
        );
    }
);

/// The latch's rules at their edges: the least `d²` and its pair, ties to the lowest id; a later state replaces it only
/// when strictly closer, so the first to reach the minimum keeps it; a NaN state never replaces it, and a NaN latch (a
/// NaN IC's) is replaced by the next state.
#[test]
fn dmin_pair_latch_edges() {
    let sep = |d2: [f32; 3]| Separations { d2 };
    for (d2, want) in [
        ([1.0, 1.0, 1.0], 0),
        ([2.0, 1.0, 1.0], 1),
        ([1.0, 2.0, 1.0], 0),
        ([2.0, 2.0, 1.0], 2),
        ([3.0, 2.0, 1.0], 2),
        ([1.0, 2.0, 3.0], 0),
        ([2.0, 1.0, 3.0], 1),
    ] {
        assert_eq!(
            sep(d2).closest(),
            (d2[want as usize], want),
            "closest of {d2:?}"
        );
    }
    let mut latch = DminLatch::new(&sep([4.0, 2.0, 3.0]));
    assert_eq!((latch.d2(), latch.pair()), (2.0, 1), "the latch's start");
    latch.update(&sep([9.0, 9.0, 2.0]));
    assert_eq!(
        (latch.d2(), latch.pair()),
        (2.0, 1),
        "a tie replaced the latch"
    );
    latch.update(&sep([9.0, 9.0, f32::NAN]));
    latch.update(&sep([f32::NAN, 9.0, 9.0]));
    assert_eq!(
        (latch.d2(), latch.pair()),
        (2.0, 1),
        "a NaN state replaced the latch"
    );
    latch.update(&sep([1.5, 9.0, 9.0]));
    assert_eq!(
        (latch.d2(), latch.pair()),
        (1.5, 0),
        "a closer state did not replace it"
    );
    let mut nan = DminLatch::new(&sep([f32::NAN, 1.0, 1.0]));
    assert!(nan.d2().is_nan(), "a NaN IC's latch is not NaN");
    nan.update(&sep([5.0, 4.0, 3.0]));
    assert_eq!(
        (nan.d2(), nan.pair()),
        (3.0, 2),
        "a NaN latch was not replaced"
    );
}

negative_control!(
    dmin_pair_latch_edges,
    "a latch replaced at a tie",
    expected = "a tie replaced the latch",
    {
        let latch = DminLatch::new(&Separations {
            d2: [9.0, 9.0, 2.0],
        });
        assert_eq!(
            (latch.d2(), latch.pair()),
            (2.0, 1),
            "a tie replaced the latch"
        );
    }
);
