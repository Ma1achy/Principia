//! The primitive registry (design `principia_chart_composition.md` § 3, § 3.3; TASK-M2-66): one table, CPU-side, of
//! the quantities an axis may be built from. Every row declares its forward (state to value), its inverse class
//! (`closed`, `numerical` or `none`), its kind (one of the four, [`AxisKind`]), its natural extension, whether it is
//! angle-like, its scale (`free`, `gauge` or `size-only`) and its class (a state function, a conserved quantity, exact
//! or approximate, or a trajectory functional, which is never an axis). [`AxisSpec`] and [`ChartSpec`] are the
//! authoring layer's data: a chart as a pair of axes, each a primitive or a typed combination, with its warp, its
//! extension and what it holds, and the chart's constraints and slice. The kernel never sees any of it
//! (REQ-CHART-004): `validate::validate_spec` checks a spec against the refusal table and `lowering::resolve_spec`
//! lowers it.
//!
//! **What is registered.** A row with an inverse ships with a round-trip case (`round_trip_complete`), so the table
//! holds the rows whose inverse exists in code today: the latent coordinates, `α`, `β`, the Jacobi ratio, the mass
//! ratios, `E`, `K`, `KE`, `L_z` and the virial ratio, each with its own inverse here ([`Row::invert`]); and the axes of
//! today's charts that are closed only on their chart's pair (`θ` and `φ`, `ν`, the acute `θ`, `δm`, the mass
//! simplex's and the Euclid plane's), whose inverse is that chart's encode. The forward-only rows (`PE`, the separation
//! ratio, a pair's binding energy, the inner eccentricity), the size-only rows (`I`, `R`, `ℓ`) and one functional
//! (the minimum separation over a run) are registered so that picking them refuses by name. The plan is unclear here:
//! § 3.1's and § 3.2's further rows are TASK-M2-74's and TASK-M2-76's with their round trips, so they are left to
//! those tasks rather than registered with no case.
//!
//! **Slopes.** Every row declares a slope tier ([`Slope`]): exact, its forward written once more over
//! `kernel::dual::Dual` (the `d…` functions), so its slope along any of the state's 15 components is the dual's; or none,
//! for a row with no forward of its own (the rows closed only on a chart's pair, whose slope is that chart's), a functional
//! and an unclassified row. A row flags the kink sites of its forward in [`Row::kinks`]. The tier is `None` in `base`, so a
//! row added without one is reported by `slope_tier_declared`. The implicit tier (a slope by the implicit-function rule
//! on a numerical inverse) has no row today and no variant here.
//!
//! **The shape sphere's `θ` and `φ`** are read in the chart's home frame, `n₀ = (1, 0, 0)`, `e₁ = (0, 1, 0)`,
//! `e₂ = (0, 0, 1)` (chart_reference § 3.1): `φ = acos n·n₀`, `θ = atan2(n·e₂, n·e₁)`. In that frame `φ = 2α` and
//! `θ = β`, which the pair table records.

use std::f64::consts::TAU;

use kernel::chart::BlockTouch;
use kernel::chart::Float;
use kernel::decode::{decode, Ic, Latent as Controls};
use kernel::dual::Dual;
use kernel::encode::encode;
use kernel::generated::links::{Defaults, Link, Selection};
use kernel::shape::{shape_vec, weighted_jacobi};
use kernel::slope::{Kink, Tier};
use kernel::stage::decode::{
    BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults, BurrauThetaLzDefaults,
    DecodeVariant, EuclidPlaneDefaults, InvariantLEDefaults, InvariantLKDefaults,
    MassSimplexDefaults, MixedAlphaVirialDefaults, MixedMu1AlphaDefaults, ShapeSphereDefaults,
};
use kernel::Real;

use super::axis::{AxisKind, Block};
use crate::contract::sim_config::Latent;

/// A row's inverse class (§ 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Inverse {
    /// A closed inverse: per row (`on_pair` `None`), or only on its chart's pair, the chart's encode.
    Closed {
        /// The chart whose pair the inverse is closed on, by `kernel::chart::REGISTRY` name.
        on_pair: Option<&'static str>,
    },
    /// A registered numerical inverse, naming what it solves for and its branch rule (§ 7.3).
    Numerical {
        /// What it solves for.
        solves_for: &'static str,
        /// Its branch rule, a pure function of the pixel.
        branch: &'static str,
    },
    /// No inverse: a forward-only overlay row.
    None,
}

impl Inverse {
    /// Whether the row has an inverse, closed or numerical.
    pub fn exists(self) -> bool {
        !matches!(self, Inverse::None)
    }
}

/// A row's scale (§ 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    /// A shape or a ratio.
    Free,
    /// Fixed by `I = 1`: a paper's value needs the recorded rescale.
    Gauge,
    /// Constant at every decode, `I = 1`: refused as an axis.
    SizeOnly,
}

/// How a conserved row is conserved (§ 3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exactness {
    /// Constant under the equations of motion at `ε = 0`, a run's drift being integration error.
    Exact,
    /// Steady only in a named limit, labelled `approximate`.
    Approximate,
}

/// A row's class (§ 3.3; the chart-space model § 6b).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// A state function.
    State,
    /// A conserved quantity.
    Conserved(Exactness),
    /// A trajectory functional: many-to-one, never an axis.
    Functional,
    /// The row's definition is unread: a definition requirement, no axis.
    Unclassified,
}

/// A row's natural extension past the unit square (§ 3; chart_reference § 5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extension {
    /// Affine.
    Affine,
    /// Bounded.
    Bounded,
    /// Periodic.
    Periodic,
    /// None: a row that is no axis.
    None,
}

/// A chart-level constraint (§ 5.3): hold `E`, hold `L_z`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// Hold `E`: fixes the momentum scale.
    E,
    /// Hold `L_z`: fixes the momentum's rigid part.
    Lz,
}

impl Hold {
    /// The held quantity's name.
    pub fn name(self) -> &'static str {
        match self {
            Hold::E => "E",
            Hold::Lz => "L_z",
        }
    }
}

/// A state's value of a row.
pub type Forward = fn(&Ic<f64>) -> f64;

/// A row's own inverse: the state `ic` moved so that the row reads `x`, holding the row's `Holds`; `None` where `x`
/// has no state there.
pub type Invert = fn(&Ic<f64>, f64) -> Option<Ic<f64>>;

/// A row's forward on the dual numbers: the same value as [`Forward`], with its slope along the seeded state component.
pub type DualForward = fn(&Ic<Dual>) -> Dual;

