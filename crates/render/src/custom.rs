//! A custom node's side of compiling: what the author is told, and what the node keeps while a source is wrong.
//!
//! **Settling a request** ([`settle`]) is the whole of per-node isolation, pure and free of the GPU: a stain that does
//! not assemble is tried again with each changed node alone at its last valid occupant, and a node that fails alone
//! takes its last valid occupant back, the others keep theirs. [`crate::pipeline_cache`] calls it on the compile
//! worker; the tests call it without a device.
//!
//! **Problems** ([`Problems`]): one list for declaration errors and WGSL errors, each against a node and, where the
//! text says where, at the author's own line of that node's source ([`line_in`]), never the assembled text's.
//!
//! **Last valid, twice** ([`CustomNodes`]): per node, the last source whose declaration parsed (its description, so
//! the controls stay while a declaration line is half typed) and the status of its compile. A pipeline's last valid
//! occupant is the cache's ([`crate::pipeline_cache::PipelineCache::last_valid`]).
//!
//! **Values follow a declaration** ([`reconcile`]): by name; a value outside a new range is clamped and said so; a
//! changed type takes the default; a removed uniform's value is remembered, and comes back if the uniform does.

use std::collections::{BTreeMap, HashMap};

use crate::assemble::{
    assemble, AssembleError, Declaration, Fragment, Kind, Node, Occupant, Stain, Tier, Uniform,
    UniformType,
};
use crate::pipeline_cache::{CompileError, NodeKey};

// ── Settling a request ────────────────────────────────────────────────────────────────────────────────────────────

/// A request as it can render: the stain (each failed node at its last valid occupant), its assembled fragment, and
/// each node that failed alone, by position, with why.
pub struct Settled {
    pub stain: Stain,
    pub fragment: Fragment,
    pub failed: Vec<(usize, AssembleError)>,
}

/// `stain`, keyed by `keys`, as it can render given each node's last valid occupant, or the one error that says why it
/// cannot: a failure no node with a last valid occupant explains (a new node's, or two nodes' together), or a stain
/// that fails even with the failing nodes at their last valid occupants.
pub fn settle(
    stain: &Stain,
    keys: &[NodeKey],
    tier: Tier,
    last_valid: &HashMap<NodeKey, Occupant>,
) -> Result<Settled, CompileError> {
    let nodes = stain.nodes();
    let whole = match assemble(stain, tier) {
        Ok(fragment) => {
            return Ok(Settled {
                stain: stain.clone(),
                fragment,
                failed: Vec::new(),
            })
        }
        Err(whole) => whole,
    };
    // The nodes that can fall back: changed, with a last valid occupant. A node with none stays as it is in every
    // trial, and a failure only it explains is the stain's.
    let revertible: Vec<usize> = (0..nodes.len())
        .filter(|&i| {
            last_valid
                .get(&keys[i])
                .is_some_and(|o| *o != nodes[i].occupant)
        })
        .collect();
    let failed: Vec<(usize, AssembleError)> = revertible
        .iter()
        .filter_map(|&i| {
            let trial = with_last_valid(nodes, keys, last_valid, |j| {
                j != i && revertible.contains(&j)
            });
            Stain::new(trial)
                .and_then(|s| assemble(&s, tier))
                .err()
                .map(|e| (i, e))
        })
        .collect();
    if failed.is_empty() {
        return Err(CompileError::Stain(whole.to_string()));
    }
    let fallback = with_last_valid(nodes, keys, last_valid, |j| {
        failed.iter().any(|(i, _)| *i == j)
    });
    match Stain::new(fallback).and_then(|s| assemble(&s, tier).map(|f| (s, f))) {
        Ok((stain, fragment)) => Ok(Settled {
            stain,
            fragment,
            failed,
        }),
        Err(e) => Err(CompileError::Stain(e.to_string())),
    }
}

/// `nodes` with each node `j` for which `swap(j)` holds at its key's last valid occupant; `swap` holds only for nodes
/// that have one.
fn with_last_valid(
    nodes: &[Node],
    keys: &[NodeKey],
    last_valid: &HashMap<NodeKey, Occupant>,
    swap: impl Fn(usize) -> bool,
) -> Vec<Node> {
    nodes
        .iter()
        .enumerate()
        .map(|(j, n)| {
            let mut n = n.clone();
            if let Some(o) = last_valid.get(&keys[j]).filter(|_| swap(j)) {
                n.occupant = o.clone();
            }
            n
        })
        .collect()
}

