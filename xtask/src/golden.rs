//! `cargo xtask golden` — the golden-image runner, and the one-variable reproduction an image artefact is diagnosed
//! with (philosophy §4.3a; pitfalls §1.3, §8).
//!
//! `cargo xtask golden <suite>` renders each case of `fixtures/golden/<suite>/` headless with native `wgpu` offscreen
//! (parity_contract §6; R-110), compares it with the case's reference image to the tolerance its requirement gives, and
//! writes a per-pixel difference image and a summary under `target/golden/<suite>/<case>/`. `--all` runs every suite,
//! and is registered in `cargo xtask ci` (R-110: native golden suites run on every commit).
//!
//! `cargo xtask golden repro <suite>/<case> --vary <field>=<a>,<b>` renders two arms whose configurations differ in
//! exactly one field, refusing a pair differing in more (or in none), and reports per arm the RGB values along each
//! line the case names (to tell a structure from the ~3-step 8-bit staircase, pitfalls §1.2 (b)) and each symptom the
//! case names in its own column, marked moved or unchanged, so a fix is closed only against the columns it moved
//! (pitfalls §8).
//!
//! A case is `fixtures/golden/<suite>/<case>/case.json`:
//!
//! ```text
//! {
//!   "render": { "shader": "gradient.wgsl", "fragment": "fs_main", "width": 256, "height": 256,
//!               "constants": { "<override>": <number>, ... } },
//!   "reference": "reference.png",
//!   "tolerance": "REQ-VAL-138",
//!   "expect": "pass" | "fail",
//!   "lines": { "<name>": { "from": [x, y], "to": [x, y] }, ... },
//!   "symptoms": [ { "name": "<name>", "kind": "count" | "fraction", "rgb": [r, g, b] }
//!               | { "name": "<name>", "kind": "mean", "channel": "r" | "g" | "b" }, ... ]
//! }
//! ```
//!
//! The shader is a WGSL fragment module, drawn over the whole target by a full-screen triangle the runner supplies,
//! into an `Rgba8Unorm` texture; `constants` set its `override` declarations. The tolerance is a requirement or
//! calibration-requirement id from [`TOLERANCES`]; a bare number is refused. `expect: "fail"` marks a case that must
//! fail its comparison: the proof that the runner can fire (pitfalls §3). `lines` and `symptoms` are optional; a repro
//! needs at least one line. The reference is refused unless `fixtures/golden/BASELINES.md` holds its hash with the
//! decision that set it (R-110: no re-baselining without a gate decision).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// A tolerance a golden case may name: the largest per-channel difference, in 8-bit steps, between the render and
/// the reference at any pixel.
#[derive(Debug)]
pub struct Tolerance {
    /// The requirement or calibration-requirement id that gives the value.
    pub id: &'static str,
    /// The largest per-channel absolute difference, in 8-bit steps, a passing case may show.
    pub max_step: u8,
    /// Where the value stands.
    pub status: &'static str,
}

/// The tolerances a case may name. The metric is the largest per-channel absolute difference over all pixels, in
/// 8-bit steps.
pub const TOLERANCES: &[Tolerance] = &[Tolerance {
    id: "REQ-VAL-138",
    max_step: 0,
    status:
        "proposed by TASK-M0-06 with its evidence; confirmed by the human at the M0 gate (R-71)",
}];

/// The environment variable naming the backend (R-169), as `validation::gpu` reads it.
pub const BACKEND_VAR: &str = "PRIN_GPU_BACKEND";

/// The runner's vertex stage: one triangle covering the whole target.
const FULL_SCREEN_VS: &str = r"
@vertex
fn golden_vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}
";

/// An 8-bit RGB image, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

impl Image {
    /// The pixel at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let i = 3 * (y as usize * self.width as usize + x as usize);
        [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
    }

    /// Reads an 8-bit RGB or RGBA PNG; alpha is dropped.
    pub fn read_png(path: &Path) -> Result<Image, String> {
        let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        // png 0.18 gives no size for an image too large to address; the empty buffer then fails `next_frame`.
        let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let channels = match (info.color_type, info.bit_depth) {
            (png::ColorType::Rgb, png::BitDepth::Eight) => 3,
            (png::ColorType::Rgba, png::BitDepth::Eight) => 4,
            (colour, depth) => {
                return Err(format!(
                    "{}: {colour:?} at {depth:?}; a golden image is 8-bit RGB or RGBA",
                    path.display()
                ))
            }
        };
        let rgb = buf[..info.buffer_size()]
            .chunks_exact(channels)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        Ok(Image {
            width: info.width,
            height: info.height,
            rgb,
        })
    }

