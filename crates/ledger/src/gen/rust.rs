//! The Rust emitter (dd_generation_root §1; dd_simstate_payload §1, §2, §6): writes the payload schema version
//! ([`crate::version::emit`]), then each of [`crate::payload::structs`] as a `#[repr(C)]`, `no_std`-compatible struct
//! into `crates/kernel/src/payload/generated.rs`, members in order, vec2 groups as `[[f32; 2]; 3]`, then the packed
//! words' pack/unpack/insert code ([`accessors`]), the word buffer's `fgw_*` accessors ([`fgw`]), payload §3's frozen
//! continuation table ([`continuation`]) and the stored buffers' binding constants ([`bindings`], R-343). [`check`] holds each member against the ledger entry or word it stores.
//! [`emit`] also writes the read side, `crates/kernel/src/payload/generated/read_side.rs` ([`super::read::rust`]).
//!
//! The `SimState` structs and `ICDescriptor` are emitted per precision (philosophy §7.1; dd_simstate_payload §1;
//! dd_generation_root §3.6; R-313): generic over the kernel's `Real`, their f32 members widen to it ([`widens`]) and
//! every other member keeps its width, with the tail padding each width needs declared, never implicit (R-86). The f32
//! instantiation keeps the struct's name. Each row of [`precisions`] gets its layout ([`layout`]) in the generated
//! `PAYLOAD_LAYOUTS` table.

use std::fmt::Write as _;
use std::mem::{align_of, size_of};
use std::path::PathBuf;

use crate::gen::Generated;
use crate::schema::{Bound, Entry, FieldType, Location, Member, Storage, Struct, Word};

/// Where the emitter writes, relative to the workspace root.
pub const PATH: &str = "crates/kernel/src/payload/generated.rs";

/// A precision row (dd_simstate_payload §1): a float type the payload's widths are a function of (philosophy §7.1),
/// its size and alignment in bytes, and whether the kernel is instantiated at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Precision {
    pub name: &'static str,
    pub size: u32,
    pub align: u32,
    pub instantiated: bool,
}

/// The precision rows, f32 first: f32 and f64, the kernel's two instantiations (R-265), and DoubleF64, an unevaluated
/// pair of f64s, a stub row for the width function only, with no `Real` impl and no arithmetic (REQ-SYS-007).
pub fn precisions() -> Vec<Precision> {
    let row = |name, size: usize, align: usize, instantiated| Precision {
        name,
        size: size as u32,
        align: align as u32,
        instantiated,
    };
    vec![
        row("f32", size_of::<f32>(), align_of::<f32>(), true),
        row("f64", size_of::<f64>(), align_of::<f64>(), true),
        row(
            "DoubleF64",
            size_of::<[f64; 2]>(),
            align_of::<[f64; 2]>(),
            false,
        ),
    ]
}

/// Whether `m` of `s` widens with the `Real` (dd_simstate_payload §1; dd_generation_root §3.6; R-313): an f32 member,
/// a scalar or a vec2 group, of any payload struct, so `SimState`'s and `ICDescriptor`'s. The packed words, the u32
/// counter, the u16 step index and reserve, the word buffer's u32s and `ICDescriptor`'s declared padding keep their
/// widths.
pub fn widens(_s: &Struct, m: &Member) -> bool {
    matches!(m.storage, Storage::F32 | Storage::Vec2x3)
}

/// Whether `s` has a member that widens with the `Real`, so it is emitted generic over it (`SimState`'s two variants
/// and `ICDescriptor`).
pub fn is_generic(s: &Struct) -> bool {
    s.members.iter().any(|m| widens(s, m))
}

/// Whether `s` is a `SimState` struct generic over the `Real`: the structs whose layouts are a precision row's
/// `structs` in `PAYLOAD_LAYOUTS` (dd_simstate_payload §1). `ICDescriptor`, generic too ([`is_generic`]), is the row's
/// `descriptor` (dd_generation_root §3.6).
pub fn is_real(s: &Struct) -> bool {
    s.buffer == Some("SimState") && is_generic(s)
}

/// A struct's layout at one precision: each member's byte offset, the end of its last member, its size (the end
/// rounded up to its alignment) and its alignment. `size - end` is its declared tail padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub offsets: Vec<u32>,
    pub end: u32,
    pub size: u32,
    pub align: u32,
}

/// `s` at precision `p`: each member packed in order at its alignment, a widened member ([`widens`]) at `p`'s size and
/// alignment, a u16 at 2 and any other at 4; the struct aligned to the greater of its own alignment and its members'.
pub fn layout(s: &Struct, p: &Precision) -> Layout {
    let mut at = 0u32;
    let mut align = s.align;
    let mut offsets = Vec::new();
    for m in &s.members {
        let (size, a) = if widens(s, m) {
            (m.storage.size() / 4 * p.size, p.align)
        } else if m.storage == Storage::U16 {
            (2, 2)
        } else {
            (m.storage.size(), 4)
        };
        align = align.max(a);
        at = at.next_multiple_of(a);
        offsets.push(at);
        at += size;
    }
    Layout {
        offsets,
        end: at,
        size: at.next_multiple_of(align),
        align,
    }
}

/// Each member's byte offset, packed in order at its storage's alignment, and the struct's size, rounded up to its
/// alignment: its layout at f32, the first of [`precisions`].
pub fn offsets(s: &Struct) -> (Vec<u32>, u32) {
    let l = layout(s, &precisions()[0]);
    (l.offsets, l.size)
}

/// The name `s` is declared under in the generated file: `<name>Of<R: PayloadReal>` for a struct generic over the
/// `Real` ([`is_generic`]), else its name.
pub fn declared_name(s: &Struct) -> String {
    if is_generic(s) {
        format!("{}Of<R: PayloadReal>", s.name)
    } else {
        s.name.to_owned()
    }
}

/// The name of the associated type of `PayloadReal` that is `s`'s declared tail padding at the `Real`.
fn tail(s: &Struct) -> String {
    format!("{}Tail", s.name)
}

/// The `name: type` of each member `s` is declared with: a widened member's f32 written `R`, and a struct generic over
/// the `Real` closing with `_tail`, its declared tail padding at `R` (R-86).
pub fn declared_members(s: &Struct) -> Vec<String> {
    let mut members: Vec<String> = s
        .members
        .iter()
        .map(|m| {
            let ty = m.storage.rust();
            let ty = if widens(s, m) {
                ty.replace("f32", "R")
            } else {
                ty
            };
            format!("{}: {ty}", m.name)
        })
        .collect();
    if is_generic(s) {
        members.push(format!("_tail: R::{}", tail(s)));
    }
    members
}

/// Whether `words` or `entries` declare any member of `structs`: whether the layout is the one the structs store. A
/// layout that declares none (a test fixture's) is not tied to them, and [`emit`] writes nothing for it.
pub fn declares(structs: &[Struct], words: &[Word], entries: &[Entry]) -> bool {
    structs
        .iter()
        .flat_map(|s| &s.members)
        .any(|m| words.iter().any(|w| w.name == m.name) || entries.iter().any(|e| e.name == m.name))
}

