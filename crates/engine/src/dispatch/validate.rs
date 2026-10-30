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
//!   there is no fallback format; a longer integration takes a coarser `dt_macro` or march epochs (R-86).
//! - **The `N_sub` table** (R-502): the host generator's refusal of the carried `r_sub`, `gamma_sub` and `N_max`
//!   (`kernel::driver::nsub_table`), an `N_max` above the table's maximum length among them, is returned as it is;
//!   and the carried table must be the generator's for them, bit for bit, never another config's.
//! - **The joint bound** `horizon_steps × N_max ≤ 2³² − 1`, so the exact `total_substeps` u32 cannot overflow
//!   ([`check_total_substeps`]). With `horizon_steps ≤ 65535` and `N_max` at most the table's maximum length it holds
//!   for every accepted sim key; it is asserted all the same, as Part 5 asks.
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
    /// `dt_macro` is not finite and positive.
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
                write!(f, "dt_macro {dt_macro} is not finite and positive")
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
    if thresholds(&uniforms.substep_law())? != uniforms.thresholds {
        return Err(DispatchError::StaleThresholds);
    }
    // `steps` is a whole number in [1, 65535] here: `t` and `dt` are finite and positive, and their quotient's
    // ceiling is at most 65535.
    let horizon_steps = steps as u32;
    check_total_substeps(horizon_steps, uniforms.n_max)?;
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
