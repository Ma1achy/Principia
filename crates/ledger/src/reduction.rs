//! `QuadReduction`, the per-quad summary and the sole automatic GPU→CPU return of simulation data (dd_generation_root
//! §3.7): its ledger, the §3.8 entries of every §3.7 member in nine packed words and four scalar members ([`ledger`]),
//! the struct generated from it ([`quad_reduction`]), and its emitter ([`emit`]), which writes the struct as Rust for
//! the engine and as WGSL for the render and kernel sides, each with the read accessors of its packed words. Its layout
//! is §3.7's "Layout and §3.8 entries" (REQ-PAY-077, REQ-PAY-094): `class_histogram`'s 32 u32 bins, one per `class ⊕
//! detail` code, a copy's code reading `detail` as 0 in a state payload §2 gives it no meaning ([`detail_states`];
//! REQ-PAY-075, R-410), then the words `qr_w0` … `qr_w8`, then the three f32 members; 176 B, aligned to
//! 4. An f16 member is §3.8's `f16`, one binary16 value at a packed 16-bit location, read widened to f32 (R-312,
//! R-317).
//!
//! It is not payload: neither the payload ledger ([`crate::layout`]) nor its structs ([`crate::payload::structs`]) hold
//! it, so the payload's emitters, catalogue and export decoder never see it. The schema version hashes §3.7's member
//! list by name and type ([`crate::payload::QUAD_REDUCTION`], R-306), not this layout.

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::{self, rust, wgsl, GenError, Generated};
use crate::schema::{
    Bound, Consumer, Entry, EntryBuilder, FieldType, Ledger, Location, Member, Overflow,
    Provenance, Range, Scale, Span, Storage, Struct, Word,
};

/// Where the Rust struct is written, relative to the workspace root: the engine, which reads the reduction back.
pub const RUST_PATH: &str = "crates/engine/src/quad/generated.rs";

/// Where the WGSL struct is written, relative to the workspace root: one file the render and kernel sides both include.
pub const WGSL_PATH: &str = "crates/render/frag/generated/quad_reduction.wgsl";

/// The width of the `class ⊕ detail` code, from the payload ledger: `sample_descriptor`'s `state` and `detail` widths
/// (payload §2; dd_generation_root §3.1), 3 + 2 = 5 bits, the width of `dominant_outcome` (§3.7).
pub fn class_detail_bits() -> u32 {
    payload_width("state") + payload_width("detail")
}

/// The width of the payload ledger's packed field `name`, 0 if it has none.
fn payload_width(name: &str) -> u32 {
    crate::payload::ledger()
        .entries
        .iter()
        .filter(|e| e.name == Some(name))
        .filter_map(|e| match e.location {
            Some(Location::Packed { width, .. }) => Some(width),
            _ => None,
        })
        .sum()
}

/// The `state` codes whose `detail` a copy's bin index reads: the states payload §2 gives `detail` a meaning
/// ([`crate::payload::detail_meanings`]), escape 0, collision 2, sim_failed 4 and decode_failed 5. In every other state,
/// bounded, running and the reserved 6–7, the index reads `detail` as 0 (dd_generation_root §3.7, REQ-PAY-075).
pub fn detail_states() -> Vec<u32> {
    let states = crate::payload::states();
    crate::payload::detail_meanings()
        .iter()
        .filter_map(|(s, _)| states.iter().position(|n| n == s))
        .map(|k| k as u32)
        .collect()
}

/// `class_histogram`'s bin count, `N_bins`: one bin per `class ⊕ detail` code, 2⁵ = 32 (REQ-PAY-075; R-72).
pub fn bins() -> u32 {
    1 << class_detail_bits()
}

