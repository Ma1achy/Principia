//! The per-PR mutation gate's fixture (TASK-M0-23, REQ-VAL-148): `change.diff` adds the negative branch of [`sign`],
//! and only the test compiled under the `kill` feature reaches it. Without that test the branch's mutants survive
//! `cargo mutants --in-diff change.diff`; with it, none does.

/// The sign of `x`: 1, -1 or 0.
pub fn sign(x: i32) -> i32 {
    if x > 0 {
        1
    } else if x < 0 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::sign;

    #[test]
    fn sign_of_positive_is_one() {
        assert_eq!(sign(3), 1);
    }

    #[test]
    fn sign_of_zero_is_zero() {
        assert_eq!(sign(0), 0);
    }

    /// The killing test: the one test that reaches the branch `change.diff` adds.
    #[cfg(feature = "kill")]
    #[test]
    fn sign_of_negative_is_minus_one() {
        assert_eq!(sign(-3), -1);
    }
}
