//! The guard on the self-hosted runner, the owner's PC (R-711 (lands first in its own rulings PR), items 1 and 5, and the
//! owner's job rules of 5 Nov 2026): "Add a CI guard test that fails if any workflow puts a self-hosted job under a
//! pull_request* trigger, uses an unpinned action in a self-hosted job, or makes any self-hosted job required or
//! gathered." Its results are measurements, never required checks (R-649).
//!
//! The repository is public, so a job that isn't plainly a GitHub-hosted image counts as self-hosted, whatever labels it
//! names: a job naming only `nvidia` would reach the PC, and so would one whose runner is an expression.
//!
//! `win-gpu.yml`'s `win-self-check` runs this file on the PC, nightly, its controls with it, where neither bash nor PyYAML
//! is known to exist. So it parses the workflows with a YAML crate, not with PyYAML as the other workflow tests do, and it
//! starts no process but git, which the PC has, and Windows' own PowerShell. Parsing, never scanning text: a trigger in
//! `on: [push, pull_request]` is the same trigger.
//!
//! The same goes for `.github/scripts/git_guard.ps1` (the owner, 5 Nov 2026), with a limit: off Windows there is no
//! PowerShell, so a test there reads the script's key list and its shape and runs its `git config` line, and only
//! `git_guard_script_refuses_what_is_planted` runs the script, on the PC, each night.
//!
//! Each test has its negative controls (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::spawn::Spawn;
use yaml_rust2::{Yaml, YamlLoader};

/// The measurement workflow and its jobs, so that the checks can't pass by finding nothing.
const WORKFLOW: &str = "win-gpu.yml";
const JOBS: [&str; 3] = ["win-vulkan", "win-tests", "win-self-check"];
/// The jobs that run nightly only.
const NIGHTLY_ONLY: [&str; 2] = ["win-tests", "win-self-check"];
const NOT_ON_PUSH: &str = "github.event_name != 'push'";
/// R-711 item 1's triggers, the only ones a workflow with a self-hosted job may have.
const TRIGGERS: [&str; 3] = ["push", "schedule", "workflow_dispatch"];
const GUARD: &str = "github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy'";
const LABELS: [&str; 3] = ["self-hosted", "windows", "nvidia"];
const SHELL: &str = "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File {0}";
const SELF_CHECK: &str = "E:\\runner-tools\\checks\\self-check.ps1";
const CARGO_BIN: &str = "E:\\runner-tools\\cargo\\bin";
/// What the runner's `.env` sets on E:; a job setting one takes it off the drive that keeps it from the owner's data.
const RUNNER_ENV: [&str; 5] = [
    "CARGO_HOME",
    "RUSTUP_HOME",
    "CARGO_TARGET_DIR",
    "TEMP",
    "TMP",
];
const RECORD: &str = ".github/scripts/win_measure.ps1";
/// The line that starts a script so that a script that can't load or parse still sets a failing status.
const FAIL_CLOSED: &str = "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File ";
const STATUS: &str = "exit $LASTEXITCODE";
const GIT_GUARD: &str = ".github/scripts/git_guard.ps1";
/// Where each job fetches the guard: its own commit, never the kept folder the guard distrusts.
const GIT_GUARD_URL: &str = "https://raw.githubusercontent.com/$env:GITHUB_REPOSITORY/$env:GITHUB_SHA/.github/scripts/git_guard.ps1";
const REFUSAL_LOG: &str = "WIN_REFUSAL_LOG";
/// The exact path: `[AppendOnly]::Add` prefixes `\\??\\` to it as it stands (#390).
const REFUSAL_LOG_PATH: &str = "E:\\runner-tools\\refusal-log\\refusals.log";
const LABEL: &str = "possibly faulty hardware (owner-reported)";
/// Branch protection's required checks on `main`. A self-hosted job bearing one of these names would report under it.
/// The first five are `FULL_ONLY` in `.github/scripts/ci_tier.sh`, which the test reads.
const REQUIRED: [&str; 8] = [
    "ci",
    "xtask-ci",
    "gpu-kernel",
    "gpu-metal",
    "gpu-lavapipe",
    "pr-check",
    "reviews-complete",
    "mutants-check",
];
const TIER_SCRIPT: &str = ".github/scripts/ci_tier.sh";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Every workflow, as (file name, text), sorted.
fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("the workflows")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let name = p
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&p).expect("a workflow");
            (name, text)
        })
        .collect();
    out.sort();
    out
}

/// The workflows with `from` replaced by `to`, once, in `file`.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files
        .iter_mut()
        .find(|(n, _)| n == file)
        .unwrap_or_else(|| panic!("no workflow {file}"));
    assert!(text.contains(from), "the edit's target is gone: {from}");
    *text = text.replacen(from, to, 1);
    files
}

/// `edited` on the measurement workflow.
#[cfg(feature = "controls")]
fn measured(from: &str, to: &str) -> Vec<(String, String)> {
    edited(WORKFLOW, from, to)
}

fn parse(file: &str, text: &str) -> Yaml {
    let mut docs = YamlLoader::load_from_str(text).unwrap_or_else(|e| panic!("{file}: {e}"));
    assert_eq!(docs.len(), 1, "{file} is not one document");
    docs.remove(0)
}

fn is_hosted(label: &str) -> bool {
    !label.contains("${{")
        && ["ubuntu-", "macos-", "windows-"]
            .iter()
            .any(|image| label.starts_with(image))
}

/// Whether the job may reach the PC: any job but a plain hosted image, or a call of a workflow, which answers for
/// itself here (and which a workflow with a self-hosted job refuses, by [`check_triggers`]).
fn is_self_hosted(job: &Yaml) -> bool {
    match &job["runs-on"] {
        Yaml::BadValue => job["uses"].as_str().is_none(),
        Yaml::String(label) => !is_hosted(label),
        _ => true,
    }
}

/// The hash's entries by string key.
fn entries(value: &Yaml) -> Vec<(&str, &Yaml)> {
    value
        .as_hash()
        .map(|h| {
            h.iter()
                .map(|(k, v)| (k.as_str().expect("a string key"), v))
                .collect()
        })
        .unwrap_or_default()
}

fn list(value: &Yaml) -> Vec<&str> {
    match value {
        Yaml::String(s) => vec![s.as_str()],
        Yaml::Array(a) => a.iter().filter_map(Yaml::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Every string of the document, keys among them. Comments aren't in it.
fn strings<'a>(value: &'a Yaml, out: &mut Vec<&'a str>) {
    match value {
        Yaml::String(s) => out.push(s),
        Yaml::Array(a) => a.iter().for_each(|v| strings(v, out)),
        Yaml::Hash(h) => h.iter().for_each(|(k, v)| {
            strings(k, out);
            strings(v, out);
        }),
        _ => {}
    }
}

fn all_strings(value: &Yaml) -> Vec<&str> {
    let mut out = Vec::new();
    strings(value, &mut out);
    out
}

/// The job's value of `key`, falling back to the workflow's.
fn effective<'a>(doc: &'a Yaml, job: &'a Yaml, key: &str) -> &'a Yaml {
    if job[key].is_badvalue() {
        &doc[key]
    } else {
        &job[key]
    }
}

fn steps(job: &Yaml) -> &[Yaml] {
    job["steps"].as_vec().map_or(&[], Vec::as_slice)
}

/// The workflows that hold a self-hosted job, each as (file, text, document, its self-hosted jobs by id). Finding fewer
/// than the measurement workflow's jobs fails, so no check here passes by reading nothing.
fn self_hosted(files: &[(String, String)]) -> Vec<(&str, &str, Yaml, Vec<String>)> {
    let mut out = Vec::new();
    for (file, text) in files {
        let doc = parse(file, text);
        let ids: Vec<String> = entries(&doc["jobs"])
            .into_iter()
            .filter(|(_, job)| is_self_hosted(job))
            .map(|(id, _)| id.to_owned())
            .collect();
        if !ids.is_empty() {
            out.push((file.as_str(), text.as_str(), doc, ids));
        }
    }
    let found: Vec<&String> = out
        .iter()
        .filter(|(file, _, _, _)| *file == WORKFLOW)
        .flat_map(|(_, _, _, ids)| ids)
        .collect();
    for job in JOBS {
        assert!(
            found.iter().any(|id| *id == job),
            "no self-hosted job {job} in {WORKFLOW}: {found:?}"
        );
    }
    out
}

// ---- Triggers (R-711 item 1) ------------------------------------------------------------------------------------

fn check_triggers(files: &[(String, String)]) {
    for (file, _, doc, _) in self_hosted(files) {
        let on = &doc["on"];
        let names = match on {
            Yaml::Hash(_) => entries(on).into_iter().map(|(k, _)| k).collect(),
            other => list(other),
        };
        assert!(!names.is_empty(), "{file}: no trigger read");
        for name in &names {
            assert!(
                TRIGGERS.contains(name),
                "{file}: a self-hosted job under the trigger `{name}` (R-711 item 1)"
            );
        }
        if names.contains(&"push") {
            let push = &on["push"];
            assert_eq!(
                list(&push["branches"]),
                ["main"],
                "{file}: a self-hosted job on a push to other than `main` alone"
            );
            for filter in ["branches-ignore", "tags", "tags-ignore"] {
                assert!(
                    push[filter].is_badvalue(),
                    "{file}: a self-hosted job on a push filtered by `{filter}`"
                );
            }
        }
    }
}

#[test]
fn self_hosted_jobs_have_only_the_three_triggers() {
    check_triggers(&workflows());
}

negative_control!(
    self_hosted_jobs_have_only_the_three_triggers,
    "pull_request added to the measurement workflow",
    expected = "win-gpu.yml: a self-hosted job under the trigger `pull_request`",
    check_triggers(&measured(
        "on:\n  push:\n",
        "on:\n  pull_request:\n  push:\n"
    ))
);

mod target {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "pull_request_target added to the measurement workflow",
        expected = "win-gpu.yml: a self-hosted job under the trigger `pull_request_target`",
        check_triggers(&measured(
            "on:\n  push:\n",
            "on:\n  pull_request_target:\n  push:\n"
        ))
    );
}

mod flow_form {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "pull_request in a list of triggers, which no scan for `pull_request:` sees",
        expected = "win-gpu.yml: a self-hosted job under the trigger `pull_request`",
        check_triggers(&{
            let (file, text) = workflows()
                .into_iter()
                .find(|(n, _)| n == WORKFLOW)
                .expect("the workflow");
            let at = text.find("\non:\n").expect("on:") + 1;
            let end = text.find("\npermissions:").expect("permissions:") + 1;
            let text = format!("{}on: [push, pull_request]\n{}", &text[..at], &text[end..]);
            vec![(file, text)]
        })
    );
}

mod reusable {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "workflow_call, through which any caller's pull_request would reach the job",
        expected = "win-gpu.yml: a self-hosted job under the trigger `workflow_call`",
        check_triggers(&measured(
            "  schedule:\n",
            "  workflow_call:\n  schedule:\n"
        ))
    );
}

mod after_a_run {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "workflow_run, which a pull request's run would start",
        expected = "win-gpu.yml: a self-hosted job under the trigger `workflow_run`",
        check_triggers(&measured(
            "  schedule:\n",
            "  workflow_run:\n    workflows: [ci]\n    types: [completed]\n  schedule:\n"
        ))
    );
}

mod any_branch {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "a push on every branch",
        expected = "win-gpu.yml: a self-hosted job on a push to other than `main` alone",
        check_triggers(&measured(
            "    branches: [main]\n",
            "    branches: ['**']\n"
        ))
    );
}

mod no_branch {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "a push with no branch filter",
        expected = "win-gpu.yml: a self-hosted job on a push to other than `main` alone",
        check_triggers(&measured("    branches: [main]\n", ""))
    );
}

mod tags {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "a push of a tag",
        expected = "win-gpu.yml: a self-hosted job on a push filtered by `tags`",
        check_triggers(&measured(
            "    branches: [main]\n",
            "    branches: [main]\n    tags: ['*']\n"
        ))
    );
}

mod label_alone {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "a pull request's job naming only a label of the PC, without `self-hosted`",
        expected = "ci.yml: a self-hosted job under the trigger `pull_request`",
        check_triggers(&edited(
            "ci.yml",
            "runs-on: ubuntu-24.04\n",
            "runs-on: nvidia\n"
        ))
    );
}

mod expression {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_only_the_three_triggers,
        "a pull request's job whose runner is an expression",
        expected = "ci.yml: a self-hosted job under the trigger `pull_request`",
        check_triggers(&edited(
            "ci.yml",
            "runs-on: ubuntu-24.04\n",
            "runs-on: ${{ matrix.runner }}\n"
        ))
    );
}

// ---- Pinned actions ----------------------------------------------------------------------------------------------

/// Whether `uses` names an action at a full commit SHA. A local action or a container isn't.
fn pinned(uses: &str) -> bool {
    let Some((action, sha)) = uses.split_once('@') else {
        return false;
    };
    !action.starts_with('.')
        && !action.contains(':')
        && sha.len() == 40
        && sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn check_pins(files: &[(String, String)]) {
    let mut seen = 0;
    for (file, text, doc, ids) in self_hosted(files) {
        for id in ids {
            assert!(
                doc["jobs"][id.as_str()]["uses"].is_badvalue(),
                "{file}/{id} calls a workflow"
            );
            for step in steps(&doc["jobs"][id.as_str()]) {
                let Some(uses) = step["uses"].as_str() else {
                    continue;
                };
                seen += 1;
                assert!(pinned(uses), "{file}/{id} uses an unpinned action: {uses}");
                let lines = text
                    .lines()
                    .filter(|l| l.contains(&format!("uses: {uses}")));
                assert!(
                    lines.into_iter().all(|l| l.contains(" # v")),
                    "{file}/{id} gives no version beside {uses}"
                );
            }
        }
    }
    assert!(seen >= 6, "only {seen} actions read");
}

#[test]
fn self_hosted_jobs_pin_every_action() {
    check_pins(&workflows());
}

/// The SHA the measurement workflow pins checkout to, replaced by `to` wherever it stands first.
#[cfg(feature = "controls")]
fn repinned(to: &str) -> Vec<(String, String)> {
    measured(
        "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
        to,
    )
}

negative_control!(
    self_hosted_jobs_pin_every_action,
    "checkout at a tag",
    expected = "uses an unpinned action: actions/checkout@v7",
    check_pins(&repinned("actions/checkout@v7"))
);

mod branch {
    use super::*;
    negative_control!(
        self_hosted_jobs_pin_every_action,
        "checkout at a branch",
        expected = "uses an unpinned action: actions/checkout@main",
        check_pins(&repinned("actions/checkout@main"))
    );
}

mod short {
    use super::*;
    negative_control!(
        self_hosted_jobs_pin_every_action,
        "checkout at a short SHA, one character short of a full one",
        expected = "uses an unpinned action",
        check_pins(&repinned(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b # v7.0.1"
        ))
    );
}

mod not_hex {
    use super::*;
    negative_control!(
        self_hosted_jobs_pin_every_action,
        "a 40-character branch name, in place of a SHA",
        expected = "uses an unpinned action",
        check_pins(&repinned(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90bg # v7.0.1"
        ))
    );
}

mod local {
    use super::*;
    negative_control!(
        self_hosted_jobs_pin_every_action,
        "a local action",
        expected = "uses an unpinned action: ./.github/actions/x",
        check_pins(&repinned("./.github/actions/x"))
    );
}

mod no_version {
    use super::*;
    negative_control!(
        self_hosted_jobs_pin_every_action,
        "a pin with no version in a comment",
        expected = "gives no version beside",
        check_pins(&repinned(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"
        ))
    );
}

// ---- Never required, never gathered (R-649) ------------------------------------------------------------------------

/// Whether `text` names any of `names` as a whole word, `-` and `_` being of a word.
fn names_any(text: &str, names: &[String]) -> bool {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
        .any(|word| names.iter().any(|n| n == word))
}

/// Every Rust source file under `dir`, by path relative to the repository.
fn sources(dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root().join(dir)).expect("a source folder") {
        let path = entry.expect("an entry").path();
        let rel = format!(
            "{dir}/{}",
            path.file_name().expect("a name").to_string_lossy()
        );
        if path.is_dir() {
            out.extend(sources(&rel));
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push((rel.clone(), read(&rel)));
        }
    }
    out
}

