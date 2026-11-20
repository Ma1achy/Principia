//! QA tests for TASK-M3-11 on the ledger side, written from the requirements, not from the implementation:
//! - REQ-VAL-055 (R-37, R-545): the ledger row for the departed bit is `packed_a` bit 15, width 1, outside the reserved
//!   span, and adding it moved the schema version (the ledger without it, bits 10–15 reserved again, hashes
//!   differently); `δ_dep` is the register's measured threshold, provisional until M3, citing REQ-VAL-055, strictly
//!   inside its recorded gap, whose populations are the ones `fixtures/gates/delta-dep/REPORT.md` records
//!   (`qa_m311_departed_*`, `qa_m311_delta_dep_*`).
//! - REQ-INT-087 (R-397): `theta_ref_steps` and `theta_ref_none` are register entries of value 65535, class
//!   achievable-maximum, citing R-397, and each, singly, moves the schema version when its value changes; the control
//!   puts an entry the corpus does not hash (`r_pole`, dd_generation_root §3.8 "The hash"; R-433) through the same
//!   check (`qa_m311_theta_ref_*`).
//!
//! Each test has its registered negative control (R-176).
// The file name `qa_TASK-M3-11` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use ledger::constants::{Admissibility, Citation, ConstantBuilder, RelativeBasis, Value, REGISTER};
use ledger::gen;
use ledger::layout;
use ledger::payload;
use ledger::schema::{Ledger, Location, Span};
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

/// The schema version of `ledger` under `register`.
fn version(ledger: &Ledger, register: &[ConstantBuilder]) -> u64 {
    let entries = gen::validate(ledger).expect("the ledger validates");
    let structs = payload::structs();
    let quad = payload::QUAD_REDUCTION.to_vec();
    schema_version(&Hashed {
        quad_reduction: &quad,
        register,
        ..Hashed::payload(&ledger.words, &entries, &structs)
    })
    .expect("the schema version")
}

fn location(ledger: &Ledger, name: &str) -> Option<Location> {
    ledger
        .entries
        .iter()
        .find(|e| e.name == Some(name))
        .and_then(|e| e.location.clone())
}

// ── REQ-VAL-055: the departed row ───────────────────────────────────────────────────────────────────────────────

/// `departed` at `packed_a` bit 15, width 1 (R-545), and no reserved span of `packed_a` covering bit 15.
fn check_departed_row(ledger: &Ledger) {
    assert_eq!(
        location(ledger, "departed"),
        Some(Location::Packed {
            word: "packed_a",
            offset: 15,
            width: 1
        }),
        "the departed row is not packed_a bit 15 (row)"
    );
    let a = ledger
        .words
        .iter()
        .find(|w| w.name == "packed_a")
        .expect("packed_a");
    for s in &a.reserved {
        assert!(
            !(s.offset <= 15 && 15 < s.offset + s.width),
            "bit 15 is still reserved ({}, {}) (row)",
            s.offset,
            s.width
        );
    }
}

#[test]
fn qa_m311_departed_row() {
    check_departed_row(&layout());
}

negative_control!(
    qa_m311_departed_row,
    "the departed bit placed at bit 14 is not R-545's",
    expected = "(row)",
    check_departed_row(&{
        let mut l = layout();
        for e in l.entries.iter_mut().filter(|e| e.name == Some("departed")) {
            e.location = Some(Location::Packed {
                word: "packed_a",
                offset: 14,
                width: 1,
            });
        }
        l
    })
);

/// `ledger` without the departed row, `packed_a`'s reserved span bits 10–15 again (payload §2 before R-545).
fn without_departed(mut l: Ledger) -> Ledger {
    l.entries.retain(|e| e.name != Some("departed"));
    for w in l.words.iter_mut().filter(|w| w.name == "packed_a") {
        w.reserved = vec![Span {
            offset: 10,
            width: 6,
        }];
    }
    l
}

/// The departed row moves the schema version: today's ledger and `strip` of it hash differently.
fn check_departed_version(strip: fn(Ledger) -> Ledger) {
    let today = version(&layout(), REGISTER);
    let before = version(&strip(layout()), REGISTER);
    assert_ne!(
        today, before,
        "the departed row did not change the schema version (version)"
    );
}

#[test]
fn qa_m311_departed_moves_the_version() {
    check_departed_version(without_departed);
}

negative_control!(
    qa_m311_departed_moves_the_version,
    "a strip that removes nothing leaves the version where it is",
    expected = "(version)",
    check_departed_version(|l| l)
);

// ── REQ-VAL-055: δ_dep ───────────────────────────────────────────────────────────────────────────────────────────

fn entry(name: &str) -> ConstantBuilder {
    *REGISTER
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("`{name}` is not in the register"))
}

