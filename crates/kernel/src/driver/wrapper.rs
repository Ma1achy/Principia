//! The fixed shared wrapper, `integrate(ic, uniforms) → SimState` (integrator_contract Part 1; systems_architecture §2,
//! the Physics rung, and § "The pixel's life"): it owns the loop, the horizon, the adaptive substepping and the
//! per-substep cadence, and calls the bound occupant through its one seam, `ADVANCE` ([`super::advance`]; R-19).
//!
//! **The loop's shape** (Part 1; gpu_determinism_note § "The discipline", rule 5). Two `while` loops, the macro-steps'
//! here and the substeps' in the occupant's `ADVANCE`, each with the `done` flag in its condition, the only exit
//! besides its count; no `break`, no `for`, and both bounded, the outer by `horizon_steps`, the inner by `N_sub`
//! (Part 2c rule 3).
//!
//! **The horizon is an integer counter** (dd_integrator §3.3; Part 5; REQ-INT-038): the loop's macro-step counter, compared
//! with the sim key's `horizon_steps = ⌈T/dt_macro⌉`, which the host computes and dispatch holds to at most 65535
//! (Part 5). No float time is accumulated: `ADVANCE`'s times are the macro-step grid's indices.
//!
//! **Per macro-step** the wrapper chooses `N_sub` from the position-quantised `d²` of the state it starts from
//! ([`SubstepState::macro_step`]; dd_integrator §3.3), which the cadence gives it ([`Cadence::bucket_d2`]): the wrapper's
//! own forms it once per step, for the collision test and `dmin_pair` too (integrator_contract Part 7; REQ-EVT-004).
//! It then calls `ADVANCE` from `macro` to `macro + 1` with it and the cadence. The occupant reports the internal steps
//! it took, and `total_substeps` is their sum (Part 2a, Cost 2): the wrapper cannot verify it, so it is asserted by test
//! against the cadence's calls (REQ-INT-022).
//!
//! **One input type, no other** (seam 3; REQ-DEC-005, REQ-INT-025; scheduler_contract Part 1): the entry point takes the
//! decoded `(m, r, p)` and the sim key's uniforms, nothing else, so no chart, neighbour or parent state reaches the
//! march, and every sample integrates from its own decoded state (REQ-SCHED-003). The occupant is a type parameter, a
//! baked variant (Part 3), so the wrapper's code holds no branch on which occupant is bound (seam 4; REQ-INT-008).
//!
//! **What it writes** of the `SimState`: the state `(r, p)` where the march stopped; `total_substeps`; `saturated`
//! (descriptor bit 5); `state`, `bounded` where the loop ran to the horizon (Part 7: reaching `T` without a terminal is
//! `bounded`; there is no timeout state) and `running` where the cadence's `done` ended it, for the terminal's label to
//! replace, every label through one precedence and one write ([`crate::detect::label`]); and `t_end_step`, the completed macro-steps (Part 5, "Exact step-index timing invariant"; payload §2): a
//! macro-step counts once the occupant reports its target reached ([`Advance::advance_reaching`]), a stepper once its
//! `N_sub` substeps were all taken and an occupant that owns its time mapping once its march stopped on the sync grid
//! at or before the target (Part 2a, Cost 2; R-565), so one that `done` cut short does not, while the loop's counter
//! `macro`, which still advances past it, bounds the loop only. **A march that ended** ([`Reached::ended`]) is never
//! run on: the loop stops at the macro-step it ended in, which is not completed, its state the one the occupant
//! latched (Part 2a § "The sync grid is a fixed quantum of physical time (R-565)": "a terminal event … still latches
//! at the step it fires on"). A spent budget leaves `state` `running`, as it stood, and sets the sticky
//! `budget_exhausted` bit of `budget_bits`, which also holds the two counted signals, `saturated` set when either fired
//! (Part 7; R-603; `crate::regularisation::budget`). A terminal found on the step the budget ran out is the outcome:
//! the march reads `done` before its budget, so the copy is labelled and the bit is never set beside a terminal.
//!
//! **A march that fails** ([`super::failure`]) ends on its first non-finite step: the cadence's per-step detection, or
//! a regularised march's own step-7 exit, which reports its cause by what failed ([`Cadence::non_finite`]; payload §2,
//! the contract's dated line of 13 Nov 2026). The macro-step it failed in is not completed, whatever the steps it took,
//! so it takes no Welford sample, shape readout or escape sample. The march keeps its record around the caller's cadence
//! ([`super::failure::Recorder`]) and writes `sim_failed` once, through [`super::failure::sim_failed`], the one writer:
//! the cause, the last finite state the cadence left, and every other field's failed-state value (payload §1).
//! [`integrate`] writes the monitored fields from
//! its cadence's monitor ([`monitored`]; Part 1, "COM projection and invariant monitoring"): `E_0` and `Lz_0`, stored
//! before the loop, and `packed_b`'s `dE_max` and `dLz_max`, the maxima latched at `Real` during the march and packed
//! once, here, in the `log16` form (R-632) (payload §1, the precision rule's option (a)); `ΔE_final` and `ΔL_z` final are not stored, the read
//! side derives them from the final state (payload §5). [`monitored`] also writes the detector's: `dmin_pair`, the
//! latched pair (payload §2), `d_min`, the `√` of its latched least `d²`, a collision's `state` and `detail` over
//! `running`, and `coll_floor`, the collision test's floor over its near-firing states and the `d_min` state (Part 7,
//! R-954, R-973; [`crate::detect`]). The three latches are packed through [`super::latch::Latches`], which clamps
//! them and writes the failed-state values (payload §1; R-271). `integrate_with` writes `dmin_pair`'s sentinel, 3, and
//! `d_min` unset, so a run whose cadence latches nothing keeps them, and carries `t_dmin_step` from where it started;
//! [`monitored`] writes the cadence's `t_dmin_step`, the macro-step index of the latched least `d²`
//! ([`WrapperCadence::dmin_step`]), which the wrapper's clock stamps ([`Cadence::clock`]). [`monitored`] writes the
//! escape detector's too ([`crate::detect::escape`]): where escape fired before any collision, `state` `escape` and its
//! `detail`, the escaper's body index or 3, over `running` or `bounded` (an escape that fires on the horizon's own
//! boundary is an escape); and the lagged shape register, `_nhat_lag_0`, `_1`, `_2` (payload §1), which a re-boot
//! hands back to its cadence ([`Cadence::resume`]). The escape test runs at the occupant's sampling
//! points ([`Cadence::sample`]): the wrapper reports each completed macro-step under an occupant that marches the
//! macro-step grid, and an occupant that owns its time mapping reports its sync boundaries from inside its march
//! ([`Advance::OWN_SAMPLING`]). The shape readout's fields, `theta`, `_reserved`, `closure_min`, `closure_step` and the departed bit,
//! are written by [`super::shape::Readout::store`] (below).
//!
//! **`r_coll`** (Part 7): [`integrate_detecting`] takes the sim key's collision threshold ([`CollisionRadius`]),
//! carried beside the loop's uniforms, and [`integrate`] is the run at `r_coll = 0`, Part 7's "refuse to regularise",
//! where no collision fires.
//!
//! **The Welford diffusion** (dd_simstate_payload §4; dd_integrator §3.5; [`super::welford`]): after each completed
//! macro-step, on the state the occupant returns (post-projection), the wrapper takes the sample `y` against the IC's
//! shape `n̂(r_0, m)`, which it derives once from `ic` before the loop, and updates `mean_y` and `C_ty` with the count
//! `n` that macro-step makes. A macro-step that `done` cut short is not completed and takes no sample, so the samples
//! taken are `t_end_step`'s count, the `n` the read side's slope divides by.
//!
//! **The shape readout** (dd_integrator §3.7; Part 1's `expose_state_readout`; [`super::shape`]): after each completed
//! macro-step, on the same state, the wrapper derives the shape point from the positions and masses and advances `θ̃`
//! with its pole hold in `_reserved` (R-389, R-392, R-397) and the closure registers with the departed bit (R-37,
//! R-545), and writes them into the `SimState`. As the Welford sample, a macro-step that `done` cut short takes none, so
//! `θ̃` and the closure are those of the last completed macro-step and `closure_step` never exceeds `t_end_step`.
//!
//! **A resumed march** ([`integrate_resumed_with`]): from a stored `SimState` of the same `ic`, at its `t_end_step`, the
//! wrapper reloads the state `(r, p)`, `total_substeps`, the sticky `saturated` bit, the Welford co-moments and the
//! readout ([`super::shape::Readout::load`]), and marches on; a march split into any number of resumed calls writes
//! these fields bit-identically to one call (R-397's "however the march is split into dispatches"). The cadence is the
//! caller's; the wrapper hands it the stored state before the first step ([`Cadence::resume`]), and the wrapper's own
//! reloads the escape register from it. The monitor's latches and the collision detector's are still the caller's to
//! resume. A stored sample that is finished is handed back word for word, never marched (`super::terminal`).
//!
//! **The carried regularised state** ([`Carried`]; Part 2a): the wrapper passes the trajectory's buffer into every
//! `ADVANCE` call ([`Advance::advance_carrying`]), so an occupant that registers once, Heggie or logH, registers at `t = 0` and
//! marches on from its own state at every later call. A run from its IC starts the buffer unregistered. Like the
//! cadence, the buffer is the caller's to resume: [`integrate_carrying`] hands it back beside the `SimState`, and
//! [`integrate_resumed_carrying`] takes it up again with the stored `SimState`, bitwise the uncut march. A resume with
//! no buffer ([`integrate_resumed_with`]) starts one unregistered, which under such an occupant is a re-registration
//! from the stored `(r, p)`, not the march registered once: it is for occupants that carry nothing.

