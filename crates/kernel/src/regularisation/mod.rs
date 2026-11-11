//! The regularisation axis: the second occupant slot, independent of the stepper (integrator_contract Part 2b, Part
//! 2a; R-19, R-561).
//!
//! **Two slots.** The stepper ([`crate::occupant`]) and the regularisation are chosen separately, and the bound
//! occupant is their composition, [`Composed<S, G>`], which implements the occupant seam [`Advance`]. The slot has Part
//! 2b's four kinds ([`Kind`]): `none`, Aarseth–Zare, Heggie and logH. Like the steppers, a regularisation is a Rust type
//! implementing [`Regularisation`], bound at build time.
//!
//! **`owns_time_mapping` is the composed occupant's** (Part 2a; R-19): [`Composed::profile`] reports it as
//! [`Kind::owns_time_mapping`], true whenever the regularisation is not `none`, whatever the regularisation's own
//! profile says.
//!
//! **`none`** ([`NoRegularisation`]) is built here: Cartesian, the independent control, always available. Its pairs are
//! separable, so each is the stepper's own form: the stepper's profile and the stepper's `ADVANCE`. Aarseth–Zare is
//! TASK-M3-68's and Heggie and logH TASK-M3-08's.
//!
//! [`pairs`] is Part 2b's pair table and the selection of a pair; [`dt_max`] is the per-trajectory largest-step
//! diagnostic with its "no usable step" sentinel (Part 2c).

pub mod dt_max;
pub mod pairs;

use core::marker::PhantomData;

use crate::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence};
use crate::occupant::{CapabilityProfile, Geom, State, Stepper};

/// The regularisation slot's four kinds (Part 2b).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `none`: Cartesian, the independent control.
    None,
    /// Aarseth–Zare: two pairs, re-registered at every sync boundary.
    AarsethZare,
    /// Heggie 1974 (global): three relative vectors, no reference body.
    Heggie,
    /// logH (algorithmic): a time transformation only.
    LogH,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Kind; 4] = [Kind::None, Kind::AarsethZare, Kind::Heggie, Kind::LogH];

    /// Whether an occupant composed with this regularisation owns its time mapping: whenever it is not `none` (Part
    /// 2a; R-19).
    #[inline]
    pub const fn owns_time_mapping(self) -> bool {
        !matches!(self, Kind::None)
    }
}

/// A regularisation occupant: its kind, and the pair it forms with a stepper `S`.
pub trait Regularisation {
    /// Which of the slot's kinds this is.
    const KIND: Kind;

    /// The profile of the pair with the stepper `S` (Part 2b: "the composed occupant's profile is the pair's, not the
    /// stepper's row"). Its `owns_time_mapping` is not read: [`Composed::profile`] sets that from [`Self::KIND`].
    fn profile<S: Stepper>() -> CapabilityProfile;

    /// The pair's `ADVANCE` ([`Advance::advance`]).
    fn advance<S: Stepper, R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R>;
}

/// `none`: no regularisation. The pair with a stepper is the stepper itself (Part 2b: "The `none` pairs are separable,
/// so each stepper's own form applies").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoRegularisation;

impl Regularisation for NoRegularisation {
    const KIND: Kind = Kind::None;

    #[inline]
    fn profile<S: Stepper>() -> CapabilityProfile {
        S::profile()
    }

    #[inline]
    fn advance<S: Stepper, R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        <S as Advance>::advance(state, geom, t_now, t_target, params, cadence)
    }
}

/// The composed occupant: the stepper `S` under the regularisation `G`. A type, never a value.
#[derive(Clone, Copy, Debug)]
pub struct Composed<S, G>(PhantomData<(S, G)>);

impl<S: Stepper, G: Regularisation> Composed<S, G> {
    /// The composed occupant's profile: the pair's ([`Regularisation::profile`]), with `owns_time_mapping` true
    /// whenever the regularisation is not `none` (Part 2a; R-19).
    #[inline]
    pub fn profile() -> CapabilityProfile {
        CapabilityProfile {
            owns_time_mapping: G::KIND.owns_time_mapping(),
            ..G::profile::<S>()
        }
    }
}

impl<S: Stepper, G: Regularisation> Advance for Composed<S, G> {
    #[inline]
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        G::advance::<S, R, C>(state, geom, t_now, t_target, params, cadence)
    }
}
