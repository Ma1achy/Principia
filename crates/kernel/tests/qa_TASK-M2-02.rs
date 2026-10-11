//! QA tests for TASK-M2-02 (`kernel::decode`: the latent type, `D_mass`, `D_cfg`), written from the requirements and
//! dd_decoder §3.1, §3.2 and §5, not from the implementation. Each check runs at the f64 and the f32 instantiation
//! natively (R-265), behind function pointers, so each negative control (R-176) runs the same check on a deliberately
//! broken block.
//!
//! - REQ-SYS-010: the latent type is exactly 8 controls, `z[0:2]` config, `z[2:6]` momentum, `z[6:8]` mass; the
//!   config block's output does not depend on the masses' controls (dd_decoder §5 test 2) and is in the canonical
//!   frame, `ρ̃` exactly on `+x`.
//! - REQ-DEC-010: the masses are `softmax(0, μ₁, μ₂)`, `μₖ = μ_max·tanh(z_μk)`, `μ_max = 5` (R-10), evaluated here in
//!   f64 from the formula; `z_μ = 0` gives `(⅓, ⅓, ⅓)`; the Burrau masses 3, 4, 5 (and a second, unsymmetric
//!   triple) give `(c, b, a)/(a+b+c)`; `Σmᵢ = 1`.
//! - REQ-DEC-011 (with REQ-DEC-039's ε): the tag is `M01_TINY` exactly when `m₀ + m₁ < ε`, strict, over fuzzed
//!   `z_μ` with saturation, over simplex points set directly (the ternary plot, the δm strip), and at `ε` and its
//!   neighbours; tagged outputs keep the link's masses, finite, and their configuration decodes finite with `I = 1`;
//!   `stick_breaking`'s `m₂ → 1` corner reaches the tag from finite `z`; `softmax_tanh` stays above its `M₀₁` floor.
//!   ε is the chart constant `ε_M01` read from dd_decoder §3.1's text (REQ-DEC-039's proposal, REQ-DEC-009), never
//!   from the block: the block's `m01_epsilon` must equal it, and every tag is checked against it. The corner's
//!   pile-up `ε_μ − ε_μ²/4` (dd_generation_root §3.9) sits at least tenfold below ε and outside the f32 rounding band
//!   (§3.1, "a tenfold margin at either precision"); on the diagonal the onset of the tag, at f64 and at f32, is where
//!   §3.9's `stick_breaking` formula, evaluated here in f64, crosses ε, within that precision's rounding band
//!   ([`K_SB`]); and over `z_μ ∈ [3, 20]²` the f64 and f32 blocks give the same tag wherever the formula's `M₀₁` lies
//!   outside the f32 band.
//! - REQ-DEC-012: `α = (π/2)σ(z_α)`, `β = πσ(z_β)`, `ρ̃ = cos α·(1, 0)`, `λ̃ = sin α·(cos β, sin β)` against the
//!   formula in f64; `z = 0` gives `α = π/4`, `β = π/2`, `‖ρ̃‖ = ‖λ̃‖`; `‖ρ̃‖² + ‖λ̃‖² = 1`; ranges.
//! - REQ-DEC-013: `r₁ − r₀ = ρ`, `r₂ − r₀₁ = λ`, CoM `= 0`, `I = 1`, and decoder test 6 (from the emitted `(m, rᵢ)`
//!   alone, §3.2's definitions give back `α`, `β` and `R̃ = 1`), over link-produced and directly set masses.
//! - REQ-DEC-015: decoder test 9, over a fine `z_α` sweep at several mass points, skewed and tagged ones included.
//! - REQ-INT-002: `G = 1`, `M = 1`, `I = 1`.
//!
//! **Tolerances.** Test 1's `Σmᵢ`, CoM and `I` and test 6's are REQ-DEC-044 (calibrated; the PR's proposal, pending
//! the M2 gate): `Σmᵢ − 1` within 4 eps, the CoM within 4 eps of `Σmᵢ‖rᵢ‖`, `I − 1` within 8 eps, and test 6's
//! `|R̃ − 1|`, `|Δα|`, `sin α·|Δβ|` and `|ρ̃_y|` within 8 eps, `eps` the precision's machine epsilon. The checks of
//! the formulae against an f64 evaluation have no calibration requirement: their bounds are the round-off of the
//! operations the formula names, each derived beside its constant, never fitted. The f32 behaviour at a saturated
//! `α` or `β` (the rounded `π/2` and `π` past the exact poles) is an open question: no check here pins it either way;
//! the ranges are asserted closed against the precision's own `π/2` and `π` on the whole domain, and open only on
//! the interior.

use std::fmt::Debug;

use kernel::decode::config::{self, Config};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::latent::Latent;
use kernel::decode::mass::{self, Masses};
use kernel::decode::units;
use kernel::generated::links::{
    Defaults, Link, LinkReal, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use proptest::prelude::*;
use validation::{negative_control, prop};

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

trait Fp: LinkReal + Debug {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    /// The machine epsilon, as f64.
    const EPS: f64;
    /// The largest finite magnitude, as f64: the widest control the precision can hold.
    const MAX: f64;
    /// The next value below `self`.
    fn down(self) -> Self;
    /// The next value above `self`, for `self > 0`.
    fn up(self) -> Self;
}

impl Fp for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    const EPS: f64 = f64::EPSILON;
    const MAX: f64 = f64::MAX;
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
    fn f(self) -> f64 {
        self as f64
    }
    const EPS: f64 = f32::EPSILON as f64;
    const MAX: f64 = f32::MAX as f64;
    fn down(self) -> Self {
        f32::from_bits(self.to_bits() - 1)
    }
    fn up(self) -> Self {
        f32::from_bits(self.to_bits() + 1)
    }
}

// ── REQ-DEC-044's proposed tolerances, in eps ───────────────────────────────────────────────────────────────────

/// Test 1, `Σmᵢ = 1`.
const TOL_SUM: f64 = 4.0;
/// Test 1, CoM `= 0`, relative to `Σmᵢ‖rᵢ‖`.
const TOL_COM: f64 = 4.0;
/// Test 1, `I = 1`.
const TOL_I: f64 = 8.0;
/// Test 6, the reconstruction inverse.
const TOL_INV: f64 = 8.0;

/// `μ_max`, R-10.
const MU_MAX: f64 = 5.0;

// ── The blocks under test ───────────────────────────────────────────────────────────────────────────────────────

struct Blocks<R: 'static> {
    name: &'static str,
    mass: fn([R; 2]) -> Masses<R>,
    from_masses: fn([R; 3]) -> Masses<R>,
    config: fn([R; 2], &Masses<R>) -> Config<R>,
}

impl<R> Clone for Blocks<R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R> Copy for Blocks<R> {}

fn blocks<S: Selection, R: Fp>(name: &'static str) -> Blocks<R> {
    Blocks {
        name,
        mass: mass::decode::<S, R>,
        from_masses: mass::from_masses::<R>,
        config: config::decode::<S, R>,
    }
}

type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

fn defaults<R: Fp>() -> Blocks<R> {
    blocks::<Defaults, R>("softmax_tanh")
}

fn stick<R: Fp>() -> Blocks<R> {
    blocks::<Stick, R>("stick_breaking")
}

// ── The domain: any finite z (R-407) ────────────────────────────────────────────────────────────────────────────

