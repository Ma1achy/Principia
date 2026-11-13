//! QA's tests for TASK-M3-10, written from the requirements it closes (payload §1, "Why closure is here, at this
//! width, unconditionally"; payload §2, "Drift latches are ABSOLUTE maxima"; R-271):
//! - `qa_layout_read_with_unpack2x16float` (REQ-PAY-041): `d_min` is the high half of `packed_a`, `dE_max` the low and
//!   `dLz_max` the high half of `packed_b`, read with WGSL's `unpack2x16float`, the descriptor's low 16 bits kept.
//! - `qa_clamp_fuzz_any_bits` (REQ-PAY-045): over every f32 bit pattern (NaN payloads, ±∞, ±10⁶, subnormals), a valid
//!   state's halves are finite, clamped to 65504, the drift maxima never negative and `d_min` never 0.0 (R-271); a failed
//!   state's halves are exactly `0x0000` in both drifts and `0x7C00` in `d_min`, tested by the bits.
//! - `qa_failed_run_packs_failed_state_values` (REQ-PAY-041, REQ-PAY-045): a march that fails at f32 (momenta whose
//!   squares overflow) stores `sim_failed` with `0x7C00` in `d_min` and 0.0 in both drift halves, though its detector
//!   latched a finite separation.
//! - `qa_reload_persist_is_exact` (REQ-PAY-044): every word a valid sample can persist, reloaded at a resume and
//!   persisted again with nothing new observed, is the same word, exhaustively over the f16 halves: one requantisation
//!   per persist boundary, none added by the resume.
//! - `qa_latches_monotone_across_persists` (REQ-PAY-044, REQ-PAY-050): over random measurements and persist cuts, the
//!   live f32 latches never move the wrong way after any observation, each bounds the observation just made, the
//!   persisted halves never move the wrong way from one persist boundary to the next, and the last persist is the f16 of
//!   the offline least separation and greatest drift magnitudes.
//!
//! **Domain** (pitfalls §10): the latch writer and its resume on synthetic measurements, and one march at f32 for the
//! failure path. The latches' agreement with a real trajectory is `tests/latch.rs`' `latches_vs_offline`.

use kernel::driver::latch::Latches;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{
    f32_to_f16_bits, pa_d_min, pb_dE_max, pb_dLz_max, sd_state, set_state, unpack2x16float,
    DminCounters, F16_FINITE_MAX, F16_MIN_SUBNORMAL, STATE_BOUNDED, STATE_COLLISION,
    STATE_DECODE_FAILED, STATE_ESCAPE, STATE_RUNNING, STATE_SIM_FAILED,
};
use proptest::prelude::*;
use validation::drift::{integrate_fixture, Fixture};
use validation::{negative_control, prop};

/// A latch writer: latches and the `packed_a` it keeps the descriptor of, in; `(packed_a, packed_b)` out.
type Persist = fn(&Latches, u32) -> (u32, u32);

/// A resume: the latches a sample continues from, given its persisted `(packed_a, packed_b)`.
type Resume = fn(u32, u32) -> Latches;

/// The writer under test, with a frame's counters of its own.
fn persist(l: &Latches, a: u32) -> (u32, u32) {
    l.persist(a, &DminCounters::new())
}

/// The resume under test.
fn reload(a: u32, b: u32) -> Latches {
    Latches::reload(a, b)
}

/// A descriptor with every used low bit set apart from `state` (`detail` 3, `saturated`, `dmin_pair` 3,
/// `last_symbol` 3), so a writer that touches the low half is caught.
fn descriptor(state: u32) -> u32 {
    set_state(0x03f8, state)
}

/// The states whose latches are untrusted: `sim_failed`, `decode_failed` and the reserved codes (payload §2).
fn is_failed(state: u32) -> bool {
    !matches!(
        state,
        STATE_ESCAPE | STATE_BOUNDED | STATE_COLLISION | STATE_RUNNING
    )
}

/// Latches that met the separations `ds` (each taken as `d²`) and the drifts `drifts`.
fn observed(ds: &[f32], drifts: &[(f32, f32)]) -> Latches {
    let mut l = Latches::unset();
    for &d in ds {
        l.separation_squared(d * d);
    }
    for &(de, dlz) in drifts {
        l.drift(de, dlz);
    }
    l
}

// ── qa_layout_read_with_unpack2x16float (REQ-PAY-041) ────────────────────────────────────────────────────────────

