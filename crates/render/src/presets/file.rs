//! The preset library's files: each `presets/{production,debug}/<name>.json` holds one preset's catalogue metadata and
//! its whole graph, so the library's listing and what selecting a preset loads come from one file.
//!
//! ```json
//! { "name": "n̂ × FTLE", "group": "addressing", "measure": "…", "graph": { "nodes": […], "wires": […] } }
//! ```
//!
//! The graph is the engine's saved stain, left as JSON here: `render` has no graph type, and `engine` reads it as it
//! reads any saved stain. A shipped preset names a shipped source by id (`{"builtin": …}`), never by value, so it follows
//! that source's file; its inline text is for what no shipped source holds. The section is the directory: a `debug`
//! preset loads locked and holds a [`DebugGroup`]; a `production` one loads editable and holds none.

use serde::Deserialize;

/// Where a preset sits in the library, and so whether it loads locked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    /// Editable on load.
    Production,
    /// Locked on load: the first edit forks a custom copy.
    Debug,
}

impl Section {
    /// Whether a preset of this section loads locked.
    pub fn locked(self) -> bool {
        self == Section::Debug
    }
}

/// The debug section's six groups (colour_composition §9.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DebugGroup {
    Addressing,
    QuadStructure,
    DecodedIc,
    Validity,
    Diagnostic,
    Profiling,
}

impl DebugGroup {
    /// The six, in the order the library lists them.
    pub const ALL: [DebugGroup; 6] = [
        DebugGroup::Addressing,
        DebugGroup::QuadStructure,
        DebugGroup::DecodedIc,
        DebugGroup::Validity,
        DebugGroup::Diagnostic,
        DebugGroup::Profiling,
    ];

    /// The group's heading.
    pub fn name(self) -> &'static str {
        match self {
            DebugGroup::Addressing => "addressing",
            DebugGroup::QuadStructure => "quad-structure",
            DebugGroup::DecodedIc => "decoded-IC",
            DebugGroup::Validity => "validity",
            DebugGroup::Diagnostic => "diagnostic",
            DebugGroup::Profiling => "profiling",
        }
    }
}

/// The group of the shipped debug source `id` (`debug/s_depth`): the group colour_composition §9.6 files it under.
// The plan is unclear here (§9.6 names groups, not every view); chose a view of the ledger's generated payload fields to
// be diagnostic and the quad cache age to be profiling, because §9.6 files the payload field views and the cache age
// there and names nothing finer.
pub fn group_of(id: &str) -> Option<DebugGroup> {
    let rest = id.strip_prefix("debug/")?;
    Some(match rest {
        "s_depth" | "s_impurity" | "s_state" | "s_spread" | "s_suspect" | "s_priority"
        | "s_coherence" | "s_ancestor_gap" => DebugGroup::QuadStructure,
        "s_cache_age" => DebugGroup::Profiling,
        r if r.starts_with("derived/total_substeps") => DebugGroup::Profiling,
        _ => DebugGroup::Diagnostic,
    })
}

/// A preset file as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: String,
    #[serde(default)]
    group: Option<DebugGroup>,
    #[serde(default)]
    measure: Option<String>,
    graph: serde_json::Value,
}

/// A preset of the library: its catalogue metadata and its graph, from one file.
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    /// The file's name in its section: `outcome`, `tl_rb`.
    pub id: &'static str,
    /// The name the library lists it under.
    pub name: String,
    pub section: Section,
    /// The debug group; `Some` exactly for a debug preset.
    pub group: Option<DebugGroup>,
    /// What a quantitative preset measures, in words; `None` for one that measures nothing.
    pub measure: Option<String>,
    /// The saved stain, as the engine reads it.
    pub graph: serde_json::Value,
}

impl File {
    /// Whether it loads locked.
    pub fn locked(&self) -> bool {
        self.section.locked()
    }
}

macro_rules! files {
    ($($section:ident / $dir:literal / $id:literal),* $(,)?) => {
        /// Every shipped preset file: (section, file name, text). A file in the directories and not here is refused by
        /// the library's own test.
        pub const FILES: &[(Section, &str, &str)] = &[
            $((Section::$section, $id, include_str!(concat!("../../presets/", $dir, "/", $id, ".json")))),*
        ];
    };
}

files! {
    Production / "production" / "outcome",
    Production / "production" / "outcome_ftle",
    Production / "production" / "outcome_t_end",
    Debug / "debug" / "uv_screen",
    Debug / "debug" / "uv_quad",
    Debug / "debug" / "coordinate_uv",
    Debug / "debug" / "coordinate_delta",
    Debug / "debug" / "tl_rb",
    Debug / "debug" / "centre_bg",
    Debug / "debug" / "index_grey",
}

/// Reads the preset file `text` of `section`, or why not.
pub fn parse(section: Section, id: &'static str, text: &str) -> Result<File, String> {
    let raw: Raw = serde_json::from_str(text).map_err(|e| format!("preset `{id}`: {e}"))?;
    match (section, raw.group) {
        (Section::Debug, None) => {
            return Err(format!("debug preset `{id}` names no group"));
        }
        (Section::Production, Some(g)) => {
            return Err(format!(
                "production preset `{id}` names the debug group {}",
                g.name()
            ));
        }
        _ => {}
    }
    Ok(File {
        id,
        name: raw.name,
        section,
        group: raw.group,
        measure: raw.measure,
        graph: raw.graph,
    })
}

/// Every shipped preset, in [`FILES`]' order.
pub fn files() -> Result<Vec<File>, String> {
    FILES
        .iter()
        .map(|&(section, id, text)| parse(section, id, text))
        .collect()
}

/// The shipped preset file `id` of `section`.
pub fn file(section: Section, id: &str) -> Result<File, String> {
    FILES
        .iter()
        .find(|(s, i, _)| *s == section && *i == id)
        .ok_or_else(|| format!("no {section:?} preset `{id}`"))
        .and_then(|&(s, i, text)| parse(s, i, text))
}
