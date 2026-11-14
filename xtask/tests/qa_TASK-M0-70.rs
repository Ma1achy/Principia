//! QA's tests for TASK-M0-70 (REQ-TOOL-243), written from the requirement, not from the implementation:
//!
//! > win-self-check's firing of the kept-checkout guard must plant a .git holding every file the guard script reads in
//! > every .git by name, as a clean checkout holds it (.git/config and a HEAD the script accepts), and one planted hook,
//! > so the guard's logged reason is the hook's, "active hook", and not "can't read the checkout"; the firing's
//! > assertion on the log prints the log and the script's output when it fails; the guard script unchanged, no
//! > assertion of the test removed or weakened (R-711, R-860).
//!
//! Off Windows there is no PowerShell, so the guard can't run here. These tests read the firing as
//! `xtask/tests/ci_self_hosted.rs` plants it, plant it on disk, and run over it a model of `Find-Planted` in
//! `.github/scripts/git_guard.ps1`: a file it reads with no guard (`ReadAllText` or `Get-Item -LiteralPath` on a
//! `Join-Path $git.FullName '<name>'`) that is missing throws, which the script's outer `catch` turns into "can't read
//! the checkout"; otherwise each optional entry it looks at, the HEAD pattern and each config key, read with real git as
//! the script reads it, against `$allowed` and `$originUrl`. The firing's reasons must be the hook's alone.
//!
//! 1. A clean checkout's HEAD, as real `git init` on `main` writes it, is the firing's HEAD, and the script accepts it.
//! 2. The firing, planted, gives exactly one reason, "active hook: pre-commit".
//! 3. The firing's log check in `check_script_refuses` still asserts "active hook" and prints the log and the script's
//!    output.
//!
//! Negative controls (R-176): the firing with no HEAD (as it was) reads as "can't read the checkout"; a HEAD the script
//! refuses gives a HEAD reason beside the hook's; a config key off the allowlist gives a second reason; a hook named
//! `*.sample` gives no hook reason; a log check without the script's output fails 3.

use std::path::{Path, PathBuf};

use validation::negative_control;

const GUARD: &str = ".github/scripts/git_guard.ps1";
const TEST: &str = "xtask/tests/ci_self_hosted.rs";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m0_70_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    dir
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.join("no_such_config"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(args)
        .output()
        .expect("git ran")
}

/// A Rust string literal's body, its escapes undone (`\n`, `\t`, `\"`, `\\`).
fn unescape(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            other => panic!("qa: an escape this reader doesn't know: {other:?}"),
        }
    }
    out
}

/// The body of `const <name>` as a string literal, from the test's source.
fn const_str(source: &str, name: &str) -> String {
    let start = source
        .find(&format!("const {name}: &str = \""))
        .unwrap_or_else(|| panic!("qa: no const {name}"));
    let rest = &source[start..];
    let open = rest.find('"').expect("a quote") + 1;
    let line = &rest[open..rest.find('\n').expect("a line end")];
    let body = line.strip_suffix("\";").expect("a literal on one line");
    unescape(body)
}

