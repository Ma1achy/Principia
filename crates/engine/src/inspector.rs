//! The Inspector, engine-side: one IC, clicked, integrated by the shared kernel at f64 on the CPU, with its independent
//! RK45 reference beside it (philosophy, "The IC inspector — the prebake standard at a single point"; trajectory_viewing
//! §1, "The single mechanism"). One type, [`Inspector`], serves both the IC Inspector and the trajectory viewer: they
//! are two readings of one [`Inspection`], the IC Inspector's the fate, the time and the reference's divergence, the
//! trajectory viewer's the full state at every macro-step projected into real space ([`Inspection::bodies`]). No
//! window hosts it here; the GUI's Inspector window is a later task.
//!
//! **The mechanism** (trajectory_viewing §1): a click's canvas position through the convention's one flip to the
//! post-flip UV ([`crate::picking::pick`]), the chart decoded there at f64 and integrated by the Precision ring's own
//! march ([`crate::precision`]), the wrapper's `integrate_detecting` at f64, bit for bit the `SimState`
//! [`crate::precision::compute_ic`] returns. The full state is recorded at each of the march's own sampling points
//! (`kernel::driver::advance::Cadence::sample`): every completed macro-step under a stepper, every sync boundary landed
//! on under an occupant that owns its time mapping (Heggie × RK4), each at its physical time `index · quantum`, by a
//! cadence that wraps the wrapper's own, forwards every call to it and changes nothing.
//!
//! **The reference** ([`crate::precision::rk45`]): RK45 in f64, adaptive, from the same IC to each of the path's
//! times, the same instants as the kernel's path (pitfalls §1: compare at a common time, and
//! a run that ended early is compared only to where it ended). It is not an occupant and never writes the `SimState`.
//! Their difference is reported, never asserted: a different integrator family diverges on a long or chaotic run when
//! both are right (integrator_contract Part 6).
//!
//! **Domain** (pitfalls §10): one IC, the chart, the sim key, the occupant and the reference's tolerances held fixed.

use kernel::chart::ChartDecode;
use kernel::decode::{Degenerate, Ic};
use kernel::detect::t0::t0;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::{integrate_with, monitored};
use kernel::generated::links::Selection as Links;
use kernel::occupant::{Geom, State};
use kernel::payload::{tm_t_end_step, SimStateBaseOf, STATE_DECODE_FAILED};
use kernel::stage::decode::DecodeOut;

use crate::compute::ComputeError;
use crate::contract::sim_config::SimConfig;
use crate::picking::{pick, Screen};
use crate::precision::rk45::{self, Rk45Params, Rk45Run};
use crate::precision::{
    built_ic, decode_only, integrate_ic, march_key, state_of, MarchKey64, SimState64,
};

/// The Inspector: a chart, the sim key it is read under, and the reference's tolerances. One type for the IC Inspector
/// and the trajectory viewer (the module's note).
#[derive(Clone, Debug, PartialEq)]
pub struct Inspector<C> {
    /// The chart the clicks land on.
    pub chart: C,
    /// The sim key: its horizon bounds the march.
    pub sim_key: SimConfig,
    /// The RK45 reference's tolerances and budget.
    pub reference: Rk45Params,
}

/// One inspected IC: the decode, the Precision ring's `SimState`, the kernel's path and the reference's.
#[derive(Clone, Debug, PartialEq)]
pub struct Inspection {
    /// The chart point inspected, post-flip, Y-up; `None` for an IC given as its state.
    pub uv: Option<[f64; 2]>,
    /// The decode at f64, its `ICDescriptor` and any `DEGENERATE` tag; `None` for an IC given as its state.
    pub decoded: Option<DecodeOut<f64>>,
    /// The IC marched, `(m, r, p)`.
    pub ic: Ic<f64>,
    /// The `SimState` the march ended in, [`crate::precision::compute_ic`]'s bit for bit.
    pub sim_state: SimState64,
    /// The macro-step, `t_end_step · dt_macro` the time the march ended at.
    pub dt_macro: f64,
    /// The physical time of each state of the path: 0, then each sampling point's `index · quantum`.
    pub times: Vec<f64>,
    /// The kernel's state at each of `times`: the IC, then the state at each sampling point the march reached
    /// (the module's note).
    pub path: Vec<State<f64>>,
    /// The RK45 reference from the same IC at the same `times`.
    pub reference: Rk45Run,
}

