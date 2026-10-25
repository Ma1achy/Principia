//! The lock, CPU-side chart construction held in `SimConfig` (chart_decoder_contract Part 4, "The lock (projective
//! microscope)" and "The lock's construction, defined"; render_gui_spec §G4; gui_state_contract §2; R-69; TASK-M2-23).
//! Every gesture here computes typed [`SetField`] edits of `SimConfig`'s `z₀` and lock group, and nothing else: the
//! lock is chart construction, so locking, unlocking and recentring are undoable through the contract's history, and
//! no edit names `ViewUI`.
//!
//! - **Setting the lock** on the affine chart ([`set`]): `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂`, the latent chart's own
//!   `Φ` at the clicked pixel, CPU arithmetic with no readback; on a nonlinear chart ([`set_on_chart`]): `Φ` and `D`
//!   replayed on the CPU at the pixel's chart point, lookup's ladder, the inverse-encode path, giving `z_locked` the
//!   controls the chart's axes set and the pixel's own frozen controls the rest, bit for bit ([`FrozenControls`]), and
//!   the chart's [`Window`] re-centred there. Either way the edits are the `SetField` on `z₀ = z_locked` (the
//!   recentring, §G4) and the lock group's, one command sent under one history key, one undo entry (gui_state_contract
//!   §2, R-466), and the centre pixel, of the chart rebuilt on that `z₀`, decodes the clicked pixel's state bit for bit.
//!   On a nonlinear chart `z_locked`'s own decode is within lookup's `ε_phys` of that state where the ladder clamps
//!   none of the controls the axes set; where it clamps one, `z_locked` holds the clamp's image and the lock's notices
//!   carry `lookup_clamped`.
//! - **Anchor-preserving edits** (tilt, zoom, chart-mode switch) are basis edits (`crate::nav`) that turn about the pin
//!   (render_gui_spec §G4; R-451; R-466 K5). At `δ = 0` the pin is the centre, so they keep `z₀` and the centre
//!   `z(½,½) = z₀` ([`centre`]) is exactly constant through them; where `δ` has an in-plane part the pin is off-centre,
//!   and the edit's `SetField` on `z₀` moves `z₀`, and with it `δ`.
//! - **The excursion** `δ = z₀ − z_locked` ([`Lock::excursion`]) is held by the stored pair (the contract's definition):
//!   a slice step or a slider move ([`move_slider`]) edits `z₀` alone, so `δ` accumulates it; each slider is re-based,
//!   not frozen ([`slider`]), showing `z₀`'s value with the anchor's marked; snap-back ([`snap_back`]) is
//!   `z₀ ← z_locked`, the stored anchor, bit for bit; unlock ([`unlock`]) clears the lock group and keeps
//!   `(z₀, q₁, q₂)`.
//! - **Lock validation** on an invariant chart ([`validate`]): each pixel's `(L_z, y)` read at its own configuration,
//!   tagged where layer 2 fails it, or `DEGENERATE` where its construction finds no seed, never dropped, with the
//!   re-centre and the switch to the latent chart offered ([`Report::offer`], [`recentre`]).
//! - **A curve's tangent tilt needs a lock** ([`curve_tilt`]): `γ′(ν₀)` pulled back to the latent controls
//!   ([`curve_tangent`]), refused where a link's inverse clamps, then `nav::tilt_tangent`.
//!
//! **What the GPU knows: nothing** (Part 4). The kernel's chart is [`kernel_chart`]: `(z₀, q₁, q₂)` and nothing of the
//! lock, so a locked and a free view of the same plane send the kernel the same uniform.

use kernel::chart::mixed::{Coordinate, Quantity};
use kernel::chart::{
    Chart, Constraint, InvariantLE, InvariantLK, Latent as LatentChart, Mixed, ValidationResult,
};
use kernel::decode::Ic;
use kernel::encode::blocks::clamps;
use kernel::encode::by_kind::{fibre, ByKind, InvariantAxis, Lookup};
use kernel::encode::SimplexClamp;
use kernel::generated::links::{Link, Selection};

use crate::chart_authoring::{CurveMeta, CurvePoint};
use crate::contract::set_field::{Edit, SetField, SimField};
use crate::contract::sim_config::{Latent, Lock, Plane, SimConfig, LATENT_DIM};
use crate::lookup::{self, Exact, Notices, Rejected, Tolerances};
use crate::nav::{self, NavError};

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
fn finite(values: &[f64]) -> Result<(), LockError> {
    match values.iter().all(|x| x.is_finite()) {
        true => Ok(()),
        false => Err(LockError::NonFinite),
    }
}

/// A `SimConfig` edit, entering the undo history (R-69).
fn edit(field: SimField) -> SetField {
    SetField {
        edit: Edit::Sim(field),
        no_history: false,
    }
}

