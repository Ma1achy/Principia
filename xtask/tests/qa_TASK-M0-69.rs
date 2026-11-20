//! QA's tests for TASK-M0-69, written from REQ-TOOL-242's statement and verify detail (R-1010, R-915, R-912), not
//! from how the change is built:
//!
//! - `scripts/wsl/wsl.conf` is the owner's file, unchanged, followed by R-1010's two additions and nothing else:
//!   `[gpu] enabled=false`, and a `[boot]` section holding one `command=`; no `systemd` key anywhere, no sudo;
//! - that boot command, run by a shell, writes `-1` to `/proc/sys/fs/binfmt_misc/WSLInterop` (which removes the
//!   registration) and does nothing else: it is run here with only that path pointed at a scratch file;
//! - ci's cargo config in the image holds the bytes `.github/actions/cargo-ci-config/config.sh` gives a hosted job's
//!   fresh CARGO_HOME (both are run, and their outputs compared), installed root's and 0644 after every command run
//!   as ci, and nothing else in the build writes it;
//! - `inside.sh parity` fails when no test ran, whatever cargo's exit, printing the end of cargo's log on stderr, and
//!   prints it whenever cargo exits non-zero; a run whose tests passed passes quietly.
//!
//! Each test has its negative controls (R-176): an edited copy that breaks the guarantee, which the same check must
//! reject.

use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

use validation::negative_control;

#[cfg(unix)]
#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

const CONF: &str = "scripts/wsl/wsl.conf";
#[cfg(unix)]
const BUILD: &str = "scripts/wsl/build-image.sh";
#[cfg(unix)]
const CARGO_CI_CONFIG: &str = ".github/actions/cargo-ci-config/config.sh";
#[cfg(unix)]
const INSIDE: &str = "scripts/wsl/inside.sh";
#[cfg(unix)]
const PARITY: &str = "scripts/wsl/parity.py";
/// The registration R-1010's boot command must remove.
#[cfg(unix)]
const WSL_INTEROP: &str = "/proc/sys/fs/binfmt_misc/WSLInterop";

/// The file the owner gave (R-915; `scripts/wsl/wsl.conf` before R-1010), byte for byte.
const OWNERS_FILE: &str =
    "[automount]\nenabled=false\n\n[interop]\nenabled=false\nappendWindowsPath=false\n\n[user]\ndefault=ci\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` with `from` replaced by `to`, once; the edit's target must be there.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa control: the edit's target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// An INI file as WSL reads `wsl.conf`: its sections in order, each with its `key=value` lines in order. Blank lines
/// and `#` comments are skipped; any other line outside `[...]` or without `=` is refused.
fn sections(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push((name.trim().to_owned(), Vec::new()));
            continue;
        }
        let (k, v) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("wsl.conf: a line that is no key=value: {line}"));
        out.last_mut()
            .unwrap_or_else(|| panic!("wsl.conf: a key outside any section: {line}"))
            .1
            .push((k.trim().to_owned(), v.trim().to_owned()));
    }
    out
}

// ---- wsl.conf: the owner's file and R-1010's two additions --------------------------------------------------------

fn check_conf(conf: &str) {
    let parsed = sections(conf);
    for (section, keys) in &parsed {
        for (k, _) in keys {
            assert!(
                !k.eq_ignore_ascii_case("systemd"),
                "wsl.conf sets systemd in [{section}]"
            );
        }
    }
    assert!(
        !conf.to_ascii_lowercase().contains("sudo"),
        "wsl.conf names sudo"
    );
    let rest = conf.strip_prefix(OWNERS_FILE).unwrap_or_else(|| {
        panic!("wsl.conf doesn't start with the owner's file, unchanged:\n{conf}")
    });
    let added = sections(rest);
    let names: Vec<&str> = added.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["gpu", "boot"],
        "wsl.conf adds sections other than R-1010's [gpu] and [boot]"
    );
    assert_eq!(
        added[0].1,
        [("enabled".to_owned(), "false".to_owned())],
        "wsl.conf's [gpu] isn't `enabled=false` alone"
    );
    assert_eq!(
        added[1].1.len(),
        1,
        "wsl.conf's [boot] holds other than one line: {:?}",
        added[1].1
    );
    assert_eq!(
        added[1].1[0].0, "command",
        "wsl.conf's [boot] line isn't a command="
    );
    assert_eq!(
        rest,
        format!(
            "\n[gpu]\nenabled=false\n\n[boot]\ncommand={}\n",
            added[1].1[0].1
        ),
        "wsl.conf's two additions aren't laid out as the owner's sections are"
    );
}