impl Inspection {
    /// The completed macro-steps, `t_end_step`.
    pub fn t_end_step(&self) -> u32 {
        tm_t_end_step(self.sim_state.times)
    }

    /// `t_end_step · dt_macro`: the macro-step grid's time the wrapper counted the march to. Under a stepper that
    /// reaches its horizon it is the last state's time; a terminal mid-macro-step leaves the `SimState` past the
    /// path's last state, and under an occupant that owns its time mapping the march stands on the sync boundary at or
    /// before it, so it can be up to one `Δ_sync` past `times.last()`, the last state's own time.
    pub fn t_end(&self) -> f64 {
        f64::from(self.t_end_step()) * self.dt_macro
    }

    /// The kernel's path in real space, body `k`'s positions at index `k`, one per state of the path (each completed
    /// macro-step under a stepper, each sync boundary under an occupant that owns its time mapping): the trajectory
    /// viewer's projection.
    pub fn bodies(&self) -> [Vec<[f64; 2]>; 3] {
        [0, 1, 2].map(|k| self.path.iter().map(|s| s.r[k]).collect())
    }

    /// The kernel's distance from the reference at each of `times` the reference reached: the largest absolute
    /// difference over every position component, a NaN where any component's is (never dropped: [`nan_max`]).
    pub fn divergence(&self) -> Vec<f64> {
        self.path
            .iter()
            .zip(&self.reference.states)
            .map(|(a, b)| {
                (0..3)
                    .flat_map(|i| (0..2).map(move |j| (i, j)))
                    .map(|(i, j)| (a.r[i][j] - b.r[i][j]).abs())
                    .fold(0.0, nan_max)
            })
            .collect()
    }

    /// The largest `|E − E₀|` along each path, the kernel's and the reference's, both measured with the reference's
    /// own energy (`rk45::energy`), `E₀` the IC's. `None` where nothing was measured: an IC whose `E₀` is not finite
    /// (a failed decode, a NaN state, two coincident bodies), or a path with no state past the IC (the reference
    /// stopped at its start). A non-finite `|E − E₀|` along a path is kept, as a NaN or `∞`, never dropped.
    pub fn energy_drift(&self) -> (Option<f64>, Option<f64>) {
        let e0 = rk45::energy(&state_of(&self.ic));
        let drift = |states: &[State<f64>]| {
            (e0.is_finite() && states.len() > 1).then(|| {
                states
                    .iter()
                    .map(|s| (rk45::energy(s) - e0).abs())
                    .fold(0.0, nan_max)
            })
        };
        (drift(&self.path), drift(&self.reference.states))
    }
}

/// The larger of `a` and `b`, a NaN where either is: `f64::max` returns the other operand for a NaN, which in a
/// reduction is a silent discard that reads as better than the data.
pub fn nan_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// The wrapper's own cadence, with the state at each of its sampling points recorded: every call forwarded to it,
/// so the march and its `SimState` are the wrapper's, bit for bit.
struct Recorder {
    inner: WrapperCadence<f64>,
    times: Vec<f64>,
    path: Vec<State<f64>>,
}

impl Cadence<f64> for Recorder {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
        self.times.push(f64::from(index) * quantum);
        self.path.push(*state);
    }

    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }

    fn regularised_drift(&mut self, drift: f64) {
        self.inner.regularised_drift(drift);
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    fn resume(&mut self, stored: &SimStateBaseOf<f64>) {
        self.inner.resume(stored);
    }

    fn latched(&mut self) {
        self.inner.latched();
    }

    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64> {
        self.inner.finish(out)
    }
}

