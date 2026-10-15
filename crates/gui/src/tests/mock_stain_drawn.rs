//! `mock_stain_drawn` (REQ-GUI-175, REQ-GUI-131; render_gui_spec Part II §7, §8; 02_stain.png): the Stain canvas as
//! drawn, read from the frame's shapes — each node's box, title, state glyph, ports and their labels, scaled by the
//! zoom; the wires between the ports they join; the grid; the hint and its type colours; the toggle at the canvas's
//! top right — and the geometry they are drawn from. Each check also runs on an input it must reject (R-176, through
//! `rejects`).

use eframe::egui::epaint::{ColorMode, CornerRadius};
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Shape};
use engine::contract::legend::SWATCHES;
use engine::contract::render_state::{NodeId, NodeKind, Occupant, PortType, StainGraph};
use engine::contract::view_ui::Mode;

use super::support::{headless, mock_app, nodes, rejects, shapes};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::MockSide;
use crate::stain::canvas::{
    default_position, grid_dots, outcome_palette, size, type_colour, type_name, wire, wire_width,
    Drawn, Zoom, GLYPH_H, HINT, NODE_W, OUT_W, PORT_R, RAMP_DARKEST, ROW_H, SELECTED_W, TITLE_H,
};
use crate::stain::Regions;

/// The zoom the drawing is read at: no constant it scales equals itself under `+` or `/` there.
const Z: f32 = 3.0;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn close_pos(a: Pos2, b: Pos2) -> bool {
    close(a.x, b.x) && close(a.y, b.y)
}

fn close_rect(a: Rect, b: Rect) -> bool {
    close_pos(a.min, b.min) && close_pos(a.max, b.max)
}

/// The mock app on the Stain page.
fn stain_app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    app.view.mode = Mode::Stain;
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

/// The app's visuals, as the frame draws with.
fn visuals(h: &Headless) -> egui::Visuals {
    h.ctx.global_style().visuals.clone()
}

fn canvas_rect(h: &Headless) -> Rect {
    let ppp = h.pixels_per_point();
    Regions::new(&Layout::new(h.screen(), ppp), ppp, false).canvas
}

/// The frame's shapes with node `id` brought to the canvas's top left at zoom [`Z`], and the node as drawn.
fn at_zoom(app: &mut App<MockSide>, h: &mut Headless, id: NodeId) -> (Vec<Shape>, Drawn) {
    let place = app.stain.canvas.positions[&id];
    app.stain.canvas.view = Zoom {
        zoom: Z,
        pan: vec2(20.0, 60.0) - place.to_vec2() * Z,
    };
    let all = shapes(h, app);
    let d = app
        .stain
        .canvas
        .drawn(&app.snapshot().render.stain_graph, canvas_rect(h).min)
        .into_iter()
        .find(|d| d.id == id)
        .expect("drawn");
    (all, d)
}

/// A text drawn: its text, its rect on screen, its font size, whether it was cut short.
#[derive(Debug)]
struct Text {
    text: String,
    rect: Rect,
    size: f32,
    elided: bool,
}

fn texts(all: &[Shape]) -> Vec<Text> {
    all.iter()
        .filter_map(|s| match s {
            Shape::Text(t) => Some(Text {
                text: t.galley.text().to_owned(),
                rect: Rect::from_min_size(t.pos, t.galley.size()),
                size: t
                    .galley
                    .job
                    .sections
                    .first()
                    .map_or(0.0, |s| s.format.font_id.size),
                elided: t.galley.elided,
            }),
            _ => None,
        })
        .collect()
}

/// The text `text` drawn inside `within`.
fn text_in<'a>(all: &'a [Text], text: &str, within: Rect) -> &'a Text {
    all.iter()
        .find(|t| t.text == text && within.expand(1.0).contains(t.rect.center()))
        .unwrap_or_else(|| panic!("no {text:?} in {within:?}"))
}