/// One control at precision `R`: the centre, the transition, saturation, and log-uniform magnitudes up to the
/// precision's largest finite value. Drawn as `R`, so the f64 reference sees exactly the control the block saw.
fn control<R: Fp>() -> BoxedStrategy<f64> {
    let top = R::MAX.log10();
    prop_oneof![
        -3.0f64..=3.0,
        -25.0f64..=25.0,
        (any::<bool>(), -3.0f64..=top)
            .prop_map(|(neg, e)| if neg { -1.0 } else { 1.0 } * 10f64.powf(e)),
        Just(R::MAX),
        Just(-R::MAX),
        Just(0.0),
    ]
    .prop_map(|x| R::of(x).f())
    .boxed()
}

/// A simplex point set directly (the ternary plot or the δm strip), each mass log-uniform from 10⁻¹² to 1 before
/// normalisation, so tiny `M₀₁` on either side of ε is drawn often; each mass rounded to `R`, so `Σmᵢ = 1` to the
/// rounding of three values, as a chart that sets masses delivers them.
fn simplex<R: Fp>() -> BoxedStrategy<[f64; 3]> {
    [-12.0f64..=0.0, -12.0f64..=0.0, -12.0f64..=0.0]
        .prop_map(|e| {
            let w = e.map(|x| 10f64.powf(x));
            let s: f64 = w.iter().sum();
            w.map(|x| R::of(x / s).f())
        })
        .boxed()
}

/// `x ≤ t`, false for NaN.
fn within(x: f64, t: f64) -> bool {
    x <= t
}

fn at<R: Fp, const N: usize>(z: [f64; N]) -> [R; N] {
    z.map(R::of)
}

fn fx<R: Fp, const N: usize>(v: [R; N]) -> [f64; N] {
    v.map(Fp::f)
}

/// Fixed controls at R: saturation and the centre at each sign.
fn fixed<R: Fp>() -> Vec<f64> {
    [
        -R::MAX,
        -1e30,
        -100.0,
        -40.0,
        -12.0,
        -3.0,
        -0.5,
        0.0,
        0.5,
        3.0,
        12.0,
        40.0,
        100.0,
        1e30,
        R::MAX,
    ]
    .iter()
    .map(|&x| R::of(x).f())
    .collect()
}

fn fixed_pairs<R: Fp>() -> Vec<[f64; 2]> {
    let f = fixed::<R>();
    f.iter()
        .flat_map(|&a| f.iter().map(move |&b| [a, b]))
        .collect()
}

// ── REQ-SYS-010: the latent type ────────────────────────────────────────────────────────────────────────────────

type Acc<const N: usize> = fn(&Latent<f64>) -> [f64; N];

fn check_layout(config: Acc<2>, momentum: Acc<4>, mass: Acc<2>) {
    assert_eq!(
        std::mem::size_of::<Latent<f64>>(),
        8 * 8,
        "layout: Latent<f64> is not 8 controls"
    );
    assert_eq!(
        std::mem::size_of::<Latent<f32>>(),
        8 * 4,
        "layout: Latent<f32> is not 8 controls"
    );
    let z = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
    let l = Latent::new(z);
    let all: [f64; 8] = l.z;
    assert_eq!(all, z, "layout: Latent::new does not keep z in order");
    assert_eq!(
        config(&l),
        [10.0, 11.0],
        "layout: the config block is not z[0:2]"
    );
    assert_eq!(
        momentum(&l),
        [12.0, 13.0, 14.0, 15.0],
        "layout: the momentum block is not z[2:6]"
    );
    assert_eq!(
        mass(&l),
        [16.0, 17.0],
        "layout: the mass block is not z[6:8]"
    );
}

#[test]
fn qa_latent_layout() {
    check_layout(Latent::config, Latent::momentum, Latent::mass);
    let l = Latent::new([1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    assert_eq!(
        (l.config(), l.momentum(), l.mass()),
        ([1.0, 2.0], [3.0, 4.0, 5.0, 6.0], [7.0, 8.0])
    );
}

negative_control!(
    qa_latent_layout,
    "a layout with the mass block first must fail",
    expected = "layout: the config block is not z[0:2]",
    check_layout(
        |l| [l.z[2], l.z[3]],
        |l| [l.z[4], l.z[5], l.z[6], l.z[7]],
        |l| [l.z[0], l.z[1]]
    )
);

/// Decoder test 2 and the canonical frame: the config block's `α, β, ρ̃, λ̃` are bit-identical whatever the masses,
/// and `ρ̃_y` is exactly 0 (rotation fixed by the decode, not by a control).
fn check_factorised<R: Fp>(b: Blocks<R>) {
    let masses: Vec<Masses<R>> = [
        [0.0, 0.0],
        [3.0, -2.0],
        [-40.0, 40.0],
        [40.0, 40.0],
        [0.7, 0.1],
    ]
    .iter()
    .map(|&zm| (b.mass)(at::<R, 2>(zm)))
    .collect();
    for zc in fixed_pairs::<R>() {
        let base = (b.config)(at::<R, 2>(zc), &masses[0]);
        assert_eq!(
            base.rho_t[1],
            R::zero(),
            "{}: ρ̃ is off +x at z_cfg = {zc:?}",
            R::NAME
        );
        for m in &masses[1..] {
            let c = (b.config)(at::<R, 2>(zc), m);
            assert!(
                c.alpha == base.alpha
                    && c.beta == base.beta
                    && c.rho_t == base.rho_t
                    && c.lambda_t == base.lambda_t,
                "{}: the config block couples to the masses at z_cfg = {zc:?}, m = {:?}",
                R::NAME,
                m.m
            );
        }
    }
}

#[test]
fn qa_factorised() {
    check_factorised(defaults::<f64>());
    check_factorised(defaults::<f32>());
    check_factorised(stick::<f64>());
    check_factorised(stick::<f32>());
}

#[cfg(feature = "controls")]
fn coupled(z: [f64; 2], m: &Masses<f64>) -> Config<f64> {
    let mut c = config::decode::<Defaults, f64>(z, m);
    c.alpha = c.alpha * 3.0 * m.m[0];
    c
}

negative_control!(
    qa_factorised,
    "an α that depends on m₀ must fail",
    expected = "couples to the masses",
    check_factorised(Blocks {
        config: coupled,
        ..defaults::<f64>()
    })
);

// ── REQ-DEC-010: the mass block ─────────────────────────────────────────────────────────────────────────────────

/// `softmax(0, μ₁, μ₂)`, `μₖ = μ_max·tanh(zₖ)`, in f64.
fn masses_ref(z: [f64; 2]) -> [f64; 3] {
    let e = z.map(|x| (MU_MAX * x.tanh()).exp());
    let s = 1.0 + e[0] + e[1];
    [1.0 / s, e[0] / s, e[1] / s]
}

/// Each mass's relative error bound, in eps: `tanh` within 1 ulp and the product `μ_max·t` rounded give
/// `|δμ| ≤ 2μ_max·eps`; each `e^μ` then carries `|δμ| + eps` relative; a term over the sum of such terms at most
/// doubles that; the add and the divide round twice more. `2(2μ_max + 1) + 2 = 4μ_max + 4`.
const TOL_MASS_REL: f64 = 4.0 * MU_MAX + 4.0;

fn check_mass_formula<R: Fp>(b: Blocks<R>, z: [f64; 2]) -> Result<(), String> {
    let out = (b.mass)(at::<R, 2>(z));
    let want = masses_ref(z);
    for (i, (mi, wi)) in out.m.iter().zip(want).enumerate() {
        let err = (mi.f() - wi).abs() / (wi * R::EPS);
        if !within(err, TOL_MASS_REL) {
            return Err(format!(
                "mass formula: m{i} = {:e} ≠ softmax(0, μ₁, μ₂) = {:e} ({err} eps) at z_μ = {z:?}, {}",
                mi.f(),
                wi,
                R::NAME
            ));
        }
    }
    let sum: f64 = out.m.iter().map(|x| x.f()).sum();
    if !within((sum - 1.0).abs(), TOL_SUM * R::EPS) {
        return Err(format!(
            "mass sum: Σm − 1 = {:e} at z_μ = {z:?}, {}",
            sum - 1.0,
            R::NAME
        ));
    }
    Ok(())
}

fn check_mass_all<R: Fp>(b: Blocks<R>) {
    for z in fixed_pairs::<R>() {
        check_mass_formula(b, z).unwrap();
    }
    prop::run(&[control::<R>(), control::<R>()], |z| {
        check_mass_formula(b, z).map_err(TestCaseError::fail)
    });
}

#[test]
fn qa_mass_formula() {
    check_mass_all(defaults::<f64>());
    check_mass_all(defaults::<f32>());
}

/// The IC Inspector's uncorrected `μ_max = 4` (R-10).
#[cfg(feature = "controls")]
struct MuMaxFour;

#[cfg(feature = "controls")]
impl Link<2, 3> for MuMaxFour {
    const NAME: &'static str = "mu_max_four";
    type Codomain = kernel::generated::links::Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let a = (R::LIT_5 * v[0].tanh()).exp();
        let b = (R::LIT_5 * v[1].tanh()).exp();
        let s = R::one() + a + b;
        [R::one() / s, a / s, b / s]
    }
    fn inverse<R: LinkReal>(_: [R; 3]) -> [R; 2] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 2]) -> R {
        unreachable!()
    }
}

