//! The subprocess bodies qa's tests spawn (R-210): not `#[test]`s, so `cargo xtask controls` never lists them. The
//! first argument names the body; each prints the marker line its parent test reads. Moved from
//! `tests/qa_TASK-M0-04.rs` (`open_harness`, `failing_property`), `tests/qa_TASK-M0-04_r2.rs` (`count_cases`) and
//! `tests/qa_R-206.rs` (`r206_opened`, R-213).

use std::sync::atomic::{AtomicU32, Ordering};

use proptest::test_runner::{Config, TestCaseError, TestRunner};
use validation::gpu::{backend_choice, GpuHarness, BACKEND_VAR};
use validation::prop;

/// Opens `GpuHarness::new()` and prints the outcome on one line.
fn open_harness() {
    match GpuHarness::new() {
        Ok(h) => println!("QA_OPEN_OK backend={:?}", h.adapter_info().backend),
        Err(e) => println!("QA_OPEN_ERR {e}"),
    }
}

/// Opens `GpuHarness::new()` and prints the backend it opened, then checks it is the one selected for this run's
/// `PRIN_GPU_BACKEND`, set or unset, and that the selection's log names it: `qa_r206_harness_opens_the_selected_backend`'s
/// child path. Printed before the check, so the parent sees what was opened even when the check fails.
fn r206_opened() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let opened = h.adapter_info().backend;
    println!("QA_R206_OPENED backend={opened:?}");
    let value = std::env::var(BACKEND_VAR).ok();
    let (want, log) = backend_choice(value.as_deref()).unwrap_or_else(|e| panic!("{e}"));
    let (got, name) = match opened {
        wgpu::Backend::Metal => (wgpu::Backends::METAL, "metal"),
        wgpu::Backend::Vulkan => (wgpu::Backends::VULKAN, "vulkan"),
        other => panic!("the harness opened {other:?}"),
    };
    assert_eq!(
        got, want,
        "{BACKEND_VAR}={value:?}: opened {opened:?}, selected {want:?}"
    );
    assert!(
        log.contains(name),
        "the log {log:?} does not name the opened {name}"
    );
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
        Some("r206_opened") => r206_opened(),
        other => panic!("qa_child: unknown body {other:?}"),
    }
}
