//! The v1 web skeleton's measurement panel and scripted pan (TASK-M8-86; R-627), the same in both builds: opened by
//! `?measure` in the browser and by `PRIN_MEASURE=1` natively, it reports what TASK-M8-96 measures with. It is beside the
//! figure, never over it (render_gui_spec G1): the app runs in the window less the panel's strip. It is not R-467's
//! View › Debug view: opening it sends no `SetField` and adds no undo entry. A run is the scripted pan or idle (no
//! input), repainting every frame throughout, for the warm-up and the window of [`stats`].
//!
//! The panel joins both keyboard trees (render_gui_spec §G3; R-390, R-409) while it is open: a big scope after the
//! footer, its two buttons inside it, Enter on a button acting as its click. The app's frame clears the scopes' places
//! and draws the ring on what it drew, so the panel draws after the app's frame, and draws its own ring.
//!
//! A frame's times come from two paint callbacks, the frame's first and last: the work time runs from the frame's
//! start, the app's input hook, to the last callback's paint, the last thing that runs before the submit (eframe has no
//! hook past it, and a callback is the one mechanism all three drivers share); the surface wait is what lies between the
//! buffers' update and the first paint, `get_current_texture` being there.

pub mod stats;

use std::sync::{Arc, Mutex};

use eframe::egui::{self, vec2, Button, Id, LayerId, Order, Rect, RichText, UiBuilder};
use eframe::egui_wgpu;
use engine::contract::set_field::{Edit, SetField};
use engine::contract::sim_config::Plane;
use engine::contract::view_ui::Mode;
use serde_json::{json, Value};

use crate::app::App;
use crate::clock::time::Instant;
use crate::explore::manifold_view::navigate;
use crate::keyboard::scopes::{Scope, ScopeId};
use crate::side::EngineSide;
use stats::{window_stats, Frame, WindowStats, WARM_UP_MS, WINDOW_MS};

/// The environment variable that opens the panel natively: `1` opens it, and anything else leaves the app as it is.
pub const MEASURE_VAR: &str = "PRIN_MEASURE";
/// The environment variable that adds a fixed delay, in ms, to each frame's work natively (`?frame_delay_ms=` in the
/// browser).
pub const DELAY_VAR: &str = "PRIN_FRAME_DELAY_MS";
/// The environment variable that starts a run natively as the panel opens, `pan` or `idle` (`?run=` in the browser).
pub const RUN_VAR: &str = "PRIN_MEASURE_RUN";

/// The panel's width, in points.
pub const PANEL_WIDTH: f32 = 300.0;
/// The panel's keyboard scope: the big scope after the footer.
pub const PANEL_SCOPE: ScopeId = "measure panel";
/// The scope of the button that runs the scripted pan.
pub const RUN_PAN: ScopeId = "measure run pan";
/// The scope of the button that runs idle.
pub const RUN_IDLE: ScopeId = "measure run idle";

/// The time the scripted pan takes over one figure width, one way, in ms.
pub const PAN_LEG_MS: f64 = 5_000.0;

/// A run's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// The view panned at a constant rate, back and forth.
    Pan,
    /// No input.
    Idle,
}

impl Run {
    /// The name flags and the report use.
    pub fn name(self) -> &'static str {
        match self {
            Run::Pan => "pan",
            Run::Idle => "idle",
        }
    }

    /// Its button's scope.
    fn scope(self) -> ScopeId {
        match self {
            Run::Pan => RUN_PAN,
            Run::Idle => RUN_IDLE,
        }
    }

    /// Its button's label, and its name in the breadcrumb.
    fn label(self) -> &'static str {
        match self {
            Run::Pan => "Run the scripted pan",
            Run::Idle => "Run idle",
        }
    }

    /// The run `value` names, or why not.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pan" => Ok(Run::Pan),
            "idle" => Ok(Run::Idle),
            other => Err(format!("the run {other:?} is neither pan nor idle")),
        }
    }
}

/// How far the scripted pan is `elapsed_ms` into a run, in figure widths: from 0 to 1 and back at a constant rate.
pub fn pan_offset(elapsed_ms: f64) -> f64 {
    let leg = elapsed_ms.rem_euclid(2.0 * PAN_LEG_MS) / PAN_LEG_MS;
    if leg <= 1.0 {
        leg
    } else {
        2.0 - leg
    }
}

/// The edit that puts the view `offset` figure widths along from `plane`, as a drag that far does (`navigate::pan`),
/// with no history: a scripted pan leaves no undo entries. From the run's first plane, not the snapshot's, which is read
/// at 10 Hz, so the pan neither stutters with it nor drifts.
pub fn pan_edit(plane: &Plane, offset: f64) -> SetField {
    let edit: Edit = navigate::pan(plane, vec2(offset as f32, 0.0), vec2(1.0, 1.0)).edit;
    SetField {
        edit,
        no_history: true,
    }
}

