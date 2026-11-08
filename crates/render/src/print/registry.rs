//! The Print graph's node types are a registry (principia_print_graph.md §4.3; R-486, R-492, R-525), the Print graph's
//! own: the stain's codegen never sees it (render_gui_spec §12).
//!
//! Each entry is declared as data, a JSON object in `src/print/nodes/<id>.json`, its passes WGSL files beside it. An
//! entry declares its `id`, `label`, `category`, a one-paragraph `doc`, typed `inputs` with their defaults, typed
//! `outputs`, its `params` (each with a kind, a default and, for a number, a range, a step and a unit), its `passes`, a
//! `cost` class, a `motion` policy, whether it is `temporally_stable`, whether it is a `screen_medium` (R-525 V1), and
//! its `deviations` from the lab (R-492). A declaration missing any of them, or carrying a member no entry has, fails to
//! register ([`Registry::register`]).
//!
//! The registry is checked at build time (§4.3, §11 test 8): render's build script registers every entry and checks
//! every preset in `src/print/presets/` against them ([`check`]), and the build fails naming the preset, the node and
//! the parameter where a preset names a type that is not registered or sets a value outside its range.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value as Json};

use super::graph::{expand, Graph, Value};
use super::ports::{validate, InputDefault, PortType};

/// A node type's category: the lab's eleven, its `source` named "Print source" in the port, as the stage the graph
/// runs in is the print stage, so neither is read as the stain graph's `source` or `post` (§4.3; the human's ruling on
/// RQ-497, R-558).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    PrintSource,
    Analyse,
    Generate,
    AdjustCombine,
    Paint,
    Print,
    Screen,
    Output,
    Craft,
    Photographic,
    Machines,
}

impl Category {
    /// Every category, in the lab's catalogue's order.
    pub const ALL: [Category; 11] = [
        Category::PrintSource,
        Category::Analyse,
        Category::Generate,
        Category::AdjustCombine,
        Category::Paint,
        Category::Print,
        Category::Screen,
        Category::Output,
        Category::Craft,
        Category::Photographic,
        Category::Machines,
    ];

    /// The key an entry's `category` member writes: the lab catalogue's, `source` qualified as `print_source`.
    pub const fn key(self) -> &'static str {
        match self {
            Category::PrintSource => "print_source",
            Category::Analyse => "analyse",
            Category::Generate => "gen",
            Category::AdjustCombine => "op",
            Category::Paint => "paint",
            Category::Print => "print",
            Category::Screen => "screen",
            Category::Output => "out",
            Category::Craft => "craft",
            Category::Photographic => "photo",
            Category::Machines => "machine",
        }
    }

    /// The label the panel and the editor show: the lab catalogue's, "Source" read "Print source".
    pub const fn label(self) -> &'static str {
        match self {
            Category::PrintSource => "Print source",
            Category::Analyse => "Analyse",
            Category::Generate => "Generate",
            Category::AdjustCombine => "Adjust & combine",
            Category::Paint => "Paint",
            Category::Print => "Print",
            Category::Screen => "Screen",
            Category::Output => "Output",
            Category::Craft => "Craft",
            Category::Photographic => "Photographic",
            Category::Machines => "Machines",
        }
    }

    /// The category whose key is `key`.
    pub fn parse(key: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.key() == key)
    }
}

/// A node type's cost class (§8.3). Provisional: REQ-PERF-096 measures every node and replaces them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CostClass {
    Light,
    Medium,
    Heavy,
}

impl CostClass {
    pub const ALL: [CostClass; 3] = [CostClass::Light, CostClass::Medium, CostClass::Heavy];

    pub const fn name(self) -> &'static str {
        match self {
            CostClass::Light => "light",
            CostClass::Medium => "medium",
            CostClass::Heavy => "heavy",
        }
    }
}

/// How a node runs while the camera moves (§8.1): at full quality, reduced (half or quarter resolution, upsampled),
/// or bypassed (the identity).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotionPolicy {
    Full,
    Reduced,
    Bypass,
}

