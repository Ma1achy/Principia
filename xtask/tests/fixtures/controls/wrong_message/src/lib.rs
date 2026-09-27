//! Fixture: two tests with the same check (R-212). `doubles`'s control trips the check, with its expected message;
//! `doubles_again`'s control panics in its own setup, before the check runs, so it must not count as failing the test.

pub fn double(x: u32) -> u32 {
    x * 2
}