use spirv_std::num_traits::Float;

use super::advance::{Advance, AdvanceParams, AdvanceReal, Cadence, Reached};
use super::cadence::{WordCadence, WrapperCadence};
use super::carried::Carried;
use super::failure::{self, Recorder};
use super::latch::{persist_de_reg, Latches};
use super::shape::{Readout, ReadoutReal};
use super::substep::{NsubThresholds, Quantise, SubstepState};
use super::welford::{self, Welford};
use crate::decode::Ic;
use crate::detect::collision::CollisionRadius;
use crate::detect::escape_rule::EscapeRule;
use crate::detect::label::{precedence, Events, Fired};
use crate::generated::links::LinkReal;
use crate::occupant::{Geom, State};
use crate::payload::{
    pack_times, sd_detail, sd_is_failed, sd_saturated, sd_state, set_coll_floor, set_d_min_unset,
    set_dmin_pair, set_saturated, set_t_dmin_step, tm_t_dmin_step, tm_t_end_step, DminCounters,
    PayloadReal, SimStateBaseOf, SD_DMIN_PAIR_SENTINEL, STATE_BOUNDED,
};
use crate::real::IeeeBits;
use crate::regularisation::budget::latch_exhausted;
use crate::shape::shape;
use crate::word::WordBuffer;

