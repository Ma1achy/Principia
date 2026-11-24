//! Time as the map's last coordinate (chart_composition § 18, as the 10 November chart decisions amend it): a view's
//! window reads nine coordinates, the latent chart's eight and time last, and the playhead is the window's centre along
//! time. An untilted view shows one time, the playhead, at every pixel; a view tilted toward time shows a time per
//! pixel, and at 90° on one screen axis that axis is the time axis. Each pixel's time maps to the latest boundary at or
//! before it on the occupant's grid, an integer index found by multiplying indices, never by trusting a float division
//! (§ 18.2: `⌊(k·10⁻³)/10⁻³⌋ ≠ k` for 731 of the 65,535 `k`), and the state at a pixel is read through the trajectory
//! interface ([`crate::trajectory`]), so the maths here is tested on exact fixtures and the real integrator plugs in
//! with no change to it.
//!
//! **The window's scale along time** is the home time window: the span of `[0, T]` at zoom 1. A coordinate `x_k` at the
//! screen point `(s, u) ∈ [0, 1]²` is `c_k + (σ_k/2)·((2s − 1)·q₁ₖ + (2u − 1)·q₂ₖ)`, `σ_k` the coordinate's span at zoom
//! 1: 2 for every latent coordinate, so the latent plane reads exactly as it does today (`crate::lock::point`), and `T`
//! for time. So a basis column of unit length along time spans the whole of `[0, T]` across the screen, and the window
//! with its centre at `T/2` and one screen axis's column `e_t` is the time axis at its home extent ([`TimedPlane::home_time_axis`]).
//!
//! **The tilt toward time** ([`TimedPlane::tilt_toward_time`]) turns one screen axis's column in the plane of its latent
//! direction and `e_t`, about the window's centre, keeping its length: the zoom is kept and the playhead is not moved.
//! Its sine and cosine are exact at the quarter turns, decided on the stored degrees as the latent tilt decides its own
//! right angles (`kernel::chart::tilt`), so at 90° the axis's latent column is exactly zero and every pixel of a screen
//! column reads one start, bit for bit: a column is one trajectory, as § 18.1's column march has it.
//!
//! **The target index** ([`TimedPlane::index_at`]): a pixel's time `t` maps to the greatest `k` with `k·Δ ≤ t`, `Δ` the
//! grid's quantum, the trajectory interface's own arithmetic. A sample's stop index `k_stop` is that index narrowed to
//! the record's `u32` ([`TimedPlane::stop_index`]); the integrator takes it as its per-sample stop
//! (`kernel::driver::wrapper::integrate_detecting_to`), and a sample stopped there "reached its time" ([`stop_label`]):
//! `running` there, or, with the stop option off, a live escape label there.
//!
//! **The zoom along time floors at one boundary** ([`TimedPlane::zoom`]): nothing finer than a boundary exists short of
//! a build constant, so a zoom in stops where a pixel's step along time reaches one boundary.

use kernel::decode::Ic;
use kernel::payload::{
    bb_budget_exhausted, sd_state, tm_t_end_step, SimStateBaseOf, STATE_ESCAPE, STATE_RUNNING,
};
use kernel::regularisation::sync::boundary_of_step;

use crate::contract::sim_config::{Latent, Plane};
use crate::trajectory::{last_at_or_before, Provider, Refused, Trajectory};

/// Why a time coordinate, an edit of it or a read of it is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeRefused {
    /// A horizon, playhead, angle, zoom factor or screen point that is not finite, or a horizon or zoom factor not
    /// positive.
    NotFinite,
    /// A pixel's time before `t = 0`: no boundary lies at or before it.
    BeforeStart { t: f64 },
    /// A pixel's time past the horizon `T`: the march never passes the axis's horizon.
    PastHorizon { t: f64, horizon: f64 },
    /// A tilt toward time of a screen axis whose column already has a time component: the tilt is set from the axis's
    /// latent direction, which a column already turned toward time no longer holds alone.
    AlreadyTilted,
    /// A tilt toward time of a screen axis whose latent column is zero: it has no direction to turn from.
    NoDirection,
    /// A stop index past the record's `u32`.
    Index { k: u64 },
    /// The grid or the trajectory refused.
    Grid(Refused),
}