/// What the panel reports of where it runs. A browser-only item is `None` natively.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Environment {
    /// `crossOriginIsolated`.
    pub cross_origin_isolated: Option<bool>,
    /// Whether `SharedArrayBuffer` exists.
    pub shared_array_buffer: Option<bool>,
    /// The wasm-threads probe's result: `ok`, or why not.
    pub wasm_threads: Option<String>,
    /// The layout the app runs in: `worker`, `main` or `native`.
    pub layout: String,
    /// Why the worker layout failed, when the app fell back from it.
    pub fallback_reason: Option<String>,
    /// The display's refresh rate, measured apart from the app's work, in Hz.
    pub refresh_hz: Option<f64>,
    /// The fixed delay added to each frame's work, in ms.
    pub frame_delay_ms: f64,
    /// The run to start as the panel opens.
    pub run: Option<Run>,
}

/// A frame delay flag's value, in ms: unset is none; anything but a finite number not below 0 is refused.
pub fn frame_delay(value: Option<&str>) -> Result<f64, String> {
    let Some(value) = value else {
        return Ok(0.0);
    };
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|ms| ms.is_finite() && *ms >= 0.0)
        .ok_or_else(|| format!("the frame delay {value:?} is not a number of ms, 0 or more"))
}

impl Environment {
    /// The native environment, from the values of [`MEASURE_VAR`], [`DELAY_VAR`] and [`RUN_VAR`], and the monitor's
    /// rate: `None` unless the first is `1`.
    pub fn native(
        measure: Option<&str>,
        delay: Option<&str>,
        run: Option<&str>,
        refresh_hz: Option<f64>,
    ) -> Result<Option<Self>, String> {
        if measure != Some("1") {
            return Ok(None);
        }
        Ok(Some(Self {
            layout: "native".to_owned(),
            refresh_hz,
            frame_delay_ms: frame_delay(delay)?,
            run: run.map(Run::parse).transpose()?,
            ..Self::default()
        }))
    }

    /// The browser's environment, from the loader's JSON: `crossOriginIsolated`, `sharedArrayBuffer`, `wasmThreads`,
    /// `layout`, `fallbackReason`, `idleLoopMs` (the empty `requestAnimationFrame` loop's stamps), `frameDelayMs` and
    /// `run`, the last two the URL's text.
    pub fn browser(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let string = |key: &str| value[key].as_str().map(str::to_owned);
        let stamps: Vec<f64> = value["idleLoopMs"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default();
        Ok(Self {
            cross_origin_isolated: value["crossOriginIsolated"].as_bool(),
            shared_array_buffer: value["sharedArrayBuffer"].as_bool(),
            wasm_threads: string("wasmThreads"),
            layout: string("layout").ok_or("the loader named no layout")?,
            fallback_reason: string("fallbackReason"),
            refresh_hz: stats::refresh_hz(&stamps),
            frame_delay_ms: frame_delay(value["frameDelayMs"].as_str())?,
            run: value["run"].as_str().map(Run::parse).transpose()?,
        })
    }
}

/// Where a session is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// No run yet.
    Ready,
    /// A run, since `start_ms`.
    Running {
        /// Its kind.
        run: Run,
        /// When it started, on the session's clock.
        start_ms: f64,
    },
    /// A run ended; `None` had no frame in its window.
    Done {
        /// Its kind.
        run: Run,
        /// Its figures.
        stats: Option<WindowStats>,
    },
}

/// What a session asks of the frame it steps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Step {
    /// A run started this frame.
    pub started: bool,
    /// The pan's offset, in figure widths, during a pan.
    pub pan: Option<f64>,
    /// A run ended this frame.
    pub finished: bool,
}

/// The times of the frame in flight, in ms on the session's clock.
#[derive(Clone, Copy, Debug, Default)]
struct Times {
    start: Option<f64>,
    prepared: Option<f64>,
    first_paint: Option<f64>,
    last_paint: Option<f64>,
}

/// The session's clock and the frame's times, shared with the paint callbacks.
#[derive(Debug)]
struct Marks {
    origin: Instant,
    times: Mutex<Times>,
}

impl Marks {
    fn now_ms(&self) -> f64 {
        self.origin.elapsed().as_secs_f64() * 1000.0
    }

    fn times(&self) -> std::sync::MutexGuard<'_, Times> {
        self.times.lock().expect("the frame times' lock")
    }
}

