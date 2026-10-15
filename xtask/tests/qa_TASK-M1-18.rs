//! QA tests for TASK-M1-18's `cargo xtask gate-provisional --milestone <Mn>`, written from REQ-SYS-081 and
//! dd_generation_root §3.8 (`provisional`: "its report lists each entry marked provisional until it, and an entry
//! still so marked blocks the gate, as an unconfirmed calibration does"), not from the implementation:
//! - the command, run as the binary, lists each entry still provisional until the gate or earlier with its calibration
//!   requirement, and exits non-zero exactly when it lists one. After R-428 confirmed §3.8's nine M1 entries, seven
//!   outright and `log_floor_rho_ratio` and `log_floor_r_min_pair_0` provisionally, now marked until M2 citing
//!   REQ-TOOL-163, the M0 and M1 runs list nothing and exit zero, and the M2 run lists those two and `eps_m01`; a
//!   confirmed entry is listed at no gate;
//! - it refuses a name that is no milestone's gate, M9 included (`plan/MILESTONES.md` has M0 … M8), rather than
//!   reporting a gate that does not exist as passing or blocked.
//!
//! Each test has a registered negative control (R-176).

use std::process::Command;

use validation::negative_control;

/// §3.8's provisional entries as R-428 leaves them: the gate each is still provisional until, with its calibration
/// requirement, or `None` for an entry confirmed at M1 (citing R-428, so no requirement is listed for it).
const TABLE: [(&str, Option<u32>, &str); 10] = [
    ("log_floor_d_min", None, ""),
    ("log_floor_dE_max", None, ""),
    ("log_floor_dLz_max", None, ""),
    ("log_floor_closure_min", None, ""),
    ("log_floor_rho_ratio", Some(2), "REQ-TOOL-163"),
    ("log_floor_r_min_pair_0", Some(2), "REQ-TOOL-163"),
    ("hatch_violet", None, ""),
    ("hatch_cyan", None, ""),
    ("hatch_half_period", None, ""),
    ("eps_m01", Some(2), "REQ-DEC-039"),
];

/// Runs `cargo xtask gate-provisional --milestone <milestone>`: whether it succeeded, and its stdout and stderr.
fn run(milestone: &str) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["gate-provisional", "--milestone", milestone])
        .output()
        .expect("run xtask");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// The gate `n`'s run fails exactly when §3.8's table has an entry provisional until `n` or earlier, and names each
/// such entry on a line with its requirement, and no entry provisional until a later gate or confirmed.
fn check_gate(n: u32) {
    let (ok, text) = run(&format!("M{n}"));
    let due: Vec<_> = TABLE
        .iter()
        .filter(|t| t.1.is_some_and(|g| g <= n))
        .collect();
    assert_eq!(ok, due.is_empty(), "the M{n} run's exit: {text}");
    for (name, _, req) in TABLE {
        let line = text
            .lines()
            .find(|l| l.split_whitespace().next() == Some(name));
        if due.iter().any(|t| t.0 == name) {
            let line =
                line.unwrap_or_else(|| panic!("the M{n} run does not list `{name}`: {text}"));
            assert!(
                line.contains(req),
                "the M{n} run lists `{name}` without {req}: {line}"
            );
        } else {
            assert!(
                line.is_none(),
                "the M{n} run lists `{name}`, not yet due: {text}"
            );
        }
    }
}

#[test]
fn qa_gate_provisional_blocks_each_gate_while_an_entry_is_due() {
    check_gate(0);
    check_gate(1);
    check_gate(2);
}

negative_control!(
    qa_gate_provisional_blocks_each_gate_while_an_entry_is_due,
    "the M1 run read as M2's misses eps_m01, so the check must fail",
    expected = "does not list `eps_m01`",
    {
        let (_, text) = run("M1");
        assert!(
            text.lines().any(|l| l.starts_with("eps_m01")),
            "the M1 run does not list `eps_m01`: {text}"
        );
    }
);

/// `milestone` is refused: the run fails without listing any entry.
fn check_refused(milestone: &str) {
    let (ok, text) = run(milestone);
    assert!(!ok, "`{milestone}` is accepted as a gate: {text}");
    for (name, _, _) in TABLE {
        assert!(
            !text.lines().any(|l| l.starts_with(name)),
            "`{milestone}` is reported as a gate, listing `{name}`: {text}"
        );
    }
}

#[test]
fn qa_gate_provisional_refuses_a_gate_that_does_not_exist() {
    for g in ["M01", "m1", "M", "X1", "M9", "M10"] {
        check_refused(g);
    }
}

negative_control!(
    qa_gate_provisional_refuses_a_gate_that_does_not_exist,
    "M8 is the last milestone's gate, and lists every entry, so the refusal check must fail on it",
    expected = "is reported as a gate",
    check_refused("M8")
);