impl From<Refused> for TimeRefused {
    fn from(e: Refused) -> Self {
        TimeRefused::Grid(e)
    }
}

/// A screen axis: the horizontal, along `s` and `q₁`, or the vertical, along `u` and `q₂`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Along `s`, the column `q₁`.
    Horizontal,
    /// Along `u`, the column `q₂`.
    Vertical,
}

impl Axis {
    fn index(self) -> usize {
        match self {
            Axis::Horizontal => 0,
            Axis::Vertical => 1,
        }
    }
}

/// The time coordinate of a window: the horizon `T`, whose `[0, T]` is the home time window and whose span is time's
/// scale `σ_t`; the playhead, the window's centre along time; and the two basis columns' components along time, in
/// units of `σ_t/2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeCoordinate {
    /// The horizon `T`.
    pub horizon: f64,
    /// The playhead: the window's centre along time.
    pub playhead: f64,
    /// `q₁`'s and `q₂`'s components along time.
    pub column: [f64; 2],
}

/// A latent view's window with time as its last coordinate: the latent plane `(z₀, q₁, q₂)` and the time coordinate.
///
/// The plan is unclear here; chose to keep the time coordinate beside the plane, not in `SimConfig`'s plane, because
/// the plane's eight-dimensional vectors are the sim key's bytes and the latent tilt's frame (`k < 8`), and time is off
/// the sim key (§ 18.1: the window along time is a view, `T` the march's top); the view's other off-key parts, the pin
/// and a curved view's tilt, are held beside it the same way (`crate::path::View`).
#[derive(Clone, Debug, PartialEq)]
pub struct TimedPlane {
    /// The latent plane: the eight latent coordinates' centre and columns.
    pub plane: Plane,
    /// Time, the last coordinate.
    pub time: TimeCoordinate,
}

/// A zoom's result: the view, and where the zoom floored at one boundary, the floor's reason.
#[derive(Clone, Debug, PartialEq)]
pub struct Zoomed {
    /// The zoomed view.
    pub view: TimedPlane,
    /// `Some` where the zoom in was stopped at one boundary.
    pub floor: Option<Floor>,
}

/// The zoom along time stopped at one boundary: the quantum it stopped at, and the factor it applied to the floored
/// axis in place of the one asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floor {
    /// The grid's quantum `Δ`.
    pub quantum: f64,
    /// The factor applied, at most 1 and at least the one asked.
    pub applied: f64,
}

impl Floor {
    /// The UI's reason: "one boundary: Δ".
    pub fn reason(&self) -> String {
        format!("one boundary: {}", self.quantum)
    }
}

/// `(sin, cos)` of `degrees`, exact at the quarter turns: 0, ±90, ±180 and ±270 give 0 and ±1 exactly, as the latent
/// tilt decides its right angles on the stored degrees; any other angle through `libm`.
pub fn sin_cos_degrees(degrees: f64) -> (f64, f64) {
    let turn = degrees.rem_euclid(360.0);
    match turn {
        0.0 => (0.0, 1.0),
        90.0 => (1.0, 0.0),
        180.0 => (0.0, -1.0),
        270.0 => (-1.0, 0.0),
        _ => {
            let r = degrees.to_radians();
            (libm::sin(r), libm::cos(r))
        }
    }
}

