//! The fixed sync quantum (`kernel::regularisation::sync`; integrator_contract Part 2a § "The sync grid is a fixed
//! quantum of physical time"): one grid `t = k·Δ_sync`, the march stopping only on it, so live march, catch-up and
//! re-boot reach the same state bitwise. Every arm runs Heggie, Aarseth–Zare and logH, each × RK4, through the seam's
//! `ADVANCE` with the trajectory's carried buffer (`Advance::advance_carrying`), at the register's `Δ_sync`, at 0.1 and
//! at 0.2, at f64 and f32:
//! - `sync_quantum_bitwise`: on Burrau's close-encounter fixture, at the horizons `T = 50` and `T = 200` (their
//!   `dt_macro`), the state and the carried buffer reached at each boundary by one `ADVANCE` from `t = 0` (catch-up), by
//!   `ADVANCE` calls each advancing the target by a fixed increment of physical time, a frame of 23 ms and 0.29, neither a
//!   multiple of `Δ_sync`, every target off the grid (live), and by a re-boot from `t = 0` after a pause, are bitwise
//!   identical; and each call's state is the direct march's at the latest boundary at or before its target, so an
//!   off-grid target returns the boundary before it. Control: a march that sizes its steps from the call's target, an
//!   interval of its own landing on each frame target.
//! - `sync_quantum_bitwise_horizons`: the boundary states at `T = 50` and at `T = 200` are bitwise identical, the same
//!   boundaries reached through each horizon's macro-steps. Control: a grid of `t_max / n_sync`, which differs between
//!   the two horizons.
//! - `sync_quantum_bitwise_collision`: a copy that collides between boundaries (two bodies falling head-on) latches at
//!   the firing step: `ADVANCE` returns the state the cadence fired on, the call reports it short, and live, catch-up
//!   and re-boot agree bitwise, in the state and in the carried buffer. Control: a march that carries the collided copy
//!   on to the next boundary.
//! - `sync_quantum_bitwise_collision_t_end`: the march's `t_end` there is the firing step's time, strictly between the
//!   boundary before it and the one after. Control: `t_end` held back to the previous boundary.
//! - `sync_quantum_bitwise_budget`: on a step budget that runs out after a few boundaries, live calls by a frame end
//!   where one call ends, bitwise: the running step total crosses the calls in the carried buffer, for
//!   Aarseth–Zare too. Control: a seam that starts the total afresh at each call.
//!
//! Each number a check reads is printed.

