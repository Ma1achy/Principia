//! Stain mode, the plain node-graph editor (render_gui_spec Part II §1, §7, §8; GUI_DESIGN_NOTES § "02 Stain";
//! 02_stain.png; R-390, ORDER item 6): the library drawer, collapsible, left (`library`); the graph canvas centre,
//! with its `Graph · Pipeline WGSL · Node WGSL` toggle (`canvas`); the preview and the node inspector right
//! (`preview`); and the assembled code (`code_view`) and the Problems pane (`problems`) under the canvas. The graph is
//! the snapshot's stain graph (gui_state_contract §2, §5), and every edit of it is one `SetField` of the whole graph,
//! undoable: the inspector's. The canvas's places, view and selection, the drawer's state, the toggle and the code
//! being typed are the editor's own. Your shaders' folder is the app's (`crate::shaders`); what is shown of it is in
//! [`yours`].
//!
//! **Keys** (render_gui_spec §G3; R-409; applied per R-369, review 5434766412 on PR #157): in Stain mode the big scopes
//! in Tab order are the top bar, the library drawer, the graph canvas, the preview and node inspector, the code and
//! Problems pane, and the footer last (R-405). Enter goes into one, landing on its first element in navigation mode,
//! and the arrows move between its elements: the drawer's collapse, filter, groups and rows; the canvas's toggle and
//! its nodes, in their order on the canvas; the inspector's fields for the selected node; and auto-recompile and
//! Recompile. Enter on a button, a row or a node acts as its click (a node's selects it, opening its inspector), on
//! the filter starts typing; on the inspector's controls it acts as [`controls`] says.
//!
//! Everything shown of a node is its description's (`engine::contract::stain::describe`), read from its source: its
//! title, its glyph, its "maps … → …" line, its controls, and the library's rows and headings. Nothing here asks where a
//! node's source comes from. What the engine says cannot change the picture now is dimmed ([`standing`]).

pub mod canvas;
pub mod catalogue;
pub mod code_view;
pub mod controls;
pub mod library;
pub mod preview;
pub mod problems;
pub mod standing;
pub mod yours;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, LazyLock, Mutex};

use eframe::egui::{pos2, Rect, Ui, UiBuilder};
use engine::contract::canvas::Canvas;
use engine::contract::render_state::{Node, NodeId, NodeKind, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::node_source;
use engine::contract::stain::custom::Status;
use engine::contract::stain::describe::{describe_node, from_source, Description, Shown, Standing};

use crate::keyboard::scopes::{Adjust, Scope, ScopeId, ScopeTree};
use crate::layout::{Layout, ARTBOARD};
use crate::windows::Keys;

pub use catalogue::Choice;

/// The Stain editor's content the engine side serves (`crate::side::EngineSide::stain_content`).
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    /// The library's groups, in order.
    pub library: Arc<[Group]>,
    /// How many modes the catalogue holds.
    pub modes: u32,
    /// The library row of the stain loaded, by its place in the library.
    pub loaded: Option<usize>,
    /// The assembled code.
    pub code: String,
    /// The whole pipeline's code (Pipeline WGSL).
    pub pipeline: String,
    /// Each node's description.
    pub described: BTreeMap<NodeId, Description>,
    /// What the engine says each node's controls can do now, of the nodes it has been asked of and has answered for.
    pub standing: BTreeMap<NodeId, Arc<[Standing]>>,
    /// Each node's code (Node WGSL), its own source: never the compile cache's, which a label edit does not reach.
    pub node_code: BTreeMap<NodeId, String>,
    /// The status beside each node whose code did not compile: it is drawn at its last valid code.
    pub status: BTreeMap<NodeId, Status>,
    /// The compile status line.
    pub compiled: String,
    /// Whether the graph is still compiling: the code, and the picture drawn, are the last valid ones.
    pub compiling: bool,
    /// The problems, one line each.
    pub problems: Vec<String>,
    /// The notes, one line each.
    pub notes: Vec<String>,
    /// The shaders the inspector offers, every kind's.
    pub choices: Arc<[Choice]>,
    /// The whole graph each library row loads (§11), by its place in the library; none for a row only selected.
    pub graphs: Arc<[Option<StainGraph>]>,
    /// Your shaders' folder and what the nodes hold of it.
    pub yours: yours::Shelf,
}

