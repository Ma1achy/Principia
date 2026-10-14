//! The Profiler (render_gui_spec §G5 "Profiler"; GUI_DESIGN_NOTES § "04 Windows"; 04_windows.png; R-56, R-129, R-152):
//! the window and its tabs, Timeline, Flame, GPU, Memory, Counters and Arbiter, with live / pause / Capture, drawn from
//! the engine side's trace in profiler schema v1's shape (the mock's canned one). Timeline: the frame-time trace with
//! frame ms, p50, p95, p99 and the worst frame (nearest rank, `engine::contract::profile::percentile`, the one
//! percentile code, telemetry §5.5), where the frame goes over the five stages (a donut), and stacked per-frame bars for
//! the last 60. Flame: the last frame's nested scopes under the five stages. GPU: its passes. Memory: heap, GPU and
//! tile cache over the trace, the live allocations by type with their change, and the leak detector's flags. Counters:
//! the frame record's counters. Arbiter: empty, TASK-M6-22's to fill (R-152).
//!
//! The passive-logging switch, the compute fast-math line and the agents' commands are shown as frames (applied per
//! R-369; R-129 checks them by presence until the M8 dev GUI); the substeps-per-pixel histogram has no field in
//! schema v1's frame record, so it is not drawn here. Export trace, Open in Tracy and Headless render… write nothing
//! on the mock; TASK-M8-28 fills the rest.

use eframe::egui::{
    self, pos2, vec2, Button, Color32, Grid, Rect, RichText, Sense, Shape, Stroke, Ui,
};
use engine::contract::profile::{percentile, FrameRecord, Scope as Span, Stage, Trace};

use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::windows::{note, section, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_profiler";
/// Its title, as 04_windows.png's.
pub const TITLE: &str = "Profiler";

/// The tabs' section.
pub const TABS: ScopeId = "pf_tabs";
/// Live, pause and Capture's section.
pub const TRANSPORT: ScopeId = "pf_transport";
/// The tools' section: the passive-logging switch and the export buttons.
pub const TOOLS: ScopeId = "pf_tools";

/// The tabs, in order (§G5): Timeline, Flame, GPU, Memory, Counters, Arbiter.
pub const TAB_IDS: [(ScopeId, &str); 6] = [
    ("pf_timeline", "Timeline"),
    ("pf_flame", "Flame"),
    ("pf_gpu", "GPU"),
    ("pf_memory", "Memory"),
    ("pf_counters", "Counters"),
    ("pf_arbiter", "Arbiter"),
];
/// Live.
pub const LIVE: ScopeId = "pf_live";
/// Pause.
pub const PAUSE: ScopeId = "pf_pause";
/// Capture.
pub const CAPTURE: ScopeId = "pf_capture";
/// The passive-logging switch (telemetry §1.2; R-129), a frame.
pub const LOGGING: ScopeId = "pf_logging";
/// The tools that write a file, in order (§G5).
pub const FILE_TOOLS: [(ScopeId, &str); 3] = [
    ("pf_export_trace", "Export trace (JSON)"),
    ("pf_tracy", "Open in Tracy"),
    ("pf_headless", "Headless render…"),
];

/// The agents' commands (§G5), shown as written.
pub const AGENTS: [&str; 3] = [
    "prin profile --scenario deep_zoom_03 --frames 600 --json out/prof.json",
    "prin profile diff out/base.json out/prof.json --threshold 5%",
    "prin profile query \"top 10 scopes by p95\" --live",
];
/// The Arbiter tab's frame (§G5's list; `principia_quality_device_note.md`, R-129).
pub const ARBITER: [&str; 4] = [
    "estimated throughput",
    "current rung",
    "headroom",
    "recent decisions and why",
];
/// What a file tool says on the mock.
pub const NO_FILE: &str = "the mock writes no file";
/// The window's line with no trace.
pub const NO_TRACE: &str = "no trace";
/// How many frames the stacked bars show (§G5).
pub const BARS: usize = 60;

/// Each stage's colour, in [`Stage::ALL`]'s order (a look choice, R-390, 04_windows.png's donut).
pub const STAGE_COLOURS: [Color32; 5] = [
    Color32::from_rgb(230, 160, 50),
    Color32::from_rgb(120, 200, 130),
    Color32::from_rgb(90, 170, 240),
    Color32::from_rgb(220, 120, 150),
    Color32::from_rgb(170, 140, 220),
];

/// The Profiler's own state: the GUI's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    /// The tab, by its index in [`TAB_IDS`].
    pub tab: usize,
    /// Live (`true`) or paused.
    pub live: bool,
    /// The last tool's note, if any.
    pub note: Option<&'static str>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tab: 0,
            live: true,
            note: None,
        }
    }
}

