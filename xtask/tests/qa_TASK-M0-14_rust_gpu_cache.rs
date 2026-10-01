//! QA tests for TASK-M0-14's rust-gpu build cache, written from REQ-SYS-075 ("Every CI job that builds the kernel must
//! restore the rust-gpu build cache (`~/.cache/rust-gpu`), and save it only in a run on a push to `main` (R-326), under
//! a key naming its job (R-285, REQ-SYS-073) and the pinned toolchain version from `rust-toolchain.toml`, so a
//! toolchain bump starts a fresh cache (R-320)") and REQ-SYS-073 (every cache key names its job; every cache step saves
//! only in a run on a push to `main`, and a pull-request run restores only).
//!
//! They read every workflow under `.github/workflows/`. A job builds the kernel when a step runs `build-kernel`
//! (`cargo xtask build-kernel`, or the cargo call the alias expands to) or `cargo xtask ci`, whose registry runs
//! build-kernel (`xtask/tests/ci.rs`). For each such job:
//! - an `actions/cache/restore` step restores `~/.cache/rust-gpu` before the first build, and runs whenever the build
//!   does (no `if:` the build step lacks) (R-320, R-326);
//! - in a workflow that runs on a push to `main`, an `actions/cache/save` step after the build saves it under the key
//!   the restore reads, only in a run on a push to `main`: its `if:` is the build's condition `&&`
//!   `github.event_name == 'push'` `&&` `github.ref == 'refs/heads/main'`; no `actions/cache@…` step, which saves in
//!   every run it restores in (R-326);
//! - its key names the job, and no two such jobs share a key (R-285);
//! - in a workflow that never runs on a push to `main` (R-337 names mutants.yml, pr-check.yml, reviews.yml,
//!   screenshot.yml and stand-in-soak.yml), the job saves no rust-gpu build, and restores it, read-only, under the key
//!   `ci.yml`'s `gpu-kernel` job saves on `main`: the same key, resolved as the runner would (R-337, REQ-SYS-075);
//! - its key, resolved as the runner would (each `steps.<id>.outputs.<name>` it uses computed by running that step's
//!   `echo "<name>=…" >> "$GITHUB_OUTPUT"` line in the repository), contains the channel `rust-toolchain.toml` pins, and
//!   so does every restore key: no fallback reaches a cache built by another toolchain.
//!
//! Each test registers a negative control (R-176).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;

const RUST_GPU: &str = "~/.cache/rust-gpu";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every workflow file, as (file name, text).
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
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

/// The workflows with `from` replaced by `to` in the named file. Only the controls use it.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files.iter_mut().find(|(f, _)| f == file).unwrap();
    assert!(text.contains(from), "{file} has no {from:?} to edit");
    *text = text.replacen(from, to, 1);
    files
}

/// The workflows with every `from` replaced by its `to` in the named file, in order. Only the controls use it.
#[cfg(feature = "controls")]
fn edited_all(file: &str, edits: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files.iter_mut().find(|(f, _)| f == file).unwrap();
    for (from, to) in edits {
        assert!(text.contains(from), "{file} has no {from:?} to edit");
        *text = text.replace(from, to);
    }
    files
}

/// The channel `rust-toolchain.toml` pins, read here independently of xtask.
fn pinned_channel() -> String {
    let text = std::fs::read_to_string(root().join("rust-toolchain.toml")).unwrap();
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("channel"))
        .expect("rust-toolchain.toml pins no channel");
    let v = line.split_once('=').unwrap().1.trim().trim_matches('"');
    assert!(!v.is_empty(), "rust-toolchain.toml's channel is empty");
    v.to_owned()
}

#[derive(Clone, Debug, Default)]
struct Step {
    keys: BTreeMap<String, String>,
    with: BTreeMap<String, String>,
}

impl Step {
    fn get(&self, k: &str) -> Option<&str> {
        self.keys.get(k).map(String::as_str)
    }
    fn input(&self, k: &str) -> Option<&str> {
        self.with.get(k).map(String::as_str)
    }
}

#[derive(Clone, Debug)]
struct Job {
    workflow: String,
    id: String,
    /// Whether the job's workflow runs on a push to `main`, the one run that saves a cache (R-326).
    on_main: bool,
    steps: Vec<Step>,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(v)
        .to_owned()
}

