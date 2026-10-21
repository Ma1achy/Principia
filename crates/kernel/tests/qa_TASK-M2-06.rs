//! QA tests for TASK-M2-06, written from the requirements, not from the implementation: the CPU side of the shared
//! source, at f32 and f64 (R-118, R-265). The GPU side is `engine`'s `qa_TASK-M2-06_gpu`.
//! - REQ-TOOL-030 / R-436 (dd_decoder §3.6, "Each producer selects its pole value"): at the poles the stored values are
//!   the IEEE patterns `+∞` (`rho_ratio` where `‖λ̃‖ = 0`), `−∞` (`V_0` wherever any pair separation is 0) and `+0`
//!   (`virial_ratio` where `V_0` is non-finite), compared on their bits, never by arithmetic on `∞`; one step off each
//!   pole the value is the plain quotient (`qa_m206_pole_*`). The select's own primitive gives exactly the pattern on
//!   one side and `self`'s bits, a NaN's payload included, on the other; the bit test is an exponent of all ones
//!   (`qa_m206_ieee_bits_*`).
//! - REQ-DEC-007 (scheduler_contract Part 5, `QuadRequest.flags` bit 0): `DECODE_MODE` is bit 0 of the flags word and
//!   nothing else in the word changes it (`qa_m206_decode_mode_bit0`); the chart's parameters, `z₀`, `q₁`, `q₂`, are
//!   carried by the uniform, every one of the 24 recoverable from it (`qa_m206_uniform_round_trip`).
//! - The `DEGENERATE` readback tag: every reason, and none, round-trips through its tag word; the words are distinct and
//!   no other word decodes (`qa_m206_tag_round_trip`).
//! - REQ-DEC-047 (dd_generation_root §3.9's `tanh` operator; the requirement's verify detail): `tanh_sat` lies at or
//!   above the first argument where `tanh` rounds to ±1, found here by search at each width, and below 43.671875, from
//!   which Metal's GPU `tanh` was measured to fail, 0 and then NaN (`qa_m206_tanh_sat_window`).
//!
//! **Domain** (pitfalls §10): the decode's stateless pieces, inputs fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use kernel::chart::Latent;
use kernel::decode::descriptor::{describe, potential, ratio, virial};
use kernel::decode::{Degenerate, Ic};
use kernel::generated::constants::tanh_sat;
use kernel::real::IeeeBits;
use kernel::stage::decode::{decode_mode, flags, tag, untag, DecodeMode, UniformChart};
use validation::negative_control;

const POS32: u32 = 0x7F80_0000;
const NEG32: u32 = 0xFF80_0000;
const POS64: u64 = 0x7FF0_0000_0000_0000;
const NEG64: u64 = 0xFFF0_0000_0000_0000;

// ── IeeeBits: the select and the bit test, both widths ─────────────────────────────────────────────────────────

/// Values whose bits the select must pass through untouched: zeros of both signs, a subnormal, ordinary values,
/// the largest finite, both infinities and NaNs with a payload and a sign.
fn samples32() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1.0,
        -2.5,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFA0_0001),
    ]
}

fn samples64() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        1.0,
        -2.5,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7FF8_0000_0000_1234),
        f64::from_bits(0xFFF4_0000_0000_0001),
    ]
}

