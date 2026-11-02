//! `SimUniforms`, the wrapper's loop parameters (integrator_contract Part 3, "Wrapper config — the loop"): constant
//! across a dispatch, bound once per frame and tier, and on the sim key, so a change to any of them re-integrates.
//! Each field is one row of Part 3's table, its default the table's, read from the constants register
//! (`ledger::constants`), and its owner the wrapper's loop bound, substepper, detector or monitor. Dispatch holds a
//! `SimUniforms` to its limits ([`crate::dispatch::validate`]).
//!
//! **The horizon** (Part 5; R-132, R-575, R-610). `T_horizon` is physical time in canonical units, default 50, raised
//! or lowered within `[0.4, 200]`, and `dt_macro = max(10⁻³, T/65535)`, so `⌈T/dt_macro⌉ ≤ 65535` at every `T` and the
//! u16 step indices of `times` stay exact (R-86). Both are carried, `dt_macro` as [`dt_macro_for`] gives it for `T`
//! ([`SimUniforms::with_horizon`]); dispatch checks the pair as carried.
//!
//! **The `N_sub` thresholds** (R-502; dd_integrator §3.3). `SimUniforms` carries the table TASK-M3-03's host generator
//! (`kernel::driver::nsub_table`) produces from its `r_sub`, `gamma_sub` and `N_max`, as exact f32 bit patterns of the
//! table's maximum length, `L_max`, laid out as the kernel reads them ([`NsubThresholds`]). The table is derived, so it
//! adds nothing to the sim key ([`SimUniforms::sim_key_bytes`]). [`SimUniforms::with_substep_law`] regenerates it with
//! the three values; the default carries the build-time table, which the host generator reproduces bit for bit at the
//! default config (REQ-INT-092).
//!
//! **Collision and softening** (dd_generation_root §3.7, "Open"; REQ-SYS-018). `r_coll` is nonzero by default and
//! `epsilon` 0 by default and optional, both lengths in canonical units (`√I = 1`), so gauge-covariant, and both fixed
//! at `t = 0`: each is one number, constant across the dispatch, and there is no co-moving option, since a co-moving
//! length makes the Hamiltonian time-dependent. Softened gravity is a different force law at any `epsilon`, so a run
//! with `epsilon > 0` is tagged ([`Softening`]) in its provenance and on its sim key, and never pooled with an
//! unsoftened run. No force law takes `epsilon` yet and the corpus does not define one (RQ-627), so dispatch refuses
//! any `epsilon` other than 0 ([`crate::dispatch::validate`]); the field, its tag and its sim-key byte stay.
//!
//! **Benettin's `n_renorm` and `δ₀`** (dd_integrator §3.8) have no value yet: they are REQ-INT-076's calibration, which
//! the task that needs them proposes. Until then each is `None`, and `Default` gives no number for them.

use kernel::driver::nsub_table::{pack, thresholds, NsubConfig, NsubTableError};
use kernel::driver::substep::NsubThresholds;
use kernel::generated::nsub_thresholds::DEFAULT_THRESHOLD_BITS;
use ledger::constants::{
    DT_MACRO_FLOOR, EPSILON, EPS_E, EPS_L, ESCAPE_WINDOW, G, HORIZON_STEPS_MAX, M_TOTAL, R_CLOSE,
    R_COLL, TAU, T_HORIZON_DEFAULT,
};

