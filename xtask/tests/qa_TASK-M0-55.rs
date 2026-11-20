//! qa for TASK-M0-55, written from REQ-TOOL-224 and the rulings it cites, not from `ci_self_hosted.rs`:
//! - REQ-TOOL-224: "From the anchor move, the Windows runner's jobs on `main` must each be a named job reporting under
//!   its own name, never a required check: the GPU suites, Vulkan, nightly and on pushes to `main`; gpu-idle, the
//!   read-only GPU sample, by `workflow_dispatch`; and the weekly proof; the runner's security rules unchanged (R-859,
//!   R-860, R-711)." Its verify detail: "every self-hosted job across the workflows being one of the listed
//!   measurements, each named, and each listed one present, with the existing trigger, pin, guard, credential and
//!   not-gathered checks holding for the gpu-idle job; controls: an unlisted self-hosted job fails ..., a listed job
//!   removed fails, and the gpu-idle job under a push trigger fails".
//! - The list, each job placed by a ruling's words: R-859, "the GPU suites ... nightly and on pushes to `main`,
//!   gpu-idle, the weekly proof"; R-860 as in force, "`win-self-check` stays as the security rules' daily re-proof";
//!   R-912, option (b)'s Job Object proof (`win-contain`) and option (c)'s WSL2 distro, "built in parallel and never
//!   blocking" (`wsl-register`, `wsl-proofs`), with R-1014's "WSL proofs 3 and 6 finish"; R-1016 and R-1017, mutation
//!   runs returned to the PC, "the venue, the WSL2 distro under `contain.sh`" (`wsl-cpu`). R-1014 took `win-tests` off
//!   the PC, so it is not on the list.
//! - The task: gpu-idle is "`workflow_dispatch` only, whose one step runs the PC's read-only GPU sample script", taken
//!   "on an idle runner, with nothing alongside it"; both workflows' header comments "cite R-859 beside R-711".
//!
//! qa's own reading: the workflows are parsed with a YAML crate, and each self-hosted job's `if:` is evaluated, by a
//! small evaluator of the expression grammar it uses, under every event its workflow triggers on and every value of its
//! hand trigger's `only` input, so that when a job runs is read from what the condition computes, not from its text.
//! An identifier or construct the evaluator can't read fails the check. Each test has its negative controls (R-176).

use std::collections::BTreeSet;
use std::path::Path;

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WIN_GPU: &str = "win-gpu.yml";
const PROOF: &str = "runner-proof.yml";
const IDLE: &str = "gpu-idle";
/// The runner's ruled list, as (workflow, job) (R-859, R-860, R-912, R-1014, R-1016, R-1017).
const RULED: [(&str, &str); 8] = [
    (WIN_GPU, "win-vulkan"),
    (WIN_GPU, "win-self-check"),
    (WIN_GPU, "win-contain"),
    (WIN_GPU, "wsl-register"),
    (WIN_GPU, "wsl-proofs"),
    (WIN_GPU, "wsl-cpu"),
    (PROOF, "runner-proof"),
    (PROOF, IDLE),
];
/// The required checks, `plan/HUMAN_SETUP.md` § 2 (R-908, R-909).
const REQUIRED: [&str; 9] = [
    "fmt",
    "clippy",
    "build",
    "ci",
    "gpu-lavapipe",
    "gpu-metal",
    "pr-check",
    "reviews-complete",
    "no-atlas-params",
];
/// Triggers that would put a job on a pull request's path or into another workflow's run.
const PR_PATH: [&str; 4] = [
    "pull_request",
    "pull_request_target",
    "merge_group",
    "workflow_call",
];
const REPO: &str = "Ma1achy/Principia";
const OWNER: &str = "Ma1achy";

// ---- Reading the workflows ----------------------------------------------------------------------------------------

/// Every workflow, as (file name, text), sorted.
fn workflows() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("qa: .github/workflows is not readable")
        .map(|e| e.expect("qa: workflow entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let text =
                std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (name, text)
        })
        .collect();
    out.sort();
    assert!(
        out.iter().any(|(n, _)| n == WIN_GPU) && out.iter().any(|(n, _)| n == PROOF),
        "qa: the runner's two workflows aren't both there"
    );
    out
}

/// The committed workflows with `file`'s first `from` replaced by `to`; the edit target missing fails here, with a
/// message no control expects.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(f, t)| {
            if f == file {
                assert!(t.contains(from), "qa setup: {file} has no {from:?} to edit");
                let t = t.replacen(from, to, 1);
                (f, t)
            } else {
                (f, t)
            }
        })
        .collect()
}

/// The committed workflows with every `from` in `file` replaced by `to`.
#[cfg(feature = "controls")]
fn every(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(f, t)| {
            if f == file {
                assert!(t.contains(from), "qa setup: {file} has no {from:?} to edit");
                let t = t.replace(from, to);
                (f, t)
            } else {
                (f, t)
            }
        })
        .collect()
}

