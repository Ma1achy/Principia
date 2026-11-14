//! The collision test's floor and its record (`kernel::detect::collision`; integrator contract Part 7, "The floor's
//! record, defined"; dd_simstate_payload §2, `coll_floor_bits`; R-954, R-973; REQ-EVT-035):
//! - `collision_floor_code`: a pair's floor code is `clamp(e − 23, −40, 104) + 41`, at each end of the clamp and on
//!   both sides of it, and the code's floor is `2^(code − 41)`.
//! - `collision_floor_pairs`: each R-22 pair's code reads the largest exponent field of its own four coordinates, cast
//!   to f32 first, a negative or a `y` coordinate included.
//! - `collision_floor_near`: near firing is strict at `d² = t·t`, holds where the test fires whatever `t`, and holds
//!   without the product from `t ≥ 2⁶⁴`, not at `t` just below it.
//! - `collision_floor_record`: the record is the larger of the near-firing maximum and the `d_min` state's code; a
//!   near-firing state the latch later leaves still counts; states after the test fires do not; the latch's code moves
//!   only where the latch does, never on a tie.
//! - `collision_floor_written`: the wrapper writes the record into `coll_floor`, and 0 for a failed run.
//!
//! **Domain** (pitfalls §10): the definition's decisions on the positions given, identical on every backend given
//! identical positions; the one march, KDK at f64, is short, and shows only that the wrapper writes what the detector
//! holds.

use kernel::decode::Ic;
use kernel::detect::collision::{
    floor_code_of_exponent, floor_codes, floor_of_code, near_firing, CollisionRadius, Detector,
};
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_detecting, WrapperUniforms};
use kernel::occupant::kdk::Kdk;
use kernel::payload::{cf_coll_floor, sd_state, STATE_SIM_FAILED};
use validation::negative_control;

/// `2^e` in f32.
fn pow2(e: i32) -> f32 {
    2f32.powi(e)
}

/// The floor code at each end of the clamp, on both sides of it and in the middle, and the floor of a code.
fn check_code(code: fn(i32) -> u32) {
    for (e, want) in [
        (-127, 1),
        (-18, 1),
        (-17, 1),
        (-16, 2),
        (-1, 17),
        (0, 18),
        (1, 19),
        (126, 144),
        (127, 145),
        (128, 145),
    ] {
        assert_eq!(code(e), want, "the floor code of exponent {e} is wrong");
    }
    for (c, e) in [(1, -40), (18, -23), (17, -24), (145, 104)] {
        assert_eq!(floor_of_code(c), pow2(e), "code {c}'s floor is not 2^{e}");
    }
}

#[test]
fn collision_floor_code() {
    check_code(floor_code_of_exponent);
}

negative_control!(
    collision_floor_code,
    "a code with no lower clamp",
    expected = "the floor code of exponent",
    check_code(|e| (e - 23 + 41).clamp(0, 145) as u32)
);

negative_control!(
    collision_floor_code_upper,
    "a code with no upper clamp",
    expected = "the floor code of exponent",
    check_code(|e| ((e - 23).max(-40) + 41) as u32)
);

/// Each pair's code reads its own coordinates' largest exponent, after the f32 cast.
fn check_pairs(codes: fn(&[[f64; 2]; 3]) -> [u32; 3]) {
    // Body 0's largest is |y| = 3 (exponent 1), body 1's 0.75 (exponent −1), body 2's −1.5 (exponent 0).
    let r = [[0.25, -3.0], [0.75, 0.5], [-1.5, 0.125]];
    assert_eq!(
        codes(&r),
        [18, 19, 19],
        "a pair's code is not its own coordinates' largest exponent"
    );
    // 2 − 2⁻³⁰ casts to 2.0 in f32, exponent 1, not 0: the f32 cast decides.
    let r = [[2.0 - 2f64.powi(-30), 0.0], [0.5, 0.0], [0.5, 0.0]];
    assert_eq!(
        codes(&r),
        [17, 19, 19],
        "a pair's code is not read after the f32 cast"
    );
}

#[test]
fn collision_floor_pairs() {
    check_pairs(floor_codes::<f64>);
}

negative_control!(
    collision_floor_pairs,
    "a code that reads x alone",
    expected = "a pair's code is not its own coordinates' largest exponent",
    check_pairs(|r| floor_codes::<f64>(&r.map(|b| [b[0], 0.0])))
);

