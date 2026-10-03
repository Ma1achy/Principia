//! The read side (lowering Part 3a; payload §5, §6; render contract Part 2): one `SimState` type, fixed across tiers,
//! that the fragment and the host program against, emitted to both targets from the ledger the stored layouts come
//! from: Rust into `crates/kernel/src/payload/generated/read_side.rs` ([`rust`]), WGSL into
//! `crates/render/frag/generated/read_side.wgsl` ([`wgsl`]), which follows the fragment unpack layer at assembly and
//! reads its accessors and stored layouts (`cargo xtask lint wgsl` lints it so).
//!
//! Its members ([`members`]) are the stored members every tier's `SimState` variant holds, each packed word expanded
//! into its fields, then the word, then the quantities derived at read (payload §5): `ftle`, `ftle_valid`,
//! `diffusion`, `diffusion_slope_valid`, `total_substeps_log2`, the two time fractions, `orbit_count`, `retrograde`,
//! the four state predicates and `ensemble_spread`. None of the derived ones is stored (R-79). The Benettin shadow,
//! tier-gated state, is not a read-side member: the read side keeps the cheap derived result, never resurrected state
//! (lowering Part 3a).
//!
//! A stored variant unpacks into it through `sim_state_from_ftle` or `sim_state_from_base`. A tier-absent derived
//! scalar reads the canonical quiet NaN, `CANONICAL_QNAN_BITS` ([`crate::payload::canonical_qnan_bits`]): `ftle`
//! from `SimStateBase`, and `ensemble_spread` when `has_ensemble` is false (E = 0, R-145). An unbound word buffer
//! reads `FGW_UNBOUND` ([`unbound_word`]). Both values are lowering Part 3a's definition (R-72; REQ-RENDER-077). An
//! invalid read writes the same NaN: `ftle` whenever `ftle_valid` is false (R-254), `diffusion` for `n < 2` (R-245).
//! The NaN is always written from its bits, never computed, and no validity logic tests a value for NaN: validity is
//! `ftle_valid`, `diffusion_slope_valid` and the state predicates (lowering Part 3a; R-255).
//!
//! The derived accessors carry payload §6's names where it gives one (`ftle_valid`, `diffusion_slope_valid`,
//! `total_substeps_log2`, `tm_t_end_fraction`, `tm_t_dmin_fraction`, the `sd_is_*` predicates; the last five are the
//! fragment unpack layer's and the Rust emitter's, [`super::rust`], already), and payload §5's otherwise (`ftle`,
//! `diffusion_slope`, `orbit_count`, `retrograde`).

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::{rust as stored, wgsl as frag, Generated};
use crate::schema::{Entry, FieldType, Location, Storage, Struct, Word};

/// Where the Rust read side is written, relative to the workspace root.
pub const RUST_PATH: &str = "crates/kernel/src/payload/generated/read_side.rs";

/// Where the WGSL read side is written, relative to the workspace root: after the unpack layer,
/// [`super::wgsl::PATH`], whose accessors and stored layouts it reads.
pub const WGSL_PATH: &str = "crates/render/frag/generated/read_side.wgsl";

/// How a read-side member is filled from a stored variant `s`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fill {
    /// A stored member of every variant, copied: `s.<name>`.
    Copy,
    /// A u16 member, widened to u32: `u32::from(s.<name>)` in Rust, `<name>(s.<pair>)` in WGSL, where `pair` is the
    /// one u32 member WGSL stores it in (R-343).
    Widen { pair: String },
    /// A packed field, through its payload §6 accessor: `<accessor>(s.<word>)`.
    Packed {
        accessor: String,
        word: &'static str,
    },
    /// The sample's word, or [`unbound_word`] when the word buffer is unbound (`has_word` false).
    Word,
    /// Computed at read (payload §5), never stored.
    Derived,
}

/// One member of the read-side `SimState`: its name, its Rust and WGSL types, and how it is filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadMember {
    pub name: String,
    pub rust: &'static str,
    pub wgsl: &'static str,
    pub fill: Fill,
}

