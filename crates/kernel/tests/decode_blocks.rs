//! The decoder's first two blocks (`kernel::decode`; dd_decoder §3.1, §3.2, §5), each run at the f64 and the f32
//! instantiation natively (R-265), over the default selection and the `stick_breaking` mass link:
//! - `decode_latent_layout`: the latent type is 8 controls, config `z[0:2]`, momentum `z[2:6]`, mass `z[6:8]`
//!   (REQ-SYS-010).
//! - `decode_mass_equal_at_zero`, `decode_mass_burrau`, `decode_mass_sum`: `z_μ = 0` gives `(⅓, ⅓, ⅓)`; the Burrau
//!   point `μ₁ = log(b/c)`, `μ₂ = log(a/c)` gives `(c, b, a)/(a+b+c)`; `Σmᵢ = 1` on fuzzed `z` (REQ-DEC-010).
//! - `decode_m01_tiny_iff`, `decode_m01_tiny_boundary`, `decode_m01_tiny_reachable`, `decode_m01_tiny_measurement`:
//!   decoder test 4. The tag fires exactly on `M₀₁ < ε` over fuzzed `z_μ`, saturation included, and nowhere else in
//!   the mass block; tagged outputs are finite and kept; the fire branch is driven by mass inputs below ε and by
//!   `stick_breaking`'s corner; the reachable `M₀₁` range is the checked-in measurement, `fixtures/gates/m01_tiny/`
//!   (REQ-DEC-011, REQ-DEC-039). `decode_m01_tiny_contraction`: fusing `stick_breaking`'s last multiply into the tag's
//!   add, as a contracting shader compiler may, flips no tag (R-34).
//! - `decode_config_golden`, `decode_config_sphere`: `z = 0` gives `α = π/4`, `β = π/2`, `‖ρ̃‖ = ‖λ̃‖`;
//!   `‖ρ̃‖² + ‖λ̃‖² = 1` and `ρ̃` on `+x` on fuzzed `z` (REQ-DEC-012).
//! - `decode_reconstruct_identities`, `decode_reconstruct_inverse`: `r₁ − r₀ = ρ`, total CoM `= 0` and
//!   `I = Σmᵢ‖rᵢ‖² = 1` on fuzzed `z`; decoder test 6, `(α, β, R̃ = 1)` recovered from the emitted `(m, rᵢ)` through
//!   §3.2's definitions (REQ-DEC-013).
//! - `decode_alpha_orientation`: decoder test 9 (REQ-DEC-015).
//! - `units_dimensionless`: `G = 1`, and decoded ICs have total mass 1 and `I = 1` (REQ-INT-002).
//!
//! **Tolerances** (REQ-DEC-044's calibration, R-71: the values here are the proposal, with the measured residuals as
//! evidence, unconfirmed until the M2 gate). Each is a multiple of the precision's machine epsilon `eps`
//! (2.2×10⁻¹⁶ at f64, 1.2×10⁻⁷ at f32), the same multiple at both, over every `z` of the domain below, saturation
//! included. Test 1's `Σmᵢ`, `CoM` and `I` and test 6 are measured here; `Σpᵢ = 0` waits for the momentum block
//! (TASK-M2-03) and test 8 for the invariant construction (TASK-M2-09). Each test prints its worst residual, in units
//! of `eps`, so the evidence re-measures with `--nocapture`.

use std::cell::RefCell;
use std::path::Path;

use kernel::decode::config::{self, Config};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::latent::Latent;
use kernel::decode::mass::{self, Masses};
use kernel::decode::units;
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::*;
use proptest::prelude::*;
use validation::{negative_control, prop};

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the decoder is instantiated at, with conversions to and from f64 for the checks.
trait P: LinkReal + Neighbours {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    /// The machine epsilon, as f64.
    fn eps() -> f64;
}

impl P for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn eps() -> f64 {
        f64::EPSILON
    }
}

impl P for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn eps() -> f64 {
        f32::EPSILON as f64
    }
}

// ── The decoder under test ──────────────────────────────────────────────────────────────────────────────────────

/// The two blocks at one precision, behind function pointers, so a control runs the same check on a broken block.
struct Dec<R> {
    mass: fn([R; 2]) -> Masses<R>,
    from_masses: fn([R; 3]) -> Masses<R>,
    config: fn([R; 2], &Masses<R>) -> Config<R>,
}

impl<R> Clone for Dec<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for Dec<R> {}

/// The selection `S`'s blocks at `R`.
fn dec<S: Selection, R: P>() -> Dec<R> {
    Dec {
        mass: mass::decode::<S, R>,
        from_masses: mass::from_masses::<R>,
        config: config::decode::<S, R>,
    }
}