/// A YAML block mapping whose keys sit at `ind`: each `key: value`, a block scalar (`|`, `>`, `>-`, …) or a nested
/// block read as its lines (trimmed, comments dropped) joined by newlines.
fn parse_map(lines: &[String], ind: usize) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        let l = &lines[i];
        i += 1;
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || indent(l) != ind {
            continue;
        }
        let Some((k, v)) = t.split_once(':') else {
            continue;
        };
        let v = v.trim();
        let mut body = Vec::new();
        while i < lines.len() && (lines[i].trim().is_empty() || indent(&lines[i]) > ind) {
            body.push(lines[i].clone());
            i += 1;
        }
        let value = if v.is_empty() || v.starts_with('|') || v.starts_with('>') {
            body.iter()
                .map(|b| b.trim())
                .filter(|b| !b.is_empty() && !b.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            unquote(v)
        };
        if k.trim() == "with" {
            let wind = body
                .iter()
                .find(|b| !b.trim().is_empty() && !b.trim().starts_with('#'))
                .map(|b| indent(b))
                .unwrap_or(ind + 2);
            map.insert(
                "\u{0}with".to_owned(),
                serde_like_join(&parse_map(&body, wind)),
            );
        }
        map.insert(k.trim().to_owned(), value);
    }
    map
}

