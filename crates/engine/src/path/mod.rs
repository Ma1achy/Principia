//! The chart-path object (TASK-M2-61; `docs/design/principia_chart_path_sketch.md`; the chart-space model's § 3;
//! chart_composition § 17): a view is a point in chart space and every edit a path through it.
//!
//! - A [`View`] is a [`Map`] seen through the window its `SimConfig` holds: the latent map reads the plane
//!   `(z₀, q₁, q₂)`, a kernel map the chart window `{centre, half}`. The view carries the whole `SimConfig`, so its bytes
//!   and sim key are today's.
//! - An edit is a [`Segment`]: two views, a named [`Family`] and its own [`Timing`]. Each constructor is an operation;
//!   it computes `B` by today's function and records the edits that take `A` to `B` ([`Segment::edits`]). A frame
//!   between the ends calls the very function the operation calls, at its parameter scaled by `s`
//!   ([`Segment::frame`]); the ends are the stored views, `s` compared exactly (§ 17.4).
//! - A [`Path`] is a chain of segments joined bit for bit (Theorem 4).
//! - A frame is recorded by value ([`FrameRecord`]).
//!
//! **Stage 1's fence** (R-863): the families are today's operations. The blend, the plane rotation, R-474's two tilt
//! maps, several fields eased together, alignment and retiming are typed and refuse to be shown
//! ([`PathError::NotBuilt`]) until their stage builds them; no special case is an if-branch outside a family.

pub mod family;
pub mod lock;
mod record;
mod timing;

use kernel::chart::saved::Saved;
use kernel::chart::{Latent as LatentChart, REGISTRY};
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;
use serde::{Deserialize, Serialize};

pub use family::{oblique, tilt_tangent, tilt_vector, Constructor};
pub use lock::ChartLock;
pub use record::{Align, FrameRecord};
pub use timing::{Easing, MonotoneTable, Timing};

use crate::contract::set_field::{Edit, SetField, SimField};
use crate::contract::sim_config::{Latent, Plane, SimConfig, Window};
use crate::lock::{FrozenControls, LockError, Nearest};
use crate::lookup::Tolerances;
use crate::nav::{BasisVector, NavError};

/// Why a path gives no segment or no frame.
#[derive(Clone, Debug, PartialEq)]
pub enum PathError {
    /// The family's own formula refused.
    Nav(NavError),
    /// The lock's own formula refused.
    Lock(LockError),
    /// A family, map or field stage 1 types and does not build (the fence, R-863).
    NotBuilt(&'static str),
    /// The operation does not act on this view's map.
    WrongMap,
    /// Two segments that do not share their join bit for bit (Theorem 4).
    Join,
    /// A record whose `B` its family does not reach from its `A`.
    Rebuilt,
    /// A path parameter or playback fraction that is NaN or outside `[0, 1]`.
    Parameter,
    /// An easing table that is not pinned, monotone and finite.
    Easing,
    /// The kernel refused the operation.
    Refused(&'static str),
    /// A record's bytes that do not read back.
    Record(String),
}

impl From<NavError> for PathError {
    fn from(e: NavError) -> Self {
        PathError::Nav(e)
    }
}

impl From<LockError> for PathError {
    fn from(e: LockError) -> Self {
        PathError::Lock(e)
    }
}

// ── The view ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// A kernel map: which registry chart. The plan is unclear here; chose the registry index alone, the chart's own
/// parameters taken by value where a family needs them (a lock, a save), because today's `SimConfig` holds no chart and
/// its hidden coordinates are `z₀`'s, so storing the chart's words would keep a second copy of `z₀` that a lock or a
/// slider move would leave stale (review 6002173203).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelMap {
    /// The chart's index in `kernel::chart::REGISTRY`.
    pub registry: u32,
}

impl KernelMap {
    /// The kernel map of `chart`; [`PathError::WrongMap`] for a chart the registry does not name.
    pub fn of<C: kernel::chart::Chart>(chart: &C) -> Result<Self, PathError> {
        REGISTRY
            .iter()
            .position(|name| *name == chart.name())
            .map(|i| KernelMap { registry: i as u32 })
            .ok_or(PathError::WrongMap)
    }
}

/// The map: a chart, named coordinates to a signed state. Maps are kernel values; the object never re-implements `Φ`.
/// The composed map and the blend are stage 3's and later (sketch § 2).
#[derive(Clone, Debug, PartialEq)]
pub enum Map {
    /// The latent chart, `Φ = z₀ + (2s−1)q₁ + (2t−1)q₂`, its window the plane.
    Latent,
    /// A registry chart, its window `Chart.window`, its hidden coordinates `z₀`'s.
    Kernel(KernelMap),
    /// A saved chart, by value (Save view as chart).
    Saved(Box<Saved>),
}

/// A view: a map seen through the window its `SimConfig` holds.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// The map.
    pub map: Map,
    /// The whole `SimConfig`: the window, the lock and every other group, as today.
    pub sim: SimConfig,
}

