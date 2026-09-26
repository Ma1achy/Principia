//! QA fixture (TASK-M0-21, REQ-VAL-147): pairing is by the name in `negative_control!`. `tests/misnamed.rs` registers
//! a discriminating control under the name `double`, which is not the test's name `doubles`; `tests/named.rs`
//! registers the same control under `doubles`. Without `named.rs` the test has no control.

pub fn double(x: u32) -> u32 {
    x * 2
}

/// The test's check: `double(3)` is 6.
pub fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6);
}
