//! `cargo xtask gate`'s arguments (TASK-M4-27): what follows a gate's name reaches validation's `gate` binary as it stands.
//! Each test registers the control that must make it fail (R-176).

use validation::negative_control;
use xtask::gate::{words, Which};

fn check_forwarded(flags: &[&str]) {
    assert_eq!(
        words(Which::One("g", flags)),
        ["g", "--record", "x.json", "--cpu-f32"],
        "the flags did not reach the binary"
    );
}

#[test]
fn gate_flags_forward_what_follows_the_name() {
    check_forwarded(&["--record", "x.json", "--cpu-f32"]);
    assert_eq!(words(Which::One("g", &[])), ["g"]);
    assert_eq!(words(Which::All), ["--all"]);
    assert_eq!(words(Which::List), ["--list"]);
}

negative_control!(
    gate_flags_forward_what_follows_the_name,
    "a forward that drops the last flag",
    expected = "the flags did not reach the binary",
    check_forwarded(&["--record", "x.json"])
);

fn check_failure_reported(manifest: &str) {
    let manifest = std::path::Path::new(manifest);
    let e = xtask::gate::run(manifest, Which::List).expect_err("a failed gate run read clean");
    assert!(e.contains("gate --list failed"), "{e}");
}

#[test]
fn gate_flags_report_a_failed_run() {
    check_failure_reported("/nowhere/Cargo.toml");
}

negative_control!(
    gate_flags_report_a_failed_run,
    "the workspace itself",
    expected = "a failed gate run read clean",
    check_failure_reported(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"))
);
