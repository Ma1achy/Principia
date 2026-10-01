//! QA tests for TASK-M0-42, written from REQ-SYS-073 and R-285: "CI must cache only the cargo registry and the
//! fixture pool (R-270), never whole target directories, each under a key naming its job, so the repository's Actions
//! cache stays well under GitHub's 10 GB limit"; verify: "no workflow caches a target directory; every cache key names
//! its job". The task adds: "The fixture-pool cache stays, saved only by the job that builds it." R-320 amends R-285:
//! the rust-gpu build, `~/.cache/rust-gpu`, joins the cached set (REQ-SYS-073, REQ-SYS-075), keyed on its job and the
//! pinned toolchain; `qa_TASK-M0-14_rust_gpu_cache.rs` checks that key's toolchain part. R-326 amends R-285 and R-320:
//! "caches are saved only on pushes to main; PR jobs restore only" (REQ-SYS-073: "every cache step saves only in a run
//! on a push to `main`, and a pull-request run restores only"). R-337 amends R-326: "a workflow that never runs on a
//! push to `main` restores, read-only, the key a `ci.yml` job saves there, and saves none" (REQ-SYS-073); "each step
//! in mutants.yml, pr-check.yml, reviews.yml, screenshot.yml and stand-in-soak.yml restores a key a `ci.yml` job saves
//! on `main`, and its comment names that key".
//!
//! These read every workflow under `.github/workflows/` and check the cache steps themselves:
//! - a `Swatinem/rust-cache` step turns off its target cache (it caches `target` by default) and its `~/.cargo/bin`
//!   cache (not the registry), and adds no directory of its own;
//! - an `actions/cache` step (save, restore or both) caches only the fixture pool, the cargo registry and git
//!   directories, or the rust-gpu build (R-320), never a target directory;
//! - no other action caches through a `cache:` input (setup-python's pip cache, for example);
//! - every cache key names its job: in a workflow that runs on a push to `main`, a rust-cache `shared-key` names the
//!   job it is in, and no two such jobs share one; an `actions/cache` key that saves names the job it is in, and a
//!   restore-only key a job that saves that path;
//! - in a workflow that never runs on a push to `main` (R-337), a rust-cache step's key is a `ci.yml` job's (the same
//!   `prefix-key` and `shared-key`, the shared-key that job's id), computed in the same environment: rust-cache hashes
//!   the variables named by its `env-vars` prefixes (default `CARGO CC CFLAGS CXX CMAKE RUST`) into the key, so the
//!   workflow and job `env:` entries with those prefixes are the `ci.yml` job's; an `actions/cache` step there
//!   restores a key a `ci.yml` job saves; no step there saves (no `actions/cache/save` or `actions/cache@…`); and a
//!   comment in the step, directly above it, or in the workflow's header names each key it restores; no comment says
//!   it restores what `main` saved under its own key;
//! - the fixture pool is saved by exactly one job, the `ci` job; any other job only restores it;
//! - every cache step restores in every run and saves only in a run on a push to `main` (R-326): a rust-cache step's
//!   `save-if` and an `actions/cache/save` step's `if:` each require both `github.event_name == 'push'` and
//!   `github.ref == 'refs/heads/main'`, joined by `&&` alone; no `actions/cache@…` step, which saves in every run it
//!   restores in; no restore step limited to pushes or to `main`; a restore-only step in a job that also saves that
//!   path names its own job.
//!
//! Timing and cache sizes are CI evidence, not checked here. Each test registers a negative control (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;

/// One step of a job: its `uses:` (empty for a `run:` step), its `if:` and its `with:` inputs, quotes stripped.
#[derive(Clone, Debug)]
struct Step {
    uses: String,
    cond: Option<String>,
    with: Vec<(String, String)>,
}

impl Step {
    fn input(&self, name: &str) -> Option<&str> {
        self.with
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Debug)]
struct Job {
    workflow: String,
    id: String,
    /// Whether the job's workflow runs on a push to `main`, the one run that saves a cache (R-326).
    on_main: bool,
    /// The `env:` entries in force for the job's steps, workflow-level then job-level, as (name, value).
    env: Vec<(String, String)>,
    steps: Vec<Step>,
}

fn workflows_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github/workflows")
}

/// Every workflow file, as (file name, text).
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(workflows_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no workflow files found");
    files
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    let v = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(v);
    v.to_owned()
}

/// A `branches:` or `branches-ignore:` filter's patterns, from `lines` (a trigger's body): inline (`[a, b]` or one
/// value) or a block list of `- ` entries below it. `None` if the filter is absent.
fn filter_list(lines: &[&str], name: &str) -> Option<Vec<String>> {
    let at = lines.iter().position(|l| {
        l.trim()
            .strip_prefix(name)
            .is_some_and(|r| r.starts_with(':'))
    })?;
    let head = lines[at].trim()[name.len() + 1..].trim();
    let mut v: Vec<String> = Vec::new();
    if !head.is_empty() {
        v.extend(
            head.trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(unquote)
                .filter(|p| !p.is_empty()),
        );
    } else {
        let ind = indent_of(lines[at]);
        for l in &lines[at + 1..] {
            let t = l.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if indent_of(l) <= ind && !t.starts_with("- ") {
                break;
            }
            match t.strip_prefix("- ") {
                Some(p) => v.push(unquote(p)),
                None => break,
            }
        }
    }
    Some(v)
}

/// Whether a branch filter pattern matches `main`: `main` itself, or a glob of `*`s alone.
fn matches_main(p: &str) -> bool {
    p == "main" || (!p.is_empty() && p.chars().all(|c| c == '*'))
}

