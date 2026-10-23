//! Lookup, CPU-side (inverse_encode_contract Part 6, "Lookup" and § "Chart-aware validation"; R-13; TASK-M2-19): an
//! entered initial condition encoded into the active chart, validated by the three layers in order, and locked.
//!
//! **The forms** (Part 6, "Lookup"):
//! - **Exact** ([`Exact`], [`lookup`]): a Pythagorean pair `(m, n)` or a triple, each the Burrau system of its
//!   triangle at rest (`kernel::burrau`); a raw `z`, decoded; or physical `(mᵢ, rᵢ, pᵢ)`. The state is encoded into
//!   the active chart by `kernel::encode::by_kind::encode_into_within`, Part 6 steps 1a to 4, with the caller's
//!   `‖·‖_phys`, `ε_phys` and curve tolerance ([`Tolerances`]), and locked: the lock's anchor is its latent `z`, and its
//!   chart point where it lands on the chart. A raw `z` keeps each control the clamp did not move bit for bit.
//! - **Partial** ([`Partial`], [`lookup_partial`]): any of the three blocks, the rest filled with Part 6's defaults
//!   (masses equal, `⅓` each, or the Burrau system's on a Burrau chart ([`Fill`]); the configuration the Burrau
//!   triangle at the locked `ν`, or `ν = ½`, on a Burrau chart, else its latent controls 0; the momenta at rest), then
//!   the same encode, ladder and lock, and a slice through the lock spanning the unspecified controls ([`Slice`];
//!   Part 6, "The partial specification's blocks and slice basis", REQ-ENC-031).
//!
//! **The masses** (Part 6, "The physical form's masses"): the section's unit of mass is `M = Σmᵢ = 1`, so an entry of
//! mass `M` is carried to it by the change of mass unit with `G = 1` and the length unit kept, `mᵢ ← mᵢ/M`,
//! `pᵢ ← pᵢ/M^{3/2}` (the velocities by `1/√M`), which keeps every ratio of the motion; `M` is recorded
//! ([`Notices::mass`]). The pair and triple forms' masses are the sides, so their `M` is the perimeter; a raw `z` has
//! `M = 1`; the partial form's default masses are `⅓` each, or the Burrau system's.
//!
//! **The ladder** ([`Layer`]; § "Chart-aware validation", in order):
//! 1. **Hypercube bounds** in the controls' clamp space (§ "Layer 1's space and bounds", REQ-ENC-023): a control fails
//!    iff the block inverse clamped it, its `lookup_clamped` bit; so does the coincident inner pair (R-13). The remedy
//!    is the clamp, the moved controls shown ([`Moved`]), unless it changes the class of `L_z` or `E`
//!    ([`Class`]; § "A qualitatively different IC"), which is rejected. The zero class is a band relative to each
//!    value's scale ([`Classes`]; § "The zero class in floating point", REQ-ENC-036), so a physically zero `L_z` or `E`
//!    has one class whatever its rounding. A state whose entered `L_z` or `E` is not finite (the coincident pair's
//!    `U = −∞`) has no class to change there, so its clamp stands: there is no coincident-bodies rejection (R-13).
//! 2. **Chart feasibility**, on a chart landing: the chart's own `validate` at its point (R-26), whose project and clamp
//!    move the lock to the point it gives and whose reject is the lookup's. A latent landing has no layer-2 constraint.
//! 3. **Decode sanity** of the lock as a pixel decodes it, the chart's decode at a chart landing, else the free decode
//!    of `z` ([`sanity`]): every mass positive, and the CoM and `Σpᵢ` within the tolerance, REQ-ENC-027's calibration
//!    (R-71), proposed by the `decode-sanity` gate. Neither a projection nor a clamp is defined for it, so it rejects.
//!
//! A rejection names its layer and its constraint ([`Rejected`]). An entry that is not finite, or masses that are not
//! positive, or `(m, n)` or a triple that is no Pythagorean one, is refused before the ladder ([`Layer::Entry`]).
//!
//! **CPU-side only**: every branch here runs at f64 on the CPU and has no GPU counterpart, so R-547's discipline for a
//! branch input that must agree across backends does not bind them. The kernel's encode it calls keeps its own.
//!
//! **`lookup_rescaled`** carries `λ` ([`Notices::lambda`]); whether it is raised, `τ_I`'s skip, is TASK-M2-47's
//! (REQ-ENC-014, R-547), so no raised flag is formed here, as `C` forms none.

