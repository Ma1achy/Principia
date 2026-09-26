//! QA fixture (TASK-M0-21, REQ-VAL-147): a member of a three-crate workspace; see the workspace's Cargo.toml.

pub fn double(x: u32) -> u32 {
    x * 2
}

/// The test's check: `double(3)` is 6.
pub fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6);
}
