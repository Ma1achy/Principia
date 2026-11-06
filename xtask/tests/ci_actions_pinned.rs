//! The owner, 5 Nov 2026: "pin every `uses:` in .github/workflows and .github/actions to a full-length commit SHA, with
//! the version tag as a trailing comment (e.g. actions/checkout@<sha> # v4.x.y) […] Add a CI check that fails on any
//! unpinned third-party or GitHub action."
//!
//! - `ci_actions_pinned_every_uses`: every `uses:` of the repository names a local path (`./…`), or an action or a
//!   reusable workflow at a full 40-character commit SHA followed by its exact version tag as a comment
//!   (`actions/checkout@<sha> # v4.4.0`). A moving tag, a branch, a short SHA, a missing comment or a Docker image
//!   fails, in a step or at job level, and in a workflow or action nobody listed.
//! - `ci_actions_pinned_rule`: what the rule accepts and refuses, at each boundary of the reference and the comment.
//! - `ci_actions_pinned_reads_the_yaml`: the scan reads the YAML, not its lines: a quoted or flow-style `uses:` counts,
//!   and a comment or a `run:` block that merely says `uses:` does not.
//! - `ci_actions_pinned_finds_every_file`: the files are found by walking, never listed: every YAML file under
//!   `.github/`, whatever its folder, and every `action.yml` or `action.yaml` in the repository, the build directory
//!   and git's own apart.
//! - `ci_actions_pinned_a_workflow_change_reaches_this_check`: `scripts/ci_changes.sh` and
//!   `.github/scripts/ci_quick_crates.py` send a pull request that changes a workflow or an action to these tests, in
//!   the quick tier and the full one.
//!
//! The limit: no test here can reach GitHub, so nothing checks that the comment names the tag the SHA really has. The
//! PR that changes a pin reads that from the action's own repository.
//!
//! Each test has its registered negative control (R-176). The workflows are parsed with PyYAML, as
//! `xtask/tests/ci_changes.rs` parses them; `yaml.compose` keeps each value's position, which the comment needs. Unix
//! only: the classifier runs under bash.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SHA: &str = "11d5960a326750d5838078e36cf38b85af677262";
#[cfg(feature = "controls")]
const CHECKOUT: &str =
    "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n";
const CHANGES: &str = "scripts/ci_changes.sh";
const QUICK: &str = ".github/scripts/ci_quick_crates.py";
/// Paths of a change to the CI's own definition: a new workflow and a new action, in every spelling the walk reads.
const CI_PATHS: [&str; 3] = [
    ".github/workflows/new.yml",
    ".github/actions/new/action.yml",
    ".github/actions/new/action.yaml",
];

/// Every `uses:` of the YAML files named as arguments, as JSON, one object a line: its file, line, value and what
/// follows the value on that line. Any mapping's `uses` key counts, whichever part of the file it is in, so a new place
/// to put one fails rather than passes; a `uses` that isn't a plain string is printed as an empty value, which fails.
const SCAN: &str = r#"
import json, sys, yaml

def uses_of(node, seen):
    if id(node) in seen:
        return
    seen.add(id(node))
    if isinstance(node, yaml.SequenceNode):
        for item in node.value:
            yield from uses_of(item, seen)
    elif isinstance(node, yaml.MappingNode):
        for key, value in node.value:
            if key.value == "uses":
                yield value
            yield from uses_of(value, seen)

for path in sys.argv[1:]:
    text = open(path, encoding="utf-8").read()
    for doc in yaml.compose_all(text, Loader=yaml.SafeLoader):
        for value in uses_of(doc, set()):
            print(json.dumps({
                "file": path,
                "line": value.start_mark.line + 1,
                "value": value.value if isinstance(value, yaml.ScalarNode) else "",
                "after": text[value.end_mark.pointer:].split("\n", 1)[0],
            }))
"#;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// One `uses:` in a file.
struct Use {
    file: String,
    line: u64,
    value: String,
    after: String,
}

