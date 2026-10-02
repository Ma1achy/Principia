//! QA tests for TASK-M0-23 under R-305, written from the ruling and REQ-VAL-148, not from the implementation:
//!
//! - R-305 (the human's words): "mutants-check becomes a required status check on main. First make sure it runs on
//!   every PR and passes trivially when the diff has no mutants (docs-only and rulings PRs), so no PR can wait on a
//!   check that never reports."
//! - REQ-VAL-148: "Every pull request must run `cargo mutants --in-diff` on its changed code in CI, sharded across
//!   parallel CI jobs (`--shard k/n`) ... The job must fail on any surviving mutant not in a checked-in list".
//! - A required check that a `push` run also creates, skipped, is counted by GitHub as passed; so no workflow that a
//!   trigger other than `pull_request` starts may carry a check named `mutants-check`.
//!
//! What these tests hold to:
//! - a PR whose diff changes no Rust source (docs-only, rulings-only) passes every shard without running
//!   cargo-mutants, and says so ("no mutant"); the aggregate `mutants-check` then runs and passes;
//! - a PR whose diff changes Rust source (an edit, a rename with an edit, a path with spaces, a quoted non-ASCII
//!   path, Rust among docs) runs `cargo mutants --in-diff <the PR's diff> --shard k/n` on every shard;
//! - a diff that cannot be computed fails the shard, never passes it as "no mutant";
//! - `mutants-check` is created only by `pull_request` runs, on every pull request (no path filter), and its `if:`
//!   holds whatever the shards' result.
//!
//! The workflows are parsed as YAML (PyYAML, as xtask's plan_check tests use), and each job is run here step by step
//! as a runner would: `if:` conditions evaluated (with the implicit `success()`), `${{ … }}` substituted, `run:`
//! scripts executed in bash as GitHub's default shell on ubuntu does, against a real git repository with an
//! `origin/main` base; `uses:` actions are stood in for (nothing), and `cargo` is a stand-in that records its
//! arguments (`cargo xtask` goes to the built xtask). An expression this evaluator does not understand fails the test
//! rather than being guessed. Each test's control (R-176) feeds the same assertion a workflow differing in the one
//! respect the requirement turns on, and trips it by its message.
// The file name `qa_TASK-M0-23_r305` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn workflow_text(name: &str) -> String {
    std::fs::read_to_string(root().join(".github/workflows").join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn mutants_yml() -> String {
    workflow_text("mutants.yml")
}

/// A fresh directory for one call: tests and their controls run in parallel, so each call gets its own. Deleted when
/// the test passes, kept with its path printed when it fails (R-342).
fn scratch(tag: &str) -> Scratch {
    let dir = Scratch::new(&format!("qa23r_{tag}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A workflow's text as JSON, parsed by PyYAML. YAML 1.1 reads the key `on` as `true`; it is put back as `on`.
fn parse(yml: &str) -> Value {
    let dir = scratch("yaml");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
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
        .expect("qa23r: python3 ran");
    assert!(
        out.status.success(),
        "qa23r: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa23r: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// A runner, in miniature: expressions, conditions and a job's steps.

#[derive(Clone)]
struct Ctx {
    event: String,
    shard: Option<String>,
    needs: HashMap<String, String>,
    steps: HashMap<String, (String, HashMap<String, String>)>,
    failed: bool,
}

impl Ctx {
    fn new(event: &str) -> Self {
        Ctx {
            event: event.to_owned(),
            shard: None,
            needs: HashMap::new(),
            steps: HashMap::new(),
            failed: false,
        }
    }

    fn value(&self, operand: &str) -> String {
        let o = operand.trim();
        if let Some(lit) = o.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
            return lit.to_owned();
        }
        match o {
            "github.event_name" => return self.event.clone(),
            "github.base_ref" => return "main".to_owned(),
            "runner.temp" => return "$RUNNER_TEMP".to_owned(),
            _ => {}
        }
        if let Some(var) = o.strip_prefix("matrix.") {
            assert_eq!(var, "shard", "qa23r: matrix variable not understood: {o}");
            return self.shard.clone().expect("qa23r: no shard in this job");
        }
        if let Some(rest) = o.strip_prefix("needs.") {
            let (job, field) = rest.split_once('.').expect("qa23r: needs.<job>.<field>");
            assert_eq!(field, "result", "qa23r: needs field not understood: {o}");
            return self.needs.get(job).cloned().unwrap_or_default();
        }
        if let Some(rest) = o.strip_prefix("steps.") {
            let (id, field) = rest.split_once('.').expect("qa23r: steps.<id>.<field>");
            let (outcome, outputs) = self.steps.get(id).cloned().unwrap_or_default();
            if field == "outcome" || field == "conclusion" {
                return outcome;
            }
            if let Some(key) = field.strip_prefix("outputs.") {
                return outputs.get(key).cloned().unwrap_or_default();
            }
        }
        panic!("qa23r: expression operand not understood: {o}");
    }

    fn atom(&self, a: &str) -> bool {
        let a = a.trim();
        assert!(
            !a.contains('(') || a.ends_with("()"),
            "qa23r: expression not understood: {a}"
        );
        match a {
            "always()" => return true,
            "success()" => return !self.failed,
            "failure()" => return self.failed,
            "cancelled()" => return false,
            "!cancelled()" => return true,
            _ => {}
        }
        if let Some((l, r)) = a.split_once("!=") {
            return self.value(l) != self.value(r);
        }
        if let Some((l, r)) = a.split_once("==") {
            return self.value(l) == self.value(r);
        }
        panic!("qa23r: expression not understood: {a}");
    }

    /// An `if:` as a runner reads it: without a status function, `success() && …` is implied.
    fn holds(&self, cond: Option<&Value>) -> bool {
        let raw = cond.map(text).unwrap_or_else(|| "success()".to_owned());
        let e = raw.trim();
        let e = e
            .strip_prefix("${{")
            .and_then(|x| x.strip_suffix("}}"))
            .unwrap_or(e)
            .trim();
        let status = ["always()", "success()", "failure()", "cancelled()"]
            .iter()
            .any(|f| e.contains(f));
        let v = e
            .split("||")
            .any(|conj| conj.split("&&").all(|a| self.atom(a)));
        if status {
            v
        } else {
            !self.failed && v
        }
    }

    /// A job's `if:`: its implicit `success()` is that every job it needs succeeded.
    fn job_holds(&self, cond: Option<&Value>) -> bool {
        let mut at_job = self.clone();
        at_job.failed = self.needs.values().any(|r| r != "success");
        at_job.holds(cond)
    }

    /// `text` with each `${{ expr }}` replaced by its value.
    fn expand(&self, text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(at) = rest.find("${{") {
            out.push_str(&rest[..at]);
            let end = rest[at..].find("}}").expect("qa23r: unclosed ${{") + at;
            out.push_str(&self.value(&rest[at + 3..end]));
            rest = &rest[end + 2..];
        }
        out.push_str(rest);
        out
    }
}

struct Env {
    cwd: PathBuf,
    temp: PathBuf,
    bin: PathBuf,
    calls: PathBuf,
    /// The scratch folder holding `temp`, `bin` and `calls`, handed to the job's run and held until its test ends.
    dir: Scratch,
}

/// A stand-in `cargo` in `bin`: `cargo xtask …` runs the built xtask; anything else is recorded, one call a line,
/// and exits 0 having written nothing (for `cargo mutants`: nothing to test).
fn env(tag: &str, cwd: &Path) -> Env {
    let dir = scratch(tag);
    let bin = dir.join("bin");
    let temp = dir.join("runner_temp");
    std::fs::create_dir_all(&bin).expect("bin");
    std::fs::create_dir_all(&temp).expect("temp");
    let calls = dir.join("cargo_calls");
    std::fs::write(&calls, "").expect("calls");
    write_executable(
        &bin.join("cargo"),
        format!(
            "#!/bin/sh\nif [ \"$1\" = xtask ]; then shift; exec \"{}\" \"$@\"; fi\n\
             printf '%s\\n' \"$*\" >> \"{}\"\nexit 0\n",
            env!("CARGO_BIN_EXE_xtask"),
            calls.display()
        ),
    )
    .expect("stand-in cargo");
    // The setup steps' installers, stood in: recorded, never run on the machine running the tests.
    for tool in [
        "sudo", "apt-get", "python3", "pip", "pip3", "rustup", "curl",
    ] {
        write_executable(
            &bin.join(tool),
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"{tool} $*\" >> \"{}\"\nexit 0\n",
                calls.display()
            ),
        )
        .expect("stand-in tool");
    }
    Env {
        cwd: cwd.to_path_buf(),
        temp,
        bin,
        calls,
        dir,
    }
}

struct JobRun {
    ran: bool,
    success: bool,
    log: String,
    calls: Vec<String>,
    /// The run's scratch folder, held until the test that made it ends.
    _scratch: Scratch,
}

/// Runs job `name` of `wf` as a runner would, under `ctx` (event, matrix shard, needs results).
fn run_job(wf: &Value, name: &str, mut ctx: Ctx, env: Env) -> JobRun {
    let job = &wf["jobs"][name];
    assert!(job.is_object(), "qa23r: no `{name}` job in the workflow");
    if !ctx.job_holds(job.get("if")) {
        return JobRun {
            ran: false,
            success: false,
            log: String::new(),
            calls: Vec::new(),
            _scratch: env.dir,
        };
    }
    let job_env: Vec<(String, String)> = job
        .get("env")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), text(v))).collect())
        .unwrap_or_default();
    let mut log = String::new();
    let steps = job["steps"].as_array().expect("qa23r: job has steps");
    for (i, step) in steps.iter().enumerate() {
        let id = step.get("id").map(text).unwrap_or_else(|| format!("__{i}"));
        if !ctx.holds(step.get("if")) {
            ctx.steps.insert(id, ("skipped".to_owned(), HashMap::new()));
            continue;
        }
        let Some(script) = step.get("run") else {
            ctx.steps.insert(id, ("success".to_owned(), HashMap::new()));
            continue;
        };
        let script = ctx.expand(&text(script));
        let output = env.temp.join(format!("github_output_{i}"));
        std::fs::write(&output, "").expect("GITHUB_OUTPUT");
        let path = std::env::var("PATH").unwrap_or_default();
        let mut cmd = Command::new("bash");
        cmd.args(["--noprofile", "--norc", "-eo", "pipefail", "-c", &script])
            .current_dir(&env.cwd)
            .env("RUNNER_TEMP", &env.temp)
            .env("GITHUB_OUTPUT", &output)
            .env("PATH", format!("{}:{path}", env.bin.display()))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1");
        for (k, v) in &job_env {
            cmd.env(k, v);
        }
        let out = cmd.timed_output().expect("qa23r: bash ran");
        log.push_str(&String::from_utf8_lossy(&out.stdout));
        log.push_str(&String::from_utf8_lossy(&out.stderr));
        let outputs: HashMap<String, String> = std::fs::read_to_string(&output)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let ok = out.status.success();
        let tolerated = step
            .get("continue-on-error")
            .is_some_and(|v| v == &Value::Bool(true));
        if !ok && !tolerated {
            ctx.failed = true;
        }
        let outcome = if ok { "success" } else { "failure" };
        ctx.steps.insert(id, (outcome.to_owned(), outputs));
    }
    let calls = std::fs::read_to_string(&env.calls)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    JobRun {
        ran: true,
        success: !ctx.failed,
        log,
        calls,
        _scratch: env.dir,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// A pull request, as a git repository whose `origin/main` is the base and whose HEAD is the PR.

enum Op {
    Write(&'static str, &'static str),
    Rename(&'static str, &'static str),
}

const LIB: &str = "pub fn a() -> u32 {\n    1\n}\n\npub fn b() -> u32 {\n    2\n}\n\npub fn c() -> u32 {\n    3\n}\n";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=qa",
            "-c",
            "user.email=qa@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .timed_output()
        .expect("qa23r: git ran");
    assert!(
        out.status.success(),
        "qa23r: git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The PR's repository: a base commit, `refs/remotes/origin/main` at it (unless `with_base` is false), and the PR's
/// changes committed on top.
fn pr(tag: &str, ops: &[Op], with_base: bool) -> Scratch {
    let dir = scratch(&format!("repo_{tag}"));
    let put = |rel: &str, body: &str| {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("dirs");
        std::fs::write(p, body).expect("file");
    };
    put("README.md", "# fixture\n");
    put("docs/guide.md", "A guide.\n");
    put("decisions.md", "# Decisions\n\n## R-1\nA ruling.\n");
    put(
        "plan/requirements.yaml",
        "- id: REQ-X-001\n  statement: x\n",
    );
    put("src/lib.rs", LIB);
    put("src/old.rs", LIB);
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    if with_base {
        git(&dir, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    }
    for op in ops {
        match op {
            Op::Write(rel, body) => {
                let p = dir.join(rel);
                std::fs::create_dir_all(p.parent().expect("parent")).expect("dirs");
                let old = std::fs::read_to_string(&p).unwrap_or_default();
                std::fs::write(&p, old + body).expect("write");
            }
            Op::Rename(from, to) => {
                std::fs::rename(dir.join(from), dir.join(to)).expect("rename");
            }
        }
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "the PR"]);
    dir
}

/// The shard job's matrix: every k, and n.
fn matrix(wf: &Value) -> Vec<String> {
    let ks: Vec<String> = wf["jobs"]["mutants"]["strategy"]["matrix"]["shard"]
        .as_array()
        .expect("qa23r: the mutants job has no shard matrix")
        .iter()
        .map(text)
        .collect();
    assert!(ks.len() >= 2, "qa23r: fewer than two shards: {ks:?}");
    ks
}

/// Every shard of the PR run in `repo`: per shard, the job's run.
fn shards_on(yml: &str, repo: &Path, tag: &str) -> Vec<(String, JobRun)> {
    let wf = parse(yml);
    matrix(&wf)
        .into_iter()
        .map(|k| {
            let mut ctx = Ctx::new("pull_request");
            ctx.shard = Some(k.clone());
            let e = env(&format!("{tag}_shard{k}"), repo);
            let run = run_job(&wf, "mutants", ctx, e);
            (k, run)
        })
        .collect()
}

fn mutants_calls(run: &JobRun) -> Vec<&String> {
    run.calls
        .iter()
        .filter(|c| c.split_whitespace().next() == Some("mutants"))
        .collect()
}

// ---------------------------------------------------------------------------------------------------------------
// R-305: a PR with no mutant passes every shard, without cargo-mutants, and mutants-check reports and passes.

/// The mutants-check job on an `event`, run from the workspace root after the shards whose job result is `result`,
/// with no shard report downloaded.
fn aggregate(yml: &str, result: &str, tag: &str, event: &str) -> JobRun {
    let wf = parse(yml);
    let mut ctx = Ctx::new(event);
    ctx.needs.insert("mutants".to_owned(), result.to_owned());
    let e = env(&format!("agg_{tag}"), &root());
    run_job(&wf, "mutants-check", ctx, e)
}

fn no_mutant_passes(yml: &str, tag: &str, ops: &[Op]) {
    let repo = pr(tag, ops, true);
    let runs = shards_on(yml, &repo, tag);
    for (k, run) in &runs {
        assert!(
            run.ran && run.success,
            "qa23r: shard {k} of a PR with no Rust change did not pass: {}",
            run.log
        );
        assert!(
            mutants_calls(run).is_empty(),
            "qa23r: cargo mutants ran on a diff with no Rust source (shard {k}): {:?}",
            run.calls
        );
        assert!(
            run.log.to_lowercase().contains("no mutant"),
            "qa23r: shard {k} passed without saying the diff has no mutant: {}",
            run.log
        );
    }
    let result = if runs.iter().all(|(_, r)| r.success) {
        "success"
    } else {
        "failure"
    };
    let agg = aggregate(yml, result, tag, "pull_request");
    assert!(
        agg.ran && agg.success,
        "qa23r: mutants-check did not report a pass on a PR with no mutant (ran: {}): {}",
        agg.ran,
        agg.log
    );
}

#[test]
fn qa23r_a_docs_only_pr_passes_without_cargo_mutants() {
    no_mutant_passes(
        &mutants_yml(),
        "docs",
        &[
            Op::Write("docs/guide.md", "More of the guide.\n"),
            Op::Write("docs/new.md", "A new page.\n"),
            Op::Write(
                "docs/lib.rs.md",
                "A page whose name holds `.rs` but is markdown.\n",
            ),
        ],
    );
}

#[test]
fn qa23r_a_rulings_only_pr_passes_without_cargo_mutants() {
    no_mutant_passes(
        &mutants_yml(),
        "rulings",
        &[
            Op::Write("decisions.md", "\n## R-2\nAnother ruling.\n"),
            Op::Write(
                "plan/requirements.yaml",
                "- id: REQ-X-002\n  statement: y\n",
            ),
        ],
    );
}

negative_control!(
    qa23r_a_docs_only_pr_passes_without_cargo_mutants,
    "a workflow whose diff step always reports Rust source",
    expected = "qa23r: cargo mutants ran on a diff with no Rust source",
    no_mutant_passes(
        &mutants_yml().replace("echo \"rust=false\"", "echo \"rust=true\""),
        "docs_ctl",
        &[Op::Write("docs/guide.md", "More of the guide.\n")],
    )
);

mod aggregate_needs_a_report {
    use super::*;
    negative_control!(
        qa23r_a_rulings_only_pr_passes_without_cargo_mutants,
        "an aggregate that fails when no shard uploaded a report",
        expected = "qa23r: mutants-check did not report a pass on a PR with no mutant",
        no_mutant_passes(
            &mutants_yml().replace(
                "echo \"mutants-check: no shard tested a mutant of this PR's diff\"",
                "echo \"mutants-check: no report\"; fail=1"
            ),
            "rulings_ctl",
            &[Op::Write("decisions.md", "\n## R-2\nAnother ruling.\n")],
        )
    );
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-148: a PR that changes Rust source runs cargo mutants on its diff, on every shard.

/// Each shard runs `cargo mutants --in-diff <file> --shard k/n` once, the file being the PR's diff, which names
/// `changed` (as git writes it).
fn rust_change_runs(yml: &str, tag: &str, ops: &[Op], changed: &str) {
    let repo = pr(tag, ops, true);
    let runs = shards_on(yml, &repo, tag);
    let n = runs.len();
    for (k, run) in &runs {
        let calls = mutants_calls(run);
        assert!(
            calls.len() == 1,
            "qa23r: a Rust change did not run cargo mutants once on shard {k}: {:?}\n{}",
            run.calls,
            run.log
        );
        let args: Vec<&str> = calls[0].split_whitespace().collect();
        let after = |flag: &str| {
            args.iter()
                .position(|a| *a == flag)
                .and_then(|i| args.get(i + 1).copied())
        };
        assert!(
            after("--shard") == Some(format!("{k}/{n}").as_str()),
            "qa23r: shard {k} did not run as --shard {k}/{n}: {}",
            calls[0]
        );
        let diff = after("--in-diff")
            .map(|f| std::fs::read_to_string(f).unwrap_or_default())
            .unwrap_or_default();
        assert!(
            diff.contains(changed),
            "qa23r: shard {k}'s --in-diff is not the PR's diff naming {changed}: {diff}"
        );
    }
}

#[test]
fn qa23r_a_rust_change_runs_every_shard() {
    let yml = mutants_yml();
    rust_change_runs(
        &yml,
        "edit",
        &[Op::Write("src/lib.rs", "\npub fn d() -> u32 {\n    4\n}\n")],
        "src/lib.rs",
    );
    rust_change_runs(
        &yml,
        "rename",
        &[
            Op::Rename("src/old.rs", "src/renamed.rs"),
            Op::Write("src/renamed.rs", "\npub fn e() -> bool {\n    true\n}\n"),
        ],
        "src/renamed.rs",
    );
    rust_change_runs(
        &yml,
        "spaces",
        &[Op::Write(
            "src/my dir/a b.rs",
            "pub fn f() -> u32 {\n    5\n}\n",
        )],
        "my dir/a b.rs",
    );
    rust_change_runs(
        &yml,
        "non_ascii",
        &[Op::Write(
            "src/donn\u{e9}es.rs",
            "pub fn g() -> u32 {\n    6\n}\n",
        )],
        "donn",
    );
    rust_change_runs(
        &yml,
        "mixed",
        &[
            Op::Write("docs/guide.md", "More.\n"),
            Op::Write("src/lib.rs", "\npub fn h() -> u32 {\n    7\n}\n"),
            Op::Write("decisions.md", "\n## R-3\nA third.\n"),
        ],
        "src/lib.rs",
    );
}

negative_control!(
    qa23r_a_rust_change_runs_every_shard,
    "a fast path that reads only unquoted paths without spaces",
    expected = "qa23r: a Rust change did not run cargo mutants once on shard 0",
    rust_change_runs(
        &mutants_yml().replace(
            r#"'^\+\+\+ .*\.rs"?[[:space:]]*$'"#,
            r#"'^\+\+\+ b/[^ "]*\.rs$'"#
        ),
        "spaces_ctl",
        &[Op::Write(
            "src/my dir/a b.rs",
            "pub fn f() -> u32 {\n    5\n}\n"
        )],
        "my dir/a b.rs",
    )
);

mod skips_everything {
    use super::*;
    negative_control!(
        qa23r_a_rust_change_runs_every_shard,
        "a workflow whose diff step never reports Rust source",
        expected = "qa23r: a Rust change did not run cargo mutants once on shard 0",
        rust_change_runs(
            &mutants_yml().replace("echo \"rust=true\"", "echo \"rust=false\""),
            "edit_ctl",
            &[Op::Write("src/lib.rs", "\npub fn d() -> u32 {\n    4\n}\n")],
            "src/lib.rs",
        )
    );
}

/// A diff that cannot be computed (no base) is not "no mutant": every shard fails.
fn no_diff_fails(yml: &str) {
    let repo = pr(
        "nobase",
        &[Op::Write("src/lib.rs", "\npub fn d() -> u32 {\n    4\n}\n")],
        false,
    );
    for (k, run) in shards_on(yml, &repo, "nobase") {
        assert!(
            run.ran && !run.success,
            "qa23r: shard {k} passed on a diff it could not compute: {}",
            run.log
        );
    }
}

#[test]
fn qa23r_a_diff_that_cannot_be_computed_fails_the_shard() {
    no_diff_fails(&mutants_yml());
}

negative_control!(
    qa23r_a_diff_that_cannot_be_computed_fails_the_shard,
    "a diff step that ignores git diff's failure",
    expected = "qa23r: shard 0 passed on a diff it could not compute",
    no_diff_fails(&mutants_yml().replace(
        "HEAD\" > \"$RUNNER_TEMP/pr.diff\"",
        "HEAD\" > \"$RUNNER_TEMP/pr.diff\" || true"
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// R-305: mutants-check is created only by pull_request runs, and on every pull request.

/// The triggers of a workflow's `on:` (a string, a list or a map).
fn triggers(wf: &Value) -> Vec<(String, Value)> {
    match &wf["on"] {
        Value::String(s) => vec![(s.clone(), Value::Null)],
        Value::Array(a) => a.iter().map(|v| (text(v), Value::Null)).collect(),
        Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        other => panic!("qa23r: `on:` not understood: {other}"),
    }
}

/// Over every workflow `(file, text)`: each job whose check is named `mutants-check` is in a workflow only
/// `pull_request` starts, with no filter that skips some pull requests; there is at least one; and its `if:` holds on a
/// pull request whatever the jobs it needs ended in.
fn only_pull_requests_create_it(workflows: &[(String, String)]) {
    let mut found = 0;
    for (file, yml) in workflows {
        let wf = parse(yml);
        let Some(jobs) = wf["jobs"].as_object() else {
            continue;
        };
        for (id, job) in jobs {
            let check = job.get("name").map(text).unwrap_or_else(|| id.clone());
            if check != "mutants-check" {
                continue;
            }
            found += 1;
            let on = triggers(&wf);
            let names: Vec<&str> = on.iter().map(|(k, _)| k.as_str()).collect();
            assert!(
                names == ["pull_request"],
                "qa23r: a mutants-check job ({file}: {id}) is reached by a trigger other than pull_request: {names:?}"
            );
            let filters = &on[0].1;
            for key in ["paths", "paths-ignore", "branches-ignore"] {
                assert!(
                    filters.get(key).is_none(),
                    "qa23r: {file}'s pull_request trigger does not run on every pull request ({key})"
                );
            }
            if let Some(branches) = filters.get("branches") {
                assert!(
                    branches
                        .as_array()
                        .is_some_and(|b| b.iter().any(|x| x == "main" || x == "**")),
                    "qa23r: {file}'s pull_request trigger does not run on every pull request to main"
                );
            }
            if let Some(types) = filters.get("types").and_then(Value::as_array) {
                for t in ["opened", "synchronize", "reopened"] {
                    assert!(
                        types.iter().any(|x| x == t),
                        "qa23r: {file}'s pull_request trigger does not run on every pull request ({t})"
                    );
                }
            }
            let needs: Vec<String> = match job.get("needs") {
                Some(Value::Array(a)) => a.iter().map(text).collect(),
                Some(v) => vec![text(v)],
                None => Vec::new(),
            };
            for result in ["success", "failure", "cancelled", "skipped"] {
                let mut ctx = Ctx::new("pull_request");
                for n in &needs {
                    ctx.needs.insert(n.clone(), result.to_owned());
                }
                assert!(
                    ctx.job_holds(job.get("if")),
                    "qa23r: mutants-check does not report when the shards' result is {result}"
                );
            }
        }
    }
    assert!(found > 0, "qa23r: no workflow has a mutants-check job");
}

fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("workflows")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".yml") || n.ends_with(".yaml"))
        .map(|n| {
            let t = workflow_text(&n);
            (n, t)
        })
        .collect();
    out.sort();
    out
}

#[cfg(feature = "controls")]
fn with(file: &str, edit: impl Fn(&str) -> String) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(n, t)| if n == file { (n, edit(&t)) } else { (n, t) })
        .collect()
}

#[test]
fn qa23r_mutants_check_is_created_only_by_pull_request_runs() {
    only_pull_requests_create_it(&workflows());
}

negative_control!(
    qa23r_mutants_check_is_created_only_by_pull_request_runs,
    "mutants.yml also started by push",
    expected =
        "qa23r: a mutants-check job (mutants.yml: mutants-check) is reached by a trigger other",
    only_pull_requests_create_it(&with("mutants.yml", |t| t.replacen(
        "on:\n  pull_request:\n",
        "on:\n  push:\n  pull_request:\n",
        1
    )))
);

mod in_ci_yml {
    use super::*;
    negative_control!(
        qa23r_mutants_check_is_created_only_by_pull_request_runs,
        "a mutants-check job in ci.yml, which push starts",
        expected =
            "qa23r: a mutants-check job (ci.yml: mutants-check) is reached by a trigger other",
        only_pull_requests_create_it(&with("ci.yml", |t| {
            format!(
            "{t}\n  mutants-check:\n    if: github.event_name == 'pull_request'\n    runs-on: ubuntu-latest\n    \
             steps:\n      - run: \"true\"\n"
        )
        }))
    );
}

mod path_filtered {
    use super::*;
    negative_control!(
        qa23r_mutants_check_is_created_only_by_pull_request_runs,
        "mutants.yml skipped on docs-only pull requests",
        expected = "qa23r: mutants.yml's pull_request trigger does not run on every pull request (paths-ignore)",
        only_pull_requests_create_it(&with("mutants.yml", |t| t.replacen(
            "on:\n  pull_request:\n",
            "on:\n  pull_request:\n    paths-ignore: ['**.md']\n",
            1
        )))
    );
}

mod skipped_after_a_failed_shard {
    use super::*;
    negative_control!(
        qa23r_mutants_check_is_created_only_by_pull_request_runs,
        "mutants-check without always()",
        expected = "qa23r: mutants-check does not report when the shards' result is failure",
        only_pull_requests_create_it(&with("mutants.yml", |t| t.replace(
            "if: always() && github.event_name == 'pull_request'",
            "if: github.event_name == 'pull_request'"
        )))
    );
}

/// The whole aggregate job, as a runner runs it: with no shard report it runs and passes when the shards passed, and
/// fails when they did not; on a push event it never runs.
fn aggregate_reports(yml: &str) {
    let ok = aggregate(yml, "success", "clean", "pull_request");
    assert!(
        ok.ran && ok.success,
        "qa23r: mutants-check did not pass with no report and every shard passed: {}",
        ok.log
    );
    for result in ["failure", "cancelled"] {
        let bad = aggregate(yml, result, result, "pull_request");
        assert!(
            bad.ran && !bad.success,
            "qa23r: mutants-check passed though the shards' result is {result}: {}",
            bad.log
        );
    }
    let push = aggregate(yml, "", "push", "push");
    assert!(!push.ran, "qa23r: mutants-check runs on a push event");
}

#[test]
fn qa23r_the_aggregate_reports_on_a_pr_with_no_report() {
    aggregate_reports(&mutants_yml());
}

negative_control!(
    qa23r_the_aggregate_reports_on_a_pr_with_no_report,
    "an aggregate that ignores the shards' result when there is no report",
    expected = "qa23r: mutants-check passed though the shards' result is failure",
    aggregate_reports(&mutants_yml().replace(
        "if [ \"${{ needs.mutants.result }}\" != success ]; then",
        "if false; then"
    ))
);
