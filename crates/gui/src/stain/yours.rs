//! Your shaders in the editor (the node SDK, `docs/design/principia_node_sdk.md` §3): the line on their folder under the
//! library, the panel on it in the graph's inspector, the update a node is offered, where a node's shader comes from,
//! Save as a node, Save to your file, Revert, and the code of a node as it is typed. A shader of yours is listed by the
//! library and the inspector's choices as ours are ([`super::catalogue`]); nothing on the canvas tells it apart.
//!
//! [`Shelf`] is what is shown, made by [`crate::shaders`], which holds the folder and is the only thing that reads or
//! writes it: a control that needs it asks the app ([`Ask`]). Nothing is written by a setting or by an edit of code,
//! which the stain holds, undoable; only a Save writes.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, pos2, vec2, Align, Button, Rect, RichText, TextEdit, Ui, UiBuilder};
use engine::contract::render_state::{Node, NodeId, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::node_source;
use engine::contract::stain::custom::{self, reconcile, Note, Reconciled, Remembered};
use engine::contract::stain::edit::Refusal;
use engine::contract::stain::folder::{self, Duplicate, Folder, Problem, Shader};
use engine::contract::stain::saved::{self, Update, FIELD_ID};

use super::{catalogue, code_view, intern, Content, Out, REVERT};
use crate::keyboard::scopes::{Scope, ScopeId};
use crate::windows::Keys;

/// The line's height under the library, in points.
pub const SHELF_H: f32 = 26.0;
/// The refused files, and the ids given twice, the panel draws; the rest are counted.
pub const SHOWN: usize = 50;

/// The library's line: it shows the panel.
pub const SHELF: ScopeId = "stain_yours";
/// The panel's Read again.
pub const READ: ScopeId = "stain_yours_read";
/// The inspector's Save as a node….
pub const SAVE_AS: ScopeId = "stain_yours_save_as";
/// The inspector's Save to your file.
pub const SAVE_FILE: ScopeId = "stain_yours_save_file";
/// Save as a node's fields: the name, the id and the author.
pub const NAME: ScopeId = "stain_yours_name";
/// See [`NAME`].
pub const ID: ScopeId = "stain_yours_id";
/// See [`NAME`].
pub const AUTHOR: ScopeId = "stain_yours_author";
/// Save as a node's Save.
pub const SAVE: ScopeId = "stain_yours_save";
/// Save as a node's Cancel.
pub const CANCEL: ScopeId = "stain_yours_cancel";
/// The node's code, typed in.
pub const CODE: ScopeId = "stain_code_text";
/// Applies the code typed.
pub const APPLY: ScopeId = "stain_code_apply";
/// Drops the code typed.
pub const DISCARD: ScopeId = "stain_code_discard";

const UPDATE: &str = "stain_yours_update_";
const KEEP: &str = "stain_yours_keep_";
const CHOOSE: &str = "stain_yours_choose_";

/// Node `node`'s Update.
pub fn update_id(node: NodeId) -> ScopeId {
    intern(&format!("{UPDATE}{}", node.0))
}

/// Node `node`'s Keep this one.
pub fn keep_id(node: NodeId) -> ScopeId {
    intern(&format!("{KEEP}{}", node.0))
}

/// The panel's choice `k`, over every file of every id given twice.
pub fn choose_id(k: usize) -> ScopeId {
    intern(&format!("{CHOOSE}{k}"))
}

/// Your shaders' folder, as the editor shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shelf {
    /// Where it is; none where there is none.
    pub folder: Option<String>,
    /// Whether it is being read.
    pub reading: bool,
    /// Whether a save is being written.
    pub saving: bool,
    /// Why it could not be read.
    pub failed: Option<String>,
    /// How many shaders it gives.
    pub shaders: usize,
    /// The updates the stain's nodes are offered.
    pub offers: Vec<Offer>,
    /// The ids two files or more give.
    pub duplicates: Vec<Duplicate>,
    /// The files refused, and why.
    pub refused: Vec<Problem>,
    /// What each node holds.
    pub nodes: BTreeMap<NodeId, Held>,
}

