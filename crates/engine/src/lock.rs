//! The lock, CPU-side chart construction held in `SimConfig` (chart_decoder_contract Part 4, "The lock (projective
//! microscope)" and "The lock's construction, defined"; render_gui_spec §G4; gui_state_contract §2; R-69; TASK-M2-23).
//! The lock is chart construction, so locking, unlocking and recentring are undoable through the contract's history,
//! and no edit names `ViewUI`.
//!
//! - **The lock's edits**, setting it on the affine or a nonlinear chart, snap-back, unlock, recentre and a slider's
//!   move, are the chart-path object's (`crate::path`, TASK-M2-61): each is a segment of a named family, its formula in
//!   `path::lock` and `path::family`. Here are the lock's state and what reads it: the affine chart's pixel ([`point`],
//!   [`centre`]), the hidden coordinates a nonlinear chart freezes ([`FrozenControls`]), the re-based sliders
//!   ([`slider`]), the kernel's chart ([`kernel_chart`]), lock validation and the curve tangent tilt.
//! - **The excursion** `δ = z₀ − z_locked` ([`Lock::excursion`]) is held by the stored pair (the contract's definition):
//!   a slice step or a slider move edits `z₀` alone, so `δ` accumulates it; each slider is re-based, not frozen
//!   ([`slider`]), showing `z₀`'s value with the anchor's marked.
//! - **Lock validation** on an invariant chart ([`validate`]): each pixel's `(L_z, y)` read at its own configuration,
//!   tagged where layer 2 fails it, or `DEGENERATE` where its construction finds no seed, never dropped; one notice for
//!   the view, the switch to the latent chart with a re-centre only where the lock's own centre has gone infeasible
//!   (R-593; [`Report::offer`]; its re-centre is a `crate::path` segment).
//! - **A curve's tangent tilt needs a lock** ([`curve_tilt`]): `γ′(ν₀)` pulled back to the latent controls
//!   ([`curve_tangent`]), refused where a link's inverse clamps, then the tilt formula (`crate::path::tilt_tangent`).
//!
//! **What the GPU knows: nothing** (Part 4). The kernel's chart is [`kernel_chart`]: `(z₀, q₁, q₂)` and nothing of the
//! lock, so a locked and a free view of the same plane send the kernel the same uniform.

use kernel::chart::mixed::{Coordinate, Quantity};
use kernel::chart::{
    Chart, Constraint, InvariantLE, InvariantLK, Latent as LatentChart, Mixed, ValidationResult,
};
use kernel::encode::blocks::clamps;
use kernel::encode::by_kind::{fibre, InvariantAxis, Lookup};
use kernel::encode::SimplexClamp;
use kernel::generated::links::{Link, Selection};

use crate::chart_authoring::{CurveMeta, CurvePoint};
use crate::contract::sim_config::{Latent, Lock, Plane, SimConfig, LATENT_DIM};
use crate::lookup::Rejected;
use crate::nav::NavError;
use crate::path::family::tilt_tangent;

/// Why a lock gesture gives no edit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LockError {
    /// An input, or the point it gives, is not finite.
    NonFinite,
    /// The gesture needs a lock and the view is free: snap-back, unlock, and a curve axis's tangent tilt (Part 4,
    /// "curve axes tilt only at a lock").
    NotLocked,
    /// A slider index past the eight latent controls.
    NoSuchControl(usize),
    /// The clicked chart point fails the chart's `validate` (R-469): it is not a system, and a lock there is refused.
    NotASystem(ValidationResult),
    /// Lookup's ladder refused the clicked state (inverse_encode_contract § "Chart-aware validation").
    Refused(Rejected),
    /// The tangent tilt gave no direction.
    Nav(NavError),
    /// A curve's tangent at a point where a link's inverse clamps (the contract's definition): `controls[k]` set for
    /// each control `z[k]` whose inverse clamps, so its pullback is not `γ′(ν₀)`'s.
    Saturated { controls: [bool; LATENT_DIM] },
}