/// What reads CI's results, or runs anything off them, beside the workflows: the scripts and `xtask`'s sources.
fn consumers() -> Vec<(String, String)> {
    let mut out = sources("xtask/src");
    for dir in [".github/scripts", "scripts"] {
        for entry in std::fs::read_dir(root().join(dir)).expect("a scripts folder") {
            let path = entry.expect("an entry").path();
            if path.is_file() {
                let rel = format!(
                    "{dir}/{}",
                    path.file_name().expect("a name").to_string_lossy()
                );
                if rel != RECORD {
                    out.push((
                        rel,
                        String::from_utf8_lossy(&std::fs::read(&path).expect("a script"))
                            .into_owned(),
                    ));
                }
            }
        }
    }
    out
}

fn check_not_gathered(files: &[(String, String)], others: &[(String, String)], tier: &str) {
    let full_only = tier
        .lines()
        .find_map(|l| l.strip_prefix("FULL_ONLY=\""))
        .and_then(|l| l.strip_suffix('"'))
        .expect("no FULL_ONLY in the tier script");
    for name in full_only.split(' ') {
        assert!(
            REQUIRED.contains(&name),
            "the required check {name} isn't in this test's list"
        );
    }
    for (file, _, doc, ids) in self_hosted(files) {
        let mut names = vec![
            file.to_owned(),
            doc["name"].as_str().unwrap_or(file).to_owned(),
        ];
        for id in &ids {
            let job = &doc["jobs"][id.as_str()];
            let shown = job["name"].as_str().unwrap_or(id);
            for name in [id.as_str(), shown] {
                assert!(
                    !REQUIRED.contains(&name),
                    "{file}/{id} reports as the required check {name}"
                );
            }
            names.push(id.clone());
            names.push(shown.to_owned());
        }
        for (other, job) in entries(&doc["jobs"]) {
            for need in list(&job["needs"]) {
                assert!(
                    !ids.iter().any(|id| id == need),
                    "{file}/{other} gathers the self-hosted job {need}"
                );
            }
        }
        for (other, text) in files.iter().filter(|(n, _)| n != file) {
            let read = parse(other, text);
            for s in all_strings(&read) {
                assert!(
                    !names_any(s, &names),
                    "{other} reads the self-hosted workflow {file}: `{s}`"
                );
            }
        }
        for (other, text) in others {
            assert!(
                !names_any(text, &names),
                "{other} reads the self-hosted workflow {file}"
            );
        }
    }
}

#[test]
fn self_hosted_jobs_are_never_required_or_gathered() {
    check_not_gathered(&workflows(), &consumers(), &read(TIER_SCRIPT));
}

negative_control!(
    self_hosted_jobs_are_never_required_or_gathered,
    "a job in the workflow that needs a self-hosted one",
    expected = "win-gpu.yml/gather gathers the self-hosted job win-tests",
    check_not_gathered(
        &measured(
            "jobs:\n",
            "jobs:\n  gather:\n    runs-on: ubuntu-24.04\n    needs: [win-tests]\n    steps: []\n"
        ),
        &consumers(),
        &read(TIER_SCRIPT)
    )
);

mod retried {
    use super::*;
    negative_control!(
        self_hosted_jobs_are_never_required_or_gathered,
        "retry.yml following the measurement workflow",
        expected = "retry.yml reads the self-hosted workflow win-gpu.yml",
        check_not_gathered(
            &edited(
                "retry.yml",
                "[ci, mutants, gate]",
                "[ci, mutants, gate, win-gpu]"
            ),
            &consumers(),
            &read(TIER_SCRIPT)
        )
    );
}

mod required_name {
    use super::*;
    negative_control!(
        self_hosted_jobs_are_never_required_or_gathered,
        "a self-hosted job named for a required check",
        expected = "win-gpu.yml/win-tests reports as the required check gpu-lavapipe",
        check_not_gathered(
            &measured("  win-tests:\n", "  win-tests:\n    name: gpu-lavapipe\n"),
            &consumers(),
            &read(TIER_SCRIPT)
        )
    );
}

mod required_id {
    use super::*;
    negative_control!(
        self_hosted_jobs_are_never_required_or_gathered,
        "a self-hosted job with a required check's id",
        expected = "reports as the required check mutants-check",
        check_not_gathered(
            &[(
                WORKFLOW.to_owned(),
                measured("  win-tests:\n", "  mutants-check:\n")
                    .into_iter()
                    .find(|(n, _)| n == WORKFLOW)
                    .expect("the workflow")
                    .1
                    .replace(
                        "jobs:\n",
                        "jobs:\n  win-tests:\n    runs-on: [self-hosted]\n"
                    )
            )],
            &consumers(),
            &read(TIER_SCRIPT)
        )
    );
}

mod by_a_script {
    use super::*;
    negative_control!(
        self_hosted_jobs_are_never_required_or_gathered,
        "the tier script gathering the job",
        expected = ".github/scripts/ci_tier.sh reads the self-hosted workflow win-gpu.yml",
        check_not_gathered(
            &workflows(),
            &[(
                TIER_SCRIPT.to_owned(),
                format!(
                    "{}\nFULL_ONLY=\"$FULL_ONLY win-vulkan\"\n",
                    read(TIER_SCRIPT)
                )
            )],
            &read(TIER_SCRIPT)
        )
    );
}

mod by_a_gate {
    use super::*;
    negative_control!(
        self_hosted_jobs_are_never_required_or_gathered,
        "xtask's sources reading the workflow's runs",
        expected = "xtask/src/x.rs reads the self-hosted workflow win-gpu.yml",
        check_not_gathered(
            &workflows(),
            &[(
                "xtask/src/x.rs".to_owned(),
                "const RUNS: &str = \"actions/workflows/win-gpu.yml/runs\";".to_owned()
            )],
            &read(TIER_SCRIPT)
        )
    );
}

mod new_required {
    use super::*;
    negative_control!(
        self_hosted_jobs_are_never_required_or_gathered,
        "a required check this test doesn't know",
        expected = "the required check gpu-new isn't in this test's list",
        check_not_gathered(
            &workflows(),
            &consumers(),
            &read(TIER_SCRIPT).replacen("gpu-lavapipe\"", "gpu-lavapipe gpu-new\"", 1)
        )
    );
}

// ---- No credential ---------------------------------------------------------------------------------------------------

fn check_no_credential(files: &[(String, String)]) {
    for (file, _, doc, ids) in self_hosted(files) {
        for s in all_strings(&doc) {
            let lower = s.to_lowercase();
            for word in ["secrets", "github.token", "github_token"] {
                assert!(!lower.contains(word), "{file} names {word}: `{s}`");
            }
        }
        for id in ids {
            let job = &doc["jobs"][id.as_str()];
            let permissions = effective(&doc, job, "permissions");
            let granted = entries(permissions);
            assert!(
                granted.len() == 1
                    && granted[0].0 == "contents"
                    && granted[0].1.as_str() == Some("read"),
                "{file}/{id} isn't `contents: read` alone: {permissions:?}"
            );
            for step in steps(job) {
                if step["uses"]
                    .as_str()
                    .is_some_and(|u| u.starts_with("actions/checkout@"))
                {
                    assert_eq!(
                        step["with"]["persist-credentials"].as_bool(),
                        Some(false),
                        "{file}/{id} leaves the token in the checkout"
                    );
                }
            }
        }
    }
}

#[test]
fn self_hosted_jobs_hold_no_credential() {
    check_no_credential(&workflows());
}

negative_control!(
    self_hosted_jobs_hold_no_credential,
    "a secret in a step's env",
    expected = "win-gpu.yml names secrets",
    check_no_credential(&measured(
        "        id: kernel\n",
        "        id: kernel\n        env:\n          K: ${{ secrets.KEY }}\n"
    ))
);

mod inherited {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "the whole secrets context, which no scan for `secrets.` sees",
        expected = "win-gpu.yml names secrets",
        check_no_credential(&measured(
            "        id: kernel\n",
            "        id: kernel\n        env:\n          K: ${{ toJSON(secrets) }}\n"
        ))
    );
}

mod the_token {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "the run's token, which is no `secrets.`",
        expected = "win-gpu.yml names github.token",
        check_no_credential(&measured(
            "        id: kernel\n",
            "        id: kernel\n        env:\n          K: ${{ github.token }}\n"
        ))
    );
}

mod write {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "write access to the contents",
        expected = "win-gpu.yml/win-vulkan isn't `contents: read` alone",
        check_no_credential(&measured(
            "permissions:\n  contents: read\n",
            "permissions:\n  contents: write\n"
        ))
    );
}

mod more_access {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "a second permission",
        expected = "win-gpu.yml/win-vulkan isn't `contents: read` alone",
        check_no_credential(&measured(
            "permissions:\n  contents: read\n",
            "permissions:\n  contents: read\n  actions: read\n"
        ))
    );
}

mod job_widens {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "a job widening the workflow's permissions",
        expected = "win-gpu.yml/win-tests isn't `contents: read` alone",
        check_no_credential(&measured(
            "  win-tests:\n",
            "  win-tests:\n    permissions: write-all\n"
        ))
    );
}

mod persisted {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "a checkout that keeps the token",
        expected = "win-gpu.yml/win-vulkan leaves the token in the checkout",
        check_no_credential(&measured(
            "          persist-credentials: false\n",
            "          persist-credentials: true\n"
        ))
    );
}

mod defaulted {
    use super::*;
    negative_control!(
        self_hosted_jobs_hold_no_credential,
        "a checkout left at its default, which keeps the token",
        expected = "win-gpu.yml/win-vulkan leaves the token in the checkout",
        check_no_credential(&measured("          persist-credentials: false\n", ""))
    );
}

// ---- The repository and actor guard ------------------------------------------------------------------------------------

/// Whether `rest`, the condition after the guard, has an `||` outside parentheses and strings, which would run the job
/// whatever the guard said.
fn has_bare_or(rest: &str) -> bool {
    let (mut depth, mut quoted) = (0, false);
    let bytes = rest.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'\'' => quoted = !quoted,
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth -= 1,
            b'|' if !quoted && depth == 0 && bytes.get(i + 1) == Some(&b'|') => return true,
            _ => {}
        }
    }
    false
}

/// Whether the job's whole condition holds only if the guard does.
fn guarded(condition: &str) -> bool {
    match condition.strip_prefix(GUARD) {
        Some("") => true,
        Some(rest) => rest.starts_with(" && ") && !has_bare_or(rest),
        None => false,
    }
}

fn check_guard(files: &[(String, String)]) {
    for (file, _, doc, ids) in self_hosted(files) {
        for id in ids {
            let condition = doc["jobs"][id.as_str()]["if"].as_str().unwrap_or("");
            assert!(
                guarded(condition),
                "{file}/{id} lacks the repository and actor guard: `{condition}`"
            );
        }
    }
}

#[test]
fn self_hosted_jobs_keep_the_repo_and_actor_guard() {
    check_guard(&workflows());
}

negative_control!(
    self_hosted_jobs_keep_the_repo_and_actor_guard,
    "the actor left out",
    expected = "win-gpu.yml/win-vulkan lacks the repository and actor guard",
    check_guard(&measured(" && github.actor == 'Ma1achy'", ""))
);

mod repo {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_repo_and_actor_guard,
        "the repository left out",
        expected = "win-gpu.yml/win-vulkan lacks the repository and actor guard",
        check_guard(&measured(
            "github.repository == 'Ma1achy/Principia' && ",
            ""
        ))
    );
}

mod no_condition {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_repo_and_actor_guard,
        "no condition at all",
        expected = "lacks the repository and actor guard",
        check_guard(&measured(
            "    if: github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name != 'push' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-tests')\n",
            ""
        ))
    );
}

mod bypassed {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_repo_and_actor_guard,
        "an `||` after the guard, which runs the job whatever it says",
        expected = "lacks the repository and actor guard",
        check_guard(&measured(
            "github.actor == 'Ma1achy' && (inputs.only",
            "github.actor == 'Ma1achy' || (inputs.only"
        ))
    );
}

mod bypassed_later {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_repo_and_actor_guard,
        "an `||` after a later term",
        expected = "lacks the repository and actor guard",
        check_guard(&measured(
            "github.actor == 'Ma1achy' && (inputs.only",
            "github.actor == 'Ma1achy' && true || (inputs.only"
        ))
    );
}

mod another_owner {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_repo_and_actor_guard,
        "another actor",
        expected = "lacks the repository and actor guard",
        check_guard(&measured(
            "github.actor == 'Ma1achy'",
            "github.actor != 'Ma1achy'"
        ))
    );
}

// ---- What the runner needs of a job (the hand-over on #390) ------------------------------------------------------------

/// Whether the step's script ends by starting `script` fail closed and exiting with its status.
fn ends_in_a_call(step: &Yaml, script: &str) -> bool {
    let lines: Vec<&str> = step["run"].as_str().unwrap_or("").lines().collect();
    lines.len() >= 2
        && lines[lines.len() - 2] == format!("{FAIL_CLOSED}{script}")
        && lines[lines.len() - 1] == STATUS
}

/// Whether the step fetches the .git guard at the job's own commit, refuses if it can't, and starts what it fetched.
fn fetches_the_guard(step: &Yaml) -> bool {
    let run = step["run"].as_str().unwrap_or("");
    ends_in_a_call(step, "$guard")
        && run.lines().any(|l| {
            l.contains("Invoke-WebRequest") && l.contains(GIT_GUARD_URL) && l.ends_with("exit 1 }")
        })
}

