//! The negative-control registry (REQ-VAL-147): every test registers the control that must make it fail, so no test
//! in the suite is one that cannot fail (philosophy §4.4; pitfalls §9). A crate reaches the macro through a
//! dev-dependency on `validation` (R-176).
//!
//! ```text
//! fn check_double(x: u32, doubled: u32) {
//!     assert_eq!(x * 2, doubled, "{doubled} is not the double of {x}");
//! }
//!
//! #[test]
//! fn doubles() {
//!     check_double(3, 6);
//! }
//!
//! validation::negative_control!(
//!     doubles,
//!     "a wrong double must fail the check",
//!     expected = "is not the double of",
//!     check_double(3, 7)
//! );
//! ```
//!
//! The first argument names the test (R-199: no test-name attribute and no proc-macro crate). `expected` is a
//! substring of the panic message of the assertion the control must trip (R-212). The control is an expression of
//! type `()` that runs the test's check against the control input, and panics when the check rejects it. It compiles
//! only under the calling crate's `controls` feature, as the test `<test_name>::negative_control` with
//! `#[should_panic(expected = …)]`; that test passes only if the control panics with the expected message, so a
//! control that panics in its own setup, before its check, fails. `cargo xtask controls` runs
//! `cargo test --features controls` in each crate that declares the feature, pairs each test with the control named
//! for it, across all of the crate's test targets (R-201), and fails naming the test when its control does not panic
//! with its expected message or it has none.
//!
//! A control's scratch (R-359): while a control runs, each scratch guard dropped on its thread hands its path to
//! [`defer`]. The control catches its own panic, compares the message with `expected`, deletes the scratch only on a
//! match, keeps it and prints its path otherwise (R-342), then resumes the panic, so libtest's verdict is unchanged.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// What settles a scratch path, given whether its test failed: a scratch guard's own.
pub type Settle = fn(&Path, bool);

thread_local! {
    /// The scratch the running control's guards have handed over; `None` outside a control.
    static DEFERRED: RefCell<Option<Vec<(PathBuf, Settle)>>> = const { RefCell::new(None) };
}

/// Hands `path` to the control running on this thread, which settles it with `settle` once it has compared its panic
/// message. Returns `path` when no control is running, for the guard to settle itself.
pub fn defer(path: PathBuf, settle: Settle) -> Option<PathBuf> {
    DEFERRED.with_borrow_mut(|deferred| match deferred {
        Some(paths) => {
            paths.push((path, settle));
            None
        }
        None => Some(path),
    })
}

/// Runs a control's body for [`negative_control!`](crate::negative_control) (R-359): settles the scratch its guards
/// handed over as passed only when it panicked with a message containing `expected`, as failed otherwise, then
/// resumes the panic.
#[doc(hidden)]
pub fn run(expected: &str, control: impl FnOnce()) {
    let outer = DEFERRED.replace(Some(Vec::new()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(control));
    let deferred = DEFERRED.replace(outer).unwrap_or_default();
    let matched = outcome.as_ref().err().is_some_and(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
        message.is_some_and(|m| m.contains(expected))
    });
    for (path, settle) in deferred {
        settle(&path, !matched);
    }
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// Registers `control` as the negative control of the test `test_name`; see the [module docs](crate::control).
#[macro_export]
macro_rules! negative_control {
    ($test_name:ident, $description:literal, expected = $expected:literal, $control:expr $(,)?) => {
        #[cfg(feature = "controls")]
        #[doc = $description]
        mod $test_name {
            use super::*;

            #[test]
            #[should_panic(expected = $expected)]
            fn negative_control() {
                $crate::control::run($expected, || $control)
            }
        }
    };
}
