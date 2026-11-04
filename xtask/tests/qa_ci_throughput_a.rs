//! qa's tests for the merge-ready mutants gate and the per-pull-request `reviews` cancel group (tooling PR #379,
//! `ci-throughput-a`), written from the rulings and the owner's words, not from the workflows' code:
//!
//! - R-656: "mutants run on the merge-ready head"; "a pull request's superseded runs are cancelled, never a run on
//!   `main`".
//! - The owner's CI plan of 2 Nov 2026: "The FULL suite plus mutants runs once on the merge-ready head (all required
//!   approvals present, or a ready-to-merge label your merge watcher applies), and again only if that head changes.
//!   Branch protection must still require the full checks on the merged head, so required check names must report
//!   correctly in both tiers (skip-as-success only where the tier logic allows it)"; "cancel superseded `reviews` runs
//!   per PR".
//! - R-305: "mutants-check becomes a required status check on main. First make sure it runs on every PR and passes
//!   trivially when the diff has no mutants (docs-only and rulings PRs), so no PR can wait on a check that never
//!   reports." R-662: a docs-only change runs none of the mutants work, its required checks still reporting.
//!
//! The tests:
//! - `qa_cta_mutants_work_runs_only_on_the_merge_ready_run`: on each event a pull request has (opened, a push,
//!   reopened, the `merge-ready` label, another label), which of `mutants.yml`'s jobs run, by evaluating each job's
//!   `if:` as a runner does (its implicit `success()` over the jobs it needs, those skipped included): the shards, the
//!   fixture and the verdict run on the `merge-ready` label's run alone, the verdict whatever the shards ended in;
//!   another label's run, put on or taken off, runs none of that work; and, under R-678, `mutants.yml`'s job
//!   `mutants-check` runs on every event, the one job of any workflow bearing that required name.
//! - `qa_cta_mutants_check_through_a_pull_requests_life`: a pull request's life, event by event, against a stand-in
//!   GitHub (the head, the labels and `mutants.yml`'s runs with their jobs kept in a file, served by a stand-in `gh`),
//!   each job run in its `needs:` order and its `gh` steps under bash as a runner runs them, the shards' outcome
//!   given. After each event the head's `mutants-check`, as branch protection reads it (R-678: the job in the latest
//!   run's suite, never a check run posted into another), is: passed "not applicable" on a head that is not
//!   merge-ready, a new code head's and one whose `merge-ready` label was taken off; failed on a failed, cancelled or
//!   skipped merge-ready verdict; passed by a passing merge-ready run, or at once on a head with no `.rs` file under
//!   `crates/` (R-677); on another label put on or taken off a merge-ready head, that head's verdict, waited for while
//!   its run is in progress, and a failure with none or on a superseded head; a push takes the label off, and a reopen
//!   leaves a passed head passed. R-678 (3 Nov 2026) replaced the posted check run, pending on a new code head, that
//!   this test followed before; that is the change here, R-678's exception to R-290.
//! - `qa_cta_superseded_runs_cancel_per_pull_request`: the concurrency groups, evaluated per event: in `mutants.yml` a
//!   push shares the merge-ready run's group (so it cancels it), another label's run, put on or taken off, does not,
//!   `merge-ready` taken off does (R-678), two pull requests never share one, and nothing on `main` is cancelled; in
//!   `reviews.yml` a review cancels the review run before it on the same pull request, never another pull request's
//!   or the `pull_request` run it re-runs (R-276), and `reviews-complete` runs on both events.
//!
//! Each test has its negative controls (R-176), each feeding the same check a workflow differing in the one respect
//! it turns on, tripping it by its message. The workflows are parsed with PyYAML, as xtask's other workflow tests
//! parse them.

use std::collections::{BTreeMap, BTreeSet, HashMap};
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

fn workflow(name: &str) -> String {
    std::fs::read_to_string(root().join(".github/workflows").join(name))
        .unwrap_or_else(|e| panic!("qa_cta: {name}: {e}"))
}

/// `workflow(name)` with `from` replaced once by `to`; `from` must be there, so a control never edits nothing.
#[cfg(feature = "controls")]
fn edited(name: &str, from: &str, to: &str) -> String {
    let yml = workflow(name);
    assert!(
        yml.contains(from),
        "qa_cta: the control's edit target is gone from {name}: {from}"
    );
    yml.replacen(from, to, 1)
}

fn scratch(tag: &str) -> Scratch {
    let dir = Scratch::new(&format!("qa_cta_{tag}"));
    std::fs::create_dir_all(&*dir).expect("qa_cta: scratch folder");
    dir
}

