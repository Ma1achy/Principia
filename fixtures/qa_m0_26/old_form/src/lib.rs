//! QA fixture, TASK-M0-26 (REQ-VAL-154): a test whose control is registered in R-199's form, without the expected
//! message R-212 adds. `negative_control!` requires the message, so this crate's tests must not compile. The
//! negative control of the fixture is the same file with `expected = "not the double of 3",` added, which compiles.

pub fn double(x: u32) -> u32 {
    x * 2
}