/// The derived members, in order, each with its type as `(name, Rust, WGSL)`.
pub const DERIVED: [(&str, &str, &str); 14] = [
    ("ftle", "f32", "f32"),
    ("ftle_valid", "bool", "bool"),
    ("diffusion", "f32", "f32"),
    ("diffusion_slope_valid", "bool", "bool"),
    ("total_substeps_log2", "u32", "u32"),
    ("t_end_fraction", "f32", "f32"),
    ("t_dmin_fraction", "f32", "f32"),
    ("orbit_count", "u32", "u32"),
    ("retrograde", "bool", "bool"),
    ("is_resolved_outcome", "bool", "bool"),
    ("is_running", "bool", "bool"),
    ("is_failed", "bool", "bool"),
    ("is_finished", "bool", "bool"),
    ("ensemble_spread", "f32", "f32"),
];

/// The `SimState` variants the stored buffer holds, one per tier (payload §1).
fn variants(structs: &[Struct]) -> Vec<&Struct> {
    structs
        .iter()
        .filter(|s| s.buffer == Some("SimState"))
        .collect()
}

/// The read-side `SimState`'s members (lowering Part 3a): each member every stored variant holds, in the first
/// variant's order, a packed word expanded into its fields in bit order and a member whose name starts with `_`
/// skipped; then `word`; then [`DERIVED`].
pub fn members(words: &[Word], entries: &[Entry]) -> Vec<ReadMember> {
    let structs = crate::payload::structs();
    let variants = variants(&structs);
    let mut out = Vec::new();
    let Some((first, rest)) = variants.split_first() else {
        return out;
    };
    let everywhere = |name: &str| {
        rest.iter()
            .all(|s| s.members.iter().any(|m| m.name == name))
    };
    for m in first.members.iter().filter(|m| everywhere(m.name)) {
        if m.name.starts_with('_') {
            continue;
        }
        let member = |rust, wgsl, fill| ReadMember {
            name: m.name.to_owned(),
            rust,
            wgsl,
            fill,
        };
        match m.storage {
            Storage::Vec2x3 => out.push(member("[[f32; 2]; 3]", "array<vec2<f32>, 3>", Fill::Copy)),
            Storage::F32 => out.push(member("f32", "f32", Fill::Copy)),
            Storage::U32 if words.iter().any(|w| w.name == m.name) => {
                out.extend(packed(m.name, entries));
            }
            Storage::U32 => out.push(member("u32", "u32", Fill::Copy)),
            Storage::U16 => {
                let pair = frag::members(first)
                    .into_iter()
                    .find(|w| w.stores.contains(&m.name))
                    .map_or_else(|| format!("{}_has_no_wgsl_member", m.name), |w| w.name);
                out.push(member("u32", "u32", Fill::Widen { pair }));
            }
            Storage::U32x4 | Storage::Pad(_) => {}
        }
    }
    out.push(ReadMember {
        name: "word".to_owned(),
        rust: "[u32; 4]",
        wgsl: "vec4<u32>",
        fill: Fill::Word,
    });
    out.extend(DERIVED.iter().map(|&(name, rust, wgsl)| ReadMember {
        name: name.to_owned(),
        rust,
        wgsl,
        fill: Fill::Derived,
    }));
    out
}

/// The fields of the packed word `word`, in bit order, each read through its payload §6 accessor: a 1-bit field a
/// `bool`, an `f16-pair` an `f32`, any other a `u32`.
fn packed(word: &'static str, entries: &[Entry]) -> Vec<ReadMember> {
    let mut fields: Vec<(&Entry, u32, u32)> = entries
        .iter()
        .filter_map(|e| match e.location {
            Location::Packed {
                word: w,
                offset,
                width,
            } if w == word => Some((e, offset, width)),
            _ => None,
        })
        .collect();
    fields.sort_by_key(|&(_, offset, _)| offset);
    fields
        .into_iter()
        .filter_map(|(e, _, width)| {
            let prefix = stored::prefix(word, e)?;
            let ty = match (&e.ty, width) {
                (FieldType::F16Pair, _) => "f32",
                (_, 1) => "bool",
                _ => "u32",
            };
            Some(ReadMember {
                name: e.name.to_owned(),
                rust: ty,
                wgsl: ty,
                fill: Fill::Packed {
                    accessor: format!("{prefix}_{}", e.name),
                    word,
                },
            })
        })
        .collect()
}

