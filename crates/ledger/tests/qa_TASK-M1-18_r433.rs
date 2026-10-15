//! QA tests for the M1 gate's other calibrations as R-433 enters them in the constants register, written from
//! REQ-INT-086, REQ-VAL-122 and dd_generation_root §3.8 (the provisional table's `r_pole` row, the table of the entry
//! R-433 confirms, "The hash"), not from the implementation:
//! - REQ-INT-086: `r_pole` is a register entry, 0.01, canonical units (a radius on the unit shape sphere), marked
//!   provisional until M3 and citing REQ-INT-086 (`r433_qa_r_pole_*`);
//! - REQ-VAL-122: `n_norm_tol` is a register entry, 64 U = 2⁻¹⁸ with U = 2⁻²⁴ the f32 unit roundoff, canonical units,
//!   unmarked, citing R-433's heading in `decisions.md` (`r433_qa_n_norm_tol_*`);
//! - §3.8 "The hash": neither is hashed into the schema version, so the register with both removed, or with either's
//!   value changed, has the same version (`r433_qa_neither_is_hashed`).
//!
//! Each test has a registered negative control (R-176).

// The file name `qa_TASK-M1-18_r433` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

/// The f32 unit roundoff, 2⁻²⁴ (REQ-VAL-122's U).
const U: f64 = 1.0 / 16_777_216.0;

/// The register entry `name` of `register`.
fn entry(register: &[ConstantBuilder], name: &str) -> ConstantBuilder {
    *register
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("the register holds no `{name}`"))
}

// ── REQ-INT-086: r_pole ──────────────────────────────────────────────────────────────────────────────────────────

/// `k` is REQ-INT-086's entry as R-433 makes it: 0.01, canonical units, provisional until M3, citing REQ-INT-086.
fn check_r_pole(k: ConstantBuilder) {
    assert_eq!(k.value, Some(Value::Exact(0.01)), "`r_pole`'s value");
    assert_eq!(
        k.class,
        Some(Admissibility::CanonicalUnits),
        "`r_pole`'s class"
    );
    assert_eq!(
        k.provisional,
        Some("M3"),
        "`r_pole` is not provisional until M3"
    );
    assert_eq!(
        k.citation,
        Some(Citation::Calibration("REQ-INT-086")),
        "`r_pole` does not cite REQ-INT-086"
    );
    k.build()
        .unwrap_or_else(|e| panic!("`r_pole` does not build: {e}"));
}

#[test]
fn r433_qa_r_pole_is_the_entry_provisional_until_m3() {
    check_r_pole(entry(REGISTER, "r_pole"));
}

negative_control!(
    r433_qa_r_pole_is_the_entry_provisional_until_m3,
    "r_pole confirmed outright at M1, its mark removed, is not provisional until M3",
    expected = "`r_pole` is not provisional until M3",
    check_r_pole(ConstantBuilder {
        provisional: None,
        ..entry(REGISTER, "r_pole")
    })
);

#[cfg(feature = "controls")]
mod r433_qa_r_pole_value {
    use super::*;
    negative_control!(
        r433_qa_r_pole_is_the_entry_provisional_until_m3,
        "a pole radius of 2e-3, the parity allowance's edge, is not R-433's 0.01",
        expected = "`r_pole`'s value",
        check_r_pole(ConstantBuilder {
            value: Some(Value::Exact(2e-3)),
            ..entry(REGISTER, "r_pole")
        })
    );
}

// ── REQ-VAL-122: n_norm_tol ──────────────────────────────────────────────────────────────────────────────────────

