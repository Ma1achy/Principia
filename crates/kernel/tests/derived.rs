//! The read side's Rust target (lowering Part 3a; payload §4–§6), `kernel::payload`'s generated `read_side.rs`, for the
//! kernel and the host: the same properties `ledger/tests/derived.rs` checks on the WGSL target, each test named with
//! `rust` after its requirement's filter, and the two targets' parity on the GPU (`read_side_parity`).
//! - REQ-PAY-021: `ftle` equals a reference with the partial renorm interval finalised (`derived_not_stored`).
//! - REQ-PAY-022: with FTLE baked out, `ftle` reads NaN and `ftle_valid` is false (`ftle_baked_out`).
//! - REQ-PAY-026: both variants unpack into the one `SimState`; with the tier off, `ftle` reads NaN
//!   (`read_type_both_tiers`).
//! - REQ-PAY-027: a resumed `total_substeps` equals the uninterrupted sum; the proxy is ⌊log₂⌋ over all of u32
//!   (`total_substeps_resume`).
//! - REQ-PAY-030: `diffusion` is NaN and invalid for `n < 2`; a latched sample uses its own `n` (`diffusion_slope`).
//! - REQ-PAY-032: `ftle_valid`'s truth table (`ftle_valid_truth_table`).
//! - REQ-RENDER-013: the tier-absent reads are lowering Part 3a's bits (`tier_absent_nan_bits`).
//! - REQ-RENDER-019: the time fractions at a zero horizon and at it (`time_fraction`).
//!
//! Each check takes the read as a function, so its control runs the same check on a read with one fault and shows it
//! fails (pitfalls §9).

use core::mem::offset_of;

use kernel::payload::*;
use proptest::prelude::*;
use validation::gpu::GpuHarness;
use validation::{negative_control, prop};

/// Lowering Part 3a's canonical quiet NaN and unbound word (R-72; REQ-RENDER-077).
const QNAN: u32 = 0x7fc0_0000;
const UNBOUND: [u32; 4] = [0, 0, 0, 0xfe00_0000];

/// One read: the FTLE variant's stored state (the base variant is the same less the shadow), which variant is read,
/// the word and ensemble arguments, and the sim-key values.
#[derive(Clone, Copy, Debug)]
struct Case {
    state: SimStateFTLE,
    ftle_variant: bool,
    word: [u32; 4],
    has_word: bool,
    spread: f32,
    has_ensemble: bool,
    params: ReadParams,
}

/// A read: the generated unpacks ([`generated_read`]), or a control's faulty one.
type Read = fn(&Case) -> SimState;

/// `c.state` as `SimStateBase`: the shadow dropped.
fn base(s: &SimStateFTLE) -> SimStateBase {
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

/// The case read through the generated unpack of its variant.
fn generated_read(c: &Case) -> SimState {
    let (word, spread, p) = (c.word, c.spread, &c.params);
    if c.ftle_variant {
        sim_state_from_ftle(&c.state, word, c.has_word, spread, c.has_ensemble, p)
    } else {
        sim_state_from_base(&base(&c.state), word, c.has_word, spread, c.has_ensemble, p)
    }
}

/// A bounded sample at step `n` whose shadow sits `ratio · δ₀` (δ₀ = 1e-6) from it along `r[0].x`, `S` = 2, `C_ty` = 3,
/// read from the FTLE variant with the word bound, E ≥ 1, `dt_macro` 0.01, `n_renorm` 16 and horizon 1000.
fn marching(n: u32, ratio: f32) -> Case {
    let r = [[0.5, -0.25], [-0.75, 0.125], [0.25, 0.125]];
    let p = [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]];
    let mut r_sh = r;
    r_sh[0][0] += ratio * 1e-6;
    let state = SimStateFTLE {
        r,
        p,
        r_sh,
        p_sh: p,
        S: 2.0,
        theta: 7.0,
        C_ty: 3.0,
        packed_a: set_state(PA_D_MIN_UNSET << 16, STATE_BOUNDED),
        times: n | (n / 2) << 16,
        total_substeps: 1000,
        closure_step: 9,
        ..SimStateFTLE::default()
    };
    Case {
        state,
        ftle_variant: true,
        word: [1, 2, 3, 4 << 25],
        has_word: true,
        spread: 0.5,
        has_ensemble: true,
        params: ReadParams {
            dt_macro: 0.01,
            delta_0: 1e-6,
            n_renorm: 16,
            horizon_steps: 1000,
        },
    }
}