/// A measurement session: its environment, and its current or last run.
#[derive(Debug)]
pub struct Session {
    /// Where it runs.
    pub env: Environment,
    /// Where it is.
    pub phase: Phase,
    /// The size of the app's part of the window, in points, as of the last frame: what the two builds' figures are laid
    /// out in, so a display too small for the window a run asked for shows in the fields.
    pub app_size: Option<egui::Vec2>,
    /// The run a button or a flag asked for, starting at the next step.
    pending: Option<Run>,
    frames: Vec<Frame>,
    marks: Arc<Marks>,
}

impl Session {
    /// A session in `env`, starting `env.run` at its first step.
    pub fn new(env: Environment) -> Self {
        Self {
            pending: env.run,
            env,
            phase: Phase::Ready,
            app_size: None,
            frames: Vec::new(),
            marks: Arc::new(Marks {
                origin: Instant::now(),
                times: Mutex::default(),
            }),
        }
    }

    /// The session's clock, in ms.
    pub fn now_ms(&self) -> f64 {
        self.marks.now_ms()
    }

    /// Whether a run is under way.
    pub fn running(&self) -> bool {
        matches!(self.phase, Phase::Running { .. })
    }

    /// Asks for a run, unless one is under way.
    pub fn request(&mut self, run: Run) {
        if !self.running() {
            self.pending = Some(run);
        }
    }

    /// The frames the current or last run recorded.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Records a frame during a run, its stamps on the session's clock: the frame finished, at `times`.
    pub(crate) fn record(&mut self, start: f64, prepared: f64, first_paint: f64, last_paint: f64) {
        if let Phase::Running { start_ms, .. } = self.phase {
            self.frames.push(Frame {
                at_ms: last_paint - start_ms,
                work_ms: last_paint - start,
                acquire_ms: first_paint - prepared,
            });
        }
    }

    /// The start of a frame, before its input is read: the last frame's times, complete now, are recorded, this
    /// frame's clock starts, and the delay flag's fixed delay is spent, as work, spinning: the main thread cannot sleep.
    pub fn begin_frame(&mut self) {
        let last = std::mem::take(&mut *self.marks.times());
        if let Times {
            start: Some(start),
            prepared: Some(prepared),
            first_paint: Some(first_paint),
            last_paint: Some(last_paint),
        } = last
        {
            self.record(start, prepared, first_paint, last_paint);
        }
        let start = self.now_ms();
        self.marks.times().start = Some(start);
        while self.now_ms() - start < self.env.frame_delay_ms {
            std::hint::spin_loop();
        }
    }

    /// Moves the session on to `now_ms`: a run asked for starts, and a run past its warm-up and its window ends.
    pub fn step(&mut self, now_ms: f64) -> Step {
        let mut step = Step::default();
        if let Some(run) = self.pending.take() {
            self.frames.clear();
            self.phase = Phase::Running {
                run,
                start_ms: now_ms,
            };
            step.started = true;
        }
        if let Phase::Running { run, start_ms } = self.phase {
            let elapsed = now_ms - start_ms;
            if elapsed >= WARM_UP_MS + WINDOW_MS {
                self.phase = Phase::Done {
                    run,
                    stats: window_stats(&self.frames, self.env.refresh_hz),
                };
                step.finished = true;
            } else if run == Run::Pan {
                step.pan = Some(pan_offset(elapsed));
            }
        }
        step
    }

    /// The figures of the last run, if it ended with any.
    fn stats(&self) -> Option<WindowStats> {
        match self.phase {
            Phase::Done { stats, .. } => stats,
            _ => None,
        }
    }

    /// The name of the phase, and of the run.
    fn status(&self) -> (&'static str, Option<&'static str>) {
        match self.phase {
            Phase::Ready => ("ready", None),
            Phase::Running { run, .. } => ("running", Some(run.name())),
            Phase::Done { run, .. } => ("done", Some(run.name())),
        }
    }

