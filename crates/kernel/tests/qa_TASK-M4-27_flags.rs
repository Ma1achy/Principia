//! QA (TASK-M4-27; REQ-VAL-347): the `INVARIANT_INFEASIBLE` read-back's sign flags, packed into one word (`flat` 1,
//! `negative` 2, `moving` 4, `branch_read`'s `FeasibilityBits`), keep each flag in its own bit. The comparison reads a
//! branch's input bits word for word across backends (REQ-VAL-347; pitfalls §9: no check whose output set excludes the
//! fork it guards), so two flag triples that pack to the same word would hide a fork in the flags. At the canonical
//! `I = 1`: `flat` is `K* = ±0`, `negative` is `K*`'s sign bit, `moving` is `L_z ≠ 0` (`decode/invariant.rs`,
//! `Feasibility`). The eight triples are reached at `K* ∈ {1, −1, +0, −0}`, `L_z ∈ {0, 1}`; each word is a literal, and
//! the eight are distinct.

use kernel::stage::branch_read as read;
use validation::negative_control;

/// `(K*, L_z, flags word)`: each flag alone, each pair, all three and none.
const CASES: [(f32, f32, u32); 8] = [
    (1.0, 0.0, 0),
    (0.0, 0.0, 1),
    (-1.0, 0.0, 2),
    (-0.0, 0.0, 3),
    (1.0, 1.0, 4),
    (0.0, 1.0, 5),
    (-1.0, 1.0, 6),
    (-0.0, 1.0, 7),
];

/// Each case's flags word, as `flags` gives it, against its literal, and the eight pairwise distinct.
fn check(flags: impl Fn(f32, f32) -> u32) {
    let mut seen = Vec::new();
    for (k, lz, want) in CASES {
        let got = flags(k, lz);
        assert_eq!(
            got, want,
            "the packed sign flags at K* = {k:?}, L_z = {lz} read {got}, not {want}"
        );
        assert!(
            !seen.contains(&got),
            "two flag triples pack to one word {got}"
        );
        seen.push(got);
    }
}

/// Word 4 of `write_feasibility`'s nine-word slot, written from a nonzero offset.
fn read_back(k: f32, lz: f32) -> u32 {
    let mut out = vec![0xdead_beefu32; 12];
    read::write_feasibility(&mut out, 2, k, lz);
    assert_eq!(
        out[..2],
        [0xdead_beef; 2],
        "the slot wrote before its offset"
    );
    assert_eq!(out[11], 0xdead_beef, "the slot wrote past its nine words");
    out[2 + 4]
}

#[test]
fn qa_branch_read_feasibility_flags_keep_their_bits() {
    check(read_back);
}

negative_control!(
    qa_branch_read_feasibility_flags_keep_their_bits,
    "a packing that ORs negative and moving into one bit",
    expected = "the packed sign flags",
    check(|k, lz| {
        let w = read_back(k, lz);
        (w & 1) | (u32::from(w & 6 != 0) << 1)
    })
);