use std::fmt;

use kernel::burrau::{euclid, Triple};
use kernel::chart::burrau::embed;
use kernel::chart::mixed::{Coordinate, Quantity};
use kernel::chart::{
    BlockTouch, Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Constraint, EuclidPlane,
    InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeSphere, ValidationResult,
};
use kernel::decode::{self, config, mass, Ic};
use kernel::encode::by_kind::{encode_into_within, invariants, ByKind, Landing};
use kernel::encode::{encode, SimplexClamp};
use kernel::generated::links::Selection;
use ledger::constants::{DECODE_SANITY, EPS_PHYS, ZERO_CLASS};

/// An exact entry (Part 6, "Exact IC (full lock)").
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Exact {
    /// Euclid's pair `(m, n)`, primitive (`kernel::burrau::euclid`).
    Pythagorean { m: u32, n: u32 },
    /// A Pythagorean triple `(a, b, c)`, `a² + b² = c²`, primitive or not (the same system under the similarity).
    Triple([u64; 3]),
    /// A raw `z`, in the latent block layout `z_α, z_β | z_q0 … z_q3 | z_μ1, z_μ2`.
    Raw([f64; 8]),
    /// Physical `(mᵢ, rᵢ, pᵢ)`.
    Physical(Ic<f64>),
}

/// A partial entry (Part 6, "Partial specification"): each block given or not.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Partial {
    /// The masses.
    pub masses: Option<[f64; 3]>,
    /// The positions, the configuration block.
    pub positions: Option<[[f64; 2]; 3]>,
    /// The momenta.
    pub momenta: Option<[[f64; 2]; 3]>,
}

/// A chart's partial-specification defaults: whether it implies Burrau (Part 6), so the masses and the configuration
/// default to the Burrau system's.
pub trait Fill: ByKind {
    /// The chart is one of the Burrau family.
    const BURRAU: bool = false;
}

impl<P: Copy> Fill for Latent<P> {}
impl Fill for ShapeSphere<f64> {}
impl Fill for ShapeSphere<f32> {}
impl<P: Copy> Fill for InvariantLE<P> {}
impl<P: Copy> Fill for InvariantLK<P> {}
impl<P: Copy> Fill for MassSimplex<P> where f64: From<P> {}
impl<H: Quantity, V: Quantity, P: Copy> Fill for Mixed<H, V, P> where f64: From<P> {}

macro_rules! burrau_fill {
    ($($chart:ident),*) => {$(
        impl<P: Copy> Fill for $chart<P> where f64: From<P> {
            const BURRAU: bool = true;
        }
    )*};
}

burrau_fill!(
    Burrau,
    EuclidPlane,
    BurrauThetaK,
    BurrauThetaLz,
    BurrauThetaDeltaM
);

/// The tolerances lookup passes and applies, at f64.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerances {
    /// `ε_phys`, Part 6 step 4's landing test (REQ-ENC-024).
    pub eps_phys: f64,
    /// Kind 4's curve tolerance (REQ-ENC-026).
    pub curve_tol: f64,
    /// Layer 3's tolerance on the CoM and `Σpᵢ` residuals ([`residuals`]; REQ-ENC-027).
    pub sanity: f64,
    /// Layer 1's zero-class band, relative to each value's scale ([`Classes`]; REQ-ENC-036, REQ-ENC-037).
    pub zero_class: f64,
}

impl Tolerances {
    /// The register's `eps_phys`, `decode_sanity` and `zero_class`, each times f64's epsilon, and the caller's
    /// `curve_tol`.
    pub fn register(curve_tol: f64) -> Tolerances {
        Tolerances {
            eps_phys: EPS_PHYS.number() * f64::EPSILON,
            curve_tol,
            sanity: DECODE_SANITY.number() * f64::EPSILON,
            zero_class: ZERO_CLASS.number() * f64::EPSILON,
        }
    }
}

