//! QA tests for TASK-M0-11 on the ledger side, written from REQ-PAY-019 (R-18, R-306; dd_generation_root §3.7),
//! payload §3's `.w` bit map and frozen continuation table (dd_simstate_payload §3, R-307), not from the
//! implementation. Each member name and type below is transcribed from §3.7's tables. Each test has a registered
//! negative control (R-176).

use ledger::gen::{self, rust};
use ledger::layout;
use ledger::payload::{self, ReductionMember};
use ledger::schema::{Bound, Ledger, Location};
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-019: §3.7's QuadReduction member list, ledger data (R-306). `spread_event` is stored as f16; no
// `ensemble_outcome_agreement`, nor any stored agreement value, exists (R-18).

/// Generation-root §3.7's tables, member and type, in order. The `escape_time_min` / `_max` row (`f16 × 2`) is two
/// f16 members. `n_unresolved` is the temporal-accumulators table's u16 row (R-315, closing RQ-183).
const SECTION_3_7: [(&str, &str); 24] = [
    ("level", "u8"),
    ("class_histogram[N]", "u8 × N"),
    ("dominant_outcome", "packed"),
    ("outcome_impurity", "f16"),
    ("terminated_fraction", "f16"),
    ("spread_shape", "f16"),
    ("spread_event", "f16"),
    ("error_ratio", "f16"),
    ("roundtrip_error", "f16"),
    ("spread_winner", "2 bits"),
    ("alpha_area", "f16"),
    ("alpha_energy", "f16"),
    ("worst_energy_drift", "f16"),
    ("running_mean_divergence", "f32"),
    ("first_divergence_t", "f32"),
    ("n_unresolved", "u16"),
    ("suspect_fraction", "f16"),
    ("saturated_fraction", "f16"),
    ("valid_sample_count", "u16"),
    ("coherence", "f32"),
    ("escape_time_min", "f16"),
    ("escape_time_max", "f16"),
    ("retrograde_fraction", "f16"),
    ("mean_orbit_count_spread", "f16"),
];

/// `spread_event` appears once, typed f16; no member is `ensemble_outcome_agreement` or any other stored agreement;
/// §3.7's non-members (`id`, the per-footprint latch `running_max_divergence`, the conditional `spread_t_end`) are
/// absent; and every typed member is §3.7's, with its type, in its order.
fn check_members(members: &[ReductionMember]) {
    let spread: Vec<Option<&str>> = members
        .iter()
        .filter(|m| m.name == "spread_event")
        .map(|m| m.ty)
        .collect();
    assert_eq!(
        spread,
        [Some("f16")],
        "`spread_event` is one member, stored as f16 (R-18)"
    );
    for m in members {
        assert!(
            !m.name.contains("agreement"),
            "`{}` stores an agreement value; it is derived from spread_event on the fly (R-18)",
            m.name
        );
        assert!(
            !["id", "running_max_divergence", "spread_t_end"].contains(&m.name),
            "`{}` is not a §3.7 member",
            m.name
        );
    }
    let listed: Vec<(&str, &str)> = members
        .iter()
        .map(|m| (m.name, m.ty.unwrap_or("<none>")))
        .collect();
    assert_eq!(
        listed, SECTION_3_7,
        "the member list differs from §3.7's tables"
    );
}

#[test]
fn qa_spread_event_quad_reduction_members_are_section_3_7() {
    check_members(payload::QUAD_REDUCTION);
}

negative_control!(
    qa_spread_event_quad_reduction_members_are_section_3_7,
    "a member list with the retired ensemble_outcome_agreement beside spread_event must fail",
    expected = "`ensemble_outcome_agreement` stores an agreement value",
    check_members(
        &payload::QUAD_REDUCTION
            .iter()
            .copied()
            .chain([ReductionMember {
                name: "ensemble_outcome_agreement",
                ty: Some("f16"),
                section: "Ensemble spread",
            }])
            .collect::<Vec<_>>()
    )
);

