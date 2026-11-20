//! QA tests for TASK-M0-13 (re-check after `members()` was rewritten), written from the requirements:
//! - REQ-PAY-091 (payload §1; R-86, R-343): the generated WGSL struct layouts match the ledger tables field by field.
//!   WGSL has no u16, so R-343 stores `closure_step` and `_reserved` as one `closure_step_reserved: u32`, the first
//!   in bits 0–15. Field by field then means: the WGSL members, read in order, store every ledger member exactly once
//!   and in the ledger's order; a member stores two ledger members only when both are adjacent u16s, named
//!   `<first>_<second>` with the second's leading `_` dropped (`closure_step` + `_reserved` →
//!   `closure_step_reserved`), typed `u32`; every other member keeps its own name and its storage's WGSL type.
//!
//! The property is checked exhaustively over every member sequence of length 0 to 6 drawn from {f32, u16, u32}
//! (1,093 structs), so a u16 pair at the start, middle and end, runs of three and more u16s, and lone u16s beside
//! f32 or u32 are all covered. Each test has a registered negative control (R-176).

use ledger::gen::wgsl::{self, WgslMember};
use ledger::schema::{Member, Storage, Struct};
use validation::negative_control;

const NAMES: [&str; 6] = ["m0", "_m1", "m2", "_m3", "m4", "_m5"];
const KINDS: [Storage; 3] = [Storage::F32, Storage::U16, Storage::U32];

fn wgsl_type(s: Storage) -> Option<&'static str> {
    match s {
        Storage::F32 => Some("f32"),
        Storage::U32 => Some("u32"),
        _ => None,
    }
}

/// Every sequence of `KINDS` of each length 0..=6.
fn all_structs() -> Vec<Struct> {
    let mut out = Vec::new();
    for len in 0..=NAMES.len() {
        let total = KINDS.len().pow(len as u32);
        for code in 0..total {
            let mut c = code;
            let members = (0..len)
                .map(|i| {
                    let storage = KINDS[c % KINDS.len()];
                    c /= KINDS.len();
                    Member {
                        name: NAMES[i],
                        storage,
                    }
                })
                .collect();
            out.push(Struct {
                name: "T",
                align: 4,
                buffer: None,
                indexed: false,
                members,
            });
        }
    }
    out
}

/// `got` stores `s`'s members field by field, as the module doc states.
fn check_field_by_field(s: &Struct, got: &[WgslMember]) {
    let ledger: Vec<&str> = s.members.iter().map(|m| m.name).collect();
    let stored: Vec<&str> = got.iter().flat_map(|w| w.stores.iter().copied()).collect();
    assert_eq!(
        stored, ledger,
        "the WGSL members do not store the ledger members once each, in order, for {s:?}"
    );
    let storage_of = |n: &str| s.members.iter().find(|m| m.name == n).unwrap().storage;
    let mut i = 0;
    for w in got {
        match w.stores.as_slice() {
            [a, b] => {
                assert!(
                    storage_of(a) == Storage::U16 && storage_of(b) == Storage::U16,
                    "a WGSL member stores two ledger members that are not both u16, for {s:?}"
                );
                assert_eq!(
                    (w.name.as_str(), w.ty.as_str()),
                    (format!("{a}_{}", b.trim_start_matches('_')).as_str(), "u32"),
                    "a u16 pair is not one u32 named <first>_<second>, for {s:?}"
                );
            }
            [a] => {
                assert_eq!(w.name, *a, "a lone member is renamed, for {s:?}");
                let st = storage_of(a);
                if let Some(t) = wgsl_type(st) {
                    assert_eq!(w.ty, t, "a lone member has the wrong WGSL type, for {s:?}");
                } else {
                    // A lone u16: it must have no u16 beside it that it could have been paired with, i.e. the
                    // pairing is greedy left to right, as two u16 members in a row are one u32 (R-343).
                    let next = s.members.get(i + 1).map(|m| m.storage);
                    assert!(
                        next != Some(Storage::U16),
                        "a u16 followed by a u16 is left unpaired, for {s:?}"
                    );
                    assert_ne!(w.ty, "u32", "a lone u16 is passed off as a u32, for {s:?}");
                }
            }
            other => panic!(
                "a WGSL member stores {} ledger members, for {s:?}",
                other.len()
            ),
        }
        i += w.stores.len();
    }
}

