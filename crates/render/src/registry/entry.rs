//! One registry entry (gui_state_contract §3; REQ-GEN-021): its six fields `{id, slot, source, category,
//! uniformSchema, inputDomains}`, keyed by directory; the slot it is valid in ([`Entry::occupant`]); and its
//! `inputDomains` (R-53; REQ-GEN-019).
//!
//! **Slot.** A file under a slot directory is that slot's occupant: `colour/`, `brightness/`, `combiner/` or `post/`.
//! A file under a `debug/` directory is tagged [`Category::Debug`] (a filter tag, not a different mechanism), and its
//! slot is the slot function it defines, exactly one of `colour`, `brightness`, `combine` and `post` (RQ-219); a
//! generated field view defines `colour`. An occupant is valid only in its slot.
//!
//! **Declarations.** `uniformSchema` and `inputDomains` are the file's `// @uniform` and `// @input` lines, read as
//! the assembler reads them ([`assemble::declaration`]), a colour or brightness that declares no input taking the
//! default one.
//!
//! **`inputDomains` (R-53; R-815).** An input that declares its domain, `// @input <name> [<lo>, <hi>]`, maps from
//! it. An input that declares none is untransformed: its entry records "inherits" ([`Input::domain`] `None`), and its
//! domain is resolved per wire, at assembly, from the manifest range of the field wired to it ([`resolve`]): the
//! range's `[lo, hi]` where both ends are finite, and `range_norm`'s `(0, 1)` ([`UNIT_DOMAIN`]) for a field whose
//! range is `Range::UNBOUNDED`. For any other range, one with exactly one finite end, the corpus defines no domain
//! (RQ-786): [`resolve`] chooses none and returns [`Unruled`].

use std::fmt;
use std::path::PathBuf;

use ledger::gen::numeric::finite;
use ledger::schema::Range;

use crate::assemble::{self, Input, Kind, Occupant, Uniform};

/// The slot directories and the slot each holds (gui_state_contract §3).
const SLOTS: [(&str, Kind); 4] = [
    ("colour", Kind::Colour),
    ("brightness", Kind::Brightness),
    ("combiner", Kind::Combiner),
    ("post", Kind::Post),
];

/// The directory whose files are tagged debug, wherever it is.
const DEBUG: &str = "debug";

/// What an occupant is for, as the GUI filters it: the polished GUI hides [`Category::Debug`] (gui_state_contract §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Under a slot directory.
    Slot,
    /// Under a `debug/` directory: reads raw payload or quad fields.
    Debug,
}

/// One registry entry (gui_state_contract §3).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The file's path under its scan root, `/`-separated, without `.wgsl`: `combiner/multiply`,
    /// `debug/generated/state`.
    pub id: String,
    pub slot: Kind,
    /// The file's WGSL.
    pub source: String,
    /// Where the source is from: a scanned file's path, or `<id>.wgsl` for a custom registered at run time.
    pub path: PathBuf,
    pub category: Category,
    pub uniform_schema: Vec<Uniform>,
    /// One per field input, in declaration order; a `domain` of `None` records "inherits" (R-815).
    pub input_domains: Vec<Input>,
}

