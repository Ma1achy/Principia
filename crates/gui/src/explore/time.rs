//! Time (render_gui_spec §G2, §G3; GUI_DESIGN_NOTES § "01 Explore"; 01_main.png; R-101, R-404, R-409 F1): bottom
//! centre, play, step, a scrubber and speed. Transport, play / pause and speed, is `ViewUI`'s: not undoable, never
//! read by the engine (R-96). The GUI's clock advances the playhead through `SetField`s marked "no history" while
//! playing (`crate::clock`); a step is one `SetField`, and a scrub is live, the playhead edit sent every frame the
//! dragged time changes, so the display time follows the drag, and the whole drag is one coalesced entry, the app
//! sending each frame's edit under the pointer's hold's key (R-96, R-101, R-458 F2). Scrubbing back re-integrates on the real engine
//! (TASK-M8-07); the mock's figure does not refine, so no progress is shown (applied per R-369).
//!
//! Its keys (§G3, R-409 F1): Time is a scope with children, step back, play, step, speed and the scrubber, in the
//! order drawn. In navigation mode the arrows move between them and step no time; Enter on a button acts as its click;
//! Enter on the scrubber or on speed starts interaction mode, in which ← → step the time by `TimeStep`, 1/100 of
//! `[0, T]`, or the speed by `Bounded`, 1/100 of its range (R-404). Space plays with the focus on Time or any element in
//! it, in either mode.

use eframe::egui::{
    self, Align, Button, Layout as EguiLayout, Rect, RichText, Slider, Ui, UiBuilder,
};
use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::Transport;

use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::keyboard::Keyboard;

/// Time's big scope.
pub const TIME: ScopeId = "time";
/// Step back one `TimeStep`.
pub const BACK: ScopeId = "time_back";
/// Play / pause.
pub const PLAY: ScopeId = "time_play";
/// Step on one `TimeStep`.
pub const STEP: ScopeId = "time_step";
/// The playback speed.
pub const SPEED: ScopeId = "time_speed";
/// The scrubber.
pub const SCRUBBER: ScopeId = "time_scrubber";

/// The title, as 01_main.png's.
pub const TITLE: &str = "Time";
/// The note under the scrubber, as 01_main.png's.
pub const NOTE: &str =
    "scrubbing back re-integrates to that time — the figure refines progressively";
/// The step-back button's label (a look choice, R-390: 01_main.png's ⏮).
pub const BACK_LABEL: &str = "⏮";
/// The play button's label while paused.
pub const PLAY_LABEL: &str = "▶";
/// The play button's label while playing.
pub const PAUSE_LABEL: &str = "⏸";
/// The step button's label (01_main.png's ⏭).
pub const STEP_LABEL: &str = "⏭";

/// The playback speed's range, `×` the clock's own rate (a look choice, R-390: 01_main.png shows "× 1"; four times
/// slower to four times faster). Its keyboard step is R-404's `Bounded`, 1/100 of it.
pub const SPEED_RANGE: (f64, f64) = (0.25, 4.0);
/// The speed the transport starts at.
pub const SPEED_DEFAULT: f64 = 1.0;

/// The panel's margin, in points (a look choice, R-390).
pub const MARGIN: f32 = 12.0;

/// Joins Time's elements to `tree` under its big scope, in the order drawn, with its rows of the `?` overlay.
pub fn register(tree: &mut ScopeTree) {
    tree.register(None, Scope::group(TIME, TITLE));
    tree.register(Some(TIME), Scope::control(BACK, "step back"));
    tree.register(Some(TIME), Scope::control(PLAY, "play"));
    tree.register(Some(TIME), Scope::control(STEP, "step"));
    tree.register(Some(TIME), Scope::value(SPEED, "speed", StepKind::Bounded));
    tree.register(
        Some(TIME),
        Scope::value(SCRUBBER, "scrubber", StepKind::TimeStep),
    );
    for (key, action) in SHORTCUTS {
        tree.register_shortcut(key, action);
    }
}

