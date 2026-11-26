//! The preset library as graphs: a preset is a whole saved stain that replaces the current graph on selection
//! (render_gui_spec §11). Its file holds the catalogue's metadata and the graph together (`render::presets::file`), so
//! the library's listing and what selecting loads cannot differ.
//!
//! **Locked.** A debug preset loads locked: [`Loaded::apply`] on it forks, the edit lands on a custom copy that is
//! editable and says where it came from, and the library's preset is never touched (it is read from its file at each
//! [`load`]). A production preset loads editable and the edit is made in place. The fork is one way: a graph never
//! becomes a preset again.

use render::presets::file::{self, DebugGroup, Section};

use crate::contract::set_field::{Edit, RenderField};
use crate::contract::stain::edit::{self, Refusal, StainEdit};
use crate::contract::stain::graph::{GraphError, StainGraph};

/// A library preset: its metadata and its graph, read from one file.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// The file's name in its section.
    pub id: &'static str,
    /// The name the library lists it under.
    pub name: String,
    pub section: Section,
    /// The debug group; `Some` exactly for a debug preset.
    pub group: Option<DebugGroup>,
    /// What a quantitative preset measures, in words.
    pub measure: Option<String>,
    pub graph: StainGraph,
}

impl Preset {
    /// Whether it loads locked.
    pub fn locked(&self) -> bool {
        self.section.locked()
    }
}

/// `file` as a preset, its graph read as any saved stain is.
pub fn read(file: file::File) -> Result<Preset, GraphError> {
    let graph = serde_json::from_value::<StainGraph>(file.graph)
        .map_err(|e| GraphError(format!("preset `{}`: {e}", file.id)))?;
    Ok(Preset {
        id: file.id,
        name: file.name,
        section: file.section,
        group: file.group,
        measure: file.measure,
        graph,
    })
}

/// Every library preset, in the library's order.
pub fn library() -> Result<Vec<Preset>, GraphError> {
    file::files()
        .map_err(GraphError)?
        .into_iter()
        .map(read)
        .collect()
}

/// The library preset `id` of `section`.
pub fn preset(section: Section, id: &str) -> Result<Preset, GraphError> {
    read(file::file(section, id).map_err(GraphError)?)
}

/// The graph of the library preset `id` of `section`.
pub fn graph(section: Section, id: &str) -> Result<StainGraph, GraphError> {
    Ok(preset(section, id)?.graph)
}

/// What an accepted edit did to a [`Loaded`] preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// The graph was editable and is edited.
    InPlace,
    /// The graph was locked: it is now a custom copy, edited, and the preset is as it was.
    Forked,
}

/// A preset selected into the editor: the graph that replaced the current one, and whether it is locked.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    /// The library preset it was loaded from, kept after a fork.
    pub from: &'static str,
    /// Its name: the preset's, or the custom copy's.
    pub name: String,
    /// Locked until the first edit forks it.
    pub locked: bool,
    pub graph: StainGraph,
}

/// Selects `preset`: its graph replaces whatever was there, locked if it is a debug preset.
pub fn select(preset: &Preset) -> Loaded {
    Loaded {
        from: preset.id,
        name: preset.name.clone(),
        locked: preset.locked(),
        graph: preset.graph.clone(),
    }
}

impl Loaded {
    /// Applies `edit`. A refused edit changes nothing and forks nothing. On a locked graph an accepted edit forks:
    /// the copy is named `"<name> (custom)"` and unlocked, and the edit is made on it.
    // Don't edit the locked graph and mark it unlocked afterwards: a preset shown locked must never hold an edit.
    pub fn apply(&mut self, edit: &StainEdit) -> Result<Applied, Refusal> {
        let set = edit::apply(&self.graph, edit)?;
        let Edit::Render(RenderField::StainGraph(graph)) = set.edit else {
            unreachable!("a stain edit writes the stain graph");
        };
        self.graph = graph;
        if self.locked {
            self.locked = false;
            self.name = format!("{} (custom)", self.name);
            Ok(Applied::Forked)
        } else {
            Ok(Applied::InPlace)
        }
    }
}
