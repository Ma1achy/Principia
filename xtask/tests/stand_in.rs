//! The stand-in `cargo` (`xtask/src/bin/stand_in.rs`) answers each call as its `stand_in.json` says (REQ-TOOL-238):
//! the tests that run xtask against it rely on each answer, here checked one by one, on calls of every kind.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::spawn::Spawn;

#[path = "support/stand_in.rs"]
mod stand_in;

#[path = "../src/bin/stand_in/plain.rs"]
mod plain;

/// The stand-in configured by `config`, in a directory of its own named `case`, called with `args`: its stdout, its
/// exit code, and its directory.
fn call(case: &str, config: Value, args: &[&str]) -> (String, i32, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stand_in_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cargo = stand_in::stand_in_cargo(&dir, config);
    let output = Command::new(cargo)
        .args(args)
        .current_dir(&dir)
        .timed_output()
        .expect("run the stand-in");
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    (out, output.status.code().unwrap_or(-1), dir)
}

const LISTED: &str = "m::a: test\nm::a::negative_control: test\nm::b: test\n";

/// Each key answers its call, and only its call. `filtered` is the value the filtered calls are configured with, read
/// as on.
fn check_answers(case: &str, filtered: bool) {
    let config = || {
        json!({
            "metadata": stand_in::metadata("fake"),
            "listed": LISTED,
            "build": 7,
            "output": "other\n",
            "status": 3,
        })
    };
    // `metadata` answers only a call whose first argument is `metadata`.
    let (out, code, _) = call(&format!("{case}_metadata"), config(), &["metadata"]);
    assert_eq!(
        (out.trim(), code),
        (stand_in::metadata("fake").as_str(), 0),
        "the stand-in did not answer metadata"
    );
    let (out, code, _) = call(
        &format!("{case}_not_metadata"),
        config(),
        &["test", "metadata"],
    );
    assert_eq!(
        (out.as_str(), code),
        ("other\n", 3),
        "the stand-in answered metadata later in a call"
    );
    // `listed` answers `--list`, and `build` a call with `build-kernel`.
    let (out, code, _) = call(&format!("{case}_list"), config(), &["test", "--", "--list"]);
    assert_eq!(
        (out.as_str(), code),
        (LISTED, 0),
        "the stand-in did not list"
    );
    let (out, code, _) = call(
        &format!("{case}_build"),
        config(),
        &["run", "--", "build-kernel"],
    );
    assert_eq!(
        (out.as_str(), code),
        ("", 7),
        "the stand-in did not answer build-kernel"
    );
    // `ran` logs and reports the names after `--exact`, and nothing before it.
    let mut ran = config();
    ran["ran"] = json!("ran.log");
    let (out, code, dir) = call(
        &format!("{case}_exact"),
        ran,
        &[
            "test",
            "--",
            "--exact",
            "m::a::negative_control",
            "m::b::negative_control",
        ],
    );
    assert_eq!(
        (out.as_str(), code),
        (
            "test m::a::negative_control - should panic ... ok\ntest m::b::negative_control - should panic ... ok\n",
            3
        ),
        "the stand-in did not report the controls named after --exact"
    );
    assert_eq!(
        fs::read_to_string(dir.join("ran.log")).unwrap(),
        "m::a::negative_control\nm::b::negative_control\n",
        "the stand-in did not log the controls named after --exact"
    );
    // `filtered` answers the filter `negative_control` from the listing; any other call, and the filter where it is
    // off, gets `output`.
    let mut on = config();
    on["filtered"] = json!(filtered);
    let (out, _, _) = call(
        &format!("{case}_filtered"),
        on.clone(),
        &["test", "--", "negative_control"],
    );
    assert_eq!(
        out, "test m::a::negative_control - should panic ... ok\n",
        "the stand-in did not answer `negative_control` from its listing"
    );
    let (out, _, _) = call(&format!("{case}_unfiltered"), on, &["test", "--", "m::a"]);
    assert_eq!(
        out, "other\n",
        "the stand-in answered a call without the filter from its listing"
    );
    let (out, _, _) = call(
        &format!("{case}_filter_off"),
        config(),
        &["test", "--", "negative_control"],
    );
    assert_eq!(
        out, "other\n",
        "the stand-in answered the filter from its listing with `filtered` off"
    );
}

