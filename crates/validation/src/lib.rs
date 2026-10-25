//! The validation harness: may depend on any crate but `gui` and `prin`; every crate but `gui`
//! reaches it only as a dev-dependency, and `gui` never does (systems_architecture §7.1; R-176,
//! R-187).

pub mod bench;
pub mod bringup;
pub mod control;
pub mod convergence;
pub mod decode_f32_eps;
pub mod decode_sanity;
pub mod eps_phys;
pub mod gate;
pub mod gauge;
pub mod golden_scene;
pub mod gpu;
pub mod inspector;
pub mod invariant_gradient;
pub mod nsub_bucket;
pub mod oklab;
pub mod prop;
pub mod reference;
pub mod roundtrip;
pub mod shape_roundtrip;
pub mod shape_scene;
pub mod spawn;
pub mod structural_scene;

/// The synthetic payload harness (TASK-M1-06), re-exported for the render crate's tests, which reach the engine only
/// through this dev-dependency (systems_architecture §7.1).
pub use engine::synthetic;

/// The kernel's `times` packing (payload §2, §6), for ledger's round-trip test, which reaches the kernel only through
/// this dev-dependency.
pub use kernel::payload::{pack_times, set_t_dmin_step, tm_t_dmin_step, tm_t_end_step};

/// The coordinate presets and the stain graph they are (TASK-M1-07), re-exported for the render crate's tests, as the
/// harness is.
pub use engine::{presets, stain};

/// `r_pole`'s register entry (REQ-INT-086, R-433), for the kernel crate's tests, which reach the ledger only through
/// this dev-dependency: the kernel takes the ledger as a build-dependency alone (systems_architecture §7.1, R-185).
pub use ledger::constants::R_POLE;

/// The invariant charts' warp defaults, `K_max` and `γ_K` (REQ-CHART-044, proposed; TASK-M2-12), for the kernel crate's
/// tests, as `r_pole` is.
pub use ledger::constants::{GAMMA_K, K_MAX};
/// The shape-sphere chart's two tolerances' register entries (REQ-CHART-049, REQ-CHART-053; R-427), for the kernel
/// crate's tests and the `shape-roundtrip` gate, as `r_pole`'s is.
pub use ledger::constants::{SHAPE_ROUNDTRIP_BOUND, SHAPE_VEC_JS_TOL};
/// dd_integrator §5 test 3's rounding-level criterion's register entries (REQ-VAL-191, proposed; TASK-M3-02), the
/// criterion, its domain's lower edges at f64 and f32 and its upper edge, for the kernel crate's tests, as `r_pole`'s
/// is.
pub use ledger::constants::{
    STEP_REVERSAL_EDGE_F32, STEP_REVERSAL_EDGE_F64, STEP_REVERSAL_EDGE_MAX, STEP_REVERSAL_TOL,
};

/// The Burrau `ν = 1/2` tolerance (REQ-VAL-118, proposed) and the Euclid plane's defaults (chart_reference §4.5;
/// TASK-M2-10), for the kernel crate's tests, as `r_pole` is.
pub use ledger::constants::{
    BURRAU_NU_HALF_TOL, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN,
};

/// The ternary mass plot's default buffer `ε_m` (chart_reference §4.5; TASK-M2-13), for the kernel and engine crates'
/// tests, as the Euclid plane's defaults are.
pub use ledger::constants::MASS_SIMPLEX_EPS_M;

/// The acute-angle Burrau charts' defaults (REQ-CHART-045, REQ-CHART-046, proposed; chart_reference §4.5; TASK-M2-11),
/// for the kernel crate's tests and the `burrau_acute` evidence, as `r_pole` is.
pub use ledger::constants::{
    BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX, BURRAU_THETA_MIN,
};

/// The payload's pair-id map (R-22; payload §2), for the kernel crate's tests, as `r_pole` is.
pub use ledger::payload::pair_bodies;

/// The assembler's declaration parser, re-exported for the ledger crate's tests, which reach render only through this
/// dev-dependency: the numeric view template's `RANGE_AUTO` header round-trips through it (TASK-M1-09).
pub use render::assemble::{Declaration, UniformType};

/// The samples' chart classification (REQ-COL-102; TASK-M2-32), and the chart trait and the latent chart it classifies
/// from, with the decoder entry `D ∘ Φ` under `η_E` (`energy_norm`) and the link selections that reach the decoder's
/// `DEGENERATE` tags, re-exported for the render crate's tests, which reach the engine and the kernel only through this
/// dev-dependency.
pub use engine::chart_class;
pub use kernel::chart;
pub use kernel::decode::{energy_norm, Degenerate};
pub use kernel::generated::links::Defaults;
pub use kernel::stage::decode::StickBreakingLinks;