fn check_shape(files: &[(String, String)]) {
    for (file, _, doc, ids) in self_hosted(files) {
        assert!(
            doc["concurrency"].is_badvalue(),
            "{file} has a concurrency group across its jobs, so a push would displace a nightly"
        );
        for (id, job) in entries(&doc["jobs"]) {
            assert!(
                is_self_hosted(job),
                "{file}/{id} is a hosted job among self-hosted ones, out of reach of the exceptions they hold"
            );
        }
        for s in all_strings(&doc) {
            assert!(!s.contains("cargo clean"), "{file} runs `cargo clean`");
            assert!(!RUNNER_ENV.contains(&s), "{file} sets {s}");
        }
        assert_eq!(
            doc["env"]["PRIN_GPU_BACKEND"].as_str(),
            Some("vulkan"),
            "{file} doesn't select Vulkan"
        );
        for id in ids {
            let job = &doc["jobs"][id.as_str()];
            assert_eq!(list(&job["runs-on"]), LABELS, "{file}/{id}'s runner");
            assert_eq!(
                job["concurrency"]["group"].as_str(),
                Some(id.as_str()),
                "{file}/{id} has no concurrency group of its own"
            );
            assert_eq!(
                job["concurrency"]["cancel-in-progress"].as_bool(),
                Some(false),
                "{file}/{id} cancels a running measurement"
            );
            assert!(
                job["timeout-minutes"].as_i64().is_some(),
                "{file}/{id} has no timeout"
            );
            assert_eq!(
                effective(&doc, job, "defaults")["run"]["shell"].as_str(),
                Some(SHELL),
                "{file}/{id}'s shell is one the runner refuses"
            );
            assert_eq!(
                NIGHTLY_ONLY.contains(&id.as_str()),
                job["if"].as_str().unwrap_or("").contains(NOT_ON_PUSH),
                "{file}/{id} runs on the wrong events"
            );
            let steps = steps(job);
            let runs = |s: &Yaml, what: &str| s["run"].as_str().is_some_and(|r| r.contains(what));
            assert!(
                steps.first().is_some_and(|s| ends_in_a_call(s, SELF_CHECK)),
                "{file}/{id} doesn't run the self-check first"
            );
            assert!(
                steps.get(1).is_some_and(fetches_the_guard),
                "{file}/{id} doesn't run the fetched .git guard second"
            );
            let checkout = steps
                .iter()
                .position(|s| {
                    s["uses"]
                        .as_str()
                        .is_some_and(|u| u.starts_with("actions/checkout@"))
                })
                .unwrap_or_else(|| panic!("{file}/{id} has no checkout"));
            assert!(checkout > 1, "{file}/{id} checks out before the .git guard");
            let with = &steps[checkout]["with"];
            assert_eq!(
                with["fetch-depth"].as_i64(),
                Some(1),
                "{file}/{id} doesn't fetch at depth 1"
            );
            assert_eq!(
                with["clean"].as_bool(),
                Some(true),
                "{file}/{id} doesn't clean the kept checkout"
            );
            assert!(
                steps
                    .iter()
                    .any(|s| runs(s, "GITHUB_PATH") && runs(s, CARGO_BIN)),
                "{file}/{id} doesn't put the toolchain on PATH"
            );
            for step in steps {
                assert!(
                    step["shell"].is_badvalue(),
                    "{file}/{id} overrides the shell in a step"
                );
                if let Some(run) = step["run"].as_str() {
                    assert!(
                        run.trim_end()
                            .lines()
                            .last()
                            .is_some_and(|l| l.starts_with("exit ")),
                        "{file}/{id} has a step with no explicit exit: {run}"
                    );
                    assert!(
                        !run.contains("SilentlyContinue"),
                        "{file}/{id} has a step that reads on past a failure: {run}"
                    );
                    assert!(
                        !run.contains("git ls-files")
                            || run.contains("$status -ne 0 -or -not $files.Count"),
                        "{file}/{id} has a git read that can pass having read nothing: {run}"
                    );
                    let lines: Vec<&str> = run.lines().collect();
                    for (at, line) in lines.iter().enumerate() {
                        if line.contains(".github\\scripts\\") {
                            assert!(
                                line.starts_with(FAIL_CLOSED) && lines.get(at + 1) == Some(&STATUS),
                                "{file}/{id} doesn't start a script fail closed: {line}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn self_hosted_jobs_have_the_shape_the_runner_needs() {
    check_shape(&workflows());
}

negative_control!(
    self_hosted_jobs_have_the_shape_the_runner_needs,
    "a job with a label fewer",
    expected = "win-gpu.yml/win-vulkan's runner",
    check_shape(&measured(
        "runs-on: [self-hosted, windows, nvidia]",
        "runs-on: [self-hosted, windows]"
    ))
);

mod plain_shell {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "plain `shell: powershell`, which the runner refuses",
        expected = "win-gpu.yml/win-vulkan's shell is one the runner refuses",
        check_shape(&measured(SHELL, "powershell"))
    );
}

mod step_shell {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a step with a shell of its own",
        expected = "win-gpu.yml/win-vulkan overrides the shell in a step",
        check_shape(&measured(
            "        id: kernel\n",
            "        id: kernel\n        shell: powershell\n"
        ))
    );
}

mod no_self_check {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a job whose first step isn't the self-check",
        expected = "win-gpu.yml/win-vulkan doesn't run the self-check first",
        check_shape(&measured(
            "      - name: Self-check, first step\n",
            "      - name: Before it\n        run: exit 0\n      - name: Self-check, first step\n"
        ))
    );
}

mod no_path {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a job without the toolchain's folder on PATH",
        expected = "win-gpu.yml/win-vulkan doesn't put the toolchain on PATH",
        check_shape(&measured(
            "          Add-Content -LiteralPath $env:GITHUB_PATH -Value 'E:\\runner-tools\\cargo\\bin'\n",
            ""
        ))
    );
}

mod no_exit {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a step that ends without its exit, so it passes whatever failed",
        expected = "win-gpu.yml/win-vulkan has a step with no explicit exit",
        check_shape(&measured(
            "win_measure.ps1 run build-kernel cargo xtask build-kernel\n          exit $LASTEXITCODE\n",
            "win_measure.ps1 run build-kernel cargo xtask build-kernel\n"
        ))
    );
}

mod backend {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "the Vulkan selection gone, so the platform's default backend runs",
        expected = "win-gpu.yml doesn't select Vulkan",
        check_shape(&measured("  PRIN_GPU_BACKEND: vulkan\n", ""))
    );
}

mod no_group {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "no concurrency group",
        expected = "win-gpu.yml/win-vulkan has no concurrency group of its own",
        check_shape(&measured(
            "    concurrency:\n      group: win-vulkan\n      cancel-in-progress: false\n",
            ""
        ))
    );
}

mod group_per_ref {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a group per branch, so that a queue per branch can pile up",
        expected = "win-gpu.yml/win-vulkan has no concurrency group of its own",
        check_shape(&measured(
            "      group: win-vulkan\n",
            "      group: win-vulkan-${{ github.ref }}\n"
        ))
    );
}

mod shared_group {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "two jobs in one group, where a run's jobs displace each other",
        expected = "win-gpu.yml/win-tests has no concurrency group of its own",
        check_shape(&measured(
            "      group: win-tests\n",
            "      group: win-vulkan\n"
        ))
    );
}

mod workflow_group {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "one group across the file, where a push displaces a nightly",
        expected = "win-gpu.yml has a concurrency group across its jobs",
        check_shape(&measured(
            "permissions:\n",
            "concurrency:\n  group: win-gpu\n  cancel-in-progress: false\n\npermissions:\n"
        ))
    );
}

mod cancels {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a new push cancelling a running measurement",
        expected = "win-gpu.yml/win-vulkan cancels a running measurement",
        check_shape(&measured(
            "      cancel-in-progress: false\n",
            "      cancel-in-progress: true\n"
        ))
    );
}

mod hosted_job {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a hosted job in the file, which the rust-gpu cache check doesn't read",
        expected = "win-gpu.yml/hosted is a hosted job among self-hosted ones",
        check_shape(&measured(
            "jobs:\n",
            "jobs:\n  hosted:\n    runs-on: ubuntu-24.04\n    steps: []\n"
        ))
    );
}

/// A script started in the step's own process: if it can't parse, the step goes on, to `exit $null`, which passes.
#[cfg(feature = "controls")]
const IN_PROCESS: &str =
    "& .github\\scripts\\win_measure.ps1 start\n          exit $LASTEXITCODE\n";

mod in_process {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a script started with `&`",
        expected = "win-gpu.yml/win-vulkan doesn't start a script fail closed",
        check_shape(&measured(
            "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_measure.ps1 start\n          exit $LASTEXITCODE\n",
            IN_PROCESS
        ))
    );
}

mod status_dropped {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a script whose status the step replaces",
        expected = "win-gpu.yml/win-vulkan doesn't start a script fail closed",
        check_shape(&measured(
            "win_measure.ps1 start\n          exit $LASTEXITCODE\n",
            "win_measure.ps1 start\n          exit 0\n"
        ))
    );
}

mod self_check_in_process {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "the self-check started with `&`",
        expected = "win-gpu.yml/win-vulkan doesn't run the self-check first",
        check_shape(&measured(
            "          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File E:\\runner-tools",
            "          & E:\\runner-tools"
        ))
    );
}

mod guard_in_process {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "the .git guard started with `&`",
        expected = "win-gpu.yml/win-vulkan doesn't run the fetched .git guard second",
        check_shape(&measured(
            "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $guard",
            "& $guard"
        ))
    );
}

mod guard_gone {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a job without the .git guard",
        expected = "win-gpu.yml/win-vulkan doesn't run the fetched .git guard second",
        check_shape(&measured(
            "      - name: Guard the kept checkout\n",
            "      - name: Another step\n        run: exit 0\n      - name: Guard the kept checkout\n"
        ))
    );
}

mod guard_from_the_kept_folder {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "the .git guard fetched from anywhere but the job's own commit",
        expected = "win-gpu.yml/win-vulkan doesn't run the fetched .git guard second",
        check_shape(&measured("/$env:GITHUB_SHA/", "/main/"))
    );
}

mod guard_fetch_may_fail {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a failed fetch that carries on",
        expected = "win-gpu.yml/win-vulkan doesn't run the fetched .git guard second",
        check_shape(&measured(
            "\"could not fetch the guard: $_\"; exit 1 }",
            "}"
        ))
    );
}

mod guard_after_checkout {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a checkout before the .git guard",
        expected = "win-gpu.yml/win-vulkan checks out before the .git guard",
        check_shape(&measured(
            "      - name: Self-check, first step\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File E:\\runner-tools\\checks\\self-check.ps1\n          exit $LASTEXITCODE\n      - name: Guard the kept checkout\n",
            "      - name: Self-check, first step\n        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File E:\\runner-tools\\checks\\self-check.ps1\n          exit $LASTEXITCODE\n      - name: Guard the kept checkout\n"
        ))
    );
}

mod no_depth {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a checkout that leaves its depth implicit",
        expected = "win-gpu.yml/win-vulkan doesn't fetch at depth 1",
        check_shape(&measured("          fetch-depth: 1\n", ""))
    );
}

mod full_clone {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a full clone, 11 minutes on this PC",
        expected = "win-gpu.yml/win-vulkan doesn't fetch at depth 1",
        check_shape(&measured(
            "          fetch-depth: 1\n",
            "          fetch-depth: 0\n"
        ))
    );
}

mod unclean {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a kept checkout left uncleaned",
        expected = "win-gpu.yml/win-vulkan doesn't clean the kept checkout",
        check_shape(&measured(
            "          clean: true\n",
            "          clean: false\n"
        ))
    );
}

mod read_failure_passed_over {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a config read that can't fail",
        expected = "win-gpu.yml/win-self-check has a step that reads on past a failure",
        check_shape(&measured(
            "-ErrorAction Stop",
            "-ErrorAction SilentlyContinue"
        ))
    );
}

mod git_failure_passed_over {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "git's failure left unread",
        expected = "win-gpu.yml/win-self-check has a git read that can pass having read nothing",
        check_shape(&measured("$status -ne 0 -or -not $files.Count", "$false"))
    );
}

mod no_timeout {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a job with GitHub's default six hours",
        expected = "win-gpu.yml/win-vulkan has no timeout",
        check_shape(&measured("    timeout-minutes: 330\n", ""))
    );
}

mod clean {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "`cargo clean` of the shared target folder",
        expected = "win-gpu.yml runs `cargo clean`",
        check_shape(&measured(
            "          exit $LASTEXITCODE\n",
            "          cargo clean\n          exit $LASTEXITCODE\n"
        ))
    );
}

mod target_dir {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a job moving the target folder",
        expected = "win-gpu.yml sets CARGO_TARGET_DIR",
        check_shape(&measured(
            "  CARGO_TERM_COLOR: never\n",
            "  CARGO_TERM_COLOR: never\n  CARGO_TARGET_DIR: C:\\t\n"
        ))
    );
}

mod temp {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "a job moving TEMP",
        expected = "win-gpu.yml sets TEMP",
        check_shape(&measured(
            "  CARGO_TERM_COLOR: never\n",
            "  CARGO_TERM_COLOR: never\n  TEMP: C:\\t\n"
        ))
    );
}

mod tests_on_push {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "the Windows tests on every push",
        expected = "win-gpu.yml/win-tests runs on the wrong events",
        check_shape(&measured(
            " && github.event_name != 'push' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-tests')",
            ""
        ))
    );
}

mod gpu_nightly_only {
    use super::*;
    negative_control!(
        self_hosted_jobs_have_the_shape_the_runner_needs,
        "the Vulkan suites left off `main`",
        expected = "win-gpu.yml/win-vulkan runs on the wrong events",
        check_shape(&measured(
            "github.actor == 'Ma1achy' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-vulkan')",
            "github.actor == 'Ma1achy' && github.event_name != 'push'"
        ))
    );
}

// ---- The record: every result labelled, its full output kept (R-711 item 4) --------------------------------------------

fn check_record(files: &[(String, String)], script: &str) {
    assert!(script.contains(LABEL), "{RECORD} doesn't carry the label");
    assert!(
        script.contains("GITHUB_SHA")
            && script.contains("driver")
            && script.contains("GITHUB_STEP_SUMMARY"),
        "{RECORD} doesn't record the commit and driver in the summary"
    );
    for (file, _, doc, ids) in self_hosted(files) {
        for id in ids {
            let steps = steps(&doc["jobs"][id.as_str()]);
            let always = |s: &Yaml| s["if"].as_str().is_some_and(|c| c.contains("always()"));
            let finish = steps
                .iter()
                .position(|s| {
                    always(s)
                        && s["run"]
                            .as_str()
                            .is_some_and(|r| r.contains("win_measure.ps1 finish"))
                })
                .unwrap_or_else(|| panic!("{file}/{id} doesn't label its result, pass or fail"));
            let keep = steps
                .iter()
                .position(|s| {
                    always(s)
                        && s["uses"]
                            .as_str()
                            .is_some_and(|u| u.starts_with("actions/upload-artifact@"))
                })
                .unwrap_or_else(|| panic!("{file}/{id} doesn't keep its output, pass or fail"));
            assert!(
                finish < keep,
                "{file}/{id} keeps its output before the record is whole"
            );
            assert!(
                steps[keep]["with"]["path"]
                    .as_str()
                    .is_some_and(|p| p.ends_with("win-measure")),
                "{file}/{id} keeps something other than its record"
            );
            let name = steps[keep]["with"]["name"].as_str().unwrap_or("");
            assert!(
                name.contains("github.sha") && name.contains("github.run_attempt"),
                "{file}/{id}'s artifact doesn't name its commit and attempt: a re-run would collide"
            );
            for step in steps {
                if let Some(run) = step["run"].as_str() {
                    assert!(
                        !run.split_whitespace().any(|w| w == "cargo")
                            || run.contains("win_measure.ps1 run "),
                        "{file}/{id} runs cargo outside the record: {run}"
                    );
                }
            }
        }
    }
}

#[test]
fn self_hosted_results_are_labelled_and_kept() {
    check_record(&workflows(), &read(RECORD));
}

negative_control!(
    self_hosted_results_are_labelled_and_kept,
    "the script without the label",
    expected = "doesn't carry the label",
    check_record(&workflows(), &read(RECORD).replace(LABEL, "a result"))
);

mod no_driver {
    use super::*;
    negative_control!(
        self_hosted_results_are_labelled_and_kept,
        "the script not recording the driver",
        expected = "doesn't record the commit and driver",
        check_record(&workflows(), &read(RECORD).replace("driver", "gpu"))
    );
}