/// A workflow's text as JSON, parsed by PyYAML. YAML 1.1 reads the key `on` as `true`; it is put back as `on`.
fn parse(yml: &str) -> Value {
    let dir = scratch("yaml");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("qa_cta: workflow written");
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
        .expect("qa_cta: python3 ran");
    assert!(
        out.status.success(),
        "qa_cta: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_cta: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

// ---- GitHub's expressions, as a runner evaluates them ------------------------------------------------------------

#[derive(Clone, Debug)]
enum V {
    S(String),
    B(bool),
    A(Vec<String>),
}

impl V {
    fn truthy(&self) -> bool {
        match self {
            V::S(s) => !s.is_empty(),
            V::B(b) => *b,
            V::A(_) => true,
        }
    }

    fn show(&self) -> String {
        match self {
            V::S(s) => s.clone(),
            V::B(b) => b.to_string(),
            V::A(a) => a.join(","),
        }
    }
}

/// What a run's expressions can read: the `github` context, each step's outcome and outputs, each needed job's result,
/// and whether the job has failed so far (for `success()` and `failure()`). An operand not here fails the test rather
/// than being guessed.
#[derive(Clone, Default)]
struct Ctx {
    vars: HashMap<String, V>,
    steps: HashMap<String, (String, HashMap<String, String>)>,
    needs: HashMap<String, String>,
    failed: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Str(String),
    Word(String),
    Op(&'static str),
    Open,
    Close,
    Comma,
}

fn lex(e: &str) -> Vec<Tok> {
    let c: Vec<char> = e.chars().collect();
    let word = |ch: char| ch.is_alphanumeric() || "._-*".contains(ch);
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '\'' {
            let mut s = String::new();
            i += 1;
            loop {
                assert!(i < c.len(), "qa_cta: an unclosed string in `{e}`");
                if c[i] == '\'' {
                    if c.get(i + 1) == Some(&'\'') {
                        s.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                s.push(c[i]);
                i += 1;
            }
            out.push(Tok::Str(s));
            continue;
        }
        let two: String = c[i..(i + 2).min(c.len())].iter().collect();
        if let Some(op) = ["==", "!=", "&&", "||"].into_iter().find(|o| *o == two) {
            out.push(Tok::Op(op));
            i += 2;
            continue;
        }
        match ch {
            '!' => out.push(Tok::Op("!")),
            '(' => out.push(Tok::Open),
            ')' => out.push(Tok::Close),
            ',' => out.push(Tok::Comma),
            _ if word(ch) => {
                let start = i;
                while i < c.len() && word(c[i]) {
                    i += 1;
                }
                out.push(Tok::Word(c[start..i].iter().collect()));
                continue;
            }
            _ => panic!("qa_cta: `{ch}` not understood in `{e}`"),
        }
        i += 1;
    }
    out
}

struct Parser<'a> {
    toks: Vec<Tok>,
    at: usize,
    ctx: &'a Ctx,
    src: &'a str,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn take(&mut self, want: &Tok) {
        assert_eq!(
            self.peek(),
            Some(want),
            "qa_cta: expected {want:?} in `{}`",
            self.src
        );
        self.at += 1;
    }

    fn or(&mut self) -> V {
        let mut left = self.and();
        while self.peek() == Some(&Tok::Op("||")) {
            self.at += 1;
            let right = self.and();
            left = if left.truthy() { left } else { right };
        }
        left
    }

    fn and(&mut self) -> V {
        let mut left = self.compare();
        while self.peek() == Some(&Tok::Op("&&")) {
            self.at += 1;
            let right = self.compare();
            left = if left.truthy() { right } else { left };
        }
        left
    }

    fn compare(&mut self) -> V {
        let left = self.unary();
        let op = match self.peek() {
            Some(Tok::Op(op)) if *op == "==" || *op == "!=" => *op,
            _ => return left,
        };
        self.at += 1;
        let right = self.unary();
        let same = left.show().eq_ignore_ascii_case(&right.show());
        V::B(if op == "==" { same } else { !same })
    }

    fn unary(&mut self) -> V {
        if self.peek() == Some(&Tok::Op("!")) {
            self.at += 1;
            return V::B(!self.unary().truthy());
        }
        self.primary()
    }

    fn primary(&mut self) -> V {
        let tok = self.peek().cloned();
        self.at += 1;
        match tok {
            Some(Tok::Open) => {
                let v = self.or();
                self.take(&Tok::Close);
                v
            }
            Some(Tok::Str(s)) => V::S(s),
            Some(Tok::Word(w)) if self.peek() == Some(&Tok::Open) => {
                self.at += 1;
                let mut args = Vec::new();
                while self.peek() != Some(&Tok::Close) {
                    args.push(self.or());
                    if self.peek() == Some(&Tok::Comma) {
                        self.at += 1;
                    }
                }
                self.take(&Tok::Close);
                self.call(&w, &args)
            }
            Some(Tok::Word(w)) if w == "true" || w == "false" => V::B(w == "true"),
            Some(Tok::Word(w)) => self.ctx.lookup(&w),
            other => panic!("qa_cta: {other:?} not understood in `{}`", self.src),
        }
    }

    fn call(&self, f: &str, args: &[V]) -> V {
        match (f, args) {
            ("always", []) => V::B(true),
            ("success", []) => V::B(!self.ctx.failed),
            ("failure", []) => V::B(self.ctx.failed),
            ("cancelled", []) => V::B(false),
            ("contains", [V::A(items), x]) => {
                V::B(items.iter().any(|i| i.eq_ignore_ascii_case(&x.show())))
            }
            ("contains", [V::S(s), x]) => V::B(s.to_lowercase().contains(&x.show().to_lowercase())),
            _ => panic!(
                "qa_cta: function {f}{args:?} not understood in `{}`",
                self.src
            ),
        }
    }
}

impl Ctx {
    fn lookup(&self, w: &str) -> V {
        if let Some(v) = self.vars.get(w) {
            return v.clone();
        }
        if let Some(rest) = w.strip_prefix("steps.") {
            if let Some((id, field)) = rest.split_once('.') {
                if let Some((outcome, outputs)) = self.steps.get(id) {
                    if field == "outcome" || field == "conclusion" {
                        return V::S(outcome.clone());
                    }
                    if let Some(k) = field.strip_prefix("outputs.") {
                        return V::S(outputs.get(k).cloned().unwrap_or_default());
                    }
                }
            }
        }
        if let Some(job) = w
            .strip_prefix("needs.")
            .and_then(|r| r.strip_suffix(".result"))
        {
            if let Some(r) = self.needs.get(job) {
                return V::S(r.clone());
            }
        }
        panic!("qa_cta: expression operand not understood: {w}");
    }

    fn eval(&self, expr: &str) -> V {
        let e = expr.trim();
        let e = e
            .strip_prefix("${{")
            .and_then(|x| x.strip_suffix("}}"))
            .unwrap_or(e);
        let mut p = Parser {
            toks: lex(e),
            at: 0,
            ctx: self,
            src: e,
        };
        let v = p.or();
        assert!(
            p.at == p.toks.len(),
            "qa_cta: expression not wholly understood: `{e}`"
        );
        v
    }

    /// An `if:` as a runner reads it: with no status function in it, `success() && …` is implied.
    fn holds(&self, cond: Option<&Value>) -> bool {
        let raw = cond.map(text).unwrap_or_else(|| "success()".to_owned());
        let status = ["always()", "success()", "failure()", "cancelled()"]
            .iter()
            .any(|f| raw.contains(f));
        let v = self.eval(&raw).truthy();
        if status {
            v
        } else {
            !self.failed && v
        }
    }

    /// `s` with each `${{ … }}` replaced by its value.
    fn interpolate(&self, s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(at) = rest.find("${{") {
            out.push_str(&rest[..at]);
            let end = rest[at..].find("}}").expect("qa_cta: unclosed ${{") + at;
            out.push_str(&self.eval(&rest[at + 3..end]).show());
            rest = &rest[end + 2..];
        }
        out.push_str(rest);
        out
    }
}

// ---- A pull request's events ---------------------------------------------------------------------------------------

/// One event of pull request #7 on `o/r`: its name and action, the label it put on (if any), its head, the labels its
/// payload lists, and the ref it runs on.
struct Ev<'a> {
    event: &'a str,
    action: &'a str,
    label: &'a str,
    head: &'a str,
    labels: Vec<String>,
    git_ref: &'a str,
    run_id: u32,
}

fn ev<'a>(action: &'a str, label: &'a str, head: &'a str, labels: &[&str]) -> Ev<'a> {
    Ev {
        event: "pull_request",
        action,
        label,
        head,
        labels: labels.iter().map(|l| (*l).to_owned()).collect(),
        git_ref: "refs/pull/7/merge",
        run_id: 100,
    }
}

fn github(e: &Ev) -> Ctx {
    let mut vars: HashMap<String, V> = HashMap::new();
    let mut put = |k: &str, v: V| {
        vars.insert(k.to_owned(), v);
    };
    put("github.event_name", V::S(e.event.to_owned()));
    put("github.event.action", V::S(e.action.to_owned()));
    put("github.event.label.name", V::S(e.label.to_owned()));
    put("github.ref", V::S(e.git_ref.to_owned()));
    put("github.run_id", V::S(e.run_id.to_string()));
    put("github.base_ref", V::S("main".to_owned()));
    put("github.repository", V::S("o/r".to_owned()));
    put("github.token", V::S("stand-in-token".to_owned()));
    put(
        "github.server_url",
        V::S("https://github.example".to_owned()),
    );
    put("github.event.pull_request.number", V::S("7".to_owned()));
    put(
        "github.event.pull_request.head.sha",
        V::S(e.head.to_owned()),
    );
    put(
        "github.event.pull_request.labels.*.name",
        V::A(e.labels.clone()),
    );
    Ctx {
        vars,
        ..Ctx::default()
    }
}

