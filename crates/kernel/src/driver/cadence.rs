//! The wrapper's per-substep cadence (integrator_contract Part 1, "Cadence (pinned)"; Part 2a, Cost 1; R-19): what runs
//! after every internal step, inside the occupant's march, as the callback [`WrapperCadence`] the wrapper passes in.
//!
//! **In order, after each `STEP`** (Part 1's loop): the failure path's read of the state the step left
//! ([`super::failure::Detector`]), the COM projection ([`WrapperCadence::project`], with the force evaluation re-formed
//! at the projected positions), the failure path's read of the projected state, the invariant accumulation on it
//! ([`WrapperCadence::monitor`]; TASK-M3-05) and the failure path's read of its energy and `L_z`, and terminal
//! detection, which sets the `done` flag on collision ([`Detector`]) and never breaks. A failure sets `done` too: the
//! march ends on its first non-finite step (`super::failure`). **At each sampling point** of the occupant's grid ([`Cadence::sample`]: a completed
//! macro-step, or a sync boundary under an occupant that owns its time mapping) the escape detector runs
//! ([`crate::detect::escape::Detector`]; Part 7), on the state the step's cadence left, and sets `done` where it fires. A run on which
//! a collision has fired takes no escape test: the collision's is the earlier step.
//!
//! **Built from the run's IC** ([`WrapperCadence::with_radius`]): the projection's `M` and weights are formed once,
//! there ([`ComProjection`]; Part 1, "with `M` constant (computed once and cached)"), `E₀`, `L_{z,0}` are evaluated
//! and stored, before the loop ([`Monitor`]), and the detectors run on the IC, before the first step, so a run that
//! begins inside `r_coll` is done at step 0, a collision (integrator_contract Part 5, "t=0 terminals are real
//! outcomes"); the escape detector's lagged shape register is set to the IC's shape and its `E_0` is the monitor's,
//! and there is no escape at `t = 0`. The wrapper writes the monitor's references and maxima, and the detector's label and `dmin_pair`, into
//! the run's `SimState` when the loop ends (`super::wrapper`).
//!
//! **The terminals' times** ([`WrapperCadence::events`]): each detector's firing is recorded with the cadence's call
//! count at that moment, the ordinal of the internal step whose state it read (0 for the IC), so the label writer
//! orders them by time and settles a same-state tie ([`crate::detect::label`]). The escape test runs at a sampling
//! point whose own step fired a collision, the one state the two can share; on any later point the collision is earlier
//! and ended the run.
//!
//! **`t_dmin_step`** ([`WrapperCadence::dmin_step`]; payload §2, "absolute macro-step index of closest approach"): the
//! wrapper reports its completed macro-steps before each macro-step and after the last ([`Cadence::clock`]). A `d_min`
//! latched mid-macro-step takes the completed count then, the macro-step it lies in; one latched on the step that
//! completes a macro-step takes the count that step completes, the grid point it reached, which is `closure_step`'s
//! convention (`super::closure`), so `t_dmin_step` never exceeds `t_end_step`. The plan is unclear here (payload §2
//! gives "the macro-step index" without a rule for a minimum between two grid points); chose the grid point at or before
//! the minimum because a collision, whose own step holds the least `d²`, then has `t_dmin_step = t_end_step`, as the
//! engine's fixtures state it (`engine::fixtures::Known::t_dmin_step`), and the IC's latch is step 0.
//!
//! **A re-boot** ([`Cadence::resume`]): the wrapper hands a resumed march's stored `SimState` to its cadence before the
//! first step, and this cadence reloads the escape detector's lagged register and `E_0` from it, so a re-booted march
//! tests escape as the uncut one does without the caller remembering to. The monitor's drift maxima and references and
//! the collision detector's `d_min` latch are still the caller's to resume.
//!
//! **One `d²` per step** (Part 7; REQ-EVT-004): the detector forms the projected state's pair separations once, and
//! the collision test, the `dmin_pair` latch and the next macro-step's bucket ([`Cadence::bucket_d2`]) all read them.
//!
//! **Instrumented** (REQ-INT-009, REQ-INT-022): the cadence counts its own invocations
//! ([`WrapperCadence::invocations`]), so a test compares them with the internal steps an occupant took and with the
//! `total_substeps` it reported.
//!
//! **The word** (integrator_contract Part 5, branch-cut crossing detection; payload §3): [`WordCadence`] adds crossing
//! detection to any cadence, after that cadence's own work on the same accepted substep, for a run with a word buffer
//! bound ([`super::wrapper::integrate_word`]). The word buffer stays out of a run without one (REQ-PERF-008's symbolic
//! features off, TASK-M4-05), which is why it is a wrapper around the cadence and not a hook in it.

