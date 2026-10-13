//! QA tests for TASK-M1-14, engine side, written from the requirements it closes, not from the implementation:
//!
//! - REQ-TOOL-018 (debug tooling plan, "Principle", "Synthetic-first"): the named synthetic fixtures (a set `state`, a
//!   known escaper, a ramp in `energy_drift`, a bitwise-adversarial descriptor, a forced-failure sample) hold what they
//!   name, and the generated accessors return the stored values, checked against payload §2's bit layout decoded here
//!   by hand (`qa_m1_14_accessors_*`, `qa_m1_14_fixture_*`).
//! - REQ-PAY-024 with its verify conformed to R-271: no stored value of the synthetic and forced-failure samples is
//!   NaN, and the only non-finite stored value is `d_min`'s unset f16 +inf, `0x7C00` in bits 16–31 of `packed_a`, held
//!   only by a failed sample (`sim_failed` or `decode_failed`, payload §2) or one before its first step
//!   (`t_end_step == 0`, payload §2 `times`: "initial: 0"). The scanner below is this file's own, written from payload
//!   §1 and §2 and R-271; each misplacement (wrong half, wrong word, wrong sign, wrong sample, f32 +inf) is a control
//!   (`qa_stored_values_no_nan_*`). The grep half of the verify: generated Rust and WGSL decide no validity by
//!   `isnan` (`qa_m1_14_no_isnan_in_generated_validity_logic`).
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use engine::fixtures;
use engine::synthetic::Synthetic;
use kernel::payload::{
    canonical_nan, pb_dE_max, pb_dLz_max, sd_detail, sd_dmin_pair, sd_last_symbol, sd_saturated,
    sd_state, sim_state_from_ftle, tm_t_dmin_step, tm_t_end_step, ReadParams, SimStateFTLE,
};
use render::raster::Grid;
use validation::negative_control;

/// The five fixtures the task's Goal names, by the names `engine::fixtures` gives them.
const NAMED: [&str; 5] = [
    "set_state",
    "known_escaper",
    "energy_drift_ramp",
    "adversarial_descriptor",
    "forced_failure",
];

/// Payload §2's state codes.
const ESCAPE: u32 = 0;
const RUNNING: u32 = 3;
const SIM_FAILED: u32 = 4;
const DECODE_FAILED: u32 = 5;

/// f16 +inf, `d_min`'s unset bits (R-271).
const F16_POS_INF: u16 = 0x7c00;

/// A row of eight fresh samples, one quad each.
fn row() -> Synthetic {
    Synthetic::flat(Grid::new([8, 1], 1, 0, 8).expect("the row"), 0)
}

fn filled(name: &str) -> Synthetic {
    let mut set = row();
    fixtures::fill(name, &mut set).unwrap_or_else(|e| panic!("{name}: {e}"));
    set
}

/// Bits `lo .. lo + width` of `w`.
fn bits(w: u32, lo: u32, width: u32) -> u32 {
    (w >> lo) & ((1u32 << width) - 1)
}

/// The value of the f16 bit pattern `h`, decoded here from IEEE 754 binary16 (no shared helper).
fn f16_value(h: u16) -> f64 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((h >> 10) & 0x1f);
    let man = f64::from(h & 0x3ff);
    match exp {
        0 => sign * man * 2f64.powi(-24),
        31 if man == 0.0 => sign * f64::INFINITY,
        31 => f64::NAN,
        _ => sign * (1.0 + man / 1024.0) * 2f64.powi(exp - 15),
    }
}

