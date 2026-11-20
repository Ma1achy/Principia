//! qa's tests for `scripts/contain.sh` (R-658; applied per R-369), written from what the script's header promises of a
//! required run, not from its body: the command's status comes back; it has the environment it was given, without the
//! heartbeat's token and with `PRIN_TEST_PIDNS`; it has this shell's limits back though `sudo` lowered them; the
//! heartbeat's status is always `success` and at most 140 characters; the run is stopped before the disk fills; `test`
//! asks for a PID namespace and passes on a SIGTERM; and a step stopped by its limit stops what it contained.
//!
//! The containment itself needs Linux and root, and `contain.sh check` proves it on a runner. Here, on any machine, the
//! tools it calls are stand-ins on `PATH`, which run their command as the real ones would, unprivileged. They prove the
//! script's own logic, not the kernel's.
//!
//! Safety: nothing here signals beyond a pid it recorded itself; no stand-in escalates; every process is under
//! `ulimit -u`, `-t` and `-f` and ends by itself: the commands within seconds, and a heartbeat left behind by the
//! stand-in `sleep`, which ends it after a few beats.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

/// The bounds on every process these tests start.
const BOUNDED: &str = "ulimit -u \"$(( $(ps -U \"$(id -u)\" -o pid= | wc -l) + 300 ))\" && ulimit -t 600 && ulimit -f 1048576 && exec \"$@\"";

fn contain_sh() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    std::fs::read_to_string(root.join("scripts/contain.sh")).expect("readable")
}

/// A machine of stand-ins: `run_contained` and `run_test` reach every tool here, each running its command as it is and
/// noting what it was asked. `sudo` lowers the soft limit on open files, as the real one lowers the core-dump limit.
struct Stand {
    dir: Scratch,
    script: PathBuf,
}

const STUBS: &[(&str, &str)] = &[
    ("uname", "echo Linux"),
    ("sudo", "ulimit -Sn 100\n[ \"$1\" = -n ] && shift\nexec \"$@\""),
    ("sysctl", "exit 0"),
    ("systemd-run", "while [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\""),
    (
        "systemctl",
        "if [ \"$1\" = kill ]; then\n  echo \"$*\" >> \"$STAND/stopped\"\n  kill -9 \"$(cat \"$STAND/pid\")\" 2> /dev/null\nfi\nexit 0",
    ),
    (
        "unshare",
        "echo \"$*\" >> \"$STAND/unshare\"\nwhile case \"$1\" in --*) true ;; *) false ;; esac; do shift; done\nexec \"$@\"",
    ),
    ("setpriv", "while [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\""),
    ("setsid", "exec \"$@\""),
    ("curl", "printf '%s\\n' \"$@\" >> \"$STAND/curl\""),
    (
        "df",
        "echo Avail\ncat \"$STAND/free\" 2> /dev/null || echo 100G",
    ),
    ("dmsetup", "cat \"$STAND/snapshot\" 2> /dev/null\nexit 0"),
    // The heartbeat's sleep: two seconds a beat, counted; and after eight beats it ends a heartbeat left running.
    (
        "sleep",
        "if [ \"$1\" != 30 ]; then exec /bin/sleep \"$@\"; fi\necho beat >> \"$STAND/beats\"\n/bin/sleep 2\nn=$(wc -l < \"$STAND/beats\" 2> /dev/null)\nif [ \"${n:-0}\" -ge 8 ] && ps -o command= -p \"$PPID\" | grep -q contain.sh; then kill -9 \"$PPID\"; fi\nexit 0",
    ),
];

