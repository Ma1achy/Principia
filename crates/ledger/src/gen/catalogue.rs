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
//! `payload` through `fgw_payload` (payload §3, §6). An `ICDescriptor` field is read as `ctx.ic.<field>`, which the
//! read side's `ic_read` fills through `ic_read_<field>` (render contract Part 6; RQ-227); its test reads the same
//! member of the stored `ICDescriptor`. [`read()`] says which, and [`Read::accessors`] names the symbols, the ones the
//! view and its test both reference.
//!
//! **Exhaustive by construction.** A field the fragment cannot read refuses generation, naming it ([`refused`]): a new
//! ledger field appears in the catalogue or generation fails (seam 13; REQ-GEN-011).
//!
//! **The colouring.** A numeric field takes the two-line template ([`numeric`](super::numeric); render_gui_spec §10.1,
//! RQ-231, TASK-M1-09), its `RANGE_AUTO` and `u_range` uniforms declared in the view's header. `state` takes the
//! six-colour `dbg_cat` palette (R-115), and `detail`, a union, is coloured per state, its palette segment and legend
//! keyed by `state` ([`detail_segments`]; TASK-M1-10). The rest is a
//! placeholder (TASK-M1-12 owns it): a categorical field `dbg_cat` with its `n`, a flag `dbg_flag`, and
//! the drift fields, which keep R-381's `symlog` default until TASK-M3-05, the literal placement of `dbg_sentinel`,
//! which needs no range (render contract Part 5). A vector field's view shows its norm, `‖·‖`, the reduction §3.8 names
//! (applied per R-369). A categorical field's stored sentinel, `dmin_pair`'s 3, shows as its literal value on the ramp
//! through `dbg_sentinel`, never as a class (R-136). `last_symbol`, which has no in-band "none" code, is gated on
//! the word's sidecar length ([`WORD_GATED`]; payload §2): the empty word and a truncated word draw the hatch. The
//! scale and range written in each view's header are the ledger's.
//!
//! **The reductions** (gui_state_contract §4; dd_generation_root §3.8; render contract Part 5; TASK-M1-12). A field's
//! view is one per field; the views that reduce a field another way, or combine fields, are written beside them to
//! [`REDUCTIONS_DIR`], which the registry scans too ([`reductions`]):
//! - for every `vector(type, k)` field, '‖·‖ as scalar' is its view, and 'as direction-cosines' is
//!   `<field>_dircos.wgsl`: for `k = 3` (`n`) `dbg_dircos3`, `½(v̂ + 1)` in RGB; for `k = 6` (`r`, `p`, the shadow)
//!   `dbg_dircos6`, the squared cosines summed per body (REQ-TOOL-157). A vector the presentation layer has no
//!   direction-cosines rendering for refuses generation, naming it ([`refused`]);
//! - the `ICDescriptor` masses `m0 m1 m2` as one ternary colour, `masses_ternary.wgsl`, `dbg_ternary`
//!   (REQ-TOOL-154; debug_tooling_plan §E).

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::numeric::NumericView;
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

/// Where the reductions are written, relative to the workspace root: beside [`DIR`], not in it, since that directory
/// holds exactly one view per field (REQ-TOOL-020).
pub const REDUCTIONS_DIR: &str = "crates/render/frag/debug/reductions";

/// The WGSL type of a stored vector, `vector(f32, 6)` as three `vec2<f32>` (R-86).
pub const VEC2X3: &str = "array<vec2<f32>, 3>";

/// The WGSL type of a derived 3-vector, `n` (dd_generation_root §3.8).
pub const VEC3: &str = "vec3<f32>";

/// The `ICDescriptor` masses the ternary view combines, in channel order (dd_generation_root §3.6; debug_tooling_plan
/// §E).
pub const MASSES: [&str; 3] = ["m0", "m1", "m2"];

/// The ternary masses view's name, its file `masses_ternary.wgsl` under [`REDUCTIONS_DIR`].
pub const MASSES_TERNARY: &str = "masses_ternary";

