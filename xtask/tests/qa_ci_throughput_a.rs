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
//!   another label's run runs nothing; and no job of any workflow bears the required name `mutants-check`, which a run
//!   that skipped it would report passed.
//! - `qa_cta_mutants_check_through_a_pull_requests_life`: a pull request's life, event by event, against a stand-in
//!   GitHub (check runs and labels kept in a file, served by a stand-in `gh`), each job's `gh` steps run under bash as
//!   a runner runs them, the shards' outcome given. After each event the head's `mutants-check`, as branch protection
//!   reads it, is: pending on a new code head; failed on a failed, cancelled or skipped merge-ready verdict; passed
//!   only by a passing merge-ready run, or at once on a head with no `.rs` file under `crates/` (R-677, which moved
//!   this from a docs-only head's; the one change here, R-677's exception to R-290); one check run per head, so no
//!   stale queued one is left beside the verdict; a push takes the label off, a reopen leaves a passed head passed, and
//!   such a head pushed while an earlier push's run already took the label off still passes at once.
//! - `qa_cta_superseded_runs_cancel_per_pull_request`: the concurrency groups, evaluated per event: in `mutants.yml` a
//!   push shares the merge-ready run's group (so it cancels it), another label's run does not, two pull requests never
//!   share one, and nothing on `main` is cancelled; in `reviews.yml` a review cancels the review run before it on the
//!   same pull request, never another pull request's or the `pull_request` run it re-runs (R-276), and
//!   `reviews-complete` runs on both events.
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
    for t in ["opened", "synchronize", "reopened", "labeled"] {
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
            !ran(&r).is_empty(),
            "qa_cta: nothing runs on `{}`, so nothing reports the new head's mutants-check",
            e.action
        );
    }
    let other = ev("labeled", "design", "c", &["design"]);
    let r = jobs_run(&wf, &other, "success");
    assert!(
        ran(&r).is_empty(),
        "qa_cta: another label's run runs jobs: {:?}",
        ran(&r)
    );
    for shards in ["success", "failure", "cancelled"] {
        let ready = ev("labeled", "merge-ready", "c", &["merge-ready"]);
        let r = jobs_run(&wf, &ready, shards);
        for job in MUTANTS_WORK {
            assert!(
                r.get(job).is_some_and(|x| x != "skipped"),
                "qa_cta: job `{job}` does not run on the merge-ready run, its shards {shards}: {r:?}"
            );
        }
    }
    for (file, yml) in
        std::iter::once(&("mutants.yml".to_owned(), mutants.to_owned())).chain(others)
    {
        let wf = parse(yml);
        let Some(jobs) = wf["jobs"].as_object() else {
            continue;
        };
        for (id, job) in jobs {
            let name = job.get("name").map(text).unwrap_or_else(|| id.clone());
            assert!(
                name != "mutants-check",
                "qa_cta: a job ({file}: {id}) bears the required name mutants-check, which a run skipping it reports \
                 passed"
            );
        }
    }
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
        expected = "bears the required name mutants-check",
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

/// The stand-in GitHub API behind the stand-in `gh`: check runs and the pull request's labels, kept in `$FAKE_STATE`,
/// each call logged to `$FAKE_CALLS`. It answers what GitHub's REST API answers for the calls the gate makes, and
/// fails, as `gh api` does on an HTTP error, on a check run or label that is not there or a call it does not know.
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
method, jq, fields, path, i = "GET", None, {}, None, 1
while i < len(args):
    a = args[i]
    if a in ("--method", "-X"):
        method = args[i + 1]; i += 2
    elif a in ("--jq", "-q"):
        jq = args[i + 1]; i += 2
    elif a == "--paginate":
        i += 1
    elif a in ("-f", "-F", "--raw-field", "--field"):
        k, v = args[i + 1].split("=", 1); fields[k] = v; i += 2
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
out = None
if method == "GET" and len(p) == 3 and p[0] == "commits" and p[2] == "check-runs":
    runs = [r for r in st["runs"] if r["head_sha"] == p[1]]
    if "check_name" in q:
        runs = [r for r in runs if r["name"] == q["check_name"]]
    if "app_id" in q:
        runs = [r for r in runs if str(r["app_id"]) == q["app_id"]]
    if q.get("filter", "latest") == "latest":
        latest = {}
        for r in runs:
            latest[r["name"]] = r
        runs = list(latest.values())
    out = {"total_count": len(runs), "check_runs": runs}