    /// The panel's fields as JSON, what the runner scripts write: every key in every phase, `null` where it has no value.
    pub fn report(&self) -> Value {
        let (status, run) = self.status();
        let stats = self.stats();
        let env = &self.env;
        json!({
            "status": status,
            "run": run,
            "build": build_name(cfg!(debug_assertions)),
            "cross_origin_isolated": env.cross_origin_isolated,
            "shared_array_buffer": env.shared_array_buffer,
            "wasm_threads": env.wasm_threads,
            "layout": env.layout,
            "fallback_reason": env.fallback_reason,
            "refresh_hz": env.refresh_hz,
            "frame_delay_ms": env.frame_delay_ms,
            "app_width": self.app_size.map(|s| s.x),
            "app_height": self.app_size.map(|s| s.y),
            "frames": stats.map(|s| s.frames),
            "mean_fps": stats.map(|s| s.mean_fps.fps),
            "mean_fps_capped": stats.map(|s| s.mean_fps.capped),
            "p5_fps": stats.map(|s| s.p5_fps.fps),
            "p5_fps_capped": stats.map(|s| s.p5_fps.capped),
            "work_mean_ms": stats.map(|s| s.work_mean_ms),
            "work_p95_ms": stats.map(|s| s.work_p95_ms),
            "acquire_mean_ms": stats.map(|s| s.acquire_mean_ms),
            "work_less_wait_p95_ms": stats.map(|s| s.work_less_wait_p95_ms),
        })
    }

    /// The panel's rows, label and value: the report's fields, as text.
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        fn yes_no(v: Option<bool>) -> String {
            v.map_or("n/a".to_owned(), |v| {
                if v { "yes" } else { "no" }.to_owned()
            })
        }
        fn ms(v: Option<f64>) -> String {
            v.map_or("–".to_owned(), |v| format!("{v:.2} ms"))
        }
        fn fps(v: Option<stats::Figure>) -> String {
            v.map_or("–".to_owned(), |f| {
                let mark = if f.capped { " capped" } else { "" };
                format!("{:.1}{mark}", f.fps)
            })
        }
        let (status, run) = self.status();
        let stats = self.stats();
        let env = &self.env;
        vec![
            ("status", format!("{status} {}", run.unwrap_or_default())),
            ("build", build_name(cfg!(debug_assertions)).to_owned()),
            ("crossOriginIsolated", yes_no(env.cross_origin_isolated)),
            ("SharedArrayBuffer", yes_no(env.shared_array_buffer)),
            (
                "wasm threads",
                env.wasm_threads.clone().unwrap_or("n/a".to_owned()),
            ),
            ("layout", env.layout.clone()),
            (
                "fallback reason",
                env.fallback_reason.clone().unwrap_or("–".to_owned()),
            ),
            (
                "refresh rate",
                env.refresh_hz
                    .map_or("–".to_owned(), |hz| format!("{hz:.1} Hz")),
            ),
            ("frame delay", format!("{:.0} ms", env.frame_delay_ms)),
            (
                "app size",
                self.app_size
                    .map_or("–".to_owned(), |s| format!("{:.0} × {:.0} pt", s.x, s.y)),
            ),
            (
                "frames",
                stats.map_or("–".to_owned(), |s| s.frames.to_string()),
            ),
            ("mean fps", fps(stats.map(|s| s.mean_fps))),
            ("5th-percentile fps", fps(stats.map(|s| s.p5_fps))),
            ("work mean", ms(stats.map(|s| s.work_mean_ms))),
            ("work p95", ms(stats.map(|s| s.work_p95_ms))),
            ("surface wait mean", ms(stats.map(|s| s.acquire_mean_ms))),
            (
                "work less wait p95",
                ms(stats.map(|s| s.work_less_wait_p95_ms)),
            ),
        ]
    }
}

/// The name a report gives its build. The site is built with `--release` and a plain native `cargo run` is unoptimised,
/// so a run says which it measured: its figures read beside another run's only on the same build.
pub(crate) fn build_name(debug_assertions: bool) -> &'static str {
    if debug_assertions {
        "debug"
    } else {
        "release"
    }
}

/// `window` split into the app's part and the panel's strip, on its right, side by side and never overlapping.
pub fn split(window: Rect) -> (Rect, Rect) {
    let x = (window.max.x - PANEL_WIDTH).max(window.min.x);
    let mut app = window;
    app.max.x = x;
    let mut panel = window;
    panel.min.x = x;
    (app, panel)
}

/// Draws the panel in `ui`, filling `rect`; a button clicked asks for its run. Returns the buttons, with their scopes.
fn panel(ui: &mut egui::Ui, rect: Rect, session: &mut Session) -> Vec<(ScopeId, egui::Response)> {
    let fill = ui.visuals().panel_fill;
    ui.painter().rect_filled(rect, 0.0, fill);
    let mut ui = ui.new_child(
        UiBuilder::new()
            .id_salt("measure panel")
            .max_rect(rect.shrink(8.0)),
    );
    ui.label(RichText::new("Measurement").strong());
    egui::Grid::new("measure rows")
        .num_columns(2)
        .show(&mut ui, |ui| {
            for (label, value) in session.rows() {
                ui.label(label);
                ui.label(value);
                ui.end_row();
            }
        });
    ui.add_space(8.0);
    let free = !session.running();
    [Run::Pan, Run::Idle]
        .map(|run| {
            let response = ui.add_enabled(free, Button::new(run.label()));
            if response.clicked() {
                session.request(run);
            }
            (run.scope(), response)
        })
        .into()
}

