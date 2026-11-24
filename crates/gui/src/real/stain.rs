//! The real engine's stain content (render_gui_spec Part II §1): the store's own graph through the real assembler and
//! the registry. [`Compiler`] holds the stain as last compiled; a graph that differs from it only in params is the same
//! code, so it is never assembled again, and one that does not compile leaves the last that did in place.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use engine::contract::render_state::StainGraph;
use engine::contract::stain::assembled::{
    assemble, library, node_source, source_fields, Assembled, Entry, Tier,
};

use crate::clock::time::Instant;
use crate::stain::{intern, Content, Group, COMBINERS};

/// The tier the real engine's records are stored at.
pub const TIER: Tier = engine::field::record::TIER;

/// The stain's tables that no graph changes, built once.
pub struct Tables {
    /// The library's groups, the registry's directories.
    pub groups: Vec<Group>,
    /// How many entries it holds.
    pub modes: u32,
    /// Each row's graph, by its place in the library.
    pub graphs: Vec<Option<StainGraph>>,
    /// Each row's name and, where it loads a graph, that graph's fragment key.
    rows: Vec<(&'static str, Option<u64>)>,
    /// The fields a source may read.
    pub fields: Vec<&'static str>,
    /// A colour node's built-in occupants, each its id and name.
    pub colours: Vec<(&'static str, &'static str)>,
    /// Why a table could not be built, for the Problems pane.
    pub problems: Vec<String>,
}

/// The heading of the library's directory `dir`.
pub fn heading(dir: &str) -> String {
    match dir {
        "colour" => "Colour".to_owned(),
        "combiner" => "Combiner".to_owned(),
        "post" => "Post".to_owned(),
        "debug" => "Debug · structure".to_owned(),
        "debug/accumulators" => "Debug · accumulators".to_owned(),
        "debug/derived" => "Debug · derived".to_owned(),
        "debug/generated" => "Debug · payload fields".to_owned(),
        "debug/reductions" => "Debug · vector reductions".to_owned(),
        "debug/word" => "Debug · word".to_owned(),
        // A directory joins by being there, as a file does (gui_state_contract §3).
        other => other.to_owned(),
    }
}

/// The library's rows' names, in order: each entry's own, with its directory's last part where two share it.
fn names(entries: &[Entry]) -> Vec<&'static str> {
    entries
        .iter()
        .map(|e| {
            let twins = entries.iter().filter(|o| o.name() == e.name()).count();
            let dir = e.dir().rsplit('/').next().unwrap_or_default();
            if twins > 1 && dir != "generated" {
                intern(&format!("{} ({dir})", e.name()))
            } else {
                intern(e.name())
            }
        })
        .collect()
}

/// The [`Tables`], built on first use and kept.
pub fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut problems = Vec::new();
        let entries = library().unwrap_or_else(|why| {
            problems.push(format!("the library: {why}"));
            Vec::new()
        });
        let fields = source_fields().unwrap_or_else(|why| {
            problems.push(format!("the source fields: {why}"));
            Vec::new()
        });
        let names = names(&entries);
        let mut groups: Vec<(&str, Vec<&'static str>)> = Vec::new();
        for (e, name) in entries.iter().zip(&names) {
            match groups.last_mut() {
                Some((dir, rows)) if *dir == e.dir() => rows.push(name),
                _ => groups.push((e.dir(), vec![name])),
            }
        }
        Tables {
            groups: groups
                .into_iter()
                .map(|(dir, rows)| Group {
                    title: intern(&heading(dir)),
                    // Kept for the session, as the interned names are: the tables are built once.
                    rows: Vec::leak(rows),
                })
                .collect(),
            modes: u32::try_from(entries.len()).unwrap_or(u32::MAX),
            rows: entries
                .iter()
                .zip(&names)
                .map(|(e, name)| (*name, e.graph.as_ref().and_then(key)))
                .collect(),
            colours: entries
                .iter()
                .filter(|e| e.dir() == "colour")
                .filter_map(|e| e.builtin.as_deref())
                .map(|id| (intern(id), intern(id)))
                .collect(),
            graphs: entries.into_iter().map(|e| e.graph).collect(),
            fields: fields.iter().map(|f| intern(f)).collect(),
            problems,
        }
    })
}

/// The fragment key of `g`: what its code is compiled under, its params not in it.
pub fn key(g: &StainGraph) -> Option<u64> {
    Some(g.canonical().ok()?.fragment_key())
}

/// What a compile did.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// It assembled the graph, in so many ms.
    Assembled(f64),
    /// The graph is the one compiled, its params aside: nothing was assembled.
    Cached,
    /// The graph does not compile, and why: the last that did stays.
    Failed(String),
}

/// The graph a compile was asked for, and what came of it.
#[derive(Clone, Debug)]
struct Asked {
    graph: StainGraph,
    key: Option<u64>,
    outcome: Outcome,
}

