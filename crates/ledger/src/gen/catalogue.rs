//! The debug catalogue (render contract Part 6; debug_tooling_plan "Principle", §B–E; dd_generation_root §1, seam 13):
//! one field view and one test per ledger field, generated from the layout table, so the catalogue is exhaustive by
//! construction. Each view is a WGSL colour occupant (gui_state_contract §3), `present(unpack(ctx))`, written to
//! [`DIR`] as `<field>.wgsl`, which the registry's scan surfaces as a debug occupant (RQ-219). Each test is a Rust
//! function in [`TESTS_PATH`], `catalogue_view_<field>`, which reads the field through the Rust twin of the accessor
//! the view reads it through (REQ-TOOL-017), so the shader and its test share one generated core.
//!
//! **The accessors.** The fragment reads a field through the generated read side (lowering Part 3a): a read-side
//! member `ctx.sample.<field>`, filled by `sample_read` through the payload §6 accessor or derived accessor; the host
//! reads the same member of the Rust read side's `SimState`, filled through the accessor's Rust twin. A field of the
//! word buffer's `.w` is read through its word accessor over the read side's `word`: `length` through `fgw_length_raw`,
//! `payload` through `fgw_payload` (payload §3, §6). [`read()`] says which, and [`Read::accessors`] names the
//! symbols, the ones the view and its test both reference.
//!
//! **Exhaustive by construction.** A field the fragment cannot read refuses generation, naming it ([`refused`]): a new
//! ledger field appears in the catalogue or generation fails (seam 13; REQ-GEN-011). [`PENDING`] lists the fields
//! whose fragment read awaits a decision, as [`crate::payload::PENDING`] does for the struct check.
//!
//! **The colouring is a placeholder** (TASK-M1-09 and TASK-M1-10 own it): a categorical field `dbg_cat` with its `n`,
//! a flag `dbg_flag`, a field whose range is closed at both ends `dbg_lin` over that range, and any other the literal
//! placement of `dbg_sentinel`, which needs no range (render contract Part 5). A vector field shows its norm, `‖·‖`,
//! the reduction §3.8 names (applied per R-369). The scale and range written in each view's header are the ledger's.

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::{read, rust, Generated};
use crate::schema::{Bound, Entry, FieldType, Location, Scale, Storage, Struct, Word};

/// Where the views are written, relative to the workspace root: beside the generated unpack layer and read side in
/// `crates/render/frag/generated/` (RQ-219).
pub const DIR: &str = "crates/render/frag/debug/generated";

/// Where the views' tests are written, relative to the workspace root: a test of the crate that holds the accessors'
/// Rust twins, the kernel (the render crate depends on the ledger alone, systems_architecture §7.1). The file is the
/// `generated` module of the kernel's `catalogue_views` test target (`tests/catalogue_views/main.rs`), so the
/// exclusion of every file named `generated.rs` from mutation covers it (R-196).
pub const TESTS_PATH: &str = "crates/kernel/tests/catalogue_views/generated.rs";

/// The word buffer's `.w` (payload §3).
const FGW_WORD: &str = "fgw_w";

