//! QA tests for TASK-M1-08 on the Rust read side (`kernel::payload`'s generated `read_side.rs`), written from RQ-228
//! and lowering Part 3a, not from the implementation: the read-side `SimState` carries the Benettin shadow `r_sh` and
//! `p_sh` at every tier; the FTLE tier reads the stored shadow, every component bit for bit, over any stored bits; the
//! base tier, which stores no shadow, reads every component as the canonical quiet NaN `0x7FC00000` (R-72), whatever
//! else it stores. Each test has a registered negative control (R-176).

use kernel::payload::*;
use proptest::prelude::*;
use validation::{negative_control, prop};

/// Lowering Part 3a's canonical quiet NaN.
const QNAN: u32 = 0x7FC0_0000;

/// A stored `SimStateFTLE` with the shadow's twelve components `sh` (bits) and the rest of the state `rest` (bits).
fn stored(sh: &[u32; 12], rest: &[u32; 12]) -> SimStateFTLE {
    let v = |w: &[u32]| {
        [
            [f32::from_bits(w[0]), f32::from_bits(w[1])],
            [f32::from_bits(w[2]), f32::from_bits(w[3])],
            [f32::from_bits(w[4]), f32::from_bits(w[5])],
        ]
    };
    SimStateFTLE {
        r: v(&rest[0..6]),
        p: v(&rest[6..12]),
        r_sh: v(&sh[0..6]),
        p_sh: v(&sh[6..12]),
        S: 0.5,
        theta: 0.25,
        mean_y: 0.125,
        C_ty: 1.0,
        E_0: -1.0,
        Lz_0: 0.5,
        packed_a: 1,
        packed_b: 0,
        times: pack_times(40, 3),
        total_substeps: 77,
        closure_min: 0.75,
        closure_step: 5,
        _reserved: 0,
        _tail: [],
    }
}

fn base_of(s: &SimStateFTLE) -> SimStateBase {
    SimStateBase {
        r: s.r,
        p: s.p,
        S: s.S,
        theta: s.theta,
        mean_y: s.mean_y,
        C_ty: s.C_ty,
        E_0: s.E_0,
        Lz_0: s.Lz_0,
        packed_a: s.packed_a,
        packed_b: s.packed_b,
        times: s.times,
        total_substeps: s.total_substeps,
        closure_min: s.closure_min,
        closure_step: s.closure_step,
        _reserved: s._reserved,
        _tail: [],
    }
}

const PARAMS: ReadParams = ReadParams {
    dt_macro: 0.125,
    delta_0: 0.0625,
    n_renorm: 16,
    horizon_steps: 1000,
};

/// The two tiers' reads of `s`.
type Reads = fn(&SimStateFTLE) -> (SimState, SimState);

fn generated(s: &SimStateFTLE) -> (SimState, SimState) {
    let word = [1, 2, 3, 4];
    let masses = [1.0, 1.0, 2.0];
    (
        sim_state_from_ftle(s, word, true, 0.5, true, masses, &PARAMS),
        sim_state_from_base(&base_of(s), word, true, 0.5, true, masses, &PARAMS),
    )
}

fn flat(v: [[f32; 2]; 3]) -> [u32; 6] {
    [v[0][0], v[0][1], v[1][0], v[1][1], v[2][0], v[2][1]].map(f32::to_bits)
}

/// RQ-228: at the FTLE tier the shadow reads as stored, bit for bit; at the base tier every component is the canonical
/// quiet NaN.
fn check_shadow(reads: Reads, sh: [u32; 12], rest: [u32; 12]) -> Result<(), TestCaseError> {
    let s = stored(&sh, &rest);
    let (ftle, base) = reads(&s);
    let got: Vec<u32> = flat(ftle.r_sh).into_iter().chain(flat(ftle.p_sh)).collect();
    prop_assert_eq!(
        &got[..],
        &sh[..],
        "at the FTLE tier the shadow does not read the stored shadow (RQ-228)"
    );
    let got: Vec<u32> = flat(base.r_sh).into_iter().chain(flat(base.p_sh)).collect();
    prop_assert!(
        got.iter().all(|&b| b == QNAN),
        "at the base tier the shadow is {:x?}, not the canonical quiet NaN in every component (RQ-228)",
        got
    );
    Ok(())
}

#[test]
fn qa_rq228_rust_shadow_reads_stored_or_nan() {
    prop::run(&(any::<[u32; 12]>(), any::<[u32; 12]>()), |(sh, rest)| {
        check_shadow(generated, sh, rest)
    });
}

negative_control!(
    qa_rq228_rust_shadow_reads_stored_or_nan,
    "a base tier that reads the state `r` as its shadow resurrects nothing it stores but is not NaN",
    expected = "at the base tier the shadow is",
    prop::run(&(any::<[u32; 12]>(), any::<[u32; 12]>()), |(sh, rest)| {
        check_shadow(
            |s| {
                let (f, mut b) = generated(s);
                b.r_sh = s.r;
                (f, b)
            },
            sh,
            rest,
        )
    })
);

/// The FTLE tier reading the state's `p` for `p_sh` is caught.
#[cfg(feature = "controls")]
mod qa_rq228_rust_shadow_reads_stored_or_nan_ftle {
    use super::*;

    negative_control!(
        qa_rq228_rust_shadow_reads_stored_or_nan,
        "an FTLE-tier `p_sh` read from `p` is not the stored shadow",
        expected = "at the FTLE tier the shadow does not read the stored shadow",
        prop::run(&(any::<[u32; 12]>(), any::<[u32; 12]>()), |(sh, rest)| {
            check_shadow(
                |s| {
                    let (mut f, b) = generated(s);
                    f.p_sh = s.p;
                    (f, b)
                },
                sh,
                rest,
            )
        })
    );
}