fn needs_of(job: &Value) -> Vec<String> {
    match job.get("needs") {
        Some(Value::Array(a)) => a.iter().map(text).collect(),
        Some(v) => vec![text(v)],
        None => Vec::new(),
    }
}

/// The jobs of `wf` that run on event `e`, each with its result: a job that runs ends in `shards` if it is `mutants`
/// (the shards' outcome), else success; one that does not run is skipped, and a job needing it sees it skipped.
fn jobs_run(wf: &Value, e: &Ev, shards: &str) -> BTreeMap<String, String> {
    let jobs = wf["jobs"].as_object().expect("qa_cta: jobs");
    let mut result: BTreeMap<String, String> = BTreeMap::new();
    while result.len() < jobs.len() {
        let before = result.len();
        for (id, job) in jobs {
            if result.contains_key(id) {
                continue;
            }
            let needs = needs_of(job);
            if !needs.iter().all(|n| result.contains_key(n)) {
                continue;
            }
            let mut ctx = github(e);
            for n in &needs {
                ctx.needs.insert(n.clone(), result[n].clone());
            }
            ctx.failed = needs.iter().any(|n| result[n] != "success");
            let r = if ctx.holds(job.get("if")) {
                if id == "mutants" {
                    shards.to_owned()
                } else {
                    "success".to_owned()
                }
            } else {
                "skipped".to_owned()
            };
            result.insert(id.clone(), r);
        }
        assert!(result.len() > before, "qa_cta: a cycle in the jobs' needs");
    }
    result
}

fn ran(results: &BTreeMap<String, String>) -> BTreeSet<String> {
    results
        .iter()
        .filter(|(_, r)| r.as_str() != "skipped")
        .map(|(id, _)| id.clone())
        .collect()
}

// ---- Test 1: which jobs run on which event -----------------------------------------------------------------------

const MUTANTS_WORK: [&str; 3] = ["mutants", "mutants-fixture", "mutants-verdict"];

fn work_runs_only_when_merge_ready(mutants: &str, others: &[(String, String)]) {
    let wf = parse(mutants);
    let types: Vec<String> = wf["on"]["pull_request"]["types"]
        .as_array()
        .map(|a| a.iter().map(text).collect())
        .unwrap_or_default();
    for t in ["opened", "synchronize", "reopened", "labeled", "unlabeled"] {
        assert!(
            types.contains(&t.to_owned()),
            "qa_cta: mutants.yml does not run on a pull request's `{t}`"
        );
    }
    let new_heads = [
        ev("opened", "", "a", &[]),
        ev("synchronize", "", "b", &["merge-ready"]),
        ev("synchronize", "", "c", &[]),
        ev("reopened", "", "c", &[]),
    ];
    for e in &new_heads {
        let r = jobs_run(&wf, e, "success");
        for job in MUTANTS_WORK {
            assert!(
                r.get(job).map(String::as_str) == Some("skipped"),
                "qa_cta: job `{job}` runs on `{}`, before the head is merge-ready",
                e.action
            );
        }
        assert!(
            ran(&r).contains("mutants-check"),
            "qa_cta: nothing runs on `{}`, so nothing reports the new head's mutants-check",
            e.action
        );
    }
    // R-678: every label change runs `mutants-check`, and only the merge-ready label's run any of the mutants work.
    for other in [
        ev("labeled", "design", "c", &["design"]),
        ev("labeled", "design", "c", &["design", "merge-ready"]),
        ev("unlabeled", "design", "c", &["merge-ready"]),
        ev("unlabeled", "merge-ready", "c", &[]),
    ] {
        let r = jobs_run(&wf, &other, "success");
        let work: Vec<String> = ran(&r)
            .into_iter()
            .filter(|j| MUTANTS_WORK.contains(&j.as_str()))
            .collect();
        assert!(
            work.is_empty(),
            "qa_cta: another label's run runs jobs: {work:?}"
        );
        assert!(
            ran(&r).contains("mutants-check"),
            "qa_cta: `{} {}` runs no mutants-check, so the run that supersedes the head's last reports none",
            other.action,
            other.label
        );
    }
    for shards in ["success", "failure", "cancelled"] {
        let ready = ev("labeled", "merge-ready", "c", &["merge-ready"]);
        let r = jobs_run(&wf, &ready, shards);
        for job in MUTANTS_WORK.iter().chain(&["mutants-check"]) {
            assert!(
                r.get(*job).is_some_and(|x| x != "skipped"),
                "qa_cta: job `{job}` does not run on the merge-ready run, its shards {shards}: {r:?}"
            );
        }
    }
    // R-678: the one job bearing the required name `mutants-check` is `mutants.yml`'s, which every run runs.
    let mut bearers = Vec::new();
    for (file, yml) in
        std::iter::once(&("mutants.yml".to_owned(), mutants.to_owned())).chain(others)
    {
        let wf = parse(yml);
        let Some(jobs) = wf["jobs"].as_object() else {
            continue;
        };
        for (id, job) in jobs {
            let name = job.get("name").map(text).unwrap_or_else(|| id.clone());
            if name == "mutants-check" {
                bearers.push(format!("{file}: {id}"));
            }
        }
    }
    assert!(
        bearers == ["mutants.yml: mutants-check"],
        "qa_cta: the jobs {bearers:?} bear the required name mutants-check, not mutants.yml's one job, which every \
         run runs"
    );
}

fn other_workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("qa_cta: workflows")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| (n.ends_with(".yml") || n.ends_with(".yaml")) && n != "mutants.yml")
        .map(|n| {
            let t = workflow(&n);
            (n, t)
        })
        .collect();
    out.sort();
    out
}

#[test]
fn qa_cta_mutants_work_runs_only_on_the_merge_ready_run() {
    work_runs_only_when_merge_ready(&workflow("mutants.yml"), &other_workflows());
}

negative_control!(
    qa_cta_mutants_work_runs_only_on_the_merge_ready_run,
    "the fixture run on every new head again",
    expected = "job `mutants-fixture` runs on `opened`, before the head is merge-ready",
    work_runs_only_when_merge_ready(
        &edited(
            "mutants.yml",
            "  mutants-fixture:\n    if: github.event_name == 'pull_request' && github.event.action == 'labeled' \
             && github.event.label.name == 'merge-ready'\n",
            "  mutants-fixture:\n    if: github.event_name == 'pull_request'\n"
        ),
        &other_workflows()
    )
);

mod any_label {
    use super::*;
    negative_control!(
        qa_cta_mutants_work_runs_only_on_the_merge_ready_run,
        "the shards started by any label, not the merge-ready one",
        expected = "another label's run runs jobs",
        work_runs_only_when_merge_ready(
            &edited(
                "mutants.yml",
                "  mutants:\n    if: github.event_name == 'pull_request' && github.event.action == 'labeled' && \
                 github.event.label.name == 'merge-ready'\n",
                "  mutants:\n    if: github.event_name == 'pull_request' && github.event.action == 'labeled'\n"
            ),
            &other_workflows()
        )
    );
}