/// The float type the wrapper runs at: an occupant's ([`AdvanceReal`]), a payload's ([`PayloadReal`]), quantised for
/// the substep law's `d²` ([`Quantise`]), with the shape map's `Float` functions for the Welford `y`
/// ([`welford::y`]), and the shape readout's ([`ReadoutReal`]): f32 and f64 (R-265).
pub trait WrapperReal: AdvanceReal + PayloadReal + Quantise + Float + ReadoutReal {}

impl<R: AdvanceReal + PayloadReal + Quantise + Float + ReadoutReal> WrapperReal for R {}

/// The sim-key values the wrapper's loop reads (integrator_contract Part 3, "Wrapper config — the loop"), constant
/// across a dispatch; `SimUniforms` carries them (TASK-M3-06 lays it out and binds it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrapperUniforms<R> {
    /// The macro-step, `max(1e-3, T/65535)` (Part 3; R-132), as the host computed it.
    pub dt_macro: R,
    /// `horizon_steps = ⌈T/dt_macro⌉`, as the host computed it, at most 65535 (Part 5).
    pub horizon_steps: u32,
    /// The substep cap `N_max`, at least 1 and at most the table's maximum length (Part 3; R-502).
    pub n_max: u32,
    /// The sim key's `N_sub` thresholds (dd_integrator §3.3; R-502).
    pub thresholds: NsubThresholds,
}

