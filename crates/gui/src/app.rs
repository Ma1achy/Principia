//! The app shell (render_gui_spec §G1, §G2; R-390): generic over the engine side, it reads snapshots and sends
//! `SetField`s and undo / redo requests only (gui_state_contract §1). egui is a toggleable debug layer (F3) over the
//! figure, which takes 01_main.png's central rect with the layer shown and fills the window with it hidden, more of the
//! field at the same scale, each point where the shown layout puts it (R-406); the toggle is `ViewUI`'s, sending no
//! `SetField`. The GUI's clock reads `ViewUI`'s transport and advances the playhead (R-101; RQ-246).
//! The keyboard layer (render_gui_spec §G3) takes its keys before egui's pass and moves `ViewUI`'s focus; its ring and
//! the breadcrumb's focus path are drawn only while keyboard navigation is on, which a key press turns on and a click
//! or a drag off, a window the mouse opened taking the focus with no ring (R-460, R-464). The
//! Manifold view, the compass and the figure's gestures send their edits through [`App::set_field`], as any control;
//! a click on the figure, or P over it, pins the sample there in the Trajectory panel through the engine side's
//! `computeIC`, sending none (R-453).
//! The windows (`crate::windows`; §G5, §G7, §G8, §G12) open from their entries and take the focus, which goes back to
//! their opener when they close; an error opens the console by itself, taking no focus (R-405). Export & share's
//! present mode hides all chrome until Esc, which returns the focus to Present (applied per R-369), its figure filling
//! the window as with F3 (R-406). Stain mode draws
//! the node-graph editor (`crate::stain`), whose graph edits are `SetField`s as any control's. Every window but the
//! console is moved and resized by the pointer, its place the GUI's own, saved to the GUI's settings as each drag ends
//! and read back when the next GUI starts (R-461; `crate::settings`). Every panel boundary on the Explore page is a
//! splitter (`crate::layout`), dragged, double-clicked to reset it, and all reset by View › reset layout; the layout is
//! saved to the same settings as each drag ends and at each reset (R-462). The field keeps its place on the screen
//! whatever the splitters, so a resized canvas shows more or less of it at the same scale, and the engine side is told
//! the span shown, computing only what it newly exposes: no `SetField` (R-462 J4).

use std::sync::Arc;

use eframe::egui::{self, Color32, Id, Key, LayerId, Order};
use eframe::egui_wgpu;
use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::set_field::{Edit, HistoryKey, RenderField, SetField};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{
    Backdrop, DebugCategories, Focus, Inspector, KeptOrbits, KeyMode, LinkedViews, Mode, Selection,
    Transport, ViewUI, Window, Windows,
};

use crate::clock::{Clock, Playback};
use crate::explore;
use crate::keyboard::scopes::ScopeId;
use crate::keyboard::Keyboard;
use crate::layout::{Layout, Splits};
use crate::settings::{Settings, Store};
use crate::side::EngineSide;
use crate::stain;
use crate::windows;

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

/// The colour the window is cleared to: what shows wherever nothing is drawn. egui's dark theme's darkest background.
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

/// The warnings and errors logged since the session began (gui_state_contract §2), until the console's clear.
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
    /// The windows' own state: their drafts and choices.
    pub windows: windows::State,
    /// The windows' places: each moved or resized, or remembered (R-461).
    pub places: windows::place::Places,
    /// Where the splitters are (R-462): dragged, reset, or remembered.
    pub splits: Splits,
    /// While a drag holds a splitter: its edge, in points, and its place when the drag began. egui ends one drag, a
    /// frame with none, before it begins another, so this is the held splitter's.
    splitting: Option<(f32, f32)>,
    /// Where the GUI's settings are kept (R-461 I4).
    store: Store,
    /// Whether a save of the settings has failed this session: it is logged once.
    save_failed: bool,
    /// Whether Export & share's present mode hides all chrome (render_gui_spec §G5); Esc returns.
    pub present: bool,
    /// The keyboard layer.
    pub keyboard: Keyboard,
    /// The Manifold view's and the compass's own scratch state.
    pub manifold: explore::manifold_view::Scratch,
    /// The Trajectory panel's own state.
    pub trajectory: explore::trajectory::Scratch,
    /// The Stain editor's own state.
    pub stain: stain::Scratch,
    /// The overlay set Overlays ▾'s save as default kept, for the session (applied per R-369: it is not among the GUI's
    /// settings, which keep the windows' places, R-461; every launch starts with every overlay off).
    pub overlay_default: engine::contract::render_state::Overlays,
    /// The hover last sent: the cursor's figure point, or none.
    hover_sent: Option<[f64; 2]>,
    /// The focus path the windows last scrolled to, and the frames left to keep scrolling to it: a window scrolls to
    /// its focused element only for a few frames after the focus moves there, so the wheel can scroll it away after.
    scrolled_to: (Vec<String>, u8),
    snapshot: Snapshot,
    console: Vec<LogEntry>,
    counts: Counts,
    last_read: Option<f64>,
    reread: bool,
    /// The history key of the pointer's hold, while a button is held: every edit sent from the press to the release goes
    /// under it, so a drag is one history entry (gui_state_contract §2; R-458 F1).
    hold: Option<HistoryKey>,
    /// The last history key taken.
    last_key: u64,
    /// Whether the hold edited the stain graph with auto-recompile on: it recompiles once, at the release.
    recompile_at_release: bool,
    format: wgpu::TextureFormat,
}