fn parse(file: &str, text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .unwrap_or_else(|e| panic!("qa: {file} doesn't parse: {e}"))
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("qa: {file} is empty"))
}

fn keys(value: &Yaml) -> Vec<String> {
    value
        .as_hash()
        .map(|h| {
            h.keys()
                .filter_map(|k| k.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The workflow's triggers.
fn triggers(doc: &Yaml) -> Vec<String> {
    match &doc["on"] {
        Yaml::String(s) => vec![s.clone()],
        Yaml::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        other => keys(other),
    }
}

/// A job's runner is the PC's, or can't be read (an expression, a group): either counts as self-hosted, so that no job
/// escapes the list by hiding its runner.
fn on_the_pc(job: &Yaml) -> bool {
    match &job["runs-on"] {
        Yaml::String(s) => s.contains("self-hosted") || s.contains("${{"),
        Yaml::Array(a) => a.iter().any(|v| {
            v.as_str()
                .is_none_or(|s| s == "self-hosted" || s.contains("${{"))
        }),
        Yaml::BadValue => false,
        _ => true,
    }
}

/// Every workflow, parsed, as (file, document).
fn docs(files: &[(String, String)]) -> Vec<(String, Yaml)> {
    files
        .iter()
        .map(|(f, t)| (f.clone(), parse(f, t)))
        .collect()
}

/// Every job on the PC across the workflows, as (file, id).
fn pc_jobs(docs: &[(String, Yaml)]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (file, doc) in docs {
        for id in keys(&doc["jobs"]) {
            if on_the_pc(&doc["jobs"][id.as_str()]) {
                out.push((file.clone(), id));
            }
        }
    }
    out
}

/// The name a job reports under: its `name:`, or its id.
fn reported(id: &str, job: &Yaml) -> String {
    job["name"]
        .as_str()
        .map_or_else(|| id.to_owned(), str::to_owned)
}

// ---- The condition evaluator --------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Str(String),
    Ident(String),
    Eq,
    Ne,
    And,
    Or,
    Not,
    Open,
    Close,
}

fn tokens(src: &str, what: &str) -> Vec<Tok> {
    let b: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        match c {
            ' ' | '\t' | '\n' => i += 1,
            '\'' => {
                let mut s = String::new();
                i += 1;
                loop {
                    match b.get(i) {
                        Some('\'') if b.get(i + 1) == Some(&'\'') => {
                            s.push('\'');
                            i += 2;
                        }
                        Some('\'') => {
                            i += 1;
                            break;
                        }
                        Some(ch) => {
                            s.push(*ch);
                            i += 1;
                        }
                        None => panic!("qa: cannot read {what}'s condition: an open string"),
                    }
                }
                out.push(Tok::Str(s));
            }
            '=' if next == Some('=') => {
                out.push(Tok::Eq);
                i += 2;
            }
            '!' if next == Some('=') => {
                out.push(Tok::Ne);
                i += 2;
            }
            '&' if next == Some('&') => {
                out.push(Tok::And);
                i += 2;
            }
            '|' if next == Some('|') => {
                out.push(Tok::Or);
                i += 2;
            }
            '!' => {
                out.push(Tok::Not);
                i += 1;
            }
            '(' => {
                out.push(Tok::Open);
                i += 1;
            }
            ')' => {
                out.push(Tok::Close);
                i += 1;
            }
            c if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' => {
                let start = i;
                while i < b.len()
                    && (b[i].is_ascii_alphanumeric() || matches!(b[i], '_' | '.' | '-'))
                {
                    i += 1;
                }
                out.push(Tok::Ident(b[start..i].iter().collect()));
            }
            other => panic!("qa: cannot read {what}'s condition: {other:?}"),
        }
    }
    out
}

/// A value of the expression language: GitHub compares strings without case, and a string is truthy when not empty.
#[derive(Debug, Clone)]
enum Val {
    Str(String),
    Bool(bool),
}

impl Val {
    fn truthy(&self) -> bool {
        match self {
            Val::Str(s) => !s.is_empty(),
            Val::Bool(b) => *b,
        }
    }
    fn equals(&self, other: &Val) -> bool {
        match (self, other) {
            (Val::Str(a), Val::Str(b)) => a.eq_ignore_ascii_case(b),
            (Val::Bool(a), Val::Bool(b)) => a == b,
            _ => false,
        }
    }
}

struct Eval<'a> {
    toks: Vec<Tok>,
    at: usize,
    ctx: &'a [(&'a str, String)],
    what: &'a str,
}