use kernel::driver::advance::{Advance, AdvanceParams, Cadence, Reached};
#[cfg(feature = "controls")]
use kernel::driver::advance::{AdvanceReal, Advanced};
use kernel::driver::carried::Carried;
use kernel::driver::wrapper::WrapperReal;
use kernel::generated::constants::DELTA_SYNC;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{
    self as az, AarsethZare, AzConfig, AzCount, AzParams, AzReal, End, ExplicitTest,
};
use kernel::regularisation::heggie::{self as hg, Heggie, HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::logh::{self as lh, LhConfig, LhParams, LogH};
use kernel::regularisation::pairs::LogHMode;
use kernel::regularisation::sync;
use kernel::regularisation::Composed;
use validation::negative_control;

// ── Precisions ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: WrapperReal + AzReal + Into<f64> + core::fmt::Debug {
    fn of(x: f64) -> Self;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// `x`'s bits, widened exactly.
fn b<R: P>(x: R) -> u64 {
    Into::<f64>::into(x).to_bits()
}

// ── The occupants' values ──────────────────────────────────────────────────────────────────────────────────────

/// Heggie's test values, as `tests/heggie_seam.rs` takes them (`η = 0.01`, the reference's `TINY`, `f = 0.02`, a
/// budget that does not bind, the landing in force, Eq. 22), on the sync quantum `delta`. Test inputs.
fn hg_values<R: AzReal>(delta: f64) -> HgParams<R> {
    HgParams {
        eta: R::at_width(0.01, 0.01),
        delta_sync: R::at_width(delta as f32, delta),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: R::at_width(0.02, 0.02),
        step_budget_rate: R::at_width(1e7, 1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

/// Aarseth–Zare's, the same values on `delta`.
fn az_values<R: AzReal>(delta: f64) -> AzParams<R> {
    let v = hg_values::<R>(delta);
    AzParams {
        eta: v.eta,
        delta_sync: v.delta_sync,
        tiny: v.tiny,
        predictive_f: v.predictive_f,
        step_budget_rate: v.step_budget_rate,
        n_min: v.n_min,
        clamp: v.clamp,
        secant_tries: v.secant_tries,
        bisect_tries: v.bisect_tries,
    }
}

/// logH's, the same values on `delta`, in the logH mode.
fn lh_values<R: AzReal>(delta: f64) -> LhParams<R> {
    let v = hg_values::<R>(delta);
    LhParams {
        eta: v.eta,
        delta_sync: v.delta_sync,
        tiny: v.tiny,
        predictive_f: v.predictive_f,
        step_budget_rate: v.step_budget_rate,
        n_min: v.n_min,
        clamp: v.clamp,
        secant_tries: v.secant_tries,
        bisect_tries: v.bisect_tries,
        mode: LogHMode::LogH,
    }
}

/// A configuration of all three regularisations on one sync quantum.
trait Quantum: HgConfig + AzConfig + LhConfig {
    /// The sync quantum, `Δ_sync`.
    const DELTA: f64;
}

/// Declares `$name`, the three regularisations' values on the quantum `$delta`.
macro_rules! quantum {
    ($(#[$doc:meta])* $name:ident, $delta:expr) => {
        $(#[$doc])*
        struct $name;

        impl Quantum for $name {
            const DELTA: f64 = $delta;
        }

        impl HgConfig for $name {
            fn params<R: AzReal>() -> HgParams<R> {
                hg_values($delta)
            }
        }

        impl AzConfig for $name {
            fn params<R: AzReal>() -> AzParams<R> {
                az_values($delta)
            }
        }

        impl LhConfig for $name {
            fn params<R: AzReal>() -> LhParams<R> {
                lh_values($delta)
            }
        }
    };
}

quantum!(
    /// The register's `Δ_sync`, the proposed value.
    Register,
    DELTA_SYNC
);
quantum!(
    /// The second value run, 0.1.
    Tenth,
    0.1
);
quantum!(
    /// The value Aarseth–Zare's proposed sync-quantum calibration picks in its own test, 0.2.
    Fifth,
    0.2
);
#[cfg(feature = "controls")]
quantum!(
    /// A grid of `t_max / n_sync` at `T = 50`, `n_sync = 400`: 1/8. Never the occupant's; a control's.
    PerSync50,
    50.0 / 400.0
);
#[cfg(feature = "controls")]
quantum!(
    /// The same grid at `T = 200`: 1/2. Never the occupant's; a control's.
    PerSync200,
    200.0 / 400.0
);

/// Heggie × RK4 on `Q`.
type Hg<Q> = Composed<Rk4, Heggie<Q>>;
/// Aarseth–Zare × RK4 on `Q`.
type Az<Q> = Composed<Rk4, AarsethZare<Q>>;
/// logH × RK4 on `Q`.
type Lh<Q> = Composed<Rk4, LogH<Q>>;

/// Which regularisation a march is: the direct march it is checked against.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Heggie,
    AarsethZare,
    LogH,
}

/// The direct march of `s` on `Q`'s values under `kind` to boundary `k_target`, the cadence `cadence` after each step:
/// the occupant's own march, registered at `t = 0` (Heggie, logH) or at every boundary (Aarseth–Zare), with no seam.
/// Returns the state, the boundary it stands on, its physical time and how it ended.
fn direct<Q: Quantum, R: P, C: Cadence<R>>(
    kind: Kind,
    s: &State<R>,
    k_target: u32,
    cadence: &mut C,
) -> (State<R>, u32, R, End) {
    match kind {
        Kind::Heggie => {
            let out = hg::march::<Rk4, R, C, ExplicitTest>(
                s,
                k_target,
                &<Q as HgConfig>::params(),
                cadence,
                &mut ExplicitTest,
            );
            (out.state, out.boundary, out.t, out.count.end)
        }
        Kind::AarsethZare => {
            let out = az::march::<Rk4, R, C, ExplicitTest>(
                s,
                0,
                k_target,
                AzCount::new(),
                &<Q as AzConfig>::params(),
                cadence,
                &mut ExplicitTest,
            );
            (out.state, out.boundary, out.t, out.count.end)
        }
        Kind::LogH => {
            let out = lh::march::<Rk4, R, C, ExplicitTest>(
                s,
                k_target,
                &<Q as LhConfig>::params(),
                cadence,
                &mut ExplicitTest,
            );
            (out.state, out.boundary, out.t, out.count.end)
        }
    }
}

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────

/// Burrau's three bodies of masses 3, 4, 5 on their triangle, with small momenta summing to zero, in the gauge `M = 1`
/// (masses over 12, lengths over 5), as `tests/heggie_seam.rs` takes them: a close encounter before `t = 2`
/// ([`close_encounter`] measures it).
fn burrau<R: P>() -> State<R> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    State {
        m: m.map(R::of),
        r: r.map(|x| x.map(R::of)),
        p: p.map(|x: [f64; 2]| x.map(R::of)),
    }
}

/// The two-body radial collision (dd_validation_orbits § 2, the radial-collision gate's fixture): two masses 1/3 from
/// rest at `(±1/2, 0)` falling head-on, the third at rest at `(0, 20)`, centred on the centre of mass. The pair collides
/// near `t = 1.36`, between boundaries at each quantum (1.25 and 1.375 at 1/8; 1.3 and 1.4 at 0.1; 1.2 and 1.4 at 0.2).
fn radial<R: P>() -> State<R> {
    let m = [1.0 / 3.0; 3];
    let y = 20.0 / 3.0;
    let r = [[-0.5, -y], [0.5, -y], [0.0, 20.0 - y]];
    State {
        m: m.map(R::of),
        r: r.map(|x: [f64; 2]| x.map(R::of)),
        p: [[R::of(0.0); 2]; 3],
    }
}

/// The least pairwise separation of `s`, in f64.
fn separation<R: P>(s: &State<R>) -> f64 {
    let r = s.r.map(|x| x.map(Into::<f64>::into));
    let d = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    d(0, 1).min(d(0, 2)).min(d(1, 2))
}

/// The bits of a state's positions and momenta.
fn state_bits<R: P>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| b(x))
        .collect()
}

/// The bits of the state and of the carried buffer: the occupant's numbers, its step total, its re-registrations and
/// whether it registered.
fn bits<R: P>(s: &State<R>, c: &Carried<R>) -> Vec<u64> {
    let mut v = state_bits(s);
    v.extend(c.x.iter().map(|&x| b(x)));
    v.extend([c.steps, c.re_registrations, c.registered].map(u64::from));
    v
}

// ── Cadences ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A cadence that never ends the march, keeping the least separation it saw and the states at the sync boundaries the
/// march reported landing on ([`Cadence::sample`]).
#[derive(Default)]
struct Free {
    least: f64,
    landed: Vec<(u32, Vec<u64>)>,
}

impl Free {
    fn new() -> Self {
        Free {
            least: f64::INFINITY,
            landed: Vec::new(),
        }
    }
}

impl<R: P> Cadence<R> for Free {
    fn after_step(&mut self, state: &mut State<R>, _geom: &mut Geom<R>) {
        self.least = self.least.min(separation(state));
    }
    fn done(&self) -> bool {
        false
    }
    fn sample(&mut self, state: &State<R>, index: u32, _quantum: R) {
        self.landed.push((index, state_bits(state)));
    }
}

/// The collision radius the collision arm fires at, `r_coll`'s default, `10⁻³`.
const R_COLL: f64 = 1e-3;

/// A cadence that ends the march once a pair is inside [`R_COLL`] (Part 7's collision, here measured in f64 on the
/// state the step left), keeping the state it fired on. With `heed` false it keeps the firing state and never ends the
/// march (a control's: the copy carried on).
struct Colliding {
    heed: bool,
    fired: Option<Vec<u64>>,
}

impl Colliding {
    fn new() -> Self {
        Colliding {
            heed: true,
            fired: None,
        }
    }
}

impl<R: P> Cadence<R> for Colliding {
    fn after_step(&mut self, state: &mut State<R>, _geom: &mut Geom<R>) {
        if self.fired.is_none() && separation(state) < R_COLL {
            self.fired = Some(state_bits(state));
        }
    }
    fn done(&self) -> bool {
        self.heed && self.fired.is_some()
    }
}

// ── The seam ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A horizon: `T` and its macro-step, `dt_macro = max(10⁻³, T/65535)` (`engine::contract::sim_uniforms::
/// dt_macro_for`, whose one-ulp raise neither horizon here needs).
#[derive(Clone, Copy, Debug)]
struct Horizon {
    t: f64,
    dt: f64,
}

impl Horizon {
    fn of(t: f64) -> Self {
        Horizon {
            t,
            dt: (t / 65535.0).max(1e-3),
        }
    }
}

/// The two horizons the grid is checked across.
const HORIZONS: [f64; 2] = [50.0, 200.0];

/// One trajectory through the seam: its state, force evaluation, carried buffer and the macro-step index it stands at.
struct Seam<R> {
    state: State<R>,
    geom: Geom<R>,
    carried: Carried<R>,
    index: u32,
}

impl<R: P> Seam<R> {
    /// A trajectory at `t = 0`, its buffer unregistered.
    fn fresh(s: &State<R>) -> Self {
        Seam {
            state: *s,
            geom: Geom::new(&s.m, &s.r),
            carried: Carried::unregistered(),
            index: 0,
        }
    }

    /// One `ADVANCE` of `A` from where the trajectory stands to the macro-step `target` at `dt_macro = dt`.
    fn call<A: Advance, C: Cadence<R>>(
        &mut self,
        target: u32,
        dt: f64,
        cadence: &mut C,
    ) -> Reached {
        let params = AdvanceParams {
            dt_macro: R::of(dt),
            n_sub: 1,
        };
        let (out, reached) = A::advance_carrying(
            &self.state,
            &self.geom,
            self.index,
            target,
            &params,
            cadence,
            &mut self.carried,
        );
        self.state = out.state;
        self.geom = out.geom;
        self.index = target;
        reached
    }

    fn bits(&self) -> Vec<u64> {
        bits(&self.state, &self.carried)
    }
}

/// The least distance in physical time a target keeps from the grid, so that the f32 product `index·dt_macro` (its
/// `dt_macro` rounded once, the product once: under `10⁻⁶` at the times here) cannot fall on the other side of a
/// boundary from the f64 one. A fixture input, not a threshold.
const OFF_GRID: f64 = 1e-5;

/// The boundary the macro-step `index` at `dt` stands after on a grid of `delta`, decided in f64: `k` with `k·delta`
/// at least [`OFF_GRID`] before `index·dt` and `(k + 1)·delta` at least [`OFF_GRID`] after it. Panics, naming the
/// target, where `index·dt` is within [`OFF_GRID`] of a boundary: such a target is not off the grid.
fn off_grid(index: u32, dt: f64, delta: f64) -> u32 {
    let t = f64::from(index) * dt;
    let k = (t / delta).floor();
    let (below, above) = (t - k * delta, (k + 1.0) * delta - t);
    assert!(
        below >= OFF_GRID && above >= OFF_GRID,
        "the target {index} (t = {t}) is within {OFF_GRID} of the grid of {delta}: {below}, {above}"
    );
    k as u32
}

/// The least macro-step index whose time is at least [`OFF_GRID`] past `t`.
fn index_past(t: f64, dt: f64) -> u32 {
    let mut m = (t / dt).floor() as u32;
    while f64::from(m) * dt - t < OFF_GRID {
        m += 1;
    }
    m
}

/// A frame's increment of physical time, 23 ms (about 43 Hz): at `T = 50` its 23 macro-steps share no factor with the
/// 125 of 1/8, the 100 of 0.1 or the 200 of 0.2, so no target before `t = 2` lies on any of the grids ([`off_grid`]
/// checks each, at both horizons). A fixture input.
const FRAME: f64 = 0.023;

/// The macro-steps of the least increment at `dt` whose physical time is at least `t`.
fn increment(t: f64, dt: f64) -> u32 {
    (t / dt).ceil() as u32
}

/// The grid time the bitwise arms reach, past Burrau's close encounter at each quantum: boundary 16 at 1/8, 20 at 0.1,
/// 10 at 0.2.
const T_GRID: f64 = 2.0;

/// The boundary of the grid time `t` on a grid of `delta`, which it must be.
fn boundary_of(t: f64, delta: f64) -> u32 {
    let k = (t / delta).round();
    assert!(
        (k * delta - t).abs() < 1e-12,
        "{t} is no boundary of the grid of {delta}"
    );
    k as u32
}

/// The boundary states of one occupant at one horizon: each reached in one `ADVANCE` from `t = 0` to a target just
/// past it (catch-up), `0 ..= K`.
type Boundaries = Vec<Vec<u64>>;

/// Catch-up at the horizon `h`: for each boundary `k` of `delta` up to `t_grid`, one `ADVANCE` of `A` from `t = 0` to
/// the first macro-step past `k·delta`, off the grid. The state and the buffer at each.
fn catch_up<A: Advance, R: P>(s: &State<R>, h: Horizon, delta: f64, t_grid: f64) -> Boundaries {
    (0..=boundary_of(t_grid, delta))
        .map(|k| {
            let target = index_past(f64::from(k) * delta, h.dt);
            assert_eq!(off_grid(target, h.dt, delta), k);
            let mut seam = Seam::fresh(s);
            let reached = seam.call::<A, _>(target, h.dt, &mut Free::new());
            assert_eq!(reached, Reached::Target, "catch-up to boundary {k}");
            seam.bits()
        })
        .collect()
}

/// `seam`'s calls of `A`, each advancing the target by `n` macro-steps at `dt`, until a target stands after boundary
/// `last`, or would stand past it; each call's state and buffer checked against the catch-up's at the boundary at or before its target. `what`
/// names the arm in a failure.
fn march_by<A: Advance, R: P>(
    what: &str,
    seam: &mut Seam<R>,
    n: u32,
    h: Horizon,
    delta: f64,
    last: u32,
    reference: &Boundaries,
) {
    let mut go = true;
    while go {
        let target = seam.index + n;
        let k = off_grid(target, h.dt, delta);
        if k > last {
            break;
        }
        go = k < last;
        let reached = seam.call::<A, _>(target, h.dt, &mut Free::new());
        assert_eq!(reached, Reached::Target, "{what}: the call to {target}");
        assert!(
            seam.bits() == reference[k as usize],
            "{what}: the state at the target {target} (t = {}, boundary {k}) differs from the catch-up's at boundary \
             {k}: live march, catch-up and re-boot are not one march",
            f64::from(target) * h.dt
        );
    }
}

/// The march's own boundary states for `kind` on `Q`, from one direct march of `s` to boundary `last` (no seam), the
/// state at each boundary as the march reports landing there; `0` is `s`. Also the least separation it passed.
fn landings<Q: Quantum, R: P>(kind: Kind, s: &State<R>, last: u32) -> (Vec<Vec<u64>>, f64) {
    let mut free = Free::new();
    let (end, k, _, how) = direct::<Q, R, Free>(kind, s, last, &mut free);
    assert_eq!(
        (k, how),
        (last, End::Marching),
        "the direct march to {last}"
    );
    let mut out = vec![state_bits(s)];
    for (i, (index, bits)) in free.landed.iter().enumerate() {
        assert_eq!(*index as usize, i + 1, "the landings in order");
        out.push(bits.clone());
    }
    assert_eq!(out.last(), Some(&state_bits(&end)), "the last landing");
    (out, free.least)
}

/// `A` on `kind` and `Q`, at `R`, at the horizon `h`: catch-up, live march by a frame and by 0.29, and a re-boot
/// from `t = 0` after a pause, bitwise at every boundary to [`T_GRID`]; and each boundary state the direct march's.
/// Returns the catch-up's boundary states.
fn check_marches<A: Advance, Q: Quantum, R: P>(kind: Kind, h: Horizon) -> Boundaries {
    let s = burrau::<R>();
    let delta = Q::DELTA;
    let last = boundary_of(T_GRID, delta);
    let name = format!("{kind:?} × RK4, Δ_sync = {delta}, T = {}, {}", h.t, R::NAME);
    let reference = catch_up::<A, R>(&s, h, delta, T_GRID);
    let (frame, wide) = (increment(FRAME, h.dt), increment(0.29, h.dt));
    for n in [frame, wide] {
        assert!(
            (f64::from(n) * h.dt / delta).fract() != 0.0,
            "the increment {n} is a multiple of Δ_sync"
        );
        march_by::<A, R>(
            &format!("{name}, live by {n}"),
            &mut Seam::fresh(&s),
            n,
            h,
            delta,
            last,
            &reference,
        );
    }
    // The re-boot: live by frames to the pause, about halfway; the trajectory evicted, nothing it held read again; then
    // one call from `t = 0` to the pause's target and frames again from there.
    let mut live = Seam::fresh(&s);
    march_by::<A, R>(
        &format!("{name}, before the pause"),
        &mut live,
        frame,
        h,
        delta,
        last / 2,
        &reference,
    );
    let pause = live.index;
    let mut rebooted = Seam::fresh(&s);
    let reached = rebooted.call::<A, _>(pause, h.dt, &mut Free::new());
    assert_eq!(reached, Reached::Target, "{name}: the re-boot's first call");
    let k = off_grid(pause, h.dt, delta);
    assert!(
        rebooted.bits() == reference[k as usize],
        "{name}: the re-boot to {pause} differs from the catch-up's at boundary {k}: live march, catch-up and re-boot \
         are not one march"
    );
    march_by::<A, R>(
        &format!("{name}, re-booted"),
        &mut rebooted,
        frame,
        h,
        delta,
        last,
        &reference,
    );
    // Each catch-up state is the direct march's at its boundary: an off-grid target returns the boundary before it.
    let (own, least) = landings::<Q, R>(kind, &s, last);
    for (k, (seam, own)) in reference.iter().zip(&own).enumerate() {
        assert!(
            seam[..own.len()] == own[..],
            "{name}: one ADVANCE to just past boundary {k} does not return the direct march's state there"
        );
    }
    println!(
        "{name}: {} boundaries, live by {frame} and {wide} macro-steps and re-booted at {pause} bitwise; least \
         separation {least:.3e}",
        last + 1
    );
    reference
}

/// [`check_marches`] at both horizons, for one occupant on one quantum at one precision.
fn check_both<A: Advance, Q: Quantum, R: P>(kind: Kind) {
    for t in HORIZONS {
        check_marches::<A, Q, R>(kind, Horizon::of(t));
    }
}

/// The least separation Burrau's fixture must pass below to be a close encounter: `r_close`'s default, 0.01, the
/// wrapper's close-encounter counting distance. Fixed before the run.
const CLOSE: f64 = 0.01;

/// Burrau's fixture passes below [`CLOSE`] before [`T_GRID`] (measured on the direct Heggie march at f64).
fn close_encounter() {
    let last = boundary_of(T_GRID, DELTA_SYNC);
    let (_, least) = landings::<Register, f64>(Kind::Heggie, &burrau(), last);
    println!("Burrau's fixture: least separation {least:.3e} before t = {T_GRID}");
    assert!(
        least < CLOSE,
        "Burrau's fixture passes no closer than {least}, not a close encounter"
    );
}

/// Every occupant, quantum and precision through [`check_both`].
fn check_all() {
    close_encounter();
    check_both::<Hg<Register>, Register, f64>(Kind::Heggie);
    check_both::<Hg<Register>, Register, f32>(Kind::Heggie);
    check_both::<Hg<Tenth>, Tenth, f64>(Kind::Heggie);
    check_both::<Hg<Tenth>, Tenth, f32>(Kind::Heggie);
    check_both::<Hg<Fifth>, Fifth, f64>(Kind::Heggie);
    check_both::<Hg<Fifth>, Fifth, f32>(Kind::Heggie);
    check_both::<Az<Register>, Register, f64>(Kind::AarsethZare);
    check_both::<Az<Register>, Register, f32>(Kind::AarsethZare);
    check_both::<Az<Tenth>, Tenth, f64>(Kind::AarsethZare);
    check_both::<Az<Tenth>, Tenth, f32>(Kind::AarsethZare);
    check_both::<Az<Fifth>, Fifth, f64>(Kind::AarsethZare);
    check_both::<Az<Fifth>, Fifth, f32>(Kind::AarsethZare);
    check_both::<Lh<Register>, Register, f64>(Kind::LogH);
    check_both::<Lh<Register>, Register, f32>(Kind::LogH);
    check_both::<Lh<Tenth>, Tenth, f64>(Kind::LogH);
    check_both::<Lh<Tenth>, Tenth, f32>(Kind::LogH);
    check_both::<Lh<Fifth>, Fifth, f64>(Kind::LogH);
    check_both::<Lh<Fifth>, Fifth, f32>(Kind::LogH);
}

#[test]
fn sync_quantum_bitwise() {
    assert_eq!(
        sync::delta_sync::<f64>(),
        0.125,
        "the register's Δ_sync, the proposal's"
    );
    check_all();
}

/// Heggie on `Q` with each call's interval sized from its own target: the march lands on every frame target, its steps
/// sized from the call's span, not from `Δ_sync`. Never an occupant; a control's.
#[cfg(feature = "controls")]
struct FrameSized<Q>(core::marker::PhantomData<Q>);

#[cfg(feature = "controls")]
impl<Q: Quantum> Advance for FrameSized<Q> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let mut none = Carried::unregistered();
        Self::advance_carrying(state, geom, t_now, t_target, params, cadence, &mut none).0
    }

    fn advance_carrying<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        _geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
        carried: &mut Carried<R>,
    ) -> (Advanced<R>, Reached) {
        let span = R::count(t_target) * params.dt_macro - R::count(t_now) * params.dt_macro;
        let values = HgParams {
            delta_sync: span,
            ..<Q as HgConfig>::params::<R>()
        };
        if carried.registered == 0 {
            let fresh = hg::register(state, values.tiny, values.time);
            hg::carry(carried, &fresh.q, &fresh.p, fresh.system.h, 0);
        }
        let (q, p, h) = hg::carried_state(carried);
        if t_target == t_now {
            let geom = Geom::new(&state.m, &state.r);
            return (
                Advanced {
                    state: *state,
                    geom,
                    substeps: 0,
                },
                Reached::Target,
            );
        }
        let system = hg::HgSystem::new(state.m, h, values.tiny, values.time);
        let from = hg::HgStanding {
            q,
            p,
            boundary: 0,
            count: hg::HgCount {
                steps: carried.steps,
                ..hg::HgCount::new()
            },
        };
        let out = hg::march_on::<Rk4, R, C, ExplicitTest>(
            &system,
            state,
            from,
            1,
            &values,
            cadence,
            &mut ExplicitTest,
        );
        hg::carry(carried, &out.q, &out.p, h, out.count.steps);
        (
            Advanced {
                state: out.state,
                geom: out.geom,
                substeps: out.count.accepted,
            },
            Reached::Target,
        )
    }
}

