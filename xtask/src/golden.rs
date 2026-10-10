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
//!               "constants": { "<override>": <number>, ... },
//!               "prepend": [ "crates/render/shaders/wgsl/lib/coords.wgsl", ... ] },
//!   "reference": "reference.png" | { "metal": "metal.png", "vulkan": "vulkan.png" },
//!   "output": "quantised" | "automatic",
//!   "tolerance": "REQ-VAL-138",
//!   "expect": "pass" | "fail",
//!   "lines": { "<name>": { "from": [x, y], "to": [x, y] }, ... },
//!   "symptoms": [ { "name": "<name>", "kind": "count" | "fraction", "rgb": [r, g, b] }
//!               | { "name": "<name>", "kind": "mean", "channel": "r" | "g" | "b" }, ... ]
//! }
//! ```
//!
//! The shader is a WGSL fragment module, drawn over the whole target by a full-screen triangle the runner supplies;
//! `constants` set its `override` declarations. `prepend`, optional, names WGSL files by their path relative to the
//! workspace root, the directory whose `fixtures/golden/` holds the case; the runner places their text, in order,
//! before the shader's, so a case calls the shared library's functions from their one source rather than a copy
//! (RQ-210: `m1-coords` takes the convention's flip from `lib/coords.wgsl`). A case of the harness kind instead names
//! a synthetic scene, `"render": { "harness": "<scene>", "width": <w>, "height": <h> }` (RQ-229, decided per R-369;
//! TASK-M1-09): the runner builds validation's `golden_harness` binary once per run and spawns it per case, up to
//! [`harness_width`] at once, reporting the cases in their order. xtask reaches the binary only as a separate process
//! (as it does `gate`; systems_architecture §7.1, R-187); it renders the scene through the render harness
//! (`render::bind`'s module and `upload`) into an `Rgba32Float` target and writes its floats to stdout
//! ([`HARNESS_MAGIC`]'s format); the runner uploads them as the case's float target and quantises and compares them as
//! for any case. The width and height must be the scene's. Its output is quantised in the runner's own shader (R-287):
//! the case's fragment writes an `Rgba32Float` target, and the runner's quantise pass scales each channel to 0..255,
//! rounds it half to even and stores the exact level `k / 255` in the `Rgba8Unorm` target, so no backend's
//! float-to-unorm conversion has a tie to break (parity_contract §4). `output: "automatic"` (R-287's control only)
//! draws the case's fragment straight into the `Rgba8Unorm` target, leaving the rounding to the backend; such a case
//! has one reference per backend, since its bytes differ between backends (R-269).
//!
//! A case keeps one reference across backends. The fallback, for a case whose bytes still differ between backends, a
//! golden near a tie among them (R-269, R-287, R-296), is one reference per backend, named by the backend (`metal`,
//! `vulkan`): the runner compares the render only with the reference of the backend it rendered on, names that
//! backend, and fails naming it when the case has no reference for it. The tolerance is a requirement or
//! calibration-requirement id from [`TOLERANCES`]; a bare number is refused. `expect: "fail"` marks a case that must
//! fail its comparison: the proof that the runner can fire (pitfalls §3). `lines` and `symptoms` are optional; a repro
//! needs at least one line. Each reference is refused unless `fixtures/golden/BASELINES.md` holds its hash with the
//! decision that set it (R-110: no re-baselining without a gate decision): the row is `<suite>/<case>` for a case's
//! one reference, `<suite>/<case>@<backend>` for a per-backend one.

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
///
/// REQ-VAL-138's default applies to every native backend CI renders a case on (lavapipe, Metal), each compared
/// against the case's one reference, or, for a case keeping one reference per backend, against the reference of the
/// backend it rendered on (R-269, R-287). With the output quantised in the shader (R-287), every backend rounds an
/// exact tie to even; a value within an ulp of a tie can still land on the other side of it on a backend whose display
/// shaders compile with fast-math (Metal via wgpu), so a golden near a tie keeps one reference per backend (R-296), as
/// R-269's half-way fixture (`fixtures/golden/quantise/halfway`) does.
pub const TOLERANCES: &[Tolerance] = &[Tolerance {
    id: "REQ-VAL-138",
    max_step: 0,
    status: "confirmed by the human at the M0 gate (R-376), for every native backend against one reference, per \
             backend where a case keeps one reference per backend (R-269, R-287, R-296)",
}];

/// The backends a case may keep a reference for, by the name `PRIN_GPU_BACKEND` gives them.
pub const BACKENDS: [&str; 2] = ["metal", "vulkan"];

/// A case's reference images.
#[derive(Clone, Debug)]
pub enum References {
    /// One reference across backends (R-287).
    Shared(PathBuf),
    /// One reference per backend, by backend name: the fallback for a case whose bytes differ between backends
    /// (R-269, R-287).
    PerBackend(BTreeMap<String, PathBuf>),
}