/// `c` at step `n` (`t_end_step`), `t_dmin_step` kept.
fn at_step(mut c: Case, n: u32) -> Case {
    c.state.times = (c.state.times & 0xffff_0000) | n;
    c
}

/// `S_final / (n · dt)`, `S_final = S + ln(δ/δ₀)`, in f64 from the stored f32 values (payload §5).
fn ftle_reference(c: &Case) -> f64 {
    let s = &c.state;
    let sq = |a: &[[f32; 2]; 3], b: &[[f32; 2]; 3]| -> f64 {
        let a = a.iter().flatten();
        a.zip(b.iter().flatten())
            .map(|(&x, &y)| (f64::from(y) - f64::from(x)).powi(2))
            .sum()
    };
    let delta = (sq(&s.r, &s.r_sh) + sq(&s.p, &s.p_sh)).sqrt();
    let s_final = f64::from(s.S) + (delta / f64::from(c.params.delta_0)).ln();
    s_final / (f64::from(tm_t_end_step(s.times)) * f64::from(c.params.dt_macro))
}

/// `C_ty / C_tt(n)`, `C_tt(n) = h²·n(n²−1)/12`, in f64 (payload §4).
fn diffusion_reference(c_ty: f32, n: u32, h: f32) -> f64 {
    let (n, h) = (f64::from(n), f64::from(h));
    f64::from(c_ty) / (h * h * n * (n * n - 1.0) / 12.0)
}

/// Whether `got` is within the f32 read's error of `want`: 1e-4 relative, against at least `scale`.
fn close(got: f32, want: f64, scale: f64) -> bool {
    (f64::from(got) - want).abs() <= 1e-4 * want.abs().max(scale)
}

// ── derived_not_stored (REQ-PAY-021) ──────────────────────────────────────────────────────────────────────────────

/// Each case's `ftle`, through `read`, against [`ftle_reference`].
fn check_ftle_reference(read: Read, cases: &[Case]) {
    for c in cases {
        let got = read(c);
        let n = tm_t_end_step(c.state.times);
        let scale = 1.0 / (f64::from(n) * f64::from(c.params.dt_macro));
        assert!(got.ftle_valid, "ftle_valid is false for {c:?}");
        let want = ftle_reference(c);
        assert!(
            close(got.ftle, want, scale),
            "ftle {} differs from the finalised reference {want} for {c:?}",
            got.ftle
        );
    }
}

/// Payload §5's example: renormalised every 16 steps, read between boundaries.
fn partial_interval() -> Vec<Case> {
    [17, 30, 31, 47].map(|n| marching(n, 50.0)).to_vec()
}

#[test]
fn derived_not_stored_rust_ftle_finalises_the_partial_interval() {
    check_ftle_reference(generated_read, &partial_interval());
}

negative_control!(
    derived_not_stored_rust_ftle_finalises_the_partial_interval,
    "plain S/t, the unfinished interval dropped, must fail the reference",
    expected = "differs from the finalised reference",
    check_ftle_reference(
        |c| {
            let mut s = generated_read(c);
            let n = tm_t_end_step(c.state.times) as f32;
            s.ftle = c.state.S / (n * c.params.dt_macro);
            s
        },
        &partial_interval()
    )
);

/// 64 cases from `seed`: positions and momenta in [−2, 2], the shadow `ratio · δ₀` off along a random direction, `S`,
/// `δ₀`, `n`, `n_renorm` and `dt` drawn; each state a resolved outcome or running.
fn random_cases(seed: u64) -> Vec<Case> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..64)
        .map(|_| {
            let mut c = marching(40, 50.0);
            let mut v = [[[0f32; 2]; 3]; 2];
            v.iter_mut()
                .flatten()
                .flatten()
                .for_each(|x| *x = (4.0 * next() - 2.0) as f32);
            let delta_0 = 10f64.powf(-5.0 + 3.0 * next());
            let ratio = 2.0 + 998.0 * next();
            let mut dir = [0f64; 12];
            dir.iter_mut().for_each(|d| *d = next() - 0.5);
            let norm = dir.iter().map(|d| d * d).sum::<f64>().sqrt();
            let mut sh = v;
            for (k, x) in sh.iter_mut().flatten().flatten().enumerate() {
                *x = (f64::from(*x) + ratio * delta_0 * dir[k] / norm) as f32;
            }
            let n_renorm = 1 + (next() * 64.0) as u32;
            let n = n_renorm + (next() * f64::from(65535 - n_renorm)) as u32;
            (c.state.r, c.state.p, c.state.r_sh, c.state.p_sh) = (v[0], v[1], sh[0], sh[1]);
            c.state.S = (50.0 * next()) as f32;
            c.state.theta = (200.0 * next() - 100.0) as f32;
            c.state.packed_a = set_state(c.state.packed_a, (next() * 4.0) as u32);
            c.state.total_substeps = (next() * f64::from(u32::MAX)) as u32;
            c.params.delta_0 = delta_0 as f32;
            c.params.n_renorm = n_renorm;
            c.params.dt_macro = (1e-3 + 0.1 * next()) as f32;
            c.params.horizon_steps = n + (next() * f64::from(65535 - n)) as u32;
            at_step(c, n)
        })
        .collect()
}

