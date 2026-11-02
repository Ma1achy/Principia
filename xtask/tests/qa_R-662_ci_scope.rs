//! qa's tests for R-662, written from the ruling and the owner's words, not from `scripts/ci_changes.sh`'s code.
//!
//! The owner, 31 Oct 2026: "Only the actual Principia CODE matters in ci? If you get me. So GUI and Physics etc for
//! Principia, not random doc stuff". R-662: full CI runs on `crates/`, `xtask/`, `fixtures/`, the Cargo files, the
//! toolchain, `.github/` and the data code loads; only the cheap checks on the rest of `docs/`, `plan/`, `scripts/`,
//! `workbench/` and the root's markdown; an unknown path, a mixed change and a diff that can't be computed run full CI;
//! the required checks still report and conclude success.
//!
//! - `qa_r662_every_tracked_path_classifies_as_the_ruling_says`: every path `git ls-files` lists, against this file's
//!   own reading of the ruling's lists (`ruling`).
//! - `qa_r662_unknown_paths_run_everything`: paths the ruling names nowhere, alone and beside a cheap path.
//! - `qa_r662_required_checks_report_on_the_cheap_path`: `ci.yml`'s jobs walked as GitHub would on each verdict, each
//!   gate's own script run under bash; the three other required checks and the atlas guard read from their workflows.
//! - `qa_r662_mutants_shard_skips_only_rust_not_principias`: `mutants.yml`'s own diff step on real `git diff`s.
//! - `qa_r662_detect_fails_towards_full_ci`: `detect` on real merge commits: a move out of the code, a deletion, an
//!   empty merge, a path git must quote, a commit that isn't there.
//!
//! Every path under `docs/` these name is made up: a literal naming a real data file would count as a reader of it
//! (`xtask/tests/ci_changes.rs`). Each test has its negative control (R-176).

use std::collections::BTreeMap;
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
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_r662: {rel}: {e}"))
}

fn script() -> String {
    read("scripts/ci_changes.sh")
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_r662: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

fn flags(text: &str) -> (bool, bool) {
    let get = |key: &str| {
        let lines: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix(&format!("{key}=")))
            .collect();
        assert_eq!(lines.len(), 1, "qa_r662: not one `{key}=` in: {text}");
        lines[0] == "true"
    };
    (get("docs_only"), get("gpu"))
}

/// The script's `(docs_only, gpu)` on one change of `paths`.
fn verdict(script: &str, paths: &[&str]) -> (bool, bool) {
    let dir = Scratch::new("qa_r662_classify");
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
        .expect("qa_r662: bash ran");
    assert!(out.status.success(), "qa_r662: classify failed");
    flags(&String::from_utf8_lossy(&out.stdout))
}

// ---- Every tracked path ------------------------------------------------------------------------------------------

/// R-662's data code loads, its section "The data code loads", and the classifier ("Applied per R-369").
fn data_code_loads(path: &str) -> bool {
    path.starts_with("docs/gui/design/")
        || path.starts_with("docs/gui/reference/")
        || path.starts_with("plan/tasks/")
        || [
            "plan/requirements.yaml",
            "plan/tasks.yaml",
            "plan/MILESTONES.md",
        ]
        .contains(&path)
        || ["scripts/ci_changes.sh", "scripts/contain.sh"].contains(&path)
}

/// R-662's "only the cheap checks": `docs/`, `plan/`, `scripts/`, `workbench/` and the root's markdown, but that data.
fn cheap(path: &str) -> bool {
    let listed = ["docs/", "plan/", "scripts/", "workbench/"]
        .iter()
        .any(|d| path.starts_with(d))
        || (!path.contains('/') && path.ends_with(".md"));
    listed && !data_code_loads(path)
}

/// The verdict the ruling gives one path: `(docs_only, gpu)`. R-656 keeps `crates/gui/` and `crates/prin/` off the GPU
/// suites, and R-662 keeps the data code loads on them.
fn ruling(path: &str) -> (bool, bool) {
    let docs_only = cheap(path);
    let no_gpu = docs_only || path.starts_with("crates/gui/") || path.starts_with("crates/prin/");
    (docs_only, !no_gpu)
}