use super::advance::Cadence;
use super::failure;
use super::latch::RegularisedDrift;
use super::monitor::Monitor;
use super::project::ComProjection;
use super::substep::Quantise;
use super::wrapper::{monitored, WrapperReal};
use crate::detect::collision::{CollisionRadius, Detector};
use crate::detect::escape_rule::EscapeRule;
use crate::detect::label::{Events, Fired};
use crate::generated::links::LinkReal;
use crate::occupant::{Geom, State};
use crate::payload::{sd_last_symbol, set_last_symbol, PayloadReal, SimStateBaseOf};
use crate::real::IeeeBits;
use crate::regularisation::budget::Signals;
use crate::word::{state_cuts, CrossingDetector, CrossingHook, WordBuffer};

/// The wrapper's per-substep callback: the projection, the monitor and the detector, the hooks in Part 1's order, the
/// run's `done` flag and the count of its calls. A run starts from [`WrapperCadence::with_radius`]: no call made, done
/// only if its IC is inside `r_coll`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrapperCadence<R, E = crate::detect::escape_rule::DefaultRule<R>> {
    /// The COM projection, its `M` cached.
    projection: ComProjection<R>,
    /// The invariant monitor, its `E₀` and `L_{z,0}` stored.
    monitor: Monitor<R>,
    /// The collision detector and the `dmin_pair` latch.
    detector: Detector,
    /// The escape rule (`crate::detect::escape_rule`): the closure detector by default, its lagged shape register, the
    /// stored `E₀` and its verdict.
    escape: E,
    /// The failure path's per-step detection: the first non-finite state or monitored quantity (`super::failure`).
    failure: failure::Detector,
    /// The calls so far.
    invocations: u32,
    /// Whether a terminal has fired.
    done: bool,
    /// The collision's step ordinal, the calls made when it fired (0 at the IC); 0 where none has.
    collision_at: u32,
    /// The escape's step ordinal; 0 where it has not fired.
    escape_at: u32,
    /// The completed macro-steps as the wrapper last reported them ([`Cadence::clock`]).
    clock: u32,
    /// `t_dmin_step`: the completed macro-steps when the `d_min` latch was last replaced (the module's note).
    dmin_step: u32,
    /// The step ordinal of that replacement, 0 for the IC.
    dmin_at: u32,
    /// Whether that replacement has not yet met a [`Cadence::clock`] report.
    dmin_pending: bool,
    /// Whether the wrapper handed a finished stored sample back ([`Cadence::latched`]): the write leaves it.
    handed_back: bool,
    /// The regularised march's two counted signals (R-603), reloaded from a re-booted march's `budget_bits`.
    signals: Signals,
    /// The regularised energy's drift latches (R-605).
    regularised: RegularisedDrift<R>,
}