negative_control!(
    sync_quantum_bitwise,
    "a march that sizes its steps from each call's target, landing on the off-grid frame targets, must differ from \
     the catch-up and fail",
    expected = "live march, catch-up and re-boot are not one march",
    {
        check_marches::<FrameSized<Register>, Register, f64>(Kind::Heggie, Horizon::of(50.0));
    }
);

// ── The horizons ───────────────────────────────────────────────────────────────────────────────────────────────

/// `A50` at `T = 50` on the quantum `delta50` and `A200` at `T = 200` on `delta200`, at `R`: the catch-up's state at
/// every time that is a boundary of both grids, up to [`T_GRID`], bitwise the same. `A50` and `A200` are one occupant
/// unless a control's grid follows the horizon.
fn check_horizons<A50: Advance, A200: Advance, R: P>(name: &str, (delta50, delta200): (f64, f64)) {
    let s = burrau::<R>();
    let at50 = catch_up::<A50, R>(&s, Horizon::of(50.0), delta50, T_GRID);
    let at200 = catch_up::<A200, R>(&s, Horizon::of(200.0), delta200, T_GRID);
    let mut common = 0;
    for (k, state) in at50.iter().enumerate() {
        let t = k as f64 * delta50;
        let j = (t / delta200).round();
        if (j * delta200 - t).abs() < 1e-12 {
            common += 1;
            assert!(
                *state == at200[j as usize],
                "{name}, {}: the state at t = {t} differs between the horizons T = 50 and T = 200",
                R::NAME
            );
        }
    }
    assert!(common > 1, "{name}: no common time past 0");
    println!(
        "{name}, {}: {common} boundary states bitwise the same at T = 50 (dt_macro {}) and T = 200 (dt_macro {})",
        R::NAME,
        Horizon::of(50.0).dt,
        Horizon::of(200.0).dt
    );
}

