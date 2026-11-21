//! `cargo xtask lint parity-tiers` — every test of the integrator drill-down suite carries its parity tier, and the tier
//! is the one parity_contract §8 indexes for its kind (TASK-M3-21). The lint is the kernel test
//! `crates/kernel/tests/parity_tiers.rs`, which reads the suite's sources and the contract's §8, so it also runs with
//! the kernel's tests in CI; this runs it alone and passes its exit status on.

use std::path::Path;
use std::process::Command;

/// Runs the kernel's `parity_tiers` test in the workspace whose `Cargo.toml` is `manifest`.
pub fn run(manifest: &Path) -> Result<(), String> {
    let status = Command::new(crate::deps::cargo())
        .args([
            "test",
            "-p",
            "kernel",
            "--test",
            "parity_tiers",
            "--manifest-path",
        ])
        .arg(manifest)
        .args(["--", "--nocapture"])
        .status()
        .map_err(|e| format!("lint parity-tiers: cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "lint parity-tiers: the kernel's parity_tiers test failed ({status})"
        ))
    }
}