/// The word an unbound word buffer reads (lowering Part 3a; R-72; REQ-RENDER-077): payload zero and `length_raw` the
/// truncation sentinel, from the word buffer `.w`'s `length` entry; every word-derived validity predicate reads it as
/// invalid. The line naming what the entry lacks otherwise ([`stored::fgw_length`]).
pub fn unbound_word(entries: &[Entry]) -> Result<[u32; 4], String> {
    let (offset, _, _, sentinel) = stored::fgw_length(entries)?;
    Ok([0, 0, 0, sentinel << offset])
}

/// A derived member's expression in Rust, `ftle` as the variant with the shadow (`shadow`) or without reads it.
fn rust_derived(name: &str, shadow: bool) -> String {
    match name {
        "ftle" if shadow => "ftle(s.S, delta, params.delta_0, n, params.dt_macro, ftle_ok)".into(),
        "ftle" => "canonical_nan()".into(),
        "ftle_valid" => "ftle_ok".into(),
        "diffusion" => "diffusion_slope(s.C_ty, n, params.dt_macro)".into(),
        "diffusion_slope_valid" => "diffusion_slope_valid(n)".into(),
        "total_substeps_log2" => "total_substeps_log2(s.total_substeps)".into(),
        "t_end_fraction" => "tm_t_end_fraction(s.times, params.horizon_steps)".into(),
        "t_dmin_fraction" => "tm_t_dmin_fraction(s.times, params.horizon_steps)".into(),
        "orbit_count" | "retrograde" => format!("{name}(s.theta)"),
        "ensemble_spread" => {
            "if has_ensemble {\n    ensemble_spread\n} else {\n    canonical_nan()\n}".into()
        }
        predicate => format!("sd_{predicate}(s.packed_a)"),
    }
}

/// A derived member's expression in WGSL, as [`rust_derived`]'s.
fn wgsl_derived(name: &str, shadow: bool) -> String {
    match name {
        "ensemble_spread" => "select(canonical_nan(), ensemble_spread, has_ensemble)".into(),
        other => rust_derived(other, shadow),
    }
}

/// `text` with every line after the first indented by `pad`.
fn indent(text: &str, pad: &str) -> String {
    text.replace('\n', &format!("\n{pad}"))
}

/// The Rust read side: the sentinels, `ReadParams`, the read-side `SimState`, the derived accessors and the two
/// unpacks, `no_std` and rustfmt's layout.
pub fn rust(words: &[Word], entries: &[Entry]) -> Generated {
    let qnan = crate::payload::canonical_qnan_bits();
    let unbound = match unbound_word(entries) {
        Ok(w) => format!(
            "[{}, {}, {}, 0x{:04x}_{:04x}]",
            w[0],
            w[1],
            w[2],
            w[3] >> 16,
            w[3] & 0xffff
        ),
        Err(why) => format!("[0; 4];\ncompile_error!({why:?})"),
    };
    let members = members(words, entries);
    let mut out = format!(
        r#"//! Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.
//! The read side (lowering Part 3a; payload §5, §6): one `SimState`, fixed across tiers, each stored variant's
//! unpack into it, and the quantities derived at read, never stored (R-79).

// The float functions are called through `Float`, as `Float::ln(x)`: the rust-gpu toolchain's `core` has no inherent
// `f32` math, and a newer one's makes a method call skip the trait.
use spirv_std::num_traits::Float;

use super::*;

/// The canonical quiet NaN's f32 bits: sign 0, exponent all ones, the quiet bit alone in the significand (lowering
/// Part 3a; R-72, R-79). A tier-absent derived scalar and an invalid read hold exactly these bits.
pub const CANONICAL_QNAN_BITS: u32 = 0x{:04x}_{:04x};

/// The word an unbound word buffer reads: payload zero, `length_raw` the truncation sentinel, so every word-derived
/// validity predicate reads it as invalid (lowering Part 3a; R-72).
pub const FGW_UNBOUND: [u32; 4] = {unbound};

/// The canonical quiet NaN, written from its bits, never computed (lowering Part 3a).
#[inline]
pub fn canonical_nan() -> f32 {{
    f32::from_bits(CANONICAL_QNAN_BITS)
}}

/// The sim-key values the read side derives from (lowering Part 3; payload §4, §5): the macro-step `dt_macro`, the
/// Benettin shadow's initial separation `delta_0`, the renormalisation interval `n_renorm` and `horizon_steps`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReadParams {{
    pub dt_macro: f32,
    pub delta_0: f32,
    pub n_renorm: u32,
    pub horizon_steps: u32,
}}