#[test]
fn derived_not_stored_rust_ftle_property_matches_the_finalised_reference() {
    prop::run(&any::<u64>(), |seed| {
        check_ftle_reference(generated_read, &random_cases(seed));
        Ok(())
    });
}

negative_control!(
    derived_not_stored_rust_ftle_property_matches_the_finalised_reference,
    "dividing by the completed renormalisations' time instead of n·dt must fail the reference",
    expected = "differs from the finalised reference",
    check_ftle_reference(
        |c| {
            let mut s = generated_read(c);
            let n = tm_t_end_step(c.state.times);
            s.ftle *= n as f32 / (n - n % c.params.n_renorm) as f32;
            s
        },
        &partial_interval()
    )
);

// ── ftle_baked_out (REQ-PAY-022) ──────────────────────────────────────────────────────────────────────────────────

/// A sample with every other `ftle_valid` clause true, read from `SimStateBase`: `ftle` is the canonical quiet NaN and
/// `ftle_valid` false.
fn check_ftle_baked_out(read: Read) {
    let mut c = marching(40, 50.0);
    c.ftle_variant = false;
    let got = read(&c);
    assert!(!got.ftle_valid, "ftle_valid is true with FTLE baked out");
    assert_eq!(
        got.ftle.to_bits(),
        QNAN,
        "ftle reads {:#010x} with FTLE baked out, not the canonical quiet NaN",
        got.ftle.to_bits()
    );
}

#[test]
fn ftle_baked_out_rust_reads_nan_and_is_invalid() {
    check_ftle_baked_out(generated_read);
}

negative_control!(
    ftle_baked_out_rust_reads_nan_and_is_invalid,
    "a base read that returns the stored S as ftle must fail",
    expected = "with FTLE baked out, not the canonical quiet NaN",
    check_ftle_baked_out(|c| {
        let mut s = generated_read(c);
        s.ftle = c.state.S;
        s
    })
);

// ── read_type_both_tiers (REQ-PAY-026) ────────────────────────────────────────────────────────────────────────────

/// One case read from each variant gives one `SimState` (the type is the same by construction): every member but
/// `ftle` and `ftle_valid` agrees, and the variant without the shadow reads `ftle` as the canonical quiet NaN.
fn check_both_tiers(read: Read) {
    let on = marching(40, 50.0);
    let off = Case {
        ftle_variant: false,
        ..on
    };
    let (a, b) = (read(&on), read(&off));
    assert_eq!(
        b.ftle.to_bits(),
        QNAN,
        "the tier without the shadow reads ftle {:#010x}, not the canonical quiet NaN",
        b.ftle.to_bits()
    );
    let strip = |s: SimState| SimState {
        ftle: 0.0,
        ftle_valid: false,
        ..s
    };
    assert_eq!(strip(a), strip(b), "the tiers' reads differ beyond ftle");
}

#[test]
fn read_type_both_tiers_rust_one_type_ftle_off_reads_nan() {
    check_both_tiers(generated_read);
}

negative_control!(
    read_type_both_tiers_rust_one_type_ftle_off_reads_nan,
    "a read that computes ftle at both tiers must fail",
    expected = "not the canonical quiet NaN",
    check_both_tiers(|c| generated_read(&Case {
        ftle_variant: true,
        ..*c
    }))
);

// ── total_substeps_resume (REQ-PAY-027) ───────────────────────────────────────────────────────────────────────────

