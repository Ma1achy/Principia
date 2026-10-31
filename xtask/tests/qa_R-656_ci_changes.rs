//! qa's tests for R-656, written from the ruling and the owner's words, not from `scripts/ci_changes.sh`'s code.
//!
//! The owner, 30 Oct 2026, on #351 (R-656): "any doc/spec/data file read by a test, build script, codegen or xtask
//! check must count as code, not docs. List those paths explicitly in the classifier and add a test that touching one
//! triggers the heavy jobs." And, of the CI throughput items (R-656): "ci.yml's `push` trigger limited to main", "a
//! per-PR `concurrency` group … never cancel runs on main", "required checks must still report", "Do the GPU and
//! mutants skips inside the job (changed-files detection, then skip as success under the same check name), not with
//! workflow-level `paths:`".
//!
//! - `qa_r656_files_real_readers_read_are_code`: a sample of readers qa found by its own search (a test's
//!   `include_str!` or `fs::read` of a contract, the ledger register's corpus citations its test resolves, plan-check's
//!   globs, lint vocab's walk, `ai_review_rules`' AGENTS.md and GEMINI.md, the screenshot artboard and the print-graph
//!   catalogue): each file, touched alone, classifies `docs_only=false gpu=true`, so the build, test and GPU jobs run.
//!   Files a directory walker would pick up if added (a new note, a new HTML page, a new archived queue file) too.
//! - `qa_r656_unread_docs_stay_docs_only`: real files under `docs/` that no code, plan check or script names (the
//!   README's images, an incoming data table, the reference sources, an experiment's results) classify
//!   `docs_only=true gpu=false`, alone and together; the test first checks that no source names them.
//! - `qa_r656_detect_diffs_the_merge_against_its_base`: `detect` on a real merge commit, as GitHub makes a pull
//!   request's, reads the change against the first parent (the base), not against the pull request's head; a non-pull
//!   request event, and a commit that is no merge, run everything.
//! - `qa_r656_required_checks_report_under_always`: the eight required checks are present; `ci.yml`'s five gates and
//!   `mutants.yml`'s `mutants-check` are `if: always()` jobs; neither workflow filters `pull_request` by path.
//! - `qa_r656_push_runs_only_main_and_measure`: `ci.yml`'s `push` names exactly `main` and `measure/**`; no other
//!   workflow's `push` names a branch other than `main`.
//!
//! Each has its negative control (R-176, REQ-VAL-147, R-199).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_r656: {rel}: {e}"))
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_r656: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// The script's verdict `(docs_only, gpu)` on a change of `paths`, through `classify`.
fn verdict(script: &str, paths: &[&str]) -> (bool, bool) {
    let dir = Scratch::new("qa_r656_classify");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_changes.sh");
    let input = dir.join("paths");
    std::fs::write(&file, script).expect("script written");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let out = Command::new("bash")
        .args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(&file)
        .arg(&input)
        .timed_output()
        .expect("qa_r656: bash ran");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "qa_r656: classify failed: {stdout}");
    flags(&stdout)
}

/// `docs_only=…` and `gpu=…` from a classifier's output.
fn flags(text: &str) -> (bool, bool) {
    let get = |key: &str| {
        let lines: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix(&format!("{key}=")))
            .collect();
        assert_eq!(lines.len(), 1, "qa_r656: not one `{key}=` in: {text}");
        match lines[0] {
            "true" => true,
            "false" => false,
            other => panic!("qa_r656: `{key}={other}`"),
        }
    };
    (get("docs_only"), get("gpu"))
}

// ---- Files code reads are code -----------------------------------------------------------------------------------