/// `or_pos_inf`/`or_neg_inf` give exactly the pattern when the condition holds and `x`'s own bits when it does not;
/// `non_finite_bits` holds exactly when the exponent field is all ones.
fn check_ieee_bits(pos: fn(f32, bool) -> f32, neg: fn(f32, bool) -> f32) {
    for x in samples32() {
        assert_eq!(
            pos(x, true).to_bits(),
            POS32,
            "or_pos_inf({x:?}, true) at f32"
        );
        assert_eq!(
            neg(x, true).to_bits(),
            NEG32,
            "or_neg_inf({x:?}, true) at f32"
        );
        assert_eq!(
            pos(x, false).to_bits(),
            x.to_bits(),
            "or_pos_inf({x:?}, false) changed the bits at f32"
        );
        assert_eq!(
            neg(x, false).to_bits(),
            x.to_bits(),
            "or_neg_inf({x:?}, false) changed the bits at f32"
        );
        assert_eq!(
            x.non_finite_bits(),
            x.to_bits() & POS32 == POS32,
            "non_finite_bits({x:?}) at f32"
        );
    }
    for x in samples64() {
        assert_eq!(
            x.or_pos_inf(true).to_bits(),
            POS64,
            "or_pos_inf({x:?}) at f64"
        );
        assert_eq!(
            x.or_neg_inf(true).to_bits(),
            NEG64,
            "or_neg_inf({x:?}) at f64"
        );
        assert_eq!(
            x.or_pos_inf(false).to_bits(),
            x.to_bits(),
            "or_pos_inf({x:?}, false) at f64"
        );
        assert_eq!(
            x.or_neg_inf(false).to_bits(),
            x.to_bits(),
            "or_neg_inf({x:?}, false) at f64"
        );
        assert_eq!(
            x.non_finite_bits(),
            x.to_bits() & POS64 == POS64,
            "non_finite_bits({x:?}) at f64"
        );
    }
    // The bit test on the boundary: the largest finite is finite, the smallest non-finite pattern is not.
    assert!(!f32::MAX.non_finite_bits() && f32::from_bits(POS32).non_finite_bits());
    assert!(!f64::MAX.non_finite_bits() && f64::from_bits(POS64).non_finite_bits());
}

#[test]
fn qa_m206_ieee_bits_select_and_test() {
    check_ieee_bits(IeeeBits::or_pos_inf, IeeeBits::or_neg_inf);
}

negative_control!(
    qa_m206_ieee_bits_select_and_test,
    "a select that keeps a NaN by arithmetic, which quiets its payload, must fail",
    expected = "changed the bits at f32",
    check_ieee_bits(
        |x, c| if c { f32::from_bits(POS32) } else { x + 0.0 },
        IeeeBits::or_neg_inf
    )
);

// ── The poles (R-436), both sides of each select, both widths ────────────────────────────────────────────────

/// Equal masses at the given positions, with momenta summing to 0.
fn ic<R: From<f32> + Copy>(r: [[f32; 2]; 3]) -> Ic<R> {
    let t = R::from(1.0 / 3.0);
    Ic {
        m: [t, t, t],
        r: r.map(|v| v.map(R::from)),
        p: [[0.5, 0.0], [-0.25, 0.1], [-0.25, -0.1]].map(|v: [f32; 2]| v.map(R::from)),
    }
}