/// The window `(c, Q)`, read from a view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewWindow<'a> {
    /// The latent map's: `c = z₀`, `Q = [q₁ q₂]`.
    Plane(&'a Plane),
    /// A kernel map's: `c = centre`, `Q = diag(half)`.
    Chart(&'a Window),
}

impl View {
    /// A view of the latent map.
    pub fn latent(sim: SimConfig) -> Self {
        View {
            map: Map::Latent,
            sim,
        }
    }

    /// A view of the registry chart `chart`.
    pub fn kernel<C: kernel::chart::Chart>(chart: &C, sim: SimConfig) -> Result<Self, PathError> {
        Ok(View {
            map: Map::Kernel(KernelMap::of(chart)?),
            sim,
        })
    }

    /// The window `(c, Q)`, by the map's window shape (one evaluator per shape).
    pub fn window(&self) -> ViewWindow<'_> {
        match &self.map {
            Map::Latent => ViewWindow::Plane(&self.sim.plane),
            Map::Kernel(_) => ViewWindow::Chart(&self.sim.chart.window),
            Map::Saved(saved) => match **saved {
                Saved::Oblique(_) => ViewWindow::Plane(&self.sim.plane),
                Saved::Curved(_) => ViewWindow::Chart(&self.sim.chart.window),
            },
        }
    }

    /// The plane a window edit acts on: the latent map's; [`PathError::WrongMap`] on a kernel map, whose window edits
    /// are the GUI's until TASK-M2-64; and [`saved_refused`] on a saved chart.
    fn plane(&self) -> Result<&Plane, PathError> {
        match &self.map {
            Map::Latent => Ok(&self.sim.plane),
            Map::Kernel(_) => Err(PathError::WrongMap),
            Map::Saved(_) => Err(saved_refused()),
        }
    }

    /// The view's canonical bytes, by which two views are one view (the record's serialisation).
    pub fn bytes(&self) -> Vec<u8> {
        record::view_bytes(self)
    }
}

/// An edit of a saved chart's view. The plan is unclear here; chose to refuse it in stage 1, because a saved chart holds
/// its definition by value (an oblique one its `z₀`, `q₁`, `q₂`) beside the window in `SimConfig`, so an edit of the
/// window would leave the two disagreeing, and no engine route edits a saved chart's view today: its tilts compose in
/// the kernel (`Composed`), and editing and saving again are TASK-M2-55's and TASK-M2-63's.
fn saved_refused() -> PathError {
    PathError::NotBuilt("an edit of a saved chart's view (TASK-M2-55, TASK-M2-63)")
}

/// `sim` after `edits`, each written as the state store writes it (`contract::store::write`).
fn apply(sim: &SimConfig, edits: &[SetField]) -> SimConfig {
    let mut out = sim.clone();
    for e in edits {
        match &e.edit {
            Edit::Sim(SimField::Z0(z0)) => out.plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => (out.plane.q1, out.plane.q2) = (*q1, *q2),
            Edit::Sim(SimField::Lock(lock)) => out.lock = lock.clone(),
            Edit::Sim(SimField::Window(window)) => out.chart.window = *window,
            other => unreachable!("no family writes {other:?}"),
        }
    }
    out
}

// ── The families ───────────────────────────────────────────────────────────────────────────────────────────────────

/// Why a cut is a cut: an edit with no interior, its `B` stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CutLabel {
    /// Today's chart switch: a jump (the chart-space model's § 3).
    ChartSwitch,
    /// The lock set, on the affine or a nonlinear chart, or its re-centre: today's lock re-centres the view.
    LockRecentre,
    /// Snap-back, `z₀ ← z_locked` by assignment.
    SnapBack,
    /// Unlock: the lock group cleared, the view unchanged.
    Unlock,
    /// An edit that jumps the view while locked (Home, the resets, presets, steers, capture, a chart switch): not
    /// conformant with R-849 (no jump under a lock) until stage 2's sliding anchor.
    JumpUnderLock,
    /// A raw tilt from a view that is not the committed plane's tilt at its start: raw tilts replace rather than
    /// compose, so the direct operation jumps there.
    TiltReplaces,
    /// A fresh oblique plane put in place of the view's.
    Oblique,
}

