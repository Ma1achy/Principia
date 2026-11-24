//! The terminal latch (integrator_contract Part 1, Part 7; payload §2; render_contract Part 1, "On termination the
//! sample latches").
//!
//! **Within a dispatch** the latch is the loop's own: a terminal sets the cadence's `done` flag
//! (`super::cadence::WrapperCadence`), and `done` is in both loops' conditions, the macro-steps' in
//! `super::wrapper` and the substeps' in the occupant's `ADVANCE`, the only exits besides their counts. So the march
//! ends on the step the terminal fired on: a collision on the internal step whose state it read, an escape at the
//! sampling point it was tested at. `t_end_step` counts the macro-steps completed by then, so an escape tested at a
//! completed macro-step's end has `t_end_step` that macro-step's, and a collision that cut a macro-step short has the
//! completed ones before it. Nothing after the firing step reaches the payload: no further step, Welford sample,
//! shape readout, renormalisation or detector call. The FTLE shadow takes the substeps the sample took and no more
//! (`super::benettin`), so the sum `S` and the shadow's `δ`, from which the read side finalises the FTLE at
//! `t_end_step` (payload §5), are those of the firing step: the FTLE is frozen at the event's time.
//!
//! **Across dispatches** ([`finished`], [`latch`], [`redispatch`]): a stored sample that is finished is handed back
//! word for word, with no march, no cadence call, no carried state touched and no field rewritten: every `SimState`
//! word is bit-identical on every later dispatch. `done` is not a stored bit: a sample is done when its `state` is not
//! `running` (payload §6, `sd_is_finished`) or its step budget ran out (`budget_bits`'s sticky `budget_exhausted` bit,
//! whose `state` stays `running`: undetermined, never marched on, R-603), with one exception. A `bounded` sample is finished under the horizon it
//! reached, and under a longer one it marches on. The plan is unclear here (it makes reaching `T` the label `bounded`
//! and `T` part of the sim key, and says nothing of a `bounded` state read under a longer `T`); chose to march it on
//! because under that key it has not reached its horizon, so it is not terminal there, and a march extended to a
//! longer horizon is how the readout and validation's δ-dependence measurement continue one (`tests/readout.rs`,
//! `validation::measure::delta_dep`). Collision, `sim_failed` and `decode_failed` are final under any key. An escape is
//! final under the stop option (`super::cadence::WrapperCadence`'s `stop_on_escape`); with the option off, the default,
//! the escape label is live and an escape short of the horizon marches on as a short `bounded` does.
//!
//! The post-escape march that pitfalls §2.4's checks need runs only in the validation harness, on its own state; the
//! payload never sees it.

use super::advance::{Advance, Cadence};
use super::carried::Carried;
use super::wrapper::{integrate_resumed_carrying, WrapperReal, WrapperUniforms};
use crate::decode::Ic;
use crate::payload::{
    sd_state, tm_t_end_step, PayloadReal, SimStateBaseOf, SimStateFTLEOf, STATE_BOUNDED,
    STATE_ESCAPE, STATE_RUNNING,
};
use crate::regularisation::budget::exhausted;

/// A stored `SimState` of any tier: what the latch reads of it, `packed_a`'s `state` and `times`' `t_end_step`.
pub trait Stored: Copy {
    /// The descriptor word.
    fn packed_a(&self) -> u32;
    /// The step indices' word.
    fn times(&self) -> u32;
    /// The budget word (payload §2; R-603).
    fn budget_bits(&self) -> u32;
}

impl<R: PayloadReal> Stored for SimStateBaseOf<R> {
    #[inline]
    fn packed_a(&self) -> u32 {
        self.packed_a
    }

    #[inline]
    fn times(&self) -> u32 {
        self.times
    }

    #[inline]
    fn budget_bits(&self) -> u32 {
        self.budget_bits
    }
}

impl<R: PayloadReal> Stored for SimStateFTLEOf<R> {
    #[inline]
    fn packed_a(&self) -> u32 {
        self.packed_a
    }

    #[inline]
    fn times(&self) -> u32 {
        self.times
    }

    #[inline]
    fn budget_bits(&self) -> u32 {
        self.budget_bits
    }
}

/// Whether a stored sample, its `packed_a`, `times` and `budget_bits`, is finished under a sim key of `horizon_steps`
/// (the module's note): any `state` but `running`, except `bounded` short of the horizon, and `escape` short of it where
/// the escape label is not final (`escape_final`, the stop option off); and a `running` sample whose step budget ran
/// out, the sticky `budget_exhausted` bit set, which is undetermined and never marched on: its remedy is finer steps as
/// an explicit re-run (R-603; `crate::regularisation::budget`).
#[inline]
pub fn finished(
    packed_a: u32,
    times: u32,
    budget_bits: u32,
    horizon_steps: u32,
    escape_final: bool,
) -> bool {
    let s = sd_state(packed_a);
    let short = tm_t_end_step(times) < horizon_steps;
    let open = s == STATE_BOUNDED || (s == STATE_ESCAPE && !escape_final);
    exhausted(budget_bits) || (s != STATE_RUNNING && !(open && short))
}

/// A later dispatch of the stored sample `stored` under a sim key of `horizon_steps`, its escape label final or not
/// (`escape_final`): `stored` itself, every word, where it is finished ([`finished`]), and otherwise what `dispatch`
/// makes of it.
#[inline]
pub fn latch<S: Stored, F: FnOnce(&S) -> S>(
    stored: &S,
    horizon_steps: u32,
    escape_final: bool,
    dispatch: F,
) -> S {
    if finished(
        stored.packed_a(),
        stored.times(),
        stored.budget_bits(),
        horizon_steps,
        escape_final,
    ) {
        *stored
    } else {
        dispatch(stored)
    }
}

/// A later dispatch of the base tier's stored sample `stored` of `ic` under `uniforms` by the occupant `A`, for at
/// most `steps` more macro-steps: `stored` word for word where it is finished, `carried` and `cadence` untouched;
/// otherwise the resumed march ([`integrate_resumed_carrying`], which hands `cadence` the stored state) and the
/// cadence's write ([`Cadence::finish`]). The wrapper's cadence reloads what it keeps from `stored`, so a cadence built
/// from `ic` and the sim key alone gives the uncut march's words (`super::cadence`'s note, "A re-boot").
#[inline]
pub fn redispatch<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
    stored: &SimStateBaseOf<R>,
    carried: &mut Carried<R>,
    steps: u32,
) -> SimStateBaseOf<R> {
    if finished(
        stored.packed_a,
        stored.times,
        stored.budget_bits,
        uniforms.horizon_steps,
        cadence.escape_final(),
    ) {
        *stored
    } else {
        let out =
            integrate_resumed_carrying::<A, R, C>(ic, uniforms, cadence, stored, carried, steps);
        cadence.finish(out)
    }
}