/// A nested map flattened to `key\u{1}value` lines, for the step to split back.
fn serde_like_join(m: &BTreeMap<String, String>) -> String {
    m.iter()
        .map(|(k, v)| format!("{k}\u{1}{}", v.replace('\n', "\u{2}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn split_with(s: &str) -> BTreeMap<String, String> {
    s.lines()
        .filter_map(|l| l.split_once('\u{1}'))
        .map(|(k, v)| (k.to_owned(), v.replace('\u{2}', "\n")))
        .collect()
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
        let ind = indent(lines[at]);
        for l in &lines[at + 1..] {
            let t = l.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if indent(l) <= ind && !t.starts_with("- ") {
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
        .take_while(|l| l.trim().is_empty() || l.trim_start().starts_with('#') || indent(l) > 0)
        .copied()
        .collect();
    let Some(p) = body
        .iter()
        .position(|l| indent(l) == 2 && (l.trim() == "push:" || l.trim().starts_with("push: ")))
    else {
        return false;
    };
    let push: Vec<&str> = body[p + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || l.trim_start().starts_with('#') || indent(l) > 2)
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

/// The jobs of one workflow: ids two spaces in under `jobs:`, each `- ` entry under `steps:` a step.
fn parse_jobs(file: &str, text: &str) -> Vec<Job> {
    let on_main = runs_on_push_to_main(text);
    let lines: Vec<&str> = text.lines().collect();
    let mut jobs: Vec<Job> = Vec::new();
    let mut in_jobs = false;
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let t = l.trim();
        i += 1;
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if indent(l) == 0 {
            in_jobs = t == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        if indent(l) == 2 && t.ends_with(':') {
            jobs.push(Job {
                workflow: file.to_owned(),
                id: t.trim_end_matches(':').to_owned(),
                on_main,
                steps: Vec::new(),
            });
            continue;
        }
        if t != "steps:" {
            continue;
        }
        let Some(job) = jobs.last_mut() else { continue };
        let steps_ind = indent(l);
        // The steps: each from its dash to the next dash at the same indent, or the end of the list.
        let mut dash: Option<usize> = None;
        let mut cur: Vec<String> = Vec::new();
        while i < lines.len() {
            let s = lines[i];
            let st = s.trim();
            if !st.is_empty() && !st.starts_with('#') && indent(s) <= steps_ind {
                break;
            }
            i += 1;
            if st.is_empty() || st.starts_with('#') {
                continue;
            }
            if st.starts_with("- ") && (dash.is_none() || dash == Some(indent(s))) {
                let d = indent(s);
                dash = Some(d);
                if !cur.is_empty() {
                    job.steps.push(step_of(&cur, d + 2));
                }
                cur = vec![format!("{}{}", " ".repeat(d + 2), &st[2..])];
            } else {
                cur.push(s.to_owned());
            }
        }
        if let (Some(d), false) = (dash, cur.is_empty()) {
            job.steps.push(step_of(&cur, d + 2));
        }
    }
    jobs
}

fn step_of(lines: &[String], ind: usize) -> Step {
    let mut keys = parse_map(lines, ind);
    let with = keys
        .remove("\u{0}with")
        .map(|w| split_with(&w))
        .unwrap_or_default();
    Step { keys, with }
}

fn all_jobs(files: &[(String, String)]) -> Vec<Job> {
    files.iter().flat_map(|(f, t)| parse_jobs(f, t)).collect()
}

/// Whether a `run:` script builds the kernel: a command line running `build-kernel`, or `cargo xtask ci`.
fn builds_kernel(run: &str) -> bool {
    run.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .any(|l| {
            let words: Vec<&str> = l.split_whitespace().collect();
            words.contains(&"build-kernel")
                || words.windows(3).any(|w| w == ["cargo", "xtask", "ci"])
        })
}

/// `actions/cache@…` (both), `actions/cache/restore@…` or `actions/cache/save@…`.
fn cache_kind(step: &Step) -> Option<&'static str> {
    let u = step.get("uses")?;
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

fn caches_rust_gpu(step: &Step) -> bool {
    cache_kind(step).is_some()
        && step
            .input("path")
            .unwrap_or("")
            .lines()
            .any(|p| p.trim().trim_end_matches('/') == RUST_GPU)
}

/// The jobs that build the kernel, each with the index of its first building step.
fn kernel_jobs(jobs: &[Job]) -> Vec<(&Job, usize)> {
    jobs.iter()
        .filter_map(|j| {
            j.steps
                .iter()
                .position(|s| s.get("run").is_some_and(builds_kernel))
                .map(|b| (j, b))
        })
        .collect()
}

/// `key`'s `-`-separated words contain `job`'s, contiguous.
fn names_job(key: &str, job: &str) -> bool {
    format!("-{key}-").contains(&format!("-{job}-"))
}

/// `key` as the runner would compute it in `job`: `runner.os` is `Linux`; `steps.<id>.outputs.<name>` is what the
/// step with that id writes for `<name>`, computed by running its `echo "<name>=…" >> "$GITHUB_OUTPUT"` lines in the
/// repository; any other expression is left in place, unresolved.
fn resolve(job: &Job, key: &str) -> String {
    let mut out = String::new();
    let mut rest = key;
    while let Some(start) = rest.find("${{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let end = after.find("}}").expect("an unclosed ${{ in a cache key");
        let expr = after[..end].trim();
        rest = &after[end + 2..];
        if expr == "runner.os" {
            out.push_str("Linux");
        } else if let Some(r) = expr.strip_prefix("steps.") {
            let (id, name) = r.split_once(".outputs.").expect("a steps expression");
            out.push_str(&step_output(job, id, name));
        } else {
            out.push_str("${{ ");
            out.push_str(expr);
            out.push_str(" }}");
        }
    }
    out.push_str(rest);
    out
}

fn step_output(job: &Job, id: &str, name: &str) -> String {
    let Some(step) = job.steps.iter().find(|s| s.get("id") == Some(id)) else {
        return String::new();
    };
    let lines: Vec<&str> = step
        .get("run")
        .unwrap_or("")
        .lines()
        .filter(|l| l.contains("GITHUB_OUTPUT") && l.contains(&format!("{name}=")))
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_m014_rust_gpu_cache");
    std::fs::create_dir_all(&dir).unwrap();
    // One file per call: the tests run in parallel and resolve the same keys, in one process (cargo test) or one
    // process each (nextest), so the name holds the process id beside the per-process count.
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let n = CALL.fetch_add(1, Ordering::Relaxed);
    let file = dir.join(format!(
        "{}-{}-{id}-{name}-{}-{n}",
        job.workflow,
        job.id,
        std::process::id()
    ));
    let _ = std::fs::remove_file(&file);
    let status = Command::new("bash")
        .arg("-c")
        .arg(lines.join("\n"))
        .current_dir(root())
        .env("GITHUB_OUTPUT", &file)
        .status()
        .expect("bash runs");
    assert!(status.success(), "{}: `{id}`'s output line failed", job.id);
    std::fs::read_to_string(&file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.strip_prefix(&format!("{name}=")))
        .next_back()
        .unwrap_or("")
        .to_owned()
}

// ---- 1. Every job that builds the kernel restores ~/.cache/rust-gpu, and saves it only on a push to main ----------

/// A condition's `&&`-joined terms, `${{ }}` and whitespace stripped; `None` for an empty condition. A term holding
/// `||` or a `!` other than `!=` is kept whole, so it never matches a plain term.
fn terms(cond: Option<&str>) -> Vec<String> {
    let Some(c) = cond else { return Vec::new() };
    let c = c.trim();
    let c = c
        .strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .unwrap_or(c);
    if c.contains("||") || c.replace("!=", "").contains('!') {
        return vec![c.to_owned()];
    }
    let mut t: Vec<String> = c
        .split("&&")
        .map(|t| t.chars().filter(|ch| !ch.is_whitespace()).collect())
        .collect();
    t.sort();
    t
}

const ON_PUSH: &str = "github.event_name=='push'";
const ON_MAIN: &str = "github.ref=='refs/heads/main'";

/// `ci.yml`'s `gpu-kernel` job and the key its rust-gpu save step saves under on `main`: the key a job in a workflow
/// that never runs on a push to `main` restores (R-337).
fn gpu_kernel_key(jobs: &[Job]) -> (&Job, &str) {
    let gk = jobs
        .iter()
        .find(|j| j.workflow == "ci.yml" && j.id == "gpu-kernel" && j.on_main)
        .expect("no `gpu-kernel` job in ci.yml running on a push to `main` (R-325, R-337)");
    let key = gk
        .steps
        .iter()
        .find(|s| caches_rust_gpu(s) && cache_kind(s) == Some("save"))
        .and_then(|s| s.input("key"))
        .expect("ci.yml's `gpu-kernel` job saves no rust-gpu build (R-320, R-337)");
    (gk, key)
}

fn check_restored_and_saved(jobs: &[Job]) {
    let found = kernel_jobs(jobs);
    assert!(
        !found.is_empty(),
        "no job builds the kernel: the parser read no run step"
    );
    for (job, b) in found {
        let at = format!("{} job `{}`", job.workflow, job.id);
        let build_if = job.steps[b].get("if");
        let steps: Vec<(usize, &Step)> = job
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| caches_rust_gpu(s))
            .collect();
        for (_, s) in &steps {
            assert!(
                cache_kind(s) != Some("both"),
                "{at}: its {RUST_GPU} cache step restores and saves at once, so it saves in every run, a pull-request \
                 run too (R-326)"
            );
        }
        let restore = steps
            .iter()
            .find(|(i, s)| *i < b && cache_kind(s) == Some("restore"))
            .map(|(_, s)| *s);
        let Some(restore) = restore else {
            panic!("{at}: builds the kernel without restoring {RUST_GPU} before it (R-320)");
        };
        let cond = restore.get("if");
        assert!(
            cond.is_none() || cond == build_if,
            "{at}: its {RUST_GPU} restore step runs under `if: {}`, which the build does not ({build_if:?})",
            cond.unwrap_or("")
        );
        if !job.on_main {
            assert!(
                steps.iter().all(|(_, s)| cache_kind(s) == Some("restore")),
                "{at}: saves {RUST_GPU}, though {} never runs on a push to `main`; it restores `gpu-kernel`'s key \
                 read-only and saves none (R-337)",
                job.workflow
            );
            let (gk, gk_key) = gpu_kernel_key(jobs);
            assert_eq!(
                resolve(job, restore.input("key").unwrap_or("")),
                resolve(gk, gk_key),
                "{at}: restores {RUST_GPU} under a key that is not the one `ci.yml`'s `gpu-kernel` job saves on `main` \
                 (R-337)"
            );
            continue;
        }
        let saves: Vec<&Step> = steps
            .iter()
            .filter(|(i, s)| *i > b && cache_kind(s) == Some("save"))
            .map(|(_, s)| *s)
            .collect();
        assert!(
            !saves.is_empty(),
            "{at}: builds the kernel without saving {RUST_GPU} after it (R-320)"
        );
        let mut want = terms(build_if);
        want.extend([ON_PUSH.to_owned(), ON_MAIN.to_owned()]);
        want.sort();
        for save in saves {
            assert_eq!(
                terms(save.get("if")),
                want,
                "{at}: its {RUST_GPU} save step's `if:` is not the build's condition and a push to `main` (R-326)"
            );
            assert_eq!(
                save.input("key"),
                restore.input("key"),
                "{at}: its {RUST_GPU} save step saves under another key than the one its restore reads"
            );
        }
    }
}

#[test]
fn qa_m014_kernel_jobs_restore_and_save_the_rust_gpu_build() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_kernel_jobs_restore_and_save_the_rust_gpu_build,
    "mutants.yml with its rust-gpu restore step caching another directory",
    expected = "job `mutants`: builds the kernel without restoring ~/.cache/rust-gpu",
    check_restored_and_saved(&all_jobs(&edited(
        "mutants.yml",
        "          path: ~/.cache/rust-gpu\n",
        "          path: ~/.cache/other\n"
    )))
);

#[test]
fn qa_m014_kernel_jobs_save_the_rust_gpu_build() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_kernel_jobs_save_the_rust_gpu_build,
    "ci.yml with the gpu-kernel job's rust-gpu save step made restore-only",
    expected = "job `gpu-kernel`: builds the kernel without saving ~/.cache/rust-gpu",
    check_restored_and_saved(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-",
        "      - uses: actions/cache/restore@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-"
    )))
);

#[test]
fn qa_m014_the_rust_gpu_cache_runs_whenever_the_build_does() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_the_rust_gpu_cache_runs_whenever_the_build_does,
    "ci.yml with the gpu-kernel job's rust-gpu restore restricted to pushes, though the kernel is built on every run",
    expected = "which the build does not",
    check_restored_and_saved(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/restore@v4\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-",
        "      - uses: actions/cache/restore@v4\n        if: github.event_name == 'push'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-"
    )))
);