    /// Writes the image as an 8-bit RGB PNG.
    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        let file = fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut w| w.write_image_data(&self.rgb))
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// A render input, flattened to its fields: `shader`, `fragment`, `width`, `height` and `constants.<override>`. Two
/// configurations are compared field by field.
#[derive(Clone, Debug, PartialEq)]
pub struct Config(pub BTreeMap<String, Value>);

const RENDER_FIELDS: [&str; 4] = ["shader", "fragment", "width", "height"];

impl Config {
    /// Flattens a case's `render` object, refusing a missing or unknown field.
    pub fn from_render(render: &Value) -> Result<Config, String> {
        let object = render.as_object().ok_or("`render` is not an object")?;
        let mut fields = BTreeMap::new();
        for (key, value) in object {
            if key == "constants" {
                let constants = value
                    .as_object()
                    .ok_or("`render.constants` is not an object")?;
                for (name, value) in constants {
                    fields.insert(format!("constants.{name}"), value.clone());
                }
            } else if RENDER_FIELDS.contains(&key.as_str()) {
                fields.insert(key.clone(), value.clone());
            } else {
                return Err(format!("`render.{key}` is not a render field"));
            }
        }
        let config = Config(fields);
        for field in RENDER_FIELDS {
            if !config.0.contains_key(field) {
                return Err(format!("`render.{field}` is missing"));
            }
        }
        config.size()?;
        config.constants()?;
        Ok(config)
    }

    /// Sets `field` to `value`: a render field, or `constants.<override>`.
    pub fn set(&mut self, field: &str, value: Value) -> Result<(), String> {
        if !RENDER_FIELDS.contains(&field) && !field.starts_with("constants.") {
            return Err(format!(
                "`{field}` is not a field; the fields are {} and constants.<override>",
                RENDER_FIELDS.join(", ")
            ));
        }
        self.0.insert(field.to_owned(), value);
        Ok(())
    }

    /// The fields in which `self` and `other` differ, in order. Numbers are compared by value, so `1` and `1.0` are
    /// the same.
    pub fn differing(&self, other: &Config) -> Vec<String> {
        let mut keys: Vec<&String> = self.0.keys().chain(other.0.keys()).collect();
        keys.sort();
        keys.dedup();
        keys.into_iter()
            .filter(|k| !same_value(self.0.get(*k), other.0.get(*k)))
            .cloned()
            .collect()
    }

    fn text(&self, field: &str) -> Result<&str, String> {
        self.0
            .get(field)
            .and_then(Value::as_str)
            .ok_or(format!("`{field}` is not a string"))
    }

    /// The target's width and height, each at least 1.
    pub fn size(&self) -> Result<(u32, u32), String> {
        let dim = |field: &str| {
            self.0
                .get(field)
                .and_then(Value::as_u64)
                .filter(|&n| (1..=8192).contains(&n))
                .map(|n| n as u32)
                .ok_or(format!("`{field}` is not an integer from 1 to 8192"))
        };
        Ok((dim("width")?, dim("height")?))
    }

    /// The `override` constants, by name.
    pub fn constants(&self) -> Result<Vec<(String, f64)>, String> {
        self.0
            .iter()
            .filter_map(|(k, v)| k.strip_prefix("constants.").map(|name| (name, v)))
            .map(|(name, v)| {
                v.as_f64()
                    .map(|x| (name.to_owned(), x))
                    .ok_or(format!("`constants.{name}` is not a number"))
            })
            .collect()
    }

    fn describe(&self) -> String {
        self.0
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Whether two field values are the same: numbers by value (`1` and `1.0` are one number, whatever their spelling),
/// anything else by equality.
fn same_value(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(Value::Number(x)), Some(Value::Number(y))) => match (x.as_i64(), y.as_i64()) {
            (Some(i), Some(j)) => i == j,
            _ => match (x.as_u64(), y.as_u64()) {
                (Some(i), Some(j)) => i == j,
                _ => x.as_f64() == y.as_f64(),
            },
        },
        _ => a == b,
    }
}

/// A named straight line through the image, sampled at one pixel per step of its longer axis.
#[derive(Clone, Debug)]
pub struct Line {
    pub name: String,
    pub from: [u32; 2],
    pub to: [u32; 2],
}

impl Line {
    /// The pixels along the line, both ends included.
    pub fn points(&self) -> Vec<(u32, u32)> {
        let [x0, y0] = self.from.map(i64::from);
        let [x1, y1] = self.to.map(i64::from);
        let n = (x1 - x0).abs().max((y1 - y0).abs());
        (0..=n)
            .map(|i| {
                let at = |a: i64, b: i64| {
                    if n == 0 {
                        a
                    } else {
                        a + ((b - a) * 2 * i + n).div_euclid(2 * n)
                    }
                };
                (at(x0, x1) as u32, at(y0, y1) as u32)
            })
            .collect()
    }
}

