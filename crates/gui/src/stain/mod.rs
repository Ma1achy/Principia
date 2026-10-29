//! Stain mode, the plain node-graph editor (render_gui_spec Part II §1, §7, §8; GUI_DESIGN_NOTES § "02 Stain";
//! 02_stain.png; R-390, ORDER item 6): the library drawer, collapsible, left (`library`); the graph canvas centre,
//! with its `Graph · Pipeline WGSL · Node WGSL` toggle (`canvas`); the preview and the node inspector right
//! (`preview`); and the assembled code (`code_view`) and the Problems pane (`problems`) under the canvas. The graph is
//! the snapshot's stain graph (gui_state_contract §2, §5), and every edit of it is one `SetField` of the whole graph,
//! undoable (R-69): the inspector's. The canvas's places, view and selection, the drawer's state and the toggle are
//! the editor's own.
//!
//! **Keys** (render_gui_spec §G3; R-409; applied per R-369, review 5434766412 on PR #157): in Stain mode the big scopes
//! in Tab order are the top bar, the library drawer, the graph canvas, the preview and node inspector, the code and
//! Problems pane, and the footer last (R-405). Enter goes into one, landing on its first element in navigation mode,
//! and the arrows move between its elements: the drawer's collapse, filter, groups and rows; the canvas's toggle and
//! its nodes, in their order on the canvas; the inspector's fields for the selected node; and auto-recompile and
//! Recompile. Enter on a button, a row or a node acts as its click (a node's selects it, opening its inspector), on
//! the filter starts typing, and on a param starts interaction, in which the arrows step it by its base step
//! (`StepKind::Bounded`, R-404), ×10 with Shift and ×0.1 with Alt.

pub mod canvas;
pub mod code_view;
pub mod library;
pub mod preview;
pub mod problems;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use eframe::egui::{pos2, Rect, Ui, UiBuilder};
use engine::contract::canvas::Canvas;
use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};

use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::layout::{Layout, ARTBOARD};
use crate::windows::Keys;

/// The Stain editor's content the engine side serves (`crate::side::EngineSide::stain_content`).
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    /// The library's groups, in order.
    pub library: &'static [Group],
    /// How many modes the catalogue holds.
    pub modes: u32,
    /// The library row of the stain loaded.
    pub loaded: Option<&'static str>,
    /// The assembled code.
    pub code: String,
    /// The whole pipeline's code (Pipeline WGSL).
    pub pipeline: String,
    /// Each node's code (Node WGSL).
    pub node_code: BTreeMap<NodeId, String>,
    /// The compile status line.
    pub compiled: String,
    /// The problems, one line each.
    pub problems: Vec<String>,
    /// The notes, one line each.
    pub notes: Vec<String>,
    /// The fields a source can read, as its inspector offers them.
    pub fields: &'static [&'static str],
    /// The combiner's built-in occupants, each its id and name.
    pub combiners: &'static [(&'static str, &'static str)],
}

/// One group of the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// Its heading.
    pub title: &'static str,
    /// Its rows.
    pub rows: &'static [&'static str],
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
/// The inspector's Revert, drawn disabled: a hand edit's revert is TASK-M7-24's.
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
/// The library's rows' scopes, by their place in the library.
pub const ROW_IDS: [ScopeId; 32] = [
    "stain_lib_row_0",
    "stain_lib_row_1",
    "stain_lib_row_2",
    "stain_lib_row_3",
    "stain_lib_row_4",
    "stain_lib_row_5",
    "stain_lib_row_6",
    "stain_lib_row_7",
    "stain_lib_row_8",
    "stain_lib_row_9",
    "stain_lib_row_10",
    "stain_lib_row_11",
    "stain_lib_row_12",
    "stain_lib_row_13",
    "stain_lib_row_14",
    "stain_lib_row_15",
    "stain_lib_row_16",
    "stain_lib_row_17",
    "stain_lib_row_18",
    "stain_lib_row_19",
    "stain_lib_row_20",
    "stain_lib_row_21",
    "stain_lib_row_22",
    "stain_lib_row_23",
    "stain_lib_row_24",
    "stain_lib_row_25",
    "stain_lib_row_26",
    "stain_lib_row_27",
    "stain_lib_row_28",
    "stain_lib_row_29",
    "stain_lib_row_30",
    "stain_lib_row_31",
];
/// The library's groups' headings' scopes.
pub const GROUP_IDS: [ScopeId; 8] = [
    "stain_lib_group_0",
    "stain_lib_group_1",
    "stain_lib_group_2",
    "stain_lib_group_3",
    "stain_lib_group_4",
    "stain_lib_group_5",
    "stain_lib_group_6",
    "stain_lib_group_7",
];
/// The inspector's choices' scopes: a source's fields, the combiner's occupants.
pub const CHOICE_IDS: [ScopeId; 8] = [
    "stain_insp_choice_0",
    "stain_insp_choice_1",
    "stain_insp_choice_2",
    "stain_insp_choice_3",
    "stain_insp_choice_4",
    "stain_insp_choice_5",
    "stain_insp_choice_6",
    "stain_insp_choice_7",
];
/// The inspector's params' scopes, by their place in the node's schema.
pub const PARAM_IDS: [ScopeId; 8] = [
    "stain_insp_param_0",
    "stain_insp_param_1",
    "stain_insp_param_2",
    "stain_insp_param_3",
    "stain_insp_param_4",
    "stain_insp_param_5",
    "stain_insp_param_6",
    "stain_insp_param_7",
];

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
}