/// One group of the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// Its heading.
    pub title: &'static str,
    /// Its rows.
    pub rows: Vec<Row>,
}

/// One row of the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Its place in the whole library, which a shader joining the library does not move.
    pub place: usize,
    /// Its name.
    pub name: &'static str,
    /// Where it comes from, as its tooltip says.
    pub from: String,
}

/// What the canvas shows: the toggle's three views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    /// The graph.
    #[default]
    Graph,
    /// The whole pipeline's WGSL.
    Pipeline,
    /// The selected node's WGSL.
    Node,
}

/// The big scopes, in Tab order after the top bar.
pub const LIBRARY: ScopeId = "stain_library";
/// See [`LIBRARY`].
pub const CANVAS: ScopeId = "stain_canvas";
/// See [`LIBRARY`].
pub const INSPECTOR: ScopeId = "stain_inspector";
/// See [`LIBRARY`].
pub const CODE: ScopeId = "stain_code";

/// The toggle's buttons, each with its view and its label.
pub const VIEWS: [(ScopeId, View, &str); 3] = [
    ("stain_view_graph", View::Graph, "Graph"),
    ("stain_view_pipeline", View::Pipeline, "Pipeline WGSL"),
    ("stain_view_node", View::Node, "Node WGSL"),
];

/// The drawer's collapse.
pub const COLLAPSE: ScopeId = "stain_lib_collapse";
/// The drawer's filter.
pub const FILTER: ScopeId = "stain_lib_filter";
/// The drawer's buttons, drawn disabled: their actions are TASK-M8-19's.
pub const BUTTONS: [(ScopeId, &str); 4] = [
    ("stain_lib_new", "New"),
    ("stain_lib_rename", "Rename"),
    ("stain_lib_import", "Import"),
    ("stain_lib_export", "Export"),
];
/// The inspector's "Edit this node's WGSL": the Node WGSL view.
pub const EDIT_WGSL: ScopeId = "stain_insp_edit_wgsl";
/// The inspector's Duplicate: a copy of the node whose code is its own source, written out, to edit.
pub const DUPLICATE: ScopeId = "stain_insp_duplicate";
/// The inspector's Revert: an edit's, back to the source it was made from.
pub const REVERT: ScopeId = "stain_insp_revert";
/// The Problems pane's auto-recompile.
pub const AUTO_RECOMPILE: ScopeId = "stain_auto_recompile";
/// The Problems pane's Recompile.
pub const RECOMPILE: ScopeId = "stain_recompile";

/// The nodes' scopes, by node id: a node past them is drawn but not reached by the keyboard.
pub const NODE_IDS: [ScopeId; 32] = [
    "stain_node_0",
    "stain_node_1",
    "stain_node_2",
    "stain_node_3",
    "stain_node_4",
    "stain_node_5",
    "stain_node_6",
    "stain_node_7",
    "stain_node_8",
    "stain_node_9",
    "stain_node_10",
    "stain_node_11",
    "stain_node_12",
    "stain_node_13",
    "stain_node_14",
    "stain_node_15",
    "stain_node_16",
    "stain_node_17",
    "stain_node_18",
    "stain_node_19",
    "stain_node_20",
    "stain_node_21",
    "stain_node_22",
    "stain_node_23",
    "stain_node_24",
    "stain_node_25",
    "stain_node_26",
    "stain_node_27",
    "stain_node_28",
    "stain_node_29",
    "stain_node_30",
    "stain_node_31",
];
/// `count` scopes, `<stem>0`, `<stem>1`, …
fn ids(stem: &str, count: usize) -> Vec<ScopeId> {
    (0..count).map(|k| intern(&format!("{stem}{k}"))).collect()
}