negative_control!(
    qa_mass_formula,
    "a mass link at μ_max = 4 must fail the formula",
    expected = "mass formula",
    check_mass_all(blocks::<
        Select<MuMaxFour, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>,
        f64,
    >("mu_max_four"))
);

/// `z_μ = 0` gives `(⅓, ⅓, ⅓)`: each the correctly rounded third of a sum of three exact ones (`tanh 0 = 0`,
/// `e⁰ = 1`), within one ulp of ⅓, `eps/2`.
fn check_equal<R: Fp>(b: Blocks<R>) {
    let m = (b.mass)([R::zero(), R::zero()]);
    for (i, mi) in m.m.iter().enumerate() {
        assert!(
            (mi.f() - 1.0 / 3.0).abs() <= R::EPS / 2.0,
            "equal masses: m{i} = {:e} at z_μ = 0, {}",
            mi.f(),
            R::NAME
        );
    }
    assert!(m.degenerate.is_none(), "equal masses tagged at {}", R::NAME);
}

/// The Burrau point for masses `(a, b, c)`: `μ₁ = log(b/c)`, `μ₂ = log(a/c)`, so `zₖ = atanh(μₖ/μ_max)`, and the
/// masses are `(c, b, a)/(a+b+c)`. Bound: [`TOL_MASS_REL`], plus the rounding of `z` to `R`, which moves `μ` by at
/// most `μ_max·|z|·eps/2 < 2μ_max·eps` for these `|z| < 1`: `TOL_MASS_REL + 2μ_max`.
fn check_burrau<R: Fp>(b: Blocks<R>) {
    for (a, bb, c) in [(3.0, 4.0, 5.0), (1.0, 2.0, 7.0), (5.0, 1.0, 1.0)] {
        let z = [
            (f64::ln(bb / c) / MU_MAX).atanh(),
            (f64::ln(a / c) / MU_MAX).atanh(),
        ];
        let m = (b.mass)(at::<R, 2>(z));
        let want = [c, bb, a].map(|x| x / (a + bb + c));
        for (i, (mi, wi)) in m.m.iter().zip(want).enumerate() {
            let err = (mi.f() - wi).abs() / (wi * R::EPS);
            assert!(
                err <= TOL_MASS_REL + 2.0 * MU_MAX,
                "Burrau ({a}, {bb}, {c}): m{i} = {} ≠ {} ({err} eps), {}",
                mi.f(),
                wi,
                R::NAME
            );
        }
    }
}

#[test]
fn qa_mass_points() {
    check_equal(defaults::<f64>());
    check_equal(defaults::<f32>());
    check_burrau(defaults::<f64>());
    check_burrau(defaults::<f32>());
}

/// `softmax(μ₁, μ₂, 0)`: body 2 the reference.
#[cfg(feature = "controls")]
fn ref_two(z: [f64; 2]) -> Masses<f64> {
    let [m0, m1, m2] = kernel::generated::links::SoftmaxTanh::forward(z);
    mass::from_masses([m1, m2, m0])
}

negative_control!(
    qa_mass_points,
    "a mass block with body 2 the softmax reference must fail the Burrau point",
    expected = "Burrau (3, 4, 5)",
    check_burrau(Blocks {
        mass: ref_two,
        ..defaults::<f64>()
    })
);

// ── REQ-DEC-011: M01_TINY ───────────────────────────────────────────────────────────────────────────────────────

/// `ε_μ`, REQ-DEC-009.
const EPS_MU: f64 = 1e-6;

/// The `ε_M01` dd_decoder §3.1 states (REQ-DEC-039's proposal, provisional until the M2 gate): the number after
/// "`ε_M01 = 10" in §3.1's text, its exponent in superscript digits.
fn doc_eps_m01() -> f64 {
    static EPS: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *EPS.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/design/principia_dd_decoder.md");
        let text = std::fs::read_to_string(&path).expect("dd_decoder reads");
        let start = text.find("### 3.1 Mass").expect("dd_decoder has §3.1");
        let end = start + text[start..].find("### 3.2").expect("dd_decoder has §3.2");
        parse_eps_m01(&text[start..end]).expect("§3.1 states `ε_M01 = 10ⁿ`")
    })
}

/// `10ⁿ` from the first "`ε_M01 = 10" followed by a superscript exponent in `section`.
fn parse_eps_m01(section: &str) -> Option<f64> {
    const KEY: &str = "ε_M01 = 10";
    let at = section.find(KEY)? + KEY.len();
    let mut exp = String::new();
    for c in section[at..].chars() {
        let d = match c {
            '⁻' => '-',
            '⁰' => '0',
            '¹' => '1',
            '²' => '2',
            '³' => '3',
            '⁴' => '4',
            '⁵' => '5',
            '⁶' => '6',
            '⁷' => '7',
            '⁸' => '8',
            '⁹' => '9',
            _ => break,
        };
        exp.push(d);
    }
    let n: i32 = exp.parse().ok()?;
    Some(10f64.powi(n))
}

/// ε at `R`: §3.1's `ε_M01`, rounded to `R` as a compile-time constant is.
fn eps_m01<R: Fp>() -> R {
    R::of(doc_eps_m01())
}