/// ⌊log₂ total⌋ for a total ≥ 2, 0 for 0 and 1 (payload §6; R-86).
fn log2_reference(total: u32) -> u32 {
    if total > 1 {
        total.ilog2()
    } else {
        0
    }
}

/// A march from `start` substeps adding `n_sub`, stored after its first `split` steps in each variant and resumed
/// from the read `total_substeps`: the resumed total equals the uninterrupted one, and the proxy is ⌊log₂⌋.
fn check_resume(read: Read, start: u32, n_sub: &[u32], split: usize) {
    let (done, rest) = n_sub.split_at(split);
    let stored = start + done.iter().sum::<u32>();
    for ftle_variant in [true, false] {
        let mut c = marching(40, 50.0);
        c.ftle_variant = ftle_variant;
        c.state.total_substeps = stored;
        let got = read(&c);
        let resumed = got.total_substeps + rest.iter().sum::<u32>();
        assert_eq!(
            resumed,
            start + n_sub.iter().sum::<u32>(),
            "the resumed total differs from the uninterrupted march (stored {stored}, read {})",
            got.total_substeps
        );
        assert_eq!(
            got.total_substeps_log2,
            log2_reference(stored),
            "total_substeps_log2({stored})"
        );
    }
}

#[test]
fn total_substeps_resume_rust_equals_the_uninterrupted_march() {
    let marches = (
        0u32..=u32::MAX - 64 * 512,
        proptest::collection::vec(1u32..=64, 1..512),
        any::<proptest::sample::Index>(),
    );
    prop::run(&marches, |(start, n_sub, split)| {
        check_resume(generated_read, start, &n_sub, split.index(n_sub.len() + 1));
        Ok(())
    });
}

negative_control!(
    total_substeps_resume_rust_equals_the_uninterrupted_march,
    "a read that keeps only the log proxy cannot resume the count",
    expected = "the resumed total differs from the uninterrupted march",
    check_resume(
        |c| {
            let mut s = generated_read(c);
            s.total_substeps = 1 << s.total_substeps_log2;
            s
        },
        1000,
        &[3, 5, 7],
        1
    )
);

/// `proxy(total)` is [`log2_reference`]'s for each total.
fn check_proxy(proxy: fn(u32) -> u32, totals: &[u32]) {
    for &t in totals {
        assert_eq!(
            proxy(t),
            log2_reference(t),
            "total_substeps_log2({t}) is not ⌊log₂⌋"
        );
    }
}

/// 0, 1, and 2^k − 1, 2^k and 2^k + 1 for every k, and u32::MAX.
fn proxy_edges() -> Vec<u32> {
    let mut out = vec![0, 1, u32::MAX];
    for k in 1..32 {
        let p = 1u32 << k;
        out.extend([p - 1, p, p + 1]);
    }
    out
}

#[test]
fn total_substeps_resume_rust_proxy_at_every_power_of_two() {
    check_proxy(total_substeps_log2, &proxy_edges());
}

negative_control!(
    total_substeps_resume_rust_proxy_at_every_power_of_two,
    "a proxy one too large must fail",
    expected = "is not ⌊log₂⌋",
    check_proxy(|t| total_substeps_log2(t) + 1, &proxy_edges())
);

#[test]
fn total_substeps_resume_rust_proxy_property_over_u32() {
    prop::run(&any::<u32>(), |t| {
        check_proxy(total_substeps_log2, &[t]);
        Ok(())
    });
}

negative_control!(
    total_substeps_resume_rust_proxy_property_over_u32,
    "a proxy that rounds up must fail",
    expected = "is not ⌊log₂⌋",
    check_proxy(|t| 32 - t.saturating_sub(1).leading_zeros(), &[3])
);

// ── diffusion_slope (REQ-PAY-030) ─────────────────────────────────────────────────────────────────────────────────

/// At `n` = 0 and 1, `diffusion` is the canonical quiet NaN and `diffusion_slope_valid` false (R-245).
fn check_diffusion_invalid(read: Read) {
    for n in [0, 1] {
        let got = read(&at_step(marching(40, 50.0), n));
        assert!(
            !got.diffusion_slope_valid,
            "diffusion_slope_valid is true at n = {n}"
        );
        assert_eq!(
            got.diffusion.to_bits(),
            QNAN,
            "diffusion at n = {n} is not NaN"
        );
    }
}

#[test]
fn diffusion_slope_rust_reads_nan_below_two() {
    check_diffusion_invalid(generated_read);
}