/// `k` is REQ-VAL-122's entry as R-433 makes it: 64 U = 2⁻¹⁸ exactly, canonical units, unmarked, citing a heading of
/// `decisions.md` that is R-433's.
fn check_n_norm_tol(k: ConstantBuilder) {
    assert_eq!(64.0 * U, 2f64.powi(-18), "64 U is 2⁻¹⁸");
    assert_eq!(
        k.value,
        Some(Value::Exact(64.0 * U)),
        "`n_norm_tol`'s value"
    );
    assert_eq!(
        k.class,
        Some(Admissibility::CanonicalUnits),
        "`n_norm_tol`'s class"
    );
    assert_eq!(k.provisional, None, "`n_norm_tol` is still provisional");
    let Some(Citation::Corpus {
        file: "decisions.md",
        section,
    }) = k.citation
    else {
        panic!(
            "`n_norm_tol` does not cite R-433 in decisions.md: {:?}",
            k.citation
        )
    };
    let decisions = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decisions.md"),
    )
    .expect("read decisions.md");
    assert!(
        section.starts_with("R-433 ") && decisions.lines().any(|l| l == format!("## {section}")),
        "`n_norm_tol` does not cite R-433's heading in decisions.md: {section}"
    );
    k.build()
        .unwrap_or_else(|e| panic!("`n_norm_tol` does not build: {e}"));
}

#[test]
fn r433_qa_n_norm_tol_is_64_u_confirmed_by_r433() {
    check_n_norm_tol(entry(REGISTER, "n_norm_tol"));
    assert_eq!(
        ledger::constants::N_NORM_TOL.number(),
        64.0 * U,
        "the tolerance code reads is not 64 U"
    );
}

negative_control!(
    r433_qa_n_norm_tol_is_64_u_confirmed_by_r433,
    "a tolerance of 128 U is not R-433's 64 U",
    expected = "`n_norm_tol`'s value",
    check_n_norm_tol(ConstantBuilder {
        value: Some(Value::Exact(128.0 * U)),
        ..entry(REGISTER, "n_norm_tol")
    })
);

#[cfg(feature = "controls")]
mod r433_qa_n_norm_tol_citation {
    use super::*;
    negative_control!(
        r433_qa_n_norm_tol_is_64_u_confirmed_by_r433,
        "an entry citing R-428's heading, the first M1 ruling, does not cite R-433",
        expected = "does not cite R-433's heading",
        check_n_norm_tol(ConstantBuilder {
            citation: entry(REGISTER, "hatch_violet").citation,
            ..entry(REGISTER, "n_norm_tol")
        })
    );
}

// ── §3.8 "The hash": neither entry is hashed ─────────────────────────────────────────────────────────────────────

fn version_with(register: &[ConstantBuilder]) -> u64 {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let structs = ledger::payload::structs();
    let h = Hashed {
        register,
        ..Hashed::payload(&l.words, &entries, &structs)
    };
    schema_version(&h).unwrap_or_else(|e| panic!("{e}"))
}

/// `register` with the entry `name` given the value `v`.
fn with_value(register: &[ConstantBuilder], name: &str, v: f64) -> Vec<ConstantBuilder> {
    let mut r = register.to_vec();
    let k = r
        .iter_mut()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("the register holds no `{name}`"));
    k.value = Some(Value::Exact(v));
    r
}

/// The register's version is unchanged by changing the value of each of `names`.
fn check_values_unhashed(names: &[&str]) {
    let base = version_with(REGISTER);
    for name in names {
        let v = entry(REGISTER, name).number();
        assert_eq!(
            version_with(&with_value(REGISTER, name, 2.0 * v + 1.0)),
            base,
            "changing `{name}`'s value changed the version"
        );
    }
}

/// The register's version is unchanged by removing all of `names`, and by changing each one's value.
fn check_unhashed(names: &[&str]) {
    let without: Vec<ConstantBuilder> = REGISTER
        .iter()
        .filter(|k| !names.contains(&k.name))
        .copied()
        .collect();
    assert_eq!(
        version_with(&without),
        version_with(REGISTER),
        "removing {names:?} changed the version"
    );
    check_values_unhashed(names);
}

#[test]
fn r433_qa_neither_is_hashed() {
    check_unhashed(&["r_pole", "n_norm_tol"]);
}

negative_control!(
    r433_qa_neither_is_hashed,
    "f16_finite_max decides stored bits and is hashed, so changing its value changes the version",
    expected = "changed the version",
    check_values_unhashed(&["f16_finite_max"])
);
