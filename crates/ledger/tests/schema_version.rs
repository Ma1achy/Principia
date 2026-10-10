//! The payload schema version, the content hash of the ledger (R-36, R-63; dd_generation_root §5 test 7, §6): one
//! ledger row, one continuation-table entry, one code assignment (R-22's pair map, payload §2's `state` codes and
//! `detail` meanings, payload §3's symbol codes) or one `QuadReduction` member changes it; the unchanged ledger gives the
//! same version on every run, in any source order; the emitted `PAYLOAD_SCHEMA_VERSION` equals the computed hash
//! (REQ-GEN-008). A hashed register entry's value, type or class changes it, its citation does not, and a register
//! entry that does not decide stored bits leaves it unchanged (dd_generation_root §3.8, "The hash"; REQ-SYS-063,
//! R-251).

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
use ledger::gen;
use ledger::layout;
use ledger::payload::{self, ReductionMember};
use ledger::schema::{Entry, Location, Storage, Struct, Word};
use ledger::version::{fnv1a64, schema_version, Hashed, STORED_BITS};
use validation::negative_control;

/// The payload ledger, owned, so a test can edit one part of it.
struct Owned {
    words: Vec<Word>,
    entries: Vec<Entry>,
    structs: Vec<Struct>,
    quad_reduction: Vec<ReductionMember>,
    register: Vec<ConstantBuilder>,
}

impl Owned {
    fn new() -> Self {
        let ledger = layout();
        Owned {
            words: ledger.words.clone(),
            entries: gen::validate(&ledger).expect("the payload ledger validates"),
            structs: payload::structs(),
            quad_reduction: payload::QUAD_REDUCTION.to_vec(),
            register: REGISTER.to_vec(),
        }
    }

    /// Its [`Hashed`], with payload §3's continuation table.
    fn hashed(&self) -> Hashed<'_> {
        Hashed {
            quad_reduction: &self.quad_reduction,
            register: &self.register,
            ..Hashed::payload(&self.words, &self.entries, &self.structs)
        }
    }
}

fn version(h: &Hashed) -> u64 {
    schema_version(h).expect("the schema version is computed")
}

/// The version of the unedited payload ledger.
fn base() -> u64 {
    version(&Owned::new().hashed())
}

/// `edited`'s version differs from the unedited ledger's.
fn check_changes(edited: &Hashed) {
    assert_ne!(version(edited), base(), "the schema version did not change");
}

/// `edited`'s version equals the unedited ledger's.
fn check_unchanged(edited: &Hashed) {
    assert_eq!(version(edited), base(), "the schema version changed");
}

/// The payload ledger with the hashed register entry `fgw_capacity` replaced by `edit` of it.
fn with_capacity(edit: impl FnOnce(ConstantBuilder) -> ConstantBuilder) -> Owned {
    let mut o = Owned::new();
    let k = o
        .register
        .iter_mut()
        .find(|k| k.name == "fgw_capacity")
        .expect("the register holds fgw_capacity");
    *k = edit(*k);
    o
}

