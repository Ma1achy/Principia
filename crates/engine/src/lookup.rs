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
//!    ([`Class`]; § "A qualitatively different IC"), which is rejected. The zero class is a band that absorbs
//!    rounding only, relative to the bound on each value's rounding on the entered state's extents ([`Classes`],
//!    [`Extents`]; § "The zero class in floating point", REQ-ENC-036), so a physically zero `L_z` or `E` has one class
//!    whatever its rounding; the clamped state is classed by the rule, from each clamp's move in closed form, not by
//!    its decoded value ([`moves`], [`after`]; § "The class a clamp moves", REQ-ENC-038). A state whose entered `L_z` or `E` is not finite (the coincident pair's
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
use kernel::decode::{self, config, mass, momentum, Ic};
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
    /// Layer 1's zero-class band, relative to the bound on each value's rounding ([`Classes`], [`Moves`]; REQ-ENC-036,
    /// REQ-ENC-037).
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

/// The entered state's extents in the canonical units, the magnitudes `C` rounds from (§ "The zero class in floating
/// point", REQ-ENC-036): `R = λ·maxᵢ‖rᵢ‖` and `P = λ^{−1/2}·maxᵢ‖pᵢ‖` of the entry, `λ = I_in^{−1/2}`, before its
/// translation, boost, rotation and rescale.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extents {
    /// `R`.
    pub r: f64,
    /// `P`.
    pub p: f64,
}

impl Extents {
    /// The extents of the entry `x`, whose canonical form has `λ = lambda`.
    pub fn entered(x: &Ic<f64>, lambda: f64) -> Extents {
        let extent = |v: &[[f64; 2]; 3]| v.iter().map(|w| w[0].hypot(w[1])).fold(0.0, f64::max);
        Extents {
            r: lambda * extent(&x.r),
            p: extent(&x.p) / lambda.sqrt(),
        }
    }
}

/// A state's two classifying constants and their scales (§ "The zero class in floating point", REQ-ENC-036).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Classes {
    /// `L_z = Σ rᵢ × pᵢ`.
    pub lz: f64,
    /// `L_z`'s scale, `Σ((1 + R)‖rᵢ‖‖pᵢ‖ + R‖pᵢ‖ + ‖rᵢ‖P)`.
    pub lz_scale: f64,
    /// `E = K + U`.
    pub e: f64,
    /// `E`'s scale, `Σ((1 + R)‖pᵢ‖²/2 + ‖pᵢ‖P)/mᵢ + Σ mᵢmⱼ/rᵢⱼ·(1 + R + 2R/rᵢⱼ)`.
    pub e_scale: f64,
}

impl Classes {
    /// `x`'s constants and their scales at no entered extent: `Σ‖rᵢ‖‖pᵢ‖` and `K + |U|`.
    pub fn of(x: &Ic<f64>) -> Classes {
        Classes::within(x, &Extents::default())
    }