/// The greatest `k` with `k·Δ ≤ t` on the grid of quantum `quantum`: refused for a `t` not finite, before 0, or a grid
/// the trajectory interface refuses.
pub fn index_at_or_before(quantum: f64, t: f64) -> Result<u64, TimeRefused> {
    if !t.is_finite() {
        return Err(TimeRefused::NotFinite);
    }
    if t < 0.0 {
        return Err(TimeRefused::BeforeStart { t });
    }
    if !(quantum.is_finite() && quantum > 0.0) {
        return Err(TimeRefused::Grid(Refused::Grid));
    }
    Ok(last_at_or_before(quantum, t)?)
}

fn finite(values: &[f64]) -> Result<(), TimeRefused> {
    match values.iter().all(|x| x.is_finite()) {
        true => Ok(()),
        false => Err(TimeRefused::NotFinite),
    }
}

fn norm(v: &Latent) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

impl TimeCoordinate {
    /// The time at the screen point `(s, u)`: the playhead plus `(T/2)·((2s − 1)·q₁ₜ + (2u − 1)·q₂ₜ)`.
    pub fn time_at(&self, s: f64, u: f64) -> f64 {
        self.time_with(s, u, self.horizon)
    }

    /// [`TimeCoordinate::time_at`] at the time scale `sigma`: the coordinate's own, `T`, or a control's.
    pub fn time_with(&self, s: f64, u: f64, sigma: f64) -> f64 {
        let [a, b] = self.column;
        self.playhead + 0.5 * sigma * ((2.0 * s - 1.0) * a + (2.0 * u - 1.0) * b)
    }

    /// The target time at `(s, u)`, checked against the horizon: refused before 0 and past `T`.
    pub fn target(&self, s: f64, u: f64) -> Result<f64, TimeRefused> {
        finite(&[s, u])?;
        let t = self.time_at(s, u);
        if !t.is_finite() {
            return Err(TimeRefused::NotFinite);
        }
        if t < 0.0 {
            return Err(TimeRefused::BeforeStart { t });
        }
        if t > self.horizon {
            return Err(TimeRefused::PastHorizon {
                t,
                horizon: self.horizon,
            });
        }
        Ok(t)
    }

    /// The boundary at `(s, u)` on the grid of quantum `quantum`: the latest at or before its target time.
    pub fn index_at(&self, s: f64, u: f64, quantum: f64) -> Result<u64, TimeRefused> {
        index_at_or_before(quantum, self.target(s, u)?)
    }

    /// The stop index at `(s, u)`: [`TimeCoordinate::index_at`] narrowed to the record's `u32`, on the occupant's own
    /// grid (the macro-step for an unregularised occupant, `Δ_sync` for a regularised one; [`macro_step_for_sync`]
    /// carries a sync index to the wrapper's macro-step count).
    pub fn stop_index(&self, s: f64, u: f64, quantum: f64) -> Result<u32, TimeRefused> {
        let k = self.index_at(s, u, quantum)?;
        u32::try_from(k).map_err(|_| TimeRefused::Index { k })
    }

    /// The playhead's boundary: the centre's, `(½, ½)`.
    pub fn playhead_index(&self, quantum: f64) -> Result<u64, TimeRefused> {
        self.index_at(0.5, 0.5, quantum)
    }

    /// The step in time between neighbouring pixels along `axis` on a screen of `n` pixels along it: `T·|qₜ|/n`.
    pub fn pixel_step(&self, axis: Axis, n: u32) -> f64 {
        self.horizon * self.column[axis.index()].abs() / f64::from(n.max(1))
    }
}

