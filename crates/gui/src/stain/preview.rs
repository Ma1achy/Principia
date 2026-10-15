//! The preview and the node inspector (render_gui_spec Part II §1, §9; GUI_DESIGN_NOTES § "Rules that hold
//! everywhere", § "02 Stain"; 02_stain.png), right. The preview is the engine side's, drawn through the canvas as the
//! figure is, live: it is redrawn from the graph as it stands, so an edit shows at once. It keeps the viewport's aspect
//! (§G1: "Every preview of the slice keeps the viewport's aspect ratio — square for the slice"), the Explore figure's
//! rect, fitted into its area, never stretched or cropped. Under it the inspector shows the selected node: its title
//! and what it maps, its choices (a source's field, the combiner's occupant) and its scalar params, each edit one
//! `SetField` of the graph (a slider's drag one, when it ends, R-96), the outcome palette's swatches, and its shader's
//! actions; with no node selected, the graph's. The full morphing inspector is TASK-M7-23's and TASK-M8-21's.

use eframe::egui::{
    self, pos2, vec2, Button, Color32, Rect, RichText, Slider, Ui, UiBuilder, Vec2,
};
use eframe::egui_wgpu;
use engine::contract::legend::SWATCHES;
use engine::contract::render_state::{NodeKind, Occupant};

use super::{
    canvas, choices, edited, params, Out, Page, Scratch, CHOICE_IDS, EDIT_WGSL, PARAM_IDS, REVERT,
};
use crate::keyboard::scopes::ScopeId;
use crate::windows::Keys;

/// The preview's area's height, in points (02_stain.png's, a look choice, R-390).
pub const PREVIEW_H: f32 = 190.0;

/// The largest rect of `viewport`'s aspect inside `area`, centred across it at its top: a square viewport gives a
/// square; an empty one, a square too.
pub fn fit(area: Rect, viewport: Vec2) -> Rect {
    let aspect = if viewport.x > 0.0 && viewport.y > 0.0 {
        viewport.x / viewport.y
    } else {
        1.0
    };
    let (w, h) = if area.width() / area.height() > aspect {
        (area.height() * aspect, area.height())
    } else {
        (area.width(), area.width() / aspect)
    };
    Rect::from_min_size(pos2(area.center().x - w / 2.0, area.min.y), vec2(w, h))
}

/// What node `kind` with `occupant` maps, as the inspector's second line reads.
pub fn maps(kind: NodeKind, occupant: &Occupant) -> String {
    if canvas::outcome_palette(kind, occupant) {
        return "maps outcome class [categorical] → vec3".to_owned();
    }
    match (kind, occupant) {
        (NodeKind::Source, Occupant::Field(f)) => format!("reads ctx.{f} → field"),
        (NodeKind::Colour, _) => "maps its field → vec3".to_owned(),
        (NodeKind::Brightness, _) => "maps its field → f32".to_owned(),
        (NodeKind::Combiner, _) => "colour · brightness → vec3".to_owned(),
        (NodeKind::Post, _) => "vec3 → vec3".to_owned(),
        (NodeKind::Out, _) => "the stain's output, to the style (Display)".to_owned(),
        (NodeKind::Source, _) => "None: no field".to_owned(),
    }
}

/// Draws the preview and the inspector in `rect`.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    page: &Page<'_>,
    s: &mut Scratch,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
    out: &mut Out,
) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        visuals.widgets.noninteractive.bg_stroke,
    );
    let inner = rect.shrink2(vec2(12.0, 10.0));
    let mut panel = ui.new_child(UiBuilder::new().max_rect(inner));
    panel.label(RichText::new("Preview").heading());
    panel.add_space(4.0);
    let (area, _) = panel.allocate_exact_size(vec2(inner.width(), PREVIEW_H), egui::Sense::hover());
    let shown = fit(area, page.viewport.size());
    match &page.preview {
        Some((canvas, format)) => {
            panel.painter().add(egui_wgpu::Callback::new_paint_callback(
                shown,
                crate::app::FigureCallback {
                    canvas: canvas.clone(),
                    format: *format,
                },
            ));
        }
        None => {
            panel
                .painter()
                .rect_filled(shown, 0.0, visuals.extreme_bg_color);
        }
    }
    s.preview = Some(shown);
    panel.add_space(6.0);
    panel.separator();
    egui::ScrollArea::vertical()
        .id_salt("stain_inspector")
        .auto_shrink(false)
        .show(&mut panel, |ui| inspector(ui, page, s, keys, clicked, out));
}