impl Stand {
    fn new(tag: &str, script: &str) -> Self {
        let dir = Scratch::new(tag);
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("scratch");
        for (name, body) in STUBS {
            write_executable(&bin.join(name), format!("#!/bin/bash\n{body}\n")).expect("stand-in");
        }
        let path = dir.join("contain.sh");
        write_executable(&path, script).expect("the script");
        Stand { dir, script: path }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).unwrap_or_default()
    }

    /// An executable shell script in the stand, which `$STAND` points at.
    fn command_script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.path(name);
        write_executable(&path, format!("#!/bin/bash\n{body}\n")).expect("command");
        path
    }

    /// `/bin/bash -c <bounds> <args>`, with this stand's tools first, and neither opt-in from a run this test may be in.
    fn bounded(&self, args: &[&str]) -> Command {
        let mut command = Command::new("/bin/bash");
        command
            .args(["-c", BOUNDED, "bounded"])
            .args(args)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.path("bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .env("STAND", self.dir.as_os_str())
            .env("RUNNER_TEMP", self.dir.as_os_str())
            .env_remove("PRIN_TEST_PIDNS")
            .env_remove("HEARTBEAT_CONTEXT")
            .env_remove("HEARTBEAT_SHA")
            .env_remove("HEARTBEAT_TOKEN")
            .env_remove("HEARTBEAT_OUT")
            .env("PRIN_CONTAIN", "required");
        command
    }

    /// A driver script, run bounded, with `$SCRIPT` the script under test.
    fn drive(&self, body: &str) -> Output {
        let driver = self.command_script("driver", body);
        let mut command = self.bounded(&[driver.to_str().expect("utf-8 path")]);
        command
            .env("SCRIPT", &self.script)
            .output_within(Duration::from_secs(90))
            .expect("the driver ran")
    }

    /// `contain.sh run <command>`, as a mutants step runs it, with the heartbeat's settings.
    fn run(&self, command: &Path, args: &[&str]) -> Output {
        let mut all = vec![
            self.script.to_str().expect("utf-8 path"),
            "run",
            command.to_str().expect("utf-8 path"),
        ];
        all.extend_from_slice(args);
        let log = self.path("mutants.out/log");
        std::fs::create_dir_all(&log).expect("scratch");
        std::fs::write(
            log.join("a_very_long_mutant_name_of_xtask_src_jobq_rs_line_459_col_7.log"),
            "",
        )
        .expect("scratch");
        let mut run = self.bounded(&all);
        run.env("HEARTBEAT_CONTEXT", "mutants-heartbeat (0)")
            .env("HEARTBEAT_SHA", "abc123")
            .env("HEARTBEAT_TOKEN", "sekrit-token")
            .env("HEARTBEAT_OUT", self.path("mutants.out"))
            .env("GITHUB_REPOSITORY", "owner/repo")
            .env("ODD_VALUE", "a \"b\" $c 'd' e")
            .output_within(Duration::from_secs(60))
            .expect("contain.sh ran")
    }
}

#[cfg(feature = "controls")]
fn mutated(from: &str, to: &str) -> String {
    let script = contain_sh();
    assert!(script.contains(from), "the control's text is gone: {from}");
    script.replace(from, to)
}

fn check_environment(script: &str, tag: &str) {
    let stand = Stand::new(tag, script);
    let command = stand.command_script(
        "command",
        "env > \"$STAND/env\"\nprintf '%s\\n' \"$@\" > \"$STAND/args\"",
    );
    let output = stand.run(&command, &["one two", "$three", "\"four\""]);
    assert_eq!(output.status.code(), Some(0), "the run failed: {output:?}");
    let env = stand.read("env");
    assert!(
        env.contains("ODD_VALUE=a \"b\" $c 'd' e"),
        "the command lost its environment: {env}"
    );
    assert_eq!(
        stand.read("args"),
        "one two\n$three\n\"four\"\n",
        "the command lost its arguments"
    );
    assert!(
        !env.contains("sekrit-token") && !env.contains("HEARTBEAT_TOKEN"),
        "the heartbeat's token reached the command"
    );
    assert!(
        !env.contains("PRIN_CONTAIN"),
        "the command would contain itself again"
    );
    assert!(
        env.contains("PRIN_TEST_PIDNS=1"),
        "the queue's tests would not get their namespaces"
    );
    let left: Vec<_> = std::fs::read_dir(&*stand.dir)
        .expect("scratch")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("contain-env"))
        .collect();
    assert!(left.is_empty(), "the environment file was left behind");
}

#[test]
fn contain_run_gives_the_command_its_environment_and_keeps_the_token() {
    check_environment(&contain_sh(), "qa_contain_env");
}

negative_control!(
    contain_run_gives_the_command_its_environment_and_keeps_the_token,
    "the token left in the environment",
    expected = "the heartbeat's token reached the command",
    check_environment(
        &mutated("unset HEARTBEAT_TOKEN PRIN_CONTAIN", "unset PRIN_CONTAIN"),
        "qa_contain_env_control"
    )
);

