//! The fragment side's occupant registry: the scanned filesystem (gui_state_contract §3). The registry is not a
//! hand-maintained list: [`scan()`] walks the hand-written occupants under `shaders/wgsl/frag/` ([`OCCUPANT_DIR`]), the
//! debug catalogue's generated views under `frag/debug/generated/` ([`GENERATED_DIR`], RQ-219) and its generated
//! reductions under `frag/debug/reductions/` ([`REDUCTIONS_DIR`]: each vector field's direction cosines and the ternary
//! masses, TASK-M1-12), one entry per `.wgsl` file, `{id, slot, source, category, uniformSchema, inputDomains}`. Adding
//! a file is the act of registering it.
//!
//! **Naming.** The registry is the only list of shipped sources: a node names any of them by its name ([`named`],
//! [`Entry::name`]), and the assembler looks the name up here. Two files a node would call by one name are refused,
//! naming both.
//!
//! **Build time and run time (TASK-M7-13; R-815, RQ-677 (e)).** The crate's own registry, [`registry`], is scanned
//! when the crate is built: `build.rs` runs the scan and embeds each file it finds ([`SHIPPED`]), so a binary moved
//! from its source tree, and the wasm build, which has no filesystem, hold the same entries. Adding a `.wgsl` file
//! reruns the build script, with no code edited. A custom authored at run time joins the same way ([`register`]): its
//! id names its directory, and its entry is made as a scanned file's is ([`Entry::new`]). [`scan()`] is the same scan
//! at run time, of any render crate on disk.
//!
//! **A vector field's two reductions** (gui_state_contract §4; dd_generation_root §3.8): '‖·‖ as scalar' is its
//! catalogue view, `debug/generated/<field>`, and 'as direction-cosines' is `debug/reductions/<field>_dircos`
//! ([`vector_reductions`]).
//!
//! **Slot, declarations and `inputDomains`** are the entry's ([`entry`]): a file under a slot directory is that
//! slot's occupant, a file under either `debug/` is tagged [`Category::Debug`], and `uniformSchema` and `inputDomains`
//! are the file's `// @uniform` and `// @input` lines.
//!
//! **The catalogue's views at runtime.** [`Catalogue`] gives the debug-view baker ([`crate::debug_bake`]) each field's
//! generated view as a stain: a source of the zero field into the view's colour, which reads its field from `ctx`
//! (render contract Part 6: `present(unpack(ctx))`), then the pass-through combiner and `OUT`. The colour's one input,
//! the default `field`, must be wired for the node to be live (render_gui_spec §13), and the view does not read it.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::assemble::{Kind, Node, Occupant};
use crate::debug_bake::ViewGenerator;

pub mod entry;
mod scan;

pub use entry::{resolve, slot_functions, Category, Entry, RegistryError, Unruled, UNIT_DOMAIN};
pub use scan::{
    occupant_files, shipped_table, Found, ScanError, GENERATED_DIR, OCCUPANT_DIR, REDUCTIONS_DIR,
    ROOTS,
};

// `SHIPPED`, the table `build.rs` wrote from its scan of this crate.
include!(concat!(env!("OUT_DIR"), "/frag_registry.rs"));

/// The names of a vector field's two reductions, as the GUI offers them (gui_state_contract §4).
pub const REDUCTION_NAMES: [&str; 2] = ["‖·‖ as scalar", "as direction-cosines"];