/// The most footprint copies one `class_histogram` bin counts: the greatest value of its range, 2³² − 1 for a u32 bin
/// (R-410). Dispatch asserts `N² × (E+1)` against it (REQ-SCHED-098); 0 if the ledger gives the bins no closed
/// greatest value.
pub fn bin_capacity() -> u64 {
    let histogram = ledger()
        .entries
        .into_iter()
        .find(|e| e.name == Some("class_histogram"));
    match histogram.and_then(|e| e.range).map(|r| r.hi) {
        Some(Bound::Closed(hi)) => hi as u64,
        _ => 0,
    }
}

/// The packed words, `qr_w0` … `qr_w8`, each 32 bits; `qr_w0`'s bit 7 is reserved (§3.7).
fn words() -> Vec<Word> {
    let word = |name, reserved| Word {
        name,
        bits: 32,
        reserved,
    };
    let mut out = vec![word(
        "qr_w0",
        vec![Span {
            offset: 7,
            width: 1,
        }],
    )];
    for name in [
        "qr_w1", "qr_w2", "qr_w3", "qr_w4", "qr_w5", "qr_w6", "qr_w7", "qr_w8",
    ] {
        out.push(word(name, vec![]));
    }
    out
}

/// A member's entry: every member is `provenance: reduction`, read by the scheduler and the debug tooling (§3.7).
fn member(
    name: &'static str,
    location: Location,
    ty: FieldType,
    scale: Scale,
    range: Range,
) -> EntryBuilder {
    EntryBuilder::new(name)
        .location(location)
        .ty(ty)
        .scale(scale)
        .range(range)
        .provenance(Provenance::Reduction)
        .consumers(&[Consumer::Scheduler, Consumer::Debug])
}

fn packed(word: &'static str, offset: u32, width: u32) -> Location {
    Location::Packed {
        word,
        offset,
        width,
    }
}

/// Unsigned bits `offset .. offset + width` of `word`, over `[0, hi]`.
fn ubits(
    name: &'static str,
    word: &'static str,
    offset: u32,
    width: u32,
    scale: Scale,
    hi: i64,
) -> EntryBuilder {
    let location = packed(word, offset, width);
    member(name, location, FieldType::UBits, scale, Range::int(0, hi))
}

/// An f16 fraction, `lin` over `[0, 1]`, at bits `offset .. offset + 16` of `word`.
fn fraction(name: &'static str, word: &'static str, offset: u32) -> EntryBuilder {
    let location = packed(word, offset, 16);
    member(name, location, FieldType::F16, Scale::Lin, Range::int(0, 1))
}

/// An f16 at bits `offset .. offset + 16` of `word` from `lo`, unbounded above, with its `overflow` (R-248).
fn unbounded(
    name: &'static str,
    word: &'static str,
    offset: u32,
    scale: Scale,
    lo: Bound,
    overflow: Overflow,
) -> EntryBuilder {
    let range = Range {
        lo,
        hi: Bound::Unbounded,
    };
    member(name, packed(word, offset, 16), FieldType::F16, scale, range).overflow(overflow)
}

/// `running_mean_divergence`'s declaration (REQ-PAY-109; R-72, R-545): §3.7's formula, a mean over the fixed sim-time
/// grid `t_j = j·Δ` of the footprints' mean `spread_shape`, each in `[0, 1]`, so `[0, 1]`, one-signed.
const RUNNING_MEAN_DIVERGENCE_3_7: &str = "dd_generation_root §3.7 \"Temporal accumulators\": \
     running_mean_divergence(t) = (1/n)·Σ D_j over the sim-time grid t_j = j·Δ (Δ = dt, or Δ_sync), \
     D_j the mean over the quad's footprints of spread_shape at t_j, each in [0, 1], a non-finite one read as 1, \
     so [0, 1] \
     (REQ-PAY-109; R-72, R-545)";

/// `coherence`'s declaration (REQ-PAY-109; R-72, R-545): §3.7's formula, `1 − n_unresolved / N²` with
/// `0 ≤ n_unresolved ≤ N²`, so `[0, 1]`, one-signed.
const COHERENCE_3_7: &str = "dd_generation_root §3.7 \"Validity and diagnostics\": \
     coherence = 1 − n_unresolved / N², 0 ≤ n_unresolved ≤ N² (R-315), so [0, 1] (REQ-PAY-109; R-72, R-545)";