negative_control!(
    qa_spread_event_duplicated,
    "a member list with spread_event twice, once f32, must fail the f16 check",
    expected = "`spread_event` is one member, stored as f16",
    check_members(
        &payload::QUAD_REDUCTION
            .iter()
            .copied()
            .chain([ReductionMember {
                name: "spread_event",
                ty: Some("f32"),
                section: "Ensemble spread",
            }])
            .collect::<Vec<_>>()
    )
);

negative_control!(
    qa_spread_event_n_unresolved_untyped,
    "a member list with n_unresolved untyped (before R-315) must fail the §3.7 comparison",
    expected = "the member list differs from §3.7's tables",
    check_members(
        &payload::QUAD_REDUCTION
            .iter()
            .copied()
            .map(|m| if m.name == "n_unresolved" {
                ReductionMember { ty: None, ..m }
            } else {
                m
            })
            .collect::<Vec<_>>()
    )
);

negative_control!(
    qa_spread_event_missing_member,
    "a member list missing coherence must fail the §3.7 comparison",
    expected = "the member list differs from §3.7's tables",
    check_members(
        &payload::QUAD_REDUCTION
            .iter()
            .copied()
            .filter(|m| m.name != "coherence")
            .collect::<Vec<_>>()
    )
);

/// R-306: the member list is not emitted and has no §3.8 entries at M0; the struct is TASK-M5-01's. So neither
/// `spread_event` nor `ensemble_outcome_agreement` is a ledger entry, no ledger struct is `QuadReduction`, and the
/// generated Rust declares no `QuadReduction`.
fn check_not_emitted(ledger: &Ledger) {
    for name in ["spread_event", "ensemble_outcome_agreement"] {
        assert!(
            !ledger.entries.iter().any(|e| e.name == Some(name)),
            "`{name}` is a §3.8 ledger entry at M0 (R-306)"
        );
    }
    assert!(
        !payload::structs().iter().any(|s| s.name == "QuadReduction"),
        "the ledger emits a QuadReduction struct at M0 (R-306)"
    );
    let files = gen::generate(ledger, &[rust::emit]).expect("the payload ledger generates");
    for f in files {
        assert!(
            !f.contents.contains("QuadReduction") && !f.contents.contains("agreement"),
            "{} declares QuadReduction or an agreement value at M0 (R-306)",
            f.path.display()
        );
    }
}

#[test]
fn qa_spread_event_member_list_is_not_emitted() {
    check_not_emitted(&layout());
}

negative_control!(
    qa_spread_event_member_list_is_not_emitted,
    "a ledger with spread_event as a §3.8 entry must fail the not-emitted check",
    expected = "`spread_event` is a §3.8 ledger entry at M0",
    check_not_emitted(&{
        let mut l = layout();
        let mut e = l
            .entries
            .iter()
            .find(|e| e.name == Some("outcome_impurity") || e.name == Some("d_min"))
            .expect("a ledger entry to copy")
            .clone();
        e.name = Some("spread_event");
        l.entries.push(e);
        l
    })
);

// ---------------------------------------------------------------------------------------------------------------
// Payload §3's `.w` bit map in the ledger: the word's `.w` holds the payload's high 25 bits at 0–24 and `length` at
// 25–31, tiling all 32 bits with nothing reserved; `length` is 0…76 valid with 127 its sentinel, outside the range.

