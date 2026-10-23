//! The link registry's generated Rust (dd_generation_root §1, §2, §3.9): the chart constants at each `Real`
//! ([`constants`]; REQ-DEC-009) and the link functions, each entry a type implementing [`links::Link`], with the
//! per-block selection type ([`links::Selection`]; REQ-GEN-013, REQ-GEN-015). `cargo xtask codegen` writes both from
//! `crates/ledger/src/links.rs`; neither is edited by hand. It writes the steppers' coefficients ([`steppers`];
//! dd_integrator §3.2, REQ-INT-037) from the constants register too.

pub mod constants;
pub mod links;
pub mod steppers;
