//! `scripts/contain.sh` (R-658; applied per R-369): CI's mutants runs of the job queue run in a PID namespace and a cgroup
//! scope, and each queue test process in a namespace of its own. Here, on any machine: the script runs its command as it
//! is unless a run opts in, and refuses to run it at all when containment is required and can't be had; both mutants
//! workflows' runs have it as their shell, having proved it; nextest wraps every queue test; and every queue test file bounds its
//! xtask calls. The containment itself needs Linux and sudo: `scripts/contain.sh check` proves it on a runner, in
//! each shard and in `mutants-fixture`, with a real `kill -9 -1` inside each.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).expect("readable")
}

/// `scripts/contain.sh <args>`, with neither opt-in from a run this test may itself be inside.
fn contain(args: &[&str]) -> Command {
    let mut command = Command::new(root().join("scripts/contain.sh"));
    command
        .args(args)
        .env_remove("PRIN_CONTAIN")
        .env_remove("PRIN_TEST_PIDNS");
    command
}

fn status(mut command: Command) -> Option<i32> {
    command
        .output_within(Duration::from_secs(60))
        .expect("contain.sh ran")
        .status
        .code()
}

/// Each mode, not opted in, must exit with the command's own status; `script` is the command.
fn passes_through(script: &str, expected: i32) {
    for mode in ["run", "test"] {
        assert_eq!(
            status(contain(&[mode, "sh", "-c", script])),
            Some(expected),
            "`{mode}` did not run its command as it is"
        );
    }
}

#[test]
fn contain_runs_its_command_as_it_is_without_the_opt_in() {
    passes_through("exit 7", 7);
    passes_through("exit 0", 0);
}

negative_control!(
    contain_runs_its_command_as_it_is_without_the_opt_in,
    "the command's status comes back, not some other",
    expected = "did not run its command as it is",
    passes_through("exit 7", 0)
);

#[test]
fn contain_refuses_an_unknown_mode() {
    assert_unknown_refused("bogus");
    assert_unknown_refused("");
}

fn assert_unknown_refused(mode: &str) {
    assert_eq!(
        status(contain(&[mode, "true"])),
        Some(125),
        "mode {mode:?} accepted"
    );
}

negative_control!(
    contain_refuses_an_unknown_mode,
    "a real mode is no unknown one",
    expected = "accepted",
    assert_unknown_refused("run")
);

/// `contain.sh run` under a `sudo` that always refuses, with `required`: whether it ran its command, and its status.
fn run_under_refusing_sudo(required: bool, tag: &str) -> (bool, Option<i32>, Scratch) {
    let dir = Scratch::new(tag);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    let sudo = bin.join("sudo");
    std::fs::write(&sudo, "#!/bin/sh\nexit 1\n").expect("stand-in");
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&sudo, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    let marker = dir.join("ran");
    let mut command = contain(&["run", "touch", marker.to_str().expect("utf-8 path")]);
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    command.env("PATH", path);
    if required {
        command.env("PRIN_CONTAIN", "required");
    }
    let code = status(command);
    (marker.exists(), code, dir)
}

fn check_fails_closed(required: bool, tag: &str) {
    let (ran, code, _dir) = run_under_refusing_sudo(required, tag);
    assert!(
        !ran && code == Some(125),
        "ran uncontained, or passed: ran {ran}, status {code:?}"
    );
}

/// Where containment is required and sudo refuses (or this isn't Linux), the command must not run.
#[test]
fn contain_never_runs_a_required_run_uncontained() {
    check_fails_closed(true, "contain_closed");
}

negative_control!(
    contain_never_runs_a_required_run_uncontained,
    "without the requirement the command does run, so the check can see it",
    expected = "ran uncontained, or passed",
    check_fails_closed(false, "contain_closed_control")
);

/// The lines of the step named `name` in `workflow`, through to the next step.
fn step<'a>(workflow: &'a str, name: &str) -> Vec<&'a str> {
    workflow
        .lines()
        .skip_while(|l| !l.trim_start().starts_with(&format!("- name: {name}")))
        .enumerate()
        .take_while(|(i, l)| *i == 0 || !l.starts_with("      - "))
        .map(|(_, l)| l)
        .collect()
}

/// The `shell:` of a contained step.
const CONTAINED: &str =
    "shell: bash scripts/contain.sh run bash --noprofile --norc -eo pipefail {0}";