/// The block's threshold is §3.1's `ε_M01` at both precisions (REQ-DEC-039, REQ-DEC-009), not `ε_μ`.
fn check_eps(doc: f64) {
    assert!(
        mass::m01_epsilon::<f64>() == doc && mass::m01_epsilon::<f32>() == doc as f32,
        "eps: the block's ε is {:e} (f64), {:e} (f32), not §3.1's ε_M01 = {doc:e}",
        mass::m01_epsilon::<f64>(),
        mass::m01_epsilon::<f32>()
    );
    assert!(doc != EPS_MU, "eps: §3.1's ε_M01 is ε_μ");
}

#[test]
fn qa_m01_epsilon() {
    assert_eq!(parse_eps_m01("x `ε_M01 = 10⁻¹²`, y"), Some(1e-12));
    check_eps(doc_eps_m01());
}

negative_control!(
    qa_m01_epsilon,
    "the old ε = ε_μ is not §3.1's ε_M01",
    expected = "eps: the block's ε",
    check_eps(EPS_MU)
);

/// The tag is `Some(M01_TINY)` exactly when `m₀ + m₁ < ε`, and the masses are finite and on the simplex.
fn check_tag<R: Fp>(out: &Masses<R>, at_z: &str) -> Result<(), String> {
    let m01 = out.m[0] + out.m[1];
    let want = if m01 < eps_m01::<R>() {
        Some(Degenerate::M01Tiny)
    } else {
        None
    };
    if out.degenerate != want {
        return Err(format!(
            "M01_TINY: tag {:?} with M₀₁ = {:e} against ε = {:e} at {at_z}, {}",
            out.degenerate,
            m01.f(),
            eps_m01::<R>().f(),
            R::NAME
        ));
    }
    if out.m.iter().any(|x| !x.is_finite()) || out.m01.f() != m01.f() {
        return Err(format!(
            "M01_TINY: the output is not kept: {out:?} at {at_z}, {}",
            R::NAME
        ));
    }
    Ok(())
}

/// A tagged output is kept and decodes: its configuration is finite and has `I = 1`.
fn check_tagged_decodes<R: Fp>(b: Blocks<R>, m: &Masses<R>, at_z: &str) -> Result<(), String> {
    if m.degenerate.is_none() {
        return Ok(());
    }
    for zc in [[0.0, 0.0], [-3.0, 2.0], [5.0, -5.0]] {
        let c = (b.config)(at::<R, 2>(zc), m);
        let r = c.r.map(fx);
        if r.iter().flatten().any(|x| !x.is_finite()) {
            return Err(format!(
                "M01_TINY: a tagged decode is not finite: {r:?} at {at_z}, {}",
                R::NAME
            ));
        }
        let i: f64 = (0..3)
            .map(|k| m.m[k].f() * (r[k][0].powi(2) + r[k][1].powi(2)))
            .sum();
        if !within((i - 1.0).abs(), TOL_I * R::EPS) {
            return Err(format!(
                "M01_TINY: a tagged decode has I − 1 = {:e} at {at_z}, {}",
                i - 1.0,
                R::NAME
            ));
        }
    }
    Ok(())
}

/// The link's own masses come out of the block unchanged: the tag adds, never alters.
fn check_kept<S: Selection, R: Fp>(b: Blocks<R>, z: [f64; 2]) -> Result<(), String> {
    let out = (b.mass)(at::<R, 2>(z));
    let link = <S::MassLink as Link<2, 3>>::forward(at::<R, 2>(z));
    if fx(out.m) != fx(link) {
        return Err(format!(
            "M01_TINY: the block's masses {:?} are not the link's {link:?}",
            out.m
        ));
    }
    let at_z = format!("{} z_μ = {z:?}", b.name);
    check_tag(&out, &at_z)?;
    check_tagged_decodes(b, &out, &at_z)
}

fn check_m01<R: Fp>(b: Blocks<R>, sb: Blocks<R>) {
    // Fuzzed and fixed z_μ, both links, saturation included.
    for z in fixed_pairs::<R>() {
        check_kept::<Defaults, R>(b, z).unwrap();
        check_kept::<Stick, R>(sb, z).unwrap();
    }
    prop::run(&[control::<R>(), control::<R>()], |z| {
        check_kept::<Defaults, R>(b, z).map_err(TestCaseError::fail)?;
        check_kept::<Stick, R>(sb, z).map_err(TestCaseError::fail)
    });
    // Directly set simplex points, M₀₁ on both sides of ε.
    prop::run(&simplex::<R>(), |m| {
        let out = (b.from_masses)(at::<R, 3>(m));
        let at_z = format!("m = {m:?}");
        check_tag(&out, &at_z).map_err(TestCaseError::fail)?;
        check_tagged_decodes(b, &out, &at_z).map_err(TestCaseError::fail)
    });
    // The boundary: M₀₁ = ε does not fire (strict), the next value below does; well below fires; well above not.
    let e = eps_m01::<R>();
    let one = R::one();
    let tiny = (e - e.down()) / R::of(8.0);
    for (m, fires) in [
        // m₁ an eighth of ε's ulp, so m₀ + m₁ rounds to m₀: exactly ε, then the value just below it.
        ([e, tiny, one - e], false),
        ([e.down(), tiny, one - e], true),
        ([e / R::of(4.0), e / R::of(4.0), one - e / R::of(2.0)], true),
        ([e, e, one - e - e], false),
        ([R::of(1e-4), R::of(1e-4), R::of(1.0 - 2e-4)], false),
    ] {
        let out = (b.from_masses)(m);
        let at_z = format!("m = {:?}", fx(m));
        check_tag(&out, &at_z).unwrap();
        assert_eq!(
            out.degenerate.is_some(),
            fires,
            "M01_TINY: the boundary at {at_z}, {}",
            R::NAME
        );
        check_tagged_decodes(b, &out, &at_z).unwrap();
    }
}

#[test]
fn qa_m01_tiny() {
    check_m01(defaults::<f64>(), stick::<f64>());
    check_m01(defaults::<f32>(), stick::<f32>());
}

/// A tag at `M₀₁ ≤ ε`, not `< ε`.
#[cfg(feature = "controls")]
fn at_or_below(m: [f64; 3]) -> Masses<f64> {
    let mut out = mass::from_masses(m);
    if out.m01 <= mass::m01_epsilon::<f64>() {
        out.degenerate = Some(Degenerate::M01Tiny);
    }
    out
}

negative_control!(
    qa_m01_tiny,
    "a tag firing at M₀₁ = ε must fail",
    expected = "M01_TINY: tag Some(M01Tiny)",
    check_m01(
        Blocks {
            from_masses: at_or_below,
            ..defaults::<f64>()
        },
        stick::<f64>()
    )
);