impl MotionPolicy {
    pub const ALL: [MotionPolicy; 3] = [
        MotionPolicy::Full,
        MotionPolicy::Reduced,
        MotionPolicy::Bypass,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            MotionPolicy::Full => "full",
            MotionPolicy::Reduced => "reduced",
            MotionPolicy::Bypass => "bypass",
        }
    }
}

/// A number parameter's unit (§6.2): a length is in `ref_px` (base px, scaled with the output's width), `device_px`
/// (true screen pixels, for effects that model the display) or `fraction` (of the figure's width); `none` is a number
/// that is no length (an amount, a count, a seed).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    RefPx,
    DevicePx,
    Fraction,
    None,
}

impl Unit {
    pub const ALL: [Unit; 4] = [Unit::RefPx, Unit::DevicePx, Unit::Fraction, Unit::None];

    pub const fn name(self) -> &'static str {
        match self {
            Unit::RefPx => "ref_px",
            Unit::DevicePx => "device_px",
            Unit::Fraction => "fraction",
            Unit::None => "none",
        }
    }
}

/// One typed input, with the default it reads unconnected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub name: String,
    pub ty: PortType,
    pub default: InputDefault,
}

/// One typed output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    pub ty: PortType,
}

/// What a parameter holds: a number in `[min, max]`, moved by `step`, in `unit`; a colour, `#rrggbb`; or a choice,
/// the index of one of `options` (the lab's `[index, [options]]`).
#[derive(Clone, Debug, PartialEq)]
pub enum ParamKind {
    Number {
        min: f64,
        max: f64,
        step: f64,
        unit: Unit,
    },
    Colour,
    Choice {
        options: Vec<String>,
    },
}

/// One parameter: its name, its default and its kind.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub default: Value,
    pub kind: ParamKind,
}

impl Param {
    /// Whether `value` is one this parameter takes: a number in its range, a colour `#rrggbb`, or an index of one of
    /// its options; with why not.
    pub fn admits(&self, value: &Value) -> Result<(), String> {
        match (&self.kind, value) {
            (ParamKind::Number { min, max, .. }, Value::Number(v)) => {
                if *min <= *v && *v <= *max {
                    Ok(())
                } else {
                    Err(format!("{v} is outside its range [{min}, {max}]"))
                }
            }
            (ParamKind::Colour, Value::Text(t)) if is_colour(t) => Ok(()),
            (ParamKind::Choice { options }, Value::Number(v))
                if v.fract() == 0.0 && *v >= 0.0 && *v < options.len() as f64 =>
            {
                Ok(())
            }
            (kind, v) => Err(format!("{v} is no value of its kind, {}", kind_name(kind))),
        }
    }
}

fn kind_name(kind: &ParamKind) -> String {
    match kind {
        ParamKind::Number { min, max, .. } => format!("a number in [{min}, {max}]"),
        ParamKind::Colour => "a colour, #rrggbb".to_owned(),
        ParamKind::Choice { options } => format!("an index of {options:?}"),
    }
}

/// Whether `t` is `#rrggbb`.
fn is_colour(t: &str) -> bool {
    t.len() == 7 && t.starts_with('#') && t[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// One pass: the file its WGSL came from and the WGSL, a function `fn run(uv: vec2<f32>) -> vec4<f32>` (the lab's
/// `vec4 run()`), which [`super::codegen`] places in the Print graph's own module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pass {
    pub file: String,
    pub wgsl: String,
}

/// A registered node type (§4.3).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: String,
    pub label: String,
    pub category: Category,
    pub doc: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub params: Vec<Param>,
    pub passes: Vec<Pass>,
    pub cost: CostClass,
    pub motion: MotionPolicy,
    pub temporally_stable: bool,
    pub screen_medium: bool,
    pub deviations: Vec<String>,
}

impl Entry {
    /// The parameter named `name`.
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }
}

