//! The fragment side's occupant registry: the scanned filesystem (gui_state_contract §3). The registry is not a
//! hand-maintained list: [`scan`] walks the hand-written occupants under `shaders/wgsl/frag/` ([`OCCUPANT_DIR`]) and the
//! debug catalogue's generated views under `frag/debug/generated/` ([`GENERATED_DIR`], RQ-219), one entry per `.wgsl`
//! file, `{id, slot, source, category, uniformSchema, inputDomains}`. Adding a file is the act of registering it.
//!
//! **Slot.** A file under a slot directory is that slot's occupant: `colour/`, `brightness/`, `combiner/` or `post/`.
//! A file under either `debug/` is tagged [`Category::Debug`] (a filter tag, not a different mechanism), and its slot is
//! the slot function it defines, exactly one of `colour`, `brightness`, `combine` and `post` (RQ-219); a generated
//! field view defines `colour`. **Declarations.** `uniformSchema` and `inputDomains` are the file's `// @uniform` and
//! `// @input` lines, read as the assembler reads them ([`assemble::declaration`]), a colour or brightness that
//! declares no input taking the default one.
//!
//! **The catalogue's views at runtime.** [`Catalogue`] gives the debug-view baker ([`crate::debug_bake`]) each field's
//! generated view as a stain: a source of the zero field into the view's colour, which reads its field from `ctx`
//! (render contract Part 6: `present(unpack(ctx))`), then the pass-through combiner and `OUT`. The colour's one input,
//! the default `field`, must be wired for the node to be live (render_gui_spec §13), and the view does not read it.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::assemble::{self, Input, Kind, Node, Occupant, Uniform};
use crate::debug_bake::ViewGenerator;

/// The hand-written occupants, relative to the render crate: one directory per slot (gui_state_contract §3).
pub const OCCUPANT_DIR: &str = "shaders/wgsl/frag";

/// The debug catalogue's generated views, relative to the render crate (RQ-219; `ledger::gen::catalogue::DIR`).
pub const GENERATED_DIR: &str = "frag/debug/generated";

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
    pub path: PathBuf,
    pub category: Category,
    pub uniform_schema: Vec<Uniform>,
    pub input_domains: Vec<Input>,
}

/// Why the scan failed, naming the file.
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
        }
    }
}

impl std::error::Error for RegistryError {}

/// The registry of the render crate at `render`: every `.wgsl` file under its [`OCCUPANT_DIR`] and its
/// [`GENERATED_DIR`], each an entry, sorted by id. A missing directory holds no file.
pub fn scan(render: &Path) -> Result<Vec<Entry>, RegistryError> {
    let mut out = Vec::new();
    for (root, prefix) in [(OCCUPANT_DIR, ""), (GENERATED_DIR, "debug/generated/")] {
        let root = render.join(root);
        let mut files = Vec::new();
        wgsl_files(&root, &mut files)?;
        for path in files {
            out.push(entry(&root, prefix, path)?);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// The render crate's own registry, scanned from its source tree.
pub fn registry() -> Result<Vec<Entry>, RegistryError> {
    scan(Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// Each `.wgsl` file under `dir`, recursively; none if `dir` does not exist.
fn wgsl_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), RegistryError> {
    if !dir.is_dir() {
        return Ok(());
    }
    let io = |e: std::io::Error| RegistryError::Io(dir.to_owned(), e.to_string());
    for item in std::fs::read_dir(dir).map_err(io)? {
        let path = item.map_err(io)?.path();
        if path.is_dir() {
            wgsl_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "wgsl") {
            out.push(path);
        }
    }
    Ok(())
}

/// The entry of `path`, found under `root`, its id led by `prefix`.
fn entry(root: &Path, prefix: &str, path: PathBuf) -> Result<Entry, RegistryError> {
    let relative = path.strip_prefix(root).unwrap_or(&path).with_extension("");
    let parts: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let id = format!("{prefix}{}", parts.join("/"));
    let source = std::fs::read_to_string(&path)
        .map_err(|e| RegistryError::Io(path.clone(), e.to_string()))?;
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

/// The source of the zero field: the input the catalogue's views take and do not read.
pub const ZERO_SOURCE: &str =
    "// The zero field, wired to a debug view's input, which it does not read.\n\
     fn source(ctx: Ctx) -> Field { return Field(0.0, 0.0, 0.0, 0.0); }\n";

/// The debug catalogue's generated views, by field: the registry's entries under [`GENERATED_DIR`].
#[derive(Clone, Debug, PartialEq)]
pub struct Catalogue {
    views: Vec<Entry>,
}

impl Catalogue {
    /// The views among `registry`'s entries.
    pub fn new(registry: &[Entry]) -> Catalogue {
        let views = registry
            .iter()
            .filter(|e| e.id.starts_with("debug/generated/"))
            .cloned()
            .collect();
        Catalogue { views }
    }

    /// The fields with a view, in id order.
    pub fn fields(&self) -> Vec<&str> {
        self.views
            .iter()
            .filter_map(|e| e.id.strip_prefix("debug/generated/"))
            .collect()
    }
}

impl ViewGenerator for Catalogue {
    /// `field`'s view as a stain: the zero source into the view's colour, the pass-through combiner, `OUT`.
    fn view(&self, field: &str) -> Option<Vec<Node>> {
        let view = self
            .views
            .iter()
            .find(|e| e.id.strip_prefix("debug/generated/") == Some(field))?;
        let node = |kind, occupant, inputs: &[Option<usize>]| Node {
            kind,
            occupant,
            inputs: inputs.to_vec(),
        };
        Some(vec![
            node(Kind::Source, Occupant::Custom(ZERO_SOURCE.to_owned()), &[]),
            node(
                Kind::Colour,
                Occupant::Custom(view.source.clone()),
                &[Some(0)],
            ),
            node(
                Kind::Combiner,
                Occupant::BuiltIn("pass_through".to_owned()),
                &[Some(1), None],
            ),
            node(Kind::Out, Occupant::None, &[Some(2)]),
        ])
    }
}