/// [`LockError::NonFinite`] unless every value is finite.
pub(crate) fn finite(values: &[f64]) -> Result<(), LockError> {
    match values.iter().all(|x| x.is_finite()) {
        true => Ok(()),
        false => Err(LockError::NonFinite),
    }
}

/// The latent chart of `plane`, `Φ(s, t) = z₀ + (2s−1)q₁ + (2t−1)q₂`.
fn latent_chart(plane: &Plane) -> LatentChart<f64> {
    LatentChart {
        z0: plane.z0,
        q1: plane.q1,
        q2: plane.q2,
    }
}

/// The affine chart's pixel `(s, t)` of `plane`: the latent chart's own `Φ` (`kernel::chart::Latent`), the one formula
/// the decode reads, so a pixel and its lock agree bit for bit.
pub fn point(plane: &Plane, s: f64, t: f64) -> Latent {
    latent_chart(plane).map::<f64>(s, t).z.z
}

/// The centre pixel `z(½, ½)`: `z₀` whatever the basis (Part 4), the point anchor-preserving edits keep.
pub fn centre(plane: &Plane) -> Latent {
    point(plane, 0.5, 0.5)
}

/// The view's window on a nonlinear chart's own parameters (the contract's definition), held in `SimConfig`'s chart
/// group (R-69; REQ-SYS-011).
pub use crate::contract::sim_config::Window;

/// A nonlinear chart's hidden coordinates (the contract's definition; `principia_chart_reference.md` §2.1: "Untilted,
/// they are `z₀`'s at every pixel"): the controls it holds frozen, and which of the eight its axes set, which it does
/// not read from the frozen ones.
pub trait FrozenControls {
    /// The frozen controls, in the decoder's block layout; those the axes set are not read.
    fn frozen(&self) -> Latent;
    /// `true` for each control the axes set.
    fn set_by_axes(&self) -> [bool; LATENT_DIM];
}

/// `true` at the coordinates the two axes set.
fn at_axes(h: Coordinate, v: Coordinate) -> [bool; LATENT_DIM] {
    std::array::from_fn(|k| k == h as usize || k == v as usize)
}

impl<H: Quantity, V: Quantity> FrozenControls for Mixed<H, V, f64> {
    fn frozen(&self) -> Latent {
        self.frozen
    }

    fn set_by_axes(&self) -> [bool; LATENT_DIM] {
        at_axes(H::COORDINATE, V::COORDINATE)
    }
}

/// The invariant charts' axes set the momenta, `z[2:6]`, through the construction; the configuration and the masses
/// are frozen.
const MOMENTA: [bool; LATENT_DIM] = [false, false, true, true, true, true, false, false];

impl FrozenControls for InvariantLE<f64> {
    fn frozen(&self) -> Latent {
        self.0.frozen::<f64>().z
    }

    fn set_by_axes(&self) -> [bool; LATENT_DIM] {
        MOMENTA
    }
}

impl FrozenControls for InvariantLK<f64> {
    fn frozen(&self) -> Latent {
        self.0.frozen::<f64>().z
    }

    fn set_by_axes(&self) -> [bool; LATENT_DIM] {
        MOMENTA
    }
}

/// One slider of the Slice centre, re-based to the anchor while locked, not frozen (Part 4, "Sliders are re-based").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider {
    /// The control's value, `z₀[k] = z_anchor[k] + δ[k]`, shown live.
    pub value: f64,
    /// The anchor's value, ghost-marked on the control; `None` while free.
    pub anchor: Option<f64>,
    /// The excursion's component `δ[k]`; `None` while free.
    pub offset: Option<f64>,
}

/// The slider of the control `k` in `sim`.
pub fn slider(sim: &SimConfig, k: usize) -> Result<Slider, LockError> {
    let value = *sim.plane.z0.get(k).ok_or(LockError::NoSuchControl(k))?;
    let locked = sim.lock.locked;
    Ok(Slider {
        value,
        anchor: locked.then_some(sim.lock.z_locked[k]),
        offset: sim.lock.excursion(&sim.plane.z0).map(|d| d[k]),
    })
}

