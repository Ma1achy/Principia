//! QA tests for TASK-M0-10's R-281/R-288 clauses of REQ-PAY-092, written from the requirement's statement and verify
//! detail, R-281, R-288 and telemetry §2, not from the implementation:
//!
//! - through the release path a NaN `d_min` stores the unset bits `0x7C00` and increments `dmin_nan_unset`; a negative
//!   `d_min` stores the floor `0x0001` and increments `dmin_negative_floored`; each case increments its own counter
//!   once and the other not at all, and a valid value increments neither;
//! - the counters are atomic u32s (telemetry §2), so concurrent packers lose no count;
//! - in a release build the crate-level `set_d_min` and the word packer `pack_packed_a` count into the kernel's
//!   `DMIN_COUNTERS` ("counted in release builds too; that's their purpose", R-288);
//! - `roundtrip_ctl`'s repack is an observation, not a store: it counts nothing (R-288's applied note).
//!
//! "Negative" is IEEE 754's: a value that compares below zero. −0.0 compares equal to +0.0, so it is a zero distance,
//! stored as the floor by R-271 and counted by neither. Each test has a registered negative control (R-176).

use kernel::payload::roundtrip::{roundtrip_ctl, PackedA};
use kernel::payload::*;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use validation::negative_control;

/// The tests in this binary that read or write the crate-level `DMIN_COUNTERS` hold this, so their deltas are theirs.
static CRATE_PAIR_LOCK: Mutex<()> = Mutex::new(());

/// `(dmin_nan_unset, dmin_negative_floored)`.
fn pair(c: &DminCounters) -> (u32, u32) {
    (
        c.dmin_nan_unset.load(Ordering::SeqCst),
        c.dmin_negative_floored.load(Ordering::SeqCst),
    )
}

/// The `d_min` half of a `packed_a` word (payload §2: bits 16–31).
fn d_min_half(w: u32) -> u32 {
    w >> 16
}

/// What R-271/R-281 require for an f32 input: `(stored f16 bits, counts it adds)`, from the IEEE classification of
/// the input alone. Only the classes this file's inputs use are listed; a valid value's stored bits are given where
/// they are fixed by R-271 (floor, unset) or exact in binary16.
fn required(bits: u32) -> (u32, (u32, u32)) {
    let v = f32::from_bits(bits);
    if v.is_nan() {
        (0x7c00, (1, 0))
    } else if v < 0.0 {
        (0x0001, (0, 1))
    } else if bits == 0x7f80_0000 {
        (0x7c00, (0, 0))
    } else if v < 2f32.powi(-24) {
        (0x0001, (0, 0))
    } else if v == 1.0 {
        (0x3c00, (0, 0))
    } else if v == 0.5 {
        (0x3800, (0, 0))
    } else if v >= 65504.0 {
        (0x7bff, (0, 0))
    } else {
        panic!("input {bits:#010x} has no listed requirement")
    }
}

/// A mixed stream: NaNs and negatives interleaved with valid values, so a packer counting the wrong case, counting
/// twice, or counting a valid value is caught at the step it does so.
const STREAM: [u32; 16] = [
    0x3f80_0000, // 1.0
    0x7fc0_0000, // quiet NaN
    0xbf80_0000, // −1.0
    0x8000_0000, // −0.0: a zero distance
    0x0000_0000, // +0.0
    0xff80_0000, // −∞
    0x7f80_0000, // +∞: the unset value itself
    0x7f80_0001, // signalling NaN
    0x8000_0001, // −(smallest f32 subnormal)
    0x3100_0000, // 2⁻²⁹, below the f16 floor
    0x3f00_0000, // 0.5
    0xffc0_0000, // negative quiet NaN
    0x4974_2400, // 1e6, past 65504
    0xc77f_e000, // −65504
    0x7f7f_ffff, // f32::MAX
    0xff7f_ffff, // −f32::MAX
];

type Counted = fn(u32, f32, &DminCounters) -> u32;

/// Over one fresh pair, `set` stores what [`required`] says for each input of [`STREAM`], keeps the descriptor half,
/// and after every input the pair equals the running tally.
fn check_stream(set: Counted) {
    let c = DminCounters::new();
    assert_eq!(pair(&c), (0, 0), "a new pair starts at zero");
    let mut tally = (0u32, 0u32);
    for (i, &bits) in STREAM.iter().enumerate() {
        let (want, add) = required(bits);
        let low = [0x0000u32, 0x03ff, 0xffff][i % 3];
        let w = set(0x7bff_0000 | low, f32::from_bits(bits), &c);
        assert_eq!(d_min_half(w), want, "input {bits:#010x}: stored d_min bits");
        assert_eq!(w & 0xffff, low, "input {bits:#010x}: descriptor half kept");
        tally = (tally.0 + add.0, tally.1 + add.1);
        assert_eq!(
            pair(&c),
            tally,
            "after input {i} ({bits:#010x}): (dmin_nan_unset, dmin_negative_floored)"
        );
    }
    assert_eq!(
        tally,
        (3, 5),
        "the stream holds three NaNs and five negatives"
    );
}

