//! The inspector's controls, drawn from a node's description: every control of the fixed set
//! (`render::assemble::declare`), in sections by control group, each section folding. One list, [`sections`], is what
//! is drawn, what joins the keyboard's scopes and what a click, Enter or an arrow acts on, so none can be drawn and not
//! reached. Every widget is drawn in the panel, none in a popup: a popup's widgets are on a layer of their own, which
//! the scopes do not reach.
//!
//! Enter on a toggle, a choice's value or a flag acts as its click; on a colour, a gradient or a vector it goes into
//! its channels, stops or components; and on a number it starts interaction, in which the arrows step it, ×10 with
//! Shift and ×0.1 with Alt. A step is a hundredth of a slider's range (of its logarithm's, a log slider's; one at
//! least, of whole numbers), one of a stepper or of an 8-bit channel, and a degree of an angle.
//!
//! A control that cannot change the picture now is dimmed, with why in words under it, or on its section's heading where
//! every control there shares them ([`standing`]), and acts as it always does.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use eframe::egui::{
    self, vec2, Button, Color32, DragValue, Rect, RichText, Sense, Slider, Stroke, Ui, WidgetInfo,
    WidgetType,
};
use engine::contract::stain::assembled::source_fields;
use engine::contract::stain::describe::{Control, ControlView, Description, Standing, UniformType};

use super::{intern, standing};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, StepKind};
use crate::windows::Keys;

/// The section of the controls that name no group.
pub const UNGROUPED: &str = "Params";

/// One scope of the inspector's controls, what it does, and the scopes inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub scope: Scope,
    pub act: Act,
    pub parts: Vec<Part>,
}

/// What a [`Part`] does.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// Nothing itself: Enter goes into its parts, and a click opens or closes them.
    Open,
    /// Enter or a click gives the uniform the value.
    Set(String, Vec<f64>),
    /// The arrows step a component.
    Step(Step),
}

/// A component the arrows step.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub uniform: String,
    value: Vec<f64>,
    at: usize,
    ty: UniformType,
    range: Option<(f64, f64)>,
    by: By,
}

/// How a component steps.
#[derive(Clone, Copy, Debug, PartialEq)]
enum By {
    /// By a share of its range.
    Share,
    /// By a share of its range in the logarithm.
    Log,
    /// By one.
    One,
    /// By a degree, the value in radians.
    Degree,
    /// By one 8-bit sRGB code, the value linear.
    Code,
    /// By a share of this: it has no range.
    Of(f64),
}

/// A section: the controls of one group.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub title: String,
    /// Its heading's scope.
    pub scope: Scope,
    /// Its controls, each its place in the description's and its part.
    pub controls: Vec<(usize, Part)>,
}

/// `d`'s controls in sections, each group where its first control is. A hidden control is in none.
pub fn sections(d: &Description) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for (k, c) in d.controls.iter().enumerate() {
        let Some(part) = part(k, c) else {
            continue;
        };
        let title = c.group.as_deref().unwrap_or(UNGROUPED);
        match out.iter_mut().find(|s| s.title == title) {
            Some(s) => s.controls.push((k, part)),
            None => {
                let id = intern(&format!("stain_insp_section_{}", out.len()));
                out.push(Section {
                    title: title.to_owned(),
                    scope: Scope::control(id, intern(title)),
                    controls: vec![(k, part)],
                });
            }
        }
    }
    out
}

/// The part `id` in `sections`.
pub fn find<'a>(sections: &'a [Section], id: &str) -> Option<&'a Part> {
    fn within<'a>(p: &'a Part, id: &str) -> Option<&'a Part> {
        if p.scope.id == id {
            return Some(p);
        }
        p.parts.iter().find_map(|q| within(q, id))
    }
    sections
        .iter()
        .flat_map(|s| &s.controls)
        .find_map(|(_, p)| within(p, id))
}

/// The scope of the control at place `k` of a description's.
pub fn scope(k: usize) -> ScopeId {
    intern(&format!("stain_insp_param_{k}"))
}

