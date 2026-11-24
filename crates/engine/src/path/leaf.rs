//! One leaf per operation: the entry points every route that edits a view calls, the engine's and the gui's alike.
//! Each builds the operation's segment by its family's constructor and passes it through the stop ([`super::stop`])
//! before anything is recorded, so the edits a leaf returns, which the history holds, are the value the stop accepted.
//!
//! | Operation | Leaf | Family | Through the stop as |
//! |---|---|---|---|
//! | Pan | [`pan`] | `Pan`, on the plane or a kernel map's window | a path |
//! | Zoom | [`zoom`] | `Zoom` in octaves, or `ZoomFactor` by a ratio of scales, on the plane or a kernel map's window | a path |
//! | Slice | [`slice`] | `Slice` | a path |
//! | A slider's move | [`control`] | `Control` | a path |
//! | Tilt | [`tilt`] | the active tilt map (Tip as sheet or Lean each axis) on a latent view ([`OnLatent`]) or a curved one ([`OnCurved`]), one target or two | a path |
//! | Lock | [`lock`] | `Cut` `Lock`, or `LockRecentre` with the centre-on-pin setting on | a lock: taken or refused |
//! | Centre on pin | [`centre_on_pin`] | `Cut` `CentreOnPin` | a lock |
//! | Snap-back | [`snap_back`] | `Cut` `SnapBack` | a lock |
//! | Recentre | [`recentre`] | `Cut` `LockRecentre` | a lock |
//! | Unlock | [`unlock`] | `Cut` `Unlock` | a path (it moves no number) |
//! | A jump: chart switch, Home, a preset, a steer | [`jump`] | `Cut` with its label | a lock on `B` |
//!
//! A path whose requested value fails stops at its last valid value: the segment is [`Family::Stopped`], its `B` that
//! value, and the leaf reports the check ([`Leaf::stopped`]). A lock whose value fails is refused: `B` is `A`, with no
//! edits. An edit the stop takes whole is its family's segment, bit for bit.
//!
//! A drag sends one leaf per frame, each from the view where it began; [`Emitted`] keeps the last view it sent, so a
//! drag held past the stop sends nothing new and one brought back inside sends the value.

use std::marker::PhantomData;

use kernel::chart::nav::{Check, Edit};
use kernel::chart::tilt::TiltSpec;
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;

use super::stop::{Gate, Numbers};
use super::{ChartLock, CutLabel, LatentTilt, PathError, Segment, SliderSpan, View};
use crate::contract::set_field::SetField;
use crate::contract::sim_config::Latent;
use crate::lock::{FrozenControls, Nearest};
use crate::lookup::Tolerances;

/// What a leaf did: its segment, the check the stop named where it stopped, and what a lock on a nonlinear chart found.
#[derive(Clone, Debug, PartialEq)]
pub struct Leaf {
    segment: Segment,
    stopped: Option<Check>,
    found: Option<ChartLock>,
}

impl Leaf {
    /// The segment, `B` the value accepted.
    pub fn segment(&self) -> &Segment {
        &self.segment
    }

    /// The view accepted.
    pub fn b(&self) -> &View {
        self.segment.b()
    }

    /// The edits to record, one command: the accepted value's. None where the stop refused the edit.
    pub fn edits(&self) -> &[SetField] {
        self.segment.edits()
    }

    /// The check that stopped the edit, its layer and its constraint ([`Check::layer`]); none where it was taken whole.
    pub fn stopped(&self) -> Option<Check> {
        self.stopped
    }

    /// What a lock on a nonlinear chart found.
    pub fn found(&self) -> Option<&ChartLock> {
        self.found.as_ref()
    }
}

/// `requested` through the stop as `kind`: taken whole where its `B` passes, else stopped (a path) or refused (a lock).
fn through(requested: Segment, gate: &dyn Gate, kind: Kind) -> Leaf {
    let (from, to) = (Numbers::of(requested.a()), Numbers::of(requested.b()));
    let edit = match kind {
        Kind::Path => Edit::Path { from, to },
        Kind::Lock => Edit::Lock { from, to },
    };
    let stop = gate.stop(requested.b(), edit);
    let segment = match stop.stopped {
        None => requested,
        Some(_) => Segment::stopped(&requested, stop.lambda),
    };
    Leaf {
        segment,
        stopped: stop.stopped,
        found: None,
    }
}

/// How an edit passes the stop (the kernel's `Edit`).
#[derive(Clone, Copy)]
enum Kind {
    /// Along the straight line to its value, stopped at the last valid one.
    Path,
    /// To its value or nowhere.
    Lock,
}

