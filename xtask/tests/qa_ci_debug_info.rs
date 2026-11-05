//! QA tests for the CI debug-info PR (the owner's CI candidates, 4 Nov 2026, item 4: "`debug = \"line-tables-only\"`
//! (or 0) for the workspace and 0 for deps, `CARGO_INCREMENTAL=0`, `strip = \"debuginfo\"` for test binaries ...
//! confirm panic backtraces stay usable"), read against R-374 (every CI job keeps line tables, so panics keep file:line
//! backtraces), R-337 (every workflow sets the same CARGO_* values, for rust-cache's key) and R-233 (local builds are
//! unaffected). `strip` is left out: it fails R-374 (the PR's own test shows it). And against the rule the PR's history
//! earned: no test, and no control, may write the CARGO_HOME of the machine it runs on.
//!
//! The workflows are parsed as YAML (PyYAML, as xtask's plan_check tests use), never scanned as text, and everything
//! that runs does so against scratch directories:
//!
//! - every job that runs cargo, in any shell form, has the config before its first cargo command, and exactly once: the
//!   script appends, and a second `[profile...]` table is a TOML error that stops every cargo in the job;
//! - no `run:` step outside the two jobs that have always written it reaches a CARGO_HOME, on any line of its script,
//!   so a test that runs a job's `run:` steps has nothing to write there;
//! - the action, run as the runner runs it, writes one file, under CARGO_HOME or else under `$HOME/.cargo`;
//! - no level of any workflow strips, or sets a build setting apart from the top-level env;
//! - on the real workspace, by cargo's own unit graph, in `dev` and in `test`, every dependency has no debug info and
//!   every member's target keeps its line tables. `--config` carries what the script writes, because the workspace's
//!   crates need the machine's registry and so its CARGO_HOME: nothing is written there;
//! - a panic that crosses a workspace member and a non-member dependency, in `dev` and in `test`, keeps file:line for
//!   the member and has none for the dependency;
//! - the fixture pool's key, run as the workflow runs it, follows each build setting and the script's text.
//!
//! Each test has its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "controls")]
use serde_json::json;
use serde_json::{Map, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

const ACTION: &str = "./.github/actions/cargo-ci-config";
const ACTION_DIR: &str = ".github/actions/cargo-ci-config";
/// The jobs that write the config in a `run:` step of their own: `ci.yml`'s, from before the action.
const OWN_STEP_JOBS: [&str; 2] = ["ci-archive", "ci-workspace"];
/// R-374's and the PR's values, as every workflow's top-level env sets them.
const SETTINGS: [(&str, &str); 3] = [
    ("CARGO_INCREMENTAL", "0"),
    ("CARGO_PROFILE_DEV_DEBUG", "line-tables-only"),
    ("CARGO_PROFILE_TEST_DEBUG", "line-tables-only"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A number no other call in this process has, for the tests and controls that run side by side.
fn unique() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// A fresh directory for `case`, this call's alone.
fn scratch(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_ci_debug_info_{}_{}_{case}",
        std::process::id(),
        unique()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn script() -> String {
    fs::read_to_string(root().join(ACTION_DIR).join("config.sh")).expect("read config.sh")
}

/// The YAML `files`, each as JSON, by PyYAML.
fn yaml(files: &[PathBuf]) -> Vec<Value> {
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\nprint(json.dumps([yaml.safe_load(open(f)) for f in sys.argv[1:]]))",
        ])
        .args(files)
        .timed_output()
        .expect("python3 ran");
    assert!(
        out.status.success(),
        "the YAML did not parse: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    match serde_json::from_slice(&out.stdout).expect("PyYAML's JSON") {
        Value::Array(docs) => docs,
        other => panic!("not a list: {other}"),
    }
}

/// Every workflow, as (file name, document).
fn workflows() -> Vec<(String, Value)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .collect();
    paths.sort();
    let docs = yaml(&paths);
    assert!(!docs.is_empty(), "no workflows");
    paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .zip(docs)
        .collect()
}

fn jobs(doc: &Value) -> &Map<String, Value> {
    doc["jobs"].as_object().expect("a workflow's jobs")
}

fn steps(job: &Value) -> &[Value] {
    job["steps"].as_array().map_or(&[], Vec::as_slice)
}

fn run_of(step: &Value) -> &str {
    step["run"].as_str().unwrap_or("")
}

/// A step's script, without its comment lines.
fn commands(step: &Value) -> Vec<&str> {
    run_of(step)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// Whether the word `word` stands at a place a shell would run it: not printed by `echo` or `printf`, not part of
/// another word or a path, not in a string, `$(word`, `;word`, `(word`, `&& word` all counting.
fn runs_command(step: &Value, word: &str) -> bool {
    commands(step).iter().any(|l| {
        !(l.starts_with("echo") || l.starts_with("printf"))
            && l.match_indices(&format!("{word} ")).any(|(at, _)| {
                l[..at].chars().next_back().is_none_or(|c| {
                    !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '"' | '\'' | '`'))
                })
            })
    })
}

