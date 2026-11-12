//! qa's tests for R-662, written from the ruling and the owner's words, not from `scripts/ci_changes.sh`'s code.
//!
//! The owner, 31 Oct 2026: "Only the actual Principia CODE matters in ci? If you get me. So GUI and Physics etc for
//! Principia, not random doc stuff". R-662: full CI runs on `crates/`, `xtask/`, `fixtures/`, the Cargo files, the
//! toolchain, `.github/` and the data code loads; only the cheap checks on the rest of `docs/`, `plan/`, `scripts/`,
//! `workbench/` and the root's markdown; an unknown path, a mixed change and a diff that can't be computed run full CI;
//! the required checks still report and conclude success.
//!
//! R-677 (2 Nov 2026) splits R-662's lists into three tiers: the plan's task data and the GUI's artboards and
//! references become tier (b), which runs no full CI (`docs_only=true`, no GPU suite) and runs their readers' tests
//! instead; the field check `scripts/ci_req_fields.py` joins the scripts that are code; mutants run only on Rust under
//! `crates/`. This file changes only where that ruling changes the behaviour it tests (R-677's exception to R-290).
//!
//! R-790 (4 Nov 2026) adds `.claude/`, its commands, agents and settings, to tier (c), the owner: "R-677 amendment:
//! .claude/ (commands, agents, settings) is docs tier." `cheap`, the model below, gains it, and the near neighbours a
//! prefix would catch join the unknown paths (R-790's exception to R-290).
//!
//! R-692 (4 Nov 2026) makes five paths of `crates/gui/` GPU paths, what `gpu-metal`'s `lattice_` step reads, and names
//! this file as what answers to it: `ruling` gains `r692_gpu_path`, and every other path of `crates/gui/` stays off the
//! GPU suites (R-692's exception to R-290).
//!
//! The split of `ci` into three shards under `tests-verdict`, which bears `ci`, and the Mesa cache's step before each apt
//! step (PR #495, 11 Nov 2026): the walk runs every step of a job that builds nothing, reads the Mesa step's output on a
//! cold cache, and takes a job's failure as a red required check when it bears one or a run where it alone fails turns
//! one red; the mutants shard's steps may join more to the diff's verdict with `&&`. Each change has its control (R-290's
//! exception, named by the orchestrator for this PR).
//!
//! - `qa_r662_every_tracked_path_classifies_as_the_ruling_says`: every path `git ls-files` lists, against this file's
//!   own reading of the ruling's lists (`ruling`).
//! - `qa_r662_unknown_paths_run_everything`: paths the ruling names nowhere, alone and beside a cheap path.
//! - `qa_r662_required_checks_report_on_the_cheap_path`: `ci.yml`'s jobs walked as GitHub would on each verdict of a
//!   pull request, each required job's classification check run under bash and its steps' `if:`s read: the PR tier's
//!   six required checks (the owner's CI model, 9 Nov 2026, in place of the full tier's gates) conclude success on the
//!   cheap path running nothing heavy; `mutants.yml` runs on no pull request; the two other required checks and the
//!   atlas guard read from their workflows (R-290's exception for the model, applied per R-369).
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

/// The one value of `key=` in a classifier's output.
fn value(text: &str, key: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix(&format!("{key}=")))
        .collect();
    assert_eq!(lines.len(), 1, "qa_r662: not one `{key}=` in: {text}");
    lines[0].to_owned()
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
    flags(&verdict_text(script, paths))
}