#[test]
fn dmin_unset_qa_r288_release_path_stores_and_counts_each_case() {
    check_stream(set_d_min_release);
}

/// A release packer that stores the right bits but never counts (its pair is a throwaway).
#[cfg(feature = "controls")]
fn stores_only(w: u32, v: f32, _: &DminCounters) -> u32 {
    set_d_min_release(w, v, &DminCounters::new())
}

negative_control!(
    dmin_unset_qa_r288_release_path_stores_and_counts_each_case,
    "a packer that stores 0x7C00/0x0001 but increments nothing, so the running tally check must fail",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_stream(stores_only)
);

// A second control for the same test: counting −0.0 as a negative value (a sign-bit test instead of a comparison).
negative_control!(
    dmin_unset_qa_r288_release_path_negative_zero_is_not_counted,
    "a packer that counts every sign-bit-set non-NaN input, −0.0 included, so the tally check must fail at −0.0",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_stream(|w, v, c| {
        let out = set_d_min_release(w, v, &DminCounters::new());
        if v.is_nan() {
            c.dmin_nan_unset.fetch_add(1, Ordering::SeqCst);
        } else if v.is_sign_negative() {
            c.dmin_negative_floored.fetch_add(1, Ordering::SeqCst);
        }
        out
    })
);

/// In a release build `set_d_min_counted` has no assertion and counts into the pair it is given, exactly as the
/// release packer does. In a debug build it asserts first, so only its valid inputs are exercised here.
fn check_counted_matches_release(set: Counted) {
    #[cfg(not(debug_assertions))]
    check_stream(set);
    #[cfg(debug_assertions)]
    {
        let c = DminCounters::new();
        for &bits in STREAM.iter().filter(|&&b| required(b).1 == (0, 0)) {
            let w = set(0, f32::from_bits(bits), &c);
            assert_eq!(
                d_min_half(w),
                required(bits).0,
                "input {bits:#010x}: stored d_min bits"
            );
        }
        assert_eq!(
            pair(&c),
            (0, 0),
            "valid inputs: (dmin_nan_unset, dmin_negative_floored)"
        );
    }
}

#[test]
fn dmin_unset_qa_r288_counted_packer_counts_into_its_pair() {
    check_counted_matches_release(set_d_min_counted);
}

negative_control!(
    dmin_unset_qa_r288_counted_packer_counts_into_its_pair,
    "a packer that counts every store as a NaN, so the pair check must fail",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_counted_matches_release(|w, v, c| {
        c.dmin_nan_unset.fetch_add(1, Ordering::SeqCst);
        set_d_min_release(w, v, &DminCounters::new())
    })
);

/// Threads sharing one pair, each storing `per_thread` NaNs and as many negatives through `set`, leave exactly
/// `threads × per_thread` in each counter: an atomic u32 loses no increment (telemetry §2).
fn check_concurrent(set: Counted, pair_under_test: &'static DminCounters) {
    const THREADS: u32 = 8;
    const PER_THREAD: u32 = 5_000;
    let before = pair(pair_under_test);
    std::thread::scope(|s| {
        for t in 0..THREADS {
            s.spawn(move || {
                for i in 0..PER_THREAD {
                    let nan = f32::from_bits(0x7fc0_0000 | (t << 8) | (i & 0xff));
                    let neg = -1.0 - i as f32;
                    assert_eq!(d_min_half(set(0, nan, pair_under_test)), 0x7c00);
                    assert_eq!(d_min_half(set(0, neg, pair_under_test)), 0x0001);
                }
            });
        }
    });
    let after = pair(pair_under_test);
    assert_eq!(
        (after.0 - before.0, after.1 - before.1),
        (THREADS * PER_THREAD, THREADS * PER_THREAD),
        "concurrent stores: (dmin_nan_unset, dmin_negative_floored)"
    );
}

static SHARED: DminCounters = DminCounters::new();

#[test]
fn dmin_unset_qa_r288_counters_lose_no_concurrent_count() {
    check_concurrent(set_d_min_release, &SHARED);
}

#[cfg(feature = "controls")]
static SHARED_CONTROL: DminCounters = DminCounters::new();

negative_control!(
    dmin_unset_qa_r288_counters_lose_no_concurrent_count,
    "a packer that drops every NaN count on the odd inputs, so the concurrent total must fall short",
    expected = "concurrent stores: (dmin_nan_unset, dmin_negative_floored)",
    check_concurrent(
        |w, v, c| {
            let throwaway = DminCounters::new();
            let keep = !v.is_nan() || v.to_bits() & 1 == 0;
            set_d_min_release(w, v, if keep { c } else { &throwaway })
        },
        &SHARED_CONTROL
    )
);

