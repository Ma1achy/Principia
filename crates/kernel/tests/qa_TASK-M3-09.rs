//! QA tests for TASK-M3-09, written from the requirements and their sources, not from the implementation
//! (integrator_contract Part 5, Part 7; dd_integrator §3.6; payload §2; chart_reference §0.7; R-13, R-22, R-60):
//!
//! - REQ-EVT-002 (`qa_t0_inside_r_coll`, `qa_t0_no_escape`, `qa_t0_decode_failed_decoder_only`): the t = 0 pass, a pure
//!   function of the decoded IC and the sim key's `r_coll`. An IC inside `r_coll` is `collision` with its pair at
//!   `t_end_step = 0`, at both precisions, and the march takes no step; an IC that is already escaping by R-29's own
//!   `E_rel` is never `escape` at t = 0 (R-60); `decode_failed` is the decoder's failure only: every one of the fifteen
//!   values non-finite (NaN, `+∞`, `−∞`) is `decode_failed` 0, each `DEGENERATE` reason is the code payload §2 maps it
//!   to, and a valid IC inside `r_coll` is never folded into `decode_failed`.
//! - REQ-EVT-003 (`qa_collision_count_before_classify`, `qa_collision_two_pairs_one_step`,
//!   `qa_collision_head_on_march`): the classification over all eight subsets of pairs inside `r_coll²` (none, one,
//!   two, three), the strict `<` at `d² = r_coll²` exactly, two pairs inside on one observed step is `detail = 3` and
//!   stays, and dd test 6, a head-on pair marched into collision for each of the three pairs, labelled with the R-22 id
//!   (the side opposite the third body), and integrated through at `r_coll = 0`.
//! - REQ-EVT-004 (`qa_collision_shared_d2`): the `d²` the bucket reads after a step is the closest pair's quantised
//!   `d²` of the state the detector observed, by an independent f32 formation here.
//! - REQ-EVT-009 (`qa_collision_t0_lookup_each_pair`): for each pair, a decoded IC with only that pair inside `r_coll`
//!   is `COLLISION_T0` with the opposite-side id and `t_event = 0`; the same IC entered via lookup is not rejected and
//!   has the same label.
//! - REQ-PAY-047 (`qa_dmin_pair_latch`, `qa_dmin_pair_initial`): over a fly-by whose closest pair at the IC, at the
//!   encounter and at the end are three different pairs, `dmin_pair` is the encounter's; a run whose cadence latches
//!   nothing keeps the initial value 3.
//!
//! REQ-ENC-033 is in `crates/engine/tests/qa_TASK-M3-09.rs`, through dispatch's own validated sim key.
//!
//! **Thresholds.** None is picked here: `r_coll` is the sim key's (the default `10⁻³`, integrator_contract Part 3's table;
//! `0.5` only where its square must be exact in f32); every fixture places its pairs a factor of two or more from it.
//!
//! **Domain** (pitfalls §10): the detectors' decisions on the states given; the marches are KDK at f64, short, and show
//! the detector's reading of a march, not the march's accuracy.

use kernel::decode::degenerate::ALL;
use kernel::decode::{decode, Degenerate, Ic};
use kernel::detect::collision::{classify, CollisionRadius, Detector, Separations, Verdict};
use kernel::detect::t0::{t0, T0};
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_detecting, integrate_with, monitored, WrapperUniforms};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_dmin_pair, sd_state, tm_t_end_step, SimStateBaseOf, STATE_BOUNDED,
    STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE, STATE_RUNNING,
};
use validation::negative_control;

/// Equal masses, `Σm = 1`.
const M: f64 = 1.0 / 3.0;

/// The default `r_coll` (integrator_contract Part 3's table).
const R_COLL: f64 = 1e-3;

/// The type of a t = 0 pass.
type Label = fn(&Ic<f64>, Option<Degenerate>, &CollisionRadius) -> T0;

/// The bodies of pair `k`, transcribed from R-22: pair 0 = (1, 2), pair 1 = (2, 0), pair 2 = (0, 1).
fn bodies(k: usize) -> (usize, usize) {
    match k {
        0 => (1, 2),
        1 => (2, 0),
        _ => (0, 1),
    }
}

