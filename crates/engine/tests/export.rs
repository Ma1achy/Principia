//! The host export decoder (render contract Part 5 (c); RQ-217; TASK-M1-08), generated from the layout table into
//! `engine::export`: a synthetic sample decodes to every ledger field, each through its accessor, the vectors
//! flattened, the shadow from the stored struct and the descriptor's fields from the descriptor (REQ-GEN-010). Each
//! test has a registered negative control (R-176).

use engine::export::{decode, Decoded};
use kernel::payload::*;
use validation::negative_control;

/// A sample with every stored field set: its `SimStateFTLE`, word and `ICDescriptor`.
fn sample() -> (SimStateFTLE, [u32; 4], ICDescriptor) {
    let mut s = SimStateFTLE {
        r: [[1.0, 2.0], [3.0, 4.0], [5.0, 6.5]],
        p: [[0.5, 0.25], [-0.5, 0.75], [0.0, -1.0]],
        r_sh: [[1.5, 2.0], [3.0, 4.0], [5.0, 6.5]],
        p_sh: [[0.5, 0.25], [-0.5, 0.75], [0.0, -1.25]],
        S: 0.75,
        theta: -7.0,
        mean_y: 0.125,
        C_ty: 3.0,
        E_0: -2.5,
        Lz_0: 0.5,
        total_substeps: 1000,
        closure_min: 0.0625,
        closure_step: 77,
        ..SimStateFTLE::default()
    };
    s.packed_a = set_state(s.packed_a, STATE_COLLISION);
    s.packed_a = set_detail(s.packed_a, 1);
    s.packed_a = set_saturated(s.packed_a, true);
    s.packed_a = set_dmin_pair(s.packed_a, 2);
    s.packed_a = set_last_symbol(s.packed_a, 3);
    s.packed_a = insert(s.packed_a, u32::from(f32_to_f16_bits(1.5)), 16, 16);
    s.packed_b = pack_packed_b(0.25, 0.375);
    s.times = pack_times(400, 120);
    let word = fgw_pack([7, 8, 9, 10], 12);
    let ic = ICDescriptor {
        m0: 1.0,
        m1: 2.0,
        m2: 3.0,
        rho_angle: 0.5,
        r_min_pair_0: 0.25,
        ..ICDescriptor::default()
    };
    (s, word, ic)
}

fn params() -> ReadParams {
    ReadParams {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
    }
}

/// `d`, decoded from [`sample`], holds every field the sample stores, and the drifts the read side derives.
fn check_decoded(d: &Decoded) {
    let (s, word, ic) = sample();
    assert_eq!(
        (d.state, d.detail, d.saturated, d.dmin_pair, d.last_symbol),
        (STATE_COLLISION, 1, true, 2, 3),
        "the descriptor's fields"
    );
    assert_eq!(
        (d.d_min, d.dE_max, d.dLz_max),
        (1.5, 0.25, 0.375),
        "the f16 latches"
    );
    assert_eq!((d.t_end_step, d.t_dmin_step), (400, 120), "the times");
    assert_eq!(
        (d.total_substeps, d.closure_step),
        (1000, 77),
        "the counters"
    );
    assert_eq!(d.r, [1.0, 2.0, 3.0, 4.0, 5.0, 6.5], "`r` flattened");
    assert_eq!(
        d.p_sh,
        [0.5, 0.25, -0.5, 0.75, 0.0, -1.25],
        "the shadow's `p_sh`"
    );
    assert_eq!((d.length, d.payload), (12, 10), "the word's `.w` fields");
    assert_eq!(
        (d.m2, d.rho_angle, d.r_min_pair_0),
        (3.0, 0.5, 0.25),
        "the descriptor"
    );
    assert_eq!((d.S, d.theta, d.E_0), (0.75, -7.0, -2.5), "the scalars");
    let m = [ic.m0, ic.m1, ic.m2];
    assert_eq!(
        d.energy_drift.to_bits(),
        energy_drift(s.r, s.p, m, s.E_0).to_bits(),
        "`energy_drift`"
    );
    assert_eq!(fgw_length_raw(word), d.length, "the word's length");
}

#[test]
fn export_decodes_every_field() {
    let (s, word, ic) = sample();
    check_decoded(&decode(&s, word, &ic, &params()));
}

negative_control!(
    export_decodes_every_field,
    "a sample whose word holds another length decodes another length",
    expected = "the word's `.w` fields",
    {
        let (s, _, ic) = sample();
        check_decoded(&decode(&s, fgw_pack([7, 8, 9, 10], 13), &ic, &params()));
    }
);