mod no_finish {
    use super::*;
    negative_control!(
        self_hosted_results_are_labelled_and_kept,
        "a job that labels only a green result",
        expected = "win-gpu.yml/win-vulkan doesn't label its result, pass or fail",
        check_record(
            &measured(
                "      - name: Label the result in the summary\n        if: ${{ always() && steps.checkout.outcome == 'success' }}\n",
                "      - name: Label the result in the summary\n"
            ),
            &read(RECORD)
        )
    );
}

mod no_artifact {
    use super::*;
    negative_control!(
        self_hosted_results_are_labelled_and_kept,
        "a job that keeps only a green run's output",
        expected = "win-gpu.yml/win-vulkan doesn't keep its output, pass or fail",
        check_record(
            &measured(
                "      - name: Keep the full output\n        if: ${{ always() && steps.checkout.outcome == 'success' }}\n",
                "      - name: Keep the full output\n"
            ),
            &read(RECORD)
        )
    );
}

mod wrong_path {
    use super::*;
    negative_control!(
        self_hosted_results_are_labelled_and_kept,
        "an artifact of something else",
        expected = "win-gpu.yml/win-vulkan keeps something other than its record",
        check_record(
            &measured(
                "          path: ${{ runner.temp }}/win-measure\n",
                "          path: target\n"
            ),
            &read(RECORD)
        )
    );
}

mod same_name {
    use super::*;
    negative_control!(
        self_hosted_results_are_labelled_and_kept,
        "an artifact name a re-run repeats",
        expected = "win-gpu.yml/win-vulkan's artifact doesn't name its commit and attempt",
        check_record(&measured("-${{ github.run_attempt }}", ""), &read(RECORD))
    );
}

mod bare_cargo {
    use super::*;
    negative_control!(
        self_hosted_results_are_labelled_and_kept,
        "a suite outside the record",
        expected = "win-gpu.yml/win-vulkan runs cargo outside the record",
        check_record(
            &measured(
                "          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_measure.ps1 run build-kernel cargo xtask build-kernel\n",
                "          cargo xtask build-kernel\n"
            ),
            &read(RECORD)
        )
    );
}

// ---- The .git guard (the owner, 5 Nov 2026) --------------------------------------------------------------------------

/// A key for each item of the owner's list, in a case other than the list's: git matches a section and name in any case.
const PLANTED: [&str; 40] = [
    "CORE.hookspath",
    "Core.FSMonitor",
    "core.sshcommand",
    "CORE.EDITOR",
    "core.Pager",
    "filter.LFS.Clean",
    "FILTER.a.smudge",
    "filter.a b.PROCESS",
    "Diff.x.TextConv",
    "Credential.Helper",
    "INCLUDE.path",
    "includeif.gitdir:/x/.path",
    "URL.https://x/.INSTEADOF",
    "HTTP.Proxy",
    "GPG.program",
    "gpg.SSH.Program",
    "core.GITPROXY",
    "Core.AskPass",
    "core.ATTRIBUTESFILE",
    "CORE.Worktree",
    "core.ExcludesFile",
    "DIFF.External",
    "diff.x.COMMAND",
    "Merge.x.Driver",
    "ALIAS.st",
    "Pager.log",
    "Sequence.Editor",
    "gpg.X509.Program",
    "Remote.origin.UploadPack",
    "remote.origin.RECEIVEPACK",
    "REMOTE.origin.proxy",
    "remote.origin.VCS",
    "Credential.https://x/.Helper",
    "HTTP.https://x/.Proxy",
    "url.https://x/.PUSHINSTEADOF",
    "FSCK.skipList",
    "fsck.MissingEmail",
    "Hook.x.COMMAND",
    "hook.x.Event",
    "Extensions.RefStorage",
];
/// The script's list `name`.
fn quoted_list<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    let list = script
        .split_once(format!("${name} = @(\n").as_str())
        .and_then(|(_, rest)| rest.split_once("\n)"))
        .expect("git_guard: no such list")
        .0;
    list.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.trim_matches('\''))
        .collect()
}

/// The script's `$keys`: what a refusal names.
fn listed_keys(script: &str) -> Vec<&str> {
    quoted_list(script, "keys")
}

/// The script's `$allowed`, its `key=value` pairs: the list of what's harmless, as all else refuses.
fn allowed_pairs(script: &str) -> Vec<&str> {
    quoted_list(script, "allowed")
}

/// The keys of `$allowed`.
fn allowed_keys(script: &str) -> Vec<&str> {
    allowed_pairs(script)
        .into_iter()
        .map(|pair| pair.split_once('=').expect("a key=value pair").0)
        .collect()
}

/// PowerShell's `-like` for these patterns: `*` is any text, and case doesn't count.
fn like(key: &str, pattern: &str) -> bool {
    let (key, pattern) = (key.to_lowercase(), pattern.to_lowercase());
    let mut parts = pattern.split('*');
    let first = parts.next().expect("a part");
    let Some(mut rest) = key.strip_prefix(first) else {
        return false;
    };
    let mut last = None;
    for part in parts {
        if let Some(prior) = last.replace(part) {
            let Some((_, after)) = rest.split_once(prior) else {
                return false;
            };
            rest = after;
        }
    }
    last.map_or(rest.is_empty(), |part| rest.ends_with(part))
}

fn check_keys(script: &str) {
    let listed = listed_keys(script);
    for key in PLANTED {
        assert!(
            listed.iter().any(|p| like(key, p)),
            "git_guard: {key} isn't refused"
        );
    }
    for key in allowed_keys(script) {
        assert!(
            !listed.iter().any(|p| like(key, p)),
            "git_guard: {key} is refused"
        );
        assert!(
            !key.contains('*') && key == key.to_lowercase(),
            "git_guard: {key} is no exact key as git prints it"
        );
    }
}

#[test]
fn git_guard_refuses_each_listed_key() {
    check_keys(&read(GIT_GUARD));
}

mod key_hooks_path {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.hooksPath off the list",
        expected = "git_guard: CORE.hookspath isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.hooksPath'\n", ""))
    );
}

mod key_fsmonitor {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.fsmonitor off the list",
        expected = "git_guard: Core.FSMonitor isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.fsmonitor'\n", ""))
    );
}

mod key_ssh_command {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.sshCommand off the list",
        expected = "git_guard: core.sshcommand isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.sshCommand'\n", ""))
    );
}

mod key_editor {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.editor off the list",
        expected = "git_guard: CORE.EDITOR isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.editor'\n", ""))
    );
}

mod key_pager {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.pager off the list",
        expected = "git_guard: core.Pager isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.pager'\n", ""))
    );
}

mod key_filter_clean {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "filter.*.clean off the list",
        expected = "git_guard: filter.LFS.Clean isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'filter.*.clean'\n", ""))
    );
}

mod key_filter_smudge {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "filter.*.smudge off the list",
        expected = "git_guard: FILTER.a.smudge isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'filter.*.smudge'\n", ""))
    );
}

mod key_filter_process {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "filter.*.process off the list",
        expected = "git_guard: filter.a b.PROCESS isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'filter.*.process'\n", ""))
    );
}

mod key_textconv {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "diff.*.textconv off the list",
        expected = "git_guard: Diff.x.TextConv isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'diff.*.textconv'\n", ""))
    );
}

mod key_credential {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "credential.helper off the list",
        expected = "git_guard: Credential.Helper isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'credential.helper'\n", ""))
    );
}

mod key_include {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "include.path off the list",
        expected = "git_guard: INCLUDE.path isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'include.path'\n", ""))
    );
}

mod key_include_if {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "includeIf.* off the list",
        expected = "git_guard: includeif.gitdir:/x/.path isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'includeIf.*'\n", ""))
    );
}

mod key_instead_of {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "url.*.insteadOf off the list",
        expected = "git_guard: URL.https://x/.INSTEADOF isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'url.*.insteadOf'\n", ""))
    );
}

mod key_proxy {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "http.proxy off the list",
        expected = "git_guard: HTTP.Proxy isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'http.proxy'\n", ""))
    );
}

mod key_gpg {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "gpg.program off the list",
        expected = "git_guard: GPG.program isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'gpg.program'\n", ""))
    );
}

mod key_gpg_ssh {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "gpg.ssh.program off the list, and the wider gpg.*.program that also takes it",
        expected = "git_guard: gpg.SSH.Program isn't refused",
        check_keys(
            &read(GIT_GUARD)
                .replace("    'gpg.ssh.program'\n", "")
                .replace("    'gpg.*.program'\n", "")
        )
    );
}

mod key_git_proxy {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.gitProxy off the list",
        expected = "git_guard: core.GITPROXY isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.gitProxy'\n", ""))
    );
}

mod key_ask_pass {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.askPass off the list",
        expected = "git_guard: Core.AskPass isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.askPass'\n", ""))
    );
}

mod key_attributes_file {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.attributesFile off the list",
        expected = "git_guard: core.ATTRIBUTESFILE isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.attributesFile'\n", ""))
    );
}

mod key_worktree {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.worktree off the list",
        expected = "git_guard: CORE.Worktree isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.worktree'\n", ""))
    );
}

mod key_excludes_file {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "core.excludesFile off the list",
        expected = "git_guard: core.ExcludesFile isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'core.excludesFile'\n", ""))
    );
}

mod key_diff_external {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "diff.external off the list",
        expected = "git_guard: DIFF.External isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'diff.external'\n", ""))
    );
}

mod key_diff_command {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "diff.*.command off the list",
        expected = "git_guard: diff.x.COMMAND isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'diff.*.command'\n", ""))
    );
}

mod key_merge_driver {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "merge.*.driver off the list",
        expected = "git_guard: Merge.x.Driver isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'merge.*.driver'\n", ""))
    );
}

mod key_alias {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "alias.* off the list",
        expected = "git_guard: ALIAS.st isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'alias.*'\n", ""))
    );
}

mod key_pager_cmd {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "pager.* off the list",
        expected = "git_guard: Pager.log isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'pager.*'\n", ""))
    );
}

mod key_sequence_editor {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "sequence.editor off the list",
        expected = "git_guard: Sequence.Editor isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'sequence.editor'\n", ""))
    );
}

mod key_gpg_other {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "gpg.*.program off the list",
        expected = "git_guard: gpg.X509.Program isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'gpg.*.program'\n", ""))
    );
}

mod key_uploadpack {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "remote.*.uploadpack off the list",
        expected = "git_guard: Remote.origin.UploadPack isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'remote.*.uploadpack'\n", ""))
    );
}

mod key_receivepack {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "remote.*.receivepack off the list",
        expected = "git_guard: remote.origin.RECEIVEPACK isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'remote.*.receivepack'\n", ""))
    );
}

mod key_remote_proxy {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "remote.*.proxy off the list",
        expected = "git_guard: REMOTE.origin.proxy isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'remote.*.proxy'\n", ""))
    );
}

mod key_remote_vcs {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "remote.*.vcs off the list",
        expected = "git_guard: remote.origin.VCS isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'remote.*.vcs'\n", ""))
    );
}

mod key_scoped_credential {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "credential.*.helper off the list",
        expected = "git_guard: Credential.https://x/.Helper isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'credential.*.helper'\n", ""))
    );
}

mod key_scoped_proxy {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "http.*.proxy off the list",
        expected = "git_guard: HTTP.https://x/.Proxy isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'http.*.proxy'\n", ""))
    );
}

mod key_fsck {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "fsck.* off the list",
        expected = "git_guard: FSCK.skipList isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'fsck.*'\n", ""))
    );
}

mod key_hook {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "hook.* off the list",
        expected = "git_guard: Hook.x.COMMAND isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'hook.*'\n", ""))
    );
}

mod key_ref_storage {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "extensions.refStorage off the list",
        expected = "git_guard: Extensions.RefStorage isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'extensions.refStorage'\n", ""))
    );
}

mod key_push_instead_of {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "url.*.pushInsteadOf off the list",
        expected = "git_guard: url.https://x/.PUSHINSTEADOF isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'url.*.pushInsteadOf'\n", ""))
    );
}

mod allowed_pattern {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "a pattern on the allowlist",
        expected = "git_guard: remote.* is no exact key as git prints it",
        check_keys(&read(GIT_GUARD).replace("    'gc.auto=0'\n", "    'remote.*=x'\n"))
    );
}

mod harmless_key {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "a pattern wide enough to take what checkout leaves",
        expected = "git_guard: core.repositoryformatversion is refused",
        check_keys(&read(GIT_GUARD).replace("    'core.editor'\n", "    'core.*'\n"))
    );
}

/// What holds the rest of the owner's order. Off Windows nothing can run these lines, so this reads that they're there;
/// `git_guard_script_refuses_what_is_planted` runs them.
fn check_guard_script(script: &str) {
    let code: Vec<&str> = script
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .collect();
    let has = |what: &str| code.iter().any(|l| l.contains(what));
    assert!(
        has("$hook.Name -notlike '*.sample'"),
        "git_guard: doesn't refuse an active hook"
    );
    assert!(
        has("'attributes'") && has("$found += 'info/attributes'"),
        "git_guard: doesn't refuse .git/info/attributes"
    );
    for (name, what) in [
        ("commondir", "'commondir'"),
        ("gitdir", "'gitdir'"),
        ("grafts", "Get-Entry $info 'grafts'"),
        ("alternates", "'alternates'"),
        ("refs/replace/", "(Get-Entry $git 'refs') 'replace'"),
        ("packed-refs", "IndexOf('refs/replace/'"),
        ("reftable", "Get-Entry $git 'reftable'"),
    ] {
        assert!(has(what), "git_guard: doesn't refuse {name}");
    }
    assert!(
        has("$arguments = 'config --no-includes --file \"{0}\" --list --null'"),
        "git_guard: doesn't have git read the config"
    );
    assert!(
        has("$gitExe = 'C:\\Program Files\\Git\\cmd\\git.exe'")
            && has("$start.FileName = $gitExe")
            && has("$start.WorkingDirectory = $env:SystemRoot")
            && !has("Get-Command"),
        "git_guard: starts git where the checkout can reach it"
    );
    assert!(
        has("if (-not [System.IO.File]::Exists($gitExe)) { throw"),
        "git_guard: a missing git doesn't refuse"
    );
    assert!(
        has("if ($run.ExitCode -ne 0) { throw"),
        "git_guard: a failed read doesn't refuse"
    );
    assert!(
        has("elseif ($allowedKeys -cnotcontains $key) {"),
        "git_guard: passes a key nobody listed"
    );
    assert!(
        has("elseif ($allowed -cnotcontains \"$key=$value\") {"),
        "git_guard: passes a value off the allowlist"
    );
    assert!(
        has("$head -cnotmatch '\\A(ref: refs/heads/main|[0-9a-f]{40})\\n?\\z'"),
        "git_guard: doesn't check HEAD"
    );
    assert!(
        has("$found += \"config key: $key\""),
        "git_guard: doesn't name a listed key"
    );
    assert!(
        has("Where-Object { $_.Count -gt 1 }"),
        "git_guard: passes a repeated key"
    );
    assert!(
        has("$originUrl = 'https://github.com/Ma1achy/Principia'")
            && has("if ($pairs -cnotcontains \"remote.origin.url=$originUrl\") {"),
        "git_guard: doesn't pin the fetch URL"
    );
    let text = code.join("\n");
    assert!(
        has("$emptyIndex = '--git-dir \"{0}\" read-tree --empty'"),
        "git_guard: doesn't empty the index"
    );
    let delete = "$null = Invoke-Git ($emptyIndex -f $git[0].FullName)";
    assert!(
        text.contains(&format!(
            "if ($git.Count -and -not $planted.Count -and -not $check) {{\n{delete}"
        )),
        "git_guard: doesn't empty the index"
    );
    assert!(
        text.find("$planted = if ($git.Count)") < text.find(delete),
        "git_guard: empties the index before the checks"
    );
    assert!(
        text.find(delete) < text.find("} catch {\n$planted ="),
        "git_guard: a failed write doesn't refuse"
    );
    let fsck = "$null = Invoke-Git ($objects -f $git.FullName)";
    assert!(has(fsck), "git_guard: doesn't check the objects");
    assert!(
        text.contains(&format!("if (-not $found.Count) {{\n{fsck}")),
        "git_guard: runs fsck on a repo that has failed"
    );
    assert!(
        text.find(fsck) > text.find("foreach ($config in $configs) {"),
        "git_guard: runs fsck before the other checks"
    );
    assert!(
        code.iter().find(|l| !l.is_empty()) == Some(&"$ErrorActionPreference = 'Stop'"),
        "git_guard: doesn't stop at what it can't read"
    );
    assert!(
        has("$planted = @(\"can't read the checkout: $_\")"),
        "git_guard: doesn't refuse what it can't read"
    );
    assert!(
        has(". E:\\runner-tools\\checks\\append-only.ps1")
            && has("[AppendOnly]::Add($env:WIN_REFUSAL_LOG"),
        "git_guard: doesn't log through the append-only writer"
    );
    assert!(
        has("if ($status -ne 0) {"),
        "git_guard: doesn't report a failed log write"
    );
    assert!(
        has("foreach ($item in @(Get-ChildItem -LiteralPath $root -Force)) {")
            && has("[System.IO.Directory]::Delete($item.FullName, $false)")
            && has("[System.IO.File]::Delete($item.FullName)"),
        "git_guard: doesn't empty the checkout"
    );
    assert!(
        !has("cmd ") && !has("Invoke-Expression") && !has("Start-Process"),
        "git_guard: puts a planted name on a shell's line"
    );
    assert!(
        code.join("\n")
            .contains("if (-not $link) {\nforeach ($inner")
            && has("-band [System.IO.FileAttributes]::ReparsePoint"),
        "git_guard: follows a link when it empties the checkout"
    );
    assert!(
        has("Get-Entry $git 'config.worktree'"),
        "git_guard: doesn't check config.worktree's keys"
    );
    assert_eq!(
        code.iter().rfind(|l| !l.is_empty()),
        Some(&"exit 1"),
        "git_guard: doesn't fail the job"
    );
}

