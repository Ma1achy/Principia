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
//! replace; and `t_end_step`, the completed macro-steps (Part 5, "Exact step-index timing invariant"; payload §2): a
//! macro-step counts once the occupant reports its target reached ([`Advance::advance_reaching`]), a stepper once its
//! `N_sub` substeps were all taken and an occupant that owns its time mapping once its march stopped on the sync grid
//! at or before the target (Part 2a, Cost 2; R-565), so one that `done` cut short does not, while the loop's counter
//! `macro`, which still advances past it, bounds the loop only. **A march that ended** ([`Reached::ended`]) is never
//! run on: the loop stops at the macro-step it ended in, which is not completed, its state the one the occupant
//! latched (Part 2a § "The sync grid is a fixed quantum of physical time (R-565)": "a terminal event … still latches
//! at the step it fires on"). A non-finite one writes `sim_failed` with its detail (`failed_detail`; payload §2) read
//! on the latched, finite state, so 2 for Aarseth–Zare: that detail is interim until RQ-806 is ruled (physics review
//! 5879658175 of PR #502, P1; the failure path is TASK-M3-18's). A spent budget leaves `state` `running`, as it stood (Part 7; R-603, whose `budget_exhausted` flag is
//! TASK-M3-41's). [`integrate`] writes the monitored fields from
//! its cadence's monitor ([`monitored`]; Part 1, "COM projection and invariant monitoring"): `E_0` and `Lz_0`, stored
//! before the loop, and `packed_b`'s `dE_max` and `dLz_max`, the maxima latched at `Real` during the march and packed
//! to f16 once, here (payload §1, the precision rule's option (a)); `ΔE_final` and `ΔL_z` final are not stored, the read
//! side derives them from the final state (payload §5). [`monitored`] also writes the detector's: `dmin_pair`, the
//! latched pair (payload §2), `d_min`, the `√` of its latched least `d²`, a collision's `state` and `detail` over
//! `running`, and `coll_floor`, the collision test's floor over its near-firing states and the `d_min` state (Part 7,
//! R-954, R-973; [`crate::detect`]). The three latches are packed through [`super::latch::Latches`], which clamps
//! them and writes the failed-state values (payload §1; R-271). `integrate_with` writes `dmin_pair`'s sentinel, 3, and
//! `d_min` unset, so a run whose cadence latches nothing keeps them; `t_dmin_step` is TASK-M3-17's, and escape's label
//! TASK-M3-12's.
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

use spirv_std::num_traits::Float;

use super::advance::{Advance, AdvanceParams, AdvanceReal, Cadence, Reached};
use super::cadence::{WordCadence, WrapperCadence};
use super::latch::Latches;
use super::substep::{NsubThresholds, Quantise, SubstepState};
use super::welford::{self, Welford};
use crate::decode::Ic;
use crate::detect::collision::CollisionRadius;
use crate::generated::links::LinkReal;
use crate::occupant::{Geom, State};
use crate::payload::{
    pack_times, sd_is_failed, set_coll_floor, set_d_min_unset, set_detail, set_dmin_pair,
    set_state, DminCounters, PayloadReal, SimStateBaseOf, SD_DMIN_PAIR_SENTINEL, STATE_BOUNDED,
    STATE_RUNNING, STATE_SIM_FAILED,
};
use crate::real::IeeeBits;
use crate::shape::shape;
use crate::word::WordBuffer;

/// The float type the wrapper runs at: an occupant's ([`AdvanceReal`]), a payload's ([`PayloadReal`]), quantised for
/// the substep law's `d²` ([`Quantise`]), and with the shape map's `Float` functions for the Welford `y`
/// ([`welford::y`]): f32 and f64 (R-265).
pub trait WrapperReal: AdvanceReal + PayloadReal + Quantise + Float {}

impl<R: AdvanceReal + PayloadReal + Quantise + Float> WrapperReal for R {}

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
    let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius);
    let out = integrate_with::<A, R, WrapperCadence<R>>(ic, uniforms, &mut cadence);
    monitored(out, &cadence)
}