/// The f64 distance of each pair `k` of `r`, by R-22's map.
fn dist(r: &[[f64; 2]; 3]) -> [f64; 3] {
    let d = |k: usize| {
        let (a, b) = bodies(k);
        (r[a][0] - r[b][0]).hypot(r[a][1] - r[b][1])
    };
    [d(0), d(1), d(2)]
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

/// An equal-mass IC with pair `k`'s bodies `sep` apart about `(0, −0.3)`, moving apart slowly, and the third body at
/// `(0, 0.6)` at rest.
fn pair_ic(k: usize, sep: f64) -> Ic<f64> {
    let (a, b) = bodies(k);
    let mut r = [[0.0, 0.6]; 3];
    let mut p = [[0.0; 2]; 3];
    r[a] = [-sep / 2.0, -0.3];
    r[b] = [sep / 2.0, -0.3];
    p[a] = [-0.01, 0.0];
    p[b] = [0.01, 0.0];
    Ic { m: [M; 3], r, p }
}

/// `x` cast to f32.
fn to_f32(x: &Ic<f64>) -> Ic<f32> {
    let v = |q: [f64; 2]| [q[0] as f32, q[1] as f32];
    Ic {
        m: x.m.map(|m| m as f32),
        r: x.r.map(v),
        p: x.p.map(v),
    }
}

// ── REQ-EVT-002: the t = 0 pass ─────────────────────────────────────────────────────────────────────────────────

/// For each pair, an IC with only that pair `4·10⁻⁴` apart: the pass `label` gives `collision` with that pair's R-22 id,
/// terminal, and the same at f32; the march from it takes no step and stores `collision`, that pair, `t_end_step = 0`,
/// the IC unmoved.
fn check_inside(label: Label) {
    let radius = CollisionRadius::from_r_coll(R_COLL);
    for k in 0..3 {
        let x = pair_ic(k, 4e-4);
        let d = dist(&x.r);
        assert!(
            (0..3).all(|j| (d[j] < R_COLL) == (j == k)),
            "fixture: pair {k} is not the only pair inside r_coll: {d:?}"
        );
        let got = label(&x, None, &radius);
        assert_eq!(
            (got.state, got.detail),
            (STATE_COLLISION, k as u32),
            "an IC with pair {k} inside r_coll is not collision with pair {k} at t = 0"
        );
        assert!(got.terminal(), "a t = 0 collision is not terminal");
        let f = t0(&to_f32(&x), None, &radius);
        assert_eq!(f, got, "the f32 t = 0 pass labels pair {k}'s IC otherwise");
        let run = integrate_detecting::<Kdk, f64>(&x, &uniforms(40), radius);
        assert_eq!(
            (
                sd_state(run.packed_a),
                sd_detail(run.packed_a),
                tm_t_end_step(run.times),
                run.total_substeps
            ),
            (STATE_COLLISION, k as u32, 0, 0),
            "the dispatch of pair {k}'s IC inside r_coll is not collision at t_end_step = 0 without a step"
        );
        assert_eq!(
            (run.r, run.p),
            (x.r, x.p),
            "the dispatch of an IC inside r_coll moved it"
        );
    }
}

#[test]
fn qa_t0_inside_r_coll() {
    check_inside(t0);
}

/// A t = 0 pass naming a pair by the lower of its two bodies, `(1, 2)` → 1, `(2, 0)` → 0, `(0, 1)` → 0, not by the
/// opposite body (R-22's map reversed).
#[cfg(feature = "controls")]
fn lower_body_pass(x: &Ic<f64>, tag: Option<Degenerate>, radius: &CollisionRadius) -> T0 {
    let got = t0(x, tag, radius);
    if got.state == STATE_COLLISION && got.detail < 3 {
        T0 {
            state: got.state,
            detail: [1, 0, 0][got.detail as usize],
        }
    } else {
        got
    }
}

negative_control!(
    qa_t0_inside_r_coll,
    "a t = 0 pass that names a pair by its lower body",
    expected = "is not collision with pair",
    check_inside(lower_body_pass)
);

/// R-29's relative two-body energy of body `b` about the other two's centre of mass, with the total mass:
/// `E_rel = ½|Δv|² − (M_pair + m_b)/d`, `G = 1`.
fn e_rel(x: &Ic<f64>, b: usize) -> f64 {
    let (i, j) = match b {
        0 => (1, 2),
        1 => (2, 0),
        _ => (0, 1),
    };
    let mp = x.m[i] + x.m[j];
    let com = |q: &[[f64; 2]; 3], w: [f64; 2]| {
        [
            (w[0] * q[i][0] + w[1] * q[j][0]) / mp,
            (w[0] * q[i][1] + w[1] * q[j][1]) / mp,
        ]
    };
    let v: [[f64; 2]; 3] = [0, 1, 2].map(|k| [x.p[k][0] / x.m[k], x.p[k][1] / x.m[k]]);
    let rc = com(&x.r, [x.m[i], x.m[j]]);
    let vc = com(&v, [x.m[i], x.m[j]]);
    let dv = [v[b][0] - vc[0], v[b][1] - vc[1]];
    let d = (x.r[b][0] - rc[0]).hypot(x.r[b][1] - rc[1]);
    0.5 * (dv[0] * dv[0] + dv[1] * dv[1]) - (mp + x.m[b]) / d
}

/// An IC already escaping: bodies 0 and 1 a bound pair, body 2 three units off and leaving fast.
fn escaping() -> Ic<f64> {
    Ic {
        m: [M; 3],
        r: [[-0.1, -1.0], [0.1, -1.0], [0.0, 2.0]],
        p: [[0.06, -0.5], [-0.06, -0.5], [0.0, 1.0]],
    }
}

/// The already-escaping IC (its `E_rel(2) > 0`, the outward radial velocity positive): `label` gives `running`, not
/// `escape` (R-60), nor any other terminal.
fn check_no_escape(label: Label) {
    let x = escaping();
    assert!(e_rel(&x, 2) > 0.0, "fixture: body 2 is not unbound");
    let radius = CollisionRadius::from_r_coll(R_COLL);
    let got = label(&x, None, &radius);
    assert_ne!(got.state, STATE_ESCAPE, "an escaping IC is escape at t = 0");
    assert_eq!(
        got,
        T0 {
            state: STATE_RUNNING,
            detail: got.detail
        },
        "an escaping IC is a t = 0 terminal"
    );
}

#[test]
fn qa_t0_no_escape() {
    check_no_escape(t0);
}

/// A t = 0 pass that classifies escape by `E_rel > 0` alone at the IC, which R-60 forbids.
#[cfg(feature = "controls")]
fn eager_escape_pass(x: &Ic<f64>, tag: Option<Degenerate>, radius: &CollisionRadius) -> T0 {
    let got = t0(x, tag, radius);
    match (0..3).find(|&b| e_rel(x, b) > 0.0) {
        Some(b) if !got.terminal() => T0 {
            state: STATE_ESCAPE,
            detail: b as u32,
        },
        _ => got,
    }
}

negative_control!(
    qa_t0_no_escape,
    "a t = 0 pass that labels an unbound body escaped at once",
    expected = "is escape at t = 0",
    check_no_escape(eager_escape_pass)
);

/// The code payload §2 maps each `DEGENERATE` reason to, transcribed: `M01_TINY` → 2, `NO_QUALIFYING_SEED` → 1,
/// `INVARIANT_INFEASIBLE`, `ENERGY_INFEASIBLE` and `ENERGY_UNSCALABLE` → 3.
fn payload_code(d: Degenerate) -> u32 {
    match d {
        Degenerate::M01Tiny => 2,
        Degenerate::NoQualifyingSeed => 1,
        Degenerate::InvariantInfeasible
        | Degenerate::EnergyInfeasible
        | Degenerate::EnergyUnscalable => 3,
    }
}

/// `decode_failed` is the decoder's failure only. From the IC inside `r_coll` (pair 2): each of its fifteen values set
/// in turn to NaN, `+∞` and `−∞` is `decode_failed` 0 (non-finite decode output), never the collision its finite
/// positions would give; each `DEGENERATE` tag on it, and on a far IC, is `decode_failed` with payload §2's code; the
/// valid IC itself is `collision`, never `decode_failed`; a valid far IC is `running`.
fn check_decode_failed(label: Label) {
    let radius = CollisionRadius::from_r_coll(R_COLL);
    let inside = pair_ic(2, 4e-4);
    let far = pair_ic(2, 0.5);
    let valid = label(&inside, None, &radius);
    assert_ne!(
        valid.state, STATE_DECODE_FAILED,
        "a valid IC inside r_coll is folded into decode_failed"
    );
    assert_eq!(
        valid.state, STATE_COLLISION,
        "a valid IC inside r_coll is not collision"
    );
    assert_eq!(
        label(&far, None, &radius).state,
        STATE_RUNNING,
        "a valid IC outside r_coll does not proceed"
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for slot in 0..15 {
            let mut x = inside;
            match slot {
                0..=2 => x.m[slot] = bad,
                3..=8 => x.r[(slot - 3) / 2][(slot - 3) % 2] = bad,
                _ => x.p[(slot - 9) / 2][(slot - 9) % 2] = bad,
            }
            let got = label(&x, None, &radius);
            assert_eq!(
                (got.state, got.detail),
                (STATE_DECODE_FAILED, 0),
                "slot {slot} = {bad} is not decode_failed 0 (non-finite decode output)"
            );
        }
    }
    for reason in ALL {
        for x in [&inside, &far] {
            let got = label(x, Some(reason), &radius);
            assert_eq!(
                (got.state, got.detail),
                (STATE_DECODE_FAILED, payload_code(reason)),
                "{} is not decode_failed with payload §2's code",
                reason.name()
            );
        }
    }
}

#[test]
fn qa_t0_decode_failed_decoder_only() {
    check_decode_failed(t0);
}

/// A t = 0 pass that does not test the decode output's values: only the tag and the collision test.
#[cfg(feature = "controls")]
fn unchecked_pass(x: &Ic<f64>, tag: Option<Degenerate>, radius: &CollisionRadius) -> T0 {
    let mut y = *x;
    for v in y.m.iter_mut() {
        if !v.is_finite() {
            *v = M;
        }
    }
    for q in y.r.iter_mut().chain(y.p.iter_mut()) {
        for v in q.iter_mut() {
            if !v.is_finite() {
                *v = 0.0;
            }
        }
    }
    t0(&y, tag, radius)
}

negative_control!(
    qa_t0_decode_failed_decoder_only,
    "a t = 0 pass blind to non-finite decode output",
    expected = "is not decode_failed 0",
    check_decode_failed(unchecked_pass)
);

// ── REQ-EVT-003: count before classifying ────────────────────────────────────────────────────────────────────────

/// Over every subset of the three pairs inside `r_coll²` (each `d²` a quarter of it inside, four times it outside):
/// none is not fired; exactly one is `collision` with that pair; two or three is `detail = 3`, never a binary label. At
/// `r_coll = 0.5`, whose square `0.25` is exact in f32, a pair at `d² = 0.25` exactly is outside (strict `<`), and one
/// a step of f32 below it inside.
fn check_count(verdict: fn(&Separations, &CollisionRadius) -> Verdict) {
    let radius = CollisionRadius::from_r_coll(R_COLL);
    let t = radius.squared();
    for mask in 0u32..8 {
        let d2 = [0, 1, 2].map(|k| {
            if mask >> k & 1 == 1 {
                0.25 * t
            } else {
                4.0 * t
            }
        });
        let v = verdict(&Separations { d2 }, &radius);
        let n = mask.count_ones();
        assert_eq!(
            v.fired,
            n > 0,
            "pairs {mask:03b} inside: fired is {}",
            v.fired
        );
        if n == 1 {
            assert_eq!(
                v.detail,
                mask.trailing_zeros(),
                "one pair inside ({mask:03b}) is not its pair id"
            );
        } else if n > 1 {
            assert_eq!(
                v.detail, 3,
                "{n} pairs inside ({mask:03b}) is a binary label, not detail = 3"
            );
        }
    }
    let half = CollisionRadius::from_r_coll(0.5);
    assert_eq!(half.squared(), 0.25, "r_coll = 0.5's square is not 0.25");
    let at = Separations {
        d2: [0.25, 1.0, 1.0],
    };
    assert!(
        !verdict(&at, &half).fired,
        "a pair at d² = r_coll² exactly collides (the test is strict)"
    );
    let below = Separations {
        d2: [f32::from_bits(0.25f32.to_bits() - 1), 1.0, 1.0],
    };
    assert_eq!(
        verdict(&below, &half),
        Verdict {
            fired: true,
            detail: 0
        },
        "a pair just inside r_coll² does not collide"
    );
}

#[test]
fn qa_collision_count_before_classify() {
    check_count(classify);
}

/// The classifier Part 7 warns of: "any pair below `r_coll`" tested first, the lowest such pair taken.
#[cfg(feature = "controls")]
fn any_pair_first(s: &Separations, radius: &CollisionRadius) -> Verdict {
    let t = radius.squared();
    match (0..3).find(|&k| s.d2[k] < t) {
        Some(k) => Verdict {
            fired: true,
            detail: k as u32,
        },
        None => Verdict {
            fired: false,
            detail: 0,
        },
    }
}

negative_control!(
    qa_collision_count_before_classify,
    "an any-pair-first classifier steals the triple into the binary arm",
    expected = "is a binary label, not detail = 3",
    check_count(any_pair_first)
);

/// Bodies on a line at `0`, `s`, `2s` (pairs 2 and 0 at `s`, pair 1 at `2s`).
fn collinear(s: f64) -> [[f64; 2]; 3] {
    [[0.0, 0.1], [s, 0.1], [2.0 * s, 0.1]]
}

/// A detector started far apart observes one step at `then`: with pairs 0 and 2 inside `r_coll` (`s = 0.6·r_coll`,
/// pair 1 at `1.2·r_coll` outside) it fires with `detail = 3`, and the label it writes is `collision` 3; a later far
/// state leaves it fired, 3 (terminal).
fn check_two_pairs(then: [[f64; 2]; 3]) {
    let radius = CollisionRadius::from_r_coll(R_COLL);
    let mut det = Detector::new(radius, &collinear(0.4));
    assert!(!det.fired(), "a detector far apart fired");
    det.observe(&then);
    assert_eq!(
        det.collision(),
        Verdict {
            fired: true,
            detail: 3
        },
        "two pairs inside r_coll on one step is not a triple collision"
    );
    let w = det.label(0);
    assert_eq!(
        (sd_state(w), sd_detail(w)),
        (STATE_COLLISION, 3),
        "the label written is not collision 3"
    );
    det.observe(&collinear(0.4));
    assert_eq!(
        det.collision(),
        Verdict {
            fired: true,
            detail: 3
        },
        "a fired collision did not stay"
    );
}

#[test]
fn qa_collision_two_pairs_one_step() {
    let s = 0.6 * R_COLL;
    let d = dist(&collinear(s));
    assert!(
        d[0] < R_COLL && d[2] < R_COLL && d[1] > R_COLL,
        "fixture: not pairs 0 and 2 alone inside: {d:?}"
    );
    check_two_pairs(collinear(s));
}

negative_control!(
    qa_collision_two_pairs_one_step,
    "a known answer: one pair inside (s = 0.6·r_coll, the third body far) is not a triple collision",
    expected = "is not a triple collision",
    check_two_pairs([[0.0, 0.1], [0.6 * R_COLL, 0.1], [0.5, 0.1]])
);

/// dd test 6's head-on pair: pair `k`'s bodies `0.2` apart on a line, closing at `0.6`, the third body `1` off at rest.
fn head_on(k: usize) -> Ic<f64> {
    let (a, b) = bodies(k);
    let mut r = [[0.0, 1.0]; 3];
    let mut p = [[0.0; 2]; 3];
    r[a] = [-0.1, 0.0];
    r[b] = [0.1, 0.0];
    p[a] = [0.1, 0.0];
    p[b] = [-0.1, 0.0];
    Ic { m: [M; 3], r, p }
}

/// For each pair, the head-on march under `radius` (horizon `t = 2`): `collision` with the pair's R-22 id, after at
/// least one step and before the horizon; and at `r_coll = 0` ("refuse to regularise") the same march is not a
/// collision.
fn check_head_on(radius: CollisionRadius) {
    for k in 0..3 {
        let out = integrate_detecting::<Kdk, f64>(&head_on(k), &uniforms(2000), radius);
        assert_eq!(
            (sd_state(out.packed_a), sd_detail(out.packed_a)),
            (STATE_COLLISION, k as u32),
            "the head-on pair {:?} is not collision with pair {k}",
            bodies(k)
        );
        let t = tm_t_end_step(out.times);
        assert!(
            t > 0 && t < 2000,
            "the head-on collision of pair {k} ended at step {t}"
        );
        let through =
            integrate_detecting::<Kdk, f64>(&head_on(k), &uniforms(400), CollisionRadius::none());
        assert_ne!(
            sd_state(through.packed_a),
            STATE_COLLISION,
            "at r_coll = 0 the head-on pair {k} collides"
        );
    }
}

#[test]
fn qa_collision_head_on_march() {
    check_head_on(CollisionRadius::from_r_coll(R_COLL));
}

negative_control!(
    qa_collision_head_on_march,
    "a march with no threshold never collides",
    expected = "is not collision with pair",
    check_head_on(CollisionRadius::none())
);

// ── REQ-EVT-004: the shared d² ───────────────────────────────────────────────────────────────────────────────────

/// The closest pair's quantised `d²`, formed here: each position cast to f32, one subtraction and one square per axis,
/// one addition, the least of the three.
fn qa_d2(r: &[[f64; 2]; 3]) -> f32 {
    let q = r.map(|v| [v[0] as f32, v[1] as f32]);
    let d2 = |a: [f32; 2], b: [f32; 2]| {
        let (x, y) = (b[0] - a[0], b[1] - a[1]);
        x * x + y * y
    };
    d2(q[1], q[2]).min(d2(q[2], q[0])).min(d2(q[0], q[1]))
}

/// A cadence recording the state each step leaves, the wrapper's own inside.
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

/// The run of `x` under `radius` for `h` macro-steps, every state the detector observed recorded (the IC first).
fn record(x: &Ic<f64>, h: u32, radius: CollisionRadius) -> (SimStateBaseOf<f64>, Recording) {
    let mut rec = Recording {
        inner: WrapperCadence::with_radius(&x.m, &x.r, &x.p, radius),
        states: vec![x.r],
    };
    let out = integrate_with::<Kdk, f64, _>(x, &uniforms(h), &mut rec);
    (monitored(out, &rec.inner), rec)
}

/// After every step of a march, the `d²` the next bucket reads (`bucket` of the cadence, given positions that are not
/// the state's, so it cannot form its own from them) is the closest pair's quantised `d²` of the state the step left,
/// formed independently here; and at the IC, the IC's.
fn check_shared(bucket: fn(&WrapperCadence<f64>, &[[f64; 2]; 3]) -> f32) {
    let x = head_on(1);
    let decoy = collinear(7.0);
    let mut cad = WrapperCadence::with_radius(&x.m, &x.r, &x.p, CollisionRadius::none());
    assert_eq!(
        bucket(&cad, &decoy).to_bits(),
        qa_d2(&x.r).to_bits(),
        "the bucket's d² at the IC is not the IC's"
    );
    let (_, rec) = record(&x, 50, CollisionRadius::none());
    assert!(
        rec.states.len() > 50,
        "fixture: the march took too few steps"
    );
    for r in rec.states.iter().skip(1).take(50) {
        let mut state = State {
            m: x.m,
            r: *r,
            p: x.p,
        };
        let mut geom = Geom::new(&state.m, &state.r);
        cad.after_step(&mut state, &mut geom);
        assert_eq!(
            bucket(&cad, &decoy).to_bits(),
            qa_d2(&state.r).to_bits(),
            "the bucket's d² after a step is not the observed state's"
        );
    }
}

#[test]
fn qa_collision_shared_d2() {
    check_shared(|c, r| c.bucket_d2(r));
}

negative_control!(
    qa_collision_shared_d2,
    "a bucket forming its own d² of the positions handed it",
    expected = "is not the IC's",
    check_shared(|_, r| qa_d2(r))
);

// ── REQ-EVT-009: COLLISION_T0 through decode and lookup ──────────────────────────────────────────────────────────

/// A state with pair `k`'s bodies `0.004` apart (a little under `0.006` at `I = 1`), the third body far.
fn close_state(k: usize) -> Ic<f64> {
    let (a, b) = bodies(k);
    let c = 3 - a - b;
    let mut r = [[0.0; 2]; 3];
    let mut p = [[0.0; 2]; 3];
    r[a] = [0.5, -0.048];
    r[b] = [0.5, -0.052];
    r[c] = [-1.0, 0.1];
    p[a] = [0.1, -0.1];
    p[b] = [-0.1, -0.1];
    p[c] = [0.0, 0.2];
    Ic { m: [M; 3], r, p }
}

/// For each pair: the IC decoded from the state's `z`, under `r_coll = 0.01`, has that pair alone inside (by the f64
/// distances here); the pass gives `COLLISION_T0` with the pair's R-22 id and the run stores it at `t_event = 0`; the
/// same IC entered via lookup is not rejected (`lookup_clamped` false) and gets the same label.
fn check_lookup(label: Label) {
    let r_coll = 0.01;
    let radius = CollisionRadius::from_r_coll(r_coll);
    for k in 0..3 {
        let z = encode::<Defaults, f64>(&close_state(k)).z;
        let dec = decode::<Defaults, f64>(&z);
        let d = dist(&dec.ic.r);
        let inside: Vec<usize> = (0..3).filter(|&j| d[j] < r_coll).collect();
        assert_eq!(
            inside.len(),
            1,
            "fixture: not one pair inside r_coll: {d:?}"
        );
        let pair = inside[0];
        let a = label(&dec.ic, dec.degenerate, &radius);
        assert_eq!(
            (a.state, a.detail),
            (STATE_COLLISION, pair as u32),
            "the decoded IC with pair {pair} inside is not COLLISION_T0({pair})"
        );
        let run = integrate_detecting::<Kdk, f64>(&dec.ic, &uniforms(10), radius);
        assert_eq!(
            (
                sd_state(run.packed_a),
                sd_detail(run.packed_a),
                tm_t_end_step(run.times)
            ),
            (STATE_COLLISION, pair as u32, 0),
            "the decoded IC's run is not COLLISION_T0({pair}) at t_event = 0"
        );
        let entered = encode::<Defaults, f64>(&dec.ic);
        assert!(
            !entered.report.lookup_clamped(),
            "lookup rejected bodies within r_coll"
        );
        let looked = decode::<Defaults, f64>(&entered.z);
        let b = label(&looked.ic, looked.degenerate, &radius);
        assert_eq!(b, a, "the IC entered via lookup is labelled otherwise");
    }
}

#[test]
fn qa_collision_t0_lookup_each_pair() {
    check_lookup(t0);
}

negative_control!(
    qa_collision_t0_lookup_each_pair,
    "a t = 0 pass that names a pair by its lower body",
    expected = "is not COLLISION_T0",
    check_lookup(lower_body_pass)
);

// ── REQ-PAY-047: the dmin_pair latch ─────────────────────────────────────────────────────────────────────────────

/// A fly-by: body 2 starts `0.2` from body 1 (pair 0 closest), runs past body 0 about `0.1` off (pair 1 at the
/// encounter), and leaves; at the end bodies 0 and 1, `1` apart, are the closest (pair 2). Total momentum zero.
fn fly_by() -> Ic<f64> {
    Ic {
        m: [M; 3],
        r: [[1.0, 0.1], [0.0, 0.0], [0.2, 0.0]],
        p: [[-0.75, 0.0], [-0.75, 0.0], [1.5, 0.0]],
    }
}

/// The R-22 id of the closest pair of `r` by f64 distance.
fn closest(r: &[[f64; 2]; 3]) -> usize {
    let d = dist(r);
    (0..3).fold(0, |k, j| if d[j] < d[k] { j } else { k })
}

/// The fly-by marched to `t = 0.6` at `r_coll = 0`: the pair of least f64 separation over every state the detector
/// observed, the IC's included, is a pair other than the IC's and the last state's closest, and below every other
/// pair's least by a factor of two; the `dmin_pair` the run stores, read by `pair_of`, is that pair.
fn check_latch(pair_of: fn(&SimStateBaseOf<f64>, &[[f64; 2]; 3]) -> u32) {
    let (out, rec) = record(&fly_by(), 600, CollisionRadius::none());
    assert_eq!(
        sd_state(out.packed_a),
        STATE_BOUNDED,
        "fixture: the fly-by did not reach its horizon"
    );
    let mut least = [f64::INFINITY; 3];
    for r in &rec.states {
        for (l, d) in least.iter_mut().zip(dist(r)) {
            *l = l.min(d);
        }
    }
    let best = (0..3).fold(0, |k, j| if least[j] < least[k] { j } else { k });
    let first = closest(&rec.states[0]);
    let last_r = *rec.states.last().expect("the run stepped");
    let last = closest(&last_r);
    assert!(
        best != first && best != last && first != last,
        "fixture: IC {first}, encounter {best}, end {last} are not three pairs ({least:?})"
    );
    assert!(
        (0..3).all(|j| j == best || least[best] * 2.0 < least[j]),
        "fixture: the least separation is not clear of the others: {least:?}"
    );
    assert_eq!(
        pair_of(&out, &last_r),
        best as u32,
        "dmin_pair is not the pair of minimum separation"
    );
}

#[test]
fn qa_dmin_pair_latch() {
    check_latch(|out, _| sd_dmin_pair(out.packed_a));
}

negative_control!(
    qa_dmin_pair_latch,
    "a dmin_pair read off the last state, not latched",
    expected = "dmin_pair is not the pair of minimum separation",
    check_latch(|_, r| closest(r) as u32)
);

/// A cadence that keeps nothing: no latch is written.
struct Bare;

impl Cadence<f64> for Bare {
    fn after_step(&mut self, _state: &mut State<f64>, _geom: &mut Geom<f64>) {}

    fn done(&self) -> bool {
        false
    }
}

/// The `dmin_pair` the loop writes before any latch is 3, unset, for a run of `h` steps; and once the wrapper's latch is
/// written over it, it is a pair id (0–2).
fn check_initial(h: u32) {
    let out = integrate_with::<Kdk, f64, _>(&fly_by(), &uniforms(h), &mut Bare);
    assert_eq!(
        sd_dmin_pair(out.packed_a),
        3,
        "dmin_pair's initial value is not 3"
    );
    let (latched, _) = record(&fly_by(), 5, CollisionRadius::none());
    assert!(
        sd_dmin_pair(latched.packed_a) < 3,
        "a latched dmin_pair is unset"
    );
}

#[test]
fn qa_dmin_pair_initial() {
    check_initial(0);
    check_initial(10);
}

negative_control!(
    qa_dmin_pair_initial,
    "a known answer: the wrapper's latched run is not unset",
    expected = "dmin_pair's initial value is not 3",
    {
        let (out, _) = record(&fly_by(), 5, CollisionRadius::none());
        assert_eq!(
            sd_dmin_pair(out.packed_a),
            3,
            "dmin_pair's initial value is not 3"
        );
    }
);
