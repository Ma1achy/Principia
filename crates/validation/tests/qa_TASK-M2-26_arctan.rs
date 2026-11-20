//! QA's tests for TASK-M2-26's fix round on RQ-628 (option (a), the kernel's own f32 `atan2`), written from REQ-RENDER-025,
//! REQ-ENC-024's `ε_phys` and RQ-628's stated mechanism, not from `kernel::arctan`'s steps.
//!
//! - `qa_m226_encode_wgsl_reads_no_backend_atan2`: REQ-RENDER-025's "as the backend compiles it" on a backend whose
//!   builtin `atan2` is only as accurate as WGSL requires: the checked-in fragment encode the ROUNDTRIP view runs calls no
//!   builtin `atan2`. Control: the same text with one call added.
//! - `qa_m226_atan2_within_half_ulp_everywhere`: the stated bound (0.50002 ULP of the f32 nearest the exact value) on
//!   seeded operands of every sign and across the exponents, against f64 `atan2`, which is far more accurate than a f32
//!   ULP. Control: a result one step off.
//! - `qa_m226_light_body_angle_keeps_rho_within_eps_phys`: RQ-628's failure. The decode rebuilds `‖ρ̃‖` from `cos α`, so
//!   an error `δα` returns as `δα / cos α` relative in `ρ̃`. Over `cos α ≥ 0.03` (RQ-628's light body has `|ρ̃|/|λ̃|` =
//!   0.034) that relative error stays within `ε_phys`. Control: RQ-628's lavapipe error, `2.07·10⁻⁵·cos α` on `α`.
//!
//! **Domain** (pitfalls §10): stateless single evaluations; no trajectory.

use kernel::arctan::atan2_f32;
use validation::negative_control;

/// The fragment encode the ROUNDTRIP preset runs, as checked in.
const ENCODE_WGSL: &str = include_str!("../../render/wgsl/encode.wgsl");

/// The accuracy `kernel::arctan`'s contract states: at most this many ULP of the f32 nearest the exact value.
const STATED_ULP: f64 = 0.50002;

/// A small deterministic generator (SplitMix64), so the operands are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A finite, normal f32 of random sign, mantissa and an exponent within `2^±spread` of 1.
    fn operand(&mut self, spread: i32) -> f32 {
        let exponent = (127 - spread + (self.next() % (2 * spread as u64 + 1)) as i32) as u32;
        let bits = (self.next() as u32 & 0x8000_0000) | (exponent << 23);
        f32::from_bits(bits | (self.next() as u32 & 0x007F_FFFF))
    }
}

// ── The fragment encode reads no builtin ─────────────────────────────────────────────────────────────────────────────

