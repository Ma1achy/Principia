//! QA tests for TASK-M0-04, round 2: written from the acceptance lines that the surviving mutants of
//! `cargo mutants --in-diff` showed untested (R-196, R-202).
//!
//! - The CI log's adapter info must name the backend (acceptance: "the adapter info each prints names the Metal and
//!   the Vulkan (llvmpipe/lavapipe) backend"; REQ-SYS-065 verify: "with the named backends").
//! - The shared config runs 256 cases per property (acceptance `prop_seed -- --nocapture`; REQ-VAL-151, R-203): the
//!   calibrated value, whatever proptest's own `PROPTEST_CASES` variable says.
//!
//! Each test carries an inline negative control (R-176; the registry is TASK-M0-21/M0-22, R-198). The environment test
//! re-runs this binary as a child, so the parent's environment is never mutated.

use proptest::test_runner::{Config, TestRunner};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use validation::gpu::{AdapterInfo, GpuHarness};
use validation::prop;

const CHILD_VAR: &str = "QA_M0_04_R2_CHILD";

fn info(backend: wgpu::Backend) -> AdapterInfo {
    AdapterInfo {
        name: "qa-adapter-name".into(),
        backend,
        driver: "qa-driver".into(),
        driver_info: "qa-driver-info".into(),
    }
}

/// REQ-SYS-065: the printed adapter info names the backend, the adapter and the driver, so the CI log shows which
/// backend each GPU job ran on.
#[test]
fn qa_adapter_info_printout_names_the_backend() {
    for (backend, name) in [
        (wgpu::Backend::Metal, "Metal"),
        (wgpu::Backend::Vulkan, "Vulkan"),
    ] {
        let shown = info(backend).to_string();
        assert!(
            shown.contains(name),
            "adapter info does not name {name}: {shown:?}"
        );
        for field in ["qa-adapter-name", "qa-driver"] {
            assert!(
                shown.contains(field),
                "adapter info omits {field}: {shown:?}"
            );
        }
    }
    // Control: the printout discriminates: a Metal adapter's line does not name Vulkan, nor the reverse.
    assert!(!info(wgpu::Backend::Metal).to_string().contains("Vulkan"));
    assert!(!info(wgpu::Backend::Vulkan).to_string().contains("Metal"));

    // The live harness's printout names the backend PRIN_GPU_BACKEND selected.
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let live = h.adapter_info();
    let expected = match std::env::var("PRIN_GPU_BACKEND").as_deref() {
        Ok("metal") => "Metal",
        Ok("vulkan") => "Vulkan",
        other => panic!("PRIN_GPU_BACKEND must be metal or vulkan here: {other:?}"),
    };
    let shown = live.to_string();
    println!("{shown}");
    assert!(
        shown.contains(expected) && shown.contains(&live.name),
        "the harness's adapter info does not name {expected} and its adapter: {shown:?}"
    );
}

/// Child mode only: counts the cases `prop::run` and a default proptest runner run, and prints both.
#[test]
fn qa_child_count_cases() {
    if std::env::var_os(CHILD_VAR).is_none() {
        return;
    }
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

fn counts_with(proptest_cases: &str) -> String {
    let o = Command::new(std::env::current_exe().expect("test binary path"))
        .args(["qa_child_count_cases", "--exact", "--nocapture"])
        .env(CHILD_VAR, "1")
        .env("PROPTEST_CASES", proptest_cases)
        .output()
        .expect("child test binary ran");
    let t = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.status.success(), "child failed: {t}");
    let at = t
        .find("QA_CASES ")
        .unwrap_or_else(|| panic!("child printed no counts: {t}"));
    t[at..].lines().next().unwrap_or_default().to_string()
}

/// REQ-VAL-151 / R-203: the shared config runs the calibrated 256 cases per property even when proptest's own
/// `PROPTEST_CASES` says otherwise, so no environment silently replaces the value the human confirms at the M0 gate.
#[test]
fn qa_prop_case_count_is_not_replaced_by_proptest_cases() {
    for n in ["3", "1000"] {
        let line = counts_with(n);
        assert!(
            line.contains("shared=256 ") && line.contains("config=256 "),
            "PROPTEST_CASES={n} changed the shared case count: {line}"
        );
        // Control: the variable reached the child and does change a default proptest runner, so the 256 above
        // is the shared config's own value, not an environment the child never saw.
        assert!(
            line.ends_with(&format!("default={n}")),
            "control: PROPTEST_CASES={n} did not reach the child: {line}"
        );
    }
}
