//! `tanh_sat`, the bound dd_generation_root §3.9's `tanh` operator clamps its argument to (an R-72 definition;
//! REQ-DEC-047's proposal, 20, provisional until the M2 gate), on the CPU at f32 and f64:
//! - `tanh_sat_keeps_every_bit`: `tanh` is exactly ±1 at `±tanh_sat` and at every argument past it, at both widths, so
//!   clamping the argument changes no CPU value; and each link whose control enters only through `tanh` (`tanh_alpha`,
//!   `tanh_beta`, `softmax_tanh`) gives, past the bound, the bits it gives at the bound, a NaN control giving NaN.
//!   (`stick_breaking` also reads its control through `σ(2z)`, so its value moves past the bound; its `tanh` terms are
//!   the first check's.)
//! - `tanh_sat_operator_is_tanh`: the operator itself, the clamp and then the select to ±1 where `|tanh| > 1`, is `tanh`
//!   bit for bit at every f32 (strided over the bit patterns, the signs, zeros, subnormals, `±∞` and NaN included) and
//!   over an f64 sweep from 10⁻³⁰⁰ to `MAX`: the CPU's `tanh` stays in `[−1, 1]`, so the select never acts there.
//! - `tanh_sat_below_metal`: the bound lies below 43.671875, from which Metal's GPU `tanh` was measured to fail (0, then
//!   NaN; TASK-M2-06's probe), so the GPU never evaluates `tanh` where it fails.
//!
//! The decode stage on the GPU at saturated controls is `engine`'s `decode_stage_identities`.

use kernel::generated::constants::tanh_sat;
use kernel::generated::links::{Link, LinkReal, SoftmaxTanh, TanhAlpha, TanhBeta};
use validation::negative_control;

/// One float width: its conversions, and the arguments past `t` the check walks.
trait Width: LinkReal {
    fn of(x: f64) -> Self;
    fn wide(self) -> f64;
    fn bits(self) -> u64;
    /// Arguments at and past `t`: every f32 bit pattern from `t` up, in steps of `stride`, with `MAX` and `∞`; at f64, a
    /// geometric sweep of `count` points from `t` to `MAX`, with each one's neighbour above.
    fn past(t: Self) -> Vec<Self>;
}

impl Width for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn wide(self) -> f64 {
        f64::from(self)
    }
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
    fn past(t: Self) -> Vec<Self> {
        let mut v: Vec<f32> = (t.to_bits()..f32::INFINITY.to_bits())
            .step_by(997)
            .map(f32::from_bits)
            .collect();
        v.extend([t, f32::from_bits(t.to_bits() + 1), f32::MAX, f32::INFINITY]);
        v
    }
}

impl Width for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn wide(self) -> f64 {
        self
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn past(t: Self) -> Vec<Self> {
        let count = 200_000;
        let step = (f64::MAX.ln() - t.ln()) / f64::from(count);
        let mut v: Vec<f64> = (0..count)
            .flat_map(|i| {
                let x = (t.ln() + step * f64::from(i)).exp().max(t);
                [x, f64::from_bits(x.to_bits() + 1)]
            })
            .collect();
        v.extend([t, f64::MAX, f64::INFINITY]);
        v
    }
}

/// A link's forward on one control, the others 0.
type Forward<R> = fn(R) -> Vec<R>;

