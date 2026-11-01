//! `gui --bench console-open-close` (R-463 K4; REQ-GUI-216; render_gui_spec §G12): the time of the frames that open and
//! close the console, and of the frames around them, with where the time goes, at the three log sizes. It runs the app
//! on the mock as the window does (a click on the footer opens the console, a click on its × closes it, at 01_main.png's
//! size, AccessKit off and one frame every 1/60 s of input time), and draws each frame through egui-wgpu on the mock's
//! device, offscreen: the surface's present and the wait for vsync are not in it. The first cycle at each size starts
//! from a new GUI, so it holds what a first opening pays; the rest are the usual case. The numbers mean something only
//! in a release build on the human's Mac (R-186).

use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2};
use eframe::egui_wgpu;
use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;

use crate::app::{App, CONSOLE_ROWS};
use crate::capture::{footer_point, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::side::{EngineSide, MockSide};
use crate::windows::console;

/// The benchmark's name on the command line.
pub const NAME: &str = "console-open-close";
/// The log sizes it measures at (R-463 K4): empty, 1 000 entries, and the most the console keeps.
pub const LOG_SIZES: [usize; 3] = [0, 1_000, CONSOLE_ROWS];
/// The target for every frame measured, in milliseconds: one frame at 60 fps (R-463).
pub const TARGET_MS: f64 = 1000.0 / 60.0;
/// The open-and-close cycles at each size.
pub const CYCLES: usize = 15;
/// The input time between frames, in seconds: 60 fps.
const FRAME_S: f64 = 1.0 / 60.0;
/// The frames between one measured stretch and the next, so that nothing the last action started is still running.
const SETTLE: usize = 10;

/// The frames each cycle measures, in order: the input frame of each action and the two after it, with one idle frame
/// before each.
pub const FRAMES: [&str; 8] = [
    "closed, before",
    "open: click",
    "open: +1",
    "open: +2",
    "open, before",
    "close: click",
    "close: +1",
    "close: +2",
];

/// One frame's time and where it went, in milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTime {
    /// egui's passes less the two below: laying out and painting everything else.
    pub layout_ms: f64,
    /// The console's filter of the log.
    pub filter_ms: f64,
    /// The console's rows.
    pub rows_ms: f64,
    /// Tessellating the frame's shapes.
    pub tessellate_ms: f64,
    /// Uploading and drawing the frame through wgpu, waiting for the device to finish.
    pub wgpu_ms: f64,
    /// egui's passes this frame: 2 where something asked to discard the first.
    pub passes: usize,
}

impl FrameTime {
    /// The whole frame.
    pub fn total_ms(&self) -> f64 {
        self.layout_ms + self.filter_ms + self.rows_ms + self.tessellate_ms + self.wgpu_ms
    }
}

/// One measured frame over every cycle at one log size.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Which frame, from [`FRAMES`].
    pub frame: &'static str,
    /// Its time in each cycle, in order.
    pub times: Vec<FrameTime>,
}

impl Row {
    /// The first cycle's total.
    pub fn first_ms(&self) -> f64 {
        self.times.first().map_or(0.0, FrameTime::total_ms)
    }

    /// The median total.
    pub fn median_ms(&self) -> f64 {
        let mut totals: Vec<f64> = self.times.iter().map(FrameTime::total_ms).collect();
        totals.sort_by(f64::total_cmp);
        totals.get(totals.len() / 2).copied().unwrap_or(0.0)
    }

    /// The slowest cycle's time.
    pub fn slowest(&self) -> FrameTime {
        self.times
            .iter()
            .copied()
            .max_by(|a, b| a.total_ms().total_cmp(&b.total_ms()))
            .unwrap_or_default()
    }
}

/// Every measured frame at one log size.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    /// The entries in the log.
    pub log: usize,
    /// One row for each of [`FRAMES`].
    pub rows: Vec<Row>,
}

impl Table {
    /// The slowest frame measured, and its time in milliseconds.
    pub fn worst(&self) -> (&'static str, f64) {
        self.rows
            .iter()
            .map(|r| (r.frame, r.slowest().total_ms()))
            .fold(("", 0.0), |a, b| if b.1 > a.1 { b } else { a })
    }

    /// Whether every frame measured is within [`TARGET_MS`].
    pub fn meets_target(&self) -> bool {
        self.worst().1 < TARGET_MS
    }
}