/// A register entry that decides no stored bit: a render constant the lint brings into the register (§3.8).
const RENDER_KNOB: ConstantBuilder = ConstantBuilder {
    name: "fx_render_knob",
    value: Some(Value::Exact(0.5)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(Citation::Corpus {
        file: "docs/contracts/principia_render_contract.md",
        section: "fixture",
    }),
    relative_basis: None,
};

// --- REQ-GEN-008: the ledger, the continuation table, stability, the emitted constant -------------------------------

/// Flipping one bit offset: the first `Packed` entry in source order, moved one bit.
fn one_row_moved() -> Owned {
    let mut o = Owned::new();
    let e = o
        .entries
        .iter_mut()
        .find(|e| matches!(e.location, Location::Packed { .. }))
        .expect("the ledger has a packed entry");
    if let Location::Packed { offset, .. } = &mut e.location {
        *offset ^= 1;
    }
    o
}

#[test]
fn schema_version_changes_with_one_ledger_row() {
    check_changes(&one_row_moved().hashed());
}

negative_control!(
    schema_version_changes_with_one_ledger_row,
    "the unedited ledger has the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    check_changes(&Owned::new().hashed())
);

#[test]
fn schema_version_changes_with_one_struct_member() {
    let mut o = Owned::new();
    o.structs[0].members[0].storage = Storage::U32x4;
    check_changes(&o.hashed());
}

negative_control!(
    schema_version_changes_with_one_struct_member,
    "the unedited structs have the base version, so the change check must fail on them",
    expected = "the schema version did not change",
    check_changes(&Owned::new().hashed())
);

#[test]
fn schema_version_changes_with_one_quad_reduction_member() {
    let mut o = Owned::new();
    o.quad_reduction[0].ty = Some("u16");
    check_changes(&o.hashed());
}

negative_control!(
    schema_version_changes_with_one_quad_reduction_member,
    "the unedited member list has the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    check_changes(&Owned::new().hashed())
);

/// Each continuation-table cell of `h` edited alone: every one changes the version.
fn check_each_cell_changes(h: Hashed) {
    for i in 0..4 {
        let mut e = h;
        e.inverse[i] ^= 1;
        check_changes(&e);
    }
    for d in 0..3 {
        for s in 0..4 {
            let mut e = h;
            e.cont_symbol[d][s] ^= 1;
            check_changes(&e);
            let mut e = h;
            e.predecessor_symbol[d][s] ^= 1;
            check_changes(&e);
        }
    }
    for p in 0..4 {
        for n in 0..4 {
            let mut e = h;
            e.continuation_index[p][n] ^= 1;
            check_changes(&e);
        }
    }
}

#[test]
fn schema_version_changes_with_one_continuation_table_entry() {
    check_each_cell_changes(Owned::new().hashed());
}

negative_control!(
    schema_version_changes_with_one_continuation_table_entry,
    "an edit undone by a second edit restores the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    {
        let o = Owned::new();
        let mut h = o.hashed();
        h.cont_symbol[1][2] ^= 1;
        h.cont_symbol[1][2] ^= 1;
        check_changes(&h);
    }
);

/// Two runs' versions are one.
fn check_runs_agree(first: u64, second: u64) {
    assert_eq!(first, second, "two runs gave two versions");
}

/// Each code assignment of `h` edited alone: every symbol code, `state` code, pair-id map entry and `detail` meaning
/// changes the version (R-22; payload §2, §3).
fn check_each_assignment_changes(h: Hashed) {
    for i in 0..4 {
        let mut e = h;
        e.symbols.swap(i, (i + 2) % 4);
        check_changes(&e);
    }
    for i in 0..6 {
        let mut e = h;
        e.states.swap(i, (i + 1) % 6);
        check_changes(&e);
    }
    for k in 0..3 {
        for b in 0..2 {
            let mut e = h;
            e.pair_bodies[k][b] ^= 3;
            check_changes(&e);
        }
    }
    for s in 0..4 {
        let mut e = h;
        e.detail_meanings[s].0 = "fx_state";
        check_changes(&e);
        for code in 0..4 {
            let mut e = h;
            e.detail_meanings[s].1[code] = "fx_meaning";
            check_changes(&e);
        }
    }
}

#[test]
fn schema_version_changes_with_one_code_assignment() {
    check_each_assignment_changes(Owned::new().hashed());
}

negative_control!(
    schema_version_changes_with_one_code_assignment,
    "a swap undone by a second swap restores the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    {
        let o = Owned::new();
        let mut h = o.hashed();
        h.states.swap(0, 3);
        h.states.swap(0, 3);
        check_changes(&h);
    }
);

#[test]
fn schema_version_is_stable_across_runs() {
    check_runs_agree(
        version(&Owned::new().hashed()),
        version(&Owned::new().hashed()),
    );
}

negative_control!(
    schema_version_is_stable_across_runs,
    "a run over an edited ledger gives another version, so the agreement check must fail on it",
    expected = "two runs gave two versions",
    check_runs_agree(base(), version(&one_row_moved().hashed()))
);

/// The version of the ledger with its words and entries in reverse source order equals the base's.
fn check_order_free(o: &mut Owned) {
    o.words.reverse();
    o.entries.reverse();
    check_unchanged(&o.hashed());
}