/// A row's declared slope (design § 3; the module's note): its tier, and for the exact tier the forward that gives it.
#[derive(Clone, Copy, Debug)]
pub enum Slope {
    /// Tier 1: the row's slope along each state component is its [`DualForward`]'s.
    Exact(DualForward),
    /// Tier 3: no slope; each sample is evaluated on the CPU, and the row is still a valid axis.
    None,
}

impl Slope {
    /// The tier this declares.
    pub fn tier(self) -> Tier {
        match self {
            Slope::Exact(_) => Tier::Exact,
            Slope::None => Tier::None,
        }
    }
}

/// One row of the registry (the module's note).
#[derive(Clone, Copy, Debug)]
pub struct Row {
    /// The row's name, as an axis shows it.
    pub name: &'static str,
    /// Its kind; `None` for a row that is no axis.
    pub kind: Option<AxisKind>,
    /// The blocks its inverse writes: a kind-3 row its sector, any other row the blocks it lies in.
    pub writes: BlockTouch,
    /// Its inverse class.
    pub inverse: Inverse,
    /// Its natural extension.
    pub extension: Extension,
    /// Whether it is angle-like (R-407 clause 3: a periodic or affine declaration needs physics review).
    pub angle_like: bool,
    /// Its scale.
    pub scale: Scale,
    /// Its class.
    pub class: Class,
    /// What its inverse holds fixed, in words (design § 12, "What § 3's solves hold").
    pub holds: &'static str,
    /// Whether chart_reference § 2.1 writes a warp for it: `PE` has none (REQ-CHART-070).
    pub warp_written: bool,
    /// The constraint that fixes it: an axis of it beside that hold is `AxisAndHold`.
    pub fixed_by: Option<Hold>,
    /// Its forward; `None` for a functional, and for a row closed only on its chart's pair, whose forward is that
    /// chart's encode (`kernel::encode::by_kind`).
    pub forward: Option<Forward>,
    /// Its own inverse, for a row whose inverse is closed per row or numerical.
    pub invert: Option<Invert>,
    /// Its definition as a combination of other rows, for a row a typed expression can match.
    pub definition: Option<fn() -> Expr>,
    /// Its slope declaration. `None` is a row that has declared nothing, which `slope_tier_declared` reports: a row a
    /// later task adds is caught with no wiring of its own.
    pub slope: Option<Slope>,
    /// The kink sites it flags: where its forward changes branch. Empty for a tier-3 row.
    pub kinks: &'static [Kink],
}

/// An expression over rows, in the stain's expression syntax's subset (§ 3, "Combinations"; R-509).
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A row, by name.
    Row(String),
    /// A constant.
    Const(f64),
    /// A sum.
    Add(Box<Expr>, Box<Expr>),
    /// A product.
    Mul(Box<Expr>, Box<Expr>),
    /// A quotient.
    Div(Box<Expr>, Box<Expr>),
    /// A negation.
    Neg(Box<Expr>),
    /// `exp`.
    Exp(Box<Expr>),
}

impl Expr {
    /// The row `name`.
    pub fn row(name: &str) -> Expr {
        Expr::Row(name.to_owned())
    }

    /// `a + b`.
    pub fn sum(a: Expr, b: Expr) -> Expr {
        Expr::Add(Box::new(a), Box::new(b))
    }

    /// `a·b`.
    pub fn product(a: Expr, b: Expr) -> Expr {
        Expr::Mul(Box::new(a), Box::new(b))
    }

    /// `a/b`.
    pub fn quotient(a: Expr, b: Expr) -> Expr {
        Expr::Div(Box::new(a), Box::new(b))
    }

    /// `−a`.
    pub fn negated(a: Expr) -> Expr {
        Expr::Neg(Box::new(a))
    }

    /// `exp(a)`.
    pub fn exp(a: Expr) -> Expr {
        Expr::Exp(Box::new(a))
    }

    /// The rows it reads, each once, in first-read order.
    pub fn parts(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<String>) {
        match self {
            Expr::Row(name) => {
                if !out.contains(name) {
                    out.push(name.clone());
                }
            }
            Expr::Const(_) => {}
            Expr::Add(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                a.collect(out);
                b.collect(out);
            }
            Expr::Neg(a) | Expr::Exp(a) => a.collect(out),
        }
    }

    /// Its value at `ic`, each row through its forward; `None` where a row has no forward.
    pub fn eval(&self, registry: &Registry, ic: &Ic<f64>) -> Option<f64> {
        Some(match self {
            Expr::Row(name) => (registry.row(name)?.forward?)(ic),
            Expr::Const(c) => *c,
            Expr::Add(a, b) => a.eval(registry, ic)? + b.eval(registry, ic)?,
            Expr::Mul(a, b) => a.eval(registry, ic)? * b.eval(registry, ic)?,
            Expr::Div(a, b) => a.eval(registry, ic)? / b.eval(registry, ic)?,
            Expr::Neg(a) => -a.eval(registry, ic)?,
            Expr::Exp(a) => libm::exp(a.eval(registry, ic)?),
        })
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Row(name) => f.write_str(name),
            Expr::Const(c) => write!(f, "{c}"),
            Expr::Add(a, b) => write!(f, "({a} + {b})"),
            Expr::Mul(a, b) => write!(f, "{a}·{b}"),
            Expr::Div(a, b) => write!(f, "{a}/{b}"),
            Expr::Neg(a) => write!(f, "−{a}"),
            Expr::Exp(a) => write!(f, "exp({a})"),
        }
    }
}

/// The latent coordinates' rows, by coordinate index (`axis::COORDINATES`).
pub const LATENT_ROWS: [&str; 8] = ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1", "z_μ2"];

/// What a row declares that is inconsistent ([`check_row`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowError {
    /// The row.
    pub row: &'static str,
    /// What is wrong.
    pub why: &'static str,
}

/// A row's declarations checked against each other: a functional or unclassified row has no inverse and no kind; a
/// size-only row has no inverse; a row with an inverse has a kind, and a forward unless its inverse is its chart's
/// pair's; a row with an inverse of its own has it in code, and a numerical one names what it solves for. Nothing
/// guards a row's subject: a row with an inverse may name a binary phase, an eccentricity, an impact parameter or a
/// fixed-`k` prescription (R-847 reverses R-669; § 12's `combination_class`).
pub fn check_row(row: &Row) -> Result<(), RowError> {
    let fail = |why| Err(RowError { row: row.name, why });
    let inverse = row.inverse.exists();
    match row.class {
        Class::Functional | Class::Unclassified if inverse || row.kind.is_some() => {
            return fail("a functional or unclassified row has no inverse and no kind")
        }
        _ => {}
    }
    if row.scale == Scale::SizeOnly && inverse {
        return fail("a size-only row has no inverse");
    }
    if inverse && row.kind.is_none() {
        return fail("a row with an inverse has a kind");
    }
    match row.inverse {
        Inverse::Closed { on_pair: None } | Inverse::Numerical { .. }
            if row.forward.is_none() || row.invert.is_none() =>
        {
            fail("an inverse of the row's own has its forward and its inverse in code")
        }
        Inverse::Closed { on_pair: Some(_) } if row.invert.is_some() => {
            fail("an inverse closed only on the pair is the chart's, not the row's")
        }
        Inverse::None if row.invert.is_some() => fail("a row classed none has no inverse"),
        _ => Ok(()),
    }
}

