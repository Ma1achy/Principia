//! `cargo xtask codegen` — regenerates the checked-in generated files from the layout table (dd_generation_root §1).
//! It runs `ledger`'s generator, which validates every entry (§3.8) and only then emits (R-241), and writes a
//! generated file only when its content differs from the file on disk, so an unchanged file keeps its modification
//! time and forces no rebuild (R-284).

use std::path::{Path, PathBuf};

use ledger::constants::ConstantBuilder;
use ledger::gen::Emitter;
use ledger::schema::Ledger;

/// What a codegen run did with each generated file (paths relative to the root), by whether it was written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Files absent from disk, or whose content differed from the file on disk: written.
    pub written: Vec<PathBuf>,
    /// Files whose content matched the file on disk: left untouched (R-284).
    pub unchanged: Vec<PathBuf>,
}

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

/// Passes `register` through the constants gate ([`ledger::constants::gate`]), generates from `ledger` with
/// `emitters` ([`ledger::gen::generate`]), and writes each file under `root` only when its content differs from the
/// file on disk (R-284). Nothing is written when the gate or the generator refuses.
pub fn generate(
    register: &[ConstantBuilder],
    ledger: &Ledger,
    emitters: &[Emitter],
    root: &Path,
) -> Result<Outcome, String> {
    ledger::constants::gate(register).map_err(|e| format!("codegen: {e}"))?;
    let files = ledger::gen::generate(ledger, emitters).map_err(|e| format!("codegen: {e}"))?;
    let mut outcome = Outcome::default();
    for file in files {
        let path = root.join(&file.path);
        // A file that cannot be read (absent, or not UTF-8) counts as differing, and is written.
        if std::fs::read_to_string(&path).is_ok_and(|on_disk| on_disk == file.contents) {
            outcome.unchanged.push(file.path);
            continue;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, &file.contents).map_err(|e| format!("{}: {e}", path.display()))?;
        outcome.written.push(file.path);
    }
    Ok(outcome)
}