#[test]
fn qa_m014_the_rust_gpu_cache_saves_only_on_a_push_to_main() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_the_rust_gpu_cache_saves_only_on_a_push_to_main,
    "ci.yml with xtask-ci's rust-gpu save step unconditioned, so a pull-request run saves it",
    expected = "job `xtask-ci`: its ~/.cache/rust-gpu save step's `if:` is not the build's condition and a push to `main`",
    check_restored_and_saved(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-",
        "      - uses: actions/cache/save@v4\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-"
    )))
);

#[test]
fn qa_m014_the_rust_gpu_save_is_main_not_any_branch() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_the_rust_gpu_save_is_main_not_any_branch,
    "ci.yml with gpu-kernel's rust-gpu save on a push to any branch or on main by any event (`||`)",
    expected = "job `gpu-kernel`: its ~/.cache/rust-gpu save step's `if:` is not the build's condition and a push to `main`",
    check_restored_and_saved(&all_jobs(&edited(
        "ci.yml",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-",
        "      - uses: actions/cache/save@v4\n        if: github.event_name == 'push' || github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-"
    )))
);

#[test]
fn qa_m014_the_rust_gpu_save_keeps_the_builds_condition() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_the_rust_gpu_save_keeps_the_builds_condition,
    "ci.yml with gpu-kernel's build made conditional and its rust-gpu save not, so it saves when no kernel was built",
    expected = "job `gpu-kernel`: its ~/.cache/rust-gpu save step's `if:` is not the build's condition and a push to `main`",
    check_restored_and_saved(&all_jobs(&edited(
        "ci.yml",
        "      - name: cargo xtask build-kernel\n        run: cargo xtask build-kernel\n",
        "      - name: cargo xtask build-kernel\n        if: hashFiles('crates/kernel/**') != ''\n        run: cargo xtask build-kernel\n"
    )))
);