/// Whether the workflow's `on:` runs it on a push to `main`: a `push` trigger (flow or block form) whose `branches`,
/// if given, match `main`, whose `branches-ignore` does not, and which is not limited to tags.
fn runs_on_push_to_main(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|l| ["on:", "\"on\":", "'on':"].iter().any(|k| l.starts_with(k)))
    else {
        return false;
    };
    let head = lines[start].split_once(':').unwrap().1.trim();
    if !head.is_empty() {
        return head
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .any(|e| e.trim() == "push");
    }
    let body: Vec<&str> = lines[start + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || l.trim_start().starts_with('#') || indent_of(l) > 0)
        .copied()
        .collect();
    let Some(p) = body
        .iter()
        .position(|l| indent_of(l) == 2 && (l.trim() == "push:" || l.trim().starts_with("push: ")))
    else {
        return false;
    };
    let push: Vec<&str> = body[p + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || l.trim_start().starts_with('#') || indent_of(l) > 2)
        .copied()
        .collect();
    let branches = filter_list(&push, "branches");
    let ignored = filter_list(&push, "branches-ignore");
    let tags = filter_list(&push, "tags").is_some() || filter_list(&push, "tags-ignore").is_some();
    if tags && branches.is_none() && ignored.is_none() {
        return false;
    }
    branches.is_none_or(|b| b.iter().any(|p| matches_main(p)))
        && !ignored.is_some_and(|b| b.iter().any(|p| matches_main(p)))
}

/// The `key: value` entries of the `env:` block that starts at `lines[at]`, whose entries sit deeper than it.
fn env_block(lines: &[&str], at: usize) -> Vec<(String, String)> {
    let ind = indent_of(lines[at]);
    let mut v = Vec::new();
    for l in &lines[at + 1..] {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if indent_of(l) <= ind {
            break;
        }
        if let Some((k, val)) = t.split_once(':') {
            v.push((k.trim().to_owned(), unquote(val)));
        }
    }
    v
}

/// The jobs of one workflow: job ids two spaces in under `jobs:`, steps as `- ` entries under `steps:`, and each
/// step's `uses:` and `with:` map. A block scalar input (`|`, `>`, `>-`) is read as its lines joined by newlines.
fn parse_jobs(file: &str, text: &str) -> Vec<Job> {
    let on_main = runs_on_push_to_main(text);
    let lines: Vec<&str> = text.lines().collect();
    let workflow_env = lines
        .iter()
        .position(|l| l.trim_end() == "env:" && indent_of(l) == 0)
        .map(|at| env_block(&lines, at))
        .unwrap_or_default();
    let mut jobs: Vec<Job> = Vec::new();
    let mut in_jobs = false;
    let mut step_indent: Option<usize> = None;
    let mut with_indent: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let body = line.trim_start();
        i += 1;
        if body.is_empty() || body.starts_with('#') {
            continue;
        }
        let ind = indent_of(line);
        if ind == 0 {
            in_jobs = body.trim_end() == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        if ind == 2 && body.ends_with(':') {
            jobs.push(Job {
                workflow: file.to_owned(),
                id: body.trim_end_matches(':').to_owned(),
                on_main,
                env: workflow_env.clone(),
                steps: Vec::new(),
            });
            step_indent = None;
            with_indent = None;
            continue;
        }
        let Some(job) = jobs.last_mut() else { continue };
        if ind == 4 && body.trim_end() == "env:" {
            job.env.extend(env_block(&lines, i - 1));
            continue;
        }
        if body.trim_end() == "steps:" {
            step_indent = None;
            with_indent = None;
            // The step list's dash sits at the next non-empty line's indent.
            continue;
        }
        let mut content = body;
        let mut content_indent = ind;
        if let Some(rest) = body.strip_prefix("- ") {
            if step_indent.is_none() || step_indent == Some(ind) {
                step_indent = Some(ind);
                job.steps.push(Step {
                    uses: String::new(),
                    cond: None,
                    with: Vec::new(),
                });
                with_indent = None;
                content = rest;
                content_indent = ind + 2;
            }
        }
        let Some(step) = job.steps.last_mut() else {
            continue;
        };
        if let Some(wi) = with_indent {
            if content_indent > wi {
                if let Some((k, v)) = content.split_once(':') {
                    let v = v.trim();
                    let value = if matches!(v, "|" | ">" | ">-" | "|-") {
                        let mut parts = Vec::new();
                        while i < lines.len()
                            && (lines[i].trim().is_empty() || indent_of(lines[i]) > content_indent)
                        {
                            parts.push(lines[i].trim().to_owned());
                            i += 1;
                        }
                        parts.join("\n")
                    } else {
                        unquote(v)
                    };
                    step.with.push((k.trim().to_owned(), value));
                }
                continue;
            }
            with_indent = None;
        }
        if let Some(v) = content.strip_prefix("uses:") {
            step.uses = unquote(v);
        } else if let (Some(v), true) = (
            content.strip_prefix("if:"),
            step_indent.is_some_and(|d| content_indent == d + 2),
        ) {
            step.cond = Some(unquote(v));
        } else if content.trim_end() == "with:" {
            with_indent = Some(content_indent);
        }
    }
    jobs
}

fn all_jobs(files: &[(String, String)]) -> Vec<Job> {
    files.iter().flat_map(|(f, t)| parse_jobs(f, t)).collect()
}

fn is_rust_cache(step: &Step) -> bool {
    step.uses.starts_with("Swatinem/rust-cache@")
}

/// `actions/cache@…` (restore and save), `actions/cache/restore@…` or `actions/cache/save@…`.
fn actions_cache_kind(step: &Step) -> Option<&'static str> {
    let u = step.uses.as_str();
    if u.starts_with("actions/cache/restore@") {
        Some("restore")
    } else if u.starts_with("actions/cache/save@") {
        Some("save")
    } else if u.starts_with("actions/cache@") {
        Some("both")
    } else {
        None
    }
}

