//! Time as the map's last coordinate (`engine::time_coordinate`): the playhead is the window's centre along
//! time; a tilt toward time at 90° is the time axis at its home extent; a target time maps to the latest boundary at
//! or before it, in integers; the zoom along time floors at one boundary; a pixel's state is read through the
//! trajectory interface; and the measurement of chart_composition § 18.5's C1 with per-sample stops: every row's words
//! bitwise those of a run with that row as its horizon, the state label excepted.
//!
//! Each pass criterion is written here before the run, and each test has a control that breaks the property it guards.

use engine::contract::sim_config::{Plane, Window};
use engine::contract::sim_uniforms::horizon_steps;
use engine::first_slice::{forward_table, narrow, uniforms, MarchKey, Quad};
use engine::time_coordinate::{
    index_at_or_before, macro_step_for_sync, sin_cos_degrees, stop_label, Axis, StopLabel,
    TimeRefused, TimedPlane,
};
use engine::trajectory::{same_bits, Lagrange, Member, Refused, Trajectory};
use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::driver::advance::Advance;
use kernel::driver::wrapper::{integrate_detecting, integrate_detecting_to};
use kernel::occupant::rk4::Rk4;
use kernel::payload::{sd_state, set_state, SimStateBase, STATE_BOUNDED, STATE_ESCAPE};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::pairs::DefaultOccupant;
use kernel::regularisation::sync::{boundary_of_step, delta_sync};
use validation::negative_control;

/// The app's start view: "z_α × z_β" at its starting `z₀`, `q₁ = e_α`, `q₂ = e_β` (`gui::mock::state`).
const Z0: [f64; 8] = [
    0.180,
    0.31757693755701344,
    0.0,
    -0.227,
    0.0,
    0.312,
    0.333,
    0.333,
];

fn e(k: usize) -> [f64; 8] {
    core::array::from_fn(|i| if i == k { 1.0 } else { 0.0 })
}

fn start_plane() -> Plane {
    Plane {
        z0: Z0,
        q1: e(0),
        q2: e(1),
    }
}

/// The horizon the tests run at: § 18.5's `T = 13`.
const T: f64 = 13.0;
/// The rows of a 1080p screen.
const ROWS: u32 = 1080;

/// The pixel centre of row (or column) `j` of `n`.
fn centre(j: u32, n: u32) -> f64 {
    (f64::from(j) + 0.5) / f64::from(n)
}

fn bits(z: &[f64; 8]) -> [u64; 8] {
    z.map(f64::to_bits)
}

/// The Lagrange circular member at unequal masses, a quantum its period over 64: a trajectory known exactly.
fn lagrange() -> Lagrange {
    Lagrange::new([0.5, 0.3, 0.2], Member::Circular, 64.0)
}

// ── The playhead is the window's centre along time ──────────────────────────────────────────────────────────────────

/// The time at a pixel, the view's own or a control's.
type TimeAt = fn(&TimedPlane, f64, f64) -> f64;

/// Criterion: on every view, untilted, tilted toward time at 30° and at 90° on either axis, and zoomed, the centre
/// pixel's time is the playhead bit for bit; on the untilted view every pixel of a 16 × 16 grid is.
fn check_playhead(time_at: TimeAt) {
    let untilted = TimedPlane::untilted(start_plane(), T, 4.25).unwrap();
    for j in 0..16 {
        for i in 0..16 {
            let t = time_at(&untilted, centre(i, 16), centre(j, 16));
            assert_eq!(
                t.to_bits(),
                4.25f64.to_bits(),
                "an untilted pixel's time is not the playhead"
            );
        }
    }
    let mut views = vec![untilted.clone()];
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for degrees in [30.0, 90.0, -45.0] {
            let v = untilted.tilt_toward_time(axis, degrees).unwrap();
            views.push(v.clone());
            views.push(v.zoom(0.37, 1e-9, [64, 64]).unwrap().view);
        }
    }
    for v in &views {
        assert_eq!(
            time_at(v, 0.5, 0.5).to_bits(),
            v.time.playhead.to_bits(),
            "the centre pixel's time is not the playhead: {v:?}"
        );
    }
}

#[test]
fn time_coordinate_playhead_is_the_centre() {
    check_playhead(TimedPlane::time_at);
}

