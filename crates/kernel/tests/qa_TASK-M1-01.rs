//! QA tests for TASK-M1-01, the Rust target of the read side (`kernel::payload`'s generated `read_side.rs`, for the
//! kernel and the host), written from the requirements and their sources, not from the implementation: hand-filled
//! stored structs read through `sim_state_from_ftle` / `sim_state_from_base` and compared with references computed
//! here in f64 from the corpus's formulas.
//! - REQ-PAY-021: `ftle = (S + ln(δ/δ₀)) / (n·dt_macro)`, the partial renorm interval finalised (payload §5; integrator
//!   dd §3.4's Benettin, `δ = ‖x' − x‖` over `(r, p)`), never plain `S/t`.
//! - REQ-PAY-022 / REQ-PAY-026: both stored variants unpack into the one `SimState`; with FTLE baked out `ftle` reads
//!   the canonical quiet NaN and `ftle_valid` is false; every other member reads the same from both variants.
//! - REQ-PAY-027 (property): `total_substeps`, an exact u32 sum of `N_sub` stored and read back at a random resume
//!   step, equals the uninterrupted sum (`horizon_steps × N_max ≤ 2³²−1`, payload §2's dispatch guarantee); the proxy is
//!   ⌊log₂ total⌋ for totals ≥ 2 and 0 for 0 and 1, over all of u32.
//! - REQ-PAY-030 (R-245): `diffusion = C_ty / C_tt(n)`, `C_tt(n) = h²·n(n²−1)/12`, `n` the sample's own `t_end_step`;
//!   NaN and invalid for `n < 2`.
//! - REQ-PAY-031 (payload §5; R-246; RQ-203 option 2): `energy_drift = H(r, p) − E_0`, `Lz_drift = L_z(r, p) − Lz_0`
//!   (integrator dd §3.5, `G = 1`; decoder dd §3.6), the masses the read's argument: hand-computed configurations,
//!   exact in f32, and an f64 reference over random configurations.
//! - REQ-PAY-032 (R-254): `ftle_valid`'s truth table over tier × state × n × completed renorms; `ftle` NaN exactly when
//!   it is false.
//! - REQ-RENDER-013 / REQ-RENDER-077: lowering Part 3a's `0x7FC00000` and unbound word `(0, 0, 0, 0xFE000000)`, which
//!   reads as truncated, so no word-derived predicate reads it as valid.
//! - REQ-RENDER-019: `tm_*_fraction(w, 0) == 0`; `(65535, 65535)` → 1.0 exactly.
//!
//! Rust's f32 `+ − × / sqrt` are correctly rounded and `ln` is within 1 ULP, so an f32 result is within
//! `ops · u · Σ|terms|` of the f64 value of the same f32 inputs, `u = 2⁻²⁴`, `ops` the rounded operations counted at each
//! use (first-order forward error). Each check takes the read as a function; its negative control (R-176) runs the
//! same check on a read with one fault.

use kernel::payload::*;
use proptest::prelude::*;
use validation::{negative_control, prop};

/// Lowering Part 3a's values, as the contract writes them (R-72; REQ-RENDER-077).
const QNAN: u32 = 0x7FC0_0000;
const UNBOUND: [u32; 4] = [0, 0, 0, 0xFE00_0000];
const U: f64 = 1.0 / 16_777_216.0;

/// The read's arguments beside the stored state.
#[derive(Clone, Copy, Debug)]
struct Args {
    word: [u32; 4],
    has_word: bool,
    spread: f32,
    has_ensemble: bool,
    masses: [f32; 3],
    params: ReadParams,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            word: [0x1234_5678, 0x9abc_def0, 0x0fed_cba9, (5 << 25) | 0x0123],
            has_word: true,
            spread: 0.375,
            has_ensemble: true,
            masses: [1.0, 1.0, 2.0],
            params: ReadParams {
                dt_macro: 0.125,
                delta_0: 0.0625,
                n_renorm: 16,
                horizon_steps: 1000,
            },
        }
    }
}