/// The read-side `SimState`, the same at every tier (lowering Part 3a): the stored members every variant holds, the
/// packed words' fields, the word, and the quantities derived at read (payload §5).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SimState {{
"#,
        qnan >> 16,
        qnan & 0xffff,
    );
    for m in &members {
        let _ = writeln!(out, "    pub {}: {},", m.name, m.rust);
    }
    out.push_str("}\n");
    out.push_str(RUST_HELPERS);
    for variant in variants(&crate::payload::structs()) {
        out.push_str(&rust_unpack(variant, &members));
    }
    Generated {
        path: PathBuf::from(RUST_PATH),
        contents: out,
    }
}

/// Whether `s` holds the Benettin shadow (`r_sh`, payload §1), so its read computes `ftle`.
fn has_shadow(s: &Struct) -> bool {
    s.members.iter().any(|m| m.name == "r_sh")
}

/// The function that unpacks the variant `s` into the read-side `SimState`: `sim_state_from_ftle` for the variant
/// with the shadow, `sim_state_from_base` for the one without.
pub fn unpack_name(s: &Struct) -> &'static str {
    if has_shadow(s) {
        "sim_state_from_ftle"
    } else {
        "sim_state_from_base"
    }
}

/// The Rust unpack of `s` into the read-side `SimState`.
fn rust_unpack(s: &Struct, members: &[ReadMember]) -> String {
    let shadow = has_shadow(s);
    let doc = if shadow {
        "/// `SimStateFTLE` read: `ftle` finalised from the shadow (payload §5), NaN whenever `ftle_valid` is false (R-254)."
    } else {
        "/// `SimStateBase` read: FTLE is baked out, so `ftle` reads the canonical quiet NaN and `ftle_valid` is false\n\
         /// (lowering Part 3a)."
    };
    let mut out = format!(
        "\n{doc}\n/// An unbound word buffer (`has_word` false) reads `FGW_UNBOUND`; E = 0 (`has_ensemble` false) reads\n\
         /// `ensemble_spread` as the canonical quiet NaN (R-145).\n\
         #[inline]\n\
         pub fn {}(\n    s: &{},\n    word: [u32; 4],\n    has_word: bool,\n    ensemble_spread: f32,\n    has_ensemble: bool,\n    \
         params: &ReadParams,\n) -> SimState {{\n    let n = tm_t_end_step(s.times);\n    \
         let ftle_ok = ftle_valid(s.packed_a, {shadow}, n, completed_renorms(n, params.n_renorm));\n",
        unpack_name(s),
        s.name,
    );
    if shadow {
        out.push_str("    let delta = benettin_delta(s.r, s.p, s.r_sh, s.p_sh);\n");
    }
    out.push_str("    SimState {\n");
    for m in members {
        let value = match &m.fill {
            Fill::Copy => format!("s.{}", m.name),
            Fill::Widen { .. } => format!("u32::from(s.{})", m.name),
            Fill::Packed { accessor, word } => format!("{accessor}(s.{word})"),
            Fill::Word => "if has_word { word } else { FGW_UNBOUND }".into(),
            Fill::Derived => rust_derived(&m.name, shadow),
        };
        let value = indent(&value, "        ");
        if value == m.name {
            let _ = writeln!(out, "        {},", m.name);
        } else {
            let _ = writeln!(out, "        {}: {value},", m.name);
        }
    }
    out.push_str("    }\n}\n");
    out
}