/// The integration of `ic` under the sim key's `uniforms` by the occupant `A` at `r_coll = 0`, where no collision
/// fires ([`CollisionRadius::none`]): [`integrate_detecting`].
#[inline]
pub fn integrate<A: Advance, R: WrapperReal>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
) -> SimStateBaseOf<R> {
    integrate_detecting::<A, R>(ic, uniforms, CollisionRadius::none())
}

/// The integration of `ic` under the sim key's `uniforms` and collision threshold `radius` by the occupant `A`, with
/// the wrapper's own cadence ([`WrapperCadence::with_radius`]), built from `ic` before the loop, its detectors run on
/// `ic` there: [`integrate_with`], then the monitored and detected fields ([`monitored`]). An `ic` already inside
/// `r_coll` takes no step and is labelled `collision` at `t_end_step = 0` (integrator_contract Part 5).
#[inline]
pub fn integrate_detecting<A: Advance, R: WrapperReal>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    radius: CollisionRadius,
) -> SimStateBaseOf<R> {
    integrate_detecting_rule::<A, R, crate::detect::escape::Detector<R>>(ic, uniforms, radius)
}

/// [`integrate_detecting`] under the escape rule `E` (`crate::detect::escape_rule`), selected as the occupant `A` is:
/// the wrapper's cadence holds `E` in place of the closure detector, and nothing else changes.
#[inline]
pub fn integrate_detecting_rule<A: Advance, R: WrapperReal, E: EscapeRule<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    radius: CollisionRadius,
) -> SimStateBaseOf<R> {
    let mut cadence = WrapperCadence::<R, E>::with_rule(&ic.m, &ic.r, &ic.p, radius);
    let out = integrate_with::<A, R, WrapperCadence<R, E>>(ic, uniforms, &mut cadence);
    monitored(out, &cadence)
}

/// `out` with the monitored fields of the run `cadence` followed (Part 1, "COM projection and invariant monitoring";
/// payload §1, §2): `E_0` and `Lz_0` as stored before the loop, and `packed_b` holding `dE_max = max|ΔE|` and
/// `dLz_max = max|ΔL_z|`, rounded to f32 and then packed in the `log16` form (R-632); `dE_reg`, the regularised drift's
/// final value and maximum as the occupant reported them, or the Cartesian ones where it reported none (R-605); and
/// `budget_bits`' two counts, `saturated` set when either fired (R-603); and its
/// detector's fields (Part 7; [`crate::detect::collision::Detector::label`]): `dmin_pair` the latched pair, `d_min` the
/// `√` of the latched least `d²`, the IC's included, so a start already collided stores its least separation (R-933),
/// and where a collision fired, `state` `collision` and its `detail`; and `coll_floor`, the floor's record
/// ([`crate::detect::collision::Detector::floor_code`]; Part 7, R-973). The latches are packed by
/// [`Latches::persist`] (payload §1's semantics (a)). Rounding the `Real` maxima to f32 at write-out gives what an f32
/// latch of the f32-rounded drifts would hold, since rounding is monotone.
///
/// **A failed run** is marked, never stored as measured: where the cadence recorded a failure (its per-step detection,
/// or a regularised march's step-7 exit it was told of; [`super::failure`]), `state` `sim_failed` with that cause, over
/// a collision too (`sim_failed` first); where it recorded none and `out` is already `sim_failed`, `out`'s cause. Either
/// way the one writer, [`super::failure::sim_failed`], writes every field's failed-state value (payload §1): the
/// latches 0.0 in both drift halves and `d_min` unset (R-271), `dmin_pair` 3, `coll_floor` 0, `t_dmin_step` 0, and no
/// f32 field holds the non-finite value that failed it, `E_0` and `Lz_0` included. The label is [`precedence`]'s, of
/// the failure, the cadence's terminals with their step ordinals and the horizon.
///
/// **A finished stored sample handed back** by a resumed call (`super::terminal`; [`Cadence::latched`]) is returned as
/// it is: the cadence followed no run, and its write would replace the stored label and latches with a fresh
/// cadence's.
///
/// The `d_min` counters [`Latches::persist`] counts into are a pair of this write's own: the latch admits no NaN `d²`
/// and the detector's quantised `d²` is never negative (`crate::driver::substep::square`), so `√` gives neither a NaN
/// nor a negative value and neither counter can count here (R-288).
#[inline]
pub fn monitored<R: WrapperReal, E: EscapeRule<R>>(
    out: SimStateBaseOf<R>,
    cadence: &WrapperCadence<R, E>,
) -> SimStateBaseOf<R> {
    if cadence.handed_back() {
        out
    } else {
        written(out, cadence)
    }
}