fn tracked() -> Vec<String> {
    let out = Command::new("git")
        .current_dir(root())
        .args(["ls-files", "-z"])
        .timed_output()
        .expect("qa_r662: git ran");
    assert!(out.status.success(), "qa_r662: git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

fn check_tracked(script: &str, paths: &[String]) {
    // Not vacuous: the tree holds something from each of the ruling's lists.
    for (what, found) in [
        ("crates/", paths.iter().any(|p| p.starts_with("crates/"))),
        (
            "a shader",
            paths
                .iter()
                .any(|p| p.starts_with("crates/") && p.ends_with(".wgsl")),
        ),
        (
            "crates/ledger/",
            paths.iter().any(|p| p.starts_with("crates/ledger/")),
        ),
        (
            "crates/gui/",
            paths.iter().any(|p| p.starts_with("crates/gui/")),
        ),
        ("xtask/", paths.iter().any(|p| p.starts_with("xtask/"))),
        (
            "fixtures/",
            paths.iter().any(|p| p.starts_with("fixtures/")),
        ),
        (".github/", paths.iter().any(|p| p.starts_with(".github/"))),
        ("Cargo.toml", paths.iter().any(|p| p == "Cargo.toml")),
        ("Cargo.lock", paths.iter().any(|p| p == "Cargo.lock")),
        (
            "the toolchain",
            paths.iter().any(|p| p == "rust-toolchain.toml"),
        ),
        ("decisions.md", paths.iter().any(|p| p == "decisions.md")),
        (
            "docs/",
            paths.iter().any(|p| cheap(p) && p.starts_with("docs/")),
        ),
        (
            "plan/'s Python",
            paths
                .iter()
                .any(|p| cheap(p) && p.ends_with(".py") && p.starts_with("plan/")),
        ),
        (
            "scripts/",
            paths.iter().any(|p| cheap(p) && p.starts_with("scripts/")),
        ),
        (
            "workbench/",
            paths
                .iter()
                .any(|p| cheap(p) && p.starts_with("workbench/")),
        ),
        (
            "the artboards",
            paths.iter().any(|p| p.starts_with("docs/gui/design/")),
        ),
        (
            "the references",
            paths.iter().any(|p| p.starts_with("docs/gui/reference/")),
        ),
        (
            "the task files",
            paths.iter().any(|p| p.starts_with("plan/tasks/")),
        ),
    ] {
        assert!(
            found,
            "qa_r662: the tree has no {what}; the fixture is stale"
        );
    }
    let dir = Scratch::new("qa_r662_tracked");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_changes.sh");
    std::fs::write(&file, script).expect("script written");

    // A change is docs-only only if each of its paths is, so one run over all the cheap paths settles each of them.
    let cheap_paths: Vec<&str> = paths
        .iter()
        .map(String::as_str)
        .filter(|p| cheap(p))
        .collect();
    assert_eq!(
        verdict(script, &cheap_paths),
        (true, false),
        "qa_r662: the {} paths the ruling gives only the cheap checks are not docs-only together",
        cheap_paths.len()
    );

    // One code path hides another in a joint run, so each of the others runs alone.
    let others: Vec<&str> = paths
        .iter()
        .map(String::as_str)
        .filter(|p| !cheap(p))
        .collect();
    let input = dir.join("others");
    std::fs::write(&input, others.join("\n") + "\n").expect("paths written");
    let out = Command::new("bash")
        .args([
            "-c",
            "while IFS= read -r p; do printf '%s\\n' \"$p\" | bash \"$0\" classify | tr '\\n' ' '; echo; done < \"$1\"",
        ])
        .arg(&file)
        .arg(&input)
        .timed_output()
        .expect("qa_r662: bash ran");
    assert!(out.status.success(), "qa_r662: the path-by-path run failed");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), others.len(), "qa_r662: not one verdict a path");
    for (path, line) in others.iter().zip(lines) {
        let got = flags(&line.replace(' ', "\n"));
        assert_eq!(
            got,
            ruling(path),
            "qa_r662: {path}: (docs_only, gpu) is {got:?}, the ruling gives {:?}",
            ruling(path)
        );
    }
}

#[test]
fn qa_r662_every_tracked_path_classifies_as_the_ruling_says() {
    check_tracked(&script(), &tracked());
}

negative_control!(
    qa_r662_every_tracked_path_classifies_as_the_ruling_says,
    "the script with the HTML references off its list of data code loads",
    expected = "the ruling gives (false, true)",
    check_tracked(
        &swap(&script(), "    docs/gui/reference/*) return 0 ;;\n", ""),
        &tracked()
    )
);

