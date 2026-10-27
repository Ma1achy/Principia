//! The fragment words' layout table (`ledger::fragment`; TASK-M2-25) and its emitter (`ledger::gen::fragment`): each
//! part's offset is the sum of the lengths before it, a layout's size their sum, an unknown part has none, and the
//! generated file holds every constant at its offset and is the one checked in.

use ledger::fragment::{self, layouts, Layout};
use validation::negative_control;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The table's offsets and sizes, `layout` added to the four.
fn check_offsets(extra: Option<Layout>) {
    let d = fragment::decode();
    assert_eq!(
        [
            d.at("UV"),
            d.at("Z"),
            d.at("Z_LATENT"),
            d.at("IC"),
            d.at("DESCRIPTOR"),
            d.at("TAG")
        ],
        [0, 2, 10, 11, 26, 38]
    );
    assert_eq!(d.words(), 39);
    let s = fragment::describe();
    assert_eq!((s.at("IC"), s.at("DESCRIPTOR"), s.words()), (0, 15, 27));
    let e = fragment::encode();
    assert_eq!((e.at("IC"), e.at("OUT"), e.words()), (0, 15, 27));
    let h = fragment::shape();
    assert_eq!((h.at("R"), h.at("M"), h.at("N"), h.words()), (0, 6, 9, 12));
    assert_eq!(d.offset("NONE"), None, "an unknown part has an offset");
    let all: Vec<Layout> = layouts().into_iter().chain(extra).collect();
    for l in &all {
        let mut at = 0;
        for p in &l.parts {
            assert_eq!(
                l.offset(p.name),
                Some(at),
                "{} {}: the offset is not the lengths before it",
                l.name,
                p.name
            );
            at += p.len;
        }
        assert_eq!(
            l.words(),
            at,
            "{}: the size is not the lengths' sum",
            l.name
        );
    }
}

#[test]
fn fragment_words_offsets_sum_the_lengths() {
    check_offsets(None);
}

negative_control!(
    fragment_words_offsets_sum_the_lengths,
    "a part named twice must not be read at its second place",
    expected = "the offset is not the lengths before it",
    check_offsets(Some(Layout {
        name: "TWICE",
        parts: vec![
            fragment::Part { name: "A", len: 1 },
            fragment::Part { name: "A", len: 2 }
        ],
    }))
);

/// The emitted file against the table, and the checked-in file against the emitted one.
fn check_file(file: &str) {
    for l in layouts() {
        for p in &l.parts {
            let line = format!(
                "pub const {}_{}: usize = {};\n",
                l.name,
                p.name,
                l.at(p.name)
            );
            assert!(
                file.contains(&line),
                "the generated file lacks `{}`",
                line.trim()
            );
        }
        let line = format!("pub const {}_WORDS: usize = {};\n", l.name, l.words());
        assert!(
            file.contains(&line),
            "the generated file lacks `{}`",
            line.trim()
        );
    }
    let checked_in =
        std::fs::read_to_string(root().join(ledger::gen::fragment::FRAGMENT_WORDS_PATH)).unwrap();
    assert_eq!(
        checked_in, file,
        "the checked-in fragment_words.rs is not codegen's"
    );
}

#[test]
fn fragment_words_file_is_the_table() {
    check_file(&ledger::gen::fragment::file());
    let emitted = ledger::gen::fragment::emit(&[], &[]);
    assert_eq!(emitted.len(), 1, "the emitter writes one file");
    assert_eq!(
        emitted[0].path,
        std::path::PathBuf::from(ledger::gen::fragment::FRAGMENT_WORDS_PATH)
    );
    assert_eq!(emitted[0].contents, ledger::gen::fragment::file());
}

negative_control!(
    fragment_words_file_is_the_table,
    "a file with an offset off by one must be found",
    expected = "the generated file lacks",
    check_file(
        &ledger::gen::fragment::file().replace("DECODE_TAG: usize = 38", "DECODE_TAG: usize = 37")
    )
);