/// Asserts the bit layout `pack` writes: `d_min = 0.5` (`0x3800`) in `packed_a`'s high half, `dE_max = |−0.25|`
/// (`0x3400`) in `packed_b`'s low half and `dLz_max = 3` (`0x4200`) in its high half, each read back by
/// `unpack2x16float`, the descriptor kept.
fn check_layout(pack: Persist) {
    let l = observed(&[2.0, 0.5, 1.0], &[(-0.25, 1.0), (0.125, -3.0)]);
    let a0 = descriptor(STATE_BOUNDED);
    let (a, b) = pack(&l, a0);
    assert_eq!(a & 0xffff, a0, "the descriptor half is not kept");
    assert_eq!(
        (a >> 16, b),
        (0x3800, 0x4200_3400),
        "the halves are not d_min high in packed_a, dE_max low and dLz_max high in packed_b"
    );
    assert_eq!(
        (unpack2x16float(a)[1], unpack2x16float(b)),
        (0.5, [0.25, 3.0]),
        "unpack2x16float does not read the latches"
    );
}

#[test]
fn qa_layout_read_with_unpack2x16float() {
    check_layout(persist);
}

negative_control!(
    qa_layout_read_with_unpack2x16float,
    "a writer that swaps dE_max and dLz_max puts them in the wrong halves",
    expected = "the halves are not",
    check_layout(|l, a| {
        let (a, b) = persist(l, a);
        (a, b.rotate_left(16))
    })
);

// ── qa_clamp_fuzz_any_bits (REQ-PAY-045) ─────────────────────────────────────────────────────────────────────────

/// Asserts the clamp and failed-state rule of `pack` on the latches of the observations `obs` (`(d, ΔE, ΔL_z)`) in
/// `state`.
fn check_clamp(pack: Persist, state: u32, obs: &[(f32, f32, f32)]) -> Result<(), TestCaseError> {
    let mut l = Latches::unset();
    for &(d, de, dlz) in obs {
        l.separation_squared(d * d);
        l.drift(de, dlz);
    }
    let (a, b) = pack(&l, descriptor(state));
    let (hd, he, hl) = ((a >> 16) as u16, b as u16, (b >> 16) as u16);
    prop_assert_eq!(a & 0xffff, descriptor(state), "the descriptor is not kept");
    if is_failed(state) {
        prop_assert_eq!(
            (hd, he, hl),
            (0x7c00, 0x0000, 0x0000),
            "state {}: the failed-state bits are not 0x7C00, 0.0, 0.0",
            state
        );
        return Ok(());
    }
    let met = obs.iter().any(|o| !(o.0 * o.0).is_nan());
    let finite = |h: u16| h & 0x7c00 != 0x7c00;
    prop_assert!(
        finite(he) && finite(hl) && (finite(hd) || (!met && hd == 0x7c00)),
        "state {}: a half of {:04x}/{:04x}/{:04x} is not finite (d_min unset only with no separation met)",
        state,
        hd,
        he,
        hl
    );
    prop_assert!(
        he & 0x8000 == 0 && hl & 0x8000 == 0 && hd & 0x8000 == 0,
        "state {}: a latch half is negative: {:04x}/{:04x}/{:04x}",
        state,
        hd,
        he,
        hl
    );
    prop_assert!(
        hd != 0x0000,
        "state {}: d_min is 0.0, which never appears (R-271)",
        state
    );
    let unpacked = [pa_d_min(a), pb_dE_max(b), pb_dLz_max(b)];
    let set = usize::from(hd == 0x7c00);
    prop_assert!(
        unpacked[set..].iter().all(|&x| x <= F16_FINITE_MAX),
        "state {}: an unpacked half {:?} exceeds 65504",
        state,
        unpacked
    );
    // A column with a NaN in it is a march that marks itself `sim_failed` (payload §1), outside this check's domain.
    let big = |x: f32| x.abs() >= F16_FINITE_MAX;
    let max_de = obs.iter().any(|o| big(o.1)) && !obs.iter().any(|o| o.1.is_nan());
    let max_dlz = obs.iter().any(|o| big(o.2)) && !obs.iter().any(|o| o.2.is_nan());
    prop_assert!(
        (!max_de || he == 0x7bff) && (!max_dlz || hl == 0x7bff),
        "state {}: a drift of magnitude at least 65504 (or ±∞) is not stored as 65504: {:04x}/{:04x}",
        state,
        he,
        hl
    );
    Ok(())
}

/// The clamp property over `pack`: any state code, one to eight observations of any f32 bit patterns, with the edges
/// ±10⁶, ±∞ and NaN drawn often.
fn clamp_fuzz(pack: Persist) {
    let value = prop_oneof![
        proptest::sample::select(vec![
            1e6f32,
            -1e6,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            65520.0,
            -65520.0,
            0.0,
            -0.0,
        ]),
        any::<u32>().prop_map(f32::from_bits),
    ];
    let obs = proptest::collection::vec((value.clone(), value.clone(), value), 1..8);
    prop::run(&(0u32..8, obs), |(state, obs)| {
        check_clamp(pack, state, &obs)
    });
}

