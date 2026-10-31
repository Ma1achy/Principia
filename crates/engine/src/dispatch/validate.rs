//! Dispatch's refusals (integrator_contract Part 5, "Exact step-index timing invariant"; dd_simstate_payload §2,
//! `times`, and §8; R-86, R-502, R-610): what a [`SimUniforms`] must satisfy before anything is dispatched under it, each
//! refusal a typed [`DispatchError`] with its reason. [`validate`] checks, in order:
//!
//! - **The horizon's range**, `T_horizon ∈ [0.4, 200]` (R-610): `T` finite; at least one escape window, the carried
//!   `escape_window` (0.4, provisional, R-29), since below it the escape arm cannot fire, the floor following the
//!   window's value; at most the cap, 200, until long horizons are measured for cost. Dispatch refuses a `T` outside
//!   the range and never clamps it: a sim key is dispatched as given or not at all. A window that is not finite and
//!   positive is refused first, since the floor is read from it.
//! - **The horizon's steps**, `horizon_steps = ⌈T/dt_macro⌉ ≤ 65535`, evaluated in f64 ([`horizon_steps`]), with
//!   `dt_macro` finite and positive: the exact u16 `t_end_step` and `t_dmin_step` hold every step index only then, and
//!   there is no fallback format; a longer integration takes a coarser `dt_macro` or march epochs (R-86). `dt_macro`
//!   must stay finite rounded to f32 as well, since [`Validated::wrapper_uniforms_f32`] hands that value to the
//!   kernel, and its smallest substep, `dt_macro / N_max` at f32, must be a normal f32: infinity or 0 would make every
//!   substep's `dt` not a time step, and a backend that flushes subnormal results (Metal; determinism note rule 6,
//!   R-547; integrator contract Part 2c) reads a subnormal substep as 0 where the CPU keeps it.
//! - **The `N_sub` table** (R-502): the host generator's refusal of the carried `r_sub`, `gamma_sub` and `N_max`
//!   (`kernel::driver::nsub_table`), an `N_max` above the table's maximum length among them, is returned as it is;
//!   and the carried table must be the generator's for them, bit for bit, never another config's.
//! - **The joint bound** `horizon_steps × N_max ≤ 2³² − 1`, so the exact `total_substeps` u32 cannot overflow
//!   ([`check_total_substeps`]). With `horizon_steps ≤ 65535` and `N_max` at most the table's maximum length it holds
//!   for every accepted sim key; it is asserted all the same, as Part 5 asks.
//! - **The carried lengths and thresholds**, each refused with its own reason where the branch it feeds would
//!   silently read as "never" or not a number: `r_coll` and `r_close` finite and not negative (`r_coll = 0` is
//!   accepted, Part 7's "refuses to regularise"); `tau` finite and positive, since Part 7's escape test `|Δn̂| < tau`
//!   is strict and `|Δn̂|` is never below 0, so at `tau = 0` escape never fires; `eps_E` and `eps_L` finite and
//!   positive, since each is only the floor under a relative drift's denominator, `δE = ΔE / max(ε_E, |E₀|)`
//!   (dd_integrator §3.5), and with a floor not positive and `E₀ = 0`, on the `E` axis of an `(L_z, E)` chart, the
//!   drift is `0/0`.
//! - **The softening length**, `epsilon = 0`: no force law takes `epsilon`, and the corpus does not define softened
//!   gravity (dd_generation_root §3.7, "Open"; RQ-627), so a run with any other `epsilon`, NaN and ±∞ included, would
//!   come back Newtonian under the `softened` tag. It is refused until the force law is built.
//!
//! A sim key that passes is a [`Validated`], the one way to the kernel's `WrapperUniforms`; [`dispatch`] launches only
//! a validated one, so a refused sim key dispatches nothing.

use core::fmt;

use kernel::driver::nsub_table::{thresholds, NsubTableError};
use kernel::driver::wrapper::WrapperUniforms;
use ledger::constants::{HORIZON_STEPS_MAX, T_HORIZON_MAX};

