//! The metadata gate (REQ-GEN-002; dd_generation_root §3.8, §5 test 6): an entry missing any required §3.8 key
//! makes generation fail, naming the field and the key; the optional keys are not required.

mod support;

use ledger::schema::{Ledger, REQUIRED_KEYS};
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
    check_deleting_refuses("saturated", "scale");
}

#[test]
fn metadata_gate_deleting_any_required_key_names_the_field() {
    for field in ["state", "t_end_step", "diffusion", "n", "t_end_fraction"] {
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
    check_generates_without("diffusion", &["sentinel", "tier_gate"]);
}

negative_control!(
    metadata_gate_deleting_scale_names_the_field,
    "deleting an optional key leaves the entry complete, so the gate check must fail on it",
    expected = "generation was not refused",
    check_deleting_refuses("saturated", "sentinel")
);

negative_control!(
    metadata_gate_deleting_any_required_key_names_the_field,
    "deleting nothing leaves generation allowed, so the gate check must fail on it",
    expected = "generation was not refused",
    check_deleting_refuses("t_end_fraction", "no such key")
);

negative_control!(
    metadata_gate_sentinel_and_tier_gate_are_optional,
    "an entry without its scale is refused, so the generates check must fail on it",
    expected = "generation refused",
    check_generates_without("diffusion", &["sentinel", "scale"])
);