/// (a reader, a file it reads), found by qa's own search of the sources. Each reader names the file's base name.
const READERS: &[(&str, &str)] = &[
    (
        "crates/kernel/tests/qa_TASK-M3-02.rs",
        "docs/design/principia_dd_integrator.md",
    ),
    (
        "crates/render/tests/qa_TASK-M7-20.rs",
        "docs/gui/principia_render_gui_spec.md",
    ),
    (
        "crates/validation/tests/part6_catalogue.rs",
        "docs/contracts/principia_render_contract.md",
    ),
    (
        "xtask/tests/qa_TASK-M0-16.rs",
        "docs/contracts/principia_canonical_spec.md",
    ),
    (
        "xtask/tests/qa_TASK-M0-03.rs",
        "docs/read_first/principia_00_philosophy.md",
    ),
    (
        "crates/render/tests/qa_TASK-M1-03.rs",
        "docs/gui/reference/principia_colour_explorer.html",
    ),
    (
        "crates/kernel/tests/qa_TASK-M2-08.rs",
        "docs/gui/reference/ic_inspector.html",
    ),
    ("xtask/tests/screenshot.rs", "docs/gui/design/01_main.png"),
    ("xtask/tests/ai_review_rules.rs", "AGENTS.md"),
    ("xtask/tests/ai_review_rules.rs", "GEMINI.md"),
    ("crates/ledger/tests/qa_TASK-M1-18.rs", "decisions.md"),
    ("plan/check_plan.py", "plan/requirements.yaml"),
    ("plan/check_plan.py", "plan/tasks.yaml"),
    ("plan/tools/veto_marks.py", "CLAUDE.md"),
    ("plan/tools/rulings.py", "REVIEW_QUEUE.md"),
    ("plan/check_plan.py", "open-questions.md"),
    ("plan/tools/rulings.py", "README.md"),
    ("plan/tools/current_rules.py", "plan/rule_groups.yaml"),
    ("scripts/progress_banner.py", "plan/MILESTONES.md"),
];

/// Files a directory-walking reader reads, the first in the tree and the rest as a pull request might add them:
/// plan-check's task files (plan/check_plan.py's `plan/tasks/M*/TASK-*.md` glob; xtask/src/reviews_check.rs); its corpus
/// glob `docs/notes/*.md` (plan/tools/sections.py) and lint vocab's `.md` walk; lint vocab's `.html` walk
/// (xtask/src/lint_vocab.rs, all of docs/ but archive/ and reference/); plan-check's queue archive
/// (plan/tools/rulings.py, `docs/archive/review_queue/*.md`) and its copies in xtask/tests/plan_check.rs; a new task
/// file; a new artboard and a new reference catalogue entry.
const WALKED: &[&str] = &[
    "plan/tasks/M0/TASK-M0-01.md",
    "docs/notes/principia_qa_new_note.md",
    "docs/experiments/qa_new/page.html",
    "docs/archive/review_queue/M9.md",
    "plan/tasks/M9/TASK-M9-99.md",
    "docs/gui/design/99_qa.png",
    "docs/gui/reference/print_graph/catalogue/qa.json",
];

/// The corpus files the ledger's constant register cites (`Citation::Corpus { file, … }` in crates/ledger/src), which
/// crates/ledger/tests/constants.rs opens to resolve each cited heading.
fn register_citations() -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root().join("crates/ledger/src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("qa_r656: crates/ledger/src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("source");
            for piece in text.split("file: \"").skip(1) {
                let file = piece.split('"').next().unwrap_or_default();
                if file.ends_with(".md") && root().join(file).is_file() {
                    out.push(file.to_owned());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn check_readers_are_code(script: &str) {
    let mut touched: Vec<String> = Vec::new();
    for (reader, file) in READERS {
        let base = Path::new(file)
            .file_name()
            .expect("a file name")
            .to_string_lossy()
            .into_owned();
        assert!(
            read(reader).contains(&base),
            "qa_r656: {reader} does not name {base}; the sample is stale"
        );
        assert!(
            root().join(file).is_file(),
            "qa_r656: {file} is not in the tree"
        );
        touched.push((*file).to_owned());
    }
    let cited = register_citations();
    assert!(
        cited.len() >= 5,
        "qa_r656: the register's corpus citations were not found: {cited:?}"
    );
    touched.extend(cited);
    touched.extend(WALKED.iter().map(|f| (*f).to_owned()));
    for file in &touched {
        assert_eq!(
            verdict(script, &[file]),
            (false, true),
            "qa_r656: touching {file}, which code reads, does not run the heavy jobs (docs_only, gpu)"
        );
    }
    // With a docs-only file beside it, the read file still makes the change code.
    assert_eq!(
        verdict(script, &["docs/incoming/qa/b.pdf", "AGENTS.md"]),
        (false, true),
        "qa_r656: AGENTS.md beside a docs-only file does not run the heavy jobs"
    );
}

#[test]
fn qa_r656_files_real_readers_read_are_code() {
    check_readers_are_code(&read("scripts/ci_changes.sh"));
}

negative_control!(
    qa_r656_files_real_readers_read_are_code,
    "the script with every `.md` under docs/ dropped from its list of files code reads",
    expected = "touching docs/design/principia_dd_integrator.md, which code reads, does not run the heavy jobs",
    check_readers_are_code(&swap(
        &read("scripts/ci_changes.sh"),
        "    docs/*.md) return 0 ;;\n",
        ""
    ))
);

// ---- Files no code reads stay docs-only --------------------------------------------------------------------------

/// Files under `docs/` that qa found no reader of, as (folder, name): the README's images, an incoming data table, a
/// reference source, an experiment's result. Real at this head, but never opened or named here as one path, so this
/// test is no reader of them (a test reading one would make it code, R-656).
const UNREAD: &[(&str, &str)] = &[
    ("docs/readme", "hero.jpg"),
    ("docs/readme", "title-dark.svg"),
    (
        "docs/incoming/2026-10-26_orbit_catalogues",
        "threebodyorbits_ics.csv",
    ),
    ("docs/reference/prin-rs/src", "outcome.rs"),
    ("docs/experiments/results/xp_results", "exp1_raw.json"),
];

/// The source files of the code that could read a repository file: every crate's and xtask's `.rs`, the plan's and the
/// scripts' Python and shell, the workflows.
fn code_sources() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = ["crates", "xtask", "plan", "scripts", ".github/workflows"]
        .iter()
        .map(|d| root().join(d))
        .collect();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let path = entry.expect("entry").path();
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                if name != "target" && name != "__pycache__" {
                    stack.push(path);
                }
            } else if [".rs", ".py", ".sh", ".yml"]
                .iter()
                .any(|x| name.ends_with(x))
            {
                out.push(path);
            }
        }
    }
    out
}

