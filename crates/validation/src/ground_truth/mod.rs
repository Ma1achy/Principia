//! The ground-truth suite (validation_ground_truth_note; TASK-M3-24): the CPU-f64 instantiation of the shared kernel,
//! the Precision ring (`engine::precision`), compared with truth from outside the system, theorems and the published
//! record, in four tiers ([`tiers`]). It is separate from parity: parity measures the GPU against the CPU, within stated
//! f32 tolerances; this suite says whether the CPU is right. Nothing here runs on the GPU, and no GPU result is
//! claimed correct by it: the GPU's agreement with the CPU is parity's to measure (the note's "correctness factoring").
//!
//! **Two entry paths**, routed by interior against degenerate (the note's "Two entry paths"):
//! - **Path A** ([`path_a`]), for a generic start: canonicalise, encode, CHECK 1 (the round trip's residual
//!   `‖D(z) − C(x)‖_phys` and `κ`), then the general forward pipeline, the chart decoded at a pixel and the wrapper's
//!   own march, exactly as a pixel runs, then CHECK 2 (the behaviour the tier asserts).
//! - **Path B** ([`path_b`]), for an exactly degenerate analytic configuration: the state `(m, r, p)` built from its
//!   closed form in the canonical gauge and handed to the wrapper's march and the shape map directly, the chart, the
//!   decoder, the encoder and canonicalisation bypassed.
//!
//! **The references are independent of the kernel they check.** A closed form, a published number, or the Precision
//! ring's RK45 (`engine::precision::rk45`), whose right-hand side is written there and not read from `kernel::physics`;
//! never the kernel's own stepper.
//!
//! **The fixture set** is `fixtures/ground_truth/fixtures.json` ([`Fixture`], [`fixtures`]): each start, its tier, its
//! path, its source and the quantities asserted on it. The note lists the same set.
//!
//! **Benchmarks carry their `r_coll`** ([`Provenance`]): a literature comparison assumes the reference computation's
//! regularisation, so every benchmark run records the `r_coll` it ran at, with its occupant, links and horizon
//! ([`BENCHMARK_R_COLL`] is the proposed pin).

pub mod path_a;
pub mod path_b;
pub mod tiers;

use std::path::PathBuf;

use serde::Deserialize;

use engine::compute::ComputeError;
use engine::contract::sim_config::{Horizon, SimConfig};
use engine::precision::march_key as sim_march_key;
/// A march's values at f64, re-exported for the kernel crate's anchor tests, which reach the engine only through this
/// dev-dependency.
pub use engine::precision::MarchKey64;
use kernel::detect::collision::CollisionRadius;
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::sync::delta_sync;

/// The benchmark collision radius, proposed: `0`, no collision terminal (integrator contract Part 7's "refuse to
/// regularise" end, under a regularised occupant, which integrates the encounters through).
///
/// **Derived from the reference computation's regularisation.** Szebehely & Peters (1967, AJ 72, 876) integrated the
/// Burrau problem with Levi-Civita regularisation and a fifth-order Runge–Kutta method (S&P pp. 881–883, as
/// `docs/incoming/2026-10-27_papers/valtonen2016/README.md` records): point masses, every binary encounter regularised
/// and integrated through, nothing terminated at a separation. The comparison is with that computation, so no
/// separation may end a march here either: `r_coll = 0`, with Heggie × RK4, the default occupant, standing in for
/// Levi-Civita. The default `10⁻³` is not the reference's: it labels as a collision any approach closer than
/// `10⁻³`, and the Burrau march's least separation is measured beside it (the test `entry_paths_burrau` prints it).
///
/// The plan is unclear here (a calibration proposed here and confirmed later); chose to hold the proposal in this
/// crate rather than the ledger's register, because the register is another task's file and the value is the
/// benchmark's alone, never a sim-key default.
pub const BENCHMARK_R_COLL: f64 = 0.0;

/// The canonical time per unit of Szebehely & Peters' time on the Burrau start: their units are `G = 1`, masses
/// `(3, 4, 5)`, sides `(3, 4, 5)`; the sim's are `G = 1`, `M = 1`, `I = 1`. The masses are divided by `M = 12`, which
/// multiplies time by `√12`, and `C` scales lengths by `λ = I_n^{−1/2}`, `I_n = 60/12 = 5`, which multiplies time by
/// `λ^{3/2}`: `√12·5^{−3/4} = 1.0360…`.
pub fn burrau_time_scale() -> f64 {
    12f64.sqrt() * 5f64.powf(-0.75)
}