/// Joins the window's sections and elements to `tree`, in order.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(TABS, "tabs"));
    for (id, label) in TAB_IDS {
        tree.register(Some(TABS), Scope::control(id, label));
    }
    tree.register(
        Some(WINDOW),
        Scope::group(TRANSPORT, "live · pause · Capture"),
    );
    for (id, label) in [(LIVE, "live"), (PAUSE, "pause"), (CAPTURE, "Capture")] {
        tree.register(Some(TRANSPORT), Scope::control(id, label));
    }
    tree.register(Some(WINDOW), Scope::group(TOOLS, "tools"));
    tree.register(Some(TOOLS), Scope::disabled(LOGGING, "passive logging"));
    for (id, label) in FILE_TOOLS {
        tree.register(Some(TOOLS), Scope::control(id, label));
    }
}

/// Enter on control `id`, as its click.
pub fn activate(id: &str, state: &mut State) {
    if let Some(i) = TAB_IDS.iter().position(|(t, _)| *t == id) {
        state.tab = i;
        return;
    }
    match id {
        LIVE => state.live = true,
        PAUSE => state.live = false,
        CAPTURE => state.note = Some(NO_FILE),
        id if FILE_TOOLS.iter().any(|(t, _)| *t == id) => state.note = Some(NO_FILE),
        _ => {}
    }
}

/// The frame line: "frame 16.7 ms · p50 … · p95 … · p99 … · worst … (frame n)", over `frames`' `frame_ms`.
pub fn stats_line(frames: &[FrameRecord]) -> String {
    let ms: Vec<f64> = frames.iter().map(|f| f.frame_ms).collect();
    let p = |percent| percentile(&ms, percent).map_or("—".to_owned(), |v| format!("{v:.1}"));
    let worst = frames
        .iter()
        .max_by(|a, b| a.frame_ms.total_cmp(&b.frame_ms))
        .map_or("—".to_owned(), |f| {
            format!("{:.1} (frame {})", f.frame_ms, f.frame)
        });
    let last = frames
        .last()
        .map_or("—".to_owned(), |f| format!("{:.1}", f.frame_ms));
    format!(
        "frame {last} ms · p50 {} · p95 {} · p99 {} · worst {worst}",
        p(50),
        p(95),
        p(99)
    )
}

/// A stage's ms in `frame`; 0 for an absent present stage.
pub fn stage_ms(frame: &FrameRecord, stage: Stage) -> f64 {
    let s = &frame.stage_ms;
    match stage {
        Stage::Integrate => s.integrate,
        Stage::Reduce => s.reduce,
        Stage::Colour => s.colour,
        Stage::Upload => s.upload,
        Stage::Present => s.present.unwrap_or(0.0),
    }
}

/// Allocates a plot of `height` across the width, framed as the artboard's.
fn plot(ui: &mut Ui, height: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    ui.painter()
        .rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    rect
}

/// The frame-time trace.
fn frame_trace(ui: &mut Ui, frames: &[FrameRecord]) {
    let rect = plot(ui, 48.0);
    let top = frames.iter().map(|f| f.frame_ms).fold(1.0, f64::max);
    let n = frames.len().max(2) - 1;
    let points: Vec<_> = frames
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let x = rect.min.x + rect.width() * i as f32 / n as f32;
            pos2(x, rect.max.y - rect.height() * (f.frame_ms / top) as f32)
        })
        .collect();
    let colour = ui.visuals().hyperlink_color;
    ui.painter()
        .add(Shape::line(points, Stroke::new(1.5, colour)));
}