mod verdict_needs_passed_shards {
    use super::*;
    negative_control!(
        qa_cta_mutants_work_runs_only_on_the_merge_ready_run,
        "a verdict that runs only when every shard passed",
        expected = "job `mutants-verdict` does not run on the merge-ready run, its shards failure",
        work_runs_only_when_merge_ready(
            &edited(
                "mutants.yml",
                "    if: always() && github.event_name == 'pull_request' && github.event.action == 'labeled'",
                "    if: github.event_name == 'pull_request' && github.event.action == 'labeled'"
            ),
            &other_workflows()
        )
    );
}

mod named_job_elsewhere {
    use super::*;
    negative_control!(
        qa_cta_mutants_work_runs_only_on_the_merge_ready_run,
        "a job named mutants-check in another workflow",
        expected = "bear the required name mutants-check, not mutants.yml's one job",
        {
            let mut others = other_workflows();
            let at = others
                .iter()
                .position(|(f, _)| f == "reviews.yml")
                .expect("qa_cta: reviews.yml");
            others[at].1 = edited(
                "reviews.yml",
                "jobs:\n  reviews-complete:\n",
                "jobs:\n  mutants-check:\n    if: github.event_name == 'push'\n    runs-on: ubuntu-24.04\n    \
                 steps:\n      - run: \"true\"\n  reviews-complete:\n",
            );
            work_runs_only_when_merge_ready(&workflow("mutants.yml"), &others)
        }
    );
}

// ---- Test 2: a pull request's life, against a stand-in GitHub -----------------------------------------------------

/// The stand-in GitHub API behind the stand-in `gh`: the pull request's head and labels, and `mutants.yml`'s runs on
/// each head with their jobs, kept in `$FAKE_STATE`, each call logged to `$FAKE_CALLS`. It answers what GitHub's REST
/// API answers for the calls the gate makes (the `--jq` filters it knows applied as `gh` applies them), and fails, as
/// `gh api` does on an HTTP error, on a label that is not there or a call it does not know: a check run posted through
/// the Checks API among them, which R-678 rules out.
const FAKE_GH: &str = r#"
import json, os, sys, urllib.parse
state_path = os.environ["FAKE_STATE"]
st = json.load(open(state_path))
args = sys.argv[1:]
with open(os.environ["FAKE_CALLS"], "a") as f:
    f.write("gh " + " ".join(args) + "\n")
def die(msg, code=1):
    sys.stderr.write("gh: " + msg + "\n")
    sys.exit(code)
if not args or args[0] != "api":
    die("stand-in: not understood: %r" % (args,), 2)
method, jq, path, i = "GET", None, None, 1
while i < len(args):
    a = args[i]
    if a in ("--method", "-X"):
        method = args[i + 1]; i += 2
    elif a in ("--jq", "-q"):
        jq = args[i + 1]; i += 2
    elif a.startswith("-"):
        die("stand-in: flag not understood: " + a, 2)
    elif path is None:
        path = a; i += 1
    else:
        die("stand-in: a second path: " + a, 2)
url = urllib.parse.urlsplit(path)
q = dict(urllib.parse.parse_qsl(url.query))
p = url.path.strip("/").split("/")
if p[:3] != ["repos", "o", "r"]:
    die("stand-in: another repository: " + path, 2)
p = p[3:]
if method == "GET" and p == ["pulls", "7"]:
    out = {"number": 7, "head": {"sha": st["head"]}}
elif method == "GET" and p == ["actions", "workflows", "mutants.yml", "runs"]:
    runs = [r for r in st["runs"] if r["head_sha"] == q.get("head_sha") and q.get("event") == "pull_request"]
    runs.sort(key=lambda r: -r["id"])
    out = {"total_count": len(runs), "workflow_runs": [{"id": r["id"], "head_sha": r["head_sha"]} for r in runs]}
elif method == "GET" and len(p) == 4 and p[:2] == ["actions", "runs"] and p[3] == "jobs":
    found = [r for r in st["runs"] if str(r["id"]) == p[2]]
    if not found:
        die("HTTP 404: no run " + p[2])
    out = {"total_count": len(found[0]["jobs"]), "jobs": found[0]["jobs"]}
elif method == "DELETE" and len(p) == 4 and p[:3] == ["issues", "7", "labels"]:
    if p[3] not in st["labels"]:
        die("HTTP 404: Label does not exist")
    st["labels"].remove(p[3])
    out = [{"name": l} for l in st["labels"]]
else:
    die("stand-in: call not understood: %s %s" % (method, path), 2)
json.dump(st, open(state_path, "w"))
def show(v):
    return "null" if v is None else str(v)
if jq is None:
    print(json.dumps(out))
elif jq == ".head.sha":
    print(out["head"]["sha"])
elif jq == ".workflow_runs[].id":
    for r in out["workflow_runs"]:
        print(r["id"])
elif jq == '.jobs[] | select(.name == "mutants-verdict") | "\\(.status) \\(.conclusion)"':
    for j in out["jobs"]:
        if j["name"] == "mutants-verdict":
            print(show(j["status"]) + " " + show(j["conclusion"]))
else:
    die("stand-in: --jq not understood: " + jq, 2)
"#;

/// The stand-in `sleep`: a wait in the gate ends when a run in progress finishes, as `$FAKE_STATE`'s `on_sleep` says
/// (`[run id, conclusion]`); with none to finish, it fails, so a wait never hangs the test.
const FAKE_SLEEP: &str = r#"
import json, os, sys
state_path = os.environ["FAKE_STATE"]
st = json.load(open(state_path))
todo = st.pop("on_sleep", None)
if todo is None:
    sys.stderr.write("sleep: stand-in: nothing would finish\n")
    sys.exit(1)
for r in st["runs"]:
    if r["id"] == todo[0]:
        for j in r["jobs"]:
            j["status"], j["conclusion"] = "completed", todo[1]
json.dump(st, open(state_path, "w"))
"#;

/// Pull request #7 on the stand-in GitHub, with `mutants.yml` as `yml`.
struct Pr {
    wf: Value,
    dir: Scratch,
}

/// Whether a script calls `gh`: a word `gh` in it, outside a comment's prose.
fn calls_gh(script: &str) -> bool {
    script
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .any(|l| {
            l.split(|c: char| c.is_whitespace() || "();|&$".contains(c))
                .any(|w| w == "gh")
        })
}

/// A job's check name as GitHub shows it: its `name:`, or its id.
fn job_name(id: &str, job: &Value) -> String {
    job.get("name").map(text).unwrap_or_else(|| id.to_owned())
}

