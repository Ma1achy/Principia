//! `prin --help` succeeds and prints the usage; an unknown argument is refused.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01.rs"]
mod qa_m0_01;

use qa_m0_01::*;

#[test]
fn qa_prin_help_succeeds_and_prints_usage() {
    let out = prin(&["--help"]);
    check_help_succeeds(&out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.to_lowercase().contains("usage"),
        "prin --help prints no usage:\n{text}"
    );
}

#[test]
fn qa_prin_refuses_an_unknown_argument() {
    // Control: the binary distinguishes --help from anything else, so the test above can fail.
    let out = prin(&["--qa-no-such-flag"]);
    check_refused(&out);
}