/// The poles' bits through `describe` at both widths and the three helpers on each side of each select.
fn check_poles(collisions: &[[[f32; 2]; 3]]) {
    // Each pair in turn coincident, and all three: V_0 is −∞'s bits for the whole sum, virial_ratio +0's,
    // r_min_pair_0 0.
    for (k, r) in collisions.iter().enumerate() {
        let d32 = describe::<f32>(&ic(*r));
        let d64 = describe::<f64>(&ic(*r));
        assert_eq!(
            (d32.V_0.to_bits(), d64.V_0.to_bits()),
            (NEG32, NEG64),
            "V_0 is not −∞'s bits at collision {k}"
        );
        assert_eq!(
            (d32.virial_ratio.to_bits(), d64.virial_ratio.to_bits()),
            (0, 0),
            "virial_ratio is not +0's bits at collision {k}"
        );
        assert_eq!(
            (d32.r_min_pair_0, d64.r_min_pair_0),
            (0.0, 0.0),
            "r_min_pair_0 is not 0 at collision {k}"
        );
        assert_eq!(
            potential::<f64>(&ic(*r)).to_bits(),
            NEG64,
            "potential at collision {k}"
        );
        assert_eq!(
            potential::<f32>(&ic(*r)).to_bits(),
            NEG32,
            "potential at collision {k}"
        );
    }
    // ‖λ̃‖ = 0: r₂ at the inner pair's centre of mass, with ‖ρ̃‖ > 0 and with ‖ρ̃‖ = 0 too (0/0 never read).
    for r in [
        [[-0.5, 0.0], [0.5, 0.0], [0.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
    ] {
        let d32 = describe::<f32>(&ic(r));
        let d64 = describe::<f64>(&ic(r));
        assert_eq!(d32.lambda_mag, 0.0, "‖λ̃‖ is not 0 at {r:?}");
        assert_eq!(
            (d32.rho_ratio.to_bits(), d64.rho_ratio.to_bits()),
            (POS32, POS64),
            "rho_ratio is not +∞'s bits where ‖λ̃‖ = 0 at {r:?}"
        );
    }
    // One step off each pole: the plain quotients, finite.
    let off = describe::<f64>(&ic([[-1e-3, 0.0], [1e-3, 0.0], [0.0, 1.0]]));
    let (a, b, m) = (f64::from(1e-3f32), 1.0f64, f64::from(1.0f32 / 3.0));
    let want_v = -(m * m) * (1.0 / (2.0 * a) + 2.0 / (a * a + b * b).sqrt());
    assert!(
        off.V_0.is_finite() && (off.V_0 - want_v).abs() <= 1e-12 * want_v.abs(),
        "V_0 one step off the collision is {}, not {want_v}",
        off.V_0
    );
    assert!(off.virial_ratio.is_finite() && off.virial_ratio > 0.0);
    let off = describe::<f32>(&ic([[-0.5, 0.0], [0.5, 0.0], [0.0, 1e-15]]));
    assert!(
        off.lambda_mag > 0.0
            && off.rho_ratio.is_finite()
            && off.rho_ratio == off.rho_mag / off.lambda_mag,
        "rho_ratio one step off ‖λ̃‖ = 0 is {}, not the quotient",
        off.rho_ratio
    );
    // The select follows the divisor as computed: where ‖λ‖² underflows at f32 (λ_y = 10⁻³⁰), ‖λ̃‖ is 0 and the
    // ratio +∞'s bits; at f64 the same λ_y is no pole.
    let under = [[-0.5, 0.0], [0.5, 0.0], [0.0, 1e-30]];
    let (d32, d64) = (describe::<f32>(&ic(under)), describe::<f64>(&ic(under)));
    assert_eq!(
        (d32.lambda_mag == 0.0, d32.rho_ratio.to_bits() == POS32),
        (true, true),
        "rho_ratio at f32 does not follow its own ‖λ̃‖ = {:e}",
        d32.lambda_mag
    );
    assert!(d64.lambda_mag > 0.0 && d64.rho_ratio == d64.rho_mag / d64.lambda_mag);
    // The helpers alone: ratio selects on its divisor, either zero; virial on V_0's bits.
    for (num, den) in [(1.0f64, 0.0), (0.0, 0.0), (f64::NAN, 0.0), (1.0, -0.0)] {
        assert_eq!(
            ratio(num, den).to_bits(),
            POS64,
            "ratio({num}, {den:?}) at f64"
        );
        assert_eq!(
            ratio(num as f32, den as f32).to_bits(),
            POS32,
            "ratio({num}, {den:?}) at f32"
        );
    }
    assert_eq!(ratio(3.0f64, 2.0), 1.5);
    assert_eq!(ratio(f32::MIN_POSITIVE, 4.0), f32::MIN_POSITIVE / 4.0);
    for v in [f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
        assert_eq!(virial(1.0f64, v).to_bits(), 0, "virial(1, {v}) at f64");
        assert_eq!(
            virial(1.0f32, v as f32).to_bits(),
            0,
            "virial(1, {v}) at f32"
        );
    }
    assert_eq!(virial(1.0f64, -4.0), 0.5);
    assert_eq!(virial(1.0f32, -f32::MAX), 2.0 / f32::MAX);
}

/// Each of the three pairs coincident, and all three bodies.
const COLLISIONS: [[[f32; 2]; 3]; 4] = [
    [[0.25, 0.0], [0.25, 0.0], [0.0, 1.0]],
    [[0.25, 0.0], [-0.5, 0.0], [0.25, 0.0]],
    [[0.0, 1.0], [0.25, 0.5], [0.25, 0.5]],
    [[0.1, 0.2], [0.1, 0.2], [0.1, 0.2]],
];

#[test]
fn qa_m206_pole_bits_both_widths() {
    check_poles(&COLLISIONS);
}

negative_control!(
    qa_m206_pole_bits_both_widths,
    "a 'collision' whose pair is 10⁻³ apart is no pole and must fail",
    expected = "V_0 is not −∞'s bits at collision 0",
    check_poles(&[[[0.25, 0.0], [0.251, 0.0], [0.0, 1.0]]])
);

// ── DECODE_MODE: bit 0 of the flags word ─────────────────────────────────────────────────────────────────────

/// `mode_of` reads bit 0 alone: every word's mode is its bit 0's, full on 0, linearised on 1; `flags` round-trips.
fn check_mode_bit(mode_of: fn(u32) -> DecodeMode) {
    let mut words: Vec<u32> = vec![
        0,
        1,
        2,
        3,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
        0x8000_0000,
        0x8000_0001,
    ];
    words.extend((0..32).flat_map(|b| [1u32 << b, (1u32 << b) | 1, !(1u32 << b)]));
    let mut x: u32 = 0x1234_5678;
    for _ in 0..4096 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        words.push(x);
    }
    for w in words {
        let want = if w & 1 == 0 {
            DecodeMode::Full
        } else {
            DecodeMode::Linearised
        };
        assert_eq!(
            mode_of(w),
            want,
            "flags word {w:#010x}: DECODE_MODE is bit 0"
        );
    }
    for m in [DecodeMode::Full, DecodeMode::Linearised] {
        assert_eq!(mode_of(flags(m)), m, "flags({m:?}) does not read back");
        assert_eq!(flags(m) & !1, 0, "flags({m:?}) sets a bit other than 0");
    }
}

#[test]
fn qa_m206_decode_mode_bit0() {
    check_mode_bit(decode_mode);
}

negative_control!(
    qa_m206_decode_mode_bit0,
    "a reader that takes any non-zero word as linearised must fail",
    expected = "DECODE_MODE is bit 0",
    check_mode_bit(|w| if w == 0 {
        DecodeMode::Full
    } else {
        DecodeMode::Linearised
    })
);

// ── The uniform carries the chart's parameters ───────────────────────────────────────────────────────────────

/// Every one of `z₀`, `q₁`, `q₂`'s 24 values, set alone to a distinct value, reads back from the uniform where it was
/// written, the others 0; and random charts round-trip bit for bit.
fn check_uniform(read: fn(&Latent<f32>) -> Latent<f32>) {
    for k in 0..24 {
        let mut c = Latent {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        };
        let v = 1.0 + k as f32;
        match k / 8 {
            0 => c.z0[k % 8] = v,
            1 => c.q1[k % 8] = v,
            _ => c.q2[k % 8] = v,
        }
        assert_eq!(
            read(&c),
            c,
            "parameter {k} does not round-trip through the uniform"
        );
    }
    let mut s: u32 = 99;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from_bits((s >> 9) | 0x3F80_0000) - 1.5
    };
    for _ in 0..64 {
        let c = Latent {
            z0: core::array::from_fn(|_| next() * 1e3),
            q1: core::array::from_fn(|_| next()),
            q2: core::array::from_fn(|_| next()),
        };
        assert_eq!(
            read(&c),
            c,
            "a chart does not round-trip through the uniform"
        );
    }
}