/// The script's whole verdict on one change of `paths`.
fn verdict_text(script: &str, paths: &[&str]) -> String {
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
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ---- Every tracked path ------------------------------------------------------------------------------------------

/// R-662's data code loads, its section "The data code loads", and the classifier ("Applied per R-369"), as R-677
/// leaves them in the code tier: the requirements (with no copies to compare, the field check can't clear them), the
/// classifier, its field check and the containment.
fn data_code_loads(path: &str) -> bool {
    path == "plan/requirements.yaml"
        || [
            "scripts/ci_changes.sh",
            "scripts/ci_req_fields.py",
            "scripts/contain.sh",
        ]
        .contains(&path)
}

/// R-677's tier (b): the plan's task data and the GUI's artboards and references, R-662's other data code loads.
fn tier_b(path: &str) -> bool {
    path.starts_with("docs/gui/design/")
        || path.starts_with("docs/gui/reference/")
        || path.starts_with("plan/tasks/")
        || ["plan/tasks.yaml", "plan/MILESTONES.md"].contains(&path)
}

/// R-662's "only the cheap checks" (R-677's tier c): `docs/`, `plan/`, `scripts/`, `workbench/`, `.claude/` (R-790) and
/// the root's markdown, but that data.
fn cheap(path: &str) -> bool {
    let listed = ["docs/", "plan/", "scripts/", "workbench/", ".claude/"]
        .iter()
        .any(|d| path.starts_with(d))
        || (!path.contains('/') && path.ends_with(".md"));
    listed && !data_code_loads(path) && !tier_b(path)
}

/// R-692's five GPU paths of `crates/gui/`: `crates/gui/src/mock/*.wgsl`, `crates/gui/src/mock/canvas.rs`,
/// `crates/gui/src/tests/mock_lattice.rs`, `crates/gui/src/tests/lattice_controls.wgsl` and `crates/gui/Cargo.toml`.
fn r692_gpu_path(path: &str) -> bool {
    path.strip_prefix("crates/gui/src/mock/")
        .is_some_and(|file| file.ends_with(".wgsl"))
        || [
            "crates/gui/src/mock/canvas.rs",
            "crates/gui/src/tests/mock_lattice.rs",
            "crates/gui/src/tests/lattice_controls.wgsl",
            "crates/gui/Cargo.toml",
        ]
        .contains(&path)
}

/// The verdict the ruling gives one path: `(docs_only, gpu, tier)`. R-656 keeps `crates/prin/` and, but for R-692's five
/// paths, `crates/gui/` off the GPU suites, and R-662 keeps the data code loads on them; R-677's tier (b) runs neither
/// full CI nor a GPU suite.
fn ruling(path: &str) -> (bool, bool, String) {
    let docs_only = cheap(path) || tier_b(path);
    let no_gpu = docs_only
        || (path.starts_with("crates/gui/") && !r692_gpu_path(path))
        || path.starts_with("crates/prin/");
    let tier = if cheap(path) {
        "docs"
    } else if tier_b(path) {
        "data"
    } else {
        "code"
    };
    (docs_only, !no_gpu, tier.to_owned())
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
        (
            "each of R-692's GPU paths",
            [
                "crates/gui/src/mock/noise.wgsl",
                "crates/gui/src/mock/canvas.rs",
                "crates/gui/src/tests/mock_lattice.rs",
                "crates/gui/src/tests/lattice_controls.wgsl",
                "crates/gui/Cargo.toml",
            ]
            .iter()
            .all(|p| r692_gpu_path(p) && paths.iter().any(|q| q == p)),
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
            ".claude/",
            paths.iter().any(|p| cheap(p) && p.starts_with(".claude/")),
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
    let out = verdict_text(script, &cheap_paths);
    assert!(
        flags(&out) == (true, false) && value(&out, "tier") == "docs",
        "qa_r662: the {} paths the ruling gives only the cheap checks are not docs-only together:\n{out}",
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
        let text = line.replace(' ', "\n");
        let (docs_only, gpu) = flags(&text);
        let got = (docs_only, gpu, value(&text, "tier"));
        assert_eq!(
            got,
            ruling(path),
            "qa_r662: {path}: (docs_only, gpu, tier) is {got:?}, the ruling gives {:?}",
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
    expected = "the ruling gives (true, false, \"data\")",
    check_tracked(
        &swap(&script(), "    docs/gui/reference/*) return 0 ;;\n", ""),
        &tracked()
    )
);

mod claude_runs_full_ci {
    use super::*;
    negative_control!(
        qa_r662_every_tracked_path_classifies_as_the_ruling_says,
        "the script with `.claude/` off its allowlist (R-790)",
        expected = "paths the ruling gives only the cheap checks are not docs-only together",
        check_tracked(
            &swap(&script(), " | .claude/*) return 0 ;;", ") return 0 ;;"),
            &tracked()
        )
    );
}

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
    ".claude",
    ".clauderc",
    ".claude2/a.md",
    ".CLAUDE/a.md",
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
    "a/.claude/b.md",
    "crates/.claude/a.md",
    ".github/.claude/a.md",
    // Paths written differently from how git prints a plain one, as git prints an odd one.
    "./docs/a.md",
    "/docs/a.md",
    " docs/a.md",
    "\"docs/a\\tb.md\"",
    "./.claude/a.md",
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
                ".claude/qa.md",
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

/// A finished run: each job's result, and each job's outputs; `env`, a job's own `env:` while its steps are read.
/// The owner's CI model (9 Nov 2026) put the required checks' skips inside each job's steps, behind job `env:` values
/// and the event, so the walk reads `github.event_name`, `github.ref` and `env.<key>` too, as a pull request's run on
/// its merge ref (R-290's exception, applied per R-369 by the orchestrator).
#[derive(Default, Clone)]
struct Walk {
    results: BTreeMap<String, String>,
    outputs: BTreeMap<String, BTreeMap<String, String>>,
    env: BTreeMap<String, String>,
}

impl Walk {
    /// One operand of an expression: `needs.<job>.result`, `needs.<job>.outputs.<key>`, `github.event_name`,
    /// `github.ref`, `env.<key>` or a quoted string. Anything else stops the test: the walk knows only what these
    /// workflows' conditions use.
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
            // A pull request's run, the one the required checks are read on.
            ["github", "event_name"] => "pull_request".to_owned(),
            ["github", "ref"] => "refs/pull/1/merge".to_owned(),
            ["env", key] => self.env.get(*key).cloned().unwrap_or_default(),
            // The Mesa cache's step (`.github/actions/mesa-debs`, 11 Nov 2026) on a cold cache, which outputs
            // `installed=false`: the apt step after it installs as before. Only this one step output is known; any
            // other still stops the walk (R-290's exception for the split, named by the orchestrator).
            ["steps", "mesa", "outputs", "installed"] => "false".to_owned(),
            // The shards' matrix (11 Nov 2026), read at its first leg, which runs every step any leg runs (the
            // doctests run there alone); that the legs' partitions cover every test once `qa_ci_split_mesa_rerun.rs`
            // checks.
            ["matrix", "shard"] => "1".to_owned(),
            // A number, as `matrix.shard == 1` compares the leg with.
            [n] if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => (*n).to_owned(),
            // A step's status functions, in a run with no failed step, which is the run this walk is of.
            ["always()"] => "true".to_owned(),
            ["failure()"] => "false".to_owned(),
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

/// `ci.yml` walked as GitHub runs it on a pull request. `verdict` is what `changes` outputs (`docs_only`, `gpu`, and
/// the tier and the rest as they follow from them); each job in `fail` fails. A job whose `needs` didn't all succeed is
/// skipped unless its `if` holds `always()` or `!cancelled()`; a matrix job counts as one. A job whose first step checks the
/// classification (`CHANGES` in its env, the owner's CI model of 9 Nov 2026) has that step's script run under bash, as
/// a gate (a job of one step with `RESULTS` in its env) has its own, with its own env expressions evaluated.
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
            // Inside `${{ }}` or not, the same expression.
            let cond = cond
                .trim()
                .strip_prefix("${{")
                .and_then(|c| c.strip_suffix("}}"))
                .map_or(cond.clone(), |c| c.trim().to_owned());
            let ok = needs.iter().all(|n| w.results[n] == "success");
            // R-684 joins an `if:` to `always()`: what follows it holds or skips the job. `!cancelled()` (the required
            // checks' jobs since 10 Nov 2026) reads as `always()` does: the run this walk is of is not cancelled.
            let runs = if let Some(rest) = cond
                .strip_prefix("always() && ")
                .or_else(|| cond.strip_prefix("!cancelled() && "))
            {
                w.eval(rest) == "true"
            } else if cond.contains("always()") || cond == "!cancelled()" {
                true
            } else {
                ok && (cond.is_empty() || w.eval(&cond) == "true")
            };
            let result = if !runs {
                "skipped"
            } else if fail.contains(&id.as_str()) {
                "failure"
            } else if job["steps"][0]
                .get("env")
                .is_some_and(|e| e.get("CHANGES").is_some())
            {
                // The classification check; and, in a job of `run:` steps alone that builds nothing (`tests-verdict`,
                // the split's `ci` over its shards, 11 Nov 2026), every step after it too, each with its own env.
                let steps = job["steps"].as_array().expect("qa_r662: steps");
                let verdict_only = steps.iter().all(|st| st.get("uses").is_none());
                let mut result = "success";
                for step in steps
                    .iter()
                    .take(if verdict_only { steps.len() } else { 1 })
                {
                    let mut cmd = Command::new("bash");
                    cmd.args(["-e", "-c", &s(&step["run"])]);
                    if let Some(env) = step["env"].as_object() {
                        for (k, v) in env {
                            cmd.env(k, w.eval(&s(v)));
                        }
                    }
                    let out = cmd.timed_output().expect("qa_r662: bash ran");
                    if !out.status.success() {
                        result = "failure";
                        break;
                    }
                }
                result
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
                    let tier = if verdict.0 { "docs" } else { "code" };
                    for (k, v) in [
                        ("docs_only", verdict.0.to_string()),
                        ("gpu", verdict.1.to_string()),
                        ("tier", tier.to_owned()),
                        ("plan_data", "false".to_owned()),
                        ("gui_data", "false".to_owned()),
                        ("process", "false".to_owned()),
                        ("packages", "--workspace".to_owned()),
                        ("doc_packages", "--workspace".to_owned()),
                    ] {
                        outputs.insert(k.to_owned(), v);
                    }
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

/// The check a job reports when it runs: its `name:`, or its id; a gate named by its one-value matrix reports that value
/// (R-684), and skipped, on a quick run, its raw `name:`.
fn reported_name(id: &str, job: &Value) -> String {
    let name = job.get("name").map_or_else(|| id.to_owned(), s);
    match job["strategy"]["matrix"]["check"]
        .as_array()
        .map(Vec::as_slice)
    {
        Some([only]) if name == "${{ matrix.check }}" => s(only),
        _ => name,
    }
}

/// The result of the job reporting the check `name`, which must be exactly one job.
fn check_result(yml: &Value, w: &Walk, name: &str) -> String {
    let by: Vec<&String> = yml["jobs"]
        .as_object()
        .expect("jobs")
        .iter()
        .filter(|(id, j)| reported_name(id, j) == name)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(by.len(), 1, "qa_r662: `{name}` is reported by {by:?}");
    w.results[by[0]].clone()
}

/// `ci.yml`'s required checks: the PR tier's six, the owner's CI model (9 Nov 2026), in place of R-656's `ci`,
/// `xtask-ci`, `gpu-kernel`, `gpu-metal` and `gpu-lavapipe` gates over the full tier.
const GATES: [&str; 6] = ["fmt", "clippy", "build", "ci", "gpu-metal", "gpu-lavapipe"];

/// The status functions a step's `if` holds read as a run with no failed step, and any other expression still stops
/// the walk: it must not read as a false that hides a step.
fn check_walk_reads_status(walk: &Walk, unknown: &[&str]) {
    assert_eq!(walk.eval("always()"), "true", "qa_r662: `always()`");
    assert_eq!(walk.eval("failure()"), "false", "qa_r662: `failure()`");
    let mut outputs = BTreeMap::new();
    outputs.insert("docs_only".to_owned(), "false".to_owned());
    let mut seen = Walk::default();
    seen.outputs.insert("changes".to_owned(), outputs);
    let upload = "always() && needs.changes.outputs.docs_only != 'true'";
    assert_eq!(seen.eval(upload), "true", "qa_r662: `{upload}`");
    for expr in unknown {
        let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| walk.eval(expr)));
        assert!(read.is_err(), "qa_r662: the walk read `{expr}`");
    }
}

#[test]
fn qa_r662_walk_reads_status_and_refuses_the_rest() {
    check_walk_reads_status(
        &Walk::default(),
        &[
            "success()",
            "cancelled()",
            "hashFiles('x')",
            "always() && cancelled()",
        ],
    );
}

negative_control!(
    qa_r662_walk_reads_status_and_refuses_the_rest,
    "a walk taken to refuse what it knows",
    expected = "qa_r662: the walk read `failure()`",
    check_walk_reads_status(&Walk::default(), &["failure()"])
);

/// The commands the jobs that ran in `w` run, as (job id, command): each step's `if:` evaluated with its job's own
/// `env:`. Until the owner's CI model (9 Nov 2026) this read `ci-workspace` alone, which ran the cheap checks; they run
/// in the PR tier's `build` now, the product tests in `tests` (R-290's exception, applied per R-369).
fn pr_runs(yml: &Value, w: &Walk) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (id, job) in yml["jobs"].as_object().expect("qa_r662: jobs") {
        if w.results.get(id).is_none_or(|r| r == "skipped") {
            continue;
        }
        let mut jw = w.clone();
        if let Some(env) = job["env"].as_object() {
            for (k, v) in env {
                let raw = s(v);
                let value = if raw.contains("${{") {
                    jw.eval(&raw)
                } else {
                    raw
                };
                jw.env.insert(k.clone(), value);
            }
        }
        for step in job["steps"].as_array().expect("qa_r662: steps") {
            if step.get("run").is_some() && step.get("if").is_none_or(|c| jw.eval(&s(c)) == "true")
            {
                out.push((id.clone(), s(&step["run"])));
            }
        }
    }
    out
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

/// Whether the job `id`'s failure is a red required check on `verdict`: it bears one, or, as the split's shards do
/// through `tests-verdict` (11 Nov 2026), a run where it alone fails turns one of them from success to failure.
fn reaches_gate(yml: &Value, verdict: (bool, bool), id: &str) -> bool {
    if GATES.contains(&reported_name(id, &yml["jobs"][id]).as_str()) {
        return true;
    }
    let clean = walk(yml, verdict, &[]);
    let failed = walk(yml, verdict, &[id]);
    GATES.iter().any(|gate| {
        check_result(yml, &clean, gate) == "success"
            && check_result(yml, &failed, gate) == "failure"
    })
}

fn check_required(ci: &str, mutants: &str) {
    let yml = parse(ci);
    // The cheap path: every gate reports success, and nothing but `changes` and the PR tier's required jobs, which skip
    // their work inside, ran.
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
            "build",
            "changes",
            "clippy",
            "fmt",
            "gpu-lavapipe-smoke",
            "gpu-metal-smoke",
            "tests",
            "tests-verdict"
        ],
        "qa_r662: the cheap path ran other jobs"
    );
    let runs = pr_runs(&yml, &cheap);
    for want in ["python3 plan/check_plan.py", "cargo xtask lint vocab"] {
        let by = runs.iter().find(|(_, r)| r.trim() == want);
        assert!(
            by.is_some(),
            "qa_r662: the cheap path does not run `{want}`:\n{runs:#?}"
        );
        // A failed cheap check is a red required check: the job running it bears one, or its failure turns one red.
        let (id, _) = by.expect("checked");
        assert!(
            reaches_gate(&yml, (true, false), id),
            "qa_r662: the cheap path's `{want}` runs in `{id}`, which bears no required check"
        );
    }
    for (id, run) in &runs {
        for never in [
            "nextest",
            "cargo test",
            "cargo build",
            "clippy",
            "cargo fmt",
            "cargo doc",
        ] {
            assert!(
                !run.contains(never),
                "qa_r662: the cheap path runs `{never}` in `{id}`:\n{run}"
            );
        }
    }
    // The other side: on code every PR-tier job runs, and runs what the cheap path skipped, and the gates pass; the
    // full tier's jobs run on `main` alone, the process tests on a change to the machinery alone.
    let full = walk(&yml, (false, true), &[]);
    for gate in GATES {
        assert_eq!(
            check_result(&yml, &full, gate),
            "success",
            "qa_r662: the required check `{gate}` is not a success on code"
        );
    }
    let ran: Vec<&str> = full
        .results
        .iter()
        .filter(|(_, r)| r.as_str() != "skipped")
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        ran,
        vec![
            "build",
            "changes",
            "clippy",
            "fmt",
            "gpu-lavapipe-smoke",
            "gpu-metal-smoke",
            "tests",
            "tests-verdict",
            "web-skeleton"
        ],
        "qa_r662: a code pull request skipped a PR-tier job or ran another: {:?}",
        full.results
    );
    let runs = pr_runs(&yml, &full);
    for want in [
        "cargo fmt --all --check",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo build --workspace",
        "cargo nextest run $PACKAGES",
        "cargo xtask build-kernel",
    ] {
        // The changed crates' tests, whole or, since the split (PR #495, 11 Nov 2026), exactly as the shards'
        // partitions, whose covering every test once `qa_ci_split_mesa_rerun.rs` checks.
        let sharded = if want == "cargo nextest run $PACKAGES" {
            format!("{want} --partition hash:${{{{ matrix.shard }}}}/3")
        } else {
            want.to_owned()
        };
        assert!(
            runs.iter()
                .any(|(_, r)| r.trim() == want || r.trim() == sharded),
            "qa_r662: a code pull request does not run `{want}`:\n{runs:#?}"
        );
        // Each runs where its failure is a red required check.
        for (id, _) in runs
            .iter()
            .filter(|(_, r)| r.trim() == want || r.trim() == sharded)
        {
            assert!(
                reaches_gate(&yml, (false, true), id),
                "qa_r662: a code pull request's `{want}` runs in `{id}`, whose failure turns no required check red"
            );
        }
    }
    // A classification that failed is no cheap path: every gate turns red, and none reports a skip.
    let lost = walk(&yml, (true, false), &["changes"]);
    for gate in GATES {
        assert_eq!(
            check_result(&yml, &lost, gate),
            "failure",
            "qa_r662: the required check `{gate}` is not red when the classification failed"
        );
    }
    // The split (11 Nov 2026): `ci`'s product tests run in shards, and a shard that fails turns `ci` red, on code and on
    // the cheap path alike; a shard that never ran (skipped) does too.
    for verdict in [(false, true), (true, false)] {
        let shard_failed = walk(&yml, verdict, &["tests"]);
        assert_eq!(
            check_result(&yml, &shard_failed, "ci"),
            "failure",
            "qa_r662: the required check `ci` is not red when a shard failed ({verdict:?})"
        );
    }
    // Retired with the full tier from the pull request's path (the owner's CI model, 9 Nov 2026): "a code change
    // whose shards were skipped is red", which read `ci-archive` under the gate `ci`. No pull request builds an archive
    // now; `ci`'s tests are built in the job that reports it.

    // `mutants-check`: R-656's merge-ready verdict and R-678's job of every run were held here. The owner's CI model
    // (9 Nov 2026) takes the per-PR mutation run and `mutants-check` off the pull request's path, so what is kept is
    // that no mutants work runs on a pull request, the cheap path's included: `mutants.yml` has no `pull_request`
    // trigger. R-290's exception, applied per R-369.
    let mutants = parse(mutants);
    assert!(
        mutants["on"].get("pull_request").is_none()
            && mutants["on"].get("pull_request_target").is_none(),
        "qa_r662: mutants.yml runs on a pull request"
    );

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
    "ci.yml with `build` skipped at job level on the cheap path",
    expected = "the required check `build` is not a success on the cheap path",
    check_required(
        &swap(
            &read(".github/workflows/ci.yml"),
            "  build:\n    needs: changes\n    if: ${{ !cancelled() && github.event_name == 'pull_request' }}\n",
            "  build:\n    needs: changes\n    if: needs.changes.outputs.docs_only != 'true'\n"
        ),
        &read(".github/workflows/mutants.yml")
    )
);

// `mutants_check_skipped_with_its_verdict` and `mutants_check_ignores_its_verdict`, the controls of `mutants-check`'s
// job, are retired with it from the pull request's path (the owner's CI model, 9 Nov 2026).
mod a_lost_classification_passed {
    use super::*;
    negative_control!(
        qa_r662_required_checks_report_on_the_cheap_path,
        "ci.yml's `fmt` passing whatever `changes` ended in",
        expected = "the required check `fmt` is not red when the classification failed",
        check_required(
            &swap(
                &read(".github/workflows/ci.yml"),
                "        run: '[ \"$CHANGES\" = success ] || { echo \"changes ended $CHANGES: no verdict to skip on\"; exit 1; }'\n      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n        if: needs.changes.outputs.docs_only != 'true'\n      # The toolchain",
                "        run: 'true'\n      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n        if: needs.changes.outputs.docs_only != 'true'\n      # The toolchain"
            ),
            &read(".github/workflows/mutants.yml")
        )
    );
}

mod a_build_on_the_cheap_path {
    use super::*;
    negative_control!(
        qa_r662_required_checks_report_on_the_cheap_path,
        "ci.yml building the workspace on every pull request",
        expected = "the cheap path runs `cargo build` in `build`",
        check_required(
            &swap(
                &read(".github/workflows/ci.yml"),
                "      - name: cargo build --workspace\n        if: needs.changes.outputs.docs_only != 'true'\n",
                "      - name: cargo build --workspace\n"
            ),
            &read(".github/workflows/mutants.yml")
        )
    );
}

mod mutants_on_a_pull_request {
    use super::*;
    negative_control!(
        qa_r662_required_checks_report_on_the_cheap_path,
        "mutants.yml triggered by pull requests again",
        expected = "mutants.yml runs on a pull request",
        check_required(
            &read(".github/workflows/ci.yml"),
            &swap(
                &read(".github/workflows/mutants.yml"),
                "on:\n  workflow_dispatch:\n",
                "on:\n  workflow_dispatch:\n  pull_request:\n"
            )
        )
    );
}

mod a_verdict_that_ignores_its_shards {
    use super::*;
    negative_control!(
        qa_r662_required_checks_report_on_the_cheap_path,
        "ci.yml's `ci` (`tests-verdict`) passing whatever its shards ended in",
        expected = "runs in `tests`, whose failure turns no required check red",
        check_required(
            &swap(
                &read(".github/workflows/ci.yml"),
                "        run: '[ \"$SHARDS\" = success ] || { echo \"the product test shards ended $SHARDS: every shard must pass\"; exit 1; }'\n",
                "        run: 'true'\n"
            ),
            &read(".github/workflows/mutants.yml")
        )
    );
}

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
/// Principia's (R-662); mutants run only when `crates/` source changes (R-677), so only a `.rs` under `crates/`, one
/// beside the others above all, reaches the gate.
const SHARD_CASES: &[(&[&str], bool)] = &[
    (&[], false),
    (&["docs/qa_note.md"], false),
    (&["docs/reference/qa/a.rs"], false),
    (&["docs/\u{e9}/a.rs"], false),
    (&["plan/qa/a.rs"], false),
    (&["scripts/qa/a.rs"], false),
    (&["workbench/qa a.rs"], false),
    (&[".claude/qa/a.rs"], false),
    (&["docs/reference/qa/a.rs", "workbench/qa/b.rs"], false),
    (&["crates/kernel/src/qa.rs"], true),
    (&["crates/gui/src/qa.rs"], true),
    // R-677: Rust outside `crates/` has no per-PR mutants (the nightly run mutates xtask).
    (&["xtask/src/qa.rs"], false),
    (&["crates/kernel/src/q a.rs"], true),
    (&["crates/kernel/src/\u{e9}.rs"], true),
    // A listed name that isn't the top of the path.
    (&["crates/docs/qa.rs"], true),
    (&["crates/kernel/plan/qa.rs"], true),
    (&["docsx/qa.rs"], false),
    (&["newtop/qa.rs"], false),
    // `crates/` below another folder is not `crates/`.
    (&["xtask/crates/qa.rs"], false),
    // Mixed, the Rust under `crates/` on either side of the others in the diff's order.
    (&["crates/kernel/src/qa.rs", "docs/reference/qa/a.rs"], true),
    (&[".claude/qa/a.rs", "crates/kernel/src/qa.rs"], true),
    (&["docs/reference/qa/a.rs", "zz/crates/qa.rs"], false),
    (
        &["docs/reference/qa/a.rs", "zz/qa.rs", "crates/zz/qa.rs"],
        true,
    ),
    (
        &["workbench/qa/a.rs", "xtask/src/qa.rs", "plan/qa/a.rs"],
        false,
    ),
    (
        &[
            "workbench/qa/a.rs",
            "xtask/src/qa.rs",
            "crates/render/src/qa.rs",
            "plan/qa/a.rs",
        ],
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
        // The diff's verdict alone, or joined by `&&` to more (the Mesa cache's own skip, 11 Nov 2026), which can
        // only hold a step back further.
        let cond = s(&step["if"]);
        let gate = "steps.diff.outputs.rust == 'true'";
        assert!(
            cond == gate
                || cond
                    .strip_prefix(gate)
                    .is_some_and(|rest| rest.starts_with(" && ") && !rest.contains("||")),
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
    "mutants.yml running a shard on xtask's Rust as well as `crates/`'s (R-677)",
    expected = "a shard on [\"xtask/src/qa.rs\"]: rust is not false",
    check_shard(&swap(
        &read(".github/workflows/mutants.yml"),
        "\"?b/crates/.*\\.rs",
        "\"?b/(crates|xtask)/.*\\.rs"
    ))
);

mod a_mesa_step_off_the_diff {
    use super::*;
    negative_control!(
        qa_r662_mutants_shard_skips_only_rust_not_principias,
        "mutants.yml's apt step for Mesa waiting on the Mesa cache alone, not on the diff's verdict",
        expected = "the shard's step `Install Mesa (lavapipe)` does not wait on the diff's verdict",
        check_shard(&swap(
            &read(".github/workflows/mutants.yml"),
            "        if: steps.diff.outputs.rust == 'true' && steps.mesa.outputs.installed != 'true'\n",
            "        if: steps.mesa.outputs.installed != 'true'\n"
        ))
    );
}

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

/// `detect`'s verdict `(docs_only, gpu, tier)`.
fn detect(script: &Path, repo: &Path, sha: &str) -> (bool, bool, String) {
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
    let text = std::fs::read_to_string(&output).expect("qa_r662: detect wrote no GITHUB_OUTPUT");
    let (docs_only, gpu) = flags(&text);
    (docs_only, gpu, value(&text, "tier"))
}

/// What a pull request does to the tree.
enum Pr {
    Add(&'static [&'static str]),
    Delete(&'static str),
    Move(&'static str),
    Nothing,
}

/// (what, the pull request, `(docs_only, gpu, tier)` as the ruling gives it).
const DETECT_CASES: &[(&str, Pr, (bool, bool, &str))] = &[
    (
        "the plan's Python, a script, the workbench and the root's markdown",
        Pr::Add(&[
            "plan/tools/qa.py",
            "scripts/qa.py",
            "workbench/qa/a.html",
            "decisions.md",
            "docs/qa/b.pdf",
        ]),
        (true, false, "docs"),
    ),
    (
        "`.claude/`'s agents, commands and settings",
        Pr::Add(&[
            ".claude/agents/qa.md",
            ".claude/commands/qa.md",
            ".claude/settings.json",
        ]),
        (true, false, "docs"),
    ),
    (
        "`.claude/` beside a crate's file",
        Pr::Add(&[".claude/agents/qa.md", "crates/kernel/src/qa.rs"]),
        (false, true, "code"),
    ),
    (
        "a name that only starts like `.claude/`",
        Pr::Add(&[".clauderc"]),
        (false, true, "code"),
    ),
    (
        "a non-ASCII name in the docs",
        Pr::Add(&["docs/\u{e9}.md"]),
        (true, false, "docs"),
    ),
    (
        "a crate's file moved into the docs",
        Pr::Move("docs/moved.rs"),
        (false, true, "code"),
    ),
    (
        "a crate's file deleted",
        Pr::Delete("crates/kernel/src/base.rs"),
        (false, true, "code"),
    ),
    (
        "a merge that changes nothing",
        Pr::Nothing,
        (false, true, "code"),
    ),
    (
        "a non-ASCII name in a crate",
        Pr::Add(&["crates/kernel/src/\u{e9}.rs"]),
        (false, true, "code"),
    ),
    (
        "a name git must quote",
        Pr::Add(&["docs/a\tb.md"]),
        (false, true, "code"),
    ),
    (
        "an unknown top-level folder",
        Pr::Add(&["newtop/a.md"]),
        (false, true, "code"),
    ),
    (
        "the classifier",
        Pr::Add(&["scripts/ci_changes.sh"]),
        (false, true, "code"),
    ),
    (
        "a task file",
        Pr::Add(&["plan/tasks/M9/TASK-M9-98.md"]),
        (true, false, "data"),
    ),
    (
        "the requirements beside prose",
        Pr::Add(&["plan/requirements.yaml", "plan/qa.md"]),
        (false, true, "code"),
    ),
    (
        "a workflow",
        Pr::Add(&[".github/workflows/qa.yml"]),
        (false, true, "code"),
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
        let (docs_only, gpu, tier) = want;
        assert_eq!(
            detect(&file, &repo, &sha),
            (*docs_only, *gpu, (*tier).to_owned()),
            "qa_r662: detect on {what} (docs_only, gpu, tier)"
        );
    }
    // A commit that can't be read: the diff can't be computed.
    let repo = dir.join("gone");
    base_repo(&repo);
    assert_eq!(
        detect(&file, &repo, "0123456789012345678901234567890123456789"),
        (false, true, "code".to_owned()),
        "qa_r662: detect on a commit that isn't there (docs_only, gpu, tier)"
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