/// PIT-3, "check the measurement can fire": from finite `z`, `stick_breaking`'s `m₂ → 1` corner reaches the tag at
/// both precisions; `softmax_tanh` keeps `M₀₁` at or above its floor `(1 + e^−μ_max)/(1 + e^−μ_max + e^μ_max)`
/// (within [`TOL_MASS_REL`]) over the whole domain, so it never fires there while ε is below that floor.
fn check_reach<R: Fp>(b: Blocks<R>, sb: Blocks<R>) {
    for z in [[40.0, 40.0], [100.0, 100.0], [R::MAX, R::MAX]] {
        let out = (sb.mass)(at::<R, 2>(z));
        assert_eq!(
            out.degenerate,
            Some(Degenerate::M01Tiny),
            "reach: stick_breaking's corner does not fire at z_μ = {z:?}, M₀₁ = {:e}, {}",
            out.m01.f(),
            R::NAME
        );
    }
    let floor = (1.0 + (-MU_MAX).exp()) / (1.0 + (-MU_MAX).exp() + MU_MAX.exp());
    assert!(
        floor > eps_m01::<R>().f(),
        "reach: ε is above softmax_tanh's floor"
    );
    let check = |z: [f64; 2]| -> Result<(), String> {
        let out = (b.mass)(at::<R, 2>(z));
        let m01 = out.m[0].f() + out.m[1].f();
        if m01 < floor * (1.0 - TOL_MASS_REL * R::EPS) || out.degenerate.is_some() {
            return Err(format!(
                "reach: softmax_tanh M₀₁ = {m01:e} below its floor at z_μ = {z:?}, {}",
                R::NAME
            ));
        }
        Ok(())
    };
    for z in fixed_pairs::<R>() {
        check(z).unwrap();
    }
    prop::run(&[control::<R>(), control::<R>()], |z| {
        check(z).map_err(TestCaseError::fail)
    });
}

#[test]
fn qa_m01_tiny_reach() {
    check_reach(defaults::<f64>(), stick::<f64>());
    check_reach(defaults::<f32>(), stick::<f32>());
}

negative_control!(
    qa_m01_tiny_reach,
    "softmax_tanh in stick_breaking's place must fail to reach the tag",
    expected = "reach: stick_breaking's corner does not fire",
    check_reach(defaults::<f64>(), defaults::<f64>())
);

// ── REQ-DEC-039: ε_M01's margin, the onset, the two precisions ──────────────────────────────────────────────────

/// `stick_breaking`'s `M₀₁ = m₀ + m₁ = (1 − s) + s(1 − u)` from dd_generation_root §3.9's formula, in f64, with
/// `1 − s` and `1 − u` in §3.9's stable forms `σ(−2z) + ½ε_μ·tanh z`: no cancellation, so its relative error is a
/// few f64 eps, far inside every band below.
fn sb_m01_ref(z: [f64; 2]) -> f64 {
    let one_minus = |x: f64| 1.0 / (1.0 + (2.0 * x).exp()) + 0.5 * EPS_MU * x.tanh();
    let (ns, nu) = (one_minus(z[0]), one_minus(z[1]));
    ns + (1.0 - ns) * nu
}

/// The relative error bound of `stick_breaking`'s emitted `M₀₁`, in the precision's eps, from §3.9's stable forms:
/// `σ(−2z)` is an `exp`, an add and a divide (3), `½ε_μ·tanh z` a `tanh` and two products (3), their sum of two
/// positive terms one more, so `1 − s` and `1 − u` carry 4; `s = 1 − (1 − s)` carries 4 too (its own form, likewise);
/// the product `s(1 − u)` 4 + 4 + 1 = 9; the final add of two positive terms one more: 10. Doubled for the
/// transcendentals' libm error, which may reach 2 ulp at f32 rather than 1: `K_SB = 20`.
const K_SB: f64 = 20.0;

/// The doc's margin: the corner's pile-up `ε_μ − ε_μ²/4` (§3.9, §3.1) lies at least tenfold below ε, so it fires
/// "with a tenfold margin at either precision" (§3.1), and the gap between them is wider than the f32 band
/// `K_SB·eps₃₂·ε`, so no f32 rounding decides a corner tag. At `ε = ε_μ` the pile-up sat 2.5×10⁻⁷ relative below ε,
/// inside that band: the F1 split.
fn check_margin(eps: f64) {
    let floor = EPS_MU - EPS_MU * EPS_MU / 4.0;
    assert!(
        10.0 * floor <= eps,
        "margin: the corner's pile-up {floor:e} is not tenfold below ε = {eps:e}"
    );
    assert!(
        (eps - floor) / eps > K_SB * f32::EPS,
        "margin: the corner's pile-up {floor:e} is inside ε = {eps:e}'s f32 band"
    );
    // ...and the floors that must not fire are above it (§3.1): softmax_tanh's and the ternary plot's 2ε_m.
    let softmax = (1.0 + (-MU_MAX).exp()) / (1.0 + (-MU_MAX).exp() + MU_MAX.exp());
    assert!(
        softmax > eps && 2e-4 > eps,
        "margin: a floor that must not fire is below ε = {eps:e}"
    );
}

#[test]
fn qa_m01_margin() {
    check_margin(doc_eps_m01());
}

negative_control!(
    qa_m01_margin,
    "ε = ε_μ puts the corner's pile-up inside the band",
    expected = "margin: the corner's pile-up",
    check_margin(EPS_MU)
);

/// The tag a block gives at `z_μ`.
type TagAt<R> = fn([R; 2]) -> bool;

fn sb_tag<R: Fp>(z: [R; 2]) -> bool {
    (stick::<R>().mass)(z).degenerate == Some(Degenerate::M01Tiny)
}

