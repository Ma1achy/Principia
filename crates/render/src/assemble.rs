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
//! file's header ([`Declaration`]; gui_state_contract §3). It reads the pixel's post-flip UV as `ctx.screen.uv` and its
//! sample's quad-local coordinate and quad as `ctx.quad.uv`, `ctx.quad.centre` and `ctx.quad.half_width`
//! (colour_composition §3's lanes; deep_zoom §1's c and h), which `shade_at`'s caller fills from the raster and the
//! per-quad frames (TASK-M1-07); its quad's `RenderQuad` metadata as the rest of `ctx.quad` (`ctx.quad.depth`,
//! `ctx.quad.state`, …), its within-tile coordinate as `ctx.tile.uv` (TASK-M1-13; RQ-238), and its sample's chart
//! classification as `ctx.validity.forbidden`, which `shade_at`'s caller fills from the chart, CPU-side (REQ-COL-102;
//! TASK-M2-32).
//!
//! **Identity** (render_gui_spec §13; colour_composition §4.1): a node whose occupant is None, or one of whose field
//! inputs is absent, is the identity. A colour None gives the combiner white, so the combiner gives the greyscale of
//! the brightness; a brightness None keeps the colour as it is; both None give the flat mid-grey OKLab (0.6, 0, 0); a
//! post None passes its colour on. A post's or `OUT`'s absent colour input reads the combiner: the chain before it is
//! empty. So every graph renders.
//!
//! **The fallback** (render_gui_spec §13; REQ-RENDER-072). A node list that is no graph ([`Stain::new`] refuses it)
//! still renders: [`assemble_or_fallback`] gives it the defined fallback, the same template with no node function and
//! a `shade()` that returns one flat error tint ([`codegen::fallback`]), and the reason it was refused. The walk is
//! the one walk: the fallback is its `shade()` with no node to visit, never a second path (REQ-RENDER-010; R-814).
//!
//! **The read side (R-378).** The assembler derives the stain's field set from its IR ([`field_set`]), every member
//! of the read-side `SimState` the live nodes read, the word as `word` or as only the components `word.x` … `word.w`
//! it reads, and generates the unpack layer and read side at its tier for that set through
//! `ledger::gen::read::assemble`, never the checked-in full-fill `read_side.wgsl`. A field `sample_read` does not
//! fill reads 0, a plausible value with no validity signal, so [`assemble_reading`] refuses a stain that reads a field
//! its set misses. Nothing a node writes may name the stored buffers or `sample_read` (R-343).
//!
//! **Two variants (R-41's baked variants; R-467 D5, R-470 G4; REQ-COL-108, REQ-COL-109).** A stain assembles as a
//! presentation view, the default ([`assemble`]), or as View › Debug view ([`assemble_view`] with [`View::Debug`]), the
//! render's own debug-view flag, which its caller sets from `RenderState`'s `debug_view`. The debug variant adds one arm
//! after the walk ([`DEBUG_ARM`]): the hatch for a pixel the chart classifies as not a system, the NaN / Inf style for a
//! non-finite sample, `debug_invalid` for any other failure, every system pixel as the stain draws it, each counted
//! into [`DEBUG_COUNTS_GROUP`]'s storage buffer for the legend's "Debug" key. The presentation variant has no arm and no
//! flag bit: its `note_non_finite()`, the hook a node calls where it reads a non-finite value, does nothing.

use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Write as _};
use std::sync::{Mutex, OnceLock, PoisonError};

use crate::codegen;
use crate::lex::{lines, tokens, Tok};
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

/// The compactions and ramps of the field ramp (dd_colouring §3.6; colour_composition §1.2), hand-written; they follow
/// the prelude at assembly, so every node may call them (TASK-M7-05).
const COMPACTION: &str = crate::colour::field_ramp::COMPACTION_WGSL;
const RAMP: &str = crate::colour::field_ramp::RAMP_WGSL;

/// The drift views' presentation, `dbg_drift` and `dbg_suspect` (render contract Part 5; TASK-M3-05), hand-written;
/// it calls the compactions and ramps, so it follows them at assembly.
pub const DRIFT: &str = include_str!("../shaders/wgsl/lib/drift.wgsl");

/// The categorical palettes and the categorical filter (colour_composition §1.4; dd_colouring §3.7), hand-written;
/// they follow the presentation layer at assembly, whose Okabe–Ito table they read, so every node may call them
/// (TASK-M7-08).
const PALETTE: &str = crate::colour::palette::WGSL;

/// The post chain's bound (colour_composition §4.2: "Bounded at ≤ 8").
pub const MAX_POSTS: usize = 8;

/// The most field inputs a node takes, the length of `ctx.inputs` (applied per R-369: the corpus gives no bound).
pub const MAX_INPUTS: usize = 4;

/// Names a node may not write: the stored buffers and the read side's readers of them (R-343, R-378), `ic_read` and
/// each `ic_read_<member>` too (RQ-227), and the debug view's counts and its note (REQ-COL-109), which only its arm
/// writes.
const RESERVED: [&str; 6] = [
    "simstate_buffer",
    "word_buffer",
    "sample_read",
    "ic_buffer",
    "debug_counts",
    "debug_non_finite",
];

/// View › Debug view's arm (R-467 D5, R-470 G4; REQ-COL-109), the debug variant's one addition:
/// `debug_view(forbidden, sample, frag_xy, rgb)` over the stain's colour, `note_non_finite()`, and the counts' storage buffer at [`DEBUG_COUNTS_GROUP`]. Its CPU
/// twin is [`crate::debug_view`].
pub const DEBUG_ARM: &str = include_str!("../shaders/wgsl/stain/debug_view.wgsl");

/// The presentation variant's `note_non_finite()`: nothing (R-470 G4: no debug arm).
const PRESENTATION_HOOK: &str = "
// The presentation variant notes nothing: it has no debug arm (R-470 G4), and draws a non-finite value as the stain's
// invalid treatment does, `#141418` by default (R-467).
fn note_non_finite() {}
";

/// The debug variant's counts' bind group and binding (REQ-COL-109): `array<atomic<u32>, 3>`, read-write storage, the
/// pixels drawn in "forbidden (chart)", "decode or sim failure" and "NaN / Inf", in that order. Group 3, past the
/// harness's three (applied per R-369: the corpus gives the counts no binding).
pub const DEBUG_COUNTS_GROUP: u32 = 3;
pub const DEBUG_COUNTS_BINDING: u32 = 0;

/// Which variant a stain assembles to (R-41; R-467 D5, R-470 G4): a presentation view, or View › Debug view, the
/// render's debug-view flag, which its caller sets from `RenderState`'s `debug_view`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum View {
    /// A presentation view: no debug arm (REQ-COL-108).
    #[default]
    Presentation,
    /// View › Debug view: the arm [`DEBUG_ARM`] over the stain (REQ-COL-109).
    Debug,
}

/// The read side's `ICDescriptor` reader, and the prefix of its per-member readers (RQ-227).
const IC_READ: &str = "ic_read";

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
    /// A shipped source, by its name in its slot (`registry::Entry::name`): `pass_through`, `debug/generated/state`.
    BuiltIn(String),
    /// A custom occupant's WGSL, as written.
    Custom(String),
}