/// The word buffer's `.w` (payload §3).
const FGW_WORD: &str = "fgw_w";

/// How a field is read, in the fragment and on the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    /// A scalar member of the read-side `SimState` under the field's name, of WGSL type `wgsl` (`f32`, `u32` or
    /// `bool`); `derived` when the read side computes it rather than unpacking it (payload §5).
    Member { wgsl: &'static str, derived: bool },
    /// A vector member of the read-side `SimState` under the field's name, of Rust type `rust` and WGSL type `wgsl`:
    /// stored, [`VEC2X3`] (R-86), or derived, `n`'s [`VEC3`].
    Vector {
        rust: &'static str,
        wgsl: &'static str,
        derived: bool,
    },
    /// A field of the word buffer's `.w`, through its word accessor `accessor` over the read side's `word`.
    Word { accessor: &'static str },
    /// A member of the sample's `ICDescriptor` (dd_generation_root §3.6), `ctx.ic.<field>` (RQ-227).
    Ic,
}

/// A symbol a view and its test both reference: a member of the read-side `SimState`, a member of `ICDescriptor`, or
/// a generated function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Accessor {
    Member(String),
    IcMember(String),
    Function(String),
}

impl Read {
    /// The accessor symbols the field's view and its test reference.
    pub fn accessors(&self, field: &str) -> Vec<Accessor> {
        match self {
            Read::Member { .. } | Read::Vector { .. } => vec![Accessor::Member(field.to_owned())],
            Read::Word { accessor, .. } => vec![
                Accessor::Member("word".to_owned()),
                Accessor::Function((*accessor).to_owned()),
            ],
            Read::Ic => vec![Accessor::IcMember(field.to_owned())],
        }
    }

    /// The field's value in WGSL, in a colour occupant whose context is `ctx` (render contract Part 1).
    pub fn wgsl(&self, field: &str) -> String {
        match self {
            Read::Member { .. } | Read::Vector { .. } => format!("ctx.sample.{field}"),
            Read::Word { accessor } => format!("{accessor}(ctx.sample.word)"),
            Read::Ic => format!("ctx.ic.{field}"),
        }
    }