/// An update a node is offered.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub node: NodeId,
    /// The node's name, its kind's and its shader's id.
    pub title: String,
    /// What is offered, in words.
    pub line: String,
    /// The hash of the text offered.
    pub hash: String,
    /// Whether the person kept what the node holds.
    pub kept: bool,
}

/// A node's shader, as the inspector shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    /// Where it comes from.
    pub from: String,
    /// Its code, where it has any.
    pub code: Option<String>,
    /// Whether its code is in this stain alone.
    pub unsaved: bool,
    /// Whether Revert has a source to go back to.
    pub revert: bool,
    /// Whether it is an edit of a shader in the folder, whose file it can be saved to.
    pub file: bool,
}

impl Shelf {
    /// What node `node` holds.
    pub fn held(&self, node: NodeId) -> Option<&Held> {
        self.nodes.get(&node)
    }

    /// The update node `node` is offered.
    fn offered(&self, node: NodeId) -> Option<&Offer> {
        self.offers.iter().find(|o| o.node == node)
    }

    /// The updates not kept.
    fn open(&self) -> impl Iterator<Item = &Offer> {
        self.offers.iter().filter(|o| !o.kept)
    }

    /// The ids given twice with no file chosen.
    fn unchosen(&self) -> usize {
        let open = self.duplicates.iter().filter(|d| d.chosen.is_none());
        open.count()
    }

    /// The files the panel offers, in order: each its id's place, its id and its file.
    fn files(&self) -> impl Iterator<Item = (usize, &str, &str)> {
        let shown = self.duplicates.iter().enumerate().take(SHOWN);
        shown.flat_map(|(i, d)| d.files.iter().map(move |f| (i, d.id.as_str(), f.as_str())))
    }
}

/// `n` of `what`, "s" for more than one.
fn count(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

/// `what`, with the author and version `source` gives.
fn labelled(what: &str, author: Option<&str>, version: Option<&str>) -> String {
    let mut out = what.to_owned();
    if let Some(a) = author {
        out.push_str(&format!(" · by {a}"));
    }
    if let Some(v) = version {
        out.push_str(&format!(" · version {v}"));
    }
    out
}

/// Where `shader`, a shader of yours, comes from.
pub fn origin(shader: &Shader) -> String {
    let d = &shader.described;
    let mut out = labelled(
        &format!("yours · {}", shader.id),
        d.author.as_deref(),
        d.node_version.as_deref(),
    );
    if let Some(licence) = &d.licence {
        out.push_str(&format!(" · {licence}"));
    }
    format!("{out} · {}", shader.file)
}

/// What node `n` holds, with `folder`'s shaders; none of a node that holds no shader.
pub fn held(n: &Node, folder: &Folder) -> Option<Held> {
    let code = node_source(n.kind, &n.occupant).ok().flatten();
    let current = |id: &str| saved::current_in(n.kind, id, Some(folder));
    let (from, unsaved, revert, file) = match &n.occupant {
        Occupant::None => return None,
        Occupant::Field(f) => (
            catalogue::ours(&format!("{FIELD_ID}{f}")),
            false,
            false,
            false,
        ),
        Occupant::Builtin(_) => {
            let id = match saved::write(n.kind, &n.occupant) {
                Ok(saved::Form::Shader(s)) => s.id.unwrap_or_default(),
                _ => String::new(),
            };
            (catalogue::ours(&id), false, false, false)
        }
        Occupant::Custom(_) => ("in this stain alone".to_owned(), true, false, false),
        Occupant::Pinned(p) if p.edited.is_some() => (
            format!("edited in this stain, not saved · from {}", p.id),
            true,
            current(&p.id).is_some(),
            folder.has(&p.id),
        ),
        Occupant::Pinned(p) => {
            let line = match folder.shader(&p.id) {
                Ok(s) if s.entry.source == p.source => origin(s),
                Ok(_) => format!("yours, another version of its file · {}", p.id),
                Err(_) if folder.has(&p.id) => format!("yours, its id in two files · {}", p.id),
                Err(_) if current(&p.id).is_some() => {
                    format!("ours, as it was when saved · {}", p.id)
                }
                Err(_) => format!("not on this machine, kept in this stain · {}", p.id),
            };
            (line, false, false, false)
        }
    };
    Some(Held {
        from,
        code,
        unsaved,
        revert,
        file,
    })
}

/// Whether `n`'s code is in its stain alone.
pub fn unsaved(n: &Node) -> bool {
    match &n.occupant {
        Occupant::Custom(_) => true,
        Occupant::Pinned(p) => p.edited.is_some(),
        _ => false,
    }
}

/// `version` as whole numbers, the missing ones 0.
fn numbers(version: &str) -> Vec<u64> {
    let mut out: Vec<u64> = version.split('.').map(|n| n.parse().unwrap_or(0)).collect();
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// The update `u` in words, `of` whose file it is.
pub fn update_line(u: &Update, of: &str) -> String {
    let which = match (&u.saved_version, &u.version) {
        (Some(a), Some(b)) if numbers(b) > numbers(a) => {
            format!("{of} is version {b}, newer than this node's version {a}")
        }
        (Some(a), Some(b)) if numbers(b) < numbers(a) => {
            format!("{of} is version {b}, older than this node's version {a}")
        }
        (Some(a), Some(_)) => format!("{of} has changed, though both are version {a}"),
        (None, Some(b)) => format!("{of} is version {b}; this node's says no version"),
        (Some(a), None) => format!("{of} says no version; this node's is version {a}"),
        (None, None) => format!("{of} has changed; neither says a version"),
    };
    let what = if u.code_changed {
        "Its code or its defaults differ, so the picture can change."
    } else {
        "Only its labels differ, so the picture stays the same."
    };
    format!("{which}. {what}")
}

/// What your shaders' controls ask of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    /// Read the folder again.
    Read,
    /// Use this file for this id.
    Choose { id: String, file: String },
    /// Update the node to the file it is offered.
    Update(NodeId),
    /// Keep what the node holds: the offer is not raised again for the same file.
    Keep(NodeId),
    /// Put the node's edit back to the source it was made from.
    Revert(NodeId),
    /// Save the node as a new shader of yours.
    SaveAs {
        node: NodeId,
        id: String,
        name: String,
        author: Option<String>,
    },
    /// Save the node's edit over its file.
    SaveFile(NodeId),
}

/// The node's code being typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub node: NodeId,
    /// The code it began as: it is shown while the node's is that.
    pub base: String,
    pub text: String,
}

