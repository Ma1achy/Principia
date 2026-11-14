//! QA's tests for TASK-M0-64 (REQ-TOOL-235), written from the requirement, not from the implementation:
//!
//! > win-self-check's re-proof of the kept-checkout guard must plant a whole allowed config as actions/checkout leaves
//! > it, the three sparse-checkout keys in .git/config.worktree and the other keys in .git/config, so the guard passes
//! > its own list and still refuses every planted case, the guard script unchanged (R-711, R-860).
//!
//! Off Windows there is no PowerShell, so the guard can't run here. These tests instead check the requirement's two
//! premises with real git and a model of the guard's key rule, as `.github/scripts/git_guard.ps1` states it: every key
//! of `.git/config` and `.git/config.worktree`, read by `git config --no-includes --file … --list --null`, must be on
//! `$allowed` with its value, and no key may be held twice across the two files.
//!
//! 1. A checkout made by actions/checkout's own commands holds exactly the three sparse-checkout keys in
//!    `.git/config.worktree`, and none of them in `.git/config`.
//! 2. The whole of `$allowed` planted that way over such a checkout, both files replaced, passes the rule; each wrong
//!    value planted the same way is refused for its value alone, never for a repeated key.
//!
//! Each has its negative control (R-176): the planting the PC's nightly failed on, the whole list in `.git/config` over
//! the checkout's own `config.worktree`, is refused for a repeated key; and a checkout made without
//! `sparse-checkout disable` fails premise 1.

use std::path::{Path, PathBuf};

use validation::negative_control;

const GUARD: &str = ".github/scripts/git_guard.ps1";
/// The requirement's three sparse-checkout keys, as git prints them.
const SPARSE: [&str; 3] = [
    "core.sparsecheckout",
    "core.sparsecheckoutcone",
    "index.sparse",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// `$allowed`'s quoted `key=value` pairs, read from the script.
fn allowed() -> Vec<String> {
    let text = std::fs::read_to_string(root().join(GUARD)).expect("the guard script");
    let start = text
        .find("\n$allowed = @(")
        .expect("the guard has $allowed");
    let body = &text[start..];
    let end = body.find("\n)").expect("$allowed ends");
    let pairs: Vec<String> = body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('\''))
        .map(|line| line.trim_matches('\'').to_owned())
        .collect();
    assert!(pairs.len() >= 10, "qa: $allowed read short: {pairs:?}");
    pairs
}

fn key(pair: &str) -> &str {
    pair.split_once('=').expect("a key=value pair").0
}

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_git_guard_split_{name}_{}_{}",
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

