//! The collision test's floor, recorded with the run (integrator contract Part 7, "The floor's record, defined";
//! dd_simstate_payload §2, `coll_floor_bits`, and §5; R-954, R-973; REQ-EVT-035):
//! - `r_coll_floor_recorded`: a run through dispatch, the kernel's wrapper and the export decoder, its bodies a binary
//!   whose coordinates stay in `[1, 2)` in `|x|`, where the f32 spacing is `2⁻²³`. At the default `r_coll` = 10⁻³ the
//!   output records that `r_coll` lies above the floor; at `2⁻²⁶`, below the spacing, that it lies below; at 0, that
//!   it lies below; and at `0.75·2⁻²³`, between the spacing at `[0.5, 1)` and at `[1, 2)`, that it lies below. Every
//!   output carries the record, decoded beside the sim key's `r_coll`, bit for bit as defined: the code 18 of `2⁻²³`,
//!   its reserved bits zero.
//!
//! The controls: a recorder that reads `r_coll` alone, taking the floor as the spacing at `[0.5, 1)`, `2⁻²⁴`,
//! misreports the run at `0.75·2⁻²³`; an output that drops the record reads none.
//!
//! **Domain** (pitfalls §10): the record of a short KDK march at f64, whose positions the test bounds; the record's
//! own decisions are `kernel`'s `collision_floor.rs`.

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::validate;
use engine::export::{below_floor, decode, Decoded};
use kernel::decode::Ic;
use kernel::driver::wrapper::integrate_detecting;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{
    cf_coll_floor, ICDescriptor, ReadParams, SimStateBaseOf, SimStateFTLE, COLL_FLOOR_BITS_RESERVED,
};
use validation::negative_control;

/// The code of the floor `2⁻²³`, the f32 spacing at `|x|` in `[1, 2)`: `−23 + 41`.
const CODE_2_POW_MINUS_23: u32 = 18;

/// The binary: bodies 0 and 1 at `(1.2, ±0.1)` on a circular orbit about `(1.2, 0)`, radius 0.1, so each of their
/// coordinates stays in `[1, 2)` in `|x|`, or below it, for the run; body 2 at rest at `(−2.4, 0)`. Equal masses, zero
/// total momentum.
fn binary() -> Ic<f64> {
    let m = 1.0 / 3.0;
    // The relative speed of a circular orbit at separation 0.2 under the pair's mass, 2m: √(2m/0.2).
    let v = 0.5 * (2.0 * m / 0.2f64).sqrt();
    Ic {
        m: [m; 3],
        r: [[1.2, 0.1], [1.2, -0.1], [-2.4, 0.0]],
        p: [[m * v, 0.0], [-m * v, 0.0], [0.0, 0.0]],
    }
}

/// The run of [`binary`] under the sim key with `r_coll`, over the shortest horizon, 0.4, at `dt_macro` 10⁻³, as
/// dispatch accepts and hands it to the kernel.
fn run(r_coll: f64) -> SimStateBaseOf<f64> {
    let v = validate(&SimUniforms {
        r_coll,
        t_horizon: 0.4,
        dt_macro: 1e-3,
        ..SimUniforms::default()
    })
    .expect("the sim key is accepted");
    let out =
        integrate_detecting::<Kdk, f64>(&binary(), &v.wrapper_uniforms_f64(), v.collision_radius());
    let largest = out.r[0]
        .iter()
        .chain(out.r[1].iter())
        .fold(0f64, |a, x| a.max(x.abs()));
    assert!(
        (1.0..2.0).contains(&largest),
        "the binary's largest coordinate left [1, 2): {largest}"
    );
    out
}

/// A run's stored sample at f32, as an output stores it: the f64 run narrowed, its words kept.
type Output = fn(&SimStateBaseOf<f64>) -> SimStateFTLE;

/// The stored sample of `out`, every word kept.
fn stored(out: &SimStateBaseOf<f64>) -> SimStateFTLE {
    let v = |a: [[f64; 2]; 3]| a.map(|p| p.map(|x| x as f32));
    SimStateFTLE {
        r: v(out.r),
        p: v(out.p),
        mean_y: out.mean_y as f32,
        C_ty: out.C_ty as f32,
        E_0: out.E_0 as f32,
        Lz_0: out.Lz_0 as f32,
        packed_a: out.packed_a,
        packed_b: out.packed_b,
        times: out.times,
        total_substeps: out.total_substeps,
        coll_floor_bits: out.coll_floor_bits,
        ..SimStateFTLE::default()
    }
}