    /// `x`'s constants and their scales, `x` the canonical form of an entry of extents `ext`.
    pub fn within(x: &Ic<f64>, ext: &Extents) -> Classes {
        let inv = invariants(x);
        let norm = |v: [f64; 2]| v[0].hypot(v[1]);
        // To first order: each position rounded by `R`, each momentum by `P`, and the rescale, `I_in` read from
        // positions rounded by `R`, scaling every vector by `1 + R`.
        let rescale = 1.0 + ext.r;
        let lz_scale: f64 = (0..3)
            .map(|i| {
                let (r, p) = (norm(x.r[i]), norm(x.p[i]));
                rescale * r * p + ext.r * p + r * ext.p
            })
            .sum();
        let kinetic: f64 = (0..3)
            .map(|i| {
                let p = norm(x.p[i]);
                (rescale * p * p / 2.0 + p * ext.p) / x.m[i]
            })
            .sum();
        let potential: f64 = [(0, 1), (0, 2), (1, 2)]
            .iter()
            .map(|&(i, j)| {
                let d = norm([x.r[i][0] - x.r[j][0], x.r[i][1] - x.r[j][1]]);
                x.m[i] * x.m[j] / d * (rescale + 2.0 * ext.r / d)
            })
            .sum();
        Classes {
            lz: inv.lz,
            lz_scale,
            e: inv.e,
            e_scale: kinetic + potential,
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

/// The clamped state's classes by the rule (§ "The class a clamp moves", REQ-ENC-038), not by its decoded value: a
/// constant in class 0 at `band` is 0 but for rounding, so after the clamp it is the move alone at the move's scale;
/// any other is its value on the clamped parameters at that value's scale.
pub fn after(before: &Classes, moves: &Moves, band: f64) -> Classes {
    let zero = |v: f64, scale: f64| class(v, scale, band) == Some(std::cmp::Ordering::Equal);
    let (lz, lz_scale) = if zero(before.lz, before.lz_scale) {
        (moves.lz, moves.lz_scale)
    } else {
        (moves.lz_after, moves.lz_after_scale)
    };
    let (e, e_scale) = if zero(before.e, before.e_scale) {
        (moves.e, moves.e_scale)
    } else {
        (moves.e_after, moves.e_after_scale)
    };
    Classes {
        lz,
        lz_scale,
        e,
        e_scale,
    }
}

/// What layer 1's clamps move (§ "The class a clamp moves", REQ-ENC-038): the change of `L_z` and of `E` from the
/// entered state's parameters to the clamped ones, in exact arithmetic on those parameters, and the scale each is
/// zero within.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Moves {
    /// `ΔL_z`.
    pub lz: f64,
    /// `ΔL_z`'s scale.
    pub lz_scale: f64,
    /// `ΔE`.
    pub e: f64,
    /// `ΔE`'s scale.
    pub e_scale: f64,
    /// `L_z` after the clamps, evaluated on the clamped parameters.
    pub lz_after: f64,
    /// Its scale.
    pub lz_after_scale: f64,
    /// `E` after the clamps, evaluated on the clamped parameters.
    pub e_after: f64,
    /// Its scale.
    pub e_after_scale: f64,
}

/// A value `v` of the entered state's parameters, its change `d` under the clamps and its value `w` after them, each
/// with its first-order rounding in units of `ε`: `|δv| ≤ ε·r`, `|δd| ≤ ε·s` and `|δw| ≤ ε·t`. Each operation forms
/// `d` from its operands' changes, never as the difference of two rounded values, and `w` from its operands' values
/// after the clamps; a parameter no clamp moved has `d = s = 0` exactly.
#[derive(Clone, Copy, Debug)]
struct Dual {
    v: f64,
    d: f64,
    w: f64,
    r: f64,
    s: f64,
    t: f64,
}

impl Dual {
    /// A parameter, `v` before and `w` after the clamps, `moved` iff a clamp moved it, its rounding `round`; the
    /// clamp's image rounded once.
    fn param(v: f64, w: f64, moved: bool, round: f64) -> Dual {
        let d = if moved { w - v } else { 0.0 };
        Dual {
            v,
            d,
            w: if moved { w } else { v },
            r: round,
            s: if moved { d.abs() + round } else { 0.0 },
            t: if moved { w.abs() } else { round },
        }
    }

    /// `cos θ` and `sin θ`, `θ → θ'`, `θ`'s rounding `round`: `Δcos = −2 sin((θ + θ')/2) sin((θ' − θ)/2)` and
    /// `Δsin = 2 cos((θ + θ')/2) sin((θ' − θ)/2)`.
    fn trig(t: f64, u: f64, moved: bool, round: f64) -> (Dual, Dual) {
        let u = if moved { u } else { t };
        let (mean, half) = (0.5 * (t + u), 0.5 * (u - t));
        let image = if moved { u.abs() } else { round };
        let one = |v: f64, slope: f64, w: f64, slope_w: f64, d: f64| Dual {
            v,
            d,
            w,
            r: slope.abs() * round + v.abs(),
            s: if moved { d.abs() + round } else { 0.0 },
            t: slope_w.abs() * image + w.abs(),
        };
        (
            one(
                t.cos(),
                t.sin(),
                u.cos(),
                u.sin(),
                -2.0 * mean.sin() * half.sin(),
            ),
            one(
                t.sin(),
                t.cos(),
                u.sin(),
                u.cos(),
                2.0 * mean.cos() * half.sin(),
            ),
        )
    }

    fn add(self, o: Dual) -> Dual {
        let (v, d, w) = (self.v + o.v, self.d + o.d, self.w + o.w);
        Dual {
            v,
            d,
            w,
            r: self.r + o.r + v.abs(),
            s: self.s + o.s + d.abs(),
            t: self.t + o.t + w.abs(),
        }
    }

    fn neg(self) -> Dual {
        Dual {
            v: -self.v,
            d: -self.d,
            w: -self.w,
            ..self
        }
    }

    fn sub(self, o: Dual) -> Dual {
        self.add(o.neg())
    }

    fn half(self) -> Dual {
        Dual {
            v: 0.5 * self.v,
            d: 0.5 * self.d,
            w: 0.5 * self.w,
            r: 0.5 * self.r,
            s: 0.5 * self.s,
            t: 0.5 * self.t,
        }
    }

    /// `Δ(xy) = Δx·y' + x·Δy`.
    fn mul(self, o: Dual) -> Dual {
        let (v, d, w) = (self.v * o.v, self.d * o.w + self.v * o.d, self.w * o.w);
        Dual {
            v,
            d,
            w,
            r: self.r * o.v.abs() + self.v.abs() * o.r + v.abs(),
            s: self.s * o.w.abs()
                + self.d.abs() * o.t
                + self.r * o.d.abs()
                + self.v.abs() * o.s
                + d.abs(),
            t: self.t * o.w.abs() + self.w.abs() * o.t + w.abs(),
        }
    }

    /// `Δ(x/y) = Δx/y' − x·Δy/(y·y')`, `y` positive.
    fn div(self, o: Dual) -> Dual {
        let (x, y, z) = (self.v, o.v, o.w);
        let (v, d, w) = (x / y, self.d / z - x * o.d / (y * z), self.w / z);
        let slope = self.d.abs() / (z * z) + x.abs() * o.d.abs() * (y + z) / (y * z * y * z);
        Dual {
            v,
            d,
            w,
            r: (self.r + v.abs() * o.r) / y + v.abs(),
            s: self.s / z
                + x.abs() * o.s / (y * z)
                + self.r * o.d.abs() / (y * z)
                + o.r * slope
                + d.abs(),
            t: (self.t + w.abs() * o.t) / z + w.abs(),
        }
    }

    /// `Δ√x = Δx/(√x + √x')`, `x` positive.
    fn sqrt(self) -> Dual {
        let (a, b) = (self.v.sqrt(), self.w.sqrt());
        let den = a + b;
        let d = self.d / den;
        Dual {
            v: a,
            d,
            w: b,
            r: self.r / (2.0 * a) + a,
            s: self.s / den + self.r * d.abs() * (0.5 / a + 0.5 / b) / den + d.abs(),
            t: self.t / (2.0 * b) + b,
        }
    }
}

/// [`Moves`] of the encoded `e` under its clamps, `z` the clamped controls, `ext` the entry's extents, under `S`
/// (§ "The class a clamp moves", REQ-ENC-038; dd_decoder §3.1, §3.2, §3.4). The entered state's parameters are the
/// block inverses': `α`, `β`, the masses and the Jacobi momenta `q = (p_ρ, p_λ)` in the canonical frame. A clamped
/// control takes its link's value at its clamped `z`; every other keeps its own. With `M₀₁ = m₀ + m₁`,
/// `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁`, `a = ‖ρ‖ = cos α/√μ_ρ` and `b = ‖λ‖ = sin α/√μ_λ`:
/// `L_z = a·q₁ + b·(cos β·q₃ − sin β·q₂)`, `K = (q₀² + q₁²)/2μ_ρ + (q₂² + q₃²)/2μ_λ` and
/// `U = −(m₀m₁/a + m₀m₂/r₀₂ + m₁m₂/r₁₂)`, `r₀₂ = ‖λ + (m₁/M₀₁)ρ‖`, `r₁₂ = ‖λ − (m₀/M₀₁)ρ‖`, `ρ = (a, 0)`,
/// `λ = b·(cos β, sin β)`. The parameters' rounding, in units of `ε`: `mᵢ` for each mass, `R·(√μ_ρ + √μ_λ) + α`
/// for `α`, `R/b + R/a + β` for `β` and `P + (R/a + R + 1)·maxₖ|qₖ|` for each `qₖ`, `R/a` the canonical frame's
/// own, its rotation being `ρ`'s direction, and `R` the rescale's. The `α = 0` pole is read at the register's
/// `zero_class` ([`moves_within`]).
pub fn moves<S: Selection>(e: &kernel::encode::Encoded<f64>, z: &[f64; 8], ext: &Extents) -> Moves {
    moves_within::<S>(e, z, ext, ZERO_CLASS.number() * f64::EPSILON)
}

/// [`moves`], the `α = 0` pole read at the zero-class band `pole` (§ "The class a clamp moves", "At the `α = 0`
/// pole"; RQ-565, interim, applied per R-369): where `b` is 0 within `pole` of its bound, `β` is undefined, so each
/// move is its worst case over every direction of `λ` after the clamp, the one of largest magnitude, and each value
/// after it the one of the range nearest the other side of its value before: for `L_z` the directions along
/// `±(q₃, −q₂)`, where `λ × p_λ = ±b·‖p_λ‖`, and for `E` those with `cos β = ±1` or `r₀₂ = r₁₂`, the extremes of `U`.
pub fn moves_within<S: Selection>(
    e: &kernel::encode::Encoded<f64>,
    z: &[f64; 8],
    ext: &Extents,
    pole: f64,
) -> Moves {
    let bits = e.blocks.clamped;
    let on = |k: u32| bits & (1 << k) != 0;
    let (za, zb) = config::angles::<S, f64>([z[0], z[1]]);
    let (alpha, beta) = (e.blocks.alpha, e.blocks.beta);
    let j = &e.blocks.jacobi;
    let q = [j.p_rho[0], j.p_rho[1], j.p_lambda[0], j.p_lambda[1]];
    let zq = momentum::controls::<S, f64>([z[2], z[3], z[4], z[5]]);
    let m = e.canonical.ic.m;
    let mass_moved = bits & 0b1100_0000 != 0;
    let m2 = mass::decode::<S, f64>([z[6], z[7]]).m;
    let ms: [Dual; 3] = std::array::from_fn(|k| Dual::param(m[k], m2[k], mass_moved, m[k]));
    let m01 = ms[0].add(ms[1]);
    let mu_rho = ms[0].mul(ms[1]).div(m01);
    let mu_lambda = ms[2].mul(m01);
    let alpha_round = ext.r * (mu_rho.v.sqrt() + mu_lambda.v.sqrt()) + alpha.abs();
    let (cos_a, sin_a) = Dual::trig(alpha, za, on(0), alpha_round);
    let a = cos_a.div(mu_rho.sqrt());
    let b = sin_a.div(mu_lambda.sqrt());
    // The frame's rotation is `ρ`'s direction, rounded by `R/a`; it turns every canonical vector by as much.
    let over = |len: f64| if len > 0.0 { ext.r / len } else { 0.0 };
    let frame = over(a.v);
    let beta_round = over(b.v) + frame + beta.abs();
    let q_bound = q.iter().fold(0.0_f64, |acc, v| acc.max(v.abs()));
    let q_round = ext.p + (frame + ext.r + 1.0) * q_bound;
    let qs: [Dual; 4] =
        std::array::from_fn(|k| Dual::param(q[k], zq[k], on(2 + k as u32), q_round));
    let square = |x: Dual| x.mul(x);
    let kinetic = square(qs[0])
        .add(square(qs[1]))
        .half()
        .div(mu_rho)
        .add(square(qs[2]).add(square(qs[3])).half().div(mu_lambda));
    let (w0, w1) = (ms[0].div(m01), ms[1].div(m01));
    // `L_z` and `E` at the direction `(c, s)` of `λ`.
    let at = |c: Dual, s: Dual| {
        let lz = a.mul(qs[1]).add(b.mul(c.mul(qs[3]).sub(s.mul(qs[2]))));
        let (bc, y) = (b.mul(c), b.mul(s));
        let r02 = square(bc.add(w1.mul(a))).add(square(y)).sqrt();
        let r12 = square(bc.sub(w0.mul(a))).add(square(y)).sqrt();
        let potential = ms[0]
            .mul(ms[1])
            .div(a)
            .add(ms[0].mul(ms[2]).div(r02))
            .add(ms[1].mul(ms[2]).div(r12))
            .neg();
        (lz, kinetic.add(potential))
    };
    let (lz, energy) = if b.v <= pole * b.r {
        // `β` before is kept as encoded; after the clamp, `λ`'s direction `(c, s)` is a unit vector, rounded once.
        let toward = |c: f64, s: f64| {
            at(
                Dual::param(beta.cos(), c, true, 1.0),
                Dual::param(beta.sin(), s, true, 1.0),
            )
        };
        let (q2, q3) = (qs[2].w, qs[3].w);
        let n = q2.hypot(q3);
        let (c, s) = if n > 0.0 {
            (q3 / n, -q2 / n)
        } else {
            (1.0, 0.0)
        };
        let lzs = [toward(c, s).0, toward(-c, -s).0];
        let mut es = vec![toward(1.0, 0.0).1, toward(-1.0, 0.0).1];
        // `r₀₂ = r₁₂` where `cos β = (w₀ − w₁)·a/2b`, after the clamp.
        let even = (w0.w - w1.w) * a.w / (2.0 * b.w);
        if even.abs() <= 1.0 {
            es.push(toward(even, (1.0 - even * even).sqrt()).1);
        }
        (worst(&lzs), worst(&es))
    } else {
        let (c, s) = Dual::trig(beta, zb, on(1), beta_round);
        let (lz, energy) = at(c, s);
        let one = |x: Dual| Worst { moved: x, after: x };
        (one(lz), one(energy))
    };
    Moves {
        lz: lz.moved.d,
        lz_scale: lz.moved.s,
        e: energy.moved.d,
        e_scale: energy.moved.s,
        lz_after: lz.after.w,
        lz_after_scale: lz.after.t,
        e_after: energy.after.w,
        e_after_scale: energy.after.t,
    }
}

/// A constant's worst case over a range of values after the clamps: the move of largest magnitude, the first where
/// two tie, and the value after nearest the other side of the value before, the largest where the value before is 0
/// (in class 0, so it takes its move and its value after is not read).
#[derive(Clone, Copy, Debug)]
struct Worst {
    moved: Dual,
    after: Dual,
}

/// The worst case over `range`, which is not empty, all one constant's, so of one value before. `min_by` returns the
/// first of equal keys.
fn worst(range: &[Dual]) -> Worst {
    let before = range[0].v;
    let first = |key: &dyn Fn(&Dual, &Dual) -> std::cmp::Ordering| {
        *range
            .iter()
            .min_by(|x, y| key(x, y))
            .expect("a range is not empty")
    };
    Worst {
        moved: first(&|x, y| y.d.abs().total_cmp(&x.d.abs())),
        after: if before > 0.0 {
            first(&|x, y| x.w.total_cmp(&y.w))
        } else {
            first(&|x, y| y.w.total_cmp(&x.w))
        },
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
    // Layer 1: the clamp stands unless it changes a class, read from the parameters, not from the decoded value.
    if moved.controls != 0 {
        let ext = Extents::entered(x, e.canonical.lambda);
        let before = Classes::within(&e.canonical.ic, &ext);
        let after = after(
            &before,
            &moves_within::<S>(e, &z, &ext, tol.zero_class),
            tol.zero_class,
        );
        if let Some(class) = changed(&before, &after, tol.zero_class) {
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