/// Save as a node's fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub node: NodeId,
    /// The code it was opened on: it is shown while the node's is that, or that edited.
    pub base: Option<String>,
    pub name: String,
    pub id: String,
    /// Whether the id has been typed: until then it follows the name.
    pub typed: bool,
    pub author: String,
}

/// What came of an ask, said where it was asked.
#[derive(Clone, Debug, PartialEq)]
pub struct Said {
    /// The node it is about; none, the folder.
    pub node: Option<NodeId>,
    pub text: String,
    pub refused: bool,
}

/// Your shaders' own state in the editor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scratch {
    /// The code being typed, each node's and text's.
    pub drafts: Vec<Draft>,
    /// The save being named.
    pub form: Option<Form>,
    /// The author last saved as.
    pub author: String,
    /// What came of the last ask.
    pub said: Option<Said>,
    /// The values each node's code edits took out with their uniforms, to come back if the uniforms do.
    // The plan is unclear here; chose to keep them in the editor until the node is saved, not in the stain, because an
    // undo already brings the graph's own values back, and a stain saved with them would hold values nothing declares.
    pub remembered: BTreeMap<NodeId, Remembered>,
}

impl Scratch {
    /// Says `text` of `node`.
    pub fn say(&mut self, node: Option<NodeId>, text: impl Into<String>, refused: bool) {
        self.said = Some(Said {
            node,
            text: text.into(),
            refused,
        });
    }

    /// Forgets the values kept for nodes no longer in `g`: a node of another stain may come to have the same id.
    pub fn forget_gone(&mut self, g: &StainGraph) {
        self.remembered.retain(|id, _| g.node(*id).is_some());
    }

