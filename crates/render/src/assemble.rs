//! The one assembler (render contract Part 2; lowering contract Part 2, Part 3a): a stain graph lowers to one fragment
//! source, `[prelude][node functions][shade()]`. Built-in, debug and custom occupants all reach it as WGSL text through
//! [`assemble`], the one entry point; there is no second compile path (REQ-RENDER-010).
//!
//! **The graph** ([`Stain`]) is a list of nodes in an order where every wire runs forward, so it is acyclic by
//! construction, and every wire obeys the port types and the backbone (render_gui_spec Part II §4, §6;
//! colour_composition §4): sources feed colour and brightness, which meet only at the combiner, then the post chain,
//! then `OUT` ([`check_wire`]). `combiner` and `OUT` are singletons, and the post chain holds at most [`MAX_POSTS`]
//! posts (colour_composition §4.2). Its canonical form ([`Stain::canonical`]) is what the fragment key hashes and what
//! is assembled. The engine's stain-graph type (`engine::stain`) lowers to it.
//!
//! **The slot functions** each occupant defines (render contract Part 2; colour_composition §4.2 for the post's `ctx`):
//! `fn source(ctx: Ctx) -> Field`, `fn colour(ctx: Ctx) -> vec3<f32>` (linear RGB), `fn brightness(ctx: Ctx) -> f32`
//! (nominal [0, 1]), `fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32>` and `fn post(ctx: Ctx, rgb: vec3<f32>) ->
//! vec3<f32>`; the compiled module is checked against them (REQ-RENDER-016). `OUT` has no occupant: it is `shade()`'s
//! return. An occupant reads the read-side `SimState` by plain member access, `ctx.sample.ftle`, at every tier
//! (lowering Part 3a), its wired fields as `ctx.inputs[k]`, and its uniforms as `uniforms.<name>`, declared in the
//! file's header ([`Declaration`]; gui_state_contract §3).
//!
//! **Identity** (render_gui_spec §13; colour_composition §4.1): a node whose occupant is None, or one of whose field
//! inputs is absent, is the identity. A colour None gives the combiner white, so the combiner gives the greyscale of
//! the brightness; a brightness None keeps the colour as it is; both None give the flat mid-grey OKLab (0.6, 0, 0); a
//! post None passes its colour on. A post's or `OUT`'s absent colour input reads the combiner: the chain before it is
//! empty. So every graph renders.
//!
//! **The read side (R-378).** The assembler derives the stain's field set from its IR ([`field_set`]), every member
//! of the read-side `SimState` the live nodes read, the word as `word` or as only the components `word.x` … `word.w`
//! it reads, and generates the unpack layer and read side at its tier for that set through
//! `ledger::gen::read::assemble`, never the checked-in full-fill `read_side.wgsl`. A field `sample_read` does not
//! fill reads 0, a plausible value with no validity signal, so [`assemble_reading`] refuses a stain that reads a field
//! its set misses. Nothing a node writes may name the stored buffers or `sample_read` (R-343).

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::sync::OnceLock;

use ledger::gen::{self as generate, prelude, read};
use ledger::schema::{Entry, Scale, Word};
use naga::valid::{Capabilities, FunctionInfo, ModuleInfo, ValidationFlags, Validator};
use naga::{Block, Expression, Function, Handle, Module, Statement, TypeInner};

pub use ledger::gen::read::Tier;

/// The colour-space maps the prelude lacks, linear sRGB → OKLab and the maps built on it (dd_colouring §3.1;
/// render_gui_spec §10.1), hand-written; they follow the prelude at assembly, so every node may call them.
const COLOUR_SPACE: &str = include_str!("../shaders/wgsl/lib/colour_space.wgsl");

/// The presentation layer, hand-written; it follows the prelude at assembly (render contract Part 5).
const PRESENT: &str = include_str!("../shaders/wgsl/lib/present.wgsl");

/// The built-in occupants, each its slot, id and WGSL file (gui_state_contract §3's `shaders/wgsl/frag/<slot>/`).
const BUILTINS: [(Kind, &str, &str); 1] = [(
    Kind::Combiner,
    "pass_through",
    include_str!("../shaders/wgsl/frag/combiner/pass_through.wgsl"),
)];

/// The post chain's bound (colour_composition §4.2: "Bounded at ≤ 8").
pub const MAX_POSTS: usize = 8;

/// The most field inputs a node takes, the length of `ctx.inputs` (applied per R-369: the corpus gives no bound).
pub const MAX_INPUTS: usize = 4;

/// The flat mid-grey's OKLab lightness, both slots None (colour_composition §4.1: `OKLab(0.6,0,0)`).
const MID_GREY_L: &str = "0.6";

/// Names a node may not write: the stored buffers and the read side's one reader of them (R-343, R-378).
const RESERVED: [&str; 3] = ["simstate_buffer", "word_buffer", "sample_read"];

// ── The graph's vocabulary ───────────────────────────────────────────────────────────────────────────────────────

/// A node kind (render_gui_spec Part II §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Source,
    Colour,
    Brightness,
    Combiner,
    Post,
    Out,
}

impl Kind {
    /// The kind's name, as the canonical form writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Source => "source",
            Kind::Colour => "colour",
            Kind::Brightness => "brightness",
            Kind::Combiner => "combiner",
            Kind::Post => "post",
            Kind::Out => "out",
        }
    }

    /// The function an occupant of this kind defines; `OUT` has no occupant.
    pub const fn slot(self) -> Option<&'static str> {
        match self {
            Kind::Source => Some("source"),
            Kind::Colour => Some("colour"),
            Kind::Brightness => Some("brightness"),
            Kind::Combiner => Some("combine"),
            Kind::Post => Some("post"),
            Kind::Out => None,
        }
    }

    /// The type on the kind's out-port; `OUT` has none.
    pub const fn output(self) -> Option<PortType> {
        match self {
            Kind::Source => Some(PortType::Field),
            Kind::Colour | Kind::Combiner | Kind::Post => Some(PortType::Vec3),
            Kind::Brightness => Some(PortType::F32),
            Kind::Out => None,
        }
    }

    /// The slot function's parameter and result types, as [`header`] normalises them.
    fn signature(self) -> (&'static [&'static str], &'static str) {
        match self {
            Kind::Source => (&["Ctx"], "vec4<f32>"),
            Kind::Colour => (&["Ctx"], "vec3<f32>"),
            Kind::Brightness => (&["Ctx"], "f32"),
            Kind::Combiner => (&["vec3<f32>", "f32"], "vec3<f32>"),
            Kind::Post | Kind::Out => (&["Ctx", "vec3<f32>"], "vec3<f32>"),
        }
    }
}

/// A wire-carried type (render_gui_spec Part II §3): a ctx field, a colour or a lightness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortType {
    Field,
    Vec3,
    F32,
}

