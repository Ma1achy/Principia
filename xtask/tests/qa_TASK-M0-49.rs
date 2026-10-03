//! QA tests for TASK-M0-49, written from REQ-VAL-179, REQ-VAL-180 and REQ-VAL-181 (R-348, R-352), not from the
//! implementation:
//!
//! - REQ-VAL-179: "Every `cargo mutants` run, the per-PR shards (REQ-VAL-148), the nightly full run (REQ-VAL-150) and a
//!   run made locally, must run under two caps: a per-mutant timeout set through cargo-mutants' timeout setting
//!   (REQ-VAL-180) ... and a per-process memory cap on the test processes, set with `ulimit -v` or `prlimit`
//!   (REQ-VAL-181), so that a runaway allocation kills one test process, not cargo-mutants or the runner ... CI's Linux
//!   runners enforce both caps, and on macOS a local run gets the timeout only (R-352)." verify (code): "both read from
//!   one place, marked provisional (R-182, R-348)".
//! - REQ-VAL-180: "the timeout, in the form cargo-mutants' timeout setting takes it (seconds, or a multiple of the
//!   baseline's test time with its floor) ... every run uses it, marked provisional until the human confirms it".
//! - REQ-VAL-181: "the cap, in the unit `ulimit -v` or `prlimit` takes ... CI's Linux runners enforce it; on macOS a
//!   local run gets the timeout only, without this cap (R-352)."
//!
//! What these tests hold to:
//! - the timeout lives in `.cargo/mutants.toml`, the file a `cargo mutants` run in the workspace reads by default, at
//!   its top level (so no platform scopes it), as a multiple of the baseline with its floor, marked confirmed by the
//!   human at the M0 gate (R-376; it was marked provisional until then, R-182);
//! - the memory cap, run as cargo-mutants runs a test (`cargo test` with the config's extra cargo and test arguments),
//!   binds the test process on Linux, where a reservation the size of the machine's memory is refused, and on macOS
//!   leaves the test to run without it (no `prlimit` there);
//! - every `cargo mutants` call in CI's workflows reads that one file: none points it at another config, turns the
//!   config off or overrides the timeout, and none runs on another directory (whose own `.cargo/mutants.toml` it
//!   would read) without `--config .cargo/mutants.toml`.
//!
//! The values themselves are calibrations (REQ-VAL-180, REQ-VAL-181), which the human confirmed at the M0 gate as they
//! stood (R-376); no test here pins them. Each test's control (R-176) feeds the same assertion an input differing in the one respect the
//! requirement turns on, and trips it by its message.
// The file name `qa_TASK-M0-49` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn mutants_toml() -> String {
    std::fs::read_to_string(root().join(".cargo/mutants.toml")).expect("qa49: .cargo/mutants.toml")
}

/// A fresh directory for one call: tests and their controls run in parallel, so each call gets its own.
fn scratch(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("qa49_{}_{n}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("qa49: scratch dir");
    dir
}

// ---------------------------------------------------------------------------------------------------------------
// The per-mutant timeout: in the one file, unscoped, in cargo-mutants' form, marked confirmed (R-376).

/// The comment block directly above the line that starts with `key`.
fn comment_above(toml: &str, key: &str) -> String {
    let lines: Vec<&str> = toml.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with(key))
        .unwrap_or_else(|| panic!("qa49: no `{key}` line"));
    let mut block: Vec<&str> = lines[..at]
        .iter()
        .rev()
        .take_while(|l| l.starts_with('#') || l.starts_with(char::is_alphanumeric))
        .copied()
        .collect();
    block.reverse();
    block.join("\n")
}

fn positive(doc: &toml_edit::DocumentMut, key: &str) -> f64 {
    let item = doc.get(key).unwrap_or_else(|| {
        panic!("qa49: the timeout is not set: no top-level `{key}` in .cargo/mutants.toml")
    });
    let value = item
        .as_float()
        .or_else(|| item.as_integer().map(|i| i as f64))
        .unwrap_or_else(|| panic!("qa49: the timeout is not set: `{key}` is not a number"));
    assert!(
        value > 0.0,
        "qa49: the timeout is not set: `{key}` = {value} is not positive"
    );
    value
}