// ---- Unknown paths -----------------------------------------------------------------------------------------------

/// Paths R-662 names nowhere, or that only look like a listed one.
const UNKNOWN: &[&str] = &[
    "newtop/a.md",
    "LICENSE",
    "Makefile",
    "package.json",
    "src/main.rs",
    "target/a",
    "results/a.csv",
    ".nvmrc",
    ".gitattributes",
    ".vscode/a.json",
    ".devcontainer/a",
    // Near neighbours of the allowlist.
    "docs",
    "plan",
    "scripts",
    "workbench",
    "docs2/a.md",
    ".docs/a.md",
    "Docs/a.md",
    "DOCS/a.md",
    "PLAN/a.md",
    "Decisions.md",
    "decisions.MD",
    "decisions.md.orig",
    // An allowlisted name below a path that isn't.
    "a/docs/b.md",
    "a/decisions.md",
    "crates/docs/a.md",
    "crates/kernel/README.md",
    "xtask/docs/a.md",
    "fixtures/docs/a.md",
    "fixtures/plan/a.yaml",
    ".github/docs/a.md",
    // Paths written differently from how git prints a plain one, as git prints an odd one.
    "./docs/a.md",
    "/docs/a.md",
    " docs/a.md",
    "\"docs/a\\tb.md\"",
    "   ",
];

fn check_unknown(script: &str) {
    for path in UNKNOWN {
        assert_eq!(
            verdict(script, &[path]),
            (false, true),
            "qa_r662: the unknown path {path:?} does not run full CI (docs_only, gpu)"
        );
        // A mixed change, with the unknown path on either side.
        assert_eq!(
            verdict(script, &["docs/qa_note.md", path, "decisions.md"]),
            (false, true),
            "qa_r662: {path:?} among docs does not run full CI (docs_only, gpu)"
        );
    }
    assert_eq!(
        verdict(script, &[]),
        (false, true),
        "qa_r662: an empty change does not run full CI (docs_only, gpu)"
    );
    // The test can tell: what is listed still doesn't run everything.
    assert_eq!(
        verdict(
            script,
            &[
                "docs/qa_note.md",
                "plan/qa.py",
                "scripts/qa.py",
                "workbench/qa",
                "decisions.md"
            ]
        ),
        (true, false),
        "qa_r662: the listed paths together are not docs-only (docs_only, gpu)"
    );
}

#[test]
fn qa_r662_unknown_paths_run_everything() {
    check_unknown(&script());
}

negative_control!(
    qa_r662_unknown_paths_run_everything,
    "the script's allowlist accepting whatever it doesn't name",
    expected = "the unknown path \"newtop/a.md\" does not run full CI",
    check_unknown(&swap(
        &script(),
        "  esac\n  return 1\n}\n",
        "  esac\n  return 0\n}\n"
    ))
);

// ---- The required checks on the cheap path -------------------------------------------------------------------------

fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_r662_yaml");
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
        .expect("qa_r662: python3 ran");
    assert!(
        out.status.success(),
        "qa_r662: not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_r662: JSON")
}

fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn needs_of(job: &Value) -> Vec<String> {
    match job.get("needs") {
        Some(Value::Array(a)) => a.iter().map(s).collect(),
        Some(one) => vec![s(one)],
        None => Vec::new(),
    }
}

/// A finished run: each job's result, and each job's outputs.
#[derive(Default)]
struct Walk {
    results: BTreeMap<String, String>,
    outputs: BTreeMap<String, BTreeMap<String, String>>,
}

impl Walk {
    /// One operand of an expression: `needs.<job>.result`, `needs.<job>.outputs.<key>` or a quoted string. Anything else
    /// stops the test: the walk knows only what these workflows' conditions use.
    fn term(&self, term: &str) -> String {
        let t = term.trim();
        if let Some(q) = t.strip_prefix('\'') {
            return q.trim_end_matches('\'').to_owned();
        }
        let parts: Vec<&str> = t.split('.').collect();
        match parts.as_slice() {
            ["needs", job, "result"] => self.results.get(*job).cloned().unwrap_or_default(),
            ["needs", job, "outputs", key] => self
                .outputs
                .get(*job)
                .and_then(|o| o.get(*key))
                .cloned()
                .unwrap_or_default(),
            _ => panic!("qa_r662: the walk does not know the expression `{t}`"),
        }
    }

