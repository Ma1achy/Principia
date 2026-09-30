//! The subprocess bodies qa's tests spawn (R-210): not `#[test]`s, so `cargo xtask controls` never lists them. The
//! first argument names the body; each prints the marker line its parent test reads. Moved from
//! `tests/qa_TASK-M0-04.rs` (`open_harness`, `failing_property`), `tests/qa_TASK-M0-04_r2.rs` (`count_cases`) and
//! `tests/qa_R-206.rs` (`r206_opened`, R-213). `r217_out_of_group` is `tests/qa_TASK-M0-26_r217.rs`'s spawner of a
//! grandchild in a process group of its own (R-276).

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

/// Spawns the command in `args[1..]` in a process group of its own, at spawn (`process_group(0)`: the group is set
/// before the command runs, so it is never in this process's group), writes its pid to the file `args[0]`, and exits
/// without waiting for it: the out-of-group grandchild of `qa_TASK-M0-26_r217.rs`'s controls (R-276). It inherits
/// this process's stdout and stderr, so it holds whatever output they lead to. Not through the spawn helper: the
/// command is meant to outlive this process, beyond any timeout's reach (the helper is for tests, REQ-VAL-155).
#[cfg(unix)]
fn r217_out_of_group(args: &[String]) {
    use std::os::unix::process::CommandExt;
    let [pid_file, program, rest @ ..] = args else {
        panic!("r217_out_of_group: want <pid file> <program> [args]; got {args:?}")
    };
    // Suppressed: the lint asks for a `wait()`, but the command is meant to outlive this process, which exits at once;
    // it is then reparented, and its new parent reaps it, so no zombie is left.
    #[allow(clippy::zombie_processes)]
    let child = std::process::Command::new(program)
        .args(rest)
        .process_group(0)
        .spawn()
        .unwrap_or_else(|e| panic!("r217_out_of_group: {program}: {e}"));
    std::fs::write(pid_file, child.id().to_string())
        .unwrap_or_else(|e| panic!("r217_out_of_group: {pid_file}: {e}"));
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("open_harness") => open_harness(),
        Some("failing_property") => failing_property(),
        Some("count_cases") => count_cases(),
        Some("r206_opened") => r206_opened(),
        #[cfg(unix)]
        Some("r217_out_of_group") => {
            r217_out_of_group(&std::env::args().skip(2).collect::<Vec<_>>())
        }
        other => panic!("qa_child: unknown body {other:?}"),
    }
}