/// Why the registry refused, naming the file.
#[derive(Clone, Debug, PartialEq)]
pub enum RegistryError {
    /// A directory or file that could not be read.
    Io(PathBuf, String),
    /// A file under no slot directory and no `debug/`.
    NoSlot(PathBuf),
    /// A debug file defining other than exactly one slot function; the ones it defines.
    DebugSlot(PathBuf, Vec<Kind>),
    /// A file whose declarations or slot the assembler refuses.
    Occupant(PathBuf, assemble::AssembleError),
    /// An occupant asked for in a slot that is not its own: its id, its slot, and the slot asked.
    WrongSlot(String, Kind, Kind),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Io(path, why) => write!(f, "{}: {why}", path.display()),
            RegistryError::NoSlot(path) => write!(
                f,
                "{}: not under a slot directory (colour/, brightness/, combiner/, post/) or debug/",
                path.display()
            ),
            RegistryError::DebugSlot(path, kinds) => write!(
                f,
                "{}: a debug occupant defines exactly one of `colour`, `brightness`, `combine` and `post`; this one \
                 defines {}",
                path.display(),
                kinds.len()
            ),
            RegistryError::Occupant(path, e) => write!(f, "{}: {e}", path.display()),
            RegistryError::WrongSlot(id, slot, asked) => write!(
                f,
                "`{id}` is a {} occupant, valid only in its slot: the {} slot does not take it",
                slot.name(),
                asked.name()
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

impl Entry {
    /// The entry of the WGSL `source` registered as `id`, `path` where it is from: a scanned file and a custom
    /// authored at run time alike. The slot and the category are read off `id`'s directories.
    pub fn new(id: String, path: PathBuf, source: String) -> Result<Entry, RegistryError> {
        let dirs: Vec<&str> = id.split('/').collect();
        let dirs = &dirs[..dirs.len() - 1];
        let (slot, category) = if dirs.contains(&DEBUG) {
            let defined =
                slot_functions(&source).map_err(|e| RegistryError::Occupant(path.clone(), e))?;
            match defined[..] {
                [kind] => (kind, Category::Debug),
                _ => return Err(RegistryError::DebugSlot(path, defined)),
            }
        } else {
            let kind = dirs
                .first()
                .and_then(|d| SLOTS.iter().find(|(name, _)| name == d))
                .map(|&(_, kind)| kind)
                .ok_or_else(|| RegistryError::NoSlot(path.clone()))?;
            (kind, Category::Slot)
        };
        let declaration = assemble::declaration(slot, &Occupant::Custom(source.clone()))
            .map_err(|e| RegistryError::Occupant(path.clone(), e))?;
        Ok(Entry {
            id,
            slot,
            source,
            path,
            category,
            uniform_schema: declaration.uniforms,
            input_domains: declaration.inputs,
        })
    }

    /// The occupant this entry gives a node of the `slot` slot: its WGSL, in its own slot only. Any other slot refuses
    /// it, since an occupant is valid only in its slot, its signature (gui_state_contract §3).
    pub fn occupant(&self, slot: Kind) -> Result<Occupant, RegistryError> {
        if slot != self.slot {
            return Err(RegistryError::WrongSlot(self.id.clone(), self.slot, slot));
        }
        Ok(Occupant::Custom(self.source.clone()))
    }

    /// The domain input `input` maps from when the field wired to it has the manifest range `wired` ([`resolve`]);
    /// `None` if the entry has no such input, or if no domain is defined for that wire yet ([`Unruled`], RQ-786:
    /// [`resolve`] on the input's own `domain` tells the two apart).
    pub fn domain(&self, input: usize, wired: Range) -> Option<(f64, f64)> {
        self.input_domains
            .get(input)
            .and_then(|i| resolve(i.domain, wired).ok())
    }
}

/// `range_norm`'s unit interval: the domain an untransformed input inherits from a field whose range is
/// `Range::UNBOUNDED` (R-815). Interface data, as the generator's fallback is, not a register constant.
pub const UNIT_DOMAIN: (f64, f64) = (0.0, 1.0);

/// No domain is defined yet for an untransformed input wired to a field with this manifest range: it neither has two
/// finite ends nor is `Range::UNBOUNDED` (RQ-786, open: R-815 rules only those two). The caller handles it; nothing
/// stands in for the domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unruled(pub Range);

impl fmt::Display for Unruled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no domain is defined for an untransformed input wired to a field of range {}, which has neither two \
             finite ends nor none (RQ-786)",
            self.0
        )
    }
}

impl std::error::Error for Unruled {}

/// The domain of an input wired to a field whose manifest range is `wired` (R-53; R-815): the `declared` one, a node
/// that transforms its field declaring its own; else inherited, the range's `[lo, hi]` where both ends are finite and
/// [`UNIT_DOMAIN`] where the range is `Range::UNBOUNDED`. Any other range has no ruled domain ([`Unruled`]).
pub fn resolve(declared: Option<(f64, f64)>, wired: Range) -> Result<(f64, f64), Unruled> {
    if let Some(own) = declared {
        return Ok(own);
    }
    match (finite(wired.lo), finite(wired.hi)) {
        (Some(lo), Some(hi)) => Ok((lo, hi)),
        _ if wired == Range::UNBOUNDED => Ok(UNIT_DOMAIN),
        _ => Err(Unruled(wired)),
    }
}

/// The slot functions `source` defines, in order: each `fn colour`, `fn brightness`, `fn combine` and `fn post`
/// outside comments.
pub fn slot_functions(source: &str) -> Result<Vec<Kind>, assemble::AssembleError> {
    let defined = assemble::defined_functions(source)?;
    Ok(SLOTS
        .iter()
        .map(|&(_, kind)| kind)
        .filter(|kind| kind.slot().is_some_and(|f| defined.iter().any(|d| d == f)))
        .collect())
}