/// The library's rows' scopes, by their place in the library: more than the registry holds, so every row is reached.
pub static ROW_IDS: LazyLock<Vec<ScopeId>> = LazyLock::new(|| ids("stain_lib_row_", 512));
/// The library's groups' headings' scopes.
pub static GROUP_IDS: LazyLock<Vec<ScopeId>> = LazyLock::new(|| ids("stain_lib_group_", 320));
/// The inspector's choices' scopes: the shaders a node can be given.
pub static CHOICE_IDS: LazyLock<Vec<ScopeId>> = LazyLock::new(|| ids("stain_insp_choice_", 512));
/// `s` as a `'static` label: a scope's label is `'static`, and the labels the graph gives (a node's title, a param's
/// name) are few, each kept once for the session.
pub fn intern(s: &str) -> &'static str {
    static SEEN: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(kept) = seen.get(s) {
        return kept;
    }
    let kept: &'static str = Box::leak(s.to_owned().into_boxed_str());
    seen.insert(kept);
    kept
}

/// 02_stain.png's edges, in its pixels: the drawer's right edge, the canvas's right edge, the code pane's top and the
/// code | Problems split.
const LIBRARY_RIGHT: f32 = 411.0;
const CANVAS_RIGHT: f32 = 1643.0;
const CODE_TOP: f32 = 959.0;
const CODE_SPLIT: f32 = 1066.0;
/// The collapsed drawer's width, in points (a look choice, R-390).
pub const COLLAPSED_W: f32 = 28.0;

/// The Stain page's regions, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Regions {
    /// The library drawer.
    pub library: Rect,
    /// The graph canvas.
    pub canvas: Rect,
    /// The preview and the node inspector.
    pub right: Rect,
    /// The assembled code.
    pub code: Rect,
    /// The Problems pane.
    pub problems: Rect,
}

impl Regions {
    /// The regions of `layout`'s page, in 02_stain.png's proportions of the window, each edge on a whole pixel; the
    /// drawer `collapsed` to its strip, the canvas and the code taking its width.
    pub fn new(layout: &Layout, pixels_per_point: f32, collapsed: bool) -> Self {
        let (x0, x1) = (layout.top_bar.min.x, layout.top_bar.max.x);
        let (y0, y1) = (layout.top_bar.min.y, layout.footer.max.y);
        let px = |edge: f32, size: f32, min: f32, extent: f32| {
            min + (edge / size * extent * pixels_per_point).round() / pixels_per_point
        };
        let x = |e| px(e, ARTBOARD[0], x0, x1 - x0);
        let y = |e| px(e, ARTBOARD[1], y0, y1 - y0);
        let (top, bottom) = (layout.page.min.y, layout.page.max.y);
        let left = if collapsed {
            x0 + (COLLAPSED_W * pixels_per_point).round() / pixels_per_point
        } else {
            x(LIBRARY_RIGHT)
        };
        let (right, code_top, split) = (x(CANVAS_RIGHT), y(CODE_TOP), x(CODE_SPLIT));
        let split = split.max(left + (right - left) / 2.0);
        let rect = |a: f32, b: f32, c: f32, d: f32| Rect::from_min_max(pos2(a, b), pos2(c, d));
        Self {
            library: rect(x0, top, left, bottom),
            canvas: rect(left, top, right, code_top),
            right: rect(right, top, x1, bottom),
            code: rect(left, code_top, split, bottom),
            problems: rect(split, code_top, right, bottom),
        }
    }
}

/// The editor's own state: the GUI's, never the engine's, never undoable.
#[derive(Clone, Debug)]
pub struct Scratch {
    /// The canvas: its view, the nodes' places, the selection.
    pub canvas: canvas::Canvas,
    /// What the canvas shows.
    pub view: View,
    /// The drawer.
    pub library: library::State,
    /// Whether an edit recompiles at once (the Problems pane's checkbox).
    pub auto_recompile: bool,
    /// The preview's rect as last drawn.
    pub preview: Option<Rect>,
    /// The inspector's sections closed, by their titles.
    pub folded: BTreeSet<String>,
    /// The colours of the inspector the mouse opened, each its node and its scope.
    pub open: BTreeSet<(NodeId, ScopeId)>,
    /// Your shaders': the code being typed, the save being named and what came of the last ask.
    pub yours: yours::Scratch,
    /// The uniforms no edit is taken of.
    #[cfg(test)]
    pub(crate) locked: BTreeSet<String>,
}

