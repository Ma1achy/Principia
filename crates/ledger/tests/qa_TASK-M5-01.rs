//! QA tests for TASK-M5-01 on the ledger side: `QuadReduction`'s ledger, its generated WGSL, and §3.8's `f16` type.
//! Written from the requirements and dd_generation_root §3.7 / §3.8, not from the implementation:
//! - REQ-PAY-077 / REQ-PAY-094 / REQ-PAY-093: the ledger is §3.7's "Layout and §3.8 entries" table, transcribed below
//!   row by row (location, type, scale, range, sentinel or overflow); every member is `provenance: reduction` read by
//!   the scheduler and the debug tooling; each f16 range lies within ±65504 or states its `overflow` (R-248). The
//!   ranges of `error_ratio`, `roundtrip_error`, `alpha_area`, `alpha_energy`, `worst_energy_drift` (REQ-PAY-093) and
//!   of `escape_time_min`, `escape_time_max`, `mean_orbit_count_spread` (RQ-268) are proposals awaiting the human;
//!   the test holds the PR to what §3.7 declares, and changes with a ruling.
//! - REQ-PAY-075 (R-410): the bin count is one bin per `class ⊕ detail` code, 2^(3+2) = 32, from `sample_descriptor`'s
//!   `state` (bits 0–2) and `detail` (bits 3–4) (§3.1); each bin is a u32 whose range ends at 2³² − 1, holding
//!   Extreme's provisional 16² × 16 = 4096; §3.7's former u8 bin fails at Medium's 16² × 2 = 512; §3.7 states the bound.
//! - REQ-REF-006 / REQ-REF-052: the validity members and `n_unresolved` carry §3.7's types.
//! - REQ-REF-007 / REQ-REF-008 / REQ-PAY-006: no `spread_t_end` and no CPU metadata anywhere in the ledger or the
//!   generated WGSL struct; every WGSL member is a fixed-size scalar or the fixed 32-bin array.
//! - REQ-PAY-089: naga lays the generated WGSL struct out at 176 B with §3.7's offsets; §3.7 and the docs citing it
//!   state 176 B.
//! - REQ-GEN-030: the static layout check on a small ledger of its own: an f16 field 15 or 17 bits wide, at a scalar
//!   index, as a vector component, beyond ±65504, or unbounded without `overflow` is refused naming the field; a 16-bit
//!   in-range one, and an unbounded one with `overflow`, pass; the generated WGSL holds no `enable f16` and no f16 type;
//!   the WGSL accessors read binary16 values whose bits are known, on the GPU.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use ledger::gen;
use ledger::reduction;
use ledger::schema::{
    Bound, Consumer, Entry, EntryBuilder, FieldType, Ledger, Location, Overflow, Provenance, Range,
    Scale, Span, Word,
};
use naga::{ArraySize, Module, Scalar, ScalarKind, TypeInner};
use validation::gpu::GpuHarness;
use validation::negative_control;

use Bound::{Closed, Unbounded};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(path: &str) -> String {
    let p = root().join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn quad_wgsl() -> String {
    read("crates/render/frag/generated/quad_reduction.wgsl")
}

fn parse(source: &str) -> Module {
    naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)))
}