/// The fixture `name`'s sample `i`, decoded by payload §2's layout: `state` 0–2, `detail` 3–4, `saturated` 5,
/// `dmin_pair` 6–7, `last_symbol` 8–9.
fn decoded(w: u32) -> (u32, u32, bool, u32, u32) {
    (
        bits(w, 0, 3),
        bits(w, 3, 2),
        bits(w, 5, 1) == 1,
        bits(w, 6, 2),
        bits(w, 8, 2),
    )
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-TOOL-018: the accessors return the stored values.

/// Checks that each generated accessor of each named fixture's samples returns the field `layout` decodes from the
/// stored words, and the f16 and u16 halves are payload §2's.
fn check_accessors(layout: fn(u32) -> (u32, u32, bool, u32, u32)) {
    for name in NAMED {
        let set = filled(name);
        for i in 0..8 {
            let s = set.simstate(i);
            let w = s.packed_a;
            let got = (
                sd_state(w),
                sd_detail(w),
                sd_saturated(w),
                sd_dmin_pair(w),
                sd_last_symbol(w),
            );
            assert_eq!(
                got,
                layout(w),
                "`{name}` sample {i}: an accessor disagrees with the stored bits"
            );
            assert_eq!(
                tm_t_end_step(s.times),
                s.times & 0xffff,
                "`{name}` {i}: t_end_step"
            );
            assert_eq!(
                tm_t_dmin_step(s.times),
                s.times >> 16,
                "`{name}` {i}: t_dmin_step"
            );
            for (got, h, what) in [
                (pb_dE_max(s.packed_b), s.packed_b as u16, "dE_max"),
                (pb_dLz_max(s.packed_b), (s.packed_b >> 16) as u16, "dLz_max"),
            ] {
                let want = f16_value(h);
                assert!(
                    f64::from(got) == want || (got.is_nan() && want.is_nan()),
                    "`{name}` {i}: {what} reads {got}, its bits {h:#06x} hold {want}"
                );
            }
        }
    }
}

#[test]
fn qa_m1_14_accessors_read_the_payload_layout() {
    check_accessors(decoded);
}

negative_control!(
    qa_m1_14_accessors_read_the_payload_layout,
    "a layout with `detail` and `saturated` read one bit off disagrees with the accessors",
    expected = "an accessor disagrees with the stored bits",
    check_accessors(|w| {
        let (s, _, _, p, l) = decoded(w);
        (s, bits(w, 4, 2), bits(w, 6, 1) == 1, p, l)
    })
);

/// Every sample of `set` read back through the kernel's read side: `(energy_drift, H(r, p) − E_0)` computed here in
/// f64 from the stored state, `G = 1` (integrator dd §3.5), and the f32 tolerance of that difference.
fn drifts(set: &mut Synthetic) -> Vec<(f32, f64, f64)> {
    let params = ReadParams {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
    };
    (0..8)
        .map(|i| {
            let ic = *set.ic(i);
            let m = [ic.m0, ic.m1, ic.m2].map(f64::from);
            let s = *set.simstate(i);
            let read = sim_state_from_ftle(
                &s,
                set.word(i),
                true,
                canonical_nan(),
                false,
                [ic.m0, ic.m1, ic.m2],
                &params,
            );
            let r = s.r.map(|v| v.map(f64::from));
            let p = s.p.map(|v| v.map(f64::from));
            let kinetic: f64 = (0..3)
                .map(|k| (p[k][0].powi(2) + p[k][1].powi(2)) / (2.0 * m[k]))
                .sum();
            let potential: f64 = [(0, 1), (0, 2), (1, 2)]
                .iter()
                .map(|&(a, b)| {
                    let d = ((r[a][0] - r[b][0]).powi(2) + (r[a][1] - r[b][1]).powi(2)).sqrt();
                    m[a] * m[b] / d
                })
                .sum();
            let h = kinetic - potential;
            // f32 evaluation of H and the subtraction: a few ulps of the largest operand.
            let tol =
                16.0 * f64::from(f32::EPSILON) * (kinetic + potential + f64::from(s.E_0).abs());
            (read.energy_drift, h - f64::from(s.E_0), tol)
        })
        .collect()
}

/// Checks that the energy-drift fixture's samples read as a ramp: each sample's `energy_drift` is `H(r, p) − E_0` of its
/// stored state, the drift rises strictly across the row, in equal steps.
fn check_ramp(set: &mut Synthetic) {
    let d = drifts(set);
    for (i, &(got, want, tol)) in d.iter().enumerate() {
        assert!(
            (f64::from(got) - want).abs() <= tol,
            "sample {i}: energy_drift reads {got}, the stored state gives {want}"
        );
    }
    let steps: Vec<f64> = d
        .windows(2)
        .map(|w| f64::from(w[1].0) - f64::from(w[0].0))
        .collect();
    let mean = steps.iter().sum::<f64>() / steps.len() as f64;
    assert!(mean > 0.0, "the drift does not rise: steps {steps:?}");
    for (k, s) in steps.iter().enumerate() {
        assert!(
            (s - mean).abs() <= d[k].2 + d[k + 1].2,
            "the drift is not a ramp: step {k} is {s}, the mean step {mean}"
        );
    }
}

#[test]
fn qa_m1_14_fixture_energy_drift_is_a_ramp() {
    check_ramp(&mut filled("energy_drift_ramp"));
}

negative_control!(
    qa_m1_14_fixture_energy_drift_is_a_ramp,
    "one sample's E_0 nudged off the ramp",
    expected = "the drift is not a ramp",
    {
        let mut set = filled("energy_drift_ramp");
        let e0 = set.simstate(4).E_0;
        set.sample(4).E_0(e0 - 0.1);
        check_ramp(&mut set);
    }
);

/// Checks the set-state fixture: its samples hold every `state` code, the six of payload §2 and the reserved 6 and 7.
fn check_set_state(set: &Synthetic) {
    let mut codes: Vec<u32> = (0..8)
        .map(|i| bits(set.simstate(i).packed_a, 0, 3))
        .collect();
    codes.sort_unstable();
    assert_eq!(
        codes,
        (0..8).collect::<Vec<_>>(),
        "the set states are not every code"
    );
}

#[test]
fn qa_m1_14_fixture_set_state_holds_every_code() {
    check_set_state(&filled("set_state"));
}

negative_control!(
    qa_m1_14_fixture_set_state_holds_every_code,
    "a set state written twice",
    expected = "the set states are not every code",
    {
        let mut set = filled("set_state");
        set.sample(7).state(0);
        check_set_state(&set);
    }
);

/// Checks the known escaper: an escape of each body (payload §2: `detail` the body id, `3` the triple ejection), each
/// at a completed step (payload §2: "Escape has no step-0 case"), and no escape in any fixture at step 0.
fn check_escaper(sets: &[(&str, Synthetic)]) {
    for (name, set) in sets {
        for i in 0..8 {
            let s = set.simstate(i);
            if bits(s.packed_a, 0, 3) == ESCAPE {
                assert!(
                    tm_t_end_step(s.times) > 0,
                    "`{name}` sample {i} escapes at step 0, which payload §2 rules out"
                );
            }
        }
    }
    let set = &sets
        .iter()
        .find(|(n, _)| *n == "known_escaper")
        .expect("the escaper")
        .1;
    for body in 0..3 {
        assert!(
            (0..8).any(|i| {
                let w = set.simstate(i).packed_a;
                bits(w, 0, 3) == ESCAPE && bits(w, 3, 2) == body
            }),
            "the known escaper has no escape of body {body}"
        );
    }
}

#[test]
fn qa_m1_14_fixture_known_escaper_escapes_each_body() {
    let sets: Vec<(&str, Synthetic)> = NAMED.iter().map(|&n| (n, filled(n))).collect();
    check_escaper(&sets);
}

negative_control!(
    qa_m1_14_fixture_known_escaper_escapes_each_body,
    "an escape recorded at step 0",
    expected = "escapes at step 0",
    {
        let mut set = filled("known_escaper");
        set.sample(0).times(0, 0);
        check_escaper(&[("known_escaper", set)]);
    }
);

/// Checks the bitwise-adversarial descriptor: the descriptor all clear and all set (bits 0–9) are both present, each
/// field takes its least and greatest value, and the reserved bits 10–15 hold zero (payload §2).
fn check_adversarial(set: &Synthetic) {
    let lows: Vec<u32> = (0..8).map(|i| set.simstate(i).packed_a & 0xffff).collect();
    assert!(
        lows.contains(&0) && lows.contains(&0x3ff),
        "no all-clear and all-set descriptor: {lows:x?}"
    );
    for (field, lo, width) in [
        ("state", 0, 3),
        ("detail", 3, 2),
        ("saturated", 5, 1),
        ("dmin_pair", 6, 2),
        ("last_symbol", 8, 2),
    ] {
        let vals: Vec<u32> = lows.iter().map(|&w| bits(w, lo, width)).collect();
        let max = (1u32 << width) - 1;
        assert!(
            vals.contains(&0) && vals.contains(&max),
            "`{field}` does not reach 0 and {max}: {vals:?}"
        );
    }
    for (i, &w) in lows.iter().enumerate() {
        assert_eq!(
            bits(w, 10, 6),
            0,
            "sample {i}: a reserved bit is set in {w:#06x}"
        );
    }
}

#[test]
fn qa_m1_14_fixture_adversarial_descriptor_spans_each_field() {
    check_adversarial(&filled("adversarial_descriptor"));
}

negative_control!(
    qa_m1_14_fixture_adversarial_descriptor_spans_each_field,
    "a contaminated reserved bit (bit 12), which every sd_* accessor masks",
    expected = "a reserved bit is set",
    {
        let mut set = filled("adversarial_descriptor");
        let w = set.simstate(2).packed_a;
        set.sample(2).packed_a(w | 1 << 12);
        check_adversarial(&set);
    }
);

/// Checks the forced-failure fixture: every sample is failed, both failed states occur, and each holds payload §1's
/// failed-state values: `dE_max` and `dLz_max` 0.0, `d_min` the unset bits `0x7C00` (R-271).
fn check_forced_failure(set: &Synthetic) {
    let mut states = std::collections::BTreeSet::new();
    for i in 0..8 {
        let s = set.simstate(i);
        let state = bits(s.packed_a, 0, 3);
        assert!(
            state == SIM_FAILED || state == DECODE_FAILED,
            "sample {i} is not failed: {state}"
        );
        states.insert(state);
        assert_eq!(
            f16_value(s.packed_b as u16),
            0.0,
            "sample {i}: dE_max is not 0.0"
        );
        assert_eq!(
            f16_value((s.packed_b >> 16) as u16),
            0.0,
            "sample {i}: dLz_max is not 0.0"
        );
        assert_eq!(
            (s.packed_a >> 16) as u16,
            F16_POS_INF,
            "sample {i}: d_min is not unset"
        );
    }
    assert_eq!(states.len(), 2, "not both failed states: {states:?}");
}

#[test]
fn qa_m1_14_fixture_forced_failure_holds_the_failed_state() {
    check_forced_failure(&filled("forced_failure"));
}

negative_control!(
    qa_m1_14_fixture_forced_failure_holds_the_failed_state,
    "a failed sample keeping the drift that failed it in dE_max",
    expected = "dE_max is not 0.0",
    {
        let mut set = filled("forced_failure");
        set.sample(5).drift_max(3.5, 0.0);
        check_forced_failure(&set);
    }
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-PAY-024 (R-79, R-271): no stored NaN; the one non-finite value, and where it may sit.

/// The members of `SimStateFTLE` and `ICDescriptor` this scanner has a rule for, in the ledger's order. A new member
/// makes `check_members` fail, so it cannot pass unscanned.
const SIMSTATE_MEMBERS: [&str; 17] = [
    "r",
    "p",
    "r_sh",
    "p_sh",
    "S",
    "theta",
    "mean_y",
    "C_ty",
    "E_0",
    "Lz_0",
    "packed_a",
    "packed_b",
    "times",
    "total_substeps",
    "closure_min",
    "closure_step",
    "_reserved",
];
const IC_MEMBERS: [&str; 13] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
    "_pad",
];

fn check_members() {
    for s in ledger::payload::structs() {
        let names: Vec<&str> = s.members.iter().map(|m| m.name).collect();
        match s.name {
            "SimStateFTLE" => assert_eq!(
                names, SIMSTATE_MEMBERS,
                "SimStateFTLE has members this scan has no rule for"
            ),
            "ICDescriptor" => assert_eq!(
                names, IC_MEMBERS,
                "ICDescriptor has members this scan has no rule for"
            ),
            _ => {}
        }
    }
}

/// The f32 members of a stored `SimStateFTLE`, by name.
fn simstate_floats(s: &SimStateFTLE) -> Vec<(String, f32)> {
    let mut out = Vec::new();
    for (name, v) in [("r", s.r), ("p", s.p), ("r_sh", s.r_sh), ("p_sh", s.p_sh)] {
        for (k, c) in v.iter().flatten().enumerate() {
            out.push((format!("{name}[{k}]"), *c));
        }
    }
    for (name, v) in [
        ("S", s.S),
        ("theta", s.theta),
        ("mean_y", s.mean_y),
        ("C_ty", s.C_ty),
        ("E_0", s.E_0),
        ("Lz_0", s.Lz_0),
        ("closure_min", s.closure_min),
    ] {
        out.push((name.to_owned(), v));
    }
    out
}

/// Checks REQ-PAY-024 on the first `n` samples of `set`:
/// - every stored f32, of `SimStateFTLE` and `ICDescriptor`, is finite (no NaN, and no ±inf: R-271's one exception is
///   an f16);
/// - `packed_b`'s two f16 halves, `dE_max` and `dLz_max`, are finite;
/// - `packed_a`'s bits 0–15 are the descriptor, whose reserved bits 10–15 are zero, so no f16 lands there;
/// - `packed_a`'s bits 16–31, `d_min`, are never NaN or −inf, and are +inf only as exactly `0x7C00` on a `sim_failed` or
///   `decode_failed` sample or one whose `t_end_step` is 0.
fn check_stored(set: &mut Synthetic, n: u32) {
    check_members();
    for i in 0..n {
        let s = *set.simstate(i);
        for (name, v) in simstate_floats(&s) {
            assert!(!v.is_nan(), "sample {i}: SimState `{name}` is NaN");
            assert!(
                v.is_finite(),
                "sample {i}: SimState `{name}` is {v}, not finite"
            );
        }
        let ic = *set.ic(i);
        for (k, v) in [
            ic.m0,
            ic.m1,
            ic.m2,
            ic.q_mass,
            ic.rho_mag,
            ic.lambda_mag,
            ic.rho_ratio,
            ic.rho_angle,
            ic.K_0,
            ic.V_0,
            ic.virial_ratio,
            ic.r_min_pair_0,
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                !v.is_nan(),
                "sample {i}: ICDescriptor `{}` is NaN",
                IC_MEMBERS[k]
            );
            assert!(
                v.is_finite(),
                "sample {i}: ICDescriptor `{}` is {v}, not finite",
                IC_MEMBERS[k]
            );
        }
        for (h, what) in [
            (s.packed_b as u16, "dE_max"),
            ((s.packed_b >> 16) as u16, "dLz_max"),
        ] {
            let v = f16_value(h);
            assert!(!v.is_nan(), "sample {i}: {what} holds the f16 NaN {h:#06x}");
            assert!(
                v.is_finite(),
                "sample {i}: {what} holds the non-finite {h:#06x}; only d_min may be +inf"
            );
        }
        let low = s.packed_a & 0xffff;
        assert_eq!(
            bits(low, 10, 6),
            0,
            "sample {i}: packed_a bits 0–15 hold {low:#06x}, reserved bits set"
        );
        let h = (s.packed_a >> 16) as u16;
        let v = f16_value(h);
        assert!(!v.is_nan(), "sample {i}: d_min holds the f16 NaN {h:#06x}");
        if !v.is_finite() {
            assert_eq!(
                h, F16_POS_INF,
                "sample {i}: d_min holds {h:#06x}, not +inf 0x7C00"
            );
            let state = bits(s.packed_a, 0, 3);
            let failed = state == SIM_FAILED || state == DECODE_FAILED;
            assert!(
                failed || tm_t_end_step(s.times) == 0,
                "sample {i}: d_min is unset on a sample neither failed nor before its first step (state {state}, \
                 step {})",
                tm_t_end_step(s.times)
            );
        }
    }
}