    /// The field's value in Rust, from the read-side `SimState` `read` and the sample's `ICDescriptor` `ic`.
    pub fn rust(&self, field: &str) -> String {
        match self {
            Read::Member { .. } | Read::Vector { .. } => format!("read.{field}"),
            Read::Word { accessor } => format!("{accessor}(read.word)"),
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
/// a field of the word buffer's `.w` with a word accessor (`length`, `payload`); or an `ICDescriptor` member.
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
            VEC2X3 | VEC3 => Some(Read::Vector {
                rust: m.rust,
                wgsl: m.wgsl,
                derived,
            }),
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
    let (_, ic) = stores();
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

    /// The accessor symbols the view and its test reference: its field's ([`Read::accessors`]); for the union
    /// [`UNION_FIELD`], its key's member [`UNION_KEY`], which the view switches on; and for [`WORD_GATED`], the word
    /// and the two word accessors its gate reads.
    pub fn accessors(&self) -> Vec<Accessor> {
        let mut out = self.read.accessors(self.field);
        if self.field == UNION_FIELD {
            out.push(Accessor::Member(UNION_KEY.to_owned()));
        }
        if self.field == WORD_GATED {
            out.push(Accessor::Member("word".to_owned()));
            out.extend(WORD_GATE.map(|f| Accessor::Function(f.to_owned())));
        }
        out
    }
}

/// The test of `field`'s view: `catalogue_view_<field>`, lowercased, as a Rust function name is.
pub fn test_name(field: &str) -> String {
    format!("catalogue_view_{}", field.to_lowercase())
}

/// The catalogue: a view of each entry the fragment reads ([`read()`]), in the ledger's order. Empty for a
/// layout that declares none of the payload structs' members, as [`rust::emit`] writes nothing for one.
pub fn views(words: &[Word], entries: &[Entry]) -> Vec<View> {
    if !rust::declares(&crate::payload::structs(), words, entries) {
        return Vec::new();
    }
    entries
        .iter()
        .filter_map(|e| {
            let read = read(words, entries, e)?;
            Some(View {
                field: e.name,
                path: PathBuf::from(format!("{DIR}/{}.wgsl", e.name)),
                wgsl: view_wgsl(words, entries, e, &read),
                read,
            })
        })
        .collect()
}

/// A line for each entry the fragment has no read of, and for each vector field the presentation layer has no
/// direction-cosines rendering of: generation refuses it, naming the field (dd_generation_root §3.8: coverage is
/// enforced, not hoped for; gui_state_contract §4: every vector offers both reductions).
pub fn refused(words: &[Word], entries: &[Entry]) -> Vec<String> {
    if !rust::declares(&crate::payload::structs(), words, entries) {
        return Vec::new();
    }
    entries
        .iter()
        .filter_map(|e| match read(words, entries, e) {
            None => Some(format!(
                "field `{}` has no debug view: the fragment's read side has no read of it (render contract Part 6; \
                 dd_generation_root §4, seam 13)",
                e.name
            )),
            Some(r) if is_vector(e) && dircos(&r).is_none() => Some(format!(
                "vector field `{}` has no direction-cosines view: the presentation layer renders the direction \
                 cosines of a `vec3<f32>` or a `{VEC2X3}` only (gui_state_contract §4; render contract Part 5)",
                e.name
            )),
            Some(_) => None,
        })
        .collect()
}

/// Whether `e` is a `vector(type, k)` field (dd_generation_root §3.8).
fn is_vector(e: &Entry) -> bool {
    matches!(e.ty, FieldType::Vector { .. })
}

/// The presentation layer's direction-cosines rendering of a vector read as `r`, or `None` for any other read: `n`'s
/// `k = 3` as `dbg_dircos3`, a stored `k = 6` as `dbg_dircos6` (render contract Part 5; REQ-TOOL-157).
fn dircos(r: &Read) -> Option<&'static str> {
    match r {
        Read::Vector { wgsl: VEC3, .. } => Some("dbg_dircos3"),
        Read::Vector { wgsl: VEC2X3, .. } => Some("dbg_dircos6"),
        _ => None,
    }
}

/// One reduction view: its name, its file under [`REDUCTIONS_DIR`], the fields it reads and its WGSL.
#[derive(Clone, Debug, PartialEq)]
pub struct Reduction {
    /// The file's stem: `<field>_dircos` or [`MASSES_TERNARY`].
    pub name: String,
    /// The fields it reads, in order.
    pub fields: Vec<&'static str>,
    pub path: PathBuf,
    pub wgsl: String,
}

/// The reductions of `entries` ([`REDUCTIONS_DIR`]): each vector field's 'as direction-cosines' view, in the ledger's
/// order, then the ternary masses view when the three masses are `ICDescriptor` fields. Empty for a layout that
/// declares none of the payload structs' members, as [`views`] is.
pub fn reductions(words: &[Word], entries: &[Entry]) -> Vec<Reduction> {
    if !rust::declares(&crate::payload::structs(), words, entries) {
        return Vec::new();
    }
    let mut out: Vec<Reduction> = entries
        .iter()
        .filter(|e| is_vector(e))
        .filter_map(|e| {
            let r = read(words, entries, e)?;
            let helper = dircos(&r)?;
            let name = format!("{}_dircos", e.name);
            let text = format!(
                "The 'as direction-cosines' view of the vector field `{field}` (gui_state_contract §4; \
                 dd_generation_root §3.8), beside its '‖·‖ as scalar' view, `{DIR}/{field}.wgsl`. A colour occupant, it \
                 reads the field through `SimState.{field}` and draws `{helper}`, render contract Part 5's rendering \
                 of its direction cosines (REQ-TOOL-157).",
                field = e.name
            );
            Some(Reduction {
                path: PathBuf::from(format!("{REDUCTIONS_DIR}/{name}.wgsl")),
                wgsl: format!(
                    "{}fn colour(ctx: Ctx) -> vec3<f32> {{\n    return {helper}({}, ctx.frag_xy);\n}}\n",
                    generated_comment(&text),
                    r.wgsl(e.name)
                ),
                name,
                fields: vec![e.name],
            })
        })
        .collect();
    let masses: Vec<String> = MASSES
        .iter()
        .filter_map(|m| {
            let e = entries.iter().find(|e| e.name == *m)?;
            (read(words, entries, e)? == Read::Ic).then(|| Read::Ic.wgsl(m))
        })
        .collect();
    if masses.len() == MASSES.len() {
        let text = "The `ICDescriptor` masses `m0 m1 m2` as one ternary colour (debug_tooling_plan §E; render contract \
                    Part 5): `dbg_ternary`, the masses as linear RGB scaled by `1/max(mᵢ)`, so equal masses draw white \
                    and each vertex its primary (REQ-TOOL-154). A colour occupant, it reads the masses through \
                    `ICDescriptor.m0`, `ICDescriptor.m1` and `ICDescriptor.m2`; it certifies the decoder, independent \
                    of any integration.";
        out.push(Reduction {
            name: MASSES_TERNARY.to_owned(),
            fields: MASSES.to_vec(),
            path: PathBuf::from(format!("{REDUCTIONS_DIR}/{MASSES_TERNARY}.wgsl")),
            wgsl: format!(
                "{}fn colour(ctx: Ctx) -> vec3<f32> {{\n    return dbg_ternary(vec3<f32>({}), ctx.frag_xy);\n}}\n",
                generated_comment(text),
                masses.join(", ")
            ),
        });
    }
    out
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
    out.extend(reductions(words, entries).into_iter().map(|r| Generated {
        path: r.path,
        contents: r.wgsl,
    }));
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
        Scale::Categorical(n) => match e.sentinel {
            // A stored sentinel shows as its literal value on the ramp, never as a class (R-136).
            Some(s) if s.is_finite() && s >= 0.0 => format!(
                "select(dbg_cat({value}, {n}u), dbg_sentinel({as_f32}, ctx.frag_xy), {value} == {}u)",
                s as u64
            ),
            _ => format!("dbg_cat({value}, {n}u)"),
        },
        Scale::Flag if wgsl == "bool" => format!("dbg_flag({value})"),
        Scale::Flag => format!("dbg_flag({value} != 0u)"),
        _ => match (closed(e.range.lo), closed(e.range.hi)) {
            (Some(lo), Some(hi)) => format!("dbg_lin({as_f32}, {}, {})", float(lo), float(hi)),
            _ => format!("dbg_sentinel({as_f32}, ctx.frag_xy)"),
        },
    }
}