/// The app's actions from one frame's controls.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Actions {
    /// Toggle the egui layer, as F3 does.
    pub toggle_layer: bool,
    /// Open the window each of these controls opens, as their clicks did (`windows::opened_by`).
    pub open: Vec<ScopeId>,
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
    /// Toggle the debug view, one `SetField` (View › Debug view, R-467 D5).
    pub debug_view: bool,
    /// Return every splitter to its default (View › reset layout, R-462 J3).
    pub reset_layout: bool,
}

impl<S: EngineSide> App<S> {
    /// The app on `side`, drawing the figure on targets of `format`; it reads its first snapshot at once.
    pub fn new(mut side: S, format: wgpu::TextureFormat) -> Self {
        let mut console = Vec::new();
        let mut counts = Counts::default();
        let snapshot = side.engine().snapshot();
        let errored = absorb(&snapshot, &mut console, &mut counts);
        Self {
            side,
            view: initial_view(),
            clock: Clock::new(),
            shown: true,
            console_open: errored,
            windows: windows::State::default(),
            places: Default::default(),
            splits: Splits::default(),
            splitting: None,
            store: Store::default(),
            save_failed: false,
            present: false,
            scrolled_to: (Vec::new(), 0),
            keyboard: Keyboard::new(),
            manifold: Default::default(),
            trajectory: Default::default(),
            stain: Default::default(),
            overlay_default: Default::default(),
            hover_sent: None,
            snapshot,
            console,
            counts,
            last_read: None,
            reread: false,
            hold: None,
            last_key: 0,
            recompile_at_release: false,
            format,
        }
    }

    /// The app on `side`, as [`App::new`], its settings kept in `store` and read from it now: each window at its
    /// remembered place, each splitter at its remembered place; a file that cannot be read ignored, with one line in the
    /// console (R-461 I4; R-462 J5).
    pub fn with_settings(side: S, format: wgpu::TextureFormat, store: Store) -> Self {
        let mut app = Self::new(side, format);
        let loaded = store.load();
        app.places = loaded.settings.places;
        app.splits = loaded.settings.layout;
        app.store = store;
        if let Some(line) = loaded.ignored {
            app.log_warning(line);
        }
        app
    }

    /// The GUI's settings as they stand: what is saved.
    pub fn settings(&self) -> Settings {
        Settings {
            places: self.places.clone(),
            layout: self.splits,
        }
    }

    /// Saves the GUI's settings to its store; a failure is logged to the console, once a session.
    pub fn save_settings(&mut self) {
        if let Err(why) = self.store.save(&self.settings()) {
            if !std::mem::replace(&mut self.save_failed, true) {
                self.log_warning(why);
            }
        }
    }