#[test]
fn qa_stored_values_no_nan_every_fixture() {
    for name in NAMED {
        check_stored(&mut filled(name), 8);
    }
}

negative_control!(
    qa_stored_values_no_nan_every_fixture,
    "a forced failure storing the NaN energy that failed it in E_0",
    expected = "SimState `E_0` is NaN",
    {
        let mut set = filled("forced_failure");
        set.sample(3).E_0(f32::NAN);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_ic() {
    check_stored(&mut filled("set_state"), 8);
}

negative_control!(
    qa_stored_values_no_nan_ic,
    "a NaN in the ICDescriptor",
    expected = "ICDescriptor `K_0` is NaN",
    {
        let mut set = filled("set_state");
        set.ic(1).K_0 = f32::NAN;
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_f32_inf() {
    check_stored(&mut filled("known_escaper"), 8);
}

negative_control!(
    qa_stored_values_no_nan_f32_inf,
    "an f32 +inf (closure_min): R-271's one +inf is d_min's f16",
    expected = "SimState `closure_min` is inf, not finite",
    {
        let mut set = filled("known_escaper");
        set.sample(2).closure_min(f32::INFINITY);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_dmin_f16_nan() {
    check_stored(&mut filled("adversarial_descriptor"), 8);
}

negative_control!(
    qa_stored_values_no_nan_dmin_f16_nan,
    "an f16 NaN, 0x7E00, in d_min's half",
    expected = "d_min holds the f16 NaN",
    {
        let mut set = filled("adversarial_descriptor");
        let w = set.simstate(6).packed_a;
        set.sample(6).packed_a((w & 0xffff) | 0x7e00 << 16);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_inf_not_in_de_max() {
    check_stored(&mut filled("forced_failure"), 8);
}

negative_control!(
    qa_stored_values_no_nan_inf_not_in_de_max,
    "f16 +inf, 0x7C00, in dE_max's half (bits 0–15 of packed_b)",
    expected = "dE_max holds the non-finite 0x7c00",
    {
        let mut set = filled("forced_failure");
        set.sample(0).packed_b(0x7c00);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_inf_not_in_dlz_max() {
    check_stored(&mut filled("forced_failure"), 8);
}

negative_control!(
    qa_stored_values_no_nan_inf_not_in_dlz_max,
    "f16 +inf, 0x7C00, in dLz_max's half (bits 16–31 of packed_b)",
    expected = "dLz_max holds the non-finite 0x7c00",
    {
        let mut set = filled("forced_failure");
        set.sample(1).packed_b(0x7c00 << 16);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_inf_not_in_descriptor_half() {
    check_stored(&mut filled("forced_failure"), 8);
}

negative_control!(
    qa_stored_values_no_nan_inf_not_in_descriptor_half,
    "0x7C00 in bits 0–15 of packed_a, the descriptor's half, not d_min's",
    expected = "reserved bits set",
    {
        let mut set = filled("forced_failure");
        let w = set.simstate(4).packed_a;
        set.sample(4)
            .packed_a((w & 0xffff_0000) | 0x7c00 | bits(w, 0, 10));
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_negative_inf() {
    check_stored(&mut filled("forced_failure"), 8);
}

negative_control!(
    qa_stored_values_no_nan_negative_inf,
    "f16 −inf, 0xFC00, in a failed sample's d_min half",
    expected = "not +inf 0x7C00",
    {
        let mut set = filled("forced_failure");
        let w = set.simstate(2).packed_a;
        set.sample(2).packed_a((w & 0xffff) | 0xfc00 << 16);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_unset_before_first_step() {
    // A fresh row: every sample running, before its first step, `d_min` unset. R-271 allows it.
    let mut set = row();
    for i in 0..8 {
        assert_eq!(
            (set.simstate(i).packed_a >> 16) as u16,
            F16_POS_INF,
            "a fresh sample's d_min is unset"
        );
        assert_eq!(bits(set.simstate(i).packed_a, 0, 3), RUNNING);
    }
    check_stored(&mut set, 8);
}

negative_control!(
    qa_stored_values_no_nan_unset_before_first_step,
    "a running sample one step in, d_min still unset",
    expected = "neither failed nor before its first step",
    {
        let mut set = row();
        set.sample(5).times(1, 0);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_unset_on_failed_after_steps() {
    // A sim_failed sample many steps in keeps d_min unset: allowed, it is failed (R-271).
    let mut set = filled("forced_failure");
    let s = *set.simstate(1);
    assert!(tm_t_end_step(s.times) > 0 && bits(s.packed_a, 0, 3) == SIM_FAILED);
    check_stored(&mut set, 8);
}

negative_control!(
    qa_stored_values_no_nan_unset_on_failed_after_steps,
    "a reserved state code, 6, stepped, with d_min unset: untrusted, but not one of payload §2's failed states",
    expected = "neither failed nor before its first step",
    {
        let mut set = filled("forced_failure");
        set.sample(1).state(6);
        check_stored(&mut set, 8);
    }
);

#[test]
fn qa_stored_values_no_nan_unset_on_valid_outcome() {
    check_stored(&mut filled("known_escaper"), 8);
}

negative_control!(
    qa_stored_values_no_nan_unset_on_valid_outcome,
    "an escape at step 120 holding the unset +inf",
    expected = "neither failed nor before its first step",
    {
        let mut set = filled("known_escaper");
        set.sample(0).d_min(f32::INFINITY);
        check_stored(&mut set, 8);
    }
);

/// Checks that `d_min` set to `v` through the generated packer stores the bits `want` in bits 16–31 of `packed_a`.
fn check_unset_bits(v: f32, want: u16) {
    let mut set = row();
    set.sample(0).state(SIM_FAILED).d_min(v);
    let got = (set.simstate(0).packed_a >> 16) as u16;
    assert_eq!(got, want, "d_min({v}) stores {got:#06x}, not {want:#06x}");
}

#[test]
fn qa_m1_14_unset_is_written_as_its_bits() {
    check_unset_bits(f32::INFINITY, F16_POS_INF);
}

negative_control!(
    qa_m1_14_unset_is_written_as_its_bits,
    "the largest finite f16 is not the unset value",
    expected = "not 0x7c00",
    check_unset_bits(65504.0, F16_POS_INF)
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-PAY-024's grep: no `isnan` in validity logic of the generated Rust and WGSL.

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every generated source file: a `.rs` or `.wgsl` whose path, under `crates/`, has a component named `generated`
/// or `generated.rs`.
fn generated_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, inside: bool, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            let here = inside || name == "generated" || name == "generated.rs";
            if p.is_dir() {
                walk(&p, here, out);
            } else if here && (name.ends_with(".rs") || name.ends_with(".wgsl")) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join("crates"), false, &mut out);
    out.sort();
    out
}

/// Checks `text`, the source `path`: WGSL holds no `isnan`/`isNan` at all; in Rust every `is_nan` sits in a function
/// whose name begins `set_`, a write-side storage guard (R-79: refuse or replace a NaN before storing it), never in a
/// reader, a `has_*` const or a validity predicate.
fn check_no_isnan(path: &str, text: &str) {
    let wgsl = path.ends_with(".wgsl");
    let mut current_fn = String::new();
    for (k, line) in text.lines().enumerate() {
        if let Some(at) = line.find("fn ") {
            let rest = &line[at + 3..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                current_fn = name;
            }
        }
        let code = line.split("//").next().unwrap_or("");
        if wgsl {
            assert!(
                !code.contains("isnan") && !code.contains("isNan"),
                "{path}:{}: isnan in generated WGSL",
                k + 1
            );
        } else if code.contains("is_nan") {
            assert!(
                current_fn.starts_with("set_"),
                "{path}:{}: is_nan in `{current_fn}`, which is no write-side guard",
                k + 1
            );
        }
    }
}

#[test]
fn qa_m1_14_no_isnan_in_generated_validity_logic() {
    let files = generated_files();
    assert!(
        files
            .iter()
            .any(|p| p.ends_with("kernel/src/payload/generated.rs"))
            && files
                .iter()
                .any(|p| p.ends_with("render/frag/generated/payload_unpack.wgsl")),
        "the generated files were not found: {files:?}"
    );
    for p in files {
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        check_no_isnan(&p.display().to_string(), &text);
    }
}

negative_control!(
    qa_m1_14_no_isnan_in_generated_validity_logic,
    "a generated validity predicate deciding by is_nan",
    expected = "which is no write-side guard",
    check_no_isnan(
        "generated.rs",
        "pub fn ftle_valid(v: f32) -> bool {\n    !v.is_nan()\n}\n"
    )
);