// ── Forwards ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The controls of `ic` under the default links.
fn controls(ic: &Ic<f64>) -> [f64; 8] {
    encode::<Defaults, f64>(ic).z.z
}

/// The controls' coordinate `K`.
fn z<const K: usize>(ic: &Ic<f64>) -> f64 {
    controls(ic)[K]
}

/// `α`, `‖ρ̃‖ = cos α`.
fn alpha(ic: &Ic<f64>) -> f64 {
    encode::<Defaults, f64>(ic).blocks.alpha
}

/// `β`, `λ̃`'s angle from `+x`.
fn beta(ic: &Ic<f64>) -> f64 {
    encode::<Defaults, f64>(ic).blocks.beta
}

/// `‖v‖`.
fn norm(v: [f64; 2]) -> f64 {
    libm::hypot(v[0], v[1])
}

/// The unweighted Jacobi vectors `ρ = r₁ − r₀`, `λ = r₂ − r₀₁`.
fn jacobi(ic: &Ic<f64>) -> ([f64; 2], [f64; 2]) {
    let [m0, m1, _] = ic.m;
    let r = ic.r;
    let r01 = [
        (m0 * r[0][0] + m1 * r[1][0]) / (m0 + m1),
        (m0 * r[0][1] + m1 * r[1][1]) / (m0 + m1),
    ];
    (
        [r[1][0] - r[0][0], r[1][1] - r[0][1]],
        [r[2][0] - r01[0], r[2][1] - r01[1]],
    )
}

/// The Jacobi ratio `‖λ‖/‖ρ‖`.
fn jacobi_ratio(ic: &Ic<f64>) -> f64 {
    let (rho, lambda) = jacobi(ic);
    norm(lambda) / norm(rho)
}

/// The shape vector `n` (chart_reference § 3.1).
fn shape(ic: &Ic<f64>) -> [f64; 3] {
    let (rho_t, lambda_t) = weighted_jacobi(ic.r, ic.m);
    shape_vec(rho_t, lambda_t)
}

/// The shape sphere's azimuth `θ` in the home frame (the module's note).
fn shape_theta(ic: &Ic<f64>) -> f64 {
    let n = shape(ic);
    libm::atan2(n[2], n[1]).rem_euclid(TAU)
}

/// The shape sphere's polar angle `φ` in the home frame.
fn shape_phi(ic: &Ic<f64>) -> f64 {
    libm::acos(shape(ic)[0].clamp(-1.0, 1.0))
}

/// `m₁/m₀`.
fn m1_over_m0(ic: &Ic<f64>) -> f64 {
    ic.m[1] / ic.m[0]
}

/// `m₀/m₁`.
fn m0_over_m1(ic: &Ic<f64>) -> f64 {
    ic.m[0] / ic.m[1]
}

/// `m₂/m₀`.
fn m2_over_m0(ic: &Ic<f64>) -> f64 {
    ic.m[2] / ic.m[0]
}

/// `r_ij`.
fn separation(ic: &Ic<f64>, i: usize, j: usize) -> f64 {
    norm([ic.r[i][0] - ic.r[j][0], ic.r[i][1] - ic.r[j][1]])
}

/// The kinetic energy `Σ ‖pᵢ‖²/2mᵢ`.
pub fn kinetic(ic: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| (ic.p[i][0] * ic.p[i][0] + ic.p[i][1] * ic.p[i][1]) / (2.0 * ic.m[i]))
        .sum()
}

/// The potential energy `U = −Σ mᵢmⱼ/r_ij`, `G = 1`, `ε = 0`.
pub fn potential(ic: &Ic<f64>) -> f64 {
    let m = ic.m;
    -(m[0] * m[1] / separation(ic, 0, 1)
        + m[0] * m[2] / separation(ic, 0, 2)
        + m[1] * m[2] / separation(ic, 1, 2))
}

/// `E = K + U`.
pub fn energy(ic: &Ic<f64>) -> f64 {
    kinetic(ic) + potential(ic)
}

/// `L_z = Σ rᵢ ∧ pᵢ`.
pub fn angular_momentum(ic: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| ic.r[i][0] * ic.p[i][1] - ic.r[i][1] * ic.p[i][0])
        .sum()
}

/// The virial ratio `Q = 2K/|U|`.
fn virial(ic: &Ic<f64>) -> f64 {
    2.0 * kinetic(ic) / -potential(ic)
}

/// `I = Σ mᵢ‖rᵢ‖²`.
fn inertia(ic: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| ic.m[i] * (ic.r[i][0] * ic.r[i][0] + ic.r[i][1] * ic.r[i][1]))
        .sum()
}

/// `R = ℓ = √I` (chart_reference § 0.5).
fn size(ic: &Ic<f64>) -> f64 {
    inertia(ic).sqrt()
}

/// `min(r_ij)/max(r_ij)`.
fn separation_ratio(ic: &Ic<f64>) -> f64 {
    let s = [
        separation(ic, 0, 1),
        separation(ic, 0, 2),
        separation(ic, 1, 2),
    ];
    s.iter().copied().fold(f64::INFINITY, f64::min) / s.iter().copied().fold(0.0, f64::max)
}

/// The relative velocity of body 1 about body 0.
fn velocity01(ic: &Ic<f64>) -> [f64; 2] {
    let v = |i: usize| [ic.p[i][0] / ic.m[i], ic.p[i][1] / ic.m[i]];
    let (v0, v1) = (v(0), v(1));
    [v1[0] - v0[0], v1[1] - v0[1]]
}

/// The binding energy of the pair 0, 1: `½μv² − m₀m₁/r₀₁`.
fn binding01(ic: &Ic<f64>) -> f64 {
    let [m0, m1, _] = ic.m;
    let mu = m0 * m1 / (m0 + m1);
    let v = velocity01(ic);
    0.5 * mu * (v[0] * v[0] + v[1] * v[1]) - m0 * m1 / separation(ic, 0, 1)
}

