//! QA tests for TASK-M3-09 on dispatch's side, written from the requirements (R-13, R-113; integrator_contract Part 5,
//! Part 7):
//!
//! - REQ-ENC-033 (`qa_lookup_t0_collision_dispatch`): a looked-up IC with bodies within the default `r_coll`, entered
//!   in an arbitrary gauge (scaled, rotated, translated, boosted), is not rejected by lookup, reaches dispatch under
//!   the validated default sim key and its own threshold, and is labelled `collision` with the pair inside, at
//!   `t_end_step = 0`, having taken no step.
//! - REQ-EVT-004, "`r_coll` a recorded parameter carried on every output" (`qa_r_coll_on_the_sim_key`): two sim keys
//!   that differ only in `r_coll` have different sim-key bytes, and each hands the kernel its own threshold.
//!
//! **Domain** (pitfalls §10): one IC, held fixed; the march is KDK at f64 and takes no step.

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::{validate, Validated};
use kernel::decode::{decode, Ic};
use kernel::detect::t0::t0;
use kernel::driver::wrapper::integrate_detecting;
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{sd_detail, sd_state, tm_t_end_step, STATE_COLLISION};
use validation::negative_control;

/// The f64 distance of each pair `k` of `r` by R-22's map: pair 0 = (1, 2), pair 1 = (2, 0), pair 2 = (0, 1).
fn dist(r: &[[f64; 2]; 3]) -> [f64; 3] {
    let d = |a: usize, b: usize| (r[a][0] - r[b][0]).hypot(r[a][1] - r[b][1]);
    [d(1, 2), d(2, 0), d(0, 1)]
}

/// The lookup fixture: bodies 1 and 2 `4·10⁻⁴` apart near `I = 1` (pair 0), body 0 far, then put in another gauge:
/// scaled by `2.5`, rotated by `0.7` rad, translated by `(4, −3)` and boosted by `(0.3, −0.2)`.
fn fixture() -> Ic<f64> {
    let m = [1.0 / 3.0; 3];
    let r0 = [[1.4, 0.0], [-0.7, 0.0002], [-0.7, -0.0002]];
    let p0 = [[0.0, 0.1], [0.05, -0.05], [-0.05, -0.05]];
    let (s, th, a, u) = (2.5_f64, 0.7_f64, [4.0, -3.0], [0.3, -0.2]);
    let rot = |q: [f64; 2]| {
        [
            th.cos() * q[0] - th.sin() * q[1],
            th.sin() * q[0] + th.cos() * q[1],
        ]
    };
    let r = r0.map(|q| {
        let v = rot(q);
        [s * v[0] + a[0], s * v[1] + a[1]]
    });
    let mut p = p0.map(|q| {
        let v = rot(q);
        [v[0] / s.sqrt(), v[1] / s.sqrt()]
    });
    for (k, q) in p.iter_mut().enumerate() {
        q[0] += m[k] * u[0];
        q[1] += m[k] * u[1];
    }
    Ic { m, r, p }
}

/// The fixture entered via lookup (encode, not clamped), decoded, and dispatched under the validated sim key `v`, its
/// loop and its threshold: the decoded IC has exactly one pair within the sim key's `r_coll`, the t = 0 pass labels it
/// `collision` with that pair, and the run stores `collision`, that pair, `t_end_step = 0` and no substep.
fn check_dispatch(v: &Validated) {
    let r_coll = SimUniforms::default().r_coll;
    let entered = encode::<Defaults, f64>(&fixture());
    assert!(
        !entered.report.lookup_clamped(),
        "lookup rejected bodies within r_coll"
    );
    let dec = decode::<Defaults, f64>(&entered.z);
    assert!(
        dec.degenerate.is_none(),
        "the looked-up IC decoded degenerate"
    );
    let d = dist(&dec.ic.r);
    let inside: Vec<usize> = (0..3).filter(|&k| d[k] < r_coll).collect();
    assert_eq!(
        inside.len(),
        1,
        "fixture: not one pair within r_coll: {d:?}"
    );
    let pair = inside[0] as u32;
    let label = t0(&dec.ic, dec.degenerate, &v.collision_radius());
    assert_eq!(
        (label.state, label.detail),
        (STATE_COLLISION, pair),
        "the looked-up IC is not a t = 0 collision at dispatch"
    );
    let out =
        integrate_detecting::<Kdk, f64>(&dec.ic, &v.wrapper_uniforms_f64(), v.collision_radius());
    assert_eq!(
        (
            sd_state(out.packed_a),
            sd_detail(out.packed_a),
            tm_t_end_step(out.times),
            out.total_substeps
        ),
        (STATE_COLLISION, pair, 0, 0),
        "the dispatched lookup is not labelled collision at t_end_step = 0"
    );
}

