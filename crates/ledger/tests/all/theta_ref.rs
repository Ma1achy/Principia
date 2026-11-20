//! `θ̃`'s frozen pole reference in the register (R-397; REQ-INT-087; payload §2, `_reserved`):
//! - `theta_ref_register`: `theta_ref_steps` and `theta_ref_none` are register entries, each 65535, an achievable
//!   maximum citing R-397, among the hashed stored-bits entries (dd_generation_root §3.8, "The hash"); changing either's
//!   value changes the schema version, and removing either is refused.
//!
//! Each test registers its negative control (R-176).

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
use ledger::gen;
use ledger::layout;
use ledger::payload;
use ledger::version::{schema_version, Hashed, STORED_BITS};
use validation::negative_control;

/// The schema version of the payload ledger under `register`.
fn version(register: &[ConstantBuilder]) -> Result<u64, String> {
    let ledger = layout();
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
    let structs = payload::structs();
    let quad = payload::QUAD_REDUCTION.to_vec();
    schema_version(&Hashed {
        quad_reduction: &quad,
        register,
        ..Hashed::payload(&ledger.words, &entries, &structs)
    })
}

/// The register with `name`'s value replaced by `value`.
fn with_value(name: &str, value: f64) -> Vec<ConstantBuilder> {
    REGISTER
        .iter()
        .map(|&k| {
            if k.name == name {
                ConstantBuilder {
                    value: Some(Value::Exact(value)),
                    ..k
                }
            } else {
                k
            }
        })
        .collect()
}

/// Checks the entry `name`: in the register at 65535, an achievable maximum citing R-397, among `hashed`; a value of
/// 65534 changes the schema version; and the register without it is refused.
fn check_entry(name: &str, hashed: &[&str]) {
    let k = REGISTER
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("`{name}` is not in the register"));
    assert_eq!(k.value, Some(Value::Exact(65535.0)), "`{name}`'s value");
    assert_eq!(
        k.class,
        Some(Admissibility::AchievableMaximum),
        "`{name}`'s class"
    );
    match k.citation {
        Some(Citation::Corpus { file, section }) => assert!(
            file == "docs/history/decisions.md" && section.starts_with("R-397 "),
            "`{name}` cites {file} § {section}, not R-397"
        ),
        other => panic!("`{name}` cites {other:?}, not R-397"),
    }
    assert!(
        hashed.contains(&name),
        "`{name}` is not among the hashed stored-bits entries"
    );
    let base = version(REGISTER).expect("the base version");
    let edited = version(&with_value(name, 65534.0)).expect("the edited version");
    assert_ne!(
        edited, base,
        "changing `{name}`'s value did not change the schema version"
    );
    let without: Vec<ConstantBuilder> = REGISTER
        .iter()
        .copied()
        .filter(|k| k.name != name)
        .collect();
    assert!(
        version(&without).is_err(),
        "the register without `{name}` was not refused"
    );
}

#[test]
fn theta_ref_register() {
    for name in ["theta_ref_steps", "theta_ref_none"] {
        check_entry(name, &STORED_BITS);
    }
}

negative_control!(
    theta_ref_register,
    "an entry left out of the hashed list fails the check",
    expected = "is not among the hashed stored-bits entries",
    {
        let hashed: Vec<&str> = STORED_BITS
            .iter()
            .copied()
            .filter(|n| *n != "theta_ref_none")
            .collect();
        check_entry("theta_ref_none", &hashed);
    }
);
