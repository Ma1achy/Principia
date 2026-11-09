//! The PR tier's choice of crates (the owner's CI model, approved 9 Nov 2026: "Product tests: changed crates + reverse
//! deps (ci_quick_crates.py selection)"), which R-684's quick tier began; the quick tier, its full tier on the
//! `merge-ready` head and the label's re-run are gone with that model. What `ci.yml` does on each change, the
//! required checks among it, is `xtask/tests/ci_changes.rs`'s. Here:
//!
//! - `ci_quick_crates_*`: `.github/scripts/ci_quick_crates.py`, on the real workspace and on a scratch one holding each
//!   kind of dependency: the changed crates plus exactly the crates that link them, and every crate on a path it cannot
//!   place.
//! - `ci_quick_library_packages`: `--library`, the packages with doctests, none on a change to a binary alone.
//! - `ci_quick_doctests_*`: the product tests' doctest step runs on those packages alone, and on none skips.
//! - `ci_tier_decide_names_the_changed_crates`: `.github/scripts/ci_tier.sh decide`, under bash: on a pull request the
//!   changed crates and their reverse dependencies, and their libraries for the doctests; on any other event, or when
//!   the change cannot be read, every crate; and no other subcommand.
//! - `ci_workflows_cancel_superseded_runs_never_on_main`: every workflow a pull request's push runs, but the two R-656
//!   exempts, keeps one group per pull request with `cancel-in-progress`, off on `main`.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script or workflow differing in
//! the one respect it turns on. The scripts run with a scratch HOME and CARGO_HOME and a cleared environment, and the
//! workflows are parsed with PyYAML, as `xtask/tests/ci_changes.rs` parses them. Unix only: the stand-ins are executable
//! scripts.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const TIER: &str = ".github/scripts/ci_tier.sh";
const CRATES: &str = ".github/scripts/ci_quick_crates.py";
const ALL_CRATES: &str = "engine gui kernel ledger prin render validation xtask";
const ALL_PACKAGES: &str =
    "-p engine -p gui -p kernel -p ledger -p prin -p render -p validation -p xtask";
/// Every crate with a library target: all but `prin`, which is a binary.
const ALL_LIBRARIES: &str = "engine gui kernel ledger render validation xtask";
const ALL_LIBRARY_PACKAGES: &str =
    "-p engine -p gui -p kernel -p ledger -p render -p validation -p xtask";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("ci_quick_tier: {rel}: {e}"))
}

#[cfg(feature = "controls")]
/// `rel`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "ci_quick_tier: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

/// A cleared environment for a script: `PATH`, led by `bin` when given, and a scratch HOME and CARGO_HOME.
fn clean(cmd: &mut Command, home: &Path, bin: Option<&Path>) {
    let path = std::env::var("PATH").expect("PATH");
    let path = bin.map_or(path.clone(), |b| format!("{}:{path}", b.display()));
    cmd.env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("CARGO_HOME", home.join("cargo"));
}

// ---- The quick tier's crates ---------------------------------------------------------------------------------------

/// What `script` prints for the changed `paths` on the workspace at `workspace`.
fn crates_of(script: &str, workspace: &Path, paths: &[&str]) -> String {
    run_crates(script, workspace, paths, "")
}

/// What `script --library` prints for the changed `paths` on the workspace at `workspace`.
fn libraries_of(script: &str, workspace: &Path, paths: &[&str]) -> String {
    run_crates(script, workspace, paths, "--library")
}