/// The view's WGSL: a header naming the field, its location, type, scale and range, and its accessors, then `colour`:
/// the union's, keyed by its key's value, for [`UNION_FIELD`] ([`union_wgsl`]); the numeric template's
/// ([`NumericView`]) for a numeric field, its uniforms declared after the header; else the placeholder's.
fn view_wgsl(words: &[Word], entries: &[Entry], e: &Entry, r: &Read) -> String {
    if let Some(wgsl) = union_wgsl(words, entries, e, r) {
        return wgsl;
    }
    let numeric = match r {
        Read::Member {
            wgsl: "f32" | "u32",
            ..
        }
        | Read::Ic
        | Read::Word { .. } => NumericView::of(e),
        _ => None,
    };
    match numeric {
        Some(n) => numeric_wgsl(e, r, &n),
        None => placeholder_wgsl(e, r),
    }
}

/// The numeric view `n` of `e`, read as `r`: the header, the view's uniforms and the template's `colour`.
pub fn numeric_wgsl(e: &Entry, r: &Read, n: &NumericView) -> String {
    let value = r.wgsl(e.name);
    let as_f32 = match r {
        Read::Member { wgsl: "f32", .. } | Read::Ic => value,
        _ => format!("f32({value})"),
    };
    format!(
        "{}{}{}",
        header(
            e,
            r,
            "Its colouring is the numeric template (`ledger::gen::numeric`; render_gui_spec §10.1, RQ-231): the NaN \
             guard, the stored sentinel's line where the field has one, and the ramp, with `RANGE_AUTO` and `u_range` its \
             uniforms."
        ),
        n.header(),
        n.colour(&as_f32)
    )
}

