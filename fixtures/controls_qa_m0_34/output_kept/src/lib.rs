//! QA fixture (TASK-M0-34, REQ-VAL-167): three tests whose controls do not make them fail, each control writing a
//! marker of its own to its output, so `cargo xtask controls`'s finding for each test can be checked for it.
//!
//! - `tests/leaky.rs`: the control of `leaks` prints its marker and leaves the check passing (`ControlPasses`).
//! - `tests/wrong.rs`: the control of `misfires` panics with its marker before the check (`WrongPanic`, R-212).
//! - `tests/nested.rs`: the control of `nests` prints a line `failures:` (as a control embedding a child's libtest
//!   report would), then its marker, and leaves the check passing: the output after that line is its own too.

pub fn double(x: u32) -> u32 {
    x * 2
}

/// The shared check: `double(3)` is 6.
pub fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6, "not the double of 3");
}
