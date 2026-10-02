//! The scratch guard's tests (REQ-VAL-178, R-342): a passing body's scratch is gone afterwards; a panicking body's is
//! kept and its path printed. xtask's, prin's and validation's `tests/scratch_guard.rs` include this file with
//! `#[path]`, beside `support/scratch.rs`. Each test registers its negative control (R-176): a guard that never
//! deletes, and one that always deletes.

use std::path::Path;
use std::process::Command;

use validation::spawn::Spawn;

use super::scratch::{settle, Scratch};

/// Set in the child run of [`scratch_guard_keeps_a_failing_tests_scratch`]: the file it writes its scratch path to.
const CHILD: &str = "SCRATCH_GUARD_CHILD";

/// Set in the child run: `always` for the control's guard that always deletes, else the guard as it is.
const SETTLE: &str = "SCRATCH_GUARD_SETTLE";

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

/// The child's body: a scratch folder whose path goes to `record`, then a panic.
fn child(record: &Path) {
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
