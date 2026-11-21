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

pub mod curved;
pub mod family;
pub mod lock;
mod record;
mod timing;

use kernel::chart::saved::{Composed, ParentParams, SaveParent, Saved};
use kernel::chart::tilt::{Placement, TiltSpec, Tiltable, Tilted};
use kernel::chart::{Latent as LatentChart, REGISTRY};
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;
use serde::{Deserialize, Serialize};

pub use curved::{held_k, CurvedTilt, Pivot, SliderSpan, LATENT_SPAN, NU0_SPAN};
pub use family::{oblique, tilt_tangent, tilt_vector, Constructor};
pub use lock::{hold_plane, hold_window, ChartLock, Held, Pivoted};
pub use record::{Align, FrameRecord};
pub use timing::{Easing, MonotoneTable, Timing};

use crate::contract::set_field::{Edit, SetField, SimField};
use crate::contract::sim_config::{Latent, Plane, SimConfig, Window};
use crate::lock::{point, FrozenControls, LockError, Nearest};
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

/// The pin (chart_decoder_contract Part 4, "Setting the lock"): the locked pixel, by its screen address in the window's
/// `[0,1]²`, stored (not re-derived), so a slice leaves it where it is on screen, and the point it is held at. Tilt and
/// zoom turn about it.
///
/// The plan is unclear here; chose to hold the pin in the view beside its `SimConfig`, not in `SimConfig`'s lock group,
/// because the sim key must not move with a lock (Part 4, "the lock changes neither") and the lock group's struct is
/// built field by field across the gui; the state store's copy is the gui adapter's (TASK-M8-125).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Pin {
    /// The window's centre, `(½, ½)`, where `z(½, ½) = z₀` whatever the basis: tilt and zoom keep the centre.
    Centre,
    /// An address off the centre, and the point it is held at: tilt and zoom re-solve the centre so this pixel is placed
    /// there again.
    At {
        /// The pixel's address in the window's `[0,1]²`.
        address: [f64; 2],
        /// Where the pixel is held.
        point: PinPoint,
    },
}

/// Where a pin is held: the point its pixel is placed at, recorded when it is set or moved on purpose (a lock, a slice,
/// a slider's move, snap-back) and held through every basis or window edit, so the edits' rounding does not accumulate.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PinPoint {
    /// On the latent map: the latent point, `Φ` at the address.
    Latent(Latent),
    /// On a kernel map: the chart point, `window.at` the address.
    Chart([f64; 2]),
}

impl Pin {
    /// The pin at the address `(s, t)` of `view`, held at the point the view places it: the centre where `2s − 1` and
    /// `2t − 1` are both 0, the kernel's offset of the centre pixel, exactly at `(½, ½)`.
    pub fn at(view: &View, s: f64, t: f64) -> Self {
        let point = match view.window() {
            ViewWindow::Plane(plane) => PinPoint::Latent(point(plane, s, t)),
            ViewWindow::Chart(window) => PinPoint::Chart(window.at(s, t)),
        };
        match (s, t) == (0.5, 0.5) {
            true => Pin::Centre,
            false => Pin::At {
                address: [s, t],
                point,
            },
        }
    }

    /// The pin's address.
    pub fn address(&self) -> [f64; 2] {
        match self {
            Pin::Centre => [0.5, 0.5],
            Pin::At { address, .. } => *address,
        }
    }
}

/// `view`'s pin re-read where an edit moved the pinned state on purpose (a slice, a slider's move): the same address,
/// held at the point the view now places it.
fn reread(view: &View) -> Option<Pin> {
    view.pin.map(|p| {
        let [s, t] = p.address();
        Pin::at(view, s, t)
    })
}