/// A read: the FTLE variant's stored state and the arguments; `ftle_variant` false reads it as `SimStateBase` (the
/// same state less the shadow).
type Read = fn(&SimStateFTLE, bool, &Args) -> SimState;

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

/// The generated reads.
fn generated(s: &SimStateFTLE, ftle_variant: bool, a: &Args) -> SimState {
    if ftle_variant {
        sim_state_from_ftle(
            s,
            a.word,
            a.has_word,
            a.spread,
            a.has_ensemble,
            a.masses,
            &a.params,
        )
    } else {
        sim_state_from_base(
            &base_of(s),
            a.word,
            a.has_word,
            a.spread,
            a.has_ensemble,
            a.masses,
            &a.params,
        )
    }
}

const R0: [[f32; 2]; 3] = [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]];
const P0: [[f32; 2]; 3] = [[0.0, 1.0], [0.0, -1.0], [0.0, 0.0]];

/// Hand configuration A (masses 1, 1, 2; see [`drift_cases`]); the shadow off by 0.5 in `x₀`, so `δ = 0.5`,
/// `δ/δ₀ = 8`; n = 30 (renorms every 16: a partial interval of 14).
fn sample() -> SimStateFTLE {
    let mut r_sh = R0;
    r_sh[0][0] += 0.5;
    SimStateFTLE {
        r: R0,
        p: P0,
        r_sh,
        p_sh: P0,
        S: 1.75,
        theta: 0.5,
        mean_y: 0.25,
        C_ty: 3.0,
        E_0: -3.25,
        Lz_0: 1.5,
        packed_a: 1,
        packed_b: 0x3c00_3800,
        times: pack_times(30, 7),
        total_substeps: 1000,
        closure_min: 0.125,
        closure_step: 9,
        _reserved: 0,
        _tail: [],
    }
}

fn n_of(s: &SimStateFTLE) -> u32 {
    s.times & 0xFFFF
}

fn delta(s: &SimStateFTLE) -> f64 {
    let mut sum = 0.0;
    for (a, b) in [(s.r, s.r_sh), (s.p, s.p_sh)] {
        for j in 0..3 {
            for k in 0..2 {
                let d = f64::from(b[j][k]) - f64::from(a[j][k]);
                sum += d * d;
            }
        }
    }
    sum.sqrt()
}

/// payload §5's finalised FTLE and the magnitude its error scales with.
fn ftle_ref(s: &SimStateFTLE, a: &Args) -> (f64, f64) {
    let l = (delta(s) / f64::from(a.params.delta_0)).ln();
    let t = f64::from(n_of(s)) * f64::from(a.params.dt_macro);
    (
        (f64::from(s.S) + l) / t,
        (f64::from(s.S).abs() + l.abs() + 1.0) / t,
    )
}

/// payload §6 / R-254: the tier on, not failed (state ≥ 4), n > 0 and n / n_renorm > 0.
fn ftle_valid_ref(s: &SimStateFTLE, has_ftle: bool, a: &Args) -> bool {
    let n = n_of(s);
    has_ftle && (s.packed_a & 7) < 4 && n > 0 && n / a.params.n_renorm > 0
}