use crate::contract::sim_uniforms::{horizon_steps, SimUniforms};

/// Why dispatch refused a sim key (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DispatchError {
    /// The escape window is not finite and positive, so the horizon has no floor.
    EscapeWindow {
        /// The window carried.
        escape_window: f64,
    },
    /// `T_horizon` is not finite.
    HorizonNotFinite {
        /// The `T` carried.
        t_horizon: f64,
    },
    /// `T_horizon` is below one escape window, where the escape arm cannot fire (R-610).
    HorizonBelowWindow {
        /// The `T` carried.
        t_horizon: f64,
        /// The escape window, the floor.
        escape_window: f64,
    },
    /// `T_horizon` is above the cap, 200 (R-610).
    HorizonAboveCap {
        /// The `T` carried.
        t_horizon: f64,
    },
    /// `dt_macro` is not finite and positive at f64, or not finite at f32, or its smallest substep, `dt_macro / N_max`
    /// at f32, is not a normal f32.
    DtMacro {
        /// The `dt_macro` carried.
        dt_macro: f64,
    },
    /// `⌈T/dt_macro⌉` is above 65535, which the exact u16 step indices cannot hold (R-86).
    HorizonSteps {
        /// The `T` carried.
        t_horizon: f64,
        /// The `dt_macro` carried.
        dt_macro: f64,
        /// `⌈T/dt_macro⌉` in f64.
        steps: f64,
    },
    /// The host generator refused the substep law (R-502).
    Nsub(NsubTableError),
    /// `r_coll` is not finite or is negative: at NaN `d² < r_coll²` is never true, so collisions would be silently off.
    CollisionRadius {
        /// The `r_coll` carried.
        r_coll: f64,
    },
    /// `tau` is not finite and positive: `|Δn̂| < tau` would never be true, so escape would be silently off.
    EscapeThreshold {
        /// The `tau` carried.
        tau: f64,
    },
    /// `r_close` is not finite or is negative, so the close-encounter count would never count.
    CloseDistance {
        /// The `r_close` carried.
        r_close: f64,
    },
    /// `eps_E` is not finite and positive: at `E₀ = 0` the relative energy drift would be `0/0` or `x/0`.
    EnergyDriftFloor {
        /// The `eps_E` carried.
        eps_e: f64,
    },
    /// `eps_L` is not finite and positive: at `L₀ = 0` the relative angular momentum drift would be `0/0` or `x/0`.
    AngularMomentumDriftFloor {
        /// The `eps_L` carried.
        eps_l: f64,
    },
    /// `epsilon` is not 0: softened gravity is not built (RQ-627).
    Softening {
        /// The `epsilon` carried.
        epsilon: f64,
    },
    /// The carried threshold table is not the host generator's for the carried `r_sub`, `gamma_sub` and `N_max`.
    StaleThresholds,
    /// `horizon_steps × N_max` is above `2³² − 1`, which the exact `total_substeps` u32 cannot hold.
    TotalSubsteps {
        /// `⌈T/dt_macro⌉`.
        horizon_steps: u32,
        /// The `N_max` carried.
        n_max: u32,
    },
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::EscapeWindow { escape_window } => write!(
                f,
                "the escape window {escape_window} is not finite and positive, so the horizon has no floor"
            ),
            Self::HorizonNotFinite { t_horizon } => {
                write!(f, "T_horizon {t_horizon} is not finite")
            }
            Self::HorizonBelowWindow {
                t_horizon,
                escape_window,
            } => write!(
                f,
                "T_horizon {t_horizon} is refused: below one closure window ({escape_window}) the escape arm cannot \
                 fire (R-610)"
            ),
            Self::HorizonAboveCap { t_horizon } => write!(
                f,
                "T_horizon {t_horizon} is refused: above the cap, {}, until long horizons are measured for cost \
                 (R-610)",
                T_HORIZON_MAX.number()
            ),
            Self::DtMacro { dt_macro } => {
                write!(
                    f,
                    "dt_macro {dt_macro} is not finite and positive at f64, or not finite at f32, or its smallest \
                     substep, dt_macro / N_max at f32, is not a normal f32"
                )
            }
            Self::HorizonSteps {
                t_horizon,
                dt_macro,
                steps,
            } => write!(
                f,
                "⌈T/dt_macro⌉ = ⌈{t_horizon}/{dt_macro}⌉ = {steps} macro-steps is above {}, which the exact u16 step \
                 indices cannot hold (R-86): use a coarser dt_macro or march epochs",
                HORIZON_STEPS_MAX.number()
            ),
            Self::Nsub(e) => write!(f, "{e}"),
            Self::CollisionRadius { r_coll } => write!(
                f,
                "r_coll {r_coll} is refused: not finite or negative, the collision branch would never fire"
            ),
            Self::EscapeThreshold { tau } => write!(
                f,
                "tau {tau} is refused: not finite and positive, the escape arm would never fire"
            ),
            Self::CloseDistance { r_close } => write!(
                f,
                "r_close {r_close} is refused: not finite or negative, no close encounter would be counted"
            ),
            Self::EnergyDriftFloor { eps_e } => write!(
                f,
                "eps_E {eps_e} is refused: the relative energy drift's floor is not finite and positive"
            ),
            Self::AngularMomentumDriftFloor { eps_l } => write!(
                f,
                "eps_L {eps_l} is refused: the relative angular momentum drift's floor is not finite and positive"
            ),
            Self::Softening { epsilon } => write!(
                f,
                "epsilon {epsilon} is refused: softened gravity is not built, no force law takes epsilon \
                 (RQ-627); only epsilon = 0 is dispatched"
            ),
            Self::StaleThresholds => write!(
                f,
                "the N_sub threshold table is not the host generator's for this r_sub, gamma_sub and N_max (R-502)"
            ),
            Self::TotalSubsteps {
                horizon_steps,
                n_max,
            } => write!(
                f,
                "horizon_steps × N_max = {horizon_steps} × {n_max} is above 2^32 − 1, which the exact \
                 total_substeps u32 cannot hold"
            ),
        }
    }
}

