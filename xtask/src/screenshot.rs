//! `cargo xtask screenshot <suite>` and `cargo xtask screenshot --all`: the runner the `GUI screenshot` verify
//! method needs (REQ-TOOL-134; RQ-93, R-113; split out of TASK-M0-06 by R-183).
//!
//! A suite is a directory `fixtures/screenshot/<suite>/` holding `cases.json`. Each case names a surface (an egui
//! surface description, relative to the suite directory) and is one of two kinds:
//!
//! - a **layout** case names an artboard (a PNG, relative to the repo root, e.g. `docs/gui/design/NN_*.png`). The
//!   runner renders the surface headless, with native wgpu offscreen (R-110), and writes the capture beside a copy of
//!   the artboard, in `target/screenshot/<suite>/<case>/`. The pair is for the GUI reviewer's layout comparison only:
//!   artboard values are illustrative and corpus values win (R-68), so the runner compares no pixels.
//! - a **presence-only** case lists the controls or items the surface must contain (R-129: the surfaces with no
//!   artboard, checked by presence, not layout). The runner lays the surface out and fails naming each one absent from
//!   its AccessKit tree, the accessible names egui gives what it actually laid out. A control counts only if its
//!   node's rect also intersects the visible surface (R-275): one in the tree but clipped out of view is named as
//!   clipped. A case that needs a control below the fold scrolls to it first. It needs no GPU.
//!
//! A case's `surface` names its kind (R-274; RQ-253): a path, as above, is a `data` surface; an object
//! `{ "kind": "gui", "screen": "01_main", "steps": [...] }` is a screen of the dev GUI itself, whose steps are a closed
//! list (`f3`, `raise_warning`, `raise_error`, `click_footer`, and the keyboard's `tab`, `shift_tab`, `enter`,
//! `escape`, `arrow_up`, `arrow_down`, `arrow_left`, `arrow_right` and `shortcuts`, and the Manifold view's
//! `shift_arrow_up`, `shift_arrow_right` and `lock`). For a `gui` surface the runner
//! spawns gui's headless capture mode,
//! `cargo run --quiet -p gui --features mock -- capture --screen … --steps … --out <case dir>`, as `gate`
//! spawns validation's binary (no crate depends on `gui`, systems_architecture §7.1), which runs the app on the mock
//! engine, and reads back its `capture.png` and `names.json`, each accessible name with its rect in pixels. A layout
//! case keeps the capture beside its artboard; a presence-only case checks the names as for a `data` surface (R-275).
//!
//! Not in the per-commit `cargo xtask ci`: `.github/workflows/screenshot.yml` runs `--all` on GUI PRs (R-177), and the
//! gate workflow at the gates (R-110). The backend is `PRIN_GPU_BACKEND`'s, or the platform's when unset (R-169, R-206).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::deps::cargo;

/// Where the suites live, relative to the repo root.
pub const SUITES: &str = "fixtures/screenshot";
/// Where the captures go, relative to the repo root.
pub const OUT: &str = "target/screenshot";
/// The file each capture is written to, in its case's output directory.
pub const CAPTURE: &str = "capture.png";
/// The environment variable naming the backend (R-169).
pub const BACKEND_VAR: &str = "PRIN_GPU_BACKEND";

/// Which suites to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which<'a> {
    /// The suite so named.
    One(&'a str),
    /// Every suite under [`SUITES`].
    All,
}

/// A surface: a panel of the given size, in points (rendered at one pixel per point), holding its controls in order.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    pub size: [u32; 2],
    pub controls: Vec<Control>,
}

/// One control of a [`Surface`]; its label is its accessible name.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Control {
    Button { label: String },
    Checkbox { label: String },
}

/// A case's surface (R-274; RQ-253): a path to a `data` surface description, relative to the suite directory, or a
/// `gui` screen.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum SurfaceRef {
    /// A `data` surface: the path of its description.
    Data(String),
    /// A screen of the dev GUI, captured by gui's headless capture mode.
    Gui(GuiSurface),
}

impl fmt::Display for SurfaceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SurfaceRef::Data(path) => f.write_str(path),
            SurfaceRef::Gui(gui) => write!(f, "gui screen {}", gui.screen),
        }
    }
}

/// A `gui` surface: `{ "kind": "gui", "screen": …, "steps": [...] }`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuiSurface {
    /// Always `gui`.
    pub kind: GuiKind,
    /// The screen, by its artboard's stem: `01_main`.
    pub screen: String,
    /// The steps played before the capture, in order.
    #[serde(default)]
    pub steps: Vec<GuiStep>,
}

