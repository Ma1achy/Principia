//! The metadata gate (REQ-GEN-002; dd_generation_root §3.8, §5 test 6): an entry missing any required §3.8 key
//! makes generation fail, naming the field and the key; the optional keys are not required.

mod support;

use ledger::schema::{FieldType, Ledger, REQUIRED_KEYS};
use support::{check_generates, check_refused_naming, entry, fixture};
use validation::negative_control;

/// The fixture with the §3.8 `keys` deleted from `field`'s entry.
fn deleting(field: &str, keys: &[&str]) -> Ledger {
    let mut ledger = fixture();
    let e = entry(&mut ledger, field);
    for key in keys {
        match *key {
            "sentinel" => e.sentinel = None,
            "tier_gate" => e.tier_gate = None,
            required => *e = e.clone().without(required),
        }
    }
    ledger
}

/// Generation from the fixture with `key` deleted from `field`'s entry is refused, naming the field and the key; an
/// entry without its name is named "entry #i (unnamed)" (R-242).
fn check_deleting_refuses(field: &str, key: &str) {
    let ledger = deleting(field, &[key]);
    let name = match key {
        "name" => {
            let i = fixture().entries.iter().position(|e| e.name == Some(field));
            format!("entry #{} (unnamed)", i.expect("the fixture has the entry"))
        }
        _ => field.to_owned(),
    };
    check_refused_naming(&ledger, &[&name, &format!("missing `{key}`")]);
}

#[test]
fn metadata_gate_deleting_scale_names_the_field() {
    check_deleting_refuses("fx_flag", "scale");
}

#[test]
fn metadata_gate_deleting_any_required_key_names_the_field() {
    for field in ["fx_enum", "fx_u16", "fx_scalar", "fx_vector", "fx_derived"] {
        for key in REQUIRED_KEYS {
            check_deleting_refuses(field, key);
        }
    }
}

/// The fixture generates with `keys` deleted from `field`'s entry.
fn check_generates_without(field: &str, keys: &[&str]) {
    check_generates(&deleting(field, keys));
}

#[test]
fn metadata_gate_sentinel_and_tier_gate_are_optional() {
    check_generates_without("fx_scalar", &["sentinel", "tier_gate"]);
}

negative_control!(
    metadata_gate_deleting_scale_names_the_field,
    "deleting an optional key leaves the entry complete, so the gate check must fail on it",
    expected = "generation was not refused",
    check_deleting_refuses("fx_flag", "sentinel")
);

negative_control!(
    metadata_gate_deleting_any_required_key_names_the_field,
    "deleting nothing leaves generation allowed, so the gate check must fail on it",
    expected = "generation was not refused",
    check_deleting_refuses("fx_derived", "no such key")
);

negative_control!(
    metadata_gate_sentinel_and_tier_gate_are_optional,
    "an entry without its scale is refused, so the generates check must fail on it",
    expected = "generation refused",
    check_generates_without("fx_scalar", &["sentinel", "scale"])
);

/// Generation from the fixture with `fx_vector`'s type set to `vector(component, k)` is refused, naming it (§3.8:
/// "`k ≥ 2` components, each of the scalar `type`").
fn check_vector_refused(component: FieldType, k: u32) {
    let mut ledger = fixture();
    entry(&mut ledger, "fx_vector").ty = Some(FieldType::Vector {
        component: Box::new(component),
        k,
    });
    check_refused_naming(&ledger, &["fx_vector"]);
}

#[test]
fn metadata_gate_vector_needs_two_or_more_scalar_components() {
    let f32x3 = FieldType::Vector {
        component: Box::new(FieldType::F32),
        k: 3,
    };
    check_vector_refused(f32x3, 2);
    check_vector_refused(FieldType::F32, 1);
    check_vector_refused(FieldType::F32, 0);
}

negative_control!(
    metadata_gate_vector_needs_two_or_more_scalar_components,
    "vector(f16-pair, 2) is a valid vector, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_vector_refused(FieldType::F16Pair, 2)
);