/// The table's text, as the PR gives it.
pub fn render_table(table: &Table) -> String {
    let mut text = format!(
        "log {} entries\n{:<16}{:>9}{:>9}{:>9} | slowest:{:>8}{:>8}{:>8}{:>8}{:>8}{:>7}\n",
        table.log,
        "frame",
        "first",
        "median",
        "max",
        "layout",
        "filter",
        "rows",
        "tess",
        "wgpu",
        "pass"
    );
    for row in &table.rows {
        let s = row.slowest();
        text += &format!(
            "{:<16}{:>9.2}{:>9.2}{:>9.2} |         {:>8.2}{:>8.2}{:>8.2}{:>8.2}{:>8.2}{:>7}\n",
            row.frame,
            row.first_ms(),
            row.median_ms(),
            s.total_ms(),
            s.layout_ms,
            s.filter_ms,
            s.rows_ms,
            s.tessellate_ms,
            s.wgpu_ms,
            s.passes
        );
    }
    let (frame, ms) = table.worst();
    let verdict = if table.meets_target() {
        "met"
    } else {
        "MISSED"
    };
    text += &format!("target {TARGET_MS:.1} ms: {verdict}, the worst {ms:.2} ms at `{frame}`\n");
    text
}

/// `seconds` in milliseconds.
pub(crate) fn ms(seconds: f64) -> f64 {
    seconds * 1e3
}

/// What of a pass of `pass_ms` is neither the filter's nor the rows': never below zero, as separate clocks can make it.
pub(crate) fn rest_of(pass_ms: f64, filter_ms: f64, rows_ms: f64) -> f64 {
    (pass_ms - filter_ms - rows_ms).max(0.0)
}

/// A primary click at `pos`: its move, press and release, one frame each.
fn click_frames(pos: Pos2) -> [Vec<Event>; 3] {
    let button = |pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    [
        vec![Event::PointerMoved(pos)],
        vec![button(true)],
        vec![button(false)],
    ]
}

/// The frame drawn through wgpu, offscreen, on the mock's device.
struct Gpu {
    canvas: Arc<MockCanvas>,
    renderer: egui_wgpu::Renderer,
    view: wgpu::TextureView,
}

impl Gpu {
    fn new(canvas: Arc<MockCanvas>) -> Self {
        let device = canvas.context().device.clone();
        let renderer = egui_wgpu::Renderer::new(&device, FORMAT, Default::default());
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("console bench"),
            size: wgpu::Extent3d {
                width: SIZE[0],
                height: SIZE[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        Self {
            canvas,
            renderer,
            view,
        }
    }

    fn draw(&mut self, textures: &egui::TexturesDelta, primitives: &[egui::ClippedPrimitive]) {
        let context = self.canvas.context();
        let (device, queue) = (&context.device, &context.queue);
        for (id, deltas) in &textures.set {
            for delta in deltas {
                self.renderer.update_texture(device, queue, *id, delta);
            }
        }
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: SIZE,
            pixels_per_point: PIXELS_PER_POINT,
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut commands =
            self.renderer
                .update_buffers(device, queue, &mut encoder, primitives, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer
                .render(&mut pass.forget_lifetime(), primitives, &screen);
        }
        commands.push(encoder.finish());
        queue.submit(commands);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll failed");
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
    }
}

/// The GUI under measurement.
pub(crate) struct Run {
    pub(crate) app: App<MockSide>,
    pub(crate) headless: Headless,
    gpu: Option<Gpu>,
}

impl Run {
    /// A new GUI with `log` entries in its console: the mock's own lines, one per `SetField`, the log the console shows
    /// after a session's use.
    pub(crate) fn new(log: usize, canvas: Option<Arc<MockCanvas>>) -> Self {
        let side = MockSide::new(MockEngine::new(), canvas.clone());
        let mut app = App::new(side, FORMAT);
        let mut headless = Headless::new(SIZE, PIXELS_PER_POINT);
        // The window has it on only for a client that asks.
        headless.ctx.disable_accesskit();
        headless.set_frame_step(FRAME_S);
        let _ = headless.frame(&mut app, Vec::new());
        let mut run = Self {
            app,
            headless,
            gpu: canvas.map(Gpu::new),
        };
        run.fill(log);
        run.idle(SETTLE);
        if let Some(gpu) = &run.gpu {
            // The frames so far went through it, the font's atlas with them: a run that drew nothing measures nothing.
            let atlas = egui::TextureId::default();
            assert!(gpu.renderer.texture(&atlas).is_some(), "nothing was drawn");
        }
        run
    }

    /// Logs `log` lines, one per `SetField`, and reads them: a frame's time is the step before it, so the second frame
    /// is the one past the snapshot's interval.
    fn fill(&mut self, log: usize) {
        for _ in 0..log {
            let edit = crate::explore::time::playhead_edit(1.0);
            self.app.side().engine().set_field(edit);
        }
        self.headless
            .set_frame_step(crate::app::SNAPSHOT_INTERVAL_S);
        for _ in 0..2 {
            let _ = self.headless.frame(&mut self.app, Vec::new());
        }
        self.headless.set_frame_step(FRAME_S);
    }

    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            let _ = self.frame(Vec::new());
        }
    }