/// `out` with the monitored fields of the run `cadence` followed (Part 1, "COM projection and invariant monitoring";
/// payload §1, §2): `E_0` and `Lz_0` as stored before the loop, and `packed_b` holding `dE_max = max|ΔE|` and
/// `dLz_max = max|ΔL_z|`, rounded to f32 and then packed to f16 (clamped to binary16's finite range first); and its
/// detector's fields (Part 7; [`crate::detect::collision::Detector::label`]): `dmin_pair` the latched pair, `d_min` the
/// `√` of the latched least `d²`, the IC's included, so a start already collided stores its least separation (R-933),
/// and where a collision fired, `state` `collision` and its `detail`; and `coll_floor`, the floor's record
/// ([`crate::detect::collision::Detector::floor_code`]; Part 7, R-973). The latches are packed by
/// [`Latches::persist`] (payload §1's semantics (a)). Rounding the `Real` maxima to f32 at write-out gives what an f32
/// latch of the f32-rounded drifts would hold, since rounding is monotone.
///
/// **A run whose monitor latched a non-finite value** is marked, never stored as measured: `state` `sim_failed`, with
/// its detail by what failed (payload §2: `failed_detail`), over a collision too (R-30: `sim_failed` first), the
/// latches the defined failed-state values, 0.0 in both drift halves and `d_min` unset (payload §1, "Failed-state
/// contents are defined"; R-79, R-271), `dmin_pair` its sentinel, 3, unset as `d_min` is, and `coll_floor` 0, "not
/// recorded" (payload §2). **A run `out` already
/// `sim_failed`**, its march ended non-finite ([`integrate_with`]), keeps its `state` and `detail`, over a collision
/// too (R-30), with the same failed-state latches, sentinel and `coll_floor`. The rest of the failure path, the terminal that ends
/// the march and the other fields' failed-state values, is TASK-M3-17's and TASK-M3-18's.
///
/// The `d_min` counters [`Latches::persist`] counts into are a pair of this write's own: the latch admits no NaN `d²`
/// and the detector's quantised `d²` is never negative (`crate::driver::substep::square`), so `√` gives neither a NaN
/// nor a negative value and neither counter can count here (R-288).
#[inline]
pub fn monitored<R: WrapperReal>(
    out: SimStateBaseOf<R>,
    cadence: &WrapperCadence<R>,
) -> SimStateBaseOf<R> {
    let m = cadence.monitor_state();
    let detector = cadence.detector();
    let detected = detector.label(out.packed_a);
    let failed = m.non_finite() || sd_is_failed(out.packed_a);
    let labelled = if m.non_finite() {
        let marked = set_state(detected, STATE_SIM_FAILED);
        let marked = set_dmin_pair(marked, SD_DMIN_PAIR_SENTINEL);
        set_detail(marked, failed_detail(&out.r, &out.p))
    } else if sd_is_failed(out.packed_a) {
        set_dmin_pair(out.packed_a, SD_DMIN_PAIR_SENTINEL)
    } else {
        detected
    };
    let mut latches = Latches::unset();
    latches.separation_squared(detector.latch().d2());
    latches.drift(m.de_max().quantise(), m.dlz_max().quantise());
    let (packed_a, packed_b) = latches.persist(labelled, &DminCounters::new());
    let floor = if failed { 0 } else { detector.floor_code() };
    SimStateBaseOf {
        E_0: m.e_0(),
        Lz_0: m.lz_0(),
        packed_a,
        packed_b,
        coll_floor_bits: set_coll_floor(out.coll_floor_bits, floor),
        ..out
    }
}

/// `sim_failed`'s detail for a run ending at `(r, p)` (payload §2): 0 for a NaN in the state, else 1 for a `±∞` in it,
/// else 2, a non-finite derived quantity on a finite state. The two are told apart on the bits
/// ([`IeeeBits::nan_bits`], [`IeeeBits::non_finite_bits`]; gpu_determinism_note § "The discipline", rule 4).
#[inline]
fn failed_detail<R: WrapperReal>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> u32 {
    let (mut nan, mut inf) = (false, false);
    let mut i = 0;
    while i < 3 {
        let mut j = 0;
        while j < 2 {
            let (a, b) = (r[i][j], p[i][j]);
            nan = nan || a.nan_bits() || b.nan_bits();
            inf = inf || a.non_finite_bits() || b.non_finite_bits();
            j += 1;
        }
        i += 1;
    }
    if nan {
        0
    } else if inf {
        1
    } else {
        2
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

/// A trajectory the wrapper marches beside the sample's own, macro-step by macro-step (dd_integrator §3.4: the
/// wrapper runs the per-`STEP` sequence on every marching trajectory it owns): the Benettin shadow
/// ([`super::benettin`]). It is never the sample: nothing it does reaches the sample's state or its cadence.
pub trait CoMarch<R: AdvanceReal> {
    /// Runs after the sample's macro-step from the grid's `t_now`: `start` the state it began from, `end` the state it
    /// reached, `params` its `dt_macro` and `N_sub`, and `taken` the substeps it took, `N_sub` unless its `done` cut
    /// the macro-step short. `A` is the bound occupant. For a stepper `taken == N_sub` is exactly the macro-step
    /// completed ([`Reached::by_count`], what the wrapper counts); under an occupant that owns its time mapping the
    /// shadow's joint step is TASK-M3-74's (R-974; Heggie × RK4), and no caller marches a shadow beside one.
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
    let mut state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let mut geom = Geom::new(&state.m, &state.r);
    let mut substep = SubstepState::default();
    let mut total_substeps: u32 = 0;
    let mut macro_step: u32 = 0;
    let mut completed: u32 = 0;
    let n_0 = shape(ic.r, ic.m);
    let mut diffusion = Welford::default();
    let mut last = Reached::Target;
    while macro_step < uniforms.horizon_steps && !cadence.done() && !last.ended() {
        let n_sub = substep.macro_step(
            cadence.bucket_d2(&state.r),
            &uniforms.thresholds,
            uniforms.n_max,
        );
        let params = AdvanceParams {
            dt_macro: uniforms.dt_macro,
            n_sub,
        };
        let (out, reached) =
            A::advance_reaching(&state, &geom, macro_step, macro_step + 1, &params, cadence);
        co.after_macro::<A>(&state, &out.state, macro_step, &params, out.substeps);
        state = out.state;
        geom = out.geom;
        total_substeps += out.substeps;
        if reached == Reached::Target {
            completed += 1;
            diffusion = diffusion.update(
                welford::y(state.r, state.m, n_0),
                completed,
                uniforms.dt_macro,
            );
        }
        last = reached;
        macro_step += 1;
    }
    let outcome = if last == Reached::NonFinite {
        STATE_SIM_FAILED
    } else if cadence.done() || last.ended() {
        STATE_RUNNING
    } else {
        STATE_BOUNDED
    };
    let described = substep.packed_a(set_state(0, outcome));
    let described = if outcome == STATE_SIM_FAILED {
        set_detail(described, failed_detail(&state.r, &state.p))
    } else {
        described
    };
    let packed_a = set_dmin_pair(set_d_min_unset(described), SD_DMIN_PAIR_SENTINEL);
    SimStateBaseOf {
        r: state.r,
        p: state.p,
        mean_y: diffusion.mean_y,
        C_ty: diffusion.c_ty,
        packed_a,
        times: pack_times(completed, 0),
        total_substeps,
        ..SimStateBaseOf::default()
    }
}