/// The raw tilt's two forms, each its own route; the targets are the segment's, the same at both ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TiltForm {
    /// One basis vector (`Constructor::tilt`), from `from` to `to` degrees.
    Single {
        /// The vector tilted.
        which: BasisVector,
        /// `τ` at `A`.
        from: f64,
        /// `τ` at `B`.
        to: f64,
        /// The direction tilted toward.
        target: Latent,
    },
    /// Both basis vectors (`Constructor::tilt_pair`), `(τ₁, τ₂)` from `from` to `to`.
    Pair {
        /// `(τ₁, τ₂)` at `A`.
        from: [f64; 2],
        /// `(τ₁, τ₂)` at `B`.
        to: [f64; 2],
        /// `τ₁`'s and `τ₂`'s targets.
        targets: [Latent; 2],
    },
}

/// A blend's path, by name: nothing outside the blend's evaluator assumes the default (R-943). Not built (stage 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendPath {
    /// The default path, block by block in each block's own geometry (design § 17.2).
    BlockLine,
}

/// The named families. Each interior frame calls the direct operation's own function at the scaled parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Family {
    /// `pan(A, s·ds, s·dt)`.
    Pan {
        /// The pan along `q₁`, in `[0,1]` addresses.
        ds: f64,
        /// The pan along `q₂`.
        dt: f64,
    },
    /// `zoom(A, s·octaves)`, log-stepped.
    Zoom {
        /// The zoom, in octaves.
        octaves: f64,
    },
    /// `slice(A, d, s·t)`, along the part of `d` off the plane.
    Slice {
        /// The direction.
        d: Latent,
        /// The step.
        t: f64,
    },
    /// A slider's move, `z₀[k] ← from + s·(to − from)`, `from` `A`'s own value.
    Control {
        /// The control.
        k: usize,
        /// `z₀[k]` at `A`.
        from: f64,
        /// `z₀[k]` at `B`.
        to: f64,
    },
    /// R-474's raw tilt from the committed plane, `τ` lerped.
    RawTilt {
        /// The committed plane the tilt starts from.
        committed: Box<Plane>,
        /// Which tilt, and its angles and targets.
        form: TiltForm,
    },
    /// No interior: `A` until `s = 1`, then `B`.
    Cut {
        /// Why.
        label: CutLabel,
    },
    /// Save view as chart: the same pixels, another map.
    Commit,
    /// Several fields eased together on one `s` (export contract Part 3): not built.
    WindowEase,
    /// R-474's Tip as sheet and Lean each axis, on a latent or a curved view: not built.
    R474Tilt,
    /// § 17.7's geodesic between two planes: not built.
    PlaneRotation,
    /// The state blend, its path named: not built (stage 3).
    Blend(BlendPath),
}

// ── The segment ────────────────────────────────────────────────────────────────────────────────────────────────────

/// One edit: two views, a named family and its timing. Built only by its constructors, each an operation.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    a: View,
    b: View,
    family: Family,
    edits: Vec<SetField>,
    timing: Timing,
}

/// `s` canonicalised: `−0` is `+0`, so one frame has one record; NaN and `s` outside `[0, 1]` are refused.
fn parameter(s: f64) -> Result<f64, PathError> {
    match (0.0..=1.0).contains(&s) {
        true => Ok(s + 0.0),
        false => Err(PathError::Parameter),
    }
}

/// Whether two vectors agree to rounding: each component within `32·ε·‖a‖`. The plan is unclear here; chose 32 ulps of
/// the vector's length because the raw tilt recomputes a basis vector through two normalisations of an 8-vector, each
/// a sum of 8 products and a square root, so a few ulps each. Measured on the baseline's planes at `τ = 0`: bit for bit
/// on the axis and deep planes, 0.49·ε·‖q‖ at most on the skew plane; a tilt that replaces another is off by the
/// replaced tilt itself, a fraction of `‖q‖`.
fn rounding_equal(a: &Latent, b: &Latent) -> bool {
    let size = crate::nav::norm(a).max(crate::nav::norm(b));
    a.iter()
        .zip(b)
        .all(|(x, y)| (x - y).abs() <= 32.0 * f64::EPSILON * size)
}

