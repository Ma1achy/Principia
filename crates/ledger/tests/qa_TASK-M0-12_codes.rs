//! QA tests for TASK-M0-12's code-assignment tables, written from payload §2 (the `state` enum, the `detail` union
//! and its failure enums, R-22's pair-id map) and payload §3 (the frozen symbol codes), not from the implementation.
//! The schema version hashes these tables (R-36, R-63), so a hashed table that differs from the corpus would version
//! the wrong format: each table the version covers must be the corpus's, and the version must move when one of the
//! corpus's own assignments is permuted. Each test has a registered negative control (R-176).

use ledger::gen;
use ledger::layout;
use ledger::payload;
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

/// Payload §2: "0 escape · 1 bounded · 2 collision · 3 running · 4 sim_failed · 5 decode_failed".
const STATES: [&str; 6] = [
    "escape",
    "bounded",
    "collision",
    "running",
    "sim_failed",
    "decode_failed",
];

/// Payload §3: "Symbol codes `a=0, A=1, b=2, B=3`".
const SYMBOLS: [&str; 4] = ["a", "A", "b", "B"];

/// Payload §2 (R-22): "pair 0 = bodies (1, 2), pair 1 = (2, 0), pair 2 = (0, 1)".
const PAIRS: [[u32; 2]; 3] = [[1, 2], [2, 0], [0, 1]];

/// Payload §2's `detail`: meaningful when state ∈ {escape, collision, sim_failed, decode_failed}; for each, a
/// keyword each code's meaning must carry. escape → body id, 3 = triple ejection; collision → pair id, 3 = triple
/// collision; sim_failed: 0 NaN in state, 1 Inf/overflow in state, 2 non-finite derived quantity, 3 reserved;
/// decode_failed: 0 non-finite decode output, 1 degenerate configuration, 2 invalid mass construction, 3
/// other/reserved.
const DETAIL: [(&str, [&str; 4]); 4] = [
    ("escape", ["body 0", "body 1", "body 2", "triple ejection"]),
    (
        "collision",
        ["pair 0", "pair 1", "pair 2", "triple collision"],
    ),
    (
        "sim_failed",
        [
            "nan in state",
            "inf/overflow in state",
            "non-finite derived",
            "reserved",
        ],
    ),
    (
        "decode_failed",
        [
            "non-finite decode output",
            "degenerate configuration",
            "invalid mass construction",
            "reserved",
        ],
    ),
];

/// The code tables `h` hashes are payload §2's and §3's.
fn check_tables(h: &Hashed) {
    assert_eq!(
        h.states, STATES,
        "the hashed state codes are not payload §2's"
    );
    assert_eq!(
        h.symbols, SYMBOLS,
        "the hashed symbol codes are not payload §3's"
    );
    assert_eq!(h.pair_bodies, PAIRS, "the hashed pair map is not R-22's");
    // R-22's rule itself: pair k is the side opposite body k.
    for (k, pair) in h.pair_bodies.iter().enumerate() {
        assert!(
            !pair.contains(&(k as u32)) && pair[0] != pair[1] && pair.iter().all(|&b| b < 3),
            "the hashed pair map is not R-22's: pair {k} = {pair:?}"
        );
    }
    // The symbol codes agree with payload §3's `inverse = [1,0,3,2]`: a symbol's inverse is its case swap.
    for (i, s) in h.symbols.iter().enumerate() {
        let inv = h.symbols[h.inverse[i] as usize];
        assert!(
            inv != *s && inv.eq_ignore_ascii_case(s),
            "the hashed symbol codes are not payload §3's: inverse({s}) = {inv}"
        );
    }
    for (i, (state, keys)) in DETAIL.iter().enumerate() {
        let (got_state, meanings) = h.detail_meanings[i];
        assert_eq!(
            got_state, *state,
            "the hashed detail meanings are not payload §2's"
        );
        assert!(
            h.states.contains(&got_state),
            "the hashed detail meanings are not payload §2's: `{got_state}` is no state"
        );
        for (code, key) in keys.iter().enumerate() {
            assert!(
                meanings[code].to_ascii_lowercase().contains(key),
                "the hashed detail meanings are not payload §2's: {state} code {code} = `{}`, wants `{key}`",
                meanings[code]
            );
        }
    }
}

#[test]
fn qa_m012_the_hashed_code_tables_are_the_corpus() {
    check_tables(&Hashed::payload(&[], &[], &[]));
}

negative_control!(
    qa_m012_the_hashed_code_tables_are_the_corpus,
    "the pair map with pair 1 = (0, 2) is pair 1 naming a side that touches body 1, so the R-22 check must fail",
    expected = "the hashed pair map is not R-22's",
    {
        let mut h = Hashed::payload(&[], &[], &[]);
        h.pair_bodies[1] = [0, 2];
        check_tables(&h);
    }
);