/// A `field` port's subtype (render_gui_spec Part II §3): a tag the inspector reads, never a wire gate (§6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Subtype {
    Scalar,
    Vector,
    Categorical,
}

/// What occupies a node (render contract Part 2: custom is an occupant, not a kind).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Occupant {
    /// The identity (colour_composition §4.1; render_gui_spec §13); `OUT`'s only occupant.
    None,
    /// The built-in source of a read-side field, by its member name (render_gui_spec Part II §3.1: "which ctx field").
    Field(String),
    /// A built-in occupant, by its id in its slot.
    BuiltIn(String),
    /// A custom occupant's WGSL, as written.
    Custom(String),
}

// ── The declaration format (gui_state_contract §3; REQ-GEN-027) ──────────────────────────────────────────────────

/// A uniform's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UniformType {
    F32,
    I32,
    U32,
    Vec2,
    Vec3,
    Vec4,
}

impl UniformType {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "f32" => UniformType::F32,
            "i32" => UniformType::I32,
            "u32" => UniformType::U32,
            "vec2<f32>" => UniformType::Vec2,
            "vec3<f32>" => UniformType::Vec3,
            "vec4<f32>" => UniformType::Vec4,
            _ => return None,
        })
    }

    /// The type as WGSL writes it.
    pub const fn wgsl(self) -> &'static str {
        match self {
            UniformType::F32 => "f32",
            UniformType::I32 => "i32",
            UniformType::U32 => "u32",
            UniformType::Vec2 => "vec2<f32>",
            UniformType::Vec3 => "vec3<f32>",
            UniformType::Vec4 => "vec4<f32>",
        }
    }

    /// Its number of components.
    pub const fn components(self) -> usize {
        match self {
            UniformType::F32 | UniformType::I32 | UniformType::U32 => 1,
            UniformType::Vec2 => 2,
            UniformType::Vec3 => 3,
            UniformType::Vec4 => 4,
        }
    }

    /// Whether `v` is a value of a component of this type: an integer in range for `i32` and `u32`; for an `f32`
    /// component, a number that rounds to a finite `f32`, as a WGSL `f32` literal does, so nothing beyond `f32::MAX`
    /// but its rounding: `1e39` is refused, and `3.4028235e38`, `f32::MAX` as Rust writes it, is `f32::MAX`.
    fn holds(self, v: f64) -> bool {
        match self {
            UniformType::I32 => {
                v.fract() == 0.0 && (-2_147_483_648.0..=2_147_483_647.0).contains(&v)
            }
            UniformType::U32 => v.fract() == 0.0 && (0.0..=4_294_967_295.0).contains(&v),
            // `as` rounds to nearest, and to infinity past f32::MAX's rounding interval.
            _ => (v as f32).is_finite(),
        }
    }
}

/// One `uniformSchema` entry: `// @uniform <name>: <type> = <default> [<lo>, <hi>]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Uniform {
    pub name: String,
    pub ty: UniformType,
    /// One value per component.
    pub default: Vec<f64>,
    /// The control's range, scalar types only.
    pub range: Option<(f64, f64)>,
}

impl Uniform {
    /// Whether `value` is a value of this uniform: one number per component, each of the type, within the range.
    pub fn admits(&self, value: &[f64]) -> bool {
        value.len() == self.ty.components()
            && value.iter().all(|&v| {
                self.ty.holds(v) && self.range.is_none_or(|(lo, hi)| (lo..=hi).contains(&v))
            })
    }
}

/// One field input: `// @input <name> [<lo>, <hi>]`, the bracket its `inputDomains` entry, inherited when absent.
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub name: String,
    pub domain: Option<(f64, f64)>,
}

/// An occupant's declarations: its `uniformSchema` and its field inputs with their `inputDomains` (R-53).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Declaration {
    pub uniforms: Vec<Uniform>,
    pub inputs: Vec<Input>,
}

impl Declaration {
    /// The declarations in `source`: each line whose text, leading whitespace removed, starts `// @`. Any word after
    /// `@` but `uniform` and `input` is refused, as is a malformed line or a name declared twice.
    pub fn parse(source: &str) -> Result<Declaration, AssembleError> {
        let mut out = Declaration::default();
        for (k, line) in source.lines().enumerate() {
            let Some(rest) = line.trim_start().strip_prefix("// @") else {
                continue;
            };
            let bad = |why: &str| AssembleError::Declaration {
                line: k + 1,
                why: format!("`{}`: {why}", line.trim()),
            };
            let (word, rest) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            match word {
                "uniform" => out.uniforms.push(uniform(rest).map_err(|w| bad(&w))?),
                "input" => out.inputs.push(input(rest).map_err(|w| bad(&w))?),
                _ => return Err(bad("a declaration is `@uniform` or `@input`")),
            }
            let names: Vec<&str> = match word {
                "uniform" => out.uniforms.iter().map(|u| u.name.as_str()).collect(),
                _ => out.inputs.iter().map(|i| i.name.as_str()).collect(),
            };
            if names[..names.len() - 1].contains(names.last().unwrap_or(&"")) {
                return Err(bad("the name is declared twice"));
            }
        }
        Ok(out)
    }
}

/// `<name>: <type> = <default>` then an optional `[<lo>, <hi>]`.
fn uniform(rest: &str) -> Result<Uniform, String> {
    let (name, rest) = rest.split_once(':').ok_or("no `:` after the name")?;
    let (ty, rest) = rest.split_once('=').ok_or("no `=` before the default")?;
    let name = identifier(name)?;
    let ty =
        UniformType::parse(ty.trim()).ok_or("the type is not f32, i32, u32 or vec2/3/4<f32>")?;
    let rest = rest.trim();
    let (default, rest) = match rest.strip_prefix('(') {
        Some(inner) => {
            let (list, rest) = inner.split_once(')').ok_or("an unclosed `(`")?;
            (numbers(list)?, rest)
        }
        None => {
            let end = rest.find(['[', ' ']).unwrap_or(rest.len());
            (vec![number(&rest[..end])?], &rest[end..])
        }
    };
    let range = bracket(rest)?;
    let u = Uniform {
        name,
        ty,
        default,
        range,
    };
    if u.range.is_some() && ty.components() > 1 {
        return Err("a range is for a scalar uniform".into());
    }
    if !u.admits(&u.default) {
        return Err("the default is not a value of the type within the range".into());
    }
    Ok(u)
}

/// `<name>` then an optional `[<lo>, <hi>]`.
fn input(rest: &str) -> Result<Input, String> {
    let rest = rest.trim();
    let end = rest.find(['[', ' ']).unwrap_or(rest.len());
    Ok(Input {
        name: identifier(&rest[..end])?,
        domain: bracket(&rest[end..])?,
    })
}

