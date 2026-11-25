//! The editor's tables that no graph changes, built once from the registry (`engine::contract::stain::assembled`): the
//! library's groups and rows, the graph each row loads, and the shaders a node can be given. Every name and group is
//! its source's own, from its description.

use std::sync::OnceLock;

use engine::contract::render_state::{NodeKind, Occupant, StainGraph};
use engine::contract::stain::assembled::{library, node_source, source_fields, Entry};
use engine::contract::stain::describe::describe;

use super::{intern, Group};

/// A shader the inspector offers a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// The kind of node it fills.
    pub slot: NodeKind,
    /// The occupant it sets.
    pub occupant: Occupant,
    /// Its name.
    pub name: &'static str,
    /// Its source: a node is this choice when its own source is this text.
    pub source: String,
}

/// The tables.
pub struct Catalogue {
    /// The library's groups, in the order their first rows have in it.
    pub groups: Vec<Group>,
    /// How many entries it holds.
    pub modes: u32,
    /// Each row's graph, by its place in the library.
    pub graphs: Vec<Option<StainGraph>>,
    /// The [`Sources`] of each row's graph.
    sources: Vec<Option<Sources>>,
    /// The shaders a node can be given, each kind's in the library's order, a source's fields first.
    pub choices: Vec<Choice>,
    /// Why a table could not be built, for the Problems pane.
    pub problems: Vec<String>,
}

impl Catalogue {
    /// The row whose graph is `g`, its params aside, whether `g` names its shaders or holds their texts.
    pub fn loaded(&self, g: &StainGraph) -> Option<usize> {
        let g = sources(g)?;
        self.sources.iter().position(|of| of.as_ref() == Some(&g))
    }
}

/// A graph's live nodes, each its kind, its source and the nodes that feed it.
type Sources = Vec<(NodeKind, Option<String>, Vec<Option<usize>>)>;

fn sources(g: &StainGraph) -> Option<Sources> {
    let nodes = g.canonical().ok()?.nodes;
    nodes
        .into_iter()
        .map(|n| Some((n.kind, node_source(n.kind, &n.occupant).ok()?, n.inputs)))
        .collect()
}

/// The fragment key of `g`: what its code is compiled under, its params not in it.
pub fn key(g: &StainGraph) -> Option<u64> {
    Some(g.canonical().ok()?.fragment_key())
}

/// The [`Catalogue`], built on first use and kept.
pub fn catalogue() -> &'static Catalogue {
    static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();
    CATALOGUE.get_or_init(build)
}

/// `entries` by their descriptions' groups, each group where its first entry is.
pub(crate) fn grouped(entries: Vec<Entry>) -> Vec<(String, Vec<Entry>)> {
    let mut out: Vec<(String, Vec<Entry>)> = Vec::new();
    for e in entries {
        match out.iter_mut().find(|(g, _)| *g == e.description.group) {
            Some((_, rows)) => rows.push(e),
            None => out.push((e.description.group.clone(), vec![e])),
        }
    }
    out
}

fn build() -> Catalogue {
    let mut problems = Vec::new();
    let entries = library().unwrap_or_else(|why| {
        problems.push(format!("the library: {why}"));
        Vec::new()
    });
    let fields = source_fields().unwrap_or_else(|why| {
        problems.push(format!("the source fields: {why}"));
        Vec::new()
    });
    let mut choices = Vec::new();
    let mut offer =
        |slot: NodeKind, occupant: Occupant, name: &str| match node_source(slot, &occupant) {
            Ok(source) => choices.push(Choice {
                slot,
                occupant,
                name: intern(name),
                source: source.unwrap_or_default(),
            }),
            Err(why) => problems.push(format!("{name}: {why}")),
        };
    for f in fields {
        let occupant = Occupant::Field(f.clone());
        let name = describe(NodeKind::Source, &occupant, &Default::default())
            .map_or(f, |described| described.name);
        offer(NodeKind::Source, occupant, &name);
    }
    // The plan is unclear here; chose to offer a node every shader of its kind but the debug views, because those are
    // whole stains the library loads.
    for e in entries.iter().filter(|e| !e.debug) {
        let occupant = Occupant::Builtin(e.name.clone());
        offer(e.slot, occupant, &e.description.name);
    }
    let modes = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    let (mut groups, mut graphs) = (Vec::new(), Vec::new());
    for (title, entries) in grouped(entries) {
        let rows: Vec<&'static str> = entries
            .iter()
            .map(|e| intern(&e.description.name))
            .collect();
        groups.push(Group {
            title: intern(&title),
            // Kept for the session, as the interned names are: the tables are built once.
            rows: Vec::leak(rows),
        });
        graphs.extend(entries.into_iter().map(|e| e.graph));
    }
    Catalogue {
        groups,
        modes,
        sources: graphs.iter().map(|g| sources(g.as_ref()?)).collect(),
        graphs,
        choices,
        problems,
    }
}