#[test]
fn stand_in_answers_as_configured() {
    check_answers("answers", true);
}

validation::negative_control!(
    stand_in_answers_as_configured,
    "`filtered` off, given to the check as on",
    expected = "the stand-in did not answer `negative_control` from its listing",
    check_answers("ctl_answers", false)
);

/// `log` records each call, with the working directory first under `log_cwd`, and `library` writes the stand-in
/// library in the working directory. `cwd` is whether the check expects the working directory; the configuration's
/// `log_cwd` is always on.
fn check_log_and_library(case: &str, cwd: bool) {
    let config = json!({ "log": "calls.log", "log_cwd": true, "library": "lib.so" });
    let (_, code, dir) = call(case, config, &["build", "--release"]);
    assert_eq!(code, 0, "the stand-in failed with no status set");
    let log = fs::read_to_string(dir.join("calls.log")).unwrap();
    let wrong = format!("the stand-in's log is not the call: {log:?}");
    if cwd {
        // The directory resolved, as `pwd -P` prints it and as build_kernel.rs's check compares it: without the
        // verbatim prefix `canonicalize` gives on Windows.
        let logged = log.strip_suffix(" build --release\n").expect(&wrong);
        assert_eq!(
            Path::new(logged),
            validation::paths::plain(&dir.canonicalize().unwrap()),
            "{wrong}: not in {}",
            dir.display()
        );
    } else {
        assert_eq!(log, "build --release\n", "{wrong}");
    }
    assert_eq!(
        fs::read_to_string(dir.join("target").join("release").join("lib.so")).unwrap(),
        "a stand-in backend\n",
        "the stand-in wrote no library"
    );
}

#[test]
fn stand_in_logs_and_writes_the_library() {
    check_log_and_library("log", true);
}

validation::negative_control!(
    stand_in_logs_and_writes_the_library,
    "a log with the working directory, given to the check as one without",
    expected = "the stand-in's log is not the call",
    check_log_and_library("ctl_log", false)
);

/// Paths of each form `canonicalize` gives, or a tool may: Windows' verbatim drive and UNC forms, the verbatim forms
/// that keep their prefix, and plain Windows and Unix paths. Literals, so the check runs alike on every platform.
const PATHS: [&str; 10] = [
    r"\\?\E:\x",
    r"\\?\e:\a\b",
    r"\\?\UNC\server\share\x",
    r"\\?\E:",
    r"\\?\1:\x",
    r"\\?\Volume{0}\a",
    r"\\?\GLOBALROOT\x",
    r"E:\x",
    r"\\server\share",
    "/private/tmp/x",
];

/// The stand-in logs its directory in the form build_kernel.rs's `check_backend_build` expects,
/// `validation::paths::plain`'s, on each of [`PATHS`]. `stripped` is the stand-in's form, given to the check.
fn check_plain_matches_validation(stripped: fn(&Path) -> PathBuf) {
    for path in PATHS {
        let path = Path::new(path);
        assert_eq!(
            stripped(path),
            validation::paths::plain(path),
            "the stand-in's logged form of {} is not validation's",
            path.display()
        );
    }
    assert_eq!(stripped(Path::new(PATHS[0])), Path::new(r"E:\x"));
}

#[test]
fn stand_in_logs_the_plain_form() {
    check_plain_matches_validation(plain::plain);
}

validation::negative_control!(
    stand_in_logs_the_plain_form,
    "the raw `canonicalize` form, verbatim prefix and all, given to the check as the stand-in's",
    expected = "the stand-in's logged form of",
    check_plain_matches_validation(Path::to_path_buf)
);