#[test]
fn qa_m206_uniform_round_trip() {
    check_uniform(|c| Latent::<f32>::from_uniform(&c.uniform()));
}

negative_control!(
    qa_m206_uniform_round_trip,
    "a uniform reader that swaps q₁ and q₂ must fail",
    expected = "does not round-trip through the uniform",
    check_uniform(|c| {
        let r = Latent::<f32>::from_uniform(&c.uniform());
        Latent {
            z0: r.z0,
            q1: r.q2,
            q2: r.q1,
        }
    })
);

// ── The DEGENERATE tag word ──────────────────────────────────────────────────────────────────────────────────

/// Every reason the type holds, and none.
///
/// TASK-M2-24 closed the set (dd_decoder §2, "The `DEGENERATE` cause set"): `INVARIANT_INFEASIBLE` and
/// `NO_QUALIFYING_SEED` joined it, so they write tag words too.
const REASONS: [Option<Degenerate>; 6] = [
    None,
    Some(Degenerate::M01Tiny),
    Some(Degenerate::EnergyInfeasible),
    Some(Degenerate::EnergyUnscalable),
    Some(Degenerate::InvariantInfeasible),
    Some(Degenerate::NoQualifyingSeed),
];

/// Each reason round-trips; the six words are distinct; every other word, 0 … 2¹⁶ and the top of the range, is
/// refused with the word itself.
fn check_tags(to: fn(Option<Degenerate>) -> u32) {
    let words: Vec<u32> = REASONS.iter().map(|&r| to(r)).collect();
    for (r, w) in REASONS.iter().zip(&words) {
        assert_eq!(
            untag(*w),
            Ok(*r),
            "{r:?}'s tag word {w} does not round-trip"
        );
    }
    let mut sorted = words.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        REASONS.len(),
        "two reasons share a tag word: {words:?}"
    );
    for w in (0..=u32::from(u16::MAX)).chain(u32::MAX - 1024..=u32::MAX) {
        if !words.contains(&w) {
            assert_eq!(
                untag(w),
                Err(w),
                "word {w:#x} decodes though no reason writes it"
            );
        }
    }
}