/// The firing, as `ci_self_hosted.rs` defines it: each (path, text), a text being a literal or a `const` of the file.
fn firing(source: &str) -> Vec<(String, String)> {
    let start = source
        .find("const FIRING")
        .expect("qa: the test defines FIRING");
    let rest = &source[start..];
    let body = &rest[..rest.find("];").expect("FIRING ends")];
    let entry = regex::Regex::new(r#"\(\s*"([^"]+)"\s*,\s*("((?:[^"\\]|\\.)*)"|[A-Z_]+)\s*\)"#)
        .expect("a pattern");
    let found: Vec<(String, String)> = entry
        .captures_iter(body)
        .map(|c| {
            let text = match c.get(3) {
                Some(literal) => unescape(literal.as_str()),
                None => const_str(source, &c[2]),
            };
            (c[1].to_owned(), text)
        })
        .collect();
    assert!(!found.is_empty(), "qa: FIRING read empty");
    found
}

fn plant(entries: &[(String, String)]) -> PathBuf {
    let dir = scratch("firing");
    for (name, text) in entries {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
        std::fs::write(path, text).expect("a file");
    }
    dir
}

/// The body of `Find-Planted`, from the script.
fn find_planted_body(script: &str) -> &str {
    let start = script
        .find("function Find-Planted($git)")
        .expect("qa: the script has Find-Planted");
    let rest = &script[start..];
    &rest[..rest.find("\n}\n").expect("Find-Planted ends")]
}

/// `$allowed`'s `key=value` pairs and `$originUrl`, from the script.
fn allowed(script: &str) -> (Vec<String>, String) {
    let start = script
        .find("\n$allowed = @(")
        .expect("the guard has $allowed");
    let body = &script[start..];
    let body = &body[..body.find("\n)").expect("$allowed ends")];
    let pairs: Vec<String> = body
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('\''))
        .map(|line| line.trim_matches('\'').to_owned())
        .collect();
    assert!(pairs.len() >= 9, "qa: $allowed read short: {pairs:?}");
    let url = script
        .split_once("\n$originUrl = '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .expect("the guard has $originUrl")
        .0
        .to_owned();
    (pairs, url)
}

/// A model of `Find-Planted` over the checkout `dir`, as the script's outer `catch` leaves its reasons.
fn model(script: &str, dir: &Path) -> Vec<String> {
    let body = find_planted_body(script);
    let git_dir = dir.join(".git");
    // Each unguarded read in Find-Planted: a missing file throws, and the outer catch replaces every reason.
    let unguarded = regex::Regex::new(
        r"(?:ReadAllText\(\(|Get-Item -LiteralPath \()Join-Path \$git\.FullName '([^']+)'\)",
    )
    .expect("a pattern");
    let reads: Vec<String> = unguarded
        .captures_iter(body)
        .map(|c| c[1].to_owned())
        .collect();
    assert!(
        reads.iter().any(|r| r == "HEAD") && reads.iter().any(|r| r == "config"),
        "qa: the model found the script's unguarded reads as {reads:?}"
    );
    for name in &reads {
        if !git_dir.join(name).is_file() {
            return vec![format!("can't read the checkout: no .git/{name}")];
        }
    }
    let mut found = Vec::new();
    for redirect in ["commondir", "gitdir"] {
        if git_dir.join(redirect).exists() {
            found.push(format!("redirect: {redirect}"));
        }
    }
    for (path, reason) in [
        ("reftable", ".git/reftable"),
        ("info/attributes", "info/attributes"),
        ("info/grafts", "info/grafts"),
        ("objects/info/alternates", "objects/info/alternates"),
    ] {
        if git_dir.join(path).exists() {
            found.push(reason.to_owned());
        }
    }
    if let Ok(hooks) = std::fs::read_dir(git_dir.join("hooks")) {
        let mut names: Vec<String> = hooks
            .map(|e| {
                e.expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|n| !n.ends_with(".sample"))
            .collect();
        names.sort();
        found.extend(names.into_iter().map(|n| format!("active hook: {n}")));
    }
    if std::fs::read_dir(git_dir.join("refs/replace")).is_ok_and(|mut d| d.next().is_some()) {
        found.push("refs/replace/".to_owned());
    }
    if std::fs::read_to_string(git_dir.join("packed-refs"))
        .is_ok_and(|t| t.to_ascii_lowercase().contains("refs/replace/"))
    {
        found.push("refs/replace/ in packed-refs".to_owned());
    }
    let pattern = body
        .split_once("$head -cnotmatch '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .expect("qa: the script's HEAD pattern")
        .0;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).expect("HEAD");
    if !regex::Regex::new(pattern)
        .expect("the pattern")
        .is_match(&head)
    {
        found.push(format!("HEAD: {head}"));
    }
    let (allowed, origin) = allowed(script);
    let allowed_keys: Vec<&str> = allowed
        .iter()
        .map(|p| p.split_once('=').expect("key=value").0)
        .collect();
    let mut configs = vec![git_dir.join("config")];
    if git_dir.join("config.worktree").exists() {
        configs.push(git_dir.join("config.worktree"));
    }
    let mut held: Vec<String> = Vec::new();
    let mut pairs: Vec<String> = Vec::new();
    for config in configs {
        let out = git(
            dir,
            &[
                "config",
                "--no-includes",
                "--file",
                &config.to_string_lossy(),
                "--list",
                "--null",
            ],
        );
        if !out.status.success() {
            return vec![format!("can't read the checkout: {out:?}")];
        }
        for entry in String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|e| !e.is_empty())
        {
            let (key, value) = entry.split_once('\n').unwrap_or((entry, ""));
            held.push(key.to_owned());
            pairs.push(format!("{key}={value}"));
            if !allowed_keys.contains(&key) {
                found.push(format!("config key off the allowlist: {key}"));
            } else if !allowed.contains(&format!("{key}={value}")) {
                found.push(format!("config value off the allowlist: {key}={value}"));
            }
        }
    }
    let mut seen = Vec::new();
    for key in &held {
        if seen.contains(key) {
            found.push(format!("repeated config key: {key}"));
        }
        seen.push(key.clone());
    }
    if !pairs.contains(&format!("remote.origin.url={origin}")) {
        found.push("remote.origin.url isn't this repo".to_owned());
    }
    found
}

/// The firing's reasons, under the model, are the hook's alone.
fn check_reasons(entries: &[(String, String)]) {
    let dir = plant(entries);
    let reasons = model(&read(GUARD), &dir);
    assert_eq!(
        reasons,
        vec!["active hook: pre-commit".to_owned()],
        "qa: the firing's reasons are not the hook's alone"
    );
}

#[test]
fn qa_m0_70_firing_gives_the_hooks_reason_alone() {
    check_reasons(&firing(&read(TEST)));
}

#[cfg(feature = "controls")]
fn without(entries: Vec<(String, String)>, path: &str) -> Vec<(String, String)> {
    let kept: Vec<_> = entries.into_iter().filter(|(p, _)| p != path).collect();
    kept
}

#[cfg(feature = "controls")]
fn replaced(entries: Vec<(String, String)>, path: &str, text: &str) -> Vec<(String, String)> {
    let mut out = without(entries, path);
    out.push((path.to_owned(), text.to_owned()));
    out
}

negative_control!(
    qa_m0_70_firing_gives_the_hooks_reason_alone,
    "the firing with no HEAD, as it was before TASK-M0-70",
    expected = "qa: the firing's reasons are not the hook's alone",
    check_reasons(&without(firing(&read(TEST)), ".git/HEAD"))
);

mod head_refused {
    use super::*;
    negative_control!(
        qa_m0_70_firing_gives_the_hooks_reason_alone,
        "a HEAD the script refuses",
        expected = "qa: the firing's reasons are not the hook's alone",
        check_reasons(&replaced(
            firing(&read(TEST)),
            ".git/HEAD",
            "ref: refs/heads/evil\n"
        ))
    );
}

mod config_off_list {
    use super::*;
    negative_control!(
        qa_m0_70_firing_gives_the_hooks_reason_alone,
        "a config key off the allowlist beside the hook",
        expected = "qa: the firing's reasons are not the hook's alone",
        check_reasons(&{
            let entries = firing(&read(TEST));
            let config = entries
                .iter()
                .find(|(p, _)| p == ".git/config")
                .expect("a config")
                .1
                .clone();
            replaced(
                entries,
                ".git/config",
                &format!("{config}[core]\n\thooksPath = elsewhere\n"),
            )
        })
    );
}

mod sample_hook {
    use super::*;
    negative_control!(
        qa_m0_70_firing_gives_the_hooks_reason_alone,
        "a hook named *.sample, which the script doesn't count",
        expected = "qa: the firing's reasons are not the hook's alone",
        check_reasons(&{
            let entries = without(firing(&read(TEST)), ".git/hooks/pre-commit");
            replaced(entries, ".git/hooks/pre-commit.sample", "x")
        })
    );
}

/// A clean checkout's HEAD, as real git writes it on `main`, is the firing's HEAD and is one the script accepts.
fn check_clean_head(head: &str) {
    let dir = scratch("clean");
    let out = git(&dir, &["init", "-q", "--initial-branch=main"]);
    assert!(out.status.success(), "git init: {out:?}");
    let real = std::fs::read_to_string(dir.join(".git/HEAD")).expect("HEAD");
    assert_eq!(
        real, head,
        "qa: the firing's HEAD is not a clean checkout's"
    );
    let pattern = read(GUARD)
        .split_once("$head -cnotmatch '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .expect("the HEAD pattern")
        .0
        .to_owned();
    assert!(
        regex::Regex::new(&pattern)
            .expect("the pattern")
            .is_match(head),
        "qa: the script refuses the firing's HEAD"
    );
}

fn firing_head() -> String {
    firing(&read(TEST))
        .into_iter()
        .find(|(p, _)| p == ".git/HEAD")
        .map_or_else(String::new, |(_, t)| t)
}

#[test]
fn qa_m0_70_firing_head_is_a_clean_checkouts() {
    check_clean_head(&firing_head());
}

negative_control!(
    qa_m0_70_firing_head_is_a_clean_checkouts,
    "a HEAD on another branch",
    expected = "qa: the firing's HEAD is not a clean checkout's",
    check_clean_head("ref: refs/heads/other\n")
);

/// The firing's log check in `check_script_refuses` asserts "active hook", and its message prints the log and the
/// script's output.
fn check_log_assertion(source: &str) {
    let start = source
        .find("fn check_script_refuses(")
        .expect("qa: the test has check_script_refuses");
    let body = &source[start..];
    let at = body
        .find("line.contains(\"active hook\")")
        .expect("qa: the firing's log check no longer asserts \"active hook\"");
    let assertion = &body[at..at + body[at..].find(");").expect("the assertion ends")];
    assert!(
        assertion.contains("{line:?}") && assertion.contains("out.stdout"),
        "qa: the firing's log check prints neither the log nor the script's output: {assertion}"
    );
}

#[test]
fn qa_m0_70_log_check_prints_log_and_output() {
    check_log_assertion(&read(TEST));
}

negative_control!(
    qa_m0_70_log_check_prints_log_and_output,
    "the log check as it was, with no log or output printed",
    expected = "qa: the firing's log check prints neither the log nor the script's output",
    check_log_assertion(
        &read(TEST).replace(
            "\"git_guard: the firing logged no reason: the log held {line:?}, the script said {:?}\",\n        String::from_utf8_lossy(&out.stdout)",
            "\"git_guard: the firing logged no reason\""
        )
    )
);