/// The slice a partial lookup sets: centred on the lock, `q₁` and `q₂` unit directions of two unspecified controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slice {
    /// `z₀`, the lock's `z`.
    pub z0: [f64; 8],
    /// `q₁`.
    pub q1: [f64; 8],
    /// `q₂`.
    pub q2: [f64; 8],
}

/// What a clamp moved (Part 6, "show which coordinates moved").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Moved {
    /// Bit `k` set iff layer 1 clamped `z[k]`.
    pub controls: u32,
    /// The inner pair coincided, `ρ̃ = 0` (R-13).
    pub coincident: bool,
    /// Which chart coordinates, `u` and `v`, layer 2's clamp moved.
    pub chart: [bool; 2],
}

/// The notices a lock carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Notices {
    /// What the clamps moved; `lookup_clamped` iff any moved.
    pub moved: Moved,
    /// Layer 2's projection, naming the constraint the entry failed.
    pub projected: Option<Constraint>,
    /// Layer 2's clamp, naming the constraint the entry failed.
    pub clamped: Option<Constraint>,
    /// `lookup_mirrored`.
    pub mirrored: bool,
    /// `λ`, `lookup_rescaled`'s value.
    pub lambda: f64,
    /// `M`, the entered masses' sum, the mass unit the entry was carried from to the section's `M = 1` (Part 6, "The
    /// physical form's masses"): with `λ`, the multiply-back's factor (REQ-ENC-014).
    pub mass: f64,
    /// A curve chart's fold flag (`kernel::encode::curve`; REQ-ENC-022).
    pub past_fold: bool,
}

impl Notices {
    /// `lookup_clamped`: a control, the coincident pair or a chart coordinate moved.
    pub fn lookup_clamped(&self) -> bool {
        let m = &self.moved;
        m.controls != 0 || m.coincident || m.chart[0] || m.chart[1]
    }
}

/// A lock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Locked {
    /// The anchor's latent `z`.
    pub z: [f64; 8],
    /// Its chart point, or latent `z`.
    pub landing: Landing<f64>,
    /// The partial form's slice; `None` for an exact entry, which leaves the slice alone.
    pub slice: Option<Slice>,
    /// The notices.
    pub notices: Notices,
}

/// Where a rejection happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// The entry itself, before the ladder.
    Entry,
    /// Layer 1, the hypercube bounds.
    Hypercube,
    /// Layer 2, chart feasibility.
    Feasibility,
    /// Layer 3, decode sanity.
    Sanity,
}

/// A constant of motion whose class a clamp changed (§ "A qualitatively different IC").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// `sign(L_z)`, the sense of rotation.
    Rotation,
    /// `sign(E)`, the energy class.
    Energy,
}

/// The constraint a rejection names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Failed {
    /// `(m, n)` is not a primitive pair.
    NotPrimitive,
    /// The three sides are no Pythagorean triple.
    NotATriple,
    /// A value entered is not finite.
    NonFinite,
    /// A mass entered is not positive.
    MassNotPositive,
    /// Layer 1's clamp of the `controls` would change `class`.
    Clamp { controls: u32, class: Class },
    /// A chart constraint (layer 2) or a decode-sanity one (layer 3).
    Constraint(Constraint),
}

/// A refused lookup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rejected {
    /// The layer.
    pub layer: Layer,
    /// The constraint.
    pub failed: Failed,
}

impl fmt::Display for Rejected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lookup refused at {:?}: {:?}", self.layer, self.failed)
    }
}

impl std::error::Error for Rejected {}

fn refuse(layer: Layer, failed: Failed) -> Rejected {
    Rejected { layer, failed }
}

/// A state's two classifying constants and their scales (§ "The zero class in floating point", REQ-ENC-036).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Classes {
    /// `L_z = Σ rᵢ × pᵢ`.
    pub lz: f64,
    /// `L_z`'s scale, `Σ‖rᵢ‖‖pᵢ‖`.
    pub lz_scale: f64,
    /// `E = K + U`.
    pub e: f64,
    /// `E`'s scale, `K + |U|`.
    pub e_scale: f64,
}