/// An optional `[<lo>, <hi>]` with `lo < hi`, and nothing after it.
fn bracket(rest: &str) -> Result<Option<(f64, f64)>, String> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(None);
    }
    let inner = rest
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or("expected `[<lo>, <hi>]` and nothing after it")?;
    match numbers(inner)?[..] {
        [lo, hi] if lo < hi => Ok(Some((lo, hi))),
        _ => Err("a range is two numbers, `lo < hi`".into()),
    }
}

fn numbers(list: &str) -> Result<Vec<f64>, String> {
    list.split(',').map(number).collect()
}

/// A WGSL decimal literal, unsuffixed, finite: Rust's `f64` syntax, whose only words, `inf`, `infinity` and `nan`, are
/// not finite.
fn number(s: &str) -> Result<f64, String> {
    let s = s.trim();
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(format!("`{s}` is not a number")),
    }
}

/// An ASCII WGSL identifier (gui_state_contract §3): an ASCII letter or `_`, then ASCII letters, digits or `_`; not
/// `_` alone and not beginning `__`, which WGSL does not take as identifiers.
fn identifier(s: &str) -> Result<String, String> {
    let s = s.trim();
    let mut chars = s.chars();
    let ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && s != "_"
        && !s.starts_with("__");
    if ok {
        Ok(s.to_owned())
    } else {
        Err(format!("`{s}` is not a name"))
    }
}

// ── Occupants and wires ───────────────────────────────────────────────────────────────────────────────────────────

/// The built-in occupant `id` of `kind`'s slot, as its WGSL file.
pub fn builtin(kind: Kind, id: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .find(|(k, i, _)| *k == kind && *i == id)
        .map(|(.., source)| *source)
}

/// The read-side fields a built-in source may read: the scalar members, `f32`, `u32` or `bool` (lowering Part 3a).
pub fn source_fields() -> Result<Vec<String>, AssembleError> {
    let (words, entries) = ledger()?;
    Ok(read::members(words, entries)
        .into_iter()
        .filter(|m| matches!(m.wgsl, "f32" | "u32" | "bool"))
        .map(|m| m.name)
        .collect())
}

/// The subtype of a built-in source's field (render_gui_spec Part II §5): categorical for a `bool` or a field the
/// ledger scales as categorical or as a flag (dd_generation_root §3.8 `scale`), scalar otherwise; or the field is none.
pub fn field_subtype(field: &str) -> Result<Subtype, AssembleError> {
    let (words, entries) = ledger()?;
    let member = read::members(words, entries)
        .into_iter()
        .find(|m| m.name == field && matches!(m.wgsl, "f32" | "u32" | "bool"))
        .ok_or_else(|| AssembleError::Occupant(format!("`{field}` is no field a source reads")))?;
    let scale = entries.iter().find(|e| e.name == field).map(|e| e.scale);
    Ok(match (member.wgsl, scale) {
        ("bool", _) | (_, Some(Scale::Categorical(_) | Scale::Flag)) => Subtype::Categorical,
        _ => Subtype::Scalar,
    })
}

/// The WGSL `occupant` gives a node of `kind`, or `None` for the identity: a built-in's file and a custom's text alike,
/// the built-in source generated from its field. An occupant the slot does not take is refused.
fn occupant_source(kind: Kind, occupant: &Occupant) -> Result<Option<String>, AssembleError> {
    let refuse = |what: &str| {
        Err(AssembleError::Occupant(format!(
            "a {} node does not take {what}",
            kind.name()
        )))
    };
    match (kind, occupant) {
        (Kind::Out, Occupant::None) => Ok(None),
        (Kind::Out, _) => refuse("an occupant"),
        (Kind::Combiner, Occupant::None) => refuse("None: the combiner is required"),
        (_, Occupant::None) => Ok(None),
        (Kind::Source, Occupant::Field(f)) => {
            let ty = field_type(f)?;
            let value = match ty {
                "bool" => format!("select(0.0, 1.0, ctx.sample.{f})"),
                _ => format!("f32(ctx.sample.{f})"),
            };
            Ok(Some(format!(
                "// The built-in source of `{f}`: the field in `.x` (render_gui_spec Part II §3.1).\n\
                 fn source(ctx: Ctx) -> Field {{ return Field({value}, 0.0, 0.0, 0.0); }}\n"
            )))
        }
        (_, Occupant::Field(_)) => refuse("a field: only a source does"),
        (_, Occupant::BuiltIn(id)) => builtin(kind, id)
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| AssembleError::Occupant(format!("no built-in {} `{id}`", kind.name()))),
        (_, Occupant::Custom(text)) => Ok(Some(text.clone())),
    }
}

/// A built-in source's field's WGSL type, or the field is none.
fn field_type(field: &str) -> Result<&'static str, AssembleError> {
    let (words, entries) = ledger()?;
    read::members(words, entries)
        .into_iter()
        .find(|m| m.name == field && matches!(m.wgsl, "f32" | "u32" | "bool"))
        .map(|m| m.wgsl)
        .ok_or_else(|| AssembleError::Occupant(format!("`{field}` is no field a source reads")))
}

/// `occupant`'s declarations in a node of `kind`, checked against the kind: a source and a combiner declare no
/// input; a colour declares one to [`MAX_INPUTS`], a brightness one, and a post up to [`MAX_INPUTS`] (a colour or a
/// brightness that declares none has one, `field`, its domain inherited); an occupant the slot does not take is refused.
pub fn declaration(kind: Kind, occupant: &Occupant) -> Result<Declaration, AssembleError> {
    let Some(source) = occupant_source(kind, occupant)? else {
        let mut d = Declaration::default();
        if matches!(kind, Kind::Colour | Kind::Brightness) {
            d.inputs.push(default_input());
        }
        return Ok(d);
    };
    let mut d = Declaration::parse(&source)?;
    if matches!(kind, Kind::Colour | Kind::Brightness) && d.inputs.is_empty() {
        d.inputs.push(default_input());
    }
    let most = match kind {
        Kind::Source | Kind::Combiner | Kind::Out => 0,
        Kind::Brightness => 1,
        Kind::Colour | Kind::Post => MAX_INPUTS,
    };
    if d.inputs.len() > most {
        return Err(AssembleError::Occupant(format!(
            "a {} node takes at most {most} field input(s); this occupant declares {}",
            kind.name(),
            d.inputs.len()
        )));
    }
    Ok(d)
}

fn default_input() -> Input {
    Input {
        name: "field".into(),
        domain: None,
    }
}