/// The default selection with the `stick_breaking` mass link.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// Both mass links' decoders at `R`, named.
fn both<R: P>() -> [(&'static str, Dec<R>); 2] {
    [
        ("softmax_tanh", dec::<Defaults, R>()),
        ("stick_breaking", dec::<Stick, R>()),
    ]
}

// ── The domain ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A control drawn from the centre, the transition into saturation, deep saturation, or anywhere up to 10³⁸ in
/// magnitude: every finite `z` is accepted (R-407), and the widest draws stay finite at f32.
fn control() -> impl Strategy<Value = f64> {
    prop_oneof![
        -2.0f64..=2.0,
        -20.0f64..=20.0,
        -60.0f64..=60.0,
        (any::<bool>(), 0.0f64..=38.0)
            .prop_map(|(neg, e)| if neg { -1.0 } else { 1.0 } * 10f64.powf(e)),
    ]
}

/// Fixed controls besides the fuzzed ones: the centre, the transition and saturation, at each sign.
const FIXED: [f64; 13] = [
    -1e38, -1e3, -40.0, -16.0, -8.0, -1.0, 0.0, 1.0, 8.0, 16.0, 40.0, 1e3, 1e38,
];

/// Every pair of fixed controls.
fn fixed_pairs() -> Vec<[f64; 2]> {
    FIXED
        .iter()
        .flat_map(|&a| FIXED.iter().map(move |&b| [a, b]))
        .collect()
}

/// The worst residual of each named quantity a check saw, in units of `eps`.
#[derive(Default)]
struct Worst(RefCell<Vec<(String, f64)>>);

impl Worst {
    fn see(&self, name: &str, v: f64) {
        let mut w = self.0.borrow_mut();
        match w.iter_mut().find(|(n, _)| n == name) {
            Some((_, x)) => *x = x.max(v),
            None => w.push((name.to_owned(), v)),
        }
    }
    fn print(&self, test: &str) {
        for (n, v) in self.0.borrow().iter() {
            eprintln!("{test}: worst {n} = {v:.3} eps");
        }
    }
}

/// Runs `check` at each fixed pair and on fuzzed pairs of mass controls and config controls together, at f64 and
/// f32.
fn over_z(check: impl Fn([f64; 2], [f64; 2])) {
    for zm in fixed_pairs() {
        for zc in [
            [0.0, 0.0],
            [-40.0, 40.0],
            [40.0, -40.0],
            [1e38, 1e38],
            [-1e38, -1e38],
        ] {
            check(zm, zc);
        }
    }
    for zc in fixed_pairs() {
        check([0.0, 0.0], zc);
        check([40.0, 40.0], zc);
    }
    let draws = ([control(), control()], [control(), control()]);
    prop::run(&draws, |(zm, zc)| {
        check(zm, zc);
        Ok(())
    });
}

fn at<R: P, const N: usize>(z: [f64; N]) -> [R; N] {
    z.map(R::of)
}

// ── The latent type ─────────────────────────────────────────────────────────────────────────────────────────────

/// The block accessors of a latent type.
struct Blocks {
    config: fn(&Latent<f64>) -> [f64; 2],
    momentum: fn(&Latent<f64>) -> [f64; 4],
    mass: fn(&Latent<f64>) -> [f64; 2],
}

/// The layout is 8 controls, `z[0:2]` config, `z[2:6]` momentum, `z[6:8]` mass (the registry's slots: α at 0, β at
/// 1, q0–q3 at 2–5, the mass control from 6).
fn check_layout(b: &Blocks) {
    assert_eq!(
        std::mem::size_of::<Latent<f32>>(),
        8 * 4,
        "the latent type is not 8 controls"
    );
    assert_eq!(
        std::mem::size_of::<Latent<f64>>(),
        8 * 8,
        "the latent type is not 8 controls"
    );
    let z = Latent::new([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
    assert_eq!((b.config)(&z), [0.0, 1.0], "the config block is not z[0:2]");
    assert_eq!(
        (b.momentum)(&z),
        [2.0, 3.0, 4.0, 5.0],
        "the momentum block is not z[2:6]"
    );
    assert_eq!((b.mass)(&z), [6.0, 7.0], "the mass block is not z[6:8]");
}

#[test]
fn decode_latent_layout() {
    check_layout(&Blocks {
        config: Latent::config,
        momentum: Latent::momentum,
        mass: Latent::mass,
    });
}

negative_control!(
    decode_latent_layout,
    "a layout with the mass block first must fail",
    expected = "the mass block is not z[6:8]",
    check_layout(&Blocks {
        config: Latent::config,
        momentum: Latent::momentum,
        mass: |z| [z.z[0], z.z[1]],
    })
);

// ── D_mass ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// `z_μ = 0` gives `(⅓, ⅓, ⅓)` exactly at `R` (each `1/3` rounded once), untagged.
fn check_equal_at_zero<R: P>(d: Dec<R>) {
    let out = (d.mass)([R::zero(), R::zero()]);
    let third = R::one() / R::of(3.0);
    assert_eq!(
        out.m,
        [third; 3],
        "z_μ = 0 does not give equal masses at {}",
        R::NAME
    );
    assert_eq!(
        out.degenerate,
        None,
        "equal masses are tagged at {}",
        R::NAME
    );
}

#[test]
fn decode_mass_equal_at_zero() {
    check_equal_at_zero(dec::<Defaults, f64>());
    check_equal_at_zero(dec::<Defaults, f32>());
}

/// `softmax(1, μ₁, μ₂)`: the reference body's logit is 1, not 0.
#[cfg(feature = "controls")]
struct ReferenceOne;

#[cfg(feature = "controls")]
impl Link<2, 3> for ReferenceOne {
    const NAME: &'static str = "reference_one";
    type Codomain = Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let e = [
            R::one().exp(),
            (R::MU_MAX * v[0].tanh()).exp(),
            (R::MU_MAX * v[1].tanh()).exp(),
        ];
        let s = e[0] + e[1] + e[2];
        [e[0] / s, e[1] / s, e[2] / s]
    }
    fn inverse<R: LinkReal>(_: [R; 3]) -> [R; 2] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 2]) -> R {
        unreachable!()
    }
}

#[cfg(feature = "controls")]
type WithMass<L> = Select<L, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

negative_control!(
    decode_mass_equal_at_zero,
    "a mass link whose reference logit is 1 must fail at z_μ = 0",
    expected = "z_μ = 0 does not give equal masses",
    check_equal_at_zero(dec::<WithMass<ReferenceOne>, f64>())
);

/// The Burrau tolerance on each mass, in `eps` (REQ-DEC-044's neighbour; the masses' round-off through `tanh`,
/// `exp` and the normalisation, and `z`'s own rounding to `R`).
const BURRAU_EPS: f64 = 4.0;

/// The Burrau point of `(a, b, c)`: `μ₁ = log(b/c)`, `μ₂ = log(a/c)`, so `z_μk = artanh(μₖ/μ_max)`, gives masses
/// `(c, b, a)/(a+b+c)`, body 0 the reference.
fn check_burrau<R: P>(d: Dec<R>, worst: &Worst) {
    for (a, b, c) in [
        (3.0f64, 4.0f64, 5.0f64),
        (5.0, 4.0, 3.0),
        (1.0, 2.0, 3.0),
        (1.0, 1.0, 1.0),
        (10.0, 1.0, 2.0),
    ] {
        let mu_max = <f64 as ChartConstants>::MU_MAX;
        let z: [f64; 2] = [f64::ln(b / c) / mu_max, f64::ln(a / c) / mu_max].map(f64::atanh);
        let out = (d.mass)(at::<R, 2>(z));
        let want = [c, b, a].map(|x| x / (a + b + c));
        for (k, (got, want)) in out.m.iter().zip(want).enumerate() {
            let err = (got.f() - want).abs() / R::eps();
            worst.see(&format!("burrau m at {}", R::NAME), err);
            assert!(
                err <= BURRAU_EPS,
                "the Burrau point ({a}, {b}, {c}) gives m{k} = {} at {}, not {want}",
                got.f(),
                R::NAME,
            );
        }
    }
}

#[test]
fn decode_mass_burrau() {
    let worst = Worst::default();
    check_burrau(dec::<Defaults, f64>(), &worst);
    check_burrau(dec::<Defaults, f32>(), &worst);
    worst.print("decode_mass_burrau");
}

/// `softmax(μ₁, μ₂, 0)`: body 2 the reference, not body 0.
#[cfg(feature = "controls")]
struct ReferenceTwo;