fn links<R: LinkReal>() -> [(&'static str, Forward<R>); 4] {
    [
        ("tanh_alpha", |x| TanhAlpha::forward([x]).to_vec()),
        ("tanh_beta", |x| TanhBeta::forward([x]).to_vec()),
        ("softmax_tanh z₁", |x| {
            SoftmaxTanh::forward([x, R::zero()]).to_vec()
        }),
        ("softmax_tanh z₂", |x| {
            SoftmaxTanh::forward([R::zero(), x]).to_vec()
        }),
    ]
}

/// At the bound `t` and past it, on both sides, `tanh` is exactly ±1; each link's bits past `t` are its bits at `t`;
/// a NaN control gives NaN.
fn check_bound<R: Width>(t: R) {
    let at = R::NAME;
    let one = R::one();
    for x in R::past(t) {
        assert_eq!(
            x.tanh().bits(),
            one.bits(),
            "tanh({:e}) is not 1 at {at}",
            x.wide()
        );
        assert_eq!(
            (-x).tanh().bits(),
            (-one).bits(),
            "tanh({:e}) is not −1 at {at}",
            -x.wide()
        );
    }
    for (name, f) in links::<R>() {
        for s in [one, -one] {
            let edge: Vec<u64> = f(s * t).into_iter().map(Width::bits).collect();
            for x in [44.0, 44.5, 1e3, 1e30]
                .map(R::of)
                .into_iter()
                .chain([R::max_value(), R::infinity()])
            {
                let got: Vec<u64> = f(s * x).into_iter().map(Width::bits).collect();
                assert_eq!(
                    got,
                    edge,
                    "{name} at {:e} differs from its value at the bound, {at}",
                    (s * x).wide()
                );
            }
        }
        assert!(
            f(R::nan()).iter().all(|y| y.is_nan()),
            "{name} of NaN is not NaN, {at}"
        );
    }
}

#[test]
fn tanh_sat_keeps_every_bit() {
    check_bound::<f32>(tanh_sat::<f32>());
    check_bound::<f64>(tanh_sat::<f64>());
}

negative_control!(
    tanh_sat_keeps_every_bit,
    "a bound of 9, where f64's tanh is not yet 1, must fail",
    expected = "is not 1 at f64",
    check_bound::<f64>(9.0)
);

/// The first argument at which Metal's GPU `tanh` was measured to fail (0 there, NaN further on).
const METAL_ONSET: f64 = 43.671875;

/// `t` lies below [`METAL_ONSET`].
fn check_below_metal(t: f64) {
    assert!(
        t < METAL_ONSET,
        "tanh_sat = {t} is not below 43.671875, where Metal's GPU tanh fails"
    );
}

#[test]
fn tanh_sat_below_metal() {
    check_below_metal(tanh_sat::<f64>());
    check_below_metal(f64::from(tanh_sat::<f32>()));
}

negative_control!(
    tanh_sat_below_metal,
    "a bound at Metal's onset must fail",
    expected = "is not below 43.671875",
    check_below_metal(METAL_ONSET)
);

/// `op` is `tanh` bit for bit, a NaN for a NaN, at every f32 in steps of 211 bit patterns (both signs, with `±0`, the
/// smallest subnormal, `±MAX`, `±∞` and NaN) and over an f64 sweep from 10⁻³⁰⁰ to `MAX` and its negation, and `tanh`
/// itself stays in `[−1, 1]` there.
fn check_operator(op32: fn(f32) -> f32, op64: fn(f64) -> f64) {
    let mut xs32: Vec<f32> = (0..=u32::MAX).step_by(211).map(f32::from_bits).collect();
    xs32.extend([
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ]);
    for x in xs32 {
        let (got, want) = (op32(x), x.tanh());
        assert!(
            got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
            "the operator at {x:e} is {got:e}, not tanh's {want:e}, at f32"
        );
        assert!(
            want.is_nan() || want.abs() <= 1.0,
            "tanh({x:e}) = {want:e} at f32"
        );
    }
    let count = 400_000;
    let (lo, hi) = (1e-300f64.ln(), f64::MAX.ln());
    for i in 0..=count {
        let x = (lo + (hi - lo) * f64::from(i) / f64::from(count))
            .exp()
            .min(f64::MAX);
        for x in [x, -x] {
            let (got, want) = (op64(x), x.tanh());
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "the operator at {x:e} is {got:e}, not tanh's {want:e}, at f64"
            );
            assert!(want.abs() <= 1.0, "tanh({x:e}) = {want:e} at f64");
        }
    }
    assert!(
        op64(f64::NAN).is_nan(),
        "the operator of NaN is not NaN at f64"
    );
}

#[test]
fn tanh_sat_operator_is_tanh() {
    check_operator(
        kernel::generated::links::tanh_sat::<f32>,
        kernel::generated::links::tanh_sat::<f64>,
    );
}

negative_control!(
    tanh_sat_operator_is_tanh,
    "an operator clamping at 9, where f64's tanh is not yet 1, must fail",
    expected = "not tanh's",
    check_operator(kernel::generated::links::tanh_sat::<f32>, |x| x
        .clamp(-9.0, 9.0)
        .tanh())
);