fn check_unread_docs_only(script: &str) {
    let sources = code_sources();
    let files: Vec<String> = UNREAD.iter().map(|(d, n)| format!("{d}/{n}")).collect();
    for (file, (_, base)) in files.iter().zip(UNREAD) {
        for src in &sources {
            if src.ends_with("qa_R-656_ci_changes.rs") {
                continue;
            }
            let text = std::fs::read_to_string(src).unwrap_or_default();
            assert!(
                !text.contains(base),
                "qa_r656: {} names {base}; it is no longer an unread file",
                src.display()
            );
        }
        assert_eq!(
            verdict(script, &[file]),
            (true, false),
            "qa_r656: {file}, which no code reads, is not docs-only (docs_only, gpu)"
        );
    }
    let all: Vec<&str> = files.iter().map(String::as_str).collect();
    assert_eq!(
        verdict(script, &all),
        (true, false),
        "qa_r656: the unread files together are not docs-only (docs_only, gpu)"
    );
}

#[test]
fn qa_r656_unread_docs_stay_docs_only() {
    check_unread_docs_only(&read("scripts/ci_changes.sh"));
}

negative_control!(
    qa_r656_unread_docs_stay_docs_only,
    "the script with every path under docs/ counted as code",
    expected = "docs/readme/hero.jpg, which no code reads, is not docs-only",
    check_unread_docs_only(&swap(
        &read("scripts/ci_changes.sh"),
        "    docs/*.md) return 0 ;;\n",
        "    docs/*) return 0 ;;\n"
    ))
);

// ---- detect, on a real merge commit ------------------------------------------------------------------------------

/// Runs `git` in `repo` with no user or system configuration.
fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_DATE", "2026-10-30T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-10-30T00:00:00Z")
        .args([
            "-c",
            "user.name=qa",
            "-c",
            "user.email=qa@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .timed_output()
        .expect("qa_r656: git ran");
    assert!(
        out.status.success(),
        "qa_r656: git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn write(repo: &Path, rel: &str, text: &str) {
    let path = repo.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    std::fs::write(path, text).expect("written");
}

/// A repository whose `main` gains a code change after `pr` branched off it, `pr` changing `pr_path`, and the merge
/// commit GitHub would make (first parent `main`, second `pr`). Returns the merge commit and a plain commit.
fn merge_repo(repo: &Path, pr_path: &str) -> (String, String) {
    std::fs::create_dir_all(repo).expect("repo");
    git(repo, &["init", "-q"]);
    write(repo, "crates/kernel/src/lib.rs", "// base\n");
    write(repo, "docs/readme/qa_base.jpg", "base\n");
    write(repo, pr_path, "base\n");
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "base"]);
    git(repo, &["checkout", "-q", "-b", "pr"]);
    write(repo, pr_path, "the pull request's change\n");
    git(repo, &["commit", "-q", "-am", "pr"]);
    let plain = git(repo, &["rev-parse", "HEAD"]);
    git(repo, &["checkout", "-q", "main"]);
    write(repo, "crates/kernel/src/lib.rs", "// main moved on\n");
    git(repo, &["commit", "-q", "-am", "main"]);
    git(repo, &["merge", "-q", "--no-ff", "-m", "merge", "pr"]);
    (git(repo, &["rev-parse", "HEAD"]), plain)
}

