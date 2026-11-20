//! The wasm entry of the v1 web skeleton (TASK-M8-86; R-627), for the skeleton's loader (`web/skeleton/`). The mock app
//! runs in one of two layouts: on the main thread, in eframe's web runner ([`start_main`]), or in a Web Worker on the
//! OffscreenCanvas the loader transfers ([`WorkerApp`]). eframe's web runner needs a `document`, so in the worker egui
//! and egui-wgpu are driven directly, in the order egui-wgpu's own painter does it. A worker has no DOM events: the
//! loader forwards them, and each method of [`WorkerApp`] is one forward. It is a skeleton on the mock engine: v1 web
//! itself places the GUI by R-620, and TASK-M8-37 takes this build, not this worker layout.

use std::sync::Arc;

use eframe::egui::{self, pos2, vec2, CursorIcon, Event, Modifiers, Rect, ViewportId};
use eframe::egui_wgpu::{self, WgpuSetup, WgpuSetupExisting};
use eframe::{WebOptions, WebRunner};
use engine::contract::canvas::Canvas;
use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, OffscreenCanvas};

use crate::app::{window_title, App};
use crate::clock::time::Instant;
use crate::measure::{Environment, Measured, Session};
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::side::MockSide;

/// The title of the page: the "mock engine" tag is never out of sight (RQ-248).
#[wasm_bindgen]
pub fn title() -> String {
    window_title(true).to_owned()
}

/// A failure, as the loader reads it.
fn fail(message: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&message.to_string())
}

/// Panics reach the console, and, thrown on as a trap, the loader.
fn hook_panics() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
    }));
}

/// The session `environment` asks for: the loader's JSON, or none without `?measure`.
fn session(environment: Option<String>) -> Result<Option<Session>, JsValue> {
    environment
        .map(|text| Environment::browser(&text).map(Session::new).map_err(fail))
        .transpose()
}

/// The sink of a session's reports: the loader's function, called with the JSON.
fn sink(report: Function) -> Box<dyn FnMut(String)> {
    Box::new(move |json| {
        if let Err(e) = report.call1(&JsValue::NULL, &JsValue::from_str(&json)) {
            web_sys::console::error_1(&e);
        }
    })
}

/// Starts the app on the main thread on `canvas`, in eframe's web runner.
#[wasm_bindgen]
pub async fn start_main(
    canvas: HtmlCanvasElement,
    environment: Option<String>,
    report: Function,
) -> Result<(), JsValue> {
    hook_panics();
    let session = session(environment)?;
    let mock = Arc::new(MockCanvas::open_browser().await.map_err(fail)?);
    let context = mock.context().clone();
    let mut options = WebOptions::default();
    options.wgpu_options.wgpu_setup = WgpuSetup::Existing(WgpuSetupExisting {
        instance: context.instance,
        adapter: context.adapter,
        device: context.device,
        queue: context.queue,
    });
    let runner = WebRunner::new();
    runner
        .start(
            canvas,
            options,
            Box::new(move |cc| {
                crate::theme::install(&cc.egui_ctx);
                let format = cc
                    .wgpu_render_state
                    .as_ref()
                    .ok_or("eframe opened no wgpu renderer")?
                    .target_format;
                let side = MockSide::new(MockEngine::new(), Some(mock));
                let app = App::new(side, format);
                Ok(Box::new(Measured::new(app, session, sink(report))))
            }),
        )
        .await?;
    // The page lives as long as the runner.
    std::mem::forget(runner);
    Ok(())
}

/// The app in a worker, on its OffscreenCanvas.
#[wasm_bindgen]
pub struct WorkerApp {
    measured: Measured<MockSide>,
    ctx: egui::Context,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: egui_wgpu::Renderer,
    clear: wgpu::Color,
    pixels_per_point: f32,
    is_mac: bool,
    events: Vec<Event>,
    modifiers: Modifiers,
    origin: Instant,
    /// When egui asked for its next frame, on `origin`, in seconds; `None` until it asks.
    repaint_at: Option<f64>,
    /// Tells the loader what only the main thread can do: `copy`, with the text; `cursor`, with the CSS name.
    output: Function,
}

/// The CSS cursor of `icon`, for the icons the app uses.
fn css_cursor(icon: CursorIcon) -> &'static str {
    match icon {
        CursorIcon::PointingHand => "pointer",
        CursorIcon::Text => "text",
        CursorIcon::Grab => "grab",
        CursorIcon::Grabbing => "grabbing",
        CursorIcon::Crosshair => "crosshair",
        CursorIcon::Move | CursorIcon::AllScroll => "move",
        CursorIcon::ResizeHorizontal | CursorIcon::ResizeColumn => "col-resize",
        CursorIcon::ResizeVertical | CursorIcon::ResizeRow => "row-resize",
        CursorIcon::ResizeNeSw => "nesw-resize",
        CursorIcon::ResizeNwSe => "nwse-resize",
        CursorIcon::NotAllowed => "not-allowed",
        _ => "default",
    }
}