/// The Rust derived accessors (payload §4–§6).
const RUST_HELPERS: &str = r#"
/// The renormalisations completed by step `n`, `n / n_renorm` under the uniform schedule (payload §5); none when
/// `n_renorm` is 0.
#[inline]
pub fn completed_renorms(n: u32, n_renorm: u32) -> u32 {
    n.checked_div(n_renorm).unwrap_or(0)
}

/// Whether `ftle` is usable: the FTLE tier on, the sample not failed, at least one step and at least one completed
/// renormalisation (payload §6). Derived, never stored; `ftle` reads NaN whenever it is false (R-254).
#[inline]
pub fn ftle_valid(state: u32, ftle_tier_on: bool, n: u32, benettin_renorms: u32) -> bool {
    ftle_tier_on && !sd_is_failed(state) && n > 0 && benettin_renorms > 0
}

/// The shadow's current separation `δ = ‖x' − x‖`, the Euclidean norm of the shadow's offset over the twelve
/// phase-space components `(r, p)` (payload §5). Constant indices only: no runtime array index (GPU determinism note
/// § "The discipline", rule 5).
#[inline]
pub fn benettin_delta(
    r: [[f32; 2]; 3],
    p: [[f32; 2]; 3],
    r_sh: [[f32; 2]; 3],
    p_sh: [[f32; 2]; 3],
) -> f32 {
    let d = |a: [f32; 2], b: [f32; 2]| {
        let (x, y) = (b[0] - a[0], b[1] - a[1]);
        x * x + y * y
    };
    let r2 = d(r[0], r_sh[0]) + d(r[1], r_sh[1]) + d(r[2], r_sh[2]);
    let p2 = d(p[0], p_sh[0]) + d(p[1], p_sh[1]) + d(p[2], p_sh[2]);
    Float::sqrt(r2 + p2)
}

/// The FTLE with the partial renormalisation interval finalised: `S_final = S + ln(δ/δ₀)`, divided by the full
/// elapsed `n · dt_macro`, never plain `S / t` (payload §5); the canonical quiet NaN when `valid` is false (R-254).
#[inline]
pub fn ftle(s_sum: f32, delta: f32, delta_0: f32, n: u32, dt_macro: f32, valid: bool) -> f32 {
    if valid {
        (s_sum + Float::ln(delta / delta_0)) / (n as f32 * dt_macro)
    } else {
        canonical_nan()
    }
}

/// Whether the diffusion fit is valid: `n ≥ 2`, as `C_tt(n)` is 0 below (payload §4, §6; R-245).
#[inline]
pub fn diffusion_slope_valid(n: u32) -> bool {
    n >= 2
}

/// `C_tt(n) = h²·n(n²−1)/12`, the time-only co-moment of the uniform samples `h, 2h, …, nh`, derived closed-form,
/// never stored (payload §4). `n − 1` and `n + 1` are exact in f32 for every u16 `n`.
#[inline]
pub fn diffusion_c_tt(n: u32, h: f32) -> f32 {
    let m = n as f32;
    h * h * m * (m - 1.0) * (m + 1.0) / 12.0
}

/// The diffusion slope `C_ty / C_tt(n)`, `n` the sample's own completed-step count (`t_end_step`, latched at
/// termination), never the playhead; the canonical quiet NaN for `n < 2`, with no division by zero (payload §4;
/// R-245).
#[inline]
pub fn diffusion_slope(c_ty: f32, n: u32, h: f32) -> f32 {
    if diffusion_slope_valid(n) {
        c_ty / diffusion_c_tt(n, h)
    } else {
        canonical_nan()
    }
}

/// The winding count `⌊|θ̃| / 2π⌋` (payload §5), clamped in f32 to the largest f32 below 2³² before the cast (GPU
/// determinism note § "The discipline", rule 3).
#[inline]
pub fn orbit_count(theta: f32) -> u32 {
    let turns = Float::floor(Float::abs(theta) / core::f32::consts::TAU);
    Float::min(turns, 4_294_967_040.0) as u32
}