#[test]
fn schema_version_does_not_depend_on_source_order() {
    check_order_free(&mut Owned::new());
}

negative_control!(
    schema_version_does_not_depend_on_source_order,
    "a moved bit offset is a real edit, so the unchanged check must fail on it",
    expected = "the schema version changed",
    check_order_free(&mut one_row_moved())
);

/// The `PAYLOAD_SCHEMA_VERSION` literal in `generated`.
fn emitted(generated: &str) -> u64 {
    let literal = generated
        .lines()
        .find_map(|l| l.strip_prefix("pub const PAYLOAD_SCHEMA_VERSION: u64 = "))
        .expect("the generated Rust declares PAYLOAD_SCHEMA_VERSION");
    let hex = literal.trim_end_matches(';').trim_start_matches("0x");
    u64::from_str_radix(hex, 16).expect("the version is a hex literal")
}

/// The generated Rust's emitted version, and the checked-in file's, equal `expected`.
fn check_emitted(expected: u64) {
    let files = gen::generate(&layout(), gen::EMITTERS).expect("the payload ledger generates");
    let rust = files
        .iter()
        .find(|f| f.path.ends_with("generated.rs"))
        .expect("the Rust emitter writes generated.rs");
    assert_eq!(
        emitted(&rust.contents),
        expected,
        "the emitted version is not the computed hash"
    );
    let on_disk = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kernel/src/payload/generated.rs"
    ))
    .expect("the checked-in generated.rs reads");
    assert_eq!(
        emitted(&on_disk),
        expected,
        "the emitted version is not the computed hash"
    );
}

#[test]
fn schema_version_emitted_equals_the_computed_hash() {
    check_emitted(base());
}

negative_control!(
    schema_version_emitted_equals_the_computed_hash,
    "an edited ledger's hash is not the emitted one, so the equality check must fail on it",
    expected = "the emitted version is not the computed hash",
    check_emitted(version(&one_row_moved().hashed()))
);

/// FNV-1a 64's published test vectors: the hash is the standard function, the same on every machine.
fn check_fnv(input: &str, expected: u64) {
    assert_eq!(fnv1a64(input.as_bytes()), expected, "not FNV-1a 64");
}

#[test]
fn schema_version_hash_is_fnv1a64() {
    check_fnv("", 0xcbf2_9ce4_8422_2325);
    check_fnv("a", 0xaf63_dc4c_8601_ec8c);
    check_fnv("foobar", 0x8594_4171_f739_67e8);
}

negative_control!(
    schema_version_hash_is_fnv1a64,
    "FNV-1 (multiply before xor) hashes \"a\" to 0xaf63bd4c8601b7be, so the FNV-1a check must fail on it",
    expected = "not FNV-1a 64",
    check_fnv("a", 0xaf63_bd4c_8601_b7be)
);

// --- REQ-SYS-063, R-251: the register entries that decide stored bits ----------------------------------------------

#[test]
fn schema_version_changes_with_a_hashed_value() {
    check_changes(
        &with_capacity(|k| ConstantBuilder {
            value: Some(Value::Exact(75.0)),
            ..k
        })
        .hashed(),
    );
}

negative_control!(
    schema_version_changes_with_a_hashed_value,
    "the same value leaves the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    check_changes(
        &with_capacity(|k| ConstantBuilder {
            value: Some(Value::Exact(76.0)),
            ..k
        })
        .hashed()
    )
);

#[test]
fn schema_version_changes_with_a_hashed_type() {
    check_changes(
        &with_capacity(|k| ConstantBuilder {
            value: Some(Value::Threshold(76.0)),
            ..k
        })
        .hashed(),
    );
}

negative_control!(
    schema_version_changes_with_a_hashed_type,
    "the same type leaves the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    check_changes(&with_capacity(|k| k).hashed())
);

#[test]
fn schema_version_changes_with_a_hashed_class() {
    check_changes(
        &with_capacity(|k| ConstantBuilder {
            class: Some(Admissibility::CanonicalUnits),
            ..k
        })
        .hashed(),
    );
}

