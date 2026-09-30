//! The Rust emitter's pack/unpack/insert code against payload §2 (REQ-PAY-013, REQ-PAY-015, REQ-PAY-092): each
//! descriptor field's generated extract and insert bits, the reserved bits 10–15 in the static check, `d_min`'s
//! sentinel (R-271), and the refusal of a packed word the accessors cannot name (payload §6).

use ledger::check;
use ledger::gen::{self, rust};
use ledger::layout;
use ledger::schema::{Ledger, Span, Word};
use validation::negative_control;

/// The emitted Rust file for the payload ledger.
fn emitted() -> String {
    let files = gen::generate(&layout(), &[rust::emit]).expect("the payload ledger generates");
    files
        .into_iter()
        .find(|f| f.path.ends_with(rust::PATH))
        .expect("the Rust emitter writes generated.rs")
        .contents
}

/// The last two arguments of the first `call(…)` in the body of `pub fn name(`, a one-line call ending the line (or
/// its line's last `)`): its offset and width.
fn bits_of(contents: &str, name: &str, call: &str) -> (u32, u32) {
    let start = contents
        .find(&format!("pub fn {name}("))
        .unwrap_or_else(|| panic!("no generated `{name}`"));
    let body = &contents[start..];
    let at = body.find(&format!("{call}(")).expect("a call in the body");
    let line = body[at + call.len() + 1..]
        .lines()
        .next()
        .expect("the call's line");
    let args: Vec<&str> = line[..line.rfind(')').expect("closing paren")]
        .split(", ")
        .collect();
    let n = |s: &str| s.trim().parse().expect("a numeric bit argument");
    (n(args[args.len() - 2]), n(args[args.len() - 1]))
}

/// Payload §2's descriptor table: `(field, first bit, width)`.
const TABLE: [(&str, u32, u32); 5] = [
    ("state", 0, 3),
    ("detail", 3, 2),
    ("saturated", 5, 1),
    ("dmin_pair", 6, 2),
    ("last_symbol", 8, 2),
];

/// Each field's generated `sd_<field>` extracts, and `set_<field>` inserts, exactly `table`'s bits.
fn check_table(contents: &str, table: &[(&str, u32, u32)]) {
    for &(field, offset, width) in table {
        let get = bits_of(contents, &format!("sd_{field}"), "extract");
        let set = bits_of(contents, &format!("set_{field}"), "insert");
        assert_eq!(
            (get, set),
            ((offset, width), (offset, width)),
            "`{field}`: generated (extract, insert) (offset, width) against payload §2"
        );
    }
}

#[test]
fn descriptor_table_generated_bits_match_payload_section_2() {
    check_table(&emitted(), &TABLE);
}

negative_control!(
    descriptor_table_generated_bits_match_payload_section_2,
    "`dmin_pair` at bits 7–8 is not payload §2's table, so the check must fail",
    expected = "`dmin_pair`: generated (extract, insert)",
    check_table(&emitted(), &[("dmin_pair", 7, 2)])
);

/// `packed_a` in `ledger` reserves exactly bits 10–15, and without that reservation the static check reports them.
fn check_reserved_listed(ledger: &Ledger) {
    let word = |l: &Ledger| -> Word {
        l.words
            .iter()
            .find(|w| w.name == "packed_a")
            .expect("packed_a")
            .clone()
    };
    assert_eq!(
        word(ledger).reserved,
        [Span {
            offset: 10,
            width: 6
        }],
        "packed_a's reserved spans in the ledger"
    );
    let mut bare = ledger.clone();
    bare.words
        .iter_mut()
        .filter(|w| w.name == "packed_a")
        .for_each(|w| w.reserved.clear());
    let entries = gen::validate(&bare).expect("validates");
    let found = check::check(&bare.words, &entries);
    assert!(
        found
            .iter()
            .any(|f| f.to_string().contains("bits 10–15 are neither covered")),
        "the static check does not report bits 10–15 unreserved: {found:?}"
    );
}