/// The numeric view of `field` in `words` and `entries`, its `RANGE_AUTO` param set to `range_auto` where given (the
/// header default the generator writes from the node's param, RQ-231), or `None` for a field the template does not
/// colour.
pub fn numeric_view(
    words: &[Word],
    entries: &[Entry],
    field: &str,
    range_auto: Option<bool>,
) -> Option<String> {
    let e = entries.iter().find(|e| e.name == field)?;
    let r = read(words, entries, e)?;
    let mut n = NumericView::of(e)?;
    if let Some(on) = range_auto {
        n = n.with_range_auto(on);
    }
    Some(numeric_wgsl(e, &r, &n))
}

// ── The `detail` union's view, keyed by `state` (REQ-COL-004, REQ-TOOL-022) ───────────────────────────────────────

/// The union field, whose meaning is keyed by another field's value: `detail` (payload §2, "union keyed by state").
pub const UNION_FIELD: &str = "detail";

/// The field the union is keyed by: `state`.
pub const UNION_KEY: &str = "state";

/// One class of a state's segment of the `detail` view: the `detail` code, its meaning in that state (payload §2, as
/// the ledger hashes it, [`crate::payload::detail_meanings`]), its legend label, 0-based (R-22), and its class, the
/// index the view colours it by, `dbg_cat(class, detail_classes())`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailClass {
    pub detail: u32,
    pub meaning: &'static str,
    pub label: String,
    pub class: u32,
}

/// The `detail` view's palette segment and legend for one state (dd_colouring §3.7: "legend and palette segment
/// switch on `state`"): the state's name and code, and its four classes, one per `detail` code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailSegment {
    pub state: &'static str,
    pub code: u32,
    pub classes: Vec<DetailClass>,
}

/// The legend label of `detail` code `d`, meaning `meaning`, in the state `state`: a collision's pair id names its two
/// bodies, the lower first, as colour_composition §1.4's table does (R-22: pair `k` is the side opposite body `k`);
/// every other meaning is the ledger's.
fn detail_label(state: &str, d: usize, meaning: &str) -> String {
    match (state, crate::payload::pair_bodies().get(d)) {
        ("collision", Some(&[a, b])) => format!("{meaning}: bodies {}–{}", a.min(b), a.max(b)),
        _ => meaning.to_owned(),
    }
}

/// The `detail` view's segments, one per state in which `detail` has a meaning (payload §2: escape, collision,
/// sim_failed and decode_failed), in the ledger's order ([`crate::payload::detail_meanings`]): segment `k`'s classes
/// are `4k + 1 … 4k + 4`, so no two classes of the view share a colour. `bounded`, `running` and the reserved codes
/// 6–7 have none: `detail` is undefined there (payload §2), and the view draws them blank, black: class
/// [`no_detail`].
pub fn detail_segments() -> Vec<DetailSegment> {
    let states = crate::payload::states();
    crate::payload::detail_meanings()
        .iter()
        .enumerate()
        .filter_map(|(k, (state, meanings))| {
            let code = states.iter().position(|s| s == state)?;
            let classes = meanings
                .iter()
                .enumerate()
                .map(|(d, meaning)| DetailClass {
                    detail: d as u32,
                    meaning,
                    label: detail_label(state, d, meaning),
                    class: no_detail() + 1 + (meanings.len() * k + d) as u32,
                })
                .collect();
            Some(DetailSegment {
                state,
                code: code as u32,
                classes,
            })
        })
        .collect()
}