    /// The draft of node `node` begun from `base`.
    fn draft(&self, node: NodeId, base: &str) -> Option<&Draft> {
        self.drafts
            .iter()
            .find(|d| d.node == node && d.base == base)
    }

    /// Drops the draft of node `node` begun from `base`.
    fn drop_draft(&mut self, node: NodeId, base: &str) {
        self.drafts.retain(|d| d.node != node || d.base != base);
    }

    /// The form, while it is node `node`'s, holding `held`.
    fn form_of(&self, node: NodeId, held: Option<&Held>) -> Option<&Form> {
        let code = &held?.code;
        let form = self.form.as_ref()?;
        (form.node == node && form.base == *code).then_some(form)
    }
}

/// The id a shader named `name` is first given.
// The plan is unclear here; chose `me/` and the name's letters and digits, because an id needs a first segment of one's
// own and the author's name is not known; it is typed over.
fn id_of(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars().flat_map(char::to_lowercase) {
        match ch {
            'a'..='z' | '0'..='9' | '_' => out.push(ch),
            _ if !out.is_empty() && !out.ends_with('-') => out.push('-'),
            _ => {}
        }
    }
    let slug = out.trim_end_matches('-');
    format!("me/{}", if slug.is_empty() { "shader" } else { slug })
}

/// Node `node`'s edit to `text`: the one `SetField` holding it in the stain, its values following the declaration by
/// name ([`reconcile`]), those an earlier edit took out back where it declares them again; and what became of the
/// values. Or why not.
// The plan is unclear here; chose an Apply, one undoable edit, because compiling as it is typed, with the last valid
// code kept, is the in-app code editor's own task.
pub fn applied(
    g: &StainGraph,
    node: NodeId,
    text: &str,
    remembered: &Remembered,
) -> Result<(SetField, Reconciled), String> {
    let why = |e: engine::contract::render_state::GraphError| e.to_string();
    let n = g.node(node).ok_or_else(|| format!("no node {}", node.0))?;
    let occupant = folder::edited(g, node, text).map_err(why)?;
    let old = g.uniforms(node).map_err(why)?;
    let new = custom::uniforms(n.kind, &occupant).map_err(why)?;
    let carried = reconcile(&old, &n.params, remembered, &new);
    let mut next = g.clone();
    next.set_occupant_with(node, occupant, &carried.values)
        .map_err(why)?;
    if next == *g {
        return Err(Refusal::NoChange.to_string());
    }
    let edit = SetField {
        edit: Edit::Render(RenderField::StainGraph(next)),
        no_history: false,
    };
    Ok((edit, carried))
}

/// What became of the values `before` held, in words: one line, empty when nothing did.
pub fn carried_line(r: &Reconciled, before: &Remembered) -> String {
    let mut out: Vec<String> = r
        .notes
        .iter()
        .map(|note| match note {
            Note::Clamped { name, was, now } => {
                format!("`{name}` {was:?} is outside its new range, so it is {now:?}")
            }
            Note::Reset { name, was } => {
                format!("`{name}` {was:?} does not fit its new type, so it is at its default")
            }
            Note::Restored { name } => format!("`{name}` has its value back"),
        })
        .collect();
    for name in r.remembered.keys().filter(|k| !before.contains_key(*k)) {
        if !r.values.contains_key(name)
            && !r
                .notes
                .iter()
                .any(|n| matches!(n, Note::Reset { name: m, .. } if m == name))
        {
            out.push(format!(
                "`{name}` is no longer declared; its value is kept until the node is saved"
            ));
        }
    }
    out.join(" · ")
}