/// Whether `s` is an exact version tag's comment: `# v` and three dotted numbers, the whole of what follows the value.
fn is_version_comment(s: &str) -> bool {
    let Some(version) = s.trim().strip_prefix("# v") else {
        return false;
    };
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether `s` is a full commit SHA, as GitHub prints one.
fn is_full_sha(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Why `uses`, with `after` on its line behind it, isn't pinned; `None` if it is.
fn unpinned(uses: &str, after: &str) -> Option<String> {
    if uses.starts_with("./") {
        return None;
    }
    let Some((action, reference)) = uses.rsplit_once('@') else {
        return Some("names no reference".to_owned());
    };
    if action.contains(':') || action.split('/').count() < 2 {
        return Some("is not an `owner/repo` action".to_owned());
    }
    if !is_full_sha(reference) {
        return Some(format!(
            "reference `{reference}` is not a full 40-character commit SHA"
        ));
    }
    if !is_version_comment(after) {
        return Some(format!(
            "has no exact version tag as its trailing comment (`# vX.Y.Z`): `{}`",
            after.trim()
        ));
    }
    None
}

/// Every file a `uses:` can be in, under `root`, relative to it and sorted: the walk, never a list.
fn files(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let entry = entry.expect("a directory entry");
            let path = entry.path();
            let relative = path.strip_prefix(root).expect("under the root");
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = entry.file_type().expect("a file type");
            if kind.is_dir() {
                if !(dir == root && (name == ".git" || name == "target")) {
                    walk(root, &path, out);
                }
            } else if kind.is_file() {
                let under_github = relative.starts_with(".github");
                let is_yaml = name.ends_with(".yml") || name.ends_with(".yaml");
                if (under_github && is_yaml) || name == "action.yml" || name == "action.yaml" {
                    out.push(relative.to_string_lossy().into_owned());
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// Every `uses:` of `paths`, relative to `root`.
fn scan(root: &Path, paths: &[String]) -> Vec<Use> {
    let dir = Scratch::new("ci_actions_pinned");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let script = dir.join("scan.py");
    std::fs::write(&script, SCAN).expect("scan written");
    let out = Command::new("python3")
        .arg("-I")
        .arg(&script)
        .args(paths)
        .current_dir(root)
        .timed_output()
        .expect("python3 ran");
    assert!(
        out.status.success(),
        "ci_actions_pinned: the scan failed (PyYAML, or a file that doesn't parse): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .map(|l| {
            let v: Value = serde_json::from_str(l).expect("the scan's json");
            let text = |key: &str| v[key].as_str().expect("a string").to_owned();
            Use {
                file: text("file"),
                line: v["line"].as_u64().expect("a line"),
                value: text("value"),
                after: text("after"),
            }
        })
        .collect()
}

/// Panics naming every `uses:` under `root` that isn't pinned; returns how many there are, pinned or local.
fn check_tree(root: &Path) -> usize {
    let found = scan(root, &files(root));
    let bad: Vec<String> = found
        .iter()
        .filter_map(|u| {
            unpinned(&u.value, &u.after)
                .map(|why| format!("{}:{}: `{}` {why}", u.file, u.line, u.value))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "ci_actions_pinned: unpinned actions:\n{}",
        bad.join("\n")
    );
    found.len()
}

/// A scratch repository holding `.github` as it is, then `edit`ed, for a control to check.
#[cfg(feature = "controls")]
fn edited_tree(edit: impl FnOnce(&Path)) -> Scratch {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("a scratch folder");
        for entry in std::fs::read_dir(from).expect("a folder") {
            let entry = entry.expect("a directory entry");
            let target = to.join(entry.file_name());
            if entry.file_type().expect("a file type").is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).expect("a file copied");
            }
        }
    }
    let dir = Scratch::new("ci_actions_pinned_tree");
    copy(&root().join(".github"), &dir.join(".github"));
    edit(&dir);
    dir
}

/// `.github` with the first pinned checkout of `ci.yml` rewritten as `line`.
#[cfg(feature = "controls")]
fn check_with_ci_line(line: &str) {
    let dir = edited_tree(|dir| {
        let path = dir.join(".github/workflows/ci.yml");
        let text = std::fs::read_to_string(&path).expect("ci.yml");
        assert!(text.contains(CHECKOUT), "ci.yml's checkout has moved");
        std::fs::write(&path, text.replacen(CHECKOUT, line, 1)).expect("ci.yml written");
    });
    check_tree(&dir);
}

/// `.github` with `text` added as the file at `path`.
#[cfg(feature = "controls")]
fn check_with_file(path: &str, text: &str) {
    let dir = edited_tree(|dir| {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("a scratch folder");
        std::fs::write(path, text).expect("a file written");
    });
    check_tree(&dir);
}

#[test]
fn ci_actions_pinned_every_uses() {
    let n = check_tree(&root());
    // More than the local actions and the reusable workflow: the scan has read the steps.
    assert!(n > 100, "the scan read only {n} `uses:`");
}

negative_control!(
    ci_actions_pinned_every_uses,
    "ci.yml's checkout back on its moving tag",
    expected = "reference `v4` is not a full 40-character commit SHA",
    check_with_ci_line("      - uses: actions/checkout@v4\n")
);

#[cfg(feature = "controls")]
mod exact_tag {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "ci.yml's checkout on an exact tag, which can still be moved",
        expected = "reference `v4.4.0` is not a full 40-character commit SHA",
        check_with_ci_line("      - uses: actions/checkout@v4.4.0\n")
    );
}

#[cfg(feature = "controls")]
mod branch {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "ci.yml's checkout on a branch",
        expected = "reference `main` is not a full 40-character commit SHA",
        check_with_ci_line("      - uses: actions/checkout@main\n")
    );
}

