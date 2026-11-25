//! A stain node as it is saved: the code it runs, by value, so a picture never changes by itself when a shipped file
//! changes (the node SDK §3, gap 17).
//!
//! **The form.** Every node but the identity is saved as one shape, whoever wrote its code:
//!
//! ```json
//! { "shader": { "id": "combiner/multiply", "version": null, "hash": "…", "source": "…" } }
//! ```
//!
//! `source` is the whole text the node runs, its declarations included; `hash` is its 64-bit FNV-1a, sixteen lowercase
//! hex digits ([`hash`]), so a damaged text is refused, never drawn. `id` names where the code came from: a shipped
//! source's registry id, [`FIELD_ID`]`<field>` for a field's generated source, a shader of your folder's `@id`, `null`
//! for a custom. `version` is the node's own version where its source declares one (`@node_version`), else `null`.
//! `"edited": "<hash>"`, written only for code edited in this stain and saved to no file, is the [`hash`] of the text
//! the edit was made from: such a node is never offered an update, only a revert, and its save over a file is refused
//! where the file is no longer that text. The identity is `"none"`.
//!
//! **Reading** ([`read`]). The text is the node. Where it is, byte for byte, what its id names in this build, the node
//! is that shipped source or field again; otherwise it stays the saved text ([`Occupant::Pinned`]), drawn as it was
//! saved, and [`StainGraph::updates`] offers the newer file. Nothing updates on its own ([`StainGraph::update`]).
//! Reading never needs your folder ([`folder`](super::folder)): a shader of yours opens as its saved text, and
//! [`StainGraph::updates_in`] offers the folder's file where it differs.
//!
//! **The old forms**, which hold no source for a shipped node: `{"builtin": <name>}` (a shipped source by its name in its
//! slot), `{"field": <field>}` and `{"custom": <text>}`. A `builtin` is read as the file its name gives in this build,
//! the only reading it has, and every old form is written in the new form. A name no file gives its slot is refused
//! with that reason; nothing is guessed.

use std::collections::BTreeMap;

use render::assemble::{self, Kind};
use render::registry;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::contract::stain::folder::Folder;
use crate::contract::stain::graph::{GraphError, Node, NodeId, NodeKind, Occupant, StainGraph};
use crate::contract::stain::ramp::Ramp;

/// The id prefix of a field's generated source: `source/ctx.E_0` is the source of `ctx.E_0`.
// The plan is unclear here; chose `source/ctx.<field>` as a field source's id because it sits in the source slot's
// directory beside `source/zero` and is the name the editor shows, `ctx.<field>`, and no shipped file can be named so.
pub const FIELD_ID: &str = "source/ctx.";

/// A node's code held by value with the id of the source it came from: a shader of your folder, a shipped source
/// whose file in this build differs from it (or this build has none), or code edited in this stain ([`Pinned::edited`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pinned {
    /// The id it was saved under: a registry id, [`FIELD_ID`]`<field>`, or a shader of yours' `@id`.
    pub id: String,
    /// The node's own version, where its source declared one.
    pub version: Option<String>,
    /// The text it runs.
    pub source: String,
    /// Where the text is an edit made in this stain and saved to no file, the [`hash`] of the text it was made from:
    /// offered a revert, never an update.
    pub edited: Option<String>,
}

/// The new form's shader: what [`read`] takes and [`write`] gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shader {
    pub id: Option<String>,
    pub version: Option<String>,
    pub hash: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited: Option<String>,
}

/// An occupant as saved: the new form, `shader`, or one of the three old forms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    None,
    Shader(Shader),
    Builtin(String),
    Field(String),
    Custom(String),
}

/// The 64-bit FNV-1a hash of `source`'s UTF-8 bytes, as sixteen lowercase hex digits: a saved node's `hash`.
pub fn hash(source: &str) -> String {
    format!("{:016x}", ledger::version::fnv1a64(source.as_bytes()))
}

/// The text a node of `kind` holding `occupant` runs, or why not; `None` for the identity.
fn text(kind: NodeKind, occupant: &Occupant) -> Result<Option<String>, GraphError> {
    Ok(assemble::occupant_source(kind.into(), &occupant.into())?)
}