#[test]
fn qa_wsl_conf_is_the_owners_with_r1010s_two_additions() {
    check_conf(&read(CONF));
}

mod conf_systemd_in_boot {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "systemd=true beside the boot command",
        expected = "wsl.conf sets systemd in [boot]",
        check_conf(&format!("{}systemd=true\n", read(CONF)))
    );
}

mod conf_systemd_in_user {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "systemd in the owner's [user] section",
        expected = "wsl.conf sets systemd in [user]",
        check_conf(&edited(
            &read(CONF),
            "default=ci\n",
            "default=ci\nSystemd = false\n"
        ))
    );
}

mod conf_owner_changed {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "the owner's default user changed",
        expected = "wsl.conf doesn't start with the owner's file, unchanged",
        check_conf(&edited(&read(CONF), "default=ci\n", "default=root\n"))
    );
}

mod conf_interop_on {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "interop switched on",
        expected = "wsl.conf doesn't start with the owner's file, unchanged",
        check_conf(&edited(
            &read(CONF),
            "[interop]\nenabled=false\n",
            "[interop]\nenabled=true\n"
        ))
    );
}

mod conf_gpu_on {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "the GPU share left on",
        expected = "wsl.conf's [gpu] isn't `enabled=false` alone",
        check_conf(&edited(
            &read(CONF),
            "[gpu]\nenabled=false\n",
            "[gpu]\nenabled=true\n"
        ))
    );
}

mod conf_gpu_missing {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "no [gpu] section",
        expected = "wsl.conf adds sections other than R-1010's [gpu] and [boot]",
        check_conf(&edited(&read(CONF), "\n[gpu]\nenabled=false\n", ""))
    );
}

mod conf_boot_missing {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "no [boot] section",
        expected = "wsl.conf adds sections other than R-1010's [gpu] and [boot]",
        {
            let conf = read(CONF);
            let cut = conf.find("\n[boot]").expect("qa control: [boot] is there");
            check_conf(&conf[..cut]);
        }
    );
}

mod conf_boot_two_lines {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "a second boot line",
        expected = "wsl.conf's [boot] holds other than one line",
        check_conf(&format!("{}protectBinfmt=false\n", read(CONF)))
    );
}

mod conf_third_section {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "a third section added",
        expected = "wsl.conf adds sections other than R-1010's [gpu] and [boot]",
        check_conf(&format!("{}\n[network]\ngenerateHosts=false\n", read(CONF)))
    );
}

mod conf_sudo {
    use super::*;
    negative_control!(
        qa_wsl_conf_is_the_owners_with_r1010s_two_additions,
        "the boot command run through sudo",
        expected = "wsl.conf names sudo",
        check_conf(&edited(&read(CONF), "command=", "command=sudo "))
    );
}

// ---- The boot command, run ----------------------------------------------------------------------------------------

