//! The graph canvas (render_gui_spec Part II §7, §8; 02_stain.png): the stain graph's nodes as glyph-forward boxes,
//! their typed ports coloured by type, the wires between them, and the gestures of §7 that this task builds: click a
//! node to select it, opening its inspector; click empty canvas to deselect, the inspector showing the graph; drag a
//! marquee to select every node it touches; drag a node to move it, its wires following, a selection moving together;
//! space-drag or middle-drag to pan; scroll to zoom about the pointer. The wiring gestures, the palette, Delete and
//! the node visuals' remaining states are TASK-M8-20's and TASK-M7-22's.
//!
//! A node's place, the pan and the zoom and the selection are the editor's own state, never the graph's, so none is a
//! `SetField` (gui_state_contract §2, "The stain graph"): the graph's edits are the inspector's.
//!
//! [`Canvas`] holds that state and the gestures as a pure machine of pointer events, which `show` feeds from egui's
//! events, so a scripted sequence of events tests each gesture (REQ-GUI-131).

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{
    self, pos2, vec2, Align, Color32, Event, FontId, Key, Layout as EguiLayout, MouseWheelUnit,
    PointerButton, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};
use engine::contract::legend::{OUTCOME_OCCUPANT, SWATCHES};
use engine::contract::render_state::{NodeId, NodeKind, Occupant, PortType, StainGraph};

use super::{Content, View, NODE_IDS, VIEWS};
use crate::keyboard::scopes::ScopeId;
use crate::windows::Keys;

/// A node's width on the canvas at zoom 1, in points; `OUT`'s is [`OUT_W`] (02_stain.png's, a look choice, R-390).
pub const NODE_W: f32 = 172.0;
/// `OUT`'s width.
pub const OUT_W: f32 = 132.0;
/// A node's title bar's height.
pub const TITLE_H: f32 = 26.0;
/// A port row's height.
pub const ROW_H: f32 = 22.0;
/// The state glyph's row's height.
pub const GLYPH_H: f32 = 28.0;
/// The space below a node's last row.
pub const PAD: f32 = 6.0;
/// A port's radius.
pub const PORT_R: f32 = 5.0;
/// How far the pointer moves, in points, before a press becomes a drag: at most this far it is a click.
pub const DRAG_THRESHOLD: f32 = 3.0;
/// The zoom's bounds.
pub const ZOOM_MIN: f32 = 0.25;
/// See [`ZOOM_MIN`].
pub const ZOOM_MAX: f32 = 3.0;
/// The zoom's change per point scrolled: the zoom is multiplied by `exp(ZOOM_PER_POINT · Δy)`.
pub const ZOOM_PER_POINT: f32 = 0.002;
/// A wheel line's length in points, for a wheel that scrolls by lines.
pub const LINE_POINTS: f32 = 40.0;
/// The grid's spacing at zoom 1.
pub const GRID: f32 = 24.0;
/// The canvas's hint line along its foot: the gestures it takes and the types' colours.
pub const HINT: &str = "drag: marquee · space- or middle-drag: pan · scroll: zoom · types:";
/// The darkest of a brightness node's ramp glyph, as a fraction of full (a look choice, R-390).
pub const RAMP_DARKEST: f32 = 0.25;
/// A selected node's accent outline's width (§8): apart from the focus ring's, so the two never read as one.
pub const SELECTED_W: f32 = 1.5;
/// The toggle's buttons' height.
pub const TOGGLE_H: f32 = 22.0;

/// The wire and port colour of each type (02_stain.png: `field` grey, colour salmon, brightness cyan; a look
/// choice, R-390).
pub fn type_colour(t: PortType) -> Color32 {
    match t {
        PortType::Field => Color32::from_gray(170),
        PortType::Vec3 => Color32::from_rgb(232, 112, 100),
        PortType::F32 => Color32::from_rgb(110, 196, 214),
    }
}

/// The pan and the zoom: a canvas point `p` is drawn at `origin + pan + p · zoom`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    /// The offset, in points.
    pub pan: Vec2,
    /// The scale.
    pub zoom: f32,
}

impl Default for Zoom {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

/// A pointer button the canvas reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// The primary button.
    Primary,
    /// The middle button.
    Middle,
}