/// The emitter: the generated file for the payload structs ([`crate::payload::structs`]), or nothing if the layout
/// declares none of their members. The driver ([`crate::gen::generate`]) has validated the layout and refused it
/// unless [`check`] ties every struct member to it.
pub fn emit(words: &[Word], entries: &[Entry]) -> Vec<Generated> {
    let structs = crate::payload::structs();
    if !declares(&structs, words, entries) {
        return Vec::new();
    }
    let mut out = String::from(
        "//! Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.\n",
    );
    out.push_str(&crate::version::emit(words, entries));
    let rows = precisions();
    for s in &structs {
        let (_, size) = offsets(s);
        let sizes: Vec<String> = rows
            .iter()
            .filter(|p| p.instantiated)
            .map(|p| {
                let l = layout(s, p);
                format!("{} B aligned to {} at {}", l.size, l.align, p.name)
            })
            .collect();
        let doc = if is_generic(s) {
            let source = if is_real(s) {
                "dd_simstate_payload §1"
            } else {
                "dd_generation_root §3.6; R-313"
            };
            format!(
                "`{}` at the `Real` `R`: {}.\n\
                 /// Its f32 members are `R`; `_tail` is its declared tail padding ({source}; philosophy §7.1; R-86)",
                s.name,
                sizes.join(", ")
            )
        } else {
            format!(
                "`{}`: {size} B, aligned to {} (dd_simstate_payload §1; dd_generation_root §3.3a, §3.6)",
                s.name, s.align
            )
        };
        let _ = write!(
            out,
            "\n/// {doc}.\n#[repr(C, align({}))]\n#[derive(Clone, Copy, Debug, Default, PartialEq)]\npub struct {} {{\n",
            s.align,
            declared_name(s)
        );
        for m in declared_members(s) {
            let _ = writeln!(out, "    pub {m},");
        }
        out.push_str("}\n");
        if is_generic(s) {
            let _ = write!(
                out,
                "\n/// `{name}` at f32, the GPU's instantiation (canonical_spec §1 item 3): {size} B.\n\
                 pub type {name} = {name}Of<f32>;\n",
                name = s.name
            );
        }
    }
    out.push_str(&precision_code(&structs, &rows));
    out.push_str(&accessors(words, entries));
    out.push_str(&continuation());
    out.push_str(&bindings());
    vec![
        Generated {
            path: PathBuf::from(PATH),
            contents: out,
        },
        super::read::rust(words, entries),
    ]
}

/// The per-precision code: the `PayloadReal` trait, which gives each struct generic over the `Real` its declared tail
/// padding, `[u32; n]`, implemented for each instantiated row of `rows`, and `PAYLOAD_LAYOUTS`, each row's layout of
/// those structs, the stub rows included: the `SimState` structs ([`is_real`]) as its `structs`, `ICDescriptor` as its
/// `descriptor` (dd_simstate_payload §1; dd_generation_root §3.6; R-313).
fn precision_code(structs: &[Struct], rows: &[Precision]) -> String {
    let generic: Vec<&Struct> = structs.iter().filter(|s| is_generic(s)).collect();
    let real: Vec<&Struct> = structs.iter().filter(|s| is_real(s)).collect();
    let descriptor = structs
        .iter()
        .find(|s| s.name == "ICDescriptor")
        .expect("the payload structs include ICDescriptor (dd_generation_root §3.6)");
    let mut out = String::from(
        "\n/// A `Real` the payload is instantiated at (R-265), with the declared tail padding (R-86) of each struct generic\n\
         /// over it at its width (dd_simstate_payload §1).\n\
         pub trait PayloadReal: crate::Real {\n",
    );
    for s in &generic {
        let _ = writeln!(
            out,
            "    type {}: Copy + Default + core::fmt::Debug + PartialEq;",
            tail(s)
        );
    }
    out.push_str("}\n");
    for p in rows.iter().filter(|p| p.instantiated) {
        let _ = write!(out, "\nimpl PayloadReal for {} {{\n", p.name);
        for s in &generic {
            let l = layout(s, p);
            let _ = writeln!(
                out,
                "    type {} = [u32; {}];",
                tail(s),
                (l.size - l.end) / 4
            );
        }
        out.push_str("}\n");
    }
    let n = real.len();
    let _ = write!(
        out,
        "\n/// A precision row of the payload (dd_simstate_payload §1): the float type, its size and alignment in bytes, whether\n\
         /// the kernel is instantiated at it (R-265), and each struct generic over it as `(name, size, alignment)` at it:\n\
         /// the `SimState` structs as `structs`, `ICDescriptor` as `descriptor` (dd_generation_root §3.6; R-313).\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n\
         pub struct PayloadLayout {{\n\
         \x20   pub real: &'static str,\n\
         \x20   pub real_size: usize,\n\
         \x20   pub real_align: usize,\n\
         \x20   pub instantiated: bool,\n\
         \x20   pub structs: [(&'static str, usize, usize); {n}],\n\
         \x20   pub descriptor: (&'static str, usize, usize),\n\
         }}\n\
         \n/// The payload's layout at each precision row, f32 first; a row not instantiated is a stub for the width function\n\
         /// only (dd_simstate_payload §1; philosophy §7.1).\n\
         pub const PAYLOAD_LAYOUTS: [PayloadLayout; {}] = [\n",
        rows.len()
    );
    for p in rows {
        let _ = write!(
            out,
            "    PayloadLayout {{\n        real: \"{}\",\n        real_size: {},\n        real_align: {},\n        \
             instantiated: {},\n",
            p.name, p.size, p.align, p.instantiated
        );
        let items: Vec<String> = real
            .iter()
            .map(|s| {
                let l = layout(s, p);
                format!("(\"{}\", {}, {})", s.name, l.size, l.align)
            })
            .collect();
        // rustfmt's layout: the array on one line if it fits in 100 columns, else one item per line.
        let line = format!("        structs: [{}],", items.join(", "));
        if line.chars().count() <= 100 {
            let _ = writeln!(out, "{line}");
        } else {
            let one_per_line: String = items
                .iter()
                .map(|i| format!("            {i},\n"))
                .collect();
            let _ = write!(out, "        structs: [\n{one_per_line}        ],\n");
        }
        let d = layout(descriptor, p);
        let _ = writeln!(
            out,
            "        descriptor: (\"{}\", {}, {}),",
            descriptor.name, d.size, d.align
        );
        out.push_str("    },\n");
    }
    out.push_str("];\n");
    out
}

/// The accessor prefix of each packed word (payload §6): `pa_`, `pb_` and `tm_`, and `sd_` for the unsigned fields of
/// `packed_a`, which are the `sample_descriptor`'s; `fgw_` for the word buffer's `.w` (payload §3), whose accessors
/// are [`fgw`]'s.
pub const PREFIXES: [(&str, &str); 4] = [
    ("packed_a", "pa"),
    ("packed_b", "pb"),
    ("times", "tm"),
    ("fgw_w", "fgw"),
];