/// Fields whose fragment read awaits a decision, so [`refused`] passes over them and the catalogue has no view of
/// them yet: `ICDescriptor`'s twelve, which render contract Part 6 reads as `ctx.ic`, a member the stain's `Ctx` does
/// not have, and the Benettin shadow `r_sh` and `p_sh`, which the read side does not hold (lowering Part 3a).
pub const PENDING: &[&str] = &[
    "r_sh",
    "p_sh",
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// How a field is read, in the fragment and on the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    /// A scalar member of the read-side `SimState` under the field's name, of WGSL type `wgsl` (`f32`, `u32` or
    /// `bool`); `derived` when the read side computes it rather than unpacking it (payload §5).
    Member { wgsl: &'static str, derived: bool },
    /// A vector member of the read-side `SimState` under the field's name, `array<vec2<f32>, 3>` (R-86).
    Vector,
    /// A field of the word buffer's `.w`, through its word accessor `accessor` over the read side's `word`.
    Word { accessor: &'static str },
    /// A stored member of `SimStateFTLE` the read side does not hold: the Benettin shadow (lowering Part 3a).
    Stored { storage: Storage },
    /// A member of `ICDescriptor` (dd_generation_root §3.6).
    Ic,
}

/// A symbol a view and its test both reference: a member of the read-side `SimState`, or a generated function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Accessor {
    Member(String),
    Function(String),
}

impl Read {
    /// Whether the fragment reads the field through the read side, so the catalogue has a view of it.
    pub fn in_fragment(&self) -> bool {
        matches!(self, Read::Member { .. } | Read::Vector | Read::Word { .. })
    }

    /// The accessor symbols the field's view and its test reference.
    pub fn accessors(&self, field: &str) -> Vec<Accessor> {
        match self {
            Read::Member { .. } | Read::Vector => vec![Accessor::Member(field.to_owned())],
            Read::Word { accessor, .. } => vec![
                Accessor::Member("word".to_owned()),
                Accessor::Function((*accessor).to_owned()),
            ],
            Read::Stored { .. } | Read::Ic => Vec::new(),
        }
    }

    /// The field's value in WGSL, in a colour occupant whose context is `ctx` (render contract Part 1).
    pub fn wgsl(&self, field: &str) -> String {
        match self {
            Read::Member { .. } | Read::Vector => format!("ctx.sample.{field}"),
            Read::Word { accessor } => format!("{accessor}(ctx.sample.word)"),
            Read::Stored { .. } | Read::Ic => String::new(),
        }
    }

    /// The field's value in Rust, from the read-side `SimState` `read`, the stored sample `s` and its `ICDescriptor`
    /// `ic`.
    pub fn rust(&self, field: &str) -> String {
        match self {
            Read::Member { .. } | Read::Vector => format!("read.{field}"),
            Read::Word { accessor } => format!("{accessor}(read.word)"),
            Read::Stored { .. } => format!("s.{field}"),
            Read::Ic => format!("ic.{field}"),
        }
    }
}

/// The structs a field may be stored in: the full tier's `SimState` and `ICDescriptor`.
fn stores() -> (Struct, Struct) {
    let structs = crate::payload::structs();
    let pick = |name: &str| {
        structs
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .unwrap_or_else(|| Struct {
                name: "",
                align: 4,
                buffer: None,
                indexed: false,
                members: Vec::new(),
            })
    };
    (pick("SimStateFTLE"), pick("ICDescriptor"))
}

/// How `e` is read, or `None` if neither the fragment nor the host has a read of it: a read-side member of its name;
/// a field of the word buffer's `.w` with a word accessor (`length`, `payload`); a stored `SimStateFTLE` member; or an
/// `ICDescriptor` member.
pub fn read(words: &[Word], entries: &[Entry], e: &Entry) -> Option<Read> {
    if let Some(m) = read::members(words, entries)
        .into_iter()
        .find(|m| m.name == e.name)
    {
        let derived = m.fill == read::Fill::Derived;
        return match m.wgsl {
            "f32" | "u32" | "bool" => Some(Read::Member {
                wgsl: m.wgsl,
                derived,
            }),
            "array<vec2<f32>, 3>" => Some(Read::Vector),
            _ => None,
        };
    }
    if let Location::Packed { word, .. } = e.location {
        return match (word, e.name) {
            (FGW_WORD, "length") => Some(Read::Word {
                accessor: "fgw_length_raw",
            }),
            (FGW_WORD, "payload") => Some(Read::Word {
                accessor: "fgw_payload",
            }),
            _ => None,
        };
    }
    let (simstate, ic) = stores();
    if let Some(m) = simstate.members.iter().find(|m| m.name == e.name) {
        return Some(Read::Stored { storage: m.storage });
    }
    ic.members
        .iter()
        .any(|m| m.name == e.name)
        .then_some(Read::Ic)
}

/// One view of the catalogue: its field, how it is read, and its WGSL file.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub field: &'static str,
    pub read: Read,
    pub path: PathBuf,
    pub wgsl: String,
}

