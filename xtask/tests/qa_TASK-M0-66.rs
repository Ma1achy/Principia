//! QA's tests for TASK-M0-66, written from REQ-TOOL-238: "Every test that stands in for `cargo` with an executable of
//! its own must run on every platform the tests run on, Windows included, through a binary built with the tests rather
//! than a shell script run as an executable". Read here as four claims, each checked from outside the binary:
//!
//! - the four xtask files that stood in for `cargo` (build_kernel.rs, ci.rs, controls_partition.rs, ci_partition.rs)
//!   write no shell script as their stand-in, and each sets up the shared stand-in binary;
//! - the stand-in copied into a test's directory is a native executable (Mach-O, ELF or PE), not a `#!` script, so
//!   the platform's loader runs it, Windows's included;
//! - the stand-in reads its configuration beside itself, not from the working directory xtask runs it in, and hands a
//!   golden-harness call, every argument kept, to the real cargo, exiting with its status;
//! - `cargo xtask` still runs xtask: the package's default binary is `xtask`, the stand-in a second binary.
//!
//! Each test has its negative control (R-176): the check pointed at an input it must reject. A test and its control
//! run at once under nextest, so each works in directories of its own, named by its `case`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

/// The xtask package's directory.
fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh directory of this file's own under the target's scratch directory.
fn scratch(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m0_66_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The built stand-in copied into `dir` as `cargo` with the platform's suffix, beside `config` as `stand_in.json`.
fn copy_stand_in(dir: &Path, config: &Value) -> PathBuf {
    fs::write(dir.join("stand_in.json"), config.to_string()).unwrap();
    let cargo = dir.join(format!("cargo{}", std::env::consts::EXE_SUFFIX));
    write_executable(&cargo, fs::read(env!("CARGO_BIN_EXE_stand_in")).unwrap()).unwrap();
    cargo
}

// --- the four files' stand-in is no shell script --------------------------------------------------------------------

const FOUR: [&str; 4] = [
    "tests/build_kernel.rs",
    "tests/ci.rs",
    "tests/controls_partition.rs",
    "tests/ci_partition.rs",
];

/// Each of `sources` (name, text) writes no `#!` script and sets up the shared stand-in binary.
fn check_no_shell_stand_in(sources: &[(&str, String)]) {
    for (name, text) in sources {
        assert!(
            !text.contains("#!/bin/sh") && !text.contains("#!/usr/bin/env"),
            "{name} still writes a shell script as its stand-in cargo"
        );
        assert!(
            text.contains("stand_in::stand_in_cargo("),
            "{name} does not set up the stand-in binary"
        );
    }
}

fn four_sources() -> Vec<(&'static str, String)> {
    FOUR.iter()
        .map(|f| (*f, fs::read_to_string(package().join(f)).unwrap()))
        .collect()
}

#[test]
fn qa_four_files_stand_in_through_the_binary() {
    check_no_shell_stand_in(&four_sources());
}

negative_control!(
    qa_four_files_stand_in_through_the_binary,
    "one file's stand-in written back as the shell script it was",
    expected = "still writes a shell script as its stand-in cargo",
    {
        let mut sources = four_sources();
        sources[2]
            .1
            .push_str("\nconst OLD: &str = \"#!/bin/sh\\nexit 0\\n\";\n");
        check_no_shell_stand_in(&sources)
    }
);

// --- the copy is a native executable ----------------------------------------------------------------------------------

/// `bytes` start with a native executable's magic: Mach-O (32 or 64 bit, either order, or fat), ELF, or PE's `MZ`.
fn check_native(bytes: &[u8]) {
    let head = &bytes[..bytes.len().min(4)];
    let native = [
        &[0xcf, 0xfa, 0xed, 0xfe][..],
        &[0xfe, 0xed, 0xfa, 0xcf][..],
        &[0xce, 0xfa, 0xed, 0xfe][..],
        &[0xfe, 0xed, 0xfa, 0xce][..],
        &[0xca, 0xfe, 0xba, 0xbe][..],
        &[0x7f, b'E', b'L', b'F'][..],
    ]
    .contains(&head)
        || head.starts_with(b"MZ");
    assert!(
        native && !head.starts_with(b"#!"),
        "the stand-in is not a native executable: it starts {head:02x?}"
    );
}

#[test]
fn qa_the_copied_stand_in_is_native() {
    let dir = scratch("native");
    let cargo = copy_stand_in(&dir, &json!({}));
    check_native(&fs::read(cargo).unwrap());
}

negative_control!(
    qa_the_copied_stand_in_is_native,
    "a shell script, which Windows refuses to run (os error 193)",
    expected = "the stand-in is not a native executable",
    check_native(b"#!/bin/sh\necho hi\n")
);

// --- configuration beside the binary; the golden harness handed to the real cargo ------------------------------------

/// The stand-in, its `stand_in.json` asking for status 4, run from another directory holding a `stand_in.json` that
/// asks for 9, exits `want`.
fn check_config_beside_itself(case: &str, want: i32) {
    let dir = scratch(&format!("{case}_beside"));
    let elsewhere = scratch(&format!("{case}_elsewhere"));
    fs::write(
        elsewhere.join("stand_in.json"),
        json!({"status": 9}).to_string(),
    )
    .unwrap();
    let cargo = copy_stand_in(
        &dir,
        &json!({"status": 4, "log": dir.join("calls.log"), "log_cwd": true}),
    );
    let out = Command::new(&cargo)
        .args(["build", "-p", "x"])
        .current_dir(&elsewhere)
        .timed_output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(want),
        "the stand-in did not read the configuration beside itself"
    );
    // And it logged where it was run, the other directory, resolved and without Windows' verbatim prefix, as
    // `pwd -P` prints it.
    let log = fs::read_to_string(dir.join("calls.log")).unwrap();
    assert_eq!(
        log,
        format!(
            "{} build -p x\n",
            validation::paths::plain(&elsewhere.canonicalize().unwrap()).display()
        ),
        "the stand-in did not log the directory it was run in"
    );
}