/// The word buffer's `.w`, which [`fgw`] emits for, not the per-field loop of [`accessors`].
const FGW_WORD: &str = "fgw_w";

/// The accessor prefix of `entry` in `word`, or `None` if the word has none.
pub(crate) fn prefix(word: &str, entry: &Entry) -> Option<&'static str> {
    let (_, p) = PREFIXES.iter().find(|(w, _)| *w == word)?;
    Some(if *p == "pa" && entry.ty == FieldType::UBits {
        "sd"
    } else {
        p
    })
}

/// The f32 bit pattern of `value`, as a Rust hex literal (`0x7f80_0000` for +∞).
fn f32_bits(value: f64) -> String {
    let b = (value as f32).to_bits();
    format!("0x{:04x}_{:04x}", b >> 16, b & 0xffff)
}

/// A sentinel as a Rust expression of the field's type: an f32 for an `f16-pair` field, a u32 for unsigned bits. A
/// non-finite f32 is written through its bits, `f32::from_bits(…)`, never as a literal or a named constant such as
/// `f32::INFINITY`: naga rejects a non-finite float literal (GPU determinism note § "The discipline", rule 4;
/// integrator contract § "Rules the new kernel must hold by construction", rule 4).
fn literal(entry: &Entry, value: f64) -> (&'static str, String) {
    match entry.ty {
        FieldType::F16Pair if !value.is_finite() => {
            ("f32", format!("f32::from_bits({})", f32_bits(value)))
        }
        FieldType::F16Pair => ("f32", format!("{value:?}")),
        _ => ("u32", format!("{}", value as u64)),
    }
}

/// The pack/unpack/insert code of the packed words, emitted from their entries (payload §2, §6), after the fixed
/// helpers ([`helpers`]). Per packed field `f` of word `w`, at bits `o .. o + n`:
/// - an unpack accessor named as payload §6, `<prefix>_f(w)`, reading `extract(w, o, n)`: a `bool` for a flag, an f32
///   through the binary16 conversion for an `f16-pair`, else a `u32`;
/// - a setter `set_f(w, v)` writing `insert(w, v, o, n)`; an `f16-pair` value is clamped to ±65504 first (payload
///   §1). An `f16-pair` whose sentinel is +∞ is `d_min` under R-271: an input whose f32 bits are +∞'s writes the
///   unset bits (f16 +∞, `0x7c00`), a value below f16's smallest positive subnormal writes that subnormal, both as bit
///   patterns, so 0.0 never appears; `set_f_unset(w)` writes the unset bits and `<prefix>_f_is_unset(w)` tests them by
///   bits (R-271). No float comparison with +∞ and no non-finite literal is emitted (GPU determinism note § "The
///   discipline", rule 4). Its release setter `set_f_release(w, v, counters)` stores a NaN as unset and a negative
///   value as the floor, counting it through `counters`' `increment_nan_unset` or `increment_negative_floored` (R-281,
///   R-288, R-300); `set_f(w, v, counters)` adds R-281's debug assertions. The counters are the frame's, passed in by
///   the caller and read back by it; there is no crate-level pair and no mutable static (R-294);
/// - a sentinel constant `<PREFIX>_F_SENTINEL` when the entry has a sentinel, a non-finite one written by its bits;
/// - per word, `W_RESERVED`, its reserved spans, and `pack_w(fields…)`, which writes each field in bit order over
///   zero, so reserved bits are zero; a word holding `d_min` takes the caller's `counters` last and passes them to
///   `set_d_min` (R-294).
///
/// The word buffer's `.w` (`fgw_w`) is not a `SimState` word: its accessors are [`fgw`]'s.
pub fn accessors(words: &[Word], entries: &[Entry]) -> String {
    let mut out = state_codes();
    out.push_str(&helpers());
    for word in words {
        if word.name == FGW_WORD {
            out.push_str(&fgw(entries));
            continue;
        }
        let mut fields: Vec<(&Entry, u32, u32)> = entries
            .iter()
            .filter_map(|e| match e.location {
                Location::Packed {
                    word: w,
                    offset,
                    width,
                } if w == word.name => Some((e, offset, width)),
                _ => None,
            })
            .collect();
        fields.sort_by_key(|&(_, offset, _)| offset);
        let mut params = Vec::new();
        let mut calls = Vec::new();
        let mut counted = false;
        for &(e, offset, width) in &fields {
            let Some(p) = prefix(word.name, e) else {
                continue;
            };
            let (w, f) = (word.name, e.name);
            let (ty, get, put) = match (&e.ty, width) {
                (FieldType::F16Pair, _) => (
                    "f32",
                    format!("f16_bits_to_f32(extract(w, {offset}, {width}) as u16)"),
                    "u32::from(f32_to_f16_bits(clamp_f16(v)))".to_owned(),
                ),
                (_, 1) => (
                    "bool",
                    format!("extract(w, {offset}, {width}) == 1"),
                    "u32::from(v)".to_owned(),
                ),
                _ => (
                    "u32",
                    format!("extract(w, {offset}, {width})"),
                    "v".to_owned(),
                ),
            };
            let bits = match width {
                1 => format!("bit {offset} of `{w}`"),
                _ => format!("bits {offset}–{} of `{w}`", offset + width - 1),
            };
            let unset = e.ty == FieldType::F16Pair && e.sentinel == Some(f64::INFINITY);
            if let Some(s) = e.sentinel {
                let (t, value) = literal(e, s);
                let name = format!("{p}_{f}_SENTINEL").to_uppercase();
                let _ = write!(
                    out,
                    "\n/// `{f}`'s sentinel in the ledger (dd_generation_root §3.8).\npub const {name}: {t} = {value};\n"
                );
            }
            let _ = write!(
                out,
                "\n/// `{f}`: {bits} (payload §2, §6).\n#[inline]\npub fn {p}_{f}(w: u32) -> {ty} {{\n    {get}\n}}\n"
            );
            if unset {
                let _ = write!(
                    out,
                    "\n/// `{f}`'s unset bits: f16 +∞, the minimum of an empty set (R-271).\n\
                     pub const {upper}_UNSET: u32 = 0x7c00;\n\
                     \n/// Whether `{f}` is unset: tested by its bits, never by a float comparison (R-271).\n\
                     #[inline]\npub fn {p}_{f}_is_unset(w: u32) -> bool {{\n    extract(w, {offset}, {width}) == {upper}_UNSET\n}}\n\
                     \n/// `{w}` with `{f}` unset: a failed sample's, and any sample's before its first step (R-271).\n\
                     #[inline]\npub fn set_{f}_unset(w: u32) -> u32 {{\n    insert(w, {upper}_UNSET, {offset}, {width})\n}}\n\
                     \n/// `{w}` with `{f}` set to `v` (R-271, payload §1): +∞, tested by its f32 bits, writes the unset bits; a value\n\
                     /// below f16's smallest positive subnormal, 2⁻²⁴, writes that subnormal (`0x0001`), so 0.0 never appears; both\n\
                     /// are written as bits, not through the conversion. Otherwise `v` is clamped to ±65504 and converted.\n\
                     ///\n\
                     /// Storage never holds NaN (R-79) and a negative value is never silently rewritten (R-281): each is a\n\
                     /// `debug_assert!` failure. The release behaviour is [`set_{f}_release`]'s, counting into `counters`, the\n\
                     /// frame's pair that the caller passes in and reads back (R-288, R-294).\n\
                     #[inline]\npub fn set_{f}(w: u32, v: f32, counters: &super::DminCounters) -> u32 {{\n\
                     \x20   debug_assert!(\n\
                     \x20       !v.is_nan(),\n\
                     \x20       \"`{f}` is NaN: storage never holds NaN (R-79, R-281)\"\n\
                     \x20   );\n\
                     \x20   debug_assert!(\n\
                     \x20       v.is_nan() || v >= 0.0,\n\
                     \x20       \"`{f}` is negative: it is never silently rewritten (R-281)\"\n\
                     \x20   );\n\
                     \x20   set_{f}_release(w, v, counters)\n}}\n\
                     \n/// [`set_{f}`] without its debug assertions, as a release build runs it (R-281): NaN writes the unset\n\
                     /// bits (never NaN, R-79) and counts it with `counters.increment_nan_unset()`; a negative value\n\
                     /// writes the floor `0x0001` and counts it with `counters.increment_negative_floored()`; any other\n\
                     /// value below 2⁻²⁴, −0.0 included, writes the floor uncounted. The counts are made in release builds\n\
                     /// as well as debug ones (R-288; telemetry §2). The pair's fields are private (R-300).\n\
                     #[inline]\npub fn set_{f}_release(w: u32, v: f32, counters: &super::DminCounters) -> u32 {{\n\
                     \x20   let h = if v.is_nan() {{\n\
                     \x20       counters.increment_nan_unset();\n\
                     \x20       {upper}_UNSET\n\
                     \x20   }} else if v.to_bits() == {inf} {{\n\
                     \x20       {upper}_UNSET\n\
                     \x20   }} else if v < 0.0 {{\n\
                     \x20       counters.increment_negative_floored();\n\
                     \x20       F16_MIN_SUBNORMAL_BITS\n\
                     \x20   }} else if v < F16_MIN_SUBNORMAL {{\n\
                     \x20       F16_MIN_SUBNORMAL_BITS\n\
                     \x20   }} else {{\n\
                     \x20       {put}\n\
                     \x20   }};\n\
                     \x20   insert(w, h, {offset}, {width})\n}}\n",
                    upper = format!("{p}_{f}").to_uppercase(),
                    inf = f32_bits(f64::INFINITY),
                );
            } else {
                let clamp = if e.ty == FieldType::F16Pair {
                    ", clamped to ±65504 first (payload §1)"
                } else {
                    ""
                };
                let _ = write!(
                    out,
                    "\n/// `{w}` with `{f}` set to `v`{clamp}; every other bit kept.\n#[inline]\n\
                     pub fn set_{f}(w: u32, v: {ty}) -> u32 {{\n    insert(w, {put}, {offset}, {width})\n}}\n"
                );
            }
            params.push(format!("{f}: {ty}"));
            if unset {
                counted = true;
                calls.push(format!("set_{f}({{}}, {f}, counters)"));
            } else {
                calls.push(format!("set_{f}({{}}, {f})"));
            }
        }
        // The `d_min` setter counts into the frame's pair, so the word's packer takes it from its caller (R-294).
        if counted {
            params.push("counters: &super::DminCounters".to_owned());
        }
        let Some(last) = calls.pop() else {
            continue;
        };
        let w = word.name;
        let line = format!("pub fn pack_{w}({}) -> u32 {{", params.join(", "));
        // rustfmt's layout: the signature on one line if it fits in 100 columns, else one parameter per line.
        let signature = if line.chars().count() <= 100 {
            line
        } else {
            let one_per_line: String = params.iter().map(|p| format!("    {p},\n")).collect();
            format!("pub fn pack_{w}(\n{one_per_line}) -> u32 {{")
        };
        let mut body = String::new();
        let mut from = "0";
        for call in &calls {
            let _ = writeln!(body, "    let w = {};", call.replace("{}", from));
            from = "w";
        }
        let _ = writeln!(body, "    {}", last.replace("{}", from));
        let spans: Vec<String> = word
            .reserved
            .iter()
            .map(|r| format!("({}, {})", r.offset, r.width))
            .collect();
        let _ = write!(
            out,
            "\n/// `{w}`'s reserved spans, `(offset, width)`, as the ledger declares them: never written, decoded as zero\n\
             /// (payload §2; dd_generation_root §5 test 1).\n\
             pub const {upper}_RESERVED: [(u32, u32); {n}] = [{spans}];\n",
            upper = w.to_uppercase(),
            n = spans.len(),
            spans = spans.join(", "),
        );
        let _ = write!(
            out,
            "\n/// `{w}` from its fields, each written in bit order over zero, so its reserved bits are zero (payload §2).\n\
             #[inline]\n{signature}\n{body}}}\n",
        );
    }
    out
}