// ── Problems ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// Where in a node a problem is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A `// @` declaration line.
    Declaration,
    /// The WGSL.
    Code,
    /// The stain as a whole: wiring, or an error no one node explains.
    Stain,
}

/// One entry of the Problems list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The node it is against; `None` for the stain's.
    pub node: Option<NodeKey>,
    pub origin: Origin,
    /// The author's own line in that node's source, 1-based, where the error says where.
    pub line: Option<usize>,
    pub message: String,
}

/// The Problems list the stain editor shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Problems(pub Vec<Problem>);

impl Problems {
    /// The problems against node `key`.
    pub fn of(&self, key: NodeKey) -> impl Iterator<Item = &Problem> {
        self.0.iter().filter(move |p| p.node == Some(key))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// `error` of node `key`, whose source is `text`, as a problem.
pub fn problem_of(key: Option<NodeKey>, text: &str, error: &AssembleError) -> Problem {
    let (origin, line) = match error {
        AssembleError::Declaration { line, .. } => (Origin::Declaration, Some(*line)),
        AssembleError::Compile(report) => (Origin::Code, line_in(text, report)),
        AssembleError::Slot(_) | AssembleError::Occupant(_) if key.is_some() => {
            (Origin::Code, None)
        }
        _ => (Origin::Stain, None),
    };
    Problem {
        node: key,
        origin,
        line,
        message: error.to_string(),
    }
}

/// The author's line (1-based) a naga report points at in `text`.
///
/// The report's own line numbers are the assembled module's, which holds the prelude and the other nodes before this
/// one, so they are no use to the author. The report quotes the offending line beside its number; this finds that
/// line in `text`. A line quoted more than once in `text` answers with its first, and one not in `text` (it came from
/// the prelude) with none.
pub fn line_in(text: &str, report: &str) -> Option<usize> {
    for quoted in report.lines().filter_map(quoted_line) {
        if let Some(at) = text.lines().position(|l| l.trim() == quoted) {
            return Some(at + 1);
        }
    }
    None
}

/// The source line a codespan report prints after its gutter: `12 │ let x = ;`.
fn quoted_line(report_line: &str) -> Option<&str> {
    let (gutter, rest) = report_line.split_once('│')?;
    gutter
        .trim()
        .chars()
        .all(|c| c.is_ascii_digit())
        .then_some(())?;
    (!gutter.trim().is_empty())
        .then(|| rest.trim())
        .filter(|t| !t.is_empty())
}

// ── Status and last valid description ─────────────────────────────────────────────────────────────────────────────

/// A custom node's compile status, for the indicator beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Its source went to the worker and no answer is in.
    Compiling,
    /// Its source compiled, or it has not been asked to compile anything else.
    Valid,
    /// Its source failed: it keeps its last valid one, and Problems says why.
    Failed,
}

/// What is kept of one node.
#[derive(Clone, Debug)]
struct Kept {
    /// The last source whose declaration parsed, and what it declared.
    text: String,
    description: Declaration,
    status: Status,
    /// The text last offered, which the node's problems are about.
    offered: String,
    problems: Vec<Problem>,
}

/// The per-node bookkeeping of custom sources: the last valid description, the status, the problems.
#[derive(Debug, Default)]
pub struct CustomNodes {
    kept: BTreeMap<NodeKey, Kept>,
}

impl CustomNodes {
    pub fn new() -> CustomNodes {
        CustomNodes::default()
    }

    /// Node `key` of kind `kind` is offered `text`, each keystroke if the app likes. The declaration is read now: if it
    /// parses, it is the node's description; if not, the node keeps the last one and its problem is listed with the
    /// author's line. Returns the description in force.
    pub fn offer(&mut self, key: NodeKey, kind: Kind, text: &str) -> Option<&Declaration> {
        let parsed = crate::assemble::declaration(kind, &Occupant::Custom(text.to_owned()));
        match parsed {
            Ok(description) => {
                self.kept.insert(
                    key,
                    Kept {
                        text: text.to_owned(),
                        description,
                        status: Status::Compiling,
                        offered: text.to_owned(),
                        problems: Vec::new(),
                    },
                );
            }
            Err(e) => {
                let problem = problem_of(Some(key), text, &e);
                if let Some(k) = self.kept.get_mut(&key) {
                    k.offered = text.to_owned();
                    k.status = Status::Failed;
                    k.problems = vec![problem];
                }
                // A node with no valid description yet has none to keep: its problem waits for one.
                else {
                    self.kept.insert(
                        key,
                        Kept {
                            text: String::new(),
                            description: Declaration::default(),
                            status: Status::Failed,
                            offered: text.to_owned(),
                            problems: vec![problem],
                        },
                    );
                }
            }
        }
        self.description(key)
    }

