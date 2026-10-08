//! The app run headless (R-274): frames on an egui context with no window, at a given size and pixels-per-point,
//! each frame's input given, and the accessible names AccessKit gives what the app laid out, with their rects in
//! pixels, for R-275's presence check. The capture mode and gui's tests drive the app through it.

use eframe::egui::{self, pos2, Event, FullOutput, Rect, TexturesDelta, ViewportId};

use crate::app::App;
use crate::side::EngineSide;
use crate::theme;

/// The time between headless frames, in seconds: 1/8, exact in binary, and longer than
/// [`SNAPSHOT_INTERVAL_S`](crate::app::SNAPSHOT_INTERVAL_S).
pub const FRAME_S: f64 = 0.125;

/// One accessible name and its node's rect, in pixels: `[x0, y0, x1, y1]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    /// The name: a node's label or value.
    pub name: String,
    /// Its node's bounds, in pixels; `None` for a node with none.
    pub rect: Option<[f64; 4]>,
}

/// A headless egui context.
pub struct Headless {
    /// The context.
    pub ctx: egui::Context,
    size: [u32; 2],
    pixels_per_point: f32,
    time: f64,
    step: f64,
    /// The texture changes of every frame so far, for a renderer that starts after them.
    pub textures: TexturesDelta,
}

impl Drop for Headless {
    /// epaint's `TexturesDelta` asserts, in debug builds, that it holds no deltas when dropped.
    fn drop(&mut self) {
        self.textures.clear();
    }
}

impl Headless {
    /// A context `size` pixels in size at `pixels_per_point`, with AccessKit on and the theme installed.
    pub fn new(size: [u32; 2], pixels_per_point: f32) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        theme::install(&ctx);
        Self {
            ctx,
            size,
            pixels_per_point,
            time: 0.0,
            step: FRAME_S,
            textures: TexturesDelta::default(),
        }
    }

    /// The pixels per point.
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    /// The window's rect, in points.
    pub fn screen(&self) -> Rect {
        let p = self.pixels_per_point;
        Rect::from_min_max(
            pos2(0.0, 0.0),
            pos2(self.size[0] as f32 / p, self.size[1] as f32 / p),
        )
    }

    /// Sets the time between frames, [`FRAME_S`] until set.
    pub fn set_frame_step(&mut self, seconds: f64) {
        self.step = seconds;
    }

    /// Runs one frame of `app` on `events`. Each frame's time is the frame step after the last: by default
    /// [`FRAME_S`], more than a snapshot interval, so each frame reads a snapshot.
    pub fn frame<S: EngineSide>(&mut self, app: &mut App<S>, events: Vec<Event>) -> FullOutput {
        let mut input = egui::RawInput {
            screen_rect: Some(self.screen()),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        input
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.pixels_per_point);
        self.time += self.step;
        let mut output = self.ctx.run_ui(input, |ui| app.ui(ui));
        self.textures
            .append(std::mem::take(&mut output.textures_delta));
        output
    }

    /// The accessible names in `output`'s AccessKit tree, with their rects in pixels.
    pub fn names(&self, output: &FullOutput) -> Vec<Name> {
        let scale = f64::from(self.pixels_per_point);
        let mut names = Vec::new();
        if let Some(update) = &output.platform_output.accesskit_update {
            for (_, node) in &update.nodes {
                let rect = node
                    .bounds()
                    .map(|b| [b.x0 * scale, b.y0 * scale, b.x1 * scale, b.y1 * scale]);
                for name in [node.label(), node.value()].into_iter().flatten() {
                    names.push(Name {
                        name: name.to_owned(),
                        rect,
                    });
                }
            }
        }
        names
    }
}