/// The report's sentence `<what>: <count> values in [<lo>, <hi>]`, wherever it stands in its paragraph.
fn report_population(report: &str, what: &str) -> (u64, f64, f64) {
    let key = format!("{what}: ");
    let at = report
        .match_indices(&key)
        .map(|(i, _)| i)
        .find(|&i| report[i + key.len()..].starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or_else(|| panic!("REPORT.md has no `{what}: <count>` sentence"));
    let rest = &report[at + key.len()..];
    let (count, rest) = rest.split_once(" values in [").expect("count");
    let (inside, _) = rest.split_once(']').expect("the closing bracket");
    let (lo, hi) = inside.split_once(", ").expect("lo, hi");
    (
        count.trim().parse().expect("count"),
        lo.trim().parse().expect("lo"),
        hi.trim().parse().expect("hi"),
    )
}

/// `δ_dep` (`k`): a threshold set from a measured gap (pitfalls §3), strictly inside it, provisional until M3 and
/// citing its calibration requirement REQ-VAL-055 (R-37, R-71, R-427); the gap's populations are the recorded
/// measurement's (`fixtures/gates/delta-dep/REPORT.md`, rounded as it prints them).
fn check_delta_dep(k: ConstantBuilder) {
    let v = match k.value {
        Some(Value::Threshold(v)) => v,
        other => panic!("delta_dep's value {other:?} is not a threshold"),
    };
    assert_eq!(
        k.provisional,
        Some("M3"),
        "delta_dep is not provisional until M3"
    );
    assert_eq!(
        k.citation,
        Some(Citation::Calibration("REQ-VAL-055")),
        "delta_dep's citation"
    );
    let Some(RelativeBasis::Gap { below, above, .. }) = k.relative_basis else {
        panic!("delta_dep has no measured gap");
    };
    assert!(
        below.hi < v && v < above.lo,
        "δ_dep {v} is not strictly inside the gap ({}, {}) (gap)",
        below.hi,
        above.lo
    );
    let path = crate::repo_root().join("fixtures/gates/delta-dep/REPORT.md");
    let report = std::fs::read_to_string(path).expect("the delta-dep report");
    for (what, p) in [("Below", below), ("Above", above)] {
        let (count, lo, hi) = report_population(&report, what);
        assert_eq!(
            count, p.count,
            "{what}: the register's count against the report's"
        );
        let close = |a: f64, b: f64| (a - b).abs() <= 5e-4 * b.abs();
        assert!(
            close(p.lo, lo) && close(p.hi, hi),
            "{what}: the register's [{}, {}] against the report's [{lo}, {hi}]",
            p.lo,
            p.hi
        );
    }
}

#[test]
fn qa_m311_delta_dep_in_measured_gap() {
    check_delta_dep(entry("delta_dep"));
}

negative_control!(
    qa_m311_delta_dep_in_measured_gap,
    "δ_dep at the gap's upper edge is not strictly inside it",
    expected = "(gap)",
    check_delta_dep({
        let k = entry("delta_dep");
        let Some(RelativeBasis::Gap { above, .. }) = k.relative_basis else {
            panic!("no gap");
        };
        ConstantBuilder {
            value: Some(Value::Threshold(above.lo)),
            ..k
        }
    })
);

// ── REQ-INT-087: the pole-reference entries ─────────────────────────────────────────────────────────────────────

/// `name`'s value changed by one moves the schema version.
fn check_moves_version(name: &str) {
    let base = version(&layout(), REGISTER);
    let k = entry(name);
    let v = match k.value {
        Some(Value::Exact(v)) | Some(Value::Threshold(v)) => v,
        other => panic!("`{name}` has no number: {other:?}"),
    };
    let edited: Vec<ConstantBuilder> = REGISTER
        .iter()
        .map(|&e| {
            if e.name == name {
                ConstantBuilder {
                    value: Some(match e.value {
                        Some(Value::Threshold(_)) => Value::Threshold(v * 0.5),
                        _ => Value::Exact(v - 1.0),
                    }),
                    ..e
                }
            } else {
                e
            }
        })
        .collect();
    assert_ne!(
        version(&layout(), &edited),
        base,
        "changing `{name}`'s value did not change the schema version (hashed)"
    );
}

/// The two entries: in the register at 65535, an achievable maximum citing R-397, and each moves the version alone.
fn check_theta_ref(names: &[&str]) {
    for &name in names {
        let k = entry(name);
        assert_eq!(k.value, Some(Value::Exact(65535.0)), "`{name}`'s value");
        assert_eq!(
            k.class,
            Some(Admissibility::AchievableMaximum),
            "`{name}`'s class"
        );
        match k.citation {
            Some(Citation::Corpus { file, section }) => assert!(
                file == "docs/history/decisions.md" && section.starts_with("R-397 "),
                "`{name}` cites {file} § {section}"
            ),
            other => panic!("`{name}` cites {other:?}"),
        }
        check_moves_version(name);
    }
}

#[test]
fn qa_m311_theta_ref_entries_each_move_the_version() {
    check_theta_ref(&["theta_ref_steps", "theta_ref_none"]);
}

negative_control!(
    qa_m311_theta_ref_entries_each_move_the_version,
    "r_pole, which §3.8 does not hash, fails the version check: the check tells hashed from unhashed",
    expected = "(hashed)",
    check_moves_version("r_pole")
);