/// [`monitored`]'s write of a run the cadence followed.
#[inline]
fn written<R: WrapperReal, E: EscapeRule<R>>(
    out: SimStateBaseOf<R>,
    cadence: &WrapperCadence<R, E>,
) -> SimStateBaseOf<R> {
    let m = cadence.monitor_state();
    let detector = cadence.detector();
    let [lag_0, lag_1, lag_2] = cadence.escape().lagged();
    // The cadence's own record of a failure first; a march's failure it was not told of keeps the march's cause.
    let own = cadence.failure();
    let failure = if own.fired {
        own
    } else if sd_is_failed(out.packed_a) {
        Fired::at(0, sd_detail(out.packed_a))
    } else {
        Fired::none()
    };
    let events = Events {
        failed: failure,
        horizon: sd_state(out.packed_a) == STATE_BOUNDED,
        ..cadence.events()
    };
    // R-603: `saturated` is set when either of the regularised march's counted signals fired.
    let signals = cadence.signal_counts();
    let described = set_saturated(out.packed_a, sd_saturated(out.packed_a) || signals.fired());
    let labelled = precedence(&events).write(set_dmin_pair(described, detector.latch().pair()));
    let mut latches = Latches::unset();
    latches.separation_squared(detector.latch().d2());
    latches.drift(m.de_max().quantise(), m.dlz_max().quantise());
    let (packed_a, packed_b) = latches.persist(labelled, &DminCounters::new());
    // R-605: the regularised drift, or the Cartesian where no regularised one was reported (regularisation off, the two
    // energies one). A failed run's is replaced by the failed-state value below (`failure::sim_failed`).
    let reg = cadence.regularised();
    let (last, max) = if reg.seen() {
        (reg.last().quantise(), reg.max().quantise())
    } else {
        (m.de_final().quantise(), m.de_max().quantise())
    };
    let measured = SimStateBaseOf {
        E_0: m.e_0(),
        Lz_0: m.lz_0(),
        packed_a,
        packed_b,
        budget_bits: signals.write(out.budget_bits),
        dE_reg: persist_de_reg(false, last, max),
        coll_floor_bits: set_coll_floor(out.coll_floor_bits, detector.floor_code()),
        times: set_t_dmin_step(out.times, cadence.dmin_step()),
        _nhat_lag_0: lag_0,
        _nhat_lag_1: lag_1,
        _nhat_lag_2: lag_2,
        ..out
    };
    if failure.fired {
        failure::sim_failed(measured, failure.detail)
    } else {
        measured
    }
}

/// The integration of `ic` under `uniforms` by the occupant `A` with its word: `cadence` the per-substep callback, with
/// crossing detection added at every accepted substep ([`WordCadence`]), the sample's word at `index` in `words`, empty
/// at the start. The returned descriptor's `last_symbol` mirrors the word (payload §3).
#[inline]
pub fn integrate_word<A, R, C, B>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    index: usize,
    words: &mut B,
) -> SimStateBaseOf<R>
where
    A: Advance,
    R: WrapperReal + LinkReal + IeeeBits,
    C: Cadence<R>,
    B: WordBuffer + ?Sized,
{
    let mut word_cadence = WordCadence::new(cadence, &ic.r, index, words);
    let out = integrate_with::<A, R, _>(ic, uniforms, &mut word_cadence);
    let packed_a = word_cadence.mirror_last_symbol(out.packed_a);
    cadence.finish(SimStateBaseOf { packed_a, ..out })
}