/// `roundtrip_ctl` fails every word whose `d_min` bits hold a value the packer would count (f16 NaNs, negatives) and
/// leaves the crate-level pair as it found it; `bump` runs once per word in place of nothing (the control's hook).
fn check_repack_counts_nothing(bump: fn(f32)) {
    let _guard = CRATE_PAIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let good = pack_packed_a(1, 0, false, 2, 1, 0.5);
    let expected = PackedA::unpack(good);
    let before = pair(&DMIN_COUNTERS);
    for h in [
        0x7c01u32, 0x7e00, 0xfe00, 0xffff, 0x8001, 0xbc00, 0xfbff, 0xfc00,
    ] {
        let observed = (h << 16) | (good & 0xffff);
        assert!(
            !roundtrip_ctl(&expected, observed),
            "roundtrip_ctl passed a contaminated d_min {h:#06x}"
        );
        bump(f16_bits_to_f32(h as u16));
    }
    assert!(
        roundtrip_ctl(&expected, good),
        "roundtrip_ctl failed the clean word"
    );
    assert_eq!(
        pair(&DMIN_COUNTERS),
        before,
        "roundtrip_ctl: DMIN_COUNTERS (dmin_nan_unset, dmin_negative_floored)"
    );
}

#[test]
fn dmin_unset_qa_r288_roundtrip_repack_counts_nothing() {
    check_repack_counts_nothing(|_| {});
}

negative_control!(
    dmin_unset_qa_r288_roundtrip_repack_counts_nothing,
    "a repack that stores each observed d_min through the crate-level pair, so the unchanged-pair check must fail",
    expected = "roundtrip_ctl: DMIN_COUNTERS",
    check_repack_counts_nothing(|v| {
        set_d_min_release(0, v, &DMIN_COUNTERS);
    })
);

/// Release builds only: `store` (a `d_min` store with no pair argument) counts a NaN and a negative value into the
/// crate-level `DMIN_COUNTERS`, one each, storing `0x7C00` and `0x0001`.
#[cfg(not(debug_assertions))]
fn check_crate_pair_counts(store: fn(f32) -> u32) {
    let _guard = CRATE_PAIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let before = pair(&DMIN_COUNTERS);
    assert_eq!(
        d_min_half(store(f32::NAN)),
        0x7c00,
        "NaN: stored d_min bits"
    );
    assert_eq!(d_min_half(store(-2.0)), 0x0001, "−2: stored d_min bits");
    assert_eq!(d_min_half(store(1.0)), 0x3c00, "1: stored d_min bits");
    let after = pair(&DMIN_COUNTERS);
    assert_eq!(
        (after.0 - before.0, after.1 - before.1),
        (1, 1),
        "release store: DMIN_COUNTERS (dmin_nan_unset, dmin_negative_floored)"
    );
}

#[cfg(not(debug_assertions))]
#[test]
fn dmin_unset_qa_r288_release_set_d_min_and_pack_count_into_the_crate_pair() {
    check_crate_pair_counts(|v| set_d_min(0x0000_03ff, v));
    check_crate_pair_counts(|v| pack_packed_a(4, 1, false, 3, 0, v));
}

#[cfg(not(debug_assertions))]
negative_control!(
    dmin_unset_qa_r288_release_set_d_min_and_pack_count_into_the_crate_pair,
    "a store that counts into a throwaway pair, so the crate-level delta check must fail",
    expected = "release store: DMIN_COUNTERS",
    check_crate_pair_counts(|v| set_d_min_release(0, v, &DminCounters::new()))
);

/// Debug builds: the crate-level store asserts before it counts nothing into `DMIN_COUNTERS` for a valid value.
#[test]
fn dmin_unset_qa_r288_valid_store_leaves_the_crate_pair() {
    check_valid_leaves_crate_pair(|v| set_d_min(0, v));
}

fn check_valid_leaves_crate_pair(store: fn(f32) -> u32) {
    let _guard = CRATE_PAIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let before = pair(&DMIN_COUNTERS);
    for bits in STREAM.iter().copied().filter(|&b| required(b).1 == (0, 0)) {
        assert_eq!(
            d_min_half(store(f32::from_bits(bits))),
            required(bits).0,
            "input {bits:#010x}: stored d_min bits"
        );
    }
    assert_eq!(
        pair(&DMIN_COUNTERS),
        before,
        "valid store: DMIN_COUNTERS (dmin_nan_unset, dmin_negative_floored)"
    );
}

negative_control!(
    dmin_unset_qa_r288_valid_store_leaves_the_crate_pair,
    "a store that counts every value as negative into the crate-level pair, so the unchanged check must fail",
    expected = "valid store: DMIN_COUNTERS",
    check_valid_leaves_crate_pair(|v| {
        DMIN_COUNTERS
            .dmin_negative_floored
            .fetch_add(1, Ordering::SeqCst);
        set_d_min(0, v)
    })
);
