//! The workspace's runners, invoked as `cargo xtask <command>` (systems_architecture §7.1: `xtask` reads
//! `cargo metadata`; no crate depends on it).

pub mod ci;
pub mod codegen;
pub mod controls;
pub mod deps;
pub mod gate;
pub mod golden;
pub mod lint_constants;
pub mod lint_vocab;
pub mod lint_wgsl;
pub mod mutants_check;
pub mod plan_check;
pub mod pr_check;
pub mod reviews_check;
pub mod screenshot;

use std::path::{Path, PathBuf};

/// This workspace's `Cargo.toml`.
pub fn workspace_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml")
}