/// The members a declaration must carry, each with what it holds.
pub const FIELDS: [(&str, &str); 13] = [
    ("id", "its id"),
    ("label", "its label"),
    ("category", "its category"),
    ("doc", "its one-paragraph doc"),
    ("inputs", "its typed inputs, with their defaults"),
    ("outputs", "its typed outputs"),
    ("params", "its parameters"),
    ("passes", "its passes, in WGSL"),
    ("cost", "its cost class: light, medium or heavy"),
    ("motion", "its motion policy: full, reduced or bypass"),
    ("temporally_stable", "whether it is temporally stable"),
    ("screen_medium", "whether it is a screen medium"),
    ("deviations", "its listed deviations from the lab"),
];

/// Why an entry did not register, or the registry and presets did not check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryError(pub String);

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RegistryError {}

/// The node types, by id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Registry {
    entries: BTreeMap<String, Entry>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Registry {
        Registry::default()
    }

    /// The entry of type `id`.
    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.get(id)
    }

    /// Every entry, by id.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    /// Registers the entry declared by `decl`, its pass files read through `wgsl`; refused, naming the entry and the
    /// member, where a member of [`FIELDS`] is missing or not of its kind, a member is one no entry has, the id is
    /// taken, not an identifier or a preset's, an input has no type or default, a number parameter has no range, step
    /// or unit, or a default outside its range, or a pass file is missing or defines no `run`.
    pub fn register(
        &mut self,
        decl: &str,
        wgsl: &dyn Fn(&str) -> Option<String>,
    ) -> Result<&Entry, RegistryError> {
        let json: Json = serde_json::from_str(decl)
            .map_err(|e| RegistryError(format!("an entry is not JSON: {e}")))?;
        let entry = entry(&json, wgsl)?;
        if self.entries.contains_key(&entry.id) {
            return Err(RegistryError(format!(
                "entry `{}` is registered twice",
                entry.id
            )));
        }
        let id = entry.id.clone();
        Ok(self.entries.entry(id).or_insert(entry))
    }
}

/// The entry `json` declares.
fn entry(json: &Json, wgsl: &dyn Fn(&str) -> Option<String>) -> Result<Entry, RegistryError> {
    let Some(obj) = json.as_object() else {
        return Err(RegistryError("an entry is not a JSON object".into()));
    };
    let id = obj
        .get("id")
        .and_then(Json::as_str)
        .unwrap_or("?")
        .to_owned();
    let refuse = |why: String| Err(RegistryError(format!("entry `{id}`: {why}")));
    for key in obj.keys() {
        if !FIELDS.iter().any(|(k, _)| k == key) {
            return refuse(format!("`{key}` is no member of an entry"));
        }
    }
    if let Some((key, what)) = FIELDS.iter().find(|(k, _)| !obj.contains_key(*k)) {
        return refuse(format!("no `{key}` ({what})"));
    }
    // An identifier holds no `:`, so no id is a preset's, `preset:<name>`.
    if !is_identifier(&id) {
        return refuse("the id is not an identifier, [a-z_][a-z0-9_]*".into());
    }
    let category = obj
        .get("category")
        .and_then(Json::as_str)
        .and_then(Category::parse);
    let Some(category) = category else {
        let keys: Vec<&str> = Category::ALL.iter().map(|c| c.key()).collect();
        return refuse(format!("the category is none of {keys:?}"));
    };
    let doc = text(obj, "doc").map_err(|w| RegistryError(format!("entry `{id}`: {w}")))?;
    if doc.contains("\n\n") {
        return refuse("the doc is more than one paragraph".into());
    }
    let named = |key: &str| -> Result<String, RegistryError> {
        text(obj, key).map_err(|w| RegistryError(format!("entry `{id}`: {w}")))
    };
    let cost = choose(obj, "cost", &CostClass::ALL, |c| c.name());
    let motion = choose(obj, "motion", &MotionPolicy::ALL, |m| m.name());
    let (Some(cost), Some(motion)) = (cost, motion) else {
        return refuse("the cost is not light, medium or heavy, or the motion policy not full, reduced or bypass".into());
    };
    let flag = |key: &str| obj.get(key).and_then(Json::as_bool);
    let (Some(temporally_stable), Some(screen_medium)) =
        (flag("temporally_stable"), flag("screen_medium"))
    else {
        return refuse("`temporally_stable` and `screen_medium` are each true or false".into());
    };
    let deviations = strings(obj, "deviations").ok_or_else(|| {
        RegistryError(format!(
            "entry `{id}`: `deviations` is not a list of strings"
        ))
    })?;
    let passes = strings(obj, "passes").ok_or_else(|| {
        RegistryError(format!(
            "entry `{id}`: `passes` is not a list of WGSL files"
        ))
    })?;
    let mut loaded = Vec::with_capacity(passes.len());
    for file in passes {
        let Some(text) = wgsl(&file) else {
            return refuse(format!("the pass `{file}` is not there"));
        };
        if !defines_run(&text) {
            return refuse(format!("the pass `{file}` defines no `fn run`"));
        }
        loaded.push(Pass { file, wgsl: text });
    }
    Ok(Entry {
        label: named("label")?,
        category,
        doc,
        inputs: inputs(obj).map_err(|w| RegistryError(format!("entry `{id}`: {w}")))?,
        outputs: outputs(obj).map_err(|w| RegistryError(format!("entry `{id}`: {w}")))?,
        params: params(obj).map_err(|w| RegistryError(format!("entry `{id}`: {w}")))?,
        passes: loaded,
        cost,
        motion,
        temporally_stable,
        screen_medium,
        deviations,
        id,
    })
}