/// Control `id`, clicked or Enter on it; whether it is one of your shaders'.
pub fn activate(
    id: &str,
    s: &mut super::Scratch,
    g: &StainGraph,
    content: Option<&Content>,
    out: &mut Out,
) -> bool {
    let shelf = content.map(|c| &c.yours);
    let node = s.selected();
    let of = |prefix: &str| id.strip_prefix(prefix)?.parse::<usize>().ok();
    let asked = match id {
        SHELF => {
            s.canvas.selection = BTreeSet::new();
            s.yours.said = None;
            return true;
        }
        READ => Some(Ask::Read),
        REVERT => node.map(Ask::Revert),
        SAVE_FILE => node.map(Ask::SaveFile),
        SAVE_AS => {
            let name = node.and_then(|n| content?.described.get(&n).map(|d| d.name.clone()));
            let name = name.unwrap_or_default();
            s.yours.said = None;
            let base = node.and_then(|n| shelf?.held(n)?.code.clone());
            s.yours.form = node.map(|node| Form {
                node,
                base,
                id: id_of(&name),
                name,
                typed: false,
                author: s.yours.author.clone(),
            });
            return true;
        }
        CANCEL => {
            s.yours.form = None;
            return true;
        }
        SAVE => s.yours.form.clone().map(|f| {
            let author = f.author.trim().to_owned();
            s.yours.author.clone_from(&author);
            Ask::SaveAs {
                node: f.node,
                id: f.id.trim().to_owned(),
                name: f.name.trim().to_owned(),
                author: Some(author).filter(|a| !a.is_empty()),
            }
        }),
        APPLY | DISCARD => {
            let code = node.and_then(|n| Some((n, shelf?.held(n)?.code.clone()?)));
            let draft = code.and_then(|(n, base)| s.yours.draft(n, &base).cloned());
            let Some(Draft { node, base, text }) = draft else {
                return true;
            };
            if id == DISCARD {
                s.yours.drop_draft(node, &base);
                s.yours.said = None;
                return true;
            }
            let before = s.yours.remembered.get(&node).cloned().unwrap_or_default();
            match applied(g, node, &text, &before) {
                Ok((edit, carried)) => {
                    s.yours.drop_draft(node, &base);
                    if let Some(form) = s.yours.form.as_mut().filter(|f| f.node == node) {
                        if form.base.as_ref() == Some(&base) {
                            form.base = Some(text);
                        }
                    }
                    let line = carried_line(&carried, &before);
                    s.yours.said = None;
                    if !line.is_empty() {
                        s.yours.say(Some(node), line, false);
                    }
                    s.yours.remembered.insert(node, carried.remembered);
                    out.edits.push(edit);
                }
                Err(why) => s.yours.say(Some(node), why, true),
            }
            return true;
        }
        _ => {
            let at = |k: usize| u32::try_from(k).ok().map(NodeId);
            if let Some(k) = of(UPDATE) {
                at(k).map(Ask::Update)
            } else if let Some(k) = of(KEEP) {
                at(k).map(Ask::Keep)
            } else if let Some(k) = of(CHOOSE) {
                shelf
                    .and_then(|shelf| shelf.files().nth(k))
                    .map(|(_, id, file)| Ask::Choose {
                        id: id.to_owned(),
                        file: file.to_owned(),
                    })
            } else {
                return false;
            }
        }
    };
    if asked.is_some() {
        s.yours.said = None;
    }
    if let Some(Ask::SaveAs { node, .. } | Ask::SaveFile(node)) = &asked {
        s.yours.remembered.remove(node);
    }
    out.asks.extend(asked);
    true
}

/// The scopes of the update node `node` is offered.
pub fn offer_scopes(shelf: &Shelf, node: NodeId) -> Vec<Scope> {
    shelf.offered(node).map_or_else(Vec::new, offer_controls)
}

fn offer_controls(o: &Offer) -> Vec<Scope> {
    let mut out = vec![Scope::control(update_id(o.node), "Update")];
    if !o.kept {
        out.push(Scope::control(keep_id(o.node), "Keep this one"));
    }
    out
}

/// The scopes of the panel.
pub fn panel_scopes(shelf: &Shelf) -> Vec<Scope> {
    if shelf.folder.is_none() {
        return Vec::new();
    }
    let mut out = vec![Scope::control(READ, "Read again")];
    out.extend(shelf.open().flat_map(offer_controls));
    let files = shelf.files().enumerate();
    out.extend(files.map(|(k, (_, _, file))| Scope::control(choose_id(k), intern(file))));
    out
}