fn saves(kind: &str) -> bool {
    kind != "restore"
}

const FIXTURE_POOL: &str = "target/tmp/fixture-targets";

fn is_false(v: Option<&str>) -> bool {
    matches!(
        v.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("false")
    )
}

/// `key`'s `-`-separated words contain `job`'s words, in order and contiguous.
fn names_job(key: &str, job: &str) -> bool {
    format!("-{key}-").contains(&format!("-{job}-"))
}

// ---- 1. No target directory, no ~/.cargo/bin, through rust-cache -------------------------------------------------

fn check_rust_cache_caches_no_target(jobs: &[Job]) {
    let mut seen = 0;
    for job in jobs {
        for step in job.steps.iter().filter(|s| is_rust_cache(s)) {
            seen += 1;
            let at = format!("{} job `{}`", job.workflow, job.id);
            assert!(
                is_false(step.input("cache-targets")),
                "{at}: Swatinem/rust-cache step caches a target directory (cache-targets is {:?}, not \"false\")",
                step.input("cache-targets")
            );
            assert!(
                is_false(step.input("cache-bin")),
                "{at}: Swatinem/rust-cache step caches ~/.cargo/bin (cache-bin is {:?}, not \"false\")",
                step.input("cache-bin")
            );
            assert!(
                step.input("cache-directories").is_none(),
                "{at}: Swatinem/rust-cache step caches extra directories: {:?}",
                step.input("cache-directories")
            );
        }
    }
    assert!(
        seen > 0,
        "no Swatinem/rust-cache step found: the parser read no step"
    );
}

/// The workflows with `from` replaced by `to` in the named file.
/// Only the controls use it.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files.iter_mut().find(|(f, _)| f == file).unwrap();
    assert!(text.contains(from), "{file} has no {from:?} to edit");
    *text = text.replacen(from, to, 1);
    files
}

