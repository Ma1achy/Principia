//! The subprocess bodies qa's tests spawn (R-210): not `#[test]`s, so `cargo xtask controls` never lists them. The
//! first argument names the body; each prints the marker line its parent test reads. Moved from
//! `tests/qa_TASK-M0-04.rs` (`open_harness`, `failing_property`) and `tests/qa_TASK-M0-04_r2.rs` (`count_cases`).

use std::sync::atomic::{AtomicU32, Ordering};

use proptest::test_runner::{Config, TestCaseError, TestRunner};
use validation::gpu::GpuHarness;
use validation::prop;

/// Opens `GpuHarness::new()` and prints the outcome on one line.
fn open_harness() {
    match GpuHarness::new() {
        Ok(h) => println!("QA_OPEN_OK backend={:?}", h.adapter_info().backend),
        Err(e) => println!("QA_OPEN_ERR {e}"),
    }
}

/// A property made to fail through `prop::run`; prints the first failing draw, then fails with `prop::run`'s panic.
fn failing_property() {
    let first = std::sync::Mutex::new(None::<u32>);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prop::run(&proptest::prelude::any::<u32>(), |x| {
            if x % 7 == 3 {
                first.lock().unwrap().get_or_insert(x);
                return Err(TestCaseError::fail("x % 7 == 3"));
            }
            Ok(())
        })
    }));
    println!("QA_FIRST_DRAW {:?}", first.lock().unwrap());
    assert!(r.is_err(), "the property was made to fail");
    std::panic::resume_unwind(r.unwrap_err());
}

/// Counts the cases `prop::run` and a default proptest runner run, and prints both.
fn count_cases() {
    let shared = AtomicU32::new(0);
    prop::run(&proptest::prelude::any::<u64>(), |_| {
        shared.fetch_add(1, Ordering::Relaxed);
        Ok(())
    });
    let default = AtomicU32::new(0);
    TestRunner::new(Config::default())
        .run(&proptest::prelude::any::<u64>(), |_| {
            default.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
        .unwrap();
    println!(
        "QA_CASES shared={} config={} default={}",
        shared.load(Ordering::Relaxed),
        prop::config(1).cases,
        default.load(Ordering::Relaxed)
    );
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("open_harness") => open_harness(),
        Some("failing_property") => failing_property(),
        Some("count_cases") => count_cases(),
        other => panic!("qa_child: unknown body {other:?}"),
    }
}