/// The winding sense: `θ̃ < 0` (payload §5).
#[inline]
pub fn retrograde(theta: f32) -> bool {
    theta < 0.0
}
"#;

/// The WGSL read side, which follows the fragment unpack layer: the sentinels, `ReadParams`, the read-side `SimState`,
/// the derived accessors and the two unpacks. `has_word` and `has_ensemble` are arguments, so the two files need no
/// bake of their own; the assembler passes its `has_word` const and its `has_ensemble` uniform (lowering Part 3a;
/// R-145).
pub fn wgsl(words: &[Word], entries: &[Entry]) -> Generated {
    let qnan = crate::payload::canonical_qnan_bits();
    let unbound = match unbound_word(entries) {
        Ok(w) => format!(
            "vec4<u32>({}u, {}u, {}u, {:#010x}u)",
            w[0], w[1], w[2], w[3]
        ),
        Err(why) => format!("vec4<u32>(0u);\nconst_assert false; // {why}"),
    };
    let members = members(words, entries);
    let mut out = format!(
        r"// Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.
// The read side (lowering Part 3a; payload §5, §6): one `SimState`, fixed across tiers, each stored variant's unpack
// into it, and the quantities derived at read, never stored (R-79). It follows the unpack layer,
// `payload_unpack.wgsl`, whose accessors and stored layouts it reads.

// The canonical quiet NaN's f32 bits: sign 0, exponent all ones, the quiet bit alone in the significand (lowering
// Part 3a; R-72, R-79). A tier-absent derived scalar and an invalid read hold exactly these bits.
const CANONICAL_QNAN_BITS: u32 = {qnan:#010x}u;

// The word an unbound word buffer reads: payload zero, `length_raw` the truncation sentinel, so every word-derived
// validity predicate reads it as invalid (lowering Part 3a; R-72).
const FGW_UNBOUND: vec4<u32> = {unbound};

// The canonical quiet NaN, written from its bits, never computed (lowering Part 3a).
fn canonical_nan() -> f32 {{ return bitcast<f32>(CANONICAL_QNAN_BITS); }}

// The sim-key values the read side derives from (lowering Part 3; payload §4, §5).
struct ReadParams {{
    dt_macro: f32,
    delta_0: f32,
    n_renorm: u32,
    horizon_steps: u32,
}}

// The read-side `SimState`, the same at every tier (lowering Part 3a).
struct SimState {{
"
    );
    for m in &members {
        let _ = writeln!(out, "    {}: {},", m.name, m.wgsl);
    }
    out.push_str("}\n");
    out.push_str(WGSL_HELPERS);
    for variant in variants(&crate::payload::structs()) {
        out.push_str(&wgsl_unpack(variant, &members));
    }
    Generated {
        path: PathBuf::from(WGSL_PATH),
        contents: out,
    }
}

/// The WGSL unpack of `s` into the read-side `SimState`.
fn wgsl_unpack(s: &Struct, members: &[ReadMember]) -> String {
    let shadow = has_shadow(s);
    let mut out = format!(
        "\n// `{}` read into the read-side `SimState`; an unbound word buffer reads `FGW_UNBOUND`, E = 0 reads\n\
         // `ensemble_spread` as the canonical quiet NaN (lowering Part 3a; R-145, R-254).\n\
         fn {}(s: {}, word: vec4<u32>, has_word: bool, ensemble_spread: f32, has_ensemble: bool, params: ReadParams) -> SimState {{\n    \
         let n = tm_t_end_step(s.times);\n    \
         let ftle_ok = ftle_valid(s.packed_a, {shadow}, n, completed_renorms(n, params.n_renorm));\n",
        s.name,
        unpack_name(s),
        s.name,
    );
    if shadow {
        out.push_str("    let delta = benettin_delta(s.r, s.p, s.r_sh, s.p_sh);\n");
    }
    out.push_str("    var out: SimState;\n");
    for m in members {
        let value = match &m.fill {
            Fill::Copy => format!("s.{}", m.name),
            Fill::Widen { pair } => format!("{}(s.{pair})", m.name),
            Fill::Packed { accessor, word } => format!("{accessor}(s.{word})"),
            Fill::Word => "select(FGW_UNBOUND, word, has_word)".into(),
            Fill::Derived => wgsl_derived(&m.name, shadow),
        };
        let _ = writeln!(out, "    out.{} = {value};", m.name);
    }
    out.push_str("    return out;\n}\n");
    out
}

/// The WGSL derived accessors, as [`RUST_HELPERS`]'s. `select` evaluates both arms; the invalid arm's arithmetic is
/// discarded, and its result is the canonical NaN's bits either way.
const WGSL_HELPERS: &str = r"
// The renormalisations completed by step `n`, `n / n_renorm` under the uniform schedule (payload §5); none when
// `n_renorm` is 0.
fn completed_renorms(n: u32, n_renorm: u32) -> u32 { return select(0u, n / max(n_renorm, 1u), n_renorm > 0u); }

// Whether `ftle` is usable: the FTLE tier on, the sample not failed, at least one step and at least one completed
// renormalisation (payload §6). Derived, never stored; `ftle` reads NaN whenever it is false (R-254).
fn ftle_valid(state: u32, ftle_tier_on: bool, n: u32, benettin_renorms: u32) -> bool {
    return ftle_tier_on && !sd_is_failed(state) && n > 0u && benettin_renorms > 0u;
}

// The shadow's current separation `δ = ‖x' − x‖`, the Euclidean norm of the shadow's offset over the twelve
// phase-space components `(r, p)` (payload §5).
fn benettin_delta(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, r_sh: array<vec2<f32>, 3>, p_sh: array<vec2<f32>, 3>) -> f32 {
    let r0 = r_sh[0] - r[0];
    let r1 = r_sh[1] - r[1];
    let r2 = r_sh[2] - r[2];
    let p0 = p_sh[0] - p[0];
    let p1 = p_sh[1] - p[1];
    let p2 = p_sh[2] - p[2];
    return sqrt(dot(r0, r0) + dot(r1, r1) + dot(r2, r2) + dot(p0, p0) + dot(p1, p1) + dot(p2, p2));
}

// The FTLE with the partial renormalisation interval finalised: `S_final = S + ln(δ/δ₀)`, divided by the full elapsed
// `n · dt_macro`, never plain `S / t` (payload §5); the canonical quiet NaN when `valid` is false (R-254).
fn ftle(s_sum: f32, delta: f32, delta_0: f32, n: u32, dt_macro: f32, valid: bool) -> f32 {
    return select(canonical_nan(), (s_sum + log(delta / delta_0)) / (f32(n) * dt_macro), valid);
}

// Whether the diffusion fit is valid: `n ≥ 2`, as `C_tt(n)` is 0 below (payload §4, §6; R-245).
fn diffusion_slope_valid(n: u32) -> bool { return n >= 2u; }

// `C_tt(n) = h²·n(n²−1)/12`, derived closed-form, never stored (payload §4).
fn diffusion_c_tt(n: u32, h: f32) -> f32 {
    let m = f32(n);
    return h * h * m * (m - 1.0) * (m + 1.0) / 12.0;
}

// The diffusion slope `C_ty / C_tt(n)`, `n` the sample's own `t_end_step`, never the playhead; the canonical quiet NaN
// for `n < 2` (payload §4; R-245).
fn diffusion_slope(c_ty: f32, n: u32, h: f32) -> f32 {
    return select(canonical_nan(), c_ty / diffusion_c_tt(n, h), diffusion_slope_valid(n));
}

// The winding count `⌊|θ̃| / 2π⌋` (payload §5), clamped in f32 to the largest f32 below 2³² before the cast.
fn orbit_count(theta: f32) -> u32 { return u32(min(floor(abs(theta) / 6.2831855), 4294967040.0)); }

// The winding sense: `θ̃ < 0` (payload §5).
fn retrograde(theta: f32) -> bool { return theta < 0.0; }
";