impl Default for Scratch {
    fn default() -> Self {
        Self {
            canvas: canvas::Canvas::default(),
            view: View::Graph,
            library: library::State::default(),
            auto_recompile: true,
            preview: None,
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

/// The inspector's choices for node `n`: a source's fields, the combiner's occupants; each the occupant it sets and
/// its label.
pub fn choices(
    n: &engine::contract::render_state::Node,
    content: &Content,
) -> Vec<(Occupant, &'static str)> {
    match n.kind {
        NodeKind::Source => content
            .fields
            .iter()
            .map(|f| (Occupant::Field((*f).to_owned()), *f))
            .collect(),
        NodeKind::Combiner => content
            .combiners
            .iter()
            .map(|(id, name)| (Occupant::Builtin((*id).to_owned()), *name))
            .collect(),
        _ => Vec::new(),
    }
}

/// The scalar params of node `id` the inspector edits: each a uniform with a range, by its place in the schema.
pub fn params(g: &StainGraph, id: NodeId) -> Vec<(String, f64, f64, f64)> {
    let Some(n) = g.node(id) else {
        return Vec::new();
    };
    g.uniforms(id)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|u| {
            let (lo, hi) = u.range?;
            let value = n.params.get(&u.name).unwrap_or(&u.default)[0];
            Some((u.name, value, lo, hi))
        })
        .collect()
}

/// The children `parent` should have, each a scope, for the graph, the content and the editor's state.
fn wanted(g: &StainGraph, s: &Scratch, content: Option<&Content>) -> [(ScopeId, Vec<Scope>); 3] {
    let mut library = vec![Scope::control(
        COLLAPSE,
        if s.library.collapsed {
            "expand"
        } else {
            "collapse"
        },
    )];
    if !s.library.collapsed {
        library.push(Scope::text(FILTER, "filter"));
        if let Some(c) = content {
            for item in library::visible(c, &s.library) {
                library.push(match item {
                    library::Item::Group(g, title) => Scope::control(GROUP_IDS[g], title),
                    library::Item::Row(k, row) => Scope::control(ROW_IDS[k], row),
                });
            }
        }
        library.extend(BUTTONS.iter().map(|(id, label)| Scope::disabled(id, label)));
    }
    let mut canvas = VIEWS
        .iter()
        .map(|(id, _, label)| Scope::control(id, label))
        .collect::<Vec<_>>();
    if s.view == View::Graph {
        for id in node_order(g, &s.canvas) {
            let (Some(scope), Some(n)) = (canvas::node_scope(id), g.node(id)) else {
                continue;
            };
            let (kind, name) = canvas::title(n.kind, &n.occupant);
            canvas.push(Scope::control(scope, intern(&format!("{kind} · {name}"))));
        }
    }
    let mut inspector = Vec::new();
    if let (Some(id), Some(c)) = (s.selected(), content) {
        if let Some(n) = g.node(id) {
            for (i, (_, label)) in choices(n, c).into_iter().enumerate().take(CHOICE_IDS.len()) {
                inspector.push(Scope::control(CHOICE_IDS[i], label));
            }
            for (i, (name, ..)) in params(g, id).into_iter().enumerate().take(PARAM_IDS.len()) {
                inspector.push(Scope::value(PARAM_IDS[i], intern(&name), StepKind::Bounded));
            }
            if n.kind != NodeKind::Out {
                inspector.push(Scope::control(EDIT_WGSL, "Edit this node's WGSL"));
                inspector.push(Scope::disabled(REVERT, "Revert"));
            }
        }
    }
    [(LIBRARY, library), (CANVAS, canvas), (INSPECTOR, inspector)]
}

/// Joins the drawer's, the canvas's and the inspector's elements to `tree` as they stand, again whenever they change.
pub fn join(tree: &mut ScopeTree, g: &StainGraph, s: &Scratch, content: Option<&Content>) {
    for (parent, scopes) in wanted(g, s, content) {
        let now: Vec<Scope> = tree
            .children(Some(parent))
            .iter()
            .filter_map(|id| tree.get(id).copied())
            .collect();
        if now != scopes {
            tree.remove_children(parent);
            for scope in scopes {
                tree.register(Some(parent), scope);
            }
        }
    }
}

/// What the editor's controls ask of the app.
#[derive(Debug, Default, PartialEq)]
pub struct Out {
    /// The graph's edits, each a `SetField`.
    pub edits: Vec<SetField>,
    /// Recompile.
    pub recompile: bool,
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
    if let Some((_, view, _)) = VIEWS.iter().find(|(v, ..)| *v == id) {
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
        s.library.selected = content.and_then(|c| library::row(c, k));
    } else if id == EDIT_WGSL {
        s.view = View::Node;
    } else if id == AUTO_RECOMPILE {
        s.auto_recompile = !s.auto_recompile;
    } else if id == RECOMPILE {
        out.recompile = true;
    } else if let Some(i) = CHOICE_IDS.iter().position(|c| *c == id) {
        let (Some(node), Some(c)) = (s.selected(), content) else {
            return;
        };
        let Some((occupant, _)) = g.node(node).and_then(|n| choices(n, c).into_iter().nth(i))
        else {
            return;
        };
        out.edits
            .extend(edited(g, |g| g.set_occupant(node, occupant)));
    }
}

/// The arrows on a param, in interaction mode: its value by its base step, `Bounded`'s share of its range, kept in it;
/// one `SetField`.
pub fn adjust(adjust: &Adjust, s: &Scratch, g: &StainGraph, out: &mut Out) {
    let Some(i) = PARAM_IDS.iter().position(|p| *p == adjust.scope) else {
        return;
    };
    let Some(node) = s.selected() else {
        return;
    };
    let Some((name, value, lo, hi)) = params(g, node).into_iter().nth(i) else {
        return;
    };
    let Base::OfRange(share) = adjust.delta() else {
        return;
    };
    let next = (value + share * (hi - lo)).clamp(lo, hi);
    out.edits
        .extend(edited(g, |g| g.set_param(node, &name, vec![next])));
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