negative_control!(
    time_coordinate_playhead_is_the_centre,
    "the playhead at the window's bottom-left corner, not its centre",
    expected = "the centre pixel's time is not the playhead",
    check_playhead(|v, s, u| {
        let [a, b] = v.time.column;
        v.time.playhead + 0.5 * v.time.horizon * (s * a + u * b)
    })
);

// ── A tilt toward time at 90° is the time axis at its home extent ───────────────────────────────────────────────────

/// Criterion: from the start view at zoom 1 and its playhead at `T/2`, the tilt toward time at 90° of either axis is
/// the home time axis on that axis bit for bit (`==` on every float), and along the tilted axis every pixel of a 1080
/// row column reads one latent point bit for bit: a column is one start.
fn check_tilt_is_home(sin_cos: fn(f64) -> (f64, f64)) {
    let home = TimedPlane::untilted(start_plane(), T, 0.5 * T).unwrap();
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let tilted = home.tilt_with(axis, 90.0, sin_cos).unwrap();
        let axis_view = home.home_time_axis(axis);
        assert!(
            tilted == axis_view,
            "the 90° tilt is not the home time axis on {axis:?}: {tilted:?} against {axis_view:?}"
        );
        for col in [0, 31, 63] {
            let s = centre(col, 64);
            let at = |j: u32| match axis {
                Axis::Vertical => tilted.point(s, centre(j, ROWS)),
                Axis::Horizontal => tilted.point(centre(j, ROWS), s),
            };
            let first = bits(&at(0));
            for j in 1..ROWS {
                assert_eq!(
                    bits(&at(j)),
                    first,
                    "the 90° tilt is not the home time axis: row {j} of a column reads another start"
                );
            }
        }
    }
}

#[test]
fn time_coordinate_tilt_90_is_home_axis() {
    check_tilt_is_home(sin_cos_degrees);
}

negative_control!(
    time_coordinate_tilt_90_is_home_axis,
    "the tilt's sine and cosine through libm at every angle, so cos 90° leaves 6e-17 of the latent column",
    expected = "the 90° tilt is not the home time axis",
    check_tilt_is_home(|d| {
        let r = d.to_radians();
        (libm::sin(r), libm::cos(r))
    })
);

/// Criterion: the home time axis spans `[0, T]` exactly: its first edge's time is 0 and its last's `T`, bit for bit;
/// its 1080 row centres' boundaries on the grid `Δ = 10⁻³` climb from 6 (`⌊0.5·13/1080 / 10⁻³⌋`) to 12993 by steps
/// that never go down, each the latest boundary at or before the row's time; a time past `T` and one before 0 are
/// refused by name.
fn check_home_extent(sigma: fn(&TimedPlane) -> f64) {
    let axis = TimedPlane::untilted(start_plane(), T, 0.0)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    let time = |u: f64| axis.time_with(0.5, u, sigma(&axis));
    assert_eq!(
        time(0.0).to_bits(),
        0f64.to_bits(),
        "the home axis does not start at 0"
    );
    assert_eq!(
        time(1.0).to_bits(),
        T.to_bits(),
        "the home axis does not end at T"
    );
    let dt = 1e-3;
    let mut last = 0;
    for j in 0..ROWS {
        let k = axis.index_at(0.5, centre(j, ROWS), dt).unwrap();
        assert!(
            k >= last,
            "row {j}'s boundary {k} is below row {}'s {last}",
            j.max(1) - 1
        );
        let t = axis.time_at(0.5, centre(j, ROWS));
        assert!(
            k as f64 * dt <= t && (k + 1) as f64 * dt > t,
            "row {j}: {k} is not the latest boundary at or before {t}"
        );
        last = k;
    }
    assert_eq!(axis.index_at(0.5, centre(0, ROWS), dt), Ok(6));
    assert_eq!(last, 12993);
    let past = axis
        .home_time_axis(Axis::Vertical)
        .with_playhead(T)
        .unwrap();
    assert!(matches!(
        past.index_at(0.5, 0.75, dt),
        Err(TimeRefused::PastHorizon { .. })
    ));
    assert!(matches!(
        axis.with_playhead(0.0).unwrap().index_at(0.5, 0.25, dt),
        Err(TimeRefused::BeforeStart { .. })
    ));
}

