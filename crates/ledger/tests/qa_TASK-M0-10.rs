//! QA tests for TASK-M0-10 on the ledger side, written from REQ-PAY-013, REQ-PAY-015 and REQ-PAY-092 against
//! payload §2's table, dd_generation_root §3.1 and §5, and R-271, not from the implementation. Each test has a
//! registered negative control (R-176).

use ledger::gen::{self, rust};
use ledger::layout;
use ledger::schema::{Ledger, Location, Span};
use validation::negative_control;

/// Payload §2's `packed_a`: `(field, first bit, width)`, the descriptor then `d_min`.
const PACKED_A: [(&str, u32, u32); 6] = [
    ("state", 0, 3),
    ("detail", 3, 2),
    ("saturated", 5, 1),
    ("dmin_pair", 6, 2),
    ("last_symbol", 8, 2),
    ("d_min", 16, 16),
];

fn location(ledger: &Ledger, name: &str) -> Option<Location> {
    ledger
        .entries
        .iter()
        .find(|e| e.name == Some(name))
        .and_then(|e| e.location.clone())
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-013: the generated extract/insert offsets and widths of every descriptor field equal payload §2's table.

/// Each of payload §2's fields sits in `ledger` at the table's `packed_a` bits.
fn check_locations(ledger: &Ledger) {
    for &(field, offset, width) in &PACKED_A {
        assert_eq!(
            location(ledger, field),
            Some(Location::Packed {
                word: "packed_a",
                offset,
                width
            }),
            "`{field}`'s location against payload §2"
        );
    }
}

#[test]
fn descriptor_table_qa_ledger_locations_match_payload_section_2() {
    check_locations(&layout());
}

/// The layout with `dmin_pair` and `last_symbol` exchanged: a valid layout, but not payload §2's.
#[cfg(feature = "controls")]
fn swapped() -> Ledger {
    let mut l = layout();
    for e in l.entries.iter_mut() {
        let to = match e.name {
            Some("dmin_pair") => 8,
            Some("last_symbol") => 6,
            _ => continue,
        };
        if let Some(Location::Packed { offset, .. }) = e.location.as_mut() {
            *offset = to;
        }
    }
    l
}

negative_control!(
    descriptor_table_qa_ledger_locations_match_payload_section_2,
    "dmin_pair and last_symbol exchanged is not payload §2's table, so the location check must fail",
    expected = "`dmin_pair`'s location",
    check_locations(&swapped())
);

/// The Rust the emitter writes from `ledger` is the checked-in `generated.rs`, so the kernel's behavioural tests of
/// the accessors' bits (`qa_TASK-M0-10.rs` in kernel) test what the ledger generates.
fn check_emitted_is_checked_in(ledger: &Ledger) {
    let files = gen::generate(ledger, &[rust::emit]).expect("the ledger generates");
    let emitted = files
        .into_iter()
        .find(|f| f.path.ends_with(rust::PATH))
        .expect("the Rust emitter writes generated.rs")
        .contents;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let on_disk =
        std::fs::read_to_string(root.join(rust::PATH)).expect("generated.rs is checked in");
    assert!(
        emitted == on_disk,
        "the emitted generated.rs differs from the checked-in one"
    );
}

#[test]
fn descriptor_table_qa_emitted_is_the_checked_in_file() {
    check_emitted_is_checked_in(&layout());
}

negative_control!(
    descriptor_table_qa_emitted_is_the_checked_in_file,
    "a layout with two descriptor fields exchanged emits different accessors, so the check must fail",
    expected = "differs from the checked-in one",
    check_emitted_is_checked_in(&swapped())
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-015: bits 10–15 are listed as reserved, and with payload §2's fields they tile packed_a exactly.

/// `packed_a`'s reserved spans in `ledger` are exactly bits 10–15, and with its fields cover each of the 32 bits
/// exactly once.
fn check_reserved_tiling(ledger: &Ledger) {
    let word = ledger
        .words
        .iter()
        .find(|w| w.name == "packed_a")
        .expect("packed_a");
    assert_eq!(
        word.reserved,
        [Span {
            offset: 10,
            width: 6
        }],
        "packed_a's reserved spans"
    );
    let mut cover = [0u32; 32];
    let spans = PACKED_A
        .iter()
        .map(|&(_, o, w)| (o, w))
        .chain(word.reserved.iter().map(|s| (s.offset, s.width)));
    for (o, w) in spans {
        for b in o..o + w {
            cover[b as usize] += 1;
        }
    }
    assert_eq!(
        cover, [1; 32],
        "packed_a's fields and reserved spans tile its 32 bits"
    );
}

#[test]
fn reserved_bits_qa_ledger_reserves_exactly_10_to_15() {
    check_reserved_tiling(&layout());
}

negative_control!(
    reserved_bits_qa_ledger_reserves_exactly_10_to_15,
    "a ledger reserving no bits leaves 10–15 unaccounted, so the reserved check must fail",
    expected = "packed_a's reserved spans",
    check_reserved_tiling(&{
        let mut l = layout();
        l.words
            .iter_mut()
            .filter(|w| w.name == "packed_a")
            .for_each(|w| w.reserved.clear());
        l
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-092: the ledger declares d_min's sentinel as +inf (R-271), no longer 0.0.

fn check_dmin_sentinel(ledger: &Ledger) {
    let e = ledger
        .entries
        .iter()
        .find(|e| e.name == Some("d_min"))
        .expect("d_min");
    assert_eq!(e.sentinel, Some(f64::INFINITY), "d_min's ledger sentinel");
}

#[test]
fn dmin_unset_qa_ledger_sentinel_is_positive_infinity() {
    check_dmin_sentinel(&layout());
}

negative_control!(
    dmin_unset_qa_ledger_sentinel_is_positive_infinity,
    "the 0.0 sentinel R-271 replaced must fail the check",
    expected = "d_min's ledger sentinel",
    check_dmin_sentinel(&{
        let mut l = layout();
        l.entries
            .iter_mut()
            .filter(|e| e.name == Some("d_min"))
            .for_each(|e| e.sentinel = Some(0.0));
        l
    })
);