/// The least macro-step count `m` whose march ends on the sync boundary `j` under an occupant that owns its time
/// mapping: macro-step `m` lands on the latest sync boundary at or before `m·dt_macro`
/// (`kernel::regularisation::sync::boundary_of_step`, at the march's f32), so the wrapper's stop for a row that reads
/// boundary `j` is the least `m` with `boundary_of_step(m) ≥ j`. A target time taken to the macro-step grid first can
/// land one sync boundary low: at `T = 13` the playhead `6.5 = 52·Δ_sync` is past `6500·fl(10⁻³)`, so its macro index
/// is 6499, whose boundary is 51.
pub fn macro_step_for_sync(j: u32, dt_macro: f32, delta_sync: f32) -> u32 {
    let at = |m: u32| boundary_of_step::<f32>(m, dt_macro, delta_sync);
    let guess = (f64::from(j) * f64::from(delta_sync) / f64::from(dt_macro)).ceil();
    let mut m = if guess >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        guess as u32
    };
    while m > 0 && at(m - 1) >= j {
        m -= 1;
    }
    while m < u32::MAX && at(m) < j {
        m += 1;
    }
    m
}

impl TimedPlane {
    /// The plane `plane` untilted in time: every pixel at the playhead `playhead`, under the horizon `horizon`.
    pub fn untilted(plane: Plane, horizon: f64, playhead: f64) -> Result<Self, TimeRefused> {
        finite(&[horizon, playhead])?;
        if horizon <= 0.0 {
            return Err(TimeRefused::NotFinite);
        }
        Ok(Self {
            plane,
            time: TimeCoordinate {
                horizon,
                playhead,
                column: [0.0; 2],
            },
        })
    }

    /// The time axis at its home extent on `axis`: the playhead at `T/2`, `axis`'s column `e_t` of unit length, its
    /// latent column zero, and the other axis as it is, so that `axis` spans `[0, T]` exactly.
    pub fn home_time_axis(&self, axis: Axis) -> Self {
        let mut view = self.clone();
        *view.latent_column_mut(axis) = [0.0; 8];
        view.time.column[axis.index()] = 1.0;
        view.time.playhead = 0.5 * self.time.horizon;
        view
    }

    fn latent_column(&self, axis: Axis) -> &Latent {
        match axis {
            Axis::Horizontal => &self.plane.q1,
            Axis::Vertical => &self.plane.q2,
        }
    }

    fn latent_column_mut(&mut self, axis: Axis) -> &mut Latent {
        match axis {
            Axis::Horizontal => &mut self.plane.q1,
            Axis::Vertical => &mut self.plane.q2,
        }
    }

    /// `axis` tilted toward time by `degrees` about the window's centre: its column `q` turned in the plane of `q/|q|`
    /// and `e_t`, `q ↦ cos τ·q + sin τ·|q|·e_t`, its length kept, so the zoom is kept, and the playhead is not moved.
    /// The angle is the axis's from its latent direction, so the axis must not already lean toward time.
    pub fn tilt_toward_time(&self, axis: Axis, degrees: f64) -> Result<Self, TimeRefused> {
        self.tilt_with(axis, degrees, sin_cos_degrees)
    }

    /// [`TimedPlane::tilt_toward_time`] with the sine and cosine `sin_cos`: the tilt's own or a control's.
    pub fn tilt_with(
        &self,
        axis: Axis,
        degrees: f64,
        sin_cos: fn(f64) -> (f64, f64),
    ) -> Result<Self, TimeRefused> {
        finite(&[degrees])?;
        if self.time.column[axis.index()] != 0.0 {
            return Err(TimeRefused::AlreadyTilted);
        }
        let length = norm(self.latent_column(axis));
        if length == 0.0 {
            return Err(TimeRefused::NoDirection);
        }
        let (s, c) = sin_cos(degrees);
        let mut view = self.clone();
        let q = view.latent_column_mut(axis);
        *q = q.map(|x| c * x);
        view.time.column[axis.index()] = s * length;
        Ok(view)
    }

    /// The playhead moved to `t`: the window's centre along time, a pan along time.
    pub fn with_playhead(&self, t: f64) -> Result<Self, TimeRefused> {
        finite(&[t])?;
        let mut view = self.clone();
        view.time.playhead = t;
        Ok(view)
    }