#[test]
fn time_coordinate_home_extent() {
    check_home_extent(|v| v.time.horizon);
}

negative_control!(
    time_coordinate_home_extent,
    "time's scale taken as T/2, the half-span, in place of the span of [0, T]",
    expected = "the home axis does not start at 0",
    check_home_extent(|v| 0.5 * v.time.horizon)
);

// ── A target time maps to the latest boundary at or before it, in integers ─────────────────────────────────────────

/// The boundary at or before `t` on the grid `dt`, the module's or a control's.
type IndexOf = fn(f64, f64) -> u64;

/// Criterion: on the grids `Δ = 10⁻³` (the unregularised macro-step), `Δ_sync` and the Lagrange fixture's quantum, every
/// row time `fl(k·Δ)`, `k = 0 … 65535`, maps back to `k`, and the float just below it, where it is above `(k − 1)·Δ`,
/// to `k − 1`.
fn check_index(index_of: IndexOf) {
    for dt in [1e-3, 0.125, lagrange().period().unwrap() / 64.0] {
        for k in 0u64..=65535 {
            let t = k as f64 * dt;
            let got = index_of(dt, t);
            assert_eq!(
                got, k,
                "the row time k·Δ = {t} on Δ = {dt} maps to {got}, not {k}"
            );
            if k > 0 {
                let below = f64::from_bits(t.to_bits() - 1);
                if below >= (k - 1) as f64 * dt {
                    assert_eq!(
                        index_of(dt, below),
                        k - 1,
                        "the time just below k·Δ = {t} on Δ = {dt} maps to k"
                    );
                }
            }
        }
    }
}

#[test]
fn time_coordinate_target_index_in_integers() {
    check_index(|dt, t| index_at_or_before(dt, t).unwrap());
    // The measurement § 18.2 quotes: how many of `k = 1 … 65535` the float row time `⌊(k·10⁻³)/10⁻³⌋` lands one low.
    let low = (1u64..=65535)
        .filter(|&k| ((k as f64 * 1e-3) / 1e-3).floor() as u64 != k)
        .count();
    println!("measured: the float row time ⌊(k·10⁻³)/10⁻³⌋ lands off k for {low} of 65535 k (§ 18.2 quotes 731)");
}

negative_control!(
    time_coordinate_target_index_in_integers,
    "a float row time, ⌊t/Δ⌋, which lands one low where k·Δ rounds down",
    expected = "maps to",
    check_index(|dt, t| (t / dt).floor() as u64)
);

// ── The zoom along time floors at one boundary ─────────────────────────────────────────────────────────────────────

/// A zoom by `k` on the grid `quantum` and screen `pixels`: the module's, or a control's.
type ZoomOf = fn(&TimedPlane, f64, f64, [u32; 2]) -> TimedPlane;

/// Criterion: on the home time axis at `T = 13`, 64 × 1080 pixels, a row step of `13/1080`, zooming in by halves:
/// on `Δ = 10⁻³` the row step never goes below `Δ`, and the zoom stops just above it (below `Δ·(1 + 10⁻¹²)`),
/// and stays there under a further zoom in, its floor reported with the reason "one boundary: 0.001"; on `Δ_sync = 1/8`, already finer than
/// a boundary per row at home, a zoom in does not move the time axis while the IC axis beside it halves, and a zoom
/// out moves the time axis.
fn check_floor(zoom: ZoomOf) {
    let pixels = [64, ROWS];
    let home = TimedPlane::untilted(start_plane(), T, 0.5 * T)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    let dt = 1e-3;
    let mut v = home.clone();
    for _ in 0..40 {
        v = zoom(&v, 0.5, dt, pixels);
        assert!(
            v.pixel_step(pixels) >= dt,
            "the zoom along time went below one boundary: a row step of {} on Δ = {dt}",
            v.pixel_step(pixels)
        );
    }
    assert!(
        v.pixel_step(pixels) < dt * (1.0 + 1e-12),
        "the zoom stopped short of the floor: {}",
        v.pixel_step(pixels)
    );
    let floored = v.zoom(0.5, dt, pixels).unwrap();
    let step = floored.view.pixel_step(pixels);
    assert!(
        step >= dt && step < dt * (1.0 + 1e-12),
        "a zoom in at the floor left it: {step}"
    );
    assert_eq!(
        floored.floor.map(|f| f.reason()),
        Some("one boundary: 0.001".into())
    );
    let sync = 0.125;
    let still = home.zoom(0.5, sync, pixels).unwrap();
    assert_eq!(
        still.view.time.column, home.time.column,
        "a zoom in below one boundary per row moved the time axis"
    );
    assert_eq!(
        still.view.plane.q1,
        home.plane.q1.map(|x| 0.5 * x),
        "the floored time axis froze the IC axis beside it"
    );
    assert!(still.floor.is_some());
    let out = home.zoom(2.0, sync, pixels).unwrap();
    assert!(out.floor.is_none() && out.view.time.column[1] == 2.0);
    // An untilted view has no floor.
    let flat = TimedPlane::untilted(start_plane(), T, 1.0).unwrap();
    assert_eq!(flat.zoom(1e-6, sync, pixels).unwrap().floor, None);
}