/// Whether the step writes the dependencies' config: the action, or a script that appends to a CARGO_HOME config.
fn writes_config(step: &Value) -> bool {
    step["uses"].as_str() == Some(ACTION)
        || commands(step)
            .iter()
            .any(|l| l.contains(">>") && l.contains("CARGO_HOME") && l.contains("config.toml"))
}

/// For every job, the config comes before its first cargo command, and any action after its checkout.
fn check_order(files: &[(String, Value)]) {
    let mut checked = Vec::new();
    for (name, doc) in files {
        for (id, job) in jobs(doc) {
            // The self-hosted runner's CARGO_HOME is read-only to its jobs and outlives them (R-711, which lands first
            // in its own rulings PR), and `check_once` refuses the config there.
            if job["runs-on"][0] == "self-hosted" {
                continue;
            }
            let steps = steps(job);
            let Some(first) = steps.iter().position(|s| runs_command(s, "cargo")) else {
                continue;
            };
            checked.push(format!("{name}/{id}"));
            let write = steps
                .iter()
                .position(writes_config)
                .unwrap_or_else(|| panic!("{name}/{id} runs cargo and writes no config"));
            assert!(
                write < first,
                "{name}/{id} runs cargo before it writes the config"
            );
            if steps[write]["uses"].is_string() {
                let checkout = steps.iter().position(|s| {
                    s["uses"]
                        .as_str()
                        .is_some_and(|u| u.starts_with("actions/checkout@"))
                });
                assert!(
                    checkout.is_some_and(|c| c < write),
                    "{name}/{id} uses the action before a checkout brings it"
                );
            }
        }
    }
    for must in [
        "ci.yml/xtask-ci",
        "ci.yml/ci",
        "gate.yml/cpu",
        "nightly.yml/mutants",
    ] {
        assert!(
            checked.iter().any(|c| c == must),
            "the check never reached {must}: {checked:?}"
        );
    }
}

#[test]
fn qa_the_config_comes_before_every_cargo_command() {
    check_order(&workflows());
}

/// The workflows with `edit` applied to job `id` of `file`.
#[cfg(feature = "controls")]
fn edited(file: &str, id: &str, edit: impl Fn(&mut Value)) -> Vec<(String, Value)> {
    let mut files = workflows();
    let doc = &mut files.iter_mut().find(|(f, _)| f == file).expect(file).1;
    edit(&mut doc["jobs"][id]);
    files
}

/// `step` put at job's steps' index `at`.
#[cfg(feature = "controls")]
fn insert(job: &mut Value, at: usize, step: Value) {
    job["steps"].as_array_mut().unwrap().insert(at, step);
}

negative_control!(
    qa_the_config_comes_before_every_cargo_command,
    "a cargo command in a command substitution, before the config",
    expected = "gate.yml/cpu runs cargo before it writes the config",
    check_order(&edited("gate.yml", "cpu", |job| {
        insert(job, 1, json!({"run": "v=$(cargo --version)\necho \"$v\""}));
    }))
);

#[cfg(feature = "controls")]
mod no_config {
    use super::*;

    negative_control!(
        qa_the_config_comes_before_every_cargo_command,
        "a job that runs cargo with its config step gone",
        expected = "pr-check.yml/pr-check runs cargo and writes no config",
        check_order(&edited("pr-check.yml", "pr-check", |job| {
            job["steps"]
                .as_array_mut()
                .unwrap()
                .retain(|s| !writes_config(s));
        }))
    );
}

/// No job writes the config twice, for the script appends and a second table of the same name is a TOML error that
/// stops every cargo in the job; and no job that writes it runs where a CARGO_HOME outlives the job, for the same
/// reason.
fn check_once(files: &[(String, Value)]) {
    for (name, doc) in files {
        for (id, job) in jobs(doc) {
            let writes = steps(job).iter().filter(|s| writes_config(s)).count();
            assert!(writes <= 1, "{name}/{id} writes the config {writes} times");
            let hosted = job["runs-on"]
                .as_str()
                .is_some_and(|r| r.starts_with("ubuntu-") || r.starts_with("macos-"));
            assert!(
                writes == 0 || hosted,
                "{name}/{id} writes the config on a runner that may keep its CARGO_HOME: {}",
                job["runs-on"]
            );
        }
    }
}

