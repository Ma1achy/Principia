//! QA fixture (TASK-M0-21, REQ-VAL-147): two test targets each hold a test `doubles` and a control named `doubles`.
//! The control in `tests/a_leaky.rs` leaves its test passing, so the command must fail naming `doubles`, whatever
//! the other target's control does.

pub fn double(x: u32) -> u32 {
    x * 2
}

/// The shared check: `double(3)` is 6.
pub fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6, "not the double of 3");
}
