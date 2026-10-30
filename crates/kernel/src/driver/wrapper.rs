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
//! ([`SubstepState::macro_step`]; dd_integrator §3.3), then calls `ADVANCE` from `macro` to `macro + 1` with it and the
//! cadence. The occupant reports the internal steps it took, and `total_substeps` is their sum (Part 2a, Cost 2): the
//! wrapper cannot verify it, so it is asserted by test against the cadence's calls (REQ-INT-022).
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
//! macro-step counts once its `N_sub` substeps were all taken, so one that `done` cut short does not, while the loop's
//! counter `macro`, which still advances past it, bounds the loop only. [`integrate`] writes the monitored fields from
//! its cadence's monitor ([`monitored`]; Part 1, "COM projection and invariant monitoring"): `E_0` and `Lz_0`, stored
//! before the loop, and `packed_b`'s `dE_max` and `dLz_max`, the maxima latched at `Real` during the march and packed
//! to f16 once, here (payload §1, the precision rule's option (a)); `ΔE_final` and `ΔL_z` final are not stored, the read
//! side derives them from the final state (payload §5). A terminal's label and step and `d_min` are the cadence's hooks'
//! (TASK-M3-09, TASK-M3-12); `d_min` is written unset and `dmin_pair` its sentinel until then.

use super::advance::{Advance, AdvanceParams, AdvanceReal, Cadence};
use super::cadence::WrapperCadence;
use super::substep::{d2_quantised, NsubThresholds, Quantise, SubstepState};
use crate::decode::Ic;
use crate::occupant::{Geom, State};
use crate::payload::{
    pack_packed_b, pack_times, set_d_min_unset, set_dmin_pair, set_state, PayloadReal,
    SimStateBaseOf, SD_DMIN_PAIR_SENTINEL, STATE_BOUNDED, STATE_RUNNING,
};

/// The float type the wrapper runs at: an occupant's ([`AdvanceReal`]), a payload's ([`PayloadReal`]), and quantised
/// for the substep law's `d²` ([`Quantise`]): f32 and f64 (R-265).
pub trait WrapperReal: AdvanceReal + PayloadReal + Quantise {}

impl<R: AdvanceReal + PayloadReal + Quantise> WrapperReal for R {}

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

/// The integration of `ic` under the sim key's `uniforms` by the occupant `A`, with the wrapper's own cadence
/// ([`WrapperCadence`]), built from `ic` before the loop: [`integrate_with`], then the monitored fields
/// ([`monitored`]).
#[inline]
pub fn integrate<A: Advance, R: WrapperReal>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
) -> SimStateBaseOf<R> {
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let out = integrate_with::<A, R, WrapperCadence<R>>(ic, uniforms, &mut cadence);
    monitored(out, &cadence)
}

/// `out` with the monitored fields of the run `cadence` followed (Part 1, "COM projection and invariant monitoring";
/// payload §1, §2): `E_0` and `Lz_0` as stored before the loop, and `packed_b` holding `dE_max = max|ΔE|` and
/// `dLz_max = max|ΔL_z|`, rounded to f32 and then packed to f16 (clamped to binary16's finite range first).
#[inline]
pub fn monitored<R: WrapperReal>(
    out: SimStateBaseOf<R>,
    cadence: &WrapperCadence<R>,
) -> SimStateBaseOf<R> {
    let m = cadence.monitor_state();
    SimStateBaseOf {
        E_0: m.e_0(),
        Lz_0: m.lz_0(),
        packed_b: pack_packed_b(m.de_max().quantise(), m.dlz_max().quantise()),
        ..out
    }
}

/// The integration of `ic` under `uniforms` by the occupant `A`, `cadence` the per-substep callback it passes in
/// (integrator_contract Part 1's loop, under `ADVANCE`; the module's note). The run starts from `ic` alone.
#[inline]
pub fn integrate_with<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
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
    while macro_step < uniforms.horizon_steps && !cadence.done() {
        let n_sub =
            substep.macro_step(d2_quantised(&state.r), &uniforms.thresholds, uniforms.n_max);
        let params = AdvanceParams {
            dt_macro: uniforms.dt_macro,
            n_sub,
        };
        let out = A::advance(&state, &geom, macro_step, macro_step + 1, &params, cadence);
        state = out.state;
        geom = out.geom;
        total_substeps += out.substeps;
        completed += u32::from(out.substeps == n_sub);
        macro_step += 1;
    }
    let outcome = if cadence.done() {
        STATE_RUNNING
    } else {
        STATE_BOUNDED
    };
    let packed_a = set_dmin_pair(
        set_d_min_unset(substep.packed_a(set_state(0, outcome))),
        SD_DMIN_PAIR_SENTINEL,
    );
    SimStateBaseOf {
        r: state.r,
        p: state.p,
        packed_a,
        times: pack_times(completed, 0),
        total_substeps,
        ..SimStateBaseOf::default()
    }
}