#[test]
fn qa_m0_42_rust_cache_turns_its_target_cache_off() {
    check_rust_cache_caches_no_target(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_rust_cache_turns_its_target_cache_off,
    "pr-check.yml with its rust-cache step's target cache turned back on",
    expected = "caches a target directory",
    check_rust_cache_caches_no_target(&all_jobs(&edited(
        "pr-check.yml",
        "cache-targets: \"false\"",
        "cache-targets: \"true\""
    )))
);

#[test]
fn qa_m0_42_no_rust_cache_step_keeps_its_default_target_cache() {
    check_rust_cache_caches_no_target(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_no_rust_cache_step_keeps_its_default_target_cache,
    "screenshot.yml with a bare `Swatinem/rust-cache@v2` step, whose default caches `target`",
    expected = "caches a target directory",
    check_rust_cache_caches_no_target(&all_jobs(&edited(
        "screenshot.yml",
        "      - uses: Swatinem/rust-cache@v2\n        with:\n          prefix-key: cargo-registry\n          shared-key: ci\n          cache-targets: \"false\"\n          cache-bin: \"false\"\n",
        "      - uses: Swatinem/rust-cache@v2\n"
    )))
);

#[test]
fn qa_m0_42_rust_cache_turns_its_bin_cache_off() {
    check_rust_cache_caches_no_target(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_rust_cache_turns_its_bin_cache_off,
    "the soak workflow with its rust-cache step caching ~/.cargo/bin",
    expected = "caches ~/.cargo/bin",
    check_rust_cache_caches_no_target(&all_jobs(&edited(
        "stand-in-soak.yml",
        "cache-bin: \"false\"",
        "cache-bin: \"true\""
    )))
);

// ---- 2. actions/cache caches only the fixture pool, the registry or the rust-gpu build; nothing else caches -------

/// The rust-gpu build (R-320, amending R-285): cargo-gpu's backend build, the one directory beside the registry and the
/// pool that CI may cache.
const RUST_GPU: &str = "~/.cache/rust-gpu";

fn allowed_cache_path(p: &str) -> bool {
    let p = p.trim().trim_end_matches('/');
    p == FIXTURE_POOL
        || p == RUST_GPU
        || p.starts_with("~/.cargo/registry")
        || p.starts_with("~/.cargo/git")
}

fn check_cache_paths(jobs: &[Job]) {
    for job in jobs {
        for step in &job.steps {
            let at = format!("{} job `{}` step `{}`", job.workflow, job.id, step.uses);
            if actions_cache_kind(step).is_some() {
                let path = step.input("path").unwrap_or("");
                assert!(
                    !path.trim().is_empty(),
                    "{at}: an actions/cache step with no path"
                );
                for p in path.lines().map(str::trim).filter(|p| !p.is_empty()) {
                    assert!(
                        allowed_cache_path(p),
                        "{at}: caches {p:?}, which is not the cargo registry, the fixture pool or the rust-gpu build"
                    );
                }
            } else if !is_rust_cache(step) {
                if let Some(c) = step.input("cache") {
                    assert!(
                        c.trim().is_empty() || is_false(Some(c)),
                        "{at}: caches through its `cache: {c}` input, which is not the cargo registry, the fixture pool or the rust-gpu build"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_m0_42_actions_cache_holds_only_the_pool_or_the_registry() {
    check_cache_paths(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_actions_cache_holds_only_the_pool_or_the_registry,
    "ci.yml with the fixture-pool cache widened to the whole `target` directory",
    expected = "which is not the cargo registry, the fixture pool or the rust-gpu build",
    check_cache_paths(&all_jobs(&edited(
        "ci.yml",
        "          path: target/tmp/fixture-targets\n          key: fixture-pool-ci-",
        "          path: target\n          key: fixture-pool-ci-"
    )))
);

#[test]
fn qa_m0_42_the_rust_gpu_cache_is_its_directory_alone() {
    check_cache_paths(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_the_rust_gpu_cache_is_its_directory_alone,
    "mutants.yml with the rust-gpu cache widened to all of ~/.cache, more than R-320 admits",
    expected = "caches \"~/.cache\", which is not the cargo registry, the fixture pool or the rust-gpu build",
    check_cache_paths(&all_jobs(&edited(
        "mutants.yml",
        "          path: ~/.cache/rust-gpu\n",
        "          path: ~/.cache\n"
    )))
);

#[test]
fn qa_m0_42_no_other_action_caches_through_a_cache_input() {
    check_cache_paths(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_no_other_action_caches_through_a_cache_input,
    "ci.yml with setup-python's pip cache turned on",
    expected = "caches through its `cache: pip` input",
    check_cache_paths(&all_jobs(&edited(
        "ci.yml",
        "          python-version: \"3.12\"\n",
        "          python-version: \"3.12\"\n          cache: pip\n"
    )))
);

// ---- 3. Every cache key names its job ----------------------------------------------------------------------------

/// rust-cache's default `env-vars` prefixes: the variables whose values it hashes into its key.
const RUST_CACHE_ENV: [&str; 6] = ["CARGO", "CC", "CFLAGS", "CXX", "CMAKE", "RUST"];

/// The `env:` entries in force for `job` that rust-cache hashes into `step`'s key, sorted, a job's entry replacing the
/// workflow's.
fn hashed_env(job: &Job, step: &Step) -> Vec<(String, String)> {
    let extra: Vec<String> = step
        .input("env-vars")
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let mut m = std::collections::BTreeMap::new();
    for (k, v) in &job.env {
        if RUST_CACHE_ENV.iter().any(|p| k.starts_with(p))
            || extra.iter().any(|p| k.starts_with(p.as_str()))
        {
            m.insert(k.clone(), v.clone());
        }
    }
    m.into_iter().collect()
}

fn check_keys_name_their_job(jobs: &[Job]) {
    // rust-cache: in a workflow that runs on a push to `main`, a shared-key names the job it is in; without one,
    // rust-cache keys on the job id itself. No two such jobs share a key. In a workflow that never runs on a push to
    // `main`, the key is one a `ci.yml` job saves there, computed in the same environment (R-337).
    let mut shared: Vec<(String, String)> = Vec::new();
    for job in jobs {
        for step in job.steps.iter().filter(|s| is_rust_cache(s)) {
            let at = format!("{} job `{}`", job.workflow, job.id);
            if !job.on_main {
                let sk = step.input("shared-key").unwrap_or("");
                let saver = jobs.iter().find(|j| {
                    j.workflow == "ci.yml"
                        && j.on_main
                        && j.id == sk
                        && j.steps.iter().any(|t| {
                            is_rust_cache(t)
                                && t.input("shared-key") == Some(sk)
                                && t.input("prefix-key") == step.input("prefix-key")
                        })
                });
                let Some(saver) = saver else {
                    panic!(
                        "{at}: rust-cache key `{}-{sk}` is one no `ci.yml` job saves on `main`, so it restores nothing \
                         (R-337)",
                        step.input("prefix-key").unwrap_or("v0-rust")
                    );
                };
                let theirs = saver
                    .steps
                    .iter()
                    .find(|t| is_rust_cache(t) && t.input("shared-key") == Some(sk))
                    .unwrap();
                assert_eq!(
                    hashed_env(job, step),
                    hashed_env(saver, theirs),
                    "{at}: rust-cache hashes another environment into its key than `ci.yml`'s `{}` job does, so it \
                     computes another key than the one it restores (R-337)",
                    saver.id
                );
                for input in [
                    "env-vars",
                    "add-rust-environment-hash-key",
                    "add-job-id-key",
                    "key",
                ] {
                    assert_eq!(
                        step.input(input),
                        theirs.input(input),
                        "{at}: rust-cache's `{input}` differs from `ci.yml`'s `{}` job's, so it computes another key \
                         (R-337)",
                        saver.id
                    );
                }
                continue;
            }
            if let Some(sk) = step.input("shared-key") {
                assert!(
                    names_job(sk, &job.id),
                    "{at}: rust-cache shared-key {sk:?} does not name its job `{}`",
                    job.id
                );
                let full = format!("{}-{sk}", step.input("prefix-key").unwrap_or("v0-rust"));
                if let Some((other, _)) = shared.iter().find(|(_, k)| *k == full) {
                    panic!(
                        "{at}: rust-cache key {full:?} is shared with {other}, so it does not name its job"
                    );
                }
                shared.push((at, full));
            }
        }
    }
    // actions/cache: the key (and every restore key) names the one job that saves that path.
    let mut savers: Vec<(String, String, bool)> = Vec::new(); // (path, job id, saved by a ci.yml job)
    for job in jobs {
        for step in &job.steps {
            if let Some(kind) = actions_cache_kind(step) {
                if saves(kind) {
                    savers.push((
                        step.input("path").unwrap_or("").to_owned(),
                        job.id.clone(),
                        job.workflow == "ci.yml" && job.on_main,
                    ));
                }
            }
        }
    }
    for job in jobs {
        for step in &job.steps {
            if actions_cache_kind(step).is_none() {
                continue;
            }
            let at = format!("{} job `{}` step `{}`", job.workflow, job.id, step.uses);
            let path = step.input("path").unwrap_or("");
            // A workflow that never runs on a push to `main` restores a key a `ci.yml` job saves there (R-337).
            let owners: Vec<&str> = savers
                .iter()
                .filter(|(p, _, ci)| p == path && (job.on_main || *ci))
                .map(|(_, j, _)| j.as_str())
                .collect();
            assert!(
                !owners.is_empty(),
                "{at}: restores {path:?}, which no job saves"
            );
            let key = step.input("key").unwrap_or("");
            // A step that saves names its own job (R-320's path is saved by several jobs, each under its own key), and
            // so does a restore-only step in a job that also saves the path (R-326 splits restore from save); any other
            // restore-only step names a job that saves the path.
            let own = [job.id.as_str()];
            let own_saves = job.steps.iter().any(|s| {
                actions_cache_kind(s).is_some_and(saves) && s.input("path").unwrap_or("") == path
            });
            let owners: Vec<&str> = match actions_cache_kind(step) {
                Some(kind) if saves(kind) => own.to_vec(),
                _ if own_saves => own.to_vec(),
                _ => owners,
            };
            let names = |k: &str| {
                owners.iter().any(|o| {
                    k.split("${{")
                        .next()
                        .is_some_and(|lit| names_job(lit.trim_end_matches('-'), o))
                })
            };
            assert!(
                names(key),
                "{at}: cache key {key:?} does not name its job ({owners:?})"
            );
            for rk in step
                .input("restore-keys")
                .unwrap_or("")
                .lines()
                .filter(|l| !l.trim().is_empty())
            {
                assert!(
                    names(rk.trim()),
                    "{at}: restore key {rk:?} does not name its job ({owners:?})"
                );
            }
        }
    }
}

#[test]
fn qa_m0_42_rust_cache_shared_key_names_its_job() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_rust_cache_shared_key_names_its_job,
    "ci.yml with gpu-metal's rust-cache shared-key set to another job's",
    expected =
        "ci.yml job `gpu-metal`: rust-cache shared-key \"gpu-lavapipe\" does not name its job",
    check_keys_name_their_job(&all_jobs(&edited(
        "ci.yml",
        "shared-key: gpu-metal\n",
        "shared-key: gpu-lavapipe\n"
    )))
);

#[test]
fn qa_m0_42_a_saving_cache_key_names_its_own_job() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_saving_cache_key_names_its_own_job,
    "ci.yml with xtask-ci saving the rust-gpu build under the ci job's key, a path other jobs save too (R-320)",
    expected = "job `xtask-ci` step `actions/cache/save@v4`: cache key \"rust-gpu-ci-",
    check_keys_name_their_job(&all_jobs(&edited(
        "ci.yml",
        "github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-",
        "github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-ci-"
    )))
);

#[test]
fn qa_m0_42_a_restore_key_in_a_saving_job_names_its_own_job() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_restore_key_in_a_saving_job_names_its_own_job,
    "ci.yml with gpu-kernel restoring the rust-gpu build under xtask-ci's key, though gpu-kernel saves its own",
    expected = "step `actions/cache/restore@v4`: cache key \"rust-gpu-xtask-ci-",
    check_keys_name_their_job(&all_jobs(&edited(
        "ci.yml",
        "key: rust-gpu-gpu-kernel-",
        "key: rust-gpu-xtask-ci-"
    )))
);

#[test]
fn qa_m0_42_no_two_jobs_share_a_rust_cache_key() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_no_two_jobs_share_a_rust_cache_key,
    "ci.yml with the ci job's shared-key made xtask-ci's, which contains the word ci but is another job's key",
    expected = "is shared with",
    check_keys_name_their_job(&all_jobs(&edited(
        "ci.yml",
        "shared-key: ci\n",
        "shared-key: xtask-ci\n"
    )))
);

#[test]
fn qa_m0_42_fixture_pool_key_names_its_job() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_fixture_pool_key_names_its_job,
    "ci.yml with the pre-R-285 fixture-pool key, which names no job",
    expected = "does not name its job",
    check_keys_name_their_job(&all_jobs(
        &workflows()
            .into_iter()
            .map(|(f, t)| {
                let t = if f == "ci.yml" {
                    t.replace("fixture-pool-ci-", "fixture-pool-")
                } else {
                    t
                };
                (f, t)
            })
            .collect::<Vec<_>>()
    ))
);

#[test]
fn qa_m0_42_fixture_pool_restore_keys_name_their_job() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_fixture_pool_restore_keys_name_their_job,
    "ci.yml with the fixture-pool restore key naming no job",
    expected = "restore key",
    check_keys_name_their_job(&all_jobs(&edited(
        "ci.yml",
        "          restore-keys: fixture-pool-ci-",
        "          restore-keys: fixture-pool-"
    )))
);

// ---- 4. Only `ci` saves the fixture pool -------------------------------------------------------------------------

fn check_only_ci_saves_the_pool(jobs: &[Job]) {
    let mut savers = Vec::new();
    for job in jobs {
        for step in &job.steps {
            if let Some(kind) = actions_cache_kind(step) {
                if saves(kind)
                    && step
                        .input("path")
                        .unwrap_or("")
                        .lines()
                        .any(|p| p.trim() == FIXTURE_POOL)
                {
                    savers.push(format!("{} job `{}`", job.workflow, job.id));
                }
            }
        }
    }
    assert!(
        savers.len() == 1 && savers[0] == "ci.yml job `ci`",
        "the fixture pool must be saved by the `ci` job alone; saved by {savers:?}"
    );
}

#[test]
fn qa_m0_42_only_the_ci_job_saves_the_fixture_pool() {
    check_only_ci_saves_the_pool(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_only_the_ci_job_saves_the_fixture_pool,
    "ci.yml with xtask-ci's restore-only pool step turned into a restore-and-save step",
    expected = "saved by the `ci` job alone",
    check_only_ci_saves_the_pool(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/restore@v4\n        with:\n          path: target/tmp/fixture-targets\n          key: fixture-pool-ci-${{ runner.os }}-${{ steps.toolchain.outputs.cachekey }}-${{ hashFiles('Cargo.lock', 'fixtures/**/Cargo.toml', 'xtask/tests/fixtures/**/Cargo.toml') }}\n          restore-keys: fixture-pool-ci-${{ runner.os }}-${{ steps.toolchain.outputs.cachekey }}-\n      # The rust-gpu build",
        "      - uses: actions/cache@v4\n        with:\n          path: target/tmp/fixture-targets\n          key: fixture-pool-ci-${{ runner.os }}-${{ steps.toolchain.outputs.cachekey }}-${{ hashFiles('Cargo.lock', 'fixtures/**/Cargo.toml', 'xtask/tests/fixtures/**/Cargo.toml') }}\n          restore-keys: fixture-pool-ci-${{ runner.os }}-${{ steps.toolchain.outputs.cachekey }}-\n      # The rust-gpu build"
    )))
);

#[test]
fn qa_m0_42_the_ci_job_saves_the_fixture_pool() {
    check_only_ci_saves_the_pool(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_the_ci_job_saves_the_fixture_pool,
    "ci.yml with the ci job's pool save step made restore-only, so no job saves the pool",
    expected = "saved by the `ci` job alone",
    check_only_ci_saves_the_pool(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: target/tmp/fixture-targets",
        "      - uses: actions/cache/restore@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: target/tmp/fixture-targets"
    )))
);

// ---- The parser reads what the checks rely on --------------------------------------------------------------------

/// Guards the checks above against a parser that reads nothing: every workflow has a job, every job its steps, and
/// the steps the task names are found with their inputs.
fn check_parser_reads_the_workflows(files: &[(String, String)]) {
    for (f, t) in files {
        let jobs = parse_jobs(f, t);
        assert!(!jobs.is_empty(), "{f}: parser read no job");
        for j in &jobs {
            assert!(
                !j.steps.is_empty(),
                "{f} job `{}`: parser read no step",
                j.id
            );
        }
    }
    let jobs = all_jobs(files);
    let ci = jobs
        .iter()
        .find(|j| j.workflow == "ci.yml" && j.id == "ci")
        .expect("parser read no `ci` job");
    for kind in ["restore", "save"] {
        let pool = ci
            .steps
            .iter()
            .find(|s| actions_cache_kind(s) == Some(kind))
            .unwrap_or_else(|| panic!("parser read no fixture-pool {kind} step in the `ci` job"));
        assert_eq!(
            pool.input("path"),
            Some(FIXTURE_POOL),
            "parser misread the pool's path"
        );
    }
    assert!(
        jobs.iter()
            .flat_map(|j| &j.steps)
            .any(|s| actions_cache_kind(s) == Some("save") && s.cond.is_some()),
        "parser read no `if:` on any cache save step"
    );
}

#[test]
fn qa_m0_42_parser_reads_the_workflows() {
    check_parser_reads_the_workflows(&workflows());
}

negative_control!(
    qa_m0_42_parser_reads_the_workflows,
    "ci.yml with its `ci` job's step list removed",
    expected = "parser read no step",
    check_parser_reads_the_workflows(
        &workflows()
            .into_iter()
            .map(|(f, t)| {
                if f != "ci.yml" {
                    return (f, t);
                }
                let mut out = Vec::new();
                let mut skipping = false;
                for line in t.lines() {
                    if line == "  ci:" {
                        out.push(line.to_owned());
                        out.push("    runs-on: ubuntu-latest".to_owned());
                        skipping = true;
                        continue;
                    }
                    if skipping
                        && indent_of(line) == 2
                        && !line.trim().is_empty()
                        && !line.trim().starts_with('#')
                    {
                        skipping = false;
                    }
                    if !skipping {
                        out.push(line.to_owned());
                    }
                }
                (f, out.join("\n"))
            })
            .collect::<Vec<_>>()
    )
);

// ---- 5. Every cache restores in every run and saves only on a push to `main` (R-326) ------------------------------

/// The `&&`-joined terms of a condition, `${{ }}` and whitespace stripped; `None` if it uses `||` or `!` anywhere a
/// conjunction cannot account for (a negated or alternative term could admit another event or branch).
fn conj_terms(cond: &str) -> Option<Vec<String>> {
    let c = cond.trim();
    let c = c
        .strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .unwrap_or(c);
    if c.contains("||") || c.replace("!=", "").contains('!') {
        return None;
    }
    Some(
        c.split("&&")
            .map(|t| t.chars().filter(|ch| !ch.is_whitespace()).collect())
            .collect(),
    )
}

const ON_PUSH: &str = "github.event_name=='push'";
const ON_MAIN: &str = "github.ref=='refs/heads/main'";

/// The condition admits only a run on a push to `main`: a conjunction holding both terms.
fn main_push_only(cond: Option<&str>) -> bool {
    cond.and_then(conj_terms)
        .is_some_and(|t| t.iter().any(|x| x == ON_PUSH) && t.iter().any(|x| x == ON_MAIN))
}

fn check_saved_only_on_main(jobs: &[Job]) {
    let mut seen = 0;
    for job in jobs {
        for step in &job.steps {
            let at = format!("{} job `{}` step `{}`", job.workflow, job.id, step.uses);
            if is_rust_cache(step) {
                seen += 1;
                assert!(
                    main_push_only(step.input("save-if")),
                    "{at}: saves in a run other than a push to `main` (save-if is {:?}) (R-326)",
                    step.input("save-if")
                );
                continue;
            }
            if !job.on_main && actions_cache_kind(step).is_some_and(saves) {
                panic!(
                    "{at}: saves a cache, though {} never runs on a push to `main`; it restores a `ci.yml` job's key \
                     read-only and saves none (R-337)",
                    job.workflow
                );
            }
            match actions_cache_kind(step) {
                Some("both") => panic!(
                    "{at}: restores and saves in one step, so it saves in every run it restores in, a pull-request \
                     run too (R-326)"
                ),
                Some("save") => {
                    seen += 1;
                    assert!(
                        main_push_only(step.cond.as_deref()),
                        "{at}: saves in a run other than a push to `main` (if: {:?}) (R-326)",
                        step.cond
                    );
                }
                Some(_) => {
                    seen += 1;
                    let terms = step
                        .cond
                        .as_deref()
                        .and_then(conj_terms)
                        .unwrap_or_default();
                    assert!(
                        !terms
                            .iter()
                            .any(|t| t.starts_with("github.event_name")
                                || t.starts_with("github.ref")),
                        "{at}: does not restore in every run: it is limited by {:?} (R-326)",
                        step.cond
                    );
                }
                None => {}
            }
        }
    }
    assert!(seen > 0, "no cache step found: the parser read no step");
}

#[test]
fn qa_m0_42_rust_cache_saves_only_on_a_push_to_main() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_rust_cache_saves_only_on_a_push_to_main,
    "pr-check.yml with its rust-cache step's save-if removed, so it saves in every run",
    expected =
        "job `pr-check` step `Swatinem/rust-cache@v2`: saves in a run other than a push to `main`",
    check_saved_only_on_main(&all_jobs(&edited(
        "pr-check.yml",
        "          save-if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}\n",
        ""
    )))
);

#[test]
fn qa_m0_42_rust_cache_save_if_is_a_conjunction() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_rust_cache_save_if_is_a_conjunction,
    "reviews.yml with its save-if an `||`, which a pull-request run satisfies",
    expected = "job `reviews-complete` step `Swatinem/rust-cache@v2`: saves in a run other than a push to `main`",
    check_saved_only_on_main(&all_jobs(&edited(
        "reviews.yml",
        "save-if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}",
        "save-if: ${{ github.event_name == 'push' || github.ref == 'refs/heads/main' }}"
    )))
);

#[test]
fn qa_m0_42_a_save_step_saves_only_on_a_push_to_main() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_save_step_saves_only_on_a_push_to_main,
    "ci.yml with the ci job's pool save step unconditioned, so a pull-request run saves the pool",
    expected = "job `ci` step `actions/cache/save@v4`: saves in a run other than a push to `main`",
    check_saved_only_on_main(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: target/tmp/fixture-targets",
        "      - uses: actions/cache/save@v4\n        with:\n          path: target/tmp/fixture-targets"
    )))
);