#[test]
fn qa_clamp_fuzz_any_bits() {
    clamp_fuzz(persist);
}

negative_control!(
    qa_clamp_fuzz_any_bits,
    "a writer that packs a failed state's latches as a valid state's leaves them in place",
    expected = "the failed-state bits are not",
    clamp_fuzz(|l, a| {
        let (p, b) = persist(l, set_state(a, STATE_RUNNING));
        ((p & 0xffff_0000) | (a & 0xffff), b)
    })
);

// ── qa_failed_run_packs_failed_state_values (REQ-PAY-041, REQ-PAY-045) ────────────────────────────────────────────

/// A march whose energy is non-finite at f32: momenta whose squares overflow, so the monitor marks it `sim_failed`.
fn overflowing() -> Fixture {
    Fixture {
        name: "momenta whose squares overflow at f32",
        ic: kernel::decode::Ic {
            m: [0.25, 0.25, 0.5],
            r: [[-0.5, -0.25], [0.5, -0.25], [0.0, 0.25]],
            p: [[3e19, 0.0], [-3e19, 0.0], [0.0, 0.0]],
        },
        horizon: 0.002,
    }
}

/// Asserts that the stored `(state, packed_a, packed_b)` of the failed march are `sim_failed`, `0x7C00` and 0.0.
fn check_failed_run((state, a, b): (u32, u32, u32)) {
    assert_eq!(
        state, STATE_SIM_FAILED,
        "the overflowing march is not sim_failed"
    );
    assert_eq!(
        ((a >> 16) as u16, b),
        (0x7c00, 0),
        "the failed march's d_min is not 0x7C00 or its drift halves are not 0.0"
    );
}

#[test]
fn qa_failed_run_packs_failed_state_values() {
    let (out, cadence) = integrate_fixture::<Kdk, f32>(&overflowing());
    let d2 = cadence.detector().latch().d2();
    assert!(
        d2.is_finite(),
        "the detector latched no finite separation, so the failure path is not what is tested"
    );
    check_failed_run((sd_state(out.packed_a), out.packed_a, out.packed_b));
    let l = observed(&[0.5], &[(1e6, -2.0)]);
    let (a, b) = persist(&l, descriptor(STATE_DECODE_FAILED));
    assert_eq!(
        ((a >> 16) as u16, b),
        (0x7c00, 0),
        "a decode_failed sample's d_min is not 0x7C00 or its drift halves are not 0.0"
    );
}

negative_control!(
    qa_failed_run_packs_failed_state_values,
    "the failed march's latches packed as a running sample's store its separation and drifts",
    expected = "d_min is not 0x7C00",
    {
        let (out, cadence) = integrate_fixture::<Kdk, f32>(&overflowing());
        let m = cadence.monitor_state();
        let mut l = Latches::unset();
        l.separation_squared(cadence.detector().latch().d2());
        l.drift(m.de_max(), m.dlz_max());
        let (a, b) = persist(&l, set_state(out.packed_a, STATE_RUNNING));
        check_failed_run((sd_state(out.packed_a), a, b));
    }
);

// ── qa_reload_persist_is_exact (REQ-PAY-044) ─────────────────────────────────────────────────────────────────────

/// Asserts that `resume` then [`persist`] gives back every persisted word of a valid sample: each `d_min` half from the
/// least subnormal to 65504 and the unset `0x7C00`, and each drift half from 0.0 to 65504, in both halves.
fn check_reload(resume: Resume) {
    for state in [STATE_RUNNING, STATE_BOUNDED, STATE_COLLISION, STATE_ESCAPE] {
        let a0 = descriptor(state);
        for h in 0x0001u32..=0x7c00 {
            let words = (a0 | (h << 16), 0x3c00_3c00);
            assert_eq!(
                persist(&resume(words.0, words.1), words.0),
                words,
                "state {state}: d_min bits {h:04x} change across a resume"
            );
        }
        for h in 0x0000u32..=0x7bff {
            let words = (a0 | (0x3c00 << 16), h | ((0x7bff - h) << 16));
            assert_eq!(
                persist(&resume(words.0, words.1), words.0),
                words,
                "state {state}: drift bits {:08x} change across a resume",
                words.1
            );
        }
    }
}

#[test]
fn qa_reload_persist_is_exact() {
    check_reload(reload);
}

negative_control!(
    qa_reload_persist_is_exact,
    "a resume that takes d_min's value as its square requantises it",
    expected = "change across a resume",
    check_reload(|a, b| {
        let mut l = Latches::unset();
        if (a >> 16) != 0x7c00 {
            l.separation_squared(pa_d_min(a));
        }
        l.drift(pb_dE_max(b), pb_dLz_max(b));
        l
    })
);