// ── The declaration format (gui_state_contract §3; REQ-GEN-027) ──────────────────────────────────────────────────

pub mod declare;
pub mod matters;
pub mod params;

pub use declare::{code_text, key_text, Check, Control, ControlLine, Described, Glyph};

/// A uniform's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UniformType {
    F32,
    I32,
    U32,
    Vec2,
    Vec3,
    Vec4,
    /// `gradient<N>`: two to `N` stops, positions and colours ([`params`]).
    Gradient(u8),
    /// `list<T, N>`: up to `N` items of `width` numbers each.
    List {
        width: u8,
        max: u8,
    },
    /// `lut<N>`: `N` numbers sampled over `[0, 1]`.
    Lut(u16),
    /// `field`: a read-side field the node's code reads, chosen by the stain.
    Field,
    /// `image`: an image's handle.
    Image,
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
    pub fn wgsl(self) -> String {
        if let Some(name) = params::struct_name(self) {
            return name;
        }
        self.scalar_wgsl().to_owned()
    }

    const fn scalar_wgsl(self) -> &'static str {
        match self {
            UniformType::F32 => "f32",
            UniformType::I32 => "i32",
            UniformType::U32 => "u32",
            UniformType::Vec2 => "vec2<f32>",
            UniformType::Vec3 => "vec3<f32>",
            UniformType::Vec4 => "vec4<f32>",
            UniformType::Image | UniformType::Field => "u32",
            UniformType::Gradient(_) | UniformType::List { .. } | UniformType::Lut(_) => "",
        }
    }

    /// Its number of components; a new type's value has no fixed number, so 0.
    pub const fn components(self) -> usize {
        match self {
            UniformType::Gradient(_)
            | UniformType::List { .. }
            | UniformType::Lut(_)
            | UniformType::Field
            | UniformType::Image => 0,
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
    /// A `field` that may be swapped for a field of the other kind, a vector for a scalar: its code reads it with
    /// `as_field`, which fits either. Written `= <field> any`.
    pub any_kind: bool,
    /// A `field` chosen among every scalar field a ramp reads ([`ramp_fields`]), with its validity, not among the fields
    /// a source reads. Written `= <field> scalar`.
    // The plan is unclear here; chose the trailing words `scalar` and `baked`, as `any` is, with no version bump,
    // because a source that never writes them reads as before, and a version would refuse the shipped files on an
    // older build for a change they do not need.
    pub scalars: bool,
    /// Written into the code where the node is made, never bound: a `field` always is, and any other uniform written
    /// `= <default> baked`. Its value reaches the code only through the words of [`params::specialise`], so choosing
    /// another is another code.
    pub baked: bool,
}

impl Uniform {
    /// Whether the uniform is written into the code rather than bound in the node's block.
    pub fn is_baked(&self) -> bool {
        self.baked || self.ty == UniformType::Field
    }

    /// The fields a `field` uniform chooses among, by their place in its value.
    pub fn field_names(&self) -> Result<Vec<String>, AssembleError> {
        if self.scalars {
            ramp_fields()
        } else {
            source_fields()
        }
    }

    /// Whether `value` is a value of this uniform: one number per component, each of the type, within the range.
    pub fn admits(&self, value: &[f64]) -> bool {
        if self.scalars {
            return value.len() == 1
                && value[0].fract() == 0.0
                && value[0] >= 0.0
                && ramp_fields().is_ok_and(|f| (value[0] as usize) < f.len());
        }
        if let Some(ok) = params::admits(self.ty, value) {
            // A field is swapped for one of its default's kind, a vector for a vector and a scalar for a scalar: the
            // code that reads it does not change with it.
            return ok
                && (self.ty != UniformType::Field
                    || self.any_kind
                    || self
                        .default
                        .first()
                        .is_some_and(|&d| params::same_field_kind(d, value[0])));
        }
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

/// An occupant's declarations: its `uniformSchema`, its field inputs with their `inputDomains`, and what it says about
/// itself for the editor ([`declare`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Declaration {
    pub uniforms: Vec<Uniform>,
    pub inputs: Vec<Input>,
    pub described: Described,
}

impl Declaration {
    /// The declarations in `source`: each line whose text, leading WGSL white space (`is_space`) removed, starts
    /// `// @` as a real line comment, not inside a block comment, its lines split at WGSL's line breaks (`lines`), where
    /// naga ends the declaration's comment. The text
    /// before `//` is WGSL source, so its white space is WGSL's, not Unicode's. Any word after `@` but `uniform`,
    /// `input` and [`declare::WORDS`] is refused, as is a malformed line, a name declared twice, or a control or glyph
    /// [`declare`] refuses.
    pub fn parse(source: &str) -> Result<Declaration, AssembleError> {
        let mut out = Declaration::default();
        let all = lines(source);
        let comments = declare::line_comments(source);
        // The version first, so a later grammar is refused as one whatever word comes before its `@version` line.
        let mut grammar = 1;
        for (k, line) in all.iter().enumerate() {
            let Some(rest) = declare::declaration_rest(source, line, &comments) else {
                continue;
            };
            let (word, v) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            if word == "version" {
                let mut read = Described::default();
                match declare::read("version", v, k + 1, &mut read, 1) {
                    Err(declare::Refusal(check, why))
                        if check == Check::Version && declare::on(check) =>
                    {
                        return Err(AssembleError::Declaration {
                            line: k + 1,
                            why: format!("`{}`: {why}", line.trim()),
                        })
                    }
                    _ => grammar = read.version.unwrap_or(1),
                }
            }
        }
        for (k, line) in all.iter().enumerate() {
            let Some(rest) = declare::declaration_rest(source, line, &comments) else {
                continue;
            };
            let bad = |why: &str| AssembleError::Declaration {
                line: k + 1,
                why: format!("`{}`: {why}", line.trim()),
            };
            let (word, rest) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            match word {
                "uniform" => match uniform(rest, grammar) {
                    Ok(u) => out.uniforms.push(u),
                    // A uniform of a new type refused for its own sake is a check a test's control can lift.
                    Err(why) if is_param(rest) && !declare::on(Check::Param) => {
                        let _ = why;
                        continue;
                    }
                    Err(why) => return Err(bad(&why)),
                },
                "input" => out.inputs.push(input(rest).map_err(|w| bad(&w))?),
                w if declare::WORDS.contains(&w) => {
                    match declare::read(w, rest, k + 1, &mut out.described, grammar) {
                        Err(declare::Refusal(check, why)) if declare::on(check) => {
                            return Err(bad(&why))
                        }
                        _ => continue,
                    }
                }
                _ if !declare::on(Check::Word) => continue,
                _ => {
                    return Err(bad(&format!(
                        "a declaration is `@uniform`, `@input` or `@{}`",
                        declare::WORDS.join("`, `@")
                    )))
                }
            }
            let names: Vec<&str> = match word {
                "uniform" => out.uniforms.iter().map(|u| u.name.as_str()).collect(),
                _ => out.inputs.iter().map(|i| i.name.as_str()).collect(),
            };
            if names[..names.len() - 1].contains(names.last().unwrap_or(&"")) {
                return Err(bad("the name is declared twice"));
            }
            let block: Vec<Uniform> = out
                .uniforms
                .iter()
                .filter(|u| !u.is_baked())
                .cloned()
                .collect();
            let size = crate::pipeline_cache::block_layout(&block).1;
            if word == "uniform" && size > params::BLOCK_MAX && declare::on(Check::Param) {
                return Err(bad(&format!(
                    "the uniforms take {size} bytes; a node's block is at most {}",
                    params::BLOCK_MAX
                )));
            }
        }
        declare::check(&out.described, &out.uniforms, &out.inputs).map_err(
            |(line, declare::Refusal(_, why))| AssembleError::Declaration {
                line,
                why: format!("`{}`: {why}", all[line - 1].trim()),
            },
        )?;
        Ok(out)
    }
}

/// Whether the `@uniform` line `rest` names a type of grammar version 2.
fn is_param(rest: &str) -> bool {
    let ty = rest
        .split_once(':')
        .map_or("", |(_, t)| t.split_once('=').map_or(t, |(t, _)| t));
    !matches!(params::parse_type(ty), Ok(None))
}

/// `<name>: <type> = <default>` then an optional `[<lo>, <hi>]`; the new types of grammar version 2 ([`params`]) read
/// their default as their own text, and take no range.
fn uniform(rest: &str, version: u32) -> Result<Uniform, String> {
    let (rest, baked) = match rest.trim_end().strip_suffix(" baked") {
        Some(r) => (r, true),
        None => (rest, false),
    };
    let (name, rest) = rest.split_once(':').ok_or("no `:` after the name")?;
    let name = identifier(name)?;
    if let Some(ty) = params::parse_type(rest.split_once('=').map_or(rest, |(t, _)| t))? {
        if version < 2 {
            return Err(format!(
                "`{}` is a grammar version 2 type: the source declares `// @version 2` first",
                rest.split_once('=').map_or(rest, |(t, _)| t).trim()
            ));
        }
        let (_, default) = rest.split_once('=').ok_or("no `=` before the default")?;
        let any_kind = ty == UniformType::Field && default.trim_end().ends_with(" any");
        let scalars = ty == UniformType::Field && default.trim_end().ends_with(" scalar");
        let default = if any_kind {
            default.trim_end().trim_end_matches("any")
        } else if scalars {
            default.trim_end().trim_end_matches("scalar")
        } else {
            default
        };
        let default = if scalars {
            let name = default.trim();
            let at = ramp_fields()
                .map_err(|e| e.to_string())?
                .iter()
                .position(|f| f == name)
                .ok_or_else(|| format!("`{name}` is no scalar field a ramp reads"))?;
            vec![at as f64]
        } else {
            params::parse_default(ty, default)?
        };
        let u = Uniform {
            name,
            ty,
            default,
            range: None,
            any_kind,
            scalars,
            baked,
        };
        if !u.admits(&u.default) {
            return Err(format!(
                "the default is not a value of the type: {}",
                params::why_not(ty, &u.default)
            ));
        }
        return Ok(u);
    }
    let (ty, rest) = rest.split_once('=').ok_or("no `=` before the default")?;
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
        any_kind: false,
        scalars: false,
        baked,
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

/// The WGSL types of the read-side members a built-in source may read (lowering Part 3a): the scalars, `f32`, `u32`
/// and `bool`, and the vector `vec3<f32>`, which a `Field` carries in `.xyz` (render_gui_spec Part II §3, §5: the
/// shape n̂ source).
const SOURCE_TYPES: [&str; 4] = ["f32", "u32", "bool", "vec3<f32>"];

/// The read-side fields a built-in source may read: the scalar members, `f32`, `u32` or `bool`, and the vector
/// members, `vec3<f32>` (lowering Part 3a; render_gui_spec Part II §5).
pub fn source_fields() -> Result<Vec<String>, AssembleError> {
    let (words, entries) = ledger()?;
    Ok(read::members(words, entries)
        .into_iter()
        .filter(|m| SOURCE_TYPES.contains(&m.wgsl))
        .map(|m| m.name)
        .collect())
}

/// Every scalar field a field ramp reads, in the ledger's order: the members of the read side and of the initial
/// conditions and the word, each as [`crate::colour::field_ramp::ScalarField`] reads it with its validity.
pub fn ramp_fields() -> Result<Vec<String>, AssembleError> {
    static KEPT: OnceLock<Vec<String>> = OnceLock::new();
    if let Some(k) = KEPT.get() {
        return Ok(k.clone());
    }
    let (_, entries) = ledger()?;
    let names: Vec<String> = entries
        .iter()
        .filter(|e| crate::colour::field_ramp::ScalarField::payload(e.name).is_ok())
        .map(|e| e.name.to_owned())
        .collect();
    Ok(KEPT.get_or_init(|| names).clone())
}

/// The subtype of a built-in source's field (render_gui_spec Part II §5: "picking the field sets the out-port's
/// subtype"): vector for a `vec3<f32>`; categorical for a `bool` or a field the ledger scales as categorical or as a
/// flag (dd_generation_root §3.8 `scale`); scalar otherwise; or the field is none.
pub fn field_subtype(field: &str) -> Result<Subtype, AssembleError> {
    let (_, entries) = ledger()?;
    let wgsl = field_type(field)?;
    let scale = entries.iter().find(|e| e.name == field).map(|e| e.scale);
    Ok(match (wgsl, scale) {
        ("vec3<f32>", _) => Subtype::Vector,
        ("bool", _) | (_, Some(Scale::Categorical(_) | Scale::Flag)) => Subtype::Categorical,
        _ => Subtype::Scalar,
    })
}

/// The WGSL `occupant` gives a node of `kind`, or `None` for the identity: a shipped source's file, named by id from
/// the registry (`registry::named`), and a custom's text alike, a field's source generated from its field. An
/// occupant the slot does not take, or a name no file gives the slot, is refused.
pub fn occupant_source(kind: Kind, occupant: &Occupant) -> Result<Option<String>, AssembleError> {
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
            field_type(f)?;
            let shipped = crate::registry::named(Kind::Source, FIELD_SOURCE)
                .map_err(|e| AssembleError::Occupant(e.to_string()))?;
            let at = shipped.source.find(FIELD_DEFAULT).ok_or_else(|| {
                AssembleError::Occupant(format!("`{FIELD_SOURCE}` declares no `{FIELD_DEFAULT}`"))
            })?;
            let mut text = shipped.source.clone();
            text.replace_range(
                at..at + FIELD_DEFAULT.len(),
                &format!("// @uniform field: field = {f} any"),
            );
            Ok(Some(text))
        }
        (_, Occupant::Field(_)) => refuse("a field: only a source does"),
        (_, Occupant::BuiltIn(id)) => crate::registry::named(kind, id)
            .map(|e| Some(e.source.clone()))
            .map_err(|e| AssembleError::Occupant(e.to_string())),
        (_, Occupant::Custom(text)) => Ok(Some(text.clone())),
    }
}

/// The shipped source of a read-side field, and the line of it that holds the field it defaults to.
const FIELD_SOURCE: &str = "field";
const FIELD_DEFAULT: &str = "// @uniform field: field = state any";

/// `text` of a node with the declaration `d`, each baked uniform written into its code as it defaults, the only value a
/// node can have where nothing chose another ([`params::specialise`]).
fn at_defaults(text: String, d: &Declaration) -> Result<String, AssembleError> {
    let baked: Vec<&Uniform> = d.uniforms.iter().filter(|u| u.is_baked()).collect();
    if baked.is_empty() {
        return Ok(text);
    }
    params::specialise(&text, &baked, &|u| u.default.clone())
}

/// The code `occupant` runs in a node of `kind` where no stain chose a field: [`occupant_source`] with each `field`
/// uniform written in as its default.
pub fn occupant_code(kind: Kind, occupant: &Occupant) -> Result<Option<String>, AssembleError> {
    let Some(text) = occupant_source(kind, occupant)? else {
        return Ok(None);
    };
    let d = declaration_of(kind, Some(&text))?;
    at_defaults(text, &d).map(Some)
}

/// Whether `a` and `b` are the same code: the same tokens, comments and white space aside, so one that differs only
/// in a label draws the same picture.
pub fn same_code(a: &str, b: &str) -> bool {
    let words = |s: &str| -> Option<Vec<String>> {
        Some(
            lex(s)
                .ok()?
                .into_iter()
                .filter(|t| !matches!(t.0, Tok::Comment | Tok::Space))
                .map(|(_, x, y)| s[x..y].to_owned())
                .collect(),
        )
    };
    matches!((words(a), words(b)), (Some(x), Some(y)) if x == y)
}

/// A built-in source's field's WGSL type, or the field is none.
fn field_type(field: &str) -> Result<&'static str, AssembleError> {
    let (words, entries) = ledger()?;
    read::members(words, entries)
        .into_iter()
        .find(|m| m.name == field && SOURCE_TYPES.contains(&m.wgsl))
        .map(|m| m.wgsl)
        .ok_or_else(|| AssembleError::Occupant(format!("`{field}` is no field a source reads")))
}

