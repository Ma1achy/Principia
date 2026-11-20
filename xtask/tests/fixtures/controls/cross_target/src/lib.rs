//! Fixture: a unit test in `src/` whose control is registered in the crate's `tests/`, paired by name (R-201).

pub fn triple(x: u32) -> u32 {
    x * 3
}

/// The unit test's check, public so that the control in `tests/` can run it against the control input.
pub fn check_triple(triple: fn(u32) -> u32) {
    assert_eq!(triple(4), 12, "not the triple of 4");
}

#[cfg(test)]
mod tests {
    #[test]
    fn triples() {
        super::check_triple(super::triple);
    }
}