/// How a symptom is measured on an image.
#[derive(Clone, Debug)]
pub enum SymptomKind {
    /// The number of pixels exactly this colour.
    Count([u8; 3]),
    /// The fraction of pixels exactly this colour.
    Fraction([u8; 3]),
    /// The mean of one channel (0 red, 1 green, 2 blue), in 8-bit steps.
    Mean(usize),
}

/// A named symptom metric: one column of a repro report.
#[derive(Clone, Debug)]
pub struct Symptom {
    pub name: String,
    pub kind: SymptomKind,
}

impl Symptom {
    /// The metric on `image`.
    pub fn measure(&self, image: &Image) -> f64 {
        let pixels = image.rgb.as_chunks::<3>().0.iter();
        let n = pixels.len() as f64;
        match self.kind {
            SymptomKind::Count(c) => pixels.filter(|p| **p == c).count() as f64,
            SymptomKind::Fraction(c) => pixels.filter(|p| **p == c).count() as f64 / n,
            SymptomKind::Mean(ch) => pixels.map(|p| f64::from(p[ch])).sum::<f64>() / n,
        }
    }
}

/// A loaded golden case.
#[derive(Debug)]
pub struct Case {
    /// `<suite>/<case>`.
    pub name: String,
    pub dir: PathBuf,
    pub config: Config,
    pub reference: PathBuf,
    pub tolerance: &'static Tolerance,
    /// Whether the case must pass its comparison (`false`: a can-fire case, which must fail it).
    pub expect_pass: bool,
    pub lines: Vec<Line>,
    pub symptoms: Vec<Symptom>,
}

impl Case {
    /// Loads `<dir>/case.json`, refusing a tolerance that is a bare number or no known requirement id.
    pub fn load(dir: &Path, name: &str) -> Result<Case, String> {
        let path = dir.join("case.json");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let json: Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let fail = |message: String| format!("golden case {name}: {message}");
        let config = Config::from_render(&json["render"]).map_err(fail)?;
        let reference = json["reference"]
            .as_str()
            .ok_or_else(|| fail("`reference` is not a file name".to_owned()))?;
        let tolerance = tolerance(&json["tolerance"]).map_err(fail)?;
        let expect_pass = match json.get("expect").map(|v| v.as_str()) {
            None | Some(Some("pass")) => true,
            Some(Some("fail")) => false,
            Some(_) => return Err(fail("`expect` is neither \"pass\" nor \"fail\"".to_owned())),
        };
        let lines = match json.get("lines") {
            None => Vec::new(),
            Some(lines) => parse_lines(lines, &config).map_err(fail)?,
        };
        let symptoms = match json.get("symptoms") {
            None => Vec::new(),
            Some(symptoms) => parse_symptoms(symptoms).map_err(fail)?,
        };
        Ok(Case {
            name: name.to_owned(),
            dir: dir.to_path_buf(),
            config,
            reference: dir.join(reference),
            tolerance,
            expect_pass,
            lines,
            symptoms,
        })
    }
}

/// The tolerance a case's `tolerance` names: a requirement id in [`TOLERANCES`], never a bare number.
pub fn tolerance(value: &Value) -> Result<&'static Tolerance, String> {
    let bare = |shown: &dyn std::fmt::Display| {
        format!(
            "tolerance {shown} is a bare number; give the requirement or calibration-requirement id that sets it \
             (one of {})",
            TOLERANCES.iter().map(|t| t.id).collect::<Vec<_>>().join(", ")
        )
    };
    match value {
        Value::Number(n) => Err(bare(n)),
        Value::String(s) if s.trim().parse::<f64>().is_ok() => Err(bare(&format!("{s:?}"))),
        Value::String(s) => TOLERANCES.iter().find(|t| t.id == s).ok_or(format!(
            "tolerance {s:?} is no requirement id with a golden tolerance (one of {})",
            TOLERANCES
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>()
                .join(", ")
        )),
        _ => Err(
            "`tolerance` is missing; give a requirement or calibration-requirement id".to_owned(),
        ),
    }
}

fn parse_lines(value: &Value, config: &Config) -> Result<Vec<Line>, String> {
    let (width, height) = config.size()?;
    let object = value.as_object().ok_or("`lines` is not an object")?;
    let point = |name: &str, v: &Value| -> Result<[u32; 2], String> {
        // Every element must be a non-negative integer: one that isn't refuses the line, rather than being dropped
        // and leaving a different pixel behind.
        let p: Vec<u64> = v
            .as_array()
            .and_then(|a| a.iter().map(Value::as_u64).collect::<Option<_>>())
            .unwrap_or_default();
        match p.as_slice() {
            [x, y] if *x < u64::from(width) && *y < u64::from(height) => Ok([*x as u32, *y as u32]),
            _ => Err(format!(
                "line {name:?}: {v} is not a pixel [x, y] inside {width}x{height}"
            )),
        }
    };
    object
        .iter()
        .map(|(name, line)| {
            Ok(Line {
                name: name.clone(),
                from: point(name, &line["from"])?,
                to: point(name, &line["to"])?,
            })
        })
        .collect()
}