/// The non-empty string member `key`.
fn text(obj: &Map<String, Json>, key: &str) -> Result<String, String> {
    match obj.get(key).and_then(Json::as_str) {
        Some(s) if !s.trim().is_empty() => Ok(s.to_owned()),
        _ => Err(format!("`{key}` is not a non-empty string")),
    }
}

/// The member `key`, one of `all` by name.
fn choose<T: Copy>(
    obj: &Map<String, Json>,
    key: &str,
    all: &[T],
    name: fn(T) -> &'static str,
) -> Option<T> {
    let s = obj.get(key).and_then(Json::as_str)?;
    all.iter().copied().find(|t| name(*t) == s)
}

/// The member `key`, a list of strings.
fn strings(obj: &Map<String, Json>, key: &str) -> Option<Vec<String>> {
    obj.get(key)?
        .as_array()?
        .iter()
        .map(|s| s.as_str().map(str::to_owned))
        .collect()
}

/// The members of each port listed in `key`: a list of objects, each carrying no member but `allowed`.
fn port_list<'a>(
    obj: &'a Map<String, Json>,
    key: &str,
    allowed: &[&str],
) -> Result<Vec<&'a Map<String, Json>>, String> {
    let Some(list) = obj.get(key).and_then(Json::as_array) else {
        return Err(format!("`{key}` is not a list"));
    };
    let mut out = Vec::with_capacity(list.len());
    for port in list {
        let Some(p) = port.as_object() else {
            return Err(format!("`{key}` holds {port}, not a port"));
        };
        if let Some(k) = p.keys().find(|k| !allowed.contains(&k.as_str())) {
            return Err(format!(
                "a port of `{key}` carries `{k}`, which no port has"
            ));
        }
        out.push(p);
    }
    Ok(out)
}

/// Port `p` of `key`'s name and type; `taken` holds the names before it.
fn port(p: &Map<String, Json>, key: &str, taken: &[&str]) -> Result<(String, PortType), String> {
    let name = p.get("name").and_then(Json::as_str).unwrap_or("");
    if !is_identifier(name) || taken.contains(&name) {
        return Err(format!(
            "the port name {name:?} of `{key}` is not an identifier, or is used twice"
        ));
    }
    let Some(ty) = p
        .get("type")
        .and_then(Json::as_str)
        .and_then(PortType::parse)
    else {
        return Err(format!("the port `{name}` of `{key}` has no type"));
    };
    Ok((name.to_owned(), ty))
}