    /// Node `key`'s description in force: the last that parsed. `None` for a node never offered a source that parsed.
    pub fn description(&self, key: NodeKey) -> Option<&Declaration> {
        self.kept
            .get(&key)
            .filter(|k| !k.text.is_empty())
            .map(|k| &k.description)
    }

    /// The last source of node `key` whose declaration parsed.
    pub fn last_valid_text(&self, key: NodeKey) -> Option<&str> {
        self.kept
            .get(&key)
            .map(|k| k.text.as_str())
            .filter(|t| !t.is_empty())
    }

    /// Node `key`'s status.
    pub fn status(&self, key: NodeKey) -> Option<Status> {
        self.kept.get(&key).map(|k| k.status)
    }

    /// A compile of the stain `nodes`, keyed by `keys`, answered with `errors` (empty: it compiled whole): each node
    /// named by a [`CompileError::Node`] is `Failed`, with the error at its author's line; every other offered node
    /// of the stain is `Valid`, its earlier problems cleared. A [`CompileError::Stain`] fails no node: it is the
    /// stain's.
    pub fn answered(&mut self, nodes: &[Node], keys: &[NodeKey], errors: &[CompileError]) {
        let failed: BTreeMap<NodeKey, &AssembleError> = errors
            .iter()
            .filter_map(|e| match e {
                CompileError::Node { key, error, .. } => Some((*key, error)),
                CompileError::Stain(_) => None,
            })
            .collect();
        for (n, key) in nodes.iter().zip(keys) {
            let Some(k) = self.kept.get_mut(key) else {
                continue;
            };
            // Only a request that was about the text now offered answers for it.
            if !matches!(&n.occupant, Occupant::Custom(t) if *t == k.offered) {
                continue;
            }
            match failed.get(key) {
                Some(e) => {
                    k.status = Status::Failed;
                    k.problems = vec![problem_of(Some(*key), &k.offered, e)];
                }
                None if k.problems.iter().any(|p| p.origin == Origin::Declaration) => {}
                None => {
                    k.status = Status::Valid;
                    k.problems.clear();
                }
            }
        }
    }

    /// The Problems list: every node's, in node-key order, then the stain's own from `errors`.
    pub fn problems(&self, errors: &[CompileError]) -> Problems {
        let mut all: Vec<Problem> = self
            .kept
            .values()
            .flat_map(|k| k.problems.clone())
            .collect();
        all.extend(errors.iter().filter_map(|e| match e {
            CompileError::Stain(why) => Some(Problem {
                node: None,
                origin: Origin::Stain,
                line: None,
                message: why.clone(),
            }),
            CompileError::Node { .. } => None,
        }));
        Problems(all)
    }

    /// Node `key` is gone from the stain.
    pub fn forget(&mut self, key: NodeKey) {
        self.kept.remove(&key);
    }
}

// ── Values follow a declaration ───────────────────────────────────────────────────────────────────────────────────

/// What became of a value when its declaration changed.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// Outside the new range: clamped to it.
    Clamped {
        name: String,
        was: Vec<f64>,
        now: Vec<f64>,
    },
    /// The type changed, or the value no longer fits it: the default.
    Reset { name: String, was: Vec<f64> },
    /// A remembered value came back with its uniform.
    Restored { name: String },
}

/// A value held for a uniform the declaration no longer has, until the edit is undone or saved.
pub type Remembered = BTreeMap<String, (UniformType, Vec<f64>)>;

/// The result of [`reconcile`].
#[derive(Clone, Debug, PartialEq)]
pub struct Reconciled {
    /// A value for each declared uniform that had one to carry; the rest read their defaults.
    pub values: BTreeMap<String, Vec<f64>>,
    pub notes: Vec<Note>,
    pub remembered: Remembered,
}

