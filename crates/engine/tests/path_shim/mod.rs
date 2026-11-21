//! Today's operations as the tests written before the chart-path object (TASK-M2-61) call them, each now run through
//! the object: a pan, a zoom, a slice, a raw tilt, a lock, a slider's move, snap-back, unlock and the re-centre are each
//! a `engine::path::Segment` built by its constructor, and the edits it records are what these return. The operations'
//! old routes are gone; these keep the earlier tests' signatures so that every one of them now tests the object.

#![allow(dead_code)]

use engine::contract::set_field::SetField;
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use engine::lock::{FrozenControls, LockError, Nearest};
use engine::lookup::{Notices, Tolerances};
use engine::nav::{BasisVector, NavError};
use engine::path::{Constructor, PathError, Segment, View};
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;

#[allow(unused_imports)]
pub use engine::path::{oblique, tilt_tangent, tilt_vector};

/// A latent view of `plane`, every other group the default.
pub fn view_of(plane: &Plane) -> View {
    View::latent(sim_of(plane))
}

/// The `SimConfig` of `plane`, every other group the default.
pub fn sim_of(plane: &Plane) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane: plane.clone(),
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 0.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

/// The family's own error, for the earlier tests' signatures.
fn nav(e: PathError) -> NavError {
    match e {
        PathError::Nav(e) => e,
        other => panic!("a navigation gesture refused by the path, not its formula: {other:?}"),
    }
}

/// The lock's own error, for the earlier tests' signatures.
fn lock(e: PathError) -> LockError {
    match e {
        PathError::Lock(e) => e,
        other => panic!("a lock refused by the path, not its formula: {other:?}"),
    }
}

/// The segment's one edit.
fn one(s: Segment) -> SetField {
    let [e] = <[SetField; 1]>::try_from(s.edits().to_vec()).expect("one edit");
    e
}

/// Pan, through `Segment::pan`.
pub fn pan(plane: &Plane, ds: f64, dt: f64) -> Result<SetField, NavError> {
    Segment::pan(&view_of(plane), ds, dt).map(one).map_err(nav)
}

/// Zoom, through `Segment::zoom`.
pub fn zoom(plane: &Plane, octaves: f64) -> Result<SetField, NavError> {
    Segment::zoom(&view_of(plane), octaves)
        .map(one)
        .map_err(nav)
}

/// Slice, through `Segment::slice`.
pub fn slice(plane: &Plane, d: &Latent, t: f64) -> Result<SetField, NavError> {
    Segment::slice(&view_of(plane), d, t).map(one).map_err(nav)
}

/// The chart constructor, its raw tilts through `Segment::raw_tilt` and `Segment::raw_tilt_pair` from `τ = 0`.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartConstructor(Constructor);

impl ChartConstructor {
    pub fn new(plane: Plane) -> Self {
        Self(Constructor::new(plane))
    }

    pub fn committed(&self) -> &Plane {
        self.0.committed()
    }

    pub fn commit(&mut self, plane: Plane) {
        self.0.commit(plane)
    }

    pub fn tilt(
        &self,
        current: &Plane,
        which: BasisVector,
        tau_deg: f64,
        target: &Latent,
    ) -> Result<SetField, NavError> {
        Segment::raw_tilt(&self.0, &view_of(current), which, (0.0, tau_deg), target)
            .map(one)
            .map_err(nav)
    }

    pub fn tilt_pair(
        &self,
        current: &Plane,
        q1: (f64, &Latent),
        q2: (f64, &Latent),
    ) -> Result<SetField, NavError> {
        Segment::raw_tilt_pair(
            &self.0,
            &view_of(current),
            [0.0, 0.0],
            [q1.0, q2.0],
            [q1.1, q2.1],
        )
        .map(one)
        .map_err(nav)
    }
}