#[test]
fn git_guard_script_holds_the_owners_order() {
    check_guard_script(&read(GIT_GUARD));
}

/// The script's lines that empty the index.
#[cfg(feature = "controls")]
const EMPTY_INDEX: &str = "    if ($git.Count -and -not $planted.Count -and -not $check) {\n        $null = Invoke-Git ($emptyIndex -f $git[0].FullName)\n    }\n";

/// The script with `from` replaced by `to`.
#[cfg(feature = "controls")]
fn guard_edited(from: &str, to: &str) -> String {
    let script = read(GIT_GUARD);
    assert!(script.contains(from), "the edit's target is gone: {from}");
    script.replacen(from, to, 1)
}

mod any_hook {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "every hook passing",
        expected = "git_guard: doesn't refuse an active hook",
        check_guard_script(&guard_edited("-notlike '*.sample'", "-notlike '*'"))
    );
}

mod attributes {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "attributes left alone",
        expected = "git_guard: doesn't refuse .git/info/attributes",
        check_guard_script(&guard_edited("$found += 'info/attributes'", "$null"))
    );
}

mod reader_of_its_own {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a config read by anything but git",
        expected = "git_guard: doesn't have git read the config",
        check_guard_script(&guard_edited(
            "$arguments = 'config --no-includes",
            "$arguments = 'config --includes"
        ))
    );
}

mod git_from_the_checkout {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "git started in the checkout",
        expected = "git_guard: starts git where the checkout can reach it",
        check_guard_script(&guard_edited(
            "$start.WorkingDirectory = $env:SystemRoot",
            "$start.WorkingDirectory = $path"
        ))
    );
}

mod git_by_name {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "git left to be found by name",
        expected = "git_guard: starts git where the checkout can reach it",
        check_guard_script(&guard_edited(
            "$start.FileName = $gitExe",
            "$start.FileName = 'git'"
        ))
    );
}

mod git_through_path {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "git found through PATH",
        expected = "git_guard: starts git where the checkout can reach it",
        check_guard_script(&guard_edited(
            "$gitExe = 'C:\\Program Files\\Git\\cmd\\git.exe'",
            "$gitExe = (Get-Command git).Source"
        ))
    );
}

mod missing_git_passes {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a missing git left to the start",
        expected = "git_guard: a missing git doesn't refuse",
        check_guard_script(&guard_edited(
            "    if (-not [System.IO.File]::Exists($gitExe)) { throw \"no git at $gitExe\" }\n",
            ""
        ))
    );
}

mod failed_read_passes {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a failed read taken as no keys",
        expected = "git_guard: a failed read doesn't refuse",
        check_guard_script(&guard_edited(
            "    if ($run.ExitCode -ne 0) { throw \"git $what failed, status $($run.ExitCode): $($errors.Result)\" }\n",
            ""
        ))
    );
}

mod objects_unchecked {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "no fsck",
        expected = "git_guard: doesn't check the objects",
        check_guard_script(&guard_edited(
            "        $null = Invoke-Git ($objects -f $git.FullName)\n",
            ""
        ))
    );
}

mod fsck_on_a_failed_repo {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "fsck run on a repo already refused",
        expected = "git_guard: runs fsck on a repo that has failed",
        check_guard_script(&guard_edited("if (-not $found.Count) {", "if ($true) {"))
    );
}

mod fsck_first {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "fsck run before the other checks",
        expected = "git_guard: runs fsck before the other checks",
        check_guard_script(
            &guard_edited(
                "    if (-not $found.Count) {\n        $null = Invoke-Git ($objects -f $git.FullName)\n    }\n",
                ""
            )
            .replacen(
                "    $found = @()\n",
                "    $found = @()\n    if (-not $found.Count) {\n        $null = Invoke-Git ($objects -f $git.FullName)\n    }\n",
                1
            )
        )
    );
}

mod commondir {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "commondir passed",
        expected = "git_guard: doesn't refuse commondir",
        check_guard_script(&guard_edited("'commondir', 'gitdir'", "'gitdir'"))
    );
}

mod gitdir {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "gitdir passed",
        expected = "git_guard: doesn't refuse gitdir",
        check_guard_script(&guard_edited("'commondir', 'gitdir'", "'commondir'"))
    );
}

mod grafts {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "info/grafts passed",
        expected = "git_guard: doesn't refuse grafts",
        check_guard_script(&guard_edited(
            "    if (Get-Entry $info 'grafts') { $found += 'info/grafts' }\n",
            ""
        ))
    );
}

mod alternates {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "objects/info/alternates passed",
        expected = "git_guard: doesn't refuse alternates",
        check_guard_script(&guard_edited(
            "    if (Get-Entry (Get-Entry (Get-Entry $git 'objects') 'info') 'alternates') { $found += 'objects/info/alternates' }\n",
            ""
        ))
    );
}

mod replace_refs {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "refs/replace/ passed",
        expected = "git_guard: doesn't refuse refs/replace/",
        check_guard_script(&guard_edited(
            "(Get-Entry $git 'refs') 'replace'",
            "(Get-Entry $git 'refs') 'other'"
        ))
    );
}

mod packed_replace_refs {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a replace ref in packed-refs passed",
        expected = "git_guard: doesn't refuse packed-refs",
        check_guard_script(&guard_edited(
            "IndexOf('refs/replace/'",
            "IndexOf('refs/other/'"
        ))
    );
}

mod unlisted_key_passes {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a key nobody listed passing",
        expected = "git_guard: passes a key nobody listed",
        check_guard_script(&guard_edited(
            "elseif ($allowedKeys -cnotcontains $key) {",
            "elseif ($false) {"
        ))
    );
}

mod listed_key_unnamed {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a listed key not named in the reason",
        expected = "git_guard: doesn't name a listed key",
        check_guard_script(&guard_edited(
            "$found += \"config key: $key\"",
            "$found += 'a config key'"
        ))
    );
}

mod repeated_key_passes {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a repeated key passing",
        expected = "git_guard: passes a repeated key",
        check_guard_script(&guard_edited(
            "Where-Object { $_.Count -gt 1 }",
            "Where-Object { $false }"
        ))
    );
}

mod value_unchecked {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "keys checked and values not",
        expected = "git_guard: passes a value off the allowlist",
        check_guard_script(&guard_edited(
            "elseif ($allowed -cnotcontains \"$key=$value\") {",
            "elseif ($false) {"
        ))
    );
}

mod head_unchecked {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "any HEAD",
        expected = "git_guard: doesn't check HEAD",
        check_guard_script(&guard_edited(
            "$head -cnotmatch '\\A(ref: refs/heads/main|[0-9a-f]{40})\\n?\\z'",
            "$head -cnotmatch '.'"
        ))
    );
}

mod url_unpinned {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "any fetch URL passing",
        expected = "git_guard: doesn't pin the fetch URL",
        check_guard_script(&guard_edited(
            "if ($pairs -cnotcontains \"remote.origin.url=$originUrl\") {",
            "if ($false) {"
        ))
    );
}

mod url_elsewhere {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "the pin on another repo",
        expected = "git_guard: doesn't pin the fetch URL",
        check_guard_script(&guard_edited(
            "$originUrl = 'https://github.com/Ma1achy/Principia'",
            "$originUrl = 'https://github.com/other/Principia'"
        ))
    );
}

mod index_line_gone {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a line that doesn't empty it",
        expected = "git_guard: doesn't empty the index",
        check_guard_script(&guard_edited(" read-tree --empty'", " read-tree HEAD'"))
    );
}

mod index_kept {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "the index left in place",
        expected = "git_guard: doesn't empty the index",
        check_guard_script(&guard_edited(EMPTY_INDEX, ""))
    );
}

mod index_deleted_by_check {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "the check form emptying the index",
        expected = "git_guard: doesn't empty the index",
        check_guard_script(&guard_edited(" -and -not $check) {", ") {"))
    );
}

mod index_deleted_first {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "the index emptied before the checks",
        expected = "git_guard: empties the index before the checks",
        check_guard_script(&guard_edited(EMPTY_INDEX, "").replacen(
            "    $planted = if ($git.Count)",
            &format!("{EMPTY_INDEX}    $planted = if ($git.Count)"),
            1
        ))
    );
}

mod index_delete_unguarded {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a failed write past the try",
        expected = "git_guard: a failed write doesn't refuse",
        check_guard_script(&guard_edited(EMPTY_INDEX, "").replacen(
            "if (-not $planted.Count) {\n",
            &format!("{EMPTY_INDEX}if (-not $planted.Count) {{\n"),
            1
        ))
    );
}

mod reftable {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a .git/reftable folder passed",
        expected = "git_guard: doesn't refuse reftable",
        check_guard_script(&guard_edited(
            "    if (Get-Entry $git 'reftable') { $found += '.git/reftable' }\n",
            ""
        ))
    );
}

mod name_on_a_shell_line {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a file deleted through cmd, where a planted name runs",
        expected = "git_guard: puts a planted name on a shell's line",
        check_guard_script(&guard_edited(
            "            [System.IO.File]::Delete($item.FullName)\n",
            "            [System.IO.File]::Delete($item.FullName)\n            cmd /c del \"$($item.FullName)\"\n"
        ))
    );
}

mod link_followed {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "emptying what a link points to",
        expected = "git_guard: follows a link when it empties the checkout",
        check_guard_script(&guard_edited(
            "            if (-not $link) {\n                foreach ($inner",
            "            if ($true) {\n                foreach ($inner"
        ))
    );
}

mod carries_on {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "an error passed over",
        expected = "git_guard: doesn't stop at what it can't read",
        check_guard_script(&guard_edited(
            "$ErrorActionPreference = 'Stop'",
            "$ErrorActionPreference = 'Continue'"
        ))
    );
}

mod unread_passes {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "an unreadable .git passing",
        expected = "git_guard: doesn't refuse what it can't read",
        check_guard_script(&guard_edited(
            "$planted = @(\"can't read the checkout: $_\")",
            "$planted = @()"
        ))
    );
}

mod unlogged {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a line through Add-Content, which the log refuses",
        expected = "git_guard: doesn't log through the append-only writer",
        check_guard_script(&guard_edited(
            "[AppendOnly]::Add($env:WIN_REFUSAL_LOG, ",
            "Add-Content $env:WIN_REFUSAL_LOG "
        ))
    );
}

mod log_status_dropped {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a failed log write passed over",
        expected = "git_guard: doesn't report a failed log write",
        check_guard_script(&guard_edited(
            "    if ($status -ne 0) { \"git guard: could not write the refusal log: status $status\" }\n",
            ""
        ))
    );
}

mod kept {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "the checkout kept after a refusal",
        expected = "git_guard: doesn't empty the checkout",
        check_guard_script(&guard_edited(
            "            [System.IO.File]::Delete($item.FullName)\n",
            ""
        ))
    );
}

mod worktree_config_skipped {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "config.worktree left unread",
        expected = "git_guard: doesn't check config.worktree's keys",
        check_guard_script(&guard_edited("Get-Entry $git 'config.worktree'", "$null"))
    );
}

mod green {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "a refusal that passes the job",
        expected = "git_guard: doesn't fail the job",
        check_guard_script(&guard_edited("left\" }\nexit 1\n", "left\" }\nexit 0\n"))
    );
}