impl Default for Scratch {
    fn default() -> Self {
        Self {
            canvas: canvas::Canvas::default(),
            view: View::Graph,
            library: library::State::default(),
            auto_recompile: true,
            preview: None,
            folded: BTreeSet::new(),
            open: BTreeSet::new(),
            yours: yours::Scratch::default(),
            #[cfg(test)]
            locked: BTreeSet::new(),
        }
    }
}

impl Scratch {
    /// The one selected node, when exactly one is.
    pub fn selected(&self) -> Option<NodeId> {
        let mut it = self.canvas.selection.iter();
        match (it.next(), it.next()) {
            (Some(id), None) => Some(*id),
            _ => None,
        }
    }

    /// The `SetField` by which node `node`'s `uniform` is `value`, as every control edits it.
    fn set_param(
        &self,
        g: &StainGraph,
        node: NodeId,
        uniform: &str,
        value: Vec<f64>,
    ) -> Option<SetField> {
        #[cfg(test)]
        if self.locked.contains(uniform) {
            return None;
        }
        edited(g, |g| g.set_param(node, uniform, value))
    }
}

/// Joins Stain mode's big scopes after the top bar: the drawer, the canvas, the inspector and the code pane, with the
/// code pane's two controls; the rest join as they are drawn ([`join`]).
pub fn register(tree: &mut ScopeTree) {
    tree.register(None, Scope::group(LIBRARY, "Library drawer"));
    tree.register(None, Scope::group(CANVAS, "Graph canvas"));
    tree.register(None, Scope::group(INSPECTOR, "Preview & node inspector"));
    tree.register(None, Scope::group(CODE, "Code & Problems"));
    tree.register(Some(CODE), Scope::control(AUTO_RECOMPILE, "auto-recompile"));
    tree.register(Some(CODE), Scope::control(RECOMPILE, "Recompile"));
}

/// The nodes of `g` in their order on the canvas: by column, then down it.
pub fn node_order(g: &StainGraph, canvas: &canvas::Canvas) -> Vec<NodeId> {
    let mut ids: Vec<(NodeId, eframe::egui::Pos2)> = g
        .nodes()
        .filter_map(|n| canvas.positions.get(&n.id).map(|p| (n.id, *p)))
        .collect();
    ids.sort_by(|a, b| {
        (a.1.x, a.1.y, a.0)
            .partial_cmp(&(b.1.x, b.1.y, b.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ids.into_iter().map(|(id, _)| id).collect()
}

/// Every node of `g` described, and why one could not be, for the Problems pane.
pub fn described(g: &StainGraph) -> (BTreeMap<NodeId, Description>, Vec<String>) {
    let mut problems = Vec::new();
    let nodes = g
        .nodes()
        .map(|n| {
            let d = describe_node(n).unwrap_or_else(|why| {
                problems.push(format!("node {}: {why}", n.id.0));
                plain(n.kind)
            });
            (n.id, d)
        })
        .collect();
    (nodes, problems)
}

/// Each node's own source, as written.
pub fn node_code(g: &StainGraph) -> BTreeMap<NodeId, String> {
    g.nodes()
        .map(|n| {
            let text = match node_source(n.kind, &n.occupant) {
                Ok(Some(text)) => text,
                Ok(None) => "// None: the identity\n".to_owned(),
                Err(why) => format!("// {why}\n"),
            };
            (n.id, text)
        })
        .collect()
}

/// The description of a node of `kind` with nothing in it.
fn plain(kind: NodeKind) -> Description {
    from_source(kind, None, &BTreeMap::new()).unwrap_or_else(|why| Description {
        name: why.to_string(),
        group: String::new(),
        about: String::new(),
        maps: String::new(),
        glyph: Shown::None,
        controls: Vec::new(),
        ports: Vec::new(),
    })
}

/// Node `n`'s description in `content`; of nothing where it holds none.
pub fn description<'a>(content: Option<&'a Content>, n: &Node) -> Cow<'a, Description> {
    match content.and_then(|c| c.described.get(&n.id)) {
        Some(d) => Cow::Borrowed(d),
        None => Cow::Owned(plain(n.kind)),
    }
}

/// The inspector's choices for node `n`: the shaders of its kind.
pub fn choices<'a>(n: &Node, content: &'a Content) -> Vec<&'a Choice> {
    content
        .choices
        .iter()
        .filter(|c| c.slot == n.kind)
        .collect()
}

