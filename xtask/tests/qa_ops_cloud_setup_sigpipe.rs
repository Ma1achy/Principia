//! qa's tests for the cloud-setup channel check (PR #136, an ops fix with no task file), written from the script's
//! contract, not from its code:
//! - `scripts/cloud-setup.sh --dry-run` accepts a root toolchain file that names a channel, and its plan lists that
//!   channel (`toolchain file <channel> rustup`);
//! - it refuses a root toolchain file that names no channel, with "the root toolchain file names no channel";
//! - neither result depends on timing: the script runs under `set -o pipefail`, so it must not hand the toolchain
//!   records to a reader that may quit at its first match (`grep -q`) through a pipe, whose writer then dies of
//!   SIGPIPE and fails the check at random.
//!
//! The timing is taken out of it: the dry run runs on a scratch copy of the script and of what it reads, with a root
//! toolchain file whose records are far larger than any pipe buffer and name the channel first, and with a stand-in
//! `grep` first on PATH that behaves as `grep -q` may, reading one line at a time and quitting at its first match.
//! Fed through a pipe, the writer is then still blocked on the full buffer when the reader quits, and dies of SIGPIPE
//! every time; the old form of the check (`printf '%s\n' "$tc" | grep -q '^channel '`) is each control's input, and
//! fails. Each test registers its negative control (R-176), with a scratch folder of its own (R-333).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use validation::spawn::{write_executable, Spawn};

/// The channel the scratch toolchain files name.
const CHANNEL: &str = "nightly-qa-sigpipe";

/// The channel check as it was before the fix: a pipe into `grep -q`.
#[cfg(feature = "controls")]
const OLD_CHECK: &str = r#"printf '%s\n' "$tc" | grep -q '^channel '"#;

/// The refusal the contract names for a toolchain file without a channel.
const NO_CHANNEL: &str = "the root toolchain file names no channel";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn script() -> String {
    fs::read_to_string(root().join("scripts/cloud-setup.sh")).unwrap()
}

/// The script with its channel check in the old, piped form. The script's check must be the one this replaces: a
/// control whose edit target is gone fails here, in its setup, not at the check.
#[cfg(feature = "controls")]
fn old_form_script() -> String {
    let text = script();
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("grep -q '^channel '"))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "qa: the script has one channel check: {lines:?}"
    );
    let check = lines[0];
    let start = check.find("grep -q").unwrap();
    let end = check.find("; then").unwrap();
    text.replace(check, &check.replace(&check[start..end], OLD_CHECK))
}

/// A root toolchain file of well over a pipe buffer (1 MiB in all): the channel, when `with_channel`, first, then
/// many long component names.
fn big_toolchain(with_channel: bool) -> String {
    let mut text = String::from("[toolchain]\n");
    if with_channel {
        text.push_str(&format!("channel = \"{CHANNEL}\"\n"));
    }
    let names: Vec<String> = (0..512)
        .map(|i| format!("\"c{i:04}-{}\"", "x".repeat(2040)))
        .collect();
    text.push_str(&format!("components = [{}]\n", names.join(", ")));
    assert!(text.len() > 1 << 20, "qa: the toolchain file is over 1 MiB");
    text
}

/// A stand-in `grep -q PATTERN` that reads its input one line at a time and quits at its first matching line, as
/// `grep -q` may: so a pipe's writer still has data to write when it goes, whatever the machine's timing.
const EARLY_GREP: &str = r#"#!/usr/bin/env bash
[ "$#" -eq 2 ] && [ "$1" = -q ] || { echo "qa stand-in grep: only -q PATTERN, given: $*" >&2; exit 2; }
while IFS= read -r line; do
  [[ $line =~ $2 ]] && exit 0
done
exit 1
"#;

