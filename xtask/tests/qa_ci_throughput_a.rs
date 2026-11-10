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
//! The owner's CI model (approved 9 Nov 2026) takes mutation testing off the pull request's path, so the first two
//! tests below are retired (see "Tests 1 and 2, retired"); the third, the concurrency groups, stands.
//!
//! The tests:
//! - `qa_cta_mutants_work_runs_only_on_the_merge_ready_run` (retired): on each event a pull request has (opened, a push,
//!   reopened, the `merge-ready` label, another label), which of `mutants.yml`'s jobs run, by evaluating each job's
//!   `if:` as a runner does (its implicit `success()` over the jobs it needs, those skipped included): the shards, the
//!   fixture and the verdict run on the `merge-ready` label's run alone, the verdict whatever the shards ended in;
//!   another label's run, put on or taken off, runs none of that work; and, under R-678, `mutants.yml`'s job
//!   `mutants-check` runs on every event, the one job of any workflow bearing that required name.
//! - `qa_cta_mutants_check_through_a_pull_requests_life` (retired): a pull request's life, event by event, against a stand-in
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

use std::collections::HashMap;
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

// ---- Tests 1 and 2, retired ---------------------------------------------------------------------------------------

// `qa_cta_mutants_work_runs_only_on_the_merge_ready_run` and `qa_cta_mutants_check_through_a_pull_requests_life`
// guarded mutation testing on the merge-ready head of every pull request and `mutants-check` reporting through its
// life. The owner's CI model (approved 9 Nov 2026; his order of 8 Nov: "Mutation testing: off the PR path entirely")
// retires both: `mutants.yml` runs on no pull request, and `mutants-check` is no longer a required check. What the
// model keeps of them, that no pull request workflow bears `mutants-check` and that mutants run on no pull request,
// `qa_tooling_pr_tier`'s `qa_pr_tier_mutants_are_off_the_pr_path` and `qa_TASK-M0-23_r305` pin. Their stand-ins
// (`jobs_run`, the stand-in `gh` and `Pr`) are retired with them (R-290's exception, applied per R-369).

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
