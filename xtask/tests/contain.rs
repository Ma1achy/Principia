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

/// The filter in front of the wrapper must select `xtask::jobq` and every `xtask::qa_R-658*`. Only the default
/// profile's scripts section counts, up to the next table: the mutants profile restates the same filter in an
/// override, and inherits the default profile's scripts.
fn check_nextest(config: &str) {
    let scripts = config
        .split("[[profile.default.scripts]]")
        .nth(1)
        .unwrap_or_default()
        .split("\n[")
        .next()
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
        // The cgroup the run notes its count from: none, as a scope killed whole leaves none noted. On Linux the real
        // one, this test's own, would read 0 in place of the last sample's.
        (
            "cut",
            "[ \"$1\" = -d: ] && { echo /nonexistent; exit 0; }\nexec /usr/bin/cut \"$@\""
                .to_owned(),
        ),
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

/// `script`, as `scripts/contain.sh run`, on stand-ins for the privileged tools, sent `signal` (1 HUP, 2 INT, 15 TERM)
/// while its command runs: what its driver saw, `status <n>` and, if the command outlived it, `command-alive`, and the
/// calls of `systemctl kill`. The stand-in `systemctl kill` ends the command; with `late`, the scope doesn't exist yet
/// for the first call, which does nothing and fails.
fn stopped_by(script: &Path, signal: i32, late: bool, tag: &str) -> (String, usize) {
    let dir = Scratch::new(tag);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    let pid = dir.join("pid");
    let calls = dir.join("calls");
    let kill_command = format!("kill -9 \"$(cat '{}')\"", pid.display());
    let kills = if late {
        format!(
            "[ \"$(wc -l < '{}')\" -gt 1 ] || exit 1\n{kill_command}",
            calls.display()
        )
    } else {
        kill_command
    };
    let stubs = [
        ("uname", "echo Linux".to_owned()),
        ("sudo", "[ \"$1\" = -n ] && shift\nexec \"$@\"".to_owned()),
        ("sysctl", "exit 0".to_owned()),
        (
            "systemctl",
            format!(
                "[ \"$1\" = kill ] || exit 0\necho kill >> '{}'\n{kills}",
                calls.display()
            ),
        ),
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
    ];
    for (name, body) in stubs {
        write_executable(&bin.join(name), format!("#!/bin/bash\n{body}\n")).expect("stand-in");
    }
    let command = dir.join("command");
    write_executable(
        &command,
        format!(
            "#!/bin/bash\necho $$ > '{}'\nexec sleep 20\n",
            pid.display()
        ),
    )
    .expect("stand-in");
    // `set -m`: a background job of a shell without it starts with SIGINT ignored, and bash can't trap that. It also
    // gives the script a group of its own, which the driver ends last: on these stand-ins nothing kills a whole scope, and
    // what's left of the run would still be writing in the scratch folder as it is removed.
    let driver = format!(
        "set -m\n'{script}' run '{command}' > /dev/null 2>&1 &\nrun=$!\ni=0\n\
         while [ ! -s '{pid}' ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done\n\
         kill -{signal} $run\nwait $run\necho \"status $?\"\n\
         kill -0 \"$(cat '{pid}')\" 2> /dev/null && echo command-alive\n\
         kill -9 -- -$run 2> /dev/null\nsleep 0.2\nexit 0",
        script = script.display(),
        command = command.display(),
        pid = pid.display(),
    );
    let out = Command::new("bash")
        .args(["-c", &driver])
        .env_remove("PRIN_TEST_PIDNS")
        .env_remove("HEARTBEAT_CONTEXT")
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
        .output_within(Duration::from_secs(60))
        .expect("the driver ran");
    let called = std::fs::read_to_string(&calls)
        .unwrap_or_default()
        .lines()
        .count();
    (String::from_utf8_lossy(&out.stdout).into_owned(), called)
}

fn check_stopped(script: &Path, signal: i32, late: bool, tag: &str) {
    let (seen, called) = stopped_by(script, signal, late, tag);
    assert!(
        seen.contains(&format!("status {}", 128 + signal)) && !seen.contains("command-alive"),
        "signal {signal} did not end the contained command: {seen:?}, {called} kills"
    );
    assert!(
        called >= if late { 2 } else { 1 },
        "signal {signal}: {called} kills of its scope"
    );
}

#[test]
fn contain_a_signal_to_a_run_ends_it_whichever_the_signal() {
    let script = root().join("scripts/contain.sh");
    check_stopped(&script, 2, false, "contain_sigint");
    check_stopped(&script, 1, false, "contain_sighup");
    check_stopped(&script, 15, false, "contain_sigterm");
}

negative_control!(
    contain_a_signal_to_a_run_ends_it_whichever_the_signal,
    "a script whose SIGINT stops nothing",
    expected = "did not end the contained command",
    {
        let dir = Scratch::new("contain_signal_control");
        std::fs::create_dir_all(&*dir).expect("scratch");
        let script = dir.join("contain.sh");
        write_executable(
            &script,
            read("scripts/contain.sh")
                .replace("trap 'stop_run 2' INT", "trap 'stopped=130' INT")
                .replace("tries=20", "tries=0"),
        )
        .expect("the script's copy");
        check_stopped(&script, 2, false, "contain_signal_control_run");
    }
);

#[test]
fn contain_a_signal_before_the_scope_exists_still_ends_it() {
    check_stopped(
        &root().join("scripts/contain.sh"),
        15,
        true,
        "contain_scope_late",
    );
}

negative_control!(
    contain_a_signal_before_the_scope_exists_still_ends_it,
    "a script that tries the scope once",
    expected = "did not end the contained command",
    {
        let dir = Scratch::new("contain_scope_late_control");
        std::fs::create_dir_all(&*dir).expect("scratch");
        let script = dir.join("contain.sh");
        write_executable(
            &script,
            read("scripts/contain.sh").replace("tries=20", "tries=0"),
        )
        .expect("the script's copy");
        check_stopped(&script, 15, true, "contain_scope_late_control_run");
    }
);

// ---- The unprivileged mode (the owner's ruling of 9 Nov 2026, for the WSL2 distro) ---------------------------------
//
// "Inside WSL, check proves the unprivileged mode, meaning user and PID namespaces with no sudo, using the real kill -9
// -1. ... Other machines keep the strict mode unchanged ... The WSL run selects the unprivileged mode explicitly, so
// nothing falls back to it silently. If user namespaces aren't available, the check fails. It never skips." The mode
// itself needs Linux with user namespaces: `wsl-proofs` proves it in the distro, with the real `kill -9 -1`. Here, on
// any machine, its selection and its refusals, on stand-ins. No `check` here ever reaches its probe on a stand-in
// `unshare` that runs its command: the probe's `kill -9 -1` would then reach this user's processes.

/// What `unshare` stands in as: running its command as it is, or refusing as a kernel without user namespaces does.
const UNSHARE_RUNS: &str =
    "while case \"$1\" in --*) true ;; *) false ;; esac; do shift; done\nexec \"$@\"";
const UNSHARE_REFUSES: &str = "echo 'unshare: unshare failed: Operation not permitted' >&2\nexit 1";

/// A machine of stand-ins in a scratch folder: `uname` says `system`, `id -u` says `uid`, `sudo` notes its call in
/// `sudo` and refuses, `unshare` notes its arguments in `unshare` and then does `unshare`, `setsid` runs its command.
/// Returns the folder and the PATH that puts them first.
fn unprivileged_machine(tag: &str, system: &str, uid: u32, unshare: &str) -> (Scratch, String) {
    let dir = Scratch::new(tag);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    let note = |name: &str| dir.join(name).display().to_string();
    let stubs = [
        ("uname", format!("echo {system}")),
        (
            "id",
            format!("[ \"$1\" = -u ] && {{ echo {uid}; exit 0; }}\nexec /usr/bin/id \"$@\""),
        ),
        ("sudo", format!("echo \"$*\" >> '{}'\nexit 1", note("sudo"))),
        (
            "unshare",
            format!("echo \"$*\" >> '{}'\n{unshare}", note("unshare")),
        ),
        ("setsid", "exec \"$@\"".to_owned()),
    ];
    for (name, body) in stubs {
        write_executable(&bin.join(name), format!("#!/bin/bash\n{body}\n")).expect("stand-in");
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    (dir, path)
}

/// `script <args>` on the machine at `path`, with PRIN_CONTAIN as `mode` (unset for `None`): its status, stdout and
/// stderr.
fn on_machine(
    script: &Path,
    args: &[&str],
    path: &str,
    mode: Option<&str>,
) -> (Option<i32>, String, String) {
    let mut command = Command::new(script);
    command
        .args(args)
        .env_remove("PRIN_CONTAIN")
        .env_remove("PRIN_TEST_PIDNS")
        .env("PATH", path);
    if let Some(mode) = mode {
        command.env("PRIN_CONTAIN", mode);
    }
    let out = command
        .output_within(Duration::from_secs(60))
        .expect("contain.sh ran");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn contain_script() -> PathBuf {
    root().join("scripts/contain.sh")
}

/// A copy of the script with `from` replaced by `to`, in `dir`.
#[cfg(feature = "controls")]
fn mutated_copy(dir: &Path, from: &str, to: &str) -> PathBuf {
    let text = read("scripts/contain.sh");
    assert!(text.contains(from), "the control's text is gone: {from}");
    let path = dir.join("contain-mutated.sh");
    write_executable(&path, text.replacen(from, to, 1)).expect("the script's copy");
    path
}

/// Where user namespaces can't be had, the mode named `mode` must fail both `check` and `run`, never pass or skip,
/// and must not run the command.
fn check_never_skips(mode: Option<&str>, system: &str, why: &str, tag: &str) {
    let (dir, path) = unprivileged_machine(tag, system, 1000, UNSHARE_REFUSES);
    let (code, out, err) = on_machine(&contain_script(), &["check"], &path, mode);
    assert!(
        code == Some(125) && !out.contains("not required") && err.contains(why),
        "the unprivileged check skipped or passed without user namespaces: {code:?}, {out:?}, {err:?}"
    );
    let marker = dir.join("ran");
    let (code, _, err) = on_machine(
        &contain_script(),
        &["run", "touch", marker.to_str().expect("utf-8 path")],
        &path,
        mode,
    );
    assert!(
        code == Some(125) && !marker.exists() && err.contains(why),
        "the unprivileged run ran, or failed for another reason, without user namespaces: {code:?}, {err:?}"
    );
}

#[test]
fn contain_unprivileged_fails_never_skips_without_user_namespaces() {
    check_never_skips(
        Some("unprivileged"),
        "Linux",
        "user and PID namespaces can't be had here",
        "contain_userns_none",
    );
    check_never_skips(
        Some("unprivileged"),
        "Darwin",
        "this is not Linux: no user namespaces",
        "contain_userns_darwin",
    );
}

negative_control!(
    contain_unprivileged_fails_never_skips_without_user_namespaces,
    "the mode not selected: check passes, saying containment isn't required",
    expected = "the unprivileged check skipped or passed without user namespaces",
    check_never_skips(
        None,
        "Linux",
        "user and PID namespaces can't be had here",
        "contain_userns_control"
    )
);

/// On this machine itself, where it isn't Linux (the Mac): the real `uname`, the real refusal.
#[cfg(not(target_os = "linux"))]
fn check_fails_here(path: &str) {
    let (code, out, err) = on_machine(&contain_script(), &["check"], path, Some("unprivileged"));
    assert!(
        code == Some(125)
            && !out.contains("not required")
            && err.contains("unprivileged containment selected, and this is not Linux: no user namespaces"),
        "the unprivileged check didn't fail here, off Linux, for want of user namespaces: {code:?}, {out:?}, {err:?}"
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn contain_unprivileged_fails_on_this_machine() {
    check_fails_here(&std::env::var("PATH").unwrap_or_default());
}

#[cfg(not(target_os = "linux"))]
negative_control!(
    contain_unprivileged_fails_on_this_machine,
    "a stand-in that says Linux: the refusal is another one",
    expected = "the unprivileged check didn't fail here",
    {
        let (_dir, path) =
            unprivileged_machine("contain_here_control", "Linux", 1000, UNSHARE_REFUSES);
        check_fails_here(&path);
    }
);

/// `required` on a machine whose sudo refuses and whose `unshare` would work: it must fail, and never fall back to the
/// unprivileged mode, which would call `unshare` and run the command.
fn check_no_fallback(mode: &str, tag: &str) {
    let (dir, path) = unprivileged_machine(tag, "Linux", 1000, UNSHARE_RUNS);
    let marker = dir.join("ran");
    let (code, _, _) = on_machine(
        &contain_script(),
        &["run", "touch", marker.to_str().expect("utf-8 path")],
        &path,
        Some(mode),
    );
    assert!(
        code == Some(125) && !marker.exists() && !dir.join("unshare").exists(),
        "a required run fell back to the unprivileged mode: {code:?}, ran {}",
        marker.exists()
    );
}

#[test]
fn contain_required_never_falls_back_to_unprivileged() {
    check_no_fallback("required", "contain_no_fallback");
}

negative_control!(
    contain_required_never_falls_back_to_unprivileged,
    "the unprivileged mode itself, on the same machine, does run the command",
    expected = "a required run fell back to the unprivileged mode",
    check_no_fallback("unprivileged", "contain_no_fallback_control")
);

/// A value of PRIN_CONTAIN that names no mode fails `run` and `check` alike: a misspelt mode never runs a command
/// uncontained, nor skips the check.
fn check_misspelt(mode: Option<&str>, tag: &str) {
    let (dir, path) = unprivileged_machine(tag, "Linux", 1000, UNSHARE_REFUSES);
    let marker = dir.join("ran");
    let (run, _, err) = on_machine(
        &contain_script(),
        &["run", "touch", marker.to_str().expect("utf-8 path")],
        &path,
        mode,
    );
    let (check, out, _) = on_machine(&contain_script(), &["check"], &path, mode);
    assert!(
        run == Some(125)
            && !marker.exists()
            && err.contains("names no mode")
            && check == Some(125)
            && !out.contains("not required"),
        "a misspelt mode ran the command or skipped the check: run {run:?}, check {check:?}"
    );
}

#[test]
fn contain_refuses_a_misspelt_mode() {
    check_misspelt(Some("Unprivileged"), "contain_misspelt");
    check_misspelt(Some("require"), "contain_misspelt_required");
}

negative_control!(
    contain_refuses_a_misspelt_mode,
    "no mode at all: the command runs as it is, and the check says containment isn't required",
    expected = "a misspelt mode ran the command or skipped the check",
    check_misspelt(None, "contain_misspelt_control")
);

/// The unprivileged run as root: refused, as the mode is an ordinary user's.
fn check_not_root(uid: u32, tag: &str) {
    let (dir, path) = unprivileged_machine(tag, "Linux", uid, UNSHARE_RUNS);
    let marker = dir.join("ran");
    let (code, _, err) = on_machine(
        &contain_script(),
        &["run", "touch", marker.to_str().expect("utf-8 path")],
        &path,
        Some("unprivileged"),
    );
    assert!(
        code == Some(125) && !marker.exists() && err.contains("this is root"),
        "the unprivileged mode ran as root: {code:?}, {err:?}"
    );
}

#[test]
fn contain_unprivileged_refuses_root() {
    check_not_root(0, "contain_root");
}

negative_control!(
    contain_unprivileged_refuses_root,
    "an ordinary user, whose run goes ahead",
    expected = "the unprivileged mode ran as root",
    check_not_root(1000, "contain_root_control")
);

/// An unprivileged run: the command's own status; `unshare` asked for a user and a PID namespace, killed with it,
/// /proc its own; sudo never called; the command's environment without the heartbeat's token or PRIN_CONTAIN, and with
/// PRIN_TEST_PIDNS, so the queue's tests get namespaces of their own inside.
fn check_unprivileged_run(script: &Path, tag: &str) {
    let (dir, path) = unprivileged_machine(tag, "Linux", 1000, UNSHARE_RUNS);
    let env = dir.join("env");
    let mut command = Command::new(script);
    command
        .args([
            "run",
            "sh",
            "-c",
            &format!("env > '{}'; exit 7", env.display()),
        ])
        .env_remove("PRIN_TEST_PIDNS")
        .env("PATH", &path)
        .env("PRIN_CONTAIN", "unprivileged")
        .env("HEARTBEAT_TOKEN", "sekrit-token");
    let code = command
        .output_within(Duration::from_secs(60))
        .expect("contain.sh ran")
        .status
        .code();
    let unshare = std::fs::read_to_string(dir.join("unshare")).unwrap_or_default();
    let env = std::fs::read_to_string(env).unwrap_or_default();
    assert_eq!(code, Some(7), "the command's status didn't come back");
    assert!(
        unshare.lines().any(|l| l.starts_with(
            "--user --map-current-user --pid --fork --kill-child --mount-proc bash -c "
        )),
        "the run wasn't in a user and PID namespace of its own: {unshare:?}"
    );
    assert!(
        !dir.join("sudo").exists(),
        "the unprivileged run called sudo"
    );
    assert!(
        !env.contains("sekrit-token")
            && !env.contains("PRIN_CONTAIN=")
            && env.contains("PRIN_TEST_PIDNS=1"),
        "the command's environment is wrong: {env}"
    );
}

#[test]
fn contain_unprivileged_run_uses_namespaces_without_sudo() {
    check_unprivileged_run(&contain_script(), "contain_unprivileged_run");
}

negative_control!(
    contain_unprivileged_run_uses_namespaces_without_sudo,
    "a run with no PID namespace",
    expected = "the run wasn't in a user and PID namespace of its own",
    {
        let dir = Scratch::new("contain_unprivileged_run_control");
        std::fs::create_dir_all(&*dir).expect("scratch");
        let script = mutated_copy(
            &dir,
            "  unshare --user --map-current-user --pid --fork --kill-child --mount-proc \\\n",
            "  unshare --user --map-current-user --fork --kill-child --mount-proc \\\n",
        );
        check_unprivileged_run(&script, "contain_unprivileged_run_control_run");
    }
);

mod unprivileged_token {
    use super::*;
    negative_control!(
        contain_unprivileged_run_uses_namespaces_without_sudo,
        "the heartbeat's token left in the command's environment",
        expected = "the command's environment is wrong",
        {
            let dir = Scratch::new("contain_unprivileged_token_control");
            std::fs::create_dir_all(&*dir).expect("scratch");
            let script = mutated_copy(
                &dir,
                "  unset HEARTBEAT_TOKEN PRIN_CONTAIN\n  export PRIN_TEST_PIDNS=1\n  trap 'stop_unprivileged",
                "  unset PRIN_CONTAIN\n  export PRIN_TEST_PIDNS=1\n  trap 'stop_unprivileged",
            );
            check_unprivileged_run(&script, "contain_unprivileged_token_control_run");
        }
    );
}

mod unprivileged_sudo {
    use super::*;
    negative_control!(
        contain_unprivileged_run_uses_namespaces_without_sudo,
        "a run that asks sudo first",
        expected = "the unprivileged run called sudo",
        {
            let dir = Scratch::new("contain_unprivileged_sudo_control");
            std::fs::create_dir_all(&*dir).expect("scratch");
            let script = mutated_copy(
                &dir,
                "run_unprivileged() {\n  require_userns\n",
                "run_unprivileged() {\n  require_userns\n  sudo -n true || true\n",
            );
            check_unprivileged_run(&script, "contain_unprivileged_sudo_control_run");
        }
    );
}

/// An unprivileged run sent SIGTERM while its command runs: the run ends at once with 143, and `unshare`, whose death
/// ends the namespace (`--kill-child`), is dead. The stand-in `unshare` records its pid, then becomes the namespace's
/// init; the command, which a real namespace would take with it, is ended by the driver.
fn check_unprivileged_signal(script: &Path, tag: &str) {
    let (dir, path) = unprivileged_machine(
        tag,
        "Linux",
        1000,
        &format!("echo $$ > '{}'\n{UNSHARE_RUNS}", dir_note(tag)),
    );
    let ns = dir_note(tag);
    let command = dir.join("command");
    let pid = dir.join("pid");
    write_executable(
        &command,
        format!(
            "#!/bin/bash\necho $$ > '{}'\nexec sleep 20\n",
            pid.display()
        ),
    )
    .expect("stand-in");
    let driver = format!(
        "set -m\n'{script}' run '{command}' > /dev/null 2>&1 &\nrun=$!\ni=0\n\
         while [ ! -s '{pid}' ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done\n\
         kill -TERM $run\nwait $run\necho \"status $?\"\nsleep 0.2\n\
         kill -0 \"$(cat '{ns}')\" 2> /dev/null && echo namespace-alive\n\
         kill -9 \"$(cat '{pid}')\" 2> /dev/null\nkill -9 -- -$run 2> /dev/null\nexit 0",
        script = script.display(),
        command = command.display(),
        pid = pid.display(),
    );
    let out = Command::new("bash")
        .args(["-c", &driver])
        .env_remove("PRIN_TEST_PIDNS")
        .env("PATH", &path)
        .env("PRIN_CONTAIN", "unprivileged")
        .output_within(Duration::from_secs(60))
        .expect("the driver ran");
    let seen = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_file(&ns);
    assert!(
        seen.contains("status 143") && !seen.contains("namespace-alive"),
        "SIGTERM did not end the unprivileged run and its namespace: {seen:?}"
    );
}

/// Where the stand-in `unshare` of the test tagged `tag` notes its pid: beside the scratch folder, which is made after.
fn dir_note(tag: &str) -> String {
    std::env::temp_dir()
        .join(format!("{tag}-{}.ns", std::process::id()))
        .display()
        .to_string()
}

#[test]
fn contain_a_signal_ends_an_unprivileged_run() {
    check_unprivileged_signal(&contain_script(), "contain_unprivileged_signal");
}

negative_control!(
    contain_a_signal_ends_an_unprivileged_run,
    "a script that ignores SIGTERM",
    expected = "SIGTERM did not end the unprivileged run and its namespace",
    {
        let dir = Scratch::new("contain_unprivileged_signal_control");
        std::fs::create_dir_all(&*dir).expect("scratch");
        let script = mutated_copy(&dir, "trap 'stop_unprivileged 15' TERM", "trap '' TERM");
        check_unprivileged_signal(&script, "contain_unprivileged_signal_control_run");
    }
);