impl References {
    /// Each reference with its BASELINES.md row name: `<case>` for the shared one, `<case>@<backend>` per backend.
    pub fn rows(&self, case: &str) -> Vec<(String, &Path)> {
        match self {
            References::Shared(path) => vec![(case.to_owned(), path.as_path())],
            References::PerBackend(map) => map
                .iter()
                .map(|(backend, path)| (format!("{case}@{backend}"), path.as_path()))
                .collect(),
        }
    }
}

/// How a case's fragment output reaches the `Rgba8Unorm` target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    /// Quantised in the runner's shader, rounding half to even, before the store (R-287): the golden path.
    Quantised,
    /// Stored through the backend's automatic float-to-unorm conversion: R-287's control only.
    Automatic,
}

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

/// A harness case's render fields: the scene, and the target's size.
const HARNESS_FIELDS: [&str; 3] = ["harness", "width", "height"];

/// The float image's file the `golden_harness` binary writes: this magic, the width and height as little-endian u32,
/// then each pixel's RGBA as four little-endian f32, rows from the top (`validation::golden_scene::MAGIC`).
pub const HARNESS_MAGIC: &[u8; 8] = b"PRINF32\0";

/// The optional render field naming the WGSL files prepended to the shader, by workspace-relative path (RQ-210).
const PREPEND: &str = "prepend";

impl Config {
    /// Flattens a case's `render` object, refusing a missing or unknown field.
    pub fn from_render(render: &Value) -> Result<Config, String> {
        let object = render.as_object().ok_or("`render` is not an object")?;
        let kind = Config::kind(object.contains_key("harness"));
        let mut fields = BTreeMap::new();
        for (key, value) in object {
            if key == "constants" && kind == RENDER_FIELDS {
                let constants = value
                    .as_object()
                    .ok_or("`render.constants` is not an object")?;
                for (name, value) in constants {
                    fields.insert(format!("constants.{name}"), value.clone());
                }
            } else if kind.contains(&key.as_str()) || (key == PREPEND && kind == RENDER_FIELDS) {
                fields.insert(key.clone(), value.clone());
            } else {
                return Err(format!("`render.{key}` is not a render field"));
            }
        }
        let config = Config(fields);
        for &field in kind {
            if !config.0.contains_key(field) {
                return Err(format!("`render.{field}` is missing"));
            }
        }
        if kind == HARNESS_FIELDS && config.harness().is_none() {
            return Err("`render.harness` is not a string".to_owned());
        }
        config.size()?;
        config.constants()?;
        config.prepend()?;
        Ok(config)
    }

    /// The render fields of a case of the harness kind, or of a shader case.
    fn kind(harness: bool) -> &'static [&'static str] {
        if harness {
            &HARNESS_FIELDS
        } else {
            &RENDER_FIELDS
        }
    }

    /// The scene a case of the harness kind names, or `None` for a shader case.
    pub fn harness(&self) -> Option<&str> {
        self.0.get("harness").and_then(Value::as_str)
    }

    /// Sets `field` to `value`: a render field of the config's kind, or a shader case's `constants.<override>`.
    pub fn set(&mut self, field: &str, value: Value) -> Result<(), String> {
        let harness = self.0.contains_key("harness");
        let kind = Config::kind(harness);
        if !kind.contains(&field) && (harness || !field.starts_with("constants.")) {
            let constants = if harness {
                ""
            } else {
                " and constants.<override>"
            };
            return Err(format!(
                "`{field}` is not a field; the fields are {}{constants}",
                kind.join(", ")
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

    /// The files `prepend` names, in order, each a relative path with no `..`; none when the field is absent.
    pub fn prepend(&self) -> Result<Vec<String>, String> {
        let Some(value) = self.0.get(PREPEND) else {
            return Ok(Vec::new());
        };
        let refused = || format!("`{PREPEND}` is not a list of workspace-relative paths");
        let list = value.as_array().ok_or_else(refused)?;
        list.iter()
            .map(|v| {
                let path = v.as_str().ok_or_else(refused)?;
                let relative = Path::new(path)
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)));
                if path.is_empty() || !relative {
                    return Err(format!(
                        "`{PREPEND}` entry {path:?} is not a workspace-relative path"
                    ));
                }
                Ok(path.to_owned())
            })
            .collect()
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
    pub references: References,
    /// How the fragment output is stored (R-287).
    pub output: Output,
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
        let references = parse_references(&json["reference"], dir).map_err(fail)?;
        let output = match json.get("output").map(|v| v.as_str()) {
            None | Some(Some("quantised")) => Output::Quantised,
            Some(Some("automatic")) => Output::Automatic,
            Some(_) => {
                return Err(fail(
                    "`output` is neither \"quantised\" nor \"automatic\"".to_owned(),
                ))
            }
        };
        if output == Output::Automatic && matches!(references, References::Shared(_)) {
            return Err(fail(
                "`output: \"automatic\"` leaves the rounding to the backend, whose bytes differ between backends \
                 (R-269); such a case keeps one reference per backend"
                    .to_owned(),
            ));
        }
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
            references,
            output,
            tolerance,
            expect_pass,
            lines,
            symptoms,
        })
    }
}