impl View {
    /// The name of the view's test in [`TESTS_PATH`]: `catalogue_view_<field>`, lowercased.
    pub fn test(&self) -> String {
        test_name(self.field)
    }
}

/// The test of `field`'s view: `catalogue_view_<field>`, lowercased, as a Rust function name is.
pub fn test_name(field: &str) -> String {
    format!("catalogue_view_{}", field.to_lowercase())
}

/// The catalogue: a view of each entry the fragment reads ([`Read::in_fragment`]), in the ledger's order. Empty for a
/// layout that declares none of the payload structs' members, as [`rust::emit`] writes nothing for one.
pub fn views(words: &[Word], entries: &[Entry]) -> Vec<View> {
    if !rust::declares(&crate::payload::structs(), words, entries) {
        return Vec::new();
    }
    entries
        .iter()
        .filter_map(|e| {
            let read = read(words, entries, e).filter(Read::in_fragment)?;
            Some(View {
                field: e.name,
                path: PathBuf::from(format!("{DIR}/{}.wgsl", e.name)),
                wgsl: view_wgsl(e, &read),
                read,
            })
        })
        .collect()
}

/// A line for each entry the fragment has no read of and [`PENDING`] does not list: generation refuses it, naming the
/// field (dd_generation_root §3.8: coverage is enforced, not hoped for).
pub fn refused(words: &[Word], entries: &[Entry]) -> Vec<String> {
    if !rust::declares(&crate::payload::structs(), words, entries) {
        return Vec::new();
    }
    entries
        .iter()
        .filter(|e| !PENDING.contains(&e.name))
        .filter(|e| !read(words, entries, e).is_some_and(|r| r.in_fragment()))
        .map(|e| {
            format!(
                "field `{}` has no debug view: the fragment's read side has no read of it (render contract Part 6; \
                 dd_generation_root §4, seam 13)",
                e.name
            )
        })
        .collect()
}

/// The emitter: each view's WGSL file, then the views' tests.
pub fn emit(words: &[Word], entries: &[Entry]) -> Vec<Generated> {
    let views = views(words, entries);
    if views.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Generated> = views
        .iter()
        .map(|v| Generated {
            path: v.path.clone(),
            contents: v.wgsl.clone(),
        })
        .collect();
    out.push(Generated {
        path: PathBuf::from(TESTS_PATH),
        contents: tests(entries, &views),
    });
    out
}

/// A finite end of a range, or `None`.
fn closed(b: Bound) -> Option<f64> {
    match b {
        Bound::Closed(v) if v.is_finite() => Some(v),
        _ => None,
    }
}

/// `b` as text for a view's header.
fn bound(b: Bound, lo: bool) -> String {
    match (b, lo) {
        (Bound::Closed(v), true) => format!("[{v}"),
        (Bound::Closed(v), false) => format!("{v}]"),
        (Bound::Open(v), true) => format!("({v}"),
        (Bound::Open(v), false) => format!("{v})"),
        (Bound::Unbounded, true) => "(−∞".to_owned(),
        (Bound::Unbounded, false) => "∞)".to_owned(),
    }
}

/// `x` as a WGSL f32 literal.
fn float(x: f64) -> String {
    format!("{x:?}")
}

