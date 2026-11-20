//! The `gui` binary exits non-zero, saying why, on a command it does not run, on the mock (the `mock` feature) or on
//! the real engine.

use std::process::Command;

#[test]
fn binary_fails_on_a_command_it_does_not_run() {
    let out = Command::new(env!("CARGO_BIN_EXE_gui"))
        .arg("bogus")
        .output()
        .expect("gui ran");
    assert!(!out.status.success(), "`gui bogus` succeeded");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("gui: "), "no reason given: {stderr}");
}