/// The scope of part `j` of `of`.
fn inner(of: ScopeId, j: usize) -> ScopeId {
    intern(&format!("{of}_{j}"))
}

fn whole(ty: UniformType) -> bool {
    matches!(ty, UniformType::I32 | UniformType::U32)
}

/// What the arrows move a component of `c` by where its control does not say.
fn by(c: &ControlView) -> By {
    if whole(c.ty) {
        return By::One;
    }
    match c.range {
        Some(_) => By::Share,
        // The plan is unclear here; chose a share of the default's size, at least 1, because it has no range to take
        // one of.
        None => By::Of(c.default.iter().fold(1.0, |m: f64, d| m.max(d.abs()))),
    }
}

/// Control `c`, at place `k`, as scopes; none where it is hidden.
fn part(k: usize, c: &ControlView) -> Option<Part> {
    let id = scope(k);
    let label = intern(&c.label);
    let step = |at: usize, by: By| {
        Act::Step(Step {
            uniform: c.uniform.clone(),
            value: c.value.clone(),
            at,
            ty: c.ty,
            range: c.range,
            by,
        })
    };
    let value = |kind: StepKind, by: By| Part {
        scope: Scope::value(id, label, kind),
        act: step(0, by),
        parts: Vec::new(),
    };
    let group = |parts: Vec<Part>| Part {
        scope: Scope::group(id, label),
        act: Act::Open,
        parts,
    };
    let sets = |names: &[String], of: &dyn Fn(usize) -> f64| {
        names
            .iter()
            .enumerate()
            .map(|(j, name)| Part {
                scope: Scope::control(inner(id, j), intern(name)),
                act: Act::Set(c.uniform.clone(), vec![of(j)]),
                parts: Vec::new(),
            })
            .collect::<Vec<_>>()
    };
    Some(match &c.control {
        // The editors of version 2's types (a gradient with its own stops, a list, a lut and an image) are not drawn
        // yet: their value is more than a few numbers, and the editor for each is its own piece of work.
        Control::Hidden | Control::List | Control::Curve | Control::ImagePicker => return None,
        // The plan is unclear here; chose to list every field, though a `field` declared without `any` takes only its
        // default's kind, because the description does not say which: the graph refuses the others, and nothing changes.
        Control::FieldPicker => group(sets(field_names(), &|j| j as f64)),
        Control::Gradient(stops) if stops.is_empty() => return None,
        Control::Slider => value(StepKind::Bounded, By::Share),
        Control::LogSlider => value(StepKind::Bounded, By::Log),
        Control::Stepper => value(StepKind::Bounded, By::One),
        Control::Angle => value(StepKind::Angle, By::Degree),
        Control::Toggle => Part {
            scope: Scope::control(id, label),
            act: Act::Set(c.uniform.clone(), vec![f64::from(first(&c.value) == 0.0)]),
            parts: Vec::new(),
        },
        Control::Choice(names) => group(sets(names, &|j| j as f64)),
        Control::Flags(names) => {
            let bits = first(&c.value) as u32;
            group(sets(names, &|j| f64::from(bits ^ (1 << j))))
        }
        Control::Colour => group(channels(id, &c.uniform, &c.value, c.range)),
        Control::Gradient(_) => {
            let stops = std::iter::once((&c.uniform, &c.value))
                .chain(c.stops.iter().map(|s| (&s.uniform, &s.value)));
            group(
                stops
                    .enumerate()
                    .map(|(j, (uniform, value))| Part {
                        scope: Scope::group(inner(id, j), intern(&stop_label(j))),
                        act: Act::Open,
                        parts: channels(inner(id, j), uniform, value, c.range),
                    })
                    .collect(),
            )
        }
        Control::Fields if c.value.len() == 1 => value(StepKind::Bounded, by(c)),
        Control::Fields => group(
            (0..c.value.len())
                .map(|j| Part {
                    scope: Scope::value(inner(id, j), COMPONENTS[j.min(3)], StepKind::Bounded),
                    act: step(j, by(c)),
                    parts: Vec::new(),
                })
                .collect(),
        ),
    })
}