/// What drawing the stain reported: the pipelines' errors, which name the node that keeps its last valid source, and
/// the notes on the records it is drawn from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// The errors.
    pub problems: Vec<String>,
    /// The notes.
    pub notes: Vec<String>,
}

/// The stain as last compiled.
#[derive(Debug, Default)]
pub struct Compiler {
    asked: Option<Asked>,
    /// The code of the last graph that compiled, and the ms its assembly took.
    valid: Option<(Assembled, f64)>,
    /// The content as last made, and what it is of: the graph and what drawing it reported.
    content: Option<(StainGraph, Drawn, Content)>,
    assemblies: u64,
}

impl Compiler {
    /// How many times a graph has been assembled.
    pub fn assemblies(&self) -> u64 {
        self.assemblies
    }

    /// Compiles `g`: assembles it unless the last graph asked for has its code.
    pub fn compile(&mut self, g: &StainGraph) -> Outcome {
        let wanted = key(g);
        self.content = None;
        if let Some((_, ms)) = self.valid.as_ref().filter(|(c, _)| Some(c.key) == wanted) {
            self.asked = Some(Asked {
                graph: g.clone(),
                key: wanted,
                outcome: Outcome::Assembled(*ms),
            });
            return Outcome::Cached;
        }
        let begun = Instant::now();
        self.assemblies += 1;
        let outcome = match assemble(g, TIER) {
            Ok(code) => {
                let ms = begun.elapsed().as_secs_f64() * 1e3;
                self.valid = Some((code, ms));
                Outcome::Assembled(ms)
            }
            Err(why) => Outcome::Failed(why.to_string()),
        };
        self.asked = Some(Asked {
            graph: g.clone(),
            key: wanted,
            outcome: outcome.clone(),
        });
        outcome
    }

    /// The graph to draw for `g`, the graph as it stands: `g` while its code is what was last compiled, so a param edit
    /// shows at once; else the graph last asked for, until it is recompiled.
    pub fn drawn(&self, g: &StainGraph) -> StainGraph {
        match &self.asked {
            Some(a) if a.key != key(g) => a.graph.clone(),
            _ => g.clone(),
        }
    }

    /// The editor's content for `g`, the graph as it stands, with what drawing it reported, `drawn`: the code of the last
    /// that compiled, `g` compiled first when none has been asked for.
    pub fn content(&mut self, g: &StainGraph, drawn: &Drawn) -> Content {
        if self.asked.is_none() {
            self.compile(g);
        }
        if let Some((of, with, content)) = &self.content {
            if of == g && with == drawn {
                return content.clone();
            }
        }
        let content = self.made(g, drawn);
        self.content = Some((g.clone(), drawn.clone(), content.clone()));
        content
    }

    /// [`Compiler::content`], made.
    fn made(&self, g: &StainGraph, drawn: &Drawn) -> Content {
        let tables = tables();
        let wanted = key(g);
        let stale = self.asked.as_ref().is_some_and(|a| a.key != wanted);
        let mut problems = tables.problems.clone();
        let mut notes = drawn.notes.clone();
        // The pipelines' errors name the node, so where there are any they stand for the assembler's.
        let failed = match self.asked.as_ref().map(|a| &a.outcome) {
            Some(Outcome::Failed(why)) => vec![why.clone()],
            _ => Vec::new(),
        };
        let failed = if drawn.problems.is_empty() {
            failed
        } else {
            drawn.problems.clone()
        };
        let compiled = match self.asked.as_ref().map(|a| &a.outcome) {
            Some(Outcome::Assembled(ms)) if failed.is_empty() => {
                format!("✔ assembled and validated · {ms:.1} ms")
            }
            _ => "✘ does not compile · the last that did stays".to_owned(),
        };
        problems.extend(failed);
        if stale {
            notes.push("note: edited since; Recompile draws it".to_owned());
        }
        if let Ok(form) = g.canonical() {
            notes.push(format!(
                "note: {} of {} nodes live",
                form.nodes.len(),
                g.nodes().count()
            ));
        }
        let (code, pipeline) = self
            .valid
            .as_ref()
            .map(|(a, _)| (a.code.clone(), a.pipeline.clone()))
            .unwrap_or_default();
        Content {
            library: &tables.groups,
            modes: tables.modes,
            loaded: wanted
                .and_then(|k| tables.rows.iter().find(|(_, of)| *of == Some(k)))
                .map(|(name, _)| *name),
            code,
            pipeline,
            node_code: g
                .nodes()
                .map(|n| {
                    let text = match node_source(n.kind, &n.occupant) {
                        Ok(Some(text)) => text,
                        Ok(None) => {
                            "// None: the identity (render_gui_spec Part II §13)\n".to_owned()
                        }
                        Err(why) => format!("// {why}\n"),
                    };
                    (n.id, text)
                })
                .collect::<BTreeMap<_, _>>(),
            compiled,
            problems,
            notes,
            fields: &tables.fields,
            combiners: &COMBINERS,
            colours: &tables.colours,
            graphs: &tables.graphs,
        }
    }
}