negative_control!(
    qa_m012_the_hashed_code_tables_are_the_corpus_detail,
    "sim_failed's NaN and Inf codes exchanged are not payload §2's enum, so the detail check must fail",
    expected = "the hashed detail meanings are not payload §2's",
    {
        let mut h = Hashed::payload(&[], &[], &[]);
        h.detail_meanings[2].1.swap(0, 1);
        check_tables(&h);
    }
);

/// Each realistic re-assignment of codes, alone, gives a version distinct from the base and from every other.
fn check_reassignments_move(edits: Vec<(&str, Hashed)>) {
    let entries = gen::validate(&layout()).expect("the payload ledger validates");
    let words = layout().words;
    let structs = payload::structs();
    let base = schema_version(&Hashed::payload(&words, &entries, &structs)).expect("computed");
    let mut seen = std::collections::HashSet::from([base]);
    for (what, edit) in edits {
        let h = Hashed {
            words: &words,
            entries: &entries,
            structs: &structs,
            ..edit
        };
        let v = schema_version(&h).expect("computed");
        assert_ne!(v, base, "the schema version did not change: {what}");
        assert!(
            seen.insert(v),
            "two re-assignments gave one version: {what}"
        );
    }
}

fn reassignments() -> Vec<(&'static str, Hashed<'static>)> {
    let base = || Hashed::payload(&[], &[], &[]);
    let mut out = Vec::new();
    let mut h = base();
    h.states.swap(4, 5);
    out.push(("sim_failed and decode_failed codes exchanged", h));
    let mut h = base();
    h.states.swap(1, 3);
    out.push(("bounded and running codes exchanged", h));
    let mut h = base();
    h.symbols.swap(1, 2);
    out.push(("symbols A and b exchanged", h));
    let mut h = base();
    h.pair_bodies = [[0, 1], [1, 2], [2, 0]];
    out.push((
        "pair k names the side from body k (R-22's pre-amendment reading)",
        h,
    ));
    let mut h = base();
    h.pair_bodies[0] = [2, 1];
    out.push(("pair 0's bodies written in the other order", h));
    let mut h = base();
    h.detail_meanings[3].1.swap(1, 2);
    out.push(("decode_failed degenerate and mass codes exchanged", h));
    let mut h = base();
    h.detail_meanings.swap(0, 1);
    out.push(("escape and collision detail unions exchanged", h));
    out
}

#[test]
fn qa_m012_each_code_reassignment_changes_the_version() {
    check_reassignments_move(reassignments());
}

negative_control!(
    qa_m012_each_code_reassignment_changes_the_version,
    "a state swapped with itself is no re-assignment, so the change check must fail",
    expected = "the schema version did not change",
    {
        let mut h = Hashed::payload(&[], &[], &[]);
        h.states.swap(2, 2);
        check_reassignments_move(vec![("no-op", h)]);
    }
);

/// The checked-in generated Rust declares `STATE_<NAME>` for exactly payload §2's six states, each at its code.
fn check_state_constants(text: &str) {
    let mut found: Vec<(String, u32)> = text
        .lines()
        .filter_map(|l| l.strip_prefix("pub const STATE_"))
        .map(|rest| {
            let (name, value) = rest.split_once(": u32 = ").expect("a u32 state constant");
            (
                name.to_ascii_lowercase(),
                value
                    .trim_end_matches(';')
                    .trim()
                    .parse()
                    .expect("a decimal code"),
            )
        })
        .collect();
    found.sort_by_key(|(_, v)| *v);
    let want: Vec<(String, u32)> = STATES
        .iter()
        .enumerate()
        .map(|(c, s)| (s.to_string(), c as u32))
        .collect();
    assert_eq!(
        found, want,
        "the generated state constants are not payload §2's"
    );
}

#[test]
fn qa_m012_the_generated_state_constants_are_payload_section_2s() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kernel/src/payload/generated.rs"
    );
    check_state_constants(&std::fs::read_to_string(path).expect("generated.rs reads"));
}

negative_control!(
    qa_m012_the_generated_state_constants_are_payload_section_2s,
    "a file whose running code is 4 is not payload §2's, so the check must fail",
    expected = "the generated state constants are not payload §2's",
    {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../kernel/src/payload/generated.rs"
        );
        let text = std::fs::read_to_string(path).expect("reads").replace(
            "pub const STATE_RUNNING: u32 = 3;",
            "pub const STATE_RUNNING: u32 = 4;",
        );
        check_state_constants(&text);
    }
);