fn rects(all: &[Shape]) -> Vec<egui::epaint::RectShape> {
    all.iter()
        .filter_map(|s| match s {
            Shape::Rect(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

fn circles(all: &[Shape]) -> Vec<(Pos2, f32, Color32)> {
    all.iter()
        .filter_map(|s| match s {
            Shape::Circle(c) => Some((c.center, c.radius, c.fill)),
            _ => None,
        })
        .collect()
}

/// The glyph's rect of node `d` at zoom `z` (§8): under the title, inset 12 points across and 5 down, its row less
/// 10.
fn glyph(d: &Drawn, z: f32) -> Rect {
    Rect::from_min_size(
        pos2(
            d.rect.min.x + 12.0 * z,
            d.rect.min.y + TITLE_H * z + 5.0 * z,
        ),
        vec2(d.rect.width() - 24.0 * z, GLYPH_H * z - 10.0 * z),
    )
}

/// Node `d`'s box, title rule, ports and port labels, at zoom [`Z`].
fn check_box(all: &[Shape], d: &Drawn, visuals: &egui::Visuals, selected: bool) {
    let rs = rects(all);
    let corner = CornerRadius::same((3.0 * Z) as u8);
    let fill = rs
        .iter()
        .find(|r| close_rect(r.rect, d.rect) && r.fill == visuals.window_fill)
        .expect("the box");
    assert_eq!(fill.corner_radius, corner);
    let stroke = if selected {
        egui::Stroke::new(SELECTED_W, visuals.selection.stroke.color)
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    let outline = rs
        .iter()
        .find(|r| close_rect(r.rect, d.rect) && r.fill == Color32::TRANSPARENT)
        .expect("the outline");
    assert_eq!((outline.stroke, outline.corner_radius), (stroke, corner));
    let rule = d.rect.min.y + TITLE_H * Z;
    assert!(
        all.iter().any(|s| matches!(s, Shape::LineSegment { points, .. }
            if close_pos(points[0], pos2(d.rect.min.x, rule)) && close_pos(points[1], pos2(d.rect.max.x, rule)))),
        "the title's rule"
    );
    let ts = texts(all);
    let cs = circles(all);
    for (i, &t) in d.ins.iter().enumerate() {
        let p = d.in_port(i, Z);
        assert!(
            cs.iter()
                .any(|c| close_pos(c.0, p) && close(c.1, PORT_R * Z) && c.2 == type_colour(t)),
            "in-port {i} at {p:?}"
        );
        let label = ts
            .iter()
            .find(|l| close(l.rect.min.x, p.x + 12.0 * Z) && close(l.rect.center().y, p.y))
            .unwrap_or_else(|| panic!("in-port {i}'s label"));
        assert_eq!(label.size, 12.0 * Z);
    }
    if let Some(t) = d.kind.output() {
        let p = d.out_port(Z);
        assert!(
            cs.iter()
                .any(|c| close_pos(c.0, p) && close(c.1, PORT_R * Z) && c.2 == type_colour(t)),
            "the out-port at {p:?}"
        );
        let label = ts
            .iter()
            .find(|l| l.text == type_name(t) && close(l.rect.max.x, p.x - 12.0 * Z))
            .expect("the out-port's label");
        assert!(close(label.rect.center().y, p.y));
        assert_eq!(label.size, 12.0 * Z);
    }
}

/// The face of a dropdown, the glyph of a source and a combiner: its fill over the glyph's rect, its value left, `▾`
/// right.
fn check_face(all: &[Shape], d: &Drawn, value: &str, visuals: &egui::Visuals) {
    let g = glyph(d, Z);
    assert!(
        rects(all)
            .iter()
            .any(|r| close_rect(r.rect, g) && r.fill == visuals.widgets.inactive.bg_fill),
        "the face over {g:?}"
    );
    let ts = texts(all);
    let v = text_in(&ts, value, g);
    assert!(close(v.rect.min.x, g.min.x + 8.0 * Z), "{v:?}");
    assert_eq!(v.size, 12.0 * Z);
    let arrow = text_in(&ts, "▾", g);
    assert!(close(arrow.rect.max.x, g.max.x - 8.0 * Z), "{arrow:?}");
}

#[test]
fn mock_stain_drawn_a_source_and_its_face() {
    let (mut app, mut h) = stain_app();
    // A long field: the title is cut short at the title bar's inset.
    let edit = crate::stain::edited(&app.snapshot().render.stain_graph, |g| {
        g.set_occupant(NodeId(2), Occupant::Field("diffusion_slope_valid".into()))
    })
    .expect("a source's field");
    app.set_field(edit);
    let _ = h.frame(&mut app, Vec::new());
    let (all, d) = at_zoom(&mut app, &mut h, NodeId(2));
    let visuals = visuals(&h);
    check_box(&all, &d, &visuals, false);
    check_face(&all, &d, "ctx.diffusion_slope_valid", &visuals);
    let ts = texts(&all);
    let title = ts
        .iter()
        .find(|t| t.text == "source · ctx.diffusion_slope_valid")
        .expect("the title");
    assert_eq!(title.size, 13.0 * Z);
    assert!(title.elided, "cut short at zoom {Z}: {title:?}");
    assert!(
        title.rect.width() <= d.rect.width() - 20.0 * Z + 0.5,
        "{title:?}"
    );
    rejects("a title past its inset", || {
        assert!(title.rect.width() <= d.rect.width() - 40.0 * Z)
    });
}

#[test]
fn mock_stain_drawn_the_palette_swatches() {
    let (mut app, mut h) = stain_app();
    let (all, d) = at_zoom(&mut app, &mut h, NodeId(4));
    let visuals = visuals(&h);
    check_box(&all, &d, &visuals, false);
    let g = glyph(&d, Z);
    let w = g.width() / SWATCHES.len() as f32;
    let rs = rects(&all);
    for (i, s) in SWATCHES.iter().enumerate() {
        let [r, gr, b] = s.srgb8;
        let chip = Rect::from_min_size(
            pos2(g.min.x + i as f32 * w, g.min.y),
            vec2(w - 1.0, g.height()),
        );
        assert!(
            rs.iter()
                .any(|x| close_rect(x.rect, chip) && x.fill == Color32::from_rgb(r, gr, b)),
            "swatch {i} at {chip:?}"
        );
    }
    let title = texts(&all)
        .into_iter()
        .find(|t| t.text == "colour · outcome_state")
        .expect("the title");
    assert!(!title.elided);
    // Selected: the accent outline.
    app.stain.canvas.selection = [NodeId(4)].into();
    let (all, d) = at_zoom(&mut app, &mut h, NodeId(4));
    check_box(&all, &d, &visuals, true);
    rejects("a selected node's plain outline", || {
        check_box(&all, &d, &visuals, false)
    });
}

#[test]
fn mock_stain_drawn_the_brightness_ramp_and_badge() {
    let (mut app, mut h) = stain_app();
    let (all, d) = at_zoom(&mut app, &mut h, NodeId(5));
    let visuals = visuals(&h);
    check_box(&all, &d, &visuals, false);
    let g = glyph(&d, Z);
    let steps = 24;
    let w = g.width() / steps as f32;
    let rs = rects(&all);
    let grey = |i: usize| {
        let f = i as f32 / (steps - 1) as f32;
        (255.0 * (1.0 - f * (1.0 - RAMP_DARKEST))).round() as u8
    };
    assert_eq!((grey(0), grey(steps - 1)), (255, 64));
    for i in 0..steps {
        let chip = Rect::from_min_size(
            pos2(g.min.x + i as f32 * w, g.min.y),
            vec2(w + 0.5, g.height()),
        );
        assert!(
            rs.iter()
                .any(|x| close_rect(x.rect, chip) && x.fill == Color32::from_gray(grey(i))),
            "ramp step {i} at {chip:?}"
        );
    }
    let title_rect = Rect::from_min_size(d.rect.min, vec2(d.rect.width(), TITLE_H * Z));
    let ts = texts(&all);
    let badge = text_in(&ts, "{ }", title_rect);
    assert!(
        close(badge.rect.max.x, title_rect.max.x - 8.0 * Z),
        "{badge:?}"
    );
    assert!(close(badge.rect.center().y, title_rect.center().y));
    assert_eq!(badge.size, 11.0 * Z);
    rejects("a ramp to black", || assert_eq!(grey(steps - 1), 0));
}

#[test]
fn mock_stain_drawn_the_combiner_and_out() {
    let (mut app, mut h) = stain_app();
    let visuals = visuals(&h);
    let (all, d) = at_zoom(&mut app, &mut h, NodeId(0));
    check_box(&all, &d, &visuals, false);
    check_face(&all, &d, "multiply", &visuals);
    let ts = texts(&all);
    for (i, label) in ["colour", "brightness"].into_iter().enumerate() {
        let p = d.in_port(i, Z);
        let l = ts
            .iter()
            .find(|t| t.text == label && close(t.rect.min.x, p.x + 12.0 * Z))
            .unwrap_or_else(|| panic!("{label}"));
        assert!(close(l.rect.center().y, p.y));
    }
    let (all, d) = at_zoom(&mut app, &mut h, NodeId(1));
    check_box(&all, &d, &visuals, false);
    assert_eq!(d.rect.width(), OUT_W * Z);
    // OUT has no glyph: no face under its title.
    assert!(!rects(&all)
        .iter()
        .any(|r| r.fill == visuals.widgets.inactive.bg_fill && d.rect.contains_rect(r.rect)));
    rejects("a face on OUT", || {
        check_face(&all, &d, "multiply", &visuals)
    });
}

/// The wires join the ports their graph joins, in the type's colour, at the zoom's width.
#[test]
fn mock_stain_drawn_the_wires() {
    let (mut app, mut h) = stain_app();
    let (all, _) = at_zoom(&mut app, &mut h, NodeId(2));
    let g: StainGraph = app.snapshot().render.stain_graph.clone();
    let drawn = app.stain.canvas.drawn(&g, canvas_rect(&h).min);
    let beziers: Vec<_> = all
        .iter()
        .filter_map(|s| match s {
            Shape::CubicBezier(b) => Some(b.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(beziers.len(), g.wires().len());
    for w in g.wires() {
        let find = |id| drawn.iter().find(|d| d.id == id).unwrap();
        let (from, to) = (find(w.from), find(w.to));
        let port = w.port as usize;
        let points = wire(from.out_port(Z), to.in_port(port, Z), Z);
        let b = beziers
            .iter()
            .find(|b| (0..4).all(|i| close_pos(b.points[i], points[i])))
            .unwrap_or_else(|| panic!("the wire {w:?}"));
        assert_eq!(b.stroke.width, wire_width(Z));
        assert_eq!(b.stroke.color, ColorMode::Solid(type_colour(to.ins[port])));
    }
    rejects("a wire from the in-port", || {
        let w = g.wires()[0].clone();
        let to = drawn.iter().find(|d| d.id == w.to).unwrap();
        let p = to.in_port(w.port as usize, Z);
        assert!(beziers.iter().any(|b| close_pos(b.points[0], p)));
    });
}

#[test]
fn mock_stain_drawn_wire_geometry() {
    // Half the horizontal distance, at least 30 points of the canvas.
    let (a, b) = (pos2(20.0, 0.0), pos2(100.0, 50.0));
    assert_eq!(wire(a, b, 1.0), [a, pos2(60.0, 0.0), pos2(60.0, 50.0), b]);
    assert_eq!(wire(a, b, 3.0), [a, pos2(110.0, 0.0), pos2(10.0, 50.0), b]);
    // Backwards: the same bend.
    assert_eq!(wire(b, a, 1.0)[1], pos2(140.0, 50.0));
    assert_eq!(
        (wire_width(3.0), wire_width(1.0), wire_width(0.25)),
        (6.0, 2.0, 1.0)
    );
    rejects("a bend of the whole distance", || {
        assert_eq!(wire(a, b, 1.0)[1], pos2(100.0, 0.0))
    });
}

#[test]
fn mock_stain_drawn_grid_dots() {
    let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
    let view = |pan: (f32, f32), zoom: f32| Zoom {
        pan: vec2(pan.0, pan.1),
        zoom,
    };
    let xs = |dots: &[Pos2]| {
        let mut v: Vec<f32> = dots.iter().map(|p| p.x).collect();
        v.dedup();
        v.sort_by(f32::total_cmp);
        v.dedup();
        v
    };
    let ys = |dots: &[Pos2]| {
        let mut v: Vec<f32> = dots.iter().map(|p| p.y).collect();
        v.sort_by(f32::total_cmp);
        v.dedup();
        v
    };
    // Every 24 points from the pan's offset; a dot on the right or bottom edge is left out.
    let dots = grid_dots(rect, view((4.0, 2.0), 1.0));
    assert_eq!(xs(&dots), [4.0, 28.0, 52.0, 76.0]);
    assert_eq!(ys(&dots), [2.0, 26.0]);
    assert_eq!(dots.len(), 8);
    // A pan past a step: its remainder; negative too.
    assert_eq!(grid_dots(rect, view((-20.0, 50.0), 1.0))[0], pos2(4.0, 2.0));
    // At zoom 2, every 48; the rect's corner offsets them.
    let off = Rect::from_min_max(pos2(10.0, 5.0), pos2(110.0, 55.0));
    let dots = grid_dots(off, view((0.0, 0.0), 2.0));
    assert_eq!(xs(&dots), [10.0, 58.0, 106.0]);
    assert_eq!(ys(&dots), [5.0, 53.0]);
    // Six points apart at the least zoom: drawn; closer, none.
    assert_eq!(grid_dots(rect, view((0.0, 0.0), 0.25)).len(), 17 * 9);
    assert!(grid_dots(rect, view((0.0, 0.0), 0.2)).is_empty());
    rejects("a dot on the edge", || {
        assert!(grid_dots(rect, view((4.0, 2.0), 1.0)).contains(&pos2(100.0, 2.0)))
    });
}

/// The canvas's grid as drawn: the dots of [`grid_dots`] for its rect and view.
#[test]
fn mock_stain_drawn_the_grid_on_the_canvas() {
    let (mut app, mut h) = stain_app();
    let (all, _) = at_zoom(&mut app, &mut h, NodeId(2));
    let rect = canvas_rect(&h);
    let want = grid_dots(rect, app.stain.canvas.view);
    let dot = visuals(&h).widgets.noninteractive.bg_stroke.color;
    let got: Vec<Pos2> = circles(&all)
        .into_iter()
        .filter(|c| c.1 == 1.0 && c.2 == dot)
        .map(|c| c.0)
        .collect();
    assert_eq!(got, want);
    rejects("no grid", || assert!(got.is_empty()));
}

/// The hint along the canvas's foot, over its strip, and each type's name and dot after it.
#[test]
fn mock_stain_drawn_the_hint() {
    let (mut app, mut h) = stain_app();
    let all = shapes(&mut h, &mut app);
    let rect = canvas_rect(&h);
    let visuals = visuals(&h);
    let y = rect.max.y - 14.0;
    let strip = Rect::from_min_max(pos2(rect.min.x, y - 10.0), rect.max);
    assert!(rects(&all)
        .iter()
        .any(|r| close_rect(r.rect, strip) && r.fill == visuals.extreme_bg_color));
    let ts = texts(&all);
    let hint = text_in(&ts, HINT, strip);
    assert!(close(hint.rect.min.x, rect.min.x + 12.0) && close(hint.rect.center().y, y));
    let mut left = hint.rect.max.x + 8.0;
    let cs = circles(&all);
    for (name, t) in [
        ("field", PortType::Field),
        ("colour", PortType::Vec3),
        ("brightness", PortType::F32),
    ] {
        let label = ts
            .iter()
            .find(|l| l.text == name && close(l.rect.center().y, y))
            .unwrap_or_else(|| panic!("{name}"));
        assert!(
            close(label.rect.min.x, left),
            "{name}: {label:?} from {left}"
        );
        let dot = pos2(label.rect.max.x + 7.0, y);
        assert!(
            cs.iter()
                .any(|c| close_pos(c.0, dot) && c.1 == 4.0 && c.2 == type_colour(t)),
            "{name}'s dot"
        );
        left = label.rect.max.x + 20.0;
    }
    rejects("the hint at the canvas's top", || {
        assert!(close(hint.rect.center().y, rect.min.y + 14.0))
    });
}

/// The toggle at the canvas's top right, the view shown selected.
#[test]
fn mock_stain_drawn_the_toggle() {
    let (mut app, mut h) = stain_app();
    let rect = canvas_rect(&h);
    let all = nodes(&mut h, &mut app);
    let button = |label: &str| {
        all.iter()
            .find(|n| n.text == label)
            .unwrap_or_else(|| panic!("{label}"))
            .rect
    };
    let node = button("Node WGSL");
    // Within half a point: AccessKit's bounds are the button's, rounded to the pixel.
    assert!(
        (node.max.x - (rect.max.x - 12.0)).abs() <= 0.5,
        "{node:?} {rect:?}"
    );
    assert!(
        (node.center().y - (rect.min.y + 8.0 + 11.0)).abs() <= 0.5,
        "{node:?} {rect:?}"
    );
    let (graph, pipeline) = (button("Graph"), button("Pipeline WGSL"));
    assert!(graph.max.x <= pipeline.min.x && pipeline.max.x <= node.min.x);
    let fill = visuals(&h).selection.bg_fill;
    let drawn = rects(&shapes(&mut h, &mut app));
    let filled = |r: Rect| {
        drawn
            .iter()
            .any(|s| s.fill == fill && r.contains(s.rect.center()))
    };
    assert!(filled(graph) && !filled(pipeline) && !filled(node));
    rejects("Pipeline WGSL shown selected", || assert!(filled(pipeline)));
}

/// A node's size and its ports' places at zoom 1 and [`Z`].
#[test]
fn mock_stain_drawn_node_geometry() {
    // A title, a glyph, a row per in-port and one for the out-port, and the pad.
    assert_eq!(
        size(NodeKind::Source, 0),
        vec2(NODE_W, 26.0 + 28.0 + 22.0 + 6.0)
    );
    assert_eq!(
        size(NodeKind::Colour, 1),
        vec2(NODE_W, 26.0 + 28.0 + 44.0 + 6.0)
    );
    assert_eq!(
        size(NodeKind::Combiner, 2),
        vec2(NODE_W, 26.0 + 28.0 + 66.0 + 6.0)
    );
    assert_eq!(size(NodeKind::Out, 1), vec2(OUT_W, 26.0 + 22.0 + 6.0));
    let drawn = |kind, ins: Vec<PortType>| Drawn {
        id: NodeId(0),
        kind,
        rect: Rect::from_min_size(pos2(10.0, 20.0), size(kind, ins.len()) * Z),
        ins,
    };
    let c = drawn(NodeKind::Combiner, vec![PortType::Vec3, PortType::F32]);
    assert_eq!(
        c.in_port(0, Z),
        pos2(10.0, 20.0 + (26.0 + 28.0 + 0.5 * ROW_H) * Z)
    );
    assert_eq!(
        c.in_port(1, Z),
        pos2(10.0, 20.0 + (26.0 + 28.0 + 1.5 * ROW_H) * Z)
    );
    assert_eq!(
        c.out_port(Z),
        pos2(10.0 + NODE_W * Z, 20.0 + (26.0 + 28.0 + 2.5 * ROW_H) * Z)
    );
    let o = drawn(NodeKind::Out, vec![PortType::Vec3]);
    assert_eq!(o.in_port(0, Z), pos2(10.0, 20.0 + (26.0 + 0.5 * ROW_H) * Z));
    assert_eq!(
        o.title(Z),
        Rect::from_min_size(pos2(10.0, 20.0), vec2(OUT_W * Z, TITLE_H * Z))
    );
    rejects("the ports under the title, past the glyph", || {
        assert_eq!(o.in_port(0, Z), c.in_port(0, Z))
    });
}

/// Only the outcome palette's colour draws swatches.
#[test]
fn mock_stain_drawn_outcome_palette() {
    let palette = Occupant::Builtin("outcome_state".into());
    assert!(outcome_palette(NodeKind::Colour, &palette));
    assert!(!outcome_palette(
        NodeKind::Colour,
        &Occupant::Builtin("other".into())
    ));
    assert!(!outcome_palette(
        NodeKind::Colour,
        &Occupant::Custom(String::new())
    ));
    assert!(!outcome_palette(NodeKind::Post, &palette));
    rejects("another built-in read as the palette", || {
        assert!(outcome_palette(
            NodeKind::Colour,
            &Occupant::Builtin("other".into())
        ))
    });
}

/// The `click_node` step's point: inside the colour node's title bar at its default place.
#[test]
fn mock_stain_drawn_click_node_point() {
    let (app, h) = stain_app();
    let ppp = h.pixels_per_point();
    let p = crate::capture::node_point(&Layout::new(h.screen(), ppp), ppp);
    let d = app
        .stain
        .canvas
        .drawn(&app.snapshot().render.stain_graph, canvas_rect(&h).min)
        .into_iter()
        .find(|d| d.id == NodeId(4))
        .unwrap();
    let title = d.title(1.0);
    assert!(title.shrink(1.0).contains(p), "{p:?} in {title:?}");
    assert_eq!(
        app.stain.canvas.positions[&NodeId(4)],
        default_position(NodeKind::Colour, 0)
    );
    rejects("a point under the title", || {
        assert!(title.shrink(1.0).contains(p + vec2(0.0, TITLE_H)))
    });
}