/// The output of the run at `r_coll`, decoded by the host export decoder.
fn output(r_coll: f64, out: Output) -> (SimStateFTLE, Decoded) {
    let s = out(&run(r_coll));
    let ic = ICDescriptor {
        m0: 1.0 / 3.0,
        m1: 1.0 / 3.0,
        m2: 1.0 / 3.0,
        ..ICDescriptor::default()
    };
    let params = ReadParams {
        dt_macro: 1e-3,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 400,
    };
    let d = decode(&s, [0; 4], &ic, &params);
    (s, d)
}

/// What an output reads as its flag: whether `r_coll` lay below the floor, from the sim key's `r_coll` and the
/// decoded output.
type Recorder = fn(f64, &Decoded) -> Option<bool>;

/// Each run's output records whether its `r_coll` lay below the floor, the record carried bit for bit.
fn check(recorder: Recorder, out: Output) {
    let between = 0.75 * 2f64.powi(-23);
    for (r_coll, below) in [
        (SimUniforms::default().r_coll, false),
        (2f64.powi(-26), true),
        (0.0, true),
        (between, true),
    ] {
        let (s, d) = output(r_coll, out);
        assert_eq!(
            recorder(r_coll, &d),
            Some(below),
            "the output at r_coll = {r_coll:e} misreports whether it lay below the floor"
        );
        assert_eq!(
            d.coll_floor, CODE_2_POW_MINUS_23,
            "the output at r_coll = {r_coll:e} does not carry the floor 2⁻²³"
        );
        assert_eq!(
            cf_coll_floor(s.coll_floor_bits),
            d.coll_floor,
            "the decoded record is not the stored one"
        );
        for (offset, width) in COLL_FLOOR_BITS_RESERVED {
            assert_eq!(
                (s.coll_floor_bits >> offset) & ((1u64 << width) - 1) as u32,
                0,
                "a reserved bit of the record is set"
            );
        }
    }
}

#[test]
fn r_coll_floor_recorded() {
    check(|r_coll, d| below_floor(r_coll, d.coll_floor), stored);
}

negative_control!(
    r_coll_floor_recorded,
    "a recorder that reads r_coll alone",
    expected = "misreports whether it lay below the floor",
    check(|r_coll, _| Some(r_coll < 2f64.powi(-24)), stored)
);

negative_control!(
    r_coll_floor_recorded_dropped,
    "an output that drops the record",
    expected = "misreports whether it lay below the floor",
    check(
        |r_coll, d| below_floor(r_coll, d.coll_floor),
        |out| SimStateFTLE {
            coll_floor_bits: 0,
            ..stored(out)
        }
    )
);

/// The flag is exact at the floor, reads nothing for an unwritten code, and is true at `r_coll = 0`.
fn check_flag(flag: fn(f64, u32) -> Option<bool>) {
    let floor = 2f64.powi(-23);
    assert_eq!(
        flag(floor, 18),
        Some(false),
        "r_coll at the floor reads below it"
    );
    assert_eq!(
        flag(floor * (1.0 - f64::EPSILON / 2.0), 18),
        Some(true),
        "r_coll just below the floor reads above it"
    );
    assert_eq!(
        flag(0.0, 1),
        Some(true),
        "r_coll = 0 reads above the least floor"
    );
    assert_eq!(
        flag(2f64.powi(104), 145),
        Some(false),
        "the code 145 is not read"
    );
    assert_eq!(flag(1e-3, 0), None, "the code 0 reads a flag");
    assert_eq!(flag(1e-3, 146), None, "an unwritten code reads a flag");
}

#[test]
fn r_coll_floor_recorded_flag() {
    check_flag(below_floor);
}

negative_control!(
    r_coll_floor_recorded_flag,
    "a flag that reads 146",
    expected = "an unwritten code reads a flag",
    check_flag(|r_coll, code| (code != 0).then(|| r_coll < 2f64.powi(code as i32 - 41)))
);

negative_control!(
    r_coll_floor_recorded_flag_145,
    "a flag that stops at 144",
    expected = "the code 145 is not read",
    check_flag(
        |r_coll, code| (code != 0 && code < 145).then(|| r_coll < 2f64.powi(code as i32 - 41))
    )
);