/// The inspector: the selected node's, or the graph's.
fn inspector(
    ui: &mut Ui,
    page: &Page<'_>,
    s: &mut Scratch,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
    out: &mut Out,
) {
    let g = page.graph;
    let Some(n) = s.selected().and_then(|id| g.node(id)) else {
        let selected = s.canvas.selection.len();
        let heading = if selected > 1 {
            format!("{selected} nodes selected")
        } else {
            "Graph".to_owned()
        };
        ui.label(RichText::new(heading).heading());
        let nodes = g.nodes().count();
        ui.label(RichText::new(format!("{nodes} nodes · {} wires", g.wires().len())).weak());
        ui.label(RichText::new("select one node to edit it").weak());
        return;
    };
    let (kind, name) = canvas::title(n.kind, &n.occupant);
    let mut kind_title = kind.to_owned();
    if let Some(first) = kind_title.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    ui.label(RichText::new(format!("{kind_title} · {name}")).heading());
    ui.label(RichText::new(maps(n.kind, &n.occupant)).weak());
    ui.add_space(6.0);
    if let Some(c) = page.content {
        let options = choices(n, c);
        if !options.is_empty() {
            ui.label(
                RichText::new(if n.kind == NodeKind::Source {
                    "▾ Field"
                } else {
                    "▾ Mode"
                })
                .strong(),
            );
            ui.horizontal_wrapped(|ui| {
                for (i, (occupant, label)) in options.into_iter().enumerate().take(CHOICE_IDS.len())
                {
                    let button = ui.add(Button::selectable(n.occupant == occupant, label));
                    if button.clicked() {
                        clicked.push(CHOICE_IDS[i]);
                    }
                    keys.place(CHOICE_IDS[i], &button);
                }
            });
            ui.add_space(6.0);
        }
    }
    let id = n.id;
    let ps = params(g, id);
    if !ps.is_empty() {
        ui.label(RichText::new("▾ Params").strong());
        for (i, (name, value, lo, hi)) in ps.into_iter().enumerate().take(PARAM_IDS.len()) {
            let mut v = match &s.param_drag {
                Some((node, dragged, v)) if *node == id && *dragged == name => *v,
                _ => value,
            };
            let slider = ui.add(Slider::new(&mut v, lo..=hi).text(name.as_str()));
            keys.place(PARAM_IDS[i], &slider);
            if slider.dragged() {
                s.param_drag = Some((id, name.clone(), v));
            }
            if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
                s.param_drag = None;
                out.edits
                    .extend(edited(g, |g| g.set_param(id, &name, vec![v])));
            }
        }
        ui.add_space(6.0);
    }
    if canvas::outcome_palette(n.kind, &n.occupant) {
        ui.label(RichText::new("▾ Palette").strong());
        for swatch in SWATCHES {
            let [r, gr, b] = swatch.srgb8;
            ui.horizontal(|ui| {
                let (chip, _) = ui.allocate_exact_size(vec2(18.0, 14.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(chip, 1.0, Color32::from_rgb(r, gr, b));
                ui.label(swatch.label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("#{r:02x}{gr:02x}{b:02x}"))
                            .monospace()
                            .weak(),
                    );
                });
            });
        }
        ui.add_space(6.0);
    }
    if n.kind != NodeKind::Out {
        ui.label(RichText::new("▾ Shader").strong());
        ui.horizontal(|ui| {
            let edit = ui.button("Edit this node's WGSL");
            if edit.clicked() {
                clicked.push(EDIT_WGSL);
            }
            keys.place(EDIT_WGSL, &edit);
            let revert = ui
                .add_enabled(false, Button::new("Revert"))
                .on_disabled_hover_text("TASK-M7-24");
            keys.place(REVERT, &revert);
        });
    }
}