#[cfg(feature = "controls")]
impl Link<2, 3> for ReferenceTwo {
    const NAME: &'static str = "reference_two";
    type Codomain = Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let e = [
            (R::MU_MAX * v[0].tanh()).exp(),
            (R::MU_MAX * v[1].tanh()).exp(),
            R::one(),
        ];
        let s = e[0] + e[1] + e[2];
        [e[0] / s, e[1] / s, e[2] / s]
    }
    fn inverse<R: LinkReal>(_: [R; 3]) -> [R; 2] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 2]) -> R {
        unreachable!()
    }
}

negative_control!(
    decode_mass_burrau,
    "a mass link with body 2 the reference must fail at the Burrau point",
    expected = "the Burrau point",
    check_burrau(dec::<WithMass<ReferenceTwo>, f64>(), &Worst::default())
);

/// `Σmᵢ = 1` to [`SUM_EPS`], REQ-DEC-044's proposal for test 1's mass identity.
const SUM_EPS: f64 = 4.0;

/// `Σmᵢ = 1` within [`SUM_EPS`] on every `z` of the domain, for each decoder of `decs`.
fn check_sum<R: P>(decs: &[(&str, Dec<R>)], worst: &Worst) {
    over_z(|zm, _| {
        for (name, d) in decs {
            let m = (d.mass)(at::<R, 2>(zm)).m;
            let err =
                (m[0].f() + m[1].f() + m[2].f() - units::total_mass::<f64>()).abs() / R::eps();
            worst.see(&format!("Σm − 1 at {}", R::NAME), err);
            assert!(
                err <= SUM_EPS,
                "{name} at z_μ = {zm:?}: Σm − 1 = {err} eps at {}",
                R::NAME
            );
        }
    });
}

#[test]
fn decode_mass_sum() {
    let worst = Worst::default();
    check_sum(&both::<f64>(), &worst);
    check_sum(&both::<f32>(), &worst);
    worst.print("decode_mass_sum");
}

/// `(1, e^μ₁, e^μ₂)/(1 + e^μ₁)`: the normalisation drops a term.
#[cfg(feature = "controls")]
struct DropsTerm;

#[cfg(feature = "controls")]
impl Link<2, 3> for DropsTerm {
    const NAME: &'static str = "drops_term";
    type Codomain = Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let e = [
            R::one(),
            (R::MU_MAX * v[0].tanh()).exp(),
            (R::MU_MAX * v[1].tanh()).exp(),
        ];
        let s = e[0] + e[1];
        [e[0] / s, e[1] / s, e[2] / s]
    }
    fn inverse<R: LinkReal>(_: [R; 3]) -> [R; 2] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 2]) -> R {
        unreachable!()
    }
}

negative_control!(
    decode_mass_sum,
    "a mass link whose normalisation drops a term must fail Σm = 1",
    expected = "Σm − 1 =",
    check_sum(
        &[("drops_term", dec::<WithMass<DropsTerm>, f64>())],
        &Worst::default()
    )
);

// ── M01_TINY ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The tag is `M01Tiny` exactly when `m₀ + m₁ < ε_μ`, `M₀₁` is that sum's bits, and the block's outputs, with the
/// config decoded from them, are finite: tagged, never NaN, never dropped.
fn check_tag<R: P>(name: &str, d: Dec<R>, out: Masses<R>, zc: [R; 2], at_z: &str) {
    let m01 = out.m[0] + out.m[1];
    assert_eq!(
        out.m01,
        m01,
        "{name} at {at_z}: M₀₁ is not m₀ + m₁ at {}",
        R::NAME
    );
    let want = if m01 < mass::m01_epsilon::<R>() {
        Some(Degenerate::M01Tiny)
    } else {
        None
    };
    assert_eq!(
        out.degenerate,
        want,
        "{name} at {at_z}: the tag is not exactly M₀₁ < ε at {} (M₀₁ = {:e})",
        R::NAME,
        m01.f()
    );
    let cfg = (d.config)(zc, &out);
    let finite = out.m.iter().all(|x| x.is_finite())
        && cfg.r.iter().flatten().all(|x| x.is_finite())
        && cfg.alpha.is_finite()
        && cfg.beta.is_finite();
    assert!(
        finite,
        "{name} at {at_z}: an output is not finite at {} ({out:?}, {cfg:?})",
        R::NAME
    );
}

/// Decoder test 4 over the domain: for each mass link, the tag is exactly `M₀₁ < ε` and the outputs finite.
fn check_iff<R: P>(decs: &[(&str, Dec<R>)]) {
    over_z(|zm, zc| {
        for (name, d) in decs {
            let out = (d.mass)(at::<R, 2>(zm));
            check_tag(name, *d, out, at::<R, 2>(zc), &format!("z_μ = {zm:?}"));
        }
    });
}

#[test]
fn decode_m01_tiny_iff() {
    check_iff(&both::<f64>());
    check_iff(&both::<f32>());
}

/// The tag decided against ε_w, not ε_μ.
#[cfg(feature = "controls")]
fn tag_at_eps_w<R: P>(m: [R; 3]) -> Masses<R> {
    let m01 = m[0] + m[1];
    Masses {
        m,
        m01,
        degenerate: (m01 < R::EPS_W).then_some(Degenerate::M01Tiny),
    }
}

negative_control!(
    decode_m01_tiny_iff,
    "a tag decided against ε_w must fail in stick_breaking's corner",
    expected = "the tag is not exactly M₀₁ < ε",
    check_iff(&[(
        "stick_breaking",
        Dec {
            mass: |z| tag_at_eps_w(<StickBreaking as Link<2, 3>>::forward::<f64>(z)),
            ..dec::<Stick, f64>()
        }
    )])
);