fn parse_symptoms(value: &Value) -> Result<Vec<Symptom>, String> {
    let list = value.as_array().ok_or("`symptoms` is not a list")?;
    let mut symptoms: Vec<Symptom> = Vec::new();
    for s in list {
        let name = s["name"]
            .as_str()
            .ok_or("a symptom has no `name`")?
            .to_owned();
        let rgb = || -> Result<[u8; 3], String> {
            // Every channel must be an integer in 0..=255: one that isn't refuses the symptom, rather than being
            // dropped and leaving a different colour behind.
            let c: Vec<u8> = s["rgb"]
                .as_array()
                .and_then(|a| {
                    a.iter()
                        .map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
                        .collect::<Option<_>>()
                })
                .unwrap_or_default();
            <[u8; 3]>::try_from(c)
                .map_err(|_| format!("symptom {name:?}: `rgb` is not [r, g, b] in 0..=255"))
        };
        let kind = match s["kind"].as_str() {
            Some("count") => SymptomKind::Count(rgb()?),
            Some("fraction") => SymptomKind::Fraction(rgb()?),
            Some("mean") => SymptomKind::Mean(match s["channel"].as_str() {
                Some("r") => 0,
                Some("g") => 1,
                Some("b") => 2,
                _ => return Err(format!("symptom {name:?}: `channel` is not r, g or b")),
            }),
            _ => {
                return Err(format!(
                    "symptom {name:?}: `kind` is not count, fraction or mean"
                ))
            }
        };
        if symptoms.iter().any(|t| t.name == name) {
            return Err(format!("symptom {name:?} is named twice"));
        }
        symptoms.push(Symptom { name, kind });
    }
    Ok(symptoms)
}

/// Checks the reference's SHA-256 against its entry in `fixtures/golden/BASELINES.md`, which must name a decision
/// that `decisions.md` records (R-110). A reference with no entry, a different hash or no recorded decision is
/// refused.
pub fn check_baseline(root: &Path, case: &Case) -> Result<(), String> {
    let path = root.join("fixtures/golden/BASELINES.md");
    let baselines = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let bytes = fs::read(&case.reference).map_err(|e| {
        format!(
            "golden case {}: {}: {e}",
            case.name,
            case.reference.display()
        )
    })?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let entry = baselines.lines().find_map(|line| {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        match cells.as_slice() {
            [name, sha, decision] if name.trim_matches('`') == case.name => {
                Some((sha.trim_matches('`').to_owned(), decision.to_string()))
            }
            _ => None,
        }
    });
    let Some((sha, decision)) = entry else {
        return Err(format!(
            "golden case {}: fixtures/golden/BASELINES.md has no entry for its reference; a baseline changes only \
             with a recorded gate decision (R-110)",
            case.name
        ));
    };
    if sha != hash {
        return Err(format!(
            "golden case {}: reference {} changed without a BASELINES.md entry naming the decision (its SHA-256 is \
             {hash}; the entry records {sha}); a baseline changes only with a recorded gate decision (R-110)",
            case.name,
            case.reference.display()
        ));
    }
    let decisions =
        fs::read_to_string(root.join("decisions.md")).map_err(|e| format!("decisions.md: {e}"))?;
    let named: Vec<&str> = decision
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|w| {
            w.strip_prefix("R-")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    if named.is_empty() {
        return Err(format!(
            "golden case {}: its BASELINES.md entry names no decision (R-n)",
            case.name
        ));
    }
    for r in named {
        if !decisions
            .lines()
            .any(|l| l.starts_with(&format!("## {r} ")))
        {
            return Err(format!(
                "golden case {}: its BASELINES.md entry names {r}, which decisions.md does not record",
                case.name
            ));
        }
    }
    Ok(())
}

/// The backend a value of `PRIN_GPU_BACKEND` selects, with its name: `metal` or `vulkan`, or unset for the platform's
/// (metal on macOS, vulkan elsewhere); anything else is an error naming the variable. The same rule as
/// `validation::gpu::backend_choice` (R-169, R-206), which xtask may not depend on (systems_architecture §7.1); a test
/// keeps the two in step.
pub fn backend_from(value: Option<&str>) -> Result<(&str, wgpu::Backends), String> {
    let name = value.unwrap_or(if cfg!(target_os = "macos") {
        "metal"
    } else {
        "vulkan"
    });
    match name {
        "metal" => Ok((name, wgpu::Backends::METAL)),
        "vulkan" => Ok((name, wgpu::Backends::VULKAN)),
        other => Err(format!(
            "{BACKEND_VAR}={other:?} is not a backend; set it to metal or vulkan"
        )),
    }
}

/// A headless device that renders golden cases offscreen.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The adapter, for the summaries.
    pub adapter: String,
}