fn timeout_is_set(toml: &str) {
    let doc: toml_edit::DocumentMut = toml.parse().expect("qa49: .cargo/mutants.toml is not TOML");
    // REQ-VAL-180's form: a multiple of the baseline's test time with its floor. cargo-mutants rejects an unknown key
    // in this file, so the names are its own; at the top level, no platform table scopes them (R-352: the Mac gets
    // the timeout too).
    positive(&doc, "timeout_multiplier");
    positive(&doc, "minimum_test_timeout");
    let comment = comment_above(toml, "timeout_multiplier");
    for (needle, what) in [
        ("REQ-VAL-180", "its calibration requirement"),
        ("R-348", "its ruling"),
        ("confirmed", "that it is confirmed"),
        ("R-376", "the ruling confirming it"),
    ] {
        assert!(
            comment.contains(needle),
            "qa49: the timeout is not marked: its comment does not give {what} ({needle})"
        );
    }
    assert!(
        !comment.contains("provisional"),
        "qa49: the timeout is not marked: its comment still calls it provisional, though R-376 confirmed it"
    );
}

#[test]
fn qa49_the_timeout_is_in_the_one_file_every_run_reads() {
    timeout_is_set(&mutants_toml());
}

negative_control!(
    qa49_the_timeout_is_in_the_one_file_every_run_reads,
    "the timeout's floor removed: a multiple alone, with no floor",
    expected = "qa49: the timeout is not set",
    timeout_is_set(&mutants_toml().replace("minimum_test_timeout", "# minimum_test_timeout"))
);

mod scoped {
    use super::*;
    negative_control!(
        qa49_the_timeout_is_in_the_one_file_every_run_reads,
        "the timeout moved into a Linux-only table, so a Mac run would not get it",
        expected = "qa49: the timeout is not set",
        timeout_is_set(&mutants_toml().replace(
            "\ntimeout_multiplier =",
            "\n[linux_only]\ntimeout_multiplier ="
        ))
    );
}

mod unmarked {
    use super::*;
    negative_control!(
        qa49_the_timeout_is_in_the_one_file_every_run_reads,
        "the timeout's comment no longer says it is confirmed",
        expected =
            "qa49: the timeout is not marked: its comment does not give that it is confirmed",
        timeout_is_set(&{
            let toml = mutants_toml();
            let comment = comment_above(&toml, "timeout_multiplier");
            assert!(
                comment.contains("confirmed"),
                "qa49: the control's edit target is gone"
            );
            toml.replace(&comment, &comment.replace("confirmed", "settled"))
        })
    );
}

mod still_provisional {
    use super::*;
    negative_control!(
        qa49_the_timeout_is_in_the_one_file_every_run_reads,
        "the timeout's comment back to its pre-R-376 form, provisional until the M0 gate",
        expected = "qa49: the timeout is not marked: its comment still calls it provisional",
        timeout_is_set(&{
            let toml = mutants_toml();
            let comment = comment_above(&toml, "timeout_multiplier");
            let from = "confirmed by the human at the M0 gate, R-376, R-71";
            assert!(
                comment.contains(from),
                "qa49: the control's edit target is gone"
            );
            toml.replace(
                &comment,
                &comment.replace(
                    from,
                    "provisional until the human confirms it at the M0 gate, R-71, R-182; confirmed, R-376",
                ),
            )
        })
    );
}

// ---------------------------------------------------------------------------------------------------------------
// The memory cap: run as cargo-mutants runs a test, it binds the test process on Linux and is absent on macOS.

/// The config's string array `key` (cargo-mutants' extra arguments), empty if it is not set.
fn args(toml: &str, key: &str) -> Vec<String> {
    let doc: toml_edit::DocumentMut = toml.parse().expect("qa49: .cargo/mutants.toml is not TOML");
    match doc.get(key) {
        None => Vec::new(),
        Some(item) => item
            .as_array()
            .unwrap_or_else(|| panic!("qa49: `{key}` is not an array"))
            .iter()
            .map(|v| {
                v.as_str()
                    .unwrap_or_else(|| panic!("qa49: `{key}` holds a non-string"))
                    .to_owned()
            })
            .collect(),
    }
}

const PROBE_MANIFEST: &str = r#"[package]
name = "qa49_probe"
version = "0.1.0"
edition = "2021"
publish = false

[workspace]
"#;

