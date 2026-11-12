//! The fragment side's occupant registry: the scanned filesystem (gui_state_contract §3). The registry is not a
//! hand-maintained list: [`scan()`] walks the hand-written occupants under `shaders/wgsl/frag/` ([`OCCUPANT_DIR`]), the
//! debug catalogue's generated views under `frag/debug/generated/` ([`GENERATED_DIR`], RQ-219) and its generated
//! reductions under `frag/debug/reductions/` ([`REDUCTIONS_DIR`]: each vector field's direction cosines and the ternary
//! masses, TASK-M1-12), one entry per `.wgsl` file, `{id, slot, source, category, uniformSchema, inputDomains}`. Adding
//! a file is the act of registering it.
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
    let found = occupant_files(render).map_err(|e| RegistryError::Io(e.path, e.why))?;
    let mut out = Vec::with_capacity(found.len());
    for f in found {
        let source = std::fs::read_to_string(&f.path)
            .map_err(|e| RegistryError::Io(f.path.clone(), e.to_string()))?;
        out.push(Entry::new(f.id, f.path, source)?);
    }
    Ok(out)
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
        .collect()
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