/// The shipped occupant id `id` names in a node of `kind` in this build, with its text: a registry entry of the slot,
/// as [`Occupant::Builtin`], or a field's generated source, as [`Occupant::Field`].
pub fn current(kind: NodeKind, id: &str) -> Option<(Occupant, String)> {
    current_in(kind, id, None)
}

/// [`current`], or where this build ships nothing under `id`, the shader of `folder` with that id, of `kind`'s slot,
/// as [`Occupant::Pinned`] with the file's text. An id your folder gives twice, not yet chosen, names nothing.
pub fn current_in(kind: NodeKind, id: &str, folder: Option<&Folder>) -> Option<(Occupant, String)> {
    shipped(kind, id).or_else(|| {
        let s = folder?.shader(id).ok()?;
        (NodeKind::from(s.entry.slot) == kind).then(|| (s.occupant(), s.entry.source.clone()))
    })
}

fn shipped(kind: NodeKind, id: &str) -> Option<(Occupant, String)> {
    if let Some(field) = id.strip_prefix(FIELD_ID) {
        let occupant = Occupant::Field(field.to_owned());
        let source = text(kind, &occupant).ok().flatten()?;
        return Some((occupant, source));
    }
    // A file under its slot's directory is named by its name there, any other by its whole id
    // (`registry::Entry::name`); either way the name must give this id back.
    let short = id.rsplit_once('/').map_or(id, |(_, name)| name);
    let entry = [short, id]
        .into_iter()
        .filter_map(|name| registry::named(Kind::from(kind), name).ok())
        .find(|e| e.id == id)?;
    Some((
        Occupant::Builtin(entry.name().to_owned()),
        entry.source.clone(),
    ))
}

/// `occupant` in a node of `kind` in the new form.
pub fn write(kind: NodeKind, occupant: &Occupant) -> Result<Form, GraphError> {
    let (id, version) = match occupant {
        Occupant::None => return Ok(Form::None),
        Occupant::Custom(_) => (None, None),
        Occupant::Field(f) => (Some(format!("{FIELD_ID}{f}")), None),
        Occupant::Builtin(name) => {
            let entry =
                registry::named(Kind::from(kind), name).map_err(|e| GraphError(e.to_string()))?;
            (Some(entry.id.clone()), node_version(&entry.source))
        }
        Occupant::Pinned(p) => (Some(p.id.clone()), p.version.clone()),
    };
    let source = text(kind, occupant)?.unwrap_or_default();
    Ok(Form::Shader(Shader {
        id,
        version,
        hash: hash(&source),
        source,
        edited: match occupant {
            Occupant::Pinned(p) => p.edited.clone(),
            _ => None,
        },
    }))
}

/// The occupant a node of `kind` saved as `form` holds, or why it cannot be read: a damaged text, a version with no
/// id, or an old `builtin` naming no file of the slot.
pub fn read(kind: NodeKind, form: Form) -> Result<Occupant, GraphError> {
    let s = match form {
        Form::None => return Ok(Occupant::None),
        Form::Field(f) => return Ok(Occupant::Field(f)),
        Form::Custom(text) => return Ok(Occupant::Custom(text)),
        Form::Builtin(name) => {
            registry::named(Kind::from(kind), &name).map_err(|e| {
                GraphError(format!("the saved built-in `{name}` cannot be read: {e}"))
            })?;
            return Ok(Occupant::Builtin(name));
        }
        Form::Shader(s) => s,
    };
    if !probe::hash_skipped() && hash(&s.source) != s.hash {
        return Err(GraphError(format!(
            "a saved node is damaged: its hash is {} but its source hashes to {}{}",
            s.hash,
            hash(&s.source),
            s.id.as_deref()
                .map(|id| format!(" (saved as `{id}`)"))
                .unwrap_or_default()
        )));
    }
    let Some(id) = s.id else {
        if s.edited.is_some() {
            return Err(GraphError(
                "a saved node is marked edited but has no id: an edit is of a source with one"
                    .into(),
            ));
        }
        if let Some(v) = s.version {
            return Err(GraphError(format!(
                "a saved node has version `{v}` but no id: a version labels a source with an id"
            )));
        }
        return Ok(Occupant::Custom(s.source));
    };
    if probe::by_id() {
        if let Some((occupant, _)) = current_in(kind, &id, probe::folder().as_ref()) {
            return Ok(occupant);
        }
    }
    match current(kind, &id) {
        Some((occupant, now)) if now == s.source && s.edited.is_none() => Ok(occupant),
        _ => Ok(Occupant::Pinned(Pinned {
            id,
            version: s.version,
            source: s.source,
            edited: s.edited,
        })),
    }
}