#[test]
fn qa_m0_42_a_save_step_saves_only_on_main() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_save_step_saves_only_on_main,
    "ci.yml with xtask-ci's rust-gpu save on a push to any branch",
    expected =
        "job `xtask-ci` step `actions/cache/save@v4`: saves in a run other than a push to `main`",
    check_saved_only_on_main(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-"
    )))
);

#[test]
fn qa_m0_42_no_step_restores_and_saves_at_once() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_no_step_restores_and_saves_at_once,
    "ci.yml with gpu-kernel's rust-gpu restore step turned back into `actions/cache@v4`, which saves in every run",
    expected = "job `gpu-kernel` step `actions/cache@v4`: restores and saves in one step",
    check_saved_only_on_main(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/restore@v4\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-",
        "      - uses: actions/cache@v4\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-"
    )))
);

#[test]
fn qa_m0_42_every_restore_runs_in_every_run() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_every_restore_runs_in_every_run,
    "ci.yml with the ci job's pool restore limited to pushes to main, so a pull-request run starts cold",
    expected = "job `ci` step `actions/cache/restore@v4`: does not restore in every run",
    check_saved_only_on_main(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/restore@v4\n        with:\n          path: target/tmp/fixture-targets",
        "      - uses: actions/cache/restore@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: target/tmp/fixture-targets"
    )))
);