/// `fgw_w`'s `length` entry as [`fgw`] needs it, `(offset, width, capacity, sentinel)`: its bits, its range's closed
/// greatest value and its sentinel. Otherwise the line naming `fgw_w.length` and what it lacks, which refuses
/// generation ([`crate::gen::validate`]; dd_generation_root §3.8: "A field without a complete entry fails generation
/// loudly").
pub fn fgw_length(entries: &[Entry]) -> Result<(u32, u32, u32, u32), String> {
    let refuse = |what: &str| {
        Err(format!(
            "field `{FGW_WORD}.length` {what}: the word buffer's accessors need it (payload §3; dd_generation_root §3.8)"
        ))
    };
    let length = entries.iter().find_map(|e| match e.location {
        Location::Packed {
            word,
            offset,
            width,
        } if word == FGW_WORD && e.name == "length" => Some((e, offset, width)),
        _ => None,
    });
    let Some((e, offset, width)) = length else {
        return refuse("has no entry");
    };
    let Bound::Closed(capacity) = e.range.hi else {
        return refuse("has no closed greatest value");
    };
    let Some(sentinel) = e.sentinel else {
        return refuse("has no sentinel");
    };
    Ok((offset, width, capacity as u32, sentinel as u32))
}

/// `fgw_w`'s `payload` entry as [`fgw`] needs it, `(offset, width)`: the bits of `.w` holding the mixed-radix `W`'s
/// high bits, its low 96 bits filling `x`, `y` and `z` (payload §3). Otherwise the line naming `fgw_w.payload`, which
/// refuses generation, as [`fgw_length`]'s does.
pub fn fgw_payload(entries: &[Entry]) -> Result<(u32, u32), String> {
    entries
        .iter()
        .find_map(|e| match e.location {
            Location::Packed {
                word,
                offset,
                width,
            } if word == FGW_WORD && e.name == "payload" => Some((offset, width)),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "field `{FGW_WORD}.payload` has no entry: the word buffer's accessors need it (payload §3; \
                 dd_generation_root §3.8)"
            )
        })
}

