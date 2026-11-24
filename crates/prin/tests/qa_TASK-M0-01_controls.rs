//! Negative controls for the `prin --help` tests: each runs its test's check, from the support module its test calls
//! too, on the other argument, which the check must reject, so the test can fail.
#![cfg(feature = "controls")]

#[path = "support/qa_m0_01.rs"]
mod qa_m0_01;

use qa_m0_01::*;
use validation::negative_control;

negative_control!(
    qa_prin_help_succeeds_and_prints_usage,
    "an unknown flag fails, so the --help check must fail on it",
    expected = "prin --help fails",
    check_help_succeeds(&prin(&["--qa-no-such-flag"]))
);

negative_control!(
    qa_prin_refuses_an_unknown_argument,
    "--help succeeds, so the refusal check must fail on it",
    expected = "prin accepts an unknown flag",
    check_refused(&prin(&["--help"]))
);