// ---- 6. A workflow that never runs on a push to `main` restores a `ci.yml` key read-only (R-337) --------------------

#[test]
fn qa_m0_42_a_pr_only_rust_cache_restores_a_ci_yml_key() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_pr_only_rust_cache_restores_a_ci_yml_key,
    "reviews.yml with its pre-R-337 rust-cache key, its own job's, which nothing saves",
    expected = "reviews.yml job `reviews-complete`: rust-cache key `cargo-registry-reviews-complete` is one no `ci.yml` job saves",
    check_keys_name_their_job(&all_jobs(&edited(
        "reviews.yml",
        "shared-key: ci\n",
        "shared-key: reviews-complete\n"
    )))
);

#[test]
fn qa_m0_42_a_pr_only_rust_cache_key_is_computed_in_ci_yml_s_environment() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_pr_only_rust_cache_key_is_computed_in_ci_yml_s_environment,
    "pr-check.yml without the CARGO_TERM_COLOR ci.yml sets, which rust-cache hashes into its key",
    expected = "pr-check.yml job `pr-check`: rust-cache hashes another environment into its key than `ci.yml`'s `ci` job does",
    check_keys_name_their_job(&all_jobs(&edited(
        "pr-check.yml",
        "env:\n  CARGO_TERM_COLOR: always\n",
        ""
    )))
);