fn run_crates(script: &str, workspace: &Path, paths: &[&str], flag: &str) -> String {
    let dir = Scratch::new("ci_quick_crates");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_quick_crates.py");
    std::fs::write(&file, script).expect("script");
    let input = dir.join("paths");
    std::fs::write(&input, paths.join("\n")).expect("paths");
    let mut cmd = Command::new("bash");
    cmd.args([
        "-c",
        r#"python3 -I "$1" --root "$2" $4 < "$3""#,
        "sh",
        file.to_str().expect("path"),
        workspace.to_str().expect("path"),
        input.to_str().expect("path"),
        flag,
    ]);
    clean(&mut cmd, &dir, None);
    let out = cmd.timed_output().expect("ci_quick_tier: python ran");
    assert!(
        out.status.success(),
        "ci_quick_tier: the script failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn check_real_workspace(script: &str) {
    let cases: &[(&[&str], &str)] = &[
        // The three crates nothing else links.
        (&["crates/gui/src/lib.rs"], "gui"),
        (&["crates/prin/src/main.rs"], "prin"),
        (&["xtask/src/main.rs"], "xtask"),
        (
            &["crates/gui/src/a.rs", "crates/prin/tests/b.rs"],
            "gui prin",
        ),
        // A crate `validation` links reaches every crate but through it, as does `validation` but for `gui`.
        (&["crates/kernel/src/lib.rs"], ALL_CRATES),
        (&["crates/ledger/src/lib.rs"], ALL_CRATES),
        (&["crates/render/src/lib.rs"], ALL_CRATES),
        (&["crates/engine/src/lib.rs"], ALL_CRATES),
        (
            &["crates/validation/src/lib.rs"],
            "engine kernel ledger prin render validation xtask",
        ),
        // What only xtask's tests read.
        (&[".github/workflows/ci.yml"], "xtask"),
        (&["scripts/contain.sh"], "xtask"),
        // Paths no build reads add nothing beside code.
        (&["crates/gui/src/lib.rs", "docs/gui/design/a.html"], "gui"),
        (&["crates/prin/src/main.rs", "plan/tasks/M1/x.md"], "prin"),
        (&["crates/gui/src/lib.rs", "workbench/x.py"], "gui"),
        (&["crates/gui/src/lib.rs", "README.md"], "gui"),
        (&["crates/gui/src/lib.rs", "decisions.md"], "gui"),
        (&["crates/gui/src/lib.rs", ".claude/agents/x.md"], "gui"),
        // What it cannot place reaches every crate.
        (&["Cargo.lock"], ALL_CRATES),
        (&["Cargo.toml"], ALL_CRATES),
        (&["rust-toolchain.toml"], ALL_CRATES),
        (&[".cargo/config.toml"], ALL_CRATES),
        (&[".config/nextest.toml"], ALL_CRATES),
        (&["fixtures/a/b.json"], ALL_CRATES),
        (&["plan/requirements.yaml"], ALL_CRATES),
        (
            &["crates/gui/src/lib.rs", "plan/requirements.yaml"],
            ALL_CRATES,
        ),
        (&["new-top-level/x"], ALL_CRATES),
        (&["top-level-file.txt"], ALL_CRATES),
        // Only the classifier's root markdown files are dropped; another is unknown, and must not narrow.
        (&["crates/gui/src/lib.rs", "NOTES.md"], ALL_CRATES),
        // A path that merely starts like a member's is no member's.
        (&["crates/gui-extra/src/lib.rs"], ALL_CRATES),
        (&["crates/gui"], ALL_CRATES),
        // No path, or none that names a crate: nothing goes untested.
        (&[], ALL_CRATES),
        (&["docs/a.md"], ALL_CRATES),
    ];
    for (paths, want) in cases {
        assert_eq!(
            crates_of(script, &root(), paths),
            *want,
            "ci_quick_tier: the crates of {paths:?}"
        );
    }
}

#[test]
fn ci_quick_crates_on_the_workspace() {
    check_real_workspace(&read(CRATES));
}

mod everything_on_a_lockfile {
    use super::*;
    negative_control!(
        ci_quick_crates_on_the_workspace,
        "the crates script ignoring a path it cannot place, so a lockfile change beside `gui` tests only `gui`",
        expected = "ci_quick_tier: the crates of [\"crates/gui/src/lib.rs\", \"plan/requirements.yaml\"]",
        check_real_workspace(&edited(
            CRATES,
            "        elif path == EVERYTHING_IN_PLAN:\n            return None\n",
            ""
        ))
    );
}

mod any_root_markdown {
    use super::*;
    negative_control!(
        ci_quick_crates_on_the_workspace,
        "the crates script dropping every root markdown file, so an unknown one beside `gui` tests only `gui`",
        expected = "ci_quick_tier: the crates of [\"crates/gui/src/lib.rs\", \"NOTES.md\"]",
        check_real_workspace(&edited(
            CRATES,
            "path in ROOT_MARKDOWN",
            "\"/\" not in path and path.endswith(\".md\")"
        ))
    );
}

mod dot_github_is_xtask {
    use super::*;
    negative_control!(
        ci_quick_crates_on_the_workspace,
        "the crates script without `.github/` and `scripts/` going to xtask",
        expected = "ci_quick_tier: the crates of [\".github/workflows/ci.yml\"]",
        check_real_workspace(&edited(
            CRATES,
            "XTASK_READS = (\".github/\", \"scripts/\")",
            "XTASK_READS = (\"nothing/\",)"
        ))
    );
}

negative_control!(
    ci_quick_crates_on_the_workspace,
    "the crates script matching a member's directory without its slash",
    expected = "ci_quick_tier: the crates of [\"crates/gui-extra/src/lib.rs\"]",
    check_real_workspace(&edited(
        CRATES,
        "path.startswith(m + \"/\")",
        "path.startswith(m)"
    ))
);

/// A scratch workspace of one member of each kind of target: `a` with `src/lib.rs`, `b` with a `[lib]` table at another
/// path, `c` with only `src/main.rs`.
fn targets_workspace(dir: &Path) {
    let member = |name: &str, body: &str, file: &str| {
        let at = dir.join("crates").join(name);
        std::fs::create_dir_all(at.join("src")).expect("member");
        std::fs::write(
            at.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n{body}"),
        )
        .expect("manifest");
        std::fs::write(at.join(file), "").expect("target");
    };
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\", \"crates/c\"]\n",
    )
    .expect("root manifest");
    member("a", "", "src/lib.rs");
    member("b", "[lib]\npath = \"other.rs\"\n", "other.rs");
    member("c", "", "src/main.rs");
}