/// The integration of `ic` under `uniforms` by the occupant `A`, `cadence` the per-substep callback it passes in
/// (integrator_contract Part 1's loop, under `ADVANCE`; the module's note). The run starts from `ic` alone, and no
/// other trajectory marches beside it: [`integrate_with_co`] with none.
#[inline]
pub fn integrate_with<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
) -> SimStateBaseOf<R> {
    integrate_with_co::<A, R, C, ()>(ic, uniforms, cadence, &mut ())
}

/// [`integrate_with`], with the trajectory's carried regularised state handed back in `carried` (the module's note):
/// the buffer is set unregistered, as a run from its IC starts it, and holds what the occupant carried where the
/// march stopped, its re-registration count with it. The `SimState` is [`integrate_with`]'s, bit for bit.
#[inline]
pub fn integrate_carrying<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    carried: &mut Carried<R>,
) -> SimStateBaseOf<R> {
    *carried = Carried::unregistered();
    let end = uniforms.horizon_steps;
    march::<A, R, C, ()>(uniforms, cadence, &mut (), Start::fresh(ic), end, carried)
}

/// A trajectory the wrapper marches beside the sample's own, macro-step by macro-step (dd_integrator §3.4: the
/// wrapper runs the per-`STEP` sequence on every marching trajectory it owns): the Benettin shadow
/// ([`super::benettin`]). It is never the sample: nothing it does reaches the sample's state or its cadence.
pub trait CoMarch<R: AdvanceReal> {
    /// Runs after the sample's macro-step from the grid's `t_now`: `start` the state it began from, `end` the state it
    /// reached, `params` its `dt_macro` and `N_sub`, and `taken` the substeps it took, `N_sub` unless its `done` cut
    /// the macro-step short. `A` is the bound occupant. For a stepper `taken == N_sub` is exactly the macro-step
    /// completed ([`Reached::by_count`], what the wrapper counts); under an occupant that owns its time mapping the
    /// shadow's joint step is [`crate::regularisation::heggie_joint`]'s (R-974; Heggie × RK4), its joint march not yet
    /// built, and no caller marches a shadow beside one.
    fn after_macro<A: Advance>(
        &mut self,
        start: &State<R>,
        end: &State<R>,
        t_now: u32,
        params: &AdvanceParams<R>,
        taken: u32,
    );
}

/// No trajectory beside the sample's.
impl<R: AdvanceReal> CoMarch<R> for () {
    #[inline]
    fn after_macro<A: Advance>(
        &mut self,
        _start: &State<R>,
        _end: &State<R>,
        _t_now: u32,
        _params: &AdvanceParams<R>,
        _taken: u32,
    ) {
    }
}

/// [`integrate_with`], with `co` marched beside the sample: after each macro-step the sample takes, `co` is handed
/// the macro-step's start and end states, its parameters and the substeps taken ([`CoMarch::after_macro`]).
#[inline]
pub fn integrate_with_co<A: Advance, R: WrapperReal, C: Cadence<R>, M: CoMarch<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    co: &mut M,
) -> SimStateBaseOf<R> {
    let mut carried = Carried::unregistered();
    let end = uniforms.horizon_steps;
    march::<A, R, C, M>(uniforms, cadence, co, Start::fresh(ic), end, &mut carried)
}

/// The march resumed from `stored`, a `SimState` this wrapper wrote for the same `ic` under the same `uniforms`, for at
/// most `steps` more macro-steps, never past the horizon (the module's note): from `stored`'s state at its
/// `t_end_step`, with its `total_substeps`, `saturated` bit, Welford co-moments, `θ̃`, `_reserved`, closure registers and
/// departed bit, and `cadence` the caller's, handed `stored` first ([`Cadence::resume`]). A march that stops short of the
/// horizon is `running`. A `stored` that is finished ([`super::terminal::finished`]) is returned as it is, every word,
/// with no step taken and no cadence call.
///
/// No carried state is resumed: the occupant is handed an unregistered buffer (the module's note).
#[inline]
pub fn integrate_resumed_with<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    stored: &SimStateBaseOf<R>,
    steps: u32,
) -> SimStateBaseOf<R> {
    let mut carried = Carried::unregistered();
    integrate_resumed_carrying::<A, R, C>(ic, uniforms, cadence, stored, &mut carried, steps)
}

