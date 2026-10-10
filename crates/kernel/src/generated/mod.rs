//! The link registry's generated Rust (dd_generation_root §1, §2, §3.9): the chart constants at each `Real`
//! ([`constants`]; REQ-DEC-009) and the link functions, each entry a type implementing [`links::Link`], with the
//! per-block selection type ([`links::Selection`]; REQ-GEN-013, REQ-GEN-015). `cargo xtask codegen` writes both from
//! `crates/ledger/src/links.rs`; neither is edited by hand.

pub mod constants;
pub mod links;