/// The one kind of an object surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuiKind {
    /// A screen of the dev GUI.
    Gui,
}

/// A step of a `gui` surface: the closed list (RQ-253).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiStep {
    /// Press F3.
    F3,
    /// Make the mock engine raise a warning.
    RaiseWarning,
    /// Make the mock engine raise an error.
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
    /// Press `?`, the keyboard's shortcuts.
    Shortcuts,
    /// Press Shift+↑.
    ShiftArrowUp,
    /// Press Shift+→.
    ShiftArrowRight,
    /// Press K over a point of the figure: lock there.
    Lock,
}

impl GuiStep {
    /// The step's name, as the capture mode takes it.
    pub fn name(self) -> &'static str {
        match self {
            GuiStep::F3 => "f3",
            GuiStep::RaiseWarning => "raise_warning",
            GuiStep::RaiseError => "raise_error",
            GuiStep::ClickFooter => "click_footer",
            GuiStep::Tab => "tab",
            GuiStep::ShiftTab => "shift_tab",
            GuiStep::Enter => "enter",
            GuiStep::Escape => "escape",
            GuiStep::ArrowUp => "arrow_up",
            GuiStep::ArrowDown => "arrow_down",
            GuiStep::ArrowLeft => "arrow_left",
            GuiStep::ArrowRight => "arrow_right",
            GuiStep::Shortcuts => "shortcuts",
            GuiStep::ShiftArrowUp => "shift_arrow_up",
            GuiStep::ShiftArrowRight => "shift_arrow_right",
            GuiStep::Lock => "lock",
        }
    }
}

/// The file gui's capture mode writes the names to, beside [`CAPTURE`].
pub const NAMES: &str = "names.json";

/// One case of `cases.json`: exactly one of `artboard` (a layout case) and `controls` (a presence-only case).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: String,
    pub surface: SurfaceRef,
    #[serde(default)]
    pub artboard: Option<String>,
    #[serde(default)]
    pub controls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    cases: Vec<Case>,
}

/// What a case that passed produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A layout case: the capture and the artboard copy beside it.
    Captured { capture: PathBuf, artboard: PathBuf },
    /// A presence-only case: every listed control was found.
    Present { controls: Vec<String> },
}

/// One case's result; `Err` names what failed.
#[derive(Debug, Clone)]
pub struct CaseResult {
    pub suite: String,
    pub case: String,
    pub result: Result<Outcome, String>,
}

impl fmt::Display for CaseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (suite, case) = (&self.suite, &self.case);
        match &self.result {
            Ok(Outcome::Captured { capture, artboard }) => write!(
                f,
                "{suite}/{case}: layout: wrote {} beside {} (for layout comparison only, R-68)",
                capture.display(),
                artboard.display()
            ),
            Ok(Outcome::Present { controls }) => write!(
                f,
                "{suite}/{case}: presence: all {} present: {}",
                controls.len(),
                controls.join(", ")
            ),
            Err(message) => write!(f, "{suite}/{case}: FAILED: {message}"),
        }
    }
}

/// Runs `which` under `root` (the repo root), printing each case's result; `Err` when a case fails.
pub fn run(root: &Path, which: Which<'_>) -> Result<(), String> {
    let root = &root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let suites = match which {
        Which::One(name) => vec![name.to_owned()],
        Which::All => list_suites(root)?,
    };
    if suites.is_empty() {
        // PIT-3: `--all` over no suite runs nothing, and must not pass.
        return Err(format!(
            "screenshot --all: no suite under {} (each is a directory holding cases.json)",
            root.join(SUITES).display()
        ));
    }
    let mut results = Vec::new();
    for suite in &suites {
        results.extend(run_suite(root, suite)?);
    }
    let failed = results.iter().filter(|r| r.result.is_err()).count();
    for result in &results {
        if result.result.is_err() {
            eprintln!("xtask screenshot: {result}");
        } else {
            println!("xtask screenshot: {result}");
        }
    }
    if failed == 0 {
        Ok(())
    } else {
        Err(format!(
            "screenshot: {failed} of {} case(s) failed",
            results.len()
        ))
    }
}

/// The suites under `root`'s [`SUITES`], sorted: each directory holding a `cases.json`.
pub fn list_suites(root: &Path) -> Result<Vec<String>, String> {
    let dir = root.join(SUITES);
    let entries = fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut suites: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().join("cases.json").is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    suites.sort();
    Ok(suites)
}