/// The scopes of node `node`'s shader, from Revert on.
pub fn shader_scopes(shelf: &Shelf, s: &Scratch, node: NodeId) -> Vec<Scope> {
    let held = shelf.held(node);
    let control = |on: bool, id, label| match on {
        true => Scope::control(id, label),
        false => Scope::disabled(id, label),
    };
    let on = can_save(shelf) && held.is_some();
    let mut out = vec![
        control(held.is_some_and(|h| h.revert), REVERT, "Revert"),
        control(on, SAVE_AS, "Save as a node…"),
    ];
    if held.is_some_and(|h| h.file) {
        out.push(control(on, SAVE_FILE, "Save to your file"));
    }
    if s.form_of(node, held).is_some() {
        out.extend([
            Scope::text(NAME, "name"),
            Scope::text(ID, "id"),
            Scope::text(AUTHOR, "author"),
            control(on, SAVE, "Save"),
            Scope::control(CANCEL, "Cancel"),
        ]);
    }
    out
}

fn can_save(shelf: &Shelf) -> bool {
    shelf.folder.is_some() && !shelf.saving
}

/// The one node of `selection` and its code, where it has any.
pub fn editable<'a>(
    content: Option<&'a Content>,
    selection: &BTreeSet<NodeId>,
) -> Option<(NodeId, &'a str)> {
    let node = *selection.iter().next().filter(|_| selection.len() == 1)?;
    let code = content?.yours.held(node)?.code.as_deref()?;
    Some((node, code))
}

/// The scopes of the code as it is typed.
pub fn code_scopes(content: &Content, s: &super::Scratch) -> impl Iterator<Item = Scope> {
    let code = editable(Some(content), &s.canvas.selection);
    let typed = code.is_some_and(|(node, base)| s.yours.draft(node, base).is_some());
    let buttons = [
        Scope::control(APPLY, "Apply"),
        Scope::control(DISCARD, "Discard"),
    ];
    code.map(|_| Scope::lines(CODE, "the node's WGSL"))
        .into_iter()
        .chain(buttons.into_iter().filter(move |_| typed))
}

/// Draws the library's line on the folder: how many shaders, updates, ids to choose and files refused.
pub fn line(ui: &mut Ui, shelf: &Shelf, keys: &mut Keys<'_>, clicked: &mut Vec<ScopeId>) {
    let visuals = ui.visuals().clone();
    let mut text = format!("Your shaders · {}", shelf.shaders);
    let mut wants = Vec::new();
    let (open, twice, bad) = (
        shelf.open().count(),
        shelf.unchosen(),
        shelf.refused.len() + usize::from(shelf.failed.is_some()),
    );
    if open > 0 {
        wants.push(count(open, "update"));
    }
    if twice > 0 {
        wants.push(format!("{twice} to choose"));
    }
    if bad > 0 {
        wants.push(format!("{bad} refused"));
    }
    let colour = match wants.is_empty() {
        true => visuals.weak_text_color(),
        false => visuals.warn_fg_color,
    };
    for want in wants {
        text.push_str(&format!(" · {want}"));
    }
    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
        let button = ui
            .add(
                Button::new(RichText::new(&text).color(colour))
                    .frame(false)
                    .truncate(),
            )
            .on_hover_text(&text);
        if button.clicked() {
            clicked.push(SHELF);
        }
        keys.place(SHELF, &button);
    });
}

fn button(
    ui: &mut Ui,
    (id, label): (ScopeId, &str),
    on: bool,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) -> egui::Response {
    let button = ui.add_enabled(on, Button::new(label));
    if button.clicked() {
        clicked.push(id);
    }
    keys.place(id, &button);
    button
}

/// Draws what was said of `node`, or of the folder.
fn said(ui: &mut Ui, s: &Scratch, node: Option<NodeId>) {
    if let Some(said) = s.said.as_ref().filter(|said| said.node == node) {
        let visuals = ui.visuals();
        let colour = match said.refused {
            true => visuals.error_fg_color,
            false => visuals.weak_text_color(),
        };
        ui.label(RichText::new(&said.text).color(colour));
    }
}

/// An offer's two buttons.
fn buttons(ui: &mut Ui, o: &Offer, keys: &mut Keys<'_>, clicked: &mut Vec<ScopeId>) {
    ui.horizontal(|ui| {
        button(ui, (update_id(o.node), "Update"), true, keys, clicked);
        if !o.kept {
            let keep = (keep_id(o.node), "Keep this one");
            button(ui, keep, true, keys, clicked);
        }
    });
}