/// `occupant`'s declarations in a node of `kind`, checked against the kind: a source and a combiner declare no
/// input; a colour declares one to [`MAX_INPUTS`], a brightness one, and a post up to [`MAX_INPUTS`] (a colour or a
/// brightness that declares none has one, `field`, its domain inherited); an occupant the slot does not take is refused.
pub fn declaration(kind: Kind, occupant: &Occupant) -> Result<Declaration, AssembleError> {
    declaration_of(kind, occupant_source(kind, occupant)?.as_deref())
}

/// [`declaration`] of a node of `kind` whose occupant's text is `source`, `None` for the identity.
fn declaration_of(kind: Kind, source: Option<&str>) -> Result<Declaration, AssembleError> {
    let declared = declaration_of_text(kind, source)?;
    if kind == Kind::Combiner && declared.uniforms.iter().any(|u| u.ty == UniformType::Field) {
        return Err(AssembleError::Occupant(
            "a combiner has no context to read a field from: it declares no `field` uniform".into(),
        ));
    }
    Ok(declared)
}

fn declaration_of_text(kind: Kind, source: Option<&str>) -> Result<Declaration, AssembleError> {
    let Some(source) = source else {
        let mut d = Declaration::default();
        if matches!(kind, Kind::Colour | Kind::Brightness) {
            d.inputs.push(default_input());
        }
        return Ok(d);
    };
    let mut d = Declaration::parse(source)?;
    // A field is written into each `uniforms.<name>` read, so a node that reads its uniforms any other way (whole, or
    // through a pointer) cannot hold one.
    if d.uniforms.iter().any(|u| u.ty == UniformType::Field)
        && matters::without_uniforms(source, &[], "").is_none()
    {
        return Err(AssembleError::Occupant(
            "a node with a `field` uniform reads its uniforms only as `uniforms.<name>`".into(),
        ));
    }
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
    /// Each node's occupant's text, read once at construction: what is declared, keyed and assembled is one text.
    texts: Vec<Option<String>>,
}

