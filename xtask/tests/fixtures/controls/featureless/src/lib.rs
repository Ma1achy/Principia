//! Fixture: a crate without the `controls` feature, skipped and reported, not failed (R-176).

pub fn double(x: u32) -> u32 {
    x * 2
}