/// [`check_horizons`] for one occupant on one quantum at both precisions.
fn check_horizons_both<A: Advance>(name: &str, delta: f64) {
    check_horizons::<A, A, f64>(name, (delta, delta));
    check_horizons::<A, A, f32>(name, (delta, delta));
}

#[test]
fn sync_quantum_bitwise_horizons() {
    check_horizons_both::<Hg<Register>>("Heggie × RK4, Δ_sync = 1/8", Register::DELTA);
    check_horizons_both::<Hg<Tenth>>("Heggie × RK4, Δ_sync = 0.1", Tenth::DELTA);
    check_horizons_both::<Hg<Fifth>>("Heggie × RK4, Δ_sync = 0.2", Fifth::DELTA);
    check_horizons_both::<Az<Register>>("Aarseth–Zare × RK4, Δ_sync = 1/8", Register::DELTA);
    check_horizons_both::<Az<Tenth>>("Aarseth–Zare × RK4, Δ_sync = 0.1", Tenth::DELTA);
    check_horizons_both::<Az<Fifth>>("Aarseth–Zare × RK4, Δ_sync = 0.2", Fifth::DELTA);
    check_horizons_both::<Lh<Register>>("logH × RK4, Δ_sync = 1/8", Register::DELTA);
    check_horizons_both::<Lh<Tenth>>("logH × RK4, Δ_sync = 0.1", Tenth::DELTA);
    check_horizons_both::<Lh<Fifth>>("logH × RK4, Δ_sync = 0.2", Fifth::DELTA);
}