/// Where a run came from and what it ran under: what a benchmark comparison must carry (integrator contract Part 7,
/// "Benchmarks pin it").
#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    /// The fixture's name.
    pub fixture: String,
    /// The entry path, `"A"` or `"B"`.
    pub path: &'static str,
    /// The occupant, by name.
    pub occupant: &'static str,
    /// The link selection, by name (Path A; Path B decodes nothing).
    pub selection: &'static str,
    /// The horizon `T` asked for, canonical units.
    pub horizon: f64,
    /// The macro-step `dt_macro`.
    pub dt_macro: f64,
    /// The collision radius the march ran at, canonical units; `0` is no collision terminal.
    pub r_coll: f64,
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} via Path {}: occupant {}, links {}, T = {}, dt_macro = {:e}, r_coll = {:e}",
            self.fixture,
            self.path,
            self.occupant,
            self.selection,
            self.horizon,
            self.dt_macro,
            self.r_coll
        )
    }
}

/// A march's values: the sim key's at horizon `t` (dispatch's validation, `engine::precision::march_key`), with the
/// collision radius `r_coll` in place of the sim key's default, since `SimConfig`'s collision group carries none yet.
pub fn march_key(t: f64, r_coll: f64) -> Result<MarchKey64, ComputeError> {
    let key = sim_march_key(&SimConfig {
        horizon: Horizon { t },
        ..latent_sim_key([0.0; 8])
    })?;
    Ok(MarchKey64 {
        radius: CollisionRadius::from_r_coll(r_coll),
        ..key
    })
}

/// [`march_key`] with its macro-step shortened so the horizon lands on `t` exactly: `dt_macro = t/n`, `n` the sim key's
/// own count `⌈t/dt_macro⌉`, so a stepper's last macro-step ends at `n·(t/n)`, `t` to rounding. The plan is unclear
/// here (the sim key's grid lands on `t` only where `t` is a multiple of it); chose this because a closed form or a
/// period is compared at its own time, and `n·dt_macro` off it by up to a step would read the step as an error.
pub fn landing_key(t: f64, r_coll: f64) -> Result<MarchKey64, ComputeError> {
    let mut key = march_key(t, r_coll)?;
    key.loop_values.dt_macro = t / f64::from(key.loop_values.horizon_steps);
    Ok(key)
}

/// The physics sim key of the latent chart through `z` with no extent, `q₁ = q₂ = 0`: what a pixel at `z` decodes
/// under. Its horizon is 0; [`march_key`] sets the march's.
pub fn latent_sim_key(z: [f64; 8]) -> SimConfig {
    use engine::contract::sim_config::Window;
    use engine::first_slice::Quad;
    let quad = Quad {
        chart: kernel::chart::Latent::<f32> {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    let mut key = quad.sim_key();
    key.plane.z0 = z;
    key.plane.q1 = [0.0; 8];
    key.plane.q2 = [0.0; 8];
    key
}

/// Heggie's step-control values for the suite's regularised marches: the CPU field backend's
/// (`engine::field::heggie_params`), which is what a pixel runs, at f64 (`tiny` at f64's own scale, as kernel's
/// `heggie_seam` test takes it). The test `ground_truth_heggie_is_the_field_backend_s` holds the f32 arm to the field
/// backend's values.
#[derive(Clone, Copy, Debug)]
pub struct FieldHeggie;

impl HgConfig for FieldHeggie {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: delta_sync(),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(4e4, 4e4),
            n_min: 100_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// Heggie × RK4 under [`FieldHeggie`]: the default occupant as the field backend marches it.
pub type DefaultOccupant = kernel::regularisation::Composed<
    kernel::occupant::rk4::Rk4,
    kernel::regularisation::heggie::Heggie<FieldHeggie>,
>;

/// One ground-truth fixture (`fixtures/ground_truth/fixtures.json`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// The fixture's name, its key in the note's table.
    pub name: String,
    /// The tier, 1 to 4 (the note's "Four tiers").
    pub tier: u8,
    /// The entry path, `"A"` or `"B"`.
    pub path: String,
    /// Where the start comes from: a paper or a closed form.
    pub source: String,
    /// How the start enters: the published values and their units, or the closed form's construction.
    pub construction: String,
    /// The quantities asserted on it, each with its criterion.
    pub asserted: Vec<String>,
    /// The quantities measured and reported, never asserted.
    pub reported: Vec<String>,
}

/// The fixture set's file.
pub fn fixtures_path() -> PathBuf {
    crate::import::root().join("fixtures/ground_truth/fixtures.json")
}

/// The fixture set.
pub fn fixtures() -> Result<Vec<Fixture>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        #[allow(dead_code)]
        about: String,
        fixtures: Vec<Fixture>,
    }
    let path = fixtures_path();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: File = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file.fixtures)
}