/// The gesture under way.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Gesture {
    /// None.
    #[default]
    Idle,
    /// A pan: the pointer's last place.
    Pan { last: Pos2 },
    /// A primary press, on a node or on empty canvas: where it began, the pointer's last place once dragging, and
    /// whether it has become a drag.
    Press {
        node: Option<NodeId>,
        origin: Pos2,
        last: Pos2,
        dragging: bool,
    },
}

/// The canvas's own state: the view, the nodes' places, the selection and the gesture under way.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Canvas {
    /// The pan and the zoom.
    pub view: Zoom,
    /// Each node's place, its top-left corner on the canvas at zoom 1.
    pub positions: BTreeMap<NodeId, Pos2>,
    /// The selected nodes.
    pub selection: BTreeSet<NodeId>,
    /// The marquee being dragged, in points.
    pub marquee: Option<Rect>,
    gesture: Gesture,
}

/// The column a kind is placed in, left to right in the pipeline's order, and the first row's top in it.
fn column(kind: NodeKind) -> (usize, f32) {
    match kind {
        NodeKind::Source => (0, 64.0),
        NodeKind::Colour | NodeKind::Brightness => (1, 64.0),
        NodeKind::Combiner | NodeKind::Post => (2, 128.0),
        NodeKind::Out => (3, 280.0),
    }
}

/// The columns' left edges, at zoom 1 (02_stain.png's, a look choice, R-390).
pub const COLUMNS: [f32; 4] = [24.0, 244.0, 464.0, 680.0];
/// The rows' spacing in a column.
pub const ROW_SPACING: f32 = 180.0;

/// Where a node of `kind` goes when it has no place yet, the `k`th of its column.
pub fn default_position(kind: NodeKind, k: usize) -> Pos2 {
    let (c, top) = column(kind);
    pos2(COLUMNS[c], top + k as f32 * ROW_SPACING)
}

/// The in-ports' labels of a node of `kind` with `ports`: the combiner's colour and brightness, else each type.
pub fn port_labels(kind: NodeKind, ports: &[PortType]) -> Vec<&'static str> {
    ports
        .iter()
        .enumerate()
        .map(|(i, &t)| match (kind, i) {
            (NodeKind::Combiner, 0) => "colour",
            (NodeKind::Combiner, _) => "brightness",
            _ => type_name(t),
        })
        .collect()
}

/// A type's name, as the ports read it.
pub fn type_name(t: PortType) -> &'static str {
    match t {
        PortType::Field => "field",
        PortType::Vec3 => "vec3",
        PortType::F32 => "f32",
    }
}

/// Whether a node of `kind` draws a state glyph row (§8): every kind but `OUT`.
fn has_glyph(kind: NodeKind) -> bool {
    kind != NodeKind::Out
}

/// A node's size at zoom 1: its title, its glyph, a row per in-port and one for its out-port.
pub fn size(kind: NodeKind, in_ports: usize) -> Vec2 {
    let rows = in_ports + usize::from(kind.output().is_some());
    let glyph = if has_glyph(kind) { GLYPH_H } else { 0.0 };
    let w = if kind == NodeKind::Out { OUT_W } else { NODE_W };
    vec2(w, TITLE_H + glyph + rows as f32 * ROW_H + PAD)
}

/// The y of a node's row `row` (0 the first port row) from its top, at zoom 1.
fn row_y(kind: NodeKind, row: usize) -> f32 {
    let glyph = if has_glyph(kind) { GLYPH_H } else { 0.0 };
    TITLE_H + glyph + (row as f32 + 0.5) * ROW_H
}

/// One node as drawn: its id, its rect in points, its in-ports and its out-port's type.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    /// The node.
    pub id: NodeId,
    /// Its kind.
    pub kind: NodeKind,
    /// Its rect, in points.
    pub rect: Rect,
    /// Its in-ports' types, in order.
    pub ins: Vec<PortType>,
}

impl Drawn {
    /// In-port `i`'s centre.
    pub fn in_port(&self, i: usize, zoom: f32) -> Pos2 {
        pos2(
            self.rect.min.x,
            self.rect.min.y + row_y(self.kind, i) * zoom,
        )
    }

    /// The out-port's centre.
    pub fn out_port(&self, zoom: f32) -> Pos2 {
        pos2(
            self.rect.max.x,
            self.rect.min.y + row_y(self.kind, self.ins.len()) * zoom,
        )
    }