// ── Navigation ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Pan by `(ds, dt)` of the window's `[0,1]²` addresses ([`Segment::pan`]), through the stop.
pub fn pan(a: &View, ds: f64, dt: f64, gate: &dyn Gate) -> Result<Leaf, PathError> {
    Ok(through(Segment::pan(a, ds, dt)?, gate, Kind::Path))
}

/// How much a zoom zooms: in octaves (the wheel, + / −), or by a ratio of scales `σ′/σ` (the zoom row).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scale {
    /// `2^−octaves`, log-stepped ([`Segment::zoom`]).
    Octaves(f64),
    /// The factor itself ([`Segment::zoom_factor`]).
    Factor(f64),
}

/// Zoom by `scale` about the pin, through the stop: one zoom leaf over both of the zoom's forms, each the same
/// arithmetic, `q` times one factor.
pub fn zoom(a: &View, scale: Scale, gate: &dyn Gate) -> Result<Leaf, PathError> {
    let seg = match scale {
        Scale::Octaves(octaves) => Segment::zoom(a, octaves)?,
        Scale::Factor(k) => Segment::zoom_factor(a, k)?,
    };
    Ok(through(seg, gate, Kind::Path))
}

/// Slice by `t` along the hidden direction `d` ([`Segment::slice`]), through the stop.
pub fn slice(a: &View, d: &Latent, t: f64, gate: &dyn Gate) -> Result<Leaf, PathError> {
    Ok(through(Segment::slice(a, d, t)?, gate, Kind::Path))
}

/// A slider's move of the control `k` to `value` ([`Segment::control`]), through the stop.
pub fn control(a: &View, k: usize, value: f64, gate: &dyn Gate) -> Result<Leaf, PathError> {
    Ok(through(Segment::control(a, k, value)?, gate, Kind::Path))
}

/// What a tilt turns: a latent view's frame or a curved view's chart. One tilt leaf over both, either tilt map (Tip as
/// sheet or Lean each axis) with one target or two read from the [`TiltSpec`].
pub trait TiltOn {
    /// The tilt of `a` to `spec`.
    fn segment(&self, a: &View, spec: &TiltSpec) -> Result<Segment, PathError>;
}

/// A latent view's tilt: its frame and angles at `A` ([`LatentTilt`]) and the rotation `γ` at `B`, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OnLatent {
    /// The frame and the angles at `A`.
    pub from: LatentTilt,
    /// `γ` at `B`, degrees.
    pub gamma: f64,
}

impl TiltOn for OnLatent {
    fn segment(&self, a: &View, spec: &TiltSpec) -> Result<Segment, PathError> {
        Segment::latent_tilt(a, &self.from, spec, self.gamma)
    }
}

/// A curved view's tilt on its chart `C` ([`Segment::curved_tilt`]), or a saved curved chart's on its parent.
#[derive(Clone, Copy, Debug)]
pub struct OnCurved<'c, C>(pub &'c C);

impl<C: SliderSpan> TiltOn for OnCurved<'_, C> {
    fn segment(&self, a: &View, spec: &TiltSpec) -> Result<Segment, PathError> {
        Segment::curved_tilt(a, self.0, spec)
    }
}

/// The tilt of `a` to `spec` on `on`, through the stop.
pub fn tilt(
    a: &View,
    on: &dyn TiltOn,
    spec: &TiltSpec,
    gate: &dyn Gate,
) -> Result<Leaf, PathError> {
    Ok(through(on.segment(a, spec)?, gate, Kind::Path))
}

// ── The lock ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The centre-on-pin setting (render_gui_spec §G4, off by default): whether a lock also brings its pixel to the centre.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CentreOnPin {
    /// The lock alone: the view does not move.
    #[default]
    Off,
    /// The lock and centre on pin, one command.
    On,
}

/// Where a lock is placed: on the affine chart ([`Affine`]) or a nonlinear one ([`OnChart`]). One lock leaf over both,
/// each window shape its own evaluator.
pub trait Locate {
    /// The lock at the pixel `at` of `a`, with the setting `setting`, and what it found on a nonlinear chart.
    fn lock(
        &self,
        a: &View,
        at: (f64, f64),
        setting: CentreOnPin,
    ) -> Result<(Segment, Option<ChartLock>), PathError>;

    /// Centre on pin.
    fn centre_on_pin(&self, a: &View) -> Result<Segment, PathError>;
}

/// The affine chart's lock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Affine;

impl Locate for Affine {
    fn lock(
        &self,
        a: &View,
        (s, t): (f64, f64),
        setting: CentreOnPin,
    ) -> Result<(Segment, Option<ChartLock>), PathError> {
        let seg = match setting {
            CentreOnPin::Off => Segment::lock(a, s, t)?,
            CentreOnPin::On => Segment::lock_centred(a, s, t)?,
        };
        Ok((seg, None))
    }