impl Classes {
    /// `x`'s constants and scales.
    pub fn of(x: &Ic<f64>) -> Classes {
        let inv = invariants(x);
        let norm = |v: [f64; 2]| v[0].hypot(v[1]);
        Classes {
            lz: inv.lz,
            lz_scale: (0..3).map(|i| norm(x.r[i]) * norm(x.p[i])).sum(),
            e: inv.e,
            e_scale: inv.k + inv.u.abs(),
        }
    }
}

/// The class of `x` at `scale`: `Equal` (class 0) where `|x| ≤ band·scale`, else `sign(x)`; `None` where `x` or
/// `scale` is not finite.
fn class(x: f64, scale: f64, band: f64) -> Option<std::cmp::Ordering> {
    if !(x.is_finite() && scale.is_finite()) {
        None
    } else if x.abs() <= band * scale {
        Some(std::cmp::Ordering::Equal)
    } else {
        x.partial_cmp(&0.0)
    }
}

/// The class a clamp from `before` to `after` changes at the zero-class `band`, `L_z`'s first; none where `before`'s
/// `L_z` or `E` is not finite (§ "Layer 1's space and bounds": such a state, the coincident pair's `U = −∞` among
/// them, has no class there to change, so its clamp stands).
pub fn changed(before: &Classes, after: &Classes, band: f64) -> Option<Class> {
    let moved =
        |b: Option<std::cmp::Ordering>, a: Option<std::cmp::Ordering>| b.is_some() && a != b;
    if !(before.lz.is_finite() && before.e.is_finite()) {
        None
    } else if moved(
        class(before.lz, before.lz_scale, band),
        class(after.lz, after.lz_scale, band),
    ) {
        Some(Class::Rotation)
    } else if moved(
        class(before.e, before.e_scale, band),
        class(after.e, after.e_scale, band),
    ) {
        Some(Class::Energy)
    } else {
        None
    }
}

/// Layer 3's residuals of `x` (§ "Decode sanity"): `‖Σmᵢrᵢ‖/R_x` and `‖Σpᵢ‖/P_x`, each sum in body order, `R_x` and
/// `P_x` Part 4's extents, the largest `‖rⱼ‖` and `‖pⱼ‖` floored at 1.
pub fn residuals(x: &Ic<f64>) -> (f64, f64) {
    let extent = |v: &[[f64; 2]; 3]| v.iter().map(|w| w[0].hypot(w[1])).fold(1.0, f64::max);
    let sum = |w: [f64; 3], v: &[[f64; 2]; 3]| {
        let axis = |k: usize| w[0] * v[0][k] + w[1] * v[1][k] + w[2] * v[2][k];
        axis(0).hypot(axis(1))
    };
    (
        sum(x.m, &x.r) / extent(&x.r),
        sum([1.0; 3], &x.p) / extent(&x.p),
    )
}

/// Layer 3 on the decoded `x` at the tolerance `tol`: the constraint it fails, the masses first; a NaN fails.
pub fn sanity(x: &Ic<f64>, tol: f64) -> Option<Constraint> {
    let (com, momentum) = residuals(x);
    if !x.m.iter().all(|&m| m > 0.0) {
        Some(Constraint::DecodedMassPositive)
    } else if com.is_nan() || com > tol {
        Some(Constraint::ComAtOrigin)
    } else if momentum.is_nan() || momentum > tol {
        Some(Constraint::MomentumZero)
    } else {
        None
    }
}

/// The order the slice basis takes unspecified controls in (Part 6, REQ-ENC-031): `z_α`, `z_β`, then `z_μ1`, `z_μ2`,
/// then `z_q0 … z_q3`.
pub const ORDER: [Coordinate; 8] = [
    Coordinate::ZAlpha,
    Coordinate::ZBeta,
    Coordinate::ZMu1,
    Coordinate::ZMu2,
    Coordinate::ZQ0,
    Coordinate::ZQ1,
    Coordinate::ZQ2,
    Coordinate::ZQ3,
];

/// The slice basis through `z0` over the unspecified blocks `free`: `ê_i`, `ê_j`, `i` and `j` the first two of their
/// controls in [`ORDER`]; `None` where no block is unspecified.
pub fn slice(z0: [f64; 8], free: BlockTouch) -> Option<Slice> {
    let open = |c: &&Coordinate| {
        let t = c.touches();
        (t.config && free.config) || (t.momentum && free.momentum) || (t.mass && free.mass)
    };
    let mut picked = ORDER.iter().filter(open);
    let (&i, &j) = (picked.next()?, picked.next()?);
    let unit = |c: Coordinate| std::array::from_fn(|n| if n == c as usize { 1.0 } else { 0.0 });
    Some(Slice {
        z0,
        q1: unit(i),
        q2: unit(j),
    })
}