impl Eval<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }
    fn or(&mut self) -> Val {
        let mut v = self.and();
        while self.peek() == Some(&Tok::Or) {
            self.at += 1;
            let r = self.and();
            v = Val::Bool(v.truthy() || r.truthy());
        }
        v
    }
    fn and(&mut self) -> Val {
        let mut v = self.compare();
        while self.peek() == Some(&Tok::And) {
            self.at += 1;
            let r = self.compare();
            v = Val::Bool(v.truthy() && r.truthy());
        }
        v
    }
    /// `==` and `!=`, below `!` as GitHub's precedence has it.
    fn compare(&mut self) -> Val {
        let left = self.unary();
        match self.peek() {
            Some(Tok::Eq) => {
                self.at += 1;
                let r = self.unary();
                Val::Bool(left.equals(&r))
            }
            Some(Tok::Ne) => {
                self.at += 1;
                let r = self.unary();
                Val::Bool(!left.equals(&r))
            }
            _ => left,
        }
    }
    fn unary(&mut self) -> Val {
        if self.peek() == Some(&Tok::Not) {
            self.at += 1;
            return Val::Bool(!self.unary().truthy());
        }
        self.primary()
    }
    fn primary(&mut self) -> Val {
        let what = self.what;
        let tok = self
            .toks
            .get(self.at)
            .cloned()
            .unwrap_or_else(|| panic!("qa: cannot read {what}'s condition: it ends early"));
        self.at += 1;
        match tok {
            Tok::Open => {
                let v = self.or();
                assert!(
                    self.peek() == Some(&Tok::Close),
                    "qa: cannot read {what}'s condition: an unclosed bracket"
                );
                self.at += 1;
                v
            }
            Tok::Str(s) => Val::Str(s),
            Tok::Ident(id) if id == "true" => Val::Bool(true),
            Tok::Ident(id) if id == "false" => Val::Bool(false),
            Tok::Ident(id) => {
                assert!(
                    self.peek() != Some(&Tok::Open),
                    "qa: cannot read {what}'s condition: a call to {id}"
                );
                let v = self
                    .ctx
                    .iter()
                    .find(|(k, _)| *k == id)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| panic!("qa: cannot read {what}'s condition: {id}"));
                Val::Str(v)
            }
            other => panic!("qa: cannot read {what}'s condition: {other:?}"),
        }
    }
}

/// Whether a job with condition `cond` runs in `ctx`: no condition runs; `${{ }}` is stripped.
fn runs(cond: Option<&str>, ctx: &[(&str, String)], what: &str) -> bool {
    let Some(cond) = cond else { return true };
    let cond = cond.trim();
    let cond = cond
        .strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .unwrap_or(cond);
    let mut e = Eval {
        toks: tokens(cond, what),
        at: 0,
        ctx,
        what,
    };
    let v = e.or();
    assert!(
        e.at == e.toks.len(),
        "qa: cannot read {what}'s condition: something after its end"
    );
    v.truthy()
}

