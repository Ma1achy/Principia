//! R-212 on libtest itself (REQ-VAL-154): `negative_control!` registers a control with
//! `#[should_panic(expected = …)]`, so a control that panics in its own setup, before its check, fails, and one whose
//! expected message reaches the check passes. Runs `cargo test --features controls` on a copy of xtask's
//! `wrong_message` fixture, which holds one control of each kind.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use validation::spawn::Spawn;

#[path = "support/own_target.rs"]
mod own_target;
use own_target::{Lease, FIXTURES};

/// libtest's stdout for `cargo test --features controls` on a copy of the `wrong_message` fixture, run once.
fn fixture_run() -> &'static str {
    static RUN: OnceLock<String> = OnceLock::new();
    RUN.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture = root.join("xtask/tests/fixtures/controls/wrong_message");
        let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let copy = tmp.join(format!("expected_message-{}", std::process::id()));
        for file in ["src/lib.rs", "tests/double.rs"] {
            std::fs::create_dir_all(copy.join(file).parent().unwrap()).unwrap();
            std::fs::copy(fixture.join(file), copy.join(file)).unwrap();
        }
        let validation = root.join("crates/validation").canonicalize().unwrap();
        let manifest = std::fs::read_to_string(fixture.join("Cargo.toml")).unwrap();
        let manifest = manifest.replace(
            "../../../../../crates/validation",
            validation.to_str().unwrap(),
        );
        std::fs::write(copy.join("Cargo.toml"), manifest).unwrap();
        std::fs::copy(root.join("Cargo.lock"), copy.join("Cargo.lock")).unwrap();
        // A target directory of its own: xtask's tests build copies of this fixture too (REQ-VAL-164).
        let target = Lease::take(FIXTURES);
        let output = Command::new(env!("CARGO"))
            .args([
                "test",
                "--features",
                "controls",
                "--tests",
                "--no-fail-fast",
            ])
            .arg("--manifest-path")
            .arg(copy.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", target.dir())
            .timed_output()
            .expect("run cargo test on the fixture");
        std::fs::remove_dir_all(&copy).unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    })
}

/// In `stdout`, the control of `setup` failed and the control of `check` passed.
fn check_outcomes(stdout: &str, setup: &str, check: &str) {
    assert!(
        stdout.contains(&format!(
            "test {setup}::negative_control - should panic ... FAILED"
        )),
        "the control panicking in its setup did not fail:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "test {check}::negative_control - should panic ... ok"
        )),
        "the control tripping its check did not pass:\n{stdout}"
    );
}

#[test]
fn control_panicking_in_setup_fails_and_one_reaching_its_check_passes() {
    check_outcomes(fixture_run(), "doubles_again", "doubles");
}

validation::negative_control!(
    control_panicking_in_setup_fails_and_one_reaching_its_check_passes,
    "the two controls swapped: the one tripping its check, required to fail",
    expected = "the control panicking in its setup did not fail",
    check_outcomes(fixture_run(), "doubles", "doubles_again")
);