/// The boot command, `command=`'s value in `[boot]`, run by `/bin/sh -c` in an empty scratch folder with only
/// `/proc/sys/fs/binfmt_misc/WSLInterop` pointed at a scratch file there: it must name that path once, write `-1` to
/// it (what removes a binfmt_misc registration) and leave nothing else behind.
#[cfg(unix)]
fn check_boot_command(conf: &str) {
    let command = sections(conf)
        .into_iter()
        .find(|(n, _)| n == "boot")
        .and_then(|(_, keys)| keys.into_iter().find(|(k, _)| k == "command"))
        .map(|(_, v)| v)
        .expect("wsl.conf has no [boot] command=");
    let named = command.matches(WSL_INTEROP).count();
    let late = command.matches(&format!("{WSL_INTEROP}-late")).count();
    assert_eq!(
        named - late,
        1,
        "the boot command doesn't name {WSL_INTEROP} once: {command}"
    );
    let dir = scratch::Scratch::new("qa_m0_69_boot");
    let cwd = dir.join("cwd");
    std::fs::create_dir_all(&cwd).expect("scratch");
    let target = dir.join("WSLInterop");
    std::fs::write(&target, "enabled\n").expect("scratch");
    // Quoted: the scratch path may hold a space.
    let pointed = command.replace(WSL_INTEROP, &format!("'{}'", target.display()));
    let out = Command::new("/bin/sh")
        .args(["-c", &pointed])
        .current_dir(&cwd)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("sh ran");
    assert!(
        out.status.success(),
        "the boot command fails: {command}: {out:?}"
    );
    let written = std::fs::read_to_string(&target).expect("scratch");
    assert_eq!(
        written.trim_end_matches('\n'),
        "-1",
        "the boot command doesn't write -1 to {WSL_INTEROP}: {command}"
    );
    let left: Vec<_> = std::fs::read_dir(&cwd).expect("scratch").collect();
    assert!(
        left.is_empty() && out.stdout.is_empty(),
        "the boot command does more than remove the registration: {command}"
    );
}

#[cfg(unix)]
#[test]
fn qa_boot_command_removes_the_wslinterop_registration() {
    check_boot_command(&read(CONF));
}

#[cfg(unix)]
mod boot_enables {
    use super::*;
    negative_control!(
        qa_boot_command_removes_the_wslinterop_registration,
        "the registration enabled, not removed",
        expected = "the boot command doesn't write -1 to",
        check_boot_command(&edited(&read(CONF), "echo -1 >", "echo 1 >"))
    );
}

#[cfg(unix)]
mod boot_late_only {
    use super::*;
    negative_control!(
        qa_boot_command_removes_the_wslinterop_registration,
        "only WSLInterop-late removed",
        expected = "the boot command doesn't name /proc/sys/fs/binfmt_misc/WSLInterop once",
        check_boot_command(&edited(
            &read(CONF),
            "/proc/sys/fs/binfmt_misc/WSLInterop\n",
            "/proc/sys/fs/binfmt_misc/WSLInterop-late\n"
        ))
    );
}

#[cfg(unix)]
mod boot_more {
    use super::*;
    negative_control!(
        qa_boot_command_removes_the_wslinterop_registration,
        "a second command chained after the removal",
        expected = "the boot command does more than remove the registration",
        check_boot_command(&edited(
            &read(CONF),
            "/proc/sys/fs/binfmt_misc/WSLInterop\n",
            "/proc/sys/fs/binfmt_misc/WSLInterop; touch mounted\n"
        ))
    );
}

// ---- ci's cargo config in the image -------------------------------------------------------------------------------