fn check_libraries(script: &str) {
    let cases: &[(&[&str], &str)] = &[
        // The binary alone has no doctests.
        (&["crates/prin/src/main.rs"], ""),
        (&["crates/gui/src/lib.rs"], "gui"),
        (&["xtask/src/main.rs"], "xtask"),
        (&["crates/gui/src/a.rs", "crates/prin/tests/b.rs"], "gui"),
        (&["crates/kernel/src/lib.rs"], ALL_LIBRARIES),
        (&["Cargo.lock"], ALL_LIBRARIES),
        (&[], ALL_LIBRARIES),
    ];
    for (paths, want) in cases {
        assert_eq!(
            libraries_of(script, &root(), paths),
            *want,
            "ci_quick_tier: the libraries of {paths:?}"
        );
    }
    let dir = Scratch::new("ci_quick_targets");
    std::fs::create_dir_all(&*dir).expect("scratch");
    targets_workspace(&dir);
    assert_eq!(
        libraries_of(script, &dir, &[]),
        "a b",
        "ci_quick_tier: each kind of target"
    );
}

#[test]
fn ci_quick_library_packages() {
    check_libraries(&read(CRATES));
}

mod every_package_a_library {
    use super::*;
    negative_control!(
        ci_quick_library_packages,
        "the script not filtering, so a change to `prin` alone asks `cargo test --doc` for a binary",
        expected = "ci_quick_tier: the libraries of [\"crates/prin/src/main.rs\"]",
        check_libraries(&edited(CRATES, "    if args.library:\n", "    if False:\n"))
    );
}

mod a_lib_table_missed {
    use super::*;
    negative_control!(
        ci_quick_library_packages,
        "the script reading only `src/lib.rs`, so a `[lib]` at another path has no doctests",
        expected = "ci_quick_tier: each kind of target",
        check_libraries(&edited(
            CRATES,
            "return \"lib\" in manifest(member_dir / \"Cargo.toml\") or ",
            "return "
        ))
    );
}

negative_control!(
    ci_quick_library_packages,
    "the script reading only `[lib]` tables, so no crate here has doctests",
    expected = "ci_quick_tier: the libraries of [\"crates/gui/src/lib.rs\"]",
    check_libraries(&edited(
        CRATES,
        " or (member_dir / \"src\" / \"lib.rs\").exists()",
        ""
    ))
);