/// The placeholder colouring of `value`, a WGSL expression of the field's type `wgsl`, by the field's scale and range.
fn ramp(e: &Entry, value: &str, wgsl: &str) -> String {
    let as_f32 = if wgsl == "f32" {
        value.to_owned()
    } else {
        format!("f32({value})")
    };
    match e.scale {
        Scale::Categorical(n) => format!("dbg_cat({value}, {n}u)"),
        Scale::Flag if wgsl == "bool" => format!("dbg_flag({value})"),
        Scale::Flag => format!("dbg_flag({value} != 0u)"),
        _ => match (closed(e.range.lo), closed(e.range.hi)) {
            (Some(lo), Some(hi)) => format!("dbg_lin({as_f32}, {}, {})", float(lo), float(hi)),
            _ => format!("dbg_sentinel({as_f32}, ctx.frag_xy)"),
        },
    }
}

/// The view's WGSL: a header naming the field, its location, type, scale and range, and its accessors, then `colour`.
fn view_wgsl(e: &Entry, r: &Read) -> String {
    let location = match &e.location {
        Location::Packed {
            word,
            offset,
            width,
        } => format!("`{word}` bits {offset}–{}", offset + width - 1),
        Location::Scalar(k) => format!("scalar index {k}"),
        Location::Derived { from } => format!("derived from `{}`", from.join("`, `")),
    };
    let ty = match &e.ty {
        FieldType::UBits => "u-bits".to_owned(),
        FieldType::F32 => "f32".to_owned(),
        FieldType::F16Pair => "f16-pair".to_owned(),
        FieldType::Fixed16 => "fixed16".to_owned(),
        FieldType::Vector { k, .. } => format!("vector(·, {k})"),
    };
    let scale = match e.scale {
        Scale::Lin => "lin".to_owned(),
        Scale::Log => "log".to_owned(),
        Scale::Cyclic => "cyclic".to_owned(),
        Scale::Diverging => "diverging".to_owned(),
        Scale::Categorical(n) => format!("categorical({n})"),
        Scale::Flag => "flag".to_owned(),
    };
    let value = r.wgsl(e.name);
    let body = match r {
        Read::Vector => format!(
            "let v = {value};\n    return dbg_sentinel(sqrt(dot(v[0], v[0]) + dot(v[1], v[1]) + dot(v[2], v[2])), \
             ctx.frag_xy);"
        ),
        Read::Member { wgsl, .. } => format!("return {};", ramp(e, &value, wgsl)),
        _ => format!("return {};", ramp(e, &value, "u32")),
    };
    let accessors: Vec<String> = r
        .accessors(e.name)
        .iter()
        .map(|a| match a {
            Accessor::Member(m) => format!("`SimState.{m}`"),
            Accessor::Function(f) => format!("`{f}`"),
        })
        .collect();
    let header = format!(
        "The debug view of `{name}` (render contract Part 6; debug_tooling_plan §B–E): {location}, {ty}, scale {scale}, \
         range {lo}, {hi}. A colour occupant, `present(unpack(ctx))` (gui_state_contract §3), it reads the field \
         through {acc}; its test, `{test}` in `{TESTS_PATH}`, reads it through their Rust twins. The colouring is a \
         placeholder (`ledger::gen::catalogue`).",
        name = e.name,
        lo = bound(e.range.lo, true),
        hi = bound(e.range.hi, false),
        acc = accessors.join(" and "),
        test = test_name(e.name),
    );
    format!(
        "// Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.\n\
         {}fn colour(ctx: Ctx) -> vec3<f32> {{\n    {body}\n}}\n",
        comment(&header, "// ")
    )
}

/// `text` as line comments led by `lead`, wrapped at 120 columns.
fn comment(text: &str, lead: &str) -> String {
    let mut out = String::new();
    let mut line = String::from(lead);
    for word in text.split(' ') {
        if line.chars().count() + word.chars().count() > 120 && line.len() > lead.len() {
            out.push_str(line.trim_end());
            out.push('\n');
            line = String::from(lead);
        }
        line.push_str(word);
        line.push(' ');
    }
    out.push_str(line.trim_end());
    out.push('\n');
    out
}

// ── The views' tests ─────────────────────────────────────────────────────────────────────────────────────────────