/// The build's lines that are commands, not comments, trimmed.
#[cfg(unix)]
fn commands(build: &str) -> Vec<&str> {
    build
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// The build's lines naming ci's cargo config: the one writing the build's copy (`$work/cargo-config.toml`), run here
/// with `work` a scratch folder, must give the bytes `config.sh` appends to a fresh CARGO_HOME, run here too; the one
/// putting it at `/home/ci/.cargo/config.toml` must be `install` as root with mode 0644 and owner and group 0, after
/// every command run as ci; no other line names the file, and none after the install changes ci's home's owners or
/// modes.
#[cfg(unix)]
fn check_cargo_config(build: &str, config_sh: &str) {
    let lines = commands(build);
    let naming: Vec<(usize, &str)> = lines
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, l)| l.contains("config.toml"))
        .collect();
    let write: Vec<&(usize, &str)> = naming
        .iter()
        .filter(|(_, l)| l.contains("> \"$work/cargo-config.toml\""))
        .collect();
    assert_eq!(
        write.len(),
        1,
        "the build doesn't write its one copy of ci's cargo config: {naming:?}"
    );
    let install: Vec<&(usize, &str)> = naming
        .iter()
        .filter(|(_, l)| l.ends_with("\"$root/home/ci/.cargo/config.toml\""))
        .collect();
    assert_eq!(
        naming.len(),
        2,
        "the build names ci's cargo config other than in its copy and its install: {naming:?}"
    );
    assert_eq!(
        install.len(),
        1,
        "the build doesn't install ci's cargo config: {naming:?}"
    );
    let (at, line) = *install[0];
    let words: Vec<&str> = line.split_whitespace().collect();
    let flag = |f: &str| {
        words
            .iter()
            .position(|w| *w == f)
            .and_then(|i| words.get(i + 1))
            .copied()
    };
    assert!(
        words.starts_with(&["sudo", "install"])
            && flag("-m") == Some("0644")
            && flag("-o") == Some("0")
            && flag("-g") == Some("0")
            && words.get(words.len() - 2) == Some(&"\"$work/cargo-config.toml\""),
        "ci's cargo config isn't installed as root's, 0644, from the build's copy: {line}"
    );
    assert!(
        !lines[at + 1..].iter().any(|l| l.starts_with("as_ci ")),
        "a command runs as ci after ci's cargo config is installed"
    );
    assert!(
        !lines[at + 1..]
            .iter()
            .any(|l| (l.contains("chown") || l.contains("chmod")) && l.contains("home")),
        "the build changes ci's home's owners or modes after installing its cargo config"
    );

    let dir = scratch::Scratch::new("qa_m0_69_cargo");
    let work = dir.join("work");
    let home = dir.join("cargo-home");
    std::fs::create_dir_all(&work).expect("scratch");
    let out = Command::new("/bin/sh")
        .args(["-c", write[0].1])
        .env("work", &work)
        .output()
        .expect("sh ran");
    assert!(
        out.status.success(),
        "the build's cargo config line fails: {out:?}"
    );
    let image =
        std::fs::read(work.join("cargo-config.toml")).expect("the build's copy was written");
    let script = dir.join("config.sh");
    std::fs::write(&script, config_sh).expect("scratch");
    let out = Command::new("bash")
        .arg(&script)
        .env("CARGO_HOME", &home)
        .output()
        .expect("bash ran");
    assert!(out.status.success(), "config.sh fails: {out:?}");
    let hosted = std::fs::read(home.join("config.toml")).expect("config.sh wrote the config");
    assert_eq!(
        String::from_utf8_lossy(&image),
        String::from_utf8_lossy(&hosted),
        "ci's cargo config in the image isn't the bytes config.sh gives a hosted job"
    );
}

#[cfg(unix)]
#[test]
fn qa_image_cargo_config_is_cis_bytes_and_roots() {
    check_cargo_config(&read(BUILD), &read(CARGO_CI_CONFIG));
}

#[cfg(all(unix, feature = "controls"))]
const INSTALL: &str =
    "sudo install -m 0644 -o 0 -g 0 \"$work/cargo-config.toml\" \"$root/home/ci/.cargo/config.toml\"";

