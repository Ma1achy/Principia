//! QA tests for TASK-M2-48, written from the requirements, not from the implementation: `M₀₁` in R-547's form.
//!
//! - REQ-DEC-011 (dd_decoder §3.1 "The branch input, rewritten"): the mass block emits `DEGENERATE(M01_TINY)` iff
//!   `M₀₁ < ε`, strict, `M₀₁` the one rounded sum of the emitted masses. Checked against an oracle independent of the
//!   block (at f64 the IEEE add, which Rust never contracts; at f32 the f64 sum of the two f32 masses rounded once to
//!   f32, innocuous double rounding since 53 ≥ 2·24 + 2): at `ε` itself and its neighbours, at sums whose exact value
//!   lies on one side of `ε` and whose rounding lands on the other, at ties, and over exponent gaps well past the
//!   form's cap of 60 (`qa_m48_strict_eps`, `qa_m48_sum_gaps`).
//! - REQ-DEC-011's control and REQ-VAL-017's (RQ-621, option (a)): at TASK-M2-02's onset crossings, 20 000 lines
//!   `z₁ ∈ [6.3, 20.3]`, bisected here on the oracle's tag, at f32 and f64, the block's `M₀₁` and tag are the
//!   oracle's at every point, while the superseded `fma(m₁, 1, m₀)` with its `×1` folded and its add contracted,
//!   `fma(t₇, t₁₃, m₀)` fused, gives a different tag at one point at least (`qa_m48_crossings_fused_flips`).
//! - REQ-DEC-011 "flushing or not": a flushing backend emits a subnormal mass as 0; the block's tag on the flushed
//!   masses is its tag on the kept ones, at f32 and f64 (`qa_m48_flush_keeps_tag`).
//!
//! **Domain** (pitfalls §10): the mass block alone on the CPU, its inputs held fixed; no trajectory. The GPU half of
//! REQ-DEC-011 and REQ-VAL-017 is `engine`'s `decode_parity_m01_crossings`, which compares the GPU's tag with this
//! block's form on the GPU's own emitted masses; these tests pin that form to the independent oracle.
//!
//! Each test has a registered negative control (R-176).

use kernel::decode::degenerate::Degenerate;
use kernel::decode::mass::{self, Masses};
use kernel::generated::links::{
    tanh_sat, Link, LinkReal, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use kernel::real::IeeeBits;
use proptest::prelude::*;
use validation::{negative_control, prop};

type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

trait Fp: LinkReal + IeeeBits + std::fmt::Debug {
    fn of(x: f64) -> Self;
    fn bits(self) -> u64;
    /// The once-rounded sum `a + b` at this precision, formed independently of the block.
    fn oracle_sum(a: Self, b: Self) -> Self;
    fn down(self) -> Self;
    fn up(self) -> Self;
    /// `self` as a flushing backend holds it: 0 if subnormal.
    fn ftz(self) -> Self;
    /// `fma(a, b, c)`, fused: one rounding.
    fn fused(a: Self, b: Self, c: Self) -> Self;
}

impl Fp for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn oracle_sum(a: Self, b: Self) -> Self {
        a + b
    }
    fn down(self) -> Self {
        f64::from_bits(self.to_bits() - 1)
    }
    fn up(self) -> Self {
        f64::from_bits(self.to_bits() + 1)
    }
    fn ftz(self) -> Self {
        if self.is_subnormal() {
            0.0
        } else {
            self
        }
    }
    fn fused(a: Self, b: Self, c: Self) -> Self {
        a.mul_add(b, c)
    }
}

impl Fp for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
    fn oracle_sum(a: Self, b: Self) -> Self {
        (f64::from(a) + f64::from(b)) as f32
    }
    fn down(self) -> Self {
        f32::from_bits(self.to_bits() - 1)
    }
    fn up(self) -> Self {
        f32::from_bits(self.to_bits() + 1)
    }
    fn ftz(self) -> Self {
        if self.is_subnormal() {
            0.0
        } else {
            self
        }
    }
    fn fused(a: Self, b: Self, c: Self) -> Self {
        a.mul_add(b, c)
    }
}

/// A mass form under test: the block's `from_masses`, or a control's.
type Form<R> = fn([R; 3]) -> Masses<R>;

/// The oracle's tag for `m`: its once-rounded `m₀ + m₁` strictly below ε.
fn oracle_tag<R: Fp>(m0: R, m1: R) -> Option<Degenerate> {
    (R::oracle_sum(m0, m1) < mass::m01_epsilon::<R>()).then_some(Degenerate::M01Tiny)
}