/// The fire boundary, on mass inputs set directly (a chart that sets masses, chart_reference §5.1): `M₀₁ = ε`
/// exactly and one float above do not fire; one float below, far below and `M₀₁ = 0` fire; every output finite but
/// the zero-mass pair's configuration, whose unweighting divides by the pair's mass.
fn check_boundary<R: P>(d: Dec<R>) {
    let eps = mass::m01_epsilon::<R>();
    let half = eps / R::of(2.0);
    let below = R::from_bits_below(eps);
    let above = R::from_bits_above(eps);
    let cases = [
        ("M₀₁ = ε", [half, half], None),
        ("M₀₁ one float above ε", [half, above - half], None),
        (
            "M₀₁ one float below ε",
            [half, below - half],
            Some(Degenerate::M01Tiny),
        ),
        (
            "M₀₁ = ε/1000",
            [eps / R::of(2000.0), eps / R::of(2000.0)],
            Some(Degenerate::M01Tiny),
        ),
    ];
    for (what, [m0, m1], want) in cases {
        let m = [m0, m1, R::one() - (m0 + m1)];
        let out = (d.from_masses)(m);
        assert_eq!(
            out.m01,
            m0 + m1,
            "{what}: M₀₁ is not m₀ + m₁ at {}",
            R::NAME
        );
        assert_eq!(
            out.degenerate,
            want,
            "{what}: the tag is wrong at {}",
            R::NAME
        );
        check_tag("from_masses", d, out, [R::zero(), R::zero()], what);
    }
    let zero = (d.from_masses)([R::zero(), R::zero(), R::one()]);
    assert_eq!(
        zero.degenerate,
        Some(Degenerate::M01Tiny),
        "M₀₁ = 0: the tag is wrong at {}",
        R::NAME
    );
    assert!(
        zero.m
            .iter()
            .chain(std::iter::once(&zero.m01))
            .all(|x| !x.is_nan()),
        "M₀₁ = 0: a mass output is NaN"
    );
}

/// Bit neighbours of a float.
trait Neighbours: Sized {
    fn from_bits_below(x: Self) -> Self;
    fn from_bits_above(x: Self) -> Self;
}

impl Neighbours for f64 {
    fn from_bits_below(x: f64) -> f64 {
        f64::from_bits(x.to_bits() - 1)
    }
    fn from_bits_above(x: f64) -> f64 {
        f64::from_bits(x.to_bits() + 1)
    }
}

impl Neighbours for f32 {
    fn from_bits_below(x: f32) -> f32 {
        f32::from_bits(x.to_bits() - 1)
    }
    fn from_bits_above(x: f32) -> f32 {
        f32::from_bits(x.to_bits() + 1)
    }
}

#[test]
fn decode_m01_tiny_boundary() {
    check_boundary(dec::<Defaults, f64>());
    check_boundary(dec::<Defaults, f32>());
}

/// The tag decided by `M₀₁ ≤ ε`, not `<`.
#[cfg(feature = "controls")]
fn tag_at_or_below<R: P>(m: [R; 3]) -> Masses<R> {
    let m01 = m[0] + m[1];
    Masses {
        m,
        m01,
        degenerate: (m01 <= mass::m01_epsilon::<R>()).then_some(Degenerate::M01Tiny),
    }
}

negative_control!(
    decode_m01_tiny_boundary,
    "a tag that fires at M₀₁ = ε must fail the boundary",
    expected = "M₀₁ = ε: the tag is wrong",
    check_boundary(Dec {
        from_masses: tag_at_or_below::<f64>,
        ..dec::<Defaults, f64>()
    })
);

/// The fire branch from finite `z`: `stick_breaking`'s saturated `m₂ → 1` corner fires at both precisions, and
/// `softmax_tanh`'s floor, the same corner, does not.
fn check_reachable<R: P>(stick: Dec<R>, softmax: Dec<R>) {
    for z in [[30.0, 30.0], [40.0, 1e3], [1e38, 1e38]] {
        let out = (stick.mass)(at::<R, 2>(z));
        assert_eq!(
            out.degenerate,
            Some(Degenerate::M01Tiny),
            "stick_breaking's corner z_μ = {z:?} does not fire at {} (M₀₁ = {:e})",
            R::NAME,
            out.m01.f()
        );
    }
    let floor = (softmax.mass)(at::<R, 2>([-40.0, 40.0]));
    assert_eq!(
        floor.degenerate,
        None,
        "softmax_tanh's floor fires at {}",
        R::NAME
    );
}

#[test]
fn decode_m01_tiny_reachable() {
    check_reachable(dec::<Stick, f64>(), dec::<Defaults, f64>());
    check_reachable(dec::<Stick, f32>(), dec::<Defaults, f32>());
}

negative_control!(
    decode_m01_tiny_reachable,
    "a tag decided against ε_w must not fire in stick_breaking's corner",
    expected = "does not fire",
    check_reachable(
        Dec {
            mass: |z| tag_at_eps_w(<StickBreaking as Link<2, 3>>::forward::<f64>(z)),
            ..dec::<Stick, f64>()
        },
        dec::<Defaults, f64>()
    )
);

/// The measurement file, relative to the workspace root.
const MEASUREMENT: &str = "fixtures/gates/m01_tiny/measurement.toml";

/// The ternary plot's interior buffer `ε_m` (chart_reference §4.5, its default).
const EPS_M: f64 = 1e-4;