/// Where the frame goes: a donut over the five stages of `frame`, the frame's ms inside, with its legend.
fn donut(ui: &mut Ui, frame: &FrameRecord) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
        let (centre, r) = (rect.center(), 38.0);
        let total: f64 = Stage::ALL.iter().map(|s| stage_ms(frame, *s)).sum();
        let mut from = -std::f64::consts::FRAC_PI_2;
        for (stage, colour) in Stage::ALL.iter().zip(STAGE_COLOURS) {
            let sweep =
                std::f64::consts::TAU * stage_ms(frame, *stage) / total.max(f64::MIN_POSITIVE);
            let steps = (sweep * 16.0).ceil().max(1.0) as usize;
            let arc: Vec<_> = (0..=steps)
                .map(|k| {
                    let a = from + sweep * k as f64 / steps as f64;
                    centre + vec2(a.cos() as f32, a.sin() as f32) * r
                })
                .collect();
            ui.painter()
                .add(Shape::line(arc, Stroke::new(14.0, colour)));
            from += sweep;
        }
        ui.painter().text(
            centre,
            egui::Align2::CENTER_CENTER,
            format!("{:.1}", frame.frame_ms),
            egui::FontId::monospace(12.0),
            ui.visuals().text_color(),
        );
        Grid::new("donut legend").show(ui, |ui| {
            for (stage, colour) in Stage::ALL.iter().zip(STAGE_COLOURS) {
                ui.label(RichText::new("■").color(colour));
                ui.label(RichText::new(stage.key()).monospace());
                ui.label(RichText::new(format!("{:.2}", stage_ms(frame, *stage))).monospace());
                ui.end_row();
            }
        });
    });
}

/// The last [`BARS`] frames' stages, stacked.
fn stacked(ui: &mut Ui, frames: &[FrameRecord]) {
    let rect = plot(ui, 64.0);
    let last = &frames[frames.len().saturating_sub(BARS)..];
    let top = last.iter().map(|f| f.frame_ms).fold(1.0, f64::max) as f32;
    let w = rect.width() / BARS as f32;
    for (i, f) in last.iter().enumerate() {
        let x = rect.min.x + w * i as f32;
        let mut y = rect.max.y;
        for (stage, colour) in Stage::ALL.iter().zip(STAGE_COLOURS) {
            let h = rect.height() * stage_ms(f, *stage) as f32 / top;
            let bar = Rect::from_min_max(pos2(x + 1.0, y - h), pos2(x + w - 1.0, y));
            ui.painter().rect_filled(bar, 0.0, colour);
            y -= h;
        }
    }
}

/// The last frame's nested scopes, a row per depth, the stages laid end to end.
fn flame(ui: &mut Ui, frame: &FrameRecord) {
    let rect = plot(ui, 72.0);
    let row = 18.0;
    let scale = rect.width() / frame.frame_ms.max(f64::MIN_POSITIVE) as f32;
    let mut stage_start = 0.0;
    for (stage, colour) in Stage::ALL.iter().zip(STAGE_COLOURS) {
        if let Some(sections) = frame.stages.get(*stage) {
            let bars = FlameBars {
                ui,
                rect,
                scale,
                row,
                colour,
            };
            for span in &sections.scopes {
                bars.draw(span, stage_start, 0);
            }
        }
        stage_start += stage_ms(frame, *stage);
    }
}

/// One stage's bars in the flame graph: where they are drawn, the time scale, the row height and the stage's colour.
struct FlameBars<'a> {
    ui: &'a Ui,
    rect: Rect,
    scale: f32,
    row: f32,
    colour: Color32,
}

