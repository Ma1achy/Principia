//! The failure path (payload §1's storage rules, §2's `sim_failed` and `decode_failed`; integrator_contract Part 2b,
//! step 7; philosophy §4.3): non-finite detection on every step, the cause a failure is recorded with, and the one
//! writer of a failed sample's stored values.
//!
//! **A failure is a measurement, never missing data** (philosophy §4.3). A march that goes non-finite ends there with
//! `state` `sim_failed`, its `detail` the cause, and `t_end_step` the macro-steps completed before it: stored and
//! counted like any outcome, never dropped, masked or filtered by drift.
//!
//! **Detection is per step** ([`Detector`]), in the wrapper's cadence (`super::cadence::WrapperCadence`), which every
//! occupant calls after each internal step: on the state the step left, before the COM projection, so a `±∞` is read
//! as itself and not as the NaN that `∞ − ∞` makes of it there; then on the projected state; then on the monitor's
//! energy and `L_z`, a non-finite derived quantity on a finite state. A regularised march's own finiteness test (Part
//! 2b, step 7) rejects a state the cadence never sees, so the march reports its cause ([`Cadence::non_finite`]). The
//! first failure is the one recorded: the march ends on it (`done`), so a later NaN cannot rewrite an earlier `∞`.
//!
//! **The cause** (payload §2): on a state, 0 where any component is NaN, else 1 ([`by_state`]); on a finite state
//! whose energy or `L_z` is not finite, 2. A regularised march's step-7 exit is 2 where the step size that produced the
//! rejected state was not finite (an entry size, a derived quantity), otherwise read on that first rejected regularised
//! state ([`regularised`]; the contract's dated line of 13 Nov 2026).
//!
//! **The stored values** ([`sim_failed`], [`decode_failed`], [`failed_shadow`]): storage never holds NaN (payload §1,
//! "Storage never holds NaN"). Each f32 field of a failed sample holds the value payload §1's table defines: a value
//! the march measured before it failed where that value is finite, and a fixed sentinel otherwise, so no stored float of
//! any sample is a NaN and none of a failed sample's is the non-finite value that failed it.
//!
//! [`Cadence::non_finite`]: super::advance::Cadence::non_finite

use super::advance::{Cadence, Reached};
use super::benettin::s_failed;
use super::shape::NO_REFERENCE;
use super::wrapper::WrapperReal;
use crate::detect::label::{Fired, Label};
use crate::occupant::{Geom, State};
use crate::payload::{
    pack_dE_reg, pack_packed_b, set_I_max, set_coll_floor, set_d_min_unset, set_dmin_pair,
    set_t_dmin_step, PayloadReal, SimStateBaseOf, SimStateFTLEOf, SD_DMIN_PAIR_SENTINEL,
    STATE_DECODE_FAILED, STATE_SIM_FAILED,
};
use crate::real::IeeeBits;

/// `sim_failed`'s `detail` 0: a NaN in the state (payload §2; the ledger's code).
pub const DETAIL_NAN: u32 = crate::payload::DETAIL_SIM_FAILED_NAN_IN_STATE;
/// `sim_failed`'s `detail` 1: a `±∞` in the state, no NaN (payload §2; the ledger's code).
pub const DETAIL_INF: u32 = crate::payload::DETAIL_SIM_FAILED_INF_OVERFLOW_IN_STATE;
/// `sim_failed`'s `detail` 2: a non-finite derived quantity, or a regularised march's non-finite step size (payload §2;
/// the ledger's code).
pub const DETAIL_DERIVED: u32 = crate::payload::DETAIL_SIM_FAILED_NON_FINITE_DERIVED_QUANTITY;

/// The cause read on a state known not to be finite: [`DETAIL_NAN`] where any component is NaN, else [`DETAIL_INF`].
#[inline]
pub fn by_state(nan: bool) -> u32 {
    if nan {
        DETAIL_NAN
    } else {
        DETAIL_INF
    }
}

