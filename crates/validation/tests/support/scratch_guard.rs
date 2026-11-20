//! The scratch guard's tests (REQ-VAL-178, R-342): a passing body's scratch is gone afterwards; a panicking body's is
//! kept and its path printed. xtask's, prin's and validation's `tests/scratch_guard.rs` include this file with
//! `#[path]`, beside `support/scratch.rs`. A control's scratch is deleted only when the control panics with its
//! expected message, and libtest's verdict is the control's own (R-359). Each test registers its negative control
//! (R-176): a guard that never deletes, and one that always deletes.

use std::path::Path;
#[cfg(feature = "controls")]
use std::path::PathBuf;
use std::process::Command;

use validation::spawn::Spawn;

use super::scratch::{settle, Scratch};

/// Set in the child run of [`scratch_guard_keeps_a_failing_tests_scratch`]: the file it writes its scratch path to.
const CHILD: &str = "SCRATCH_GUARD_CHILD";

/// Set in the child run: `always` for the control's guard that always deletes, else the guard as it is.
const SETTLE: &str = "SCRATCH_GUARD_SETTLE";

/// Set in the child run of [`scratch_guard_keeps_a_wrong_message_controls_scratch`]'s control: how it panics.
#[cfg(feature = "controls")]
const MESSAGE: &str = "SCRATCH_GUARD_MESSAGE";

/// A guard that never deletes: the control of [`scratch_guard_deletes_a_passing_tests_scratch`].
#[cfg(feature = "controls")]
fn never_delete(_: &Path, _: bool) {}

/// A guard that always deletes: the control of [`scratch_guard_keeps_a_failing_tests_scratch`].
#[cfg(feature = "controls")]
fn always_delete(path: &Path, _: bool) {
    settle(path, false);
}

/// A body that makes a scratch folder, with a file in a subfolder, and a scratch file, under guards settled by
/// `with`, and passes; neither is left once the guards drop.
fn check_pass_deletes(with: fn(&Path, bool)) {
    let made = {
        let mut folder = Scratch::new("scratch_guard_pass");
        let mut file = Scratch::new("scratch_guard_pass_file");
        (folder.settle, file.settle) = (with, with);
        std::fs::create_dir_all(folder.join("sub")).expect("the scratch folder is made");
        std::fs::write(folder.join("sub/f"), "f").expect("a file in the folder is written");
        std::fs::write(&file, "f").expect("the scratch file is written");
        [folder.to_path_buf(), file.to_path_buf()]
    };
    let left: Vec<_> = made.iter().filter(|p| p.exists()).collect();
    // The control's guard leaves them; they are removed before the check, so no run leaves them behind.
    for path in &left {
        settle(path, false);
    }
    assert!(
        left.is_empty(),
        "a passing test's scratch was left: {left:?}"
    );
}

#[test]
fn scratch_guard_deletes_a_passing_tests_scratch() {
    check_pass_deletes(settle);
}

validation::negative_control!(
    scratch_guard_deletes_a_passing_tests_scratch,
    "a guard that never deletes must fail the check",
    expected = "a passing test's scratch was left",
    check_pass_deletes(never_delete)
);

/// Runs [`scratch_guard_keeps_a_failing_tests_scratch`] as a child whose body makes a scratch folder and panics, its
/// guard the control's that always deletes when `how` is `always`. The folder is kept and its path is in the child's
/// failure output.
fn check_panic_keeps(how: &str) {
    let record = Scratch::new("scratch_guard_record");
    let test = module_path!().split_once("::").map_or("", |(_, m)| m);
    let out = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            &format!("{test}::scratch_guard_keeps_a_failing_tests_scratch"),
            "--exact",
            "--nocapture",
        ])
        .env(CHILD, &*record)
        .env(SETTLE, how)
        .timed_output()
        .expect("the child test ran");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let made = std::fs::read_to_string(&record)
        .unwrap_or_else(|_| panic!("the child made no scratch:\n{stderr}"));
    let made = Path::new(made.trim());
    let kept = made.exists();
    // The child's folder is this test's to remove, once looked at.
    settle(made, false);
    assert!(
        !out.status.success(),
        "the child's body did not fail:\n{stderr}"
    );
    assert!(
        kept,
        "a failing test's scratch was deleted: {}",
        made.display()
    );
    assert!(
        stderr.contains(&format!("scratch kept: {}", made.display())),
        "the kept scratch's path is not in the failure output:\n{stderr}"
    );
}