#[test]
fn qa_m014_no_rust_gpu_step_restores_and_saves_at_once() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_no_rust_gpu_step_restores_and_saves_at_once,
    "mutants.yml with its rust-gpu restore turned back into `actions/cache@v4`, which saves in every run",
    expected = "job `mutants`: its ~/.cache/rust-gpu cache step restores and saves at once",
    check_restored_and_saved(&all_jobs(&edited(
        "mutants.yml",
        "      - uses: actions/cache/restore@v4\n        if: steps.diff.outputs.rust == 'true'\n        with:\n          path: ~/.cache/rust-gpu",
        "      - uses: actions/cache@v4\n        if: steps.diff.outputs.rust == 'true'\n        with:\n          path: ~/.cache/rust-gpu"
    )))
);

#[test]
fn qa_m014_the_rust_gpu_save_key_is_the_restored_key() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_the_rust_gpu_save_key_is_the_restored_key,
    "ci.yml with xtask-ci saving the rust-gpu build under a key its restore never reads",
    expected = "job `xtask-ci`: its ~/.cache/rust-gpu save step saves under another key",
    check_restored_and_saved(&all_jobs(&edited(
        "ci.yml",
        "github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-${{ runner.os }}-",
        "github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-xtask-ci-${{ runner.os }}-v2-"
    )))
);