elif method == "POST" and p == ["check-runs"]:
    if "name" not in fields or "head_sha" not in fields:
        die("HTTP 422: name and head_sha are required")
    status = fields.get("status", "queued")
    conclusion = fields.get("conclusion")
    if conclusion is not None:
        status = "completed"
    if status == "completed" and conclusion is None:
        die("HTTP 422: a completed check run needs a conclusion")
    st["next"] += 1
    run = {"id": st["next"], "name": fields["name"], "head_sha": fields["head_sha"], "app_id": 15368,
           "status": status, "conclusion": conclusion}
    st["runs"].append(run)
    out = run
elif method == "PATCH" and len(p) == 2 and p[0] == "check-runs":
    found = [r for r in st["runs"] if str(r["id"]) == p[1]]
    if not found:
        die("HTTP 404: no check run " + p[1])
    run = found[0]
    if "status" in fields:
        run["status"] = fields["status"]
    if "conclusion" in fields:
        run["conclusion"] = fields["conclusion"]; run["status"] = "completed"
    if run["status"] == "completed" and run["conclusion"] is None:
        die("HTTP 422: a completed check run needs a conclusion")
    out = run
elif method == "DELETE" and len(p) == 4 and p[0] == "issues" and p[2] == "labels":
    if p[3] not in st["labels"]:
        die("HTTP 404: Label does not exist")
    st["labels"].remove(p[3])
    out = [{"name": l} for l in st["labels"]]
else:
    die("stand-in: call not understood: %s %s" % (method, path), 2)
json.dump(st, open(state_path, "w"))
if jq is None:
    print(json.dumps(out))
elif jq == ".total_count":
    print(out["total_count"])
elif jq == ".check_runs[].id":
    for r in out["check_runs"]:
        print(r["id"])
else:
    die("stand-in: --jq not understood: " + jq, 2)
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

