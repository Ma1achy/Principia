//! Only the validator builds a `ValidatedChart` (`engine::chart_authoring`; lowering contract Part 5; REQ-SYS-013;
//! TASK-M2-14): `validated_chart_only_from_validate` checks that `fixtures/validated_chart`, which builds one with a
//! struct literal outside the crate, fails to compile, and fails only for the literal's private fields (E0451). Its
//! control builds the same chart through `validate`, which compiles, so the check fails.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../validation/tests/support/fixture_tree.rs"]
mod fixture_tree;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The fixture's forging line.
const FORGED: &str = "ValidatedChart { chart, descriptor, solves: Vec::new() } // FORGED";

/// `cargo check` on a copy of `fixtures/validated_chart`, its forging line replaced by `body`, in a directory and a
/// target directory of `own`'s (R-224), each kept across runs and written only where it changed (R-231).
fn check_fixture(own: &str, body: &str) -> Output {
    let fixture = root().join("fixtures/validated_chart");
    let read = |file: &str| std::fs::read_to_string(fixture.join(file)).unwrap();
    let crates = root().join("crates");
    let manifest = read("Cargo.toml").replace("../../crates", crates.to_str().unwrap());
    let lib = read("src/lib.rs");
    assert!(lib.contains(FORGED), "the fixture changed");
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let copy = tmp.join(format!("validated_chart-{own}-src"));
    fixture_tree::write_tree(
        &copy,
        &[
            ("src/lib.rs", lib.replace(FORGED, body)),
            ("Cargo.toml", manifest),
            (
                "Cargo.lock",
                std::fs::read_to_string(root().join("Cargo.lock")).unwrap(),
            ),
        ],
    );
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .current_dir(root())
        .env(
            "CARGO_TARGET_DIR",
            tmp.join(format!("validated_chart-{own}")),
        )
        .args([
            "check",
            "--quiet",
            "--message-format",
            "short",
            "--manifest-path",
        ])
        .arg(copy.join("Cargo.toml"))
        .timed_output()
        .expect("run cargo check on the fixture")
}

/// The build failed in the fixture's `src/lib.rs`, every error the private fields' E0451.
fn check_refused(o: &Output) {
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        !o.status.success(),
        "a ValidatedChart built outside the validator compiled:\n{stderr}"
    );
    let errors: Vec<&str> = stderr.lines().filter(|l| l.contains("error[")).collect();
    assert!(
        !errors.is_empty()
            && errors
                .iter()
                .all(|l| l.contains("error[E0451]") && l.contains("src/lib.rs")),
        "the build failed, but not only at the private fields:\n{stderr}"
    );
}

#[test]
fn validated_chart_only_from_validate() {
    check_refused(&check_fixture("test", FORGED));
}

negative_control!(
    validated_chart_only_from_validate,
    "the chart built through validate compiles",
    expected = "built outside the validator compiled",
    check_refused(&check_fixture(
        "control",
        "{ let _ = descriptor; engine::chart_authoring::validate(chart, None).unwrap() }"
    ))
);