/// git, away from this user's config and any repo the test stands in.
fn git(dir: &Path) -> std::process::Command {
    let mut git = std::process::Command::new("git");
    git.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.join("no_such_config"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    git
}

/// `git args` in `dir`, which must work; its output.
fn git_ok(dir: &Path, args: &[&str]) -> String {
    let out = git(dir).args(args).timed_output().expect("git ran");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// `git args` in `dir`, as the action runs what it only warns about.
fn git_try(dir: &Path, args: &[&str]) {
    git(dir).args(args).timed_output().expect("git ran");
}

/// A commit of what's in `dir`'s index, at most.
fn commit(dir: &Path, message: &str) {
    git_ok(
        dir,
        &[
            "-c",
            "user.name=x",
            "-c",
            "user.email=x@x",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            message,
        ],
    );
}

/// A repo with a history, to fetch from.
fn origin_repo(dir: &Path) {
    std::fs::create_dir_all(dir.join(".cargo")).expect("a folder");
    git_ok(dir, &["init", "-q"]);
    std::fs::write(dir.join(".cargo/config.toml"), "[build]\n").expect("a file");
    for text in ["one", "two"] {
        std::fs::write(dir.join("a.txt"), text).expect("a file");
        git_ok(dir, &["add", "."]);
        commit(dir, text);
    }
}

/// What the action does to a repo it keeps, before it fetches, in its order (`git-directory-helper.ts:34-105`). Where a step
/// fails the action gives up on the repo and deletes it, green, so a step here fails the test.
fn action_prepare(dir: &Path) {
    let step = |args: &[&str]| {
        let out = git(dir).args(args).timed_output().expect("git ran");
        assert!(
            out.status.success(),
            "the action would recreate the repo: git {args:?}: {out:?}"
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    for lock in ["index.lock", "shallow.lock"] {
        let _ = std::fs::remove_file(dir.join(".git").join(lock));
    }
    let head = git(dir)
        .args([
            "rev-parse",
            "--symbolic-full-name",
            "--verify",
            "--quiet",
            "HEAD",
        ])
        .timed_output()
        .expect("git ran");
    if String::from_utf8_lossy(&head.stdout).starts_with("refs/heads/") {
        step(&["checkout", "--detach", "-q"]);
    }
    for branch in step(&["rev-parse", "--symbolic-full-name", "--branches"]).lines() {
        let name = branch.strip_prefix("refs/heads/").unwrap_or(branch);
        step(&["branch", "--delete", "--force", name]);
    }
    step(&["submodule", "status"]);
    step(&["clean", "-ffdx", "-q"]);
    step(&["reset", "--hard", "-q", "HEAD"]);
}

/// What the action does to fetch `origin`'s HEAD and check it out (`git-source-provider.ts`), with `fetch-depth: 1`.
fn action_fetch(origin: &Path, dir: &Path) {
    let sha = git_ok(origin, &["rev-parse", "HEAD"]);
    let refspec = format!("+{sha}:refs/remotes/origin/main");
    git_ok(dir, &["config", "--local", "gc.auto", "0"]);
    git_ok(
        dir,
        &[
            "-c",
            "protocol.version=2",
            "fetch",
            "-q",
            "--no-tags",
            "--prune",
            "--no-recurse-submodules",
            "--depth=1",
            "origin",
            &refspec,
        ],
    );
    git_ok(dir, &["sparse-checkout", "disable"]);
    git_try(
        dir,
        &[
            "config",
            "--local",
            "--unset-all",
            "extensions.worktreeConfig",
        ],
    );
    git_ok(
        dir,
        &[
            "checkout",
            "-q",
            "--force",
            "-B",
            "main",
            "refs/remotes/origin/main",
        ],
    );
}

/// `dir` as actions/checkout leaves it on a first run.
fn kept_repo(origin: &Path, dir: &Path) {
    let url = origin.to_string_lossy().replace('\\', "/");
    let url = format!("file:///{}", url.trim_start_matches('/'));
    std::fs::create_dir_all(dir).expect("a folder");
    git_ok(dir, &["init", "-q"]);
    git_ok(dir, &["remote", "add", "origin", &url]);
    action_fetch(origin, dir);
    assert!(dir.join(".git/shallow").exists(), "a shallow repo");
}

/// Puts other content under the name of the blob at `a.txt`. Renames, as Windows renames a read-only object but won't
/// overwrite one, and the fetch left it loose, as it has too few objects to keep a pack.
fn overwrite_blob(dir: &Path) {
    let object = |id: &str| dir.join(".git/objects").join(&id[..2]).join(&id[2..]);
    let old = git_ok(dir, &["rev-parse", "HEAD:a.txt"]);
    std::fs::write(dir.join("planted.txt"), "planted").expect("a file");
    let new = git_ok(dir, &["hash-object", "-w", "planted.txt"]);
    std::fs::remove_file(dir.join("planted.txt")).expect("a file gone");
    std::fs::rename(object(&old), dir.join(".git/moved")).expect("a loose object");
    std::fs::rename(object(&new), object(&old)).expect("a loose object");
}

/// A scratch folder of this test's own.
fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "ci_self_hosted_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// git on the script's `$name` line, with `path` for its `{0}`.
fn script_git(script: &str, name: &str, path: &Path, cwd: &Path) -> std::process::Output {
    let start = format!("${name} = '");
    let template = script
        .lines()
        .find_map(|l| l.trim().strip_prefix(start.as_str()))
        .and_then(|l| l.strip_suffix('\''))
        .expect("git_guard: no such line");
    let args = template.split_whitespace().map(|word| {
        if word == "\"{0}\"" {
            path.as_os_str().to_owned()
        } else {
            word.into()
        }
    });
    git(cwd).args(args).timed_output().expect("git ran")
}

/// The keys of `file` by the script's own `git config` line, or nothing if git refuses it.
fn git_keys(script: &str, file: &Path) -> Option<Vec<String>> {
    let out = script_git(script, "arguments", file, file.parent().expect("a parent"));
    out.status.success().then(|| {
        String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|k| !k.is_empty())
            .map(|entry| entry.split('\n').next().unwrap_or(entry).to_owned())
            .collect()
    })
}

/// A config read by the script's own line finds the key under each form that hides one from a reader of its own, follows
/// no include, and fails on what git can't read.
fn check_git_reading(script: &str) {
    let dir = scratch("git");
    let config = dir.join("config");
    let listed = listed_keys(script);
    for (form, text) in [
        (
            "a \\ continuation",
            "[core]\n\tx = a\\\n[other]\n\thooksPath = /evil\n",
        ),
        (
            "a bare CR",
            "[core]\n\tx = a\r[other]\n\thooksPath = /evil\n",
        ),
        ("an empty url subsection", "[url \"\"]\n\tinsteadOf = x\n"),
        ("an empty filter subsection", "[filter \"\"]\n\tclean = x\n"),
    ] {
        std::fs::write(&config, text).expect("a config");
        let keys = git_keys(script, &config).unwrap_or_default();
        assert!(
            keys.iter().any(|k| listed.iter().any(|p| like(k, p))),
            "git_guard: the config read passed {form}: {keys:?}"
        );
    }
    std::fs::write(dir.join("other"), "[core]\n\tpager = x\n").expect("a config");
    std::fs::write(&config, "[include]\n\tpath = other\n").expect("a config");
    let keys = git_keys(script, &config).unwrap_or_default();
    assert_eq!(
        keys,
        ["include.path"],
        "git_guard: the config read followed an include"
    );
    std::fs::write(&config, "[core\n\tbare = false\n").expect("a config");
    assert_eq!(
        git_keys(script, &config),
        None,
        "git_guard: a config git can't read was read"
    );
}

#[test]
fn git_guard_reads_a_config_as_git_does() {
    check_git_reading(&read(GIT_GUARD));
}

negative_control!(
    git_guard_reads_a_config_as_git_does,
    "a read that follows an include",
    expected = "git_guard: the config read followed an include",
    check_git_reading(&read(GIT_GUARD).replace("--no-includes", "--includes"))
);

mod no_separator {
    use super::*;
    negative_control!(
        git_guard_reads_a_config_as_git_does,
        "a read without its separator",
        expected = "git_guard: the config read passed a \\ continuation",
        check_git_reading(&read(GIT_GUARD).replace(" --null", ""))
    );
}

mod empty_url {
    use super::*;
    negative_control!(
        git_guard_reads_a_config_as_git_does,
        "`[url \"\"]` off the list",
        expected = "git_guard: the config read passed an empty url subsection",
        check_git_reading(&read(GIT_GUARD).replace("    'url.*.insteadOf'\n", ""))
    );
}

mod empty_filter {
    use super::*;
    negative_control!(
        git_guard_reads_a_config_as_git_does,
        "`[filter \"\"]` off the list",
        expected = "git_guard: the config read passed an empty filter subsection",
        check_git_reading(&read(GIT_GUARD).replace("    'filter.*.clean'\n", ""))
    );
}

/// The `key=value` pairs of `file` by the script's own line, as the script joins them.
fn git_pairs(script: &str, file: &Path) -> Vec<String> {
    let out = script_git(script, "arguments", file, file.parent().expect("a parent"));
    assert!(out.status.success(), "the config: {out:?}");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.replacen('\n', "=", 1))
        .collect()
}

/// `$allowed` is what the action's own commands leave: all of it, and only it, in a real repo.
fn check_allowlist(script: &str) {
    // Written only where the filesystem's probe calls for it. Each form the PC has is on the list; a Mac's aren't.
    const PROBED: [&str; 3] = [
        "core.symlinks=false",
        "core.ignorecase=true",
        "core.filemode=false",
    ];
    const NOT_THE_PC: [&str; 2] = ["core.filemode=true", "core.precomposeunicode=true"];
    let base = scratch("allow");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let mut held = git_pairs(script, &dir.join(".git/config"));
    held.extend(git_pairs(script, &dir.join(".git/config.worktree")));
    // This one's value is the scratch origin here.
    held.retain(|pair| {
        !NOT_THE_PC.contains(&pair.as_str()) && !pair.starts_with("remote.origin.url=")
    });
    let allowed = allowed_pairs(script);
    for pair in &held {
        assert!(
            allowed.contains(&pair.as_str()),
            "git_guard: a clean checkout holds {pair}, off the allowlist"
        );
    }
    for pair in allowed {
        assert!(
            PROBED.contains(&pair)
                || pair.starts_with("remote.origin.url=")
                || held.iter().any(|p| p == pair),
            "git_guard: no clean checkout holds {pair}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(dir.join(".git/HEAD")).expect("HEAD"),
        "ref: refs/heads/main\n",
        "git_guard: a clean checkout's HEAD has another form"
    );
}

#[test]
fn git_guard_allows_what_a_clean_checkout_holds() {
    check_allowlist(&read(GIT_GUARD));
}

negative_control!(
    git_guard_allows_what_a_clean_checkout_holds,
    "a key the action writes off the list",
    expected = "git_guard: a clean checkout holds gc.auto=0, off the allowlist",
    check_allowlist(&read(GIT_GUARD).replace("    'gc.auto=0'\n", ""))
);

mod sparse_key_off {
    use super::*;
    negative_control!(
        git_guard_allows_what_a_clean_checkout_holds,
        "a key of config.worktree off the list",
        expected = "git_guard: a clean checkout holds index.sparse=false, off the allowlist",
        check_allowlist(&read(GIT_GUARD).replace("    'index.sparse=false'\n", ""))
    );
}

mod allowed_stale {
    use super::*;
    negative_control!(
        git_guard_allows_what_a_clean_checkout_holds,
        "a key no checkout writes on the list",
        expected = "git_guard: no clean checkout holds protocol.ext.allow=always",
        check_allowlist(&read(GIT_GUARD).replace(
            "    'gc.auto=0'\n",
            "    'gc.auto=0'\n    'protocol.ext.allow=always'\n"
        ))
    );
}

/// A value other than the one on the list, one for each key there, each named by its key.
const WRONG_VALUES: [&str; 14] = [
    "core.repositoryformatversion=1",
    "core.filemode=true",
    "core.bare=true",
    "core.logallrefupdates=false",
    "core.symlinks=true",
    "core.ignorecase=false",
    "remote.origin.url=https://github.com/Ma1achy/Principia.git",
    "remote.origin.fetch=+refs/heads/*:refs/replace/*",
    "gc.auto=1",
    "branch.main.remote=ext::x",
    "branch.main.merge=refs/heads/other",
    "core.sparsecheckout=true",
    "core.sparsecheckoutcone=true",
    "index.sparse=true",
];

/// The whole of `$allowed` as a config file's text, with `wrong`, a pair, in place of its key's value.
fn config_text(script: &str, wrong: Option<&str>) -> String {
    let mut text = String::new();
    for pair in allowed_pairs(script) {
        let (key, value) = pair.split_once('=').expect("a pair");
        let value = wrong
            .and_then(|w| w.split_once('='))
            .filter(|(k, _)| *k == key)
            .map_or(value, |(_, v)| v);
        let (name, variable) = key.rsplit_once('.').expect("a section");
        match name.split_once('.') {
            Some((section, sub)) => text.push_str(&format!("[{section} \"{sub}\"]\n")),
            None => text.push_str(&format!("[{name}]\n")),
        }
        text.push_str(&format!("\t{variable} = {value}\n"));
    }
    text
}

/// Each key on the list has its one value: the script's list comes back from git as it was written, and no wrong value for
/// any key is on it, as the script's `-cnotcontains` compares.
fn check_values(script: &str) {
    let allowed = allowed_pairs(script);
    let file = scratch("values").join("config");
    std::fs::write(&file, config_text(script, None)).expect("a config");
    assert_eq!(
        git_pairs(script, &file),
        allowed,
        "git_guard: git doesn't read the list back"
    );
    for wrong in WRONG_VALUES {
        assert!(!allowed.contains(&wrong), "git_guard: {wrong} passes");
    }
    assert_eq!(
        allowed_keys(script).len(),
        WRONG_VALUES.len(),
        "git_guard: a key has no wrong value here"
    );
}

#[test]
fn git_guard_pins_each_value() {
    check_values(&read(GIT_GUARD));
}

negative_control!(
    git_guard_pins_each_value,
    "a key with no case here",
    expected = "git_guard: a key has no wrong value here",
    check_values(&read(GIT_GUARD).replace("    'gc.auto=0'\n", ""))
);

mod list_misread {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a read of keys only",
        expected = "git_guard: git doesn't read the list back",
        check_values(&read(GIT_GUARD).replace(" --list --null", " --list --name-only --null"))
    );
}

mod value_core_format {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.repositoryformatversion",
        expected = "git_guard: core.repositoryformatversion=1 passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.repositoryformatversion=0'\n",
            "    'core.repositoryformatversion=0'\n    'core.repositoryformatversion=1'\n"
        ))
    );
}

mod value_core_filemode {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.filemode",
        expected = "git_guard: core.filemode=true passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.filemode=false'\n",
            "    'core.filemode=false'\n    'core.filemode=true'\n"
        ))
    );
}

mod value_core_bare {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.bare",
        expected = "git_guard: core.bare=true passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.bare=false'\n",
            "    'core.bare=false'\n    'core.bare=true'\n"
        ))
    );
}

mod value_core_reflog {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.logallrefupdates",
        expected = "git_guard: core.logallrefupdates=false passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.logallrefupdates=true'\n",
            "    'core.logallrefupdates=true'\n    'core.logallrefupdates=false'\n"
        ))
    );
}

mod value_core_symlinks {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.symlinks",
        expected = "git_guard: core.symlinks=true passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.symlinks=false'\n",
            "    'core.symlinks=false'\n    'core.symlinks=true'\n"
        ))
    );
}

mod value_core_ignorecase {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.ignorecase",
        expected = "git_guard: core.ignorecase=false passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.ignorecase=true'\n",
            "    'core.ignorecase=true'\n    'core.ignorecase=false'\n"
        ))
    );
}

mod value_origin_url {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for remote.origin.url",
        expected = "git_guard: remote.origin.url=https://github.com/Ma1achy/Principia.git passes",
        check_values(&read(GIT_GUARD).replace(
            "    'remote.origin.url=https://github.com/Ma1achy/Principia'\n",
            "    'remote.origin.url=https://github.com/Ma1achy/Principia'\n    'remote.origin.url=https://github.com/Ma1achy/Principia.git'\n"
        ))
    );
}

mod value_origin_fetch {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for remote.origin.fetch",
        expected = "git_guard: remote.origin.fetch=+refs/heads/*:refs/replace/* passes",
        check_values(&read(GIT_GUARD).replace(
            "    'remote.origin.fetch=+refs/heads/*:refs/remotes/origin/*'\n",
            "    'remote.origin.fetch=+refs/heads/*:refs/remotes/origin/*'\n    'remote.origin.fetch=+refs/heads/*:refs/replace/*'\n"
        ))
    );
}

mod value_gc_auto {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for gc.auto",
        expected = "git_guard: gc.auto=1 passes",
        check_values(
            &read(GIT_GUARD).replace("    'gc.auto=0'\n", "    'gc.auto=0'\n    'gc.auto=1'\n")
        )
    );
}