/// The in-ports of a node of `kind` with `declaration`, in order (render_gui_spec Part II §3.1): a post's `vec3` first,
/// then its field inputs; the combiner's colour then brightness.
pub fn in_ports(kind: Kind, declaration: &Declaration) -> Vec<PortType> {
    let fields = vec![PortType::Field; declaration.inputs.len()];
    match kind {
        Kind::Source => Vec::new(),
        Kind::Colour | Kind::Brightness => fields,
        Kind::Combiner => vec![PortType::Vec3, PortType::F32],
        Kind::Post => [vec![PortType::Vec3], fields].concat(),
        Kind::Out => vec![PortType::Vec3],
    }
}

/// Whether a wire from a `from` node into `to`'s `port` is allowed: the out-port's type is the in-port's
/// (render_gui_spec Part II §6), and the wire keeps the backbone (§4; colour_composition §4): the combiner's colour
/// comes from a colour node, and a post's or `OUT`'s colour from the combiner or a post.
pub fn check_wire(from: Kind, to: Kind, port: PortType) -> Result<(), AssembleError> {
    if from.output() != Some(port) {
        return Err(AssembleError::Wire(format!(
            "a {} out-port ({:?}) cannot feed a {:?} in-port of a {} node",
            from.name(),
            from.output(),
            port,
            to.name()
        )));
    }
    let backbone = match (to, port) {
        (Kind::Combiner, PortType::Vec3) => from == Kind::Colour,
        (Kind::Post | Kind::Out, PortType::Vec3) => matches!(from, Kind::Combiner | Kind::Post),
        _ => true,
    };
    if backbone {
        Ok(())
    } else {
        Err(AssembleError::Backbone(format!(
            "a {} node cannot feed a {} node's colour: the backbone is sources → colour / brightness → combiner → \
             (post)* → OUT",
            from.name(),
            to.name()
        )))
    }
}

// ── The graph ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// One node: its kind, its occupant, and the node feeding each in-port, by position, `None` where absent.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Node {
    pub kind: Kind,
    pub occupant: Occupant,
    pub inputs: Vec<Option<usize>>,
}

/// A stain graph the assembler takes, checked at construction ([`Stain::new`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Stain {
    nodes: Vec<Node>,
    declarations: Vec<Declaration>,
}

impl Stain {
    /// `nodes` as a stain, or why not: an occupant its slot does not take or a malformed declaration; a node whose
    /// inputs are not its in-ports; a wire that runs backward (from a later node or itself, so a cycle cannot be
    /// written) or breaks the port types or the backbone ([`check_wire`]); other than one combiner and one `OUT`; or
    /// more than [`MAX_POSTS`] posts in the live chain.
    pub fn new(nodes: Vec<Node>) -> Result<Stain, AssembleError> {
        let mut declarations = Vec::new();
        for (i, n) in nodes.iter().enumerate() {
            let d = declaration(n.kind, &n.occupant)?;
            let ports = in_ports(n.kind, &d);
            if ports.len() != n.inputs.len() {
                return Err(AssembleError::Wire(format!(
                    "node {i} ({}) has {} in-port(s); {} input(s) given",
                    n.kind.name(),
                    ports.len(),
                    n.inputs.len()
                )));
            }
            for (&port, input) in ports.iter().zip(&n.inputs) {
                let Some(j) = *input else { continue };
                if j >= i {
                    return Err(AssembleError::Cycle(format!(
                        "node {i} reads node {j}: every wire runs forward, so the graph is acyclic"
                    )));
                }
                check_wire(nodes[j].kind, n.kind, port)?;
            }
            declarations.push(d);
        }
        for kind in [Kind::Combiner, Kind::Out] {
            let count = nodes.iter().filter(|n| n.kind == kind).count();
            if count != 1 {
                return Err(AssembleError::Singleton(format!(
                    "a stain has one {} node; this one has {count}",
                    kind.name()
                )));
            }
        }
        let stain = Stain {
            nodes,
            declarations,
        };
        let posts = stain.chain().len();
        if posts > MAX_POSTS {
            return Err(AssembleError::PostChain(format!(
                "the post chain holds {posts} posts; at most {MAX_POSTS} (colour_composition §4.2)"
            )));
        }
        Ok(stain)
    }

    /// The nodes, in order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// The declarations of node `i`'s occupant.
    pub fn declaration(&self, i: usize) -> &Declaration {
        &self.declarations[i]
    }

    fn position(&self, kind: Kind) -> usize {
        self.nodes.iter().position(|n| n.kind == kind).unwrap_or(0)
    }

    /// Whether each node is the identity: its occupant None, or a field input absent or fed by an identity source.
    fn identity(&self) -> Vec<bool> {
        let mut out: Vec<bool> = Vec::with_capacity(self.nodes.len());
        for n in &self.nodes {
            let ports = in_ports(n.kind, &self.declarations[out.len()]);
            let dangling = ports
                .iter()
                .zip(&n.inputs)
                .any(|(&p, i)| p == PortType::Field && i.is_none_or(|j| out[j]));
            out.push(n.occupant == Occupant::None || dangling);
        }
        out
    }

    /// The live post chain, from the combiner to `OUT`: walking back from `OUT` through each colour input, an identity
    /// post passed over, to the combiner or an absent input.
    fn chain(&self) -> Vec<usize> {
        let identity = self.identity();
        let mut chain = Vec::new();
        let mut at = self.nodes[self.position(Kind::Out)]
            .inputs
            .first()
            .copied()
            .flatten();
        while let Some(i) = at.filter(|&i| self.nodes[i].kind == Kind::Post) {
            if !identity[i] {
                chain.push(i);
            }
            at = self.nodes[i].inputs[0];
        }
        chain.reverse();
        chain
    }

    /// The combiner's colour and brightness nodes, each `None` when absent or the identity.
    fn slots(&self) -> (Option<usize>, Option<usize>) {
        let identity = self.identity();
        let c = &self.nodes[self.position(Kind::Combiner)];
        let live = |k: usize| c.inputs[k].filter(|&i| !identity[i]);
        (live(0), live(1))
    }

    /// The sources live nodes read, in order of first use: the colour's field inputs, the brightness's, then each
    /// post's in chain order.
    fn sources(&self) -> Vec<usize> {
        let (colour, brightness) = self.slots();
        let mut out = Vec::new();
        for i in colour.into_iter().chain(brightness).chain(self.chain()) {
            for j in self.field_inputs(i).into_iter().flatten() {
                if !out.contains(&j) {
                    out.push(j);
                }
            }
        }
        out
    }

    /// Node `i`'s field inputs, in port order.
    fn field_inputs(&self, i: usize) -> Vec<Option<usize>> {
        let skip = usize::from(self.nodes[i].kind == Kind::Post);
        self.nodes[i].inputs[skip..].to_vec()
    }