negative_control!(
    diffusion_slope_rust_reads_nan_below_two,
    "a fit valid from n = 1 must fail",
    expected = "diffusion_slope_valid is true at n = 1",
    check_diffusion_invalid(|c| {
        let mut s = generated_read(c);
        s.diffusion_slope_valid = tm_t_end_step(c.state.times) >= 1;
        s
    })
);

/// A sample latched at step 40 (an escape) and one running at step 100, the horizon 1000: each slope is
/// `C_ty / C_tt` at its own `t_end_step`, and valid.
fn check_diffusion_own_n(read: Read) {
    let mut latched = at_step(marching(40, 50.0), 40);
    latched.state.packed_a = set_state(latched.state.packed_a, STATE_ESCAPE);
    let mut running = at_step(marching(40, 50.0), 100);
    running.state.packed_a = set_state(running.state.packed_a, STATE_RUNNING);
    for c in [latched, running] {
        let n = tm_t_end_step(c.state.times);
        let got = read(&c);
        let want = diffusion_reference(c.state.C_ty, n, c.params.dt_macro);
        assert!(
            got.diffusion_slope_valid,
            "diffusion_slope_valid is false at n = {n}"
        );
        assert!(
            close(got.diffusion, want, 0.0),
            "diffusion {} is not C_ty/C_tt at the sample's own n = {n} ({want})",
            got.diffusion
        );
    }
}

#[test]
fn diffusion_slope_rust_latched_sample_uses_its_own_n() {
    check_diffusion_own_n(generated_read);
}

negative_control!(
    diffusion_slope_rust_latched_sample_uses_its_own_n,
    "a slope at the horizon's n must fail",
    expected = "is not C_ty/C_tt at the sample's own n",
    check_diffusion_own_n(|c| {
        let mut s = generated_read(c);
        let h = c.params.horizon_steps;
        s.diffusion = diffusion_slope(c.state.C_ty, h, c.params.dt_macro);
        s
    })
);

const SLOPE_NS: [u32; 6] = [2, 3, 17, 1000, 40000, 65535];

/// At each `n`, `diffusion` is [`diffusion_reference`]'s.
fn check_diffusion_reference(read: Read, ns: &[u32]) {
    for &n in ns {
        let c = at_step(marching(40, 50.0), n);
        let want = diffusion_reference(c.state.C_ty, n, c.params.dt_macro);
        let got = read(&c).diffusion;
        assert!(
            close(got, want, 0.0),
            "diffusion {got} at n = {n} differs from C_ty/C_tt(n) = {want}"
        );
    }
}

#[test]
fn diffusion_slope_rust_matches_the_closed_form() {
    check_diffusion_reference(generated_read, &SLOPE_NS);
}

negative_control!(
    diffusion_slope_rust_matches_the_closed_form,
    "a C_tt without its /12 must fail",
    expected = "differs from C_ty/C_tt(n)",
    check_diffusion_reference(
        |c| {
            let mut s = generated_read(c);
            s.diffusion /= 12.0;
            s
        },
        &SLOPE_NS
    )
);

// ── ftle_valid_truth_table (REQ-PAY-032) ──────────────────────────────────────────────────────────────────────────

/// Every tier, state code 0–7, `n` and `n_renorm` of the table: `ftle_valid` is the tier on, the state not failed,
/// `n > 0` and `n / n_renorm > 0` (none when `n_renorm` is 0); `ftle` is the canonical quiet NaN exactly when it is
/// false (R-254); and the state predicates follow the code (payload §6).
fn check_truth_table(read: Read) {
    for tier in [true, false] {
        for state in 0..8 {
            for n in [0, 1, 15, 16, 17, 40, 65535] {
                for n_renorm in [0, 1, 16] {
                    let mut c = at_step(marching(40, 50.0), n);
                    c.state.packed_a = set_state(c.state.packed_a, state);
                    c.ftle_variant = tier;
                    c.params.n_renorm = n_renorm;
                    let got = read(&c);
                    let renorms = n.checked_div(n_renorm).unwrap_or(0);
                    let valid = tier && state <= 3 && n > 0 && renorms > 0;
                    let row = format!("tier {tier}, state {state}, n {n}, n_renorm {n_renorm}");
                    assert_eq!(got.ftle_valid, valid, "ftle_valid at {row}");
                    assert_eq!(
                        got.ftle.to_bits() == QNAN,
                        !valid,
                        "ftle is NaN exactly when ftle_valid is false, at {row}"
                    );
                    let predicates = [state <= 2, state == 3, state >= 4, state != 3];
                    let got_predicates = [
                        got.is_resolved_outcome,
                        got.is_running,
                        got.is_failed,
                        got.is_finished,
                    ];
                    assert_eq!(got_predicates, predicates, "the state predicates at {row}");
                }
            }
        }
    }
}