impl Pr {
    fn new(yml: &str, tag: &str) -> Self {
        let dir = scratch(tag);
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("qa_cta: bin");
        for (name, body) in [("gh", FAKE_GH), ("sleep", FAKE_SLEEP)] {
            let py = dir.join(format!("fake_{name}.py"));
            std::fs::write(&py, body).expect("qa_cta: stand-in");
            write_executable(
                &bin.join(name),
                format!("#!/bin/sh\nexec python3 -I \"{}\" \"$@\"\n", py.display()),
            )
            .expect("qa_cta: stand-in");
        }
        std::fs::write(
            dir.join("state.json"),
            r#"{"head": "", "next": 0, "runs": [], "labels": []}"#,
        )
        .expect("qa_cta: state");
        std::fs::write(dir.join("calls"), "").expect("qa_cta: calls");
        Pr {
            wf: parse(yml),
            dir,
        }
    }

    fn state(&self) -> Value {
        serde_json::from_str(
            &std::fs::read_to_string(self.dir.join("state.json")).expect("qa_cta: state"),
        )
        .expect("qa_cta: state JSON")
    }

    fn save(&self, st: &Value) {
        std::fs::write(self.dir.join("state.json"), st.to_string()).expect("qa_cta: state");
    }

    fn set_labels(&self, labels: &[&str]) {
        let mut st = self.state();
        st["labels"] = serde_json::json!(labels);
        self.save(&st);
    }

    fn set_head(&self, head: &str) {
        let mut st = self.state();
        st["head"] = serde_json::json!(head);
        self.save(&st);
    }

    fn labels(&self) -> Vec<String> {
        self.state()["labels"]
            .as_array()
            .expect("qa_cta: labels")
            .iter()
            .map(text)
            .collect()
    }

    fn calls(&self) -> String {
        std::fs::read_to_string(self.dir.join("calls")).unwrap_or_default()
    }

    /// A new run of the workflow on `head`, in progress with `jobs`: its id.
    fn add_run(&self, head: &str, jobs: Value) -> u64 {
        let mut st = self.state();
        let id = st["next"].as_u64().expect("qa_cta: next") + 1;
        st["next"] = serde_json::json!(id);
        st["runs"]
            .as_array_mut()
            .expect("qa_cta: runs")
            .push(serde_json::json!({"id": id, "head_sha": head, "jobs": jobs}));
        self.save(&st);
        id
    }

    /// A merge-ready run on `head` still running its shards, which the next wait finishes with `conclusion`.
    fn merge_ready_in_progress(&self, head: &str, conclusion: &str) -> u64 {
        let id = self.add_run(
            head,
            serde_json::json!([
                {"name": "mutants-verdict", "status": "in_progress", "conclusion": null},
                {"name": "mutants-check", "status": "in_progress", "conclusion": null},
            ]),
        );
        let mut st = self.state();
        st["on_sleep"] = serde_json::json!([id, conclusion]);
        self.save(&st);
        id
    }

    /// The head's required `mutants-check` as branch protection reads it (R-678): of the workflow's runs on the head,
    /// only the latest run's suite counts, and in it the job `mutants-check`: `expected` with no run or no such job,
    /// `pending` while it runs, `skipped`, which GitHub counts as passed and R-305 never allows, `failed` or `passed`.
    fn gate(&self, head: &str) -> String {
        let st = self.state();
        let latest = st["runs"]
            .as_array()
            .expect("qa_cta: runs")
            .iter()
            .filter(|r| r["head_sha"] == head)
            .max_by_key(|r| r["id"].as_u64());
        let Some(run) = latest else {
            return "expected".to_owned();
        };
        let job = run["jobs"]
            .as_array()
            .expect("qa_cta: jobs")
            .iter()
            .find(|j| j["name"] == "mutants-check");
        match job.map(|j| (text(&j["status"]), text(&j["conclusion"]))) {
            None => "expected".to_owned(),
            Some((s, _)) if s != "completed" => "pending".to_owned(),
            Some((_, c)) if c == "success" => "passed".to_owned(),
            Some((_, c)) if c == "skipped" => "skipped".to_owned(),
            Some(_) => "failed".to_owned(),
        }
    }

    /// Event `e` on the pull request, run as GitHub runs it: nothing if the workflow's `pull_request` types leave the
    /// action out; else a new run on the head, whose jobs run in their `needs:` order, each `if:` evaluated over the
    /// results of the jobs it needs. The shards and the verdict end in `verdict`; in every other job that runs, the
    /// classification step reads the change (R-677: `no_mutant`, no `.rs` file under `crates/`, is `mutants=false`)
    /// when its `if:` holds, each step calling `gh` runs under bash as a runner runs it, with its `env:` and the
    /// stand-in `gh` and `sleep`, and any other step stands in as passed; a failed step fails the job. Every job's
    /// result is recorded on the run, by its check name. Returns the jobs that ran, and the `gh` steps' logs and job
    /// summaries.
    fn on(&self, e: &Ev, no_mutant: bool, verdict: &str) -> (BTreeSet<String>, String) {
        let types: Vec<String> = self.wf["on"]["pull_request"]["types"]
            .as_array()
            .map(|a| a.iter().map(text).collect())
            .unwrap_or_else(|| {
                ["opened", "synchronize", "reopened"]
                    .map(str::to_owned)
                    .to_vec()
            });
        if !types.iter().any(|t| t == e.action) {
            return (
                BTreeSet::new(),
                format!("mutants.yml does not run on `{}`\n", e.action),
            );
        }
        let run_id = self.add_run(e.head, serde_json::json!([]));
        let e = Ev {
            event: e.event,
            action: e.action,
            label: e.label,
            head: e.head,
            labels: e.labels.clone(),
            git_ref: e.git_ref,
            run_id: u32::try_from(run_id).expect("qa_cta: run id"),
        };
        let jobs = self.wf["jobs"].as_object().expect("qa_cta: jobs");
        let mut results: BTreeMap<String, String> = BTreeMap::new();
        let mut log = String::new();
        while results.len() < jobs.len() {
            let before = results.len();
            for (id, job) in jobs {
                let needs = needs_of(job);
                if results.contains_key(id) || !needs.iter().all(|n| results.contains_key(n)) {
                    continue;
                }
                let mut ctx = github(&e);
                for n in &needs {
                    ctx.needs.insert(n.clone(), results[n].clone());
                }
                ctx.failed = needs.iter().any(|n| results[n] != "success");
                let r = if !ctx.holds(job.get("if")) {
                    "skipped".to_owned()
                } else if id == "mutants" || id == "mutants-verdict" {
                    verdict.to_owned()
                } else {
                    ctx.failed = false;
                    self.steps(job, &mut ctx, no_mutant, &mut log);
                    if ctx.failed { "failure" } else { "success" }.to_owned()
                };
                results.insert(id.clone(), r);
            }
            assert!(results.len() > before, "qa_cta: a cycle in the jobs' needs");
        }
        let recorded: Vec<Value> = jobs
            .iter()
            .map(|(id, job)| {
                serde_json::json!({"name": job_name(id, job), "status": "completed", "conclusion": results[id]})
            })
            .collect();
        let mut st = self.state();
        for r in st["runs"].as_array_mut().expect("qa_cta: runs") {
            if r["id"] == run_id {
                r["jobs"] = Value::Array(recorded.clone());
            }
        }
        self.save(&st);
        (ran(&results), log)
    }

