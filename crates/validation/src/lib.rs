//! The validation harness: may depend on any crate but `gui` and `prin`; every crate but `gui`
//! reaches it only as a dev-dependency, and `gui` never does (systems_architecture §7.1; R-176,
//! R-187).

pub mod bench;
pub mod bringup;
pub mod control;
pub mod convergence;
pub mod decode_f32_eps;
pub mod gate;
pub mod golden_scene;
pub mod gpu;
pub mod oklab;
pub mod prop;
pub mod shape_roundtrip;
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

/// The shape-sphere chart's two tolerances' register entries (REQ-CHART-049, REQ-CHART-053; R-427), for the kernel
/// crate's tests and the `shape-roundtrip` gate, as `r_pole`'s is.
pub use ledger::constants::{SHAPE_ROUNDTRIP_BOUND, SHAPE_VEC_JS_TOL};

/// The assembler's declaration parser, re-exported for the ledger crate's tests, which reach render only through this
/// dev-dependency: the numeric view template's `RANGE_AUTO` header round-trips through it (TASK-M1-09).
pub use render::assemble::{Declaration, UniformType};