#[wasm_bindgen]
impl WorkerApp {
    /// Starts the app on `canvas`, `width` × `height` physical pixels at `pixels_per_point`. Rejects with the reason if
    /// the worker layout cannot run: the loader then falls back.
    #[allow(clippy::too_many_arguments)] // Reason: a wasm-bindgen entry takes the loader's values one by one.
    pub async fn start(
        canvas: OffscreenCanvas,
        environment: Option<String>,
        width: u32,
        height: u32,
        pixels_per_point: f32,
        is_mac: bool,
        report: Function,
        output: Function,
    ) -> Result<WorkerApp, JsValue> {
        hook_panics();
        let session = session(environment)?;
        let mock = Arc::new(MockCanvas::open_browser().await.map_err(fail)?);
        let context = mock.context().clone();
        let surface = context
            .instance
            .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas))
            .map_err(fail)?;
        let capabilities = surface.get_capabilities(&context.adapter);
        let format =
            egui_wgpu::preferred_framebuffer_format(&capabilities.formats).map_err(fail)?;
        let config = wgpu::SurfaceConfiguration {
            format,
            view_formats: vec![format],
            ..surface
                .get_default_config(&context.adapter, width.max(1), height.max(1))
                .ok_or("the OffscreenCanvas's surface isn't supported by this adapter")?
        };
        surface.configure(&context.device, &config);
        let renderer = egui_wgpu::Renderer::new(
            &context.device,
            format,
            egui_wgpu::RendererOptions::default(),
        );
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        if is_mac {
            ctx.set_os(egui::os::OperatingSystem::Mac);
        }
        let side = MockSide::new(MockEngine::new(), Some(mock.clone()));
        let app = App::new(side, format);
        let [r, g, b, a] =
            crate::app::clear_colour(&ctx.global_style().visuals).to_normalized_gamma_f32();
        Ok(WorkerApp {
            measured: Measured::new(app, session, sink(report)),
            ctx,
            device: context.device,
            queue: context.queue,
            surface,
            config,
            renderer,
            clear: wgpu::Color {
                r: f64::from(r),
                g: f64::from(g),
                b: f64::from(b),
                a: f64::from(a),
            },
            pixels_per_point,
            is_mac,
            events: Vec::new(),
            modifiers: Modifiers::default(),
            origin: Instant::now(),
            repaint_at: Some(0.0),
            output,
        })
    }

    /// Forward: the element resized.
    pub fn resize(&mut self, width: u32, height: u32, pixels_per_point: f32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.pixels_per_point = pixels_per_point;
        self.surface.configure(&self.device, &self.config);
        self.repaint_at = Some(0.0);
    }

    /// The modifiers of an event, as egui reads them on this platform.
    fn mods(&mut self, shift: bool, ctrl: bool, alt: bool, meta: bool) -> Modifiers {
        let modifiers = Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd: self.is_mac && meta,
            command: if self.is_mac { meta } else { ctrl },
        };
        if modifiers != self.modifiers {
            self.modifiers = modifiers;
            self.events.push(Event::ModifiersChanged(modifiers));
        }
        modifiers
    }

    /// Queues `event`, which asks for a frame.
    fn push(&mut self, event: Event) {
        self.events.push(event);
        self.repaint_at = Some(0.0);
    }

    /// Forward: the pointer moved, in points.
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        self.push(Event::PointerMoved(pos2(x, y)));
    }

    /// Forward: the pointer left.
    pub fn pointer_gone(&mut self) {
        self.push(Event::PointerGone);
    }

    /// Forward: a button pressed or released, `button` as `MouseEvent.button`.
    #[allow(clippy::too_many_arguments)] // Reason: a wasm-bindgen entry takes the loader's values one by one.
    pub fn pointer_button(
        &mut self,
        x: f32,
        y: f32,
        button: u8,
        pressed: bool,
        shift: bool,
        ctrl: bool,
        alt: bool,
        meta: bool,
    ) {
        let modifiers = self.mods(shift, ctrl, alt, meta);
        let button = match button {
            1 => egui::PointerButton::Middle,
            2 => egui::PointerButton::Secondary,
            _ => egui::PointerButton::Primary,
        };
        self.push(Event::PointerButton {
            pos: pos2(x, y),
            button,
            pressed,
            modifiers,
        });
    }

    /// Forward: the wheel, `mode` as `WheelEvent.deltaMode`. The sign is egui's: up and left are positive.
    #[allow(clippy::too_many_arguments)] // Reason: a wasm-bindgen entry takes the loader's values one by one.
    pub fn wheel(
        &mut self,
        dx: f32,
        dy: f32,
        mode: u32,
        shift: bool,
        ctrl: bool,
        alt: bool,
        meta: bool,
    ) {
        let modifiers = self.mods(shift, ctrl, alt, meta);
        let unit = match mode {
            1 => egui::MouseWheelUnit::Line,
            2 => egui::MouseWheelUnit::Page,
            _ => egui::MouseWheelUnit::Point,
        };
        self.push(Event::MouseWheel {
            unit,
            delta: vec2(-dx, -dy),
            phase: egui::TouchPhase::Move,
            modifiers,
        });
    }

    /// Forward: a key, `key` and `code` as `KeyboardEvent`'s. A key egui has no name for is dropped.
    #[allow(clippy::too_many_arguments)] // Reason: a wasm-bindgen entry takes the loader's values one by one.
    pub fn key(
        &mut self,
        key: &str,
        code: &str,
        pressed: bool,
        repeat: bool,
        shift: bool,
        ctrl: bool,
        alt: bool,
        meta: bool,
    ) {
        let modifiers = self.mods(shift, ctrl, alt, meta);
        let physical_key = egui::Key::from_name(code);
        if let Some(key) = egui::Key::from_name(key).or(physical_key) {
            self.push(Event::Key {
                key,
                physical_key,
                pressed,
                repeat,
                modifiers,
            });
        }
    }

    /// Forward: typed text.
    pub fn text(&mut self, text: String) {
        self.push(Event::Text(text));
    }

    /// Forward: the page's `paste`, with its text.
    pub fn paste(&mut self, text: String) {
        self.push(Event::Paste(text));
    }

    /// Forward: the page's `copy`.
    pub fn copy(&mut self) {
        self.push(Event::Copy);
    }

    /// Forward: the page's `cut`.
    pub fn cut(&mut self) {
        self.push(Event::Cut);
    }

    /// Forward: the page gained or lost the focus.
    pub fn focus(&mut self, focused: bool) {
        self.push(Event::WindowFocused(focused));
    }

    /// One animation frame: runs a frame if one is due.
    pub fn frame(&mut self) -> Result<(), JsValue> {
        let seconds = self.origin.elapsed().as_secs_f64();
        if self.repaint_at.is_none_or(|at| seconds < at) {
            return Ok(());
        }
        let ppp = self.pixels_per_point;
        let size = vec2(self.config.width as f32, self.config.height as f32) / ppp;
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), size)),
            time: Some(seconds),
            events: std::mem::take(&mut self.events),
            focused: true,
            max_texture_side: Some(self.device.limits().max_texture_dimension_2d as usize),
            ..Default::default()
        };
        raw.viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(ppp);
        self.measured.raw_input_hook(&self.ctx, &mut raw);
        let measured = &mut self.measured;
        let out = self.ctx.run_ui(raw, |ui| measured.ui(ui));
        for command in &out.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.tell("copy", text);
            }
        }
        self.tell("cursor", css_cursor(out.platform_output.cursor_icon));
        self.repaint_at = out
            .viewport_output
            .get(&ViewportId::ROOT)
            .map(|v| seconds + v.repaint_delay.as_secs_f64())
            .filter(|at| at.is_finite());
        let primitives = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        self.paint(&primitives, out.textures_delta, out.pixels_per_point)
    }
}

impl WorkerApp {
    /// Tells the loader `what`, with `value`.
    fn tell(&self, what: &str, value: &str) {
        let (what, value) = (JsValue::from_str(what), JsValue::from_str(value));
        if let Err(e) = self.output.call2(&JsValue::NULL, &what, &value) {
            web_sys::console::error_1(&e);
        }
    }

    /// Paints `primitives`, in the order egui-wgpu's own painter does: the textures and buffers, the surface's texture,
    /// the pass, the submit, the present.
    fn paint(
        &mut self,
        primitives: &[egui::ClippedPrimitive],
        textures: egui::TexturesDelta,
        pixels_per_point: f32,
    ) -> Result<(), JsValue> {
        let descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point,
        };
        for (id, deltas) in &textures.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let commands = self.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            primitives,
            &descriptor,
        );
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                // A frame lost, as egui-wgpu's own painter loses it: reconfigured for the next.
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_render"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.renderer
            .render(&mut pass.forget_lifetime(), primitives, &descriptor);
        self.queue
            .submit(commands.into_iter().chain([encoder.finish()]));
        self.queue.present(frame);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
        Ok(())
    }
}