/// The registry ids of `field`'s two reductions, each with its name ([`REDUCTION_NAMES`]): '‖·‖ as scalar', the
/// field's catalogue view, then 'as direction-cosines', its reduction view.
pub fn vector_reductions(field: &str) -> [(&'static str, String); 2] {
    [
        (REDUCTION_NAMES[0], format!("debug/generated/{field}")),
        (
            REDUCTION_NAMES[1],
            format!("debug/reductions/{field}_dircos"),
        ),
    ]
}

/// The registry of the render crate at `render`, scanned now: every `.wgsl` file under its [`OCCUPANT_DIR`], its
/// [`GENERATED_DIR`] and its [`REDUCTIONS_DIR`], each an entry, sorted by id. A missing directory holds no file.
pub fn scan(render: &Path) -> Result<Vec<Entry>, RegistryError> {
    let found = occupant_files(render).map_err(|e| match e.twice {
        Some((id, first)) => RegistryError::Twice(id, first, e.path),
        None => RegistryError::Io(e.path, e.why),
    })?;
    let mut out = Vec::with_capacity(found.len());
    for f in found {
        let source = std::fs::read_to_string(&f.path)
            .map_err(|e| RegistryError::Io(f.path.clone(), e.to_string()))?;
        out.push(Entry::new(f.id, f.path, source)?);
    }
    one_name_each(&out)?;
    Ok(out)
}

/// Refuses two entries a node of one slot would call by one name ([`Entry::name`]), naming both files.
fn one_name_each(entries: &[Entry]) -> Result<(), RegistryError> {
    if scan::lifted() {
        return Ok(());
    }
    for (k, a) in entries.iter().enumerate() {
        if let Some(b) = entries[k + 1..]
            .iter()
            .find(|b| b.slot == a.slot && b.name() == a.name())
        {
            return Err(RegistryError::Twice(
                a.name().to_owned(),
                a.path.clone(),
                b.path.clone(),
            ));
        }
    }
    Ok(())
}

/// The render crate's own registry, built once ([`registry`]): what a node names by id is looked up here.
fn shipped() -> Result<&'static [Entry], RegistryError> {
    static SHIPPED_ENTRIES: OnceLock<Result<Vec<Entry>, RegistryError>> = OnceLock::new();
    // Building it must not look a source up by name: `Entry::new` reads each file's declarations as text.
    match SHIPPED_ENTRIES.get_or_init(registry) {
        Ok(entries) => Ok(entries),
        Err(e) => Err(e.clone()),
    }
}

/// The shipped source a node of `slot` names `name` ([`Entry::name`]): `pass_through` in a combiner,
/// `debug/generated/state` in a colour. A name no file gives that slot is refused, naming it; a name of another
/// slot's file is refused as that file's wrong slot.
pub fn named(slot: Kind, name: &str) -> Result<&'static Entry, RegistryError> {
    let entries = shipped()?;
    #[cfg(feature = "controls")]
    if probe::any_file() {
        if let Some(e) = entries.iter().find(|e| e.slot == slot) {
            return Ok(e);
        }
    }
    if let Some(e) = entries.iter().find(|e| e.slot == slot && e.name() == name) {
        return Ok(e);
    }
    #[cfg(feature = "controls")]
    if probe::unknown_allowed() {
        if let Some(e) = entries.iter().find(|e| e.slot == slot) {
            return Ok(e);
        }
    }
    match entries.iter().find(|e| e.name() == name) {
        Some(e) => Err(RegistryError::WrongSlot(e.id.clone(), e.slot, slot)),
        None => Err(RegistryError::Unknown(slot, name.to_owned())),
    }
}

/// A test's control's reach into the registry, on this thread only.
#[cfg(feature = "controls")]
pub mod probe {
    use std::cell::Cell;

    pub use super::scan::probe::twice_allowed;

    thread_local! {
        static UNKNOWN: Cell<bool> = const { Cell::new(false) };
        static ANY_FILE: Cell<bool> = const { Cell::new(false) };
    }

    /// `f` run with every name read as its slot's first source, known names too, as a lookup that ignored the name
    /// would.
    pub fn any_file_in<T>(f: impl FnOnce() -> T) -> T {
        let before = ANY_FILE.with(|l| l.replace(true));
        let out = f();
        ANY_FILE.with(|l| l.set(before));
        out
    }

    pub(super) fn any_file() -> bool {
        ANY_FILE.with(Cell::get)
    }