    /// The canonical form (lowering contract Part 5; render contract Part 3; REQ-RENDER-075): the live nodes, those
    /// `OUT` depends on with the identity dropped, in canonical order — the sources in order of first use, then the
    /// colour, the brightness, the combiner, the posts from the combiner to `OUT`, and `OUT` — each wired by position:
    /// the combiner to its live colour and brightness, a post and `OUT` to the node before it in the chain, and each
    /// field input to its source. The order the nodes were given in is not in it.
    pub fn canonical(&self) -> Canonical {
        let (colour, brightness) = self.slots();
        let order: Vec<usize> = self
            .sources()
            .into_iter()
            .chain(colour)
            .chain(brightness)
            .chain([self.position(Kind::Combiner)])
            .chain(self.chain())
            .chain([self.position(Kind::Out)])
            .collect();
        let at = |i: usize| order.iter().position(|&o| o == i);
        let mut nodes: Vec<Node> = Vec::with_capacity(order.len());
        for &i in &order {
            let n = &self.nodes[i];
            let inputs = match n.kind {
                // A post and OUT read the node before them in the chain, an identity post passed over and an
                // absent input read as the combiner; every other input its live node, an identity one absent.
                Kind::Out => vec![nodes.len().checked_sub(1)],
                Kind::Post => [nodes.len().checked_sub(1)]
                    .into_iter()
                    .chain(self.field_inputs(i).into_iter().map(|j| j.and_then(at)))
                    .collect(),
                _ => n.inputs.iter().map(|j| j.and_then(at)).collect(),
            };
            nodes.push(Node {
                kind: n.kind,
                occupant: n.occupant.clone(),
                inputs,
            });
        }
        let declarations = order
            .iter()
            .map(|&i| self.declarations[i].clone())
            .collect();
        Canonical {
            order,
            stain: Stain {
                nodes,
                declarations,
            },
        }
    }
}

/// A stain's canonical form ([`Stain::canonical`]): its live nodes in canonical order, as a stain.
#[derive(Clone, Debug, PartialEq)]
pub struct Canonical {
    order: Vec<usize>,
    stain: Stain,
}

impl Canonical {
    /// Each canonical node's position in the stain it was taken from.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// The form as a stain: its nodes in canonical order.
    pub fn stain(&self) -> &Stain {
        &self.stain
    }

    /// The form's text (lowering contract Part 5): `{"nodes":[…]}`, each node `{"inputs":[…],"kind":…,"occupant":…}`,
    /// its occupant `null`, `{"field":…}`, `{"builtin":…}` or `{"custom":…}`, in JCS (RFC 8785: keys sorted, no white
    /// space, strings escaped as ECMAScript's `JSON.stringify` escapes them). The fragment key's input.
    pub fn text(&self) -> String {
        let mut out = String::from("{\"nodes\":[");
        for (k, n) in self.stain.nodes.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            out.push_str("{\"inputs\":[");
            for (m, input) in n.inputs.iter().enumerate() {
                if m > 0 {
                    out.push(',');
                }
                match input {
                    Some(j) => {
                        let _ = write!(out, "{j}");
                    }
                    None => out.push_str("null"),
                }
            }
            out.push_str("],\"kind\":");
            json_string(n.kind.name(), &mut out);
            out.push_str(",\"occupant\":");
            let (key, value) = match &n.occupant {
                Occupant::None => {
                    out.push_str("null}");
                    continue;
                }
                Occupant::Field(f) => ("field", f),
                Occupant::BuiltIn(id) => ("builtin", id),
                Occupant::Custom(text) => ("custom", text),
            };
            out.push('{');
            json_string(key, &mut out);
            out.push(':');
            json_string(value, &mut out);
            out.push_str("}}");
        }
        out.push_str("]}");
        out
    }

    /// The fragment key (lowering contract Part 5): the 64-bit FNV-1a hash of [`Canonical::text`]'s UTF-8 bytes.
    pub fn fragment_key(&self) -> u64 {
        ledger::version::fnv1a64(self.text().as_bytes())
    }
}

/// `s` as a JSON string, escaped as JCS escapes it (RFC 8785 §3.2.2.2): `"` and `\` escaped, the control characters
/// as `\b`, `\t`, `\n`, `\f`, `\r` or `\u00xx`, every other character as itself.
fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ── Assembly ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// One node's uniform block: `n<i>_Uniforms`, bound at `@group(group) @binding(binding)` as `n<i>_uniforms`, holding
/// `uniforms` in declaration order.
#[derive(Clone, Debug, PartialEq)]
pub struct UniformBlock {
    pub node: usize,
    pub group: u32,
    pub binding: u32,
    pub uniforms: Vec<Uniform>,
}

/// An assembled stain: its WGSL, the read-side fields its `sample_read` fills, and its nodes' uniform blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    pub source: String,
    pub fields: Vec<String>,
    pub uniforms: Vec<UniformBlock>,
}

/// The one assembler entry point (render contract Part 2): `stain` at `tier`, its read side filling the fields its IR
/// reads ([`field_set`]), refused where any step fails ([`assemble_reading`]).
pub fn assemble(stain: &Stain, tier: Tier) -> Result<Fragment, AssembleError> {
    let fields = field_set(stain, tier)?;
    let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
    assemble_reading(stain, tier, &fields)
}