/// A scratch folder whose path goes to `record`, its guard the one [`SETTLE`] names.
fn recorded(record: &Path) -> Scratch {
    let folder = Scratch::new("scratch_guard_fail");
    #[cfg(feature = "controls")]
    let folder = {
        let mut folder = folder;
        if std::env::var(SETTLE).as_deref() == Ok("always") {
            folder.settle = always_delete;
        }
        folder
    };
    std::fs::create_dir_all(&folder).expect("the scratch folder is made");
    std::fs::write(record, folder.to_str().expect("a UTF-8 path")).expect("the path is recorded");
    folder
}

/// The child's body: a recorded scratch folder, then a panic.
fn child(record: &Path) {
    let _folder = recorded(record);
    panic!("the child's body fails, as a failing test's does");
}

#[test]
fn scratch_guard_keeps_a_failing_tests_scratch() {
    match std::env::var_os(CHILD) {
        Some(record) => child(Path::new(&record)),
        None => check_panic_keeps("guard"),
    }
}

validation::negative_control!(
    scratch_guard_keeps_a_failing_tests_scratch,
    "a guard that always deletes must fail the check",
    expected = "a failing test's scratch was deleted",
    check_panic_keeps("always")
);

/// Runs [`scratch_guard_keeps_a_wrong_message_controls_scratch`]'s control as a child whose body makes a scratch
/// folder, its guard the one [`SETTLE`] names by `how`, then panics with the control's expected message (`expected`),
/// another (`wrong`), or not at all (`none`). Returns whether libtest passed it, its stderr, and its folder.
#[cfg(feature = "controls")]
fn run_control(message: &str, how: &str) -> (bool, String, PathBuf) {
    let record = Scratch::new("scratch_guard_record");
    let test = module_path!().split_once("::").map_or("", |(_, m)| m);
    let out = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            &format!(
                "{test}::scratch_guard_keeps_a_wrong_message_controls_scratch::negative_control"
            ),
            "--exact",
            "--nocapture",
        ])
        .env(CHILD, &*record)
        .env(SETTLE, how)
        .env(MESSAGE, message)
        .timed_output()
        .expect("the child control ran");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let made = std::fs::read_to_string(&record)
        .unwrap_or_else(|_| panic!("the child control made no scratch:\n{stderr}"));
    (out.status.success(), stderr, PathBuf::from(made.trim()))
}

/// A control that panics with a wrong message, or not at all, fails and keeps its scratch, its path in the failure
/// output; one that panics with its expected message passes and leaves none (R-359).
#[cfg(feature = "controls")]
fn check_control_scratch(how: &str) {
    for message in ["wrong", "none", "expected"] {
        let (passed, stderr, made) = run_control(message, how);
        let kept = made.exists();
        settle(&made, false);
        let matched = message == "expected";
        assert_eq!(
            passed, matched,
            "libtest's verdict on a `{message}` control:\n{stderr}"
        );
        assert!(
            matched || kept,
            "a `{message}` control's scratch was deleted: {}",
            made.display()
        );
        assert!(
            matched || stderr.contains(&format!("scratch kept: {}", made.display())),
            "the kept scratch's path is not in the failure output:\n{stderr}"
        );
        assert!(
            !(matched && kept),
            "a matching control kept its scratch: {}",
            made.display()
        );
    }
}

#[cfg(feature = "controls")]
#[test]
fn scratch_guard_keeps_a_wrong_message_controls_scratch() {
    check_control_scratch("guard");
}

// Run as a child, the control is the subject of the check: a recorded scratch folder, then a panic with the expected
// message, with another, or none. Its control proper: a guard that deletes on any panic, as #114's first design did.
validation::negative_control!(
    scratch_guard_keeps_a_wrong_message_controls_scratch,
    "a negative_control! that deletes the scratch on any panic must fail the check",
    expected = "control's scratch was deleted",
    match std::env::var_os(CHILD) {
        Some(record) => {
            let _folder = recorded(Path::new(&record));
            match std::env::var(MESSAGE).as_deref() {
                Ok("expected") => {
                    panic!("the child control panics with its expected message: a control's scratch was deleted")
                }
                Ok("wrong") => panic!("the child control panicked with another message"),
                _ => {}
            }
        }
        None => check_control_scratch("always"),
    }
);