// ---- 2. Its key names the job and the pinned toolchain; no fallback crosses toolchains ------------------------------

fn check_keys(jobs: &[Job]) {
    let channel = pinned_channel();
    let mut seen: Vec<(String, String)> = Vec::new();
    for (job, _) in kernel_jobs(jobs) {
        let at = format!("{} job `{}`", job.workflow, job.id);
        for s in job.steps.iter().filter(|s| caches_rust_gpu(s)) {
            let key = s.input("key").unwrap_or("");
            let literal = key.split("${{").next().unwrap_or("").trim_end_matches('-');
            if job.on_main {
                assert!(
                    names_job(literal, &job.id),
                    "{at}: rust-gpu cache key {key:?} does not name its job"
                );
                // A job's restore and save steps share their key (R-326); another saving job's never does.
                if let Some((other, _)) = seen.iter().find(|(o, k)| k == key && *o != at) {
                    panic!(
                        "{at}: rust-gpu cache key {key:?} is shared with {other}, so it does not name its job"
                    );
                }
                seen.push((at.clone(), key.to_owned()));
            } else {
                // A workflow that never runs on a push to `main` restores the key the saving job names (R-337).
                assert!(
                    names_job(literal, "gpu-kernel"),
                    "{at}: rust-gpu cache key {key:?} does not name `ci.yml`'s `gpu-kernel` job, whose key it restores \
                     (R-337)"
                );
            }
            let resolved = resolve(job, key);
            assert!(
                resolved.contains(&channel),
                "{at}: rust-gpu cache key {key:?} (resolved {resolved:?}) does not name the pinned toolchain {channel}"
            );
            for rk in s
                .input("restore-keys")
                .unwrap_or("")
                .lines()
                .filter(|l| !l.trim().is_empty())
            {
                let r = resolve(job, rk.trim());
                assert!(
                    r.contains(&channel),
                    "{at}: rust-gpu restore key {rk:?} (resolved {r:?}) falls back across toolchains: it does not name {channel}"
                );
            }
        }
    }
    assert!(!seen.is_empty(), "no rust-gpu cache key found");
}