#[cfg(feature = "controls")]
mod short_sha {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "ci.yml's checkout on a short SHA",
        expected = "reference `11d5960` is not a full 40-character commit SHA",
        check_with_ci_line("      - uses: actions/checkout@11d5960 # v4.4.0\n")
    );
}

#[cfg(feature = "controls")]
mod missing_comment {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "ci.yml's checkout with no version comment",
        expected = "has no exact version tag as its trailing comment",
        check_with_ci_line(
            "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262\n"
        )
    );
}

#[cfg(feature = "controls")]
mod moving_comment {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "ci.yml's checkout commented with the moving tag",
        expected = "has no exact version tag as its trailing comment",
        check_with_ci_line(
            "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4\n"
        )
    );
}

#[cfg(feature = "controls")]
mod new_workflow {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "a new workflow nobody listed, on a tag",
        expected = "new.yml:4: `actions/checkout@v4`",
        check_with_file(
            ".github/workflows/new.yml",
            "jobs:\n  a:\n    steps:\n      - uses: actions/checkout@v4\n"
        )
    );
}

#[cfg(feature = "controls")]
mod job_level {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "a reusable workflow called on a tag at job level",
        expected = "`some-owner/repo/.github/workflows/w.yml@v1`",
        check_with_file(
            ".github/workflows/reusable.yaml",
            "jobs:\n  a:\n    uses: some-owner/repo/.github/workflows/w.yml@v1\n"
        )
    );
}

#[cfg(feature = "controls")]
mod composite {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "a new composite action with a step on a tag",
        expected = "action.yml:3: `actions/setup-python@v5`",
        check_with_file(
            ".github/actions/new/action.yml",
            "runs:\n  steps:\n    - uses: actions/setup-python@v5\n"
        )
    );
}

#[cfg(feature = "controls")]
mod docker {
    use super::*;

    negative_control!(
        ci_actions_pinned_every_uses,
        "a Docker image, which no SHA pins here",
        expected = "is not an `owner/repo` action",
        check_with_ci_line(
            "      - uses: docker://alpine@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n"
        )
    );
}

/// The rule's table: each `uses`, what follows it, and part of the refusal (`None` where it passes).
const RULE: &[(&str, &str, Option<&str>)] = &[
    // Local paths are never pinned, and need no comment.
    ("./.github/actions/cargo-ci-config", "", None),
    ("./.github/workflows/ci-quick.yml", "", None),
    // A local path is `./`, and nothing else near it.
    (".github/actions/x", "", Some("names no reference")),
    ("../x", "", Some("names no reference")),
    // Actions, sub-actions and reusable workflows, on a SHA with an exact tag.
    (
        "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.0",
        None,
    ),
    (
        "actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830",
        " # v4.3.0",
        None,
    ),
    (
        "o/r/.github/workflows/w.yml@0057852bfaa89a56745cba8c7296529d2fc39830",
        "   # v1.0.12  ",
        None,
    ),
    // The reference, either side of its length, and of its alphabet.
    (
        "a/b@11d5960a326750d5838078e36cf38b85af67726",
        " # v4.4.0",
        Some("is not a full 40-character"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af6772620",
        " # v4.4.0",
        Some("is not a full 40-character"),
    ),
    (
        "a/b@ffffffffffffffffffffffffffffffffffffffff",
        " # v4.4.0",
        None,
    ),
    (
        "a/b@0000000000000000000000000000000000000000",
        " # v4.4.0",
        None,
    ),
    (
        "a/b@gggggggggggggggggggggggggggggggggggggggg",
        " # v4.4.0",
        Some("is not a full 40-character"),
    ),
    (
        "a/b@````````````````````````````````````````",
        " # v4.4.0",
        Some("is not a full 40-character"),
    ),
    (
        "a/b@11D5960A326750D5838078E36CF38B85AF677262",
        " # v4.4.0",
        Some("is not a full 40-character"),
    ),
    ("a/b@", " # v4.4.0", Some("is not a full 40-character")),
    // The action's name.
    (
        "a@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.0",
        Some("`owner/repo`"),
    ),
    (
        "docker://a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.0",
        Some("`owner/repo`"),
    ),
    ("docker://alpine:3.20", "", Some("names no reference")),
    // The comment: exactly `# vX.Y.Z`.
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        "",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.0.1",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # 4.4.0",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.x",
        Some("no exact version tag"),
    ),
    (
        "a/b@11d5960a326750d5838078e36cf38b85af677262",
        " # v4.4.0, the latest",
        Some("no exact version tag"),
    ),
];