    /// Runs one frame on `events`, drawn, and returns its time.
    fn frame(&mut self, events: Vec<Event>) -> FrameTime {
        self.app.windows.console.timing = console::Timing::default();
        let start = Instant::now();
        let output = self.headless.frame(&mut self.app, events);
        let pass_ms = ms(start.elapsed().as_secs_f64());
        let passes = output.platform_output.num_completed_passes;
        let start = Instant::now();
        let primitives = self
            .headless
            .ctx
            .tessellate(output.shapes, PIXELS_PER_POINT);
        let tessellate_ms = ms(start.elapsed().as_secs_f64());
        let mut textures = std::mem::take(&mut self.headless.textures);
        let wgpu_ms = self.gpu.as_mut().map_or(0.0, |gpu| {
            let start = Instant::now();
            gpu.draw(&textures, &primitives);
            ms(start.elapsed().as_secs_f64())
        });
        textures.clear();
        let timing = self.app.windows.console.timing;
        let (filter_ms, rows_ms) = (ms(timing.filter_s), ms(timing.rows_s));
        FrameTime {
            layout_ms: rest_of(pass_ms, filter_ms, rows_ms),
            filter_ms,
            rows_ms,
            tessellate_ms,
            wgpu_ms,
            passes,
        }
    }

    /// A click at `pos`, then the idle frames: the frame before its input, not its move or its press, the release
    /// that acts, and the two after it.
    fn clicked(&mut self, pos: Pos2) -> [FrameTime; 3] {
        let [moved, pressed, released] = click_frames(pos);
        let _ = self.frame(moved);
        let _ = self.frame(pressed);
        [
            self.frame(released),
            self.frame(Vec::new()),
            self.frame(Vec::new()),
        ]
    }

    /// One open-and-close cycle, by the frames of [`FRAMES`].
    fn cycle(&mut self) -> [FrameTime; FRAMES.len()] {
        let screen = self.headless.screen();
        let layout = Layout::with_splits(screen, PIXELS_PER_POINT, &self.app.splits);
        let closed = self.frame(Vec::new());
        let [open, open_1, open_2] = self.clicked(footer_point(layout.footer));
        self.idle(SETTLE);
        let shown = self.frame(Vec::new());
        let close_at = self
            .app
            .keyboard
            .place_of(console::CLOSE)
            .expect("the console is drawn with its ×")
            .rect
            .center();
        let [close, close_1, close_2] = self.clicked(close_at);
        self.idle(SETTLE);
        [closed, open, open_1, open_2, shown, close, close_1, close_2]
    }
}

/// Measures a table of `cycles` cycles at each of `sizes`; the frames drawn on `canvas`, if any.
pub fn measure(sizes: &[usize], cycles: usize, canvas: Option<Arc<MockCanvas>>) -> Vec<Table> {
    sizes
        .iter()
        .map(|log| {
            let mut run = Run::new(*log, canvas.clone());
            let mut rows: Vec<Row> = FRAMES
                .iter()
                .map(|frame| Row {
                    frame,
                    times: Vec::with_capacity(cycles),
                })
                .collect();
            for _ in 0..cycles {
                for (row, time) in rows.iter_mut().zip(run.cycle()) {
                    row.times.push(time);
                }
            }
            Table { log: *log, rows }
        })
        .collect()
}

/// The benchmark `name`, on `canvas`: every table's text, or why it cannot run.
pub fn run(
    name: Option<&str>,
    canvas: impl FnOnce() -> Result<Arc<MockCanvas>, String>,
) -> Result<String, String> {
    match name {
        Some(NAME) => {}
        Some(other) => {
            return Err(format!(
                "gui: no benchmark `{other}`; the benchmark is {NAME}"
            ))
        }
        None => {
            return Err(format!(
                "gui: --bench needs a name; the benchmark is {NAME}"
            ))
        }
    }
    Ok(report(&measure(&LOG_SIZES, CYCLES, Some(canvas()?))))
}

/// The run's text: what it ran on, then each table.
pub fn report(tables: &[Table]) -> String {
    let mut text = format!(
        "{NAME}: {} x {} px at {PIXELS_PER_POINT} pixels per point, {CYCLES} cycles, in milliseconds{}\n",
        SIZE[0],
        SIZE[1],
        if cfg!(debug_assertions) {
            "; A DEBUG BUILD, run with --release"
        } else {
            ""
        }
    );
    for table in tables {
        text += "\n";
        text += &render_table(table);
    }
    text
}