/// Runs every case of `suite` under `root`. `Err` when the suite cannot be read; a case's own failure is in its
/// [`CaseResult`].
pub fn run_suite(root: &Path, suite: &str) -> Result<Vec<CaseResult>, String> {
    let dir = root.join(SUITES).join(suite);
    let path = dir.join("cases.json");
    let text = fs::read_to_string(&path).map_err(|e| {
        format!(
            "screenshot: no suite `{suite}`: cannot read {}: {e}",
            path.display()
        )
    })?;
    let cases: Cases =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut gpu = None;
    Ok(cases
        .cases
        .iter()
        .map(|case| CaseResult {
            suite: suite.to_owned(),
            case: case.name.clone(),
            result: run_case(root, &dir, suite, case, &mut gpu),
        })
        .collect())
}

fn run_case(
    root: &Path,
    dir: &Path,
    suite: &str,
    case: &Case,
    gpu: &mut Option<Gpu>,
) -> Result<Outcome, String> {
    let out = root.join(OUT).join(suite).join(&case.name);
    let shot = match &case.surface {
        SurfaceRef::Data(path) => {
            let surface_path = dir.join(path);
            let text = fs::read_to_string(&surface_path)
                .map_err(|e| format!("cannot read surface {}: {e}", surface_path.display()))?;
            let surface: Surface = serde_json::from_str(&text)
                .map_err(|e| format!("surface {}: {e}", surface_path.display()))?;
            Shot::Data(lay_out(&surface))
        }
        SurfaceRef::Gui(gui) => Shot::Gui(capture_gui(gui, &out)?),
    };
    let (names, clipped, size) = match &shot {
        Shot::Data(frame) => (frame.names.clone(), frame.clipped.clone(), frame.size),
        Shot::Gui(gui) => (gui.names.clone(), gui.clipped.clone(), gui.size),
    };
    match (&case.artboard, &case.controls) {
        (Some(artboard), None) => {
            let artboard = root.join(artboard);
            if !artboard.is_file() {
                return Err(format!("the artboard {} does not exist", artboard.display()));
            }
            fs::create_dir_all(&out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
            let capture = out.join(CAPTURE);
            if let Shot::Data(frame) = &shot {
                if gpu.is_none() {
                    *gpu = Some(Gpu::new()?);
                }
                let rgba = gpu.as_mut().expect("opened above").render(frame);
                write_png(&capture, frame.size, &rgba)?;
            }
            let name = artboard.file_name().expect("a file has a name");
            let beside = out.join(name);
            fs::copy(&artboard, &beside)
                .map_err(|e| format!("cannot copy {} to {}: {e}", artboard.display(), beside.display()))?;
            Ok(Outcome::Captured {
                capture,
                artboard: beside,
            })
        }
        (None, Some(controls)) if controls.is_empty() => Err(
            "presence-only case names no control, so it asserts nothing and can never fail (R-129, PIT-3)"
                .to_owned(),
        ),
        (None, Some(controls)) => {
            // R-275: a control counts only if it is in the tree and its rect intersects the visible surface.
            let (mut absent, mut clipped_out) = (Vec::new(), Vec::new());
            for control in controls {
                if names.contains(control) {
                    continue;
                }
                if clipped.contains(control) {
                    clipped_out.push(format!("`{control}`"));
                } else {
                    absent.push(format!("`{control}`"));
                }
            }
            if absent.is_empty() && clipped_out.is_empty() {
                return Ok(Outcome::Present {
                    controls: controls.clone(),
                });
            }
            let [w, h] = size;
            let mut parts = Vec::new();
            if !absent.is_empty() {
                parts.push(format!(
                    "control(s) absent from surface {}: {} (R-129)",
                    case.surface,
                    absent.join(", ")
                ));
            }
            if !clipped_out.is_empty() {
                parts.push(format!(
                    "control(s) clipped out of surface {}, in the tree but outside its visible {w}×{h}: {} (R-275)",
                    case.surface,
                    clipped_out.join(", ")
                ));
            }
            Err(format!("presence-only case: {}", parts.join("; ")))
        }
        _ => Err(
            "a case names exactly one of `artboard` (a layout case) and `controls` (a presence-only case)"
                .to_owned(),
        ),
    }
}

/// What a case's surface produced: a `data` surface laid out here, or a `gui` screen captured by gui.
enum Shot {
    Data(Frame),
    Gui(GuiCapture),
}

/// A `gui` screen as gui's capture mode wrote it: its size, and its names split by visibility (R-275).
#[derive(Debug, Clone, PartialEq)]
pub struct GuiCapture {
    pub size: [u32; 2],
    /// The names whose rect intersects the capture.
    pub names: Vec<String>,
    /// The names with no rect, or a rect outside the capture.
    pub clipped: Vec<String>,
}

/// The command that captures `gui` into `out`: gui's headless capture mode on the mock, through the cargo running
/// xtask, on xtask's own workspace.
pub fn capture_command(gui: &GuiSurface, out: &Path) -> Command {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let steps: Vec<&str> = gui.steps.iter().map(|s| s.name()).collect();
    let mut command = Command::new(cargo());
    command
        .args(["run", "--quiet", "--manifest-path"])
        .arg(manifest)
        .args([
            "-p",
            "gui",
            "--features",
            "mock",
            "--",
            "capture",
            "--screen",
        ])
        .arg(&gui.screen)
        .arg("--steps")
        .arg(steps.join(","))
        .arg("--out")
        .arg(out);
    command
}

/// Runs gui's capture mode for `gui` into `out` and reads back its capture and names.
fn capture_gui(gui: &GuiSurface, out: &Path) -> Result<GuiCapture, String> {
    let status = capture_command(gui, out)
        .status()
        .map_err(|e| format!("cannot run gui's capture mode: {e}"))?;
    if !status.success() {
        return Err(format!("gui's capture mode failed ({status})"));
    }
    let (size, _) = read_png(&out.join(CAPTURE))?;
    let path = out.join(NAMES);
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    read_names(&text, size).map_err(|e| format!("{}: {e}", path.display()))
}

/// The names file's form: the capture's size, and each name with its rect in pixels, `null` for none.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NamesFile {
    size: [u32; 2],
    names: Vec<NamedRect>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedRect {
    name: String,
    rect: Option<[f64; 4]>,
}

/// Reads a names file for a capture of `size`, splitting the names by whether their rect meets it (R-275).
pub fn read_names(text: &str, size: [u32; 2]) -> Result<GuiCapture, String> {
    let file: NamesFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if file.size != size {
        return Err(format!(
            "the names are for a {:?} capture, the capture is {size:?}",
            file.size
        ));
    }
    let surface =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
    let (mut names, mut clipped) = (Vec::new(), Vec::new());
    for named in file.names {
        let visible = named.rect.is_some_and(|[x0, y0, x1, y1]| {
            visible_in(egui::accesskit::Rect { x0, y0, x1, y1 }, surface)
        });
        if visible { &mut names } else { &mut clipped }.push(named.name);
    }
    Ok(GuiCapture {
        size,
        names,
        clipped,
    })
}

/// A laid-out surface: its size in pixels, its triangles and textures, and the accessible names in its AccessKit tree,
/// split by whether their node's rect intersects the visible surface (R-275).
pub struct Frame {
    pub size: [u32; 2],
    primitives: Vec<egui::ClippedPrimitive>,
    textures: egui::TexturesDelta,
    /// The names whose node's rect intersects the visible surface: the controls a presence check counts.
    pub names: Vec<String>,
    /// The names in the tree whose node has no rect, or a rect outside the visible surface: clipped out of view.
    pub clipped: Vec<String>,
}

impl Drop for Frame {
    /// epaint panics (a `debug_assert!` in `TexturesDelta`'s own `Drop`) when a `TexturesDelta` still holding deltas
    /// is dropped. `Gpu::render` uploads a layout frame's deltas without clearing them, and a presence-only case never
    /// uploads its deltas, so every frame clears its deltas here before they are dropped.
    fn drop(&mut self) {
        self.textures.clear();
    }
}

/// Lays `surface` out in egui, on a CPU pass alone. Two passes, as egui may size text on the first.
pub fn lay_out(surface: &Surface) -> Frame {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let [w, h] = surface.size;
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w as f32, h as f32));
    let mut textures = egui::TexturesDelta::default();
    let mut last = None;
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(rect),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                for control in &surface.controls {
                    match control {
                        Control::Button { label } => {
                            let _ = ui.button(label.as_str());
                        }
                        Control::Checkbox { label } => {
                            let mut on = false;
                            ui.checkbox(&mut on, label.as_str());
                        }
                    }
                }
            });
        });
        textures.append(std::mem::take(&mut output.textures_delta));
        last = Some(output);
    }
    let output = last.expect("two passes ran");
    let (mut names, mut clipped) = (Vec::new(), Vec::new());
    if let Some(update) = &output.platform_output.accesskit_update {
        for (_, node) in &update.nodes {
            // egui puts a widget's text in `label`, and a plain label's in `value`. Its bounds are in points: the root
            // node alone carries the pixels-per-point scale, so they compare with the surface's rect directly.
            let visible = node.bounds().is_some_and(|b| visible_in(b, rect));
            let into = if visible { &mut names } else { &mut clipped };
            into.extend(node.label().map(str::to_owned));
            into.extend(node.value().map(str::to_owned));
        }
    }
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    Frame {
        size: surface.size,
        primitives,
        textures,
        names,
        clipped,
    }
}