impl Segment {
    /// The segment from `a` by `family` with `edits`, `B` the view they leave.
    fn of(a: &View, family: Family, edits: Vec<SetField>) -> Self {
        let b = View {
            map: a.map.clone(),
            sim: apply(&a.sim, &edits),
        };
        Segment {
            a: a.clone(),
            b,
            family,
            edits,
            timing: Timing::default(),
        }
    }

    /// The view at `s = 0`.
    pub fn a(&self) -> &View {
        &self.a
    }

    /// The view at `s = 1`.
    pub fn b(&self) -> &View {
        &self.b
    }

    /// The family.
    pub fn family(&self) -> &Family {
        &self.family
    }

    /// The edits that take `A` to `B`, one command, as today's operation returned them.
    pub fn edits(&self) -> &[SetField] {
        &self.edits
    }

    /// The timing.
    pub fn timing(&self) -> &Timing {
        &self.timing
    }

    /// This segment with `timing` in place of its own.
    pub fn with_timing(mut self, timing: Timing) -> Self {
        self.timing = timing;
        self
    }

    /// The frame at path parameter `s`: `s == 0` is `A` and `s == 1` is `B`, the stored views, never computed; between
    /// them the family's own formula at its scaled parameter.
    pub fn frame(&self, s: f64) -> Result<View, PathError> {
        let s = parameter(s)?;
        if s == 0.0 {
            return Ok(self.a.clone());
        }
        if s == 1.0 {
            return Ok(self.b.clone());
        }
        let a = &self.a;
        let edits = match &self.family {
            Family::Pan { ds, dt } => vec![family::pan(a.plane()?, s * ds, s * dt)?],
            Family::Zoom { octaves } => vec![family::zoom(a.plane()?, s * octaves)?],
            Family::Slice { d, t } => vec![family::slice(a.plane()?, d, s * t)?],
            Family::Control { k, from, to } => {
                vec![family::control(&a.sim, *k, from + s * (to - from))?]
            }
            Family::RawTilt { committed, form } => {
                vec![raw_tilt_at(committed, a.plane()?, form, s)?]
            }
            Family::Cut { .. } | Family::Commit => return Ok(a.clone()),
            Family::WindowEase => return Err(PathError::NotBuilt("several fields eased together")),
            Family::R474Tilt => return Err(PathError::NotBuilt("R-474's tilt maps")),
            Family::PlaneRotation => return Err(PathError::NotBuilt("the plane rotation")),
            Family::Blend(_) => return Err(PathError::NotBuilt("the state blend")),
        };
        if self.timing.retime.is_some() {
            return Err(PathError::NotBuilt("retiming"));
        }
        Ok(View {
            map: a.map.clone(),
            sim: apply(&a.sim, &edits),
        })
    }

    /// The frame shown at playback fraction `u`: the frame at `s = ease(u)`. The family never sees `u`.
    pub fn at(&self, u: f64) -> Result<View, PathError> {
        let u = parameter(u)?;
        self.frame(self.timing.easing.ease(u))
    }

    /// The record of the frame at `s`, by value.
    pub fn record(&self, s: f64) -> Result<FrameRecord, PathError> {
        Ok(FrameRecord {
            a: self.a.clone(),
            b: self.b.clone(),
            family: self.family.clone(),
            s_bits: parameter(s)?.to_bits(),
            align: None,
            retime_hash: self.timing.retime,
            anchor_words: None,
        })
    }

    // ── The constructors: today's operations ───────────────────────────────────────────────────────────────────────

    /// Pan by `(ds, dt)` of the view's `[0,1]²` addresses (chart_decoder_contract Part 4, "Pan"): `Δz₀ = 2ds·q₁ +
    /// 2dt·q₂`, the same plane, another window.
    pub fn pan(a: &View, ds: f64, dt: f64) -> Result<Self, PathError> {
        let e = family::pan(a.plane()?, ds, dt)?;
        Ok(Self::of(a, Family::Pan { ds, dt }, vec![e]))
    }

    /// Zoom by `octaves` about the centre (Part 4, "Zoom"; R-83): `q₁, q₂ ×= 2^−octaves`, `z₀` kept.
    pub fn zoom(a: &View, octaves: f64) -> Result<Self, PathError> {
        let e = family::zoom(a.plane()?, octaves)?;
        Ok(Self::of(a, Family::Zoom { octaves }, vec![e]))
    }