#[test]
fn qa_each_job_writes_the_config_once() {
    check_once(&workflows());
}

negative_control!(
    qa_each_job_writes_the_config_once,
    "a job with the action twice",
    expected = "gate.yml/cpu writes the config 2 times",
    check_once(&edited("gate.yml", "cpu", |job| {
        insert(job, 2, json!({"uses": ACTION}));
    }))
);

#[cfg(feature = "controls")]
mod self_hosted {
    use super::*;

    negative_control!(
        qa_each_job_writes_the_config_once,
        "a job on a self-hosted runner, whose config would hold the last run's tables",
        expected = "gate.yml/cpu writes the config on a runner that may keep its CARGO_HOME",
        check_once(&edited("gate.yml", "cpu", |job| {
            job["runs-on"] = json!(["self-hosted", "macOS"]);
        }))
    );
}

/// No `run:` step reaches a CARGO_HOME, but for [`OWN_STEP_JOBS`]' own and the line that hashes the script into the
/// pool's key: a test that runs a job's `run:` steps outside CI (`mutants_no_mutant`, `qa_TASK-M0-23_r305`,
/// `qa_mutants_check_always`) passes over the action, so it has nothing to write to that machine's CARGO_HOME.
fn check_no_run_step_reaches(files: &[(String, Value)]) {
    for (name, doc) in files {
        for (id, job) in jobs(doc) {
            if name == "ci.yml" && OWN_STEP_JOBS.contains(&id.as_str()) {
                continue;
            }
            for step in steps(job) {
                for l in commands(step) {
                    if l.contains("echo \"cachekey=") {
                        continue;
                    }
                    assert!(
                        !["CARGO_HOME", ".cargo/config", "cargo-ci-config", "config.sh"]
                            .iter()
                            .any(|n| l.contains(n)),
                        "{name}/{id} reaches a CARGO_HOME in a `run:` step, which a test runs: `{l}`"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_no_run_step_reaches_a_cargo_home() {
    check_no_run_step_reaches(&workflows());
}

negative_control!(
    qa_no_run_step_reaches_a_cargo_home,
    "the script run from the second line of a multi-line `run:` step, the first design's slip",
    expected = "gate.yml/cpu reaches a CARGO_HOME in a `run:` step",
    check_no_run_step_reaches(&edited("gate.yml", "cpu", |job| {
        insert(
            job,
            2,
            json!({"run": "set -e\nbash .github/actions/cargo-ci-config/config.sh\n"}),
        );
    }))
);

#[cfg(feature = "controls")]
mod inline_write {
    use super::*;

    negative_control!(
        qa_no_run_step_reaches_a_cargo_home,
        "the config written in place, with the script out of it",
        expected = "mutants.yml/mutants reaches a CARGO_HOME in a `run:` step",
        check_no_run_step_reaches(&edited("mutants.yml", "mutants", |job| {
            insert(
                job,
                2,
                json!({"run": "printf 'x' >> ~/.cargo/config.toml\n"}),
            );
        }))
    );
}

/// Every file under `dir`, relative to it, sorted.
fn files_under(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push(p.strip_prefix(dir).unwrap().display().to_string());
            }
        }
    }
    out.sort();
    out
}

/// What the action's one step leaves in a scratch tree, run as a runner runs it, with `script` for `config.sh`,
/// `$HOME` always in the scratch tree, and `CARGO_HOME` set there if `set_home`, unset otherwise: nothing here can
/// reach the machine's own.
fn action_leaves(script: &str, set_home: bool, case: &str) -> Vec<String> {
    let base = scratch(case);
    let action = base.join("action");
    fs::create_dir_all(&action).unwrap();
    fs::write(action.join("config.sh"), script).unwrap();
    let tree = base.join("tree");
    let cwd = tree.join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    let doc = yaml(&[root().join(ACTION_DIR).join("action.yml")]).remove(0);
    assert_eq!(doc["runs"]["using"], "composite", "not a composite action");
    let steps = doc["runs"]["steps"].as_array().expect("the action's steps");
    assert_eq!(steps.len(), 1, "the action does more than run its script");
    assert_eq!(
        steps[0]["shell"], "bash",
        "a composite step names its shell"
    );
    let mut command = Command::new("bash");
    command
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c"])
        .arg(run_of(&steps[0]))
        .current_dir(&cwd)
        .env("GITHUB_ACTION_PATH", &action)
        .env("HOME", tree.join("home"))
        .env_remove("CARGO_HOME");
    if set_home {
        command.env("CARGO_HOME", tree.join("cargo_home"));
    }
    let out = command.timed_output().expect("run the action's step");
    assert!(
        out.status.success(),
        "the action's step failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    files_under(&tree)
}

fn check_action_writes(script: &str, case: &str) {
    assert_eq!(
        action_leaves(script, true, &format!("{case}_set")),
        ["cargo_home/config.toml"],
        "the action wrote elsewhere than the CARGO_HOME it was given"
    );
    assert_eq!(
        action_leaves(script, false, &format!("{case}_unset")),
        ["home/.cargo/config.toml"],
        "the action wrote elsewhere than $HOME/.cargo with no CARGO_HOME"
    );
}

#[test]
fn qa_the_action_writes_one_file_in_cargo_home() {
    check_action_writes(&script(), "writes");
}

negative_control!(
    qa_the_action_writes_one_file_in_cargo_home,
    "a script that writes the repository's own `.cargo`, which local builds read (R-233)",
    expected = "the action wrote elsewhere than the CARGO_HOME it was given",
    check_action_writes(
        &script().replace("home=\"${CARGO_HOME:-$HOME/.cargo}\"", "home=\".cargo\""),
        "repo"
    )
);

#[cfg(feature = "controls")]
mod ignores_cargo_home {
    use super::*;

    negative_control!(
        qa_the_action_writes_one_file_in_cargo_home,
        "a script that leaves CARGO_HOME out and always writes `$HOME/.cargo`",
        expected = "the action wrote elsewhere than the CARGO_HOME it was given",
        check_action_writes(
            &script().replace("${CARGO_HOME:-$HOME/.cargo}", "$HOME/.cargo"),
            "home"
        )
    );
}

/// Each `env:` of the workflow, with where it is.
fn envs(name: &str, doc: &Value) -> Vec<(String, bool, Map<String, Value>)> {
    let mut out = Vec::new();
    let mut add = |at: String, top: bool, env: &Value| {
        if let Some(env) = env.as_object() {
            out.push((at, top, env.clone()));
        }
    };
    add(name.to_owned(), true, &doc["env"]);
    for (id, job) in jobs(doc) {
        add(format!("{name}/{id}"), false, &job["env"]);
        for step in steps(job) {
            add(format!("{name}/{id}"), false, &step["env"]);
        }
    }
    out
}

/// No level of any workflow strips or changes the debug info; the settings are in the top-level env and only there;
/// no step's script sets them or runs `strip`.
fn check_no_strip(files: &[(String, Value)]) {
    for (name, doc) in files {
        for (at, top, env) in envs(name, doc) {
            for (key, value) in &env {
                let value = value.to_string();
                let squeezed = value.replace(' ', "");
                assert!(
                    !key.contains("STRIP") && !squeezed.contains("strip="),
                    "{at} strips, which takes R-374's line tables: {key}: {value}"
                );
                assert!(
                    !squeezed.contains("debuginfo="),
                    "{at} sets debug info in flags: {key}: {value}"
                );
                let setting = key == "CARGO_HOME"
                    || key == "CARGO_BUILD_INCREMENTAL"
                    || (key.starts_with("CARGO_")
                        && (key.contains("INCREMENTAL") || key.starts_with("CARGO_PROFILE_")));
                assert!(
                    top && key != "CARGO_HOME" || !setting,
                    "{at} sets {key} outside the top-level env's settings, apart from rust-cache's key (R-337)"
                );
            }
        }
        let top: &Value = &doc["env"];
        for (key, value) in SETTINGS {
            assert_eq!(top[key], value, "{name} does not set {key} to {value}");
        }
        for (id, job) in jobs(doc) {
            for step in steps(job) {
                assert!(
                    !runs_command(step, "strip"),
                    "{name}/{id} strips a binary, which takes R-374's line tables"
                );
                for l in commands(step) {
                    assert!(
                        !l.contains("CARGO_INCREMENTAL=")
                            && !l.contains("CARGO_PROFILE_")
                            && !l.contains("-Cdebuginfo")
                            && !l.contains("-C debuginfo"),
                        "{name}/{id} sets a build setting in a script: `{l}`"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_no_level_of_any_workflow_strips_or_moves_the_settings() {
    check_no_strip(&workflows());
}

negative_control!(
    qa_no_level_of_any_workflow_strips_or_moves_the_settings,
    "stripping at the workflows' top level, the same in each, which rust-cache's key would not catch",
    expected = "strips, which takes R-374's line tables: CARGO_PROFILE_TEST_STRIP",
    {
        let mut files = workflows();
        for (_, doc) in &mut files {
            doc["env"]["CARGO_PROFILE_TEST_STRIP"] = json!("debuginfo");
        }
        check_no_strip(&files)
    }
);

#[cfg(feature = "controls")]
mod strip_command {
    use super::*;

    negative_control!(
        qa_no_level_of_any_workflow_strips_or_moves_the_settings,
        "a step that runs `strip` on the test binaries",
        expected = "gate.yml/cpu strips a binary",
        check_no_strip(&edited("gate.yml", "cpu", |job| {
            insert(
                job,
                2,
                json!({"run": "find target -type f -perm +111 | xargs -n1 true && strip target/debug/deps/x\n"}),
            );
        }))
    );
}

#[cfg(feature = "controls")]
mod step_env {
    use super::*;

    negative_control!(
        qa_no_level_of_any_workflow_strips_or_moves_the_settings,
        "a step that takes the line tables away for itself",
        expected =
            "gate.yml/cpu sets CARGO_PROFILE_TEST_DEBUG outside the top-level env's settings",
        check_no_strip(&edited("gate.yml", "cpu", |job| {
            insert(
                job,
                2,
                json!({"run": "true", "env": {"CARGO_PROFILE_TEST_DEBUG": "0"}}),
            );
        }))
    );
}

#[cfg(feature = "controls")]
mod moved_home {
    use super::*;

    negative_control!(
        qa_no_level_of_any_workflow_strips_or_moves_the_settings,
        "a workflow that moves CARGO_HOME from where cargo and the action look",
        expected = "sets CARGO_HOME outside the top-level env's settings",
        {
            let mut files = workflows();
            files[0].1["env"]["CARGO_HOME"] = json!("/tmp/x");
            check_no_strip(&files)
        }
    );
}

/// The config the action writes.
fn config_written(case: &str) -> String {
    let base = scratch(&format!("written_{case}"));
    let home = base.join("cargo_home");
    fs::create_dir_all(&home).unwrap();
    let path = base.join("config.sh");
    fs::write(&path, script()).unwrap();
    let out = Command::new("bash")
        .arg(&path)
        .env("HOME", &base)
        .env("CARGO_HOME", &home)
        .timed_output()
        .expect("run the script");
    assert!(out.status.success(), "the script failed");
    fs::read_to_string(home.join("config.toml")).expect("the written config")
}

/// cargo's unit graph of the real workspace, with `config` for the config, in the workflows' env, for `args`. A
/// listing: nothing builds, in a target directory of its own.
fn unit_graph(config: &str, args: &[&str], case: &str) -> Value {
    let base = scratch(case);
    let file = base.join("config.toml");
    fs::write(&file, config).unwrap();
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    command
        .current_dir(root())
        .arg("--config")
        .arg(&file)
        .args(args)
        .args([
            "-Zunstable-options",
            "--unit-graph",
            "--workspace",
            "--locked",
        ])
        .env("CARGO_TARGET_DIR", base.join("target"));
    for (key, value) in SETTINGS {
        command.env(key, value);
    }
    let out = command.timed_output().expect("cargo ran");
    assert!(
        out.status.success(),
        "cargo's unit graph failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("cargo's unit graph")
}

/// In `graph`, of profile `profile`: every unit of a dependency has no debug info, and every member's target its line
/// tables. A member's build script is outside R-374: nothing in a test's backtrace runs it.
fn check_units(graph: &Value, profile: &str) {
    let units = graph["units"].as_array().expect("the graph's units");
    let (mut members, mut dependencies) = (0, 0);
    for unit in units {
        let id = unit["pkg_id"].as_str().expect("a unit's package");
        let debug = &unit["profile"]["debuginfo"];
        assert_eq!(unit["profile"]["name"], profile, "another profile: {id}");
        let kinds = unit["target"]["kind"].as_array().expect("a target's kinds");
        if kinds.iter().any(|k| k == "custom-build") && id.starts_with("path+") {
            continue;
        }
        if id.starts_with("path+") {
            members += 1;
            assert_eq!(
                debug, "line-tables-only",
                "the member {id} lost its line tables (R-374), in {profile}: {debug}"
            );
        } else {
            dependencies += 1;
            assert_eq!(
                debug, 0,
                "the dependency {id} carries debug info, in {profile}: {debug}"
            );
        }
    }
    assert!(
        members >= 8 && dependencies > 0,
        "the graph has {members} member units, {dependencies} dependency units"
    );
}

fn check_workspace(config: &str, case: &str) {
    let test = ["test", "--all-targets", "--no-run"];
    let dev = ["build"];
    check_units(&unit_graph(config, &test, &format!("{case}_test")), "test");
    check_units(&unit_graph(config, &dev, &format!("{case}_dev")), "dev");
}

#[test]
fn qa_on_the_real_workspace_dependencies_lose_debug_info_and_members_keep_line_tables() {
    check_workspace(&config_written("workspace"), "workspace");
}

negative_control!(
    qa_on_the_real_workspace_dependencies_lose_debug_info_and_members_keep_line_tables,
    "dependencies set back to line tables, over whatever the machine's CARGO_HOME holds",
    expected = "carries debug info, in test",
    check_workspace(
        &config_written("deps").replace("debug = false", "debug = \"line-tables-only\""),
        "deps"
    )
);

#[cfg(feature = "controls")]
mod member_loses {
    use super::*;

    negative_control!(
        qa_on_the_real_workspace_dependencies_lose_debug_info_and_members_keep_line_tables,
        "a config that takes a member's line tables too",
        expected = "lost its line tables (R-374), in test",
        check_workspace(
            &format!(
                "{}[profile.test.package.validation]\ndebug = false\n",
                config_written("member")
            ),
            "member"
        )
    );
}

#[cfg(feature = "controls")]
mod dev_left_out {
    use super::*;

    negative_control!(
        qa_on_the_real_workspace_dependencies_lose_debug_info_and_members_keep_line_tables,
        "the config on `test` alone, so `cargo xtask` and `cargo run` keep the dependencies' line tables",
        expected = "carries debug info, in dev",
        check_workspace(
            &config_written("dev")
                .replace("[profile.dev.package.\"*\"]\ndebug = false", "[profile.dev.package.\"*\"]\ndebug = \"line-tables-only\""),
            "dev"
        )
    );
}

/// A fresh directory for a scratch workspace, under the system's temporary directory and with no cargo config in any
/// directory above it. cargo reads every `.cargo/config.toml` from its working directory up to the root, and those
/// outrank `CARGO_HOME`'s: on a runner the target directory sits under `/home/runner`, whose `.cargo/config.toml` is
/// the one CI writes, so a workspace there builds with that config whatever scratch `CARGO_HOME` it is given, and a
/// control that leaves the config out cannot fail (run 41769443878, xtask-ci (3) and (4)).
fn workspace_dir(case: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "qa_ci_debug_info_{}_{}_{case}",
        std::process::id(),
        unique()
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    for dir in base.ancestors() {
        for file in ["config.toml", "config"] {
            let config = dir.join(".cargo").join(file);
            assert!(
                !config.exists(),
                "{} would reach the scratch workspace",
                config.display()
            );
        }
    }
    base
}

/// What a scratch workspace prints, panicking through a member's and a non-member's frames, run by `cargo args`, with
/// `config` as its CARGO_HOME config.
fn run_scratch(config: &str, args: &[&str], case: &str) -> String {
    let base = workspace_dir(case);
    let write = |p: PathBuf, s: &str| {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    };
    let package = |name: &str, deps: &str| {
        format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}")
    };
    // The dependency sits outside the workspace's directory, so it is no member.
    write(base.join("dep/Cargo.toml"), &package("qadbg_dep", ""));
    write(
        base.join("dep/src/lib.rs"),
        "#[inline(never)]\npub fn qadbg_through(f: fn()) {\n    f();\n    std::hint::black_box(());\n}\n",
    );
    write(
        base.join("ws/Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"member\"]\nresolver = \"2\"\n",
    );
    write(
        base.join("ws/member/Cargo.toml"),
        &package("qadbg_member", ""),
    );
    write(
        base.join("ws/member/src/lib.rs"),
        "#[inline(never)]\npub fn qadbg_boom() {\n    panic!(\"qadbg boom\");\n}\n",
    );
    write(
        base.join("ws/app/Cargo.toml"),
        &package(
            "qadbg_app",
            "qadbg_dep = { path = \"../../dep\" }\nqadbg_member = { path = \"../member\" }\n",
        ),
    );
    write(
        base.join("ws/app/src/main.rs"),
        "fn main() {\n    qadbg_dep::qadbg_through(qadbg_member::qadbg_boom);\n}\n\n#[test]\nfn panics() {\n    main();\n}\n",
    );
    let home = base.join("cargo_home");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("config.toml"), config).unwrap();
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    command
        .current_dir(base.join("ws"))
        .args(args)
        .args(["--offline", "-p", "qadbg_app"])
        .env("CARGO_HOME", &home)
        .env("CARGO_TARGET_DIR", base.join("target"))
        .env("RUST_BACKTRACE", "1")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER");
    for (key, value) in SETTINGS {
        command.env(key, value);
    }
    let out = command
        .timed_output()
        .expect("run cargo on the scratch workspace");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = fs::remove_dir_all(&base);
    assert!(
        !out.status.success() && text.contains("qadbg boom"),
        "the scratch workspace did not build and panic:\n{text}"
    );
    text
}

/// The location printed under the backtrace frame of function `symbol`, which must be there. A frame with line tables
/// prints its name from them, unqualified; one without, from the symbol table, with its path.
fn location(text: &str, symbol: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| {
            l.trim_start().split_once(": ").is_some_and(|(n, s)| {
                let s = s.trim();
                n.parse::<u32>().is_ok() && (s == symbol || s.ends_with(&format!("::{symbol}")))
            })
        })
        .unwrap_or_else(|| panic!("the backtrace has no frame `{symbol}`:\n{text}"));
    lines
        .get(at + 1)
        .and_then(|l| l.trim().strip_prefix("at "))
        .map(str::to_owned)
}

/// R-374 across a member: the panic and the member's frame have file and line; the dependency's has neither.
fn check_panic(text: &str) {
    assert!(
        text.contains("panicked at member/src/lib.rs:3:5"),
        "the panic does not name its file and line:\n{text}"
    );
    let member = location(text, "qadbg_boom");
    assert!(
        member
            .as_deref()
            .is_some_and(|l| l.ends_with("member/src/lib.rs:3:5")),
        "the member's frame has no file:line (R-374): {member:?}\n{text}"
    );
    let dep = location(text, "qadbg_through");
    assert!(
        dep.is_none(),
        "the dependency's frame still has file:line: {dep:?}"
    );
}

/// [`check_panic`] in both profiles a job's cargo builds in.
fn check_both_profiles(config: &str, case: &str) {
    check_panic(&run_scratch(config, &["run"], &format!("{case}_dev")));
    check_panic(&run_scratch(config, &["test"], &format!("{case}_test")));
}

#[test]
fn qa_a_panic_through_a_member_keeps_its_file_and_line_in_both_profiles() {
    check_both_profiles(&config_written("panic"), "panic");
}

negative_control!(
    qa_a_panic_through_a_member_keeps_its_file_and_line_in_both_profiles,
    "no config, so the dependency keeps its line tables",
    expected = "the dependency's frame still has file:line",
    check_both_profiles("", "none")
);

#[cfg(feature = "controls")]
mod member_stripped {
    use super::*;

    negative_control!(
        qa_a_panic_through_a_member_keeps_its_file_and_line_in_both_profiles,
        "a config that reaches the member too",
        expected = "the member's frame has no file:line (R-374)",
        check_both_profiles(
            &format!(
                "{}[profile.dev.package.qadbg_member]\ndebug = false\n",
                config_written("reached")
            ),
            "reached"
        )
    );
}

/// The workflow line computing `cachekey` in `ci.yml`'s job `job`, from YAML.
fn key_line(job: &str) -> String {
    let files = workflows();
    let ci = &files.iter().find(|(f, _)| f == "ci.yml").expect("ci.yml").1;
    let step = steps(&ci["jobs"][job])
        .iter()
        .find(|s| s["id"] == "toolchain")
        .unwrap_or_else(|| panic!("{job} has no step `toolchain`"));
    commands(step)
        .into_iter()
        .find(|l| l.contains("echo \"cachekey="))
        .unwrap_or_else(|| panic!("{job} computes no `cachekey`"))
        .to_owned()
}

/// What `line` writes as `cachekey`, run as a runner runs it, with `rustc` a stand-in printing `compiler`, `script` the
/// config script in the checkout, and `env` the CARGO_* environment.
fn key(line: &str, compiler: &str, script: &str, env: &[(&str, &str)], case: &str) -> String {
    let base = scratch(case);
    let bin = base.join("bin");
    fs::create_dir_all(&bin).unwrap();
    write_executable(
        &bin.join("rustc"),
        format!("#!/bin/sh\necho '{compiler}'\n"),
    )
    .unwrap();
    let cwd = base.join("checkout");
    fs::create_dir_all(cwd.join(ACTION_DIR)).unwrap();
    fs::write(cwd.join(ACTION_DIR).join("config.sh"), script).unwrap();
    let output = base.join("github_output");
    let path = std::env::var("PATH").unwrap_or_default();
    let out = Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c", line])
        .current_dir(&cwd)
        .env_clear()
        .env("PATH", format!("{}:{path}", bin.display()))
        .env("GITHUB_OUTPUT", &output)
        .envs(env.iter().copied())
        .timed_output()
        .expect("run the key's line");
    assert!(
        out.status.success(),
        "the key's line failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = fs::read_to_string(&output).expect("GITHUB_OUTPUT");
    let value = written
        .trim()
        .strip_prefix("cachekey=")
        .unwrap_or_else(|| panic!("no key in `{written}`"));
    assert!(
        value.len() == 20 && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "`{value}` is not a key"
    );
    value.to_owned()
}

/// The key follows the compiler, each build setting and the script's text, and nothing else in the environment; and
/// `ci`, which saves the pool, and `xtask-ci`, which restores it, compute the one key.
fn check_key(saves: &str, restores: &str) {
    let base = [
        ("CARGO_INCREMENTAL", "0"),
        ("CARGO_PROFILE_DEV_DEBUG", "line-tables-only"),
        ("CARGO_PROFILE_TEST_DEBUG", "line-tables-only"),
        ("CARGO_TERM_COLOR", "always"),
    ];
    let with = |at: usize, value: &'static str| {
        let mut env = base;
        env[at].1 = value;
        env
    };
    let script = script();
    let k = |compiler: &str, script: &str, env: &[(&str, &str)], case: &str| {
        key(saves, compiler, script, env, case)
    };
    let usual = k("rustc 1", &script, &base, "key_usual");
    assert_eq!(
        usual,
        key(restores, "rustc 1", &script, &base, "key_restores"),
        "`ci` and `xtask-ci` compute different keys, so `xtask-ci` never restores the pool"
    );
    assert_eq!(
        usual,
        k("rustc 1", &script, &with(3, "never"), "key_colour"),
        "the key moved with a setting that changes no build"
    );
    let moved = [
        ("the compiler", k("rustc 2", &script, &base, "key_compiler")),
        (
            "the script",
            k("rustc 1", &format!("{script}# x\n"), &base, "key_script"),
        ),
        (
            "CARGO_INCREMENTAL",
            k("rustc 1", &script, &with(0, "1"), "key_incremental"),
        ),
        (
            "CARGO_PROFILE_DEV_DEBUG",
            k("rustc 1", &script, &with(1, "2"), "key_dev"),
        ),
        (
            "CARGO_PROFILE_TEST_DEBUG",
            k("rustc 1", &script, &with(2, "2"), "key_test"),
        ),
    ];
    for (what, key) in moved {
        assert_ne!(
            usual, key,
            "the key stays with {what} changed, so a pool of other builds restores and is rebuilt beside itself"
        );
    }
}

#[test]
fn qa_the_pool_key_moves_with_each_build_setting() {
    check_key(&key_line("ci"), &key_line("xtask-ci"));
}

/// `cachekey` as it was before this PR.
#[cfg(feature = "controls")]
const OLD_KEY: &str =
    "echo \"cachekey=$(rustc --version --verbose | sha256sum | cut -c1-20)\" >> \"$GITHUB_OUTPUT\"";

negative_control!(
    qa_the_pool_key_moves_with_each_build_setting,
    "`cachekey` on the compiler alone, in both jobs",
    expected = "the key stays with the script changed",
    check_key(OLD_KEY, OLD_KEY)
);

#[cfg(feature = "controls")]
mod apart {
    use super::*;

    negative_control!(
        qa_the_pool_key_moves_with_each_build_setting,
        "`xtask-ci` on the old key",
        expected = "`ci` and `xtask-ci` compute different keys",
        check_key(&key_line("ci"), OLD_KEY)
    );
}