/// The stain's field set (R-378): every member of the read-side `SimState` its live nodes read, by naga's IR of the
/// stain assembled with every field filled, the word as `word` or as only the components it reads ([`fields_read`]).
pub fn field_set(stain: &Stain, tier: Tier) -> Result<Vec<String>, AssembleError> {
    let (words, entries) = ledger()?;
    let every: Vec<String> = read::members(words, entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    let (source, _) = source(stain, tier, &every)?;
    let (module, info) = compile(&source)?;
    Ok(fields_read(&module, &info).into_iter().collect())
}

/// [`assemble`]'s second step: `stain` at `tier` with a read side whose `sample_read` fills `fields`, compiled;
/// refused when the compiled stain reads a field `fields` misses,
/// which `sample_read` would leave reading 0 (R-378).
pub fn assemble_reading(
    stain: &Stain,
    tier: Tier,
    fields: &[&str],
) -> Result<Fragment, AssembleError> {
    let (source, uniforms) = source(stain, tier, fields)?;
    let (module, info) = compile(&source)?;
    let filled = |f: &str| {
        fields.contains(&f)
            || f.strip_prefix("word.").is_some() && fields.contains(&"word")
            || f == "word"
                && read::WORD_COMPONENTS
                    .iter()
                    .all(|c| fields.contains(&&*format!("word.{c}")))
    };
    if let Some(missing) = fields_read(&module, &info).into_iter().find(|f| !filled(f)) {
        return Err(AssembleError::UnfilledField(missing));
    }
    let mut fields: Vec<String> = fields.iter().map(|&f| f.to_owned()).collect();
    fields.sort();
    fields.dedup();
    Ok(Fragment {
        source,
        fields,
        uniforms,
    })
}

/// The ledger's words and its validated entries.
type Ledger = (Vec<Word>, Vec<Entry>);

/// The ledger's words and validated entries, the read side's inputs.
fn ledger() -> Result<(&'static [Word], &'static [Entry]), AssembleError> {
    static LEDGER: OnceLock<Result<Ledger, String>> = OnceLock::new();
    let got = LEDGER.get_or_init(|| {
        let l = ledger::payload::ledger();
        generate::validate(&l)
            .map(|entries| (l.words, entries))
            .map_err(|e| e.to_string())
    });
    match got {
        Ok((w, e)) => Ok((w.as_slice(), e.as_slice())),
        Err(why) => Err(AssembleError::Ledger(why.clone())),
    }
}

/// The stain's context (render contract Part 1's `RenderContext`, its lanes as M1 fills them; R-369).
const CONTEXT: &str = r"
// A field on a wire (render_gui_spec Part II §3): a scalar or categorical value in `.x`, a vector in `.xyz`.
alias Field = vec4<f32>;

// What every node reads (render contract Part 1): the sample's read-side `SimState`, the pixel's position for the
// invalid hatch (`debug_invalid`), and the node's wired fields, `inputs[k]` its k-th field input.
struct Ctx {
    sample: SimState,
    frag_xy: vec2<f32>,
    inputs: array<Field, INPUTS>,
}
";

/// `stain`'s WGSL at `tier`, its read side filling `fields`, and its uniform blocks.
fn source(
    stain: &Stain,
    tier: Tier,
    fields: &[&str],
) -> Result<(String, Vec<UniformBlock>), AssembleError> {
    let (words, entries) = ledger()?;
    let read_side = read::assemble(words, entries, tier, fields).map_err(AssembleError::Field)?;
    let mut out = format!(
        "{}\n{COLOUR_SPACE}\n{PRESENT}\n{read_side}\n// ── The stain (TASK-M1-04): context, node functions, shade() ──\n{}",
        prelude::wgsl(tier),
        CONTEXT.replace("INPUTS", &MAX_INPUTS.to_string())
    );
    // The canonical form's stain: its live nodes only, numbered in canonical order, so two wirings of one graph
    // assemble to one source.
    let canonical = stain.canonical();
    let stain = canonical.stain();
    let (colour, brightness) = stain.slots();
    let chain = stain.chain();
    let sources = stain.sources();
    let combiner = stain.position(Kind::Combiner);
    let live: Vec<usize> = sources
        .iter()
        .copied()
        .chain(colour)
        .chain(brightness)
        .chain([combiner])
        .chain(chain.iter().copied())
        .collect();
    let first = prelude::uniforms_binding();
    let mut uniforms = Vec::new();
    for &i in &live {
        let n = &stain.nodes[i];
        let text = occupant_source(n.kind, &n.occupant)?.unwrap_or_default();
        let schema = &stain.declarations[i].uniforms;
        let mut extra = Vec::new();
        if !schema.is_empty() {
            let binding = first.binding + 1 + uniforms.len() as u32;
            let _ = writeln!(
                out,
                "\n// Node {i}'s uniforms (its `uniformSchema`, gui_state_contract §3)."
            );
            let _ = writeln!(out, "struct n{i}_Uniforms {{");
            for u in schema {
                let _ = writeln!(out, "    {}: {},", u.name, u.ty.wgsl());
            }
            let _ = writeln!(
                out,
                "}}\n@group({}) @binding({binding}) var<uniform> n{i}_uniforms: n{i}_Uniforms;",
                first.group
            );
            uniforms.push(UniformBlock {
                node: i,
                group: first.group,
                binding,
                uniforms: schema.clone(),
            });
            extra.push("uniforms");
        }
        let _ = writeln!(
            out,
            "\n// Node {i}: {}, {}.",
            n.kind.name(),
            label(&n.occupant)
        );
        out.push_str(&prefixed(&text, &format!("n{i}_"), n.kind, &extra)?);
    }
    out.push_str(&shade(
        stain, &sources, colour, brightness, combiner, &chain,
    ));
    Ok((out, uniforms))
}

fn label(occupant: &Occupant) -> String {
    match occupant {
        Occupant::None => "None".into(),
        Occupant::Field(f) => format!("the built-in source of `{f}`"),
        Occupant::BuiltIn(id) => format!("built-in `{id}`"),
        Occupant::Custom(_) => "custom".into(),
    }
}

/// `shade(ctx)`, which walks the graph in backbone order: the sources, the colour and the brightness, the combiner,
/// the post chain, `OUT`; and `shade_sample`, which reads sample `i` into a context and shades it.
fn shade(
    stain: &Stain,
    sources: &[usize],
    colour: Option<usize>,
    brightness: Option<usize>,
    combiner: usize,
    chain: &[usize],
) -> String {
    let mut out = String::from(
        "\n// The generated walk of the graph, in backbone order (render contract Part 2): sources → colour / brightness \
         → combiner → (post)* → OUT.\nfn shade(ctx: Ctx) -> vec3<f32> {\n",
    );
    for &s in sources {
        let _ = writeln!(out, "    let f{s} = n{s}_source(ctx);");
    }
    let context = |i: usize, out: &mut String| {
        let _ = writeln!(out, "    var c{i} = ctx;");
        for (k, j) in stain.field_inputs(i).into_iter().enumerate() {
            let _ = writeln!(out, "    c{i}.inputs[{k}] = f{};", j.unwrap_or_default());
        }
    };
    if let Some(c) = colour {
        context(c, &mut out);
        let _ = writeln!(out, "    let rgb = n{c}_colour(c{c});");
    }
    if let Some(b) = brightness {
        context(b, &mut out);
        let _ = writeln!(out, "    let b = n{b}_brightness(c{b});");
    }
    let combine = match (colour, brightness) {
        (Some(_), Some(_)) => format!("n{combiner}_combine(rgb, b);"),
        (Some(_), None) => "rgb; // brightness None: the colour as it is".into(),
        (None, Some(_)) => format!(
            "n{combiner}_combine(vec3<f32>(1.0), b); // colour None: white, so the greyscale of the brightness"
        ),
        (None, None) => format!("ramp_grey({MID_GREY_L}); // both None: the flat mid-grey OKLab (0.6, 0, 0)"),
    };
    let _ = writeln!(out, "    var out = {combine}");
    for &p in chain {
        context(p, &mut out);
        let _ = writeln!(out, "    out = n{p}_post(c{p}, out);");
    }
    out.push_str(
        "    return out; // OUT\n}\n\n\
         // Sample `i` read through the generated read side into a context at pixel `frag_xy`, and shaded. The read\n\
         // side's arguments are its own (`ledger::gen::read`); `has_ensemble` is the prelude's uniform (R-145).\n\
         fn shade_sample(i: u32, frag_xy: vec2<f32>, ensemble_spread: f32, masses: vec3<f32>, params: ReadParams) -> vec3<f32> {\n    \
         var ctx: Ctx;\n    \
         ctx.sample = sample_read(i, ensemble_spread, has_ensemble(), masses, params);\n    \
         ctx.frag_xy = frag_xy;\n    \
         return shade(ctx);\n}\n",
    );
    out
}

// ── Node functions: one per node, its names prefixed ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tok {
    Ident,
    Number,
    Comment,
    Space,
    Punct,
}