/// The rows Time adds to the `?` overlay (§G3, R-409 F1).
pub const SHORTCUTS: [(&str, &str); 2] = [
    ("Time: Space", "play / pause, in either mode"),
    ("Time: ← →", "step the time, after Enter on the scrubber"),
];

/// The playhead edit to `t`: a manual move, so it enters the history, one entry (R-96).
pub fn playhead_edit(t: f64) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history: false,
    }
}

/// One `TimeStep`'s length on the scrubber `[0, t_max]`: R-404's 1/100 of the range.
pub fn time_step(t_max: f64) -> f64 {
    let Base::OfRange(fraction) = StepKind::TimeStep.base() else {
        unreachable!("the time step is a fraction of the scrubber's range")
    };
    fraction * t_max
}

/// `t` moved by `delta`, kept on the scrubber `[0, t_max]`.
pub fn moved(t: f64, delta: f64, t_max: f64) -> f64 {
    (t + delta).clamp(0.0, t_max)
}

/// The speed after an adjustment of `fraction` of [`SPEED_RANGE`], kept in it.
pub fn speed_after(speed: f64, fraction: f64) -> f64 {
    let (lo, hi) = SPEED_RANGE;
    (speed + fraction * (hi - lo)).clamp(lo, hi)
}

/// Play or pause: from the horizon, play starts again from 0, one entry in the history.
pub fn toggle_play(snapshot: &Snapshot, transport: &mut Transport, edits: &mut Vec<SetField>) {
    if !transport.playing && snapshot.render.playhead.t >= snapshot.sim.horizon.t {
        edits.push(playhead_edit(0.0));
    }
    transport.playing = !transport.playing;
}

/// Enter on Time's control `id`, as its click: play toggles, the steps step.
pub fn activate(
    id: &str,
    snapshot: &Snapshot,
    transport: &mut Transport,
    edits: &mut Vec<SetField>,
) {
    let (t, t_max) = (snapshot.render.playhead.t, snapshot.sim.horizon.t);
    match id {
        PLAY => toggle_play(snapshot, transport, edits),
        BACK => edits.push(playhead_edit(moved(t, -time_step(t_max), t_max))),
        STEP => edits.push(playhead_edit(moved(t, time_step(t_max), t_max))),
        _ => {}
    }
}

/// The arrows on the scrubber or on speed, in interaction mode: the time by `TimeStep`, the speed by `Bounded`, each
/// times Shift's ×10 or Alt's ×0.1.
pub fn adjust(
    adjust: &Adjust,
    snapshot: &Snapshot,
    transport: &mut Transport,
    edits: &mut Vec<SetField>,
) {
    let Base::OfRange(fraction) = adjust.delta() else {
        return;
    };
    let t_max = snapshot.sim.horizon.t;
    match adjust.scope {
        SCRUBBER => edits.push(playhead_edit(moved(
            snapshot.render.playhead.t,
            fraction * t_max,
            t_max,
        ))),
        SPEED => transport.speed = speed_after(transport.speed, fraction),
        _ => {}
    }
}

/// The time readout: "t 12.40".
pub fn readout(t: f64) -> String {
    format!("t {t:.2}")
}

/// The speed readout: "× 1.00".
pub fn speed_readout(speed: f64) -> String {
    format!("× {speed:.2}")
}

/// The scrubber's tick labels: 0 to `t_max` in five equal steps, each as a whole number when it is one.
pub fn ticks(t_max: f64) -> Vec<(f64, String)> {
    (0..=5)
        .map(|i| {
            let t = t_max * f64::from(i) / 5.0;
            (t, number(t))
        })
        .collect()
}

/// `x` as a whole number when it is one, else with one decimal.
pub fn number(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{x:.0}")
    } else {
        format!("{x:.1}")
    }
}

/// What Time reads and records into.
pub struct Panel<'a> {
    /// The latest snapshot.
    pub snapshot: &'a Snapshot,
    /// The transport, `ViewUI`'s.
    pub transport: &'a mut Transport,
    /// The keyboard layer, for the places of the controls.
    pub keyboard: &'a mut Keyboard,
    /// The edits its controls make, in order.
    pub edits: &'a mut Vec<SetField>,
}