#[test]
fn qa_lookup_t0_collision_dispatch() {
    check_dispatch(&validate(&SimUniforms::default()).expect("the default sim key is accepted"));
}

negative_control!(
    qa_lookup_t0_collision_dispatch,
    "a dispatch whose sim key refuses to regularise (r_coll = 0) marches the lookup on",
    expected = "is not a t = 0 collision at dispatch",
    check_dispatch(
        &validate(&SimUniforms {
            r_coll: 0.0,
            ..SimUniforms::default()
        })
        .expect("r_coll = 0 is accepted")
    )
);

/// The default sim key and one whose `r_coll` is `alt`, otherwise equal: their sim-key bytes differ, and each validated
/// key's threshold is its own `r_coll`'s square, rounded once to f32.
fn check_key(alt: f64) {
    let base = SimUniforms::default();
    let other = SimUniforms {
        r_coll: alt,
        ..base
    };
    assert_ne!(
        base.sim_key_bytes(),
        other.sim_key_bytes(),
        "a sim key differing only in r_coll has the same bytes"
    );
    for u in [base, other] {
        let v = validate(&u).expect("the sim key is accepted");
        assert_eq!(
            v.collision_radius().squared(),
            (u.r_coll * u.r_coll) as f32,
            "the threshold handed the kernel is not its sim key's r_coll"
        );
    }
}

#[test]
fn qa_r_coll_on_the_sim_key() {
    check_key(2e-3);
}

negative_control!(
    qa_r_coll_on_the_sim_key,
    "a known answer: the same r_coll is not a different key",
    expected = "has the same bytes",
    check_key(SimUniforms::default().r_coll)
);

/// Dispatch accepts the slider's tiny and large `r_coll` (Part 7, "all valid") and hands the kernel a threshold under
/// which an IC within `r_coll` is `collision` at t = 0: two coincident-in-f32 bodies at every tiny positive `r_coll`
/// (one pair), every pair of the lookup fixture at a large one (`detail = 3`) (REQ-EVT-002, REQ-EVT-003).
fn check_extremes(rule: fn(&SimUniforms) -> Validated) {
    let with = |r_coll: f64| {
        rule(&SimUniforms {
            r_coll,
            ..SimUniforms::default()
        })
    };
    let m = [1.0 / 3.0; 3];
    for r_coll in [1e-20, 1e-30, f64::MIN_POSITIVE] {
        let r = [[0.5, 0.0], [0.5 + r_coll / 4.0, 0.0], [-0.5, 0.0]];
        let x = Ic {
            m,
            r,
            p: [[0.0; 2]; 3],
        };
        assert!(dist(&x.r)[2] < r_coll, "fixture: pair 2 not within r_coll");
        let label = t0(&x, None, &with(r_coll).collision_radius());
        assert_eq!(
            (label.state, label.detail),
            (STATE_COLLISION, 2),
            "pair 2 within r_coll = {r_coll:e} is not a t = 0 collision at dispatch"
        );
    }
    for r_coll in [1e10, 1e18] {
        let v = with(r_coll);
        assert!(
            v.collision_radius().squared().is_finite(),
            "the threshold of r_coll = {r_coll:e} is not finite"
        );
        let label = t0(&fixture(), None, &v.collision_radius());
        assert_eq!(
            (label.state, label.detail),
            (STATE_COLLISION, 3),
            "every pair within r_coll = {r_coll:e} is not a triple collision at dispatch"
        );
    }
}

#[test]
fn qa_dispatch_r_coll_extremes() {
    check_extremes(|u| validate(u).expect("the sim key is accepted"));
}

negative_control!(
    qa_dispatch_r_coll_extremes,
    "a dispatch that hands the kernel r_coll = 0 for every r_coll below 10⁻¹⁹",
    expected = "is not a t = 0 collision at dispatch",
    check_extremes(|u| {
        let r_coll = if u.r_coll < 1e-19 { 0.0 } else { u.r_coll };
        validate(&SimUniforms { r_coll, ..*u }).expect("the sim key is accepted")
    })
);
