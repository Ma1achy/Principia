//! The headless capture mode (R-274; RQ-253): `gui capture --screen <screen> --steps <steps> --out <dir>` runs the app
//! on the mock, its clock frozen (RQ-246), plays the steps, renders the named screen offscreen at 01_main.png's 2160 ×
//! 1350 pixels, and writes `capture.png` and `names.json`, each accessible name with its rect in pixels, for R-275.
//! The screenshot runner spawns it as a separate process, as `gate` spawns validation's binary: no crate depends on
//! `gui` (systems_architecture §7.1). Always compiled, as the mock is (RQ-252).

use std::fs;
use std::path::Path;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton};
use eframe::egui_wgpu;
use engine::contract::canvas::Canvas;

use crate::headless::{Headless, Name};

/// The file the capture is written to, in the output directory.
pub const CAPTURE: &str = "capture.png";
/// The file the names are written to, in the output directory.
pub const NAMES: &str = "names.json";
/// The screens the capture mode renders.
pub const SCREENS: &[&str] = &["01_main"];
/// The capture's size: 01_main.png's, in pixels.
pub const SIZE: [u32; 2] = [2160, 1350];
/// The capture's pixels per point, set to match the artboard's text (the gui reviewer's, recorded in the PR).
pub const PIXELS_PER_POINT: f32 = 1.5;
/// The capture's target format: gamma-space, as the window's surface is (egui-wgpu prefers a non-sRGB one).
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// One step, from the closed list (RQ-253).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Press F3.
    F3,
    /// Make the mock raise a warning.
    RaiseWarning,
    /// Make the mock raise an error.
    RaiseError,
    /// Click the footer.
    ClickFooter,
    /// Press Tab.
    Tab,
    /// Press Shift+Tab.
    ShiftTab,
    /// Press Enter.
    Enter,
    /// Press Esc.
    Escape,
    /// Press ↑.
    ArrowUp,
    /// Press ↓.
    ArrowDown,
    /// Press ←.
    ArrowLeft,
    /// Press →.
    ArrowRight,
    /// Press `?` (Shift+/).
    Shortcuts,
    /// Press Shift+↑.
    ShiftArrowUp,
    /// Press Shift+→.
    ShiftArrowRight,
    /// Press K with the pointer over the figure at [`LOCK_AT`]: lock there (render_gui_spec §G4).
    Lock,
}

/// Where the `lock` step locks, `(s, t)` of the figure, `t` Y-up: off the centre, so the recentring shows.
pub const LOCK_AT: (f32, f32) = (0.35, 0.6);

/// The steps by name: the closed list (RQ-253), with the keyboard's keys (TASK-M6-25) and the Manifold view's
/// (TASK-M6-26).
pub const STEPS: [(&str, Step); 16] = [
    ("f3", Step::F3),
    ("raise_warning", Step::RaiseWarning),
    ("raise_error", Step::RaiseError),
    ("click_footer", Step::ClickFooter),
    ("tab", Step::Tab),
    ("shift_tab", Step::ShiftTab),
    ("enter", Step::Enter),
    ("escape", Step::Escape),
    ("arrow_up", Step::ArrowUp),
    ("arrow_down", Step::ArrowDown),
    ("arrow_left", Step::ArrowLeft),
    ("arrow_right", Step::ArrowRight),
    ("shortcuts", Step::Shortcuts),
    ("shift_arrow_up", Step::ShiftArrowUp),
    ("shift_arrow_right", Step::ShiftArrowRight),
    ("lock", Step::Lock),
];

impl Step {
    /// The step named `name`.
    pub fn parse(name: &str) -> Result<Self, String> {
        STEPS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, step)| *step)
            .ok_or_else(|| {
                let names: Vec<&str> = STEPS.iter().map(|(n, _)| *n).collect();
                format!("no step `{name}`: the steps are {}", names.join(", "))
            })
    }

    /// The steps of a comma-separated list; an empty list has none.
    pub fn parse_list(list: &str) -> Result<Vec<Self>, String> {
        list.split(',')
            .filter(|s| !s.is_empty())
            .map(Step::parse)
            .collect()
    }
}