/// The smallest `z` on the diagonal `z_μ1 = z_μ2 = z` in `[0, 40]` at which `stick_breaking` fires at `R`, to
/// 10⁻⁶, by bisection on the tag.
fn fire_onset<R: P>(d: Dec<R>) -> f64 {
    let fires = |z: f64| (d.mass)([R::of(z), R::of(z)]).degenerate.is_some();
    let (mut lo, mut hi) = (0.0, 40.0);
    assert!(
        !fires(lo) && fires(hi),
        "the onset is not bracketed at {}",
        R::NAME
    );
    while hi - lo > 1e-6 {
        let mid = 0.5 * (lo + hi);
        if fires(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// The M₀₁ range reachable at μ_max = 5, each value measured, as `(key, value, what)`.
fn measure() -> Vec<(&'static str, f64, &'static str)> {
    let (mu, eps_mu) = (f64::MU_MAX, f64::EPS_MU);
    let eps = mass::m01_epsilon::<f64>();
    let soft_exact = (1.0 + (-mu).exp()) / (1.0 + (-mu).exp() + mu.exp());
    let soft64 = mass::decode::<Defaults, f64>([-40.0, 40.0]).m01;
    let soft32 = mass::decode::<Defaults, f32>([-40.0, 40.0]).m01;
    let stick64 = mass::decode::<Stick, f64>([40.0, 40.0]).m01;
    let stick32 = mass::decode::<Stick, f32>([40.0, 40.0]).m01;
    let ternary = mass::from_masses::<f64>([EPS_M, EPS_M, 1.0 - 2.0 * EPS_M]).m01;
    vec![
        ("eps", eps, "the proposed ε: ε_M01"),
        (
            "softmax_tanh_floor_exact",
            soft_exact,
            "(1 + e^−μ_max)/(1 + e^−μ_max + e^μ_max)",
        ),
        (
            "softmax_tanh_floor_f64",
            soft64,
            "M₀₁ at z_μ = (−40, 40), f64",
        ),
        (
            "softmax_tanh_floor_f32",
            soft32 as f64,
            "M₀₁ at z_μ = (−40, 40), f32",
        ),
        (
            "stick_breaking_floor_exact",
            eps_mu - eps_mu * eps_mu / 4.0,
            "ε_μ − ε_μ²/4",
        ),
        (
            "stick_breaking_floor_f64",
            stick64,
            "M₀₁ at z_μ = (40, 40), f64",
        ),
        (
            "stick_breaking_floor_f32",
            stick32 as f64,
            "M₀₁ at z_μ = (40, 40), f32",
        ),
        (
            "eps_over_stick_breaking_floor",
            eps / stick32.max(stick64 as f32) as f64,
            "ε's margin above the corner's pile-up",
        ),
        (
            "ternary_floor_over_eps",
            ternary / eps,
            "the ternary floor's margin above ε",
        ),
        (
            "stick_breaking_onset_f64",
            fire_onset(dec::<Stick, f64>()),
            "first firing z on the diagonal, f64",
        ),
        (
            "stick_breaking_onset_f32",
            fire_onset(dec::<Stick, f32>()),
            "first firing z on the diagonal, f32",
        ),
        (
            "ternary_floor",
            ternary,
            "2ε_m, ε_m = 1e-4: the buffered vertex m₂ = 1",
        ),
    ]
}

/// The measurement file's text for `values`.
fn measurement_text(values: &[(&str, f64, &str)]) -> String {
    let mut s = String::from(
        "# The M01_TINY ε calibration's evidence (REQ-DEC-039; R-71): the M₀₁ range reachable at μ_max = 5.\n\
         # Measured by crates/kernel/tests/decode_blocks.rs, decode_m01_tiny_measurement; regenerate with\n\
         # PRIN_WRITE_M01_MEASUREMENT=1. Compared at a relative 1e-6. Each value is analytic, a saturated floor, or an\n\
         # onset where M₀₁ still changes with z at f32 resolution (ε sits in the gap above stick_breaking's floor), so\n\
         # a few ulps of a platform's exp or tanh move it by far less than that; at ε = ε_μ the f32 onset would not be\n\
         # (a 1-ulp tanhf moves it by 1.7%).\n",
    );
    for (k, v, what) in values {
        s.push_str(&format!("{k} = {v:e}  # {what}\n"));
    }
    s
}

/// Each key of `text` holds its measured value, within a relative 10⁻⁶.
fn check_measurement(text: &str, values: &[(&str, f64, &str)]) {
    for (k, v, _) in values {
        let line = text
            .lines()
            .find(|l| l.split('=').next().is_some_and(|key| key.trim() == *k))
            .unwrap_or_else(|| panic!("{MEASUREMENT}: no `{k}`"));
        let held: f64 = line
            .split('=')
            .nth(1)
            .and_then(|r| r.split('#').next())
            .map(str::trim)
            .and_then(|r| r.parse().ok())
            .unwrap_or_else(|| panic!("{MEASUREMENT}: `{k}` holds no number"));
        assert!(
            (held - v).abs() <= 1e-6 * v.abs(),
            "{MEASUREMENT}: `{k}` holds {held:e}, measured {v:e}"
        );
    }
}

#[test]
fn decode_m01_tiny_measurement() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(MEASUREMENT);
    let values = measure();
    if std::env::var_os("PRIN_WRITE_M01_MEASUREMENT").is_some() {
        std::fs::write(&path, measurement_text(&values)).expect("the measurement writes");
    }
    let text = std::fs::read_to_string(&path).expect("the measurement reads");
    check_measurement(&text, &values);
    for (k, v, _) in &values {
        eprintln!("decode_m01_tiny_measurement: {k} = {v:e}");
    }
}

negative_control!(
    decode_m01_tiny_measurement,
    "a measurement file holding a wrong floor must fail",
    expected = "holds 7e-3, measured",
    check_measurement(
        "softmax_tanh_floor_exact = 7e-3\n",
        &[("softmax_tanh_floor_exact", 6.7e-3, "")]
    )
);

/// `stick_breaking`'s forward in the generated op order (`kernel::generated::links`), stopped before its last
/// multiply: `(m₀, t₇, t₁₃)`, with `m₁ = t₇·t₁₃`.
fn stick_parts<R: P>(z: [R; 2]) -> (R, R, R) {
    let sigmoid = |x: R| R::one() / (R::one() + (-x).exp());
    let t0 = R::LIT_3 * z[0];
    let t4 = R::LIT_1 * R::EPS_MU * z[0].tanh();
    let t8 = R::LIT_3 * z[1];
    let t12 = R::LIT_1 * R::EPS_MU * z[1].tanh();
    (sigmoid(-t0) + t4, sigmoid(t0) - t4, sigmoid(-t8) + t12)
}

/// The contraction exposure (R-34; physics F2): inlined, `M₀₁ = m₀ + m₁` under `stick_breaking` reaches a shader
/// compiler as `m₀ + t₇·t₁₃`, which a contracting backend may fuse. Over `[3, 20]²` at a step of 1/32, through the
/// tag's onset and the saturated corner, the tag `< eps` is the same from the separate add and from the fused
/// `fma(t₇, t₁₃, m₀)`, at `R`. The replica is checked against the generated forward bit for bit. Returns how many
/// points' `M₀₁` the fusion changes, the measure of the exposure.
fn check_contraction<R: P>(eps: R) -> usize {
    let mut differ = 0;
    for i in 0..=544 {
        for j in 0..=544 {
            let z = [
                R::of(3.0 + f64::from(i) / 32.0),
                R::of(3.0 + f64::from(j) / 32.0),
            ];
            let (m0, t7, t13) = stick_parts(z);
            let m = <StickBreaking as Link<2, 3>>::forward(z);
            assert_eq!(
                [m[0], m[1]],
                [m0, t7 * t13],
                "the replica is not the generated forward at {z:?}"
            );
            let (apart, fused) = (m0 + t7 * t13, t7.mul_add(t13, m0));
            differ += usize::from(apart != fused);
            assert_eq!(
                apart < eps,
                fused < eps,
                "z_μ = {z:?}: fusing m₀ + t₇·t₁₃ flips the tag at {} ({:e} against {:e})",
                R::NAME,
                apart.f(),
                fused.f()
            );
        }
    }
    differ
}

#[test]
fn decode_m01_tiny_contraction() {
    let d64 = check_contraction(mass::m01_epsilon::<f64>());
    let d32 = check_contraction(mass::m01_epsilon::<f32>());
    eprintln!("decode_m01_tiny_contraction: fusion changes M₀₁ at {d64} points at f64, {d32} at f32, of 545²; no tag flips");
}

/// A threshold set between the separate and the fused `M₀₁` of a point where they differ.
#[cfg(feature = "controls")]
fn straddling() -> f32 {
    for i in 0..=544 {
        for j in 0..=544 {
            let z = [3.0 + i as f32 / 32.0, 3.0 + j as f32 / 32.0];
            let (m0, t7, t13) = stick_parts(z);
            let (apart, fused) = (m0 + t7 * t13, t7.mul_add(t13, m0));
            if apart != fused {
                return apart.max(fused);
            }
        }
    }
    unreachable!("fusion never changes M₀₁")
}

negative_control!(
    decode_m01_tiny_contraction,
    "a threshold between a point's separate and fused M₀₁ must flip",
    expected = "flips the tag",
    {
        check_contraction(straddling());
    }
);

// ── D_cfg ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The tolerance of `‖ρ̃‖ = ‖λ̃‖` at the golden IC and of `‖ρ̃‖² + ‖λ̃‖² = 1`, in `eps`.
const SPHERE_EPS: f64 = 4.0;

/// `z = 0`: `α = π/4` and `β = π/2` (each `(π/2)·½` and `π·½`, exact at `R`), `‖ρ̃‖ = ‖λ̃‖`, `I = 1`.
fn check_golden<R: P>(d: Dec<R>) {
    let m = (d.mass)([R::zero(), R::zero()]);
    let c = (d.config)([R::zero(), R::zero()], &m);
    let half = R::one() / R::of(2.0);
    assert_eq!(
        c.alpha,
        R::LIT_6 * half,
        "z = 0: α is not π/4 at {}",
        R::NAME
    );
    assert_eq!(
        c.beta,
        R::LIT_7 * half,
        "z = 0: β is not π/2 at {}",
        R::NAME
    );
    let (nr, nl) = (norm(c.rho_t), norm(c.lambda_t));
    assert!(
        (nr - nl).abs() <= SPHERE_EPS * R::eps(),
        "z = 0: ‖ρ̃‖ = {nr} ≠ ‖λ̃‖ = {nl} at {}",
        R::NAME
    );
    assert!(
        (inertia(&m, &c) - 1.0).abs() <= INERTIA_EPS * R::eps(),
        "z = 0: I ≠ 1 at {}",
        R::NAME
    );
}

fn norm<R: P>(v: [R; 2]) -> f64 {
    v[0].f().hypot(v[1].f())
}

#[test]
fn decode_config_golden() {
    check_golden(dec::<Defaults, f64>());
    check_golden(dec::<Defaults, f32>());
}

/// α = π·σ(z_α), the β link's width on the α control.
#[cfg(feature = "controls")]
struct WideAlpha;

#[cfg(feature = "controls")]
impl Link<1, 1> for WideAlpha {
    const NAME: &'static str = "wide_alpha";
    type Codomain = Alpha;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidBeta::forward(v)
    }
    fn inverse<R: LinkReal>(_: [R; 1]) -> [R; 1] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 1]) -> R {
        unreachable!()
    }
}