/// The fields a field picker lists, in the order its value counts them: a field's value is its place here.
pub fn field_names() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| source_fields().unwrap_or_default())
}

/// A vector's components' names.
const COMPONENTS: [&str; 4] = ["x", "y", "z", "w"];
/// A colour's channels' names.
const CHANNELS: [&str; 3] = ["red", "green", "blue"];

fn stop_label(j: usize) -> String {
    format!("Stop {}", j + 1)
}

/// The three channels of the colour `value` of `uniform`, in scope `id`.
fn channels(id: ScopeId, uniform: &str, value: &[f64], range: Option<(f64, f64)>) -> Vec<Part> {
    (0..3)
        .map(|j| Part {
            scope: Scope::value(inner(id, j), CHANNELS[j], StepKind::Bounded),
            act: Act::Step(Step {
                uniform: uniform.to_owned(),
                value: value.to_vec(),
                at: j,
                ty: UniformType::Vec3,
                range,
                by: By::Code,
            }),
            parts: Vec::new(),
        })
        .collect()
}

fn first(v: &[f64]) -> f64 {
    v.first().copied().unwrap_or(0.0)
}

impl Step {
    /// The uniform's value after `adjust`, kept in its range and its type's.
    pub fn moved(&self, adjust: &Adjust) -> Vec<f64> {
        let v = self.value.get(self.at).copied().unwrap_or(0.0);
        let share = match adjust.delta() {
            Base::OfRange(share) => share,
            _ => 0.0,
        };
        let one = adjust.times.signum() * adjust.times.abs().round().max(1.0);
        let next = match (self.by, self.range) {
            (By::Share, Some((lo, hi))) if whole(self.ty) => {
                let step = share * (hi - lo);
                v + step.signum() * step.abs().round().max(1.0)
            }
            (By::Share, Some((lo, hi))) => v + share * (hi - lo),
            (By::Log, Some((lo, hi))) => v * engine::exact::pow(hi / lo, share),
            (By::Share | By::Log, None) => v + share,
            (By::One, _) => v + one,
            (By::Degree, _) => match adjust.delta() {
                Base::Degrees(d) => v + d.to_radians(),
                _ => v,
            },
            (By::Code, _) => linear(code(v) + one),
            (By::Of(size), _) => v + share * size,
        };
        let mut out = self.value.clone();
        if let Some(slot) = out.get_mut(self.at) {
            *slot = within(next, self.ty, self.range);
        }
        out
    }
}

/// `v` kept in `range` and in the values of `ty`.
fn within(v: f64, ty: UniformType, range: Option<(f64, f64)>) -> f64 {
    let (lo, hi) = range.unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
    let (least, most) = bounds(ty);
    v.clamp(lo.max(least), hi.min(most))
}

/// The least and the most value of a component of `ty`.
fn bounds(ty: UniformType) -> (f64, f64) {
    match ty {
        UniformType::I32 => (f64::from(i32::MIN), f64::from(i32::MAX)),
        UniformType::U32 => (0.0, f64::from(u32::MAX)),
        _ => (f64::from(f32::MIN), f64::from(f32::MAX)),
    }
}

/// The 8-bit sRGB code of the linear value `v`, clamped to the codes.
pub fn code(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * engine::exact::pow(v, 1.0 / 2.4) - 0.055
    };
    (255.0 * s).round()
}

/// The linear value of the 8-bit sRGB code `c`, clamped to the codes.
pub fn linear(c: f64) -> f64 {
    let s = c.clamp(0.0, 255.0) / 255.0;
    if s <= 0.040_45 {
        s / 12.92
    } else {
        engine::exact::pow((s + 0.055) / 1.055, 2.4)
    }
}

/// The linear colour `rgb` as drawn.
pub fn colour(rgb: [f64; 3]) -> Color32 {
    let [r, g, b] = rgb.map(|v| code(v) as u8);
    Color32::from_rgb(r, g, b)
}