    /// A job's steps, run as `on` describes; `ctx.failed` says whether one failed.
    fn steps(&self, job: &Value, ctx: &mut Ctx, no_mutant: bool, log: &mut String) {
        for (i, step) in job["steps"]
            .as_array()
            .expect("qa_cta: steps")
            .iter()
            .enumerate()
        {
            let sid = step.get("id").map(text).unwrap_or_else(|| format!("__{i}"));
            if !ctx.holds(step.get("if")) {
                ctx.steps
                    .insert(sid, ("skipped".to_owned(), HashMap::new()));
                continue;
            }
            if sid == "changes" {
                let outputs = HashMap::from([("mutants".to_owned(), (!no_mutant).to_string())]);
                ctx.steps.insert(sid, ("success".to_owned(), outputs));
                continue;
            }
            let script = step.get("run").map(text).unwrap_or_default();
            if !calls_gh(&script) {
                ctx.steps
                    .insert(sid, ("success".to_owned(), HashMap::new()));
                continue;
            }
            let summary = self
                .dir
                .join(format!("summary_{}", ctx.lookup("github.run_id").show()));
            let mut cmd = Command::new("bash");
            cmd.args([
                "--noprofile",
                "--norc",
                "-eo",
                "pipefail",
                "-c",
                &ctx.interpolate(&script),
            ])
            .current_dir(&*self.dir)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.dir.join("bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .env("GITHUB_STEP_SUMMARY", &summary)
            .env("FAKE_STATE", self.dir.join("state.json"))
            .env("FAKE_CALLS", self.dir.join("calls"));
            if let Some(env) = step.get("env").and_then(Value::as_object) {
                for (k, v) in env {
                    cmd.env(k, ctx.interpolate(&text(v)));
                }
            }
            let out = cmd.timed_output().expect("qa_cta: bash ran");
            log.push_str(&String::from_utf8_lossy(&out.stdout));
            log.push_str(&String::from_utf8_lossy(&out.stderr));
            log.push_str(&std::fs::read_to_string(&summary).unwrap_or_default());
            let outcome = if out.status.success() {
                "success"
            } else {
                ctx.failed = true;
                "failure"
            };
            ctx.steps.insert(sid, (outcome.to_owned(), HashMap::new()));
        }
    }

    /// The watcher's label (with the user's token: GitHub adds it, then the `labeled` run starts), the shards and the
    /// verdict ending in `verdict`.
    fn merge_ready(
        &self,
        head: &str,
        no_mutant: bool,
        verdict: &str,
    ) -> (BTreeSet<String>, String) {
        self.label_on("merge-ready", head, no_mutant, verdict)
    }

    /// Label `label` put on the pull request, then its `labeled` run.
    fn label_on(
        &self,
        label: &str,
        head: &str,
        no_mutant: bool,
        verdict: &str,
    ) -> (BTreeSet<String>, String) {
        let mut labels = self.labels();
        labels.retain(|l| l != label);
        labels.push(label.to_owned());
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.set_labels(&refs);
        self.on(&ev("labeled", label, head, &refs), no_mutant, verdict)
    }

    /// Label `label` taken off the pull request, then its `unlabeled` run.
    fn label_off(&self, label: &str, head: &str, no_mutant: bool) -> (BTreeSet<String>, String) {
        let mut labels = self.labels();
        labels.retain(|l| l != label);
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.set_labels(&refs);
        self.on(&ev("unlabeled", label, head, &refs), no_mutant, "success")
    }

    /// A push of `head`: the `synchronize` event's payload lists the labels on the pull request when it fired.
    fn push(&self, head: &str, no_mutant: bool) -> (BTreeSet<String>, String) {
        self.set_head(head);
        let labels = self.labels();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.on(&ev("synchronize", "", head, &refs), no_mutant, "success")
    }
}

/// The reason a not-applicable pass gives, in its job summary.
const NOT_APPLICABLE: &str = "not applicable";

fn life(yml: &str) {
    let pr = Pr::new(yml, "life");
    let say = |what: &str, log: &str| format!("{what} (calls:\n{}log:\n{log})", pr.calls());
    let gate_is = |head: &str, want: &str, what: &str, log: &str| {
        assert_eq!(pr.gate(head), want, "qa_cta: {}", say(what, log));
    };

    // Opened on a code head: not merge-ready, so passed "not applicable" at once (R-678), never left expected.
    pr.set_head("a");
    let (_, log) = pr.on(&ev("opened", "", "a", &[]), false, "success");
    gate_is(
        "a",
        "passed",
        "a new code head's mutants-check is not passed, not applicable",
        &log,
    );
    assert!(
        log.contains(NOT_APPLICABLE),
        "qa_cta: {}",
        say("a new code head's pass does not say not applicable", &log)
    );

    // Another label on a head that is not merge-ready: none of the mutants work, `mutants-check` passed again.
    let (jobs, log) = pr.label_on("design", "a", false, "success");
    assert!(
        jobs.iter().all(|j| !MUTANTS_WORK.contains(&j.as_str())) && pr.gate("a") == "passed",
        "qa_cta: {}",
        say(
            &format!(
                "another label ran {jobs:?} or moved the gate to {}",
                pr.gate("a")
            ),
            &log
        )
    );

    // The merge-ready run: its verdict, failure, fails the head's check; another label put on then keeps it failed.
    let (_, log) = pr.merge_ready("a", false, "failure");
    gate_is(
        "a",
        "failed",
        "a failed merge-ready verdict did not fail mutants-check",
        &log,
    );
    let (_, log) = pr.label_on("urgent", "a", false, "success");
    gate_is(
        "a",
        "failed",
        "another label on a merge-ready head whose run failed did not keep it failed",
        &log,
    );

    // Taking merge-ready off: the head is no longer merge-ready, passed "not applicable" (the owner, 3 Nov 2026).
    let (_, log) = pr.label_off("merge-ready", "a", false);
    gate_is(
        "a",
        "passed",
        "taking merge-ready off left mutants-check other than passed, not applicable",
        &log,
    );

    // Re-labelled, success: passed; another label put on and taken off keeps that verdict.
    let (_, log) = pr.merge_ready("a", false, "success");
    gate_is(
        "a",
        "passed",
        "a passing merge-ready run did not pass mutants-check",
        &log,
    );
    let (_, log) = pr.label_off("urgent", "a", false);
    gate_is(
        "a",
        "passed",
        "another label taken off a passed merge-ready head did not keep it passed",
        &log,
    );

    // A push: the label comes off, and the new head, not merge-ready, passes "not applicable".
    let (_, log) = pr.push("b", false);
    assert!(
        !pr.labels().contains(&"merge-ready".to_owned()),
        "qa_cta: {}",
        say("a push left the merge-ready label on", &log)
    );
    gate_is(
        "b",
        "passed",
        "a pushed code head's mutants-check is not passed, not applicable",
        &log,
    );

    // A label run on a head a push has since superseded: failed, never the old head's verdict.
    let labels = pr.labels();
    let mut refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    refs.push("merge-ready");
    let (_, log) = pr.on(&ev("labeled", "design", "a", &refs), false, "success");
    gate_is(
        "a",
        "failed",
        "a label run on a superseded head passed",
        &log,
    );

    // Never passed on a skip: a cancelled or skipped verdict is a failure.
    for verdict in ["cancelled", "skipped"] {
        pr.set_labels(&[]);
        let (_, log) = pr.merge_ready("b", false, verdict);
        gate_is(
            "b",
            "failed",
            &format!("a {verdict} merge-ready verdict did not fail mutants-check"),
            &log,
        );
    }
    pr.set_labels(&[]);
    let (_, log) = pr.merge_ready("b", false, "success");
    gate_is("b", "passed", "b's passing re-run did not pass it", &log);

    // Reopened on a merge-ready head that passed: it stays passed.
    let labels = pr.labels();
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let (_, log) = pr.on(&ev("reopened", "", "b", &refs), false, "success");
    gate_is(
        "b",
        "passed",
        "a reopened pull request's passed head is no longer passed",
        &log,
    );

    // Another label while the merge-ready run is still running: it waits, then reports that run's verdict.
    pr.merge_ready_in_progress("b", "failure");
    let (_, log) = pr.label_on("design", "b", false, "success");
    gate_is(
        "b",
        "failed",
        "another label during the merge-ready run did not wait for its verdict",
        &log,
    );

    // A head with no `.rs` file under `crates/` passes at once (R-305, R-677), and the label comes off.
    let (_, log) = pr.push("c", true);
    assert!(
        pr.gate("c") == "passed" && !pr.labels().contains(&"merge-ready".to_owned()),
        "qa_cta: {}",
        say(
            &format!(
                "a head with no Rust under crates/: its mutants-check is {}, not passed at once",
                pr.gate("c")
            ),
            &log
        )
    );
    // And it never waits for a merge-ready run, even with the label on and none on the head.
    pr.set_labels(&["merge-ready"]);
    let (_, log) = pr.label_on("design", "c", true, "success");
    gate_is(
        "c",
        "passed",
        "a head with no Rust under crates/ is not passed at once when another label goes on",
        &log,
    );

    // A code push with no label: nothing to take off, the head passed "not applicable".
    pr.set_labels(&[]);
    let (_, log) = pr.push("d", false);
    gate_is(
        "d",
        "passed",
        "an unlabelled push's code head is not passed, not applicable",
        &log,
    );

    // The merge-ready label on, but no run of it on the head (its run lost): another label's run fails, never passes.
    pr.set_labels(&["merge-ready"]);
    let (_, log) = pr.label_on("design", "d", false, "success");
    gate_is(
        "d",
        "failed",
        "another label on a merge-ready head with no merge-ready run did not fail",
        &log,
    );

    // A head no earlier run reported on (its push run lost): the merge-ready run still reports.
    pr.set_head("e");
    pr.set_labels(&[]);
    let (_, log) = pr.merge_ready("e", false, "success");
    gate_is(
        "e",
        "passed",
        "a head with no earlier mutants-check did not pass on its merge-ready run",
        &log,
    );

    // Two pushes close together: the second's payload still lists the label the first's run has since taken off. The
    // head still reports, passed, so no pull request waits on a check that never reports (R-305, R-678).
    pr.set_labels(&[]);
    pr.set_head("f");
    let (_, log) = pr.on(
        &ev("synchronize", "", "f", &["merge-ready"]),
        false,
        "success",
    );
    gate_is(
        "f",
        "passed",
        "a head pushed after the label came off is not passed at once",
        &log,
    );
}

#[test]
fn qa_cta_mutants_check_through_a_pull_requests_life() {
    life(&workflow("mutants.yml"));
}

negative_control!(
    qa_cta_mutants_check_through_a_pull_requests_life,
    "mutants-check posted through the Checks API by a job of another name, as #379 had it",
    expected = "a new code head's mutants-check is not passed, not applicable",
    life(&edited(
        "mutants.yml",
        "  mutants-check:\n    if: always()\n",
        "  mutants-report:\n    if: always()\n"
    ))
);

mod new_head_waits {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a head that is not merge-ready waiting for a merge-ready run, as R-676's pending check did",
        expected = "a new code head's mutants-check is not passed, not applicable",
        life(&edited(
            "mutants.yml",
            "          if [ \"$ACTION\" = synchronize ] || [ \"$READY\" != true ]; then\n",
            "          if false; then\n"
        ))
    );
}

