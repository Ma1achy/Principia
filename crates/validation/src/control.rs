//! The negative-control registry (REQ-VAL-147): every test registers the control that must make it fail, so no test
//! in the suite is one that cannot fail (philosophy §4.4; pitfalls §9). A crate reaches the macro through a
//! dev-dependency on `validation` (R-176).
//!
//! ```ignore
//! fn check_double(x: u32, doubled: u32) {
//!     assert_eq!(x * 2, doubled);
//! }
//!
//! #[test]
//! fn doubles() {
//!     check_double(3, 6);
//! }
//!
//! validation::negative_control!(doubles, "a wrong double must fail the check", check_double(3, 7));
//! ```
//!
//! The first argument names the test (R-199: no test-name attribute and no proc-macro crate). The control is an
//! expression of type `()` that runs the test's check against the control input, and panics when the check rejects
//! it. It compiles only under the calling crate's `controls` feature, as the test `<test_name>::negative_control`;
//! that test passes only if the control panics. `cargo xtask controls` runs `cargo test --features controls` in each
//! crate that declares the feature, pairs each test with the control named for it, across all of the crate's test
//! targets (R-201), and fails naming the test when its control does not panic or it has none.

/// Registers `control` as the negative control of the test `test_name`; see the [module docs](crate::control).
#[macro_export]
macro_rules! negative_control {
    ($test_name:ident, $description:literal, $control:expr $(,)?) => {
        #[cfg(feature = "controls")]
        #[doc = $description]
        mod $test_name {
            use super::*;

            #[test]
            #[should_panic]
            fn negative_control() {
                $control
            }
        }
    };
}