fn rgb(v: &[f64]) -> [f64; 3] {
    [0, 1, 2].map(|i| v.get(i).copied().unwrap_or(0.0))
}

/// Draws a gradient of `stops`, evenly spaced and mixed in linear RGB, the library's ramps' rule, in `rect`.
pub fn gradient(painter: &egui::Painter, rect: Rect, stops: &[[f64; 3]]) {
    const STEPS: usize = 96;
    let Some(last) = stops.len().checked_sub(1) else {
        return;
    };
    let w = rect.width() / STEPS as f32;
    for i in 0..STEPS {
        let t = i as f64 / (STEPS - 1) as f64 * last as f64;
        let (k, f) = (t as usize, t.fract());
        let (a, b) = (stops[k], stops[(k + 1).min(last)]);
        let mixed = [0, 1, 2].map(|j| a[j] + f * (b[j] - a[j]));
        let chip = Rect::from_min_size(
            rect.min + vec2(i as f32 * w, 0.0),
            vec2(w + 0.5, rect.height()),
        )
        .intersect(rect);
        painter.rect_filled(chip, 0.0, colour(mixed));
    }
}

/// What the controls are drawn with.
pub struct Draw<'a, 'k> {
    /// The focus path: a colour it is in shows its channels.
    pub focus: &'a [String],
    /// The colours the mouse opened.
    pub open: &'a BTreeSet<ScopeId>,
    pub keys: &'a mut Keys<'k>,
    pub clicked: &'a mut Vec<ScopeId>,
    /// The uniforms edited, each with its value.
    pub edits: &'a mut Vec<(String, Vec<f64>)>,
    /// What the engine says of the node's uniforms.
    pub standing: &'a [Standing],
    /// Whether the section's heading says why none of its controls can matter, so they do not.
    pub shared: bool,
}

impl Draw<'_, '_> {
    /// Places `part`'s widget, and takes its click.
    fn put(&mut self, part: &Part, response: &egui::Response) {
        if response.clicked() {
            self.clicked.push(part.scope.id);
        }
        self.keys.place(part.scope.id, response);
    }

    /// Whether colour `id` shows its channels: opened by the mouse, or the focus in it.
    fn opened(&self, id: ScopeId) -> bool {
        let above = self.focus.len().saturating_sub(1);
        self.open.contains(id) || self.focus[..above].iter().any(|f| f == id)
    }
}

/// How far in the words under a dimmed control start, in points.
const WORDS_INDENT: i8 = 10;

/// Draws `control`, dimmed where there are words of `why`, over them unless they are `shared`.
fn dimmed(ui: &mut Ui, why: Option<String>, shared: bool, control: impl FnOnce(&mut Ui)) {
    let full = ui.opacity();
    if why.is_some() {
        ui.multiply_opacity(standing::DIMMED);
    }
    control(ui);
    ui.set_opacity(full);
    if let Some(why) = why.filter(|_| !shared) {
        // Under its control, nearer it than the next.
        ui.add_space(-0.5 * ui.spacing().item_spacing.y);
        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: WORDS_INDENT,
                bottom: 2,
                ..Default::default()
            })
            .show(ui, |ui| ui.label(RichText::new(why).small().weak()));
    }
}

/// Draws control `c` as `part`.
pub fn show(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    let why = standing::words(c, draw.standing);
    ui.push_id(part.scope.id, |ui| {
        dimmed(ui, why, draw.shared, |ui| control(ui, c, part, draw));
    });
}

