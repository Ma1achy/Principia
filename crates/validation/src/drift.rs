//! The drift measurements behind the drift suspect predicates' thresholds and the drift views' `lin` range
//! (TASK-M3-05; REQ-INT-085, REQ-TOOL-150; R-71, R-381, R-382): every occupant's energy and `L_z` drift, final and max,
//! through the one wrapper (`kernel::driver::wrapper`) and its monitor (`kernel::driver::monitor`), on dd_integrator §5
//! test 2's long bounded orbit and test 7's close-encounter IC, at CPU-f32 (standing in for GPU-f32, as REQ-INT-030
//! does) and at CPU-f64.
//!
//! **The fixtures** (dd_integrator §5, tests 2 and 7, the R-72 definitions TASK-M3-05 writes there):
//! - **test 2, the long bounded orbit** ([`figure_eight`]): the Chenciner–Montgomery figure-eight (dd_validation_orbits
//!   §1.2), equal masses, `L_z = 0`, stable to small perturbations, from its published initial conditions in `G = 1`,
//!   `mᵢ = 1` (`x₁ = −x₂ = (0.97000436, −0.24308753)`, `x₃ = 0`, `v₃ = (−0.93240737, −0.86473146)`,
//!   `v₁ = v₂ = −v₃/2`), brought to the sim's units, `M = 1`, `G = 1`, `I = 1` (integrator_contract Part 5): the
//!   masses `1/3`, the velocities scaled by `√(1/3)` so the orbit keeps its shape, then canonicalised
//!   (`kernel::canon::canonicalise`), and run to the horizon cap `T = 200` (Part 5), its longest;
//! - **test 7, the close-encounter IC** ([`close_encounter`]): a hierarchical triple whose inner binary's pericentre
//!   passages are the close encounters ([`hierarchy`]): masses `(0.45, 0.45, 0.1)`, the binary `a = 1`, `e = 0.9`,
//!   started at apocentre, the third body on a circular orbit at `d = 4` about the binary's centre of mass,
//!   canonicalised, run to the default horizon `T = 50`. Its pericentre in the canonical units, about `0.067`, stays
//!   above the substep law's `r_sub = 0.05`, so every step is one substep and the encounters are resolved by the step
//!   itself: a spike in `|ΔE|` at each passage that recovers after it, the shape test 7 asks for. The Burrau IC, the
//!   golden anchor, is not used: its encounters are deep enough that the substep law saturates and the f32 runs'
//!   drifts do not recover, nor stay round-off in `L_z`, so it cannot carry R-382's criterion.
//!
//! Both at the default sim key otherwise: `dt_macro = max(1e-3, T/65535)` (R-132), `horizon_steps = ⌈T/dt_macro⌉`,
//! and the default substep law (`r_sub = 0.05`, `γ_sub = 1.5`, `N_max = 64`).

use kernel::canon::canonicalise;
use kernel::decode::Ic;
use kernel::driver::advance::Advance;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_with, monitored, WrapperReal, WrapperUniforms};
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, Stepper};
use kernel::payload::{pb_dE_max, pb_dLz_max, sd_saturated, sd_state, tm_t_end_step};

/// The figure-eight's published initial conditions in `G = 1`, `mᵢ = 1` (Chenciner & Montgomery 2000; Simó's
/// digits): `x₁`, and `v₃`; `x₂ = −x₁`, `x₃ = 0`, `v₁ = v₂ = −v₃/2`.
pub const FIGURE_EIGHT_X1: [f64; 2] = [0.970_004_36, -0.243_087_53];

/// The figure-eight's `v₃` (the module's note).
pub const FIGURE_EIGHT_V3: [f64; 2] = [-0.932_407_37, -0.864_731_46];

/// The sim's default horizon `T` (integrator_contract Part 5).
pub const T_DEFAULT: f64 = 50.0;

/// The horizon cap (integrator_contract Part 5; R-610).
pub const T_CAP: f64 = 200.0;

/// The relative-drift floors `ε_E = ε_L = 10⁻⁶`, the sim key's defaults (dd_integrator §3.5; integrator_contract
/// Part 3).
pub const EPS_DEFAULT: f64 = 1e-6;