/// A scratch workspace holding each kind of dependency on `a`, and `d`, a dev-dependant of a dev-dependant.
fn kinds_workspace(dir: &Path) {
    let member = |name: &str, body: &str| {
        let at = dir.join("crates").join(name);
        std::fs::create_dir_all(&at).expect("member");
        std::fs::write(
            at.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n{body}"),
        )
        .expect("manifest");
    };
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\", \"crates/c\", \"crates/d\", \"crates/e\", \
         \"crates/f\", \"crates/g\", \"crates/h\", \"crates/i\"]\n\n[workspace.dependencies]\na = { path = \
         \"crates/a\" }\nregistry-only = \"1\"\n",
    )
    .expect("root manifest");
    member("a", "[dependencies]\nregistry-only = \"1\"\n");
    // A normal dependency, and one through it.
    member("b", "[dependencies]\na = { path = \"../a\" }\n");
    member("i", "[dependencies]\nb = { path = \"../b\" }\n");
    // A dev-dependency on a crate the change reaches: its test binaries link `b`, so `a`.
    member("c", "[dev-dependencies]\nb = { path = \"../b\" }\n");
    // A dev-dependency on `c`: its test binaries link `c`, never `c`'s dev-dependencies.
    member("d", "[dev-dependencies]\nc = { path = \"../c\" }\n");
    member("e", "[build-dependencies]\na = { path = \"../a\" }\n");
    member(
        "f",
        "[target.'cfg(unix)'.dependencies]\na = { path = \"../a\" }\n",
    );
    member(
        "g",
        "[dependencies]\nrenamed = { package = \"a\", path = \"../a\" }\n",
    );
    member("h", "[dependencies]\na = { workspace = true }\n");
}

fn check_kinds(script: &str) {
    let dir = Scratch::new("ci_quick_kinds");
    std::fs::create_dir_all(&*dir).expect("scratch");
    kinds_workspace(&dir);
    assert_eq!(
        crates_of(script, &dir, &["crates/a/src/lib.rs"]),
        "a b c e f g h i",
        "ci_quick_tier: the crates that link `a`"
    );
    assert_eq!(
        crates_of(script, &dir, &["crates/c/src/lib.rs"]),
        "c d",
        "ci_quick_tier: the crates that link `c`"
    );
    assert_eq!(
        crates_of(script, &dir, &["crates/d/src/lib.rs"]),
        "d",
        "ci_quick_tier: the crates that link `d`"
    );
}

#[test]
fn ci_quick_crates_follow_what_links() {
    check_kinds(&read(CRATES));
}

mod dev_dependencies_do_not_chain {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script following dev-dependencies as it follows normal ones",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "if pkg not in reached and any(d in reached and k == \"normal\" for d, k in its):",
            "if pkg not in reached and any(d in reached for d, k in its):"
        ))
    );
}

mod dev_dependencies_count {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script leaving out dev-dependants",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "    return reached | with_dev\n",
            "    return reached\n"
        ))
    );
}

mod build_dependencies_count {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script reading no build-dependencies",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "TABLES = (\"dependencies\", \"build-dependencies\", \"dev-dependencies\")",
            "TABLES = (\"dependencies\", \"dev-dependencies\")"
        ))
    );
}

mod target_tables_count {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script reading no target-specific tables",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "    tables.extend(doc.get(\"target\", {}).values())\n",
            ""
        ))
    );
}

negative_control!(
    ci_quick_crates_follow_what_links,
    "the crates script naming a renamed dependency by its key",
    expected = "ci_quick_tier: the crates that link `a`",
    check_kinds(&edited(
        CRATES,
        "name = spec.get(\"package\", dep) if isinstance(spec, dict) else dep",
        "name = dep"
    ))
);

// ---- ci_tier.sh decide ----------------------------------------------------------------------------------------------

/// A run of the script: whether it passed, its output and its `$GITHUB_OUTPUT`.
struct Ran {
    code: Option<i32>,
    out: String,
    outputs: BTreeMap<String, String>,
}

