//! QA tests for TASK-M0-57 at gui's mock canvas, written from REQ-SYS-097's statement: DX12 "selected only by an
//! explicit `PRIN_GPU_BACKEND=dx12` in a build with that feature; an unset value never selects it (R-206); without the
//! feature `dx12` is refused as an error naming the variable and the feature".
//!
//! Which build this is comes from `cfg!(feature = "dx12")` in this test crate, never from gui's own flag. gui takes no
//! dependency on validation (R-187), so each check runs its own control (R-176) in the test, through [`rejects`].

use gui::mock::canvas::{backend, BACKEND_VAR};

/// Whether this test build has gui's `dx12` feature, read by the test crate itself.
const FEATURE: bool = cfg!(feature = "dx12");

/// `check` must panic: the control that shows the check can fail.
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(outcome.is_err(), "control passed: {what}");
}

/// The selection `backend` makes, as a function of the variable's value.
type Choice<'a> = &'a dyn Fn(Option<&str>) -> Result<wgpu::Backends, String>;

/// `PRIN_GPU_BACKEND=dx12` as `choice` takes it in a build with the feature (`feature`) or without: with it, DX12 and
/// only DX12; without it, refused naming the variable and, once the variable's own setting is taken out, the feature.
/// Unset is the platform's backend, never DX12, either way.
fn check(choice: Choice, feature: bool) {
    match (feature, choice(Some("dx12"))) {
        (true, got) => assert_eq!(
            got,
            Ok(wgpu::Backends::DX12),
            "dx12 with the feature did not select DX12"
        ),
        (false, Ok(b)) => panic!("dx12 without the feature was accepted, as {b:?}"),
        (false, Err(err)) => {
            assert!(
                err.contains(BACKEND_VAR),
                "the refusal names no variable: {err}"
            );
            let mut rest = err.clone();
            for setting in ["=\"dx12\"", "='dx12'", "=dx12"] {
                rest = rest.replace(&format!("{BACKEND_VAR}{setting}"), BACKEND_VAR);
            }
            let rest = rest.to_lowercase();
            assert!(
                rest.contains("feature") && rest.contains("dx12"),
                "the refusal does not name the feature `dx12`: {err}"
            );
        }
    }
    let platform = if cfg!(target_os = "macos") {
        wgpu::Backends::METAL
    } else {
        wgpu::Backends::VULKAN
    };
    assert_eq!(
        choice(None),
        Ok(platform),
        "unset is not the platform's backend"
    );
}

#[test]
fn qa_m0_57_mock_canvas_reads_dx12_as_this_build() {
    check(&backend, FEATURE);
    rejects("this build read as the other one", || {
        check(&backend, !FEATURE)
    });
    rejects("a refusal naming the variable and no feature", || {
        check(
            &|v| match v {
                Some("dx12") => backend(Some("gl")).map(|_| wgpu::Backends::DX12),
                v => backend(v),
            },
            false,
        )
    });
    rejects("unset selecting DX12", || {
        check(
            &|v| match v {
                None => Ok(wgpu::Backends::DX12),
                v => backend(v),
            },
            FEATURE,
        )
    });
}