impl<R: WrapperReal> WrapperCadence<R> {
    /// The cadence of a run from masses `m`, positions `r` and momenta `p` at `r_coll = 0`
    /// ([`CollisionRadius::none`], Part 7's "refuse to regularise"), so no collision fires:
    /// [`WrapperCadence::with_radius`].
    #[inline]
    pub fn new(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> Self {
        Self::with_radius(m, r, p, CollisionRadius::none())
    }

    /// The cadence of a run from masses `m`, positions `r` and momenta `p` under the sim key's collision threshold
    /// `radius`, before its first step: the projection's `M` formed from `m` and cached, `E₀` and `L_{z,0}` evaluated
    /// on the state and stored, and the detector run on the IC, which makes the cadence done where a pair is already
    /// inside `r_coll`.
    #[inline]
    pub fn with_radius(
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        radius: CollisionRadius,
    ) -> Self {
        Self::with_rule(m, r, p, radius)
    }
}

impl<R: WrapperReal, E: EscapeRule<R>> WrapperCadence<R, E> {
    /// [`WrapperCadence::with_radius`] under the escape rule `E` (`crate::detect::escape_rule`), started on the IC.
    #[inline]
    pub fn with_rule(
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        radius: CollisionRadius,
    ) -> Self {
        let detector = Detector::new(radius, r);
        let monitor = Monitor::new(m, r, p, &Geom::new(m, r));
        // An IC whose `E₀` or `L_{z,0}` is not finite fails before its first step, at ordinal 0: every drift of its run
        // would be measured against a non-finite reference (`super::failure`). An IC already inside `r_coll` is a
        // collision at t = 0, a real outcome (integrator_contract Part 5), and no failure is read on it.
        let mut failed = failure::Detector::new();
        failed.derived(
            0,
            !detector.fired()
                && (monitor.e_0().non_finite_bits() || monitor.lz_0().non_finite_bits()),
        );
        WrapperCadence {
            projection: ComProjection::new(m),
            monitor,
            detector,
            escape: E::start(m, r, p, monitor.e_0()),
            failure: failed,
            invocations: 0,
            done: detector.fired() || failed.fired(),
            collision_at: 0,
            escape_at: 0,
            clock: 0,
            dmin_step: 0,
            dmin_at: 0,
            dmin_pending: false,
            handed_back: false,
            signals: Signals::none(),
            regularised: RegularisedDrift::none(),
        }
    }

    /// Whether the run was a finished stored sample handed back ([`Cadence::latched`]), which [`monitored`] leaves as
    /// it is.
    #[inline]
    pub fn handed_back(&self) -> bool {
        self.handed_back
    }

    /// The run's terminal events for its label ([`crate::detect::label::precedence`]): the failure, the collision and the
    /// escape, each with the step ordinal it fired on and its `detail`. The horizon is the wrapper's to add.
    #[inline]
    pub fn events(&self) -> Events {
        let collision = if self.detector.fired() {
            Fired::at(self.collision_at, self.detector.collision().detail)
        } else {
            Fired::none()
        };
        let escape = if self.escape.fired() {
            Fired::at(self.escape_at, self.escape.verdict().detail)
        } else {
            Fired::none()
        };
        Events {
            failed: self.failure.failure(),
            collision,
            escape,
            horizon: false,
        }
    }

    /// `t_dmin_step`: the macro-step index of the closest approach the `d_min` latch holds (the module's note).
    #[inline]
    pub fn dmin_step(&self) -> u32 {
        self.dmin_step
    }

    /// The regularised march's two counted signals so far (R-603).
    #[inline]
    pub fn signal_counts(&self) -> Signals {
        self.signals
    }

    /// The regularised energy's drift latches so far (R-605).
    #[inline]
    pub fn regularised(&self) -> &RegularisedDrift<R> {
        &self.regularised
    }

    /// The calls made so far, one per internal step taken.
    #[inline]
    pub fn invocations(&self) -> u32 {
        self.invocations
    }

    /// The COM projection, its `M` the one [`WrapperCadence::new`] formed.
    #[inline]
    pub fn projection(&self) -> &ComProjection<R> {
        &self.projection
    }

    /// The invariant monitor: `E₀`, `L_{z,0}` and the drifts accumulated so far.
    #[inline]
    pub fn monitor_state(&self) -> &Monitor<R> {
        &self.monitor
    }

    /// The detector: the collision verdict, the `dmin_pair` latch and the last state's separations.
    #[inline]
    pub fn detector(&self) -> &Detector {
        &self.detector
    }

    /// The escape detector: its verdict and the lagged shape register.
    #[inline]
    pub fn escape(&self) -> &E {
        &self.escape
    }

    /// Takes up the escape detector of a march still running from what its `SimState` `stored` holds: the lagged
    /// shape register and `E_0`. With it a march re-booted from `stored` tests escape as the uncut march does, since
    /// which sampling points are window marks follows from their indices alone ([`escape::is_mark`]).
    #[inline]
    pub fn resume_escape(&mut self, stored: &SimStateBaseOf<R>) {
        self.escape.resume(
            [stored._nhat_lag_0, stored._nhat_lag_1, stored._nhat_lag_2],
            stored.E_0,
        );
    }