impl Case {
    /// The reference the render on `backend` is compared with: the case's one reference, or its reference for
    /// `backend`. A case keeping one reference per backend with none for `backend` is an error naming the backend.
    pub fn reference_for(&self, backend: &str) -> Result<&Path, String> {
        match &self.references {
            References::Shared(path) => Ok(path),
            References::PerBackend(map) => map.get(backend).map(PathBuf::as_path).ok_or(format!(
                "golden case {}: no reference for backend {backend}, the backend it rendered on ({BACKEND_VAR}); \
                 it keeps one per backend, and has one for {}",
                self.name,
                map.keys().cloned().collect::<Vec<_>>().join(", ")
            )),
        }
    }
}

/// A case's `reference`: a file name (one reference across backends), or an object naming one file per backend.
fn parse_references(value: &Value, dir: &Path) -> Result<References, String> {
    match value {
        Value::String(name) => Ok(References::Shared(dir.join(name))),
        Value::Object(map) if !map.is_empty() => map
            .iter()
            .map(|(backend, name)| {
                if !BACKENDS.contains(&backend.as_str()) {
                    return Err(format!(
                        "`reference.{backend}` names no backend; the backends are {}",
                        BACKENDS.join(", ")
                    ));
                }
                name.as_str()
                    .map(|name| (backend.clone(), dir.join(name)))
                    .ok_or(format!("`reference.{backend}` is not a file name"))
            })
            .collect::<Result<_, _>>()
            .map(References::PerBackend),
        _ => Err(
            "`reference` is neither a file name nor an object naming one file per backend"
                .to_owned(),
        ),
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

/// Checks each reference's SHA-256 against its entry in `fixtures/golden/BASELINES.md`, which must name a decision
/// that `decisions.md` records (R-110). A reference with no entry, a different hash or no recorded decision is
/// refused. The entry is `<suite>/<case>` for a case's one reference, `<suite>/<case>@<backend>` for a per-backend one.
pub fn check_baseline(root: &Path, case: &Case) -> Result<(), String> {
    let path = root.join("fixtures/golden/BASELINES.md");
    let baselines = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decisions =
        fs::read_to_string(root.join("decisions.md")).map_err(|e| format!("decisions.md: {e}"))?;
    for (row, reference) in case.references.rows(&case.name) {
        check_row(&baselines, &decisions, &row, reference)?;
    }
    Ok(())
}

fn check_row(baselines: &str, decisions: &str, row: &str, reference: &Path) -> Result<(), String> {
    let bytes = fs::read(reference)
        .map_err(|e| format!("golden case {row}: {}: {e}", reference.display()))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let entry = baselines.lines().find_map(|line| {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        match cells.as_slice() {
            [name, sha, decision] if name.trim_matches('`') == row => {
                Some((sha.trim_matches('`').to_owned(), decision.to_string()))
            }
            _ => None,
        }
    });
    let Some((sha, decision)) = entry else {
        return Err(format!(
            "golden case {row}: fixtures/golden/BASELINES.md has no entry for its reference; a baseline changes only \
             with a recorded gate decision (R-110)"
        ));
    };
    if sha != hash {
        return Err(format!(
            "golden case {row}: reference {} changed without a BASELINES.md entry naming the decision (its SHA-256 is \
             {hash}; the entry records {sha}); a baseline changes only with a recorded gate decision (R-110)",
            reference.display()
        ));
    }
    let named: Vec<&str> = decision
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|w| {
            w.strip_prefix("R-")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    if named.is_empty() {
        return Err(format!(
            "golden case {row}: its BASELINES.md entry names no decision (R-n)"
        ));
    }
    for r in named {
        if !decisions
            .lines()
            .any(|l| l.starts_with(&format!("## {r} ")))
        {
            return Err(format!(
                "golden case {row}: its BASELINES.md entry names {r}, which decisions.md does not record"
            ));
        }
    }
    Ok(())
}

/// The backend a value of `PRIN_GPU_BACKEND` selects, with its name: `metal` or `vulkan`, or unset for the platform's
/// (metal on macOS, vulkan elsewhere); anything else is an error naming the variable. The same rule as
/// `validation::gpu::backend_choice` (R-169, R-206), which xtask may not depend on (systems_architecture §7.1); a test
/// keeps the two in step.
pub fn backend_from(value: Option<&str>) -> Result<(&'static str, wgpu::Backends), String> {
    let name = value.unwrap_or(if cfg!(target_os = "macos") {
        "metal"
    } else {
        "vulkan"
    });
    match name {
        "metal" => Ok(("metal", wgpu::Backends::METAL)),
        "vulkan" => Ok(("vulkan", wgpu::Backends::VULKAN)),
        other => Err(format!(
            "{BACKEND_VAR}={other:?} is not a backend; set it to metal or vulkan"
        )),
    }
}

/// The runner's quantise pass (R-287): reads the case's `Rgba32Float` render at each pixel, clamps each channel to
/// 0..1, scales it to 0..255, rounds it half to even, and returns the exact level `k / 255`, which the `Rgba8Unorm`
/// store converts back to `k` with no tie to break. The rounding is written out, not left to `round`, so no shader
/// translation decides the tie.
const QUANTISE_FS: &str = r"
@group(0) @binding(0) var golden_float: texture_2d<f32>;

fn golden_round_half_even(x: f32) -> f32 {
    let f = floor(x);
    let d = x - f;
    if d > 0.5 {
        return f + 1.0;
    }
    if d < 0.5 {
        return f;
    }
    // A tie: up only from an odd floor.
    return f + (f - 2.0 * floor(0.5 * f));
}

@fragment
fn golden_quantise(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let v = clamp(textureLoad(golden_float, vec2<i32>(pos.xy), 0), vec4<f32>(0.0), vec4<f32>(1.0)) * 255.0;
    let k = vec4<f32>(
        golden_round_half_even(v.x),
        golden_round_half_even(v.y),
        golden_round_half_even(v.z),
        golden_round_half_even(v.w),
    );
    return k / 255.0;
}
";

/// The workspace root of the case directory `dir`: the directory whose `fixtures/golden/` holds it, against which a
/// case's `prepend` paths resolve.
pub fn workspace_root(dir: &Path) -> Result<PathBuf, String> {
    dir.ancestors()
        .find(|a| dir.starts_with(a.join("fixtures").join("golden")))
        .map(Path::to_path_buf)
        .ok_or(format!(
            "{} is not under a workspace's fixtures/golden/, so `{PREPEND}` has no root",
            dir.display()
        ))
}

/// A headless device that renders golden cases offscreen.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The backend it renders on: `metal` or `vulkan`.
    pub backend: &'static str,
    /// The adapter, for the summaries.
    pub adapter: String,
    /// The `golden_harness` binary, built on the renderer's first harness case and spawned for each one after.
    harness: std::sync::OnceLock<PathBuf>,
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
            backend: name,
            adapter,
            harness: std::sync::OnceLock::new(),
        })
    }

    /// Renders `config`, its shader path relative to `dir`, with the output quantised in the shader (R-287), and
    /// reads the image back.
    pub fn render(&self, dir: &Path, config: &Config) -> Result<Image, String> {
        self.render_output(dir, config, Output::Quantised)
    }

    /// Renders `config`, its shader path relative to `dir`, storing the output as `output` says, and reads the image
    /// back.
    pub fn render_output(
        &self,
        dir: &Path,
        config: &Config,
        output: Output,
    ) -> Result<Image, String> {
        self.render_output_with(dir, config, output, None)
    }

    /// [`Renderer::render_output`], a harness case taking its floats from `floats` ([`Renderer::harness_all`]) where
    /// given, rather than running the harness itself.
    fn render_output_with(
        &self,
        dir: &Path,
        config: &Config,
        output: Output,
        floats: Option<Result<Vec<u8>, String>>,
    ) -> Result<Image, String> {
        let (width, height, bytes) =
            self.render_raw(dir, config, Readback::Unorm(output), floats)?;
        let rgb = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        Ok(Image { width, height, rgb })
    }

    /// Renders `config`'s fragment alone into an `Rgba32Float` target and reads back its RGBA values, rows top to
    /// bottom: the floats the quantise pass (R-287) reads, so its rounding can be checked against them.
    pub fn render_float(&self, dir: &Path, config: &Config) -> Result<Vec<[f32; 4]>, String> {
        let (_, _, bytes) = self.render_raw(dir, config, Readback::Float, None)?;
        Ok(bytes
            .as_chunks::<16>()
            .0
            .iter()
            .map(|p| {
                let (channels, _) = p.as_chunks::<4>();
                [0, 1, 2, 3].map(|c| f32::from_le_bytes(channels[c]))
            })
            .collect())
    }

    /// Renders `config` and reads the target back as `readback` says: its width, height and tightly packed pixels. A
    /// harness case takes its floats from `floats` where given, or runs the harness here.
    fn render_raw(
        &self,
        dir: &Path,
        config: &Config,
        readback: Readback,
        floats: Option<Result<Vec<u8>, String>>,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        let (width, height) = config.size()?;
        // A harness case's float target comes from the `golden_harness` binary: read back as floats, it is the
        // render; quantised, it is uploaded into the float target the quantise pass reads; it has no automatic output.
        let harness = match config.harness() {
            None => None,
            Some(scene) => {
                let floats = match floats {
                    Some(floats) => floats?,
                    None => harness_floats(self.harness_binary()?, scene, width, height)?,
                };
                match readback {
                    Readback::Float => return Ok((width, height, floats)),
                    Readback::Unorm(Output::Automatic) => {
                        return Err(format!(
                            "harness scene `{scene}`: a harness case has only the quantised output (R-287)"
                        ))
                    }
                    Readback::Unorm(Output::Quantised) => Some(floats),
                }
            }
        };
        // A shader case's fragment; a harness case has none.
        let shader = match harness {
            Some(_) => None,
            None => {
                let shader_path = dir.join(config.text("shader")?);
                let mut source = String::new();
                let prepend = config.prepend()?;
                if !prepend.is_empty() {
                    let root = workspace_root(dir)?;
                    for file in prepend {
                        let path = root.join(&file);
                        let text = fs::read_to_string(&path)
                            .map_err(|e| format!("{}: {e}", path.display()))?;
                        source.push_str(&text);
                        source.push('\n');
                    }
                }
                source.push_str(
                    &fs::read_to_string(&shader_path)
                        .map_err(|e| format!("{}: {e}", shader_path.display()))?,
                );
                Some((shader_path, source, config.text("fragment")?))
            }
        };
        let constants = config.constants()?;
        let constants: Vec<(&str, f64)> = constants.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let unorm = wgpu::TextureFormat::Rgba8Unorm;
        let float_format = wgpu::TextureFormat::Rgba32Float;
        // Quantised, the case's fragment writes floats, which the quantise pass rounds; automatic, it writes the
        // unorm target itself; read back as floats, it writes the float target that is read back.
        let case_format = match readback {
            Readback::Unorm(Output::Automatic) => unorm,
            Readback::Unorm(Output::Quantised) | Readback::Float => float_format,
        };
        let (target_format, pixel_bytes) = match readback {
            Readback::Unorm(_) => (unorm, 4),
            Readback::Float => (float_format, 16),
        };
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = |label, source: &str| {
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                })
        };
        let vs = module("golden_vs", FULL_SCREEN_VS);
        let pipeline = |layout: Option<&wgpu::PipelineLayout>,
                        fs: &wgpu::ShaderModule,
                        entry: &str,
                        constants: &[(&str, f64)],
                        format: wgpu::TextureFormat| {
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("golden"),
                    layout,
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
                        module: fs,
                        entry_point: Some(entry),
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants,
                            ..Default::default()
                        },
                        targets: &[Some(format.into())],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let case_pipeline = shader.as_ref().map(|(_, source, fragment)| {
            pipeline(
                None,
                &module(fragment, source),
                fragment,
                &constants,
                case_format,
            )
        });
        if let (Some(error), Some((shader_path, ..))) = (pollster::block_on(scope.pop()), &shader) {
            return Err(format!("{}: {error}", shader_path.display()));
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let target = |label, format, usage| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        // `union`, as the flags are disjoint: `|` and `^` would agree.
        let texture = target(
            "golden target",
            target_format,
            wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
        );
        let row = (pixel_bytes * width).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("golden readback"),
            size: u64::from(row) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let draw = |encoder: &mut wgpu::CommandEncoder,
                    view: &wgpu::TextureView,
                    pipeline: &wgpu::RenderPipeline,
                    bind_group: Option<&wgpu::BindGroup>| {
            // No pass label: it is for debuggers only, and nothing reads it.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            if let Some(bind_group) = bind_group {
                pass.set_bind_group(0, bind_group, &[]);
            }
            pass.draw(0..3, 0..1);
        };
        match readback {
            Readback::Unorm(Output::Automatic) | Readback::Float => {
                if let Some(case_pipeline) = &case_pipeline {
                    draw(&mut encoder, &view, case_pipeline, None);
                }
            }
            Readback::Unorm(Output::Quantised) => {
                let float = target(
                    "golden float target",
                    case_format,
                    wgpu::TextureUsages::RENDER_ATTACHMENT
                        .union(wgpu::TextureUsages::TEXTURE_BINDING)
                        .union(wgpu::TextureUsages::COPY_DST),
                );
                let float_view = float.create_view(&Default::default());
                let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
                // An explicit layout: `Rgba32Float` is unfilterable, and `textureLoad` needs no sampler.
                let bind_layout =
                    self.device
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some("golden quantise"),
                            entries: &[wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: false,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            }],
                        });
                let layout = self
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("golden quantise"),
                        bind_group_layouts: &[Some(&bind_layout)],
                        immediate_size: 0,
                    });
                let quantise_module = module("golden_quantise", QUANTISE_FS);
                let quantise = pipeline(
                    Some(&layout),
                    &quantise_module,
                    "golden_quantise",
                    &[],
                    unorm,
                );
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("golden quantise"),
                    layout: &bind_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&float_view),
                    }],
                });
                if let Some(error) = pollster::block_on(scope.pop()) {
                    return Err(format!("golden quantise pass: {error}"));
                }
                match (&case_pipeline, &harness) {
                    (Some(case_pipeline), _) => {
                        draw(&mut encoder, &float_view, case_pipeline, None)
                    }
                    // The queue's write lands before the encoder's commands, which the next submit carries.
                    (None, Some(floats)) => self.queue.write_texture(
                        float.as_image_copy(),
                        floats,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(16 * width),
                            rows_per_image: Some(height),
                        },
                        size,
                    ),
                    (None, None) => {}
                }
                draw(&mut encoder, &view, &quantise, Some(&bind_group));
            }
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| {
            r.expect("golden readback map failed")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("device poll failed: {e}"))?;
        let mapped = slice
            .get_mapped_range()
            .map_err(|e| format!("readback: {e:?}"))?;
        let bytes = mapped
            .chunks_exact(row as usize)
            .flat_map(|r| r[..(pixel_bytes * width) as usize].iter().copied())
            .collect();
        drop(mapped);
        buffer.unmap();
        Ok((width, height, bytes))
    }
}