/// The lock's edits at `z_locked`: the recentring, a `SetField` on `z₀` (§G4), then the lock group, flag and anchor.
/// The pair is one command: the binding sends it in one frame under one history key, so the history keeps one entry
/// and one undo restores both (gui_state_contract §2, "A command of several edits is one history entry"; R-466).
fn lock_edits(z_locked: Latent) -> [SetField; 2] {
    [
        edit(SimField::Z0(z_locked)),
        edit(SimField::Lock(Lock {
            locked: true,
            z_locked,
        })),
    ]
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

/// Sets the lock at the affine chart's pixel `(s, t)` (Part 4, "Setting the lock"): `z_locked = z₀ + (2s−1)q₁ +
/// (2t−1)q₂`, CPU arithmetic with no readback, as the recentring `SetField` on `z₀` and the lock group's, one command
/// under one history key (gui_state_contract §2, R-466).
pub fn set(plane: &Plane, s: f64, t: f64) -> Result<[SetField; 2], LockError> {
    finite(&[s, t])?;
    let z_locked = point(plane, s, t);
    finite(&z_locked)?;
    Ok(lock_edits(z_locked))
}

/// The view's window on a nonlinear chart's own parameters (the contract's definition): the pixel `(s, t)` is the chart
/// point `c + ((2s−1)·h_u, (2t−1)·h_v)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// The centre `c`, the chart point of the pixel `(½, ½)`.
    pub centre: [f64; 2],
    /// The half-widths `(h_u, h_v)`.
    pub half: [f64; 2],
}

impl Window {
    /// The chart point of the pixel `(s, t)`.
    pub fn at(&self, s: f64, t: f64) -> [f64; 2] {
        [
            self.centre[0] + (s + s - 1.0) * self.half[0],
            self.centre[1] + (t + t - 1.0) * self.half[1],
        ]
    }
}

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

/// A lock set on a nonlinear chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartLock {
    /// The recentring `SetField` on `z₀ = z_locked` and the lock group's, one command under one history key
    /// (gui_state_contract §2, R-466).
    pub edits: [SetField; 2],
    /// The window re-centred on the clicked chart point, its half-widths kept. It is chart construction and belongs in
    /// `SimConfig`'s chart group (R-69), which TASK-M2-21 fills (REQ-SYS-011); until then it is returned here, and
    /// undoing the lock does not restore it.
    pub window: Window,
    /// The clicked pixel's state, `D ∘ Φ` replayed on the CPU: the centre pixel's under the re-centred window.
    pub ic: Ic<f64>,
    /// The ladder's notices on the lock.
    pub notices: Notices,
}

/// Sets the lock at the pixel `(s, t)` of `window` on the nonlinear `chart`, under the link selection `S` (the
/// contract's definition): `Φ` and `D` replayed on the CPU at the pixel's chart point, which must pass `validate`
/// (R-469); the state they decode through lookup's ladder, `norm` and `tol` its `‖·‖_phys` and tolerances, whose `z`
/// gives `z_locked` the controls the chart's axes set, every other control the chart's frozen one, bit for bit, so the
/// chart rebuilt on `z₀ = z_locked` decodes the clicked state at the window re-centred on the chart point, bit for bit.
/// `z_locked`'s own decode is within `tol`'s `ε_phys` of that state where the ladder clamps none of the controls the
/// axes set; where it clamps one, `z_locked` holds the clamp's image and [`ChartLock::notices`] carry `lookup_clamped`.
/// The edits are one command under one history key (gui_state_contract §2, R-466).
pub fn set_on_chart<S, C, N>(
    chart: &C,
    window: &Window,
    s: f64,
    t: f64,
    norm: N,
    tol: &Tolerances,
) -> Result<ChartLock, LockError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: ByKind + FrozenControls,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    finite(&[s, t])?;
    let [u, v] = window.at(s, t);
    finite(&[u, v])?;
    match chart.validate(u, v) {
        ValidationResult::Valid => {}
        refused => return Err(LockError::NotASystem(refused)),
    }
    let ic = chart.decode_at::<S, f64>(u, v).ic;
    let locked = lookup::lookup::<S, C, N>(chart, &Exact::Physical(ic), norm, tol)
        .map_err(LockError::Refused)?;
    let (frozen, set) = (chart.frozen(), chart.set_by_axes());
    let z_locked: Latent = std::array::from_fn(|k| match set[k] {
        true => locked.z[k],
        false => frozen[k],
    });
    Ok(ChartLock {
        edits: lock_edits(z_locked),
        window: Window {
            centre: [u, v],
            half: window.half,
        },
        ic,
        notices: locked.notices,
    })
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