/// `form`'s `M₀₁` is the oracle's bit for bit and its tag the oracle's.
fn check_point<R: Fp>(form: Form<R>, m0: R, m1: R) -> Result<(), String> {
    let out = form([m0, m1, R::one() - m0 - m1]);
    let want = R::oracle_sum(m0, m1);
    if out.m01.bits() != want.bits() {
        return Err(format!(
            "M₀₁ is not the one rounded sum at {}, m = ({m0:?}, {m1:?}): {:?}, want {want:?}",
            R::NAME,
            out.m01
        ));
    }
    if out.degenerate != oracle_tag(m0, m1) {
        return Err(format!(
            "the tag is not M₀₁ < ε at {}, m = ({m0:?}, {m1:?}), M₀₁ = {want:?}: {:?}",
            R::NAME,
            out.degenerate
        ));
    }
    Ok(())
}

// ── Strict `< ε`, on the rounded sum ────────────────────────────────────────────────────────────────────────────

/// REQ-DEC-011's tag at `ε` and around it, each pair in both orders: the rounded sum exactly `ε` (by several splits),
/// one float below and above, an exact sum below `ε` that rounds up to it, one above that rounds down to it, and ties.
/// Both tags are reached, and `M₀₁ = ε` is untagged by name.
fn check_strict<R: Fp>(form: Form<R>) {
    let eps = mass::m01_epsilon::<R>();
    let half = R::of(0.5);
    let below = eps.down();
    let ulp_lo = eps - below;
    let ulp_hi = eps.up() - eps;
    let pairs = [
        (eps, R::zero()),
        (eps * half, eps * half),
        (below, ulp_lo),
        (eps * R::of(0.25), eps * R::of(0.75)),
        (below, R::zero()),
        (eps.up(), R::zero()),
        (below.down(), ulp_lo),
        // exact sum below ε, rounded up to ε: untagged, the tag reads the rounded sum
        (below, ulp_lo * R::of(0.75)),
        // exact sum below ε, rounded down: tagged
        (below, ulp_lo * R::of(0.25)),
        // exact sum above ε, rounded down to ε: untagged
        (eps, ulp_hi * R::of(0.25)),
        // ties, rounded to even either way
        (below, ulp_lo * half),
        (eps, ulp_hi * half),
        (below.down(), ulp_lo * half),
    ];
    let mut tagged = 0;
    for &(a, b) in &pairs {
        for (m0, m1) in [(a, b), (b, a)] {
            check_point(form, m0, m1).unwrap();
            let out = form([m0, m1, R::one() - m0 - m1]);
            if R::oracle_sum(m0, m1).bits() == eps.bits() {
                assert_eq!(
                    out.degenerate,
                    None,
                    "M₀₁ = ε tagged at {}, m = ({m0:?}, {m1:?}): the tag must be strict",
                    R::NAME
                );
            }
            tagged += usize::from(out.degenerate.is_some());
        }
    }
    assert!(
        tagged > 0 && tagged < 2 * pairs.len(),
        "the points reach one tag only at {}",
        R::NAME
    );
}

#[test]
fn qa_m48_strict_eps() {
    check_strict::<f64>(mass::from_masses);
    check_strict::<f32>(mass::from_masses);
}

/// The block with its tag read as `M₀₁ ≤ ε`.
#[cfg(feature = "controls")]
fn at_or_below(m: [f32; 3]) -> Masses<f32> {
    let out = mass::from_masses(m);
    Masses {
        degenerate: (out.m01 <= mass::m01_epsilon::<f32>()).then_some(Degenerate::M01Tiny),
        ..out
    }
}

negative_control!(
    qa_m48_strict_eps,
    "a tag read as M₀₁ ≤ ε must fail at M₀₁ = ε",
    expected = "the tag is not M₀₁ < ε",
    check_strict::<f32>(at_or_below)
);

// ── The sum over exponent gaps past the cap ─────────────────────────────────────────────────────────────────────

/// REQ-DEC-011's `M₀₁` over masses on very different scales, the smaller `2^0` to `2^−160` below the larger (past the
/// form's alignment cap of 60, and into the subnormals at f32), and the larger from `ε`'s binade to 1: bit for bit the
/// oracle's, and the tag its.
fn check_gaps<R: Fp>(form: Form<R>) {
    let strat = (
        0.0..1.0f64,
        1.0..2.0f64,
        -17i32..0,
        0i32..161,
        any::<bool>(),
    );
    prop::run(&strat, |(a, b, e, gap, swap)| {
        let big = R::of((1.0 + a) * 2f64.powi(e));
        let small = R::of(b * 2f64.powi(e - gap));
        let (m0, m1) = if swap { (small, big) } else { (big, small) };
        check_point(form, m0, m1).map_err(TestCaseError::fail)
    });
}