/// The wrapper's loop parameters (integrator_contract Part 3), with the `N_sub` threshold table generated from three
/// of them (R-502).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimUniforms {
    /// `T_horizon`, when to stop, physical time (Part 5): default 50, within `[0.4, 200]` (R-610).
    pub t_horizon: f64,
    /// `dt_macro`, the base macro-step: `max(10⁻³, T/65535)` for the carried `T` ([`dt_macro_for`]; R-132).
    pub dt_macro: f64,
    /// `N_max`, the substep cap per macro-step: default 64, user-adjustable, at most the table's maximum length (R-502).
    pub n_max: u32,
    /// `r_sub`, the substep law's length: default 0.05.
    pub r_sub: f64,
    /// `gamma_sub`, the substep law's exponent: default 1.5.
    pub gamma_sub: f64,
    /// `r_coll`, the collision threshold, a length in canonical units: default `10⁻³`, nonzero, user-exposed (Part 7).
    pub r_coll: f64,
    /// `tau`, the escape settling threshold on `|Δn̂|`: `10⁻³`, provisional (R-173) until REQ-EVT-025 confirms it.
    pub tau: f64,
    /// The escape window `|Δn̂|` is taken over, in time units: 0.4, provisional (R-29). The horizon's floor (R-610).
    pub escape_window: f64,
    /// `r_close`, the close-encounter counting distance: default 0.01.
    pub r_close: f64,
    /// `eps_E`, the relative energy drift's floor at rest starts: default `10⁻⁶`.
    pub eps_e: f64,
    /// `eps_L`, the relative angular momentum drift's floor at rest starts: default `10⁻⁶`.
    pub eps_l: f64,
    /// `G`, 1.
    pub g: f64,
    /// `M_total`, 1.
    pub m_total: f64,
    /// `epsilon`, the optional softening length, canonical and fixed at `t = 0`: default 0, unsoftened.
    pub epsilon: f64,
    /// `n_renorm`, Benettin's renormalisation interval in macro-steps: `None` until REQ-INT-076 calibrates it.
    pub n_renorm: Option<u32>,
    /// `δ₀`, Benettin's initial separation: `None` until REQ-INT-076 calibrates it.
    pub delta0: Option<f64>,
    /// The `N_sub` thresholds `THR[1] … THR[L_max]` for `r_sub` and `gamma_sub`, f32 bit patterns (R-502).
    pub thresholds: NsubThresholds,
}

impl Default for SimUniforms {
    /// Part 3's defaults, the horizon's `dt_macro`, and the build-time table, the default config's.
    fn default() -> Self {
        let law = NsubConfig::DEFAULT;
        let t = T_HORIZON_DEFAULT.number();
        SimUniforms {
            t_horizon: t,
            dt_macro: dt_macro_for(t),
            n_max: law.n_max,
            r_sub: law.r_sub,
            gamma_sub: law.gamma_sub,
            r_coll: R_COLL.number(),
            tau: TAU.number(),
            escape_window: ESCAPE_WINDOW.number(),
            r_close: R_CLOSE.number(),
            eps_e: EPS_E.number(),
            eps_l: EPS_L.number(),
            g: G.number(),
            m_total: M_TOTAL.number(),
            epsilon: EPSILON.number(),
            n_renorm: None,
            delta0: None,
            thresholds: pack(&DEFAULT_THRESHOLD_BITS),
        }
    }
}

/// `dt_macro` for the horizon `t`: `max(10⁻³, t/65535)` (R-132), rounded to f64, and raised by one ulp where the
/// f64 quotient `t/dt_macro` would otherwise round above 65535.
///
/// R-132 makes `⌈T/dt_macro⌉ ≤ 65535` hold at every `T`, which it does in exact arithmetic; dispatch evaluates it in
/// f64 ([`horizon_steps`]), where `t/65535` rounded down to the nearest f64 can give a quotient one ulp above 65535,
/// and a ceiling of 65536, for about one `T` in seven above 65.535 (65.538 is one). The one ulp, a relative change of
/// at most `2⁻⁵²`, restores the bound: with `dt ≥ (t/65535)(1 − 2⁻⁵³)` raised by an ulp, at least `dt·2⁻⁵³`, the exact
/// quotient is within `65535·2⁻¹⁰⁶` of 65535, far inside half its ulp, so it rounds to 65535 exactly. Where the floor
/// `10⁻³` is taken the same bound holds, `10⁻³` being at least `t/65535` rounded; one ulp is the most ever raised.
pub fn dt_macro_for(t: f64) -> f64 {
    let max_steps = HORIZON_STEPS_MAX.number();
    let dt = DT_MACRO_FLOOR.number().max(t / max_steps);
    if (t / dt).ceil() > max_steps {
        dt.next_up()
    } else {
        dt
    }
}