/// rustfmt's layout of the statement `<lead><f>(<args>);` at `indent` spaces (its defaults: 100 columns, a call's
/// arguments on one line only within 60): on one line when both hold, else one argument a line.
fn call(indent: usize, lead: &str, f: &str, args: &[String]) -> String {
    let pad = " ".repeat(indent);
    let joined = args.join(", ");
    let one = format!("{pad}{lead}{f}({joined});");
    if joined.chars().count() <= 60 && one.chars().count() <= 100 {
        return one;
    }
    let inner: String = args.iter().map(|a| format!("{pad}    {a},\n")).collect();
    format!("{pad}{lead}{f}(\n{inner}{pad});")
}

/// The probe value of a stored field, as a Rust literal, and the statement writing it, or its altered value, into the
/// probe.
struct Probe {
    value: String,
    write: String,
}

/// A float for the probe: f32-exact, and f16-exact for an `f16-pair` (multiples of 1/32 below 2048).
fn probe_float(k: usize, j: usize) -> f64 {
    0.25 * (k + 1) as f64 + 0.03125 * j as f64
}

/// The vector literal `[[f32; 2]; 3]` of entry `k`, its first component raised by `bump`.
fn probe_vector(k: usize, bump: f64) -> String {
    let c: Vec<String> = (0..6)
        .map(|j| float(probe_float(k, j) + if j == 0 { bump } else { 0.0 }))
        .collect();
    format!(
        "[[{}, {}], [{}, {}], [{}, {}]]",
        c[0], c[1], c[2], c[3], c[4], c[5]
    )
}

/// The probe's unsigned value for a `width`-bit field whose range ends at `hi`: the middle of the range, rounded up,
/// so a flag is set; and its altered value, one more, wrapped within the width.
fn probe_bits(hi: Bound, width: u32) -> (u64, u64) {
    let mask = u64::MAX >> (64 - width.clamp(1, 64));
    let top = match hi {
        Bound::Closed(v) if v.is_finite() && v >= 0.0 => (v as u64).min(mask),
        _ => mask,
    };
    let v = top.div_ceil(2).max(1);
    (v, (v + 1) & mask)
}

/// How entry `k`, `e`, is written into the probe, or `None` for a derived field, which is not stored.
fn probe(e: &Entry, k: usize, simstate: &Struct, ic: &Struct) -> Option<Probe> {
    let n = e.name;
    let pick = |value: &str, altered: &str| format!("pick(alter, \"{n}\", {value}, {altered})");
    let floats = |v: f64| (float(v), float(v + 0.25));
    match &e.location {
        Location::Derived { .. } => None,
        Location::Packed {
            word,
            offset,
            width,
        } => {
            let target = if *word == FGW_WORD {
                "word[3]".to_owned()
            } else {
                format!("s.{word}")
            };
            let (value, bits) = if e.ty == FieldType::F16Pair {
                let (v, a) = floats(probe_float(k, 0));
                let bits = format!("u32::from(f32_to_f16_bits({}))", pick(&v, &a));
                (v, bits)
            } else {
                let (v, a) = probe_bits(e.range.hi, *width);
                (v.to_string(), pick(&v.to_string(), &a.to_string()))
            };
            let args = [target.clone(), bits, offset.to_string(), width.to_string()];
            Some(Probe {
                write: call(4, &format!("{target} = "), "insert", &args),
                value,
            })
        }
        Location::Scalar(_) => {
            let (owner, member) = simstate
                .members
                .iter()
                .map(|m| ("s", m))
                .chain(ic.members.iter().map(|m| ("ic", m)))
                .find(|(_, m)| m.name == n)?;
            let (value, altered) = match member.storage {
                Storage::Vec2x3 => (probe_vector(k, 0.0), probe_vector(k, 0.25)),
                Storage::F32 => floats(probe_float(k, 0)),
                storage => {
                    let width = if storage == Storage::U16 { 16 } else { 32 };
                    let (v, a) = probe_bits(e.range.hi, width);
                    (v.to_string(), a.to_string())
                }
            };
            let args = [
                "alter".to_owned(),
                format!("\"{n}\""),
                value.clone(),
                altered,
            ];
            Some(Probe {
                write: call(4, &format!("{owner}.{n} = "), "pick", &args),
                value,
            })
        }
    }
}