fn check_limits(script: &str, tag: &str) {
    let stand = Stand::new(tag, script);
    stand.command_script("command", "ulimit -n > \"$STAND/limit\"");
    let output = stand.drive("ulimit -Sn 300\nexec \"$SCRIPT\" run \"$STAND/command\"");
    assert_eq!(output.status.code(), Some(0), "the run failed: {output:?}");
    assert_eq!(
        stand.read("limit").trim(),
        "300",
        "the command did not get this shell's limit back from what sudo left"
    );
}

#[test]
fn contain_run_gives_the_command_this_shells_limits() {
    check_limits(&contain_sh(), "qa_contain_limits");
}

negative_control!(
    contain_run_gives_the_command_this_shells_limits,
    "sudo's limits left in place",
    expected = "did not get this shell's limit back",
    check_limits(
        &mutated("eval \"$limits\"", ":"),
        "qa_contain_limits_control"
    )
);

/// The run's status comes back, a heartbeat status says it finished, and that status never holds a merge.
fn check_status(script: &str, tag: &str) {
    let stand = Stand::new(tag, script);
    let command = stand.command_script("command", "exit 7");
    let output = stand.run(&command, &[]);
    assert_eq!(
        output.status.code(),
        Some(7),
        "the command's status did not come back"
    );
    let curl = stand.read("curl");
    assert!(
        curl.contains("Bearer sekrit-token"),
        "no authorised heartbeat: {curl}"
    );
    assert!(
        curl.contains("/repos/owner/repo/statuses/abc123"),
        "a heartbeat elsewhere: {curl}"
    );
    let finished = curl
        .lines()
        .find(|l| l.contains("finished, exit 7"))
        .unwrap_or_else(|| panic!("no heartbeat says the run finished: {curl}"));
    assert!(
        finished.contains("\"state\":\"success\""),
        "a heartbeat that could hold a merge"
    );
    let description = finished
        .split("\"description\":\"")
        .nth(1)
        .and_then(|d| d.strip_suffix("\"}"))
        .expect("the status's description");
    assert!(
        description.chars().count() <= 140,
        "over a status's 140: {description}"
    );
}

#[test]
fn contain_run_returns_the_commands_status_and_reports_it() {
    check_status(&contain_sh(), "qa_contain_status");
}

negative_control!(
    contain_run_returns_the_commands_status_and_reports_it,
    "a run that always says it passed",
    expected = "the command's status did not come back",
    check_status(
        &mutated("return \"$rc\"", "return 0"),
        "qa_contain_status_control"
    )
);

/// A run on a machine whose disk or copy-on-write store stands as given: whether it was stopped, by SIGKILL to its scope.
fn check_floor(free: &str, snapshot: &str, stops: bool, tag: &str) {
    let stand = Stand::new(tag, &contain_sh());
    std::fs::write(stand.path("free"), format!("{free}\n")).expect("scratch");
    std::fs::write(stand.path("snapshot"), snapshot).expect("scratch");
    let command = stand.command_script(
        "command",
        "echo $$ > \"$STAND/pid\"\ni=0\nwhile [ ! -e \"$STAND/stopped\" ] && [ $i -lt 25 ]; do /bin/sleep 0.2; i=$((i + 1)); done\n/bin/sleep 1",
    );
    let output = stand.run(&command, &[]);
    let stopped = stand.read("stopped");
    assert_eq!(
        stopped.contains("--signal=SIGKILL") && stopped.contains(".scope"),
        stops,
        "free {free}, snapshot {snapshot:?}: stopped says {stopped:?}, wanted {stops}"
    );
    if stops {
        assert_eq!(
            output.status.code(),
            Some(137),
            "a stopped run reads as passed: {output:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("::error::contain.sh")
                && stand.read("curl").contains("disk floor: stopped"),
            "a stopped run does not say why: {output:?}"
        );
    }
}

#[test]
fn contain_run_stops_before_the_disk_fills_and_only_then() {
    check_floor("100G", "", false, "qa_contain_floor_room");
    check_floor("1G", "", true, "qa_contain_floor_full");
    check_floor(
        "100G",
        "cow-root: 0 151 snapshot 50/100 8\n",
        false,
        "qa_contain_cow_room",
    );
    check_floor(
        "100G",
        "cow-root: 0 151 snapshot 99/100 8\n",
        true,
        "qa_contain_cow_full",
    );
    check_floor(
        "100G",
        "other: 0 151 linear 99/100 8\n",
        false,
        "qa_contain_no_snapshot",
    );
}