mod cancelled_passes {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a verdict that passes anything but a failure",
        expected = "a cancelled merge-ready verdict did not fail mutants-check",
        life(&edited(
            "mutants.yml",
            "            if [ \"$1\" = success ]; then\n",
            "            if [ \"$1\" != failure ]; then\n"
        ))
    );
}

mod label_stays {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "the label taken off only when the pull request is opened",
        expected = "a push left the merge-ready label on",
        life(&edited(
            "mutants.yml",
            "          github.event.action == 'synchronize'\n",
            "          github.event.action == 'opened'\n"
        ))
    );
}

mod no_unlabeled_run {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "mutants.yml not run when a label is taken off, so the merge-ready run's failure stands",
        expected = "taking merge-ready off left mutants-check other than passed, not applicable",
        life(&edited(
            "mutants.yml",
            "    types: [opened, synchronize, reopened, labeled, unlabeled]\n",
            "    types: [opened, synchronize, reopened, labeled]\n"
        ))
    );
}

mod other_label_not_applicable {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "another label's run passing a merge-ready head not applicable, without its verdict",
        expected = "another label on a merge-ready head whose run failed did not keep it failed",
        life(&edited(
            "mutants.yml",
            "          if [ \"$ACTION\" = synchronize ] || [ \"$READY\" != true ]; then\n",
            "          if true; then\n"
        ))
    );
}

mod no_wait {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a merge-ready run in progress passed over, not waited for",
        expected = "another label during the merge-ready run did not wait for its verdict",
        life(&edited(
            "mutants.yml",
            "                *) wait=\"run $id: ${state:-no jobs yet}\"; break ;;\n",
            "                *) continue ;;\n"
        ))
    );
}

mod superseded_passes {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a label run reading an old head's verdict after a push",
        expected = "a label run on a superseded head passed",
        life(&edited(
            "mutants.yml",
            "            if [ \"$live\" != \"$HEAD_SHA\" ]; then\n",
            "            if false; then\n"
        ))
    );
}

mod docs_only_waits {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a head with no Rust under crates/ waiting for a merge-ready run like a code head's",
        expected =
            "a head with no Rust under crates/ is not passed at once when another label goes on",
        life(&edited(
            "mutants.yml",
            "          if [ \"$MUTANTS\" = false ]; then\n",
            "          if false; then\n"
        ))
    );
}

// ---- Test 3: the concurrency groups ------------------------------------------------------------------------------

/// A run's concurrency group and whether it cancels the run in progress in it.
fn group(wf: &Value, e: &Ev) -> (String, bool) {
    let ctx = github(e);
    let c = &wf["concurrency"];
    let g = ctx.interpolate(&text(&c["group"]));
    let cancel = ctx.interpolate(&text(&c["cancel-in-progress"]));
    assert!(
        cancel == "true" || cancel == "false",
        "qa_cta: cancel-in-progress is not a boolean: {cancel}"
    );
    (g, cancel == "true")
}