negative_control!(
    sync_quantum_bitwise_horizons,
    "a grid of t_max / n_sync, 1/8 at T = 50 and 1/2 at T = 200, must give different states at a common time and fail",
    expected = "differs between the horizons",
    check_horizons::<Hg<PerSync50>, Hg<PerSync200>, f64>(
        "Heggie × RK4, Δ_sync = T/400",
        (PerSync50::DELTA, PerSync200::DELTA),
    )
);

// ── A collision between boundaries ─────────────────────────────────────────────────────────────────────────────

/// How a march that collided ended: its state's bits, the boundary it stood after, its `t_end`, and the state the
/// cadence fired on.
struct Collided {
    state: Vec<u64>,
    boundary: u32,
    t_end: f64,
    fired: Vec<u64>,
}

/// The collided copy is latched at the firing step: its state is the one the cadence fired on, and its `t_end` is that
/// step's time, strictly between the boundary before it and the one after on the grid of `delta`.
fn check_latched(name: &str, c: &Collided, delta: f64) {
    assert!(
        c.state == c.fired,
        "{name}: the collided copy is not latched at the firing step"
    );
    let (before, after) = (
        f64::from(c.boundary) * delta,
        f64::from(c.boundary + 1) * delta,
    );
    assert!(
        before < c.t_end && c.t_end < after,
        "{name}: t_end {} is not the firing step's time, strictly between the boundaries {before} and {after}",
        c.t_end
    );
}

