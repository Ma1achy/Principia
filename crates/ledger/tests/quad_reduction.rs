//! `QuadReduction`, the per-quad summary (dd_generation_root §3.7; TASK-M5-01), its ledger, its generated Rust struct
//! (`crates/engine/src/quad/generated.rs`, included here as written) and its generated WGSL struct:
//! - REQ-PAY-075: the bin count is the `class ⊕ detail` code space, each bin a u32 that counts Extreme's 16² × 16 copies
//!   and the bound 2³² − 1 without wrapping (`quad_reduction_histogram_capacity`).
//! - REQ-PAY-077: the ledger is §3.7's "Layout and §3.8 entries" table, and the generated Rust and WGSL layouts and
//!   accessors match it (`quad_reduction_layout_*`).
//! - REQ-REF-006, REQ-REF-007, REQ-REF-008, REQ-REF-052: the member list and types are §3.7's, with no `spread_t_end`
//!   and no CPU metadata (`quad_reduction_members_*`).
//! - REQ-PAY-089: `size_of::<QuadReduction>()` is the aligned sum of its member list, and the WGSL size agrees
//!   (`quad_reduction_size_*`).
//! - REQ-GEN-030: the generated Rust and WGSL `f16` accessors read the same binary16 value as f32, and no generated
//!   WGSL declares `enable f16` or an f16 value (`layout_static_f16_*`).
//!
//! Each test registers its negative control (R-176).

use std::mem::{offset_of, size_of};
use std::path::Path;

use ledger::gen::rust;
use ledger::payload;
use ledger::reduction::{self, RUST_PATH, WGSL_PATH};
use ledger::schema::{
    Bound, Entry, EntryBuilder, FieldType, Ledger, Location, Overflow, Scale, Storage, Struct,
};
use naga::{Module, Scalar, ScalarKind, TypeInner};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The generated Rust struct and accessors, as checked in.
mod generated {
    include!("../../engine/src/quad/generated.rs");
}

use generated::*;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn checked_in(path: &str) -> String {
    let path = root().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn generation_root() -> String {
    checked_in("docs/design/principia_dd_generation_root.md")
}

fn parse(source: &str) -> Module {
    naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)))
}

/// `ledger`'s entries, validated.
fn entries(ledger: &Ledger) -> Vec<Entry> {
    reduction::validated(ledger).unwrap_or_else(|e| panic!("{e}"))
}

/// The entry named `name` of `ledger`, for a test to change.
fn entry<'a>(ledger: &'a mut Ledger, name: &str) -> &'a mut EntryBuilder {
    ledger
        .entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("the ledger has no entry `{name}`"))
}