/// The probe test: it records that it ran, its own address-space limit, and whether a reservation the size of the
/// machine's memory (`QA49_RESERVE` bytes, never touched) is refused.
const PROBE_LIB: &str = r#"
#[test]
fn probe() {
    let dir = std::path::PathBuf::from(std::env::var("QA49_DIR").expect("QA49_DIR"));
    let limits = std::fs::read_to_string("/proc/self/limits").unwrap_or_default();
    let reserve: usize = std::env::var("QA49_RESERVE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let refused = if reserve == 0 {
        "n/a".to_owned()
    } else {
        let mut v: Vec<u8> = Vec::new();
        v.try_reserve_exact(reserve).is_err().to_string()
    };
    std::fs::write(dir.join("probe.out"), format!("refused={refused}\n{limits}")).expect("probe.out");
}
"#;

/// The soft limit on the address space in a `/proc/<pid>/limits` text, `None` if unlimited.
fn soft_address_space(limits: &str) -> Option<u64> {
    let line = limits
        .lines()
        .find(|l| l.starts_with("Max address space"))
        .expect("qa49: no address-space line in /proc/self/limits");
    let soft = line.split_whitespace().nth(3).expect("qa49: no soft limit");
    if soft == "unlimited" {
        None
    } else {
        Some(soft.parse().expect("qa49: soft limit is not a number"))
    }
}

fn mem_total_bytes() -> u64 {
    let meminfo = std::fs::read_to_string("/proc/meminfo").expect("qa49: /proc/meminfo");
    let kib: u64 = meminfo
        .lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .expect("qa49: no MemTotal in /proc/meminfo");
    kib * 1024
}