    /// An expression's value, GitHub's way: `a == b`, `a != b`, `&&` between them, or one operand.
    fn eval(&self, expr: &str) -> String {
        let e = expr
            .trim()
            .trim_start_matches("${{")
            .trim_end_matches("}}")
            .trim();
        let mut value = String::new();
        for clause in e.split("&&") {
            value = if let Some((a, b)) = clause.split_once("==") {
                (self.term(a) == self.term(b)).to_string()
            } else if let Some((a, b)) = clause.split_once("!=") {
                (self.term(a) != self.term(b)).to_string()
            } else {
                self.term(clause)
            };
            if e.contains("&&") && value != "true" {
                return "false".to_owned();
            }
        }
        value
    }
}

/// `ci.yml` walked as GitHub runs it. `verdict` is what `changes` outputs; each job in `fail` fails. A job whose `needs`
/// didn't all succeed is skipped unless its `if` holds `always()`; a matrix job counts as one. A gate (a job of one step
/// with `RESULTS` in its env) has its own script run under bash, with its own env expressions evaluated.
fn walk(yml: &Value, verdict: (bool, bool), fail: &[&str]) -> Walk {
    let jobs = yml["jobs"].as_object().expect("qa_r662: jobs");
    let mut w = Walk::default();
    while w.results.len() < jobs.len() {
        let before = w.results.len();
        for (id, job) in jobs {
            let needs = needs_of(job);
            if w.results.contains_key(id) || needs.iter().any(|n| !w.results.contains_key(n)) {
                continue;
            }
            let cond = s(&job["if"]);
            let ok = needs.iter().all(|n| w.results[n] == "success");
            let runs = if cond.contains("always()") {
                true
            } else {
                ok && (cond.is_empty() || w.eval(&cond) == "true")
            };
            let result = if !runs {
                "skipped"
            } else if fail.contains(&id.as_str()) {
                "failure"
            } else if let Some(env) = job["steps"][0]
                .get("env")
                .filter(|e| e.get("RESULTS").is_some())
            {
                let results: Vec<&str> = needs.iter().map(|n| w.results[n].as_str()).collect();
                let mut cmd = Command::new("bash");
                cmd.args(["-e", "-c", &s(&job["steps"][0]["run"])]);
                cmd.env("RESULTS", results.join(" "));
                for (k, v) in env.as_object().expect("env") {
                    if k != "RESULTS" {
                        cmd.env(k, w.eval(&s(v)));
                    }
                }
                let out = cmd.timed_output().expect("qa_r662: bash ran");
                if out.status.success() {
                    "success"
                } else {
                    "failure"
                }
            } else {
                "success"
            };
            let mut outputs = BTreeMap::new();
            if id == "changes" {
                if result == "success" {
                    outputs.insert("docs_only".to_owned(), verdict.0.to_string());
                    outputs.insert("gpu".to_owned(), verdict.1.to_string());
                }
            } else if result != "skipped" {
                if let Some(o) = job["outputs"].as_object() {
                    for (k, v) in o {
                        if s(v).contains("needs.") {
                            outputs.insert(k.clone(), w.eval(&s(v)));
                        }
                    }
                }
            }
            w.outputs.insert(id.clone(), outputs);
            w.results.insert(id.clone(), result.to_owned());
        }
        assert!(
            w.results.len() > before,
            "qa_r662: the jobs' `needs` never resolve"
        );
    }
    w
}

/// The result of the job reporting the check `name`, which must be exactly one job.
fn check_result(yml: &Value, w: &Walk, name: &str) -> String {
    let by: Vec<&String> = yml["jobs"]
        .as_object()
        .expect("jobs")
        .iter()
        .filter(|(id, j)| j.get("name").map_or(id.as_str() == name, |n| s(n) == name))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(by.len(), 1, "qa_r662: `{name}` is reported by {by:?}");
    w.results[by[0]].clone()
}

const GATES: [&str; 5] = ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"];