negative_control!(
    decode_config_golden,
    "an α link of width π must fail the golden α",
    expected = "z = 0: α is not π/4",
    check_golden(dec::<
        Select<SoftmaxTanh, WideAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>,
        f64,
    >())
);

/// `‖ρ̃‖² + ‖λ̃‖² = 1` within [`SPHERE_EPS`], `ρ̃` exactly on the x axis, over the domain.
fn check_sphere<R: P>(hyper: fn(R, R) -> ([R; 2], [R; 2]), worst: &Worst) {
    over_z(|_, zc| {
        let (alpha, beta) = config::angles::<Defaults, R>(at::<R, 2>(zc));
        let (rt, lt) = hyper(alpha, beta);
        assert_eq!(
            rt[1],
            R::zero(),
            "z_cfg = {zc:?}: ρ̃ is off the x axis at {}",
            R::NAME
        );
        let err =
            (rt[0].f().powi(2) + lt[0].f().powi(2) + lt[1].f().powi(2) - 1.0).abs() / R::eps();
        worst.see(&format!("‖ρ̃‖² + ‖λ̃‖² − 1 at {}", R::NAME), err);
        assert!(
            err <= SPHERE_EPS,
            "z_cfg = {zc:?}: ‖ρ̃‖² + ‖λ̃‖² − 1 = {err} eps at {}",
            R::NAME
        );
    });
}

#[test]
fn decode_config_sphere() {
    let worst = Worst::default();
    check_sphere::<f64>(config::hyperspherical, &worst);
    check_sphere::<f32>(config::hyperspherical, &worst);
    worst.print("decode_config_sphere");
}

negative_control!(
    decode_config_sphere,
    "λ̃ without its sin α factor must fail the sphere",
    expected = "‖ρ̃‖² + ‖λ̃‖² − 1 =",
    check_sphere::<f64>(
        |a, b| ([a.cos(), 0.0], [b.cos(), b.sin()]),
        &Worst::default()
    )
);

// ── Reconstruction ──────────────────────────────────────────────────────────────────────────────────────────────

/// REQ-DEC-044's proposals for test 1, in `eps`: the CoM relative to the size of its terms, `Σmᵢ‖rᵢ‖`; `I − 1`; and
/// `r₁ − r₀ = ρ` relative to the largest of `‖r₀‖`, `‖r₁‖` and `‖ρ‖`, the size of the subtraction's operands (near
/// `α = π/2`, `ρ → 0` while `r₀` and `r₁` stay near `r₀₁`, so the difference's round-off is relative to them).
const COM_EPS: f64 = 4.0;
const INERTIA_EPS: f64 = 8.0;
const RHO_EPS: f64 = 4.0;

fn inertia<R: P>(m: &Masses<R>, c: &Config<R>) -> f64 {
    (0..3)
        .map(|i| m.m[i].f() * (c.r[i][0].f().powi(2) + c.r[i][1].f().powi(2)))
        .sum()
}