    fn centre_on_pin(&self, a: &View) -> Result<Segment, PathError> {
        Segment::centre_on_pin(a)
    }
}

/// A nonlinear chart's lock: the chart, lookup's norm and tolerances, under the link selection `S`.
pub struct OnChart<'c, S, C, N> {
    /// The chart.
    pub chart: &'c C,
    /// Lookup's norm.
    pub norm: N,
    /// Lookup's tolerances.
    pub tol: &'c Tolerances,
    /// The link selection.
    pub selection: PhantomData<S>,
}

impl<S, C, N> Locate for OnChart<'_, S, C, N>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: ByKind + FrozenControls,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64 + Copy,
{
    fn lock(
        &self,
        a: &View,
        at: (f64, f64),
        setting: CentreOnPin,
    ) -> Result<(Segment, Option<ChartLock>), PathError> {
        let (seg, found) = match setting {
            CentreOnPin::Off => {
                Segment::lock_on_chart::<S, C, N>(a, self.chart, at, self.norm, self.tol)?
            }
            CentreOnPin::On => {
                Segment::lock_on_chart_centred::<S, C, N>(a, self.chart, at, self.norm, self.tol)?
            }
        };
        Ok((seg, Some(found)))
    }

    fn centre_on_pin(&self, a: &View) -> Result<Segment, PathError> {
        Segment::centre_on_pin_chart(a, self.chart)
    }
}

/// The lock at the pixel `at` of `a` on `on`, with the centre-on-pin `setting`: through the stop as a lock, taken where
/// its view's centre passes and refused otherwise.
pub fn lock(
    a: &View,
    on: &dyn Locate,
    at: (f64, f64),
    setting: CentreOnPin,
    gate: &dyn Gate,
) -> Result<Leaf, PathError> {
    let (seg, found) = on.lock(a, at, setting)?;
    let mut leaf = through(seg, gate, Kind::Lock);
    if leaf.stopped.is_none() {
        leaf.found = found;
    }
    Ok(leaf)
}

/// Centre on pin, on `on`, through the stop as a lock.
pub fn centre_on_pin(a: &View, on: &dyn Locate, gate: &dyn Gate) -> Result<Leaf, PathError> {
    Ok(through(on.centre_on_pin(a)?, gate, Kind::Lock))
}

/// Snap-back ([`Segment::snap_back`]), through the stop as a lock.
pub fn snap_back(a: &View, gate: &dyn Gate) -> Result<Leaf, PathError> {
    Ok(through(Segment::snap_back(a)?, gate, Kind::Lock))
}

/// The re-centre lock validation offers ([`Segment::recentre`]), through the stop as a lock.
pub fn recentre(a: &View, nearest: &Nearest, gate: &dyn Gate) -> Leaf {
    through(Segment::recentre(a, nearest), gate, Kind::Lock)
}

/// Unlock ([`Segment::unlock`]): it moves no number, so the stop takes it whole wherever `A` passes.
pub fn unlock(a: &View, gate: &dyn Gate) -> Result<Leaf, PathError> {
    Ok(through(Segment::unlock(a)?, gate, Kind::Path))
}

/// A jump from `a` to the view `b` (a chart switch, Home, a reset, a preset, a steer), a cut labelled `label`, or
/// `JumpUnderLock` where `a` is locked and the label is a chart switch: no path, so taken where `b`'s centre passes on
/// `gate`, which reads `b`'s map, and refused otherwise, `B` then `A`.
pub fn jump(a: &View, b: View, label: CutLabel, gate: &dyn Gate) -> Leaf {
    let seg = match label {
        CutLabel::ChartSwitch => Segment::chart_switch(a, b),
        label => Segment::cut(a, b, label),
    };
    match gate.check(seg.b()) {
        None => Leaf {
            segment: seg,
            stopped: None,
            found: None,
        },
        Some(check) => Leaf {
            segment: Segment::stopped(&seg, 0.0),
            stopped: Some(check),
            found: None,
        },
    }
}

// ── A drag ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// What a drag has sent: the last view, by its bytes. Each frame's leaf is made from the view where the drag began; a
/// frame whose accepted view is the one sent last sends nothing, so a drag held past the stop adds nothing to the
/// history and one brought back inside sends the value. A drag's first frame always sends, as today's does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Emitted {
    last: Option<Vec<u8>>,
}

impl Emitted {
    /// The edits to send for `leaf`: its own, or none where its view is the one sent last.
    pub fn send(&mut self, leaf: &Leaf) -> Option<Vec<SetField>> {
        let bytes = leaf.b().bytes();
        if self.last.as_ref() == Some(&bytes) {
            return None;
        }
        self.last = Some(bytes);
        Some(leaf.edits().to_vec())
    }
}