/// A regularised march's step-7 cause (the module's note): [`DETAIL_DERIVED`] where the step size that produced the
/// rejected state was not finite, else [`by_state`] on that state.
#[inline]
pub fn regularised(size_finite: bool, nan: bool) -> u32 {
    if size_finite {
        by_state(nan)
    } else {
        DETAIL_DERIVED
    }
}

/// Whether any of the twelve components of `(r, p)` is not finite, and whether any is NaN, each on its bits
/// (gpu_determinism_note § "The discipline", rule 4).
#[inline]
pub fn state_bits<R: IeeeBits>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> (bool, bool) {
    // Unrolled over the six rows at constant indices: a runtime index lowers to a bounds check, a multi-level exit
    // (gpu_determinism_note § "The discipline", rule 5; `tests/discipline_lint.rs`).
    let row = |(bad, nan): (bool, bool), [a, b]: [R; 2]| {
        (
            bad || a.non_finite_bits() || b.non_finite_bits(),
            nan || a.nan_bits() || b.nan_bits(),
        )
    };
    let rs = row(row(row((false, false), r[0]), r[1]), r[2]);
    row(row(row(rs, p[0]), p[1]), p[2])
}

/// The cause read on the state `(r, p)`: [`DETAIL_NAN`] for a NaN in it, else [`DETAIL_INF`] for a `±∞`, else
/// [`DETAIL_DERIVED`], the state finite.
#[inline]
pub fn state_detail<R: IeeeBits>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> u32 {
    let (bad, nan) = state_bits(r, p);
    if bad {
        by_state(nan)
    } else {
        DETAIL_DERIVED
    }
}

/// The first failure of a march (the module's note): whether one has fired, the step ordinal it fired on and its cause.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Detector {
    first: Fired,
}

impl Detector {
    /// No failure yet.
    #[inline]
    pub fn new() -> Self {
        Detector {
            first: Fired::none(),
        }
    }

    /// Whether a failure has fired.
    #[inline]
    pub fn fired(&self) -> bool {
        self.first.fired
    }

    /// The first failure, or [`Fired::none`].
    #[inline]
    pub fn failure(&self) -> Fired {
        self.first
    }

    /// Records a failure at the step ordinal `at` with `detail`, unless one is already recorded: the first stands.
    #[inline]
    pub fn fire(&mut self, at: u32, detail: u32) {
        if !self.first.fired {
            self.first = Fired::at(at, detail);
        }
    }

    /// Reads the state `(r, p)` of the step ordinal `at`: a failure, its cause read on the state, where it is not finite.
    /// Returns whether this state is not finite.
    #[inline]
    pub fn state<R: IeeeBits>(&mut self, at: u32, r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> bool {
        let (bad, nan) = state_bits(r, p);
        if bad {
            self.fire(at, by_state(nan));
        }
        bad
    }

    /// Reads whether a quantity derived from the finite state of the step ordinal `at` is not finite: a failure with
    /// [`DETAIL_DERIVED`] where it is.
    #[inline]
    pub fn derived(&mut self, at: u32, non_finite: bool) {
        if non_finite {
            self.fire(at, DETAIL_DERIVED);
        }
    }
}

/// The march's cadence with the failure path's record kept beside it (`super::wrapper`'s loop): every call passed on to
/// `inner`, and two things the wrapper needs whatever the cadence is. The failure a regularised march reports
/// ([`Cadence::non_finite`]), so its cause reaches the label under a cadence that keeps none; and the last finite state
/// the cadence left, so a failed sample stores a state the march reached, never the one that failed it.
pub struct Recorder<'a, R, C> {
    inner: &'a mut C,
    reported: Detector,
    read: bool,
    last: State<R>,
}

impl<'a, R: WrapperReal, C: Cadence<R>> Recorder<'a, R, C> {
    /// The record of a march from `start`, around `inner`.
    #[inline]
    pub fn new(inner: &'a mut C, start: &State<R>) -> Self {
        // A failure the cadence holds before the march (its IC's `E₀` not finite) is one it read itself.
        let read = inner.failure().fired;
        Recorder {
            inner,
            reported: Detector::new(),
            read,
            last: *start,
        }
    }