    /// The latent point at `(s, u)`: today's `z₀ + (2s − 1)·q₁ + (2u − 1)·q₂` (`crate::lock::point`), except that a
    /// column whose latent part is zero, an axis turned wholly to time, adds nothing. Added, its `±0` would follow the
    /// sign of `2u − 1` and turn a `−0` in `z₀` to `+0` on one half of the screen, so one screen column would read two
    /// starts (pitfalls § 11); skipped, every pixel along a time axis reads `z₀ + (2s − 1)·q₁` bit for bit.
    pub fn point(&self, s: f64, u: f64) -> Latent {
        let zero = |q: &Latent| q.iter().all(|&x| x == 0.0);
        let p = &self.plane;
        match (zero(&p.q1), zero(&p.q2)) {
            (false, false) => crate::lock::point(p, s, u),
            (true, true) => p.z0,
            (false, true) => {
                let a = s + s - 1.0;
                core::array::from_fn(|i| p.z0[i] + a * p.q1[i])
            }
            (true, false) => {
                let b = u + u - 1.0;
                core::array::from_fn(|i| p.z0[i] + b * p.q2[i])
            }
        }
    }

    /// The time at `(s, u)` ([`TimeCoordinate::time_at`]).
    pub fn time_at(&self, s: f64, u: f64) -> f64 {
        self.time.time_at(s, u)
    }

    /// [`TimeCoordinate::time_with`].
    pub fn time_with(&self, s: f64, u: f64, sigma: f64) -> f64 {
        self.time.time_with(s, u, sigma)
    }

    /// The map's nine coordinates at `(s, u)`: the latent point, then time, the last.
    pub fn coordinates(&self, s: f64, u: f64) -> [f64; 9] {
        let z = self.point(s, u);
        core::array::from_fn(|i| if i < 8 { z[i] } else { self.time_at(s, u) })
    }

    /// [`TimeCoordinate::index_at`].
    pub fn index_at(&self, s: f64, u: f64, quantum: f64) -> Result<u64, TimeRefused> {
        self.time.index_at(s, u, quantum)
    }

    /// [`TimeCoordinate::stop_index`].
    pub fn stop_index(&self, s: f64, u: f64, quantum: f64) -> Result<u32, TimeRefused> {
        self.time.stop_index(s, u, quantum)
    }

    /// [`TimeCoordinate::playhead_index`].
    pub fn playhead_index(&self, quantum: f64) -> Result<u64, TimeRefused> {
        self.time.playhead_index(quantum)
    }

    /// The state at the pixel `(s, u)` on `trajectory`, the pixel's start's: the trajectory's state at the pixel's
    /// boundary on the trajectory's own grid, through the trajectory interface; refused past the trajectory's horizon
    /// as past the window's.
    pub fn state_at<P: Provider + ?Sized>(
        &self,
        trajectory: &Trajectory<'_, P>,
        s: f64,
        u: f64,
    ) -> Result<Ic<f64>, TimeRefused> {
        let k = self.index_at(s, u, trajectory.quantum())?;
        Ok(trajectory.state(k)?)
    }

    /// The largest step in time between neighbouring pixels on a screen of `pixels` `[across, up]`, along the steeper
    /// axis; 0 on a view untilted in time.
    pub fn pixel_step(&self, pixels: [u32; 2]) -> f64 {
        let h = self.time.pixel_step(Axis::Horizontal, pixels[0]);
        h.max(self.time.pixel_step(Axis::Vertical, pixels[1]))
    }