/// `z*` on the diagonal where §3.9's `M₀₁(z, z)` crosses `eps`, by bisection in f64 (`M₀₁` falls with `z`).
fn onset_ref(eps: f64) -> f64 {
    let (mut lo, mut hi) = (0.0f64, 40.0f64);
    assert!(sb_m01_ref([lo, lo]) > eps && sb_m01_ref([hi, hi]) < eps);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if sb_m01_ref([mid, mid]) < eps {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// The relative band around ε within which `R`'s rounding may decide a tag: the block's [`K_SB`]·eps plus the f64
/// reference's own error, at most [`K_SB`] f64 eps.
fn band<R: Fp>() -> f64 {
    K_SB * (R::EPS + f64::EPSILON)
}

/// On the diagonal, the tag's onset at `R` is the formula's: across every `R` value of `z` from `z* − w` to
/// `z* + w`, `w` the band's width in `z` (from the formula's slope) plus two `R` ulps of `z`, the tag is `M₀₁ < ε`
/// of §3.9's `M₀₁` wherever that lies outside the band; it is off at the window's bottom and on at its top, so the
/// onset is inside it. The window is narrow because the slope is: the onset is where `M₀₁` still changes with `z`.
fn check_onset<R: Fp>(tag: TagAt<R>, eps: f64) {
    let zs = onset_ref(eps);
    let h = 1e-4;
    let slope = (sb_m01_ref([zs + h, zs + h]) - sb_m01_ref([zs - h, zs - h])) / (2.0 * h);
    let ulp = R::of(zs).up().f() - R::of(zs).f();
    let w = band::<R>() * eps / slope.abs() + 2.0 * ulp;
    let (lo, hi) = (R::of(zs - w), R::of(zs + w));
    assert!(
        !tag([lo, lo]) && tag([hi, hi]),
        "onset: the tag at {} does not switch on within z* = {zs} ± {w:e} (ε = {eps:e})",
        R::NAME
    );
    let mut z = lo;
    let mut n = 0;
    while z.f() <= hi.f() {
        let m = sb_m01_ref([z.f(), z.f()]);
        if (m - eps).abs() > band::<R>() * eps {
            assert_eq!(
                tag([z, z]),
                m < eps,
                "onset: at z = {:e}, {}, the formula's M₀₁ = {m:e} against ε = {eps:e}",
                z.f(),
                R::NAME
            );
        }
        z = z.up();
        n += 1;
        assert!(n < 1_000_000, "onset: the window is not narrow");
    }
}

#[test]
fn qa_m01_onset() {
    check_onset::<f64>(sb_tag::<f64>, doc_eps_m01());
    check_onset::<f32>(sb_tag::<f32>, doc_eps_m01());
}

/// A tag at `M₀₁ < ε·(1 + 10⁻⁴)`: an onset moved by rounding-sized amounts many times over.
#[cfg(feature = "controls")]
fn sb_tag_shifted(z: [f32; 2]) -> bool {
    (stick::<f32>().mass)(z).m01.f() < doc_eps_m01() * (1.0 + 1e-4)
}

negative_control!(
    qa_m01_onset,
    "an f32 tag whose onset is not the formula's must fail",
    expected = "onset:",
    check_onset::<f32>(sb_tag_shifted, doc_eps_m01())
);

/// Over `z_μ ∈ [3, 20]²` (the corner's approach; fuzzed and on a grid), the f64 and f32 blocks give the same tag at
/// the same (f32) input wherever §3.9's `M₀₁` there lies outside the f32 band around ε.
fn check_precisions(t64: TagAt<f64>, t32: TagAt<f32>) {
    let eps = doc_eps_m01();
    let check = |z: [f64; 2]| -> Result<(), String> {
        let z32 = at::<f32, 2>(z);
        let z = fx(z32);
        let m = sb_m01_ref(z);
        if (m - eps).abs() <= band::<f32>() * eps {
            return Ok(());
        }
        let (a, b) = (t64(z), t32(z32));
        if a != b || a != (m < eps) {
            return Err(format!(
                "precisions: at z_μ = {z:?} the f64 tag is {a}, the f32 {b}; the formula's M₀₁ = {m:e} against ε = {eps:e}"
            ));
        }
        Ok(())
    };
    for i in 0..=17 * 32 {
        for j in 0..=17 * 32 {
            check([3.0 + i as f64 / 32.0, 3.0 + j as f64 / 32.0]).unwrap();
        }
    }
    prop::run(&[3.0f64..=20.0, 3.0f64..=20.0], |z| {
        check([z[0], z[1]]).map_err(TestCaseError::fail)
    });
}

#[test]
fn qa_m01_precisions() {
    check_precisions(sb_tag::<f64>, sb_tag::<f32>);
}

/// An f32 tag at the old ε = ε_μ.
#[cfg(feature = "controls")]
fn sb_tag_eps_mu(z: [f32; 2]) -> bool {
    (stick::<f32>().mass)(z).m01 < EPS_MU as f32
}

negative_control!(
    qa_m01_precisions,
    "an f32 tag at ε_μ disagrees with f64's at ε_M01",
    expected = "precisions:",
    check_precisions(sb_tag::<f64>, sb_tag_eps_mu)
);

// ── REQ-DEC-012: the configuration block ────────────────────────────────────────────────────────────────────────

fn sigma(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// `|Δα|` and `|Δβ|` against the f64 formula, in eps of the angle's range: σ is `exp`, an add and a divide (its
/// relative error at most 1 ulp + two roundings, 2 eps), times the rounded constant `π/2` or `π` and one more
/// rounding (1 eps): 3 eps of the value, at most 3 eps of the range; 4 with a half-ulp of slack per step. Measured
/// in absolute terms: where f32's `e^−z` overflows, σ is 0 against f64's 10⁻⁴⁴.
const TOL_ANGLE: f64 = 4.0;

/// `ρ̃, λ̃` against the formula at the block's own `(α, β)`: `cos`/`sin` within 1 ulp each, and one product, at most
/// 2 eps on unit-bounded components.
const TOL_TRIG: f64 = 2.0;

fn check_cfg_formula<R: Fp>(b: Blocks<R>, zc: [f64; 2]) -> Result<(), String> {
    let m = (b.mass)([R::zero(), R::zero()]);
    let c = (b.config)(at::<R, 2>(zc), &m);
    let half_pi = std::f64::consts::FRAC_PI_2;
    let pi = std::f64::consts::PI;
    let (a, be) = (c.alpha.f(), c.beta.f());
    let ea = (a - half_pi * sigma(zc[0])).abs() / (half_pi * R::EPS);
    let eb = (be - pi * sigma(zc[1])).abs() / (pi * R::EPS);
    if !(within(ea, TOL_ANGLE) && within(eb, TOL_ANGLE)) {
        return Err(format!(
            "config formula: (α, β) = ({a}, {be}) ≠ ((π/2)σ, πσ) by ({ea}, {eb}) eps at z_cfg = {zc:?}, {}",
            R::NAME
        ));
    }
    if c.rho_t[1] != R::zero() {
        return Err(format!(
            "config formula: ρ̃ is off +x at z_cfg = {zc:?}, {}",
            R::NAME
        ));
    }
    let want = [a.cos(), 0.0, a.sin() * be.cos(), a.sin() * be.sin()];
    let got = [
        c.rho_t[0].f(),
        c.rho_t[1].f(),
        c.lambda_t[0].f(),
        c.lambda_t[1].f(),
    ];
    for k in 0..4 {
        if !within((got[k] - want[k]).abs(), TOL_TRIG * R::EPS) {
            return Err(format!(
                "config formula: (ρ̃, λ̃) = {got:?} ≠ (cos α·(1,0), sin α·(cos β, sin β)) = {want:?} at z_cfg = {zc:?}, {}",
                R::NAME
            ));
        }
    }
    let i = got.iter().map(|x| x * x).sum::<f64>();
    if !within((i - 1.0).abs(), TOL_I * R::EPS) {
        return Err(format!(
            "config sphere: ‖ρ̃‖² + ‖λ̃‖² − 1 = {:e} at z_cfg = {zc:?}, {}",
            i - 1.0,
            R::NAME
        ));
    }
    // Ranges: closed against the precision's own π/2 and π everywhere; open on the interior.
    if !(within(-a, 0.0) && within(a, R::LIT_6.f()) && within(-be, 0.0) && within(be, R::LIT_7.f()))
    {
        return Err(format!(
            "config range: (α, β) = ({a}, {be}) at z_cfg = {zc:?}, {}",
            R::NAME
        ));
    }
    if zc[0].abs() <= 10.0
        && zc[1].abs() <= 10.0
        && !(a > 0.0 && a < half_pi && be > 0.0 && be < pi)
    {
        return Err(format!(
            "config range: (α, β) = ({a}, {be}) not interior at z_cfg = {zc:?}, {}",
            R::NAME
        ));
    }
    Ok(())
}

fn check_cfg_all<R: Fp>(b: Blocks<R>) {
    for zc in fixed_pairs::<R>() {
        check_cfg_formula(b, zc).unwrap();
    }
    prop::run(&[control::<R>(), control::<R>()], |zc| {
        check_cfg_formula(b, zc).map_err(TestCaseError::fail)
    });
}

/// `z = 0`: `α = π/4`, `β = π/2`, `‖ρ̃‖ = ‖λ̃‖` (`cos π/4` against `sin π/4·1`, [`TOL_TRIG`] each).
fn check_cfg_golden<R: Fp>(b: Blocks<R>) {
    let m = (b.mass)([R::zero(), R::zero()]);
    let c = (b.config)([R::zero(), R::zero()], &m);
    let q = std::f64::consts::FRAC_PI_4;
    assert!(
        (c.alpha.f() - q).abs() <= TOL_ANGLE * q * R::EPS,
        "golden: α = {} not π/4, {}",
        c.alpha.f(),
        R::NAME
    );
    assert!(
        (c.beta.f() - 2.0 * q).abs() <= TOL_ANGLE * 2.0 * q * R::EPS,
        "golden: β = {} not π/2, {}",
        c.beta.f(),
        R::NAME
    );
    let nr = c.rho_t[0].f().hypot(c.rho_t[1].f());
    let nl = c.lambda_t[0].f().hypot(c.lambda_t[1].f());
    assert!(
        (nr - nl).abs() <= 2.0 * TOL_TRIG * R::EPS,
        "golden: ‖ρ̃‖ = {nr} ≠ ‖λ̃‖ = {nl}, {}",
        R::NAME
    );
}

#[test]
fn qa_config_formula() {
    check_cfg_golden(defaults::<f64>());
    check_cfg_golden(defaults::<f32>());
    check_cfg_all(defaults::<f64>());
    check_cfg_all(defaults::<f32>());
}

/// β through the α link's half-width: β = (π/2)σ(z_β).
#[cfg(feature = "controls")]
struct NarrowBeta;

#[cfg(feature = "controls")]
impl Link<1, 1> for NarrowBeta {
    const NAME: &'static str = "narrow_beta";
    type Codomain = kernel::generated::links::Beta;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidAlpha::forward(v)
    }
    fn inverse<R: LinkReal>(_: [R; 1]) -> [R; 1] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 1]) -> R {
        unreachable!()
    }
}