/// The segment of the state whose code is `state`, or `None` where `detail` has no meaning: `bounded`, `running` and
/// the reserved codes (payload §2).
pub fn detail_segment(state: u32) -> Option<DetailSegment> {
    detail_segments().into_iter().find(|s| s.code == state)
}

/// The class of a state with no `detail`, drawn blank, black, not by `dbg_cat`: class 0, so the segments' classes
/// start at 1, whose golden-angle colours all sit at least 0.02 of an 8-bit step from a rounding tie in linear RGB,
/// where class 0's green sits 0.005 from one (applied per R-369; `render/tests/numeric_views.rs`'s margin).
pub fn no_detail() -> u32 {
    0
}

/// The number of classes the `detail` view tells apart: [`no_detail`] and four per segment, the `n` of its
/// `dbg_cat(class, n)`.
pub fn detail_classes() -> u32 {
    1 + detail_segments()
        .iter()
        .map(|s| s.classes.len() as u32)
        .sum::<u32>()
}

/// The view of [`UNION_FIELD`], or `None` for any other entry, or where the key is not a field the fragment reads as a
/// `u32`: `colour` reads the key, and each state's segment colours `detail` by its class
/// ([`detail_segments`]), a state with no segment drawing blank, black. The legend the view is keyed by is the same
/// data, so the two cannot disagree (the three-colours-bug guard, dd_colouring §2).
fn union_wgsl(words: &[Word], entries: &[Entry], e: &Entry, r: &Read) -> Option<String> {
    if e.name != UNION_FIELD {
        return None;
    }
    let key = entries.iter().find(|k| k.name == UNION_KEY)?;
    let key_read = read(words, entries, key)?;
    if key_read
        != (Read::Member {
            wgsl: "u32",
            derived: false,
        })
    {
        return None;
    }
    let n = detail_classes();
    let mut body = format!(
        "    let s = {};\n    let d = {};\n",
        key_read.wgsl(key.name),
        r.wgsl(e.name)
    );
    for seg in detail_segments() {
        let first = seg.classes.first().map_or(0, |c| c.class);
        let _ = writeln!(
            body,
            "    if (s == STATE_{}) {{ return dbg_cat({first}u + d, {n}u); }}",
            seg.state.to_uppercase()
        );
    }
    body.push_str("    return vec3<f32>(0.0, 0.0, 0.0);\n");
    let segments: Vec<String> = detail_segments()
        .iter()
        .map(|s| {
            let labels: Vec<String> = s
                .classes
                .iter()
                .map(|c| format!("{} {}", c.detail, c.label))
                .collect();
            format!("{} ({})", s.state, labels.join(", "))
        })
        .collect();
    let colouring = format!(
        "Its colouring is the union's, keyed by `{UNION_KEY}` (debug_tooling_plan §B; dd_colouring §3.7; \
         `ledger::gen::catalogue::detail_segments`, the legend's data): each state's palette segment is four classes \
         of `dbg_cat(·, {n}u)`, one per code: {}. Every other state, bounded, running and the reserved codes, has no \
         `detail` and draws blank, black, class {} (payload §2).",
        segments.join("; "),
        no_detail()
    );
    Some(format!(
        "{}fn colour(ctx: Ctx) -> vec3<f32> {{\n{body}}}\n",
        header(e, r, &colouring)
    ))
}

/// The field whose validity gates on the word's sidecar length: `last_symbol`, which has no in-band "none" code and is
/// meaningful only for a nonempty word that is not truncated (payload §2): payload §6's `sd_last_symbol_valid` of the
/// word's `fgw_length_raw`, both generated from the ledger.
pub const WORD_GATED: &str = "last_symbol";