    /// The COM projection after a `STEP` (integrator_contract Part 1, "COM projection and invariant monitoring"):
    /// `state`'s positions and momenta projected with the cached `M`, never its energy or `L_z` (REQ-INT-011), and
    /// `geom` re-formed at the projected positions, the force evaluation the next step reads and the monitor's
    /// energy reuses.
    #[inline]
    pub fn project(&self, state: &mut State<R>, geom: &mut Geom<R>) {
        let (r, p) = self.projection.project(&state.r, &state.p);
        state.r = r;
        state.p = p;
        *geom = Geom::new(&state.m, &state.r);
    }

    /// The invariant accumulation on the projected state (Part 1): `E(t)` from `geom`'s pairwise distances and
    /// `L_z(t)`, against the stored references.
    #[inline]
    pub fn monitor(&mut self, state: &State<R>, geom: &Geom<R>) {
        self.monitor.update(&state.m, &state.r, &state.p, geom);
    }
}

impl<R: WrapperReal, E: EscapeRule<R>> WrapperCadence<R, E> {
    /// The cadence's work after a `STEP` past the projection, on a state already projected: the call counted, the
    /// invariant accumulation ([`WrapperCadence::monitor`]) and terminal detection, which sets `done` on a collision.
    /// [`Cadence::after_step`] is the projection then this; the FTLE shadow's displacement form projects its own
    /// state and calls this on it (R-974; `super::benettin`).
    #[inline]
    pub fn watch(&mut self, state: &State<R>, geom: &Geom<R>) {
        self.invocations += 1;
        self.failure.state(self.invocations, &state.r, &state.p);
        self.monitor(state, geom);
        self.failure
            .derived(self.invocations, self.monitor.non_finite());
        let before = self.detector.latch();
        let fired = self.detector.fired();
        self.detector.observe(&state.r);
        let after = self.detector.latch();
        if after.d2().to_bits() != before.d2().to_bits() || after.pair() != before.pair() {
            self.dmin_at = self.invocations;
            self.dmin_step = self.clock;
            self.dmin_pending = true;
        }
        if self.detector.fired() && !fired {
            self.collision_at = self.invocations;
        }
        self.done = self.detector.fired() || self.escape.fired() || self.failure.fired();
    }
}

impl<R: WrapperReal, E: EscapeRule<R>> Cadence<R> for WrapperCadence<R, E> {
    /// The state the step left read for a failure before the projection, so a `±∞` is read as itself
    /// (`super::failure`), then the projection and [`WrapperCadence::watch`].
    #[inline]
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.failure.state(self.invocations + 1, &state.r, &state.p);
        self.project(state, geom);
        self.watch(state, geom);
    }

    #[inline]
    fn done(&self) -> bool {
        self.done
    }

    /// The escape test at a sampling point, on a run nothing ended before this point's state; `done` where it fires.
    /// A collision that fired on this point's own step is a tie, not an earlier event: the test runs on that state as
    /// well, and the label settles the tie ([`crate::detect::label::precedence`]). Where that point is a window mark
    /// the lagged register takes its shape, so a collision on a mark stores the mark's register, which a run that ended
    /// on a collision never reads again.
    #[inline]
    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        let tie = self.detector.fired() && self.collision_at == self.invocations;
        if !self.escape.fired() && !self.failure.fired() && (!self.detector.fired() || tie) {
            self.escape
                .sample(&state.m, &state.r, &state.p, index, quantum);
            if self.escape.fired() {
                self.escape_at = self.invocations;
            }
        }
        self.done = self.detector.fired() || self.escape.fired() || self.failure.fired();
    }

    #[inline]
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.signals.observe(floored, cap_hit);
    }

    #[inline]
    fn regularised_drift(&mut self, drift: R) {
        self.regularised.report(drift);
    }

    /// The completed macro-steps, `completed`, before each macro-step and after the last (the module's note): a `d_min`
    /// latched on the step that completed the macro-step just ended takes that macro-step's end, `completed`.
    #[inline]
    fn clock(&mut self, completed: u32) {
        if self.dmin_pending && self.dmin_at == self.invocations {
            self.dmin_step = completed;
        }
        self.dmin_pending = false;
        self.clock = completed;
    }

    #[inline]
    fn latched(&mut self) {
        self.handed_back = true;
    }

    /// The occupant's step-7 failure, which this cadence was never handed: recorded as its first if it has none, and
    /// the run done.
    #[inline]
    fn non_finite(&mut self, detail: u32) {
        self.failure.fire(self.invocations, detail);
        self.done = true;
    }

    #[inline]
    fn failure(&self) -> Fired {
        self.failure.failure()
    }

    /// The escape register of a re-booted march reloaded from `stored` ([`WrapperCadence::resume_escape`]), where
    /// `stored` holds one: a shape point is a unit vector, so a register of three zeros is a `SimState` the wrapper's
    /// [`monitored`] never wrote, and the cadence keeps its own, the shape at the re-boot.
    #[inline]
    fn resume(&mut self, stored: &SimStateBaseOf<R>)
    where
        R: PayloadReal,
    {
        let zero = R::default();
        let held =
            stored._nhat_lag_0 != zero || stored._nhat_lag_1 != zero || stored._nhat_lag_2 != zero;
        if held {
            self.resume_escape(stored);
        }
        // The budget word's counts go on from the stored ones, so a march split at any persist boundary stores the
        // uncut march's word (R-603; `crate::regularisation::budget`).
        self.signals = Signals::reload(stored.budget_bits);
    }

    #[inline]
    fn bucket_d2(&self, _r: &[[R; 2]; 3]) -> f32 {
        self.detector.last_d2()
    }

    #[inline]
    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R>
    where
        R: PayloadReal,
    {
        monitored(out, self)
    }
}

