//! QA tests for TASK-M0-09, round 2, written from REQ-PAY-002 ("generated Rust layouts compared field-by-field
//! against the ledger tables"), REQ-PAY-009 (`closure_step: u16`), REQ-PAY-010 (the accumulators and drift references
//! are f32) and dd_generation_root §3.8 ("A field without a complete entry fails generation loudly"): the generated
//! structs are tied to the ledger at generation time, so a ledger that disagrees with a struct member is refused,
//! naming the member, and nothing is written. Each test has a registered negative control (R-176).

use ledger::gen::{self, rust, Emitter};
use ledger::layout;
use ledger::schema::{Bound, FieldType, Ledger, Range};
use validation::negative_control;

/// The payload ledger with `name`'s entry changed by `edit`.
fn edited(name: &str, edit: impl FnOnce(&mut ledger::schema::EntryBuilder)) -> Ledger {
    let mut ledger = layout();
    let e = ledger
        .entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("the payload ledger has `{name}`"));
    edit(e);
    ledger
}

/// Generation from `ledger` with `emitters` is refused, and the refusal names each of `names`.
fn check_refused(ledger: &Ledger, emitters: &[Emitter], names: &[&str]) {
    let message = match gen::generate(ledger, emitters) {
        Ok(_) => panic!("generation was not refused"),
        Err(error) => error.to_string(),
    };
    for name in names {
        assert!(
            message.contains(name),
            "refused, but not naming `{name}`: {message}"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-010: `E_0` is f32 in both variants; a ledger typing it otherwise disagrees with the struct and is refused.
// The emitter is passed as a function pointer taken in this crate, not through `EMITTERS`.

fn check_e0_typed(ty: FieldType) {
    let ledger = edited("E_0", |e| e.ty = Some(ty));
    let own: [Emitter; 1] = [rust::emit];
    check_refused(&ledger, &own, &["`SimStateFTLE.E_0`", "`SimStateBase.E_0`"]);
}

#[test]
fn qa_payload_precision_generation_refuses_a_non_f32_drift_reference() {
    check_e0_typed(FieldType::UBits);
}

negative_control!(
    qa_payload_precision_generation_refuses_a_non_f32_drift_reference,
    "E_0 typed f32, as payload §1 stores it, agrees with both structs, so generation must not be refused",
    expected = "generation was not refused",
    check_e0_typed(FieldType::F32)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-009: `closure_step` is a u16; a ledger whose range for it exceeds u16 cannot be stored there and is refused.

fn check_closure_step_hi(hi: f64) {
    let ledger = edited("closure_step", |e| {
        e.range = Some(Range {
            lo: Bound::Closed(0.0),
            hi: Bound::Closed(hi),
        })
    });
    check_refused(
        &ledger,
        gen::EMITTERS,
        &["`SimStateFTLE.closure_step`", "`SimStateBase.closure_step`"],
    );
}

#[test]
fn qa_closure_fields_generation_refuses_a_closure_step_wider_than_u16() {
    check_closure_step_hi(f64::from(u16::MAX) + 1.0);
}

negative_control!(
    qa_closure_fields_generation_refuses_a_closure_step_wider_than_u16,
    "a closure_step range ending at u16::MAX fits the u16 member, so generation must not be refused",
    expected = "generation was not refused",
    check_closure_step_hi(f64::from(u16::MAX))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-002 through `cargo xtask codegen`'s path (`gen::run` with `EMITTERS`): a ledger missing a struct member's
// entry is refused, naming the member, and no generated file is written.

fn check_run_refuses_without(name: Option<&str>) {
    let mut ledger = layout();
    if let Some(name) = name {
        ledger.entries.retain(|e| e.name != Some(name));
    }
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("qa_m0_09_r2_run_{}", name.unwrap_or("none")));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create the scratch root");
    let result = gen::run(&ledger, gen::EMITTERS, &root);
    let written = root.join(rust::PATH).exists();
    let message = match result {
        Ok(_) => panic!("codegen was not refused (wrote generated.rs: {written})"),
        Err(message) => message,
    };
    assert!(
        !written,
        "codegen was refused but still wrote {}",
        rust::PATH
    );
    assert!(
        message.contains("`SimStateFTLE.theta`") && message.contains("`SimStateBase.theta`"),
        "refused, but not naming theta in both variants: {message}"
    );
}

#[test]
fn qa_payload_fields_codegen_refuses_a_member_with_no_entry_and_writes_nothing() {
    check_run_refuses_without(Some("theta"));
}

negative_control!(
    qa_payload_fields_codegen_refuses_a_member_with_no_entry_and_writes_nothing,
    "the whole payload ledger ties every member, so codegen must write generated.rs and not be refused",
    expected = "codegen was not refused (wrote generated.rs: true)",
    check_run_refuses_without(None)
);