/// `r₁ − r₀ = ρ`, CoM `= 0` and `I = 1` for the positions `r` and inner Jacobi vector `rho` reconstructed with masses
/// `m`.
fn check_positions<R: P>(at_z: &str, m: &Masses<R>, rho: [R; 2], r: [[R; 2]; 3], worst: &Worst) {
    let r = r.map(|ri| ri.map(P::f));
    let mm = m.m.map(P::f);
    let d01 = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let rho = rho.map(P::f);
    let scale = rho[0]
        .hypot(rho[1])
        .max(r[0][0].hypot(r[0][1]))
        .max(r[1][0].hypot(r[1][1]));
    let err = (d01[0] - rho[0]).hypot(d01[1] - rho[1]) / (scale * R::eps());
    worst.see(&format!("r₁ − r₀ vs ρ at {}", R::NAME), err);
    assert!(
        err <= RHO_EPS,
        "{at_z}: r₁ − r₀ − ρ = {err} eps of the operands"
    );
    let com = [0, 1].map(|k| (0..3).map(|i| mm[i] * r[i][k]).sum::<f64>());
    let size: f64 = (0..3).map(|i| mm[i] * r[i][0].hypot(r[i][1])).sum();
    let err = com[0].hypot(com[1]) / (size * R::eps());
    worst.see(&format!("CoM at {}", R::NAME), err);
    assert!(err <= COM_EPS, "{at_z}: the CoM is {err} eps of Σmᵢ‖rᵢ‖");
    let i: f64 = (0..3)
        .map(|k| mm[k] * (r[k][0].powi(2) + r[k][1].powi(2)))
        .sum();
    let err = (i - units::inertia::<f64>()).abs() / R::eps();
    worst.see(&format!("I − 1 at {}", R::NAME), err);
    assert!(err <= INERTIA_EPS, "{at_z}: I − 1 = {err} eps");
}