/// [`integrate_resumed_with`], with the trajectory's carried regularised state resumed too (the module's note): the
/// occupant marches on from `carried`, the buffer the wrapper handed back with `stored`, both of the same trajectory
/// at the same macro-step, and its state where this march stops is written there. A march split into any number of
/// these calls is the uncut march, bit for bit, in the `SimState` and in the buffer.
#[inline]
pub fn integrate_resumed_carrying<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    stored: &SimStateBaseOf<R>,
    carried: &mut Carried<R>,
    steps: u32,
) -> SimStateBaseOf<R> {
    if super::terminal::finished(
        stored.packed_a,
        stored.times,
        stored.budget_bits,
        uniforms.horizon_steps,
    ) {
        cadence.latched();
        *stored
    } else {
        resume_march::<A, R, C>(ic, uniforms, cadence, stored, carried, steps)
    }
}

/// [`integrate_resumed_carrying`] of a march still running: the cadence handed `stored` ([`Cadence::resume`]), then the
/// march from `stored`'s state.
#[inline]
fn resume_march<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    stored: &SimStateBaseOf<R>,
    carried: &mut Carried<R>,
    steps: u32,
) -> SimStateBaseOf<R> {
    cadence.resume(stored);
    let completed = tm_t_end_step(stored.times);
    let start = Start {
        state: State {
            m: ic.m,
            r: stored.r,
            p: stored.p,
        },
        completed,
        t_dmin_step: tm_t_dmin_step(stored.times),
        total_substeps: stored.total_substeps,
        saturated: sd_saturated(stored.packed_a),
        diffusion: Welford {
            mean_y: stored.mean_y,
            c_ty: stored.C_ty,
        },
        readout: Readout::load(stored, shape(ic.r, ic.m), ic.m),
    };
    // The guard is written out, not as `saturating_sub`, for rust-gpu (R-169), as `Advance::advance`'s is.
    #[allow(clippy::implicit_saturating_sub)]
    let left = if uniforms.horizon_steps > completed {
        uniforms.horizon_steps - completed
    } else {
        0
    };
    let end = if steps < left {
        completed + steps
    } else {
        uniforms.horizon_steps
    };
    march::<A, R, C, ()>(uniforms, cadence, &mut (), start, end, carried)
}

/// Where a march starts: the state, the completed macro-steps, `total_substeps` and `saturated` so far, the Welford
/// co-moments and the shape readout. A fresh run's is the IC's with every count 0; a resumed run's is a stored
/// `SimState`'s.
struct Start<R> {
    state: State<R>,
    completed: u32,
    t_dmin_step: u32,
    total_substeps: u32,
    saturated: bool,
    diffusion: Welford<R>,
    readout: Readout<R>,
}

impl<R: WrapperReal> Start<R> {
    /// A run from its IC: the IC's state with every count 0.
    #[inline]
    fn fresh(ic: &Ic<R>) -> Self {
        Start {
            state: State {
                m: ic.m,
                r: ic.r,
                p: ic.p,
            },
            completed: 0,
            t_dmin_step: 0,
            total_substeps: 0,
            saturated: false,
            diffusion: Welford::default(),
            readout: Readout::fresh(ic.r, ic.m),
        }
    }
}