/// The osculating eccentricity of the pair 0, 1: `√(1 + 2εh²/M²)`, `ε` and `h` per unit reduced mass.
fn eccentricity01(ic: &Ic<f64>) -> f64 {
    let total = ic.m[0] + ic.m[1];
    let (rho, _) = jacobi(ic);
    let v = velocity01(ic);
    let eps = 0.5 * (v[0] * v[0] + v[1] * v[1]) - total / norm(rho);
    let h = rho[0] * v[1] - rho[1] * v[0];
    (1.0 + 2.0 * eps * h * h / (total * total)).max(0.0).sqrt()
}

// ── Inverses ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The decode of `z` under the default links.
fn decoded(z: [f64; 8]) -> Ic<f64> {
    decode::<Defaults, f64>(&Controls::new(z)).ic
}

/// `ic` with its control `K` set to `x`, the other controls held.
fn set_z<const K: usize>(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    let mut z = controls(ic);
    z[K] = x;
    x.is_finite().then(|| decoded(z))
}

/// The control `z[0]`.
fn z_alpha(ic: &Ic<f64>) -> f64 {
    z::<0>(ic)
}

/// `ic` with `z[0] = x`.
fn set_z_alpha(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<0>(ic, x)
}

/// The control `z[1]`.
fn z_beta(ic: &Ic<f64>) -> f64 {
    z::<1>(ic)
}

/// `ic` with `z[1] = x`.
fn set_z_beta(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<1>(ic, x)
}

/// The control `z[2]`.
fn z_q0(ic: &Ic<f64>) -> f64 {
    z::<2>(ic)
}

/// `ic` with `z[2] = x`.
fn set_z_q0(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<2>(ic, x)
}

/// The control `z[3]`.
fn z_q1(ic: &Ic<f64>) -> f64 {
    z::<3>(ic)
}

/// `ic` with `z[3] = x`.
fn set_z_q1(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<3>(ic, x)
}

/// The control `z[4]`.
fn z_q2(ic: &Ic<f64>) -> f64 {
    z::<4>(ic)
}

/// `ic` with `z[4] = x`.
fn set_z_q2(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<4>(ic, x)
}

/// The control `z[5]`.
fn z_q3(ic: &Ic<f64>) -> f64 {
    z::<5>(ic)
}

/// `ic` with `z[5] = x`.
fn set_z_q3(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<5>(ic, x)
}

/// The control `z[6]`.
fn z_mu1(ic: &Ic<f64>) -> f64 {
    z::<6>(ic)
}

/// `ic` with `z[6] = x`.
fn set_z_mu1(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<6>(ic, x)
}

/// The control `z[7]`.
fn z_mu2(ic: &Ic<f64>) -> f64 {
    z::<7>(ic)
}

/// `ic` with `z[7] = x`.
fn set_z_mu2(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    set_z::<7>(ic, x)
}

/// `ic` at `α = x`, `β`, the masses and the momentum controls held.
fn set_alpha(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    if !(x > 0.0 && x < std::f64::consts::FRAC_PI_2) {
        return None;
    }
    let [z] = <<Defaults as Selection>::AlphaLink as Link<1, 1>>::inverse([x]);
    set_z::<0>(ic, z)
}

/// `ic` at `β = x`, `α`, the masses and the momentum controls held.
fn set_beta(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    let [z] = <<Defaults as Selection>::BetaLink as Link<1, 1>>::inverse([x]);
    set_z::<1>(ic, z)
}

/// `ic` at `‖λ‖/‖ρ‖ = x`: `tan α = x·√(μ_λ/μ_ρ)`, `β` and the masses held.
fn set_jacobi_ratio(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    let [m0, m1, m2] = ic.m;
    let mu_rho = m0 * m1 / (m0 + m1);
    let mu_lambda = m2 * (m0 + m1);
    (x > 0.0).then_some(())?;
    set_alpha(ic, libm::atan(x * (mu_lambda / mu_rho).sqrt()))
}

/// `ic` at the masses `m`, the configuration and momentum controls held; `None` where the mass link does not reach
/// `m`.
fn set_masses(ic: &Ic<f64>, m: [f64; 3]) -> Option<Ic<f64>> {
    if !m.iter().all(|&x| x > 0.0 && x.is_finite()) {
        return None;
    }
    let mut z = controls(ic);
    let [a, b] = <<Defaults as Selection>::MassLink as Link<2, 3>>::inverse(m);
    // Masses past the link's range come back clamped, another state: that target has no state here.
    let back = <<Defaults as Selection>::MassLink as Link<2, 3>>::forward([a, b]);
    if (0..3).any(|i| (back[i] - m[i]).abs() > 1e-12) {
        return None;
    }
    (z[6], z[7]) = (a, b);
    Some(decoded(z))
}

/// `m₁/m₀ = x`, `m₂` held.
fn set_m1_over_m0(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    let rest = 1.0 - ic.m[2];
    let m0 = rest / (1.0 + x);
    set_masses(ic, [m0, rest - m0, ic.m[2]])
}

/// `m₀/m₁ = x`, `m₂` held.
fn set_m0_over_m1(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    let rest = 1.0 - ic.m[2];
    let m1 = rest / (1.0 + x);
    set_masses(ic, [rest - m1, m1, ic.m[2]])
}

/// `m₂/m₀ = x`, `m₁` held.
fn set_m2_over_m0(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    let rest = 1.0 - ic.m[1];
    let m0 = rest / (1.0 + x);
    set_masses(ic, [m0, ic.m[1], rest - m0])
}

/// The momenta scaled to the kinetic energy `k`, their direction held (inverse_encode_contract Part 5, kind 3).
fn scale_to(ic: &Ic<f64>, k: f64) -> Option<Ic<f64>> {
    let now = kinetic(ic);
    if !(now > 0.0 && k >= 0.0 && k.is_finite()) {
        return None;
    }
    let c = (k / now).sqrt();
    let mut out = *ic;
    out.p = ic.p.map(|p| [c * p[0], c * p[1]]);
    Some(out)
}

/// `K = x`.
fn set_kinetic(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    scale_to(ic, x)
}

/// `E = x`: `K* = x − U`.
fn set_energy(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    scale_to(ic, x - potential(ic))
}

/// `Q = x`: `K* = x·|U|/2`.
fn set_virial(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    scale_to(ic, x * -potential(ic) / 2.0)
}