/// Whether node `n` is `choice`: its source is.
pub fn chosen(n: &Node, choice: &Choice, content: &Content) -> bool {
    content.node_code.get(&n.id) == Some(&choice.source)
}

/// Whether node `n` can be duplicated: it holds code, and is not one of the graph's fixed singletons.
pub fn duplicable(n: &Node, content: &Content) -> bool {
    let code = content.yours.held(n.id).is_some_and(|h| h.code.is_some());
    code && !matches!(n.kind, NodeKind::Combiner | NodeKind::Out)
}

/// The sections of the selected node's controls.
fn selected_sections(s: &Scratch, content: Option<&Content>) -> Vec<controls::Section> {
    s.selected()
        .and_then(|id| content?.described.get(&id))
        .map(controls::sections)
        .unwrap_or_default()
}

/// The scopes below each of the page's big ones, for the graph, the content and the editor's state, each with its
/// parent.
fn wanted(g: &StainGraph, s: &Scratch, content: Option<&Content>) -> Vec<(ScopeId, Scope)> {
    let mut out = vec![(
        LIBRARY,
        Scope::control(
            COLLAPSE,
            if s.library.collapsed {
                "expand"
            } else {
                "collapse"
            },
        ),
    )];
    if !s.library.collapsed {
        out.push((LIBRARY, Scope::text(FILTER, "filter")));
        if let Some(c) = content {
            for item in library::visible(c, &s.library) {
                let scope = match item {
                    library::Item::Group(g, title) => Scope::control(GROUP_IDS[g], title),
                    library::Item::Row(k, row) => Scope::control(ROW_IDS[k], row),
                };
                out.push((LIBRARY, scope));
            }
            out.push((LIBRARY, Scope::control(yours::SHELF, "Your shaders")));
        }
        out.extend(BUTTONS.map(|(id, label)| (LIBRARY, Scope::disabled(id, label))));
    }
    out.extend(VIEWS.map(|(id, _, label)| (CANVAS, Scope::control(id, label))));
    if s.view == View::Graph {
        for id in node_order(g, &s.canvas) {
            let (Some(scope), Some(n)) = (canvas::node_scope(id), g.node(id)) else {
                continue;
            };
            let title = canvas::title(n.kind, &description(content, n));
            out.push((CANVAS, Scope::control(scope, intern(&title))));
        }
    }
    if let (View::Node, Some(c)) = (s.view, content) {
        out.extend(yours::code_scopes(c, s).map(|scope| (CANVAS, scope)));
    }
    if let (true, Some(c)) = (s.canvas.selection.is_empty(), content) {
        let scopes = yours::panel_scopes(&c.yours);
        out.extend(scopes.into_iter().map(|scope| (INSPECTOR, scope)));
    }
    if let (Some(n), Some(c)) = (s.selected().and_then(|id| g.node(id)), content) {
        let offered = yours::offer_scopes(&c.yours, n.id);
        out.extend(offered.into_iter().map(|scope| (INSPECTOR, scope)));
        for (choice, id) in choices(n, c).into_iter().zip(CHOICE_IDS.iter()) {
            out.push((INSPECTOR, Scope::control(id, choice.name)));
        }
        for section in selected_sections(s, content) {
            out.push((INSPECTOR, section.scope));
            if !s.folded.contains(&section.title) {
                for (_, part) in &section.controls {
                    parts(INSPECTOR, part, &mut out);
                }
            }
        }
        if n.kind != NodeKind::Out {
            out.push((
                INSPECTOR,
                Scope::control(EDIT_WGSL, "Edit this node's WGSL"),
            ));
            let duplicate = match duplicable(n, c) {
                true => Scope::control(DUPLICATE, "Duplicate"),
                false => Scope::disabled(DUPLICATE, "Duplicate"),
            };
            out.push((INSPECTOR, duplicate));
            let shader = yours::shader_scopes(&c.yours, &s.yours, n.id);
            out.extend(shader.into_iter().map(|scope| (INSPECTOR, scope)));
        }
    }
    out
}