impl FlameBars<'_> {
    /// `span`, starting `at` ms into the frame, on row `depth`, then its children a row below.
    fn draw(&self, span: &Span, at: f64, depth: usize) {
        let x0 = self.rect.min.x + (at + span.start_ms) as f32 * self.scale;
        let bar = Rect::from_min_size(
            pos2(x0, self.rect.min.y + self.row * depth as f32),
            vec2((span.ms as f32 * self.scale).max(1.0), self.row - 2.0),
        );
        self.ui.painter().rect_filled(bar, 1.0, self.colour);
        let label = format!("{} {:.1}", span.name, span.ms);
        let clipped = self.ui.painter().with_clip_rect(bar);
        clipped.text(
            bar.left_center() + vec2(3.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            Color32::BLACK,
        );
        for child in &span.children {
            self.draw(child, at + span.start_ms, depth + 1);
        }
    }
}

/// `bytes` in MB, as the footer's readout.
fn mb(bytes: u64) -> String {
    crate::explore::footer::megabytes(bytes)
}

/// Memory over the trace, the live allocations by type with their change over it, and the leak flags.
fn memory(ui: &mut Ui, trace: &Trace) {
    let (first, last) = (&trace.frames[0], &trace.frames[trace.frames.len() - 1]);
    fn pools(f: &FrameRecord) -> [&engine::contract::profile::PoolLive; 3] {
        [
            &f.live_memory.heap,
            &f.live_memory.gpu,
            &f.live_memory.tile_cache,
        ]
    }
    ui.horizontal(|ui| {
        for ((name, p), colour) in ["heap", "GPU", "tile cache"]
            .iter()
            .zip(pools(last))
            .zip(STAGE_COLOURS)
        {
            ui.label(
                RichText::new(format!("{name} {}", mb(p.bytes)))
                    .monospace()
                    .color(colour),
            );
        }
    });
    let rect = plot(ui, 48.0);
    let top = trace
        .frames
        .iter()
        .flat_map(|f| pools(f).map(|p| p.bytes))
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let n = trace.frames.len().max(2) - 1;
    for (k, colour) in STAGE_COLOURS.iter().take(3).enumerate() {
        let points: Vec<_> = trace
            .frames
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let x = rect.min.x + rect.width() * i as f32 / n as f32;
                pos2(
                    x,
                    rect.max.y - rect.height() * pools(f)[k].bytes as f32 / top,
                )
            })
            .collect();
        ui.painter()
            .add(Shape::line(points, Stroke::new(1.0, *colour)));
    }
    ui.label(RichText::new("live allocations by type, change over the trace").strong());
    Grid::new("live allocations").show(ui, |ui| {
        for (now, then) in pools(last).into_iter().zip(pools(first)) {
            for kind in &now.by_kind {
                let before = then
                    .by_kind
                    .iter()
                    .find(|k| k.kind == kind.kind)
                    .map_or(0, |k| k.count);
                ui.label(RichText::new(&kind.kind).monospace());
                ui.label(RichText::new(kind.count.to_string()).monospace());
                ui.label(
                    RichText::new(format!("{:+}", kind.count as i64 - before as i64)).monospace(),
                );
                ui.end_row();
            }
        }
    });
    let flags = trace.leak_flags().unwrap_or(&[]);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(RichText::new("leak detector").color(ui.visuals().warn_fg_color));
        for flag in flags {
            let message = flag.get("message").and_then(|m| m.as_str()).unwrap_or("—");
            ui.label(format!("· {message}"));
        }
    });
}

/// The last frame's GPU passes, per stage.
fn gpu(ui: &mut Ui, frame: &FrameRecord) {
    Grid::new("gpu passes").striped(true).show(ui, |ui| {
        for stage in Stage::ALL {
            for pass in frame.stages.get(stage).map_or(&[][..], |s| &s.gpu_passes) {
                ui.label(RichText::new(stage.key()).monospace().weak());
                ui.label(RichText::new(&pass.name).monospace());
                ui.label(RichText::new(format!("{:.2} ms", pass.ms)).monospace());
                ui.end_row();
            }
        }
    });
}