impl Ran {
    fn output(&self, key: &str) -> &str {
        self.outputs.get(key).map_or("", String::as_str)
    }
}

/// `script` run as `subcommand` in a scratch folder, beside the real crates script, on the event `event`. `paths` are
/// what `scripts/ci_changes.sh paths` prints; `None`, it fails.
fn tier(script: &str, subcommand: &str, event: &str, paths: Option<&[&str]>) -> Ran {
    let dir = Scratch::new("ci_tier");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_tier.sh");
    write_executable(&file, script).expect("script");
    std::fs::write(dir.join("ci_quick_crates.py"), read(CRATES)).expect("crates script");
    let changes = dir.join("changes.sh");
    let body = match paths {
        Some(p) => format!("#!/bin/bash\nprintf '%s\\n' {}\n", p.join(" ")),
        None => "#!/bin/bash\nexit 1\n".to_owned(),
    };
    write_executable(&changes, body).expect("changes stand-in");
    let output = dir.join("output");
    let mut cmd = Command::new("bash");
    cmd.arg(&file).arg(subcommand);
    clean(&mut cmd, &dir, None);
    cmd.env("GITHUB_OUTPUT", &output)
        .env("GITHUB_EVENT_NAME", event)
        .env("CI_CHANGES", &changes)
        .env("CI_ROOT", root());
    let out = cmd.timed_output().expect("ci_quick_tier: bash ran");
    let outputs = std::fs::read_to_string(&output)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    Ran {
        code: out.status.code(),
        out: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        outputs,
    }
}

const GUI: Option<&[&str]> = Some(&["crates/gui/src/lib.rs"]);
const XTASK: Option<&[&str]> = Some(&["xtask/src/main.rs"]);
const PRIN: Option<&[&str]> = Some(&["crates/prin/src/main.rs"]);
const LOCKFILE: Option<&[&str]> = Some(&["Cargo.lock"]);

/// Each run of `decide`: what it is, the event, the changed paths, and the `packages` and `doc_packages` it must give.
const DECIDES: [(&str, &str, Option<&[&str]>, &str, &str); 7] = [
    ("a change to gui", "pull_request", GUI, "-p gui", "-p gui"),
    (
        "a change to xtask",
        "pull_request",
        XTASK,
        "-p xtask",
        "-p xtask",
    ),
    // `prin` is a binary: no doctests, and `cargo test --doc` fails on a list with none.
    (
        "a change to a binary alone",
        "pull_request",
        PRIN,
        "-p prin",
        "",
    ),
    (
        "a lockfile change",
        "pull_request",
        LOCKFILE,
        ALL_PACKAGES,
        ALL_LIBRARY_PACKAGES,
    ),
    (
        "a change that cannot be read",
        "pull_request",
        None,
        "--workspace",
        "--workspace",
    ),
    ("a push", "push", GUI, "--workspace", "--workspace"),
    (
        "a dispatch",
        "workflow_dispatch",
        GUI,
        "--workspace",
        "--workspace",
    ),
];

fn check_decide(script: &str) {
    for (what, event, paths, packages, doc_packages) in DECIDES {
        let ran = tier(script, "decide", event, paths);
        assert_eq!(
            ran.code,
            Some(0),
            "ci_quick_tier: decide failed on {what}: {}",
            ran.out
        );
        assert_eq!(
            (ran.output("packages"), ran.output("doc_packages")),
            (packages, doc_packages),
            "ci_quick_tier: decide on {what}"
        );
        assert!(
            ran.out.contains(&format!("packages={packages}\n")),
            "ci_quick_tier: decide on {what} does not print what it decided: {}",
            ran.out
        );
    }
    let ran = tier(script, "report", "pull_request", GUI);
    assert_eq!(
        ran.code,
        Some(2),
        "ci_quick_tier: a subcommand decide does not know ran: {}",
        ran.out
    );
}

#[test]
fn ci_tier_decide_names_the_changed_crates() {
    check_decide(&read(TIER));
}

