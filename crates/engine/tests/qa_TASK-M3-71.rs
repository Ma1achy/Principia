//! QA tests for the collision floor's record, written from integrator contract Part 7 ("The floor's record, defined")
//! and dd_simstate_payload §2, `coll_floor_bits`, and §5, not from the implementation: runs
//! through dispatch's validation, the kernel's wrapper and the host export decoder.
//! - The flag follows the pair's coordinates, not `r_coll` alone: the same `r_coll`, `1.5·2⁻²⁴`, between the f32
//!   spacing at `[0.5, 1)` (`2⁻²⁴`) and at `[1, 2)` (`2⁻²³`), lies above the floor of a run whose closest pair sits
//!   in `[0.5, 1)` and below that of the same run scaled by 2, into `[1, 2)`. At the default `r_coll` both lie
//!   above; at `2⁻²⁶` and at 0, both below. Every output carries the record beside `r_coll`, bit for bit: the
//!   code (17 for `2⁻²⁴`, 18 for `2⁻²³`) in bits 0–7 of `coll_floor_bits`, its reserved bits zero.
//! - A failed sample records no floor: the code 0, which reads no flag.
//!
//! Each test has its negative controls.

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::validate;
use engine::export::{below_floor, decode, Decoded};
use kernel::decode::Ic;
use kernel::driver::wrapper::integrate_detecting;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{
    sd_state, ICDescriptor, ReadParams, SimStateBaseOf, SimStateFTLE, STATE_SIM_FAILED,
};
use validation::negative_control;

const M: f64 = 1.0 / 3.0;

/// Three equal masses at rest, the pair (0, 1) at `(±0.75 s, 0.6 s)` and body 2 at `(0, −1.2 s)`: the centre of mass
/// at the origin, the pair (0, 1) the closest (`1.5 s` against `1.95 s`). At `s = 1` the pair's coordinates lie in
/// `[0.5, 1)`, at `s = 2` in `[1, 2)`; over the short horizon they move by about 1 %.
fn ic(s: f64) -> Ic<f64> {
    Ic {
        m: [M; 3],
        r: [[0.75 * s, 0.6 * s], [-0.75 * s, 0.6 * s], [0.0, -1.2 * s]],
        p: [[0.0; 2]; 3],
    }
}

fn run(ic: &Ic<f64>, r_coll: f64) -> SimStateBaseOf<f64> {
    let v = validate(&SimUniforms {
        r_coll,
        t_horizon: 0.4,
        dt_macro: 1e-3,
        ..SimUniforms::default()
    })
    .expect("the sim key is accepted");
    integrate_detecting::<Kdk, f64>(ic, &v.wrapper_uniforms_f64(), v.collision_radius())
}