/// Draws Time in `rect`.
pub fn show(ui: &mut Ui, rect: Rect, panel: &mut Panel<'_>) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    ui.painter().rect_stroke(
        rect,
        0.0,
        visuals.widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    panel.keyboard.place(TIME, rect);
    let snapshot = panel.snapshot;
    let (t, t_max) = (snapshot.render.playhead.t, snapshot.sim.horizon.t);
    let body = rect.shrink(MARGIN);
    let mut column = ui.new_child(UiBuilder::new().max_rect(body));
    column.spacing_mut().item_spacing.y = 8.0;
    column.label(RichText::new(TITLE).heading());
    column.horizontal(|row| {
        let back = row.add(Button::new(BACK_LABEL).min_size(egui::vec2(32.0, 0.0)));
        panel.keyboard.place_widget(BACK, &back);
        if back.clicked() {
            activate(BACK, snapshot, panel.transport, panel.edits);
        }
        let label = if panel.transport.playing {
            PAUSE_LABEL
        } else {
            PLAY_LABEL
        };
        let play = row.add(
            Button::selectable(panel.transport.playing, label).min_size(egui::vec2(32.0, 0.0)),
        );
        panel.keyboard.place_widget(PLAY, &play);
        if play.clicked() {
            activate(PLAY, snapshot, panel.transport, panel.edits);
        }
        let step = row.add(Button::new(STEP_LABEL).min_size(egui::vec2(32.0, 0.0)));
        panel.keyboard.place_widget(STEP, &step);
        if step.clicked() {
            activate(STEP, snapshot, panel.transport, panel.edits);
        }
        row.add_space(8.0);
        row.label(RichText::new(readout(t)).monospace());
        row.add_space(8.0);
        let mut speed = panel.transport.speed;
        row.spacing_mut().slider_width = 90.0;
        let slider = row.add(
            Slider::new(&mut speed, SPEED_RANGE.0..=SPEED_RANGE.1)
                .show_value(false)
                .text(RichText::new(speed_readout(panel.transport.speed)).monospace()),
        );
        panel.keyboard.place_widget(SPEED, &slider);
        if slider.changed() {
            panel.transport.speed = speed.clamp(SPEED_RANGE.0, SPEED_RANGE.1);
        }
    });
    let mut scrub = t;
    let width = column.available_width();
    column.spacing_mut().slider_width = (width - 70.0).max(40.0);
    let scrubber = column
        .horizontal(|row| {
            let s = row
                .add(Slider::new(&mut scrub, 0.0..=t_max.max(f64::MIN_POSITIVE)).show_value(false));
            row.label(RichText::new(format!("{t:.1}")).monospace());
            s
        })
        .inner;
    panel.keyboard.place_widget(SCRUBBER, &scrubber);
    let rail = scrubber.rect;
    let ticks_rect = Rect::from_min_size(rail.left_bottom(), egui::vec2(rail.width(), 14.0));
    for (tick, text) in ticks(t_max) {
        let x = rail.left() + rail.width() * (tick / t_max.max(f64::MIN_POSITIVE)) as f32;
        column.painter().text(
            egui::pos2(x, ticks_rect.center().y),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(10.0),
            visuals.weak_text_color(),
        );
    }
    column.add_space(14.0);
    column.with_layout(EguiLayout::left_to_right(Align::Min), |row| {
        row.label(RichText::new(NOTE).weak().small());
    });
    if let Some(edit) = scrub_edit(&scrubber, scrub) {
        panel.edits.push(edit);
    }
}

/// A scrub: each frame the scrubber's value changes, dragged or clicked, one `SetField` to it, the drag's frames one
/// coalesced entry under the pointer's hold's key (R-96, R-458 F2).
pub fn scrub_edit(response: &egui::Response, value: f64) -> Option<SetField> {
    response.changed().then(|| playhead_edit(value))
}
