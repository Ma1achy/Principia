//! The stepper occupants of the integrator's swappable `STEP` slot and their capability profile (integrator_contract
//! Part 2, Part 2a; dd_integrator §2, §3.2).
//!
//! **A closed, build-time set** (canonical_spec §1 item 4; gui_state_contract §3, §5; REQ-INT-005). Each occupant is a
//! Rust type implementing [`Stepper`], generic over the kernel's float type and monomorphised by the compiler; the set
//! is the enums [`Occupant`] (the production steppers), [`DebugOccupant`] (Euler) and [`Selection`], one of either. No
//! code path compiles, loads or interprets an occupant at run time, and there is no user-authored compute occupant: a
//! new stepper is a new type here and a new build.
//!
//! **The capability profile** (Part 2; REQ-INT-013): every occupant advertises a [`CapabilityProfile`], `{order,
//! force_evals, symplectic, step_reversible, run_reversible, owns_time_mapping}` (R-19, R-500; the re-registration
//! field is Part 2b's, added with the regularisations), and downstream features read the profile, never the occupant's
//! name: [`CapabilityProfile::drift_reads_dynamics`] for energy drift, [`CapabilityProfile::reversibility_diagnostic`]
//! and [`CapabilityProfile::reversibility_test`] for reversal, and [`CapabilityProfile::resolution_bits`],
//! [`CapabilityProfile::force_evaluations`] and [`CapabilityProfile::ftle_confidence`] for the `total_substeps_log2`
//! complexity proxy and FTLE confidence, Part 2's definition (REQ-INT-078). [`table`] is the Part 2 / Part 2a table and
//! each occupant's default tier binding.
//!
//! **One `STEP`** (`fn step(state, dt, geom)`): an occupant takes a [`State`], `(m, r, p)` (the canonicalise seam's;
//! core design seam 5), a step `dt`, and the force evaluation at the state's positions, [`Geom`], and returns the state
//! one step on with the force evaluation at its positions, which the next step and the wrapper's monitoring read
//! (REQ-PERF-001: each evaluation's pairwise distances computed once). So an occupant's [`Geom::new`] calls per step
//! are its profile's `force_evals`. A step holds no data-dependent branch, no loop and no runtime index (integrator_contract
//! Part 2c);
//! its numbers are the generated [`StepperCoefficients`] (the constants register's, dd_generation_root §3.8) and the
//! structural `1`, `2` and `6` of the update rules, formed from [`One`].

pub mod euler;
pub mod kdk;
pub mod rk4;
pub mod table;
pub mod yoshida4;
pub mod yoshida6;

use spirv_std::num_traits::One;

pub use crate::generated::steppers::StepperCoefficients;
use crate::physics::force::force;
use crate::physics::PairGeom;
use crate::Real;

/// The float type a stepper runs at: the kernel's [`Real`] (f32 on the GPU, f64 on the CPU; R-265), with `1` and the
/// steppers' coefficients.
pub trait StepReal: Real + One + StepperCoefficients {}

impl<R: Real + One + StepperCoefficients> StepReal for R {}

/// The state a stepper advances: the masses, positions and momenta, `(m, r, p)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State<R> {
    /// The masses `m₀, m₁, m₂`.
    pub m: [R; 3],
    /// The positions `r₀, r₁, r₂`.
    pub r: [[R; 2]; 3],
    /// The momenta `p₀, p₁, p₂`.
    pub p: [[R; 2]; 3],
}

/// One force evaluation at a state's positions: their pairwise geometry and the accelerations read from it
/// (dd_integrator §3.1). The geometry is the one the energy and `r_min` read (REQ-PERF-001).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geom<R> {
    /// The pairwise geometry of the positions.
    pub pair: PairGeom<R>,
    /// The accelerations `aᵢ`.
    pub accel: [[R; 2]; 3],
}

impl<R: Real> Geom<R> {
    /// The force evaluation at positions `r` of bodies of masses `m`: one [`PairGeom`], then the force from it.
    #[inline]
    pub fn new(m: &[R; 3], r: &[[R; 2]; 3]) -> Self {
        let pair = PairGeom::new(r);
        Geom {
            accel: force(m, &pair),
            pair,
        }
    }
}