mod a_push_reads_the_change {
    use super::*;
    negative_control!(
        ci_tier_decide_names_the_changed_crates,
        "decide reading the change on every event",
        expected = "ci_quick_tier: decide on a push",
        check_decide(&edited(
            TIER,
            "  if [ \"${GITHUB_EVENT_NAME:-}\" = pull_request ]; then\n",
            "  if true; then\n"
        ))
    );
}

mod doctests_of_a_binary {
    use super::*;
    negative_control!(
        ci_tier_decide_names_the_changed_crates,
        "decide giving the doctests every changed crate, binaries among them",
        expected = "ci_quick_tier: decide on a change to a binary alone",
        check_decide(&edited(TIER, "--library <<< \"$paths\"", "<<< \"$paths\""))
    );
}

mod unread_is_no_crate {
    use super::*;
    negative_control!(
        ci_tier_decide_names_the_changed_crates,
        "decide testing no crate when the change cannot be read",
        expected = "ci_quick_tier: decide on a change that cannot be read",
        check_decide(&edited(
            TIER,
            "  if [ -z \"$crates\" ]; then\n    packages=--workspace\n",
            "  if [ -z \"$crates\" ]; then\n    packages=\n"
        ))
    );
}

negative_control!(
    ci_tier_decide_names_the_changed_crates,
    "decide taking any subcommand for itself",
    expected = "ci_quick_tier: a subcommand decide does not know ran",
    check_decide(&edited(TIER, "  decide) decide ;;\n", "  *) decide ;;\n"))
);

// ---- The workflows -------------------------------------------------------------------------------------------------

/// The workflows parsed by PyYAML, as JSON, keyed by file name, YAML's `on:` read back as the key `on`.
fn parse(files: &[(String, String)]) -> BTreeMap<String, Value> {
    let dir = Scratch::new("ci_quick_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    files
        .iter()
        .map(|(name, text)| {
            let file = dir.join(name);
            std::fs::write(&file, text).expect("workflow");
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
                .expect("python ran");
            assert!(
                out.status.success(),
                "ci_quick_tier: {name}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            (
                name.clone(),
                serde_json::from_slice(&out.stdout).expect("json"),
            )
        })
        .collect()
}

/// Every workflow's file name and text.
fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("workflows")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .map(|p| {
            (
                p.file_name().expect("name").to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("workflow"),
            )
        })
        .collect();
    out.sort();
    out
}

