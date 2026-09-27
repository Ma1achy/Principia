//! QA tests for R-206, written from the ruling's words and REQ-SYS-065 as amended: "When PRIN_GPU_BACKEND is unset,
//! default by platform: metal on macOS, vulkan elsewhere. An explicit value still overrides, and an unknown value is
//! still an error. [...] Log which backend was chosen, so a test run always says what it ran on."
//!
//! The log's wording is not fixed by the ruling, so these tests assert only that it names the backend chosen and not
//! the other one. Each test registers a negative control (R-176, R-199). The harness test that needs the variable
//! unset re-runs this binary as a child with `QA_R206_CHILD` set, so the parent's environment is never mutated.

use std::process::Command;
use validation::gpu::{backend_choice, backend_from, GpuHarness, BACKEND_VAR};

const CHILD_VAR: &str = "QA_R206_CHILD";

/// R-206's platform default, from the ruling (not from the implementation): metal on macOS, vulkan elsewhere.
fn ruling_default() -> (&'static str, wgpu::Backends) {
    if cfg!(target_os = "macos") {
        ("metal", wgpu::Backends::METAL)
    } else {
        ("vulkan", wgpu::Backends::VULKAN)
    }
}

/// The backend that is not this platform's default.
fn ruling_other() -> (&'static str, wgpu::Backends) {
    if cfg!(target_os = "macos") {
        ("vulkan", wgpu::Backends::VULKAN)
    } else {
        ("metal", wgpu::Backends::METAL)
    }
}

/// The selection for `value` is `want`, and its log line names `name` and not the other backend.
fn check_selects(value: Option<&str>, want: wgpu::Backends, name: &str) {
    let (backends, log) =
        backend_choice(value).unwrap_or_else(|e| panic!("{BACKEND_VAR}={value:?}: {e}"));
    assert_eq!(
        backends, want,
        "{BACKEND_VAR}={value:?} selected {backends:?}"
    );
    assert_eq!(
        backend_from(value),
        Ok(want),
        "backend_from and backend_choice disagree for {value:?}"
    );
    let other = if name == "metal" { "vulkan" } else { "metal" };
    assert!(
        log.contains(name) && !log.contains(other),
        "{BACKEND_VAR}={value:?}: the log does not say it chose {name}: {log:?}"
    );
}

/// Unset selects the platform's backend and logs it; an explicit value overrides the default, including the backend
/// that is not the platform's (R-206).
#[test]
fn qa_r206_unset_defaults_by_platform_and_explicit_overrides() {
    let (name, want) = ruling_default();
    check_selects(None, want, name);
    let (other_name, other) = ruling_other();
    check_selects(Some(other_name), other, other_name);
    check_selects(Some(name), want, name);
}

validation::negative_control!(
    qa_r206_unset_defaults_by_platform_and_explicit_overrides,
    "an unset variable claimed to select the other platform's backend must fail",
    {
        let (other_name, other) = ruling_other();
        check_selects(None, other, other_name)
    }
);

/// `value` is refused with an error that names the variable.
fn check_refused(value: &str) {
    let err =
        backend_from(Some(value)).expect_err(&format!("{BACKEND_VAR}={value:?} was accepted"));
    assert!(
        err.to_string().contains(BACKEND_VAR),
        "{BACKEND_VAR}={value:?}: the error does not name the variable: {err}"
    );
    assert!(
        backend_choice(Some(value)).is_err(),
        "{BACKEND_VAR}={value:?}: backend_choice accepted it"
    );
}

/// An unknown value is still an error naming the variable (R-206): a set-but-empty value is a value, not "unset",
/// and near-misses of the two names are not silently taken as the platform default.
#[test]
fn qa_r206_unknown_value_is_still_an_error() {
    for value in [
        "",
        " ",
        "dx12",
        "gl",
        "METAL",
        "Vulkan",
        " metal",
        "vulkan ",
        "metal,vulkan",
        "default",
    ] {
        check_refused(value);
    }
}

validation::negative_control!(
    qa_r206_unknown_value_is_still_an_error,
    "a known backend name must not be refused",
    check_refused(ruling_default().0)
);