#[test]
fn ftle_valid_truth_table_rust_over_tier_state_n_and_renorms() {
    check_truth_table(generated_read);
}

negative_control!(
    ftle_valid_truth_table_rust_over_tier_state_n_and_renorms,
    "an ftle_valid that ignores a failed state must fail",
    expected = "ftle_valid at tier true, state 4",
    check_truth_table(|c| {
        let mut s = generated_read(c);
        let n = tm_t_end_step(c.state.times);
        let renorms = completed_renorms(n, c.params.n_renorm);
        s.ftle_valid = ftle_valid(STATE_BOUNDED, c.ftle_variant, n, renorms);
        s
    })
);

#[test]
fn ftle_valid_truth_table_rust_accessor_takes_the_tier() {
    check_accessor_tier(ftle_valid);
}

/// `ftle_valid(state, tier, n, renorms)`, payload §6's accessor, is false with the tier off and true with it on, for
/// a bounded sample at step 40 with 2 completed renorms.
fn check_accessor_tier(valid: fn(u32, bool, u32, u32) -> bool) {
    assert!(
        valid(STATE_BOUNDED, true, 40, 2),
        "ftle_valid is false with the tier on"
    );
    assert!(
        !valid(STATE_BOUNDED, false, 40, 2),
        "ftle_valid is true with the tier off"
    );
}

negative_control!(
    ftle_valid_truth_table_rust_accessor_takes_the_tier,
    "an accessor that ignores the tier must fail",
    expected = "ftle_valid is true with the tier off",
    check_accessor_tier(|s, _, n, r| ftle_valid(s, true, n, r))
);

// ── tier_absent_nan_bits (REQ-RENDER-013) ─────────────────────────────────────────────────────────────────────────

/// A no-FTLE read's `ftle`, and `ensemble_spread` at E = 0, are the canonical quiet NaN's bits, and an unbound word
/// reads the unbound word; at E ≥ 1 and with the word bound, the values pass through. The generated constants are
/// lowering Part 3a's.
fn check_tier_absent(read: Read) {
    assert_eq!(CANONICAL_QNAN_BITS, QNAN, "CANONICAL_QNAN_BITS");
    assert_eq!(FGW_UNBOUND, UNBOUND, "FGW_UNBOUND");
    let present = marching(40, 50.0);
    let base = read(&Case {
        ftle_variant: false,
        ..present
    });
    let absent = read(&Case {
        has_ensemble: false,
        has_word: false,
        ..present
    });
    let bound = read(&present);
    assert_eq!(
        base.ftle.to_bits(),
        QNAN,
        "a no-FTLE read's ftle is {:#010x}, not the canonical quiet NaN",
        base.ftle.to_bits()
    );
    assert_eq!(
        absent.ensemble_spread.to_bits(),
        QNAN,
        "ensemble_spread at E = 0 is not the canonical quiet NaN"
    );
    assert_eq!(
        absent.word, UNBOUND,
        "an unbound word buffer does not read the unbound word"
    );
    assert_eq!(
        bound.ensemble_spread, present.spread,
        "ensemble_spread at E ≥ 1 is not the resolve value"
    );
    assert_eq!(
        bound.word, present.word,
        "a bound word does not read through"
    );
    assert!(
        !fgw_truncated(bound.word) && fgw_truncated(absent.word),
        "the unbound word is not invalid"
    );
}

#[test]
fn tier_absent_nan_bits_rust_read_the_canonical_patterns() {
    check_tier_absent(generated_read);
}

negative_control!(
    tier_absent_nan_bits_rust_read_the_canonical_patterns,
    "a NaN computed by arithmetic, sign set, must fail",
    expected = "a no-FTLE read's ftle is",
    check_tier_absent(|c| {
        let mut s = generated_read(c);
        if !c.ftle_variant {
            s.ftle = -f32::NAN;
        }
        s
    })
);

// ── time_fraction (REQ-RENDER-019) ────────────────────────────────────────────────────────────────────────────────

