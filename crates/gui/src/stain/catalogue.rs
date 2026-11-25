//! The editor's tables that no graph changes, built once from the registry (`engine::contract::stain::assembled`): the
//! library's groups and rows, the graph each row loads, and the shaders a node can be given. Every name and group is
//! its source's own, from its description. A folder's shaders join them ([`Catalogue::with`]), each row after ours.

use std::sync::{Arc, OnceLock};

use engine::contract::render_state::{NodeKind, Occupant, StainGraph};
use engine::contract::stain::assembled::{library, node_source, source_fields, Entry};
use engine::contract::stain::describe::{describe, from_source};
use engine::contract::stain::folder::Folder;
use engine::contract::stain::saved::FIELD_ID;

use super::yours::origin;
use super::{intern, Group, Row};

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
    /// Where it comes from, as its tooltip says.
    pub from: String,
}

/// What the library and the inspector list.
#[derive(Clone, Debug, PartialEq)]
pub struct Tables {
    /// The library's groups, in the order their first rows have in it.
    pub groups: Arc<[Group]>,
    /// How many entries it holds.
    pub modes: u32,
    /// Each row's graph, by its place in the library.
    pub graphs: Arc<[Option<StainGraph>]>,
    /// The shaders a node can be given, each kind's in the library's order, a source's fields first.
    pub choices: Arc<[Choice]>,
}

/// The shaders of a folder the tables list, the rest counted.
// A bound on the rows drawn each frame: the rest stay usable by the stains that hold them, and the panel says how many.
pub const LISTED: usize = 256;

/// The tables.
pub struct Catalogue {
    /// What the registry lists.
    pub tables: Tables,
    /// The [`Sources`] of each row's graph.
    sources: Vec<Option<Sources>>,
    /// Why a table could not be built, for the Problems pane.
    pub problems: Vec<String>,
}

impl Catalogue {
    /// The row whose graph is `g`, its params aside, whether `g` names its shaders or holds their texts.
    pub fn loaded(&self, g: &StainGraph) -> Option<usize> {
        let g = sources(g)?;
        self.sources.iter().position(|of| of.as_ref() == Some(&g))
    }

    /// The tables with `folder`'s shaders in them, each in its own group beside ours and among its kind's choices, the
    /// first [`LISTED`] by id.
    // The plan is unclear here; a shader of yours under `debug/` is offered to a node, though ours are not, because it
    // has no whole stain to load and could not be used otherwise.
    pub fn with(&self, folder: &Folder) -> Tables {
        let mut groups = self.tables.groups.to_vec();
        let mut graphs = self.tables.graphs.to_vec();
        let mut choices = self.tables.choices.to_vec();
        for shader in folder.shaders().take(LISTED) {
            let source = &shader.entry.source;
            let Ok(described) = from_source(shader.kind(), Some(source), &Default::default())
            else {
                continue;
            };
            let (name, from) = (intern(&described.name), origin(shader));
            let row = Row {
                place: graphs.len(),
                name,
                from: from.clone(),
            };
            match groups.iter_mut().find(|g| g.title == described.group) {
                Some(group) => group.rows.push(row),
                None => groups.push(Group {
                    title: intern(&described.group),
                    rows: vec![row],
                }),
            }
            graphs.push(None);
            choices.push(Choice {
                slot: shader.kind(),
                occupant: shader.occupant(),
                name,
                source: source.clone(),
                from,
            });
        }
        Tables {
            modes: u32::try_from(graphs.len()).unwrap_or(u32::MAX),
            groups: groups.into(),
            graphs: graphs.into(),
            choices: choices.into(),
        }
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
    let mut offer = |slot: NodeKind, occupant: Occupant, name: &str, id: &str| match node_source(
        slot, &occupant,
    ) {
        Ok(source) => choices.push(Choice {
            slot,
            occupant,
            name: intern(name),
            source: source.unwrap_or_default(),
            from: ours(id),
        }),
        Err(why) => problems.push(format!("{name}: {why}")),
    };
    for f in fields {
        let id = format!("{FIELD_ID}{f}");
        let occupant = Occupant::Field(f.clone());
        let name = describe(NodeKind::Source, &occupant, &Default::default())
            .map_or(f, |described| described.name);
        offer(NodeKind::Source, occupant, &name, &id);
    }
    // The plan is unclear here; chose to offer a node every shader of its kind but the debug views, because those are
    // whole stains the library loads.
    for e in entries.iter().filter(|e| !e.debug) {
        let occupant = Occupant::Builtin(e.name.clone());
        offer(e.slot, occupant, &e.description.name, &e.id);
    }
    let (mut groups, mut graphs) = (Vec::new(), Vec::new());
    for (title, entries) in grouped(entries) {
        let mut rows = Vec::new();
        for e in entries {
            rows.push(Row {
                place: graphs.len(),
                name: intern(&e.description.name),
                from: ours(&e.id),
            });
            graphs.push(e.graph);
        }
        groups.push(Group {
            title: intern(&title),
            rows,
        });
    }
    Catalogue {
        sources: graphs.iter().map(|g| sources(g.as_ref()?)).collect(),
        tables: Tables {
            modes: u32::try_from(graphs.len()).unwrap_or(u32::MAX),
            groups: groups.into(),
            graphs: graphs.into(),
            choices: choices.into(),
        },
        problems,
    }
}

/// Where the shipped source `id` comes from.
pub fn ours(id: &str) -> String {
    format!("ours · {id}")
}