#[test]
fn qa_m48_sum_gaps() {
    check_gaps::<f64>(mass::from_masses);
    check_gaps::<f32>(mass::from_masses);
}

/// The block with `M₀₁` the sum truncated toward zero, as a form that drops the smaller term's low bits would give.
#[cfg(feature = "controls")]
fn truncated(m: [f64; 3]) -> Masses<f64> {
    let s = m[0] + m[1];
    let (big, small) = if m[0] >= m[1] {
        (m[0], m[1])
    } else {
        (m[1], m[0])
    };
    let m01 = if small - (s - big) < 0.0 { s.down() } else { s };
    Masses {
        m01,
        degenerate: (m01 < mass::m01_epsilon::<f64>()).then_some(Degenerate::M01Tiny),
        ..mass::from_masses(m)
    }
}

negative_control!(
    qa_m48_sum_gaps,
    "M₀₁ truncated toward zero must fail",
    expected = "M₀₁ is not the one rounded sum",
    check_gaps::<f64>(truncated)
);

// ── The onset crossings: the folded-and-fused control flips, the block does not ─────────────────────────────────

/// TASK-M2-02's lines.
const LINES: u32 = 20_000;

/// `stick_breaking`'s forward, from `kernel::generated::links`' op order, up to its last multiply: `(m₀, t₇, t₁₃)`,
/// `m₁ = t₇·t₁₃`. Checked against the link at every point it is used.
fn last_step<R: Fp>(z: [R; 2]) -> (R, R, R) {
    let sigmoid = |x: R| R::one() / (R::one() + (-x).exp());
    let t0 = R::LIT_3 * z[0];
    let t4 = R::LIT_1 * R::EPS_MU * tanh_sat(z[0]);
    let t8 = R::LIT_3 * z[1];
    let t12 = R::LIT_1 * R::EPS_MU * tanh_sat(z[1]);
    (sigmoid(-t0) + t4, sigmoid(t0) - t4, sigmoid(-t8) + t12)
}

