//! QA tests for TASK-M0-04, round 2: written from the acceptance lines that the surviving mutants of
//! `cargo mutants --in-diff` showed untested (R-196, R-202).
//!
//! - The CI log's adapter info must name the backend (acceptance: "the adapter info each prints names the Metal and
//!   the Vulkan (llvmpipe/lavapipe) backend"; REQ-SYS-065 verify: "with the named backends").
//! - The shared config runs 256 cases per property (acceptance `prop_seed -- --nocapture`; REQ-VAL-151, R-203): the
//!   calibrated value, whatever proptest's own `PROPTEST_CASES` variable says.
//!
//! Each test carries an inline negative control (R-176; the registry is TASK-M0-21/M0-22, R-198). The environment test
//! runs the `qa_child` binary as a child (R-210), so the parent's environment is never mutated.

use std::process::Command;
use validation::gpu::{AdapterInfo, GpuHarness};

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

    // The live harness's printout names the backend PRIN_GPU_BACKEND, or the platform default, selected.
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let live = h.adapter_info();
    let expected = match std::env::var("PRIN_GPU_BACKEND").as_deref() {
        Ok("metal") => "Metal",
        Ok("vulkan") => "Vulkan",
        // Unset: the platform default (R-206).
        Err(std::env::VarError::NotPresent) if cfg!(target_os = "macos") => "Metal",
        Err(std::env::VarError::NotPresent) => "Vulkan",
        other => panic!("PRIN_GPU_BACKEND must be metal, vulkan or unset here: {other:?}"),
    };
    let shown = live.to_string();
    println!("{shown}");
    assert!(
        shown.contains(expected) && shown.contains(&live.name),
        "the harness's adapter info does not name {expected} and its adapter: {shown:?}"
    );
}

fn counts_with(proptest_cases: &str) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_qa_child"))
        .arg("count_cases")
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