negative_control!(
    collision_floor_pairs_cast,
    "a code read before the f32 cast",
    expected = "a pair's code is not read after the f32 cast",
    check_pairs(|r| {
        let e = |b: [f64; 2]| {
            let m = b[0].abs().max(b[1].abs());
            if m == 0.0 {
                -127
            } else {
                m.log2().floor() as i32
            }
        };
        let code = |a: [f64; 2], b: [f64; 2]| floor_code_of_exponent(e(a).max(e(b)));
        [code(r[1], r[2]), code(r[2], r[0]), code(r[0], r[1])]
    })
);

/// The near-firing rule.
type Near = fn(f32, u32, &CollisionRadius) -> bool;

/// Near firing is strict at `d² = t·t`, holds where the test fires, and holds without the product from `t ≥ 2⁶⁴`.
fn check_near(near: Near) {
    // r = 0.5, floor 2⁻²³: t = 0.5 + 2⁻²³ exactly, and t·t = 0.25 + 2⁻²³ + 2⁻⁴⁶ rounds to 0.25 + 2⁻²³.
    let radius = CollisionRadius::from_squared(0.25);
    assert_eq!(radius.root(), 0.5, "the root of 0.25 is not 0.5");
    let tt = 0.25 + pow2(-23);
    assert!(
        !near(tt, 18, &radius),
        "near firing is not strict at d² = t·t"
    );
    assert!(
        near(f32::from_bits(tt.to_bits() - 1), 18, &radius),
        "a d² just below t·t is not near firing"
    );
    // A root far below √(r_coll²): t·t is below the threshold, and a firing pair is still near firing.
    let low = CollisionRadius::from_parts(0.25, 0.0);
    assert!(near(0.2, 1, &low), "a firing pair is not near firing");
    assert!(
        !near(0.25, 1, &low),
        "a pair at d² = r_coll² is near firing"
    );
    // t at 2⁶⁴ is near firing without the product, even for a d² of +∞, which no comparison is below; t just below
    // it compares d² with t·t, which +∞ is not below.
    let big = CollisionRadius::from_parts(0.0, pow2(64));
    assert!(near(f32::INFINITY, 1, &big), "t = 2⁶⁴ is not near firing");
    let below = CollisionRadius::from_parts(0.0, f32::from_bits(pow2(64).to_bits() - 1));
    assert!(
        !near(f32::INFINITY, 1, &below),
        "t just below 2⁶⁴ does not compare d² with t·t"
    );
    // At r_coll = 0, near firing is d² < floor²: 2⁻⁴⁶ against 2⁻²³, squared.
    let none = CollisionRadius::none();
    assert!(
        near(pow2(-47), 18, &none),
        "r_coll = 0 is not near firing below the floor"
    );
    assert!(
        !near(pow2(-46), 18, &none),
        "r_coll = 0 is near firing at the floor"
    );
}

#[test]
fn collision_floor_near() {
    check_near(near_firing);
}

negative_control!(
    collision_floor_near,
    "near firing that is not strict",
    expected = "near firing is not strict",
    check_near(|d2, code, radius| {
        let t = radius.root() + floor_of_code(code);
        d2 < radius.squared() || d2 <= t * t
    })
);

negative_control!(
    collision_floor_near_fires,
    "near firing without the firing clause",
    expected = "a firing pair is not near firing",
    check_near(|d2, code, radius| {
        let t = radius.root() + floor_of_code(code);
        t >= pow2(64) || d2 < t * t
    })
);

negative_control!(
    collision_floor_near_overflow,
    "near firing without the 2⁶⁴ clause",
    expected = "t = 2⁶⁴ is not near firing",
    check_near(|d2, code, radius| {
        let t = radius.root() + floor_of_code(code);
        d2 < radius.squared() || d2 < t * t
    })
);

negative_control!(
    collision_floor_near_63,
    "the 2⁶⁴ clause one exponent early",
    expected = "t just below 2⁶⁴ does not compare",
    check_near(|d2, code, radius| {
        let t = radius.root() + floor_of_code(code);
        d2 < radius.squared() || t >= pow2(63) || d2 < t * t
    })
);

/// Three bodies: the pair (0, 1) at `(c, 0)` and `(c + d, 0)`, body 2 far off at `(−4, 0)`.
fn pair_at(c: f32, d: f32) -> [[f32; 2]; 3] {
    [[c, 0.0], [c + d, 0.0], [-4.0, 0.0]]
}