/// Whether `bounds` meets `surface` in a region of positive area: a rect that only touches the surface's edge, or has
/// no area, shows nothing on it (R-275).
fn visible_in(bounds: egui::accesskit::Rect, surface: egui::Rect) -> bool {
    let x0 = bounds.x0.max(f64::from(surface.min.x));
    let y0 = bounds.y0.max(f64::from(surface.min.y));
    let x1 = bounds.x1.min(f64::from(surface.max.x));
    let y1 = bounds.y1.min(f64::from(surface.max.y));
    x0 < x1 && y0 < y1
}

/// The backend `value` (of [`BACKEND_VAR`]) selects: metal or vulkan; unset selects the platform's (R-169, R-206).
pub fn backend(value: Option<&str>) -> Result<wgpu::Backends, String> {
    let name = value.unwrap_or(if cfg!(target_os = "macos") {
        "metal"
    } else {
        "vulkan"
    });
    match name {
        "metal" => Ok(wgpu::Backends::METAL),
        "vulkan" => Ok(wgpu::Backends::VULKAN),
        other => Err(format!(
            "{BACKEND_VAR}={other:?} is not a backend; set it to metal or vulkan"
        )),
    }
}

/// A headless wgpu device (no surface) and egui's renderer on it, drawing into an offscreen texture.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: egui_wgpu::Renderer,
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// The offscreen target is drawn to, then copied out. `union`, as the flags are disjoint: `|` and `^` would agree.
const TARGET_USAGE: wgpu::TextureUsages =
    wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC);