/// The commands `ci-workspace` runs when `changes` said `docs_only`.
fn workspace_runs(yml: &Value, docs_only: bool) -> String {
    let mut w = Walk::default();
    let mut outputs = BTreeMap::new();
    outputs.insert("docs_only".to_owned(), docs_only.to_string());
    w.outputs.insert("changes".to_owned(), outputs);
    yml["jobs"]["ci-workspace"]["steps"]
        .as_array()
        .expect("qa_r662: ci-workspace's steps")
        .iter()
        .filter(|step| step.get("if").is_none_or(|c| w.eval(&s(c)) == "true"))
        .map(|step| s(&step["run"]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The check `check` of `wf` runs on every pull request, on no path filter, and can't be skipped; returns its job.
fn unconditional(wf: &str, check: &str) -> Value {
    let yml = parse(&read(&format!(".github/workflows/{wf}")));
    let pr = yml["on"].get("pull_request");
    assert!(pr.is_some(), "qa_r662: {wf} does not run on a pull request");
    let pr = pr.expect("checked");
    for key in ["paths", "paths-ignore", "branches", "branches-ignore"] {
        assert!(
            pr.get(key).is_none(),
            "qa_r662: {wf} filters pull requests by `{key}:`"
        );
    }
    let job = yml["jobs"][check].clone();
    assert!(
        job.get("needs").is_none() && job.get("if").is_none(),
        "qa_r662: {wf}'s `{check}` can be skipped"
    );
    job
}

fn check_required(ci: &str, mutants: &str) {
    let yml = parse(ci);
    // The cheap path: every gate reports success, and nothing but `changes`, the cheap checks and the gates ran.
    let cheap = walk(&yml, (true, false), &[]);
    for gate in GATES {
        assert_eq!(
            check_result(&yml, &cheap, gate),
            "success",
            "qa_r662: the required check `{gate}` is not a success on the cheap path"
        );
    }
    let ran: Vec<&str> = cheap
        .results
        .iter()
        .filter(|(_, r)| r.as_str() != "skipped")
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        ran,
        vec![
            "changes",
            "ci-gate",
            "ci-workspace",
            "gpu-kernel-gate",
            "gpu-lavapipe-gate",
            "gpu-metal-gate",
            "xtask-ci-gate"
        ],
        "qa_r662: the cheap path ran other jobs"
    );
    let runs = workspace_runs(&yml, true);
    for want in ["python3 plan/check_plan.py", "cargo xtask lint vocab"] {
        assert!(
            runs.contains(want),
            "qa_r662: the cheap path does not run `{want}`:\n{runs}"
        );
    }
    for never in [
        "nextest",
        "cargo test",
        "cargo build",
        "clippy",
        "cargo fmt",
        "cargo doc",
    ] {
        assert!(
            !runs.contains(never),
            "qa_r662: the cheap path runs `{never}`:\n{runs}"
        );
    }
    // The other side: full CI on code runs what the cheap path skipped, and the gates pass.
    let full = walk(&yml, (false, true), &[]);
    assert!(
        full.results.values().all(|r| r == "success"),
        "qa_r662: full CI skipped a job: {:?}",
        full.results
    );
    let runs = workspace_runs(&yml, false);
    assert!(
        runs.contains("cargo nextest run -p xtask") && !runs.contains("cargo xtask lint vocab"),
        "qa_r662: ci-workspace on code does not run its tests:\n{runs}"
    );
    // A failed cheap check is a red `ci`: the skips never hide it.
    let red = walk(&yml, (true, false), &["ci-workspace"]);
    assert_eq!(
        check_result(&yml, &red, "ci"),
        "failure",
        "qa_r662: `ci` passes a failed cheap check"
    );
    // A classification that failed is no cheap path: every gate turns red, and none reports a skip.
    let lost = walk(&yml, (true, false), &["changes"]);
    for gate in GATES {
        assert_eq!(
            check_result(&yml, &lost, gate),
            "failure",
            "qa_r662: the required check `{gate}` is not red when the classification failed"
        );
    }
    // A code change whose shards were skipped is red, verdict and all.
    let skipped = walk(&yml, (false, true), &["ci-archive"]);
    assert_eq!(
        check_result(&yml, &skipped, "ci"),
        "failure",
        "qa_r662: `ci` passes when its archive job failed"
    );

    // `mutants-check`: always runs, and its own script passes a run with no report and fails a failed shard.
    let mutants = parse(mutants);
    let job = &mutants["jobs"]["mutants-check"];
    assert!(
        s(&job["if"]).starts_with("always()"),
        "qa_r662: `mutants-check` can be skipped"
    );
    let steps = job["steps"]
        .as_array()
        .expect("qa_r662: mutants-check's steps");
    let step = steps
        .iter()
        .find(|st| s(&st["run"]).contains("cargo xtask mutants-check"))
        .expect("qa_r662: mutants-check's step");
    for (result, pass) in [("success", true), ("failure", false), ("cancelled", false)] {
        let dir = Scratch::new("qa_r662_mutants_check");
        std::fs::create_dir_all(&*dir).expect("scratch");
        let run = s(&step["run"]).replace("${{ needs.mutants.result }}", result);
        let out = Command::new("bash")
            .args(["-e", "-c", &run])
            .env("RUNNER_TEMP", &*dir)
            .timed_output()
            .expect("qa_r662: bash ran");
        assert_eq!(
            out.status.success(),
            pass,
            "qa_r662: `mutants-check` with no report and its shards `{result}`: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }

    // `pr-check`, `reviews-complete` and the atlas guard take no part in the classification and run on every head.
    unconditional("pr-check.yml", "pr-check");
    unconditional("reviews.yml", "reviews-complete");
    let guard = unconditional("no-atlas-params.yml", "no-atlas-params");
    assert!(
        guard["steps"]
            .as_array()
            .expect("qa_r662: the guard's steps")
            .iter()
            .any(
                |st| s(&st["run"]).contains("scripts/check_no_atlas_params.sh")
                    && st.get("if").is_none()
            ),
        "qa_r662: the atlas guard does not run its script unconditionally"
    );
}

#[test]
fn qa_r662_required_checks_report_on_the_cheap_path() {
    check_required(
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/mutants.yml"),
    );
}

negative_control!(
    qa_r662_required_checks_report_on_the_cheap_path,
    "ci.yml with the gate `xtask-ci` run only when its shards succeeded",
    expected = "the required check `xtask-ci` is not a success on the cheap path",
    check_required(
        &swap(
            &read(".github/workflows/ci.yml"),
            "    name: xtask-ci\n    needs: [xtask-ci, changes]\n    if: always()\n",
            "    name: xtask-ci\n    needs: [xtask-ci, changes]\n"
        ),
        &read(".github/workflows/mutants.yml")
    )
);

// ---- Git, on real commits -------------------------------------------------------------------------------------------

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
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
        .expect("qa_r662: git ran");
    assert!(
        out.status.success(),
        "qa_r662: git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn write(repo: &Path, rel: &str, text: &str) {
    let path = repo.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    std::fs::write(path, text).expect("written");
}

/// A long enough body that git sees a moved file as a rename.
const BODY: &str =
    "pub fn a() {}\npub fn b() {}\npub fn c() {}\npub fn d() {}\npub fn e() {}\npub fn f() {}\n";

/// A repository with a `main` and a checked-out branch `pr` off it, which holds no commit of its own yet.
fn base_repo(repo: &Path) {
    std::fs::create_dir_all(repo).expect("repo");
    git(repo, &["init", "-q"]);
    write(repo, "crates/kernel/src/base.rs", BODY);
    write(repo, "docs/qa_base.md", "base\n");
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "base"]);
    git(repo, &["checkout", "-q", "-b", "pr"]);
}

fn commit_all(repo: &Path) {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "--allow-empty", "-m", "pr"]);
}