/// §3.7's declaration of `alpha_area`'s and `alpha_energy`'s ranges (R-426, applied per R-369): its "§3.8 entries"
/// table gives each `(−∞, ∞)`, REQ-PAY-093's proposal (R-71, R-418) until the human confirms it at the M5 gate.
const PROPOSED_3_7: &str =
    "dd_generation_root §3.7 \"The §3.8 entries\": the member's row gives it (−∞, ∞), \
                            proposed by REQ-PAY-093 (R-71, R-418) until the M5 gate";

/// An f32 at scalar slot `slot`, over `lo .. hi`.
fn f32_at(name: &'static str, slot: u32, lo: Bound, hi: Bound) -> EntryBuilder {
    let range = Range { lo, hi };
    member(
        name,
        Location::Scalar(slot),
        FieldType::F32,
        Scale::Lin,
        range,
    )
}

/// Every member's §3.8 entry, as §3.7's "Layout and §3.8 entries" table gives it, in its order. The ranges and
/// `overflow` of `error_ratio`, `roundtrip_error`, `alpha_area`, `alpha_energy`, `worst_energy_drift`,
/// `escape_time_min`, `escape_time_max` and `mean_orbit_count_spread` are REQ-PAY-093's proposal, until the human
/// confirms them at the M5 gate (R-71, R-418). With no copy escaped, `escape_time_min` holds +∞ and `escape_time_max`
/// −∞, each its reduction's identity, as their sentinels (§3.7's "No escape (R-418)"; REQ-REF-006). Ledger data, the
/// members' §3.8 entries; not register constants (REQ-SYS-082).
fn entries() -> Vec<EntryBuilder> {
    let zero = Bound::Closed(0.0);
    let one = Bound::Closed(1.0);
    let histogram = FieldType::Vector {
        component: Box::new(FieldType::UBits),
        k: bins(),
    };
    let bins = i64::from(bins());
    let code = Scale::Categorical(bins as u32);
    vec![
        member(
            "class_histogram",
            Location::Scalar(0),
            histogram,
            Scale::Lin,
            Range::int(0, u32::MAX.into()),
        ),
        ubits(
            "dominant_outcome",
            "qr_w0",
            0,
            class_detail_bits(),
            code,
            bins - 1,
        ),
        ubits("spread_winner", "qr_w0", 5, 2, Scale::Categorical(2), 1),
        ubits("level", "qr_w0", 8, 8, Scale::Lin, u8::MAX.into()),
        fraction("outcome_impurity", "qr_w0", 16),
        fraction("terminated_fraction", "qr_w1", 0),
        fraction("spread_shape", "qr_w1", 16),
        fraction("spread_event", "qr_w2", 0),
        unbounded("error_ratio", "qr_w2", 16, Scale::Log, zero, Overflow::Inf),
        unbounded(
            "roundtrip_error",
            "qr_w3",
            0,
            Scale::Log,
            zero,
            Overflow::Inf,
        ),
        unbounded(
            "alpha_area",
            "qr_w3",
            16,
            Scale::Diverging,
            Bound::Unbounded,
            Overflow::Inf,
        )
        .cited(PROPOSED_3_7),
        unbounded(
            "alpha_energy",
            "qr_w4",
            0,
            Scale::Lin,
            Bound::Unbounded,
            Overflow::Inf,
        )
        .cited(PROPOSED_3_7),
        unbounded(
            "worst_energy_drift",
            "qr_w4",
            16,
            Scale::Log,
            zero,
            Overflow::Inf,
        ),
        ubits("n_unresolved", "qr_w5", 0, 16, Scale::Lin, u16::MAX.into()),
        fraction("suspect_fraction", "qr_w5", 16),
        fraction("saturated_fraction", "qr_w6", 0),
        ubits(
            "valid_sample_count",
            "qr_w6",
            16,
            16,
            Scale::Lin,
            u16::MAX.into(),
        ),
        // With no copy escaped: +∞, the min reduction's identity, with no branch (R-418).
        unbounded(
            "escape_time_min",
            "qr_w7",
            0,
            Scale::Lin,
            zero,
            Overflow::Saturate,
        )
        .sentinel(f64::INFINITY),
        // With no copy escaped: −∞, the max reduction's identity, with no branch (R-418).
        unbounded(
            "escape_time_max",
            "qr_w7",
            16,
            Scale::Lin,
            zero,
            Overflow::Saturate,
        )
        .sentinel(f64::NEG_INFINITY),
        fraction("retrograde_fraction", "qr_w8", 0),
        unbounded(
            "mean_orbit_count_spread",
            "qr_w8",
            16,
            Scale::Lin,
            zero,
            Overflow::Saturate,
        ),
        f32_at("running_mean_divergence", 41, zero, one).cited(RUNNING_MEAN_DIVERGENCE_3_7),
        // Held until the first crossing: +∞, the minimum of the empty set of crossing times (§3.7, REQ-PAY-094).
        f32_at("first_divergence_t", 42, zero, Bound::Unbounded).sentinel(f64::INFINITY),
        f32_at("coherence", 43, zero, one).cited(COHERENCE_3_7),
    ]
}