/// `(t_end_step, t_dmin_step, horizon_steps)` and the expected fractions.
type FractionCase = (u32, u32, u32, f32, f32);

const FRACTIONS: [FractionCase; 4] = [
    (5, 7, 0, 0.0, 0.0),
    (0, 0, 0, 0.0, 0.0),
    (65535, 65535, 65535, 1.0, 1.0),
    (1000, 0, 1000, 1.0, 0.0),
];

/// Each case's fractions, bit for bit: 0 at a zero horizon, exactly 1.0 at it (payload §2, §6; R-361).
fn check_fractions(read: Read, fractions: &[FractionCase]) {
    for &(end, dmin, horizon, e, d) in fractions {
        let mut c = marching(40, 50.0);
        c.state.times = end | dmin << 16;
        c.params.horizon_steps = horizon;
        let got = read(&c);
        assert_eq!(
            [got.t_end_fraction.to_bits(), got.t_dmin_fraction.to_bits()],
            [e.to_bits(), d.to_bits()],
            "the fractions of ({end}, {dmin}) at horizon {horizon} are ({}, {}), not ({e}, {d})",
            got.t_end_fraction,
            got.t_dmin_fraction
        );
    }
}

#[test]
fn time_fraction_rust_zero_horizon_and_endpoint() {
    check_fractions(generated_read, &FRACTIONS);
}

negative_control!(
    time_fraction_rust_zero_horizon_and_endpoint,
    "fractions without the zero-horizon guard must fail",
    expected = "at horizon 0",
    check_fractions(
        |c| {
            let mut s = generated_read(c);
            let h = c.params.horizon_steps as f32;
            s.t_end_fraction = tm_t_end_step(c.state.times) as f32 / h;
            s
        },
        &FRACTIONS
    )
);

// ── read_side_parity: the Rust and WGSL targets (lowering Part 3a; dd_generation_root §1) ────────────────────────

/// The generated fragment unpack layer and the read side that follows it, and the read side's test entry point.
const LAYER_WGSL: &str = include_str!("../../render/frag/generated/payload_unpack.wgsl");
const READ_SIDE_WGSL: &str = include_str!("../../render/frag/generated/read_side.wgsl");
const ENTRY_WGSL: &str = include_str!("../../ledger/tests/derived_entry.wgsl");
/// `derived_entry.wgsl`'s selectors, 0–23.
const MEMBERS: u32 = 24;

/// `SimStateFTLE`'s and `SimStateBase`'s f32 members, each with its byte offset in the variant.
macro_rules! words {
    ($ty:ty, $s:expr, $($f:ident),*) => {{
        let mut out = vec![0u32; size_of::<$ty>() / 4];
        let s = $s;
        $(
            let bits: Vec<u32> = words!(@bits s.$f);
            let at = offset_of!($ty, $f) / 4;
            out[at..at + bits.len()].copy_from_slice(&bits);
        )*
        out[offset_of!($ty, closure_step) / 4] = u32::from(s.closure_step) | u32::from(s._reserved) << 16;
        out
    }};
    (@bits $v:expr) => {
        Bits::bits(&$v)
    };
}

/// A stored member's u32 words.
trait Bits {
    fn bits(&self) -> Vec<u32>;
}

impl Bits for f32 {
    fn bits(&self) -> Vec<u32> {
        vec![self.to_bits()]
    }
}

impl Bits for u32 {
    fn bits(&self) -> Vec<u32> {
        vec![*self]
    }
}

impl Bits for [[f32; 2]; 3] {
    fn bits(&self) -> Vec<u32> {
        self.iter().flatten().map(|x| x.to_bits()).collect()
    }
}