impl Renderer {
    /// The floats of each harness case among `configs`, in their order, `None` for any other case. The harness is built
    /// once ([`Renderer::harness_binary`]) and its cases render [`harness_width`] at a time, each in a process of its
    /// own; a failed build is each harness case's error. Each result is used where the case renders, so a run reports
    /// its cases, and the first that fails, in the order it did when each case ran the harness in turn.
    fn harness_all(&self, configs: &[&Config]) -> Vec<Option<Result<Vec<u8>, String>>> {
        let jobs: Vec<(usize, &str, u32, u32)> = configs
            .iter()
            .enumerate()
            .filter_map(|(k, c)| {
                let (width, height) = c.size().ok()?;
                Some((k, c.harness()?, width, height))
            })
            .collect();
        let mut out: Vec<Option<Result<Vec<u8>, String>>> = configs.iter().map(|_| None).collect();
        if jobs.is_empty() {
            return out;
        }
        let rendered = match self.harness_binary() {
            Ok(binary) => in_parallel(jobs.len(), harness_width(), |j| {
                let (_, scene, width, height) = jobs[j];
                harness_floats(binary, scene, width, height)
            }),
            Err(e) => jobs.iter().map(|_| Err(e.clone())).collect(),
        };
        for ((k, ..), floats) in jobs.iter().zip(rendered) {
            out[*k] = Some(floats);
        }
        out
    }