/// What an occupant advertises (integrator_contract Part 2, Part 2a; R-19, R-500), read by every feature whose meaning
/// depends on the occupant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityProfile {
    /// The order `p`: a step of size `h` commits a local error `O(h^{p+1})`.
    pub order: u32,
    /// Force evaluations per `STEP`.
    pub force_evals: u32,
    /// Whether a step is a symplectic map.
    pub symplectic: bool,
    /// Whether the `STEP` is time-symmetric at a fixed step: `STEP(dt)`, momentum negation, `STEP(dt)`, negation
    /// returns the state to rounding level, with no wrapper projection or substepping (dd_integrator §5, test 3).
    pub step_reversible: bool,
    /// Whether the bound occupant's whole run is time-symmetric, as the reversibility test binds it: logH's TTL time
    /// mode only (R-162).
    pub run_reversible: bool,
    /// Whether the occupant, not the wrapper, maps its steps to physical time (Part 2a): true for the regularised
    /// compositions.
    pub owns_time_mapping: bool,
}

/// FTLE confidence as the profile reads it (integrator_contract Part 2, REQ-INT-078): whether the occupant's own error
/// leaves the exponent unbiased in kind, and the run's resolution in bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FtleConfidence {
    /// Whether the occupant is symplectic, so that its error does not bias the exponent by a numerical dissipation or
    /// excitation of phase-space volume.
    pub volume_preserving: bool,
    /// [`CapabilityProfile::resolution_bits`] of the run.
    pub resolution_bits: u32,
}

impl CapabilityProfile {
    /// Whether energy drift measures the dynamics (close-encounter stress) rather than the integrator: on a symplectic
    /// occupant only (Part 2, the first consequence).
    #[inline]
    pub const fn drift_reads_dynamics(&self) -> bool {
        self.symplectic
    }

    /// Whether the occupant may serve Part 6's reversibility diagnostic, either route: `step_reversible` (R-500).
    #[inline]
    pub const fn reversibility_diagnostic(&self) -> bool {
        self.step_reversible
    }

    /// Whether the occupant is the one the reversibility test binds: `run_reversible` (R-162, R-500).
    #[inline]
    pub const fn reversibility_test(&self) -> bool {
        self.run_reversible
    }

    /// Whether the occupant's substep count is the wrapper's law (`owns_time_mapping` false), the same for every such
    /// occupant on the same trajectory, rather than its own schedule (Part 2a, Cost 2).
    #[inline]
    pub const fn substeps_follow_wrapper(&self) -> bool {
        !self.owns_time_mapping
    }

    /// The force evaluations a run of `total_substeps` steps made: `total_substeps · force_evals`, the cost the proxy
    /// compares across occupants.
    #[inline]
    pub const fn force_evaluations(&self, total_substeps: u32) -> u64 {
        total_substeps as u64 * self.force_evals as u64
    }

    /// The run's resolution in bits, `order · (total_substeps_log2 − step_count_log2)`: each doubling of the steps per
    /// macro-step is worth `order` bits of accuracy (Part 2's definition). `step_count_log2` is `⌊log₂ step_count⌋` of
    /// the macro-steps taken, as `total_substeps_log2` is of the steps; `total_substeps ≥ step_count`, so the difference
    /// is at least 0 up to the floors, and a floor that inverts it reads 0.
    #[inline]
    pub const fn resolution_bits(&self, total_substeps_log2: u32, step_count_log2: u32) -> u32 {
        self.order * total_substeps_log2.saturating_sub(step_count_log2)
    }

    /// FTLE confidence (Part 2's definition): `symplectic`, and the run's [`CapabilityProfile::resolution_bits`].
    #[inline]
    pub const fn ftle_confidence(
        &self,
        total_substeps_log2: u32,
        step_count_log2: u32,
    ) -> FtleConfidence {
        FtleConfidence {
            volume_preserving: self.symplectic,
            resolution_bits: self.resolution_bits(total_substeps_log2, step_count_log2),
        }
    }
}

