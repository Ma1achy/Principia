//! The collision threshold dispatch hands the kernel (`engine::dispatch::validate::Validated::collision_radius`;
//! integrator_contract Part 7; TASK-M3-09):
//! - `dispatch_collision_radius`: an accepted sim key's threshold is its own `r_coll`'s, `r_coll²` rounded once to f32,
//!   at the default, at another value and at `r_coll = 0`, and a run under it collides where its IC starts inside
//!   `r_coll`.
//! - `dispatch_collision_threshold_finite`: an `r_coll` whose square rounded to f32 is `+∞` is refused with its reason,
//!   at the exact edge, where the f64 square is the midpoint between `f32::MAX` and `2¹²⁸` and rounds up, and above it;
//!   the `r_coll` one below the edge is accepted with the threshold `f32::MAX` (gpu_determinism_note rule 4).
//!
//! **Domain** (pitfalls §10): the sim key's value handed through; the detector's own tests are kernel's `collision.rs`.

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::{validate, DispatchError, Validated};
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::wrapper::integrate_detecting;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{sd_state, tm_t_end_step, STATE_COLLISION};
use validation::negative_control;

/// The threshold `radius` gives an accepted sim key is `r_coll`'s, at three values of `r_coll`, and a run from an IC
/// with bodies 0 and 1 `5·10⁻⁴` apart collides at step 0 under the default.
fn check_radius(radius: fn(&Validated) -> CollisionRadius) {
    for r_coll in [SimUniforms::default().r_coll, 0.25, 0.0] {
        let v = validate(&SimUniforms {
            r_coll,
            ..SimUniforms::default()
        })
        .expect("an r_coll that is finite and not negative is accepted");
        assert_eq!(
            radius(&v).squared(),
            (r_coll * r_coll) as f32,
            "the threshold handed the kernel is not r_coll = {r_coll}'s"
        );
    }
    let v = validate(&SimUniforms::default()).expect("the default sim key is accepted");
    let ic = Ic {
        m: [1.0 / 3.0; 3],
        r: [[-0.00025, -0.3], [0.00025, -0.3], [0.0, 0.6]],
        p: [[0.0; 2]; 3],
    };
    let out = integrate_detecting::<Kdk, f64>(&ic, &v.wrapper_uniforms_f64(), radius(&v));
    assert_eq!(
        (sd_state(out.packed_a), tm_t_end_step(out.times)),
        (STATE_COLLISION, 0),
        "an IC inside the default r_coll does not collide at step 0"
    );
}

#[test]
fn dispatch_collision_radius() {
    check_radius(Validated::collision_radius);
}

negative_control!(
    dispatch_collision_radius,
    "a threshold that is not the sim key's",
    expected = "is not r_coll",
    check_radius(|_| CollisionRadius::none())
);

/// The rule under test: the threshold dispatch hands the kernel for `u`, or its refusal.
type Rule = fn(&SimUniforms) -> Result<CollisionRadius, DispatchError>;

/// An `r_coll` whose f32 square is `+∞` is refused with its reason, never handed the kernel; the one below the edge is
/// accepted with a finite threshold.
fn check_finite(rule: Rule) {
    let with = |r_coll: f64| {
        rule(&SimUniforms {
            r_coll,
            ..SimUniforms::default()
        })
    };
    // √(2¹²⁸ − 2¹⁰³), correctly rounded: its f64 square is exactly the midpoint between f32::MAX and 2¹²⁸, which rounds
    // to even, 2¹²⁸, +∞ in f32.
    let edge = (2f64.powi(128) - 2f64.powi(103)).sqrt();
    assert_eq!(
        edge * edge,
        2f64.powi(128) - 2f64.powi(103),
        "the fixture's square is not the midpoint"
    );
    assert_eq!(
        with(edge.next_down()).map(|c| c.squared()),
        Ok(f32::MAX),
        "the r_coll below the edge is not accepted at f32::MAX"
    );
    for r_coll in [edge, 2f64.powi(64), 1e100, f64::MAX] {
        assert_eq!(
            with(r_coll),
            Err(DispatchError::CollisionThreshold { r_coll }),
            "r_coll = {r_coll:e}, whose f32 square is +inf, is not refused"
        );
    }
}

#[test]
fn dispatch_collision_threshold_finite() {
    check_finite(|u| validate(u).map(|v| v.collision_radius()));
}

negative_control!(
    dispatch_collision_threshold_finite,
    "a dispatch that refuses only an r_coll not finite or negative",
    expected = "is not refused",
    check_finite(|u| {
        if u.r_coll.is_finite() && u.r_coll >= 0.0 {
            Ok(CollisionRadius::from_r_coll(u.r_coll))
        } else {
            Err(DispatchError::CollisionRadius { r_coll: u.r_coll })
        }
    })
);