#[test]
fn reserved_bits_listed_as_reserved_by_the_static_check() {
    check_reserved_listed(&layout());
}

negative_control!(
    reserved_bits_listed_as_reserved_by_the_static_check,
    "a ledger reserving bits 10–14 only is not payload §2's, so the check must fail",
    expected = "packed_a's reserved spans in the ledger",
    check_reserved_listed(&{
        let mut l = layout();
        l.words[0].reserved = vec![Span {
            offset: 10,
            width: 5,
        }];
        l
    })
);

/// `d_min`'s sentinel in `ledger` is +∞ (R-271).
fn check_dmin_sentinel(ledger: &Ledger) {
    let e = ledger
        .entries
        .iter()
        .find(|e| e.name == Some("d_min"))
        .expect("d_min");
    assert_eq!(e.sentinel, Some(f64::INFINITY), "d_min's sentinel");
}

#[test]
fn dmin_unset_ledger_sentinel_is_inf() {
    check_dmin_sentinel(&layout());
}

negative_control!(
    dmin_unset_ledger_sentinel_is_inf,
    "the 0.0 sentinel R-271 replaced must fail the check",
    expected = "d_min's sentinel",
    check_dmin_sentinel(&{
        let mut l = layout();
        l.entries
            .iter_mut()
            .filter(|e| e.name == Some("d_min"))
            .for_each(|e| e.sentinel = Some(0.0));
        l
    })
);

/// Generation from `ledger` is refused, naming the packed word with no accessor prefix, `name`.
fn check_prefix_refused(ledger: &Ledger, name: &str) {
    let message = match gen::generate(ledger, &[rust::emit]) {
        Ok(_) => String::from("generated"),
        Err(e) => e.to_string(),
    };
    assert!(
        message.contains(&format!("packed word `{name}` has no accessor prefix")),
        "generation not refused naming `{name}`: {message}"
    );
}

#[test]
fn descriptor_table_generation_refuses_a_word_without_a_prefix() {
    let mut l = layout();
    l.words.push(Word {
        name: "packed_c",
        bits: 32,
        reserved: vec![Span {
            offset: 0,
            width: 32,
        }],
    });
    check_prefix_refused(&l, "packed_c");
}

negative_control!(
    descriptor_table_generation_refuses_a_word_without_a_prefix,
    "the payload ledger's own words all have prefixes, so it generates and the check must fail",
    expected = "generation not refused naming `packed_c`",
    check_prefix_refused(&layout(), "packed_c")
);

/// With `dE_max`'s sentinel set to `value`, the Rust emitter writes it as an f32 constant of that value: an
/// `f16-pair` sentinel is a float, finite or not.
fn check_f16_sentinel(value: Option<f64>, line: &str) {
    let mut l = layout();
    l.entries
        .iter_mut()
        .filter(|e| e.name == Some("dE_max"))
        .for_each(|e| e.sentinel = value);
    let files = gen::generate(&l, &[rust::emit]).expect("the ledger generates");
    let contents = &files
        .iter()
        .find(|f| f.path.ends_with(rust::PATH))
        .expect("the Rust emitter writes generated.rs")
        .contents;
    assert!(
        contents.lines().any(|l| l == line),
        "no line `{line}` in the generated file"
    );
}

#[test]
fn f16_pairs_finite_sentinel_is_emitted_as_f32() {
    check_f16_sentinel(Some(1.5), "pub const PB_DE_MAX_SENTINEL: f32 = 1.5;");
}

negative_control!(
    f16_pairs_finite_sentinel_is_emitted_as_f32,
    "the payload ledger gives `dE_max` no sentinel, so no constant is emitted and the check must fail",
    expected = "no line `pub const PB_DE_MAX_SENTINEL: f32 = 1.5;`",
    check_f16_sentinel(None, "pub const PB_DE_MAX_SENTINEL: f32 = 1.5;")
);
