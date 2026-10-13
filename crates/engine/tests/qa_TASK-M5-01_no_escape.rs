//! QA tests for TASK-M5-01, R-418 on the engine side: the generated Rust `QuadReduction` accessors and sentinels of
//! `escape_time_min` and `escape_time_max`. Written from R-418, REQ-REF-006 and dd_generation_root §3.7 ("No escape
//! (R-418)", "Layout and §3.8 entries"), not from the implementation:
//! - with no copy escaped, `escape_time_min` holds +∞ (binary16 `0x7c00`, bits 0–15 of `qr_w7`) and `escape_time_max`
//!   −∞ (`0xfc00`, bits 16–31), each its reduction's identity, so the min and max reductions over a quad's escaped
//!   copies need no branch: folding any set of recorded times from them gives the set's min and max;
//! - "no escape" reads `_min > _max`; a quad with any escaped copy never does (its `_min` ≤ `_max`);
//! - an escape time past f16's range saturates to 65504, so no recorded time is either identity, and the generated
//!   sentinel tests fire on the identities' bits only.
//!
//! Each test has a registered negative control (R-176).

use engine::quad::*;
use validation::negative_control;

/// binary16 `h`'s value by IEEE 754 (sign, 5-bit exponent biased by 15, 10-bit significand), written here from the
/// definition.
fn half(h: u16) -> f64 {
    let s = if h >> 15 == 1 { -1.0 } else { 1.0 };
    let e = i32::from((h >> 10) & 0x1f);
    let m = f64::from(h & 0x3ff);
    s * match (e, m == 0.0) {
        (0, _) => m * 2f64.powi(-24),
        (31, true) => f64::INFINITY,
        (31, false) => f64::NAN,
        _ => (1.0 + m / 1024.0) * 2f64.powi(e - 15),
    }
}

/// Recorded escape times, binary16 bits, each finite and in `[0, ∞)`: 0, the smallest subnormal, 2⁻¹⁴, 0.5, 1,
/// 204.75 and 65504 (a saturated time).
const ESCAPES: [u16; 7] = [0x0000, 0x0001, 0x0400, 0x3800, 0x3c00, 0x5a66, 0x7bff];

/// The `qr_w7` a quad stores after folding the escape times `times` from the starting pair (`min0`, `max0`), each a
/// binary16 pattern, by the min and max reductions with no branch, read back through the generated accessors.
fn reduce(min0: u16, max0: u16, times: &[u16]) -> u32 {
    let start = u32::from(max0) << 16 | u32::from(min0);
    let (mut lo, mut hi) = (qr_escape_time_min(start), qr_escape_time_max(start));
    for &t in times {
        let t = half(t) as f32;
        lo = lo.min(t);
        hi = hi.max(t);
    }
    // Back to binary16: every value here is a binary16 value or ±∞, so the pattern is found exactly.
    let bits = |v: f32| {
        (0..=u16::MAX)
            .find(|&h| half(h).to_bits() == f64::from(v).to_bits())
            .unwrap_or_else(|| panic!("{v} is not a binary16 value"))
    };
    u32::from(bits(hi)) << 16 | u32::from(bits(lo))
}

/// Every subset of [`ESCAPES`] (the empty one first) reduced from (`min0`, `max0`), checked against R-418.
fn check_no_escape(min0: u16, max0: u16) {
    for mask in 0u32..(1 << ESCAPES.len()) {
        let times: Vec<u16> = (0..ESCAPES.len())
            .filter(|i| mask >> i & 1 == 1)
            .map(|i| ESCAPES[i])
            .collect();
        let w = reduce(min0, max0, &times);
        let (lo, hi) = (qr_escape_time_min(w), qr_escape_time_max(w));
        if times.is_empty() {
            assert_eq!(
                (w & 0xffff, w >> 16),
                (0x7c00, 0xfc00),
                "a quad with no escaped copy stores {w:#010x}, not +∞ and −∞"
            );
            assert!(lo > hi, "no escape does not read _min > _max: {lo} ≤ {hi}");
            assert!(
                qr_escape_time_min_is_sentinel(w) && qr_escape_time_max_is_sentinel(w),
                "no escape ({w:#010x}) is not each escape time's sentinel"
            );
        } else {
            let want_lo = times.iter().map(|&t| half(t)).fold(f64::INFINITY, f64::min);
            let want_hi = times
                .iter()
                .map(|&t| half(t))
                .fold(f64::NEG_INFINITY, f64::max);
            assert_eq!(
                (f64::from(lo), f64::from(hi)),
                (want_lo, want_hi),
                "the reductions from the stored identities over {times:#06x?} are not the times' min and max"
            );
            assert!(
                lo <= hi,
                "a quad with escaped copies {times:#06x?} reads _min > _max"
            );
            assert!(
                !qr_escape_time_min_is_sentinel(w) && !qr_escape_time_max_is_sentinel(w),
                "a quad with escaped copies {times:#06x?} reads as an escape-time sentinel ({w:#010x})"
            );
        }
    }
}

#[test]
fn qa_quad_reduction_no_escape_identities_rust() {
    // The generated constants are R-418's bits and decode to +∞ and −∞.
    assert_eq!(
        (
            QR_ESCAPE_TIME_MIN_SENTINEL_BITS,
            QR_ESCAPE_TIME_MAX_SENTINEL_BITS
        ),
        (0x7c00, 0xfc00)
    );
    assert_eq!(half(QR_ESCAPE_TIME_MIN_SENTINEL_BITS as u16), f64::INFINITY);
    assert_eq!(
        half(QR_ESCAPE_TIME_MAX_SENTINEL_BITS as u16),
        f64::NEG_INFINITY
    );
    check_no_escape(
        QR_ESCAPE_TIME_MIN_SENTINEL_BITS as u16,
        QR_ESCAPE_TIME_MAX_SENTINEL_BITS as u16,
    );
}

negative_control!(
    qa_quad_reduction_no_escape_identities_rust,
    "RQ-268's option (d), +∞ as escape_time_max's start, is not the max reduction's identity",
    expected = "a quad with no escaped copy stores 0x7c007c00, not +∞ and −∞",
    check_no_escape(0x7c00, 0x7c00)
);

negative_control!(
    qa_quad_reduction_no_escape_identities_rust_zero_start,
    "a max reduction starting at 0, a recorded time, makes no escape read like an escape at t = 0",
    expected = "not +∞ and −∞",
    check_no_escape(0x7c00, 0x0000)
);

negative_control!(
    qa_quad_reduction_no_escape_identities_rust_finite_min,
    "a min reduction starting at 65504, the saturated time, is not the identity",
    expected = "not +∞ and −∞",
    check_no_escape(0x7bff, 0xfc00)
);
