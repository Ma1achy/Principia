//! The Windows runner's security rules, as properties of the workflow files: a self-hosted job runs only in a
//! workflow triggered by push to main, schedule or workflow_dispatch; every self-hosted job is guarded on the
//! repository and the actor; such a workflow's token is `contents: read` and writes nothing; it reads no secrets; and
//! every action from outside the repository is pinned to a full commit SHA. Each check has a control that edits a
//! workflow to break it.

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const REPO_GUARD: &str = "github.repository == 'Ma1achy/Principia'";
const ACTOR_GUARD: &str = "github.actor == 'Ma1achy'";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// Every workflow as (file name, text).
fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p.file_name().unwrap().to_string_lossy().into_owned(), text)
        })
        .collect();
    out.sort();
    assert!(
        out.iter().any(|(n, _)| n == "win-gpu.yml"),
        "win-gpu.yml wasn't read, so there is no self-hosted job to check"
    );
    out
}

/// The workflows with `name` edited, `from` replaced by `to`; the edit must apply.
#[cfg(feature = "controls")]
fn edited(name: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut all = workflows();
    let w = all
        .iter_mut()
        .find(|(n, _)| n == name)
        .expect("the workflow");
    assert!(
        w.1.contains(from),
        "the control's edit doesn't apply to {name}"
    );
    w.1 = w.1.replacen(from, to, 1);
    all
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text).unwrap().remove(0)
}

fn is_self_hosted(job: &Yaml) -> bool {
    match &job["runs-on"] {
        Yaml::String(s) => s == "self-hosted",
        Yaml::Array(a) => a.iter().any(|l| l.as_str() == Some("self-hosted")),
        _ => false,
    }
}

/// The names of the jobs of `doc` that run on a self-hosted runner.
fn self_hosted_jobs(doc: &Yaml) -> Vec<String> {
    doc["jobs"]
        .as_hash()
        .map(|jobs| {
            jobs.iter()
                .filter(|(_, j)| is_self_hosted(j))
                .map(|(n, _)| n.as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn check_self_hosted(all: &[(String, String)]) {
    let mut seen = 0;
    for (name, text) in all {
        let doc = parse(text);
        let jobs = self_hosted_jobs(&doc);
        if jobs.is_empty() {
            continue;
        }
        // `on:` is the string key `on` in YAML 1.2; a YAML 1.1 reader would make it the boolean `true`.
        let on = if doc["on"].is_badvalue() {
            doc.as_hash()
                .and_then(|h| h.get(&Yaml::Boolean(true)))
                .unwrap_or(&doc["on"])
        } else {
            &doc["on"]
        };
        let triggers: Vec<String> = match on {
            Yaml::Hash(h) => h.keys().map(|k| k.as_str().unwrap().to_owned()).collect(),
            Yaml::String(s) => vec![s.clone()],
            Yaml::Array(a) => a.iter().map(|k| k.as_str().unwrap().to_owned()).collect(),
            _ => panic!("{name}: no triggers"),
        };
        for t in &triggers {
            assert!(
                ["push", "schedule", "workflow_dispatch"].contains(&t.as_str()),
                "{name}: a self-hosted job under the trigger `{t}`"
            );
        }
        if triggers.iter().any(|t| t == "push") {
            let branches: Vec<&str> = on["push"]["branches"]
                .as_vec()
                .map(|v| v.iter().filter_map(Yaml::as_str).collect())
                .unwrap_or_default();
            assert_eq!(
                branches,
                ["main"],
                "{name}: a self-hosted job runs on a push to a branch other than main"
            );
        }
        let perms = &doc["permissions"];
        let perms = perms
            .as_hash()
            .unwrap_or_else(|| panic!("{name}: no top-level permissions"));
        for (k, v) in perms {
            let (k, v) = (k.as_str().unwrap(), v.as_str().unwrap_or(""));
            assert!(
                k == "contents" && v == "read",
                "{name}: the token has `{k}: {v}`, where a workflow with a self-hosted job has `contents: read` alone"
            );
        }
        assert!(
            !text.contains("secrets."),
            "{name}: a workflow with a self-hosted job reads a secret"
        );
        for job in &jobs {
            let guard = doc["jobs"][job.as_str()]["if"].as_str().unwrap_or("");
            assert!(
                guard.contains(REPO_GUARD) && guard.contains(ACTOR_GUARD),
                "{name}/{job}: a self-hosted job without the repository and actor guard"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "no self-hosted job was checked");
}

#[test]
fn self_hosted_jobs_keep_the_runner_rules() {
    check_self_hosted(&workflows());
}

negative_control!(
    self_hosted_jobs_keep_the_runner_rules,
    "a pull request trigger added to the runner's workflow",
    expected = "a self-hosted job under the trigger `pull_request`",
    check_self_hosted(&edited(
        "win-gpu.yml",
        "\non:\n",
        "\non:\n  pull_request:\n"
    ))
);

mod guard {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_runner_rules,
        "the actor guard removed from a self-hosted job",
        expected = "without the repository and actor guard",
        check_self_hosted(&edited(
            "win-gpu.yml",
            "github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'",
            "github.repository == 'Ma1achy/Principia' && github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'"
        ))
    );
}

mod token {
    use super::*;
    negative_control!(
        self_hosted_jobs_keep_the_runner_rules,
        "write access given to the runner's workflow",
        expected = "where a workflow with a self-hosted job has `contents: read` alone",
        check_self_hosted(&edited(
            "win-gpu.yml",
            "permissions:\n  contents: read\n",
            "permissions:\n  contents: write\n"
        ))
    );
}

fn check_pinned(all: &[(String, String)]) {
    let mut seen = 0;
    for (name, text) in all {
        for line in text.lines() {
            let line = line.trim_start().trim_start_matches("- ");
            let Some(rest) = line.strip_prefix("uses:") else {
                continue;
            };
            let action = rest
                .split('#')
                .next()
                .unwrap()
                .trim()
                .trim_matches(['"', '\'']);
            if action.starts_with("./") {
                continue;
            }
            let (_, rev) = action
                .rsplit_once('@')
                .unwrap_or_else(|| panic!("{name}: `{action}` has no ref"));
            assert!(
                rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit()),
                "{name}: `{action}` is not pinned to a full commit SHA"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "no action was checked");
}

#[test]
fn every_action_is_pinned_to_a_commit() {
    check_pinned(&workflows());
}

negative_control!(
    every_action_is_pinned_to_a_commit,
    "an action pinned to a tag",
    expected = "is not pinned to a full commit SHA",
    check_pinned(&edited(
        "ci.yml",
        "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        "actions/checkout@v4"
    ))
);