#[test]
fn qa_the_stand_in_reads_its_configuration_beside_itself() {
    check_config_beside_itself("test", 4);
}

negative_control!(
    qa_the_stand_in_reads_its_configuration_beside_itself,
    "the working directory's configuration taken as the one that counts",
    expected = "the stand-in did not read the configuration beside itself",
    check_config_beside_itself("ctl", 9)
);

/// An outer stand-in whose `real` cargo is an inner stand-in (status 5, logging its calls), called with `args`: the
/// outer's exit code and the inner's log, if it was called.
fn handed(case: &str, args: &[&str]) -> (Option<i32>, Option<String>) {
    let inner_dir = scratch(&format!("{case}_inner"));
    let inner = copy_stand_in(
        &inner_dir,
        &json!({"log": inner_dir.join("calls.log"), "status": 5}),
    );
    let outer_dir = scratch(&format!("{case}_outer"));
    let outer = copy_stand_in(
        &outer_dir,
        &json!({"real": inner, "output": "outer\n", "status": 0}),
    );
    let out = Command::new(outer)
        .args(args)
        .current_dir(&outer_dir)
        .timed_output()
        .unwrap();
    (
        out.status.code(),
        fs::read_to_string(inner_dir.join("calls.log")).ok(),
    )
}

/// A call naming the golden harness reaches the real cargo with every argument, and the stand-in exits with its status;
/// a call not naming it, `other`, never reaches it.
fn check_handed(case: &str, other: &[&str]) {
    let args = ["test", "-p", "golden_harness", "--", "--nocapture"];
    assert_eq!(
        handed(&format!("{case}_golden"), &args),
        (Some(5), Some(format!("{}\n", args.join(" ")))),
        "the golden-harness call was not handed to the real cargo as it was made"
    );
    let (code, log) = handed(&format!("{case}_not_golden"), other);
    assert_eq!(
        (code, log),
        (Some(0), None),
        "a call not naming the golden harness reached the real cargo"
    );
}

#[test]
fn qa_the_golden_harness_goes_to_the_real_cargo() {
    check_handed("test", &["test", "-p", "kernel"]);
}

negative_control!(
    qa_the_golden_harness_goes_to_the_real_cargo,
    "a golden-harness call given to the check as one that must not be handed on",
    expected = "a call not naming the golden harness reached the real cargo",
    check_handed("ctl", &["run", "-p", "golden_harness"])
);

// --- `cargo xtask` still runs xtask ------------------------------------------------------------------------------------

/// `metadata`'s xtask package runs `xtask` by default and builds `stand_in` as a second binary.
fn check_default_run(metadata: &Value) {
    let xtask = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "xtask")
        .expect("the xtask package");
    assert_eq!(
        xtask["default_run"], "xtask",
        "`cargo xtask` would not run xtask"
    );
    let bins: Vec<&str> = xtask["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["kind"].as_array().unwrap().iter().any(|k| k == "bin"))
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        bins.contains(&"xtask") && bins.contains(&"stand_in"),
        "xtask's binaries are {bins:?}"
    );
}

fn metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ])
        .current_dir(package())
        .timed_output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn qa_cargo_xtask_still_runs_xtask() {
    check_default_run(&metadata());
}

negative_control!(
    qa_cargo_xtask_still_runs_xtask,
    "the package without its default binary",
    expected = "`cargo xtask` would not run xtask",
    {
        let mut m = metadata();
        for p in m["packages"].as_array_mut().unwrap() {
            p["default_run"] = Value::Null;
        }
        check_default_run(&m)
    }
);