#[test]
fn qa_m0_42_a_pr_only_job_env_enters_the_key_too() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_pr_only_job_env_enters_the_key_too,
    "screenshot.yml's job setting RUSTFLAGS, which ci.yml's `ci` job does not",
    expected =
        "screenshot.yml job `screenshot`: rust-cache hashes another environment into its key",
    check_keys_name_their_job(&all_jobs(&edited(
        "screenshot.yml",
        "    env:\n      PRIN_GPU_BACKEND: vulkan\n",
        "    env:\n      PRIN_GPU_BACKEND: vulkan\n      RUSTFLAGS: -Dwarnings\n"
    )))
);

#[test]
fn qa_m0_42_a_pr_only_restore_reads_a_ci_yml_saved_path() {
    check_keys_name_their_job(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_a_pr_only_restore_reads_a_ci_yml_saved_path,
    "mutants.yml restoring the rust-gpu build under its own pre-R-337 key, which no ci.yml job saves",
    expected = "job `mutants` step `actions/cache/restore@v4`: cache key \"rust-gpu-mutants-",
    check_keys_name_their_job(&all_jobs(&edited(
        "mutants.yml",
        "key: rust-gpu-gpu-kernel-",
        "key: rust-gpu-mutants-"
    )))
);

#[test]
fn qa_m0_42_no_pr_only_workflow_saves() {
    check_saved_only_on_main(&all_jobs(&workflows()));
}

negative_control!(
    qa_m0_42_no_pr_only_workflow_saves,
    "stand-in-soak.yml with an actions/cache/save step, conditioned on a push to `main`, after its rust-cache step",
    expected = "stand-in-soak.yml job `soak` step `actions/cache/save@v4`: saves a cache, though stand-in-soak.yml never runs on a push to `main`",
    check_saved_only_on_main(&all_jobs(&edited(
        "stand-in-soak.yml",
        "          save-if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}\n",
        "          save-if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}\n      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cargo/registry\n          key: cargo-registry-ci-soak\n"
    )))
);

/// The keys a PR-only workflow's cache steps restore, each as the literal the comments are to name: a rust-cache
/// step's `<prefix-key>-<shared-key>`, an `actions/cache` step's key up to its first `${{`, trailing `-` dropped. Each
/// with the step's own comments and the comments directly above it.
fn restored_keys_with_comments(text: &str) -> Vec<(String, Vec<String>)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        let Some(uses) = t.strip_prefix("- uses:").map(str::trim) else {
            continue;
        };
        let rust_cache = uses.starts_with("Swatinem/rust-cache@");
        if !rust_cache && !uses.starts_with("actions/cache") {
            continue;
        }
        let dash = indent_of(l);
        let mut comments: Vec<String> = lines[..i]
            .iter()
            .rev()
            .take_while(|p| p.trim_start().starts_with('#'))
            .map(|p| p.trim().to_owned())
            .collect();
        let body: Vec<&str> = lines[i + 1..]
            .iter()
            .take_while(|b| {
                b.trim().is_empty()
                    || (indent_of(b) > dash && !b.trim().starts_with("- "))
                    || (b.trim_start().starts_with('#') && indent_of(b) > dash)
            })
            .copied()
            .collect();
        comments.extend(
            body.iter()
                .filter(|b| b.trim_start().starts_with('#'))
                .map(|b| b.trim().to_owned()),
        );
        let input = |k: &str| {
            body.iter()
                .find_map(|b| b.trim().strip_prefix(k).and_then(|r| r.strip_prefix(':')))
                .map(unquote)
        };
        let key = if rust_cache {
            format!(
                "{}-{}",
                input("prefix-key").unwrap_or_else(|| "v0-rust".to_owned()),
                input("shared-key").unwrap_or_default()
            )
        } else {
            input("key")
                .unwrap_or_default()
                .split("${{")
                .next()
                .unwrap_or("")
                .trim_end_matches('-')
                .to_owned()
        };
        out.push((key, comments));
    }
    out
}