    /// The march's failure: the cadence's own where it recorded one, else the one the occupant reported.
    #[inline]
    pub fn failure(&self) -> Fired {
        let own = self.inner.failure();
        if own.fired {
            own
        } else {
            self.reported.failure()
        }
    }

    /// The march's failure where it ended with `reached`: [`Recorder::failure`], or, for an occupant that ended
    /// [`Reached::NonFinite`] without reporting a cause, the cause read on the state `out` it latched.
    #[inline]
    pub fn failure_at(&self, reached: Reached, out: &State<R>) -> Fired {
        let f = self.failure();
        if !f.fired && reached == Reached::NonFinite {
            Fired::at(0, state_detail(&out.r, &out.p))
        } else {
            f
        }
    }

    /// Whether the cadence read the failure itself, on a state it was handed: its per-step detection, not a report.
    #[inline]
    pub fn read_by_cadence(&self) -> bool {
        self.read
    }

    /// The last finite state the cadence left: the march's start before its first step.
    #[inline]
    pub fn last(&self) -> &State<R> {
        &self.last
    }
}

impl<R: WrapperReal, C: Cadence<R>> Cadence<R> for Recorder<'_, R, C> {
    #[inline]
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.read = self.read || self.inner.failure().fired;
        let (bad, _) = state_bits(&state.r, &state.p);
        if !bad && !self.failure().fired {
            self.last = *state;
        }
    }

    #[inline]
    fn done(&self) -> bool {
        self.inner.done() || self.reported.fired()
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
    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    #[inline]
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    #[inline]
    fn resume(&mut self, stored: &SimStateBaseOf<R>) {
        self.inner.resume(stored);
    }

    #[inline]
    fn latched(&mut self) {
        self.inner.latched();
    }

    #[inline]
    fn escape_final(&self) -> bool {
        self.inner.escape_final()
    }

    #[inline]
    fn non_finite(&mut self, detail: u32) {
        self.reported.fire(0, detail);
        self.inner.non_finite(detail);
    }

    #[inline]
    fn failure(&self) -> Fired {
        Recorder::failure(self)
    }
}

/// `x` where it is finite, else `sentinel`, tested on the bits.
#[inline]
fn finite_or<R: IeeeBits>(x: R, sentinel: R) -> R {
    if x.non_finite_bits() {
        sentinel
    } else {
        x
    }
}

/// `x` where it is not NaN, else `sentinel`: for a register whose `+∞` is its own unset value.
#[inline]
fn number_or<R: IeeeBits>(x: R, sentinel: R) -> R {
    if x.nan_bits() {
        sentinel
    } else {
        x
    }
}

/// `(r, p)` where all twelve components are finite, else every one 0.0: a state is kept whole or not at all.
#[inline]
fn whole_state<R: IeeeBits>(r: [[R; 2]; 3], p: [[R; 2]; 3]) -> ([[R; 2]; 3], [[R; 2]; 3]) {
    let (bad, _) = state_bits(&r, &p);
    if bad {
        let zero = [[R::default(); 2]; 3];
        (zero, zero)
    } else {
        (r, p)
    }
}

/// The words every failed sample shares (payload §1, §2): `packed_a` labelled `state` with `detail`, its `d_min` unset
/// and `dmin_pair` the sentinel 3; both drift halves 0.0; `t_dmin_step` 0; `coll_floor` 0, "not recorded".
#[inline]
fn failed_words<R: PayloadReal>(
    s: SimStateBaseOf<R>,
    state: u32,
    detail: u32,
) -> SimStateBaseOf<R> {
    let labelled = Label { state, detail }.write(s.packed_a);
    SimStateBaseOf {
        // `I_max`'s level 0, the fresh latch's (chazy drill-down §6): a failed sample has no motion to classify.
        packed_a: set_I_max(
            set_dmin_pair(set_d_min_unset(labelled), SD_DMIN_PAIR_SENTINEL),
            0,
        ),
        packed_b: pack_packed_b(0.0, 0.0),
        // The regularised drift's halves, 0.0 as `packed_b`'s (payload §2).
        dE_reg: pack_dE_reg(0.0, 0.0),
        times: set_t_dmin_step(s.times, 0),
        coll_floor_bits: set_coll_floor(s.coll_floor_bits, 0),
        // The re-boot's registers (payload §1, "Payload purity"): a failed sample is finished, never re-booted.
        _dmin_d2: 0,
        _march_bits: 0,
        ..s
    }
}

