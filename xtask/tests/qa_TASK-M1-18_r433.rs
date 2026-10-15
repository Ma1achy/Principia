//! QA tests for `cargo xtask gate-provisional --milestone <Mn>` after R-433, written from REQ-INT-086's verify ("`cargo
//! xtask gate-provisional --milestone M3` lists it until the human re-confirms it at the M3 gate") and
//! dd_generation_root §3.8 (`provisional`: "its report lists each entry marked provisional until it, and an entry still
//! so marked blocks the gate"), not from the implementation: run as the binary, the M3 run lists `r_pole` with
//! REQ-INT-086 and exits non-zero; the M1 and M2 runs do not list it (R-433: "the M1 and M2 runs do not list it").
//!
//! Each test has a registered negative control (R-176).

// The file name `qa_TASK-M1-18_r433` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::process::Command;

use validation::negative_control;

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

/// The line of `text` naming `r_pole` as its first word, if any.
fn r_pole_line(text: &str) -> Option<&str> {
    text.lines()
        .find(|l| l.split_whitespace().next() == Some("r_pole"))
}

/// The run at `blocked` fails and lists `r_pole` with REQ-INT-086; each run of `clear` does not list it.
fn check_r_pole(blocked: &str, clear: &[&str]) {
    let (ok, text) = run(blocked);
    assert!(
        !ok,
        "the {blocked} run passed with `r_pole` provisional: {text}"
    );
    let line = r_pole_line(&text)
        .unwrap_or_else(|| panic!("the {blocked} run does not list `r_pole`: {text}"));
    assert!(
        line.contains("REQ-INT-086"),
        "the {blocked} run lists `r_pole` without REQ-INT-086: {line}"
    );
    for m in clear {
        let (_, text) = run(m);
        assert!(
            r_pole_line(&text).is_none(),
            "the {m} run lists `r_pole`, provisional until M3: {text}"
        );
    }
}

#[test]
fn qa_gate_provisional_m3_lists_r_pole() {
    check_r_pole("M3", &["M1", "M2"]);
}

negative_control!(
    qa_gate_provisional_m3_lists_r_pole,
    "the M2 run, read as the gate r_pole blocks, does not list it",
    expected = "the M2 run does not list `r_pole`",
    check_r_pole("M2", &["M1"])
);
