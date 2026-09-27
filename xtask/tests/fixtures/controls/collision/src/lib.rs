//! Fixture: two test targets each hold a test `doubles`, and only one registers a control named `doubles`. Each
//! test needs its own control, so the command fails naming `doubles` (REQ-VAL-147; applied per R-204).

pub fn double(x: u32) -> u32 {
    x * 2
}

/// The check both tests run: `double(3)` is 6.
pub fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6);
}
