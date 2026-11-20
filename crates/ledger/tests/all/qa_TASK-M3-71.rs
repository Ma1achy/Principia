//! QA tests for TASK-M3-71's register entries, written from dd_simstate_payload §2, `coll_floor_bits` (R-973;
//! integrator contract Part 7, "The floor's record, defined"), not from the implementation: the code table there
//! stores the floor `2^(code − 41)` as codes 1 to 145, the floors `2⁻⁴⁰` to `2¹⁰⁴`, in bits 0–7, with 0 "not recorded"
//! and 146 to 255 never written. So the register's `floor_exp_min` is −40, `floor_exp_max` 104 and `floor_code_bias`
//! 41: the least code, `floor_exp_min + floor_code_bias`, is 1 (0 stays free for "not recorded"), the greatest,
//! `floor_exp_max + floor_code_bias`, is 145, inside the 8-bit field.
//!
//! The three entries give a stored code its floor and its clamps, so each is hashed into the schema version as a
//! stored-bits entry by its value, type and class, never its citation (dd_generation_root §3.8, "The hash"; R-251):
//! changing any one of them changes the version, even where the stored range, codes 0 to 145, stays as it is (a bias of
//! 42 with `floor_exp_max` 103) (`qa_coll_floor_constants_move_the_version`).
//!
//! Its negative controls (R-176): a register missing an entry, one whose bias would put the least floor on code 0, and
//! an edit to an unhashed register entry, which does not move the version.
// The file name `qa_TASK-M3-71` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
use ledger::version::{canonical, fnv1a64, Hashed};
use validation::negative_control;

/// The settled number of the register entry `name` in `register`.
fn entry(register: &[ConstantBuilder], name: &str) -> f64 {
    register
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("the register has no `{name}`"))
        .number()
}

fn check_floor_entries(register: &[ConstantBuilder]) {
    let min = entry(register, "floor_exp_min");
    let max = entry(register, "floor_exp_max");
    let bias = entry(register, "floor_code_bias");
    assert!(
        min == -40.0,
        "`floor_exp_min` is {min}, not payload §2's −40"
    );
    assert!(
        max == 104.0,
        "`floor_exp_max` is {max}, not payload §2's 104"
    );
    assert!(
        bias == 41.0,
        "`floor_code_bias` is {bias}, not payload §2's 41"
    );
    assert!(
        min + bias == 1.0,
        "the least floor's code is {}, not 1",
        min + bias
    );
    assert!(
        max + bias == 145.0 && max + bias <= 255.0,
        "the greatest floor's code is {}, not 145",
        max + bias
    );
}

#[test]
fn qa_coll_floor_register_entries() {
    check_floor_entries(REGISTER);
}

#[cfg(feature = "controls")]
mod missing {
    use super::*;
    negative_control!(
        qa_coll_floor_register_entries,
        "a register without floor_code_bias",
        expected = "the register has no `floor_code_bias`",
        check_floor_entries(
            &REGISTER
                .iter()
                .copied()
                .filter(|k| k.name != "floor_code_bias")
                .collect::<Vec<_>>()
        )
    );
}

#[cfg(feature = "controls")]
mod bias_40 {
    use super::*;
    negative_control!(
        qa_coll_floor_register_entries,
        "a bias of 40, the least floor on code 0",
        expected = "`floor_code_bias` is 40",
        check_floor_entries(
            &REGISTER
                .iter()
                .map(|k| match k.name {
                    "floor_code_bias" => ConstantBuilder {
                        value: Some(ledger::constants::Value::Exact(40.0)),
                        ..*k
                    },
                    _ => *k,
                })
                .collect::<Vec<_>>()
        )
    );
}

/// The schema version with `register` in place of the constants register, every other input as generation hashes it.
fn version_with(register: &[ConstantBuilder]) -> u64 {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).expect("the payload ledger validates");
    let structs = ledger::payload::structs();
    fnv1a64(
        &canonical(&Hashed {
            register,
            ..Hashed::payload(&l.words, &entries, &structs)
        })
        .expect("canonicalised"),
    )
}