/// A fixture: its name, its IC at f64 and its horizon `T`.
#[derive(Clone, Copy, Debug)]
pub struct Fixture {
    pub name: &'static str,
    pub ic: Ic<f64>,
    pub horizon: f64,
}

/// `x` in the canonical frame (CoM at the origin, `P = 0`, `I = 1`, `ρ̃` on `+x`), the masses kept.
fn canonical(ic: &Ic<f64>) -> Ic<f64> {
    canonicalise(ic).ic
}

/// dd_integrator §5 test 2's long bounded orbit: the figure-eight in the sim's units (the module's note), to `T_CAP`.
pub fn figure_eight() -> Fixture {
    let third: f64 = 1.0 / 3.0;
    let s = third.sqrt();
    let [x, y] = FIGURE_EIGHT_X1;
    let [vx, vy] = FIGURE_EIGHT_V3;
    let v = [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]];
    let ic = Ic {
        m: [third; 3],
        r: [[x, y], [-x, -y], [0.0, 0.0]],
        p: v.map(|[a, b]| [third * s * a, third * s * b]),
    };
    Fixture {
        name: "test 2: the figure-eight",
        ic: canonical(&ic),
        horizon: T_CAP,
    }
}

/// Test 7's hierarchy: the masses, the inner binary's semi-major axis and eccentricity, and the outer body's distance
/// from the binary's centre of mass, before canonicalisation (the module's note).
pub const HIERARCHY_MASSES: [f64; 3] = [0.45, 0.45, 0.1];

/// The inner binary's semi-major axis `a` (test 7).
pub const HIERARCHY_A: f64 = 1.0;

/// The inner binary's eccentricity `e` (test 7).
pub const HIERARCHY_E: f64 = 0.9;

/// The outer body's distance `d` from the inner binary's centre of mass (test 7).
pub const HIERARCHY_D: f64 = 4.0;

/// A hierarchical triple: bodies 0 and 1 a Kepler binary of masses `m₀, m₁`, semi-major axis `a` and eccentricity
/// `e`, started at apocentre, and body 2 on a circular orbit of radius `d` about the binary's centre of mass, in
/// `G = 1`, the whole at rest in its centre of mass; canonicalised. Also returns the binary's pericentre in the
/// canonical units, `a(1 − e)·λ`, `λ` the canonicalisation's length scale.
pub fn hierarchy(m: [f64; 3], a: f64, e: f64, d: f64) -> (Ic<f64>, f64) {
    let [m0, m1, m2] = m;
    let mb = m0 + m1;
    let mt = mb + m2;
    let ra = a * (1.0 + e);
    let va = (mb * (1.0 - e) / (a * (1.0 + e))).sqrt();
    let vout = (mt / d).sqrt();
    let (xb, vb) = (-m2 / mt * d, -m2 / mt * vout);
    let (x2, v2) = (mb / mt * d, mb / mt * vout);
    let ic = Ic {
        m,
        r: [[-m1 / mb * ra + xb, 0.0], [m0 / mb * ra + xb, 0.0], [x2, 0.0]],
        p: [
            [0.0, m0 * (-m1 / mb * va + vb)],
            [0.0, m1 * (m0 / mb * va + vb)],
            [0.0, m2 * v2],
        ],
    };
    let c = canonicalise(&ic);
    (c.ic, a * (1.0 - e) * c.lambda)
}

/// dd_integrator §5 test 7's close-encounter IC: the resolved hierarchy (the module's note), to `T_DEFAULT`.
pub fn close_encounter() -> Fixture {
    Fixture {
        name: "test 7: the close-encounter hierarchy",
        ic: hierarchy(HIERARCHY_MASSES, HIERARCHY_A, HIERARCHY_E, HIERARCHY_D).0,
        horizon: T_DEFAULT,
    }
}

/// The sim key at horizon `t`: `dt_macro = max(1e-3, t/65535)`, `horizon_steps = ⌈t/dt_macro⌉`, the default
/// substep law.
pub fn sim_key<R: WrapperReal>(t: f64) -> WrapperUniforms<R>
where
    f64: AsR<R>,
{
    let dt = (t / 65535.0).max(1e-3);
    let config = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: dt.as_r(),
        horizon_steps: (t / dt).ceil() as u32,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default sim key generates"),
    }
}