impl Renderer {
    /// Opens a device on the backend `PRIN_GPU_BACKEND` names (`metal` or `vulkan`), or on the platform's (metal on
    /// macOS, vulkan elsewhere) when it is unset, as the harness does (R-169, R-206).
    pub fn new() -> Result<Renderer, String> {
        let value = std::env::var(BACKEND_VAR).ok();
        let (name, backends) = backend_from(value.as_deref())?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .map_err(|e| format!("no {name} adapter ({BACKEND_VAR}): {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("xtask::golden"),
            ..Default::default()
        }))
        .map_err(|e| format!("request_device failed: {e}"))?;
        let info = adapter.get_info();
        let adapter = format!(
            "adapter: {} | backend: {:?} | driver: {} {}",
            info.name, info.backend, info.driver, info.driver_info
        );
        Ok(Renderer {
            device,
            queue,
            adapter,
        })
    }

    /// Renders `config`, its shader path relative to `dir`, and reads the image back.
    pub fn render(&self, dir: &Path, config: &Config) -> Result<Image, String> {
        let (width, height) = config.size()?;
        let shader_path = dir.join(config.text("shader")?);
        let source = fs::read_to_string(&shader_path)
            .map_err(|e| format!("{}: {e}", shader_path.display()))?;
        let fragment = config.text("fragment")?;
        let constants = config.constants()?;
        let constants: Vec<(&str, f64)> = constants.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = |label, source: &str| {
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                })
        };
        let vs = module("golden_vs", FULL_SCREEN_VS);
        let fs_module = module(fragment, &source);
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("golden"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("golden_vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs_module,
                    entry_point: Some(fragment),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &constants,
                        ..Default::default()
                    },
                    targets: &[Some(format.into())],
                }),
                multiview_mask: None,
                cache: None,
            });
        if let Some(error) = pollster::block_on(scope.pop()) {
            return Err(format!("{}: {error}", shader_path.display()));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("golden target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // `union`, as the flags are disjoint: `|` and `^` would agree.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
            view_formats: &[],
        });
        let row = (4 * width).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("golden readback"),
            size: u64::from(row) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            // No pass label: it is for debuggers only, and nothing reads it.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            pass.set_pipeline(&pipeline);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| {
            r.expect("golden readback map failed")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("device poll failed: {e}"))?;
        let mapped = slice
            .get_mapped_range()
            .map_err(|e| format!("readback: {e:?}"))?;
        let rgb = mapped
            .chunks_exact(row as usize)
            .flat_map(|r| {
                r[..4 * width as usize]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|p| [p[0], p[1], p[2]])
            })
            .collect();
        drop(mapped);
        readback.unmap();
        Ok(Image { width, height, rgb })
    }
}

/// The per-pixel comparison of a render with its reference.
#[derive(Debug)]
pub struct Diff {
    /// The largest per-channel absolute difference, in 8-bit steps: the metric a tolerance bounds.
    pub max_step: u8,
    /// The pixels with any channel differing.
    pub differing: usize,
    pub pixels: usize,
    /// The mean per-channel absolute difference, in 8-bit steps.
    pub mean_step: f64,
    /// The first differing pixel, in row order.
    pub first: Option<(u32, u32)>,
    /// The per-channel absolute difference at each pixel.
    pub image: Image,
}

/// Compares `render` with `reference`, which must be the same size.
pub fn diff(render: &Image, reference: &Image) -> Result<Diff, String> {
    if (render.width, render.height) != (reference.width, reference.height) {
        return Err(format!(
            "render is {}x{} and reference {}x{}",
            render.width, render.height, reference.width, reference.height
        ));
    }
    let rgb: Vec<u8> = render
        .rgb
        .iter()
        .zip(&reference.rgb)
        .map(|(a, b)| a.abs_diff(*b))
        .collect();
    let pixels = rgb.len() / 3;
    let differing_at: Vec<usize> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| p.iter().any(|&d| d != 0))
        .map(|(i, _)| i)
        .collect();
    let width = render.width as usize;
    Ok(Diff {
        max_step: rgb.iter().copied().max().unwrap_or(0),
        differing: differing_at.len(),
        pixels,
        mean_step: rgb.iter().map(|&d| f64::from(d)).sum::<f64>() / rgb.len().max(1) as f64,
        first: differing_at
            .first()
            .map(|&i| ((i % width) as u32, (i / width) as u32)),
        image: Image {
            width: render.width,
            height: render.height,
            rgb,
        },
    })
}

