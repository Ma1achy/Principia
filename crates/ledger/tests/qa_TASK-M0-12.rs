//! QA tests for TASK-M0-12, written from REQ-GEN-008 (R-36, R-63, R-22; dd_simstate_payload §3;
//! dd_generation_root §4 seam 13, §5 test 7, §6) and REQ-SYS-063 as R-251 scopes the hash (dd_generation_root §3.8,
//! "The hash"), not from the implementation:
//! - any one ledger change changes the version: every field of every entry, every word, every struct member, every
//!   `QuadReduction` member, and a reordering of the frozen continuation table's digits; the single-edit ledgers all
//!   give distinct versions (no two edits collide);
//! - the hash covers payload §3's frozen table as the corpus writes it;
//! - each of §3.8's five stored-bits register entries is hashed by its value, its type and its class, never its
//!   citation; a register entry outside that list, even one whose name extends a hashed name, is never hashed;
//! - the emitted constant is the computed hash, also for an edited ledger, and the checked-in file carries it; no
//!   hand-written schema version exists in any crate's source.
//!
//! Each test has a registered negative control (R-176).

use std::collections::HashSet;
use std::path::Path;

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
use ledger::gen;
use ledger::layout;
use ledger::payload::{self, ReductionMember};
use ledger::schema::{
    Bound, Consumer, Entry, FieldType, Location, Overflow, Provenance, Scale, Span, Storage,
    Struct, Word,
};
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// The payload ledger, owned, so one part can be edited.
#[derive(Clone)]
struct Owned {
    words: Vec<Word>,
    entries: Vec<Entry>,
    structs: Vec<Struct>,
    quad: Vec<ReductionMember>,
    register: Vec<ConstantBuilder>,
    cont_symbol: [[u32; 4]; 3],
}

impl Owned {
    fn new() -> Self {
        let l = layout();
        Owned {
            words: l.words.clone(),
            entries: gen::validate(&l).expect("the payload ledger validates"),
            structs: payload::structs(),
            quad: payload::QUAD_REDUCTION.to_vec(),
            register: REGISTER.to_vec(),
            cont_symbol: Hashed::payload(&[], &[], &[]).cont_symbol,
        }
    }

    fn version(&self) -> u64 {
        let h = Hashed {
            quad_reduction: &self.quad,
            register: &self.register,
            cont_symbol: self.cont_symbol,
            ..Hashed::payload(&self.words, &self.entries, &self.structs)
        };
        schema_version(&h).expect("the schema version is computed")
    }
}

fn base() -> u64 {
    Owned::new().version()
}