    /// Logs `message` to the console as the app's warning, counted in the footer.
    fn log_warning(&mut self, message: String) {
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        self.console.push(LogEntry {
            severity: Severity::Warn,
            at,
            source: Source::App,
            message,
        });
        self.counts.warnings = self.counts.warnings.saturating_add(1);
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

    /// A history key not taken before.
    pub(crate) fn fresh_key(&mut self) -> HistoryKey {
        self.last_key += 1;
        HistoryKey(self.last_key)
    }

    /// Sends one command's edits under one history key, so they are one history entry (gui_state_contract §2; R-458
    /// F1, R-466): the pointer's hold's while a button is held, so every frame of a drag joins one entry, else a fresh
    /// key, ended at once. An undoable edit rereads the snapshot and resyncs the clock, as [`App::set_field`]'s.
    fn send(&mut self, command: Vec<SetField>) {
        if command.is_empty() {
            return;
        }
        let key = match self.hold {
            Some(key) => key,
            None => self.fresh_key(),
        };
        for edit in command {
            if !edit.no_history {
                self.clock.resync();
                self.reread = true;
            }
            self.side.engine().set_field_under(edit, key);
        }
        if self.hold.is_none() {
            self.side.engine().end_key(key);
        }
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

    /// Reads a snapshot: its entries join the console and the counts; an error among them opens the console, taking
    /// no focus (§G12; R-405).
    fn read(&mut self) {
        self.snapshot = self.side.engine().snapshot();
        if absorb(&self.snapshot, &mut self.console, &mut self.counts) {
            self.console_open = true;
        }
    }

    /// Whether `window` is open.
    pub fn is_open(&self, window: Window) -> bool {
        match window {
            Window::Console => self.console_open,
            w => self.view.windows.open.contains(&w),
        }
    }

    /// Opens `window`, opened by control `opener`, and gives it the focus on its first section (render_gui_spec §G3;
    /// R-405); the focus goes back to `opener` when it closes. A window newly open is seeded: the Chart builder from
    /// the chart, Run's `T_horizon` from the horizon, the Inspector from the orbit the Trajectory panel shows.
    pub fn open_window(&mut self, ctx: &egui::Context, window: Window, opener: Option<ScopeId>) {
        if !self.is_open(window) {
            match window {
                Window::ChartBuilder => self.windows.chart.seed(&self.snapshot.sim.plane),
                Window::Run => self.windows.run.seed(&self.snapshot.sim.horizon),
                Window::Inspector => {
                    let shown = explore::trajectory::shown(
                        self.trajectory.tab,
                        self.trajectory.first(),
                        &self.view.kept_orbits,
                    );
                    self.windows.inspector.seed(shown);
                }
                _ => {}
            }
            match window {
                Window::Console => self.console_open = true,
                w => self.view.windows.open.push(w),
            }
        }
        let tree = self.keyboard.tree(self.view.mode);
        let opener = opener.map(|id| tree.path_to(id)).unwrap_or_default();
        let scope = windows::entry(window).scope;
        self.keyboard
            .open_window(ctx, self.view.mode, &mut self.view.focus, scope, opener);
    }

    /// Closes `window`: with the focus in it, the focus goes back to the scope that opened it.
    pub fn close_window(&mut self, ctx: &egui::Context, window: Window) {
        match window {
            Window::Console => self.console_open = false,
            w => self.view.windows.open.retain(|o| *o != w),
        }
        let scope = windows::entry(window).scope;
        self.keyboard
            .close_window(ctx, self.view.mode, &mut self.view.focus, scope);
    }

    /// Before each frame's egui pass: the keyboard layer takes its keys out of `raw` while the layer is shown, and not
    /// in present mode, whose Esc is egui's to read; then it only follows keyboard navigation (R-460, R-464).
    pub fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if self.shown && !self.present {
            self.keyboard.take_keys(ctx, raw);
        } else {
            self.keyboard.follow_input(raw);
        }
    }

    /// Present mode: the figure only, every chrome hidden (render_gui_spec §G5), filling the window as with F3 (R-406);
    /// Esc returns, the focus on Present, in navigation mode (applied per R-369).
    fn presenting(&mut self, ctx: &egui::Context, layout: &Layout) {
        self.keyboard.stand_down();
        self.send_hover(None);
        if self.view.mode == Mode::Explore {
            self.paint_figure(ctx, layout, false);
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.present = false;
            let tree = self.keyboard.tree(self.view.mode);
            self.view.focus.path = tree.path_to(windows::export::PRESENT);
            self.view.focus.mode = KeyMode::Navigation;
        }
    }

    /// One frame on the root `ui`. The first frame a pointer button is held takes the hold's history key, which every
    /// edit sent until the release goes under, the release frame's too; the release frame ends it after its edits, so
    /// the drag's entry closes and the engine logs it once (R-458 F1). A press released within one frame holds nothing:
    /// its edits are commands, each under a fresh key. The snapshot is read again after the frame's edits, so it, the
    /// console and every control read the edit at once.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let down = ctx.input(|i| i.pointer.any_down());
        if self.hold.is_none() && down {
            self.hold = Some(self.fresh_key());
        }
        self.frame(ui);
        if !down {
            if let Some(key) = self.hold.take() {
                self.side.engine().end_key(key);
                self.reread = true;
            }
            if std::mem::take(&mut self.recompile_at_release) {
                self.side.recompile();
            }
        }
        if std::mem::take(&mut self.reread) {
            self.read();
        }
    }