/// `L_z = x`: the rigid part set, `pᵢ += c·mᵢ·Jrᵢ`, `c = (x − L_z)/I`, the part orthogonal to `Jr` in `⟨·,·⟩_m` held
/// (`v^(L)`, inverse_encode_contract Part 5, kind 3); `Σ mᵢJrᵢ = 0` at the centre of mass, so `Σp` stays 0.
fn set_angular_momentum(ic: &Ic<f64>, x: f64) -> Option<Ic<f64>> {
    if !x.is_finite() {
        return None;
    }
    let c = (x - angular_momentum(ic)) / inertia(ic);
    let mut out = *ic;
    for i in 0..3 {
        out.p[i][0] -= c * ic.m[i] * ic.r[i][1];
        out.p[i][1] += c * ic.m[i] * ic.r[i][0];
    }
    Some(out)
}

/// `Q`'s definition, `2·K/(−PE)`.
fn virial_definition() -> Expr {
    Expr::quotient(
        Expr::product(Expr::Const(2.0), Expr::row("K")),
        Expr::negated(Expr::row("PE")),
    )
}

// ── Forwards on the dual numbers ──────────────────────────────────────────────────────────────────────────────────

// Each is the f64 forward above written once more over `Dual`, the same formula, so its value is the forward's (a test
// holds it to that) and its slope is the exact one. Don't call the f64 forward's `libm` or `f64::clamp`: a dual has
// neither, and a clamp's slope is the kink the row flags.

/// The controls' coordinate `K`, the encode at `Dual`.
fn dz<const K: usize>(ic: &Ic<Dual>) -> Dual {
    encode::<Defaults, Dual>(ic).z.z[K]
}

fn dz_0(ic: &Ic<Dual>) -> Dual {
    dz::<0>(ic)
}

fn dz_1(ic: &Ic<Dual>) -> Dual {
    dz::<1>(ic)
}

fn dz_2(ic: &Ic<Dual>) -> Dual {
    dz::<2>(ic)
}

fn dz_3(ic: &Ic<Dual>) -> Dual {
    dz::<3>(ic)
}

fn dz_4(ic: &Ic<Dual>) -> Dual {
    dz::<4>(ic)
}

fn dz_5(ic: &Ic<Dual>) -> Dual {
    dz::<5>(ic)
}

fn dz_6(ic: &Ic<Dual>) -> Dual {
    dz::<6>(ic)
}

fn dz_7(ic: &Ic<Dual>) -> Dual {
    dz::<7>(ic)
}

fn dalpha(ic: &Ic<Dual>) -> Dual {
    encode::<Defaults, Dual>(ic).blocks.alpha
}

fn dbeta(ic: &Ic<Dual>) -> Dual {
    encode::<Defaults, Dual>(ic).blocks.beta
}

fn dnorm(v: [Dual; 2]) -> Dual {
    Real::hypotenuse(v[0], v[1])
}

fn djacobi(ic: &Ic<Dual>) -> ([Dual; 2], [Dual; 2]) {
    let [m0, m1, _] = ic.m;
    let r = ic.r;
    let r01 = [
        (m0 * r[0][0] + m1 * r[1][0]) / (m0 + m1),
        (m0 * r[0][1] + m1 * r[1][1]) / (m0 + m1),
    ];
    (
        [r[1][0] - r[0][0], r[1][1] - r[0][1]],
        [r[2][0] - r01[0], r[2][1] - r01[1]],
    )
}

fn djacobi_ratio(ic: &Ic<Dual>) -> Dual {
    let (rho, lambda) = djacobi(ic);
    dnorm(lambda) / dnorm(rho)
}

fn dshape(ic: &Ic<Dual>) -> [Dual; 3] {
    let (rho_t, lambda_t) = weighted_jacobi(ic.r, ic.m);
    shape_vec(rho_t, lambda_t)
}

/// `θ`'s branch cut at `θ = 0` is `rem_euclid`'s; its slope is the angle's on either side.
fn dshape_theta(ic: &Ic<Dual>) -> Dual {
    let n = dshape(ic);
    let t = Real::arctan2(n[2], n[1]);
    if t.v < 0.0 {
        t + Dual::constant(TAU)
    } else {
        t
    }
}

fn dshape_phi(ic: &Ic<Dual>) -> Dual {
    let x = dshape(ic)[0];
    let one = Dual::constant(1.0);
    Real::arccosine(Float::max(Float::min(x, one), -one))
}

fn dm1_over_m0(ic: &Ic<Dual>) -> Dual {
    ic.m[1] / ic.m[0]
}

fn dm0_over_m1(ic: &Ic<Dual>) -> Dual {
    ic.m[0] / ic.m[1]
}

fn dm2_over_m0(ic: &Ic<Dual>) -> Dual {
    ic.m[2] / ic.m[0]
}

fn dseparation(ic: &Ic<Dual>, i: usize, j: usize) -> Dual {
    dnorm([ic.r[i][0] - ic.r[j][0], ic.r[i][1] - ic.r[j][1]])
}

fn dkinetic(ic: &Ic<Dual>) -> Dual {
    let two = Dual::constant(2.0);
    (0..3)
        .map(|i| (ic.p[i][0] * ic.p[i][0] + ic.p[i][1] * ic.p[i][1]) / (two * ic.m[i]))
        .fold(Dual::constant(0.0), |a, b| a + b)
}

fn dpotential(ic: &Ic<Dual>) -> Dual {
    let m = ic.m;
    -(m[0] * m[1] / dseparation(ic, 0, 1)
        + m[0] * m[2] / dseparation(ic, 0, 2)
        + m[1] * m[2] / dseparation(ic, 1, 2))
}

fn denergy(ic: &Ic<Dual>) -> Dual {
    dkinetic(ic) + dpotential(ic)
}

fn dangular_momentum(ic: &Ic<Dual>) -> Dual {
    (0..3)
        .map(|i| ic.r[i][0] * ic.p[i][1] - ic.r[i][1] * ic.p[i][0])
        .fold(Dual::constant(0.0), |a, b| a + b)
}

fn dvirial(ic: &Ic<Dual>) -> Dual {
    Dual::constant(2.0) * dkinetic(ic) / -dpotential(ic)
}

fn dinertia(ic: &Ic<Dual>) -> Dual {
    (0..3)
        .map(|i| ic.m[i] * (ic.r[i][0] * ic.r[i][0] + ic.r[i][1] * ic.r[i][1]))
        .fold(Dual::constant(0.0), |a, b| a + b)
}

fn dsize(ic: &Ic<Dual>) -> Dual {
    Float::sqrt(dinertia(ic))
}