/// `detect` run in `repo` on `sha` under `event`: the verdict it writes to GITHUB_OUTPUT.
fn detect(script: &str, repo: &Path, event: &str, sha: &str) -> (bool, bool) {
    let file = repo.join("ci_changes_under_test.sh");
    let output = repo.join("github_output");
    std::fs::write(&file, script).expect("script written");
    let _ = std::fs::remove_file(&output);
    let out = Command::new("bash")
        .arg(&file)
        .arg("detect")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GITHUB_EVENT_NAME", event)
        .env("GITHUB_SHA", sha)
        .env("GITHUB_OUTPUT", &output)
        .timed_output()
        .expect("qa_r656: bash ran");
    assert!(
        out.status.success(),
        "qa_r656: detect failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    flags(&std::fs::read_to_string(&output).expect("qa_r656: detect wrote no GITHUB_OUTPUT"))
}

fn check_detect(script: &str) {
    let dir = Scratch::new("qa_r656_detect");
    // A docs-only pull request on a base that moved on with code: docs-only, no GPU suites.
    let docs = dir.join("docs");
    let (merge, plain) = merge_repo(&docs, "docs/incoming/qa/table.csv");
    assert_eq!(
        detect(script, &docs, "pull_request", &merge),
        (true, false),
        "qa_r656: detect on a docs-only pull request's merge commit is not docs-only (docs_only, gpu)"
    );
    // Any other event runs everything, whatever the commit.
    assert_eq!(
        detect(script, &docs, "push", &merge),
        (false, true),
        "qa_r656: detect on a push does not run everything (docs_only, gpu)"
    );
    // A commit that is no merge has no base to diff against: everything runs.
    assert_eq!(
        detect(script, &docs, "pull_request", &plain),
        (false, true),
        "qa_r656: detect on a commit that is no merge does not run everything (docs_only, gpu)"
    );
    // A pull request touching a file code reads runs everything.
    let read = dir.join("read");
    let (merge, _) = merge_repo(&read, "docs/contracts/principia_parity_contract.md");
    assert_eq!(
        detect(script, &read, "pull_request", &merge),
        (false, true),
        "qa_r656: detect on a pull request touching a contract does not run everything (docs_only, gpu)"
    );
}

#[test]
fn qa_r656_detect_diffs_the_merge_against_its_base() {
    check_detect(&read("scripts/ci_changes.sh"));
}

negative_control!(
    qa_r656_detect_diffs_the_merge_against_its_base,
    "the script diffing the merge commit against its second parent, the pull request's head",
    expected = "detect on a docs-only pull request's merge commit is not docs-only",
    check_detect(&swap(
        &read("scripts/ci_changes.sh"),
        "\"$head^1\" \"$head\"",
        "\"$head^2\" \"$head\""
    ))
);

// ---- The workflows -----------------------------------------------------------------------------------------------

/// A workflow parsed by PyYAML, as JSON, its `on` key restored (YAML 1.1 reads it as `true`).
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_r656_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("wf.yml");
    std::fs::write(&file, yml).expect("written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa_r656: python3 ran");
    assert!(
        out.status.success(),
        "qa_r656: not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_r656: JSON")
}

fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The jobs of `wf` reporting the check `name` (their `name:`, or their id where they have none).
fn reporting<'a>(wf: &'a Value, name: &str) -> Vec<(&'a String, &'a Value)> {
    wf["jobs"]
        .as_object()
        .expect("qa_r656: jobs")
        .iter()
        .filter(|(id, job)| {
            job.get("name")
                .map_or(id.as_str() == name, |n| s(n) == name)
        })
        .collect()
}