/// The register with each entry named in `edits` replaced by its edit.
/// One register entry's change: its name, and the edit.
type EntryEdit = (&'static str, fn(ConstantBuilder) -> ConstantBuilder);

fn edited(edits: &[EntryEdit]) -> Vec<ConstantBuilder> {
    REGISTER
        .iter()
        .map(|k| {
            edits
                .iter()
                .filter(|(n, _)| *n == k.name)
                .fold(*k, |k, (_, f)| f(k))
        })
        .collect()
}

/// A register edit: what it is, and each entry's change.
type Case = (&'static str, Vec<EntryEdit>);

fn plus_one(k: ConstantBuilder) -> ConstantBuilder {
    ConstantBuilder {
        value: Some(Value::Exact(k.number() + 1.0)),
        ..k
    }
}

fn minus_one(k: ConstantBuilder) -> ConstantBuilder {
    ConstantBuilder {
        value: Some(Value::Exact(k.number() - 1.0)),
        ..k
    }
}

fn as_threshold(k: ConstantBuilder) -> ConstantBuilder {
    ConstantBuilder {
        value: Some(Value::Threshold(k.number())),
        ..k
    }
}

fn as_conservation(k: ConstantBuilder) -> ConstantBuilder {
    ConstantBuilder {
        class: Some(Admissibility::ConservationLaw),
        ..k
    }
}

fn recited(k: ConstantBuilder) -> ConstantBuilder {
    ConstantBuilder {
        citation: Some(Citation::Calibration("REQ-QA-000")),
        ..k
    }
}

/// Each floor entry's value up and down, its value's type and its class, one at a time.
fn floor_cases() -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();
    for name in ["floor_exp_min", "floor_exp_max", "floor_code_bias"] {
        cases.push(("value + 1", vec![(name, plus_one)]));
        cases.push(("value − 1", vec![(name, minus_one)]));
        cases.push(("value's type", vec![(name, as_threshold)]));
        cases.push(("class", vec![(name, as_conservation)]));
    }
    // Code review C5's scenario: the greatest code `floor_exp_max + floor_code_bias` stays 145, so the stored range
    // does not move, but every stored code means a floor twice as large.
    cases.push((
        "bias 42 with floor_exp_max 103",
        vec![("floor_code_bias", plus_one), ("floor_exp_max", minus_one)],
    ));
    // The least code stays 1 and the greatest 145, yet the codes move by one.
    cases.push((
        "bias 40 with floor_exp_min −39",
        vec![("floor_code_bias", minus_one), ("floor_exp_min", plus_one)],
    ));
    cases
}

/// Each case in `cases` changes the schema version; a citation-only edit of a floor entry does not (R-251).
fn check_floor_constants_hashed(cases: &[Case]) {
    let base = version_with(REGISTER);
    for (what, edits) in cases {
        let names: Vec<&str> = edits.iter().map(|(n, _)| *n).collect();
        assert_ne!(
            version_with(&edited(edits)),
            base,
            "{names:?}: {what} changed and the schema version did not (floor hash)"
        );
    }
    for name in ["floor_exp_min", "floor_exp_max", "floor_code_bias"] {
        assert_eq!(
            version_with(&edited(&[(name, recited)])),
            base,
            "`{name}`'s citation changed the schema version (citation)"
        );
    }
}

#[test]
fn qa_coll_floor_constants_move_the_version() {
    check_floor_constants_hashed(&floor_cases());
}

negative_control!(
    qa_coll_floor_constants_move_the_version,
    "an edit to an unhashed register entry, the log floor of d_min, does not move the version",
    expected = "(floor hash)",
    check_floor_constants_hashed(&[("value + 1", vec![("log_floor_d_min", plus_one)])])
);