/// Draws the update node `node` is offered, if any.
pub fn offer(
    ui: &mut Ui,
    shelf: &Shelf,
    node: NodeId,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    let Some(o) = shelf.offered(node) else {
        return;
    };
    let line = RichText::new(&o.line);
    ui.label(match o.kept {
        true => line.weak(),
        false => line.color(ui.visuals().warn_fg_color),
    });
    buttons(ui, o, keys, clicked);
    ui.add_space(6.0);
}

/// Draws the panel: the folder, the updates offered, the ids given twice and the files refused.
pub fn panel(
    ui: &mut Ui,
    shelf: &Shelf,
    s: &Scratch,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Your shaders").heading());
    let Some(path) = &shelf.folder else {
        ui.label(RichText::new("there is no folder for them here").weak());
        return;
    };
    let status = match (shelf.reading, shelf.shaders) {
        (true, _) => "reading the folder…".to_owned(),
        (false, 0) => "none yet".to_owned(),
        (false, n) => count(n, "shader"),
    };
    ui.horizontal(|ui| {
        button(ui, (READ, "Read again"), true, keys, clicked);
        ui.label(RichText::new(status).weak());
    });
    ui.label(RichText::new(path).monospace().weak());
    if shelf.shaders == 0 && !shelf.reading {
        ui.label(RichText::new("Save as a node…, on a node, makes one.").weak());
    }
    if shelf.shaders > catalogue::LISTED {
        let more = shelf.shaders - catalogue::LISTED;
        ui.label(RichText::new(format!("{more} more are not listed in the library")).weak());
    }
    if let Some(why) = &shelf.failed {
        ui.label(RichText::new(why).color(ui.visuals().error_fg_color));
    }
    said(ui, s, None);
    for o in shelf.open() {
        ui.add_space(8.0);
        ui.label(RichText::new(format!("Update · {}", o.title)).strong());
        ui.label(&o.line);
        buttons(ui, o, keys, clicked);
    }
    let mut files = shelf.files().enumerate().peekable();
    for (i, d) in shelf.duplicates.iter().enumerate().take(SHOWN) {
        ui.add_space(8.0);
        ui.label(RichText::new(format!("One id, {} files · {}", d.files.len(), d.id)).strong());
        ui.label(match d.chosen {
            None => "None of them is used until you choose one.",
            Some(_) => "The one you chose is used; the others stay as they are.",
        });
        while let Some((k, (_, _, file))) = files.next_if(|(_, (of, ..))| *of == i) {
            let on = d.chosen.as_deref() == Some(file);
            let choice = ui.add(Button::selectable(on, file).wrap());
            if choice.clicked() {
                clicked.push(choose_id(k));
            }
            keys.place(choose_id(k), &choice);
        }
    }
    more(ui, shelf.duplicates.len(), "id");
    if !shelf.refused.is_empty() {
        ui.add_space(8.0);
        let heading = format!("{} not used", count(shelf.refused.len(), "file"));
        ui.label(RichText::new(heading).strong());
    }
    for p in shelf.refused.iter().take(SHOWN) {
        // Most reasons begin with the file.
        if !p.why.starts_with(&p.file) {
            let file = RichText::new(&p.file).monospace();
            ui.label(file.color(ui.visuals().warn_fg_color));
        }
        ui.label(&p.why);
    }
    more(ui, shelf.refused.len(), "file");
}

/// How many of `n` the panel leaves out.
fn more(ui: &mut Ui, n: usize, what: &str) {
    if n > SHOWN {
        ui.label(RichText::new(format!("and {}", count(n - SHOWN, what))).weak());
    }
}