    /// The title bar's rect.
    pub fn title(&self, zoom: f32) -> Rect {
        Rect::from_min_size(self.rect.min, vec2(self.rect.width(), TITLE_H * zoom))
    }
}

impl Canvas {
    /// Gives each node of `g` without a place its default one, in id order within its column, and forgets the places
    /// and the selection of nodes no longer in it.
    pub fn place(&mut self, g: &StainGraph) {
        let ids: BTreeSet<NodeId> = g.nodes().map(|n| n.id).collect();
        self.positions.retain(|id, _| ids.contains(id));
        self.selection.retain(|id| ids.contains(id));
        let mut counts = [0usize; 4];
        for n in g.nodes() {
            let (c, _) = column(n.kind);
            let k = counts[c];
            counts[c] += 1;
            self.positions
                .entry(n.id)
                .or_insert_with(|| default_position(n.kind, k));
        }
    }

    /// The nodes of `g` as drawn on a canvas whose top-left is `origin`, in id order, the last on top.
    pub fn drawn(&self, g: &StainGraph, origin: Pos2) -> Vec<Drawn> {
        g.nodes()
            .filter_map(|n| {
                let at = *self.positions.get(&n.id)?;
                let ins = g.in_ports(n.id).unwrap_or_default();
                let size = size(n.kind, ins.len()) * self.view.zoom;
                let min = origin + self.view.pan + at.to_vec2() * self.view.zoom;
                Some(Drawn {
                    id: n.id,
                    kind: n.kind,
                    rect: Rect::from_min_size(min, size),
                    ins,
                })
            })
            .collect()
    }

    /// Whether a gesture is under way.
    pub fn busy(&self) -> bool {
        self.gesture != Gesture::Idle
    }

    /// The node under `at`: the topmost whose rect holds it.
    pub fn hit(nodes: &[Drawn], at: Pos2) -> Option<NodeId> {
        nodes
            .iter()
            .rev()
            .find(|d| d.rect.contains(at))
            .map(|d| d.id)
    }

    /// A press of `button` at `at`, `space` held or not, over `nodes`: the middle button, or the primary with Space
    /// held, starts a pan; the primary on a node selects it, unless it is selected already (so a selection drags
    /// together), and on empty canvas starts a click or a marquee. A press during a gesture is ignored.
    pub fn press(&mut self, at: Pos2, button: Button, space: bool, nodes: &[Drawn]) {
        if self.busy() {
            return;
        }
        if button == Button::Middle || space {
            self.gesture = Gesture::Pan { last: at };
            return;
        }
        let node = Self::hit(nodes, at);
        if let Some(n) = node {
            if !self.selection.contains(&n) {
                self.selection = BTreeSet::from([n]);
            }
        }
        self.gesture = Gesture::Press {
            node,
            origin: at,
            last: at,
            dragging: false,
        };
    }

    /// The pointer moved to `at`: a pan moves the view; a press past [`DRAG_THRESHOLD`] becomes a drag, moving the
    /// selection with it from a node, or drawing the marquee from empty canvas.
    pub fn moved(&mut self, at: Pos2) {
        let zoom = self.view.zoom;
        match &mut self.gesture {
            Gesture::Idle => {}
            Gesture::Pan { last } => {
                self.view.pan += at - *last;
                *last = at;
            }
            Gesture::Press {
                node,
                origin,
                last,
                dragging,
            } => {
                if !*dragging && (at - *origin).length() > DRAG_THRESHOLD {
                    *dragging = true;
                }
                if !*dragging {
                    return;
                }
                match node {
                    Some(_) => {
                        let delta = (at - *last) / zoom;
                        for id in &self.selection {
                            if let Some(p) = self.positions.get_mut(id) {
                                *p += delta;
                            }
                        }
                    }
                    None => self.marquee = Some(Rect::from_two_pos(*origin, at)),
                }
                *last = at;
            }
        }
    }

