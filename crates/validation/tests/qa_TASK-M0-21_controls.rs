//! Negative controls for qa's tests in `qa_TASK-M0-21.rs` and `qa_TASK-M0-21_r2.rs` (REQ-VAL-153; R-199, R-209).
//! Each runs its test's check, copied here, on a fixture in `fixtures/controls_qa_m0_21/` with one file removed, which
//! flips the verdict the check must reject, so the test can fail (philosophy §4.4).
#![cfg(feature = "controls")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use validation::negative_control;
use validation::spawn::Spawn;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The workspace's target directory, where `validation` is already built.
fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cargo() -> Command {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cmd.env("CARGO_TARGET_DIR", target_dir());
    cmd
}

/// Copies `from` to `to`, making each manifest's relative `crates/validation` path absolute.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else if path.file_name().unwrap() == "Cargo.toml" {
            let validation = root().join("crates/validation");
            let text = std::fs::read_to_string(&path).unwrap();
            let text = text.replace(
                "../../../../crates/validation",
                validation.to_str().unwrap(),
            );
            std::fs::write(
                &dest,
                text.replace("../../../crates/validation", validation.to_str().unwrap()),
            )
            .unwrap();
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}

/// `cargo <args> --manifest-path <copy>` on a copy of fixture `name` outside this workspace, with `remove` deleted.
fn on_copy(name: &str, remove: &str, args: &[&str]) -> Output {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_21_controls")
        .join(format!(
            "{name}-{}-{}",
            std::process::id(),
            RUN.fetch_add(1, Ordering::Relaxed)
        ));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&root().join("fixtures/controls_qa_m0_21").join(name), &dir);
    std::fs::remove_file(dir.join(remove)).unwrap();
    std::fs::copy(root().join("Cargo.lock"), dir.join("Cargo.lock")).unwrap();
    let output = cargo()
        .args(args)
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .timed_output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    output
}

/// `cargo xtask controls` on the copy: (success, stdout and stderr).
fn controls(name: &str, remove: &str) -> (bool, String) {
    let manifest = root().join("Cargo.toml");
    let xtask = [
        "run",
        "-q",
        "-p",
        "xtask",
        "--manifest-path",
        manifest.to_str().unwrap(),
        "--",
        "controls",
    ];
    let o = on_copy(name, remove, &xtask);
    let (out, err) = (
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr),
    );
    (
        o.status.success(),
        format!("stdout:\n{out}\nstderr:\n{err}"),
    )
}

negative_control!(
    qa_leaky_control_beside_a_sound_one_of_the_same_name_fails_naming_the_test,
    "with the leaky control removed, the command passes, so the must-fail check must fail",
    expected = "a control that leaves `doubles` passing passed the command",
    {
        let (ok, all) = controls("dup_name", "tests/a_leaky.rs");
        assert!(
            !ok,
            "a control that leaves `doubles` passing passed the command:\n{all}"
        );
    }
);

negative_control!(
    qa_control_under_another_name_does_not_pair,
    "with the misnamed control removed, the rightly named one pairs, so the must-fail check must fail",
    expected = "a test whose only control names another test passed",
    {
        let (ok, all) = controls("misnamed", "tests/misnamed.rs");
        assert!(!ok, "a test whose only control names another test passed:\n{all}");
    }
);

negative_control!(
    qa_unit_test_in_a_binary_target_pairs_with_its_control_in_tests,
    "with the control in tests/ removed, the must-pass check must fail",
    expected = "the binary's unit test did not pair with its control",
    {
        let (ok, all) = controls("bin_target", "tests/controls.rs");
        assert!(
            ok,
            "the binary's unit test did not pair with its control:\n{all}"
        );
    }
);

negative_control!(
    qa_workspace_pairs_within_each_crate_and_skips_the_featureless_one,
    "with bare's uncontrolled test removed, the command passes, so the must-fail check must fail",
    expected = "a test paired with a control in another crate",
    {
        let (ok, all) = controls("workspace", "bare/tests/doubles.rs");
        assert!(!ok, "a test paired with a control in another crate:\n{all}");
    }
);

negative_control!(
    qa_binary_only_crate_is_checked_and_passes,
    "with the control in tests/ removed, the must-pass check must fail",
    expected = "a binary-only crate with a discriminating control failed the command",
    {
        let (ok, all) = controls("bin_only", "tests/controls.rs");
        assert!(
            ok,
            "a binary-only crate with a discriminating control failed the command:\n{all}"
        );
    }
);

negative_control!(
    qa_controls_exist_only_under_the_feature,
    "under the feature the sound control compiles and runs, so the no-control check must fail",
    expected = "a control compiled",
    {
        let args = [
            "test",
            "--tests",
            "--no-fail-fast",
            "--features",
            "controls",
        ];
        let o = on_copy("dup_name", "tests/a_leaky.rs", &args);
        let out = String::from_utf8_lossy(&o.stdout);
        assert!(o.status.success(), "cargo test failed:\n{out}");
        assert!(
            !out.contains("negative_control"),
            "a control compiled:\n{out}"
        );
    }
);