#[test]
fn qa_m206_tag_round_trip() {
    check_tags(tag);
}

negative_control!(
    qa_m206_tag_round_trip,
    "a tag that writes both η_E reasons as the same word must fail",
    expected = "does not round-trip",
    check_tags(|r| match r {
        Some(Degenerate::EnergyUnscalable) => tag(Some(Degenerate::EnergyInfeasible)),
        other => tag(other),
    })
);

// ── tanh_sat's window (REQ-DEC-047) ──────────────────────────────────────────────────────────────────────────

/// The least f32 `x > 1` with `tanh(x) = 1` exactly, by bisection on the bit pattern (tanh is monotone).
fn saturates32() -> f32 {
    let (mut lo, mut hi) = (1.0f32.to_bits(), 100.0f32.to_bits());
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if f32::from_bits(mid).tanh() == 1.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    f32::from_bits(hi)
}

/// The least f64 `x > 1` with `tanh(x) = 1` exactly.
fn saturates64() -> f64 {
    let (mut lo, mut hi) = (1.0f64.to_bits(), 100.0f64.to_bits());
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if f64::from_bits(mid).tanh() == 1.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    f64::from_bits(hi)
}

/// `t` is at or above where `tanh` rounds to ±1 at both widths (and `tanh` is ±1 at `±t`, so the clamp changes no
/// CPU value), and below 43.671875, the first argument where Metal's GPU `tanh` was measured to fail, 0 and then NaN
/// (REQ-DEC-047's verify detail). Both widths' `tanh_sat` are the same number.
fn check_window(t32: f32, t64: f64) {
    let (s32, s64) = (saturates32(), saturates64());
    println!("tanh rounds to 1 from {s32} at f32 and {s64} at f64; tanh_sat = {t64}");
    assert!(
        t32 >= s32 && t64 >= s64,
        "tanh_sat ({t32}, {t64}) is below where tanh rounds to 1 ({s32} at f32, {s64} at f64)"
    );
    assert_eq!(t32.tanh(), 1.0);
    assert_eq!((-t32).tanh(), -1.0);
    assert_eq!(t64.tanh(), 1.0);
    assert_eq!((-t64).tanh(), -1.0);
    assert!(
        t64 < 43.671_875,
        "tanh_sat = {t64} is not below 43.671875, Metal's failure"
    );
    assert_eq!(f64::from(t32), t64, "tanh_sat differs between the widths");
}

#[test]
fn qa_m206_tanh_sat_window() {
    check_window(tanh_sat::<f32>(), tanh_sat::<f64>());
}

negative_control!(
    qa_m206_tanh_sat_window,
    "a bound of 10, where f64's tanh is not yet 1, must fail",
    expected = "is below where tanh rounds to 1",
    check_window(10.0, 10.0)
);