fn git_ok(dir: &Path, args: &[&str]) -> String {
    let out = git(dir, args);
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A checkout made as actions/checkout makes one on a first run (`init`, `remote add`, `fetch --depth=1`,
/// `sparse-checkout disable`, `config --unset-all extensions.worktreeConfig`, `checkout --force -B main`), the
/// `sparse-checkout disable` step left out when `disable` is false.
fn checkout(base: &Path, disable: bool) -> PathBuf {
    let origin = base.join("origin");
    std::fs::create_dir_all(&origin).expect("a folder");
    git_ok(&origin, &["init", "-q"]);
    std::fs::write(origin.join("a.txt"), "a").expect("a file");
    git_ok(&origin, &["add", "."]);
    git_ok(
        &origin,
        &[
            "-c",
            "user.name=qa",
            "-c",
            "user.email=qa@localhost",
            "commit",
            "-q",
            "-m",
            "a",
        ],
    );
    let sha = git_ok(&origin, &["rev-parse", "HEAD"]);
    let url = origin.to_string_lossy().replace('\\', "/");
    let url = format!("file:///{}", url.trim_start_matches('/'));
    let dir = base.join("kept");
    std::fs::create_dir_all(&dir).expect("a folder");
    git_ok(&dir, &["init", "-q"]);
    git_ok(&dir, &["remote", "add", "origin", &url]);
    git_ok(&dir, &["config", "--local", "gc.auto", "0"]);
    git_ok(
        &dir,
        &[
            "fetch",
            "-q",
            "--no-tags",
            "--depth=1",
            "origin",
            &format!("+{sha}:refs/remotes/origin/main"),
        ],
    );
    if disable {
        git_ok(&dir, &["sparse-checkout", "disable"]);
    }
    let _ = git(
        &dir,
        &[
            "config",
            "--local",
            "--unset-all",
            "extensions.worktreeConfig",
        ],
    );
    git_ok(
        &dir,
        &[
            "checkout",
            "-q",
            "--force",
            "-B",
            "main",
            "refs/remotes/origin/main",
        ],
    );
    dir
}

/// `file`'s `key=value` pairs as the guard reads them; nothing if it isn't there.
fn pairs(file: &Path) -> Vec<String> {
    if !file.exists() {
        return Vec::new();
    }
    let parent = file.parent().expect("a parent");
    let out = git(
        parent,
        &[
            "config",
            "--no-includes",
            "--file",
            file.to_str().expect("a path"),
            "--list",
            "--null",
        ],
    );
    assert!(out.status.success(), "git reads {file:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.replacen('\n', "=", 1))
        .collect()
}

/// `pairs` as a config file's text, written in git's own sections.
fn text(pairs: &[String]) -> String {
    let mut out = String::new();
    for pair in pairs {
        let (key, value) = pair.split_once('=').expect("a pair");
        let (section, rest) = key.split_once('.').expect("a section");
        let header = match rest.rsplit_once('.') {
            Some((sub, _)) => format!("[{section} \"{sub}\"]"),
            None => format!("[{section}]"),
        };
        let name = rest.rsplit('.').next().expect("a name");
        out.push_str(&format!("{header}\n\t{name} = {value}\n"));
    }
    out
}

/// The guard's rule on a checkout's two config files: each key on `$allowed` with its value, and none held twice. What
/// it finds, in the guard's own words.
fn guard_findings(dir: &Path, allowed: &[String]) -> Vec<String> {
    let mut held: Vec<String> = Vec::new();
    let mut found = Vec::new();
    for file in ["config", "config.worktree"] {
        for pair in pairs(&dir.join(".git").join(file)) {
            let k = key(&pair).to_owned();
            if !allowed.iter().any(|a| key(a) == k) {
                found.push(format!("config key off the allowlist: {k}"));
            } else if !allowed.contains(&pair) {
                found.push(format!("config value off the allowlist: {pair}"));
            }
            held.push(k);
        }
    }
    let mut sorted = held.clone();
    sorted.sort();
    sorted.dedup();
    for k in sorted {
        if held.iter().filter(|h| h.eq_ignore_ascii_case(&k)).count() > 1 {
            found.push(format!("repeated config key: {k}"));
        }
    }
    found
}

/// The whole list, with `wrong` in place of its key's value, as the requirement says to plant it: the sparse keys in
/// `.git/config.worktree`, the rest in `.git/config`, both written over the checkout's own.
fn plant_split(dir: &Path, allowed: &[String], wrong: Option<&str>) {
    let pairs: Vec<String> = allowed
        .iter()
        .map(|pair| match wrong {
            Some(w) if key(w) == key(pair) => w.to_owned(),
            _ => pair.clone(),
        })
        .collect();
    let (worktree, config): (Vec<String>, Vec<String>) =
        pairs.into_iter().partition(|p| SPARSE.contains(&key(p)));
    std::fs::write(dir.join(".git/config"), text(&config)).expect("a planted config");
    std::fs::write(dir.join(".git/config.worktree"), text(&worktree)).expect("a planted config");
}

/// The planting the PC's nightly failed on: the whole list in `.git/config`, the checkout's `config.worktree` kept.
fn plant_whole_in_config(dir: &Path, allowed: &[String]) {
    std::fs::write(dir.join(".git/config"), text(allowed)).expect("a planted config");
}

fn check_clean_checkout_split(disable: bool) {
    let allowed = allowed();
    for k in SPARSE {
        assert!(
            allowed.iter().any(|pair| key(pair) == k),
            "qa: {k} isn't on $allowed"
        );
    }
    let dir = checkout(&scratch("clean"), disable);
    let mut worktree: Vec<String> = pairs(&dir.join(".git/config.worktree"))
        .iter()
        .map(|p| key(p).to_owned())
        .collect();
    worktree.sort();
    assert_eq!(
        worktree, SPARSE,
        "qa: a clean checkout's config.worktree doesn't hold exactly the sparse keys"
    );
    for pair in pairs(&dir.join(".git/config")) {
        assert!(
            !SPARSE.contains(&key(&pair)),
            "qa: a clean checkout's .git/config holds {pair}"
        );
    }
    // What's there is on the list, each value as the list has it.
    for pair in pairs(&dir.join(".git/config.worktree")) {
        assert!(
            allowed.contains(&pair),
            "qa: config.worktree's {pair} isn't on $allowed"
        );
    }
}

#[test]
fn qa_clean_checkout_holds_the_sparse_keys_in_config_worktree() {
    check_clean_checkout_split(true);
}

negative_control!(
    qa_clean_checkout_holds_the_sparse_keys_in_config_worktree,
    "a checkout made without sparse-checkout disable",
    expected = "qa: a clean checkout's config.worktree doesn't hold exactly the sparse keys",
    check_clean_checkout_split(false)
);

/// The whole list planted per the requirement passes the guard's key rule; each wrong value planted the same way is
/// refused for its value, and nothing is held twice.
fn check_split_planting(split: bool) {
    let allowed = allowed();
    let dir = checkout(&scratch("plant"), true);
    if split {
        plant_split(&dir, &allowed, None);
    } else {
        plant_whole_in_config(&dir, &allowed);
    }
    let found = guard_findings(&dir, &allowed);
    assert!(
        found.is_empty(),
        "qa: the guard's rule refuses its own list: {found:?}"
    );
    let mut all = pairs(&dir.join(".git/config"));
    all.extend(pairs(&dir.join(".git/config.worktree")));
    all.sort();
    let mut want = allowed.clone();
    want.sort();
    assert_eq!(
        all, want,
        "qa: the planted files aren't the list, each pair once"
    );
    for k in SPARSE {
        let wrong = format!("{k}=true");
        assert!(!allowed.contains(&wrong), "qa: {wrong} is on $allowed");
        let dir = checkout(&scratch("wrong"), true);
        plant_split(&dir, &allowed, Some(&wrong));
        assert_eq!(
            guard_findings(&dir, &allowed),
            vec![format!("config value off the allowlist: {wrong}")],
            "qa: a wrong sparse value isn't refused for its value alone"
        );
    }
    let wrong = "gc.auto=1";
    let dir = checkout(&scratch("wrong"), true);
    plant_split(&dir, &allowed, Some(wrong));
    assert_eq!(
        guard_findings(&dir, &allowed),
        vec![format!("config value off the allowlist: {wrong}")],
        "qa: a wrong value in .git/config isn't refused for its value alone"
    );
}

#[test]
fn qa_whole_list_planted_split_passes_the_guards_rule() {
    check_split_planting(true);
}

negative_control!(
    qa_whole_list_planted_split_passes_the_guards_rule,
    "the whole list in .git/config over the checkout's own config.worktree",
    expected = "qa: the guard's rule refuses its own list",
    check_split_planting(false)
);