/// `QuadReduction`'s ledger: its packed words and its members' entries.
pub fn ledger() -> Ledger {
    Ledger {
        words: words(),
        entries: entries(),
    }
}

/// The `QuadReduction` struct generated from [`ledger`]: `class_histogram`'s bins, then the packed words in order,
/// then the three f32 members, each a 4-byte slot, aligned to 4, with no padding (§3.7, REQ-PAY-077). Its buffer is
/// the reduction buffer, one element per quad. Ledger data, the struct's layout (REQ-SYS-082).
pub fn quad_reduction() -> Struct {
    let mut members = vec![Member {
        name: "class_histogram",
        storage: Storage::U32Array(bins()),
    }];
    members.extend(words().iter().map(|w| Member {
        name: w.name,
        storage: Storage::U32,
    }));
    members.extend(
        ["running_mean_divergence", "first_divergence_t", "coherence"].map(|name| Member {
            name,
            storage: Storage::F32,
        }),
    );
    Struct {
        name: "QuadReduction",
        align: 4,
        buffer: Some("reduction"),
        indexed: true,
        members,
    }
}

/// `ledger`'s entries, validated against §3.8 ([`gen::validate`]) and passed through the static layout check
/// ([`crate::check::check`]), or why generation is refused, naming each field.
pub fn validated(ledger: &Ledger) -> Result<Vec<Entry>, String> {
    let entries = gen::validate(ledger).map_err(|e| e.to_string())?;
    let found = crate::check::check(&ledger.words, &entries);
    if found.is_empty() {
        Ok(entries)
    } else {
        Err(GenError::Layout(found).to_string())
    }
}

/// The emitter, registered in [`gen::EMITTERS`]: `QuadReduction`'s two files ([`files`]) when the layout generated
/// from is the payload's, whose run `cargo xtask codegen` makes; nothing for a layout that declares none of the payload
/// structs' members (a test fixture's), as the payload's struct emitters do ([`rust::declares`]).
pub fn emit(words: &[Word], entries: &[Entry]) -> Vec<Generated> {
    if rust::declares(&crate::payload::structs(), words, entries) {
        files(&ledger(), &quad_reduction())
    } else {
        Vec::new()
    }
}