// ---- The mutants shard's diff step ---------------------------------------------------------------------------------

/// `mutants.yml`'s own diff step, on a pull request adding `paths`: its `rust=` output.
fn shard_rust(step: &str, repo: &Path, paths: &[&str]) -> bool {
    base_repo(repo);
    for path in paths {
        write(repo, path, BODY);
    }
    commit_all(repo);
    git(repo, &["update-ref", "refs/remotes/origin/main", "main"]);
    let temp = repo.join(".runner");
    std::fs::create_dir_all(&temp).expect("runner temp");
    let output = temp.join("output");
    let run = step.replace("${{ github.base_ref }}", "main");
    let out = Command::new("bash")
        .args(["-e", "-c", &run])
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("RUNNER_TEMP", &temp)
        .env("GITHUB_OUTPUT", &output)
        .timed_output()
        .expect("qa_r662: bash ran");
    assert!(
        out.status.success(),
        "qa_r662: the diff step failed on {paths:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&output).expect("qa_r662: no output");
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("rust="))
        .expect("qa_r662: no rust=");
    line == "true"
}

/// (the paths a pull request adds, whether a shard runs on them). Reference sources and the papers' code aren't
/// Principia's (R-662); a `.rs` anywhere else, one beside them above all, still reaches the gate.
const SHARD_CASES: &[(&[&str], bool)] = &[
    (&[], false),
    (&["docs/qa_note.md"], false),
    (&["docs/reference/qa/a.rs"], false),
    (&["docs/\u{e9}/a.rs"], false),
    (&["plan/qa/a.rs"], false),
    (&["scripts/qa/a.rs"], false),
    (&["workbench/qa a.rs"], false),
    (&["docs/reference/qa/a.rs", "workbench/qa/b.rs"], false),
    (&["crates/kernel/src/qa.rs"], true),
    (&["crates/gui/src/qa.rs"], true),
    (&["xtask/src/qa.rs"], true),
    (&["crates/kernel/src/q a.rs"], true),
    (&["crates/kernel/src/\u{e9}.rs"], true),
    // A listed name that isn't the top of the path.
    (&["crates/docs/qa.rs"], true),
    (&["crates/kernel/plan/qa.rs"], true),
    (&["docsx/qa.rs"], true),
    (&["newtop/qa.rs"], true),
    // Mixed, the Rust on either side of the reference source in the diff's order.
    (&["crates/kernel/src/qa.rs", "docs/reference/qa/a.rs"], true),
    (&["docs/reference/qa/a.rs", "zz/qa.rs"], true),
    (
        &["workbench/qa/a.rs", "xtask/src/qa.rs", "plan/qa/a.rs"],
        true,
    ),
];