/// `part` under `parent`, then its own parts.
fn parts(parent: ScopeId, part: &controls::Part, out: &mut Vec<(ScopeId, Scope)>) {
    out.push((parent, part.scope));
    for p in &part.parts {
        parts(part.scope.id, p, out);
    }
}

/// The scopes `tree` has below `parent`, in the order [`wanted`] lists them.
fn below(tree: &ScopeTree, parent: ScopeId, out: &mut Vec<(ScopeId, Scope)>) {
    for id in tree.children(Some(parent)) {
        out.extend(tree.get(id).map(|scope| (parent, *scope)));
        below(tree, id, out);
    }
}

/// Joins the drawer's, the canvas's and the inspector's elements to `tree` as they stand, again whenever they change.
pub fn join(tree: &mut ScopeTree, g: &StainGraph, s: &Scratch, content: Option<&Content>) {
    let mut now = Vec::new();
    for big in [LIBRARY, CANVAS, INSPECTOR] {
        below(tree, big, &mut now);
    }
    let scopes = wanted(g, s, content);
    if now == scopes {
        return;
    }
    for big in [LIBRARY, CANVAS, INSPECTOR] {
        tree.remove_children(big);
    }
    for (parent, scope) in scopes {
        tree.register(Some(parent), scope);
    }
}

/// What the editor's controls ask of the app.
#[derive(Debug, Default, PartialEq)]
pub struct Out {
    /// The graph's edits, each a `SetField`.
    pub edits: Vec<SetField>,
    /// Recompile.
    pub recompile: bool,
    /// What your shaders' controls ask, which needs the folder.
    pub asks: Vec<yours::Ask>,
    /// Whether an edit replaces the graph by another stain.
    pub loads: bool,
}

/// The `SetField` of the graph `g` after `edit`, or nothing when the graph refuses it.
pub(crate) fn edited(
    g: &StainGraph,
    edit: impl FnOnce(&mut StainGraph) -> Result<(), engine::contract::render_state::GraphError>,
) -> Option<SetField> {
    let mut next = g.clone();
    edit(&mut next).ok()?;
    (next != *g).then_some(SetField {
        edit: Edit::Render(RenderField::StainGraph(next)),
        no_history: false,
    })
}

/// Control `id`, clicked or Enter on it: each acts on its own (the one path for both).
pub fn activate(
    id: &str,
    s: &mut Scratch,
    g: &StainGraph,
    content: Option<&Content>,
    out: &mut Out,
) {
    if yours::activate(id, s, g, content, out) {
    } else if let Some((_, view, _)) = VIEWS.iter().find(|(v, ..)| *v == id) {
        s.view = *view;
    } else if let Some(node) = NODE_IDS.iter().position(|n| *n == id) {
        s.canvas.selection = BTreeSet::from([NodeId(node as u32)]);
    } else if id == COLLAPSE {
        s.library.collapsed = !s.library.collapsed;
    } else if let Some(group) = GROUP_IDS.iter().position(|g| *g == id) {
        if !s.library.closed.remove(&group) {
            s.library.closed.insert(group);
        }
    } else if let Some(k) = ROW_IDS.iter().position(|r| *r == id) {
        // A row with a graph loads it, replacing the graph wholesale (§11), and its highlight is then the loaded
        // stain's; any other row is only selected.
        match content.and_then(|c| c.graphs.get(k)?.as_ref()) {
            Some(load) => {
                s.library.selected = None;
                let edit = edited(g, |g| {
                    *g = load.clone();
                    Ok(())
                });
                out.loads |= edit.is_some();
                out.edits.extend(edit);
            }
            None => s.library.selected = content.and_then(|c| library::row(c, k)).map(|_| k),
        }
    } else if id == EDIT_WGSL {
        s.view = View::Node;
    } else if id == DUPLICATE {
        let Some(node) = s.selected() else {
            return;
        };
        let mut copy = None;
        let edit = edited(g, |g| {
            copy = Some(g.duplicate_as_custom(node)?);
            Ok(())
        });
        if let (Some(edit), Some(copy)) = (edit, copy) {
            out.edits.push(edit);
            s.canvas.selection = BTreeSet::from([copy]);
        }
    } else if id == AUTO_RECOMPILE {
        s.auto_recompile = !s.auto_recompile;
    } else if id == RECOMPILE {
        out.recompile = true;
    } else if let Some(i) = CHOICE_IDS.iter().position(|c| *c == id) {
        let (Some(n), Some(c)) = (s.selected().and_then(|id| g.node(id)), content) else {
            return;
        };
        if let Some(choice) = choices(n, c).get(i).filter(|choice| !chosen(n, choice, c)) {
            out.edits
                .extend(edited(g, |g| g.set_occupant(n.id, choice.occupant.clone())));
        }
    } else if let Some(node) = s.selected() {
        let sections = selected_sections(s, content);
        if let Some(section) = sections.iter().find(|t| t.scope.id == id) {
            if !s.folded.remove(&section.title) {
                s.folded.insert(section.title.clone());
            }
        } else if let Some(part) = controls::find(&sections, id) {
            match &part.act {
                controls::Act::Set(uniform, value) => {
                    out.edits
                        .extend(s.set_param(g, node, uniform, value.clone()))
                }
                controls::Act::Open => {
                    let key = (node, part.scope.id);
                    if !s.open.remove(&key) {
                        s.open.insert(key);
                    }
                }
                controls::Act::Step(_) => {}
            }
        }
    }
}