    /// The pointer released at `at`, over `nodes`: a click on a node selects it alone, a click on empty canvas
    /// deselects, and a marquee selects every node it touches.
    pub fn release(&mut self, at: Pos2, nodes: &[Drawn]) {
        self.moved(at);
        if let Gesture::Press { node, dragging, .. } = self.gesture {
            match (node, dragging) {
                (Some(n), false) => self.selection = BTreeSet::from([n]),
                (None, false) => self.selection.clear(),
                (None, true) => {
                    let marquee = self.marquee.unwrap_or(Rect::NOTHING);
                    self.selection = nodes
                        .iter()
                        .filter(|d| d.rect.intersects(marquee))
                        .map(|d| d.id)
                        .collect();
                }
                (Some(_), true) => {}
            }
        }
        self.marquee = None;
        self.gesture = Gesture::Idle;
    }

    /// A scroll of `dy` points at `at` on a canvas whose top-left is `origin`: the zoom, about `at`, kept within
    /// [`ZOOM_MIN`, `ZOOM_MAX`].
    pub fn scroll(&mut self, origin: Pos2, at: Pos2, dy: f32) {
        let old = self.view.zoom;
        let new = (old * (dy * ZOOM_PER_POINT).exp()).clamp(ZOOM_MIN, ZOOM_MAX);
        let from = at - origin;
        self.view.pan = from - (from - self.view.pan) * (new / old);
        self.view.zoom = new;
    }

    /// Pans the least that brings `rect` inside `within`, when it fits: the focus's scroll to a node.
    pub fn reveal(&mut self, rect: Rect, within: Rect) {
        let shift = |lo: f32, hi: f32, min: f32, max: f32| {
            if lo < min {
                min - lo
            } else if hi > max {
                (max - hi).max(min - lo)
            } else {
                0.0
            }
        };
        self.view.pan += vec2(
            shift(rect.min.x, rect.max.x, within.min.x, within.max.x),
            shift(rect.min.y, rect.max.y, within.min.y, within.max.y),
        );
    }
}

/// A node's title: its kind and its occupant, as the breadcrumb and the inspector read it.
pub fn title(kind: NodeKind, occupant: &Occupant) -> (&'static str, String) {
    let kind = match kind {
        NodeKind::Source => "source",
        NodeKind::Colour => "colour",
        NodeKind::Brightness => "brightness",
        NodeKind::Combiner => "combiner",
        NodeKind::Post => "post",
        NodeKind::Out => "OUT",
    };
    let name = match occupant {
        Occupant::None if kind == "OUT" => "to the style".to_owned(),
        Occupant::None => "None".to_owned(),
        Occupant::Field(f) => format!("ctx.{f}"),
        Occupant::Builtin(id) => id.clone(),
        Occupant::Custom(_) => "custom".to_owned(),
    };
    (kind, name)
}

/// Feeds the frame's pointer events on `rect` to `canvas`: presses inside it, off `toggles` and on its own layer;
/// every move and release, so a drag goes on past its edge; and the wheel over it.
fn gestures(ui: &Ui, rect: Rect, toggles: Rect, canvas: &mut Canvas, nodes: &[Drawn]) {
    let (events, space, hover) = ui.input(|i| {
        (
            i.events.clone(),
            i.key_down(Key::Space),
            i.pointer.latest_pos(),
        )
    });
    let ctx = ui.ctx();
    let ours = |p: Pos2| {
        rect.contains(p)
            && !toggles.contains(p)
            && ctx.layer_id_at(p).is_none_or(|l| l == ui.layer_id())
    };
    let mut at = hover;
    for event in events {
        match event {
            Event::PointerMoved(p) => {
                at = Some(p);
                canvas.moved(p);
            }
            Event::PointerButton {
                pos,
                button,
                pressed: true,
                ..
            } if ours(pos) => {
                let button = match button {
                    PointerButton::Primary => Button::Primary,
                    PointerButton::Middle => Button::Middle,
                    _ => continue,
                };
                canvas.press(pos, button, space, nodes);
            }
            Event::PointerButton {
                pos,
                pressed: false,
                button: PointerButton::Primary | PointerButton::Middle,
                ..
            } => canvas.release(pos, nodes),
            Event::MouseWheel { unit, delta, .. } => {
                let Some(p) = at.filter(|p| ours(*p)) else {
                    continue;
                };
                let dy = match unit {
                    MouseWheelUnit::Point => delta.y,
                    MouseWheelUnit::Line => delta.y * LINE_POINTS,
                    MouseWheelUnit::Page => delta.y * rect.height(),
                };
                canvas.scroll(rect.min, p, dy);
            }
            _ => {}
        }
    }
}