#[cfg(unix)]
mod cargo_as_ci {
    use super::*;
    negative_control!(
        qa_image_cargo_config_is_cis_bytes_and_roots,
        "ci's cargo config written by ci, as the image did before",
        expected = "the build doesn't install ci's cargo config",
        check_cargo_config(
            &edited(
                &read(BUILD),
                INSTALL,
                "as_ci cp /tmp/cargo-config.toml /home/ci/.cargo/config.toml"
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

#[cfg(unix)]
mod cargo_world_writable {
    use super::*;
    negative_control!(
        qa_image_cargo_config_is_cis_bytes_and_roots,
        "the config installed writable by all",
        expected = "ci's cargo config isn't installed as root's, 0644, from the build's copy",
        check_cargo_config(
            &edited(
                &read(BUILD),
                "install -m 0644 -o 0 -g 0 \"$work/cargo",
                "install -m 0666 -o 0 -g 0 \"$work/cargo"
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

#[cfg(unix)]
mod cargo_cis_group {
    use super::*;
    negative_control!(
        qa_image_cargo_config_is_cis_bytes_and_roots,
        "the config installed in ci's group",
        expected = "ci's cargo config isn't installed as root's, 0644, from the build's copy",
        check_cargo_config(
            &edited(
                &read(BUILD),
                "-o 0 -g 0 \"$work/cargo",
                "-o 0 -g 1000 \"$work/cargo"
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

#[cfg(unix)]
mod cargo_ci_after {
    use super::*;
    negative_control!(
        qa_image_cargo_config_is_cis_bytes_and_roots,
        "a command run as ci after the install",
        expected = "a command runs as ci after ci's cargo config is installed",
        check_cargo_config(
            &edited(
                &read(BUILD),
                INSTALL,
                &format!("{INSTALL}\nas_ci cargo --version")
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

#[cfg(unix)]
mod cargo_chown_after {
    use super::*;
    negative_control!(
        qa_image_cargo_config_is_cis_bytes_and_roots,
        "ci's home handed back after the install",
        expected =
            "the build changes ci's home's owners or modes after installing its cargo config",
        check_cargo_config(
            &edited(
                &read(BUILD),
                INSTALL,
                &format!("{INSTALL}\nsudo chown -R 1000:1000 \"$root/home/ci\"")
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

#[cfg(unix)]
mod cargo_other_bytes {
    use super::*;
    negative_control!(
        qa_image_cargo_config_is_cis_bytes_and_roots,
        "CI's config changed and the image's not",
        expected = "ci's cargo config in the image isn't the bytes config.sh gives a hosted job",
        check_cargo_config(
            &read(BUILD),
            &edited(
                &read(CARGO_CI_CONFIG),
                "[profile.test.package.\"*\"]\\ndebug = false",
                "[profile.test.package.\"*\"]\\ndebug = 1"
            )
        )
    );
}

// ---- inside.sh parity on an empty run -----------------------------------------------------------------------------

/// The first program named `name` on this test's PATH.
#[cfg(unix)]
fn which(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("no {name} on PATH"))
}

/// The last line of the fake cargo's log, which `parity` must show on stderr.
#[cfg(unix)]
const LOG_END: &str = "error: qa-m0-69 the last line of cargo's log";

/// `inside` at `scripts/wsl/inside.sh` of a scratch git repository, with the real parity.py, run as
/// `inside.sh parity <its commit>` with HOME a scratch home whose `.cargo/bin` holds git, python3 and a `cargo` that
/// prints `events` on stdout, 60 lines of log on stderr ending with [`LOG_END`], and exits `exit`. Returns the verb's
/// exit code, stdout and stderr.
#[cfg(unix)]
fn parity(inside: &str, events: &str, exit: i32) -> (Option<i32>, String, String) {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch::Scratch::new("qa_m0_69_parity");
    let repo = dir.join("repo");
    let home = dir.join("home");
    let bin = home.join(".cargo/bin");
    std::fs::create_dir_all(repo.join("scripts/wsl")).expect("scratch");
    std::fs::create_dir_all(&bin).expect("scratch");
    std::fs::write(repo.join(INSIDE), inside).expect("scratch");
    std::fs::write(repo.join(PARITY), read(PARITY)).expect("scratch");
    for tool in ["git", "python3"] {
        std::os::unix::fs::symlink(which(tool), bin.join(tool)).expect("scratch");
    }
    std::fs::write(dir.join("events"), events).expect("scratch");
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        format!(
            "#!/bin/sh\ncat '{}'\ni=1\nwhile [ $i -le 59 ]; do echo \"cargo log line $i\" >&2; i=$((i+1)); done\necho \"{LOG_END}\" >&2\nexit {exit}\n",
            dir.join("events").display()
        ),
    )
    .expect("scratch");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("scratch");
    let git = |args: &[&str]| {
        let out = Command::new(bin.join("git"))
            .args([
                "-c",
                "user.name=qa",
                "-c",
                "user.email=qa@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-C",
            ])
            .arg(&repo)
            .args(args)
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git ran");
        assert!(out.status.success(), "qa scratch git {args:?}: {out:?}");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "qa"]);
    let commit = git(&["rev-parse", "HEAD"]);
    let out = Command::new("bash")
        .arg(repo.join(INSIDE))
        .args(["parity", &commit])
        .env("HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("bash ran");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[cfg(unix)]
const ONE_OK: &str = "{\"type\":\"test\",\"event\":\"ok\",\"name\":\"kernel::qa\"}\n";
#[cfg(unix)]
const ONE_FAILED: &str = "{\"type\":\"test\",\"event\":\"failed\",\"name\":\"kernel::qa\"}\n";

#[cfg(unix)]
fn check_parity(inside: &str) {
    for exit in [101, 0] {
        let (code, out, err) = parity(inside, "", exit);
        assert!(
            code.is_some_and(|c| c != 0),
            "parity passes with no test run, cargo exiting {exit}: {code:?}\n{out}\n{err}"
        );
        assert!(
            err.contains(LOG_END) && !out.contains(LOG_END),
            "parity doesn't print the end of cargo's log on stderr with no test run, cargo exiting {exit}:\n{out}\n{err}"
        );
    }
    let (_, out, err) = parity(inside, ONE_FAILED, 100);
    assert!(
        err.contains(LOG_END) && !out.contains(LOG_END),
        "parity doesn't print the end of cargo's log on stderr when cargo exits non-zero:\n{out}\n{err}"
    );
    let (code, out, err) = parity(inside, ONE_OK, 0);
    assert!(
        code == Some(0) && out.contains("tests 1"),
        "parity fails a run whose one test passed: {code:?}\n{out}\n{err}"
    );
    assert!(
        !err.contains(LOG_END),
        "parity prints cargo's log for a run that passed:\n{err}"
    );
}

#[cfg(unix)]
#[test]
fn qa_parity_fails_when_no_test_ran() {
    check_parity(&read(INSIDE));
}

/// `inside.sh` with its `parity` function's empty-run lines (from the `tests=` line to the function's end) replaced
/// by `body`.
#[cfg(all(unix, feature = "controls"))]
fn parity_ending(body: &str) -> String {
    let inside = read(INSIDE);
    let start = inside
        .find("  tests=$(sed")
        .expect("qa control: parity's tests= line is there");
    let end = start
        + inside[start..]
            .find("\n}\n")
            .expect("qa control: parity's end is there");
    format!("{}{body}{}", &inside[..start], &inside[end..])
}

#[cfg(unix)]
mod parity_as_before {
    use super::*;
    negative_control!(
        qa_parity_fails_when_no_test_ran,
        "parity as it was: the summary alone",
        expected = "parity passes with no test run, cargo exiting 101",
        check_parity(&parity_ending("  :"))
    );
}

#[cfg(unix)]
mod parity_fails_on_exit_only {
    use super::*;
    negative_control!(
        qa_parity_fails_when_no_test_ran,
        "parity failing on cargo's exit alone",
        expected = "parity passes with no test run, cargo exiting 0",
        check_parity(&parity_ending(
            "  if [ \"$code\" -ne 0 ]; then tail -n 40 \"$work/parity.log\" >&2; return 1; fi"
        ))
    );
}

#[cfg(unix)]
mod parity_silent {
    use super::*;
    negative_control!(
        qa_parity_fails_when_no_test_ran,
        "parity failing with no test run, but saying nothing of cargo's log",
        expected = "parity doesn't print the end of cargo's log on stderr with no test run, cargo exiting 101",
        check_parity(&parity_ending(
            "  tests=$(sed -n '1s/^# wall .* exit .* tests \\([0-9][0-9]*\\)$/\\1/p' \"$work/parity.tsv\")\n  [ \"${tests:-0}\" -ne 0 ] || return 1"
        ))
    );
}

#[cfg(unix)]
mod parity_log_head {
    use super::*;
    negative_control!(
        qa_parity_fails_when_no_test_ran,
        "the start of cargo's log printed, not its end",
        expected = "parity doesn't print the end of cargo's log on stderr with no test run, cargo exiting 101",
        check_parity(&edited(
            &read(INSIDE),
            "tail -n 40 \"$work/parity.log\" >&2",
            "head -n 40 \"$work/parity.log\" >&2"
        ))
    );
}
