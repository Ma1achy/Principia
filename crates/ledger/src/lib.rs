//! The roots: the layout table and the link registry, and the generators that read them
//! (systems_architecture §7.1; dd_generation_root §1). A root: no workspace dependency.
//!
//! The generation root (debug_tooling_plan step 0a): the §3.8 entry schema ([`schema`]), the static layout check
//! ([`check`]) and the generator driver ([`gen`]), which `cargo xtask codegen` runs (R-241); the constants register
//! and its generation gate ([`constants`]); the display names, what a person reads for a body, a pair, a `state` and
//! `detail` and an outcome class, 1-based ([`display`]; R-449); the link registry ([`links`]); the payload schema
//! version, the ledger's content hash ([`version`]); `RenderQuad`'s ledger table, the CPU-written quad record
//! ([`quad`]; dd_generation_root §3.7a); and `QuadReduction`'s ledger, the per-quad summary, with its struct and
//! emitter ([`reduction`]; dd_generation_root §3.7).

pub mod check;
pub mod constants;
pub mod display;
pub mod gen;
pub mod links;
pub mod payload;
pub mod quad;
pub mod reduction;
pub mod schema;
pub mod version;

/// The layout table `cargo xtask codegen` generates from: so far the payload's (dd_simstate_payload §0–§3;
/// dd_generation_root §3.1, §3.3–§3.6).
pub fn layout() -> schema::Ledger {
    payload::ledger()
}