    /// The `golden_harness` binary: built by [`build_harness`] on the first call, the same path after.
    fn harness_binary(&self) -> Result<&Path, String> {
        if let Some(path) = self.harness.get() {
            return Ok(path);
        }
        let path = build_harness()?;
        Ok(self.harness.get_or_init(|| path))
    }
}

/// Builds validation's `golden_harness` binary on this workspace, through the cargo that runs xtask, as `cargo run`
/// would (its profile and features, and `CARGO_TARGET_DIR`), and returns its path. A golden run builds it once and
/// spawns it per harness case, so it takes cargo's build lock once, not once per case.
fn build_harness() -> Result<PathBuf, String> {
    let output = std::process::Command::new(crate::deps::cargo())
        .args([
            "build",
            "--quiet",
            "--message-format=json-render-diagnostics",
            "--manifest-path",
        ])
        .arg(crate::workspace_manifest())
        .args(["-p", "validation", "--bin", "golden_harness"])
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run cargo build -p validation --bin golden_harness: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo build -p validation --bin golden_harness failed ({})",
            output.status
        ));
    }
    harness_executable(&String::from_utf8_lossy(&output.stdout))
}

/// The path of the `golden_harness` executable in `messages`, cargo's `--message-format=json` output: its one
/// `compiler-artifact` message for the `golden_harness` target with an executable.
pub fn harness_executable(messages: &str) -> Result<PathBuf, String> {
    let mut found = Vec::new();
    for line in messages.lines().filter(|l| l.starts_with('{')) {
        let message: Value =
            serde_json::from_str(line).map_err(|e| format!("cargo's message `{line}`: {e}"))?;
        if message["reason"] == "compiler-artifact" && message["target"]["name"] == "golden_harness"
        {
            if let Some(path) = message["executable"].as_str() {
                found.push(PathBuf::from(path));
            }
        }
    }
    match found.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err("cargo build named no golden_harness executable".to_owned()),
        _ => Err(format!(
            "cargo build named {} golden_harness executables",
            found.len()
        )),
    }
}