/// `v` rotated by 0.6 rad.
fn rotated<R: P>(v: [R; 2]) -> [R; 2] {
    let (s, c) = (R::of(0.6f64.sin()), R::of(0.6f64.cos()));
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// `r₁ − r₀ = ρ`, CoM `= 0` and `I = 1` over the domain, for each decoder of `decs`; and for the reconstruction itself
/// on `ρ̃, λ̃` rotated off the canonical frame, since §3.2's unweighting and reconstruction hold for any `ρ̃, λ̃`, and
/// the canonical `ρ̃_y = 0` would hide the y components' terms.
fn check_identities<R: P>(decs: &[(&str, Dec<R>)], worst: &Worst) {
    over_z(|zm, zc| {
        for (name, d) in decs {
            let m = (d.mass)(at::<R, 2>(zm));
            let c = (d.config)(at::<R, 2>(zc), &m);
            let at_z = format!("{name} at z_μ = {zm:?}, z_cfg = {zc:?}, {}", R::NAME);
            check_positions(&at_z, &m, c.rho, c.r, worst);
            let (rho, _, r) = config::reconstruct(&m, rotated(c.rho_t), rotated(c.lambda_t));
            check_positions(&format!("{at_z}, rotated"), &m, rho, r, worst);
        }
    });
}

#[test]
fn decode_reconstruct_identities() {
    let worst = Worst::default();
    check_identities(&both::<f64>(), &worst);
    check_identities(&both::<f32>(), &worst);
    worst.print("decode_reconstruct_identities");
}

/// The reconstruction with the inner pair's weights crossed: `r₀ = r₀₁ − (m₀/M₀₁)ρ`, `r₁ = r₀₁ + (m₁/M₀₁)ρ`.
#[cfg(feature = "controls")]
fn crossed<S: Selection>(z: [f64; 2], m: &Masses<f64>) -> Config<f64> {
    let mut c = config::decode::<S, f64>(z, m);
    let r01 = [-(m.m[2] * c.lambda[0]), -(m.m[2] * c.lambda[1])];
    let (w0, w1) = (m.m[0] / m.m01, m.m[1] / m.m01);
    c.r[0] = [r01[0] - w0 * c.rho[0], r01[1] - w0 * c.rho[1]];
    c.r[1] = [r01[0] + w1 * c.rho[0], r01[1] + w1 * c.rho[1]];
    c
}

negative_control!(
    decode_reconstruct_identities,
    "a reconstruction with the inner pair's weights crossed must fail the CoM",
    expected = "the CoM is",
    check_identities(
        &[(
            "crossed",
            Dec {
                config: crossed::<Defaults>,
                ..dec::<Defaults, f64>()
            }
        )],
        &Worst::default()
    )
);

/// REQ-DEC-044's proposals for test 6, in `eps`: `|Δα|` and `|R̃ − 1|`, and `sin α·|Δβ|`, β's error as an arc on the
/// sphere (β is undefined at `α = 0`, where `λ̃ = 0`).
const INVERSE_EPS: f64 = 8.0;

/// Decoder test 6: from the emitted `(m, rᵢ)` alone, §3.2's definitions give back `ρ̃` on `+x`, `(α, β)` and `R̃ = 1`.
fn check_inverse<R: P>(decs: &[(&str, Dec<R>)], worst: &Worst) {
    over_z(|zm, zc| {
        for (name, d) in decs {
            let m = (d.mass)(at::<R, 2>(zm));
            let c = (d.config)(at::<R, 2>(zc), &m);
            let at_z = format!("{name} at z_μ = {zm:?}, z_cfg = {zc:?}, {}", R::NAME);
            let [m0, m1, m2] = m.m.map(P::f);
            let r = c.r.map(|ri| ri.map(P::f));
            let m01 = m0 + m1;
            let (mu_rho, mu_lambda) = (m0 * m1 / m01, m2 * m01);
            let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
            let r01 = [0, 1].map(|k| (m0 * r[0][k] + m1 * r[1][k]) / m01);
            let lambda = [r[2][0] - r01[0], r[2][1] - r01[1]];
            let rho_t = rho.map(|x| mu_rho.sqrt() * x);
            let lambda_t = lambda.map(|x| mu_lambda.sqrt() * x);
            let (nr, nl) = (rho_t[0].hypot(rho_t[1]), lambda_t[0].hypot(lambda_t[1]));
            let r_t = nr.hypot(nl);
            let alpha = nl.atan2(rho_t[0]);
            let beta = lambda_t[1].atan2(lambda_t[0]);
            let errs = [
                ("|R̃ − 1|", (r_t - 1.0).abs()),
                ("|ρ̃_y|", rho_t[1].abs()),
                ("|Δα|", (alpha - c.alpha.f()).abs()),
                ("sin α·|Δβ|", c.alpha.f().sin() * wrapped(beta - c.beta.f())),
            ];
            for (what, e) in errs {
                let e = e / R::eps();
                worst.see(&format!("{what} at {}", R::NAME), e);
                assert!(e <= INVERSE_EPS, "{at_z}: {what} = {e} eps");
            }
        }
    });
}

#[test]
fn decode_reconstruct_inverse() {
    let worst = Worst::default();
    check_inverse(&both::<f64>(), &worst);
    check_inverse(&both::<f32>(), &worst);
    worst.print("decode_reconstruct_inverse");
}

/// `|d|` for an angle difference `d`, wrapped into `[0, π]`: β's recovered value is `atan2`'s, in `(−π, π]`, and at
/// f32's saturated `β = fl(π)`, past π, `λ̃_y` is negative and `atan2` gives a value near `−π`.
fn wrapped(d: f64) -> f64 {
    d.sin().atan2(d.cos()).abs()
}

/// The reconstruction with body 2 reflected through the inner pair's CoM: `r₂ = −M₀₁λ`.
#[cfg(feature = "controls")]
fn reflected(z: [f64; 2], m: &Masses<f64>) -> Config<f64> {
    let mut c = config::decode::<Defaults, f64>(z, m);
    c.r[2] = [-(m.m01 * c.lambda[0]), -(m.m01 * c.lambda[1])];
    c
}

negative_control!(
    decode_reconstruct_inverse,
    "a reconstruction placing body 2 at −M₀₁λ must fail the inverse",
    expected = "|R̃ − 1| =",
    check_inverse(
        &[(
            "reflected",
            Dec {
                config: reflected,
                ..dec::<Defaults, f64>()
            }
        )],
        &Worst::default()
    )
);

// ── The α orientation ───────────────────────────────────────────────────────────────────────────────────────────

/// Decoder test 9 at each of a few mass points and β: as `z_α` rises from −12 to 12 at f64, −4 to 4 at f32 (α from
/// near 0 to near π/2, short of where `cos α` or `sin α` rounds to 1 at the precision), `‖r₁ − r₀‖` falls and `‖λ‖`
/// rises strictly, so `α → 0` has the largest inner-pair separation and the smallest
/// `‖λ‖`; and `‖ρ̃‖ − ‖λ̃‖` is positive below `α = π/4`, negative above it, and zero to round-off at it.
fn check_orientation<R: P>(d: Dec<R>) {
    let half_width = if R::eps() < 1e-10 { 48 } else { 16 };
    let grid: Vec<f64> = (-half_width..=half_width)
        .map(|k| f64::from(k) / 4.0)
        .collect();
    for zm in [[0.0, 0.0], [1.0, -0.5], [-2.0, 3.0]] {
        let m = (d.mass)(at::<R, 2>(zm));
        for zb in [-3.0, 0.0, 2.0] {
            let cs: Vec<Config<R>> = grid
                .iter()
                .map(|&za| (d.config)(at::<R, 2>([za, zb]), &m))
                .collect();
            let sep: Vec<f64> = cs
                .iter()
                .map(|c| norm([c.r[1][0] - c.r[0][0], c.r[1][1] - c.r[0][1]]))
                .collect();
            let lam: Vec<f64> = cs.iter().map(|c| norm(c.lambda)).collect();
            for k in 1..grid.len() {
                assert!(
                    sep[k] < sep[k - 1],
                    "z_μ = {zm:?}: ‖r₁ − r₀‖ does not fall from z_α = {} to {} at {}",
                    grid[k - 1],
                    grid[k],
                    R::NAME
                );
                assert!(
                    lam[k] > lam[k - 1],
                    "z_μ = {zm:?}: ‖λ‖ does not rise from z_α = {} to {} at {}",
                    grid[k - 1],
                    grid[k],
                    R::NAME
                );
            }
            for (za, c) in grid.iter().zip(&cs) {
                let diff = norm(c.rho_t) - norm(c.lambda_t);
                let ok = if *za < 0.0 {
                    diff > 0.0
                } else if *za > 0.0 {
                    diff < 0.0
                } else {
                    diff.abs() <= SPHERE_EPS * R::eps()
                };
                assert!(
                    ok,
                    "z_α = {za}: ‖ρ̃‖ − ‖λ̃‖ = {diff} does not cross at π/4 at {}",
                    R::NAME
                );
            }
        }
    }
}

#[test]
fn decode_alpha_orientation() {
    check_orientation(dec::<Defaults, f64>());
    check_orientation(dec::<Defaults, f32>());
}

/// The orientation gotcha itself: `ρ̃ = sin α·(1, 0)`, `λ̃ = cos α·(cos β, sin β)`.
#[cfg(feature = "controls")]
fn swapped(z: [f64; 2], m: &Masses<f64>) -> Config<f64> {
    let (alpha, beta) = config::angles::<Defaults, f64>(z);
    let rho_t = [alpha.sin(), 0.0];
    let lambda_t = [alpha.cos() * beta.cos(), alpha.cos() * beta.sin()];
    let (rho, lambda, r) = config::reconstruct(m, rho_t, lambda_t);
    Config {
        alpha,
        beta,
        rho_t,
        lambda_t,
        rho,
        lambda,
        r,
    }
}

negative_control!(
    decode_alpha_orientation,
    "a decode with ‖ρ̃‖ = sin α must fail the orientation",
    expected = "does not fall",
    check_orientation(Dec {
        config: swapped,
        ..dec::<Defaults, f64>()
    })
);

// ── Units ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// `G = 1`, and over a set of `z` the decoded ICs have `Σmᵢ = M = 1` and `I = 1` (test 1's tolerances).
fn check_units<R: P>(g: f64, d: Dec<R>) {
    assert_eq!(g, 1.0, "G = {g}, not 1");
    for zm in fixed_pairs() {
        let m = (d.mass)(at::<R, 2>(zm));
        let c = (d.config)(at::<R, 2>([0.5, -1.5]), &m);
        let total: f64 = m.m.iter().map(|x| x.f()).sum();
        assert!(
            (total - units::total_mass::<f64>()).abs() <= SUM_EPS * R::eps(),
            "z_μ = {zm:?}: the total mass is {total}, not 1, at {}",
            R::NAME
        );
        assert!(
            (inertia(&m, &c) - units::inertia::<f64>()).abs() <= INERTIA_EPS * R::eps(),
            "z_μ = {zm:?}: I is not 1 at {}",
            R::NAME
        );
    }
}

#[test]
fn units_dimensionless() {
    assert_eq!(units::g::<f32>(), 1.0, "G = 1 at f32");
    assert_eq!(units::total_mass::<f64>(), 1.0, "M = 1");
    assert_eq!(units::inertia::<f64>(), 1.0, "I = 1");
    check_units(units::g::<f64>(), dec::<Defaults, f64>());
    check_units(units::g::<f32>() as f64, dec::<Stick, f32>());
}

negative_control!(
    units_dimensionless,
    "G in SI units must fail",
    expected = "not 1",
    check_units(6.674e-11, dec::<Defaults, f64>())
);