// ── qa_latches_monotone_across_persists (REQ-PAY-044, REQ-PAY-050) ───────────────────────────────────────────────

/// The f16 bits a valid sample stores for a `d_min` of `d`: clamped to 65504, below the least subnormal floored to it.
fn d_min_bits(d: f32) -> u32 {
    if d < F16_MIN_SUBNORMAL {
        0x0001
    } else {
        u32::from(f32_to_f16_bits(d.min(F16_FINITE_MAX)))
    }
}

/// The f16 bits a valid sample stores for a drift maximum `x >= 0`, clamped to 65504.
fn drift_bits(x: f32) -> u32 {
    u32::from(f32_to_f16_bits(x.min(F16_FINITE_MAX)))
}

/// Asserts, for a `running` sample observing `obs` (`(d, ΔE, ΔL_z)`, finite) and persisting after each index in
/// `cuts` and at the end, resumed by `resume` at each boundary: the live latches never move the wrong way and bound
/// each observation; the persisted halves never move the wrong way between boundaries; the last persist is the f16 of
/// the offline extremes.
fn check_monotone(
    resume: Resume,
    obs: &[(f32, f32, f32)],
    cuts: &[usize],
) -> Result<(), TestCaseError> {
    let a0 = descriptor(STATE_RUNNING);
    let mut words = persist(&Latches::unset(), a0);
    let mut prev = (pa_d_min(words.0), pb_dE_max(words.1), pb_dLz_max(words.1));
    let mut start = 0;
    for &end in cuts.iter().chain(std::iter::once(&obs.len())) {
        let mut l = resume(words.0, words.1);
        let mut live = (l.d_min(), l.de_max(), l.dlz_max());
        for &(d, de, dlz) in &obs[start..end] {
            l.separation_squared(d * d);
            l.drift(de, dlz);
            let now = (l.d_min(), l.de_max(), l.dlz_max());
            prop_assert!(
                now.0 <= live.0 && now.1 >= live.1 && now.2 >= live.2,
                "the live latches moved the wrong way: {:?} after {:?}",
                now,
                live
            );
            prop_assert!(
                now.0 <= (d * d).sqrt() && now.1 >= de.abs() && now.2 >= dlz.abs(),
                "the live latches {:?} do not bound the observation {:?}",
                now,
                (d, de, dlz)
            );
            live = now;
        }
        words = persist(&l, words.0);
        let stored = (pa_d_min(words.0), pb_dE_max(words.1), pb_dLz_max(words.1));
        prop_assert!(
            stored.0 <= prev.0 && stored.1 >= prev.1 && stored.2 >= prev.2,
            "the persisted latches moved the wrong way at {}: {:?} after {:?}",
            end,
            stored,
            prev
        );
        prev = stored;
        start = end;
    }
    let d2 = obs.iter().map(|o| o.0 * o.0).fold(f32::INFINITY, f32::min);
    let de = obs.iter().map(|o| o.1.abs()).fold(0.0f32, f32::max);
    let dlz = obs.iter().map(|o| o.2.abs()).fold(0.0f32, f32::max);
    let want = (
        d_min_bits(d2.sqrt()),
        drift_bits(de) | (drift_bits(dlz) << 16),
    );
    prop_assert_eq!(
        (words.0 >> 16, words.1),
        want,
        "the last persist is not the f16 of the offline extremes"
    );
    Ok(())
}

/// A finite measurement, either sign, spread from 2⁻³⁰ to past 65504.
fn measure() -> impl Strategy<Value = f32> {
    (any::<bool>(), -30i32..20, 1.0f32..2.0).prop_map(|(neg, e, m)| {
        let x = m * 2f32.powi(e);
        if neg {
            -x
        } else {
            x
        }
    })
}

/// The monotone property over `resume`: one to 48 observations, cut at any subset of their indices.
fn monotone(resume: Resume) {
    let cases =
        proptest::collection::vec((measure(), measure(), measure()), 1..48).prop_flat_map(|o| {
            let n = o.len();
            (Just(o), proptest::collection::btree_set(0..=n, 0..n + 1))
        });
    prop::run(&cases, |(obs, cuts)| {
        let cuts: Vec<usize> = cuts.into_iter().collect();
        check_monotone(resume, &obs, &cuts)
    });
}

#[test]
fn qa_latches_monotone_across_persists() {
    monotone(reload);
}

negative_control!(
    qa_latches_monotone_across_persists,
    "a resume that starts afresh lets the persisted d_min rise",
    expected = "moved the wrong way",
    {
        let obs = [(1.0f32, 2.0f32, 2.0f32), (3.0, 0.5, 0.5)];
        if let Err(e) = check_monotone(|_, _| Latches::unset(), &obs, &[1]) {
            panic!("{e}");
        }
    }
);
