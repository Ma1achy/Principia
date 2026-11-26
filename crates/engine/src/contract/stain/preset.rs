//! The preset library as graphs: a preset is a whole saved stain with its catalogue metadata, from one file of the
//! render crate (`render::presets::library`), and selecting it replaces the current graph wholesale ([`Preset::load`]).
//!
//! **Locked.** A debug preset loads locked: the graph is read-only, and the first edit that changes it forks a custom
//! copy ([`Loaded::apply`]), after which the copy is the user's and the library's preset is as it was. A production
//! preset loads editable. The lock is held beside the graph, never inside it, so a graph's saved form and keys do not
//! depend on where it came from. Preset → graph is one way: nothing writes a graph back to the library.

use render::presets::library::{self, File};

use crate::contract::set_field::{Edit, RenderField, SetField};
use crate::contract::stain::edit::{apply, Refusal, StainEdit};
use crate::contract::stain::graph::{GraphError, StainGraph};

/// One preset of the library: its row's metadata and the graph that selecting it loads.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// The registry id of the view it shows (`debug/generated/state`).
    pub id: String,
    pub name: String,
    /// The debug group, or the production section.
    pub group: String,
    /// Whether it loads read-only.
    pub locked: bool,
    pub about: String,
    /// What a quantitative preset measures.
    pub measure: Option<String>,
    graph: StainGraph,
}

impl Preset {
    /// The preset a file holds: its graph read as a saved stain.
    pub fn from_file(f: File) -> Result<Preset, GraphError> {
        let graph: StainGraph = serde_json::from_value(f.graph)
            .map_err(|e| GraphError(format!("preset `{}`: {e}", f.id)))?;
        Ok(Preset {
            id: f.id,
            name: f.name,
            group: f.group,
            locked: f.locked,
            about: f.about,
            measure: f.measure,
            graph,
        })
    }

    /// The preset's graph, as the file gives it.
    pub fn graph(&self) -> &StainGraph {
        &self.graph
    }

    /// Selecting the preset: a copy of its graph to replace the current one, locked where the preset is.
    pub fn load(&self) -> Loaded {
        Loaded {
            graph: self.graph.clone(),
            name: self.name.clone(),
            preset: self.id.clone(),
            locked: self.locked,
            forked: false,
        }
    }
}

/// The library: every preset, in the files' order.
pub fn library() -> Result<Vec<Preset>, GraphError> {
    library::files()
        .map_err(GraphError)?
        .into_iter()
        .map(Preset::from_file)
        .collect()
}

/// The preset `id`.
pub fn preset(id: &str) -> Result<Preset, GraphError> {
    Preset::from_file(library::file(id).map_err(GraphError)?)
}

/// A graph as selecting a preset left it: the graph the stain editor holds, and whether it is the library's, read-only.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    graph: StainGraph,
    name: String,
    preset: String,
    locked: bool,
    forked: bool,
}

impl Loaded {
    pub fn graph(&self) -> &StainGraph {
        &self.graph
    }

    /// The name shown for it: the preset's, or its copy's.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether edits are refused until they fork a copy.
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// The preset this is a custom copy of, once an edit forked it.
    pub fn forked_from(&self) -> Option<&str> {
        self.forked.then_some(self.preset.as_str())
    }

    /// Applies `edit`. On a locked graph an accepted edit first forks: the graph becomes a custom copy named
    /// `<name> (custom)`, editable, and the edit is made on the copy. A refused edit changes nothing, the lock included.
    pub fn apply(&mut self, edit: &StainEdit) -> Result<SetField, Refusal> {
        let done = apply(&self.graph, edit)?;
        if let Edit::Render(RenderField::StainGraph(next)) = &done.edit {
            self.graph = next.clone();
        }
        if self.locked {
            self.locked = false;
            self.forked = true;
            self.name = format!("{} (custom)", self.name);
        }
        Ok(done)
    }
}
