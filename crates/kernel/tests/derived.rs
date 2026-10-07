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
//! - REQ-PAY-031: the current drifts `energy_drift = H(r, p) − E_0` and `Lz_drift = L_z(r, p) − Lz_0`, computed at read
//!   (payload §5; dd_generation_root §3.8), against hand-computed configurations and an f64 host reference, and the
//!   two targets' agreement on them on the GPU (`current_drift`).
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
/// the word and ensemble arguments, the sample's `ICDescriptor` masses, and the sim-key values.
#[derive(Clone, Copy, Debug)]
struct Case {
    state: SimStateFTLE,
    masses: [f32; 3],
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
    let (word, spread, m, p) = (c.word, c.spread, c.masses, &c.params);
    if c.ftle_variant {
        sim_state_from_ftle(&c.state, word, c.has_word, spread, c.has_ensemble, m, p)
    } else {
        sim_state_from_base(
            &base(&c.state),
            word,
            c.has_word,
            spread,
            c.has_ensemble,
            m,
            p,
        )
    }
}

/// A bounded sample at step `n` whose shadow sits `ratio · δ₀` (δ₀ = 1e-6) from it along `r[0].x`, `S` = 2, `C_ty` = 3,
/// `E_0` = −1, `Lz_0` = 0.1, read from the FTLE variant with the word bound, E ≥ 1, masses (0.25, 0.35, 0.4),
/// `dt_macro` 0.01, `n_renorm` 16 and horizon 1000.
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
        E_0: -1.0,
        Lz_0: 0.1,
        packed_a: set_state(PA_D_MIN_UNSET << 16, STATE_BOUNDED),
        times: n | (n / 2) << 16,
        total_substeps: 1000,
        closure_step: 9,
        ..SimStateFTLE::default()
    };
    Case {
        state,
        masses: [0.25, 0.35, 0.4],
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

/// `S_final / (n · dt)`, `S_final = S + ln(δ/δ₀)`, in f64 from the stored f32 values (payload §5), and the f32
/// read's error bound against it ([`ftle_bound`]).
fn ftle_reference(c: &Case) -> (f64, f64) {
    let s = &c.state;
    let sq = |a: &[[f32; 2]; 3], b: &[[f32; 2]; 3]| -> f64 {
        let a = a.iter().flatten();
        a.zip(b.iter().flatten())
            .map(|(&x, &y)| (f64::from(y) - f64::from(x)).powi(2))
            .sum()
    };
    let delta = (sq(&s.r, &s.r_sh) + sq(&s.p, &s.p_sh)).sqrt();
    let l = (delta / f64::from(c.params.delta_0)).ln();
    let s_final = f64::from(s.S) + l;
    let n_dt = f64::from(tm_t_end_step(s.times)) * f64::from(c.params.dt_macro);
    (s_final / n_dt, ftle_bound(l, s_final, n_dt))
}

/// `C_ty / C_tt(n)`, `C_tt(n) = h²·n(n²−1)/12`, in f64 (payload §4).
fn diffusion_reference(c_ty: f32, n: u32, h: f32) -> f64 {
    let (n, h) = (f64::from(n), f64::from(h));
    f64::from(c_ty) / (h * h * n * (n * n - 1.0) / 12.0)
}

/// The f32 unit roundoff, `2⁻²⁴`: a correctly rounded f32 `+ − ×` is within `U` of the exact result, relatively.
/// The bounds below take WGSL's stated accuracies (WGSL § "Floating Point Accuracy"), the looser target's: `x / y`
/// within 2.5 ULP (at most `5U` relative), `sqrt` inherited from `1 / inverseSqrt` (2 + 2.5 ULP, at most `9U`), and
/// `log` within an absolute `2⁻²¹` (`8U`) on [0.5, 2] and 3 ULP (`6U` relative) outside it. Rust's `/`, `sqrt` and
/// `ln` are within those, so a Rust read meets the same bound. A sum of `k` non-negative rounded terms, in any order,
/// is within `γ_k ≈ kU` (Higham, *Accuracy and Stability of Numerical Algorithms*, §3.1), so the GPU's reassociation
/// does not loosen them. The f64 reference's own error, ~1e-16, is covered by rounding each constant up.
const U: f64 = f32::EPSILON as f64 / 2.0;

/// The f32 `ftle`'s worst-case error against [`ftle_reference`]'s f64 value, derived from the read's operations:
/// - `δ²`: 12 differences and 12 squares, each term within `3U`, summed within `11U` more: `14U` relative;
/// - `δ = sqrt(δ²)`: half of `14U`, plus `sqrt`'s `9U`: `16U`; `δ / δ₀`: `5U` more, `21U`;
/// - `L = ln(δ/δ₀)`: its argument's `21U` becomes an absolute `22U`, plus `log`'s own, at most `8U + 6U·|L|`;
/// - `S + L`: `U·|S_final|` more, so `S_final` is within `30U + 6U·|L| + U·|S_final|` absolutely;
/// - `/ (f32(n) · dt)`: `f32(n)` is exact (`n < 2²⁴`), the product `U`, the division `5U`: `7U·|ftle|` with the
///   second-order terms, and `|ftle| = |S_final| / (n·dt)`.
///
/// Rounded up: `U·(32 + 8·|L| + 10·|S_final|) / (n·dt)`.
fn ftle_bound(l: f64, s_final: f64, n_dt: f64) -> f64 {
    U * (32.0 + 8.0 * l.abs() + 10.0 * s_final.abs()) / n_dt
}

/// The f32 `diffusion`'s worst-case error against [`diffusion_reference`]'s, relative: `f32(n)`, `n − 1` and `n + 1`
/// are exact (`n < 2²⁴`), the four products `h·h·m·(m−1)·(m+1)` within `U` each, the two divisions (`/ 12`,
/// `C_ty / C_tt`) within `5U` each: `γ_14`, rounded up to `15U`.
const DIFFUSION_REL: f64 = 15.0 * U;

/// Whether `got` is within `bound` of `want`.
fn close(got: f32, want: f64, bound: f64) -> bool {
    (f64::from(got) - want).abs() <= bound
}

// ── derived_not_stored (REQ-PAY-021) ──────────────────────────────────────────────────────────────────────────────

/// Each case's `ftle`, through `read`, against [`ftle_reference`].
fn check_ftle_reference(read: Read, cases: &[Case]) {
    for c in cases {
        let got = read(c);
        assert!(got.ftle_valid, "ftle_valid is false for {c:?}");
        let (want, bound) = ftle_reference(c);
        assert!(
            close(got.ftle, want, bound),
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
            c.masses = [0, 1, 2].map(|_| (0.05 + 0.95 * next()) as f32);
            c.state.E_0 = (8.0 * next() - 4.0) as f32;
            c.state.Lz_0 = (8.0 * next() - 4.0) as f32;
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
/// `ftle`, `ftle_valid` and the shadow agrees, and the variant without the shadow reads `ftle` and every component of
/// `r_sh` and `p_sh` as the canonical quiet NaN (lowering Part 3a; RQ-228).
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
    let shadow: Vec<u32> = [b.r_sh, b.p_sh]
        .iter()
        .flatten()
        .flatten()
        .map(|x| x.to_bits())
        .collect();
    assert!(
        shadow.iter().all(|&x| x == QNAN),
        "the tier without the shadow reads r_sh and p_sh {shadow:#010x?}, not the canonical quiet NaN"
    );
    let strip = |s: SimState| SimState {
        ftle: 0.0,
        ftle_valid: false,
        r_sh: [[0.0; 2]; 3],
        p_sh: [[0.0; 2]; 3],
        ..s
    };
    assert_eq!(
        strip(a),
        strip(b),
        "the tiers' reads differ beyond ftle and the shadow"
    );
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

#[test]
fn read_type_both_tiers_rust_shadow_off_reads_nan() {
    check_both_tiers(generated_read);
}

negative_control!(
    read_type_both_tiers_rust_shadow_off_reads_nan,
    "a base read that reads the shadow as zero must fail",
    expected = "reads r_sh and p_sh",
    check_both_tiers(|c| {
        let mut s = generated_read(c);
        if !c.ftle_variant {
            s.p_sh[2][1] = 0.0;
        }
        s
    })
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
            close(got.diffusion, want, DIFFUSION_REL * want.abs()),
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
            close(got, want, DIFFUSION_REL * want.abs()),
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
/// `derived_entry.wgsl`'s selectors, 0–25.
const MEMBERS: u32 = 26;

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

/// The checked-in WGSL, the full tier, as the harness binds it: its two buffers moved from group 1 to group 0's
/// bindings 2 (the state) and 3 (the word), after the entry point's selectors and arguments, then the entry point and
/// its output at binding 4. The binding numbers are `cargo xtask lint wgsl`'s to check; the reads are the generated
/// `sample_read`'s.
fn wgsl_module() -> String {
    let mut text = format!("{LAYER_WGSL}{READ_SIDE_WGSL}");
    for (from, buffer, to) in [(0, "simstate_buffer", 2), (1, "word_buffer", 3)] {
        let at = format!("@group(1) @binding({from}) var<storage, read> {buffer}");
        assert!(text.contains(&at), "the generated WGSL has no `{at}`");
        text = text.replace(
            &at,
            &format!("@group(0) @binding({to}) var<storage, read> {buffer}"),
        );
    }
    format!(
        "{text}\n{ENTRY_WGSL}\n@group(0) @binding(4) var<storage, read_write> t_out: array<u32>;\n"
    )
}

/// Each case's members 0–25 read on the GPU through the WGSL read side's `sample_read` at the full tier, the one the
/// checked-in files declare (R-343): every case is read from `SimStateFTLE` with the word bound.
fn wgsl_reads(gpu: &GpuHarness, cases: &[Case]) -> Vec<Vec<u32>> {
    let (mut sel, mut ftle, mut word, mut args) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, c) in (0u32..).zip(cases) {
        assert!(
            c.ftle_variant && c.has_word,
            "the checked-in WGSL is the full tier: {c:?}"
        );
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
        word.extend(c.word);
        let p = &c.params;
        args.extend([
            u32::from(c.has_ensemble) << 2,
            c.spread.to_bits(),
            p.dt_macro.to_bits(),
            p.delta_0.to_bits(),
        ]);
        args.extend([p.n_renorm, p.horizon_steps, 0, 0]);
        args.extend(c.masses.map(f32::to_bits));
        args.push(0);
    }
    let out = gpu.run_wgsl(&wgsl_module(), "t_derived", &[&sel, &args, &ftle, &word]);
    out.chunks(MEMBERS as usize).map(<[u32]>::to_vec).collect()
}

/// `s`'s members 0–25 in `derived_entry.wgsl`'s order, an f32 as its bits, a bool as 0 or 1.
fn rust_members(s: &SimState) -> [u32; 26] {
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
        s.energy_drift.to_bits(),
        s.Lz_drift.to_bits(),
    ]
}

/// The continuous members, compared within tolerance; every other member is compared bit for bit: an exact field, a
/// predicate, a sentinel (dd_simstate_payload §1, "Parity"). The drifts, members 24 and 25, are continuous too, held
/// to [`DRIFT_TOL`] against their terms' scale.
const CONTINUOUS: [usize; 4] = [0, 2, 5, 6];
const DRIFTS: [usize; 2] = [24, 25];

/// How far the Rust and WGSL reads of continuous member `k` (`a`, `g`) may differ: each is within its bound of the f64
/// reference, so the two are within the sum, twice the WGSL bound, which covers Rust's too ([`U`]). `ftle` (member 0)
/// and `diffusion` (member 2) take [`ftle_bound`] and [`DIFFUSION_REL`]; the fractions (members 5, 6) are one division
/// of exact operands (`t_*_step` and `horizon_steps` below 2²⁴), within `U` in Rust and `5U` in WGSL: `6U` of the exact
/// quotient, `7U` of the larger read with the second-order term.
fn continuous_bound(k: usize, c: &Case, a: f64, g: f64) -> f64 {
    match k {
        0 => 2.0 * ftle_reference(c).1,
        2 => {
            let n = tm_t_end_step(c.state.times);
            2.0 * DIFFUSION_REL * diffusion_reference(c.state.C_ty, n, c.params.dt_macro).abs()
        }
        _ => 7.0 * U * a.abs().max(g.abs()),
    }
}

/// Each case read through `read` and through the WGSL read side agree: exact members bit for bit, the continuous ones
/// within [`continuous_bound`], or bit for bit where either is the canonical NaN, and the drifts within [`DRIFT_TOL`] of
/// their terms' scale ([`drift_scales`]). `orbit_count` may differ by one at an exact multiple of 2π, which the cases
/// avoid.
fn check_parity(gpu: &GpuHarness, read: Read, cases: &[Case]) {
    for (c, gpu_m) in cases.iter().zip(wgsl_reads(gpu, cases)) {
        let cpu_m = rust_members(&read(c));
        let scales = drift_scales(c);
        for (k, (&a, &g)) in cpu_m.iter().zip(&gpu_m).enumerate() {
            let same = if let Some(d) = DRIFTS.iter().position(|&m| m == k) {
                let (a, g) = (f64::from(f32::from_bits(a)), f64::from(f32::from_bits(g)));
                (a - g).abs() <= DRIFT_TOL * scales[d]
            } else if CONTINUOUS.contains(&k) && a != QNAN && g != QNAN {
                let (a, g) = (f64::from(f32::from_bits(a)), f64::from(f32::from_bits(g)));
                (a - g).abs() <= continuous_bound(k, c, a, g)
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

/// Every random case at the full tier, the checked-in WGSL's, with the ensemble each bound and absent. The other tiers'
/// WGSL is the assembler's (`ledger::gen::read::assemble`), checked against the same references on the GPU in
/// `ledger/tests/derived.rs` as the Rust read is here.
fn parity_cases(seed: u64) -> Vec<Case> {
    random_cases(seed)
        .into_iter()
        .enumerate()
        .map(|(k, c)| Case {
            ftle_variant: true,
            has_word: true,
            has_ensemble: k % 2 != 0,
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

// ── current_drift (REQ-PAY-031) ───────────────────────────────────────────────────────────────────────────────────

/// The host reference of the current drifts' terms in f64, from the stored f32 values, in CoM-frame particle
/// coordinates with `G = 1` (integrator dd §3.5; decoder dd §3.6): `[K, V, L_z]`, `K = Σᵢ ‖pᵢ‖²/2mᵢ`,
/// `V = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, `L_z = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})`.
type Reference = fn(&[[f32; 2]; 3], &[[f32; 2]; 3], &[f32; 3]) -> [f64; 3];

fn drift_reference(r: &[[f32; 2]; 3], p: &[[f32; 2]; 3], m: &[f32; 3]) -> [f64; 3] {
    let f = |x: f32| f64::from(x);
    let k: f64 = (0..3)
        .map(|i| (f(p[i][0]).powi(2) + f(p[i][1]).powi(2)) / (2.0 * f(m[i])))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let d = (f(r[i][0]) - f(r[j][0])).hypot(f(r[i][1]) - f(r[j][1]));
            -f(m[i]) * f(m[j]) / d
        })
        .sum();
    let lz: f64 = (0..3)
        .map(|i| f(r[i][0]) * f(p[i][1]) - f(r[i][1]) * f(p[i][0]))
        .sum();
    [k, v, lz]
}

/// The drifts' tolerance, relative to the scale of the terms they are differences of: parity §4's class for the
/// monitored `E₀`/`L_z`, ~1e-6 relative. The drift is a cancellation (payload §5), so its error is the terms', not its
/// own magnitude's.
const DRIFT_TOL: f64 = 1e-6;

/// The terms' scales the drifts are held to: `|K| + |V| + |E_0|`, and `Σᵢ (|xᵢ p_{y,i}| + |yᵢ p_{x,i}|) + |Lz_0|`.
fn drift_scales(c: &Case) -> [f64; 2] {
    let s = &c.state;
    let [k, v, _] = drift_reference(&s.r, &s.p, &c.masses);
    let f = |x: f32| f64::from(x);
    let lz: f64 = (0..3)
        .map(|i| (f(s.r[i][0]) * f(s.p[i][1])).abs() + (f(s.r[i][1]) * f(s.p[i][0])).abs())
        .sum();
    [k.abs() + v.abs() + f(s.E_0).abs(), lz + f(s.Lz_0).abs()]
}

/// A case with masses `m`, positions `r`, momenta `p`, `E_0` and `Lz_0`, read from the FTLE variant or not; the shadow
/// sits 50 δ₀ from it along `r[0].x`, as [`marching`]'s does.
fn drift_case(
    m: [f32; 3],
    r: [[f32; 2]; 3],
    p: [[f32; 2]; 3],
    e_0: f32,
    lz_0: f32,
    ftle: bool,
) -> Case {
    let mut c = marching(40, 50.0);
    let mut r_sh = r;
    r_sh[0][0] += 50.0 * 1e-6;
    (c.state.r, c.state.p, c.state.r_sh, c.state.p_sh) = (r, p, r_sh, p);
    (c.state.E_0, c.state.Lz_0) = (e_0, lz_0);
    c.masses = m;
    c.ftle_variant = ftle;
    c
}

/// Hand-computed configurations, each from both variants, with its `(ΔE, ΔLz)`:
/// - equal unit masses at (1, 0), (−1, 0), (0, 0) with momenta (0, 1), (0, −1), 0: `K = ½ + ½ = 1`,
///   `V = −(1/2 + 1 + 1) = −5/2`, `H = −3/2`; `L_z = 1·1 + (−1)(−1) = 2`. Against `E_0 = −1`, `Lz_0 = 0.5`:
///   `ΔE = −1/2`, `ΔLz = 3/2`; against `E_0 = H`, `Lz_0 = L_z`: both 0.
/// - Burrau's problem (dd_predictability_horizon § "Units, so the numbers mean something": `G = 1`,
///   `m = (3, 4, 5)`, `E = −12.82`), at rest at (1, 3), (−2, −1), (1, −1), the CoM at the origin: the pair distances
///   5, 4 and 3 give `V = −(12/5 + 15/4 + 20/3) = −769/60`, `K = 0`, `L_z = 0`. Against `E_0 = 0`:
///   `ΔE = −769/60 ≈ −12.8167`.
/// - The Chenciner–Montgomery figure-eight (dd_validation_orbits §0: equal masses, `E = −1.2871419918`, `L_z = 0`
///   exactly), from its published initial condition, unit masses: `r₁ = −r₂ = (0.97000436, −0.24308753)`, `r₃ = 0`,
///   `p₃ = (−0.93240737, −0.86473146)`, `p₁ = p₂ = −p₃/2`. Against its own `E_0` and `Lz_0 = 0`: both drifts 0.
fn hand_cases() -> Vec<(Case, (f64, f64))> {
    const EIGHT_E: f32 = -1.287_141_991_8_f64 as f32;
    let unit = [1.0, 1.0, 1.0];
    let line = [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]];
    let line_p = [[0.0, 1.0], [0.0, -1.0], [0.0, 0.0]];
    let burrau = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]];
    let (x, y) = (0.970_004_36_f64 as f32, -0.243_087_53_f64 as f32);
    let (vx, vy) = (-0.932_407_37_f64 as f32, -0.864_731_46_f64 as f32);
    let eight = [[x, y], [-x, -y], [0.0, 0.0]];
    let eight_p = [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]];
    let mut out = Vec::new();
    for ftle in [true, false] {
        out.push((drift_case(unit, line, line_p, -1.0, 0.5, ftle), (-0.5, 1.5)));
        out.push((drift_case(unit, line, line_p, -1.5, 2.0, ftle), (0.0, 0.0)));
        out.push((
            drift_case([3.0, 4.0, 5.0], burrau, [[0.0; 2]; 3], 0.0, 0.0, ftle),
            (-769.0 / 60.0, 0.0),
        ));
        out.push((
            drift_case(unit, eight, eight_p, EIGHT_E, 0.0, ftle),
            (0.0, 0.0),
        ));
    }
    out
}

/// Each case's drifts through `read` are within [`DRIFT_TOL`] of the f64 reference's and, where given, of `want`.
fn check_drifts(read: Read, cases: &[(Case, Option<(f64, f64)>)]) {
    for (c, want) in cases {
        let got = read(c);
        let s = &c.state;
        let [k, v, lz] = drift_reference(&s.r, &s.p, &c.masses);
        let [e_scale, lz_scale] = drift_scales(c);
        let reference = (k + v - f64::from(s.E_0), lz - f64::from(s.Lz_0));
        for (name, (want_e, want_lz)) in [("the reference", Some(reference)), ("by hand", *want)]
            .into_iter()
            .filter_map(|(n, w)| Some((n, w?)))
        {
            assert!(
                (f64::from(got.energy_drift) - want_e).abs() <= DRIFT_TOL * e_scale,
                "energy_drift {} differs from H(r,p) − E_0 = {want_e} ({name}) for {c:?}",
                got.energy_drift
            );
            assert!(
                (f64::from(got.Lz_drift) - want_lz).abs() <= DRIFT_TOL * lz_scale,
                "Lz_drift {} differs from L_z(r,p) − Lz_0 = {want_lz} ({name}) for {c:?}",
                got.Lz_drift
            );
        }
    }
}

fn hand_drifts() -> Vec<(Case, Option<(f64, f64)>)> {
    hand_cases()
        .into_iter()
        .map(|(c, w)| (c, Some(w)))
        .collect()
}

#[test]
fn current_drift_rust_hand_computed_configurations() {
    check_drifts(generated_read, &hand_drifts());
}

negative_control!(
    current_drift_rust_hand_computed_configurations,
    "a read with the masses in reverse order must fail Burrau's energy",
    expected = "differs from H(r,p) − E_0",
    check_drifts(
        |c| {
            let [a, b, d] = c.masses;
            generated_read(&Case {
                masses: [d, b, a],
                ..*c
            })
        },
        &hand_drifts()
    )
);

/// The host reference gives each hand-computed configuration's drifts: the reference itself is right.
fn check_reference_by_hand(reference: Reference) {
    for (c, (want_e, want_lz)) in hand_cases() {
        let s = &c.state;
        let [k, v, lz] = reference(&s.r, &s.p, &c.masses);
        let (e, l) = (k + v - f64::from(s.E_0), lz - f64::from(s.Lz_0));
        assert!(
            (e - want_e).abs() <= 1e-6 && (l - want_lz).abs() <= 1e-6,
            "the reference gives ({e}, {l}), by hand ({want_e}, {want_lz}), for {c:?}"
        );
    }
}

#[test]
fn current_drift_rust_reference_matches_the_hand_values() {
    check_reference_by_hand(drift_reference);
}

negative_control!(
    current_drift_rust_reference_matches_the_hand_values,
    "a reference without the ½ in its kinetic energy must fail",
    expected = "the reference gives",
    check_reference_by_hand(|r, p, m| {
        let mut out = drift_reference(r, p, m);
        out[0] *= 2.0;
        out
    })
);

#[test]
fn current_drift_rust_property_matches_the_reference() {
    prop::run(&any::<u64>(), |seed| {
        let cases: Vec<_> = random_cases(seed).into_iter().map(|c| (c, None)).collect();
        check_drifts(generated_read, &cases);
        Ok(())
    });
}

negative_control!(
    current_drift_rust_property_matches_the_reference,
    "a read whose L_z drift has the wrong sign must fail the reference",
    expected = "differs from L_z(r,p) − Lz_0",
    check_drifts(
        |c| {
            let mut s = generated_read(c);
            s.Lz_drift = -s.Lz_drift;
            s
        },
        &random_cases(1)
            .into_iter()
            .map(|c| (c, None))
            .collect::<Vec<_>>()
    )
);

/// The hand-computed configurations and the random cases at the full tier, the checked-in WGSL's.
fn drift_parity_cases(seed: u64) -> Vec<Case> {
    let hand = hand_cases()
        .into_iter()
        .map(|(c, _)| c)
        .filter(|c| c.ftle_variant);
    hand.chain(parity_cases(seed)).collect()
}

#[test]
fn current_drift_rust_and_wgsl_agree() {
    let gpu = GpuHarness::new().expect("a GPU device");
    prop::run(&any::<u64>(), |seed| {
        check_parity(&gpu, generated_read, &drift_parity_cases(seed));
        Ok(())
    });
}

negative_control!(
    current_drift_rust_and_wgsl_agree,
    "a Rust read with the masses in reverse order must fail the parity",
    expected = "member 24: Rust",
    check_parity(
        &GpuHarness::new().expect("a GPU device"),
        |c| {
            let [a, b, d] = c.masses;
            generated_read(&Case {
                masses: [d, b, a],
                ..*c
            })
        },
        &drift_parity_cases(1)
    )
);