fn control(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    match &c.control {
        Control::Hidden | Control::List | Control::Curve | Control::ImagePicker => {}
        Control::FieldPicker => named(ui, c, part, draw, |ui, j| {
            ui.add(Button::selectable(
                first(&c.value) == j as f64,
                field_names()[j].as_str(),
            ))
        }),
        Control::Gradient(stops) if stops.is_empty() => {}
        Control::Slider | Control::LogSlider => slider(ui, c, part, draw),
        Control::Stepper => stepper(ui, c, part, draw),
        Control::Toggle => {
            let mut on = first(&c.value) != 0.0;
            let toggle = ui.checkbox(&mut on, &c.label);
            draw.put(part, &toggle);
        }
        Control::Choice(names) => named(ui, c, part, draw, |ui, j| {
            ui.add(Button::selectable(
                first(&c.value) == j as f64,
                names[j].as_str(),
            ))
        }),
        Control::Flags(names) => named(ui, c, part, draw, |ui, j| {
            let mut on = first(&c.value) as u32 & (1 << j) != 0;
            ui.checkbox(&mut on, names[j].as_str())
        }),
        Control::Colour => swatch(ui, &c.label, &c.uniform, &c.value, c.range, part, draw),
        Control::Gradient(_) => stops(ui, c, part, draw),
        Control::Angle => angle(ui, c, part, draw),
        Control::Fields => fields(ui, c, part, draw),
    }
}

fn slider(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    let (lo, hi) = c.range.unwrap_or((0.0, 1.0));
    let mut v = first(&c.value);
    let slider = Slider::new(&mut v, lo..=hi)
        .logarithmic(c.control == Control::LogSlider)
        .text(c.label.as_str());
    let slider = ui.add(if whole(c.ty) {
        slider.integer()
    } else {
        slider
    });
    draw.keys.place(part.scope.id, &slider);
    // Live: each frame the value changes is an edit, the drag's frames one history entry.
    if slider.changed() {
        draw.edits.push((c.uniform.clone(), vec![v]));
    }
}

/// A number field of component `at` of `c`.
fn number(
    ui: &mut Ui,
    c: &ControlView,
    at: usize,
    edits: &mut Vec<(String, Vec<f64>)>,
) -> egui::Response {
    let mut v = c.value.get(at).copied().unwrap_or(0.0);
    let (lo, hi) = c.range.unwrap_or_else(|| bounds(c.ty));
    let field = DragValue::new(&mut v).range(lo..=hi);
    let field = match by(c) {
        By::One => field.speed(0.1).fixed_decimals(0),
        By::Of(size) => field.speed(0.01 * size).max_decimals(6),
        _ => field.speed(0.01 * (hi - lo)).max_decimals(6),
    };
    let field = ui.add(field);
    if field.changed() {
        let mut value = c.value.clone();
        value[at] = if whole(c.ty) { v.round() } else { v };
        edits.push((c.uniform.clone(), value));
    }
    field
}

fn stepper(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    ui.horizontal(|ui| {
        let less = ui.small_button("−");
        let field = number(ui, c, 0, draw.edits);
        let more = ui.small_button("+");
        let pressed = f64::from(more.clicked()) - f64::from(less.clicked());
        if pressed != 0.0 {
            let next = within(first(&c.value) + pressed, c.ty, c.range);
            draw.edits.push((c.uniform.clone(), vec![next]));
        }
        ui.label(&c.label);
        draw.keys.place(part.scope.id, &(field | less | more));
    });
}

/// The label of `c`, then a widget per name, each a part of `part`.
fn named(
    ui: &mut Ui,
    c: &ControlView,
    part: &Part,
    draw: &mut Draw<'_, '_>,
    mut widget: impl FnMut(&mut Ui, usize) -> egui::Response,
) {
    let shown = ui.vertical(|ui| {
        ui.label(&c.label);
        ui.horizontal_wrapped(|ui| {
            for (j, p) in part.parts.iter().enumerate() {
                let response = widget(ui, j);
                draw.put(p, &response);
            }
        });
    });
    draw.keys.place_rect(ui, part.scope.id, shown.response.rect);
}