/// The line refusing generation if `words` declare the word buffer's `.w`, `fgw_w`, and its `length` entry is not as
/// [`fgw_length`] needs it, or it has no `payload` entry ([`fgw_payload`]).
pub fn fgw_problem(words: &[Word], entries: &[Entry]) -> Option<String> {
    if words.iter().any(|w| w.name == FGW_WORD) {
        fgw_length(entries)
            .err()
            .or_else(|| fgw_payload(entries).err())
    } else {
        None
    }
}

/// The value of [`fgw`]'s `FGW_NO_SYMBOL`: one past the last of payload §3's symbol codes, so no symbol.
pub fn fgw_no_symbol() -> u32 {
    crate::payload::symbols().len() as u32
}

/// The identity permutation of payload §3's symbol codes, packed as `fgw_symbol` composes them: code `x`'s image in
/// bits `2x .. 2x + 2`.
pub fn fgw_identity() -> u32 {
    (0..fgw_no_symbol()).map(|x| x << (2 * x)).sum()
}

/// The word buffer's accessors, emitted from `fgw_w`'s `length` and `payload` entries (payload §3, §6; R-86's names),
/// each taking the whole `vec4<u32>` as `[u32; 4]`, as §3's `fgw_length_raw(w: vec4u)` does:
/// - `FGW_CAPACITY`, `length`'s range's greatest value, and `FGW_LENGTH_SENTINEL`, its sentinel;
/// - `fgw_length_raw`, `fgw_truncated`, `fgw_reduced_length_valid`, `fgw_reduced_length` and
///   `fgw_retained_prefix_length`, which read `.w` alone;
/// - `fgw_symbol(w, k)`, symbol `k` of the retained prefix, 0-based, or `FGW_NO_SYMBOL` past it, decoded sequentially
///   in O(length) (payload §3: the base-3 tail popped by depth, the residue `d₀`, the continuations replayed), through
///   `fgw_mixed_radix`, the integer `W`, `fgw_div3`, one pop, and `fgw_after`, one continuation composed;
/// - `fgw_pack`, the word from `W` and a length, which the kernel's append writes (`kernel::word`); the fragment side
///   only reads, so the WGSL emitter writes no such setter.
///
/// The driver refuses a ledger whose entries lack any of them ([`fgw_problem`]); an emitter called past it writes a
/// `compile_error!` naming the entry, so the file never builds without them.
pub fn fgw(entries: &[Entry]) -> String {
    let (offset, width, capacity, sentinel) = match fgw_length(entries) {
        Ok(length) => length,
        Err(why) => return format!("\ncompile_error!({why:?});\n"),
    };
    let (p_offset, p_width) = match fgw_payload(entries) {
        Ok(payload) => payload,
        Err(why) => return format!("\ncompile_error!({why:?});\n"),
    };
    let bits = format!("bits {offset}–{}", offset + width - 1);
    let p_bits = format!("bits {p_offset}–{}", p_offset + p_width - 1);
    let no_symbol = fgw_no_symbol();
    let identity = fgw_identity();
    format!(
        r#"
/// The word's capacity in symbols, `length`'s greatest valid value (payload §3; the register's `fgw_capacity`).
pub const FGW_CAPACITY: u32 = {capacity};

/// `length`'s sentinel in the ledger: the word is truncated (payload §3; dd_generation_root §3.8).
pub const FGW_LENGTH_SENTINEL: u32 = {sentinel};

/// `length_raw`: {bits} of the word's `.w`, element 3 of its `vec4<u32>`; 0…{capacity} valid, {sentinel} truncated
/// (payload §3). Never a crossing count: [`fgw_retained_prefix_length`] clamps the sentinel.
#[inline]
pub fn fgw_length_raw(w: [u32; 4]) -> u32 {{
    extract(w[3], {offset}, {width})
}}

/// Whether the word is truncated: `length_raw` is the sentinel (payload §3).
#[inline]
pub fn fgw_truncated(w: [u32; 4]) -> bool {{
    fgw_length_raw(w) == FGW_LENGTH_SENTINEL
}}

/// Whether the reduced crossing count is valid: only when the word is not truncated, as later cancellations are
/// untracked after the cap (payload §3, §5, §6).
#[inline]
pub fn fgw_reduced_length_valid(w: [u32; 4]) -> bool {{
    !fgw_truncated(w)
}}

/// The reduced crossing count, the net branch-cut crossings; use only when [`fgw_reduced_length_valid`] (payload §5,
/// §6).
#[inline]
pub fn fgw_reduced_length(w: [u32; 4]) -> u32 {{
    fgw_length_raw(w)
}}

/// The retained prefix's length: `length_raw`, the sentinel clamped to the capacity; debug and export only, never the
/// reduced crossing count (payload §3, §6; R-86).
#[inline]
pub fn fgw_retained_prefix_length(w: [u32; 4]) -> u32 {{
    if fgw_truncated(w) {{
        FGW_CAPACITY
    }} else {{
        fgw_length_raw(w)
    }}
}}

/// No symbol: one past the last symbol code. [`fgw_symbol`] returns it past the retained prefix, and the append's
/// `prev` holds it after a pop to the empty word (payload §3's `INVALID`).
pub const FGW_NO_SYMBOL: u32 = {no_symbol};

/// The mixed-radix integer `W` the word packs, as four 32-bit limbs, low first: `x`, `y`, `z`, then `.w`'s `payload`,
/// {p_bits} (payload §3).
#[inline]
pub fn fgw_mixed_radix(w: [u32; 4]) -> [u32; 4] {{
    [w[0], w[1], w[2], extract(w[3], {p_offset}, {p_width})]
}}

/// The word holding `W` (four limbs, low first, as [`fgw_mixed_radix`] reads them) and `length`, which the append
/// writes (payload §3).
#[inline]
pub fn fgw_pack(v: [u32; 4], length: u32) -> [u32; 4] {{
    [
        v[0],
        v[1],
        v[2],
        insert(insert(0, v[3], {p_offset}, {p_width}), length, {offset}, {width}),
    ]
}}

/// One limb of [`fgw_div3`]: `r · 2³² + limb`, `r` the remainder carried from the limb above (< 3), divided by 3 over
/// its two 16-bit halves, so no step exceeds a u32 (WGSL has no u64). The quotient limb and the remainder.
#[inline]
pub fn fgw_div3_limb(limb: u32, r: u32) -> (u32, u32) {{
    let hi = (r << 16) | (limb >> 16);
    let lo = ((hi % 3) << 16) | (limb & 0xffff);
    (((hi / 3) << 16) | (lo / 3), lo % 3)
}}

/// `v` (four limbs, low first) divided by 3, and the remainder: one pop of the base-3 tail, the remainder its digit
/// (payload §3). Long division from the high limb down, by constant indices.
#[inline]
pub fn fgw_div3(v: [u32; 4]) -> ([u32; 4], u32) {{
    let (q3, r) = fgw_div3_limb(v[3], 0);
    let (q2, r) = fgw_div3_limb(v[2], r);
    let (q1, r) = fgw_div3_limb(v[1], r);
    let (q0, r) = fgw_div3_limb(v[0], r);
    ([q0, q1, q2, q3], r)
}}

/// `perm` composed after digit `e`'s continuation: `x ↦ perm(continuation_symbol(x, e))`, each permutation of the
/// symbol codes packed with code `x`'s image in bits `2x .. 2x + 2` (payload §3's table, read through
/// [`continuation_symbol`]).
#[inline]
pub fn fgw_after(perm: u32, e: u32) -> u32 {{
    extract(perm, 2 * continuation_symbol(0, e), 2)
        | (extract(perm, 2 * continuation_symbol(1, e), 2) << 2)
        | (extract(perm, 2 * continuation_symbol(2, e), 2) << 4)
        | (extract(perm, 2 * continuation_symbol(3, e), 2) << 6)
}}

/// Symbol `k` of the word's retained prefix, 0-based (`d₀` is symbol 0), or [`FGW_NO_SYMBOL`] at or past its length.
/// Sequential, O(length), never random-access (payload §3): the base-3 tail is popped by depth down to the prefix
/// ending at symbol `k`, then its `k` digits are popped, last first, composing their continuations ([`fgw_after`]),
/// and the composition is applied to the residue, `d₀`. A truncated word's retained prefix is its 76 stored symbols,
/// a debug quantity, not the reduced word (payload §3, §5).
#[inline]
pub fn fgw_symbol(w: [u32; 4], k: u32) -> u32 {{
    let length = fgw_retained_prefix_length(w);
    if k >= length {{
        return FGW_NO_SYMBOL;
    }}
    let mut v = fgw_mixed_radix(w);
    let mut i = k + 1;
    while i < length {{
        v = fgw_div3(v).0;
        i += 1;
    }}
    let mut perm = {identity:#x};
    let mut j = 0;
    while j < k {{
        let (q, e) = fgw_div3(v);
        v = q;
        perm = fgw_after(perm, e);
        j += 1;
    }}
    extract(perm, 2 * (v[0] & 3), 2)
}}
"#
    )
}

/// `rows` as a Rust array literal, `[[a, b, …], …]`.
fn array(rows: &[[u32; 4]]) -> String {
    let rows: Vec<String> = rows
        .iter()
        .map(|r| format!("[{}, {}, {}, {}]", r[0], r[1], r[2], r[3]))
        .collect();
    format!("[{}]", rows.join(", "))
}

/// `cells` as a comparison chain on `var`, `if var == 0 { cells[0] } else if var == 1 { … } else { cells[n − 1] }`,
/// each cell's text indented to `depth` levels of four spaces, rustfmt's layout. The first cell opens the chain, the
/// middle cells are its `else if` arms, and the last is the `else`, so a `var` past the table's last code reads the
/// last cell; the callers pass codes only, each masking or clamping its inputs first (R-321, R-324). A last cell that is itself a chain continues this one, `else if …`, clippy's
/// collapsed form of `else { if … }`. No runtime array index is emitted: rust-gpu lowers one to an implicit bounds
/// check, a compiler-injected multi-level exit (GPU determinism note § "The discipline", rule 5).
fn select(var: &str, cells: &[String], depth: usize) -> String {
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let indent = |cell: &str, to: &str| cell.replace('\n', &format!("\n{to}"));
    let Some((first, rest)) = cells.split_first() else {
        return String::new();
    };
    let mut out = format!(
        "if {var} == 0 {{\n{inner}{}\n{pad}}}",
        indent(first, &inner)
    );
    let Some((last, middle)) = rest.split_last() else {
        return out;
    };
    for (code, cell) in (1..).zip(middle) {
        let _ = write!(
            out,
            " else if {var} == {code} {{\n{inner}{}\n{pad}}}",
            indent(cell, &inner)
        );
    }
    if last.starts_with("if ") {
        let _ = write!(out, " else {}", indent(last, &pad));
    } else {
        let _ = write!(out, " else {{\n{inner}{}\n{pad}}}", indent(last, &inner));
    }
    out
}

/// The stored buffers' bindings in the fragment's group 1, from the ledger's one table ([`crate::payload::bindings`],
/// R-343): `<PREFIX>_GROUP` and `<PREFIX>_BINDING` for each, which the host's bind group layouts read. The WGSL emitter
/// writes the same constants and numbers for the unpack layer's two ([`super::wgsl`]), and `render::bind` for the
/// others.
pub fn bindings() -> String {
    let mut out = String::new();
    for b in crate::payload::bindings() {
        let _ = write!(
            out,
            "\n/// `{buf}`'s bind group in the fragment (R-343).\n\
             pub const {c}_GROUP: u32 = {g};\n\
             \n/// `{buf}`'s binding number in its group (R-343).\n\
             pub const {c}_BINDING: u32 = {n};\n",
            buf = b.buffer,
            c = b.constant,
            g = b.group,
            n = b.binding,
        );
    }
    out
}

/// `row` as cells: its values as literals.
fn cells(row: &[u32]) -> Vec<String> {
    row.iter().map(u32::to_string).collect()
}

/// `table[outer][inner]` as a comparison chain on `outer`, each cell a chain on `inner` ([`select`]), at one level
/// of indentation, a function body's.
fn select2(outer: &str, inner: &str, table: &[[u32; 4]]) -> String {
    let rows: Vec<String> = table.iter().map(|r| select(inner, &cells(r), 0)).collect();
    select(outer, &rows, 1)
}

/// Payload §3's frozen continuation table, from the ledger's ([`crate::payload`]): the arrays `INVERSE`,
/// `CONT_SYMBOL`, `PREDECESSOR_SYMBOL` and `CONTINUATION_INDEX` (3 in its four `next = inverse(prev)` cells, R-307),
/// and §3's small tables as functions: `inverse`, `continuation_symbol`, `predecessor_symbol` and
/// `continuation_index`. The arrays are data, for host code; each function is a comparison chain over the same table's
/// literals ([`select`]), never a runtime index into an array, which rust-gpu would bounds-check (GPU determinism note
/// § "The discipline", rule 5). Each function is total (R-321, R-324): it `debug_assert!`s each symbol input < 4 and
/// each digit < 3, then reads the table at the symbol masked to 2 bits (`& 3`) and the digit clamped (`min(d, 2)`), so
/// a release build given an input out of range reads the cell at the masked or clamped input, and
/// `continuation_index` returns 3 only in its inverse cells (R-307). The two functions that take a digit are not
/// `const`: `u32::min` is not a `const fn`. `CONTINUATION_INDEX`'s array is on its own line, rustfmt's layout for a line
/// past 100 columns.
pub fn continuation() -> String {
    use crate::payload::{cont_symbol, continuation_index, inverse, predecessor_symbol};
    let i = inverse();
    format!(
        r#"
/// Payload §3's frozen `inverse` (symbol codes `a = 0, A = 1, b = 2, B = 3`): part of the binary format.
pub const INVERSE: [u32; 4] = [{}, {}, {}, {}];

/// Payload §3's frozen `cont_symbol`: `next = CONT_SYMBOL[digit][prev]`.
pub const CONT_SYMBOL: [[u32; 4]; 3] = {};

/// `predecessor_symbol`, `prev = PREDECESSOR_SYMBOL[digit][next]`: `CONT_SYMBOL` inverted, so equal to it (payload §3).
pub const PREDECESSOR_SYMBOL: [[u32; 4]; 3] = {};

/// `continuation_index`, `digit = CONTINUATION_INDEX[prev][next]`: `CONT_SYMBOL` inverted, and 3 ("invalid") where
/// `next = inverse(prev)` (R-307, payload §3).
pub const CONTINUATION_INDEX: [[u32; 4]; 4] =
    {};

/// The inverse of symbol `s` (payload §3): `INVERSE[s]`, as a comparison chain, not an array index (GPU determinism
/// note § "The discipline", rule 5). `s` is a symbol code, 0…3: `debug_assert!`ed, then masked `& 3` (R-321).
#[inline]
pub const fn inverse(s: u32) -> u32 {{
    debug_assert!(s < 4, "s is not a symbol code (R-321)");
    let s = s & 3;
    {}
}}

/// The symbol digit `e` continues `prev` with (payload §3): `CONT_SYMBOL[e][prev]`, as a comparison chain. `e` is a
/// digit, 0…2: `debug_assert!`ed, then clamped `min(e, 2)` (R-324); `prev` a symbol code, 0…3: `debug_assert!`ed, then
/// masked `& 3` (R-321).
#[inline]
pub fn continuation_symbol(prev: u32, e: u32) -> u32 {{
    debug_assert!(prev < 4, "prev is not a symbol code (R-321)");
    debug_assert!(e < 3, "e is not a digit (R-324)");
    let prev = prev & 3;
    let e = e.min(2);
    {}
}}

/// The `prev` that digit `e` continued to `next`: the reverse table a cancellation-pop reads (payload §3):
/// `PREDECESSOR_SYMBOL[e][next]`, as a comparison chain. `e` is a digit, 0…2: `debug_assert!`ed, then clamped
/// `min(e, 2)` (R-324); `next` a symbol code, 0…3: `debug_assert!`ed, then masked `& 3` (R-321).
#[inline]
pub fn predecessor_symbol(next: u32, e: u32) -> u32 {{
    debug_assert!(next < 4, "next is not a symbol code (R-321)");
    debug_assert!(e < 3, "e is not a digit (R-324)");
    let next = next & 3;
    let e = e.min(2);
    {}
}}

/// The digit that continues `prev` with `s`; 3 where `s = inverse(prev)`, which the append never reads (R-307):
/// `CONTINUATION_INDEX[prev][s]`, as a comparison chain. `prev` and `s` are symbol codes, 0…3: each `debug_assert!`ed,
/// then masked `& 3` (R-321), so 3 is returned only in the four inverse cells.
#[inline]
pub const fn continuation_index(prev: u32, s: u32) -> u32 {{
    debug_assert!(prev < 4, "prev is not a symbol code (R-321)");
    debug_assert!(s < 4, "s is not a symbol code (R-321)");
    let prev = prev & 3;
    let s = s & 3;
    {}
}}
"#,
        i[0],
        i[1],
        i[2],
        i[3],
        array(&cont_symbol()),
        array(&predecessor_symbol()),
        array(&continuation_index()),
        select("s", &cells(&i), 1),
        select2("e", "prev", &cont_symbol()),
        select2("e", "next", &predecessor_symbol()),
        select2("prev", "s", &continuation_index()),
    )
}

/// Payload §2's `state` codes as constants, `STATE_<NAME>`, each the state's code in the ledger's table
/// ([`crate::payload::states`]); the `sd_is_*` predicates read them, never a literal code (W8).
pub fn state_codes() -> String {
    let mut out = String::new();
    for (code, name) in crate::payload::states().iter().enumerate() {
        let _ = write!(
            out,
            "\n/// The `state` code of {name} (payload §2).\npub const STATE_{}: u32 = {code};\n",
            name.to_uppercase()
        );
    }
    out
}

/// The fixed part of the accessor code: the bit helpers, the `no_std` binary16 conversion and the `pack2x16float` /
/// `unpack2x16float` equivalents, the ±65504 clamp (the register's `f16_finite_max`), the subnormal floor 2⁻²⁴ (the
/// register's `f16_min_subnormal`, R-278), and payload §6's accessors that
/// no single entry determines: the state predicates, `sd_last_symbol_valid` (the register's `fgw_length_sentinel`),
/// `total_substeps_log2` and the `times` fractions.
fn helpers() -> String {
    let f16_max = crate::constants::F16_FINITE_MAX.number();
    let f16_floor = crate::constants::F16_MIN_SUBNORMAL.number() as f32;
    let truncated = crate::constants::FGW_LENGTH_SENTINEL.number();
    format!(
        r#"
/// binary16's greatest finite value, the pack clamp (payload §1; the register's `f16_finite_max`).
pub const F16_FINITE_MAX: f32 = {f16_max:?};

/// binary16's smallest positive subnormal, 2⁻²⁴, and its bits (R-271; the register's `f16_min_subnormal`, R-278).
pub const F16_MIN_SUBNORMAL: f32 = {f16_floor:?};
pub const F16_MIN_SUBNORMAL_BITS: u32 = 0x0001;

/// Bits `offset .. offset + width` of `w`, `width` in 1..=32 (the u32 `extractBits`, payload §6).
#[inline]
pub const fn extract(w: u32, offset: u32, width: u32) -> u32 {{
    (w >> offset) & (u32::MAX >> (32 - width))
}}

/// `w` with bits `offset .. offset + width` replaced by the low `width` bits of `v`, every other bit kept (the u32
/// `insertBits`, payload §6).
#[inline]
pub const fn insert(w: u32, v: u32, offset: u32, width: u32) -> u32 {{
    let mask = (u32::MAX >> (32 - width)) << offset;
    (w & !mask) | ((v << offset) & mask)
}}

/// `x` clamped to binary16's finite range, ±65504, as payload §1 requires before packing; NaN stays NaN.
#[inline]
pub fn clamp_f16(x: f32) -> f32 {{
    x.clamp(-F16_FINITE_MAX, F16_FINITE_MAX)
}}

/// The binary16 bits of `x`, rounded to nearest, ties to even: one of the two results WGSL's `pack2x16float` may give
/// for an inexact value, and the exact one otherwise, subnormals kept. Past the finite range the result is ±∞, where
/// `pack2x16float` is indeterminate, so packers clamp first (payload §1). NaN stays NaN, keeping its payload's top bits.
pub fn f32_to_f16_bits(x: f32) -> u16 {{
    let b = x.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32;
    let man = b & 0x007f_ffff;
    if exp == 0xff {{
        let payload = (man >> 13) as u16;
        let nan = if man != 0 && payload == 0 {{
            0x0200
        }} else {{
            payload
        }};
        return sign | 0x7c00 | nan;
    }}
    let e = exp - 112;
    if e >= 0x1f {{
        return sign | 0x7c00;
    }}
    let (full, shift) = if e <= 0 {{
        (man | 0x0080_0000, (14 - e) as u32)
    }} else {{
        (man, 13)
    }};
    if shift >= 32 {{
        return sign;
    }}
    let q = full >> shift;
    let rem = full & ((1 << shift) - 1);
    let half = 1 << (shift - 1);
    let q = if rem > half || (rem == half && q & 1 == 1) {{
        q + 1
    }} else {{
        q
    }};
    let h = if e <= 0 {{ q }} else {{ ((e as u32) << 10) + q }};
    sign | h as u16
}}

/// The f32 value of the binary16 bits `h`, exact for every one of them (`unpack2x16float`'s conversion).
pub fn f16_bits_to_f32(h: u16) -> f32 {{
    let sign = u32::from(h & 0x8000) << 16;
    let exp = u32::from((h >> 10) & 0x1f);
    let man = u32::from(h & 0x03ff);
    if exp == 0 {{
        let magnitude = man as f32 * F16_MIN_SUBNORMAL;
        return f32::from_bits(sign | magnitude.to_bits());
    }}
    if exp == 0x1f {{
        return f32::from_bits(sign | 0x7f80_0000 | (man << 13));
    }}
    f32::from_bits(sign | ((exp + 112) << 23) | (man << 13))
}}

/// WGSL's `pack2x16float`: `v[0]` in bits 0–15, `v[1]` in bits 16–31. No clamp: callers clamp first (payload §1).
#[inline]
pub fn pack2x16float(v: [f32; 2]) -> u32 {{
    u32::from(f32_to_f16_bits(v[0])) | (u32::from(f32_to_f16_bits(v[1])) << 16)
}}

/// WGSL's `unpack2x16float`: `[bits 0–15, bits 16–31]` (`.x`, `.y`).
#[inline]
pub fn unpack2x16float(w: u32) -> [f32; 2] {{
    [f16_bits_to_f32(w as u16), f16_bits_to_f32((w >> 16) as u16)]
}}

/// Escape, bounded or collision: a resolved outcome. sim_failed and decode_failed are finished but not resolved, so
/// never gate re-dispatch on this (payload §6).
#[inline]
pub fn sd_is_resolved_outcome(w: u32) -> bool {{
    let s = sd_state(w);
    s == STATE_ESCAPE || s == STATE_BOUNDED || s == STATE_COLLISION
}}

/// Still marching: `STATE_RUNNING` (payload §6).
#[inline]
pub fn sd_is_running(w: u32) -> bool {{
    sd_state(w) == STATE_RUNNING
}}

/// Untrusted: neither a resolved outcome nor running, so sim_failed, decode_failed and the reserved codes (payload §2,
/// §6).
#[inline]
pub fn sd_is_failed(w: u32) -> bool {{
    !sd_is_resolved_outcome(w) && !sd_is_running(w)
}}

/// Finished, so the scheduler stops marching it: every state but running, the reserved 6–7 included (payload §2, §6).
#[inline]
pub fn sd_is_finished(w: u32) -> bool {{
    !sd_is_running(w)
}}

/// Whether `last_symbol` is meaningful, from the sidecar word's `length_raw`: at least 1 and not the truncation
/// sentinel (payload §2, §6).
#[inline]
pub fn sd_last_symbol_valid(len: u32) -> bool {{
    len >= 1 && len != {truncated}
}}

/// The complexity proxy, ⌊log₂ total⌋ and 0 for a total ≤ 1, derived from the exact `total_substeps` (payload §6, R-86).
#[inline]
pub fn total_substeps_log2(total: u32) -> u32 {{
    if total > 1 {{
        31 - total.leading_zeros()
    }} else {{
        0
    }}
}}

/// `t_end_step / horizon_steps`, 0 when `horizon_steps` is 0 (payload §2, §6).
#[inline]
pub fn tm_t_end_fraction(w: u32, horizon_steps: u32) -> f32 {{
    if horizon_steps > 0 {{
        tm_t_end_step(w) as f32 / horizon_steps as f32
    }} else {{
        0.0
    }}
}}

/// `t_dmin_step / horizon_steps`, 0 when `horizon_steps` is 0 (payload §2, §6).
#[inline]
pub fn tm_t_dmin_fraction(w: u32, horizon_steps: u32) -> f32 {{
    if horizon_steps > 0 {{
        tm_t_dmin_step(w) as f32 / horizon_steps as f32
    }} else {{
        0.0
    }}
}}
"#,
        truncated = truncated as u32,
    )
}

/// Every mismatch between `structs` and the ledger: a member stored as other than its entry's type or word's width, an
/// indexed struct's member off its entry's scalar index, trailing padding, a member that is neither an entry, a word,
/// nor `_`-prefixed and not listed in `pending`, or a packed word with no accessor prefix ([`PREFIXES`]).
pub fn check(
    structs: &[Struct],
    words: &[Word],
    entries: &[Entry],
    pending: &[&str],
) -> Vec<String> {
    let mut found: Vec<String> = words
        .iter()
        .filter(|w| !PREFIXES.iter().any(|(name, _)| *name == w.name))
        .map(|w| {
            format!(
                "packed word `{}` has no accessor prefix (payload §6)",
                w.name
            )
        })
        .collect();
    for s in structs {
        let (offsets, size) = offsets(s);
        let used: u32 = s.members.iter().map(|m| m.storage.size()).sum();
        if used != size {
            found.push(format!("`{}` pads {} B", s.name, size - used));
        }
        for (m, at) in s.members.iter().zip(offsets) {
            let at_name = format!("`{}.{}`", s.name, m.name);
            if let Some(w) = words.iter().find(|w| w.name == m.name) {
                if m.storage.size() * 8 != w.bits {
                    found.push(format!("{at_name} is not {} bits wide", w.bits));
                }
            } else if let Some(e) = entries.iter().find(|e| e.name == m.name) {
                let fits = match (&e.ty, m.storage) {
                    (FieldType::F32, Storage::F32) => true,
                    (FieldType::UBits, Storage::U32) => true,
                    (FieldType::UBits, Storage::U16) => {
                        matches!(e.range.hi, Bound::Closed(x) if x <= f64::from(u16::MAX))
                    }
                    (FieldType::Vector { component, k: 6 }, Storage::Vec2x3) => {
                        **component == FieldType::F32
                    }
                    _ => false,
                };
                if !fits {
                    found.push(format!(
                        "{at_name} does not store its entry's type {:?}",
                        e.ty
                    ));
                }
                match e.location {
                    Location::Scalar(slot) if s.indexed && slot * 4 != at => {
                        found.push(format!("{at_name} is at byte {at}, not slot {slot}"));
                    }
                    Location::Scalar(_) => {}
                    _ => found.push(format!("{at_name}'s entry is not at a scalar index")),
                }
            } else if !m.name.starts_with('_') && !pending.contains(&m.name) {
                found.push(format!("{at_name} has no ledger entry or word"));
            }
        }
    }
    found
}