/// Each case's members 0–23 read on the GPU through the WGSL read side, the tier `has_ftle = true`.
fn wgsl_reads(gpu: &GpuHarness, cases: &[Case]) -> Vec<Vec<u32>> {
    let (mut sel, mut ftle, mut base_w, mut word, mut args) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, c) in (0u32..).zip(cases) {
        sel.extend((0..MEMBERS).map(|m| i << 8 | m));
        let s = &c.state;
        ftle.extend(words!(
            SimStateFTLE,
            s,
            r,
            p,
            r_sh,
            p_sh,
            S,
            theta,
            mean_y,
            C_ty,
            E_0,
            Lz_0,
            packed_a,
            packed_b,
            times,
            total_substeps,
            closure_min
        ));
        let b = &base(s);
        base_w.extend(words!(
            SimStateBase,
            b,
            r,
            p,
            S,
            theta,
            mean_y,
            C_ty,
            E_0,
            Lz_0,
            packed_a,
            packed_b,
            times,
            total_substeps,
            closure_min
        ));
        word.extend(c.word);
        let flags =
            u32::from(c.ftle_variant) | u32::from(c.has_word) << 1 | u32::from(c.has_ensemble) << 2;
        let p = &c.params;
        args.extend([
            flags,
            c.spread.to_bits(),
            p.dt_macro.to_bits(),
            p.delta_0.to_bits(),
        ]);
        args.extend([p.n_renorm, p.horizon_steps, 0, 0]);
    }
    let module =
        format!("const has_ftle: bool = true;\n{LAYER_WGSL}{READ_SIDE_WGSL}\n{ENTRY_WGSL}");
    let out = gpu.run_wgsl(&module, "t_derived", &[&sel, &ftle, &base_w, &word, &args]);
    out.chunks(MEMBERS as usize).map(<[u32]>::to_vec).collect()
}

/// `s`'s members 0–23 in `derived_entry.wgsl`'s order, an f32 as its bits, a bool as 0 or 1.
fn rust_members(s: &SimState) -> [u32; 24] {
    let b = u32::from;
    [
        s.ftle.to_bits(),
        b(s.ftle_valid),
        s.diffusion.to_bits(),
        b(s.diffusion_slope_valid),
        s.total_substeps_log2,
        s.t_end_fraction.to_bits(),
        s.t_dmin_fraction.to_bits(),
        s.orbit_count,
        b(s.retrograde),
        b(s.is_resolved_outcome),
        b(s.is_running),
        b(s.is_failed),
        b(s.is_finished),
        s.ensemble_spread.to_bits(),
        s.word[0],
        s.word[1],
        s.word[2],
        s.word[3],
        s.total_substeps,
        s.t_end_step,
        s.state,
        s.S.to_bits(),
        s.closure_step,
        s.d_min.to_bits(),
    ]
}

/// The continuous members, compared within tolerance; every other member is compared bit for bit: an exact field, a
/// predicate, a sentinel (dd_simstate_payload §1, "Parity").
const CONTINUOUS: [usize; 4] = [0, 2, 5, 6];

/// Each case read through `read` and through the WGSL read side agree: exact members bit for bit, the continuous ones
/// within 1e-4 relative, or bit for bit where either is the canonical NaN. `orbit_count` may differ by one at an exact
/// multiple of 2π, which the cases avoid.
fn check_parity(gpu: &GpuHarness, read: Read, cases: &[Case]) {
    for (c, gpu_m) in cases.iter().zip(wgsl_reads(gpu, cases)) {
        let cpu_m = rust_members(&read(c));
        for (k, (&a, &g)) in cpu_m.iter().zip(&gpu_m).enumerate() {
            let same = if CONTINUOUS.contains(&k) && a != QNAN && g != QNAN {
                let (a, g) = (f64::from(f32::from_bits(a)), f64::from(f32::from_bits(g)));
                (a - g).abs() <= 1e-4 * a.abs().max(g.abs()).max(1e-30)
            } else {
                a == g
            };
            assert!(
                same,
                "member {k}: Rust {a:#010x}, WGSL {g:#010x}, for {c:?}"
            );
        }
    }
}

/// Every random case at each variant, with the word and ensemble each bound and absent.
fn parity_cases(seed: u64) -> Vec<Case> {
    random_cases(seed)
        .into_iter()
        .enumerate()
        .map(|(k, c)| Case {
            ftle_variant: k % 2 == 0,
            has_word: k % 3 != 0,
            has_ensemble: k % 5 != 0,
            ..c
        })
        .collect()
}

#[test]
fn read_side_parity_rust_and_wgsl_agree() {
    let gpu = GpuHarness::new().expect("a GPU device");
    prop::run(&any::<u64>(), |seed| {
        check_parity(&gpu, generated_read, &parity_cases(seed));
        Ok(())
    });
}

negative_control!(
    read_side_parity_rust_and_wgsl_agree,
    "a Rust read with orbit_count off by one must fail",
    expected = "member 7: Rust",
    check_parity(
        &GpuHarness::new().expect("a GPU device"),
        |c| {
            let mut s = generated_read(c);
            s.orbit_count += 1;
            s
        },
        &parity_cases(1)
    )
);