negative_control!(
    qa_config_formula,
    "β = (π/2)σ(z_β) must fail the formula",
    expected = "config formula: (α, β)",
    check_cfg_all(blocks::<
        Select<
            kernel::generated::links::SoftmaxTanh,
            SigmoidAlpha,
            NarrowBeta,
            SigmoidQ,
            SigmoidQ,
            SigmoidQ,
            SigmoidQ,
        >,
        f64,
    >("narrow_beta"))
);

// ── REQ-DEC-013: reconstruction ─────────────────────────────────────────────────────────────────────────────────

fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

/// `r₁ − r₀ = ρ` and `r₂ − r₀₁ = λ` (`r₀₁` the inner pair's CoM, recomputed here from `r₀, r₁`), each relative to the
/// size of the operands the round-off is relative to: `‖r₀‖ + ‖r₁‖ + ‖ρ‖`, and `(m₀‖r₀‖ + m₁‖r₁‖)/M₀₁ + ‖r₂‖ + ‖λ‖`
/// (near `α = 0`, `r₀` and `r₁` are large against their weighted mean). `r₀` and `r₁` are each `r₀₁ ∓ w·ρ`, a product,
/// a weight `mₖ/M₀₁` and a sum, about 1.5 eps of those operands each, and `w₀ + w₁ = 1` to 1 eps; `2·1.5 + 1 = 4`.
const TOL_JACOBI: f64 = 4.0;

/// Test 1's identities and test 6 for the masses `m` and config controls `zc`.
fn check_recon<R: Fp>(b: Blocks<R>, m: &Masses<R>, zc: [f64; 2], at_z: &str) -> Result<(), String> {
    let c = (b.config)(at::<R, 2>(zc), m);
    let at_z = format!("{at_z}, z_cfg = {zc:?}, {}", R::NAME);
    let mm = fx(m.m);
    let r = c.r.map(fx);
    if r.iter().flatten().any(|x| !x.is_finite()) {
        return Err(format!("reconstruct: non-finite positions {r:?} at {at_z}"));
    }
    let (rho, lambda) = (fx(c.rho), fx(c.lambda));
    let m01 = mm[0] + mm[1];
    let r01 = [0, 1].map(|k| (mm[0] * r[0][k] + mm[1] * r[1][k]) / m01);
    let d = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let e = norm([d[0] - rho[0], d[1] - rho[1]]) / ((norm(r[0]) + norm(r[1]) + norm(rho)) * R::EPS);
    if !within(e, TOL_JACOBI) {
        return Err(format!("reconstruct: r₁ − r₀ ≠ ρ by {e} eps at {at_z}"));
    }
    let d = [r[2][0] - r01[0], r[2][1] - r01[1]];
    let inner = (mm[0] * norm(r[0]) + mm[1] * norm(r[1])) / m01;
    let e =
        norm([d[0] - lambda[0], d[1] - lambda[1]]) / ((inner + norm(r[2]) + norm(lambda)) * R::EPS);
    if !within(e, TOL_JACOBI) {
        return Err(format!("reconstruct: r₂ − r₀₁ ≠ λ by {e} eps at {at_z}"));
    }
    let com = [0, 1].map(|k| (0..3).map(|i| mm[i] * r[i][k]).sum::<f64>());
    let size: f64 = (0..3).map(|i| mm[i] * norm(r[i])).sum();
    let e = norm(com) / (size * R::EPS);
    if !within(e, TOL_COM) {
        return Err(format!(
            "reconstruct: CoM = {com:?}, {e} eps of Σmᵢ‖rᵢ‖ at {at_z}"
        ));
    }
    let i: f64 = (0..3)
        .map(|k| mm[k] * (r[k][0].powi(2) + r[k][1].powi(2)))
        .sum();
    if !within((i - 1.0).abs(), TOL_I * R::EPS) {
        return Err(format!("reconstruct: I − 1 = {:e} at {at_z}", i - 1.0));
    }
    // Test 6: §3.2's definitions on the emitted (m, rᵢ).
    let mu_rho = mm[0] * mm[1] / m01;
    let mu_lambda = mm[2] * m01;
    let rho_t = [r[1][0] - r[0][0], r[1][1] - r[0][1]].map(|x| mu_rho.sqrt() * x);
    let lambda_t = [r[2][0] - r01[0], r[2][1] - r01[1]].map(|x| mu_lambda.sqrt() * x);
    let r_t = norm(rho_t).hypot(norm(lambda_t));
    let alpha = norm(lambda_t).atan2(rho_t[0]);
    let beta = lambda_t[1].atan2(lambda_t[0]);
    let db = beta - c.beta.f();
    let db = db.sin().atan2(db.cos()).abs();
    for (what, v) in [
        ("|R̃ − 1|", (r_t - 1.0).abs()),
        ("|ρ̃_y|", rho_t[1].abs()),
        ("|Δα|", (alpha - c.alpha.f()).abs()),
        ("sin α·|Δβ|", c.alpha.f().sin() * db),
    ] {
        if !within(v, TOL_INV * R::EPS) {
            return Err(format!("inverse: {what} = {} eps at {at_z}", v / R::EPS));
        }
    }
    Ok(())
}

fn check_recon_all<R: Fp>(b: Blocks<R>) {
    let fixed_cfg = [
        [0.0, 0.0],
        [-12.0, 3.0],
        [12.0, -3.0],
        [-40.0, 40.0],
        [40.0, -40.0],
        [R::MAX, -R::MAX],
    ];
    for zm in fixed_pairs::<R>() {
        let m = (b.mass)(at::<R, 2>(zm));
        for zc in fixed_cfg {
            check_recon(
                b,
                &m,
                zc.map(|x| R::of(x).f()),
                &format!("{} z_μ = {zm:?}", b.name),
            )
            .unwrap();
        }
    }
    prop::run(
        &[
            control::<R>(),
            control::<R>(),
            control::<R>(),
            control::<R>(),
        ],
        |z| {
            let m = (b.mass)(at::<R, 2>([z[0], z[1]]));
            check_recon(
                b,
                &m,
                [z[2], z[3]],
                &format!("{} z_μ = {:?}", b.name, [z[0], z[1]]),
            )
            .map_err(TestCaseError::fail)
        },
    );
    // Directly set masses (the ternary plot, the δm strip), tagged ones included.
    prop::run(
        &(simplex::<R>(), [control::<R>(), control::<R>()]),
        |(m, zc)| {
            let out = (b.from_masses)(at::<R, 3>(m));
            check_recon(b, &out, zc, &format!("m = {m:?}")).map_err(TestCaseError::fail)
        },
    );
}