/// The arrows on one of the inspector's numbers, in interaction mode: one step of it, one `SetField`.
pub fn adjust(
    adjust: &Adjust,
    s: &Scratch,
    g: &StainGraph,
    content: Option<&Content>,
    out: &mut Out,
) {
    let Some(node) = s.selected() else {
        return;
    };
    let sections = selected_sections(s, content);
    let Some(controls::Act::Step(step)) =
        controls::find(&sections, adjust.scope).map(|part| &part.act)
    else {
        return;
    };
    let next = step.moved(adjust);
    out.edits.extend(s.set_param(g, node, &step.uniform, next));
}

/// What the page draws from.
pub struct Page<'a> {
    /// The graph.
    pub graph: &'a StainGraph,
    /// The content, if the engine side serves one.
    pub content: Option<&'a Content>,
    /// The preview's canvas, if any, with its target format.
    pub preview: Option<(Arc<dyn Canvas>, wgpu::TextureFormat)>,
    /// The Explore figure's rect: the viewport whose aspect the preview keeps.
    pub viewport: Rect,
    /// The keyboard's focus path.
    pub focus: &'a [String],
}

/// Draws the Stain page in `layout`'s page; the controls clicked go to `clicked`, the edits to `out`.
pub fn show(
    ui: &mut Ui,
    layout: &Layout,
    page: &Page<'_>,
    s: &mut Scratch,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
    out: &mut Out,
) {
    let regions = Regions::new(layout, ui.ctx().pixels_per_point(), s.library.collapsed);
    keys.keyboard.place(LIBRARY, regions.library);
    keys.keyboard.place(CANVAS, regions.canvas);
    keys.keyboard.place(INSPECTOR, regions.right);
    keys.keyboard
        .place(CODE, regions.code.union(regions.problems));
    let mut own = |salt: &str| ui.new_child(UiBuilder::new().id_salt(salt).max_rect(ui.max_rect()));
    library::show(
        &mut own("stain_library"),
        regions.library,
        page.content,
        &mut s.library,
        keys,
        clicked,
    );
    let mut frame = canvas::Frame {
        graph: page.graph,
        content: page.content,
        view: &mut s.view,
        canvas: &mut s.canvas,
        yours: &mut s.yours,
    };
    canvas::show(
        &mut own("stain_canvas"),
        regions.canvas,
        &mut frame,
        keys,
        clicked,
    );
    preview::show(
        &mut own("stain_inspector"),
        regions.right,
        page,
        s,
        keys,
        clicked,
        out,
    );
    code_view::show(&mut own("stain_code"), regions.code, page.content);
    problems::show(
        &mut own("stain_problems"),
        regions.problems,
        page.content,
        s.auto_recompile,
        keys,
        clicked,
    );
}