/// `(name, offset, width)` of every entry in the `.w` word, sorted by offset.
fn w_fields(ledger: &Ledger) -> Vec<(&'static str, u32, u32)> {
    let word = ledger
        .words
        .iter()
        .find(|w| {
            ledger.entries.iter().any(|e| {
                e.name == Some("length")
                    && matches!(e.location, Some(Location::Packed { word, .. }) if word == w.name)
            })
        })
        .expect("a ledger word holds `length`");
    assert_eq!(word.bits, 32, "the `.w` word is a u32");
    assert!(
        word.reserved.is_empty(),
        "`.w` reserves nothing (payload §3)"
    );
    let mut fields: Vec<(&'static str, u32, u32)> = ledger
        .entries
        .iter()
        .filter_map(|e| match (e.name, &e.location) {
            (
                Some(n),
                Some(Location::Packed {
                    word: w,
                    offset,
                    width,
                }),
            ) if *w == word.name => Some((n, *offset, *width)),
            _ => None,
        })
        .collect();
    fields.sort_by_key(|f| f.1);
    fields
}

fn check_w_bit_map(ledger: &Ledger) {
    let fields = w_fields(ledger);
    let spans: Vec<(u32, u32)> = fields.iter().map(|&(_, o, w)| (o, w)).collect();
    assert_eq!(
        spans,
        [(0, 25), (25, 7)],
        "`.w` is the payload's 25 bits at 0–24 then `length`'s 7 at 25–31 (payload §3): {fields:?}"
    );
    assert_eq!(fields[1].0, "length", "bits 25–31 are `length`");
    let entries = gen::validate(ledger).expect("the ledger validates");
    let length = entries
        .iter()
        .find(|e| e.name == "length")
        .expect("`length` validates");
    assert_eq!(
        (length.range.lo, length.range.hi, length.sentinel),
        (Bound::Closed(0.0), Bound::Closed(76.0), Some(127.0)),
        "`length` is 0…76 with 127 the truncation sentinel (payload §3)"
    );
}

#[test]
fn qa_word_w_bit_map_is_payload_section_3() {
    check_w_bit_map(&layout());
}

negative_control!(
    qa_word_w_bit_map_is_payload_section_3,
    "a ledger whose `length` range runs to 127 (sentinel inside the range) must fail",
    expected = "`length` is 0…76 with 127 the truncation sentinel",
    check_w_bit_map(&{
        let mut l = layout();
        for e in l.entries.iter_mut() {
            if e.name == Some("length") {
                e.range = Some(ledger::schema::Range::int(0, 127));
            }
        }
        l
    })
);

negative_control!(
    qa_word_w_bit_map_length_moved,
    "a ledger with `length` at bits 24–30 must fail the bit-map check",
    expected = "`.w` is the payload's 25 bits at 0–24",
    check_w_bit_map(&{
        let mut l = layout();
        for e in l.entries.iter_mut() {
            if e.name == Some("length") {
                if let Some(Location::Packed { offset, .. }) = &mut e.location {
                    *offset = 24;
                }
            }
        }
        l
    })
);

// ---------------------------------------------------------------------------------------------------------------
// The ledger's continuation table is payload §3's frozen arrays, `continuation_index` with 3 in its inverse cells.

type Arrays = ([u32; 4], [[u32; 4]; 3], [[u32; 4]; 3], [[u32; 4]; 4]);

fn check_ledger_tables(t: Arrays) {
    let (inv, cont, pred, index) = t;
    assert_eq!(inv, [1, 0, 3, 2], "inverse (payload §3)");
    assert_eq!(
        cont,
        [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]],
        "cont_symbol (payload §3)"
    );
    assert_eq!(pred, cont, "predecessor_symbol == cont_symbol (payload §3)");
    assert_eq!(
        index,
        [[0, 3, 1, 2], [3, 0, 2, 1], [1, 2, 0, 3], [2, 1, 3, 0]],
        "continuation_index (payload §3, R-307)"
    );
}

fn ledger_arrays() -> Arrays {
    (
        payload::inverse(),
        payload::cont_symbol(),
        payload::predecessor_symbol(),
        payload::continuation_index(),
    )
}

#[test]
fn qa_ledger_continuation_table_is_payload_section_3() {
    check_ledger_tables(ledger_arrays());
}

negative_control!(
    qa_ledger_continuation_table_is_payload_section_3,
    "a continuation_index with 2 in cell [0][1] (an inverse cell) must fail",
    expected = "continuation_index (payload §3, R-307)",
    check_ledger_tables({
        let mut t = ledger_arrays();
        t.3[0][1] = 2;
        t
    })
);
