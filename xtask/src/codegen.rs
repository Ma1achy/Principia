//! `cargo xtask codegen` — regenerates the checked-in generated files from the layout table (dd_generation_root §1).
//! It runs `ledger`'s generator driver, which validates every entry (§3.8) and only then emits (R-241), and writes a
//! generated file only when its content differs from the file on disk, so an unchanged file keeps its modification
//! time and forces no rebuild (R-284).

use std::path::Path;

use ledger::constants::ConstantBuilder;
use ledger::gen::Emitter;
use ledger::schema::Ledger;

pub use ledger::gen::Outcome;

/// Runs the generator over the workspace whose `Cargo.toml` is `manifest`, writing under its root.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let outcome = generate(
        ledger::constants::REGISTER,
        &ledger::layout(),
        ledger::gen::EMITTERS,
        root,
    )?;
    println!(
        "codegen: {} generated file(s) written, {} unchanged",
        outcome.written.len(),
        outcome.unchanged.len()
    );
    for path in &outcome.written {
        println!("codegen:   written   {}", path.display());
    }
    for path in &outcome.unchanged {
        println!("codegen:   unchanged {}", path.display());
    }
    Ok(())
}

/// Runs `ledger`'s generator driver ([`ledger::gen::run_with_register`]) with `register`, `emitters` and `root`: the
/// constants gate, then generation from `ledger`, then each file written under `root` only when its content differs
/// from the file on disk (R-284). Nothing is written when the gate or the generator refuses.
pub fn generate(
    register: &[ConstantBuilder],
    ledger: &Ledger,
    emitters: &[Emitter],
    root: &Path,
) -> Result<Outcome, String> {
    ledger::gen::run_with_register(register, ledger, emitters, root)
        .map_err(|e| format!("codegen: {e}"))
}