#[cfg(feature = "controls")]
/// Every workflow, with `from` replaced by `to` once in `file`.
fn with_edit(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut all = workflows();
    let at = all
        .iter_mut()
        .find(|(n, _)| n == file)
        .expect("the workflow");
    assert!(
        at.1.contains(from),
        "ci_quick_tier: the control's edit target is gone from {file}: {from}"
    );
    at.1 = at.1.replacen(from, to, 1);
    all
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

/// The product tests' doctest step in `ci.yml`'s `tests` job, and what `changes` hands it.
fn check_doctests(files: &[(String, String)]) {
    let parsed = parse(files);
    let ci = &parsed["ci.yml"]["jobs"];
    let steps: Vec<&Value> = ci["tests"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter(|s| str_of(&s["run"]).contains("--doc"))
        .collect();
    assert_eq!(
        steps.len(),
        1,
        "ci_quick_tier: the tests job has not one doctest step"
    );
    assert_eq!(
        str_of(&steps[0]["env"]["PACKAGES"]),
        "${{ needs.changes.outputs.doc_packages }}",
        "ci_quick_tier: the doctests run on packages that may have no library"
    );
    assert_eq!(
        str_of(&steps[0]["if"]),
        "env.CODE == 'true' && needs.changes.outputs.doc_packages != ''",
        "ci_quick_tier: the doctest step runs with no package, on the whole default set"
    );
    assert_eq!(
        str_of(&ci["changes"]["outputs"]["doc_packages"]),
        "${{ steps.tier.outputs.doc_packages }}",
        "ci_quick_tier: `changes` does not pass on the doctests' packages"
    );
}

#[test]
fn ci_quick_doctests_run_on_the_libraries_alone() {
    check_doctests(&workflows());
}

mod doctests_on_every_package {
    use super::*;
    negative_control!(
        ci_quick_doctests_run_on_the_libraries_alone,
        "the doctest step reading `packages`, which holds `prin`",
        expected = "ci_quick_tier: the doctests run on packages that may have no library",
        check_doctests(&with_edit(
            "ci.yml",
            "          PACKAGES: ${{ needs.changes.outputs.doc_packages }}\n",
            "          PACKAGES: ${{ needs.changes.outputs.packages }}\n"
        ))
    );
}

mod doctests_on_nothing {
    use super::*;
    negative_control!(
        ci_quick_doctests_run_on_the_libraries_alone,
        "the doctest step running on an empty list",
        expected = "ci_quick_tier: the doctest step runs with no package",
        check_doctests(&with_edit(
            "ci.yml",
            "        if: env.CODE == 'true' && needs.changes.outputs.doc_packages != ''\n",
            "        if: env.CODE == 'true'\n"
        ))
    );
}

negative_control!(
    ci_quick_doctests_run_on_the_libraries_alone,
    "`changes` not passing the doctests' packages on",
    expected = "ci_quick_tier: `changes` does not pass on the doctests' packages",
    check_doctests(&with_edit(
        "ci.yml",
        "      doc_packages: ${{ steps.tier.outputs.doc_packages }}\n",
        ""
    ))
);

const NO_GROUP: [&str; 2] = ["gemini-dispatch.yml", "no-atlas-params.yml"];

fn check_concurrency(files: &[(String, String)]) {
    let parsed = parse(files);
    let mut seen = 0;
    for (file, wf) in &parsed {
        if wf["on"].get("pull_request").is_none() || NO_GROUP.contains(&file.as_str()) {
            continue;
        }
        seen += 1;
        let c = &wf["concurrency"];
        assert_eq!(
            str_of(&c["cancel-in-progress"]),
            "${{ github.ref != 'refs/heads/main' }}",
            "ci_quick_tier: {file} does not cancel a superseded run on a pull request, and never on main"
        );
        let group = str_of(&c["group"]);
        assert!(
            group.contains("github.ref") && group.contains("github.run_id"),
            "ci_quick_tier: {file}'s group `{group}` is not one per pull request, with main's runs apart"
        );
    }
    assert!(
        seen >= 5,
        "ci_quick_tier: only {seen} pull-request workflows were checked"
    );
    // The runs on `main`: none of the workflows here may cancel one; `progress.yml` and `nightly.yml` keep their own.
    for (file, wf) in &parsed {
        if wf["on"].get("pull_request").is_some() {
            assert_ne!(
                str_of(&wf["concurrency"]["cancel-in-progress"]),
                "true",
                "ci_quick_tier: {file} cancels on every ref, main's among them"
            );
        }
    }
}

#[test]
fn ci_workflows_cancel_superseded_runs_never_on_main() {
    check_concurrency(&workflows());
}

mod a_workflow_without_a_group {
    use super::*;
    negative_control!(
        ci_workflows_cancel_superseded_runs_never_on_main,
        "pr-check.yml keeping the old run",
        expected = "ci_quick_tier: pr-check.yml does not cancel a superseded run",
        check_concurrency(&with_edit(
            "pr-check.yml",
            "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
            "  cancel-in-progress: false\n"
        ))
    );
}

mod a_group_on_main {
    use super::*;
    negative_control!(
        ci_workflows_cancel_superseded_runs_never_on_main,
        "ci.yml cancelling on main",
        expected = "ci_quick_tier: ci.yml does not cancel a superseded run on a pull request, and never on main",
        check_concurrency(&with_edit(
            "ci.yml",
            "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
            "  cancel-in-progress: true\n"
        ))
    );
}

negative_control!(
    ci_workflows_cancel_superseded_runs_never_on_main,
    "ci.yml's group shared by main's runs",
    expected = "ci_quick_tier: ci.yml's group",
    check_concurrency(&with_edit(
        "ci.yml",
        "group: ci-${{ github.event_name }}-${{ github.ref == 'refs/heads/main' && github.run_id || github.ref }}",
        "group: ci-${{ github.event_name }}-${{ github.ref }}"
    ))
);