/// What the canvas draws from: the graph, the content, and the editor's state.
pub struct Frame<'a> {
    /// The graph.
    pub graph: &'a StainGraph,
    /// The content, for the code views.
    pub content: Option<&'a Content>,
    /// The view the toggle shows.
    pub view: &'a mut View,
    /// The canvas's state.
    pub canvas: &'a mut Canvas,
}

/// Draws the canvas in `rect`: its toggle, and the graph or the code its toggle shows. The controls clicked are pushed
/// to `clicked`, for the one path a click and Enter share.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    frame: &mut Frame<'_>,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    let visuals = ui.visuals().clone();
    ui.painter()
        .rect_filled(rect, 0.0, visuals.extreme_bg_color);
    let mut own = ui.new_child(UiBuilder::new().max_rect(rect).id_salt("stain_canvas"));
    own.set_clip_rect(rect.intersect(ui.clip_rect()));
    let toggles = toggle(&mut own, rect, *frame.view, keys, clicked);
    match frame.view {
        View::Graph => graph(&mut own, rect, toggles, frame, keys),
        View::Pipeline | View::Node => {
            let text = frame
                .content
                .map_or_else(String::new, |c| match frame.view {
                    View::Pipeline => c.pipeline.clone(),
                    _ => match frame.canvas.selection.iter().next() {
                        Some(id) if frame.canvas.selection.len() == 1 => {
                            c.node_code.get(id).cloned().unwrap_or_default()
                        }
                        _ => "// select one node to see its WGSL".to_owned(),
                    },
                });
            let below = Rect::from_min_max(pos2(rect.min.x, toggles.max.y), rect.max).shrink(12.0);
            let mut code = own.new_child(UiBuilder::new().max_rect(below));
            egui::ScrollArea::both()
                .id_salt("stain_canvas_code")
                .auto_shrink(false)
                .show(&mut code, |ui| super::code_view::code(ui, &text));
        }
    }
}

/// Draws the `Graph · Pipeline WGSL · Node WGSL` toggle at the canvas's top right; returns its rect.
fn toggle(
    ui: &mut Ui,
    rect: Rect,
    view: View,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) -> Rect {
    let bar = Rect::from_min_max(
        pos2(rect.min.x, rect.min.y + 8.0),
        pos2(rect.max.x - 12.0, rect.min.y + 8.0 + TOGGLE_H),
    );
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(bar)
            .layout(EguiLayout::right_to_left(Align::Center)),
    );
    let mut used = Rect::NOTHING;
    for (id, this, label) in VIEWS.iter().rev() {
        let response = row.add(egui::Button::selectable(view == *this, *label));
        if response.clicked() {
            clicked.push(id);
        }
        keys.place(id, &response);
        used = used.union(response.rect);
    }
    used
}