/// A key's press and release with `modifiers`, as two frames' events.
pub fn key(key: Key, modifiers: Modifiers) -> [Vec<Event>; 2] {
    let event = |pressed| Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    };
    [vec![event(true)], vec![event(false)]]
}

/// A primary click at `pos`, as three frames' events: move, press, release.
pub fn click(pos: egui::Pos2) -> [Vec<Event>; 3] {
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

/// The point the `lock` step locks at, on `figure`: [`LOCK_AT`].
pub fn lock_point(figure: egui::Rect) -> egui::Pos2 {
    egui::pos2(
        figure.min.x + figure.width() * LOCK_AT.0,
        figure.max.y - figure.height() * LOCK_AT.1,
    )
}

/// The point a footer click lands on: the footer's left end, on its counts.
pub fn footer_point(footer: egui::Rect) -> egui::Pos2 {
    egui::pos2(footer.min.x + 40.0, footer.center().y)
}

/// A rendered screen: its pixels, RGBA8 row-major, and its names.
pub struct Shot {
    /// The size, in pixels.
    pub size: [u32; 2],
    /// The pixels.
    pub rgba: Vec<u8>,
    /// The accessible names with their rects.
    pub names: Vec<Name>,
}

/// Runs the app on the mock, its clock frozen, plays `steps` and renders the screen through `canvas`'s device.
pub fn shoot(canvas: std::sync::Arc<crate::mock::canvas::MockCanvas>, steps: &[Step]) -> Shot {
    use crate::app::App;
    use crate::layout::Layout;
    use crate::side::{EngineSide, MockSide};

    let side = MockSide::new(crate::mock::MockEngine::frozen(), Some(canvas.clone()));
    let mut app = App::new(side, FORMAT);
    let mut headless = Headless::new(SIZE, PIXELS_PER_POINT);
    let layout = Layout::new(headless.screen(), PIXELS_PER_POINT);
    let mut frames: Vec<Vec<Event>> = vec![Vec::new(), Vec::new()];
    for step in steps {
        for events in frames.drain(..) {
            let _ = headless.frame(&mut app, events);
        }
        match step {
            Step::F3 => frames.extend(key(Key::F3, Modifiers::NONE)),
            Step::Tab => frames.extend(key(Key::Tab, Modifiers::NONE)),
            Step::ShiftTab => frames.extend(key(Key::Tab, Modifiers::SHIFT)),
            Step::Enter => frames.extend(key(Key::Enter, Modifiers::NONE)),
            Step::Escape => frames.extend(key(Key::Escape, Modifiers::NONE)),
            Step::ArrowUp => frames.extend(key(Key::ArrowUp, Modifiers::NONE)),
            Step::ArrowDown => frames.extend(key(Key::ArrowDown, Modifiers::NONE)),
            Step::ArrowLeft => frames.extend(key(Key::ArrowLeft, Modifiers::NONE)),
            Step::ArrowRight => frames.extend(key(Key::ArrowRight, Modifiers::NONE)),
            Step::Shortcuts => frames.extend(key(Key::Questionmark, Modifiers::SHIFT)),
            Step::ShiftArrowUp => frames.extend(key(Key::ArrowUp, Modifiers::SHIFT)),
            Step::ShiftArrowRight => frames.extend(key(Key::ArrowRight, Modifiers::SHIFT)),
            Step::Lock => {
                frames.push(vec![Event::PointerMoved(lock_point(layout.figure))]);
                frames.extend(key(Key::K, Modifiers::NONE));
            }
            Step::RaiseWarning => app.side().engine().raise_warning(),
            Step::RaiseError => app.side().engine().raise_error(),
            Step::ClickFooter => frames.extend(click(footer_point(layout.footer))),
        }
        frames.extend([Vec::new(), Vec::new()]);
    }
    for events in frames.drain(..) {
        let _ = headless.frame(&mut app, events);
    }
    render_frame(&canvas, &mut app, &mut headless)
}

/// Runs one more frame of `app` on `headless` (at [`SIZE`] and [`PIXELS_PER_POINT`]) with no input and renders it
/// through `canvas`'s device: the capture of the app as it stands.
pub fn render_frame(
    canvas: &crate::mock::canvas::MockCanvas,
    app: &mut crate::app::App<crate::side::MockSide>,
    headless: &mut Headless,
) -> Shot {
    let output = headless.frame(app, Vec::new());
    let names = headless.names(&output);
    let primitives = headless.ctx.tessellate(output.shapes, PIXELS_PER_POINT);
    let clear = crate::app::clear_colour(&headless.ctx.global_style().visuals);
    let rgba = render(canvas, &headless.textures, &primitives, clear);
    Shot {
        size: SIZE,
        rgba,
        names,
    }
}

/// Renders `primitives` offscreen on `canvas`'s device at [`SIZE`] and reads the pixels back, the target first
/// cleared to `clear`.
fn render(
    canvas: &dyn Canvas,
    textures: &egui::TexturesDelta,
    primitives: &[egui::ClippedPrimitive],
    clear: egui::Color32,
) -> Vec<u8> {
    let context = canvas.context();
    let (device, queue) = (&context.device, &context.queue);
    let [w, h] = SIZE;
    let mut renderer = egui_wgpu::Renderer::new(device, FORMAT, Default::default());
    for (id, deltas) in &textures.set {
        for delta in deltas {
            renderer.update_texture(device, queue, *id, delta);
        }
    }
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: SIZE,
        pixels_per_point: PIXELS_PER_POINT,
    };
    let extent = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("capture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("capture readback"),
        size: u64::from(row) * u64::from(h),
        usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut commands = renderer.update_buffers(device, queue, &mut encoder, primitives, &screen);
    let [r, g, b, a] = clear.to_normalized_gamma_f32().map(f64::from);
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        renderer.render(&mut pass.forget_lifetime(), primitives, &screen);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(h),
            },
        },
        extent,
    );
    commands.push(encoder.finish());
    queue.submit(commands);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| {
        r.expect("capture readback map failed")
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    let mapped = slice.get_mapped_range().expect("capture readback range");
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        let start = y * row as usize;
        rgba.extend_from_slice(&mapped[start..start + (w * 4) as usize]);
    }
    drop(mapped);
    readback.unmap();
    rgba
}