/// A stepper occupant: one `STEP` and its profile.
pub trait Stepper {
    /// The occupant's capability profile (integrator_contract Part 2): interface data, what the occupant advertises
    /// at the seam, not a register constant (REQ-SYS-082).
    fn profile() -> CapabilityProfile;

    /// The state `s` one step `dt` on, and the force evaluation at its positions; `geom` is the force evaluation at
    /// `s`'s positions, `Geom::new(&s.m, &s.r)`.
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>);
}

/// The production steppers (integrator_contract Part 2): the occupants a tier binds by default or a user selects.
/// Euler is not one of them (REQ-INT-015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occupant {
    /// The KDK leapfrog ([`kdk::Kdk`]).
    Kdk,
    /// Yoshida's fourth-order composition ([`yoshida4::Yoshida4`]).
    Yoshida4,
    /// Yoshida's sixth-order composition, solution A ([`yoshida6::Yoshida6`]).
    Yoshida6,
    /// The classical fourth-order Runge–Kutta method, the validation cross-check ([`rk4::Rk4`]).
    Rk4,
}

impl Occupant {
    /// Every production stepper.
    pub const PRODUCTION: [Occupant; 4] = [
        Occupant::Kdk,
        Occupant::Yoshida4,
        Occupant::Yoshida6,
        Occupant::Rk4,
    ];

    /// The occupant's advertised profile.
    #[inline]
    pub fn profile(self) -> CapabilityProfile {
        match self {
            Occupant::Kdk => kdk::Kdk::profile(),
            Occupant::Yoshida4 => yoshida4::Yoshida4::profile(),
            Occupant::Yoshida6 => yoshida6::Yoshida6::profile(),
            Occupant::Rk4 => rk4::Rk4::profile(),
        }
    }

    /// One `STEP` of the occupant ([`Stepper::step`]).
    #[inline]
    pub fn step<R: StepReal>(self, s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        match self {
            Occupant::Kdk => kdk::Kdk::step(s, dt, geom),
            Occupant::Yoshida4 => yoshida4::Yoshida4::step(s, dt, geom),
            Occupant::Yoshida6 => yoshida6::Yoshida6::step(s, dt, geom),
            Occupant::Rk4 => rk4::Rk4::step(s, dt, geom),
        }
    }
}

/// The debug occupants: reachable only through [`Selection::Debug`], never a tier default or a production option
/// (integrator_contract Part 2, "Euler is kept deliberately, as a debug occupant"; REQ-INT-015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugOccupant {
    /// Explicit Euler ([`euler::Euler`]), whose secular energy drift tests the diagnostic plumbing.
    Euler,
}

impl DebugOccupant {
    /// Every debug occupant.
    pub const DEBUG: [DebugOccupant; 1] = [DebugOccupant::Euler];

    /// The occupant's advertised profile.
    #[inline]
    pub fn profile(self) -> CapabilityProfile {
        match self {
            DebugOccupant::Euler => euler::Euler::profile(),
        }
    }

    /// One `STEP` of the occupant ([`Stepper::step`]).
    #[inline]
    pub fn step<R: StepReal>(self, s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        match self {
            DebugOccupant::Euler => euler::Euler::step(s, dt, geom),
        }
    }
}

/// The stepper a sim key binds (integrator_contract Part 3): a production stepper, or, from a debug selection only, a
/// debug occupant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    /// A production stepper.
    Production(Occupant),
    /// A debug occupant, selected in a debug view only.
    Debug(DebugOccupant),
}

impl Selection {
    /// The selected occupant's advertised profile.
    #[inline]
    pub fn profile(self) -> CapabilityProfile {
        match self {
            Selection::Production(o) => o.profile(),
            Selection::Debug(o) => o.profile(),
        }
    }

    /// One `STEP` of the selected occupant ([`Stepper::step`]).
    #[inline]
    pub fn step<R: StepReal>(self, s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        match self {
            Selection::Production(o) => o.step(s, dt, geom),
            Selection::Debug(o) => o.step(s, dt, geom),
        }
    }
}