/// A mutants run in `workflow`, named `name`, has the script as its shell, as required, after the check.
fn check_workflow(file: &str, workflow: &str, name: &str) {
    let run = step(workflow, name);
    assert!(
        run.iter().any(|l| l.trim() == CONTAINED),
        "{file}: the mutants run does not go through contain.sh"
    );
    assert!(
        run.iter().any(|l| l.trim() == "PRIN_CONTAIN: required"),
        "{file}: the mutants run does not require containment"
    );
    let check = workflow
        .find("shell: bash scripts/contain.sh check {0}")
        .unwrap_or(usize::MAX);
    assert!(
        check < workflow.find(run[0]).unwrap_or(0),
        "{file}: the mutants run isn't preceded by the containment's check"
    );
}

fn check_workflows(mutants: &str, nightly: &str) {
    check_workflow("mutants.yml", mutants, "cargo mutants --in-diff --shard");
    check_workflow("nightly.yml", nightly, "cargo mutants --package");
}

#[test]
fn contain_every_mutants_run_of_the_queue_goes_through_it() {
    check_workflows(
        &read(".github/workflows/mutants.yml"),
        &read(".github/workflows/nightly.yml"),
    );
}

negative_control!(
    contain_every_mutants_run_of_the_queue_goes_through_it,
    "a shard's run outside the script",
    expected = "mutants.yml: the mutants run does not go through contain.sh",
    check_workflows(
        &read(".github/workflows/mutants.yml").replace(CONTAINED, "shell: bash"),
        &read(".github/workflows/nightly.yml"),
    )
);

mod no_check {
    use super::*;
    negative_control!(
        contain_every_mutants_run_of_the_queue_goes_through_it,
        "a shard's run with no check before it",
        expected = "mutants.yml: the mutants run isn't preceded by the containment's check",
        check_workflows(
            &read(".github/workflows/mutants.yml")
                .replace("shell: bash scripts/contain.sh check {0}", "shell: bash"),
            &read(".github/workflows/nightly.yml"),
        )
    );
}

mod nightly {
    use super::*;
    negative_control!(
        contain_every_mutants_run_of_the_queue_goes_through_it,
        "the nightly run not required to be contained",
        expected = "nightly.yml: the mutants run does not require containment",
        check_workflows(
            &read(".github/workflows/mutants.yml"),
            &read(".github/workflows/nightly.yml").replace(
                "          SHARD: ${{ matrix.k }}/${{ matrix.n }}\n          PRIN_CONTAIN: required\n",
                "          SHARD: ${{ matrix.k }}/${{ matrix.n }}\n"
            ),
        )
    );
}

/// The queue's test files: `jobq.rs` and every `qa_R-658*.rs`.
fn queue_tests() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root().join("xtask/tests"))
        .expect("tests")
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.strip_suffix(".rs")?.to_owned();
            (name == "jobq" || name.starts_with("qa_R-658")).then_some(name)
        })
        .collect();
    names.sort();
    names
}

/// The filter in front of the wrapper must select `xtask::jobq` and every `xtask::qa_R-658*`.
fn check_nextest(config: &str) {
    let scripts = config
        .split("[[profile.default.scripts]]")
        .nth(1)
        .unwrap_or_default();
    assert!(
        scripts.contains("filter = \"binary_id(=xtask::jobq) | binary_id(/^xtask::qa_R-658/)\"")
            && scripts.contains("run-wrapper = \"queue-pidns\""),
        "the queue's tests are not wrapped"
    );
    assert!(
        config.contains("command-line = \"scripts/contain.sh test\"")
            && config.contains("target-runner = \"within-wrapper\""),
        "the queue's wrapper is not contain.sh, inside the memory cap's runner"
    );
    let names = queue_tests();
    assert!(
        names.len() >= 5
            && names
                .iter()
                .all(|n| n == "jobq" || n.starts_with("qa_R-658")),
        "the queue's tests not found: {names:?}"
    );
}

#[test]
fn contain_wraps_every_queue_test_process() {
    check_nextest(&read(".config/nextest.toml"));
}

negative_control!(
    contain_wraps_every_queue_test_process,
    "qa's tests left out of the wrapper",
    expected = "the queue's tests are not wrapped",
    check_nextest(&read(".config/nextest.toml").replace(
        "filter = \"binary_id(=xtask::jobq) | binary_id(/^xtask::qa_R-658/)\"\nrun-wrapper",
        "filter = \"binary_id(=xtask::jobq)\"\nrun-wrapper"
    ))
);