mod value_branch_remote {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for branch.main.remote",
        expected = "git_guard: branch.main.remote=ext::x passes",
        check_values(&read(GIT_GUARD).replace(
            "    'branch.main.remote=origin'\n",
            "    'branch.main.remote=origin'\n    'branch.main.remote=ext::x'\n"
        ))
    );
}

mod value_branch_merge {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for branch.main.merge",
        expected = "git_guard: branch.main.merge=refs/heads/other passes",
        check_values(&read(GIT_GUARD).replace(
            "    'branch.main.merge=refs/heads/main'\n",
            "    'branch.main.merge=refs/heads/main'\n    'branch.main.merge=refs/heads/other'\n"
        ))
    );
}

mod value_sparse {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.sparsecheckout",
        expected = "git_guard: core.sparsecheckout=true passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.sparsecheckout=false'\n",
            "    'core.sparsecheckout=false'\n    'core.sparsecheckout=true'\n"
        ))
    );
}

mod value_sparse_cone {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for core.sparsecheckoutcone",
        expected = "git_guard: core.sparsecheckoutcone=true passes",
        check_values(&read(GIT_GUARD).replace(
            "    'core.sparsecheckoutcone=false'\n",
            "    'core.sparsecheckoutcone=false'\n    'core.sparsecheckoutcone=true'\n"
        ))
    );
}

mod value_index_sparse {
    use super::*;
    negative_control!(
        git_guard_pins_each_value,
        "a second value for index.sparse",
        expected = "git_guard: index.sparse=true passes",
        check_values(&read(GIT_GUARD).replace(
            "    'index.sparse=false'\n",
            "    'index.sparse=false'\n    'index.sparse=true'\n"
        ))
    );
}

/// A planted refspec has the action's own fetch write a replace ref after the guard has finished. The value is what shows
/// it, as every key is on the list; with `names_only` the check is the one that passed this.
fn check_planted_refspec(script: &str, names_only: bool) {
    let base = scratch("refspec");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let blob = git_ok(&origin, &["rev-parse", "HEAD:a.txt"]);
    std::fs::write(origin.join("planted.txt"), "planted").expect("a file");
    let planted = git_ok(&origin, &["hash-object", "-w", "planted.txt"]);
    std::fs::remove_file(origin.join("planted.txt")).expect("a file gone");
    git_ok(
        &origin,
        &["config", "uploadpack.allowAnySHA1InWant", "true"],
    );
    let refspec = format!("+{planted}:refs/replace/{blob}");
    git_ok(&dir, &["config", "remote.origin.fetch", &refspec]);
    let held = git_pairs(script, &dir.join(".git/config"));
    let allowed = allowed_pairs(script);
    let keys = allowed_keys(script);
    let on_the_list = |pair: &String| {
        let key = pair.split_once('=').expect("a pair").0;
        if names_only {
            keys.contains(&key)
        } else {
            allowed.contains(&pair.as_str())
        }
    };
    assert!(
        !held
            .iter()
            .filter(|p| p.starts_with("remote.origin.fetch="))
            .all(on_the_list),
        "git_guard: passes a planted refspec"
    );
    action_prepare(&dir);
    action_fetch(&origin, &dir);
    assert!(
        dir.join(".git/refs/replace").join(&blob).exists(),
        "the plant no longer makes a replace ref"
    );
}

#[test]
fn git_guard_refuses_a_planted_refspec() {
    check_planted_refspec(&read(GIT_GUARD), false);
}

negative_control!(
    git_guard_refuses_a_planted_refspec,
    "a list of names, without their values",
    expected = "git_guard: passes a planted refspec",
    check_planted_refspec(&read(GIT_GUARD), true)
);

/// The script's own line that empties the index.
fn guard_writes_the_index(script: &str, dir: &Path) {
    let out = script_git(
        script,
        "emptyIndex",
        &dir.join(".git"),
        dir.parent().expect("a parent"),
    );
    assert!(out.status.success(), "the empty index: {out:?}");
}

/// One kept run and then a second: `.git` stays, so the clone is paid only when something was planted, and the checkout is
/// the commit's. `wipe` is what the guard did before: no index at all.
fn check_kept_runs(script: &str, wipe: bool) {
    let base = scratch("kept");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let marker = dir.join(".git/kept_marker");
    std::fs::write(&marker, "x").expect("a file");
    for run in ["next", "after"] {
        if wipe {
            std::fs::remove_file(dir.join(".git/index")).expect("the index");
        } else {
            guard_writes_the_index(script, &dir);
        }
        action_prepare(&dir);
        commit(&origin, run);
        action_fetch(&origin, &dir);
        assert_eq!(
            git_ok(&dir, &["rev-parse", "HEAD"]),
            git_ok(&origin, &["rev-parse", "HEAD"]),
            "git_guard: the kept run isn't at the commit"
        );
        assert_eq!(
            git_ok(&dir, &["status", "--porcelain"]),
            "",
            "git_guard: the kept run isn't the commit's tree"
        );
    }
    assert!(marker.exists(), "git_guard: the kept run lost .git");
}

#[test]
fn git_guard_keeps_the_repo_across_runs() {
    check_kept_runs(&read(GIT_GUARD), false);
}

negative_control!(
    git_guard_keeps_the_repo_across_runs,
    "no index at all, which the action answers by recreating the repo",
    expected = "the action would recreate the repo",
    check_kept_runs(&read(GIT_GUARD), true)
);

/// A detached HEAD at an empty tree, the form the review planted: the guard's HEAD check allows the form, so what comes after
/// it has to put the tree right.
fn check_planted_head(script: &str, empty_index: bool) {
    let base = scratch("head");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let tree = git_ok(&dir, &["mktree", "--missing"]);
    let empty = git_ok(
        &dir,
        &[
            "-c",
            "user.name=x",
            "-c",
            "user.email=x@x",
            "commit-tree",
            &tree,
            "-m",
            "x",
        ],
    );
    std::fs::write(dir.join(".git/HEAD"), format!("{empty}\n")).expect("HEAD");
    if empty_index {
        guard_writes_the_index(script, &dir);
    }
    action_prepare(&dir);
    commit(&origin, "next");
    action_fetch(&origin, &dir);
    assert_eq!(
        git_ok(&dir, &["status", "--porcelain"]),
        "",
        "git_guard: a planted HEAD changed the tree"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("a.txt")).expect("the file"),
        "two",
        "git_guard: a planted HEAD changed the tree"
    );
}

#[test]
fn git_guard_a_detached_head_changes_nothing() {
    check_planted_head(&read(GIT_GUARD), true);
}

negative_control!(
    git_guard_a_detached_head_changes_nothing,
    "a line that writes no index",
    expected = "the empty index",
    check_planted_head(
        &read(GIT_GUARD).replace(" read-tree --empty", " read-tree --no-such-flag"),
        true
    )
);

/// A tracked file edited in place, its time restored and `core.trustctime` off, so the index calls it unchanged; then the
/// next run, with the index deleted as the guard deletes it or not.
fn check_tampered_file(empty_index: bool) {
    let base = scratch("tamper");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let file = dir.join("a.txt");
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let stamp = || {
        std::fs::File::options()
            .write(true)
            .open(&file)
            .and_then(|f| f.set_modified(past))
            .expect("a time set");
    };
    stamp();
    git_ok(&dir, &["update-index", "--refresh", "-q"]);
    std::fs::write(&file, "owt").expect("the edit");
    stamp();
    git_ok(&dir, &["config", "core.trustctime", "false"]);
    assert_eq!(git_ok(&dir, &["status", "--porcelain"]), "", "the plant");
    if empty_index {
        guard_writes_the_index(&read(GIT_GUARD), &dir);
    }
    action_prepare(&dir);
    commit(&origin, "next");
    action_fetch(&origin, &dir);
    assert_eq!(
        std::fs::read_to_string(&file).expect("the file"),
        "two",
        "git_guard: a tampered file survived the checkout"
    );
}

/// A gitlink over `.cargo` in the index, with `.cargo` a repo of its own that holds a planted file.
fn check_gitlink(empty_index: bool) {
    let base = scratch("gitlink");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let head = git_ok(&dir, &["rev-parse", "HEAD"]);
    git_ok(&dir, &["rm", "-q", "--cached", "-r", ".cargo"]);
    let entry = format!("160000,{head},.cargo");
    git_ok(&dir, &["update-index", "--add", "--cacheinfo", &entry]);
    // Mapped, or the action's `submodule status` fails and it recreates the repo, which would clear the plant at a clone's
    // cost.
    let mapping = "[submodule \".cargo\"]\n\tpath = .cargo\n\turl = x\n";
    std::fs::write(dir.join(".gitmodules"), mapping).expect("a file");
    git_ok(&dir, &["add", ".gitmodules"]);
    let nested = dir.join(".cargo");
    git_ok(&nested, &["init", "-q"]);
    std::fs::write(nested.join("config"), "planted").expect("a file");
    git_ok(&nested, &["add", "config"]);
    commit(&nested, "planted");
    if empty_index {
        guard_writes_the_index(&read(GIT_GUARD), &dir);
    }
    action_prepare(&dir);
    commit(&origin, "next");
    action_fetch(&origin, &dir);
    assert!(
        !nested.join("config").exists(),
        "git_guard: a file planted under a gitlink survived the checkout"
    );
}

#[test]
fn git_guard_emptying_the_index_clears_both_plants() {
    check_tampered_file(true);
    check_gitlink(true);
}

negative_control!(
    git_guard_emptying_the_index_clears_both_plants,
    "the index kept, with the tampered file",
    expected = "git_guard: a tampered file survived the checkout",
    check_tampered_file(false)
);

mod gitlink_kept {
    use super::*;
    negative_control!(
        git_guard_emptying_the_index_clears_both_plants,
        "the index kept, with the gitlink",
        expected = "git_guard: a file planted under a gitlink survived the checkout",
        check_gitlink(false)
    );
}

/// The script's own `fsck` line passes a kept shallow clone and refuses a blob overwritten under its own name.
fn check_git_objects(script: &str) {
    let base = scratch("fsck");
    let (origin, dir) = (base.join("origin"), base.join("kept"));
    origin_repo(&origin);
    kept_repo(&origin, &dir);
    let fsck = || script_git(script, "objects", &dir.join(".git"), &base);
    assert!(
        fsck().status.success(),
        "git_guard: fsck refused a kept shallow clone"
    );
    overwrite_blob(&dir);
    assert!(
        !fsck().status.success(),
        "git_guard: fsck passed an overwritten blob"
    );
}

#[test]
fn git_guard_finds_a_planted_object() {
    check_git_objects(&read(GIT_GUARD));
}

negative_control!(
    git_guard_finds_a_planted_object,
    "connectivity only, which doesn't hash a blob",
    expected = "git_guard: fsck passed an overwritten blob",
    check_git_objects(&read(GIT_GUARD).replace("--no-dangling", "--connectivity-only"))
);

mod fsck_refuses_all {
    use super::*;
    negative_control!(
        git_guard_finds_a_planted_object,
        "a line that refuses every repo",
        expected = "git_guard: fsck refused a kept shallow clone",
        check_git_objects(&read(GIT_GUARD).replace("--no-dangling", "--no-such-flag"))
    );
}