/// How many harness processes a golden run keeps running at once: the parallelism the machine offers its process
/// (`std::thread::available_parallelism`, which honours CPU affinity and cgroup quotas), at least one. Each process
/// spends most of its time compiling the scene's shaders, on one CPU (llvmpipe on CI's lavapipe runners).
pub fn harness_width() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// `job(0)` … `job(count - 1)`, run on up to `width` threads at once, each thread taking the next index in turn;
/// returned in index order, whatever order they finished in.
pub fn in_parallel<T: Send>(count: usize, width: usize, job: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<T>>> =
        (0..count).map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..width.clamp(1, count.max(1)) {
            scope.spawn(|| loop {
                let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if k >= count {
                    break;
                }
                let result = job(k);
                *results[k].lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
            });
        }
    });
    results
        .into_iter()
        .map(|r| {
            r.into_inner()
                .unwrap_or_else(|e| e.into_inner())
                .expect("in_parallel: every job ran")
        })
        .collect()
}

/// Renders the harness scene `scene` by running the `golden_harness` binary at `binary` ([`build_harness`]) and
/// returns its floats, tightly packed: refused unless the image is `width` × `height`.
fn harness_floats(binary: &Path, scene: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let output = std::process::Command::new(binary)
        .args(["--scene", scene])
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run {}: {e}", binary.display()))?;
    if !output.status.success() {
        return Err(format!(
            "golden_harness --scene {scene} failed ({})",
            output.status
        ));
    }
    decode_harness(&output.stdout, width, height)
        .map_err(|e| format!("golden_harness --scene {scene}: {e}"))
}