/// `x` carried to the section's mass unit and `M`, its masses' sum (Part 6, "The physical form's masses"): with `G = 1`
/// and the length unit kept, `mᵢ ← mᵢ/M`, `vᵢ ← vᵢ/√M`, so `pᵢ ← pᵢ/M^{3/2}`; refused where a value is not finite or a
/// mass is not positive.
fn entered(x: &Ic<f64>) -> Result<(Ic<f64>, f64), Rejected> {
    let finite =
        x.m.iter()
            .chain(x.r.iter().flatten())
            .chain(x.p.iter().flatten());
    if !finite.into_iter().all(|v| v.is_finite()) {
        return Err(refuse(Layer::Entry, Failed::NonFinite));
    }
    if !x.m.iter().all(|&m| m > 0.0) {
        return Err(refuse(Layer::Entry, Failed::MassNotPositive));
    }
    let total = x.m[0] + x.m[1] + x.m[2];
    let carry = total * total.sqrt();
    let carried = Ic {
        m: x.m.map(|v| v / total),
        r: x.r,
        p: x.p.map(|v| v.map(|c| c / carry)),
    };
    Ok((carried, total))
}

/// The masses of a Burrau triangle `(a, b, c)` as entered, its sides (`principia_chart_reference.md` §4.1): their sum.
fn perimeter(t: &Triple) -> f64 {
    [t.a, t.b, t.c].iter().map(|&s| s as f64).sum()
}

/// The Burrau system `s` at rest.
fn at_rest(s: kernel::burrau::System<f64>) -> Ic<f64> {
    Ic {
        m: s.m,
        r: s.r,
        p: [[0.0; 2]; 3],
    }
}

/// An exact entry, encoded into `chart` under `S`, validated and locked (the module's note).
pub fn lookup<S, C, N>(
    chart: &C,
    entry: &Exact,
    norm: N,
    tol: &Tolerances,
) -> Result<Locked, Rejected>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: ByKind,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    let (x, raw, mass) = match *entry {
        Exact::Pythagorean { m, n } => match euclid(m, n) {
            Some(t) => (at_rest(t.burrau()), None, perimeter(&t)),
            None => return Err(refuse(Layer::Entry, Failed::NotPrimitive)),
        },
        Exact::Triple([a, b, c]) => {
            let sq = |s: u64| u128::from(s) * u128::from(s);
            if a == 0 || b == 0 || sq(a).checked_add(sq(b)) != Some(sq(c)) {
                return Err(refuse(Layer::Entry, Failed::NotATriple));
            }
            let t = Triple { a, b, c };
            (at_rest(t.burrau()), None, perimeter(&t))
        }
        Exact::Raw(z) => {
            if !z.iter().all(|v| v.is_finite()) {
                return Err(refuse(Layer::Entry, Failed::NonFinite));
            }
            (
                decode::decode::<S, f64>(&decode::Latent::new(z)).ic,
                Some(z),
                1.0,
            )
        }
        Exact::Physical(x) => {
            let (x, mass) = entered(&x)?;
            (x, None, mass)
        }
    };
    ladder::<S, C, N>(chart, &x, raw, mass, norm, tol)
}