fn check_rule(table: &[(&str, &str, Option<&str>)]) {
    for (uses, after, want) in table {
        let got = unpinned(uses, after);
        match want {
            None => assert!(
                got.is_none(),
                "ci_actions_pinned: `{uses}`{after} refused: {got:?}"
            ),
            Some(why) => assert!(
                got.as_deref().is_some_and(|g| g.contains(why)),
                "ci_actions_pinned: `{uses}`{after}: {got:?}, not {why:?}"
            ),
        }
    }
}

#[test]
fn ci_actions_pinned_rule() {
    check_rule(RULE);
}

negative_control!(
    ci_actions_pinned_rule,
    "a SHA one character short taken for a pin",
    expected = "refused:",
    check_rule(&[(
        "a/b@11d5960a326750d5838078e36cf38b85af67726",
        " # v4.4.0",
        None
    )])
);

/// A workflow with one `uses:` of each spelling the scan must read, and text that isn't one.
const SPELLINGS: &str = concat!(
    "# uses: actions/checkout@v4\n",
    "jobs:\n",
    "  plain:\n",
    "    steps:\n",
    "      - name: A step\n",
    "        uses: a/plain@11d5960a326750d5838078e36cf38b85af677262 # v1.0.0\n",
    "      - run: |\n",
    "          uses: actions/checkout@v4\n",
    "      - uses: \"a/quoted@11d5960a326750d5838078e36cf38b85af677262\" # v1.0.1\n",
    "      - {uses: a/flow@v1}\n",
    "  called:\n",
    "    uses: ./.github/workflows/local.yml\n",
);

/// The `(line, value, after)` of each `uses:` the scan finds in `text`.
fn spellings_of(text: &str) -> Vec<(u64, String, String)> {
    let dir = Scratch::new("ci_actions_pinned_spellings");
    std::fs::create_dir_all(&*dir).expect("scratch");
    std::fs::write(dir.join("w.yml"), text).expect("written");
    scan(&dir, &["w.yml".to_owned()])
        .into_iter()
        .map(|u| (u.line, u.value, u.after))
        .collect()
}

fn check_spellings(text: &str) {
    let sha = format!("@{SHA}");
    let want: Vec<(u64, String, String)> = vec![
        (6, format!("a/plain{sha}"), " # v1.0.0".to_owned()),
        (9, format!("a/quoted{sha}"), " # v1.0.1".to_owned()),
        (10, "a/flow@v1".to_owned(), "}".to_owned()),
        (
            12,
            "./.github/workflows/local.yml".to_owned(),
            String::new(),
        ),
    ];
    assert_eq!(
        spellings_of(text),
        want,
        "ci_actions_pinned: the scan's reading of the spellings"
    );
}

#[test]
fn ci_actions_pinned_reads_the_yaml() {
    check_spellings(SPELLINGS);
}

negative_control!(
    ci_actions_pinned_reads_the_yaml,
    "a `uses:` beyond those expected, in a nested job's step: the scan sees it",
    expected = "the scan's reading of the spellings",
    check_spellings(&format!(
        "{SPELLINGS}  more:\n    steps:\n      - uses: a/extra@v1\n"
    ))
);