/// The inputs listed in `inputs`, each `{ name, type, default }`.
fn inputs(obj: &Map<String, Json>) -> Result<Vec<Input>, String> {
    let mut out: Vec<Input> = Vec::new();
    for p in port_list(obj, "inputs", &["name", "type", "default"])? {
        let taken: Vec<&str> = out.iter().map(|i| i.name.as_str()).collect();
        let (name, ty) = port(p, "inputs", &taken)?;
        let default = p
            .get("default")
            .and_then(Json::as_str)
            .and_then(InputDefault::parse);
        let Some(default) = default else {
            return Err(format!("the input `{name}` has no default"));
        };
        out.push(Input { name, ty, default });
    }
    Ok(out)
}

/// The outputs listed in `outputs`, each `{ name, type }`.
fn outputs(obj: &Map<String, Json>) -> Result<Vec<Output>, String> {
    let mut out: Vec<Output> = Vec::new();
    for p in port_list(obj, "outputs", &["name", "type"])? {
        let taken: Vec<&str> = out.iter().map(|o| o.name.as_str()).collect();
        let (name, ty) = port(p, "outputs", &taken)?;
        out.push(Output { name, ty });
    }
    Ok(out)
}

/// The parameters listed in `params`.
fn params(obj: &Map<String, Json>) -> Result<Vec<Param>, String> {
    let Some(list) = obj.get("params").and_then(Json::as_array) else {
        return Err("`params` is not a list".into());
    };
    let mut out: Vec<Param> = Vec::with_capacity(list.len());
    for p in list {
        let Some(p) = p.as_object() else {
            return Err(format!("`params` holds {p}, not a parameter"));
        };
        let name = p.get("name").and_then(Json::as_str).unwrap_or("");
        if !is_identifier(name) || out.iter().any(|q| q.name == name) {
            return Err(format!(
                "the parameter name {name:?} is not an identifier, or is used twice"
            ));
        }
        let param = param(name, p).map_err(|why| format!("parameter `{name}`: {why}"))?;
        out.push(param);
    }
    Ok(out)
}

/// Parameter `name`, declared by `p`.
fn param(name: &str, p: &Map<String, Json>) -> Result<Param, String> {
    let kind = p.get("kind").and_then(Json::as_str).unwrap_or("");
    let allowed: &[&str] = match kind {
        "number" => &["name", "kind", "default", "min", "max", "step", "unit"],
        "colour" => &["name", "kind", "default"],
        "choice" => &["name", "kind", "default", "options"],
        _ => return Err("its kind is not number, colour or choice".into()),
    };
    if let Some(k) = p.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(format!("`{k}` is no member of a {kind} parameter"));
    }
    let num = |key: &str| p.get(key).and_then(Json::as_f64).filter(|v| v.is_finite());
    let default = match (kind, p.get("default")) {
        ("colour", Some(Json::String(s))) => Some(Value::Text(s.clone())),
        (_, Some(Json::Number(n))) => n.as_f64().map(Value::Number),
        _ => None,
    };
    let Some(default) = default else {
        return Err("it has no default of its kind".into());
    };
    let kind = match kind {
        "number" => {
            let (Some(min), Some(max)) = (num("min"), num("max")) else {
                return Err("it has no range, `min` and `max`".into());
            };
            if min >= max {
                return Err(format!("its range [{min}, {max}] is empty"));
            }
            let Some(step) = num("step").filter(|s| *s > 0.0) else {
                return Err("it has no step, a positive number".into());
            };
            let unit = p.get("unit").and_then(Json::as_str);
            let Some(unit) = unit.and_then(|u| Unit::ALL.into_iter().find(|x| x.name() == u))
            else {
                return Err(
                    "it has no unit: `ref_px`, `device_px` or `fraction` for a length (§6.2), `none` otherwise"
                        .into(),
                );
            };
            ParamKind::Number {
                min,
                max,
                step,
                unit,
            }
        }
        "colour" => ParamKind::Colour,
        _ => {
            let options = strings(p, "options").filter(|o| !o.is_empty());
            let Some(options) = options else {
                return Err("it has no options".into());
            };
            ParamKind::Choice { options }
        }
    };
    let param = Param {
        name: name.to_owned(),
        default,
        kind,
    };
    param
        .admits(&param.default)
        .map_err(|why| format!("its default {why}"))?;
    Ok(param)
}

