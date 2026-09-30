//! The validation harness: may depend on any crate but `gui` and `prin`; every crate but `gui`
//! reaches it only as a dev-dependency, and `gui` never does (systems_architecture §7.1; R-176,
//! R-187).

pub mod control;
pub mod convergence;
pub mod gate;
pub mod gpu;
pub mod prop;
pub mod spawn;