/// A node as it is saved: its occupant in [`Form`].
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeForm {
    id: NodeId,
    kind: NodeKind,
    occupant: Form,
    #[serde(default)]
    params: BTreeMap<String, Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ramp: Option<Ramp>,
}

impl Serialize for Node {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let occupant = write(self.kind, &self.occupant).map_err(S::Error::custom)?;
        NodeForm {
            id: self.id,
            kind: self.kind,
            occupant,
            params: self.params.clone(),
            ramp: self.ramp.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Node, D::Error> {
        let n = NodeForm::deserialize(d)?;
        let occupant = read(n.kind, n.occupant)
            .map_err(|e| D::Error::custom(format!("node {}: {e}", n.id.0)))?;
        Ok(Node {
            id: n.id,
            kind: n.kind,
            occupant,
            params: n.params,
            ramp: n.ramp,
        })
    }
}

/// A newer file for a node saved by value ([`Occupant::Pinned`]): what [`StainGraph::updates`] offers and
/// [`StainGraph::update`] applies.
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    /// The node.
    pub node: NodeId,
    /// The id it was saved under.
    pub id: String,
    /// The version it was saved at, and this build's, each where declared.
    pub saved_version: Option<String>,
    pub version: Option<String>,
    /// The text it runs, and this build's file.
    pub saved: String,
    pub source: String,
    /// Whether the code differs, so the picture can change; `false` where only the declarations (labels, controls'
    /// words) do.
    pub code_changed: bool,
    /// What the node holds once updated.
    pub occupant: Occupant,
}

/// What applying an update did to the node's params: those this build's file still declares and admits are kept; the
/// rest go back to the file's defaults, each listed by name.
#[derive(Clone, Debug, PartialEq)]
pub struct Updated {
    pub reset: Vec<String>,
}

/// The `@node_version` `source` declares, if it parses and declares one.
pub(crate) fn node_version(source: &str) -> Option<String> {
    assemble::Declaration::parse(source)
        .ok()
        .and_then(|d| d.described.node_version)
}

impl StainGraph {
    /// Each node saved by value whose id names a file in this build: the update it is offered, in node order.
    pub fn updates(&self) -> Vec<Update> {
        self.updates_in(None)
    }

    /// [`StainGraph::updates`], a shader of yours' file looked for in `folder` too.
    // The plan is unclear here; chose to offer an update whenever the file differs, an older `@node_version` included,
    // and to give both versions so the app can say which is newer, rather than withhold an offer by a version.
    pub fn updates_in(&self, folder: Option<&Folder>) -> Vec<Update> {
        self.nodes()
            .filter_map(|n| self.offer_in(n.id, folder))
            .collect()
    }

    /// The update node `node` is offered, if any ([`StainGraph::updates`]).
    pub fn offer(&self, node: NodeId) -> Option<Update> {
        self.offer_in(node, None)
    }

    /// The update node `node` is offered, if any ([`StainGraph::updates_in`]): none where its text is the file's, or
    /// it holds an edit ([`Pinned::edited`]).
    pub fn offer_in(&self, node: NodeId, folder: Option<&Folder>) -> Option<Update> {
        let n = self.node(node)?;
        let Occupant::Pinned(p) = &n.occupant else {
            return None;
        };
        if p.edited.is_some() {
            return None;
        }
        let (occupant, source) = current_in(n.kind, &p.id, folder)?;
        if source == p.source {
            return None;
        }
        Some(Update {
            node,
            id: p.id.clone(),
            saved_version: p.version.clone(),
            version: node_version(&source),
            code_changed: assemble::key_text(&p.source) != assemble::key_text(&source),
            saved: p.source.clone(),
            source,
            occupant,
        })
    }