/// A scratch tree of its own for `case`, emptied first: the script (`script_text`), CI's workflows, `toolchain` as the
/// root toolchain file, and, when `stand_in`, a `bin/grep` that quits at its first match.
fn tree(case: &str, script_text: &str, toolchain: &str, stand_in: bool) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_ops_sigpipe_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::write(dir.join("scripts/cloud-setup.sh"), script_text).unwrap();
    for entry in fs::read_dir(root().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
            fs::copy(
                &path,
                dir.join(".github/workflows")
                    .join(path.file_name().unwrap()),
            )
            .unwrap();
        }
    }
    fs::write(dir.join("rust-toolchain.toml"), toolchain).unwrap();
    if stand_in {
        write_executable(&dir.join("bin/grep"), EARLY_GREP).unwrap();
    }
    dir
}

/// `bash scripts/cloud-setup.sh --dry-run` in the tree at `dir`, with its `bin` first on PATH.
fn run_dry(dir: &Path) -> Output {
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("bash")
        .arg(dir.join("scripts/cloud-setup.sh"))
        .arg("--dry-run")
        .env("PATH", path)
        .timed_output()
        .expect("run bash")
}

/// The dry run in the tree at `dir` accepts its toolchain file, `runs` times out of `runs`, and lists its channel.
fn check_accepted(dir: &Path, runs: usize) {
    for run in 1..=runs {
        let output = run_dry(dir);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "the dry run refused a toolchain file that names a channel (run {run} of {runs}, {}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout
                .lines()
                .any(|l| l == format!("toolchain file {CHANNEL} rustup")),
            "the dry run's plan does not list the toolchain file's channel `{CHANNEL}` (run {run} of {runs})"
        );
    }
}

/// The dry run in the tree at `dir` refuses its toolchain file, naming the missing channel.
fn check_refused(dir: &Path) {
    let output = run_dry(dir);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && stderr.contains(NO_CHANNEL),
        "the dry run did not refuse a toolchain file that names no channel with `{NO_CHANNEL}` ({}):\n{stderr}",
        output.status
    );
}

#[test]
fn dry_run_accepts_a_channel_when_grep_quits_at_its_first_match() {
    let dir = tree("accept_early", &script(), &big_toolchain(true), true);
    check_accepted(&dir, 3);
}

validation::negative_control!(
    dry_run_accepts_a_channel_when_grep_quits_at_its_first_match,
    "the old channel check, a pipe into `grep -q` under pipefail, required to accept a channel when grep quits at its \
     first match",
    expected = "the dry run refused a toolchain file that names a channel",
    {
        let dir = tree(
            "accept_early_control",
            &old_form_script(),
            &big_toolchain(true),
            true,
        );
        check_accepted(&dir, 3)
    }
);

#[test]
fn dry_run_accepts_a_channel_with_the_system_grep() {
    let dir = tree("accept_system", &script(), &big_toolchain(true), false);
    check_accepted(&dir, 5);
}

validation::negative_control!(
    dry_run_accepts_a_channel_with_the_system_grep,
    "a toolchain file without its channel, required to be accepted with the channel listed",
    expected = "the dry run refused a toolchain file that names a channel",
    {
        let dir = tree(
            "accept_system_control",
            &script(),
            &big_toolchain(false),
            false,
        );
        check_accepted(&dir, 1)
    }
);

#[test]
fn dry_run_refuses_a_toolchain_file_that_names_no_channel() {
    check_refused(&tree(
        "refuse_early",
        &script(),
        &big_toolchain(false),
        true,
    ));
    check_refused(&tree(
        "refuse_system",
        &script(),
        &big_toolchain(false),
        false,
    ));
    check_refused(&tree(
        "refuse_small",
        &script(),
        "[toolchain]\ncomponents = [\"rust-src\"]\n",
        false,
    ));
}

validation::negative_control!(
    dry_run_refuses_a_toolchain_file_that_names_no_channel,
    "a toolchain file that names its channel, required to be refused for naming none",
    expected = "the dry run did not refuse a toolchain file that names no channel",
    check_refused(&tree(
        "refuse_control",
        &script(),
        &big_toolchain(true),
        true,
    ))
);