/// One case's outcome.
#[derive(Debug)]
pub struct Outcome {
    pub case: String,
    pub diff: Diff,
    /// Whether the diff is within the case's tolerance.
    pub within: bool,
    /// Whether that is what the case expects.
    pub ok: bool,
}

/// The directory `target/golden/` output goes under: `$CARGO_TARGET_DIR/golden`, or `<root>/target/golden`.
pub fn output_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("golden")
}

/// The suites under `<root>/fixtures/golden/`, sorted.
pub fn suites(root: &Path) -> Result<Vec<String>, String> {
    subdirs(&root.join("fixtures/golden"))
}

fn subdirs(dir: &Path) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

/// Loads every case of `suite`, refusing any whose tolerance or baseline is refused, before anything renders.
pub fn load_suite(root: &Path, suite: &str) -> Result<Vec<Case>, String> {
    let dir = root.join("fixtures/golden").join(suite);
    let cases = subdirs(&dir)?;
    if cases.is_empty() {
        return Err(format!("golden suite {suite} has no case"));
    }
    cases
        .iter()
        .map(|c| {
            let case = Case::load(&dir.join(c), &format!("{suite}/{c}"))?;
            check_baseline(root, &case)?;
            Ok(case)
        })
        .collect()
}

/// Loads every case of `suite` under `root`, with [`load_suite`]'s refusals (tolerance and BASELINES.md), and prints
/// each; it opens no device and renders nothing, so it is golden's listing-only form (R-235).
pub fn list_suite(root: &Path, suite: &str) -> Result<(), String> {
    for case in load_suite(root, suite)? {
        println!(
            "xtask golden: {}: tolerance {} (max step {}), expect {}",
            case.name,
            case.tolerance.id,
            case.tolerance.max_step,
            if case.expect_pass { "pass" } else { "fail" }
        );
    }
    Ok(())
}

/// Runs `suite` under `root`, writing each case's render, difference image and summary under `out/<suite>/<case>/`.
/// `Err` names each case that did not do what it expects.
pub fn run_suite(root: &Path, suite: &str, out: &Path) -> Result<Vec<Outcome>, String> {
    let cases = load_suite(root, suite)?;
    let renderer = Renderer::new()?;
    println!("xtask golden: {suite}: {}", renderer.adapter);
    let mut outcomes = Vec::new();
    for case in &cases {
        let render = renderer.render(&case.dir, &case.config)?;
        let reference = Image::read_png(&case.reference)?;
        let diff =
            diff(&render, &reference).map_err(|e| format!("golden case {}: {e}", case.name))?;
        let within = diff.max_step <= case.tolerance.max_step;
        let ok = within == case.expect_pass;
        let verdict = match (case.expect_pass, within) {
            (true, true) => "pass",
            (true, false) => "FAIL",
            (false, false) => "fails as expected (the runner can fire)",
            (false, true) => "FAIL: a can-fire case passed, so the runner cannot fire",
        };
        let summary = format!(
            "case: {}\n{}\nverdict: {verdict}\nmetric: largest per-channel absolute difference, 8-bit steps\n\
             max step: {}\ntolerance: {} (max step {}; {})\ndiffering pixels: {} of {}\nmean step: {:.6}\n\
             first differing pixel: {}\n",
            case.name,
            renderer.adapter,
            diff.max_step,
            case.tolerance.id,
            case.tolerance.max_step,
            case.tolerance.status,
            diff.differing,
            diff.pixels,
            diff.mean_step,
            diff.first
                .map_or("none".to_owned(), |(x, y)| format!("({x}, {y})")),
        );
        let dir = out.join(&case.name);
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        render.write_png(&dir.join("render.png"))?;
        diff.image.write_png(&dir.join("diff.png"))?;
        fs::write(dir.join("summary.txt"), &summary)
            .map_err(|e| format!("{}: {e}", dir.display()))?;
        println!(
            "xtask golden: {}: {verdict} (max step {}, {} of {} pixels differ; tolerance {}: max step {}) -> {}",
            case.name,
            diff.max_step,
            diff.differing,
            diff.pixels,
            case.tolerance.id,
            case.tolerance.max_step,
            dir.display()
        );
        outcomes.push(Outcome {
            case: case.name.clone(),
            diff,
            within,
            ok,
        });
    }
    let failed: Vec<&str> = outcomes
        .iter()
        .filter(|o| !o.ok)
        .map(|o| o.case.as_str())
        .collect();
    if failed.is_empty() {
        Ok(outcomes)
    } else {
        Err(format!("golden case(s) failed: {}", failed.join(", ")))
    }
}