/// Each edited ledger's version differs from the base's and from every other edit's.
fn check_all_distinct(label: &str, edits: Vec<(String, Owned)>) {
    assert!(!edits.is_empty(), "{label}: no edits");
    let b = base();
    let mut seen: HashSet<u64> = HashSet::from([b]);
    for (what, o) in &edits {
        let v = o.version();
        assert_ne!(v, b, "{label}: the schema version did not change: {what}");
        assert!(
            seen.insert(v),
            "{label}: two edits gave one version: {what}"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-008: change one ledger row → the version changes. Every field of every entry, one at a time.

fn other_bound(b: Bound) -> Bound {
    match b {
        Bound::Closed(v) => Bound::Open(v),
        Bound::Open(v) => Bound::Closed(v),
        Bound::Unbounded => Bound::Closed(0.0),
    }
}

/// One edit of each §3.8 field of `e`.
fn entry_edits(e: &Entry) -> Vec<(&'static str, Entry)> {
    let mut out = Vec::new();
    let mut push = |what, f: &dyn Fn(&mut Entry)| {
        let mut x = e.clone();
        f(&mut x);
        out.push((what, x));
    };
    push("name", &|x| x.name = leak(format!("{}_qa", x.name)));
    match &e.location {
        Location::Packed { .. } => {
            push("offset", &|x| {
                if let Location::Packed { offset, .. } = &mut x.location {
                    *offset ^= 1
                }
            });
            push("width", &|x| {
                if let Location::Packed { width, .. } = &mut x.location {
                    *width += 1
                }
            });
            push("word", &|x| {
                if let Location::Packed { word, .. } = &mut x.location {
                    *word = leak(format!("{word}_qa"))
                }
            });
        }
        Location::Scalar(_) => push("scalar index", &|x| {
            if let Location::Scalar(i) = &mut x.location {
                *i += 1
            }
        }),
        Location::Derived { .. } => push("derived from", &|x| {
            if let Location::Derived { from } = &mut x.location {
                from.push("qa_extra")
            }
        }),
    }
    push("type", &|x| {
        x.ty = if x.ty == FieldType::F32 {
            FieldType::UBits
        } else {
            FieldType::F32
        }
    });
    push("scale", &|x| {
        x.scale = if x.scale == Scale::Lin {
            Scale::Log
        } else {
            Scale::Lin
        }
    });
    push("range lo", &|x| x.range.lo = other_bound(x.range.lo));
    push("range hi", &|x| x.range.hi = other_bound(x.range.hi));
    push("sentinel", &|x| {
        x.sentinel = match x.sentinel {
            None => Some(0.0),
            // `None`: a sentinel may be ±∞ (`d_min`'s), where `v + 1` is no edit.
            Some(_) => None,
        }
    });
    push("tier_gate", &|x| {
        x.tier_gate = match x.tier_gate {
            None => Some("qa_gate"),
            Some(_) => None,
        }
    });
    push("overflow", &|x| {
        x.overflow = match x.overflow {
            None => Some(Overflow::Saturate),
            Some(Overflow::Saturate) => Some(Overflow::Inf),
            Some(Overflow::Inf) => None,
        }
    });
    push("floor", &|x| {
        x.floor = match x.floor {
            None => Some("qa_floor"),
            Some(_) => None,
        }
    });
    push("provenance", &|x| {
        x.provenance = if x.provenance == Provenance::Kernel {
            Provenance::Cpu
        } else {
            Provenance::Kernel
        }
    });
    push("consumers", &|x| {
        if x.consumers.contains(&Consumer::Scheduler) {
            x.consumers.retain(|c| *c != Consumer::Scheduler)
        } else {
            x.consumers.push(Consumer::Scheduler)
        }
    });
    out
}

fn every_entry_edit(base: &Owned) -> Vec<(String, Owned)> {
    let mut out = Vec::new();
    for (i, e) in base.entries.iter().enumerate() {
        for (what, edited) in entry_edits(e) {
            let mut o = base.clone();
            o.entries[i] = edited;
            out.push((format!("entry `{}` {what}", e.name), o));
        }
    }
    out
}

#[test]
fn qa_m012_every_field_of_every_entry_changes_the_version() {
    let o = Owned::new();
    assert!(o.entries.len() >= 10, "the payload ledger has its entries");
    check_all_distinct("entries", every_entry_edit(&o));
}

negative_control!(
    qa_m012_every_field_of_every_entry_changes_the_version,
    "an edit that writes back the same entry leaves the base version, so the change check must fail",
    expected = "the schema version did not change",
    {
        let o = Owned::new();
        check_all_distinct("entries", vec![("a no-op edit".into(), o)]);
    }
);

negative_control!(
    qa_m012_every_field_of_every_entry_changes_the_version_collision,
    "the same edit listed twice gives one version twice, so the distinctness check must fail",
    expected = "two edits gave one version",
    {
        let mut edits = every_entry_edit(&Owned::new());
        edits.truncate(1);
        let again = edits[0].clone();
        edits.push(again);
        check_all_distinct("entries", edits);
    }
);

/// R-22: the body-index names are a schema event. Swapping the names `m0` and `m1` (each at the other's slot), and
/// renaming `m0` alone, each change the version.
#[test]
fn qa_m012_body_index_renaming_changes_the_version() {
    let base = Owned::new();
    let find = |n: &str| {
        base.entries
            .iter()
            .position(|e| e.name == n)
            .unwrap_or_else(|| panic!("the ledger has `{n}` (dd_generation_root §3.6)"))
    };
    let (i0, i1) = (find("m0"), find("m1"));
    let mut swapped = base.clone();
    swapped.entries[i0].name = "m1";
    swapped.entries[i1].name = "m0";
    let mut renamed = base.clone();
    renamed.entries[i0].name = "m_0";
    check_all_distinct(
        "R-22",
        vec![
            ("m0 and m1 swapped".into(), swapped),
            ("m0 renamed".into(), renamed),
        ],
    );
}

negative_control!(
    qa_m012_body_index_renaming_changes_the_version,
    "renaming m0 to itself is no edit, so the change check must fail",
    expected = "the schema version did not change",
    {
        let mut o = Owned::new();
        for e in &mut o.entries {
            if e.name == "m0" {
                e.name = "m0";
            }
        }
        check_all_distinct("R-22", vec![("m0 to m0".into(), o)]);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-008: the words, the struct layout and the QuadReduction member list are ledger too.

fn every_layout_edit(base: &Owned) -> Vec<(String, Owned)> {
    let mut out = Vec::new();
    for (i, w) in base.words.iter().enumerate() {
        let mut o = base.clone();
        o.words[i].bits += 1;
        out.push((format!("word `{}` bits", w.name), o));
        let mut o = base.clone();
        o.words[i].name = leak(format!("{}_qa", w.name));
        out.push((format!("word `{}` name", w.name), o));
        let mut o = base.clone();
        if o.words[i].reserved.is_empty() {
            o.words[i].reserved.push(Span {
                offset: 0,
                width: 1,
            });
        } else {
            o.words[i].reserved.pop();
        }
        out.push((format!("word `{}` reserved", w.name), o));
    }
    for (i, s) in base.structs.iter().enumerate() {
        let mut o = base.clone();
        o.structs[i].align *= 2;
        out.push((format!("struct `{}` align", s.name), o));
        let mut o = base.clone();
        o.structs[i].indexed = !s.indexed;
        out.push((format!("struct `{}` indexed", s.name), o));
        let mut o = base.clone();
        o.structs[i].buffer = match s.buffer {
            None => Some("qa_buffer"),
            Some(_) => None,
        };
        out.push((format!("struct `{}` buffer", s.name), o));
        for (j, m) in s.members.iter().enumerate() {
            let mut o = base.clone();
            o.structs[i].members[j].storage = match m.storage {
                Storage::F32 => Storage::U32,
                Storage::Pad(n) => Storage::Pad(n + 1),
                _ => Storage::F32,
            };
            out.push((
                format!("struct `{}` member `{}` storage", s.name, m.name),
                o,
            ));
            let mut o = base.clone();
            o.structs[i].members[j].name = leak(format!("{}_qa", m.name));
            out.push((format!("struct `{}` member `{}` name", s.name, m.name), o));
        }
        // Two adjacent members exchanged: the repr(C) layout changes.
        if s.members.len() >= 2 && s.members[0] != s.members[1] {
            let mut o = base.clone();
            o.structs[i].members.swap(0, 1);
            out.push((format!("struct `{}` members 0 and 1 swapped", s.name), o));
        }
    }
    for (i, m) in base.quad.iter().enumerate() {
        let mut o = base.clone();
        o.quad[i].name = leak(format!("{}_qa", m.name));
        out.push((format!("QuadReduction `{}` name", m.name), o));
        let mut o = base.clone();
        o.quad[i].ty = match m.ty {
            Some("f32") => Some("f16"),
            Some(_) => Some("f32"),
            None => Some("u8"),
        };
        out.push((format!("QuadReduction `{}` type", m.name), o));
    }
    let mut o = base.clone();
    o.quad.pop();
    out.push(("QuadReduction last member removed".into(), o));
    out
}

#[test]
fn qa_m012_every_word_struct_member_and_reduction_member_changes_the_version() {
    let o = Owned::new();
    assert!(
        !o.words.is_empty() && !o.structs.is_empty() && !o.quad.is_empty(),
        "the ledger has words, structs and QuadReduction members"
    );
    check_all_distinct("layout", every_layout_edit(&o));
}

negative_control!(
    qa_m012_every_word_struct_member_and_reduction_member_changes_the_version,
    "a word edited and edited back is the base ledger, so the change check must fail",
    expected = "the schema version did not change",
    {
        let mut o = Owned::new();
        o.words[0].bits += 1;
        o.words[0].bits -= 1;
        check_all_distinct("layout", vec![("bits +1 −1".into(), o)]);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-008, R-63: the continuation table — as payload §3 writes it — is hashed.

/// Payload §3's frozen table, transcribed from the corpus.
const INVERSE: [u32; 4] = [1, 0, 3, 2];
const CONT_SYMBOL: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
const CONTINUATION_INDEX: [[u32; 4]; 4] = [[0, 3, 1, 2], [3, 0, 2, 1], [1, 2, 0, 3], [2, 1, 3, 0]];

/// The table the version hashes is payload §3's: `inverse`, `cont_symbol`, `predecessor_symbol == cont_symbol`
/// (each digit map is an involution) and `continuation_index` with 3 at the inverse (R-307).
fn check_hashed_table(h: &Hashed) {
    assert_eq!(h.inverse, INVERSE, "the hashed table is not payload §3's");
    assert_eq!(
        h.cont_symbol, CONT_SYMBOL,
        "the hashed table is not payload §3's"
    );
    assert_eq!(
        h.predecessor_symbol, CONT_SYMBOL,
        "the hashed table is not payload §3's"
    );
    assert_eq!(
        h.continuation_index, CONTINUATION_INDEX,
        "the hashed table is not payload §3's"
    );
}

#[test]
fn qa_m012_the_hashed_continuation_table_is_payload_section_3s() {
    check_hashed_table(&Hashed::payload(&[], &[], &[]));
}

negative_control!(
    qa_m012_the_hashed_continuation_table_is_payload_section_3s,
    "a table with digits 1 and 2 exchanged is not payload §3's, so the table check must fail",
    expected = "the hashed table is not payload §3's",
    {
        let mut h = Hashed::payload(&[], &[], &[]);
        h.cont_symbol.swap(1, 2);
        check_hashed_table(&h);
    }
);

/// Exchanging digits 1 and 2 of `cont_symbol` gives another valid table (each digit still a permutation avoiding the
/// inverse) but changes what every stored word means: the version changes.
#[test]
fn qa_m012_reordering_the_continuation_digits_changes_the_version() {
    let mut o = Owned::new();
    o.cont_symbol.swap(1, 2);
    let mut p = Owned::new();
    p.cont_symbol.swap(0, 1);
    check_all_distinct(
        "continuation",
        vec![
            ("digits 1, 2 swapped".into(), o),
            ("digits 0, 1 swapped".into(), p),
        ],
    );
}

negative_control!(
    qa_m012_reordering_the_continuation_digits_changes_the_version,
    "a digit swapped with itself is no edit, so the change check must fail",
    expected = "the schema version did not change",
    {
        let mut o = Owned::new();
        o.cont_symbol.swap(1, 1);
        check_all_distinct("continuation", vec![("digit 1 with itself".into(), o)]);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-SYS-063 as R-251 scopes the hash: §3.8's five stored-bits entries, by value, type and class, not citation.

/// dd_generation_root §3.8, "The hash": "the word's capacity and length sentinel (§3.3), the `horizon_steps` limit
/// (§3.1), the f16 pack clamp (payload §1) and the f16 subnormal floor (R-271), which today are all five entries".
const SECTION_3_8_HASHED: [&str; 5] = [
    "horizon_steps_max",
    "fgw_capacity",
    "fgw_length_sentinel",
    "f16_finite_max",
    "f16_min_subnormal",
];

fn with_register_edit(name: &str, edit: impl FnOnce(&mut ConstantBuilder)) -> Owned {
    let mut o = Owned::new();
    let k = o
        .register
        .iter_mut()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("the register holds `{name}`"));
    edit(k);
    o
}

fn number(k: &ConstantBuilder) -> f64 {
    match k.value {
        Some(Value::Exact(v)) | Some(Value::Threshold(v)) => v,
        _ => panic!("`{}` has a settled number", k.name),
    }
}

/// For `name`: a value edit, a type edit (exact ↔ threshold at the same number, and to calibration) and each other
/// class.
fn hashed_edits(name: &'static str) -> Vec<(String, Owned)> {
    let k = *REGISTER
        .iter()
        .find(|k| k.name == name)
        .expect("in the register");
    let v = number(&k);
    let mut out = vec![
        (
            format!("`{name}` value ×2"),
            with_register_edit(name, |k| {
                k.value = Some(match k.value {
                    Some(Value::Threshold(_)) => Value::Threshold(v * 2.0),
                    _ => Value::Exact(v * 2.0),
                })
            }),
        ),
        (
            format!("`{name}` type flipped"),
            with_register_edit(name, |k| {
                k.value = Some(match k.value {
                    Some(Value::Threshold(_)) => Value::Exact(v),
                    _ => Value::Threshold(v),
                })
            }),
        ),
        (
            format!("`{name}` type calibration"),
            with_register_edit(name, |k| k.value = Some(Value::Calibration)),
        ),
    ];
    for class in [
        Admissibility::AchievableMaximum,
        Admissibility::ConservationLaw,
        Admissibility::CanonicalUnits,
    ] {
        if k.class != Some(class) {
            out.push((
                format!("`{name}` class {class:?}"),
                with_register_edit(name, |k| k.class = Some(class)),
            ));
        }
    }
    out
}

#[test]
fn qa_m012_each_hashed_register_entry_value_type_class_changes_the_version() {
    let mut edits = Vec::new();
    for name in SECTION_3_8_HASHED {
        edits.extend(hashed_edits(name));
    }
    check_all_distinct("register", edits);
}

negative_control!(
    qa_m012_each_hashed_register_entry_value_type_class_changes_the_version,
    "a class set to the one it already has is no edit, so the change check must fail",
    expected = "the schema version did not change",
    {
        let o = with_register_edit("f16_min_subnormal", |k| {
            k.class = Some(Admissibility::AchievableMaximum)
        });
        check_all_distinct("register", vec![("same class".into(), o)]);
    }
);

/// Every citation-only edit to `name` leaves the base version.
fn check_citation_free(name: &str) {
    let b = base();
    for c in [
        Citation::Corpus {
            file: "docs/design/principia_dd_generation_root.md",
            section: "qa another section",
        },
        Citation::PrinRs {
            commit: "0000000",
            path: "results/qa.md",
        },
        Citation::Calibration("REQ-QA-000"),
    ] {
        let o = with_register_edit(name, |k| k.citation = Some(c));
        assert_eq!(
            o.version(),
            b,
            "a citation-only edit to `{name}` changed the version"
        );
    }
    let o = with_register_edit(name, |k| k.citation = None);
    assert_eq!(
        o.version(),
        b,
        "a citation-only edit to `{name}` changed the version"
    );
}

#[test]
fn qa_m012_a_citation_only_edit_to_any_hashed_entry_leaves_the_version() {
    for name in SECTION_3_8_HASHED {
        check_citation_free(name);
    }
}

negative_control!(
    qa_m012_a_citation_only_edit_to_any_hashed_entry_leaves_the_version,
    "a register whose fgw_capacity value moved gives another version, so the unchanged check must fail",
    expected = "changed the version",
    {
        let o = with_register_edit("fgw_capacity", |k| k.value = Some(Value::Exact(75.0)));
        assert_eq!(
            o.version(),
            base(),
            "a citation-only edit to `fgw_capacity` changed the version"
        );
    }
);

/// Register entries outside §3.8's list, added and then edited in value, type, class and citation, never move the
/// version — including ones whose names extend a hashed name (a prefix match would wrongly hash them).
fn check_unhashed(names: &[&'static str]) {
    let b = base();
    for &name in names {
        let knob = ConstantBuilder {
            name,
            value: Some(Value::Exact(0.5)),
            class: Some(Admissibility::CanonicalUnits),
            citation: Some(Citation::Corpus {
                file: "docs/contracts/principia_render_contract.md",
                section: "qa",
            }),
            relative_basis: None,
        };
        for k in [
            knob,
            ConstantBuilder {
                value: Some(Value::Exact(7.0)),
                ..knob
            },
            ConstantBuilder {
                value: Some(Value::Calibration),
                citation: Some(Citation::Calibration("REQ-QA-001")),
                ..knob
            },
            ConstantBuilder {
                class: Some(Admissibility::ConservationLaw),
                ..knob
            },
        ] {
            let mut o = Owned::new();
            o.register.push(k);
            assert_eq!(
                o.version(),
                b,
                "the unhashed entry `{name}` changed the version"
            );
        }
    }
}

#[test]
fn qa_m012_an_entry_that_does_not_decide_stored_bits_is_not_hashed() {
    check_unhashed(&[
        "qa_render_gamma",
        "qa_scheduler_budget",
        "fgw_capacity_display",
        "f16_finite_max_render",
        "horizon_steps_max2",
    ]);
}

negative_control!(
    qa_m012_an_entry_that_does_not_decide_stored_bits_is_not_hashed,
    "an entry named fgw_capacity, with another value and class, is the hashed entry, so the unchanged check must \
     fail on it",
    expected = "fgw_capacity",
    {
        let b = base();
        let mut o = Owned::new();
        o.register.retain(|k| k.name != "fgw_capacity");
        o.register.push(ConstantBuilder {
            name: "fgw_capacity",
            value: Some(Value::Exact(0.5)),
            class: Some(Admissibility::CanonicalUnits),
            citation: None,
            relative_basis: None,
        });
        assert_eq!(o.version(), b, "the unhashed entry `fgw_capacity` changed the version");
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-008: the emitted constant is the computed hash, for the base ledger and for an edited one (seam 13: an edit
// to one ledger entry changes the generated artefact with it), and the checked-in file carries the base's.

fn emitted_literal(text: &str) -> u64 {
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("PAYLOAD_SCHEMA_VERSION"))
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "PAYLOAD_SCHEMA_VERSION is declared once: {lines:?}"
    );
    let lit = lines[0]
        .split('=')
        .nth(1)
        .expect("a value")
        .trim()
        .trim_end_matches(';')
        .replace('_', "");
    let lit = lit.trim();
    if let Some(hex) = lit.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).expect("a hex literal")
    } else {
        lit.parse().expect("a decimal literal")
    }
}

fn generated_rust(l: &ledger::schema::Ledger) -> String {
    gen::generate(l, gen::EMITTERS)
        .expect("the ledger generates")
        .into_iter()
        .find(|f| f.path.ends_with("generated.rs"))
        .expect("the Rust emitter writes generated.rs")
        .contents
}

/// A ledger edit that still passes generation: the first entry's provenance moved.
fn edited_ledger() -> ledger::schema::Ledger {
    let mut l = layout();
    let e = &mut l.entries[0];
    e.provenance = Some(match e.provenance {
        Some(Provenance::Kernel) => Provenance::Cpu,
        _ => Provenance::Kernel,
    });
    l
}

fn version_of(l: &ledger::schema::Ledger) -> u64 {
    let entries = gen::validate(l).expect("validates");
    let structs = payload::structs();
    schema_version(&Hashed::payload(&l.words, &entries, &structs)).expect("computed")
}

fn check_emitted_is_hash(l: &ledger::schema::Ledger, expected: u64) {
    assert_eq!(
        emitted_literal(&generated_rust(l)),
        expected,
        "the emitted version is not the ledger's hash"
    );
}

#[test]
fn qa_m012_the_emitted_constant_is_the_hash_of_base_and_edited_ledgers() {
    let base_l = layout();
    let edited = edited_ledger();
    assert_ne!(
        version_of(&base_l),
        version_of(&edited),
        "the edit is a ledger change"
    );
    check_emitted_is_hash(&base_l, version_of(&base_l));
    check_emitted_is_hash(&edited, version_of(&edited));
    assert_eq!(
        version_of(&base_l),
        base(),
        "Hashed::payload is the ledger's"
    );
}

negative_control!(
    qa_m012_the_emitted_constant_is_the_hash_of_base_and_edited_ledgers,
    "the edited ledger's generated file carries the edited hash, not the base's, so the check must fail",
    expected = "the emitted version is not the ledger's hash",
    check_emitted_is_hash(&edited_ledger(), version_of(&layout()))
);

fn check_checked_in(expected: u64) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kernel/src/payload/generated.rs");
    let text = std::fs::read_to_string(&path).expect("generated.rs reads");
    assert_eq!(
        emitted_literal(&text),
        expected,
        "the checked-in version is not the ledger's hash"
    );
}

/// The checked-in file was generated in another process (and, on CI, read on another machine): its literal equals
/// this process's hash, so the version is stable across runs and machines.
#[test]
fn qa_m012_the_checked_in_constant_is_the_hash() {
    check_checked_in(version_of(&layout()));
}

negative_control!(
    qa_m012_the_checked_in_constant_is_the_hash,
    "an edited ledger's hash is not the checked-in one, so the check must fail",
    expected = "the checked-in version is not the ledger's hash",
    check_checked_in(version_of(&edited_ledger()))
);

// ---------------------------------------------------------------------------------------------------------------
// R-36 / dd_generation_root §6: no hand-bumped version number exists anywhere. No crate's hand-written source
// declares a schema-version constant; only the generated file carries one.

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).expect("reads").flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                rust_files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The hand-written `.rs` files under `crates/*/src` that declare a `const`/`static` whose name contains
/// `SCHEMA_VERSION` or `PAYLOAD_VERSION` and gives it a numeric literal.
fn hand_versions(crates: &Path, generated: &[&str]) -> Vec<String> {
    let mut files = Vec::new();
    for c in std::fs::read_dir(crates).expect("crates/ reads").flatten() {
        let src = c.path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    let mut found = Vec::new();
    for f in files {
        let s = f.to_string_lossy().replace('\\', "/");
        if generated.iter().any(|g| s.ends_with(g)) {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        for (n, line) in text.lines().enumerate() {
            let t = line.trim_start();
            let decl = t.starts_with("const ")
                || t.starts_with("pub const ")
                || t.starts_with("static ")
                || t.starts_with("pub static ")
                || t.starts_with("pub(crate) const ");
            let upper = t.to_ascii_uppercase();
            // A hand-bumped version is a numeric literal; an emitter's format string (`= {v:#018x}`) is not one.
            let literal = t.split_once('=').is_some_and(|(_, rhs)| {
                rhs.trim_start().starts_with(|ch: char| ch.is_ascii_digit())
            });
            if decl
                && literal
                && (upper.contains("SCHEMA_VERSION") || upper.contains("PAYLOAD_VERSION"))
            {
                found.push(format!("{s}:{}", n + 1));
            }
        }
    }
    found
}

fn check_no_hand_version(generated: &[&str]) {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let found = hand_versions(&crates, generated);
    assert!(found.is_empty(), "a hand-written schema version: {found:?}");
}

#[test]
fn qa_m012_no_hand_written_schema_version_exists() {
    check_no_hand_version(&["kernel/src/payload/generated.rs"]);
}

negative_control!(
    qa_m012_no_hand_written_schema_version_exists,
    "without the generated file's exemption, its PAYLOAD_SCHEMA_VERSION is found, so the check must fail",
    expected = "a hand-written schema version",
    check_no_hand_version(&[])
);