/// The paint callback that stamps a frame's times; see the module's note.
struct Mark {
    marks: Arc<Marks>,
    first: bool,
}

impl egui_wgpu::CallbackTrait for Mark {
    fn finish_prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
        _resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let now = self.marks.now_ms();
        self.marks.times().prepared = Some(now);
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        _render_pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        let now = self.marks.now_ms();
        let mut times = self.marks.times();
        if self.first {
            times.first_paint = Some(now);
        } else {
            times.last_paint = Some(now);
        }
    }
}

/// The app, with the panel when a session is open; with none it is the app as it is.
pub struct Measured<S: EngineSide> {
    /// The app.
    pub app: App<S>,
    /// The session.
    pub session: Option<Session>,
    /// The plane a pan runs from.
    from: Option<Plane>,
    /// Told the report, as JSON, when the panel opens and when a run ends.
    sink: Box<dyn FnMut(String)>,
    /// Whether the report has been sent since the panel opened.
    reported: bool,
}

impl<S: EngineSide> Measured<S> {
    /// `app` under `session`, its reports told to `sink`; with a session, the panel joins both keyboard trees.
    pub fn new(mut app: App<S>, session: Option<Session>, sink: Box<dyn FnMut(String)>) -> Self {
        if session.is_some() {
            for mode in [Mode::Explore, Mode::Stain] {
                let tree = app.keyboard.tree_mut(mode);
                tree.register(None, Scope::group(PANEL_SCOPE, "Measurement"));
                for run in [Run::Pan, Run::Idle] {
                    tree.register(Some(PANEL_SCOPE), Scope::control(run.scope(), run.label()));
                }
            }
        }
        Self {
            app,
            session,
            from: None,
            sink,
            reported: false,
        }
    }

    /// The app's input hook, the frame's start.
    pub fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some(session) = &mut self.session {
            session.begin_frame();
        }
        self.app.raw_input_hook(ctx, raw);
    }

    /// One frame on the root `ui`.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if self.session.is_none() {
            self.app.ui(ui);
            return;
        }
        let window = ui.max_rect();
        let (app_rect, panel_rect) = split(window);
        let ctx = ui.ctx().clone();
        let session = self.session.as_mut().expect("checked above");
        session.app_size = Some(app_rect.size());
        let step = session.step(session.now_ms());
        if session.running() {
            ctx.request_repaint();
        }
        let marks = session.marks.clone();
        let report = step.finished || !self.reported;
        if report {
            self.reported = true;
            (self.sink)(session.report().to_string());
        }
        if step.started {
            self.from = Some(self.app.snapshot().sim.plane.clone());
        }
        if let (Some(offset), Some(plane)) = (step.pan, &self.from) {
            self.app.set_field(pan_edit(plane, offset));
        }
        self.app
            .ui(&mut ui.new_child(UiBuilder::new().id_salt("measured app").max_rect(app_rect)));
        for run in [Run::Pan, Run::Idle] {
            if self.app.keyboard.activated().contains(&run.scope()) {
                session.request(run);
            }
        }
        let buttons = panel(ui, panel_rect, session);
        // A run asked for starts next frame; the layer's keys are out of egui's input, so nothing else asks for one.
        if session.pending.is_some() {
            ctx.request_repaint();
        }
        // The places only while the layer runs: nothing clears them otherwise.
        if self.app.shown && !self.app.present {
            let keyboard = &mut self.app.keyboard;
            keyboard.place(PANEL_SCOPE, panel_rect);
            for (scope, response) in &buttons {
                keyboard.place_widget(scope, response);
            }
            if self.app.view.focus.path.first().map(String::as_str) == Some(PANEL_SCOPE) {
                self.app.draw_ring(&ctx);
            }
        }
        for (layer, first) in [
            (LayerId::background(), true),
            (LayerId::new(Order::Debug, Id::new("measure last")), false),
        ] {
            let mark = Mark {
                marks: marks.clone(),
                first,
            };
            ctx.layer_painter(layer)
                .add(egui_wgpu::Callback::new_paint_callback(window, mark));
        }
    }
}

impl<S: EngineSide + 'static> eframe::App for Measured<S> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        Measured::ui(self, ui);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        Measured::raw_input_hook(self, ctx, raw_input);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        crate::app::clear_colour(visuals).to_normalized_gamma_f32()
    }
}
