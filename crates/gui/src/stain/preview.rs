//! The preview and the node inspector (render_gui_spec Part II §1, §9; GUI_DESIGN_NOTES § "Rules that hold
//! everywhere", § "02 Stain"; 02_stain.png), right. The preview is the engine side's, drawn through the canvas as the
//! figure is, live: it is redrawn from the graph as it stands, so an edit shows at once. It keeps the viewport's aspect
//! (§G1: "Every preview of the slice keeps the viewport's aspect ratio — square for the slice"), the Explore figure's
//! rect, fitted into its area, never stretched or cropped. Under it the inspector shows the selected node from its
//! description: its title, what it is and maps, the shaders of its kind, its controls ([`controls`]) and its shader's
//! actions, each edit one `SetField` of the graph (a slider's drag applied every frame its value changes, one history
//! entry); with no node selected, the graph's.

use std::collections::BTreeSet;

use eframe::egui::{
    self, pos2, vec2, Align, Button, FontSelection, Rect, RichText, Ui, UiBuilder, Vec2,
};
use eframe::egui_wgpu;
use engine::contract::render_state::NodeKind;
use engine::contract::stain::describe::{Description, Standing};

use super::{
    canvas, choices, chosen, controls, description, standing, yours, Out, Page, Scratch,
    CHOICE_IDS, EDIT_WGSL, REVERT,
};
use crate::keyboard::scopes::ScopeId;
use crate::windows::Keys;

/// The preview's area's height, in points (02_stain.png's, a look choice, R-390).
pub const PREVIEW_H: f32 = 190.0;

/// The largest rect of `viewport`'s aspect inside `area`, centred across it at its top: a square viewport gives a
/// square; an empty one, a square too.
pub fn fit(area: Rect, viewport: Vec2) -> Rect {
    let v = if viewport.x > 0.0 && viewport.y > 0.0 {
        viewport
    } else {
        Vec2::splat(1.0)
    };
    // The largest scale at which both sides fit.
    let size = v * (area.width() / v.x).min(area.height() / v.y);
    Rect::from_min_size(pos2(area.center().x - size.x / 2.0, area.min.y), size)
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

/// Why none of `section`'s controls can matter now, in words, where it is the same for every one of them.
fn section_words(section: &controls::Section, d: &Description, all: &[Standing]) -> Option<String> {
    let mut each = section
        .controls
        .iter()
        .map(|(k, _)| standing::words(&d.controls[*k], all));
    let first = each.next()??;
    each.all(|w| w.as_ref() == Some(&first)).then_some(first)
}

/// A section's heading: its title and, where its controls share them, the words of why none can matter.
fn heading(ui: &mut Ui, title: &str, words: Option<&str>) -> egui::Response {
    let mut job = egui::text::LayoutJob::default();
    let style = ui.style().clone();
    let (font, strong) = (FontSelection::Default, ui.visuals().strong_text_color());
    RichText::new(title)
        .color(strong)
        .append_to(&mut job, &style, font.clone(), Align::Center);
    if let Some(words) = words {
        RichText::new(format!(" · {words}"))
            .small()
            .weak()
            .append_to(&mut job, &style, font, Align::Center);
    }
    ui.add(Button::new(job).frame(false).wrap())
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
        // No node, or several: the one-node case is the node's.
        let heading = if s.canvas.selection.is_empty() {
            "Graph".to_owned()
        } else {
            format!("{} nodes selected", s.canvas.selection.len())
        };
        ui.label(RichText::new(heading).heading());
        let nodes = g.nodes().count();
        ui.label(RichText::new(format!("{nodes} nodes · {} wires", g.wires().len())).weak());
        ui.label(RichText::new("select one node to edit it").weak());
        if let (true, Some(c)) = (s.canvas.selection.is_empty(), page.content) {
            yours::panel(ui, &c.yours, &s.yours, keys, clicked);
        }
        return;
    };
    let d = description(page.content, n);
    let mut title = canvas::title(n.kind, &d);
    if let Some(first) = title.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    ui.label(RichText::new(title).heading());
    if !d.about.is_empty() {
        ui.label(&d.about);
    }
    ui.label(RichText::new(&d.maps).weak());
    ui.add_space(6.0);
    if let Some(c) = page.content {
        yours::offer(ui, &c.yours, n.id, keys, clicked);
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
                for (choice, id) in options.into_iter().zip(CHOICE_IDS.iter()) {
                    let button = ui
                        .add(Button::selectable(chosen(n, choice, c), choice.name))
                        .on_hover_text(&choice.from);
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
            });
            ui.add_space(6.0);
        }
    }
    let open: BTreeSet<ScopeId> = s
        .open
        .iter()
        .filter(|(of, _)| *of == n.id)
        .map(|(_, scope)| *scope)
        .collect();
    let standing = page.content.and_then(|c| c.standing.get(&n.id));
    let standing: &[Standing] = standing.map_or(&[], |s| s);
    let mut edits = Vec::new();
    for section in controls::sections(&d) {
        let folded = s.folded.contains(&section.title);
        let arrow = if folded { "▸" } else { "▾" };
        let shared = section_words(&section, &d, standing);
        let heading = heading(ui, &format!("{arrow} {}", section.title), shared.as_deref());
        if heading.clicked() {
            clicked.push(section.scope.id);
        }
        keys.place(section.scope.id, &heading);
        if !folded {
            let mut draw = controls::Draw {
                focus: page.focus,
                open: &open,
                keys,
                clicked,
                edits: &mut edits,
                standing,
                shared: shared.is_some(),
            };
            for (k, part) in &section.controls {
                controls::show(ui, &d.controls[*k], part, &mut draw);
            }
        }
        ui.add_space(6.0);
    }
    for (uniform, value) in edits {
        out.edits.extend(s.set_param(g, n.id, &uniform, value));
    }
    if n.kind != NodeKind::Out {
        ui.label(RichText::new("▾ Shader").strong());
        ui.horizontal(|ui| {
            let edit = ui.button("Edit this node's WGSL");
            if edit.clicked() {
                clicked.push(EDIT_WGSL);
            }
            keys.place(EDIT_WGSL, &edit);
            let revert = page
                .content
                .is_some_and(|c| c.yours.held(n.id).is_some_and(|h| h.revert));
            let revert = ui
                .add_enabled(revert, Button::new("Revert"))
                .on_disabled_hover_text("this node's code is not an edit of one here");
            if revert.clicked() {
                clicked.push(REVERT);
            }
            keys.place(REVERT, &revert);
        });
        if let Some(c) = page.content {
            yours::shader(ui, &c.yours, &mut s.yours, n.id, keys, clicked);
        }
    }
}
