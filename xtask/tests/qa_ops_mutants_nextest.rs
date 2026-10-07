//! qa's tests for the ops PR #156 (branch `ops/mutants-test-selection`, R-388 item 4 as amended by R-393): every
//! `cargo mutants` run tests each mutant through cargo-nextest (`test_tool = "nextest"` in `.cargo/mutants.toml`).
//! Written from the requirements the change must keep, not from the config:
//!
//! - REQ-VAL-148: "every mutant that survives in the PR's diff is a qa finding ... The job must fail on any surviving
//!   mutant not in a checked-in list". cargo-mutants counts a mutant as caught when its test run exits non-zero, so a
//!   package whose tests cannot fail, one with no tests, must pass its test run: its mutants then survive and are
//!   named. nextest, by default, fails a run that finds no tests.
//! - REQ-VAL-179: "a per-mutant timeout ... so that a mutant whose tests hang is recorded as a timeout and the shard
//!   finishes, never a dead shard". cargo-mutants ends a timed-out test run by sending SIGTERM to its process group
//!   and then waits for it; nextest runs each test in a process group of its own, so the hung test is outside the
//!   group the signal reaches. The test run must still exit, and the hung test process with it.
//! - REQ-VAL-179 (every run: the per-PR shards, the nightly run, the fixture runs): a run whose config names nextest
//!   needs cargo-nextest where it runs, so each CI job that runs `cargo mutants` installs it.
//!
//! Each probe runs as cargo-mutants runs a test (`cargo nextest run`, or `cargo test`, with the config's
//! `additional_cargo_args` and `additional_cargo_test_args`), on a scratch crate of its own. Each test has its negative
//! control (R-176), which feeds the same assertion an input differing in the one respect the requirement turns on.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use validation::negative_control;
use validation::spawn::Spawn;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn mutants_toml() -> String {
    std::fs::read_to_string(root().join(".cargo/mutants.toml"))
        .expect("qa-ops: .cargo/mutants.toml")
}

/// A fresh directory for one call: tests and their controls run in parallel, so each call gets its own.
fn scratch(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("qa_ops_nextest_{}_{n}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).expect("qa-ops: scratch dir");
    dir
}

/// The config's string array `key`, empty if it is not set.
fn args(toml: &str, key: &str) -> Vec<String> {
    let doc: toml_edit::DocumentMut = toml
        .parse()
        .expect("qa-ops: .cargo/mutants.toml is not TOML");
    match doc.get(key) {
        None => Vec::new(),
        Some(item) => item
            .as_array()
            .unwrap_or_else(|| panic!("qa-ops: `{key}` is not an array"))
            .iter()
            .map(|v| {
                v.as_str()
                    .expect("qa-ops: a non-string argument")
                    .to_owned()
            })
            .collect(),
    }
}

/// Whether the config has cargo-mutants test through cargo-nextest.
fn uses_nextest(toml: &str) -> bool {
    let doc: toml_edit::DocumentMut = toml
        .parse()
        .expect("qa-ops: .cargo/mutants.toml is not TOML");
    match doc.get("test_tool").map(|v| v.as_str()) {
        None | Some(Some("cargo")) => false,
        Some(Some("nextest")) => true,
        Some(other) => panic!("qa-ops: `test_tool` is not one cargo-mutants takes: {other:?}"),
    }
}

/// A command for the probe crate in `dir` as cargo-mutants runs its test phase (`phase` empty) or build phase
/// (`["--no-run"]`): the config's test tool, `additional_cargo_args`, and in the test phase `additional_cargo_test_args`.
/// nextest's variables from the run this test is part of are cleared, as a `cargo mutants` run starts without them.
fn as_mutants_runs(toml: &str, dir: &Path, build: bool) -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("NEXTEST") {
            cmd.env_remove(key);
        }
    }
    if uses_nextest(toml) {
        cmd.args(["nextest", "run"]);
    } else {
        cmd.arg("test");
    }
    if build {
        cmd.arg("--no-run");
    }
    cmd.args(args(toml, "additional_cargo_args"));
    if !build {
        cmd.args(args(toml, "additional_cargo_test_args"));
    }
    cmd.arg("--target-dir")
        .arg(dir.join("target"))
        .current_dir(dir)
        .env("QA_OPS_DIR", dir)
        .env_remove("CARGO_TARGET_DIR");
    cmd
}