/// The last frame's counters.
fn counters(ui: &mut Ui, f: &FrameRecord) {
    Grid::new("counters").striped(true).show(ui, |ui| {
        for (name, value) in [
            ("quads_computed", f.quads_computed),
            ("quads_reused", f.quads_reused),
            ("samples", f.samples),
            ("substeps_total", f.substeps_total),
            ("tree_depth_max", u64::from(f.tree_depth_max)),
            ("leaf_count", f.leaf_count),
        ] {
            ui.label(RichText::new(name).monospace());
            ui.label(RichText::new(crate::explore::top_bar::grouped(value)).monospace());
            ui.end_row();
        }
    });
}

/// Draws the window's content from `trace`; its clicks act as Enter does.
pub fn show(ui: &mut Ui, keys: &mut Keys<'_>, state: &mut State, trace: Option<&Trace>) {
    let mut clicked: Vec<ScopeId> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        let tabs = ui
            .scope(|ui| {
                for (i, (id, label)) in TAB_IDS.iter().enumerate() {
                    let tab = ui.add(Button::selectable(state.tab == i, *label));
                    if tab.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &tab);
                }
            })
            .response;
        keys.place_rect(TABS, tabs.rect);
    });
    // The transport on its own row: beside the tabs it would be squeezed into what the row has left.
    ui.horizontal_wrapped(|ui| {
        let transport = ui
            .scope(|ui| {
                for (id, label, on) in [
                    (LIVE, "live", state.live),
                    (PAUSE, "pause", !state.live),
                    (CAPTURE, "Capture", false),
                ] {
                    let button = ui.add(Button::selectable(on, label));
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
            })
            .response;
        keys.place_rect(TRANSPORT, transport.rect);
    });
    match trace.filter(|t| !t.frames.is_empty()) {
        None => note(ui, NO_TRACE),
        Some(trace) => {
            let last = &trace.frames[trace.frames.len() - 1];
            ui.label(RichText::new(stats_line(&trace.frames)).monospace());
            match TAB_IDS[state.tab].0 {
                "pf_timeline" => {
                    frame_trace(ui, &trace.frames);
                    ui.label(RichText::new("▾ Where the frame goes").strong());
                    donut(ui, last);
                    ui.label(RichText::new(format!("▾ Per frame · stacked, last {BARS}")).strong());
                    stacked(ui, &trace.frames);
                }
                "pf_flame" => {
                    ui.label(RichText::new("▾ Flame · main thread").strong());
                    flame(ui, last);
                }
                "pf_gpu" => gpu(ui, last),
                "pf_memory" => memory(ui, trace),
                "pf_counters" => counters(ui, last),
                _ => {
                    for line in ARBITER {
                        ui.horizontal(|ui| {
                            ui.label(line);
                            note(ui, crate::explore::top_bar::ABSENT);
                        });
                    }
                }
            }
            let fast = &trace.header.fast_math;
            let compiled = fast.compiled.map_or("—".to_owned(), |c| {
                format!("{:?}", c.compute).to_lowercase()
            });
            ui.label(
                RichText::new(format!(
                    "compute fast-math: {} · compiled {compiled}",
                    format!("{:?}", fast.setting).to_lowercase()
                ))
                .monospace()
                .weak(),
            );
        }
    }
    ui.add_space(6.0);
    section(
        ui,
        keys,
        TOOLS,
        "For agents · the same data, structured",
        |ui, keys| {
            let mut logging = false;
            let switch =
                ui.add_enabled(false, egui::Checkbox::new(&mut logging, "passive logging"));
            keys.place(LOGGING, &switch);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                for line in AGENTS {
                    ui.label(RichText::new(line).monospace());
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (id, label) in FILE_TOOLS {
                    let button = ui.button(label);
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
            });
            if let Some(text) = state.note {
                note(ui, text);
            }
        },
    );
    for id in clicked {
        activate(id, state);
    }
}