    /// The view zoomed by the factor `k` about its centre, each screen axis's column, its latent part and its time part
    /// alike, times `k` (the zoom family's `q·k`), except that a zoom in, `k < 1`, of an axis with a time part stops
    /// where that axis's pixel step along time reaches one boundary of the grid of quantum `quantum` ([`Floor`]), and
    /// one that starts below it does not move. An axis with no time part has no floor: on a time-axis view the IC axis
    /// zooms freely while the time axis floors (§ 18.4: time zoom and IC zoom are separate).
    ///
    /// The plan is unclear here; chose the floor at a pixel step of one boundary, as § 18.3 has it ("it stops when the
    /// row spacing reaches one boundary: there is nothing finer"), not the whole window's extent at one boundary,
    /// because a pixel step below one boundary only repeats the rows it already shows; and chose to floor each axis on
    /// its own, so a floored time axis does not freeze the IC axis beside it.
    pub fn zoom(&self, k: f64, quantum: f64, pixels: [u32; 2]) -> Result<Zoomed, TimeRefused> {
        finite(&[k, quantum])?;
        if !(k > 0.0 && quantum > 0.0) {
            return Err(TimeRefused::NotFinite);
        }
        let mut view = self.clone();
        let mut floor = None;
        for (axis, n) in [(Axis::Horizontal, pixels[0]), (Axis::Vertical, pixels[1])] {
            let step = self.time.pixel_step(axis, n);
            let applied = if k < 1.0 && step > 0.0 && step * k < quantum {
                let mut applied = (quantum / step).clamp(k, 1.0);
                let at = |f: f64| {
                    let mut t = self.time;
                    t.column[axis.index()] *= f;
                    t.pixel_step(axis, n)
                };
                while applied < 1.0 && at(applied) < quantum {
                    applied = f64::from_bits(applied.to_bits() + 1);
                }
                let applied = applied.min(1.0);
                floor = Some(Floor { quantum, applied });
                applied
            } else {
                k
            };
            let q = view.latent_column_mut(axis);
            *q = q.map(|x| x * applied);
            view.time.column[axis.index()] *= applied;
        }
        Ok(Zoomed { view, floor })
    }
}

/// How a sample stopped at its own stop index reads: "reached its time" where it stopped there short of the horizon,
/// otherwise the payload's own label.
///
/// The plan is unclear here; chose to read "reached its time" from the payload and the sim key's stop option, and not
/// to add a sixth state to the payload's state enum, because that enum is generated from the ledger and read by every
/// consumer, the GPU's included, and the per-sample stop is not yet in the GPU record; the label here is the engine's
/// until the record carries `k_stop`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopLabel {
    /// The sample stopped at its own stop index, short of the horizon, with no final outcome: `running`, or, with the
    /// stop option off, a live escape label (it may still change past `k_stop`).
    ReachedItsTime,
    /// The payload's state code (`sd_state`).
    Payload(u32),
}

/// The label of `stopped`, a sample marched to its stop index `k_stop` under a horizon of `horizon_steps` with the sim
/// key's stop option `stop_on_escape` (`kernel::driver::wrapper::integrate_detecting_to`'s).
///
/// A march that stopped at `t_end_step = k_stop` below the horizon, its budget not spent, reached its time when it is
/// `running` there, or, with the stop option off, when it is `escape` there: off, the escape label is live, re-read at
/// every window mark and free to change, and the march never ends on it, so an escape at `k_stop` is the label the
/// sample carries at its time, not an outcome. With the option on, the first label ends the march and stays, so an
/// escape is final wherever it stands. The plan is unclear here for a first label landing exactly on `k_stop` under the
/// option; chose final, because the label stays from the step it fires whatever the stop index, as it would at any
/// earlier step.
pub fn stop_label<R: kernel::payload::PayloadReal>(
    stopped: &SimStateBaseOf<R>,
    k_stop: u32,
    horizon_steps: u32,
    stop_on_escape: bool,
) -> StopLabel {
    let state = sd_state(stopped.packed_a);
    let not_final = state == STATE_RUNNING || (state == STATE_ESCAPE && !stop_on_escape);
    if not_final
        && k_stop < horizon_steps
        && tm_t_end_step(stopped.times) == k_stop
        && !bb_budget_exhausted(stopped.budget_bits)
    {
        StopLabel::ReachedItsTime
    } else {
        StopLabel::Payload(state)
    }
}