#[test]
fn qa_reconstruct() {
    check_recon_all(defaults::<f64>());
    check_recon_all(defaults::<f32>());
    check_recon_all(stick::<f64>());
    check_recon_all(stick::<f32>());
}

/// Body 2 and the inner pair's CoM with their weights swapped: `r₀₁ = −M₀₁λ`, `r₂ = m₂λ`.
#[cfg(feature = "controls")]
fn swapped_weights(z: [f64; 2], m: &Masses<f64>) -> Config<f64> {
    let mut c = config::decode::<Defaults, f64>(z, m);
    let l = c.lambda;
    let r01 = [-m.m01 * l[0], -m.m01 * l[1]];
    let (w0, w1) = (m.m[1] / m.m01, m.m[0] / m.m01);
    c.r = [
        [r01[0] - w0 * c.rho[0], r01[1] - w0 * c.rho[1]],
        [r01[0] + w1 * c.rho[0], r01[1] + w1 * c.rho[1]],
        [m.m[2] * l[0], m.m[2] * l[1]],
    ];
    c
}

negative_control!(
    qa_reconstruct,
    "a reconstruction with r₀₁ = −M₀₁λ and r₂ = m₂λ must fail",
    expected = "reconstruct: CoM",
    check_recon_all(Blocks {
        config: swapped_weights,
        ..defaults::<f64>()
    })
);

// ── REQ-DEC-015: the α orientation ──────────────────────────────────────────────────────────────────────────────

/// Decoder test 9: over a `z_α` sweep (α from near 0 to near π/2, within the range where the precision still
/// resolves a step in `cos α` and `sin α`), `‖r₁ − r₀‖` falls and `‖λ‖` rises strictly, so the largest separation and
/// the smallest `‖λ‖` are at the smallest α; `‖ρ̃‖ > ‖λ̃‖` below `α = π/4` and `<` above it.
fn check_orientation<R: Fp>(b: Blocks<R>) {
    let (lo, step) = if R::EPS < 1e-10 {
        (-10.0, 0.125)
    } else {
        (-5.0, 0.25)
    };
    let n = (2.0 * -lo / step) as usize;
    let grid: Vec<f64> = (0..=n).map(|k| lo + step * k as f64).collect();
    let mass_points: Vec<Masses<R>> = [
        [0.0, 0.0],
        [2.0, -1.0],
        [-1.5, 0.3],
        [40.0, -40.0],
        [40.0, 40.0],
    ]
    .iter()
    .map(|&zm| (b.mass)(at::<R, 2>(zm)))
    .chain(
        [
            [0.98, 0.01, 0.01],
            [1e-4, 1e-4, 1.0 - 2e-4],
            [0.2, 0.5, 0.3],
        ]
        .iter()
        .map(|&m| (b.from_masses)(at::<R, 3>(m))),
    )
    .collect();
    for m in &mass_points {
        for zb in [-2.0, 0.0, 1.0, 4.0] {
            let cs: Vec<Config<R>> = grid
                .iter()
                .map(|&za| (b.config)(at::<R, 2>([za, zb]), m))
                .collect();
            let sep: Vec<f64> = cs
                .iter()
                .map(|c| norm([c.r[1][0].f() - c.r[0][0].f(), c.r[1][1].f() - c.r[0][1].f()]))
                .collect();
            let lam: Vec<f64> = cs.iter().map(|c| norm(fx(c.lambda))).collect();
            for k in 1..grid.len() {
                assert!(
                    sep[k] < sep[k - 1] && lam[k] > lam[k - 1],
                    "orientation: ‖r₁ − r₀‖ does not fall or ‖λ‖ does not rise from z_α = {} to {}, m = {:?}, {}",
                    grid[k - 1],
                    grid[k],
                    m.m,
                    R::NAME
                );
            }
            for (za, c) in grid.iter().zip(&cs) {
                let diff = norm(fx(c.rho_t)) - norm(fx(c.lambda_t));
                let ok = if *za < 0.0 {
                    diff > 0.0
                } else if *za > 0.0 {
                    diff < 0.0
                } else {
                    diff.abs() <= 2.0 * TOL_TRIG * R::EPS
                };
                assert!(
                    ok,
                    "orientation: ‖ρ̃‖ − ‖λ̃‖ = {diff} at z_α = {za}: no crossover at π/4, {}",
                    R::NAME
                );
            }
        }
    }
}

#[test]
fn qa_alpha_orientation() {
    check_orientation(defaults::<f64>());
    check_orientation(defaults::<f32>());
}

/// The gotcha: `ρ̃ = sin α·(1, 0)`, `λ̃ = cos α·(cos β, sin β)`.
#[cfg(feature = "controls")]
fn gotcha(z: [f64; 2], m: &Masses<f64>) -> Config<f64> {
    let (a, be) = config::angles::<Defaults, f64>(z);
    let rho_t = [a.sin(), 0.0];
    let lambda_t = [a.cos() * be.cos(), a.cos() * be.sin()];
    let (rho, lambda, r) = config::reconstruct(m, rho_t, lambda_t);
    Config {
        alpha: a,
        beta: be,
        rho_t,
        lambda_t,
        rho,
        lambda,
        r,
    }
}

negative_control!(
    qa_alpha_orientation,
    "‖ρ̃‖ = sin α must fail the orientation",
    expected = "orientation: ‖r₁ − r₀‖ does not fall",
    check_orientation(Blocks {
        config: gotcha,
        ..defaults::<f64>()
    })
);

// ── REQ-INT-002: units ──────────────────────────────────────────────────────────────────────────────────────────

fn check_units(g: f64, m: f64, i: f64) {
    assert!(
        g == 1.0 && m == 1.0 && i == 1.0,
        "units: (G, M, I) = ({g}, {m}, {i}), not (1, 1, 1)"
    );
}

#[test]
fn qa_units() {
    check_units(
        units::g::<f64>(),
        units::total_mass::<f64>(),
        units::inertia::<f64>(),
    );
    check_units(
        units::g::<f32>() as f64,
        units::total_mass::<f32>() as f64,
        units::inertia::<f32>() as f64,
    );
    // Decoded ICs carry M = 1 and I = 1: qa_mass_formula and qa_reconstruct assert both over the domain; here at
    // the golden IC, z = 0, at both precisions, for the record.
    let b = defaults::<f32>();
    let m = (b.mass)([0.0, 0.0]);
    check_recon(b, &m, [0.0, 0.0], "golden").unwrap();
    let b = defaults::<f64>();
    let m = (b.mass)([0.0, 0.0]);
    check_recon(b, &m, [0.0, 0.0], "golden").unwrap();
}

negative_control!(
    qa_units,
    "G in SI units must fail",
    expected = "units: (G, M, I)",
    check_units(6.674e-11, 1.0, 1.0)
);