/// The chart the kernel receives for `sim` (Part 4, "What the GPU knows about all of this: nothing"): the latent chart
/// of `(z₀, q₁, q₂)`, narrowed to the kernel's f32 on the CPU. It reads nothing of the lock, the excursion or the
/// slice frame, so a locked and a free view of one plane give one uniform.
pub fn kernel_chart(sim: &SimConfig) -> LatentChart<f32> {
    let narrow = |v: &Latent| v.map(|x| x as f32);
    LatentChart {
        z0: narrow(&sim.plane.z0),
        q1: narrow(&sim.plane.q1),
        q2: narrow(&sim.plane.q2),
    }
}

// ── Lock validation ────────────────────────────────────────────────────────────────────────────────────────────────

/// One pixel of a locked view on an invariant chart: the chart frozen at the pixel's own hidden coordinates, and the
/// pair it reads there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel<C> {
    /// The chart, its frozen configuration and mass controls the pixel's.
    pub chart: C,
    /// `L_z`.
    pub lz: f64,
    /// The vertical value, `E` on `(L_z, E)` and `K` on `(L_z, K)`.
    pub y: f64,
}

/// A tagged pixel's nearest feasible point (inverse_encode_contract § "The nearest valid point").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nearest {
    /// Its `L_z`.
    pub lz: f64,
    /// Its kinetic energy `K*`, kept unless it was below 0.
    pub k: f64,
    /// Its latent `z`: the pixel's frozen controls with the constructed momenta.
    pub z: Latent,
}

/// What lock validation says of one pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tag {
    /// The pair is feasible at the pixel's configuration.
    Feasible,
    /// The pair fails `failed` there; `nearest` is its nearest feasible point, `None` where projecting would change
    /// `L_z`'s class or the pair is not finite.
    Infeasible {
        failed: Constraint,
        nearest: Option<Nearest>,
    },
    /// The construction's seed rule found no seed (R-82): the pixel's `DEGENERATE` label.
    Degenerate,
}

/// What lock validation found: one tag per pixel, in order, none dropped, and the lock's own centre's.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// The tags.
    pub tags: Vec<Tag>,
    /// The tag of the lock's own centre, the view's centre pixel `(s, t) = (½, ½)`, whose IC is `z₀`.
    pub centre: Tag,
}

/// The one notice lock validation gives a view with a pixel tagged infeasible (inverse_encode_contract § "Lock
/// validation"; R-593): it always offers the switch to the latent chart, where the lock is valid, the latent chart
/// having no layer-2 constraint, and the re-centre only where the lock's own centre has gone infeasible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Notice {
    /// The re-centre: a lock at the centre's nearest feasible point ([`recentre`]); `None` where the centre is
    /// feasible, `DEGENERATE`, or tagged with no nearest point.
    pub recentre: Option<Nearest>,
}

impl Report {
    /// The number of infeasible pixels.
    pub fn infeasible(&self) -> usize {
        self.tags
            .iter()
            .filter(|t| matches!(t, Tag::Infeasible { .. }))
            .count()
    }

    /// The offer (R-593): no notice where neither a pixel nor the centre is tagged infeasible; else one notice for the
    /// view, never one per pixel, offering the switch to the latent chart and, only where the lock's own centre is
    /// tagged infeasible with a nearest feasible point, the re-centre there. The other tagged pixels' nearest points
    /// stay in their tags and are not offered.
    pub fn offer(&self) -> Option<Notice> {
        let recentre = match self.centre {
            Tag::Infeasible { nearest, .. } => nearest,
            Tag::Feasible | Tag::Degenerate if self.infeasible() == 0 => return None,
            Tag::Feasible | Tag::Degenerate => None,
        };
        Some(Notice { recentre })
    }
}