/// The one writer of `sim_failed` (the module's note): `s` labelled `sim_failed` with `detail`, every f32 field its
/// failed-state value (payload §1's table): `r`, `p` as given, the last finite state the march reached, kept whole
/// where all twelve are finite and 0.0 otherwise; `S` the failed mark ([`s_failed`]); `theta`, `mean_y`, `C_ty`,
/// `E_0`, `Lz_0` and the lagged register as given where finite, 0.0 where not; `closure_min` as given unless NaN, `+∞`
/// (unset) then. With the failed words: `d_min` unset, both drift halves 0.0, `dmin_pair` 3, `t_dmin_step` 0,
/// `coll_floor` 0, `dE_reg` 0.0 in both halves, `_dmin_d2` and `_march_bits` 0, `I_max`'s level 0. `t_end_step`, `total_substeps`, `budget_bits`, `saturated`, the departed
/// bit and `closure_step` are kept: the
/// march's counts up to its failure. Writing it twice gives the same words.
#[inline]
pub fn sim_failed<R: WrapperReal>(s: SimStateBaseOf<R>, detail: u32) -> SimStateBaseOf<R> {
    let zero = R::default();
    let (r, p) = whole_state(s.r, s.p);
    let words = failed_words(s, STATE_SIM_FAILED, detail);
    SimStateBaseOf {
        r,
        p,
        S: s_failed::<R>(),
        theta: finite_or(s.theta, zero),
        mean_y: finite_or(s.mean_y, zero),
        C_ty: finite_or(s.C_ty, zero),
        E_0: finite_or(s.E_0, zero),
        Lz_0: finite_or(s.Lz_0, zero),
        closure_min: number_or(s.closure_min, zero.or_pos_inf(true)),
        _nhat_lag_0: finite_or(s._nhat_lag_0, zero),
        _nhat_lag_1: finite_or(s._nhat_lag_1, zero),
        _nhat_lag_2: finite_or(s._nhat_lag_2, zero),
        ..words
    }
}

/// The stored sample of a decode that failed with `detail` (payload §1's table; [`crate::detect::t0`]): no march ran,
/// so every count is 0 and every f32 field its failed-state value: `r`, `p`, `theta`, `mean_y`, `C_ty`, `E_0`,
/// `Lz_0` and the lagged register 0.0, `S` the failed mark ([`s_failed`]), `closure_min` `+∞` (unset) with
/// `closure_step` 0 and `_reserved` "no reference"; with the failed words of [`sim_failed`].
#[inline]
pub fn decode_failed<R: WrapperReal>(detail: u32) -> SimStateBaseOf<R> {
    let blank = SimStateBaseOf {
        S: s_failed::<R>(),
        closure_min: R::default().or_pos_inf(true),
        closure_step: 0,
        _reserved: NO_REFERENCE as u16,
        ..SimStateBaseOf::default()
    };
    failed_words(blank, STATE_DECODE_FAILED, detail)
}

/// The FTLE tier's stored sample of the failed sample `s` ([`sim_failed`] or [`decode_failed`]): every field of `s`,
/// and the failed shadow, `r_sh`, `p_sh` the sample's own `r`, `p` and `S` the failed mark, the values a shadow
/// whose sum went non-finite takes.
#[inline]
pub fn failed_shadow<R: WrapperReal>(s: SimStateBaseOf<R>) -> SimStateFTLEOf<R> {
    super::benettin::ftle_of(s, s.r, s.p, s_failed::<R>())
}