/// The generated accessors [`WORD_GATED`]'s gate reads (payload §6): the word's stored length, and `last_symbol`'s
/// validity from it.
const WORD_GATE: [&str; 2] = ["fgw_length_raw", "sd_last_symbol_valid"];

/// The field whose probe sets [`WORD_GATED`]'s gate: the word's `length`.
const GATE_FIELD: &str = "length";

/// The placeholder view of `e`, read as `r`.
fn placeholder_wgsl(e: &Entry, r: &Read) -> String {
    let value = r.wgsl(e.name);
    let body = match r {
        Read::Member { wgsl, .. } if e.name == WORD_GATED => format!(
            "if (!{}({}(ctx.sample.word))) {{\n        return debug_invalid(ctx.frag_xy);\n    }}\n    return {};",
            WORD_GATE[1],
            WORD_GATE[0],
            ramp(e, &value, wgsl)
        ),
        Read::Vector { wgsl: VEC2X3, .. } => format!(
            "let v = {value};\n    return dbg_sentinel(sqrt(dot(v[0], v[0]) + dot(v[1], v[1]) + dot(v[2], v[2])), \
             ctx.frag_xy);"
        ),
        Read::Vector { .. } => format!("return dbg_sentinel(length({value}), ctx.frag_xy);"),
        Read::Member { wgsl, .. } => format!("return {};", ramp(e, &value, wgsl)),
        Read::Ic => format!("return {};", ramp(e, &value, "f32")),
        Read::Word { .. } => format!("return {};", ramp(e, &value, "u32")),
    };
    let colouring = if e.name == UNION_KEY {
        "Its colouring is the six-colour `dbg_cat` palette, one colour per state, not the outcome palette (R-115; \
         debug_tooling_plan §B)."
    } else if e.name == WORD_GATED {
        "The colouring is a placeholder (`ledger::gen::catalogue`), gated on the word: the field has no in-band \
         \"none\" code and is meaningful where `sd_last_symbol_valid(fgw_length_raw(word))` holds, so the empty \
         word and a truncated word draw the hatch (payload §2, §6; applied per R-369)."
    } else {
        "The colouring is a placeholder (`ledger::gen::catalogue`)."
    };
    format!(
        "{}fn colour(ctx: Ctx) -> vec3<f32> {{\n    {body}\n}}\n",
        header(e, r, colouring)
    )
}

/// The generated-file line and the comment naming `e`'s field, location, type, scale, range and accessors, then
/// `colouring`.
fn header(e: &Entry, r: &Read, colouring: &str) -> String {
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
    let mut symbols = r.accessors(e.name);
    if e.name == UNION_FIELD {
        symbols.push(Accessor::Member(UNION_KEY.to_owned()));
    }
    let accessors: Vec<String> = symbols
        .iter()
        .map(|a| match a {
            Accessor::Member(m) => format!("`SimState.{m}`"),
            Accessor::IcMember(m) => format!("`ICDescriptor.{m}`"),
            Accessor::Function(f) => format!("`{f}`"),
        })
        .collect();
    let reduction = if is_vector(e) {
        " For a vector field it is the '‖·‖ as scalar' reduction; its 'as direction-cosines' view is \
         `<field>_dircos.wgsl` in `crates/render/frag/debug/reductions/` (gui_state_contract §4)."
    } else {
        ""
    };
    let text = format!(
        "The debug view of `{name}` (render contract Part 6; debug_tooling_plan §B–E): {location}, {ty}, scale {scale}, \
         range {lo}, {hi}. A colour occupant, `present(unpack(ctx))` (gui_state_contract §3), it reads the field \
         through {acc}; its test, `{test}` in `{TESTS_PATH}`, reads it through their Rust twins. {colouring}{reduction}",
        name = e.name,
        lo = bound(e.range.lo, true),
        hi = bound(e.range.hi, false),
        acc = accessors.join(" and "),
        test = test_name(e.name),
    );
    generated_comment(&text)
}