/// The floats of the harness's image `bytes` ([`HARNESS_MAGIC`]'s format), tightly packed: refused unless it is
/// `width` × `height` and whole.
pub fn decode_harness(bytes: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let rest = bytes
        .strip_prefix(HARNESS_MAGIC.as_slice())
        .ok_or("the image does not start with the harness's magic")?;
    let (size, floats) = rest.split_at_checked(8).ok_or("the image has no size")?;
    let (w, h) = size.split_at(4);
    let dim = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let (w, h) = (dim(w), dim(h));
    if (w, h) != (width, height) {
        return Err(format!(
            "the scene is {w} × {h}, but the case says {width} × {height}"
        ));
    }
    let want = 16 * u64::from(w) * u64::from(h);
    if floats.len() as u64 != want {
        return Err(format!(
            "the image holds {} bytes of pixels, not {want}",
            floats.len()
        ));
    }
    Ok(floats.to_vec())
}

/// What a render reads back: the `Rgba8Unorm` target, its output stored as the [`Output`] says, or the case's own
/// `Rgba32Float` render, before any quantisation.
#[derive(Clone, Copy, Debug)]
enum Readback {
    Unorm(Output),
    Float,
}

/// The per-pixel comparison of a render with its reference. It compares RGB only: alpha is dropped on reading and
/// readback, so a golden whose alpha carries meaning is outside the metric.
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
    /// The backend the render was made on, whose reference it was compared with.
    pub backend: String,
    /// The reference it was compared with.
    pub reference: PathBuf,
    pub diff: Diff,
    /// Whether the diff is within the case's tolerance.
    pub within: bool,
    /// Whether that is what the case expects.
    pub ok: bool,
}

/// Compares `render`, made on `backend`, with the case's reference for that backend (its one reference, or its
/// per-backend one, R-269, R-287), to the case's tolerance. A case with no reference for `backend` is an error naming
/// it.
pub fn judge(case: &Case, backend: &str, render: &Image) -> Result<Outcome, String> {
    let reference = case.reference_for(backend)?;
    let image = Image::read_png(reference)?;
    let diff = diff(render, &image).map_err(|e| format!("golden case {}: {e}", case.name))?;
    let within = diff.max_step <= case.tolerance.max_step;
    Ok(Outcome {
        case: case.name.clone(),
        backend: backend.to_owned(),
        reference: reference.to_path_buf(),
        diff,
        within,
        ok: within == case.expect_pass,
    })
}