    /// Slice by `t` along the hidden direction `d` (Part 4, "Slice"): `z₀ += t·off_plane(d)`.
    pub fn slice(a: &View, d: &Latent, t: f64) -> Result<Self, PathError> {
        let e = family::slice(a.plane()?, d, t)?;
        Ok(Self::of(a, Family::Slice { d: *d, t }, vec![e]))
    }

    /// A slider's move of the control `k` to `value` (Part 4, "Sliders are re-based"): `z₀[k] ← value`, from `A`'s own
    /// value. While locked it is an excursion from the anchor, which it keeps.
    pub fn control(a: &View, k: usize, value: f64) -> Result<Self, PathError> {
        if let Map::Saved(_) = a.map {
            return Err(saved_refused());
        }
        let e = family::control(&a.sim, k, value)?;
        let from = a.sim.plane.z0[k];
        Ok(Self::of(a, Family::Control { k, from, to: value }, vec![e]))
    }

    /// The raw tilt of `which` toward `target` from `from` to `to` degrees, from `constructor`'s committed plane
    /// (Part 4, "Tilt"; chart_reference §1.1). A raw tilt family where the committed plane's tilt at `from` is `A`'s
    /// basis to rounding, so the interior starts at `A`; otherwise a cut labelled `TiltReplaces` (raw tilts replace
    /// rather than compose, so the direct operation jumps there).
    pub fn raw_tilt(
        constructor: &Constructor,
        a: &View,
        which: BasisVector,
        (from, to): (f64, f64),
        target: &Latent,
    ) -> Result<Self, PathError> {
        let form = TiltForm::Single {
            which,
            from,
            to,
            target: *target,
        };
        Self::tilted(constructor, a, form)
    }

    /// The raw tilt of both basis vectors, `(τ₁, τ₂)` from `from` to `to` toward `targets`; as [`Segment::raw_tilt`].
    pub fn raw_tilt_pair(
        constructor: &Constructor,
        a: &View,
        from: [f64; 2],
        to: [f64; 2],
        targets: [&Latent; 2],
    ) -> Result<Self, PathError> {
        let form = TiltForm::Pair {
            from,
            to,
            targets: [*targets[0], *targets[1]],
        };
        Self::tilted(constructor, a, form)
    }

    /// A raw tilt of `form`: its end by the direct operation, its family a raw tilt where it starts at `A`.
    fn tilted(constructor: &Constructor, a: &View, form: TiltForm) -> Result<Self, PathError> {
        let plane = a.plane()?;
        let committed = constructor.committed().clone();
        let end = raw_tilt_at(&committed, plane, &form, 1.0)?;
        let family = match starts_at(&committed, plane, &form) {
            true => Family::RawTilt {
                committed: Box::new(committed),
                form,
            },
            false => Family::Cut {
                label: CutLabel::TiltReplaces,
            },
        };
        Ok(Self::of(a, family, vec![end]))
    }

    /// The lock at the affine chart's pixel `(s, t)` (Part 4, "Setting the lock"): today's lock re-centres `z₀` on the
    /// clicked pixel, so a cut labelled `LockRecentre`.
    pub fn lock(a: &View, s: f64, t: f64) -> Result<Self, PathError> {
        a.plane()?;
        let edits = lock::set(&a.sim.plane, s, t)?;
        Ok(Self::of(a, recentre_cut(), Vec::from(edits)))
    }

    /// The lock at the pixel `(s, t)` of `A`'s window on the nonlinear `chart`, under the link selection `S`, `norm` and
    /// `tol` lookup's (the contract's definition): `z₀`, the lock group and the window in one cut labelled
    /// `LockRecentre`, and what the lock found.
    pub fn lock_on_chart<S, C, N>(
        a: &View,
        chart: &C,
        (s, t): (f64, f64),
        norm: N,
        tol: &Tolerances,
    ) -> Result<(Self, ChartLock), PathError>
    where
        S: Selection,
        S::MassLink: SimplexClamp,
        C: ByKind + FrozenControls,
        f64: From<C::Param>,
        N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
    {
        if a.map != Map::Kernel(KernelMap::of(chart)?) {
            return Err(PathError::WrongMap);
        }
        let (edits, found) =
            lock::set_on_chart::<S, C, N>(chart, &a.sim.chart.window, s, t, norm, tol)?;
        Ok((Self::of(a, recentre_cut(), Vec::from(edits)), found))
    }

