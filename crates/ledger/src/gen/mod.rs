//! The generator driver (dd_generation_root §1; debug_tooling_plan step 0a): validate every entry against §3.8,
//! then run each emitter. It refuses to emit anything when an entry is incomplete, naming each field and the missing
//! key. The emitters are registered in [`EMITTERS`]; the static layout check (§5 test 1) is TASK-M0-35's (R-240).

use std::fmt;
use std::path::{Path, PathBuf};

use crate::schema::{Entry, IncompleteEntry, Ledger, Location, Word};

/// One generated file: its path, relative to the workspace root, and its contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub path: PathBuf,
    pub contents: String,
}

/// An emitter: the files it generates from a validated layout.
pub type Emitter = fn(&[Word], &[Entry]) -> Vec<Generated>;

/// The registered emitters, run in order. None yet: this driver is built before them (TASK-M0-07 Deliverables).
pub const EMITTERS: &[Emitter] = &[];

/// Why generation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenError {
    /// Entries missing a required §3.8 key (REQ-GEN-002).
    Incomplete(Vec<IncompleteEntry>),
    /// Derived fields whose `from` is empty or names something other than a stored entry (§3.8 `location`).
    BadDerived(Vec<String>),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = match self {
            GenError::Incomplete(entries) => entries.iter().map(ToString::to_string).collect(),
            GenError::BadDerived(lines) => lines.clone(),
        };
        write!(
            f,
            "generation refused: {} ledger entry problem(s):\n  {}",
            lines.len(),
            lines.join("\n  ")
        )
    }
}

/// Every entry of `ledger` built, or every incomplete one; an unnamed entry is reported as "entry #i (unnamed)"
/// (R-242).
pub fn validate(ledger: &Ledger) -> Result<Vec<Entry>, GenError> {
    let mut entries = Vec::new();
    let mut incomplete = Vec::new();
    for (i, builder) in ledger.entries.iter().enumerate() {
        match builder.build() {
            Ok(entry) => entries.push(entry),
            Err(mut error) => {
                if builder.name.is_none() {
                    error.field = format!("entry #{i} (unnamed)");
                }
                incomplete.push(error);
            }
        }
    }
    if !incomplete.is_empty() {
        return Err(GenError::Incomplete(incomplete));
    }
    let bad = bad_derived(&entries);
    if bad.is_empty() {
        Ok(entries)
    } else {
        Err(GenError::BadDerived(bad))
    }
}

/// Each derived entry whose `from` is empty, or names anything but a packed or scalar entry, as a line naming it
/// (§3.8: "each must be an entry of the ledger whose location is a packed word or a scalar index").
fn bad_derived(entries: &[Entry]) -> Vec<String> {
    let stored = |name: &str| {
        entries
            .iter()
            .any(|e| e.name == name && !matches!(e.location, Location::Derived { .. }))
    };
    let mut bad = Vec::new();
    for e in entries {
        let Location::Derived { from } = &e.location else {
            continue;
        };
        if from.is_empty() {
            bad.push(format!(
                "derived field `{}` has an empty `from` (dd_generation_root §3.8)",
                e.name
            ));
        }
        for source in from.iter().filter(|s| !stored(s)) {
            bad.push(format!(
                "derived field `{}`: `from` names `{source}`, not a stored ledger entry (dd_generation_root §3.8)",
                e.name
            ));
        }
    }
    bad
}

/// Validates `ledger`, then runs `emitters` over it; the files they generate, or why not.
pub fn generate(ledger: &Ledger, emitters: &[Emitter]) -> Result<Vec<Generated>, GenError> {
    let entries = validate(ledger)?;
    Ok(emitters
        .iter()
        .flat_map(|emit| emit(&ledger.words, &entries))
        .collect())
}

/// Generates from `ledger` with [`EMITTERS`] and writes each file under `root`; the paths written.
pub fn run(ledger: &Ledger, root: &Path) -> Result<Vec<PathBuf>, String> {
    let files = generate(ledger, EMITTERS).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for file in files {
        let path = root.join(&file.path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, &file.contents).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(file.path);
    }
    Ok(written)
}