impl Stain {
    /// `nodes` as a stain, or why not: an occupant its slot does not take or a malformed declaration; a node whose
    /// inputs are not its in-ports; a wire that runs backward (from a later node or itself, so a cycle cannot be
    /// written) or breaks the port types or the backbone ([`check_wire`]); other than one combiner and one `OUT`; or
    /// more than [`MAX_POSTS`] posts in the live chain.
    pub fn new(nodes: Vec<Node>) -> Result<Stain, AssembleError> {
        let mut declarations = Vec::new();
        let mut texts = Vec::new();
        for (i, n) in nodes.iter().enumerate() {
            let text = occupant_source(n.kind, &n.occupant)?;
            let d = declaration_of(n.kind, text.as_deref())?;
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
            // The text made at the defaults declares what its block holds: a line a `//? when` hid declares nothing.
            let made = text.clone().map(|t| at_defaults(t, &d)).transpose()?;
            let d = if made == text {
                d
            } else {
                declaration_of(n.kind, made.as_deref())?
            };
            let text = made;
            texts.push(text);
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
            texts,
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

    /// Node `i`'s occupant's WGSL as written, `None` for the identity: the code its key hashes and it assembles.
    pub fn text(&self, i: usize) -> Option<&str> {
        self.texts[i].as_deref()
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
        let texts = order.iter().map(|&i| self.texts[i].clone()).collect();
        Canonical {
            order,
            stain: Stain {
                nodes,
                declarations,
                texts,
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
    /// its occupant `null` for the identity, else `{"code":…}`, the code it runs as [`key_text`] gives it, its
    /// descriptive lines removed, in JCS (RFC 8785: keys sorted, no white space, strings escaped as ECMAScript's
    /// `JSON.stringify` escapes them). The fragment key's input.
    ///
    /// Who wrote the code is not in it: a shipped source, the same text pasted as a custom and a field's generated
    /// source are keyed alike, so one code compiles once.
    // Don't put a shipped source's id or a field's name in it: the same code would compile twice, and a shipped file
    // that changed under its id would keep its old key.
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
            let Some(code) = self.stain.text(k) else {
                out.push_str("null}");
                continue;
            };
            out.push('{');
            #[cfg(feature = "controls")]
            if let (true, Occupant::BuiltIn(id)) = (probe::id_in_key(), &n.occupant) {
                json_string("builtin", &mut out);
                out.push(':');
                json_string(id, &mut out);
                out.push(',');
            }
            json_string("code", &mut out);
            out.push(':');
            json_string(&key_text(code), &mut out);
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

/// An assembled stain: its WGSL, the read-side fields its `sample_read` fills, its nodes' uniform blocks, whether
/// it carries a debug arm, which binds the counts at [`DEBUG_COUNTS_GROUP`], and what the proof left out of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    pub source: String,
    pub fields: Vec<String>,
    pub uniforms: Vec<UniformBlock>,
    pub debug_arm: bool,
    pub dropped: Dropped,
}

/// What provably cannot change a stain's picture, and so is left out of its assembly ([`matters`]): nodes whose result
/// nothing reads, never emitted; field inputs the node never reads, never wired; uniforms the node never reads, out of
/// its block, a read of one in dead code given its type's zero. Each by its node's canonical position.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dropped {
    pub nodes: Vec<usize>,
    /// (node, field input).
    pub inputs: Vec<(usize, usize)>,
    pub uniforms: Vec<(usize, Uniform)>,
}

impl Dropped {
    /// Node `i`'s uniforms left out.
    pub fn uniforms_of(&self, i: usize) -> Vec<&Uniform> {
        self.uniforms
            .iter()
            .filter(|(n, _)| *n == i)
            .map(|(_, u)| u)
            .collect()
    }
}

/// The one assembler entry point (render contract Part 2): `stain` at `tier`, a presentation view, its read side
/// filling the fields its IR reads ([`field_set`]), refused where any step fails ([`assemble_reading`]).
pub fn assemble(stain: &Stain, tier: Tier) -> Result<Fragment, AssembleError> {
    assemble_view(stain, tier, View::Presentation)
}

/// [`assemble`] as the variant `view` (R-41; R-467 D5, R-470 G4): the debug view's with [`DEBUG_ARM`].
pub fn assemble_view(stain: &Stain, tier: Tier, view: View) -> Result<Fragment, AssembleError> {
    assemble_arm(
        stain,
        tier,
        match view {
            View::Presentation => None,
            View::Debug => Some(DEBUG_ARM),
        },
    )
}

/// What a node list renders as ([`assemble_or_fallback`]): its fragment, and, where the list is no graph and the
/// fragment is the fallback's, why it was refused.
#[derive(Clone, Debug, PartialEq)]
pub struct Rendered {
    pub fragment: Fragment,
    pub invalid: Option<AssembleError>,
}

/// `nodes` as a fragment, whatever they are (render_gui_spec §13, "Always-renderable"; REQ-RENDER-072): a graph
/// ([`Stain::new`] takes it) assembles as `view` ([`assemble_view`]); a node list that is no graph, which the editor's
/// fixed `OUT` and combiner and the identity rules leave unreachable, renders the defined fallback ([`fallback`]) and
/// carries the refusal. An error is returned only where a graph's assembly fails, an occupant's WGSL that does not
/// compile among them: that is a node's failure, not an invalid graph, and the runtime keeps that node's last valid
/// occupant ([`crate::pipeline_cache`]; gui_state_contract §5).
pub fn assemble_or_fallback(
    nodes: Vec<Node>,
    tier: Tier,
    view: View,
) -> Result<Rendered, AssembleError> {
    match Stain::new(nodes) {
        Ok(stain) => Ok(Rendered {
            fragment: assemble_view(&stain, tier, view)?,
            invalid: None,
        }),
        Err(why) => Ok(Rendered {
            fragment: fallback(tier)?,
            invalid: Some(why),
        }),
    }
}

/// The invalid-graph fallback's fragment at `tier` (render_gui_spec §13; [`codegen::fallback`]): the template with no
/// node function, its `shade()` returning the fallback's flat colour at every pixel. It reads no field and has no
/// uniform block; it has no debug arm in either view, so it is flat there too and binds no counts. It is a function
/// of the tier alone, so each tier's is assembled once and kept.
pub fn fallback(tier: Tier) -> Result<Fragment, AssembleError> {
    static KEPT: OnceLock<Mutex<HashMap<Tier, Fragment>>> = OnceLock::new();
    let kept = KEPT.get_or_init(Mutex::default);
    // A poisoned lock still holds whole fragments: each is inserted complete.
    if let Some(f) = kept
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&tier)
    {
        return Ok(f.clone());
    }
    let mut source = head(tier, &[])?;
    source.push_str(&shade_functions(
        &format!("    {}\n", codegen::fallback::statement()),
        false,
    ));
    source.push_str(PRESENTATION_HOOK);
    compile(&source)?;
    let fragment = Fragment {
        source,
        fields: Vec::new(),
        uniforms: Vec::new(),
        debug_arm: false,
        dropped: Dropped::default(),
    };
    kept.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(tier, fragment.clone());
    Ok(fragment)
}

/// [`assemble`] with the debug arm `arm`, WGSL defining
/// `debug_view(forbidden: bool, s: SimState, frag_xy: vec2<f32>, rgb: vec3<f32>) -> vec3<f32>` and `note_non_finite()`, or none, the presentation variant: [`assemble_view`]'s step, open so a test's control can
/// assemble an altered arm.
pub fn assemble_arm(
    stain: &Stain,
    tier: Tier,
    arm: Option<&str>,
) -> Result<Fragment, AssembleError> {
    assemble_placed(stain, tier, arm, false)
}

/// [`assemble_view`] with nothing the proof finds dead left out: every node, wire and uniform, and every field its IR
/// names. The picture is the same bit for bit; a test sets what [`Fragment::dropped`] names here to anything at all.
pub fn assemble_kept(stain: &Stain, tier: Tier, view: View) -> Result<Fragment, AssembleError> {
    let arm = match view {
        View::Presentation => None,
        View::Debug => Some(DEBUG_ARM),
    };
    let every = every_field()?;
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    let (source, _) = source(stain, tier, &every, arm, false, &Dropped::default(), true)?;
    let (module, info) = compile(&source)?;
    let fields: Vec<String> = fields_read(&module, &info).into_iter().collect();
    let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
    assemble_reading_arm(stain, tier, &fields, arm, false, &Dropped::default())
}

/// [`assemble`] with each occupant placed as written, its descriptive lines kept rather than blanked ([`code_text`]):
/// the check that those lines are not code, since naga must make one module of the two.
pub fn assemble_written(stain: &Stain, tier: Tier) -> Result<Fragment, AssembleError> {
    assemble_placed(stain, tier, None, true)
}

/// [`assemble_arm`], each occupant placed as written where `written`, else with its descriptive lines blanked.
fn assemble_placed(
    stain: &Stain,
    tier: Tier,
    arm: Option<&str>,
    written: bool,
) -> Result<Fragment, AssembleError> {
    let (fields, dropped) = plan(stain, tier, arm, written)?;
    let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
    assemble_reading_arm(stain, tier, &fields, arm, written, &dropped)
}

/// The stain's field set: every member of the read-side `SimState` its live nodes read into its picture, by
/// naga's IR of the stain assembled with every field filled, the word as `word` or as only the components it reads,
/// and each member of `ctx.ic` it reads as `ic.<member>` ([`fields_read`], less what the proof finds cannot reach the
/// picture, [`matters::live_fields`]).
pub fn field_set(stain: &Stain, tier: Tier) -> Result<Vec<String>, AssembleError> {
    field_set_arm(stain, tier, None, false)
}

/// [`field_set`] with the debug arm `arm`, whose reads join the stain's.
fn field_set_arm(
    stain: &Stain,
    tier: Tier,
    arm: Option<&str>,
    written: bool,
) -> Result<Vec<String>, AssembleError> {
    plan(stain, tier, arm, written).map(|(fields, _)| fields)
}

/// Every field the read side can fill: each read-side member and each `ic.<member>`.
fn every_field() -> Result<Vec<String>, AssembleError> {
    let (words, entries) = ledger()?;
    let mut every: Vec<String> = read::members(words, entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    every.extend(read::ic_fields());
    Ok(every)
}

/// The stain's field set and what its assembly may leave out, from the proof over its IR assembled with every field
/// filled and nothing dropped.
// Don't drop on a node's present settings (question 2): a pipeline is shared by every stain of its key, and settings are
// not in the key.
fn plan(
    stain: &Stain,
    tier: Tier,
    arm: Option<&str>,
    written: bool,
) -> Result<(Vec<String>, Dropped), AssembleError> {
    let every = every_field()?;
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    let (source, _) = source(stain, tier, &every, arm, written, &Dropped::default(), true)?;
    let (module, info) = compile(&source)?;
    let held = matters::Held::new();
    let proof = matters::Proof::new(&module, &info, &matters::STAIN_ROOTS, &held);
    let fields = matters::live_fields(&module, &proof, fields_read(&module, &info));
    let dropped = matters::dropped(stain, &proof, written)?;
    Ok((fields.into_iter().collect(), dropped))
}

/// [`assemble`]'s second step: `stain` at `tier` with a read side whose `sample_read` fills `fields`, compiled;
/// refused when the compiled stain reads a field `fields` misses,
/// which `sample_read` would leave reading 0 (R-378).
pub fn assemble_reading(
    stain: &Stain,
    tier: Tier,
    fields: &[&str],
) -> Result<Fragment, AssembleError> {
    assemble_reading_arm(stain, tier, fields, None, false, &Dropped::default())
}

/// [`assemble_reading`] with the debug arm `arm`.
fn assemble_reading_arm(
    stain: &Stain,
    tier: Tier,
    fields: &[&str],
    arm: Option<&str>,
    written: bool,
    dropped: &Dropped,
) -> Result<Fragment, AssembleError> {
    let (source, uniforms) = source(stain, tier, fields, arm, written, dropped, true)?;
    let (module, info) = compile(&source)?;
    let filled = |f: &str| {
        fields.contains(&f)
            || f.strip_prefix("word.").is_some() && fields.contains(&"word")
            || f == "word"
                && read::WORD_COMPONENTS
                    .iter()
                    .all(|c| fields.contains(&&*format!("word.{c}")))
    };
    // A read the proof finds cannot reach the picture may read 0: only the reads that can must be filled.
    let held = matters::Held::new();
    let proof = matters::Proof::new(&module, &info, &matters::STAIN_ROOTS, &held);
    let reads = matters::live_fields(&module, &proof, fields_read(&module, &info));
    if let Some(missing) = reads.into_iter().find(|f| !filled(f)) {
        return Err(AssembleError::UnfilledField(missing));
    }
    let mut fields: Vec<String> = fields.iter().map(|&f| f.to_owned()).collect();
    fields.sort();
    fields.dedup();
    Ok(Fragment {
        source,
        fields,
        uniforms,
        debug_arm: arm.is_some(),
        dropped: dropped.clone(),
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

/// The stain's context (render contract Part 1's `RenderContext`, its lanes as M1 fills them; R-369), its `inputs`
/// [`MAX_INPUTS`] long: a file of its own, so `cargo xtask lint wgsl` lints the generated debug views after it.
pub const CONTEXT: &str = include_str!("../shaders/wgsl/stain/context.wgsl");

/// What precedes the node functions at `tier`: the prelude, the maps and layers that follow it, the read side filling
/// `fields`, and the stain's context.
fn head(tier: Tier, fields: &[&str]) -> Result<String, AssembleError> {
    let (words, entries) = ledger()?;
    let read_side = read::assemble(words, entries, tier, fields).map_err(AssembleError::Field)?;
    Ok(format!(
        "{}\n{COLOUR_SPACE}\n{PRESENT}\n{COMPACTION}\n{RAMP}\n{DRIFT}\n{PALETTE}\n{read_side}\n// ── The stain (TASK-M1-04): context, node functions, shade() ──\n{}",
        prelude::wgsl(tier),
        CONTEXT
    ))
}

/// `stain`'s WGSL at `tier`, its read side filling `fields`, with the debug arm `arm` or the presentation variant's
/// hook, and its uniform blocks; each occupant as written where `written`, else its descriptive lines blanked; what
/// `dropped` names left out.
fn source(
    stain: &Stain,
    tier: Tier,
    fields: &[&str],
    arm: Option<&str>,
    written: bool,
    dropped: &Dropped,
    standard: bool,
) -> Result<(String, Vec<UniformBlock>), AssembleError> {
    let mut out = head(tier, fields)?;
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
    let mut standard_nodes: Vec<usize> = Vec::new();
    // The structs of the new parameter types, once each, before the first node that holds one: a dropped uniform's
    // struct too, since the dropped block names it.
    let mut library: Vec<UniformType> = Vec::new();
    for &i in &live {
        if dropped.nodes.contains(&i) {
            continue;
        }
        let n = &stain.nodes[i];
        let text = stain.text(i).unwrap_or_default();
        let text = if written {
            std::borrow::Cow::Borrowed(text)
        } else {
            code_text(text)
        };
        let gone = dropped.uniforms_of(i);
        let new: Vec<UniformType> = stain.declarations[i]
            .uniforms
            .iter()
            .map(|u| u.ty)
            .filter(|t| params::struct_name(*t).is_some() && !library.contains(t))
            .collect();
        out.push_str(&params::library(&new));
        library.extend(new);
        let defaults: Vec<&Uniform> = stain.declarations[i]
            .uniforms
            .iter()
            .filter(|u| u.is_baked())
            .collect();
        let text = std::borrow::Cow::Owned(params::specialise(&text, &defaults, &|u| {
            u.default.clone()
        })?);
        let text = match gone.is_empty() {
            true => text,
            false => {
                // Don't give a dropped uniform a constant: naga folds constant expressions, and `1.0 / f32()` is
                // refused. A zero private struct reads zero at run time.
                let _ = writeln!(
                    out,
                    "\n// Node {i}'s uniforms that cannot change the picture: never bound, each read zero."
                );
                let _ = writeln!(out, "struct n{i}_Dropped {{");
                for u in &gone {
                    let _ = writeln!(out, "    {}: {},", u.name, u.ty.wgsl());
                }
                let _ = writeln!(out, "}}\nvar<private> n{i}_dropped: n{i}_Dropped;");
                let to = format!("n{i}_dropped");
                std::borrow::Cow::Owned(matters::without_uniforms(&text, &gone, &to).ok_or_else(
                    || {
                        AssembleError::Occupant(
                            "a dropped uniform is read other than as `uniforms.<name>`".into(),
                        )
                    },
                )?)
            }
        };
        // A baked uniform is written into the node's code, so it has no member.
        let schema: Vec<Uniform> = stain.declarations[i]
            .uniforms
            .iter()
            .filter(|u| !gone.contains(u) && !u.is_baked())
            .cloned()
            .collect();
        let schema = &schema;
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
        // The debug variant draws no standard treatment: its arm is the instrument's own check of every pixel.
        if standard && arm.is_none() && wrapped(n.kind, stain.text(i).unwrap_or_default()) {
            out.push_str(&standard_treatment(i, n.kind));
            standard_nodes.push(i);
        }
    }
    out.push_str(&shade(
        stain,
        &sources,
        colour,
        brightness,
        combiner,
        &chain,
        arm.is_some(),
        dropped,
        &standard_nodes,
    ));
    out.push_str(arm.unwrap_or(PRESENTATION_HOOK));
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

/// `shade(ctx)`, which walks the graph in backbone order: the sources, the colour and the brightness, the combiner, the
/// post chain in declaration order (colour_composition §4.2), `OUT`; with `shade_at` and `shade_sample`
/// ([`shade_functions`]).
#[allow(clippy::too_many_arguments)] // the walk's parts, as `source` has them; a struct would only rename them
fn shade(
    stain: &Stain,
    sources: &[usize],
    colour: Option<usize>,
    brightness: Option<usize>,
    combiner: usize,
    chain: &[usize],
    debug: bool,
    dropped: &Dropped,
    standard: &[usize],
) -> String {
    let mut out = String::new();
    let gone = |i: usize| dropped.nodes.contains(&i);
    let slot_fn = |i: usize, slot: &str| {
        if standard.contains(&i) {
            format!("n{i}_standard")
        } else {
            format!("n{i}_{slot}")
        }
    };
    for &s in sources.iter().filter(|&&s| !gone(s)) {
        let _ = writeln!(out, "    let f{s} = n{s}_source(ctx);");
    }
    let context = |i: usize, out: &mut String| {
        let _ = writeln!(out, "    var c{i} = ctx;");
        for (k, j) in stain.field_inputs(i).into_iter().enumerate() {
            if !dropped.inputs.contains(&(i, k)) {
                let _ = writeln!(out, "    c{i}.inputs[{k}] = f{};", j.unwrap_or_default());
            }
        }
    };
    // A dropped colour or brightness is still named: the combiner's statement reads it, and provably not into the
    // picture, so its type's zero stands in.
    match colour {
        Some(c) if gone(c) => out.push_str("    let rgb = vec3<f32>();\n"),
        Some(c) => {
            context(c, &mut out);
            let _ = writeln!(out, "    let rgb = {}(c{c});", slot_fn(c, "colour"));
        }
        None => {}
    }
    match brightness {
        Some(b) if gone(b) => out.push_str("    let b = 0.0;\n"),
        Some(b) => {
            context(b, &mut out);
            let _ = writeln!(out, "    let b = {}(c{b});", slot_fn(b, "brightness"));
        }
        None => {}
    }
    let combine = codegen::combine::statement(
        &format!("n{combiner}_combine"),
        colour.is_some(),
        brightness.is_some(),
    );
    let _ = writeln!(out, "    {combine}");
    for &p in chain.iter().filter(|&&p| !gone(p)) {
        context(p, &mut out);
        let _ = writeln!(out, "    out = n{p}_post(c{p}, out);");
    }
    shade_functions(&out, debug)
}

/// `shade(ctx)` with the body `walk`, the statements that leave the graph's colour in `out`, a graph's walk ([`shade`])
/// or the fallback's one statement ([`fallback`]); `shade_at`, which reads sample `i` into a context with its screen,
/// quad, tile and validity lanes and shades it; and `shade_sample`, the same with the screen, quad and tile lanes
/// absent (the absence NaN) and the validity lane unclassified, no sample forbidden, for a pass that places no quad
/// (REQ-COL-102). With `debug`, `shade_at` returns the debug view's arm over the walk's colour,
/// `debug_view(ctx.validity.forbidden, ctx.sample, ctx.frag_xy, shade(ctx))` (REQ-COL-109).
fn shade_functions(walk: &str, debug: bool) -> String {
    let mut out = String::from(
        "\n// The generated walk of the graph, in backbone order (render contract Part 2): sources → colour / brightness \
         → combiner → (post)* → OUT.\nfn shade(ctx: Ctx) -> vec3<f32> {\n",
    );
    out.push_str(walk);
    out.push_str(
        "    return out; // OUT\n}\n\n\
         // Sample `i` read through the generated read side into a context at pixel `frag_xy`, its `ICDescriptor` too\n\
         // (`ctx.ic`, only the members the stain reads), with its screen, quad, tile and validity lanes, and shaded.\n\
         // The read side's arguments are its own (`ledger::gen::read`); `has_ensemble` is the prelude's uniform (R-145).\n\
         fn shade_at(i: u32, frag_xy: vec2<f32>, screen: CtxScreen, quad: CtxQuad, tile: CtxTile, validity: CtxValidity, ensemble_spread: f32, masses: vec3<f32>, params: ReadParams) -> vec3<f32> {\n    \
         var ctx: Ctx;\n    \
         ctx.sample = sample_read(i, ensemble_spread, has_ensemble(), masses, params);\n    \
         ctx.ic = ic_read(i);\n    \
         ctx.frag_xy = frag_xy;\n    \
         ctx.screen = screen;\n    \
         ctx.quad = quad;\n    \
         ctx.tile = tile;\n    \
         ctx.validity = validity;\n    \
         ctx.params = params;\n",
    );
    // The debug variant draws its arm over the walk's colour; the presentation variant returns the walk's (R-470 G4).
    out.push_str(if debug {
        "    return debug_view(ctx.validity.forbidden, ctx.sample, ctx.frag_xy, shade(ctx));\n}\n\n"
    } else {
        "    return shade(ctx);\n}\n\n"
    });
    out.push_str(
        "// Sample `i` shaded at pixel `frag_xy` where no raster places it in a target, a quad and a tile: its screen,\n\
         // quad and tile lanes hold the absence NaN, the canonical quiet NaN (`is_absent_nan`), each u32 member its\n\
         // bits (`ctx_quad_absent`, `ctx_tile_absent`), and no chart classification reaches it\n\
         // (`ctx_validity_unclassified`).\n\
         fn shade_sample(i: u32, frag_xy: vec2<f32>, ensemble_spread: f32, masses: vec3<f32>, params: ReadParams) -> vec3<f32> {\n    \
         let absent = vec2<f32>(bitcast<f32>(0x7fc00000u));\n    \
         return shade_at(i, frag_xy, CtxScreen(absent), ctx_quad_absent(), ctx_tile_absent(), ctx_validity_unclassified(), ensemble_spread, masses, params);\n}\n",
    );
    out
}

// ── Node functions: one per node, its names prefixed ──────────────────────────────────────────────────────────────

/// The name a source uses to say it draws not-a-system, invalid and not-yet-run pixels itself: it calls
/// `handles_validity();`, or declares `const HANDLES_VALIDITY: bool = true;` where a call has no place. The assembler
/// wraps every other colour and brightness node in the standard treatment ([`standard_treatment`]).
pub const HANDLES_VALIDITY: &str = "handles_validity";

/// The constant form of the marker, for a source where a call has no place; a different name, so a source that is linted
/// beside the context's own `handles_validity` does not redefine it.
pub const HANDLES_VALIDITY_CONST: &str = "HANDLES_VALIDITY";

/// Whether `text` says it handles validity itself: it names [`HANDLES_VALIDITY`] outside a comment.
pub fn handles_validity(text: &str) -> bool {
    #[cfg(feature = "controls")]
    if probe::marker_ignored() {
        return false;
    }
    tokens(text).0.iter().any(|(t, a, b)| {
        *t == Tok::Ident
            && matches!(
                text.get(*a..*b),
                Some(HANDLES_VALIDITY | HANDLES_VALIDITY_CONST)
            )
    })
}

/// Whether a node of `kind` with source `text` is wrapped in the standard treatment: a colour or a brightness that
/// does not handle validity itself. A source, a combiner and a post take a colour or a field already treated.
pub fn wrapped(kind: Kind, text: &str) -> bool {
    #[cfg(feature = "controls")]
    if probe::standard_off() {
        return false;
    }
    matches!(kind, Kind::Colour | Kind::Brightness) && !handles_validity(text)
}

/// Node `i`'s standard treatment, `n<i>_standard`, which `shade()` calls in place of its slot function: a sample the
/// chart classifies as not a system, or a `decode_failed` one, is the canvas background; a `sim_failed` sample or a
/// reserved state is the presentation views' invalid colour, or a brightness's zero; any other sample is the node's.
// Don't compute the node first and select: a node that divides or loops on a failed sample's garbage would still run.
// The plan is unclear here; chose to leave a `running` sample to the node, because the outcome palette and the field
// ramps draw it by their own map, so the standard treatment would have to be opted out of by every shipped colour.
fn standard_treatment(i: usize, kind: Kind) -> String {
    let (ty, blank, invalid, slot) = match kind {
        Kind::Brightness => ("f32", "0.0", "0.0", "brightness"),
        _ => (
            "vec3<f32>",
            "canvas_background()",
            "PRESENT_INVALID",
            "colour",
        ),
    };
    format!(
        "\n// Node {i}'s standard treatment of samples that are not a system or failed.\n\
         fn n{i}_standard(ctx: Ctx) -> {ty} {{\n    \
         if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {{ return {blank}; }}\n    \
         if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) {{ return {invalid}; }}\n    \
         return n{i}_{slot}(ctx);\n}}\n"
    )
}

/// `s` as [`tokens`]: refused where a block comment is never closed.
fn lex(s: &str) -> Result<Vec<(Tok, usize, usize)>, AssembleError> {
    match tokens(s) {
        (toks, true) => Ok(toks),
        (_, false) => Err(AssembleError::Compile("an unclosed `/*` comment".into())),
    }
}

/// The names of the functions `text` defines, in order: each name after `fn`, comments skipped (the registry reads
/// a debug occupant's slot from them, gui_state_contract §3).
pub fn defined_functions(text: &str) -> Result<Vec<String>, AssembleError> {
    let toks = lex(text)?;
    let sig: Vec<&str> = toks
        .iter()
        .filter(|(t, ..)| !matches!(t, Tok::Comment | Tok::Space))
        .map(|&(_, a, b)| &text[a..b])
        .collect();
    Ok(sig
        .windows(2)
        .filter(|w| w[0] == "fn")
        .map(|w| w[1].to_owned())
        .collect())
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
            w if (RESERVED.contains(&w) || is_ic_reader(w)) && toks[sig[k]].0 == Tok::Ident => {
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

/// Whether `name` is the read side's `ICDescriptor` reader or one of its per-member readers (RQ-227).
fn is_ic_reader(name: &str) -> bool {
    name.strip_prefix(IC_READ)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
}

/// The handle of the type named `name` in `module`.
fn named_type(module: &Module, name: &str) -> Option<Handle<naga::Type>> {
    module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .map(|(h, _)| h)
}

/// The read-side fields the stain in `module` reads, by naga's IR: each member of the read-side `SimState` taken in
/// any function but `sample_read`, which fills it; the word as `word`, or as `word.x` … `word.w` where only those
/// components are read (`components`); and each member of `ICDescriptor` taken in any function but the read side's
/// `ic_read` and `ic_read_<member>`, which fill it, as `ic.<member>` (RQ-227). These are the fields its `sample_read`
/// and `ic_read` must fill (R-378).
pub fn fields_read(module: &Module, info: &ModuleInfo) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let struct_of = |h: Option<Handle<naga::Type>>| {
        h.and_then(|h| match &module.types[h].inner {
            TypeInner::Struct { members, .. } => Some((h, members.clone())),
            _ => None,
        })
    };
    let simstate = struct_of(named_type(module, "SimState"));
    let ic = struct_of(named_type(module, read::IC_STRUCT));
    let mut read = |f: &Function, fn_info: &FunctionInfo, reader: bool| {
        for (h, x) in f.expressions.iter() {
            let Expression::AccessIndex { base, index } = *x else {
                continue;
            };
            let of = match *fn_info[base].ty.inner_with(&module.types) {
                TypeInner::Struct { .. } => fn_info[base].ty.handle(),
                TypeInner::Pointer { base, .. } => Some(base),
                _ => None,
            };
            let name = |members: &[naga::StructMember]| {
                members[index as usize].name.clone().unwrap_or_default()
            };
            if let Some((t, members)) = &ic {
                if of == Some(*t) && !reader {
                    out.insert(format!("ic.{}", name(members)));
                }
            }
            let Some((t, members)) = &simstate else {
                continue;
            };
            if of != Some(*t) || f.name.as_deref() == Some("sample_read") {
                continue;
            }
            let name = name(members);
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
        read(f, &info[h], f.name.as_deref().is_some_and(is_ic_reader));
    }
    for (k, ep) in module.entry_points.iter().enumerate() {
        read(&ep.function, info.get_entry_point(k), false);
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

/// A test's control's reach into the key, on this thread only.
#[cfg(feature = "controls")]
pub mod probe {
    use std::cell::Cell;

    thread_local! {
        static ID_IN_KEY: Cell<bool> = const { Cell::new(false) };
    }

    /// `f` run with a shipped source's name written into the fragment key beside its code, as a key by id would.
    pub fn id_in_key_in<T>(f: impl FnOnce() -> T) -> T {
        let before = ID_IN_KEY.with(|l| l.replace(true));
        let out = f();
        ID_IN_KEY.with(|l| l.set(before));
        out
    }

    pub(super) fn id_in_key() -> bool {
        ID_IN_KEY.with(Cell::get)
    }

    thread_local! {
        static STANDARD_OFF: Cell<bool> = const { Cell::new(false) };
    }

    /// `f` run with no node wrapped in the standard treatment.
    pub fn without_standard_treatment<T>(f: impl FnOnce() -> T) -> T {
        let before = STANDARD_OFF.with(|l| l.replace(true));
        let out = f();
        STANDARD_OFF.with(|l| l.set(before));
        out
    }

    pub(super) fn standard_off() -> bool {
        STANDARD_OFF.with(Cell::get)
    }

    thread_local! {
        static MARKER_IGNORED: Cell<bool> = const { Cell::new(false) };
    }

    /// `f` run with no source's `handles_validity();` read, so every colour and brightness is wrapped.
    pub fn ignoring_the_marker<T>(f: impl FnOnce() -> T) -> T {
        let before = MARKER_IGNORED.with(|l| l.replace(true));
        let out = f();
        MARKER_IGNORED.with(|l| l.set(before));
        out
    }

    pub(super) fn marker_ignored() -> bool {
        MARKER_IGNORED.with(Cell::get)
    }
}