    /// The re-centre lock validation offers (R-593): the lock at `nearest`, a cut labelled `LockRecentre`.
    pub fn recentre(a: &View, nearest: &Nearest) -> Self {
        Self::of(a, recentre_cut(), Vec::from(lock::recentre(nearest)))
    }

    /// Snap-back: `z₀ ← z_locked`, by assignment, bit for bit; a cut labelled `SnapBack`.
    pub fn snap_back(a: &View) -> Result<Self, PathError> {
        let e = lock::snap_back(&a.sim)?;
        let label = CutLabel::SnapBack;
        Ok(Self::of(a, Family::Cut { label }, vec![e]))
    }

    /// Unlock: the lock group cleared, `(z₀, q₁, q₂)` kept; a cut labelled `Unlock`.
    pub fn unlock(a: &View) -> Result<Self, PathError> {
        let e = lock::unlock(&a.sim)?;
        let label = CutLabel::Unlock;
        Ok(Self::of(a, Family::Cut { label }, vec![e]))
    }

    /// A cut from `a` to the view `b` labelled `label`: no interior, `B` stored.
    pub fn cut(a: &View, b: View, label: CutLabel) -> Self {
        Segment {
            a: a.clone(),
            b,
            family: Family::Cut { label },
            edits: Vec::new(),
            timing: Timing::default(),
        }
    }

    /// The chart switch to `b` (the chart-space model's § 3, "a jump"): a cut labelled `ChartSwitch`, or
    /// `JumpUnderLock` where `A` is locked, its non-conformance with R-849 stated until stage 2's sliding anchor.
    pub fn chart_switch(a: &View, b: View) -> Self {
        let label = match a.sim.lock.locked {
            true => CutLabel::JumpUnderLock,
            false => CutLabel::ChartSwitch,
        };
        Self::cut(a, b, label)
    }

    /// Save view as chart of a latent view (R-475 S1 to S3): the oblique chart `(z₀, q₁, q₂)`, the stored vectors
    /// exactly, by the kernel's `Saved::latent`; the same pixels, another map.
    pub fn save_view(a: &View) -> Result<Self, PathError> {
        let plane = a.plane()?;
        let saved = Saved::latent(&LatentChart {
            z0: plane.z0,
            q1: plane.q1,
            q2: plane.q2,
        });
        Ok(Self::committed(a, saved))
    }