/// The readback buffer is copied into, then mapped. `union`, as the flags are disjoint: `|` and `^` would agree.
const READBACK_USAGE: wgpu::BufferUsages =
    wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST);

impl Gpu {
    fn new() -> Result<Self, String> {
        let backends = backend(std::env::var(BACKEND_VAR).ok().as_deref())?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .map_err(|e| format!("no {backends:?} adapter ({BACKEND_VAR}): {e}"))?;
        let info = adapter.get_info();
        eprintln!(
            "xtask screenshot: adapter {} ({:?})",
            info.name, info.backend
        );
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
            .map_err(|e| format!("request_device failed: {e}"))?;
        let renderer = egui_wgpu::Renderer::new(&device, FORMAT, Default::default());
        Ok(Self {
            device,
            queue,
            renderer,
        })
    }

    /// Draws `frame` offscreen and reads it back: RGBA8, sRGB, row-major, `size[0] * size[1] * 4` bytes.
    fn render(&mut self, frame: &Frame) -> Vec<u8> {
        let [w, h] = frame.size;
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [w, h],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &frame.textures.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: TARGET_USAGE,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot readback"),
            size: u64::from(row) * u64::from(h),
            usage: READBACK_USAGE,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut commands = self.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &frame.primitives,
            &screen,
        );
        {
            // No debug label: the pass's is not observable, and the target and buffer are labelled.
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                .render(&mut pass.forget_lifetime(), &frame.primitives, &screen);
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
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        commands.push(encoder.finish());
        self.queue.submit(commands);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll failed");
        let mapped = slice.get_mapped_range().expect("readback range");
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h as usize {
            let start = y * row as usize;
            rgba.extend_from_slice(&mapped[start..start + (w * 4) as usize]);
        }
        drop(mapped);
        readback.unmap();
        for id in &frame.textures.free {
            self.renderer.free_texture(id);
        }
        rgba
    }
}

fn write_png(path: &Path, [w, h]: [u32; 2], rgba: &[u8]) -> Result<(), String> {
    let file =
        fs::File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads a PNG as 8-bit RGBA: its size and pixels. For the capture checks.
pub fn read_png(path: &Path) -> Result<([u32; 2], Vec<u8>), String> {
    let file = fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    // `union`, as the flags are disjoint: `|` and `^` would agree.
    decoder.set_transformations(
        png::Transformations::normalize_to_color8().union(png::Transformations::ALPHA),
    );
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    buf.truncate(info.buffer_size());
    Ok(([info.width, info.height], buf))
}