fn check_shard(mutants: &str) {
    let yml = parse(mutants);
    let steps = yml["jobs"]["mutants"]["steps"]
        .as_array()
        .expect("qa_r662: the shard's steps");
    let step = steps
        .iter()
        .find(|st| s(&st["id"]) == "diff")
        .expect("qa_r662: the shard's diff step");
    let run = s(&step["run"]);
    for (i, (paths, want)) in SHARD_CASES.iter().enumerate() {
        let dir = Scratch::new("qa_r662_shard");
        let repo = dir.join(format!("r{i}"));
        assert_eq!(
            shard_rust(&run, &repo, paths),
            *want,
            "qa_r662: a shard on {paths:?}: rust is not {want}"
        );
    }
    // With no Rust of Principia's, nothing that costs runs: each step that isn't the checkout, the diff, the two that
    // report and upload, waits on the diff's verdict.
    for step in steps {
        let name = s(&step["name"]);
        let uses = s(&step["uses"]);
        if uses.starts_with("actions/checkout")
            || uses.starts_with("actions/upload-artifact")
            || s(&step["id"]) == "diff"
            || name == "Shard cut off or failed"
            || name.starts_with("cargo xtask mutants-check")
        {
            continue;
        }
        assert_eq!(
            s(&step["if"]),
            "steps.diff.outputs.rust == 'true'",
            "qa_r662: the shard's step `{name}{uses}` does not wait on the diff's verdict"
        );
    }
}

#[test]
fn qa_r662_mutants_shard_skips_only_rust_not_principias() {
    check_shard(&read(".github/workflows/mutants.yml"));
}

negative_control!(
    qa_r662_mutants_shard_skips_only_rust_not_principias,
    "mutants.yml with `crates/` among the folders whose Rust it ignores",
    expected = "a shard on [\"crates/kernel/src/qa.rs\"]: rust is not true",
    check_shard(&swap(
        &read(".github/workflows/mutants.yml"),
        "b/(docs|plan|scripts|workbench)/",
        "b/(docs|plan|scripts|workbench|crates)/"
    ))
);

// ---- detect, on real merge commits -----------------------------------------------------------------------------------

/// Merges the checked-out `pr` into `main`, moved on with code, as GitHub's merge commit does. Returns its id.
fn merged(repo: &Path) -> String {
    git(repo, &["checkout", "-q", "main"]);
    write(repo, "crates/render/src/moved_on.rs", "// main moved on\n");
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "main"]);
    git(repo, &["merge", "-q", "--no-ff", "-m", "merge", "pr"]);
    git(repo, &["rev-parse", "HEAD"])
}

