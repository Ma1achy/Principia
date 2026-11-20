//! QA fixture (TASK-M0-21): a crate that declares the `controls` feature and has a binary target and no library, so
//! it has no doctests to list. Its unit test `tests::doubles` has its control in `tests/controls.rs` (R-201).

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    println!("{}", double(3));
}

#[cfg(test)]
mod tests {
    #[test]
    fn doubles() {
        assert_eq!(super::double(3), 6);
    }
}