/// Draws the graph view: the grid, the wires, the nodes and the marquee, after the frame's gestures; then the hint.
fn graph(ui: &mut Ui, rect: Rect, toggles: Rect, frame: &mut Frame<'_>, keys: &mut Keys<'_>) {
    let g = frame.graph;
    let canvas = &mut *frame.canvas;
    canvas.place(g);
    let nodes = canvas.drawn(g, rect.min);
    gestures(ui, rect, toggles, canvas, &nodes);
    // The focus's scroll: a node the focus lands on is brought into view.
    if let Some(d) = nodes
        .iter()
        .find(|d| keys.focused.as_deref() == node_scope(d.id))
    {
        canvas.reveal(d.rect, rect.shrink(8.0));
    }
    let nodes = canvas.drawn(g, rect.min);
    let zoom = canvas.view.zoom;
    let painter = ui.painter().clone();
    let visuals = ui.visuals().clone();
    // The grid: dots every GRID points of the canvas.
    let step = GRID * zoom;
    if step >= 6.0 {
        let start = rect.min
            + vec2(
                (canvas.view.pan.x).rem_euclid(step),
                (canvas.view.pan.y).rem_euclid(step),
            );
        let dot = visuals.widgets.noninteractive.bg_stroke.color;
        let mut y = start.y;
        while y < rect.max.y {
            let mut x = start.x;
            while x < rect.max.x {
                painter.circle_filled(pos2(x, y), 1.0, dot);
                x += step;
            }
            y += step;
        }
    }
    // The wires, under the nodes, from each out-port to the in-port it feeds, in its type's colour.
    for w in g.wires() {
        let (Some(from), Some(to)) = (
            nodes.iter().find(|d| d.id == w.from),
            nodes.iter().find(|d| d.id == w.to),
        ) else {
            continue;
        };
        let port = w.port as usize;
        let Some(&t) = to.ins.get(port) else {
            continue;
        };
        let (a, b) = (from.out_port(zoom), to.in_port(port, zoom));
        let bend = ((b.x - a.x).abs() * 0.5).max(30.0 * zoom);
        painter.add(Shape::CubicBezier(
            egui::epaint::CubicBezierShape::from_points_stroke(
                [a, a + vec2(bend, 0.0), b - vec2(bend, 0.0), b],
                false,
                Color32::TRANSPARENT,
                Stroke::new(2.0 * zoom.max(0.5), type_colour(t)),
            ),
        ));
    }
    for d in &nodes {
        node(
            ui,
            &painter,
            &visuals,
            g,
            d,
            zoom,
            canvas.selection.contains(&d.id),
        );
        if let Some(scope) = node_scope(d.id) {
            keys.keyboard.place(scope, d.rect.intersect(rect));
        }
    }
    if let Some(m) = canvas.marquee {
        painter.rect(
            m,
            0.0,
            visuals.selection.bg_fill.gamma_multiply(0.25),
            Stroke::new(1.0, visuals.selection.stroke.color),
            StrokeKind::Inside,
        );
    }
    hint(&painter, &visuals, rect);
}

/// Node `id`'s keyboard scope, while the pool holds one for it.
pub fn node_scope(id: NodeId) -> Option<ScopeId> {
    NODE_IDS.get(id.0 as usize).copied()
}

/// The hint along the canvas's foot, with the types' colours.
fn hint(painter: &egui::Painter, visuals: &egui::Visuals, rect: Rect) {
    let font = FontId::monospace(11.0);
    let y = rect.max.y - 14.0;
    let strip = Rect::from_min_max(pos2(rect.min.x, y - 10.0), pos2(rect.max.x, rect.max.y));
    painter.rect_filled(strip, 0.0, visuals.extreme_bg_color);
    let text = painter.text(
        pos2(rect.min.x + 12.0, y),
        egui::Align2::LEFT_CENTER,
        HINT,
        font.clone(),
        visuals.weak_text_color(),
    );
    let mut x = text.max.x + 8.0;
    for (name, t) in [
        ("field", PortType::Field),
        ("colour", PortType::Vec3),
        ("brightness", PortType::F32),
    ] {
        let r = painter.text(
            pos2(x, y),
            egui::Align2::LEFT_CENTER,
            name,
            font.clone(),
            visuals.weak_text_color(),
        );
        painter.circle_filled(pos2(r.max.x + 7.0, y), 4.0, type_colour(t));
        x = r.max.x + 20.0;
    }
}