/// A colour: its chip, its label and its code, and, open, its channels.
fn swatch(
    ui: &mut Ui,
    label: &str,
    uniform: &str,
    value: &[f64],
    range: Option<(f64, f64)>,
    part: &Part,
    draw: &mut Draw<'_, '_>,
) {
    let shade = colour(rgb(value));
    let row = ui.horizontal(|ui| {
        let (rect, chip) = ui.allocate_exact_size(vec2(18.0, 14.0), Sense::click());
        ui.painter().rect_filled(rect, 1.0, shade);
        let name = ui.add(egui::Label::new(label).sense(Sense::click()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let [r, g, b, _] = shade.to_array();
            let hex = format!("#{r:02x}{g:02x}{b:02x}");
            ui.label(RichText::new(hex).monospace().weak());
        });
        chip.widget_info(|| WidgetInfo::labeled(WidgetType::ColorButton, true, label));
        chip | name
    });
    draw.put(part, &row.inner);
    if !draw.opened(part.scope.id) {
        return;
    }
    ui.indent("channels", |ui| {
        for (j, p) in part.parts.iter().enumerate() {
            let mut at = code(rgb(value)[j]);
            let slider = ui.add(
                Slider::new(&mut at, 0.0..=255.0)
                    .integer()
                    .text(CHANNELS[j]),
            );
            draw.keys.place(p.scope.id, &slider);
            if slider.changed() {
                let mut next = rgb(value).to_vec();
                next[j] = within(linear(at), UniformType::Vec3, range);
                draw.edits.push((uniform.to_owned(), next));
            }
        }
    });
}

fn stops(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    let shown = ui.vertical(|ui| {
        ui.label(&c.label);
        let values = std::iter::once((&c.uniform, &c.value))
            .chain(c.stops.iter().map(|s| (&s.uniform, &s.value)))
            .collect::<Vec<_>>();
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 14.0), Sense::hover());
        let shades: Vec<[f64; 3]> = values.iter().map(|(_, v)| rgb(v)).collect();
        // Its chips overlap, so it is drawn whole and then covered: a see-through one would show its seams.
        let mut whole = ui.painter().clone();
        whole.set_opacity(1.0);
        gradient(&whole, bar, &shades);
        let cover = ui.visuals().panel_fill.gamma_multiply(1.0 - ui.opacity());
        whole.rect_filled(bar, 0.0, cover);
        for (j, ((uniform, value), p)) in values.into_iter().zip(&part.parts).enumerate() {
            ui.push_id(j, |ui| {
                let why = standing::stop_words(c, uniform, draw.standing);
                dimmed(ui, why, false, |ui| {
                    swatch(ui, &stop_label(j), uniform, value, c.range, p, draw);
                });
            });
        }
    });
    draw.keys.place_rect(ui, part.scope.id, shown.response.rect);
}

fn angle(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    ui.horizontal(|ui| {
        let v = first(&c.value);
        // A dial beside the degrees, from its right, counter-clockwise.
        let (dial, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        let stroke = ui.visuals().widgets.inactive.fg_stroke;
        ui.painter().circle_stroke(dial.center(), 8.0, stroke);
        let hand = vec2(v.cos() as f32, -v.sin() as f32) * 8.0;
        ui.painter().line_segment(
            [dial.center(), dial.center() + hand],
            Stroke::new(1.5, stroke.color),
        );
        let mut degrees = v.to_degrees();
        let mut field = DragValue::new(&mut degrees)
            .speed(1.0)
            .suffix("°")
            .max_decimals(2);
        if let Some((lo, hi)) = c.range {
            field = field.range(lo.to_degrees()..=hi.to_degrees());
        }
        let field = ui.add(field);
        ui.label(&c.label);
        draw.keys.place(part.scope.id, &field);
        if field.changed() {
            let next = within(degrees.to_radians(), c.ty, c.range);
            draw.edits.push((c.uniform.clone(), vec![next]));
        }
    });
}

fn fields(ui: &mut Ui, c: &ControlView, part: &Part, draw: &mut Draw<'_, '_>) {
    let shown = ui.horizontal(|ui| {
        for at in 0..c.value.len() {
            let field = number(ui, c, at, draw.edits);
            let scope = part.parts.get(at).map_or(part.scope.id, |p| p.scope.id);
            draw.keys.place(scope, &field);
        }
        ui.label(&c.label);
    });
    if !part.parts.is_empty() {
        draw.keys.place_rect(ui, part.scope.id, shown.response.rect);
    }
}