/// `min/max`: the slope is that of the smallest over the largest separation, switching where two separations tie.
fn dseparation_ratio(ic: &Ic<Dual>) -> Dual {
    let s = [
        dseparation(ic, 0, 1),
        dseparation(ic, 0, 2),
        dseparation(ic, 1, 2),
    ];
    let (lo, hi) = (
        Float::min(Float::min(s[0], s[1]), s[2]),
        Float::max(Float::max(s[0], s[1]), s[2]),
    );
    lo / hi
}

fn dvelocity01(ic: &Ic<Dual>) -> [Dual; 2] {
    let v = |i: usize| [ic.p[i][0] / ic.m[i], ic.p[i][1] / ic.m[i]];
    let (v0, v1) = (v(0), v(1));
    [v1[0] - v0[0], v1[1] - v0[1]]
}

fn dbinding01(ic: &Ic<Dual>) -> Dual {
    let [m0, m1, _] = ic.m;
    let mu = m0 * m1 / (m0 + m1);
    let v = dvelocity01(ic);
    Dual::constant(0.5) * mu * (v[0] * v[0] + v[1] * v[1]) - m0 * m1 / dseparation(ic, 0, 1)
}

fn deccentricity01(ic: &Ic<Dual>) -> Dual {
    let total = ic.m[0] + ic.m[1];
    let (rho, _) = djacobi(ic);
    let v = dvelocity01(ic);
    let eps = Dual::constant(0.5) * (v[0] * v[0] + v[1] * v[1]) - total / dnorm(rho);
    let h = rho[0] * v[1] - rho[1] * v[0];
    let under = Float::max(
        Dual::constant(1.0) + Dual::constant(2.0) * eps * h * h / (total * total),
        Dual::constant(0.0),
    );
    Float::sqrt(under)
}

// ── The table ─────────────────────────────────────────────────────────────────────────────────────────────────────

const CONFIG: BlockTouch = BlockTouch {
    config: true,
    momentum: false,
    mass: false,
};
const MOMENTUM: BlockTouch = BlockTouch {
    config: false,
    momentum: true,
    mass: false,
};
const MASS: BlockTouch = BlockTouch {
    config: false,
    momentum: false,
    mass: true,
};
const CONFIG_MASS: BlockTouch = BlockTouch {
    config: true,
    momentum: false,
    mass: true,
};
const NONE: BlockTouch = BlockTouch {
    config: false,
    momentum: false,
    mass: false,
};

/// A row with every optional field empty: a state row of no kind, no inverse.
const fn base(name: &'static str, writes: BlockTouch) -> Row {
    Row {
        name,
        kind: None,
        writes,
        inverse: Inverse::None,
        extension: Extension::None,
        angle_like: false,
        scale: Scale::Free,
        class: Class::State,
        holds: "",
        warp_written: true,
        fixed_by: None,
        forward: None,
        invert: None,
        definition: None,
        slope: None,
        kinks: &[],
    }
}

/// A latent coordinate's row: kind 1, closed, assigned; the other seven controls held.
const fn latent(name: &'static str, block: Block, f: Forward, i: Invert) -> Row {
    let writes = match block {
        Block::Config => CONFIG,
        Block::Momentum => MOMENTUM,
        Block::Mass => MASS,
    };
    let mass = matches!(block, Block::Mass);
    Row {
        kind: Some(AxisKind::RawLatent),
        inverse: Inverse::Closed { on_pair: None },
        extension: Extension::Affine,
        // The mass controls are a parameter: no motion moves them (§ 3.3).
        class: if mass {
            Class::Conserved(Exactness::Exact)
        } else {
            Class::State
        },
        holds: "the other seven controls",
        forward: Some(f),
        invert: Some(i),
        ..base(name, writes)
    }
}

/// A kind-2 row of `block` with its own closed inverse.
const fn derived(name: &'static str, writes: BlockTouch, f: Forward, i: Invert) -> Row {
    Row {
        kind: Some(AxisKind::DerivedInBlock),
        inverse: Inverse::Closed { on_pair: None },
        extension: Extension::Bounded,
        forward: Some(f),
        invert: Some(i),
        ..base(name, writes)
    }
}

/// A kind-3 row solved into momentum, closed: its own inverse, gauge.
const fn invariant(name: &'static str, f: Forward, i: Invert, holds: &'static str) -> Row {
    Row {
        kind: Some(AxisKind::CrossBlockInvariant),
        inverse: Inverse::Closed { on_pair: None },
        extension: Extension::Bounded,
        scale: Scale::Gauge,
        holds,
        forward: Some(f),
        invert: Some(i),
        ..base(name, MOMENTUM)
    }
}

/// A row closed only on `chart`'s pair, the chart's encode its inverse and forward.
const fn on_pair(
    name: &'static str,
    kind: AxisKind,
    writes: BlockTouch,
    chart: &'static str,
    holds: &'static str,
) -> Row {
    Row {
        kind: Some(kind),
        inverse: Inverse::Closed {
            on_pair: Some(chart),
        },
        extension: Extension::Bounded,
        holds,
        ..base(name, writes)
    }
}

/// What a scale solve holds.
const DIRECTION: &str = "the momenta's direction";
/// What a mass ratio holds.
const THIRD_MASS: &str = "the third mass, and the configuration and momentum controls";

/// `row` with its slope declared exact, by `f`, and the kink sites it flags.
const fn exact(row: Row, f: DualForward, kinks: &'static [Kink]) -> Row {
    Row {
        slope: Some(Slope::Exact(f)),
        kinks,
        ..row
    }
}

/// `row` with its slope declared tier 3: none, each sample evaluated on the CPU.
const fn no_slope(row: Row) -> Row {
    Row {
        slope: Some(Slope::None),
        ..row
    }
}