impl<C> Inspector<C>
where
    C: ChartDecode,
    f64: From<C::Param>,
{
    /// The click at the canvas position `at` (Y-down from the top-left, as a pointer event gives it) on `screen`: its
    /// post-flip UV ([`pick`]), then [`Inspector::inspect`] there. `None` outside the canvas.
    pub fn click<S: Links, A: Advance>(
        &self,
        screen: &Screen,
        at: [f64; 2],
    ) -> Result<Option<Inspection>, ComputeError> {
        match pick(screen, at) {
            Some(p) => self.inspect::<S, A>(p.uv).map(Some),
            None => Ok(None),
        }
    }

    /// The IC at `uv` under the link selection `S`, integrated by the occupant `A` at f64 (the module's note).
    pub fn inspect<S: Links, A: Advance>(&self, uv: [f64; 2]) -> Result<Inspection, ComputeError> {
        let decoded = decode_only::<C, S>(&self.chart, uv, &self.sim_key)?;
        let mut out = self.run::<A>(&decoded.ic, decoded.degenerate)?;
        out.uv = Some(uv);
        out.decoded = Some(decoded);
        Ok(out)
    }
}

impl<C> Inspector<C> {
    /// An IC given as its state, `(m, r, p)` in the canonical gauge, inspected as [`Inspector::inspect`] inspects a
    /// decoded one: parity_contract §1's `computeICState`, with the path and the reference.
    pub fn inspect_state<A: Advance>(&self, ic: &Ic<f64>) -> Result<Inspection, ComputeError> {
        self.run::<A>(ic, None)
    }

    /// A state the IC constructor built, `as_built`, inspected from `C(as_built)` ([`built_ic`]): the march
    /// [`crate::precision::compute_built`] runs, its `SimState` bit for bit, with the path and the RK45 reference from
    /// the same canonical state. [`Inspection::ic`] is the state integrated.
    pub fn inspect_built<A: Advance>(
        &self,
        as_built: &Ic<f64>,
    ) -> Result<Inspection, ComputeError> {
        self.run::<A>(&built_ic(as_built), None)
    }

    /// The march of `ic`, tagged `degenerate` or not, with its path and the reference (the module's note).
    fn run<A: Advance>(
        &self,
        ic: &Ic<f64>,
        degenerate: Option<Degenerate>,
    ) -> Result<Inspection, ComputeError> {
        let key = march_key(&self.sim_key)?;
        let (sim_state, times, path) = march::<A>(ic, degenerate, &key);
        let reference = rk45::integrate(&state_of(ic), &times, &self.reference);
        Ok(Inspection {
            uv: None,
            decoded: None,
            ic: *ic,
            sim_state,
            dt_macro: key.loop_values.dt_macro,
            times,
            path,
            reference,
        })
    }
}

/// The Precision ring's march of `ic` with its path: [`integrate_ic`]'s `SimState`, from the same wrapper call with the
/// recorder wrapping its cadence, and the times and states of the path (the module's note). An IC labelled
/// `decode_failed` is not marched; its path is the IC at time 0.
fn march<A: Advance>(
    ic: &Ic<f64>,
    degenerate: Option<Degenerate>,
    key: &MarchKey64,
) -> (SimState64, Vec<f64>, Vec<State<f64>>) {
    if t0(ic, degenerate, &key.radius).state == STATE_DECODE_FAILED {
        return (
            integrate_ic::<A>(ic, degenerate, key),
            vec![0.0],
            vec![state_of(ic)],
        );
    }
    let mut recorder = Recorder {
        inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, key.radius)
            .stopping_on_escape(key.stop_on_escape),
        times: vec![0.0],
        path: vec![state_of(ic)],
    };
    let out = integrate_with::<A, f64, _>(ic, &key.loop_values, &mut recorder);
    let sim_state = monitored(out, &recorder.inner);
    (sim_state, recorder.times, recorder.path)
}