/// The values a node held under its old declaration `old`, under the new one `new`. `values` are what was set;
/// `remembered` is what earlier edits took out.
// The plan is unclear here; `was <old name>` carrying a value over a rename is a grammar word, and chose to leave it
// out, because it changes the declaration grammar, a version of its own.
pub fn reconcile(
    old: &[Uniform],
    values: &BTreeMap<String, Vec<f64>>,
    remembered: &Remembered,
    new: &[Uniform],
) -> Reconciled {
    let mut out = BTreeMap::new();
    let mut notes = Vec::new();
    let mut keep: Remembered = remembered.clone();
    for u in new {
        let held = values
            .get(&u.name)
            .and_then(|v| {
                old.iter()
                    .find(|o| o.name == u.name)
                    .map(|o| (o.ty, v.clone()))
            })
            .map(|h| (h, false))
            .or_else(|| keep.get(&u.name).cloned().map(|h| (h, true)));
        let Some(((ty, value), was_remembered)) = held else {
            continue;
        };
        keep.remove(&u.name);
        if ty != u.ty {
            // The old value stays remembered in case the type comes back.
            keep.insert(u.name.clone(), (ty, value.clone()));
            if !was_remembered {
                notes.push(Note::Reset {
                    name: u.name.clone(),
                    was: value,
                });
            }
            continue;
        }
        if u.admits(&value) {
            if was_remembered {
                notes.push(Note::Restored {
                    name: u.name.clone(),
                });
            }
            out.insert(u.name.clone(), value);
            continue;
        }
        match clamped(u, &value) {
            Some(now) => {
                notes.push(Note::Clamped {
                    name: u.name.clone(),
                    was: value,
                    now: now.clone(),
                });
                out.insert(u.name.clone(), now);
            }
            None => notes.push(Note::Reset {
                name: u.name.clone(),
                was: value,
            }),
        }
    }
    for o in old {
        if !new.iter().any(|u| u.name == o.name) {
            if let Some(v) = values.get(&o.name) {
                keep.insert(o.name.clone(), (o.ty, v.clone()));
            }
        }
    }
    Reconciled {
        values: out,
        notes,
        remembered: keep,
    }
}

/// `value` brought into `u`'s range, where it is a scalar with one and that makes it a value of `u`.
fn clamped(u: &Uniform, value: &[f64]) -> Option<Vec<f64>> {
    let (lo, hi) = u.range?;
    let [v] = value else { return None };
    if v.is_nan() {
        return None;
    }
    let now = vec![v.clamp(lo, hi)];
    u.admits(&now).then_some(now)
}

// ── New from template ─────────────────────────────────────────────────────────────────────────────────────────────

/// A new shader for a node of `kind`: a source that declares itself, with the slot's function and one parameter, to
/// start from. `None` for `Out`, which has no code.
pub fn template(kind: Kind, id: &str, name: &str) -> Option<String> {
    let (group, about, maps, code) = match kind {
        Kind::Source => (
            "Source",
            "A field of your own.",
            "nothing",
            "fn source(ctx: Ctx) -> Field { return Field(node_scale, 0.0, 0.0, 0.0); }",
        ),
        Kind::Colour => (
            "Colour",
            "A colour of your own.",
            "its input",
            "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(node_scale); }",
        ),
        Kind::Brightness => (
            "Brightness",
            "A lightness of your own.",
            "its input",
            "fn brightness(ctx: Ctx) -> f32 { return node_scale; }",
        ),
        Kind::Combiner => (
            "Combiner",
            "A combiner of your own.",
            "colour · brightness",
            "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b * node_scale; }",
        ),
        Kind::Post => (
            "Post",
            "A post-process of your own.",
            "the colour",
            "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * node_scale; }",
        ),
        Kind::Out => return None,
    };
    // The parameter is a uniform: it is read as `uniforms.scale`, which the assembler gives its block.
    let code = code.replace("node_scale", "uniforms.scale");
    Some(format!(
        "// @id {id}\n// @name {name}\n// @group {group}\n// @about {about}\n// @maps {maps}\n// @glyph name\n\
         // @uniform scale: f32 = 1 [0, 1]\n// @control scale slider \"Scale\"\n{code}\n"
    ))
}