fn write_probe(dir: &Path, name: &str, lib: &str) {
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n"
        ),
    )
    .expect("qa-ops: probe manifest");
    std::fs::write(dir.join("src/lib.rs"), lib).expect("qa-ops: probe lib");
}

fn build_probe(toml: &str, dir: &Path) {
    let build = as_mutants_runs(toml, dir, true)
        .timed_output()
        .expect("qa-ops: cargo ran");
    assert!(
        build.status.success(),
        "qa-ops: the probe does not build: {}",
        String::from_utf8_lossy(&build.stderr)
    );
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-148: a package with no tests passes its test run, so its mutants survive and are named, never caught.

fn untested_package_passes(toml: &str) {
    let dir = scratch("untested");
    write_probe(
        &dir,
        "qa_ops_untested",
        "pub fn sign(x: i32) -> i32 {\n    if x < 0 { -1 } else { 1 }\n}\n",
    );
    build_probe(toml, &dir);
    let run = as_mutants_runs(toml, &dir, false)
        .timed_output()
        .expect("qa-ops: cargo ran");
    assert!(
        run.status.success(),
        "qa-ops: a package with no tests fails its test run ({}), so cargo-mutants would count each of its mutants as \
         caught: {}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn qa_ops_a_package_without_tests_passes_its_mutants_test_run() {
    untested_package_passes(&mutants_toml());
}

// nextest's own default for a run that finds no tests: the config's `--no-tests` option taken out.
negative_control!(
    qa_ops_a_package_without_tests_passes_its_mutants_test_run,
    "the config's `--no-tests` option removed, so nextest's default (fail) applies",
    expected = "qa-ops: a package with no tests fails its test run",
    untested_package_passes(&{
        let toml = mutants_toml();
        let mut doc: toml_edit::DocumentMut = toml.parse().expect("qa-ops: TOML");
        let array = doc["additional_cargo_test_args"]
            .as_array_mut()
            .expect("qa-ops control: no additional_cargo_test_args");
        let before = array.len();
        array.retain(|v| !v.as_str().is_some_and(|s| s.starts_with("--no-tests")));
        assert!(
            array.len() < before,
            "qa-ops control: the config has no `--no-tests` option to remove"
        );
        doc.to_string()
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-179: the timeout's SIGTERM to the test run's process group ends the run and its hung test process.

const HANG_LIB: &str = r#"
#[test]
fn hangs() {
    let dir = std::path::PathBuf::from(std::env::var("QA_OPS_DIR").expect("QA_OPS_DIR"));
    std::fs::write(dir.join("hung.pid.tmp"), std::process::id().to_string()).expect("pid");
    std::fs::rename(dir.join("hung.pid.tmp"), dir.join("hung.pid")).expect("pid");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
"#;

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-s", "0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn signal(sig: &str, target: &str) {
    let _ = Command::new("kill")
        .args(["-s", sig, "--", target])
        .stderr(Stdio::null())
        .status();
}

/// Runs a probe whose test hangs as cargo-mutants runs it, in a process group of its own, and once the test is
/// running sends `sig` to that group, as cargo-mutants' timeout sends SIGTERM. The run must then exit, and the hung
/// test process be gone, within `within`.
fn timeout_ends_the_hung_test(toml: &str, sig: &str, within: Duration) {
    use std::os::unix::process::CommandExt;
    const FAIL: &str =
        "qa-ops: the timeout's signal does not end the test run and its hung test process";
    let dir = scratch("hang");
    write_probe(&dir, "qa_ops_hang", HANG_LIB);
    build_probe(toml, &dir);
    let mut child = as_mutants_runs(toml, &dir, false)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("qa-ops: cargo ran");
    let group = format!("-{}", child.id());
    let started = Instant::now();
    let pid_file = dir.join("hung.pid");
    while !pid_file.exists() {
        if let Some(status) = child.try_wait().expect("qa-ops: wait") {
            panic!("qa-ops: the hanging probe's run exited ({status}) before its test started");
        }
        if started.elapsed() > Duration::from_secs(300) {
            signal("KILL", &group);
            let _ = child.wait();
            panic!("qa-ops: the hanging probe's test did not start within 300 s");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let test_pid: u32 = std::fs::read_to_string(&pid_file)
        .expect("qa-ops: pid file")
        .trim()
        .parse()
        .expect("qa-ops: pid");
    signal(sig, &group);
    let deadline = Instant::now() + within;
    let mut exited = false;
    let mut gone = false;
    while Instant::now() < deadline {
        if !exited {
            exited = child.try_wait().expect("qa-ops: wait").is_some();
        }
        if exited {
            gone = !alive(test_pid);
            if gone {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // Whatever the outcome, nothing is left running.
    signal("KILL", &test_pid.to_string());
    if !exited {
        signal("KILL", &group);
    }
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        exited,
        "{FAIL}: the test run is still running {within:?} after SIGTERM to its process group (a dead shard)"
    );
    assert!(
        gone,
        "{FAIL}: the hung test process {test_pid} outlives its test run by {within:?}"
    );
}

#[test]
fn qa_ops_the_timeout_ends_the_test_run_and_its_hung_test() {
    timeout_ends_the_hung_test(&mutants_toml(), "TERM", Duration::from_secs(60));
}

// The same run sent a signal that ends nothing: the hung test keeps the run alive.
negative_control!(
    qa_ops_the_timeout_ends_the_test_run_and_its_hung_test,
    "SIGCONT in place of the timeout's SIGTERM",
    expected = "qa-ops: the timeout's signal does not end the test run and its hung test process",
    timeout_ends_the_hung_test(&mutants_toml(), "CONT", Duration::from_secs(10))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-179: every CI job that runs `cargo mutants` under a config naming nextest installs cargo-nextest.

/// The jobs of a workflow, each its name and its lines.
fn jobs(yml: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_jobs = false;
    for line in yml.lines() {
        if line == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if indent == 2 && t.ends_with(':') && !t.starts_with('#') && !t.contains(' ') {
            out.push((t.trim_end_matches(':').to_owned(), Vec::new()));
        } else if let Some((_, lines)) = out.last_mut() {
            lines.push(line.to_owned());
        }
    }
    out
}

/// A line of a job's code (not a comment, a step name or echoed text).
fn code(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with('#')
        || t.starts_with("- name:")
        || t.starts_with("name:")
        || t.contains("echo ")
    {
        None
    } else {
        Some(t)
    }
}

fn every_mutants_job_installs_nextest(toml: &str, workflows: &[(String, String)]) {
    let mut found = 0;
    for (file, yml) in workflows {
        for (job, lines) in jobs(yml) {
            if !lines
                .iter()
                .filter_map(|l| code(l))
                .any(|t| t.contains("cargo mutants "))
            {
                continue;
            }
            found += 1;
            if uses_nextest(toml) {
                assert!(
                    lines.iter().filter_map(|l| code(l)).any(|t| t.contains("cargo-nextest")),
                    "qa-ops: a job runs `cargo mutants` with `test_tool = \"nextest\"` but installs no cargo-nextest \
                     ({file}: {job})"
                );
            }
        }
    }
    assert!(
        found > 0,
        "qa-ops: no job runs `cargo mutants` in the workflows"
    );
}

fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("qa-ops: .github/workflows")
        .map(|e| e.expect("qa-ops: entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("qa-ops: workflow"),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn qa_ops_every_job_running_cargo_mutants_installs_nextest() {
    every_mutants_job_installs_nextest(&mutants_toml(), &workflows());
}

// The fixture job as it stood before this change: cargo-mutants installed alone.
negative_control!(
    qa_ops_every_job_running_cargo_mutants_installs_nextest,
    "a job that runs `cargo mutants` and installs cargo-mutants alone",
    expected = "qa-ops: a job runs `cargo mutants` with `test_tool = \"nextest\"` but installs no cargo-nextest",
    every_mutants_job_installs_nextest(
        &mutants_toml(),
        &[(
            "control.yml".to_owned(),
            "jobs:\n  fixture:\n    steps:\n      - uses: taiki-e/install-action@v2\n        with:\n          \
             tool: cargo-mutants@27.1.0\n      # cargo-nextest is not installed here\n      - run: |\n          \
             cargo mutants --dir fixtures/mutants/untested --config .cargo/mutants.toml\n"
                .to_owned()
        )]
    )
);
