//! What the Stain editor shows of a graph (render_gui_spec Part II §1, §10, §11): the code the one assembler makes of
//! it ([`assemble`]), each node's own WGSL ([`node_source`]), the fields a source may read and the library, the render
//! crate's registry ([`library`]). The GUI reaches the assembler and the registry here, through the contract.

use render::assemble::{self, Kind};
use render::registry::{self, Category};

pub use render::assemble::Tier;

use crate::contract::stain::describe::{describe, Description};
use crate::contract::stain::graph::{GraphError, NodeKind, Occupant, StainGraph};
use crate::presets;

/// A graph assembled.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembled {
    /// The graph's own part of [`Assembled::pipeline`]: each live node's uniforms and function, and `shade()`.
    pub code: String,
    /// The whole fragment source: the prelude, the read side, then the graph's own part.
    pub pipeline: String,
    /// The fragment key it was assembled under: the params are not in it.
    pub key: u64,
}

/// `g` assembled at `tier`, a presentation view, or why not, as the assembler says: every live node's code, with
/// nothing the proof finds cannot change the picture left out (`render::assemble::assemble_kept`). The pipeline that
/// draws leaves that out, and draws the same bits.
// The plan is unclear here; chose to show the code with nothing dropped, because the code view maps its text back to
// the nodes, and a node or wire the proof drops would vanish from it while still on the canvas.
pub fn assemble(g: &StainGraph, tier: Tier) -> Result<Assembled, GraphError> {
    let canonical = g.canonical()?;
    let pipeline =
        assemble::assemble_kept(canonical.stain(), tier, assemble::View::Presentation)?.source;
    // The nodes follow the context, the last thing before them.
    let code = match pipeline.split_once(assemble::CONTEXT) {
        Some((_, nodes)) => nodes.trim_start().to_owned(),
        None => pipeline.clone(),
    };
    Ok(Assembled {
        code,
        pipeline,
        key: canonical.fragment_key(),
    })
}

/// The WGSL `occupant` gives a node of `kind`, as written: a shipped source's file, a custom's text, a field's
/// generated source; `None` for the identity.
pub fn node_source(kind: NodeKind, occupant: &Occupant) -> Result<Option<String>, GraphError> {
    Ok(assemble::occupant_source(kind.into(), &occupant.into())?)
}

/// The fields a built-in source may read.
pub fn source_fields() -> Result<Vec<String>, GraphError> {
    Ok(assemble::source_fields()?)
}

/// One entry of the library: an occupant of the registry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its registry id, `colour/outcome_state`.
    pub id: String,
    /// Whether it is a debug occupant.
    pub debug: bool,
    /// The slot it is valid in.
    pub slot: NodeKind,
    /// The name a node of its slot names it by (`render::registry::Entry::name`): `outcome_state`,
    /// `debug/generated/state`.
    pub name: String,
    /// The whole graph that shows it, where it is a view by itself: what selecting it loads (§11).
    pub graph: Option<StainGraph>,
    /// Its description, from its source: its name and group are the row's and its heading's.
    pub description: Description,
}

impl Entry {
    /// Its directory, `debug/generated`.
    pub fn dir(&self) -> &str {
        self.id.rsplit_once('/').map_or("", |(dir, _)| dir)
    }
}

/// The library: the registry's entries, the slots' before the debug ones, each directory's together, in id order.
///
/// An entry's graph is the outcome stain for the outcome palette and, for a debug colour view, the catalogue's own
/// wiring of it (`render::registry::named_view_nodes`); a colour whose fields no entry names (`margin`), a source, a
/// combiner and a post are no view by themselves. Every node of every graph names a shipped source by id, or a field.
// The plan is unclear here (§11 has debug presets load locked); chose editable because no graph holds a lock.
pub fn library() -> Result<Vec<Entry>, String> {
    let mut out = registry::registry()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|e| {
            let name = e.name().to_owned();
            let debug = e.category == Category::Debug;
            let graph = match e.slot {
                Kind::Colour if name == render::colour::outcome::name() => Some(presets::outcome()),
                Kind::Colour if debug => Some(presets::named_view(&name)),
                _ => None,
            }
            .transpose()
            .map_err(|why| format!("{}: {why}", e.id))?;
            let occupant = Occupant::Builtin(name.clone());
            let description = describe(e.slot.into(), &occupant, &Default::default())
                .map_err(|why| format!("{}: {why}", e.id))?;
            Ok(Entry {
                slot: e.slot.into(),
                id: e.id,
                debug,
                name,
                graph,
                description,
            })
        })
        .collect::<Result<Vec<Entry>, String>>()?;
    out.sort_by(|a, b| (a.debug, a.dir(), &a.id).cmp(&(b.debug, b.dir(), &b.id)));
    Ok(out)
}