/// The packed width of the payload ledger's `name` (`sample_descriptor`'s `state` and `detail`, §3.1).
fn payload_width(name: &str) -> u32 {
    payload::ledger()
        .entries
        .iter()
        .find(|e| e.name == Some(name))
        .and_then(|e| match e.location {
            Some(Location::Packed { width, .. }) => Some(width),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the payload ledger has no packed `{name}`"))
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-075: the bin count and each bin's capacity.

/// Extreme's provisional `N` and `E` (memory_tiers §4; R-137, R-398): 16² footprints of 16 copies.
const EXTREME_COPIES: u64 = 16 * 16 * 16;

/// Medium's `N` and `E` (memory_tiers §4): 16² footprints of 2 copies.
const MEDIUM_COPIES: u64 = 16 * 16 * 2;

/// What a bin `bits` wide reads after counting `copies` copies, one at a time: the count modulo 2^bits, as unsigned
/// addition wraps.
fn counted(bits: u32, copies: u64) -> u64 {
    copies % (1u64 << bits)
}

/// The histogram's bins are the joint `class ⊕ detail` set: 2^(`state` + `detail` widths) of them, one per code of
/// `dominant_outcome`, which holds that code; each bin is a u32, stored as a full u32 slot; and a bin of `bin_bits`,
/// the ledger's bin width, counts each of `copies` without wrapping.
fn check_capacity(ledger: &Ledger, bin_bits: u32, copies: &[u64]) {
    let es = entries(ledger);
    let code = payload_width("state") + payload_width("detail");
    let bins = 1u32 << code;
    assert_eq!(
        (bins, reduction::bins(), reduction::class_detail_bits()),
        (32, 32, 5),
        "the bin count is not the 5-bit class ⊕ detail code space"
    );
    let get = |name| {
        es.iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no `{name}`"))
    };
    let histogram = get("class_histogram");
    assert_eq!(
        (&histogram.ty, &histogram.location),
        (
            &FieldType::Vector {
                component: Box::new(FieldType::UBits),
                k: bins,
            },
            &Location::Scalar(0),
        ),
        "class_histogram is not one u-bits bin per class ⊕ detail code at a scalar index"
    );
    let dominant = get("dominant_outcome");
    assert_eq!(
        (&dominant.location, dominant.scale, dominant.range.hi),
        (
            &Location::Packed {
                word: "qr_w0",
                offset: 0,
                width: code,
            },
            Scale::Categorical(bins),
            Bound::Closed(f64::from(bins - 1)),
        ),
        "dominant_outcome is not the class ⊕ detail code, one category per bin"
    );
    let stored = reduction::quad_reduction().members[0];
    assert_eq!(
        (stored.name, stored.storage),
        ("class_histogram", Storage::U32Array(bins)),
        "each bin is not a full u32 slot"
    );
    let capacity = (1u64 << bin_bits) - 1;
    assert_eq!(
        (histogram.range.lo, histogram.range.hi),
        (Bound::Closed(0.0), Bound::Closed(capacity as f64)),
        "a bin's range is not its width's"
    );
    assert_eq!(
        reduction::bin_capacity(),
        u64::from(u32::MAX),
        "a bin counts up to 2³² − 1 (R-410)"
    );
    for &n in copies {
        assert_eq!(
            counted(bin_bits, n),
            n,
            "a {bin_bits}-bit bin wraps at {n} copies"
        );
    }
    let mut bin = 0u32;
    for _ in 0..EXTREME_COPIES {
        bin = bin.wrapping_add(1);
    }
    assert_eq!(u64::from(bin), EXTREME_COPIES, "a u32 bin wraps at 4096");
}

/// The width of a bin, from the ledger's histogram range: ⌈log₂(greatest + 1)⌉.
fn bin_bits(ledger: &Ledger) -> u32 {
    let es = entries(ledger);
    let histogram = es
        .iter()
        .find(|e| e.name == "class_histogram")
        .expect("histogram");
    match histogram.range.hi {
        Bound::Closed(hi) => (hi + 1.0).log2().ceil() as u32,
        other => panic!("the histogram's range has no closed greatest value: {other:?}"),
    }
}

#[test]
fn quad_reduction_histogram_capacity() {
    let l = reduction::ledger();
    let bound = u64::from(u32::MAX);
    check_capacity(&l, bin_bits(&l), &[MEDIUM_COPIES, EXTREME_COPIES, bound]);
    let doc = generation_root();
    assert!(
        doc.contains("**Dispatch asserts `N² × (E+1) ≤ 2³² − 1`**"),
        "§3.7 does not state the bound (R-410)"
    );
}

negative_control!(
    quad_reduction_histogram_capacity,
    "§3.7's former u8 bin wraps at Medium's 16² × 2 = 512 copies",
    expected = "a 8-bit bin wraps at 512 copies",
    {
        let mut l = reduction::ledger();
        entry(&mut l, "class_histogram").range = Some(ledger::schema::Range::int(0, 255));
        check_capacity(&l, bin_bits(&l), &[MEDIUM_COPIES]);
    }
);

negative_control!(
    quad_reduction_histogram_capacity_bins_short,
    "18 bins, only the codes payload §2 names, are not the 5-bit code space",
    expected = "class_histogram is not one u-bits bin per class ⊕ detail code",
    {
        let mut l = reduction::ledger();
        entry(&mut l, "class_histogram").ty = Some(FieldType::Vector {
            component: Box::new(FieldType::UBits),
            k: 18,
        });
        check_capacity(&l, 32, &[EXTREME_COPIES]);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-REF-006, REQ-REF-007, REQ-REF-008, REQ-REF-052: the member list and types.

/// A member's §3.7 type, from its entry: `u32 × N` for the histogram, `u8`, `u16`, `2 bits` and `packed` (the 5-bit
/// `class ⊕ detail`) for unsigned bits by width, `f16` for one binary16 value, `f32` for a scalar f32.
fn section_type(e: &Entry) -> &'static str {
    match (&e.ty, &e.location) {
        (FieldType::Vector { component, .. }, Location::Scalar(_))
            if **component == FieldType::UBits =>
        {
            "u32 × N"
        }
        (FieldType::UBits, Location::Packed { width: 8, .. }) => "u8",
        (FieldType::UBits, Location::Packed { width: 16, .. }) => "u16",
        (FieldType::UBits, Location::Packed { width: 2, .. }) => "2 bits",
        (FieldType::UBits, Location::Packed { width: 5, .. }) => "packed",
        (FieldType::F16, Location::Packed { width: 16, .. }) => "f16",
        (FieldType::F32, Location::Scalar(_)) => "f32",
        _ => "not a §3.7 type",
    }
}

/// REQ-REF-006's validity members and REQ-REF-052's `n_unresolved`, with their types.
const VALIDITY: [(&str, &str); 9] = [
    ("suspect_fraction", "f16"),
    ("saturated_fraction", "f16"),
    ("valid_sample_count", "u16"),
    ("coherence", "f32"),
    ("escape_time_min", "f16"),
    ("escape_time_max", "f16"),
    ("retrograde_fraction", "f16"),
    ("mean_orbit_count_spread", "f16"),
    ("n_unresolved", "u16"),
];

/// The CPU-side quad metadata, §3.7's names and the engine's (REQ-REF-008), and the conditional `spread_t_end`
/// (REQ-REF-007): none is a member.
const NOT_MEMBERS: [&str; 10] = [
    "spread_t_end",
    "priority",
    "cacheAge",
    "cache_age",
    "lifecycle",
    "computeCostMs",
    "compute_cost_ms",
    "gentime",
    "id",
    "running_max_divergence",
];

/// The ledger's members are §3.7's member list ([`payload::QUAD_REDUCTION`]), each of its §3.7 type;
/// the validity members and `n_unresolved` have REQ-REF-006's and REQ-REF-052's types; no member is `spread_t_end` or
/// CPU metadata; and the generated struct `s`, Rust `rust_src` and WGSL `wgsl_src` hold every member, by its struct
/// member or its accessor, and none of the non-members.
fn check_members(ledger: &Ledger, s: &Struct, rust_src: &str, wgsl_src: &str) {
    let es = entries(ledger);
    // The ledger is in layout order (§3.7's "Layout and §3.8 entries"), §3.7's tables in theirs: compared by name.
    let mut listed: Vec<(String, &str)> = es
        .iter()
        .map(|e| (e.name.to_owned(), section_type(e)))
        .collect();
    let mut section: Vec<(String, &str)> = payload::QUAD_REDUCTION
        .iter()
        .map(|m| (m.name.replace("[N]", ""), m.ty.unwrap_or("<none>")))
        .collect();
    listed.sort();
    section.sort();
    assert_eq!(listed, section, "the ledger's members differ from §3.7's");
    for (name, ty) in VALIDITY {
        assert!(
            listed.contains(&(name.to_owned(), ty)),
            "`{name}` is not a {ty} member (REQ-REF-006, REQ-REF-052)"
        );
    }
    let struct_names: Vec<&str> = s.members.iter().map(|m| m.name).collect();
    for name in NOT_MEMBERS {
        let found = es.iter().any(|e| e.name == name)
            || struct_names.contains(&name)
            || rust_src.contains(&format!("pub {name}:"))
            || rust_src.contains(&format!("qr_{name}("))
            || wgsl_src.contains(&format!("    {name}:"))
            || wgsl_src.contains(&format!("qr_{name}("));
        assert!(
            !found,
            "`{name}` is in the GPU struct (REQ-REF-007, REQ-REF-008)"
        );
    }
    for e in &es {
        let held = match e.location {
            Location::Packed { .. } => {
                rust_src.contains(&format!("pub fn qr_{}(w: u32)", e.name))
                    && wgsl_src.contains(&format!("fn qr_{}(w: u32)", e.name))
            }
            _ => {
                struct_names.contains(&e.name)
                    && rust_src.contains(&format!("    pub {}: ", e.name))
                    && wgsl_src.contains(&format!("    {}: ", e.name))
            }
        };
        assert!(held, "the generated struct does not hold `{}`", e.name);
    }
}

#[test]
fn quad_reduction_members_are_section_3_7() {
    check_members(
        &reduction::ledger(),
        &reduction::quad_reduction(),
        &checked_in(RUST_PATH),
        &checked_in(WGSL_PATH),
    );
}

negative_control!(
    quad_reduction_members_are_section_3_7,
    "a ledger with escape_time_min a u16 differs from §3.7",
    expected = "the ledger's members differ from §3.7's",
    {
        let mut l = reduction::ledger();
        let e = entry(&mut l, "escape_time_min");
        e.ty = Some(FieldType::UBits);
        e.range = Some(ledger::schema::Range::int(0, 65535));
        e.overflow = None;
        check_members(
            &l,
            &reduction::quad_reduction(),
            &checked_in(RUST_PATH),
            &checked_in(WGSL_PATH),
        );
    }
);

negative_control!(
    quad_reduction_members_cpu_metadata,
    "a generated struct with a `priority` member holds CPU metadata",
    expected = "`priority` is in the GPU struct",
    check_members(
        &reduction::ledger(),
        &reduction::quad_reduction(),
        &checked_in(RUST_PATH).replace(
            "    pub coherence: f32,",
            "    pub coherence: f32,\n    pub priority: f32,"
        ),
        &checked_in(WGSL_PATH),
    )
);

negative_control!(
    quad_reduction_members_missing_accessor,
    "a WGSL without `qr_level` does not hold `level`",
    expected = "the generated struct does not hold `level`",
    check_members(
        &reduction::ledger(),
        &reduction::quad_reduction(),
        &checked_in(RUST_PATH),
        &checked_in(WGSL_PATH).replace("fn qr_level(w: u32)", "fn qr_depth(w: u32)"),
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-077: the ledger is §3.7's layout table, and the generated layouts and accessors match it.

/// The rows of §3.7's "Layout and §3.8 entries" table in `doc`: member, location, type, scale, range, sentinel ·
/// overflow.
fn section_table(doc: &str) -> Vec<[String; 6]> {
    doc.lines()
        .skip_while(|l| *l != "#### Layout and §3.8 entries")
        .skip(1)
        .take_while(|l| !l.starts_with("#"))
        .filter_map(|l| {
            let cells: Vec<String> = l.split('|').map(|c| c.trim().to_owned()).collect();
            let name = cells
                .get(1)?
                .strip_prefix('`')?
                .strip_suffix('`')?
                .to_owned();
            Some([
                name,
                cells.get(2)?.clone(),
                cells.get(3)?.clone(),
                cells.get(4)?.clone(),
                cells.get(5)?.clone(),
                cells.get(6)?.clone(),
            ])
        })
        .collect()
}

fn type_name(ty: &FieldType) -> String {
    match ty {
        FieldType::UBits => "u-bits".to_owned(),
        FieldType::F32 => "f32".to_owned(),
        FieldType::F16 => "f16".to_owned(),
        FieldType::F16Pair => "f16-pair".to_owned(),
        FieldType::Fixed16 => "fixed16".to_owned(),
        FieldType::Vector { component, k } => format!("vector({}, {k})", type_name(component)),
    }
}

/// An entry as the table writes it.
fn row(e: &Entry) -> [String; 6] {
    let location = match e.location {
        Location::Packed {
            word,
            offset,
            width,
        } => format!("({word}, {offset}, {width})"),
        Location::Scalar(k) => format!("scalar-index {k}"),
        Location::Derived { .. } => "derived".to_owned(),
    };
    let scale = match e.scale {
        Scale::Lin => "lin".to_owned(),
        Scale::Log => "log".to_owned(),
        Scale::Cyclic => "cyclic".to_owned(),
        Scale::Diverging => "diverging".to_owned(),
        Scale::Categorical(n) => format!("categorical({n})"),
        Scale::Flag => "flag".to_owned(),
    };
    let lo = match e.range.lo {
        Bound::Closed(x) => format!("[{x}"),
        Bound::Open(x) => format!("({x}"),
        Bound::Unbounded => "(−∞".to_owned(),
    };
    let hi = match e.range.hi {
        Bound::Closed(x) => format!("{x}]"),
        Bound::Open(x) => format!("{x})"),
        Bound::Unbounded => "∞)".to_owned(),
    };
    let extra = match (e.sentinel, e.overflow) {
        (Some(s), _) if s == f64::INFINITY => "sentinel +∞".to_owned(),
        (Some(s), _) => format!("sentinel {s}"),
        (None, Some(Overflow::Inf)) => "overflow inf".to_owned(),
        (None, Some(Overflow::Saturate)) => "overflow saturate".to_owned(),
        (None, None) => "—".to_owned(),
    };
    [
        e.name.to_owned(),
        location,
        type_name(&e.ty),
        scale,
        format!("{lo}, {hi}"),
        extra,
    ]
}

/// The ledger's entries are `doc`'s §3.7 table, row by row, and every entry is `provenance: reduction` read by the
/// scheduler and the debug tooling, as the table's preamble says.
fn check_table(doc: &str, ledger: &Ledger) {
    let es = entries(ledger);
    let rows: Vec<[String; 6]> = es.iter().map(row).collect();
    assert_eq!(
        section_table(doc),
        rows,
        "the ledger differs from §3.7's layout table"
    );
    for e in &es {
        assert_eq!(
            (e.provenance, e.consumers.as_slice()),
            (
                ledger::schema::Provenance::Reduction,
                [
                    ledger::schema::Consumer::Scheduler,
                    ledger::schema::Consumer::Debug
                ]
                .as_slice()
            ),
            "`{}` is not a reduction read by the scheduler and the debug tooling",
            e.name
        );
    }
}

#[test]
fn quad_reduction_layout_ledger_is_the_section_3_7_table() {
    check_table(&generation_root(), &reduction::ledger());
}

negative_control!(
    quad_reduction_layout_ledger_is_the_section_3_7_table,
    "a table with alpha_area in qr_w3's low half differs from the ledger",
    expected = "the ledger differs from §3.7's layout table",
    check_table(
        &generation_root().replace(
            "| `alpha_area` | (qr_w3, 16, 16) |",
            "| `alpha_area` | (qr_w3, 0, 16) |"
        ),
        &reduction::ledger()
    )
);

/// One generated struct member: its name, its byte offset and its type as the language writes it.
type Laid = (String, u32, String);

/// The Rust struct as compiled: each member's offset by `offset_of!` and its type, and the struct's size.
fn rust_layout() -> (Vec<Laid>, usize) {
    let m = |name: &str, at: usize, ty: &str| (name.to_owned(), at as u32, ty.to_owned());
    (
        vec![
            m(
                "class_histogram",
                offset_of!(QuadReduction, class_histogram),
                "[u32; 32]",
            ),
            m("qr_w0", offset_of!(QuadReduction, qr_w0), "u32"),
            m("qr_w1", offset_of!(QuadReduction, qr_w1), "u32"),
            m("qr_w2", offset_of!(QuadReduction, qr_w2), "u32"),
            m("qr_w3", offset_of!(QuadReduction, qr_w3), "u32"),
            m("qr_w4", offset_of!(QuadReduction, qr_w4), "u32"),
            m("qr_w5", offset_of!(QuadReduction, qr_w5), "u32"),
            m("qr_w6", offset_of!(QuadReduction, qr_w6), "u32"),
            m("qr_w7", offset_of!(QuadReduction, qr_w7), "u32"),
            m("qr_w8", offset_of!(QuadReduction, qr_w8), "u32"),
            m(
                "running_mean_divergence",
                offset_of!(QuadReduction, running_mean_divergence),
                "f32",
            ),
            m(
                "first_divergence_t",
                offset_of!(QuadReduction, first_divergence_t),
                "f32",
            ),
            m("coherence", offset_of!(QuadReduction, coherence), "f32"),
        ],
        size_of::<QuadReduction>(),
    )
}

/// The WGSL struct `QuadReduction` in `source` as naga lays it out: each member's offset and type, and its size.
fn wgsl_layout(source: &str) -> (Vec<Laid>, u32) {
    let module = parse(source);
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("QuadReduction"))
        .expect("the WGSL declares QuadReduction");
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("QuadReduction is not a struct")
    };
    let name_of = |h: naga::Handle<naga::Type>| match &module.types[h].inner {
        TypeInner::Scalar(Scalar {
            kind: ScalarKind::Uint,
            width: 4,
        }) => "u32".to_owned(),
        TypeInner::Scalar(Scalar {
            kind: ScalarKind::Float,
            width: 4,
        }) => "f32".to_owned(),
        TypeInner::Array {
            base,
            size: naga::ArraySize::Constant(n),
            ..
        } if matches!(
            module.types[*base].inner,
            TypeInner::Scalar(Scalar {
                kind: ScalarKind::Uint,
                width: 4
            })
        ) =>
        {
            format!("[u32; {n}]")
        }
        other => format!("{other:?}"),
    };
    let laid = members
        .iter()
        .map(|m| (m.name.clone().unwrap_or_default(), m.offset, name_of(m.ty)))
        .collect();
    (laid, *span)
}

/// The ledger's struct as the emitters lay it out: each member's offset and its storage's Rust type, and its size.
fn ledger_layout(s: &Struct) -> (Vec<Laid>, u32) {
    let (offsets, size) = rust::offsets(s);
    let laid = s
        .members
        .iter()
        .zip(offsets)
        .map(|(m, at)| (m.name.to_owned(), at, m.storage.rust()))
        .collect();
    (laid, size)
}

/// The generated Rust and WGSL structs lay out `s` member for member; each scalar-located entry of `ledger` is the
/// member at byte 4·slot, stored as its type; each packed word is one u32 member; and every member is one of these.
fn check_layouts(ledger: &Ledger, s: &Struct, wgsl_src: &str) {
    let (want, size) = ledger_layout(s);
    let (rust_members, rust_size) = rust_layout();
    assert_eq!(
        (&rust_members, rust_size),
        (&want, size as usize),
        "the generated Rust QuadReduction differs from the ledger's layout"
    );
    assert_eq!(
        wgsl_layout(wgsl_src),
        (want.clone(), size),
        "the generated WGSL QuadReduction differs from the ledger's layout"
    );
    let es = entries(ledger);
    for (m, &(ref name, at, _)) in s.members.iter().zip(&want) {
        let word = ledger.words.iter().find(|w| w.name == m.name);
        let scalar = es.iter().find(|e| e.name == m.name);
        let ok = match (word, scalar) {
            (Some(w), None) => w.bits == 32 && m.storage == Storage::U32,
            (None, Some(e)) => {
                let fits = match (&e.ty, m.storage) {
                    (FieldType::F32, Storage::F32) => true,
                    (FieldType::Vector { component, k }, Storage::U32Array(n)) => {
                        **component == FieldType::UBits && *k == n
                    }
                    _ => false,
                };
                fits && e.location == Location::Scalar(at / 4)
            }
            _ => false,
        };
        assert!(
            ok,
            "`QuadReduction.{name}` at byte {at} is not its ledger word or entry"
        );
    }
    for w in &ledger.words {
        assert!(
            s.members.iter().any(|m| m.name == w.name),
            "word `{}` is not a QuadReduction member",
            w.name
        );
    }
}

#[test]
fn quad_reduction_layout_rust_and_wgsl_match_the_ledger() {
    check_layouts(
        &reduction::ledger(),
        &reduction::quad_reduction(),
        &checked_in(WGSL_PATH),
    );
}

negative_control!(
    quad_reduction_layout_rust_and_wgsl_match_the_ledger,
    "a WGSL struct with first_divergence_t and coherence swapped differs from the ledger",
    expected = "the generated WGSL QuadReduction differs from the ledger's layout",
    check_layouts(
        &reduction::ledger(),
        &reduction::quad_reduction(),
        &checked_in(WGSL_PATH).replace(
            "    first_divergence_t: f32,\n    coherence: f32,",
            "    coherence: f32,\n    first_divergence_t: f32,"
        ),
    )
);

negative_control!(
    quad_reduction_layout_scalar_off_its_slot,
    "a ledger with coherence at slot 40 is off the struct's member at byte 172",
    expected = "`QuadReduction.coherence` at byte 172 is not its ledger word or entry",
    {
        let mut l = reduction::ledger();
        entry(&mut l, "coherence").location = Some(Location::Scalar(40));
        check_layouts(&l, &reduction::quad_reduction(), &checked_in(WGSL_PATH));
    }
);

negative_control!(
    quad_reduction_layout_word_not_a_member,
    "a ledger with a tenth word the struct does not store",
    expected = "word `qr_w9` is not a QuadReduction member",
    {
        let mut l = reduction::ledger();
        let mut w = l.words[1].clone();
        w.name = "qr_w9";
        w.reserved = vec![ledger::schema::Span {
            offset: 0,
            width: 32,
        }];
        l.words.push(w);
        check_layouts(&l, &reduction::quad_reduction(), &checked_in(WGSL_PATH));
    }
);

/// The files the emitter writes are `on_disk`'s, and the payload's codegen run writes them.
fn check_checked_in(on_disk: &dyn Fn(&str) -> String) {
    let files = reduction::files(&reduction::ledger(), &reduction::quad_reduction());
    let paths: Vec<String> = files.iter().map(|f| f.path.display().to_string()).collect();
    assert_eq!(paths, [RUST_PATH, WGSL_PATH], "the emitter's files");
    for f in &files {
        let path = f.path.display().to_string();
        assert!(
            on_disk(&path) == f.contents,
            "the checked-in {path} is not the emitter's output: run `cargo xtask codegen`"
        );
    }
    let run = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS).expect("codegen");
    for f in &files {
        assert!(
            run.contains(f),
            "`cargo xtask codegen` does not write {}",
            f.path.display()
        );
    }
}

#[test]
fn quad_reduction_layout_checked_in_files_are_the_emitted_ones() {
    check_checked_in(&|p| checked_in(p));
}

negative_control!(
    quad_reduction_layout_checked_in_files_are_the_emitted_ones,
    "a checked-in WGSL with level at bits 8–14 is not the emitter's output",
    expected = "the checked-in crates/render/frag/generated/quad_reduction.wgsl is not the emitter's output",
    check_checked_in(&|p| checked_in(p).replace("extractBits(w, 8u, 8u)", "extractBits(w, 8u, 7u)"))
);

/// A ledger the static check refuses gives files that fail to build, naming the field: a `compile_error!` and a
/// `const_assert false`; and the emitter writes nothing for a layout that is not the payload's.
fn check_refused_files(l: &Ledger, field: &str) {
    let files = reduction::files(l, &reduction::quad_reduction());
    assert_eq!(files.len(), 2, "the emitter's files");
    assert!(
        files[0].contents.starts_with("compile_error!(") && files[0].contents.contains(field),
        "the Rust file of a refused ledger builds: {}",
        files[0].contents
    );
    assert!(
        files[1].contents.starts_with("const_assert false; //")
            && files[1].contents.contains(field),
        "the WGSL file of a refused ledger builds: {}",
        files[1].contents
    );
    assert!(
        reduction::emit(&l.words, &entries(&reduction::ledger())).is_empty(),
        "the emitter writes QuadReduction for a layout that is not the payload's"
    );
}

#[test]
fn quad_reduction_layout_refused_ledger_fails_to_build() {
    let mut l = reduction::ledger();
    entry(&mut l, "level").location = Some(Location::Packed {
        word: "qr_w0",
        offset: 8,
        width: 7,
    });
    check_refused_files(&l, "level");
}

negative_control!(
    quad_reduction_layout_refused_ledger_fails_to_build,
    "the ledger as written generates, so its files build",
    expected = "the Rust file of a refused ledger builds",
    check_refused_files(&reduction::ledger(), "level")
);

/// The f32 value of the binary16 bits `h`, from IEEE 754's definition: sign, a 5-bit exponent biased by 15, a 10-bit
/// significand; subnormal at exponent 0, ±∞ and NaN at 31.
fn binary16(h: u16) -> f32 {
    let sign = if h & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exp = i32::from((h >> 10) & 0x1f);
    let man = f64::from(h & 0x3ff);
    let magnitude = match exp {
        0 => man * 2f64.powi(-24),
        31 if man == 0.0 => f64::INFINITY,
        31 => f64::NAN,
        _ => (1.0 + man / 1024.0) * 2f64.powi(exp - 15),
    };
    (sign * magnitude) as f32
}

/// A generated accessor, by kind: unsigned bits as a `u32`, or one binary16 value as f32.
#[derive(Clone, Copy)]
enum Accessor {
    Bits(fn(u32) -> u32),
    Half(fn(u32) -> f32),
}

/// The generated Rust accessors, by member.
fn rust_accessors() -> Vec<(&'static str, Accessor)> {
    use Accessor::{Bits, Half};
    vec![
        ("dominant_outcome", Bits(qr_dominant_outcome)),
        ("spread_winner", Bits(qr_spread_winner)),
        ("level", Bits(qr_level)),
        ("outcome_impurity", Half(qr_outcome_impurity)),
        ("terminated_fraction", Half(qr_terminated_fraction)),
        ("spread_shape", Half(qr_spread_shape)),
        ("spread_event", Half(qr_spread_event)),
        ("error_ratio", Half(qr_error_ratio)),
        ("roundtrip_error", Half(qr_roundtrip_error)),
        ("alpha_area", Half(qr_alpha_area)),
        ("alpha_energy", Half(qr_alpha_energy)),
        ("worst_energy_drift", Half(qr_worst_energy_drift)),
        ("n_unresolved", Bits(qr_n_unresolved)),
        ("suspect_fraction", Half(qr_suspect_fraction)),
        ("saturated_fraction", Half(qr_saturated_fraction)),
        ("valid_sample_count", Bits(qr_valid_sample_count)),
        ("escape_time_min", Half(qr_escape_time_min)),
        ("escape_time_max", Half(qr_escape_time_max)),
        ("retrograde_fraction", Half(qr_retrograde_fraction)),
        ("mean_orbit_count_spread", Half(qr_mean_orbit_count_spread)),
    ]
}

/// The packed members of `ledger`, each `(name, offset, width, is f16)`, in the ledger's order.
fn packed_members(ledger: &Ledger) -> Vec<(&'static str, u32, u32, bool)> {
    entries(ledger)
        .iter()
        .filter_map(|e| match e.location {
            Location::Packed { offset, width, .. } => {
                Some((e.name, offset, width, e.ty == FieldType::F16))
            }
            _ => None,
        })
        .collect()
}

/// The binary16 patterns the accessors are read at: zeros, the smallest subnormal and the largest, the smallest
/// normal, 1, a third, the finite extremes ±65504, ±∞, and −2.
const HALVES: [u16; 12] = [
    0x0000, 0x8000, 0x0001, 0x03ff, 0x0400, 0x3c00, 0x3555, 0x7bff, 0xfbff, 0x7c00, 0xfc00, 0xc000,
];

/// The word with `value` in bits `offset .. offset + width` and every other bit set, so an accessor that reads past
/// its bits reads ones.
fn word_with(offset: u32, width: u32, value: u32) -> u32 {
    let mask = (u32::MAX >> (32 - width)) << offset;
    !mask | ((value << offset) & mask)
}

/// Each generated Rust accessor of `accessors` reads its member's bits per `ledger`: unsigned bits as their value, an
/// f16 as its binary16 value widened to f32, at every value or pattern, with every other bit of the word set.
fn check_rust_accessors(ledger: &Ledger, accessors: &[(&str, Accessor)]) {
    let packed = packed_members(ledger);
    let names: Vec<&str> = accessors.iter().map(|a| a.0).collect();
    let want: Vec<&str> = packed.iter().map(|p| p.0).collect();
    assert_eq!(names, want, "the Rust accessors are not the packed members");
    for (&(name, offset, width, _), &(_, accessor)) in packed.iter().zip(accessors) {
        match accessor {
            Accessor::Bits(read) => {
                let top = u32::MAX >> (32 - width);
                for v in [0, 1, top / 2, top - 1, top] {
                    let got = read(word_with(offset, width, v));
                    assert_eq!(got, v, "Rust qr_{name} reads {got}, not {v}");
                }
            }
            Accessor::Half(read) => {
                for h in HALVES {
                    let got = read(word_with(offset, width, u32::from(h)));
                    let want = binary16(h);
                    assert_eq!(
                        got.to_bits(),
                        want.to_bits(),
                        "Rust qr_{name} reads {got} at {h:#06x}, not {want}"
                    );
                }
            }
        }
    }
}

#[test]
fn quad_reduction_layout_rust_accessors_read_the_ledger_bits() {
    check_rust_accessors(&reduction::ledger(), &rust_accessors());
}

negative_control!(
    quad_reduction_layout_rust_accessors_read_the_ledger_bits,
    "a ledger with error_ratio in qr_w2's low half is not where its accessor reads",
    expected = "Rust qr_spread_event reads",
    {
        let mut l = reduction::ledger();
        entry(&mut l, "spread_event").location = Some(Location::Packed {
            word: "qr_w2",
            offset: 16,
            width: 16,
        });
        entry(&mut l, "error_ratio").location = Some(Location::Packed {
            word: "qr_w2",
            offset: 0,
            width: 16,
        });
        check_rust_accessors(&l, &rust_accessors());
    }
);

/// The test kernel: for case `i`, the accessor of member `i % members` of the word `a[i]`, an f16's value by its bits.
fn accessor_kernel(packed: &[(&str, u32, u32, bool)]) -> String {
    let mut cases = String::new();
    for (j, &(name, _, _, half)) in packed.iter().enumerate() {
        let read = if half {
            format!("bitcast<u32>(qr_{name}(w))")
        } else {
            format!("qr_{name}(w)")
        };
        cases.push_str(&format!("        case {j}u: {{ out[i] = {read}; }}\n"));
    }
    format!(
        r"
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;

@compute @workgroup_size(64)
fn t_accessors(@builtin(global_invocation_id) id: vec3<u32>) {{
    let i = id.x;
    if i >= arrayLength(&a) {{ return; }}
    let w = a[i];
    switch i % {}u {{
{cases}        default: {{ out[i] = 0u; }}
    }}
}}
",
        packed.len()
    )
}

/// The generated WGSL accessors of `wgsl_src`, run on the GPU, read each word as the generated Rust accessors do: the
/// same unsigned value, and the same binary16 value as f32, bit for bit, at every pattern of [`HALVES`] in an f16's
/// bits and in the other half of its word.
fn check_wgsl_accessors(wgsl_src: &str) {
    let packed = packed_members(&reduction::ledger());
    let accessors = rust_accessors();
    let m = packed.len();
    let mut words = Vec::new();
    for (k, &h) in HALVES.iter().enumerate() {
        let other = u32::from(HALVES[(k + 5) % HALVES.len()]);
        for &(_, offset, width, _) in &packed {
            let mask = (u32::MAX >> (32 - width)) << offset;
            let background = other | (other << 16);
            words.push((background & !mask) | ((u32::from(h) << offset) & mask));
        }
    }
    let module = format!("{wgsl_src}\n{}", accessor_kernel(&packed));
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = gpu.run_wgsl(&module, "t_accessors", &[&words]);
    for (i, &w) in words.iter().enumerate() {
        let (name, accessor) = accessors[i % m];
        let want = match accessor {
            Accessor::Bits(read) => read(w),
            Accessor::Half(read) => read(w).to_bits(),
        };
        assert_eq!(
            got[i], want,
            "WGSL qr_{name}({w:#010x}) reads {:#010x}, the Rust accessor {want:#010x}",
            got[i]
        );
    }
}

#[test]
fn layout_static_f16_rust_and_wgsl_accessors_read_the_same_value() {
    check_wgsl_accessors(&checked_in(WGSL_PATH));
}

negative_control!(
    layout_static_f16_rust_and_wgsl_accessors_read_the_same_value,
    "a WGSL error_ratio that reads the word's low half reads another value than the Rust accessor",
    expected = "WGSL qr_error_ratio(",
    check_wgsl_accessors(&checked_in(WGSL_PATH).replace(
        "fn qr_error_ratio(w: u32) -> f32 { return unpack2x16float(extractBits(w, 16u, 16u)).x; }",
        "fn qr_error_ratio(w: u32) -> f32 { return unpack2x16float(w).x; }"
    ))
);

/// No generated WGSL in `sources` declares `enable f16` or holds an f16 value: its text has no `enable f16`, and naga's
/// module of it has no 16-bit float type (R-317).
fn check_no_f16(sources: &[(&str, String)]) {
    for (path, source) in sources {
        assert!(
            !source.contains("enable f16"),
            "{path} declares `enable f16` (R-317)"
        );
        let module = parse(source);
        for (_, ty) in module.types.iter() {
            let half = matches!(
                ty.inner,
                TypeInner::Scalar(Scalar {
                    kind: ScalarKind::Float,
                    width: 2
                }) | TypeInner::Vector {
                    scalar: Scalar {
                        kind: ScalarKind::Float,
                        width: 2
                    },
                    ..
                }
            );
            assert!(!half, "{path} holds an f16 value (R-317)");
        }
    }
}

#[test]
fn layout_static_f16_generated_wgsl_declares_no_f16() {
    check_no_f16(&[(WGSL_PATH, checked_in(WGSL_PATH))]);
}

negative_control!(
    layout_static_f16_generated_wgsl_declares_no_f16,
    "a WGSL that enables f16 and widens through an f16 value must fail",
    expected = "declares `enable f16`",
    check_no_f16(&[(WGSL_PATH, format!("enable f16;\n{}", checked_in(WGSL_PATH)))])
);

negative_control!(
    layout_static_f16_generated_wgsl_holds_an_f16_value,
    "a WGSL with an f16 constant, its `enable` spaced past the text check, holds an f16 value",
    expected = "holds an f16 value",
    check_no_f16(&[(
        WGSL_PATH,
        format!(
            "enable  f16;\n{}\nconst QR_HALF: f16 = 1.0h;\n",
            checked_in(WGSL_PATH)
        )
    )])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-089: the size.

/// `size_of::<QuadReduction>()` and the generated `QUAD_REDUCTION_SIZE` are the aligned sum of the ledger's member
/// list `s`, as are naga's size of the WGSL struct and §3.7's stated size; and the bin count constant is the ledger's.
fn check_size(s: &Struct, wgsl_src: &str, doc: &str) {
    let sum: u32 = s.members.iter().map(|m| m.storage.size()).sum();
    let aligned = sum.next_multiple_of(s.align) as usize;
    assert_eq!(
        (size_of::<QuadReduction>(), QUAD_REDUCTION_SIZE),
        (aligned, aligned),
        "the Rust QuadReduction is not the aligned sum of its member list"
    );
    assert_eq!(
        wgsl_layout(wgsl_src).1 as usize,
        aligned,
        "the WGSL QuadReduction's size is not the Rust one"
    );
    assert!(
        doc.contains(&format!("**{aligned} B, aligned to {}**", s.align)),
        "§3.7 does not state the measured {aligned} B"
    );
    assert_eq!(
        CLASS_HISTOGRAM_BINS,
        reduction::bins() as usize,
        "the generated bin count is not the ledger's"
    );
    assert_eq!(
        QuadReduction::default().class_histogram.len(),
        CLASS_HISTOGRAM_BINS,
        "the Rust histogram does not hold one bin per code"
    );
}

#[test]
fn quad_reduction_size_is_the_aligned_member_list() {
    check_size(
        &reduction::quad_reduction(),
        &checked_in(WGSL_PATH),
        &generation_root(),
    );
    println!(
        "size_of::<QuadReduction>() = {} B",
        size_of::<QuadReduction>()
    );
}

negative_control!(
    quad_reduction_size_is_the_aligned_member_list,
    "a member list with one more f32 sums past the generated struct",
    expected = "the Rust QuadReduction is not the aligned sum of its member list",
    check_size(
        &{
            let mut s = reduction::quad_reduction();
            s.members.push(ledger::schema::Member {
                name: "extra",
                storage: Storage::F32,
            });
            s
        },
        &checked_in(WGSL_PATH),
        &generation_root()
    )
);

negative_control!(
    quad_reduction_size_wgsl_disagrees,
    "a WGSL struct with a 33-bin histogram is larger than the Rust one",
    expected = "the WGSL QuadReduction's size is not the Rust one",
    check_size(
        &reduction::quad_reduction(),
        &checked_in(WGSL_PATH).replace("array<u32, 32>", "array<u32, 33>"),
        &generation_root()
    )
);

// ---------------------------------------------------------------------------------------------------------------
// `first_divergence_t`'s sentinel (REQ-PAY-094): +∞, tested by its bits in both languages.

/// The Rust sentinel test holds at +∞'s bits only, and the WGSL declares the same bits.
fn check_sentinel(wgsl_src: &str) {
    assert_eq!(
        QR_FIRST_DIVERGENCE_T_SENTINEL_BITS,
        f32::INFINITY.to_bits(),
        "first_divergence_t's sentinel is not +∞"
    );
    for (v, is) in [
        (f32::INFINITY, true),
        (f32::MAX, false),
        (0.0, false),
        (f32::NEG_INFINITY, false),
        (f32::NAN, false),
    ] {
        assert_eq!(
            qr_first_divergence_t_is_sentinel(v),
            is,
            "qr_first_divergence_t_is_sentinel({v}) is not {is}"
        );
    }
    assert!(
        wgsl_src.contains("const QR_FIRST_DIVERGENCE_T_SENTINEL_BITS: u32 = 0x7f800000u;"),
        "the WGSL sentinel is not +∞'s bits"
    );
}

#[test]
fn quad_reduction_layout_first_divergence_t_sentinel_is_infinity() {
    check_sentinel(&checked_in(WGSL_PATH));
}

negative_control!(
    quad_reduction_layout_first_divergence_t_sentinel_is_infinity,
    "a WGSL sentinel of −1's bits is not +∞",
    expected = "the WGSL sentinel is not +∞'s bits",
    check_sentinel(&checked_in(WGSL_PATH).replace("0x7f800000u", "0xbf800000u"))
);