/// The registry's rows (the module's note).
pub static ROWS: [Row; 39] = [
    exact(
        latent("z_α", Block::Config, z_alpha, set_z_alpha),
        dz_0,
        &[Kink::CanonBranch],
    ),
    exact(
        latent("z_β", Block::Config, z_beta, set_z_beta),
        dz_1,
        &[Kink::CanonBranch],
    ),
    exact(
        latent("z_q0", Block::Momentum, z_q0, set_z_q0),
        dz_2,
        &[Kink::CanonBranch],
    ),
    exact(
        latent("z_q1", Block::Momentum, z_q1, set_z_q1),
        dz_3,
        &[Kink::CanonBranch],
    ),
    exact(
        latent("z_q2", Block::Momentum, z_q2, set_z_q2),
        dz_4,
        &[Kink::CanonBranch],
    ),
    exact(
        latent("z_q3", Block::Momentum, z_q3, set_z_q3),
        dz_5,
        &[Kink::CanonBranch],
    ),
    exact(latent("z_μ1", Block::Mass, z_mu1, set_z_mu1), dz_6, &[]),
    exact(latent("z_μ2", Block::Mass, z_mu2, set_z_mu2), dz_7, &[]),
    exact(
        Row {
            angle_like: true,
            holds: "β, the masses and the momentum controls",
            ..derived("α", CONFIG, alpha, set_alpha)
        },
        dalpha,
        &[Kink::CanonBranch],
    ),
    exact(
        Row {
            angle_like: true,
            extension: Extension::Periodic,
            holds: "α, the masses and the momentum controls",
            ..derived("β", CONFIG, beta, set_beta)
        },
        dbeta,
        &[Kink::CanonBranch],
    ),
    exact(
        Row {
            holds: "β, the masses and the momentum controls",
            ..derived("‖λ‖/‖ρ‖", CONFIG, jacobi_ratio, set_jacobi_ratio)
        },
        djacobi_ratio,
        &[Kink::Pole],
    ),
    exact(
        Row {
            angle_like: true,
            extension: Extension::Periodic,
            forward: Some(shape_theta),
            ..on_pair(
                "θ",
                AxisKind::DerivedInBlock,
                CONFIG,
                "ShapeSphere",
                "the masses and the momenta",
            )
        },
        dshape_theta,
        &[Kink::Pole],
    ),
    exact(
        Row {
            angle_like: true,
            forward: Some(shape_phi),
            ..on_pair(
                "φ",
                AxisKind::DerivedInBlock,
                CONFIG,
                "ShapeSphere",
                "the masses and the momenta",
            )
        },
        dshape_phi,
        &[Kink::Pole],
    ),
    exact(
        Row {
            class: Class::Conserved(Exactness::Exact),
            holds: THIRD_MASS,
            ..derived("m₁/m₀", MASS, m1_over_m0, set_m1_over_m0)
        },
        dm1_over_m0,
        &[],
    ),
    exact(
        Row {
            class: Class::Conserved(Exactness::Exact),
            holds: THIRD_MASS,
            ..derived("m₀/m₁", MASS, m0_over_m1, set_m0_over_m1)
        },
        dm0_over_m1,
        &[],
    ),
    exact(
        Row {
            class: Class::Conserved(Exactness::Exact),
            holds: THIRD_MASS,
            ..derived("m₂/m₀", MASS, m2_over_m0, set_m2_over_m0)
        },
        dm2_over_m0,
        &[],
    ),
    exact(
        Row {
            class: Class::Conserved(Exactness::Exact),
            fixed_by: Some(Hold::E),
            ..invariant("E", energy, set_energy, DIRECTION)
        },
        denergy,
        &[],
    ),
    exact(
        Row {
            fixed_by: Some(Hold::E),
            ..invariant("K", kinetic, set_kinetic, DIRECTION)
        },
        dkinetic,
        &[],
    ),
    exact(
        Row {
            fixed_by: Some(Hold::E),
            ..invariant("KE", kinetic, set_kinetic, DIRECTION)
        },
        dkinetic,
        &[],
    ),
    exact(
        Row {
            class: Class::Conserved(Exactness::Exact),
            fixed_by: Some(Hold::Lz),
            ..invariant(
                "L_z",
                angular_momentum,
                set_angular_momentum,
                "the momenta's part orthogonal to Jr in ⟨·,·⟩_m",
            )
        },
        dangular_momentum,
        &[],
    ),
    exact(
        Row {
            scale: Scale::Free,
            fixed_by: Some(Hold::E),
            definition: Some(virial_definition),
            ..invariant("Q", virial, set_virial, DIRECTION)
        },
        dvirial,
        &[],
    ),
    no_slope(on_pair(
        "ν",
        AxisKind::CoupledCurve,
        CONFIG_MASS,
        "Burrau",
        "the momenta: the construction's",
    )),
    no_slope(Row {
        angle_like: true,
        ..on_pair(
            "θ_B",
            AxisKind::CoupledCurve,
            CONFIG_MASS,
            "BurrauThetaK",
            "the momenta: the construction's",
        )
    }),
    no_slope(on_pair(
        "δm",
        AxisKind::DerivedInBlock,
        MASS,
        "BurrauThetaDeltaM",
        "the curve's point",
    )),
    no_slope(on_pair(
        "m",
        AxisKind::DerivedInBlock,
        MASS,
        "EuclidPlane",
        "the Burrau triangle at ν",
    )),
    no_slope(on_pair(
        "m₁ = x",
        AxisKind::DerivedInBlock,
        MASS,
        "MassSimplex",
        "the Burrau triangle at ν₀, at rest",
    )),
    no_slope(on_pair(
        "y = (1 − x)·v",
        AxisKind::DerivedInBlock,
        MASS,
        "MassSimplex",
        "the Burrau triangle at ν₀, at rest",
    )),
    exact(
        Row {
            kind: Some(AxisKind::CrossBlockInvariant),
            scale: Scale::Gauge,
            warp_written: false,
            forward: Some(potential),
            ..base("PE", MOMENTUM)
        },
        dpotential,
        &[],
    ),
    exact(
        Row {
            forward: Some(separation_ratio),
            ..base("min(r_ij)/max(r_ij)", CONFIG)
        },
        dseparation_ratio,
        &[Kink::OrderSwitch],
    ),
    exact(
        Row {
            scale: Scale::Gauge,
            forward: Some(binding01),
            ..base("pair binding energy (0, 1)", NONE)
        },
        dbinding01,
        &[],
    ),
    exact(
        Row {
            class: Class::Conserved(Exactness::Approximate),
            forward: Some(eccentricity01),
            ..base("inner eccentricity e (0, 1)", NONE)
        },
        deccentricity01,
        &[Kink::ClampEdge],
    ),
    exact(
        Row {
            scale: Scale::SizeOnly,
            forward: Some(inertia),
            ..base("I", NONE)
        },
        dinertia,
        &[],
    ),
    exact(
        Row {
            scale: Scale::SizeOnly,
            forward: Some(size),
            ..base("R", NONE)
        },
        dsize,
        &[],
    ),
    exact(
        Row {
            scale: Scale::SizeOnly,
            forward: Some(size),
            ..base("ℓ", NONE)
        },
        dsize,
        &[],
    ),
    // The plan is unclear here: § 3.3 says a functional has "no row in the registry, and none may be added", and § 12's
    // `primitive_class` asks that one be refused as an axis by name; chose to register two, classed functional, with
    // no kind, forward or inverse, so that picking one is refused `Functional` by name and nothing else reads them.
    no_slope(Row {
        class: Class::Functional,
        ..base("minimum separation over the run", NONE)
    }),
    no_slope(Row {
        class: Class::Functional,
        ..base("escape time t_end", NONE)
    }),
    no_slope(Row {
        class: Class::Unclassified,
        ..base("isosceles family with matched velocities", NONE)
    }),
    no_slope(Row {
        class: Class::Unclassified,
        ..base("BHH (a, c; L)", NONE)
    }),
    no_slope(Row {
        class: Class::Unclassified,
        ..base("Mikkola–Hietarinta (R, θ)", NONE)
    }),
];