impl From<NsubTableError> for DispatchError {
    fn from(e: NsubTableError) -> Self {
        Self::Nsub(e)
    }
}

/// A sim key dispatch accepted ([`validate`]), with its `horizon_steps`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Validated {
    uniforms: SimUniforms,
    horizon_steps: u32,
}

impl Validated {
    /// The sim key accepted.
    pub fn uniforms(&self) -> &SimUniforms {
        &self.uniforms
    }

    /// `horizon_steps = ⌈T/dt_macro⌉`, at most 65535: the macro-steps to the horizon, and the denominator of the
    /// `times` display fractions.
    pub fn horizon_steps(&self) -> u32 {
        self.horizon_steps
    }

    /// The kernel wrapper's loop values at f64: `dt_macro`, `horizon_steps`, `N_max` and the table.
    pub fn wrapper_uniforms_f64(&self) -> WrapperUniforms<f64> {
        WrapperUniforms {
            dt_macro: self.uniforms.dt_macro,
            horizon_steps: self.horizon_steps,
            n_max: self.uniforms.n_max,
            thresholds: self.uniforms.thresholds,
        }
    }

    /// The kernel wrapper's loop values at f32: `dt_macro` rounded once to f32, the rest as at f64.
    pub fn wrapper_uniforms_f32(&self) -> WrapperUniforms<f32> {
        WrapperUniforms {
            dt_macro: self.uniforms.dt_macro as f32,
            horizon_steps: self.horizon_steps,
            n_max: self.uniforms.n_max,
            thresholds: self.uniforms.thresholds,
        }
    }
}

/// The joint bound `horizon_steps × N_max ≤ 2³² − 1` (Part 5; payload §8), in exact integer arithmetic.
pub fn check_total_substeps(horizon_steps: u32, n_max: u32) -> Result<(), DispatchError> {
    if u64::from(horizon_steps) * u64::from(n_max) > u64::from(u32::MAX) {
        return Err(DispatchError::TotalSubsteps {
            horizon_steps,
            n_max,
        });
    }
    Ok(())
}

