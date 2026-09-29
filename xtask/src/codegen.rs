//! `cargo xtask codegen` — regenerates the checked-in generated files from the layout table (dd_generation_root §1).
//! It runs `ledger`'s generator driver, which validates every entry (§3.8) and only then emits (R-241).

use std::path::Path;

/// Runs the generator over the workspace whose `Cargo.toml` is `manifest`, writing under its root.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let written = ledger::gen::run(&ledger::layout(), ledger::gen::EMITTERS, root)
        .map_err(|e| format!("codegen: {e}"))?;
    println!("codegen: {} generated file(s) written", written.len());
    for path in written {
        println!("codegen:   {}", path.display());
    }
    Ok(())
}