/// Whether `s` is `[a-z_][a-z0-9_]*`: an id, a port or a parameter name, each one a WGSL identifier's part.
fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Whether `wgsl` holds `fn run(`, spaces allowed between the tokens.
fn defines_run(wgsl: &str) -> bool {
    wgsl.split("fn").skip(1).any(|rest| {
        rest.starts_with(char::is_whitespace)
            && rest
                .trim_start()
                .strip_prefix("run")
                .is_some_and(|r| r.trim_start().starts_with('('))
    })
}

// ── The build-time check ────────────────────────────────────────────────────────────────────────────────────────

/// The registry and the presets, checked together.
#[derive(Clone, Debug, PartialEq)]
pub struct Checked {
    pub registry: Registry,
    pub presets: BTreeMap<String, Graph>,
}

/// The check render's build script runs (§4.3, §11 test 8): every entry of `nodes` (file name, declaration)
/// registers, its passes read from `wgsl` (file name, WGSL), and every preset of `presets` (file name, graph) parses,
/// its name unique, and passes [`check_preset`]. Refused with the first failure, named.
pub fn check(
    nodes: &[(&str, &str)],
    wgsl: &[(&str, &str)],
    presets: &[(&str, &str)],
) -> Result<Checked, RegistryError> {
    let read = |file: &str| {
        wgsl.iter()
            .find(|(name, _)| *name == file)
            .map(|(_, text)| (*text).to_owned())
    };
    let mut registry = Registry::new();
    for (file, decl) in nodes {
        registry
            .register(decl, &read)
            .map_err(|e| RegistryError(format!("{file}: {e}")))?;
    }
    let mut graphs = BTreeMap::new();
    for (file, text) in presets {
        let graph =
            Graph::parse(text).map_err(|e| RegistryError(format!("preset file {file}: {e}")))?;
        if graphs.contains_key(&graph.name) {
            return Err(RegistryError(format!(
                "preset file {file}: a preset named `{}` is already there",
                graph.name
            )));
        }
        graphs.insert(graph.name.clone(), graph);
    }
    for name in graphs.keys() {
        check_preset(name, &graphs, &registry)?;
    }
    Ok(Checked {
        registry,
        presets: graphs,
    })
}

/// Preset `name` of `presets` against `registry`: it expands, every node's type is registered and its wiring checks
/// ([`validate`]), and every parameter is one its type declares, with a value in its range; refused naming the
/// preset, the node and the parameter.
pub fn check_preset(
    name: &str,
    presets: &BTreeMap<String, Graph>,
    registry: &Registry,
) -> Result<(), RegistryError> {
    let refuse = |why: String| Err(RegistryError(format!("preset `{name}`: {why}")));
    let Some(graph) = presets.get(name) else {
        return refuse("there is no such preset".into());
    };
    let expanded = match expand(graph, &|n: &str| presets.get(n).cloned()) {
        Ok(e) => e,
        Err(e) => return refuse(e.0),
    };
    if let Err(e) = validate(&expanded, registry) {
        return refuse(e.0);
    }
    for (id, node) in &expanded.nodes {
        // `validate` has refused every node whose type is not registered.
        let Some(entry) = registry.get(&node.ty) else {
            continue;
        };
        check_params(id, entry, &node.params)
            .map_err(|why| RegistryError(format!("preset `{name}`: {why}")))?;
    }
    Ok(())
}

/// Node `id`'s parameters `params` against its type `entry`: each one `entry` declares, with a value it admits;
/// refused naming the node, its type and the parameter.
pub fn check_params(
    id: &str,
    entry: &Entry,
    params: &BTreeMap<String, Value>,
) -> Result<(), String> {
    for (param, value) in params {
        let Some(p) = entry.param(param) else {
            return Err(format!(
                "node `{id}` ({}): `{param}` is no parameter of `{}`",
                entry.id, entry.id
            ));
        };
        if let Err(why) = p.admits(value) {
            return Err(format!(
                "node `{id}` ({}): parameter `{param}` = {why}",
                entry.id
            ));
        }
    }
    Ok(())
}