/// The derived accessor's expression for `field` over an unaltered probe, `reference`, after the locals it reads, as
/// the read side computes them (payload §5).
fn derived_locals(field: &str) -> (String, String) {
    let expr = read::rust_derived(field, true);
    let mut locals = vec![
        "let reference = probe(None);".to_owned(),
        "let s = &reference.s;".to_owned(),
    ];
    if expr.contains("params") {
        locals.push("let params = &reference.params;".to_owned());
    }
    if expr.contains("masses") {
        locals.push("let masses = [reference.ic.m0, reference.ic.m1, reference.ic.m2];".to_owned());
    }
    if expr.contains(" n,") || expr.contains("ftle_ok") {
        locals.push("let n = tm_t_end_step(s.times);".to_owned());
    }
    if expr.contains("ftle_ok") {
        locals.push(
            "let ftle_ok = ftle_valid(s.packed_a, true, n, completed_renorms(n, params.n_renorm));"
                .to_owned(),
        );
    }
    if expr.contains("delta,") {
        locals.push("let delta = benettin_delta(s.r, s.p, s.r_sh, s.p_sh);".to_owned());
    }
    let locals: String = locals.iter().map(|l| format!("    {l}\n")).collect();
    (locals, expr)
}

/// The message a check fails with when the field's read is not the probe's value, and when a derived field's is not
/// its derived accessor's.
const NOT_STORED: &str = "does not read back the value stored";
const NOT_DERIVED: &str = "is not its derived accessor's value";

/// The check of view `v`'s field: read through the Rust twin of its view's accessor from `p`, it is the value the
/// probe stored, or, for a derived field, the value its derived accessor gives from an unaltered probe.
fn check(v: &View, stored: Option<&Probe>) -> String {
    let n = v.field;
    let got = v.read.rust(n);
    let lower = n.to_lowercase();
    let message = |what: &str| format!("\"`{n}` {what}\"");
    let (doc, body) = match (&v.read, stored) {
        (Read::Member { derived: true, .. }, _) => {
            let (locals, expr) = derived_locals(n);
            let args = [
                format!("{got}.to_bits()"),
                "expected.to_bits()".to_owned(),
                message(NOT_DERIVED),
            ];
            (
                "is the value its derived accessor gives from the unaltered probe",
                format!(
                    "    let read = read(p);\n{locals}    let expected: f32 = {expr};\n{}",
                    call(4, "", "expect", &args)
                ),
            )
        }
        (_, Some(p)) => {
            let (got, want) = match &v.read {
                Read::Member { wgsl: "f32", .. } => (
                    format!("{got}.to_bits()"),
                    format!("{}_f32.to_bits()", p.value),
                ),
                Read::Member { wgsl: "bool", .. } => (got, (p.value != "0").to_string()),
                _ => (got, p.value.clone()),
            };
            (
                "is the value the probe stored",
                format!(
                    "    let read = read(p);\n{}",
                    call(4, "", "expect", &[got, want, message(NOT_STORED)])
                ),
            )
        }
        (_, None) => (
            "has no probe value",
            format!("    compile_error!(\"`{n}` is neither stored nor derived\");"),
        ),
    };
    format!(
        "\n/// `{n}`, read through the Rust twin of its view's accessor, {doc}.\nfn check_{lower}(p: &Probe) {{\n{body}\n}}\n"
    )
}