fn check_required(ci: &str, mutants: &str, pr_check: &str, reviews: &str) {
    let ci = parse(ci);
    for gate in ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"] {
        let jobs = reporting(&ci, gate);
        assert_eq!(
            jobs.len(),
            1,
            "qa_r656: ci.yml has {} jobs reporting the required check `{gate}`",
            jobs.len()
        );
        assert_eq!(
            s(&jobs[0].1["if"]),
            "always()",
            "qa_r656: ci.yml's required check `{gate}` is not `if: always()`"
        );
    }
    let mutants = parse(mutants);
    let jobs = reporting(&mutants, "mutants-check");
    assert_eq!(
        jobs.len(),
        1,
        "qa_r656: mutants.yml does not report `mutants-check` once"
    );
    assert!(
        s(&jobs[0].1["if"]).starts_with("always()"),
        "qa_r656: mutants.yml's required check `mutants-check` is not `if: always()`"
    );
    for (wf, check) in [
        (parse(pr_check), "pr-check"),
        (parse(reviews), "reviews-complete"),
    ] {
        assert_eq!(
            reporting(&wf, check).len(),
            1,
            "qa_r656: the required check `{check}` is not reported"
        );
    }
    for (file, wf) in [("ci.yml", &ci), ("mutants.yml", &mutants)] {
        let pr = &wf["on"]["pull_request"];
        assert!(
            wf["on"].get("pull_request").is_some()
                && pr.get("paths").is_none()
                && pr.get("paths-ignore").is_none(),
            "qa_r656: {file} does not run on every pull request"
        );
    }
}

fn workflow(name: &str) -> String {
    read(&format!(".github/workflows/{name}"))
}

#[test]
fn qa_r656_required_checks_report_under_always() {
    check_required(
        &workflow("ci.yml"),
        &workflow("mutants.yml"),
        &workflow("pr-check.yml"),
        &workflow("reviews.yml"),
    );
}

negative_control!(
    qa_r656_required_checks_report_under_always,
    "ci.yml with the `gpu-lavapipe` gate run only when its suites succeeded",
    expected = "ci.yml's required check `gpu-lavapipe` is not `if: always()`",
    check_required(
        &swap(
            &workflow("ci.yml"),
            "    name: gpu-lavapipe\n    needs: [changes, gpu-lavapipe]\n    if: always()\n",
            "    name: gpu-lavapipe\n    needs: [changes, gpu-lavapipe]\n    if: success()\n"
        ),
        &workflow("mutants.yml"),
        &workflow("pr-check.yml"),
        &workflow("reviews.yml"),
    )
);

/// Every workflow file, by name.
fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("qa_r656: workflows")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            (
                p.file_name().expect("name").to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("workflow"),
            )
        })
        .collect();
    out.sort();
    out
}

fn check_push(workflows: &[(String, String)]) {
    let mut saw_ci = false;
    for (file, yml) in workflows {
        let wf = parse(yml);
        let on = &wf["on"];
        let push = match on {
            Value::Object(m) => m.get("push").cloned(),
            Value::Array(a) if a.iter().any(|t| t == "push") => Some(Value::Null),
            Value::String(t) if t == "push" => Some(Value::Null),
            _ => None,
        };
        if file == "ci.yml" {
            saw_ci = true;
            let push = push.expect("qa_r656: ci.yml has no push trigger");
            let keys: Vec<&String> = push
                .as_object()
                .map(|m| m.keys().collect())
                .unwrap_or_default();
            assert_eq!(
                keys,
                vec!["branches"],
                "qa_r656: ci.yml's push trigger is not limited to branches"
            );
            let branches: Vec<String> = push["branches"]
                .as_array()
                .expect("qa_r656: branches list")
                .iter()
                .map(s)
                .collect();
            assert_eq!(
                branches,
                vec!["main", "measure/**"],
                "qa_r656: ci.yml's push runs on branches other than main and measure/**"
            );
        } else if let Some(push) = push {
            let branches: Vec<String> = push
                .get("branches")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("qa_r656: {file}'s push trigger is not limited to main"))
                .iter()
                .map(s)
                .collect();
            assert_eq!(
                branches,
                vec!["main"],
                "qa_r656: {file}'s push trigger is not limited to main"
            );
        }
    }
    assert!(saw_ci, "qa_r656: no ci.yml");
}

#[test]
fn qa_r656_push_runs_only_main_and_measure() {
    check_push(&workflows());
}

negative_control!(
    qa_r656_push_runs_only_main_and_measure,
    "ci.yml with its push trigger unfiltered, as before R-656",
    expected = "ci.yml's push trigger is not limited to branches",
    {
        let mut all = workflows();
        let at = all.iter().position(|(f, _)| f == "ci.yml").expect("ci.yml");
        all[at].1 = swap(
            &all[at].1,
            "  push:\n    branches: [main, \"measure/**\"]\n",
            "  push:\n",
        );
        check_push(&all)
    }
);