/// What the record reads from a detector.
type Record = fn(&Detector) -> u32;

/// The record over a run of hand-made states.
fn check_record(record: Record) {
    let r = 1e-7f32;
    let radius = CollisionRadius::from_squared(r * r);
    // State A: the pair at 1.5, one f32 spacing apart, 2⁻²³ = 1.19e-7 ≥ r: not firing, but near (code 18).
    let a = pair_at(1.5, pow2(-23));
    // State B: the pair at 10⁻³ (code 8), 1.1e-7 apart: closer than A, so the latch moves, and neither firing nor near.
    let b = pair_at(1e-3, 1.1e-7);
    let mut det = Detector::new(radius, &a);
    assert!(!det.fired(), "state A fires");
    det.observe(&b);
    assert!(!det.fired(), "state B fires");
    assert_eq!(det.latched_code(), 8, "the latch's code is not state B's");
    assert_eq!(
        record(&det),
        18,
        "the record misses a near-firing state the latch left"
    );
    // A run that never comes near: the record is the d_min state's code.
    let far = pair_at(1e-3, 1e-4);
    let det = Detector::new(radius, &far);
    assert_eq!(det.near_code(), 0, "a far pair is near firing");
    assert_eq!(record(&det), 8, "the record is not the d_min state's code");
    // A run that fires at C (code 8) and then meets a near-firing pair at 1.5 (code 18), farther than C: the test no
    // longer reads, so the record stays C's.
    let c = pair_at(1e-3, 5e-8);
    let mut det = Detector::new(radius, &far);
    det.observe(&c);
    assert!(det.fired(), "state C does not fire");
    det.observe(&a);
    assert_eq!(
        record(&det),
        8,
        "the record reads a state after the test fired"
    );
    // A tie with the latched d² (2⁻¹² apart, exactly, in both) at another code (16) keeps the latch and its code.
    let mut det = Detector::new(radius, &pair_at(1e-3, pow2(-12)));
    det.observe(&pair_at(0.25, pow2(-12)));
    assert_eq!(det.latched_code(), 8, "a tie moved the latch's code");
}

#[test]
fn collision_floor_record() {
    check_record(Detector::floor_code);
}

negative_control!(
    collision_floor_record,
    "a record of the d_min state alone",
    expected = "the record misses a near-firing state the latch left",
    check_record(Detector::latched_code)
);

negative_control!(
    collision_floor_record_near,
    "a record of the near-firing states alone",
    expected = "the record is not the d_min state's code",
    check_record(Detector::near_code)
);

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

/// What the wrapper writes into `coll_floor` for a run.
type Write = fn(&Ic<f64>, &WrapperUniforms<f64>, CollisionRadius) -> u32;

/// The wrapper writes the detector's record, and 0 for a failed run.
fn check_written(write: Write) {
    let third = 1.0 / 3.0;
    // The pair (0, 1) at x = 1.2, 0.2 apart, body 2 at −2.4: the d_min state's pair has its largest |x| in [1, 2).
    let ic = Ic {
        m: [third; 3],
        r: [[1.2, 0.1], [1.2, -0.1], [-2.4, 0.0]],
        p: [[0.0; 2]; 3],
    };
    let radius = CollisionRadius::from_r_coll(1e-3);
    assert_eq!(
        write(&ic, &uniforms(4), radius),
        18,
        "the record written is not 2⁻²³'s code"
    );
    let nan = Ic {
        p: [[f64::NAN, 0.0], [0.0; 2], [0.0; 2]],
        ..ic
    };
    let out = integrate_detecting::<Kdk, f64>(&nan, &uniforms(4), radius);
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "the NaN run is not sim_failed"
    );
    assert_eq!(
        write(&nan, &uniforms(4), radius),
        0,
        "a failed run's record is not 0"
    );
}

#[test]
fn collision_floor_written() {
    check_written(|ic, u, radius| {
        cf_coll_floor(integrate_detecting::<Kdk, f64>(ic, u, radius).coll_floor_bits)
    });
}

negative_control!(
    collision_floor_written,
    "a failed run's record kept",
    expected = "a failed run's record is not 0",
    check_written(|ic, _, radius| Detector::new(radius, &ic.r).floor_code())
);