#[test]
fn qa_m014_rust_gpu_keys_name_the_pinned_toolchain() {
    check_keys(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_rust_gpu_keys_name_the_pinned_toolchain,
    "ci.yml with the xtask-ci job's rust-gpu key on the rustc hash output, not the channel",
    expected = "does not name the pinned toolchain",
    check_keys(&all_jobs(&edited(
        "ci.yml",
        "key: rust-gpu-xtask-ci-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}",
        "key: rust-gpu-xtask-ci-${{ runner.os }}-${{ steps.toolchain.outputs.cachekey }}"
    )))
);

#[test]
fn qa_m014_the_channel_output_is_rust_toolchain_tomls() {
    check_keys(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_the_channel_output_is_rust_toolchain_tomls,
    "mutants.yml computing its channel output from a file other than rust-toolchain.toml",
    expected = "job `mutants`: rust-gpu cache key",
    check_keys(&all_jobs(&edited(
        "mutants.yml",
        "\\(.*\\)\"$/\\1/p' rust-toolchain.toml)",
        "\\(.*\\)\"$/\\1/p' Cargo.toml)"
    )))
);

#[test]
fn qa_m014_no_rust_gpu_restore_key_crosses_toolchains() {
    check_keys(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_no_rust_gpu_restore_key_crosses_toolchains,
    "ci.yml with a restore key that falls back to any channel's rust-gpu build",
    expected = "falls back across toolchains",
    check_keys(&all_jobs(&edited(
        "ci.yml",
        "          key: rust-gpu-gpu-kernel-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}\n",
        "          key: rust-gpu-gpu-kernel-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}\n          restore-keys: rust-gpu-gpu-kernel-${{ runner.os }}-\n"
    )))
);

#[test]
fn qa_m014_rust_gpu_keys_name_their_job() {
    check_keys(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_rust_gpu_keys_name_their_job,
    "ci.yml with the xtask-ci job's rust-gpu key made the ci job's",
    expected = "job `xtask-ci`: rust-gpu cache key \"rust-gpu-ci-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}\" does not name its job",
    check_keys(&all_jobs(&edited(
        "ci.yml",
        "key: rust-gpu-xtask-ci-",
        "key: rust-gpu-ci-"
    )))
);

#[test]
fn qa_m014_no_two_jobs_share_a_rust_gpu_key() {
    check_keys(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_no_two_jobs_share_a_rust_gpu_key,
    "ci.yml with gpu-kernel's and xtask-ci's rust-gpu keys both made `rust-gpu-gpu-kernel-xtask-ci-…`, which names each \
     job but is shared",
    expected = "job `xtask-ci`: rust-gpu cache key \"rust-gpu-gpu-kernel-xtask-ci-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}\" is shared with ci.yml job `gpu-kernel`",
    check_keys(&all_jobs(&edited_all(
        "ci.yml",
        &[
            (
                "key: rust-gpu-xtask-ci-",
                "key: rust-gpu-gpu-kernel-xtask-ci-"
            ),
            (
                "key: rust-gpu-gpu-kernel-$",
                "key: rust-gpu-gpu-kernel-xtask-ci-$"
            ),
        ]
    )))
);

// ---- 3. The parser reads what the checks rely on -------------------------------------------------------------------

/// Guards the checks against a parser that reads nothing: a job running `cargo xtask ci` (R-235) is found among the
/// kernel builders, and its rust-gpu cache step with its path and key.
fn check_parser(jobs: &[Job]) {
    let found = kernel_jobs(jobs);
    let xci = found
        .iter()
        .find(|(j, b)| {
            j.steps[*b]
                .get("run")
                .is_some_and(|r| r.contains("cargo xtask ci"))
        })
        .expect("the parser found no job running `cargo xtask ci` among the kernel builders");
    assert!(
        xci.0
            .steps
            .iter()
            .any(|s| caches_rust_gpu(s) && s.input("key").is_some()),
        "the parser read no rust-gpu cache step with a key in {} job `{}`",
        xci.0.workflow,
        xci.0.id
    );
}

#[test]
fn qa_m014_parser_reads_the_kernel_jobs() {
    check_parser(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_parser_reads_the_kernel_jobs,
    "ci.yml with `cargo xtask ci` renamed, so no job runs it",
    expected = "found no job running `cargo xtask ci`",
    check_parser(&all_jobs(&edited(
        "ci.yml",
        "        run: cargo xtask ci\n",
        "        run: cargo xtask controls\n"
    )))
);

// ---- 4. A workflow that never runs on a push to `main` restores `gpu-kernel`'s key and saves none (R-337) ----------

#[test]
fn qa_m014_a_pr_only_kernel_job_saves_no_rust_gpu_build() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_a_pr_only_kernel_job_saves_no_rust_gpu_build,
    "mutants.yml with a rust-gpu save step after its build again, under `gpu-kernel`'s key and on a push to `main`",
    expected = "job `mutants`: saves ~/.cache/rust-gpu, though mutants.yml never runs on a push to `main`",
    check_restored_and_saved(&all_jobs(&edited(
        "mutants.yml",
        "        run: cargo run --quiet --package xtask -- build-kernel\n",
        "        run: cargo run --quiet --package xtask -- build-kernel\n      - uses: actions/cache/save@v4\n        if: steps.diff.outputs.rust == 'true' && github.event_name == 'push' && github.ref == 'refs/heads/main'\n        with:\n          path: ~/.cache/rust-gpu\n          key: rust-gpu-gpu-kernel-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}\n"
    )))
);

#[test]
fn qa_m014_a_pr_only_kernel_job_restores_gpu_kernels_key() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_a_pr_only_kernel_job_restores_gpu_kernels_key,
    "mutants.yml restoring the rust-gpu build under its own pre-R-337 key, which nothing saves",
    expected = "job `mutants`: restores ~/.cache/rust-gpu under a key that is not the one `ci.yml`'s `gpu-kernel` job saves",
    check_restored_and_saved(&all_jobs(&edited(
        "mutants.yml",
        "key: rust-gpu-gpu-kernel-",
        "key: rust-gpu-mutants-"
    )))
);

#[test]
fn qa_m014_a_pr_only_rust_gpu_key_resolves_as_gpu_kernels() {
    check_restored_and_saved(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_a_pr_only_rust_gpu_key_resolves_as_gpu_kernels,
    "mutants.yml's rust-gpu key naming `gpu-kernel` and the channel, but with a word gpu-kernel's key lacks",
    expected = "job `mutants`: restores ~/.cache/rust-gpu under a key that is not the one `ci.yml`'s `gpu-kernel` job saves",
    check_restored_and_saved(&all_jobs(&edited(
        "mutants.yml",
        "key: rust-gpu-gpu-kernel-${{ runner.os }}-",
        "key: rust-gpu-gpu-kernel-${{ runner.os }}-v2-"
    )))
);

#[test]
fn qa_m014_a_pr_only_rust_gpu_key_names_gpu_kernel() {
    check_keys(&all_jobs(&workflows()));
}

negative_control!(
    qa_m014_a_pr_only_rust_gpu_key_names_gpu_kernel,
    "mutants.yml restoring the rust-gpu build under its own pre-R-337 key",
    expected = "job `mutants`: rust-gpu cache key \"rust-gpu-mutants-${{ runner.os }}-${{ steps.toolchain.outputs.channel }}\" does not name `ci.yml`'s `gpu-kernel` job",
    check_keys(&all_jobs(&edited(
        "mutants.yml",
        "key: rust-gpu-gpu-kernel-",
        "key: rust-gpu-mutants-"
    )))
);

/// The workflows R-337 names never run on a push to `main`, and `ci.yml`, which saves, does: the parser reads `on:`.
fn check_pr_only(files: &[(String, String)]) {
    for (f, t) in files {
        let pr_only = [
            "mutants.yml",
            "pr-check.yml",
            "reviews.yml",
            "screenshot.yml",
            "stand-in-soak.yml",
        ]
        .contains(&f.as_str());
        if pr_only {
            assert!(
                !runs_on_push_to_main(t),
                "{f} runs on a push to `main`, though R-337 names it as never doing so"
            );
        } else if f == "ci.yml" {
            assert!(
                runs_on_push_to_main(t),
                "ci.yml does not run on a push to `main`, where its jobs save the caches (R-326)"
            );
        }
    }
}

#[test]
fn qa_m014_the_parser_reads_which_workflows_run_on_main() {
    check_pr_only(&workflows());
}

negative_control!(
    qa_m014_the_parser_reads_which_workflows_run_on_main,
    "ci.yml with its push trigger limited to branches other than `main`",
    expected = "ci.yml does not run on a push to `main`",
    check_pr_only(&edited(
        "ci.yml",
        "on:\n  push:\n",
        "on:\n  push:\n    branches-ignore:\n      - main\n"
    ))
);

#[test]
fn qa_m014_the_parser_reads_a_push_trigger_on_a_pr_only_workflow() {
    check_pr_only(&workflows());
}

negative_control!(
    qa_m014_the_parser_reads_a_push_trigger_on_a_pr_only_workflow,
    "mutants.yml with a push trigger on `main` added",
    expected = "mutants.yml runs on a push to `main`",
    check_pr_only(&edited(
        "mutants.yml",
        "on:\n  pull_request:\n",
        "on:\n  push:\n    branches: [main]\n  pull_request:\n"
    ))
);