impl Pr {
    fn new(yml: &str, tag: &str) -> Self {
        let dir = scratch(tag);
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("qa_cta: bin");
        std::fs::write(dir.join("fake_gh.py"), FAKE_GH).expect("qa_cta: stand-in gh");
        write_executable(
            &bin.join("gh"),
            format!(
                "#!/bin/sh\nexec python3 -I \"{}\" \"$@\"\n",
                dir.join("fake_gh.py").display()
            ),
        )
        .expect("qa_cta: gh");
        std::fs::write(
            dir.join("state.json"),
            r#"{"next": 0, "runs": [], "labels": []}"#,
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

    fn set_labels(&self, labels: &[&str]) {
        let mut st = self.state();
        st["labels"] = serde_json::json!(labels);
        std::fs::write(self.dir.join("state.json"), st.to_string()).expect("qa_cta: state");
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

    /// The head's `mutants-check` runs from the GitHub Actions app (id 15368, the source branch protection pins), each
    /// as `status/conclusion`.
    fn checks(&self, head: &str) -> Vec<String> {
        self.state()["runs"]
            .as_array()
            .expect("qa_cta: runs")
            .iter()
            .filter(|r| {
                r["head_sha"] == head && r["name"] == "mutants-check" && r["app_id"] == 15368
            })
            .map(|r| format!("{}/{}", text(&r["status"]), text(&r["conclusion"])))
            .collect()
    }

    /// The head's required `mutants-check` as branch protection reads it, strictly: `expected` with none, `pending`
    /// with any not completed, `failed` with any completed other than success, `passed` only with every one passed.
    fn gate(&self, head: &str) -> &'static str {
        let checks = self.checks(head);
        if checks.is_empty() {
            "expected"
        } else if checks.iter().any(|c| !c.starts_with("completed/")) {
            "pending"
        } else if checks.iter().any(|c| c != "completed/success") {
            "failed"
        } else {
            "passed"
        }
    }

    /// Event `e` on the pull request: the jobs that run, and in each the steps that call `gh`, run under bash as a
    /// runner runs them with their `env:`. The change classification says whether the change has no mutant (R-677:
    /// `no_mutant`, a change with no `.rs` file under `crates/`, `mutants=false`), and the verdict step's
    /// outcome is `verdict` (the shards ending likewise); every other step that calls no `gh` stands in as passed.
    /// Returns the jobs that ran, and each `gh` step's log.
    fn on(&self, e: &Ev, no_mutant: bool, verdict: &str) -> (BTreeSet<String>, String) {
        let results = jobs_run(&self.wf, e, verdict);
        let jobs = ran(&results);
        let mut log = String::new();
        for id in &jobs {
            let job = &self.wf["jobs"][id];
            let mut ctx = github(e);
            for n in needs_of(job) {
                ctx.needs.insert(n.clone(), results[&n].clone());
            }
            for (i, step) in job["steps"]
                .as_array()
                .expect("qa_cta: steps")
                .iter()
                .enumerate()
            {
                let sid = step.get("id").map(text).unwrap_or_else(|| format!("__{i}"));
                if sid == "changes" {
                    // R-677: the gate passes at once on `mutants=false`, whatever the tier. The classification here
                    // is never `docs_only`, so a workflow still passing on `docs_only` posts nothing passed.
                    let outputs = HashMap::from([
                        ("docs_only".to_owned(), "false".to_owned()),
                        ("mutants".to_owned(), (!no_mutant).to_string()),
                    ]);
                    ctx.steps.insert(sid, ("success".to_owned(), outputs));
                    continue;
                }
                if sid == "verdict" {
                    ctx.steps.insert(sid, (verdict.to_owned(), HashMap::new()));
                    if verdict != "success" {
                        ctx.failed = true;
                    }
                    continue;
                }
                let script = step.get("run").map(text).unwrap_or_default();
                if !calls_gh(&script) {
                    ctx.steps
                        .insert(sid, ("success".to_owned(), HashMap::new()));
                    continue;
                }
                if !ctx.holds(step.get("if")) {
                    ctx.steps
                        .insert(sid, ("skipped".to_owned(), HashMap::new()));
                    continue;
                }
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
                let outcome = if out.status.success() {
                    "success"
                } else {
                    ctx.failed = true;
                    "failure"
                };
                ctx.steps.insert(sid, (outcome.to_owned(), HashMap::new()));
            }
        }
        (jobs, log)
    }

    /// The watcher's label (with the user's token: GitHub adds it, then the `labeled` run starts), the shards and the
    /// verdict ending in `verdict`.
    fn merge_ready(
        &self,
        head: &str,
        no_mutant: bool,
        verdict: &str,
    ) -> (BTreeSet<String>, String) {
        let mut labels = self.labels();
        labels.retain(|l| l != "merge-ready");
        labels.push("merge-ready".to_owned());
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.set_labels(&refs);
        self.on(
            &ev("labeled", "merge-ready", head, &refs),
            no_mutant,
            verdict,
        )
    }

    /// A push of `head`: the `synchronize` event's payload lists the labels on the pull request when it fired.
    fn push(&self, head: &str, no_mutant: bool) -> (BTreeSet<String>, String) {
        let labels = self.labels();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.on(&ev("synchronize", "", head, &refs), no_mutant, "success")
    }
}

fn life(yml: &str) {
    let pr = Pr::new(yml, "life");
    let say = |what: &str, log: &str| format!("{what} (calls:\n{}log:\n{log})", pr.calls());

    // Opened on a code head: the gate is pending, not passed, until the merge-ready run.
    let (_, log) = pr.on(&ev("opened", "", "a", &[]), false, "success");
    assert_eq!(
        pr.gate("a"),
        "pending",
        "qa_cta: {}",
        say("a new code head's mutants-check is not pending", &log)
    );

    // Another label changes nothing.
    pr.set_labels(&["design"]);
    let (jobs, log) = pr.on(&ev("labeled", "design", "a", &["design"]), false, "success");
    assert!(
        jobs.is_empty() && pr.gate("a") == "pending",
        "qa_cta: {}",
        say(
            &format!(
                "another label ran {jobs:?} or moved the gate to {}",
                pr.gate("a")
            ),
            &log
        )
    );

    // The merge-ready run: its verdict, failure then (re-labelled) success, completes the head's check.
    let (_, log) = pr.merge_ready("a", false, "failure");
    assert_eq!(
        pr.gate("a"),
        "failed",
        "qa_cta: {}",
        say(
            "a failed merge-ready verdict did not fail mutants-check",
            &log
        )
    );
    pr.set_labels(&["design"]);
    let (_, log) = pr.merge_ready("a", false, "success");
    assert_eq!(
        pr.gate("a"),
        "passed",
        "qa_cta: {}",
        say("a passing merge-ready run did not pass mutants-check", &log)
    );
    assert_eq!(
        pr.checks("a").len(),
        1,
        "qa_cta: {}",
        say("the head has more than one mutants-check", "")
    );

    // A push: the label comes off, and the new head waits for its own merge-ready run.
    let (_, log) = pr.push("b", false);
    assert!(
        !pr.labels().contains(&"merge-ready".to_owned()),
        "qa_cta: {}",
        say("a push left the merge-ready label on", &log)
    );
    assert_eq!(
        pr.gate("b"),
        "pending",
        "qa_cta: {}",
        say("a pushed code head's mutants-check is not pending", &log)
    );

    // Never passed on a skip: a cancelled or skipped verdict is a failure.
    for verdict in ["cancelled", "skipped"] {
        pr.set_labels(&[]);
        let (_, log) = pr.merge_ready("b", false, verdict);
        assert_eq!(
            pr.gate("b"),
            "failed",
            "qa_cta: {}",
            say(
                &format!("a {verdict} merge-ready verdict did not fail mutants-check"),
                &log
            )
        );
    }
    pr.set_labels(&[]);
    let (_, log) = pr.merge_ready("b", false, "success");
    assert!(
        pr.gate("b") == "passed" && pr.checks("b").len() == 1,
        "qa_cta: {}",
        say(&format!("b's re-run left {:?}", pr.checks("b")), &log)
    );

    // Reopened on a head that passed: it stays passed.
    let labels = pr.labels();
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let (_, log) = pr.on(&ev("reopened", "", "b", &refs), false, "success");
    assert_eq!(
        pr.gate("b"),
        "passed",
        "qa_cta: {}",
        say(
            "a reopened pull request's passed head is no longer passed",
            &log
        )
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

    // A code push with no label: nothing to take off, the head pending.
    let (_, log) = pr.push("d", false);
    assert_eq!(
        pr.gate("d"),
        "pending",
        "qa_cta: {}",
        say("an unlabelled push's code head is not pending", &log)
    );

    // A head no earlier run posted on (its push run lost): the merge-ready run still reports.
    pr.set_labels(&[]);
    let (_, log) = pr.merge_ready("e", false, "success");
    assert!(
        pr.gate("e") == "passed" && pr.checks("e").len() == 1,
        "qa_cta: {}",
        say(
            &format!(
                "a head with no earlier mutants-check got {:?}",
                pr.checks("e")
            ),
            &log
        )
    );

    // Two pushes close together: the second's payload still lists the label the first's run has since taken off. The
    // head with no Rust under crates/ still passes at once, so no pull request waits on a check that never reports
    // (R-305, R-677).
    pr.set_labels(&[]);
    let (_, log) = pr.on(
        &ev("synchronize", "", "f", &["merge-ready"]),
        true,
        "success",
    );
    assert_eq!(
        pr.gate("f"),
        "passed",
        "qa_cta: {}",
        say(
            "a head with no Rust under crates/ pushed after the label came off is not passed at once",
            &log
        )
    );
}

#[test]
fn qa_cta_mutants_check_through_a_pull_requests_life() {
    life(&workflow("mutants.yml"));
}

negative_control!(
    qa_cta_mutants_check_through_a_pull_requests_life,
    "a verdict posted as a new check run, leaving the head's queued one",
    expected = "a failed merge-ready verdict did not fail mutants-check",
    life(&edited(
        "mutants.yml",
        "ids=$(gh api --paginate \"$runs&per_page=100\" --jq '.check_runs[].id')",
        "ids=\"\""
    ))
);

mod new_head_passed {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "every new head's mutants-check posted passed, with Rust under crates/ or not",
        expected = "a new code head's mutants-check is not pending",
        life(&edited(
            "mutants.yml",
            "          if [ \"$MUTANTS\" = false ]; then\n",
            "          if true; then\n"
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
            "          if [ \"$VERDICT\" = success ]; then\n",
            "          if [ \"$VERDICT\" != failure ]; then\n"
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

mod reopen_requeues {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a new head's check posted queued even where the head already has one",
        expected = "a reopened pull request's passed head is no longer passed",
        life(&edited(
            "mutants.yml",
            "          if [ \"$existing\" != 0 ]; then\n",
            "          if false; then\n"
        ))
    );
}

mod docs_only_waits {
    use super::*;
    negative_control!(
        qa_cta_mutants_check_through_a_pull_requests_life,
        "a head with no Rust under crates/ posted queued like a code head's",
        expected =
            "a head with no Rust under crates/: its mutants-check is pending, not passed at once",
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
            "(github.event.action == 'labeled' && github.event.label.name != 'merge-ready')",
            "(github.event.action == 'labeled')"
        ),
        &workflow("reviews.yml")
    )
);

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