/// `s` as tokens, each its kind and byte range: comments (`//` to the line's end, `/* */` nested), identifiers,
/// numbers (a digit and the letters and digits after it), white space and single punctuation characters.
fn lex(s: &str) -> Result<Vec<(Tok, usize, usize)>, AssembleError> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let c = b[i];
        // The comment branches compare bytes, never slice `s`: inside a comment `i` steps a byte at a time, through
        // the middle of a multi-byte character, and a `/`, `*` or newline byte is never part of one (UTF-8).
        let tok = if b[i..].starts_with(b"//") {
            i = b[i..]
                .iter()
                .position(|&x| x == b'\n')
                .map_or(b.len(), |k| i + k);
            Tok::Comment
        } else if b[i..].starts_with(b"/*") {
            let mut depth = 0;
            loop {
                if b[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if b[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else if i >= b.len() {
                    return Err(AssembleError::Compile("an unclosed `/*` comment".into()));
                } else {
                    i += 1;
                }
            }
            Tok::Comment
        } else if c.is_ascii_whitespace() {
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            Tok::Space
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            Tok::Ident
        } else if c.is_ascii_digit() {
            // A number's letters — its suffix, hex digits and exponent letter — are its own, never a name; a `.`, an
            // exponent's sign and the digits after them lex as punctuation and numbers, which nothing renames.
            while i < b.len() && b[i].is_ascii_alphanumeric() {
                i += 1;
            }
            Tok::Number
        } else {
            i += s[i..].chars().next().map_or(1, char::len_utf8);
            Tok::Punct
        };
        out.push((tok, start, i));
    }
    Ok(out)
}

