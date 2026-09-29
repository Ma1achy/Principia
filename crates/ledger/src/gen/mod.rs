//! The generator driver (dd_generation_root §1; debug_tooling_plan step 0a): validate every entry against §3.8, run
//! the static layout check (§5 test 1, [`crate::check`]), then run each emitter. It refuses to emit anything when an
//! entry is incomplete, naming each field and the missing key, or when the layout check finds anything. The emitters
//! are registered in [`EMITTERS`].

use std::fmt;
use std::path::{Path, PathBuf};

use crate::check::{self, LayoutError};
use crate::schema::{Entry, FieldType, IncompleteEntry, Ledger, Location, Word};

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
    /// Derived fields whose `from` is empty or names something other than a stored entry, and vectors whose
    /// component is not a scalar type or whose `k` is below 2 (§3.8 `location`, `type`).
    Malformed(Vec<String>),
    /// Findings of the static layout check (REQ-GEN-003, REQ-GEN-028).
    Layout(Vec<LayoutError>),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = match self {
            GenError::Incomplete(entries) => entries.iter().map(ToString::to_string).collect(),
            GenError::Malformed(lines) => lines.clone(),
            GenError::Layout(errors) => errors.iter().map(ToString::to_string).collect(),
        };
        write!(
            f,
            "generation refused: {} ledger problem(s):\n  {}",
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
    let mut bad = bad_derived(&entries);
    bad.extend(entries.iter().filter_map(bad_vector));
    if bad.is_empty() {
        Ok(entries)
    } else {
        Err(GenError::Malformed(bad))
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

/// A line naming `e` if it is a vector whose component is itself a vector or whose `k` is below 2 (§3.8: "`k ≥ 2`
/// components, each of the scalar `type`").
fn bad_vector(e: &Entry) -> Option<String> {
    match &e.ty {
        FieldType::Vector { component, k }
            if matches!(**component, FieldType::Vector { .. }) || *k < 2 =>
        {
            Some(format!(
                "vector field `{}` needs k ≥ 2 scalar components (dd_generation_root §3.8)",
                e.name
            ))
        }
        _ => None,
    }
}

/// Validates `ledger` and runs the static layout check over it, then runs `emitters` over it; the files they
/// generate, or why not.
pub fn generate(ledger: &Ledger, emitters: &[Emitter]) -> Result<Vec<Generated>, GenError> {
    let entries = validate(ledger)?;
    let findings = check::check(&ledger.words, &entries);
    if !findings.is_empty() {
        return Err(GenError::Layout(findings));
    }
    Ok(emitters
        .iter()
        .flat_map(|emit| emit(&ledger.words, &entries))
        .collect())
}

/// Generates from `ledger` with `emitters` (`cargo xtask codegen` passes [`EMITTERS`]) and writes each file under
/// `root`; the paths written.
pub fn run(ledger: &Ledger, emitters: &[Emitter], root: &Path) -> Result<Vec<PathBuf>, String> {
    let files = generate(ledger, emitters).map_err(|e| e.to_string())?;
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