/// A cadence with crossing detection added: after `inner`'s work on each accepted substep, the substep's crossings are
/// appended to the word of the sample at the hook's index ([`CrossingDetector::accept_into`]). The cuts are read from
/// the state `inner` leaves; `d` and the classifier read differences of positions, so the COM projection cannot change
/// them beyond its rounding.
pub struct WordCadence<'a, R, C, B: ?Sized> {
    inner: &'a mut C,
    detector: CrossingDetector<R>,
    hook: CrossingHook,
    words: &'a mut B,
    /// The last crossing's `packed_a`: only `last_symbol` is read from it ([`Self::mirror_last_symbol`]).
    packed_a: u32,
}

impl<'a, R: LinkReal + IeeeBits, C, B: WordBuffer + ?Sized> WordCadence<'a, R, C, B> {
    /// The cadence of a march that starts at positions `start`, whose word is at `index` in `words` and empty.
    #[inline]
    pub fn new(inner: &'a mut C, start: &[[R; 2]; 3], index: usize, words: &'a mut B) -> Self {
        Self {
            inner,
            detector: CrossingDetector::new(state_cuts(start)),
            hook: CrossingHook::new(index),
            words,
            packed_a: 0,
        }
    }

    /// `packed_a` with `last_symbol` as this march's crossings left it, every other bit kept: the wrapper writes the
    /// descriptor at the end of the march, so the mirror reaches it here (payload §3).
    #[inline]
    pub fn mirror_last_symbol(&self, packed_a: u32) -> u32 {
        set_last_symbol(packed_a, sd_last_symbol(self.packed_a))
    }
}

impl<R: LinkReal + IeeeBits, C: Cadence<R>, B: WordBuffer + ?Sized> Cadence<R>
    for WordCadence<'_, R, C, B>
{
    #[inline]
    /// `inner`'s work, then the substep's crossings, unless `inner` has recorded a failure: a state that failed is
    /// no crossing (`super::failure`).
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        if !self.inner.failure().fired {
            self.packed_a = self.detector.accept_into(
                state_cuts(&state.r),
                &mut self.hook,
                self.words,
                self.packed_a,
            );
        }
    }

    #[inline]
    fn done(&self) -> bool {
        self.inner.done()
    }

    #[inline]
    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
    }

    #[inline]
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }

    #[inline]
    fn regularised_drift(&mut self, drift: R) {
        self.inner.regularised_drift(drift);
    }

    #[inline]
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    #[inline]
    fn latched(&mut self) {
        self.inner.latched();
    }

    #[inline]
    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }

    #[inline]
    fn failure(&self) -> Fired {
        self.inner.failure()
    }

    #[inline]
    fn resume(&mut self, stored: &SimStateBaseOf<R>)
    where
        R: PayloadReal,
    {
        self.inner.resume(stored);
    }

    #[inline]
    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32
    where
        R: Quantise,
    {
        self.inner.bucket_d2(r)
    }
}
