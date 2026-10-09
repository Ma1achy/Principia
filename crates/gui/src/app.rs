//! The app shell (render_gui_spec §G1, §G2; R-390): generic over the engine side, it reads snapshots and sends
//! `SetField`s and undo / redo requests only (gui_state_contract §1). egui is a toggleable debug layer (F3) over the
//! figure, which keeps 01_main.png's central rect in both states, the rest of the window the clear colour while the
//! layer is hidden (RQ-248). The GUI's clock reads `ViewUI`'s transport and advances the playhead (R-101; RQ-246).
//! The keyboard layer (render_gui_spec §G3) takes its keys before egui's pass and moves `ViewUI`'s focus.

use std::sync::Arc;

use eframe::egui::{self, Color32, Id, Key, LayerId, Order, Rect};
use eframe::egui_wgpu;
use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{
    Backdrop, DebugCategories, Focus, Inspector, KeptOrbits, LinkedViews, Mode, Selection,
    Transport, ViewUI, Windows,
};

use crate::clock::Clock;
use crate::explore;
use crate::keyboard::Keyboard;
use crate::layout::Layout;
use crate::side::EngineSide;

/// How often the app reads a snapshot: the engine posts it throttled to ~10 Hz, never per frame (caching contract
/// Part 6a; gui_state_contract §1).
pub const SNAPSHOT_INTERVAL_S: f64 = 0.1;

/// The most rows the console keeps; the oldest go first. The footer's counts are kept apart and lose nothing.
pub const CONSOLE_ROWS: usize = 10_000;

/// The window's title: "principia · dev", and "— mock engine" on the mock, so the tag is never out of sight while
/// the egui layer is hidden (RQ-248).
pub fn window_title(is_mock: bool) -> &'static str {
    if is_mock {
        "principia · dev — mock engine"
    } else {
        "principia · dev"
    }
}

/// The colour the window is cleared to: what shows wherever nothing is drawn, all but the figure while the egui layer
/// is hidden. egui's dark theme's darkest background.
pub fn clear_colour(visuals: &egui::Visuals) -> Color32 {
    visuals.extreme_bg_color
}

/// The app's initial `ViewUI`: the Explore mode, the transport paused, no scope focused, and every other group
/// empty.
pub fn initial_view() -> ViewUI {
    ViewUI {
        backdrop: Backdrop {},
        debug_categories: DebugCategories {},
        focus: Focus { path: Vec::new() },
        selection: Selection {},
        kept_orbits: KeptOrbits {},
        inspector: Inspector {},
        windows: Windows {},
        linked_views: LinkedViews {},
        transport: Transport { playing: false },
        mode: Mode::Explore,
    }
}

/// The warnings and errors logged since the session began (gui_state_contract §2), until TASK-M6-28's "clear".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// `warn` entries.
    pub warnings: u32,
    /// `error` entries.
    pub errors: u32,
}

/// The app.
pub struct App<S: EngineSide> {
    side: S,
    /// The GUI's own state, which the engine never reads.
    pub view: ViewUI,
    clock: Clock,
    /// Whether the egui layer is shown (F3).
    pub shown: bool,
    /// Whether the console is open.
    pub console_open: bool,
    /// The keyboard layer.
    pub keyboard: Keyboard,
    snapshot: Snapshot,
    console: Vec<LogEntry>,
    counts: Counts,
    last_read: Option<f64>,
    reread: bool,
    format: wgpu::TextureFormat,
}

/// The app's actions from one frame's controls.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Actions {
    /// Toggle the egui layer, as F3 does.
    pub toggle_layer: bool,
    /// Toggle the console.
    pub toggle_console: bool,
    /// Switch to this mode.
    pub mode: Option<Mode>,
    /// Open the `?` shortcuts.
    pub shortcuts: bool,
    /// Close the window.
    pub quit: bool,
}