/// `script` run on each case as the job's step would run it, with `powershell`, which only Windows has. A key of
/// [`PLANTED`] in a `.git/config` of its own, and each other thing the owner listed, is refused; a clean kept checkout and
/// a first run pass.
// Compiled everywhere so a mistake here shows on the Mac, not first on the PC; called only under `cfg(windows)`.
#[cfg_attr(not(windows), allow(dead_code))]
fn check_script_refuses(script: &Path) {
    const CLEAN: &str = "[core]\n\trepositoryformatversion = 0\n\tfilemode = false\n\tbare = false\n\tlogallrefupdates = true\n\tsymlinks = false\n\tignorecase = true\n[remote \"origin\"]\n\turl = https://github.com/Ma1achy/Principia\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n[gc]\n\tauto = 0\n";
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "ci_self_hosted_guard_{}_{}",
        std::process::id(),
        script.file_name().expect("a name").to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let origin = base.join("origin");
    origin_repo(&origin);
    // A real repo for each case that gets past the rest, or a failed fsck would hide what was meant to refuse.
    let plant = |case: &str, repo: bool, files: &[(&str, &str)]| {
        let dir = base.join(case);
        if repo {
            kept_repo(&origin, &dir);
        }
        std::fs::create_dir_all(&dir).expect("a case's folder");
        for (name, text) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
            std::fs::write(path, text).expect("a planted file");
        }
        dir
    };
    let run = |dir: &Path| {
        let out = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .arg("check")
            .arg(dir)
            .timed_output()
            .expect("powershell ran");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let passes = |case: &str, files: &[(&str, &str)]| run(&plant(case, true, files)).0;
    let config = |text: &str| format!("{CLEAN}{text}");
    for (at, key) in PLANTED.iter().enumerate() {
        let (section, name) = key.split_once('.').expect("a section");
        let (sub, name) = name.rsplit_once('.').unwrap_or(("", name));
        let header = if sub.is_empty() {
            format!("[{section}]\n")
        } else {
            format!("[{section} \"{sub}\"]\n")
        };
        let text = config(&format!("{header}\t{name} = x\n"));
        assert!(
            !passes(&format!("key_{at}"), &[(".git/config", &text)]),
            "git_guard: the script passed {key}"
        );
    }
    let clean = [
        (".git/config", CLEAN),
        (".git/hooks/pre-commit.sample", "#\n"),
    ];
    assert!(
        run(&plant("first_run", false, &[])).0,
        "git_guard: the script refused a first run, or the folder a refusal leaves"
    );
    let sparse = [
        (".git/config", CLEAN),
        (".git/config.worktree", "[core]\n\tsparseCheckout = false\n"),
    ];
    assert!(
        passes("sparse", &sparse),
        "git_guard: the script refused a sparse config.worktree"
    );
    assert!(
        passes("clean", &clean),
        "git_guard: the script refused a clean checkout"
    );
    assert!(
        base.join("clean/.git/index").exists(),
        "git_guard: the check form deleted the index"
    );
    let unlisted = config("[some]\n\tfuture = x\n");
    let protocol = config("[protocol \"ext\"]\n\tallow = always\n");
    let gc_hook = config("[gc]\n\trecentObjectsHook = x\n");
    let trust = config("[core]\n\ttrustctime = false\n");
    let repeated = config("[gc]\n\tauto = 0\n");
    let two_urls = format!("[remote \"origin\"]\n\turl = ext::x\n{CLEAN}");
    let refspec = CLEAN.replace(
        "+refs/heads/*:refs/remotes/origin/*",
        "+abc:refs/replace/def",
    );
    let with_git = CLEAN.replace("Ma1achy/Principia", "Ma1achy/Principia.git");
    let elsewhere = CLEAN.replace("Ma1achy/Principia", "other/Principia");
    // A case, what the script must say of it, and what's planted.
    type Refused<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);
    let refused: [Refused; 28] = [
        (
            "a planted refspec",
            "config value off the allowlist: remote.origin.fetch",
            &[(".git/config", refspec.as_str())],
        ),
        (
            "HEAD on another branch",
            "HEAD:",
            &[
                (".git/config", CLEAN),
                (".git/HEAD", "ref: refs/heads/other\n"),
            ],
        ),
        (
            "HEAD through a path",
            "HEAD:",
            &[
                (".git/config", CLEAN),
                (".git/HEAD", "ref: refs/heads/../x\n"),
            ],
        ),
        (
            "HEAD as a short id",
            "HEAD:",
            &[(".git/config", CLEAN), (".git/HEAD", "abc123\n")],
        ),
        (
            "HEAD in capitals",
            "HEAD:",
            &[
                (".git/config", CLEAN),
                (".git/HEAD", "REF: refs/heads/main\n"),
            ],
        ),
        (
            "a key nobody listed",
            "off the allowlist: some.future",
            &[(".git/config", unlisted.as_str())],
        ),
        (
            "protocol.ext.allow",
            "off the allowlist: protocol.ext.allow",
            &[(".git/config", protocol.as_str())],
        ),
        (
            "gc.recentObjectsHook",
            "off the allowlist: gc.recentobjectshook",
            &[(".git/config", gc_hook.as_str())],
        ),
        (
            "core.trustctime",
            "off the allowlist: core.trustctime",
            &[(".git/config", trust.as_str())],
        ),
        (
            "a repeated key",
            "repeated config key: gc.auto",
            &[(".git/config", repeated.as_str())],
        ),
        (
            "a first URL",
            "repeated config key: remote.origin.url",
            &[(".git/config", two_urls.as_str())],
        ),
        (
            "a URL with .git",
            "remote.origin.url isn't this repo",
            &[(".git/config", with_git.as_str())],
        ),
        (
            "another URL",
            "remote.origin.url isn't this repo",
            &[(".git/config", elsewhere.as_str())],
        ),
        (
            "commondir",
            "redirect: commondir",
            &[(".git/config", CLEAN), (".git/commondir", "planted\n")],
        ),
        (
            "gitdir",
            "redirect: gitdir",
            &[(".git/config", CLEAN), (".git/gitdir", "planted\n")],
        ),
        (
            "reftable",
            ".git/reftable",
            &[(".git/config", CLEAN), (".git/reftable/tables.list", "x\n")],
        ),
        (
            "grafts",
            "info/grafts",
            &[(".git/config", CLEAN), (".git/info/grafts", "x\n")],
        ),
        (
            "alternates",
            "objects/info/alternates",
            &[
                (".git/config", CLEAN),
                (".git/objects/info/alternates", "x\n"),
            ],
        ),
        (
            "a replace ref",
            "refs/replace/",
            &[(".git/config", CLEAN), (".git/refs/replace/a", "x\n")],
        ),
        (
            "a packed replace ref",
            "in packed-refs",
            &[
                (".git/config", CLEAN),
                (".git/packed-refs", "abc refs/replace/a\n"),
            ],
        ),
        (
            "a continuation",
            "config key",
            &[(
                ".git/config",
                "[core]\n\tx = a\\\n[other]\n\thooksPath = /evil\n",
            )],
        ),
        (
            "a bare CR",
            "config key",
            &[(
                ".git/config",
                "[core]\n\tx = a\r[other]\n\thooksPath = /evil\n",
            )],
        ),
        (
            "an empty subsection",
            "config key",
            &[(".git/config", "[url \"\"]\n\tinsteadOf = x\n")],
        ),
        (
            "hook",
            "active hook",
            &[(".git/config", CLEAN), (".git/hooks/pre-commit", "x")],
        ),
        (
            "attributes",
            "info/attributes",
            &[(".git/config", CLEAN), (".git/info/attributes", "x")],
        ),
        (
            "an unreadable line",
            "can't read",
            &[(".git/config", "[core\n\tbare = false\n")],
        ),
        (
            "a key on the section's line",
            "config key",
            &[(".git/config", "[core] hooksPath = x\n")],
        ),
        (
            "a key in config.worktree",
            "config key",
            &[
                (".git/config", CLEAN),
                (".git/config.worktree", "[core]\n\tpager = x\n"),
            ],
        ),
    ];
    let refuses = |case: &str, reason: &str, dir: &Path| {
        let (passed, said) = run(dir);
        assert!(!passed, "git_guard: the script passed {case}");
        assert!(
            said.contains(reason),
            "git_guard: the script passed {case}, and something else refused it: {said}"
        );
    };
    for (at, (case, reason, files)) in refused.iter().enumerate() {
        refuses(case, reason, &plant(&format!("refused_{at}"), true, files));
    }
    refuses(
        ".git as a file",
        ".git is a file",
        &plant("gitfile", false, &[(".git", "gitdir: elsewhere\n")]),
    );
    refuses(
        "no config",
        "can't read",
        &plant("no_config", false, &[(".git/HEAD", "x")]),
    );
    let text = std::fs::read_to_string(script).expect("the script");
    for (at, wrong) in WRONG_VALUES.iter().enumerate() {
        let config = config_text(&text, Some(wrong));
        let dir = plant(
            &format!("wrong_{at}"),
            true,
            &[(".git/config", config.as_str())],
        );
        refuses(wrong, "off the allowlist", &dir);
    }
    let whole = config_text(&text, None);
    assert!(
        passes("whole", &[(".git/config", whole.as_str())]),
        "git_guard: the script refused its own list"
    );
    let detached = plant("detached", true, &[(".git/config", CLEAN)]);
    let id = git_ok(&detached, &["rev-parse", "HEAD"]);
    std::fs::write(detached.join(".git/HEAD"), format!("{id}\n")).expect("HEAD");
    assert!(
        run(&detached).0,
        "git_guard: the script refused a detached HEAD"
    );
    let overwritten = plant("overwritten", true, &[(".git/config", CLEAN)]);
    overwrite_blob(&overwritten);
    refuses("an overwritten blob", "fsck", &overwritten);
    // A firing, as the job's step runs it, standing in the folder as the runner does, with a log of this test's own.
    let dir = base.join("firing");
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("a folder");
    std::fs::write(dir.join(".git/config"), CLEAN).expect("a file");
    std::fs::write(dir.join(".git/hooks/pre-commit"), "x").expect("a file");
    std::fs::write(dir.join("kept.txt"), "x").expect("a file");
    // A planted name that a shell would run, and a junction whose target must stay.
    let marker = base.join("ran.txt");
    std::fs::write(
        dir.join("z.cmd"),
        format!("@echo x > \"{}\"\r\n", marker.display()),
    )
    .expect("a file");
    std::fs::write(dir.join("a&z.cmd"), "x").expect("a file");
    let target = base.join("outside");
    std::fs::create_dir_all(&target).expect("a folder");
    std::fs::write(target.join("kept.txt"), "x").expect("a file");
    let made = std::process::Command::new("cmd")
        .arg("/c")
        .arg("mklink")
        .arg("/J")
        .arg(dir.join("link"))
        .arg(&target)
        .timed_output()
        .expect("cmd ran");
    assert!(made.status.success(), "git_guard: no junction to plant");
    // [AppendOnly]::Add opens with FILE_OPEN, so it creates nothing, and prefixes `\\??\\` to the path as it stands: a full
    // path, in backslashes, of a file that exists.
    let log = PathBuf::from(
        base.join("refusals.log")
            .to_string_lossy()
            .replace('/', "\\"),
    );
    std::fs::write(&log, "").expect("the log");
    let job = |dir: &Path| {
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .current_dir(dir)
            .env("GITHUB_WORKSPACE", dir)
            .env("WIN_REFUSAL_LOG", &log)
            .timed_output()
            .expect("powershell ran")
    };
    let out = job(&dir);
    assert!(
        !out.status.success(),
        "git_guard: the firing passed the job"
    );
    let left = std::fs::read_dir(&dir).expect("the folder").count();
    assert_eq!(left, 0, "git_guard: the firing left the checkout unemptied");
    assert!(!marker.exists(), "git_guard: a planted name ran");
    assert!(
        target.join("kept.txt").exists(),
        "git_guard: the firing went through a link"
    );
    let line = std::fs::read_to_string(&log).expect("the refusal log");
    assert!(
        line.contains("active hook"),
        "git_guard: the firing logged no reason"
    );
    // Passing, as the job does: the index is emptied, and if it can't be, the job refuses.
    let kept = plant("kept", true, &[(".git/config", CLEAN)]);
    assert!(
        job(&kept).status.success(),
        "git_guard: the job refused a clean checkout"
    );
    assert!(
        kept.join(".git/index").exists() && git_ok(&kept, &["ls-files"]).is_empty(),
        "git_guard: the job left the index's entries"
    );
    let stuck = plant(
        "stuck",
        true,
        &[(".git/config", CLEAN), (".git/index.lock", "")],
    );
    assert!(
        !job(&stuck).status.success(),
        "git_guard: a failed delete passed the job"
    );
}

#[cfg(windows)]
#[test]
fn git_guard_script_refuses_what_is_planted() {
    check_script_refuses(&root().join(GIT_GUARD));
}

#[cfg(windows)]
negative_control!(
    git_guard_script_refuses_what_is_planted,
    "the script with its lists off",
    expected = "git_guard: the script passed",
    {
        let edited = Path::new(env!("CARGO_TARGET_TMPDIR")).join("git_guard_no_keys.ps1");
        let script = read(GIT_GUARD);
        let (head, tail) = script.split_once("$keys = @(\n").expect("the list");
        let (_, tail) = tail.split_once("\n)").expect("its end");
        let edited_text = format!("{head}$keys = @(){tail}").replace(
            "elseif ($allowedKeys -cnotcontains $key)",
            "elseif ($false)",
        );
        std::fs::write(&edited, edited_text).expect("the edit");
        check_script_refuses(&edited)
    }
);

// ---- The refusal log ---------------------------------------------------------------------------------------------------

fn check_refusal_log(files: &[(String, String)]) {
    for (file, _, doc, _) in self_hosted(files) {
        let path = doc["env"][REFUSAL_LOG].as_str().unwrap_or("");
        assert_eq!(
            path, REFUSAL_LOG_PATH,
            "{file}: {REFUSAL_LOG} is no path on the PC"
        );
    }
}

#[test]
fn self_hosted_refusal_log_is_a_path_on_the_pc() {
    check_refusal_log(&workflows());
}

negative_control!(
    self_hosted_refusal_log_is_a_path_on_the_pc,
    "a value that's not a path",
    expected = "win-gpu.yml: WIN_REFUSAL_LOG is no path on the PC",
    check_refusal_log(&{
        let mut files = workflows();
        let (_, text) = files
            .iter_mut()
            .find(|(n, _)| n == WORKFLOW)
            .expect("the workflow");
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with(REFUSAL_LOG))
            .expect("the value")
            .to_owned();
        *text = text.replace(&line, "  WIN_REFUSAL_LOG: pending");
        files
    })
);

mod refusal_log_relative {
    use super::*;
    negative_control!(
        self_hosted_refusal_log_is_a_path_on_the_pc,
        "a path that isn't full",
        expected = "win-gpu.yml: WIN_REFUSAL_LOG is no path on the PC",
        check_refusal_log(&{
            let mut files = workflows();
            let (_, text) = files
                .iter_mut()
                .find(|(n, _)| n == WORKFLOW)
                .expect("the workflow");
            *text = text.replace(REFUSAL_LOG_PATH, "refusal-log\\refusals.log");
            files
        })
    );
}

// ---- The paths filter (R-711 item 5) -----------------------------------------------------------------------------------

/// Whether GitHub's filter takes `path`: the last pattern to match it decides. Only the two forms the workflow writes are
/// read: a file, and a folder with `/**`.
fn filter_takes(patterns: &[&str], path: &str) -> bool {
    let mut taken = false;
    for pattern in patterns {
        let (keep, pattern) = pattern
            .strip_prefix('!')
            .map_or((true, *pattern), |p| (false, p));
        assert!(
            !pattern.replace("/**", "").contains('*'),
            "a pattern this test can't read: {pattern}"
        );
        let matched = pattern
            .strip_suffix("/**")
            .map_or(path == pattern, |folder| {
                path.strip_prefix(folder)
                    .is_some_and(|p| p.starts_with('/'))
            });
        if matched {
            taken = keep;
        }
    }
    taken
}

fn check_paths(files: &[(String, String)]) {
    for (file, _, doc, _) in self_hosted(files) {
        let push = &doc["on"]["push"];
        assert!(
            push["paths-ignore"].is_badvalue(),
            "{file} filters by paths-ignore, which runs a path nobody listed"
        );
        let patterns = list(&push["paths"]);
        assert!(!patterns.is_empty(), "{file} has no paths filter");
        for path in [
            "docs/read_first/principia_INDEX.md",
            "decisions.md",
            "REVIEW_QUEUE.md",
            "plan/WORKFLOW.md",
            "workbench/a.py",
            "crates/gui/src/lib.rs",
            "crates/prin/src/main.rs",
        ] {
            assert!(!filter_takes(&patterns, path), "{file} runs on {path}");
        }
        for path in [
            "crates/kernel/src/lib.rs",
            "crates/engine/src/lib.rs",
            "xtask/src/golden.rs",
            "fixtures/golden/BASELINES.md",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            ".config/nextest.toml",
            ".github/workflows/win-gpu.yml",
            ".github/scripts/win_measure.ps1",
            ".github/scripts/git_guard.ps1",
        ] {
            assert!(
                filter_takes(&patterns, path),
                "{file} doesn't run on {path}"
            );
        }
    }
}

#[test]
fn self_hosted_paths_skip_docs_only_pushes() {
    check_paths(&workflows());
}

negative_control!(
    self_hosted_paths_skip_docs_only_pushes,
    "no filter",
    expected = "win-gpu.yml has no paths filter",
    check_paths(&measured("    paths:\n", "    unrelated:\n"))
);

mod docs {
    use super::*;
    negative_control!(
        self_hosted_paths_skip_docs_only_pushes,
        "docs in the filter",
        expected = "win-gpu.yml runs on docs/read_first/principia_INDEX.md",
        check_paths(&measured(
            "      - \"xtask/**\"\n",
            "      - \"xtask/**\"\n      - \"docs/**\"\n"
        ))
    );
}

mod gui {
    use super::*;
    negative_control!(
        self_hosted_paths_skip_docs_only_pushes,
        "the crates no GPU job builds back in the filter",
        expected = "win-gpu.yml runs on crates/gui/src/lib.rs",
        check_paths(&measured("      - \"!crates/gui/**\"\n", ""))
    );
}

mod ignored {
    use super::*;
    negative_control!(
        self_hosted_paths_skip_docs_only_pushes,
        "paths-ignore in place of paths",
        expected = "win-gpu.yml filters by paths-ignore",
        check_paths(&measured("    paths:\n", "    paths-ignore:\n"))
    );
}

mod missing_gpu_path {
    use super::*;
    negative_control!(
        self_hosted_paths_skip_docs_only_pushes,
        "a GPU path left out",
        expected = "win-gpu.yml doesn't run on xtask/src/golden.rs",
        check_paths(&measured("      - \"xtask/**\"\n", ""))
    );
}

mod own_file {
    use super::*;
    negative_control!(
        self_hosted_paths_skip_docs_only_pushes,
        "a change to this workflow left out",
        expected = "win-gpu.yml doesn't run on .github/workflows/win-gpu.yml",
        check_paths(&measured("      - \".github/workflows/win-gpu.yml\"\n", ""))
    );
}

mod wide_pattern {
    use super::*;
    negative_control!(
        self_hosted_paths_skip_docs_only_pushes,
        "a pattern the test can't read",
        expected = "a pattern this test can't read: *.md",
        check_paths(&measured(
            "      - \"xtask/**\"\n",
            "      - \"*.md\"\n      - \"xtask/**\"\n"
        ))
    );
}