/// One run's drifts, as the monitor latched them at the run's precision (`live`) and as the payload stores them (the
/// f16 latches, `stored`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drifts {
    /// `E₀`.
    pub e_0: f64,
    /// `L_{z,0}`.
    pub lz_0: f64,
    /// `ΔE_final`, signed.
    pub de_final: f64,
    /// `max|ΔE|`.
    pub de_max: f64,
    /// `ΔL_z` final, signed.
    pub dlz_final: f64,
    /// `max|ΔL_z|`.
    pub dlz_max: f64,
    /// The stored `dE_max`, the f16 latch.
    pub de_max_stored: f64,
    /// The stored `dLz_max`, the f16 latch.
    pub dlz_max_stored: f64,
    /// The stored `E_0`, rounded to f32 as the read side reads it.
    pub e_0_stored: f32,
    /// The macro-steps completed.
    pub steps: u32,
    /// The run's `state` label.
    pub state: u32,
    /// Whether the substep law hit `N_max` (the stored `saturated` bit): the step outran the law's ratio somewhere.
    pub saturated: bool,
}

impl Drifts {
    /// `δ_E` of the final drift, `ΔE_final / max(ε, |E₀|)`.
    pub fn rel_e_final(&self, eps: f64) -> f64 {
        self.de_final / eps.max(self.e_0.abs())
    }

    /// The relative form of `max|ΔE|`.
    pub fn rel_e_max(&self, eps: f64) -> f64 {
        self.de_max / eps.max(self.e_0.abs())
    }
}

/// A run of the occupant `A` at `R` from `fixture`, through the wrapper and its cadence.
pub fn run<A: Advance, R: WrapperReal + Into<f64>>(fixture: &Fixture) -> Drifts
where
    f64: AsR<R>,
{
    let ic = Ic {
        m: fixture.ic.m.map(AsR::as_r),
        r: fixture.ic.r.map(|v| v.map(AsR::as_r)),
        p: fixture.ic.p.map(|v| v.map(AsR::as_r)),
    };
    let key = sim_key::<R>(fixture.horizon);
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let out = monitored(
        integrate_with::<A, R, _>(&ic, &key, &mut cadence),
        &cadence,
    );
    let m = cadence.monitor_state();
    Drifts {
        e_0: m.e_0().into(),
        lz_0: m.lz_0().into(),
        de_final: m.de_final().into(),
        de_max: m.de_max().into(),
        dlz_final: m.dlz_final().into(),
        dlz_max: m.dlz_max().into(),
        de_max_stored: f64::from(pb_dE_max(out.packed_b)),
        dlz_max_stored: f64::from(pb_dLz_max(out.packed_b)),
        e_0_stored: {
            let e: f64 = out.E_0.into();
            e as f32
        },
        steps: tm_t_end_step(out.times),
        state: sd_state(out.packed_a),
        saturated: sd_saturated(out.packed_a),
    }
}

/// An f64 IC value at the run's precision: itself at f64, rounded to the nearest f32 at f32.
pub trait AsR<R> {
    fn as_r(self) -> R;
}

impl AsR<f64> for f64 {
    fn as_r(self) -> f64 {
        self
    }
}

impl AsR<f32> for f64 {
    fn as_r(self) -> f32 {
        self as f32
    }
}

/// An occupant measured: its name, its profile, and its runs at f32 and f64.
#[derive(Clone, Copy, Debug)]
pub struct Measured {
    pub occupant: &'static str,
    pub profile: CapabilityProfile,
    pub f32: Drifts,
    pub f64: Drifts,
}

fn both<S: Stepper>(occupant: &'static str, fixture: &Fixture) -> Measured {
    Measured {
        occupant,
        profile: S::profile(),
        f32: run::<S, f32>(fixture),
        f64: run::<S, f64>(fixture),
    }
}

/// Every occupant measured on `fixture`: Euler, RK4, KDK, Yoshida-4, Yoshida-6, at f32 and f64.
pub fn measure(fixture: &Fixture) -> Vec<Measured> {
    vec![
        both::<Euler>("Euler", fixture),
        both::<Rk4>("RK4", fixture),
        both::<Kdk>("KDK", fixture),
        both::<Yoshida4>("Yoshida-4", fixture),
        both::<Yoshida6>("Yoshida-6", fixture),
    ]
}