negative_control!(
    schema_version_changes_with_a_hashed_class,
    "the same class leaves the base version, so the change check must fail on it",
    expected = "the schema version did not change",
    check_changes(
        &with_capacity(|k| ConstantBuilder {
            class: Some(Admissibility::AchievableMaximum),
            ..k
        })
        .hashed()
    )
);

#[test]
fn schema_version_ignores_a_citation_only_edit() {
    let cited = |k| ConstantBuilder {
        citation: Some(Citation::Calibration("REQ-FX-000")),
        ..k
    };
    check_unchanged(&with_capacity(cited).hashed());
}

negative_control!(
    schema_version_ignores_a_citation_only_edit,
    "a value edit is hashed, so the unchanged check must fail on it",
    expected = "the schema version changed",
    check_unchanged(
        &with_capacity(|k| ConstantBuilder {
            value: Some(Value::Exact(75.0)),
            ..k
        })
        .hashed()
    )
);

/// The register with a render knob added, then the knob's value, type and class edited: the base version each time.
fn check_knob_ignored(knob: ConstantBuilder) {
    let mut o = Owned::new();
    o.register.push(knob);
    check_unchanged(&o.hashed());
    for edit in [
        ConstantBuilder {
            value: Some(Value::Exact(0.25)),
            ..knob
        },
        ConstantBuilder {
            value: Some(Value::Calibration),
            citation: Some(Citation::Calibration("REQ-FX-000")),
            ..knob
        },
        ConstantBuilder {
            class: Some(Admissibility::ConservationLaw),
            ..knob
        },
    ] {
        *o.register.last_mut().expect("the knob") = edit;
        check_unchanged(&o.hashed());
    }
}

#[test]
fn schema_version_ignores_an_entry_that_does_not_decide_stored_bits() {
    check_knob_ignored(RENDER_KNOB);
}

negative_control!(
    schema_version_ignores_an_entry_that_does_not_decide_stored_bits,
    "a register entry named as a hashed one is hashed, so the unchanged check must fail on its edits",
    expected = "the schema version changed",
    {
        let mut o = Owned::new();
        o.register.retain(|k| k.name != "fgw_capacity");
        let knob = ConstantBuilder { name: "fgw_capacity", ..RENDER_KNOB };
        o.register.push(knob);
        check_unchanged(&o.hashed());
    }
);

/// The version is refused, naming `name`, when the register lacks it.
fn check_missing_refused(name: &str) {
    let mut o = Owned::new();
    o.register.retain(|k| k.name != name);
    let error = schema_version(&o.hashed()).expect_err("the version was not refused");
    assert!(
        error.contains(name),
        "refused, but not naming `{name}`: {error}"
    );
}

#[test]
fn schema_version_refuses_a_register_missing_a_hashed_entry() {
    for name in STORED_BITS {
        check_missing_refused(name);
    }
}

negative_control!(
    schema_version_refuses_a_register_missing_a_hashed_entry,
    "removing an unhashed name leaves every hashed entry, so the refusal check must fail",
    expected = "the version was not refused",
    check_missing_refused("fx_render_knob")
);

#[test]
fn schema_version_hashes_the_registers_stored_bits_entries() {
    for name in STORED_BITS {
        assert!(
            REGISTER.iter().any(|k| k.name == name),
            "`{name}` is not in the register"
        );
    }
    // §3.8 hashes the register entries that decide stored bits; every other entry is a chart constant, hashed by
    // value through the link registry instead (§3.9, "The hash"; R-344).
    for k in REGISTER.iter().filter(|k| !STORED_BITS.contains(&k.name)) {
        assert!(
            ledger::links::CHART_CONSTANTS
                .iter()
                .any(|c| c.name == k.name),
            "`{}` is neither a stored-bits entry nor a chart constant, so it is not hashed",
            k.name
        );
    }
}

negative_control!(
    schema_version_hashes_the_registers_stored_bits_entries,
    "a name outside the register fails the membership check",
    expected = "is not in the register",
    {
        let name = "fx_render_knob";
        assert!(
            REGISTER.iter().any(|k| k.name == name),
            "`{name}` is not in the register"
        );
    }
);
