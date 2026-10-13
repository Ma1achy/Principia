//! The app shell (render_gui_spec §G1, §G2; R-390): generic over the engine side, it reads snapshots and sends
//! `SetField`s and undo / redo requests only (gui_state_contract §1). egui is a toggleable debug layer (F3) over the
//! figure, which keeps 01_main.png's central rect in both states, the rest of the window the clear colour while the
//! layer is hidden (RQ-248). The GUI's clock reads `ViewUI`'s transport and advances the playhead (R-101; RQ-246).
//! The keyboard layer (render_gui_spec §G3) takes its keys before egui's pass and moves `ViewUI`'s focus. The
//! Manifold view, the compass and the figure's gestures send their edits through [`App::set_field`], as any control.

use std::sync::Arc;

use eframe::egui::{self, Color32, Id, Key, LayerId, Order, Rect};
use eframe::egui_wgpu;
use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity};
use engine::contract::set_field::{Edit, RenderField};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{
    Backdrop, DebugCategories, Focus, Inspector, KeptOrbits, KeyMode, LinkedViews, Mode, Selection,
    Transport, ViewUI, Windows,
};

use crate::clock::{Clock, Playback};
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

/// The app's initial `ViewUI`: the Explore mode, the transport paused, no scope focused (the keyboard in navigation
/// mode), and every other group empty.
pub fn initial_view() -> ViewUI {
    ViewUI {
        backdrop: Backdrop {},
        debug_categories: DebugCategories {},
        focus: Focus {
            path: Vec::new(),
            mode: KeyMode::Navigation,
        },
        selection: Selection {},
        kept_orbits: KeptOrbits::default(),
        inspector: Inspector {},
        windows: Windows { open: Vec::new() },
        linked_views: LinkedViews {},
        transport: Transport {
            playing: false,
            speed: explore::time::SPEED_DEFAULT,
        },
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
    /// The Manifold view's and the compass's own scratch state.
    pub manifold: explore::manifold_view::Scratch,
    /// Time's own scratch state: the scrub in progress.
    pub time: explore::time::Scratch,
    /// The Trajectory panel's own state.
    pub trajectory: explore::trajectory::Scratch,
    /// The overlay set Overlays ▾'s save as default kept, for the session (applied per R-369: the mock has no settings
    /// store; every launch starts with every overlay off).
    pub overlay_default: engine::contract::render_state::Overlays,
    /// The hover last sent: the cursor's figure point, or none.
    hover_sent: Option<[f64; 2]>,
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
    /// Set the overlay set to this: a toggle, or all off, one `SetField` (Overlays ▾).
    pub overlays: Option<engine::contract::render_state::Overlays>,
    /// Keep the overlay set as the default (Overlays ▾'s save as default).
    pub save_overlays: bool,
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
            manifold: Default::default(),
            time: Default::default(),
            trajectory: Default::default(),
            overlay_default: Default::default(),
            hover_sent: None,
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
        let playback = Playback {
            playing: self.view.transport.playing,
            speed: self.view.transport.speed,
            t: self.snapshot.render.playhead.t,
            t_max: self.snapshot.sim.horizon.t,
        };
        let orbit = explore::trajectory::shown(
            self.trajectory.tab,
            self.snapshot.sample.as_ref(),
            &self.view.kept_orbits,
        );
        let span = orbit.map_or(0.0, |o| o.path_t);
        let moving = self.trajectory.moving(orbit.is_some());
        let ticked = self.side.time_source().map(|source| {
            (
                self.clock.frame(source, playback),
                source.rate_hz(),
                source.held_seconds(),
            )
        });
        let now = ctx.input(|i| i.time);
        // What turns reads the wall clock, unless the source holds it (capture mode's frozen clock).
        let mut seconds = now;
        if let Some((edit, rate_hz, held)) = ticked {
            seconds = held.unwrap_or(now);
            if playback.playing || moving {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(1.0 / rate_hz));
            }
            if let Some(edit) = edit {
                if let Edit::Render(RenderField::Playhead(p)) = &edit.edit {
                    // At the horizon the clock stops, and so does the transport.
                    if p.t >= playback.t_max {
                        self.view.transport.playing = false;
                    }
                }
                self.side.engine().set_field(edit);
            }
        } else if moving {
            ctx.request_repaint();
        }
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
        self.trajectory.advance(seconds, span);
        let layout = Layout::new(ui.max_rect(), ctx.pixels_per_point());
        if self.view.mode == Mode::Explore {
            self.paint_figure(&ctx, layout.figure);
        }
        if !self.shown {
            // Hidden, the layer takes no key: it forgets the ones it held, whose releases now go to egui. The hover,
            // the layer's, goes with it.
            self.keyboard.stand_down();
            self.send_hover(None);
            return;
        }
        let mut actions = Actions::default();
        let mut out = explore::manifold_view::Out::default();
        let mut edits = Vec::new();
        explore::manifold_view::join(self.keyboard.tree_mut(Mode::Explore));
        explore::trajectory::join_kept(
            self.keyboard.tree_mut(Mode::Explore),
            self.view.kept_orbits.orbits.len(),
        );
        let activated = self
            .keyboard
            .run(&ctx, self.view.mode, &mut self.view.focus, now);
        for id in activated {
            explore::top_bar::activate(id, &mut actions);
            explore::overlays_menu::activate(id, &self.snapshot.render.overlays, &mut actions);
            explore::manifold_view::activate(id, &self.snapshot, &mut self.manifold, &mut out);
            explore::time::activate(id, &self.snapshot, &mut self.view.transport, &mut edits);
            explore::trajectory::activate(
                id,
                self.snapshot.sample.as_ref(),
                &mut self.view.kept_orbits,
                &mut self.trajectory,
            );
        }
        for adjust in self.keyboard.adjusted().to_vec() {
            explore::manifold_view::adjust(&adjust, &self.snapshot, &mut self.manifold, &mut out);
            explore::time::adjust(
                &adjust,
                &self.snapshot,
                &mut self.view.transport,
                &mut edits,
            );
            explore::trajectory::adjust(&adjust, span, &mut self.trajectory);
        }
        self.shortcuts(&ctx, &mut actions, &mut edits);
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
                keys: self.view.focus.mode,
            },
            &mut self.keyboard,
            &mut actions,
        );
        match self.view.mode {
            Mode::Explore => self.explore(ui, &layout, &mut out, &mut edits),
            Mode::Stain => {
                explore::stain_page(ui, layout.page);
                self.send_hover(None);
            }
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
        if let Some(overlays) = actions.overlays.take() {
            edits.push(engine::contract::set_field::SetField {
                edit: Edit::Render(RenderField::Overlays(overlays)),
                no_history: false,
            });
        }
        if std::mem::take(&mut actions.save_overlays) {
            self.overlay_default = self.snapshot.render.overlays.clone();
        }
        self.apply(&ctx, actions);
        for edit in edits {
            self.set_field(edit);
        }
        // Locked, every basis edit turns about the pin (render_gui_spec §G4).
        for edit in explore::manifold_view::about_the_pin(out.edits, &self.snapshot.sim) {
            self.set_field(edit);
        }
        for window in out.windows {
            if !self.view.windows.open.contains(&window) {
                self.view.windows.open.push(window);
            }
        }
        if self.keyboard.shortcuts_open {
            let rows = self.keyboard.tree(self.view.mode).shortcuts();
            let rect = crate::keyboard::overlay::show(&ctx, rows);
            self.keyboard.overlay_drawn(rect);
        }
    }

    /// The Explore page below the top bar: the Manifold view, the figure's gestures, its axis labels and the lock's
    /// reticle, the compass, and the regions later tasks fill.
    fn explore(
        &mut self,
        ui: &mut egui::Ui,
        layout: &Layout,
        out: &mut explore::manifold_view::Out,
        edits: &mut Vec<engine::contract::set_field::SetField>,
    ) {
        explore::place(&mut self.keyboard, layout);
        // Each panel draws in a child of its own, so how many widgets it draws (the trajectory panel's change with the
        // orbit shown) never moves the ids of the widgets drawn after it, and a click that lands as the panel changes
        // still reaches its widget.
        let mut own =
            |salt: &str| ui.new_child(egui::UiBuilder::new().id_salt(salt).max_rect(ui.max_rect()));
        explore::trajectory::show(
            &mut own("trajectory"),
            layout.trajectory,
            &mut explore::trajectory::Panel {
                sample: self.snapshot.sample.as_ref(),
                kept: &mut self.view.kept_orbits,
                scratch: &mut self.trajectory,
                keyboard: &mut self.keyboard,
            },
        );
        explore::time::show(
            &mut own("time"),
            layout.time,
            &mut explore::time::Panel {
                snapshot: &self.snapshot,
                transport: &mut self.view.transport,
                scratch: &mut self.time,
                keyboard: &mut self.keyboard,
                edits,
            },
        );
        explore::legend::show(
            &mut own("legend"),
            layout.legend,
            &self.snapshot.legend,
            &mut self.keyboard,
        );
        let focused_figure = self
            .view
            .focus
            .path
            .last()
            .is_some_and(|f| f == explore::FIGURE);
        explore::manifold_view::navigate::figure(
            ui,
            layout.figure,
            focused_figure,
            &self.snapshot,
            &mut self.manifold,
            out,
        );
        let mut panel = explore::manifold_view::Panel {
            snapshot: &self.snapshot,
            scratch: &mut self.manifold,
            keyboard: &mut self.keyboard,
            focus: &self.view.focus.path,
            out,
        };
        explore::manifold_view::show(ui, layout.manifold_view, &mut panel);
        let sim = &self.snapshot.sim;
        explore::compass::show(
            ui,
            layout.compass,
            sim,
            &mut self.keyboard,
            &mut self.manifold,
            out,
        );
        explore::axis_labels::show(ui, layout.axis_x, layout.axis_y, layout.figure, &sim.plane);
        if let Some(at) = explore::lock::reticle_at(&sim.lock, &sim.plane, layout.figure) {
            explore::lock::reticle(ui.ctx(), layout.figure, at);
        }
        let ctx = ui.ctx().clone();
        let pointer = ctx.pointer_hover_pos();
        let over_figure = pointer
            .and_then(|p| ctx.layer_id_at(p))
            .is_none_or(|layer| layer == ui.layer_id());
        self.send_hover(explore::hover_label::figure_point(
            layout.figure,
            pointer,
            over_figure,
        ));
        if let (Some(sample), Some(cursor), Some(_)) =
            (&self.snapshot.sample, pointer, self.hover_sent)
        {
            let text = explore::hover_label::text(
                sample,
                self.snapshot.render.playhead.t,
                self.snapshot.sim.horizon.t,
            );
            explore::hover_label::show(&ctx, layout.figure, cursor, text);
        }
    }

    /// Sends the hover `at` when it differs from the one last sent: one no-history `SetField` (R-101).
    fn send_hover(&mut self, at: Option<[f64; 2]>) {
        if at != self.hover_sent {
            self.hover_sent = at;
            self.side
                .engine()
                .set_field(explore::hover_label::hover_edit(at));
        }
    }

    /// The letter, Space and Alt+digit shortcuts (render_gui_spec §G2, §G3; R-409 F1): with the focus in the Figure,
    /// Space keeps the orbit under the cursor and L toggles listen; in Time, Space plays; anywhere, Alt+digit toggles
    /// an overlay. Each acts in either mode, but not while a text field holds the keyboard or the `?` overlay is open.
    fn shortcuts(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Actions,
        edits: &mut Vec<engine::contract::set_field::SetField>,
    ) {
        if ctx.text_edit_focused()
            || self.keyboard.shortcuts_open
            || self.view.mode != Mode::Explore
        {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut overlays = self.snapshot.render.overlays.clone();
        let presses = explore::overlays_menu::shortcut_presses(&events);
        for i in &presses {
            overlays = explore::overlays_menu::toggled(&overlays, *i);
        }
        if !presses.is_empty() {
            actions.overlays = Some(overlays);
        }
        let pressed = |key: Key| {
            events.iter().any(|e| {
                matches!(e, egui::Event::Key { key: k, pressed: true, repeat: false, modifiers, .. }
                    if *k == key && modifiers.is_none())
            })
        };
        let scope = self.view.focus.path.first().map(String::as_str);
        if pressed(Key::Space) {
            match scope {
                Some(explore::FIGURE) => {
                    if let Some(sample) = &self.snapshot.sample {
                        explore::trajectory::keep(
                            sample,
                            &mut self.view.kept_orbits,
                            &mut self.trajectory,
                        );
                    }
                }
                Some(explore::time::TIME) => {
                    explore::time::toggle_play(&self.snapshot, &mut self.view.transport, edits)
                }
                _ => {}
            }
        }
        if pressed(Key::L) && scope == Some(explore::FIGURE) {
            self.trajectory.listen = !self.trajectory.listen;
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
        explore::breadcrumb::ring(ctx, place, scope.ring, self.view.focus.mode);
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