/// The two arms of a controlled reproduction.
#[derive(Debug)]
pub struct ArmResult {
    pub config: Config,
    pub image: Image,
    /// Per line of the case, in order: the RGB values along it.
    pub profiles: Vec<Vec<[u8; 3]>>,
    /// Per symptom of the case, in order: its value.
    pub symptoms: Vec<f64>,
}

/// A controlled reproduction: two arms differing in one field.
#[derive(Debug)]
pub struct Repro {
    pub case: String,
    /// The one field the arms differ in.
    pub field: String,
    pub lines: Vec<Line>,
    pub symptoms: Vec<Symptom>,
    pub arms: [ArmResult; 2],
}

/// A symptom's column: its value in each arm, and whether the change moved it.
#[derive(Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub values: [f64; 2],
    /// `false` when the two values are identical: the unchanged column, the evidence of a second defect (pitfalls §8).
    pub moved: bool,
}

/// The one field `a` and `b` differ in; a pair differing in more, or in none, is refused (philosophy §4.3a).
pub fn single_differing_field(a: &Config, b: &Config) -> Result<String, String> {
    let fields = a.differing(b);
    match fields.as_slice() {
        [field] => Ok(field.clone()),
        _ => Err(format!(
            "repro refused: the arms differ in {} field(s) ({}); a controlled reproduction changes exactly one \
             (philosophy §4.3a)",
            fields.len(),
            fields.join(", ")
        )),
    }
}

/// Checks the two arms `configs` of `case` before anything is rendered: they differ in exactly one field, the case
/// names a line, and every line lies inside each arm's image (a `width` or `height` arm may shrink the target past
/// it). Returns the differing field.
pub fn check_arms(case: &Case, configs: &[Config; 2]) -> Result<String, String> {
    let field = single_differing_field(&configs[0], &configs[1])?;
    if case.lines.is_empty() {
        return Err(format!(
            "repro refused: golden case {} names no line to profile",
            case.name
        ));
    }
    for (config, tag) in configs.iter().zip(["a", "b"]) {
        let (width, height) = config
            .size()
            .map_err(|e| format!("repro refused: arm {tag}: {e}"))?;
        for line in &case.lines {
            let inside = |[x, y]: [u32; 2]| x < width && y < height;
            if !(inside(line.from) && inside(line.to)) {
                return Err(format!(
                    "repro refused: line {:?} ({:?} to {:?}) falls outside arm {tag}'s {width}x{height} image",
                    line.name, line.from, line.to
                ));
            }
        }
    }
    Ok(field)
}

/// Renders the two arms `configs` of `case`, refusing them as [`check_arms`] does, and measures each.
pub fn repro(case: &Case, configs: [Config; 2], renderer: &Renderer) -> Result<Repro, String> {
    let field = check_arms(case, &configs)?;
    let [a, b] = configs;
    let arm = |config: Config| -> Result<ArmResult, String> {
        let image = renderer.render(&case.dir, &config)?;
        let profiles = case
            .lines
            .iter()
            .map(|l| {
                l.points()
                    .into_iter()
                    .map(|(x, y)| image.pixel(x, y))
                    .collect()
            })
            .collect();
        let symptoms = case.symptoms.iter().map(|s| s.measure(&image)).collect();
        Ok(ArmResult {
            config,
            image,
            profiles,
            symptoms,
        })
    };
    Ok(Repro {
        case: case.name.clone(),
        field,
        lines: case.lines.clone(),
        symptoms: case.symptoms.clone(),
        arms: [arm(a)?, arm(b)?],
    })
}