/// Draws node `node`'s saves and where its shader comes from.
pub fn shader(
    ui: &mut Ui,
    shelf: &Shelf,
    s: &mut Scratch,
    node: NodeId,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    let held = shelf.held(node);
    let on = can_save(shelf) && held.is_some();
    let why = match (&shelf.folder, shelf.saving) {
        (None, _) => "there is no folder for your shaders here",
        (_, true) => "saving…",
        _ => "this node holds no shader",
    };
    ui.horizontal(|ui| {
        let save = (SAVE_AS, "Save as a node…");
        button(ui, save, on, keys, clicked).on_disabled_hover_text(why);
        if held.is_some_and(|h| h.file) {
            let save = (SAVE_FILE, "Save to your file");
            button(ui, save, on, keys, clicked).on_disabled_hover_text(why);
        }
    });
    if let Some(held) = held {
        ui.label(RichText::new(&held.from).weak());
        let typed = held.code.as_ref().and_then(|base| s.draft(node, base));
        if typed.is_some() {
            let note = "code typed in Node WGSL is not applied, so not saved";
            ui.label(RichText::new(note).weak());
        }
    }
    let follows = s.form_of(node, held).is_some();
    if let Some(f) = s.form.as_mut().filter(|_| follows) {
        ui.add_space(4.0);
        egui::Grid::new("stain_yours_form")
            .num_columns(2)
            .show(ui, |ui| {
                let mut field = |ui: &mut Ui, id, label: &str, text: &mut String| {
                    ui.label(label);
                    let width = ui.available_width().max(160.0);
                    let edit = ui.add(TextEdit::singleline(text).desired_width(width));
                    keys.place(id, &edit);
                    ui.end_row();
                    edit.changed()
                };
                if field(ui, NAME, "name", &mut f.name) && !f.typed {
                    f.id = id_of(&f.name);
                }
                f.typed |= field(ui, ID, "id", &mut f.id);
                field(ui, AUTHOR, "author", &mut f.author);
            });
        ui.horizontal(|ui| {
            button(ui, (SAVE, "Save"), on, keys, clicked);
            button(ui, (CANCEL, "Cancel"), true, keys, clicked);
        });
    }
    said(ui, s, Some(node));
}

/// Draws node `node`'s code, `base`, to be typed in, in `area`: its buttons on the toggle's row, the text under it.
pub fn code(
    ui: &mut Ui,
    area: Rect,
    (node, base): (NodeId, &str),
    s: &mut Scratch,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    let bar_h = super::canvas::TOGGLE_H;
    let bar = Rect::from_min_size(area.min + vec2(12.0, 0.0), vec2(area.width() / 2.0, bar_h));
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(bar)
            .layout(egui::Layout::left_to_right(Align::Center)),
    );
    if s.draft(node, base).is_some() {
        button(&mut row, (APPLY, "Apply"), true, keys, clicked);
        button(&mut row, (DISCARD, "Discard"), true, keys, clicked);
        row.label(RichText::new("typed, not applied").weak());
    } else {
        row.label(RichText::new("click the code to edit it").weak());
    }
    let mut top = area.min.y + bar_h + 8.0;
    let refused = s.said.as_ref().is_some_and(|said| said.refused);
    if refused && s.draft(node, base).is_some() {
        let mut strip = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(
            pos2(area.min.x + 12.0, top),
            pos2(area.max.x - 12.0, area.max.y),
        )));
        said(&mut strip, s, Some(node));
        top = strip.min_rect().max.y + 8.0;
    }
    let below = Rect::from_min_max(pos2(area.min.x, top), area.max).shrink(12.0);
    let mut pane = ui.new_child(UiBuilder::new().max_rect(below));
    let mut text = s
        .draft(node, base)
        .map_or_else(|| base.to_owned(), |d| d.text.clone());
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _: f32| {
        let job = code_view::job(ui, text.as_str());
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let edit = egui::ScrollArea::both()
        .id_salt(("stain_yours_code", node))
        .auto_shrink(false)
        .show(&mut pane, |ui| {
            ui.add(
                TextEdit::multiline(&mut text)
                    .id_salt(("stain_yours_code", node))
                    .code_editor()
                    .frame(egui::Frame::NONE.inner_margin(6.0))
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            )
        })
        .inner;
    keys.place(CODE, &edit);
    if edit.changed() {
        s.drop_draft(node, base);
        if text != base {
            s.drafts.push(Draft {
                node,
                base: base.to_owned(),
                text,
            });
        }
    }
}
