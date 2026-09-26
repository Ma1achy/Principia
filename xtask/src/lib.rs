//! The workspace's runners, invoked as `cargo xtask <command>` (systems_architecture §7.1: `xtask` reads
//! `cargo metadata`; no crate depends on it).

pub mod ci;
pub mod deps;