/// Moves the slider of the control `k` to `value`: the `SetField` on `z₀` with that control set, the others kept. While
/// locked it is an excursion along `ê_k` from the anchor, which it keeps.
pub fn move_slider(sim: &SimConfig, k: usize, value: f64) -> Result<SetField, LockError> {
    finite(&[value])?;
    let mut z0 = sim.plane.z0;
    *z0.get_mut(k).ok_or(LockError::NoSuchControl(k))? = value;
    Ok(edit(SimField::Z0(z0)))
}

/// Snap-back: `δ → 0` exactly, the `SetField` on `z₀` setting it to the stored anchor, bit for bit.
pub fn snap_back(sim: &SimConfig) -> Result<SetField, LockError> {
    match sim.lock.locked {
        true => Ok(edit(SimField::Z0(sim.lock.z_locked))),
        false => Err(LockError::NotLocked),
    }
}

/// Unlock: the lock group's `SetField`, flag and anchor cleared; `(z₀, q₁, q₂)` is not edited, so the view as it stands
/// is the new free state.
pub fn unlock(sim: &SimConfig) -> Result<SetField, LockError> {
    match sim.lock.locked {
        true => Ok(edit(SimField::Lock(Lock {
            locked: false,
            z_locked: [0.0; LATENT_DIM],
        }))),
        false => Err(LockError::NotLocked),
    }
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

/// What lock validation found: one tag per pixel, in order, none dropped.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// The tags.
    pub tags: Vec<Tag>,
}

/// A remedy lock validation offers (inverse_encode_contract § "Lock validation").
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Remedy {
    /// Re-centre: a lock at the tagged pixel's nearest feasible point ([`recentre`]).
    Recentre { pixel: usize, nearest: Nearest },
    /// Switch to the latent chart, where the lock is valid: it has no layer-2 constraint.
    SwitchToLatent,
}

impl Report {
    /// The number of infeasible pixels.
    pub fn infeasible(&self) -> usize {
        self.tags
            .iter()
            .filter(|t| matches!(t, Tag::Infeasible { .. }))
            .count()
    }

    /// The remedies offered: none where every pixel is feasible; else the re-centre to each tagged pixel's nearest
    /// feasible point, in pixel order, then the switch to the latent chart. A `Degenerate` pixel is offered nothing.
    /// The contract offers one re-centre, "to the nearest feasible point"; which one for a view of many tagged pixels
    /// is RQ-572's, open, and this offer stands until it is ruled.
    pub fn offer(&self) -> Vec<Remedy> {
        if self.infeasible() == 0 {
            return Vec::new();
        }
        let recentre = self
            .tags
            .iter()
            .enumerate()
            .filter_map(|(pixel, t)| match t {
                Tag::Infeasible {
                    nearest: Some(nearest),
                    ..
                } => Some(Remedy::Recentre {
                    pixel,
                    nearest: *nearest,
                }),
                _ => None,
            });
        recentre.chain([Remedy::SwitchToLatent]).collect()
    }
}

/// Lock validation of a locked view's `pixels` on an invariant chart, under the link selection `S` (the contract's
/// definition): each pair tested at its own configuration by the chart's own feasibility test and remedy
/// (`kernel::encode::by_kind::fibre`), and tagged.
pub fn validate<S, C>(pixels: &[Pixel<C>]) -> Report
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
    }
}

/// The re-centre lock validation offers: the lock at the nearest feasible point, the `SetField` on `z₀` and the lock
/// group's, one command under one history key (gui_state_contract §2, R-466).
pub fn recentre(nearest: &Nearest) -> [SetField; 2] {
    lock_edits(nearest.z)
}

// ── A curve axis's tangent tilt ────────────────────────────────────────────────────────────────────────────────────

/// `softmax(0, μ₁, μ₂)`, the masses of the curve's mass logits (`CurvePoint`'s definition).
fn masses(mu: [f64; 2]) -> [f64; 3] {
    let top = mu[0].max(mu[1]).max(0.0);
    let e = [(-top).exp(), (mu[0] - top).exp(), (mu[1] - top).exp()];
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
    let slope_alpha = <S::AlphaLink as Link<1, 1>>::log_det([z_alpha]).exp();
    let slope_beta = <S::BetaLink as Link<1, 1>>::log_det([z_beta]).exp();
    let dmu = [d.mu1, d.mu2];
    let size = dmu[0].hypot(dmu[1]);
    let masses_at = |k: f64| masses([p.mu1 + k * dmu[0], p.mu2 + k * dmu[1]]);
    let mut mass_clamps = <S::MassLink as SimplexClamp>::clamps(masses([p.mu1, p.mu2]));
    // A curve that moves no mass has a mass tangent of 0, with no step to take.
    let dz_mass = match size == 0.0 {
        true => [0.0; 2],
        false => {
            let h = f64::EPSILON.cbrt() * (1.0 + p.mu1.hypot(p.mu2)) / size;
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
    nav::tilt_tangent(&tangent, length, tau_deg, target).map_err(LockError::Nav)
}
