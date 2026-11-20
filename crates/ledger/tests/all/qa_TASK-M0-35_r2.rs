//! QA round 2 for TASK-M0-35, from REQ-GEN-003 (dd_generation_root §5 test 1): an overlap is refused naming the word
//! and the bits, and the bits named are the overlapping ones, also when three fields share bits and one pair's
//! overlap is split by another. Each test has a registered negative control (R-176).

use ledger::gen;
use ledger::schema::{
    Bound, Consumer, EntryBuilder, FieldType, Ledger, Location, Provenance, Range, Scale, Span,
    Word,
};
use validation::negative_control;

fn ubits(name: &'static str, offset: u32, width: u32) -> EntryBuilder {
    EntryBuilder::new(name)
        .location(Location::Packed {
            word: "qa_t",
            offset,
            width,
        })
        .ty(FieldType::UBits)
        .scale(Scale::Lin)
        .range(Range {
            lo: Bound::Closed(0.0),
            hi: Bound::Closed(((1u64 << width) - 1) as f64),
        })
        .provenance(Provenance::Kernel)
        .consumers(&[Consumer::Render, Consumer::Export, Consumer::Debug])
}

/// `qa_t` (8 bits, `reserved` spans as given) with fields at the given `(offset, width)`.
fn layout(reserved: &[(u32, u32)], fields: &[(&'static str, u32, u32)]) -> Ledger {
    Ledger {
        words: vec![Word {
            name: "qa_t",
            bits: 8,
            reserved: reserved
                .iter()
                .map(|&(offset, width)| Span { offset, width })
                .collect(),
        }],
        entries: fields.iter().map(|&(n, o, w)| ubits(n, o, w)).collect(),
    }
}

/// The bits named after `bits` in a line: each `a` or inclusive run `a–b`, expanded.
fn named_bits(line: &str) -> Vec<u32> {
    let Some((_, rest)) = line.split_once("bits") else {
        return Vec::new();
    };
    let mut bits = Vec::new();
    for token in rest
        .split(|c: char| !(c.is_ascii_digit() || c == '–' || c == '-'))
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
    {
        let mut ends = token
            .split(['–', '-'])
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().unwrap());
        if let Some(lo) = ends.next() {
            bits.extend(lo..=ends.next().unwrap_or(lo));
        }
    }
    bits
}

/// Generation is refused; every overlap line names word `qa_t`; the overlap lines together name exactly `expected`.
fn overlap_names_exactly(ledger: &Ledger, expected: &[u32]) {
    let message = match gen::generate(ledger, gen::EMITTERS) {
        Ok(_) => panic!("qa: layout not refused"),
        Err(e) => e.to_string(),
    };
    let lines: Vec<&str> = message.lines().filter(|l| l.contains("overlap")).collect();
    assert!(
        !lines.is_empty() && lines.iter().all(|l| l.contains("qa_t")),
        "qa: no overlap line naming the word: {message}"
    );
    let mut named: Vec<u32> = lines.iter().flat_map(|l| named_bits(l)).collect();
    named.sort_unstable();
    named.dedup();
    assert!(
        named == expected,
        "qa: overlap bits named {named:?}, not {expected:?}: {message}"
    );
}

#[test]
fn qa_layout_static_three_way_overlap_names_exactly_the_shared_bits() {
    // `qa_a` 0–7, `qa_b` 2–5, `qa_c` 4–6: bits 2–6 are claimed twice or more; 0–1 and 7 once.
    overlap_names_exactly(
        &layout(&[], &[("qa_a", 0, 8), ("qa_b", 2, 4), ("qa_c", 4, 3)]),
        &[2, 3, 4, 5, 6],
    );
}

negative_control!(
    qa_layout_static_three_way_overlap_names_exactly_the_shared_bits,
    "`qa_c` at 5–7 shares bit 7 too, so the exact-bits check must fail on 2–6",
    expected = "qa: overlap bits named",
    overlap_names_exactly(
        &layout(&[], &[("qa_a", 0, 8), ("qa_b", 2, 4), ("qa_c", 5, 3)]),
        &[2, 3, 4, 5, 6],
    )
);

#[test]
fn qa_layout_static_split_pair_overlap_names_exactly_the_shared_bits() {
    // Reserved bit 3; `qa_a` 0–7 overlaps it at 3, `qa_b` at 1–2 and `qa_c` at 5–6; bits 0, 4 and 7 are `qa_a`'s alone.
    overlap_names_exactly(
        &layout(&[(3, 1)], &[("qa_a", 0, 8), ("qa_b", 1, 2), ("qa_c", 5, 2)]),
        &[1, 2, 3, 5, 6],
    );
}

negative_control!(
    qa_layout_static_split_pair_overlap_names_exactly_the_shared_bits,
    "`qa_c` at 4–6 shares bit 4 too, so the exact-bits check must fail",
    expected = "qa: overlap bits named",
    overlap_names_exactly(
        &layout(&[(3, 1)], &[("qa_a", 0, 8), ("qa_b", 1, 2), ("qa_c", 4, 3)]),
        &[1, 2, 3, 5, 6],
    )
);
