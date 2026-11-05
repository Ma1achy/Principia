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
//! starts no process but Windows' own PowerShell. Parsing, never scanning text: a trigger in `on: [push, pull_request]` is
//! the same trigger.
//!
//! The same goes for `.github/scripts/git_guard.ps1` (the owner, 5 Nov 2026), with a limit: off Windows there is no
//! PowerShell, so a test there reads the script's key list and its shape, and only `git_guard_script_refuses_what_is_planted`
//! runs it, on the PC, each night.
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
const PLANTED: [&str; 35] = [
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
];
/// What actions/checkout leaves in a kept `.git/config`, and near misses of the list.
const HARMLESS: [&str; 16] = [
    "core.repositoryformatversion",
    "core.filemode",
    "core.bare",
    "core.logallrefupdates",
    "core.symlinks",
    "core.ignorecase",
    "core.autocrlf",
    "remote.origin.url",
    "remote.origin.fetch",
    "gc.auto",
    "http.https://github.com/.extraheader",
    "filter.lfs.required",
    "diff.x.binary",
    "core.sparsecheckout",
    "core.sparsecheckoutcone",
    "extensions.worktreeconfig",
];

/// The script's `$keys`, its one list.
fn listed_keys(script: &str) -> Vec<&str> {
    let list = script
        .split_once("$keys = @(\n")
        .and_then(|(_, rest)| rest.split_once("\n)"))
        .expect("git_guard: no $keys list")
        .0;
    list.lines().map(|l| l.trim().trim_matches('\'')).collect()
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
    for key in HARMLESS {
        assert!(
            !listed.iter().any(|p| like(key, p)),
            "git_guard: {key} is refused"
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

mod key_push_instead_of {
    use super::*;
    negative_control!(
        git_guard_refuses_each_listed_key,
        "url.*.pushInsteadOf off the list",
        expected = "git_guard: url.https://x/.PUSHINSTEADOF isn't refused",
        check_keys(&read(GIT_GUARD).replace("    'url.*.pushInsteadOf'\n", ""))
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
    assert!(
        has("[System.IO.File]::ReadAllLines($path)") && !has("git config"),
        "git_guard: doesn't read the config as a file"
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
            && has("cmd /c rmdir /s /q \"$($item.FullName)\"")
            && has("cmd /c del /f /q /a \"$($item.FullName)\""),
        "git_guard: doesn't empty the checkout"
    );
    assert!(
        has("$_.Name -eq 'config.worktree'"),
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

mod through_git {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "the config read through git, which can run what it reads",
        expected = "git_guard: doesn't read the config as a file",
        check_guard_script(&guard_edited(
            "[System.IO.File]::ReadAllLines($path)",
            "(git config --list --file $path)"
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
            "cmd /c rmdir /s /q \"$($item.FullName)\"",
            "$null"
        ))
    );
}

mod worktree_config_skipped {
    use super::*;
    negative_control!(
        git_guard_script_holds_the_owners_order,
        "config.worktree left unread",
        expected = "git_guard: doesn't check config.worktree's keys",
        check_guard_script(&guard_edited("$_.Name -eq 'config.worktree'", "$false"))
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
    let passes = |case: &str, files: &[(&str, &str)]| {
        let dir = base.join(case);
        std::fs::create_dir_all(&dir).expect("a case's folder");
        for (name, text) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
            std::fs::write(path, text).expect("a planted file");
        }
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
            .arg(&dir)
            .timed_output()
            .expect("powershell ran");
        out.status.success()
    };
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
        passes("first_run", &[]),
        "git_guard: the script refused a first run, or the folder a refusal leaves"
    );
    let sparse = [
        (".git/config", CLEAN),
        (".git/config.worktree", "[core]\n\tsparseCheckout = true\n"),
    ];
    assert!(
        passes("sparse", &sparse),
        "git_guard: the script refused a sparse config.worktree"
    );
    assert!(
        passes("clean", &clean),
        "git_guard: the script refused a clean checkout"
    );
    let refused: [(&str, &[(&str, &str)]); 7] = [
        (
            "hook",
            &[(".git/config", CLEAN), (".git/hooks/pre-commit", "x")],
        ),
        (
            "attributes",
            &[(".git/config", CLEAN), (".git/info/attributes", "x")],
        ),
        (".git as a file", &[(".git", "gitdir: elsewhere\n")]),
        ("no config", &[(".git/HEAD", "x")]),
        (
            "an unreadable line",
            &[(".git/config", "[core\n\tbare = false\n")],
        ),
        (
            "a key on the section's line",
            &[(".git/config", "[core] hooksPath = x\n")],
        ),
        (
            "a key in config.worktree",
            &[
                (".git/config", CLEAN),
                (".git/config.worktree", "[core]\n\tpager = x\n"),
            ],
        ),
    ];
    for (at, (case, files)) in refused.iter().enumerate() {
        assert!(
            !passes(&format!("refused_{at}"), files),
            "git_guard: the script passed {case}"
        );
    }
    // A firing, as the job's step runs it, standing in the folder as the runner does, with a log of this test's own.
    let dir = base.join("firing");
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("a folder");
    std::fs::write(dir.join(".git/config"), CLEAN).expect("a file");
    std::fs::write(dir.join(".git/hooks/pre-commit"), "x").expect("a file");
    std::fs::write(dir.join("kept.txt"), "x").expect("a file");
    let log = base.join("refusals.log");
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .current_dir(&dir)
        .env("GITHUB_WORKSPACE", &dir)
        .env("WIN_REFUSAL_LOG", &log)
        .timed_output()
        .expect("powershell ran");
    assert!(
        !out.status.success(),
        "git_guard: the firing passed the job"
    );
    let left = std::fs::read_dir(&dir).expect("the folder").count();
    assert_eq!(left, 0, "git_guard: the firing left the checkout unemptied");
    let line = std::fs::read_to_string(&log).expect("the refusal log");
    assert!(
        line.contains("active hook"),
        "git_guard: the firing logged no reason"
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
    "the script with its list empty",
    expected = "git_guard: the script passed",
    {
        let edited = Path::new(env!("CARGO_TARGET_TMPDIR")).join("git_guard_no_keys.ps1");
        let script = read(GIT_GUARD);
        let (head, tail) = script.split_once("$keys = @(\n").expect("the list");
        let (_, tail) = tail.split_once("\n)").expect("its end");
        std::fs::write(&edited, format!("{head}$keys = @(){tail}")).expect("the edit");
        check_script_refuses(&edited)
    }
);

// ---- The refusal log ---------------------------------------------------------------------------------------------------

fn check_refusal_log(files: &[(String, String)]) {
    for (file, _, doc, _) in self_hosted(files) {
        let path = doc["env"][REFUSAL_LOG].as_str().unwrap_or("");
        let bytes = path.as_bytes();
        assert!(
            bytes.len() > 3 && bytes[0].is_ascii_uppercase() && path[1..].starts_with(":\\"),
            "{file}: {REFUSAL_LOG} is no path on the PC: `{path}`"
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