/// Draws node `d`: its box, its title, its state glyph, its ports and their labels, and the accent outline when
/// selected (§8).
fn node(
    ui: &mut Ui,
    painter: &egui::Painter,
    visuals: &egui::Visuals,
    g: &StainGraph,
    d: &Drawn,
    zoom: f32,
    selected: bool,
) {
    let Some(n) = g.node(d.id) else {
        return;
    };
    let r = d.rect;
    painter.rect_filled(r, 3.0 * zoom, visuals.window_fill);
    let stroke = if selected {
        Stroke::new(SELECTED_W, visuals.selection.stroke.color)
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    painter.rect_stroke(r, 3.0 * zoom, stroke, StrokeKind::Inside);
    let title_rect = d.title(zoom);
    painter.line_segment(
        [title_rect.left_bottom(), title_rect.right_bottom()],
        visuals.widgets.noninteractive.bg_stroke,
    );
    let (kind, name) = title(n.kind, &n.occupant);
    let size = 13.0 * zoom;
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("{kind} · "),
        0.0,
        egui::TextFormat::simple(FontId::proportional(size), visuals.weak_text_color()),
    );
    job.append(
        &name,
        0.0,
        egui::TextFormat::simple(FontId::proportional(size), visuals.strong_text_color()),
    );
    let label = title_rect.shrink2(vec2(10.0 * zoom, 0.0));
    ui.put(label, egui::Label::new(job).selectable(false).truncate());
    // A custom occupant's code is hand-written: the `{ }` badge (§8).
    if matches!(n.occupant, Occupant::Custom(_)) {
        painter.text(
            pos2(title_rect.max.x - 8.0 * zoom, title_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "{ }",
            FontId::monospace(11.0 * zoom),
            visuals.warn_fg_color,
        );
    }
    if has_glyph(n.kind) {
        let glyph = Rect::from_min_size(
            pos2(r.min.x + 12.0 * zoom, title_rect.max.y + 5.0 * zoom),
            vec2(r.width() - 24.0 * zoom, GLYPH_H * zoom - 10.0 * zoom),
        );
        state_glyph(painter, visuals, n, glyph, zoom);
    }
    let font = FontId::proportional(12.0 * zoom);
    for (i, label) in port_labels(n.kind, &d.ins).into_iter().enumerate() {
        let p = d.in_port(i, zoom);
        painter.circle_filled(p, PORT_R * zoom, type_colour(d.ins[i]));
        painter.text(
            p + vec2(12.0 * zoom, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            visuals.weak_text_color(),
        );
    }
    if let Some(t) = n.kind.output() {
        let p = d.out_port(zoom);
        painter.circle_filled(p, PORT_R * zoom, type_colour(t));
        painter.text(
            p - vec2(12.0 * zoom, 0.0),
            egui::Align2::RIGHT_CENTER,
            type_name(t),
            font,
            visuals.weak_text_color(),
        );
    }
}

/// The state glyph (§8): a source's field and a combiner's occupant as a dropdown's face, the outcome palette's
/// swatches as chips, a brightness as its ramp from full to its darkest, anything else as its occupant's name.
fn state_glyph(
    painter: &egui::Painter,
    visuals: &egui::Visuals,
    n: &engine::contract::render_state::Node,
    rect: Rect,
    zoom: f32,
) {
    let font = FontId::proportional(12.0 * zoom);
    let face = |text: &str| {
        painter.rect_filled(rect, 2.0, visuals.widgets.inactive.bg_fill);
        painter.text(
            rect.left_center() + vec2(8.0 * zoom, 0.0),
            egui::Align2::LEFT_CENTER,
            text,
            font.clone(),
            visuals.text_color(),
        );
        painter.text(
            rect.right_center() - vec2(8.0 * zoom, 0.0),
            egui::Align2::RIGHT_CENTER,
            "▾",
            font.clone(),
            visuals.text_color(),
        );
    };
    match (&n.kind, &n.occupant) {
        (NodeKind::Source, _) | (NodeKind::Combiner, _) => face(&title(n.kind, &n.occupant).1),
        (NodeKind::Colour, Occupant::Builtin(id)) if id == OUTCOME_OCCUPANT => {
            let w = rect.width() / SWATCHES.len() as f32;
            for (i, s) in SWATCHES.iter().enumerate() {
                let [r, gr, b] = s.srgb8;
                let chip = Rect::from_min_size(
                    pos2(rect.min.x + i as f32 * w, rect.min.y),
                    vec2(w - 1.0, rect.height()),
                );
                painter.rect_filled(chip, 0.0, Color32::from_rgb(r, gr, b));
            }
        }
        (NodeKind::Brightness, _) => {
            // A ramp, full to dark: what a brightness does, whatever its occupant.
            let steps = 24;
            let w = rect.width() / steps as f32;
            for i in 0..steps {
                let f = i as f32 / (steps - 1) as f32;
                let v = (255.0 * (1.0 - f * (1.0 - RAMP_DARKEST))).round() as u8;
                let chip = Rect::from_min_size(
                    pos2(rect.min.x + i as f32 * w, rect.min.y),
                    vec2(w + 0.5, rect.height()),
                );
                painter.rect_filled(chip, 0.0, Color32::from_gray(v));
            }
        }
        _ => {
            painter.text(
                rect.left_center(),
                egui::Align2::LEFT_CENTER,
                title(n.kind, &n.occupant).1,
                font,
                visuals.weak_text_color(),
            );
        }
    }
}
