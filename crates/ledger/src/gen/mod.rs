//! The generator driver (dd_generation_root §1; debug_tooling_plan step 0a): validate every entry against §3.8, run
//! the static layout check (§5 test 1, [`crate::check`]), then run each emitter. It refuses to emit anything when an
//! entry is incomplete, naming each field and the missing key, when the layout check finds anything, or when a
//! payload struct member the Rust emitter would write is off the ledger ([`rust::check`]). The emitters are
//! registered in [`EMITTERS`].

pub mod rust;

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

/// The registered emitters, run in order.
pub const EMITTERS: &[Emitter] = &[rust::emit];

/// Why generation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenError {
    /// Entries missing a required §3.8 key (REQ-GEN-002).
    Incomplete(Vec<IncompleteEntry>),
    /// Names given to more than one entry, derived fields whose `from` is empty or names something other than exactly
    /// one stored entry, vectors whose component is not a scalar type or whose `k` is below 2, and a `floor` that is
    /// empty or names a ledger entry or a register constant (§3.8 `location`, `type`, `floor`).
    Malformed(Vec<String>),
    /// Findings of the static layout check (REQ-GEN-003, REQ-GEN-028).
    Layout(Vec<LayoutError>),
    /// Struct members off the ledger: stored as other than their entry's type or word's width, off their entry's
    /// scalar slot, or with no entry or word, each naming the member ([`rust::check`]; dd_generation_root §3.8: "A
    /// field without a complete entry fails generation loudly").
    Structs(Vec<String>),
    /// Constants-register entries missing a value, class or citation, or thresholds without an admissible relative
    /// basis, each naming the constant ([`crate::constants::gate`]; REQ-SYS-001, REQ-SYS-005, REQ-VAL-006).
    Constants(Vec<String>),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = match self {
            GenError::Incomplete(entries) => entries.iter().map(ToString::to_string).collect(),
            GenError::Malformed(lines) | GenError::Structs(lines) | GenError::Constants(lines) => {
                lines.clone()
            }
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
    let mut bad: Vec<String> = entries
        .iter()
        .enumerate()
        .filter(|(i, e)| entries[..*i].iter().any(|f| f.name == e.name))
        .map(|(_, e)| {
            format!(
                "field `{}` has more than one entry: names are unique across the ledger (dd_generation_root §3.8)",
                e.name
            )
        })
        .collect();
    bad.extend(bad_derived(&entries));
    bad.extend(entries.iter().filter_map(bad_vector));
    bad.extend(bad_floor(&ledger.words, &entries));
    if bad.is_empty() {
        Ok(entries)
    } else {
        Err(GenError::Malformed(bad))
    }
}

/// Each derived entry whose `from` is empty, or names anything but exactly one entry, a packed or scalar one, as a
/// line naming it (§3.8: "each must be an entry of the ledger whose location is a packed word or a scalar index").
fn bad_derived(entries: &[Entry]) -> Vec<String> {
    let stored = |name: &str| {
        let mut named = entries.iter().filter(|e| e.name == name);
        let one = named.next().filter(|_| named.next().is_none());
        one.is_some_and(|e| !matches!(e.location, Location::Derived { .. }))
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
                "derived field `{}`: `from` names `{source}`, not exactly one stored ledger entry (dd_generation_root §3.8)",
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

/// Each entry whose `floor` is not a sim-key parameter name, as a line naming it: §3.8's `floor` names a sim-key
/// parameter, which is "neither a ledger entry nor a register constant" (R-263). So an empty floor, one that is not a
/// name ([`is_name`]: a number such as `1e-6` is a value), a ledger entry's or packed word's name (a packed word is a
/// stored field of the ledger, payload §1) or a register constant's name fails.
fn bad_floor(words: &[Word], entries: &[Entry]) -> Vec<String> {
    let register = crate::constants::REGISTER;
    entries
        .iter()
        .filter_map(|e| e.floor.map(|floor| (e.name, floor)))
        .filter_map(|(name, floor)| {
            let what = if floor.is_empty() {
                "is empty"
            } else if !is_name(floor) {
                "is not a name"
            } else if entries.iter().any(|f| f.name == floor) {
                "names a ledger entry"
            } else if words.iter().any(|w| w.name == floor) {
                "names a ledger word"
            } else if register.iter().any(|c| c.name == floor) {
                "names a register constant"
            } else {
                return None;
            };
            Some(format!(
                "field `{name}`: `floor` `{floor}` {what}, not a sim-key parameter (dd_generation_root §3.8, R-263)"
            ))
        })
        .collect()
}

/// Whether `s` is a name: an ASCII letter or `_`, then ASCII letters, digits or `_` (the form of every sim-key
/// parameter the corpus names, such as `eps_E`). The corpus does not define "name"; this is the tightest reading
/// that admits §3.4's `eps_E` and `eps_L` (applied per R-204).
fn is_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validates `ledger` and runs the static layout check over it; if `emitters` include the Rust struct emitter
/// ([`rust::emit`]) and `ledger` declares any member of the payload structs, checks every member against it
/// ([`rust::check`], exempting [`crate::payload::PENDING`]); then runs `emitters` over it. The files they generate, or
/// why not.
pub fn generate(ledger: &Ledger, emitters: &[Emitter]) -> Result<Vec<Generated>, GenError> {
    let entries = validate(ledger)?;
    let findings = check::check(&ledger.words, &entries);
    if !findings.is_empty() {
        return Err(GenError::Layout(findings));
    }
    let structs = crate::payload::structs();
    let writes_structs = emitters
        .iter()
        .any(|&e| std::ptr::fn_addr_eq(e, rust::emit as Emitter));
    if writes_structs && rust::declares(&structs, &ledger.words, &entries) {
        let found = rust::check(&structs, &ledger.words, &entries, crate::payload::PENDING);
        if !found.is_empty() {
            return Err(GenError::Structs(found));
        }
    }
    Ok(emitters
        .iter()
        .flat_map(|emit| emit(&ledger.words, &entries))
        .collect())
}

/// What a generator run did with each generated file (paths relative to the root), by whether it was written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Files absent from disk, or whose content differed from the file on disk: written.
    pub written: Vec<PathBuf>,
    /// Files whose content matched the file on disk: left untouched, so their modification time does not move
    /// (R-284).
    pub unchanged: Vec<PathBuf>,
}

/// Passes the constants register ([`crate::constants::REGISTER`]) through its gate, then generates from `ledger` with
/// `emitters` (`cargo xtask codegen` passes [`EMITTERS`]) and writes each file under `root` only when its content
/// differs from the file on disk (R-284); which files were written and which were unchanged.
pub fn run(ledger: &Ledger, emitters: &[Emitter], root: &Path) -> Result<Outcome, String> {
    run_with_register(crate::constants::REGISTER, ledger, emitters, root)
}

/// [`run`] with `register` as the constants register: generation is refused, and nothing written, unless every entry
/// of `register` passes [`crate::constants::gate`].
pub fn run_with_register(
    register: &[crate::constants::ConstantBuilder],
    ledger: &Ledger,
    emitters: &[Emitter],
    root: &Path,
) -> Result<Outcome, String> {
    crate::constants::gate(register).map_err(|e| e.to_string())?;
    let files = generate(ledger, emitters).map_err(|e| e.to_string())?;
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