/// Every run a workflow can start as (event, its context): each trigger, and on a hand trigger each value of each
/// choice input, the other inputs at their defaults; a non-dispatch event sees every input empty. `repo` and `actor`
/// are the run's.
fn contexts(doc: &Yaml, repo: &str, actor: &str) -> Vec<(String, Vec<(&'static str, String)>)> {
    let inputs = &doc["on"]["workflow_dispatch"]["inputs"];
    let names: Vec<String> = keys(inputs);
    let text = |v: &Yaml| match v {
        Yaml::String(s) => s.clone(),
        Yaml::Boolean(b) => b.to_string(),
        Yaml::Integer(i) => i.to_string(),
        _ => String::new(),
    };
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    let mut out = Vec::new();
    for event in triggers(doc) {
        let base = vec![
            ("github.repository", repo.to_owned()),
            ("github.actor", actor.to_owned()),
            ("github.event_name", event.clone()),
        ];
        if event != "workflow_dispatch" {
            let mut ctx = base.clone();
            for n in &names {
                ctx.push((leak(format!("inputs.{n}")), String::new()));
            }
            out.push((event.clone(), ctx));
            continue;
        }
        // Each choice input over its options, one at a time, the others at their defaults.
        let defaults: Vec<(&'static str, String)> = names
            .iter()
            .map(|n| {
                (
                    leak(format!("inputs.{n}")),
                    text(&inputs[n.as_str()]["default"]),
                )
            })
            .collect();
        out.push((event.clone(), [base.clone(), defaults.clone()].concat()));
        for n in &names {
            let input = &inputs[n.as_str()];
            let mut values: Vec<String> = input["options"]
                .as_vec()
                .map(|o| o.iter().map(text).collect())
                .unwrap_or_default();
            if input["type"].as_str() == Some("boolean") {
                values = vec!["true".into(), "false".into()];
            }
            let key = format!("inputs.{n}");
            for v in values {
                let ctx: Vec<(&'static str, String)> = defaults
                    .iter()
                    .map(|(k, d)| (*k, if *k == key { v.clone() } else { d.clone() }))
                    .collect();
                out.push((event.clone(), [base.clone(), ctx].concat()));
            }
        }
    }
    out
}

/// The events `id` of `doc` runs under, as its owner, in this repository, sorted.
fn events_run(file: &str, doc: &Yaml, id: &str) -> BTreeSet<String> {
    let job = &doc["jobs"][id];
    let what = format!("{file}/{id}");
    contexts(doc, REPO, OWNER)
        .into_iter()
        .filter(|(_, ctx)| runs(job["if"].as_str(), ctx, &what))
        .map(|(e, _)| e)
        .collect()
}

fn find<'a>(docs: &'a [(String, Yaml)], file: &str) -> &'a Yaml {
    &docs
        .iter()
        .find(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("qa: no workflow {file}"))
        .1
}

// ---- 1. The ruled list, each by name ------------------------------------------------------------------------------

fn check_list(files: &[(String, String)]) {
    let docs = docs(files);
    let found: BTreeSet<(String, String)> = pc_jobs(&docs).into_iter().collect();
    let ruled: BTreeSet<(String, String)> = RULED
        .iter()
        .map(|(f, j)| ((*f).to_owned(), (*j).to_owned()))
        .collect();
    let mut wrong = Vec::new();
    for (f, j) in found.difference(&ruled) {
        wrong.push(format!(
            "qa: {f}/{j} runs on the PC, and no ruling places it there"
        ));
    }
    for (f, j) in ruled.difference(&found) {
        wrong.push(format!(
            "qa: {f}/{j}, on the ruled list, is not a job on the PC"
        ));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn qa_pc_jobs_are_exactly_the_ruled_list() {
    check_list(&workflows());
}

mod list_unlisted {
    use super::*;
    negative_control!(
        qa_pc_jobs_are_exactly_the_ruled_list,
        "an unlisted job on the PC in the measurement workflow",
        expected = "qa: win-gpu.yml/win-bench runs on the PC, and no ruling places it there",
        check_list(&edited(
            WIN_GPU,
            "\njobs:\n",
            "\njobs:\n  win-bench:\n    runs-on: [self-hosted, windows, nvidia]\n    steps: []\n"
        ))
    );
}

mod list_unlisted_elsewhere {
    use super::*;
    negative_control!(
        qa_pc_jobs_are_exactly_the_ruled_list,
        "a job on the PC in a hosted workflow",
        expected = "qa: ci.yml/pc runs on the PC, and no ruling places it there",
        check_list(&edited(
            "ci.yml",
            "\njobs:\n",
            "\njobs:\n  pc:\n    runs-on: self-hosted\n    steps: []\n"
        ))
    );
}

mod list_hidden_runner {
    use super::*;
    negative_control!(
        qa_pc_jobs_are_exactly_the_ruled_list,
        "a job whose runner is an expression",
        expected = "qa: ci.yml/pc runs on the PC, and no ruling places it there",
        check_list(&edited(
            "ci.yml",
            "\njobs:\n",
            "\njobs:\n  pc:\n    runs-on: ${{ inputs.where }}\n    steps: []\n"
        ))
    );
}

mod list_idle_removed {
    use super::*;
    negative_control!(
        qa_pc_jobs_are_exactly_the_ruled_list,
        "gpu-idle removed",
        expected = "qa: runner-proof.yml/gpu-idle, on the ruled list, is not a job on the PC",
        check_list(&edited(PROOF, "  gpu-idle:\n", "  gpu-sample:\n"))
    );
}

mod list_vulkan_off_the_pc {
    use super::*;
    negative_control!(
        qa_pc_jobs_are_exactly_the_ruled_list,
        "the Vulkan suites moved off the PC",
        expected = "qa: win-gpu.yml/win-vulkan, on the ruled list, is not a job on the PC",
        check_list(&{
            let files = workflows();
            let text = &files.iter().find(|(f, _)| f == WIN_GPU).unwrap().1;
            let at = text.find("  win-vulkan:\n").expect("qa setup: win-vulkan");
            let from = at
                + text[at..]
                    .find("runs-on: [self-hosted, windows, nvidia]")
                    .unwrap();
            let mut t = text.clone();
            t.replace_range(
                from..from + "runs-on: [self-hosted, windows, nvidia]".len(),
                "runs-on: ubuntu-24.04",
            );
            files
                .into_iter()
                .map(|(f, x)| if f == WIN_GPU { (f, t.clone()) } else { (f, x) })
                .collect::<Vec<_>>()
        })
    );
}

// ---- 2. Each under its own name, never a required check -----------------------------------------------------------

fn check_names(files: &[(String, String)]) {
    let docs = docs(files);
    let mut wrong = Vec::new();
    for (file, id) in pc_jobs(&docs) {
        let doc = find(&docs, &file);
        let name = reported(&id, &doc["jobs"][id.as_str()]);
        if name != id {
            wrong.push(format!(
                "qa: {file}/{id} reports under {name:?}, not its own name"
            ));
        }
        if REQUIRED.contains(&name.as_str()) {
            wrong.push(format!(
                "qa: {file}/{id} reports as the required check {name}"
            ));
        }
        for (other_file, other) in &docs {
            for other_id in keys(&other["jobs"]) {
                if (other_file, &other_id) == (&file, &id) {
                    continue;
                }
                if reported(&other_id, &other["jobs"][other_id.as_str()]) == name {
                    wrong.push(format!(
                        "qa: {other_file}/{other_id} reports under {file}/{id}'s name {name:?}"
                    ));
                }
            }
        }
        let doc_triggers = triggers(doc);
        for t in PR_PATH {
            if doc_triggers.iter().any(|x| x == t) {
                wrong.push(format!(
                    "qa: {file}, which runs {id} on the PC, triggers on {t}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn qa_pc_jobs_report_under_their_own_names_never_required() {
    check_names(&workflows());
}

mod names_renamed {
    use super::*;
    negative_control!(
        qa_pc_jobs_report_under_their_own_names_never_required,
        "gpu-idle reporting under another name",
        expected = "qa: runner-proof.yml/gpu-idle reports under \"GPU idle\", not its own name",
        check_names(&edited(
            PROOF,
            "  gpu-idle:\n",
            "  gpu-idle:\n    name: GPU idle\n"
        ))
    );
}

mod names_required {
    use super::*;
    negative_control!(
        qa_pc_jobs_report_under_their_own_names_never_required,
        "the Vulkan suites reporting as a required check",
        expected = "qa: win-gpu.yml/win-vulkan reports as the required check gpu-lavapipe",
        check_names(&edited(
            WIN_GPU,
            "  win-vulkan:\n",
            "  win-vulkan:\n    name: gpu-lavapipe\n"
        ))
    );
}

mod names_shared {
    use super::*;
    negative_control!(
        qa_pc_jobs_report_under_their_own_names_never_required,
        "a hosted job reporting under gpu-idle's name",
        expected = "qa: ci.yml/fmt reports under runner-proof.yml/gpu-idle's name \"gpu-idle\"",
        check_names(&edited(
            "ci.yml",
            "\n  fmt:\n",
            "\n  fmt:\n    name: gpu-idle\n"
        ))
    );
}

mod names_pull_request {
    use super::*;
    negative_control!(
        qa_pc_jobs_report_under_their_own_names_never_required,
        "a pull-request trigger on the weekly proof's workflow",
        expected = "qa: runner-proof.yml, which runs gpu-idle on the PC, triggers on pull_request",
        check_names(&edited(PROOF, "\non:\n", "\non:\n  pull_request:\n"))
    );
}

mod names_called {
    use super::*;
    negative_control!(
        qa_pc_jobs_report_under_their_own_names_never_required,
        "the measurement workflow callable from another, a required one among them",
        expected = "qa: win-gpu.yml, which runs win-vulkan on the PC, triggers on workflow_call",
        check_names(&edited(WIN_GPU, "\non:\n", "\non:\n  workflow_call:\n"))
    );
}

// ---- 3. When each runs, as its condition computes -----------------------------------------------------------------

fn set(events: &[&str]) -> BTreeSet<String> {
    events.iter().map(|e| (*e).to_owned()).collect()
}

fn check_events(files: &[(String, String)]) {
    let docs = docs(files);
    let mut wrong = Vec::new();

    // The Vulkan suites, nightly and on pushes to `main` (R-859).
    let gpu = find(&docs, WIN_GPU);
    let vulkan = events_run(WIN_GPU, gpu, "win-vulkan");
    for e in ["push", "schedule"] {
        if !vulkan.contains(e) {
            wrong.push(format!("qa: win-vulkan doesn't run on {e}: {vulkan:?}"));
        }
    }
    let branches: Vec<&str> = gpu["on"]["push"]["branches"]
        .as_vec()
        .map(|b| b.iter().filter_map(Yaml::as_str).collect())
        .unwrap_or_default();
    if branches != ["main"] {
        wrong.push(format!(
            "qa: {WIN_GPU}'s push is not to main alone: {branches:?}"
        ));
    }

    // The self-check, nightly: the security rules' daily re-proof (R-860).
    let check = events_run(WIN_GPU, gpu, "win-self-check");
    if !check.contains("schedule") {
        wrong.push(format!("qa: win-self-check doesn't run nightly: {check:?}"));
    }

    // gpu-idle by `workflow_dispatch` alone, and only on the dispatch that names it, with nothing alongside it.
    let proof = find(&docs, PROOF);
    let idle_job = &proof["jobs"][IDLE];
    for (event, ctx) in contexts(proof, REPO, OWNER) {
        let only = ctx
            .iter()
            .find(|(k, _)| *k == "inputs.only")
            .map_or("", |(_, v)| v.as_str());
        let idle = runs(idle_job["if"].as_str(), &ctx, "runner-proof.yml/gpu-idle");
        let named = event == "workflow_dispatch" && only == IDLE;
        if idle && event != "workflow_dispatch" {
            wrong.push(format!("qa: gpu-idle runs on {event}"));
        } else if idle != named {
            wrong.push(format!(
                "qa: gpu-idle runs={idle} on a dispatch naming {only:?}"
            ));
        }
        if idle {
            for other in keys(&proof["jobs"]) {
                if other != IDLE
                    && runs(
                        proof["jobs"][other.as_str()]["if"].as_str(),
                        &ctx,
                        &format!("{PROOF}/{other}"),
                    )
                {
                    wrong.push(format!(
                        "qa: {PROOF}/{other} runs alongside gpu-idle on {event}"
                    ));
                }
            }
        }
    }
    if !events_run(PROOF, proof, IDLE).contains("workflow_dispatch") {
        wrong.push("qa: gpu-idle can't be dispatched".to_owned());
    }

    // The weekly proof, on a weekly schedule.
    let weekly = events_run(PROOF, proof, "runner-proof");
    if !weekly.contains("schedule") {
        wrong.push(format!(
            "qa: the weekly proof doesn't run on its schedule: {weekly:?}"
        ));
    }
    let crons: Vec<&str> = proof["on"]["schedule"]
        .as_vec()
        .map(|s| s.iter().filter_map(|c| c["cron"].as_str()).collect())
        .unwrap_or_default();
    let weekly_cron = |c: &str| {
        let f: Vec<&str> = c.split_whitespace().collect();
        f.len() == 5
            && f[..4].iter().all(|x| x.parse::<u32>().is_ok() || *x == "*")
            && f[2] == "*"
            && f[3] == "*"
            && f[0].parse::<u32>().is_ok()
            && f[1].parse::<u32>().is_ok()
            && f[4].parse::<u32>().is_ok()
    };
    if crons.len() != 1 || !weekly_cron(crons[0]) {
        wrong.push(format!(
            "qa: the weekly proof's schedule isn't once a week: {crons:?}"
        ));
    }

    // R-711 item 1's triggers, the security rules unchanged: no job on the PC runs on any other event.
    for (file, id) in pc_jobs(&docs) {
        let doc = find(&docs, &file);
        for e in events_run(&file, doc, &id) {
            if !set(&["push", "schedule", "workflow_dispatch"]).contains(&e) {
                wrong.push(format!("qa: {file}/{id} runs on {e}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn qa_pc_jobs_run_when_their_rulings_say() {
    check_events(&workflows());
}

mod events_idle_on_push {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "gpu-idle under a push trigger",
        expected = "qa: gpu-idle runs on push",
        check_events(&{
            let files = edited(
                PROOF,
                "github.event_name == 'workflow_dispatch' && inputs.only == 'gpu-idle'",
                "github.event_name != 'schedule' && inputs.only != 'runner-proof'",
            );
            files
                .into_iter()
                .map(|(f, t)| {
                    if f == PROOF {
                        let t = t.replacen("\non:\n", "\non:\n  push:\n    branches: [main]\n", 1);
                        (f, t)
                    } else {
                        (f, t)
                    }
                })
                .collect::<Vec<_>>()
        })
    );
}

mod events_idle_on_schedule {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "gpu-idle on the weekly schedule",
        expected = "qa: gpu-idle runs on schedule",
        check_events(&edited(
            PROOF,
            "github.event_name == 'workflow_dispatch' && inputs.only == 'gpu-idle'",
            "inputs.only != 'runner-proof'"
        ))
    );
}

mod events_idle_alongside {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "the weekly proof on the dispatch naming gpu-idle",
        expected = "qa: runner-proof.yml/runner-proof runs alongside gpu-idle on workflow_dispatch",
        check_events(&edited(PROOF, " && inputs.only != 'gpu-idle'", ""))
    );
}

mod events_idle_on_plain_dispatch {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "gpu-idle on a dispatch that doesn't name it",
        expected = "qa: gpu-idle runs=true on a dispatch naming \"runner-proof\"",
        check_events(&edited(
            PROOF,
            "github.event_name == 'workflow_dispatch' && inputs.only == 'gpu-idle'",
            "github.event_name == 'workflow_dispatch'"
        ))
    );
}

mod events_idle_never {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "gpu-idle that no dispatch can start",
        expected = "qa: gpu-idle can't be dispatched",
        check_events(&edited(
            PROOF,
            "inputs.only == 'gpu-idle'",
            "inputs.only == 'never'"
        ))
    );
}

mod events_idle_or {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "an `||` that lets gpu-idle run on the schedule",
        expected = "qa: gpu-idle runs on schedule",
        check_events(&edited(
            PROOF,
            "inputs.only == 'gpu-idle'\n",
            "inputs.only == 'gpu-idle' || github.event_name == 'schedule'\n"
        ))
    );
}

mod events_vulkan_not_nightly {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "the Vulkan suites off the nightly schedule",
        expected = "qa: win-vulkan doesn't run on schedule",
        check_events(&edited(
            WIN_GPU,
            "github.actor == 'Ma1achy' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-vulkan')",
            "github.actor == 'Ma1achy' && github.event_name != 'schedule' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-vulkan')"
        ))
    );
}

mod events_vulkan_other_branch {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "the measurement workflow pushed from another branch",
        expected = "qa: win-gpu.yml's push is not to main alone",
        check_events(&edited(
            WIN_GPU,
            "branches: [main]",
            "branches: [main, dev]"
        ))
    );
}

mod events_self_check_gone_nightly {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "the self-check only by hand",
        expected = "qa: win-self-check doesn't run nightly",
        check_events(&edited(
            WIN_GPU,
            "github.event_name != 'push' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-self-check')",
            "github.event_name == 'workflow_dispatch' && (inputs.only == '' || inputs.only == 'all' || inputs.only == 'win-self-check')"
        ))
    );
}

mod events_proof_daily {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "the weekly proof made daily",
        expected = "qa: the weekly proof's schedule isn't once a week",
        check_events(&edited(PROOF, "\"0 5 * * 0\"", "\"0 5 * * *\""))
    );
}

mod events_unreadable {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "a condition the evaluator can't read, which fails rather than passes",
        expected = "qa: cannot read runner-proof.yml/gpu-idle's condition: a call to always",
        check_events(&edited(
            PROOF,
            "inputs.only == 'gpu-idle'\n",
            "inputs.only == 'gpu-idle' || always()\n"
        ))
    );
}

mod events_issue_comment {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_when_their_rulings_say,
        "a job on the PC started by a comment",
        expected = "qa: win-gpu.yml/win-vulkan runs on issue_comment",
        check_events(&edited(WIN_GPU, "\non:\n", "\non:\n  issue_comment:\n"))
    );
}

// ---- 4. The guard: no job on the PC runs for another repository or actor ------------------------------------------

fn check_guard(files: &[(String, String)]) {
    let docs = docs(files);
    let mut wrong = Vec::new();
    for (file, id) in pc_jobs(&docs) {
        let doc = find(&docs, &file);
        let job = &doc["jobs"][id.as_str()];
        let what = format!("{file}/{id}");
        for (repo, actor) in [
            ("someone/Principia", OWNER),
            (REPO, "someone"),
            ("someone/Principia", "someone"),
        ] {
            for (event, ctx) in contexts(doc, repo, actor) {
                if runs(job["if"].as_str(), &ctx, &what) {
                    wrong.push(format!("qa: {what} runs on {event} for {repo} by {actor}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn qa_pc_jobs_run_only_for_the_owner_here() {
    check_guard(&workflows());
}

mod guard_actor {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_only_for_the_owner_here,
        "the actor left out of gpu-idle's guard",
        expected = "qa: runner-proof.yml/gpu-idle runs on workflow_dispatch for Ma1achy/Principia by someone",
        check_guard(&edited(
            PROOF,
            "github.actor == 'Ma1achy' && github.event_name",
            "github.event_name"
        ))
    );
}

mod guard_repository {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_only_for_the_owner_here,
        "the repository left out of the weekly proof's guard",
        expected = "qa: runner-proof.yml/runner-proof runs on schedule for someone/Principia by Ma1achy",
        check_guard(&edited(
            PROOF,
            "if: github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && inputs.only != 'gpu-idle'",
            "if: github.actor == 'Ma1achy' && inputs.only != 'gpu-idle'"
        ))
    );
}

mod guard_or {
    use super::*;
    negative_control!(
        qa_pc_jobs_run_only_for_the_owner_here,
        "an `||` around the guard",
        expected = "qa: runner-proof.yml/gpu-idle runs on workflow_dispatch for someone/Principia by someone",
        check_guard(&edited(
            PROOF,
            "if: github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name",
            "if: github.repository == 'Ma1achy/Principia' || github.actor == 'Ma1achy' || github.event_name"
        ))
    );
}

// ---- 5. gpu-idle: the read-only sample, alone, with no credential -------------------------------------------------

/// The PC's sample script, which the task names and the test branch's two runs called.
const SAMPLE: &str = "06-gpu-sample.ps1";

fn check_idle(files: &[(String, String)]) {
    let docs = docs(files);
    let doc = find(&docs, PROOF);
    let job = &doc["jobs"][IDLE];
    let text = &files.iter().find(|(f, _)| f == PROOF).unwrap().1;
    let mut wrong = Vec::new();
    let steps = job["steps"].as_vec().cloned().unwrap_or_default();
    if steps.len() != 1 {
        wrong.push(format!(
            "qa: gpu-idle has {} steps, not its one",
            steps.len()
        ));
    }
    for step in &steps {
        if !step["uses"].is_badvalue() {
            wrong.push(format!("qa: gpu-idle uses an action: {:?}", step["uses"]));
        }
        if !step["continue-on-error"].is_badvalue() {
            wrong.push("qa: gpu-idle's step may fail and pass".to_owned());
        }
        let run = step["run"].as_str().unwrap_or("");
        let lines: Vec<&str> = run
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let starts = lines
            .first()
            .is_some_and(|l| l.starts_with("powershell ") && l.contains(SAMPLE));
        if !starts || lines.last() != Some(&"exit $LASTEXITCODE") || lines.len() != 2 {
            wrong.push(format!(
                "qa: gpu-idle's step isn't the sample alone, its status kept: {lines:?}"
            ));
        }
    }
    if !job["continue-on-error"].is_badvalue() {
        wrong.push("qa: gpu-idle may fail and pass".to_owned());
    }
    // No credential: the workflow reads contents only, the job asks for nothing more, and no secret is named anywhere
    // in its workflow.
    let perms = &doc["permissions"];
    if keys(perms) != ["contents"] || perms["contents"].as_str() != Some("read") {
        wrong.push(format!(
            "qa: {PROOF}'s permissions aren't contents: read: {perms:?}"
        ));
    }
    if !job["permissions"].is_badvalue() {
        wrong.push(format!(
            "qa: gpu-idle sets permissions: {:?}",
            job["permissions"]
        ));
    }
    if text.contains("secrets.") || text.contains("GITHUB_TOKEN") || text.contains("github.token") {
        wrong.push(format!("qa: {PROOF} names a credential"));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn qa_gpu_idle_is_the_read_only_sample_alone() {
    check_idle(&workflows());
}

mod idle_checkout {
    use super::*;
    negative_control!(
        qa_gpu_idle_is_the_read_only_sample_alone,
        "a checkout ahead of the sample",
        expected = "qa: gpu-idle has 2 steps, not its one",
        check_idle(&edited(
            PROOF,
            "      - name: Sample the GPU, read-only\n",
            "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1\n      - name: Sample the GPU, read-only\n"
        ))
    );
}

mod idle_exit_zero {
    use super::*;
    negative_control!(
        qa_gpu_idle_is_the_read_only_sample_alone,
        "the sample's status dropped",
        expected = "qa: gpu-idle's step isn't the sample alone, its status kept",
        check_idle(&edited(
            PROOF,
            "service-job\n          exit $LASTEXITCODE",
            "service-job\n          exit 0"
        ))
    );
}

mod idle_other_script {
    use super::*;
    negative_control!(
        qa_gpu_idle_is_the_read_only_sample_alone,
        "another script in place of the sample",
        expected = "qa: gpu-idle's step isn't the sample alone, its status kept",
        check_idle(&every(PROOF, "06-gpu-sample.ps1", "07-gpu-load.ps1"))
    );
}

mod idle_secret {
    use super::*;
    negative_control!(
        qa_gpu_idle_is_the_read_only_sample_alone,
        "a secret handed to gpu-idle",
        expected = "qa: runner-proof.yml names a credential",
        check_idle(&edited(
            PROOF,
            "    timeout-minutes: 14\n",
            "    timeout-minutes: 14\n    env:\n      T: ${{ secrets.PAT }}\n"
        ))
    );
}

mod idle_write {
    use super::*;
    negative_control!(
        qa_gpu_idle_is_the_read_only_sample_alone,
        "gpu-idle granted write",
        expected = "qa: gpu-idle sets permissions",
        check_idle(&edited(
            PROOF,
            "    timeout-minutes: 14\n",
            "    timeout-minutes: 14\n    permissions:\n      contents: write\n"
        ))
    );
}

mod idle_may_fail {
    use super::*;
    negative_control!(
        qa_gpu_idle_is_the_read_only_sample_alone,
        "gpu-idle passing whatever it finds",
        expected = "qa: gpu-idle may fail and pass",
        check_idle(&edited(
            PROOF,
            "    timeout-minutes: 14\n",
            "    timeout-minutes: 14\n    continue-on-error: true\n"
        ))
    );
}

// ---- 6. The header comments cite R-859 beside R-711 ---------------------------------------------------------------

/// The comment block a workflow opens with.
fn header(text: &str) -> String {
    text.lines()
        .take_while(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_headers(files: &[(String, String)]) {
    for file in [WIN_GPU, PROOF] {
        let text = &files
            .iter()
            .find(|(f, _)| f == file)
            .unwrap_or_else(|| panic!("qa: no workflow {file}"))
            .1;
        let head = header(text);
        for r in ["R-711", "R-859"] {
            assert!(
                head.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .any(|w| w == r),
                "qa: {file}'s header doesn't cite {r}"
            );
        }
    }
}

#[test]
fn qa_runner_headers_cite_r859_beside_r711() {
    check_headers(&workflows());
}

mod headers_proof {
    use super::*;
    negative_control!(
        qa_runner_headers_cite_r859_beside_r711,
        "R-859 gone from the weekly proof's header",
        expected = "qa: runner-proof.yml's header doesn't cite R-859",
        check_headers(&every(PROOF, "R-859", "the ruling"))
    );
}

mod headers_gpu {
    use super::*;
    negative_control!(
        qa_runner_headers_cite_r859_beside_r711,
        "R-859 gone from the measurement workflow's header",
        expected = "qa: win-gpu.yml's header doesn't cite R-859",
        check_headers(&every(WIN_GPU, "R-859", "the ruling"))
    );
}