/// The horizon's range (R-610): `t` finite, at least `escape_window`, at most the cap.
fn check_horizon(t: f64, escape_window: f64) -> Result<(), DispatchError> {
    if !(escape_window.is_finite() && escape_window > 0.0) {
        return Err(DispatchError::EscapeWindow { escape_window });
    }
    if !t.is_finite() {
        return Err(DispatchError::HorizonNotFinite { t_horizon: t });
    }
    if t < escape_window {
        return Err(DispatchError::HorizonBelowWindow {
            t_horizon: t,
            escape_window,
        });
    }
    if t > T_HORIZON_MAX.number() {
        return Err(DispatchError::HorizonAboveCap { t_horizon: t });
    }
    Ok(())
}

/// A length that must be finite and not negative.
fn finite_not_negative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

/// A threshold or floor that must be finite and positive; `-0.0 > 0.0` is false, so ±0 are both refused.
fn finite_positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// The carried lengths and thresholds, and the softening length (the module's note).
fn check_carried(u: &SimUniforms) -> Result<(), DispatchError> {
    if !finite_not_negative(u.r_coll) {
        return Err(DispatchError::CollisionRadius { r_coll: u.r_coll });
    }
    if !finite_positive(u.tau) {
        return Err(DispatchError::EscapeThreshold { tau: u.tau });
    }
    if !finite_not_negative(u.r_close) {
        return Err(DispatchError::CloseDistance { r_close: u.r_close });
    }
    if !finite_positive(u.eps_e) {
        return Err(DispatchError::EnergyDriftFloor { eps_e: u.eps_e });
    }
    if !finite_positive(u.eps_l) {
        return Err(DispatchError::AngularMomentumDriftFloor { eps_l: u.eps_l });
    }
    // NaN != 0, so NaN is refused here too.
    if u.epsilon != 0.0 {
        return Err(DispatchError::Softening { epsilon: u.epsilon });
    }
    Ok(())
}

/// `uniforms` checked against every refusal of the module's note, in its order: the accepted sim key with its
/// `horizon_steps`, or the first refusal.
pub fn validate(uniforms: &SimUniforms) -> Result<Validated, DispatchError> {
    let (t, dt) = (uniforms.t_horizon, uniforms.dt_macro);
    check_horizon(t, uniforms.escape_window)?;
    if !(dt.is_finite() && dt > 0.0) {
        return Err(DispatchError::DtMacro { dt_macro: dt });
    }
    let steps = horizon_steps(t, dt);
    if steps > HORIZON_STEPS_MAX.number() {
        return Err(DispatchError::HorizonSteps {
            t_horizon: t,
            dt_macro: dt,
            steps,
        });
    }
    let dt_f32 = dt as f32;
    if !(dt_f32.is_finite() && dt_f32 / uniforms.n_max as f32 >= f32::MIN_POSITIVE) {
        return Err(DispatchError::DtMacro { dt_macro: dt });
    }
    if thresholds(&uniforms.substep_law())? != uniforms.thresholds {
        return Err(DispatchError::StaleThresholds);
    }
    // `steps` is a whole number in [1, 65535] here: `t` and `dt` are finite and positive, and their quotient's
    // ceiling is at most 65535.
    let horizon_steps = steps as u32;
    check_total_substeps(horizon_steps, uniforms.n_max)?;
    check_carried(uniforms)?;
    Ok(Validated {
        uniforms: *uniforms,
        horizon_steps,
    })
}

/// Validates `uniforms` and, only if dispatch accepts it, calls `launch` with the validated sim key, once; a refused
/// sim key returns its refusal and launches nothing.
pub fn dispatch<T>(
    uniforms: &SimUniforms,
    launch: impl FnOnce(&Validated) -> T,
) -> Result<T, DispatchError> {
    let validated = validate(uniforms)?;
    Ok(launch(&validated))
}
