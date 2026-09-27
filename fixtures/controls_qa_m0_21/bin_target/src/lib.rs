//! QA fixture (TASK-M0-21, REQ-VAL-147, R-201): a unit test in the crate's binary target (`src/main.rs`) whose
//! control is registered in the crate's `tests/`; pairing is across all of the crate's test targets.

pub fn quadruple(x: u32) -> u32 {
    x * 4
}

/// The binary's unit test's check: `quadruple(2)` is 8.
pub fn check_quadruple(quadruple: fn(u32) -> u32) {
    assert_eq!(quadruple(2), 8, "not the quadruple of 2");
}