fn on_ref<'a>(mut e: Ev<'a>, git_ref: &'a str, event: &'a str, run_id: u32) -> Ev<'a> {
    e.git_ref = git_ref;
    e.event = event;
    e.run_id = run_id;
    e
}

fn groups(mutants: &str, reviews: &str) {
    let m = parse(mutants);
    let ready = group(&m, &ev("labeled", "merge-ready", "a", &["merge-ready"]));
    let push = group(
        &m,
        &on_ref(
            ev("synchronize", "", "b", &["merge-ready"]),
            "refs/pull/7/merge",
            "pull_request",
            101,
        ),
    );
    assert!(
        push.0 == ready.0 && push.1,
        "qa_cta: a push does not cancel the merge-ready run on the head it supersedes: {ready:?} {push:?}"
    );
    let design = group(
        &m,
        &on_ref(
            ev("labeled", "design", "a", &["design"]),
            "refs/pull/7/merge",
            "pull_request",
            102,
        ),
    );
    assert!(
        design.0 != ready.0,
        "qa_cta: another label's run shares the merge-ready run's group, so it cancels it: {design:?}"
    );
    // R-678: another label taken off runs too, in a group of its own; `merge-ready` taken off cancels its run.
    let design_off = group(
        &m,
        &on_ref(
            ev("unlabeled", "design", "a", &["merge-ready"]),
            "refs/pull/7/merge",
            "pull_request",
            106,
        ),
    );
    assert!(
        design_off.0 != ready.0,
        "qa_cta: another label taken off shares the merge-ready run's group, so it cancels it: {design_off:?}"
    );
    let ready_off = group(
        &m,
        &on_ref(
            ev("unlabeled", "merge-ready", "a", &[]),
            "refs/pull/7/merge",
            "pull_request",
            107,
        ),
    );
    assert!(
        ready_off.0 == ready.0 && ready_off.1,
        "qa_cta: taking merge-ready off does not cancel its run: {ready:?} {ready_off:?}"
    );
    let other_pr = group(
        &m,
        &on_ref(
            ev("synchronize", "", "z", &[]),
            "refs/pull/8/merge",
            "pull_request",
            103,
        ),
    );
    assert!(
        other_pr.0 != push.0,
        "qa_cta: two pull requests share mutants.yml's group: {other_pr:?}"
    );
    let main = group(
        &m,
        &on_ref(ev("", "", "m", &[]), "refs/heads/main", "push", 104),
    );
    let main2 = group(
        &m,
        &on_ref(ev("", "", "n", &[]), "refs/heads/main", "push", 105),
    );
    assert!(
        !main.1 && main.0 != main2.0,
        "qa_cta: a run on main is cancelled or queued behind another: {main:?} {main2:?}"
    );

    let r = parse(reviews);
    let review = |pr_ref: &'static str, run: u32| {
        group(
            &r,
            &on_ref(
                ev("submitted", "", "a", &[]),
                pr_ref,
                "pull_request_review",
                run,
            ),
        )
    };
    let first = review("refs/pull/7/merge", 200);
    let second = review("refs/pull/7/merge", 201);
    assert!(
        first.0 == second.0 && second.1,
        "qa_cta: a review does not cancel the review run before it on the same pull request: {first:?} {second:?}"
    );
    let elsewhere = review("refs/pull/8/merge", 202);
    assert!(
        elsewhere.0 != first.0,
        "qa_cta: two pull requests' review runs share a group: {elsewhere:?}"
    );
    let pr_run = group(
        &r,
        &on_ref(
            ev("synchronize", "", "a", &[]),
            "refs/pull/7/merge",
            "pull_request",
            203,
        ),
    );
    assert!(
        pr_run.0 != first.0,
        "qa_cta: the pull_request run a review re-runs (R-276) shares the review runs' group, so a review cancels it"
    );
    let main = group(
        &r,
        &on_ref(ev("", "", "m", &[]), "refs/heads/main", "push", 204),
    );
    let main2 = group(
        &r,
        &on_ref(ev("", "", "n", &[]), "refs/heads/main", "push", 205),
    );
    assert!(
        !main.1 && main.0 != main2.0,
        "qa_cta: a reviews run on main is cancelled or queued behind another: {main:?}"
    );
    let job = &r["jobs"]["reviews-complete"];
    for (event, action) in [
        ("pull_request", "synchronize"),
        ("pull_request_review", "submitted"),
    ] {
        let ctx = github(&on_ref(
            ev(action, "", "a", &[]),
            "refs/pull/7/merge",
            event,
            206,
        ));
        assert!(
            job.is_object() && needs_of(job).is_empty() && ctx.holds(job.get("if")),
            "qa_cta: reviews-complete does not run on {event}"
        );
    }
}

#[test]
fn qa_cta_superseded_runs_cancel_per_pull_request() {
    groups(&workflow("mutants.yml"), &workflow("reviews.yml"));
}

negative_control!(
    qa_cta_superseded_runs_cancel_per_pull_request,
    "every label's run, the merge-ready one included, in a group of its own",
    expected = "a push does not cancel the merge-ready run on the head it supersedes",
    groups(
        &edited(
            "mutants.yml",
            "    && github.event.label.name != 'merge-ready'))\n",
            "    ))\n"
        ),
        &workflow("reviews.yml")
    )
);

mod label_off_cancels {
    use super::*;
    negative_control!(
        qa_cta_superseded_runs_cancel_per_pull_request,
        "only a label put on in a group of its own, so taking another label off cancels the merge-ready run",
        expected = "another label taken off shares the merge-ready run's group",
        groups(
            &edited(
                "mutants.yml",
                " || github.event.action == 'unlabeled')\n",
                ")\n"
            ),
            &workflow("reviews.yml")
        )
    );
}

mod reviews_one_group {
    use super::*;
    negative_control!(
        qa_cta_superseded_runs_cancel_per_pull_request,
        "reviews.yml's group one for every pull request",
        expected = "two pull requests' review runs share a group",
        groups(
            &workflow("mutants.yml"),
            &edited(
                "reviews.yml",
                "&& github.run_id || github.ref }}",
                "&& github.run_id || 'all' }}"
            )
        )
    );
}

mod reviews_queue {
    use super::*;
    negative_control!(
        qa_cta_superseded_runs_cancel_per_pull_request,
        "reviews.yml's runs queued, not cancelled",
        expected = "a review does not cancel the review run before it on the same pull request",
        groups(
            &workflow("mutants.yml"),
            &edited(
                "reviews.yml",
                "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
                "  cancel-in-progress: false\n"
            )
        )
    );
}

mod main_cancelled {
    use super::*;
    negative_control!(
        qa_cta_superseded_runs_cancel_per_pull_request,
        "mutants.yml cancelling on main too",
        expected = "a run on main is cancelled or queued behind another",
        groups(
            &edited(
                "mutants.yml",
                "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
                "  cancel-in-progress: true\n"
            ),
            &workflow("reviews.yml")
        )
    );
}