/// The walk of a scratch repository holding each kind of file, and `extra`.
fn check_finds_every_file(extra: &[&str]) {
    const FOUND: [&str; 6] = [
        ".github/actions/a/action.yml",
        ".github/actions/b/action.yaml",
        ".github/other.yaml",
        ".github/workflows/deep/er.yml",
        ".github/workflows/new.yml",
        "tools/c/action.yaml",
    ];
    const IGNORED: [&str; 6] = [
        ".git/action.yml",
        "target/action.yml",
        "other/plain.yml",
        "other/more.yaml",
        ".github/workflows/notes.md",
        "tools/c/reaction.yml",
    ];
    let dir = Scratch::new("ci_actions_pinned_files");
    for path in FOUND.iter().chain(&IGNORED).chain(extra) {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("a folder");
        std::fs::write(path, "").expect("written");
    }
    assert_eq!(
        files(&dir),
        FOUND,
        "ci_actions_pinned: the files the walk finds"
    );
}

#[test]
fn ci_actions_pinned_finds_every_file() {
    check_finds_every_file(&[]);
    // A folder called `target` or `.git` is only left out at the repository's root.
    let dir = Scratch::new("ci_actions_pinned_nested");
    let nested = dir.join("tools/target/action.yml");
    std::fs::create_dir_all(nested.parent().expect("a folder")).expect("a folder");
    std::fs::write(nested, "").expect("written");
    assert_eq!(files(&dir), ["tools/target/action.yml"]);
}

negative_control!(
    ci_actions_pinned_finds_every_file,
    "a workflow added beside the others, which the walk must report",
    expected = "the files the walk finds",
    check_finds_every_file(&[".github/workflows/added.yml"])
);

/// Each of [`CI_PATHS`] reaches the code tier under `classifier`, `scripts/ci_changes.sh`'s text, and xtask's tests under
/// `quick`, `.github/scripts/ci_quick_crates.py`'s.
fn check_reaches(classifier: &str, quick: &str) {
    let dir = Scratch::new("ci_actions_pinned_reach");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let changes = dir.join("ci_changes.sh");
    let crates = dir.join("ci_quick_crates.py");
    let paths = dir.join("paths");
    std::fs::write(&changes, classifier).expect("classifier written");
    std::fs::write(&crates, quick).expect("crates written");
    for path in CI_PATHS {
        std::fs::write(&paths, path).expect("paths written");
        let out = Command::new("bash")
            .args(["-c", "bash \"$0\" classify < \"$1\""])
            .arg(&changes)
            .arg(&paths)
            .env_remove("CI_REQS_BASE")
            .env_remove("CI_REQS_HEAD")
            .env_remove("CI_REQ_FIELDS")
            .timed_output()
            .expect("the classifier ran");
        let verdict = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && verdict.lines().any(|l| l == "tier=code"),
            "ci_actions_pinned: {path} doesn't reach the code tier, so xtask's tests skip it:\n{verdict}"
        );
        let out = Command::new("bash")
            .args(["-c", "python3 -I \"$0\" --root \"$1\" < \"$2\""])
            .arg(&crates)
            .arg(root())
            .arg(&paths)
            .timed_output()
            .expect("the crates script ran");
        let crates_run = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && crates_run.split_whitespace().any(|c| c == "xtask"),
            "ci_actions_pinned: the quick tier of {path} tests no xtask: `{}`",
            crates_run.trim()
        );
    }
}

#[test]
fn ci_actions_pinned_a_workflow_change_reaches_this_check() {
    check_reaches(&read(CHANGES), &read(QUICK));
}

negative_control!(
    ci_actions_pinned_a_workflow_change_reaches_this_check,
    "the classifier with `.github/` on its allowlist of what is not code",
    expected = "doesn't reach the code tier",
    check_reaches(
        &read(CHANGES).replace(
            "docs/* | plan/* | scripts/*",
            "docs/* | plan/* | .github/* | scripts/*"
        ),
        &read(QUICK)
    )
);

#[cfg(feature = "controls")]
mod quick_tier {
    use super::*;

    negative_control!(
        ci_actions_pinned_a_workflow_change_reaches_this_check,
        "the quick tier's crates sending `.github/` to another crate than xtask",
        expected = "tests no xtask",
        check_reaches(
            &read(CHANGES),
            &read(QUICK).replace("out.add(\"xtask\")", "out.add(\"prin\")")
        )
    );
}
