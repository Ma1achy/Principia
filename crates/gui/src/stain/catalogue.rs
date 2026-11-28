//! The editor's tables that no graph changes, built once from the registry (`engine::contract::stain::assembled`) and
//! the preset files (`engine::contract::stain::preset`): the library's groups and rows, the graph each row loads, and
//! the shaders a node can be given. A registry row's name and group are its source's own, from its description; a debug
//! view's group, and every preset's name, group, lock and measure, are the library's (colour_composition §9.6). A
//! folder's shaders join them ([`Catalogue::with`]), each row after ours.

use std::sync::{Arc, OnceLock};

use engine::contract::render_state::{NodeKind, Occupant, StainGraph};
use engine::contract::stain::assembled::{library, node_source, source_fields, Entry};
use engine::contract::stain::describe::from_source;
use engine::contract::stain::folder::Folder;
use engine::contract::stain::preset::{self, group_of, DebugGroup, Section};
use engine::contract::stain::saved::FIELD_SOURCE;

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
    /// The shaders a node can be given, each kind's in the library's order, the field source first.
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
    /// The row whose graph is `g`, its params aside but a field they pick, whether `g` names its shaders or holds their
    /// texts.
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
                locked: false,
                measure: None,
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

/// A graph's live nodes, each its kind, its source with any field it picks ([`super::needs::read_as`]) and the nodes
/// that feed it.
type Sources = Vec<(NodeKind, Option<String>, Vec<Option<usize>>)>;

fn sources(g: &StainGraph) -> Option<Sources> {
    let nodes = g.canonical().ok()?.nodes;
    nodes
        .into_iter()
        .map(|n| {
            let read_as = super::needs::read_as(n.kind, &n.occupant, &n.params);
            Some((n.kind, node_source(n.kind, &read_as).ok()?, n.inputs))
        })
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
    // The one field source, offered as "Field" in its own spelling, `Occupant::Field`, at the field it defaults to; the
    // field is then chosen with its picker. Its registry entry is not offered again below.
    let field_source = entries.iter().find(|e| is_field_source(e.slot, &e.name));
    let default = field_source.and_then(|e| {
        let c = e
            .description
            .controls
            .iter()
            .find(|c| c.uniform == FIELD_SOURCE)?;
        fields.get(*c.default.first()? as usize).cloned()
    });
    let unoffered = match (field_source, default) {
        (Some(e), Some(f)) => {
            offer(NodeKind::Source, Occupant::Field(f), FIELD, &e.id);
            None
        }
        _ => Some(format!(
            "{FIELD}: no shipped `{FIELD_SOURCE}` source with a default field"
        )),
    };
    // The plan is unclear here; chose to offer a node every shader of its kind but the debug views, because those are
    // whole stains the library loads.
    for e in entries
        .iter()
        .filter(|e| !e.debug && !is_field_source(e.slot, &e.name))
    {
        let occupant = Occupant::Builtin(e.name.clone());
        offer(e.slot, occupant, &e.description.name, &e.id);
    }
    problems.extend(unoffered);
    let presets = preset::library().unwrap_or_else(|why| {
        problems.push(format!("the presets: {why}"));
        Vec::new()
    });
    let (mut groups, mut graphs) = (Vec::new(), Vec::<Option<StainGraph>>::new());
    let mut push = |title: &str, row: Listed, graphs: &mut Vec<Option<StainGraph>>| {
        let row_ = Row {
            place: graphs.len(),
            name: intern(&row.name),
            from: row.from,
            locked: row.locked,
            measure: row.measure,
        };
        graphs.push(row.graph);
        match groups.iter_mut().find(|g: &&mut Group| g.title == title) {
            Some(group) => group.rows.push(row_),
            None => groups.push(Group {
                title: intern(title),
                rows: vec![row_],
            }),
        }
    };
    let (ours, debug): (Vec<Entry>, Vec<Entry>) = entries.into_iter().partition(|e| !e.debug);
    for (title, entries) in grouped(ours) {
        for e in entries {
            let field = is_field_source(e.slot, &e.name);
            let mut row = Listed::entry(e);
            // Named as the menu names it, not by the field it starts on.
            if field {
                row.name = FIELD.to_owned();
            }
            push(&title, row, &mut graphs);
        }
    }
    // A preset file whose graph a registry row already loads (the outcome palette's) is that row, not listed again.
    let listed: Vec<StainGraph> = graphs.iter().flatten().cloned().collect();
    let (production, files): (Vec<_>, Vec<_>) = presets
        .into_iter()
        .filter(|p| !listed.contains(&p.graph))
        .partition(|p| p.section == Section::Production);
    for p in production {
        push(PRESETS, Listed::preset(p), &mut graphs);
    }
    for group in DebugGroup::ALL {
        for p in files.iter().filter(|p| p.group == Some(group)) {
            push(group.name(), Listed::preset(p.clone()), &mut graphs);
        }
        let views: Vec<&Entry> = debug
            .iter()
            .filter(|e| group_of(&e.id) == Some(group))
            .collect();
        for e in &views {
            let mut row = Listed::entry((*e).clone());
            let name = &e.description.name;
            if views.iter().filter(|v| v.description.name == *name).count() > 1 {
                row.name = told_apart(e);
            }
            push(group.name(), row, &mut graphs);
        }
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

/// The name of the one source that reads a field of the run, chosen with its picker.
pub const FIELD: &str = "Field";

/// Whether the shipped shader `name` of `slot` is the field source.
fn is_field_source(slot: NodeKind, name: &str) -> bool {
    slot == NodeKind::Source && name == FIELD_SOURCE
}

/// The heading of the production presets that no registry row loads.
// The plan is unclear here (a production preset names no group); chose one heading of their own, after the slots' groups
// and before the debug section, because the bivariate presets are whole stains, not one slot's shader.
pub const PRESETS: &str = "Presets";

/// The name of a debug view that shares its name with another of its group (the derived `ftle` and the payload's):
/// the name, then its source's own group, `ftle (derived)`.
pub fn told_apart(e: &Entry) -> String {
    let d = &e.description;
    let from = d.group.strip_prefix("Debug · ").unwrap_or(&d.group);
    format!("{} ({from})", d.name)
}

/// A row as the library lists it, before its place is known.
struct Listed {
    name: String,
    from: String,
    locked: bool,
    measure: Option<String>,
    graph: Option<StainGraph>,
}

impl Listed {
    fn entry(e: Entry) -> Self {
        Self {
            name: e.description.name,
            from: ours(&e.id),
            locked: e.locked,
            measure: None,
            graph: e.graph,
        }
    }

    fn preset(p: preset::Preset) -> Self {
        let dir = match p.section {
            Section::Production => "production",
            Section::Debug => "debug",
        };
        Self {
            locked: p.locked(),
            from: ours(&format!("presets/{dir}/{}", p.id)),
            name: p.name,
            measure: p.measure,
            graph: Some(p.graph),
        }
    }
}

/// Where the shipped source `id` comes from.
pub fn ours(id: &str) -> String {
    format!("ours · {id}")
}
