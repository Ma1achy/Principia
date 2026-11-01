//! QA's tests for TASK-M3-06's `tau` entry, written from REQ-EVT-024, REQ-EVT-025, R-173 and R-427, each with its
//! negative control (R-176). REQ-EVT-024: the provisional `tau` is `1e-3`, inside the recorded range `7.04e-05 …
//! 2.70e-02`, and its definition cites its source and marks it provisional. REQ-EVT-025: the confirmed `tau` is a
//! calibration confirmed at the M3 gate, replacing it. So the entry is marked until M3 and cites that requirement,
//! and the requirement it cites is in the plan, a calibration, at that milestone.

use ledger::constants::{gate, provisional_until, Citation, ConstantBuilder, Value, REGISTER};
use validation::negative_control;

/// The register's `tau`.
fn tau() -> ConstantBuilder {
    *REGISTER
        .iter()
        .find(|k| k.name == "tau")
        .expect("the register holds no `tau`")
}

/// `k` holds REQ-EVT-024's number, inside its recorded range.
fn check_value(k: &ConstantBuilder) {
    let Some(Value::Exact(v)) = k.value else {
        panic!("`tau` holds no exact number: {:?}", k.value)
    };
    assert_eq!(v, 1e-3, "`tau` is not REQ-EVT-024's provisional number");
    assert!(
        (7.04e-5..=2.70e-2).contains(&v),
        "`tau` {v} is outside the recorded range"
    );
}

#[test]
fn qa_tau_is_the_provisional_number() {
    check_value(&tau());
    assert_eq!(
        ledger::constants::TAU.value,
        tau().value,
        "`TAU` and the register's `tau`"
    );
}

negative_control!(
    qa_tau_is_the_provisional_number,
    "a `tau` of 1e-2, in the range but not the number R-173 rules, fails",
    expected = "not REQ-EVT-024's provisional number",
    check_value(&ConstantBuilder {
        value: Some(Value::Exact(1e-2)),
        ..tau()
    })
);

/// `k` is marked until M3, cites REQ-EVT-025, and so appears in the M3 gate's list of `register`, and passes the gate.
fn check_mark(k: &ConstantBuilder, register: &[ConstantBuilder]) {
    assert_eq!(
        k.provisional,
        Some("M3"),
        "`tau` is not provisional until M3"
    );
    assert_eq!(
        k.citation,
        Some(Citation::Calibration("REQ-EVT-025")),
        "`tau` does not cite REQ-EVT-025"
    );
    assert!(
        provisional_until(register, "M3").contains(&k.name),
        "the M3 gate does not list `tau`"
    );
    gate(&[*k]).unwrap_or_else(|e| panic!("the gate refuses `tau`: {e:?}"));
}

#[test]
fn qa_tau_waits_for_the_m3_gate() {
    check_mark(&tau(), REGISTER);
}

negative_control!(
    qa_tau_waits_for_the_m3_gate,
    "`tau` as it stood, settled and citing R-173's heading, is off the M3 gate's list",
    expected = "`tau` is not provisional until M3",
    check_mark(
        &ConstantBuilder {
            provisional: None,
            citation: Some(Citation::Corpus {
                file: "decisions.md",
                section: "R-173 — Tau is split into a provisional and a confirmed value *(closes C2, C3, S3, G6)*",
            }),
            ..tau()
        },
        REGISTER
    )
);

mod earlier_gate {
    use super::*;
    negative_control!(
        qa_tau_waits_for_the_m3_gate,
        "a `tau` marked until M2 would have been due a gate too early",
        expected = "`tau` is not provisional until M3",
        check_mark(
            &ConstantBuilder {
                provisional: Some("M2"),
                ..tau()
            },
            REGISTER
        )
    );
}

/// The block of `plan/requirements.yaml` that defines `id`.
fn requirement(plan: &str, id: &str) -> Option<String> {
    let head = format!("- id: {id}\n");
    let from = plan.find(&head)? + head.len();
    let rest = &plan[from..];
    let end = rest.find("\n- id: ").unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// `id` is a calibration requirement at milestone M3 in `plan`.
fn check_cited(plan: &str, id: &str) {
    let block = requirement(plan, id).unwrap_or_else(|| panic!("{id} is not in the plan"));
    assert!(
        block.contains("\n  kind: calibration") || block.starts_with("  kind: calibration"),
        "{id} is not a calibration"
    );
    assert!(block.contains("\n  milestone: M3"), "{id} is not at M3");
}

fn plan() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plan/requirements.yaml"),
    )
    .expect("read plan/requirements.yaml")
}

#[test]
fn qa_tau_cites_a_calibration_at_m3() {
    let Some(Citation::Calibration(id)) = tau().citation else {
        panic!(
            "`tau` cites no calibration requirement: {:?}",
            tau().citation
        )
    };
    check_cited(&plan(), id);
}

negative_control!(
    qa_tau_cites_a_calibration_at_m3,
    "REQ-EVT-024, the provisional value's own requirement, is not the calibration that confirms it",
    expected = "REQ-EVT-024 is not a calibration",
    check_cited(&plan(), "REQ-EVT-024")
);