fn near(got: f32, want: f64, mag: f64, ops: u32) -> bool {
    let g = f64::from(got);
    g.is_finite() && (g - want).abs() <= f64::from(ops) * U * mag
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-021

fn check_ftle_finalised(read: Read) {
    let a = Args::default();
    let mut b = sample();
    b.r_sh[2][1] += 0.25;
    b.p_sh[1][0] -= 0.125;
    b.S = -0.5;
    b.times = pack_times(47, 3);
    for s in [sample(), b] {
        let (want, mag) = ftle_ref(&s, &a);
        let got = read(&s, true, &a);
        assert!(
            got.ftle_valid,
            "a partial-interval sample past its first renorm is valid"
        );
        assert!(
            near(got.ftle, want, mag, 40),
            "ftle is {}, not the finalised (S + ln(δ/δ₀))/(n·dt) = {want}",
            got.ftle
        );
    }
}

#[test]
fn qa_pay021_rust_ftle_finalises_the_partial_interval() {
    check_ftle_finalised(generated);
}

negative_control!(
    qa_pay021_rust_ftle_finalises_the_partial_interval,
    "plain S/t, the partial interval dropped, must fail",
    expected = "not the finalised",
    check_ftle_finalised(|s, v, a| {
        let mut out = generated(s, v, a);
        out.ftle = s.S / (n_of(s) as f32 * a.params.dt_macro);
        out
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-032 / REQ-PAY-022 / REQ-PAY-026

fn check_truth_table(read: Read) {
    for n_renorm in [1u32, 16] {
        let a = Args {
            params: ReadParams {
                n_renorm,
                ..Args::default().params
            },
            ..Args::default()
        };
        for variant in [true, false] {
            for state in 0..8 {
                for n in [0u32, 1, 15, 16, 17, 40, 65535] {
                    let mut s = sample();
                    s.packed_a = state;
                    s.times = pack_times(n, 0);
                    let got = read(&s, variant, &a);
                    let valid = ftle_valid_ref(&s, variant, &a);
                    let ctx = format!("tier {variant} state {state} n {n} n_renorm {n_renorm}");
                    assert_eq!(
                        got.ftle_valid, valid,
                        "ftle_valid is not payload §6's for {ctx}"
                    );
                    if valid {
                        let (want, mag) = ftle_ref(&s, &a);
                        assert!(
                            near(got.ftle, want, mag, 40),
                            "a valid ftle is wrong for {ctx}"
                        );
                    } else {
                        assert_eq!(
                            got.ftle.to_bits(),
                            QNAN,
                            "ftle is not the canonical quiet NaN when invalid, {ctx}"
                        );
                    }
                    let want = (state <= 2, state == 3, state >= 4, state != 3);
                    assert_eq!(
                        (
                            got.is_resolved_outcome,
                            got.is_running,
                            got.is_failed,
                            got.is_finished
                        ),
                        want,
                        "the sd_is_* predicates for {ctx}"
                    );
                    assert_eq!(got.state, state, "state for {ctx}");
                }
            }
        }
    }
}

#[test]
fn qa_pay032_rust_ftle_valid_truth_table() {
    check_truth_table(generated);
}

negative_control!(
    qa_pay032_rust_ftle_valid_truth_table,
    "an ftle_valid that ignores the completed renorms must fail",
    expected = "ftle_valid is not payload §6's",
    check_truth_table(|s, v, a| {
        let mut out = generated(s, v, a);
        out.ftle_valid = v && (s.packed_a & 7) < 4 && n_of(s) > 0;
        out
    })
);

/// The two variants of one state read the same in every member but `ftle` and `ftle_valid`; the no-FTLE read's
/// `ftle` is lowering Part 3a's NaN and `ftle_valid` false, whatever the state.
fn check_one_type(read: Read) {
    let a = Args::default();
    for state in [0u32, 1, 3, 4] {
        let mut s = sample();
        s.packed_a = state;
        let f = read(&s, true, &a);
        let b = read(&s, false, &a);
        assert_eq!(
            b.ftle.to_bits(),
            QNAN,
            "with FTLE baked out, ftle is not the canonical quiet NaN"
        );
        assert!(!b.ftle_valid, "with FTLE baked out, ftle_valid is true");
        let strip = |mut x: SimState| {
            x.ftle = 0.0;
            x.ftle_valid = false;
            x
        };
        assert_eq!(
            strip(f),
            strip(b),
            "the two variants read differently outside ftle"
        );
    }
}

#[test]
fn qa_pay026_rust_both_variants_read_one_type() {
    check_one_type(generated);
}

negative_control!(
    qa_pay026_rust_both_variants_read_one_type,
    "a baked-out ftle that reads 0.0 must fail",
    expected = "with FTLE baked out, ftle is not the canonical quiet NaN",
    check_one_type(|s, v, a| {
        let mut out = generated(s, v, a);
        if !v {
            out.ftle = 0.0;
        }
        out
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-RENDER-013 / REQ-RENDER-077

fn check_tier_absent(read: Read) {
    let s = sample();
    for variant in [true, false] {
        let off = Args {
            has_word: false,
            has_ensemble: false,
            ..Args::default()
        };
        let got = read(&s, variant, &off);
        assert_eq!(
            got.ensemble_spread.to_bits(),
            QNAN,
            "ensemble_spread at E = 0 is not the canonical quiet NaN"
        );
        assert_eq!(
            got.word, UNBOUND,
            "an unbound word does not read the sentinel word"
        );
        // the sentinel reads as truncated: no word-derived predicate reads it as valid (payload §3, §6).
        assert!(
            fgw_truncated(got.word),
            "the unbound word does not read as truncated"
        );
        assert!(
            !sd_last_symbol_valid(fgw_length_raw(got.word)),
            "last_symbol is valid on the unbound word"
        );
        let on = Args::default();
        let got = read(&s, variant, &on);
        assert_eq!(got.word, on.word, "a bound word does not read as stored");
        assert_eq!(
            got.ensemble_spread.to_bits(),
            on.spread.to_bits(),
            "E > 0 does not read the spread given"
        );
    }
}

#[test]
fn qa_render013_rust_tier_absent_values() {
    check_tier_absent(generated);
    assert_eq!(CANONICAL_QNAN_BITS, QNAN);
    assert_eq!(canonical_nan().to_bits(), QNAN);
    assert_eq!(FGW_UNBOUND, UNBOUND);
}

negative_control!(
    qa_render013_rust_tier_absent_values,
    "an unbound word read as the empty word must fail",
    expected = "an unbound word does not read the sentinel word",
    check_tier_absent(|s, v, a| {
        let mut out = generated(s, v, a);
        if !a.has_word {
            out.word = [0; 4];
        }
        out
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-027

/// ⌊log₂ t⌋ for t ≥ 2, 0 for 0 and 1, by repeated halving.
fn log2_ref(t: u32) -> u32 {
    let mut k = 0;
    let mut v = t;
    while v > 1 {
        v /= 2;
        k += 1;
    }
    k
}

/// Accumulates `n_sub` into the stored `total_substeps`, stopping at `resume` and reading the stored struct back
/// through the read side to continue, as a cached-state resume does; returns the resumed and uninterrupted totals.
fn resumed(read: Read, n_sub: &[u32], resume: usize) -> (u32, u32) {
    let a = Args::default();
    let mut s = sample();
    s.total_substeps = 0;
    for &k in &n_sub[..resume] {
        s.total_substeps += k;
    }
    let cached = read(&s, true, &a).total_substeps;
    let mut t = cached;
    for &k in &n_sub[resume..] {
        t += k;
    }
    (t, n_sub.iter().sum())
}

fn check_resume(read: Read, n_sub: &[u32], resume: usize) -> Result<(), TestCaseError> {
    let (got, want) = resumed(read, n_sub, resume);
    prop_assert_eq!(
        got,
        want,
        "a resumed total_substeps is not the uninterrupted sum"
    );
    let mut s = sample();
    s.total_substeps = want;
    prop_assert_eq!(
        read(&s, true, &Args::default()).total_substeps_log2,
        log2_ref(want),
        "the proxy of the resumed total"
    );
    Ok(())
}

/// A march: `horizon` macro-steps of `N_sub ≤ n_max` with `horizon × n_max ≤ 2³²−1` (payload §2), and a resume step.
fn march() -> impl Strategy<Value = (Vec<u32>, usize)> {
    (1u32..=200, 1u32..=u32::MAX)
        .prop_flat_map(|(horizon, n_max)| {
            let n_max = n_max.min(u32::MAX / horizon);
            proptest::collection::vec(0..=n_max, horizon as usize)
        })
        .prop_flat_map(|v| {
            let len = v.len();
            (Just(v), 0..=len)
        })
}

#[test]
fn qa_pay027_rust_total_substeps_resume() {
    prop::run(&march(), |(v, k)| check_resume(generated, &v, k));
}

negative_control!(
    qa_pay027_rust_total_substeps_resume,
    "a total stored as its log proxy (not resumable) must fail",
    expected = "a resumed total_substeps is not the uninterrupted sum",
    prop::run(&march(), |(v, k)| check_resume(
        |s, var, a| {
            let mut out = generated(s, var, a);
            out.total_substeps = 1u32 << total_substeps_log2(s.total_substeps).min(31);
            out
        },
        &v,
        k.max(1)
    ))
);

fn check_log2(read: Read, t: u32) -> Result<(), TestCaseError> {
    let mut s = sample();
    s.total_substeps = t;
    let got = read(&s, true, &Args::default());
    prop_assert_eq!(got.total_substeps, t);
    prop_assert_eq!(
        got.total_substeps_log2,
        log2_ref(t),
        "total_substeps_log2({}) is not ⌊log₂⌋",
        t
    );
    Ok(())
}

#[test]
fn qa_pay027_rust_log2_over_u32() {
    for t in [0, 1, 2, 3, u32::MAX] {
        check_log2(generated, t).unwrap();
    }
    for k in 1..32 {
        let p = 1u32 << k;
        for t in [p - 1, p, p.wrapping_add(1)] {
            check_log2(generated, t).unwrap();
        }
    }
    prop::run(&any::<u32>(), |t| check_log2(generated, t));
}

negative_control!(
    qa_pay027_rust_log2_over_u32,
    "a ceil-log proxy must fail",
    expected = "is not ⌊log₂⌋",
    for t in [0u32, 1, 2, 3, 5] {
        check_log2(
            |s, v, a| {
                let mut out = generated(s, v, a);
                out.total_substeps_log2 = 32 - s.total_substeps.saturating_sub(1).leading_zeros();
                out
            },
            t,
        )
        .unwrap();
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-030

fn check_diffusion(read: Read) {
    let a = Args::default();
    // the valid cases first; then a latched sample read at two horizons; then n < 2
    for (n, horizon) in [
        (2u32, 1000u32),
        (3, 1000),
        (50, 1000),
        (50, 200),
        (999, 1000),
        (0, 1000),
        (1, 1000),
    ] {
        let mut s = sample();
        s.times = pack_times(n, 0);
        let args = Args {
            params: ReadParams {
                horizon_steps: horizon,
                ..a.params
            },
            ..a
        };
        let got = read(&s, true, &args);
        if n < 2 {
            assert_eq!(
                got.diffusion.to_bits(),
                QNAN,
                "diffusion at n = {n} is not the canonical NaN (R-245)"
            );
            assert!(
                !got.diffusion_slope_valid,
                "diffusion_slope_valid at n = {n}"
            );
        } else {
            let h = f64::from(a.params.dt_macro);
            let nf = f64::from(n);
            let want = f64::from(s.C_ty) / (h * h * nf * (nf * nf - 1.0) / 12.0);
            assert!(
                got.diffusion_slope_valid,
                "diffusion_slope_valid false at n = {n}"
            );
            assert!(
                near(got.diffusion, want, want.abs(), 10),
                "diffusion at n = {n} is {}, not C_ty / C_tt(n) = {want} with the sample's own n",
                got.diffusion
            );
        }
    }
}

#[test]
fn qa_pay030_rust_diffusion_slope() {
    check_diffusion(generated);
}

negative_control!(
    qa_pay030_rust_diffusion_slope,
    "a slope at the horizon, not the sample's own n, must fail",
    expected = "with the sample's own n",
    check_diffusion(|s, v, a| {
        let mut out = generated(s, v, a);
        out.diffusion = diffusion_slope(s.C_ty, a.params.horizon_steps, a.params.dt_macro);
        out
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-RENDER-019

fn check_fractions(read: Read) {
    for (te, td, h) in [
        (0u32, 0u32, 0u32),
        (17, 9, 0),
        (65535, 65535, 0),
        (65535, 65535, 65535),
        (1000, 1000, 1000),
        (3, 7, 1000),
        (333, 65535, 65535),
    ] {
        let mut s = sample();
        s.times = pack_times(te, td);
        let a = Args {
            params: ReadParams {
                horizon_steps: h,
                ..Args::default().params
            },
            ..Args::default()
        };
        let got = read(&s, true, &a);
        for (step, f, name) in [
            (te, got.t_end_fraction, "tm_t_end_fraction"),
            (td, got.t_dmin_fraction, "tm_t_dmin_fraction"),
        ] {
            if h == 0 {
                assert_eq!(f.to_bits(), 0, "{name}({step}, 0) is not 0");
            } else if step == h {
                assert_eq!(f, 1.0, "{name}({step}, {h}) is not exactly 1.0");
            } else {
                let want = f64::from(step) / f64::from(h);
                assert!(
                    near(f, want, want, 1),
                    "{name}({step}, {h}) is not step / horizon"
                );
            }
        }
    }
    assert_eq!(tm_t_end_fraction(pack_times(65535, 0), 65535), 1.0);
    assert_eq!(tm_t_end_fraction(pack_times(12, 0), 0).to_bits(), 0);
}

#[test]
fn qa_render019_rust_time_fraction() {
    check_fractions(generated);
}

negative_control!(
    qa_render019_rust_time_fraction,
    "an unguarded division at a zero horizon must fail",
    expected = "is not 0",
    check_fractions(|s, v, a| {
        let mut out = generated(s, v, a);
        out.t_end_fraction = (s.times & 0xFFFF) as f32 / a.params.horizon_steps as f32;
        out
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-031: the current drifts (RQ-203 option 2), and orbit_count / retrograde.

fn hamiltonian(r: [[f32; 2]; 3], p: [[f32; 2]; 3], m: [f32; 3]) -> (f64, f64) {
    let f = f64::from;
    let mut k = 0.0;
    let mut mag = 0.0;
    for i in 0..3 {
        let t = (f(p[i][0]).powi(2) + f(p[i][1]).powi(2)) / (2.0 * f(m[i]));
        k += t;
        mag += t.abs();
    }
    let mut v = 0.0;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let d = ((f(r[i][0]) - f(r[j][0])).powi(2) + (f(r[i][1]) - f(r[j][1])).powi(2)).sqrt();
        let t = f(m[i]) * f(m[j]) / d;
        v -= t;
        mag += t.abs();
    }
    (k + v, mag)
}

fn lz(r: [[f32; 2]; 3], p: [[f32; 2]; 3]) -> (f64, f64) {
    let f = f64::from;
    let mut l = 0.0;
    let mut mag = 0.0;
    for i in 0..3 {
        let a = f(r[i][0]) * f(p[i][1]);
        let b = f(r[i][1]) * f(p[i][0]);
        l += a - b;
        mag += a.abs() + b.abs();
    }
    (l, mag)
}

/// Hand-computed configurations (integrator dd §3.5, `G = 1`), `(state, masses, ΔE, ΔLz)`, each exact in f32:
/// - A: masses (1, 1, 2), r = (1,0), (−1,0), (0,0), p = (0,1), (0,−1), 0: K = ½ + ½ = 1; r₀₁ = 2, r₀₂ = r₁₂ = 1,
///   V = −(1/2 + 2 + 2) = −4.5, H = −3.5; L_z = 1·1 + (−1)(−1) = 2. E₀ = −3.25, Lz₀ = 1.5: ΔE = −0.25, ΔLz = 0.5.
/// - C: A with the momenta reversed: L_z = −2, ΔLz = −3.5 (the sign convention x·p_y − y·p_x).
/// - D: A with masses (2, 2, 4): K = ¼ + ¼ = ½, V = −(4/2 + 8 + 8) = −18, H = −17.5; E₀ = −3.5: ΔE = −14 (the masses
///   are the read's argument, the sample's `ICDescriptor`'s).
/// - E: masses (1, 1, 1), r = (0,0), (4,0), (0,−2), p = (2,0), (0,−2), (−2,2): K = 2 + 2 + 4 = 8; r₀₁ = 4, r₀₂ = 2,
///   r₁₂ = √20 is inexact, so E is checked against the f64 reference, not exactly.
fn drift_cases() -> Vec<(SimStateFTLE, [f32; 3], f64, f64, bool)> {
    let a = sample();
    let c = SimStateFTLE {
        p: [[0.0, -1.0], [0.0, 1.0], [0.0, 0.0]],
        ..sample()
    };
    let d = SimStateFTLE {
        E_0: -3.5,
        ..sample()
    };
    let e = SimStateFTLE {
        r: [[0.0, 0.0], [4.0, 0.0], [0.0, -2.0]],
        p: [[2.0, 0.0], [0.0, -2.0], [-2.0, 2.0]],
        E_0: 7.0,
        Lz_0: -1.0,
        ..sample()
    };
    let (he, _) = hamiltonian(e.r, e.p, [1.0; 3]);
    let (le, _) = lz(e.r, e.p);
    vec![
        (a, [1.0, 1.0, 2.0], -0.25, 0.5, true),
        (c, [1.0, 1.0, 2.0], -0.25, -3.5, true),
        (d, [2.0, 2.0, 4.0], -14.0, 0.5, true),
        (e, [1.0, 1.0, 1.0], he - 7.0, le + 1.0, false),
    ]
}

fn check_drifts(read: Read) {
    for (s, masses, de, dl, exact) in drift_cases() {
        // the f64 reference reproduces the hand values, so they are the formula's
        let (h, hmag) = hamiltonian(s.r, s.p, masses);
        let (l, lmag) = lz(s.r, s.p);
        assert!(
            (h - f64::from(s.E_0) - de).abs() < 1e-12 && (l - f64::from(s.Lz_0) - dl).abs() < 1e-12
        );
        for variant in [true, false] {
            let a = Args {
                masses,
                ..Args::default()
            };
            let got = read(&s, variant, &a);
            if exact {
                assert_eq!(
                    f64::from(got.energy_drift),
                    de,
                    "energy_drift is not the hand-computed H(r, p) − E_0"
                );
                assert_eq!(
                    f64::from(got.Lz_drift),
                    dl,
                    "Lz_drift is not the hand-computed L_z(r, p) − Lz_0"
                );
            } else {
                assert!(
                    near(got.energy_drift, de, hmag + f64::from(s.E_0).abs(), 30),
                    "energy_drift is not the f64 H(r, p) − E_0"
                );
                assert!(
                    near(got.Lz_drift, dl, lmag + f64::from(s.Lz_0).abs(), 8),
                    "Lz_drift is not the f64 L_z − Lz_0"
                );
            }
        }
    }
}

#[test]
fn qa_pay031_rust_current_drift_hand_configurations() {
    check_drifts(generated);
}

negative_control!(
    qa_pay031_rust_current_drift_hand_configurations,
    "an energy drift with the sim key's unit masses, not the sample's, must fail",
    expected = "energy_drift is not the hand-computed",
    check_drifts(|s, v, a| {
        let mut out = generated(s, v, a);
        out.energy_drift = energy_drift(s.r, s.p, [1.0, 1.0, 2.0], s.E_0);
        out
    })
);

/// A drift configuration: `r`, `p`, the masses, `E_0` and `Lz_0`.
type Config = ([[f32; 2]; 3], [[f32; 2]; 3], [f32; 3], f32, f32);

/// Random configurations: separations bounded away from 0, masses in (0.05, 2]; the f32 drift within the forward
/// error bound of the f64 reference.
fn config() -> impl Strategy<Value = Config> {
    let v = || {
        [
            [-4.0f32..4.0, -4.0f32..4.0],
            [-4.0f32..4.0, -4.0f32..4.0],
            [-4.0f32..4.0, -4.0f32..4.0],
        ]
    };
    (
        v(),
        v(),
        [0.05f32..2.0, 0.05f32..2.0, 0.05f32..2.0],
        -10.0f32..10.0,
        -10.0f32..10.0,
    )
        .prop_filter("separations ≥ 0.05", |(r, ..)| {
            [(0, 1), (0, 2), (1, 2)].iter().all(|&(i, j)| {
                let dx = r[i][0] - r[j][0];
                let dy = r[i][1] - r[j][1];
                (dx * dx + dy * dy).sqrt() >= 0.05
            })
        })
}

fn check_drift_reference(read: Read, (r, p, m, e0, l0): Config) -> Result<(), TestCaseError> {
    let s = SimStateFTLE {
        r,
        p,
        E_0: e0,
        Lz_0: l0,
        ..sample()
    };
    let (h, hmag) = hamiltonian(r, p, m);
    let (l, lmag) = lz(r, p);
    for variant in [true, false] {
        let got = read(
            &s,
            variant,
            &Args {
                masses: m,
                ..Args::default()
            },
        );
        prop_assert!(
            near(
                got.energy_drift,
                h - f64::from(e0),
                hmag + f64::from(e0).abs(),
                30
            ),
            "energy_drift {} is not the f64 H − E_0 {}",
            got.energy_drift,
            h - f64::from(e0)
        );
        prop_assert!(
            near(
                got.Lz_drift,
                l - f64::from(l0),
                lmag + f64::from(l0).abs(),
                8
            ),
            "Lz_drift {} is not the f64 L_z − Lz_0 {}",
            got.Lz_drift,
            l - f64::from(l0)
        );
    }
    Ok(())
}

#[test]
fn qa_pay031_rust_current_drift_f64_reference() {
    prop::run(&config(), |c| check_drift_reference(generated, c));
}

negative_control!(
    qa_pay031_rust_current_drift_f64_reference,
    "a potential with each pair's mass product taken once for all pairs must fail",
    expected = "energy_drift",
    prop::run(&config(), |c| check_drift_reference(
        |s, v, a| {
            let mut out = generated(s, v, a);
            // K − V with V's pairs all weighted m0·m1
            let k: f32 = (0..3)
                .map(|i| (s.p[i][0] * s.p[i][0] + s.p[i][1] * s.p[i][1]) / (2.0 * a.masses[i]))
                .sum();
            let v: f32 = [(0, 1), (0, 2), (1, 2)]
                .iter()
                .map(|&(i, j)| {
                    let dx = s.r[i][0] - s.r[j][0];
                    let dy = s.r[i][1] - s.r[j][1];
                    a.masses[0] * a.masses[1] / (dx * dx + dy * dy).sqrt()
                })
                .sum();
            out.energy_drift = k - v - s.E_0;
            out
        },
        c
    ))
);

fn check_winding(read: Read) {
    let tau = std::f64::consts::TAU;
    for t in [0.25f64, -0.25, 3.5 * tau, -2.5 * tau, 10.25 * tau] {
        let s = SimStateFTLE {
            theta: t as f32,
            ..sample()
        };
        let got = read(&s, true, &Args::default());
        let t = f64::from(s.theta);
        assert_eq!(
            got.orbit_count,
            (t.abs() / tau).floor() as u32,
            "orbit_count({t}) is not ⌊|θ|/2π⌋"
        );
        assert_eq!(got.retrograde, t < 0.0, "retrograde({t}) is not θ < 0");
    }
}

#[test]
fn qa_pay031_rust_orbit_count_and_retrograde() {
    check_winding(generated);
}

negative_control!(
    qa_pay031_rust_orbit_count_and_retrograde,
    "a winding count of the signed θ must fail",
    expected = "is not ⌊|θ|/2π⌋",
    check_winding(|s, v, a| {
        let mut out = generated(s, v, a);
        out.orbit_count = (s.theta / std::f32::consts::TAU).floor().max(0.0) as u32;
        out
    })
);