/// Writes `shot` into `out`: [`CAPTURE`] and [`NAMES`].
pub fn write(shot: &Shot, out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let path = out.join(CAPTURE);
    let file =
        fs::File::create(&path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let [w, h] = shot.size;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&shot.rgba))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let names: Vec<serde_json::Value> = shot
        .names
        .iter()
        .map(|n| serde_json::json!({ "name": n.name, "rect": n.rect }))
        .collect();
    let text = serde_json::json!({ "size": shot.size, "names": names }).to_string();
    let path = out.join(NAMES);
    fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// The capture command's arguments, after `capture`: `--screen <screen> --steps <steps> --out <dir>`.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// The screen.
    pub screen: String,
    /// The steps.
    pub steps: Vec<Step>,
    /// The output directory.
    pub out: std::path::PathBuf,
}

impl Args {
    /// Parses `args`; `--steps` may be empty or left out, and each flag is given once.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let (mut screen, mut steps, mut out) = (None, None, None);
        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let slot = match flag.as_str() {
                "--screen" => &mut screen,
                "--steps" => &mut steps,
                "--out" => &mut out,
                other => return Err(format!("capture: unknown argument `{other}`")),
            };
            let value = it
                .next()
                .ok_or_else(|| format!("capture: {flag} needs a value"))?;
            if slot.replace(value.clone()).is_some() {
                return Err(format!("capture: {flag} given twice"));
            }
        }
        let screen = screen.ok_or("capture: --screen is required")?;
        if !SCREENS.contains(&screen.as_str()) {
            return Err(format!(
                "capture: no screen `{screen}`; the screens are {}",
                SCREENS.join(", ")
            ));
        }
        Ok(Self {
            screen,
            steps: Step::parse_list(steps.as_deref().unwrap_or(""))?,
            out: out.ok_or("capture: --out is required")?.into(),
        })
    }
}
