//! The workspace's runners, invoked as `cargo xtask <command>` (systems_architecture §7.1: `xtask` reads
//! `cargo metadata`; no crate depends on it).

pub mod ci;
pub mod codegen;
pub mod controls;
pub mod deps;
pub mod lint_constants;
pub mod plan_check;
pub mod reviews_check;

use std::path::{Path, PathBuf};

/// This workspace's `Cargo.toml`.
pub fn workspace_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml")
}