    /// `f` run with a name no file gives its slot read as the slot's first source, as a lookup that guessed would.
    pub fn unknown_allowed_in<T>(f: impl FnOnce() -> T) -> T {
        let before = UNKNOWN.with(|l| l.replace(true));
        let out = f();
        UNKNOWN.with(|l| l.set(before));
        out
    }

    pub(super) fn unknown_allowed() -> bool {
        UNKNOWN.with(Cell::get)
    }
}

/// The render crate's own registry: the entries of the files its build scanned and embedded ([`SHIPPED`]). It reads
/// no file, so it is the same wherever the binary runs; each entry's `path` names the file in the crate it was built
/// from.
pub fn registry() -> Result<Vec<Entry>, RegistryError> {
    embedded(Path::new(env!("CARGO_MANIFEST_DIR")), SHIPPED)
}

/// The entries of an embedded table `shipped`, each `(id, rel, source)`, in its order: `rel` is the file's path
/// relative to its render crate, `render`, and only names it.
pub fn embedded(
    render: &Path,
    shipped: &[(&str, &str, &str)],
) -> Result<Vec<Entry>, RegistryError> {
    shipped
        .iter()
        .map(|&(id, rel, source)| Entry::new(id.to_owned(), render.join(rel), source.to_owned()))
        .collect::<Result<Vec<Entry>, RegistryError>>()
        .and_then(|entries| one_name_each(&entries).map(|()| entries))
}

/// Registers a custom authored at run time in `registry`, a registry sorted by id: the WGSL `source` as `id`, whose
/// directory is its slot, `colour/<name>`, as a file dropped there would be (gui_state_contract §3). It is refused as
/// a scanned file is. An entry already at `id` is replaced, as a file saved over another is, and returned.
pub fn register(
    registry: &mut Vec<Entry>,
    id: &str,
    source: &str,
) -> Result<Option<Entry>, RegistryError> {
    let entry = Entry::new(
        id.to_owned(),
        PathBuf::from(format!("{id}.wgsl")),
        source.to_owned(),
    )?;
    match registry.binary_search_by(|e| e.id.as_str().cmp(id)) {
        Ok(at) => Ok(Some(std::mem::replace(&mut registry[at], entry))),
        Err(at) => {
            registry.insert(at, entry);
            Ok(None)
        }
    }
}

/// The source of the zero field: the input the catalogue's views take and do not read. A shipped source, named
/// [`ZERO`].
pub const ZERO_SOURCE: &str = include_str!("../shaders/wgsl/frag/source/zero.wgsl");

/// The zero field's name in the source slot.
pub const ZERO: &str = "zero";

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

/// A debug view whose colour is the WGSL `colour` as a stain's nodes: the zero source into the colour, the
/// pass-through combiner, `OUT`.
pub fn view_nodes(colour: &str) -> Vec<Node> {
    view_of(Occupant::Custom(colour.to_owned()))
}

/// The shipped debug view `name` ([`Entry::name`]) as a stain's nodes, each named by id: [`view_nodes`] of its
/// file, the same code.
pub fn named_view_nodes(name: &str) -> Vec<Node> {
    view_of(Occupant::BuiltIn(name.to_owned()))
}

/// The zero source into `colour`, the pass-through combiner, `OUT`.
fn view_of(colour: Occupant) -> Vec<Node> {
    let node = |kind, occupant, inputs: &[Option<usize>]| Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    vec![
        node(Kind::Source, Occupant::BuiltIn(ZERO.to_owned()), &[]),
        node(Kind::Colour, colour, &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".to_owned()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ]
}

impl ViewGenerator for Catalogue {
    /// `field`'s view as a stain: [`view_nodes`] of the view's colour.
    // Don't name the view by id here: a catalogue may hold entries no node can name (another crate's scan, a test's
    // altered copy), and naming would draw the shipped file instead of the entry's own text.
    fn view(&self, field: &str) -> Option<Vec<Node>> {
        let view = self
            .views
            .iter()
            .find(|e| e.id.strip_prefix("debug/generated/") == Some(field))?;
        Some(view_nodes(&view.source))
    }
}