/// Lock validation of a locked view's `pixels` on an invariant chart, under the link selection `S` (the contract's
/// definition): each pair, and the lock's own `centre`, the view's centre pixel, tested at its own configuration by the
/// chart's own feasibility test and remedy (`kernel::encode::by_kind::fibre`), and tagged.
pub fn validate<S, C>(pixels: &[Pixel<C>], centre: &Pixel<C>) -> Report
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: InvariantAxis,
    f64: From<C::Param>,
{
    let tag = |p: &Pixel<C>| match fibre::<S, f64, C>(&p.chart, p.lz, p.y) {
        Lookup::Encoded {
            projected: None, ..
        } => Tag::Feasible,
        Lookup::Encoded {
            z,
            lz,
            k,
            projected: Some(failed),
            ..
        } => Tag::Infeasible {
            failed,
            nearest: Some(Nearest { lz, k, z }),
        },
        Lookup::Rejected { failed } => Tag::Infeasible {
            failed,
            nearest: None,
        },
        Lookup::Degenerate => Tag::Degenerate,
    };
    Report {
        tags: pixels.iter().map(tag).collect(),
        centre: tag(centre),
    }
}

// ── A curve axis's tangent tilt ────────────────────────────────────────────────────────────────────────────────────

/// `softmax(0, μ₁, μ₂)`, the masses of the curve's mass logits (`CurvePoint`'s definition). Its `exp` rounds correctly
/// (`crate::exact`), as every transcendental of [`curve_tangent`]: the platforms' own differ in a last bit, which the
/// central difference there amplifies, so the tangent tilt had other bits on macOS than on Linux.
fn masses(mu: [f64; 2]) -> [f64; 3] {
    masses_by(mu, crate::exact::exp)
}

/// [`masses`] with `exp` as its exponential: the correctly rounded one there, another in its test's control.
fn masses_by(mu: [f64; 2], exp: fn(f64) -> f64) -> [f64; 3] {
    let top = mu[0].max(mu[1]).max(0.0);
    let e = [exp(-top), exp(mu[0] - top), exp(mu[1] - top)];
    let sum = e[0] + e[1] + e[2];
    e.map(|x| x / sum)
}

/// `γ′(ν₀)` of `curve` pulled back to the latent controls under the link selection `S`, at the embed's controls
/// `z(ν₀)` (the contract's definition): `dz_α = dα/f_α′(z_α)`, `dz_β = dβ/f_β′(z_β)`, `f′ = exp(log_det)`; the momenta
/// 0, the curve a rest start; the mass controls the derivative of the mass link's inverse along `(dμ₁, dμ₂)`, by a
/// central difference of step `h = ε^{1/3}·(1 + ‖μ‖)/‖dμ‖`. [`LockError::Saturated`], naming the controls, where `α`'s
/// or `β`'s inverse clamps at `z(ν₀)`, or the mass link's at `μ` or at either difference point `μ ± h·dμ`.
pub fn curve_tangent<S>(curve: &CurveMeta, nu0: f64) -> Result<Latent, LockError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    finite(&[nu0])?;
    let p: CurvePoint = (curve.embed)(nu0);
    let d: CurvePoint = (curve.tangent)(nu0);
    finite(&[p.alpha, p.beta, p.mu1, p.mu2, d.alpha, d.beta, d.mu1, d.mu2])?;
    let [z_alpha] = <S::AlphaLink as Link<1, 1>>::inverse([p.alpha]);
    let [z_beta] = <S::BetaLink as Link<1, 1>>::inverse([p.beta]);
    let slope_alpha = crate::exact::exp(<S::AlphaLink as Link<1, 1>>::log_det([z_alpha]));
    let slope_beta = crate::exact::exp(<S::BetaLink as Link<1, 1>>::log_det([z_beta]));
    let dmu = [d.mu1, d.mu2];
    let size = crate::exact::hypot(dmu[0], dmu[1]);
    let masses_at = |k: f64| masses([p.mu1 + k * dmu[0], p.mu2 + k * dmu[1]]);
    let mut mass_clamps = <S::MassLink as SimplexClamp>::clamps(masses([p.mu1, p.mu2]));
    // A curve that moves no mass has a mass tangent of 0, with no step to take.
    let dz_mass = match size == 0.0 {
        true => [0.0; 2],
        false => {
            let h =
                crate::exact::cbrt(f64::EPSILON) * (1.0 + crate::exact::hypot(p.mu1, p.mu2)) / size;
            let (m_hi, m_lo) = (masses_at(h), masses_at(-h));
            mass_clamps |= <S::MassLink as SimplexClamp>::clamps(m_hi)
                | <S::MassLink as SimplexClamp>::clamps(m_lo);
            let hi = <S::MassLink as Link<2, 3>>::inverse(m_hi);
            let lo = <S::MassLink as Link<2, 3>>::inverse(m_lo);
            [(hi[0] - lo[0]) / (2.0 * h), (hi[1] - lo[1]) / (2.0 * h)]
        }
    };
    let saturated = [
        clamps::<S::AlphaLink, f64>(p.alpha),
        clamps::<S::BetaLink, f64>(p.beta),
        false,
        false,
        false,
        false,
        mass_clamps & 1 != 0,
        mass_clamps & 2 != 0,
    ];
    if saturated.contains(&true) {
        return Err(LockError::Saturated {
            controls: saturated,
        });
    }
    let tangent = [
        d.alpha / slope_alpha,
        d.beta / slope_beta,
        0.0,
        0.0,
        0.0,
        0.0,
        dz_mass[0],
        dz_mass[1],
    ];
    Ok(tangent)
}