/// Runs a probe crate's test as cargo-mutants runs one, with `toml`'s extra arguments: `cargo test --no-run` with
/// `additional_cargo_args` (its build phase), then `cargo test` with those and `additional_cargo_test_args` (its test
/// phase). On Linux the probe's test process must run under a finite address-space cap that refuses a reservation the
/// size of the machine's memory; on macOS it must run, without `prlimit`, which the Mac does not have (R-352).
fn cap_binds_where_enforced(toml: &str) {
    const FAIL: &str = "qa49: the memory cap does not bind the test process where it is enforced";
    let dir = scratch("probe");
    std::fs::create_dir_all(dir.join("src")).expect("qa49: probe src");
    std::fs::write(dir.join("Cargo.toml"), PROBE_MANIFEST).expect("qa49: probe manifest");
    std::fs::write(dir.join("src/lib.rs"), PROBE_LIB).expect("qa49: probe lib");
    let linux = cfg!(target_os = "linux");
    let reserve = if linux { mem_total_bytes() } else { 0 };
    let cargo_args = args(toml, "additional_cargo_args");
    let test_args = args(toml, "additional_cargo_test_args");
    let run = |phase: &[&str], extra: &[String]| {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("test")
            .args(phase)
            .args(extra)
            .arg("--target-dir")
            .arg(dir.join("target"))
            .current_dir(&dir)
            .env("QA49_DIR", &dir)
            .env("QA49_RESERVE", reserve.to_string())
            .env_remove("CARGO_TARGET_DIR");
        cmd.timed_output().expect("qa49: cargo ran")
    };
    let build = run(&["--no-run"], &cargo_args);
    assert!(
        build.status.success(),
        "qa49: the probe does not build: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let all: Vec<String> = cargo_args.iter().chain(test_args.iter()).cloned().collect();
    let test = run(&[], &all);
    let stderr = String::from_utf8_lossy(&test.stderr);
    assert!(
        test.status.success(),
        "{FAIL}: the probe's test did not run ({}): {stderr}",
        test.status
    );
    let probe = std::fs::read_to_string(dir.join("probe.out"))
        .unwrap_or_else(|_| panic!("{FAIL}: the probe's test did not record itself: {stderr}"));
    if linux {
        let cap = soft_address_space(&probe);
        assert!(
            cap.is_some(),
            "{FAIL}: the test process's address space is unlimited"
        );
        assert!(
            probe.starts_with("refused=true"),
            "{FAIL}: a reservation of the machine's memory ({reserve} bytes) is not refused under cap {cap:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn qa49_the_memory_cap_binds_the_test_process_where_enforced() {
    cap_binds_where_enforced(&mutants_toml());
}

// The runner scoped to the other platform, macOS: on Linux the test then runs uncapped, and on macOS cargo is asked to
// run a `prlimit` the Mac does not have.
negative_control!(
    qa49_the_memory_cap_binds_the_test_process_where_enforced,
    "the cap scoped to macOS instead of Linux",
    expected = "qa49: the memory cap does not bind the test process where it is enforced",
    cap_binds_where_enforced(&{
        let toml = mutants_toml();
        assert!(
            toml.contains(r#"target_os = \"linux\""#),
            "qa49 control: the config does not scope its runner to Linux"
        );
        toml.replace(r#"target_os = \"linux\""#, r#"target_os = \"macos\""#)
    })
);

// ---------------------------------------------------------------------------------------------------------------
// Every `cargo mutants` call in CI reads the one file.

/// The `cargo mutants` command lines in a workflow, continuation lines joined; comments, step names and echoed text
/// left out.
fn invocations(yml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    for line in yml.lines() {
        let t = line.trim();
        if let Some(mut cmd) = pending.take() {
            cmd.push(' ');
            cmd.push_str(t.trim_end_matches('\\'));
            if t.ends_with('\\') {
                pending = Some(cmd);
            } else {
                out.push(cmd);
            }
            continue;
        }
        if t.starts_with('#')
            || t.starts_with("- name:")
            || t.starts_with("name:")
            || t.contains("echo ")
        {
            continue;
        }
        if let Some(at) = t.find("cargo mutants ") {
            let cmd = t[at..].trim_end_matches('\\').to_owned();
            if t.ends_with('\\') {
                pending = Some(cmd);
            } else {
                out.push(cmd);
            }
        }
    }
    out
}

fn every_run_reads_the_caps(workflows: &[(String, String)]) {
    let mut found = 0;
    for (file, yml) in workflows {
        for cmd in invocations(yml) {
            found += 1;
            let words: Vec<&str> = cmd.split_whitespace().collect();
            for banned in [
                "--no-config",
                "--timeout",
                "--timeout-multiplier",
                "--minimum-test-timeout",
            ] {
                assert!(
                    !words.iter().any(|w| *w == banned || w.starts_with(&format!("{banned}="))),
                    "qa49: a cargo mutants run does not read the caps from .cargo/mutants.toml ({file}: `{banned}`): {cmd}"
                );
            }
            let config = words
                .iter()
                .position(|w| *w == "--config")
                .map(|i| words.get(i + 1).copied().unwrap_or(""));
            match config {
                Some(path) => assert!(
                    path.trim_matches('"') == ".cargo/mutants.toml",
                    "qa49: a cargo mutants run does not read the caps from .cargo/mutants.toml ({file}: --config {path}): {cmd}"
                ),
                None => assert!(
                    !words.iter().any(|w| *w == "--dir" || w.starts_with("--dir=") || *w == "-d"),
                    "qa49: a cargo mutants run does not read the caps from .cargo/mutants.toml ({file}: another --dir, no --config): {cmd}"
                ),
            }
        }
    }
    assert!(
        found > 0,
        "qa49: no cargo mutants run found in the workflows"
    );
}

fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("qa49: .github/workflows")
        .map(|e| e.expect("qa49: entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("qa49: workflow"),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn qa49_every_cargo_mutants_run_in_ci_reads_the_caps() {
    every_run_reads_the_caps(&workflows());
}

negative_control!(
    qa49_every_cargo_mutants_run_in_ci_reads_the_caps,
    "a run on a fixture directory without `--config .cargo/mutants.toml`",
    expected = "qa49: a cargo mutants run does not read the caps from .cargo/mutants.toml",
    every_run_reads_the_caps(&[(
        "control.yml".to_owned(),
        "      - run: |\n          cargo mutants --dir \"$fixture\" --in-diff \"$fixture/change.diff\" \\\n            -o out\n"
            .to_owned()
    )])
);

mod overridden {
    use super::*;
    negative_control!(
        qa49_every_cargo_mutants_run_in_ci_reads_the_caps,
        "a shard that overrides the timeout on its command line",
        expected = "qa49: a cargo mutants run does not read the caps from .cargo/mutants.toml",
        every_run_reads_the_caps(&[(
            "control.yml".to_owned(),
            "      - run: |\n          cargo mutants --in-place --in-diff pr.diff --timeout 9999 -o out\n".to_owned()
        )])
    );
}
