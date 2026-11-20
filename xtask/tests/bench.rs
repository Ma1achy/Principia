//! `cargo xtask bench` (TASK-M0-19; telemetry §1.1, R-186) through a stand-in `cargo`: `--all` runs each bench
//! validation's `bench --list` names, writing its trace under `target/bench/`; `--bless` writes each trace as its
//! baseline; the run fails when the list, a bench or the baseline is missing or failed; and `run` builds the kernel
//! before any bench. Each test registers the control that must make it fail (R-176).

#![cfg(unix)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::bench::{baseline_path, result_path, run, run_built, Which};

/// How the stand-in `cargo` answers: what `bench --list` prints and exits with, and the exit of a bench run and of
/// `prin profile diff`.
#[derive(Clone, Copy)]
struct Answers {
    list: &'static str,
    list_exit: u8,
    bench_exit: u8,
    diff_exit: u8,
}

const OK: Answers = Answers {
    list: "a\nb\n",
    list_exit: 0,
    bench_exit: 0,
    diff_exit: 0,
};

/// A scratch workspace `case` with a stand-in `cargo` answering as `answers` says, logging each call's arguments. A
/// bench run writes a profiler schema v1 header line to the file after `--out`. Returns (workspace root, stand-in).
fn stand_in(case: &str, answers: Answers) -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("bench_{case}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("scratch workspace made");
    let cargo = root.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            r#"#!/bin/sh
echo "$*" >> '{root}/calls.log'
prev=
for a in "$@"; do
  case "$a" in
    --list) printf '{list}'; exit {list_exit} ;;
    diff) exit {diff_exit} ;;
  esac
  if [ "$prev" = --out ]; then printf '{{"schema":"principia-profile-v1"}}\n' > "$a"; fi
  prev=$a
done
exit {bench_exit}
"#,
            root = root.display(),
            list = answers.list.replace('\n', "\\n"),
            list_exit = answers.list_exit,
            diff_exit = answers.diff_exit,
            bench_exit = answers.bench_exit,
        ),
    )
    .expect("stand-in cargo written");
    (root, cargo)
}

/// Runs `spawn`, which spawns the stand-in from xtask's own code and so not through `validation::spawn::Spawn`, while
/// no other test writes a stand-in: a child forked as another thread holds an executable open for writing inherits
/// that descriptor until it execs, and an exec of that executable meanwhile fails with ETXTBSY (REQ-SYS-070).
fn no_write<T>(spawn: impl FnOnce() -> T) -> T {
    validation::spawn::while_no_spawn(spawn)
}

/// `run_built` on the scratch workspace `case`: its result, and the benches the stand-in was asked to run.
fn bench(
    case: &str,
    answers: Answers,
    which: Which<'_>,
    bless: bool,
) -> (Result<(), String>, PathBuf, Vec<String>) {
    let (root, cargo) = stand_in(case, answers);
    let result = no_write(|| run_built(cargo.as_os_str(), &root.join("Cargo.toml"), which, bless));
    let ran = fs::read_to_string(root.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|call| {
            let (_, rest) = call.split_once(&format!("--root {} ", root.display()))?;
            Some(
                rest.split_once(" --out")
                    .map_or(rest, |(name, _)| name)
                    .to_owned(),
            )
        })
        .collect();
    (result, root, ran)
}

/// `--all --bless` runs each listed bench, writing its trace and its baseline.
fn check_all(case: &str, answers: Answers) {
    let (result, root, ran) = bench(case, answers, Which::All, true);
    assert_eq!(ran, ["a", "b"], "bench --all did not run each listed bench");
    assert!(result.is_ok(), "bench --all failed: {result:?}");
    for name in ["a", "b"] {
        assert_eq!(
            fs::read(result_path(&root, name)).ok(),
            fs::read(baseline_path(&root, name)).ok(),
            "bench {name}'s baseline is not its trace"
        );
    }
}

#[test]
fn bench_all_runs_each_listed_bench() {
    check_all("all", OK);
}

negative_control!(
    bench_all_runs_each_listed_bench,
    "a list naming no bench runs none",
    expected = "bench --all did not run each listed bench",
    check_all("all_control", Answers { list: "", ..OK })
);

/// The run fails, saying `why`.
fn check_fails(case: &str, answers: Answers, which: Which<'_>, bless: bool, why: &str) {
    let (result, _, _) = bench(case, answers, which, bless);
    assert!(
        result.as_ref().is_err_and(|e| e.contains(why)),
        "bench did not fail saying `{why}`: {result:?}"
    );
}

#[test]
fn bench_fails_when_the_list_fails() {
    check_fails(
        "list_fails",
        Answers { list_exit: 1, ..OK },
        Which::All,
        true,
        "bench --list failed",
    );
}

negative_control!(
    bench_fails_when_the_list_fails,
    "a list that succeeds",
    expected = "bench did not fail saying",
    check_fails(
        "list_fails_control",
        OK,
        Which::All,
        true,
        "bench --list failed"
    )
);

#[test]
fn bench_fails_when_a_bench_fails() {
    check_fails(
        "bench_fails",
        Answers {
            bench_exit: 1,
            ..OK
        },
        Which::One("a"),
        true,
        "bench a failed",
    );
}

negative_control!(
    bench_fails_when_a_bench_fails,
    "a bench that succeeds",
    expected = "bench did not fail saying",
    check_fails(
        "bench_fails_control",
        OK,
        Which::One("a"),
        true,
        "bench a failed"
    )
);

#[test]
fn bench_fails_without_a_baseline() {
    check_fails(
        "no_baseline",
        OK,
        Which::One("a"),
        false,
        "bench a: no baseline at",
    );
}

negative_control!(
    bench_fails_without_a_baseline,
    "--bless writes the baseline",
    expected = "bench did not fail saying",
    check_fails(
        "no_baseline_control",
        OK,
        Which::One("a"),
        true,
        "bench a: no baseline at"
    )
);

#[test]
fn bench_fails_when_the_diff_fails() {
    check_fails(
        "diff_fails",
        Answers { diff_exit: 2, ..OK },
        Which::One("a"),
        true,
        "prin profile diff failed (exit 2)",
    );
}

negative_control!(
    bench_fails_when_the_diff_fails,
    "a diff that reports a rise passes",
    expected = "bench did not fail saying",
    check_fails(
        "diff_fails_control",
        Answers { diff_exit: 1, ..OK },
        Which::One("a"),
        true,
        "prin profile diff failed"
    )
);

/// `bench` builds the kernel first: on a workspace with no `rust-toolchain.toml`, that build fails, and so the run.
fn check_builds_kernel(case: &str, bench: impl Fn(&OsStr, &Path) -> Result<(), String>) {
    let (root, cargo) = stand_in(case, OK);
    let result = no_write(|| bench(cargo.as_os_str(), &root.join("Cargo.toml")));
    assert!(
        result
            .as_ref()
            .is_err_and(|e| e.contains("rust-toolchain.toml")),
        "bench ran without building the kernel: {result:?}"
    );
}

#[test]
fn bench_builds_the_kernel_first() {
    check_builds_kernel("kernel", |_, manifest| run(manifest, Which::One("a"), true));
}

negative_control!(
    bench_builds_the_kernel_first,
    "the run that skips the kernel build passes",
    expected = "bench ran without building the kernel",
    check_builds_kernel("kernel_control", |cargo, manifest| run_built(
        cargo,
        manifest,
        Which::One("a"),
        true
    ))
);