/// The macro-step loop from `start` to the macro-step `end`, at most the horizon (integrator_contract Part 1's loop,
/// under `ADVANCE`; the module's note): `co` marched beside it, `carried` the trajectory's buffer, passed into every
/// call, and `cadence` with the failure path's record kept around it ([`Recorder`]). The run is `bounded` where it
/// reaches the horizon, `running` where `done`, an ended march or `end` stops it first, and `sim_failed` where its march
/// went non-finite, written by [`failure::sim_failed`] on the last finite state the cadence left.
#[inline]
fn march<A: Advance, R: WrapperReal, C: Cadence<R>, M: CoMarch<R>>(
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    co: &mut M,
    start: Start<R>,
    end: u32,
    carried: &mut Carried<R>,
) -> SimStateBaseOf<R> {
    let mut state = start.state;
    let mut geom = Geom::new(&state.m, &state.r);
    let mut cadence = Recorder::new(cadence, &start.state);
    let mut substep = SubstepState {
        saturated: start.saturated,
        ..SubstepState::default()
    };
    let mut total_substeps = start.total_substeps;
    let mut macro_step = start.completed;
    let mut completed = start.completed;
    let mut readout = start.readout;
    let mut diffusion = start.diffusion;
    let mut last = Reached::Target;
    while macro_step < end && !cadence.done() && !last.ended() {
        cadence.clock(completed);
        let n_sub = substep.macro_step(
            cadence.bucket_d2(&state.r),
            &uniforms.thresholds,
            uniforms.n_max,
        );
        let params = AdvanceParams {
            dt_macro: uniforms.dt_macro,
            n_sub,
        };
        let (out, reached) = A::advance_carrying(
            &state,
            &geom,
            macro_step,
            macro_step + 1,
            &params,
            &mut cadence,
            carried,
        );
        co.after_macro::<A>(&state, &out.state, macro_step, &params, out.substeps);
        state = out.state;
        geom = out.geom;
        total_substeps += out.substeps;
        // A macro-step the march failed in is not completed, whatever steps it took (the module's note).
        let reached = if cadence.failure().fired {
            Reached::NonFinite
        } else {
            reached
        };
        if reached == Reached::Target {
            completed += 1;
            diffusion = diffusion.update(
                welford::y(state.r, state.m, readout.n_0),
                completed,
                uniforms.dt_macro,
            );
            readout.after_macro(state.r, state.m, completed);
            // The escape window's sampling point under an occupant that marches the macro-step grid; one that owns
            // its time mapping has reported its sync boundaries from inside the call (Part 7).
            if !A::OWN_SAMPLING {
                cadence.sample(&state, completed, uniforms.dt_macro);
            }
        }
        last = reached;
        macro_step += 1;
    }
    cadence.clock(completed);
    // The march's own label, whether it reached the horizon; the cadence's terminals are written over it by its
    // `finish` ([`monitored`]), through the same precedence. A failed march is written once, by the one writer.
    let failure = cadence.failure_at(last, &state);
    // The state a failed march stores: a regularised march's step-7 exit latched its last finite state, which the
    // contract stores (Part 2b, the dated line of 13 Nov 2026); a failure the cadence read stores the last finite state
    // the cadence left. Either is finite; one that is not is the writer's to blank.
    let latched = last == Reached::NonFinite
        && !cadence.read_by_cadence()
        && !failure::state_bits(&state.r, &state.p).0;
    let events = Events {
        horizon: !(cadence.done() || last.ended() || macro_step < uniforms.horizon_steps),
        ..Events::default()
    };
    let described = precedence(&events).write(substep.packed_a(0));
    let packed_a = set_dmin_pair(set_d_min_unset(described), SD_DMIN_PAIR_SENTINEL);
    let stopped = if !failure.fired || latched {
        state
    } else {
        *cadence.last()
    };
    let out = readout.store(SimStateBaseOf {
        r: stopped.r,
        p: stopped.p,
        mean_y: diffusion.mean_y,
        C_ty: diffusion.c_ty,
        packed_a,
        times: pack_times(completed, start.t_dmin_step),
        total_substeps,
        // R-603: a spent budget is the sticky flag, `state` left `running` (`crate::regularisation::budget`).
        budget_bits: latch_exhausted(0, last == Reached::Exhausted),
        ..SimStateBaseOf::default()
    });
    if failure.fired {
        failure::sim_failed(out, failure.detail)
    } else {
        out
    }
}