/// R-337's comment half: in each workflow that never runs on a push to `main`, a comment names each key a cache step
/// restores, in the step, directly above it or in the workflow's header; no comment says the step restores what `main`
/// saved under its own key.
fn check_pr_only_comments(files: &[(String, String)]) {
    let mut seen = 0;
    for (f, t) in files.iter().filter(|(_, t)| !runs_on_push_to_main(t)) {
        let header: Vec<String> = t
            .lines()
            .take_while(|l| l.trim_start().starts_with('#') || l.trim().is_empty())
            .map(|l| l.trim().to_owned())
            .collect();
        let joined = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .map(|l| l.trim().trim_start_matches('#').trim())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !joined.contains("restores what `main` saved under its key")
                && !joined.contains("restores what `main` saved under their keys"),
            "{f}: a comment says it restores what `main` saved under its own key, which nothing saves (R-337)"
        );
        for (key, comments) in restored_keys_with_comments(t) {
            seen += 1;
            assert!(
                !key.is_empty(),
                "{f}: a cache step whose key the check could not read"
            );
            assert!(
                comments.iter().chain(&header).any(|c| c.contains(&key)),
                "{f}: no comment in its cache step, above it or in its header names the key it restores, `{key}` (R-337)"
            );
        }
    }
    assert!(seen > 0, "no cache step found in a PR-only workflow");
}

#[test]
fn qa_m0_42_a_pr_only_cache_comment_names_its_key() {
    check_pr_only_comments(&workflows());
}

negative_control!(
    qa_m0_42_a_pr_only_cache_comment_names_its_key,
    "screenshot.yml's comment naming no key",
    expected = "screenshot.yml: no comment in its cache step, above it or in its header names the key it restores, `cargo-registry-ci`",
    check_pr_only_comments(&edited(
        "screenshot.yml",
        "`cargo-registry-ci-…`",
        "the `ci` job's"
    ))
);

#[test]
fn qa_m0_42_no_pr_only_comment_says_main_saved_its_key() {
    check_pr_only_comments(&workflows());
}

negative_control!(
    qa_m0_42_no_pr_only_comment_says_main_saved_its_key,
    "pr-check.yml with its pre-R-337 comment back",
    expected = "pr-check.yml: a comment says it restores what `main` saved under its own key",
    check_pr_only_comments(&edited(
        "pr-check.yml",
        "which this workflow never is, so it saves nothing (R-326). It",
        "which this workflow never is: it restores what `main` saved under its key, and saves nothing (R-326). It"
    ))
);