#[test]
fn qa_wgsl_layouts_members_store_the_ledger_field_by_field() {
    for s in all_structs() {
        check_field_by_field(&s, &wgsl::members(&s));
    }
}

/// The emitter's output with its last member dropped: a layout that loses a field.
#[cfg(feature = "controls")]
fn dropping_last(s: &Struct) -> Vec<WgslMember> {
    let mut m = wgsl::members(s);
    m.pop();
    m
}

negative_control!(
    qa_wgsl_layouts_members_store_the_ledger_field_by_field,
    "a layout that drops its last member does not store the ledger field by field",
    expected = "once each, in order",
    for s in all_structs() {
        check_field_by_field(&s, &dropping_last(&s));
    }
);

/// Every struct with two u16s in a row has them paired, greedily from the left (R-343).
#[test]
fn qa_wgsl_layouts_adjacent_u16s_are_paired() {
    for s in all_structs().into_iter().filter(|s| {
        s.members
            .windows(2)
            .any(|w| w.iter().all(|m| m.storage == Storage::U16))
    }) {
        check_field_by_field(&s, &wgsl::members(&s));
    }
}

/// The emitter's output with every u16 pair split back into two lone members.
#[cfg(feature = "controls")]
fn unpaired(s: &Struct) -> Vec<WgslMember> {
    wgsl::members(s)
        .into_iter()
        .flat_map(|w| {
            if w.stores.len() == 2 {
                w.stores
                    .iter()
                    .map(|n| WgslMember {
                        name: (*n).to_owned(),
                        ty: "u16_has_no_wgsl_type".to_owned(),
                        stores: vec![*n],
                    })
                    .collect()
            } else {
                vec![w]
            }
        })
        .collect()
}

negative_control!(
    qa_wgsl_layouts_adjacent_u16s_are_paired,
    "two adjacent u16s left as two members are not paired",
    expected = "a u16 followed by a u16 is left unpaired",
    for s in all_structs() {
        check_field_by_field(&s, &unpaired(&s));
    }
);

/// The payload ledger's own structs (`SimStateFTLE`, `SimStateBase`, `ICDescriptor`, ...) hold the same property.
fn check_payload(emit: fn(&Struct) -> Vec<WgslMember>) {
    let structs = ledger::payload::structs();
    assert!(
        structs.iter().any(|s| s.name == "SimStateFTLE"),
        "the payload ledger has no SimStateFTLE"
    );
    for s in &structs {
        let got = emit(s);
        let stored: Vec<&str> = got.iter().flat_map(|w| w.stores.iter().copied()).collect();
        let ledger: Vec<&str> = s.members.iter().map(|m| m.name).collect();
        assert_eq!(
            stored, ledger,
            "{}: WGSL members lose or reorder a field",
            s.name
        );
    }
    let ftle = structs.iter().find(|s| s.name == "SimStateFTLE").unwrap();
    let pair: Vec<_> = emit(ftle)
        .into_iter()
        .filter(|w| w.stores.len() == 2)
        .map(|w| (w.name, w.ty, w.stores))
        .collect();
    assert_eq!(
        pair,
        vec![(
            "closure_step_reserved".to_owned(),
            "u32".to_owned(),
            vec!["closure_step", "_reserved"]
        )],
        "SimStateFTLE's one u16 pair is not closure_step_reserved: u32 (R-343)"
    );
}

#[test]
fn qa_wgsl_layouts_payload_structs_store_the_ledger_field_by_field() {
    check_payload(wgsl::members);
}

negative_control!(
    qa_wgsl_layouts_payload_structs_store_the_ledger_field_by_field,
    "a layout that drops each payload struct's last member loses a field",
    expected = "WGSL members lose or reorder a field",
    check_payload(dropping_last)
);