/// The opened adapter's backend is the one `backend_choice` selected for `value`, and its log names it.
fn check_opened(value: Option<&str>, opened: wgpu::Backend) {
    let (want, log) = backend_choice(value).unwrap_or_else(|e| panic!("{e}"));
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

/// `GpuHarness::new()` opens the backend selected for this run's variable, set or unset. In child mode it also prints
/// the backend it opened, for `qa_r206_harness_logs_the_backend_it_ran_on`.
#[test]
fn qa_r206_harness_opens_the_selected_backend() {
    let value = std::env::var(BACKEND_VAR).ok();
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let opened = h.adapter_info().backend;
    // Printed before the check, so a parent sees what was opened even when the check fails.
    if std::env::var_os(CHILD_VAR).is_some() {
        println!("QA_R206_OPENED backend={opened:?}");
    }
    check_opened(value.as_deref(), opened);
}

validation::negative_control!(
    qa_r206_harness_opens_the_selected_backend,
    "an adapter on the other backend must not pass as the selected one",
    {
        let (name, _) = ruling_default();
        let other = if name == "metal" {
            wgpu::Backend::Vulkan
        } else {
            wgpu::Backend::Metal
        };
        check_opened(None, other)
    }
);

/// Runs `qa_r206_harness_opens_the_selected_backend` in a child with `PRIN_GPU_BACKEND` set to `value` (or removed);
/// returns (stdout, stderr).
fn run_child(value: Option<&str>) -> (String, String) {
    let (ok, out, err) = spawn_child(value);
    assert!(ok, "{BACKEND_VAR}={value:?}: child failed:\n{out}\n{err}");
    (out, err)
}

/// `run_child` without the success assertion: (success, stdout, stderr).
fn spawn_child(value: Option<&str>) -> (bool, String, String) {
    let mut cmd = Command::new(std::env::current_exe().expect("test binary path"));
    cmd.args([
        "qa_r206_harness_opens_the_selected_backend",
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ])
    .env(CHILD_VAR, "1");
    match value {
        Some(v) => cmd.env(BACKEND_VAR, v),
        None => cmd.env_remove(BACKEND_VAR),
    };
    let o = cmd.output().expect("child test binary ran");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    (o.status.success(), out, err)
}

/// The child opened `shown` (wgpu's `Debug` name) and its stderr has a line naming `name` and not the other backend.
fn check_logged(out: &str, err: &str, name: &str, shown: &str) {
    assert!(
        out.contains(&format!("QA_R206_OPENED backend={shown}")),
        "the harness did not open {shown}:\n{out}"
    );
    let other = if name == "metal" { "vulkan" } else { "metal" };
    assert!(
        err.lines()
            .any(|l| l.to_lowercase().contains(name) && !l.to_lowercase().contains(other)),
        "no stderr line says the run used {name}:\n{err}"
    );
}

/// "Log which backend was chosen, so a test run always says what it ran on" (R-206): `GpuHarness::new()` writes a
/// line naming the backend to stderr, whether the variable is unset (the platform default) or set explicitly.
#[test]
fn qa_r206_harness_logs_the_backend_it_ran_on() {
    let (name, _) = ruling_default();
    let shown = if name == "metal" { "Metal" } else { "Vulkan" };
    for value in [None, Some(name)] {
        let (out, err) = run_child(value);
        check_logged(&out, &err, name, shown);
    }
}

validation::negative_control!(
    qa_r206_harness_logs_the_backend_it_ran_on,
    "a run on the platform's backend must not pass as logging the other one",
    {
        let (other, _) = ruling_other();
        let (out, err) = run_child(None);
        let shown = if other == "metal" { "Metal" } else { "Vulkan" };
        check_logged(&out, &err, other, shown)
    }
);

/// With `PRIN_GPU_BACKEND` set to `value`, the harness never opens the platform default's backend: it opens the
/// named one, or fails for want of an adapter (a macOS host has no Vulkan driver unless one is installed).
fn check_not_the_default(value: Option<&str>) {
    let (name, _) = ruling_default();
    let shown = if name == "metal" { "Metal" } else { "Vulkan" };
    let (_, out, err) = spawn_child(value);
    assert!(
        !out.contains(&format!("QA_R206_OPENED backend={shown}")),
        "{BACKEND_VAR}={value:?}: the harness opened the platform default {shown} instead:\n{out}\n{err}"
    );
}

/// "An explicit value still overrides" (R-206), at the harness: setting the other platform's backend does not open
/// this platform's default.
#[test]
fn qa_r206_harness_explicit_value_overrides_the_default() {
    check_not_the_default(Some(ruling_other().0));
}

validation::negative_control!(
    qa_r206_harness_explicit_value_overrides_the_default,
    "an unset variable opens the default, so it must fail the check",
    check_not_the_default(None)
);