/// The lock on the affine chart with the centre-on-pin setting on, today's lock before the pin (the lock and centre on
/// pin in one command, `z₀ ← z_locked`), through `Segment::lock_centred`: the earlier tests' lock.
pub fn set(plane: &Plane, s: f64, t: f64) -> Result<[SetField; 2], LockError> {
    Segment::lock_centred(&view_of(plane), s, t)
        .map(|seg| <[SetField; 2]>::try_from(seg.edits().to_vec()).expect("two edits"))
        .map_err(lock)
}

/// The lock on the affine chart, the setting off, through `Segment::lock`: the lock group alone, the view unchanged.
pub fn set_pinned(plane: &Plane, s: f64, t: f64) -> Result<[SetField; 1], LockError> {
    Segment::lock(&view_of(plane), s, t)
        .map(|seg| <[SetField; 1]>::try_from(seg.edits().to_vec()).expect("one edit"))
        .map_err(lock)
}

/// A lock on a nonlinear chart: its one command and what it found.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartLock {
    pub edits: [SetField; 3],
    pub window: Window,
    pub ic: Ic<f64>,
    pub notices: Notices,
}

/// The lock on the nonlinear `chart` through `window` with the centre-on-pin setting on, today's lock before the pin,
/// through `Segment::lock_on_chart_centred`: the earlier tests' lock.
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
    let mut sim = sim_of(&Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    });
    sim.chart.window = *window;
    let view = View::kernel(chart, sim).expect("a registry chart");
    Segment::lock_on_chart_centred::<S, C, N>(&view, chart, (s, t), norm, tol)
        .map(|(seg, found)| ChartLock {
            edits: <[SetField; 3]>::try_from(seg.edits().to_vec()).expect("three edits"),
            window: seg.b().sim.chart.window,
            ic: found.ic,
            notices: found.notices,
        })
        .map_err(lock)
}

/// A lock on a nonlinear chart with the setting off: its one edit and what it found.
#[derive(Clone, Debug, PartialEq)]
pub struct PinnedLock {
    pub edits: [SetField; 1],
    pub window: Window,
    pub point: [f64; 2],
    pub ic: Ic<f64>,
    pub notices: Notices,
}

/// The lock on the nonlinear `chart` through `window`, the setting off, through `Segment::lock_on_chart`.
pub fn set_on_chart_pinned<S, C, N>(
    chart: &C,
    window: &Window,
    s: f64,
    t: f64,
    norm: N,
    tol: &Tolerances,
) -> Result<PinnedLock, LockError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: ByKind + FrozenControls,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    let mut sim = sim_of(&Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    });
    sim.chart.window = *window;
    let view = View::kernel(chart, sim).expect("a registry chart");
    Segment::lock_on_chart::<S, C, N>(&view, chart, (s, t), norm, tol)
        .map(|(seg, found)| PinnedLock {
            edits: <[SetField; 1]>::try_from(seg.edits().to_vec()).expect("one edit"),
            window: found.window,
            point: found.point,
            ic: found.ic,
            notices: found.notices,
        })
        .map_err(lock)
}

/// A slider's move, through `Segment::control`.
pub fn move_slider(sim: &SimConfig, k: usize, value: f64) -> Result<SetField, LockError> {
    Segment::control(&View::latent(sim.clone()), k, value)
        .map(one)
        .map_err(lock)
}

/// Snap-back, through `Segment::snap_back`.
pub fn snap_back(sim: &SimConfig) -> Result<SetField, LockError> {
    Segment::snap_back(&View::latent(sim.clone()))
        .map(one)
        .map_err(lock)
}

/// Unlock, through `Segment::unlock`.
pub fn unlock(sim: &SimConfig) -> Result<SetField, LockError> {
    Segment::unlock(&View::latent(sim.clone()))
        .map(one)
        .map_err(lock)
}

/// The re-centre, through `Segment::recentre`.
pub fn recentre(nearest: &Nearest) -> [SetField; 2] {
    let plane = Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    let seg = Segment::recentre(&view_of(&plane), nearest);
    <[SetField; 2]>::try_from(seg.edits().to_vec()).expect("two edits")
}