/// The field whose alteration the negative control of `e`'s test stores: the field itself, or, for a derived field,
/// the first stored field it is derived from.
fn altered_field(e: &Entry) -> &'static str {
    match &e.location {
        Location::Derived { from } => from.first().copied().unwrap_or(e.name),
        _ => e.name,
    }
}

/// The views' tests: the probe, the read, and a check, a test and a negative control per view.
fn tests(entries: &[Entry], views: &[View]) -> String {
    let (simstate, ic) = stores();
    let probes: Vec<Option<Probe>> = entries
        .iter()
        .enumerate()
        .map(|(k, e)| probe(e, k, &simstate, &ic))
        .collect();
    let mut out = String::from(
        "//! Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.\n\
         //! The debug catalogue's tests (debug_tooling_plan \"Principle\"; render contract Part 6), one per view in\n\
         //! `crates/render/frag/debug/generated/`: each reads its field from a synthetic sample through the Rust twin of\n\
         //! the accessor its view reads it through (REQ-TOOL-017, REQ-TOOL-020), and its negative control stores another\n\
         //! value.\n\n\
         use kernel::payload::*;\n\
         use validation::negative_control;\n\n\
         /// A synthetic sample: the stored `SimStateFTLE`, its word, its `ICDescriptor` and the read's parameters.\n\
         struct Probe {\n    s: SimStateFTLE,\n    word: [u32; 4],\n    ic: ICDescriptor,\n    params: ReadParams,\n}\n\n\
         /// `altered` when `alter` names `field`, else `value`.\n\
         fn pick<T>(alter: Option<&str>, field: &str, value: T, altered: T) -> T {\n    \
         if alter == Some(field) {\n        altered\n    } else {\n        value\n    }\n}\n\n\
         /// Asserts that the field read, `got`, is `want`, failing with `what`.\n\
         fn expect<T: PartialEq + std::fmt::Debug>(got: T, want: T, what: &str) {\n    \
         assert_eq!(got, want, \"{what}\");\n}\n\n\
         /// The probe: every stored field at its own value, the one `alter` names at another.\n\
         fn probe(alter: Option<&str>) -> Probe {\n    \
         let mut s = SimStateFTLE::default();\n    let mut word = [0u32; 4];\n    let mut ic = ICDescriptor::default();\n",
    );
    for p in probes.iter().flatten() {
        out.push_str(&p.write);
        out.push('\n');
    }
    out.push_str(
        "    let params = ReadParams {\n        dt_macro: 0.01,\n        delta_0: 1e-6,\n        n_renorm: 16,\n        \
         horizon_steps: 65535,\n    };\n    Probe {\n        s,\n        word,\n        ic,\n        params,\n    }\n}\n\n\
         /// `p` through the read side, the Rust twin of the fragment's (lowering Part 3a): its word bound, E = 0, its own\n\
         /// masses.\n\
         fn read(p: &Probe) -> SimState {\n    sim_state_from_ftle(\n        &p.s,\n        p.word,\n        true,\n        \
         canonical_nan(),\n        false,\n        [p.ic.m0, p.ic.m1, p.ic.m2],\n        &p.params,\n    )\n}\n",
    );
    for v in views {
        let Some((k, e)) = entries.iter().enumerate().find(|(_, e)| e.name == v.field) else {
            continue;
        };
        out.push_str(&check(v, probes[k].as_ref()));
        let test = v.test();
        let lower = v.field.to_lowercase();
        let alter = altered_field(e);
        let expected = match v.read {
            Read::Member { derived: true, .. } => NOT_DERIVED,
            _ => NOT_STORED,
        };
        let _ = write!(
            out,
            "\n#[test]\nfn {test}() {{\n    check_{lower}(&probe(None));\n}}\n\n\
             negative_control!(\n    {test},\n    \"a probe storing another `{alter}` reads another `{n}`\",\n    \
             expected = \"{expected}\",\n    check_{lower}(&probe(Some(\"{alter}\")))\n);\n",
            n = v.field,
        );
    }
    out
}