/// The direct march of the radial collision on `Q` under `kind` at `R`, to a boundary well past the collision, its
/// cadence heeded (`heed`) or not; with `heed` false the march stops at boundary `stop`.
fn collided<Q: Quantum, R: P>(kind: Kind, heed: bool, stop: u32) -> Collided {
    let mut cadence = Colliding::new();
    cadence.heed = heed;
    let (state, boundary, t_end, end) =
        direct::<Q, R, Colliding>(kind, &radial(), stop, &mut cadence);
    if heed {
        assert_eq!(end, End::Done, "the direct march ended by the collision");
    }
    Collided {
        state: state_bits(&state),
        boundary,
        t_end: t_end.into(),
        fired: cadence.fired.expect("the pair collided"),
    }
}

/// The boundary the collision arm marches to, past the collision near `t = 1.36` at each quantum: `t = 2`.
fn past(delta: f64) -> u32 {
    boundary_of(T_GRID, delta)
}

/// The radial collision on `Q` under `kind` at `R` through the seam of `A` at the horizon `h`: one `ADVANCE` to
/// [`T_GRID`] (catch-up), calls by a frame (live), and a re-boot from `t = 0` after a pause before the collision
/// then frames again, each until a call reports the march short. The state and buffer bitwise the same in the three,
/// each the state its cadence fired on, and the direct march's latched one.
fn check_collision<A: Advance, Q: Quantum, R: P>(kind: Kind, h: Horizon) {
    let delta = Q::DELTA;
    let name = format!("{kind:?} × RK4, Δ_sync = {delta}, T = {}, {}", h.t, R::NAME);
    let s = radial::<R>();
    let end = index_past(T_GRID, h.dt);
    let mut one = Seam::fresh(&s);
    let mut cadence = Colliding::new();
    let reached = one.call::<A, _>(end, h.dt, &mut cadence);
    assert_eq!(
        reached,
        Reached::Short,
        "{name}: catch-up stopped by the collision"
    );
    let catch_up = (
        one.bits(),
        cadence.fired.clone().expect("the pair collided"),
    );
    // Live and re-booted: calls by a frame until one reports the march short; `from` the re-boot's one first call.
    let by_frames = |mut seam: Seam<R>, mut cadence: Colliding| {
        let n = increment(FRAME, h.dt);
        // As the wrapper's loop: no call once the cadence is done.
        while !Cadence::<R>::done(&cadence) {
            assert!(seam.index < end, "{name}: no collision before t = {T_GRID}");
            let reached = seam.call::<A, _>(seam.index + n, h.dt, &mut cadence);
            assert!(
                reached == Reached::Target || Cadence::<R>::done(&cadence),
                "{name}: a call ended {reached:?} with no collision"
            );
        }
        (seam.bits(), cadence.fired.expect("the pair collided"))
    };
    let live = by_frames(Seam::fresh(&s), Colliding::new());
    let pause = index_past(0.7, h.dt);
    let mut rebooted = Seam::fresh(&s);
    let mut cadence = Colliding::new();
    assert_eq!(
        rebooted.call::<A, _>(pause, h.dt, &mut cadence),
        Reached::Target,
        "{name}: the pause is before the collision"
    );
    let rebooted = by_frames(rebooted, cadence);
    for (arm, (bits, fired)) in [("live", &live), ("re-booted", &rebooted)] {
        assert!(
            *bits == catch_up.0,
            "{name}: the {arm} march's latched state differs from the catch-up's"
        );
        assert!(
            *fired == catch_up.1,
            "{name}: the {arm} march fired on another state"
        );
    }
    let direct = collided::<Q, R>(kind, true, past(delta));
    let (t_end, boundary, state_len) = (direct.t_end, direct.boundary, direct.state.len());
    assert!(
        catch_up.0[..state_len] == direct.state[..],
        "{name}: ADVANCE does not return the direct march's latched state"
    );
    check_latched(
        &name,
        &Collided {
            state: catch_up.0[..state_len].to_vec(),
            fired: catch_up.1.clone(),
            ..direct
        },
        delta,
    );
    println!(
        "{name}: latched at t_end = {:.6} (boundaries {} and {}), live, catch-up and re-boot bitwise",
        t_end,
        f64::from(boundary) * delta,
        f64::from(boundary + 1) * delta
    );
}