/// A view: a map seen through the window its `SimConfig` holds, the lock's pin, and on a curved map its own tilt and
/// pivot record.
///
/// The plan is unclear here; chose to hold a curved view's tilt in the view beside its `SimConfig`, as the pin is, not
/// in a `SimConfig` group, because today's `SimConfig` has no group for a kernel map's tilt and adding one would move
/// every view's sim key; the tilt's sim-key wiring is the gui adapter's (TASK-M2-64). Absent, a view's bytes are
/// stage 1's.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// The map.
    pub map: Map,
    /// The whole `SimConfig`: the window, the lock and every other group, as today.
    pub sim: SimConfig,
    /// The pin while locked, `None` while free.
    pub pin: Option<Pin>,
    /// A curved view's own tilt ([`Family::CurvedTilt`]); on a saved curved chart the new tilt `R′`. `None` untilted.
    pub tilt: Option<CurvedTilt>,
    /// R-899's pivot record, set by a lock on a tilted curved view; `None` elsewhere.
    pub pivot: Option<Pivot>,
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
            pin: None,
            tilt: None,
            pivot: None,
        }
    }

    /// A view of the registry chart `chart`.
    pub fn kernel<C: kernel::chart::Chart>(chart: &C, sim: SimConfig) -> Result<Self, PathError> {
        Ok(View {
            map: Map::Kernel(KernelMap::of(chart)?),
            sim,
            pin: None,
            tilt: None,
            pivot: None,
        })
    }

    /// A curved chart's tilt `spec` of this view, the targets' slider spans `span`, turned about the pin: where the view
    /// holds a pivot record, the pixel at its stored address placed at its stored point with its stored offsets `d0`;
    /// else the pivot at the pinned pixel, `window.at(pin)`, placed at the pin's held chart point, its hidden offsets zero
    /// (`[−0, −0]`); or about the window's centre while free or pinned there, the kernel's tilt bit for bit.
    /// [`PathError::WrongMap`] off a kernel map.
    pub fn pivoted(
        &self,
        spec: &kernel::chart::tilt::TiltSpec,
        span: [f64; 2],
    ) -> Result<Pivoted, PathError> {
        let ViewWindow::Chart(window) = self.window() else {
            return Err(PathError::WrongMap);
        };
        if let Some(Pivot {
            address: [s, t],
            point,
            d0,
            ..
        }) = self.pivot
        {
            return Ok(Pivoted::new(spec, window.at(s, t), point, d0, span));
        }
        let (at, point) = match self.pin {
            Some(Pin::At {
                address: [s, t],
                point: PinPoint::Chart(point),
            }) => (window.at(s, t), point),
            _ => (window.centre, window.centre),
        };
        Ok(Pivoted::new(spec, at, point, [-0.0, -0.0], span))
    }

    /// The view's own placement on a kernel map: its held tilt about its pivot ([`View::pivoted`]), the chart's own
    /// where it holds none.
    pub fn placement(&self) -> Result<Pivoted, PathError> {
        let tilt = self.tilt.unwrap_or_else(untilted);
        self.pivoted(&tilt.spec(), tilt.span)
    }

    /// The kernel map `chart` as this view shows it: `chart` under the view's placement ([`View::placement`]);
    /// [`PathError::WrongMap`] where the view is not of `chart`, [`PathError::Refused`] where a target is not hidden.
    pub fn tilted<C>(&self, chart: C) -> Result<Tilted<C, f64, Pivoted>, PathError>
    where
        C: Tiltable<Param = f64>,
    {
        if self.map != Map::Kernel(KernelMap::of(&chart)?) {
            return Err(PathError::WrongMap);
        }
        Tilted::placed(chart, self.placement()?).ok_or(PathError::Refused(TARGET_REFUSED))
    }

    /// A saved curved chart as this view shows it (R-475, R-513): reopened from its definition alone on its parent `C`,
    /// under the view's own tilt `R′` about the window's centre (the saved `R` after it, [`Composed`]); at `R′ = 0` the
    /// view it was saved from, bit for bit. [`PathError::WrongMap`] off a saved curved chart or where `C` is not its
    /// parent; [`PathError::NotBuilt`] with the pin off the centre, about which `Composed` does not turn.
    pub fn reopened<C>(&self) -> Result<Tilted<C, f64, Composed<f64>>, PathError>
    where
        C: Tiltable<Param = f64> + ParentParams,
    {
        let Map::Saved(saved) = &self.map else {
            return Err(PathError::WrongMap);
        };
        let Saved::Curved(curved) = **saved else {
            return Err(PathError::WrongMap);
        };
        if matches!(self.pin, Some(Pin::At { .. })) {
            return Err(PathError::NotBuilt(
                "a saved curved chart's tilt about a pin off its centre",
            ));
        }
        curved.parent::<C>().ok_or(PathError::WrongMap)?;
        let tilt = self.tilt.unwrap_or_else(untilted);
        curved
            .open::<C>(&tilt.spec(), self.sim.chart.window.centre, tilt.span)
            .ok_or(PathError::Refused(TARGET_REFUSED))
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

    /// The plane a window edit acts on: the latent map's; [`PathError::WrongMap`] on a kernel map, whose pan and slice
    /// are the GUI's until TASK-M2-64 (its zoom has its own evaluator, [`family::zoom_window`]); and [`saved_refused`]
    /// on a saved chart.
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

/// A curved view with no tilt of its own: every angle 0, so no tilt path is taken; its targets name no coordinate.
fn untilted() -> CurvedTilt {
    CurvedTilt {
        tau1: 0.0,
        tau2: 0.0,
        lean: false,
        x: u32::MAX,
        y: u32::MAX,
        span: [LATENT_SPAN, LATENT_SPAN],
    }
}

/// The kernel's refusal of a tilt whose target is not one of the chart's hidden coordinates.
const TARGET_REFUSED: &str = "a target that is not one of the chart's hidden coordinates";

/// An edit of a saved chart's view. The plan is unclear here; chose to refuse it in stage 1, because a saved chart holds
/// its definition by value (an oblique one its `z₀`, `q₁`, `q₂`) beside the window in `SimConfig`, so an edit of the
/// window would leave the two disagreeing. A saved curved chart's new tilt is built ([`Segment::curved_tilt`], composed
/// in the kernel's `Composed`); editing its window and saving it again are TASK-M2-55's.
fn saved_refused() -> PathError {
    PathError::NotBuilt("an edit of a saved chart's view (TASK-M2-55)")
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
    /// The lock set at a pixel, on the affine or a nonlinear chart: the lock group and the pin, the view unchanged
    /// (chart_decoder_contract Part 4, "Setting the lock": locking does not move the view).
    Lock,
    /// A lock that moves the view to its point: lock validation's re-centre at the nearest feasible point, and a lock
    /// with the centre-on-pin setting on, the lock and centre on pin in one command.
    LockRecentre,
    /// Centre on pin: the pinned pixel brought to the centre, a navigation edit by assignment.
    CentreOnPin,
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
    /// R-474's Tip as sheet and Lean each axis on a latent view: not built. On a curved view it is
    /// [`Family::CurvedTilt`].
    R474Tilt,
    /// A curved view's own tilt (R-473 T2, R-474 on a curved chart; R-513 on a saved one), `(τ₁, τ₂)` lerped from `from`
    /// to `to`, the map, targets and spans `tilt`'s, the same at both ends; it turns about the view's pivot
    /// ([`View::pivoted`]).
    CurvedTilt {
        /// `(τ₁, τ₂)` at `A`.
        from: [f64; 2],
        /// `(τ₁, τ₂)` at `B`.
        to: [f64; 2],
        /// The map, the targets and the spans.
        tilt: CurvedTilt,
    },
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
            ..a.clone()
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
        let mut pin = a.pin;
        let mut tilt = a.tilt;
        let edits = match &self.family {
            Family::Pan { ds, dt } => {
                pin = panned(a, s * ds, s * dt);
                vec![family::pan(a.plane()?, s * ds, s * dt)?]
            }
            Family::Zoom { octaves } => zoomed(a, s * octaves)?,
            Family::Slice { d, t } => vec![family::slice(a.plane()?, d, s * t)?],
            Family::Control { k, from, to } => {
                vec![family::control(&a.sim, *k, from + s * (to - from))?]
            }
            Family::RawTilt { committed, form } => {
                about_pin(a, raw_tilt_at(committed, a.plane()?, form, s)?)
            }
            Family::CurvedTilt { from, to, tilt: t } => {
                let lerp = |i: usize| from[i] + s * (to[i] - from[i]);
                tilt = Some(t.at([lerp(0), lerp(1)]));
                Vec::new()
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
        let view = View {
            map: a.map.clone(),
            sim: apply(&a.sim, &edits),
            pin,
            tilt,
            pivot: a.pivot,
        };
        Ok(match self.family {
            Family::Slice { .. } | Family::Control { .. } => View {
                pin: reread(&view),
                ..view
            },
            _ => view,
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
    /// The pin moves with the content: it stays on the pixel it marked.
    pub fn pan(a: &View, ds: f64, dt: f64) -> Result<Self, PathError> {
        let e = family::pan(a.plane()?, ds, dt)?;
        let seg = Self::of(a, Family::Pan { ds, dt }, vec![e]);
        Ok(seg.pinned(panned(a, ds, dt)))
    }

    /// Zoom by `octaves` about the pin, or the centre while free (Part 4, "Zoom"; R-83): on the latent map
    /// `q₁, q₂ ×= 2^−octaves`, on a kernel map the window's half-widths, each by one common factor; where the pin is off
    /// the centre, the centre re-solved so the pinned pixel keeps its point ([`hold_plane`], [`hold_window`]).
    pub fn zoom(a: &View, octaves: f64) -> Result<Self, PathError> {
        let edits = zoomed(a, octaves)?;
        Ok(Self::of(a, Family::Zoom { octaves }, edits))
    }

    /// `B`'s pin set to `pin`.
    fn pinned(mut self, pin: Option<Pin>) -> Self {
        self.b.pin = pin;
        self
    }

    /// Slice by `t` along the hidden direction `d` (Part 4, "Slice"): `z₀ += t·off_plane(d)`.
    /// The pin stays at its address, held at the point the slice moves it to.
    pub fn slice(a: &View, d: &Latent, t: f64) -> Result<Self, PathError> {
        let e = family::slice(a.plane()?, d, t)?;
        let seg = Self::of(a, Family::Slice { d: *d, t }, vec![e]);
        let pin = reread(&seg.b);
        Ok(seg.pinned(pin))
    }

    /// A slider's move of the control `k` to `value` (Part 4, "Sliders are re-based"): `z₀[k] ← value`, from `A`'s own
    /// value. While locked it is an excursion from the anchor, which it keeps.
    pub fn control(a: &View, k: usize, value: f64) -> Result<Self, PathError> {
        if let Map::Saved(_) = a.map {
            return Err(saved_refused());
        }
        let e = family::control(&a.sim, k, value)?;
        let from = a.sim.plane.z0[k];
        let seg = Self::of(a, Family::Control { k, from, to: value }, vec![e]);
        let pin = reread(&seg.b);
        Ok(seg.pinned(pin))
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
        let end = about_pin(a, raw_tilt_at(&committed, plane, &form, 1.0)?);
        let family = match starts_at(&committed, plane, &form) {
            true => Family::RawTilt {
                committed: Box::new(committed),
                form,
            },
            false => Family::Cut {
                label: CutLabel::TiltReplaces,
            },
        };
        Ok(Self::of(a, family, end))
    }

    /// The lock at the affine chart's pixel `(s, t)` (Part 4, "Setting the lock"): the lock group, `z_locked` the
    /// kernel's `Φ` there, and the pin at `(s, t)`; `z₀` and the basis are not edited. A cut labelled `Lock`.
    pub fn lock(a: &View, s: f64, t: f64) -> Result<Self, PathError> {
        a.plane()?;
        let e = lock::set(&a.sim.plane, s, t)?;
        let seg = Self::of(a, cut(CutLabel::Lock), vec![e]);
        Ok(seg.pinned(Some(Pin::at(a, s, t))))
    }

    /// The lock at the affine chart's pixel `(s, t)` with the centre-on-pin setting on (render_gui_spec §G4, off by
    /// default): the lock and centre on pin in one command, `z₀ ← z_locked` and the lock group, the pin at the centre.
    /// A cut labelled `LockRecentre`.
    pub fn lock_centred(a: &View, s: f64, t: f64) -> Result<Self, PathError> {
        let locked = Self::lock(a, s, t)?;
        let centred = Self::centre_on_pin(locked.b())?;
        Ok(Self::joined(a, &[locked, centred], CutLabel::LockRecentre))
    }

    /// One command of `parts`, a chain from `a`, `B` the last part's, a cut labelled `label`: their edits in the order a
    /// lock's command sends them, `z₀`, then the lock group, then the window (the order is not read: each edits its own
    /// field).
    fn joined(a: &View, parts: &[Segment], label: CutLabel) -> Self {
        let rank = |e: &SetField| match e.edit {
            Edit::Sim(SimField::Z0(_)) => 0,
            Edit::Sim(SimField::Lock(_)) => 1,
            _ => 2,
        };
        let mut edits: Vec<SetField> = parts.iter().flat_map(|p| p.edits.clone()).collect();
        edits.sort_by_key(rank);
        let b = parts.last().map_or_else(|| a.clone(), |p| p.b.clone());
        Segment {
            a: a.clone(),
            b,
            family: cut(label),
            edits,
            timing: Timing::default(),
        }
    }

    /// Centre on pin (Part 4, "Setting the lock"): a navigation edit that brings the pinned pixel to the centre. On the
    /// latent map `z₀ ← ` the pin's held point, by assignment (`Φ` at the pinned address where nothing has rounded it),
    /// so after snap-back `z₀ ← z_locked`; the pin is then the centre. A cut labelled `CentreOnPin`;
    /// [`LockError::NotLocked`] while free. On a kernel map, see [`Segment::centre_on_pin_chart`].
    pub fn centre_on_pin(a: &View) -> Result<Self, PathError> {
        let pin = a.pin.ok_or(LockError::NotLocked)?;
        let plane = a.plane()?;
        let held = match pin {
            Pin::At {
                point: PinPoint::Latent(z),
                ..
            } => z,
            other => point(plane, other.address()[0], other.address()[1]),
        };
        let e = lock::centre_on_pin(held);
        let seg = Self::of(a, cut(CutLabel::CentreOnPin), vec![e]);
        Ok(seg.pinned(Some(Pin::Centre)))
    }

    /// Centre on pin on the nonlinear `chart`: the window's centre moved to the pin's chart point, and `z₀` the
    /// centre's controls, `z_locked`'s where the chart's axes set them, the chart's frozen ones elsewhere; the pin is then
    /// the centre. A cut labelled `CentreOnPin`.
    pub fn centre_on_pin_chart<C>(a: &View, chart: &C) -> Result<Self, PathError>
    where
        C: kernel::chart::Chart + FrozenControls,
    {
        if a.map != Map::Kernel(KernelMap::of(chart)?) {
            return Err(PathError::WrongMap);
        }
        let pin = a.pin.ok_or(LockError::NotLocked)?;
        let held = match pin {
            Pin::At {
                point: PinPoint::Chart(uv),
                ..
            } => uv,
            other => a
                .sim
                .chart
                .window
                .at(other.address()[0], other.address()[1]),
        };
        let edits = lock::centre_on_pin_chart(&a.sim, chart.frozen(), chart.set_by_axes(), held);
        let seg = Self::of(a, cut(CutLabel::CentreOnPin), Vec::from(edits));
        Ok(seg.pinned(Some(Pin::Centre)))
    }

    /// The lock at the pixel `(s, t)` of `A`'s window on the nonlinear `chart`, under the link selection `S`, `norm` and
    /// `tol` lookup's (the contract's definition): the lock group alone, the pin at `(s, t)`, the window not re-centred,
    /// so the pinned pixel decodes the clicked state bit for bit. A cut labelled `Lock`, and what the lock found. On a
    /// tilted curved view the lock is placed at the centre only, where the tilt is anchor-preserving, and the view gains
    /// R-899's pivot record there ([`Pivot`]); off the centre it is the untilt's (R-899 (a1)), not built.
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
        let pivot = pivot_at(a, (s, t))?;
        let (e, found) =
            lock::set_on_chart::<S, C, N>(chart, &a.sim.chart.window, s, t, norm, tol)?;
        let mut seg = Self::of(a, cut(CutLabel::Lock), vec![e]).pinned(Some(Pin::at(a, s, t)));
        seg.b.pivot = pivot;
        Ok((seg, found))
    }

    /// The lock on the nonlinear `chart` with the centre-on-pin setting on: the lock and centre on pin in one command,
    /// `z₀`, the lock group and the window, the pin at the centre. A cut labelled `LockRecentre`.
    pub fn lock_on_chart_centred<S, C, N>(
        a: &View,
        chart: &C,
        at: (f64, f64),
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
        let (locked, found) = Self::lock_on_chart::<S, C, N>(a, chart, at, norm, tol)?;
        let centred = Self::centre_on_pin_chart(locked.b(), chart)?;
        let seg = Self::joined(a, &[locked, centred], CutLabel::LockRecentre);
        Ok((seg, found))
    }

    /// The re-centre lock validation offers (R-593): the lock at `nearest`, `z₀` moved there, the pin at the centre; a
    /// cut labelled `LockRecentre`.
    pub fn recentre(a: &View, nearest: &Nearest) -> Self {
        let seg = Self::of(
            a,
            cut(CutLabel::LockRecentre),
            Vec::from(lock::recentre(nearest)),
        );
        seg.pinned(Some(Pin::Centre))
    }

    /// Snap-back (Part 4, "The lock's construction, defined"): the slice undone, the pin where it is on screen. On the
    /// latent map with the pin off the centre, `z₀` re-solved for the basis so the pinned pixel's `Φ` is `z_locked`
    /// ([`hold_plane`]); with the pin at the centre, and on a kernel map, whose hidden coordinates are `z₀`'s and whose
    /// window the lock did not move, `z₀ ← z_locked` by assignment. A cut labelled `SnapBack`.
    pub fn snap_back(a: &View) -> Result<Self, PathError> {
        if !a.sim.lock.locked {
            return Err(LockError::NotLocked.into());
        }
        let anchor = a.sim.lock.z_locked;
        let (z0, pin) = match (a.window(), a.pin) {
            (ViewWindow::Plane(plane), Some(Pin::At { address, .. })) => {
                let point = PinPoint::Latent(anchor);
                (
                    hold_plane(plane, address, &anchor).0,
                    Some(Pin::At { address, point }),
                )
            }
            (_, pin) => (anchor, pin),
        };
        let e = family::edit(SimField::Z0(z0));
        Ok(Self::of(a, cut(CutLabel::SnapBack), vec![e]).pinned(pin))
    }

    /// Unlock: the lock group cleared, `(z₀, q₁, q₂)` kept, the pin gone; a cut labelled `Unlock`.
    pub fn unlock(a: &View) -> Result<Self, PathError> {
        let e = lock::unlock(&a.sim)?;
        let mut seg = Self::of(a, cut(CutLabel::Unlock), vec![e]).pinned(None);
        seg.b.pivot = None;
        Ok(seg)
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

    /// Save view as chart of the curved view `a` of `parent` (R-475 S2, L5): its parent, its slice centre and its own tilt
    /// (the angles, map, targets and spans), read from the view. The slice centre `s` is the point the view's tilt turns
    /// about, the window's centre, or the pivot record's point where its pixel is placed there with no hidden offset;
    /// the saved chart places the pixel at window point `u` at `s + R·(u − s)`, the view's own placement, so reopened at
    /// zero tilt it decodes the view bit for bit. A view pivoted elsewhere (a pivot record with an offset) is the
    /// untilt's (R-899 (a1)): not built.
    pub fn save_view_curved<C>(a: &View, parent: &C) -> Result<Self, PathError>
    where
        C: SaveParent + ParentParams,
    {
        if a.map != Map::Kernel(KernelMap::of(parent)?) {
            return Err(PathError::WrongMap);
        }
        let tilt = a.tilt.unwrap_or_else(untilted);
        let (at, point, d0) = a.placement()?.pivot();
        let about_own_point = at.map(f64::to_bits) == point.map(f64::to_bits);
        if !about_own_point || d0.iter().any(|d| *d != 0.0) {
            return Err(PathError::NotBuilt(
                "Save view as chart of a view pivoted off its own point: the untilt (R-899 (a1))",
            ));
        }
        Self::save_curved(a, parent, tilt.spec(), at, tilt.span)
    }

    /// The tilt `spec` of the curved view `a` of `chart`, or on a saved curved chart whose parent is `chart` its new tilt
    /// `R′` (R-473 T2, R-474, R-513): the view's tilt set to `spec` at the targets' slider spans, R-920's, read from the
    /// chart ([`SliderSpan`]), never from the view. A curved tilt family from the view's own tilt where only the angles
    /// change; otherwise a cut labelled `TiltReplaces`. The tilt turns about the view's pivot ([`View::pivoted`]). A
    /// non-finite angle is refused ([`NavError::NonFinite`]).
    pub fn curved_tilt<C>(a: &View, chart: &C, spec: &TiltSpec) -> Result<Self, PathError>
    where
        C: SliderSpan,
    {
        crate::nav::finite(&[spec.tau1, spec.tau2])?;
        let index = KernelMap::of(chart)?;
        let named = match &a.map {
            Map::Kernel(k) => *k == index,
            Map::Saved(saved) => matches!(**saved, Saved::Curved(c) if c.parent == index.registry),
            Map::Latent => false,
        };
        if !named {
            return Err(PathError::WrongMap);
        }
        let span = |k: u32| {
            chart
                .slider_span(k)
                .ok_or(PathError::Refused(TARGET_REFUSED))
        };
        let to = CurvedTilt::of(spec, [span(spec.x)?, span(spec.y)?]);
        let from = a.tilt.unwrap_or(to.at([0.0, 0.0]));
        let family = match from.same_axes(&to) {
            true => Family::CurvedTilt {
                from: [from.tau1, from.tau2],
                to: [to.tau1, to.tau2],
                tilt: to,
            },
            false => cut(CutLabel::TiltReplaces),
        };
        let mut seg = Self::of(a, family, Vec::new());
        seg.b.tilt = Some(to);
        Ok(seg)
    }

    /// The commit of `a` as the saved chart `saved`.
    fn committed(a: &View, saved: Saved) -> Self {
        Segment {
            a: a.clone(),
            b: View {
                map: Map::Saved(Box::new(saved)),
                sim: a.sim.clone(),
                pin: a.pin,
                tilt: None,
                pivot: a.pivot,
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
                let edits = about_pin(a, raw_tilt_at(committed, a.plane()?, form, 1.0)?);
                Some(Self::of(a, family.clone(), edits))
            }
            Family::CurvedTilt { from, to, tilt } => {
                let start = a.tilt.unwrap_or(tilt.at([0.0, 0.0]));
                let starts = start.same_axes(tilt)
                    && [start.tau1, start.tau2].map(f64::to_bits) == from.map(f64::to_bits);
                if !starts {
                    return Err(PathError::Rebuilt);
                }
                let mut seg = Self::of(a, family.clone(), Vec::new());
                seg.b.tilt = Some(tilt.at(*to));
                Some(seg)
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

/// R-899's pivot record of a lock at the address `(s, t)` of `a`: on a tilted curved view, the pin's screen position and
/// the pinned state as the view places it there, its chart point and hidden offsets; `None` on any other view, whose
/// bytes stay stage 2's. A lock off the centre of a tilted curved view is the untilt's (R-899 (a1)): not built.
fn pivot_at(a: &View, (s, t): (f64, f64)) -> Result<Option<Pivot>, PathError> {
    match a.tilt {
        Some(tilt) if !tilt.spec().untilted() => {}
        _ => return Ok(None),
    }
    if (s, t) != (0.5, 0.5) {
        return Err(PathError::NotBuilt(
            "a lock off the centre of a tilted curved view: the untilt (R-899 (a1))",
        ));
    }
    let [u, v] = a.sim.chart.window.at(s, t);
    let (point, shift) = a.placement()?.place(u, v);
    Ok(Some(Pivot {
        address: [s, t],
        assumed: false,
        point,
        d0: [shift.dx, shift.dy],
    }))
}

/// A cut labelled `label`.
fn cut(label: CutLabel) -> Family {
    Family::Cut { label }
}

/// The pin after a pan by `(ds, dt)`: it moves with the content, so it stays on the pixel it marked. The plan is unclear
/// here (no pan under a lock is specified); chose to move it, because the pin marks a state on screen and a pan moves
/// every state by `(−ds, −dt)` of the window.
fn panned(a: &View, ds: f64, dt: f64) -> Option<Pin> {
    a.pin.map(|p| match p {
        Pin::At {
            address: [s, t],
            point,
        } => Pin::At {
            address: [s - ds, t - dt],
            point,
        },
        Pin::Centre => Pin::At {
            address: [0.5 - ds, 0.5 - dt],
            point: PinPoint::Latent(point(&a.sim.plane, 0.5, 0.5)),
        },
    })
}

/// The edits of a basis `change` to `a`'s plane about its pin: the change alone where the pin is the centre or there is
/// none (the centre pixel is `z₀` whatever the basis), else the change and `z₀` re-solved so the pinned pixel keeps the
/// point it had in `a` ([`hold_plane`]).
fn about_pin(a: &View, change: SetField) -> Vec<SetField> {
    let Some(Pin::At {
        address: st,
        point: held,
    }) = a.pin
    else {
        return vec![change];
    };
    let held = match held {
        PinPoint::Latent(z) => z,
        PinPoint::Chart(_) => point(&a.sim.plane, st[0], st[1]),
    };
    let plane = apply(&a.sim, std::slice::from_ref(&change)).plane;
    let (z0, _) = hold_plane(&plane, st, &held);
    vec![change, family::edit(SimField::Z0(z0))]
}

/// The zoom of `a` by `octaves`, about its pin: by the window shape (one evaluator per shape), the plane's basis
/// ([`family::zoom`]) or a kernel map's window ([`family::zoom_window`]), the centre re-solved where the pin is off it.
fn zoomed(a: &View, octaves: f64) -> Result<Vec<SetField>, PathError> {
    match (&a.map, a.window()) {
        (Map::Saved(_), _) => Err(saved_refused()),
        (_, ViewWindow::Plane(plane)) => Ok(about_pin(a, family::zoom(plane, octaves)?)),
        (_, ViewWindow::Chart(window)) => {
            let mut zoomed = family::zoom_window(window, octaves)?;
            if let Some(Pin::At { address, point }) = a.pin {
                let held = match point {
                    PinPoint::Chart(uv) => uv,
                    PinPoint::Latent(_) => window.at(address[0], address[1]),
                };
                zoomed.centre = hold_window(&zoomed, address, &held).0;
            }
            Ok(vec![family::edit(SimField::Window(zoomed))])
        }
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
