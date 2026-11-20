//! QA tests for TASK-M2-02: the M01_TINY branch input is the one rounded sum `m₀ + m₁` (dd_decoder §3.1 "The branch
//! input"; REQ-DEC-011; R-34), bit for bit, at f64 and f32, whatever form the block computes it in.
//!
//! §3.1 fixes `M₀₁` as "the one rounded sum of the emitted masses `m₀ + m₁`", compared strictly against `ε_M01`. The
//! oracle is independent of the block: at f64 the IEEE add `m₀ + m₁` (Rust never contracts it); at f32 the exact
//! f64 sum of the two f32 masses rounded once to f32, which equals the correctly rounded f32 sum because f64 carries
//! more than `2·24 + 2` bits, so the double rounding is innocuous (Figueroa, "When is double rounding innocuous?",
//! 1995). The inputs are chosen where a different form differs by one rounding: `m₁` below half an ulp of `m₀`,
//! round-half-even ties, subnormals, `ε` and its neighbours, simplex points set directly, and `stick_breaking`'s
//! bisected onset crossings, where one rounding decides the tag. Each control (R-176) swaps in a form one rounding
//! away and must fail.

use kernel::decode::degenerate::Degenerate;
use kernel::decode::mass::{self, Masses};
use kernel::generated::links::{
    Defaults, Link, LinkReal, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use proptest::prelude::*;
use validation::{negative_control, prop};

type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

trait Fp: LinkReal + std::fmt::Debug {
    fn of(x: f64) -> Self;
    fn bits(self) -> u64;
    /// The once-rounded sum `a + b` at this precision, computed independently of the block.
    fn oracle_sum(a: Self, b: Self) -> Self;
    fn down(self) -> Self;
    fn up(self) -> Self;
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
}

/// `out` carries `M₀₁` bit-equal to the once-rounded `m₀ + m₁` of its own masses, and the tag is exactly
/// `M₀₁ < ε`.
fn check<R: Fp>(out: &Masses<R>, at: &str) -> Result<(), String> {
    let want = R::oracle_sum(out.m[0], out.m[1]);
    if out.m01.bits() != want.bits() {
        return Err(format!(
            "M₀₁ is not the one rounded sum at {at}, {}: {:?} (bits {:x}), m₀ + m₁ = {:?} (bits {:x})",
            R::NAME,
            out.m01,
            out.m01.bits(),
            want,
            want.bits()
        ));
    }
    let tag = (want < mass::m01_epsilon::<R>()).then_some(Degenerate::M01Tiny);
    if out.degenerate != tag {
        return Err(format!(
            "M₀₁ is not the one rounded sum at {at}, {}: tag {:?}, want {tag:?}",
            R::NAME,
            out.degenerate
        ));
    }
    Ok(())
}

/// Simplex points set directly, built where one rounding matters.
fn edge_points<R: Fp>() -> Vec<[R; 3]> {
    let one = R::one();
    let eps = mass::m01_epsilon::<R>();
    let half = R::of(0.5);
    let mut v = Vec::new();
    for m0 in [
        eps,
        eps.down(),
        eps.up(),
        R::of(1e-4),
        R::of(0.25),
        R::of(1.0 / 3.0),
        half,
        R::of(0.75).down(),
    ] {
        // ulp(m0)'s half (a tie), a quarter, three quarters, and just above and below the half.
        let ulp = m0.up() - m0;
        for m1 in [
            ulp * half,
            ulp * R::of(0.25),
            ulp * R::of(0.75),
            (ulp * half).up(),
            (ulp * half).down(),
            ulp * R::of(1.5),
            ulp * R::of(2.5),
            R::zero(),
        ] {
            v.push([m0, m1, one - m0 - m1]);
            v.push([m1, m0, one - m0 - m1]);
            // the odd neighbour, so ties round both ways
            let m0o = m0.up();
            v.push([m0o, m1, one - m0o - m1]);
        }
    }
    // subnormal and zero masses
    let tiny = R::min_positive_value() * R::of(0.5);
    v.push([R::zero(), R::zero(), one]);
    v.push([tiny, tiny, one]);
    v.push([R::min_positive_value(), tiny, one]);
    v
}

/// The block's `M₀₁` for a mass form under test: `from_masses`, or a control's.
fn check_sum<R: Fp>(form: fn([R; 3]) -> Masses<R>) {
    for m in edge_points::<R>() {
        check(&form(m), &format!("m = {m:?}")).unwrap();
    }
    let strat = (0.0..1.0f64, 0.0..1.0f64, -60i32..1);
    prop::run(&strat, |(a, b, e)| {
        // m₀ and m₁ on very different scales, so the sum is inexact
        let m0 = R::of(a);
        let m1 = R::of(b * 2f64.powi(e));
        let m = [m0, m1, R::one() - m0 - m1];
        check(&form(m), &format!("m = {m:?}")).map_err(TestCaseError::fail)
    });
}

#[test]
fn qa_m01_one_rounded_sum() {
    check_sum::<f64>(mass::from_masses::<f64>);
    check_sum::<f32>(mass::from_masses::<f32>);
}

/// `M₀₁ = 1 − m₂`: the formula's value, not its one rounded sum.
#[cfg(feature = "controls")]
fn one_minus_m2(m: [f32; 3]) -> Masses<f32> {
    let m01 = 1.0 - m[2];
    Masses {
        m01,
        degenerate: (m01 < mass::m01_epsilon::<f32>()).then_some(Degenerate::M01Tiny),
        ..mass::from_masses(m)
    }
}

negative_control!(
    qa_m01_one_rounded_sum,
    "M₀₁ formed as 1 − m₂ must fail",
    expected = "M₀₁ is not the one rounded sum",
    check_sum::<f32>(one_minus_m2)
);

/// `M₀₁` one rounding off: the sum rounded toward zero where it is inexact.
#[cfg(feature = "controls")]
fn toward_zero(m: [f64; 3]) -> Masses<f64> {
    let s = m[0] + m[1];
    let (big, small) = if m[0] >= m[1] {
        (m[0], m[1])
    } else {
        (m[1], m[0])
    };
    let err = small - (s - big);
    let m01 = if err < 0.0 { s.down() } else { s };
    Masses {
        m01,
        degenerate: (m01 < mass::m01_epsilon::<f64>()).then_some(Degenerate::M01Tiny),
        ..mass::from_masses(m)
    }
}

negative_control!(
    qa_m01_one_rounded_sum_ulp,
    "M₀₁ one ulp off the rounded sum must fail",
    expected = "M₀₁ is not the one rounded sum",
    check_sum::<f64>(toward_zero)
);

/// At `stick_breaking`'s onset crossings, bisected in `z₀` on lines `z₁ ∈ [6.3, 20.3]`, the block's `M₀₁` is the
/// once-rounded sum of its emitted masses, and its tag is that sum's. `m(z)` is the block under test.
fn check_crossings<R: Fp>(block: fn([R; 2]) -> Masses<R>) {
    let eps = mass::m01_epsilon::<R>();
    let mut fired = 0usize;
    for k in 0..2000u32 {
        let z1 = R::of(6.3 + 14.0 * f64::from(k) / 2000.0);
        let below = |z0: R| {
            let m = <StickBreaking as Link<2, 3>>::forward([z0, z1]);
            R::oracle_sum(m[0], m[1]) < eps
        };
        let (mut lo, mut hi) = (R::zero(), R::of(30.0));
        assert!(!below(lo) && below(hi), "no crossing on z₁ = {z1:?}");
        loop {
            let mid = (lo + hi) * R::of(0.5);
            if mid == lo || mid == hi {
                break;
            }
            if below(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        for z in [[lo, z1], [hi, z1]] {
            let out = block(z);
            assert_eq!(
                out.m,
                <StickBreaking as Link<2, 3>>::forward(z),
                "the block's masses are not the link's"
            );
            check(&out, &format!("z_μ = {z:?}")).unwrap();
            fired += usize::from(out.degenerate.is_some());
        }
    }
    // PIT-3: every pair straddles ε, so half the points fire.
    assert_eq!(
        fired,
        2000,
        "the crossings do not straddle ε at {}",
        R::NAME
    );
}

#[test]
fn qa_m01_crossings() {
    check_crossings::<f64>(mass::decode::<Stick, f64>);
    check_crossings::<f32>(mass::decode::<Stick, f32>);
    // the default link stays well above ε; the block still forms the same sum there
    for z in [[0.0f64, 0.0], [-40.0, 40.0], [40.0, -40.0], [3.0, 7.0]] {
        check(&mass::decode::<Defaults, f64>(z), &format!("{z:?}")).unwrap();
        let z32 = [z[0] as f32, z[1] as f32];
        check(&mass::decode::<Defaults, f32>(z32), &format!("{z:?}")).unwrap();
    }
}

/// The block with `M₀₁` read one ulp low wherever it is near ε: the tag flips at the crossings.
#[cfg(feature = "controls")]
fn low_at_crossing(z: [f32; 2]) -> Masses<f32> {
    let out = mass::decode::<Stick, f32>(z);
    let m01 = out.m01.down();
    Masses {
        m01,
        degenerate: (m01 < mass::m01_epsilon::<f32>()).then_some(Degenerate::M01Tiny),
        ..out
    }
}

negative_control!(
    qa_m01_crossings,
    "M₀₁ one ulp low at the crossings must fail",
    expected = "M₀₁ is not the one rounded sum",
    check_crossings::<f32>(low_at_crossing)
);