/// [`check_collision`] for one occupant on one quantum at both precisions and both horizons.
fn check_collision_all<A: Advance, Q: Quantum>(kind: Kind) {
    for t in HORIZONS {
        check_collision::<A, Q, f64>(kind, Horizon::of(t));
        check_collision::<A, Q, f32>(kind, Horizon::of(t));
    }
}

#[test]
fn sync_quantum_bitwise_collision() {
    check_collision_all::<Hg<Register>, Register>(Kind::Heggie);
    check_collision_all::<Hg<Tenth>, Tenth>(Kind::Heggie);
    check_collision_all::<Hg<Fifth>, Fifth>(Kind::Heggie);
    check_collision_all::<Az<Register>, Register>(Kind::AarsethZare);
    check_collision_all::<Az<Tenth>, Tenth>(Kind::AarsethZare);
    check_collision_all::<Az<Fifth>, Fifth>(Kind::AarsethZare);
    check_collision_all::<Lh<Register>, Register>(Kind::LogH);
    check_collision_all::<Lh<Tenth>, Tenth>(Kind::LogH);
    check_collision_all::<Lh<Fifth>, Fifth>(Kind::LogH);
}

negative_control!(
    sync_quantum_bitwise_collision,
    "a march that carries the collided copy on to the next boundary must not read as latched and fail",
    expected = "is not latched at the firing step",
    {
        let at = collided::<Register, f64>(Kind::Heggie, true, past(Register::DELTA));
        let on = collided::<Register, f64>(Kind::Heggie, false, at.boundary + 1);
        check_latched("Heggie, carried on", &on, Register::DELTA);
    }
);