/// `text`, a node's WGSL, with each name it declares at module scope, and each of `extra`, prefixed with `prefix`
/// wherever it is used: not after a `.`, and not a member of a struct it declares. It must define its slot's function;
/// it may not declare a module-scope `var` or `override` or an entry point, whose bindings and stages are the
/// assembler's, nor name the stored buffers or `sample_read` (R-343, R-378).
fn prefixed(text: &str, prefix: &str, kind: Kind, extra: &[&str]) -> Result<String, AssembleError> {
    let toks = lex(text)?;
    let sig: Vec<usize> = (0..toks.len())
        .filter(|&k| !matches!(toks[k].0, Tok::Comment | Tok::Space))
        .collect();
    let at = |k: usize| &text[toks[sig[k]].1..toks[sig[k]].2];
    let refuse = |why: String| Err(AssembleError::Occupant(why));
    let mut declared: Vec<&str> = extra.to_vec();
    let mut members = BTreeSet::new();
    // The brace depth, and whether the braces open are a struct's body, which holds no braces of its own.
    let (mut depth, mut in_struct, mut pending_struct) = (0usize, false, false);
    for k in 0..sig.len() {
        let word = at(k);
        let prev = if k > 0 { at(k - 1) } else { "" };
        match word {
            "{" => {
                depth += 1;
                in_struct = pending_struct;
                pending_struct = false;
            }
            "}" => {
                in_struct = false;
                depth = depth.saturating_sub(1);
            }
            ":" if in_struct => {
                members.insert(sig[k - 1]);
            }
            "fn" | "const" | "struct" | "alias" if depth == 0 => {
                let name = sig
                    .get(k + 1)
                    .filter(|&&t| toks[t].0 == Tok::Ident)
                    .map(|&t| &text[toks[t].1..toks[t].2]);
                let Some(name) = name else {
                    return refuse(format!("`{word}` with no name"));
                };
                if extra.contains(&name) {
                    return refuse(format!(
                        "`{name}` is the assembler's, declared from the uniformSchema"
                    ));
                }
                declared.push(name);
                pending_struct = word == "struct";
            }
            "var" | "override" if depth == 0 => {
                return refuse(format!(
                    "a node declares no module-scope `{word}`: its uniforms come from its uniformSchema"
                ));
            }
            "fragment" | "vertex" | "compute" | "group" | "binding" if prev == "@" => {
                return refuse(format!(
                    "a node declares no `@{word}`: stages and bindings are the assembler's"
                ));
            }
            w if RESERVED.contains(&w) && toks[sig[k]].0 == Tok::Ident => {
                return refuse(format!(
                    "a node does not name `{w}`: it reads the sample as `ctx.sample` (R-343, R-378)"
                ));
            }
            _ => {}
        }
    }
    if let Some(slot) = kind.slot() {
        let Some(k) = (1..sig.len()).find(|&k| at(k) == slot && at(k - 1) == "fn") else {
            return refuse(format!("a {} occupant defines `fn {slot}`", kind.name()));
        };
        let words: Vec<&str> = (k + 1..sig.len())
            .map(at)
            .take_while(|&w| w != "{")
            .collect();
        let (params, result) = kind.signature();
        let want = format!("fn({}) -> {result}", params.join(", "));
        let got = header(&words);
        if got != want {
            return Err(AssembleError::Slot(format!(
                "this {} occupant's `{slot}` is `{got}`; a {} slot is `{want}` (render contract Part 2)",
                kind.name(),
                kind.name()
            )));
        }
    }
    let mut out = String::with_capacity(text.len() + 64);
    let mut previous = "";
    for (k, &(tok, a, b)) in toks.iter().enumerate() {
        let word = &text[a..b];
        if tok == Tok::Ident && previous != "." && !members.contains(&k) && declared.contains(&word)
        {
            out.push_str(prefix);
        }
        out.push_str(word);
        if !matches!(tok, Tok::Comment | Tok::Space) {
            previous = word;
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// A function header's tokens after its name, `( <name>: <type>, … ) -> <type>`, as `fn(<type>, …) -> <type>`, each
/// type with its white space removed and WGSL's predeclared aliases `vec3f` and `vec4f` written out, as is the
/// assembler's `Field`.
fn header(words: &[&str]) -> String {
    let ty = |t: &[&str]| match t.concat().as_str() {
        "vec3f" => "vec3<f32>".to_owned(),
        "vec4f" | "Field" => "vec4<f32>".to_owned(),
        other => other.to_owned(),
    };
    let close = words.iter().position(|&w| w == ")").unwrap_or(words.len());
    let inner = words.get(1..close).unwrap_or_default();
    let params: Vec<String> = inner
        .split(|&w| w == ",")
        .filter(|p| !p.is_empty())
        .map(|p| ty(p.iter().position(|&w| w == ":").map_or(p, |c| &p[c + 1..])))
        .collect();
    let result = words.get(close + 1..).unwrap_or_default();
    let result = match result {
        ["-", ">", rest @ ..] => ty(rest),
        _ => String::new(),
    };
    format!("fn({}) -> {result}", params.join(", "))
}

// ── Compilation and the field set ─────────────────────────────────────────────────────────────────────────────────

/// `source` parsed and validated by naga with naga's default capabilities and `SHADER_FLOAT16_IN_FLOAT32`, which the
/// unpack layer's `unpack2x16float` needs (core WGSL; render contract Part 5); an error names the line.
fn compile(source: &str) -> Result<(Module, ModuleInfo), AssembleError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| AssembleError::Compile(e.emit_to_string(source)))?;
    let mut capabilities = Capabilities::default();
    capabilities.insert(Capabilities::SHADER_FLOAT16_IN_FLOAT32);
    let info = Validator::new(ValidationFlags::all(), capabilities)
        .validate(&module)
        .map_err(|e| AssembleError::Compile(e.emit_to_string(source)))?;
    Ok((module, info))
}

/// Each `Call` in `block`, nested blocks included: the callee and its arguments.
fn calls(block: &Block, out: &mut Vec<(Handle<Function>, Vec<Handle<Expression>>)>) {
    for s in block.iter() {
        match s {
            Statement::Call {
                function,
                arguments,
                ..
            } => out.push((*function, arguments.clone())),
            Statement::Block(b) => calls(b, out),
            Statement::If { accept, reject, .. } => {
                calls(accept, out);
                calls(reject, out);
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                calls(body, out);
                calls(continuing, out);
            }
            Statement::Switch { cases, .. } => cases.iter().for_each(|c| calls(&c.body, out)),
            _ => {}
        }
    }
}

/// The components of the `vec4<u32>` value `e` in `f` that are read: each `.x` … `.w` taken of it, and through each
/// function it is passed to, that function's reads of its argument; all four if it is used any other way.
fn components(
    module: &Module,
    info: &ModuleInfo,
    f: &Function,
    fn_info: &FunctionInfo,
    e: Handle<Expression>,
) -> [bool; 4] {
    let mut out = [false; 4];
    let mut uses = 0;
    for (_, x) in f.expressions.iter() {
        if let Expression::AccessIndex { base, index } = *x {
            if base == e {
                out[index as usize] = true;
                uses += 1;
            }
        }
    }
    let mut cs = Vec::new();
    calls(&f.body, &mut cs);
    for (callee, args) in cs {
        for (k, &a) in args.iter().enumerate() {
            if a != e {
                continue;
            }
            uses += 1;
            let g = &module.functions[callee];
            let arg = g
                .expressions
                .iter()
                .find(|(_, x)| matches!(x, Expression::FunctionArgument(j) if *j as usize == k))
                .map(|(h, _)| h);
            let inner = arg.map_or([true; 4], |h| components(module, info, g, &info[callee], h));
            for (o, i) in out.iter_mut().zip(inner) {
                *o |= i;
            }
        }
    }
    if fn_info[e].ref_count > uses {
        return [true; 4];
    }
    out
}

/// The read-side fields the stain in `module` reads, by naga's IR: each member of the read-side `SimState` taken in
/// any function but `sample_read`, which fills it; the word as `word`, or as `word.x` … `word.w` where only those
/// components are read ([`components`]). These are the fields its `sample_read` must fill (R-378).
pub fn fields_read(module: &Module, info: &ModuleInfo) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(simstate) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("SimState"))
        .map(|(h, _)| h)
    else {
        return out;
    };
    let TypeInner::Struct { members, .. } = &module.types[simstate].inner else {
        return out;
    };
    let mut read = |f: &Function, fn_info: &FunctionInfo| {
        for (h, x) in f.expressions.iter() {
            let Expression::AccessIndex { base, index } = *x else {
                continue;
            };
            let on_simstate = match *fn_info[base].ty.inner_with(&module.types) {
                TypeInner::Struct { .. } => fn_info[base].ty.handle() == Some(simstate),
                TypeInner::Pointer { base, .. } => base == simstate,
                _ => false,
            };
            if !on_simstate {
                continue;
            }
            let name = members[index as usize].name.clone().unwrap_or_default();
            if name != "word" {
                out.insert(name);
                continue;
            }
            let parts = components(module, info, f, fn_info, h);
            if parts.iter().all(|&p| p) {
                out.insert(name);
            } else {
                for (c, _) in parts.iter().enumerate().filter(|(_, &p)| p) {
                    out.insert(format!("word.{}", read::WORD_COMPONENTS[c]));
                }
            }
        }
    };
    for (h, f) in module.functions.iter() {
        if f.name.as_deref() != Some("sample_read") {
            read(f, &info[h]);
        }
    }
    for (k, ep) in module.entry_points.iter().enumerate() {
        read(&ep.function, info.get_entry_point(k));
    }
    if out.contains("word") {
        out.retain(|f| !f.starts_with("word."));
    }
    out
}

// ── Errors ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Why a stain was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssembleError {
    /// A wire whose out-port type is not its in-port's, or a node whose inputs are not its in-ports.
    Wire(String),
    /// A wire that breaks the backbone.
    Backbone(String),
    /// A wire that runs backward, which a cycle needs.
    Cycle(String),
    /// Other than one combiner and one `OUT`.
    Singleton(String),
    /// A post chain longer than [`MAX_POSTS`].
    PostChain(String),
    /// An occupant its slot does not take, or WGSL a node may not hold.
    Occupant(String),
    /// A malformed declaration line (1-based).
    Declaration { line: usize, why: String },
    /// A slot function with the wrong signature.
    Slot(String),
    /// The ledger did not validate.
    Ledger(String),
    /// A field name the read side does not have.
    Field(String),
    /// naga refused the assembled WGSL.
    Compile(String),
    /// The stain reads a field its read side does not fill (R-378).
    UnfilledField(String),
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssembleError::Declaration { line, why } => write!(f, "declaration, line {line}: {why}"),
            AssembleError::UnfilledField(field) => write!(
                f,
                "the stain reads `{field}`, which its field set misses: `sample_read` would leave it reading 0 (R-378)"
            ),
            AssembleError::Wire(s)
            | AssembleError::Backbone(s)
            | AssembleError::Cycle(s)
            | AssembleError::Singleton(s)
            | AssembleError::PostChain(s)
            | AssembleError::Occupant(s)
            | AssembleError::Slot(s)
            | AssembleError::Ledger(s)
            | AssembleError::Field(s)
            | AssembleError::Compile(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for AssembleError {}
