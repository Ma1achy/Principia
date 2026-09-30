//! QA fixture (TASK-M0-38, REQ-SYS-069, R-212): controls whose own output embeds a child's libtest report, with the
//! child's own `---- x stdout ----` header and the child's own wrong-panic note, so `cargo xtask controls` can be
//! checked to read each control's note from the control's own section only.
//!
//! - `tests/embeds.rs`: the control of `embeds` prints [`CHILD_REPORT`], then panics without its expected message
//!   (`WrongPanic`): its note is its own panic and expected substring, after the embedded header.
//! - `tests/decoy.rs`: the control of `decoy` prints [`CHILD_REPORT`] and leaves its test passing (`ControlPasses`):
//!   the only wrong-panic note in its section is the child's, which is not its own.
//! - `tests/plain.rs`: the control of `plain` panics without its expected message and embeds nothing (`WrongPanic`).

/// A child's `cargo test` report, as a control that runs a child prints it: the child's test `x` failed with the
/// wrong panic, and libtest's note for it names the child's messages, never the control's.
pub const CHILD_REPORT: &str = "running 1 test
test x - should panic ... FAILED

failures:

---- x stdout ----

thread 'x' panicked at src/lib.rs:1:1:
QA_M038_DECOY_PANIC
note: panic did not contain expected string
      panic message: \"QA_M038_DECOY_PANIC\"
 expected substring: \"QA_M038_DECOY_EXPECTED\"

failures:
    x

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";

pub fn double(x: u32) -> u32 {
    x * 2
}

/// The shared check: `double(3)` is 6.
pub fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6, "not the double of 3");
}