impl<S: EngineSide> App<S> {
    /// The app on `side`, drawing the figure on targets of `format`; it reads its first snapshot at once.
    pub fn new(mut side: S, format: wgpu::TextureFormat) -> Self {
        let mut console = Vec::new();
        let mut counts = Counts::default();
        let snapshot = side.engine().snapshot();
        absorb(&snapshot, &mut console, &mut counts);
        Self {
            side,
            view: initial_view(),
            clock: Clock::new(),
            shown: true,
            console_open: false,
            keyboard: Keyboard::new(),
            snapshot,
            console,
            counts,
            last_read: None,
            reread: false,
            format,
        }
    }

    /// The engine side.
    pub fn side(&mut self) -> &mut S {
        &mut self.side
    }

    /// The latest snapshot the app read.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// The console's rows, oldest first.
    pub fn console(&self) -> &[LogEntry] {
        &self.console
    }

    /// The footer's counts.
    pub fn counts(&self) -> Counts {
        self.counts
    }

    /// Sends a `SetField`; an undoable one rereads the snapshot next frame and resyncs the clock.
    pub fn set_field(&mut self, edit: engine::contract::set_field::SetField) {
        if !edit.no_history {
            self.clock.resync();
            self.reread = true;
        }
        self.side.engine().set_field(edit);
    }

    /// Requests undo, rereading the snapshot next frame.
    pub fn undo(&mut self) {
        self.side.engine().undo();
        self.clock.resync();
        self.reread = true;
    }

    /// Requests redo, rereading the snapshot next frame.
    pub fn redo(&mut self) {
        self.side.engine().redo();
        self.clock.resync();
        self.reread = true;
    }

    /// Reads a snapshot: its entries join the console and the counts.
    fn read(&mut self) {
        self.snapshot = self.side.engine().snapshot();
        absorb(&self.snapshot, &mut self.console, &mut self.counts);
    }

