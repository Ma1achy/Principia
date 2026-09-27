//! The helpers and checks `qa_TASK-M0-04_r2.rs` shares with its controls in `qa_TASK-M0-04_controls.rs`, so each
//! control runs the check it controls, not a copy of it (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its
//! own, so each includes this file with `#[path]`; every item here is used by both.

use std::process::Command;
use validation::gpu::AdapterInfo;
use validation::spawn::Spawn;

pub fn info(backend: wgpu::Backend) -> AdapterInfo {
    AdapterInfo {
        name: "qa-adapter-name".into(),
        backend,
        driver: "qa-driver".into(),
        driver_info: "qa-driver-info".into(),
    }
}

pub fn counts_with(proptest_cases: &str) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_qa_child"))
        .arg("count_cases")
        .env("PROPTEST_CASES", proptest_cases)
        .timed_output()
        .expect("qa_child ran");
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

/// `qa_adapter_info_printout_names_the_backend`'s check that the printout names the backend.
pub fn check_names(shown: &str, name: &str) {
    assert!(
        shown.contains(name),
        "adapter info does not name {name}: {shown:?}"
    );
}

/// `qa_prop_case_count_is_not_replaced_by_proptest_cases`'s check.
pub fn check_count_kept(n: &str, line: &str) {
    assert!(
        line.contains("shared=256 ") && line.contains("config=256 "),
        "PROPTEST_CASES={n} changed the shared case count: {line}"
    );
}
