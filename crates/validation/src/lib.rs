//! The validation harness: may depend on any crate but `gui` and `prin`; every crate but `gui`
//! reaches it only as a dev-dependency, and `gui` never does (systems_architecture §7.1; R-176,
//! R-187).

pub mod bench;
pub mod bringup;
pub mod control;
pub mod convergence;
pub mod gate;
pub mod gpu;
pub mod oklab;
pub mod prop;
pub mod spawn;

/// The synthetic payload harness (TASK-M1-06), re-exported for the render crate's tests, which reach the engine only
/// through this dev-dependency (systems_architecture §7.1).
pub use engine::synthetic;