/// The Rust and WGSL files for `s` from `ledger`. A ledger [`validated`] refuses gives files that fail to build,
/// naming why: a `compile_error!` and a `const_assert false`, so generation fails loudly (§3.8).
pub fn files(ledger: &Ledger, s: &Struct) -> Vec<Generated> {
    let (rust, wgsl) = match validated(ledger) {
        Ok(entries) => (
            rust_file(s, &ledger.words, &entries),
            wgsl_file(s, &ledger.words, &entries),
        ),
        Err(why) => (
            format!("compile_error!({why:?});\n"),
            format!("const_assert false; // {}\n", why.replace('\n', " ")),
        ),
    };
    vec![
        Generated {
            path: PathBuf::from(RUST_PATH),
            contents: rust,
        },
        Generated {
            path: PathBuf::from(WGSL_PATH),
            contents: wgsl,
        },
    ]
}

/// One packed member: its entry, its word, and its bits `offset .. offset + width`.
struct Packed<'a> {
    entry: &'a Entry,
    word: &'static str,
    offset: u32,
    width: u32,
}

impl Packed<'_> {
    /// Its bits as a doc comment says them: "bits 0–4 of `qr_w0`".
    fn bits(&self) -> String {
        format!(
            "bits {}–{} of `{}`",
            self.offset,
            self.offset + self.width - 1,
            self.word
        )
    }
}

/// The packed members, word by word in `words`' order, each word's by offset.
fn packed_members<'a>(words: &[Word], entries: &'a [Entry]) -> Vec<Packed<'a>> {
    let mut out = Vec::new();
    for w in words {
        let mut fields: Vec<Packed> = entries
            .iter()
            .filter_map(|entry| match entry.location {
                Location::Packed {
                    word,
                    offset,
                    width,
                } if word == w.name => Some(Packed {
                    entry,
                    word,
                    offset,
                    width,
                }),
                _ => None,
            })
            .collect();
        fields.sort_by_key(|f| f.offset);
        out.extend(fields);
    }
    out
}

/// The f32 bits `b` as a hex literal with its halves split by `sep` (`_` in Rust, nothing in WGSL).
fn f32_bits(b: u32, sep: &str) -> String {
    format!("0x{:04x}{sep}{:04x}", b >> 16, b & 0xffff)
}

/// The value of the binary16 bits `h`, exactly, by IEEE 754's definition: sign, a 5-bit exponent biased by 15, a
/// 10-bit significand; subnormal at exponent 0, ±∞ at 31. A NaN pattern (exponent 31, significand not 0) reads ±∞
/// here too: [`binary16_bits`] never looks NaN up, and meets ±∞'s own patterns, `0x7c00` and `0xfc00`, first.
fn binary16_value(h: u16) -> f64 {
    let sign = if h & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exp = i32::from((h >> 10) & 0x1f);
    let man = f64::from(h & 0x3ff);
    sign * match exp {
        0 => man * 2f64.powi(-24),
        31 => f64::INFINITY,
        _ => (1.0 + man / 1024.0) * 2f64.powi(exp - 15),
    }
}

/// The binary16 bits of `value` where binary16 holds it exactly (±∞ and ±0 included), else `None`: NaN, which storage
/// never holds (R-79), or a value binary16 would round. The search runs up from 0, so ±∞ is found at its own pattern.
fn binary16_bits(value: f64) -> Option<u16> {
    if value.is_nan() {
        return None;
    }
    (0..=u16::MAX).find(|&h| binary16_value(h).to_bits() == value.to_bits())
}