fn detect(script: &Path, repo: &Path, sha: &str) -> (bool, bool) {
    let output = repo.join(".github_output");
    let _ = std::fs::remove_file(&output);
    let out = Command::new("bash")
        .arg(script)
        .arg("detect")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GITHUB_EVENT_NAME", "pull_request")
        .env("GITHUB_SHA", sha)
        .env("GITHUB_OUTPUT", &output)
        .timed_output()
        .expect("qa_r662: bash ran");
    assert!(
        out.status.success(),
        "qa_r662: detect failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    flags(&std::fs::read_to_string(&output).expect("qa_r662: detect wrote no GITHUB_OUTPUT"))
}

/// What a pull request does to the tree.
enum Pr {
    Add(&'static [&'static str]),
    Delete(&'static str),
    Move(&'static str),
    Nothing,
}

const DETECT_CASES: &[(&str, Pr, (bool, bool))] = &[
    (
        "the plan's Python, a script, the workbench and the root's markdown",
        Pr::Add(&[
            "plan/tools/qa.py",
            "scripts/qa.py",
            "workbench/qa/a.html",
            "decisions.md",
            "docs/qa/b.pdf",
        ]),
        (true, false),
    ),
    (
        "a non-ASCII name in the docs",
        Pr::Add(&["docs/\u{e9}.md"]),
        (true, false),
    ),
    (
        "a crate's file moved into the docs",
        Pr::Move("docs/moved.rs"),
        (false, true),
    ),
    (
        "a crate's file deleted",
        Pr::Delete("crates/kernel/src/base.rs"),
        (false, true),
    ),
    ("a merge that changes nothing", Pr::Nothing, (false, true)),
    (
        "a non-ASCII name in a crate",
        Pr::Add(&["crates/kernel/src/\u{e9}.rs"]),
        (false, true),
    ),
    (
        "a name git must quote",
        Pr::Add(&["docs/a\tb.md"]),
        (false, true),
    ),
    (
        "an unknown top-level folder",
        Pr::Add(&["newtop/a.md"]),
        (false, true),
    ),
    (
        "the classifier",
        Pr::Add(&["scripts/ci_changes.sh"]),
        (false, true),
    ),
    (
        "a task file",
        Pr::Add(&["plan/tasks/M9/TASK-M9-98.md"]),
        (false, true),
    ),
    (
        "the requirements beside prose",
        Pr::Add(&["plan/requirements.yaml", "plan/qa.md"]),
        (false, true),
    ),
    (
        "a workflow",
        Pr::Add(&[".github/workflows/qa.yml"]),
        (false, true),
    ),
];

fn check_detect(script_text: &str) {
    let dir = Scratch::new("qa_r662_detect");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("under_test.sh");
    std::fs::write(&file, script_text).expect("script written");
    for (i, (what, pr, want)) in DETECT_CASES.iter().enumerate() {
        let repo = dir.join(format!("r{i}"));
        base_repo(&repo);
        match pr {
            Pr::Add(paths) => {
                for path in *paths {
                    write(&repo, path, "the pull request's\n");
                }
            }
            Pr::Delete(path) => {
                std::fs::remove_file(repo.join(path)).expect("deleted");
            }
            Pr::Move(to) => {
                write(&repo, to, BODY);
                std::fs::remove_file(repo.join("crates/kernel/src/base.rs")).expect("moved");
            }
            Pr::Nothing => {}
        }
        commit_all(&repo);
        let sha = merged(&repo);
        assert_eq!(
            detect(&file, &repo, &sha),
            *want,
            "qa_r662: detect on {what} (docs_only, gpu)"
        );
    }
    // A commit that can't be read: the diff can't be computed.
    let repo = dir.join("gone");
    base_repo(&repo);
    assert_eq!(
        detect(&file, &repo, "0123456789012345678901234567890123456789"),
        (false, true),
        "qa_r662: detect on a commit that isn't there (docs_only, gpu)"
    );
}

#[test]
fn qa_r662_detect_fails_towards_full_ci() {
    check_detect(&script());
}

negative_control!(
    qa_r662_detect_fails_towards_full_ci,
    "the script letting git report a moved file by its new path alone",
    expected = "detect on a crate's file moved into the docs",
    check_detect(&swap(&script(), " --no-renames", ""))
);
