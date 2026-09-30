//! The roots: the layout table and the link registry, and the generators that read them
//! (systems_architecture §7.1; dd_generation_root §1). A root: no workspace dependency.
//!
//! The generation root (debug_tooling_plan step 0a): the §3.8 entry schema ([`schema`]), the static layout check
//! ([`check`]) and the generator driver ([`gen`]), which `cargo xtask codegen` runs (R-241); the constants register
//! and its generation gate ([`constants`]).

pub mod check;
pub mod constants;
pub mod gen;
pub mod payload;
pub mod schema;

/// The layout table `cargo xtask codegen` generates from: so far the payload's (dd_simstate_payload §0–§1;
/// dd_generation_root §3.1, §3.3a–§3.6).
pub fn layout() -> schema::Ledger {
    payload::ledger()
}