impl Repro {
    /// One column per symptom, each with its value per arm.
    pub fn columns(&self) -> Vec<Column> {
        self.symptoms
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let values = [self.arms[0].symptoms[i], self.arms[1].symptoms[i]];
                Column {
                    name: s.name.clone(),
                    values,
                    moved: values[0] != values[1],
                }
            })
            .collect()
    }

    /// The report: the arms' configurations and the differing field; per line and arm the RGB values along it and
    /// the largest step in R+G+B between neighbours; the symptom table, one column per symptom.
    pub fn report(&self) -> String {
        let mut r = String::new();
        let value = |i: usize| {
            self.arms[i]
                .config
                .0
                .get(&self.field)
                .map_or("(absent)".to_owned(), Value::to_string)
        };
        let _ = writeln!(r, "repro: {}", self.case);
        let _ = writeln!(
            r,
            "differing field: {} (a: {}, b: {})",
            self.field,
            value(0),
            value(1)
        );
        for (i, arm) in self.arms.iter().enumerate() {
            let _ = writeln!(r, "arm {}: {}", ["a", "b"][i], arm.config.describe());
        }
        for (l, line) in self.lines.iter().enumerate() {
            let _ = writeln!(
                r,
                "\nline {}: ({}, {}) -> ({}, {})",
                line.name, line.from[0], line.from[1], line.to[0], line.to[1]
            );
            for (i, arm) in self.arms.iter().enumerate() {
                let profile = &arm.profiles[l];
                let sums: Vec<i32> = profile
                    .iter()
                    .map(|p| p.iter().map(|&c| i32::from(c)).sum())
                    .collect();
                let step = sums
                    .windows(2)
                    .map(|w| (w[1] - w[0]).abs())
                    .max()
                    .unwrap_or(0);
                let values: Vec<String> = profile
                    .iter()
                    .map(|p| format!("({},{},{})", p[0], p[1], p[2]))
                    .collect();
                let _ = writeln!(
                    r,
                    "arm {} line {} (largest R+G+B step between neighbours: {step}): {}",
                    ["a", "b"][i],
                    line.name,
                    values.join(" ")
                );
            }
        }
        let columns = self.columns();
        let width = |c: &Column| c.name.len().max(12);
        let _ = write!(r, "\n{:<8}", "arm");
        for c in &columns {
            let _ = write!(r, " {:>w$}", c.name, w = width(c));
        }
        for (i, arm) in ["a", "b"].iter().enumerate() {
            let _ = write!(r, "\n{arm:<8}");
            for c in &columns {
                let _ = write!(r, " {:>w$}", format!("{}", c.values[i]), w = width(c));
            }
        }
        let _ = write!(r, "\n{:<8}", "change");
        for c in &columns {
            let _ = write!(
                r,
                " {:>w$}",
                if c.moved { "moved" } else { "unchanged" },
                w = width(c)
            );
        }
        r.push('\n');
        r
    }
}

/// `cargo xtask golden <args>` on the workspace at `root`.
pub fn cli(root: &Path, args: &[&str]) -> Result<(), String> {
    let out = output_dir(root);
    match args {
        ["--all"] => {
            let suites = suites(root)?;
            let failed: Vec<&String> = suites
                .iter()
                .filter(|s| {
                    run_suite(root, s, &out)
                        .map_err(|e| eprintln!("xtask golden: {s}: {e}"))
                        .is_err()
                })
                .collect();
            if failed.is_empty() {
                println!("xtask golden: all {} suite(s) passed", suites.len());
                Ok(())
            } else {
                Err(format!(
                    "golden suite(s) failed: {}",
                    failed
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
        }
        ["--list"] => {
            let suites = suites(root)?;
            let failed: Vec<&String> = suites
                .iter()
                .filter(|s| {
                    list_suite(root, s)
                        .map_err(|e| eprintln!("xtask golden: {s}: {e}"))
                        .is_err()
                })
                .collect();
            if failed.is_empty() {
                println!(
                    "xtask golden: {} suite(s) listed, none rendered",
                    suites.len()
                );
                Ok(())
            } else {
                Err(format!(
                    "golden suite(s) refused: {}",
                    failed
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
        }
        ["repro", case, rest @ ..] => {
            let (suite, name) = case
                .split_once('/')
                .ok_or(format!("repro: {case} is not <suite>/<case>"))?;
            let case = Case::load(&root.join("fixtures/golden").join(suite).join(name), case)?;
            let mut arms = [case.config.clone(), case.config.clone()];
            let mut rest = rest;
            while let ["--vary", spec, tail @ ..] = rest {
                let (field, values) = spec
                    .split_once('=')
                    .ok_or(format!("repro: --vary {spec} is not <field>=<a>,<b>"))?;
                let (a, b) = values
                    .split_once(',')
                    .ok_or(format!("repro: --vary {spec} is not <field>=<a>,<b>"))?;
                let parse =
                    |s: &str| serde_json::from_str(s).unwrap_or(Value::String(s.to_owned()));
                arms[0].set(field, parse(a))?;
                arms[1].set(field, parse(b))?;
                rest = tail;
            }
            if !rest.is_empty() {
                return Err(format!("repro: unrecognised arguments: {}", rest.join(" ")));
            }
            check_arms(&case, &arms)?;
            let repro = repro(&case, arms, &Renderer::new()?)?;
            let report = repro.report();
            let dir = out.join("repro").join(&case.name);
            fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            for (arm, tag) in repro.arms.iter().zip(["a", "b"]) {
                arm.image.write_png(&dir.join(format!("arm_{tag}.png")))?;
            }
            fs::write(dir.join("report.txt"), &report)
                .map_err(|e| format!("{}: {e}", dir.display()))?;
            println!("{report}\nxtask golden: repro written to {}", dir.display());
            Ok(())
        }
        [suite] if !suite.starts_with('-') => run_suite(root, suite, &out).map(|_| ()),
        _ => Err(format!(
            "golden: unrecognised arguments: {}",
            args.join(" ")
        )),
    }
}