    /// Applies node `node`'s update: it holds this build's file from now on. Its params are kept by name where the
    /// file declares and admits them, and the others are reset to the file's defaults and listed. Refused where the
    /// node is offered none. One edit: refused, the graph is as it was.
    // The plan is unclear here; chose to reset a param the new file no longer admits rather than clamp it, until
    // editing a declaration says how a value carries over.
    pub fn update(&mut self, node: NodeId) -> Result<Updated, GraphError> {
        self.update_in(node, None)
    }

    /// [`StainGraph::update`] to the update [`StainGraph::offer_in`] gives.
    pub fn update_in(
        &mut self,
        node: NodeId,
        folder: Option<&Folder>,
    ) -> Result<Updated, GraphError> {
        let offer = self
            .offer_in(node, folder)
            .ok_or_else(|| GraphError(format!("node {} is offered no update", node.0)))?;
        self.replace(node, offer.occupant)
    }

    /// Node `node`'s occupant made `occupant`, its params kept where `occupant` declares and admits them, the others
    /// reset and listed. One edit.
    pub(crate) fn replace(
        &mut self,
        node: NodeId,
        occupant: Occupant,
    ) -> Result<Updated, GraphError> {
        let kind = self
            .node(node)
            .map(|n| n.kind)
            .ok_or_else(|| GraphError(format!("no node {}", node.0)))?;
        let d = assemble::declaration(kind.into(), &(&occupant).into())?;
        let params = &self.node(node).expect("checked above").params;
        let reset: Vec<String> = params
            .iter()
            .filter(|(name, v)| !d.uniforms.iter().any(|u| &u.name == *name && u.admits(v)))
            .map(|(name, _)| name.clone())
            .collect();
        let mut next = self.clone();
        for name in &reset {
            next.unset_param(node, name);
        }
        next.set_occupant(node, occupant)?;
        *self = next;
        Ok(Updated { reset })
    }
}

/// A test's control's reach into the reader, on this thread only.
pub mod probe {
    #[cfg(feature = "controls")]
    use std::cell::Cell;

    #[cfg(feature = "controls")]
    use crate::contract::stain::folder::Folder;

    #[cfg(feature = "controls")]
    thread_local! {
        static BY_ID: Cell<bool> = const { Cell::new(false) };
        static NO_HASH: Cell<bool> = const { Cell::new(false) };
        static FOLDER: std::cell::RefCell<Option<Folder>> = const { std::cell::RefCell::new(None) };
    }

    /// `f` run with a saved node read as whatever its id names in this build or in `folder`, its saved text ignored.
    #[cfg(feature = "controls")]
    pub fn by_id_with_folder_in<T>(folder: &Folder, f: impl FnOnce() -> T) -> T {
        let before = FOLDER.with(|l| l.replace(Some(folder.clone())));
        let out = by_id_in(f);
        FOLDER.with(|l| l.replace(before));
        out
    }

    pub(super) fn folder() -> Option<crate::contract::stain::folder::Folder> {
        #[cfg(feature = "controls")]
        return FOLDER.with(|l| l.borrow().clone());
        #[cfg(not(feature = "controls"))]
        None
    }

    /// `f` run with a saved node read as whatever its id names in this build, its saved text ignored, as a reader that
    /// resolves by id would.
    #[cfg(feature = "controls")]
    pub fn by_id_in<T>(f: impl FnOnce() -> T) -> T {
        let before = BY_ID.with(|l| l.replace(true));
        let out = f();
        BY_ID.with(|l| l.set(before));
        out
    }

    /// `f` run with a saved node's hash not checked, as a reader that trusts its text would.
    #[cfg(feature = "controls")]
    pub fn hash_skipped_in<T>(f: impl FnOnce() -> T) -> T {
        let before = NO_HASH.with(|l| l.replace(true));
        let out = f();
        NO_HASH.with(|l| l.set(before));
        out
    }

    pub(super) fn by_id() -> bool {
        #[cfg(feature = "controls")]
        return BY_ID.with(Cell::get);
        #[cfg(not(feature = "controls"))]
        false
    }

    pub(super) fn hash_skipped() -> bool {
        #[cfg(feature = "controls")]
        return NO_HASH.with(Cell::get);
        #[cfg(not(feature = "controls"))]
        false
    }
}