    /// The frame's work on the root `ui`, between the hold's start and end.
    fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.present && ctx.input(|i| i.key_pressed(Key::F3)) {
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
            self.trajectory.first(),
            &self.view.kept_orbits,
        );
        let span = orbit.map_or(0.0, |o| o.path_t);
        let moving = self.trajectory.moving(orbit.is_some())
            || (self.is_open(Window::Inspector) && self.windows.inspector.moving());
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
        self.follow_sample();
        self.trajectory.advance(seconds, span);
        self.windows.inspector.advance(seconds);
        // Only Explore's shown layer lays the splitters' handles, so only there can a drag take one; one that began
        // there is followed to its end, wherever the pointer and the layer then are.
        let held = Layout::with_splits(ui.max_rect(), ctx.pixels_per_point(), &self.splits);
        self.follow_splitter(&ctx, &held);
        let layout = Layout::with_splits(ui.max_rect(), ctx.pixels_per_point(), &self.splits);
        if self.present {
            self.presenting(&ctx, &layout);
            return;
        }
        if self.view.mode == Mode::Explore {
            self.paint_figure(&ctx, &layout, self.shown);
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
        explore::manifold_view::join_targets(
            self.keyboard.tree_mut(Mode::Explore),
            &explore::manifold_view::View::read(&self.snapshot.sim),
        );
        explore::trajectory::join_kept(
            self.keyboard.tree_mut(Mode::Explore),
            self.view.kept_orbits.orbits.len(),
        );
        explore::trajectory::join(self.keyboard.tree_mut(Mode::Explore), &self.trajectory);
        explore::compass::join(
            self.keyboard.tree_mut(Mode::Explore),
            &self.manifold.compass,
            &self.snapshot.sim,
            &mut self.view.focus,
        );
        windows::chart_builder::join_presets(
            self.keyboard.tree_mut(self.view.mode),
            self.windows.chart.presets.len(),
        );
        let content = self.side.stain_content();
        let mut stain_out = stain::Out::default();
        stain::join(
            self.keyboard.tree_mut(Mode::Stain),
            &self.snapshot.render.stain_graph,
            &self.stain,
            content.as_ref(),
        );
        let activated = self
            .keyboard
            .run(&ctx, self.view.mode, &mut self.view.focus, now);
        // The Plücker view's cell cursor follows interaction mode on the compass (R-452 C6).
        explore::compass::follow_focus(
            &self.view.focus,
            &self.snapshot.sim.plane,
            &mut self.manifold.compass,
        );
        let mut window_edits = Vec::new();
        let mut acts = Acts::default();
        for scope in self.keyboard.left().to_vec() {
            if let Some(e) = windows::REGISTRY.iter().find(|e| e.scope == scope) {
                self.close_window(&ctx, e.window);
            }
        }
        for id in &activated {
            // × in a window, Enter on it: the window closes through the window rule, the focus back on its opener.
            if let Some(window) = windows::closed_by(id) {
                self.close_window(&ctx, window);
            }
            self.activate_in_window(id, &mut window_edits, &mut acts);
            if windows::opened_by(id).is_some() {
                actions.open.push(id);
            }
        }
        for id in activated {
            explore::top_bar::activate(id, &mut actions);
            explore::overlays_menu::activate(id, &self.snapshot.render.overlays, &mut actions);
            explore::manifold_view::activate(id, &self.snapshot, &mut self.manifold, &mut out);
            explore::time::activate(id, &self.snapshot, &mut self.view.transport, &mut edits);
            explore::trajectory::activate(id, &mut self.view.kept_orbits, &mut self.trajectory);
            let graph = &self.snapshot.render.stain_graph;
            stain::activate(id, &mut self.stain, graph, content.as_ref(), &mut stain_out);
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
            windows::run::adjust(&adjust, &mut self.windows.run);
            windows::chart_builder::adjust(&adjust, &mut self.windows.chart);
            windows::inspector::adjust(&adjust, &mut self.windows.inspector);
            stain::adjust(
                &adjust,
                &self.stain,
                &self.snapshot.render.stain_graph,
                &mut stain_out,
            );
        }
        for scroll in self.keyboard.scrolled().to_vec() {
            windows::console::scroll(&scroll, &mut self.windows.console, &self.console);
        }
        self.undo_keys(&ctx);
        self.shortcuts(&ctx, &mut actions, &mut edits, &mut out);
        if let Some(due) = self.keyboard.next_repeat_s() {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64((due - now).max(0.0)));
        }
        self.keyboard.clear_places();
        let tree = self.keyboard.tree(self.view.mode);
        // The breadcrumb's focus path is drawn only while keyboard navigation is on, as the ring is (R-460 H4).
        let breadcrumb = if self.keyboard.navigating() {
            explore::breadcrumb::text(tree, &self.view.focus)
        } else {
            String::new()
        };
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
        let focused = self.scroll_focus();
        match self.view.mode {
            Mode::Explore => self.explore(ui, &layout, &mut out, &mut edits),
            Mode::Stain => {
                self.stain_page(
                    ui,
                    &layout,
                    content.as_ref(),
                    focused.clone(),
                    &mut stain_out,
                );
                self.send_hover(None);
            }
        }
        let stain_edited = !stain_out.edits.is_empty();
        edits.append(&mut stain_out.edits);
        // During a hold, a param's drag edits the graph every frame: auto-recompile waits for the hold's end, one
        // recompile per drag (applied per R-369).
        if stain_edited && self.stain.auto_recompile && self.hold.is_some() && !stain_out.recompile
        {
            self.recompile_at_release = true;
        } else if stain_out.recompile || (stain_edited && self.stain.auto_recompile) {
            self.side.recompile();
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
            &mut self.keyboard,
            &mut actions,
        );
        let (closed, held) =
            self.draw_windows(&ctx, &layout, focused, &mut window_edits, &mut acts);
        // A move or resize ended: the places are saved (R-461 I4).
        if !held && self.places.release() {
            self.save_settings();
        }
        for window in closed {
            self.close_window(&ctx, window);
        }
        self.act(&ctx, acts);
        // A click on a text field starts typing in it, on every page and in every window (R-456).
        self.keyboard
            .follow_text_clicks(&ctx, self.view.mode, &mut self.view.focus);
        self.draw_ring(&ctx);
        if let Some(overlays) = actions.overlays.take() {
            edits.push(engine::contract::set_field::SetField {
                edit: Edit::Render(RenderField::Overlays(overlays)),
                no_history: false,
            });
        }
        if std::mem::take(&mut actions.debug_view) {
            edits.push(explore::top_bar::debug_view_edit(
                !self.snapshot.render.debug_view,
            ));
        }
        if std::mem::take(&mut actions.save_overlays) {
            self.overlay_default = self.snapshot.render.overlays.clone();
        }
        let asked = std::mem::take(&mut actions.open);
        self.apply(&ctx, actions);
        for edit in edits {
            self.send(vec![edit]);
        }
        // Locked, every basis edit turns about the pin (render_gui_spec §G4): the Manifold view's and the windows'. Each
        // is sent with its recentring as one command, one history entry.
        out.edits.extend(window_edits);
        let sim = &self.snapshot.sim;
        for command in explore::manifold_view::commands_about_the_pin(out.edits, sim) {
            self.send(command);
        }
        for command in std::mem::take(&mut out.commands) {
            self.send(command);
        }
        // The windows asked for, each with the control that opened it: a click, Enter, or the Manifold view's and the
        // Trajectory panel's own openers.
        let mut opens: Vec<(Window, Option<ScopeId>)> = Vec::new();
        for id in asked {
            if let Some(w) = windows::opened_by(id) {
                opens.push((w, Some(id)));
            }
        }
        for window in std::mem::take(&mut out.windows) {
            let opener = match window {
                Window::ChartBuilder => explore::manifold_view::BUILDER,
                _ => explore::manifold_view::OPEN_INSPECTOR,
            };
            opens.push((window, Some(opener)));
        }
        if let Some(opener) = self.trajectory.open_inspector.take() {
            opens.push((Window::Inspector, Some(opener)));
        }
        for (window, opener) in opens {
            self.open_window(&ctx, window, opener);
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
        // The figure's canvas is `layout.figure`; its points are read in `layout.field`, which no splitter moves.
        let (canvas, field) = (layout.figure, layout.field);
        // Each panel draws in a child of its own, so how many widgets it draws (the trajectory panel's change with the
        // orbit shown) never moves the ids of the widgets drawn after it, and a click that lands as the panel changes
        // still reaches its widget.
        let mut own =
            |salt: &str| ui.new_child(egui::UiBuilder::new().id_salt(salt).max_rect(ui.max_rect()));
        explore::trajectory::show(
            &mut own("trajectory"),
            layout.trajectory,
            &mut explore::trajectory::Panel {
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
        let pin = explore::manifold_view::navigate::figure(
            ui,
            canvas,
            field,
            focused_figure,
            &self.snapshot,
            &mut self.manifold,
            out,
        );
        if let Some(at) = pin {
            self.pin_at(at);
        }
        // The compass takes its drag before the Manifold view is drawn, and the view reads the plane as the drag's
        // edit leaves it, so its sliders show the drag's target in the frame the pointer moves, with the snapshot and
        // the compass's readout (REQ-GUI-206).
        let pending = explore::compass::take(
            ui,
            layout.compass,
            &self.snapshot.sim,
            &mut self.manifold,
            out,
        );
        let targeted;
        let shown = if pending.is_empty() {
            &self.snapshot
        } else {
            let mut snapshot = self.snapshot.clone();
            snapshot.sim.plane = explore::compass::after(&snapshot.sim.plane, &pending);
            targeted = snapshot;
            &targeted
        };
        let mut panel = explore::manifold_view::Panel {
            snapshot: shown,
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
            &pending,
            &mut self.keyboard,
            &mut self.manifold,
            out,
        );
        let span = layout.span(true);
        explore::axis_labels::show(ui, layout.axis_x, layout.axis_y, canvas, &sim.plane, span);
        let reticle = explore::lock::reticle_at(&sim.lock, &sim.plane, field);
        if let Some(at) = reticle.filter(|at| canvas.contains(*at)) {
            explore::lock::reticle(ui.ctx(), canvas, at);
        }
        // The splitters' handles, after the panels, so they take the pointer at the boundaries; a double-click resets
        // one, and the layout is saved (R-462 J3, J5).
        let resets = crate::layout::splitters(ui, layout);
        let default = Layout::new(layout.window, ui.ctx().pixels_per_point());
        for splitter in &resets {
            layout.reset(&default, *splitter, &mut self.splits);
        }
        if !resets.is_empty() {
            self.save_settings();
        }
        let ctx = ui.ctx().clone();
        let pointer = ctx.pointer_hover_pos();
        // A pointer carrying a window, by its title bar or an edge, hovers no figure point (R-461; gui_state_contract
        // §2's hover is the figure point under the cursor), nor one on a splitter or dragging it (R-462).
        let carrying = windows::REGISTRY
            .iter()
            .any(|e| windows::held(&ctx, e).is_some())
            || self.splitting.is_some()
            || pointer.and_then(|p| layout.splitter_at(p)).is_some();
        let over_figure = !carrying
            && pointer
                .and_then(|p| ctx.layer_id_at(p))
                .is_none_or(|layer| layer == ui.layer_id());
        // The figure point is read in the field, on the canvas only.
        self.send_hover(explore::hover_label::figure_point(
            field,
            pointer.filter(|p| canvas.contains(*p)),
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
            explore::hover_label::show(&ctx, canvas, cursor, text);
        }
    }

    /// This frame's splitter drag, read before anything is drawn, so the layout is drawn where the drag puts it, under
    /// the pointer: the splitter at its edge when the drag began moved by the pointer's travel since its press, held to
    /// its range, the panels on each side of it at their minimums (R-462 J1, J2); back where it began, at its place
    /// then. When the drag ends the layout is saved (J5).
    fn follow_splitter(&mut self, ctx: &egui::Context, layout: &Layout) {
        match crate::layout::held(ctx) {
            Some((splitter, delta)) => {
                let (start, from) = *self
                    .splitting
                    .get_or_insert_with(|| (layout.edge(splitter), self.splits.get(splitter)));
                let fraction = if delta == 0.0 {
                    from
                } else {
                    layout.dragged(splitter, start, delta)
                };
                self.splits.set(splitter, fraction);
            }
            None => {
                if self.splitting.take().is_some() {
                    self.save_settings();
                }
            }
        }
    }

    /// The Stain page (`crate::stain`): its controls clicked act through the one path Enter takes.
    fn stain_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &Layout,
        content: Option<&stain::Content>,
        focused: Option<String>,
        out: &mut stain::Out,
    ) {
        let page = stain::Page {
            graph: &self.snapshot.render.stain_graph,
            content,
            preview: self.side.preview().map(|c| (c, self.format)),
            viewport: layout.figure,
        };
        let mut keys = windows::Keys {
            keyboard: &mut self.keyboard,
            focused,
        };
        let mut clicked = Vec::new();
        stain::show(
            ui,
            layout,
            &page,
            &mut self.stain,
            &mut keys,
            &mut clicked,
            out,
        );
        for id in clicked {
            stain::activate(id, &mut self.stain, page.graph, content, out);
        }
    }

    /// The focused scope while a focus that moved is still to be scrolled to: for [`windows::SCROLL_FRAMES`] frames
    /// after it moves (a new window's first frame is egui's sizing pass, and its scroll target is lost); after that
    /// the wheel is the user's.
    fn scroll_focus(&mut self) -> Option<String> {
        if self.scrolled_to.0 != self.view.focus.path {
            self.scrolled_to = (self.view.focus.path.clone(), windows::SCROLL_FRAMES);
        }
        let scrolling = self.scrolled_to.1 > 0;
        self.scrolled_to.1 = self.scrolled_to.1.saturating_sub(1);
        self.view.focus.path.last().filter(|_| scrolling).cloned()
    }

    /// Follows the snapshot's sample into the Trajectory panel: the last hovered kept while the pointer is off the
    /// figure, and the pinned IC re-integrated when what its integration read has changed (R-453 D3, D5).
    fn follow_sample(&mut self) {
        self.trajectory.hovered(self.snapshot.sample.as_ref());
        let inputs = explore::trajectory::Inputs::of(&self.snapshot);
        let side = &mut self.side;
        self.trajectory
            .repin(inputs, |ic, at| side.compute_ic(ic, at));
    }

    /// Pins the sample at the figure point `at`, `(s, t)`, in the Trajectory panel: its IC on the chart as it stands,
    /// integrated through the engine side's `computeIC`, held as `ViewUI` state, sending no `SetField` (R-453 D1, D5).
    /// Nothing when the engine side has no `computeIC`.
    pub fn pin_at(&mut self, at: [f64; 2]) {
        let ic = explore::manifold_view::point(&self.snapshot.sim.plane, at[0], at[1]);
        if let Some(sample) = self.side.compute_ic(&ic, at) {
            let inputs = explore::trajectory::Inputs::of(&self.snapshot);
            self.trajectory.pin(sample, inputs);
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
    /// Space keeps the orbit under the cursor, or the pinned one, and L toggles listen; in Time, Space plays; on the
    /// compass in the Plücker view's interaction mode, Space steers to the cell cursor (R-452 C6); anywhere, Alt+digit
    /// toggles an overlay. Each acts in either mode, but not while a text field holds the keyboard or the `?`
    /// overlay is open. And + and − zoom a Trajectory view in interaction mode on it (R-453 D6).
    /// Ctrl+Z (Cmd+Z): one undo from the contract's history a press, on either page and in either keyboard mode,
    /// except while a text field holds the keyboard, whose own edit it undoes, or while the `?` overlay, which takes
    /// every key, is open (§G3; R-409 G5; R-52). The system's repeats of a held press are not presses.
    fn undo_keys(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() || self.keyboard.shortcuts_open {
            return;
        }
        let presses = ctx.input(|i| {
            i.events
                .iter()
                .filter(|e| {
                    matches!(e, egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. }
                        if crate::keyboard::keymap::undoes(*key, *modifiers))
                })
                .count()
        });
        for _ in 0..presses {
            self.undo();
        }
    }

    fn shortcuts(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Actions,
        edits: &mut Vec<engine::contract::set_field::SetField>,
        out: &mut explore::manifold_view::Out,
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
        // + and − on a Trajectory view in interaction mode zoom it (R-453 D6).
        if self.view.focus.mode == KeyMode::Interaction {
            if let Some(id) = self.view.focus.path.last() {
                for times in explore::trajectory::zoom_presses(&events) {
                    explore::trajectory::zoom_view(id, times, &mut self.trajectory);
                }
            }
        }
        if pressed(Key::Space) {
            match scope {
                Some(explore::FIGURE) => {
                    // The first tab's orbit: the pinned one, or the one under the cursor (R-453 D2).
                    explore::trajectory::keep_first(
                        &mut self.view.kept_orbits,
                        &mut self.trajectory,
                    );
                }
                Some(explore::time::TIME) => {
                    explore::time::toggle_play(&self.snapshot, &mut self.view.transport, edits)
                }
                Some(explore::COMPASS) => explore::compass::space(
                    &self.view.focus,
                    &self.snapshot.sim,
                    &mut self.manifold,
                    out,
                ),
                _ => {}
            }
        }
        if pressed(Key::L) && scope == Some(explore::FIGURE) {
            self.trajectory.listen = !self.trajectory.listen;
        }
    }

    /// Enter on control `id` of a window, as its click: each window acts on its own ids.
    fn activate_in_window(&mut self, id: &str, edits: &mut Vec<SetField>, acts: &mut Acts) {
        let w = &mut self.windows;
        windows::run::activate(id, &mut w.run, &self.snapshot.sim.horizon, edits);
        windows::profiler::activate(id, &mut w.profiler);
        windows::display::activate(id, &mut w.display, &self.snapshot.render.overlays, edits);
        let plane = &self.snapshot.sim.plane;
        windows::chart_builder::activate(id, &mut w.chart, plane, edits);
        windows::inspector::activate(id, &mut w.inspector, &mut self.view.kept_orbits, edits);
        let (sim, render) = (&self.snapshot.sim, &self.snapshot.render);
        if let Some(act) = windows::export::activate(id, &mut w.export, sim, render) {
            acts.export.push(act);
        }
        if let Some(act) = windows::console::activate(id, &mut w.console, &self.console) {
            acts.console.push(act);
        }
    }

    /// Draws the open windows, the console last, each at its place (R-461): moved or resized by a drag on its title bar
    /// or its edges. Returns those closed by their ×, and whether a drag holds one.
    fn draw_windows(
        &mut self,
        ctx: &egui::Context,
        layout: &Layout,
        focused: Option<String>,
        edits: &mut Vec<SetField>,
        acts: &mut Acts,
    ) -> (Vec<Window>, bool) {
        let mut closed = Vec::new();
        let mut held = false;
        let mut open = self.view.windows.open.clone();
        if self.console_open {
            open.push(Window::Console);
        }
        let ppp = ctx.pixels_per_point();
        let view_px =
            [layout.figure.width(), layout.figure.height()].map(|v| (v * ppp).round() as u32);
        let quick = self.side.canvas().map(|c| (c, self.format));
        let mut keys = windows::Keys {
            keyboard: &mut self.keyboard,
            focused,
        };
        for window in open {
            let e = windows::entry(window);
            let default = windows::rect(window, layout);
            let mut rect = if windows::place::movable(window) {
                self.places.shown(window, default, layout.window)
            } else {
                default
            };
            // This frame's drag first, so the window is drawn under the pointer, never a frame behind it.
            if let Some(grip) = windows::held(ctx, e) {
                let total = ctx
                    .input(|i| i.pointer.total_drag_delta())
                    .unwrap_or_default();
                self.places.drag(window, grip, rect, total, layout.window);
                rect = self.places.shown(window, default, layout.window);
                held = true;
            }
            let w = &mut self.windows;
            let snapshot = &self.snapshot;
            let still = windows::frame(
                ctx,
                e,
                rect,
                &mut keys,
                window != Window::Console,
                |ui, keys| match window {
                    Window::Run => {
                        windows::run::show(ui, keys, &mut w.run, &snapshot.sim.horizon, edits)
                    }
                    Window::Profiler => {
                        windows::profiler::show(ui, keys, &mut w.profiler, self.side.trace())
                    }
                    Window::Export => {
                        let state = (&snapshot.sim, &snapshot.render);
                        if let Some(act) =
                            windows::export::show(ui, keys, &mut w.export, state, view_px)
                        {
                            acts.export.push(act);
                        }
                    }
                    Window::Display => windows::display::show(
                        ui,
                        keys,
                        &mut w.display,
                        &snapshot.render.overlays,
                        edits,
                    ),
                    Window::ChartBuilder => {
                        let view = windows::chart_builder::View {
                            plane: &snapshot.sim.plane,
                            quick: quick.clone(),
                        };
                        windows::chart_builder::show(ui, keys, &mut w.chart, &view, edits)
                    }
                    Window::Inspector => windows::inspector::show(
                        ui,
                        keys,
                        &mut w.inspector,
                        (&mut self.view.kept_orbits, edits),
                    ),
                    Window::Console => {
                        let view = windows::console::View {
                            entries: &self.console,
                            counts: self.counts,
                        };
                        let shown = windows::console::show(ui, keys, &mut w.console, &view);
                        acts.console.extend(shown);
                    }
                },
            );
            if !still {
                closed.push(window);
            }
        }
        (closed, held)
    }

    /// Carries out what the windows' controls asked: a copy to the clipboard, the console's clear, present mode.
    fn act(&mut self, ctx: &egui::Context, acts: Acts) {
        for act in acts.console {
            match act {
                windows::console::Act::Copy(text) => ctx.copy_text(text),
                windows::console::Act::Clear => {
                    self.console.clear();
                    self.counts = Counts::default();
                }
                windows::console::Act::Close => self.close_window(ctx, Window::Console),
                windows::console::Act::Focus(id) => self.keyboard.focus_by_click(
                    ctx,
                    self.view.mode,
                    &mut self.view.focus,
                    windows::console::WINDOW,
                    id,
                ),
            }
        }
        for act in acts.export {
            match act {
                windows::export::Act::Copy(text) => ctx.copy_text(text),
                windows::export::Act::Present => self.present = true,
            }
        }
    }

    /// Draws the focus ring on the focused scope, where it was drawn this frame, while keyboard navigation is on
    /// (R-460 H4, R-464).
    fn draw_ring(&self, ctx: &egui::Context) {
        if !self.keyboard.navigating() {
            return;
        }
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
        if let Some(mode) = actions.mode {
            self.view.mode = mode;
        }
        if actions.shortcuts {
            self.keyboard.shortcuts_open = true;
        }
        if actions.reset_layout {
            self.splits = Splits::default();
            self.save_settings();
        }
        if actions.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Draws the figure through the engine side's figure canvas, above the regions' frames, so nothing of theirs
    /// reaches it. The canvas draws over the whole window, the field at the default layout's placement, and is clipped
    /// to the figure's rect as it stands: the canvas with the egui layer `shown`, else the window (R-406). So over the
    /// canvas the two states draw the same pixels, and a splitter that resizes the canvas re-frames the figure, never
    /// moving or scaling the field (R-462 J4). The engine side is told the span shown first, to compute only what it
    /// does not hold, then the figure's view with its background (R-467 D2).
    fn paint_figure(&mut self, ctx: &egui::Context, layout: &Layout, shown: bool) {
        self.side.show_span(layout.span(shown));
        let background = clear_colour(&ctx.global_style().visuals).to_array();
        let [r, g, b, _] = background;
        let view = crate::side::FigureView::of(layout, shown, ctx.pixels_per_point(), [r, g, b]);
        self.side.figure_view(view);
        let Some(canvas) = self.side.figure(layout.placement()) else {
            return;
        };
        let callback = egui_wgpu::Callback::new_paint_callback(
            layout.window,
            FigureCallback {
                canvas,
                format: self.format,
            },
        );
        ctx.layer_painter(LayerId::new(Order::Middle, Id::new("figure")))
            .with_clip_rect(layout.figure_rect(shown))
            .add(callback);
    }
}

/// What the windows' controls asked of the app this frame.
#[derive(Debug, Default)]
struct Acts {
    console: Vec<windows::console::Act>,
    export: Vec<windows::export::Act>,
}

/// Adds `snapshot`'s log entries to the console and the counts; returns whether an error was among them.
fn absorb(snapshot: &Snapshot, console: &mut Vec<LogEntry>, counts: &mut Counts) -> bool {
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
    snapshot.log.iter().any(|e| e.severity == Severity::Error)
}

/// The figure, drawn by the canvas into egui-wgpu's pass with its viewport set to the callback's rect: the figure's,
/// or the Chart builder's quick render's.
pub(crate) struct FigureCallback {
    pub(crate) canvas: Arc<dyn Canvas>,
    pub(crate) format: wgpu::TextureFormat,
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