/// A partial entry on `chart` under `S` (the module's note): the blocks not given filled, Burrau's at `locked_nu` or
/// `ν = ½` on a Burrau chart, then [`lookup`]'s ladder, and the slice over the unspecified controls.
pub fn lookup_partial<S, C, N>(
    chart: &C,
    partial: &Partial,
    locked_nu: Option<f64>,
    norm: N,
    tol: &Tolerances,
) -> Result<Locked, Rejected>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: Fill,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    let nu = locked_nu.unwrap_or(0.5);
    if C::BURRAU && !(nu > 0.0 && nu < 1.0) {
        return Err(refuse(
            Layer::Feasibility,
            Failed::Constraint(Constraint::BurrauNu),
        ));
    }
    let burrau = embed(nu);
    let m = match partial.masses {
        Some(m) => m,
        None if C::BURRAU => burrau.m,
        None => [1.0 / 3.0; 3],
    };
    let p = partial.momenta.unwrap_or([[0.0; 2]; 3]);
    let r = match partial.positions {
        Some(r) => r,
        None if C::BURRAU => burrau.r,
        None => [[0.0; 2]; 3],
    };
    // The carry keeps the positions, so the latent default configuration is decoded once the masses are carried.
    let (mut x, total) = entered(&Ic { m, r, p })?;
    if partial.positions.is_none() && !C::BURRAU {
        x.r = config::decode::<S, f64>([0.0; 2], &mass::from_masses(x.m)).r;
    }
    let free = BlockTouch {
        config: partial.positions.is_none(),
        momentum: partial.momenta.is_none(),
        mass: partial.masses.is_none(),
    };
    let mut locked = ladder::<S, C, N>(chart, &x, None, total, norm, tol)?;
    locked.slice = slice(locked.z, free);
    Ok(locked)
}

/// The ladder on the entered state `x` (the module's note); `raw` the entered `z`, whose unclamped controls the lock
/// keeps; `mass` the entry's `M`.
fn ladder<S, C, N>(
    chart: &C,
    x: &Ic<f64>,
    raw: Option<[f64; 8]>,
    mass: f64,
    norm: N,
    tol: &Tolerances,
) -> Result<Locked, Rejected>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: ByKind,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    let ce = encode_into_within::<S, f64, C, _>(chart, x, &norm, tol.eps_phys, tol.curve_tol);
    let e = &ce.encoded;
    let mut moved = Moved {
        controls: e.report.clamped_controls,
        coincident: e.report.coincident,
        chart: [false; 2],
    };
    let mut z = e.z.z;
    if let Some(r) = raw {
        for (k, zk) in z.iter_mut().enumerate() {
            if moved.controls & (1 << k) == 0 {
                *zk = r[k];
            }
        }
    }
    // Layer 1: the clamp stands unless it changes a class.
    if moved.controls != 0 {
        let after = decode::decode::<S, f64>(&decode::Latent::new(z)).ic;
        let before = Classes::of(&e.canonical.ic);
        if let Some(class) = changed(&before, &Classes::of(&after), tol.zero_class) {
            let failed = Failed::Clamp {
                controls: moved.controls,
                class,
            };
            return Err(refuse(Layer::Hypercube, failed));
        }
    }
    // Layer 2: the chart's own check at a chart landing.
    let mut landing = ce.landing;
    let (mut projected, mut clamped) = (None, None);
    if let Landing::Chart { u, v } = landing {
        let to = match chart.validate(u, v) {
            ValidationResult::Valid => None,
            ValidationResult::Project { u, v, failed } => {
                projected = Some(failed);
                Some((u, v))
            }
            ValidationResult::Clamp {
                u,
                v,
                moved: m,
                failed,
            } => {
                moved.chart = m;
                clamped = Some(failed);
                Some((u, v))
            }
            ValidationResult::Reject { failed } => {
                return Err(refuse(Layer::Feasibility, Failed::Constraint(failed)))
            }
        };
        if let Some((u, v)) = to {
            landing = Landing::Chart { u, v };
            z = encode::<S, f64>(&chart.decode_at::<S, f64>(u, v).ic).z.z;
        }
    }
    // Layer 3: the lock decoded as a pixel decodes it.
    let decoded = match landing {
        Landing::Chart { u, v } => chart.decode_at::<S, f64>(u, v).ic,
        Landing::Latent => decode::decode::<S, f64>(&decode::Latent::new(z)).ic,
    };
    if let Some(failed) = sanity(&decoded, tol.sanity) {
        return Err(refuse(Layer::Sanity, Failed::Constraint(failed)));
    }
    Ok(Locked {
        z,
        landing,
        slice: None,
        notices: Notices {
            moved,
            projected,
            clamped,
            mirrored: e.report.mirrored,
            lambda: e.report.lambda,
            mass,
            past_fold: ce.curve.is_some_and(|c| c.closed.past_fold),
        },
    })
}