/// Each queue test file's xtask calls must have CPU and file-size limits, so a mutant's spinning or endlessly writing
/// runner ends.
fn check_limits(text: impl Fn(&str) -> String) {
    for name in queue_tests() {
        assert!(
            text(&name).contains("ulimit -t 600 && ulimit -f 1048576"),
            "{name}: xtask calls without CPU and file limits"
        );
    }
}

#[test]
fn contain_every_queue_test_file_limits_its_calls() {
    check_limits(|name| read(&format!("xtask/tests/{name}.rs")));
}

negative_control!(
    contain_every_queue_test_file_limits_its_calls,
    "a file with the process cap alone",
    expected = "xtask calls without CPU and file limits",
    check_limits(|name| read(&format!("xtask/tests/{name}.rs"))
        .replace("ulimit -t 600 && ulimit -f 1048576", "ulimit -u 300"))
);

/// A required run on stand-ins for the privileged tools, whose command notes a sample in the heartbeat's log and exits
/// `code`: the job's log, and the finished status posted.
fn finished_run(code: i32, tag: &str) -> (String, String, Scratch) {
    let dir = Scratch::new(tag);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    let curl = dir.join("curl");
    let stubs = [
        ("uname", "echo Linux".to_owned()),
        ("sudo", "[ \"$1\" = -n ] && shift\nexec \"$@\"".to_owned()),
        ("sysctl", "exit 0".to_owned()),
        ("systemctl", "exit 0".to_owned()),
        ("dmsetup", "exit 0".to_owned()),
        ("df", "echo Avail\necho 100G".to_owned()),
        (
            "systemd-run",
            "while [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\"".to_owned(),
        ),
        (
            "unshare",
            "while case \"$1\" in --*) true ;; *) false ;; esac; do shift; done\nexec \"$@\""
                .to_owned(),
        ),
        (
            "setpriv",
            "while [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\"".to_owned(),
        ),
        ("setsid", "exec \"$@\"".to_owned()),
        (
            "curl",
            format!("printf '%s\\n' \"$@\" >> '{}'", curl.display()),
        ),
    ];
    for (name, body) in stubs {
        write_executable(&bin.join(name), format!("#!/bin/bash\n{body}\n")).expect("stand-in");
    }
    let script = format!(
        "echo '+1m mem 1G oom 5 logs 1 sample-while-alive' >> \"$RUNNER_TEMP/heartbeat.log\"; exit {code}"
    );
    let mut command = contain(&["run", "sh", "-c", &script]);
    command
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("PRIN_CONTAIN", "required")
        .env("RUNNER_TEMP", &*dir)
        .env("HEARTBEAT_CONTEXT", "context")
        .env("HEARTBEAT_SHA", "sha")
        .env("HEARTBEAT_TOKEN", "token")
        .env("GITHUB_REPOSITORY", "owner/repo");
    let out = command
        .output_within(Duration::from_secs(60))
        .expect("contain.sh ran");
    assert_eq!(
        out.status.code(),
        Some(code),
        "the command's status is lost"
    );
    let posted = std::fs::read_to_string(&curl).unwrap_or_default();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        posted,
        dir,
    )
}

/// The finished status must carry the count of the last sample made while the scope lived, whatever the run's exit; and
/// the job's log must hold the samples on a non-zero exit, as `heartbeat.log` is not uploaded.
fn check_finished(code: i32, tag: &str) {
    let (log, posted, _dir) = finished_run(code, tag);
    assert!(
        posted.contains(&format!("finished, exit {code}.")) && posted.contains(" oom 5 "),
        "the finished status lost the out-of-memory count: {posted}"
    );
    assert_eq!(
        log.contains("sample-while-alive"),
        code != 0,
        "the heartbeat's samples are in the job's log only on a non-zero exit: {log}"
    );
}

#[test]
fn contain_a_finished_run_keeps_its_last_samples() {
    check_finished(3, "contain_finished");
    check_finished(0, "contain_finished_clean");
}

negative_control!(
    contain_a_finished_run_keeps_its_last_samples,
    "a run with an unsampled status, seen by the same check",
    expected = "the finished status lost the out-of-memory count",
    {
        let (_, posted, _dir) = finished_run(3, "contain_finished_control");
        assert!(
            posted.contains("oom 6"),
            "the finished status lost the out-of-memory count: {posted}"
        );
    }
);