/// The registry: [`ROWS`], and any rows a caller adds (a test's own).
#[derive(Clone, Debug)]
pub struct Registry {
    rows: Vec<Row>,
}

impl Registry {
    /// [`ROWS`].
    pub fn standard() -> Self {
        Registry {
            rows: ROWS.to_vec(),
        }
    }

    /// [`ROWS`] and `extra`, each checked by [`check_row`]: an inconsistent row is not registered.
    pub fn with(extra: impl IntoIterator<Item = Row>) -> Result<Self, RowError> {
        Registry::from_rows(ROWS.iter().copied().chain(extra))
    }

    /// Exactly `rows`, each checked by [`check_row`].
    pub fn from_rows(rows: impl IntoIterator<Item = Row>) -> Result<Self, RowError> {
        let rows: Vec<Row> = rows.into_iter().collect();
        rows.iter().try_for_each(check_row)?;
        Ok(Registry { rows })
    }

    /// Every row.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The row `name`.
    pub fn row(&self, name: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.name == name)
    }
}

/// An axis's warp (§ 4): four, closed; R-506 stands over all of them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Warp {
    /// The axis's own feasibility warp, the default for a physical quantity.
    #[default]
    Feasibility,
    /// `lo + (hi − lo)·t`.
    Linear,
    /// `lo + (hi − lo)·t^γ`.
    Power(f64),
    /// `lo·(hi/lo)^t`.
    Log,
}

/// What an axis is built from.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    /// A latent direction `q`, assigned (kind 1).
    Direction(Latent),
    /// A registered row, picked by name.
    Pick(String),
    /// A combination typed as an expression.
    Typed(Expr),
}

/// One axis (§ 2): its primitive, warp, extension and what it holds fixed.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisSpec {
    /// What it is built from.
    pub primitive: Primitive,
    /// Its warp.
    pub warp: Warp,
    /// Its extension; `None` for the row's natural one.
    pub extension: Option<Extension>,
    /// What it holds, where it says (§ 3, "Mass-weighted shape"); `None` for the row's own.
    pub holds: Option<String>,
}

impl AxisSpec {
    /// An axis on `primitive` at every default.
    pub fn of(primitive: Primitive) -> Self {
        AxisSpec {
            primitive,
            warp: Warp::default(),
            extension: None,
            holds: None,
        }
    }

    /// An axis on the row `name`.
    pub fn pick(name: &str) -> Self {
        AxisSpec::of(Primitive::Pick(name.to_owned()))
    }
}

/// A chart-level pair map (§ 5.1), before the axes: the kernel's `chart::paired`, its centre and ranges the view's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PairMap {
    /// Each axis written alone.
    #[default]
    Independent,
    /// Polar about a centre, the radius pole-crossing or bounded at the pole.
    Polar { pole_crossing: bool },
    /// Log-polar about a centre, no pole in view.
    LogPolar,
}

/// The chart's constraints (§ 5.3): hold `E`, hold `L_z`, each off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Constraints {
    /// Hold `E` at the value.
    pub hold_e: Option<f64>,
    /// Hold `L_z` at the value.
    pub hold_lz: Option<f64>,
}

impl Constraints {
    /// Whether `hold` is on.
    pub fn holds(&self, hold: Hold) -> bool {
        match hold {
            Hold::E => self.hold_e.is_some(),
            Hold::Lz => self.hold_lz.is_some(),
        }
    }
}

/// A chart (§ 2): two axes, the pair map, the constraints and the slice, `z₀`.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSpec {
    /// The axes, horizontal then vertical.
    pub axes: [AxisSpec; 2],
    /// The pair map.
    pub pair_map: PairMap,
    /// The constraints.
    pub constraints: Constraints,
    /// The slice centre.
    pub slice: Latent,
}

impl ChartSpec {
    /// The chart of `h` and `v`, independent, unconstrained, at the slice `z₀`.
    pub fn new(h: AxisSpec, v: AxisSpec, slice: Latent) -> Self {
        ChartSpec {
            axes: [h, v],
            pair_map: PairMap::default(),
            constraints: Constraints::default(),
            slice,
        }
    }
}

/// The registered pairs: today's charts, each a pair of rows closed together, by `kernel::chart::REGISTRY` name, and
/// the compute entry the kernel bakes it as (`kernel::stage::decode`'s variant at the default links; § 7.1). The
/// latent chart's pairs are uniform, not baked.
pub static PAIRS: [(&str, &str, &str, &str); 11] = [
    ("θ", "φ", "ShapeSphere", ShapeSphereDefaults::ENTRY),
    ("ν", "K", "Burrau", BurrauDefaults::ENTRY),
    ("m", "ν", "EuclidPlane", EuclidPlaneDefaults::ENTRY),
    ("θ_B", "K", "BurrauThetaK", BurrauThetaKDefaults::ENTRY),
    ("θ_B", "L_z", "BurrauThetaLz", BurrauThetaLzDefaults::ENTRY),
    (
        "θ_B",
        "δm",
        "BurrauThetaDeltaM",
        BurrauThetaDeltaMDefaults::ENTRY,
    ),
    ("L_z", "E", "InvariantLE", InvariantLEDefaults::ENTRY),
    ("L_z", "K", "InvariantLK", InvariantLKDefaults::ENTRY),
    (
        "m₁ = x",
        "y = (1 − x)·v",
        "MassSimplex",
        MassSimplexDefaults::ENTRY,
    ),
    ("z_μ1", "α", "Mixed", MixedMu1AlphaDefaults::ENTRY),
    ("α", "Q", "Mixed", MixedAlphaVirialDefaults::ENTRY),
];

/// The registered pair `(h, v)`, in that order: its chart and entry.
pub fn pair(h: &str, v: &str) -> Option<(&'static str, &'static str)> {
    PAIRS
        .iter()
        .find(|p| p.0 == h && p.1 == v)
        .map(|p| (p.2, p.3))
}