/// The run's sample at f32, as the payload stores it.
fn stored(out: &SimStateBaseOf<f64>) -> SimStateFTLE {
    let v = |a: [[f64; 2]; 3]| a.map(|p| p.map(|x| x as f32));
    SimStateFTLE {
        r: v(out.r),
        p: v(out.p),
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

fn decoded(s: &SimStateFTLE) -> Decoded {
    let ic = ICDescriptor {
        m0: M as f32,
        m1: M as f32,
        m2: M as f32,
        ..ICDescriptor::default()
    };
    let params = ReadParams {
        dt_macro: 1e-3,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 400,
    };
    decode(s, [0; 4], &ic, &params)
}

/// An output's sample as written: `stored`, or a variant of it.
type Output = fn(&SimStateBaseOf<f64>) -> SimStateFTLE;

/// What an output reads as its flag, from the sim key's `r_coll`, the decoded output and the stored sample.
type Recorder = fn(f64, &Decoded, &SimStateFTLE) -> Option<bool>;

fn the_flag(r_coll: f64, d: &Decoded, _: &SimStateFTLE) -> Option<bool> {
    below_floor(r_coll, d.coll_floor)
}

fn check(recorder: Recorder, output: Output) {
    let between = 1.5 * 2f64.powi(-24);
    // Each r_coll with whether it lies below the floor, in the [0.5, 1) run and in the [1, 2) run.
    let cases = [
        (SimUniforms::default().r_coll, [false, false]),
        (between, [false, true]),
        (2f64.powi(-26), [true, true]),
        (0.0, [true, true]),
    ];
    let bands = [
        ("[0.5, 1)", 1.0, 17, 0.5, 1.0),
        ("[1, 2)", 2.0, 18, 1.0, 2.0),
    ];
    for (i, &(band, scale, code, lo, hi)) in bands.iter().enumerate() {
        for &(r_coll, below) in &cases {
            let want = below[i];
            let out = run(&ic(scale), r_coll);
            // The premise: the pair (0, 1) stays in its band over the run.
            for x in out.r[0].iter().chain(out.r[1].iter()) {
                assert!(
                    (lo..hi).contains(&x.abs()),
                    "the {band} run's pair left its band: {x}"
                );
            }
            let s = output(&out);
            let d = decoded(&s);
            assert_eq!(
                recorder(r_coll, &d, &s),
                Some(want),
                "the {band} run at r_coll = {r_coll:e} misreports its flag"
            );
            assert_eq!(
                d.coll_floor, code,
                "the {band} run at r_coll = {r_coll:e} does not carry its floor's code"
            );
            assert_eq!(
                s.coll_floor_bits, code,
                "the {band} run's stored word is not the code in bits 0-7 over zero"
            );
        }
    }
}

#[test]
fn qa_flag_follows_the_pairs_coordinates() {
    check(the_flag, stored);
}

negative_control!(
    qa_flag_follows_the_pairs_coordinates,
    "a flag from r_coll alone, the floor taken as 2^-23",
    expected = "the [0.5, 1) run at r_coll = 8.940696716308594e-8 misreports its flag",
    check(|r_coll, _, _| Some(r_coll < 2f64.powi(-23)), stored)
);

negative_control!(
    qa_flag_follows_the_pairs_coordinates_lower,
    "a flag from r_coll alone, the floor taken as 2^-24",
    expected = "the [1, 2) run at r_coll = 8.940696716308594e-8 misreports its flag",
    check(|r_coll, _, _| Some(r_coll < 2f64.powi(-24)), stored)
);

negative_control!(
    qa_flag_follows_the_pairs_coordinates_all_bodies,
    "a flag from the largest coordinate of all three bodies, not the pair's",
    expected = "the [0.5, 1) run at r_coll = 8.940696716308594e-8 misreports its flag",
    check(
        |r_coll, _, s| {
            let x = s.r.iter().flatten().fold(0f32, |a, x| a.max(x.abs()));
            let e = ((x.to_bits() >> 23) & 0xff) as i32 - 127;
            Some(r_coll < 2f64.powi(e - 23))
        },
        stored
    )
);

negative_control!(
    qa_flag_follows_the_pairs_coordinates_dropped,
    "an output that drops the record",
    expected = "the [0.5, 1) run at r_coll = 1e-3 misreports its flag",
    check(the_flag, |out| SimStateFTLE {
        coll_floor_bits: 0,
        ..stored(out)
    })
);

negative_control!(
    qa_flag_follows_the_pairs_coordinates_reserved,
    "an output that writes a reserved bit",
    expected = "stored word is not the code in bits 0-7 over zero",
    check(the_flag, |out| {
        let s = stored(out);
        SimStateFTLE {
            coll_floor_bits: s.coll_floor_bits | (1 << 8),
            ..s
        }
    })
);

/// A failed sample: its state `sim_failed`, its record 0, which reads no flag.
fn check_failed(output: Output) {
    let mut bad = ic(1.0);
    bad.p[0][0] = f64::NAN;
    bad.p[1][0] = f64::NAN;
    let out = run(&bad, 1e-3);
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "the non-finite run is not marked sim_failed"
    );
    let s = output(&out);
    let d = decoded(&s);
    assert_eq!(
        (s.coll_floor_bits, d.coll_floor),
        (0, 0),
        "a failed sample records a floor"
    );
    assert_eq!(
        below_floor(1e-3, d.coll_floor),
        None,
        "a failed sample reads a flag"
    );
}

#[test]
fn qa_failed_sample_records_no_floor() {
    check_failed(stored);
}

negative_control!(
    qa_failed_sample_records_no_floor,
    "a failed sample that keeps a floor",
    expected = "a failed sample records a floor",
    check_failed(|out| SimStateFTLE {
        coll_floor_bits: 17,
        ..stored(out)
    })
);