/// For a case keeping one reference per backend, the comparison of its references with each other, as a line of
/// the summary: how far apart the backends' bytes are (R-269). `None` for a case with one reference.
pub fn cross_backend(case: &Case) -> Result<Option<String>, String> {
    let References::PerBackend(map) = &case.references else {
        return Ok(None);
    };
    let images = map
        .iter()
        .map(|(backend, path)| Ok((backend, Image::read_png(path)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let mut lines = Vec::new();
    for (i, (a, image_a)) in images.iter().enumerate() {
        for (b, image_b) in &images[i + 1..] {
            let d =
                diff(image_a, image_b).map_err(|e| format!("golden case {}: {e}", case.name))?;
            lines.push(format!(
                "references {a} and {b} differ by max step {} on {} of {} pixels",
                d.max_step, d.differing, d.pixels
            ));
        }
    }
    Ok(Some(lines.join("; ")))
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
        let references = match &case.references {
            References::Shared(_) => "one across backends".to_owned(),
            References::PerBackend(map) => format!(
                "one per backend ({})",
                map.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        };
        println!(
            "xtask golden: {}: tolerance {} (max step {}), expect {}, reference {references}",
            case.name,
            case.tolerance.id,
            case.tolerance.max_step,
            if case.expect_pass { "pass" } else { "fail" }
        );
    }
    Ok(())
}

/// Runs `suite` under `root`, writing each case's render, difference image and summary under `out/<suite>/<case>/`.
/// Each render is compared with the case's reference for the backend it rendered on ([`judge`]). `Err` names each case
/// that did not do what it expects.
pub fn run_suite(root: &Path, suite: &str, out: &Path) -> Result<Vec<Outcome>, String> {
    let cases = load_suite(root, suite)?;
    let renderer = Renderer::new()?;
    println!("xtask golden: {suite}: {}", renderer.adapter);
    let configs: Vec<&Config> = cases.iter().map(|c| &c.config).collect();
    let harness = renderer.harness_all(&configs);
    let mut outcomes = Vec::new();
    for (case, floats) in cases.iter().zip(harness) {
        let render = renderer.render_output_with(&case.dir, &case.config, case.output, floats)?;
        let outcome = judge(case, renderer.backend, &render)?;
        let diff = &outcome.diff;
        let verdict = match (case.expect_pass, outcome.within) {
            (true, true) => "pass",
            (true, false) => "FAIL",
            (false, false) => "fails as expected (the runner can fire)",
            (false, true) => "FAIL: a can-fire case passed, so the runner cannot fire",
        };
        let output = match case.output {
            Output::Quantised => "quantised in the shader, half to even (R-287)",
            Output::Automatic => "the backend's automatic conversion (R-287's control)",
        };
        let reference = match &case.references {
            References::Shared(_) => "the case's one reference, across backends".to_owned(),
            References::PerBackend(_) => format!(
                "the case's {} reference (one per backend)",
                renderer.backend
            ),
        };
        let cross = cross_backend(case)?;
        let render_sha = format!("{:x}", Sha256::digest(&render.rgb));
        let summary = format!(
            "case: {}\n{}\nbackend: {}\noutput: {output}\nreference: {reference}: {}\nrender RGB sha256: {render_sha}\n\
             verdict: {verdict}\nmetric: largest per-channel absolute difference, 8-bit steps\n\
             max step: {}\ntolerance: {} (max step {}; {})\ndiffering pixels: {} of {}\nmean step: {:.6}\n\
             first differing pixel: {}\n{}",
            case.name,
            renderer.adapter,
            renderer.backend,
            outcome.reference.display(),
            diff.max_step,
            case.tolerance.id,
            case.tolerance.max_step,
            case.tolerance.status,
            diff.differing,
            diff.pixels,
            diff.mean_step,
            diff.first
                .map_or("none".to_owned(), |(x, y)| format!("({x}, {y})")),
            cross.as_ref().map_or(String::new(), |c| format!("{c}\n")),
        );
        let dir = out.join(&case.name);
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        render.write_png(&dir.join("render.png"))?;
        diff.image.write_png(&dir.join("diff.png"))?;
        fs::write(dir.join("summary.txt"), &summary)
            .map_err(|e| format!("{}: {e}", dir.display()))?;
        println!(
            "xtask golden: {}: {verdict} on {} against {reference} (max step {}, {} of {} pixels differ; tolerance {}: \
             max step {}; output {output}; render RGB sha256 {render_sha}) -> {}",
            case.name,
            renderer.backend,
            diff.max_step,
            diff.differing,
            diff.pixels,
            case.tolerance.id,
            case.tolerance.max_step,
            dir.display()
        );
        if let Some(cross) = cross {
            println!("xtask golden: {}: {cross}", case.name);
        }
        outcomes.push(outcome);
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
        let image = renderer.render_output(&case.dir, &config, case.output)?;
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
        ["repro"] => Err(
            "golden repro: usage: golden repro <suite>/<case> [--vary <field>=<a>,<b>]..."
                .to_owned(),
        ),
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