/// A member's sentinel as the emitters write it.
enum Sentinel {
    /// An f32 member's, as its f32 bits.
    F32 { name: &'static str, bits: u32 },
    /// An f16 member's, as its binary16 bits at bits `offset .. offset + width` of its word.
    F16 {
        name: &'static str,
        bits: u16,
        offset: u32,
        width: u32,
    },
    /// One neither emitter can write, which fails the build naming the member.
    Refused { name: &'static str },
}

/// The sentinels of `entries`, in their order: an f32 member's, an f16 member's that binary16 holds exactly (R-418's
/// ±∞), and any other refused.
fn sentinels(entries: &[Entry]) -> Vec<Sentinel> {
    entries
        .iter()
        .filter_map(|e| {
            let s = e.sentinel?;
            let name = e.name;
            Some(match (&e.ty, &e.location, binary16_bits(s)) {
                (FieldType::F32, _, _) => Sentinel::F32 {
                    name,
                    bits: (s as f32).to_bits(),
                },
                (FieldType::F16, &Location::Packed { offset, width, .. }, Some(bits)) => {
                    Sentinel::F16 {
                        name,
                        bits,
                        offset,
                        width,
                    }
                }
                _ => Sentinel::Refused { name },
            })
        })
        .collect()
}

/// A copy's `class_histogram` bin index as source, in either language: `head` opens the function, `body` holds
/// `{reads}` (whether `state` reads `detail`, a disjunction over [`detail_states`], `false` if there is none) and
/// `{shift}` (`state`'s width), `suffix` is the language's unsigned-literal suffix and `doc` its comment marker.
fn bin_index(head: &str, body: &str, suffix: &str, doc: &str) -> String {
    let reads: Vec<String> = detail_states()
        .iter()
        .map(|s| format!("state == {s}{suffix}"))
        .collect();
    let reads = if reads.is_empty() {
        "false".to_owned()
    } else {
        reads.join(" || ")
    };
    format!(
        "{doc} The `class_histogram` bin of a copy with `state` and `detail` (`sample_descriptor` bits 0–2 and 3–4):\n\
         {doc} `state | d << {shift}`, `d` its `detail` in a state payload §2 gives `detail` a meaning (escape, collision,\n\
         {doc} sim_failed, decode_failed) and 0 in every other: bounded, running and the reserved 6–7. `dominant_outcome`\n\
         {doc} holds it (dd_generation_root §3.7, REQ-PAY-075).\n{head}{}",
        body.replace("{reads}", &reads)
            .replace("{shift}", &payload_width("state").to_string()),
        shift = payload_width("state"),
    )
}

/// The Rust file: the struct, `#[repr(C)]` with its members in order; its size and bin count; per packed member an
/// accessor `qr_<member>(w)` of its word, a `u32` for unsigned bits and an f32 for an `f16` (the binary16 value widened,
/// R-317); per f32 member with a sentinel, `QR_<MEMBER>_SENTINEL_BITS` and `qr_<member>_is_sentinel(v)`, tested by its
/// bits (R-297); then the private bit helper and the binary16 decode the payload's accessors read through
/// ([`rust::f16_decode`]). The file's comments are plain, not `//!`, so a test can `include!` it.
fn rust_file(s: &Struct, words: &[Word], entries: &[Entry]) -> String {
    let (_, size) = rust::offsets(s);
    let mut out = format!(
        "// Generated by `cargo xtask codegen` from the `QuadReduction` ledger (`crates/ledger/src/reduction.rs`); do not\n\
         // edit. `QuadReduction`, one per quad, the sole automatic GPU→CPU return of simulation data (dd_generation_root\n\
         // §3.7): its layout and the read accessors of its packed words. An f16 member is one binary16 value, read widened\n\
         // to f32 (R-312, R-317).\n\
         \n/// `{name}`: {size} B, aligned to {align} (dd_generation_root §3.7, \"Layout and §3.8 entries\").\n\
         #[repr(C, align({align}))]\n#[derive(Clone, Copy, Debug, Default, PartialEq)]\npub struct {name} {{\n",
        name = s.name,
        align = s.align,
    );
    for m in &s.members {
        let _ = writeln!(out, "    pub {}: {},", m.name, m.storage.rust());
    }
    let _ = write!(
        out,
        "}}\n\n/// `{name}`'s size in bytes (dd_generation_root §3.7).\npub const QUAD_REDUCTION_SIZE: usize = {size};\n\
         \n/// `class_histogram`'s bin count, one bin per `class ⊕ detail` code (dd_generation_root §3.7, REQ-PAY-075).\n\
         pub const CLASS_HISTOGRAM_BINS: usize = {bins};\n\n{bin}",
        name = s.name,
        bins = bins(),
        bin = bin_index(
            "#[inline]\npub fn qr_class_histogram_bin(state: u32, detail: u32) -> u32 {\n",
            "    let d = if {reads} {\n        detail\n    } else {\n        0\n    };\n    state | (d << {shift})\n}\n",
            "",
            "///"
        ),
    );
    for p in packed_members(words, entries) {
        let (f, o, n) = (p.entry.name, p.offset, p.width);
        let (doc, ty, get) = match p.entry.ty {
            FieldType::F16 => (
                (", one binary16 value widened to f32", "; R-317"),
                "f32",
                format!("f16_bits_to_f32(qr_extract(w, {o}, {n}) as u16)"),
            ),
            _ => (("", ""), "u32", format!("qr_extract(w, {o}, {n})")),
        };
        let _ = write!(
            out,
            "\n/// `{f}`: {}{} (dd_generation_root §3.7{}).\n#[inline]\npub fn qr_{f}(w: u32) -> {ty} {{\n    {get}\n}}\n",
            p.bits(),
            doc.0,
            doc.1
        );
    }
    for sentinel in sentinels(entries) {
        let _ = match sentinel {
            Sentinel::F32 { name: f, bits } => {
                let upper = format!("QR_{}_SENTINEL_BITS", f.to_uppercase());
                write!(
                    out,
                    "\n/// `{f}`'s sentinel as its f32 bits (dd_generation_root §3.7, REQ-PAY-094).\n\
                     pub const {upper}: u32 = {bits};\n\
                     \n/// Whether `v` is `{f}`'s sentinel: tested by its bits, never by a float comparison (R-297).\n\
                     #[inline]\npub fn qr_{f}_is_sentinel(v: f32) -> bool {{\n    v.to_bits() == {upper}\n}}\n",
                    bits = f32_bits(bits, "_"),
                )
            }
            Sentinel::F16 {
                name: f,
                bits,
                offset: o,
                width: n,
            } => {
                let upper = format!("QR_{}_SENTINEL_BITS", f.to_uppercase());
                write!(
                    out,
                    "\n/// `{f}`'s sentinel as its binary16 bits (dd_generation_root §3.7, R-418).\n\
                     pub const {upper}: u32 = {bits:#06x};\n\
                     \n/// Whether `{f}` in `w` is its sentinel: tested by its bits, never by a float comparison (R-297).\n\
                     #[inline]\npub fn qr_{f}_is_sentinel(w: u32) -> bool {{\n    qr_extract(w, {o}, {n}) == {upper}\n}}\n",
                )
            }
            Sentinel::Refused { name: f } => write!(
                out,
                "\ncompile_error!(\"`{f}` has a sentinel, which only an f32 member's or an f16 member's exact binary16 value may be here\");\n"
            ),
        };
    }
    let floor = crate::constants::F16_MIN_SUBNORMAL.number() as f32;
    let _ = write!(
        out,
        "\n/// Bits `offset .. offset + width` of `w`, `width` in 1..=32 (the u32 `extractBits`).\n\
         #[inline]\nconst fn qr_extract(w: u32, offset: u32, width: u32) -> u32 {{\n    \
         (w >> offset) & (u32::MAX >> (32 - width))\n}}\n\
         \n/// binary16's smallest positive subnormal, 2⁻²⁴ (the register's `f16_min_subnormal`, R-278).\n\
         const F16_MIN_SUBNORMAL: f32 = {floor:?};\n\n{}",
        rust::f16_decode(""),
    );
    out
}

/// The WGSL file: the struct, its members as [`wgsl::members`] maps them; its bin count; per packed member an accessor
/// `qr_<member>(w)` of its word through the u32 `extractBits`, an `f16` read by core `unpack2x16float` and widened to
/// f32, never an `f16` value (R-317); per f32 member with a sentinel, `QR_<MEMBER>_SENTINEL_BITS` and
/// `qr_<member>_is_sentinel(v)`, by its bits (R-297). Only reads: the writes are the passes that fill it.
fn wgsl_file(s: &Struct, words: &[Word], entries: &[Entry]) -> String {
    let (_, size) = rust::offsets(s);
    let mut out = format!(
        "// Generated by `cargo xtask codegen` from the `QuadReduction` ledger (`crates/ledger/src/reduction.rs`); do not\n\
         // edit. `QuadReduction`, one per quad (dd_generation_root §3.7), for the render and kernel sides: its layout and\n\
         // the read accessors of its packed words. WGSL: u32 `extractBits` only; an f16 member is read through core\n\
         // `unpack2x16float` and widened to f32, with no f16 extension and no f16 value (R-317).\n\
         \n// `{name}`: {size} B, aligned to {align} (dd_generation_root §3.7, \"Layout and §3.8 entries\").\n\
         struct {name} {{\n",
        name = s.name,
        align = s.align,
    );
    for m in wgsl::members(s) {
        let _ = writeln!(out, "    {}: {},", m.name, m.ty);
    }
    let _ = write!(
        out,
        "}}\n\n// `class_histogram`'s bin count, one bin per `class ⊕ detail` code (dd_generation_root §3.7).\n\
         const QR_CLASS_HISTOGRAM_BINS: u32 = {}u;\n\n{}",
        bins(),
        bin_index(
            "fn qr_class_histogram_bin(state: u32, detail: u32) -> u32 {\n",
            "    let d = select(0u, detail, {reads});\n    return state | (d << {shift}u);\n}\n",
            "u",
            "//"
        ),
    );
    for p in packed_members(words, entries) {
        let (f, o, n) = (p.entry.name, p.offset, p.width);
        let (doc, ty, get) = match p.entry.ty {
            FieldType::F16 => (
                (", read widened to f32", "; R-317"),
                "f32",
                format!("unpack2x16float(extractBits(w, {o}u, {n}u)).x"),
            ),
            _ => (("", ""), "u32", format!("extractBits(w, {o}u, {n}u)")),
        };
        let _ = write!(
            out,
            "\n// `{f}`: {}{} (dd_generation_root §3.7{}).\nfn qr_{f}(w: u32) -> {ty} {{ return {get}; }}\n",
            p.bits(),
            doc.0,
            doc.1
        );
    }
    for sentinel in sentinels(entries) {
        let _ = match sentinel {
            Sentinel::F32 { name: f, bits } => {
                let upper = format!("QR_{}_SENTINEL_BITS", f.to_uppercase());
                write!(
                    out,
                    "\n// `{f}`'s sentinel as its f32 bits (dd_generation_root §3.7).\nconst {upper}: u32 = {}u;\n\
                     \n// Whether `v` is `{f}`'s sentinel: tested by its bits, never by a float comparison (R-297).\n\
                     fn qr_{f}_is_sentinel(v: f32) -> bool {{ return bitcast<u32>(v) == {upper}; }}\n",
                    f32_bits(bits, ""),
                )
            }
            Sentinel::F16 {
                name: f,
                bits,
                offset: o,
                width: n,
            } => {
                let upper = format!("QR_{}_SENTINEL_BITS", f.to_uppercase());
                write!(
                    out,
                    "\n// `{f}`'s sentinel as its binary16 bits (dd_generation_root §3.7, R-418).\nconst {upper}: u32 = {bits:#06x}u;\n\
                     \n// Whether `{f}` in `w` is its sentinel: tested by its bits, never by a float comparison (R-297).\n\
                     fn qr_{f}_is_sentinel(w: u32) -> bool {{ return extractBits(w, {o}u, {n}u) == {upper}; }}\n",
                )
            }
            Sentinel::Refused { name: f } => write!(
                out,
                "\nconst_assert false; // `{f}` has a sentinel, which only an f32 member's or an f16 member's exact binary16 value may be here\n"
            ),
        };
    }
    out
}