/// The onset crossings at `R`: on each line `z₁ = 6.3 + 14k/LINES`, `z₀ ∈ [0, 30]` bisected to adjacent floats where
/// the oracle's tag on the link's masses turns on. Both points of each pair.
fn crossings<R: Fp>() -> Vec<[R; 2]> {
    let mut out = Vec::with_capacity(2 * LINES as usize);
    for k in 0..LINES {
        let z1 = R::of(6.3 + 14.0 * f64::from(k) / f64::from(LINES));
        let tag = |z0: R| {
            let m = <StickBreaking as Link<2, 3>>::forward([z0, z1]);
            oracle_tag(m[0], m[1]).is_some()
        };
        let (mut lo, mut hi) = (R::zero(), R::of(30.0));
        assert!(!tag(lo) && tag(hi), "no crossing on z₁ = {z1:?}");
        // bounded: each step halves the bracket, so a float's width is reached in under 1100 steps
        for _ in 0..1100 {
            let mid = (lo + hi) * R::of(0.5);
            if mid == lo || mid == hi {
                break;
            }
            if tag(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        assert_eq!(lo.up().bits(), hi.bits(), "the bisection did not close");
        out.push([lo, z1]);
        out.push([hi, z1]);
    }
    out
}

/// At every crossing, `block`'s masses are the link's and its tag the oracle's; returns how many points the folded and
/// fused `fma(t₇, t₁₃, m₀)` tags differently from the block.
fn check_crossings<R: Fp>(block: fn([R; 2]) -> Masses<R>) -> usize {
    let eps = mass::m01_epsilon::<R>();
    let (mut flips, mut tagged) = (0, 0);
    let points = crossings::<R>();
    for &z in &points {
        let m = <StickBreaking as Link<2, 3>>::forward(z);
        let (m0, t7, t13) = last_step(z);
        assert_eq!(
            [m0.bits(), (t7 * t13).bits()],
            [m[0].bits(), m[1].bits()],
            "the last step is not the link's at {z:?}"
        );
        let out = block(z);
        assert_eq!(out.m, m, "the block's masses are not the link's at {z:?}");
        assert_eq!(
            out.degenerate,
            oracle_tag(m[0], m[1]),
            "the block's tag is not the one rounded sum's at {}, z_μ = {z:?}",
            R::NAME
        );
        tagged += usize::from(out.degenerate.is_some());
        let fused = R::fused(t7, t13, m0) < eps;
        flips += usize::from(fused != out.degenerate.is_some());
    }
    // PIT-3: each pair straddles ε
    assert_eq!(
        tagged,
        LINES as usize,
        "the crossings do not straddle ε at {}",
        R::NAME
    );
    flips
}

#[test]
fn qa_m48_crossings_fused_flips() {
    let f64_flips = check_crossings::<f64>(mass::decode::<Stick, f64>);
    let f32_flips = check_crossings::<f32>(mass::decode::<Stick, f32>);
    eprintln!(
        "qa_m48_crossings_fused_flips: of {} points, fma(t₇, t₁₃, m₀) fused flips {f64_flips} at f64, {f32_flips} at \
         f32; the block flips none",
        2 * LINES
    );
    assert!(
        f64_flips > 0 && f32_flips > 0,
        "the folded-and-fused form flips no crossing ({f64_flips} at f64, {f32_flips} at f32): the control cannot fail"
    );
}

/// The block with `M₀₁` as a folding, contracting backend evaluates the superseded `fma(m₁, 1, m₀)`.
#[cfg(feature = "controls")]
fn folded_fused(z: [f32; 2]) -> Masses<f32> {
    let (m0, t7, t13) = last_step(z);
    let m01 = t7.mul_add(t13, m0);
    Masses {
        m01,
        degenerate: (m01 < mass::m01_epsilon::<f32>()).then_some(Degenerate::M01Tiny),
        ..mass::decode::<Stick, f32>(z)
    }
}

negative_control!(
    qa_m48_crossings_fused_flips,
    "a block whose M₀₁ is the folded-and-fused fma(t₇, t₁₃, m₀) must flip a crossing's tag",
    expected = "the block's tag is not the one rounded sum's",
    {
        check_crossings::<f32>(folded_fused);
    }
);

// ── Flushing or not ─────────────────────────────────────────────────────────────────────────────────────────────

/// REQ-DEC-011's "flushing or not": a flushing backend emits each subnormal mass as 0. On masses mixing zero,
/// subnormal and normal values, at and around `ε`, `form`'s tag on the flushed masses is its tag on the kept ones, and
/// on the kept ones it is the oracle's.
fn check_flush<R: Fp>(form: Form<R>) {
    let eps = mass::m01_epsilon::<R>();
    let tiny = R::min_positive_value();
    let sub = [
        R::zero(),
        tiny * R::of(0.5),
        tiny * R::of(0.25),
        tiny.down(),
    ];
    let mut values: Vec<R> = sub.to_vec();
    values.extend([tiny, eps.down(), eps, eps.up(), R::of(0.25), R::of(1e-30)]);
    let mut subnormal_pairs = 0;
    for &m0 in &values {
        for &m1 in &values {
            let kept = form([m0, m1, R::one() - m0 - m1]).degenerate;
            let flushed = form([m0.ftz(), m1.ftz(), R::one() - m0 - m1]).degenerate;
            assert_eq!(
                flushed,
                kept,
                "flushing moves the tag at {}, m = ({m0:?}, {m1:?})",
                R::NAME
            );
            subnormal_pairs += usize::from(m0.ftz() != m0 || m1.ftz() != m1);
        }
    }
    for &m0 in &values {
        for &m1 in &values {
            check_point(form, m0, m1).unwrap();
        }
    }
    assert!(subnormal_pairs > 0, "no subnormal mass at {}", R::NAME);
}

#[test]
fn qa_m48_flush_keeps_tag() {
    check_flush::<f64>(mass::from_masses);
    check_flush::<f32>(mass::from_masses);
}

/// A form with an intermediate a flush can zero: `M₀₁ = m₀·(1 + m₁/m₀)`, its tag read from it.
#[cfg(feature = "controls")]
fn ratio_form(m: [f32; 3]) -> Masses<f32> {
    let m01 = m[0] * (1.0 + m[1] / m[0]);
    Masses {
        degenerate: (m01 < mass::m01_epsilon::<f32>()).then_some(Degenerate::M01Tiny),
        ..mass::from_masses(m)
    }
}

negative_control!(
    qa_m48_flush_keeps_tag,
    "a form dividing by a mass a flush zeroes must move the tag",
    expected = "flushing moves the tag",
    check_flush::<f32>(ratio_form)
);