/// Calls of the builtin `atan2` in `wgsl`: an `atan2(` not part of a longer name.
fn builtin_atan2_calls(wgsl: &str) -> usize {
    wgsl.match_indices("atan2(")
        .filter(|(at, _)| {
            !wgsl[..*at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
        .count()
}

fn check_no_builtin(wgsl: &str) {
    assert!(wgsl.contains("fn "), "the encode's text holds no function");
    assert_eq!(
        builtin_atan2_calls(wgsl),
        0,
        "the fragment encode calls the backend's atan2"
    );
}

#[test]
fn qa_m226_encode_wgsl_reads_no_backend_atan2() {
    check_no_builtin(ENCODE_WGSL);
}

negative_control!(
    qa_m226_encode_wgsl_reads_no_backend_atan2,
    "an encode with one builtin atan2 added is refused",
    expected = "calls the backend's atan2",
    check_no_builtin(&format!(
        "{ENCODE_WGSL}\nfn added(y: f32) -> f32 {{ return atan2(y, 1f); }}\n"
    ))
);

// ── The stated accuracy ──────────────────────────────────────────────────────────────────────────────────────────────

/// One ULP at `v`: the gap from `|v|` to the next f32 up.
fn ulp(v: f32) -> f64 {
    let a = v.abs();
    f64::from(f32::from_bits(a.to_bits() + 1)) - f64::from(a)
}

fn check_half_ulp(atan2: fn(f32, f32) -> f32) {
    let mut rng = Rng(0x0343_0628);
    let mut worst = 0.0_f64;
    let (mut below_pi_4, mut past_pi_4) = (0_u32, 0_u32);
    for i in 0..200_000_u32 {
        // Half the draws have nearby magnitudes, so every breakpoint and octant is reached; the rest are far apart.
        let spread = if i % 2 == 0 { 2 } else { 60 };
        let (y, x) = (rng.operand(spread), rng.operand(spread));
        let exact = f64::from(y).atan2(f64::from(x));
        let nearest = exact as f32;
        let got = atan2(y, x);
        let ulps = (f64::from(got) - exact).abs() / ulp(nearest);
        assert!(
            ulps <= STATED_ULP,
            "atan2({y:e}, {x:e}) = {got:e}, {ulps} ULP from {exact:e}"
        );
        assert_eq!(
            got.is_sign_negative(),
            y.is_sign_negative(),
            "atan2({y:e}, {x:e}) has not y's sign"
        );
        worst = worst.max(ulps);
        if y.abs() > x.abs() {
            past_pi_4 += 1;
        } else {
            below_pi_4 += 1;
        }
    }
    println!("worst {worst} ULP; |y| > |x| {past_pi_4}, else {below_pi_4}");
    assert!(
        below_pi_4 > 20_000 && past_pi_4 > 20_000,
        "the draws miss an octant"
    );
}

#[test]
fn qa_m226_atan2_within_half_ulp_everywhere() {
    check_half_ulp(atan2_f32);
}

negative_control!(
    qa_m226_atan2_within_half_ulp_everywhere,
    "a result one step up is past the stated bound",
    expected = "ULP from",
    check_half_ulp(|y, x| f32::from_bits(atan2_f32(y, x).to_bits() + 1))
);

// ── RQ-628's light body ──────────────────────────────────────────────────────────────────────────────────────────────

/// The least `cos α` the sweep reaches, under RQ-628's `|ρ̃|/|λ̃| = 0.034`.
const LEAST_COS: f64 = 0.03;

/// `ε_phys` at f32: REQ-ENC-024's `k·2⁻²³`.
fn eps_phys() -> f64 {
    ledger::constants::EPS_PHYS.number() * f64::from(f32::EPSILON)
}

fn check_light_body(atan2: fn(f32, f32) -> f32) {
    let mut rng = Rng(0x0343_0629);
    let (mut worst, mut samples) = (0.0_f64, 0_u32);
    for _ in 0..200_000 {
        // `α = atan2(‖λ̃‖, ‖ρ̃‖)` with both on their section's scale and `α` in `[π/4, acos(0.03)]`.
        let ratio = 0.031 + 0.969 * rng.unit();
        let lambda = 0.5 + rng.unit();
        let (y, x) = (lambda as f32, (lambda * ratio) as f32);
        let (y64, x64) = (f64::from(y), f64::from(x));
        let cos_exact = x64 / y64.hypot(x64);
        if cos_exact < LEAST_COS {
            continue;
        }
        let relative = (f64::from(atan2(y, x)).cos() - cos_exact).abs() / cos_exact;
        assert!(
            relative <= eps_phys(),
            "atan2({y:e}, {x:e}): ρ̃ returns {relative:e} relative, past ε_phys {:e}",
            eps_phys()
        );
        worst = worst.max(relative);
        samples += 1;
    }
    println!(
        "worst {worst:e} relative in ρ̃ over {samples} samples; ε_phys {:e}",
        eps_phys()
    );
    assert!(samples > 180_000, "the sweep lost its samples");
}

#[test]
fn qa_m226_light_body_angle_keeps_rho_within_eps_phys() {
    check_light_body(atan2_f32);
}

negative_control!(
    qa_m226_light_body_angle_keeps_rho_within_eps_phys,
    "RQ-628's lavapipe error near π/2 returns ρ̃ past ε_phys",
    expected = "past ε_phys",
    check_light_body(|y, x| {
        let exact = f64::from(y).atan2(f64::from(x));
        (exact + 2.07e-5 * exact.cos()) as f32
    })
);
