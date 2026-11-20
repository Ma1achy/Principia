//! Fixture: one test with a control and one without; only the one without fails the command (REQ-VAL-147).

pub fn double(x: u32) -> u32 {
    x * 2
}