#[test]
fn time_coordinate_zoom_floor() {
    check_floor(|v, k, dt, px| v.zoom(k, dt, px).unwrap().view);
}

negative_control!(
    time_coordinate_zoom_floor,
    "a zoom with no floor: every column times k, whatever the step along time",
    expected = "the zoom along time went below one boundary",
    check_floor(|v, k, _, px| v.zoom(k, 1e-300, px).unwrap().view)
);

// ── A pixel's state through the trajectory interface ───────────────────────────────────────────────────────────────

/// The state a pixel reads, the module's or a control's.
type StateOf = fn(&TimedPlane, &Trajectory<'_, Lagrange>, f64, f64) -> Result<Ic<f64>, TimeRefused>;

/// Criterion: on the home time axis over the Lagrange circular member's period, each of 1080 rows reads the
/// trajectory's state at the latest boundary at or before its time, bit for bit, the closed form's; the first row
/// reads the start; and on the homothetic member, which collapses to a triple collision, a row past the collision
/// reads the terminal state.
fn check_reads(state_of: StateOf) {
    let fixture = lagrange();
    let horizon = fixture.period().unwrap();
    let trajectory = Trajectory::new(&fixture, fixture.start(), horizon).unwrap();
    let axis = TimedPlane::untilted(start_plane(), horizon, 0.0)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    for j in 0..ROWS {
        let u = centre(j, ROWS);
        let t = axis.time_at(0.5, u);
        let k = index_at_or_before(trajectory.quantum(), t).unwrap();
        let got = state_of(&axis, &trajectory, 0.5, u).unwrap();
        assert!(
            same_bits(&got, &trajectory.state(k).unwrap()),
            "row {j} at t = {t} does not read its boundary {k}'s state"
        );
    }
    assert!(same_bits(
        &state_of(&axis, &trajectory, 0.5, centre(0, ROWS)).unwrap(),
        &fixture.start()
    ));
    let collapse = Lagrange::new([1.0 / 3.0; 3], Member::Homothetic, 32.0);
    let tc = collapse.collapse().unwrap();
    let falls = Trajectory::new(&collapse, collapse.start(), 2.0 * tc).unwrap();
    let terminal = falls.terminal().expect("the homothetic member collapses");
    let axis = TimedPlane::untilted(start_plane(), 2.0 * tc, 0.0)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    let held = falls.state(terminal.index).unwrap();
    for j in (ROWS / 2 + 10)..ROWS {
        let got = state_of(&axis, &falls, 0.5, centre(j, ROWS)).unwrap();
        assert!(
            same_bits(&got, &held),
            "row {j}, past the collision, does not hold the terminal state"
        );
    }
}

#[test]
fn time_coordinate_reads_the_trajectory() {
    check_reads(|v, tr, s, u| v.state_at(tr, s, u));
}

negative_control!(
    time_coordinate_reads_the_trajectory,
    "a row read one boundary off, at the boundary after its own",
    expected = "does not read its boundary",
    check_reads(|v, tr, s, u| {
        let k = v.index_at(s, u, tr.quantum())?;
        tr.state((k + 1).min(tr.last())).map_err(TimeRefused::Grid)
    })
);

/// A tilt toward time, the module's or a control's.
type TiltOf = fn(&TimedPlane, Axis, f64) -> Result<TimedPlane, TimeRefused>;

/// Criterion: a second tilt toward time of an axis already tilted, a NaN angle, a zero horizon, a zero quantum, and a
/// pixel past the trajectory's own horizon are each refused by name.
fn check_refusals(tilt: TiltOf) {
    let home = TimedPlane::untilted(start_plane(), T, 0.5 * T).unwrap();
    let tilted = tilt(&home, Axis::Vertical, 30.0).unwrap();
    assert_eq!(
        tilt(&tilted, Axis::Vertical, 10.0),
        Err(TimeRefused::AlreadyTilted),
        "a second tilt toward time is accepted on an axis already tilted"
    );
    assert_eq!(
        tilt(&home, Axis::Vertical, f64::NAN),
        Err(TimeRefused::NotFinite)
    );
    assert_eq!(
        TimedPlane::untilted(start_plane(), 0.0, 0.0),
        Err(TimeRefused::NotFinite)
    );
    assert_eq!(
        index_at_or_before(0.0, 1.0),
        Err(TimeRefused::Grid(Refused::Grid))
    );
    let fixture = lagrange();
    let short = Trajectory::new(&fixture, fixture.start(), 1.0).unwrap();
    let axis = home.home_time_axis(Axis::Vertical);
    assert!(matches!(
        axis.state_at(&short, 0.5, 0.9),
        Err(TimeRefused::Grid(Refused::PastHorizon { .. }))
    ));
}

#[test]
fn time_coordinate_refusals() {
    check_refusals(|v, axis, d| v.tilt_toward_time(axis, d));
}

negative_control!(
    time_coordinate_refusals,
    "a tilt that forgets the axis's time part, so a second tilt toward time is taken from scratch",
    expected = "a second tilt toward time is accepted on an axis already tilted",
    check_refusals(|v, axis, d| {
        let mut w = v.clone();
        w.time.column = [0.0; 2];
        w.tilt_toward_time(axis, d)
    })
);

// ── A column along a time axis is one start, signed zeros included ─────────────────────────────────────────────────

/// The latent point at a pixel, the module's or a control's.
type PointOf = fn(&TimedPlane, f64, f64) -> [f64; 8];

/// Criterion (pitfalls § 11): with `z₀` holding a `−0.0` and the tilted column a negative component, so that the tilt
/// at 90° leaves `−0.0` in it, every pixel of a 1080 row column along the time axis reads one latent point bit for
/// bit, at three columns.
fn check_signed_zero(point_of: PointOf) {
    let mut z0 = Z0;
    z0[2] = -0.0;
    let mut q2 = e(1);
    q2[2] = -0.25;
    let plane = Plane { z0, q1: e(0), q2 };
    let view = TimedPlane::untilted(plane, T, 0.5 * T)
        .unwrap()
        .tilt_toward_time(Axis::Vertical, 90.0)
        .unwrap();
    assert!(view
        .plane
        .q2
        .iter()
        .any(|x| x.to_bits() == (-0.0f64).to_bits()));
    for col in [0, 31, 63] {
        let s = centre(col, 64);
        let first = bits(&point_of(&view, s, centre(0, ROWS)));
        for j in 1..ROWS {
            assert_eq!(
                bits(&point_of(&view, s, centre(j, ROWS))),
                first,
                "row {j} of a time-axis column reads another start: a signed zero split it"
            );
        }
    }
}

#[test]
fn time_coordinate_signed_zero_column() {
    check_signed_zero(TimedPlane::point);
}

negative_control!(
    time_coordinate_signed_zero_column,
    "the point through today's map with the zero column added, its ±0 following the sign of 2u − 1",
    expected = "a signed zero split it",
    check_signed_zero(|v, s, u| engine::lock::point(&v.plane, s, u))
);

// ── The default occupant's stop on the sync grid ──────────────────────────────────────────────────────────────────

/// The wrapper's macro-step count for a row that reads sync boundary `j` at time `t`: the module's or a control's.
type MacroOf = fn(u32, f64, f32, f32) -> u32;

/// Criterion: under the default key at `T = 13` (f32 `dt_macro`, `Δ_sync = 1/8`), for every sync boundary
/// `j = 0 … 104`, read at its own time `j·Δ_sync`, and for every one of 1080 home-axis rows and the playhead at `T/2`,
/// the wrapper's march to the macro-step count given lands on `j`, the latest sync boundary at or before the row's
/// time: `boundary_of_step(m) = j`, and `m` is the least such.
fn check_sync_stop(macro_of: MacroOf) {
    let key = uniforms(T).unwrap();
    let dt = key.loop_values.dt_macro;
    let sync: f32 = delta_sync();
    let axis = TimedPlane::untilted(start_plane(), T, 0.0)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    let mut times: Vec<f64> = (0..=104).map(|j| f64::from(j) * 0.125).collect();
    times.extend((0..ROWS).map(|j| axis.time_at(0.5, centre(j, ROWS))));
    times.push(0.5 * T);
    for t in times {
        let j = index_at_or_before(f64::from(sync), t).unwrap() as u32;
        let m = macro_of(j, t, dt, sync);
        let lands = boundary_of_step::<f32>(m, dt, sync);
        assert_eq!(
            lands, j,
            "the stop for t = {t} lands on sync boundary {lands}, not {j}"
        );
        assert!(
            m == 0 || boundary_of_step::<f32>(m - 1, dt, sync) < j,
            "t = {t}: {m} is not the least"
        );
    }
}

#[test]
fn time_coordinate_sync_stop() {
    check_sync_stop(|j, _, dt, sync| macro_step_for_sync(j, dt, sync));
    // The measurement: how often the target time taken to the macro-step grid first lands off its sync boundary.
    let key = uniforms(T).unwrap();
    let dt = key.loop_values.dt_macro;
    let sync: f32 = delta_sync();
    let off = (0..=104u32)
        .filter(|&j| {
            let t = f64::from(j) * 0.125;
            let m = index_at_or_before(f64::from(dt), t).unwrap() as u32;
            boundary_of_step::<f32>(m, dt, sync) != j
        })
        .count();
    println!("measured: of the 105 sync boundaries' own times at T = 13, {off} land one boundary low through the macro-step grid");
}

negative_control!(
    time_coordinate_sync_stop,
    "the target time taken to the macro-step grid first, on the f32 dt_macro, then to the sync boundary",
    expected = "lands on sync boundary",
    check_sync_stop(|_, t, dt, _| index_at_or_before(f64::from(dt), t).unwrap() as u32)
);

// ── C1 with per-sample stops ────────────────────────────────────────────────────────────────────────────────────────

/// The CPU field's Heggie values (`engine::field::heggie_params`), at either width: the default occupant's.
struct FieldValues;

impl HgConfig for FieldValues {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: delta_sync::<R>(),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(4e4, 4e4),
            n_min: 100_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// Heggie × RK4, the default occupant, at the CPU field's values.
type HeggieRk4 = DefaultOccupant<FieldValues>;

/// Every word of a `SimState`, by name, its state code masked out of `packed_a` where `mask`.
fn words(s: &SimStateBase, mask: bool) -> Vec<(&'static str, u64)> {
    let f = |x: f32| u64::from(x.to_bits());
    let mut w = vec![];
    for i in 0..3 {
        for d in 0..2 {
            w.push(("r", f(s.r[i][d])));
            w.push(("p", f(s.p[i][d])));
        }
    }
    let packed_a = if mask {
        set_state(s.packed_a, 0)
    } else {
        s.packed_a
    };
    w.extend([
        ("S", f(s.S)),
        ("theta", f(s.theta)),
        ("mean_y", f(s.mean_y)),
        ("C_ty", f(s.C_ty)),
        ("E_0", f(s.E_0)),
        ("Lz_0", f(s.Lz_0)),
        ("packed_a", u64::from(packed_a)),
        ("packed_b", u64::from(s.packed_b)),
        ("times", u64::from(s.times)),
        ("total_substeps", u64::from(s.total_substeps)),
        ("budget_bits", u64::from(s.budget_bits)),
        ("dE_reg", u64::from(s.dE_reg)),
        ("closure_min", f(s.closure_min)),
        ("closure_step", u64::from(s.closure_step)),
        ("_reserved", u64::from(s._reserved)),
        ("coll_floor_bits", u64::from(s.coll_floor_bits)),
        ("_nhat_lag_0", f(s._nhat_lag_0)),
        ("_nhat_lag_1", f(s._nhat_lag_1)),
        ("_nhat_lag_2", f(s._nhat_lag_2)),
        ("_dmin_d2", u64::from(s._dmin_d2)),
        ("_march_bits", u64::from(s._march_bits)),
    ]);
    w
}

/// The start view's chart decoded on a `w × h` quad, narrowed once to f32.
fn starts(size: [usize; 2]) -> Vec<Ic<f32>> {
    let narrow8 = |v: [f64; 8]| v.map(|x| x as f32);
    let quad = Quad {
        chart: Latent {
            z0: narrow8(Z0),
            q1: narrow8(e(0)),
            q2: narrow8(e(1)),
        },
        window: Window::SQUARE,
        size,
    };
    forward_table(&quad).unwrap().iter().map(narrow).collect()
}

/// C1's columns under the occupant `A`: the eight starts across the start view's middle row, and the first four
/// starts of a 16 × 16 scan of the view whose whole march to `T` ends in a terminal (a collision or a failure) before
/// it, so that rows past a terminal are compared too. An escape is no terminal: with the stop option off its label is
/// live and the march goes on.
fn columns<A: Advance>(key: &MarchKey) -> Vec<Ic<f32>> {
    let mut out = starts([8, 1]);
    out.extend(
        starts([16, 16])
            .into_iter()
            .filter(|ic| {
                let s = integrate_detecting::<A, f32>(ic, &key.loop_values, key.radius);
                let state = sd_state(s.packed_a);
                state != STATE_BOUNDED && state != STATE_ESCAPE
            })
            .take(4),
    );
    out
}

/// What C1 found on one occupant: rows compared, rows whose words differ, the fields that differ, and how the rows
/// ended.
#[derive(Debug, Default)]
struct C1 {
    rows: usize,
    differing: usize,
    fields: Vec<&'static str>,
    reached: usize,
    live_escape: usize,
    terminal: usize,
}

/// The wrapper's stop for a row at time `t` under the occupant's grid: the macro-step index for an unregularised
/// occupant, the macro-step count that lands on the row's sync boundary for a regularised one; `offset` boundaries
/// past it on that grid, the control's.
type StopOf = fn(f64, &MarchKey, u32) -> u32;

fn macro_stop(t: f64, key: &MarchKey, offset: u32) -> u32 {
    index_at_or_before(f64::from(key.loop_values.dt_macro), t).unwrap() as u32 + offset
}

fn sync_stop(t: f64, key: &MarchKey, offset: u32) -> u32 {
    let sync: f32 = delta_sync();
    let j = index_at_or_before(f64::from(sync), t).unwrap() as u32 + offset;
    macro_step_for_sync(j, key.loop_values.dt_macro, sync)
}

/// C1 on the occupant `A`: for each column and each 97th of 1080 rows of the home time axis at `T = 13`, the row's stop
/// `m` (`stop_of`), the sample marched to `m` under the horizon `T` (`integrate_detecting_to`), against a run with `m`
/// as its horizon (`horizon_steps = m`, the same `dt_macro`): every word but the state code compared. The reference's
/// stop is `offset = 0`'s; the stopped sample's is `offset`'s.
fn c1<A: Advance>(key: &MarchKey, stop_of: StopOf, offset: u32) -> C1 {
    let axis = TimedPlane::untilted(start_plane(), T, 0.0)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    let horizon = key.loop_values.horizon_steps;
    let mut found = C1::default();
    for ic in columns::<A>(key) {
        let whole = integrate_detecting::<A, f32>(&ic, &key.loop_values, key.radius);
        let at_top =
            integrate_detecting_to::<A, f32>(&ic, &key.loop_values, key.radius, horizon, false);
        assert_eq!(
            words(&whole, false),
            words(&at_top, false),
            "a stop at the horizon is not the run to it"
        );
        for j in (0..ROWS).step_by(97) {
            let t = axis.time.target(0.5, centre(j, ROWS)).unwrap();
            let m = stop_of(t, key, 0);
            let stop = stop_of(t, key, offset);
            let stopped =
                integrate_detecting_to::<A, f32>(&ic, &key.loop_values, key.radius, stop, false);
            let mut at_m = key.loop_values;
            at_m.horizon_steps = m;
            let reference = integrate_detecting::<A, f32>(&ic, &at_m, key.radius);
            found.rows += 1;
            let (a, b) = (words(&stopped, true), words(&reference, true));
            if a != b {
                found.differing += 1;
                for (x, y) in a.iter().zip(&b) {
                    if x != y && !found.fields.contains(&x.0) {
                        found.fields.push(x.0);
                    }
                }
            }
            if offset > 0 {
                continue;
            }
            let label = stop_label(&stopped, stop, horizon, false);
            // With the stop option off the escape label is live, so a reference that ends `escape` at its horizon
            // carries the label there, not an outcome: the stopped sample reached its time, as a `bounded` one did.
            let state = sd_state(reference.packed_a);
            if state == STATE_BOUNDED || state == STATE_ESCAPE {
                assert_eq!(
                    label,
                    StopLabel::ReachedItsTime,
                    "row {j}: a sample stopped at its time is not labelled so (reference state {state})"
                );
                found.reached += 1;
                found.live_escape += usize::from(state == STATE_ESCAPE);
            } else {
                assert_eq!(
                    label,
                    StopLabel::Payload(sd_state(reference.packed_a)),
                    "row {j}: a terminal's label is lost"
                );
                found.terminal += 1;
            }
        }
    }
    found
}

/// Criterion (§ 18.5's C1, as it was written, with per-sample stops), on the occupant `A` at f32, `T = 13`: every
/// row's words bitwise equal to a run with that row as its horizon, the state label excepted (`bounded` belongs to the
/// horizon; the stopped sample "reached its time"), 0 differing rows; a terminal's label kept; and at least one row
/// past a terminal compared. The control's rows stopped one boundary past their own must differ in `r` or `p`, not
/// only in the step count.
fn check_c1<A: Advance>(name: &str, stop_of: StopOf, offset: u32) {
    let key = uniforms(T).unwrap();
    let found = c1::<A>(&key, stop_of, offset);
    println!(
        "measured: C1 on {name}: {} rows, {} differing {:?}; {} reached their time ({} on a live escape label), {} past a \
         terminal",
        found.rows, found.differing, found.fields, found.reached, found.live_escape, found.terminal
    );
    if offset > 0 {
        assert!(
            found.fields.contains(&"r") || found.fields.contains(&"p"),
            "the control's rows differ only in {:?}",
            found.fields
        );
    } else {
        assert!(
            found.terminal > 0,
            "C1 on {name} compared no row past a terminal"
        );
    }
    assert_eq!(
        found.differing, 0,
        "C1 on {name}: a row stopped at its index differs from a run with it as horizon: {:?}",
        found.fields
    );
}

#[test]
fn time_coordinate_c1_unregularised() {
    // A target time floors, the horizon ceils: how many rows' `fl(k·dt)` a host's `⌈T/dt⌉` would not take back to `k`.
    let key = uniforms(T).unwrap();
    let dt = f64::from(key.loop_values.dt_macro);
    let axis = TimedPlane::untilted(start_plane(), T, 0.0)
        .unwrap()
        .home_time_axis(Axis::Vertical);
    let drift = (0..ROWS)
        .map(|j| axis.stop_index(0.5, centre(j, ROWS), dt).unwrap())
        .filter(|&k| horizon_steps(f64::from(k) * dt, dt) as u32 != k)
        .count();
    println!("measured: {drift} of {ROWS} rows' fl(k·dt_macro) give ⌈T/dt_macro⌉ ≠ k (the reference sets horizon_steps = k directly)");
    check_c1::<Rk4>("RK4, unregularised", macro_stop, 0);
}

negative_control!(
    time_coordinate_c1_unregularised,
    "each row stopped one macro-step past its own",
    expected = "C1 on RK4, unregularised: a row stopped at its index differs",
    check_c1::<Rk4>("RK4, unregularised", macro_stop, 1)
);

#[test]
fn time_coordinate_c1_regularised() {
    check_c1::<HeggieRk4>("Heggie × RK4", sync_stop, 0);
}

negative_control!(
    time_coordinate_c1_regularised,
    "each row stopped one sync boundary past its own",
    expected = "C1 on Heggie × RK4: a row stopped at its index differs",
    check_c1::<HeggieRk4>("Heggie × RK4", sync_stop, 1)
);