/// [`check_latched`]'s `t_end` arm on the direct marches: each kind, quantum and precision.
fn check_t_end(hold_back: bool) {
    let each = |kind: Kind, delta: f64, c: Collided, precision: &str| {
        let c = if hold_back {
            Collided {
                t_end: f64::from(c.boundary) * delta,
                ..c
            }
        } else {
            c
        };
        let name = format!("{kind:?} × RK4, Δ_sync = {delta}, {precision}");
        check_latched(&name, &c, delta);
        println!(
            "{name}: t_end = {:.9}, after boundary {}",
            c.t_end, c.boundary
        );
    };
    for kind in [Kind::Heggie, Kind::AarsethZare, Kind::LogH] {
        let (r, t) = (Register::DELTA, Tenth::DELTA);
        each(
            kind,
            r,
            collided::<Register, f64>(kind, true, past(r)),
            "f64",
        );
        each(
            kind,
            r,
            collided::<Register, f32>(kind, true, past(r)),
            "f32",
        );
        each(kind, t, collided::<Tenth, f64>(kind, true, past(t)), "f64");
        each(kind, t, collided::<Tenth, f32>(kind, true, past(t)), "f32");
        let f = Fifth::DELTA;
        each(kind, f, collided::<Fifth, f64>(kind, true, past(f)), "f64");
        each(kind, f, collided::<Fifth, f32>(kind, true, past(f)), "f32");
    }
}

#[test]
fn sync_quantum_bitwise_collision_t_end() {
    check_t_end(false);
}

negative_control!(
    sync_quantum_bitwise_collision_t_end,
    "a t_end held back to the boundary before the collision must not read as the firing step's time and fail",
    expected = "is not the firing step's time",
    check_t_end(true)
);

// ── The step budget across calls ───────────────────────────────────────────────────────────────────────────────

/// The step total [`Tight`] allows: more than any one interval of Burrau's march takes and fewer than the march to
/// [`T_GRID`] takes together, both of which [`check_budget`] asserts, so that only a total carried across calls spends
/// it where the one call does. A fixture input, not a threshold.
const TIGHT_STEPS: u32 = 1000;

/// [`Register`]'s values on a budget of [`TIGHT_STEPS`] steps at rate 0.
struct Tight;

impl Quantum for Tight {
    const DELTA: f64 = DELTA_SYNC;
}

impl HgConfig for Tight {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            n_min: TIGHT_STEPS,
            step_budget_rate: R::default(),
            ..hg_values(DELTA_SYNC)
        }
    }
}

impl AzConfig for Tight {
    fn params<R: AzReal>() -> AzParams<R> {
        AzParams {
            n_min: TIGHT_STEPS,
            step_budget_rate: R::default(),
            ..az_values(DELTA_SYNC)
        }
    }
}

impl LhConfig for Tight {
    fn params<R: AzReal>() -> LhParams<R> {
        LhParams {
            n_min: TIGHT_STEPS,
            step_budget_rate: R::default(),
            ..lh_values(DELTA_SYNC)
        }
    }
}

/// `A` on [`Tight`] at `R`, `T = 50`: one `ADVANCE` to [`T_GRID`] (catch-up) runs out of budget after its first
/// interval; live calls by a frame, stopped at the first that reports the budget spent, end bitwise where it did.
fn check_budget<A: Advance, R: P>(name: &str) {
    let h = Horizon::of(50.0);
    let s = burrau::<R>();
    let mut one = Seam::fresh(&s);
    let reached = one.call::<A, _>(index_past(T_GRID, h.dt), h.dt, &mut Free::new());
    assert_eq!(
        reached,
        Reached::Exhausted,
        "{name}: the budget binds before t = {T_GRID}"
    );
    let n = increment(FRAME, h.dt);
    let mut live = Seam::fresh(&s);
    let mut reached = Reached::Target;
    let end = index_past(T_GRID, h.dt);
    while reached == Reached::Target && live.index + n < end {
        reached = live.call::<A, _>(live.index + n, h.dt, &mut Free::new());
    }
    let after = off_grid(live.index, h.dt, DELTA_SYNC);
    assert!(
        after > 1,
        "{name}: the budget ran out in the first interval, at boundary {after}"
    );
    println!(
        "{name}: catch-up and live ({n} macro-steps a call) both {reached:?}, live in its call to boundary {after}; \
         live bitwise {}",
        live.bits() == one.bits()
    );
    assert_eq!(
        reached,
        Reached::Exhausted,
        "{name}: the live march ended {reached:?}"
    );
    assert!(
        live.bits() == one.bits(),
        "{name}: the live march spends its budget elsewhere than the catch-up: the step total is not carried"
    );
}

#[test]
fn sync_quantum_bitwise_budget() {
    check_budget::<Hg<Tight>, f64>("Heggie × RK4, f64");
    check_budget::<Lh<Tight>, f64>("logH × RK4, f64");
    check_budget::<Az<Tight>, f64>("Aarseth–Zare × RK4, f64");
    check_budget::<Hg<Tight>, f32>("Heggie × RK4, f32");
    check_budget::<Lh<Tight>, f32>("logH × RK4, f32");
    check_budget::<Az<Tight>, f32>("Aarseth–Zare × RK4, f32");
}

/// `A` with the running step total dropped from the buffer before each call, so each call's budget starts afresh. Never
/// an occupant; a control's (Aarseth–Zare's seam before it carried the total).
#[cfg(feature = "controls")]
struct Forgetful<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for Forgetful<A> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, cadence)
    }

    fn advance_carrying<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
        carried: &mut Carried<R>,
    ) -> (Advanced<R>, Reached) {
        carried.steps = 0;
        A::advance_carrying(state, geom, t_now, t_target, params, cadence, carried)
    }
}

negative_control!(
    sync_quantum_bitwise_budget,
    "a seam that starts its step total afresh at each call never spends the budget the one call spends, and must fail",
    expected = "the live march ended Target",
    check_budget::<Forgetful<Az<Tight>>, f64>("Aarseth–Zare × RK4, the total dropped")
);