/// `⌈t/dt_macro⌉`, the macro-steps to the horizon, evaluated in f64: the quotient rounded once, then its ceiling.
/// Not finite, or not positive, where `t` or `dt_macro` makes the quotient so; dispatch refuses those
/// ([`crate::dispatch::validate`]).
pub fn horizon_steps(t: f64, dt_macro: f64) -> f64 {
    (t / dt_macro).ceil()
}

/// Whether a run is softened (dd_generation_root §3.7, "Open"): `epsilon > 0` is a different force law, tagged in the
/// run's provenance and on its sim key, never pooled with an unsoftened run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Softening {
    /// `epsilon` is 0: Newtonian gravity, unsoftened.
    Unsoftened = 0,
    /// `epsilon > 0`: softened gravity.
    Softened = 1,
}

impl Softening {
    /// The provenance tag a result carries: `None` for an unsoftened run, `"softened"` for a run with `epsilon > 0`.
    pub fn tag(self) -> Option<&'static str> {
        match self {
            Softening::Unsoftened => None,
            Softening::Softened => Some("softened"),
        }
    }
}

impl SimUniforms {
    /// `self` with the horizon `t` and its `dt_macro` ([`dt_macro_for`]).
    pub fn with_horizon(self, t: f64) -> Self {
        SimUniforms {
            t_horizon: t,
            dt_macro: dt_macro_for(t),
            ..self
        }
    }

    /// `self` with the substep law `r_sub`, `gamma_sub` and `n_max`, and the table the host generator produces from
    /// them, or the generator's refusal (R-502): an `n_max` below 1 or above the table's maximum length, an `r_sub` or
    /// `gamma_sub` not finite and positive, or a table the comparison tree cannot read.
    pub fn with_substep_law(
        self,
        r_sub: f64,
        gamma_sub: f64,
        n_max: u32,
    ) -> Result<Self, NsubTableError> {
        let law = NsubConfig {
            r_sub,
            gamma_sub,
            n_max,
        };
        Ok(SimUniforms {
            r_sub,
            gamma_sub,
            n_max,
            thresholds: thresholds(&law)?,
            ..self
        })
    }

    /// The substep law the table is generated from: `r_sub`, `gamma_sub` and `N_max`.
    pub fn substep_law(&self) -> NsubConfig {
        NsubConfig {
            r_sub: self.r_sub,
            gamma_sub: self.gamma_sub,
            n_max: self.n_max,
        }
    }

    /// Whether the run is softened: [`Softening::Softened`] where `epsilon > 0`. Dispatch refuses every `epsilon`
    /// other than 0 until softened gravity is built (RQ-627).
    pub fn softening(&self) -> Softening {
        if self.epsilon > 0.0 {
            Softening::Softened
        } else {
            Softening::Unsoftened
        }
    }

    /// `SimUniforms`' contribution to the sim key's bytes: each field in declaration order, a float by its f64 bits
    /// and an integer as a u32, big-endian; an optional field as a presence byte, 0 or 1, then its value when present;
    /// and last the softening tag, [`Softening`] as a byte. The threshold table is derived from `r_sub`, `gamma_sub`
    /// and `N_max`, so it adds nothing (R-502). The sim key's full composition is REQ-SCHED-007's (M4).
    pub fn sim_key_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for x in [self.t_horizon, self.dt_macro] {
            out.extend(x.to_bits().to_be_bytes());
        }
        out.extend(self.n_max.to_be_bytes());
        let floats = [
            self.r_sub,
            self.gamma_sub,
            self.r_coll,
            self.tau,
            self.escape_window,
            self.r_close,
            self.eps_e,
            self.eps_l,
            self.g,
            self.m_total,
            self.epsilon,
        ];
        for x in floats {
            out.extend(x.to_bits().to_be_bytes());
        }
        match self.n_renorm {
            Some(n) => {
                out.push(u8::from(true));
                out.extend(n.to_be_bytes());
            }
            None => out.push(u8::from(false)),
        }
        match self.delta0 {
            Some(d) => {
                out.push(u8::from(true));
                out.extend(d.to_bits().to_be_bytes());
            }
            None => out.push(u8::from(false)),
        }
        out.push(self.softening() as u8);
        out
    }
}
