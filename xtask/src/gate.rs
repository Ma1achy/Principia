//! `cargo xtask gate <gate>` and `cargo xtask gate --all` (TASK-M0-05): run a numerical gate from
//! `fixtures/gates/<gate>/`, writing its reports under `target/gates/` (philosophy §4.5a, §4.6). The runner lives in
//! `validation` (`validation::gate`), which xtask reaches only as a dev-dependency (systems_architecture §7.1; R-176,
//! R-187), so xtask runs validation's `gate` binary as a separate process, through the cargo that runs it. What follows a
//! gate's name goes to that binary as it stands (TASK-M4-27): a measurement gate's flags, `--write-inputs`, `--record`,
//! `--cpu-f32`, `--compare`, `--hardware-label` and TASK-M4-28's `--replay` and `--replay-state`.

use std::path::Path;
use std::process::Command;

use crate::deps::cargo;

/// Which gates to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which<'a> {
    /// The gate so named, and what followed its name.
    One(&'a str, &'a [&'a str]),
    /// Every registered gate.
    All,
    /// None: list the registered gates, their thresholds and inputs (the listing-only form, R-235).
    List,
}

/// The words `which` gives validation's `gate` binary after `--root <workspace>`.
pub fn words<'a>(which: Which<'a>) -> Vec<&'a str> {
    match which {
        Which::One(name, flags) => std::iter::once(name).chain(flags.iter().copied()).collect(),
        Which::All => vec!["--all"],
        Which::List => vec!["--list"],
    }
}

/// Runs validation's `gate` binary on the workspace of `manifest`, for `which`; `Err` when it fails.
pub fn run(manifest: &Path, which: Which<'_>) -> Result<(), String> {
    let root = manifest.parent().unwrap_or(Path::new("."));
    let words = words(which);
    let what = words.join(" ");
    let status = Command::new(cargo())
        .args(["run", "--quiet", "--manifest-path"])
        .arg(manifest)
        .args(["-p", "validation", "--bin", "gate", "--", "--root"])
        .arg(root)
        .args(&words)
        .status()
        .map_err(|e| format!("cannot run cargo run -p validation --bin gate: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("gate {what} failed ({status})"))
    }
}