    /// Before each frame's egui pass: the keyboard layer takes its keys out of `raw` while the layer is shown.
    pub fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if self.shown {
            self.keyboard.take_keys(ctx, raw);
        }
    }

    /// One frame on the root `ui`.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.key_pressed(Key::F3)) {
            self.shown = !self.shown;
        }
        // A request of the GUI's own moved the state: read it before the clock advances from it.
        if std::mem::take(&mut self.reread) {
            self.read();
        }
        let playing = self.view.transport.playing;
        let t = self.snapshot.render.playhead.t;
        let ticked = self
            .side
            .time_source()
            .map(|source| (self.clock.frame(source, playing, t), source.rate_hz()));
        if let Some((edit, rate_hz)) = ticked {
            if playing {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(1.0 / rate_hz));
            }
            if let Some(edit) = edit {
                self.side.engine().set_field(edit);
            }
        }
        let now = ctx.input(|i| i.time);
        let due = self
            .last_read
            .is_none_or(|last| now - last >= SNAPSHOT_INTERVAL_S);
        if due {
            self.read();
            self.last_read = Some(now);
        }
        if !due {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(SNAPSHOT_INTERVAL_S));
        }
        let layout = Layout::new(ui.max_rect(), ctx.pixels_per_point());
        if self.view.mode == Mode::Explore {
            self.paint_figure(&ctx, layout.figure);
        }
        if !self.shown {
            // Hidden, the layer takes no key: it forgets the ones it held, whose releases now go to egui.
            self.keyboard.stand_down();
            return;
        }
        let mut actions = Actions::default();
        let activated = self.keyboard.run(self.view.mode, &mut self.view.focus, now);
        for id in activated {
            let place = self.keyboard.place_of(id);
            explore::top_bar::activate(&ctx, id, place, &mut actions);
        }
        if let Some(due) = self.keyboard.next_repeat_s() {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64((due - now).max(0.0)));
        }
        self.keyboard.clear_places();
        let tree = self.keyboard.tree(self.view.mode);
        let breadcrumb = explore::breadcrumb::text(tree, &self.view.focus);
        explore::top_bar::show(
            ui,
            layout.top_bar,
            &self.snapshot,
            &explore::top_bar::TopBar {
                mode: self.view.mode,
                breadcrumb: &breadcrumb,
            },
            &mut self.keyboard,
            &mut actions,
        );
        match self.view.mode {
            Mode::Explore => {
                explore::regions(ui, &layout);
                explore::place(&mut self.keyboard, &layout);
            }
            Mode::Stain => explore::stain_page(ui, layout.page),
        }
        explore::footer::show(
            ui,
            layout.footer,
            &explore::footer::Footer {
                counts: self.counts,
                latest: self.console.last(),
                memory: self.snapshot.frame.live_memory,
                is_mock: self.side.is_mock(),
                console_open: self.console_open,
            },
            &mut actions,
        );
        if self.console_open {
            let mut open = true;
            crate::console::show(&ctx, layout.bottom_row, &self.console, &mut open);
            actions.toggle_console |= !open;
        }
        self.draw_ring(&ctx);
        self.apply(&ctx, actions);
        if self.keyboard.shortcuts_open {
            let rows = self.keyboard.tree(self.view.mode).shortcuts();
            let rect = crate::keyboard::overlay::show(&ctx, rows);
            self.keyboard.overlay_drawn(rect);
        }
    }

    /// Draws the focus ring on the focused scope, where it was drawn this frame.
    fn draw_ring(&self, ctx: &egui::Context) {
        let tree = self.keyboard.tree(self.view.mode);
        let path = &self.view.focus.path;
        let Some(id) = path[..tree.valid_prefix(path)].last() else {
            return;
        };
        let (Some(scope), Some(place)) = (tree.get(id), self.keyboard.place_of(id)) else {
            return;
        };
        explore::breadcrumb::ring(ctx, place, scope.ring);
    }

    fn apply(&mut self, ctx: &egui::Context, actions: Actions) {
        if actions.toggle_layer {
            self.shown = !self.shown;
        }
        if actions.toggle_console {
            self.console_open = !self.console_open;
        }
        if let Some(mode) = actions.mode {
            self.view.mode = mode;
        }
        if actions.shortcuts {
            self.keyboard.shortcuts_open = true;
        }
        if actions.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Draws the figure through the engine side's canvas into `rect`, above the regions' frames, so nothing of
    /// theirs reaches it.
    fn paint_figure(&self, ctx: &egui::Context, rect: Rect) {
        let Some(canvas) = self.side.canvas() else {
            return;
        };
        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            FigureCallback {
                canvas,
                format: self.format,
            },
        );
        ctx.layer_painter(LayerId::new(Order::Middle, Id::new("figure")))
            .add(callback);
    }
}

/// Adds `snapshot`'s log entries to the console and the counts.
fn absorb(snapshot: &Snapshot, console: &mut Vec<LogEntry>, counts: &mut Counts) {
    for entry in &snapshot.log {
        match entry.severity {
            Severity::Warn => counts.warnings = counts.warnings.saturating_add(1),
            Severity::Error => counts.errors = counts.errors.saturating_add(1),
            Severity::Info => {}
        }
    }
    console.extend(snapshot.log.iter().cloned());
    let excess = console.len().saturating_sub(CONSOLE_ROWS);
    console.drain(..excess);
}

/// The figure, drawn by the canvas into egui-wgpu's pass with its viewport set to the figure's rect.
struct FigureCallback {
    canvas: Arc<dyn Canvas>,
    format: wgpu::TextureFormat,
}

impl egui_wgpu::CallbackTrait for FigureCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        self.canvas.draw_figure(render_pass, self.format);
    }
}

impl<S: EngineSide + 'static> eframe::App for App<S> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        App::ui(self, ui);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        App::raw_input_hook(self, ctx, raw_input);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        clear_colour(visuals).to_normalized_gamma_f32()
    }
}