    /// Save view as chart of a curved view of `parent` (R-475 L5; R-513): the kernel's `Saved::curved` under `tilt`
    /// about `centre`, the targets' spans `span`; refused where the kernel refuses the parent.
    pub fn save_curved<C>(
        a: &View,
        parent: &C,
        tilt: kernel::chart::tilt::TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Result<Self, PathError>
    where
        C: kernel::chart::saved::SaveParent + kernel::chart::saved::ParentParams,
    {
        if a.map != Map::Kernel(KernelMap::of(parent)?) {
            return Err(PathError::WrongMap);
        }
        let saved = Saved::curved(parent, tilt, centre, span)
            .ok_or(PathError::Refused("the kernel refuses this parent"))?;
        Ok(Self::committed(a, saved))
    }

    /// The commit of `a` as the saved chart `saved`.
    fn committed(a: &View, saved: Saved) -> Self {
        Segment {
            a: a.clone(),
            b: View {
                map: Map::Saved(Box::new(saved)),
                sim: a.sim.clone(),
            },
            family: Family::Commit,
            edits: Vec::new(),
            timing: Timing::default(),
        }
    }

    /// The segment a record names, rebuilt from its `A` and its family; its `B` must be the record's, bit for bit, where
    /// the family reaches one, and is the record's where it is a cut or a commit.
    fn rebuilt(a: &View, b: &View, family: &Family) -> Result<Self, PathError> {
        let reached = match family {
            Family::Pan { ds, dt } => Some(Self::pan(a, *ds, *dt)?),
            Family::Zoom { octaves } => Some(Self::zoom(a, *octaves)?),
            Family::Slice { d, t } => Some(Self::slice(a, d, *t)?),
            Family::Control { k, from, to } => {
                (a.sim.plane.z0.get(*k).map(|x| x.to_bits()) == Some(from.to_bits()))
                    .then_some(())
                    .ok_or(PathError::Rebuilt)?;
                Some(Self::control(a, *k, *to)?)
            }
            Family::RawTilt { committed, form } => {
                if !starts_at(committed, a.plane()?, form) {
                    return Err(PathError::Rebuilt);
                }
                let edit = raw_tilt_at(committed, a.plane()?, form, 1.0)?;
                Some(Self::of(a, family.clone(), vec![edit]))
            }
            _ => None,
        };
        match reached {
            Some(seg) if seg.b.bytes() == b.bytes() && &seg.family == family => Ok(seg),
            Some(_) => Err(PathError::Rebuilt),
            None => Ok(Segment {
                a: a.clone(),
                b: b.clone(),
                family: family.clone(),
                edits: Vec::new(),
                timing: Timing::default(),
            }),
        }
    }
}

/// The lock's re-centring cut.
fn recentre_cut() -> Family {
    Family::Cut {
        label: CutLabel::LockRecentre,
    }
}

/// Whether the raw tilt of `form` from `committed` starts at the plane `a`: its tilt at `from` is `a`'s basis to
/// rounding. Where it is not, raw tilts replace rather than compose and the direct operation jumps: a cut.
fn starts_at(committed: &Plane, a: &Plane, form: &TiltForm) -> bool {
    match raw_tilt_at(committed, a, form, 0.0).map(|e| e.edit) {
        Ok(Edit::Sim(SimField::Basis { q1, q2 })) => {
            rounding_equal(&q1, &a.q1) && rounding_equal(&q2, &a.q2)
        }
        _ => false,
    }
}

/// The raw tilt of `form` at path parameter `s`: `τ = from + s·(to − from)`, through `Constructor::tilt` or
/// `Constructor::tilt_pair` from `committed`, at the lengths `current` has. At `s = 1` the angle is `to` itself.
fn raw_tilt_at(
    committed: &Plane,
    current: &Plane,
    form: &TiltForm,
    s: f64,
) -> Result<SetField, PathError> {
    let constructor = Constructor::new(committed.clone());
    let lerp = |from: f64, to: f64| match s {
        0.0 => from,
        1.0 => to,
        s => from + s * (to - from),
    };
    Ok(match form {
        TiltForm::Single {
            which,
            from,
            to,
            target,
        } => constructor.tilt(current, *which, lerp(*from, *to), target)?,
        TiltForm::Pair { from, to, targets } => constructor.tilt_pair(
            current,
            (lerp(from[0], to[0]), &targets[0]),
            (lerp(from[1], to[1]), &targets[1]),
        )?,
    })
}

// ── The path ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// A chain of segments, each join bit for bit (Theorem 4): the shared view is one frame, the end of one segment and
/// the start of the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Path(Vec<Segment>);

impl Path {
    /// The path of `segments`; [`PathError::Join`] where one's `B` is not the next one's `A` bit for bit, or there is
    /// none.
    pub fn new(segments: Vec<Segment>) -> Result<Self, PathError> {
        let joined = segments
            .windows(2)
            .all(|w| w[0].b.bytes() == w[1].a.bytes());
        match joined && !segments.is_empty() {
            true => Ok(Path(segments)),
            false => Err(PathError::Join),
        }
    }

    /// The segments.
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// The frame shown at playback fraction `u` of segment `k`, under that segment's own timing. A frame is addressed
    /// by its segment and its place in it, as export contract Part 3's frame grid does, never by one fraction of the
    /// whole path: `k/n` is not exact in f64 for most `n`, and keyframe `k`'s frame would then come from segment
    /// `k − 1`'s arithmetic. Keyframe `k < n` is `(k, 0)`, the last keyframe `(n − 1, 1)`.
    pub fn at(&self, k: usize, u: f64) -> Result<View, PathError> {
        self.0.get(k).ok_or(PathError::Parameter)?.at(u)
    }

    /// Keyframe `k`'s own frame, the stored view: segment `k`'s `A`, or for `k = n` the last segment's `B`.
    pub fn keyframe(&self, k: usize) -> Result<View, PathError> {
        let n = self.0.len();
        match k.cmp(&n) {
            std::cmp::Ordering::Less => self.at(k, 0.0),
            std::cmp::Ordering::Equal => self.at(n - 1, 1.0),
            std::cmp::Ordering::Greater => Err(PathError::Parameter),
        }
    }
}