fn built(l: &Ledger) -> Vec<Entry> {
    l.entries
        .iter()
        .map(|b| {
            b.build()
                .unwrap_or_else(|e| panic!("incomplete entry: {e:?}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------------------------
// §3.7's "Layout and §3.8 entries" table, transcribed.

/// A member's location in the table.
#[derive(Clone, Copy, Debug, PartialEq)]
enum At {
    Packed(&'static str, u32, u32),
    Scalar(u32),
}

/// The type column.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ty {
    UBits,
    F16,
    F32,
    VectorUBits(u32),
}

/// The "sentinel · overflow" column.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Extra {
    None,
    Inf,
    Saturate,
    SentinelPosInf,
}

type Row = (&'static str, At, Ty, Scale, Bound, Bound, Extra);

const INF: f64 = f64::INFINITY;

fn table() -> Vec<Row> {
    use At::{Packed as P, Scalar as S};
    use Extra as X;
    use Scale::*;
    use Ty::*;
    let u32max = 4_294_967_295.0;
    vec![
        (
            "class_histogram",
            S(0),
            VectorUBits(32),
            Lin,
            Closed(0.0),
            Closed(u32max),
            X::None,
        ),
        (
            "dominant_outcome",
            P("qr_w0", 0, 5),
            UBits,
            Categorical(32),
            Closed(0.0),
            Closed(31.0),
            X::None,
        ),
        (
            "spread_winner",
            P("qr_w0", 5, 2),
            UBits,
            Categorical(2),
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "level",
            P("qr_w0", 8, 8),
            UBits,
            Lin,
            Closed(0.0),
            Closed(255.0),
            X::None,
        ),
        (
            "outcome_impurity",
            P("qr_w0", 16, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "terminated_fraction",
            P("qr_w1", 0, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "spread_shape",
            P("qr_w1", 16, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "spread_event",
            P("qr_w2", 0, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "error_ratio",
            P("qr_w2", 16, 16),
            F16,
            Log,
            Closed(0.0),
            Unbounded,
            X::Inf,
        ),
        (
            "roundtrip_error",
            P("qr_w3", 0, 16),
            F16,
            Log,
            Closed(0.0),
            Unbounded,
            X::Inf,
        ),
        (
            "alpha_area",
            P("qr_w3", 16, 16),
            F16,
            Diverging,
            Unbounded,
            Unbounded,
            X::Inf,
        ),
        (
            "alpha_energy",
            P("qr_w4", 0, 16),
            F16,
            Lin,
            Unbounded,
            Unbounded,
            X::Inf,
        ),
        (
            "worst_energy_drift",
            P("qr_w4", 16, 16),
            F16,
            Log,
            Closed(0.0),
            Unbounded,
            X::Inf,
        ),
        (
            "n_unresolved",
            P("qr_w5", 0, 16),
            UBits,
            Lin,
            Closed(0.0),
            Closed(65535.0),
            X::None,
        ),
        (
            "suspect_fraction",
            P("qr_w5", 16, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "saturated_fraction",
            P("qr_w6", 0, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "valid_sample_count",
            P("qr_w6", 16, 16),
            UBits,
            Lin,
            Closed(0.0),
            Closed(65535.0),
            X::None,
        ),
        (
            "escape_time_min",
            P("qr_w7", 0, 16),
            F16,
            Lin,
            Closed(0.0),
            Unbounded,
            X::Saturate,
        ),
        (
            "escape_time_max",
            P("qr_w7", 16, 16),
            F16,
            Lin,
            Closed(0.0),
            Unbounded,
            X::Saturate,
        ),
        (
            "retrograde_fraction",
            P("qr_w8", 0, 16),
            F16,
            Lin,
            Closed(0.0),
            Closed(1.0),
            X::None,
        ),
        (
            "mean_orbit_count_spread",
            P("qr_w8", 16, 16),
            F16,
            Lin,
            Closed(0.0),
            Unbounded,
            X::Saturate,
        ),
        (
            "running_mean_divergence",
            S(41),
            F32,
            Lin,
            Unbounded,
            Unbounded,
            X::None,
        ),
        (
            "first_divergence_t",
            S(42),
            F32,
            Lin,
            Closed(0.0),
            Unbounded,
            X::SentinelPosInf,
        ),
        ("coherence", S(43), F32, Lin, Unbounded, Unbounded, X::None),
    ]
}

/// An entry as a row of the table, or why it can't be one.
fn as_row(e: &Entry) -> Row {
    let at = match e.location {
        Location::Packed {
            word,
            offset,
            width,
        } => At::Packed(word, offset, width),
        Location::Scalar(i) => At::Scalar(i),
        Location::Derived { .. } => panic!("`{}` is derived, not stored", e.name),
    };
    let ty = match &e.ty {
        FieldType::UBits => Ty::UBits,
        FieldType::F16 => Ty::F16,
        FieldType::F32 => Ty::F32,
        FieldType::Vector { component, k } if **component == FieldType::UBits => {
            Ty::VectorUBits(*k)
        }
        other => panic!("`{}` has type {other:?}, not one §3.7's table uses", e.name),
    };
    let extra = match (e.overflow, e.sentinel) {
        (None, None) => Extra::None,
        (Some(Overflow::Inf), None) => Extra::Inf,
        (Some(Overflow::Saturate), None) => Extra::Saturate,
        (None, Some(s)) if s.to_bits() == INF.to_bits() => Extra::SentinelPosInf,
        other => panic!(
            "`{}` has sentinel · overflow {other:?}, not one §3.7's table uses",
            e.name
        ),
    };
    (e.name, at, ty, e.scale, e.range.lo, e.range.hi, extra)
}

fn check_table(l: &Ledger) {
    let es = built(l);
    let got: Vec<Row> = es.iter().map(as_row).collect();
    let want = table();
    for w in &want {
        let g = got.iter().find(|g| g.0 == w.0);
        assert_eq!(g, Some(w), "the ledger's `{}` is not §3.7's row", w.0);
    }
    assert_eq!(
        got.len(),
        want.len(),
        "the ledger has members §3.7's table has not"
    );
    for e in &es {
        assert_eq!(
            e.provenance,
            Provenance::Reduction,
            "`{}` is not provenance: reduction",
            e.name
        );
        for c in [Consumer::Scheduler, Consumer::Debug] {
            assert!(
                e.consumers.contains(&c),
                "`{}` is not read by {c:?}",
                e.name
            );
        }
    }
    // The packed words: qr_w0 … qr_w8, 32 bits each, with only qr_w0's bit 7 reserved.
    let names: Vec<&str> = l.words.iter().map(|w| w.name).collect();
    assert_eq!(
        names,
        ["qr_w0", "qr_w1", "qr_w2", "qr_w3", "qr_w4", "qr_w5", "qr_w6", "qr_w7", "qr_w8"]
    );
    for w in &l.words {
        assert_eq!(w.bits, 32, "`{}` is not 32 bits", w.name);
        let want: Vec<Span> = if w.name == "qr_w0" {
            vec![Span {
                offset: 7,
                width: 1,
            }]
        } else {
            vec![]
        };
        assert_eq!(
            w.reserved, want,
            "`{}` reserves other bits than §3.7's",
            w.name
        );
    }
}

#[test]
fn qa_quad_reduction_layout_ledger_is_section_3_7_table() {
    check_table(&reduction::ledger());
}

negative_control!(
    qa_quad_reduction_layout_ledger_is_section_3_7_table,
    "a ledger with spread_winner at bits 6–7 is not §3.7's row",
    expected = "the ledger's `spread_winner` is not §3.7's row",
    {
        let mut l = reduction::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("spread_winner"))
            .unwrap();
        e.location = Some(Location::Packed {
            word: "qr_w0",
            offset: 6,
            width: 2,
        });
        check_table(&l);
    }
);

negative_control!(
    qa_quad_reduction_layout_sentinel_nan,
    "a first_divergence_t sentinel of NaN is not §3.7's +∞",
    expected = "`first_divergence_t` has sentinel · overflow",
    {
        let mut l = reduction::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("first_divergence_t"))
            .unwrap();
        *e = e.clone().sentinel(f64::NAN);
        check_table(&l);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-REF-006, REQ-REF-052: validity members and n_unresolved, by type.

const VALIDITY: [(&str, Ty, u32); 9] = [
    ("suspect_fraction", Ty::F16, 16),
    ("saturated_fraction", Ty::F16, 16),
    ("valid_sample_count", Ty::UBits, 16),
    ("coherence", Ty::F32, 32),
    ("escape_time_min", Ty::F16, 16),
    ("escape_time_max", Ty::F16, 16),
    ("retrograde_fraction", Ty::F16, 16),
    ("mean_orbit_count_spread", Ty::F16, 16),
    ("n_unresolved", Ty::UBits, 16),
];

fn check_validity(l: &Ledger, s: &ledger::schema::Struct) {
    let rows: Vec<Row> = built(l).iter().map(as_row).collect();
    for (name, ty, bits) in VALIDITY {
        let r = rows
            .iter()
            .find(|r| r.0 == name)
            .unwrap_or_else(|| panic!("no member `{name}`"));
        assert_eq!(r.2, ty, "`{name}` is not §3.7's type");
        match r.1 {
            At::Packed(_, _, w) => assert_eq!(w, bits, "`{name}` is {w} bits, not {bits}"),
            At::Scalar(_) => {
                assert_eq!(bits, 32, "`{name}` is a scalar slot, not {bits} bits");
                let m = s
                    .members
                    .iter()
                    .find(|m| m.name == name)
                    .unwrap_or_else(|| panic!("no struct member `{name}`"));
                assert_eq!(
                    m.storage,
                    ledger::schema::Storage::F32,
                    "`{name}` is not stored as f32"
                );
            }
        }
    }
}

#[test]
fn qa_quad_reduction_members_validity_and_n_unresolved_types() {
    check_validity(&reduction::ledger(), &reduction::quad_reduction());
}

negative_control!(
    qa_quad_reduction_members_validity_and_n_unresolved_types,
    "an n_unresolved 8 bits wide is not §3.7's u16",
    expected = "`n_unresolved` is 8 bits, not 16",
    {
        let mut l = reduction::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("n_unresolved"))
            .unwrap();
        e.location = Some(Location::Packed {
            word: "qr_w5",
            offset: 0,
            width: 8,
        });
        check_validity(&l, &reduction::quad_reduction());
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-093 / R-248: every f16 member's range lies within ±65504 or states its overflow.

fn check_f16_ranges(l: &Ledger) {
    for e in built(l).iter().filter(|e| e.ty == FieldType::F16) {
        for end in [e.range.lo, e.range.hi] {
            match end {
                Closed(v) | Bound::Open(v) => {
                    assert!(
                        v.abs() <= 65504.0,
                        "`{}`'s range end {v} lies beyond ±65504",
                        e.name
                    )
                }
                Unbounded => assert!(
                    e.overflow.is_some(),
                    "`{}` has an unbounded end and states no overflow",
                    e.name
                ),
            }
        }
    }
}

#[test]
fn qa_quad_reduction_f16_members_within_f16_or_overflow() {
    check_f16_ranges(&reduction::ledger());
}

negative_control!(
    qa_quad_reduction_f16_members_within_f16_or_overflow,
    "an error_ratio unbounded above with no overflow",
    expected = "`error_ratio` has an unbounded end and states no overflow",
    {
        let mut l = reduction::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("error_ratio"))
            .unwrap();
        e.overflow = None;
        check_f16_ranges(&l);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-075 (R-410): bin count and bin capacity.

/// `class ⊕ detail`'s width from the payload ledger: `state` must sit at bits 0–2 and `detail` at 3–4 (§3.1).
fn class_detail_width() -> u32 {
    let mut width = 0;
    for (name, want_offset, want_width) in [("state", 0, 3), ("detail", 3, 2)] {
        let e = ledger::payload::ledger()
            .entries
            .into_iter()
            .find(|e| e.name == Some(name))
            .unwrap_or_else(|| panic!("no payload entry `{name}`"));
        match e.location {
            Some(Location::Packed {
                offset, width: w, ..
            }) => {
                assert_eq!(
                    (offset, w),
                    (want_offset, want_width),
                    "`{name}` is not at §3.1's bits"
                );
                width += w;
            }
            other => panic!("`{name}` is at {other:?}"),
        }
    }
    width
}

/// The most copies a bin whose range ends at `hi` counts without wrapping is checked against each of `copies`.
fn check_capacity(hi: Bound, copies: &[u64]) {
    let Closed(hi) = hi else {
        panic!("the bin's range has no closed greatest value")
    };
    for &n in copies {
        assert!(
            n as f64 <= hi,
            "a bin of greatest value {hi} cannot count {n} copies"
        );
    }
}

/// N² × (E+1) with N the samples per quad side and E+1 the copies per footprint.
fn copies(n: u64, e: u64) -> u64 {
    n * n * (e + 1)
}

#[test]
fn qa_quad_reduction_histogram_capacity_bins_and_bound() {
    let width = class_detail_width();
    assert_eq!(width, 5, "class ⊕ detail is 5 bits (§3.1)");
    let rows: Vec<Row> = built(&reduction::ledger()).iter().map(as_row).collect();
    let hist = rows
        .iter()
        .find(|r| r.0 == "class_histogram")
        .expect("class_histogram");
    assert_eq!(
        hist.2,
        Ty::VectorUBits(1 << width),
        "one bin per class ⊕ detail code"
    );
    assert_eq!(reduction::bins(), 32);
    // dominant_outcome holds a bin index: its range is every code.
    let dom = rows
        .iter()
        .find(|r| r.0 == "dominant_outcome")
        .expect("dominant_outcome");
    assert_eq!((dom.4, dom.5), (Closed(0.0), Closed(31.0)));
    // Each bin is a u32: the range ends at 2³² − 1, and the generated WGSL stores it as array<u32, 32>.
    assert_eq!(
        hist.5,
        Closed(4_294_967_295.0),
        "a bin's range is not a u32's"
    );
    assert_eq!(reduction::bin_capacity(), (1 << 32) - 1);
    check_capacity(hist.5, &[copies(16, 15), copies(16, 1), (1 << 32) - 1]);
    assert_eq!(copies(16, 15), 4096);
    let module = parse(&quad_wgsl());
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("QuadReduction"))
        .unwrap();
    let TypeInner::Struct { members, .. } = &ty.inner else {
        panic!()
    };
    let m = members
        .iter()
        .find(|m| m.name.as_deref() == Some("class_histogram"))
        .unwrap();
    match &module.types[m.ty].inner {
        TypeInner::Array {
            base,
            size: ArraySize::Constant(n),
            ..
        } => {
            assert_eq!(n.get(), 32);
            assert_eq!(
                module.types[*base].inner,
                TypeInner::Scalar(Scalar {
                    kind: ScalarKind::Uint,
                    width: 4
                }),
                "a WGSL bin is not a u32"
            );
        }
        other => panic!("class_histogram is {other:?}"),
    }
    // §3.7 states the bound and that dispatch asserts it.
    let doc = read("docs/design/principia_dd_generation_root.md");
    assert!(
        doc.contains("N² × (E+1) ≤ 2³² − 1"),
        "§3.7 does not state the bound"
    );
}

negative_control!(
    qa_quad_reduction_histogram_capacity_bins_and_bound,
    "§3.7's former u8 bin cannot count Medium's 16² × 2 = 512 copies",
    expected = "a bin of greatest value 255 cannot count 512 copies",
    check_capacity(Closed(255.0), &[copies(16, 1)])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-089, REQ-PAY-077, REQ-PAY-006, REQ-REF-007, REQ-REF-008: the generated WGSL struct.

/// §3.7's struct: member, byte offset, WGSL type.
const WGSL_LAYOUT: [(&str, u32, &str); 13] = [
    ("class_histogram", 0, "array<u32, 32>"),
    ("qr_w0", 128, "u32"),
    ("qr_w1", 132, "u32"),
    ("qr_w2", 136, "u32"),
    ("qr_w3", 140, "u32"),
    ("qr_w4", 144, "u32"),
    ("qr_w5", 148, "u32"),
    ("qr_w6", 152, "u32"),
    ("qr_w7", 156, "u32"),
    ("qr_w8", 160, "u32"),
    ("running_mean_divergence", 164, "f32"),
    ("first_divergence_t", 168, "f32"),
    ("coherence", 172, "f32"),
];

const NOT_MEMBERS: [&str; 11] = [
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
    "divergence_trend",
];

fn check_wgsl_struct(source: &str) {
    let module = parse(source);
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("QuadReduction"))
        .expect("the WGSL declares QuadReduction");
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("not a struct")
    };
    let mut got = Vec::new();
    for m in members {
        let name = m.name.clone().unwrap_or_default();
        assert!(
            !NOT_MEMBERS.contains(&name.as_str()),
            "`{name}` is a WGSL QuadReduction member"
        );
        let t = match &module.types[m.ty].inner {
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
                size: ArraySize::Constant(n),
                ..
            } if module.types[*base].inner
                == TypeInner::Scalar(Scalar {
                    kind: ScalarKind::Uint,
                    width: 4,
                }) =>
            {
                format!("array<u32, {n}>")
            }
            other => {
                panic!("`{name}` is {other:?}: not a fixed-size scalar or the fixed histogram")
            }
        };
        got.push((name, m.offset, t));
    }
    let want: Vec<(String, u32, String)> = WGSL_LAYOUT
        .iter()
        .map(|&(n, o, t)| (n.to_owned(), o, t.to_owned()))
        .collect();
    assert_eq!(got, want, "the WGSL QuadReduction is not §3.7's layout");
    assert_eq!(*span, 176, "the WGSL QuadReduction is {span} B, not 176");
    // The ledger names no excluded member either.
    for e in &reduction::ledger().entries {
        let n = e.name.unwrap_or_default();
        assert!(
            !NOT_MEMBERS.contains(&n),
            "`{n}` is a ledger QuadReduction member"
        );
    }
}

#[test]
fn qa_quad_reduction_size_wgsl_is_176_with_section_3_7_offsets() {
    check_wgsl_struct(&quad_wgsl());
    // §3.7 and the docs citing it give the measured size (REQ-PAY-089).
    for path in [
        "docs/design/principia_dd_generation_root.md",
        "docs/design/principia_systems_architecture.md",
        "docs/contracts/principia_render_contract.md",
        "docs/contracts/principia_canonical_spec.md",
        "plan/reviewers/perf.md",
    ] {
        let doc = read(path);
        assert!(
            doc.contains("176 B"),
            "{path} does not state QuadReduction's 176 B"
        );
    }
}

negative_control!(
    qa_quad_reduction_size_wgsl_is_176_with_section_3_7_offsets,
    "a WGSL struct with a spread_t_end member",
    expected = "`spread_t_end` is a WGSL QuadReduction member",
    check_wgsl_struct(&quad_wgsl().replace(
        "    coherence: f32,",
        "    coherence: f32,\n    spread_t_end: f32,"
    ))
);

negative_control!(
    qa_quad_reduction_size_wgsl_runtime_array,
    "a WGSL histogram that grows with the sample count (a runtime-sized array) is not a fixed-size member",
    expected = "not a fixed-size scalar or the fixed histogram",
    check_wgsl_struct(
        &quad_wgsl()
            .replace("    class_histogram: array<u32, 32>,\n", "")
            .replace("    coherence: f32,", "    coherence: f32,\n    class_histogram: array<u32>,")
    )
);

negative_control!(
    qa_quad_reduction_size_wgsl_u8_sized,
    "a WGSL histogram of 8 u32s (u8 bins packed four to a word) is the old 80 B struct",
    expected = "the WGSL QuadReduction is not §3.7's layout",
    check_wgsl_struct(&quad_wgsl().replace("array<u32, 32>", "array<u32, 8>"))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-030: §3.8's f16 type in the static layout check, on a ledger of the test's own.

fn ent(name: &'static str, location: Location, ty: FieldType, range: Range) -> EntryBuilder {
    EntryBuilder::new(name)
        .location(location)
        .ty(ty)
        .scale(Scale::Lin)
        .range(range)
        .provenance(Provenance::Kernel)
        .consumers(&[Consumer::Debug])
}

fn at(offset: u32, width: u32) -> Location {
    Location::Packed {
        word: "qa_word",
        offset,
        width,
    }
}

/// One 32-bit word: the f16 field `qa_half` at `half_at`, with `rest` covering the other bits.
fn half_ledger(
    half_at: Location,
    half_ty: FieldType,
    range: Range,
    overflow: Option<Overflow>,
    rest: Vec<EntryBuilder>,
    reserved: Vec<Span>,
) -> Ledger {
    let mut half = ent("qa_half", half_at, half_ty, range);
    if let Some(o) = overflow {
        half = half.overflow(o);
    }
    let mut entries = vec![half];
    entries.extend(rest);
    Ledger {
        words: vec![Word {
            name: "qa_word",
            bits: 32,
            reserved,
        }],
        entries,
    }
}

fn low_half(range: Range, overflow: Option<Overflow>) -> Ledger {
    half_ledger(
        at(0, 16),
        FieldType::F16,
        range,
        overflow,
        vec![ent(
            "qa_count",
            at(16, 16),
            FieldType::UBits,
            Range::int(0, 65535),
        )],
        vec![],
    )
}

fn unit() -> Range {
    Range {
        lo: Closed(-1.0),
        hi: Closed(1.0),
    }
}

fn refused(l: &Ledger, why: &str) {
    match gen::generate(l, gen::EMITTERS) {
        Ok(_) => panic!("{why}: generation passed"),
        Err(e) => {
            let m = e.to_string();
            assert!(
                m.contains("qa_half"),
                "{why}: refused without naming `qa_half`: {m}"
            );
        }
    }
}

fn passes(l: &Ledger, why: &str) {
    if let Err(e) = gen::generate(l, gen::EMITTERS) {
        panic!("{why}: refused: {e}");
    }
}

/// The refused cases, each by name.
fn bad_cases() -> Vec<(&'static str, Ledger)> {
    let rest15 = vec![ent(
        "qa_count",
        at(16, 16),
        FieldType::UBits,
        Range::int(0, 65535),
    )];
    let rest17 = vec![ent(
        "qa_count",
        at(17, 15),
        FieldType::UBits,
        Range::int(0, 32767),
    )];
    let whole = vec![ent(
        "qa_count",
        at(0, 32),
        FieldType::UBits,
        Range::int(0, 4_294_967_295),
    )];
    vec![
        (
            "15 bits wide",
            half_ledger(
                at(0, 15),
                FieldType::F16,
                unit(),
                None,
                rest15,
                vec![Span {
                    offset: 15,
                    width: 1,
                }],
            ),
        ),
        (
            "17 bits wide",
            half_ledger(at(0, 17), FieldType::F16, unit(), None, rest17, vec![]),
        ),
        (
            "at a scalar index",
            half_ledger(
                Location::Scalar(0),
                FieldType::F16,
                unit(),
                None,
                whole.clone(),
                vec![],
            ),
        ),
        (
            "a vector component",
            half_ledger(
                Location::Scalar(0),
                FieldType::Vector {
                    component: Box::new(FieldType::F16),
                    k: 2,
                },
                unit(),
                None,
                whole,
                vec![],
            ),
        ),
        (
            "range above 65504",
            low_half(
                Range {
                    lo: Closed(0.0),
                    hi: Closed(65505.0),
                },
                None,
            ),
        ),
        (
            "range below −65504",
            low_half(
                Range {
                    lo: Closed(-70000.0),
                    hi: Closed(0.0),
                },
                None,
            ),
        ),
        (
            "range above 65504 even with overflow",
            low_half(
                Range {
                    lo: Closed(0.0),
                    hi: Closed(1.0e6),
                },
                Some(Overflow::Saturate),
            ),
        ),
        (
            "unbounded above, no overflow",
            low_half(
                Range {
                    lo: Closed(0.0),
                    hi: Unbounded,
                },
                None,
            ),
        ),
        (
            "unbounded below, no overflow",
            low_half(
                Range {
                    lo: Unbounded,
                    hi: Closed(0.0),
                },
                None,
            ),
        ),
    ]
}

fn good_cases() -> Vec<(&'static str, Ledger)> {
    vec![
        ("16 bits in [−1, 1]", low_half(unit(), None)),
        (
            "16 bits at ±65504 exactly",
            low_half(
                Range {
                    lo: Closed(-65504.0),
                    hi: Closed(65504.0),
                },
                None,
            ),
        ),
        (
            "16 bits in the high half",
            half_ledger(
                at(16, 16),
                FieldType::F16,
                unit(),
                None,
                vec![ent(
                    "qa_count",
                    at(0, 16),
                    FieldType::UBits,
                    Range::int(0, 65535),
                )],
                vec![],
            ),
        ),
        (
            "unbounded above with saturate",
            low_half(
                Range {
                    lo: Closed(0.0),
                    hi: Unbounded,
                },
                Some(Overflow::Saturate),
            ),
        ),
        (
            "unbounded both ends with inf",
            low_half(
                Range {
                    lo: Unbounded,
                    hi: Unbounded,
                },
                Some(Overflow::Inf),
            ),
        ),
    ]
}

#[test]
fn layout_static_f16_qa_refuses_and_passes_by_r248_r312() {
    for (why, l) in bad_cases() {
        refused(&l, why);
    }
    for (why, l) in good_cases() {
        passes(&l, why);
    }
}

negative_control!(
    layout_static_f16_qa_refuses_and_passes_by_r248_r312,
    "a 16-bit in-range f16 field is not refused, so `refused` fails on it",
    expected = "within range: generation passed",
    refused(&low_half(unit(), None), "within range")
);

negative_control!(
    layout_static_f16_qa_unbounded_with_overflow_passes,
    "the same unbounded field without its overflow is refused, so `passes` fails on it",
    expected = "no overflow: refused",
    passes(
        &low_half(
            Range {
                lo: Closed(0.0),
                hi: Unbounded
            },
            None
        ),
        "no overflow"
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-030 / R-317: no generated WGSL declares `enable f16` or an f16 value.

/// `source` without its `//` comments.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_no_f16(path: &str, source: &str) {
    let c = code(source);
    assert!(!c.contains("enable f16"), "{path} declares `enable f16`");
    let token = c
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|t| t == "f16" || t == "vec2h" || t == "vec3h" || t == "vec4h");
    assert!(!token, "{path} names an f16 type");
    let module = parse(source);
    for (_, t) in module.types.iter() {
        let half = |s: &Scalar| s.kind == ScalarKind::Float && s.width == 2;
        let bad = match &t.inner {
            TypeInner::Scalar(s) => half(s),
            TypeInner::Vector { scalar, .. } | TypeInner::Matrix { scalar, .. } => half(scalar),
            _ => false,
        };
        assert!(!bad, "{path} holds an f16 type {:?}", t.inner);
    }
}

#[test]
fn layout_static_f16_qa_generated_wgsl_has_no_f16() {
    for path in [
        "crates/render/frag/generated/quad_reduction.wgsl",
        "crates/render/frag/generated/payload_unpack.wgsl",
    ] {
        check_no_f16(path, &read(path));
    }
}

negative_control!(
    layout_static_f16_qa_generated_wgsl_has_no_f16,
    "an accessor returning an f16 value names the f16 type",
    expected = "names an f16 type",
    check_no_f16(
        "quad_reduction.wgsl",
        &quad_wgsl().replace(
            "fn qr_error_ratio(w: u32) -> f32 { return unpack2x16float(extractBits(w, 16u, 16u)).x; }",
            "fn qr_error_ratio(w: u32) -> f32 { let h: f16 = f16(0.0); return unpack2x16float(extractBits(w, 16u, 16u)).x; }"
        )
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-030 / REQ-PAY-077 on the GPU: the WGSL accessors read binary16 values whose bits are known, widened to f32,
// and unsigned members at §3.7's bits.

/// §3.7's packed members: name, offset, width, f16.
const PACKED: [(&str, u32, u32, bool); 20] = [
    ("dominant_outcome", 0, 5, false),
    ("spread_winner", 5, 2, false),
    ("level", 8, 8, false),
    ("outcome_impurity", 16, 16, true),
    ("terminated_fraction", 0, 16, true),
    ("spread_shape", 16, 16, true),
    ("spread_event", 0, 16, true),
    ("error_ratio", 16, 16, true),
    ("roundtrip_error", 0, 16, true),
    ("alpha_area", 16, 16, true),
    ("alpha_energy", 0, 16, true),
    ("worst_energy_drift", 16, 16, true),
    ("n_unresolved", 0, 16, false),
    ("suspect_fraction", 16, 16, true),
    ("saturated_fraction", 0, 16, true),
    ("valid_sample_count", 16, 16, false),
    ("escape_time_min", 0, 16, true),
    ("escape_time_max", 16, 16, true),
    ("retrograde_fraction", 0, 16, true),
    ("mean_orbit_count_spread", 16, 16, true),
];

/// Normal binary16 values and infinities with their f32 bits (IEEE 754); subnormals and NaN are left out, as a GPU
/// may flush or canonicalise them.
const HALVES: [(u16, u32); 9] = [
    (0x0000, 0x0000_0000), // 0
    (0x3c00, 0x3f80_0000), // 1
    (0x3800, 0x3f00_0000), // 0.5
    (0xc000, 0xc000_0000), // −2
    (0x7bff, 0x477f_e000), // 65504
    (0xfbff, 0xc77f_e000), // −65504
    (0x7c00, 0x7f80_0000), // +∞
    (0x0400, 0x3880_0000), // 2⁻¹⁴
    (0x5a66, 0x434c_c000), // 204.75, §3.7's measured error_ratio 204.8 to binary16
];

fn kernel() -> String {
    let mut cases = String::new();
    for (j, &(name, _, _, half)) in PACKED.iter().enumerate() {
        let r = if half {
            format!("bitcast<u32>(qr_{name}(w))")
        } else {
            format!("qr_{name}(w)")
        };
        cases.push_str(&format!("        case {j}u: {{ out[i] = {r}; }}\n"));
    }
    format!(
        r"
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;

@compute @workgroup_size(64)
fn qa_read(@builtin(global_invocation_id) id: vec3<u32>) {{
    let i = id.x;
    if i >= arrayLength(&a) {{ return; }}
    let w = a[i];
    switch i % {n}u {{
{cases}        default: {{ out[i] = 0xffffffffu; }}
    }}
}}
",
        n = PACKED.len()
    )
}

fn check_gpu(wgsl: &str) {
    let mut words = Vec::new();
    let mut want = Vec::new();
    for (k, &(h, f)) in HALVES.iter().enumerate() {
        let background = [0u32, u32::MAX, 0xa5a5_a5a5][k % 3];
        for &(_, offset, width, half) in &PACKED {
            let max = u32::MAX >> (32 - width);
            let v = if half {
                u32::from(h)
            } else {
                (0x9e37_79b9u32.rotate_left(k as u32)) & max
            };
            let mask = max << offset;
            words.push((background & !mask) | ((v << offset) & mask));
            want.push(if half { f } else { v });
        }
    }
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = gpu.run_wgsl(&format!("{wgsl}\n{}", kernel()), "qa_read", &[&words]);
    for (i, (&g, &w)) in got.iter().zip(&want).enumerate() {
        let name = PACKED[i % PACKED.len()].0;
        assert_eq!(
            g, w,
            "WGSL qr_{name}({:#010x}) reads {g:#010x}, not {w:#010x}",
            words[i]
        );
    }
}

#[test]
fn layout_static_f16_qa_wgsl_accessors_read_known_binary16() {
    check_gpu(&quad_wgsl());
}

negative_control!(
    layout_static_f16_qa_wgsl_accessors_read_known_binary16,
    "a WGSL alpha_area reading the low half of qr_w3 reads roundtrip_error's bits",
    expected = "WGSL qr_alpha_area(",
    check_gpu(&quad_wgsl().replace(
        "fn qr_alpha_area(w: u32) -> f32 { return unpack2x16float(extractBits(w, 16u, 16u)).x; }",
        "fn qr_alpha_area(w: u32) -> f32 { return unpack2x16float(extractBits(w, 0u, 16u)).x; }"
    ))
);