negative_control!(
    contain_run_stops_before_the_disk_fills_and_only_then,
    "a machine with room is not stopped, so the check can tell",
    expected = "wanted true",
    check_floor("100G", "", true, "qa_contain_floor_control")
);

/// `test`, opted in: a PID namespace asked for; a SIGTERM to its init reaches the test, whose status comes back.
fn check_test_mode(script: &str, tag: &str) {
    let stand = Stand::new(tag, script);
    stand.command_script(
        "command",
        "trap 'echo got > \"$STAND/marker\"; exit 9' TERM\necho $$ > \"$STAND/pid\"\ni=0\nwhile [ $i -lt 40 ]; do /bin/sleep 0.2; i=$((i + 1)); done\nexit 3",
    );
    let output = stand.drive(
        "export PRIN_TEST_PIDNS=1\n\"$SCRIPT\" test \"$STAND/command\" > /dev/null 2>&1 &\npid=$!\ni=0\nwhile [ ! -s \"$STAND/pid\" ] && [ $i -lt 50 ]; do /bin/sleep 0.2; i=$((i + 1)); done\nkill -TERM $pid\nwait $pid\necho \"status=$?\"",
    );
    let unshare = stand.read("unshare");
    for flag in ["--user", "--pid", "--fork", "--kill-child", "--mount-proc"] {
        assert!(
            unshare.contains(flag),
            "`test` does not ask for {flag}: {unshare}"
        );
    }
    assert!(
        !stand.read("marker").is_empty(),
        "the init did not pass on the SIGTERM"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("status=9"),
        "the test's status did not come back: {output:?}"
    );
}

#[test]
fn contain_test_asks_for_a_namespace_and_passes_on_a_sigterm() {
    check_test_mode(&contain_sh(), "qa_contain_test");
}

negative_control!(
    contain_test_asks_for_a_namespace_and_passes_on_a_sigterm,
    "an init that does not pass the signal on",
    expected = "did not pass on the SIGTERM",
    check_test_mode(
        &mutated(
            "trap '\\''kill -TERM \"$child\" 2> /dev/null'\\'' TERM INT",
            ":"
        ),
        "qa_contain_test_control"
    )
);

/// A step's limit, or a cancelled run, signals the step's shell, this script, and not what it runs under `sudo`, whose
/// processes the runner's user cannot signal. The script's header: "all of it dies with the namespace"; `mutants.yml`
/// § "Shard cut off or failed": the upload follows a cut-off shard. So a SIGTERM to `run` must end the command and the
/// heartbeat, which otherwise go on through the upload, with the run's report still being written.
fn check_stopped_step(script: &str, tag: &str) {
    let stand = Stand::new(tag, script);
    stand.command_script(
        "command",
        "echo $$ > \"$STAND/pid\"\ni=0\nwhile [ $i -lt 60 ]; do /bin/sleep 0.2; i=$((i + 1)); done",
    );
    let output = stand.drive(
        "\"$SCRIPT\" run \"$STAND/command\" > /dev/null 2>&1 &\npid=$!\ni=0\nwhile [ ! -s \"$STAND/pid\" ] && [ $i -lt 50 ]; do /bin/sleep 0.2; i=$((i + 1)); done\nkill -TERM $pid\nwait $pid\n/bin/sleep 1\nkill -0 \"$(cat \"$STAND/pid\")\" 2> /dev/null && echo command-alive\nn=$(wc -l < \"$STAND/beats\")\n/bin/sleep 5\nm=$(wc -l < \"$STAND/beats\")\n[ \"$m\" -gt \"$n\" ] && echo heartbeat-alive\nexit 0",
    );
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        !out.contains("command-alive"),
        "the contained command outlived its stopped step"
    );
    assert!(
        !out.contains("heartbeat-alive"),
        "the heartbeat outlived its stopped step"
    );
}

#[test]
fn contain_run_stopped_by_a_signal_ends_what_it_contained() {
    check_stopped_step(&contain_sh(), "qa_contain_stopped");
}

negative_control!(
    contain_run_stopped_by_a_signal_ends_what_it_contained,
    "a known-bad wrapper, which leaves its command running",
    expected = "the contained command outlived its stopped step",
    check_stopped_step(
        "#!/bin/bash\nsleep 30 > /dev/null 2>&1 &\n\"$2\" > /dev/null 2>&1 &\nwait\n",
        "qa_contain_stopped_control"
    )
);