/// A curve axis's tangent tilt (Part 4, "Directions are axis kinds"): `q_tilt(τ) = length·normalise(cos τ·γ̂′(ν₀) +
/// sin τ·d̂)`, `γ′(ν₀)` the [`curve_tangent`] at the centre's `ν₀`. Refused without a lock: the tangent moves along the
/// curve, so the tilt is well-posed only at a pinned centre.
pub fn curve_tilt<S>(
    lock: &Lock,
    curve: &CurveMeta,
    nu0: f64,
    length: f64,
    tau_deg: f64,
    target: &Latent,
) -> Result<Latent, LockError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    if !lock.locked {
        return Err(LockError::NotLocked);
    }
    let tangent = curve_tangent::<S>(curve, nu0)?;
    tilt_tangent(&tangent, length, tau_deg, target).map_err(LockError::Nav)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mass logits and their masses, as bits, each `exp`'s exact value from mpmath: at the first the pinned libm's `exp`
    /// gives the third mass an ulp off, at the second macOS's own gives the second mass an ulp off (measured on an M3
    /// Pro).
    const MASSES: [([u64; 2], [u64; 3]); 2] = [
        (
            [0xc005_eb85_1eb8_51ec, 0xc009_51eb_851e_b852],
            [
                0x3fec_e99f_2278_844a,
                0x3fad_decd_edbe_5c0f,
                0x3fa3_873f_eab9_5f54,
            ],
        ),
        (
            [0xbffa_4dd2_f1a9_fbe8, 0x3ffe_624d_d2f1_a9fc],
            [
                0x3fc0_4261_7e8a_7b8e,
                0x3f99_2190_8f8f_1b4a,
                0x3feb_265b_1be0_e843,
            ],
        ),
    ];

    /// Panics unless `masses` at each logit pair of [`MASSES`] are its bits.
    fn check(masses: fn([f64; 2]) -> [f64; 3]) {
        for (mu, want) in MASSES {
            let got = masses(mu.map(f64::from_bits)).map(f64::to_bits);
            assert!(
                got == want,
                "the curve's masses are not correctly rounded: {got:x?}, want {want:x?}"
            );
        }
    }

    /// The control's exponential: on macOS the platform's own; elsewhere, where it may be exact here, the pinned libm's.
    #[cfg(feature = "controls")]
    fn inexact_exp(x: f64) -> f64 {
        if cfg!(target_os = "macos") {
            x.exp()
        } else {
            libm::exp(x)
        }
    }

    /// **The criterion**, written before the run: the curve's masses at each pair of [`MASSES`] are its bits exactly, on
    /// every platform.
    #[test]
    fn curve_masses_exact_bits() {
        check(masses);
    }

    validation::negative_control!(
        curve_masses_exact_bits,
        "the platform's exp in the curve's masses (macOS), or the pinned libm's (elsewhere)",
        expected = "the curve's masses are not correctly rounded",
        check(|mu| masses_by(mu, inexact_exp))
    );
}