/// The generated-file line, then `text` as line comments.
fn generated_comment(text: &str) -> String {
    format!(
        "// Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.\n{}",
        comment(text, "// ")
    )
}

/// `text` as line comments led by `lead`, wrapped at 120 columns.
fn comment(text: &str, lead: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split(' ') {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= 120 => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(format!("{lead}{word}")),
        }
    }
    lines
        .iter()
        .map(|l| format!("{}\n", l.trim_end()))
        .collect()
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
/// probe stored, or, for a derived field, the value its derived accessor gives from an unaltered probe. The union's
/// view also reads its key, `key`, the key's name and probe: its check reads the key back too, so the view and its
/// test reference the same members (REQ-TOOL-017). [`WORD_GATED`]'s view also reads its gate, the word's length by
/// `gate`'s probe: its check reads the gate back through the same word accessors.
fn check(
    v: &View,
    stored: Option<&Probe>,
    key: Option<(&str, &Probe)>,
    gate: Option<&Probe>,
) -> String {
    let n = v.field;
    let got = v.read.rust(n);
    let lower = n.to_lowercase();
    let message = |what: &str| format!("\"`{n}` {what}\"");
    let (doc, body) = match (&v.read, stored) {
        (
            Read::Vector {
                derived: true,
                rust,
                ..
            },
            _,
        ) => {
            let (locals, expr) = derived_locals(n);
            let args = [
                format!("{got}.map(f32::to_bits)"),
                "expected.map(f32::to_bits)".to_owned(),
                message(NOT_DERIVED),
            ];
            (
                "is the value its derived accessor gives from the unaltered probe",
                format!(
                    "    let read = read(p);\n{locals}    let expected: {rust} = {expr};\n{}",
                    call(4, "", "expect", &args)
                ),
            )
        }
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
        (Read::Ic, Some(p)) => {
            let args = [
                format!("{got}.to_bits()"),
                format!("{}_f32.to_bits()", p.value),
                message(NOT_STORED),
            ];
            (
                "is the value the probe stored",
                format!("    let ic = &p.ic;\n{}", call(4, "", "expect", &args)),
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
            let mut body = format!(
                "    let read = read(p);\n{}",
                call(4, "", "expect", &[got, want, message(NOT_STORED)])
            );
            if let Some((k, kp)) = key {
                let args = [
                    format!("read.{k}"),
                    kp.value.clone(),
                    format!("\"`{n}`'s key `{k}` {NOT_STORED}\""),
                ];
                body.push('\n');
                body.push_str(&call(4, "", "expect", &args));
            }
            if let Some(g) = gate {
                let length = &g.value;
                let args = [
                    format!("{}({}(read.word))", WORD_GATE[1], WORD_GATE[0]),
                    format!("{}({length})", WORD_GATE[1]),
                    format!("\"`{n}`'s gate, the word's length {length}, {NOT_STORED}\""),
                ];
                body.push('\n');
                body.push_str(&call(4, "", "expect", &args));
            }
            ("is the value the probe stored", body)
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
        let key = (v.field == UNION_FIELD)
            .then(|| {
                let (kk, ke) = entries
                    .iter()
                    .enumerate()
                    .find(|(_, e)| e.name == UNION_KEY)?;
                Some((ke.name, probes[kk].as_ref()?))
            })
            .flatten();
        let gate = (v.field == WORD_GATED)
            .then(|| {
                let (gk, _) = entries
                    .iter()
                    .enumerate()
                    .find(|(_, e)| e.name == GATE_FIELD)?;
                probes[gk].as_ref()
            })
            .flatten();
        out.push_str(&check(v, probes[k].as_ref(), key, gate));
        let test = v.test();
        let lower = v.field.to_lowercase();
        let alter = altered_field(e);
        let expected = match v.read {
            Read::Member { derived: true, .. } | Read::Vector { derived: true, .. } => NOT_DERIVED,
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
